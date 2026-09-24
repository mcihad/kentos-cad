import { describe, expect, it } from 'vitest';
import type { Entity, EntityKind, NewEntity } from '../model/entities';
import { compose, mirror, rotation, scaling, translation, type Affine } from '../model/geom/affine';
import { transformEntities, transformedFrom } from '../model/ops/transform';
import { PickIndex } from '../viewport/picking';
import { Gen } from './calls/harness';
import { entity } from './calls/sets/p5-entities';
import { sceneDocument } from './calls/sets/store-scene';
import { CoreStore } from './core';
import { packEntities, unpackEntities } from './pack';

/**
 * Move, copy and paste without JSON (docs/adr/0008): the geometry store
 * transforms its own copies of the objects and hands the new geometry back
 * packed (`CoreStore.transformPacked`, read by `unpackEntities` and
 * `transformedFrom`). On random objects of every kind, near the origin and
 * in a TM zone, under random affines (translation, rotation, uniform and
 * non-uniform scale, reflection, composed ones), the result must be the
 * JSON call's (`transformEntities`), bit for bit and field for field. JSON
 * cannot carry −0, so those inputs are left out there and checked apart:
 * the packed path keeps −0 and NaN the way the packer does.
 */

const env = (globalThis as unknown as { process?: { env: Record<string, string | undefined> } }).process?.env ?? {};
const ROUNDS = Number(env.TRANSFORM_ROUNDS ?? 200);
const KINDS: EntityKind[] = ['point', 'line', 'polyline', 'polygon', 'circle', 'arc', 'ellipse', 'xline', 'ray', 'spline', 'text', 'dimension', 'hatch'];

/** The first difference: keys in order, numbers bit for bit (NaN equals NaN, −0 is not 0). */
function difference(a: unknown, b: unknown, path = ''): string | null {
  if (typeof a === 'number' && typeof b === 'number') return Object.is(a, b) ? null : `${path}: ${a} ≠ ${b}`;
  if (Array.isArray(a) || Array.isArray(b)) {
    if (!Array.isArray(a) || !Array.isArray(b) || a.length !== b.length) return `${path}: ${JSON.stringify(a)} ≠ ${JSON.stringify(b)}`;
    for (let i = 0; i < a.length; i++) {
      const d = difference(a[i], b[i], `${path}[${i}]`);
      if (d) return d;
    }
    return null;
  }
  if (a && b && typeof a === 'object' && typeof b === 'object') {
    const ka = Object.keys(a);
    const kb = Object.keys(b);
    if (ka.join() !== kb.join()) return `${path}: alanlar ${ka.join()} ≠ ${kb.join()}`;
    for (const k of ka) {
      const d = difference((a as Record<string, unknown>)[k], (b as Record<string, unknown>)[k], `${path}.${k}`);
      if (d) return d;
    }
    return null;
  }
  return Object.is(a, b) ? null : `${path}: ${String(a)} ≠ ${String(b)}`;
}

/** −0 as 0 everywhere: what JSON.stringify makes of it. */
function withoutNegativeZero<T>(v: T): T {
  if (typeof v === 'number') return (Object.is(v, -0) ? 0 : v) as T;
  if (Array.isArray(v)) return v.map(withoutNegativeZero) as T;
  if (v && typeof v === 'object') return Object.fromEntries(Object.entries(v).map(([k, x]) => [k, withoutNegativeZero(x)])) as T;
  return v;
}

/** Objects of a scene: every kind, each in its own frame (the origin or a TM zone), ids 1…n. */
function objects(g: Gen, n: number): Entity[] {
  return Array.from({ length: n }, (_, i) => {
    g.frame();
    return { ...entity(g, g.pick(KINDS)), id: i + 1 };
  });
}

/** Objects the random ones rarely or never are: fields left out and given, holes on a polyline, empty lists. */
const SPECIAL: NewEntity[] = [
  { layerId: 'a', attrs: {}, kind: 'dimension', a: { x: 486520, y: 4420200 }, b: { x: 486530, y: 4420207 }, offset: -2, height: 0.5, style: 'linear' },
  { layerId: 'a', attrs: { Not: 'x' }, kind: 'dimension', a: { x: 0, y: 0 }, b: { x: 3, y: 4 }, offset: 2, height: 0.5, text: '', style: 'radius', c: { x: 1, y: 1 } },
  { layerId: 'a', attrs: {}, kind: 'polyline', pts: [{ x: 0, y: 0 }, { x: 4, y: 0 }], holes: [{ pts: [{ x: 1, y: 1 }], bulges: [0.5] }] },
  { layerId: 'a', attrs: {}, kind: 'polygon', pts: [{ x: 0, y: 0 }, { x: 4, y: 0 }, { x: 4, y: 4 }], bulges: [], holes: [] },
  { layerId: 'a', attrs: {}, kind: 'polyline', pts: [] },
  { layerId: 'a', attrs: {}, kind: 'spline', pts: [], closed: true },
  { layerId: 'b', attrs: {}, kind: 'hatch', ring: [], holes: [[]], pattern: { type: 'solid', angle: 0, spacing: 1 } },
  { layerId: 'b', attrs: {}, label: '', kind: 'text', p: { x: 486520, y: 4420200 }, text: 'Çiçek 😀', height: 2, rotation: 359.5 },
  { layerId: 'b', attrs: {}, kind: 'ellipse', c: { x: 486520, y: 4420200 }, major: { x: -10, y: 3 }, ratio: 0.4, t0: 2, t1: 2 },
];

