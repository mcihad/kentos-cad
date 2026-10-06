import type { Bounds, Vec2 } from '../../model/geometry';
import { OPEN_SHAPES, shapeOutline, type PathSink } from '../../render/canvasShapes';
import { MARKER_STRIDE, STROKE_STRIDE, type MarkerBatch, type PaintFillBatch, type RGBA, type SceneLayer, type StrokeBatch, type StyleUnit } from '../../render/types';

/**
 * A map frame's drawing as vectors for the PDF (docs/sheet/design.md §9a,
 * `MapContent::Vector`): what the map frames draw through the drawing's own
 * pipeline (the styled layers the style core built at the map's scale,
 * render/styledLayer.ts), turned back into paths on the ground (Y east, X
 * north, metres) with their looks on the paper (widths and dashes in mm):
 *
 * - a stroke batch's segments joined into its paths (the batch marks where a
 *   path ends), its colour, width, dashes and caps;
 * - a solid fill's triangles into the rings that bound them (each triangle
 *   turned counter-clockwise, the edges no other triangle shares, chained),
 *   filled non-zero: holes stay holes, overlaps stay filled;
 * - a hatch as its lines, clipped to the triangles, dashes kept on the
 *   ground as the shader keeps them;
 * - a marker of one of the drawing's shapes as its outline (the shapes'
 *   own code, render/canvasShapes.ts), placed, turned and anchored as the
 *   shader places it.
 *
 * What has no vector form here (a pattern or picture fill, a picture or
 * text marker, a blurred line) is named: the map then goes to the PDF as a
 * picture instead (`MapContent::Raster`), and the export window says why.
 */

export interface VecStroke {
  /** `#rrggbb`. */
  readonly color: string;
  readonly opacity: number;
  /** Paper mm. */
  readonly width: number;
  /** On/off lengths in paper mm; null: continuous. */
  readonly dash: readonly number[] | null;
  readonly dashOffset: number;
  readonly cap: 'butt' | 'round' | 'square';
  readonly join: 'round' | 'miter';
}

export interface VecFill {
  readonly color: string;
  readonly opacity: number;
  readonly rule: 'nonzero' | 'evenodd';
}

/** One path: rings or lines of ground points (x east, y north, metres) drawn with one look. */
export interface VecPath {
  readonly parts: readonly { readonly points: readonly number[]; readonly closed: boolean }[];
  readonly stroke?: VecStroke;
  readonly fill?: VecFill;
  /**
   * Where each shape's parts start (a marker's outline and its hole are one shape): an even-odd fill is
   * one shape's rings, never another's (two markers overlapping both stay filled). None: one shape.
   */
  readonly groups?: readonly number[];
}

export interface VecLayerShapes {
  readonly paths: VecPath[];
  /** What this layer draws that has no vector form here (each said once). */
  readonly unsupported: string[];
}

export interface VectorScale {
  /** The map's scale denominator (1/N): metres of ground per metre of paper. */
  readonly scale: number;
  /** The document's origin: the batches' numbers are relative to it. */
  readonly origin: Vec2;
}

const hex = (c: RGBA) => `#${[c[0], c[1], c[2]].map((v) => Math.round(Math.min(1, Math.max(0, v)) * 255).toString(16).padStart(2, '0')).join('')}`;

/** Paper mm of a length in a batch's unit: metres of ground at the map's scale, or CSS px (96 dpi). */
export function paperMm(v: number, unit: StyleUnit, scale: number): number {
  return unit === 'world' ? (v * 1000) / scale : (v * 25.4) / 96;
}

/** The ground offset of a batch's numbers: the document's origin and the batch's tile (docs/adr/0157). */
function base(o: VectorScale, tile?: readonly [number, number]): [number, number] {
  return [o.origin.x + (tile?.[0] ?? 0), o.origin.y + (tile?.[1] ?? 0)];
}

