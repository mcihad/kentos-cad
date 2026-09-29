import { describe, expect, it } from 'vitest';
import { Signal } from '../core/signal';
import type { Entity } from '../model/entities';
import { entityGrips } from '../model/ops/grips';
import { SelectTool } from './SelectTool';
import { at, canvasLog, pt, toolHarness } from './toolHarness';

/**
 * The tag of a grip whose vertex has an elevation (docs/adr/0142): `Kot 98.500 m` beside the pointer resting on it,
 * and the same line in the tag of a grip being moved. Nothing changes for a vertex without one. Over a document and
 * a stand-in for the viewport that answers `gripAt` as the test says.
 */
const SQUARE = [pt(0, 0), pt(10, 0), pt(10, 10), pt(0, 10)];

function scene() {
  const h = toolHarness();
  const path = h.add({ kind: 'polyline', pts: [pt(0, 0), pt(5, 0), pt(9, 4)], zs: [98.5, null, 105.25] });
  const area = h.add({ kind: 'polygon', pts: SQUARE, zs: [1, 2, 3, 4], holes: [{ pts: [pt(2, 2), pt(4, 2), pt(4, 4)], zs: [7, null, 9] }] });
  const line = h.add({ kind: 'line', a: pt(20, 0), b: pt(30, 0), za: 12, zb: 34.5 });
  const plain = h.add({ kind: 'line', a: pt(20, 5), b: pt(30, 5) });
  const spot = h.add({ kind: 'point', p: pt(40, 40), z: 7.25 });
  const state = { grip: null as { id: number; index: number } | null, calls: 0 };
  Object.assign(h.ctx.view, {
    cursorWorld: new Signal<{ x: number; y: number } | null>(pt(0, 0)),
    gripAt: () => (state.calls++, state.grip),
  });
  const tool = h.use(new SelectTool(h.ctx));
  /** The tool drawn once with the pointer resting at `p` over the grip `hit`; the texts it wrote. */
  const hover = (hit: { id: number; index: number } | null, p = pt(50, 50)) => {
    state.grip = hit;
    tool.pointerMove(at(p.x, p.y));
    const c = canvasLog();
    tool.draw(c.g, c.view);
    return c.texts;
  };
  return { h, tool, state, hover, path, area, line, plain, spot };
}

const select = (h: ReturnType<typeof toolHarness>, ...es: Entity[]) => h.ctx.selection.set(es.map((e) => e.id));

