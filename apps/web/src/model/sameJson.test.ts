import { describe, expect, it } from 'vitest';
import { CadDocument } from './document';
import type { Entity, NewEntity } from './entities';
import { LayerStore } from './layers';
import { sameJson } from './sameJson';

/**
 * sameJson must answer exactly what comparing JSON.stringify strings did
 * (the document's geometry test used them): these tests use the strings as
 * the reference, named cases first, then random data, then every entity
 * kind and field through CadDocument.update.
 */

const byStrings = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b);

/** The document's test before sameJson: attributes aside, the objects' JSON. */
function stringsSayChanged(a: Entity, b: Entity): boolean {
  if (a.layerId !== b.layerId || a.color !== b.color || a.label !== b.label) return true;
  const { attrs: _a, ...ga } = a;
  const { attrs: _b, ...gb } = b;
  return JSON.stringify(ga) !== JSON.stringify(gb);
}

describe('sameJson', () => {
  it('agrees with JSON.stringify on numbers, nulls, holes, key order and left-out values', () => {
    const fn = () => 0;
    // Index 0 is a hole.
    const sparse: unknown[] = [];
    sparse[1] = 1;
    const cases: [unknown, unknown, boolean][] = [
      [0, -0, true],
      [NaN, NaN, true],
      [NaN, Infinity, true],
      [-Infinity, null, true],
      [NaN, 0, false],
      [1, 1 + Number.EPSILON, false],
      [1e21, 1e21, true],
      ['1', 1, false],
      [true, 'true', false],
      [null, undefined, false],
      [undefined, undefined, true],
      [[undefined], [null], true],
      [[NaN, -0], [null, 0], true],
      [[fn], [null], true],
      [sparse, [null, 1], true],
      [[1, 2], [2, 1], false],
      [[1], [1, undefined], false],
      [{ a: undefined }, {}, true],
      [{ a: null }, {}, false],
      [{ a: 1, f: fn }, { a: 1 }, true],
      [{ a: 1, b: 2 }, { b: 2, a: 1 }, false],
      [{ 2: 'x', 1: 'y' }, { 1: 'y', 2: 'x' }, true],
      [{ x: {} }, { x: [] }, false],
      [{}, [], false],
      [{ p: [{ x: 0, y: -0 }] }, { p: [{ x: -0, y: 0 }] }, true],
      [{ p: { x: 1, y: 2 } }, { p: { y: 2, x: 1 } }, false],
      [{ p: { x: NaN, y: 2 } }, { p: { x: null, y: 2 } }, true],
      [{ t: 'ş' }, { t: 'ş' }, true],
      [{ t: 'ş' }, { t: 's' }, false],
    ];
    for (const [a, b, same] of cases) {
      expect(byStrings(a, b), JSON.stringify([a, b])).toBe(same);
      expect(sameJson(a, b), `${String(a)} / ${String(b)}: ${JSON.stringify([a, b])}`).toBe(same);
      expect(sameJson(b, a)).toBe(same);
    }
  });

  it('leaves one outer key out, wherever it stands', () => {
    expect(sameJson({ attrs: { N: '1' }, x: 1 }, { attrs: { N: '2' }, x: 1 }, 'attrs')).toBe(true);
    expect(sameJson({ attrs: {}, x: 1 }, { x: 1 }, 'attrs')).toBe(true);
    expect(sameJson({ x: 1, attrs: {} }, { attrs: {}, x: 1 }, 'attrs')).toBe(true);
    expect(sameJson({ x: 1, attrs: {} }, { attrs: {}, x: 2 }, 'attrs')).toBe(false);
    // Only the outer object: a nested "attrs" still counts.
    expect(sameJson({ p: { attrs: 1 } }, { p: { attrs: 2 } }, 'attrs')).toBe(false);
  });

  it('agrees with JSON.stringify on random data and near copies of it', () => {
    let seed = 424242;
    const rnd = (n: number) => (seed = (seed * 16807) % 2147483647) % n;
    const NUMBERS = [0, -0, 1, -1, 0.1, 0.30000000000000004, 486512.34, 4420100.000000001, NaN, Infinity, -Infinity, 1e-300, 5e-324];
    const KEYS = ['x', 'y', 'pts', 'bulges', 'holes', 'z', '1', '10', 'attrs'];
    const value = (depth: number): unknown => {
      switch (rnd(depth > 2 ? 5 : 8)) {
        case 0:
          return NUMBERS[rnd(NUMBERS.length)];
        case 1:
          return rnd(2) ? 'a' : 'ş';
        case 2:
          return [null, undefined, true, false][rnd(4)];
        case 3:
        case 4:
          return NUMBERS[rnd(NUMBERS.length)] + rnd(3);
        case 5:
        case 6:
          return Array.from({ length: rnd(4) }, () => value(depth + 1));
        default: {
          const o: Record<string, unknown> = {};
          for (let k = rnd(4); k > 0; k--) o[KEYS[rnd(KEYS.length)]] = value(depth + 1);
          return o;
        }
      }
    };
    /** A copy with at most one small change: a number, its sign, a null for NaN, key order, a left-out key. */
    const nearCopy = (v: unknown): unknown => {
      if (typeof v === 'number') return [v, -v, v === 0 ? -0 : v, Number.isFinite(v) ? v : null, v + 1][rnd(5)];
      if (Array.isArray(v)) {
        const out = v.map((x) => (rnd(3) ? structuredClone(x) : nearCopy(x)));
        if (!rnd(6)) out.push(undefined);
        return out;
      }
      if (v && typeof v === 'object') {
        const entries = Object.entries(v).map(([k, x]): [string, unknown] => [k, rnd(3) ? structuredClone(x) : nearCopy(x)]);
        if (!rnd(5)) entries.reverse();
        if (!rnd(5)) entries.push(['yeni', rnd(2) ? undefined : 0]);
        return Object.fromEntries(entries);
      }
      return rnd(4) ? v : value(3);
    };
    let equal = 0;
    for (let i = 0; i < 4000; i++) {
      const a = value(0);
      const b = rnd(4) ? nearCopy(a) : value(0);
      const same = byStrings(a, b);
      if (same) equal++;
      expect(sameJson(a, b), JSON.stringify([a, b])).toBe(same);
      expect(sameJson(b, a)).toBe(same);
    }
    // Both answers came up often.
    expect(equal).toBeGreaterThan(400);
    expect(equal).toBeLessThan(3600);
  });
});

