import { h } from '../lib/dom';
import { COMMON, merge, type SettingDef } from '../lib/settings';
import { applyUiTheme, PRESETS, THEME } from '../lib/theme';
import type { OverlayInstance } from '../lib/types';
import { byKind, OVERLAYS } from '../overlays/registry';
import { form } from './form';
import { client, download, save, state, uid, upload } from './state';

export type Page = { id: string; title: string; render(el: HTMLElement): void };

const card = (title: string, ...children: (Node | string)[]) => {
  const c = h('section', 'card');
  c.append(h('h2', '', title), ...children);
  return c;
};

const button = (label: string, onClick: () => void, cls = '') => {
  const b = h('button', cls, label);
  b.onclick = (e) => {
    e.preventDefault();
    onClick();
  };
  return b;
};

export const overlayUrl = (id: string) => `${location.origin}/o/${encodeURIComponent(id)}`;

let selected = '';

function copy(text: string, btn: HTMLButtonElement) {
  navigator.clipboard?.writeText(text).then(() => {
    const old = btn.textContent;
    btn.textContent = 'Copied';
    setTimeout(() => (btn.textContent = old), 1200);
  });
}

function overlayEditor(o: OverlayInstance, rerender: () => void): HTMLElement {
  const def = byKind(o.kind)!;
  const wrap = h('div', 'editor');
  const head = h('div', 'editor-head');
  const title = h('input', 'title-input');
  title.value = o.name;
  title.placeholder = def.name;
  title.oninput = () => {
    o.name = title.value;
    save();
  };
  const enabled = h('label', 'inline');
  const cb = h('input');
  cb.type = 'checkbox';
  cb.className = 'switch';
  cb.checked = o.enabled;
  cb.onchange = () => {
    o.enabled = cb.checked;
    save();
    rerender();
  };
  enabled.append(cb, ' Enabled');
  const actions = h('div', 'actions');
  const url = overlayUrl(o.id);
  const copyBtn = button('Copy OBS URL', () => copy(url, copyBtn));
  actions.append(
    copyBtn,
    button('Duplicate', () => {
      const c = structuredClone(o);
      c.id = uid(o.kind);
      c.name = `${o.name || def.name} copy`;
      c.x += 30;
      c.y += 30;
      state.config!.overlays.push(c);
      selected = c.id;
      save();
      rerender();
    }),
    button('Reset settings', () => {
      if (!confirm('Reset this overlay to default settings?')) return;
      o.settings = {};
      save();
      rerender();
    }),
    button(
      'Delete',
      () => {
        if (!confirm(`Delete ${o.name || def.name}?`)) return;
        state.config!.overlays = state.config!.overlays.filter((x) => x !== o);
        selected = '';
        save();
        rerender();
      },
      'danger',
    ),
  );
  head.append(title, enabled, actions);

  const geo = h('div', 'geo');
  for (const k of ['x', 'y', 'width', 'height'] as const) {
    const l = h('label', 'inline', k === 'width' ? 'W' : k === 'height' ? 'H' : k.toUpperCase());
    const i = h('input');
    i.type = 'number';
    i.value = String(o[k]);
    i.dataset.geo = k;
    i.oninput = () => {
      const v = parseInt(i.value, 10);
      if (Number.isFinite(v)) {
        o[k] = v;
        save();
      }
    };
    l.append(i);
    geo.append(l);
  }
  geo.append(h('span', 'dim small', 'Tip: turn on Edit mode to drag overlays on screen.'));

  const preview = h('div', 'preview');
  const frame = h('iframe');
  frame.src = `${url}?preview`;
  const scale = Math.min(1, 520 / o.width, 300 / o.height);
  frame.style.width = `${o.width}px`;
  frame.style.height = `${o.height}px`;
  frame.style.transform = `scale(${scale})`;
  preview.style.height = `${o.height * scale + 16}px`;
  preview.append(frame);

  const values = merge([...COMMON, ...def.settings], o.settings);
  const transparencyKeys = ['opacity', 'bgOpacity'];
  const quick = h('div', 'quick');
  quick.append(
    form(
      COMMON.filter((x) => transparencyKeys.includes(x.key)),
      values,
      (k, v) => {
        o.settings = { ...o.settings, [k]: v };
        save();
      },
    ),
    button('Apply this transparency to all overlays', () => {
      for (const other of state.config!.overlays) other.settings = { ...other.settings, opacity: values.opacity, bgOpacity: values.bgOpacity };
      save();
    }),
  );
  const settings = form(
    [...def.settings, ...COMMON.filter((x) => !transparencyKeys.includes(x.key))],
    values,
    (k, v) => {
      values[k] = v;
      o.settings = { ...o.settings, [k]: v };
      save();
    },
  );
  wrap.append(head, h('p', 'dim', def.description), geo, quick, preview, settings);
  return wrap;
}

