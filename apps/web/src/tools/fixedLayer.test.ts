import { afterEach, describe, expect, it, vi } from 'vitest';
import type { AppContext } from '../app/context';
import { Formatter } from '../app/format';
import { MessageLog } from '../app/state';
import { Signal } from '../core/signal';
import { CadDocument } from '../model/document';
import { LayerStore, type LayerInit } from '../model/layers';
import { standardLayers } from '../model/standardLayers';
import { pointCreate } from '../product/pointCreate';
import { Selection } from '../model/selection';
import { PointTool } from './drawTools';
import { PathTool } from './pathTool';

/**
 * The path tool's G while U waits for a length, and the tools that write to
 * their own layer (Parsel, Kot noktası) on a locked one: typed input only,
 * over a document and a log, without a view.
 */
function harness(layers: LayerInit[] = [{ id: 'cizim', name: 'Çizim' }, { id: 'parsel', name: 'Parsel sınırı' }, { id: 'kot', name: 'Kot noktaları' }]) {
  const doc = new CadDocument({
    name: 'Deneme',
    layers: new LayerStore(layers, 'cizim'),
    origin: { x: 0, y: 0 },
  });
  const log = new MessageLog();
  const ctx = {
    doc,
    log,
    selection: new Selection(),
    format: new Formatter({ lengthDecimals: new Signal(3), areaDecimals: new Signal(2), areaUnit: new Signal('m2' as const), angleUnit: new Signal('grad' as const) }),
    settings: { color: new Signal<string | null>(null), lineWeight: new Signal<number | null>(null) },
    prefs: { snapAperture: new Signal(8) },
    view: { requestOverlay: () => {}, trackAlong: () => null, camera: { worldToScreen: (p: unknown) => p } },
    tools: { exit: () => {} },
  } as unknown as AppContext;
  const said = () => log.entries.value.map((e) => e.text);
  return { ctx, doc, said, settings: ctx.settings };
}

const measure = (ctx: AppContext) => new PathTool(ctx, { id: 'measure', label: 'Mesafe ölç', closed: false, measureOnly: true });

describe('path tool: G while U waits for a length', () => {
  it('G ends the length prompt: the line-mode prompt comes back and a number is no longer a length', () => {
    const { ctx, said } = harness();
    const tool = measure(ctx);
    tool.activate();
    for (const p of ['0,0', '10,0', '20,0']) expect(tool.input(p)).toBe(true);
    tool.input('U');
    expect(tool.prompt.value).toBe('Mesafe ölç: son doğrultuda devam edilecek uzunluğu yazın [Geri (G)]');
    tool.input('G');
    expect(tool.prompt.value).toBe('Mesafe ölç: sonraki noktayı belirtin [Yay (Y) / Uzunluk (U) / Geri (G) / Bitir (Enter)]');
    tool.input('G');
    expect(tool.prompt.value).toBe('Mesafe ölç: sonraki noktayı belirtin [Yay (Y) / Uzunluk (U) / Geri (G)]');
    // With one point left there is no direction to continue: before, this said “Uzunluk sıfırdan büyük olmalı.”
    expect(tool.input('5')).toBe(false);
    expect(said()).not.toContain('Uzunluk sıfırdan büyük olmalı.');
  });

  it('Ctrl+Z is G: it takes back the point and ends the length prompt', () => {
    const { ctx } = harness();
    const tool = measure(ctx);
    tool.activate();
    for (const p of ['0,0', '10,0', '20,0']) tool.input(p);
    tool.input('U');
    expect(tool.undoStep()).toBe(true);
    expect(tool.pointCount).toBe(2);
    expect(tool.prompt.value).toBe('Mesafe ölç: sonraki noktayı belirtin [Yay (Y) / Uzunluk (U) / Geri (G) / Bitir (Enter)]');
    // U again continues the direction of what is left.
    tool.input('U');
    expect(tool.input('5')).toBe(true);
    expect(tool.pointCount).toBe(3);
  });
});

