import { describe, expect, it } from 'vitest';
import { RasterQuads, RasterSlots, SLOT, rasterQuads, type RasterPaint } from './rasterPass';
import type { AtlasSource } from './types';

/**
 * The raster pass's CPU half (docs/adr/0204 §5), as both web backends run it: the atlas page's slots (the least
 * recently drawn let go, never one drawn this frame), and a raster's quads this frame: the tiles in view from the
 * core (`rasterTiles`), each from its slot or, while it is made, from the nearest coarser tile the page holds.
 */

/** An atlas whose workers have made `made` tiles (`level|tx|ty`); the others are asked for. */
function atlas(made: Set<string>, asked: string[]): AtlasSource {
  return {
    size: 2048,
    generation: 0,
    attach: () => {},
    beginFrame: () => {},
    lookup: () => null,
    picture: () => null,
    rasterTile: (_r, _u, _l, _a, level, tx, ty) => {
      const key = `${level}|${tx}|${ty}`;
      if (made.has(key)) return new Uint8Array(SLOT * SLOT * 4);
      asked.push(key);
      return null;
    },
    rasterFrame: () => {},
  };
}

/** A 1024 × 512 raster of half-metre pixels from (0, 512) down, north up. */
const PAINT: RasterPaint = { kind: 'raster', raster: 'file:a.tif', url: null, look: '{"render":"gray","bands":[1]}', affine: [0, 0.5, 0, 256, 0, -0.5], size: [1024, 512], nearest: false, opacity: 1 };

describe('raster slots', () => {
  it('let go the least recently drawn, never one drawn this frame', () => {
    const slots = new RasterSlots(SLOT * 2, SLOT);
    slots.beginFrame();
    expect(slots.take('a')).toBe(0);
    expect(slots.take('b')).toBe(1);
    // Both drawn this frame: none may go.
    expect(slots.take('c')).toBeNull();
    slots.beginFrame();
    slots.touch(1);
    expect(slots.take('c')).toBe(0);
    expect(slots.get('a')).toBeUndefined();
    expect(slots.get('b')).toBe(1);
    const uv = slots.uv(1, 0, 0, 256, 256);
    [(SLOT + 1) / (2 * SLOT), 1 / SLOT, (SLOT + 1 + 256) / (2 * SLOT), 257 / SLOT].forEach((v, i) => expect(uv[i]).toBeCloseTo(v, 12));
  });
});

describe('a raster’s quads', () => {
  it('draw the tiles the atlas holds and the coarser tile over those it does not', () => {
    // The whole raster in view at its pixel's size: level 0, eight tiles; level 2 is one tile.
    const view: [number, number, number, number] = [0, 0, 512, 256];
    const asked: string[] = [];
    const slots = new RasterSlots(SLOT * 16, SLOT * 2);
    slots.beginFrame();
    const quads = new RasterQuads();
    const put: number[] = [];
    rasterQuads(PAINT, [0, 0], view, 2, slots, atlas(new Set(['0|0|0', '2|0|0']), asked), (s) => put.push(s), { left: 32 }, quads);
    // One tile made: drawn; the other seven asked for and, with no coarser tile held yet, left out.
    expect(put).toHaveLength(1);
    expect(quads.count).toBe(6);
    expect(asked).toHaveLength(7);
    // Its corners: pixel (0, 0) to (256, 256), in metres from the batch's tile.
    const xy = Array.from({ length: 6 }, (_, i) => [quads.data[4 * i], quads.data[4 * i + 1]]);
    expect(xy).toEqual([
      [0, 256],
      [128, 256],
      [128, 128],
      [0, 256],
      [128, 128],
      [0, 128],
    ]);
    // Far out the last level is asked for; once held, the finer tiles take their part of it.
    const far = new RasterQuads();
    rasterQuads(PAINT, [0, 0], view, 0.5, slots, atlas(new Set(['2|0|0']), []), () => {}, { left: 32 }, far);
    expect(far.count).toBe(6);
    const again = new RasterQuads();
    rasterQuads(PAINT, [0, 0], view, 2, slots, atlas(new Set(), []), () => {}, { left: 32 }, again);
    // Every tile drawn: the made one from its slot, the seven from the coarse tile's parts.
    expect(again.count).toBe(8 * 6);
  });
});
