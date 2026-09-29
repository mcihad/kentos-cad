import { afterEach, describe, expect, it } from 'vitest';
import { PickIndex } from '../viewport/picking';
import { parsePrompt } from '../ui/promptOptions';
import { CircleSelectTool, ContainingSelectTool, FenceSelectTool } from './selectTools';
import { at, canvasLog, pt, toolHarness } from './toolHarness';

/**
 * Çitle seç, Daireyle seç and İçeren alanı seç (docs/adr/0141), on the hand-worked scenes of the core's
 * tests (crates/shared/geometry-core/src/store/select.rs): the prompts and chips, what each puts in
 * the selection and says, Shift adding, going back to Seç, and the previews. The store is the real one.
 */

const stores: PickIndex[] = [];
afterEach(() => stores.splice(0).forEach((s) => s.dispose()));

/** The harness's drawing with the geometry store behind `ctx.view`, as the viewport answers the queries. */
function harness() {
  const h = toolHarness();
  const picker = new PickIndex(h.doc);
  stores.push(picker);
  const tol = () => h.ctx.view.worldTolerance(h.ctx.prefs.pickAperture.value);
  Object.assign(h.ctx.view, {
    inFence: (fence: readonly { x: number; y: number }[]) => picker.inFence(fence, tol()),
    inCircle: (c: { x: number; y: number }, r: number, crossing: boolean) => picker.inCircle(c, r, crossing),
    containing: (p: { x: number; y: number }) => picker.containing(p),
  });
  h.ctx.prefs.pickAperture.set(0.1);
  return h;
}
type Harness = ReturnType<typeof harness>;

const square = (h: Harness, x: number, y: number, s: number, extra: object = {}) =>
  h.add({ kind: 'polygon', pts: [pt(x, y), pt(x + s, y), pt(x + s, y + s), pt(x, y + s)], ...extra });
const line = (h: Harness, a: [number, number], b: [number, number]) => h.add({ kind: 'line', a: pt(...a), b: pt(...b) });
const ids = (h: Harness) => [...h.ctx.selection.ids.value];
const keys = (tool: { prompt: { value: string } }) => parsePrompt(tool.prompt.value).options.map((o) => o.key);
const last = (h: Harness) => h.log.entries.value.at(-1)!;

/** Three lines across the fence's way, a square it crosses, a spot near it and one far from it, and a line beyond its end. */
function fenceScene(h: Harness) {
  const l1 = line(h, [0, 0], [0, 10]);
  const l2 = line(h, [5, 0], [5, 10]);
  const l3 = line(h, [20, 0], [20, 10]);
  const s4 = square(h, 8, 4, 2);
  const p5 = h.add({ kind: 'point', p: pt(12, 5.05) });
  const p6 = h.add({ kind: 'point', p: pt(12, 7) });
  return { l1, l2, l3, s4, p5, p6 };
}

