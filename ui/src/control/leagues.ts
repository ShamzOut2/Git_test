// League builder: custom classes (by car, number, driver, team, ...) and
// driver display overrides. Applied live to standings, relative, map, etc.

import { h } from '../lib/dom';
import type { ClassRules, League, LeagueClass } from '../lib/types';
import type { Page } from './pages';
import { download, save, state, uid, upload } from './state';

let selected = '';

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

const emptyRules = (): ClassRules => ({ carIds: [], carNames: [], carNumbers: [], customerIds: [], teamNames: [], driverNames: [], iracingClassIds: [] });

function btn(label: string, fn: () => void, cls = '') {
  const b = h('button', cls, label);
  b.onclick = (e) => {
    e.preventDefault();
    fn();
  };
  return b;
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
    const r = h('label', 'field');
    r.append(
      h('span', 'field-label', label),
      listInput(c.rules[key] as (string | number)[], kind, ph, (v) => {
        (c.rules[key] as (string | number)[]) = v;
        save();
      }),
    );
    rules.append(r);
  }
  el.append(head, rules);
  return el;
}

function sessionDrivers(l: League, rerender: () => void) {
  const rows = state.standings?.classes.flatMap((c) => c.rows) ?? [];
  if (!rows.length) return h('p', 'dim', 'Join a session (or run the mock) to assign the drivers in it to classes with one click.');
  const table = h('table', 'drivers');
  const head = h('tr');
  for (const t of ['#', 'Driver', 'Car', 'Customer ID', 'Class']) head.append(h('th', '', t));
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
      const target = l.classes.find((c) => c.id === sel.value);
      target?.rules.customerIds.push(r.userId);
      save();
      rerender();
    };
    const tdSel = h('td');
    tdSel.append(sel);
    tr.append(h('td', '', r.number), h('td', '', r.name), h('td', '', r.car), h('td', 'dim', String(r.userId)), tdSel);
    table.append(tr);
  }
  return table;
}

function overridesEditor(l: League, rerender: () => void) {
  const wrap = h('div');
  const table = h('table', 'drivers');
  const head = h('tr');
  for (const t of ['Customer ID', 'Display name', 'Team name', 'Hide', '']) head.append(h('th', '', t));
  table.append(head);
  l.driverOverrides.forEach((o, i) => {
    const tr = h('tr');
    const cell = (el: HTMLElement) => {
      const td = h('td');
      td.append(el);
      tr.append(td);
    };
    const id = h('input');
    id.type = 'number';
    id.value = String(o.customerId || '');
    id.oninput = () => {
      o.customerId = parseInt(id.value, 10) || 0;
      save();
    };
    const dn = h('input');
    dn.value = o.displayName;
    dn.oninput = () => {
      o.displayName = dn.value;
      save();
    };
    const tn = h('input');
    tn.value = o.teamName;
    tn.oninput = () => {
      o.teamName = tn.value;
      save();
    };
    const hide = h('input');
    hide.type = 'checkbox';
    hide.checked = o.hidden;
    hide.onchange = () => {
      o.hidden = hide.checked;
      save();
    };
    cell(id);
    cell(dn);
    cell(tn);
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
      l.driverOverrides.push({ customerId: 0, displayName: '', teamName: '', hidden: false });
      save();
      rerender();
    }),
  );
  return wrap;
}

export const leaguesPage: Page = {
  id: 'leagues',
  title: 'League builder',
  render(el) {
    const cfg = state.config!;
    const rerender = () => this.render(el);
    if (!cfg.leagues.some((l) => l.id === selected)) selected = cfg.activeLeague ?? cfg.leagues[0]?.id ?? '';

    const top = h('section', 'card');
    top.append(h('h2', '', 'League builder'), h('p', 'dim', 'Create your own classes for league racing — group cars by number, driver, team or car model and give each class its name and colour. The active league replaces iRacing’s classes in every overlay.'));
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
    const activeRow = h('label', 'field');
    activeRow.append(h('span', 'field-label', 'Active league'), active);
    const bar = h('div', 'toolbar');
    bar.append(
      btn('New league', () => {
        const l: League = { id: uid('league'), name: 'My league', classes: [], driverOverrides: [], fallback: 'iracing' };
        cfg.leagues.push(l);
        selected = l.id;
        save();
        rerender();
      }, 'primary'),
      btn('Import league', async () => {
        const l = (await upload()) as League;
        if (!l || !Array.isArray(l.classes)) return alert('That file is not a league export.');
        l.id = uid('league');
        cfg.leagues.push(l);
        selected = l.id;
        save();
        rerender();
      }),
    );
    for (const l of cfg.leagues) {
      const b = btn(l.name || '(unnamed)', () => {
        selected = l.id;
        rerender();
      }, `tab${l.id === selected ? ' sel' : ''}`);
      bar.append(b);
    }
    top.append(activeRow, bar);

    const l = cfg.leagues.find((x) => x.id === selected);
    if (!l) {
      el.replaceChildren(top);
      return;
    }
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
    const fbRow = h('label', 'field');
    fbRow.append(h('span', 'field-label', 'Cars matching no class'), fallback);
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
    ed.append(h('h2', '', 'League'), name, fbRow, actions, h('p', 'dim small', 'A car joins the first class (top to bottom) where any rule matches. Text rules are case-insensitive; use * as a wildcard. Separate values with commas.'));
    l.classes.forEach((c, i) => ed.append(classEditor(l, c, i, rerender)));

    const assign = h('section', 'card');
    assign.append(h('h2', '', 'Assign drivers from the current session'), sessionDrivers(l, rerender));
    const ov = h('section', 'card');
    ov.append(h('h2', '', 'Driver overrides'), h('p', 'dim', 'Rename drivers or teams (e.g. league nicknames) or hide spectators/admins from every overlay.'), overridesEditor(l, rerender));
    el.replaceChildren(top, ed, assign, ov);
  },
};
