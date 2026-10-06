import { describe, expect, it } from 'vitest';
import text from '../../../../fixtures/point-calc/v1/cases.json?raw';
import type { Entity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { bisectorNearest, bisectorPoint, kmText, kmValue, pathReading, pathStation, slopeHorizontal } from './constructions';

/**
 * Nokta hesaplayıcı ekleri (docs/adr/0188) through the WASM core, on the independent reference's cases
 * (fixtures/point-calc/v1/cases.json, scripts/fixtures/point_calc_cases.py): a path's point at a distance and an offset,
 * where a point stands against a path, the km read and written, a slope distance's horizontal and the bisector. The core
 * runs the same file natively (crates/shared/geometry-core/tests/all/point_calc.rs). A curve's cases hold to 0.1 mm.
 */
const f = JSON.parse(text);
const xy = (v: number[]): Vec2 => ({ x: v[0], y: v[1] });

function near(got: number, want: number, tol: number, what: string): void {
  expect(Math.abs(got - want), `${what}: ${got} ≠ ${want}`).toBeLessThanOrEqual(tol);
}

function pointNear(got: Vec2 | null | undefined, want: number[] | null, tol: number, what: string): void {
  if (want === null) return void expect(got ?? null, what).toBeNull();
  expect(got, what).toBeTruthy();
  near(got!.x, want[0], tol, `${what} x`);
  near(got!.y, want[1], tol, `${what} y`);
}

describe('point calculator extras (fixtures/point-calc/v1)', () => {
  it('puts a point at a distance along a path and an offset square to it', () => {
    for (const c of f.stations) {
      const got = pathStation(c.shape as Entity, c.fromEnd, c.s, c.offset);
      pointNear(got.point, c.expect.point, c.tolerance, c.name);
      near(got.length, c.expect.length, c.tolerance, `${c.name}: length`);
    }
  });

  it('reads where a point stands against a path', () => {
    for (const c of f.readings) {
      const got = pathReading(c.shape as Entity, c.fromEnd, xy(c.at))!;
      for (const key of ['s', 'offset', 'length'] as const) near(got[key], c.expect[key], c.tolerance, `${c.name}: ${key}`);
    }
  });

  it('reads and writes the km', () => {
    for (const c of f.km) {
      const got = kmValue(c.text);
      if (c.value === null) expect(got, c.text).toBeNull();
      else near(got!, c.value, 1e-9, c.text);
    }
    for (const c of f.kmText) expect(kmText(c.value, c.decimals), String(c.value)).toBe(c.text);
  });

  it('takes a slope distance’s horizontal and the bisector', () => {
    for (const c of f.slopes) {
      const got = slopeHorizontal(c.s, c.percent);
      near(got.horizontal, c.expect.horizontal, 1e-9, 'horizontal');
      near(got.rise, c.expect.rise, 1e-9, 'rise');
    }
    for (const c of f.bisectors) pointNear(bisectorPoint(xy(c.k), xy(c.a), xy(c.b), c.d), c.expect, 1e-7, c.name);
    for (const c of f.bisectorClicks) pointNear(bisectorNearest(xy(c.k), xy(c.a), xy(c.b), xy(c.at)), c.expect, 1e-7, c.name);
  });
});