const overlaysPage: Page = {
  id: 'overlays',
  title: 'Overlays',
  render(el) {
    const cfg = state.config!;
    const rerender = () => this.render(el);
    if (!cfg.overlays.some((o) => o.id === selected)) selected = cfg.overlays[0]?.id ?? '';
    const bar = h('div', 'toolbar');
    const editing = state.status?.editMode ?? false;
    const visible = state.status?.visible ?? true;
    bar.append(
      button(editing ? 'Exit edit mode' : 'Edit mode (move & resize)', () => client.send({ op: 'setEditMode', value: !editing }), editing ? 'primary active' : 'primary'),
      button(visible ? 'Hide all overlays' : 'Show overlays', () => client.send({ op: 'setVisible', value: !visible })),
    );
    const add = h('select');
    add.append(h('option', '', 'Add overlay…'));
    for (const d of OVERLAYS) {
      const opt = h('option', '', d.name);
      opt.value = d.kind;
      add.append(opt);
    }
    add.onchange = () => {
      const d = byKind(add.value);
      if (!d) return;
      const o: OverlayInstance = { id: uid(d.kind), kind: d.kind, name: '', enabled: true, x: 200, y: 200, width: d.size[0], height: d.size[1], settings: {} };
      cfg.overlays.push(o);
      selected = o.id;
      save();
      rerender();
    };
    bar.append(add);

    const list = h('div', 'ov-list');
    for (const o of cfg.overlays) {
      const d = byKind(o.kind);
      const item = h('button', `ov-item${o.id === selected ? ' sel' : ''}${o.enabled ? '' : ' off'}`);
      item.append(h('span', 'ov-name', o.name || d?.name || o.kind), h('span', 'ov-kind', d?.name ?? o.kind));
      item.onclick = () => {
        selected = o.id;
        rerender();
      };
      list.append(item);
    }
    const sel = cfg.overlays.find((o) => o.id === selected);
    const main = h('div', 'ov-main');
    main.append(sel ? overlayEditor(sel, rerender) : h('p', 'dim', 'No overlays yet — add one above.'));
    const layout = h('div', 'ov-layout');
    layout.append(list, main);
    el.replaceChildren(bar, layout);
  },
};

function swatch(name: string, theme: Record<string, unknown>, onApply: () => void, onDelete?: () => void) {
  const t = merge(THEME, theme);
  const el = h('button', 'swatch');
  el.style.background = t.background as string;
  el.style.color = t.textColor as string;
  const bars = h('div', 'swatch-bars');
  for (const k of ['accent', 'playerColor', 'goodColor', 'badColor', 'lapBehindColor']) {
    const b = h('span');
    b.style.background = t[k] as string;
    bars.append(b);
  }
  el.append(h('span', 'swatch-name', name), bars);
  el.onclick = (e) => {
    e.preventDefault();
    onApply();
  };
  if (onDelete) {
    const x = h('span', 'swatch-x', '✕');
    x.title = 'Delete';
    x.onclick = (e) => {
      e.stopPropagation();
      onDelete();
    };
    el.append(x);
  }
  return el;
}

const COLOUR_KEYS = THEME.filter((x) => x.type === 'color').map((x) => x.key);

