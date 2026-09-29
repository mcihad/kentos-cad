import { describe, expect, it } from 'vitest';
import type { PolylineEntity } from '../model/entities';
import { pt, toolHarness } from '../tools/toolHarness';
import { entitiesTransform } from './entitiesTransform';

/**
 * `cad.entities.transform` on an area of several parts (docs/adr/0143), beside the shared cases: every part is
 * moved, in place or as a copy, with its holes and every elevation on its own vertex; a transform that carries a
 * coordinate of a part past the largest float64 is refused.
 */
const square = (x: number, y: number, side: number) => [pt(x, y), pt(x + side, y), pt(x + side, y + side), pt(x, y + side)];

function scene() {
  const h = toolHarness();
  const area = h.add({
    kind: 'polygon',
    pts: square(0, 0, 10),
    zs: [1, 2, 3, 4],
    holes: [{ pts: square(2, 2, 2), zs: [10, 11, 12, 13] }],
    parts: [{ pts: square(20, 0, 10), bulges: [0, 0.5, 0, 0], zs: [5, 6, 7, 8], holes: [{ pts: square(24, 4, 2), zs: [20, 21, 22, 23] }] }],
  }) as PolylineEntity;
  const input = (extra: object) => ({ uids: [h.doc.uidOf(area.id)!], transform: { kind: 'move' as const, dx: 100, dy: 50 }, ...extra });
  return { h, area, input };
}

describe('Taşı and Kopyala an area of several parts', () => {
  it('move every part with its holes; the elevations stay with their vertices', () => {
    const { h, area, input } = scene();
    expect(entitiesTransform.execute({ doc: h.doc }, input({})).status).toBe('completed');
    const e = h.doc.get(area.id) as PolylineEntity;
    expect(e.pts).toEqual(square(100, 50, 10));
    expect(e.holes).toEqual([{ pts: square(102, 52, 2), zs: [10, 11, 12, 13] }]);
    expect(e.parts).toEqual([{ pts: square(120, 50, 10), bulges: [0, 0.5, 0, 0], zs: [5, 6, 7, 8], holes: [{ pts: square(124, 54, 2), zs: [20, 21, 22, 23] }] }]);
    expect(e.zs).toEqual([1, 2, 3, 4]);
    expect(h.doc.undo()).toBe('Taşı');
    expect(h.doc.get(area.id)).toEqual(area);
  });

  it('copy every part, and leave the original as it was', () => {
    const { h, area, input } = scene();
    const r = entitiesTransform.execute({ doc: h.doc }, input({ copy: true }));
    expect(r.status).toBe('completed');
    expect(h.doc.size).toBe(2);
    expect(h.doc.get(area.id)).toEqual(area);
    const copy = [...h.doc.all()].find((e) => e.id !== area.id) as PolylineEntity;
    expect(copy.parts).toEqual([{ pts: square(120, 50, 10), bulges: [0, 0.5, 0, 0], zs: [5, 6, 7, 8], holes: [{ pts: square(124, 54, 2), zs: [20, 21, 22, 23] }] }]);
    expect(copy.uid).not.toBe(area.uid);
  });

  it('turn them all: a quarter turn about the first part’s corner', () => {
    const { h, area, input } = scene();
    entitiesTransform.execute({ doc: h.doc }, input({ transform: { kind: 'rotate', center: pt(0, 0), angle: Math.PI / 2 } }));
    const e = h.doc.get(area.id) as PolylineEntity;
    // (20, 0) is (0, 20) a quarter turn round the origin, and the arcs keep their sign.
    expect(e.parts![0].pts[0].x).toBeCloseTo(0, 9);
    expect(e.parts![0].pts[0].y).toBeCloseTo(20, 9);
    expect(e.parts![0].bulges).toEqual([0, 0.5, 0, 0]);
    expect(e.parts![0].zs).toEqual([5, 6, 7, 8]);
  });

  it('refuse a move that carries a coordinate of a part past the largest float64', () => {
    const { h, area, input } = scene();
    const huge = (h.doc.get(area.id) as PolylineEntity).parts![0].pts;
    huge[2] = pt(1e308, 1e308);
    const r = entitiesTransform.execute({ doc: h.doc }, input({ transform: { kind: 'move', dx: 1e308, dy: 0 } }));
    expect(r.status).toBe('failed');
    expect((h.doc.get(area.id) as PolylineEntity).pts).toEqual(square(0, 0, 10));
  });
});
