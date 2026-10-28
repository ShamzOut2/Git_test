import { h, show, style, text, toggle } from '../lib/dom';
import * as f from '../lib/format';
import { headerSettings, sessionHeader } from '../lib/header';
import type { OverlayDef } from '../lib/overlay';
import { columns, enabledColumns, NAME_STYLES, type SettingDef } from '../lib/settings';
import { type Col, columnFitter, Table } from '../lib/table';
import type { Approach, Driver, Extras, Hazard, Safety, Session, Status } from '../lib/types';
import { COLUMN_STYLE_SETTINGS, driverCols, extraCols, tireCell } from './common';

// Built like the standings: a section per alert (header bar + table) with the same columns,
// column styles, flag marks and colours, so the two read the same on screen.

const AHEAD_COLS = [
  { id: 'alert', label: 'Flag (yellow / meatball; also shown before the name) — FLAG' },
  { id: 'pos', label: 'Class position — POS' },
  { id: 'overall', label: 'Overall position — OVR' },
  { id: 'number', label: 'Number — #' },
  { id: 'flag', label: 'Country flag — NAT' },
  { id: 'name', label: 'Driver' },
  { id: 'team', label: 'Team' },
  { id: 'make', label: 'Manufacturer — MAKE' },
  { id: 'class', label: 'Class' },
  { id: 'car', label: 'Car' },
  { id: 'license', label: 'Licence — LIC' },
  { id: 'irating', label: 'iRating — IR' },
  { id: 'reason', label: 'What is happening — STATUS' },
  { id: 'srDelta', label: 'Safety rating gain/loss (est.) — SR ±' },
  { id: 'incidents', label: 'Incidents — INC' },
  { id: 'gap', label: 'Gap ahead (seconds) — GAP' },
  { id: 'distance', label: 'Distance ahead — DIST' },
  { id: 'speed', label: 'Their speed — SPD' },
  { id: 'lastLap', label: 'Last lap — LAST' },
  { id: 'bestLap', label: 'Best lap — BEST' },
  { id: 'lap', label: 'Current lap — LAP' },
  { id: 'stint', label: 'Stint lap / OUTLAP / PIT — STINT' },
  { id: 'tire', label: 'Tyre (wet/dry) — TYRE' },
];

const BEHIND_COLS = [
  { id: 'class', label: 'Class' },
  { id: 'pos', label: 'Class position — POS' },
  { id: 'overall', label: 'Overall position — OVR' },
  { id: 'number', label: 'Number — #' },
  { id: 'flag', label: 'Country flag — NAT' },
  { id: 'name', label: 'Driver' },
  { id: 'team', label: 'Team' },
  { id: 'make', label: 'Manufacturer — MAKE' },
  { id: 'car', label: 'Car' },
  { id: 'license', label: 'Licence — LIC' },
  { id: 'irating', label: 'iRating — IR' },
  { id: 'srDelta', label: 'Safety rating gain/loss (est.) — SR ±' },
  { id: 'incidents', label: 'Incidents — INC' },
  { id: 'gap', label: 'Gap behind (seconds) — GAP' },
  { id: 'distance', label: 'Distance behind — DIST' },
  { id: 'fasterBy', label: 'How much faster a lap — FASTER' },
  { id: 'catchIn', label: 'Time until they reach you — REACH' },
  { id: 'speed', label: 'Their speed — SPD' },
  { id: 'lastLap', label: 'Last lap — LAST' },
  { id: 'bestLap', label: 'Best lap — BEST' },
  { id: 'stint', label: 'Stint lap / OUTLAP / PIT — STINT' },
  { id: 'tire', label: 'Tyre (wet/dry) — TYRE' },
];

const REASONS: Record<Hazard['reason'], string> = { stopped: 'Stopped', slow: 'Slow', offTrack: 'Off track', rejoining: 'Rejoining', damaged: 'Damaged' };

const sample = (d: Partial<Driver & Extras>): Driver & Extras => ({
  idx: 0, number: '', name: '', abbrev: '', team: '', car: '', classId: '', className: '', classColor: '#888888', userId: 0, carId: 0,
  irating: 0, license: '', licenseColor: '#0153db', club: '', flair: '', country: '', make: '', isPlayer: false, isFocus: false,
  lap: 12, stintState: 'lap', stintLap: 6, bestLap: 101.4, lastLap: 103.2, classBest: false, lastIsPb: false, finished: false, lapsLed: 0,
  flags: [], penalty: null, penaltyServed: false, incidents: 4, irDelta: null, srDelta: null, points: null, seasonPoints: null, ...d,
});

