import { h, show, text, toggle } from '../lib/dom';
import * as f from '../lib/format';
import type { OverlayDef } from '../lib/overlay';
import type { Fuel } from '../lib/types';

const FIELDS: [string, string, boolean][] = [
  ['level', 'Fuel in tank', true],
  ['perLapAvg', 'Average per lap', true],
  ['expected', 'Usual for this car & track', true],
  ['perLapLast', 'Last lap', true],
  ['perLapMax', 'Max per lap', false],
  ['lapsInTank', 'Laps of fuel', true],
  ['lapsRemaining', 'Race laps left', true],
  ['fuelToFinish', 'Needed to finish', true],
  ['fuelToAdd', 'Add at next stop', true],
  ['stops', 'Stops required', true],
  ['window', 'Pit window', true],
  ['targetPerLap', 'Target per lap (no stop)', false],
  ['savePerLap', 'Save per lap to skip a stop', true],
  ['lapsPerTank', 'Laps per full tank', false],
  ['usePerHour', 'Use per hour', false],
];

export const fuel: OverlayDef = {
  kind: 'fuel',
  name: 'Fuel calculator',
  description: 'Live consumption (average, last, max), laps of fuel left, fuel needed to finish, how much to add, stops and pit window. Works for lap- and time-limited races.',
  topics: ['fuel', 'session'],
  size: [300, 230],
  settings: [
    ...FIELDS.map(([key, label, on]) => ({ key: `show_${key}`, label, type: 'bool' as const, default: on, group: 'Fields' })),
    { key: 'showGauge', label: 'Tank gauge', type: 'bool', default: true, group: 'Elements' },
    { key: 'showHistory', label: 'Per-lap history bars', type: 'bool', default: true, group: 'Elements' },
    { key: 'showHeader', label: 'Show header', type: 'bool', default: true, group: 'Elements' },
    { key: 'decimals', label: 'Decimals', type: 'number', default: 2, min: 0, max: 3, group: 'Format' },
    { key: 'warnLaps', label: 'Warn when laps of fuel below', type: 'number', default: 2, min: 0, max: 10, step: 0.5, group: 'Warnings' },
  ],
  create(root, ctx) {
    const s = ctx.settings;
    const u = () => ctx.units;
    const dec = s.decimals as number;
    const header = h('div', 'header');
    header.append(h('span', 'title', 'Fuel'), h('span', 'info'));
    show(header, s.showHeader as boolean);
    const gauge = h('div', 'gauge');
    const gaugeFill = h('div', 'fill');
    gauge.append(gaugeFill);
    show(gauge, s.showGauge as boolean);
    const grid = h('div', 'kv');
    const cells: Record<string, HTMLElement> = {};
    for (const [key, label] of FIELDS) {
      if (!s[`show_${key}`]) continue;
      const k = h('div', 'k', label);
      const v = h('div', 'v');
      grid.append(k, v);
      cells[key] = v;
    }
    const hist = h('div', 'history');
    show(hist, s.showHistory as boolean);
    root.append(header, gauge, grid, hist);
    const vol = (l: number | null) => (l == null ? '–' : `${f.fuel(l, u()).toFixed(dec)} ${f.fuelUnit(u())}`);
    const num = (v: number | null, d = 1) => (v == null ? '–' : v.toFixed(d));

    return {
      update(topic, data) {
        if (topic !== 'fuel') return;
        const d = data as Fuel;
        const set = (k: string, v: string) => cells[k] && text(cells[k], v);
        set('level', vol(d.level));
        set('perLapAvg', d.estimateFrom === 'history' ? `${vol(d.perLapAvg)}*` : vol(d.perLapAvg));
        const x = d.expected;
        set('expected', x ? `${vol(x.perLap)} · ${x.laps} laps${x.lapTime > 0 ? ` · ${f.lapTime(x.lapTime)}` : ''}` : 'Not driven here yet');
        if (cells.expected) cells.expected.title = x ? `${x.car} at ${x.track}: average of your recent laps in earlier sessions (highest ${vol(x.max)}). Last driven ${new Date(x.updated * 1000).toLocaleDateString()}.` : '';
        set('perLapLast', vol(d.perLapLast));
        set('perLapMax', vol(d.perLapMax));
        set('lapsInTank', num(d.lapsInTank));
        set('lapsRemaining', num(d.lapsRemaining));
        set('fuelToFinish', vol(d.fuelToFinish));
        set('fuelToAdd', d.stops === 0 ? 'None' : vol(d.fuelToAdd));
        set('stops', d.stops == null ? '–' : String(d.stops));
        set('window', d.windowOpensLap != null && d.windowClosesLap != null ? `L${d.windowOpensLap} – L${d.windowClosesLap}` : d.stops === 0 ? 'No stop' : '–');
        set('targetPerLap', vol(d.targetPerLap));
        set('savePerLap', d.savePerLap == null ? '–' : vol(d.savePerLap));
        set('lapsPerTank', num(d.lapsPerTank));
        set('usePerHour', `${d.usePerHour.toFixed(1)} kg/h`);
        if (cells.lapsInTank) toggle(cells.lapsInTank, 'bad', d.lapsInTank != null && d.lapsInTank < (s.warnLaps as number));
        if (cells.stops) toggle(cells.stops, 'good', d.stops === 0);
        const pct = d.capacity > 0 ? Math.min(1, d.level / d.capacity) : d.pct;
        gaugeFill.style.transform = `scaleX(${pct.toFixed(3)})`;
        toggle(gauge, 'low', d.lapsInTank != null && d.lapsInTank < (s.warnLaps as number));
        const info = header.lastElementChild as HTMLElement;
        text(info, d.lapsSampled ? `${d.lapsSampled} laps sampled` : d.estimateFrom === 'history' ? '* from earlier sessions' : 'waiting for a clean lap');
        if (s.showHistory) {
          const max = Math.max(...d.history, 0.001);
          const hl = [...d.history].reverse();
          while (hist.children.length < hl.length) hist.append(h('div', 'hbar'));
          [...hist.children].forEach((c, i) => {
            const el = c as HTMLElement;
            show(el, i < hl.length);
            if (i < hl.length) {
              el.style.height = `${(hl[i] / max) * 100}%`;
              el.title = vol(hl[i]);
            }
          });
        }
      },
    };
  },
};
