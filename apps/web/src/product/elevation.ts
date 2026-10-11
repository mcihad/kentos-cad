import type { Entity, NewEntity, RingGeometry } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { carryElevations, type Elevated } from '../model/ops/elevation';

/**
 * Elevations of what `cad.entities.edit` writes (docs/adr/0142). The core's
 * geometry comes without them, so each vertex of a changed or new object
 * takes one from the objects the edit names, by the shared core's rules
 * (`model/ops/elevation`). A route's measures (M) are carried by the same
 * rules (docs/adr/0224 §2). The desktop's is
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
 * line's two ends, a polyline then each other part (docs/adr/0174), a
 * polygon's outer ring then its holes, then each other part's ring and holes
 * (docs/adr/0143); nothing for the other kinds. The desktop's is `crates/native/application/src/elevation.rs`
 * `paths`: the order is the one every elevation index goes by.
 */
export function elevatedPaths(e: Entity | NewEntity): Elevated[] {
  switch (e.kind) {
    case 'line':
      return [{ pts: [e.a, e.b], closed: false, zs: [e.za ?? null, e.zb ?? null] }];
    case 'polyline':
      return [path(e, false), ...(e.parts ?? []).map((part) => path(part, false))];
    case 'polygon':
      return [path(e, true), ...(e.holes ?? []).map((h) => path(h, true)), ...(e.parts ?? []).flatMap((part) => [path(part, true), ...(part.holes ?? []).map((h) => path(h, true))])];
    default:
      return [];
  }
}

/**
 * `g`, a geometry, without any vertex elevation or measure it carries: an area's `zs`, its holes', and each part's
 * and its holes' (docs/adr/0142, 0143), a route's `ms` (docs/adr/0224); the rest as it is. What the core computes has none, so what a tool writes of it
 * leaves each vertex to take its own from the objects the edit names (`carryInto`). Elevations that ride along on
 * an object the core gave back are the object's old ones, and no longer fit its vertices once they moved, were
 * dropped or turned round.
 */
export function withoutElevations<T extends object>(g: T): T {
  const out = { ...g } as Record<string, unknown>;
  delete out.zs;
  delete out.ms;
  // The holes and parts of an area are rings of their own (a hatch's holes are lists of points: nothing to take out).
  for (const key of ['holes', 'parts'])
    if (Array.isArray(out[key])) out[key] = (out[key] as unknown[]).map((ring) => (ring !== null && typeof ring === 'object' && !Array.isArray(ring) ? withoutElevations(ring) : ring));
  return out as T;
}

/**
 * A route's paths with their measures (M) in the elevations' place (docs/adr/0224 §2): a line's two ends, a polyline
 * then each other part; nothing for the other kinds (an area has none). The core's elevation rules carry measures as
 * they carry elevations. The desktop's is `crates/native/application/src/elevation.rs` `measure_paths`.
 */
export function measuredPaths(e: Entity | NewEntity): Elevated[] {
  const measured = (p: { pts: Vec2[]; bulges?: number[]; ms?: (number | null)[] }): Elevated => ({
    pts: p.pts,
    ...(p.bulges ? { bulges: p.bulges } : {}),
    closed: false,
    zs: p.ms ?? p.pts.map(() => null),
  });
  switch (e.kind) {
    case 'line':
      return [{ pts: [e.a, e.b], closed: false, zs: [e.ma ?? null, e.mb ?? null] }];
    case 'polyline':
      return [measured(e), ...(e.parts ?? []).map(measured)];
    default:
      return [];
  }
}

/**
 * `e` with the measures its vertices take from `sources` and from `same`, as `carryInto` does for elevations
 * (docs/adr/0224 §2); a field without any is left out. Whether any vertex got one: an area or another kind gets none.
 * The desktop's is `crates/native/application/src/elevation.rs` `carry_measures`.
 */
export function carryMeasuresInto(e: NewEntity, sources: readonly Elevated[], same: readonly Elevated[], offset: boolean): boolean {
  const run = (pts: Vec2[], k: number): (number | null)[] | undefined => {
    const ms = carryElevations(pts, false, same[k] ?? null, [...sources], offset);
    return ms.some((m) => m !== null) ? ms : undefined;
  };
  const out = e as unknown as Record<string, unknown>;
  switch (e.kind) {
    case 'line': {
      delete out.ms;
      const ms = run([e.a, e.b], 0);
      if (ms?.[0] != null) out.ma = ms[0];
      else delete out.ma;
      if (ms?.[1] != null) out.mb = ms[1];
      else delete out.mb;
      return ms !== undefined;
    }
    case 'polyline': {
      delete out.ma;
      delete out.mb;
      const ms = run(e.pts, 0);
      if (ms) out.ms = ms;
      else delete out.ms;
      let any = ms !== undefined;
      if (e.parts)
        e.parts = e.parts.map((part, i) => {
          const { ms: _m, ...rest } = part;
          const pm = run(part.pts, i + 1);
          any ||= pm !== undefined;
          return pm ? { ...rest, ms: pm } : rest;
        });
      return any;
    }
    default:
      delete out.ma;
      delete out.mb;
      delete out.ms;
      if (e.kind === 'polygon' && e.parts) e.parts = e.parts.map(({ ms: _m, ...rest }) => rest);
      return false;
  }
}

/**
 * `e`'s routes given the measures `ms`, in `measuredPaths`' order, as `assignElevations` gives elevations
 * (docs/adr/0224 §2). The desktop's is `crates/native/application/src/elevation.rs` `assign_measures`.
 */
export function assignMeasures(e: NewEntity, ms: readonly (number | null)[][]): void {
  const take = (k: number) => (ms[k]?.some((m) => m !== null) ? [...ms[k]] : undefined);
  const out = e as unknown as Record<string, unknown>;
  if (e.kind === 'line') {
    const ends = ms[0];
    if (ends?.[0] != null) out.ma = ends[0];
    else delete out.ma;
    if (ends?.[1] != null) out.mb = ends[1];
    else delete out.mb;
  } else if (e.kind === 'polyline') {
    const own = take(0);
    if (own) out.ms = own;
    else delete out.ms;
    if (e.parts)
      e.parts = e.parts.map((part, i) => {
        const { ms: _m, ...rest } = part;
        const pm = take(i + 1);
        return pm ? { ...rest, ms: pm } : rest;
      });
  }
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
      // Each other part of an area or a polyline (docs/adr/0143, 0174).
      if (e.parts)
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
      // Each other part of an area or a polyline (docs/adr/0143, 0174).
      if (e.parts)
        e.parts = e.parts.map((part) => {
          const { zs: _z, holes: _h, bulges, ...rest } = part;
          const pz = run(part.pts, e.kind === 'polygon', k++);
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