/** A stroke batch's paths: segments joined where one ends and the next begins (the batch marks path ends). */
export function strokePaths(b: StrokeBatch, o: VectorScale): VecPath | null {
  const [bx, by] = base(o, b.origin);
  const s = b.segments;
  const parts: { points: number[]; closed: boolean }[] = [];
  let cur: number[] | null = null;
  for (let i = 0; i < s.length; i += STROKE_STRIDE) {
    const ax = s[i] + bx;
    const ay = s[i + 1] + by;
    const ex = s[i + 2] + bx;
    const ey = s[i + 3] + by;
    const flags = s[i + 5];
    const n: number = cur ? cur.length : 0;
    if (!cur || flags & 1 || cur[n - 2] !== ax || cur[n - 1] !== ay) {
      cur = [ax, ay];
      parts.push({ points: cur, closed: false });
    }
    cur.push(ex, ey);
    if (flags & 2) cur = null;
  }
  for (const p of parts) {
    const n = p.points.length;
    // A path that comes back to where it began is a ring (its corner then joins, not caps).
    if (n > 6 && p.points[0] === p.points[n - 2] && p.points[1] === p.points[n - 1]) {
      p.points.length = n - 2;
      p.closed = true;
    }
  }
  if (!parts.length) return null;
  return {
    parts,
    stroke: {
      color: hex(b.color),
      opacity: b.color[3],
      width: paperMm(b.width, b.unit, o.scale),
      dash: b.dash?.length ? b.dash.map((d) => paperMm(d, b.unit, o.scale)) : null,
      dashOffset: paperMm(b.dashOffset, b.unit, o.scale),
      cap: b.cap,
      join: 'round',
    },
  };
}

/** The rings bounding a triangle list (x, y pairs): each triangle counter-clockwise, its unshared edges chained. */
export function triangleRings(t: Float32Array | readonly number[], dx: number, dy: number): { points: number[]; closed: true }[] {
  const key = (x: number, y: number) => `${x},${y}`;
  const edges = new Map<string, string[]>();
  const point = new Map<string, [number, number]>();
  const add = (a: [number, number], b: [number, number]) => {
    const ka = key(...a);
    const kb = key(...b);
    if (ka === kb) return;
    // An edge another triangle runs the other way is inside: both go.
    const back = edges.get(kb);
    const at = back?.indexOf(ka) ?? -1;
    if (back && at >= 0) {
      back.splice(at, 1);
      return;
    }
    point.set(ka, a);
    point.set(kb, b);
    const list = edges.get(ka) ?? [];
    list.push(kb);
    edges.set(ka, list);
  };
  for (let i = 0; i + 5 < t.length; i += 6) {
    const a: [number, number] = [t[i], t[i + 1]];
    let b: [number, number] = [t[i + 2], t[i + 3]];
    let c: [number, number] = [t[i + 4], t[i + 5]];
    const cross = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
    if (cross === 0) continue;
    if (cross < 0) [b, c] = [c, b];
    add(a, b);
    add(b, c);
    add(c, a);
  }
  const rings: { points: number[]; closed: true }[] = [];
  for (const [start, nexts] of edges) {
    while (nexts.length) {
      const ring: number[] = [];
      let at = start;
      for (let guard = 0; guard < 1e7; guard++) {
        const p = point.get(at)!;
        ring.push(p[0] + dx, p[1] + dy);
        const list = edges.get(at);
        const next = list?.pop();
        if (next === undefined || next === start) break;
        at = next;
      }
      if (ring.length >= 6) rings.push({ points: ring, closed: true });
    }
  }
  return rings;
}

/** The parts of a line `n·p = c` inside the triangles, as parameter intervals along `d` (merged). */
function lineInTriangles(t: Float32Array, n: Vec2, d: Vec2, c: number): [number, number][] {
  const spans: [number, number][] = [];
  for (let i = 0; i + 5 < t.length; i += 6) {
    const pts: number[] = [];
    for (let k = 0; k < 3; k++) {
      const ax = t[i + 2 * k];
      const ay = t[i + 2 * k + 1];
      const bx = t[i + ((2 * k + 2) % 6)];
      const by = t[i + ((2 * k + 3) % 6)];
      const sa = n.x * ax + n.y * ay - c;
      const sb = n.x * bx + n.y * by - c;
      if ((sa < 0 && sb < 0) || (sa > 0 && sb > 0) || sa === sb) continue;
      const f = sa / (sa - sb);
      const x = ax + (bx - ax) * f;
      const y = ay + (by - ay) * f;
      pts.push(d.x * x + d.y * y);
    }
    if (pts.length >= 2) {
      const lo = Math.min(...pts);
      const hi = Math.max(...pts);
      if (hi > lo) spans.push([lo, hi]);
    }
  }
  spans.sort((a, b) => a[0] - b[0]);
  const out: [number, number][] = [];
  for (const s of spans) {
    const last = out[out.length - 1];
    if (last && s[0] <= last[1] + 1e-9) last[1] = Math.max(last[1], s[1]);
    else out.push([s[0], s[1]]);
  }
  return out;
}