describe('tools that write to their own layer, on a locked one', () => {
  const LOCKED = (name: string, tool: string) => `“${name}” katmanı kilitli; ${tool} bu katmana yazar. Kilidi Katmanlar panelinden açın.`;

  it('Parsel says the parcel layer is locked in its own words and writes nothing', () => {
    const { ctx, doc, said } = harness();
    doc.layers.toggleLocked('parsel');
    const tool = new PathTool(ctx, { id: 'parcel', label: 'Parsel', closed: true, parcelLayer: 'parsel' });
    tool.activate();
    for (const p of ['0,0', '10,0', '10,10']) tool.input(p);
    tool.confirm();
    expect(doc.size).toBe(0);
    expect(doc.canUndo.value).toBe(false);
    expect(said().at(-1)).toBe(LOCKED('Parsel sınırı', 'Parsel'));
  });

  it('Kot noktası says the elevation layer is locked in its own words and writes nothing', () => {
    const { ctx, doc, said } = harness();
    doc.layers.toggleLocked('kot');
    const tool = new PointTool(ctx, { id: 'spot', label: 'Kot noktası', askZ: true, layerId: 'kot' });
    tool.activate();
    tool.input('5,5');
    expect(tool.input('12.5')).toBe(true);
    expect(doc.size).toBe(0);
    expect(said().at(-1)).toBe(LOCKED('Kot noktaları', 'Kot noktası'));
  });

  it('an unlocked layer is written as before, the hidden-layer warning still the command’s', () => {
    const { ctx, doc, said } = harness();
    doc.layers.toggleVisible('kot');
    const tool = new PointTool(ctx, { id: 'spot', label: 'Kot noktası', askZ: true, layerId: 'kot' });
    tool.activate();
    tool.input('5,5');
    tool.input('12.5');
    expect(doc.size).toBe(1);
    expect(said()).toContain('“Kot noktaları” katmanı gizli; çizilen nesne görünmeyecek.');
  });
});

/**
 * Parsel oluştur and Kot noktası on a drawing without their standard layer
 * (an imported plan, an older file; docs/adr/0067): the layer is opened as a
 * new project has it, in the object's own undo step, and said.
 */
