import type { RasterQuads, RasterSlots } from './rasterPass';
import type { FillPaintBatch, StyledBatch } from './types';

/**
 * Map services' tiles on the web's GPU (docs/adr/0208 §3, §9), the CPU half both backends share, twin of
 * crates/render/wgpu/src/styled/service_tiles.rs. A service layer is one batch (`paint.kind: 'service'`) drawn in the
 * layer tree's place. Each frame it shows the tiles of the level whose pixel is nearest a device pixel (the geometry
 * core's `shown` through the services module, the view's centre first). A picture tile is in the raster atlas in
 * 256-pixel slots (a 512-pixel tile takes four) and is drawn as a mesh: its box divided n × n in the service's system,
 * each node in the project's (one cell when the two systems are the same); a tile not here yet shows the part of the
 * nearest coarser tile the atlas holds. The quads are from the view's own position tile (65 536 m,
 * docs/adr/0157), written each frame, so a basemap far from the drawing keeps its digits. A vector tile is a styled
 * layer of its own drawn where the batch is (the backends').
 */

export type ServicePaint = Extract<FillPaintBatch, { kind: 'service' }>;

/** A service's tiles as the hub knows them (the services module's `View`). */
export interface ServiceGrid {
  /** Its tiles in the view `[x₁, y₁, x₂, y₂]` (the project's system): `[level, unitsPerPx, box ×4, centre ×2, col, row, …]`. */
  tiles(x1: number, y1: number, x2: number, y2: number, pxPerUnit: number): Float64Array;
  /** A tile's mesh: `(n + 1)²` nodes in the project's system, row by row from its top left (NaN: no place). */
  mesh(level: number, col: number, row: number): Float64Array;
  /** How finely a tile at `level` is divided. */
  cells(level: number): number;
  displayZoom(unitsPerPx: number): number;
  readonly across: number;
  readonly down: number;
  readonly quadtree: boolean;
  readonly vector: boolean;
}

/** A picture tile's slots: `across` × `down`, row by row, each 258 × 258 premultiplied RGBA. */
export interface ServicePicture {
  readonly across: number;
  readonly down: number;
  readonly slots: readonly Uint8Array[];
}

/** A vector tile built: its batches and an id the backends keep its buffers under. */
export interface ServiceVector {
  readonly id: string;
  readonly batches: readonly StyledBatch[];
}

/** What the backends ask the services for each frame (render/serviceHub.ts). */
export interface ServiceSource {
  /** A frame begins: requests not asked for again soon are let go. */
  frame(): void;
  /** A service's grid, when it is ready. */
  grid(key: string): ServiceGrid | null;
  /** A picture tile when here (`'empty'`: nothing to draw there); asked for when not (null). */
  tile(key: string, level: number, col: number, row: number): ServicePicture | 'empty' | null;
  /** A vector tile built for `zoom`, an earlier build meanwhile; asked for when not (null). */
  vector(key: string, level: number, col: number, row: number, zoom: number): ServiceVector | null;
}

/** The side of the drawing's position tiles, metres (docs/adr/0157). */
const POSITION_TILE = 65536;

/** The position tile `p` (metres from the drawing's anchor) is in. */
export function positionTile(x: number, y: number): [number, number] {
  const t = (v: number) => {
    const i = Math.round(v / POSITION_TILE) * POSITION_TILE;
    return Number.isFinite(i) ? i : 0;
  };
  return [t(x), t(y)];
}

/** Meshes worked out lately, by service, tile and cells: a pan over the same tiles reads them. */
export class ServiceMeshes {
  private readonly held = new Map<string, { nodes: Float64Array; at: number }>();
  private frame = 0;

  beginFrame(): void {
    this.frame++;
    if (this.held.size > 4096) for (const [k, m] of this.held) if (m.at < this.frame - 4) this.held.delete(k);
  }

  get(key: string, grid: ServiceGrid, level: number, col: number, row: number): Float64Array {
    const id = `${key}|${level}|${col}|${row}`;
    let m = this.held.get(id);
    if (!m) {
      m = { nodes: grid.mesh(level, col, row), at: this.frame };
      this.held.set(id, m);
    }
    m.at = this.frame;
    return m.nodes;
  }

  /** A service's meshes forgotten (its view changed). */
  forget(key: string): void {
    for (const k of this.held.keys()) if (k.startsWith(`${key}|`)) this.held.delete(k);
  }
}

/** A tile's slots this frame: each slot's atlas rectangle (`u₀, v₀, u₁, v₁`), or the parent's part over it. */
type Rects = ([number, number, number, number] | null)[];

/**
 * A service's picture tiles in view as quads into `out`: `view` is the frame's box from the drawing's anchor
 * (`anchor` the anchor in the project's system), `pxPerUnit` device pixels a unit; the quads are from `origin` (a
 * position tile, from the anchor). New slots go to the atlas by `put` (at most `budget.left` a frame).
 */