/** A fill batch: solid as its rings, a hatch as its clipped lines; null with the reason when it has no vector form. */
export function fillPaths(b: PaintFillBatch, o: VectorScale): { paths: VecPath[] } | { why: string } {
  const [bx, by] = base(o, b.origin);
  const p = b.paint;
  if (p.kind === 'solid') {
    const parts = triangleRings(b.positions, bx, by);
    return { paths: parts.length ? [{ parts, fill: { color: hex(p.color), opacity: p.color[3], rule: 'nonzero' } }] : [] };
  }
  if (p.kind === 'hatch') {
    if (p.spacing <= 0) return { paths: [] };
    // The batch's angle is in radians (the renderers' cos(angle)).
    const d = { x: Math.cos(p.angle), y: Math.sin(p.angle) };
    const n = { x: -d.y, y: d.x };
    const t = b.positions;
    let lo = Infinity;
    let hi = -Infinity;
    for (let i = 0; i + 1 < t.length; i += 2) {
      const v = n.x * t[i] + n.y * t[i + 1];
      lo = Math.min(lo, v);
      hi = Math.max(hi, v);
    }
    // Lines where n·p − offset is a whole number of spacings (the shader's), in the batch's own frame.
    const first = Math.ceil((lo - p.offset) / p.spacing);
    const last = Math.floor((hi - p.offset) / p.spacing);
    if (last - first > 20_000) return { why: 'çok sık tarama' };
    // A dashed family's dashes are cut here, each line from its own phase (the shader's dashCover: along − line·stagger
    // + dash offset; docs/adr/0186 §3); a dot is a point. The desktop's is map_vectors.rs's `dash_pieces`.
    const dashes = p.dash?.length ? shaderDashes(p.dash) : null;
    const parts: { points: number[]; closed: boolean }[] = [];
    const dots: { points: number[]; closed: boolean }[] = [];
    for (let j = first; j <= last; j++) {
      const c = p.offset + j * p.spacing;
      for (const span of lineInTriangles(t, n, d, c)) {
        const pieces = dashes ? dashPieces(span, dashes, p.dashOffset - j * (p.stagger ?? 0)) : [span];
        for (const [s0, s1] of pieces) (s1 > s0 ? parts : dots).push({ points: [d.x * s0 + n.x * c + bx, d.y * s0 + n.y * c + by, d.x * s1 + n.x * c + bx, d.y * s1 + n.y * c + by], closed: false });
        if (parts.length + dots.length > MAX_HATCH_PIECES) return { why: 'çok sık tarama' };
      }
    }
    const width = paperMm(p.width, p.unit, o.scale);
    const stroke = (w: number, cap: 'butt' | 'round') => ({ color: hex(p.color), opacity: p.color[3], width: w, dash: null, dashOffset: 0, cap, join: 'miter' as const });
    const paths: VecPath[] = [];
    if (parts.length) paths.push({ parts, stroke: stroke(width, 'butt') });
    if (dots.length) paths.push({ parts: dots, stroke: stroke(Math.max(width, DOT_MM), 'round') });
    return { paths };
  }
  if (p.kind === 'gradient') return { why: 'degrade dolgu' };
  return { why: p.kind === 'pattern' ? 'desen dolgusu' : 'resimli dolgu' };
}

/** The most lines and dashes a hatch batch is cut into here. */
const MAX_HATCH_PIECES = 200_000;
/** A hatch's dot on the paper, millimetres across. */
const DOT_MM = 0.3;

/** A dash list as the shaders repeat it: an odd one twice, at most eight values. */
function shaderDashes(dash: readonly number[]): number[] {
  return (dash.length % 2 ? [...dash, ...dash] : [...dash]).slice(0, 8);
}

/**
 * The drawn pieces of a span of a dashed line whose pattern starts `phase` before the line's zero (dashCover's
 * t = s + phase), as parameter intervals; a dot (a drawn dash of no length before a gap) as a zero-length one.
 */
function dashPieces(span: readonly [number, number], dashes: readonly number[], phase: number): [number, number][] {
  const total = dashes.reduce((a, b) => a + b, 0);
  if (!(total > 0)) return [[span[0], span[1]]];
  const out: [number, number][] = [];
  for (let k = Math.floor((span[0] + phase) / total); k * total - phase < span[1]; k++) {
    let at = k * total - phase;
    dashes.forEach((len, i) => {
      if (i % 2 === 0) {
        const [a, b] = [at, at + len];
        if (len === 0) {
          if (a >= span[0] && a <= span[1] && (dashes[i + 1] ?? 0) > 0) out.push([a, a]);
        } else if (b > span[0] && a < span[1]) out.push([Math.max(a, span[0]), Math.min(b, span[1])]);
      }
      at += len;
    });
  }
  return out;
}

