import type { ArrayLayout } from '../../contracts/generated/ArrayLayout';
import type { Transform } from '../../contracts/generated/Transform';
import type { Entity, EntityKind, NewEntity } from '../entities';
import type { Affine } from '../geom/affine';
import { arrayObjects as coreArrayObjects, transformObjects as coreTransformObjects } from '../../wasm/core';
import { packEntities, unpackEntities, type Geometry, type Packed } from '../../wasm/pack';
import { entityOp } from './entityOp';

/**
 * Applies a similarity transform (move, rotate, uniform scale, mirror) to
 * any entity. Returns a copy with the same id; callers decide whether to
 * update the original or add the copy. Computed by the geometry core
 * (docs/adr/0008).
 */
export const transformEntity = entityOp<<E extends Entity>(e: E, m: Affine) => E>('transformEntity');

export const translateEntity = entityOp<<E extends Entity>(e: E, dx: number, dy: number) => E>('translateEntity');

/**
 * `transformEntity` of every entity by every affine, affine after affine,
 * in one call, through JSON. The tools ask a geometry store instead, which
 * holds the objects already (`transformedFrom`); this JSON call stays as
 * the reference that path is held to (src/wasm/transform.wasm.test.ts).
 */
export const transformEntities = entityOp<<E extends Entity>(list: readonly E[], ms: readonly Affine[]) => E[]>('transformEntities');

/** A kind's geometry fields, as the core's JSON names them (geometry-core entity.rs `Shape`). */
export const SHAPE_FIELDS: Record<EntityKind, readonly string[]> = {
  point: ['p', 'z'],
  line: ['a', 'b'],
  polyline: ['pts', 'bulges', 'holes'],
  polygon: ['pts', 'bulges', 'holes'],
  circle: ['c', 'r'],
  arc: ['c', 'r', 'a0', 'a1'],
  ellipse: ['c', 'major', 'ratio', 't0', 't1'],
  xline: ['p', 'dir'],
  ray: ['p', 'dir'],
  spline: ['pts', 'closed'],
  text: ['p', 'text', 'height', 'rotation'],
  dimension: ['a', 'b', 'offset', 'height', 'text', 'style', 'angle', 'c'],
  hatch: ['ring', 'holes', 'pattern'],
};

/**
 * Every number in an object's geometry fields is finite: its points, radii,
 * angles, bulges, heights, offsets, a hatch pattern's angle and spacing.
 * `cad.entities.transform` refuses a transform that carries one past the
 * largest float64 (docs/adr/0037); the desktop checks the same numbers.
 */
export function geometryIsFinite(e: Entity): boolean {
  const ok = (v: unknown): boolean => (typeof v === 'number' ? Number.isFinite(v) : Array.isArray(v) ? v.every(ok) : v !== null && typeof v === 'object' ? Object.values(v).every(ok) : true);
  const src = e as unknown as Record<string, unknown>;
  return (SHAPE_FIELDS[e.kind] ?? []).every((key) => ok(src[key]));
}

/**
 * `e` with another geometry, laid out as the JSON call gives it back: every
 * other field of its own first, then the kind and its fields. The other
 * fields keep their values; the attributes, the one object among them,
 * are copied, so a copy shares nothing with its original (as after JSON).
 * A cleared `bulges` or `holes` is written as undefined, as `entityOp`
 * does, so `CadDocument.update` drops the old one.
 */
function withGeometry<E extends Entity | NewEntity>(e: E, g: Geometry): E {
  const src = e as unknown as Record<string, unknown>;
  const own = SHAPE_FIELDS[src.kind as EntityKind] ?? [];
  const out: Record<string, unknown> = {};
  for (const key in src) {
    if (key === 'kind' || own.includes(key)) continue;
    const v = src[key];
    out[key] = key === 'attrs' && v && typeof v === 'object' ? { ...v } : v;
  }
  for (const key in g) out[key] = g[key];
  if ((g.kind === 'polyline' || g.kind === 'polygon') && !('bulges' in g)) out.bulges = undefined;
  if ((g.kind === 'polygon' || g.kind === 'hatch') && !('holes' in g)) out.holes = undefined;
  return out as unknown as E;
}

