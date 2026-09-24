import type { Entity } from '../model/entities';
import { compileExpression, type CompiledExpression, type ExprAs, type ExprColumn, type ExprObjects, type ExprValue } from '../model/expression/expression';
import { offsetPath } from '../model/geom/offset';
import type { Vec2 } from '../model/geometry';
import { interiorPoint, placeAlong, wavePaths, type StyledGeometry } from './geometry';
import type { FillPaint, MarkerCommon, MarkerStyle, PrimitiveSink, PrimUnit, ShapeMarkStyle, StrokeStyle } from './primitives';
import type { Anchor, DataDefined, FillLayer, LineLayer, MarkerLayer, MarkerSymbol, SizeUnit, Symbol } from '../model/style';

/**
 * Symbol × geometry × object → drawing primitives. Pure and CPU-side:
 * units are converted (mm on paper → metres at the plot scale), data-
 * defined values are evaluated, parallel offsets and marker positions
 * along lines are computed in float64. The result feeds the GPU batches
 * and the Canvas2D previews alike.
 */

/** A CSS pixel on paper (96 dpi), used where a length in px must become geometry. */
export const MM_PER_PX = 25.4 / 96;

/** Compiled expressions by source, shared across a build (null = does not compile). */
export class ExprCache {
  private readonly map = new Map<string, CompiledExpression | null>();
  /** Sources that did not compile, for the style designer to show. */
  readonly errors = new Map<string, string>();

  get(src: string): CompiledExpression | null {
    let e = this.map.get(src);
    if (e === undefined) {
      const r = compileExpression(src);
      e = r.ok ? r.expr : null;
      if (!r.ok) this.errors.set(src, r.error);
      this.map.set(src, e);
    }
    return e;
  }
}

/**
 * The expressions of one build (a layer, a preview) evaluated for all its
 * objects at once: the first object that needs an expression's value gets
 * every object's, in one call to the core (docs/adr/0008 “İfade dili”).
 */
export class ExprRun {
  private readonly exprs: ExprCache;
  private readonly objects: ExprObjects;
  private readonly columns = new Map<string, ExprColumn | null>();

  constructor(exprs: ExprCache, objects: ExprObjects) {
    this.exprs = exprs;
    // The geometry values are asked for once, whichever expression first reads one.
    const fetch = objects.measures;
    let measures: Float64Array | null = null;
    this.objects = fetch ? { ...objects, measures: () => (measures ??= fetch()) } : objects;
  }

  /** The value of `src` for the object at 1-based `index`, as `as` asks; null when it does not compile. */
  value(src: string, index: number, as: ExprAs): ExprValue {
    const key = `${as}\u0000${src}`;
    let c = this.columns.get(key);
    if (c === undefined) {
      const e = this.exprs.get(src);
      c = e ? e.evaluateAll(this.objects, as) : null;
      this.columns.set(key, c);
    }
    return c ? c.value(index - 1) : null;
  }
}

export interface CompileEnv {
  /** Denominator of the project's plot scale (1000 for 1:1000): 1 mm on paper = plotScale/1000 m. */
  readonly plotScale: number;
  readonly exprs: ExprCache;
  /** The data-defined values of this build's objects; `CompileTarget.index` is a position in them. */
  readonly run: ExprRun;
  /** Height/width of an image asset (tiles keep their proportions); 1 when unknown. */
  assetAspect?(id: string): number;
}

/** The object and its 1-based position in the build (`CompileEnv.run`), as expressions see it. */
export interface CompileTarget {
  readonly entity: Entity;
  readonly index: number;
}

export function ddNumber(v: DataDefined<number> | undefined, t: CompileTarget, env: CompileEnv, fallback: number): number {
  if (v === undefined) return fallback;
  if (typeof v === 'number') return v;
  const n = env.run.value(v.expr, t.index, 'number') as number | null;
  return n ?? v.fallback ?? fallback;
}

export function ddText(v: DataDefined<string> | undefined, t: CompileTarget, env: CompileEnv): string {
  if (v === undefined) return '';
  if (typeof v === 'string') return v;
  const r = env.run.value(v.expr, t.index, 'text') as string | null;
  return r === null ? (v.fallback ?? '') : r;
}

export function ddBool(v: DataDefined<boolean> | undefined, t: CompileTarget, env: CompileEnv): boolean {
  if (v === undefined) return true;
  if (typeof v === 'boolean') return v;
  const r = env.run.value(v.expr, t.index, 'bool');
  return r === null ? (v.fallback ?? true) : r === true;
}

