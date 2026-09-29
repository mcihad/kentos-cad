import type { Entity, NewEntity } from '../model/entities';
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
 * line's two ends, a polyline, a polygon's outer ring then its holes; nothing
 * for the other kinds.
 */
export function elevatedPaths(e: Entity | NewEntity): Elevated[] {
  switch (e.kind) {
    case 'line':
      return [{ pts: [e.a, e.b], closed: false, zs: [e.za ?? null, e.zb ?? null] }];
    case 'polyline':
      return [path(e, false)];
    case 'polygon':
      return [path(e, true), ...(e.holes ?? []).map((h) => path(h, true))];
    default:
      return [];
  }
}

/** Whether a vertex of the paths has an elevation. */
export const hasElevation = (paths: readonly Elevated[]): boolean => paths.some((p) => p.zs.some((z) => z !== null));

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
      if (e.kind === 'polygon' && e.holes)
        e.holes = e.holes.map((h, i) => {
          const { zs: _old, ...ring } = h;
          const hz = run(h.pts, true, i + 1);
          any ||= hz !== undefined;
          return hz ? { ...ring, zs: hz } : ring;
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
