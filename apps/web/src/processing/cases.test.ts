import { describe, expect, it } from 'vitest';
import { CadDocument } from '../model/document';
import type { Entity } from '../model/entities';
import { LayerStore } from '../model/layers';
import { readSnapshot } from '../model/snapshot';
import { BUILTIN_TOOLS } from './builtin';
import { BUILTIN_MODELS } from './builtin/models';
import { clientExecutor } from './job';
import { modelAsTool, runModel } from './modelRunner';
import { defaultValues } from './parameters';
import { ProcessingRunner, type RunOutcome, type TargetChoice } from './runner';
import { handleJob } from './worker/handleJob';
import type { WorkerRequest } from './worker/protocol';
import { workerExecutor, type WorkerLike } from './worker/workerExecutor';
import type { FileValue, ProcessingTool } from './types';
import type { TableFileRead } from '../contracts/generated/TableFileRead';
import { geoMeasure } from '../model/ops/geoprocess';

/**
 * The shared processing cases (fixtures/processing/v1, format in
 * fixtures/processing/README.md): each case runs a built-in tool or model on a drawing through
 * ProcessingRunner, in the page and again through the worker's path
 * (handleJob), and what the run did is compared with what the case says.
 * The desktop's kentos-processing plays the same files: cases.json,
 * queries.json (docs/adr/0200), whose file values name files beside it,
 * read through the formats module as the dialog reads a chosen file, and
 * geometry.json (docs/adr/0201), whose new objects are measured
 * (`addedShapes`).
 */

const files = import.meta.glob<string>('../../../../fixtures/processing/v1/*', { query: '?raw', import: 'default', eager: true });
const file = (name: string): string => {
  const text = files[`../../../../fixtures/processing/v1/${name}`];
  if (text === undefined) throw new Error(`fixtures/processing/v1/${name} yok`);
  return text;
};

type Json = Record<string, unknown>;
interface Case {
  id: string;
  title: string;
  document: string;
  selection?: number[];
  view?: [number, number, number, number];
  run: { tool: string } | { model: string };
  values?: Json;
  expect: Json;
}
interface CaseFile {
  format: string;
  version: number;
  tolerance: number;
  documents: Record<string, { defaults: Json; tools: Record<string, Json> }>;
  /** A file parameter's value names one of these files (the value → the file beside the cases). */
  files?: Record<string, string>;
  /** How far an added object's area or length may be from the reference's (`addedShapes`). */
  measureTolerance?: number;
  cases: Case[];
}

const CASES = JSON.parse(file('cases.json')) as CaseFile;
const QUERIES = JSON.parse(file('queries.json')) as CaseFile;
const GEOMETRY = JSON.parse(file('geometry.json')) as CaseFile;

interface Formats {
  initSync(o: { module: BufferSource }): unknown;
  readTableFile(bytes: Uint8Array): Uint8Array;
}
const glue = import.meta.glob<Formats>('../io/pkg/kentos_formats_wasm.js');
const loader = Object.values(glue)[0];
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL): Uint8Array<ArrayBuffer> } } }).process.getBuiltinModule('node:fs');
let formats: Formats | undefined;

/** A case's file value as the dialog makes it: the file's name and its first sheet's rows (Tablo ekle's reader). */
async function fileValue(cases: CaseFile, name: string): Promise<FileValue> {
  const path = cases.files?.[name];
  if (!path) throw new Error(`dosya yok: ${name}`);
  if (!formats) {
    const w = await loader!();
    w.initSync({ module: fs.readFileSync(new URL('../io/pkg/kentos_formats_wasm_bg.wasm', import.meta.url)) });
    formats = w;
  }
  const read = JSON.parse(new TextDecoder().decode(formats.readTableFile(fs.readFileSync(new URL(`../../../../fixtures/processing/v1/${path}`, import.meta.url))))) as TableFileRead;
  return { name, rows: read.sheets[0]?.rows ?? [] };
}

/** The case's values with each file parameter's name turned into the file's value. */
async function caseValues(cases: CaseFile, tool: ProcessingTool, values: Json | undefined): Promise<Json> {
  const out: Json = { ...values };
  for (const p of tool.parameters) if (p.type === 'file' && typeof out[p.name] === 'string') out[p.name] = await fileValue(cases, out[p.name] as string);
  return out;
}
const TOOLS = new Map(BUILTIN_TOOLS.map((t) => [t.id, t]));
const MODELS = new Map(BUILTIN_MODELS.map((m) => [m.id, m]));
const lookup = (id: string) => TOOLS.get(id);

