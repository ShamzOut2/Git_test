// League builder: custom classes (by car, number, driver, team, ...), points
// systems and driver overrides. Leagues can be imported from iRacing by league
// ID or created from the session you are in. Applied live to every overlay.

import { h } from '../lib/dom';
import type { ClassRules, League, LeagueClass, PointsConfig } from '../lib/types';
import { flagEmoji } from '../overlays/common';
import type { Page } from './pages';
import { client, download, save, state, uid, upload } from './state';

let selected = '';
let importLeagueId = '';
let importSeasonId = '';

const RULES: [keyof ClassRules, string, 'num' | 'text', string][] = [
  ['carNumbers', 'Car numbers', 'text', '1, 7, 42'],
  ['customerIds', 'iRacing customer IDs', 'num', '123456, 234567'],
  ['driverNames', 'Driver names', 'text', 'Max*, *Smith'],
  ['teamNames', 'Team names', 'text', '*Racing Academy*'],
  ['carNames', 'Car names', 'text', 'Porsche 911*, *GT3*'],
  ['carIds', 'iRacing car IDs', 'num', '169, 173'],
  ['iracingClassIds', 'iRacing class IDs', 'num', '4083'],
];

const PALETTE = ['#ff4d6d', '#3fa7ff', '#ffd23f', '#3ddc84', '#b26bff', '#ff8c42', '#2ee6d6', '#f2f4f8'];

const POINT_PRESETS: [string, number[]][] = [
  ['F1 (top 10)', [25, 18, 15, 12, 10, 8, 6, 4, 2, 1]],
  ['F1 sprint (top 8)', [8, 7, 6, 5, 4, 3, 2, 1]],
  ['MotoGP (top 15)', [25, 20, 16, 13, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1]],
  ['IndyCar (top 25)', [50, 40, 35, 32, 30, 28, 26, 24, 22, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5]],
  ['Linear 30 → 1', Array.from({ length: 30 }, (_, i) => 30 - i)],
];

const emptyRules = (): ClassRules => ({ carIds: [], carNames: [], carNumbers: [], customerIds: [], teamNames: [], driverNames: [], iracingClassIds: [] });
const emptyPoints = (): PointsConfig => ({ table: [], defaultPoints: 0, basis: 'class', fastestLapBonus: 0, systemName: '', description: '' });
const newLeague = (name: string): League => ({
  id: uid('league'),
  name,
  classes: [],
  driverOverrides: [],
  fallback: 'iracing',
  iracingLeagueId: 0,
  seasonId: 0,
  seasonName: '',
  points: emptyPoints(),
  seasonPoints: [],
  importedAt: '',
});

function btn(label: string, fn: () => void, cls = '') {
  const b = h('button', cls, label);
  b.onclick = (e) => {
    e.preventDefault();
    fn();
  };
  return b;
}

function labeled(label: string, el: HTMLElement, help?: string) {
  const r = h('label', 'field');
  const l = h('span', 'field-label', label);
  if (help) l.append(h('small', 'help', help));
  r.append(l, el);
  return r;
}

function listInput(value: (string | number)[], kind: 'num' | 'text', placeholder: string, onChange: (v: (string | number)[]) => void) {
  const i = h('input');
  i.value = value.join(', ');
  i.placeholder = placeholder;
  i.oninput = () => {
    const parts = i.value
      .split(',')
      .map((s) => s.trim())
      .filter(Boolean);
    onChange(kind === 'num' ? parts.map(Number).filter(Number.isFinite) : parts);
  };
  return i;
}

