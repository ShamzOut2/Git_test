import { h, show, text, toggle } from '../lib/dom';
import { STATS, statById } from '../lib/monitorStats';
import type { OverlayDef } from '../lib/overlay';
import { columns, enabledColumns } from '../lib/settings';
import type { Monitor } from '../lib/types';

export const monitor: OverlayDef = {
  kind: 'monitor',
  name: 'Performance monitor',
  description: 'A small widget showing what Apex Overlays itself uses: CPU, memory, data sent to the overlays, telemetry rate and more. Pick and order the readings.',
  topics: ['monitor'],
  size: [200, 120],
  settings: [
    { key: 'stats', label: 'Readings', type: 'columns', ...columns(STATS.map((s) => ({ id: s.id, label: s.label })), ['cpu', 'memory', 'sent', 'telemetry']), help: 'Hover a reading in the Monitoring section (General) for what it means', group: 'What to show' },
    {
      key: 'layout',
      label: 'Layout',
      type: 'select',
      default: 'list',
      options: [
        ['list', 'List (one reading per line)'],
        ['row', 'Row (side by side)'],
      ],
      group: 'Layout',
    },
    { key: 'showTitle', label: 'Title', type: 'bool', default: false, group: 'Layout' },
    { key: 'warnCpu', label: 'Highlight CPU above', type: 'number', default: 5, min: 1, max: 50, unit: '%', group: 'Alerts', help: 'Amber above this, red above twice this' },
  ],
  commonDefaults: { autoHeight: true, hideWhenDisconnected: false },
  create(root, ctx) {
    const s = ctx.settings;
    root.classList.add('monitor', `mon-${s.layout}`);
    const title = h('div', 'mon-title', 'Apex Overlays');
    show(title, !!s.showTitle);
    const grid = h('div', 'mon-grid');
    root.append(title, grid);
    const items = enabledColumns(s.stats)
      .map((id) => statById(id))
      .filter((x) => !!x)
      .map((stat) => {
        const el = h('div', 'mon-item');
        const v = h('span', 'mon-val', '–');
        el.append(h('span', 'mon-label', stat.label), v);
        el.title = stat.help;
        grid.append(el);
        return { stat, v };
      });
    if (!items.length) grid.append(h('div', 'mon-item dim', 'Pick readings in the settings'));
    return {
      update(topic, data) {
        if (topic !== 'monitor') return;
        const m = data as Monitor;
        for (const { stat, v } of items) {
          text(v, stat.value(m));
          const lvl = stat.level?.(m, s.warnCpu as number) ?? '';
          toggle(v, 'warn', lvl === 'warn');
          toggle(v, 'bad', lvl === 'bad');
        }
      },
    };
  },
};
