import { describe, expect, it } from 'vitest';
import { Signal } from '../core/signal';
import type { LineEntity, PointEntity, PolylineEntity as PolygonEntity } from '../model/entities';
import { entityGrips } from '../model/ops/grips';
import { SelectTool } from './SelectTool';
import { at, canvasLog, pt, toolHarness } from './toolHarness';

/**
 * Topolojik düzenleme through the select tool's grips (docs/adr/0160): with the mode on, the neighbours sharing what a
 * grip changed go with it in the same step, a locked layer's stay and are said, points only with Noktalar da; off,
 * only the object changes. The stand-in viewport's window query answers `inWindow` whatever the box; the store's own
 * query is played in the shared trace (fixtures/interaction/v1/topology-edit.json).
 */
function scene() {
  const h = toolHarness();
  const a = h.add({ kind: 'polygon', pts: [pt(0, 0), pt(20, 0), pt(20, 30), pt(0, 30)], zs: [null, 100, 103, null] }) as PolygonEntity;
  const b = h.add({ kind: 'polygon', pts: [pt(20, 0), pt(40, 0), pt(40, 30), pt(20, 30)], zs: [100, null, null, 103] }) as PolygonEntity;
  const road = h.add({ kind: 'line', layerId: 'yol', a: pt(20, 30), b: pt(35, 42) }) as LineEntity;
  const locked = h.add({ kind: 'polygon', layerId: 'kilitli', pts: [pt(0, 30), pt(20, 30), pt(10, 42)] }) as PolygonEntity;
  const p1 = h.add({ kind: 'point', p: pt(20, 30), label: 'P1', z: 103 }) as PointEntity;
  h.state.inWindow = [a.id, b.id, road.id, locked.id, p1.id];
  const grip = { id: a.id, index: 2 };
  // The screen is the world, one pixel to the metre.
  const camera = h.ctx.view.camera as unknown as { screenToWorld: (p: { x: number; y: number }) => { x: number; y: number }; scale: number };
  camera.screenToWorld = (p) => p;
  camera.scale = 1;
  Object.assign(h.ctx.view, { cursorWorld: new Signal<{ x: number; y: number } | null>(pt(0, 0)), gripAt: () => grip });
  const tool = h.use(new SelectTool(h.ctx));
  const settings = h.ctx.settings;
  /** The grip clicked without dragging, then a typed place for it. */
  const typed = (where: { x: number; y: number }, text: string) => {
    tool.pointerDown(at(where.x, where.y));
    tool.pointerUp(at(where.x, where.y));
    expect(tool.input(text)).toBe(true);
  };
  return { h, a, b, road, locked, p1, grip, tool, settings, typed };
}

describe('Topolojik düzenleme through grips', () => {
  it('off, as every session starts: the corner moves in its own object only', () => {
    const { h, a, b, road, typed } = scene();
    typed(pt(20, 30), '@1,1');
    expect(h.doc.get(a.id)).toMatchObject({ pts: [pt(0, 0), pt(20, 0), pt(21, 31), pt(0, 30)] });
    expect(h.doc.get(b.id)).toEqual(b);
    expect(h.doc.get(road.id)).toEqual(road);
    expect(h.said().some((t) => t.startsWith('Topolojik'))).toBe(false);
  });

  it('on: the shared corner moves in the neighbour and the road too, the locked one stays and is said, all in one step', () => {
    const { h, a, b, road, locked, p1, settings, typed } = scene();
    settings.topology.set(true);
    typed(pt(20, 30), '@1,1');
    expect(h.doc.get(a.id)).toMatchObject({ pts: [pt(0, 0), pt(20, 0), pt(21, 31), pt(0, 30)], zs: [null, 100, 103, null] });
    // The neighbour's corner keeps its elevation as the object's does (docs/adr/0160 §4).
    expect(h.doc.get(b.id)).toMatchObject({ pts: [pt(20, 0), pt(40, 0), pt(40, 30), pt(21, 31)], zs: [100, null, null, 103] });
    expect(h.doc.get(road.id)).toMatchObject({ a: pt(21, 31), b: pt(35, 42) });
    expect(h.doc.get(locked.id)).toEqual(locked);
    // Without Noktalar da the point is no corner.
    expect(h.doc.get(p1.id)).toEqual(p1);
    expect(h.said()).toEqual(['Topolojik düzenleme: 2 komşu nesne de değişti.', 'Kilitli katmandaki 1 komşu nesne değişmedi; ortak sınır ayrıldı.']);
    expect(h.doc.undo()).toBe('Tutamaçla düzenle');
    expect(h.doc.get(a.id)).toEqual(a);
    expect(h.doc.get(b.id)).toEqual(b);
    expect(h.doc.get(road.id)).toEqual(road);
  });

  it('with Noktalar da the point comes along, keeping its name and elevation', () => {
    const { h, p1, settings, typed } = scene();
    settings.topology.set(true);
    settings.topologyPoints.set(true);
    typed(pt(20, 30), '@1,1');
    expect(h.doc.get(p1.id)).toMatchObject({ p: pt(21, 31), label: 'P1', z: 103 });
    expect(h.said()[0]).toBe('Topolojik düzenleme: 3 komşu nesne de değişti.');
  });

  it('a corner added on the shared edge goes into the neighbour too, its elevation along the edge in both', () => {
    const { h, a, b, grip, settings, typed } = scene();
    settings.topology.set(true);
    // The mid grip of the shared edge, from (20, 0) to (20, 30): after the four vertices, the second edge's.
    grip.index = 5;
    expect(entityGrips(a)[5]).toEqual(pt(20, 15));
    typed(pt(20, 15), '@0,-3');
    const z = 100 + 3 * 0.4;
    const [na, nb] = [h.doc.get(a.id) as PolygonEntity, h.doc.get(b.id) as PolygonEntity];
    expect(na.pts).toEqual([pt(0, 0), pt(20, 0), pt(20, 12), pt(20, 30), pt(0, 30)]);
    expect(nb.pts).toEqual([pt(20, 0), pt(40, 0), pt(40, 30), pt(20, 30), pt(20, 12)]);
    expect(na.zs![2]).toBeCloseTo(z, 9);
    expect(nb.zs![4]).toBeCloseTo(z, 9);
    expect(h.said()).toEqual(['Topolojik düzenleme: 1 komşu nesne de değişti.']);
  });

  it('the preview draws the neighbours that follow, dashed, as the object', () => {
    const { tool, settings } = scene();
    settings.topology.set(true);
    tool.pointerDown(at(20, 30));
    tool.pointerUp(at(20, 30));
    tool.pointerMove(at(25, 35));
    const { g, view, paths } = canvasLog();
    tool.draw(g, view);
    const drawn = (pts: [number, number][]) => paths.some((p) => p.dashed && pts.every(([x, y]) => p.points.some(([px, py]) => px === x && py === y)));
    expect(drawn([[0, 0], [25, 35], [0, 30]])).toBe(true);
    expect(drawn([[40, 0], [40, 30], [25, 35]])).toBe(true);
    expect(drawn([[25, 35], [35, 42]])).toBe(true);
    // Off, only the object's.
    settings.topology.set(false);
    const off = canvasLog();
    tool.draw(off.g, off.view);
    expect(off.paths.some((p) => p.points.some(([x, y]) => x === 40 && y === 30))).toBe(false);
  });
});
