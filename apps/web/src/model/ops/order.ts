import { op } from '../../wasm/core';

/**
 * The order of values (docs/adr/0217 §6): the core's `ops::order`, one for both platforms. A value is none, a truth
 * value, a number or a text (an expression's results); one without a place (none, or a number that is not finite)
 * goes where Boşlar puts it. Kinds go truth values, numbers, texts; false before true, numbers by value, texts in the
 * natural order; Azalan turns the values round; equal values keep the input's order. The answer is the values' indices
 * in that order. The independent reference is scripts/fixtures/layer_ops_cases.py.
 */
export type OrderKey = boolean | number | string | null;

const orderBy = op<(keys: readonly OrderKey[], descending: boolean, nullsFirst: boolean) => number[]>('orderBy');

export function orderOf(keys: readonly OrderKey[], descending: boolean, nullsFirst: boolean): number[] {
  // JSON writes a number that is not finite as null: none, as the core reads it.
  return orderBy(keys, descending, nullsFirst);
}
