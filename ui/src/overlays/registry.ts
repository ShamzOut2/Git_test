// Every overlay kind. To add an overlay: create a file exporting an
// OverlayDef and list it here — the control panel picks it up automatically.

import type { OverlayDef } from '../lib/overlay';
import { battle } from './battle';
import { databox } from './databox';
import { flags } from './flags';
import { fuel } from './fuel';
import { garage61 } from './garage61';
import { inputs } from './inputs';
import { relative } from './relative';
import { session } from './session';
import { standings } from './standings';
import { trackmap } from './trackmap';
import { tyres } from './tyres';
import { weather } from './weather';

export const OVERLAYS: OverlayDef[] = [relative, standings, trackmap, inputs, fuel, battle, session, flags, weather, tyres, databox, garage61];

export const byKind = (kind: string) => OVERLAYS.find((o) => o.kind === kind);
