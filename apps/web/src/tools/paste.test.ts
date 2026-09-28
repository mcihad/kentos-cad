import { describe, expect, it } from 'vitest';
import type { AppContext } from '../app/context';
import { MessageLog } from '../app/state';
import { CadDocument } from '../model/document';
import type { NewEntity } from '../model/entities';
import { LayerStore } from '../model/layers';
import { Selection } from '../model/selection';
import { pasteEntities } from './editTools';

/**
 * Yapıştır writes through the product commands: `cad.entities.create` for
 * each run of objects going to one layer, `cad.entities.set` for their
 * symbols, all in one step “Yapıştır”. Over a document and a log, without a
 * view.
 */
function harness() {
  const doc = new CadDocument({
    name: 'Pano',
    layers: new LayerStore(
      [
        { id: 'cizim', name: 'Çizim' },
        { id: 'yol', name: 'Yol' },
        { id: 'kilitli', name: 'Kilitli', locked: true },
        { id: 'gizli', name: 'Gizli', visible: false },
        { id: 'grup', name: 'Grup', type: 'group', children: [{ id: 'ic', name: 'İç' }] },
      ],
      'cizim',
    ),
    origin: { x: 0, y: 0 },
  });
  doc.add({ kind: 'point', layerId: 'cizim', p: { x: -5, y: -5 }, attrs: {} });
  const log = new MessageLog();
  const ctx = { doc, log, selection: new Selection() } as unknown as AppContext;
  const said = () => log.entries.value.map((e) => e.text);
  return { ctx, doc, said };
}

const at = (x: number, y: number) => ({ x, y });

describe('Yapıştır', () => {
  it('pastes onto each object’s own layer, else the active one; keeps colour, line weight, attributes, label and symbol; one step, slots in the clipboard’s order', () => {
    const { ctx, doc, said } = harness();
    const items: NewEntity[] = [
      { kind: 'point', layerId: 'yol', p: at(0, 0), attrs: { Ad: 'P1' }, label: 'P1', symbol: 'nirengi' },
      { kind: 'line', layerId: 'kilitli', a: at(0, 0), b: at(1, 0), attrs: {} },
      { kind: 'circle', layerId: 'baska-cizimin', c: at(0, 0), r: 2, lineWeight: 0, attrs: {} },
      { kind: 'text', layerId: 'grup', p: at(0, 0), text: 'Ada 101', height: 2, rotation: 0, attrs: {} },
      { kind: 'line', layerId: 'yol', a: at(0, 1), b: at(1, 1), color: '#E5484D', lineWeight: 0.7, attrs: {} },
    ];
    const ids = pasteEntities(ctx, items, 10, 0);
    // Slot 1 is the drawing's own point: the pasted ones follow in the clipboard's order.
    expect(ids).toEqual([2, 3, 4, 5, 6]);
    // A locked layer, one this drawing lacks and a group send their objects to the active layer.
    expect(ids.map((id) => doc.get(id)?.layerId)).toEqual(['yol', 'cizim', 'cizim', 'cizim', 'yol']);
    expect(doc.get(2)).toMatchObject({ kind: 'point', p: at(10, 0), attrs: { Ad: 'P1' }, label: 'P1', symbol: 'nirengi' });
    // Its own line weight too (docs/adr/0139), 0 the thinnest line; none stays none.
    expect(doc.get(6)).toMatchObject({ kind: 'line', a: at(10, 1), b: at(11, 1), color: '#E5484D', lineWeight: 0.7 });
    expect(doc.get(4)).toMatchObject({ kind: 'circle', lineWeight: 0 });
    expect('lineWeight' in doc.get(3)!).toBe(false);
    expect(said().at(-1)).toBe('5 nesne yapıştırıldı.');
    expect(doc.undo()).toBe('Yapıştır');
    expect(doc.size).toBe(1);
    // The step before it is the drawing's own point: the paste was one step.
    expect(doc.undo()).toBe('Ekle');
  });

  it('says a hidden layer once', () => {
    const { ctx, doc, said } = harness();
    const items: NewEntity[] = [
      { kind: 'point', layerId: 'gizli', p: at(0, 0), attrs: {} },
      { kind: 'point', layerId: 'gizli', p: at(1, 0), attrs: {} },
    ];
    expect(pasteEntities(ctx, items, 0, 0)).toHaveLength(2);
    expect(doc.size).toBe(3);
    expect(said().slice(-2)).toEqual(['“Gizli” katmanı gizli; yapıştırılan nesneler görünmeyecek.', '2 nesne yapıştırıldı.']);
  });

  it('a refused object takes the whole paste back and is said in the command’s words', () => {
    const { ctx, doc, said } = harness();
    const items: NewEntity[] = [
      { kind: 'line', layerId: 'yol', a: at(0, 0), b: at(1, 0), attrs: {} },
      { kind: 'text', layerId: 'cizim', p: at(0, 0), text: '  ', height: 2, rotation: 0, attrs: {} },
    ];
    expect(pasteEntities(ctx, items, 0, 0)).toEqual([]);
    expect(doc.size).toBe(1);
    // No step was recorded: the newest is still the drawing's own point.
    expect(doc.undo()).toBe('Ekle');
    expect(said().at(-1)).toMatch(/metni boş olamaz/);
  });
});