/** Shown while positioning the overlay: a stopped car and a meatball ahead, a faster class behind. */
const SAMPLE: Safety = {
  hazards: [
    { ...sample({ idx: 61, number: '27', name: 'Tom Walker', car: 'BMW M2 CS', className: 'M2', classColor: '#3aa0ff', make: 'BMW', irating: 2140, license: 'A 3.12', country: 'GB', flags: ['yellow'], incidents: 6 }), flag: 'yellow', reason: 'stopped', gap: 4.6, distance: 212, speed: 0.4, position: 14, classPosition: 6, tireWet: false },
    { ...sample({ idx: 62, number: '9', name: 'Kenji Sato', car: 'BMW M2 CS', className: 'M2', classColor: '#3aa0ff', make: 'BMW', irating: 2950, license: 'A 3.40', country: 'JP', flags: ['repair'] }), flag: 'repair', reason: 'damaged', gap: 7.9, distance: 371, speed: 41, position: 9, classPosition: 4, tireWet: false },
  ],
  approaching: [
    { ...sample({ idx: 63, number: '3', name: 'Lena Fischer', car: 'Porsche 911 GT3 R (992)', className: 'GT3', classId: 'gt3', classColor: '#ff5a36', make: 'Porsche', irating: 3380, license: 'A 4.40', country: 'DE' }), gap: 2.1, distance: 104, fasterBy: 6.4, catchIn: 28, speed: 61, position: 2, classPosition: 2, tireWet: false },
  ],
};

const TITLE_TEXT: SettingDef[] = [
  { key: 'yellowTitle', label: 'Title: yellow ahead', type: 'text', default: 'YELLOW AHEAD', group: 'Header' },
  { key: 'meatballTitle', label: 'Title: meatball ahead', type: 'text', default: 'MEATBALL AHEAD', group: 'Header' },
  { key: 'behindTitle', label: 'Title: faster class behind', type: 'text', default: 'FASTER CLASS BEHIND', group: 'Header' },
];

interface Section<R> {
  el: HTMLElement;
  head: HTMLElement;
  icon: HTMLElement;
  name: HTMLElement;
  meta: HTMLElement;
  table: Table<R>;
}

