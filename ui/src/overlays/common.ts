// Column renderers shared by relative, standings and battle.

import { text } from '../lib/dom';
import * as f from '../lib/format';
import { chip, type Col } from '../lib/table';
import type { Driver } from '../lib/types';

export function driverCols<R extends Driver>(nameStyle: string): Col<R>[] {
  return [
    {
      id: 'number',
      label: '#',
      width: '2.9em',
      align: 'center',
      render: (c, r) => chip(c, r.number, r.classColor),
    },
    {
      id: 'name',
      label: 'Driver',
      width: 'minmax(4em, 1fr)',
      render: (c, r) => text(c, f.name(r.name, nameStyle)),
    },
    {
      id: 'team',
      label: 'Team',
      width: 'minmax(3em, 0.8fr)',
      render: (c, r) => text(c, r.team === r.name ? '' : r.team),
    },
    {
      id: 'license',
      label: 'Lic',
      width: '3.4em',
      align: 'center',
      render: (c, r) => chip(c, r.license.split(' ')[0] + ' ' + (r.license.split(' ')[1] ?? '').slice(0, 3), r.licenseColor),
    },
    {
      id: 'irating',
      label: 'iR',
      width: '3em',
      align: 'right',
      render: (c, r) => text(c, r.irating > 0 ? f.iRating(r.irating) : ''),
    },
    {
      id: 'car',
      label: 'Car',
      width: 'minmax(3em, 0.7fr)',
      render: (c, r) => text(c, r.car),
    },
    {
      id: 'class',
      label: 'Class',
      width: '3.6em',
      align: 'center',
      render: (c, r) => chip(c, r.className, r.classColor),
    },
  ];
}

export function tireCell(c: HTMLElement, wet: boolean | null) {
  text(c, wet == null ? '' : wet ? 'W' : 'D');
  c.classList.toggle('wet', wet === true);
}
