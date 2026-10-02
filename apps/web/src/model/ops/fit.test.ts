import { describe, expect, it } from 'vitest';
import { fitScaleTurn, fitTransform, type FitKind, type FitPair } from './fit';

/**
 * Vektör oturtma's solution (docs/adr/0156 §2–§3) through the WASM core, against the independent reference in
 * fixtures/fit/v1/solve.json (scripts/fixtures/fit_cases.py, exact fractions, no KentOS code), the cases the core runs
 * natively in crates/shared/geometry-core/src/ops/fit.rs: centres, residuals and m0 within the file's metres, the
 * numbers and derived values within its relative tolerance (of their size, at least 1).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface File {
  format: string;
  version: number;
  tolerance: { metres: number; relative: number };
  cases: { name: string; kind: FitKind; pairs: FitPair[]; expected: unknown }[];
}

const METRES = ['from', 'to', 'residuals', 'm0'];

function differ(a: unknown, e: unknown, path: string, tol: File['tolerance']): string | null {
  if (typeof a === 'number' && typeof e === 'number') {
    const limit = METRES.some((k) => path.startsWith(`.${k}`)) ? tol.metres : tol.relative * Math.max(Math.abs(e), 1);
    return Math.abs(a - e) <= limit ? null : `${path}: ${a} ≠ ${e}`;
  }
  if (Array.isArray(a) || Array.isArray(e)) {
    if (!Array.isArray(a) || !Array.isArray(e) || a.length !== e.length) return `${path}: ${JSON.stringify(a)?.slice(0, 80)} ≠ ${JSON.stringify(e)?.slice(0, 80)}`;
    for (let i = 0; i < a.length; i++) {
      const d = differ(a[i], e[i], `${path}[${i}]`, tol);
      if (d) return d;
    }
    return null;
  }
  if (a && e && typeof a === 'object' && typeof e === 'object') {
    for (const k of new Set([...Object.keys(a), ...Object.keys(e)])) {
      const d = differ((a as Record<string, unknown>)[k] ?? null, (e as Record<string, unknown>)[k] ?? null, `${path}.${k}`, tol);
      if (d) return d;
    }
    return null;
  }
  return (a ?? null) === (e ?? null) ? null : `${path}: ${String(a)} ≠ ${String(e)}`;
}

describe('Vektör oturtma: çözüm', () => {
  const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/fit/v1/solve.json', import.meta.url), 'utf8')) as File;

  it('is the reference’s file', () => {
    expect([file.format, file.version]).toEqual(['kentos.fit-solve', 1]);
    expect(file.cases.length).toBeGreaterThanOrEqual(15);
  });

  it('solves every case as the reference does', () => {
    const off = file.cases.flatMap((c) => {
      const d = differ(fitTransform(c.pairs, c.kind), c.expected, '', file.tolerance);
      return d ? [`${c.name}: ${d}`] : [];
    });
    expect(off).toEqual([]);
  });
});

describe('Vektör oturtma: Parametrelerle', () => {
  const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/fit/v1/parameters.json', import.meta.url), 'utf8')) as {
    format: string;
    tolerance: number;
    cases: { name: string; east: number; north: number; rotation: number; m: number[] }[];
  };

  it('turns the scales and the rotation into the reference’s linear part', () => {
    expect(file.format).toBe('kentos.fit-parameters');
    expect(file.cases.length).toBeGreaterThanOrEqual(12);
    const off = file.cases.flatMap((c) => {
      const m = fitScaleTurn(c.east, c.north, c.rotation);
      const size = Math.max(1, Math.abs(c.east), Math.abs(c.north));
      const bad = m.some((v, i) => Math.abs(v - c.m[i]) > file.tolerance * size);
      const similar = c.east !== c.north || (m[0] === m[3] && m[1] === -m[2]);
      return bad || !similar ? [`${c.name}: ${m.join(', ')} ≠ ${c.m.join(', ')}`] : [];
    });
    expect(off).toEqual([]);
  });
});
