import { describe, expect, it } from 'vitest';
import { CadDocument, Refusal } from './document';
import type { Entity } from './entities';
import { LayerStore } from './layers';

/**
 * What the shared cases (fixtures/document-ops/v1/layer-add.json) leave
 * out: another editor's objects on a layer this drawing added, and the
 * refusal's type.
 */
function drawing() {
  return new CadDocument({ name: 'Katmanlar', layers: new LayerStore([{ id: 'cizim', name: 'Çizim' }], 'cizim'), origin: { x: 0, y: 0 } });
}

/** An object as another editor's change brings it, on `layerId`. */
const theirs = (doc: CadDocument, layerId: string): Entity => ({ id: doc.allocateId(), uid: crypto.randomUUID(), kind: 'point', layerId, p: { x: 1, y: 1 }, attrs: {} });

describe('adding a layer', () => {
  it('another editor’s object on a layer added here drops the step: undo cannot take the layer from under it', () => {
    const doc = drawing();
    doc.addLayer({ id: 'yeni', name: 'Yeni' }, null);
    expect(doc.canUndo.value).toBe(true);
    doc.applyExternal({ put: [theirs(doc, 'yeni')] });
    expect(doc.canUndo.value).toBe(false);
    expect(doc.layers.get('yeni')).toBeDefined();
  });

  it('a layer put since into a group added here: the group’s step goes too, as the step that added the layer', () => {
    const doc = drawing();
    doc.addLayer({ id: 'grup', name: 'Grup', type: 'group', children: [] }, null);
    doc.addLayer({ id: 'ic', name: 'İç' }, 'grup');
    doc.applyExternal({ put: [theirs(doc, 'ic')] });
    expect(doc.canUndo.value).toBe(false);
    expect(doc.layers.tree.map((n) => n.id)).toEqual(['cizim', 'grup']);
  });

  it('a step the other editor’s object does not concern stays', () => {
    const doc = drawing();
    doc.addLayer({ id: 'bir', name: 'Bir' }, null);
    doc.addLayer({ id: 'iki', name: 'İki' }, null);
    doc.applyExternal({ put: [theirs(doc, 'iki')] });
    expect(doc.undo()).toBe('Katman ekle');
    expect(doc.layers.tree.map((n) => n.id)).toEqual(['cizim', 'iki']);
    expect(doc.canUndo.value).toBe(false);
  });

  it('refuses an id the tree has with a Refusal that names it, and changes nothing', () => {
    const doc = drawing();
    expect(() => doc.addLayer({ id: 'cizim', name: 'Başka' }, null)).toThrow(Refusal);
    expect(() => doc.addLayer({ id: 'cizim', name: 'Başka' }, null)).toThrow('“cizim” kimlikli katman zaten var; katman eklenmedi.');
    expect([doc.layers.tree.length, doc.canUndo.value, doc.dirty.value]).toEqual([1, false, false]);
  });
});
