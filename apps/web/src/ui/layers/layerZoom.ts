import type { AppContext } from '../../app/context';
import type { LayerNode } from '../../model/layers';
import type { MenuItem } from '../widgets/PopupMenu';

/** The objects of a layer, or of every layer under a group, in layer order. */
export function objectsOfNode(ctx: Pick<AppContext, 'doc'>, n: LayerNode): number[] {
  return ctx.doc.layers.leavesOf(n.id).flatMap((l) => ctx.doc.byLayer(l.id).map((e) => e.id));
}

/**
 * The layer tree's Katmana yakınlaştır (Gruba yakınlaştır on a group, docs/adr/0141): the view goes to
 * the box of the objects of the layer, or of every layer of the group, and is kept in the view history.
 * Nothing to go to on a layer or group without objects, so the item is off there.
 */
export function zoomItem(ctx: Pick<AppContext, 'doc' | 'view'>, n: LayerNode): MenuItem {
  return {
    label: n.type === 'layer' ? 'Katmana yakınlaştır' : 'Gruba yakınlaştır',
    icon: 'zoomSelection',
    disabled: objectsOfNode(ctx, n).length === 0,
    run: () => void ctx.view.zoomToObjects(objectsOfNode(ctx, n)),
  };
}
