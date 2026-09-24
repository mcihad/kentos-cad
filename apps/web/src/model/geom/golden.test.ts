import { describe, expect, it } from 'vitest';
import text from '../../../../../fixtures/geometry/v1/cases.json?raw';
import { close, run, type GoldenFile } from './goldenCases';

/**
 * Golden geometry cases shared with the Rust core (crates/shared/geometry-core,
 * tests/golden.rs) and its WASM build: the same inputs must give the same
 * results within the tolerance in the file (docs/adr/0002-contracts-fixtures.md).
 * The expected values were recorded once from this TypeScript reference
 * (scripts/fixtures/record-geometry.test.ts, GOLDEN_WRITE=1) and are locked:
 * changing them is a deliberate, reviewed change of behaviour.
 */

const file = JSON.parse(text) as GoldenFile;

describe('golden geometry cases (shared with Rust)', () => {
  it('has a versioned file with every case recorded', () => {
    expect([file.format, file.version]).toEqual(['kentos.geometry-fixtures', 1]);
    // The tolerance is written for projected metres; other systems need their own file and bound.
    expect([file.crs.kind, file.crs.unit]).toEqual(['projected', 'metre']);
    expect(file.cases.every((c) => 'expected' in c)).toBe(true);
    expect(new Set(file.cases.map((c) => c.name)).size).toBe(file.cases.length);
  });
  for (const c of file.cases)
    it(`${c.op}: ${c.name}`, () => {
      expect(close(run(c), c.expected, file.tolerance, c.name)).toBeNull();
    });
});
