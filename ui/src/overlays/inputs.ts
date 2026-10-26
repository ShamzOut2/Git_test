import { fitCanvas, h, show, style, text, toggle } from '../lib/dom';
import * as f from '../lib/format';
import type { OverlayDef } from '../lib/overlay';
import type { Player } from '../lib/types';

const WHEEL_SVG = `<svg viewBox="-50 -50 100 100"><circle r="42" fill="none" stroke="currentColor" stroke-width="9"/><path d="M-40 4 H-12 L-6 14 H6 L12 4 H40" fill="none" stroke="currentColor" stroke-width="8" stroke-linejoin="round"/><path d="M0 14 V40" stroke="currentColor" stroke-width="8"/><rect x="-3" y="-50" width="6" height="12" fill="var(--accent)"/></svg>`;

export const inputs: OverlayDef = {
  kind: 'inputs',
  name: 'Driver inputs',
  description: 'Throttle, brake and clutch bars with a scrolling input trace, steering wheel, gear and speed. Updates every telemetry tick.',
  topics: ['player', 'session'],
  size: [420, 120],
  settings: [
    { key: 'showTrace', label: 'Input trace graph', type: 'bool', default: true, group: 'Elements' },
    { key: 'showBars', label: 'Pedal bars', type: 'bool', default: true, group: 'Elements' },
    { key: 'showClutch', label: 'Clutch', type: 'bool', default: true, group: 'Elements' },
    { key: 'showWheel', label: 'Steering wheel', type: 'bool', default: true, group: 'Elements' },
    { key: 'showGear', label: 'Gear', type: 'bool', default: true, group: 'Elements' },
    { key: 'showSpeed', label: 'Speed', type: 'bool', default: true, group: 'Elements' },
    {
      key: 'valuePosition',
      label: 'Percent values',
      type: 'select',
      default: 'top',
      options: [['top', 'Above each bar'], ['inside', 'Inside the bar'], ['off', 'Off']],
      group: 'Elements',
    },
    { key: 'valueColored', label: 'Colour values like their bar', type: 'bool', default: true, group: 'Elements' },
    { key: 'traceSeconds', label: 'Trace length', type: 'number', default: 6, min: 1, max: 30, unit: 's', group: 'Trace' },
    { key: 'traceWidth', label: 'Trace line width', type: 'number', default: 2, min: 0.5, max: 6, step: 0.5, unit: 'px', group: 'Trace' },
    { key: 'traceSteering', label: 'Show steering in trace', type: 'bool', default: false, group: 'Trace' },
    { key: 'traceFill', label: 'Fill under throttle/brake', type: 'bool', default: true, group: 'Trace' },
    { key: 'absHighlight', label: 'Colour brake when ABS is active', type: 'bool', default: true, group: 'Trace' },
    { key: 'throttleColor', label: 'Throttle', type: 'color', default: '#3ddc84', group: 'Colours' },
    { key: 'brakeColor', label: 'Brake', type: 'color', default: '#ff4d4d', group: 'Colours' },
    { key: 'clutchColor', label: 'Clutch', type: 'color', default: '#4da3ff', group: 'Colours' },
    { key: 'absColor', label: 'ABS', type: 'color', default: '#ffd84d', group: 'Colours' },
    { key: 'steeringColor', label: 'Steering', type: 'color', default: '#e6e9ef', group: 'Colours' },
    { key: 'wheelColor', label: 'Wheel', type: 'color', default: '#e6e9ef', group: 'Colours' },
  ],
  create(root, ctx) {
    const s = ctx.settings;
    root.classList.add('inputs');
    const canvas = h('canvas', 'trace');
    const bars = h('div', 'bars');
    const valuePos = s.valuePosition as string;
    const mk = (cls: string, color: string) => {
      const wrap = h('div', `barcol ${cls}`);
      const bar = h('div', 'bar');
      const fill = h('div', 'fill');
      const val = h('div', `val ${valuePos}`);
      fill.style.background = color;
      if (s.valueColored && valuePos === 'top') val.style.color = color;
      bar.append(fill);
      if (valuePos === 'top') wrap.append(val, bar);
      else if (valuePos === 'inside') {
        bar.append(val);
        wrap.append(bar);
      } else wrap.append(bar);
      bars.append(wrap);
      return { wrap, fill, val };
    };
    const clutch = mk('clutch', s.clutchColor as string);
    const brake = mk('brake', s.brakeColor as string);
    const throttle = mk('throttle', s.throttleColor as string);
    show(clutch.wrap, s.showClutch as boolean);
    const side = h('div', 'side');
    const wheel = h('div', 'wheel');
    wheel.innerHTML = WHEEL_SVG;
    wheel.style.color = s.wheelColor as string;
    const gear = h('div', 'gear');
    const spd = h('div', 'speed');
    side.append(wheel, gear, spd);
    show(canvas, s.showTrace as boolean);
    show(bars, s.showBars as boolean);
    show(wheel, s.showWheel as boolean);
    show(gear, s.showGear as boolean);
    show(spd, s.showSpeed as boolean);
    show(side, (s.showWheel || s.showGear || s.showSpeed) as boolean);
    root.append(canvas, bars, side);

    const n = Math.round((s.traceSeconds as number) * 60);
    const thr = new Float32Array(n);
    const brk = new Float32Array(n);
    const clt = new Float32Array(n);
    const ste = new Float32Array(n);
    const abs = new Uint8Array(n);
    let head = 0;
    const g = canvas.getContext('2d')!;

    const series = (arr: Float32Array, color: string, fill: boolean, absArr?: Uint8Array) => {
      const w = canvas.width;
      const hh = canvas.height;
      const pad = 2 * (window.devicePixelRatio || 1);
      const y = (v: number) => hh - pad - v * (hh - pad * 2);
      const x = (i: number) => (i / (n - 1)) * w;
      g.beginPath();
      for (let i = 0; i < n; i++) {
        const v = arr[(head + i) % n];
        if (i === 0) g.moveTo(x(i), y(v));
        else g.lineTo(x(i), y(v));
      }
      g.strokeStyle = color;
      g.lineWidth = (s.traceWidth as number) * (window.devicePixelRatio || 1);
      g.stroke();
      if (fill) {
        g.lineTo(w, hh);
        g.lineTo(0, hh);
        g.closePath();
        g.globalAlpha = 0.15;
        g.fillStyle = color;
        g.fill();
        g.globalAlpha = 1;
      }
      if (absArr && s.absHighlight) {
        g.strokeStyle = s.absColor as string;
        for (let i = 1; i < n; i++) {
          const k = (head + i) % n;
          if (!absArr[k]) continue;
          g.beginPath();
          g.moveTo(x(i - 1), y(arr[(head + i - 1) % n]));
          g.lineTo(x(i), y(arr[k]));
          g.stroke();
        }
      }
    };

    return {
      update(topic, data) {
        if (topic !== 'player') return;
        const p = data as Player;
        thr[head] = p.throttle;
        brk[head] = p.brake;
        clt[head] = p.clutch;
        ste[head] = p.steeringMax ? 0.5 - p.steering / p.steeringMax / 2 : 0.5;
        abs[head] = p.abs ? 1 : 0;
        head = (head + 1) % n;

        const bar = (b: { fill: HTMLElement; val: HTMLElement }, v: number) => {
          style(b.fill, 'transform', `scaleY(${v.toFixed(3)})`);
          if (valuePos !== 'off') text(b.val, String(Math.round(v * 100)));
        };
        bar(throttle, p.throttle);
        bar(brake, p.brake);
        bar(clutch, p.clutch);
        toggle(brake.wrap, 'abs', p.abs);
        style(wheel, 'transform', `rotate(${(-p.steering * 180) / Math.PI}deg)`);
        text(gear, p.gear === -1 ? 'R' : p.gear === 0 ? 'N' : String(p.gear));
        text(spd, `${Math.round(f.speed(p.speed, ctx.units))}`);
        spd.dataset.unit = f.speedUnit(ctx.units);

        if (s.showTrace) {
          fitCanvas(canvas);
          g.clearRect(0, 0, canvas.width, canvas.height);
          if (s.traceSteering) series(ste, s.steeringColor as string, false);
          if (s.showClutch) series(clt, s.clutchColor as string, false);
          series(thr, s.throttleColor as string, s.traceFill as boolean);
          series(brk, s.brakeColor as string, s.traceFill as boolean, abs);
        }
      },
    };
  },
};
