//! Apex Overlays core: reads telemetry (live iRacing or the built-in mock),
//! derives race information at the simulator's 60 Hz, and serves it to
//! overlays over a local WebSocket.

pub mod config;
pub mod corners;
pub mod engine;
pub mod garage61;
pub mod hub;
pub mod iracing_api;
pub mod league;
pub mod lookup;
pub mod model;
pub mod net;
pub mod server;
pub mod source;
pub mod topics;

use config::ConfigStore;
use engine::Engine;
use hub::{Command, Hub};
use model::SessionInfo;
use source::{Poll, Source};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{mpsc, Arc, Mutex};
use topics::Topic;

pub struct Options {
    pub data_dir: PathBuf,
    /// Use the built-in simulator instead of iRacing.
    pub mock: bool,
    /// Serve UI files from this directory first (development / custom overlays).
    pub ui_dir: Option<PathBuf>,
    /// Override the configured port.
    pub port: Option<u16>,
}

pub struct App {
    pub hub: Arc<Hub>,
    pub config: ConfigStore,
    pub addr: SocketAddr,
    pub data_dir: PathBuf,
}

impl App {
    /// URL of the control panel / settings page.
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}/", self.addr.port())
    }

    /// URL of a single overlay (also usable as an OBS browser source).
    pub fn overlay_url(&self, id: &str) -> String {
        format!("http://127.0.0.1:{}/o/{id}", self.addr.port())
    }
}

/// Start the telemetry thread, the Garage 61 worker and the HTTP server.
pub fn start(opts: Options) -> std::io::Result<App> {
    std::fs::create_dir_all(&opts.data_dir)?;
    let config = ConfigStore::load(&opts.data_dir);
    let hub = Hub::new();
    let (port, lan) = config.read(|c| (opts.port.unwrap_or(c.server.port), c.server.lan_access));
    let ip = if lan { [0, 0, 0, 0] } else { [127, 0, 0, 1] };
    let listener = std::net::TcpListener::bind(SocketAddr::from((ip, port)))?;
    listener.set_nonblocking(true)?;
    let addr = listener.local_addr()?;

    let state = server::AppState { hub: hub.clone(), config: config.clone(), ui_dir: opts.ui_dir.clone(), data_dir: opts.data_dir.clone() };
    std::thread::Builder::new().name("http".into()).spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("tokio runtime");
        rt.block_on(async move {
            // Mirror config changes to clients (overlays apply settings live).
            let (hub2, cfg2) = (state.hub.clone(), state.config.clone());
            tokio::spawn(async move {
                let mut rx = cfg2.subscribe();
                loop {
                    hub2.publish("config", &cfg2.read(server::redacted).to_string());
                    if rx.changed().await.is_err() {
                        break;
                    }
                }
            });
            let listener = tokio::net::TcpListener::from_std(listener).expect("listener");
            if let Err(e) = axum::serve(listener, server::router(state)).await {
                log::error!("server stopped: {e}");
            }
        });
    })?;

    let combo = Arc::new(Mutex::new(garage61::Combo::default()));
    garage61::spawn(hub.clone(), config.clone(), combo.clone());
    let track = Arc::new(Mutex::new((0i64, String::new())));
    if !opts.mock {
        corners::spawn(hub.clone(), opts.data_dir.clone(), track.clone());
    }

    let (cmd_tx, cmd_rx) = mpsc::channel();
    hub.set_command_sender(cmd_tx);
    let source: Box<dyn Source> = if opts.mock { Box::new(source::mock::Mock::new()) } else { Box::new(source::iracing::IRacing::new()) };
    *hub.source.lock().unwrap() = source.name();
    hub.publish_status();
    let (h, c, d) = (hub.clone(), config.clone(), opts.data_dir.clone());
    std::thread::Builder::new().name("telemetry".into()).spawn(move || run(source, h, c, combo, track, cmd_rx, d))?;

    Ok(App { hub, config, addr, data_dir: opts.data_dir })
}

#[allow(clippy::too_many_arguments)]
fn run(
    mut source: Box<dyn Source>,
    hub: Arc<Hub>,
    config: ConfigStore,
    combo: Arc<Mutex<garage61::Combo>>,
    track: Arc<Mutex<(i64, String)>>,
    cmds: mpsc::Receiver<Command>,
    data_dir: PathBuf,
) {
    let mut e = Engine::new(config.get(), Some(data_dir));
    e.source = source.name();
    let mut cfg_rx = config.subscribe();
    let mut map_key = (u64::MAX, -1i32, -1i64);
    loop {
        if cfg_rx.has_changed().unwrap_or(false) {
            cfg_rx.mark_unchanged();
            e.set_config(config.get());
        }
        while let Ok(cmd) = cmds.try_recv() {
            match cmd {
                Command::SetCorners { track_id, corners, source } => e.set_remote_corners(track_id, corners, source),
                Command::ResetTrackMap => {
                    e.map.clear();
                    if let Some(m) = source.track_map(e.info.track_id) {
                        e.map.set(m);
                    }
                }
            }
        }
        match source.poll(&mut e.tel) {
            Poll::Idle => continue,
            Poll::Disconnected => {
                if e.connected {
                    e.reset();
                    hub.connected.store(false, Ordering::Relaxed);
                    hub.publish_status();
                    publish(&e, &hub, &mut map_key, true);
                    *combo.lock().unwrap() = Default::default();
                }
            }
            Poll::Data { session_yaml } => {
                if !e.connected {
                    e.connected = true;
                    hub.connected.store(true, Ordering::Relaxed);
                    hub.publish_status();
                }
                if let Some(y) = session_yaml {
                    e.set_session(SessionInfo::parse(&y));
                    if e.map.map.is_none() {
                        if let Some(m) = source.track_map(e.info.track_id) {
                            e.map.set(m);
                        }
                    }
                    if let Some(c) = source.corners() {
                        e.set_remote_corners(e.info.track_id, c, "mock");
                    }
                    *track.lock().unwrap() = (e.info.track_id, e.info.track_name.clone());
                    let me = e.info.driver(e.info.player_car_idx.max(0) as usize);
                    *combo.lock().unwrap() = garage61::Combo {
                        car_id: me.map(|d| d.car_id).unwrap_or(0),
                        car_name: me.map(|d| d.car_name.clone()).unwrap_or_default(),
                        track_id: e.info.track_id,
                        track_name: e.info.track_display_name.clone(),
                        track_config: e.info.track_config.clone(),
                    };
                }
                e.tick();
                publish(&e, &hub, &mut map_key, false);
            }
        }
    }
}

fn publish(e: &Engine, hub: &Hub, map_key: &mut (u64, i32, i64), force: bool) {
    for t in Topic::ALL {
        if !force && !hub.has_subscribers(t.name()) {
            continue;
        }
        if t == Topic::TrackMap {
            // The outline is large and rarely changes: rebuild only when it does.
            let key = (e.map.revision, (e.map.progress() * 100.0) as i32, e.info.track_id);
            if key == *map_key && !force {
                continue;
            }
            *map_key = key;
        }
        hub.publish(t.name(), &t.build(e));
    }
}
