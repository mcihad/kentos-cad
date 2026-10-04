import { afterEach, describe, expect, it } from 'vitest';
import { parsePrompt } from '../ui/promptOptions';
import { AreaMeasureTool, DistanceTool } from './measureTools';
import { at, canvasLog, pt, toolHarness } from './toolHarness';

/**
 * Mesafe ölç's “Sabit ilk nokta” and Alan hesapla's “İçine tıkla” and “Alan olarak çiz” (docs/adr/0141):
 * the prompts and chips, what is written to the log, the preview, the region found by a click,
 * and the one undo step the area is drawn in. The toggles are kept for the session (static), so each
 * test sets them as it needs them.
 */

const keys = (tool: { prompt: { value: string } }) => parsePrompt(tool.prompt.value).options.map((o) => o.key);
const isOn = (tool: { prompt: { value: string } }, chip: string) => tool.prompt.value.includes(`${chip}: açık`);

/** A Mesafe ölç with “Sabit ilk nokta” set as asked. */
function distance(h: ReturnType<typeof toolHarness>, fixed: boolean): DistanceTool {
  const tool = h.use(new DistanceTool(h.ctx));
  tool.activate();
  if (isOn(tool, 'Sabit ilk nokta (S)') !== fixed) expect(tool.input('S')).toBe(true);
  return tool;
}

/** An Alan hesapla with “İçine tıkla” set as asked; its faces released with `stop`. */
function area(h: ReturnType<typeof toolHarness>, inside: boolean): AreaMeasureTool {
  const tool = h.use(new AreaMeasureTool(h.ctx));
  tool.activate();
  if (isOn(tool, 'İçine tıkla (I)') !== inside) expect(tool.input('I')).toBe(true);
  active.push(tool);
  return tool;
}
const active: AreaMeasureTool[] = [];
afterEach(() => active.splice(0).forEach((t) => t.deactivate()));

describe('Mesafe ölç: the chain as it was', () => {
  it('keeps the chain’s options at the prompts the interaction traces read', () => {
    const h = toolHarness();
    const tool = distance(h, false);
    expect(tool.prompt.value).toBe('Mesafe ölç: ilk noktayı belirtin [Sabit ilk nokta (S)]');
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(10, 0));
    // fixtures/interaction/v1/measure-parcel.json: Yay, Uzunluk, Geri, Bitir.
    expect(keys(tool)).toEqual(['Y', 'U', 'İ', 'A', 'G', 'Enter']);
    tool.input('U');
    expect(keys(tool)).toEqual(['G']);
  });

  it('writes the total at the end, as before', () => {
    const h = toolHarness();
    const tool = distance(h, false);
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(10, 0));
    tool.pointerDown(at(10, 5));
    tool.confirm();
    expect(h.said().at(-1)).toBe('Toplam uzunluk 15.000 m (2 kenar)');
    expect(tool.pointCount).toBe(0);
  });
});