describe('Çitle seç', () => {
  it('asks for the first point, then the next, with Geri and Bitir', () => {
    const h = harness();
    const tool = h.use(new FenceSelectTool(h.ctx));
    tool.activate();
    expect(tool.prompt.value).toBe('Çitle seç: çitin ilk noktasına tıklayın');
    expect(keys(tool)).toEqual([]);
    tool.pointerDown(at(-1, 5));
    expect(tool.prompt.value).toBe('Çitle seç: sonraki noktaya tıklayın [Geri (G) / Bitir (Enter)]');
    expect(keys(tool)).toEqual(['G', 'Enter']);
  });

  it('selects what the fence crosses, a point within the aperture too, and says how many', () => {
    const h = harness();
    const s = fenceScene(h);
    const tool = h.use(new FenceSelectTool(h.ctx));
    tool.activate();
    tool.pointerDown(at(-1, 5));
    tool.pointerDown(at(14, 5));
    tool.confirm();
    // The lines and the square it crosses and the spot 5 cm off it; not the spot 2 m off, nor the line past its end.
    expect(ids(h)).toEqual([s.l1.id, s.l2.id, s.s4.id, s.p5.id]);
    expect(last(h)).toMatchObject({ level: 'info', text: 'Çit 4 nesneyi kesti; seçildi.' });
  });

  it('goes back to Seç when it ends', () => {
    const h = harness();
    fenceScene(h);
    const tool = h.use(new FenceSelectTool(h.ctx));
    tool.activate();
    tool.pointerDown(at(-1, 5));
    tool.pointerDown(at(14, 5));
    expect(h.state.exited).toBe(0);
    tool.confirm();
    expect(h.state.exited).toBe(1);
  });

  it('replaces the selection, or adds to it with Shift on the pointer', () => {
    const h = harness();
    const s = fenceScene(h);
    h.ctx.selection.set([s.l3.id]);
    const replace = h.use(new FenceSelectTool(h.ctx));
    replace.activate();
    replace.pointerDown(at(-1, 5));
    replace.pointerDown(at(6, 5));
    replace.confirm();
    expect(ids(h)).toEqual([s.l1.id, s.l2.id]);
    h.ctx.selection.set([s.l3.id]);
    const add = h.use(new FenceSelectTool(h.ctx));
    add.activate();
    add.pointerDown(at(-1, 5));
    add.pointerDown(at(6, 5, { shift: true }));
    add.confirm();
    expect(ids(h)).toEqual([s.l3.id, s.l1.id, s.l2.id]);
  });

  it('remembers Shift from the last click, whatever the mouse does before it ends', () => {
    const h = harness();
    const s = fenceScene(h);
    h.ctx.selection.set([s.l3.id]);
    const tool = h.use(new FenceSelectTool(h.ctx));
    tool.activate();
    tool.pointerDown(at(-1, 5));
    tool.pointerDown(at(6, 5, { shift: true }));
    // Shift let go and the mouse moved on before the right click.
    tool.pointerMove(at(7, 5));
    tool.confirm();
    expect(ids(h)).toEqual([s.l3.id, s.l1.id, s.l2.id]);
    // A fence clicked without it replaces, and a mouse move with Shift held does not make it add.
    h.ctx.selection.set([s.l3.id]);
    const plain = h.use(new FenceSelectTool(h.ctx));
    plain.activate();
    plain.pointerDown(at(-1, 5));
    plain.pointerDown(at(6, 5));
    plain.pointerMove(at(7, 5, { shift: true }));
    plain.confirm();
    expect(ids(h)).toEqual([s.l1.id, s.l2.id]);
  });

  it('adds with Shift+Enter', () => {
    const h = harness();
    const s = fenceScene(h);
    h.ctx.selection.set([s.l3.id]);
    const tool = h.use(new FenceSelectTool(h.ctx));
    tool.activate();
    tool.pointerDown(at(-1, 5));
    tool.pointerDown(at(6, 5));
    tool.confirm({ shift: true });
    expect(ids(h)).toEqual([s.l3.id, s.l1.id, s.l2.id]);
  });

  it('says so when the fence crosses nothing, and leaves the selection as it was', () => {
    const h = harness();
    const s = fenceScene(h);
    h.ctx.selection.set([s.l3.id]);
    const tool = h.use(new FenceSelectTool(h.ctx));
    tool.activate();
    tool.pointerDown(at(30, 5));
    tool.pointerDown(at(40, 5));
    tool.confirm();
    expect(last(h)).toMatchObject({ level: 'warn', text: 'Çit hiçbir nesneyi kesmedi.' });
    expect(ids(h)).toEqual([s.l3.id]);
    expect(h.state.exited).toBe(1);
  });

  it('takes only visible objects', () => {
    const h = harness();
    const s = fenceScene(h);
    h.doc.layers.setVisible('yol', false);
    // A line on the hidden layer across the fence.
    h.add({ kind: 'line', layerId: 'yol', a: pt(2, 0), b: pt(2, 10) });
    const tool = h.use(new FenceSelectTool(h.ctx));
    tool.activate();
    tool.pointerDown(at(-1, 5));
    tool.pointerDown(at(6, 5));
    tool.confirm();
    expect(ids(h)).toEqual([s.l1.id, s.l2.id]);
  });

  it('takes typed points like clicks', () => {
    const h = harness();
    const s = fenceScene(h);
    const tool = h.use(new FenceSelectTool(h.ctx));
    tool.activate();
    expect(tool.input('-1,5')).toBe(true);
    expect(tool.input('6,5')).toBe(true);
    tool.confirm();
    expect(ids(h)).toEqual([s.l1.id, s.l2.id]);
  });

  it('Geri drops the last point, and so does Ctrl+Z', () => {
    const h = harness();
    const tool = h.use(new FenceSelectTool(h.ctx));
    tool.activate();
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(5, 5));
    tool.pointerDown(at(9, 0));
    expect(tool.pointCount).toBe(3);
    expect(tool.input('G')).toBe(true);
    expect(tool.pointCount).toBe(2);
    expect(tool.undoStep()).toBe(true);
    expect(tool.pointCount).toBe(1);
    expect(tool.input('G')).toBe(true);
    expect(tool.prompt.value).toBe('Çitle seç: çitin ilk noktasına tıklayın');
    expect(tool.input('G')).toBe(false);
  });

  it('asks for a second point before it ends, and stays', () => {
    const h = harness();
    const tool = h.use(new FenceSelectTool(h.ctx));
    tool.activate();
    tool.pointerDown(at(0, 0));
    tool.confirm();
    expect(last(h).level).toBe('warn');
    expect(h.state.exited).toBe(0);
    expect(tool.pointCount).toBe(1);
  });

  it('leaves at once on Enter without a fence', () => {
    const h = harness();
    const tool = h.use(new FenceSelectTool(h.ctx));
    tool.activate();
    tool.confirm();
    expect(h.state.exited).toBe(1);
  });

  it('never writes to the drawing', () => {
    const h = harness();
    fenceScene(h);
    const revision = h.doc.revision;
    const tool = h.use(new FenceSelectTool(h.ctx));
    tool.activate();
    tool.pointerDown(at(-1, 5));
    tool.pointerDown(at(14, 5));
    tool.confirm();
    expect(h.doc.revision).toBe(revision);
  });

  it('previews the fence as a dashed path to the cursor', () => {
    const h = harness();
    const tool = h.use(new FenceSelectTool(h.ctx));
    tool.activate();
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(10, 0));
    tool.pointerMove(at(10, 10));
    const { g, view, paths } = canvasLog();
    tool.draw(g, view);
    expect(paths.filter((p) => p.dashed).map((p) => p.points)).toEqual([[[0, 0], [10, 0], [10, 10]]]);
  });
});

