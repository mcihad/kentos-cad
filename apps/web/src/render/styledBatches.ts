import type { LibraryAsset } from '../model/style';
import { MM_PER_PX, type FillPaint, type MarkerStyle, type PrimUnit, type ShapeMarkStyle, type StrokeStyle, type TileSource } from '../style/primitives';
import { parseHex, resolveColor, type CanvasPalette } from './color';
import { TEXT_BOX, type AtlasImage, type FillPaintBatch, type MarkerLook, type RGBA, type ShapeId, type StyledBatch, type TileMark } from './types';

/**
 * The page's half of a styled layer (docs/STYLE.md §6): the style core
 * (crates/shared/style-core/src/style/batch.rs) merges equal styles, packs
 * the geometry origin-relative into float32 and orders the batches by
 * symbol level; here each batch gets its colours from the theme palette,
 * its images atlas keys, and how far it reaches past its geometry.
 */

export interface BatchOptions {
  palette: CanvasPalette;
  /** Denominator of the plot scale, to relate px and paper mm inside pattern tiles. */
  plotScale: number;
  asset(id: string): LibraryAsset | undefined;
}

/** One batch as the core describes it: the first style it was made for, its numbers and extent. */
type Described =
  | { kind: 'stroke'; style: StrokeStyle; minScale?: number; maxScale?: number; from: number; len: number; bounds: [number, number, number, number]; w: number; h: number }
  | { kind: 'fill'; style: FillPaint; minScale?: number; maxScale?: number; from: number; len: number; bounds: [number, number, number, number]; w: number; h: number }
  | { kind: 'marker'; style: MarkerStyle; minScale?: number; maxScale?: number; from: number; len: number; bounds: [number, number, number, number]; w: number; h: number };

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

/** Text marker faces; the regulation's legends use Arial, Arial Black and Times (metric twins on Linux). */
const FONT_FAMILY = {
  ui: 'Barlow, "Segoe UI", sans-serif',
  sans: 'Arial, "Liberation Sans", Arimo, Helvetica, sans-serif',
  black: '"Arial Black", "Arial", "Liberation Sans", Arimo, sans-serif',
  narrow: '"Arial Narrow", "Liberation Sans Narrow", Arial, sans-serif',
  serif: '"Times New Roman", "Liberation Serif", Tinos, Times, serif',
  mono: '"IBM Plex Mono", monospace',
} as const;

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

/** Drawn where a symbol's SVG asset is missing from the library: a crossed box. */
const MISSING_SVG = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24"><rect x="2" y="2" width="20" height="20" fill="none" stroke="#E0457B" stroke-width="2"/><path d="M4 4L20 20M20 4L4 20" stroke="#E0457B" stroke-width="2"/></svg>';

class Looks {
  private readonly opts: BatchOptions;

  constructor(opts: BatchOptions) {
    this.opts = opts;
  }

  rgba(color: string, opacity: number): RGBA {
    const c = parseHex(resolveColor(color, this.opts.palette));
    return [c[0], c[1], c[2], c[3] * opacity];
  }

