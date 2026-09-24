// The expression language on 100 000 objects (docs/adr/0008 “İfade dili”):
// the table the app builds, the core's evaluation in WASM, and every value
// read back, as a processing run or a layer's style asks. Node and the WASM
// package, one process; p50 and p95 of repeated runs after a warm-up. Runs
// only on purpose:
//   EXPRESSION_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/expression.test.ts --disable-console-intercept
import { it } from 'vitest';
import type { Entity } from '../../src/model/entities';
import { compileExpression, type ExprAs } from '../../src/model/expression/expression';
import { MEASURE_STRIDE } from '../../src/model/expression/expressionLib';

const N = 100_000;
const RUNS = Number(process.env.EXPRESSION_RUNS ?? 20);
const WARM = 3;

/** A condition, a numbering, an area as text, a rounded number. */
const CASES: [string, ExprAs][] = [
  ["Nitelik = 'Arsa' ve $alan > 500", 'bool'],
  ["'P' || doldur($sıra, 5)", 'text'],
  ["metin($alan, 2) || ' m²'", 'text'],
  ['yuvarla($alan, 2)', 'number'],
];

function stats(ms: number[]): string {
  const s = [...ms].sort((a, b) => a - b);
  const q = (p: number) => s[Math.min(s.length - 1, Math.floor(p * s.length))];
  return `p50 ${q(0.5).toFixed(1).padStart(6)} ms, p95 ${q(0.95).toFixed(1).padStart(6)} ms (${s.length} koşu)`;
}

it.runIf(!!process.env.EXPRESSION_BENCH)('measures expressions on 100 000 objects', () => {
  const entities: Entity[] = Array.from({ length: N }, (_, i) => ({ id: i + 1, kind: 'point', layerId: 'a', attrs: { Parsel: String(i + 1), Nitelik: i % 3 === 0 ? 'Tarla' : 'Arsa' }, p: { x: 0, y: 0 } }));
  // The geometry store's answer: an area on every object, 0 … 1000 m².
  const measures = new Float64Array(N * MEASURE_STRIDE);
  for (let i = 0; i < N; i++) {
    measures[i * MEASURE_STRIDE] = 7;
    measures[i * MEASURE_STRIDE + 2] = (i * 2.22) % 1000;
  }
  const rows: string[] = [];
  for (const [source, as] of CASES) {
    const r = compileExpression(source);
    if (!r.ok) throw new Error(r.error);
    const ms: number[] = [];
    for (let run = 0; run < WARM + RUNS; run++) {
      const t = performance.now();
      const c = r.expr.evaluateAll({ entities, layerName: (id) => id, measures: () => measures }, as);
      let filled = 0;
      for (let i = 0; i < N; i++) if (c.value(i) !== null) filled++;
      if (run >= WARM) ms.push(performance.now() - t);
      if (filled !== N) throw new Error(`${source}: ${N - filled} boş`);
    }
    rows.push(`${source.padEnd(34)} ${stats(ms)}`);
  }
  console.log(`\n${N} nesne, değerler okunarak\n${rows.join('\n')}`);
});
