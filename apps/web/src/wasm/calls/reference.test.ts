import { describe, expect, it } from 'vitest';
import text from '../../../../../fixtures/geometry/v1/reference-calls.json?raw';
import { callNamed } from '../core';

/**
 * Accuracy against the independent reference for named operations
 * (fixtures/geometry/v1/reference-calls.json: Python fractions and 60-digit
 * roots, scripts/fixtures/geometry_call_reference.py; CLAUDE.md §23.4):
 * the Rust core through WASM (natively: crates/shared/geometry-core/tests/calls.rs).
 * Agreeing with the TypeScript it replaced was not accuracy; the core must
 * stay within each case's bound.
 */
interface RefCase {
  name: string;
  fn: string;
  args: unknown[];
  expect: unknown;
  bound: string;
}
const file = JSON.parse(text) as { format: string; version: number; cases: RefCase[] };

function within(actual: unknown, expected: unknown, bound: number, path: string): string | null {
  if (typeof expected === 'string' && typeof actual === 'number') {
    const err = Math.abs(actual - Number(expected));
    return err <= bound ? null : `${path}: hata ${err} > ${bound}`;
  }
  if (Array.isArray(expected) && Array.isArray(actual) && actual.length === expected.length) {
    for (let i = 0; i < expected.length; i++) {
      const r = within(actual[i], expected[i], bound, `${path}[${i}]`);
      if (r) return r;
    }
    return null;
  }
  if (expected && typeof expected === 'object' && actual && typeof actual === 'object') {
    for (const [k, e] of Object.entries(expected)) {
      const r = within((actual as Record<string, unknown>)[k], e, bound, `${path}.${k}`);
      if (r) return r;
    }
    return null;
  }
  return actual === expected ? null : `${path}: ${JSON.stringify(actual)} ≠ ${JSON.stringify(expected)}`;
}

describe('named operations against the independent reference', () => {
  it('reads a versioned reference file', () => expect([file.format, file.version]).toEqual(['kentos.geometry-call-reference', 1]));
  for (const c of file.cases) {
    it(`Rust (WASM) ${c.fn}: ${c.name}`, () => expect(within(callNamed(c.fn, c.args), c.expect, Number(c.bound), c.name)).toBeNull());
  }
});
