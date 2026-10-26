//! Driver countries from the iRacing Data API, for drivers whose session info carries no
//! usable flair. Results are cached on disk (customer ID → ISO code) so each driver is only
//! looked up once, and the cache is used even when no API login is set up.

use crate::config::ConfigStore;
use crate::hub::{Command, Hub};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Country of one member from a `/data/member/get` entry.
pub fn member_country(m: &Value) -> &'static str {
    let s = |k: &str| m.get(k).and_then(|v| v.as_str()).unwrap_or("");
    crate::lookup::driver_country(s("flair_country_code"), s("flair_shortname"), s("flair_name"), m.get("flair_id").and_then(|v| v.as_i64()).unwrap_or(0))
}

fn load(path: &PathBuf) -> BTreeMap<i64, String> {
    std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

/// `wanted` holds the customer IDs in the current session that have no country yet.
pub fn spawn(hub: Arc<Hub>, config: ConfigStore, data_dir: PathBuf, wanted: Arc<Mutex<Vec<i64>>>) {
    let _ = std::thread::Builder::new().name("countries".into()).spawn(move || {
        let path = data_dir.join("countries.json");
        let mut known = load(&path);
        if !known.is_empty() {
            hub.command(Command::SetCountries(known.clone()));
        }
        // Looked up already (found or not) — iRacing is asked about each driver once per run.
        let mut tried: HashSet<i64> = known.keys().copied().collect();
        let mut client: Option<crate::iracing_api::Client> = None;
        loop {
            std::thread::sleep(Duration::from_secs(5));
            let api = config.read(|c| c.iracing_api.clone());
            if !api.configured() {
                client = None;
                continue;
            }
            let ids: Vec<i64> = wanted.lock().unwrap().iter().copied().filter(|id| *id > 0 && !tried.contains(id)).take(50).collect();
            if ids.is_empty() {
                continue;
            }
            let c = client.get_or_insert_with(|| crate::iracing_api::Client::new(api));
            let list = ids.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",");
            match c.get("member/get", &[("cust_ids", list)]) {
                Ok(v) => {
                    tried.extend(&ids);
                    let mut found = 0;
                    for m in v.get("members").and_then(|m| m.as_array()).into_iter().flatten() {
                        let (Some(id), country) = (m.get("cust_id").and_then(|v| v.as_i64()), member_country(m)) else { continue };
                        if !country.is_empty() {
                            known.insert(id, country.to_string());
                            found += 1;
                        }
                    }
                    if found > 0 {
                        let _ = std::fs::write(&path, serde_json::to_string(&known).unwrap_or_default());
                        hub.command(Command::SetCountries(known.clone()));
                    }
                    log::info!("countries: looked up {} drivers, {found} with a flag", ids.len());
                }
                Err(e) => {
                    log::warn!("countries: {e}");
                    // Don't hammer the API after a failure.
                    std::thread::sleep(Duration::from_secs(60));
                    client = None;
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_member_flair() {
        let m = serde_json::json!({ "cust_id": 1, "flair_id": 222, "flair_name": "United Kingdom", "flair_shortname": "GBR" });
        assert_eq!(member_country(&m), "GB");
        let m = serde_json::json!({ "cust_id": 2, "flair_country_code": "AU" });
        assert_eq!(member_country(&m), "AU");
        assert_eq!(member_country(&serde_json::json!({ "cust_id": 3 })), "");
    }
}
