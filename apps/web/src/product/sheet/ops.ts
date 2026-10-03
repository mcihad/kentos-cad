import type { Item } from '../../contracts/generated/sheet/Item';
import type { Sheet } from '../../contracts/generated/sheet/Sheet';

/**
 * The operations' data the interface puts together before the engine checks
 * and applies them (docs/sheet/design.md §4): the engine makes no ids (the
 * host does), so a copy of items gets new ones here, every reference among
 * the copies follows them (a group's children, a scale bar's map), and a
 * new group item is an empty frame the engine fits to its children. No rule
 * of the sheet is decided here.
 */

/** Items and, for a group among them, everything in it (the drawing order kept). */
export function withChildren(sheet: Sheet, ids: Iterable<string>): string[] {
  const want = new Set(ids);
  let grew = true;
  while (grew) {
    grew = false;
    for (const i of sheet.items)
      if (i.group && want.has(i.group) && !want.has(i.id)) {
        want.add(i.id);
        grew = true;
      }
  }
  return sheet.items.filter((i) => want.has(i.id)).map((i) => i.id);
}

/** A name not on the sheet yet: the name, then “(2)”, “(3)” … */
export function freeName(sheet: Sheet, name: string, taken: Set<string> = new Set(sheet.items.map((i) => i.name))): string {
  if (!taken.has(name)) return name;
  const base = name.replace(/ \(\d+\)$/, '');
  for (let n = 2; ; n++) if (!taken.has(`${base} (${n})`)) return `${base} (${n})`;
}

/**
 * Copies of items (with their groups' children), new ids from `newId`, names
 * the sheet does not have, frames moved by `offset` µm. Ids inside the
 * copies that name copied items are renamed with them (a group's children, a
 * scale bar's or a legend's map); the others are kept (a copy of a scale bar
 * still reads the original map).
 */
export function copyItems(sheet: Sheet, ids: Iterable<string>, newId: () => string, offset: number): Item[] {
  const chosen = withChildren(sheet, ids);
  const map = new Map(chosen.map((id) => [id, newId()]));
  const taken = new Set(sheet.items.map((i) => i.name));
  return sheet.items
    .filter((i) => map.has(i.id))
    .map((i) => {
      let text = JSON.stringify(i);
      for (const [from, to] of map) text = text.split(JSON.stringify(from)).join(JSON.stringify(to));
      const copy = JSON.parse(text) as Item;
      const name = freeName(sheet, i.name, taken);
      taken.add(name);
      return { ...copy, name, frame: { ...copy.frame, left: copy.frame.left + offset, top: copy.frame.top + offset } };
    });
}

/** A new group item: an empty frame the engine makes its children's union. */
export function groupItem(id: string, name: string): Item {
  return {
    id,
    name,
    frame: { left: 0, top: 0, width: 1000, height: 1000 },
    rotation: 0,
    constraints: { h: 'left', v: 'top', relativeTo: 'margins' },
    locked: false,
    hidden: false,
    printable: true,
    opacity: 100,
    padding: 0,
    bindings: [],
    kind: { type: 'group' } as Item['kind'],
  };
}
