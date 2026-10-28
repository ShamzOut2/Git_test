// A hotkey box: click it to record, press a key combination and it is saved; Backspace clears it.
// Keys are stored the way the desktop app's global shortcuts read them, e.g. "Ctrl+Shift+1".

import { h, text } from '../lib/dom';

const MODIFIERS = new Set(['Control', 'Shift', 'Alt', 'Meta', 'OS']);

/** A key from a KeyboardEvent.code, in a readable form the desktop app accepts. */
function keyName(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code;
  if (/^Numpad[0-9]$/.test(code)) return `Num${code.slice(6)}`;
  if (/^Arrow(Up|Down|Left|Right)$/.test(code)) return code.slice(5);
  const named: Record<string, string> = {
    NumpadAdd: 'NumAdd',
    NumpadSubtract: 'NumSubtract',
    NumpadMultiply: 'NumMultiply',
    NumpadDivide: 'NumDivide',
    NumpadDecimal: 'NumDecimal',
    NumpadEnter: 'NumEnter',
    Space: 'Space',
    Enter: 'Enter',
    Tab: 'Tab',
    Insert: 'Insert',
    Delete: 'Delete',
    Home: 'Home',
    End: 'End',
    PageUp: 'PageUp',
    PageDown: 'PageDown',
    Minus: '-',
    Equal: '=',
    BracketLeft: '[',
    BracketRight: ']',
    Semicolon: ';',
    Quote: "'",
    Comma: ',',
    Period: '.',
    Slash: '/',
    Backslash: '\\',
    Backquote: '`',
    Pause: 'Pause',
    ScrollLock: 'ScrollLock',
  };
  return named[code] ?? null;
}

/** Keys that are fine on their own (no Ctrl/Alt/Shift): they don't get in the way of typing. */
const ALONE = /^(F\d+|Num.+|Pause|ScrollLock|Insert|Home|End|PageUp|PageDown)$/;

export function hotkeyInput(value: string, onChange: (v: string) => void): HTMLElement {
  const wrap = h('span', 'hotkey');
  const box = h('button', 'hotkey-box');
  const clear = h('button', 'mini subtle', '✕');
  clear.title = 'Remove this hotkey';
  const saved = h('span', 'hotkey-saved', 'Saved ✓');
  box.title = 'Click, then press the keys you want';
  let current = value;
  let flash = 0;
  const show = (msg?: string) => {
    box.classList.toggle('unset', !current && !msg);
    text(box, msg ?? (current.replace(/^(CmdOrCtrl|CommandOrControl)\+/i, 'Ctrl+') || 'Click to record'));
    clear.hidden = !current;
  };
  const set = (v: string) => {
    current = v;
    onChange(v);
    box.classList.remove('listening');
    box.blur();
    show();
    saved.textContent = v ? 'Saved ✓' : 'Cleared';
    saved.classList.add('on');
    clearTimeout(flash);
    flash = window.setTimeout(() => saved.classList.remove('on'), 1400);
  };
  box.onclick = (e) => {
    e.preventDefault();
    box.classList.add('listening');
    show('Recording… press keys');
  };
  box.onblur = () => {
    box.classList.remove('listening');
    show();
  };
  box.onkeydown = (e) => {
    if (!box.classList.contains('listening')) return;
    e.preventDefault();
    e.stopPropagation();
    const mods = [e.ctrlKey && 'Ctrl', e.altKey && 'Alt', e.shiftKey && 'Shift', e.metaKey && 'Super'].filter(Boolean) as string[];
    if (e.key === 'Escape' && !mods.length) {
      box.classList.remove('listening');
      box.blur();
      return show();
    }
    if ((e.key === 'Backspace' || e.key === 'Delete') && !mods.length) return set('');
    if (MODIFIERS.has(e.key)) return show(`${mods.join('+')}+…`);
    const key = keyName(e.code);
    if (!key) return show('That key can’t be used — try another');
    if (!mods.length && !ALONE.test(key)) return show('Add Ctrl, Alt or Shift to that key');
    set([...mods, key].join('+'));
  };
  clear.onclick = (e) => {
    e.preventDefault();
    set('');
  };
  show();
  wrap.append(saved, box, clear);
  return wrap;
}