const themePage: Page = {
  id: 'theme',
  title: 'Theme',
  render(el) {
    const cfg = state.config!;
    const rerender = () => this.render(el);
    applyUiTheme(cfg.theme, document.documentElement);
    const values = merge(THEME, cfg.theme);
    cfg.savedThemes ??= [];
    const apply = (theme: Record<string, unknown>, coloursOnly: boolean) => {
      // Presets only carry colours, so fonts and sizes the user picked survive.
      const base = coloursOnly ? Object.fromEntries(Object.entries(cfg.theme).filter(([k]) => !COLOUR_KEYS.includes(k) && k !== 'backgroundOpacity' && k !== 'shadow')) : {};
      cfg.theme = { ...base, ...theme };
      save();
      rerender();
    };
    const presets = h('div', 'swatches');
    for (const p of PRESETS) presets.append(swatch(p.name, p.values, () => apply(p.values, true)));
    const saved = h('div', 'swatches');
    cfg.savedThemes.forEach((t, i) =>
      saved.append(
        swatch(
          t.name,
          t.theme,
          () => apply(t.theme, false),
          () => {
            cfg.savedThemes.splice(i, 1);
            save();
            rerender();
          },
        ),
      ),
    );
    if (!cfg.savedThemes.length) saved.append(h('p', 'dim small', 'No saved themes yet.'));
    const nameInput = h('input');
    nameInput.placeholder = 'Theme name';
    const saveRow = h('div', 'toolbar');
    saveRow.append(
      nameInput,
      button(
        'Save current theme',
        () => {
          const name = nameInput.value.trim() || `Theme ${cfg.savedThemes.length + 1}`;
          cfg.savedThemes = cfg.savedThemes.filter((t) => t.name !== name);
          cfg.savedThemes.push({ name, theme: { ...cfg.theme } });
          save();
          rerender();
        },
        'primary',
      ),
      button('Export theme', () => download('apex-overlays-theme.json', { name: nameInput.value.trim() || 'My theme', theme: cfg.theme })),
      button('Import theme', async () => {
        const t = (await upload()) as { name?: string; theme?: Record<string, unknown> };
        if (!t?.theme) return alert('That file is not an Apex Overlays theme.');
        cfg.savedThemes.push({ name: t.name || 'Imported theme', theme: t.theme });
        save();
        rerender();
      }),
    );
    el.replaceChildren(
      card(
        'Colour themes',
        h('p', 'dim', 'A theme colours every overlay and this settings window. Pick a preset, tweak it below, then save it as your own.'),
        h('h3', '', 'Presets'),
        presets,
        h('h3', '', 'My themes'),
        saved,
        saveRow,
      ),
      card(
        'Customise',
        form(THEME, values, (k, v) => {
          cfg.theme = { ...cfg.theme, [k]: v };
          save();
          applyUiTheme(cfg.theme, document.documentElement);
        }),
        button('Reset to default', () => apply({}, false)),
      ),
    );
  },
};

const WETNESS: [string, string][] = [
  ['1', '1 – Dry'],
  ['2', '2 – Mostly dry'],
  ['3', '3 – Very lightly wet'],
  ['4', '4 – Lightly wet'],
  ['5', '5 – Moderately wet'],
  ['6', '6 – Very wet'],
  ['7', '7 – Extremely wet'],
];

