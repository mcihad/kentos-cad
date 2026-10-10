// Veri aktar on the web (docs/adr/0216 §7), the same work as the desktop's `transfer_timing`
// (crates/native/processing/tests/cases.rs, `transfer_scene`): 10 000 parcels on a layer with typed fields and a deed
// list of 10 000 rows (five fields mapped, 100 new rows added from their east and north); the whole run through the
// runner on the page (the check, the tool, its one undo step), each run undone before the next, p50 and p95 after a
// warm-up against the ADR's budget. Runs only on purpose, with the WASM that ships:
//   KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm; TRANSFER_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/transfer.test.ts --disable-console-intercept; rm apps/web/src/wasm/pkg/.stamp
import { expect, it } from 'vitest';
import { CadDocument } from '../../src/model/document';
import { LayerStore } from '../../src/model/layers';
import { readSnapshot } from '../../src/model/snapshot';
import { transfer } from '../../src/processing/builtin/transfer';
import { clientExecutor } from '../../src/processing/job';
import { defaultValues } from '../../src/processing/parameters';
import { ProcessingRunner } from '../../src/processing/runner';

const RUNS = Number(process.env.TRANSFER_RUNS ?? 10);
const WARM = 2;
const BUDGET = 300;

/** The desktop's `transfer_scene`, value for value. */
function scene(): { doc: CadDocument; values: Record<string, unknown> } {
  const [e0, n0] = [487_000, 4_420_000];
  const entities = [];
  for (let i = 0; i < 10_000; i++) {
    const [x, y] = [e0 + (i % 100) * 20, n0 + Math.floor(i / 100) * 20];
    entities.push({
      kind: 'polygon',
      id: i + 1,
      layerId: 'parsel',
      attrs: { Parsel: String(i + 1), Ada: '101', Mahalle: 'Merkez' },
      pts: [
        { x, y },
        { x: x + 20, y },
        { x: x + 20, y: y + 20 },
        { x, y: y + 20 },
      ],
    });
  }
  const fields = [
    { name: 'Parsel', kind: 'integer', required: true },
    { name: 'Ada', kind: 'integer', required: true },
    { name: 'Malik', kind: 'text', length: 40 },
    { name: 'Hisse', kind: 'text' },
    { name: 'Alan', kind: 'decimal', scale: 2 },
    { name: 'Tescil', kind: 'date' },
    { name: 'Durum', kind: 'text', default: 'Beyanlı' },
    { name: 'Mahalle', kind: 'text', required: true, default: 'Merkez' },
  ];
  const json = JSON.stringify({
    format: 'kentos.document',
    version: 1,
    name: 'Tapu',
    settings: { srid: 5256, lengthDecimals: 3, areaDecimals: 2, areaUnit: 'm2', angleUnit: 'grad', plotScale: 1000, workspace: 'gis', drawingFont: 'barlow' },
    origin: { x: e0, y: n0 },
    layers: [{ id: 'parsel', name: 'Parsel', type: 'layer', visible: true, locked: false, expanded: true, style: { color: 'fg', lineType: 'continuous', lineWeight: 0.25 }, fields, children: [] }],
    activeLayer: 'parsel',
    entities,
    styles: { items: [], categories: [] },
  });
  const read = readSnapshot(json);
  if (!read.ok) throw new Error(read.error);
  const doc = new CadDocument({ name: read.content.name, layers: new LayerStore([], ''), origin: read.content.origin });
  doc.replaceWith(read.content);
  const rows: string[][] = [['PARSEL', 'ADA', 'MALIK_ADI', 'HISSE', 'YUZOLCUM', 'TESCIL_TARIHI', 'Y', 'X']];
  for (let i = 1; i <= 10_000; i++) {
    const key = i <= 9_900 ? i : 10_000 + i - 9_900;
    rows.push([String(key), '101', `Malik ${i}`, '1/2', `${400 + (i % 100)},25`, '03.05.2021', String(e0 + 2_100 + (i % 10)), String(n0 + Math.floor(i / 10))]);
  }
  const values = {
    target: { scope: 'layer', layerId: 'parsel' },
    targetKey: 'Parsel',
    sourceKind: 'file',
    file: { name: 'tapu.csv', rows },
    sourceKey: 'PARSEL',
    map: [
      { target: 'Malik', source: 'MALIK_ADI', default: '' },
      { target: 'Hisse', source: 'HISSE', default: '' },
      { target: 'Alan', source: 'YUZOLCUM', default: '' },
      { target: 'Tescil', source: 'TESCIL_TARIHI', default: '' },
      { target: 'Ada', source: 'ADA', default: '' },
    ],
    existing: 'overwrite',
    geometry: false,
    append: true,
    east: 'Y',
    north: 'X',
    reportOnly: false,
  };
  return { doc, values };
}

it.runIf(!!process.env.TRANSFER_BENCH)('measures Veri aktar against its budget', async () => {
  const { doc, values } = scene();
  const runner = new ProcessingRunner({ doc, selectedIds: () => [], visibleBounds: () => null, select: () => {} }, [clientExecutor]);
  const all = { ...defaultValues(transfer, runner.defaults()), ...values };
  const ms: number[] = [];
  let summary = '';
  for (let run = 0; run < WARM + RUNS; run++) {
    const t = performance.now();
    const outcome = await runner.run(transfer, all as never, { target: 'client' });
    const took = performance.now() - t;
    if (outcome.status !== 'ok') throw new Error(JSON.stringify(outcome));
    summary = outcome.record.summary;
    expect(doc.undo()).toBeTruthy();
    if (run >= WARM) ms.push(took);
  }
  const s = [...ms].sort((a, b) => a - b);
  const [p50, p95] = [s[Math.floor(s.length / 2)], s[Math.min(s.length - 1, Math.floor(0.95 * s.length))]];
  console.log(`Veri aktar, 10 000 parsel ve 10 000 satır (beş alan, 100 ekleme): p50 ${p50.toFixed(1)} ms, p95 ${p95.toFixed(1)} ms, bütçe ${BUDGET} ms\n  ${summary}`);
  expect(p50).toBeLessThanOrEqual(BUDGET);
}, 120_000);
