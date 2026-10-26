// Fixed-row grid table with user-selectable, re-orderable columns. Rows are
// created once and updated in place.

import { h, show, text, toggle } from './dom';
import { onColor } from './theme';

export interface Col<R> {
  id: string;
  label: string;
  /** CSS grid track size, e.g. '2.4em' or 'minmax(0, 1fr)'. */
  width: string;
  align?: 'left' | 'right' | 'center';
  render(cell: HTMLElement, r: R): void;
}

interface RowEl {
  el: HTMLElement;
  cells: HTMLElement[];
  key: unknown;
}

export class Table<R> {
  readonly el = h('div', 'table');
  private header = h('div', 'row head');
  private body = h('div', 'body');
  private rows: RowEl[] = [];
  private cols: Col<R>[];

  constructor(all: Col<R>[], order: string[], showHeader: boolean) {
    this.cols = order.map((id) => all.find((c) => c.id === id)).filter((c): c is Col<R> => !!c);
    this.el.style.setProperty('--cols', this.cols.map((c) => c.width).join(' '));
    for (const c of this.cols) this.header.append(h('div', `cell c-${c.id} ${c.align ?? ''}`, c.label));
    show(this.header, showHeader);
    this.el.append(this.header, this.body);
  }

  private ensure(n: number) {
    while (this.rows.length < n) {
      const el = h('div', 'row');
      const cells = this.cols.map((c) => {
        const cell = h('div', `cell c-${c.id} ${c.align ?? ''}`);
        el.append(cell);
        return cell;
      });
      this.body.append(el);
      this.rows.push({ el, cells, key: undefined });
    }
    this.rows.forEach((r, i) => show(r.el, i < n));
  }

  /** Render `data` into the table; `null` entries become empty placeholder rows. */
  render(data: (R | null)[], classes: (r: R) => Record<string, boolean>, before?: (row: HTMLElement, r: R) => void) {
    this.ensure(data.length);
    data.forEach((r, i) => {
      const row = this.rows[i];
      toggle(row.el, 'empty', r == null);
      if (r == null) {
        row.cells.forEach((c) => text(c, ''));
        return;
      }
      for (const [cls, on] of Object.entries(classes(r))) toggle(row.el, cls, on);
      before?.(row.el, r);
      this.cols.forEach((c, j) => c.render(row.cells[j], r));
    });
  }
}

/** Renders a coloured chip (class number, licence) without recreating nodes. */
export function chip(cell: HTMLElement, label: string, bg: string, fg?: string) {
  let el = cell.firstElementChild as HTMLElement | null;
  if (!el) {
    el = h('span', 'chip');
    cell.append(el);
  }
  text(el, label);
  if (el.style.background !== bg) el.style.background = bg;
  const color = fg ?? contrast(bg);
  if (el.style.color !== color) el.style.color = color;
}

export const contrast = (hex: string) => onColor(hex);
