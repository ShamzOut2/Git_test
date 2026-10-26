# Apex Overlays

A lightweight, fully customisable overlay suite for **iRacing**. It reads telemetry at the sim's native **60 Hz**,
does all the race maths once in a small native core (Rust), and draws each overlay as a minimal web page
in a transparent, click-through window. The same pages work as **OBS browser sources** for streaming.

| Relative | Standings (with league classes) | Track map |
|---|---|---|
| ![Relative](docs/images/relative.png) | ![Standings](docs/images/standings.png) | ![Track map](docs/images/trackmap.png) |

| Fuel calculator | Battle box | Tyre advisor |
|---|---|---|
| ![Fuel](docs/images/fuel.png) | ![Battle](docs/images/battle.png) | ![Tyres](docs/images/tyres.png) |

![Driver inputs](docs/images/inputs.png)
![Session info](docs/images/session.png)

## Overlays

| Overlay | What it shows |
|---|---|
| **Relative** | Cars around you on track with live gaps, class colour, licence/iRating, pit/off-track flags and colours for cars a lap ahead or behind. You choose and reorder the columns. |
| **Standings** | Multi-class leaderboard: gap to leader, interval, last/best lap (class-fastest highlighted), laps, pit stops, stint length, positions gained, tyre. It shows the top rows and a window around you, per class or as one overall list. |
| **Track map** | Three styles: the **full circuit** (outline recorded automatically from your first clean lap and cached per track), a **flat** line (optionally one lane per class) and a **round** map. Includes sectors, start/finish, class-leader rings, labels, rotation and mirroring. |
| **Driver inputs** | Throttle, brake and clutch bars, a scrolling input trace (ABS shown in colour), steering wheel, gear and speed. |
| **Fuel calculator** | Average/last/max use per lap, laps of fuel, race laps left (lap- or time-limited, lapped-car aware), fuel to finish, fuel to add, stops, pit window and how much to save to skip a stop. |
| **Battle box** | The car ahead and behind in your class: gap, how fast it is closing (s/lap), laps until you catch them (or they catch you), and their last lap against yours. |
| **Tyre advisor** | Tells you when to change between slicks and wets (**BOX FOR WETS / BOX FOR SLICKS**). It combines track wetness, the rain trend, what your class is running and the wet/dry lap-time crossover, and explains why. |
| **Session info** | Track and air temperature (with trend), wetness, rain, declared wet, wind, humidity, sky, time or laps remaining, sim clock, SOF, incidents and flags. You can pick and order any of these fields. |
| **Garage 61** | Your Garage 61 personal best and your teams' best laps for the current car and track, compared with your session best. |

### Customisation

- **Every overlay** has its own scale, background opacity, visibility rules (hide when iRacing isn't running, when you're not in the car, on pit road, or outside races) and **custom CSS**.
- **Column and field overlays** (relative, standings, session info) let you toggle and reorder every column, plus set name formats, gap signs and decimals.
- **The global theme** sets font, size and weight, every colour (player, lapped, lapping, pit, fastest, good/bad, caution), corner radius, row shading and padding, outline and text shadow.
- **Multiple copies of the same overlay** can run side by side with different settings, for example a compact relative for VR-style layouts and a full one for streaming.
- **The League builder** creates your own classes and assigns cars to them by car number, customer ID, driver or team name (wildcards), car model, car ID or iRacing class ID. You can also assign drivers in the current session from a dropdown, rename drivers and teams, or hide spectators. Leagues import and export as JSON, so a league admin can share one file with everybody.
- **Strategy settings** cover the fuel averaging window and safety margin, the wet/dry thresholds and crossover margin for the tyre advisor, and the battle range.
- **Export and import** cover the whole setup.

## Getting started (Windows)

1. Download the latest build from the CI artifacts (`apex-overlays-windows`). It contains the installer (`bundle/nsis/*.exe`), the
   desktop app `apex-overlays.exe` and the headless `apex-server.exe`.
2. Start **Apex Overlays**. The settings window opens. The app lives in the system tray, and closing the settings window keeps the overlays running.
3. Press **Ctrl+Shift+E** (or *Edit mode* in settings) to drag and resize the overlays, then press it again to lock them. Locked overlays are click-through.
   Press **Ctrl+Shift+H** to show or hide everything. Both hotkeys can be changed.
4. Start iRacing. The overlays appear as soon as you are in a session.

WebView2 is already present on Windows 10/11. The installer fetches it if it is missing.

### Streaming (OBS)

Every overlay is a URL such as `http://127.0.0.1:8182/o/relative`. Add it as an OBS **Browser Source** with the overlay's size
(the *Streaming / OBS* page lists every URL). It has the same data, the same settings and a transparent background. To use a second streaming PC,
enable *Allow other PCs on my network* and use this PC's IP address.

### Without iRacing: the built-in simulator

`apex-overlays.exe --mock` (or `apex-server --mock`) runs a three-class race with fuel use, pit stops and a rain shower that
comes and goes. Use it to set up layouts or develop new overlays.

## Building from source

Requirements: Rust (stable) and Node.js 20 or newer. For the desktop app on Windows you also need the
[Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) (MSVC build tools + WebView2).

```sh
cd ui && npm ci && npm run build && cd ..   # overlay UI → ui/dist (embedded into the binaries)
cargo test                                  # core + SDK reader tests
cargo run -p apex-server -- --mock          # headless server + simulator → http://127.0.0.1:8182
# Windows desktop app:
cd apps/desktop && npx @tauri-apps/cli@2 build
```

For UI development, run `npm run watch` in `ui/` and `cargo run -p apex-server -- --mock --ui-dir ui/dist`, then reload the page.

## Project layout

```
crates/irsdk     iRacing shared-memory reader + tolerant session-info YAML parser (no dependencies)
crates/core      telemetry engine, calculators, league classes, Garage 61 client, HTTP/WebSocket server
apps/server      headless server binary (OBS / second PC / development)
apps/desktop     Tauri desktop host: transparent click-through overlay windows, tray, hotkeys
ui               overlays + settings panel (TypeScript, no framework, ~100 KB total)
docs             architecture, how to add an overlay, roadmap
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), [docs/ADDING_AN_OVERLAY.md](docs/ADDING_AN_OVERLAY.md) and
[docs/ROADMAP.md](docs/ROADMAP.md) (VR support, stream chat overlay and more).
