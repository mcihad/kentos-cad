import { describe, expect, it } from 'vitest';
import { Signal } from '../core/signal';
import type { Entity } from '../model/entities';
import { entityGrips } from '../model/ops/grips';
import { SelectTool } from './SelectTool';
import { at, canvasLog, pt, toolHarness } from './toolHarness';

/**
 * The tag of a grip whose vertex has an elevation (docs/adr/0142): `Kot 98.500 m` beside the pointer resting on it
 * (the nearest vertex of the selection within the grips' 6 px), and the same line in the tag of a grip being moved.
 * Nothing changes for a vertex without one. Over a document and a stand-in for the viewport whose screen is the
 * world, 1 px to the metre.
 */
const SQUARE = [pt(200, 100), pt(300, 100), pt(300, 200), pt(200, 200)];

function scene() {
  const h = toolHarness();
  const path = h.add({ kind: 'polyline', pts: [pt(0, 0), pt(100, 0), pt(180, 60)], zs: [98.5, null, 105.25] });
  const area = h.add({ kind: 'polygon', pts: SQUARE, zs: [1, 2, 3, 4], holes: [{ pts: [pt(220, 120), pt(240, 120), pt(240, 140)], zs: [7, null, 9] }] });
  const line = h.add({ kind: 'line', a: pt(400, 0), b: pt(500, 0), za: 12, zb: 34.5 });
  const plain = h.add({ kind: 'line', a: pt(400, 50), b: pt(500, 50) });
  const spot = h.add({ kind: 'point', p: pt(600, 0), z: 7.25 });
  const state = { grip: null as { id: number; index: number } | null };
  const worldToScreen = { calls: 0 };
  // The screen is the world, one pixel to the metre: the tag looks at the pointer's world place and never at the screen's vertices.
  const camera = h.ctx.view.camera as unknown as { worldToScreen: (p: { x: number; y: number }) => { x: number; y: number }; screenToWorld: (p: { x: number; y: number }) => { x: number; y: number }; scale: number };
  const identity = camera.worldToScreen;
  camera.worldToScreen = (p) => (worldToScreen.calls++, identity(p));
  camera.screenToWorld = (p) => p;
  camera.scale = 1;
  Object.assign(h.ctx.view, {
    cursorWorld: new Signal<{ x: number; y: number } | null>(pt(0, 0)),
    gripAt: () => state.grip,
  });
  const tool = h.use(new SelectTool(h.ctx));
  const drawn = () => {
    const c = canvasLog();
    tool.draw(c.g, c.view);
    return c.texts;
  };
  /** The pointer moved to (x, y); what the tool then draws. */
  const hover = (x: number, y: number) => {
    tool.pointerMove(at(x, y));
    return drawn();
  };
  return { h, tool, state, hover, drawn, worldToScreen, path, area, line, plain, spot };
}

const select = (h: ReturnType<typeof toolHarness>, ...es: Entity[]) => h.ctx.selection.set(es.map((e) => e.id));
/** A polyline with an elevation at its first vertex, on the drawing of `h`. */
const scene0 = (h: ReturnType<typeof toolHarness>): Entity => h.add({ kind: 'polyline', pts: [pt(0, 0), pt(100, 0)], zs: [98.5, null] });