describe('tools that write to their own layer, on a drawing without it', () => {
  afterEach(() => vi.restoreAllMocks());

  const drawn = [{ id: 'cizim', name: 'Çizim' }];
  const standard = (id: string) => standardLayers(1000).flatMap((g) => g.children ?? [g]).find((l) => l.id === id)!;
  const parcel = (ctx: AppContext) => {
    const tool = new PathTool(ctx, { id: 'parcel', label: 'Parsel', closed: true, parcelLayer: 'parsel' });
    tool.activate();
    for (const p of ['0,0', '10,0', '10,10']) tool.input(p);
    tool.confirm();
  };
  const spot = (ctx: AppContext) => {
    const tool = new PointTool(ctx, { id: 'spot', label: 'Kot noktası', askZ: true, layerId: 'kot' });
    tool.activate();
    tool.input('5,5');
    tool.input('12.5');
  };
  const cases = [
    { tool: 'Parsel', id: 'parsel', name: 'Parsel sınırı', what: 'parsel', run: parcel, success: 'Parsel 1 oluşturuldu; geometrik alanı 50.00 m². Ada, mahalle ve tapu alanı bilgisini Öznitelikler panelinden girin.' },
    { tool: 'Kot noktası', id: 'kot', name: 'Kot noktaları', what: 'kot noktası', run: spot, success: null },
  ];

  for (const c of cases) {
    it(`${c.tool}: opens the layer as a new project has it, at the top and not active, and writes the object on it`, () => {
      const { ctx, doc, said } = harness(drawn);
      c.run(ctx);
      expect(doc.size).toBe(1);
      const layer = doc.layers.get(c.id)!;
      expect(layer.name).toBe(c.name);
      expect(layer.type).toBe('layer');
      // The template's style, the same fresh copy a new project has.
      expect(layer.style).toMatchObject(standard(c.id).style!);
      expect(doc.layers.tree.map((n) => n.id)).toEqual(['cizim', c.id]);
      expect(doc.layers.parentOf(c.id)).toBeNull();
      expect(doc.layers.active.value).toBe('cizim');
      expect([...doc.all()].map((e) => e.layerId)).toEqual([c.id]);
      const info = `“${c.name}” katmanı çizimde yoktu; ${c.what} için açıldı.`;
      expect(said()).toContain(info);
      // Said after the object is written, before the tool's own success message.
      if (c.success) expect(said().indexOf(info)).toBeLessThan(said().indexOf(c.success));
      expect(said().at(-1)).toBe(c.success ?? info);
    });

    it(`${c.tool}: one undo takes the object and the layer, one redo brings both`, () => {
      const { ctx, doc } = harness(drawn);
      c.run(ctx);
      expect(doc.canUndo.value).toBe(true);
      // The step is named as the object's own is; the layer went into it, not into a step of its own.
      const steps = (doc as unknown as { undoStack: { label: string }[] }).undoStack;
      expect(steps.map((t) => t.label)).toEqual(['Ekle']);
      doc.undo();
      expect(doc.size).toBe(0);
      expect(doc.layers.get(c.id)).toBeUndefined();
      expect(doc.layers.tree.map((n) => n.id)).toEqual(['cizim']);
      expect(doc.canUndo.value).toBe(false);
      doc.redo();
      expect(doc.size).toBe(1);
      expect(doc.layers.get(c.id)?.name).toBe(c.name);
      expect(doc.layers.active.value).toBe('cizim');
    });

    it(`${c.tool}: a refused object leaves no layer, no undo step and no message about one`, () => {
      const { ctx, doc, said, settings } = harness(drawn);
      // The command refuses the object: a line weight past 100 mm for the parcel, the spy for the point (nothing typed is refused).
      if (c.id === 'parsel') settings.lineWeight.set(500);
      else vi.spyOn(pointCreate, 'execute').mockReturnValue({ status: 'failed', error: { code: 'not_finite', message: 'Kot sonlu bir sayı değil.', path: 'z' } });
      c.run(ctx);
      expect(doc.size).toBe(0);
      expect(doc.layers.get(c.id)).toBeUndefined();
      expect(doc.layers.tree.map((n) => n.id)).toEqual(['cizim']);
      expect(doc.canUndo.value).toBe(false);
      expect(doc.dirty.value).toBe(false);
      expect(said().some((t) => t.includes('için açıldı'))).toBe(false);
      expect(said().at(-1)).toBe(c.id === 'parsel' ? 'Çizgi kalınlığı 0 ile 100 mm arasında bir sayı olmalı (0 en ince çizgidir). Bir kalınlık ya da “Katmana göre” seçin.' : 'Kot sonlu bir sayı değil.');
    });

    it(`${c.tool}: a locked standard layer keeps its refusal: nothing is opened or written`, () => {
      const { ctx, doc, said } = harness([...drawn, { id: c.id, name: c.name, locked: true }]);
      c.run(ctx);
      expect(doc.size).toBe(0);
      expect(doc.canUndo.value).toBe(false);
      expect(doc.layers.tree.map((n) => n.id)).toEqual(['cizim', c.id]);
      expect(said().at(-1)).toBe(`“${c.name}” katmanı kilitli; ${c.tool} bu katmana yazar. Kilidi Katmanlar panelinden açın.`);
      expect(said().some((t) => t.includes('için açıldı'))).toBe(false);
    });

    it(`${c.tool}: a layer the drawing has is not opened again, and says nothing of it`, () => {
      const { ctx, doc, said } = harness(drawn);
      c.run(ctx);
      c.run(ctx);
      expect(doc.size).toBe(2);
      expect(doc.layers.tree.map((n) => n.id)).toEqual(['cizim', c.id]);
      expect(said().filter((t) => t.includes('için açıldı'))).toHaveLength(1);
      // The second object is its own step; undoing it keeps the layer the first one opened.
      doc.undo();
      expect(doc.size).toBe(1);
      expect(doc.layers.get(c.id)).toBeDefined();
    });
  }
});
