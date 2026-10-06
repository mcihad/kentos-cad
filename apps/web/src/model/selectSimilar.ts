import type { Entity, EntityKind } from './entities';

/**
 * Benzerini seç's rule (docs/adr/0187 §4): an object is similar when it equals at least one example in every criterion
 * that is on: Tür (its kind, and an insert's block too), Katman, Renk (its own colour; “by layer” is a value of its own)
 * and Sembol (its own symbol). With none on, every object is similar. The desktop's twin is
 * `kentos_interaction::select_similar`; both run fixtures/selection/v1/similar.json, which an independent Python
 * reference writes (scripts/fixtures/selection_cases.py).
 */

export interface SimilarCriteria {
  kind: boolean;
  layer: boolean;
  color: boolean;
  symbol: boolean;
}

/** What an object is compared by. */
export interface SimilarFacts {
  id: number;
  kind: EntityKind;
  /** An insert's block, null for any other object. */
  block: string | null;
  layer: string;
  color: string | null;
  symbol: string | null;
}

export const ALL_CRITERIA: SimilarCriteria = { kind: true, layer: true, color: true, symbol: true };

export function similarFacts(e: Entity): SimilarFacts {
  return {
    id: e.id,
    kind: e.kind,
    block: e.kind === 'insert' ? e.block : null,
    layer: e.layerId,
    color: e.color ?? null,
    symbol: e.symbol ?? null,
  };
}

/** The key the criteria compare: the facts that are on, joined. */
function key(f: SimilarFacts, c: SimilarCriteria): string {
  return JSON.stringify([c.kind ? [f.kind, f.block] : null, c.layer ? f.layer : null, c.color ? f.color : null, c.symbol ? f.symbol : null]);
}

/** The ids of `objects` similar to at least one of `examples`, in the objects' order. */
export function similarTo(objects: readonly SimilarFacts[], examples: readonly SimilarFacts[], c: SimilarCriteria): number[] {
  const wanted = new Set(examples.map((e) => key(e, c)));
  return objects.filter((o) => wanted.has(key(o, c))).map((o) => o.id);
}
