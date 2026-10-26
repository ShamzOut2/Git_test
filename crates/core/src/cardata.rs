//! Car data from the community Lovely Sim Racing projects (CC BY-NC-SA 4.0), downloaded
//! when first needed and cached on disk:
//! - Lovely Car Data (https://github.com/Lovely-Sim-Racing/lovely-car-data): shift-light
//!   RPMs per gear, LED colours and the car's class shorthand, per iRacing `CarPath`.
//! - Lovely Car Classes (https://github.com/Lovely-Sim-Racing/lovely-car-classes):
//!   short names for iRacing's car class names ("GT3 Class" → "GT3").

use crate::hub::{Command, Hub};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const CARS: &str = "https://raw.githubusercontent.com/Lovely-Sim-Racing/lovely-car-data/main/data/";
const CLASSES: &str = "https://raw.githubusercontent.com/Lovely-Sim-Racing/lovely-car-classes/main/data/car-classes.json";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CarData {
    pub car_name: String,
    /// Class shorthand, e.g. "TCR".
    pub car_class: String,
    /// Number of shift lights.
    pub leds: usize,
    /// Redline flash interval in ms (0 = the overlay's default).
    pub blink_ms: u32,
    /// CSS colour of each light (index 0 = the redline colour, "" = none).
    pub colors: Vec<String>,
    /// Gear ("R", "N", "1"...) → [redline RPM, light 1 RPM, light 2 RPM, ...].
    pub rpm: BTreeMap<String, Vec<f32>>,
}

/// "#AARRGGBB" (Lovely / SimHub) → "#RRGGBBAA" (CSS); "" when fully transparent.
/// "#RRGGBB" and colour names pass through.
fn css_color(c: &str) -> String {
    let c = c.trim();
    match c.strip_prefix('#') {
        Some(h) if h.len() == 8 && h.chars().all(|x| x.is_ascii_hexdigit()) => {
            if &h[..2] == "00" {
                String::new()
            } else {
                format!("#{}{}", &h[2..], &h[..2])
            }
        }
        _ => c.to_string(),
    }
}

pub fn parse_car(v: &Value) -> Option<CarData> {
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").trim().to_string();
    let colors: Vec<String> = v.get("ledColor")?.as_array()?.iter().map(|c| css_color(c.as_str().unwrap_or(""))).collect();
    let mut rpm = BTreeMap::new();
    // ledRpm is a list holding one object of gear → RPMs.
    for obj in v.get("ledRpm").and_then(|r| r.as_array()).into_iter().flatten() {
        for (gear, list) in obj.as_object().into_iter().flatten() {
            let r: Vec<f32> = list.as_array().into_iter().flatten().filter_map(|x| x.as_f64()).map(|x| x as f32).collect();
            if r.len() >= 2 {
                rpm.insert(gear.to_uppercase(), r);
            }
        }
    }
    if rpm.is_empty() {
        return None;
    }
    let leds = v.get("ledNumber").and_then(|n| n.as_u64()).map(|n| n as usize).unwrap_or(colors.len().saturating_sub(1));
    Some(CarData {
        car_name: s("carName"),
        car_class: s("carClass"),
        leds: leds.min(colors.len().saturating_sub(1)).max(1),
        blink_ms: v.get("redlineBlinkInterval").and_then(|n| n.as_u64()).unwrap_or(0) as u32,
        colors,
        rpm,
    })
}

/// Lovely file path for an iRacing car (`CarPath`, e.g. "hyundaielantracn7").
fn find_car(manifest: &Value, car_path: &str) -> Option<String> {
    manifest
        .pointer("/cars/iracing")?
        .as_array()?
        .iter()
        .find(|c| c.get("carId").and_then(|i| i.as_str()).is_some_and(|i| i.eq_ignore_ascii_case(car_path)))
        .and_then(|c| c.get("path")?.as_str().map(String::from))
}

fn read_json(path: &Path) -> Option<Value> {
    std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok())
}

fn write_json(path: &Path, v: &impl Serialize) {
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = std::fs::write(path, serde_json::to_string(v).unwrap_or_default());
}