/** Points of a shape's outline (arcs as short chords), from the shapes' own code. */
class Outline implements PathSink {
  readonly parts: { points: number[]; closed: boolean }[] = [];
  private cur: number[] | null = null;
  private readonly step: number;

  /** `step`: the largest chord angle of an arc, radians. */
  constructor(step = Math.PI / 32) {
    this.step = step;
  }
  moveTo(x: number, y: number): void {
    this.cur = [x, y];
    this.parts.push({ points: this.cur, closed: false });
  }
  lineTo(x: number, y: number): void {
    if (!this.cur) return this.moveTo(x, y);
    this.cur.push(x, y);
  }
  closePath(): void {
    const p = this.parts[this.parts.length - 1];
    if (p && this.cur === p.points) p.closed = true;
    this.cur = null;
  }

  /** The parts, a part that ends where it starts (a full circle drawn as an arc) closed: a fill fills it, as a canvas does. */
  rings(): { points: number[]; closed: boolean }[] {
    return this.parts.map((p) => {
      const n = p.points.length;
      const back = !p.closed && n >= 8 && Math.abs(p.points[0] - p.points[n - 2]) < 1e-9 && Math.abs(p.points[1] - p.points[n - 1]) < 1e-9;
      return back ? { points: p.points.slice(0, n - 2), closed: true } : p;
    });
  }
  rect(x: number, y: number, w: number, h: number): void {
    this.moveTo(x, y);
    this.lineTo(x + w, y);
    this.lineTo(x + w, y + h);
    this.lineTo(x, y + h);
    this.closePath();
  }
  arc(x: number, y: number, r: number, a0: number, a1: number, ccw = false): void {
    let sweep = a1 - a0;
    if (!ccw && sweep < 0) sweep += 2 * Math.PI;
    if (ccw && sweep > 0) sweep -= 2 * Math.PI;
    if (Math.abs(a1 - a0) >= 2 * Math.PI) sweep = ccw ? -2 * Math.PI : 2 * Math.PI;
    const n = Math.max(2, Math.ceil(Math.abs(sweep) / this.step));
    for (let i = 0; i <= n; i++) {
      const a = a0 + (sweep * i) / n;
      const px = x + Math.cos(a) * r;
      const py = y + Math.sin(a) * r;
      if (i === 0 && !this.cur) this.moveTo(px, py);
      else this.lineTo(px, py);
    }
  }
}

/** A marker batch of the drawing's shapes as outlines on the ground; the reason when its look has no vector form here. */
export function markerPaths(b: MarkerBatch, o: VectorScale): { paths: VecPath[] } | { why: string } {
  const look = b.look;
  if (look.kind !== 'shape') return { why: look.image.kind === 'text' ? 'yazı simgesi' : 'resimli simge' };
  const [bx, by] = base(o, b.origin);
  const open = OPEN_SHAPES.has(look.shape);
  const strokeColor = look.stroke ?? (open ? look.fill : null);
  const toMm = (v: number) => paperMm(v, b.unit, o.scale);
  // Sizes in the batch's unit; ground metres per unit: 1 for world, a CSS px's paper mm at the scale for px.
  const perUnit = b.unit === 'world' ? 1 : ((25.4 / 96) * o.scale) / 1000;
  const fills: { points: number[]; closed: boolean }[] = [];
  // Where each marker's parts start among `fills`: its even-odd fill is its own (an outline and its hole).
  const groups: number[] = [];
  const strokes: { points: number[]; closed: boolean }[] = [];
  const dots: { points: number[]; closed: boolean }[] = [];
  const v = b.instances;
  for (let i = 0; i < v.length; i += MARKER_STRIDE) {
    const w = v[i + 3];
    const h = v[i + 4] > 0 ? v[i + 4] : w;
    const outline = new Outline();
    shapeOutline(outline, look.shape, w / 2, h / 2, look.params);
    const ca = Math.cos(v[i + 2]);
    const sa = Math.sin(v[i + 2]);
    // The shape's centre from its point: the anchor and the offset, turned with it (the marker shader's q).
    const qx = b.offset[0] - b.anchor[0] * w;
    const qy = b.offset[1] - b.anchor[1] * h;
    const px = v[i] + bx;
    const py = v[i + 1] + by;
    const place = (x: number, y: number): [number, number] => [px + (ca * (x + qx) - sa * (y + qy)) * perUnit, py + (sa * (x + qx) + ca * (y + qy)) * perUnit];
    if (!open) groups.push(fills.length);
    for (const part of outline.rings()) {
      const pts: number[] = [];
      for (let k = 0; k + 1 < part.points.length; k += 2) pts.push(...place(part.points[k], part.points[k + 1]));
      (open ? strokes : fills).push({ points: pts, closed: part.closed });
    }
    if (look.shape === 'ring' && strokeColor) {
      const dot = new Outline();
      dot.arc(0, 0, Math.max(look.strokeWidth, Math.min(w, h) * 0.08), 0, 2 * Math.PI);
      dot.closePath();
      for (const part of dot.parts) {
        const pts: number[] = [];
        for (let k = 0; k + 1 < part.points.length; k += 2) pts.push(...place(part.points[k], part.points[k + 1]));
        dots.push({ points: pts, closed: true });
      }
    }
  }
  const paths: VecPath[] = [];
  const op = b.opacity;
  if (fills.length && ((look.fill && !open) || strokeColor)) {
    paths.push({
      parts: fills,
      groups,
      ...(!open && look.fill && { fill: { color: hex(look.fill), opacity: look.fill[3] * op, rule: 'evenodd' as const } }),
      ...(strokeColor && { stroke: { color: hex(strokeColor), opacity: strokeColor[3] * op, width: toMm(Math.max(look.strokeWidth, 0)), dash: null, dashOffset: 0, cap: 'butt' as const, join: 'miter' as const } }),
    });
  }
  if (strokes.length && strokeColor) paths.push({ parts: strokes, stroke: { color: hex(strokeColor), opacity: strokeColor[3] * op, width: toMm(Math.max(look.strokeWidth, 0)), dash: null, dashOffset: 0, cap: 'butt', join: 'miter' } });
  if (dots.length && strokeColor) paths.push({ parts: dots, fill: { color: hex(strokeColor), opacity: strokeColor[3] * op, rule: 'nonzero' } });
  return { paths };
}