describe('Mesafe ölç: Sabit ilk nokta', () => {
  it('offers the chip before the first point, and says when it is on', () => {
    const h = toolHarness();
    const tool = distance(h, true);
    expect(tool.prompt.value).toBe('Mesafe ölç: ilk noktayı belirtin [Sabit ilk nokta (S): açık]');
    expect(keys(tool)).toEqual(['S']);
    expect(parsePrompt(tool.prompt.value).options[0].value).toBe('açık');
    tool.input('S');
    expect(tool.prompt.value).toBe('Mesafe ölç: ilk noktayı belirtin [Sabit ilk nokta (S)]');
  });

  it('measures every new point from the first and writes one line for each click', () => {
    const h = toolHarness();
    const tool = distance(h, true);
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(10, 0));
    tool.pointerDown(at(0, 10));
    tool.pointerDown(at(-10, 0));
    // The bearing (semt) is from north, clockwise, in the project's angle unit (grad here).
    const lines = h.said().filter((t) => /^\d+: /.test(t));
    expect(lines).toEqual(['1: 10.000 m, semt 100.0000 g', '2: 10.000 m, semt 0.0000 g', '3: 10.000 m, semt 300.0000 g']);
    expect(tool.pointCount).toBe(4);
  });

  it('measures from the first point even when the points do not run on from each other', () => {
    const h = toolHarness();
    const tool = distance(h, true);
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(30, 0));
    tool.pointerDown(at(30, 40));
    // A chain would give 40 m for the second leg; from the first point it is the 50 m diagonal.
    expect(h.said().filter((t) => /^\d+: /.test(t))).toEqual(['1: 30.000 m, semt 100.0000 g', '2: 50.000 m, semt 40.9666 g']);
  });

  it('writes no total at the end', () => {
    const h = toolHarness();
    const tool = distance(h, true);
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(10, 0));
    tool.pointerDown(at(10, 10));
    const before = h.said().length;
    tool.confirm();
    expect(h.said().length).toBe(before);
    expect(h.said().some((t) => t.includes('Toplam'))).toBe(false);
    expect(tool.pointCount).toBe(0);
  });

  it('asks for two points before it ends, as the chain does', () => {
    const h = toolHarness();
    const tool = distance(h, true);
    tool.pointerDown(at(0, 0));
    tool.confirm();
    expect(h.said().at(-1)).toBe('Mesafe ölç için en az 2 nokta gerekir.');
  });

  it('shows its own options: the chip on, Geri, then Bitir', () => {
    const h = toolHarness();
    const tool = distance(h, true);
    tool.pointerDown(at(0, 0));
    expect(tool.prompt.value).toBe('Mesafe ölç: sonraki noktayı belirtin [Sabit ilk nokta (S): açık / Geri (G)]');
    tool.pointerDown(at(10, 0));
    expect(keys(tool)).toEqual(['S', 'G', 'Enter']);
  });

  it('has no arcs or lengths: Yay and Uzunluk are not taken', () => {
    const h = toolHarness();
    const tool = distance(h, true);
    tool.pointerDown(at(0, 0));
    expect(tool.input('Y')).toBe(false);
    expect(tool.input('U')).toBe(false);
  });

  it('turning it on leaves the arc mode', () => {
    const h = toolHarness();
    const tool = distance(h, false);
    tool.input('Y');
    tool.input('S');
    expect(tool.prompt.value).toContain('Sabit ilk nokta (S): açık');
    tool.input('S');
  });

  it('is offered where it works: not in the middle of a chain, and off again in the middle of rays', () => {
    const h = toolHarness();
    const tool = distance(h, false);
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(10, 0));
    // A chain under way is not turned into rays.
    expect(tool.input('S')).toBe(false);
    expect(keys(tool)).toEqual(['Y', 'U', 'İ', 'A', 'G', 'Enter']);
    tool.confirm();
    tool.input('S');
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(10, 0));
    expect(tool.pointCount).toBe(2);
    // Off in the middle of rays: a run of one kind is not carried over into the other, it starts again.
    expect(tool.input('S')).toBe(true);
    expect(tool.pointCount).toBe(0);
    expect(tool.prompt.value).toBe('Mesafe ölç: ilk noktayı belirtin [Sabit ilk nokta (S)]');
  });

  it('Geri takes the last ray back', () => {
    const h = toolHarness();
    const tool = distance(h, true);
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(10, 0));
    tool.pointerDown(at(0, 10));
    expect(tool.pointCount).toBe(3);
    expect(tool.input('G')).toBe(true);
    expect(tool.pointCount).toBe(2);
  });

  it('takes typed relative points from the first point, not the last', () => {
    const h = toolHarness();
    const tool = distance(h, true);
    tool.pointerDown(at(100, 200));
    expect(tool.input('@10,0')).toBe(true);
    expect(tool.input('@0,10')).toBe(true);
    // Both from (100, 200): the second ray is due north, 10 m, not the chain's 14.142 m.
    expect(h.said().filter((t) => /^\d+: /.test(t))).toEqual(['1: 10.000 m, semt 100.0000 g', '2: 10.000 m, semt 0.0000 g']);
  });

  it('refers perpendicular snaps to the first point, as a chain refers them to the last', () => {
    const h = toolHarness();
    const rays = distance(h, true);
    expect(rays.snapFrom()).toBeNull();
    rays.pointerDown(at(5, 6));
    rays.pointerDown(at(9, 9));
    expect(rays.snapFrom()).toEqual(pt(5, 6));
    const chain = distance(h, false);
    chain.pointerDown(at(5, 6));
    chain.pointerDown(at(9, 9));
    expect(chain.snapFrom()).toEqual(pt(9, 9));
  });

  it('is kept for the session: the next Mesafe ölç starts with it on', () => {
    const h = toolHarness();
    distance(h, true);
    const again = h.use(new DistanceTool(h.ctx));
    again.activate();
    expect(isOn(again, 'Sabit ilk nokta (S)')).toBe(true);
    again.input('S');
    const third = new DistanceTool(h.ctx);
    third.activate();
    expect(isOn(third, 'Sabit ilk nokta (S)')).toBe(false);
  });

  it('previews rays from the first point, not the chain, with the distance and bearing by the cursor', () => {
    const h = toolHarness();
    const tool = distance(h, true);
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(10, 0));
    tool.pointerDown(at(10, 10));
    tool.pointerMove(at(-20, 0));
    const { g, view, paths, texts } = canvasLog();
    tool.draw(g, view);
    const solid = paths.filter((p) => !p.dashed).map((p) => p.points);
    // Each measured point is joined to the first; the second and the third are not joined to each other.
    expect(solid).toEqual([
      [[0, 0], [10, 0]],
      [[0, 0], [10, 10]],
    ]);
    expect(paths.filter((p) => p.dashed).map((p) => p.points)).toEqual([[[0, 0], [-20, 0]]]);
    expect(texts).toContain('1');
    expect(texts).toContain('2');
    expect(texts).toContain('20.000 m');
    expect(texts).toContain('Semt 300.0000 g');
  });

  it('draws the chain when it is off', () => {
    const h = toolHarness();
    const tool = distance(h, false);
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(10, 0));
    tool.pointerMove(at(10, 10));
    const { g, view, paths } = canvasLog();
    tool.draw(g, view);
    expect(paths.some((p) => p.points.length === 3)).toBe(true);
  });
});

