import type { Entity } from '../../model/entities';
import { compileExpression, expressionError } from '../../model/expression/expression';
import type { LayerRenderer, Rule } from '../../model/style';
import { geometryClassOf } from '../../style/geometry';

/**
 * What the layer style window counts as the user edits: which objects each
 * category, class and rule takes, as the style core draws them
 * (crates/shared/style-core/src/style/resolve.rs), so the Nesne column says
 * what the map will show. Only objects the style engine draws count (texts
 * and dimensions are drawn elsewhere). An object goes to the first category
 * of its value and to the first class that takes its number; a child rule
 * counts among its parent's objects, a “değilse” rule what no sibling took.
 * Held to fixtures/style/v1/tally.json, as the desktop's
 * (crates/native/style/src/tally.rs).
 */

type Categorized = Extract<LayerRenderer, { type: 'categorized' }>;
type Graduated = Extract<LayerRenderer, { type: 'graduated' }>;

/** Whether the style engine draws each object. */
export const drawable = (entities: readonly Entity[]): boolean[] => entities.map((e) => geometryClassOf(e) !== null);

/**
 * How many drawn objects each category takes, and how many none does
 * (“Diğer değerler”). An object without a value has the value '', as the
 * core reads it; a switched-off category keeps its objects.
 */
export function categoryTally(values: readonly (string | null)[], drawn: readonly boolean[], categories: Categorized['categories']): { counts: number[]; rest: number } {
  const first = new Map<string, number>();
  categories.forEach((k, i) => {
    if (!first.has(k.value)) first.set(k.value, i);
  });
  const counts = categories.map(() => 0);
  let rest = 0;
  values.forEach((v, i) => {
    if (!drawn[i]) return;
    const k = first.get(v ?? '');
    if (k === undefined) rest++;
    else counts[k]++;
  });
  return { counts, rest };
}

/** Whether an earlier category has the same value: this one never draws. */
export const shadowed = (categories: Categorized['categories'], i: number): boolean => categories.slice(0, i).some((o) => o.value === categories[i]?.value);

/**
 * How many drawn objects each class takes (the first that takes the number:
 * its lower bound included, its upper bound too for the last), and how many
 * none does (no number, or outside every class): those are not drawn.
 */
export function classTally(numbers: readonly (number | null)[], drawn: readonly boolean[], classes: Graduated['classes']): { counts: number[]; rest: number } {
  const counts = classes.map(() => 0);
  let rest = 0;
  const last = classes.length - 1;
  numbers.forEach((n, i) => {
    if (!drawn[i]) return;
    const k = n === null ? -1 : classes.findIndex((c, j) => n >= c.min && (n < c.max || (j === last && n <= c.max)));
    if (k < 0) rest++;
    else counts[k]++;
  });
  return { counts, rest };
}

/** What a rule takes, or why its condition cannot be read (“12. karakterde: …”). */
export type RuleCount = { n: number } | { error: string };

type Scope = { layerName(id: string): string; measures?(entities: readonly Entity[]): Float64Array };

/**
 * Each rule's count by its path (indices joined with '/'), of the drawn
 * objects. A rule takes its parent's objects that meet its condition (none:
 * all); a “değilse” rule takes its parent's objects that no enabled sibling
 * with a readable condition took. A rule that is off still says what it
 * would take, and does not keep its siblings' objects from “değilse”.
 */
export function ruleCounts(rules: readonly Rule[], entities: readonly Entity[], scope: Scope): Map<string, RuleCount> {
  const masks = new Map<string, boolean[] | string>();
  const measures = scope.measures;
  const maskOf = (filter: string | undefined): boolean[] | string => {
    if (filter === undefined) return entities.map(() => true);
    let m = masks.get(filter);
    if (m === undefined) {
      const c = compileExpression(filter);
      if (!c.ok) m = expressionError(c);
      else {
        const col = c.expr.evaluateAll({ entities, layerName: scope.layerName, measures: measures && (() => measures(entities)) }, 'bool');
        m = entities.map((_, i) => col.value(i) === true);
      }
      masks.set(filter, m);
    }
    return m;
  };
  const out = new Map<string, RuleCount>();
  const walk = (list: readonly Rule[], parent: readonly boolean[], path: readonly number[]) => {
    const own = list.map((r) => (r.isElse ? null : maskOf(r.filter)));
    // What the enabled plain siblings take: the rest is the “değilse” rules'.
    const taken = parent.map((_, i) => list.some((r, k) => r.enabled !== false && Array.isArray(own[k]) && (own[k] as boolean[])[i]));
    list.forEach((r, k) => {
      const key = [...path, k];
      const m = own[k];
      let mask: boolean[];
      if (typeof m === 'string') {
        out.set(key.join('/'), { error: m });
        mask = parent.map(() => false);
      } else {
        mask = m === null ? parent.map((a, i) => a && !taken[i]) : parent.map((a, i) => a && m[i]);
        out.set(key.join('/'), { n: mask.filter(Boolean).length });
      }
      walk(r.children ?? [], mask, key);
    });
  };
  walk(rules, drawable(entities), []);
  return out;
}

/** The number each object's value reads as (null: none), in the objects' order, or the expression's error. */
export function numbersPerObject(entities: readonly Entity[], expr: string, scope: Scope): { values: (number | null)[]; error?: string } {
  const c = compileExpression(expr);
  if (!c.ok) return { values: [], error: expressionError(c) };
  const measures = scope.measures;
  const col = c.expr.evaluateAll({ entities, layerName: scope.layerName, measures: measures && (() => measures(entities)) }, 'textNumber');
  return {
    values: entities.map((_, i) => {
      const v = col.value(i);
      return typeof v === 'number' && Number.isFinite(v) ? v : null;
    }),
  };
}