// ── The document's geometry test, every kind and field ─────────────────────

const V = (x: number, y: number) => ({ x, y });
const ring = (x: number, y: number) => [V(x, y), V(x + 10, y), V(x + 10, y + 10), V(x, y + 10)];

/** One object of each kind with every optional field set. */
const OBJECTS: NewEntity[] = [
  { kind: 'point', layerId: 'a', p: V(486512.34, 4420100.5), z: 12.5, color: '#FF0000', label: '101', symbol: 'temel.nokta', attrs: { Ad: '101' } },
  { kind: 'line', layerId: 'a', a: V(0, 0), b: V(-0, 5), attrs: {} },
  { kind: 'polyline', layerId: 'a', pts: ring(0, 0), bulges: [0, 0.5, 0, -0], attrs: { N: '1' } },
  { kind: 'polygon', layerId: 'a', pts: ring(0, 0), bulges: [0, 0, 0.25, 0], holes: [{ pts: ring(2, 2), bulges: [0, 0, 0, 1] }, { pts: ring(6, 6) }], label: '7', attrs: { Parsel: '7' } },
  { kind: 'circle', layerId: 'a', c: V(3, 4), r: 2, attrs: {} },
  { kind: 'arc', layerId: 'a', c: V(3, 4), r: 2, a0: 0, a1: Math.PI / 2, attrs: {} },
  { kind: 'ellipse', layerId: 'a', c: V(3, 4), major: V(5, 0), ratio: 0.5, t0: 0, t1: 0, attrs: {} },
  { kind: 'spline', layerId: 'a', pts: ring(0, 0), closed: false, attrs: {} },
  { kind: 'xline', layerId: 'a', p: V(1, 1), dir: V(Math.SQRT1_2, Math.SQRT1_2), attrs: {} },
  { kind: 'ray', layerId: 'a', p: V(1, 1), dir: V(1, 0), attrs: {} },
  { kind: 'text', layerId: 'a', p: V(1, 1), text: 'Yazı', height: 2.5, rotation: 0, attrs: {} },
  { kind: 'dimension', layerId: 'a', a: V(0, 0), b: V(10, 0), offset: 3, height: 2.5, text: '', style: 'angular', angle: 90, c: V(5, 5), attrs: {} },
  { kind: 'hatch', layerId: 'a', ring: ring(0, 0), holes: [ring(2, 2)], pattern: { type: 'lines', angle: 45, spacing: 1.5 }, attrs: {} },
];

/** Patches for one field: its value again as a fresh copy, and small changes a JSON comparison may or may not see. */
function patchesFor(key: string, v: unknown): unknown[] {
  const out: unknown[] = [structuredClone(v), undefined];
  if (v === undefined) out.push('x', 0, null);
  if (typeof v === 'number') out.push(v + 1, -v, v === 0 ? -0 : v, NaN);
  if (typeof v === 'string') out.push(`${v}x`);
  if (typeof v === 'boolean') out.push(!v);
  const isVec = (p: unknown): p is { x: number; y: number } => !!p && typeof p === 'object' && 'x' in p && 'y' in p;
  if (isVec(v)) out.push(V(v.x + 1e-9, v.y), { y: v.y, x: v.x }, V(v.x === 0 ? -0 : v.x, v.y), V(NaN, v.y));
  if (Array.isArray(v) && v.length) {
    const last = v.length - 1;
    const edit = (f: (x: unknown) => unknown) => v.map((x, i) => (i === last ? f(structuredClone(x)) : structuredClone(x)));
    out.push(v.slice(0, last), [...structuredClone(v), structuredClone(v[last])]);
    if (isVec(v[last])) out.push(edit((p) => ({ ...(p as object), x: (p as { x: number }).x + 1 })), edit((p) => ({ y: (p as { y: number }).y, x: (p as { x: number }).x })));
    if (typeof v[last] === 'number') out.push(edit((b) => (b === 0 ? -0 : (b as number) + 1)), edit(() => NaN));
    if (key === 'holes') out.push(edit((h) => (Array.isArray(h) ? h.slice(1) : { ...(h as object), bulges: undefined })));
  }
  if (key === 'pattern') out.push({ ...(v as object), angle: 46 }, { type: 'lines', spacing: 1.5, angle: 45 });
  return out;
}

