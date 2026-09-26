import { describe, expect, it } from 'vitest';
import type { AppContext } from '../app/context';
import { Formatter } from '../app/format';
import { MessageLog } from '../app/state';
import { Signal } from '../core/signal';
import { CadDocument } from '../model/document';
import { LayerStore } from '../model/layers';
import { Selection } from '../model/selection';
import { PointTool } from './drawTools';
import { PathTool } from './pathTool';

/**
 * The path tool's G while U waits for a length, and the tools that write to
 * their own layer (Parsel, Kot noktası) on a locked one: typed input only,
 * over a document and a log, without a view.
 */
function harness() {
  const doc = new CadDocument({
    name: 'Deneme',
    layers: new LayerStore(
      [
        { id: 'cizim', name: 'Çizim' },
        { id: 'parsel', name: 'Parsel sınırı' },
        { id: 'kot', name: 'Kot noktaları' },
      ],
      'cizim',
    ),
    origin: { x: 0, y: 0 },
  });
  const log = new MessageLog();
  const ctx = {
    doc,
    log,
    selection: new Selection(),
    format: new Formatter({ lengthDecimals: new Signal(3), areaDecimals: new Signal(2), areaUnit: new Signal('m2' as const), angleUnit: new Signal('grad' as const) }),
    settings: { color: new Signal<string | null>(null) },
    prefs: { snapAperture: new Signal(8) },
    view: { requestOverlay: () => {}, trackAlong: () => null, camera: { worldToScreen: (p: unknown) => p } },
    tools: { exit: () => {} },
  } as unknown as AppContext;
  const said = () => log.entries.value.map((e) => e.text);
  return { ctx, doc, said };
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
