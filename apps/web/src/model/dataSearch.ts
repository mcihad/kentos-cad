import type { CadDocument } from './document';
import { ENTITY_KIND_LABEL, type Entity } from './entities';
import { naturalOrder } from './ops/pointEditor';
import type { SearchRecord } from './ops/dataSearch';

/**
 * What Veride ara reads of a drawing (docs/adr/0178 §1, §3): each object's words as a record for the core's
 * `dataSearch` (`./ops/dataSearch.ts`, the matching), and which of them a scope takes. The desktop's is
 * `kentos_interaction::data_search`; `fixtures/search/v1` holds both to the same records.
 */

/**
 * The record of an object: its kind's name, its layer's path, its label as it is stored, its words (a text's text, a
 * leader's note, a dimension's own text), the name of its insert's block (none for an unknown block) and its
 * attributes as they are stored; null when it has no label, text, block name or attribute value that is not blank.
 */
export function recordOf(e: Entity, layerPath: string, blockName: (id: string) => string | undefined): SearchRecord | null {
  const label = e.label ?? null;
  const text = e.kind === 'text' || e.kind === 'leader' || e.kind === 'dimension' ? (e.text ?? null) : null;
  const block = e.kind === 'insert' ? (blockName(e.block) ?? null) : null;
  const attrs = Object.entries(e.attrs);
  const blank = (v: string | null) => !v || !v.trim();
  if (blank(label) && blank(text) && blank(block) && attrs.every(([, v]) => blank(v))) return null;
  return { kind: ENTITY_KIND_LABEL[e.kind], layer: layerPath, label, text, block, attrs };
}

/** The objects of a drawing that have something to find, in the drawing's order, with their records. */
export interface SearchIndex {
  /** Each object's slot. */
  ids: number[];
  /** Each object's layer. */
  layerIds: string[];
  records: SearchRecord[];
}

export function searchIndex(doc: CadDocument): SearchIndex {
  const paths = new Map<string, string>();
  const path = (id: string) => {
    let p = paths.get(id);
    if (p === undefined) paths.set(id, (p = doc.layers.path(id)));
    return p;
  };
  const names = new Map(doc.blocks.value.map((b) => [b.id, b.name]));
  const out: SearchIndex = { ids: [], layerIds: [], records: [] };
  for (const e of doc.all()) {
    const r = recordOf(e, path(e.layerId), (id) => names.get(id));
    if (!r) continue;
    out.ids.push(e.id);
    out.layerIds.push(e.layerId);
    out.records.push(r);
  }
  return out;
}

/** What a search looks at: one layer's objects (none: every layer's) and only the selected ones (none: all). */
export interface SearchScope {
  layerId: string | null;
  selected: ReadonlySet<number> | null;
}

/** The index positions a scope takes, in the drawing's order. */
export function inScope(index: SearchIndex, scope: SearchScope): number[] {
  const out: number[] = [];
  for (let i = 0; i < index.ids.length; i++) {
    if (scope.layerId !== null && index.layerIds[i] !== scope.layerId) continue;
    if (scope.selected && !scope.selected.has(index.ids[i])) continue;
    out.push(i);
  }
  return out;
}

/** The layers that hold objects with something to find, each with its count, in the layer list's order. */
export function layerCounts(index: SearchIndex, doc: CadDocument): { id: string; name: string; count: number }[] {
  const counts = new Map<string, number>();
  for (const id of index.layerIds) counts.set(id, (counts.get(id) ?? 0) + 1);
  const order = doc.layers.leaves().map((l) => l.id);
  const rank = (id: string) => order.indexOf(id) + 1 || Infinity;
  return [...counts.keys()]
    .sort((a, b) => rank(a) - rank(b))
    .map((id) => ({ id, name: doc.layers.get(id)?.name ?? id, count: counts.get(id) ?? 0 }));
}

/** The attribute names the drawing's objects carry with a value, in the natural order (Ada, Kat 2, Kat 10). */
export function attributeNames(index: SearchIndex): string[] {
  const names = new Set<string>();
  for (const r of index.records) for (const [name, value] of r.attrs) if (value.trim()) names.add(name);
  const list = [...names];
  return naturalOrder(list).map((i) => list[i]);
}