/**
 * Whether a batch's geometry reaches the ground box `b`. Its box is from the document's origin, as every batch's
 * (the style core's `batch.rs`, `batchInView`), not from its tile: the tile moves only its numbers (docs/adr/0157).
 * With the tile added, a drawing a tile or more from the anchor lost its map.
 */
function batchReaches(bounds: readonly [number, number, number, number], o: VectorScale, b: Bounds | undefined): boolean {
  if (!b) return true;
  const { x: bx, y: by } = o.origin;
  return bounds[2] + bx >= b.minX && bounds[0] + bx <= b.maxX && bounds[3] + by >= b.minY && bounds[1] + by <= b.maxY;
}

/**
 * A built layer's drawing as paths, in its draw order; what has no vector
 * form is named. With `within`, batches that do not reach that ground box
 * are left out (and so is what they could not say).
 */
export function layerPaths(layer: SceneLayer, o: VectorScale, within?: Bounds): VecLayerShapes {
  const paths: VecPath[] = [];
  const unsupported = new Set<string>();
  // The plain batches (the simple look without the style engine): fills, then lines, then points.
  for (const f of layer.fills) {
    const parts = triangleRings(f.positions, o.origin.x, o.origin.y);
    if (parts.length) paths.push({ parts, fill: { color: hex(f.color), opacity: f.color[3], rule: 'nonzero' } });
  }
  for (const b of layer.styled ?? []) {
    if (!batchReaches(b.bounds, o, within)) continue;
    if (b.kind === 'stroke') {
      if (b.blur > 0) unsupported.add('yumuşak kenarlı çizgi');
      const p = strokePaths(b, o);
      if (p) paths.push(p);
    } else if (b.kind === 'fill') {
      const r = fillPaths(b, o);
      if ('why' in r) unsupported.add(r.why);
      else paths.push(...r.paths);
    } else {
      const r = markerPaths(b, o);
      if ('why' in r) unsupported.add(r.why);
      else paths.push(...r.paths);
    }
  }
  for (const l of layer.lines) {
    const parts: { points: number[]; closed: boolean }[] = [];
    for (let i = 0; i + 3 < l.positions.length; i += 4) parts.push({ points: [l.positions[i] + o.origin.x, l.positions[i + 1] + o.origin.y, l.positions[i + 2] + o.origin.x, l.positions[i + 3] + o.origin.y], closed: false });
    if (parts.length) paths.push({ parts, stroke: { color: hex(l.color), opacity: l.color[3], width: 0.1, dash: l.dash?.length ? l.dash.map((d) => (d * 25.4) / 96) : null, dashOffset: 0, cap: 'butt', join: 'round' } });
  }
  if (layer.points.length) unsupported.add('düz noktalar');
  return { paths, unsupported: [...unsupported] };
}
