//! Wet/dry tyre advisor. Combines the sim's track wetness, the weather trend,
//! what the rest of the class is running and the lap-time crossover between
//! cars on wets and cars on slicks.

use super::weather::Trend;
use super::Engine;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Advice {
    /// "dry" | "wet" | "unknown"
    pub current: &'static str,
    pub car_has_wets: bool,
    /// "stayOut" | "considerWets" | "boxForWets" | "considerSlicks" | "boxForSlicks" | "noWets"
    pub recommendation: &'static str,
    pub confidence: f32,
    /// True when the sim allows fitting wets (track declared wet).
    pub wets_allowed: bool,
    pub wetness: i32,
    pub wetness_label: &'static str,
    pub precipitation: f32,
    pub trend: &'static str,
    pub field_on_wets: usize,
    pub field_total: usize,
    pub wet_pace: Option<f32>,
    pub dry_pace: Option<f32>,
    pub reasons: Vec<String>,
}

fn median(mut v: Vec<f32>) -> Option<f32> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    Some(v[v.len() / 2])
}

pub fn advise(e: &Engine) -> Advice {
    let t = &e.tel;
    let cfg = &e.cfg.tyres;
    let on_wet = e.info.tire_is_wet(t.tire_compound);
    let has_wets = e.info.has_wet_tires();
    let trend = e.weather.trend();
    let mut reasons = vec![];
    let mut score = 0.0f32;

    let w = t.track_wetness;
    if w > 0 {
        if w >= cfg.wet_threshold {
            score += 2.0;
            reasons.push(format!("Track is {}", crate::model::wetness_label(w).to_lowercase()));
        } else if w <= cfg.dry_threshold {
            score -= 2.0;
            reasons.push(format!("Track is {}", crate::model::wetness_label(w).to_lowercase()));
        } else {
            let mid = (cfg.wet_threshold + cfg.dry_threshold) as f32 / 2.0;
            score += (w as f32 - mid) * 0.75;
            reasons.push(format!("Track is {} (crossover zone)", crate::model::wetness_label(w).to_lowercase()));
        }
    }
    match trend {
        Trend::Wetting => {
            score += 0.75;
            reasons.push("Conditions getting wetter".into());
        }
        Trend::Drying => {
            score -= 0.75;
            reasons.push("Track drying".into());
        }
        Trend::Stable => {}
    }
    if t.precipitation > 0.3 {
        score += 0.5;
        reasons.push(format!("Rain falling ({:.0}%)", t.precipitation * 100.0));
    }

    // Field consensus and lap-time crossover within the focus car's class.
    let focus = t.focus_idx();
    let class = focus.and_then(|f| e.static_of(f)).map(|s| s.class.id.clone());
    let (mut on_wets, mut total) = (0, 0);
    let (mut wet_laps, mut dry_laps) = (vec![], vec![]);
    for r in &e.order {
        if Some(r.idx) == focus || e.static_of(r.idx).map(|s| &s.class.id) != class.as_ref() {
            continue;
        }
        let c = &t.cars[r.idx];
        if !c.in_world() {
            continue;
        }
        if let Some(wet) = e.info.tire_is_wet(c.tire_compound) {
            total += 1;
            on_wets += wet as usize;
        }
        let recent: Vec<f32> = e.hist[r.idx]
            .laps
            .iter()
            .filter(|l| !l.pit && t.session_time - l.at < 240.0)
            .filter_map(|l| l.wet.map(|w| (w, l.time)))
            .map(|(w, time)| if w { -time } else { time })
            .collect();
        if let Some(best_wet) = recent.iter().filter(|v| **v < 0.0).map(|v| -v).reduce(f32::min) {
            wet_laps.push(best_wet);
        }
        if let Some(best_dry) = recent.iter().filter(|v| **v > 0.0).copied().reduce(f32::min) {
            dry_laps.push(best_dry);
        }
    }
    if cfg.use_field && total >= 3 {
        let frac = on_wets as f32 / total as f32;
        if frac >= 0.5 {
            score += 1.0;
            reasons.push(format!("{on_wets}/{total} of your class on wets"));
        } else if frac <= 0.2 && w > 0 && w < cfg.wet_threshold {
            score -= 0.5;
            reasons.push(format!("{}/{total} of your class on slicks", total - on_wets));
        }
    }
    let (wet_pace, dry_pace) = if wet_laps.len() >= 2 && dry_laps.len() >= 2 { (median(wet_laps), median(dry_laps)) } else { (None, None) };
    if let (true, Some(wp), Some(dp)) = (cfg.use_crossover, wet_pace, dry_pace) {
        if wp + cfg.crossover_margin < dp {
            score += 1.5;
            reasons.push(format!("Wets {:.1}s/lap faster", dp - wp));
        } else if dp + cfg.crossover_margin < wp {
            score -= 1.5;
            reasons.push(format!("Slicks {:.1}s/lap faster", wp - dp));
        }
    }

    let recommendation = match (has_wets, on_wet) {
        (false, _) if !e.info.tires.is_empty() => "noWets",
        (_, Some(true)) if score <= -2.0 => "boxForSlicks",
        (_, Some(true)) if score <= -1.0 => "considerSlicks",
        (_, Some(true)) => "stayOut",
        (_, _) if score >= 2.0 => "boxForWets",
        (_, _) if score >= 1.0 => "considerWets",
        _ => "stayOut",
    };
    if recommendation.ends_with("Wets") && !t.declared_wet {
        reasons.push("Track not declared wet yet — wets may not be available".into());
    }
    Advice {
        current: match on_wet {
            Some(true) => "wet",
            Some(false) => "dry",
            None => "unknown",
        },
        car_has_wets: has_wets,
        recommendation,
        confidence: (score.abs() / 4.0).clamp(0.0, 1.0),
        wets_allowed: t.declared_wet,
        wetness: w,
        wetness_label: crate::model::wetness_label(w),
        precipitation: t.precipitation,
        trend: trend.as_str(),
        field_on_wets: on_wets,
        field_total: total,
        wet_pace,
        dry_pace,
        reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{engine, place};
    use super::*;
    use crate::model::{SessionInfo, TireDef};

    fn with_tires(mut e: Engine) -> Engine {
        let mut info: SessionInfo = e.info.clone();
        info.tires = vec![TireDef { index: 0, kind: "Hard".into() }, TireDef { index: 1, kind: "Wet".into() }];
        e.set_session(info);
        e
    }

    #[test]
    fn advises_wets_when_track_wet() {
        let mut e = with_tires(engine(2, true));
        place(&mut e, 0, 1, 0.1);
        e.tel.track_wetness = 5;
        e.tel.declared_wet = true;
        e.tick();
        let a = advise(&e);
        assert_eq!(a.current, "dry");
        assert_eq!(a.recommendation, "boxForWets");
    }

    #[test]
    fn advises_slicks_when_drying() {
        let mut e = with_tires(engine(2, true));
        place(&mut e, 0, 1, 0.1);
        e.tel.tire_compound = 1;
        e.tel.track_wetness = 1;
        e.tick();
        assert_eq!(advise(&e).recommendation, "boxForSlicks");
        e.tel.track_wetness = 3;
        e.tick();
        assert_eq!(advise(&e).recommendation, "stayOut");
    }
}
