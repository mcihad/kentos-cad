import { describe, expect, it } from 'vitest';
import type { PointEntity } from '../model/entities';
import { IntersectPointTool } from './meetingTool';
import { at, pt, recorder, toolHarness } from './toolHarness';

/**
 * Kesişim noktası (docs/adr/0140): two distances, two bearings or two lines; one point
 * through cad.point.create, its Y and X in the log.
 */
const points = (h: ReturnType<typeof toolHarness>) => [...h.doc.all()].filter((e): e is PointEntity => e.kind === 'point');

describe('Kesişim noktası: İki uzaklık', () => {
  it('takes A, its distance, B, its distance and then the click near the wanted of the two meetings', () => {
    const h = toolHarness();
    const tool = h.use(new IntersectPointTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value).toContain('A noktasını belirtin');
    expect(tool.prompt.value).toContain('İki doğrultu (D) / İki doğru (L)');
    tool.pointerDown(at(0, 0));
    // The options go once something is given.
    expect(tool.prompt.value).not.toContain('İki doğrultu');
    expect(tool.prompt.value).toContain('A noktasından uzaklığı yazın ya da çember üzerinde tıklayın (Enter: 10.000 m)');
    expect(tool.input('5')).toBe(true);
    tool.pointerDown(at(8, 0));
    expect(tool.prompt.value).toContain('B noktasından uzaklığı yazın');
    expect(tool.input('5')).toBe(true);
    expect(tool.prompt.value).toContain('istediğiniz kesişime tıklayın');
    expect(tool.prompt.value).toContain('Sağdaki (Enter)');
    const r = recorder();
    tool.pointerMove(at(4, -3));
    tool.draw(r.g, r.view);
    expect(r.calls).toContain('arc');
    // The click chooses by the pointer, not by where a snap would put it.
    tool.pointerDown(at(4, -3));
    expect(points(h)).toHaveLength(1);
    expect(points(h)[0].p.x).toBeCloseTo(4);
    expect(points(h)[0].p.y).toBeCloseTo(-3);
    expect(h.said().at(-1)).toBe('Kesişim noktası kondu: Y 4.000  X -3.000');
    expect(h.doc.undo()).toBe('Kesişim noktası');
    expect(tool.prompt.value).toContain('A noktasını belirtin');
  });

  it('a distance is shown by a click on its circle, and Enter takes the one to the right of A towards B', () => {
    const h = toolHarness();
    const tool = h.use(new IntersectPointTool(h.ctx));
    tool.activate();
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(3, 4));
    expect(h.said().at(-1)).toBe('  Uzaklık 5.000 m');
    tool.pointerDown(at(8, 0));
    // Enter takes the distance kept from the last one.
    tool.confirm();
    expect(h.said().at(-1)).toBe('  Uzaklık 5.000 m');
    expect(tool.prompt.value).toContain('istediğiniz kesişime tıklayın');
    tool.confirm();
    // Right of A looking to B (east) is south.
    expect(points(h)[0].p.y).toBeCloseTo(-3);
  });

  it('circles that touch give the one point at once; circles that do not meet say why and ask the second distance again', () => {
    const h = toolHarness();
    const tool = h.use(new IntersectPointTool(h.ctx));
    tool.activate();
    tool.input('0,0');
    tool.input('4');
    tool.input('10,0');
    tool.input('1');
    expect(h.said().at(-1)).toMatch(/^İki uzaklık kesişmiyor: A ile B arası 10.000 m, uzaklıklar 4.000 m ve 1.000 m/);
    expect(points(h)).toHaveLength(0);
    expect(tool.prompt.value).toContain('B noktasından uzaklığı yazın');
    expect(tool.input('6')).toBe(true);
    expect(points(h)).toHaveLength(1);
    expect(points(h)[0].p).toEqual(pt(4, 0));
  });

  it('a distance of nothing is refused', () => {
    const h = toolHarness();
    const tool = h.use(new IntersectPointTool(h.ctx));
    tool.activate();
    tool.input('0,0');
    expect(tool.input('0')).toBe(true);
    expect(h.said().at(-1)).toBe('Uzaklık sıfırdan büyük olmalı.');
    expect(tool.input('-3')).toBe(true);
  });
});

