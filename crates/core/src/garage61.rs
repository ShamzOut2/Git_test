//! Garage 61 integration (https://garage61.net/developer). For the current
//! car/track combination it fetches your personal best and your teams' best
//! laps, and publishes them on the `garage61` topic.
//!
//! Auth is a personal access token sent as `Authorization: Bearer <token>`.

use crate::config::{ConfigStore, Garage61Config};
use crate::hub::Hub;
use serde::Serialize;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const API: &str = "https://garage61.net/api/v1";

/// The car/track currently driven, published by the telemetry thread.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Combo {
    pub car_id: i64,
    pub car_name: String,
    pub track_id: i64,
    pub track_name: String,
    pub track_config: String,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LapInfo {
    pub driver: String,
    pub lap_time: f64,
    pub date: String,
    pub is_me: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct G61State {
    /// "disabled" | "noToken" | "waiting" | "loading" | "ok" | "unmapped" | "error"
    pub status: String,
    pub message: String,
    pub user: String,
    pub car: String,
    pub track: String,
    pub personal_best: Option<LapInfo>,
    pub team_best: Option<LapInfo>,
    pub leaderboard: Vec<LapInfo>,
}

struct Client {
    agent: ureq::Agent,
    token: String,
    cars: Option<Vec<Value>>,
    tracks: Option<Vec<Value>>,
    me: Option<Value>,
}

impl Client {
    fn new(token: &str) -> Self {
        let builder = ureq::AgentBuilder::new().timeout(Duration::from_secs(15)).user_agent(concat!("ApexOverlays/", env!("CARGO_PKG_VERSION")));
        #[cfg(windows)]
        let builder = match native_tls::TlsConnector::new() {
            Ok(c) => builder.tls_connector(Arc::new(c)),
            Err(_) => builder,
        };
        let agent = builder.build();
        Self { agent, token: token.to_string(), cars: None, tracks: None, me: None }
    }

    fn get(&self, path: &str, query: &[(&str, String)]) -> Result<Value, String> {
        let mut req = self.agent.get(&format!("{API}{path}")).set("Authorization", &format!("Bearer {}", self.token));
        for (k, v) in query {
            req = req.query(k, v);
        }
        match req.call() {
            Ok(r) => r.into_json::<Value>().map_err(|e| e.to_string()),
            Err(ureq::Error::Status(401 | 403, _)) => Err("Token rejected — check it has the driving data permission".into()),
            Err(ureq::Error::Status(429, _)) => Err("Rate limited by Garage 61, will retry".into()),
            Err(e) => Err(e.to_string()),
        }
    }

    fn items(v: Value) -> Vec<Value> {
        match v.get("items") {
            Some(Value::Array(a)) => a.clone(),
            _ => vec![],
        }
    }
}

fn norm(s: &str) -> String {
    s.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect()
}

fn platform_id(v: &Value) -> Option<i64> {
    ["platform_id", "platformId"].iter().find_map(|k| match v.get(*k)? {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.parse().ok(),
        _ => None,
    })
}

fn is_iracing(v: &Value) -> bool {
    v.get("platform").and_then(|p| p.as_str()).is_none_or(|p| p.eq_ignore_ascii_case("iracing"))
}

/// Find the Garage 61 car matching an iRacing car (by platform id, then by name).
pub fn match_car(cars: &[Value], car_id: i64, car_name: &str) -> Option<i64> {
    let ir: Vec<&Value> = cars.iter().filter(|c| is_iracing(c)).collect();
    if let Some(c) = ir.iter().find(|c| platform_id(c) == Some(car_id)) {
        return c.get("id")?.as_i64();
    }
    let n = norm(car_name);
    ir.iter().find(|c| c.get("name").and_then(|v| v.as_str()).is_some_and(|s| norm(s) == n)).and_then(|c| c.get("id")?.as_i64())
}

/// Find the Garage 61 track layout matching an iRacing track.
pub fn match_track(tracks: &[Value], track_id: i64, name: &str, config: &str) -> Option<i64> {
    let ir: Vec<&Value> = tracks.iter().filter(|t| is_iracing(t)).collect();
    if let Some(t) = ir.iter().find(|t| platform_id(t) == Some(track_id)) {
        return t.get("id")?.as_i64();
    }
    let (n, c) = (norm(name), norm(config));
    let field = |t: &Value, k: &str| norm(t.get(k).and_then(|v| v.as_str()).unwrap_or(""));
    ir.iter()
        .filter(|t| field(t, "name") == n || n.starts_with(&field(t, "name")) && !field(t, "name").is_empty())
        .max_by_key(|t| (field(t, "variant") == c) as i32 * 2 + (field(t, "name") == n) as i32)
        .and_then(|t| t.get("id")?.as_i64())
}

fn lap_info(v: &Value, me_slug: &str) -> Option<LapInfo> {
    let d = v.get("driver");
    let first = d.and_then(|d| d.get("firstName")).and_then(|v| v.as_str()).unwrap_or("");
    let last = d.and_then(|d| d.get("lastName")).and_then(|v| v.as_str()).unwrap_or("");
    let slug = d.and_then(|d| d.get("slug")).and_then(|v| v.as_str()).unwrap_or("");
    Some(LapInfo {
        driver: format!("{first} {last}").trim().to_string(),
        lap_time: v.get("lapTime")?.as_f64()?,
        date: v.get("startTime").and_then(|v| v.as_str()).unwrap_or("").chars().take(10).collect(),
        is_me: !slug.is_empty() && slug == me_slug,
    })
}

fn fetch(client: &mut Client, combo: &Combo, cfg: &Garage61Config) -> Result<G61State, String> {
    if client.me.is_none() {
        client.me = Some(client.get("/me", &[])?);
    }
    if client.cars.is_none() {
        client.cars = Some(Client::items(client.get("/cars", &[])?));
    }
    if client.tracks.is_none() {
        client.tracks = Some(Client::items(client.get("/tracks", &[])?));
    }
    let me = client.me.clone().unwrap_or_default();
    let slug = me.get("slug").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let user = format!("{} {}", me.get("firstName").and_then(|v| v.as_str()).unwrap_or(""), me.get("lastName").and_then(|v| v.as_str()).unwrap_or(""))
        .trim()
        .to_string();
    let mut st = G61State { status: "ok".into(), user, car: combo.car_name.clone(), track: combo.track_name.clone(), ..Default::default() };
    let car = match_car(client.cars.as_deref().unwrap_or(&[]), combo.car_id, &combo.car_name);
    let track = match_track(client.tracks.as_deref().unwrap_or(&[]), combo.track_id, &combo.track_name, &combo.track_config);
    let (Some(car), Some(track)) = (car, track) else {
        st.status = "unmapped".into();
        st.message = "This car/track combination was not found on Garage 61".into();
        return Ok(st);
    };
    let base = [("cars", car.to_string()), ("tracks", track.to_string()), ("group", "driver".to_string())];
    let mine = client.get("/laps", &[base.as_slice(), &[("drivers", "me".into()), ("limit", "1".into())]].concat())?;
    st.personal_best = Client::items(mine).first().and_then(|l| lap_info(l, &slug)).map(|mut l| {
        l.is_me = true;
        l
    });
    let teams: Vec<String> = me
        .get("teams")
        .and_then(|t| t.as_array())
        .map(|a| a.iter().filter_map(|t| t.get("slug").and_then(|s| s.as_str()).map(String::from)).collect())
        .unwrap_or_default();
    if cfg.include_team && !teams.is_empty() {
        let team = client.get("/laps", &[base.as_slice(), &[("teams", teams.join(",")), ("limit", "10".into())]].concat())?;
        st.leaderboard = Client::items(team).iter().filter_map(|l| lap_info(l, &slug)).collect();
        st.leaderboard.sort_by(|a, b| a.lap_time.total_cmp(&b.lap_time));
        st.team_best = st.leaderboard.first().cloned();
    } else if let Some(pb) = &st.personal_best {
        st.leaderboard = vec![pb.clone()];
    }
    Ok(st)
}

/// Background worker: refetches when the combo, token or refresh interval changes.
pub fn spawn(hub: Arc<Hub>, config: ConfigStore, combo: Arc<Mutex<Combo>>) {
    std::thread::Builder::new()
        .name("garage61".into())
        .spawn(move || {
            let mut last: Option<(Combo, String, bool)> = None;
            let mut last_fetch = Instant::now() - Duration::from_secs(3600);
            let mut client: Option<Client> = None;
            let publish = |s: &G61State| {
                hub.publish("garage61", &serde_json::to_string(s).unwrap_or_default());
            };
            loop {
                let cfg = config.read(|c| c.garage61.clone());
                let cur = combo.lock().unwrap().clone();
                let key = (cur.clone(), cfg.token.clone(), cfg.include_team);
                if !cfg.enabled {
                    publish(&G61State { status: "disabled".into(), message: "Enable Garage 61 in Settings → Garage 61".into(), ..Default::default() });
                    last = None;
                } else if cfg.token.trim().is_empty() {
                    publish(&G61State { status: "noToken".into(), message: "Add your Garage 61 token in Settings".into(), ..Default::default() });
                    last = None;
                } else if cur.car_id == 0 || cur.track_id == 0 {
                    publish(&G61State { status: "waiting".into(), message: "Waiting for a car and track".into(), ..Default::default() });
                } else if last.as_ref() != Some(&key) || last_fetch.elapsed() > Duration::from_secs(cfg.refresh_minutes.max(1) as u64 * 60) {
                    if client.as_ref().is_none_or(|c| c.token != cfg.token) {
                        client = Some(Client::new(&cfg.token));
                    }
                    publish(&G61State { status: "loading".into(), car: cur.car_name.clone(), track: cur.track_name.clone(), ..Default::default() });
                    let st =
                        fetch(client.as_mut().unwrap(), &cur, &cfg).unwrap_or_else(|e| G61State { status: "error".into(), message: e, ..Default::default() });
                    publish(&st);
                    last = Some(key);
                    last_fetch = Instant::now();
                }
                std::thread::sleep(Duration::from_secs(2));
            }
        })
        .expect("spawn garage61 thread");
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn matches_cars_and_tracks() {
        let cars = vec![
            json!({"id": 10, "name": "Porsche 911 GT3 R (992)", "platform": "iracing", "platform_id": "169"}),
            json!({"id": 11, "name": "BMW M4 GT3", "platform": "iracing"}),
        ];
        assert_eq!(match_car(&cars, 169, "whatever"), Some(10));
        assert_eq!(match_car(&cars, 1, "BMW M4 GT3"), Some(11));
        assert_eq!(match_car(&cars, 1, "Nope"), None);
        let tracks = vec![
            json!({"id": 1, "name": "Spa-Francorchamps", "variant": "Grand Prix Pits", "platform": "iracing"}),
            json!({"id": 2, "name": "Spa-Francorchamps", "variant": "Endurance", "platform": "iracing"}),
        ];
        assert_eq!(match_track(&tracks, 0, "Spa-Francorchamps", "Endurance"), Some(2));
        assert_eq!(match_track(&tracks, 0, "Circuit de Spa-Francorchamps", "x"), None);
    }

    #[test]
    fn parses_laps() {
        let l = json!({"lapTime": 137.512, "startTime": "2026-05-01T10:00:00Z", "driver": {"firstName": "Max", "lastName": "V", "slug": "max-v"}});
        let i = lap_info(&l, "max-v").unwrap();
        assert_eq!(i.driver, "Max V");
        assert_eq!(i.date, "2026-05-01");
        assert!(i.is_me);
    }
}
