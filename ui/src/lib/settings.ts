// Declarative settings schema. Overlays declare their options once; the
// control panel renders forms from it and overlays read merged values.

export interface ColumnDef {
  id: string;
  label: string;
}

export interface ColumnSetting {
  id: string;
  on: boolean;
}

interface Base {
  key: string;
  label: string;
  help?: string;
  group?: string;
}

export type SettingDef =
  | (Base & { type: 'bool'; default: boolean })
  | (Base & { type: 'number'; default: number; min?: number; max?: number; step?: number; unit?: string })
  | (Base & { type: 'color'; default: string })
  | (Base & { type: 'select'; default: string; options: [string, string][] })
  | (Base & { type: 'text'; default: string; multiline?: boolean; placeholder?: string })
  | (Base & { type: 'columns'; default: ColumnSetting[]; columns: ColumnDef[] });

export type Values = Record<string, unknown>;

/** Column list helper: every known column on/off in the stored order. */
export function columns(def: ColumnDef[], on: string[]): { columns: ColumnDef[]; default: ColumnSetting[] } {
  return { columns: def, default: def.map((c) => ({ id: c.id, on: on.includes(c.id) })) };
}

function mergeColumns(stored: unknown, def: Extract<SettingDef, { type: 'columns' }>): ColumnSetting[] {
  if (!Array.isArray(stored)) return def.default.map((c) => ({ ...c }));
  const known = new Set(def.columns.map((c) => c.id));
  const out = (stored as ColumnSetting[]).filter((c) => c && known.has(c.id)).map((c) => ({ id: c.id, on: !!c.on }));
  for (const d of def.default) if (!out.some((c) => c.id === d.id)) out.push({ ...d });
  return out;
}

/** Stored values over schema defaults; unknown keys are dropped, wrong types fall back. */
export function merge(schema: SettingDef[], stored: Values | undefined): Values {
  const v: Values = {};
  for (const s of schema) {
    const raw = stored?.[s.key];
    switch (s.type) {
      case 'bool':
        v[s.key] = typeof raw === 'boolean' ? raw : s.default;
        break;
      case 'number': {
        let n = typeof raw === 'number' && Number.isFinite(raw) ? raw : s.default;
        if (s.min != null) n = Math.max(s.min, n);
        if (s.max != null) n = Math.min(s.max, n);
        v[s.key] = n;
        break;
      }
      case 'columns':
        v[s.key] = mergeColumns(raw, s);
        break;
      case 'select':
        v[s.key] = typeof raw === 'string' && s.options.some((o) => o[0] === raw) ? raw : s.default;
        break;
      default:
        v[s.key] = typeof raw === 'string' ? raw : s.default;
    }
  }
  return v;
}

export function enabledColumns(v: unknown): string[] {
  return (v as ColumnSetting[]).filter((c) => c.on).map((c) => c.id);
}

/** Options every overlay gets. */
export const COMMON: SettingDef[] = [
  { key: 'opacity', label: 'Overlay opacity', type: 'number', default: 100, min: 10, max: 100, step: 1, unit: '%', help: 'Fades the whole overlay, text included', group: 'Transparency' },
  { key: 'bgOpacity', label: 'Background opacity', type: 'number', default: -1, min: -1, max: 100, step: 1, unit: '%', help: '-1 follows the theme; 0 = fully see-through background', group: 'Transparency' },
  { key: 'scale', label: 'Scale', type: 'number', default: 100, min: 40, max: 300, step: 5, unit: '%', group: 'General' },
  {
    key: 'autoHeight',
    label: 'Grow/shrink the window to fit the content',
    type: 'bool',
    default: false,
    help: 'The height follows the number of rows; you only set the width',
    group: 'General',
  },
  {
    key: 'autoWidth',
    label: 'Fit the window width to the columns',
    type: 'bool',
    default: false,
    help: 'Adding or removing columns resizes the overlay (relative, standings and other tables)',
    group: 'General',
  },
  { key: 'hideWhenDisconnected', label: 'Hide when iRacing is not running', type: 'bool', default: true, group: 'Visibility' },
  { key: 'hideWhenNotDriving', label: 'Hide unless I am in the car', type: 'bool', default: false, group: 'Visibility' },
  { key: 'hideInPits', label: 'Hide while on pit road', type: 'bool', default: false, group: 'Visibility' },
  { key: 'raceOnly', label: 'Only show in race sessions', type: 'bool', default: false, group: 'Visibility' },
  { key: 'customCss', label: 'Custom CSS', type: 'text', multiline: true, default: '', placeholder: '.row.player { outline: 1px solid gold; }', group: 'Advanced' },
];

/** Common options with an overlay's own defaults applied (e.g. standings auto-fits by default). */
export function commonFor(defaults?: Values): SettingDef[] {
  if (!defaults) return COMMON;
  return COMMON.map((s) => (s.key in defaults ? ({ ...s, default: defaults[s.key] } as SettingDef) : s));
}

export const NAME_STYLES: [string, string][] = [
  ['full', 'Full name'],
  ['initialLast', 'J. Smith'],
  ['lastUpper', 'J. SMITH'],
  ['last', 'Smith'],
  ['first', 'John'],
];