export function serviceQuads(
  paint: ServicePaint,
  origin: readonly [number, number],
  view: readonly [number, number, number, number],
  pxPerUnit: number,
  slots: RasterSlots,
  source: ServiceSource,
  meshes: ServiceMeshes,
  put: (slot: number, rgba: Uint8Array) => void,
  budget: { left: number },
  out: RasterQuads,
): void {
  out.count = 0;
  const grid = source.grid(paint.service);
  if (!grid || grid.vector) return;
  const [ax, ay] = paint.anchor;
  const seen = grid.tiles(view[0] + ax, view[1] + ay, view[2] + ax, view[3] + ay, pxPerUnit);
  if (seen.length < 8) return;
  const level = seen[0];
  const across = Math.max(1, grid.across);
  const down = Math.max(1, grid.down);
  const n = Math.max(1, grid.cells(level));
  const kx = n / across;
  const ky = n / down;
  const slotId = (lv: number, col: number, row: number, sx: number, sy: number) => `${paint.service}|${lv}|${col}|${row}|${sx}|${sy}`;
  const full = (slot: number) => slots.uv(slot, 0, 0, 256, 256);
  for (let k = 8; k + 1 < seen.length; k += 2) {
    const col = seen[k];
    const row = seen[k + 1];
    const tile = source.tile(paint.service, level, col, row);
    if (tile === 'empty') continue;
    const rects: Rects = [];
    let any = false;
    if (tile) {
      for (let sy = 0; sy < down; sy++)
        for (let sx = 0; sx < across; sx++) {
          const id = slotId(level, col, row, sx, sy);
          let slot = slots.get(id);
          if (slot === undefined && budget.left > 0) {
            const s = slots.take(id);
            const rgba = tile.slots[sy * tile.across + sx];
            if (s !== null && rgba) {
              put(s, rgba);
              budget.left--;
              slot = s;
            }
          }
          if (slot !== undefined) {
            slots.touch(slot);
            rects.push(full(slot));
            any = true;
          } else rects.push(null);
        }
    }
    // Missing slots: the nearest coarser tile's part, where the grid is a quadtree.
    const parentRect = (fu0: number, fv0: number, fu1: number, fv1: number): [number, number, number, number] | null => {
      if (!grid.quadtree) return null;
      let [pc, pr, pl] = [col, row, level];
      let [u0, v0, u1, v1] = [fu0, fv0, fu1, fv1];
      for (let up = 1; up <= 8 && pl > 0; up++) {
        // The child's fractions in its parent.
        const cu = pc % 2;
        const cv = pr % 2;
        [u0, v0, u1, v1] = [(cu + u0) / 2, (cv + v0) / 2, (cu + u1) / 2, (cv + v1) / 2];
        pc = Math.floor(pc / 2);
        pr = Math.floor(pr / 2);
        pl--;
        const sx = Math.min(across - 1, Math.floor(u0 * across + 1e-9));
        const sy = Math.min(down - 1, Math.floor(v0 * down + 1e-9));
        const slot = slots.get(slotId(pl, pc, pr, sx, sy));
        if (slot === undefined) continue;
        slots.touch(slot);
        // The part, in the slot's own fractions.
        const [U0, V0, U1, V1] = full(slot);
        const lu = (f: number) => U0 + (U1 - U0) * (f * across - sx);
        const lv = (f: number) => V0 + (V1 - V0) * (f * down - sy);
        return [lu(u0), lv(v0), lu(u1), lv(v1)];
      }
      return null;
    };
    if (!any && !grid.quadtree) continue;
    const nodes = meshes.get(paint.service, grid, level, col, row);
    const w = n + 1;
    const at = (i: number, j: number): [number, number] => [nodes[(j * w + i) * 2] - origin[0] - ax, nodes[(j * w + i) * 2 + 1] - origin[1] - ay];
    for (let j = 0; j < n; j++)
      for (let i = 0; i < n; i++) {
        const sx = Math.floor(i / kx);
        const sy = Math.floor(j / ky);
        let rect = rects[sy * across + sx] ?? null;
        let [lu0, lv0, lu1, lv1] = [(i - sx * kx) / kx, (j - sy * ky) / ky, (i + 1 - sx * kx) / kx, (j + 1 - sy * ky) / ky];
        if (rect) {
          const [U0, V0, U1, V1] = rect;
          [lu0, lv0, lu1, lv1] = [U0 + (U1 - U0) * lu0, V0 + (V1 - V0) * lv0, U0 + (U1 - U0) * lu1, V0 + (V1 - V0) * lv1];
        } else {
          rect = parentRect(i / n, j / n, (i + 1) / n, (j + 1) / n);
          if (!rect) continue;
          [lu0, lv0, lu1, lv1] = rect;
        }
        const p00 = at(i, j);
        const p10 = at(i + 1, j);
        const p11 = at(i + 1, j + 1);
        const p01 = at(i, j + 1);
        if (!(Number.isFinite(p00[0]) && Number.isFinite(p10[0]) && Number.isFinite(p11[0]) && Number.isFinite(p01[0]))) continue;
        out.push(p00[0], p00[1], lu0, lv0);
        out.push(p10[0], p10[1], lu1, lv0);
        out.push(p11[0], p11[1], lu1, lv1);
        out.push(p00[0], p00[1], lu0, lv0);
        out.push(p11[0], p11[1], lu1, lv1);
        out.push(p01[0], p01[1], lu0, lv1);
      }
  }
}

/**
 * The vector tiles a service batch draws this frame, coarser first (a finer one over the part it shares). `pxPerUnit`
 * is CSS pixels a unit: a style's zoom is the CSS pixel's, as MapLibre's (picture tiles take the device's, for
 * sharpness on a high DPI screen; docs/adr/0208 §3).
 */
export function serviceVectors(
  paint: ServicePaint,
  view: readonly [number, number, number, number],
  pxPerUnit: number,
  source: ServiceSource,
): ServiceVector[] {
  const grid = source.grid(paint.service);
  if (!grid || !grid.vector) return [];
  const [ax, ay] = paint.anchor;
  const seen = grid.tiles(view[0] + ax, view[1] + ay, view[2] + ax, view[3] + ay, pxPerUnit);
  if (seen.length < 8) return [];
  const level = seen[0];
  const zoom = grid.displayZoom(seen[1]);
  const out: ServiceVector[] = [];
  for (let k = 8; k + 1 < seen.length; k += 2) {
    const t = source.vector(paint.service, level, seen[k], seen[k + 1], zoom);
    if (t) out.push(t);
  }
  return out;
}
