import type { Entity } from '../model/entities';
import type { Bounds, Vec2 } from '../model/geometry';
import { op } from '../wasm/core';

/**
 * Geometry as the style engine sees it: every object is a point, a set of
 * lines or an area. Areas come with the outer ring counter-clockwise and
 * holes clockwise, so "left of the drawing direction" is always into the
 * area (the offset rule of edge layers). The geometry store tessellates
 * the curves and orients the rings (docs/adr/0008, S2), a whole layer in
 * one call; `DrawnReader` reads its records (the plain scene builder's
 * layers). Where symbols go on it is the style core's
 * (crates/shared/style-core/src/style/place.rs).
 */

export type GeometryClass = 'marker' | 'line' | 'fill';

export type StyledGeometry =
  | { readonly cls: 'marker'; readonly point: Vec2 }
  | { readonly cls: 'line'; readonly paths: readonly { readonly pts: readonly Vec2[]; readonly closed: boolean }[] }
  | { readonly cls: 'fill'; readonly rings: readonly (readonly Vec2[])[] };

/**
 * Areas are polygons and hatches; circles and ellipses stay curves (as in
 * CAD, a circle is not filled). Text and dimensions are drawn elsewhere.
 */
export function geometryClassOf(e: Entity): GeometryClass | null {
  switch (e.kind) {
    case 'point':
      return 'marker';
    case 'polygon':
    case 'hatch':
      return 'fill';
    case 'text':
    case 'dimension':
      return null;
    default:
      return 'line';
  }
}

/** Record kinds and point references of the store's drawn geometry (crates/shared/geometry-core/src/store/draw.rs). */
const MARKER = 1;
const LINE = 2;
const FILL = 3;
const SOURCE = -1;
const REVERSED = -2;

/** The object's own points of path or ring `k`, which a record refers to instead of copying them. */
function ownPoints(e: Entity, k: number): readonly Vec2[] {
  switch (e.kind) {
    case 'line':
      return [e.a, e.b];
    case 'polyline':
      return e.pts;
    case 'polygon':
      return k === 0 ? e.pts : (e.holes?.[k - 1]?.pts ?? []);
    case 'hatch':
      return k === 0 ? e.ring : (e.holes?.[k - 1] ?? []);
    default:
      return [];
  }
}

/**
 * Reads the geometry store's drawn-geometry records (`CoreStore.drawn`), one
 * per object in the order they were asked for. Each record must be read,
 * also the objects that draw nothing (text: null).
 */
export class DrawnReader {
  private readonly buf: Float64Array;
  private at = 0;

  constructor(buf: Float64Array) {
    this.buf = buf;
  }

  private points(e: Entity, k: number): readonly Vec2[] {
    const b = this.buf;
    const n = b[this.at++];
    if (n === SOURCE) return ownPoints(e, k);
    if (n === REVERSED) return [...ownPoints(e, k)].reverse();
    const out: Vec2[] = new Array(n);
    for (let i = 0; i < n; i++) out[i] = { x: b[this.at + 2 * i], y: b[this.at + 2 * i + 1] };
    this.at += 2 * n;
    return out;
  }

  /** The next record, for `e`: the object it was asked for. */
  read(e: Entity): StyledGeometry | null {
    const b = this.buf;
    switch (b[this.at++]) {
      case MARKER: {
        const point = { x: b[this.at], y: b[this.at + 1] };
        this.at += 2;
        return { cls: 'marker', point };
      }
      case LINE: {
        const count = b[this.at++];
        const paths: { pts: readonly Vec2[]; closed: boolean }[] = [];
        for (let k = 0; k < count; k++) {
          const closed = b[this.at++] === 1;
          paths.push({ pts: this.points(e, k), closed });
        }
        return { cls: 'line', paths };
      }
      case FILL: {
        const count = b[this.at++];
        const rings: (readonly Vec2[])[] = [];
        for (let k = 0; k < count; k++) rings.push(this.points(e, k));
        return { cls: 'fill', rings };
      }
      default:
        return null;
    }
  }
}

const drawnGeometry = op<(e: Entity, oriented: boolean, clip: Bounds | null) => number[]>('drawnGeometry');

/**
 * One object's geometry for the style engine (symbol previews and tests; a
 * layer asks the store for all its objects at once). Text and dimensions
 * have none; construction lines are drawn only clipped to `clip`.
 */
export function styledGeometry(e: Entity, clip?: Bounds): StyledGeometry | null {
  if (e.kind === 'text' || e.kind === 'dimension') return null;
  return new DrawnReader(Float64Array.from(drawnGeometry(e, true, clip ?? null))).read(e);
}
