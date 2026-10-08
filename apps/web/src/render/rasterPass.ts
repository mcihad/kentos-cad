import { rasterLevelCount, rasterTiles } from '../wasm/core';
import type { AtlasSource, FillPaintBatch } from './types';

/**
 * Rasters' tiles on the web's GPU (docs/adr/0204 §5), the CPU half both
 * backends share, twin of crates/render/wgpu/src/styled/raster_tiles.rs: one
 * atlas page of 258-pixel slots (a tile and a pixel of its neighbours round
 * it, so bilinear sampling shows no seam), the least recently drawn let go
 * first; each frame, each raster batch in view picks the tiles of the level
 * whose pixel is about a device pixel (the geometry core's `level_for` and
 * `visible` through WASM, the view's centre first), draws those the atlas
 * holds and, for the others, the part of the nearest coarser tile it holds
 * while the workers make them. A raster's quads go into its own reused
 * array: it is one draw call. The backend owns the texture: `put` writes a
 * tile's colours into a slot.
 */

/** A slot's side: a tile and a pixel round it. */
export const SLOT = 258;
const TILE = 256;
/** At most this many tiles go to the GPU in a frame (8.5 MB); the rest the next. */
export const UPLOADS_PER_FRAME = 32;
/** Numbers a quad vertex takes: x, y (from the batch's tile), u, v. */
export const RASTER_STRIDE = 4;

export type RasterPaint = Extract<FillPaintBatch, { kind: 'raster' }>;

/** The atlas page's size: 8192 × 4096 where the device allows (465 slots), else 4096 × 4096 (225). */
export function rasterAtlasSize(maxTexture: number): [number, number] {
  return maxTexture >= 8192 ? [8192, 4096] : [4096, 4096];
}

/** Which tile each slot holds, and when it was drawn last. */
export class RasterSlots {
  readonly width: number;
  readonly height: number;
  private readonly perRow: number;
  private readonly held = new Map<string, number>();
  private readonly owner: (string | null)[];
  private readonly used: number[];
  private frame = 0;

  constructor(width: number, height: number) {
    this.width = width;
    this.height = height;
    this.perRow = Math.floor(width / SLOT);
    const n = this.perRow * Math.floor(height / SLOT);
    this.owner = new Array<string | null>(n).fill(null);
    this.used = new Array<number>(n).fill(0);
  }

  beginFrame(): void {
    this.frame++;
  }

  /** A slot's upper left pixel. */
  origin(slot: number): [number, number] {
    return [(slot % this.perRow) * SLOT, Math.floor(slot / this.perRow) * SLOT];
  }

  /** The atlas rectangle of a slot's tile pixels (x, y) to (x + w, y + h), its apron left round them. */
  uv(slot: number, x: number, y: number, w: number, h: number): [number, number, number, number] {
    const [ox, oy] = this.origin(slot);
    const u0 = (ox + 1 + x) / this.width;
    const v0 = (oy + 1 + y) / this.height;
    return [u0, v0, u0 + w / this.width, v0 + h / this.height];
  }

  get(id: string): number | undefined {
    return this.held.get(id);
  }

  touch(slot: number): void {
    this.used[slot] = this.frame;
  }

  /** A slot for a new tile: a free one, else the one drawn longest ago, never one drawn this frame; null when all are. */
  take(id: string): number | null {
    let slot = this.owner.indexOf(null);
    if (slot < 0) {
      let at = Infinity;
      for (let i = 0; i < this.used.length; i++)
        if (this.used[i] < at) {
          at = this.used[i];
          slot = i;
        }
      if (slot < 0 || at === this.frame) return null;
      const old = this.owner[slot];
      if (old !== null) this.held.delete(old);
    }
    this.owner[slot] = id;
    this.held.set(id, slot);
    this.touch(slot);
    return slot;
  }

  /** Every slot freed (the texture was made again). */
  clear(): void {
    this.held.clear();
    this.owner.fill(null);
  }
}

