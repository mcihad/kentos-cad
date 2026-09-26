import { entityArea, entityLength, entityVertices, type Entity } from '../../model/entities';
import type { Vec2 } from '../../model/geometry';

/**
 * What the coordinate list shows of one object. Its vertices come in rings
 * (a polygon's outer ring, then each hole, as the core lists them), so an
 * edge never joins two rings. Its area and length are the object's own, as
 * the core measures them and the properties panel shows them: arcs of bulged
 * edges followed, holes taken out of the area and counted in the perimeter.
 * They are never measured from the vertex list, which strings the rings
 * together and cuts across arcs.
 */
export interface VertexListing {
  pts: Vec2[];
  /** The vertex after `i` along its ring, or null where the ring ends open. */
  next: (i: number) => number | null;
  area: number | null;
  length: number | null;
}

export function vertexListing(e: Entity): VertexListing {
  const pts = entityVertices(e);
  const closed = e.kind === 'polygon';
  const sizes = e.kind === 'polygon' ? [e.pts.length, ...(e.holes ?? []).map((h) => h.pts.length)] : [pts.length];
  const rings: [number, number][] = [];
  let at = 0;
  for (const n of sizes) {
    rings.push([at, at + n]);
    at += n;
  }
  const next = (i: number): number | null => {
    const ring = rings.find(([s, end]) => i >= s && i < end);
    if (!ring) return null;
    if (i + 1 < ring[1]) return i + 1;
    return closed ? ring[0] : null;
  };
  return { pts, next, area: entityArea(e), length: entityLength(e) };
}
