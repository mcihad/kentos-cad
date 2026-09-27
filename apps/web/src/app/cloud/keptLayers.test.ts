import { describe, expect, it } from 'vitest';
import { CadDocument } from '../../model/document';
import { LayerStore, type LayerInit } from '../../model/layers';
import { keepUnsentLayers } from './keptLayers';

/** Where a layer another editor's tree drops, but that holds unsent objects, goes back. */
function drawing() {
  const doc = new CadDocument({
    name: 'Ada',
    layers: new LayerStore(
      [
        { id: 'g', name: 'Yapılar', type: 'group', children: [{ id: 'a', name: 'Bina' }, { id: 'b', name: 'Yol' }] },
        { id: 'c', name: 'Parsel' },
      ],
      'a',
    ),
    origin: { x: 0, y: 0 },
  });
  const onB = doc.add({ kind: 'point', layerId: 'b', p: { x: 1, y: 1 }, attrs: {} });
  const onC = doc.add({ kind: 'point', layerId: 'c', p: { x: 2, y: 2 }, attrs: {} });
  return { doc, unsent: new Set([onB.uid]), onC };
}

const top = (layers: readonly LayerInit[]) => layers.map((n) => [n.id, (n.children ?? []).map((c) => c.id)]);

describe('keeping a dropped layer that holds unsent objects', () => {
  it('goes back into its group, at its place, when the tree still has the group', () => {
    const { doc, unsent } = drawing();
    const incoming: LayerInit[] = [{ id: 'g', name: 'Yapılar', type: 'group', children: [{ id: 'a', name: 'Bina' }] }, { id: 'c', name: 'Parsel' }];
    const r = keepUnsentLayers(doc, incoming, (uid) => unsent.has(uid));
    expect(r?.kept).toEqual([{ id: 'b', name: 'Yol', unsent: 1 }]);
    expect(top(r!.layers)).toEqual([['g', ['a', 'b']], ['c', []]]);
    // The incoming tree itself is not changed.
    expect(top(incoming)).toEqual([['g', ['a']], ['c', []]]);
  });

  it('goes to the top when its group went too; a dropped layer whose objects are all sent is not kept', () => {
    const { doc, unsent } = drawing();
    const r = keepUnsentLayers(doc, [{ id: 'd', name: 'Yeni' }], (uid) => unsent.has(uid));
    expect(r?.kept.map((k) => k.id)).toEqual(['b']);
    expect(top(r!.layers)).toEqual([['d', []], ['b', []]]);
  });

  it('keeps nothing when no dropped layer holds an unsent object', () => {
    const { doc } = drawing();
    expect(keepUnsentLayers(doc, [{ id: 'g', name: 'Yapılar', type: 'group', children: [] }], () => false)).toBeNull();
  });
});
