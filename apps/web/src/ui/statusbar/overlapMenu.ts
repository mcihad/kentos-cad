import type { AppContext } from '../../app/context';
import { commandItem } from '../../app/menus';
import type { LayerNode } from '../../model/layers';
import { layerSwatch } from '../layers/swatch';
import type { MenuItem } from '../widgets/PopupMenu';

/**
 * The Çakışma cell's right-click menu (docs/adr/0162 §1): the overlap control's three modes, and Seçili katmanlarda
 * önle's layers with their swatches (a hidden one says so: its areas do not count while it is hidden). Ticking a layer
 * puts the mode on Seçili katmanlarda önle. The desktop's is the cell's menu in `apps/desktop/src/view.rs`.
 */
export function overlapMenu(ctx: AppContext): MenuItem[] {
  const s = ctx.settings;
  const layers = ctx.doc.layers;
  const layerItems = (): MenuItem[] => {
    const out: MenuItem[] = [];
    const walk = (nodes: readonly LayerNode[]) => {
      for (const n of nodes) {
        if (n.type === 'group') {
          out.push({ kind: 'header', label: layers.path(n.id) });
          walk(n.children);
          continue;
        }
        out.push({
          label: n.name,
          swatch: layerSwatch(n, ctx.view.palette),
          checked: s.overlapLayers.value.has(n.id),
          hint: layers.isVisible(n.id) ? undefined : 'gizli',
          run: () => {
            const next = new Set(s.overlapLayers.value);
            if (!next.delete(n.id)) next.add(n.id);
            s.overlapLayers.set(next);
            s.overlap.set('layers');
            s.overlapLast.set('layers');
          },
        });
      }
    };
    walk(layers.tree);
    return out;
  };
  return [
    { kind: 'header', label: 'Çakışma' },
    commandItem(ctx, 'draft.overlap.allow'),
    commandItem(ctx, 'draft.overlap.layer'),
    commandItem(ctx, 'draft.overlap.layers'),
    { kind: 'separator' },
    { label: 'Katmanlar', icon: 'layers', items: layerItems },
  ];
}
