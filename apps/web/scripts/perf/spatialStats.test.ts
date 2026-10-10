// Mekânsal istatistik on the web (docs/adr/0238 §12): each tool's call to the geometry core as the tools make it (the
// objects' geometry and the values as JSON both ways), over 100 000 points on 10 km × 10 km with a smooth value and
// noise; the band gives about ten neighbours a point (the desktop's crates/shared/geometry-core/tests/all/
// spatial_stats_timing.rs, the same scene). Node and the WASM package, one process; p50 and p95 of repeated runs after
// a warm-up. Runs only on purpose, with the shipped WASM (docs/adr/0170 §3; remove its stamp afterwards so the next
// test run builds the tests' own again):
//   KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm
//   STATS_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/spatialStats.test.ts --disable-console-intercept
//   rm apps/web/src/wasm/pkg/.stamp
import { expect, it } from 'vitest';
import type { EntityGeometry } from '../../src/model/entities';
import { statsCenters, statsDbscan, statsHotSpots, statsKMeans, statsMoran, statsNearest } from '../../src/model/ops/spatialStats';

const RUNS = Number(process.env.STATS_RUNS ?? 5);
const WARM = 1;
const N = 100_000;

/** The desktop's hash: the same places and values. */
function hash(i: bigint): bigint {
  const M = (1n << 64n) - 1n;
  let h = (i * 0x9e3779b97f4a7c15n) & M;
  h ^= h >> 31n;
  h = (h * 0xbf58476d1ce4e5b9n) & M;
  return h ^ (h >> 29n);
}

function scene(n: number): { shapes: EntityGeometry[]; values: string[] } {
  const shapes: EntityGeometry[] = [];
  const values: string[] = [];
  for (let i = 0; i < n; i++) {
    const x = Number(hash(2n * BigInt(i)) % 1_000_000n) / 100;
    const y = Number(hash(2n * BigInt(i) + 1n) % 1_000_000n) / 100;
    shapes.push({ kind: 'point', p: { x: 500_000 + x, y: 4_400_000 + y } } as EntityGeometry);
    const v = 100 + 40 * Math.sin(x / 1500) * Math.cos(y / 2100) + Number(hash(BigInt(i) + 7n) % 1000n) / 50;
    values.push(v.toFixed(2));
  }
  return { shapes, values };
}

function stats(ms: number[]): string {
  const s = [...ms].sort((a, b) => a - b);
  const q = (p: number) => s[Math.min(s.length - 1, Math.floor(p * s.length))];
  return `p50 ${q(0.5).toFixed(1).padStart(7)} ms, p95 ${q(0.95).toFixed(1).padStart(7)} ms`;
}

function time(name: string, budget: number, f: () => unknown): void {
  const ms: number[] = [];
  for (let k = 0; k < WARM + RUNS; k++) {
    const t = performance.now();
    f();
    if (k >= WARM) ms.push(performance.now() - t);
  }
  console.log(`${name.padEnd(44)} ${stats(ms)}  bütçe ${budget} ms`);
}

it.runIf(!!process.env.STATS_BENCH)('measures the spatial statistics tools on the web', () => {
  const { shapes, values } = scene(N);
  const band = Math.sqrt(10 / (Math.PI * 1e-3));
  for (const [name, kind] of [
    ['Ortalama merkez', 'mean'],
    ['Ortanca merkez', 'median'],
    ['Standart uzaklık', 'distance'],
    ['Yön dağılımı', 'ellipse'],
  ] as const) {
    time(name, 600, () => statsCenters(shapes, kind, null, null, '', 1));
  }
  time('En yakın komşu', 1500, () => statsNearest(shapes, null));
  time('Moran I, sabit bant', 3000, () => statsMoran(shapes, values, 'Değer', 'band', band, 8, true));
  time('Sıcak nokta, sabit bant', 3000, () => statsHotSpots(shapes, values, 'Değer', 'band', band, 8));
  time('DBSCAN, ε 40 m, 5 nokta', 3000, () => statsDbscan(shapes, 40, 5, false));
  time('k-ortalamalar, k = 10', 3000, () => statsKMeans(shapes, 10));
  expect(statsKMeans(shapes, 10).summary).toMatch(/^100000 nesne 10 kümeye ayrıldı/);
}, 300_000);
