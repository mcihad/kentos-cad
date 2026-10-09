import { describe, expect, it } from 'vitest';
import type { Entity } from '../entities';
import type { Vec2 } from '../geometry';
import { contourElevations } from './contourElevations';

/**
 * Eğrilere kot ver (docs/adr/0234 §9) through WASM against the shared cases (fixtures/raster-vector/v1/cases.json's
 * `elevations`, written by scripts/fixtures/raster_vector_cases.py with exact rationals, not KentOS code); the core
 * runs them natively (crates/shared/geometry-core/src/ops/contour_elevations.rs).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface File {
  format: string;
  elevations: { name: string; curves: Entity[]; start: Vec2; end: Vec2; first: number; step: number; expect: (number | null)[] }[];
}

const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/raster-vector/v1/cases.json', import.meta.url), 'utf8')) as File;

describe('Eğrilere kot ver (docs/adr/0234 §9)', () => {
  it('orders the curves as the shared cases say', () => {
    expect(file.format).toBe('kentos.raster-vector-cases');
    expect(file.elevations.length).toBeGreaterThanOrEqual(2);
    for (const c of file.elevations) expect(contourElevations(c.curves, c.start, c.end, c.first, c.step), c.name).toEqual(c.expect);
  });
});
