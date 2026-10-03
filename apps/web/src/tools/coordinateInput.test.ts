import { describe, expect, it } from 'vitest';
import cases from '../../../../fixtures/point-input/v1/cases.json?raw';
import type { Vec2 } from '../model/geometry';
import { UNIT_PER_METRE, type DrawingUnit } from '../model/projectSettings';
import { looksLikeCoordinate, parseNumber, parsePointInput, pointName } from './coordinateInput';

describe('parsePointInput', () => {
  const last = { x: 100, y: 200 };
  it('parses absolute Y,X', () => {
    expect(parsePointInput('486512.34,4420118.9', null, null)).toEqual({ x: 486512.34, y: 4420118.9 });
    expect(parsePointInput('10;20', null, null)).toEqual({ x: 10, y: 20 });
  });
  it('parses relative @dY,dX', () => {
    expect(parsePointInput('@5,-3', last, null)).toEqual({ x: 105, y: 197 });
    expect(parsePointInput('@5,-3', null, null)).toBeNull();
  });
  it('parses polar @distance<angle', () => {
    const p = parsePointInput('@10<90', last, null)!;
    expect(p.x).toBeCloseTo(100);
    expect(p.y).toBeCloseTo(210);
  });
  it('uses a bare number as distance along the cursor', () => {
    const p = parsePointInput('5', last, { x: 110, y: 200 })!;
    expect(p).toEqual({ x: 105, y: 200 });
  });
  it('rejects garbage', () => {
    expect(parsePointInput('abc', last, last)).toBeNull();
  });
});

describe('parseNumber', () => {
  it('accepts a comma decimal for single numbers', () => {
    expect(parseNumber('2,5')).toBe(2.5);
    expect(parseNumber('x')).toBeNull();
  });
});

/** One shared grammar case (fixtures/point-input/v1/cases.json). */
interface Case {
  name: string;
  fn: 'point' | 'number' | 'looksLikeCoordinate' | 'pointName';
  text: string;
  last?: Vec2 | null;
  cursor?: Vec2 | null;
  expect: Vec2 | number | boolean | string | null;
  tolerance?: number;
  /** A local project's drawing unit the text is typed in; the point is in metres. */
  unit?: DrawingUnit;
  /** The project's type and angle unit a polar angle is typed in (docs/adr/0165 §4); CAD's and degrees by default. */
  convention?: 'cad' | 'gis';
  angleUnit?: 'deg' | 'grad';
}

/**
 * The shared grammar cases: the desktop's reader
 * (crates/shared/geometry-core/src/tools/point_text.rs) runs the same file,
 * so the two readers cannot drift (docs/adr/0021).
 */
describe('shared point input cases (fixtures/point-input/v1)', () => {
  const file = JSON.parse(cases) as { format: string; version: number; cases: Case[] };
  it('is the v1 case file', () => {
    expect(file.format).toBe('kentos.point-input-cases');
    expect(file.version).toBe(1);
    expect(file.cases.length).toBeGreaterThan(40);
  });
  for (const c of file.cases) {
    it(`${c.fn}: ${c.name} (${JSON.stringify(c.text)})`, () => {
      if (c.fn === 'point') {
        const perMetre = UNIT_PER_METRE[c.unit ?? 'm'];
        // The conversion the formatter's `toMetres` makes (app/format.ts).
        const angles = { fromNorth: c.convention === 'gis', grads: c.angleUnit === 'grad' };
        const got = parsePointInput(c.text, c.last ?? null, c.cursor ?? null, undefined, (v) => v / perMetre, angles);
        const want = c.expect as Vec2 | null;
        if (!want) return void expect(got).toBeNull();
        const tol = c.tolerance ?? 0;
        // Exact unless the case gives a tolerance; 0 and −0 are the same coordinate.
        const near = !!got && Math.abs(got.x - want.x) <= tol && Math.abs(got.y - want.y) <= tol;
        expect(near, `${JSON.stringify(got)}, beklenen ${JSON.stringify(want)}`).toBe(true);
      } else if (c.fn === 'number') {
        expect(parseNumber(c.text)).toBe(c.expect);
      } else if (c.fn === 'pointName') {
        expect(pointName(c.text)).toBe(c.expect);
      } else {
        expect(looksLikeCoordinate(c.text)).toBe(c.expect);
      }
    });
  }
});
