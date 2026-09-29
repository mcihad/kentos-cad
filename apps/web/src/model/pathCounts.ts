import type { Entity, PolylineEntity } from './entities';

/**
 * What a path or an area has, counted over every part of a multi-part area (docs/adr/0143): a polygon's own fields
 * are its first part and `parts` are the rest. Plain reading of the entity, no geometry: the panels and the export
 * windows say these numbers, the core's lengths and areas are already part-aware.
 */
export interface PathCounts {
  /** Vertices of every part's outer ring; the holes' vertices are not counted. */
  corners: number;
  /** Parts, the area's own included: 1 for a one-part area and for a polyline. */
  parts: number;
  /** Islands (holes) of every part. */
  holes: number;
}

export function pathCounts(e: Pick<PolylineEntity, 'pts' | 'holes' | 'parts'>): PathCounts {
  let corners = e.pts.length;
  let holes = e.holes?.length ?? 0;
  for (const part of e.parts ?? []) {
    corners += part.pts.length;
    holes += part.holes?.length ?? 0;
  }
  return { corners, parts: (e.parts?.length ?? 0) + 1, holes };
}

const ARC = 1e-12;
const arced = (bulges: readonly number[] | undefined) => (bulges ?? []).some((b) => Math.abs(b) > ARC);

/** Whether a polyline or an area has an arc edge: in its ring, in a hole, or in any part of a multi-part area. */
export function hasArcs(e: Entity): boolean {
  if (e.kind !== 'polyline' && e.kind !== 'polygon') return false;
  if (arced(e.bulges) || (e.holes ?? []).some((r) => arced(r.bulges))) return true;
  return (e.parts ?? []).some((part) => arced(part.bulges) || (part.holes ?? []).some((r) => arced(r.bulges)));
}

/** Whether an area has an island (a hole): in its own ring or in any of its parts. */
export function hasIslands(e: Entity): boolean {
  return e.kind === 'polygon' && pathCounts(e).holes > 0;
}
