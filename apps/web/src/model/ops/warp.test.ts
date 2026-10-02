import { describe, expect, it } from 'vitest';
import { rubberShapes, warpShapes, type PathElevations, type Warp } from './warp';

/**
 * Vektör oturtma's warping of objects (docs/adr/0156 §4–§5) through the WASM core, against the independent reference
 * in fixtures/fit/v1/warp.json (scripts/fixtures/warp_cases.py, no KentOS code), the cases the core runs natively in
 * crates/shared/geometry-core/src/ops/warp.rs: coordinates within the file's metres, every other number within its
 * relative tolerance (of its size, at least 1), straight vertices one for one.
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface File {
  format: string;
  version: number;
  tolerance: { metres: number; relative: number };
  warps: Record<string, Warp>;
  cases: { name: string; warp: string; objects: unknown[]; zs: PathElevations[]; expected: unknown }[];
}

function differ(a: unknown, e: unknown, path: string, tol: File['tolerance']): string | null {
  if (typeof a === 'number' && typeof e === 'number') {
    // Coordinates and a sheet's bend are metres.
    const limit = path.endsWith('.x') || path.endsWith('.y') || path === '.bend' ? tol.metres : tol.relative * Math.max(Math.abs(e), 1);
    return Math.abs(a - e) <= limit ? null : `${path}: ${a} ≠ ${e}`;
  }
  if (Array.isArray(a) || Array.isArray(e)) {
    if (!Array.isArray(a) || !Array.isArray(e) || a.length !== e.length) return `${path}: ${Array.isArray(a) ? a.length : typeof a} öğe ≠ ${Array.isArray(e) ? e.length : typeof e}`;
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

describe('Vektör oturtma: nesnelerin dönüşmesi', () => {
  const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/fit/v1/warp.json', import.meta.url), 'utf8')) as File;

  it('is the reference’s file', () => {
    expect([file.format, file.version]).toEqual(['kentos.fit-warp', 1]);
    expect(file.cases.length).toBeGreaterThanOrEqual(6);
  });

  it('warps every case as the reference does', () => {
    const off = file.cases.flatMap((c) => {
      const d = differ(warpShapes(c.objects, c.zs, file.warps[c.warp]), c.expected, '', file.tolerance);
      return d ? [`${c.name}: ${d}`] : [];
    });
    expect(off).toEqual([]);
  });
});

describe('Kauçuk levha: nesneler', () => {
  const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/fit/v1/rubber-warp.json', import.meta.url), 'utf8')) as Omit<File, 'warps'> & {
    sheets: Record<string, { from: { x: number; y: number }; to: { x: number; y: number } }[]>;
  };

  it('puts every case on its sheet as the reference does', () => {
    expect(file.format).toBe('kentos.fit-rubber-warp');
    expect(file.cases.length).toBeGreaterThanOrEqual(3);
    const off = file.cases.flatMap((c) => {
      const d = differ(rubberShapes(c.objects, c.zs, file.sheets[(c as unknown as { sheet: string }).sheet]), c.expected, '', file.tolerance);
      return d ? [`${c.name}: ${d}`] : [];
    });
    expect(off).toEqual([]);
  });
});
