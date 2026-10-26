import { h, text, toggle } from '../lib/dom';
import * as f from '../lib/format';
import type { OverlayDef } from '../lib/overlay';
import { columns, enabledColumns } from '../lib/settings';
import type { Session } from '../lib/types';

const FIELDS = [
  { id: 'track', label: 'Track' },
  { id: 'session', label: 'Session' },
  { id: 'time', label: 'Time / laps left' },
  { id: 'lap', label: 'Leader lap' },
  { id: 'clock', label: 'Time of day (in sim)' },
  { id: 'trackTemp', label: 'Track temperature' },
  { id: 'airTemp', label: 'Air temperature' },
  { id: 'wetness', label: 'Track wetness' },
  { id: 'rain', label: 'Rain' },
  { id: 'declared', label: 'Declared wet' },
  { id: 'skies', label: 'Sky' },
  { id: 'wind', label: 'Wind' },
  { id: 'humidity', label: 'Humidity' },
  { id: 'sof', label: 'Strength of field' },
  { id: 'cars', label: 'Cars' },
  { id: 'incidents', label: 'Incidents' },
  { id: 'flags', label: 'Flags' },
];

const LABELS: Record<string, string> = {
  track: 'TRACK',
  session: 'SESSION',
  time: 'REMAINING',
  lap: 'LAP',
  clock: 'TIME',
  trackTemp: 'TRACK',
  airTemp: 'AIR',
  wetness: 'SURFACE',
  rain: 'RAIN',
  declared: 'DECLARED WET',
  skies: 'SKY',
  wind: 'WIND',
  humidity: 'HUMIDITY',
  sof: 'SOF',
  cars: 'CARS',
  incidents: 'INC',
  flags: 'FLAG',
};

export const session: OverlayDef = {
  kind: 'session',
  name: 'Session info',
  description: 'Track & air temperature (with trend), wetness, rain, wind, sky, time remaining, sim clock, SOF, incidents and flags. Pick and order any fields.',
  topics: ['session'],
  size: [900, 52],
  settings: [
    { key: 'fields', label: 'Fields', type: 'columns', ...columns(FIELDS, ['session', 'time', 'clock', 'trackTemp', 'airTemp', 'wetness', 'rain', 'wind', 'sof', 'incidents']), group: 'Fields' },
    { key: 'layout', label: 'Layout', type: 'select', default: 'bar', options: [['bar', 'Horizontal bar'], ['stack', 'Vertical list']], group: 'Layout' },
    { key: 'showLabels', label: 'Show labels', type: 'bool', default: true, group: 'Layout' },
    { key: 'clock24', label: '24-hour clock', type: 'bool', default: true, group: 'Format' },
    { key: 'showTempTrend', label: 'Track temperature trend arrow', type: 'bool', default: true, group: 'Format' },
  ],
  create(root, ctx) {
    const s = ctx.settings;
    root.classList.add('session-info', `layout-${s.layout}`);
    const cells: Record<string, HTMLElement> = {};
    for (const id of enabledColumns(s.fields)) {
      const item = h('div', `item f-${id}`);
      if (s.showLabels) item.append(h('div', 'label', LABELS[id] ?? id));
      const v = h('div', 'value');
      item.append(v);
      root.append(item);
      cells[id] = v;
    }
    return {
      update(topic, data) {
        if (topic !== 'session') return;
        const d = data as Session;
        const u = ctx.units;
        const set = (k: string, v: string) => cells[k] && text(cells[k], v);
        set('track', d.trackName + (d.trackConfig ? ` – ${d.trackConfig}` : ''));
        set('session', d.sessionName || d.sessionType || '–');
        set('time', d.timeRemain != null ? f.duration(d.timeRemain) : d.lapsRemain != null ? `${d.lapsRemain} laps` : '–');
        set('lap', d.leaderLap ? `${d.leaderLap}${d.lapsTotal ? ' / ' + d.lapsTotal : ''}` : '–');
        set('clock', f.clock(d.timeOfDay, s.clock24 as boolean));
        const trend = s.showTempTrend && Math.abs(d.trackTempDelta) >= 0.3 ? (d.trackTempDelta > 0 ? ' ▲' : ' ▼') : '';
        set('trackTemp', f.temp(d.trackTemp, u) + trend);
        set('airTemp', f.temp(d.airTemp, u));
        set('wetness', d.wetnessLabel);
        set('declared', d.declaredWet ? 'Yes' : 'No');
        if (cells.wetness) {
          toggle(cells.wetness, 'wet', d.wetness >= 4);
          toggle(cells.wetness, 'damp', d.wetness === 2 || d.wetness === 3);
        }
        set('rain', d.precipitation > 0 ? `${Math.round(d.precipitation * 100)}%` : 'None');
        set('skies', d.skies);
        set('wind', `${Math.round(f.speed(d.windSpeed, u))} ${f.speedUnit(u)} ${f.compass(d.windDir)}`);
        set('humidity', `${Math.round(d.humidity * 100)}%`);
        set('sof', d.sof ? String(d.sof) : '–');
        set('cars', String(d.numCars));
        set('incidents', d.incidentLimit ? `${d.incidents} / ${d.incidentLimit}` : `${d.incidents}x`);
        set('flags', d.flags.length ? d.flags.map((x) => x.replace(/([A-Z])/g, ' $1').toLowerCase()).join(', ') : '–');
        if (cells.flags) {
          const fl = d.flags;
          cells.flags.dataset.flag = fl.includes('red') ? 'red' : fl.some((x) => x.startsWith('yellow') || x.startsWith('caution')) ? 'yellow' : fl.includes('blue') ? 'blue' : fl.includes('white') ? 'white' : fl.includes('checkered') ? 'checkered' : fl.includes('green') ? 'green' : '';
        }
      },
    };
  },
};
