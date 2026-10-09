// Katman süzgeci on the web (docs/adr/0211 §6): a layer's filter set and the geometry store following it as the
// viewport asks it (compiled, asked of every object, marked, counted), for a condition on an attribute, one on a
// geometry value and a list of objects; beside it the evaluation alone. 100 000 parcels, a third of them “Arsa”. Node
// and the WASM package, one process; p50 and p95 of repeated runs after a warm-up. Runs only on purpose, with the
// shipped WASM (docs/adr/0170 §3; remove its stamp afterwards so the next test run builds the tests' own again):
//   KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm
//   FILTER_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/filter.test.ts --disable-console-intercept
//   rm apps/web/src/wasm/pkg/.stamp
import { expect, it } from 'vitest';
import type { LayerFilter } from '../../src/contracts/generated/LayerFilter';
import type { NewEntity } from '../../src/model/entities';
import { compileFilter, filterPassesIn } from '../../src/model/layerFilter';
import type { LayerStyle } from '../../src/model/layers';
import { layerDocument } from '../../src/style/cases';
import { PickIndex } from '../../src/viewport/picking';

const RUNS = Number(process.env.FILTER_RUNS ?? 10);
const WARM = 2;
const N = 100_000;
const X0 = 486000;
const Y0 = 4420000;
const STYLE: LayerStyle = { color: '#3E63DD', lineType: 'continuous', lineWeight: 0.25 };
const KINDS = ['Arsa', 'Tarla', 'Bağ'];

/** Five-cornered parcels, a little skewed each (crates/native/interaction/tests/perf.rs' drawing). */
function parcels(n: number): NewEntity[] {
  const out: NewEntity[] = [];
  const side = Math.ceil(Math.sqrt(n));
  for (let i = 0; i < n; i++) {
    const x = X0 + (i % side) * 20;
    const y = Y0 + Math.floor(i / side) * 20;
    const k = (((Math.floor(i / side) * 31 + (i % side) * 17) % 7) as number) * 0.137;
    const pts = [
      { x, y },
      { x: x + 18, y: y + k },
      { x: x + 18.5, y: y + 9 },
      { x: x + 17, y: y + 18 - k },
      { x: x + k, y: y + 17.5 },
    ];
    out.push({ kind: 'polygon', layerId: 'k', attrs: { Nitelik: KINDS[i % KINDS.length] }, pts });
  }
  return out;
}

function stats(ms: number[]): string {
  const s = [...ms].sort((a, b) => a - b);
  const q = (p: number) => s[Math.min(s.length - 1, Math.floor(p * s.length))];
  return `p50 ${q(0.5).toFixed(1).padStart(7)} ms, p95 ${q(0.95).toFixed(1).padStart(7)} ms`;
}

it.runIf(!!process.env.FILTER_BENCH)('measures a layer filter on the web', () => {
  const doc = layerDocument('Parsel', STYLE, parcels(N));
  const index = new PickIndex(doc);
  index.filterCounts('k');
  const list = doc.byLayer('k');
  const uids = list.filter((_, i) => i % 2 === 0).map((e) => doc.uidOf(e.id)!);
  const filters: [string, LayerFilter, number][] = [
    ["öznitelik ifadesi (Nitelik = 'Arsa')", { expression: "Nitelik = 'Arsa'" }, 50],
    ['geometri ifadesi ($alan > 310)', { expression: '$alan > 310' }, 150],
    ['nesne listesi (her ikinci parsel)', { objects: uids }, 10],
  ];
  const rows: string[] = [];
  for (const [name, filter, budget] of filters) {
    const store: number[] = [];
    const alone: number[] = [];
    let counted = { passed: 0, total: 0 };
    for (let run = 0; run < WARM + RUNS; run++) {
      doc.setLayerFilter('k', null, 'Katman süzgeci');
      index.filterCounts('k');
      doc.setLayerFilter('k', filter, 'Katman süzgeci');
      let t = performance.now();
      counted = index.filterCounts('k')!;
      if (run >= WARM) store.push(performance.now() - t);
      t = performance.now();
      const r = compileFilter(filter);
      if (!r.ok) throw new Error(r.error);
      const pass = filterPassesIn(doc, r.filter, list, () => 'Parsel', index);
      if (run >= WARM) alone.push(performance.now() - t);
      expect(pass.filter(Boolean).length).toBe(counted.passed);
    }
    rows.push(`${name.padEnd(40)} geçen ${counted.passed}/${counted.total}: depo ${stats(store)}; değerlendirme ${stats(alone)}   bütçe ${budget} ms`);
  }
  console.log(`\nKatman süzgeci, ${N} parsel:\n${rows.join('\n')}`);
});
