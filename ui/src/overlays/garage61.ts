import { h, show, text } from '../lib/dom';
import * as f from '../lib/format';
import type { OverlayDef } from '../lib/overlay';
import type { Garage61, Player } from '../lib/types';

export const garage61: OverlayDef = {
  kind: 'garage61',
  name: 'Garage 61',
  description: 'Your Garage 61 personal best and your teams’ best laps for the current car and track, compared with your best lap this session.',
  topics: ['garage61', 'player'],
  size: [300, 170],
  settings: [
    { key: 'rows', label: 'Team leaderboard rows', type: 'number', default: 5, min: 0, max: 10, group: 'Layout' },
    { key: 'showDates', label: 'Show lap dates', type: 'bool', default: false, group: 'Layout' },
    { key: 'showSessionDelta', label: 'Compare with my session best', type: 'bool', default: true, group: 'Layout' },
  ],
  create(root, ctx) {
    const s = ctx.settings;
    root.classList.add('g61');
    const header = h('div', 'header');
    const title = h('span', 'title', 'Garage 61');
    const info = h('span', 'info');
    header.append(title, info);
    const status = h('div', 'g61-status dim');
    const pb = h('div', 'kv');
    const pbK = h('div', 'k', 'Personal best');
    const pbV = h('div', 'v');
    const sbK = h('div', 'k', 'Session best');
    const sbV = h('div', 'v');
    pb.append(pbK, pbV, sbK, sbV);
    show(sbK, s.showSessionDelta as boolean);
    show(sbV, s.showSessionDelta as boolean);
    const list = h('div', 'g61-list');
    root.append(header, status, pb, list);
    let g: Garage61 | null = null;
    let best: number | null = null;

    const render = () => {
      if (!g) return;
      const ok = g.status === 'ok';
      show(status, !ok);
      show(pb, ok);
      show(list, ok);
      text(info, ok ? `${g.car}` : '');
      text(status, g.status === 'loading' ? 'Loading…' : g.message || g.status);
      if (!ok) return;
      text(pbV, g.personalBest ? f.lapTime(g.personalBest.lapTime) : 'No lap yet');
      const delta = best != null && g.personalBest ? best - g.personalBest.lapTime : null;
      text(sbV, best ? `${f.lapTime(best)}${delta != null ? ` (${f.signed(delta, 3)})` : ''}` : '–');
      sbV.classList.toggle('good', delta != null && delta < 0);
      const rows = g.leaderboard.slice(0, s.rows as number);
      while (list.children.length < rows.length) {
        const r = h('div', 'g61-row');
        r.append(h('span', 'p'), h('span', 'n'), h('span', 't'));
        list.append(r);
      }
      [...list.children].forEach((el, i) => {
        const r = el as HTMLElement;
        show(r, i < rows.length);
        if (i >= rows.length) return;
        const l = rows[i];
        const [p, n, t] = [...r.children] as HTMLElement[];
        text(p, String(i + 1));
        text(n, l.driver + (s.showDates && l.date ? `  ${l.date}` : ''));
        text(t, f.lapTime(l.lapTime));
        r.classList.toggle('me', l.isMe);
      });
    };

    return {
      update(topic, data) {
        if (topic === 'garage61') g = data as Garage61;
        if (topic === 'player') {
          const b = (data as Player).bestLap;
          if (b === best) return;
          best = b;
        }
        render();
      },
    };
  },
};
