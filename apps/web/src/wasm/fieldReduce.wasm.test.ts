import { describe, expect, it } from 'vitest';
import { fieldReduce, type FieldStation, type Tolerances } from '../model/geom/surveyCalc';

/**
 * The field book's reduction (docs/adr/0169 §3) through the WASM core the app calls, against
 * fixtures/field/v1/reduce.json (scripts/fixtures/field_reduce_cases.py: mpmath, 50 digits, from the rules alone): every
 * row within the file's tolerances and the project's tolerances it is above, each observation's face, the observations
 * left out. The core runs the same file natively
 * (crates/shared/geometry-core/tests/field_reduce.rs).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

type Row = Record<string, number | string | number[] | null>;

interface File {
  format: string;
  tolerance: { metres: number; angle: number };
  cases: { name: string; unit: 'grad' | 'deg'; k: number; setup: FieldStation; tolerances?: Tolerances; expect: { rows: Row[]; problems: { observation: number }[]; faces: (number | null)[] } }[];
}

describe('Karne: indirgeme', () => {
  const file = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/field/v1/reduce.json', import.meta.url), 'utf8')) as File;
  const ANGLES = new Set(['hz', 'zenith', 'hzDiff', 'index']);

  it('reduces every case as the reference does', () => {
    expect(file.format).toBe('kentos.field-reduce');
    expect(file.cases.length).toBeGreaterThanOrEqual(10);
    for (const c of file.cases) {
      const got = fieldReduce(c.setup, c.unit, c.k, c.tolerances ?? null);
      expect(got.rows.length, c.name).toBe(c.expect.rows.length);
      got.rows.forEach((g, i) => {
        const w = c.expect.rows[i]!;
        expect([g.target, g.faces, g.observations], c.name).toEqual([w.target, w.faces, w.observations]);
        for (const k of ['hz', 'zenith', 'hzDiff', 'index', 'slope', 'slopeDiff', 'targetHeight', 'horizontal', 'dh'] as const) {
          const [a, b] = [(g as unknown as Row)[k] ?? null, w[k] ?? null];
          if (a === null || b === null) expect(a, `${c.name}: ${k}`).toBe(b);
          else expect(Math.abs((a as number) - (b as number)), `${c.name}: ${k}`).toBeLessThanOrEqual(ANGLES.has(k) ? file.tolerance.angle : file.tolerance.metres);
        }
        expect(g.over, `${c.name}: over`).toEqual(w.over);
      });
      expect(got.faces, `${c.name}: faces`).toEqual(c.expect.faces);
      expect(got.problems.map((p) => p.observation), c.name).toEqual(c.expect.problems.map((p) => p.observation));
    }
  });
});
