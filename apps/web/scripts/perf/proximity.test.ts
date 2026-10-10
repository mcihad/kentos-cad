// Yakınlık analizi on the web (docs/adr/0215 §6), the same work as the core's
// `proximity::timing` (crates/shared/geometry-core/tests/all/proximity.rs): 10 000
// parcels with moved corners and 104 squares overlapping them, 2 000 stops, 100
// hubs and 2 000 road pieces, the same scene to the bit; the five tools' searches
// through a run's store (`ObjectStore`: the core in WASM, every record read back
// as the tools read it). p50 and p95 of repeated runs after a warm-up, against
// the ADR's budget (2.5 times the core's). Runs only on purpose, with the WASM
// that ships:
//   KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm; PROXIMITY_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/proximity.test.ts --disable-console-intercept; rm apps/web/src/wasm/pkg/.stamp
import { cpus, release } from 'node:os';
import { expect, it } from 'vitest';
import type { Entity } from '../../src/model/entities';
import type { Vec2 } from '../../src/model/geometry';
import { ObjectStore } from '../../src/processing/geometry';

const RUNS = Number(process.env.PROXIMITY_RUNS ?? 10);
const WARM = 2;

/** p50 and p95 in milliseconds. */
function quantiles(ms: number[]): [number, number] {
  const s = [...ms].sort((a, b) => a - b);
  const q = (p: number) => s[Math.min(s.length - 1, Math.floor(p * s.length))];
  return [q(0.5), q(0.95)];
}

/** Runs a search; its p50 against `budget`, said. */
function measure(name: string, work: () => number, budget: number): boolean {
  const ms: number[] = [];
  let n = 0;
  for (let run = 0; run < WARM + RUNS; run++) {
    const t = performance.now();
    n = work();
    if (run >= WARM) ms.push(performance.now() - t);
  }
  const [p50, p95] = quantiles(ms);
  const ok = p50 <= budget;
  console.log(`${name.padEnd(46)} p50 ${p50.toFixed(2).padStart(7)} ms (p95 ${p95.toFixed(2).padStart(7)}) ${String(n).padStart(6)} kayıt, bütçe ${budget.toFixed(0).padStart(4)} ms: ${ok ? 'bütçede' : 'BÜTÇEYİ AŞIYOR'}`);
  return ok;
}

/** The R2 sequence's fractions (Roberts 2018), as the core's test takes them. */
const r2 = (k: number): [number, number] => [(k * 0.754877666246693) % 1, (k * 0.5698402909980532) % 1];

const polygon = (id: number, layerId: string, pts: Vec2[]): Entity => ({ id, kind: 'polygon', layerId, attrs: {}, pts });
const point = (id: number, layerId: string, p: Vec2): Entity => ({ id, kind: 'point', layerId, attrs: {}, p });

it.runIf(!!process.env.PROXIMITY_BENCH)('measures the proximity searches against their budgets', () => {
  const [e0, n0] = [487_000, 4_420_000];
  const corner = (i: number, j: number): Vec2 => {
    const [u, v] = r2(i * 101 + j + 0.5);
    return { x: e0 + i * 20 + 6 * (u - 0.5), y: n0 + j * 20 + 6 * (v - 0.5) };
  };
  const objects: Entity[] = [];
  const parcels: number[] = [];
  const overlapping: number[] = [];
  for (let i = 0; i < 100; i++)
    for (let j = 0; j < 100; j++) {
      const id = i * 100 + j + 1;
      const pts = [corner(i, j), corner(i + 1, j), corner(i + 1, j + 1), corner(i, j + 1)];
      if ((i * 100 + j) % 97 === 0) {
        const x = (pts[0].x + pts[1].x + pts[2].x + pts[3].x) / 4;
        const y = (pts[0].y + pts[1].y + pts[2].y + pts[3].y) / 4;
        const small = 14_101 + overlapping.length;
        objects.push(polygon(small, 'parsel', [{ x: x - 2.5, y: y - 2.5 }, { x: x + 2.5, y: y - 2.5 }, { x: x + 2.5, y: y + 2.5 }, { x: x - 2.5, y: y + 2.5 }]));
        overlapping.push(small);
      }
      objects.push(polygon(id, 'parsel', pts));
      parcels.push(id);
    }
  const stops: number[] = [];
  for (let k = 0; k < 2000; k++) {
    const [u, v] = r2(k + 0.5);
    objects.push(point(10_001 + k, 'durak', { x: e0 + 2000 * u, y: n0 + 2000 * v }));
    stops.push(10_001 + k);
  }
  const hubs: number[] = [];
  for (let a = 0; a < 10; a++)
    for (let b = 0; b < 10; b++) {
      objects.push(point(12_001 + a * 10 + b, 'merkez', { x: e0 + 100 + a * 200, y: n0 + 100 + b * 200 }));
      hubs.push(12_001 + a * 10 + b);
    }
  const roads: number[] = [];
  for (let k = 0; k < 2000; k++) {
    const [u, v] = r2(k + 0.25);
    const [x, y] = [e0 + 2000 * u, n0 + 2000 * v];
    objects.push({ id: 12_101 + k, kind: 'line', layerId: 'yol', attrs: {}, a: { x, y }, b: { x: x + 15 * (2 * v - 1), y: y + 10 } });
    roads.push(12_101 + k);
  }
  const areas = [...parcels, ...overlapping];
  const store = new ObjectStore(objects);
  // The store is packed on the first question, as a run's is; not measured.
  store.inBox({ minX: 0, minY: 0, maxX: 1, maxY: 1 });
  const over: string[] = [];
  console.log(`\n${parcels.length} parsel, ${overlapping.length} örtüşen kare, ${stops.length} durak, ${hubs.length} merkez, ${roads.length} yol parçası`);
  for (const [name, work, budget] of [
    ['En yakını bul (10 000 → 2 000 durak)', () => store.nearest(parcels, stops, 1, Infinity, 'edges').length, 100],
    ['Uzaklık matrisi (10 000 → 2 000, k 5)', () => store.nearest(parcels, stops, 5, Infinity, 'edges').length, 150],
    ['En yakın merkeze bağla (10 000 → 100 merkez)', () => store.nearest(parcels, hubs, 1, Infinity, 'centers').length, 50],
    ['Komşu alanlar (10 104, köşeler, örtüşmeler)', () => store.neighbors(areas, 0.001, true, true).length, 250],
    ['En kısa çizgi (10 000 → 2 000 yol, k 1)', () => store.nearest(parcels, roads, 1, Infinity, 'edges').length, 100],
  ] as [string, () => number, number][])
    if (!measure(name, work, budget)) over.push(name);
  // The scene's answers, as the core's test has them.
  const found = store.neighbors(areas, 0.001, true, true);
  const kinds = (kind: string) => found.filter((f) => f.kind === kind).length;
  expect([kinds('edge'), kinds('corner'), kinds('overlap')]).toEqual([2 * 2 * 99 * 100, 2 * 2 * 99 * 99, 2 * overlapping.length]);
  expect(store.nearest(parcels, stops, 5, Infinity, 'edges').length).toBe(5 * parcels.length);
  store.dispose();
  console.log(`\n${cpus()[0]?.model ?? '?'}, ${cpus().length} iş parçacığı; Linux ${release()}; Node ${process.version}`);
  expect(over, `bütçeyi aşanlar: ${over.join(', ')}`).toEqual([]);
}, 900_000);
