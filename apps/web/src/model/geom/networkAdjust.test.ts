import { describe, expect, it } from 'vitest';
import { SIGMA_DEFAULTS } from '../projectSettings';
import { chi2Quantile, fieldLevels, fieldNetwork, levelAdjust, networkAdjust, type NetworkResult, type Statistics } from './networkAdjust';

/**
 * Ağ dengelemesi ve kot ağı (docs/adr/0203) through the WASM core, against the independent reference in
 * fixtures/network-adjust/v1/cases.json (scripts/fixtures/network_adjust_cases.py, mpmath at 50 digits, no KentOS code),
 * the cases the core runs natively in crates/shared/geometry-core/tests/all/network_adjust.rs.
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

type Num = number | null;
interface Case {
  name: string;
  kind: 'horizontal' | 'level';
  unit?: 'grad' | 'deg';
  levelKind?: 'geometric' | 'trigonometric';
  sigma: Record<string, number>;
  grid?: unknown;
  known: unknown[];
  approx?: unknown[];
  rows: unknown[];
  tolerances?: Record<string, number>;
  error?: string;
  expect?: Statistics & {
    points: Record<string, Num | string>[];
    orientations?: { station: string; z: number }[];
    observations: { kind: string; row: number; v: number; sigma: number; r: number; w: Num; flag: string }[];
    m0: Num;
    chi2: Num;
    passed: boolean | null;
    worst: number | null;
  };
}
interface File {
  format: string;
  defaults: Record<string, number>;
  tolerances: Record<string, number>;
  chi2: { f: number; chi2: number }[];
  cases: Case[];
}

const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/network-adjust/v1/cases.json', import.meta.url), 'utf8')) as File;

const near = (got: number | undefined | null, want: Num, abs: number, rel: number): boolean =>
  want === null ? got === undefined || got === null : got !== undefined && got !== null && Math.abs(got - want) <= Math.max(abs, rel * Math.abs(want));

describe('Ağ dengelemesi ve kot ağı (docs/adr/0203)', () => {
  it('has the reference’s defaults', () => {
    expect(file.format).toBe('kentos.network-adjust-cases');
    expect({ ...SIGMA_DEFAULTS }).toEqual(file.defaults);
  });

  it('adjusts every case as the reference does', () => {
    for (const c of file.cases) {
      const tol = { ...file.tolerances, ...(c.tolerances ?? {}) } as Record<string, number>;
      const run = (): NetworkResult | Statistics =>
        c.kind === 'horizontal'
          ? networkAdjust({ unit: c.unit!, sigma: c.sigma as never, grid: (c.grid ?? null) as never, known: c.known as never, approx: (c.approx ?? []) as never, rows: c.rows as never })
          : levelAdjust({ kind: c.levelKind!, sigma: c.sigma as never, known: c.known as never, rows: c.rows as never });
      if (c.error) {
        expect(run, c.name).toThrow(c.error);
        continue;
      }
      const got = run() as NetworkResult & { points: Record<string, number | string>[] };
      const want = c.expect!;
      expect([got.n, got.u, got.f, got.worst ?? null, got.passed ?? null], c.name).toEqual([want.n, want.u, want.f, want.worst, want.passed]);
      expect(near(got.omega, want.omega, 1e-9, tol.relative!), `${c.name}: Ω`).toBe(true);
      expect(near(got.m0, want.m0, 1e-12, tol.relative!), `${c.name}: m0`).toBe(true);
      expect(near(got.chi2, want.chi2, 0, 1e-9), `${c.name}: χ²`).toBe(true);
      expect(got.points.length, c.name).toBe(want.points.length);
      want.points.forEach((w, i) => {
        const g = got.points[i]!;
        expect(g.name, c.name).toBe(w.name);
        if (c.kind === 'horizontal') {
          for (const k of ['y', 'x']) expect(near(g[k] as number, w[k] as number, tol.coordinate!, 0), `${c.name} ${w.name} ${k}`).toBe(true);
          for (const k of ['sy', 'sx', 'sp', 'a', 'b']) expect(near(g[k] as number, w[k] as number, 1e-9, tol.relative!), `${c.name} ${w.name} ${k}`).toBe(true);
        } else {
          expect(near(g.h as number, w.h as number, tol.height!, 0), `${c.name} ${w.name} h`).toBe(true);
          expect(near(g.sh as number, w.sh as number, 1e-9, tol.relative!), `${c.name} ${w.name} σH`).toBe(true);
        }
      });
      want.observations.forEach((w, i) => {
        const g = got.observations[i]!;
        expect([g.kind, g.row, g.flag], `${c.name} ${i}`).toEqual([w.kind, w.row, w.flag]);
        expect(near(g.v, w.v, w.kind === 'direction' ? tol.angle! : 1e-7, 0), `${c.name} ${i} v`).toBe(true);
        expect(near(g.r, w.r, 1e-7, 0), `${c.name} ${i} r`).toBe(true);
        expect(near(g.w, w.w ?? null, tol.w!, 0), `${c.name} ${i} w`).toBe(true);
      });
    }
  });

  it('gives the χ² quantiles the reference gives', () => {
    for (const q of file.chi2) expect(near(chi2Quantile(q.f, 0.95), q.chi2, 0, 1e-9), `f ${q.f}`).toBe(true);
  });

  it('turns a field book into the networks’ rows', () => {
    const station = {
      station: 'S1',
      instrumentHeight: 1.5,
      observations: [
        { target: 'A', hz: 0, zenith: 100, slope: 100, targetHeight: 1.5 },
        { target: 'B', hz: 100, zenith: 100, slope: 50 },
        { target: 'C', hz: 200 },
      ],
    };
    const rows = fieldNetwork([station as never], 'grad', 0.13, null, 'deg');
    expect(rows.map((r) => [r.station, r.target])).toEqual([['S1', 'A'], ['S1', 'B'], ['S1', 'C']]);
    expect(rows[1]!.direction).toBeCloseTo(90, 12);
    expect(rows[2]!.distance ?? null).toBeNull();
    expect(fieldLevels([station as never], 'grad', 0.13, null).length).toBe(2);
  });
});
