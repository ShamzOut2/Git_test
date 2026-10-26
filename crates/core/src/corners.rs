//! Corner names / turn numbers for the track map, downloaded from the community
//! Lovely Track Data project (https://github.com/Lovely-Sim-Racing/lovely-track-data)
//! and cached per TrackID. Users can override them in Settings → Track map.

use crate::config::Corner;
use crate::hub::{Command, Hub};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const BASE: &str = "https://raw.githubusercontent.com/Lovely-Sim-Racing/lovely-track-data/main/data/";

/// Turn list from a Lovely track file.
pub fn parse_lovely(v: &Value) -> Vec<Corner> {
    let turns = v.get("turn").and_then(|t| t.as_array()).cloned().unwrap_or_default();
    turns
        .iter()
        .enumerate()
        .filter_map(|(i, t)| {
            let start = t.get("start")?.as_f64()? as f32;
            let end = t.get("end").and_then(|e| e.as_f64()).map(|e| e as f32).unwrap_or(start);
            let name = t.get("name").and_then(|n| n.as_str()).unwrap_or("").trim().to_string();
            let digits: String = name.chars().filter(|c| c.is_ascii_digit()).collect();
            let number = t
                .get("number")
                .and_then(|n| n.as_i64())
                .map(|n| n as i32)
                .or_else(|| (name.len() <= 4 && !digits.is_empty()).then(|| digits.parse().ok()).flatten())
                .unwrap_or(i as i32 + 1);
            let name = if name.len() <= 4 && !digits.is_empty() { String::new() } else { name };
            Some(Corner { name, number, start: start.clamp(0.0, 1.0), end: end.clamp(0.0, 1.0) })
        })
        .collect()
}

/// Lovely file path for an iRacing track (its `TrackName`, e.g. "spa combined").
fn find_path(manifest: &Value, track_name: &str) -> Option<String> {
    let list = manifest.pointer("/tracks/iracing")?.as_array()?;
    let clean = |s: &str| s.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect::<String>();
    let want = clean(track_name);
    list.iter().find(|t| t.get("trackId").and_then(|i| i.as_str()).is_some_and(|i| clean(i) == want)).and_then(|t| t.get("path")?.as_str().map(String::from))
}

pub fn spawn(hub: Arc<Hub>, data_dir: PathBuf, track: Arc<Mutex<(i64, String)>>) {
    std::thread::Builder::new()
        .name("corners".into())
        .spawn(move || {
            let dir = data_dir.join("corners");
            let agent = crate::net::agent();
            let mut manifest: Option<Value> = None;
            let mut done: Option<i64> = None;
            let mut retry_at = Instant::now();
            loop {
                std::thread::sleep(Duration::from_secs(2));
                let (id, name) = track.lock().unwrap().clone();
                if id == 0 || done == Some(id) {
                    continue;
                }
                let cache = dir.join(format!("{id}.json"));
                if let Some(c) = std::fs::read_to_string(&cache).ok().and_then(|s| serde_json::from_str::<Vec<Corner>>(&s).ok()) {
                    hub.command(Command::SetCorners { track_id: id, corners: c, source: "lovely" });
                    done = Some(id);
                    continue;
                }
                if Instant::now() < retry_at {
                    continue;
                }
                if manifest.is_none() {
                    manifest = agent.get(&format!("{BASE}manifest.json")).call().ok().and_then(|r| r.into_json().ok());
                }
                let Some(m) = &manifest else {
                    retry_at = Instant::now() + Duration::from_secs(600);
                    continue;
                };
                let Some(path) = find_path(m, &name) else {
                    log::info!("no corner data for {name}");
                    done = Some(id);
                    continue;
                };
                match agent.get(&format!("{BASE}{path}")).call().ok().and_then(|r| r.into_json::<Value>().ok()) {
                    Some(v) => {
                        let corners = parse_lovely(&v);
                        let _ = std::fs::create_dir_all(&dir);
                        let _ = std::fs::write(&cache, serde_json::to_string(&corners).unwrap_or_default());
                        hub.command(Command::SetCorners { track_id: id, corners, source: "lovely" });
                        done = Some(id);
                    }
                    None => retry_at = Instant::now() + Duration::from_secs(600),
                }
            }
        })
        .expect("spawn corners thread");
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_turns() {
        let v = json!({"turn": [
            {"start": 0.05, "end": 0.08, "name": "T1"},
            {"start": 0.1, "end": 0.12, "name": "Eau Rouge", "number": 2},
            {"start": 0.2, "end": 0.25, "name": "Kemmel"}
        ]});
        let c = parse_lovely(&v);
        assert_eq!(c[0], Corner { name: "".into(), number: 1, start: 0.05, end: 0.08 });
        assert_eq!(c[1].name, "Eau Rouge");
        assert_eq!(c[1].number, 2);
        assert_eq!(c[2].number, 3);
    }

    #[test]
    fn finds_track_in_manifest() {
        let m = json!({"tracks": {"iracing": [{"trackId": "spa combined", "path": "iracing/spa-combined.json"}]}});
        assert_eq!(find_path(&m, "spa combined").as_deref(), Some("iracing/spa-combined.json"));
        assert_eq!(find_path(&m, "monza full"), None);
    }
}
