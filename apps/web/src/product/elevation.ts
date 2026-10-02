import type { Entity, NewEntity, RingGeometry } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { carryElevations, type Elevated } from '../model/ops/elevation';

/**
 * Elevations of what `cad.entities.edit` writes (docs/adr/0142). The core's
 * geometry comes without them, so each vertex of a changed or new object
 * takes one from the objects the edit names, by the shared core's rules
 * (`model/ops/elevation`). The desktop's is
 * `crates/native/application/src/elevation.rs`; both pass the shared cases.
 */

type Paths = { pts: Vec2[]; bulges?: number[]; zs?: (number | null)[] };

const path = (p: Paths, closed: boolean): Elevated => ({
  pts: p.pts,
  ...(p.bulges ? { bulges: p.bulges } : {}),
  closed,
  zs: p.zs ?? p.pts.map(() => null),
});

/**
 * An object's paths with their elevations, null for a vertex without one: a
 * line's two ends, a polyline, a polygon's outer ring then its holes, then
 * each other part's ring and holes (docs/adr/0143); nothing for the other
 * kinds. The desktop's is `crates/native/application/src/elevation.rs`
 * `paths`: the order is the one every elevation index goes by.
 */
export function elevatedPaths(e: Entity | NewEntity): Elevated[] {
  switch (e.kind) {
    case 'line':
      return [{ pts: [e.a, e.b], closed: false, zs: [e.za ?? null, e.zb ?? null] }];
    case 'polyline':
      return [path(e, false)];
    case 'polygon':
      return [path(e, true), ...(e.holes ?? []).map((h) => path(h, true)), ...(e.parts ?? []).flatMap((part) => [path(part, true), ...(part.holes ?? []).map((h) => path(h, true))])];
    default:
      return [];
  }
}

/**
 * `g`, a geometry, without any vertex elevation it carries: an area's `zs`, its holes', and each part's and its
 * holes' (docs/adr/0142, 0143); the rest as it is. What the core computes has none, so what a tool writes of it
 * leaves each vertex to take its own from the objects the edit names (`carryInto`). Elevations that ride along on
 * an object the core gave back are the object's old ones, and no longer fit its vertices once they moved, were
 * dropped or turned round.
 */
export function withoutElevations<T extends object>(g: T): T {
  const out = { ...g } as Record<string, unknown>;
  delete out.zs;
  // The holes and parts of an area are rings of their own (a hatch's holes are lists of points: nothing to take out).
  for (const key of ['holes', 'parts'])
    if (Array.isArray(out[key])) out[key] = (out[key] as unknown[]).map((ring) => (ring !== null && typeof ring === 'object' && !Array.isArray(ring) ? withoutElevations(ring) : ring));
  return out as T;
}

/** Whether a vertex of the paths has an elevation. */
export const hasElevation = (paths: readonly Elevated[]): boolean => paths.some((p) => p.zs.some((z) => z !== null));

/**
 * `e` with its paths given the elevations `zs`, in `elevatedPaths`' order (a line's two ends); a path where no
 * vertex has one keeps none. What Oturt writes: the core warps the paths with their elevations (docs/adr/0156 §4).
 * The desktop's is `crates/native/application/src/elevation.rs` `assign`.
 */
export function assignElevations(e: NewEntity, zs: readonly (number | null)[][]): void {
  const take = (k: number) => (zs[k]?.some((z) => z !== null) ? [...zs[k]] : undefined);
  const out = e as unknown as Record<string, unknown>;
  switch (e.kind) {
    case 'line': {
      const ends = zs[0];
      if (ends?.[0] != null) out.za = ends[0];
      else delete out.za;
      if (ends?.[1] != null) out.zb = ends[1];
      else delete out.zb;
      return;
    }
    case 'polyline':
    case 'polygon': {
      const outer = take(0);
      if (outer) out.zs = outer;
      else delete out.zs;
      let k = 1;
      const ring = (h: RingGeometry) => {
        const { zs: _old, ...rest } = h;
        const hz = take(k++);
        return hz ? { ...rest, zs: hz } : rest;
      };
      if (e.kind === 'polygon' && e.holes) e.holes = e.holes.map(ring);
      if (e.kind === 'polygon' && e.parts)
        e.parts = e.parts.map((part) => {
          const { zs: _z, holes: _h, bulges, ...rest } = part;
          const pz = take(k++);
          return { ...rest, ...(bulges && { bulges }), ...(pz && { zs: pz }), ...(part.holes && { holes: part.holes.map(ring) }) };
        });
      return;
    }
  }
}

/**
 * `e` with the elevations its vertices take from `sources` and from `same`,
 * the paths of the object it replaces (in `elevatedPaths`' order); a field
 * without any is left out. Whether any vertex got one: an object of a kind
 * without elevations (an arc, a circle …) gets none.
 */
export function carryInto(e: NewEntity, sources: readonly Elevated[], same: readonly Elevated[], offset: boolean): boolean {
  const run = (pts: Vec2[], closed: boolean, k: number): (number | null)[] | undefined => {
    const zs = carryElevations(pts, closed, same[k] ?? null, [...sources], offset);
    return zs.some((z) => z !== null) ? zs : undefined;
  };
  // An update keeps the object's other fields: the elevations of a kind it no longer is go (a line turned
  // into an arc keeps no `za`); the kept ones are written in their places, so an edit that changes
  // nothing stays no edit.
  const out = e as unknown as Record<string, unknown>;
  switch (e.kind) {
    case 'line': {
      delete out.zs;
      const zs = run([e.a, e.b], false, 0);
      if (zs?.[0] != null) out.za = zs[0];
      else delete out.za;
      if (zs?.[1] != null) out.zb = zs[1];
      else delete out.zb;
      return zs !== undefined;
    }
    case 'polyline':
    case 'polygon': {
      delete out.za;
      delete out.zb;
      const zs = run(e.pts, e.kind === 'polygon', 0);
      if (zs) out.zs = zs;
      else delete out.zs;
      let any = zs !== undefined;
      // The rings after the outer one, in `elevatedPaths`' order: the holes, then each part's ring and its holes.
      let k = 1;
      const holes = (list: RingGeometry[]) =>
        list.map((h) => {
          const { zs: _old, ...ring } = h;
          const hz = run(h.pts, true, k++);
          any ||= hz !== undefined;
          return hz ? { ...ring, zs: hz } : ring;
        });
      if (e.kind === 'polygon' && e.holes) e.holes = holes(e.holes);
      if (e.kind === 'polygon' && e.parts)
        e.parts = e.parts.map((part) => {
          const { zs: _z, holes: _h, bulges, ...rest } = part;
          const pz = run(part.pts, true, k++);
          any ||= pz !== undefined;
          // As the typed columns lay a part out: its ring, its arcs, its elevations, its holes.
          return { ...rest, ...(bulges && { bulges }), ...(pz && { zs: pz }), ...(part.holes && { holes: holes(part.holes) }) };
        });
      return any;
    }
    default:
      delete out.za;
      delete out.zb;
      delete out.zs;
      return false;
  }
}