/** A Worker stand-in: messages are structured-cloned both ways, as the browser does. */
function fakeWorker(): WorkerLike {
  const w: WorkerLike & { terminated: boolean } = {
    onmessage: null,
    onerror: null,
    terminated: false,
    postMessage(m) {
      const copy = structuredClone(m) as WorkerRequest;
      queueMicrotask(() => void handleJob(copy, (r) => !w.terminated && w.onmessage?.({ data: structuredClone(r) }), lookup));
    },
    terminate() {
      w.terminated = true;
    },
  };
  return w;
}

function load(name: string): CadDocument {
  const read = readSnapshot(file(name));
  if (!read.ok) throw new Error(`${name}: ${read.error}`);
  const doc = new CadDocument({ name: read.content.name, layers: new LayerStore([], ''), origin: read.content.origin });
  doc.replaceWith(read.content);
  return doc;
}

/** An object as the cases write it: every field but the ids. */
function plain(e: Entity): Json {
  const { id: _id, uid: _uid, ...rest } = e as Entity & { uid?: string };
  return JSON.parse(JSON.stringify(rest)) as Json;
}

/** The drawing's objects (by id) and layer tree, to compare before and after. */
function state(doc: CadDocument): { objects: Map<number, string>; layers: string } {
  return { objects: new Map([...doc.all()].map((e) => [e.id, JSON.stringify(plain(e))])), layers: JSON.stringify(doc.layers.tree) };
}

/** Whether two values are equal, coordinates (x, y) within `tol`. */
function close(a: unknown, b: unknown, tol: number): boolean {
  if (typeof a === 'number' && typeof b === 'number') return Math.abs(a - b) <= tol;
  if (Array.isArray(a) && Array.isArray(b)) return a.length === b.length && a.every((v, i) => close(v, b[i], tol));
  if (a && b && typeof a === 'object' && typeof b === 'object') {
    const ka = Object.keys(a).sort();
    const kb = Object.keys(b).sort();
    return JSON.stringify(ka) === JSON.stringify(kb) && ka.every((k) => close((a as Json)[k], (b as Json)[k], tol));
  }
  return a === b;
}

/** Numbers other than coordinates are exact: only objects' geometry fields get the tolerance. */
const GEOMETRY_FIELDS = new Set(['p', 'a', 'b', 'c', 'pts', 'holes']);
function sameObject(have: Json, want: Json, tol: number): boolean {
  const keys = (o: Json) => JSON.stringify(Object.keys(o).sort());
  if (keys(have) !== keys(want)) return false;
  return Object.keys(want).every((k) => (GEOMETRY_FIELDS.has(k) ? close(have[k], want[k], tol) : JSON.stringify(have[k]) === JSON.stringify(want[k])));
}

/** The fields a geometry tool's new object may have: its kind, layer and attributes, and its geometry's. */
const SHAPE_FIELDS = new Set(['kind', 'layerId', 'attrs', 'pts', 'bulges', 'holes', 'parts', 'p', 'a', 'b', 'c', 'r']);

/** An object's vertices in order: an area's ring then its holes, a path's, a point's and its points'. */
function vertices(e: Json): unknown[] {
  if (e.kind === 'point') return [e.p, ...((e.parts as Json[] | undefined) ?? []).map((q) => q.p)];
  return [...(e.pts as unknown[]), ...((e.holes as Json[] | undefined) ?? []).flatMap((h) => h.pts as unknown[])];
}

/**
 * A geometry tool's new object against the case (`addedShapes`, docs/adr/0201): its kind, layer and attributes
 * exactly (in any order), no other fields, and its measures (parts, holes, area, length, points; the core's
 * `geoMeasure`) and vertices within the tolerances.
 */
