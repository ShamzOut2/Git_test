import { h, show, text } from '../lib/dom';
import * as f from '../lib/format';
import type { OverlayDef } from '../lib/overlay';
import { columns, enabledColumns, NAME_STYLES } from '../lib/settings';
import { type Col, Table } from '../lib/table';
import type { RelRow, Relative, Session } from '../lib/types';
import { driverCols, tireCell } from './common';

const COLS = [
  { id: 'pos', label: 'Pos' },
  { id: 'number', label: 'Number' },
  { id: 'name', label: 'Driver' },
  { id: 'team', label: 'Team' },
  { id: 'class', label: 'Class' },
  { id: 'car', label: 'Car' },
  { id: 'license', label: 'Licence' },
  { id: 'irating', label: 'iRating' },
  { id: 'lastLap', label: 'Last lap' },
  { id: 'tire', label: 'Tyre (wet/dry)' },
  { id: 'pit', label: 'Pit flag' },
  { id: 'gap', label: 'Gap' },
];

export const relative: OverlayDef = {
  kind: 'relative',
  name: 'Relative',
  description: 'Cars around you on track with live time gaps, lap-ahead/behind colouring and class colours.',
  topics: ['relative', 'session'],
  size: [460, 300],
  settings: [
    { key: 'carsAhead', label: 'Cars ahead', type: 'number', default: 4, min: 0, max: 10, group: 'Layout' },
    { key: 'carsBehind', label: 'Cars behind', type: 'number', default: 4, min: 0, max: 10, group: 'Layout' },
    { key: 'keepEmptyRows', label: 'Keep empty rows (fixed height)', type: 'bool', default: true, group: 'Layout' },
    { key: 'columns', label: 'Columns', type: 'columns', ...columns(COLS, ['pos', 'number', 'name', 'license', 'irating', 'pit', 'gap']), group: 'Columns' },
    { key: 'nameStyle', label: 'Name format', type: 'select', default: 'full', options: NAME_STYLES, group: 'Columns' },
    { key: 'gapSign', label: 'Gap sign', type: 'select', default: 'aheadNegative', options: [['abs', 'No sign'], ['aheadNegative', 'Ahead negative'], ['aheadPositive', 'Ahead positive']], group: 'Columns' },
    { key: 'gapDecimals', label: 'Gap decimals', type: 'number', default: 1, min: 0, max: 3, group: 'Columns' },
    { key: 'showHeader', label: 'Show header', type: 'bool', default: true, group: 'Header' },
    { key: 'showColumnTitles', label: 'Show column titles', type: 'bool', default: false, group: 'Header' },
    { key: 'lapColors', label: 'Colour cars a lap ahead / behind', type: 'bool', default: true, group: 'Colours' },
    { key: 'classStripe', label: 'Class colour stripe', type: 'bool', default: true, group: 'Colours' },
    { key: 'dimPitCars', label: 'Dim cars in the pits', type: 'bool', default: true, group: 'Colours' },
  ],
  create(root, ctx) {
    const s = ctx.settings;
    const header = h('div', 'header');
    const title = h('span', 'title', 'Relative');
    const info = h('span', 'info');
    header.append(title, info);
    show(header, s.showHeader as boolean);

    const cols: Col<RelRow>[] = [
      { id: 'pos', label: 'P', width: '2.2em', align: 'center', render: (c, r) => text(c, r.classPosition > 0 ? String(r.classPosition) : '') },
      ...driverCols<RelRow>(s.nameStyle as string),
      { id: 'lastLap', label: 'Last', width: '4.6em', align: 'right', render: (c, r) => text(c, r.lastLap ? f.lapTime(r.lastLap) : '') },
      { id: 'tire', label: 'T', width: '1.4em', align: 'center', render: (c, r) => tireCell(c, r.tireWet) },
      { id: 'pit', label: '', width: '2.4em', align: 'center', render: (c, r) => text(c, r.onPitRoad ? 'PIT' : r.offTrack ? 'OFF' : '') },
      {
        id: 'gap',
        label: 'Gap',
        width: '3.6em',
        align: 'right',
        render: (c, r) => {
          if (r.isFocus) return text(c, '');
          const v = s.gapSign === 'abs' ? Math.abs(r.gap) : s.gapSign === 'aheadNegative' ? -r.gap : r.gap;
          text(c, s.gapSign === 'abs' ? v.toFixed(s.gapDecimals as number) : f.signed(v, s.gapDecimals as number));
        },
      },
    ];
    const table = new Table<RelRow>(cols, enabledColumns(s.columns), s.showColumnTitles as boolean);
    root.append(header, table.el);
    root.classList.toggle('stripe', s.classStripe as boolean);

    const ahead = s.carsAhead as number;
    const behind = s.carsBehind as number;
    return {
      update(topic, data) {
        if (topic === 'session') {
          const d = data as Session;
          const left = d.timeRemain != null ? f.duration(d.timeRemain) : d.lapsRemain != null ? `${d.lapsRemain} laps` : '';
          text(title, d.sessionName || d.sessionType || 'Relative');
          text(info, [left, d.sof ? `SOF ${f.iRating(d.sof)}` : '', d.incidentLimit ? `${d.incidents}/${d.incidentLimit}x` : `${d.incidents}x`].filter(Boolean).join('  ·  '));
          return;
        }
        if (topic !== 'relative') return;
        const rows = (data as Relative).rows;
        const me = rows.findIndex((r) => r.isFocus);
        const slots: (RelRow | null)[] = [];
        for (let k = -ahead; k <= behind; k++) slots.push(me < 0 ? null : (rows[me + k] ?? null));
        const out = s.keepEmptyRows ? slots : slots.filter((r) => r != null);
        table.render(
          out,
          (r) => ({
            player: r.isFocus,
            'lap-ahead': (s.lapColors as boolean) && r.lapDiff > 0,
            'lap-behind': (s.lapColors as boolean) && r.lapDiff < 0,
            pit: (s.dimPitCars as boolean) && r.onPitRoad,
            off: r.offTrack,
          }),
          (row, r) => row.style.setProperty('--class', r.classColor),
        );
      },
    };
  },
};
