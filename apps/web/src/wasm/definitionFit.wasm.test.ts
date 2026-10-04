import { describe, expect, it } from 'vitest';
import { buildDefinition, emptyForm, FIT_TEXTS, fitPlane, planeTexts, unreadRows, type FitRow, type PlaneKind } from '../model/definitionForm';

/**
 * Özel koordinat sistemi's Ortak noktalardan hesapla (docs/adr/0168 §1, §6) through the WASM core the app calls, against
 * fixtures/crs/v1/definition-fit.json (scripts/fixtures/crs_definition_fit_cases.py: Vektör oturtma's independent
 * least-squares reference, exact fractions): the plane, its rows, residuals and m0 within the file's tolerances, the rows
 * left out, or the same words; the plane written into the fields builds the same definition. The desktop reads the same
 * file (crates/native/project/src/definition_form.rs).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface Expect {
  plane: Record<string, number | string>;
  rows: number[];
  skipped: number[];
  residuals: [number, number, number][];
  m0: number | null;
}

interface File {
  format: string;
  tolerance: { metres: number; relative: number; degrees: number };
  texts: Record<string, string>;
  cases: { name: string; plane: PlaneKind; typed: FitRow[]; expect?: Expect; problem?: string }[];
}

describe('Özel koordinat sistemi: Ortak noktalardan hesapla', () => {
  const file = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/crs/v1/definition-fit.json', import.meta.url), 'utf8')) as File;
  const { metres, relative, degrees } = file.tolerance;
  const near = (got: number, want: number, abs: number, what: string) => expect(Math.abs(got - want), `${what}: ${got} ≠ ${want}`).toBeLessThanOrEqual(Math.max(abs, relative * Math.abs(want)));

  it('says what the shared cases say', () => {
    expect(file.format).toBe('kentos.crs-definition-fit');
    for (const [key, text] of Object.entries(FIT_TEXTS)) expect(file.texts[key], key).toBe(text);
  });

  it('gives the shared cases’ plane', () => {
    expect(file.cases.length).toBeGreaterThanOrEqual(13);
    for (const c of file.cases) {
      const got = fitPlane(c.typed, c.plane);
      if (c.problem !== undefined) {
        expect(got, c.name).toEqual({ problem: c.problem });
        continue;
      }
      if ('problem' in got) throw new Error(`${c.name}: ${got.problem}`);
      const want = c.expect!;
      const p = got.plane as unknown as Record<string, number | string>;
      expect(p.kind, c.name).toBe(want.plane.kind);
      for (const [k, v] of Object.entries(want.plane)) {
        if (k === 'kind') continue;
        const abs = k === 'rotation' ? degrees : k === 'east' || k === 'north' || k === 'c' || k === 'f' ? metres : 1e-15;
        near(p[k] as number, v as number, abs, `${c.name}: ${k}`);
      }
      expect(got.rows, c.name).toEqual(want.rows);
      expect(unreadRows(c.typed), c.name).toEqual(want.skipped);
      got.residuals.forEach((r, i) => r.forEach((v, k) => near(v, want.residuals[i]![k]!, metres, `${c.name}: residual`)));
      if (want.m0 === null) expect(got.m0, c.name).toBeNull();
      else near(got.m0!, want.m0, metres, `${c.name}: m0`);
      // The plane goes into the fields and builds again from them.
      const form = { ...emptyForm(), name: 'Ortak', kind: 'local' as const, base: '5254' };
      planeTexts(form, got.plane);
      const built = buildDefinition(form);
      if ('problems' in built) throw new Error(`${c.name}: ${JSON.stringify(built.problems)}`);
      expect(built.definition.system.kind === 'local' && built.definition.system.plane, c.name).toEqual(got.plane);
    }
  });
});
