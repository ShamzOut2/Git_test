// Contract between the overlay host page and individual overlays.

import type { Units } from './format';
import type { SettingDef, Values } from './settings';
import type { TopicMap, TopicName } from './types';

export interface Ctx {
  /** Merged settings (schema defaults + user values). */
  settings: Values;
  /** Merged theme values (for canvas overlays that need colours). */
  theme: Values;
  units: Units;
  latest: Partial<TopicMap>;
  send(msg: object): void;
}

export interface View {
  update(topic: TopicName, data: any): void;
  /** Called every animation frame while visible (optional). */
  frame?(): void;
}

export interface OverlayDef {
  kind: string;
  name: string;
  description: string;
  topics: TopicName[];
  /** Default window size in CSS pixels. */
  size: [number, number];
  settings: SettingDef[];
  create(root: HTMLElement, ctx: Ctx): View;
}
