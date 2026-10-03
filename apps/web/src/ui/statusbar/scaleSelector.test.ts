import { describe, expect, it } from 'vitest';
import { offeredScales, typedScale } from './scaleSelector';

describe('the status bar’s scale selector', () => {
  it('reads a typed scale as the desktop does', () => {
    expect(['1:500', '2.500', ' 1:25.000 ', '0', 'abc', '1:'].map(typedScale)).toEqual([500, 2500, 25_000, null, null, null]);
  });
  it('offers the project type’s scales', () => {
    expect(offeredScales('cad')[0]).toBe(1);
    expect(offeredScales('gis')).toContain(25_000);
  });
});
