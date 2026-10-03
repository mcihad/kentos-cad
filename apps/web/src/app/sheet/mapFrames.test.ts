import { describe, expect, it } from 'vitest';
import { unpapered } from './mapFrames';

/**
 * The PDF's fallback map picture without paper (as the desktop's: straight
 * alpha): the same tile over white and over black gives what covers the paper.
 */
describe('a map picture without paper', () => {
  /** A pixel of colour `c` and coverage `a` (0–255) over a paper of grey `p`, as the canvas keeps it. */
  const over = (c: readonly number[], a: number, p: number) => c.map((x) => Math.round((x * a) / 255 + p * (1 - a / 255)));

  it('recovers each pixel’s colour and coverage from the two papers', () => {
    const pixels: [number[], number][] = [
      [[200, 30, 30], 255], // ink
      [[0, 0, 0], 0], // nothing: the paper
      [[0, 0, 255], 128], // a half-covered edge
      [[40, 160, 90], 64], // a light fill
    ];
    const white = Uint8ClampedArray.from(pixels.flatMap(([c, a]) => [...over(c, a, 255), 255]));
    const black = Uint8ClampedArray.from(pixels.flatMap(([c, a]) => [...over(c, a, 0), 255]));
    const out = [...unpapered(white, black)];
    const got = pixels.map((_, i) => out.slice(i * 4, i * 4 + 4));
    expect(got[0]).toEqual([200, 30, 30, 255]);
    expect(got[1][3]).toBe(0);
    // Edges keep their colour to a step or two of 8 bits (the picture's two roundings).
    for (const i of [2, 3]) {
      const [c, a] = pixels[i];
      expect(Math.abs(got[i][3] - a)).toBeLessThanOrEqual(1);
      for (let k = 0; k < 3; k++) expect(Math.abs(got[i][k] - c[k])).toBeLessThanOrEqual(4);
    }
  });
});