describe('the tag of a grip with an elevation, hovering', () => {
  it('says Kot and the elevation, in metres, for a vertex that has one', () => {
    const { h, hover, path } = scene();
    select(h, path);
    expect(hover(0, 0)).toEqual(['Kot 98.500 m']);
    expect(hover(180, 60)).toEqual(['Kot 105.250 m']);
  });

  it('a line’s two ends and a point’s grip', () => {
    const { h, hover, line, spot } = scene();
    select(h, line, spot);
    expect(hover(400, 0)).toEqual(['Kot 12.000 m']);
    expect(hover(500, 0)).toEqual(['Kot 34.500 m']);
    expect(hover(600, 0)).toEqual(['Kot 7.250 m']);
  });

  it('an area’s vertices and its holes’', () => {
    const { h, hover, area } = scene();
    select(h, area);
    expect(hover(200, 200)).toEqual(['Kot 4.000 m']);
    expect(hover(220, 120)).toEqual(['Kot 7.000 m']);
    expect(hover(240, 140)).toEqual(['Kot 9.000 m']);
  });

  it('within the grips’ 6 px, not beyond', () => {
    const { h, hover, path } = scene();
    select(h, path);
    expect(hover(3, 4)).toEqual(['Kot 98.500 m']);
    expect(hover(0, 7)).toEqual([]);
    expect(hover(100, 100)).toEqual([]);
  });

  it('nothing for a vertex without one, a mid grip, or a hole’s vertex without one', () => {
    const { h, hover, path, area } = scene();
    select(h, path, area);
    // The second vertex of the polyline has none; the middle of its first edge is a mid grip, no vertex.
    expect(hover(100, 0)).toEqual([]);
    expect(hover(50, 0)).toEqual([]);
    expect(hover(240, 120)).toEqual([]);
  });

  it('the nearest vertex is the one: a vertex without an elevation beside one that has it takes the tag away', () => {
    const { h, hover } = scene();
    const twin = h.add({ kind: 'polyline', pts: [pt(700, 0), pt(704, 0)], zs: [null, 55] });
    select(h, twin);
    expect(hover(704, 0)).toEqual(['Kot 55.000 m']);
    // 1 px from the first vertex, 3 px from the second: the first is the grip that would be taken.
    expect(hover(701, 0)).toEqual([]);
    // Both within 6 px, the second nearer.
    expect(hover(703, 0)).toEqual(['Kot 55.000 m']);
  });

  it('a vertex of another selected object counts too: the nearest of them all', () => {
    const { h, hover, path } = scene();
    const near = h.add({ kind: 'line', a: pt(1, 1), b: pt(50, 50) });
    select(h, path, near);
    // The line without elevations has its end at (1, 1), nearer than the polyline's (0, 0).
    expect(hover(2, 2)).toEqual([]);
    expect(hover(0, 0)).toEqual(['Kot 98.500 m']);
  });

  it('a selected object on a locked layer has no grips to take: no tag', () => {
    const { h, hover } = scene();
    const locked = h.add({ kind: 'line', layerId: 'kilitli', a: pt(800, 0), b: pt(900, 0), za: 5, zb: 6 });
    select(h, locked);
    expect(hover(800, 0)).toEqual([]);
  });

  it('a selection whose objects have no elevation costs no geometry at all, and no vertex is ever brought to the screen', () => {
    const { h, hover, worldToScreen, plain } = scene();
    select(h, plain);
    expect(hover(400, 50)).toEqual([]);
    expect(worldToScreen.calls).toBe(0);
    // Nor for one that has: the pointer's place is taken to the world once, the vertices stay where they are.
    select(h, scene0(h));
    expect(hover(0, 0)).toEqual(['Kot 98.500 m']);
    expect(worldToScreen.calls).toBe(0);
  });

  it('nothing when the pointer has left the drawing, and nothing selected', () => {
    const { h, tool, drawn, path } = scene();
    select(h, path);
    tool.pointerMove(at(0, 0));
    (h.ctx.view as unknown as { cursorWorld: Signal<unknown> }).cursorWorld.set(null);
    expect(drawn()).toEqual([]);
    (h.ctx.view as unknown as { cursorWorld: Signal<unknown> }).cursorWorld.set(pt(0, 0));
    h.ctx.selection.clear();
    expect(drawn()).toEqual([]);
  });

  it('a selection past the grips’ limit of 150 objects has no tag', () => {
    const { h, hover, path } = scene();
    const many = Array.from({ length: 150 }, (_, i) => h.add({ kind: 'line', a: pt(1000, i * 20), b: pt(1010, i * 20) }));
    select(h, path, ...many);
    expect(hover(0, 0)).toEqual([]);
  });

  it('follows the drawing: an elevation changed under the pointer shows at once, one cleared no longer', () => {
    const { h, tool, drawn, path } = scene();
    select(h, path);
    tool.pointerMove(at(0, 0));
    expect(drawn()).toEqual(['Kot 98.500 m']);
    // The pointer has not moved: the drawing was edited (a Kot row, an undo) and the tag is the new one.
    h.doc.update(path.id, { zs: [10, null, 105.25] } as Partial<Entity>);
    expect(drawn()).toEqual(['Kot 10.000 m']);
    h.doc.update(path.id, { zs: [null, null, 105.25] } as Partial<Entity>);
    expect(drawn()).toEqual([]);
  });

  it('the pointer is on the vertex, not on the object: no highlight of the object (and no card) over a tagged grip only', () => {
    const { h, hover, path } = scene();
    select(h, path);
    h.state.hit = path;
    hover(0, 0);
    expect(h.ctx.selection.hover.value).toBeNull();
    // A vertex without an elevation, as before: the object under the pointer is the one hovered.
    hover(100, 0);
    expect(h.ctx.selection.hover.value).toBe(path.id);
    hover(50, 30);
    expect(h.ctx.selection.hover.value).toBe(path.id);
  });

  it('is not drawn once a press starts a selection box', () => {
    const { h, hover, tool, state, path } = scene();
    select(h, path);
    expect(hover(0, 0)).toEqual(['Kot 98.500 m']);
    state.grip = null;
    tool.pointerDown(at(70, 70));
    const c = canvasLog();
    tool.draw(c.g, c.view);
    expect(c.texts).toEqual([]);
  });
});

describe('the tag of a grip being moved', () => {
  it('says the distance moved and, on a line of its own, the elevation the vertex keeps', () => {
    const { h, tool, state, path } = scene();
    select(h, path);
    const grips = entityGrips(path);
    state.grip = { id: path.id, index: 2 };
    tool.pointerDown(at(grips[2].x, grips[2].y));
    tool.pointerMove(at(180, 110, { snap: { kind: 'endpoint', point: pt(180, 110), entityId: path.id } as never }));
    const c = canvasLog();
    tool.draw(c.g, c.view);
    expect(c.texts).toEqual(['50.000 m', 'Kot 105.250 m']);
  });

  it('a vertex without an elevation has the distance alone', () => {
    const { h, tool, state, path } = scene();
    select(h, path);
    const grips = entityGrips(path);
    state.grip = { id: path.id, index: 1 };
    tool.pointerDown(at(grips[1].x, grips[1].y));
    tool.pointerMove(at(100, 20, { snap: { kind: 'endpoint', point: pt(100, 20), entityId: path.id } as never }));
    const c = canvasLog();
    tool.draw(c.g, c.view);
    expect(c.texts).toEqual(['20.000 m']);
  });
});
