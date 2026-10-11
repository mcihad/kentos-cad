// Örnekleme on the web (docs/adr/0220 §7), the same work as the desktop's `sampling_timing`
// (crates/native/processing/tests/cases.rs, `layer_scene`, `stars_scene`): a hexagonal grid over 100 000 parcels
// keeping the cells that meet them, regular, random and along-the-boundary points on them, ten 10 000-vertex stars cut
// into pieces of at most 256 vertices; each whole run through the runner on the page (the check, the tool, its one
// undo step), each run undone before the next, p50 and p95 after a warm-up against the ADR's budgets. Runs only on
// purpose, with the WASM that ships:
//   KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm; SAMPLING_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/sampling.test.ts --disable-console-intercept; rm apps/web/src/wasm/pkg/.stamp
import { expect, it } from 'vitest';
import { CadDocument } from '../../src/model/document';
import { LayerStore } from '../../src/model/layers';
import { readSnapshot } from '../../src/model/snapshot';
import { alongLines } from '../../src/processing/builtin/sampling/along';
import { grid } from '../../src/processing/builtin/sampling/grid';
import { randomPoints, regularPoints } from '../../src/processing/builtin/sampling/points';
import { subdivide } from '../../src/processing/builtin/sampling/subdivide';
import { clientExecutor } from '../../src/processing/job';
import { defaultValues } from '../../src/processing/parameters';
import { ProcessingRunner } from '../../src/processing/runner';
import type { ProcessingTool } from '../../src/processing/types';

const RUNS = Number(process.env.SAMPLING_RUNS ?? 6);
const WARM = 2;

/** The desktop's `layer_scene`, value for value. */
function parcels(): CadDocument {
  const [e0, n0] = [487_000, 4_420_000];
  const entities = [];
  for (let i = 0; i < 100_000; i++) {
    const k = i % 100 === 99 ? i - 1 : i;
    const [x, y] = [e0 + (k % 316) * 20, n0 + Math.floor(k / 316) * 20];
    const size = 10 + (k % 7);
    entities.push({
      kind: 'polygon',
      id: i + 1,
      layerId: i < 50_000 ? 'pa' : 'pb',
      attrs: { Parsel: String(k + 1), Ada: String(Math.floor(k / 100) % 1000), Mahalle: `Mahalle ${(k % 50) + 1}` },
      pts: [
        { x, y },
        { x: x + size, y },
        { x: x + size, y: y + size },
        { x, y: y + size },
      ],
    });
  }
  const layer = (id: string, name: string) => ({
    id,
    name,
    type: 'layer',
    visible: true,
    locked: false,
    expanded: true,
    style: { color: 'fg', lineType: 'continuous', lineWeight: 0.25 },
    children: [],
  });
  const json = JSON.stringify({
    format: 'kentos.document',
    version: 1,
    name: 'Katmanlar',
    settings: { srid: 5256, lengthDecimals: 3, areaDecimals: 2, areaUnit: 'm2', angleUnit: 'grad', plotScale: 1000, workspace: 'gis', drawingFont: 'barlow' },
    origin: { x: e0, y: n0 },
    layers: [layer('pa', 'Pa'), layer('pb', 'Pb')],
    activeLayer: 'pa',
    entities,
    styles: { items: [], categories: [] },
  });
  const read = readSnapshot(json);
  if (!read.ok) throw new Error(read.error);
  const doc = new CadDocument({ name: read.content.name, layers: new LayerStore([], ''), origin: read.content.origin });
  doc.replaceWith(read.content);
  return doc;
}

/** The desktop's `stars_scene`, value for value: ten stars of 10 000 vertices. */
function stars(): CadDocument {
  const [e0, n0] = [487_000, 4_420_000];
  const entities = [];
  for (let s = 0; s < 10; s++) {
    const [cx, cy] = [e0 + 1000 * s, n0];
    const pts = [];
    for (let i = 0; i < 10_000; i++) {
      const t = (Math.PI * i) / 5000;
      const r = i % 2 === 0 ? 400 : 300 + (i % 7) * 10;
      pts.push({ x: cx + r * Math.cos(t), y: cy + r * Math.sin(t) });
    }
    entities.push({ kind: 'polygon', id: s + 1, layerId: 'y', attrs: { Ad: `Yıldız ${s + 1}` }, pts });
  }
  const json = JSON.stringify({
    format: 'kentos.document',
    version: 1,
    name: 'Yıldızlar',
    settings: { srid: 5256, lengthDecimals: 3, areaDecimals: 2, areaUnit: 'm2', angleUnit: 'grad', plotScale: 1000, workspace: 'gis', drawingFont: 'barlow' },
    origin: { x: e0, y: n0 },
    layers: [{ id: 'y', name: 'Yıldız', type: 'layer', visible: true, locked: false, expanded: true, style: { color: 'fg', lineType: 'continuous', lineWeight: 0.25 }, children: [] }],
    activeLayer: 'y',
    entities,
    styles: { items: [], categories: [] },
  });
  const read = readSnapshot(json);
  if (!read.ok) throw new Error(read.error);
  const doc = new CadDocument({ name: read.content.name, layers: new LayerStore([], ''), origin: read.content.origin });
  doc.replaceWith(read.content);
  return doc;
}

const ALL = { scope: 'all' };
const RUNS_OF: readonly { tool: ProcessingTool; title: string; scene: () => CadDocument; values: Record<string, unknown>; budget: number }[] = [
  {
    tool: grid,
    title: "Ağ oluştur, 100 000 parselin kutusunda 12 m'lik altıgenler, Nesnelere değenler",
    scene: parcels,
    values: { input: ALL, shape: 'hexagon', side: 12, snap: true, keep: 'meeting' },
    budget: 1500,
  },
  { tool: regularPoints, title: 'Düzenli noktalar, 100 000 parselde 12 m aralık', scene: parcels, values: { input: ALL, dx: 12, dy: 12, snap: true, shift: 0 }, budget: 1300 },
  {
    tool: randomPoints,
    title: 'Rastgele noktalar, 100 000 parselde birer, en az 2 m arayla',
    scene: parcels,
    values: { input: ALL, count: '1', min: 2, tries: 10, seed: 1 },
    budget: 1300,
  },
  {
    tool: alongLines,
    title: "Hat boyunca nokta, 100 000 parselin sınırında 25 m'de bir",
    scene: parcels,
    values: { input: ALL, mode: 'interval', step: 25, start: 0, end: 0, ends: true },
    budget: 1800,
  },
  { tool: subdivide, title: 'Alt bölümlere ayır, 10 alanda 100 000 köşe, en çok 256', scene: stars, values: { input: ALL, max: 256 }, budget: 1500 },
];

it.runIf(!!process.env.SAMPLING_BENCH)('measures Örnekleme against its budgets', async () => {
  const over: string[] = [];
  for (const { tool, title, scene, values, budget } of RUNS_OF) {
    const doc = scene();
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
}, 900_000);