/** A small square, a line through the middle and a spot in it, a very large square around, and a spot far away. */
function circleScene(h: Harness) {
  const small = square(h, -1, -1, 2);
  const through = line(h, [0, 0], [20, 0]);
  const spot = h.add({ kind: 'point', p: pt(3, 0) });
  const huge = square(h, -100, -100, 200);
  const far = h.add({ kind: 'point', p: pt(30, 30) });
  return { small, through, spot, huge, far };
}

describe('Daireyle seç', () => {
  it('asks for the centre, then the radius, with Kesişen at both', () => {
    const h = harness();
    const tool = h.use(new CircleSelectTool(h.ctx));
    tool.activate();
    if (tool.prompt.value.includes(': açık')) tool.input('K');
    expect(tool.prompt.value).toBe('Daireyle seç: dairenin merkezine tıklayın [Kesişen (K)]');
    expect(keys(tool)).toEqual(['K']);
    tool.pointerDown(at(0, 0));
    expect(tool.prompt.value).toBe('Daireyle seç: yarıçapı gösterin ya da yazın [Kesişen (K)]');
    expect(keys(tool)).toEqual(['K']);
    tool.input('K');
    expect(tool.prompt.value).toBe('Daireyle seç: yarıçapı gösterin ya da yazın [Kesişen (K): açık]');
    expect(parsePrompt(tool.prompt.value).options[0]).toMatchObject({ label: 'Kesişen', key: 'K', value: 'açık' });
    tool.input('K');
  });

  it('selects what lies inside, and says how many', () => {
    const h = harness();
    const s = circleScene(h);
    const tool = h.use(new CircleSelectTool(h.ctx));
    tool.activate();
    if (tool.prompt.value.includes(': açık')) tool.input('K');
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(5, 0));
    expect(ids(h)).toEqual([s.small.id, s.spot.id]);
    expect(last(h)).toMatchObject({ level: 'info', text: 'Dairenin içinde 2 nesne; seçildi.' });
    expect(h.state.exited).toBe(1);
  });

  it('with Kesişen it takes what the circle touches and the closed object around it too', () => {
    const h = harness();
    const s = circleScene(h);
    const tool = h.use(new CircleSelectTool(h.ctx));
    tool.activate();
    if (!tool.prompt.value.includes(': açık')) tool.input('K');
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(5, 0));
    expect(ids(h)).toEqual([s.small.id, s.through.id, s.spot.id, s.huge.id]);
    expect(last(h)).toMatchObject({ level: 'info', text: 'Daireye dokunan 4 nesne; seçildi.' });
    tool.input('K');
  });

  it('says so when the circle holds nothing, and leaves the selection as it was', () => {
    const h = harness();
    const s = circleScene(h);
    h.ctx.selection.set([s.far.id]);
    const tool = h.use(new CircleSelectTool(h.ctx));
    tool.activate();
    if (tool.prompt.value.includes(': açık')) tool.input('K');
    tool.pointerDown(at(60, 0));
    tool.pointerDown(at(62, 0));
    expect(last(h)).toMatchObject({ level: 'warn', text: 'Dairede nesne yok.' });
    expect(ids(h)).toEqual([s.far.id]);
    expect(h.state.exited).toBe(1);
  });

  it('takes a typed radius', () => {
    const h = harness();
    const s = circleScene(h);
    const tool = h.use(new CircleSelectTool(h.ctx));
    tool.activate();
    if (tool.prompt.value.includes(': açık')) tool.input('K');
    tool.pointerDown(at(0, 0));
    expect(tool.input('5')).toBe(true);
    expect(ids(h)).toEqual([s.small.id, s.spot.id]);
  });

  it('takes a typed centre and a typed point of the circle', () => {
    const h = harness();
    const s = circleScene(h);
    const tool = h.use(new CircleSelectTool(h.ctx));
    tool.activate();
    if (tool.prompt.value.includes(': açık')) tool.input('K');
    expect(tool.input('0,0')).toBe(true);
    expect(tool.input('0,5')).toBe(true);
    expect(ids(h)).toEqual([s.small.id, s.spot.id]);
  });

  it('asks again for a radius above zero, and stays', () => {
    const h = harness();
    circleScene(h);
    const tool = h.use(new CircleSelectTool(h.ctx));
    tool.activate();
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(0, 0));
    expect(last(h)).toMatchObject({ level: 'warn', text: 'Yarıçap sıfırdan büyük olmalı.' });
    expect(h.state.exited).toBe(0);
    expect(tool.pointCount).toBe(1);
  });

  it('adds with Shift on the finishing click', () => {
    const h = harness();
    const s = circleScene(h);
    h.ctx.selection.set([s.far.id]);
    const tool = h.use(new CircleSelectTool(h.ctx));
    tool.activate();
    if (tool.prompt.value.includes(': açık')) tool.input('K');
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(5, 0, { shift: true }));
    expect(ids(h)).toEqual([s.far.id, s.small.id, s.spot.id]);
  });

  it('the radius is the true distance: ortho does not bend it', () => {
    const h = harness();
    const near = h.add({ kind: 'point', p: pt(3, 4) });
    const off = h.add({ kind: 'point', p: pt(0, 4.6) });
    h.ctx.settings.ortho.set(true);
    const tool = h.use(new CircleSelectTool(h.ctx));
    tool.activate();
    if (tool.prompt.value.includes(': açık')) tool.input('K');
    tool.pointerDown(at(0, 0));
    // Ortho against the centre would take (3, 4) to (0, 4) and make the radius 4: neither spot would be inside.
    // The radius is the 5 m to (3, 4), which holds both.
    tool.pointerDown(at(3, 4));
    expect(ids(h)).toEqual([near.id, off.id]);
    h.ctx.settings.ortho.set(false);
  });

  it('keeps Kesişen for the session', () => {
    const h = harness();
    const first = h.use(new CircleSelectTool(h.ctx));
    first.activate();
    if (!first.prompt.value.includes(': açık')) first.input('K');
    const next = new CircleSelectTool(h.ctx);
    next.activate();
    expect(next.prompt.value).toContain('Kesişen (K): açık');
    next.input('K');
    const after = new CircleSelectTool(h.ctx);
    after.activate();
    expect(after.prompt.value).not.toContain(': açık');
  });

  it('previews a dashed circle to the cursor with its radius in the tag', () => {
    const h = harness();
    const tool = h.use(new CircleSelectTool(h.ctx));
    tool.activate();
    if (tool.prompt.value.includes(': açık')) tool.input('K');
    tool.pointerDown(at(10, 10));
    tool.pointerMove(at(10, 17));
    const { g, view, texts, arcs } = canvasLog();
    tool.draw(g, view);
    expect(texts).toEqual(['R 7.000 m']);
    expect(arcs).toContainEqual([10, 10]);
  });
});

