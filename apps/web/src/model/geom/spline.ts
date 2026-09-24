import { op } from '../../wasm/core';
import type { Vec2 } from '../geometry';

/**
 * Centripetal Catmull-Rom curve through fit points (Barry–Goldman
 * evaluation, α = 0.5), `perSpan` points a span (16 by default). It passes
 * through every point, never forms cusps or self-loops within a span, and
 * is invariant under similarity transforms — so rotating/scaling/mirroring
 * the fit points is exact. Computed by the geometry core (docs/adr/0008).
 */
export const catmullRom = op<(pts: readonly Vec2[], closed: boolean, perSpan?: number) => Vec2[]>('catmullRom');
