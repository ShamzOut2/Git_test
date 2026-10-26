# Roadmap

Done in 0.1:
- [x] Relative, standings, track map (full / flat / round), driver inputs, fuel calculator, battle box, session info,
      tyre (wet/dry) advisor, Garage 61
- [x] League builder (custom classes, driver overrides, import/export)
- [x] Desktop host (transparent click-through windows, edit mode, tray, hotkeys) and OBS browser sources
- [x] Built-in race simulator for development

Done in 0.2:
- [x] Per-overlay transparency (overall + background), colour themes (presets, saved custom themes) for overlays and settings
- [x] League import from iRacing by league ID (classes, nicknames, points system, championship standings)
- [x] Points systems by class or overall position, fastest-lap bonus, live points and projected championship
- [x] Relative/standings: customisable session header; flags, manufacturer, iRating/SR estimates, stint lap with OUTLAP,
      best/last lap, tyre, current lap, overall position
- [x] Track map corner labels (turn numbers and/or names), downloadable and user-editable

Done in 0.3:
- [x] Column titles, overlay-name headers with a session type badge, chequered flag for finishers
- [x] Relative gap modes (on track / to class leader / to car ahead), best & last lap colouring
- [x] Manual league setup with a driver roster pulled from the session, pole / laps-led / most-laps-led / finish bonuses

Next:
- [ ] **Stream chat overlay**: Twitch chat over the anonymous IRC WebSocket (no login needed) and YouTube live chat,
      as an in-game overlay and an OBS source
- [ ] **VR support**: render overlays into OpenXR/OpenVR overlay layers. Overlays are already plain pages with no host
      assumptions, so a VR host only needs off-screen web rendering and a texture upload
- [ ] Code-signed installer (removes the SmartScreen warning) and auto-update
- [ ] Delta bar (to session best, personal best and Garage 61 reference lap)
- [ ] Pit strategy: tyre-change timing, fuel to add pre-filled to the pit service, undercut/overcut estimates
- [ ] Flags / race control overlay, blind-spot (car left/right) indicator, radar
- [ ] Head-to-head lap comparison with sector times
- [ ] IBT replay source (same file format as the live feed) for building layouts from saved sessions
- [ ] Layout profiles per car or session type, switched automatically
- [ ] Shareable overlay presets / theme gallery
- [ ] Plug-in folder for user-made overlays loaded at runtime
