import type { AppContext } from '../../app/context';
import { resolveMenu, SELECT_FILTER_KINDS } from '../../app/menus';
import type { MenuItem } from '../widgets/PopupMenu';

/**
 * The Süzgeç cell's right-click menu (docs/adr/0187 §5): the kinds the selection filter holds by their short names and
 * their drawing tools' icons, each ticked or not (ticking or unticking one turns the filter on), then every kind or none
 * at once. It is the list Giriş › Seçim süzgeci ▾ shows under its on/off row, and it stays open as rows are ticked. The
 * desktop's is the cell's menu in `apps/desktop/src/selection_commands.rs`.
 */
export function selectFilterMenu(ctx: AppContext): MenuItem[] {
  return resolveMenu(ctx, SELECT_FILTER_KINDS, { checklist: true });
}
