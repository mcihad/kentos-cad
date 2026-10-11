// Genelleştirme on the web (docs/adr/0221 §10), the desktop's `generalize_timing` work
// (crates/native/processing/tests/cases.rs: `coverage_scene`, `buildings_scene`, `slivers_scene`): a coverage of
// 10 000 and of 100 000 wobbling parcels, 10 000 skewed and stepped buildings in blocks of four (2 500 for Alanları
// kümele, which closes its clusters one after another here and on several threads on the desktop), 100 000 parcels with
// 5 000 slivers; each whole run through the runner on the page (the check, the tool, its one undo step), each run undone
// before the next, p50 and p95 after a warm-up against the ADR's budgets. Runs only on purpose, with the WASM that ships:
//   KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm; GENERALIZE_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/generalize.test.ts --disable-console-intercept; rm apps/web/src/wasm/pkg/.stamp
import { expect, it } from 'vitest';
import { CadDocument } from '../../src/model/document';
import { LayerStore } from '../../src/model/layers';
import { readSnapshot } from '../../src/model/snapshot';
import { aggregate } from '../../src/processing/builtin/generalize/aggregate';
import { building } from '../../src/processing/builtin/generalize/building';
import { coverageSimplify } from '../../src/processing/builtin/generalize/coverage';
import { eliminate } from '../../src/processing/builtin/generalize/eliminate';
import { dropParts } from '../../src/processing/builtin/generalize/parts';
import { smooth } from '../../src/processing/builtin/generalize/smooth';
import { square } from '../../src/processing/builtin/generalize/square';
import { clientExecutor } from '../../src/processing/job';
import { defaultValues } from '../../src/processing/parameters';
import { ProcessingRunner } from '../../src/processing/runner';
import type { ProcessingTool } from '../../src/processing/types';

const RUNS = Number(process.env.GENERALIZE_RUNS ?? 4);
const WARM = 2;
const [E0, N0] = [487_000, 4_420_000];

/** A drawing of polygons (id, attributes, ring) on one layer. */
function scene(name: string, polygons: [number, Record<string, string>, [number, number][]][]): CadDocument {
  const entities = polygons.map(([id, attrs, ring]) => ({ kind: 'polygon', id, layerId: 'g', attrs, pts: ring.map(([x, y]) => ({ x: E0 + x, y: N0 + y })) }));
  const json = JSON.stringify({
    format: 'kentos.document',
    version: 1,
    name,
    settings: { srid: 5256, lengthDecimals: 3, areaDecimals: 2, areaUnit: 'm2', angleUnit: 'grad', plotScale: 1000, workspace: 'gis', drawingFont: 'barlow' },
    origin: { x: E0, y: N0 },
    layers: [{ id: 'g', name: 'Genelleştirme', type: 'layer', visible: true, locked: false, expanded: true, style: { color: 'fg', lineType: 'continuous', lineWeight: 0.25 }, children: [] }],
    activeLayer: 'g',
    entities,
    styles: { items: [], categories: [] },
  });
  const read = readSnapshot(json);
  if (!read.ok) throw new Error(read.error);
  const doc = new CadDocument({ name: read.content.name, layers: new LayerStore([], ''), origin: read.content.origin });
  doc.replaceWith(read.content);
  return doc;
}

const wobble = (a: number, b: number, k: number) => (((a * 31 + b * 17 + k * 7) % 9) - 4) * 0.02;

/** The desktop's `coverage_scene`: parcels of 20 m, three wobbling inner vertices a side, shared exactly. */
function coverage(cols: number, rows: number): CadDocument {
  const polygons: [number, Record<string, string>, [number, number][]][] = [];
  for (let r = 0; r < rows; r++) {
    for (let c = 0; c < cols; c++) {
      const [x0, y0] = [c * 20, r * 20];
      const [x1, y1] = [x0 + 20, y0 + 20];
      const ring: [number, number][] = [[x0, y0]];
      for (let k = 1; k < 4; k++) ring.push([x0 + 5 * k, y0 + wobble(c, 2 * r, k)]);
      ring.push([x1, y0]);
      for (let k = 1; k < 4; k++) ring.push([x1 + wobble(c + 1, 2 * r + 1, k), y0 + 5 * k]);
      ring.push([x1, y1]);
      for (let k = 3; k >= 1; k--) ring.push([x0 + 5 * k, y1 + wobble(c, 2 * (r + 1), k)]);
      ring.push([x0, y1]);
      for (let k = 3; k >= 1; k--) ring.push([x0 + wobble(c, 2 * r + 1, k), y0 + 5 * k]);
      const id = r * cols + c + 1;
      polygons.push([id, { Parsel: String(id) }, ring]);
    }
  }
  return scene('Kaplama', polygons);
}

