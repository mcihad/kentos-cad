import { describe, expect, it } from 'vitest';
import type { Entity, PolylineEntity } from '../model/entities';
import { CoreStore } from './core';
import { packEntities, unpackEntities } from './pack';
import { Gen } from './calls/harness';
import { entity } from './calls/sets/p5-entities';

/**
 * Packed objects (./pack.ts → crates/shared/geometry-core/src/store/pack.rs) must
 * build the very objects their JSON builds: every kind, with bulges, holes,
 * optional fields present and absent, labels and layers.
 */
describe('packEntities', () => {
  it('gives the store the same objects as JSON', () => {
    const g = new Gen(1_2026);
    const list: Entity[] = [];
    for (let i = 1; i <= 3000; i++) {
      g.frame();
      list.push({ ...entity(g), id: i });
    }
    list.push(
      { id: 5001, layerId: 'a', attrs: {}, kind: 'point', p: { x: 1, y: 2 }, z: 850.5 },
      { id: 5002, layerId: 'a', attrs: {}, kind: 'dimension', a: { x: 0, y: 0 }, b: { x: 3, y: 4 }, c: { x: 1, y: 1 }, offset: 2, height: 0.5, style: 'angular', text: '' },
      { id: 5003, layerId: 'a', attrs: {}, kind: 'dimension', a: { x: 0, y: 0 }, b: { x: 3, y: 4 }, offset: -2, height: 0.5, style: 'linear', angle: 90, text: 'Çıkma' },
      { id: 5004, layerId: 'b', attrs: {}, kind: 'hatch', ring: [{ x: 0, y: 0 }, { x: 4, y: 0 }, { x: 4, y: 4 }], holes: [[{ x: 1, y: 1 }, { x: 2, y: 1 }, { x: 2, y: 2 }]], pattern: { type: 'cross', angle: 30, spacing: 0.5 } },
      { id: 5005, layerId: 'b', attrs: {}, kind: 'polygon', pts: [{ x: 0, y: 0 }, { x: 4, y: 0 }, { x: 4, y: 4 }], bulges: [0.2, 0, 0], holes: [{ pts: [{ x: 1, y: 1 }, { x: 2, y: 1 }, { x: 2, y: 2 }], bulges: [0, 0.3, 0] }], label: '12' },
      { id: 5006, layerId: 'b', attrs: {}, kind: 'polyline', pts: [] },
      { id: 5007, layerId: 'b', attrs: {}, kind: 'text', p: { x: 1, y: 1 }, text: 'Ada 104 😀', height: 2, rotation: -30 },
    );
    const packed = new CoreStore();
    const p = packEntities(list);
    packed.putPacked(p.nums, p.strings);
    const json = new CoreStore();
    json.put(JSON.stringify(list));
    expect(packed.size).toBe(list.length);
    const diff = list.map((e) => e.id).filter((id) => packed.itemJson(id) !== json.itemJson(id));
    expect(diff.slice(0, 3).map((id) => `${packed.itemJson(id)}\n≠ ${json.itemJson(id)}`).join('\n')).toBe('');
    expect(Array.from(packed.ids())).toEqual(Array.from(json.ids()));
    packed.dispose();
    json.dispose();
  });

  it('keeps −0 and NaN that JSON would lose', () => {
    const s = new CoreStore();
    const p = packEntities([{ id: 1, layerId: 'a', attrs: {}, kind: 'line', a: { x: -0, y: NaN }, b: { x: 1, y: 1 } }]);
    s.putPacked(p.nums, p.strings);
    expect(s.itemJson(1)).toBe('{"id":1,"layerId":"a","label":false,"kind":"line","a":{"x":-0,"y":"#NaN"},"b":{"x":1,"y":1}}');
    expect(() => s.putPacked(packEntities([{ id: 2, layerId: 'a', kind: 'daire' }]).nums, '[]')).toThrow();
    s.dispose();
  });
});

const square = (x: number, y: number, side: number) => [
  { x, y },
  { x: x + side, y },
  { x: x + side, y: y + side },
  { x, y: y + side },
];

