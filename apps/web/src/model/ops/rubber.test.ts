import { describe, expect, it } from 'vitest';
import { rubberSheet, type RubberLink } from './rubber';
import type { Vec2 } from '../geometry';

/**
 * Kauçuk levha's solution (docs/adr/0158 §2) through the WASM core, against the independent reference in
 * fixtures/fit/v1/rubber.json (scripts/fixtures/rubber_cases.py, mpmath at 50 digits, no KentOS code), the cases the
 * core runs natively in crates/shared/geometry-core/src/ops/rubber.rs.
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface File {
  format: string;
  tolerance: { metres: number; jacobian: number };
  cases: { name: string; links: RubberLink[]; probes: Vec2[]; expected: { error?: string; map?: Vec2[]; jacobian?: number[][] } }[];
}

describe('Kauçuk levha: çözüm', () => {
  const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/fit/v1/rubber.json', import.meta.url), 'utf8')) as File;

  it('solves every case as the reference does', () => {
    expect(file.format).toBe('kentos.fit-rubber');
    expect(file.cases.length).toBeGreaterThanOrEqual(9);
    const off = file.cases.flatMap((c) => {
      const got = rubberSheet(c.links, c.probes);
      if ('error' in got) return got.error === c.expected.error ? [] : [`${c.name}: ${got.error} ≠ ${c.expected.error}`];
      if (c.expected.error) return [`${c.name}: a sheet ≠ ${c.expected.error}`];
      return c.probes.flatMap((_, i) => {
        const [g, w] = [got.map[i], c.expected.map![i]];
        const out: string[] = [];
        if (!(Math.hypot(g.x - w.x, g.y - w.y) <= file.tolerance.metres)) out.push(`${c.name}: probe ${i}: ${g.x}, ${g.y} ≠ ${w.x}, ${w.y}`);
        if (got.jacobian[i].some((v, k) => !(Math.abs(v - c.expected.jacobian![i][k]) <= file.tolerance.jacobian))) out.push(`${c.name}: probe ${i}: J`);
        return out;
      });
    });
    expect(off).toEqual([]);
  });
});
