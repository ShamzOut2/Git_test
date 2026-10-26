// Global look & feel, applied as CSS variables. Every value is user editable.

import { merge, type SettingDef, type Values } from './settings';

export const THEME: SettingDef[] = [
  { key: 'fontFamily', label: 'Font family', type: 'text', default: "'Segoe UI', 'Inter', system-ui, sans-serif", group: 'Text' },
  { key: 'fontSize', label: 'Base font size', type: 'number', default: 13, min: 8, max: 32, step: 0.5, unit: 'px', group: 'Text' },
  { key: 'fontWeight', label: 'Font weight', type: 'select', default: '500', options: [['400', 'Regular'], ['500', 'Medium'], ['600', 'Semi bold'], ['700', 'Bold']], group: 'Text' },
  { key: 'textColor', label: 'Text', type: 'color', default: '#f2f4f8', group: 'Colours' },
  { key: 'dimColor', label: 'Secondary text', type: 'color', default: '#98a2b3', group: 'Colours' },
  { key: 'background', label: 'Background', type: 'color', default: '#0b0e13', group: 'Colours' },
  { key: 'backgroundOpacity', label: 'Background opacity', type: 'number', default: 80, min: 0, max: 100, unit: '%', group: 'Colours' },
  { key: 'headerBackground', label: 'Header background', type: 'color', default: '#161b24', group: 'Colours' },
  { key: 'accent', label: 'Accent', type: 'color', default: '#ff3d57', group: 'Colours' },
  { key: 'playerColor', label: 'Your car highlight', type: 'color', default: '#ffd84d', group: 'Colours' },
  { key: 'lapAheadColor', label: 'Car a lap ahead of you', type: 'color', default: '#ff7a7a', group: 'Colours' },
  { key: 'lapBehindColor', label: 'Car a lap behind you', type: 'color', default: '#6cb4ff', group: 'Colours' },
  { key: 'pitColor', label: 'Car in pits', type: 'color', default: '#7d8593', group: 'Colours' },
  { key: 'bestColor', label: 'Fastest lap', type: 'color', default: '#c27bff', group: 'Colours' },
  { key: 'goodColor', label: 'Gaining / good', type: 'color', default: '#3ddc84', group: 'Colours' },
  { key: 'badColor', label: 'Losing / warning', type: 'color', default: '#ff5c5c', group: 'Colours' },
  { key: 'warnColor', label: 'Caution', type: 'color', default: '#ffb020', group: 'Colours' },
  { key: 'radius', label: 'Corner radius', type: 'number', default: 6, min: 0, max: 24, unit: 'px', group: 'Shape' },
  { key: 'rowAlt', label: 'Alternate row shading', type: 'number', default: 4, min: 0, max: 30, unit: '%', group: 'Shape' },
  { key: 'rowPadding', label: 'Row padding', type: 'number', default: 0.22, min: 0, max: 1, step: 0.02, unit: 'em', group: 'Shape' },
  { key: 'border', label: 'Outline', type: 'bool', default: true, group: 'Shape' },
  { key: 'shadow', label: 'Text shadow (better on bright scenes)', type: 'bool', default: true, group: 'Shape' },
];

function rgba(hex: string, alpha: number): string {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim());
  if (!m) return hex;
  const n = parseInt(m[1], 16);
  return `rgba(${n >> 16}, ${(n >> 8) & 255}, ${n & 255}, ${alpha})`;
}

export function applyTheme(stored: Values | undefined, el: HTMLElement, overrides: { scale?: number; bgOpacity?: number } = {}) {
  const t = merge(THEME, stored);
  const s = el.style;
  const bgOpacity = overrides.bgOpacity != null && overrides.bgOpacity >= 0 ? overrides.bgOpacity : (t.backgroundOpacity as number);
  s.setProperty('--font', t.fontFamily as string);
  s.setProperty('--font-size', `${((t.fontSize as number) * (overrides.scale ?? 100)) / 100}px`);
  s.setProperty('--weight', t.fontWeight as string);
  s.setProperty('--text', t.textColor as string);
  s.setProperty('--dim', t.dimColor as string);
  s.setProperty('--bg', rgba(t.background as string, bgOpacity / 100));
  s.setProperty('--bg-solid', t.background as string);
  s.setProperty('--header', rgba(t.headerBackground as string, Math.min(1, bgOpacity / 100 + 0.1)));
  s.setProperty('--accent', t.accent as string);
  s.setProperty('--player', t.playerColor as string);
  s.setProperty('--player-bg', rgba(t.playerColor as string, 0.16));
  s.setProperty('--lap-ahead', t.lapAheadColor as string);
  s.setProperty('--lap-behind', t.lapBehindColor as string);
  s.setProperty('--pit', t.pitColor as string);
  s.setProperty('--best', t.bestColor as string);
  s.setProperty('--good', t.goodColor as string);
  s.setProperty('--bad', t.badColor as string);
  s.setProperty('--warn', t.warnColor as string);
  s.setProperty('--radius', `${t.radius}px`);
  s.setProperty('--row-alt', `rgba(255,255,255,${(t.rowAlt as number) / 100})`);
  s.setProperty('--row-pad', `${t.rowPadding}em`);
  s.setProperty('--border', t.border ? '1px solid rgba(255,255,255,0.09)' : 'none');
  s.setProperty('--shadow', t.shadow ? '0 1px 2px rgba(0,0,0,0.8)' : 'none');
  return t;
}