/** A random affine: translation, rotation, uniform or non-uniform scale, reflection, or two of them composed. */
function affine(g: Gen, depth = 0): Affine {
  const o = g.pt();
  const pick = g.int(0, depth ? 4 : 5);
  if (pick === 0) return translation(g.num(-100, 100), g.num(-100, 100));
  if (pick === 1) return rotation(g.chance(0.2) ? g.pick([0, Math.PI / 2, Math.PI, -Math.PI / 2]) : g.num(-7, 7), o);
  if (pick === 2) return scaling(g.num(0.1, 4), o);
  if (pick === 3) {
    // Non-uniform: no tool makes one, but the transform must not differ either.
    const sx = g.num(0.2, 3);
    const sy = g.num(0.2, 3);
    return [sx, 0, 0, sy, o.x * (1 - sx), o.y * (1 - sy)];
  }
  if (pick === 4) return mirror(o, g.pt());
  return compose(affine(g, 1), affine(g, 1));
}

/** The packed path on a store of its own holding `list` (by the list's ids, or 1…n for objects without one). */
function packedPath<E extends Entity | NewEntity>(list: readonly E[], ms: readonly Affine[]): E[] {
  const ids = Float64Array.from(list, (e, i) => ('id' in e ? e.id : i + 1));
  const store = new CoreStore();
  const p = packEntities(list.map((e, i) => ({ ...e, id: ids[i] })));
  store.putPacked(p.nums, p.strings);
  try {
    return transformedFrom(list, ids, ms.length, store.transformPacked(ids, Float64Array.from(ms.flat())));
  } finally {
    store.dispose();
  }
}

/** The first result that differs from the JSON call's, and whether each kept its own fields. */
function compare(list: readonly Entity[], ms: readonly Affine[], got: readonly Entity[]): string | null {
  const want = transformEntities(list, ms);
  if (got.length !== want.length) return `${got.length} sonuç ≠ ${want.length}`;
  for (let i = 0; i < got.length; i++) {
    const d = difference(got[i], want[i]);
    if (d) return `${i}. sonuç (${want[i].kind}): ${d}`;
    const src = list[i % list.length] as unknown as Record<string, unknown>;
    const out = got[i] as unknown as Record<string, unknown>;
    for (const k of ['id', 'layerId', 'color', 'label', 'symbol']) if (!Object.is(out[k], src[k])) return `${i}. sonuç: ${k} değişti`;
    if (out.attrs === src.attrs || JSON.stringify(out.attrs) !== JSON.stringify(src.attrs)) return `${i}. sonuç: öznitelikler kendi kopyası değil`;
  }
  return null;
}

