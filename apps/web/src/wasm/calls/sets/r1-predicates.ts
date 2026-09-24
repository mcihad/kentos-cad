import type { Vec2 } from '../../../model/geometry';
import { repeat, type CallSet } from '../harness';

/**
 * R1: the exact orientation predicate (CLAUDE.md §23.3, docs/adr/0008):
 * which side of a→b the point c lies on, decided on the exact float64
 * values. The frozen answers come from the core; the independent exact
 * signs are in fixtures/geometry/v1/reference-calls.json.
 */

const v = (x: number, y: number): Vec2 => ({ x, y });
/** The ulp of 0.5. */
const U = 2 ** -53;
const E = 486512.5;
const N = 4420187.25;

/** The next float64 above or below x (x itself for 0). */
function step(x: number, up: boolean): number {
  if (x === 0) return x;
  const f = new Float64Array([x]);
  const bits = new BigInt64Array(f.buffer);
  bits[0] += x > 0 === up ? 1n : -1n;
  return f[0];
}

/** Kettner et al.'s classroom grid: a few ulps around (0.5, 0.5) against the line through (12, 12) and (24, 24). */
const grid = (i: number, j: number) => [v(0.5 + i * U, 0.5 + j * U), v(12, 12), v(24, 24)];

/** A point exactly on the TM line from (E, N) to (E + 80.5, N + 60.25). */
const ON = v(E + 40.25, N + 30.125);
const LINE = [v(E, N), v(E + 80.5, N + 60.25)];

export const R1_PREDICATES: CallSet = {
  file: 'calls-r1-predicates.json',
  named: [
    { name: 'sol dönüş', fn: 'orientation', args: [v(0, 0), v(1, 0), v(0, 1)] },
    { name: 'sağ dönüş', fn: 'orientation', args: [v(0, 0), v(1, 0), v(0, -1)] },
    { name: 'aynı doğru', fn: 'orientation', args: [v(0, 0), v(1, 1), v(2, 2)] },
    { name: 'çakışık iki nokta', fn: 'orientation', args: [v(3, 4), v(3, 4), v(7, -1)] },
    { name: 'üç nokta aynı', fn: 'orientation', args: [v(1, 1), v(1, 1), v(1, 1)] },
    { name: 'sınıf ızgarası (0, 0)', fn: 'orientation', args: grid(0, 0) },
    { name: 'sınıf ızgarası (5, 3)', fn: 'orientation', args: grid(5, 3) },
    { name: 'sınıf ızgarası (3, 5)', fn: 'orientation', args: grid(3, 5) },
    { name: 'sınıf ızgarası (13, 12)', fn: 'orientation', args: grid(13, 12) },
    { name: 'sınıf ızgarası (47, 50)', fn: 'orientation', args: grid(47, 50) },
    { name: 'TM doğrusu üzerinde', fn: 'orientation', args: [...LINE, ON] },
    { name: 'TM doğrusundan bir ulp yukarı', fn: 'orientation', args: [...LINE, v(ON.x, step(ON.y, true))] },
    { name: 'TM doğrusundan bir ulp aşağı', fn: 'orientation', args: [...LINE, v(ON.x, step(ON.y, false))] },
    { name: 'TM doğrusundan bir ulp doğuya', fn: 'orientation', args: [...LINE, v(step(ON.x, true), ON.y)] },
    { name: 'TM doğrusundan bir ulp batıya', fn: 'orientation', args: [...LINE, v(step(ON.x, false), ON.y)] },
    { name: 'NaN karar değil', fn: 'orientation', args: [v(NaN, 0), v(1, 0), v(0, 1)] },
  ],
  random: (g, n) =>
    repeat(g, 'orientation', n, () => {
      const a = g.pt(1000);
      const b = g.chance(0.05) ? a : g.pt(1000);
      if (g.chance(0.3)) return [a, b, g.pt(1000)];
      // Near a→b: on the line as far as rounding allows, then a few ulps off.
      const t = g.num(-2, 3);
      let c = v(a.x + t * (b.x - a.x), a.y + t * (b.y - a.y));
      for (let k = g.int(0, 3); k > 0; k--) c = g.chance(0.5) ? v(step(c.x, g.chance(0.5)), c.y) : v(c.x, step(c.y, g.chance(0.5)));
      return [a, b, c];
    }),
};
