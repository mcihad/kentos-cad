import { describe, expect, it } from 'vitest';
import type { PolylineEntity } from '../model/entities';
import { ChamferTool, FilletTool } from './cornerTools';
import { VertexTool } from './pathEditTools';
import { at, pt, toolHarness } from './toolHarness';

/**
 * The vertex tool and the corner tools on a multi-part area (docs/adr/0143): they act on the part the click is on,
 * the other parts and every hole stay, and the elevations follow by the command's rules (docs/adr/0142). The
 * harness's tolerance is one metre to the pixel, so the parts are 100 m squares.
 */
const square = (x: number, y: number, side: number) => [pt(x, y), pt(x + side, y), pt(x + side, y + side), pt(x, y + side)];

function scene() {
  const h = toolHarness();
  const area = h.add({
    kind: 'polygon',
    pts: square(0, 0, 100),
    zs: [1, 2, 3, 4],
    holes: [{ pts: square(20, 20, 10), zs: [10, 11, 12, 13] }],
    parts: [{ pts: square(200, 0, 100), zs: [5, 6, 7, 8], holes: [{ pts: square(240, 40, 20), zs: [20, 21, 22, 23] }] }],
  }) as PolylineEntity;
  h.state.hit = area;
  h.state.inWindow = [area.id];
  const now = () => h.doc.get(area.id) as PolylineEntity;
  return { h, area, now };
}

describe('Köşe ekle/sil on a multi-part area', () => {
  it('removes the vertex under the click from its own part, the counting running part after part', () => {
    const { h, area, now } = scene();
    const tool = h.use(new VertexTool(h.ctx));
    tool.activate();
    tool.pointerDown(at(300, 0));
    expect(h.said()).toEqual(['Köşe silindi.']);
    expect(now().parts![0].pts).toEqual([pt(200, 0), pt(300, 100), pt(200, 100)]);
    expect(now().parts![0].zs).toEqual([5, 7, 8]);
    expect(now().parts![0].holes).toEqual(area.parts![0].holes);
    expect(now().pts).toEqual(area.pts);
    expect(now().holes).toEqual(area.holes);
    expect(now().zs).toEqual([1, 2, 3, 4]);
    expect(h.doc.undo()).toBe('Köşe sil');
    expect(now()).toEqual(area);
  });

  it('still removes a vertex of the first part, and leaves the parts after it whole', () => {
    const { h, area, now } = scene();
    const tool = h.use(new VertexTool(h.ctx));
    tool.pointerDown(at(0, 0));
    expect(now().pts).toEqual(area.pts.slice(1));
    expect(now().zs).toEqual([2, 3, 4]);
    expect(now().parts).toEqual(area.parts);
    expect(now().holes).toEqual(area.holes);
  });

  it('adds a vertex on the edge clicked in the second part, with the elevation along that edge', () => {
    const { h, area, now } = scene();
    const tool = h.use(new VertexTool(h.ctx));
    tool.pointerDown(at(250, 0));
    expect(h.said()).toEqual(['Köşe eklendi.']);
    expect(now().parts![0].pts).toEqual([pt(200, 0), pt(250, 0), ...square(200, 0, 100).slice(1)]);
    expect(now().parts![0].zs).toEqual([5, 5.5, 6, 7, 8]);
    expect(now().pts).toEqual(area.pts);
    expect(now().zs).toEqual([1, 2, 3, 4]);
    expect(now().parts![0].holes).toEqual(area.parts![0].holes);
  });

  it('keeps away from the corners of a hole of a part: they move with grips', () => {
    const { h, area, now } = scene();
    const tool = h.use(new VertexTool(h.ctx));
    tool.pointerDown(at(240, 50));
    expect(h.said()).toEqual(['İç halkanın köşeleri tutamaçla taşınır; köşe eklemek ya da silmek için alanı Patlat ile halkalarına ayırın.']);
    expect(now()).toEqual(area);
  });
});

