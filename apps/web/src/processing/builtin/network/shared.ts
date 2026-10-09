import type { NetworkDef } from '../../../contracts/generated/NetworkDef';
import { fixed } from '../../../core/displayNumber';
import type { Entity } from '../../../model/entities';
import type { NetworkPlace } from '../../../model/networkAnswers';
import { networkInput } from '../../../model/networkInput';
import { costNames } from '../../../model/networkRules';
import { CoreStore, type CoreNetwork } from '../../../wasm/core';
import { packEntities } from '../../../wasm/pack';
import type { Feedback, NetworkValue, RunContext } from '../../types';

/**
 * What Ağ analizi's İşlemler tools share (docs/adr/0209 §7, §10; the desktop's `builtin/network/mod.rs`): the run's
 * network built in a store of its own from its layers' objects as the run sees them (the page's or the worker's copy),
 * the points of a features input found on it, the names the results say, and costs written as numbers.
 */

/** A network built for a run: its definition, the cost asked with (its place), and the core's copy (`free` it). */
export interface RunNetwork {
  readonly def: NetworkDef;
  readonly cost: number;
  readonly costNames: readonly string[];
  readonly net: CoreNetwork;
}

/** Builds the run's network; says what building said (expressions that did not compile, objects not taken). */
export function runNetwork(value: NetworkValue, ctx: RunContext, feedback: Feedback): RunNetwork {
  const def = value.def;
  if (!def) throw new Error(`“${value.network}” ağı projede yok; Ağlar penceresinden tanımlayın.`);
  const layers = [...new Set([...def.edges.map((l) => l.layer), ...(def.junctions ?? []).map((l) => l.layer)])];
  const store = new CoreStore();
  try {
    const objects = layers.flatMap((l) => ctx.doc.byLayer(l));
    const packed = packEntities(objects);
    store.putPacked(packed.nums, packed.strings);
    const input = networkInput(def, { byLayer: (l) => ctx.doc.byLayer(l), layerName: (id) => ctx.layerName(id) }, store);
    for (const p of input.problems) feedback.warn(`${def.name}: ${p}`);
    const net = store.buildNetwork(JSON.stringify(def), input.edgeIds, input.edgeValues, input.junctionIds, input.junctionValues);
    const summary = JSON.parse(net.summary()) as { skipped: unknown[] };
    if (summary.skipped.length) feedback.warn(`${def.name}: ağın katmanlarında ${summary.skipped.length} nesne çizgi, çoklu çizgi, yay ya da nokta değil; ağa alınmadı.`);
    const names = costNames(def);
    return { def, cost: Math.max(0, names.indexOf(value.cost)), costNames: names, net };
  } finally {
    store.dispose();
  }
}

/** A point object's place (its first point). */
export const pointOf = (e: Entity): readonly [number, number] | null => (e.kind === 'point' ? [e.p.x, e.p.y] : null);

/** The objects found on the network within `reach` (in their order) and those that are not. */
export function placesOf(net: CoreNetwork, objects: readonly Entity[], reach: number): { found: { entity: Entity; at: readonly [number, number] }[]; missing: Entity[] } {
  const found: { entity: Entity; at: readonly [number, number] }[] = [];
  const missing: Entity[] = [];
  for (const e of objects) {
    const p = pointOf(e);
    const place = p ? (JSON.parse(net.locate(p[0], p[1], reach)) as NetworkPlace | null) : null;
    if (place) found.push({ entity: e, at: p! });
    else missing.push(e);
  }
  return { found, missing };
}

/** A point's name in the results: its label, its `Ad`, else its number. */
export const nameOf = (e: Entity): string => e.label?.trim() || e.attrs['Ad']?.trim() || `#${e.id}`;

/** A cost's value as an attribute: the length with the project's decimals, the others with two; empty for none. */
export const costValue = (v: number | null | undefined, c: number, lengthDecimals: number): string => (v === null || v === undefined || !Number.isFinite(v) ? '' : fixed(v, c === 0 ? lengthDecimals : 2));

/** The note about points not found on the network. */
export function missingNote(missing: readonly Entity[], what: string, reach: number, feedback: Feedback): void {
  if (missing.length) feedback.warn(`${missing.length} ${what} ağa ${fixed(reach, 2)} m içinde değil (ya da nokta değil); alınmadı.`);
}
