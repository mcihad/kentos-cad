import { describe, expect, it } from 'vitest';
import { parseNumber, parsePointInput } from './coordinateInput';

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
