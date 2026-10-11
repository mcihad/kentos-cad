// Geometri dönüştürme on the web (docs/adr/0218 §13), the same work as the desktop's `convert_timing`
// (crates/native/processing/tests/cases.rs, `layer_scene`, `route_points_scene`, `bearing_table`): 100 000 parcels'
// interior points, all their vertices as points and their edges densified at 5 m, 100 000 points joined into 1000
// ordered routes, 100 000 table rows as lines from a bearing and a length; and Tablodan nokta's (docs/adr/0223 §8,
// the desktop's `table_points_timing` and `point_table`): 100 000 rows in the project's plane and from WGS 84; each
// whole run through the runner on the page (the check, the tool, its one undo step), each run undone before the next,
// p50 and p95 after a warm-up against the ADRs' budgets. Runs only on purpose, with the WASM that ships:
//   KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm; CONVERT_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/convert.test.ts --disable-console-intercept; rm apps/web/src/wasm/pkg/.stamp
import { expect, it } from 'vitest';
import { CadDocument } from '../../src/model/document';
import { LayerStore } from '../../src/model/layers';
import { readSnapshot } from '../../src/model/snapshot';
import { densify } from '../../src/processing/builtin/convert/paths';
import { pointsToPath } from '../../src/processing/builtin/convert/pointsToPath';
import { bearingLine, tablePoints } from '../../src/processing/builtin/convert/tables';
import { toPoints } from '../../src/processing/builtin/convert/toPoints';
import { vertices } from '../../src/processing/builtin/convert/vertices';
import { clientExecutor } from '../../src/processing/job';
import { defaultValues } from '../../src/processing/parameters';
import { ProcessingRunner } from '../../src/processing/runner';
import type { ProcessingTool } from '../../src/processing/types';

const RUNS = Number(process.env.CONVERT_RUNS ?? 6);
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

/** The desktop's `route_points_scene`, value for value. */
function points(): CadDocument {
  const [e0, n0] = [487_000, 4_420_000];
  const entities = [];
  for (let i = 0; i < 100_000; i++) {
    entities.push({
      kind: 'point',
      id: i + 1,
      layerId: 'nokta',
      attrs: { Hat: `H${i % 1000}`, Sıra: String(100 - Math.floor(i / 1000)) },
      p: { x: e0 + (i % 400) * 5, y: n0 + Math.floor(i / 400) * 5 },
      z: (i % 97) / 4,
    });
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

/** The desktop's `bearing_table`, value for value. */
function bearingTable(): { name: string; rows: string[][] } {
  const rows = [['Ad', 'Doğu', 'Kuzey', 'Semt', 'Uzunluk']];
  for (let i = 0; i < 100_000; i++) {
    const semt = i % 10 === 0 ? `${i % 400},5` : String((i * 7) % 400);
    rows.push([`S${i}`, String(487_000 + (i % 400) * 5), String(4_420_000 + Math.floor(i / 400) * 5), semt, String(5 + (i % 20))]);
  }
  return { name: 'semt.csv', rows };
}

/** The desktop's `point_table`, value for value: the project's plane, or WGS 84 (every tenth row in DMS). */
function pointTable(geographic: boolean): { name: string; rows: string[][] } {
  const rows = [['Ad', 'Doğu', 'Kuzey', 'Kot', 'Kod']];
  const dms = (v: number) => {
    const d = Math.trunc(v);
    const m = Math.trunc((v - d) * 60);
    return `${d} ${m} ${(((v - d) * 60 - m) * 60).toFixed(3)}`;
  };
  for (let i = 0; i < 100_000; i++) {
    let e: string;
    let n: string;
    if (geographic) {
      const [lon, lat] = [35.95 + (i % 400) * 0.0002, 39.85 + Math.floor(i / 400) * 0.0002];
      [e, n] = i % 10 === 0 ? [dms(lon), dms(lat)] : [lon.toFixed(7), lat.toFixed(7)];
    } else {
      const [x, y] = [487_000 + (i % 400) * 5 + 0.125, 4_420_000 + Math.floor(i / 400) * 5 + 0.5];
      [e, n] = i % 10 === 0 ? [String(x).replace('.', ','), String(y).replace('.', ',')] : [String(x), String(y)];
    }
    rows.push([`N${i}`, e, n, String(800 + (i % 97)), ['SN', 'YL', 'BN'][i % 3]]);
  }
  return { name: geographic ? 'wgs84.csv' : 'duzlem.csv', rows };
}

const ALL = { scope: 'all' };
const POINT_COLUMNS = { sourceKind: 'file', east: 'Doğu', north: 'Kuzey', elevation: 'Kot', name: 'Ad' };
const RUNS_OF: readonly { tool: ProcessingTool; title: string; scene: () => CadDocument; values: Record<string, unknown>; budget: number }[] = [
  { tool: toPoints, title: 'Nesneden nokta, 100 000 parsel, iç nokta', scene: parcels, values: { input: ALL, location: 'inside', parts: false }, budget: 800 },
  { tool: vertices, title: 'Köşeleri noktaya çevir, 100 000 parsel, bütün köşeler (400 000 nokta)', scene: parcels, values: { input: ALL, which: 'all' }, budget: 2000 },
  { tool: densify, title: 'Köşe ekle, 100 000 parsel, 5 m aralık', scene: parcels, values: { input: ALL, method: 'interval', interval: 5, count: 1 }, budget: 2000 },
  { tool: pointsToPath, title: 'Noktalardan çizgi, 100 000 nokta, 1000 hat, sıralı', scene: points, values: { input: ALL, order: 'Sıra', group: 'Hat', close: false }, budget: 800 },
  {
    tool: bearingLine,
    title: 'Semt ve uzunluktan çizgi, 100 000 satır',
    scene: parcels,
    values: { sourceKind: 'file', file: bearingTable(), east: 'Doğu', north: 'Kuzey', bearing: 'Semt', length: 'Uzunluk', unit: 'grad' },
    budget: 800,
  },
  { tool: tablePoints, title: 'Tablodan nokta, 100 000 satır, projenin düzleminde', scene: parcels, values: { ...POINT_COLUMNS, file: pointTable(false), system: 'project' }, budget: 800 },
  { tool: tablePoints, title: "Tablodan nokta, 100 000 satır, WGS 84'ten projenin sistemine", scene: parcels, values: { ...POINT_COLUMNS, file: pointTable(true), system: '4326' }, budget: 900 },
];

it.runIf(!!process.env.CONVERT_BENCH)('measures Geometri dönüştürme against its budgets', async () => {
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
