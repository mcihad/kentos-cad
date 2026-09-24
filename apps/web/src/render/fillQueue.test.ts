import { describe, expect, it } from 'vitest';
import type { Vec2 } from '../model/geometry';
import { callNamed } from '../wasm/core';
import { Gen } from '../wasm/calls/harness';
import { fillPolygon } from '../wasm/calls/sets/p8-triangulate';
import { FillQueue } from './fillQueue';

/**
 * A layer's fills triangulated together (`FillQueue` over `triangulateMany`)
 * give each fill exactly the triangles the named operation gives it alone,
 * into its own array and in the order queued, degenerate rings included.
 */
describe('FillQueue', () => {
  it('gives each fill the triangles of triangulating it alone', () => {
    const g = new Gen(2602);
    const outs: number[][] = [[], [], []];
    const want: number[][] = [[], [], []];
    const queue = new FillQueue();
    const origin: Vec2 = { x: 486_000, y: 4_420_000 };
    for (let i = 0; i < 300; i++) {
      g.frame();
      // Now and then a ring that has no triangles (empty, or two points).
      const [outer, holes]: [Vec2[], Vec2[][]] = g.chance(0.05) ? [g.chance(0.5) ? [] : [{ x: 1, y: 1 }, { x: 2, y: 2 }], []] : fillPolygon(g);
      const k = g.int(0, 2);
      queue.add(outs[k], [outer, ...holes]);
      want[k].push(...(callNamed('triangulate', [outer, holes, origin]) as number[]));
    }
    queue.run(origin);
    expect(outs).toEqual(want);
    // The queue is empty again: another run adds nothing.
    queue.run(origin);
    expect(outs).toEqual(want);
  });
});