const strategyPage: Page = {
  id: 'strategy',
  title: 'Strategy',
  render(el) {
    const cfg = state.config!;
    const section = <T extends object>(title: string, obj: T, schema: SettingDef[], intro: string, map?: (k: string, v: unknown) => unknown) =>
      card(
        title,
        h('p', 'dim', intro),
        form(schema, obj as Record<string, unknown>, (k, v) => {
          (obj as Record<string, unknown>)[k] = map ? map(k, v) : v;
          save();
        }),
      );
    el.replaceChildren(
      section(
        'Fuel calculator',
        cfg.fuel,
        [
          { key: 'avgLaps', label: 'Laps averaged', type: 'number', default: 5, min: 1, max: 20 },
          { key: 'marginLaps', label: 'Safety margin', type: 'number', default: 1, min: 0, max: 5, step: 0.1, unit: 'laps' },
          { key: 'includePitLaps', label: 'Include in/out laps', type: 'bool', default: false },
          { key: 'includeCautionLaps', label: 'Include caution laps', type: 'bool', default: false },
        ],
        'Consumption is measured per lap. Laps with refuelling, off-track resets or (optionally) pit/caution laps are ignored.',
      ),
      section(
        'Tyre advisor',
        { ...cfg.tyres, wetThreshold: String(cfg.tyres.wetThreshold), dryThreshold: String(cfg.tyres.dryThreshold) },
        [
          { key: 'wetThreshold', label: 'Advise wets from', type: 'select', default: '4', options: WETNESS },
          { key: 'dryThreshold', label: 'Advise slicks at or below', type: 'select', default: '2', options: WETNESS },
          { key: 'useField', label: 'Use what my class is running', type: 'bool', default: true },
          { key: 'useCrossover', label: 'Compare wet vs dry lap times', type: 'bool', default: true },
          { key: 'crossoverMargin', label: 'Crossover margin', type: 'number', default: 0.5, min: 0, max: 5, step: 0.1, unit: 's/lap' },
        ],
        'The advisor scores track wetness, the rain trend, the field and lap-time crossover. Lower thresholds = earlier calls.',
        (k, v) => {
          const n = k.endsWith('Threshold') ? parseInt(v as string, 10) : v;
          (cfg.tyres as Record<string, unknown>)[k] = n;
          return v;
        },
      ),
      section(
        'Battle box',
        cfg.battle,
        [
          { key: 'maxGap', label: 'Battle range', type: 'number', default: 3, min: 0.2, max: 20, step: 0.1, unit: 's' },
          { key: 'sameClassOnly', label: 'Only cars in my class', type: 'bool', default: true },
        ],
        'Cars within the battle range are highlighted and their gap trend is tracked.',
      ),
    );
  },
};

const garage61Page: Page = {
  id: 'garage61',
  title: 'Garage 61',
  render(el) {
    const g = state.config!.garage61;
    const status = state.garage61;
    const statusLine = h('p', `status-line s-${status?.status ?? 'disabled'}`, status ? `${status.status}${status.message ? ' — ' + status.message : ''}${status.user ? ` (signed in as ${status.user})` : ''}` : 'disabled');
    const token = h('input');
    token.type = 'password';
    token.placeholder = g.token === '__keep__' ? 'Token saved — paste a new one to replace' : 'Paste your personal access token';
    token.value = '';
    token.onchange = () => {
      g.token = token.value.trim();
      save(true);
    };
    const tokRow = h('label', 'field');
    tokRow.append(h('span', 'field-label', 'Access token'), token);
    el.replaceChildren(
      card(
        'Garage 61',
        h('p', 'dim', 'Shows your Garage 61 personal best and your teams’ best laps for the car and track you are driving. Create a personal access token on garage61.net (Developer → My applications) with the “driving data” permission.'),
        form(
          [
            { key: 'enabled', label: 'Enable integration', type: 'bool', default: false },
            { key: 'includeTeam', label: 'Include team leaderboard', type: 'bool', default: true },
            { key: 'refreshMinutes', label: 'Refresh every', type: 'number', default: 5, min: 1, max: 60, unit: 'min' },
          ],
          g as unknown as Record<string, unknown>,
          (k, v) => {
            (g as unknown as Record<string, unknown>)[k] = v;
            save();
          },
        ),
        tokRow,
        statusLine,
        h('p', 'dim small', 'Remember to enable the “Garage 61” overlay on the Overlays page.'),
      ),
    );
  },
};

