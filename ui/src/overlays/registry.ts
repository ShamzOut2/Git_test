// Every overlay kind. To add an overlay: create a file exporting an
// OverlayDef and list it here — the control panel picks it up automatically.

import type { OverlayDef } from '../lib/overlay';
import { battle } from './battle';
import { databox } from './databox';
import { flags } from './flags';
import { fuel } from './fuel';
import { garage61 } from './garage61';
import { incidents } from './incidents';
import { inputs } from './inputs';
import { pit } from './pit';
import { relative } from './relative';
import { session } from './session';
import { spotter } from './spotter';
import { standings } from './standings';
import { trackmap } from './trackmap';
import { tyres } from './tyres';
import { weather } from './weather';

export const OVERLAYS: OverlayDef[] = [relative, standings, trackmap, inputs, fuel, battle, session, flags, weather, tyres, databox, spotter, pit, incidents, garage61];

export const byKind = (kind: string) => OVERLAYS.find((o) => o.kind === kind);