/** The square of 10 m as four lines, with a 2 m island polygon inside it. */
function square(h: ReturnType<typeof toolHarness>) {
  const line = (a: [number, number], b: [number, number]) => h.add({ kind: 'line', a: pt(...a), b: pt(...b) });
  line([0, 0], [10, 0]);
  line([10, 0], [10, 10]);
  line([10, 10], [0, 10]);
  line([0, 10], [0, 0]);
  h.add({ kind: 'polygon', pts: [pt(4, 4), pt(6, 4), pt(6, 6), pt(4, 6)] });
}

describe('Alan hesapla: İçine tıkla', () => {
  it('offers its chip before the first point and takes clicks as regions', () => {
    const h = toolHarness();
    const tool = area(h, true);
    expect(tool.prompt.value).toBe('Alan hesapla: alanı ölçülecek bölgenin içine tıklayın [İçine tıkla (I): açık]');
    expect(keys(tool)).toEqual(['I']);
    tool.input('I');
    expect(tool.prompt.value).toBe('Alan hesapla: ilk noktayı belirtin [İçine tıkla (I)]');
  });

  it('reports the region around a click like any area, the islands taken off', () => {
    const h = toolHarness();
    square(h);
    const tool = area(h, true);
    tool.pointerDown(at(1, 1));
    // 100 m² less the 2 m island; the perimeter is the outline's 40 m and the island's 8 m, as a polygon's.
    expect(h.said().at(-1)).toBe('Alan 96.00 m²   Çevre 48.000 m');
  });

  it('writes nothing to the drawing', () => {
    const h = toolHarness();
    square(h);
    const tool = area(h, true);
    const before = h.doc.size;
    const revision = h.doc.revision;
    tool.pointerDown(at(1, 1));
    expect(h.doc.size).toBe(before);
    expect(h.doc.revision).toBe(revision);
  });

  it('says so when the click is not inside a closed region', () => {
    const h = toolHarness();
    square(h);
    const tool = area(h, true);
    tool.pointerDown(at(30, 30));
    expect(h.log.entries.value.at(-1)).toMatchObject({ level: 'warn', text: 'Tıklanan noktayı çevreleyen kapalı bölge yok.' });
    expect(keys(tool)).toEqual(['I']);
  });

  it('takes a typed point as a click', () => {
    const h = toolHarness();
    square(h);
    const tool = area(h, true);
    expect(tool.input('1,1')).toBe(true);
    expect(h.said().at(-1)).toBe('Alan 96.00 m²   Çevre 48.000 m');
  });

  it('takes no object snap: the click is where the cursor is', () => {
    const h = toolHarness();
    const tool = area(h, true);
    expect(tool.snaps).toBe(false);
    tool.input('I');
    expect(tool.snaps).toBe(true);
  });

  it('shows the region under the cursor filled, with its area, perimeter and islands', () => {
    const h = toolHarness();
    square(h);
    const tool = area(h, true);
    tool.pointerMove(at(1, 1));
    const { g, view, texts } = canvasLog();
    tool.draw(g, view);
    expect(texts).toEqual(['Alan 96.00 m²', 'Çevre 48.000 m', '1 ada']);
  });

  it('is kept for the session', () => {
    const h = toolHarness();
    area(h, true);
    const again = h.use(new AreaMeasureTool(h.ctx));
    again.activate();
    active.push(again);
    expect(isOn(again, 'İçine tıkla (I)')).toBe(true);
    again.input('I');
  });
});

