import { describe, expect, it } from 'vitest';
import { topologyClean, type TopoObject, type TopoResult, type TopoWorks } from './topology';

/**
 * Topolojik temizlik (docs/adr/0148) through the WASM core, against the independent reference in
 * fixtures/topology/v1/clean.json (scripts/fixtures/topology_cases.py, no KentOS code), the cases the core runs
 * natively in crates/shared/geometry-core/tests/topology.rs: numbers within 1e-9 m, everything else exactly.
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface Case {
  name: string;
  objects: TopoObject[];
  tolerance: number;
  works: TopoWorks;
  expected: TopoResult;
}

function differ(a: unknown, e: unknown, path: string): string | null {
  if (typeof a === 'number' && typeof e === 'number') return Math.abs(a - e) <= 1e-9 ? null : `${path}: ${a} ≠ ${e}`;
  if (Array.isArray(a) || Array.isArray(e)) {
    if (!Array.isArray(a) || !Array.isArray(e) || a.length !== e.length) return `${path}: ${JSON.stringify(a)?.slice(0, 80)} ≠ ${JSON.stringify(e)?.slice(0, 80)}`;
    for (let i = 0; i < a.length; i++) {
      const d = differ(a[i], e[i], `${path}[${i}]`);
      if (d) return d;
    }
    return null;
  }
  if (a && e && typeof a === 'object' && typeof e === 'object') {
    const keys = new Set([...Object.keys(a), ...Object.keys(e)]);
    for (const k of keys) {
      const d = differ((a as Record<string, unknown>)[k] ?? null, (e as Record<string, unknown>)[k] ?? null, `${path}.${k}`);
      if (d) return d;
    }
    return null;
  }
  return a === e ? null : `${path}: ${String(a)} ≠ ${String(e)}`;
}

describe('Topolojik temizlik', () => {
  it('cleans every case as the reference cleans it', () => {
    const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/topology/v1/clean.json', import.meta.url), 'utf8')) as { format: string; version: number; cases: Case[] };
    expect([file.format, file.version]).toEqual(['kentos.topology-fixtures', 1]);
    expect(file.cases.length).toBeGreaterThanOrEqual(50);
    const off = file.cases.flatMap((c) => {
      const d = differ(topologyClean(c.objects, c.tolerance, c.works), c.expected, c.name);
      return d ? [d] : [];
    });
    expect(off).toEqual([]);
  });

  it('refuses a tolerance below a micrometre', () => {
    expect(() => topologyClean([], 1e-7, { ends: true, vertices: false, extend: true, trim: true })).toThrow(/0\.000001/);
  });
});
