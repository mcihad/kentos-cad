import { beforeEach, describe, expect, it } from 'vitest';
import type { ArcEntity, Entity, LineEntity, PolylineEntity } from '../model/entities';
import { at, canvasLog, pt, toolHarness } from './toolHarness';
import { FIRST_TOLERANCE, FIRST_WORKS, geometryOf, topoObject, TopologyTool } from './topologyTool';

/**
 * Topolojik temizlik (docs/adr/0148 §9): the scope taken when it starts, the finding shown first, the tolerance
 * typed and the works turned on and off, one step written. The cases are worked out by hand from the rules of
 * §4–§6; the core itself is checked against the independent reference in model/ops/topology.test.ts. The desktop
 * walks the same in crates/native/interaction/tests/all/topology.rs.
 */

const OPTIONS = (tolerance: string, works = 'açık / Köşeler (K): kapalı / Uzat (Z): açık / Buda (B): açık') =>
  `[Tolerans (T): ${tolerance} / Uçlar (U): ${works} / Uygula (Enter)]`;

beforeEach(() => {
  TopologyTool.tolerance = FIRST_TOLERANCE;
  TopologyTool.works = { ...FIRST_WORKS };
});

const line = (e: Entity | undefined) => e as LineEntity;

describe('Topolojik temizlik', () => {
  it('joins the ends within the tolerance on the whole drawing, an end at a point taking its elevation, in one step', () => {
    const h = toolHarness();
    const a = h.add({ kind: 'line', a: pt(0, 0), b: pt(10, 0) });
    const b = h.add({ kind: 'line', a: pt(10.006, 0.004), b: pt(20, 0) });
    h.add({ kind: 'point', p: pt(20, 10), z: 105 });
    const c = h.add({ kind: 'line', a: pt(20, 0), b: pt(20.004, 9.997), za: 100, zb: 104 });
    const tool = h.use(new TopologyTool(h.ctx));
    tool.activate();
    expect(h.said().at(-1)).toBe('Topolojik temizlik: 2 uç birleşir; en büyük kayma 0.007 m. Enter ile uygulayın (bütün çizim: 3 nesne, 1 dayanak).');
    expect(tool.prompt.value).toBe(`Topolojik temizlik: 2 uç birleşir; en büyük kayma 0.007 m ${OPTIONS('0.010 m')}`);
    // Shown, not written.
    expect(line(h.doc.get(b.id)).a).toEqual(pt(10.006, 0.004));
    tool.pointerMove(at(5, 5));
    const log = canvasLog();
    tool.draw(log.g, log.view);
    // A ring where each end lands and a fine line from where it was, the old outlines dashed under the new ones, the
    // finding beside the cursor.
    expect(log.arcs).toContainEqual([10, 0]);
    expect(log.arcs).toContainEqual([20, 10]);
    expect(log.arcs).toContainEqual([10.006, 0.004]);
    expect(log.paths.filter((p) => p.dashed)).toHaveLength(4);
    expect(log.paths).toContainEqual({ points: [[10.006, 0.004], [10, 0]], dashed: true });
    expect(log.texts).toEqual(['2 uç birleşir', 'en büyük kayma 0.007 m', 'Enter: uygula']);
    tool.confirm();
    // The second line's start goes to the first's end (the drawing's order breaks the tie); the third's end to the point.
    expect(line(h.doc.get(b.id))).toMatchObject({ a: pt(10, 0), b: pt(20, 0) });
    expect(line(h.doc.get(b.id)).za).toBeUndefined();
    expect(line(h.doc.get(c.id))).toMatchObject({ a: pt(20, 0), b: pt(20, 10), za: 100, zb: 105 });
    expect(line(h.doc.get(a.id))).toMatchObject({ a: pt(0, 0), b: pt(10, 0) });
    expect(h.said().at(-1)).toBe('Topolojik temizlik: 2 uç birleşti; 2 nesne değişti, en büyük kayma 0.007 m.');
    expect(h.state.exited).toBe(1);
    expect(h.doc.undo()).toBe('Topolojik temizlik');
    expect(line(h.doc.get(c.id))).toMatchObject({ b: pt(20.004, 9.997), zb: 104 });
  });

  it('takes a typed tolerance, turns the works on and off, and extends and trims what stops short or runs past', () => {
    const h = toolHarness();
    h.add({ kind: 'line', a: pt(0, 0), b: pt(20, 0) });
    const short = h.add({ kind: 'line', a: pt(5, 10), b: pt(5, 0.03) });
    const past = h.add({ kind: 'line', a: pt(10, 10), b: pt(10, -0.02) });
    const far = h.add({ kind: 'line', a: pt(15, 10), b: pt(15, 0.2) });
    const tool = h.use(new TopologyTool(h.ctx));
    tool.activate();
    expect(h.said().at(-1)).toBe('Topolojik temizlik: 0.010 m toleransla düzeltilecek bir şey yok; daha büyük bir tolerans yazın (bütün çizim: 4 nesne, 0 dayanak).');
    expect(tool.prompt.value).toBe(`Topolojik temizlik: düzeltilecek bir şey yok ${OPTIONS('0.010 m')}`);
    expect(tool.input('0.05')).toBe(true);
    expect(tool.prompt.value).toBe(`Topolojik temizlik: 1 uç uzar, 1 uç kısalır; en büyük kayma 0.030 m ${OPTIONS('0.050 m')}`);
    // Each new finding is said: the history and the status bar keep the current one.
    expect(h.said().at(-1)).toBe('Topolojik temizlik: 1 uç uzar, 1 uç kısalır; en büyük kayma 0.030 m.');
    // Uzat off: the end that stops short goes onto the nearest line instead.
    tool.input('z');
    expect(tool.prompt.value).toBe(`Topolojik temizlik: 1 uç kısalır, 1 uç kenara taşınır; en büyük kayma 0.030 m ${OPTIONS('0.050 m', 'açık / Köşeler (K): kapalı / Uzat (Z): kapalı / Buda (B): açık')}`);
    tool.input('U');
    expect(tool.prompt.value).toMatch(/^Topolojik temizlik: 1 uç kısalır; en büyük kayma 0.020 m \[/);
    tool.input('B');
    expect(tool.prompt.value).toBe(`Topolojik temizlik: düzeltilecek bir şey yok ${OPTIONS('0.050 m', 'kapalı / Köşeler (K): kapalı / Uzat (Z): kapalı / Buda (B): kapalı')}`);
    expect(h.said().at(-1)).toBe('Topolojik temizlik: 0.050 m toleransla düzeltilecek bir şey yok.');
    const said = h.said().length;
    // T asks for the tolerance; Esc goes back, keeping it, and the finding is not said again.
    tool.input('T');
    expect(tool.prompt.value).toBe('Topolojik temizlik: toleransı yazın, metre (Enter: 0.050 m)');
    expect(tool.cancel()).toBe(true);
    expect(tool.prompt.value).toMatch(/^Topolojik temizlik: düzeltilecek bir şey yok \[Tolerans \(T\): 0.050 m/);
    expect(h.said()).toHaveLength(said);
    // Below a micrometre: said, the tolerance kept; not a number: not taken.
    expect(tool.input('0')).toBe(true);
    expect(h.said().at(-1)).toBe('Tolerans en az 0.000001 m olmalı.');
    expect(tool.input('abc')).toBe(false);
    expect(TopologyTool.tolerance).toBe(0.05);
    for (const k of ['U', 'Z', 'B']) tool.input(k);
    tool.confirm();
    expect(line(h.doc.get(short.id))).toMatchObject({ a: pt(5, 10), b: pt(5, 0) });
    expect(line(h.doc.get(past.id))).toMatchObject({ a: pt(10, 10), b: pt(10, 0) });
    expect(line(h.doc.get(far.id)).b).toEqual(pt(15, 0.2));
    expect(h.said().at(-1)).toBe('Topolojik temizlik: 1 uç uzadı, 1 uç kısaldı; 2 nesne değişti, en büyük kayma 0.030 m.');
    // The tolerance is kept for the session.
    const again = h.use(new TopologyTool(h.ctx));
    again.activate();
    expect(again.prompt.value).toMatch(/Tolerans \(T\): 0.050 m/);
  });

  it('with a selection corrects only it: the other objects and those on a locked layer are supports', () => {
    const h = toolHarness();
    const a = h.add({ kind: 'line', a: pt(0, 0), b: pt(10, 0) });
    const b = h.add({ kind: 'line', a: pt(10.004, 0), b: pt(20, 0) });
    const c = h.add({ kind: 'line', layerId: 'kilitli', a: pt(20.003, 0), b: pt(30, 0) });
    const t = h.add({ kind: 'text', p: pt(5, 5), text: 'Ada 101', height: 1, rotation: 0 });
    h.ctx.selection.set([b.id, c.id, t.id]);
    const tool = h.use(new TopologyTool(h.ctx));
    tool.activate();
    expect(h.said()).toEqual([
      '1 nesne kilitli katmanda olduğu için düzeltilmez; dayanak olarak kalır.',
      '1 nesne topolojik temizliğe katılmaz: yazı, ölçü, tarama, blok, kılavuz ve yardımcı çizgiler girmez.',
      'Topolojik temizlik: 2 uç birleşir; en büyük kayma 0.004 m. Enter ile uygulayın (seçili 1 nesne, 2 dayanak).',
    ]);
    tool.confirm();
    expect(line(h.doc.get(b.id))).toMatchObject({ a: pt(10, 0), b: pt(20.003, 0) });
    expect(line(h.doc.get(a.id)).b).toEqual(pt(10, 0));
    expect(line(h.doc.get(c.id)).a).toEqual(pt(20.003, 0));
  });

  it('leaves what is on a hidden layer out, as a support too', () => {
    const h = toolHarness();
    h.add({ kind: 'line', a: pt(0, 0), b: pt(10, 0) });
    h.add({ kind: 'line', layerId: 'yol', a: pt(10.004, 0), b: pt(20, 0) });
    h.doc.layers.setVisible('yol', false);
    const tool = h.use(new TopologyTool(h.ctx));
    tool.activate();
    expect(h.said().at(-1)).toMatch(/^Topolojik temizlik: 0.010 m toleransla düzeltilecek bir şey yok; .* \(bütün çizim: 1 nesne, 0 dayanak\)\.$/);
    // Nothing to write: said, and the tool leaves.
    tool.confirm();
    expect(h.said().at(-1)).toBe('Topolojik temizlik: düzeltilecek bir şey yok; hiçbir şey değişmedi.');
    expect(h.state.exited).toBe(1);
    expect(line([...h.doc.all()][1]).a).toEqual(pt(10.004, 0));
  });

  it('with nothing it corrects says so and leaves', async () => {
    const h = toolHarness();
    h.add({ kind: 'point', p: pt(0, 0) });
    h.add({ kind: 'circle', c: pt(5, 5), r: 2 });
    const tool = h.use(new TopologyTool(h.ctx));
    tool.activate();
    expect(h.said().at(-1)).toBe('Topolojik temizlik: düzeltilecek çizgi, çoklu çizgi, yay ya da alan yok.');
    await Promise.resolve();
    expect(h.state.exited).toBe(1);
  });

  it('moves an arc’s end keeping its angle (docs/adr/0148 §4.6)', () => {
    const h = toolHarness();
    h.add({ kind: 'line', a: pt(0, 0), b: pt(10, 0) });
    const arc = h.add({ kind: 'arc', c: pt(10.006, 5), r: 5, a0: -Math.PI / 2, a1: 0 });
    const tool = h.use(new TopologyTool(h.ctx));
    tool.activate();
    tool.confirm();
    const e = h.doc.get(arc.id) as ArcEntity;
    const on = (a: number) => pt(e.c.x + e.r * Math.cos(a), e.c.y + e.r * Math.sin(a));
    // Its start is on the line's end now, its end where it was, and it still turns a quarter.
    expect(on(e.a0).x).toBeCloseTo(10, 9);
    expect(on(e.a0).y).toBeCloseTo(0, 9);
    expect(on(e.a1).x).toBeCloseTo(15.006, 9);
    expect(on(e.a1).y).toBeCloseTo(5, 9);
    expect((e.a1 - e.a0 + 2 * Math.PI) % (2 * Math.PI)).toBeCloseTo(Math.PI / 2, 12);
  });

  it('joins the vertices of areas only with Köşeler, the holes kept', () => {
    const h = toolHarness();
    h.add({ kind: 'polygon', pts: [pt(0, 0), pt(10, 0), pt(10, 10), pt(0, 10)] });
    const hole = { pts: [pt(12, 2), pt(18, 2), pt(18, 8), pt(12, 8)] };
    const right = h.add({ kind: 'polygon', pts: [pt(10.004, 0), pt(20, 0), pt(20, 10), pt(10.003, 10)], holes: [hole] });
    const tool = h.use(new TopologyTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value).toMatch(/^Topolojik temizlik: düzeltilecek bir şey yok \[/);
    tool.input('K');
    expect(tool.prompt.value).toBe(`Topolojik temizlik: 2 köşe birleşir; en büyük kayma 0.004 m ${OPTIONS('0.010 m', 'açık / Köşeler (K): açık / Uzat (Z): açık / Buda (B): açık')}`);
    tool.confirm();
    const e = h.doc.get(right.id) as PolylineEntity;
    expect(e.pts).toEqual([pt(10, 0), pt(20, 0), pt(20, 10), pt(10, 10)]);
    expect(e.holes).toEqual([hole]);
  });
});

describe('the objects the cleanup takes', () => {
  const h = toolHarness();
  it('are paths with their elevations, an area ring after ring, and come back as the same geometry', () => {
    const l = h.add({ kind: 'line', a: pt(0, 0), b: pt(1, 0), za: 5 });
    expect(topoObject(l, false)).toEqual({ kind: 'line', fixed: false, paths: [{ pts: [pt(0, 0), pt(1, 0)], closed: false, zs: [5, null] }] });
    // Bulges one a vertex, the missing last one straight.
    const p = h.add({ kind: 'polyline', pts: [pt(0, 0), pt(1, 0), pt(2, 1)], bulges: [0.5, 0], zs: [1, null, 3] });
    expect(topoObject(p, true)?.paths).toEqual([{ pts: [pt(0, 0), pt(1, 0), pt(2, 1)], bulges: [0.5, 0, 0], closed: false, zs: [1, null, 3] }]);
    const area = h.add({
      kind: 'polygon',
      pts: [pt(0, 0), pt(4, 0), pt(4, 4)],
      holes: [{ pts: [pt(1, 1), pt(2, 1), pt(2, 2)] }],
      parts: [{ pts: [pt(10, 0), pt(14, 0), pt(14, 4)], holes: [{ pts: [pt(11, 1), pt(12, 1), pt(12, 2)], zs: [7, 8, 9] }] }],
    });
    const o = topoObject(area, false)!;
    expect(o.kind).toBe('area');
    expect(o.paths.map((q) => q.pts[0])).toEqual([pt(0, 0), pt(1, 1), pt(10, 0), pt(11, 1)]);
    expect(o.paths.every((q) => q.closed)).toBe(true);
    expect(geometryOf(area, o.paths)).toEqual({
      kind: 'polygon',
      pts: [pt(0, 0), pt(4, 0), pt(4, 4)],
      zs: [null, null, null],
      holes: [{ pts: [pt(1, 1), pt(2, 1), pt(2, 2)], zs: [null, null, null] }],
      parts: [{ pts: [pt(10, 0), pt(14, 0), pt(14, 4)], zs: [null, null, null], holes: [{ pts: [pt(11, 1), pt(12, 1), pt(12, 2)], zs: [7, 8, 9] }] }],
    });
    expect(geometryOf(p, topoObject(p, false)!.paths)).toEqual({ kind: 'polyline', pts: [pt(0, 0), pt(1, 0), pt(2, 1)], bulges: [0.5, 0, 0], zs: [1, null, 3] });
  });

  it('points are always fixed; circles, ellipses and curves are edges; texts take no part', () => {
    expect(topoObject(h.add({ kind: 'point', p: pt(3, 4), z: 9 }), false)).toEqual({ kind: 'point', fixed: true, paths: [{ pts: [pt(3, 4)], closed: false, zs: [9] }] });
    expect(topoObject(h.add({ kind: 'circle', c: pt(0, 0), r: 2 }), false)).toEqual({
      kind: 'edges',
      fixed: true,
      paths: [{ pts: [pt(2, 0), pt(-2, 0)], bulges: [1, 1], closed: true, zs: [null, null] }],
    });
    const curve = topoObject(h.add({ kind: 'spline', pts: [pt(0, 0), pt(5, 3), pt(10, 0)], closed: false }), false)!;
    expect(curve.kind).toBe('edges');
    expect(curve.fixed).toBe(true);
    // Chords within 0.1 mm: many of them, each its own two-point path.
    expect(curve.paths.length).toBeGreaterThan(20);
    expect(curve.paths.every((q) => q.pts.length === 2)).toBe(true);
    expect(topoObject(h.add({ kind: 'text', p: pt(0, 0), text: 'A', height: 1, rotation: 0 }), false)).toBeNull();
  });
});
