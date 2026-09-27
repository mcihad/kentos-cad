import { describe, expect, it } from 'vitest';
import { CadDocument, Refusal } from './document';
import { LayerStore } from './layers';

/**
 * What the shared cases (fixtures/document-ops/v1/layer-remove.json) leave
 * out: another editor's layer tree and the refusal's type.
 */
function drawing() {
  const doc = new CadDocument({
    name: 'Katmanlar',
    layers: new LayerStore(
      [
        { id: 'cizim', name: 'Çizim' },
        { id: 'parsel', name: 'Parsel' },
      ],
      'cizim',
    ),
    origin: { x: 0, y: 0 },
  });
  doc.add({ kind: 'point', layerId: 'parsel', p: { x: 1, y: 2 }, attrs: {} });
  return doc;
}

describe('removing a layer', () => {
  it('another editor’s tree drops the steps that removed or brought back a layer: their places may not fit it', () => {
    const doc = drawing();
    doc.removeLayer('parsel');
    expect(doc.canUndo.value).toBe(true);
    doc.applyExternal({ meta: { layers: [{ id: 'cizim', name: 'Çizim' }, { id: 'yol', name: 'Yol' }] } });
    // The step that added the object stays; the one that removed the layer with it went.
    expect(doc.undo()).toBe('Ekle');
    expect(doc.layers.get('parsel')).toBeUndefined();
  });

  it('refuses with a Refusal that names the layer, and changes nothing', () => {
    const doc = drawing();
    expect(() => doc.removeLayer('cizim')).toThrow(Refusal);
    expect(() => doc.removeLayer('cizim')).toThrow('“Çizim” etkin katman; silinemez. Önce başka bir katmanı etkinleştirin.');
    expect(doc.layerRemovalRefused('cizim')).toBe('“Çizim” etkin katman; silinemez. Önce başka bir katmanı etkinleştirin.');
    expect(doc.layerRemovalRefused('parsel')).toBeNull();
    expect(doc.layers.tree.map((n) => n.id)).toEqual(['cizim', 'parsel']);
  });
});
