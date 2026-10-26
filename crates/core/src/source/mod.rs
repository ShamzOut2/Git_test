//! Telemetry sources. A source fills a [`Telemetry`] each tick and hands back
//! the raw session-info YAML whenever it changes.

pub mod iracing;
pub mod mock;

use crate::model::Telemetry;

pub enum Poll {
    /// Simulator not running / not in a session. The source already waited a bit.
    Disconnected,
    /// No new data this time around.
    Idle,
    /// `tel` holds a new row. `session_yaml` is set when the session info changed.
    Data { session_yaml: Option<String> },
}

pub trait Source: Send {
    fn name(&self) -> &'static str;
    /// Wait for and read the next telemetry row (blocks at most ~1 frame).
    fn poll(&mut self, tel: &mut Telemetry) -> Poll;
    /// A known track outline, for sources that have one (the mock does).
    fn track_map(&self, _track_id: i64) -> Option<crate::engine::trackmap::TrackMap> {
        None
    }
    /// Corner list for sources that know their track (the mock does).
    fn corners(&self) -> Option<Vec<crate::config::Corner>> {
        None
    }
    /// Pit entry and exit as lap fractions, for sources that know their track.
    fn pit_lane(&self) -> Option<(f32, f32)> {
        None
    }
}
