import type { LayerState } from '../contracts/generated/LayerState';
import type { LayerStateNode } from '../contracts/generated/LayerStateNode';
import type { LayerNode, LayerStyle } from './layers';

/**
 * The layer states' rules (docs/adr/0177 §4; the desktop's `kentos_domain::layer_states`, shared cases
 * fixtures/layers/v1/states.json from scripts/fixtures/layer_state_cases.py): what a state keeps of the tree, what
 * applying it changes and whether the tree is in it.
 */

/** What a state keeps besides the visibility. */
export interface LayerStateParts {
  readonly locks: boolean;
  readonly styles: boolean;
}

/** Every node of the tree in its order (a group before what it holds). */
function walk(nodes: readonly LayerNode[], out: LayerNode[] = []): LayerNode[] {
  for (const n of nodes) {
    out.push(n);
    walk(n.children, out);
  }
  return out;
}

/** The tree as a state keeps it: every node in its order with its own visibility; with `locks` its own lock, with `styles` a layer's style. */
export function captureLayerState(tree: readonly LayerNode[], id: string, name: string, parts: LayerStateParts): LayerState {
  const nodes = walk(tree).map(
    (n): LayerStateNode => ({
      node: n.id,
      visible: n.visible,
      ...(parts.locks && { locked: n.locked }),
      ...(parts.styles && n.type === 'layer' && { style: structuredClone(n.style) }),
    }),
  );
  return { id, name, nodes };
}

/** What applying a state changes, in its order; `missing` counts its nodes the tree no longer has. */
export interface LayerStateChanges {
  readonly visible: readonly [string, boolean][];
  readonly locked: readonly [string, boolean][];
  readonly styles: readonly [string, LayerStyle][];
  readonly missing: number;
}

/** Two styles alike, whatever the order of their keys (the renderer's JSON too). */
function same(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (typeof a !== 'object' || typeof b !== 'object' || a === null || b === null) return false;
  if (Array.isArray(a) !== Array.isArray(b)) return false;
  const ka = Object.keys(a).filter((k) => (a as Record<string, unknown>)[k] !== undefined);
  const kb = Object.keys(b).filter((k) => (b as Record<string, unknown>)[k] !== undefined);
  return ka.length === kb.length && ka.every((k) => same((a as Record<string, unknown>)[k], (b as Record<string, unknown>)[k]));
}

/**
 * What applying `state` to the tree changes: the visibility and the lock (when kept) of the nodes it names that the tree
 * still has, the style (when kept) of those that are layers; a node the tree lacks is counted, one the state does not
 * name stays.
 */
export function layerStateChanges(tree: readonly LayerNode[], state: LayerState): LayerStateChanges {
  const by = new Map(walk(tree).map((n) => [n.id, n]));
  const visible: [string, boolean][] = [];
  const locked: [string, boolean][] = [];
  const styles: [string, LayerStyle][] = [];
  let missing = 0;
  for (const e of state.nodes) {
    const n = by.get(e.node);
    if (!n) {
      missing++;
      continue;
    }
    if (n.visible !== e.visible) visible.push([e.node, e.visible]);
    if (e.locked !== undefined && n.locked !== e.locked) locked.push([e.node, e.locked]);
    if (e.style !== undefined && n.type === 'layer' && !same(n.style, e.style)) styles.push([e.node, e.style as LayerStyle]);
  }
  return { visible, locked, styles, missing };
}

/** Whether the tree is in the state: one of its nodes at least is in the tree, and applying it would change nothing. */
export function layerStateMatches(tree: readonly LayerNode[], state: LayerState): boolean {
  const c = layerStateChanges(tree, state);
  return state.nodes.length - c.missing > 0 && !c.visible.length && !c.locked.length && !c.styles.length;
}