describe('Alan hesapla: Alan olarak çiz', () => {
  it('is offered only after a measurement', () => {
    const h = toolHarness();
    square(h);
    const tool = area(h, true);
    expect(keys(tool)).toEqual(['I']);
    expect(tool.input('A')).toBe(false);
    tool.pointerDown(at(1, 1));
    expect(keys(tool)).toEqual(['I', 'A']);
    expect(tool.prompt.value).toBe('Alan hesapla: alanı ölçülecek bölgenin içine tıklayın [İçine tıkla (I): açık / Alan olarak çiz (A)]');
  });

  it('draws the area with its holes on the active layer, in one undo step of that name', () => {
    const h = toolHarness();
    square(h);
    const tool = area(h, true);
    tool.pointerDown(at(1, 1));
    const before = h.doc.size;
    expect(tool.input('A')).toBe(true);
    expect(h.doc.size).toBe(before + 1);
    const made = [...h.doc.all()].at(-1)!;
    expect(made).toMatchObject({ kind: 'polygon', layerId: 'cizim' });
    expect(made.kind === 'polygon' && made.holes?.length).toBe(1);
    expect(made.kind === 'polygon' && made.pts.length).toBe(4);
    expect(h.said().at(-1)).toBe('Alan olarak çizildi: 96.00 m².');
    // One step, and it is named for the tool.
    expect(h.doc.undo()).toBe('Alan olarak çiz');
    expect(h.doc.size).toBe(before);
    expect(h.doc.undo()).not.toBe('Alan olarak çiz');
  });

  it('draws a measured ring as clicked', () => {
    const h = toolHarness();
    const tool = area(h, false);
    tool.pointerDown(at(0, 0));
    tool.pointerDown(at(10, 0));
    tool.pointerDown(at(10, 10));
    tool.confirm();
    expect(h.said().at(-1)).toBe('Alan 50.00 m²   Çevre 34.142 m');
    expect(keys(tool)).toEqual(['I', 'A']);
    tool.input('A');
    const made = [...h.doc.all()].at(-1)!;
    expect(made.kind === 'polygon' && made.pts).toEqual([pt(0, 0), pt(10, 0), pt(10, 10)]);
    expect(made.kind === 'polygon' && made.holes).toBeUndefined();
    expect(h.said().at(-1)).toBe('Alan olarak çizildi: 50.00 m².');
  });

  it('draws it in the current colour and line weight, explicit as the drawing tools give them', () => {
    const h = toolHarness();
    h.ctx.settings.color.set('#E5484D');
    h.ctx.settings.lineWeight.set(0.5);
    const tool = area(h, false);
    for (const p of [pt(0, 0), pt(4, 0), pt(4, 3)]) tool.pointerDown(at(p.x, p.y));
    tool.confirm();
    tool.input('A');
    expect([...h.doc.all()].at(-1)).toMatchObject({ color: '#E5484D', lineWeight: 0.5 });
  });

  it('stays until the next measurement starts', () => {
    const h = toolHarness();
    const tool = area(h, false);
    for (const p of [pt(0, 0), pt(4, 0), pt(4, 3)]) tool.pointerDown(at(p.x, p.y));
    tool.confirm();
    expect(keys(tool)).toEqual(['I', 'A']);
    // Drawn, it is still offered (the area is still the last measured).
    tool.input('A');
    expect(keys(tool)).toEqual(['I', 'A']);
    // The first corner of the next one takes it away.
    tool.pointerDown(at(20, 20));
    expect(keys(tool)).toEqual(['Y', 'U', 'İ', 'A', 'G']);
    tool.confirm();
    tool.pointerDown(at(20, 20));
    // Line mode's A is Akış (docs/adr/0161 §3): Alan olarak çiz is offered no more.
    expect(tool.prompt.value).not.toContain('Alan olarak çiz');
  });

  it('is refused as the polygon command refuses: a locked layer, in the command’s words, nothing written', () => {
    const h = toolHarness();
    square(h);
    const tool = area(h, true);
    tool.pointerDown(at(1, 1));
    h.doc.layers.setActive('kilitli');
    const before = h.doc.size;
    const revision = h.doc.revision;
    expect(tool.input('A')).toBe(true);
    expect(h.doc.size).toBe(before);
    expect(h.doc.revision).toBe(revision);
    expect(h.log.entries.value.at(-1)).toMatchObject({ level: 'warn', text: expect.stringContaining('kilitli') });
    h.doc.layers.setActive('cizim');
  });

  it('Ctrl+Z inside the tool takes the drawn area back', () => {
    const h = toolHarness();
    square(h);
    const tool = area(h, true);
    tool.pointerDown(at(1, 1));
    const before = h.doc.size;
    tool.input('A');
    expect(h.doc.size).toBe(before + 1);
    expect(tool.undoStep()).toBe(true);
    expect(h.doc.size).toBe(before);
  });

  it('shows what it would draw, dashed, until the next measurement', () => {
    const h = toolHarness();
    const tool = area(h, false);
    for (const p of [pt(0, 0), pt(4, 0), pt(4, 3)]) tool.pointerDown(at(p.x, p.y));
    tool.confirm();
    const { g, view, paths } = canvasLog();
    tool.draw(g, view);
    expect(paths.some((p) => p.dashed)).toBe(true);
  });

  it('leaves at once on Esc, chip or no chip', () => {
    const h = toolHarness();
    const tool = area(h, false);
    for (const p of [pt(0, 0), pt(4, 0), pt(4, 3)]) tool.pointerDown(at(p.x, p.y));
    tool.confirm();
    expect(h.state.exited).toBe(0);
    h.ctx.tools.exit();
    expect(h.state.exited).toBe(1);
  });

  it('keeps the arc options where they were: A and I in the middle of a path are the arc’s', () => {
    const h = toolHarness();
    const tool = area(h, false);
    tool.pointerDown(at(0, 0));
    tool.input('Y');
    // With a point taken and the arc mode on, A is Açı and I is İkinci nokta, not the chips.
    expect(tool.input('A')).toBe(true);
    expect(tool.prompt.value).toContain('iç açısını');
    tool.input('D');
  });
});

