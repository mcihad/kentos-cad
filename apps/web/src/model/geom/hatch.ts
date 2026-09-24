import { fromXY, hatchLinesXY, toXY } from '../../wasm/core';
import type { Vec2 } from '../geometry';

/** Upper bound on generated hatch lines; beyond this the spacing is unusable anyway. */
export const MAX_HATCH_LINES = 20_000;

/**
 * Hatch lines as flat coordinates: `[capped (1/0), ax, ay, bx, by, …]`,
 * computed by the geometry core (docs/adr/0008). The hatch tool's preview
 * draws from these every frame without making points.
 */
export function hatchSegments(ring: readonly Vec2[], angleDeg: number, spacing: number, holes: readonly (readonly Vec2[])[] = []): Float64Array {
  return hatchLinesXY(toXY(ring), toXY(holes.flat()), Uint32Array.from(holes, (h) => h.length), angleDeg, spacing);
}

/**
 * Parallel hatch lines clipped to a ring and its holes (even–odd rule).
 * Lines sit on world-anchored multiples of `spacing`, so neighbouring
 * hatches with the same pattern line up across shared boundaries.
 *
 * @param angleDeg direction of the lines, CCW from east
 * @returns segment endpoints as [a, b] pairs and whether output was capped
 */
export function hatchLines(ring: readonly Vec2[], angleDeg: number, spacing: number, holes: readonly (readonly Vec2[])[] = []): { segments: [Vec2, Vec2][]; capped: boolean } {
  const xy = hatchSegments(ring, angleDeg, spacing, holes);
  const pts = fromXY(xy, 1);
  const segments: [Vec2, Vec2][] = new Array(pts.length >> 1);
  for (let i = 0; i < segments.length; i++) segments[i] = [pts[2 * i], pts[2 * i + 1]];
  return { segments, capped: xy[0] === 1 };
}
