// The language's additions on the web (docs/adr/0214 §5), the same work as the
// core's `measures_the_additions` (crates/shared/expression/tests/perf.rs): date
// parts, a regular expression and arrays on 10⁵ objects; an aggregate by group,
// a spatial relation over 500 protected areas, the nearest of 2 000 stops and a
// value from another layer on 10⁴ parcels, the layers given as one table each
// run. The table the app builds, the core's evaluation in WASM and every value
// read back; p50 and p95 of repeated runs after a warm-up, against the ADR's
// budget (2.5 times the core's). Runs only on purpose, with the WASM that ships:
//   KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm; EXTRAS_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/expressionExtras.test.ts --disable-console-intercept; rm apps/web/src/wasm/pkg/.stamp
import { cpus, release } from 'node:os';
import { expect, it } from 'vitest';
import type { Entity } from '../../src/model/entities';
import { CoreStore } from '../../src/wasm/core';
import { compileExpression, type ExprAs, type ExprObjects } from '../../src/model/expression/expression';
import { exprLayers } from '../../src/model/expression/layers';

const RUNS = Number(process.env.EXPRESSION_RUNS ?? 20);
const WARM = 3;

/** p50 and p95 in milliseconds. */
function quantiles(ms: number[]): [number, number] {
  const s = [...ms].sort((a, b) => a - b);
  const q = (p: number) => s[Math.min(s.length - 1, Math.floor(p * s.length))];
  return [q(0.5), q(0.95)];
}

/** Runs and reads an expression's values; its p50 against `budget`, said. */
function measure(source: string, as: ExprAs, objects: ExprObjects, budget: number): boolean {
  const r = compileExpression(source, { world: true });
  if (!r.ok) throw new Error(`${source}: ${r.error}`);
  const n = objects.entities.length;
  const ms: number[] = [];
  for (let run = 0; run < WARM + RUNS; run++) {
    const t = performance.now();
    const c = r.expr.evaluateAll(objects, as);
    let filled = 0;
    for (let i = 0; i < n; i++) if (c.value(i) !== null) filled++;
    if (run >= WARM) ms.push(performance.now() - t);
    if (filled !== n) throw new Error(`${source}: ${n - filled} boş`);
  }
  const [p50, p95] = quantiles(ms);
  const ok = p50 <= budget;
  console.log(`${source.padEnd(50)} p50 ${p50.toFixed(2).padStart(7)} ms (p95 ${p95.toFixed(2).padStart(7)}), bütçe ${budget.toFixed(1).padStart(6)} ms: ${ok ? 'bütçede' : 'BÜTÇEYİ AŞIYOR'}`);
  return ok;
}

const square = (id: number, layerId: string, x: number, y: number, side: number, attrs: Record<string, string> = {}): Entity => ({
  id,
  kind: 'polygon',
  layerId,
  attrs,
  pts: [
    { x, y },
    { x: x + side, y },
    { x: x + side, y: y + side },
    { x, y: y + side },
  ],
});

it.runIf(!!process.env.EXTRAS_BENCH)('measures the language additions against their budgets', () => {
  const over: string[] = [];
  const pad = (k: number) => String(k).padStart(2, '0');
  // 10⁵ objects: dates, names with a number, tags.
  const n = 100_000;
  const rows: Entity[] = Array.from({ length: n }, (_, i) => ({
    id: i + 1,
    kind: 'point',
    layerId: 'parsel',
    p: { x: 0, y: 0 },
    attrs: { Tarih: `20${pad(10 + (i % 17))}-${pad(1 + (i % 12))}-${pad(1 + (i % 28))}`, Ad: `Parsel ${i} B`, Etiketler: ['imar', 'ifraz,tevhit', 'a,b,c,d'][i % 3] },
  }));
  console.log(`\n${n} nesne, bütün değerler okunarak`);
  for (const [source, as, budget] of [
    ['yıl(Tarih)', 'number', 37.5],
    ["düzenli_parça(Ad, '(\\d+)')", 'text', 100],
    ['dizi_uzunluğu(metin_dizi(Etiketler))', 'number', 100],
  ] as [string, ExprAs, number][])
    if (!measure(source, as, { entities: rows, layerName: () => 'Parsel' }, budget)) over.push(source);
  // 10⁴ parcels in a 100 × 100 grid of 20 m squares, 500 protected areas, 2 000 stops, 100 districts.
  const side = 100;
  const parcels: Entity[] = [];
  for (let r = 0; r < side; r++)
    for (let c = 0; c < side; c++) {
      const d = Math.floor(r / 10) * 10 + Math.floor(c / 10);
      parcels.push(square(r * side + c + 1, 'parsel', c * 20, r * 20, 20, { Mahalle: `M${d}`, MahalleKodu: `K${d}` }));
    }
  const sit = Array.from({ length: 500 }, (_, k) => square(100_000 + k, 'sit', (k * 37) % 1960, (k * 53) % 1960, 35, { Ad: `Sit ${k}` }));
  const stops: Entity[] = Array.from({ length: 2000 }, (_, k) => ({ id: 200_000 + k, kind: 'point', layerId: 'durak', p: { x: ((k * 97) % 2000) + 0.5, y: ((k * 61) % 2000) + 0.5 }, attrs: { Ad: `Durak ${k}` } }));
  const districts = Array.from({ length: 100 }, (_, k) => square(300_000 + k, 'mahalle', (k % 10) * 200, Math.floor(k / 10) * 200, 200, { Kod: `K${k}`, Ad: `Mahalle ${k}` }));
  const store = new CoreStore();
  store.put(JSON.stringify([...parcels, ...sit, ...stops, ...districts]));
  const names: Record<string, string> = { parsel: 'Parsel', sit: 'Sit alanı', durak: 'Durak', mahalle: 'Mahalle' };
  const byLayer: Record<string, Entity[]> = { parsel: parcels, sit, durak: stops, mahalle: districts };
  const layers = exprLayers(
    Object.entries(names).map(([id, name]) => [id, name] as const),
    (id) => byLayer[id] ?? [],
  );
  const objects: ExprObjects = { entities: parcels, layerName: (id) => names[id] ?? id, geometry: store, layers };
  console.log(`\n${parcels.length} parsel, dünyanın tablosu her koşuda kurularak`);
  for (const [source, as, budget] of [
    ['$alan / topla($alan, Mahalle)', 'number', 37.5],
    ["kesişir('Sit alanı')", 'bool', 150],
    ["en_yakın('Durak', Ad)", 'text', 200],
    ["katmandan('Mahalle', Ad, 'Kod', MahalleKodu)", 'text', 25],
  ] as [string, ExprAs, number][])
    if (!measure(source, as, objects, budget)) over.push(source);
  store.dispose();
  console.log(`\n${cpus()[0]?.model ?? '?'}, ${cpus().length} iş parçacığı; Linux ${release()}; Node ${process.version}`);
  expect(over, `bütçeyi aşanlar: ${over.join(', ')}`).toEqual([]);
}, 900_000);