  look(style: MarkerStyle): MarkerLook {
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
        return { kind: 'image', image: this.assetImage(style.kind === 'svg' ? style : { ...style, fill: null, stroke: null }), fit: 'width' };
    }
  }

  private assetImage(s: { asset: string; fill: string | null; stroke: string | null }): AtlasImage {
    const a = this.opts.asset(s.asset);
    if (!a) return { key: `missing|${s.asset}`, kind: 'svg', svg: MISSING_SVG, width: 24, height: 24 };
    if (a.format === 'svg') {
      const fill = s.fill ? resolveColor(s.fill, this.opts.palette) : null;
      const stroke = s.stroke ? resolveColor(s.stroke, this.opts.palette) : null;
      return { key: `svg|${a.id}|${fill}|${stroke}|${a.data.length}`, kind: 'svg', svg: applySvgParams(a.data, fill, stroke), width: a.width, height: a.height };
    }
    return { key: `img|${a.id}|${a.data.length}`, kind: 'raster', url: a.data, width: a.width, height: a.height };
  }

  paint(p: FillPaint): FillPaintBatch {
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
    if (tile.kind === 'asset') return this.assetImage({ asset: tile.asset, fill: null, stroke: null });
    const worldPerPx = (MM_PER_PX * this.opts.plotScale) / 1000;
    const inTileUnit = (v: number, u: PrimUnit) => (u === unit ? v : u === 'px' ? v * worldPerPx : v / worldPerPx);
    const tw = size[0];
    const draw: TileMark[] = tile.markers.map((m) => {
      const w = inTileUnit(m.size, m.common.unit) / tw;
      const h = inTileUnit(m.kind === 'shape' ? m.height : m.size, m.common.unit) / tw;
      const look = this.look(m);
      const lookScaled = look.kind === 'shape' ? { ...look, strokeWidth: inTileUnit(look.strokeWidth, m.common.unit) / tw } : look;
      return { look: lookScaled, w, h, offset: [inTileUnit(m.common.offset[0], m.common.unit) / tw, inTileUnit(m.common.offset[1], m.common.unit) / tw], rotation: m.common.rotation };
    });
    // A staggered tile holds two rows (the second half-shifted).
    const aspect = (size[1] / size[0]) * (tile.stagger ? 2 : 1);
    return { key: `tile|${JSON.stringify(draw)}|${aspect.toFixed(4)}|${tile.stagger}`, kind: 'tile', aspect, stagger: tile.stagger, draw };
  }
}

/** A batch's scale range, only where the rule gave one. */
const scaleOf = (d: Described) => ({ ...(d.minScale !== undefined ? { minScale: d.minScale } : {}), ...(d.maxScale !== undefined ? { maxScale: d.maxScale } : {}) });

/** The core's batches of a layer as GPU batches, in the core's (draw) order. */
export function styledBatches(json: string, data: Float32Array, opts: BatchOptions): StyledBatch[] {
  const looks = new Looks(opts);
  return (JSON.parse(json) as Described[]).map((d): StyledBatch => {
    const nums = data.subarray(d.from, d.from + d.len);
    switch (d.kind) {
      case 'stroke': {
        const s = d.style;
        return { kind: 'stroke', segments: nums, color: looks.rgba(s.color, s.opacity), width: s.width, unit: s.unit, dash: s.dash ? s.dash.slice(0, 8) : null, dashOffset: s.dashOffset, cap: s.cap, blur: s.blur, bounds: d.bounds, reach: d.w + 1, reachUnit: s.unit, ...scaleOf(d) };
      }
      case 'fill':
        return { kind: 'fill', positions: nums, paint: looks.paint(d.style), bounds: d.bounds, reach: 0, reachUnit: 'world', ...scaleOf(d) };
      case 'marker': {
        const s = d.style;
        const look = looks.look(s);
        // A text marker is as wide as its text; an image as its proportions say.
        let w = d.w;
        if (look.kind === 'image') {
          const img = look.image;
          if (img.kind === 'text') w = Math.max(w, (d.h / TEXT_BOX) * 0.62 * img.text.length);
          else if (img.kind === 'svg' || img.kind === 'raster') w = Math.max(w, (d.h || d.w) * (img.width / Math.max(img.height, 1e-9)));
        }
        const reach = Math.max(w, d.h, d.w) * 1.5 + Math.hypot(s.common.offset[0], s.common.offset[1]) + 2;
        return {
          kind: 'marker',
          instances: nums,
          unit: s.common.unit,
          look,
          offset: s.common.offset,
          anchor: ANCHOR[s.common.anchor] ?? [0, 0],
          opacity: s.common.opacity,
          extent: [d.w, d.h],
          bounds: d.bounds,
          reach,
          reachUnit: s.common.unit,
          ...scaleOf(d),
        };
      }
    }
  });
}
