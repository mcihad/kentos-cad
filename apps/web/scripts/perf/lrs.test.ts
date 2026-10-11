// Doğrusal referanslama on the web (docs/adr/0224 §4), the same work as the core's
// `lrs::timing` (crates/shared/geometry-core/tests/all/lrs.rs): 1 000 routes of 1 km
// (100 edges, every tenth an arc), 100 000 point and 10 000 line events, 100 000
// points to locate, 10 000 pieces of 100 routes in no order, 50 000 and 50 000 line
// events to overlay, 10 000 calibration marks; the same scene to the bit (the core's
// SplitMix64). Each through the operation the tools call (`model/ops/lrs`: one call
// a run, whole lists in and out). p50 and p95 of repeated runs after a warm-up,
// against the ADR's budget (2.5 times the core's). Runs only on purpose, with the
// WASM that ships:
//   KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm; LRS_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/lrs.test.ts --disable-console-intercept; rm apps/web/src/wasm/pkg/.stamp
import { cpus, release } from 'node:os';
import { expect, it } from 'vitest';
import type { EntityGeometry } from '../../src/model/entities';
import { lrsCalibrate, lrsCreate, lrsEvents, lrsLocate, lrsOverlay, type EventIn, type LineEvent, type Mark, type Piece, type Route } from '../../src/model/ops/lrs';

const RUNS = Number(process.env.LRS_RUNS ?? 10);
const WARM = 2;

/** p50 and p95 in milliseconds. */
function quantiles(ms: number[]): [number, number] {
  const s = [...ms].sort((a, b) => a - b);
  const q = (p: number) => s[Math.min(s.length - 1, Math.floor(p * s.length))];
  return [q(0.5), q(0.95)];
}

/** Runs a work; its p50 against `budget`, said. */
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
  console.log(`${name.padEnd(52)} p50 ${p50.toFixed(2).padStart(8)} ms (p95 ${p95.toFixed(2).padStart(8)}) ${String(n).padStart(7)} kayıt, bütçe ${budget.toFixed(0).padStart(4)} ms: ${ok ? 'bütçede' : 'BÜTÇEYİ AŞIYOR'}`);
  return ok;
}

/** The core's test's generator (SplitMix64), to the bit. */
class Mix {
  private s: bigint;
  constructor(seed: bigint) {
    this.s = seed;
  }
  next(): number {
    const M = (1n << 64n) - 1n;
    this.s = (this.s + 0x9e3779b97f4a7c15n) & M;
    let z = this.s;
    z = ((z ^ (z >> 30n)) * 0xbf58476d1ce4e5b9n) & M;
    z = ((z ^ (z >> 27n)) * 0x94d049bb133111ebn) & M;
    return Number((z ^ (z >> 31n)) >> 11n) / 2 ** 53;
  }
}

it.runIf(!!process.env.LRS_BENCH)('measures the linear referencing work against its budgets', () => {
  const routes: Route[] = Array.from({ length: 1000 }, (_, r) => ({
    parts: [
      {
        pts: Array.from({ length: 101 }, (_, i) => ({ x: i * 10, y: r * 50 + (i % 2 === 1 ? 2 : 0) })),
        bulges: Array.from({ length: 100 }, (_, i) => (i % 10 === 5 ? 0.2 : 0)),
        ms: Array.from({ length: 101 }, (_, i) => i * 10),
      },
    ],
  }));
  const mix = new Mix(0x4c525321n);
  const points: EventIn[] = Array.from({ length: 100_000 }, (_, i) => ({ route: i % 1000, m: mix.next() * 1000, offset: i % 2 === 0 ? 5 : -5 }));
  const lines: EventIn[] = Array.from({ length: 10_000 }, (_, i) => {
    const from = mix.next() * 900;
    return { route: i % 1000, from, to: from + 10 + mix.next() * 90, offset: 8 };
  });
  const features: EntityGeometry[] = Array.from({ length: 100_000 }, () => {
    const x = mix.next() * 1000;
    const y = mix.next() * 50_000 - 25;
    return { kind: 'point', p: { x, y } } as EntityGeometry;
  });
  // 10 000 pieces of 100 routes, 100 a route, in no order.
  const pieces: Piece[] = Array.from({ length: 10_000 }, (_, i) => {
    const [k, j] = [Math.floor(i / 100), i % 100];
    return { id: `R${k}`, pts: [{ x: j * 10, y: k * 50 }, { x: j * 10 + 10, y: k * 50 }] };
  });
  for (let i = pieces.length - 1; i >= 1; i--) {
    const j = Math.floor(mix.next() * (i + 1));
    [pieces[i], pieces[j]] = [pieces[j], pieces[i]];
  }
  const side = (): LineEvent[] =>
    Array.from({ length: 50_000 }, (_, i) => {
      const from = mix.next() * 950;
      return { route: `R${i % 1000}`, from, to: from + 1 + mix.next() * 49 };
    });
  const [a, b] = [side(), side()];
  // Ten marks a route, 1 % off its measures.
  const marks: Mark[][] = Array.from({ length: 1000 }, (_, r) =>
    Array.from({ length: 10 }, (_, j) => {
      const x = j * 100 + 40;
      return { p: { x, y: r * 50 }, m: x * 1.01 };
    }),
  );
  const placed = (out: { fault?: unknown }[]) => out.filter((e) => !e.fault).length;
  const over: string[] = [];
  console.log(`\n${routes.length} rota, ${points.length} nokta olayı, ${lines.length} çizgi olayı, ${features.length} nokta, ${pieces.length} parça, ${a.length} + ${b.length} olay`);
  for (const [name, work, budget] of [
    ['Olayları rotada göster: 100 000 nokta, 1 000 rota', () => placed(lrsEvents(routes, points)), 250],
    ['Olayları rotada göster: 10 000 çizgi, ötelemeli', () => placed(lrsEvents(routes, lines)), 250],
    ['Rota boyunca konumla: 100 000 nokta, 20 m', () => lrsLocate(routes, features, 20, false).filter((r) => r.length > 0).length, 750],
    ['Rota oluştur: 10 000 parçadan 100 rota', () => lrsCreate(pieces, { method: 'length' }).routes.length, 50],
    ['Olayları çakıştır: 50 000 ve 50 000 olay, birleşim', () => lrsOverlay(a, b, true).length, 375],
    ['Rotayı kalibre et: 1 000 rota, 10 000 nokta', () => lrsCalibrate(routes, marks, { radius: 1 }).filter((c) => !c.fault).length, 250],
  ] as [string, () => number, number][])
    if (!measure(name, work, budget)) over.push(name);
  // The scene's answers, as the core's test has them.
  expect(placed(lrsEvents(routes, points))).toBe(points.length);
  expect(placed(lrsEvents(routes, lines))).toBe(lines.length);
  console.log(`\n${cpus()[0]?.model ?? '?'}, ${cpus().length} iş parçacığı; Linux ${release()}; Node ${process.version}`);
  expect(over, `bütçeyi aşanlar: ${over.join(', ')}`).toEqual([]);
}, 900_000);
