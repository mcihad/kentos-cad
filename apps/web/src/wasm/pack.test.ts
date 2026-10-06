import { describe, expect, it } from 'vitest';
import type { Entity, PolylineEntity } from '../model/entities';
import { CoreStore } from './core';
import { packEntities, unpackEntities } from './pack';
import { Gen } from './calls/harness';
import { entity } from './calls/sets/p5-entities';

/** Hatches of docs/adr/0186's kinds: a pattern of families with dashes and dots, tied to its objects; a gradient. */
const HATCHES = [
  {
    kind: 'hatch',
    ring: [{ x: 0, y: 0 }, { x: 4, y: 0 }, { x: 4, y: 4 }],
    pattern: {
      type: 'pattern',
      angle: 15,
      spacing: 1,
      name: 'ANSI35',
      scale: 0.5,
      lines: [
        { angle: 45, origin: [0, 0], offset: [0, 6.35] },
        { angle: 45, origin: [4.490128, 0], offset: [0, 6.35], dashes: [7.9375, -1.5875, 0, -1.5875] },
      ],
    },
    assoc: { outer: '0192a3b4-c5d6-7e8f-9012-3456789abcde', islands: ['0192a3b4-c5d6-7e8f-9012-3456789abcdf'], seed: { x: 1, y: 0.5 } },
  },
  { kind: 'hatch', ring: [{ x: 0, y: 0 }, { x: 4, y: 0 }, { x: 4, y: 4 }], pattern: { type: 'gradient', angle: 90, spacing: 1, gradient: { shape: 'spherical', inverted: true, color2: '#FFFFFF' } } },
  { kind: 'hatch', ring: [{ x: 0, y: 0 }, { x: 4, y: 0 }, { x: 4, y: 4 }], pattern: { type: 'gradient', angle: 0, spacing: 1, gradient: { shape: 'linear', color2: '#102030' } }, assoc: { outer: '0192a3b4-c5d6-7e8f-9012-3456789abce0', cutouts: ['0192a3b4-c5d6-7e8f-9012-3456789abce1'], seed: { x: 2, y: 1 } } },
];

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
      // docs/adr/0147: the new kinds, a mask and a slope's elevations.
      { id: 5008, layerId: 'a', attrs: {}, kind: 'dimension', a: { x: 0, y: 0 }, b: { x: 40, y: 0 }, offset: 1.5, height: 2, style: 'slope', za: 105.25, zb: 104.75, mask: true },
      { id: 5009, layerId: 'a', attrs: {}, kind: 'dimension', a: { x: 10, y: 20 }, b: { x: 30, y: 26 }, offset: 0, height: 2.5, style: 'ordinate', angle: 90 },
      { id: 5010, layerId: 'a', attrs: {}, kind: 'dimension', a: { x: 60, y: 0 }, b: { x: 50, y: 10 }, c: { x: 50, y: 0 }, offset: 2, height: 2, style: 'arcLength' },
      // docs/adr/0183: a text's style and face, a dimension's style and look with its value's writing.
      { id: 5011, layerId: 'a', attrs: {}, kind: 'text', p: { x: 2, y: 3 }, text: 'Ada 105', height: 2, rotation: 0, textStyle: '0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0101', font: 'arimo', bold: true, italic: true, oblique: 15 },
      { id: 5012, layerId: 'a', attrs: {}, kind: 'text', p: { x: 2, y: 3 }, text: 'Not', height: 2, rotation: 0, font: 'plex-mono' },
      {
        id: 5013,
        layerId: 'a',
        attrs: {},
        kind: 'dimension',
        a: { x: 0, y: 0 },
        b: { x: 8, y: 0 },
        offset: 2,
        height: 0.5,
        dimStyle: '0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0201',
        arrow: 'closed',
        arrowSize: 1.2,
        extOffset: 0,
        extBeyond: 0.8,
        textGap: 0.5,
        textPlace: 'centre',
        decimals: 0,
        unit: 'cm',
        prefix: 'L=',
        suffix: ' cm',
        font: 'courier-prime',
      },
      { id: 5014, layerId: 'a', attrs: {}, kind: 'dimension', a: { x: 0, y: 0 }, b: { x: 8, y: 0 }, offset: 2, height: 0.5, arrow: 'none' },
      // docs/adr/0186: a pattern's families, a gradient, the objects a hatch follows.
      ...HATCHES.map((h, i) => ({ ...h, id: 5015 + i, layerId: 'b', attrs: {} }) as Entity),
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

  it('reads a hatch’s pattern, gradient and tie back as they went (docs/adr/0186)', () => {
    const list = HATCHES.map((h, i) => ({ ...h, id: i + 1, layerId: 'a', attrs: {} }));
    const back = unpackEntities(packEntities(list));
    expect(back.map(({ geometry }) => geometry)).toEqual(HATCHES);
  });

  it('reads a text’s face and a dimension’s look back as they went (docs/adr/0183)', () => {
    const list = [
      { id: 1, layerId: 'a', attrs: {}, kind: 'text', p: { x: 2, y: 3 }, text: 'Ada 105', height: 2, rotation: 0, textStyle: 's', font: 'arimo', bold: true, italic: true, oblique: -15 },
      { id: 2, layerId: 'a', attrs: {}, kind: 'dimension', a: { x: 0, y: 0 }, b: { x: 8, y: 0 }, offset: 2, height: 0.5, dimStyle: 'd', arrow: 'dot', arrowSize: 1.2, textPlace: 'centre', decimals: 3, unit: 'mm', prefix: 'R', suffix: '″', font: 'quicksand' },
    ];
    const back = unpackEntities(packEntities(list));
    expect(back.map(({ geometry }) => geometry)).toEqual(list.map(({ id: _id, layerId: _layer, attrs: _attrs, ...g }) => g));
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


describe('a block insert packed (docs/adr/0144)', () => {
  /** Kapı: a 1 m line from its base point, with an attribute NO (default “?”) 0.25 m high at (0.5, 0.2). */
  const KAPI = JSON.stringify([
    {
      id: 'k',
      name: 'Kapı',
      base: { x: 0, y: 0 },
      entities: [{ kind: 'line', id: 1, layerId: '', attrs: {}, a: { x: 0, y: 0 }, b: { x: 1, y: 0 } }],
      attributes: [{ tag: 'NO', value: '?', p: { x: 0.5, y: 0.2 }, height: 0.25, rotation: 0 }],
    },
  ]);
  const insert = (id: number, y: number, attrs: Record<string, string>): Entity => ({ id, layerId: 'a', attrs, kind: 'insert', block: 'k', p: { x: 100, y }, scale: 2, rotation: 0 });

  it('gives the store its attributes: the value an attribute text shows is what it is picked by (§7)', () => {
    const list = [insert(1, 200, { NO: 'K-1234567', Malzeme: 'Beton' }), insert(2, 300, {}), insert(3, 400, { NO: '' })];
    const packed = new CoreStore();
    packed.setBlocks(KAPI);
    const p = packEntities(list);
    packed.putPacked(p.nums, p.strings);
    const json = new CoreStore();
    json.setBlocks(KAPI);
    json.put(JSON.stringify(list));
    for (const store of [packed, json]) {
      // 1.8 m into “K-1234567” (0.5 m high at 101, 200.4): the insert; beside the others' “?”, nothing.
      expect([store.hit(102.8, 200.6, 0.1), store.hit(102.8, 300.6, 0.1), store.hit(102.8, 400.6, 0.1)]).toEqual([1, undefined, undefined]);
      store.dispose();
    }
  });

  it('is read back as its geometry: the attributes are the object’s, not its shape’s', () => {
    const list = [insert(1, 200, { NO: 'K7', KOT: '' }), { id: 2, layerId: 'a', attrs: {}, kind: 'point' as const, p: { x: 1, y: 2 } }];
    const { nums } = packEntities(list);
    // id, layer, label, kind 14, p, scale, rotation, mirror, block (string 1); then 2 tag and value pairs of strings.
    expect(Array.from(nums.subarray(0, 15))).toEqual([1, 0, 0, 14, 100, 200, 2, 0, 0, 1, 2, 2, 3, 4, 5]);
    const back = unpackEntities(packEntities(list));
    expect(back.map((r) => r.geometry)).toEqual([
      { kind: 'insert', block: 'k', p: { x: 100, y: 200 }, scale: 2, rotation: 0 },
      { kind: 'point', p: { x: 1, y: 2 } },
    ]);
  });
});

/** A road of two strips (the second with an arc) and three survey marks (docs/adr/0174). */
const road = (id: number): PolylineEntity => ({
  id,
  layerId: 'a',
  attrs: {},
  kind: 'polyline',
  pts: [{ x: 0, y: 0 }, { x: 10, y: 0 }, { x: 10, y: 5 }],
  zs: [1, null, 3],
  parts: [{ pts: [{ x: 30, y: 0 }, { x: 36, y: 0 }], bulges: [0.25, 0] }],
});
const marks = (id: number): Entity => ({
  id,
  layerId: 'b',
  attrs: {},
  label: 'P1',
  kind: 'point',
  p: { x: 0, y: 0 },
  z: 100,
  parts: [{ p: { x: 6, y: -0 } }, { p: { x: 0, y: 9 }, z: 102.5 }],
});

describe('a multi-part polyline and a multi-point object packed (docs/adr/0174)', () => {
  it('give the store the very objects their JSON gives', () => {
    const list = [road(1), marks(2)];
    const packed = new CoreStore();
    const p = packEntities(list);
    packed.putPacked(p.nums, p.strings);
    const json = new CoreStore();
    json.put(JSON.stringify(list));
    expect(packed.itemJson(1)).toBe(json.itemJson(1));
    // JSON has no −0: the packed point keeps it.
    expect(packed.itemJson(2)).toBe(json.itemJson(2)?.replace('"x":6,"y":0', '"x":6,"y":-0'));
    expect(packed.itemJson(1)).toContain('"parts":[{"pts"');
    packed.dispose();
    json.dispose();
  });

  it('are kinds 16 and 17: the first part as one object of the kind, the count of the others, each of them', () => {
    const { nums } = packEntities([road(1), marks(2)]);
    // The road: id, layer, label, kind 16; its 3 points, no arcs, no holes; 1 more part: 2 points, 2 arcs, no holes.
    expect(Array.from(nums.subarray(0, 4))).toEqual([1, 0, 0, 16]);
    const after = 4 + 1 + 6 + 1 + 1;
    expect(Array.from(nums.subarray(after, after + 1 + 1 + 4 + 1 + 2 + 1))).toEqual([1, 2, 30, 0, 36, 0, 2, 0.25, 0, -1]);
    // The marks: id, layer, label, kind 17; x, y, has z, z; 2 more points, each x, y, has z, z.
    const at = after + 10;
    expect(Array.from(nums.subarray(at, at + 4))).toEqual([2, 1, 1, 17]);
    expect(Array.from(nums.subarray(at + 4, at + 9))).toEqual([0, 0, 1, 100, 2]);
    expect(Array.from(nums.subarray(at + 9, at + 17))).toEqual([6, -0, 0, NaN, 0, 9, 1, 102.5]);
  });

  it('are read back as they were, bit for bit, and come back from the store moved', () => {
    const back = unpackEntities(packEntities([road(1), marks(2)]));
    expect(back[0].geometry).toEqual({ kind: 'polyline', pts: road(1).pts, parts: road(1).parts });
    expect(back[1].geometry).toEqual({ kind: 'point', p: { x: 0, y: 0 }, z: 100, parts: [{ p: { x: 6, y: -0 } }, { p: { x: 0, y: 9 }, z: 102.5 }] });
    expect(Object.is((back[1].geometry.parts as { p: { y: number } }[])[0].p.y, -0)).toBe(true);
    const store = new CoreStore();
    const p = packEntities([road(1), marks(2)]);
    store.putPacked(p.nums, p.strings);
    const moved = unpackEntities(store.transformPacked(Float64Array.of(1, 2), Float64Array.of(1, 0, 0, 1, 100, 200)));
    expect((moved[0].geometry.parts as { pts: { x: number; y: number }[] }[])[0].pts[0]).toEqual({ x: 130, y: 200 });
    expect(moved[1].geometry.parts).toEqual([{ p: { x: 106, y: 200 } }, { p: { x: 100, y: 209 }, z: 102.5 }]);
    store.dispose();
  });
});
