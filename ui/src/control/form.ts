// Renders forms from a settings schema (see lib/settings.ts).

import { h } from '../lib/dom';
import type { ColumnSetting, SettingDef, Values } from '../lib/settings';

export type OnChange = (key: string, value: unknown) => void;

const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b);

/** Shown columns in order (reorder / remove), the rest as chips to add. */
function columnsControl(s: Extract<SettingDef, { type: 'columns' }>, value: ColumnSetting[], onChange: OnChange): HTMLElement {
  const box = h('div', 'columns');
  const cols = value.map((c) => ({ ...c }));
  const label = (id: string) => s.columns.find((c) => c.id === id)?.label ?? id;
  const commit = () => {
    onChange(s.key, cols.map((x) => ({ ...x })));
    draw();
  };
  const draw = () => {
    const shown = cols.filter((c) => c.on);
    const list = h('div', 'col-list');
    shown.forEach((c, i) => {
      const item = h('div', 'col-item');
      const move = (d: number) => (e: Event) => {
        e.preventDefault();
        // Swap with the neighbouring shown column (hidden ones keep their place).
        const a = cols.indexOf(c);
        const b = cols.indexOf(shown[i + d]);
        [cols[a], cols[b]] = [cols[b], cols[a]];
        commit();
      };
      const up = h('button', 'mini', '▲');
      const down = h('button', 'mini', '▼');
      const remove = h('button', 'mini subtle', '✕');
      up.title = 'Move left';
      down.title = 'Move right';
      remove.title = 'Hide this column';
      up.disabled = i === 0;
      down.disabled = i === shown.length - 1;
      up.onclick = move(-1);
      down.onclick = move(1);
      remove.onclick = (e) => {
        e.preventDefault();
        c.on = false;
        commit();
      };
      const name = h('span', 'col-name', label(c.id));
      name.title = label(c.id);
      item.append(h('span', 'col-pos', String(i + 1)), name, up, down, remove);
      list.append(item);
    });
    if (!shown.length) list.append(h('p', 'dim small', 'No columns shown — add some below.'));
    const hidden = cols.filter((c) => !c.on);
    const chips = h('div', 'col-chips');
    for (const c of hidden) {
      const b = h('button', 'chip', `+ ${label(c.id)}`);
      b.onclick = (e) => {
        e.preventDefault();
        c.on = true;
        // Added columns go to the end of the shown ones.
        cols.splice(cols.indexOf(c), 1);
        const last = cols.map((x) => x.on).lastIndexOf(true);
        cols.splice(last + 1, 0, c);
        commit();
      };
      chips.append(b);
    }
    box.replaceChildren(list, ...(hidden.length ? [h('div', 'col-add dim small', 'Add a column'), chips] : []));
  };
  draw();
  return box;
}

function control(s: SettingDef, value: unknown, onChange: OnChange): HTMLElement {
  switch (s.type) {
    case 'bool': {
      const i = h('input');
      i.type = 'checkbox';
      i.className = 'switch';
      i.checked = value as boolean;
      i.onchange = () => onChange(s.key, i.checked);
      return i;
    }
    case 'number': {
      const wrap = h('span', 'num');
      const i = h('input');
      i.type = 'number';
      if (s.min != null) i.min = String(s.min);
      if (s.max != null) i.max = String(s.max);
      i.step = String(s.step ?? 1);
      i.value = String(value);
      i.oninput = () => {
        const n = parseFloat(i.value);
        if (Number.isFinite(n)) onChange(s.key, n);
      };
      wrap.append(i);
      if (s.min != null && s.max != null && s.max - s.min <= 400) {
        const r = h('input');
        r.type = 'range';
        r.min = String(s.min);
        r.max = String(s.max);
        r.step = String(s.step ?? 1);
        r.value = String(value);
        r.oninput = () => {
          i.value = r.value;
          onChange(s.key, parseFloat(r.value));
        };
        i.addEventListener('input', () => (r.value = i.value));
        wrap.prepend(r);
      }
      if (s.unit) wrap.append(h('span', 'unit', s.unit));
      return wrap;
    }
    case 'color': {
      const wrap = h('span', 'color');
      const c = h('input');
      c.type = 'color';
      c.value = String(value);
      const t = h('input');
      t.value = String(value);
      t.size = 8;
      c.oninput = () => {
        t.value = c.value;
        onChange(s.key, c.value);
      };
      t.onchange = () => {
        c.value = t.value;
        onChange(s.key, t.value);
      };
      wrap.append(c, t);
      return wrap;
    }
    case 'select': {
      const sel = h('select');
      for (const [v, l] of s.options) {
        const o = h('option', '', l);
        o.value = v;
        sel.append(o);
      }
      sel.value = String(value);
      sel.onchange = () => onChange(s.key, sel.value);
      return sel;
    }
    case 'text': {
      const i = s.multiline ? h('textarea') : h('input');
      i.value = String(value ?? '');
      if (s.placeholder) i.placeholder = s.placeholder;
      i.oninput = () => onChange(s.key, i.value);
      return i;
    }
    case 'columns':
      return columnsControl(s, value as ColumnSetting[], onChange);
  }
}

/**
 * One setting: label (with help underneath), its control, and a reset button while it
 * differs from the default.
 */
export function field(s: SettingDef, values: Values, onChange: OnChange): HTMLElement {
  const wide = s.type === 'columns' || (s.type === 'text' && !!s.multiline);
  // Columns hold several controls, so they must not be wrapped in a <label>.
  const row = h(wide ? 'div' : 'label', `field type-${s.type}${wide ? ' wide' : ''}`);
  row.dataset.key = s.key;
  const build = () => {
    const lab = h('span', 'field-label');
    lab.append(h('span', 'field-name', s.label));
    if (s.help) lab.append(h('small', 'help', s.help));
    const reset = h('button', 'reset', '↺');
    reset.title = 'Back to the default';
    reset.hidden = same(values[s.key], s.default);
    reset.onclick = (e) => {
      e.preventDefault();
      values[s.key] = structuredClone(s.default);
      onChange(s.key, values[s.key]);
      build();
    };
    const ctl = control(s, values[s.key], (k, v) => {
      values[k] = v;
      reset.hidden = same(v, s.default);
      onChange(k, v);
    });
    const end = h('span', 'field-end');
    end.append(reset);
    if (wide) row.replaceChildren(lab, ctl, end);
    else {
      end.prepend(ctl);
      row.replaceChildren(lab, end);
    }
  };
  build();
  return row;
}

/** A form with one fieldset per settings group. */
export function form(schema: SettingDef[], values: Values, onChange: OnChange): HTMLElement {
  const el = h('div', 'form');
  const groups = new Map<string, HTMLElement>();
  for (const s of schema) {
    const g = s.group ?? 'Options';
    let fs = groups.get(g);
    if (!fs) {
      fs = h('fieldset');
      fs.append(h('legend', '', g));
      groups.set(g, fs);
      el.append(fs);
    }
    fs.append(field(s, values, onChange));
  }
  return el;
}
