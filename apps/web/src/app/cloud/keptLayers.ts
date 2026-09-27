import type { CadDocument } from '../../model/document';
import { LayerStore, type LayerInit } from '../../model/layers';

/**
 * Another editor's layer tree may drop a layer that still holds this
 * device's unsent objects (drawn there, or moved there, before the removal
 * reached us: the server's guard sees only what it has). Taking the tree as
 * it is would leave those objects on no layer: they could not be sent (the
 * server refuses an object on a layer it lacks) and the drawing could not be
 * saved or opened again. So such a layer stays in this drawing and goes back
 * to the server with them: losing the user's objects is worse than bringing
 * back a layer someone removed. A layer whose objects are all on the server
 * leaves as the other editor wants.
 */

/** A layer kept that way: its id, its name, and how many unsent objects it holds. */
export interface KeptLayer {
  id: string;
  name: string;
  unsent: number;
}

type Tree = readonly LayerInit[];

function ids(nodes: Tree, out = new Set<string>()): Set<string> {
  for (const n of nodes) {
    if (n.id !== undefined) out.add(n.id);
    ids(n.children ?? [], out);
  }
  return out;
}

function find(nodes: LayerInit[], id: string): LayerInit | undefined {
  for (const n of nodes) {
    if (n.id === id) return n;
    const inner = find(n.children ?? [], id);
    if (inner) return inner;
  }
  return undefined;
}

/**
 * `incoming` with the layers of this drawing it drops that hold objects with
 * unsent changes (`unsent(uid)`) put back: each into its group when the
 * incoming tree has that group, else at the top, at its place there when it
 * fits. Null when it drops none of those.
 */
export function keepUnsentLayers(doc: CadDocument, incoming: Tree, unsent: (uid: string) => boolean): { layers: LayerInit[]; kept: KeptLayer[] } | null {
  const present = ids(incoming);
  const kept: KeptLayer[] = [];
  for (const leaf of doc.layers.leaves()) {
    if (present.has(leaf.id)) continue;
    const count = doc.byLayer(leaf.id).filter((e) => {
      const uid = doc.uidOf(e.id);
      return uid !== undefined && unsent(uid);
    }).length;
    if (count) kept.push({ id: leaf.id, name: leaf.name, unsent: count });
  }
  if (!kept.length) return null;
  const layers = structuredClone(incoming) as LayerInit[];
  for (const k of kept) {
    const node = structuredClone(doc.layers.get(k.id)!) as LayerInit;
    const place = doc.layers.placeOf(k.id)!;
    const group = place.parent === null ? undefined : find(layers, place.parent);
    const list = group && group.type !== 'layer' ? (group.children ??= []) : layers;
    list.splice(Math.min(place.index, list.length), 0, node);
  }
  return { layers, kept };
}

/** A tree as the drawing writes it (`metaParts`), for comparing with the drawing's own. */
export function treeText(layers: Tree): string {
  return JSON.stringify(new LayerStore([...layers], '').tree);
}

/** What the user hears for a kept layer. */
export const keptText = (k: KeptLayer): string =>
  `“${k.name}” katmanını başka biri sildi; üzerinde gönderilmemiş ${k.unsent} nesneniz olduğu için katman bu çizimde kaldı ve yeniden kaydedilecek.`;
