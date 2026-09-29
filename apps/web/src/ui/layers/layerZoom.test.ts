import { describe, expect, it } from 'vitest';
import type { AppContext } from '../../app/context';
import { CadDocument } from '../../model/document';
import { LayerStore } from '../../model/layers';
import { objectsOfNode, zoomItem } from './layerZoom';

/**
 * The layer tree's Katmana yakınlaştır and Gruba yakınlaştır (docs/adr/0141): the item's name, whether it
 * can run, and the objects the view goes to.
 */
function setup() {
  const doc = new CadDocument({
    name: 'Deneme',
    layers: new LayerStore(
      [
        { id: 'yol', name: 'Yol' },
        { id: 'bos', name: 'Boş' },
        {
          id: 'binalar',
          name: 'Binalar',
          children: [
            { id: 'ev', name: 'Ev' },
            { id: 'ahir', name: 'Ahır' },
            { id: 'bos-alt', name: 'Boş alt' },
          ],
        },
        { id: 'bos-grup', name: 'Boş grup', children: [{ id: 'bos-yaprak', name: 'Yaprak' }] },
      ],
      'yol',
    ),
    origin: { x: 0, y: 0 },
  });
  const point = (layerId: string, x: number) => doc.add({ kind: 'point', layerId, attrs: {}, p: { x, y: 0 } });
  const yol = point('yol', 1);
  const ev = point('ev', 2);
  const ev2 = point('ev', 3);
  const ahir = point('ahir', 4);
  const zoomed: number[][] = [];
  const ctx = { doc, view: { zoomToObjects: (ids: number[]) => (zoomed.push([...ids]), true) } } as unknown as AppContext;
  const node = (id: string) => doc.layers.get(id)!;
  return { ctx, node, zoomed, ids: { yol: yol.id, ev: ev.id, ev2: ev2.id, ahir: ahir.id } };
}

describe('Katmana yakınlaştır', () => {
  it('is Katmana yakınlaştır on a layer and Gruba yakınlaştır on a group', () => {
    const { ctx, node } = setup();
    expect(zoomItem(ctx, node('yol')).label).toBe('Katmana yakınlaştır');
    expect(zoomItem(ctx, node('binalar')).label).toBe('Gruba yakınlaştır');
  });

  it('goes to the objects of the layer', () => {
    const { ctx, node, zoomed, ids } = setup();
    const item = zoomItem(ctx, node('ev'));
    expect(item.disabled).toBe(false);
    item.run!();
    expect(zoomed).toEqual([[ids.ev, ids.ev2]]);
  });

  it('goes to the objects of every layer of a group', () => {
    const { ctx, node, zoomed, ids } = setup();
    zoomItem(ctx, node('binalar')).run!();
    expect(zoomed).toEqual([[ids.ev, ids.ev2, ids.ahir]]);
    expect(objectsOfNode(ctx, node('binalar'))).toEqual([ids.ev, ids.ev2, ids.ahir]);
  });

  it('is off on a layer or a group without objects', () => {
    const { ctx, node } = setup();
    expect(zoomItem(ctx, node('bos')).disabled).toBe(true);
    expect(zoomItem(ctx, node('bos-alt')).disabled).toBe(true);
    expect(zoomItem(ctx, node('bos-grup')).disabled).toBe(true);
  });

  it('follows the drawing: a layer that gains an object can be gone to', () => {
    const { ctx, node } = setup();
    expect(zoomItem(ctx, node('bos')).disabled).toBe(true);
    ctx.doc.add({ kind: 'point', layerId: 'bos', attrs: {}, p: { x: 9, y: 9 } });
    expect(zoomItem(ctx, node('bos')).disabled).toBe(false);
  });
});
