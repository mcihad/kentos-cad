import type { Vec2 } from '../model/geometry';
import type { LibraryAsset } from '../model/style';
import { MM_PER_PX } from '../style/compile';
import type { FillPaint, MarkerStyle, PrimitiveSink, PrimUnit, ShapeMarkStyle, StrokeStyle, TileSource } from '../style/primitives';
import { parseHex, resolveColor, type CanvasPalette } from './color';
import { FillQueue } from './fillQueue';
import { MARKER_STRIDE, STROKE_STRIDE, TEXT_BOX, type AtlasImage, type FillPaintBatch, type MarkerLook, type RGBA, type ScaleRange, type ShapeId, type StyledBatch, type TileMark } from './types';

/**
 * Receives the style engine's primitives for one layer and packs them into
 * GPU batches: equal styles share a batch, geometry becomes origin-relative
 * float32 (never absolute coordinates on the GPU), colours are resolved
 * with the theme palette, and images get atlas keys. Fills are triangulated
 * together when the layer is finished (one core call). Batches come out in
 * symbol-level order: by level, and within a level fills, lines, markers,
 * each with its box so a frame can skip what is out of view.
 */

export interface SinkOptions {
  origin: Vec2;
  palette: CanvasPalette;
  /** Denominator of the plot scale, to relate px and paper mm inside pattern tiles. */
  plotScale: number;
  asset(id: string): LibraryAsset | undefined;
}

const ANCHOR: Record<string, readonly [number, number]> = {
  center: [0, 0],
  top: [0, 0.5],
  bottom: [0, -0.5],
  left: [-0.5, 0],
  right: [0.5, 0],
  'top-left': [-0.5, 0.5],
  'top-right': [0.5, 0.5],
  'bottom-left': [-0.5, -0.5],
  'bottom-right': [0.5, -0.5],
};

const KIND_ORDER = { fill: 0, stroke: 1, marker: 2 } as const;

/** Text marker faces; the regulation's legends use Arial, Arial Black and Times (metric twins on Linux). */
const FONT_FAMILY = {
  ui: 'Barlow, "Segoe UI", sans-serif',
  sans: 'Arial, "Liberation Sans", Arimo, Helvetica, sans-serif',
  black: '"Arial Black", "Arial", "Liberation Sans", Arimo, sans-serif',
  narrow: '"Arial Narrow", "Liberation Sans Narrow", Arial, sans-serif',
  serif: '"Times New Roman", "Liberation Serif", Tinos, Times, serif',
  mono: '"IBM Plex Mono", monospace',
} as const;

/** A batch while it is being filled: its data, box and largest marker. */
interface Entry {
  batch: StyledBatch;
  level: number;
  order: number;
  data: number[];
  box: [number, number, number, number];
  /** Largest marker width and height (markers), or half the stroke width (strokes). */
  w: number;
  h: number;
}

const SHAPE_IDS = new Set<string>(['circle', 'ring', 'square', 'rectangle', 'diamond', 'triangle', 'pentagon', 'hexagon', 'octagon', 'star', 'cross', 'x', 'line', 'arrow', 'arrowhead', 'chevron', 'semicircle', 'quartercircle', 'gear', 'arc']);
const shapeId = (s: string): ShapeId => (SHAPE_IDS.has(s) ? (s as ShapeId) : 'circle');
const OPEN_SHAPES = new Set<string>(['cross', 'x', 'line', 'arrow', 'chevron']);

/** Share of its box a shape covers when filled (for the far-zoom tint of patterns). */
const FILLED_SHARE: Record<string, number> = { circle: 0.785, ring: 0.785, square: 1, rectangle: 1, diamond: 0.5, triangle: 0.43, pentagon: 0.6, hexagon: 0.65, octagon: 0.8, star: 0.35, semicircle: 0.39, quartercircle: 0.2, arrowhead: 0.4, gear: 0.7 };

/** How much of a cell a pattern shape inks, 0–1. */
function patternTint(m: ShapeMarkStyle, cellArea: number): number {
  const w = m.size;
  const h = m.height || m.size;
  let ink = 0;
  if (m.fill && !OPEN_SHAPES.has(m.shape)) ink = w * h * (FILLED_SHARE[m.shape] ?? 0.6) * (1 - m.params[0] * m.params[0]);
  else if (m.stroke || OPEN_SHAPES.has(m.shape)) ink = 3.2 * Math.max(w, h) * Math.max(m.strokeWidth, 0.05 * Math.max(w, h));
  return Math.min(1, ink / Math.max(cellArea, 1e-12));
}

