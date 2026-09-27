// The expression language on 10⁵ and 10⁶ objects (docs/adr/0008 “İfade dili”,
// docs/adr/0100): the table the app builds, the core's evaluation in WASM,
// and every value read back, as a processing run or a layer's style asks.
// Node and the WASM package, one process; p50 and p95 of repeated runs after
// a warm-up. Runs only on purpose:
//   EXPRESSION_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/expression.test.ts --disable-console-intercept
// EXPRESSION_N sets the numbers of objects (comma separated, default 100000),
// EXPRESSION_RUNS the runs; with EXPRESSION_PERF_OUT (a directory, relative to
// the repository) and EXPRESSION_PERF_LABEL the table is also written to
// `<out>/expression-web-<label>-<date>.md` (docs/perf/README.md).
import { execFileSync } from 'node:child_process';
import { writeFileSync } from 'node:fs';
import { cpus, release, totalmem } from 'node:os';
import { it } from 'vitest';
import type { Entity } from '../../src/model/entities';
import { CoreStore } from '../../src/wasm/core';
import { compileExpression, type ExprAs } from '../../src/model/expression/expression';
import { MEASURE_STRIDE } from '../../src/model/expression/expressionLib';

const SIZES = (process.env.EXPRESSION_N ?? '100000').split(',').map(Number);
const RUNS = Number(process.env.EXPRESSION_RUNS ?? 20);
const WARM = 3;
const ROOT = new URL('../../../../', import.meta.url);

/** A condition, a numbering, an area as text, a rounded number, a geometry condition, arithmetic on an attribute. */
const CASES: [string, ExprAs][] = [
  ["Nitelik = 'Arsa' ve $alan > 500", 'bool'],
  ["'P' || doldur($sıra, 5)", 'text'],
  ["metin($alan, 2) || ' m²'", 'text'],
  ['yuvarla($alan, 2)', 'number'],
  ['$alan / 10000 > 0.05 ve $uzunluk < 400', 'bool'],
  ['Parsel * 2 + 1', 'number'],
];

/** p50 and p95 in milliseconds. */
function quantiles(ms: number[]): [number, number] {
  const s = [...ms].sort((a, b) => a - b);
  const q = (p: number) => s[Math.min(s.length - 1, Math.floor(p * s.length))];
  return [q(0.5), q(0.95)];
}

/** A command's first line of output, or "?". */
function output(cmd: string, args: string[]): string {
  try {
    return execFileSync(cmd, args, { encoding: 'utf8', cwd: ROOT }).split('\n')[0].trim();
  } catch {
    return '?';
  }
}

function measure(n: number): [number, number][] {
  const entities: Entity[] = Array.from({ length: n }, (_, i) => ({ id: i + 1, kind: 'point', layerId: 'a', attrs: { Parsel: String(i + 1), Nitelik: i % 3 === 0 ? 'Tarla' : 'Arsa' }, p: { x: 0, y: 0 } }));
  // The geometry store's answer: an area on every object, 0 … 1000 m².
  const measures = new Float64Array(n * MEASURE_STRIDE);
  for (let i = 0; i < n; i++) {
    measures[i * MEASURE_STRIDE] = 7;
    measures[i * MEASURE_STRIDE + 2] = (i * 2.22) % 1000;
  }
  const out: [number, number][] = [];
  console.log(`\n${n} nesne, değerler okunarak`);
  for (const [source, as] of CASES) {
    const r = compileExpression(source);
    if (!r.ok) throw new Error(r.error);
    const ms: number[] = [];
    for (let run = 0; run < WARM + RUNS; run++) {
      const t = performance.now();
      const c = r.expr.evaluateAll({ entities, layerName: (id) => id, measures: () => measures }, as);
      let filled = 0;
      for (let i = 0; i < n; i++) if (c.value(i) !== null) filled++;
      if (run >= WARM) ms.push(performance.now() - t);
      if (filled !== n) throw new Error(`${source}: ${n - filled} boş`);
    }
    const [p50, p95] = quantiles(ms);
    console.log(`${source.padEnd(40)} p50 ${p50.toFixed(1).padStart(7)} ms, p95 ${p95.toFixed(1).padStart(7)} ms (${ms.length} koşu)`);
    out.push([p50, p95]);
  }
  return out;
}

/** Geometry expressions on squares in a geometry store: read there from the shapes, or through its measures answer. */
const STORE_CASES: [string, 'geometry' | 'measures'][] = [
  ['$alan > 500', 'geometry'],
  ['$alan > 500', 'measures'],
  ['yuvarla($alan, 2)', 'geometry'],
  ['yuvarla($alan, 2)', 'measures'],
  ['$merkez_y', 'geometry'],
  ['$genişlik * $yükseklik', 'geometry'],
];

