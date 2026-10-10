// Katman işlemleri on the web (docs/adr/0217 §9), the same work as the desktop's `layer_timing`
// (crates/native/processing/tests/cases.rs, `layer_scene`): 100 000 parcels in two layers of 50 000 split by a field of
// 50 values, their duplicates by two fields and the shape deleted, ordered by their area and the two layers merged; each
// whole run through the runner on the page (the check, the tool, its one undo step), each run undone before the next,
// p50 and p95 after a warm-up against the ADR's budgets. Runs only on purpose, with the WASM that ships:
//   KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm; LAYERS_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/layers.test.ts --disable-console-intercept; rm apps/web/src/wasm/pkg/.stamp
import { expect, it } from 'vitest';
import { CadDocument } from '../../src/model/document';
import { LayerStore } from '../../src/model/layers';
import { readSnapshot } from '../../src/model/snapshot';
import { deleteDuplicates } from '../../src/processing/builtin/layers/dedupe';
import { mergeLayers } from '../../src/processing/builtin/layers/merge';
import { orderByExpression } from '../../src/processing/builtin/layers/orderBy';
import { splitByAttribute } from '../../src/processing/builtin/layers/split';
import { clientExecutor } from '../../src/processing/job';
import { defaultValues } from '../../src/processing/parameters';
import { ProcessingRunner } from '../../src/processing/runner';
import type { ProcessingTool } from '../../src/processing/types';

const RUNS = Number(process.env.LAYERS_RUNS ?? 6);
const WARM = 2;

/** The desktop's `layer_scene`, value for value. */
function scene(): CadDocument {
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

const ALL = { scope: 'all' };
const RUNS_OF: readonly { tool: ProcessingTool; title: string; values: Record<string, unknown>; budget: number }[] = [
  { tool: splitByAttribute, title: 'Özniteliğe göre böl, 100 000 parsel, 50 değer, kopya', values: { input: ALL, fields: 'Mahalle', mode: 'copy' }, budget: 800 },
  { tool: deleteDuplicates, title: 'Yinelenenleri sil, 100 000 parsel, iki alan ve geometri', values: { input: ALL, fields: 'Mahalle, Ada', geometry: true }, budget: 500 },
  {
    tool: orderByExpression,
    title: 'İfadeye göre sırala, 100 000 parsel, $alan, kopya',
    values: { input: ALL, expression: '$alan', direction: 'desc', nulls: 'last', orderField: 'Sıra', output: { newName: 'Sıralı' } },
    budget: 800,
  },
  {
    tool: mergeLayers,
    title: "Vektör katmanlarını birleştir, iki 50 000'lik katman",
    values: { layers: ['pa', 'pb'], map: [], sourceField: 'Kaynak katman', output: { newName: 'Birleşik' } },
    budget: 800,
  },
];

it.runIf(!!process.env.LAYERS_BENCH)('measures Katman işlemleri against their budgets', async () => {
  const doc = scene();
  const runner = new ProcessingRunner({ doc, selectedIds: () => [], visibleBounds: () => null, select: () => {} }, [clientExecutor]);
  const over: string[] = [];
  for (const { tool, title, values, budget } of RUNS_OF) {
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
}, 600_000);
