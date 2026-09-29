import { describe, expect, it } from 'vitest';
import { CoreStore } from './core';

/**
 * The selection queries of docs/adr/0141 through the WASM store, on the
 * hand-worked scenes of the Rust tests (crates/shared/geometry-core/src/store/select.rs):
 * the closed shapes around a point, what a fence crosses, what a circle
 * holds or touches, and the objects far from the rest of the drawing.
 */

type Item = Record<string, unknown>;

const square = (id: number, x: number, y: number, s: number): Item => ({
  id,
  layerId: 'a',
  attrs: {},
  kind: 'polygon',
  pts: [
    { x, y },
    { x: x + s, y },
    { x: x + s, y: y + s },
    { x, y: y + s },
  ],
});
const point = (id: number, x: number, y: number): Item => ({ id, layerId: 'a', attrs: {}, kind: 'point', p: { x, y } });
const line = (id: number, a: [number, number], b: [number, number]): Item => ({
  id,
  layerId: 'a',
  attrs: {},
  kind: 'line',
  a: { x: a[0], y: a[1] },
  b: { x: b[0], y: b[1] },
});

function withStore(items: Item[], check: (s: CoreStore) => void): void {
  const s = new CoreStore();
  try {
    s.put(JSON.stringify(items));
    check(s);
  } finally {
    s.dispose();
  }
}

describe('selection queries (docs/adr/0141)', () => {
  it('gives the closed shapes around a point smallest first, with their areas', () => {
    withStore([square(1, -50, -50, 100), square(2, 0, 0, 10), square(3, -10, -10, 40), square(4, 20, 20, 5), line(5, [0, 5], [10, 5])], (s) => {
      expect(Array.from(s.containing(5, 5))).toEqual([2, 100, 3, 1600, 1, 10_000]);
      expect(s.containing(500, 500).length).toBe(0);
    });
  });

  it('takes what a fence crosses and a point within the tolerance', () => {
    withStore(
      [line(1, [0, 0], [0, 10]), line(2, [5, 0], [5, 10]), line(3, [20, 0], [20, 10]), square(4, 8, 4, 2), point(5, 12, 5.05), point(6, 12, 7)],
      (s) => {
        expect(Array.from(s.inFence(Float64Array.from([-1, 5, 14, 5]), 0.1))).toEqual([1, 2, 4, 5]);
        expect(s.inFence(Float64Array.from([8.5, 5, 9.5, 5]), 0).length).toBe(0);
      },
    );
  });

  it('holds what is inside a circle and, crossing, what it touches', () => {
    withStore([square(1, -1, -1, 2), line(2, [0, 0], [20, 0]), point(3, 3, 0), square(4, -100, -100, 200), point(5, 30, 30)], (s) => {
      expect(Array.from(s.inCircle(0, 0, 5, false))).toEqual([1, 3]);
      expect(Array.from(s.inCircle(0, 0, 5, true))).toEqual([1, 2, 3, 4]);
    });
  });

  it('finds the objects far from the drawing and leaves the drawing alone', () => {
    const parcels = Array.from({ length: 100 }, (_, i) => square(i + 1, 500_000 + (i % 10) * 20, 4_400_000 + Math.floor(i / 10) * 20, 15));
    withStore([...parcels, square(200, 0, 0, 15), point(201, 530_000, 4_400_000), square(202, 501_000, 4_400_000, 15)], (s) => {
      expect(Array.from(s.extentOutliers())).toEqual([200, 201]);
    });
  });
});
