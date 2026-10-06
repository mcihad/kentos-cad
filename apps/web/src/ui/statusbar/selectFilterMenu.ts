import type { AppContext } from '../../app/context';
import { commandItem } from '../../app/menus';
import { ENTITY_KIND_LABEL } from '../../model/entities';
import { FILTER_KINDS } from '../../tools/selectable';
import { ENTITY_KIND_ICON } from '../kindIcons';
import type { MenuItem } from '../widgets/PopupMenu';

/**
 * The Süzgeç cell's right-click menu (docs/adr/0187 §5): the kinds the selection filter holds, each ticked or not
 * (ticking or unticking one turns the filter on), then every kind or none at once. The desktop's is the cell's menu in
 * `apps/desktop/src/view.rs`.
 */
export function selectFilterMenu(ctx: AppContext): MenuItem[] {
  const s = ctx.settings;
  const all = (on: boolean) => () => {
    s.selectKinds.set(new Set(on ? FILTER_KINDS : []));
    s.selectFilter.set(true);
  };
  return [
    { kind: 'header', label: 'Seçilebilir türler' },
    // Each kind by its name and its drawing tool's icon, ticked or not (the desktop's menu alike).
    ...FILTER_KINDS.map((k) => commandItem(ctx, `edit.selectFilter.${k}`, { label: ENTITY_KIND_LABEL[k], icon: ENTITY_KIND_ICON[k] })),
    { kind: 'separator' },
    { label: 'Bütün türler', icon: 'selectAll', run: all(true) },
    { label: 'Hiçbir tür', icon: 'deselect', run: all(false) },
  ];
}
