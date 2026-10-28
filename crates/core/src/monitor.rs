//! What Apex Overlays itself uses: CPU and memory of the app and its overlay windows
//! (WebView2 runs each window in child processes), data sent to the windows, and how
//! busy the telemetry engine is. Published once a second on the `monitor` topic, only
//! while the Monitoring section or the performance widget is open.

use crate::hub::Hub;
use serde::Serialize;
use std::collections::HashSet;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

#[derive(Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MonitorOut {
    /// CPU used by the whole app, % of the PC (like Task Manager): the app itself and its
    /// overlay windows.
    pub cpu: f32,
    pub cpu_app: f32,
    pub cpu_windows: f32,
    /// The whole PC's CPU use, %.
    pub system_cpu: f32,
    /// Memory (working set), MB: total, the app itself, overlay windows.
    pub memory_mb: f32,
    pub memory_app_mb: f32,
    pub memory_windows_mb: f32,
    /// Processes counted (the app and its window processes).
    pub processes: u32,
    /// Data sent to overlays and the settings page.
    pub sent_kbps: f32,
    pub messages_per_sec: f32,
    /// Connected overlay windows / pages.
    pub clients: u32,
    /// Telemetry ticks processed per second (iRacing sends 60) and the time each takes.
    pub telemetry_hz: f32,
    pub tick_us: f32,
    /// Share of one CPU core the telemetry engine uses, %.
    pub engine_cpu: f32,
    pub uptime: u64,
    pub cores: u32,
}

/// The process and everything it started (WebView2 browser, renderer and GPU processes).
fn tree(sys: &System, root: Pid) -> HashSet<Pid> {
    let mut set = HashSet::from([root]);
    loop {
        let before = set.len();
        for (pid, p) in sys.processes() {
            if p.parent().is_some_and(|parent| set.contains(&parent)) {
                set.insert(*pid);
            }
        }
        if set.len() == before {
            return set;
        }
    }
}

fn round(v: f32, digits: i32) -> f32 {
    let m = 10f32.powi(digits);
    (v * m).round() / m
}

pub fn spawn(hub: Arc<Hub>) {
    let _ = std::thread::Builder::new().name("monitor".into()).spawn(move || {
        let Ok(me) = sysinfo::get_current_pid() else { return };
        let started = Instant::now();
        let mut sys = System::new();
        let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1) as f32;
        let counters = |h: &Hub| {
            let s = &h.stats;
            [s.bytes_sent.load(Ordering::Relaxed), s.messages_sent.load(Ordering::Relaxed), s.ticks.load(Ordering::Relaxed), s.tick_ns.load(Ordering::Relaxed)]
        };
        let mut last = (Instant::now(), counters(&hub));
        let mut warm = false;
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let now = (Instant::now(), counters(&hub));
            let dt = now.0.duration_since(last.0).as_secs_f32().max(0.001);
            let d = |i: usize| now.1[i].saturating_sub(last.1[i]) as f32;
            let (bytes, msgs, ticks, tick_ns) = (d(0), d(1), d(2), d(3));
            last = now;
            if !hub.has_subscribers("monitor") {
                warm = false;
                continue;
            }
            sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_cpu().with_memory());
            sys.refresh_cpu_usage();
            let pids = tree(&sys, me);
            let (mut cpu_app, mut cpu_win, mut mem_app, mut mem_win) = (0.0f32, 0.0f32, 0u64, 0u64);
            for pid in &pids {
                let Some(p) = sys.process(*pid) else { continue };
                if *pid == me {
                    cpu_app += p.cpu_usage();
                    mem_app += p.memory();
                } else {
                    cpu_win += p.cpu_usage();
                    mem_win += p.memory();
                }
            }
            // CPU use is measured between two refreshes: the first reading after the monitor
            // opens has nothing to compare with.
            if !warm {
                warm = true;
                continue;
            }
            let mb = |b: u64| round(b as f32 / 1_048_576.0, 1);
            let out = MonitorOut {
                cpu: round((cpu_app + cpu_win) / cores, 1),
                cpu_app: round(cpu_app / cores, 1),
                cpu_windows: round(cpu_win / cores, 1),
                system_cpu: round(sys.global_cpu_usage(), 1),
                memory_mb: mb(mem_app + mem_win),
                memory_app_mb: mb(mem_app),
                memory_windows_mb: mb(mem_win),
                processes: pids.len() as u32,
                sent_kbps: round(bytes / 1024.0 / dt, 1),
                messages_per_sec: round(msgs / dt, 0),
                clients: hub.clients.load(Ordering::Relaxed) as u32,
                telemetry_hz: round(ticks / dt, 1),
                tick_us: if ticks > 0.0 { round(tick_ns / ticks / 1000.0, 1) } else { 0.0 },
                engine_cpu: round(tick_ns / 1e9 / dt * 100.0, 2),
                uptime: started.elapsed().as_secs(),
                cores: cores as u32,
            };
            hub.publish("monitor", &serde_json::to_string(&out).unwrap_or_default());
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measures_this_process() {
        let mut sys = System::new();
        sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_cpu().with_memory());
        let me = sysinfo::get_current_pid().unwrap();
        let t = tree(&sys, me);
        assert!(t.contains(&me));
        assert!(sys.process(me).is_some_and(|p| p.memory() > 0));
    }
}