describe('CadDocument: geometry or attributes changed', () => {
  const makeDoc = () => new CadDocument({ name: 'G', layers: new LayerStore([{ id: 'a', name: 'A' }, { id: 'b', name: 'B' }], 'a'), origin: { x: 0, y: 0 } });

  /** Applies the patch and says which event the document sent. */
  function classify(init: NewEntity, patch: Record<string, unknown>): { changed: boolean; expected: boolean } {
    const doc = makeDoc();
    const before = doc.add(init);
    let changed = false;
    let attrs = false;
    doc.events.on('changed', () => (changed = true));
    doc.events.on('attrs', () => (attrs = true));
    doc.update(before.id, patch as Partial<Entity>);
    const after = doc.get(before.id)!;
    // A patch that changes nothing is no edit and sends nothing (docs/adr/0020); any other sends exactly one of the two.
    if (sameJson(before, after)) expect(changed || attrs).toBe(false);
    else expect(changed).toBe(!attrs);
    return { changed, expected: stringsSayChanged(before, after) };
  }

  it('sends what the JSON comparison did for every kind and field', () => {
    const seen = { changed: 0, attrs: 0 };
    for (const init of OBJECTS) {
      const keys = new Set([...Object.keys(init), 'color', 'label', 'symbol', 'layerId']);
      for (const key of keys) {
        const value = (init as unknown as Record<string, unknown>)[key];
        const patches = key === 'layerId' ? ['a', 'b'] : key === 'attrs' ? [{ N: '2' }, {}] : key === 'kind' ? [init.kind] : patchesFor(key, value);
        for (const p of patches) {
          const r = classify(init, { [key]: p });
          expect(r.changed, `${init.kind}.${key} = ${JSON.stringify(p)}`).toBe(r.expected);
          seen[r.changed ? 'changed' : 'attrs']++;
        }
      }
    }
    expect(seen.changed).toBeGreaterThan(100);
    expect(seen.attrs).toBeGreaterThan(50);
  });

  it('keeps the JSON rules: attributes, fresh copies, -0, NaN, undefined and key order', () => {
    const [point, line, polyline, polygon] = OBJECTS;
    const hatch = OBJECTS[12];
    const straight: NewEntity = { kind: 'polyline', layerId: 'a', pts: ring(0, 0), attrs: {} };
    const q = (init: NewEntity, patch: Record<string, unknown>) => classify(init, patch).changed;
    expect(q(point, { attrs: { Ad: '102' } })).toBe(false);
    expect(q(point, { p: V(486512.34, 4420100.5) })).toBe(false);
    expect(q(point, { p: V(486512.34, 4420100.500000001) })).toBe(true);
    expect(q(point, { z: 12.5 })).toBe(false);
    expect(q(point, { z: undefined })).toBe(true);
    expect(q(point, { symbol: 'temel.kare' })).toBe(true);
    // As JSON writes it: -0 is 0, NaN and Infinity are null, an undefined field is no field.
    expect(q(line, { b: V(0, 5) })).toBe(false);
    const noHeight = { ...point, z: NaN } as NewEntity;
    expect(q(noHeight, { z: Infinity })).toBe(false);
    expect(q(noHeight, { z: 0 })).toBe(true);
    expect(q(straight, { bulges: undefined })).toBe(false);
    expect(q(straight, { bulges: [0, 0, 0, 0] })).toBe(true);
    expect(q(polyline, { bulges: [0, 0.5, 0, 0] })).toBe(false);
    expect(q(polyline, { bulges: undefined })).toBe(true);
    expect(q(polygon, { holes: [{ pts: ring(2, 2), bulges: [0, 0, 0, 1] }, { pts: ring(6, 6), bulges: undefined }] })).toBe(false);
    expect(q(polygon, { holes: [{ pts: ring(2, 2), bulges: [0, 0, 0, 1] }, { pts: ring(6, 7) }] })).toBe(true);
    expect(q(hatch, { holes: [ring(2, 2), ring(4, 4)] })).toBe(true);
    // Key order is part of JSON: the same points written y-first count as a change.
    expect(q(straight, { pts: ring(0, 0).map((p) => ({ y: p.y, x: p.x })) })).toBe(true);
  });
});
