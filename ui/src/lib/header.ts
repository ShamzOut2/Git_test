// Customisable session header for table overlays (relative, standings).

import { h, show, text } from './dom';
import type { Units } from './format';
import { fieldValue, flagKind, headerFields } from './sessionFields';
import { enabledColumns, type SettingDef, type Values } from './settings';
import type { Session } from './types';

export function headerSettings(on: string[]): SettingDef[] {
  return [
    { key: 'showHeader', label: 'Show header', type: 'bool', default: true, group: 'Header' },
    {
      key: 'headerTitle',
      label: 'Header title',
      type: 'select',
      default: 'session',
      options: [
        ['session', 'Session name'],
        ['track', 'Track name'],
        ['league', 'League name'],
        ['custom', 'Custom text'],
        ['none', 'No title'],
      ],
      group: 'Header',
    },
    { key: 'headerCustomTitle', label: 'Custom title', type: 'text', default: '', placeholder: 'My League · Round 3', group: 'Header' },
    { key: 'headerFields', label: 'Session details in the header', type: 'columns', ...headerFields(on), group: 'Header' },
    { key: 'headerSeparator', label: 'Separator', type: 'text', default: '  ·  ', group: 'Header' },
  ];
}

export function sessionHeader(s: Values, fallbackTitle: string, units: () => Units) {
  const el = h('div', 'header');
  const title = h('span', 'title', fallbackTitle);
  const info = h('span', 'info');
  el.append(title, info);
  show(el, s.showHeader as boolean);
  show(title, s.headerTitle !== 'none');
  const fields = enabledColumns(s.headerFields);
  return {
    el,
    update(d: Session) {
      const t = s.headerTitle === 'custom' ? (s.headerCustomTitle as string) : s.headerTitle === 'none' ? '' : fieldValue(s.headerTitle as string, d, units(), { compact: true });
      text(title, t || fallbackTitle);
      const parts = fields.map((id) => fieldValue(id, d, units(), { compact: true })).filter(Boolean);
      text(info, parts.join(s.headerSeparator as string));
      info.dataset.flag = fields.includes('flags') ? flagKind(d) : '';
    },
  };
}
