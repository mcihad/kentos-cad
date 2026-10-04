import { describe, expect, it } from 'vitest';
import { fieldPolar, fieldReduce, fieldTraverse, surveyTraverseClosure, type Closure, type FieldStation, type PolarTransfer, type Tolerances, type TraverseTransfer } from '../model/geom/surveyCalc';

/**
 * The field book's reduction (docs/adr/0169 §3) through the WASM core the app calls, against
 * fixtures/field/v1/reduce.json (scripts/fixtures/field_reduce_cases.py: mpmath, 50 digits, from the rules alone): every
 * row within the file's tolerances and the project's tolerances it is above, each observation's face, the observations
 * left out. The core runs the same file natively
 * (crates/shared/geometry-core/tests/all/field_reduce.rs).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

type Row = Record<string, number | string | number[] | null>;

interface File {
  format: string;
  tolerance: { metres: number; angle: number };
  cases: { name: string; unit: 'grad' | 'deg'; k: number; setup: FieldStation; tolerances?: Tolerances; expect: { rows: Row[]; problems: { observation: number }[]; faces: (number | null)[]; polar: Record<'grad' | 'deg', PolarTransfer> | null } }[];
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
      // Kutupsal alım's fields with the first row the back sight, in either unit.
      for (const to of ['grad', 'deg'] as const) {
        const p = fieldPolar(c.setup, c.unit, c.k, c.tolerances ?? null, 0, to);
        const w = c.expect.polar?.[to] ?? null;
        if (!p || !w) {
          expect(p, `${c.name}: polar ${to}`).toBe(w);
          continue;
        }
        expect([p.back, p.left, p.shots.map((s) => s.name)], c.name).toEqual([w.back, w.left, w.shots.map((s) => s.name)]);
        expect(Math.abs(p.backReading - w.backReading)).toBeLessThanOrEqual(file.tolerance.angle);
        p.shots.forEach((s, i) => {
          const ws = w.shots[i]!;
          for (const [a, b, tol] of [[s.reading, ws.reading, file.tolerance.angle], [s.zenith, ws.zenith, file.tolerance.angle], [s.slope, ws.slope, file.tolerance.metres]] as const) expect(Math.abs(a - b), `${c.name}: ${to}`).toBeLessThanOrEqual(tol);
          expect(s.targetHeight ?? null, c.name).toBe(ws.targetHeight ?? null);
        });
      }
      expect(got.problems.map((p) => p.observation), c.name).toEqual(c.expect.problems.map((p) => p.observation));
    }
  });

  it('runs the traverse of a field book as the reference does (fixtures/field/v1/traverse.json)', () => {
    const t = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/field/v1/traverse.json', import.meta.url), 'utf8')) as {
      format: string;
      tolerance: { metres: number; angle: number };
      cases: { name: string; unit: 'grad' | 'deg'; k: number; back: string; fore: string | null; to: 'grad' | 'deg'; twoWay?: number; book: FieldStation[]; expect: TraverseTransfer }[];
      closures: { unit: 'grad' | 'deg'; angleMisclosure?: number; linearMisclosure?: number; angle?: number; coord?: number; angleOver?: boolean; coordOver?: boolean }[];
    };
    expect(t.format).toBe('kentos.field-traverse');
    expect(t.cases.length).toBeGreaterThanOrEqual(6);
    const near = (a: number | null | undefined, b: number | null | undefined, tol: number, what: string) => {
      if (a === null || a === undefined || b === null || b === undefined) expect(a ?? null, what).toBe(b ?? null);
      else expect(Math.abs(a - b), what).toBeLessThanOrEqual(tol);
    };
    for (const c of t.cases) {
      const got = fieldTraverse(c.book, c.unit, c.k, c.twoWay === undefined ? null : { twoWay: c.twoWay }, c.back, c.fore, c.to);
      expect([got.stations, got.missing], c.name).toEqual([c.expect.stations, c.expect.missing]);
      expect(got.angles.length, c.name).toBe(c.expect.angles.length);
      got.angles.forEach((a, i) => near(a, c.expect.angles[i], t.tolerance.angle, `${c.name}: angle ${i}`));
      got.legs.forEach((l, i) => {
        const w = c.expect.legs[i]!;
        expect([l.from, l.to], c.name).toEqual([w.from, w.to]);
        for (const k of ['forward', 'backward', 'mean', 'diff'] as const) near(l[k], w[k], t.tolerance.metres, `${c.name}: ${k}`);
        expect(l.over, `${c.name}: over`).toBe(w.over);
      });
    }
    // Poligon hesabı's misclosures against the project's tolerances.
    expect(t.closures.length).toBeGreaterThanOrEqual(5);
    for (const c of t.closures) {
      const got: Closure = surveyTraverseClosure(c.unit, c.angleMisclosure ?? null, c.linearMisclosure ?? null, c.angle ?? null, c.coord ?? null);
      expect([got.angleOver ?? null, got.coordOver ?? null], JSON.stringify(c)).toEqual([c.angleOver ?? null, c.coordOver ?? null]);
    }
  });
});
