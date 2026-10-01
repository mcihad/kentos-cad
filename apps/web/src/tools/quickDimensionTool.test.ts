import { describe, expect, it } from 'vitest';
import type { DimensionEntity } from '../model/entities';
import { QuickDimensionTool } from './quickDimensionTool';
import { at, pt, recorder, toolHarness } from './toolHarness';

/**
 * Hızlı ölçü (docs/adr/0147 §7) through the tool: the selection first, then where the dimensions go or their distance
 * typed, all in one step; as the desktop's crates/native/interaction/tests/quick_dimension.rs walks it. Values worked
 * out by hand; what each edge gets is the core's, checked against the independent reference in dimension.test.ts.
 */
const dims = (h: ReturnType<typeof toolHarness>) => [...h.doc.all()].filter((e): e is DimensionEntity => e.kind === 'dimension');
const near = (a: number, b: number) => Math.abs(a - b) < 1e-9;

/** Two parcels side by side, sharing their 30 m edge, and a road edge: 10 m straight, then a quarter arc of 10 m. */
function ground() {
  const h = toolHarness();
  const west = h.add({ kind: 'polygon', pts: [pt(0, 0), pt(20, 0), pt(20, 30), pt(0, 30)] });
  const east = h.add({ kind: 'polygon', pts: [pt(20, 0), pt(40, 0), pt(40, 30), pt(20, 30)] });
  const road = h.add({ kind: 'polyline', pts: [pt(0, -10), pt(10, -10), pt(20, -20)], bulges: [0, -Math.tan(Math.PI / 8)] });
  const spot = h.add({ kind: 'point', p: pt(50, 50) });
  return { h, west, east, road, spot };
}

describe('Hızlı ölçü', () => {
  it('measures every edge of the selection at once, the shared edge once, out of the areas, by the cursor', () => {
    const { h, west, east, road, spot } = ground();
    h.ctx.selection.set([spot.id, east.id, west.id, road.id]);
    const tool = h.use(new QuickDimensionTool(h.ctx));
    expect(tool.snaps).toBe(false);
    tool.activate();
    expect(tool.prompt.value).toBe('Hızlı ölçü: ölçülerin yerini gösterin ya da uzaklık yazın [Zemin (Z): kapalı]');
    // 4 m over the parcels' north edge: the nearest edge.
    tool.pointerMove(at(10, 34));
    const r = recorder();
    tool.draw(r.g, r.view);
    expect(r.calls).toContain('stroke');
    tool.pointerDown(at(10, 34));
    const made = dims(h);
    // West 4 edges, east 3 (their shared edge once), the road's straight part and its arc.
    expect(made).toHaveLength(9);
    expect(h.said().at(-1)).toBe('Hızlı ölçü: 9 ölçü eklendi; 1 nesne atlandı (yalnız çizgi, çoklu çizgi ve alan ölçülür).');
    expect(h.state.exited).toBe(1);
    // Out of the west parcel (counter-clockwise: to the right of its edges).
    expect(made[0]).toMatchObject({ a: pt(0, 0), b: pt(20, 0), offset: -4, height: 2.5 });
    expect(made[0].style).toBeUndefined();
    // The road: its side is the cursor's, left of its way (north, as the cursor is).
    const [straight, arc] = made.slice(-2);
    expect(straight).toMatchObject({ a: pt(0, -10), b: pt(10, -10), offset: 4 });
    // The clockwise arc about (10, −20): its ends counter-clockwise, its dimension outwards (its left).
    expect(arc).toMatchObject({ style: 'arcLength', a: pt(20, -20), b: pt(10, -10), offset: 4 });
    expect(near(arc.c!.x, 10) && near(arc.c!.y, -20)).toBe(true);
    // One step.
    expect(h.doc.undo()).toBe('Ekle');
    expect(dims(h)).toHaveLength(0);
  });

  it('takes a typed distance, and Zemin writes the values over the background', () => {
    const { h, west } = ground();
    h.ctx.selection.set([west.id]);
    const tool = h.use(new QuickDimensionTool(h.ctx));
    tool.activate();
    expect(tool.input('Z')).toBe(true);
    expect(tool.prompt.value.endsWith('[Zemin (Z): açık]')).toBe(true);
    // Not a distance: refused.
    expect(tool.input('12,5')).toBe(false);
    tool.pointerMove(at(10, 15));
    expect(tool.input('-2.5')).toBe(true);
    const made = dims(h);
    expect(made).toHaveLength(4);
    expect(made.every((d) => d.offset === -2.5 && d.mask === true)).toBe(true);
    expect(h.said().at(-1)).toBe('Hızlı ölçü: 4 ölçü eklendi.');
    // Zemin off again: the memory as found.
    const again = h.use(new QuickDimensionTool(h.ctx));
    again.activate();
    expect(again.input('Z')).toBe(true);
    expect(again.prompt.value.endsWith('[Zemin (Z): kapalı]')).toBe(true);
  });

  it('picks first without a selection; with nothing to measure it says so and leaves', async () => {
    const { h, spot } = ground();
    const tool = h.use(new QuickDimensionTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value).toBe('Hızlı ölçü: nesnelere tıklayın ya da pencereyle seçin, bitince sağ tıklayın (0 seçili)');
    h.ctx.selection.set([spot.id]);
    tool.confirm();
    expect(h.said().at(-1)).toBe('Seçimde ölçülecek çizgi, çoklu çizgi ya da alan yok.');
    await Promise.resolve();
    expect(h.state.exited).toBe(1);
    expect(dims(h)).toHaveLength(0);
  });

  it('a selection on a locked layer is measured; the dimensions go on the active layer', () => {
    const { h, west } = ground();
    const locked = h.add({ kind: 'line', a: pt(0, 50), b: pt(30, 50), layerId: 'kilitli' });
    h.ctx.selection.set([west.id, locked.id]);
    const tool = h.use(new QuickDimensionTool(h.ctx));
    tool.activate();
    tool.pointerDown(at(15, 53));
    const made = dims(h);
    expect(made).toHaveLength(5);
    expect(made.every((d) => d.layerId === h.doc.layers.active.value)).toBe(true);
  });
});
