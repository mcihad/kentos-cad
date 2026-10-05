import type { EntityGeometry as EditGeometry } from '../contracts/generated/EntityGeometry';
import type { Entity, RingGeometry } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { length3d } from '../model/ops/elevation';
import { gripPart, holeGrip } from '../model/ops/grips';
import { geometryOf } from './entitiesEdit';
import { elevatedPaths } from './elevation';

/**
 * The vertex elevations of an object as the interface reads and writes them (docs/adr/0142): Kot ver, Öznitelikler,
 * the grip tag, Koordinat oku and the hover card all go through here, so the web says and does one thing. A vertex
 * without an elevation is `null` (not 0); a point's is its `z`. The desktop's are in `kentos-domain`.
 *
 * The 3D length is the core's (`length3d`); nothing is computed here but sums of what it answers and the
 * arithmetic of Kot ver's Artır (a plain addition: the value written is the sum, not a rounded one, CLAUDE.md §23.2).
 */

/** The kinds whose vertices take elevations: a point (its `z`), a line, a polyline and an area. */
export const takesElevation = (e: { kind: Entity['kind'] }): boolean => e.kind === 'point' || e.kind === 'line' || e.kind === 'polyline' || e.kind === 'polygon';

/**
 * Every vertex's elevation, in one order: a point's `z`, then each other point's (docs/adr/0174); a line's two ends;
 * a polyline's vertices, then each other part's; a polygon's outer ring, then each hole's, then each other part's
 * ring and its holes (docs/adr/0143). Empty for a kind without elevations.
 */
export function vertexElevations(e: Entity): (number | null)[] {
  if (e.kind === 'point') return [e.z ?? null, ...(e.parts ?? []).map((q) => q.z ?? null)];
  return elevatedPaths(e).flatMap((p) => p.zs);
}

const hasOne = (z: number | null) => z !== null;
const anyOne = (zs: readonly (number | null)[] | undefined) => !!zs?.some(hasOne);
const ringHasOne = (ring: { zs?: (number | null)[] }) => anyOne(ring.zs);
const partHasOne = (part: { zs?: (number | null)[]; holes?: { zs?: (number | null)[] }[] }) => anyOne(part.zs) || !!part.holes?.some(ringHasOne);

/** Whether any vertex has an elevation (the cheap check before the rest: nothing is made for it, however many parts an area has). */
export function hasVertexElevation(e: Entity): boolean {
  switch (e.kind) {
    // Every point and every part too (docs/adr/0174).
    case 'point':
      return e.z !== undefined || !!e.parts?.some((q) => q.z !== undefined);
    case 'line':
      return e.za !== undefined || e.zb !== undefined;
    case 'polyline':
      return anyOne(e.zs) || !!e.parts?.some(ringHasOne);
    case 'polygon':
      return anyOne(e.zs) || !!e.holes?.some(ringHasOne) || !!e.parts?.some(partHasOne);
    default:
      return false;
  }
}

/**
 * The vertex of a point, line, polyline or area (holes included) nearest to `p`, when it is within `within` (m,
 * inclusive): its place, its elevation (null for one without) and its distance. Where the grips of those kinds
 * stand: the select tool's tag looks for the one the pointer rests on, on every move, so this makes no
 * allocation per vertex and turns most away by their box alone. Null for other kinds or when none is that near.
 */
export function nearestVertex(e: Entity, p: Vec2, within: number): { at: Vec2; z: number | null; d: number } | null {
  let best: { at: Vec2; z: number | null; d: number } | null = null;
  let reach = within;
  const test = (v: Vec2, z: number | null | undefined) => {
    const dx = v.x - p.x;
    if (dx > reach || dx < -reach) return;
    const dy = v.y - p.y;
    if (dy > reach || dy < -reach) return;
    const d = Math.hypot(dx, dy);
    if (d <= reach) {
      reach = d;
      best = { at: v, z: z ?? null, d };
    }
  };
  switch (e.kind) {
    case 'point':
      test(e.p, e.z);
      // A multi-point object's other points (docs/adr/0174).
      for (const q of e.parts ?? []) test(q.p, q.z);
      break;
    case 'line':
      test(e.a, e.za);
      test(e.b, e.zb);
      break;
    case 'polyline':
    case 'polygon': {
      const ring = (pts: readonly Vec2[], zs: readonly (number | null)[] | undefined) => {
        for (let i = 0; i < pts.length; i++) test(pts[i], zs?.[i]);
      };
      ring(e.pts, e.zs);
      // A multi-part polyline's other parts (docs/adr/0174).
      if (e.kind === 'polyline') for (const part of e.parts ?? []) ring(part.pts, part.zs);
      if (e.kind === 'polygon') {
        for (const h of e.holes ?? []) ring(h.pts, h.zs);
        // The other parts of a multi-part area (docs/adr/0143): each ring, its holes.
        for (const part of e.parts ?? []) {
          ring(part.pts, part.zs);
          for (const h of part.holes ?? []) ring(h.pts, h.zs);
        }
      }
      break;
    }
  }
  return best;
}