describe('move, copy and paste through the geometry store, packed', () => {
  it('reads back what was packed, every kind, bit for bit', () => {
    const g = new Gen(3_2026);
    const list: Entity[] = [...objects(g, 2000), ...SPECIAL.map((e, i) => ({ ...e, id: 5001 + i }) as Entity)];
    list.push({ id: 6001, layerId: 'a', attrs: {}, kind: 'point', p: { x: -0, y: NaN }, z: -0 }, { id: 6002, layerId: 'a', attrs: {}, kind: 'line', a: { x: Infinity, y: -Infinity }, b: { x: 486512.34, y: 4420187.52 } });
    const back = unpackEntities(packEntities(list));
    expect(back.length).toBe(list.length);
    // The store's own copy written as JSON, fields in the core's order: the geometry must be the same.
    const store = new CoreStore();
    const p = packEntities(list);
    store.putPacked(p.nums, p.strings);
    const failures: string[] = [];
    back.forEach((r, i) => {
      const e = list[i];
      if (r.id !== e.id || r.layerId !== e.layerId || r.labelled !== !!e.label) failures.push(`${i}: kimlik, katman ya da etiket`);
      const { id: _id, layerId: _l, label: _t, ...geometry } = JSON.parse(store.itemJson(e.id)!, (_k, v: unknown) => (v === '#NaN' ? NaN : v === '#Inf' ? Infinity : v === '#-Inf' ? -Infinity : v)) as Record<string, unknown>;
      const d = difference(r.geometry, geometry);
      if (d) failures.push(`${i} (${e.kind}): ${d}`);
    });
    store.dispose();
    expect(failures.slice(0, 5).join('\n')).toBe('');
    const point = back.at(-2)!.geometry;
    expect([Object.is((point.p as { x: number }).x, -0), Number.isNaN((point.p as { y: number }).y), Object.is(point.z, -0)]).toEqual([true, true, true]);
    expect(() => unpackEntities({ nums: Float64Array.of(1, 0, 0, 13), strings: '["a"]' })).toThrow(/bilinmeyen bir nesne türü/);
  });

  it('gives what the JSON call gives, bit for bit, on random objects and affines', () => {
    const g = new Gen(20_260_924);
    const failures: string[] = [];
    for (let round = 0; round < ROUNDS && failures.length < 5; round++) {
      const list = withoutNegativeZero([...objects(g, g.int(1, 40)), ...(round % 10 === 0 ? SPECIAL.map((e, i) => ({ ...e, id: 1001 + i }) as Entity) : [])]);
      const ms = withoutNegativeZero(Array.from({ length: g.int(1, 3) }, () => affine(g)));
      const d = compare(list, ms, packedPath(list, ms));
      if (d) failures.push(`${round}. tur: ${d}`);
    }
    expect(failures.join('\n')).toBe('');
  }, 600_000);

  it('gives the same through the view’s store, as the tools ask it', () => {
    const g = new Gen(8_1);
    const doc = sceneDocument(g, 300);
    const index = new PickIndex(doc);
    const failures: string[] = [];
    for (let round = 0; round < 40 && failures.length < 5; round++) {
      const all = [...doc.all()];
      const list = withoutNegativeZero(all.filter(() => g.chance(0.2)));
      const ms = withoutNegativeZero(Array.from({ length: g.int(1, 4) }, () => affine(g)));
      // The tools hand in the drawing's own objects (after an edit the store sends them again first).
      const mine = all.filter((e) => list.some((x) => x.id === e.id));
      const d = compare(list, ms, index.transformEntities(mine, ms));
      if (d) failures.push(`${round}. tur: ${d}`);
      if (g.chance(0.5) && all.length) {
        const e = g.pick(all);
        doc.update(e.id, transformEntities([e], [translation(g.num(-5, 5), g.num(-5, 5))])[0]);
      }
    }
    index.dispose();
    expect(failures.join('\n')).toBe('');
  });

  it('pastes objects that are not in the drawing, numbered 1…n', () => {
    const g = new Gen(44);
    const list = withoutNegativeZero(objects(g, 300));
    const items = list.map(({ id: _id, ...rest }) => rest as NewEntity);
    const m = [translation(12.5, -7.25)];
    const got = packedPath(items, m);
    const want = transformEntities(
      items.map((e) => ({ ...e, id: 0 }) as Entity),
      m,
    ).map(({ id: _id, ...rest }) => rest);
    expect(got.map((e, i) => difference(e, want[i])).find((d) => d) ?? null).toBe(null);
  });

  it('keeps −0 and NaN the way the packer does; a missing object fails loudly', () => {
    type Point = Extract<Entity, { kind: 'point' }>;
    type Circle = Extract<Entity, { kind: 'circle' }>;
    const e: Point = { id: 1, layerId: 'a', attrs: {}, kind: 'point', p: { x: 486512.34, y: 4420187.52 }, z: -0 };
    const c: Circle = { id: 2, layerId: 'a', attrs: {}, kind: 'circle', c: { x: 486520, y: NaN }, r: NaN };
    const m = [translation(10, 0)];
    const [moved, circle] = packedPath<Entity>([e, c], m) as [Point, Circle];
    expect([moved.p.x, moved.p.y, Object.is(moved.z, -0)]).toEqual([486512.34 + 10, 4420187.52, true]);
    expect([circle.c.x, circle.c.y, circle.r]).toEqual([NaN, NaN, NaN]);
    // The JSON call reads the −0 as 0; NaN it keeps as well.
    const [jp, jc] = transformEntities<Entity>([e, c], m) as [Point, Circle];
    expect([Object.is(jp.z, 0), jc.r]).toEqual([true, NaN]);

    const store = new CoreStore();
    const pk = packEntities([e]);
    store.putPacked(pk.nums, pk.strings);
    const ids = Float64Array.of(1, 2);
    expect(() => transformedFrom([e, { ...e, id: 2 }], ids, 1, store.transformPacked(ids, Float64Array.from(translation(1, 1))))).toThrow(/Geometri deposu/);
    store.dispose();
  });
});
