import type { Entity } from '../model/entities';
import { centroid, type Bounds, type Vec2 } from '../model/geometry';
import type { MarkerPlacement } from '../model/style';
import { op } from '../wasm/core';

/**
 * Geometry as the style engine sees it: every object is a point, a set of
 * lines or an area. Areas come with the outer ring counter-clockwise and
 * holes clockwise, so "left of the drawing direction" is always into the
 * area (the offset rule of edge layers). The geometry store tessellates
 * the curves and orients the rings (docs/adr/0008, S2), a whole layer in
 * one call; `DrawnReader` reads its records.
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

// ── Walking along a path ───────────────────────────────────────────────

export interface Placed {
  readonly at: Vec2;
  /** Direction of the path there, radians. */
  readonly angle: number;
}

/** No path gets more markers than this (a tiny interval would hang the page). */
export const MAX_MARKERS_PER_PATH = 50_000;

const dirOf = (a: Vec2, b: Vec2) => Math.atan2(b.y - a.y, b.x - a.x);

const SHARP_TURN = Math.PI / 6;

/** Distances along the path of the corners that turn more than 30° (a closed path's start included). */
function sharpCorners(segs: readonly { start: number; angle: number }[], closed: boolean): number[] {
  const out: number[] = [];
  for (let i = closed ? 0 : 1; i < segs.length; i++) {
    const prev = segs[(i - 1 + segs.length) % segs.length];
    let turn = Math.abs(segs[i].angle - prev.angle) % (2 * Math.PI);
    if (turn > Math.PI) turn = 2 * Math.PI - turn;
    if (turn > SHARP_TURN) out.push(segs[i].start);
  }
  return out;
}

/** The bisector direction at a corner: the mean of the directions in and out. */
function meanAngle(a: number, b: number): number {
  return Math.atan2(Math.sin(a) + Math.sin(b), Math.cos(a) + Math.cos(b));
}

/** Several markers per place, `spacing` apart along the path and centred on it. */
export interface PlaceGroup {
  readonly count: number;
  readonly spacing: number;
}

/**
 * Marker positions along a path. `interval` places one every `interval`
 * from `offsetAlong` (a closed path does not repeat its start at the end);
 * vertices face the bisector of their corner. A `group` puts `count`
 * markers at each place instead of one, centred on it (dots in a dash
 * gap); on a closed path they wrap around, on an open one those past an
 * end are left out. `clear` keeps interval places that far from sharp
 * corners (turns over 30°): a code written along a boundary is left out
 * rather than bent around a corner.
 */
export function placeAlong(pts: readonly Vec2[], closed: boolean, placement: MarkerPlacement, interval = 0, offsetAlong = 0, group?: PlaceGroup, clear = 0): Placed[] {
  const n = pts.length;
  if (n < 2) return n === 1 ? [{ at: pts[0], angle: 0 }] : [];
  const segCount = closed ? n : n - 1;
  const segs: { a: Vec2; b: Vec2; len: number; start: number; angle: number }[] = [];
  let total = 0;
  for (let i = 0; i < segCount; i++) {
    const a = pts[i];
    const b = pts[(i + 1) % n];
    const len = Math.hypot(b.x - a.x, b.y - a.y);
    if (len < 1e-12) continue;
    segs.push({ a, b, len, start: total, angle: dirOf(a, b) });
    total += len;
  }
  if (!segs.length) return [];
  const at = (s: number): Placed => {
    // Binary search for the segment containing s.
    let lo = 0;
    let hi = segs.length - 1;
    while (lo < hi) {
      const mid = (lo + hi + 1) >> 1;
      if (segs[mid].start <= s) lo = mid;
      else hi = mid - 1;
    }
    const g = segs[lo];
    const t = Math.min(1, Math.max(0, (s - g.start) / g.len));
    return { at: { x: g.a.x + (g.b.x - g.a.x) * t, y: g.a.y + (g.b.y - g.a.y) * t }, angle: g.angle };
  };
  // Where along the path each place is (vertices keep their bisector unless grouped).
  const places: { s: number; placed?: Placed }[] = [];
  switch (placement) {
    case 'interval': {
      if (!(interval > 0)) return [];
      const first = ((offsetAlong % interval) + interval) % interval;
      const end = closed ? total - 1e-9 : total + 1e-9;
      for (let s = closed ? first : offsetAlong; s <= end && places.length < MAX_MARKERS_PER_PATH; s += interval) if (s >= -1e-9) places.push({ s: Math.max(0, s) });
      if (clear > 0) {
        const corners = sharpCorners(segs, closed);
        if (corners.length) {
          const gap = (s: number, c: number) => (closed ? Math.min(Math.abs(s - c), total - Math.abs(s - c)) : Math.abs(s - c));
          const kept = places.filter((p) => corners.every((c) => gap(p.s, c) >= clear));
          places.length = 0;
          places.push(...kept);
        }
      }
      break;
    }
    case 'center':
      places.push({ s: total / 2 });
      break;
    case 'segmentCenter':
      for (const g of segs) places.push({ s: g.start + g.len / 2, placed: { at: { x: (g.a.x + g.b.x) / 2, y: (g.a.y + g.b.y) / 2 }, angle: g.angle } });
      break;
    case 'first':
      places.push({ s: 0, placed: { at: segs[0].a, angle: segs[0].angle } });
      break;
    case 'last': {
      const g = segs[segs.length - 1];
      places.push({ s: total, placed: { at: g.b, angle: g.angle } });
      break;
    }
    case 'vertex':
    case 'innerVertex': {
      for (let i = 0; i < segs.length; i++) {
        const prev = segs[i - 1] ?? (closed ? segs[segs.length - 1] : null);
        if (!prev && placement === 'innerVertex') continue;
        places.push({ s: segs[i].start, placed: { at: segs[i].a, angle: prev ? meanAngle(prev.angle, segs[i].angle) : segs[i].angle } });
      }
      if (!closed && placement === 'vertex') {
        const g = segs[segs.length - 1];
        places.push({ s: total, placed: { at: g.b, angle: g.angle } });
      }
      break;
    }
  }
  const count = group ? Math.max(1, Math.floor(group.count)) : 1;
  if (count === 1 || !(group!.spacing > 0)) return places.map((p) => p.placed ?? at(p.s));
  const out: Placed[] = [];
  const half = ((count - 1) * group!.spacing) / 2;
  for (const p of places) {
    for (let i = 0; i < count && out.length < MAX_MARKERS_PER_PATH; i++) {
      let s = p.s - half + i * group!.spacing;
      if (closed) s = ((s % total) + total) % total;
      else if (s < -1e-9 || s > total + 1e-9) continue;
      out.push(at(Math.min(total, Math.max(0, s))));
    }
  }
  return out;
}

