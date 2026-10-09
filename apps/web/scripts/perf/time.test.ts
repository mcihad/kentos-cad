// Zaman sürgüsü on the web (docs/adr/0210 §12): a temporal layer's objects' times read into the geometry store as
// the viewport reads them, and the step the slider takes there: the window to the store, the shown objects asked
// in one call and the layer built again whole (the web has no parts). Parcels whose validity starts one of a
// hundred years apart, none ending: a step shows 1 % more. Node and the WASM package, one process; p50 and p95 of
// repeated runs after a warm-up. Runs only on purpose, with the shipped WASM (docs/adr/0170 §3: the tests' own is
// built without whole-program optimisation; remove its stamp afterwards so the next test run builds that one again):
//   KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm
//   TIME_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/time.test.ts --disable-console-intercept
//   rm apps/web/src/wasm/pkg/.stamp
import { it } from 'vitest';
import type { NewEntity } from '../../src/model/entities';
import type { LayerStyle } from '../../src/model/layers';
import { readTime } from '../../src/model/time';
import { buildStyledLayer } from '../../src/render/styledLayer';
import { layerDocument, PALETTE } from '../../src/style/cases';
import { SYSTEM_LIBRARY } from '../../src/style/system';
import { PickIndex } from '../../src/viewport/picking';

const RUNS = Number(process.env.TIME_RUNS ?? 10);
const WARM = 2;
const X0 = 486000;
const Y0 = 4420000;
const STYLE: LayerStyle = { color: '#E06C75', lineType: 'continuous', lineWeight: 0.35 };
const library = { symbol: (id: string) => SYSTEM_LIBRARY.items.find((i) => i.kind === 'symbol' && i.id === id)?.symbol, asset: () => undefined };

function parcels(n: number): NewEntity[] {
  const out: NewEntity[] = [];
  for (let i = 0; i < n; i++) {
    const x = X0 + (i % 300) * 31.7;
    const y = Y0 + Math.floor(i / 300) * 27.3;
    const pts = Array.from({ length: 20 }, (_, k) => {
      const a = (k / 20) * 2 * Math.PI;
      return { x: x + 12.5 + 11 * Math.cos(a), y: y + 12.5 + 9 * Math.sin(a) };
    });
    out.push({ kind: 'polygon', layerId: 'k', attrs: { baslangic: `${1925 + (i % 100)}-01-01` }, pts });
  }
  return out;
}

function stats(ms: number[]): string {
  const s = [...ms].sort((a, b) => a - b);
  const q = (p: number) => s[Math.min(s.length - 1, Math.floor(p * s.length))];
  return `p50 ${q(0.5).toFixed(1).padStart(7)} ms, p95 ${q(0.95).toFixed(1).padStart(7)} ms`;
}

function year(y: number): number {
  const t = readTime(`${y}-01-01`);
  if (t.kind !== 'moment') throw new Error(`${y}`);
  return t.t;
}

it.runIf(!!process.env.TIME_BENCH)('measures the time slider on the web', () => {
  const rows: string[] = [];
  for (const n of [10_000, 100_000]) {
    const doc = layerDocument(`Parsel ×${n}`, STYLE, parcels(n));
    const reads: number[] = [];
    const index = new PickIndex(doc);
    index.timeSummary();
    for (let run = 0; run < WARM + RUNS; run++) {
      doc.setLayerTime('k', null, 'Zaman ayarları');
      index.timeSummary();
      // The layer's time set: the store reads every object's time as it next answers, and the slider's range.
      doc.setLayerTime('k', { start: 'baslangic', end: 'bitis' }, 'Zaman ayarları');
      const t = performance.now();
      index.timeSummary();
      if (run >= WARM) reads.push(performance.now() - t);
    }
    rows.push(`${`${n} nesnenin zamanı`.padEnd(40)} ${stats(reads)}   bütçe 30 ms (100 000'de)`);
    if (n > 10_000) continue;
    const list = doc.byLayer('k');
    const ids = list.map((e) => e.id);
    const steps: number[] = [];
    for (let run = 0; run < WARM + RUNS; run++) {
      const t = performance.now();
      index.setTimeWindow({ kind: 'instant', a: year(1990 + run) });
      const shown = index.timeShown(ids);
      const kept = list.filter((_, i) => shown[i]);
      buildStyledLayer('k', kept, STYLE, { origin: { x: X0, y: Y0 }, palette: PALETTE, plotScale: 1000, library, layerName: () => 'Parsel', geometry: index });
      if (run >= WARM) steps.push(performance.now() - t);
    }
    rows.push(`${`${n} nesnede sürgünün bir adımı`.padEnd(40)} ${stats(steps)}   bütçe 50 ms`);
  }
  console.log(`\nZaman sürgüsü (${RUNS} koşu)\n${rows.join('\n')}`);
}, 1_800_000);