function sameShape(id: string, i: number, have: Json, want: Json, tol: number, measureTol: number): void {
  const at = `${id}: addedShapes[${i}] ${JSON.stringify(have)}`;
  expect(Object.keys(have).filter((k) => !SHAPE_FIELDS.has(k)), at).toEqual([]);
  expect(have.kind, at).toBe(want.kind);
  expect(have.layerId, at).toBe(want.layerId);
  expect(have.attrs, at).toEqual(want.attrs);
  const m = geoMeasure([have as never])[0];
  expect([m.parts, m.holes], `${at}: parça, delik`).toEqual([want.parts, want.holes]);
  for (const k of ['area', 'length'] as const) {
    const w = want[k];
    if (typeof w === 'number') expect(Math.abs((m[k] ?? NaN) - w), `${at}: ${k} ${m[k]}, beklenen ${w}`).toBeLessThanOrEqual(measureTol);
  }
  if (want.points) expect(close(m.points, want.points, tol), `${at}: noktalar`).toBe(true);
  if (want.vertices) expect(close(vertices(have), want.vertices, tol), `${at}: köşeler`).toBe(true);
}

interface Seen {
  outcome: RunOutcome;
  log: { level: string; text: string }[];
  doc: CadDocument;
  before: ReturnType<typeof state>;
  selection: number[];
}

async function play(cases: CaseFile, c: Case, target: TargetChoice): Promise<Seen> {
  const doc = load(c.document);
  let selection = [...(c.selection ?? [])];
  const view = c.view ? { minX: c.view[0], minY: c.view[1], maxX: c.view[2], maxY: c.view[3] } : null;
  const runner = new ProcessingRunner(
    { doc, selectedIds: () => selection, visibleBounds: () => view, select: (ids) => (selection = [...ids]) },
    [clientExecutor, workerExecutor(fakeWorker, new Set(TOOLS.keys()))],
  );
  const before = state(doc);
  const log: Seen['log'] = [];
  const opts = { log: (level: string, text: string) => log.push({ level, text }), target };
  let outcome: RunOutcome;
  if ('tool' in c.run) {
    const tool = TOOLS.get(c.run.tool);
    if (!tool) throw new Error(`${c.id}: araç yok: ${c.run.tool}`);
    outcome = await runner.run(tool, { ...defaultValues(tool, runner.defaults()), ...(await caseValues(cases, tool, c.values)) }, opts);
  } else {
    const model = MODELS.get(c.run.model);
    if (!model) throw new Error(`${c.id}: model yok: ${c.run.model}`);
    const asTool = modelAsTool(model, lookup);
    outcome = await runModel(model, { ...defaultValues(asTool, runner.defaults()), ...c.values }, runner, lookup, opts);
  }
  return { outcome, log, doc, before, selection };
}

/** What a run did, in the cases' terms. */
function observed(s: Seen): Json {
  const { outcome, doc, before } = s;
  const out: Json = { status: outcome.status };
  if (outcome.status === 'invalid') return { ...out, issues: outcome.issues };
  if (outcome.status !== 'ok') return { ...out, message: outcome.message };
  const was = JSON.parse(before.layers) as { id: string }[];
  const known = new Set<string>();
  const walk = (nodes: readonly { id: string; children?: unknown[] }[]) => nodes.forEach((n) => (known.add(n.id), walk((n.children ?? []) as never)));
  walk(was);
  const layers = [...doc.layers.leaves()].filter((l) => !known.has(l.id)).map((l) => ({ id: l.id, name: l.name, style: l.style }));
  const added: Json[] = [];
  const updated: Json[] = [];
  for (const e of doc.all()) {
    const old = before.objects.get(e.id);
    if (old === undefined) added.push(plain(e));
    else if (old !== JSON.stringify(plain(e))) updated.push({ id: e.id, attrs: e.attrs, ...(e.label !== undefined ? { label: e.label } : {}) });
  }
  const removed = [...before.objects.keys()].filter((id) => !doc.get(id));
  return {
    ...out,
    summary: outcome.record.summary,
    log: s.log,
    layers: JSON.parse(JSON.stringify(layers)),
    added,
    updated,
    removed,
    selection: s.selection,
    outputs: outcome.result.outputs ?? {},
  };
}