const COLOR = /^(#[0-9a-f]{6}([0-9a-f]{2})?|ink|paper|fg|fg-dim)$/i;

export function ddColor(v: DataDefined<string> | null | undefined, t: CompileTarget, env: CompileEnv): string | null {
  if (v === undefined || v === null) return null;
  if (typeof v === 'string') return v;
  const r = ((env.run.value(v.expr, t.index, 'text') as string | null) ?? '').trim();
  return COLOR.test(r) ? r : (v.fallback ?? null);
}

/** A length that changes geometry (offset, interval, spacing) in metres. */
export function toWorld(v: number, unit: SizeUnit | undefined, env: CompileEnv): number {
  if (unit === 'm') return v;
  const mm = unit === 'px' ? v * MM_PER_PX : v;
  return (mm * env.plotScale) / 1000;
}

/** A drawn size (width, marker size, dash): px stays px for the shader, the rest becomes metres. */
export function toDrawn(v: number, unit: SizeUnit | undefined, env: CompileEnv): { v: number; unit: PrimUnit } {
  return unit === 'px' ? { v, unit: 'px' } : { v: toWorld(v, unit, env), unit: 'world' };
}

const DEG = Math.PI / 180;

// ── Markers ────────────────────────────────────────────────────────────

/** One marker layer at a point; `angle` is the placement direction (radians). */
export function emitMarker(layer: MarkerLayer, at: Vec2, angle: number, level: number, t: CompileTarget, env: CompileEnv, sink: PrimitiveSink): void {
  const style = markerStyle(layer, level, t, env);
  if (style) sink.marker(style, at, angle);
}

export function markerStyle(layer: MarkerLayer, level: number, t: CompileTarget, env: CompileEnv): MarkerStyle | null {
  if (!ddBool(layer.enabled, t, env)) return null;
  const size = toDrawn(ddNumber(layer.size, t, env, 2), layer.unit, env);
  const off = layer.offset ?? [0, 0];
  const conv = (v: number) => (size.unit === 'px' ? v : toWorld(v, layer.unit, env));
  const common: MarkerCommon = {
    unit: size.unit,
    opacity: layer.opacity ?? 1,
    offset: [conv(off[0]), conv(off[1])],
    anchor: layer.anchor ?? 'center',
    rotation: ddNumber(layer.rotation, t, env, 0) * DEG,
    level,
  };
  switch (layer.type) {
    case 'shape':
      return {
        kind: 'shape',
        shape: layer.shape,
        size: size.v,
        height: layer.height !== undefined ? conv(layer.height) : size.v,
        fill: ddColor(layer.fill, t, env),
        stroke: ddColor(layer.stroke, t, env),
        strokeWidth: layer.strokeWidth !== undefined ? conv(layer.strokeWidth) : 0,
        params: [
          Math.min(0.95, Math.max(0, layer.hole ?? 0)),
          Math.round(Math.min(64, Math.max(3, layer.teeth ?? 12))),
          Math.min(360, Math.max(1, layer.sweep ?? 180)) * DEG,
          Math.min(0.6, Math.max(0.02, layer.teethDepth ?? 0.2)),
        ],
        common,
      };
    case 'svg':
      return { kind: 'svg', asset: layer.asset, size: size.v, fill: ddColor(layer.fill, t, env), stroke: ddColor(layer.stroke, t, env), common };
    case 'raster':
      return { kind: 'raster', asset: layer.asset, size: size.v, common };
    case 'text': {
      const text = ddText(layer.text, t, env);
      if (!text) return null;
      return {
        kind: 'text',
        text,
        size: size.v,
        font: layer.font ?? 'ui',
        weight: layer.weight ?? 400,
        italic: !!layer.italic,
        color: ddColor(layer.color, t, env) ?? 'ink',
        halo: layer.halo ? { color: layer.halo.color, width: conv(layer.halo.width) } : null,
        common,
      };
    }
  }
}

function emitMarkerSymbol(symbol: MarkerSymbol, at: Vec2, angle: number, level: number, t: CompileTarget, env: CompileEnv, sink: PrimitiveSink): void {
  symbol.layers.forEach((layer, j) => emitMarker(layer, at, angle, subLevel(level, j), t, env, sink));
}

/**
 * The drawing level of layer `j` of a marker symbol nested in a line or fill
 * layer. The sink merges look-alike marks of every object on a CAD layer into
 * one batch, so without its own level a paper-filled frame of one object
 * could cover the mark another object draws over its frame.
 */
const subLevel = (level: number, j: number) => level + Math.min(j, 255) / 256;

/** Text following a line turns half a turn where it would read upside down (leftwards, or downwards). */
function readsBackwards(angle: number): boolean {
  const c = Math.cos(angle);
  return c < -1e-9 || (Math.abs(c) <= 1e-9 && Math.sin(angle) < 0);
}

/** About half a text mark's length along its line, in world units (px-sized text is left alone). */
function textHalfLength(st: MarkerStyle): number {
  if (st.kind !== 'text' || st.common.unit !== 'world') return 0;
  return (st.text.length * 0.3 + 0.3) * st.size + Math.abs(st.common.offset[0]);
}

const MIRRORED_ANCHOR: Record<Anchor, Anchor> = {
  center: 'center',
  top: 'bottom',
  bottom: 'top',
  left: 'right',
  right: 'left',
  'top-left': 'bottom-right',
  'top-right': 'bottom-left',
  'bottom-left': 'top-right',
  'bottom-right': 'top-left',
};

/**
 * A text style turned half a turn in place: the offset and the anchor are
 * mirrored, so the text keeps the same box on the same side of the line and
 * only its letters turn.
 */
function turnedText(st: MarkerStyle): MarkerStyle {
  const c = st.common;
  return { ...st, common: { ...c, offset: [-c.offset[0], -c.offset[1]], anchor: MIRRORED_ANCHOR[c.anchor] ?? c.anchor } };
}

// ── Lines ──────────────────────────────────────────────────────────────

function strokeStyle(layer: Extract<LineLayer, { type: 'simpleLine' }>, level: number, t: CompileTarget, env: CompileEnv): StrokeStyle | null {
  const color = ddColor(layer.color, t, env);
  if (!color) return null;
  const width = toDrawn(ddNumber(layer.width, t, env, 0), layer.unit, env);
  const dash = layer.dash?.length ? layer.dash.map((d) => (width.unit === 'px' ? d : toWorld(d, layer.unit, env))) : null;
  return {
    color,
    opacity: layer.opacity ?? 1,
    width: width.v,
    unit: width.unit,
    dash,
    dashOffset: layer.dashOffset ? (width.unit === 'px' ? layer.dashOffset : toWorld(layer.dashOffset, layer.unit, env)) : 0,
    cap: layer.cap ?? 'butt',
    join: layer.join ?? 'miter',
    blur: layer.blur ? (width.unit === 'px' ? layer.blur : toWorld(layer.blur, layer.unit, env)) : 0,
    level,
  };
}

/** A line layer on one path (a line, or an area ring when `ring` says which). */
function emitLineLayer(layer: LineLayer, pts: readonly Vec2[], closed: boolean, level: number, t: CompileTarget, env: CompileEnv, sink: PrimitiveSink): void {
  if (!ddBool(layer.enabled, t, env)) return;
  const off = ddNumber(layer.offset, t, env, 0);
  const d = off ? toWorld(off, layer.unit, env) : 0;
  let path = d ? offsetPath(pts, d, closed) : pts;
  // A page shift moves the whole line the same way (north-up view: the page's right is east).
  const shift = layer.type === 'simpleLine' && layer.shift;
  if (shift && (shift[0] || shift[1])) {
    const dx = toWorld(shift[0], layer.unit, env);
    const dy = toWorld(shift[1], layer.unit, env);
    path = path.map((p) => ({ x: p.x + dx, y: p.y + dy }));
  }
  if (path.length < 2) return;
  if (layer.type === 'simpleLine') {
    const style = strokeStyle(layer, level, t, env);
    if (!style) return;
    const w = layer.wave;
    if (w && w.length > 0) {
      const len = (v: number) => toWorld(v, layer.unit, env);
      const spec = { shape: w.shape, length: len(w.length), amplitude: len(w.amplitude), spacing: len(w.spacing ?? w.length), connect: w.connect !== false, offsetAlong: w.offsetAlong !== undefined ? len(w.offsetAlong) : undefined };
      for (const piece of wavePaths(path, closed, spec)) sink.stroke(style, piece, false);
    } else sink.stroke(style, path, closed);
    return;
  }
  const interval = layer.interval !== undefined ? toWorld(layer.interval, layer.unit, env) : 0;
  const along = layer.offsetAlong !== undefined ? toWorld(layer.offsetAlong, layer.unit, env) : 0;
  const group = layer.group && layer.group.count > 1 ? { count: layer.group.count, spacing: toWorld(layer.group.spacing, layer.unit, env) } : undefined;
  // One style object per marker layer for the whole path: the sink keys batches by identity.
  const styles = layer.marker.layers.flatMap((m, j) => markerStyle(m, subLevel(level, j), t, env) ?? []);
  if (!styles.length) return;
  const follow = layer.rotate !== false;
  const turned = styles.map((st) => (follow && st.kind === 'text' ? turnedText(st) : null));
  // Text that follows the line keeps half its length clear of sharp corners instead of bending round them.
  const clear = follow ? Math.max(0, ...styles.map(textHalfLength)) : 0;
  for (const p of placeAlong(path, closed, layer.placement, interval, along, group, clear))
    styles.forEach((st, i) => {
      if (!follow) return sink.marker(st, p.at, 0);
      const alt = turned[i];
      if (alt && readsBackwards(p.angle + st.common.rotation)) sink.marker(alt, p.at, p.angle + Math.PI);
      else sink.marker(st, p.at, p.angle);
    });
}

// ── Fills ──────────────────────────────────────────────────────────────

function emitFillLayer(layer: FillLayer, rings: readonly (readonly Vec2[])[], level: number, t: CompileTarget, env: CompileEnv, sink: PrimitiveSink): void {
  if (layer.type === 'simpleLine' || layer.type === 'markerLine') {
    const which = layer.rings ?? 'all';
    rings.forEach((r, i) => {
      if ((which === 'exterior' && i > 0) || (which === 'interior' && i === 0)) return;
      emitLineLayer(layer, r, true, level, t, env, sink);
    });
    return;
  }
  if (!ddBool(layer.enabled, t, env)) return;
  const opacity = layer.opacity ?? 1;
  switch (layer.type) {
    case 'simpleFill': {
      const color = ddColor(layer.color, t, env);
      if (color) sink.fill({ kind: 'solid', color, opacity, level }, rings);
      return;
    }
    case 'hatchFill': {
      const color = ddColor(layer.color, t, env);
      if (!color || !(layer.spacing > 0)) return;
      const px = layer.unit === 'px';
      const len = (v: number) => (px ? v : toWorld(v, layer.unit, env));
      const paint: FillPaint = {
        kind: 'hatch',
        color,
        opacity,
        angle: layer.angle * DEG,
        spacing: len(layer.spacing),
        width: len(layer.width),
        offset: len(layer.offset ?? 0),
        dash: layer.dash?.length ? layer.dash.map(len) : null,
        dashOffset: len(layer.dashOffset ?? 0),
        unit: px ? 'px' : 'world',
        level,
      };
      sink.fill(paint, rings);
      return;
    }
    case 'patternFill': {
      if (!(layer.spacingX > 0 && layer.spacingY > 0)) return;
      const markers = layer.marker.layers.flatMap((m) => markerStyle(m, level, t, env) ?? []);
      if (!markers.length) return;
      const px = layer.unit === 'px';
      const len = (v: number) => (px ? v : toWorld(v, layer.unit, env));
      const off = layer.offset ?? [0, 0];
      const unit: PrimUnit = px ? 'px' : 'world';
      const size: [number, number] = [len(layer.spacingX), len(layer.spacingY)];
      const angle = (layer.angle ?? 0) * DEG;
      const offset: [number, number] = [len(off[0]), len(off[1])];
      // Shapes are drawn by the shader, one paint per marker layer, in the pattern's unit;
      // text and images go into one atlas tile after them.
      const inUnit = (v: number, u: PrimUnit) => (u === unit ? v : u === 'px' ? v * ((MM_PER_PX * env.plotScale) / 1000) : v / ((MM_PER_PX * env.plotScale) / 1000));
      for (const m of markers) {
        if (m.kind !== 'shape') continue;
        const u = m.common.unit;
        const mark: ShapeMarkStyle = {
          ...m,
          size: inUnit(m.size, u),
          height: inUnit(m.height, u),
          strokeWidth: inUnit(m.strokeWidth, u),
          common: { ...m.common, unit, offset: [inUnit(m.common.offset[0], u), inUnit(m.common.offset[1], u)] },
        };
        sink.fill({ kind: 'pattern', mark, size, stagger: !!layer.stagger, angle, offset, jitter: Math.min(1, Math.max(0, layer.jitter ?? 0)), coverage: Math.min(1, Math.max(0, layer.coverage ?? 1)), seed: layer.seed ?? 0, opacity, unit, level }, rings);
      }
      const others = markers.filter((m) => m.kind !== 'shape');
      if (others.length) sink.fill({ kind: 'tile', tile: { kind: 'markers', markers: others, stagger: !!layer.stagger }, size, angle, offset, opacity, unit, level }, rings);
      return;
    }
    case 'imageFill': {
      if (!(layer.tileSize > 0)) return;
      const px = layer.unit === 'px';
      const w = px ? layer.tileSize : toWorld(layer.tileSize, layer.unit, env);
      const aspect = env.assetAspect?.(layer.asset) ?? 1;
      sink.fill({ kind: 'tile', tile: { kind: 'asset', asset: layer.asset }, size: [w, w * aspect], angle: (layer.angle ?? 0) * DEG, offset: [0, 0], opacity, unit: px ? 'px' : 'world', level }, rings);
      return;
    }
    case 'centroidMarker': {
      const at = layer.position === 'centroid' ? centroidOf(rings[0]) : interiorPoint(rings);
      if (at) emitMarkerSymbol(layer.marker, at, 0, level, t, env, sink);
      return;
    }
  }
}

function centroidOf(ring: readonly Vec2[] | undefined): Vec2 | null {
  if (!ring?.length) return null;
  let a = 0;
  let cx = 0;
  let cy = 0;
  for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
    const f = ring[j].x * ring[i].y - ring[i].x * ring[j].y;
    a += f;
    cx += (ring[j].x + ring[i].x) * f;
    cy += (ring[j].y + ring[i].y) * f;
  }
  return Math.abs(a) < 1e-12 ? ring[0] : { x: cx / (3 * a), y: cy / (3 * a) };
}