/** A corner tool at the second part's corner (300, 0), given the size typed. */
function cornerAt(tool: ChamferTool | FilletTool, ctxTool = tool) {
  ctxTool.activate();
  tool.pointerMove(at(300, 0));
  tool.pointerDown(at(300, 0));
}

describe('Pah and Köşe yuvarla on a multi-part area', () => {
  it('cut the corner of the second part; the first part, the holes and the other elevations stay', () => {
    const { h, area, now } = scene();
    const tool = h.use(new ChamferTool(h.ctx));
    cornerAt(tool);
    expect(tool.input('10')).toBe(true);
    expect(now().parts![0].pts).toEqual([pt(200, 0), pt(290, 0), pt(300, 10), pt(300, 100), pt(200, 100)]);
    // The new corners lie on the edges of the corner: the elevation along each.
    const zs = now().parts![0].zs!;
    expect(zs).toHaveLength(5);
    expect([zs[0], zs[3], zs[4]]).toEqual([5, 7, 8]);
    expect(zs[1]).toBeCloseTo(5.9, 9);
    expect(zs[2]).toBeCloseTo(6.1, 9);
    expect(now().parts![0].holes).toEqual(area.parts![0].holes);
    expect(now().pts).toEqual(area.pts);
    expect(now().holes).toEqual(area.holes);
    expect(now().zs).toEqual([1, 2, 3, 4]);
    expect(h.doc.undo()).toBe('Pah');
    expect(now()).toEqual(area);
  });

  it('round it: an arc between the tangent points of the part’s own edges', () => {
    const { h, area, now } = scene();
    const tool = h.use(new FilletTool(h.ctx));
    cornerAt(tool);
    expect(tool.input('10')).toBe(true);
    const part = now().parts![0];
    expect(part.pts).toHaveLength(5);
    // The tangent points, to the last digit a circle allows.
    expect([part.pts[0], part.pts[3], part.pts[4]]).toEqual([pt(200, 0), pt(300, 100), pt(200, 100)]);
    expect(part.pts[1].x).toBeCloseTo(290, 9);
    expect(part.pts[2].y).toBeCloseTo(10, 9);
    expect(part.bulges?.some((b) => b !== 0)).toBe(true);
    expect(part.holes).toEqual(area.parts![0].holes);
    expect(now().pts).toEqual(area.pts);
    expect(now().holes).toEqual(area.holes);
    expect(h.said().at(-1)).toBe('Köşe 10.000 m yarıçapla yuvarlandı.');
  });

  it('take the corner two neighbouring edges of the second part share', () => {
    const { h, area, now } = scene();
    const tool = h.use(new ChamferTool(h.ctx));
    tool.activate();
    // Away from any corner: the middles of the bottom and right edges of the second part.
    tool.pointerDown(at(250, 0));
    tool.pointerDown(at(300, 50));
    expect(tool.input('10')).toBe(true);
    expect(now().parts![0].pts).toEqual([pt(200, 0), pt(290, 0), pt(300, 10), pt(300, 100), pt(200, 100)]);
    expect(now().pts).toEqual(area.pts);
  });

  it('say the edges are not neighbours when they are the edges of two parts', () => {
    const { h, area, now } = scene();
    const tool = h.use(new ChamferTool(h.ctx));
    tool.activate();
    tool.pointerDown(at(50, 0));
    tool.pointerDown(at(250, 0));
    expect(h.said().at(-1)).toBe('Seçilen kenarlar komşu değil; ortak köşesi olan iki kenar seçin.');
    expect(now()).toEqual(area);
  });

  it('still cut a corner of the first part, the second part whole', () => {
    const { h, area, now } = scene();
    const tool = h.use(new ChamferTool(h.ctx));
    tool.activate();
    tool.pointerMove(at(100, 100));
    tool.pointerDown(at(100, 100));
    expect(tool.input('10')).toBe(true);
    expect(now().pts).toEqual([pt(0, 0), pt(100, 0), pt(100, 90), pt(90, 100), pt(0, 100)]);
    expect(now().parts).toEqual(area.parts);
    expect(now().holes).toEqual(area.holes);
  });
});
