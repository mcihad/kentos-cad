import type { ArrayLayout } from '../../contracts/generated/ArrayLayout';
import type { Transform } from '../../contracts/generated/Transform';
import type { Entity, EntityKind, NewEntity } from '../entities';
import type { Affine } from '../geom/affine';
import { arrayObjects as coreArrayObjects, op, transformObjects as coreTransformObjects } from '../../wasm/core';
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
  // An area's parts past its first are its own (docs/adr/0143); the elevations (`zs`) are the object's, not the core's.
  polygon: ['pts', 'bulges', 'holes', 'parts'],
  circle: ['c', 'r'],
  arc: ['c', 'r', 'a0', 'a1'],
  ellipse: ['c', 'major', 'ratio', 't0', 't1'],
  xline: ['p', 'dir'],
  ray: ['p', 'dir'],
  spline: ['pts', 'closed'],
  text: ['p', 'text', 'height', 'rotation'],
  dimension: ['a', 'b', 'offset', 'height', 'text', 'style', 'angle', 'c'],
  hatch: ['ring', 'holes', 'pattern'],
  // A block's placement (docs/adr/0144); `mirror` only when true.
  insert: ['block', 'p', 'scale', 'rotation', 'mirror'],
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
 * A cleared `bulges`, `holes` or `parts` is written as undefined, as
 * `entityOp` does, so `CadDocument.update` drops the old one.
 */
export function withGeometry<E extends Entity | NewEntity>(e: E, g: Geometry): E {
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
  if (g.kind === 'polygon' && !('parts' in g)) out.parts = undefined;
  // A transform moves each vertex and keeps it: the elevations stay with their vertices, the
  // holes' too, and each part's and its holes' (docs/adr/0143); a vertex count that changed
  // leaves them out (docs/adr/0142).
  if (g.kind === 'polygon') {
    const holes = src.holes as Ring[] | undefined;
    if (Array.isArray(out.holes) && holes?.some((h) => h.zs)) out.holes = withElevations(out.holes as Ring[], holes);
    const was = src.parts as ElevatedPart[] | undefined;
    if (Array.isArray(out.parts) && was) out.parts = (out.parts as ElevatedPart[]).map((part, k) => withPartElevations(part, was[k]));
  }
  if (Array.isArray(out.zs) && (!Array.isArray(out.pts) || out.zs.length !== out.pts.length)) out.zs = undefined;
  return out as unknown as E;
}

/** A ring of an area (a hole) and a part of it, with the elevations their vertices may carry. */
type Ring = { pts: unknown[]; zs?: (number | null)[] };
type ElevatedPart = Ring & { holes?: Ring[] };

/** `now`, the rings a transform gave back, with the elevations of `was` (the same ring before it) where the vertex count is the same. */
function withElevations(now: Ring[], was: readonly Ring[]): Ring[] {
  return now.map((ring, i) => (was[i]?.zs && was[i].pts.length === ring.pts.length ? { ...ring, zs: was[i].zs } : ring));
}

/** A part a transform gave back with the elevations of the part it was: its own and its holes'. */
function withPartElevations(part: ElevatedPart, was: ElevatedPart | undefined): ElevatedPart {
  if (!was) return part;
  const out = { ...part };
  if (was.zs && was.pts.length === part.pts.length) out.zs = was.zs;
  if (Array.isArray(out.holes) && was.holes?.some((h) => h.zs)) out.holes = withElevations(out.holes, was.holes);
  return out;
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

/**
 * An array of `cad.entities.array` as the core's `array_transforms` takes it: its kind and numbers. A path
 * array's maps come from its path in the document (`pathArrayTransforms`): none here.
 */
export function arrayNumbers(layout: ArrayLayout): [string, number[]] {
  if (layout.kind === 'grid') return ['grid', [layout.rows, layout.cols, layout.dx, layout.dy]];
  if (layout.kind === 'path') return ['affines', []];
  return ['polar', [layout.center.x, layout.center.y, layout.count, layout.fill, layout.rotate ? 1 : 0]];
}

/**
 * Yol boyunca dizi (docs/adr/0140): the copies' maps along `path` (a line, an arc, a circle or a polyline),
 * `count` places from its start, `spacing` metres apart or spread over it, turned with it when `align`.
 * Null for another kind of path, one of no length, or places past its end (computed by the geometry core).
 */
export const pathArrayTransforms = op<(path: Entity, count: number, spacing: number | null, align: boolean) => Affine[] | null>('pathArrayTransforms');

/**
 * Copies of `list` laid out by an array of the product command
 * `cad.entities.array` (docs/adr/0047), with no store and no JSON: the core
 * lays the copies out (`array_transforms`; a polar array that does not turn
 * places them by the middle of the list's box, text measured in `font`, a
 * `DrawingFont` id) and moves every object, packed. The copies come place
 * after place, each place in the list's order, each with its original's
 * other fields.
 */
export function arrayCopies<E extends Entity>(list: readonly E[], layout: ArrayLayout, font: string, maps?: readonly Affine[]): E[] {
  if (!list.length) return [];
  const packed = packEntities(list);
  const [kind, numbers] = arrayNumbers(layout);
  // A path array's maps, made from its path, go to the core as six numbers each.
  const params = maps ? maps.flat() : numbers;
  const copies = coreArrayObjects(packed.nums, packed.strings, kind, Float64Array.from(params), font);
  // As many runs of the list as the core laid places out (rows × cols − 1, or count − 1).
  return transformedFrom(
    list,
    list.map((e) => e.id),
    null,
    copies,
  );
}
