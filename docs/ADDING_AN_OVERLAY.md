# Adding an overlay

1. Create `ui/src/overlays/myoverlay.ts`:

```ts
import { h, text } from '../lib/dom';
import type { OverlayDef } from '../lib/overlay';
import type { Player } from '../lib/types';

export const myoverlay: OverlayDef = {
  kind: 'myoverlay',
  name: 'My overlay',
  description: 'Shown in the settings panel.',
  topics: ['player'],          // which data streams to receive
  size: [240, 80],             // default window size
  settings: [                  // rendered automatically in the settings panel
    { key: 'showKph', label: 'Show speed', type: 'bool', default: true, group: 'Elements' },
    { key: 'color', label: 'Colour', type: 'color', default: '#ff3d57', group: 'Colours' },
  ],
  create(root, ctx) {
    const el = h('div', 'big');
    el.style.color = ctx.settings.color as string;
    root.append(el);
    return {
      update(topic, data) {
        if (topic === 'player') text(el, `${Math.round((data as Player).speed * 3.6)} km/h`);
      },
    };
  },
};
```

2. Add it to `OVERLAYS` in `ui/src/overlays/registry.ts`.
3. Run `npm run build` in `ui/`. The overlay now shows up under *Add overlay…* and works in OBS at `/o/<id>`.

Notes:

- `create` runs again whenever the user changes a setting, so read `ctx.settings` once and don't handle changes yourself.
- `update` runs at most once per animation frame per topic. Use `text()`, `toggle()` and `style()` from `lib/dom`. They only
  touch the DOM when a value changes.
- Common options (scale, opacity, visibility rules, custom CSS) are added for you.
- Need new data? Add a field to an existing topic in `crates/core/src/topics.rs`, or add a new `Topic`, and mirror the type in
  `ui/src/lib/types.ts`.
