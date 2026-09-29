import type { EntityGeometry as EditGeometry } from '../contracts/generated/EntityGeometry';
import type { Entity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { length3d } from '../model/ops/elevation';
import { holeGrip } from '../model/ops/grips';
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
 * Every vertex's elevation, in one order: a point's `z`; a line's two ends; a polyline's vertices; a polygon's outer
 * ring, then each hole's. Empty for a kind without elevations.
 */
export function vertexElevations(e: Entity): (number | null)[] {
  if (e.kind === 'point') return [e.z ?? null];
  return elevatedPaths(e).flatMap((p) => p.zs);
}

/** Whether any vertex has an elevation (the cheap check before the rest: nothing is allocated). */
export function hasVertexElevation(e: Entity): boolean {
  switch (e.kind) {
    case 'point':
      return e.z !== undefined;
    case 'line':
      return e.za !== undefined || e.zb !== undefined;
    case 'polyline':
      return !!e.zs?.some((z) => z !== null);
    case 'polygon':
      return !!e.zs?.some((z) => z !== null) || !!e.holes?.some((h) => h.zs?.some((z) => z !== null));
    default:
      return false;
  }
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
  const have = zs.filter((z): z is number => z !== null);
  if (!have.length) return { kind: 'none' };
  const min = Math.min(...have);
  const max = Math.max(...have);
  if (have.length < zs.length) return { kind: 'partial', min, max };
  return min === max ? { kind: 'value', z: min } : { kind: 'range', min, max };
}

/**
 * The length in space beside the plan one, when every vertex has an elevation: `3B uzunluk` of a line or a
 * polyline, `3B çevre` of an area (its outer ring and every hole, all closed, as the plan perimeter counts them).
 * Null for anything else or when a vertex has none.
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
 * the vertices of each hole.
 */
export function gripElevation(e: Entity, index: number): number | null {
  switch (e.kind) {
    case 'point':
      return index === 0 ? (e.z ?? null) : null;
    case 'line':
      return index === 0 ? (e.za ?? null) : index === 1 ? (e.zb ?? null) : null;
    case 'polyline':
      return index < e.pts.length ? (e.zs?.[index] ?? null) : null;
    case 'polygon': {
      if (index < e.pts.length) return e.zs?.[index] ?? null;
      const hole = holeGrip(e, index);
      return hole ? (e.holes?.[hole.hole]?.zs?.[hole.vertex] ?? null) : null;
    }
    default:
      return null;
  }
}

/** How near a vertex must be to a point to be the vertex there (m; the core's rule for carrying elevations). */
const ON = 1e-6;

/** The elevation of the object's vertex at `p`, or null: no vertex there, or it has none. A line's end counts. */
export function elevationAt(e: Entity, p: Vec2): number | null {
  if (e.kind === 'point') return e.z !== undefined && Math.abs(e.p.x - p.x) <= ON && Math.abs(e.p.y - p.y) <= ON ? e.z : null;
  for (const path of elevatedPaths(e))
    for (const [i, v] of path.pts.entries()) {
      const z = path.zs[i];
      if (z !== null && Math.abs(v.x - p.x) <= ON && Math.abs(v.y - p.y) <= ON) return z;
    }
  return null;
}

/**
 * The object's own geometry with each vertex's elevation `f` gives, as `cad.entities.edit` takes it (the operation
 * `elevation`): `zs` for a line (its two ends), a polyline and an area (each hole's in its own `zs`), `z` for a
 * point (none when `f` says null). `f` gets the vertex's elevation and its place in `vertexElevations`' order. Null
 * for a kind without elevations.
 */
export function mapElevations(e: Entity, f: (z: number | null, index: number) => number | null): EditGeometry | null {
  if (!takesElevation(e)) return null;
  const g = geometryOf(e as unknown as EditGeometry) as Record<string, unknown>;
  if (e.kind === 'point') {
    const z = f(e.z ?? null, 0);
    if (z === null) delete g.z;
    else g.z = z;
    return g as unknown as EditGeometry;
  }
  let next = 0;
  const run = (zs: readonly (number | null)[]) => zs.map((z) => f(z, next++));
  if (e.kind === 'line') g.zs = run([e.za ?? null, e.zb ?? null]);
  else if (e.kind === 'polyline' || e.kind === 'polygon') {
    const nulls = (n: number) => Array.from({ length: n }, () => null);
    g.zs = run(e.zs ?? nulls(e.pts.length));
    if (e.kind === 'polygon' && e.holes) g.holes = e.holes.map((h) => ({ ...structuredClone(h), zs: run(h.zs ?? nulls(h.pts.length)) }));
  }
  return g as unknown as EditGeometry;
}