describe('Kesişim noktası: İki doğrultu', () => {
  it('takes bearings in the project unit, from north, clockwise, and puts the point where they meet', () => {
    const h = toolHarness();
    const tool = h.use(new IntersectPointTool(h.ctx));
    tool.activate();
    expect(tool.input('D')).toBe(true);
    expect(tool.prompt.value).toContain('İki uzaklık (U) / İki doğru (L)');
    tool.pointerDown(at(0, 0));
    expect(tool.prompt.value).toContain('A noktasından doğrultuyu yazın (grad, kuzeyden saat yönünde)');
    // 50 grad is north-east, 350 grad is north-west (400 grad make the turn).
    expect(tool.input('50')).toBe(true);
    expect(h.said().at(-1)).toBe('  Semt 50.0000 g');
    tool.pointerDown(at(10, 0));
    tool.input('350');
    expect(points(h)).toHaveLength(1);
    expect(points(h)[0].p.x).toBeCloseTo(5);
    expect(points(h)[0].p.y).toBeCloseTo(5);
    expect(h.said().at(-1)).toBe('Kesişim noktası kondu: Y 5.000  X 5.000');
  });

  it('a bearing is shown by a click along it', () => {
    const h = toolHarness();
    const tool = h.use(new IntersectPointTool(h.ctx));
    tool.activate();
    tool.input('D');
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(7, 7));
    expect(h.said().at(-1)).toBe('  Semt 50.0000 g');
    const r = recorder();
    tool.pointerMove(at(3, 3));
    tool.draw(r.g, r.view);
    expect(r.calls).toContain('stroke');
  });

  it('a meeting behind a point, or parallel bearings, say so and ask the second bearing again', () => {
    const h = toolHarness();
    const tool = h.use(new IntersectPointTool(h.ctx));
    tool.activate();
    tool.input('D');
    tool.input('0,0');
    tool.input('50');
    tool.input('10,0');
    // North-east from A, and 150 grad (south-east) from B: they meet behind B.
    tool.input('150');
    expect(h.said().at(-1)).toMatch(/^Doğrultular kesişmiyor/);
    expect(points(h)).toHaveLength(0);
    expect(tool.prompt.value).toContain('B noktasından doğrultuyu yazın');
  });
});

describe('Kesişim noktası: İki doğru', () => {
  it('takes four points and puts the point where the lines meet, wherever they extend to', () => {
    const h = toolHarness();
    const tool = h.use(new IntersectPointTool(h.ctx));
    tool.activate();
    tool.input('L');
    expect(tool.prompt.value).toContain('1. doğrunun ilk noktasını belirtin');
    tool.pointerDown(at(0, 0));
    expect(tool.prompt.value).toContain('1. doğrunun ikinci noktasını belirtin');
    tool.pointerDown(at(2, 0));
    expect(tool.prompt.value).toContain('2. doğrunun ilk noktasını belirtin');
    tool.pointerDown(at(5, -4));
    const r = recorder();
    tool.pointerMove(at(5, -2));
    tool.draw(r.g, r.view);
    expect(r.calls).toContain('stroke');
    tool.pointerDown(at(5, -2));
    expect(points(h)[0].p.x).toBeCloseTo(5);
    expect(points(h)[0].p.y).toBeCloseTo(0);
  });

  it('parallel lines say so and ask the fourth point again', () => {
    const h = toolHarness();
    const tool = h.use(new IntersectPointTool(h.ctx));
    tool.activate();
    tool.input('L');
    for (const p of [[0, 0], [4, 0], [0, 3], [4, 3]]) tool.input(p.join(','));
    expect(h.said().at(-1)).toBe('Doğrular paralel; kesişmiyorlar. Dördüncü noktayı değiştirin.');
    expect(points(h)).toHaveLength(0);
    expect(tool.prompt.value).toContain('2. doğrunun ikinci noktasını belirtin');
  });
});

describe('Kesişim noktası: Esc ve Enter', () => {
  it('Esc steps back one answer at a time, then leaves; the method letters work only before anything is given', () => {
    const h = toolHarness();
    const tool = h.use(new IntersectPointTool(h.ctx));
    tool.activate();
    tool.pointerDown(at(0, 0));
    tool.input('5');
    expect(tool.input('D')).toBe(false);
    expect(tool.cancel()).toBe(true);
    expect(tool.prompt.value).toContain('A noktasından uzaklığı yazın');
    expect(tool.cancel()).toBe(true);
    expect(tool.prompt.value).toContain('A noktasını belirtin');
    expect(tool.cancel()).toBe(false);
    tool.confirm();
    expect(h.state.exited).toBe(1);
  });

  it('writes nothing on a locked layer', () => {
    const h = toolHarness();
    h.doc.layers.active.set('kilitli');
    const tool = h.use(new IntersectPointTool(h.ctx));
    tool.activate();
    tool.input('0,0');
    tool.input('5');
    tool.input('10,0');
    tool.input('5');
    tool.confirm();
    expect(points(h)).toHaveLength(0);
    expect(h.said().at(-1)).toMatch(/kilitli/);
  });
});