// ── Waves ──────────────────────────────────────────────────────────────

/** Points per wave: enough for a smooth sine at any zoom a symbol is drawn at. */
const WAVE_STEPS = 16;

export interface WaveSpec {
  readonly shape: 'sine' | 'zigzag' | 'square';
  /** One wave's length along the path, its height to each side, and the repeat (world units). */
  readonly length: number;
  readonly amplitude: number;
  readonly spacing: number;
  /** Straight line between waves when the repeat is longer than a wave. */
  readonly connect: boolean;
  /** Start of the first wave from the path's start; absent = waves centred on the path. */
  readonly offsetAlong?: number;
}

/** Height of a wave at t ∈ [0, 1] of its length, in amplitudes (starts and ends on the line). */
function waveAt(shape: WaveSpec['shape'], t: number): number {
  if (shape === 'sine') return Math.sin(t * 2 * Math.PI);
  if (shape === 'zigzag') return t < 0.25 ? t * 4 : t < 0.75 ? 2 - t * 4 : t * 4 - 4;
  return t <= 0 || t >= 1 ? 0 : t < 0.5 ? 1 : -1;
}

/**
 * A path drawn as waves: each wave is laid along the path (following its
 * bends) and pushed to the left by the wave height. Returns the pieces to
 * stroke: one continuous path when the waves connect, one per wave when the
 * line between them is left out. Waves that would pass an open path's end
 * are cut short there.
 */
export function wavePaths(pts: readonly Vec2[], closed: boolean, w: WaveSpec): Vec2[][] {
  if (!(w.length > 0) || pts.length < 2) return [pts.slice()];
  const spacing = Math.max(w.length, w.spacing);
  const along = walker(pts, closed);
  if (!along) return [];
  const total = along.total;
  const anchored = w.offsetAlong !== undefined;
  // Anchored: waves from `offsetAlong` on, as many as fit (a closed path wraps the start into its first repeat).
  const first = !anchored ? 0 : closed ? ((w.offsetAlong! % spacing) + spacing) % spacing : Math.max(0, w.offsetAlong!);
  const count = Math.min(MAX_MARKERS_PER_PATH, anchored ? Math.floor((total - first - w.length + 1e-9) / spacing) + 1 : Math.floor((total + 1e-9) / spacing));
  if (count < 1) return [pts.slice()];
  const out: Vec2[][] = [];
  let current: Vec2[] = [];
  const push = (s: number, h: number) => {
    const p = along.at(s);
    current.push({ x: p.at.x - Math.sin(p.angle) * h, y: p.at.y + Math.cos(p.angle) * h });
  };
  // Waves centred along the path, the rest shared at both ends (or from the anchor on).
  const start = anchored ? first : (total - count * spacing) / 2 + (spacing - w.length) / 2;
  const steps = w.shape === 'square' ? 0 : WAVE_STEPS;
  for (let k = 0; k < count; k++) {
    const s0 = start + k * spacing;
    if (w.connect) {
      if (k === 0) push(0, 0);
      // The corners of the path between waves stay corners.
      along.cornersBetween(k === 0 ? 0 : Math.max(0, s0 - (spacing - w.length)), s0).forEach((c) => current.push(c));
    } else if (current.length) {
      out.push(current);
      current = [];
    }
    if (w.shape === 'square') {
      const a = w.amplitude;
      push(s0, 0);
      push(s0, a);
      push(s0 + w.length / 2, a);
      push(s0 + w.length / 2, -a);
      push(s0 + w.length, -a);
      push(s0 + w.length, 0);
    } else for (let i = 0; i <= steps; i++) push(s0 + (w.length * i) / steps, waveAt(w.shape, i / steps) * w.amplitude);
  }
  if (w.connect) {
    along.cornersBetween(start + (count - 1) * spacing + w.length, total).forEach((c) => current.push(c));
    push(closed ? 0 : total, 0);
  }
  if (current.length > 1) out.push(current);
  return out;
}

