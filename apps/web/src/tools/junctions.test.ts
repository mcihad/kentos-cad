import { describe, expect, it } from 'vitest';
import type { Entity, PolylineEntity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { AdjoinTool } from './adjoinTool';
import { PathTool } from './pathTool';
import { RectangleTool } from './shapeTools';
import { at, pt, toolHarness } from './toolHarness';

/**
 * Corners shared with the neighbours (docs/adr/0162 §4): with Topoloji on, a new area drawn by its outline is joined
 * with its neighbours corner by corner, in its own undo step. The scene is fixtures/interaction/v1/adjoin.kcad's:
 * parcels 101 (−30..−10) and 103 (10..30) between y 0 and 24 and a 4 m transformer lot (−2..2, 10..14) between them on
 * the active layer, a road below them on Yol (y −8..0). Expected values worked out by hand; the desktop's are
 * crates/native/interaction/tests/junctions.rs, the shared trace fixtures/interaction/v1/junctions.json.
 */
const rect = (x0: number, y0: number, x1: number, y1: number): Vec2[] => [pt(x0, y0), pt(x1, y0), pt(x1, y1), pt(x0, y1)];
const xy = (pts: readonly [number, number][]): Vec2[] => pts.map(([x, y]) => pt(x, y));

function scene() {
  const h = toolHarness();
  h.ctx.settings.topology.set(true);
  // Screen is world here: a click within 1 m (not the harness's 8) of the first corner closes the area.
  h.ctx.prefs.snapAperture.set(1);
  const p101 = h.add({ kind: 'polygon', pts: rect(-30, 0, -10, 24) });
  const p103 = h.add({ kind: 'polygon', pts: rect(10, 0, 30, 24) });
  h.add({ kind: 'polygon', pts: rect(-2, 10, 2, 14) });
  const road = h.add({ kind: 'polygon', layerId: 'yol', pts: rect(-40, -8, 40, 0) });
  return { h, p101, p103, road };
}

type Harness = ReturnType<typeof scene>['h'];

const corners = (h: Harness, e: Entity): Vec2[] => (h.doc.get(e.id) as PolylineEntity).pts;
const newest = (h: Harness) => [...h.doc.all()].at(-1) as PolylineEntity;
const count = (h: Harness) => [...h.doc.all()].length;

/** Draws through `points` with `tool` and confirms; how many messages there were before the confirm. */
function draw(h: Harness, tool: PathTool, ...points: [number, number][]): number {
  h.use(tool);
  tool.activate();
  for (const [x, y] of points) tool.pointerDown(at(x, y));
  const before = h.said().length;
  tool.confirm();
  tool.deactivate();
  return before;
}

const polygon = (h: Harness) => new PathTool(h.ctx, { id: 'polygon', label: 'Kapalı alan', closed: true });
const GAP: [number, number][] = [
  [-10, 5],
  [10, 5],
  [10, 20],
  [-10, 20],
];

describe('Topoloji açıkken komşulara köşe', () => {
  it('leaves the neighbours as they are while Topoloji is off', () => {
    const { h, p101, p103 } = scene();
    h.ctx.settings.topology.set(false);
    draw(h, polygon(h), ...GAP);
    expect(corners(h, p101)).toEqual(rect(-30, 0, -10, 24));
    expect(corners(h, p103)).toEqual(rect(10, 0, 30, 24));
  });

  it('gives the new area’s corners to the parcels on either side, in one undo step', () => {
    const { h, p101, p103 } = scene();
    const before = draw(h, polygon(h), ...GAP);
    expect(newest(h).pts).toEqual(xy(GAP));
    expect(corners(h, p101)).toEqual(xy([[-30, 0], [-10, 0], [-10, 5], [-10, 20], [-10, 24], [-30, 24]]));
    expect(corners(h, p103)).toEqual(xy([[10, 0], [30, 0], [30, 24], [10, 24], [10, 20], [10, 5]]));
    expect(h.said().slice(before)).toEqual(['Topolojik düzenleme: 2 komşu nesneye yeni alanın 4 köşesi eklendi.', 'Kapalı alan eklendi: 300.00 m²']);
    expect(h.doc.undo()).toBe('Ekle');
    expect(count(h)).toBe(4);
    expect(corners(h, p101)).toEqual(rect(-30, 0, -10, 24));
    expect(corners(h, p103)).toEqual(rect(10, 0, 30, 24));
  });

  it('gives the new area the parcels’ corners on its edge', () => {
    const { h, p101, p103 } = scene();
    // A strip over both parcels: their corners (±10, 24) lie on its bottom edge, its own (±12, 24) on their tops.
    const before = draw(h, polygon(h), [-12, 24], [12, 24], [12, 30], [-12, 30]);
    expect(newest(h).pts).toEqual(xy([[-12, 24], [-10, 24], [10, 24], [12, 24], [12, 30], [-12, 30]]));
    expect(corners(h, p101)).toEqual(xy([[-30, 0], [-10, 0], [-10, 24], [-12, 24], [-30, 24]]));
    expect(corners(h, p103)).toEqual(xy([[10, 0], [30, 0], [30, 24], [12, 24], [10, 24]]));
    expect(h.said().slice(before)).toEqual([
      'Topolojik düzenleme: 2 komşu nesneye yeni alanın 2 köşesi eklendi.',
      'Topolojik düzenleme: yeni alana komşulardan 2 köşe eklendi.',
      'Kapalı alan eklendi: 144.00 m²',
    ]);
  });

  it('gives a locked neighbour no corner and says so', () => {
    const { h, road } = scene();
    h.doc.layers.toggleLocked('yol');
    // Its corners (±5, 0) lie on the road's top edge.
    const before = draw(h, polygon(h), [-5, 0], [5, 0], [5, 6], [-5, 6]);
    expect(corners(h, road)).toEqual(rect(-40, -8, 40, 0));
    expect(h.said().slice(before)).toEqual(['Kilitli katmandaki 1 komşu nesneye köşe eklenmedi.', 'Kapalı alan eklendi: 60.00 m²']);
  });

  it('gives Bitişik alan’s corners to the parcels and the road', () => {
    const { h, p101, p103, road } = scene();
    h.ctx.settings.overlap.set('layers');
    h.ctx.settings.overlapLayers.set(new Set(['cizim', 'yol']));
    const before = draw(h, new AdjoinTool(h.ctx), [-15, 20], [15, 20]);
    expect(corners(h, p101)).toEqual(xy([[-30, 0], [-10, 0], [-10, 20], [-10, 24], [-30, 24]]));
    expect(corners(h, p103)).toEqual(xy([[10, 0], [30, 0], [30, 24], [10, 24], [10, 20]]));
    // The new area's corners (±10, 0) lie on the road's top edge.
    expect(corners(h, road)).toEqual(xy([[-40, -8], [40, -8], [40, 0], [10, 0], [-10, 0], [-40, 0]]));
    expect(h.said().slice(before)).toEqual(['Topolojik düzenleme: 3 komşu nesneye yeni alanın 4 köşesi eklendi.', 'Bitişik alan eklendi: 384.00 m²']);
    expect(h.doc.undo()).toBe('Bitişik alan');
    expect(corners(h, road)).toHaveLength(4);
  });

  it('joins a rectangle and a parcel too', () => {
    const { h, p101, p103 } = scene();
    const rectangle = h.use(new RectangleTool(h.ctx));
    rectangle.activate();
    rectangle.pointerDown(at(-10, 5));
    rectangle.pointerDown(at(-4, 9));
    expect(corners(h, p101)).toEqual(xy([[-30, 0], [-10, 0], [-10, 5], [-10, 9], [-10, 24], [-30, 24]]));
    // Ctrl+Z inside the tool takes the rectangle and 101's new corners back in one step; drawn again, they come back.
    expect(rectangle.undoStep()).toBe(true);
    expect(corners(h, p101)).toEqual(rect(-30, 0, -10, 24));
    rectangle.pointerDown(at(-10, 5));
    rectangle.pointerDown(at(-4, 9));
    expect(corners(h, p101)).toHaveLength(6);
    draw(h, new PathTool(h.ctx, { id: 'parcel', label: 'Parsel', closed: true, parcelLayer: 'parsel' }), [10, 10], [16, 10], [16, 14], [10, 14]);
    expect(newest(h).layerId).toBe('parsel');
    expect(corners(h, p103)).toEqual(xy([[10, 0], [30, 0], [30, 24], [10, 24], [10, 14], [10, 10]]));
  });

  it('takes a point as a corner with Noktalar da', () => {
    const { h } = scene();
    h.add({ kind: 'point', p: pt(0, 5) });
    draw(h, polygon(h), ...GAP);
    expect(newest(h).pts).toEqual(xy(GAP));
    h.ctx.settings.topologyPoints.set(true);
    draw(h, polygon(h), ...GAP);
    expect(newest(h).pts).toEqual(xy([[-10, 5], [0, 5], [10, 5], [10, 20], [-10, 20]]));
  });

  it('gives a neighbour’s new corner its elevation along the edge', () => {
    const { h } = scene();
    const lot = h.add({ kind: 'polygon', pts: rect(40, 0, 50, 10), zs: [100, 110, 120, 130] } as unknown as Parameters<Harness['add']>[0]);
    draw(h, polygon(h), [50, 4], [56, 4], [56, 8], [50, 8]);
    const zs = (h.doc.get(lot.id) as PolylineEntity & { zs?: (number | null)[] }).zs ?? [];
    expect(zs).toHaveLength(6);
    [100, 110, 114, 118, 120, 130].forEach((z, i) => expect(zs[i]).toBeCloseTo(z, 9));
  });
});