describe('the tag of a grip with an elevation, hovering', () => {
  it('says Kot and the elevation, in metres, for a vertex that has one', () => {
    const { h, hover, path } = scene();
    select(h, path);
    expect(hover({ id: path.id, index: 0 })).toEqual(['Kot 98.500 m']);
    expect(hover({ id: path.id, index: 2 })).toEqual(['Kot 105.250 m']);
  });

  it('a line’s two ends and a point’s grip', () => {
    const { h, hover, line, spot } = scene();
    select(h, line, spot);
    expect(hover({ id: line.id, index: 0 })).toEqual(['Kot 12.000 m']);
    expect(hover({ id: line.id, index: 1 })).toEqual(['Kot 34.500 m']);
    expect(hover({ id: spot.id, index: 0 })).toEqual(['Kot 7.250 m']);
  });

  it('an area’s vertices and its holes’', () => {
    const { h, hover, area } = scene();
    select(h, area);
    expect(hover({ id: area.id, index: 3 })).toEqual(['Kot 4.000 m']);
    // Four vertices and four mid grips come first: the hole's are 8, 9, 10.
    expect(hover({ id: area.id, index: 8 })).toEqual(['Kot 7.000 m']);
    expect(hover({ id: area.id, index: 10 })).toEqual(['Kot 9.000 m']);
  });

  it('nothing for a vertex without one, a mid grip, or a hole’s vertex without one', () => {
    const { h, hover, path, area } = scene();
    select(h, path, area);
    // The second vertex of the polyline has none; index 3 is its first mid grip.
    expect(hover({ id: path.id, index: 1 })).toEqual([]);
    expect(hover({ id: path.id, index: 3 })).toEqual([]);
    expect(hover({ id: area.id, index: 4 })).toEqual([]);
    expect(hover({ id: area.id, index: 9 })).toEqual([]);
  });

  it('nothing where no grip is under the pointer', () => {
    const { h, hover, path } = scene();
    select(h, path);
    expect(hover(null)).toEqual([]);
  });

  it('a selection whose objects have no elevation is not even asked for its grips', () => {
    const { h, hover, state, plain } = scene();
    select(h, plain);
    expect(hover({ id: plain.id, index: 0 })).toEqual([]);
    expect(state.calls).toBe(0);
  });

  it('nothing when the pointer has left the drawing, and nothing selected', () => {
    const { h, tool, state, path } = scene();
    select(h, path);
    state.grip = { id: path.id, index: 0 };
    tool.pointerMove(at(5, 5));
    (h.ctx.view as unknown as { cursorWorld: Signal<unknown> }).cursorWorld.set(null);
    const c = canvasLog();
    tool.draw(c.g, c.view);
    expect(c.texts).toEqual([]);
    (h.ctx.view as unknown as { cursorWorld: Signal<unknown> }).cursorWorld.set(pt(0, 0));
    h.ctx.selection.clear();
    const d = canvasLog();
    tool.draw(d.g, d.view);
    expect(d.texts).toEqual([]);
  });

  it('follows the drawing: an elevation changed under the pointer shows at once, one cleared no longer', () => {
    const { h, tool, state, path } = scene();
    select(h, path);
    state.grip = { id: path.id, index: 0 };
    tool.pointerMove(at(50, 50));
    const texts = () => {
      const c = canvasLog();
      tool.draw(c.g, c.view);
      return c.texts;
    };
    expect(texts()).toEqual(['Kot 98.500 m']);
    // The pointer has not moved: the drawing was edited (a Kot row, an undo) and the tag is the new one.
    h.doc.update(path.id, { zs: [10, null, 105.25] } as Partial<Entity>);
    expect(texts()).toEqual(['Kot 10.000 m']);
    h.doc.update(path.id, { zs: [null, null, 105.25] } as Partial<Entity>);
    expect(texts()).toEqual([]);
  });

  it('the pointer is on the vertex, not on the object: no highlight of the object (and no card) over a tagged grip only', () => {
    const { h, hover, path } = scene();
    select(h, path);
    h.state.hit = path;
    hover({ id: path.id, index: 0 });
    expect(h.ctx.selection.hover.value).toBeNull();
    // A vertex without an elevation, as before: the object under the pointer is the one hovered.
    hover({ id: path.id, index: 1 });
    expect(h.ctx.selection.hover.value).toBe(path.id);
    hover(null);
    expect(h.ctx.selection.hover.value).toBe(path.id);
  });

  it('is not drawn once a press starts a selection box', () => {
    const { h, hover, tool, state, path } = scene();
    select(h, path);
    expect(hover({ id: path.id, index: 0 })).toEqual(['Kot 98.500 m']);
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
    tool.pointerMove(at(9, 9, { snap: { kind: 'endpoint', point: pt(9, 9), entityId: path.id } as never }));
    const c = canvasLog();
    tool.draw(c.g, c.view);
    expect(c.texts).toEqual(['5.000 m', 'Kot 105.250 m']);
  });

  it('a vertex without an elevation has the distance alone', () => {
    const { h, tool, state, path } = scene();
    select(h, path);
    const grips = entityGrips(path);
    state.grip = { id: path.id, index: 1 };
    tool.pointerDown(at(grips[1].x, grips[1].y));
    tool.pointerMove(at(5, 2, { snap: { kind: 'endpoint', point: pt(5, 2), entityId: path.id } as never }));
    const c = canvasLog();
    tool.draw(c.g, c.view);
    expect(c.texts).toEqual(['2.000 m']);
  });
});
