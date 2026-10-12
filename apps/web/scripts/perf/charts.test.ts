// Grafikler ve alan istatistikleri on the web (docs/adr/0227 §11), the same work as the desktop's
// `charts_on_a_large_layer` (crates/native/interaction/tests/perf.rs), the same objects to the digit: 100 000 points with
// Grup of 30 values, Değer with two fraction digits and Yaş. Their data read (the fields' cells checked, the columns
// handed to the core in WASM and read: a summary of each column), a field's summary, a field's statistics and values,
// Çubuk (Toplam, 30 categories) with its layout, Histogram and Kutu (30 groups), Saçılım of 100 000 points with its
// layout and its points handed back, the selection's marks and a hover's hit. The median of repeated runs after a
// warm-up against the ADR's web budget. Runs only on purpose, with the shipped WASM (docs/adr/0170 §3; remove its
// stamp afterwards so the next test run builds the tests' own again):
//   KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm
//   CHARTS_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/charts.test.ts --disable-console-intercept
//   rm apps/web/src/wasm/pkg/.stamp
import { expect, it } from 'vitest';
import type { Entity } from '../../src/model/entities';
import type { LayerField } from '../../src/model/layerFields';
import { layerColumns } from '../../src/model/chartData';
import { ChartData, type ChartFrame, type ChartSpec } from '../../src/model/ops/charts';

const RUNS = Number(process.env.CHARTS_RUNS ?? 7);
const WARM = 1;
const N = 100_000;

/** The desktop's points: Grup, Değer and (with `xy`) Yaş from the R2 sequence's fractions. */
function entities(xy: boolean): Entity[] {
  const out: Entity[] = [];
  for (let i = 0; i < N; i++) {
    const u = (i * 0.754877666246693) % 1;
    const v = (i * 0.5698402909980532) % 1;
    const attrs: Record<string, string> = { Grup: `G${String(1 + (i % 30)).padStart(2, '0')}`, Değer: (1000 + 9000 * u).toFixed(2) };
    if (xy) attrs['Yaş'] = String(Math.floor(v * 80));
    out.push({ id: i + 1, kind: 'point', layerId: 'nokta', attrs, p: { x: 487000 + u * 1000, y: 4420000 + v * 1000 } } as Entity);
  }
  return out;
}

const median = (ms: number[]) => [...ms].sort((a, b) => a - b)[Math.floor(ms.length / 2)];

/** Runs `work` repeatedly; its median against `budget`, said. */
function measure(name: string, work: () => void, budget: number): boolean {
  const ms: number[] = [];
  for (let run = 0; run < WARM + RUNS; run++) {
    const t = performance.now();
    work();
    if (run >= WARM) ms.push(performance.now() - t);
  }
  const m = median(ms);
  const ok = m <= budget;
  console.log(`  ${name.padEnd(52)} ${m.toFixed(2).padStart(7)} ms; bütçe ${String(budget).padStart(4)} ms: ${ok ? 'bütçede' : 'BÜTÇEYİ AŞIYOR'}`);
  return ok;
}

it.runIf(!!process.env.CHARTS_BENCH)('measures the charts against their budgets', () => {
  const grup: LayerField = { name: 'Grup', kind: 'text' };
  const deger: LayerField = { name: 'Değer', kind: 'decimal', scale: 2 };
  const yas: LayerField = { name: 'Yaş', kind: 'integer' };
  const frame: ChartFrame = { width: 900, height: 260, font: 12, family: 'Plus Jakarta Sans' };
  const rows = Uint32Array.from({ length: N }, (_, i) => i);
  console.log(`\nGrafikler (web), ${N} nesne:`);
  let ok = true;
  const two = entities(false);
  ok &&= measure(
    'verinin kurulması (2 sütun)',
    () => {
      const data = new ChartData(N, layerColumns([grup, deger], two));
      data.summary(0, rows);
      data.summary(1, rows);
      data.free();
    },
    80,
  );
  const data = new ChartData(N, layerColumns([grup, deger, yas], entities(true)));
  ok &&= measure('özet tablonun bir alanı (Değer)', () => void data.summary(1, rows), 25);
  ok &&= measure('seçilen alanın istatistikleri ve değerleri (Değer)', () => void data.detail(1, rows), 50);
  const chart = (spec: ChartSpec, points = false) => () => {
    data.chart(spec, rows);
    data.layOut(frame);
    if (points) {
      data.points();
      data.pointElements();
    }
  };
  ok &&= measure('çubuk (Toplam, 30 kategori), model ve yerleşim', chart({ kind: 'bar', category: 'Grup', aggregate: 'sum', value: 'Değer' }), 30);
  ok &&= measure('histogram, model ve yerleşim', chart({ kind: 'histogram', value: 'Değer' }), 50);
  ok &&= measure('kutu (30 grup), model ve yerleşim', chart({ kind: 'box', value: 'Değer', group: 'Grup' }), 50);
  ok &&= measure('saçılım (100 000 nokta), model, yerleşim ve noktalar', chart({ kind: 'scatter', x: 'Değer', y: ['Yaş'] }, true), 50);
  const halves = [rows.filter((r) => r % 2 === 0), rows.filter((r) => r % 2 === 1)];
  let k = 0;
  ok &&= measure('seçimin vurgusu (50 000 seçili nesne)', () => void data.select(halves[++k % 2]), 8);
  data.layOut(frame);
  let worst = 0;
  const t0 = performance.now();
  for (let i = 0; i < 1000; i++) {
    const t = performance.now();
    data.hit(20 + ((i * 0.7548777) % 1) * 860, 20 + ((i * 0.5698403) % 1) * 220);
    worst = Math.max(worst, performance.now() - t);
  }
  const hover = (performance.now() - t0) / 1000;
  const hoverOk = hover <= 2;
  console.log(`  ${`üzerine gelme (tıklama sınaması; en yavaşı ${(worst * 1000).toFixed(0)} µs)`.padEnd(52)} ${hover.toFixed(2).padStart(7)} ms; bütçe    2 ms: ${hoverOk ? 'bütçede' : 'BÜTÇEYİ AŞIYOR'}`);
  data.free();
  expect(ok && hoverOk).toBe(true);
});
