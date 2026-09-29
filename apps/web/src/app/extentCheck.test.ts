import { afterEach, describe, expect, it } from 'vitest';
import { CadDocument } from '../model/document';
import { LayerStore } from '../model/layers';
import { Selection } from '../model/selection';
import { PickIndex } from '../viewport/picking';
import type { AppContext } from './context';
import { checkExtent } from './extentCheck';
import { MessageLog } from './state';

/** Kapsam denetimi on drawings the way the ADR's scenes have them (docs/adr/0141): TM-coordinate parcels, and strays. */
const disposables: PickIndex[] = [];
afterEach(() => disposables.splice(0).forEach((p) => p.dispose()));

function drawing() {
  const doc = new CadDocument({ name: 'Deneme', layers: new LayerStore([{ id: 'a', name: 'A' }], 'a'), origin: { x: 0, y: 0 } });
  const square = (x: number, y: number, s = 15) => doc.add({ kind: 'polygon', layerId: 'a', attrs: {}, pts: [{ x, y }, { x: x + s, y }, { x: x + s, y: y + s }, { x, y: y + s }] });
  const picker = new PickIndex(doc);
  disposables.push(picker);
  const log = new MessageLog();
  const selection = new Selection();
  const ctx = { view: { extentOutliers: () => picker.extentOutliers() }, selection, log } as unknown as AppContext;
  return { doc, square, ctx, log, selection, said: () => log.entries.value.map((e) => ({ level: e.level, text: e.text })) };
}

describe('Kapsam denetimi', () => {
  it('selects the objects far from a hundred parcels and says how many', () => {
    const { square, ctx, selection, said } = drawing();
    for (let i = 0; i < 100; i++) square(500_000 + (i % 10) * 20, 4_400_000 + Math.floor(i / 10) * 20);
    const zero = square(0, 0);
    const wrong = square(3_000_000, 1_000_000);
    checkExtent(ctx);
    expect([...selection.ids.value].sort((a, b) => a - b)).toEqual([zero.id, wrong.id]);
    expect(said()).toEqual([{ level: 'warn', text: '2 nesne çizimin geri kalanından çok uzakta; seçildi.' }]);
  });

  it('says so, and leaves the selection as it was, when nothing is far', () => {
    const { square, ctx, selection, said } = drawing();
    const parcels = Array.from({ length: 40 }, (_, i) => square(500_000 + (i % 8) * 20, 4_400_000 + Math.floor(i / 8) * 20));
    selection.set([parcels[3].id, parcels[4].id]);
    checkExtent(ctx);
    expect([...selection.ids.value]).toEqual([parcels[3].id, parcels[4].id]);
    expect(said()).toEqual([{ level: 'info', text: 'Çizimin kapsamını bozan nesne yok.' }]);
  });

  it('finds nothing in a drawing of two towns fifty kilometres apart', () => {
    const { square, ctx, selection, said } = drawing();
    for (let i = 0; i < 50; i++) square(500_000 + (i % 10) * 20, 4_400_000 + Math.floor(i / 10) * 20);
    for (let i = 0; i < 50; i++) square(550_000 + (i % 10) * 20, 4_400_000 + Math.floor(i / 10) * 20);
    checkExtent(ctx);
    expect(selection.size).toBe(0);
    expect(said().map((m) => m.level)).toEqual(['info']);
  });

  it('has no core in a drawing of fewer than four objects', () => {
    const { square, ctx, said } = drawing();
    square(0, 0);
    square(900_000, 900_000);
    checkExtent(ctx);
    expect(said()[0].text).toBe('Çizimin kapsamını bozan nesne yok.');
  });

  it('moves and deletes nothing', () => {
    const { doc, square, ctx } = drawing();
    for (let i = 0; i < 20; i++) square(500_000 + (i % 5) * 20, 4_400_000 + Math.floor(i / 5) * 20);
    square(0, 0);
    const before = doc.size;
    const revision = doc.revision;
    checkExtent(ctx);
    expect(doc.size).toBe(before);
    expect(doc.revision).toBe(revision);
  });
});