/** The ADR's three: a parcel in a block in a district, and a fourth square off to the side. */
function nested(h: Harness) {
  const parcel = square(h, 0, 0, 10);
  const block = square(h, -10, -10, 40);
  const district = square(h, -50, -50, 100);
  const aside = square(h, 200, 200, 5);
  return { parcel, block, district, aside };
}

describe('İçeren alanı seç', () => {
  it('asks for a click inside an area', () => {
    const h = harness();
    const tool = h.use(new ContainingSelectTool(h.ctx));
    expect(tool.prompt.value).toBe('İçeren alanı seç: alanın içine tıklayın');
    expect(keys(tool)).toEqual([]);
  });

  it('selects the smallest area around the click, and says which of how many', () => {
    const h = harness();
    const s = nested(h);
    const tool = h.use(new ContainingSelectTool(h.ctx));
    tool.pointerDown(at(5, 5));
    expect(ids(h)).toEqual([s.parcel.id]);
    expect(last(h)).toMatchObject({ level: 'info', text: 'Alan seçildi (1/3, 100.00 m²).' });
  });

  it('the same place again moves to the next larger one, and wraps after the last', () => {
    const h = harness();
    const s = nested(h);
    const tool = h.use(new ContainingSelectTool(h.ctx));
    const said: string[] = [];
    const seen: number[][] = [];
    for (const p of [pt(5, 5), pt(5.05, 5), pt(5, 5.05), pt(5.02, 5), pt(5, 5)]) {
      tool.pointerDown(at(p.x, p.y));
      said.push(last(h).text);
      seen.push(ids(h));
    }
    expect(seen).toEqual([[s.parcel.id], [s.block.id], [s.district.id], [s.parcel.id], [s.block.id]]);
    expect(said).toEqual([
      'Alan seçildi (1/3, 100.00 m²).',
      'Alan seçildi (2/3, 1600.00 m²).',
      'Alan seçildi (3/3, 10000.00 m²).',
      'Alan seçildi (1/3, 100.00 m²).',
      'Alan seçildi (2/3, 1600.00 m²).',
    ]);
  });

  it('another place starts at the smallest again', () => {
    const h = harness();
    const s = nested(h);
    const tool = h.use(new ContainingSelectTool(h.ctx));
    tool.pointerDown(at(5, 5));
    tool.pointerDown(at(5, 5));
    expect(ids(h)).toEqual([s.block.id]);
    // Far beyond the aperture: a click of its own, inside the block only.
    tool.pointerDown(at(-5, -5));
    expect(ids(h)).toEqual([s.block.id]);
    expect(last(h).text).toBe('Alan seçildi (1/2, 1600.00 m²).');
    tool.pointerDown(at(5, 5));
    expect(ids(h)).toEqual([s.parcel.id]);
  });

  it('says so when no closed area is around the click, and leaves the selection as it was', () => {
    const h = harness();
    const s = nested(h);
    h.ctx.selection.set([s.aside.id]);
    const tool = h.use(new ContainingSelectTool(h.ctx));
    tool.pointerDown(at(500, 500));
    expect(last(h)).toMatchObject({ level: 'warn', text: 'Tıklanan noktayı içeren kapalı alan yok.' });
    expect(ids(h)).toEqual([s.aside.id]);
  });

  it('adds to the selection with Shift', () => {
    const h = harness();
    const s = nested(h);
    h.ctx.selection.set([s.aside.id]);
    const tool = h.use(new ContainingSelectTool(h.ctx));
    tool.pointerDown(at(5, 5, { shift: true }));
    expect(ids(h)).toEqual([s.aside.id, s.parcel.id]);
    // Cycling with Shift adds each one it reaches.
    tool.pointerDown(at(5, 5, { shift: true }));
    expect(ids(h)).toEqual([s.aside.id, s.parcel.id, s.block.id]);
  });

  it('stays for more clicks, and ends on Enter or a right click', () => {
    const h = harness();
    nested(h);
    const tool = h.use(new ContainingSelectTool(h.ctx));
    tool.pointerDown(at(5, 5));
    tool.pointerDown(at(220, 202));
    expect(h.state.exited).toBe(0);
    tool.confirm();
    expect(h.state.exited).toBe(1);
  });

  it('an area with a hole does not hold a click in the hole', () => {
    const h = harness();
    const parcel = h.add({ kind: 'polygon', pts: [pt(0, 0), pt(10, 0), pt(10, 10), pt(0, 10)], holes: [{ pts: [pt(4, 4), pt(6, 4), pt(6, 6), pt(4, 6)] }] });
    const block = square(h, -10, -10, 40);
    const tool = h.use(new ContainingSelectTool(h.ctx));
    // The area is its 100 m² less the 4 m² hole.
    tool.pointerDown(at(1, 1));
    expect(ids(h)).toEqual([parcel.id]);
    expect(last(h).text).toBe('Alan seçildi (1/2, 96.00 m²).');
    tool.pointerDown(at(5, 5));
    expect(ids(h)).toEqual([block.id]);
    expect(last(h).text).toBe('Alan seçildi (1/1, 1600.00 m²).');
  });

  it('takes a circle as an area too', () => {
    const h = harness();
    const circle = h.add({ kind: 'circle', c: pt(0, 0), r: 5 });
    const tool = h.use(new ContainingSelectTool(h.ctx));
    tool.pointerDown(at(1, 1));
    expect(ids(h)).toEqual([circle.id]);
    expect(last(h).text).toBe(`Alan seçildi (1/1, ${(Math.PI * 25).toFixed(2)} m²).`);
  });

  it('takes a typed point as a click', () => {
    const h = harness();
    const s = nested(h);
    const tool = h.use(new ContainingSelectTool(h.ctx));
    expect(tool.input('5,5')).toBe(true);
    expect(ids(h)).toEqual([s.parcel.id]);
    expect(tool.input('bir şey')).toBe(false);
  });

  it('shows the rank and the area as a tag at the click', () => {
    const h = harness();
    nested(h);
    const tool = h.use(new ContainingSelectTool(h.ctx));
    const none = canvasLog();
    tool.draw(none.g, none.view);
    expect(none.texts).toEqual([]);
    tool.pointerDown(at(5, 5));
    tool.pointerDown(at(5, 5));
    const { g, view, texts } = canvasLog();
    tool.draw(g, view);
    expect(texts).toEqual(['2/3 · 1600.00 m²']);
  });

  it('never writes to the drawing', () => {
    const h = harness();
    nested(h);
    const revision = h.doc.revision;
    const tool = h.use(new ContainingSelectTool(h.ctx));
    tool.pointerDown(at(5, 5));
    tool.pointerDown(at(5, 5));
    expect(h.doc.revision).toBe(revision);
  });
});
