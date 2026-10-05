import { describe, expect, it } from 'vitest';
import { dataCompare, type CompareMember, type CompareRow, type CompareSettings } from './compare';

/**
 * Veri karşılaştır (docs/adr/0179) through WASM against the shared cases (fixtures/compare/v1/cases.json, written by
 * scripts/fixtures/compare_cases.py from the ADR, not KentOS code); the core runs them natively
 * (crates/shared/geometry-core/src/ops/compare.rs).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

/** A row as the cases write it: none is null. */
type CaseRow = { [K in keyof CompareRow]-?: Exclude<CompareRow[K], undefined> | (K extends 'status' | 'fields' ? never : null) };

interface File {
  format: string;
  cases: { name: string; old: CompareMember[]; new: CompareMember[]; settings: CompareSettings; rows: CaseRow[] }[];
}

const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/compare/v1/cases.json', import.meta.url), 'utf8')) as File;

describe('Veri karşılaştır (docs/adr/0179)', () => {
  it('pairs and compares as the shared cases say', () => {
    expect(file.format).toBe('kentos.compare-cases');
    expect(file.cases.length).toBeGreaterThanOrEqual(5);
    for (const c of file.cases) {
      const rows = dataCompare(c.old, c.new, c.settings);
      expect(rows.length, c.name).toBe(c.rows.length);
      rows.forEach((r, i) => {
        const w = c.rows[i];
        expect({ status: r.status, old: r.old ?? null, new: r.new ?? null, fields: r.fields }, `${c.name}: ${i}`).toEqual({
          status: w.status,
          old: w.old,
          new: w.new,
          fields: w.fields,
        });
        if (w.distance === null) expect(r.distance, `${c.name}: ${i}`).toBeUndefined();
        else expect(Math.abs((r.distance ?? NaN) - w.distance), `${c.name}: ${i}`).toBeLessThanOrEqual(1e-9);
      });
    }
  });
});
