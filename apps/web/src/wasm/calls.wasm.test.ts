import { describe, expect, it } from 'vitest';
import { callNamed, writeArgs } from './core';
import { sameResult, toJson, type CallFile } from './calls/harness';

/**
 * The frozen call fixtures (fixtures/geometry/v1/calls-*.json) through the
 * app's own path into the WASM core; Rust runs the same files natively
 * (crates/shared/geometry-core/tests/calls.rs). The answers were recorded from the
 * TypeScript each operation was ported from, before it was deleted
 * (docs/adr/0008, S3).
 */
const files = import.meta.glob<string>('../../../../fixtures/geometry/v1/calls-*.json', { query: '?raw', import: 'default', eager: true });

for (const [path, text] of Object.entries(files)) {
  const file = JSON.parse(text) as CallFile;
  describe(`WASM: ${path.split('/').pop()}`, () => {
    it('is a call fixture for projected metres', () => {
      expect(file.format).toBe('kentos.geometry-calls');
      expect(file.version).toBe(1);
      expect(file.crs.unit).toBe('metre');
      expect(file.cases.length).toBeGreaterThan(0);
    });
    it('every case matches', () => {
      const failures = file.cases.map((c) => sameResult(toJson(callNamed(c.fn, c.args)), c.expect, c.tol ?? file.tolerance, `${c.fn}: ${c.name}`)).filter(Boolean);
      expect(failures.slice(0, 5).join('\n')).toBe('');
    });
  });
}

describe('arguments', () => {
  it('keep NaN and infinities: JSON alone would turn them into null', () => {
    expect(writeArgs([1 / 0, -1 / 0, NaN, null, undefined, 0, 'a'])).toBe('["#Inf","#-Inf","#NaN",null,null,0,"a"]');
    expect(writeArgs([{ x: 1, y: 2 }, [3]])).toBe('[{"x":1,"y":2},[3]]');
    // A distance of 1 / 0 along a line stays infinite in the core.
    expect(callNamed('alongLine', [{ x: 0, y: 0 }, { x: 3, y: 4 }, Infinity])).toEqual({ x: Infinity, y: Infinity });
    expect(callNamed('alongLine', [{ x: 0, y: 0 }, { x: 10, y: 0 }, NaN])).toEqual({ x: NaN, y: NaN });
  });
});