function classEditor(l: League, c: LeagueClass, i: number, rerender: () => void) {
  const el = h('div', 'league-class');
  el.style.setProperty('--class', c.color);
  const head = h('div', 'lc-head');
  const name = h('input', 'title-input');
  name.value = c.name;
  name.placeholder = 'Class name';
  name.oninput = () => {
    c.name = name.value;
    save();
  };
  const color = h('input');
  color.type = 'color';
  color.value = c.color;
  color.oninput = () => {
    c.color = color.value;
    el.style.setProperty('--class', c.color);
    save();
  };
  const move = (d: number) => () => {
    const [x] = l.classes.splice(i, 1);
    l.classes.splice(i + d, 0, x);
    save();
    rerender();
  };
  const up = btn('▲', move(-1), 'mini');
  const down = btn('▼', move(1), 'mini');
  up.disabled = i === 0;
  down.disabled = i === l.classes.length - 1;
  head.append(
    color,
    name,
    up,
    down,
    btn(
      'Remove',
      () => {
        l.classes.splice(i, 1);
        save();
        rerender();
      },
      'danger mini',
    ),
  );
  const rules = h('div', 'rules');
  for (const [key, label, kind, ph] of RULES) {
    rules.append(
      labeled(
        label,
        listInput(c.rules[key] as (string | number)[], kind, ph, (v) => {
          (c.rules[key] as (string | number)[]) = v;
          save();
        }),
      ),
    );
  }
  el.append(head, rules);
  return el;
}

/** Login + league ID import from the iRacing Data API. */
function importCard(rerender: () => void) {
  const cfg = state.config!;
  const api = cfg.iracingApi;
  const configured = !!(api.clientId && api.username && api.clientSecretMasked && api.passwordMasked);
  const c = h('section', 'card');
  c.append(
    h('h2', '', 'Import a league from iRacing'),
    h('p', 'dim', 'Enter a league ID to pull its name, roster nicknames, car classes, points system and championship standings straight from iRacing.'),
  );
  const creds = h('details');
  creds.open = !configured;
  creds.append(h('summary', '', configured ? `iRacing API login: ${api.username} (saved)` : 'iRacing API login (required once)'));
  creds.append(
    h(
      'p',
      'dim small',
      'iRacing only allows Data API access through OAuth. You need a client ID and secret registered with iRacing (request a “password limited” client from iRacing support) plus your iRacing email and password. Your password and secret are stored only in the scrambled form iRacing’s login expects, never as plain text.',
    ),
  );
  const field = (label: string, type: string, value: string, placeholder: string) => {
    const i = h('input');
    i.type = type;
    i.value = value;
    i.placeholder = placeholder;
    creds.append(labeled(label, i));
    return i;
  };
  const clientId = field('Client ID', 'text', api.clientId, 'my-client-id');
  const secret = field('Client secret', 'password', '', api.clientSecretMasked ? 'saved — type to replace' : 'client secret');
  const user = field('iRacing email', 'email', api.username, 'you@example.com');
  const pass = field('iRacing password', 'password', '', api.passwordMasked ? 'saved — type to replace' : 'password');
  creds.append(
    btn(
      'Save login',
      () => {
        client.send({ op: 'setIracingApi', clientId: clientId.value, clientSecret: secret.value, username: user.value, password: pass.value });
        secret.value = '';
        pass.value = '';
      },
      'primary',
    ),
  );
  const row = h('div', 'toolbar');
  const id = h('input');
  id.type = 'number';
  id.placeholder = 'League ID';
  id.style.width = '140px';
  id.value = importLeagueId || (state.session?.leagueId ? String(state.session.leagueId) : '');
  id.oninput = () => (importLeagueId = id.value);
  const imp = state.leagueImport;
  const season = h('select');
  const auto = h('option', '', 'Current season');
  auto.value = '';
  season.append(auto);
  for (const s of imp?.leagueId === Number(id.value) ? imp.seasons : []) {
    const o = h('option', '', `${s.name}${s.active ? ' (active)' : ''}`);
    o.value = String(s.seasonId);
    season.append(o);
  }
  season.value = importSeasonId;
  season.onchange = () => (importSeasonId = season.value);
  row.append(
    id,
    season,
    btn(
      'Import / refresh',
      () => {
        const leagueId = parseInt(id.value, 10);
        if (!leagueId) return alert('Enter the league ID (the number in the league page URL on iracing.com).');
        importLeagueId = id.value;
        client.send({ op: 'importLeague', leagueId, seasonId: importSeasonId ? parseInt(importSeasonId, 10) : null });
      },
      'primary',
    ),
  );
  c.append(creds, row);
  if (imp) {
    const status = h('div', 'notice', imp.message);
    if (imp.status === 'ok' && imp.id) {
      status.append(
        ' ',
        btn('Open it', () => {
          selected = imp.id!;
          rerender();
        }, 'mini'),
      );
    }
    c.append(status);
  }
  c.append(h('p', 'dim small', 'No API access? Use “Create from current session” above while you are in a league session, then add the points table by hand.'));
  return c;
}

