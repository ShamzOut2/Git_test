// Renders forms from a settings schema (see lib/settings.ts).

import { h } from '../lib/dom';
import type { ColumnSetting, SettingDef, Values } from '../lib/settings';

export type OnChange = (key: string, value: unknown) => void;

function field(s: SettingDef, value: unknown, onChange: OnChange): HTMLElement {
  // Columns hold several controls, so they must not be wrapped in a <label>.
  const row = h(s.type === 'columns' ? 'div' : 'label', `field type-${s.type}`);
  const lab = h('span', 'field-label', s.label);
  if (s.help) lab.append(h('small', 'help', s.help));
  row.append(lab);
  switch (s.type) {
    case 'bool': {
      const i = h('input');
      i.type = 'checkbox';
      i.className = 'switch';
      i.checked = value as boolean;
      i.onchange = () => onChange(s.key, i.checked);
      row.append(i);
      break;
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
      row.append(wrap);
      break;
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
      row.append(wrap);
      break;
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
      row.append(sel);
      break;
    }
    case 'text': {
      const i = s.multiline ? h('textarea') : h('input');
      i.value = String(value ?? '');
      if (s.placeholder) i.placeholder = s.placeholder;
      i.oninput = () => onChange(s.key, i.value);
      row.append(i);
      if (s.multiline) row.classList.add('wide');
      break;
    }
    case 'columns': {
      row.classList.add('wide');
      const list = h('div', 'columns');
      const cols = (value as ColumnSetting[]).map((c) => ({ ...c }));
      const label = (id: string) => s.columns.find((c) => c.id === id)?.label ?? id;
      const draw = () => {
        list.replaceChildren(
          ...cols.map((c, i) => {
            const item = h('div', 'col-item');
            const cb = h('input');
            cb.type = 'checkbox';
            cb.checked = c.on;
            cb.onchange = () => {
              c.on = cb.checked;
              onChange(s.key, cols.map((x) => ({ ...x })));
            };
            const up = h('button', 'mini', '▲');
            const down = h('button', 'mini', '▼');
            up.disabled = i === 0;
            down.disabled = i === cols.length - 1;
            const move = (d: number) => (e: Event) => {
              e.preventDefault();
              const [x] = cols.splice(i, 1);
              cols.splice(i + d, 0, x);
              draw();
              onChange(s.key, cols.map((y) => ({ ...y })));
            };
            up.onclick = move(-1);
            down.onclick = move(1);
            item.append(cb, h('span', '', label(c.id)), up, down);
            return item;
          }),
        );
      };
      draw();
      row.append(list);
      break;
    }
  }
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
    fs.append(field(s, values[s.key], onChange));
  }
  return el;
}
