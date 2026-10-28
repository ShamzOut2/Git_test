# Changelog

The section for each version is shown in the in-app update pop-up and on the GitHub release.
Each version groups its changes under `### New features`, `### Improvements` and `### Fixes`
(plus `### Performance` or `### Removed` when needed).

## 0.5.9

### New features
- **Saved layouts**: save the overlays you have as a named layout (Race, Qualifying, Streaming…), set up others, and switch between them from Overlays → Layouts. Each layout remembers its own overlays, positions and settings
- **Session layouts**: tick P, Q, W or R next to a saved layout and it is switched to automatically when a Practice, Qualifying, Warmup or Race session starts
- **Share layouts**: export a saved layout to a file (⇩) and send it to other drivers; they import it as a new saved layout of their own, with its session types and, if they want, its colour theme
- **More hotkeys** (General → Hotkeys): next layout, one per saved layout, and one per overlay to show or hide it — all blank until you set them. Click a box (it shows Recording…), press the keys and it is saved straight away; a hotkey used twice is flagged
- **Monitoring** (General): live CPU and memory use of Apex Overlays and its overlay windows (with a one-minute CPU graph), your PC's CPU, data sent to the overlays, telemetry rate, engine time per update, windows open and uptime
- **Performance monitor overlay**: a small on-screen widget with the readings you pick and order; CPU turns amber or red above a level you choose
- **Flag abbreviations** (League builder → Drivers): every flag code the Flag box accepts, with the flag and country name — searchable, click a row to copy the code

### Improvements
- General is now the first page in the settings window
- Hotkeys apply straight away, no restart needed
- The update pop-up shows what's new grouped into new features, improvements and fixes

### Fixes
- iRating pill: the gain/loss (green ▲ / red ▼) now sits inside the pill with the rating

## 0.5.8

- iRating is shown in a soft frosted pill, with the estimated race gain (green ▲) or loss (red ▼) right next to it instead of in a separate column; switch it off with "iRating gain/loss next to iRating" in the Columns tab
- Columns: drag columns to rearrange them, or drag a hidden column into place (the ▲▼ buttons still work)
- Settings: long drop-down options no longer squash the setting's name (e.g. the overlay update rate under General)
- Closing the settings window now asks whether to close the overlays too or keep them running (from the tray icon), with "don't ask again"; the choice can be changed under General → When I close this window

## 0.5.7

- Shared track map library: tracks other drivers have recorded get their outline straight away, no clean lap needed (Road Atlanta, Brands Hatch, Virginia International Raceway and Charlotte Motor Speedway to start with). Your own recording always takes priority
- iRacing's official track maps: the list used to share them is now written as soon as the app starts (for when iRacing reopens API access)

## 0.5.6

- Track map: iRacing's official track maps no longer need the iRacing login — the app downloads the list of maps published with Apex Overlays, so every track has its real outline straight away instead of waiting for a clean lap (tracks missing from the list are still recorded from your driving)
- Overlay settings redesigned: each overlay's settings are split into tabs (What to show, Layout, Columns, Colours, Size & visibility…), with a search box, a live preview at the top and a ↺ button to put any setting back to its default; tabs with changed settings are marked
- Columns: the columns you show are listed in order (move or remove them), the others are one click away to add
- Overlay list: switch overlays on and off right in the list; "Add overlay" shows every overlay with what it does; moving all overlays and sharing layouts are in one "Layout & sharing" menu
- Faster: timing tables are sent at a sensible rate instead of 60 times a second (standings 10/s, relative 20/s, fuel and session 4/s; inputs, track map and spotter stay at 60/s) — about 70% less data, and the standings window uses about a fifth of the CPU it did; *General → Performance → Overlay update rate* switches back to everything 60 times a second
- Faster: overlays only read the newest update of each kind per screen refresh, skip drawing while hidden, and no longer receive the 60/s input data unless they need it; the settings window only receives live race data on the pages that show it

## 0.5.5

