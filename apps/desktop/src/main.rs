//! Desktop host: runs the Apex core in-process and shows every enabled overlay
//! as its own transparent, always-on-top, click-through window. In edit mode the
//! windows accept the mouse so they can be dragged and resized; their geometry
//! is written back to the config.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use apex_core::config::{Config, ConfigStore};
use apex_core::hub::{HostEvent, Hub};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, RunEvent, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

const SETTINGS: &str = "settings";
const PREFIX: &str = "ov-";

/// Pending (position, size) of an overlay window, in logical pixels.
type Geometry = (Option<(i32, i32)>, Option<(u32, u32)>);

struct Core {
    hub: Arc<Hub>,
    config: ConfigStore,
    url: String,
    port: u16,
    /// Window geometry changes waiting to be written to the config (debounced).
    pending: Mutex<HashMap<String, Geometry>>,
}

fn label_for(id: &str) -> String {
    let safe: String = id.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    format!("{PREFIX}{safe}")
}

fn id_for(cfg: &Config, label: &str) -> Option<String> {
    cfg.overlays.iter().find(|o| label_for(&o.id) == label).map(|o| o.id.clone())
}

fn show_settings(app: &AppHandle) {
    let core = app.state::<Core>();
    if let Some(w) = app.get_webview_window(SETTINGS) {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
        return;
    }
    let url = core.url.parse().expect("valid url");
    let _ = WebviewWindowBuilder::new(app, SETTINGS, WebviewUrl::External(url))
        .title("Apex Overlays")
        .inner_size(1240.0, 860.0)
        .min_inner_size(900.0, 600.0)
        .build();
}

/// Make the set of overlay windows match the config.
fn reconcile(app: &AppHandle) {
    let core = app.state::<Core>();
    let cfg = core.config.get();
    let edit = core.hub.edit_mode.load(Ordering::Relaxed);
    let visible = core.hub.visible.load(Ordering::Relaxed);
    let mut wanted = Vec::new();
    for o in cfg.overlays.iter().filter(|o| o.enabled) {
        let label = label_for(&o.id);
        wanted.push(label.clone());
        let win = match app.get_webview_window(&label) {
            Some(w) => {
                sync_geometry(&w, o.x, o.y, o.width, o.height);
                w
            }
            None => {
                let url = format!("http://127.0.0.1:{}/o/{}", core.port, o.id).parse().expect("valid url");
                match WebviewWindowBuilder::new(app, &label, WebviewUrl::External(url))
                    .title(if o.name.is_empty() { &o.kind } else { &o.name })
                    .transparent(true)
                    .decorations(false)
                    .shadow(false)
                    .always_on_top(true)
                    .skip_taskbar(true)
                    .resizable(true)
                    .focused(false)
                    .visible(visible || edit)
                    .position(o.x as f64, o.y as f64)
                    .inner_size(o.width.max(40) as f64, o.height.max(20) as f64)
                    .build()
                {
                    Ok(w) => w,
                    Err(e) => {
                        log::error!("could not open overlay {}: {e}", o.id);
                        continue;
                    }
                }
            }
        };
        let _ = win.set_ignore_cursor_events(!edit);
        let _ = if visible || edit { win.show() } else { win.hide() };
    }
    for (label, w) in app.webview_windows() {
        if label.starts_with(PREFIX) && !wanted.contains(&label) {
            let _ = w.close();
        }
    }
}

fn sync_geometry(w: &WebviewWindow, x: i32, y: i32, width: u32, height: u32) {
    let scale = w.scale_factor().unwrap_or(1.0);
    if let Ok(p) = w.outer_position() {
        let p: LogicalPosition<f64> = p.to_logical(scale);
        if (p.x - x as f64).abs() > 1.0 || (p.y - y as f64).abs() > 1.0 {
            let _ = w.set_position(LogicalPosition::new(x as f64, y as f64));
        }
    }
    if let Ok(s) = w.inner_size() {
        let s: LogicalSize<f64> = s.to_logical(scale);
        if (s.width - width as f64).abs() > 1.0 || (s.height - height as f64).abs() > 1.0 {
            let _ = w.set_size(LogicalSize::new(width as f64, height as f64));
        }
    }
}

fn handle_host_event(app: &AppHandle, ev: HostEvent) {
    match ev {
        HostEvent::Drag { id } => {
            if let Some(w) = app.get_webview_window(&label_for(&id)) {
                let _ = w.start_dragging();
            }
        }
        HostEvent::Resize { id, dir } => {
            if let Some(w) = app.get_webview_window(&label_for(&id)) {
                use tauri_runtime::ResizeDirection as D;
                let d = match dir.as_str() {
                    "East" => D::East,
                    "South" => D::South,
                    "SouthWest" => D::SouthWest,
                    "West" => D::West,
                    "North" => D::North,
                    _ => D::SouthEast,
                };
                let _ = w.as_ref().window().start_resize_dragging(d);
            }
        }
        HostEvent::EditMode { .. } | HostEvent::Visible { .. } => reconcile(app),
        HostEvent::OpenSettings => show_settings(app),
    }
}

/// Remember where the user put an overlay (flushed to disk by `flush_geometry`).
fn record_geometry(app: &AppHandle, label: &str, pos: Option<(i32, i32)>, size: Option<(u32, u32)>) {
    let core = app.state::<Core>();
    let Some(id) = core.config.read(|c| id_for(c, label)) else { return };
    let mut pending = core.pending.lock().unwrap();
    let e = pending.entry(id).or_default();
    if pos.is_some() {
        e.0 = pos;
    }
    if size.is_some() {
        e.1 = size;
    }
}

