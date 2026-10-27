# Published data

## iracing-track-maps.json

Where iRacing's official track map of each track lives: track ID → map folder and SVG layer
files (from the iRacing Data API's `track/assets`). The SVG files themselves are public; only
this index needs an iRacing login. Apex Overlays downloads this file so every user gets the
official maps without setting up a login.

To refresh it (new tracks or layouts): set up the iRacing login in a copy of Apex Overlays
(League builder → Import from iRacing), drive or join a session on any track, then copy
`%APPDATA%\Apex Overlays\iracing\iracing-track-maps.json` here and commit it.

## trackmaps/

The shared track map library: outlines recorded by drivers (one clean lap each), so other
users get a map for these tracks without recording one. `index.json` lists the tracks;
each `<trackId>.json` is one outline in Apex Overlays' own format. A user's own recording
always takes priority, and so does iRacing's official map when it is available.

To add a track: drive a clean lap so Apex Overlays records it, then take
`%APPDATA%\Apex Overlays\trackmaps\<trackId>.json`, copy it here (set `"source"` to
`"library"`) and add the track to `index.json`.