- New Pit lane overlay: a BRAKE sign and the distance to the pit entry line on the way in; your speed and a pit limiter light in the lane (solid green on, flashing red off); WAIT and a flashing GO in your box; the distance to the pit exit line; and a flashing "turn off pit limiter" once you cross it. It only appears when you head for the pits and hides 5 s after you leave
- New Incident tracker overlay: each incident with its likely type (off track, loss of control, wall, car contact), the points, who you made contact with, your total against the limit, and a replay button that jumps the iRacing replay to the moment (clickable while a replay is playing)
- Inputs: while ABS is working, the brake trace (thicker line and filled area) and the brake bar change to the ABS colour
- New Spotter overlay: left and right bars that only appear when a car is alongside you, showing where it overlaps your car; red when three wide or two cars on one side
- Garage 61: live delta to your Garage 61 best lap or your team's fastest lap, with sector deltas, using the lap's telemetry (downloaded once and cached)
- Garage 61: choose which laps are searched (all time, this season, this and last season, last 90/30/7 days); all time by default
- Garage 61: optionally only compare with laps driven in similar conditions (track temperature within a range, and the same dry or wet track)
- Garage 61: the team leaderboard shows the gap to the fastest lap, and the settings page tells you if your token is missing the driving data permission
- League class badges: Pro and Semi-Pro (also Semi Pro, SemiPro, Professional)
- League classes are matched by their name only: a league class with no matching badge shows its name instead of the badge of the cars in it (Pro and Semi Pro classes of GT3 cars showed the GT3 badge)

## 0.5.4

- League class badges: Gold, Silver, Bronze, Am and Pro-Am, coloured to match the league class colour; Amateur, Amatuer and Amz show as AM, Pro Am / ProAm / Pro-Amateur as PRO-AM
- Wide class badges are scaled to fit the class column instead of being cut off
- Shift lights: blank spots in a car's light bar no longer show as lights that are always on (Mercedes-AMG GT3 and GT4, BMW M4 GT3, Audi R8, McLaren 720S, Lamborghini Huracán, Acura NSX, Aston Martin Vantage GT4, Corvette C8.R and BMW M8 GTE, Mercedes W12/W13, Mustang GT4, Porsche 992 Cup); checked against all 84 cars in Lovely Car Data

## 0.5.3

- Fuel calculator remembers what each car uses at each track: in practice (or any session) it shows the usual consumption per lap, lap time and laps counted from earlier sessions, and plans with it until you've driven a clean lap
- Standings and other auto-sized overlays resize again after the app restarts or reconnects
- Team themes: Gassed Up Motorsports, VCR and JWB
- Standings class headers show just the class badge when there is one (the class name still shows for classes without a badge; the old badge-and-name look is an option)
- Shift lights are back to each car's own lights and shift points (the extra blue shift-point lights from 0.5.2 are gone), with a new light pattern option: as the car has it, left to right, right to left, outside in, or inside out

## 0.5.2

- Track map: iRacing's official track map outline, downloaded with your iRacing login (League builder → Import from iRacing), so the full circuit shows straight away; the outline recorded from your driving is still used without a login, or if you prefer it
- Class badges for 52 iRacing classes (GT3, LMP2, TCR, GTP, MX-5, oval and dirt classes…), coloured to match each class's colour in the session, in the standings class headers and the class column
- Shift lights finish with blue shift-point lights at the ideal shift RPM (in the middle for centre-out patterns), and only flash once you go past it
- Flags overlay: the writing on every flag is now readable (yellow, white and LED-style flags)
- iRating gain/loss estimate counts the whole field from the start, so it no longer shows large negative numbers while drivers are still loading in
- Before the start, the standings show the qualifying grid, including drivers who haven't loaded in yet; the grid is remembered if the app restarts

## 0.5.1

- Update pop-up: when a new version is out, Apex Overlays offers it at start-up — update now, remind me tomorrow or in a week, or skip that version
- Gaps in the relative, standings and battle box no longer jump when a car crosses the start/finish line
- League builder follows the session: a league is used only in sessions with its iRacing league ID (or always, if you choose)
- Shift lights in the driver inputs overlay, with per-gear shift points from Lovely Car Data
- Track map: corner names for more tracks, pit entry/exit markers and optional straight names
- Short class names (TCR, GT3, LMP2…), class logos in the standings, built-in manufacturer logos
- Flags for every country and territory, and a globe for the Global flair
- Column lists show each column's title (e.g. "— PTS")

## 0.5.0

- Track map sector colours (purple / green / yellow), driver input percentages, readable text on bright theme colours
- Spectated-car compass, D/W tyre badges, PIT tag next to names, auto-fit width, move all overlays together, overlay sharing

## 0.4.0

- Flags, weather and data box overlays, battle box modes, auto-fit height, timed-race laps (x/∞), in-app updates
- Driver flags and penalty box, race control page
