// "Show Flag Abbreviations": the country codes the league roster's Flag column accepts,
// with each flag and country name, searchable, click a row to copy its code.

import { h, text } from '../lib/dom';
import { flagEmoji } from '../overlays/common';

/** Region codes the browser knows a name for but that have no flag: groupings, test codes,
 *  aliases (UK) and countries that no longer exist. */
const NOT_COUNTRIES = new Set(['AN', 'BU', 'CQ', 'CS', 'DD', 'DY', 'EZ', 'FX', 'HV', 'NH', 'QO', 'RH', 'SU', 'TP', 'UK', 'VD', 'XA', 'XB', 'YD', 'YU', 'ZR', 'ZZ']);
const EXTRA: [string, string][] = [
  ['GB-ENG', 'England'],
  ['GB-SCT', 'Scotland'],
  ['GB-WLS', 'Wales'],
  ['GLOBE', 'Global (iRacing’s “Global” flair)'],
];

let cache: [string, string][] | null = null;

function codes(): [string, string][] {
  if (cache) return cache;
  const names = new Intl.DisplayNames(['en'], { type: 'region', fallback: 'code' });
  const out: [string, string][] = [];
  for (let a = 65; a <= 90; a++) {
    for (let b = 65; b <= 90; b++) {
      const code = String.fromCharCode(a, b);
      const name = names.of(code);
      if (name && name !== code && !NOT_COUNTRIES.has(code)) out.push([code, name]);
    }
  }
  out.push(...EXTRA);
  out.sort((x, y) => x[1].localeCompare(y[1]));
  return (cache = out);
}

export function showFlagCodes() {
  const back = h('div', 'modal-back');
  const dlg = h('div', 'modal flag-codes');
  const close = () => back.remove();
  back.onclick = (e) => {
    if (e.target === back) close();
  };
  const onKey = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      close();
      document.removeEventListener('keydown', onKey);
    }
  };
  document.addEventListener('keydown', onKey);
  const head = h('div', 'modal-head');
  const x = h('button', 'subtle', '✕');
  x.onclick = (e) => {
    e.preventDefault();
    close();
  };
  head.append(h('h2', '', 'Flag abbreviations'), x);
  const note = h('p', 'dim small', 'Type the code in a driver’s Flag box. Click a row to copy its code.');
  const search = h('input', 'search');
  search.type = 'search';
  search.placeholder = 'Search a country or code…';
  const list = h('div', 'flag-list');
  const toast = h('div', 'flag-toast');
  const draw = () => {
    const q = search.value.trim().toLowerCase();
    const rows = codes().filter(([c, n]) => !q || c.toLowerCase().includes(q) || n.toLowerCase().includes(q));
    list.replaceChildren(
      ...rows.map(([code, name]) => {
        const row = h('button', 'flag-row');
        row.title = `Copy ${code}`;
        row.append(h('span', 'flag-emoji', flagEmoji(code)), h('span', 'flag-code', code), h('span', 'flag-name', name));
        row.onclick = (e) => {
          e.preventDefault();
          navigator.clipboard?.writeText(code);
          text(toast, `Copied ${code}`);
          toast.classList.add('on');
          setTimeout(() => toast.classList.remove('on'), 1200);
        };
        return row;
      }),
    );
    if (!rows.length) list.append(h('p', 'dim', `No country matches “${search.value}”.`));
  };
  search.oninput = draw;
  draw();
  dlg.append(head, note, search, list, toast);
  back.append(dlg);
  document.body.append(back);
  search.focus();
}
