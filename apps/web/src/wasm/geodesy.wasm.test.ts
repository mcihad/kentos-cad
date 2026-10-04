import { describe, expect, it } from 'vitest';
import { tmForward, type Tm } from '../model/geom/geodesy';

/**
 * The forward transverse Mercator projection against PROJ (fixtures/geodesy/v1/tm-forward.json, written by
 * scripts/fixtures/tm_cases.py from PROJ's tmerc, not KentOS code; docs/adr/0165 §3), through the WASM core the app
 * calls: every case within a micrometre. The core runs the same file natively (crates/shared/geometry-core/tests/all/geodesy.rs).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface Case {
  readonly grid: string;
  readonly tm: Tm;
  readonly lat: number;
  readonly lon: number;
  readonly east: number;
  readonly north: number;
}

describe('tmForward', () => {
  it('puts every case where PROJ puts it, and has no point where the projection cannot reach', () => {
    const file = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/geodesy/v1/tm-forward.json', import.meta.url), 'utf8')) as { format: string; cases: Case[] };
    expect(file.format).toBe('kentos.tm-forward-cases');
    const off = file.cases.flatMap((c) => {
      const p = tmForward(c.tm, c.lat, c.lon);
      const far = p ? Math.max(Math.abs(p.x - c.east), Math.abs(p.y - c.north)) : Infinity;
      return far <= 1e-6 ? [] : [`${c.grid} ${c.lat},${c.lon}: ${far} m`];
    });
    expect(off).toEqual([]);
    expect(file.cases.length).toBeGreaterThanOrEqual(80);
    expect(tmForward(file.cases[0].tm, 0, file.cases[0].tm.centralMeridian + 90)).toBeNull();
  });
});