/** Track outline + corner list editor for the current track. */
function cornersCard(rerender: () => void): HTMLElement {
  const cfg = state.config!;
  const tm = state.trackmap;
  const c = card('Track map & corners');
  if (!tm?.trackId) {
    c.append(h('p', 'dim', 'Join a session to manage the track outline and corner names for that track.'));
    return c;
  }
  const key = String(tm.trackId);
  const custom = cfg.trackCorners?.[key];
  const sourceText: Record<string, string> = { custom: 'your custom list', lovely: 'Lovely Track Data (community)', mock: 'the built-in simulator', none: 'none found yet' };
  c.append(
    h('p', 'dim', `${tm.trackName}: outline ${tm.source ?? (tm.recording ? 'recording…' : 'not recorded')}. Corners from ${sourceText[tm.cornersSource] ?? tm.cornersSource}.`),
    button('Re-record track outline', () => client.send({ op: 'resetTrackMap' })),
  );
  const list = custom ?? tm.corners;
  const table = h('table', 'corners');
  const head = h('tr');
  for (const t of ['Turn', 'Name', 'Start %', 'End %', '']) head.append(h('th', '', t));
  table.append(head);
  list.forEach((corner, i) => {
    const tr = h('tr');
    const cell = (node: Node) => {
      const td = h('td');
      td.append(node);
      tr.append(td);
    };
    if (!custom) {
      tr.append(h('td', '', String(corner.number)), h('td', '', corner.name || '–'), h('td', '', (corner.start * 100).toFixed(1)), h('td', '', (corner.end * 100).toFixed(1)), h('td'));
      table.append(tr);
      return;
    }
    const num = h('input');
    num.type = 'number';
    num.value = String(corner.number);
    num.oninput = () => {
      corner.number = parseInt(num.value, 10) || 0;
      save();
    };
    const name = h('input', 'cname');
    name.value = corner.name;
    name.placeholder = 'e.g. Eau Rouge';
    name.oninput = () => {
      corner.name = name.value;
      save();
    };
    const pct = (field: 'start' | 'end') => {
      const inp = h('input');
      inp.type = 'number';
      inp.step = '0.1';
      inp.value = (corner[field] * 100).toFixed(1);
      inp.oninput = () => {
        const v = parseFloat(inp.value);
        if (Number.isFinite(v)) {
          corner[field] = Math.min(1, Math.max(0, v / 100));
          save();
        }
      };
      return inp;
    };
    cell(num);
    cell(name);
    cell(pct('start'));
    cell(pct('end'));
    cell(
      button(
        '✕',
        () => {
          custom.splice(i, 1);
          save();
          rerender();
        },
        'mini',
      ),
    );
    table.append(tr);
  });
  const bar = h('div', 'toolbar');
  if (!custom) {
    bar.append(
      button(
        list.length ? 'Edit these corners' : 'Create a corner list',
        () => {
          cfg.trackCorners = { ...(cfg.trackCorners ?? {}), [key]: list.map((x) => ({ ...x })) };
          save();
          rerender();
        },
        'primary',
      ),
    );
  } else {
    bar.append(
      button(
        'Add corner at my car’s position',
        () => {
          const p = state.player?.lapPct ?? 0;
          custom.push({ name: '', number: (custom.reduce((m, x) => Math.max(m, x.number), 0) || 0) + 1, start: Math.max(0, p - 0.005), end: Math.min(1, p + 0.005) });
          custom.sort((a, b) => a.start - b.start);
          save();
          rerender();
        },
        'primary',
      ),
      button('Revert to downloaded corners', () => {
        const next = { ...cfg.trackCorners };
        delete next[key];
        cfg.trackCorners = next;
        save();
        rerender();
      }),
    );
  }
  c.append(table, bar, h('p', 'dim small', 'Tip: drive to a corner, then click “Add corner at my car’s position”. Corner labels are switched on in the Track map overlay settings.'));
  return c;
}

