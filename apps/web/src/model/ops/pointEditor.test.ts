import { describe, expect, it } from 'vitest';
import { duplicatePoints, followPoint, naturalOrder, pointTable } from './pointEditor';

/**
 * Nokta editörü's computations (docs/adr/0153 §6) through the WASM core, against the independent reference in
 * fixtures/point-editor/v1/cases.json (scripts/fixtures/point_editor_cases.py, no KentOS code), the cases the core runs
 * natively in crates/shared/geometry-core/tests/point_editor.rs: numbers within 1e-9 m, a missing field null.
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

type Case = Record<string, unknown> & { name: string; expected: unknown };

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
    for (const k of new Set([...Object.keys(a), ...Object.keys(e)])) {
      const d = differ((a as Record<string, unknown>)[k] ?? null, (e as Record<string, unknown>)[k] ?? null, `${path}.${k}`);
      if (d) return d;
    }
    return null;
  }
  return (a ?? null) === (e ?? null) ? null : `${path}: ${String(a)} ≠ ${String(e)}`;
}

const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/point-editor/v1/cases.json', import.meta.url), 'utf8')) as Record<string, Case[]> & { format: string; version: number };

/** Every case of `section` through `run`, against its expected answer. */
function offs(section: string, least: number, run: (c: Case) => unknown): string[] {
  const cases = file[section];
  expect(cases.length).toBeGreaterThanOrEqual(least);
  return cases.flatMap((c) => {
    const d = differ(run(c), c.expected, c.name);
    return d ? [d] : [];
  });
}

describe('Nokta editörü', () => {
  it('is the reference’s file', () => expect([file.format, file.version]).toEqual(['kentos.point-editor-fixtures', 1]));
  it('sorts names in the natural order', () => expect(offs('natural', 15, (c) => naturalOrder(c.names as string[]))).toEqual([]));
  it('shows and sorts the table’s rows', () => expect(offs('table', 100, (c) => pointTable(c.rows as never, c.query as never))).toEqual([]));
  it('groups duplicates and keeps one of each', () => expect(offs('duplicates', 30, (c) => duplicatePoints(c.points as never, c.by as never, c.tolerance as number, c.keep as never))).toEqual([]));
  it('moves the vertices that follow a point', () => expect(offs('follow', 25, (c) => followPoint(c.paths as never, c.from as never, c.to as never, c.setZ as boolean, c.z as number | null))).toEqual([]));
});
