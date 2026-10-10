import type { Entity } from '../entities';
import { variableKey } from '../projectVariables';
import type { ExprLayer, ExprLayers } from './expression';

/**
 * The layers a host gives an expression's calls to other objects (docs/adr/0214 §3): by the name the tree shows
 * (the first leaf of that name, Turkish letters and case aside, as the core's `layer_key`) and by id, each with its
 * objects in the document's order. `leaves` are the tree's layers in order, `byLayer` a layer's objects.
 */
export function exprLayers(leaves: readonly (readonly [string, string])[], byLayer: (layerId: string) => readonly Entity[]): ExprLayers {
  const made = new Map<string, ExprLayer>();
  const of = (id: string, name: string): ExprLayer => {
    let l = made.get(id);
    if (!l) {
      l = { name, entities: byLayer(id) };
      made.set(id, l);
    }
    return l;
  };
  const key = (name: string) => variableKey(name.trim());
  return {
    named(name) {
      const k = key(name);
      const leaf = leaves.find(([, n]) => key(n) === k);
      return leaf ? of(leaf[0], leaf[1]) : null;
    },
    byId(id) {
      const leaf = leaves.find(([i]) => i === id);
      return leaf ? of(id, leaf[1]) : null;
    },
  };
}

/** The objects of the layers an expression looks at (docs/adr/0214 §3): the inputs' own for an aggregate, then the named. */
export function worldObjects(world: { readonly own: boolean; readonly layers: readonly string[] } | null, inputs: readonly Entity[], layers: ExprLayers): Entity[] {
  if (!world) return [];
  const out: Entity[] = [];
  const seen = new Set<ExprLayer>();
  const take = (l: ExprLayer | null) => {
    if (l && !seen.has(l)) {
      seen.add(l);
      out.push(...l.entities);
    }
  };
  if (world.own) for (const id of new Set(inputs.map((e) => e.layerId))) take(layers.byId(id));
  for (const name of world.layers) take(layers.named(name));
  return out;
}