/** A raster's name in the atlas: its file, look and pixel's size and turn (a shaded relief's slopes are by them). */
export function rasterPaintKey(p: RasterPaint): string {
  const [, a, b, , c, d] = p.affine;
  return `${p.raster}|${p.look}|${a},${b},${c},${d}`;
}

/** A frame's quads of one raster, grown as needed and reused. */
export class RasterQuads {
  data = new Float32Array(6 * RASTER_STRIDE * 64);
  /** Vertices this frame. */
  count = 0;

  private ensure(n: number): void {
    if (n <= this.data.length) return;
    let size = this.data.length;
    while (size < n) size *= 2;
    const grown = new Float32Array(size);
    grown.set(this.data.subarray(0, this.count * RASTER_STRIDE));
    this.data = grown;
  }

  push(x: number, y: number, u: number, v: number): void {
    this.ensure((this.count + 1) * RASTER_STRIDE);
    this.data.set([x, y, u, v], this.count * RASTER_STRIDE);
    this.count++;
  }
}

/**
 * A raster batch's quads this frame into `out`: the tiles `view` (origin-relative min x, min y, max x, max y) meets
 * at `pxPerM` device pixels a metre, each from the atlas or, while it is made, from the nearest coarser tile there;
 * new tiles written by `put` (at most `budget.left` a frame). `origin` is the batch's tile, which the paint's
 * affine and the quads' positions are from.
 */
export function rasterQuads(
  paint: RasterPaint,
  origin: readonly [number, number],
  view: readonly [number, number, number, number],
  pxPerM: number,
  slots: RasterSlots,
  atlas: AtlasSource,
  put: (slot: number, rgba: Uint8Array) => void,
  budget: { left: number },
  out: RasterQuads,
): void {
  out.count = 0;
  const [x0, a, b, y0, c, d] = paint.affine;
  const [w, h] = paint.size;
  const key = rasterPaintKey(paint);
  const levels = rasterLevelCount(w, h);
  // The affine from the layers' origin, as the view is. The tiles are in the core's memory: no call into the core
  // below, which could move it.
  const n = rasterTiles(x0 + origin[0], a, b, y0 + origin[1], c, d, w, h, view, pxPerM);
  const tiles = n.numbers;
  for (let k = 0; k < n.count; k++) {
    const t = tiles.subarray(13 * k, 13 * k + 13);
    const [level, tx, ty, su, sv] = [t[0], t[1], t[2], t[3], t[4]];
    const id = `${key}|${level}|${tx}|${ty}`;
    let slot = slots.get(id);
    if (slot === undefined) {
      const rgba = atlas.rasterTile(paint.raster, paint.url, paint.look, paint.affine, level, tx, ty);
      if (rgba && budget.left > 0) {
        const s = slots.take(id);
        if (s !== null) {
          put(s, rgba);
          budget.left--;
          slot = s;
        }
      }
    }
    let rect: [number, number, number, number] | null = null;
    const [tw, th] = [su * TILE, sv * TILE];
    if (slot !== undefined) {
      slots.touch(slot);
      rect = slots.uv(slot, 0, 0, tw, th);
    } else {
      // The nearest coarser tile the atlas holds, the part over this one.
      for (let up = 1; level + up < levels; up++) {
        const parent = slots.get(`${key}|${level + up}|${tx >> up}|${ty >> up}`);
        if (parent === undefined) continue;
        const f = 2 ** up;
        const span = TILE / f;
        slots.touch(parent);
        rect = slots.uv(parent, (tx & (f - 1)) * span, (ty & (f - 1)) * span, tw / f, th / f);
        break;
      }
    }
    if (!rect) continue;
    const [u0, v0, u1, v1] = rect;
    // Corners: (x₀, y₀) the tile's first pixel, then along the row, then down; from the batch's tile.
    const px = (i: number) => t[5 + i] - origin[0];
    const py = (i: number) => t[9 + i] - origin[1];
    out.push(px(0), py(0), u0, v0);
    out.push(px(1), py(1), u1, v0);
    out.push(px(2), py(2), u1, v1);
    out.push(px(0), py(0), u0, v0);
    out.push(px(2), py(2), u1, v1);
    out.push(px(3), py(3), u0, v1);
  }
}