/// `wanted` holds the `CarPath`s in the current session.
pub fn spawn(hub: Arc<Hub>, data_dir: PathBuf, wanted: Arc<Mutex<Vec<String>>>) {
    let _ = std::thread::Builder::new().name("cardata".into()).spawn(move || {
        let dir = data_dir.join("lovely");
        let agent = crate::net::agent();
        let get = |url: &str| agent.get(url).call().ok().and_then(|r| r.into_json::<Value>().ok());

        // Class shorthands: cached copy first, then refreshed once per run.
        let classes_file = dir.join("car-classes.json");
        let parse_classes = |v: &Value| -> HashMap<String, String> {
            v.as_object().into_iter().flatten().filter_map(|(k, s)| s.as_str().map(|s| (k.clone(), s.to_string()))).collect()
        };
        if let Some(v) = read_json(&classes_file) {
            hub.command(Command::SetClassNames(parse_classes(&v)));
        }
        let mut classes_fresh = false;

        let mut manifest: Option<Value> = read_json(&dir.join("car-manifest.json"));
        let mut manifest_fresh = false;
        let mut known: HashMap<String, CarData> = HashMap::new();
        let mut tried: HashSet<String> = HashSet::new();
        loop {
            std::thread::sleep(Duration::from_secs(3));
            let paths: Vec<String> = wanted.lock().unwrap().iter().map(|p| p.to_lowercase()).filter(|p| !p.is_empty() && !tried.contains(p)).collect();
            if paths.is_empty() {
                continue;
            }
            if !classes_fresh {
                if let Some(v) = get(CLASSES) {
                    write_json(&classes_file, &v);
                    hub.command(Command::SetClassNames(parse_classes(&v)));
                    classes_fresh = true;
                }
            }
            let mut changed = false;
            for p in paths {
                let cache = dir.join("cars").join(format!("{p}.json"));
                if let Some(c) = read_json(&cache).and_then(|v| serde_json::from_value::<CarData>(v).ok()) {
                    known.insert(p.clone(), c);
                    tried.insert(p);
                    changed = true;
                    continue;
                }
                // A car missing from a cached manifest may be new: fetch the manifest once per run.
                if !manifest_fresh && manifest.as_ref().and_then(|m| find_car(m, &p)).is_none() {
                    if let Some(m) = get(&format!("{CARS}manifest.json")) {
                        write_json(&dir.join("car-manifest.json"), &m);
                        manifest = Some(m);
                    }
                    manifest_fresh = true;
                }
                let Some(m) = &manifest else { break };
                tried.insert(p.clone());
                let Some(file) = find_car(m, &p) else {
                    log::info!("no Lovely car data for {p}");
                    continue;
                };
                match get(&format!("{CARS}{file}")).as_ref().and_then(parse_car) {
                    Some(c) => {
                        write_json(&cache, &c);
                        known.insert(p, c);
                        changed = true;
                    }
                    None => log::warn!("could not read Lovely car data {file}"),
                }
            }
            if changed {
                hub.command(Command::SetCarData(known.clone()));
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_lovely_car() {
        let v = json!({
            "carName": "Hyundai Elantra N", "carId": "hyundaielantracn7", "carClass": "TCR", "ledNumber": 4,
            "redlineBlinkInterval": 0,
            "ledColor": ["#00000000", "#FF00FF00", "#FFFFFF00", "#FFFF0000", "Blue"],
            "ledRpm": [{ "1": [6500, 5700, 6050, 6300, 6450], "n": [6500, 5700, 6050, 6300, 6450] }]
        });
        let c = parse_car(&v).unwrap();
        assert_eq!(c.car_class, "TCR");
        assert_eq!(c.leds, 4);
        assert_eq!(c.colors, ["", "#00FF00FF", "#FFFF00FF", "#FF0000FF", "Blue"]);
        assert_eq!(c.rpm["1"][0], 6500.0);
        assert!(c.rpm.contains_key("N"));
    }

    #[test]
    fn finds_car_by_path() {
        let m = json!({"cars": {"iracing": [{"carId": "hyundaielantracn7", "path": "iracing/hyundaielantracn7.json"}]}});
        assert_eq!(find_car(&m, "HyundaiElantraCN7").as_deref(), Some("iracing/hyundaielantracn7.json"));
        assert_eq!(find_car(&m, "nope"), None);
    }
}