fn flush_geometry(app: &AppHandle) {
    let core = app.state::<Core>();
    let pending: Vec<_> = core.pending.lock().unwrap().drain().collect();
    if pending.is_empty() {
        return;
    }
    core.config.update(|c| {
        for (id, (pos, size)) in pending {
            if let Some(o) = c.overlays.iter_mut().find(|o| o.id == id) {
                if let Some((x, y)) = pos {
                    o.x = x;
                    o.y = y;
                }
                if let Some((w, h)) = size {
                    o.width = w;
                    o.height = h;
                }
            }
        }
    });
}

fn register_hotkeys(app: &AppHandle) {
    let core = app.state::<Core>();
    let keys = core.config.read(|c| c.hotkeys.clone());
    let gs = app.global_shortcut();
    let hub = core.hub.clone();
    if let Err(e) = gs.on_shortcut(keys.toggle_edit.as_str(), move |_, _, ev| {
        if ev.state == ShortcutState::Pressed {
            hub.set_edit_mode(!hub.edit_mode.load(Ordering::Relaxed));
        }
    }) {
        log::warn!("hotkey {}: {e}", keys.toggle_edit);
    }
    let hub = core.hub.clone();
    if let Err(e) = gs.on_shortcut(keys.toggle_visibility.as_str(), move |_, _, ev| {
        if ev.state == ShortcutState::Pressed {
            hub.set_visible(!hub.visible.load(Ordering::Relaxed));
        }
    }) {
        log::warn!("hotkey {}: {e}", keys.toggle_visibility);
    }
}

fn tray(app: &AppHandle) -> tauri::Result<()> {
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let edit = MenuItem::with_id(app, "edit", "Toggle edit mode", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "visible", "Show / hide overlays", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&settings, &edit, &hide, &PredefinedMenuItem::separator(app)?, &quit])?;
    let mut builder = TrayIconBuilder::with_id("main").tooltip("Apex Overlays").menu(&menu).on_menu_event(|app, ev| {
        let hub = app.state::<Core>().hub.clone();
        match ev.id.as_ref() {
            "settings" => show_settings(app),
            "edit" => hub.set_edit_mode(!hub.edit_mode.load(Ordering::Relaxed)),
            "visible" => hub.set_visible(!hub.visible.load(Ordering::Relaxed)),
            "quit" => {
                flush_geometry(app);
                app.exit(0);
            }
            _ => {}
        }
    });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

fn main() {
    let mock = std::env::args().any(|a| a == "--mock");
    let data_dir = apex_core::config::default_data_dir();
    let opts = |port| apex_core::Options { data_dir: data_dir.clone(), mock, ui_dir: None, port };
    // If the configured port is taken, fall back to any free port so the app still works.
    let core = apex_core::start(opts(None)).or_else(|_| apex_core::start(opts(Some(0)))).expect("failed to start Apex core");

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| show_settings(app)))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(Core { hub: core.hub.clone(), config: core.config.clone(), url: core.url(), port: core.addr.port(), pending: Mutex::new(HashMap::new()) })
        .setup(|app| {
            let handle = app.handle().clone();
            tray(&handle)?;
            register_hotkeys(&handle);
            reconcile(&handle);
            // Start with the settings window unless launched to the tray (e.g. at Windows start-up).
            if !std::env::args().any(|a| a == "--tray") {
                show_settings(&handle);
            }
            // React to config edits and requests coming from the overlay/settings pages.
            let (hub, config) = {
                let c = handle.state::<Core>();
                (c.hub.clone(), c.config.clone())
            };
            let h = handle.clone();
            tauri::async_runtime::spawn(async move {
                let mut host = hub.host_events();
                let mut cfg = config.subscribe();
                let mut tick = tokio_interval();
                loop {
                    tokio::select! {
                        ev = host.recv() => match ev {
                            Ok(ev) => handle_host_event(&h, ev),
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                            Err(_) => break,
                        },
                        r = cfg.changed() => {
                            if r.is_err() { break; }
                            reconcile(&h);
                        }
                        _ = tick.tick() => flush_geometry(&h),
                    }
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            let app = window.app_handle();
            let label = window.label().to_string();
            match event {
                WindowEvent::CloseRequested { api, .. } if label == SETTINGS => {
                    // Closing settings keeps the overlays running in the tray.
                    api.prevent_close();
                    let _ = window.hide();
                }
                WindowEvent::Moved(p) if label.starts_with(PREFIX) => {
                    let s = window.scale_factor().unwrap_or(1.0);
                    let l: LogicalPosition<f64> = p.to_logical(s);
                    record_geometry(app, &label, Some((l.x.round() as i32, l.y.round() as i32)), None);
                }
                WindowEvent::Resized(sz) if label.starts_with(PREFIX) => {
                    let s = window.scale_factor().unwrap_or(1.0);
                    let l: LogicalSize<f64> = sz.to_logical(s);
                    record_geometry(app, &label, None, Some((l.width.round() as u32, l.height.round() as u32)));
                }
                _ => {}
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building Apex Overlays")
        .run(|_, event| {
            if let RunEvent::ExitRequested { api, code, .. } = event {
                // Keep running in the tray when the last window closes; only "Quit" exits.
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}

fn tokio_interval() -> tokio::time::Interval {
    let mut i = tokio::time::interval(Duration::from_millis(400));
    i.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    i
}
