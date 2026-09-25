import { describe, expect, it } from 'vitest';
import type { Bounds } from '../model/geometry';
import type { CanvasPalette } from './color';
import { buildGrid, gridExtent, gridSpacing, type GridExtent } from './grid';

const palette = (): CanvasPalette => ({
  background: [0.08, 0.1, 0.13, 1],
  fg: '#E4EAF0',
  fgDim: '#A9B4C0',
  ink: '#FFFFFF',
  paper: '#151B22',
  gridMinor: [1, 1, 1, 0.04],
  gridMajor: [1, 1, 1, 0.08],
  accent: '#F5A524',
  snap: '#39D353',
  danger: '#FF5C5C',
  label: '#E4EAF0',
  labelHalo: '#151B22',
  font: 'system-ui, sans-serif',
});

const ORIGIN = { x: 486500, y: 4420000 };
/** A 1600 × 900 px view centred on (cx, cy) at `scale` px/m, in absolute world coordinates. */
const view = (cx: number, cy: number, scale: number): Bounds => ({ minX: cx - 800 / scale, minY: cy - 450 / scale, maxX: cx + 800 / scale, maxY: cy + 450 / scale });

/** x of the vertical lines (y of the horizontal ones) in a grid layer, absolute and sorted. */
function lines(layer: ReturnType<typeof buildGrid>, axis: 'x' | 'y'): { at: number; major: boolean; from: number; to: number }[] {
  const out: { at: number; major: boolean; from: number; to: number }[] = [];
  layer.lines.forEach((batch, b) => {
    const p = batch.positions;
    for (let i = 0; i < p.length; i += 4) {
      const vertical = p[i] === p[i + 2];
      if (vertical !== (axis === 'x')) continue;
      out.push(axis === 'x' ? { at: p[i], major: b === 1, from: p[i + 1], to: p[i + 3] } : { at: p[i + 1], major: b === 1, from: p[i], to: p[i + 2] });
    }
  });
  return out.sort((a, c) => a.at - c.at);
}

describe('grid', () => {
  it('keeps the grid it built while the view stays inside it with the same spacing, origin and colours', () => {
    const pal = palette();
    // At 6 px/m the minor spacing is 5 m; it stays 5 m from 2.8 up to 7 px/m.
    expect(gridSpacing(6)).toEqual({ minor: 5, major: 20 });
    const v = view(486512.3, 4420007.7, 6);
    const first = gridExtent(v, 6, ORIGIN, pal, null);
    const w = v.maxX - v.minX;
    const h = v.maxY - v.minY;
    expect(first.box).toEqual({ minX: v.minX - w, minY: v.minY - h, maxX: v.maxX + w, maxY: v.maxY + h });
    // Panning inside it and zooming within the spacing band upload nothing.
    expect(gridExtent(view(486512.3 + 200, 4420007.7 - 70, 6), 6, ORIGIN, pal, first)).toBe(first);
    expect(gridExtent(view(486512.3, 4420007.7, 2.9), 2.9, ORIGIN, pal, first)).toBe(first);
    expect(gridExtent(view(486512.3, 4420007.7, 6.9), 6.9, ORIGIN, pal, first)).toBe(first);
    // Leaving it, another spacing, another origin or other colours build a new one around the view.
    const panned = view(486512.3 + 300, 4420007.7, 6);
    const next = gridExtent(panned, 6, ORIGIN, pal, first);
    expect(next).not.toBe(first);
    expect(next.box.minX).toBeCloseTo(panned.minX - w, 9);
    expect(gridExtent(view(486512.3, 4420007.7, 7), 7, ORIGIN, pal, first).spacing.minor).toBe(2);
    expect(gridExtent(view(486512.3, 4420007.7, 2.7), 2.7, ORIGIN, pal, first).spacing.minor).toBe(10);
    expect(gridExtent(v, 6, { x: ORIGIN.x + 1, y: ORIGIN.y }, pal, first)).not.toBe(first);
    expect(gridExtent(v, 6, ORIGIN, palette(), first)).not.toBe(first);
  });

  it('draws world-aligned lines across the whole box, relative to the origin', () => {
    const extent = gridExtent(view(486512.3, 4420007.7, 6), 6, ORIGIN, palette(), null);
    const { box } = extent;
    const layer = buildGrid(extent);
    expect(layer.id).toBe('__grid');
    for (const axis of ['x', 'y'] as const) {
      const [lo, hi] = axis === 'x' ? [box.minX, box.maxX] : [box.minY, box.maxY];
      const o = axis === 'x' ? ORIGIN.x : ORIGIN.y;
      const found = lines(layer, axis);
      // Every multiple of 5 m from the one at or before the box's edge, majors on multiples of 20 m, small float32 numbers near the origin.
      expect(found.length).toBe(Math.floor(hi / 5) - Math.floor(lo / 5) + 1);
      for (const l of found) {
        expect((l.at + o) % 5).toBe(0);
        expect(l.major).toBe((l.at + o) % 20 === 0);
        expect(Math.abs(l.at)).toBeLessThan(1000);
      }
      // Each line spans the box across.
      const [from, to] = axis === 'x' ? [box.minY - ORIGIN.y, box.maxY - ORIGIN.y] : [box.minX - ORIGIN.x, box.maxX - ORIGIN.x];
      expect(found.every((l) => l.from === Math.fround(from) && l.to === Math.fround(to))).toBe(true);
    }
    // Screen-bounded: at most three views of lines 14 px apart per axis.
    expect(lines(layer, 'x').length).toBeLessThanOrEqual((3 * 1600) / 14 + 2);
  });

  it('draws inside the view the lines a grid of the view alone had', () => {
    const pal = palette();
    // Whole-metre spacings (5 m, 200 m) give the very same numbers; a 1 cm spacing may round a line's last float32 bit either way.
    for (const [cx, cy, scale, tol] of [
      [486512.3, 4420007.7, 6, 0],
      [480000, 4400000, 0.081, 0],
      [486500.123, 4420000.456, 1797, 1e-6],
    ]) {
      const v = view(cx, cy, scale);
      const alone: GridExtent = { box: v, spacing: gridSpacing(scale), origin: ORIGIN, palette: pal };
      const around = buildGrid(gridExtent(v, scale, ORIGIN, pal, null));
      for (const axis of ['x', 'y'] as const) {
        const wide = lines(around, axis);
        const own = lines(buildGrid(alone), axis);
        expect(own.length).toBeGreaterThan(10);
        let j = 0;
        for (const l of own) {
          while (j < wide.length && wide[j].at < l.at - tol) j++;
          expect(Math.abs(wide[j].at - l.at), `${axis} ${l.at} at ${scale} px/m`).toBeLessThanOrEqual(tol);
          expect(wide[j].major).toBe(l.major);
        }
      }
    }
  });
});
