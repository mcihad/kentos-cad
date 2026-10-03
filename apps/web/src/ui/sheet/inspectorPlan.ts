import type { Bindable } from '../../contracts/generated/sheet/Bindable';
import { kindName, KIND_ICON, type SheetProfile } from '../../product/sheet/profile';
import type { ItemView } from '../../product/sheet/view';

/**
 * What the inspector shows for a choice of items (docs/sheet/design.md §11,
 * H4): one item's values, or the values several share, each one they do not
 * share left empty (“—”), so a value typed is given to all; the head's
 * summary; and the properties bound to data with their names. Apart from the
 * DOM (SheetInspector.ts).
 */

/** The value every item has, or null when they differ (or none is chosen). */
export function common<T>(items: readonly ItemView[], of: (i: ItemView) => T, same: (a: T, b: T) => boolean = Object.is): T | null {
  if (!items.length) return null;
  const first = of(items[0]);
  return items.every((i) => same(of(i), first)) ? first : null;
}

/** Numbers are the same when they agree to a thousandth of their unit (a micrometre for millimetres). */
export const sameNumber = (a: number, b: number) => Math.abs(a - b) < 5e-4;

export interface FrameValues {
  readonly left: number | null;
  readonly top: number | null;
  readonly width: number | null;
  readonly height: number | null;
  readonly rotation: number | null;
}

export function frameValues(items: readonly ItemView[]): FrameValues {
  return {
    left: common(items, (i) => i.frame.left, sameNumber),
    top: common(items, (i) => i.frame.top, sameNumber),
    width: common(items, (i) => i.frame.width, sameNumber),
    height: common(items, (i) => i.frame.height, sameNumber),
    rotation: common(items, (i) => i.rotation, sameNumber),
  };
}

export interface Summary {
  readonly icon: string;
  readonly title: string;
  readonly sub: string;
}

/** The inspector's head: one item's kind and name, or how many and of which kinds. */
export function summaryOf(items: readonly ItemView[], profile: SheetProfile): Summary {
  if (items.length === 1) {
    const i = items[0];
    const kind = kindName(profile, i.kind);
    // The name when it says more than the kind (“Harita” named Harita says it once).
    const name = i.name === kind ? null : i.name;
    return { icon: KIND_ICON[i.kind], title: kind, sub: [name, i.detail].filter(Boolean).join(' · ') };
  }
  const kinds = new Map<string, number>();
  for (const i of items) {
    const k = kindName(profile, i.kind);
    kinds.set(k, (kinds.get(k) ?? 0) + 1);
  }
  const sameKind = kinds.size === 1 ? items[0].kind : null;
  return {
    icon: sameKind ? KIND_ICON[sameKind] : 'selectAll',
    title: `${items.length} öğe seçili`,
    sub: [...kinds].map(([k, n]) => `${n} ${k.toLocaleLowerCase('tr-TR')}`).join(', '),
  };
}

/**
 * One item's bound properties (design §7) in the engine's order of what may
 * be bound (`bindableProperties`), each with the engine's name for it; a
 * path the engine does not list (an older file's) is shown as it is.
 */
export function bindingsOf(item: ItemView, bindables: readonly Bindable[]): { path: string; label: string; unit: string; expression: string }[] {
  const order = bindables.map((b) => b.property);
  const rank = (p: string) => (order.includes(p) ? order.indexOf(p) : order.length);
  return Object.entries(item.bindings)
    .map(([path, expression]) => {
      const b = bindables.find((x) => x.property === path);
      return { path, label: b?.label ?? path, unit: b?.unit ?? '', expression };
    })
    .sort((a, b) => rank(a.path) - rank(b.path) || a.path.localeCompare(b.path));
}

/** What may be bound on items of these kinds: the properties of every kind, and the kind's own when all are one kind. */
export function bindablesFor(items: readonly ItemView[], bindables: readonly Bindable[]): Bindable[] {
  const kind = common(items, (i) => i.kind);
  return bindables.filter((b) => !b.kinds.length || (kind !== null && b.kinds.includes(kind)));
}

/** Whether all the chosen items are in one group (the constraint box may then be the group). */
export const sameGroup = (items: readonly ItemView[]): boolean => items.length > 0 && items.every((i) => i.group !== null && i.group === items[0].group);

/** The engine's notes for chosen items another work mode's tools made (design §11a), each said once. */
export const notesOf = (items: readonly ItemView[]): string[] => [...new Set(items.map((i) => i.note).filter((m): m is string => !!m))];
