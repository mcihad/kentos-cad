import type { Vec2 } from '../model/geometry';
import { triangulateMany } from '../wasm/core';

/**
 * The fills of one layer build, triangulated together in one call to the
 * geometry core (`triangulateMany`, docs/adr/0008 S2) instead of one ear
 * clipping per polygon. Each fill is queued with the array its triangles
 * go into; `run` appends them, three vertices per triangle as x, y
 * relative to the origin, in the order the fills were queued.
 */
export class FillQueue {
  private readonly outs: number[][] = [];
  private readonly polygons: (readonly (readonly Vec2[])[])[] = [];
  private points = 0;
  private rings = 0;

  /** A polygon: its outer ring, then its holes. */
  add(out: number[], rings: readonly (readonly Vec2[])[]): void {
    this.outs.push(out);
    this.polygons.push(rings);
    this.rings += rings.length;
    for (const r of rings) this.points += r.length;
  }

  run(origin: Vec2): void {
    const n = this.outs.length;
    if (!n) return;
    const xy = new Float64Array(2 * this.points);
    const ringSizes = new Uint32Array(this.rings);
    const polyRings = new Uint32Array(n);
    // Where each polygon's points end: triangles come polygon after polygon, each within its own points.
    const ends = new Uint32Array(n);
    let p = 0;
    let r = 0;
    for (let i = 0; i < n; i++) {
      const rings = this.polygons[i];
      polyRings[i] = rings.length;
      for (const ring of rings) {
        ringSizes[r++] = ring.length;
        for (const q of ring) {
          xy[2 * p] = q.x;
          xy[2 * p + 1] = q.y;
          p++;
        }
      }
      ends[i] = p;
    }
    const idx = triangulateMany(xy, ringSizes, polyRings);
    const ox = origin.x;
    const oy = origin.y;
    let poly = 0;
    for (let t = 0; t + 2 < idx.length; t += 3) {
      const a = 2 * idx[t];
      const b = 2 * idx[t + 1];
      const c = 2 * idx[t + 2];
      while (a >= 2 * ends[poly]) poly++;
      this.outs[poly].push(xy[a] - ox, xy[a + 1] - oy, xy[b] - ox, xy[b + 1] - oy, xy[c] - ox, xy[c + 1] - oy);
    }
    this.outs.length = 0;
    this.polygons.length = 0;
    this.points = 0;
    this.rings = 0;
  }
}
