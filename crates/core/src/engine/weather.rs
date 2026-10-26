//! Rolling history of track wetness and precipitation for trend detection.

use crate::model::Telemetry;
use std::collections::VecDeque;

const SAMPLE_EVERY: f64 = 5.0;
const KEEP: f64 = 15.0 * 60.0;

#[derive(Debug, Clone, Copy)]
pub struct Sample {
    pub at: f64,
    pub wetness: i32,
    pub precipitation: f32,
    pub track_temp: f32,
}

#[derive(Debug, Default)]
pub struct WeatherTracker {
    pub samples: VecDeque<Sample>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trend {
    Drying,
    Stable,
    Wetting,
}

impl Trend {
    pub fn as_str(self) -> &'static str {
        match self {
            Trend::Drying => "drying",
            Trend::Stable => "stable",
            Trend::Wetting => "wetting",
        }
    }
}

impl WeatherTracker {
    pub fn update(&mut self, t: &Telemetry) {
        if let Some(last) = self.samples.back() {
            if t.session_time < last.at {
                self.samples.clear();
            } else if t.session_time - last.at < SAMPLE_EVERY {
                return;
            }
        }
        self.samples.push_back(Sample { at: t.session_time, wetness: t.track_wetness, precipitation: t.precipitation, track_temp: t.track_temp });
        while self.samples.front().is_some_and(|s| t.session_time - s.at > KEEP) {
            self.samples.pop_front();
        }
    }

    fn avg_between(&self, from: f64, to: f64) -> Option<(f32, f32)> {
        let v: Vec<&Sample> = self.samples.iter().filter(|s| s.at >= from && s.at <= to).collect();
        if v.is_empty() {
            return None;
        }
        let n = v.len() as f32;
        Some((v.iter().map(|s| s.wetness as f32).sum::<f32>() / n, v.iter().map(|s| s.precipitation).sum::<f32>() / n))
    }

    /// Compares the last minute with conditions ~5 minutes ago.
    pub fn trend(&self) -> Trend {
        let Some(now) = self.samples.back().map(|s| s.at) else { return Trend::Stable };
        let (Some(recent), Some(old)) = (self.avg_between(now - 60.0, now), self.avg_between(now - 360.0, now - 240.0)) else {
            return Trend::Stable;
        };
        let score = (recent.0 - old.0) + (recent.1 - old.1) * 4.0;
        if score > 0.4 {
            Trend::Wetting
        } else if score < -0.4 {
            Trend::Drying
        } else {
            Trend::Stable
        }
    }

    /// Track temperature change over the last ~5 minutes (°C).
    pub fn track_temp_delta(&self) -> f32 {
        let (Some(first), Some(last)) = (self.samples.iter().find(|s| self.samples.back().unwrap().at - s.at <= 300.0), self.samples.back()) else {
            return 0.0;
        };
        last.track_temp - first.track_temp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_rain() {
        let mut w = WeatherTracker::default();
        let mut t = Telemetry::default();
        for s in 0..80 {
            t.session_time = s as f64 * 5.0;
            t.track_wetness = if s < 30 { 1 } else { 4 };
            t.precipitation = if s < 30 { 0.0 } else { 0.4 };
            w.update(&t);
        }
        assert_eq!(w.trend(), Trend::Wetting);
    }
}
