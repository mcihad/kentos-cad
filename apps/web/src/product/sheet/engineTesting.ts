import type { InstanceOptions } from '../../contracts/generated/sheet/InstanceOptions';
import type { SheetBook } from '../../contracts/generated/sheet/SheetBook';
import { bookText, sheetEngineOf, type BookText, type SheetEngine } from './engine';
import * as pkg from './pkg/kentos_sheet_wasm';

/**
 * The sheet engine in the tests (Node): the built package started with
 * `initSync` from its bytes, as crates/wasm/sheet-wasm/tests/smoke.mjs does,
 * and the shared fixtures (fixtures/sheet/v1, read as files). Books made
 * from the engine's own system templates stand in for a project's, so a test
 * holds what the engine makes, not a hand-made copy. Tests only: nothing in
 * the app imports this (the app fetches the package lazily, engine.ts).
 */

type Fs = { readFileSync(u: URL, enc?: 'utf8'): string & Uint8Array<ArrayBuffer>; readdirSync(u: URL): string[] };
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): Fs } }).process.getBuiltinModule('node:fs');

let engine: SheetEngine | null = null;

export function testEngine(): SheetEngine {
  if (engine) return engine;
  pkg.initSync({ module: fs.readFileSync(new URL('./pkg/kentos_sheet_wasm_bg.wasm', import.meta.url)) });
  return (engine = sheetEngineOf(pkg));
}

const FIXTURES = new URL('../../../../../fixtures/sheet/v1/', import.meta.url);

/** A fixture file's text. */
export const fixtureText = (path: string): string => fs.readFileSync(new URL(path, FIXTURES), 'utf8');
/** A fixture file's JSON. */
export const fixture = <T = unknown>(path: string): T => JSON.parse(fixtureText(path)) as T;
/** The files of a fixture directory with this ending (JSON when none is given), by name. */
export const fixtureFiles = (dir: string, ending = '.json'): string[] =>
  fs
    .readdirSync(new URL(`${dir}/`, FIXTURES))
    .filter((f) => f.endsWith(ending))
    .sort();

/** An empty book, as the engine reads one. */
export function emptyBook(e: SheetEngine = testEngine()): BookText {
  const b: SheetBook = { schema: e.info.bookSchema, sheets: [], masters: [], assets: [], variables: [] };
  return e.readBook(JSON.stringify(b));
}

/**
 * A book with one sheet made from a system template (`sys:ifraz-paftasi`
 * by default), with ids of its own (`<prefix>-sheet`, `<prefix>-0` …): the
 * map at the given centre and scale.
 */
export function templateBook(id = 'sys:ifraz-paftasi', prefix = 't', options: Partial<InstanceOptions> = {}, e: SheetEngine = testEngine()): { book: BookText; sheet: string } {
  const t = e.systemTemplates().find((x) => x.meta.id === id);
  if (!t) throw new Error(`${id} yok`);
  const ids = {
    sheet: `${prefix}-sheet`,
    items: t.sheet.items.map((_, i) => `${prefix}-${i}`),
    masterItems: (t.master?.items ?? []).map((_, i) => `${prefix}-m${i}`),
    ...(t.master ? { master: `${prefix}-master` } : {}),
  };
  const inst = e.instantiateTemplate(t, ids, { center: { x: 486_780, y: 4_420_080 }, scale: 1000, values: [], ...options });
  const a = e.applyOps(emptyBook(e), [...(inst.master ? [{ op: 'addMaster' as const, master: inst.master }] : []), { op: 'addSheet', sheet: inst.sheet }, ...(inst.assets.length ? [{ op: 'addAssets' as const, assets: inst.assets }] : [])]);
  return { book: bookText(a.book), sheet: inst.sheet.id };
}
