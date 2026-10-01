import { describe, expect, it } from 'vitest';
import { fixed } from '../core/displayNumber';
import { op } from './core';

/**
 * The web's display rule and the core's (docs/adr/0149) write the same text: `core/displayNumber.ts` against
 * `kentos_geometry_core::display::fixed` through the WASM core, on random values of every size and on halves at the
 * digits shown carrying the noise a measure carries. The fixed cases of both are fixtures/numeric/v1/display.json.
 */
const coreFixed = op<(v: number, d: number) => string>('displayFixed');

describe('display rule, web and core', () => {
  it('write the same text for 100 000 values', () => {
    let seed = 20261001;
    const random = () => ((seed = (seed * 1103515245 + 12345) >>> 0) / 2 ** 32);
    const noise = [0, 3e-10, -3e-10, 1e-9, -1e-9, 4.9e-8, -4.9e-8, 5.1e-8, -5.1e-8, 1e-6];
    const differ: string[] = [];
    for (let i = 0; i < 100_000; i++) {
      const d = Math.floor(random() * 13);
      let v: number;
      if (i % 2 === 0) v = (random() < 0.5 ? -1 : 1) * 10 ** (random() * 13 - 6);
      else {
        const shown = Math.min(d, 6);
        const k = Math.floor(random() * 5_000_000);
        v = (k + 0.5) / 10 ** shown + noise[Math.floor(random() * noise.length)];
      }
      const web = fixed(v, d);
      const core = coreFixed(v, d);
      if (web !== core) differ.push(`${v} with ${d}: web ${web}, core ${core}`);
    }
    expect(differ.slice(0, 10)).toEqual([]);
  });
});