const generalPage: Page = {
  id: 'general',
  title: 'General',
  render(el) {
    const cfg = state.config!;
    const opts = {
      units: cfg.units,
      port: cfg.server.port,
      lanAccess: cfg.server.lanAccess,
      toggleEdit: cfg.hotkeys.toggleEdit,
      toggleVisibility: cfg.hotkeys.toggleVisibility,
      ratingsOfficialOnly: cfg.ratingsOfficialOnly,
    };
    el.replaceChildren(
      card(
        'General',
        form(
          [
            { key: 'units', label: 'Units', type: 'select', default: 'auto', options: [['auto', 'Same as iRacing'], ['metric', 'Metric'], ['imperial', 'Imperial']], group: 'Display' },
            {
              key: 'ratingsOfficialOnly',
              label: 'Only estimate iRating / SR in official races',
              type: 'bool',
              default: true,
              help: 'Leagues and hosted sessions do not change iRating or SR',
              group: 'Display',
            },
            { key: 'toggleEdit', label: 'Toggle edit mode', type: 'text', default: 'CmdOrCtrl+Shift+E', group: 'Hotkeys (desktop app, restart to apply)' },
            { key: 'toggleVisibility', label: 'Show / hide overlays', type: 'text', default: 'CmdOrCtrl+Shift+H', group: 'Hotkeys (desktop app, restart to apply)' },
            { key: 'port', label: 'Port', type: 'number', default: 8182, min: 1024, max: 65535, group: 'Server (restart to apply)' },
            { key: 'lanAccess', label: 'Allow other PCs on my network (streaming PC)', type: 'bool', default: false, group: 'Server (restart to apply)' },
          ],
          opts,
          (k, v) => {
            if (k === 'units') cfg.units = v as typeof cfg.units;
            if (k === 'port') cfg.server.port = v as number;
            if (k === 'lanAccess') cfg.server.lanAccess = v as boolean;
            if (k === 'toggleEdit') cfg.hotkeys.toggleEdit = v as string;
            if (k === 'toggleVisibility') cfg.hotkeys.toggleVisibility = v as string;
            if (k === 'ratingsOfficialOnly') cfg.ratingsOfficialOnly = v as boolean;
            save();
          },
        ),
      ),
      cornersCard(() => this.render(el)),
      card(
        'Backup',
        h('p', 'dim', 'Export everything (overlays, theme, leagues) to a file, or import a file shared by someone else. Your Garage 61 token is never exported.'),
        button('Export settings', () => download('apex-overlays.json', { ...cfg, garage61: { ...cfg.garage61, token: '' } })),
        button('Import settings', async () => {
          const data = (await upload()) as typeof cfg;
          if (!data || !Array.isArray(data.overlays)) return alert('That file is not an Apex Overlays export.');
          state.config = { ...data, garage61: { ...data.garage61, token: cfg.garage61.token } };
          save(true);
          this.render(el);
        }),
        button(
          'Reset everything',
          () => {
            if (!confirm('Reset all settings to defaults?')) return;
            fetch('/api/config', { method: 'PUT', headers: { 'content-type': 'application/json' }, body: '{}' }).then(() => location.reload());
          },
          'danger',
        ),
      ),
    );
  },
};

const streamingPage: Page = {
  id: 'streaming',
  title: 'Streaming / OBS',
  render(el) {
    const rows = state.config!.overlays.map((o) => {
      const tr = h('tr');
      const url = overlayUrl(o.id);
      const b = button('Copy', () => copy(url, b));
      const code = h('code', '', url);
      tr.append(h('td', '', o.name || byKind(o.kind)?.name || o.kind), h('td', '', `${o.width}×${o.height}`), h('td'), h('td'));
      tr.children[2].append(code);
      tr.children[3].append(b);
      return tr;
    });
    const table = h('table', 'urls');
    table.append(...rows);
    el.replaceChildren(
      card(
        'Overlays as browser sources',
        h('p', 'dim', 'Every overlay is a web page. In OBS add a Browser Source with the URL and size below — the same settings and data as in-game, transparent background included.'),
        table,
        h('p', 'dim small', 'Streaming from a second PC: enable “Allow other PCs on my network” under General, restart, and replace 127.0.0.1 with this PC’s IP address.'),
      ),
      card('Coming next', h('p', 'dim', 'Stream chat overlay (Twitch / YouTube), VR overlay support and more are on the roadmap — see docs/ROADMAP.md.')),
    );
  },
};

export const PAGES: Page[] = [overlaysPage, themePage, strategyPage, garage61Page, generalPage, streamingPage];