/**
 * The one elevation every vertex of the object has: that number; null when none has one (also a kind that takes
 * none); `'mixed'` when they differ or some vertices lack one. Cheap on a large selection: an object without
 * elevations answers at once, one with them stops at the first vertex that differs, and nothing is allocated.
 */
export function uniformElevation(e: Entity): number | null | 'mixed' {
  // A multi-point object's points all alike (docs/adr/0174).
  if (e.kind === 'point') return (e.parts ?? []).every((q) => (q.z ?? null) === (e.z ?? null)) ? (e.z ?? null) : 'mixed';
  if (e.kind === 'line') return e.za === undefined && e.zb === undefined ? null : e.za !== undefined && e.za === e.zb ? e.za : 'mixed';
  if ((e.kind !== 'polyline' && e.kind !== 'polygon') || !hasVertexElevation(e)) return null;
  const first = e.zs?.[0];
  if (first == null) return 'mixed';
  const same = (zs: readonly (number | null)[] | undefined, n: number) => {
    for (let i = 0; i < n; i++) if (zs?.[i] !== first) return false;
    return true;
  };
  if (!same(e.zs, e.pts.length)) return 'mixed';
  if (e.kind === 'polyline') for (const part of e.parts ?? []) if (!same(part.zs, part.pts.length)) return 'mixed';
  if (e.kind === 'polygon') {
    for (const h of e.holes ?? []) if (!same(h.zs, h.pts.length)) return 'mixed';
    for (const part of e.parts ?? []) {
      if (!same(part.zs, part.pts.length)) return 'mixed';
      for (const h of part.holes ?? []) if (!same(h.zs, h.pts.length)) return 'mixed';
    }
  }
  return first;
}

/** What a list of elevations comes to, as the Kot rows say it (`kot yok`, a value, a range). */
export type ElevationSummary =
  /** No vertex has one. */
  | { kind: 'none' }
  /** Every vertex has this one. */
  | { kind: 'value'; z: number }
  /** Every vertex has one and they differ: the lowest and the highest. */
  | { kind: 'range'; min: number; max: number }
  /** Some vertices have none: the lowest and the highest of those that have one (equal when they agree). */
  | { kind: 'partial'; min: number; max: number };

export function summarizeElevations(zs: readonly (number | null)[]): ElevationSummary {
  // A loop, not `Math.min(...zs)`: a path of a few hundred thousand vertices would overflow the call's arguments.
  let min = Infinity;
  let max = -Infinity;
  let have = 0;
  for (const z of zs) {
    if (z === null) continue;
    have++;
    if (z < min) min = z;
    if (z > max) max = z;
  }
  if (!have) return { kind: 'none' };
  if (have < zs.length) return { kind: 'partial', min, max };
  return min === max ? { kind: 'value', z: min } : { kind: 'range', min, max };
}

/**
 * The length in space beside the plan one, when every vertex has an elevation: `3B uzunluk` of a line or a
 * polyline, `3B çevre` of an area (its outer ring and every hole, all closed, as the plan perimeter counts them, of
 * every part, docs/adr/0143). Null for anything else or when a vertex has none.
 */
export function spaceLength(e: Entity): { label: '3B uzunluk' | '3B çevre'; value: number } | null {
  if (e.kind !== 'line' && e.kind !== 'polyline' && e.kind !== 'polygon') return null;
  if (!hasVertexElevation(e)) return null;
  const paths = elevatedPaths(e);
  if (paths.some((p) => p.zs.some((z) => z === null))) return null;
  let sum = 0;
  for (const p of paths) {
    const len = length3d(p.pts, p.bulges ?? null, p.closed, p.zs);
    if (len === null) return null;
    sum += len;
  }
  return { label: e.kind === 'polygon' ? '3B çevre' : '3B uzunluk', value: sum };
}

/**
 * The elevation of the vertex a grip stands on, or null: a mid grip is no vertex, and a vertex may have none.
 * `index` counts as the core lists the grips (`entityGrips`): a path's vertices, then one mid grip per edge, then
 * the vertices of each hole; a multi-part area's and polyline's part after part (docs/adr/0143, 0174); a multi-point
 * object's points.
 */
