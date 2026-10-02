import type { AppContext } from '../../app/context';
import { sameSnap, type LayerNode, type LayerSnap, type LayerStore } from '../../model/layers';
import type { MenuItem } from '../widgets/PopupMenu';

/**
 * A layer's own snapping in the Katmanlar panel (docs/adr/0163 §4): what a row's magnet shows and the row menu's
 * Kenet ▸. A group's magnet and menu write to all its layers (a group keeps none). The desktop's is
 * `apps/desktop/src/layer_snap.rs`.
 */

/** What a row's magnet shows: the general kinds, off, or kinds of its own (a group's: of some of its layers). */
export type SnapState = 'none' | 'off' | 'kinds';

/** What a magnet says, by its state (the desktop's tips are the same words). */
export const SNAP_TIP: Record<SnapState, string> = {
  none: 'Kenet: genel türler; kapatmak için tıklayın',
  kinds: 'Kenet: katmanın kendi türleri; kapatmak için tıklayın',
  off: 'Kenet kapalı; açmak için tıklayın',
};

/** The snap kinds a layer can keep to, in the Kenet menu's order (contracts' `LAYER_SNAP_KINDS`). */
export const SNAP_KINDS = ['endpoint', 'midpoint', 'intersection', 'center', 'perpendicular', 'tangent', 'node', 'nearest', 'centroid', 'extension', 'parallel', 'grid'] as const;

/** A row's magnet: a layer's own state; a group's off when all its layers are, general when all are, else own kinds. */
export function snapState(layers: LayerStore, n: LayerNode): SnapState {
  const of = (l: LayerNode): SnapState => (l.snap ? (l.snap.off ? 'off' : 'kinds') : 'none');
  if (n.type === 'layer') return of(n);
  const states = new Set(layers.leavesOf(n.id).map(of));
  return states.size === 1 ? [...states][0] : states.size ? 'kinds' : 'none';
}

/** The snapping a node's layers share; `mixed` when they differ. */
export function commonSnap(layers: LayerStore, n: LayerNode): LayerSnap | null | 'mixed' {
  const leaves = layers.leavesOf(n.id);
  const first = leaves[0]?.snap ?? null;
  return leaves.every((l) => sameSnap(l.snap, first)) ? first : 'mixed';
}

/** A magnet's click: off goes back to the general kinds, anything else turns off. */
export function toggledSnap(layers: LayerStore, n: LayerNode): LayerSnap | null {
  return snapState(layers, n) === 'off' ? null : { off: true };
}

/**
 * Kenet ▸: the general kinds, off, or only some kinds. A kind ticked makes the layer's own list from what it takes now
 * (the general kinds when it has none, none when it is off); the last one unticked turns it off.
 */
export function layerSnapItems(ctx: AppContext, n: LayerNode): MenuItem[] {
  const layers = ctx.doc.layers;
  const common = commonSnap(layers, n);
  const general = SNAP_KINDS.filter((k) => ctx.commands.get(`draft.snap.${k}`)?.isChecked?.());
  const ticked: readonly string[] = common === 'mixed' ? [] : common === null ? general : (common.kinds ?? []);
  return [
    { label: 'Genel türler', radio: true, checked: common === null, run: () => layers.setSnap(n.id, null) },
    { label: 'Kapalı', radio: true, checked: common !== null && common !== 'mixed' && !!common.off, run: () => layers.setSnap(n.id, { off: true }) },
    { kind: 'separator' },
    { kind: 'header', label: 'Yalnız bu türler' },
    ...SNAP_KINDS.map((k): MenuItem => {
      const cmd = ctx.commands.get(`draft.snap.${k}`);
      const on = ticked.includes(k);
      return {
        label: cmd?.short ?? k,
        icon: cmd?.icon,
        checked: on,
        run: () => {
          const kinds = SNAP_KINDS.filter((x) => (x === k ? !on : ticked.includes(x)));
          layers.setSnap(n.id, kinds.length ? { kinds } : { off: true });
        },
      };
    }),
  ];
}
