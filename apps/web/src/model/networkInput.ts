import type { NetworkDef } from '../contracts/generated/NetworkDef';
import type { Entity } from './entities';
import { compileExpression, expressionError, type ExprGeometry } from './expression/expression';

/**
 * A network's input from the drawing (docs/adr/0209 §2, §3): which objects of its layers are its edges and junctions,
 * with what the app evaluates for them (the core gets only the results): each edge layer's objects its filter holds
 * for, in the definition's order and each layer's document order, each object once; their direction field's value,
 * their costs' fields' values and whether the closed edges' expression holds; each junction layer's points with its
 * role and whether its closed expression holds. A point on a layer that is both an edge and a junction layer is a
 * junction, a line on it an edge; any other kind goes to the core, which says it by its kind. An expression's empty
 * answer is “no”; an expression that does not compile matches nothing and is said. Runs where the network is built:
 * the network worker (io/network/handleNetwork.ts) and the İşlemler tools, in the page or their worker. The desktop's
 * is `kentos_interaction::network::input`.
 */

/** Where the objects come from: a document, a run's copy of it. */
export interface NetworkSource {
  byLayer(layerId: string): readonly Entity[];
  layerName(id: string): string;
}

export interface NetworkInput {
  readonly edgeIds: Float64Array;
  /** `[[direction | null, [cost values], closed], …]`, the edges' order. */
  readonly edgeValues: string;
  readonly junctionIds: Float64Array;
  /** `[[role, closed], …]`, the junctions' order. */
  readonly junctionValues: string;
  /** Expressions that did not compile, in words. */
  readonly problems: readonly string[];
}

/**
 * Which of `list` an expression holds for (true only); `none` for each when there is no expression (a filter takes
 * every object, a closed expression closes none).
 */
function holds(source: string | null | undefined, none: boolean, list: readonly Entity[], src: NetworkSource, geometry: ExprGeometry | undefined, what: string, problems: string[]): boolean[] {
  const text = source?.trim();
  if (!text) return list.map(() => none);
  const r = compileExpression(text);
  if (!r.ok) {
    problems.push(`${what} değerlendirilemedi (${expressionError(r)}); bu ifadeye hiçbir nesne uymuş sayılmadı.`);
    return list.map(() => false);
  }
  if (!list.length) return [];
  const c = r.expr.evaluateAll({ entities: list, layerName: (id) => src.layerName(id), geometry }, 'bool');
  return list.map((_, i) => c.value(i) === true);
}

/** An attribute's value as the core reads it: the text, none when the object has no such attribute. */
const value = (e: Entity, field: string): string | null => (Object.hasOwn(e.attrs, field) ? e.attrs[field] : null);

/**
 * The input of network `def` from `src`. `geometry`: the store the objects are in, where expressions read their
 * geometry values (`$uzunluk` …); without one they are computed per object.
 */
export function networkInput(def: NetworkDef, src: NetworkSource, geometry?: ExprGeometry): NetworkInput {
  const problems: string[] = [];
  const edgeLayers = new Set(def.edges.map((l) => l.layer));
  const junctionLayers = new Set((def.junctions ?? []).map((l) => l.layer));
  const taken = new Set<number>();
  const edges: Entity[] = [];
  for (const [i, l] of def.edges.entries()) {
    // A point on a layer that is a junction layer too is that layer's junction.
    const list = src.byLayer(l.layer).filter((e) => !taken.has(e.id) && !(e.kind === 'point' && junctionLayers.has(l.layer)));
    const ok = holds(l.filter, true, list, src, geometry, `${i + 1}. kenar katmanının süzgeci`, problems);
    list.forEach((e, k) => {
      if (ok[k]) {
        taken.add(e.id);
        edges.push(e);
      }
    });
  }
  const closed = holds(def.closed, false, edges, src, geometry, 'Kapalı kenarların ifadesi', problems);
  const field = def.direction.kind === 'field' ? def.direction.field : null;
  const rows = edges.map((e, k) => [field === null ? null : value(e, field), (def.costs ?? []).map((c) => value(e, c.field)), closed[k]]);

  const junctions: Entity[] = [];
  const junctionRows: [string, boolean][] = [];
  const takenJ = new Set<number>();
  for (const [i, l] of (def.junctions ?? []).entries()) {
    // A line on a layer that is an edge layer too is that layer's edge.
    const list = src.byLayer(l.layer).filter((e) => !takenJ.has(e.id) && !(e.kind !== 'point' && edgeLayers.has(l.layer)));
    const ok = holds(l.filter, true, list, src, geometry, `${i + 1}. düğüm katmanının süzgeci`, problems);
    const mine = list.filter((_, k) => ok[k]);
    const shut = holds(l.closed, false, mine, src, geometry, `${i + 1}. düğüm katmanının kapalı ifadesi`, problems);
    mine.forEach((e, k) => {
      takenJ.add(e.id);
      junctions.push(e);
      junctionRows.push([l.role, shut[k]]);
    });
  }
  return {
    edgeIds: Float64Array.from(edges, (e) => e.id),
    edgeValues: JSON.stringify(rows),
    junctionIds: Float64Array.from(junctions, (e) => e.id),
    junctionValues: JSON.stringify(junctionRows),
    problems,
  };
}
