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

/** A node of a tree with its parent group's id (null at the top) and its place there. */
function locate(nodes: Tree, id: string, parent: string | null = null): { node: LayerInit; parent: string | null; index: number } | undefined {
  for (const [index, n] of nodes.entries()) {
    if (n.id === id) return { node: n, parent, index };
    const inner = locate(n.children ?? [], id, n.id ?? null);
    if (inner) return inner;
  }
  return undefined;
}

/**
 * `into` with the nodes `ids` of `from` put in, as copies: each into its
 * parent group in `from` when `into` has that group, else at the top; at its
 * place there when it fits. `into` itself is not changed.
 */
export function withNodesFrom(into: Tree, from: Tree, ids: readonly string[]): LayerInit[] {
  const out = structuredClone(into) as LayerInit[];
  for (const id of ids) {
    const at = locate(from, id);
    if (!at) continue;
    const group = at.parent === null ? undefined : find(out, at.parent);
    const list = group && group.type !== 'layer' ? (group.children ??= []) : out;
    list.splice(Math.min(at.index, list.length), 0, structuredClone(at.node));
  }
  return out;
}

/** The ids of every node of a tree. */
export function treeIds(nodes: Tree): Set<string> {
  return ids(nodes);
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
  return { layers: withNodesFrom(incoming, doc.layers.tree, kept.map((k) => k.id)), kept };
}

/** A tree as the drawing writes it (`metaParts`), for comparing with the drawing's own. */
export function treeText(layers: Tree): string {
  return JSON.stringify(new LayerStore([...layers], '').tree);
}

/** What the user hears for a layer given back because someone else's objects are still on it. */
export const givenBackText = (name: string): string => `“${name}” katmanında başkasının nesnesi olduğu için katman silinmedi; sizin nesneleriniz silindi.`;

/** What the user hears for a kept layer. */
export const keptText = (k: KeptLayer): string =>
  `“${k.name}” katmanını başka biri sildi; üzerinde gönderilmemiş ${k.unsent} nesneniz olduğu için katman bu çizimde kaldı ve yeniden kaydedilecek.`;
