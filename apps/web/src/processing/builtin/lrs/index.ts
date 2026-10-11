import type { ProcessingTool } from '../../types';
import { overlayEvents, routeEvents } from './events';
import { locateAlong } from './locate';
import { calibrateRoutes, createRoutes, extractMeasures } from './routes';

/** Doğrusal referans (docs/adr/0224 §5): routes made and calibrated, events shown and overlaid, objects located, measures extracted. */
export const LRS_TOOLS: readonly ProcessingTool[] = [createRoutes, calibrateRoutes, routeEvents, locateAlong, overlayEvents, extractMeasures];