function pointsCard(l: League, rerender: () => void) {
  l.points ??= emptyPoints();
  const p = l.points;
  const c = h('section', 'card');
  c.append(h('h2', '', 'Points system'));
  if (p.systemName || p.description) c.append(h('p', 'dim', `Imported: ${p.systemName}${p.description ? ` — ${p.description}` : ''}`));
  const basis = h('select');
  for (const [v, t] of [
    ['class', 'Class position (multi-class leagues)'],
    ['overall', 'Overall position'],
  ]) {
    const o = h('option', '', t);
    o.value = v;
    basis.append(o);
  }
  basis.value = p.basis || 'class';
  basis.onchange = () => {
    p.basis = basis.value as PointsConfig['basis'];
    save();
  };
  const num = (value: number, set: (v: number) => void) => {
    const i = h('input');
    i.type = 'number';
    i.step = '0.5';
    i.value = String(value);
    i.oninput = () => {
      set(parseFloat(i.value) || 0);
      save();
    };
    return i;
  };
  c.append(
    labeled('Points are scored by', basis, 'Leagues differ: some score by class finishing position, some overall'),
    labeled(
      'Fastest lap bonus',
      num(p.fastestLapBonus, (v) => (p.fastestLapBonus = v)),
      'Given to the fastest lap of the class (or overall, matching the setting above)',
    ),
    labeled('Points beyond the table', num(p.defaultPoints, (v) => (p.defaultPoints = v)), 'For positions after the last one listed'),
  );
  const table = h('div', 'points-table');
  p.table.forEach((v, i) => {
    const cell = h('label', '', `P${i + 1}`);
    cell.append(num(v, (x) => (p.table[i] = x)));
    table.append(cell);
  });
  const presets = h('select');
  presets.append(h('option', '', 'Load a preset…'));
  for (const [name] of POINT_PRESETS) presets.append(h('option', '', name));
  presets.onchange = () => {
    const preset = POINT_PRESETS.find(([n]) => n === presets.value);
    if (preset) {
      p.table = [...preset[1]];
      save();
      rerender();
    }
  };
  const paste = h('input');
  paste.placeholder = 'or paste: 25-18-15-12-10…';
  paste.onchange = () => {
    const nums = paste.value
      .split(/[^0-9.]+/)
      .filter(Boolean)
      .map(Number)
      .filter(Number.isFinite);
    if (nums.length) {
      p.table = nums;
      save();
      rerender();
    }
  };
  const bar = h('div', 'toolbar');
  bar.append(
    btn('Add position', () => {
      p.table.push(Math.max(0, (p.table[p.table.length - 1] ?? 1) - 1));
      save();
      rerender();
    }),
    btn('Remove last', () => {
      p.table.pop();
      save();
      rerender();
    }),
    presets,
    paste,
  );
  c.append(table, bar, h('p', 'dim small', 'Add the “League points” and “Championship total” columns to the Relative or Standings overlay to see live points.'));
  if (l.seasonPoints?.length) {
    const top = [...l.seasonPoints].sort((a, b) => b.points - a.points).slice(0, 10);
    const t = h('table', 'drivers');
    t.append(...top.map((e, i) => {
      const tr = h('tr');
      tr.append(h('td', '', String(e.position || i + 1)), h('td', '', e.name), h('td', '', String(e.points)));
      return tr;
    }));
    c.append(h('h3', '', `Championship standings ${l.seasonName ? `(${l.seasonName})` : ''}`), t);
  }
  return c;
}

