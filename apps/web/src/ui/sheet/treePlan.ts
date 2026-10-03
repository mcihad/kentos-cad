import type { ItemView, SheetView } from '../../product/sheet/view';

/**
 * The item tree's shape and where a dragged row lands (ItemTree.ts): the
 * sheet's items front first (the reverse of the drawing order), a group's
 * items under it, the master page's under one row at the bottom; and, for a
 * row dropped on another, whether it goes in front of it, behind it or into
 * it (a group), as the drawing order's `before` and the group. Apart from
 * the DOM.
 */

/** The row the master page's items hang under. */
export const MASTER_NODE = '@ana-sayfa';

export interface ItemNode {
  readonly id: string;
  readonly label: string;
  readonly item: ItemView | null;
  readonly children: readonly ItemNode[];
}

/** The tree's roots: own items front first, groups with their items, then the master page's row. */
export function itemNodes(sheet: SheetView): ItemNode[] {
  const own = sheet.items.filter((i) => !i.master);
  const under = (group: string | null): ItemNode[] =>
    own
      .filter((i) => i.group === group)
      .reverse()
      .map((i) => ({ id: i.id, label: i.name, item: i, children: i.kind === 'group' ? under(i.id) : [] }));
  const roots = under(null);
  const masters = sheet.items.filter((i) => i.master).reverse();
  if (masters.length) roots.push({ id: MASTER_NODE, label: sheet.master ? `Ana sayfa: ${sheet.master}` : 'Ana sayfa', item: null, children: masters.map((i) => ({ id: i.id, label: i.name, item: i, children: [] })) });
  return roots;
}

export type DropZone = 'before' | 'after';

export interface Drop {
  /** Where the row shows the mark: above it (in front) or below it (behind). */
  readonly zone: DropZone;
  /** The drawing order's `before`: the item the dragged ones go under, or null for the top of their group. */
  readonly before: string | null;
  /** The group they are in (null: the sheet itself). */
  readonly group: string | null;
}

/**
 * Where items dropped on a row at `y` (0 at its top, 1 at its bottom) go:
 * the upper half puts them in front of the row's item, the lower half
 * behind it. Only among their own siblings (an item changes its group with
 * Grupla and Grubu çöz, not by a drag), never on themselves or among the
 * master page's items.
 */
export function dropOf(sheet: SheetView, dragged: readonly string[], target: string, y: number): Drop | null {
  const t = sheet.items.find((i) => i.id === target);
  if (!t || t.master || dragged.includes(target)) return null;
  const moving = sheet.items.filter((i) => dragged.includes(i.id));
  if (!moving.length || moving.some((i) => i.group !== t.group)) return null;
  const siblings = sheet.items.filter((i) => !i.master && i.group === t.group && !dragged.includes(i.id));
  const at = siblings.indexOf(t);
  if (y <= 0.5) return { zone: 'before', before: siblings[at + 1]?.id ?? null, group: t.group };
  return { zone: 'after', before: t.id, group: t.group };
}

/**
 * The sheet's whole drawing order after a drop (`SetOrder`): the dragged
 * items, in their own order, put just under `before`; with none, on top of
 * their group (just under the group item, which stands above its
 * children) or on top of the sheet.
 */
export function orderAfterDrop(sheet: SheetView, dragged: readonly string[], drop: Drop): string[] {
  const own = sheet.items.filter((i) => !i.master).map((i) => i.id);
  const moving = own.filter((id) => dragged.includes(id));
  const rest = own.filter((id) => !dragged.includes(id));
  let at = rest.length;
  if (drop.before) at = rest.indexOf(drop.before);
  else if (drop.group) at = rest.indexOf(drop.group);
  if (at < 0) at = rest.length;
  return [...rest.slice(0, at), ...moving, ...rest.slice(at)];
}