function check(c: Case, s: Seen, tol: number, measureTol = tol): void {
  const got = observed(s);
  const want = c.expect;
  const where = `${c.id}: ${JSON.stringify(got, null, 1)}`;
  expect(got.status, where).toBe(want.status);
  if (want.status === 'invalid') return void expect(got.issues, where).toEqual(want.issues);
  if (want.status !== 'ok') return void expect(got.message, where).toBe(want.message);
  expect(got.summary, where).toBe(want.summary);
  expect(got.log, where).toEqual(want.log ?? []);
  expect(got.layers, where).toEqual(want.layers ?? []);
  const added = got.added as Json[];
  if (want.addedShapes) {
    const shapes = want.addedShapes as Json[];
    expect(added.length, where).toBe(shapes.length);
    shapes.forEach((w, i) => sameShape(c.id, i, added[i], w, tol, measureTol));
  } else {
    const wantAdded = (want.added ?? []) as Json[];
    expect(added.length, where).toBe(wantAdded.length);
    wantAdded.forEach((w, i) => expect(sameObject(added[i], w, tol), `${c.id}: added[${i}] ${JSON.stringify(added[i])}`).toBe(true));
  }
  expect(got.updated, where).toEqual(want.updated ?? []);
  expect(got.removed, where).toEqual(want.removed ?? []);
  if (want.selection) expect(got.selection, where).toEqual(want.selection);
  const outputs = got.outputs as Json;
  for (const [k, v] of Object.entries((want.outputs ?? {}) as Json)) expect(outputs[k], `${c.id}: outputs.${k}`).toEqual(v);
  // One undo step takes the whole run back, and redo brings it again; a run that edits nothing leaves no step.
  const doc = s.doc;
  if (want.undo === null) return void expect(doc.canUndo.value, `${c.id}: geri alınacak adım olmamalı`).toBe(false);
  const after = state(doc);
  expect(doc.undo(), `${c.id}: geri alma adımı`).toBe(want.undo);
  expect(state(doc), `${c.id}: geri alınınca çizim eski hâline dönmeli`).toEqual(s.before);
  expect(doc.canUndo.value, `${c.id}: tek adım`).toBe(false);
  doc.redo();
  expect(state(doc), `${c.id}: yinelenince sonuç geri gelmeli`).toEqual(after);
}

describe('processing cases (fixtures/processing/v1)', () => {
  it('is a v1 case file', () => {
    expect([CASES.format, CASES.version]).toEqual(['kentos.processing-cases', 1]);
  });

  // Each drawing's defaults (DefaultsContext) and each tool's default values on it, as the desktop must read them.
  for (const [name, d] of [...Object.entries(CASES.documents), ...Object.entries(GEOMETRY.documents)]) {
    it(`${name}: the defaults the tools take from the drawing`, () => {
      const runner = new ProcessingRunner({ doc: load(name), selectedIds: () => [], visibleBounds: () => null });
      expect(runner.defaults()).toEqual(d.defaults);
      for (const [id, values] of Object.entries(d.tools)) {
        const tool = TOOLS.get(id) ?? (MODELS.has(id) ? modelAsTool(MODELS.get(id)!, lookup) : undefined);
        expect(tool, id).toBeDefined();
        expect(JSON.parse(JSON.stringify(defaultValues(tool!, runner.defaults()))), id).toEqual(values);
      }
    });
  }

  for (const c of CASES.cases) {
    it(`${c.id}: ${c.title}`, async () => {
      check(c, await play(CASES, c, 'client'), CASES.tolerance);
      // The same run through the worker's path gives the same result.
      check(c, await play(CASES, c, 'worker'), CASES.tolerance);
    });
  }
});

describe.skipIf(!loader)('query cases (fixtures/processing/v1/queries.json, docs/adr/0200)', () => {
  it('is a v1 case file', () => {
    expect([QUERIES.format, QUERIES.version]).toEqual(['kentos.processing-cases', 1]);
  });

  for (const c of QUERIES.cases) {
    it(`${c.id}: ${c.title}`, async () => {
      check(c, await play(QUERIES, c, 'client'), QUERIES.tolerance);
      check(c, await play(QUERIES, c, 'worker'), QUERIES.tolerance);
    });
  }
});

describe('geometry cases (fixtures/processing/v1/geometry.json, docs/adr/0201)', () => {
  it('is a v1 case file', () => {
    expect([GEOMETRY.format, GEOMETRY.version]).toEqual(['kentos.processing-cases', 1]);
  });

  for (const c of GEOMETRY.cases) {
    it(`${c.id}: ${c.title}`, async () => {
      check(c, await play(GEOMETRY, c, 'client'), GEOMETRY.tolerance, GEOMETRY.measureTolerance);
      check(c, await play(GEOMETRY, c, 'worker'), GEOMETRY.tolerance, GEOMETRY.measureTolerance);
    });
  }
});