function sessionDrivers(l: League, rerender: () => void) {
  const rows = state.standings?.classes.flatMap((c) => c.rows) ?? [];
  if (!rows.length) return h('p', 'dim', 'Join a session (or run the mock) to assign the drivers in it to classes with one click.');
  const table = h('table', 'drivers');
  const head = h('tr');
  for (const t of ['#', '', 'Driver', 'Car', 'Customer ID', 'Class']) head.append(h('th', '', t));
  table.append(head);
  for (const r of rows.sort((a, b) => a.position - b.position)) {
    const tr = h('tr');
    const sel = h('select');
    sel.append(h('option', '', '—'));
    for (const c of l.classes) {
      const o = h('option', '', c.name || '(unnamed)');
      o.value = c.id;
      if (c.rules.customerIds.includes(r.userId)) o.selected = true;
      sel.append(o);
    }
    sel.onchange = () => {
      for (const c of l.classes) c.rules.customerIds = c.rules.customerIds.filter((x) => x !== r.userId);
      l.classes.find((c) => c.id === sel.value)?.rules.customerIds.push(r.userId);
      save();
      rerender();
    };
    const tdSel = h('td');
    tdSel.append(sel);
    tr.append(h('td', '', r.number), h('td', 'flag', flagEmoji(r.country)), h('td', '', r.name), h('td', '', r.car), h('td', 'dim', String(r.userId)), tdSel);
    table.append(tr);
  }
  return table;
}

function overridesEditor(l: League, rerender: () => void) {
  const wrap = h('div');
  const table = h('table', 'drivers');
  const head = h('tr');
  for (const t of ['Customer ID', 'Display name', 'Team name', 'Flag (ISO code)', 'Hide', '']) head.append(h('th', '', t));
  table.append(head);
  l.driverOverrides.forEach((o, i) => {
    const tr = h('tr');
    const cell = (el: HTMLElement) => {
      const td = h('td');
      td.append(el);
      tr.append(td);
    };
    const input = (value: string, set: (v: string) => void, placeholder = '') => {
      const el = h('input');
      el.value = value;
      el.placeholder = placeholder;
      el.oninput = () => {
        set(el.value);
        save();
      };
      return el;
    };
    cell(input(o.customerId ? String(o.customerId) : '', (v) => (o.customerId = parseInt(v, 10) || 0)));
    cell(input(o.displayName, (v) => (o.displayName = v)));
    cell(input(o.teamName, (v) => (o.teamName = v)));
    cell(input(o.country ?? '', (v) => (o.country = v.trim().toUpperCase()), 'GB, DE, US…'));
    const hide = h('input');
    hide.type = 'checkbox';
    hide.checked = o.hidden;
    hide.onchange = () => {
      o.hidden = hide.checked;
      save();
    };
    cell(hide);
    cell(
      btn(
        '✕',
        () => {
          l.driverOverrides.splice(i, 1);
          save();
          rerender();
        },
        'mini',
      ),
    );
    table.append(tr);
  });
  wrap.append(
    table,
    btn('Add driver override', () => {
      l.driverOverrides.push({ customerId: 0, displayName: '', teamName: '', country: '', hidden: false });
      save();
      rerender();
    }),
  );
  return wrap;
}

/** A league built from the session you are in: its iRacing classes and league ID. */
function fromSession(): League | null {
  const st = state.standings;
  if (!st?.classes.length) return null;
  const l = newLeague(state.session?.leagueId ? `League ${state.session.leagueId}` : `${state.session?.trackName ?? 'Session'} league`);
  l.iracingLeagueId = state.session?.leagueId ?? 0;
  st.classes.forEach((c, i) => {
    const irClass = /^ir(\d+)$/.exec(c.id);
    const carIds = [...new Set(c.rows.map((r) => r.carId))];
    l.classes.push({
      id: uid('c'),
      name: c.name,
      color: c.color && c.color !== '#ffffff' ? c.color : PALETTE[i % PALETTE.length],
      rules: { ...emptyRules(), iracingClassIds: irClass ? [Number(irClass[1])] : [], carIds: irClass ? [] : carIds },
    });
  });
  l.points.basis = l.classes.length > 1 ? 'class' : 'overall';
  return l;
}

