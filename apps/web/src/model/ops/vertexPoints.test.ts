import { describe, expect, it } from 'vitest';
import type { Vec2 } from '../geometry';
import type { Elevated } from './elevation';
import { vertexPoints, type VertexPoints } from './vertexPoints';

/**
 * Köşelere nokta (docs/adr/0152 §5) through the WASM core, against the independent reference in
 * fixtures/vertex-points/v1/cases.json (scripts/fixtures/vertex_points_cases.py, no KentOS code), the cases the core
 * runs natively in crates/shared/geometry-core/tests/vertex_points.rs: numbers within 1e-9 m, a missing field null.
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface Case {
  name: string;
  objects: Elevated[][];
  existing: Vec2[];
  first: string | null;
  expected: VertexPoints;
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
    for (const k of new Set([...Object.keys(a), ...Object.keys(e)])) {
      const d = differ((a as Record<string, unknown>)[k] ?? null, (e as Record<string, unknown>)[k] ?? null, `${path}.${k}`);
      if (d) return d;
    }
    return null;
  }
  return (a ?? null) === (e ?? null) ? null : `${path}: ${String(a)} ≠ ${String(e)}`;
}

describe('Köşelere nokta', () => {
  it('places every case as the reference places it', () => {
    const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/vertex-points/v1/cases.json', import.meta.url), 'utf8')) as { format: string; version: number; cases: Case[] };
    expect([file.format, file.version]).toEqual(['kentos.vertex-points-fixtures', 1]);
    expect(file.cases.length).toBeGreaterThanOrEqual(40);
    const off = file.cases.flatMap((c) => {
      const d = differ(vertexPoints(c.objects, c.existing, c.first), c.expected, c.name);
      return d ? [d] : [];
    });
    expect(off).toEqual([]);
  });
});