/** SVG colours given by the symbol: param(fill) / param(stroke) (optionally with a default after them), and currentColor. */
export function applySvgParams(svg: string, fill: string | null, stroke: string | null): string {
  return svg
    .replace(/param\(\s*(fill|stroke)\s*\)(\s+#[0-9a-f]{3,8})?/gi, (_m, which: string, def?: string) => (which.toLowerCase() === 'fill' ? fill : stroke) ?? def?.trim() ?? '#000000')
    .replace(/currentColor/g, fill ?? stroke ?? '#000000');
}

export class StyledSink implements PrimitiveSink {
  private readonly opts: SinkOptions;
  private readonly entries = new Map<string, Entry>();
  /** Batch keys by style object (markers along a line share one object). */
  private readonly keys = new WeakMap<object, string>();
  private scale: ScaleRange = {};
  private seq = 0;
  private readonly fills = new FillQueue();

  constructor(opts: SinkOptions) {
    this.opts = opts;
  }

  /** Scale range of what follows (a rule's range), until changed. */
  setScale(range: ScaleRange): void {
    this.scale = range;
  }

  private rgba(color: string, opacity: number): RGBA {
    const c = parseHex(resolveColor(color, this.opts.palette));
    return [c[0], c[1], c[2], c[3] * opacity];
  }

  private entry(key: string, level: number, make: () => StyledBatch): Entry {
    const k = `${key}|${this.scale.minScale ?? ''}|${this.scale.maxScale ?? ''}`;
    let e = this.entries.get(k);
    if (!e) this.entries.set(k, (e = { batch: { ...make(), ...this.scale }, level, order: this.seq++, data: [], box: [Infinity, Infinity, -Infinity, -Infinity], w: 0, h: 0 }));
    return e;
  }

  private grow(e: Entry, x: number, y: number): void {
    const b = e.box;
    if (x < b[0]) b[0] = x;
    if (y < b[1]) b[1] = y;
    if (x > b[2]) b[2] = x;
    if (y > b[3]) b[3] = y;
  }

  // ── PrimitiveSink ────────────────────────────────────────────────────

  stroke(style: StrokeStyle, path: readonly Vec2[], closed: boolean): void {
    const { origin } = this.opts;
    const e = this.entry(`s|${JSON.stringify(style)}`, style.level, () => ({
      kind: 'stroke',
      segments: new Float32Array(0),
      color: this.rgba(style.color, style.opacity),
      width: style.width,
      unit: style.unit,
      dash: style.dash ? style.dash.slice(0, 8) : null,
      dashOffset: style.dashOffset,
      cap: style.cap,
      blur: style.blur,
      bounds: [0, 0, 0, 0],
      reach: 0,
      reachUnit: style.unit,
    }));
    e.w = Math.max(e.w, style.width / 2 + style.blur);
    const n = path.length;
    const count = closed ? n : n - 1;
    let d = 0;
    for (let i = 0; i < count; i++) {
      const a = path[i];
      const b = path[(i + 1) % n];
      const len = Math.hypot(b.x - a.x, b.y - a.y);
      if (len < 1e-12) continue;
      const ends = (!closed && i === 0 ? 1 : 0) | (!closed && i === count - 1 ? 2 : 0);
      const ax = a.x - origin.x;
      const ay = a.y - origin.y;
      e.data.push(ax, ay, b.x - origin.x, b.y - origin.y, d, ends);
      this.grow(e, ax, ay);
      d += len;
    }
    // The last point of an open path is no segment's start.
    if (n) this.grow(e, path[closed ? 0 : n - 1].x - origin.x, path[closed ? 0 : n - 1].y - origin.y);
  }

  fill(paint: FillPaint, rings: readonly (readonly Vec2[])[]): void {
    if (!rings.length || rings[0].length < 3) return;
    const e = this.entry(`f|${JSON.stringify(paint)}`, paint.level, () => ({ kind: 'fill', positions: new Float32Array(0), paint: this.paint(paint), bounds: [0, 0, 0, 0], reach: 0, reachUnit: 'world' }));
    const { origin } = this.opts;
    for (const p of rings[0]) this.grow(e, p.x - origin.x, p.y - origin.y);
    this.fills.add(e.data, rings);
  }

  marker(style: MarkerStyle, at: Vec2, angle: number): void {
    let key = this.keys.get(style);
    if (!key) {
      // Size and rotation vary per object (data-defined) without splitting the batch.
      const { size: _s, ...rest } = style as MarkerStyle & { height?: number };
      key = `m|${JSON.stringify({ ...rest, height: undefined, common: { ...style.common, rotation: 0 } })}`;
      this.keys.set(style, key);
    }
    const e = this.entry(key, style.common.level, () => ({
      kind: 'marker',
      instances: new Float32Array(0),
      unit: style.common.unit,
      look: this.look(style),
      offset: style.common.offset,
      anchor: ANCHOR[style.common.anchor] ?? [0, 0],
      opacity: style.common.opacity,
      extent: [0, 0],
      bounds: [0, 0, 0, 0],
      reach: 0,
      reachUnit: style.common.unit,
    }));
    const { origin } = this.opts;
    const w = style.kind === 'text' ? 0 : style.size;
    const h = style.kind === 'shape' ? style.height : style.kind === 'text' ? style.size * TEXT_BOX : 0;
    const x = at.x - origin.x;
    const y = at.y - origin.y;
    e.data.push(x, y, angle + style.common.rotation, w, h);
    this.grow(e, x, y);
    if (w > e.w) e.w = w;
    if (h > e.h) e.h = h;
  }

  // ── Looks and paints ─────────────────────────────────────────────────

  private look(style: MarkerStyle): MarkerLook {
    const op = style.common.opacity;
    switch (style.kind) {
      case 'shape':
        return { kind: 'shape', shape: shapeId(style.shape), fill: style.fill ? this.rgba(style.fill, 1) : null, stroke: style.stroke ? this.rgba(style.stroke, 1) : null, strokeWidth: style.strokeWidth, params: style.params };
      case 'text': {
        const color = resolveColor(style.color, this.opts.palette);
        const halo = style.halo ? { color: resolveColor(style.halo.color, this.opts.palette), width: style.halo.width / Math.max(style.size, 1e-9) } : null;
        const font = FONT_FAMILY[style.font === 'sans' && style.weight >= 900 ? 'black' : style.font];
        const key = `t|${style.text}|${style.font}|${style.weight}|${style.italic}|${color}|${halo ? `${halo.color}/${halo.width.toFixed(3)}` : ''}`;
        return { kind: 'image', image: { key, kind: 'text', text: style.text, font, weight: style.weight, italic: style.italic, color, halo }, fit: 'height' };
      }
      case 'svg':
      case 'raster':
        return { kind: 'image', image: this.assetImage(style.kind === 'svg' ? style : { ...style, fill: null, stroke: null }, op), fit: 'width' };
    }
  }

  private assetImage(s: { asset: string; fill: string | null; stroke: string | null }, _opacity: number): AtlasImage {
    const a = this.opts.asset(s.asset);
    if (!a) return { key: `missing|${s.asset}`, kind: 'svg', svg: MISSING_SVG, width: 24, height: 24 };
    if (a.format === 'svg') {
      const fill = s.fill ? resolveColor(s.fill, this.opts.palette) : null;
      const stroke = s.stroke ? resolveColor(s.stroke, this.opts.palette) : null;
      return { key: `svg|${a.id}|${fill}|${stroke}|${a.data.length}`, kind: 'svg', svg: applySvgParams(a.data, fill, stroke), width: a.width, height: a.height };
    }
    return { key: `img|${a.id}|${a.data.length}`, kind: 'raster', url: a.data, width: a.width, height: a.height };
  }

  private paint(p: FillPaint): FillPaintBatch {
    switch (p.kind) {
      case 'solid':
        return { kind: 'solid', color: this.rgba(p.color, p.opacity) };
      case 'hatch':
        return { kind: 'hatch', color: this.rgba(p.color, p.opacity), angle: p.angle, spacing: p.spacing, width: p.width, offset: p.offset, dash: p.dash ? p.dash.slice(0, 8) : null, dashOffset: p.dashOffset, unit: p.unit };
      case 'pattern': {
        const m = p.mark;
        const size: [number, number] = [p.size[0], p.size[1]];
        return {
          kind: 'pattern',
          shape: shapeId(m.shape),
          fill: m.fill ? this.rgba(m.fill, 1) : null,
          stroke: m.stroke ? this.rgba(m.stroke, 1) : null,
          strokeWidth: m.strokeWidth,
          half: [m.size / 2, (m.height || m.size) / 2],
          markOffset: m.common.offset,
          markRotation: m.common.rotation,
          params: m.params,
          size,
          stagger: p.stagger,
          angle: p.angle,
          offset: p.offset,
          jitter: p.jitter,
          coverage: p.coverage,
          seed: p.seed,
          tint: patternTint(m, size[0] * size[1]) * p.coverage,
          opacity: p.opacity * m.common.opacity,
          unit: p.unit,
        };
      }
      case 'tile': {
        const stagger = p.tile.kind === 'markers' && p.tile.stagger;
        const size: [number, number] = [p.size[0], p.size[1] * (stagger ? 2 : 1)];
        return { kind: 'tile', image: this.tileImage(p.tile, p.size, p.unit), size, angle: p.angle, offset: p.offset, opacity: p.opacity, unit: p.unit };
      }
    }
  }

  /** A pattern tile: marker looks placed in a cell, sized relative to the tile width. */
  private tileImage(tile: TileSource, size: readonly [number, number], unit: PrimUnit): AtlasImage {
    if (tile.kind === 'asset') return this.assetImage({ asset: tile.asset, fill: null, stroke: null }, 1);
    const worldPerPx = (MM_PER_PX * this.opts.plotScale) / 1000;
    const inTileUnit = (v: number, u: PrimUnit) => (u === unit ? v : u === 'px' ? v * worldPerPx : v / worldPerPx);
    const tw = size[0];
    const draw: TileMark[] = tile.markers.map((m) => {
      const w = inTileUnit(m.kind === 'text' ? m.size : m.size, m.common.unit) / tw;
      const h = inTileUnit(m.kind === 'shape' ? m.height : m.size, m.common.unit) / tw;
      const look = this.look(m);
      const lookScaled = look.kind === 'shape' ? { ...look, strokeWidth: inTileUnit(look.strokeWidth, m.common.unit) / tw } : look;
      return { look: lookScaled, w, h, offset: [inTileUnit(m.common.offset[0], m.common.unit) / tw, inTileUnit(m.common.offset[1], m.common.unit) / tw], rotation: m.common.rotation };
    });
    // A staggered tile holds two rows (the second half-shifted).
    const aspect = (size[1] / size[0]) * (tile.stagger ? 2 : 1);
    return { key: `tile|${JSON.stringify(draw)}|${aspect.toFixed(4)}|${tile.stagger}`, kind: 'tile', aspect, stagger: tile.stagger, draw };
  }

  /** The batches, in draw order. */
  finish(): StyledBatch[] {
    this.fills.run(this.opts.origin);
    const list = [...this.entries.values()].sort((a, b) => a.level - b.level || KIND_ORDER[a.batch.kind] - KIND_ORDER[b.batch.kind] || a.order - b.order);
    return list.flatMap((e): StyledBatch[] => {
      const data = new Float32Array(e.data);
      const b = e.batch;
      const bounds = e.box;
      if (b.kind === 'stroke') return data.length >= STROKE_STRIDE ? [{ ...b, segments: data, bounds, reach: e.w + 1 }] : [];
      if (b.kind === 'fill') return data.length >= 6 ? [{ ...b, positions: data, bounds }] : [];
      if (data.length < MARKER_STRIDE) return [];
      // A text marker is as wide as its text; an image as its proportions say.
      let w = e.w;
      if (b.look.kind === 'image') {
        const img = b.look.image;
        if (img.kind === 'text') w = Math.max(w, (e.h / TEXT_BOX) * 0.62 * img.text.length);
        else if (img.kind === 'svg' || img.kind === 'raster') w = Math.max(w, (e.h || e.w) * (img.width / Math.max(img.height, 1e-9)));
      }
      const reach = Math.max(w, e.h, e.w) * 1.5 + Math.hypot(b.offset[0], b.offset[1]) + 2;
      return [{ ...b, instances: data, bounds, reach, extent: [e.w, e.h] }];
    });
  }
}

/** Drawn where a symbol's SVG asset is missing from the library: a crossed box. */
const MISSING_SVG = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24"><rect x="2" y="2" width="20" height="20" fill="none" stroke="#E0457B" stroke-width="2"/><path d="M4 4L20 20M20 4L4 20" stroke="#E0457B" stroke-width="2"/></svg>';
