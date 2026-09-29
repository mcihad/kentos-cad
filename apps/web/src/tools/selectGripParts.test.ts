import { describe, expect, it } from 'vitest';
import { Signal } from '../core/signal';
import type { PolylineEntity } from '../model/entities';
import { entityGrips } from '../model/ops/grips';
import { SelectTool } from './SelectTool';
import { at, canvasLog, pt, toolHarness } from './toolHarness';

/**
 * A grip of a multi-part area (docs/adr/0143): dragged or typed to a place it moves in its own part, and the other
 * parts, their holes and every elevation stay; the moved vertex keeps its own (docs/adr/0142). The tag of a grip
 * says the elevation of its vertex, whichever part it is in.
 */
const square = (x: number, y: number, side: number) => [pt(x, y), pt(x + side, y), pt(x + side, y + side), pt(x, y + side)];

function scene() {
  const h = toolHarness();
  const area = h.add({
    kind: 'polygon',
    pts: square(0, 0, 10),
    zs: [1, 2, 3, 4],
    holes: [{ pts: square(2, 2, 2), zs: [10, 11, 12, 13] }],
    parts: [{ pts: square(200, 0, 100), zs: [5, 6, 7, 8], holes: [{ pts: square(240, 40, 20), zs: [20, 21, 22, 23] }] }],
  }) as PolylineEntity;
  const grip = { id: area.id, index: 0 };
  // The screen is the world, one pixel to the metre.
  const camera = h.ctx.view.camera as unknown as { worldToScreen: (p: { x: number; y: number }) => { x: number; y: number }; screenToWorld: (p: { x: number; y: number }) => { x: number; y: number }; scale: number };
  camera.screenToWorld = (p) => p;
  camera.scale = 1;
  Object.assign(h.ctx.view, { cursorWorld: new Signal<{ x: number; y: number } | null>(pt(0, 0)), gripAt: () => grip });
  const tool = h.use(new SelectTool(h.ctx));
  const now = () => h.doc.get(area.id) as PolylineEntity;
  return { h, area, grip, tool, now };
}

describe('a grip of the second part', () => {
  it('moves in its own part: the first part, the holes and every elevation stay, the moved vertex keeps its own', () => {
    const { h, area, grip, tool, now } = scene();
    // The second vertex of the second part: the first part has 12 grips (4 vertices, 4 mids, 4 hole vertices).
    grip.index = 13;
    expect(entityGrips(area)[13]).toEqual(pt(300, 0));
    tool.pointerDown(at(300, 0));
    tool.pointerUp(at(300, 0));
    expect(tool.input('@0,40')).toBe(true);
    expect(now().parts![0].pts[1]).toEqual(pt(300, 40));
    expect(now().pts).toEqual(area.pts);
    expect(now().holes).toEqual(area.holes);
    expect(now().zs).toEqual([1, 2, 3, 4]);
    expect(now().parts![0].zs).toEqual([5, 6, 7, 8]);
    expect(now().parts![0].holes).toEqual(area.parts![0].holes);
    expect(h.doc.undo()).toBe('Tutamaçla düzenle');
    expect(now()).toEqual(area);
  });

  it('a mid grip of the second part becomes a new corner of its own part, the elevations of the others staying', () => {
    const { grip, area, tool, now } = scene();
    // Its first edge’s mid grip is the fifth of the part’s grips: index 12 + 4.
    grip.index = 16;
    expect(entityGrips(area)[16]).toEqual(pt(250, 0));
    tool.pointerDown(at(250, 0));
    tool.pointerUp(at(250, 0));
    expect(tool.input('@0,-20')).toBe(true);
    expect(now().parts![0].pts).toEqual([pt(200, 0), pt(250, -20), ...square(200, 0, 100).slice(1)]);
    // The new corner lies on no vertex or edge of the object: it has no elevation (none, not 0).
    expect(now().parts![0].zs).toEqual([5, null, 6, 7, 8]);
    expect(now().pts).toEqual(area.pts);
    expect(now().zs).toEqual([1, 2, 3, 4]);
    expect(now().parts![0].holes).toEqual(area.parts![0].holes);
  });
});

describe('the tag of a grip in another part', () => {
  it('says the elevation of the vertex the pointer rests on, in a part or in a part’s hole', () => {
    const { h, tool } = scene();
    h.ctx.selection.set([...h.doc.all()].map((e) => e.id));
    const tag = (x: number, y: number) => {
      tool.pointerMove(at(x, y));
      const c = canvasLog();
      tool.draw(c.g, c.view);
      return c.texts;
    };
    expect(tag(300, 100)).toEqual(['Kot 7.000 m']);
    expect(tag(240, 60)).toEqual(['Kot 23.000 m']);
    expect(tag(150, 50)).toEqual([]);
  });
});