export const safety: OverlayDef = {
  kind: 'safety',
  name: 'Safety alert',
  description:
    'Stays hidden until something needs your attention: cars ahead causing a yellow (stopped, slow, off track) or carrying the meatball, until you have passed them, and cars of a faster class closing from behind. Same columns, flags and styles as the standings.',
  topics: ['safety', 'session', 'status'],
  size: [520, 200],
  fitsWidth: true,
  commonDefaults: { hideWhenNotDriving: true },
  settings: [
    { key: 'showHazards', label: 'Warn about yellows and meatballs ahead', type: 'bool', default: true, group: 'Layout' },
    { key: 'aheadSeconds', label: 'Look this far ahead', type: 'number', default: 10, min: 1, max: 30, step: 0.5, unit: 's', group: 'Layout' },
    { key: 'yellow', label: 'Cars causing a yellow', type: 'bool', default: true, group: 'Layout', help: 'Stopped, slow, off track or rejoining' },
    { key: 'meatball', label: 'Cars with the meatball', type: 'bool', default: true, group: 'Layout', help: 'Damaged cars told to pit for repairs' },
    { key: 'maxHazards', label: 'Rows ahead', type: 'number', default: 3, min: 1, max: 4, group: 'Layout' },
    { key: 'showApproach', label: 'Warn about faster classes behind', type: 'bool', default: true, group: 'Layout' },
    { key: 'behindSeconds', label: 'Look this far behind', type: 'number', default: 5, min: 1, max: 30, step: 0.5, unit: 's', group: 'Layout' },
    { key: 'maxApproach', label: 'Rows behind', type: 'number', default: 2, min: 1, max: 4, group: 'Layout' },
    { key: 'order', label: 'Order', type: 'select', default: 'ahead', options: [['ahead', 'Cars ahead first'], ['behind', 'Faster class behind first']], group: 'Layout' },
    { key: 'holdMs', label: 'Keep showing after it clears', type: 'number', default: 800, min: 0, max: 5000, step: 100, unit: 'ms', group: 'Layout', help: 'Stops the alert flickering at the edge of the window' },
    { key: 'aheadColumns', label: 'Columns: cars ahead', type: 'columns', ...columns(AHEAD_COLS, ['number', 'flag', 'name', 'license', 'irating', 'reason', 'gap', 'distance']), group: 'Columns' },
    { key: 'behindColumns', label: 'Columns: faster class behind', type: 'columns', ...columns(BEHIND_COLS, ['class', 'number', 'flag', 'name', 'license', 'irating', 'gap', 'distance', 'catchIn']), group: 'Columns' },
    { key: 'nameStyle', label: 'Name format', type: 'select', default: 'full', options: NAME_STYLES, group: 'Columns' },
    { key: 'gapDecimals', label: 'Gap decimals', type: 'number', default: 1, min: 0, max: 3, group: 'Columns' },
    ...COLUMN_STYLE_SETTINGS,
    ...headerSettings(['time', 'lap']).map((d): SettingDef => (d.key === 'showHeader' && d.type === 'bool' ? { ...d, default: false } : d)),
    { key: 'showSectionHeader', label: 'Show alert titles (flag and title bar)', type: 'bool', default: true, group: 'Header' },
    ...TITLE_TEXT,
    { key: 'showColumnTitles', label: 'Show column titles', type: 'bool', default: false, group: 'Header' },
    { key: 'yellowColor', label: 'Yellow', type: 'color', default: '#ffd400', group: 'Colours' },
    { key: 'meatballColor', label: 'Meatball', type: 'color', default: '#ff7a00', group: 'Colours' },
    { key: 'behindClassColor', label: 'Faster class in its class colour', type: 'bool', default: true, group: 'Colours' },
    { key: 'behindColor', label: 'Faster class colour (when not the class colour)', type: 'color', default: '#4da3ff', group: 'Colours' },
    { key: 'animate', label: 'Wave the flag', type: 'bool', default: true, group: 'Colours' },
    { key: 'pulse', label: 'Pulse the alert title', type: 'bool', default: true, group: 'Colours' },
  ],
  create(root, ctx) {
    const s = ctx.settings;
    const dec = s.gapDecimals as number;
    const preview = new URLSearchParams(location.search).has('preview');
    root.classList.add('safety');
    toggle(root, 'still', !s.animate);
    toggle(root, 'pulse', !!s.pulse);
    const header = sessionHeader(s, 'Safety alert', () => ctx.units);
    const body = h('div', 'sections');
    root.append(header.el, body);

    const units = () => ctx.units;
    const dist = (m: number) => (units() === 'imperial' ? `${Math.round(m * 3.28084)} ft` : `${Math.round(m)} m`);
    const spd = (ms: number) => `${Math.round(f.speed(ms, units()))}`;
    const shared = <R extends Driver & Extras & { position: number; classPosition: number; gap: number; distance: number; speed: number; tireWet: boolean | null }>(): Col<R>[] => [
      { id: 'pos', label: 'Pos', width: '2.4em', align: 'center', render: (c, r) => text(c, r.classPosition ? String(r.classPosition) : '') },
      { id: 'overall', label: 'Ovr', width: '2.4em', align: 'center', render: (c, r) => text(c, r.position ? String(r.position) : '') },
      ...driverCols<R>(s),
      { id: 'gap', label: 'Gap', width: '3.6em', align: 'right', render: (c, r) => text(c, `${r.gap.toFixed(dec)}s`) },
      { id: 'distance', label: 'Dist', width: '4em', align: 'right', render: (c, r) => text(c, dist(r.distance)) },
      { id: 'speed', label: f.speedUnit(units()), width: '3em', align: 'right', render: (c, r) => text(c, spd(r.speed)) },
      { id: 'tire', label: 'Tyre', width: '2.4em', align: 'center', render: (c, r) => tireCell(c, r.tireWet) },
      ...extraCols<R>(s),
    ];
    const aheadCols: Col<Hazard>[] = [
      {
        id: 'alert',
        label: 'Flag',
        width: '2.2em',
        align: 'center',
        render: (c, r) => {
          let m = c.firstElementChild as HTMLElement | null;
          if (!m) {
            m = h('span', 'mk');
            c.append(m);
          }
          const cls = `mk mk-${r.flag} sa-mk`;
          if (m.className !== cls) m.className = cls;
        },
      },
      {
        id: 'reason',
        label: 'Status',
        width: '5em',
        render: (c, r) => text(c, REASONS[r.reason]),
      },
      ...shared<Hazard>(),
    ];
    const behindCols: Col<Approach>[] = [
      ...shared<Approach>(),
      { id: 'fasterBy', label: 'Faster', width: '4.4em', align: 'right', render: (c, r) => text(c, r.fasterBy > 0 ? `−${r.fasterBy.toFixed(1)}s` : '') },
      { id: 'catchIn', label: 'Reach', width: '3.6em', align: 'right', render: (c, r) => text(c, r.catchIn == null ? '' : r.catchIn < 3 ? 'now' : `~${r.catchIn}s`) },
    ];

    const section = <R,>(cols: Col<R>[], order: string[], kind: string): Section<R> => {
      const el = h('div', `section sa-${kind}`);
      const head = h('div', 'class-head sa-head');
      const left = h('span', 'class-left');
      const icon = h('span', 'sa-icon');
      const name = h('span', 'class-name');
      const meta = h('span', 'info');
      left.append(icon, name);
      head.append(left, meta);
      show(head, s.showSectionHeader as boolean);
      const table = new Table<R>(cols, order, s.showColumnTitles as boolean);
      el.append(head, table.el);
      return { el, head, icon, name, meta, table };
    };
    const ahead = section(aheadCols, enabledColumns(s.aheadColumns), 'ahead');
    const behind = section(behindCols, enabledColumns(s.behindColumns), 'behind');
    ahead.icon.className = 'sa-icon mk';
    behind.icon.innerHTML = `<svg viewBox="0 0 40 40" aria-hidden="true"><path d="M8 22 20 10l12 12M8 33 20 21l12 12" fill="none" stroke="currentColor" stroke-width="6" stroke-linecap="round" stroke-linejoin="round"/></svg>`;
    body.append(...(s.order === 'behind' ? [behind.el, ahead.el] : [ahead.el, behind.el]));
    style(root, '--sa-yellow', s.yellowColor as string);
    style(root, '--sa-meatball', s.meatballColor as string);

    const fit = columnFitter();
    let live: Safety | null = null;
    let editing = false;
    let clearTimer = 0;
    let shown = false;

    const pick = (d: Safety | null) => {
      const hz = s.showHazards
        ? (d?.hazards ?? []).filter((z) => z.gap <= (s.aheadSeconds as number) && (z.flag === 'repair' ? s.meatball : s.yellow)).slice(0, s.maxHazards as number)
        : [];
      const ap = s.showApproach ? (d?.approaching ?? []).filter((a) => a.gap <= (s.behindSeconds as number)).slice(0, s.maxApproach as number) : [];
      return { hz, ap };
    };

    const draw = (hz: Hazard[], ap: Approach[]) => {
      show(ahead.el, hz.length > 0);
      show(behind.el, ap.length > 0);
      if (hz.length) {
        const yellow = hz.some((z) => z.flag === 'yellow');
        const meat = hz.some((z) => z.flag === 'repair');
        const flag = yellow ? 'yellow' : 'repair';
        ahead.icon.className = `sa-icon mk mk-${flag}`;
        text(ahead.name, yellow && meat ? `${s.yellowTitle} · ${s.meatballTitle}` : ((yellow ? s.yellowTitle : s.meatballTitle) as string));
        style(ahead.head, '--class', flag === 'yellow' ? (s.yellowColor as string) : (s.meatballColor as string));
        text(ahead.meta, hz.length > 1 ? `${hz.length} cars · nearest ${hz[0].gap.toFixed(dec)}s` : `${hz[0].gap.toFixed(dec)}s · ${dist(hz[0].distance)}`);
        ahead.table.render(hz, () => ({}), (row, r) => row.style.setProperty('--class', r.classColor));
      }
      if (ap.length) {
        const c = s.behindClassColor ? ap[0].classColor : (s.behindColor as string);
        style(behind.head, '--class', c);
        style(behind.icon, 'color', c);
        text(behind.name, s.behindTitle as string);
        text(behind.meta, [...new Set(ap.map((a) => a.className))].join(' · '));
        behind.table.render(ap, () => ({}), (row, r) => row.style.setProperty('--class', r.classColor));
      }
      shown = hz.length + ap.length > 0;
      root.classList.toggle('auto-hidden', !shown);
      if (s.autoWidth && shown) fit([ahead.table as Table<unknown>, behind.table as Table<unknown>]);
    };

    const update = () => {
      let { hz, ap } = pick(live);
      if ((editing || preview) && !hz.length && !ap.length) ({ hz, ap } = pick(SAMPLE));
      if (hz.length || ap.length) {
        clearTimeout(clearTimer);
        clearTimer = 0;
        draw(hz, ap);
      } else if (shown && !clearTimer) {
        clearTimer = window.setTimeout(() => {
          clearTimer = 0;
          draw([], []);
        }, s.holdMs as number);
      } else if (!shown) {
        root.classList.add('auto-hidden');
      }
    };

    return {
      update(topic, data) {
        if (topic === 'session') return header.update(data as Session);
        if (topic === 'safety') live = data as Safety;
        if (topic === 'status') editing = !!(data as Status).editMode;
        update();
      },
    };
  },
};
