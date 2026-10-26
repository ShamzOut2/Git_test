// Minimal DOM helpers. Overlays update text/attributes only when values
// change, so a 60 Hz feed costs almost nothing in layout and paint.

export function h<K extends keyof HTMLElementTagNameMap>(tag: K, cls = '', text = ''): HTMLElementTagNameMap[K] {
  const el = document.createElement(tag);
  if (cls) el.className = cls;
  if (text) el.textContent = text;
  return el;
}

export function text(el: HTMLElement, value: string) {
  if (el.textContent !== value) el.textContent = value;
}

export function toggle(el: Element, cls: string, on: boolean) {
  if (el.classList.contains(cls) !== on) el.classList.toggle(cls, on);
}

export function style(el: HTMLElement, prop: string, value: string) {
  if (el.style.getPropertyValue(prop) !== value) el.style.setProperty(prop, value);
}

export function show(el: HTMLElement, visible: boolean) {
  const d = visible ? '' : 'none';
  if (el.style.display !== d) el.style.display = d;
}

/** Canvas sized to its CSS box at device pixel ratio. Returns true if resized. */
export function fitCanvas(c: HTMLCanvasElement): boolean {
  const dpr = window.devicePixelRatio || 1;
  const w = Math.max(1, Math.round(c.clientWidth * dpr));
  const hgt = Math.max(1, Math.round(c.clientHeight * dpr));
  if (c.width !== w || c.height !== hgt) {
    c.width = w;
    c.height = hgt;
    return true;
  }
  return false;
}