export const leaguesPage: Page = {
  id: 'leagues',
  title: 'League builder',
  render(el) {
    const cfg = state.config!;
    const rerender = () => this.render(el);
    if (!cfg.leagues.some((l) => l.id === selected)) selected = cfg.activeLeague ?? cfg.leagues[0]?.id ?? '';

    const top = h('section', 'card');
    top.append(
      h('h2', '', 'League builder'),
      h('p', 'dim', 'Create your own classes for league racing — group cars by number, driver, team or car model and give each class its name and colour, set the points system, and see live points in the relative and standings. The active league replaces iRacing’s classes in every overlay.'),
    );
    const active = h('select');
    const none = h('option', '', 'None (use iRacing classes)');
    none.value = '';
    active.append(none);
    for (const l of cfg.leagues) {
      const o = h('option', '', l.name || '(unnamed league)');
      o.value = l.id;
      active.append(o);
    }
    active.value = cfg.activeLeague ?? '';
    active.onchange = () => {
      cfg.activeLeague = active.value || null;
      save();
    };
    const bar = h('div', 'toolbar');
    const add = (l: League) => {
      cfg.leagues.push(l);
      selected = l.id;
      save();
      rerender();
    };
    bar.append(
      btn('New league', () => add(newLeague('My league')), 'primary'),
      btn('Create from current session', () => {
        const l = fromSession();
        if (!l) return alert('Join a session first — the classes and drivers of that session are used.');
        add(l);
      }),
      btn('Import league file', async () => {
        const l = (await upload()) as League;
        if (!l || !Array.isArray(l.classes)) return alert('That file is not a league export.');
        add({ ...newLeague(l.name), ...l, id: uid('league') });
      }),
    );
    for (const l of cfg.leagues) {
      bar.append(
        btn(l.name || '(unnamed)', () => {
          selected = l.id;
          rerender();
        }, `tab${l.id === selected ? ' sel' : ''}`),
      );
    }
    top.append(labeled('Active league', active), bar);

    const l = cfg.leagues.find((x) => x.id === selected);
    if (!l) {
      el.replaceChildren(top, importCard(rerender));
      return;
    }
    l.driverOverrides ??= [];
    const ed = h('section', 'card');
    const name = h('input', 'title-input');
    name.value = l.name;
    name.oninput = () => {
      l.name = name.value;
      save();
    };
    const fallback = h('select');
    for (const [v, t] of [
      ['iracing', 'Keep their iRacing class'],
      ['other', 'Put them in an “Other” class'],
    ]) {
      const o = h('option', '', t);
      o.value = v;
      fallback.append(o);
    }
    fallback.value = l.fallback || 'iracing';
    fallback.onchange = () => {
      l.fallback = fallback.value as League['fallback'];
      save();
    };
    const actions = h('div', 'toolbar');
    actions.append(
      btn('Add class', () => {
        l.classes.push({ id: uid('c'), name: `Class ${l.classes.length + 1}`, color: PALETTE[l.classes.length % PALETTE.length], rules: emptyRules() });
        save();
        rerender();
      }, 'primary'),
      btn('Export league', () => download(`${l.name || 'league'}.json`, l)),
      btn('Delete league', () => {
        if (!confirm(`Delete ${l.name}?`)) return;
        cfg.leagues = cfg.leagues.filter((x) => x !== l);
        if (cfg.activeLeague === l.id) cfg.activeLeague = null;
        save();
        rerender();
      }, 'danger'),
    );
    const meta = l.iracingLeagueId ? h('p', 'dim small', `iRacing league ${l.iracingLeagueId}${l.seasonName ? ` · ${l.seasonName}` : ''}${l.importedAt ? ` · imported ${new Date(Number(l.importedAt) * 1000).toLocaleString()}` : ''}`) : h('span');
    ed.append(
      h('h2', '', 'League'),
      name,
      meta,
      labeled('Cars matching no class', fallback),
      actions,
      h('p', 'dim small', 'A car joins the first class (top to bottom) where any rule matches. Text rules are case-insensitive; use * as a wildcard. Separate values with commas.'),
    );
    l.classes.forEach((c, i) => ed.append(classEditor(l, c, i, rerender)));

    const assign = h('section', 'card');
    assign.append(h('h2', '', 'Assign drivers from the current session'), sessionDrivers(l, rerender));
    const ov = h('section', 'card');
    ov.append(h('h2', '', 'Driver overrides'), h('p', 'dim', 'Rename drivers or teams (e.g. league nicknames), set the flag shown for a driver, or hide spectators/admins from every overlay.'), overridesEditor(l, rerender));
    el.replaceChildren(top, importCard(rerender), ed, pointsCard(l, rerender), assign, ov);
  },
};
