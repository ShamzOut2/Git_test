import { fitCanvas, h, show, text } from '../lib/dom';
import type { OverlayDef } from '../lib/overlay';
import type { CarDot, Cars, TrackMap } from '../lib/types';

type Pt = [number, number];

export const trackmap: OverlayDef = {
  kind: 'trackmap',
  name: 'Track map',
  description: 'Every car on track. Three styles: the real circuit outline (recorded automatically from your first clean lap), a flat line and a round map.',
  topics: ['cars', 'trackmap'],
  size: [340, 340],
  settings: [
    { key: 'style', label: 'Style', type: 'select', default: 'full', options: [['full', 'Full circuit'], ['flat', 'Flat (straight line)'], ['round', 'Round']], group: 'Style' },
    { key: 'fallback', label: 'Until the outline is recorded', type: 'select', default: 'round', options: [['round', 'Use round map'], ['flat', 'Use flat map']], group: 'Style' },
    { key: 'flatLanes', label: 'Flat map: one lane per class', type: 'bool', default: true, group: 'Style' },
    { key: 'rotate', label: 'Rotate', type: 'number', default: 0, min: 0, max: 359, step: 1, unit: '°', group: 'Orientation' },
    { key: 'flipX', label: 'Mirror horizontally', type: 'bool', default: false, group: 'Orientation' },
    { key: 'flipY', label: 'Mirror vertically', type: 'bool', default: false, group: 'Orientation' },
    { key: 'trackWidth', label: 'Track width', type: 'number', default: 6, min: 1, max: 30, unit: 'px', group: 'Track' },
    { key: 'trackColor', label: 'Track colour', type: 'color', default: '#e6e9ef', group: 'Track' },
    { key: 'outlineWidth', label: 'Outline width', type: 'number', default: 3, min: 0, max: 20, unit: 'px', group: 'Track' },
    { key: 'outlineColor', label: 'Outline colour', type: 'color', default: '#000000', group: 'Track' },
    { key: 'showSectors', label: 'Sector marks', type: 'bool', default: true, group: 'Track' },
    { key: 'sectorColor', label: 'Sector colour', type: 'color', default: '#ffb020', group: 'Track' },
    { key: 'showStartFinish', label: 'Start / finish line', type: 'bool', default: true, group: 'Track' },
    {
      key: 'cornerLabels',
      label: 'Corner labels',
      type: 'select',
      default: 'number',
      options: [['none', 'None'], ['number', 'Turn numbers (T3)'], ['name', 'Corner names (Eau Rouge)'], ['both', 'Both (T3 Eau Rouge)']],
      help: 'Corner data is downloaded per track; edit or add your own under Settings → General',
      group: 'Corners',
    },
    { key: 'cornerPrefix', label: 'Turn number prefix', type: 'text', default: 'T', group: 'Corners' },
    { key: 'cornerFontSize', label: 'Label size', type: 'number', default: 10, min: 6, max: 24, unit: 'px', group: 'Corners' },
    { key: 'cornerColor', label: 'Label colour', type: 'color', default: '#ffffff', group: 'Corners' },
    { key: 'cornerBackground', label: 'Label background', type: 'bool', default: true, group: 'Corners' },
    { key: 'cornerOffset', label: 'Distance from track', type: 'number', default: 14, min: 0, max: 60, unit: 'px', group: 'Corners' },
    { key: 'dotSize', label: 'Car size', type: 'number', default: 7, min: 2, max: 30, unit: 'px', group: 'Cars' },
    { key: 'playerScale', label: 'Your car size', type: 'number', default: 1.45, min: 1, max: 3, step: 0.05, unit: '×', group: 'Cars' },
    { key: 'label', label: 'Label on cars', type: 'select', default: 'classPosition', options: [['none', 'None'], ['number', 'Car number'], ['position', 'Overall position'], ['classPosition', 'Class position']], group: 'Cars' },
    { key: 'colorBy', label: 'Car colour', type: 'select', default: 'class', options: [['class', 'Class colour'], ['single', 'Single colour']], group: 'Cars' },
    { key: 'carColor', label: 'Single car colour', type: 'color', default: '#ffffff', group: 'Cars' },
    { key: 'leaderRing', label: 'Ring around class leaders', type: 'bool', default: true, group: 'Cars' },
    { key: 'dimPitCars', label: 'Dim cars in the pits', type: 'bool', default: true, group: 'Cars' },
    { key: 'showPanel', label: 'Background panel', type: 'bool', default: false, group: 'Style' },
  ],
  create(root, ctx) {
    const s = ctx.settings;
    const t = ctx.theme;
    root.classList.toggle('transparent', !s.showPanel);
    const canvas = h('canvas', 'fill');
    const note = h('div', 'map-note');
    root.append(canvas, note);
    const g = canvas.getContext('2d')!;
    let map: TrackMap | null = null;
    let cars: Cars | null = null;
    let base: HTMLCanvasElement | null = null;
    let pts: Pt[] = [];
    let mode: string = s.style as string;
    let family = '';
    const dpr = () => window.devicePixelRatio || 1;

    const transform = (p: Pt, w: number, hh: number): Pt => {
      let x = p[0] - 0.5;
      let y = p[1] - 0.5;
      if (s.flipX) x = -x;
      if (s.flipY) y = -y;
      const a = ((s.rotate as number) * Math.PI) / 180;
      const rx = x * Math.cos(a) - y * Math.sin(a);
      const ry = x * Math.sin(a) + y * Math.cos(a);
      const labels = s.cornerLabels !== 'none' && map?.corners?.length ? (s.cornerOffset as number) + (s.cornerFontSize as number) : 0;
      const pad = ((s.dotSize as number) * (s.playerScale as number) + 4 + labels) * dpr();
      const size = Math.min(w, hh) - pad * 2;
      return [w / 2 + rx * size, hh / 2 + ry * size];
    };

    /** Screen position of a lap fraction in the current mode. */
    const locate = (pct: number, lane: number, lanes: number): Pt => {
      const w = canvas.width;
      const hh = canvas.height;
      const pad = ((s.dotSize as number) * (s.playerScale as number) + 4) * dpr();
      if (mode === 'flat') {
        const y = lanes > 1 ? pad + ((hh - pad * 2) * (lane + 0.5)) / lanes : hh / 2;
        return [pad + (w - pad * 2) * pct, y];
      }
      if (mode === 'round') {
        const r = Math.min(w, hh) / 2 - pad;
        const a = -Math.PI / 2 + pct * Math.PI * 2;
        return [w / 2 + Math.cos(a) * r, hh / 2 + Math.sin(a) * r];
      }
      const n = pts.length;
      const fi = (((pct % 1) + 1) % 1) * n;
      const i = Math.floor(fi) % n;
      const j = (i + 1) % n;
      const k = fi - Math.floor(fi);
      return [pts[i][0] + (pts[j][0] - pts[i][0]) * k, pts[i][1] + (pts[j][1] - pts[i][1]) * k];
    };

    const lanes = () => {
      const ids: string[] = [];
      for (const c of cars?.cars ?? []) if (!ids.includes(c.cls)) ids.push(c.cls);
      return mode === 'flat' && s.flatLanes ? ids : [];
    };

    /** Draws the static track into an offscreen canvas (only on resize / map change). */
    const buildBase = () => {
      const w = canvas.width;
      const hh = canvas.height;
      mode = s.style as string;
      if (mode === 'full' && !map?.points?.length) mode = s.fallback as string;
      pts = mode === 'full' && map?.points ? map.points.map((p) => transform(p, w, hh)) : [];
      base = document.createElement('canvas');
      base.width = w;
      base.height = hh;
      const b = base.getContext('2d')!;
      b.lineCap = 'round';
      b.lineJoin = 'round';
      const path = new Path2D();
      const laneIds = lanes();
      const nl = Math.max(1, laneIds.length);
      if (mode === 'flat') {
        for (let l = 0; l < nl; l++) {
          const a = locate(0, l, nl);
          const z = locate(1, l, nl);
          path.moveTo(a[0], a[1]);
          path.lineTo(z[0], z[1]);
        }
      } else if (mode === 'round') {
        const c = locate(0, 0, 1);
        const r = hh / 2 - c[1];
        path.arc(w / 2, hh / 2, Math.abs(r), 0, Math.PI * 2);
      } else {
        pts.forEach((p, i) => (i ? path.lineTo(p[0], p[1]) : path.moveTo(p[0], p[1])));
        path.closePath();
      }
      const tw = (s.trackWidth as number) * dpr();
      if ((s.outlineWidth as number) > 0) {
        b.strokeStyle = s.outlineColor as string;
        b.lineWidth = tw + (s.outlineWidth as number) * 2 * dpr();
        b.stroke(path);
      }
      b.strokeStyle = s.trackColor as string;
      b.lineWidth = tw;
      b.stroke(path);
      const tick = (pct: number, color: string, len: number) => {
        for (let l = 0; l < (mode === 'flat' ? nl : 1); l++) {
          const p = locate(pct, l, nl);
          const q = locate(pct + 0.002, l, nl);
          const ang = Math.atan2(q[1] - p[1], q[0] - p[0]) + Math.PI / 2;
          b.strokeStyle = color;
          b.lineWidth = 2.5 * dpr();
          b.beginPath();
          b.moveTo(p[0] - Math.cos(ang) * len, p[1] - Math.sin(ang) * len);
          b.lineTo(p[0] + Math.cos(ang) * len, p[1] + Math.sin(ang) * len);
          b.stroke();
        }
      };
      if (s.cornerLabels !== 'none' && map?.corners?.length) drawCorners(b, nl);
      const sectors = map?.sectors ?? cars?.sectors ?? [];
      const across = tw / 2 + ((s.outlineWidth as number) + 3) * dpr();
      if (s.showSectors) for (const sp of sectors) if (sp > 0) tick(sp, s.sectorColor as string, across);
      if (s.showStartFinish) tick(0, '#ffffff', across * 1.3);
    };

    const cornerText = (c: { name: string; number: number }) => {
      const num = `${s.cornerPrefix}${c.number}`;
      if (s.cornerLabels === 'number') return num;
      if (s.cornerLabels === 'name') return c.name || num;
      return c.name ? `${num} ${c.name}` : num;
    };

    /** Corner labels, pushed outwards from the track so they don't cover cars. */
    const drawCorners = (b: CanvasRenderingContext2D, nl: number) => {
      const w = canvas.width;
      const hh = canvas.height;
      const k = dpr();
      family ||= getComputedStyle(root).getPropertyValue('--font') || 'sans-serif';
      b.font = `600 ${(s.cornerFontSize as number) * k}px ${family}`;
      b.textAlign = 'center';
      b.textBaseline = 'middle';
      map!.corners.forEach((c, i) => {
        const mid = c.end >= c.start ? (c.start + c.end) / 2 : ((c.start + c.end + 1) / 2) % 1;
        const p = locate(mid, 0, nl);
        const q = locate(mid + 0.003, 0, nl);
        let nx = -(q[1] - p[1]);
        let ny = q[0] - p[0];
        const len = Math.hypot(nx, ny) || 1;
        nx /= len;
        ny /= len;
        if (mode === 'flat') {
          nx = 0;
          ny = i % 2 ? 1 : -1;
        } else if ((p[0] + nx - w / 2) ** 2 + (p[1] + ny - hh / 2) ** 2 < (p[0] - w / 2) ** 2 + (p[1] - hh / 2) ** 2) {
          nx = -nx;
          ny = -ny;
        }
        const off = ((s.cornerOffset as number) + (s.trackWidth as number) / 2) * k;
        const label = cornerText(c);
        const half = b.measureText(label).width / 2 + 4 * k;
        const fh = (s.cornerFontSize as number) * k;
        // Keep labels inside the overlay.
        const x = Math.min(w - half, Math.max(half, p[0] + nx * off));
        const y = Math.min(hh - fh, Math.max(fh, p[1] + ny * off));
        if (s.cornerBackground) {
          const tw = b.measureText(label).width + 6 * k;
          const th = (s.cornerFontSize as number) * 1.45 * k;
          b.fillStyle = 'rgba(0,0,0,0.6)';
          b.beginPath();
          b.roundRect(x - tw / 2, y - th / 2, tw, th, 3 * k);
          b.fill();
        }
        b.fillStyle = s.cornerColor as string;
        b.fillText(label, x, y + 0.5);
      });
    };

    const draw = () => {
      if (fitCanvas(canvas) || !base) buildBase();
      const w = canvas.width;
      const hh = canvas.height;
      g.clearRect(0, 0, w, hh);
      g.drawImage(base!, 0, 0);
      if (!cars) return;
      const laneIds = lanes();
      const nl = Math.max(1, laneIds.length);
      const leaders = new Set<number>();
      const seen = new Set<string>();
      for (const c of [...cars.cars].sort((a, b) => a.pos - b.pos)) {
        if (!seen.has(c.cls)) {
          seen.add(c.cls);
          leaders.add(c.i);
        }
      }
      const focus = cars.focus;
      // Focus car last so it is drawn on top.
      const list: CarDot[] = [...cars.cars].sort((a, b) => (a.i === focus ? 1 : b.i === focus ? -1 : b.pos - a.pos));
      const font = Math.max(8, (s.dotSize as number) * 1.15) * dpr();
      family ||= getComputedStyle(root).getPropertyValue('--font') || 'sans-serif';
      g.textAlign = 'center';
      g.textBaseline = 'middle';
      for (const c of list) {
        const isFocus = c.i === focus;
        const [x, y] = locate(c.p, Math.max(0, laneIds.indexOf(c.cls)), nl);
        const r = (s.dotSize as number) * dpr() * (isFocus ? (s.playerScale as number) : 1);
        g.globalAlpha = s.dimPitCars && c.pit ? 0.4 : 1;
        g.beginPath();
        g.arc(x, y, r, 0, Math.PI * 2);
        g.fillStyle = isFocus ? (t.playerColor as string) : s.colorBy === 'single' ? (s.carColor as string) : c.c;
        g.fill();
        g.lineWidth = 1.5 * dpr();
        g.strokeStyle = c.off ? (t.warnColor as string) : 'rgba(0,0,0,0.85)';
        g.stroke();
        if (s.leaderRing && leaders.has(c.i)) {
          g.beginPath();
          g.arc(x, y, r + 2.5 * dpr(), 0, Math.PI * 2);
          g.strokeStyle = '#ffffff';
          g.lineWidth = 1.5 * dpr();
          g.stroke();
        }
        const label = s.label === 'number' ? c.n : s.label === 'position' ? String(c.pos) : s.label === 'classPosition' ? String(c.cpos) : '';
        if (label) {
          g.font = `700 ${font * (isFocus ? 1.1 : 1)}px ${family}`;
          g.fillStyle = '#0b0e13';
          g.fillText(label, x, y + 0.5);
        }
      }
      g.globalAlpha = 1;
    };

    new ResizeObserver(() => {
      base = null;
      draw();
    }).observe(canvas);

    return {
      update(topic, data) {
        if (topic === 'trackmap') {
          map = data as TrackMap;
          base = null;
          const recording = (s.style as string) === 'full' && !map.points?.length;
          show(note, recording);
          text(note, map.recording ? `Recording track outline… ${Math.round(map.progress * 100)}%` : 'Drive a clean lap to record the track outline');
        } else if (topic === 'cars') {
          const prev = lanes().join();
          cars = data as Cars;
          if (mode === 'flat' && lanes().join() !== prev) base = null;
        }
        draw();
      },
    };
  },
};