/** An area of three parts: arcs and a hole in the first, a hole in the second, arcs in the third (docs/adr/0143). */
const threeParts = (id: number): PolylineEntity => ({
  id,
  layerId: 'a',
  attrs: {},
  label: '7',
  kind: 'polygon',
  pts: square(0, 0, 10),
  bulges: [0, 0.4, 0, 0],
  holes: [{ pts: square(2, 2, 3), bulges: [0, 0, -0.2, 0] }],
  parts: [
    { pts: square(20, 0, 10), holes: [{ pts: square(22, 2, 3) }, { pts: square(26, 6, 2) }] },
    { pts: [{ x: 40, y: 0 }, { x: 50, y: 0 }], bulges: [0.5, 0.5] },
  ],
});

describe('a multi-part area packed (docs/adr/0143)', () => {
  it('gives the store the very area its JSON gives, every part with its arcs and holes', () => {
    const list = [threeParts(1), { ...threeParts(2), parts: [{ pts: square(30, 30, 4) }], holes: undefined, bulges: undefined }];
    const packed = new CoreStore();
    const p = packEntities(list);
    packed.putPacked(p.nums, p.strings);
    const json = new CoreStore();
    json.put(JSON.stringify(list));
    expect(packed.itemJson(1)).toBe(json.itemJson(1));
    expect(packed.itemJson(2)).toBe(json.itemJson(2));
    expect(packed.itemJson(1)).toContain('"parts":[{"pts"');
    packed.dispose();
    json.dispose();
  });

  it('is kind 13 in the layout: the first part as a polygon, the count of the others, each of them as a path', () => {
    const { nums } = packEntities([threeParts(1)]);
    // id, layer, label, kind; then the first part's points (4), its arcs (4), its holes (1: 4 points, 4 arcs) …
    expect(Array.from(nums.subarray(0, 4))).toEqual([1, 0, 1, 13]);
    // … then 2 more parts after the first's fields.
    const first = 4 + (1 + 8) + (1 + 4) + 1 + (1 + 8) + (1 + 4);
    expect(nums[first]).toBe(2);
    // The first other part: 4 points, no arcs (−1), 2 holes.
    expect([nums[first + 1], nums[first + 1 + 1 + 8], nums[first + 1 + 1 + 8 + 1]]).toEqual([4, -1, 2]);
  });

  it('a one-part area stays as it always was: kind 3, whatever the empty parts say', () => {
    const one = { id: 1, layerId: 'a', attrs: {}, kind: 'polygon' as const, pts: [{ x: 0, y: 0 }, { x: 4, y: 0 }, { x: 4, y: 4 }] };
    const layout = [1, 0, 0, 3, 3, 0, 0, 4, 0, 4, 4, -1, -1];
    expect(Array.from(packEntities([one]).nums)).toEqual(layout);
    expect(Array.from(packEntities([{ ...one, parts: [] }]).nums)).toEqual(layout);
    expect(Array.from(packEntities([{ ...one, parts: undefined }]).nums)).toEqual(layout);
  });

  it('is read back as it was, bit for bit, parts in their order', () => {
    const list = [threeParts(1), { ...threeParts(2), parts: [{ pts: [{ x: -0, y: 1e-300 }, { x: 5, y: 0 }, { x: 5, y: 5 }] }] }];
    const back = unpackEntities(packEntities(list));
    expect(back.map((r) => r.id)).toEqual([1, 2]);
    for (const [i, r] of back.entries()) {
      const { id: _i, layerId: _l, label: _t, attrs: _a, ...geometry } = list[i];
      expect(r.geometry).toEqual(geometry);
      expect(r.labelled).toBe(true);
    }
    expect(Object.is((back[1].geometry.parts as { pts: { x: number }[] }[])[0].pts[0].x, -0)).toBe(true);
    // The fields come in the order the core's JSON writes them.
    expect(Object.keys(back[0].geometry)).toEqual(['kind', 'pts', 'bulges', 'holes', 'parts']);
    expect(Object.keys((back[0].geometry.parts as object[])[0])).toEqual(['pts', 'holes']);
  });

  it('comes back from the store moved, part after part (transformPacked)', () => {
    const store = new CoreStore();
    const p = packEntities([threeParts(1)]);
    store.putPacked(p.nums, p.strings);
    const moved = unpackEntities(store.transformPacked(Float64Array.of(1), Float64Array.of(1, 0, 0, 1, 100, 200)));
    const parts = moved[0].geometry.parts as { pts: { x: number; y: number }[] }[];
    expect(parts.map((q) => q.pts[0])).toEqual([{ x: 120, y: 200 }, { x: 140, y: 200 }]);
    expect((moved[0].geometry.pts as { x: number; y: number }[])[0]).toEqual({ x: 100, y: 200 });
    store.dispose();
  });
});

