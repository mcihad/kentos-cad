import { describe, expect, it } from 'vitest';
import { surveyPolar, surveyStakeout, surveyTraverse, type Grid, type TraverseInput } from '../model/geom/surveyCalc';
import type { Vec2 } from '../model/geometry';

/**
 * The survey windows with the project's grid (docs/adr/0171 §4) against fixtures/geodesy/v1/ground-survey.json (written by
 * scripts/fixtures/ground_survey_cases.py from known points on the grid and PROJ's and GeographicLib's factors, not KentOS
 * code), through the WASM core: Kutupsal alım places the points where they are, Aplikasyon gives their ground distances,
 * Poligon hesabı gives its points back with no misclosure; lengths within 1e-6 m, factors within 1e-10. The core runs
 * the same file natively (crates/shared/geometry-core/tests/all/crs_ground.rs).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

type Factors = { scale: number; heightFactor: number };
interface File {
  format: string;
  polar: {
    name: string;
    input: { unit: 'grad' | 'deg'; station: Vec2; back: Vec2; backReading: number; shots: { reading: number; distance: number; zenith?: number }[]; grid: Grid };
    expect: ({ p: [number, number]; grid: number } & Factors)[];
  }[];
  stakeout: { name: string; input: { unit: 'grad' | 'deg'; station: Vec2; back: Vec2; targets: Vec2[]; grid: Grid }; expect: ({ distance: number; ground: number } & Factors)[] }[];
  traverse: { name: string; input: TraverseInput; expect: { points: Vec2[]; legs: ({ distance: number } & Factors)[] } }[];
}

const file = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/geodesy/v1/ground-survey.json', import.meta.url), 'utf8')) as File;
const off = (what: string, got: number | undefined, want: number, tolerance: number) =>
  got !== undefined && Math.abs(got - want) <= tolerance ? [] : [`${what}: ${got} ≠ ${want}`];

describe('survey windows on the grid (docs/adr/0171 §4)', () => {
  it('reads the reference', () => expect(file.format).toBe('kentos.ground-survey'));

  it('Kutupsal alım places the points where they are', () => {
    const wrong = file.polar.flatMap((c) =>
      surveyPolar({
        ...c.input,
        stationZ: null,
        instrumentHeight: null,
        shots: c.input.shots.map((s) => ({ reading: s.reading, distance: s.distance, zenith: s.zenith ?? null, targetHeight: null })),
      }).flatMap(
        (p, i) => {
          const w = c.expect[i]!;
          const what = `${c.name} ${i + 1}`;
          return [
            ...off(`${what} Y`, p.p.x, w.p[0], 1e-6),
            ...off(`${what} X`, p.p.y, w.p[1], 1e-6),
            ...off(`${what} grid`, p.grid, w.grid, 1e-6),
            ...off(`${what} scale`, p.scale, w.scale, 1e-10),
            ...off(`${what} height factor`, p.heightFactor, w.heightFactor, 1e-10),
          ];
        },
      ),
    );
    expect(wrong).toEqual([]);
  });

  it('Aplikasyon gives the ground distances', () => {
    const wrong = file.stakeout.flatMap((c) =>
      surveyStakeout(c.input).flatMap((s, i) => {
        const w = c.expect[i]!;
        const what = `${c.name} ${i + 1}`;
        return [
          ...off(`${what} distance`, s.distance, w.distance, 1e-6),
          ...off(`${what} ground`, s.ground, w.ground, 1e-6),
          ...off(`${what} scale`, s.scale, w.scale, 1e-10),
          ...off(`${what} height factor`, s.heightFactor, w.heightFactor, 1e-10),
        ];
      }),
    );
    expect(wrong).toEqual([]);
  });

  it('Poligon hesabı gives its points back with no misclosure', () => {
    const wrong = file.traverse.flatMap((c) => {
      const r = surveyTraverse(c.input);
      return [
        ...r.points.flatMap((p, i) => [...off(`${c.name} P${i + 1} Y`, p.x, c.expect.points[i]!.x, 1e-6), ...off(`${c.name} P${i + 1} X`, p.y, c.expect.points[i]!.y, 1e-6)]),
        ...((r.linearMisclosure ?? 1) <= 1e-6 ? [] : [`${c.name}: misclosure ${r.linearMisclosure}`]),
        ...r.legs.flatMap((l, i) => [
          ...off(`${c.name} kenar ${i + 1}`, l.distance, c.expect.legs[i]!.distance, 1e-6),
          ...off(`${c.name} kenar ${i + 1} scale`, l.scale, c.expect.legs[i]!.scale, 1e-10),
          ...off(`${c.name} kenar ${i + 1} height factor`, l.heightFactor, c.expect.legs[i]!.heightFactor, 1e-10),
        ]),
      ];
    });
    expect(wrong).toEqual([]);
  });
});
