//! Live estimates of iRating change, safety-rating change and league points.
//!
//! iRacing publishes neither formula. The iRating estimate is the widely used
//! community model (the "SOF iRating calculator" spreadsheet), which is usually
//! within a few points. The SR estimate is a rough corners-per-incident model and
//! only indicates direction and approximate size.

use crate::config::PointsConfig;
use std::f32::consts::LN_2;

const IR_DEFAULT: f32 = 1350.0;

fn chance(a: f32, b: f32) -> f32 {
    let k = 1600.0 / LN_2;
    let (ea, eb) = ((-a / k).exp(), (-b / k).exp());
    (1.0 - ea) * eb / ((1.0 - eb) * ea + (1.0 - ea) * eb)
}

/// Expected score of each driver against the whole group (same iRacing class).
pub fn expected_scores(irs: &[i32]) -> Vec<f32> {
    let r: Vec<f32> = irs.iter().map(|&i| if i > 0 { i as f32 } else { IR_DEFAULT }).collect();
    r.iter().map(|&a| r.iter().map(|&b| chance(a, b)).sum::<f32>() - 0.5).collect()
}

/// iRating change for finishing at `pos` (1-based) among `n` starters.
pub fn ir_change(n: usize, pos: usize, expected: f32) -> f32 {
    let (n, p) = (n as f32, pos as f32);
    let fudge = (n / 2.0 - p) / 100.0;
    (n - p - expected - fudge) * 200.0 / n
}

/// Corners per incident needed to hold safety rating, by licence class (community ballpark).
fn target_cpi(class: char) -> f32 {
    match class.to_ascii_uppercase() {
        'R' => 8.0,
        'D' => 18.0,
        'C' => 35.0,
        'B' => 55.0,
        'A' => 75.0,
        _ => 80.0,
    }
}

/// Rough SR change for this session so far.
pub fn sr_change(class: char, corners: f32, incidents: i32) -> f32 {
    if corners <= 0.0 {
        return 0.0;
    }
    let target = target_cpi(class);
    // One "virtual" incident at target CPI keeps short sessions near zero.
    let cpi = (corners + target) / (incidents.max(0) as f32 + 1.0);
    let x = (cpi / target).log2() * 0.05 * (corners / 200.0);
    let x = if x < 0.0 { x * 2.5 } else { x };
    x.clamp(-1.5, 1.0)
}

/// Points for a finishing position (1-based) under a league points system.
pub fn points(cfg: &PointsConfig, pos: i32, fastest_lap: bool) -> Option<f32> {
    if cfg.table.is_empty() || pos <= 0 {
        return None;
    }
    let base = cfg.table.get(pos as usize - 1).copied().unwrap_or(cfg.default_points);
    Some(base + if fastest_lap { cfg.fastest_lap_bonus } else { 0.0 })
}

/// Pull a points table out of free text such as "25-18-15-12-10" or "1st 40, 2nd 35, 3rd 32".
pub fn parse_points_table(text: &str) -> Vec<f32> {
    let nums: Vec<f32> = text.split(|c: char| !(c.is_ascii_digit() || c == '.')).filter_map(|t| t.parse::<f32>().ok()).collect();
    // Ordinal style: drop the position numbers (1, 2, 3 ...) that alternate with points.
    let ordinal = nums.len() >= 4 && nums.iter().step_by(2).enumerate().all(|(i, &v)| v == (i + 1) as f32);
    let v: Vec<f32> = if ordinal { nums.iter().skip(1).step_by(2).copied().collect() } else { nums };
    // A plausible table is non-increasing with at least three entries.
    let mut best: Vec<f32> = vec![];
    let mut cur: Vec<f32> = vec![];
    for x in v {
        if cur.last().is_some_and(|&l| x > l) {
            if cur.len() > best.len() {
                best = std::mem::take(&mut cur);
            }
            cur.clear();
        }
        cur.push(x);
    }
    if cur.len() > best.len() {
        best = cur;
    }
    if best.len() >= 3 {
        best
    } else {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn irating_is_zero_sum_ish() {
        let irs = [3000, 2500, 2000, 1500, 1000];
        let e = expected_scores(&irs);
        let changes: Vec<f32> = (0..5).map(|i| ir_change(5, i + 1, e[i])).collect();
        assert!(changes[0] > 0.0 && changes[4] < 0.0, "{changes:?}");
        // Upset: lowest rated wins big.
        assert!(ir_change(5, 1, e[4]) > 100.0);
        assert!(changes.iter().sum::<f32>().abs() < 15.0, "{changes:?}");
        // Equal field: winner gains, last loses, roughly symmetric.
        let e = expected_scores(&[2000; 10]);
        assert!((ir_change(10, 1, e[0]) + ir_change(10, 10, e[9])).abs() < 10.0);
    }

    #[test]
    fn safety_rating_direction() {
        assert!(sr_change('B', 240.0, 0) > 0.05);
        assert!(sr_change('B', 240.0, 8) < -0.05);
        assert!(sr_change('B', 12.0, 0).abs() < 0.01);
    }

    #[test]
    fn points_tables() {
        assert_eq!(parse_points_table("F1 style: 25-18-15-12-10-8-6-4-2-1"), vec![25., 18., 15., 12., 10., 8., 6., 4., 2., 1.]);
        assert_eq!(parse_points_table("1st 40, 2nd 35, 3rd 32, 4th 30"), vec![40., 35., 32., 30.]);
        assert!(parse_points_table("Season 2026 rules").is_empty());
        let cfg = PointsConfig { table: vec![25.0, 18.0], default_points: 1.0, fastest_lap_bonus: 1.0, ..Default::default() };
        assert_eq!(points(&cfg, 1, true), Some(26.0));
        assert_eq!(points(&cfg, 5, false), Some(1.0));
    }
}
