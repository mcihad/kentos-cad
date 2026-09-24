import { describe, expect, it } from 'vitest';
import type { Vec2 } from '../model/geometry';
import { constrainCursor } from './tracking';

/** The ortho and polar cursor, from the Rust core (docs/adr/0008, S5). */

const v = (x: number, y: number): Vec2 => ({ x, y });
const E = 486512.34;
const N = 4420118.9;

describe('constrainCursor', () => {
  it('leaves exact points and the first point alone', () => {
    expect(constrainCursor(null, v(5, 7), false, true, 15, 1)).toEqual({ point: v(5, 7), tracking: null });
    expect(constrainCursor(v(0, 0), v(5, 7), true, true, 15, 1)).toEqual({ point: v(5, 7), tracking: null });
  });
  it('keeps the larger of the two moves in ortho', () => {
    expect(constrainCursor(v(E, N), v(E + 10, N + 3), false, true, null, 1).point).toEqual(v(E + 10, N));
    expect(constrainCursor(v(E, N), v(E + 3, N - 10), false, true, 15, 1).point).toEqual(v(E, N - 10));
  });
  it('locks onto the nearest polar ray within the tolerance', () => {
    const r = constrainCursor(v(E, N), v(E + 10, N + 10.3), false, false, 45, 0.5);
    expect(r.tracking).toEqual({ origin: v(E, N), angle: 45 });
    expect(r.point.x - E).toBeCloseTo(10.15, 8);
    expect(r.point.y - N).toBeCloseTo(10.15, 8);
    // Just below east is 0°, not 360°.
    expect(constrainCursor(v(0, 0), v(10, -0.1), false, false, 30, 1).tracking!.angle).toBe(0);
  });
  it('lets the cursor go when it is off every ray or polar tracking is off', () => {
    expect(constrainCursor(v(0, 0), v(10, 3), false, false, 90, 1)).toEqual({ point: v(10, 3), tracking: null });
    expect(constrainCursor(v(0, 0), v(10, 0.2), false, false, null, 1)).toEqual({ point: v(10, 0.2), tracking: null });
  });
});
