import { describe, expect, it } from 'vitest';
import type { Elevated } from './elevation';
import { vertexTableInsert, vertexTableMove, vertexTableRadius, vertexTableRemove, vertexTableRows, vertexTableZ, type VertexKind } from './vertexTable';

/**
 * Köşe tablosu's rows and writes (docs/adr/0172 §7) through the WASM core, against the independent reference in
 * fixtures/vertex-table/v1/cases.json (scripts/fixtures/vertex_table_cases.py, no KentOS code), the cases the core runs
 * natively in crates/shared/geometry-core/tests/all/vertex_table.rs: coordinates and elevations bit for bit; chords,
 * radii, bulges and the least radius within 1e-12 (relative); a missing field null.
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

/** Whether the value at `path` is worked out rather than copied. */
const computed = (path: string) => path.includes('.bulges') || /\.(chord|radius|least)$/.test(path);

function differ(a: unknown, e: unknown, path: string): string | null {
  if (typeof a === 'number' && typeof e === 'number') {
    const close = computed(path) ? Math.abs(a - e) <= 1e-12 * Math.max(Math.abs(e), 1) : a === e;
    return close ? null : `${path}: ${a} ≠ ${e}`;
  }
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

interface Shape {
  kind: VertexKind;
  paths: Elevated[];
}
type Op = Record<string, unknown> & { kind: 'move' | 'z' | 'radius' | 'insert' | 'remove' };
const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/vertex-table/v1/cases.json', import.meta.url), 'utf8')) as {
  format: string;
  version: number;
  shapes: Record<string, Shape>;
  rows: { shape: string; expect: unknown }[];
  cases: { name: string; shape: string; op: Op; expect: unknown }[];
};

function write({ kind, paths }: Shape, op: Op): unknown {
  const at = (k: string) => op[k] as number;
  switch (op.kind) {
    case 'move':
      return vertexTableMove(kind, paths, at('path'), at('index'), op.to as never);
    case 'z':
      return vertexTableZ(kind, paths, at('path'), at('index'), op.z as number | null);
    case 'radius':
      return vertexTableRadius(kind, paths, at('path'), at('index'), op.radius as number | null, at('slack'));
    case 'insert':
      return vertexTableInsert(kind, paths, at('path'), at('after'), op.at as never, op.z as number | null);
    case 'remove':
      return vertexTableRemove(kind, paths, op.at as [number, number][]);
  }
}

describe('Köşe tablosu', () => {
  it('is the reference’s file', () => expect([file.format, file.version]).toEqual(['kentos.vertex-table', 1]));

  it('shows every vertex with its edge’s chord and radius', () => {
    expect(file.rows.length).toBe(5);
    expect(file.rows.flatMap((r) => differ(vertexTableRows(file.shapes[r.shape].paths), r.expect, r.shape) ?? [])).toEqual([]);
  });

  it('writes a move, an elevation, a radius, a vertex added and vertices removed, or refuses them', () => {
    expect(file.cases.length).toBeGreaterThanOrEqual(45);
    expect(file.cases.flatMap((c) => differ(write(file.shapes[c.shape], c.op), c.expect, c.name) ?? [])).toEqual([]);
  });
});