export function gripElevation(e: Entity, index: number): number | null {
  switch (e.kind) {
    case 'point':
      return index === 0 ? (e.z ?? null) : (e.parts?.[index - 1]?.z ?? null);
    case 'line':
      return index === 0 ? (e.za ?? null) : index === 1 ? (e.zb ?? null) : null;
    case 'polyline': {
      // The grip's own part and its place there; the first part is the polyline's own fields.
      const at = e.parts?.length ? gripPart(e, index) : { part: 0, index };
      if (!at) return null;
      const own = at.part === 0 ? e : e.parts?.[at.part - 1];
      return own && at.index < own.pts.length ? (own.zs?.[at.index] ?? null) : null;
    }
    case 'polygon': {
      // The grip's own part and its place there; the first part is the area's own fields.
      const at = e.parts?.length ? gripPart(e, index) : { part: 0, index };
      if (!at) return null;
      const own = at.part === 0 ? e : e.parts?.[at.part - 1];
      if (!own) return null;
      if (at.index < own.pts.length) return own.zs?.[at.index] ?? null;
      // `holeGrip` counts within the grip's own part.
      const hole = holeGrip(e, index);
      return hole ? (own.holes?.[hole.hole]?.zs?.[hole.vertex] ?? null) : null;
    }
    default:
      return null;
  }
}

/** How near a vertex must be to a point to be the vertex there (m; the core's rule for carrying elevations). */
const ON = 1e-6;

/** The elevation of the object's vertex at `p`, or null: no vertex there, or it has none. A line's end counts. */
export function elevationAt(e: Entity, p: Vec2): number | null {
  if (e.kind === 'point') {
    // Any point of a multi-point object (docs/adr/0174).
    const there = (q: Vec2) => Math.abs(q.x - p.x) <= ON && Math.abs(q.y - p.y) <= ON;
    if (e.z !== undefined && there(e.p)) return e.z;
    return e.parts?.find((q) => q.z !== undefined && there(q.p))?.z ?? null;
  }
  for (const path of elevatedPaths(e))
    for (const [i, v] of path.pts.entries()) {
      const z = path.zs[i];
      if (z !== null && Math.abs(v.x - p.x) <= ON && Math.abs(v.y - p.y) <= ON) return z;
    }
  return null;
}

/**
 * The object's own geometry with each vertex's elevation `f` gives, as `cad.entities.edit` takes it (the operation
 * `elevation`): `zs` for a line (its two ends), a polyline and an area (each hole's in its own `zs`, each other
 * part's and its holes' in theirs, docs/adr/0143), `z` for a point (none when `f` says null). `f` gets the vertex's
 * elevation and its place in `vertexElevations`' order. Null for a kind without elevations.
 */
export function mapElevations(e: Entity, f: (z: number | null, index: number) => number | null): EditGeometry | null {
  if (!takesElevation(e)) return null;
  const g = geometryOf(e as unknown as EditGeometry) as Record<string, unknown>;
  if (e.kind === 'point') {
    const z = f(e.z ?? null, 0);
    if (z === null) delete g.z;
    else g.z = z;
    // Every point of a multi-point object, in turn (docs/adr/0174).
    if (e.parts)
      g.parts = e.parts.map((q, k) => {
        const qz = f(q.z ?? null, k + 1);
        return qz === null ? { p: q.p } : { p: q.p, z: qz };
      });
    return g as unknown as EditGeometry;
  }
  let next = 0;
  const run = (zs: readonly (number | null)[]) => zs.map((z) => f(z, next++));
  if (e.kind === 'line') g.zs = run([e.za ?? null, e.zb ?? null]);
  else if (e.kind === 'polyline' || e.kind === 'polygon') {
    const nulls = (n: number) => Array.from({ length: n }, () => null);
    g.zs = run(e.zs ?? nulls(e.pts.length));
    const holes = (list: readonly RingGeometry[]) => list.map((h) => ({ ...structuredClone(h), zs: run(h.zs ?? nulls(h.pts.length)) }));
    if (e.kind === 'polygon' && e.holes) g.holes = holes(e.holes);
    // A multi-part polyline's other parts (docs/adr/0174).
    if (e.kind === 'polyline' && e.parts) g.parts = e.parts.map((part) => ({ ...structuredClone(part), zs: run(part.zs ?? nulls(part.pts.length)) }));
    // Each other part's ring, then its holes: `vertexElevations`' order.
    if (e.kind === 'polygon' && e.parts)
      g.parts = e.parts.map((part) => {
        const zs = run(part.zs ?? nulls(part.pts.length));
        return { ...structuredClone(part), zs, ...(part.holes && { holes: holes(part.holes) }) };
      });
  }
  return g as unknown as EditGeometry;
}