/**
 * `transformEntities(list, ms)` read from a geometry store's packed answer
 * (`CoreStore.transformPacked` over `ids`, the list's objects as that store
 * numbers them, and `count` affines; null: as many as the answer holds, an
 * array's copies): the store transforms its own copies and only the new
 * geometry crosses, as numbers (docs/adr/0008). Each object keeps its other
 * fields; the result is the JSON call's, field for field, except that −0
 * survives here as everywhere the store is packed.
 */
export function transformedFrom<E extends Entity | NewEntity>(list: readonly E[], ids: ArrayLike<number>, count: number | null, packed: Packed): E[] {
  const records = unpackEntities(packed);
  const out: E[] = [];
  let at = 0;
  const runs = count ?? records.length / list.length;
  for (let k = 0; k < runs; k++)
    for (let i = 0; i < list.length; i++) {
      const r = records[at++];
      // The store skips an id it does not hold: the drawing and its copy would have drifted apart.
      if (!r || r.id !== ids[i]) throw new Error('Geometri deposu seçili nesnelerin hepsini bulamadı; işlem uygulanmadı. Sayfayı yenileyip yeniden deneyin.');
      out.push(withGeometry(list[i], r.geometry));
    }
  if (at !== records.length) throw new Error('Geometri deposu beklenenden çok nesne döndürdü; işlem uygulanmadı.');
  return out;
}

/** A transform of `cad.entities.transform` as the core's `similarity` takes it: its kind and numbers. */
function similarityOf(t: Transform): [string, number[]] {
  switch (t.kind) {
    case 'move':
      return ['move', [t.dx, t.dy]];
    case 'rotate':
      return ['rotate', [t.center.x, t.center.y, t.angle]];
    case 'scale':
      return ['scale', [t.center.x, t.center.y, t.factor]];
    case 'mirror':
      return ['mirror', [t.a.x, t.a.y, t.b.x, t.b.y]];
    case 'align': {
      const first = [t.source.x, t.source.y, t.target.x, t.target.y];
      if (!t.source2 || !t.target2) return ['align', first];
      return [t.scale === true ? 'alignScale' : 'align', [...first, t.source2.x, t.source2.y, t.target2.x, t.target2.y]];
    }
  }
}

/**
 * `list` moved by one transform of the product command
 * `cad.entities.transform` (docs/adr/0037), with no store and no JSON: the
 * objects are packed (../../wasm/pack.ts), the core builds the matrix from
 * the transform's numbers and moves every object (`transform_packed_objects`),
 * and only the new geometry comes back. Each object keeps its other fields,
 * as `transformedFrom` keeps them; −0 and every other float64 survive, which
 * the JSON call (`transformEntities`) cannot promise.
 */
export function transformObjects<E extends Entity>(list: readonly E[], t: Transform): E[] {
  if (!list.length) return [];
  const packed = packEntities(list);
  const [kind, params] = similarityOf(t);
  const moved = coreTransformObjects(packed.nums, packed.strings, kind, Float64Array.from(params));
  return transformedFrom(
    list,
    list.map((e) => e.id),
    1,
    moved,
  );
}

/** An array of `cad.entities.array` as the core's `array_transforms` takes it: its kind and numbers. */
export function arrayNumbers(layout: ArrayLayout): [string, number[]] {
  if (layout.kind === 'grid') return ['grid', [layout.rows, layout.cols, layout.dx, layout.dy]];
  return ['polar', [layout.center.x, layout.center.y, layout.count, layout.fill, layout.rotate ? 1 : 0]];
}

/**
 * Copies of `list` laid out by an array of the product command
 * `cad.entities.array` (docs/adr/0047), with no store and no JSON: the core
 * lays the copies out (`array_transforms`; a polar array that does not turn
 * places them by the middle of the list's box, text measured in `font`, a
 * `DrawingFont` id) and moves every object, packed. The copies come place
 * after place, each place in the list's order, each with its original's
 * other fields.
 */
export function arrayCopies<E extends Entity>(list: readonly E[], layout: ArrayLayout, font: string): E[] {
  if (!list.length) return [];
  const packed = packEntities(list);
  const [kind, params] = arrayNumbers(layout);
  const copies = coreArrayObjects(packed.nums, packed.strings, kind, Float64Array.from(params), font);
  // As many runs of the list as the core laid places out (rows × cols − 1, or count − 1).
  return transformedFrom(
    list,
    list.map((e) => e.id),
    null,
    copies,
  );
}