/** Arc-length access to a path: point and direction at a distance, and the corners inside a stretch. */
function walker(pts: readonly Vec2[], closed: boolean) {
  const segs: { a: Vec2; b: Vec2; len: number; start: number; angle: number }[] = [];
  let total = 0;
  const n = pts.length;
  for (let i = 0; i < (closed ? n : n - 1); i++) {
    const a = pts[i];
    const b = pts[(i + 1) % n];
    const len = Math.hypot(b.x - a.x, b.y - a.y);
    if (len < 1e-12) continue;
    segs.push({ a, b, len, start: total, angle: dirOf(a, b) });
    total += len;
  }
  if (!segs.length) return null;
  const find = (s: number) => {
    let lo = 0;
    let hi = segs.length - 1;
    while (lo < hi) {
      const mid = (lo + hi + 1) >> 1;
      if (segs[mid].start <= s) lo = mid;
      else hi = mid - 1;
    }
    return lo;
  };
  return {
    total,
    at(s: number): Placed {
      const g = segs[find(Math.min(total, Math.max(0, s)))];
      const t = Math.min(1, Math.max(0, (s - g.start) / g.len));
      return { at: { x: g.a.x + (g.b.x - g.a.x) * t, y: g.a.y + (g.b.y - g.a.y) * t }, angle: g.angle };
    },
    cornersBetween(s0: number, s1: number): Vec2[] {
      const out: Vec2[] = [];
      for (const g of segs) if (g.start > s0 + 1e-9 && g.start < s1 - 1e-9) out.push(g.a);
      return out;
    },
  };
}

// ── Inside point ───────────────────────────────────────────────────────

function insideRings(rings: readonly (readonly Vec2[])[], p: Vec2): boolean {
  let inside = false;
  for (const r of rings)
    for (let i = 0, j = r.length - 1; i < r.length; j = i++) {
      const a = r[i];
      const b = r[j];
      if (a.y > p.y !== b.y > p.y && p.x < ((b.x - a.x) * (p.y - a.y)) / (b.y - a.y) + a.x) inside = !inside;
    }
  return inside;
}

/**
 * A point inside the area where a symbol or text sits well: the centroid
 * when it is inside, else the middle of the widest inside stretch of a
 * few horizontal scan lines (GEOS "point on surface", simplified).
 */
export function interiorPoint(rings: readonly (readonly Vec2[])[]): Vec2 | null {
  const outer = rings[0];
  if (!outer || outer.length < 3) return null;
  const c = centroid(outer);
  if (insideRings(rings, c)) return c;
  let minY = Infinity;
  let maxY = -Infinity;
  for (const p of outer) {
    minY = Math.min(minY, p.y);
    maxY = Math.max(maxY, p.y);
  }
  let best: { x: number; y: number; w: number } | null = null;
  for (const f of [0.5, 0.35, 0.65, 0.2, 0.8, 0.1, 0.9]) {
    const y = minY + (maxY - minY) * f;
    const xs: number[] = [];
    for (const r of rings)
      for (let i = 0, j = r.length - 1; i < r.length; j = i++) {
        const a = r[i];
        const b = r[j];
        if (a.y > y !== b.y > y) xs.push(a.x + ((y - a.y) * (b.x - a.x)) / (b.y - a.y));
      }
    xs.sort((p, q) => p - q);
    for (let k = 0; k + 1 < xs.length; k += 2) {
      const w = xs[k + 1] - xs[k];
      if (!best || w > best.w) best = { x: (xs[k] + xs[k + 1]) / 2, y, w };
    }
  }
  return best ? { x: best.x, y: best.y } : c;
}
