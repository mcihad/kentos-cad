// Geometri üretimi on the web (docs/adr/0219 §8), the same work as the desktop's `generate_timing`
// (crates/native/processing/tests/cases.rs, `scatter_scene`, `layer_scene`): 100 000 scattered points' Voronoi cells,
// Delaunay triangles and concave hull, 100 000 parcels' rotated boxes and poles; each whole run through the runner on
// the page (the check, the tool, its one undo step), each run undone before the next, p50 and p95 after a warm-up
// against the ADR's budgets. Runs only on purpose, with the WASM that ships:
//   KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm; GENERATE_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/generate.test.ts --disable-console-intercept; rm apps/web/src/wasm/pkg/.stamp
import { expect, it } from 'vitest';
import { CadDocument } from '../../src/model/document';
import { LayerStore } from '../../src/model/layers';
import { readSnapshot } from '../../src/model/snapshot';
import { bounding, concaveHull } from '../../src/processing/builtin/generate/bounding';
import { pole } from '../../src/processing/builtin/generate/pole';
import { delaunay, voronoi } from '../../src/processing/builtin/generate/triangulation';
import { clientExecutor } from '../../src/processing/job';
import { defaultValues } from '../../src/processing/parameters';
import { ProcessingRunner } from '../../src/processing/runner';
import type { ProcessingTool } from '../../src/processing/types';

const RUNS = Number(process.env.GENERATE_RUNS ?? 6);
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

/** The desktop's `scatter_scene`, value for value: 100 000 points over 2 km by 2 km to the centimetre. */
function scatter(): CadDocument {
  const [e0, n0] = [487_000, 4_420_000];
  let x = 20_261_011;
  const next = () => {
    x = (Math.imul(x, 1_664_525) + 1_013_904_223) >>> 0;
    return (x % 200_000) / 100;
  };
  const entities = [];
  for (let i = 0; i < 100_000; i++) {
    const [dx, dy] = [next(), next()];
    entities.push({ kind: 'point', id: i + 1, layerId: 'nokta', attrs: { No: `P${i + 1}` }, p: { x: e0 + dx, y: n0 + dy } });
  }
  const json = JSON.stringify({
    format: 'kentos.document',
    version: 1,
    name: 'Noktalar',
    settings: { srid: 5256, lengthDecimals: 3, areaDecimals: 2, areaUnit: 'm2', angleUnit: 'grad', plotScale: 1000, workspace: 'gis', drawingFont: 'barlow' },
    origin: { x: e0, y: n0 },
    layers: [{ id: 'nokta', name: 'Nokta', type: 'layer', visible: true, locked: false, expanded: true, style: { color: 'fg', lineType: 'continuous', lineWeight: 0.25 }, children: [] }],
    activeLayer: 'nokta',
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
  { tool: voronoi, title: 'Voronoi alanları, 100 000 nokta', scene: scatter, values: { input: ALL, margin: 10 }, budget: 1500 },
  { tool: delaunay, title: 'Delaunay üçgenleri, 100 000 nokta (200 000 üçgen)', scene: scatter, values: { input: ALL, name: 'No' }, budget: 1500 },
  { tool: concaveHull, title: 'İçbükey zarf, 100 000 nokta, oran 0,3', scene: scatter, values: { input: ALL, ratio: 0.3, holes: false, group: 'all' }, budget: 1500 },
  { tool: bounding, title: 'Sınırlayıcı geometri, 100 000 parsel, döndürülmüş kutu', scene: parcels, values: { input: ALL, kind: 'rotated', group: 'each' }, budget: 1200 },
  { tool: pole, title: 'Ulaşılmazlık kutbu, 100 000 parsel, 0,1 m', scene: parcels, values: { input: ALL, precision: 0.1 }, budget: 2500 },
];

it.runIf(!!process.env.GENERATE_BENCH)('measures Geometri üretimi against its budgets', async () => {
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
