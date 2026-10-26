//! Local HTTP + WebSocket server. Serves the overlay UI (also usable as OBS
//! browser sources) and streams topics over `/ws`.

use crate::config::{Config, ConfigStore};
use crate::hub::{Command, HostEvent, Hub};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::{header, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use rust_embed::RustEmbed;
use serde::Deserialize;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::broadcast::error::RecvError;

#[derive(RustEmbed)]
#[folder = "../../ui/dist"]
struct Assets;

pub const KEEP_TOKEN: &str = "__keep__";

#[derive(Clone)]
pub struct AppState {
    pub hub: Arc<Hub>,
    pub config: ConfigStore,
    pub ui_dir: Option<PathBuf>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(|s: State<AppState>| async move { asset(&s, "control.html") }))
        .route("/o/{id}", get(|s: State<AppState>, _: Path<String>| async move { asset(&s, "overlay.html") }))
        .route("/api/config", get(get_config).put(put_config))
        .route("/api/status", get(|s: State<AppState>| async move { s.hub.latest("status").map(|m| m.to_string()).unwrap_or_default() }))
        .route("/ws", get(ws))
        .fallback(|s: State<AppState>, uri: Uri| async move { asset(&s, uri.path().trim_start_matches('/')) })
        .with_state(state)
}

fn asset(s: &AppState, path: &str) -> Response {
    let path = if path.is_empty() { "control.html" } else { path };
    if path.contains("..") {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let mime = mime_for(path);
    if let Some(dir) = &s.ui_dir {
        if let Ok(bytes) = std::fs::read(dir.join(path)) {
            return ([(header::CONTENT_TYPE, mime), (header::CACHE_CONTROL, "no-cache")], bytes).into_response();
        }
    }
    match Assets::get(path) {
        Some(f) => ([(header::CONTENT_TYPE, mime), (header::CACHE_CONTROL, "no-cache")], f.data.into_owned()).into_response(),
        None if path.ends_with(".html") => (StatusCode::NOT_FOUND, "UI not built: run `npm run build` in ui/").into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

fn mime_for(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "map" => "application/json",
        _ => "application/octet-stream",
    }
}

/// Config as sent to browsers: the Garage 61 token is never echoed back.
pub fn redacted(c: &Config) -> serde_json::Value {
    let mut v = serde_json::to_value(c).unwrap_or_default();
    if let Some(t) = v.pointer_mut("/garage61/token") {
        if t.as_str().is_some_and(|s| !s.is_empty()) {
            *t = KEEP_TOKEN.into();
        }
    }
    v
}

fn merge_incoming(store: &ConfigStore, mut incoming: Config) {
    if incoming.garage61.token == KEEP_TOKEN {
        incoming.garage61.token = store.read(|c| c.garage61.token.clone());
    }
    store.replace(incoming);
}

async fn get_config(State(s): State<AppState>) -> Json<serde_json::Value> {
    Json(s.config.read(redacted))
}

async fn put_config(State(s): State<AppState>, Json(c): Json<Config>) -> StatusCode {
    merge_incoming(&s.config, c);
    StatusCode::NO_CONTENT
}

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
enum ClientMsg {
    Sub { topics: Vec<String> },
    Unsub { topics: Vec<String> },
    SetEditMode { value: bool },
    ToggleEditMode,
    SetVisible { value: bool },
    Drag { id: String },
    Resize { id: String, dir: String },
    SetConfig { config: Box<Config> },
    PatchOverlay { id: String, patch: serde_json::Value },
    ResetTrackMap,
    OpenSettings,
}

async fn ws(upgrade: WebSocketUpgrade, State(s): State<AppState>) -> Response {
    upgrade.on_upgrade(move |socket| client(socket, s))
}

async fn client(mut socket: WebSocket, s: AppState) {
    s.hub.clients.fetch_add(1, Ordering::Relaxed);
    let mut rx = s.hub.receiver();
    let mut subs: HashSet<String> = HashSet::new();
    loop {
        tokio::select! {
            msg = socket.recv() => {
                let text = match msg {
                    Some(Ok(Message::Text(t))) => t,
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    _ => continue,
                };
                let Ok(m) = serde_json::from_str::<ClientMsg>(&text) else { continue };
                let mut send_now: Vec<Arc<str>> = vec![];
                handle(&s, m, &mut subs, &mut send_now);
                for m in send_now {
                    if socket.send(Message::Text(m.to_string().into())).await.is_err() {
                        break;
                    }
                }
            }
            r = rx.recv() => match r {
                Ok((topic, msg)) => {
                    if subs.contains(&*topic) && socket.send(Message::Text(msg.to_string().into())).await.is_err() {
                        break;
                    }
                }
                Err(RecvError::Lagged(_)) => {
                    // Slow client: skip ahead and send the newest state of everything it watches.
                    for t in &subs {
                        if let Some(m) = s.hub.latest(t) {
                            let _ = socket.send(Message::Text(m.to_string().into())).await;
                        }
                    }
                }
                Err(RecvError::Closed) => break,
            }
        }
    }
    for t in &subs {
        s.hub.unsubscribe_topic(t);
    }
    s.hub.clients.fetch_sub(1, Ordering::Relaxed);
}

fn handle(s: &AppState, m: ClientMsg, subs: &mut HashSet<String>, send_now: &mut Vec<Arc<str>>) {
    match m {
        ClientMsg::Sub { topics } => {
            for t in topics {
                if subs.insert(t.clone()) {
                    s.hub.subscribe_topic(&t);
                    if let Some(m) = s.hub.latest(&t) {
                        send_now.push(m);
                    }
                }
            }
        }
        ClientMsg::Unsub { topics } => {
            for t in topics {
                if subs.remove(&t) {
                    s.hub.unsubscribe_topic(&t);
                }
            }
        }
        ClientMsg::SetEditMode { value } => s.hub.set_edit_mode(value),
        ClientMsg::ToggleEditMode => s.hub.set_edit_mode(!s.hub.edit_mode.load(Ordering::Relaxed)),
        ClientMsg::SetVisible { value } => s.hub.set_visible(value),
        ClientMsg::Drag { id } => s.hub.host(HostEvent::Drag { id }),
        ClientMsg::Resize { id, dir } => s.hub.host(HostEvent::Resize { id, dir }),
        ClientMsg::SetConfig { config } => merge_incoming(&s.config, *config),
        ClientMsg::PatchOverlay { id, patch } => s.config.update(|c| {
            if let Some(o) = c.overlays.iter_mut().find(|o| o.id == id) {
                let mut v = serde_json::to_value(&*o).unwrap_or_default();
                merge_json(&mut v, &patch);
                if let Ok(n) = serde_json::from_value(v) {
                    *o = n;
                }
            }
        }),
        ClientMsg::ResetTrackMap => s.hub.command(Command::ResetTrackMap),
        ClientMsg::OpenSettings => s.hub.host(HostEvent::OpenSettings),
    }
}

/// RFC 7386-style merge patch.
pub fn merge_json(target: &mut serde_json::Value, patch: &serde_json::Value) {
    match (target, patch) {
        (serde_json::Value::Object(t), serde_json::Value::Object(p)) => {
            for (k, v) in p {
                if v.is_null() {
                    t.remove(k);
                } else {
                    merge_json(t.entry(k.clone()).or_insert(serde_json::Value::Null), v);
                }
            }
        }
        (t, p) => *t = p.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_patch() {
        let mut a = serde_json::json!({"x": 1, "s": {"a": 1, "b": 2}});
        merge_json(&mut a, &serde_json::json!({"s": {"b": null, "c": 3}, "y": true}));
        assert_eq!(a, serde_json::json!({"x": 1, "s": {"a": 1, "c": 3}, "y": true}));
    }

    #[test]
    fn token_is_redacted_and_kept() {
        let store = ConfigStore::new(Config::default(), None);
        store.update(|c| c.garage61.token = "secret".into());
        let red = store.read(redacted);
        assert_eq!(red["garage61"]["token"], KEEP_TOKEN);
        let incoming: Config = serde_json::from_value(red).unwrap();
        merge_incoming(&store, incoming);
        assert_eq!(store.read(|c| c.garage61.token.clone()), "secret");
    }
}
