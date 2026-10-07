import { describe, expect, it } from 'vitest';
import type { AppContext } from './context';
import { CadDocument } from '../model/document';
import { LayerStore } from '../model/layers';
import { duplicateLayer, mergeLayers } from './layerActions';

/**
 * Katmanları birleştir and Kopyasını oluştur (docs/adr/0177 §3) on the desktop's sample drawing's tree: Kadastro /
 * Parsel (active, one object), Kadastro / Bina (locked, three), Çizim (hidden, two). The desktop's `layer_merge::tests`
 * are the same.
 */
function setup() {
  const doc = new CadDocument({
    name: 'Deneme',
    layers: new LayerStore(
      [
        {
          id: 'layer-g',
          name: 'Kadastro',
          children: [
            { id: 'parsel', name: 'Parsel' },
            { id: 'bina', name: 'Bina', locked: true },
          ],
        },
        { id: 'cizim', name: 'Çizim', visible: false },
      ],
      'parsel',
    ),
    origin: { x: 0, y: 0 },
  });
  const point = (layerId: string, x: number) => doc.add({ kind: 'point', layerId, attrs: {}, p: { x, y: 0 } });
  const parcel = point('parsel', 1);
  for (const x of [2, 3, 4]) point('bina', x);
  for (const x of [5, 6]) point('cizim', x);
  const said: string[] = [];
  const say = (text: string) => void said.push(text);
  const ctx = { doc, log: { info: say, warn: say, success: say }, cloud: { project: { value: null } } } as unknown as AppContext;
  return { ctx, doc, said, parcel: parcel.id };
}

describe('Katmanları birleştir', () => {
  it('makes the target active when the active layer merges, and undoes in one step', () => {
    const { ctx, doc, said, parcel } = setup();
    expect(mergeLayers(ctx, ['parsel'], 'cizim')).toBe(true);
    expect(doc.layers.active.value).toBe('cizim');
    expect(doc.layers.get('parsel')).toBeUndefined();
    expect(doc.get(parcel)?.layerId).toBe('cizim');
    expect(said.at(-1)).toBe('1 katman “Çizim” katmanına birleştirildi: 1 nesne taşındı.');
    doc.undo();
    expect(doc.layers.get('parsel')).toBeDefined();
    expect(doc.get(parcel)?.layerId).toBe('parsel');
    // The active layer is the tree's own change: undo leaves it.
    expect(doc.layers.active.value).toBe('cizim');
  });

  it('neither merges nor copies a locked layer', () => {
    const { ctx, said } = setup();
    expect(mergeLayers(ctx, ['bina'], 'parsel')).toBe(false);
    expect(said.at(-1)).toBe('“Bina” katmanı kilitli; birleştirilemez. Kilidini Katmanlar panelinden açın.');
    duplicateLayer(ctx, 'bina');
    expect(said.at(-1)).toBe('“Bina” katmanı kilitli; kopyası oluşturulmaz. Kilidini Katmanlar panelinden açın.');
  });
});

describe('Kopyasını oluştur', () => {
  it('gives the copy the layer’s fields, the values going as they are (docs/adr/0199 §1)', () => {
    const { ctx, doc, said, parcel } = setup();
    doc.update(parcel, { attrs: { Kat: '3a' } });
    doc.setLayerFields('parsel', [{ name: 'Kat', kind: 'integer' }]);
    duplicateLayer(ctx, 'parsel');
    expect(said.at(-1)).toBe('“Parsel” katmanı “Parsel kopyası” olarak kopyalandı: 1 nesne.');
    const copy = doc.layers.leaves().find((l) => l.name === 'Parsel kopyası')!;
    expect(copy.fields).toEqual([{ name: 'Kat', kind: 'integer' }]);
    expect(doc.byLayer(copy.id).map((e) => e.attrs.Kat)).toEqual(['3a']);
  });


  it('gives every copy a name of its own, last in the layer’s group', () => {
    const { ctx, doc, said } = setup();
    duplicateLayer(ctx, 'parsel');
    duplicateLayer(ctx, 'parsel');
    expect(said.at(-1)).toBe('“Parsel” katmanı “Parsel kopyası 2” olarak kopyalandı: 1 nesne.');
    expect(doc.layers.leaves().map((l) => l.name)).toEqual(['Parsel', 'Bina', 'Parsel kopyası', 'Parsel kopyası 2', 'Çizim']);
  });
});
