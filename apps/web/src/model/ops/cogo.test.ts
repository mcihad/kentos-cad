import { describe, expect, it } from 'vitest';
import { cogoCheck, cogoMeasure, cogoRecord, type CogoFinding } from './cogo';

/**
 * Kayıtlı ölçüler (docs/adr/0180) through WASM against the shared cases (fixtures/cogo/v1/cases.json, written by
 * scripts/fixtures/cogo_cases.py from the ADR, not KentOS code); the core runs them natively
 * (crates/shared/geometry-core/src/ops/cogo.rs).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

type Shape = Record<string, unknown>;
interface File {
  format: string;
  measures: { name: string; shape: Shape; measured: Record<string, number> | null }[];
  checks: { name: string; shape: Shape; attrs: Record<string, string>; toleranceLength: number; toleranceCc: number; result: (Omit<CogoFinding, 'items'> & { items: { field: string; recorded: string; measured: number; difference: number | null; over: boolean }[] }) | null }[];
  records: { name: string; text: string; convention: 'gis' | 'cad'; angleUnit: string; lengthUnit: string; recorded: Record<string, string> | null }[];
}

const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/cogo/v1/cases.json', import.meta.url), 'utf8')) as File;
const near = (a: number, b: number, tol: number) => Math.abs(a - b) <= tol * (1 + Math.abs(b));

describe('Kayıtlı ölçüler (docs/adr/0180)', () => {
  it('measures as the shared cases say', () => {
    expect(file.format).toBe('kentos.cogo-cases');
    for (const c of file.measures) {
      const got = cogoMeasure(c.shape);
      if (c.measured === null) {
        expect(got, c.name).toBeNull();
        continue;
      }
      expect(Object.keys(got ?? {}).sort(), c.name).toEqual(Object.keys(c.measured).sort());
      for (const [k, v] of Object.entries(c.measured)) expect(near((got as unknown as Record<string, number>)[k], v, 1e-9), `${c.name}: ${k}`).toBe(true);
    }
  });

  it('checks as the shared cases say', () => {
    for (const c of file.checks) {
      const got = cogoCheck(c.shape, c.attrs, { length: c.toleranceLength, cc: c.toleranceCc });
      if (c.result === null) {
        expect(got, c.name).toBeNull();
        continue;
      }
      expect(got?.status, c.name).toBe(c.result.status);
      expect(got?.items.length, c.name).toBe(c.result.items.length);
      got?.items.forEach((g, i) => {
        const w = c.result!.items[i];
        expect([g.field, g.recorded, g.over], c.name).toEqual([w.field, w.recorded, w.over]);
        expect(near(g.measured, w.measured, 1e-9), c.name).toBe(true);
        if (w.difference === null) expect(g.difference, c.name).toBeUndefined();
        else expect(Math.abs((g.difference ?? NaN) - w.difference), c.name).toBeLessThanOrEqual(1e-6);
      });
    }
  });

  it('records as the shared cases say', () => {
    for (const c of file.records) expect(cogoRecord(c.text, c.convention, c.angleUnit, c.lengthUnit), c.name).toEqual(c.recorded);
  });
});
