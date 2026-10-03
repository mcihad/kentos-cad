import { describe, expect, it } from 'vitest';
import { fitPage, MAX_ZOOM, MIN_ZOOM, nextStep, panBy, PX_PER_MM, realSize, toPaper, toScreen, wheelFactor, zoomAt, zoomPercent } from './paperView';
import { labelStep, rulerTicks, stepText } from './rulerTicks';
import { thumbSize } from './thumbPainter';

/**
 * Where the paper lies on the desk (docs/sheet/design.md §11): the page
 * fitted, real size (a millimetre of paper on a 96 dpi screen), zooming about
 * the pointer, panning; the rulers' 1-2-5 marks; a small paper's size.
 */
describe('paper view', () => {
  const A3 = { width: 420, height: 297 };
  const area = { width: 1000, height: 700 };

  it('fits the whole page with a margin, centred', () => {
    const v = fitPage(A3, area, 20);
    expect(v.scale).toBeCloseTo(Math.min(960 / 420, 660 / 297));
    const tl = toScreen(v, { x: 0, y: 0 });
    const br = toScreen(v, { x: 420, y: 297 });
    expect((tl.x + br.x) / 2).toBeCloseTo(500);
    expect((tl.y + br.y) / 2).toBeCloseTo(350);
    expect(Math.min(tl.x, tl.y, 1000 - br.x, 700 - br.y)).toBeGreaterThanOrEqual(20 - 1e-9);
  });

  it('shows real size about the point in the middle of the area', () => {
    const v = fitPage(A3, area);
    const before = toPaper(v, { x: 500, y: 350 });
    const r = realSize(A3, area, v);
    expect(r.scale).toBeCloseTo(96 / 25.4);
    expect(zoomPercent(r)).toBe(100);
    const after = toPaper(r, { x: 500, y: 350 });
    expect(after.x).toBeCloseTo(before.x);
    expect(after.y).toBeCloseTo(before.y);
  });

  it('zooms about the pointer, which stays over the same paper point, within its limits', () => {
    const v = fitPage(A3, area);
    const at = { x: 321, y: 123 };
    const p = toPaper(v, at);
    const z = zoomAt(v, 2.5, at);
    const q = toPaper(z, at);
    expect(q.x).toBeCloseTo(p.x);
    expect(q.y).toBeCloseTo(p.y);
    expect(zoomAt(v, 1e9, at).scale).toBeCloseTo(MAX_ZOOM * PX_PER_MM);
    expect(zoomAt(v, 1e-9, at).scale).toBeCloseTo(MIN_ZOOM * PX_PER_MM);
    expect(panBy(v, 10, -5)).toEqual({ scale: v.scale, x: v.x + 10, y: v.y - 5 });
  });

  it('turns a wheel notch into about 1.2× and steps through the zoom list', () => {
    expect(wheelFactor(-100, 0)).toBeCloseTo(1.2);
    expect(wheelFactor(100, 0)).toBeCloseTo(1 / 1.2);
    expect(wheelFactor(-3, 1)).toBeCloseTo(Math.pow(1.2, 0.99));
    expect(nextStep(1, 1)).toBe(1.5);
    expect(nextStep(1, -1)).toBe(0.75);
    expect(nextStep(0.6, 1)).toBe(0.67);
    expect(nextStep(64, 1)).toBe(MAX_ZOOM);
  });
});

describe('ruler marks', () => {
  it('numbers at a 1-2-5 step that leaves them room', () => {
    expect(labelStep(PX_PER_MM)).toBe(20);
    expect(labelStep(PX_PER_MM * 4)).toBe(5);
    expect(labelStep(0.5)).toBe(200);
    expect(labelStep(100)).toBe(1);
    expect(labelStep(400)).toBe(0.2);
  });

  it('writes the numbers as whole millimetres, or with the decimals a step below one needs', () => {
    expect(stepText(40, 20)).toBe('40');
    expect(stepText(-60, 20)).toBe('-60');
    expect(stepText(1e-12, 20)).toBe('0');
    expect(stepText(0.4, 0.2)).toBe('0.4');
    expect(stepText(0.25, 0.05)).toBe('0.25');
  });

  it('marks numbers, halves and the finest steps between them', () => {
    const ticks = rulerTicks(-5, 45, PX_PER_MM);
    const labelled = ticks.filter((t) => t.level === 2);
    expect(labelled.map((t) => t.label)).toEqual(['0', '20', '40']);
    expect(ticks.filter((t) => t.level === 1).map((t) => t.at)).toEqual([10, 30]);
    // 2 mm apart at real size (7.6 px on screen).
    const fine = ticks.filter((t) => t.level === 0).map((t) => t.at);
    expect(fine[1] - fine[0]).toBeCloseTo(2);
    expect(rulerTicks(10, 5, 1)).toEqual([]);
  });
});

describe('small papers', () => {
  it('fits the paper in its box, its longer side first', () => {
    const landscape = thumbSize({ width: 420, height: 297 }, 100);
    expect([landscape.width, landscape.height]).toEqual([92, 65]);
    expect(landscape.scale).toBeCloseTo(92 / 420);
    const portrait = thumbSize({ width: 210, height: 297 }, 150, 0.92, 100);
    expect(portrait.height).toBe(92);
    expect(portrait.width).toBe(65);
  });
});