function measureStore(n: number): number[] {
  const entities: Entity[] = Array.from({ length: n }, (_, i) => {
    const x = 487000 + (i % 1000) * 20;
    const y = 4420000 + Math.floor(i / 1000) * 20;
    const s = 10 + (i % 4);
    return { id: i + 1, kind: 'polygon', layerId: 'a', attrs: {}, pts: [{ x, y }, { x: x + s, y }, { x: x + s, y: y + s }, { x, y: y + s }] };
  });
  const store = new CoreStore();
  store.put(JSON.stringify(entities));
  const out: number[] = [];
  console.log(`\n${n} kare deponun içinde, değerler okunarak`);
  for (const [source, path] of STORE_CASES) {
    const r = compileExpression(source);
    if (!r.ok) throw new Error(r.error);
    const objects = path === 'geometry' ? { entities, layerName: (id: string) => id, geometry: store } : { entities, layerName: (id: string) => id, measures: () => store.measures(Float64Array.from(entities, (e) => e.id)) };
    const ms: number[] = [];
    for (let run = 0; run < WARM + RUNS; run++) {
      const t = performance.now();
      const c = r.expr.evaluateAll(objects, 'value');
      let filled = 0;
      for (let i = 0; i < n; i++) if (c.value(i) !== null) filled++;
      if (run >= WARM) ms.push(performance.now() - t);
      if (filled !== n) throw new Error(`${source}: ${n - filled} boş`);
    }
    const [p50] = quantiles(ms);
    console.log(`${source.padEnd(28)} ${path.padEnd(9)} p50 ${p50.toFixed(1).padStart(7)} ms`);
    out.push(p50);
  }
  store.dispose();
  return out;
}

it.runIf(!!process.env.EXPRESSION_BENCH)(`measures expressions on ${SIZES.join(', ')} objects`, () => {
  const results = SIZES.map(measure);
  // The store takes its objects as JSON here: up to 200 000 (a million squares would be 160 MB of text).
  const STORE_SIZES = SIZES.filter((n) => n <= 200_000);
  const inStore = STORE_SIZES.map(measureStore);
  const dir = process.env.EXPRESSION_PERF_OUT;
  if (!dir) return;
  const label = process.env.EXPRESSION_PERF_LABEL ?? 'olcum';
  const day = new Date().toISOString().slice(0, 10);
  const commit = output('git', ['rev-parse', '--short', 'HEAD']);
  const machine = `${cpus()[0]?.model ?? '?'}, ${cpus().length} iş parçacığı, ${Math.round(totalmem() / 2 ** 30)} GB; Linux ${release()}; Node ${process.version}, WASM \`--profile wasm\``;
  const head = `| İfade | Biçim |${SIZES.map((n) => ` ${n} nesne |`).join('')}\n|---|---|${SIZES.map(() => '---|').join('')}`;
  const rows = CASES.map(([source, as], k) => `| \`${source.replace(/\|/g, '\\|')}\` | ${as} |${results.map((r) => ` ${r[k][0].toFixed(1)} / ${r[k][1].toFixed(1)} |`).join('')}`);
  const md = `# İfade motoru, web (WASM yolu): ${label} (${day}, ${commit})

${machine}. ${RUNS} koşu (${WARM} ısınma), p50 / p95 ms.

Test \`apps/web/scripts/perf/expression.test.ts\`: nokta nesneleri (Parsel 1…n, her üçüncüsü Tarla ötekiler Arsa), deponun ölçü yanıtında 0…1000 m² alan. Süre sayfanın tabloyu kurmasını (\`exprTable\`), çekirdeğin değerlendirmesini (\`exprEvaluate\`) ve bütün değerlerin okunmasını kapsar.

${head}
${rows.join('\n')}

Geometri deposundaki kareler (10 × 10 … 13 × 13 m; ADR 0100 §3), p50 ms: “depoda” ifade deponun içinde değerlendirilir ve geometri değerlerini şekillerden okur (\`CoreStore.evaluateExpression\`); “ölçü kaydıyla” deponun \`measures\` yanıtı alınır ve \`exprEvaluate\`'e verilir (sayfanın bugünkü yolu, iki kopya dahil).

| İfade | Yol |${STORE_SIZES.map((n) => ` ${n} nesne |`).join('')}
|---|---|${STORE_SIZES.map(() => '---|').join('')}
${STORE_CASES.map(([source, path], k) => `| \`${source}\` | ${path === 'geometry' ? 'depoda' : 'ölçü kaydıyla'} |${inStore.map((r) => ` ${r[k].toFixed(1)} |`).join('')}`).join('\n')}
`;
  const path = new URL(`${dir}/expression-web-${label}-${day}.md`, ROOT);
  writeFileSync(path, md);
  console.log(`\n${path.pathname} yazıldı.`);
}, 900_000);