/** The desktop's `buildings_scene`: blocks of four stepped buildings 3 m apart, turned up to 1.2°. */
function buildings(blocks: number): CadDocument {
  const shape: [number, number][] = [
    [0, 0],
    [12, 0],
    [12, 8],
    [6, 8],
    [6, 8.8],
    [0, 8.8],
  ];
  const polygons: [number, Record<string, string>, [number, number][]][] = [];
  let id = 0;
  for (let b = 0; b < blocks; b++) {
    const [bx, by] = [(b % 50) * 70, Math.floor(b / 50) * 60];
    for (const [ox, oy] of [
      [0, 0],
      [15, 0],
      [0, 11.8],
      [15, 11.8],
    ]) {
      id++;
      const turn = (((id % 7) - 3) * 0.4 * Math.PI) / 180;
      const [sin, cos] = [Math.sin(turn), Math.cos(turn)];
      const ring = shape.map(([x, y], i): [number, number] => {
        const [nx, ny] = [((id * 7 + i * 3) % 7) * 0.05 - 0.15, ((id * 5 + i * 11) % 7) * 0.05 - 0.15];
        const [px, py] = [x - 6 + nx, y - 4.4 + ny];
        return [bx + ox + 6 + px * cos - py * sin, by + oy + 4.4 + px * sin + py * cos];
      });
      polygons.push([id, { Bina: String(id), Blok: String(b + 1) }, ring]);
    }
  }
  return scene('Binalar', polygons);
}

/** The desktop's `slivers_scene`: rows of parcels, nineteen 20 m wide then a 2 m sliver. */
function slivers(cols: number, rows: number): CadDocument {
  const polygons: [number, Record<string, string>, [number, number][]][] = [];
  for (let r = 0; r < rows; r++) {
    let x = 0;
    const y0 = r * 20;
    for (let c = 0; c < cols; c++) {
      const w = c % 20 === 19 ? 2 : 20;
      const id = r * cols + c + 1;
      polygons.push([
        id,
        { Parsel: String(id) },
        [
          [x, y0],
          [x + w, y0],
          [x + w, y0 + 20],
          [x, y0 + 20],
        ],
      ]);
      x += w;
    }
  }
  return scene('Şeritler', polygons);
}

const ALL = { scope: 'all' };
const RUNS_OF: readonly { tool: ProcessingTool; title: string; scene: () => CadDocument; values: Record<string, unknown>; budget: number }[] = [
  {
    tool: smooth,
    title: 'Yumuşat, 10 000 parselde ortak kenarlarla, üç yineleme',
    scene: () => coverage(100, 100),
    values: { input: ALL, iterations: 3, offset: 0.25, maxAngle: 180, shared: true },
    budget: 1200,
  },
  { tool: square, title: 'Dik açılandır, 10 000 eğik bina', scene: () => buildings(2500), values: { input: ALL, threshold: 15, iterations: 1000 }, budget: 1500 },
  { tool: building, title: 'Bina sadeleştir, 10 000 basamaklı bina, 3 m', scene: () => buildings(2500), values: { input: ALL, minEdge: 3, parallel: 20, orthogonal: 20 }, budget: 800 },
  {
    tool: aggregate,
    title: 'Alanları kümele, 2 500 bina dörtlü bloklarda, 5 m',
    scene: () => buildings(625),
    values: { input: ALL, distance: 5, minArea: 0, minHole: 0, fillNotches: false },
    budget: 8000,
  },
  { tool: dropParts, title: 'Küçük parçaları ayıkla, 100 000 parsel', scene: () => coverage(320, 313), values: { input: ALL, minArea: 10, percent: 0, parts: true, holes: true }, budget: 1500 },
  { tool: eliminate, title: 'Küçükleri komşuya kat, 100 000 parselin 5 000 şeridi', scene: () => slivers(400, 250), values: { input: ALL, minArea: 100, into: 'longest' }, budget: 2000 },
  {
    tool: coverageSimplify,
    title: 'Ortak kenarları koruyarak sadeleştir, 100 000 parsel, 1,6 milyon köşe, 1 m',
    scene: () => coverage(320, 313),
    values: { input: ALL, tolerance: 1, keepBoundary: false },
    budget: 4000,
  },
];

it.runIf(!!process.env.GENERALIZE_BENCH)('measures Genelleştirme against its budgets', async () => {
  const over: string[] = [];
  const only = process.env.GENERALIZE_ONLY;
  for (const { tool, title, scene: make, values, budget } of RUNS_OF) {
    if (only && !only.split(',').some((o) => tool.id.endsWith(o))) continue;
    const doc = make();
    const runner = new ProcessingRunner({ doc, selectedIds: () => [], visibleBounds: () => null, select: () => {} }, [clientExecutor]);
    const all = { ...defaultValues(tool, runner.defaults()), ...values };
    const ms: number[] = [];
    let summary = '';
    for (let run = 0; run < WARM + RUNS; run++) {
      const t = performance.now();
      const outcome = await runner.run(tool, all as never, { target: 'client' });
      const took = performance.now() - t;
      if (outcome.status !== 'ok') throw new Error(`${tool.id}: ${JSON.stringify(outcome)}`);
      summary = outcome.record.summary;
      expect(doc.undo()).toBeTruthy();
      if (run >= WARM) ms.push(took);
    }
    const s = [...ms].sort((a, b) => a - b);
    const [p50, p95] = [s[Math.floor(s.length / 2)], s[Math.min(s.length - 1, Math.floor(0.95 * s.length))]];
    console.log(`${title}: p50 ${p50.toFixed(1)} ms, p95 ${p95.toFixed(1)} ms, bütçe ${budget} ms\n  ${summary}`);
    if (p50 > budget) over.push(`${title}: p50 ${p50.toFixed(1)} ms > ${budget} ms`);
  }
  expect(over).toEqual([]);
}, 1_800_000);