// ── Entry point ────────────────────────────────────────────────────────

/**
 * Draws a symbol on a geometry. A symbol of another class adapts, so any
 * symbol can be given to any object: a line symbol on an area draws its
 * edges (the area's rings as closed lines, left = inside), a marker symbol
 * sits at an area's inside point or a line's middle, a fill symbol fills a
 * closed line. A fill symbol on an open line or a point, and a line symbol
 * on a point, draw nothing. `levelBase` orders symbol layers across a layer
 * (fills first, then lines, then markers: see the scene builder).
 */
export function compileSymbol(symbol: Symbol, geom: StyledGeometry, t: CompileTarget, env: CompileEnv, sink: PrimitiveSink, levelBase = 0): void {
  if (symbol.type === 'marker') {
    const at = geom.cls === 'marker' ? geom.point : geom.cls === 'fill' ? interiorPoint(geom.rings) : lineMiddle(geom.paths);
    if (at) symbol.layers.forEach((l, i) => emitMarker(l, at, 0, levelBase + i, t, env, sink));
  } else if (symbol.type === 'line') {
    const paths = geom.cls === 'line' ? geom.paths : geom.cls === 'fill' ? geom.rings.map((pts) => ({ pts, closed: true })) : [];
    symbol.layers.forEach((l, i) => {
      for (const p of paths) emitLineLayer(l, p.pts, p.closed, levelBase + i, t, env, sink);
    });
  } else {
    const rings = geom.cls === 'fill' ? geom.rings : geom.cls === 'line' ? geom.paths.filter((p) => p.closed && p.pts.length > 2).map((p) => p.pts) : [];
    if (rings.length) symbol.layers.forEach((l, i) => emitFillLayer(l, rings, levelBase + i, t, env, sink));
  }
}

/** The point halfway along the longest path of a line geometry. */
function lineMiddle(paths: readonly { readonly pts: readonly Vec2[]; readonly closed: boolean }[]): Vec2 | null {
  let best: { pts: readonly Vec2[]; closed: boolean; len: number } | null = null;
  for (const p of paths) {
    let len = 0;
    const n = p.pts.length;
    for (let i = 0; i < (p.closed ? n : n - 1); i++) len += Math.hypot(p.pts[(i + 1) % n].x - p.pts[i].x, p.pts[(i + 1) % n].y - p.pts[i].y);
    if (!best || len > best.len) best = { pts: p.pts, closed: p.closed, len };
  }
  if (!best) return null;
  return placeAlong(best.pts, best.closed, 'center')[0]?.at ?? best.pts[0] ?? null;
}
