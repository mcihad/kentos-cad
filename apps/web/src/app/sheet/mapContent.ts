import type { MapPrim } from '../../contracts/generated/sheet/MapPrim';
import type { Bounds } from '../../model/geometry';
import { buildStyledLayer } from '../../render/styledLayer';
import type { AppContext } from '../context';
import { mapTexts, type VecText } from './mapLabels';
import { labelSpots, paperPalette, shownLayers, styleAt } from './mapFrames';
import { layerPaths, type VecPath } from './mapVectors';

/**
 * A map frame's content as vectors (docs/sheet/design.md §9a,
 * `MapContent::Vector`), one group per drawing layer, bottom first: what the
 * map frames draw (the same styled layers at the map's scale on the paper's
 * palette, mapFrames.ts) as paths, and what they write (mapLabels.ts) as
 * texts, on the ground. Only what reaches the map's box is kept (the core
 * clips to the frame). A layer drawing something with no vector form here is
 * named with what it is: the map then goes as a picture (the export window
 * says so).
 */

export interface VecLayer {
  readonly id: string;
  readonly name: string;
  readonly paths: VecPath[];
  /** Text masks (the paper's colour under a text), drawn before the texts. */
  readonly masks: VecPath[];
  readonly texts: VecText[];
}

export interface VectorMap {
  readonly layers: VecLayer[];
  /** Layers with something that has no vector form: their names and what (empty: the map is all vectors). */
  readonly unsupported: { readonly layer: string; readonly what: readonly string[] }[];
}

/** The ground a map's content covers: the engine's extent, else its centre and size turned (with a margin for symbols). */
function boxOf(prim: MapPrim): Bounds {
  const v = prim.view;
  if (prim.extent) return { minX: prim.extent[0], minY: prim.extent[1], maxX: prim.extent[2], maxY: prim.extent[3] };
  const turn = (v.rotation * Math.PI) / 180_000;
  const w = (prim.clip.width * v.scale) / 1_000_000;
  const h = (prim.clip.height * v.scale) / 1_000_000;
  const hw = (Math.abs(Math.cos(turn)) * w + Math.abs(Math.sin(turn)) * h) / 2;
  const hh = (Math.abs(Math.sin(turn)) * w + Math.abs(Math.cos(turn)) * h) / 2;
  return { minX: v.center!.x - hw, minY: v.center!.y - hh, maxX: v.center!.x + hw, maxY: v.center!.y + hh };
}

/** Whether a part's points reach the box. */
function reaches(points: readonly number[], b: Bounds): boolean {
  let x0 = Infinity;
  let y0 = Infinity;
  let x1 = -Infinity;
  let y1 = -Infinity;
  for (let i = 0; i + 1 < points.length; i += 2) {
    const x = points[i];
    const y = points[i + 1];
    if (x < x0) x0 = x;
    if (x > x1) x1 = x;
    if (y < y0) y0 = y;
    if (y > y1) y1 = y;
  }
  return x1 >= b.minX && x0 <= b.maxX && y1 >= b.minY && y0 <= b.maxY;
}

/** Only the parts of each path that reach the box; a path left with none goes. */
function within(paths: readonly VecPath[], b: Bounds): VecPath[] {
  const out: VecPath[] = [];
  for (const p of paths) {
    const parts = p.parts.filter((x) => reaches(x.points, b));
    if (parts.length) out.push(parts.length === p.parts.length ? p : { ...p, parts });
  }
  return out;
}

let measuring: CanvasRenderingContext2D | null = null;
const measure = (font: string, text: string) => {
  measuring ??= document.createElement('canvas').getContext('2d');
  if (!measuring) return text.length * 6;
  measuring.font = font;
  return measuring.measureText(text).width;
};

/** A map frame's content as vector layers; null for a map with no place. */
export function vectorMap(ctx: AppContext, prim: MapPrim): VectorMap | null {
  const v = prim.view;
  if (!v.center) return null;
  const { doc } = ctx;
  const box = boxOf(prim);
  // A margin round the box: half a symbol or a label past the edge still shows inside.
  const pad = (20 * v.scale) / 1000;
  const reach: Bounds = { minX: box.minX - pad, minY: box.minY - pad, maxX: box.maxX + pad, maxY: box.maxY + pad };
  const palette = paperPalette(ctx);
  const style = styleAt(ctx, v.scale, palette, reach);
  const scale = { scale: v.scale, origin: doc.origin };
  const layers: VecLayer[] = [];
  const unsupported: { layer: string; what: string[] }[] = [];
  const order = shownLayers(ctx, prim.layers);
  for (const id of order) {
    const l = doc.layers.get(id);
    if (!l || l.type !== 'layer') continue;
    const objects = doc.byLayer(id);
    if (!objects.length) continue;
    const shapes = layerPaths(buildStyledLayer(id, objects, l.style, style), scale, reach);
    if (shapes.unsupported.length) unsupported.push({ layer: l.name, what: shapes.unsupported });
    layers.push({ id, name: l.name, paths: within(shapes.paths, reach), masks: [], texts: [] });
  }
  // The texts, at the paper's CSS px per ground metre (96 dpi over the scale), thinned over the map's box as its picture is.
  const pxPerM = 96_000 / (25.4 * v.scale);
  const spots = labelSpots(ctx, prim, box, pxPerM);
  const written = mapTexts({
    doc,
    palette,
    font: doc.settings.drawingFont.value,
    spots,
    pxPerM,
    box: { minX: box.minX, maxY: box.maxY, width: (box.maxX - box.minX) * pxPerM, height: (box.maxY - box.minY) * pxPerM },
    dimensionText: (m, look) => ctx.view.dimensionText(m, look),
    pieces: (b) => ctx.view.geometry.blockPieces(b),
    measure,
  });
  const byId = new Map(layers.map((x) => [x.id, x]));
  for (const t of written.texts) byId.get(t.layer)?.texts.push(t);
  for (const m of written.masks) byId.get(m.layer)?.masks.push(m.path);
  return { layers: layers.filter((x) => x.paths.length || x.texts.length || x.masks.length), unsupported };
}