describe('The second system’s plane (docs/adr/0167 §2)', () => {
  // fixtures/geodesy/v1/measure.json (PROJ): TUREF TM30 at İstanbul, measured in ED50 TM30.
  const ist = (x: number, y: number) => at(414000 + x, 4540000 + y);

  it('Mesafe ölç says the total in the second plane after its own', () => {
    const h = toolHarness();
    h.doc.settings.assign({ srid: 5254, secondSrid: 2320 });
    const tool = distance(h, false);
    tool.pointerDown(ist(0, 0));
    tool.pointerDown(ist(60.25, 12.5));
    tool.pointerDown(ist(84.75, -30.125));
    tool.confirm();
    expect(h.said().slice(-2)).toEqual(['Toplam uzunluk 110.697 m (2 kenar)', 'ED50 TM30 düzleminde: Toplam uzunluk 110.698 m']);
  });

  it('Alan hesapla says the area and the perimeter there; a ray gives its length there too', () => {
    const h = toolHarness();
    h.doc.settings.assign({ srid: 5254, secondSrid: 2320 });
    const tool = area(h, false);
    for (const [x, y] of [
      [0, 0],
      [40, 0],
      [40, 25],
      [0, 25],
    ])
      tool.pointerDown(ist(x, y));
    tool.confirm();
    expect(h.said().slice(-2)).toEqual(['Alan 1000.00 m²   Çevre 130.000 m', 'ED50 TM30 düzleminde: Alan 1000.01 m²   Çevre 130.001 m']);
    const rays = distance(h, true);
    rays.pointerDown(ist(0, 0));
    rays.pointerDown(ist(40, 0));
    expect(h.said().at(-1)).toMatch(/^1: 40\.000 m, semt 100\.0000 g \(ED50 TM30 düzleminde 40\.\d{3} m\)$/);
    rays.input('S');
  });

  it('says why there is none in a geographic system or the Pseudo-Mercator', () => {
    const h = toolHarness();
    h.doc.settings.assign({ srid: 5254, secondSrid: 4326 });
    const tool = distance(h, false);
    tool.pointerDown(ist(0, 0));
    tool.pointerDown(ist(10, 0));
    tool.confirm();
    expect(h.said().at(-1)).toBe('WGS 84 coğrafi bir sistem: uzunluk ve alan onun düzleminde verilmez.');
    h.doc.settings.assign({ secondSrid: 3857 });
    tool.pointerDown(ist(0, 0));
    tool.pointerDown(ist(10, 0));
    tool.confirm();
    expect(h.said().at(-1)).toBe("WGS 84 Pseudo-Mercator: uzunluk ve alan verilmez, Pseudo-Mercator'un ölçeği her enlemde başkadır.");
  });
});
