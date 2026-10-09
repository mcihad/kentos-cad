import type { AnalysisResult, PointResult } from '../io/rasterAnalysisProtocol';
import type { RasterRunHost } from './rasterHost';

/**
 * What the surface and interpolation cases need in a test (processing/cases.test.ts, fixtures/processing/v1/surface.json
 * and interpolation.json; docs/adr/0231, 0232):
 * the raster analysis module run in process (crates/wasm/raster-wasm, `pnpm wasm`), a raster host whose rasters are
 * the cases' files and which keeps what a run writes, and the formats module reading a written GeoTIFF's level 0.
 * Test code only.
 */

type Bytes = Uint8Array<ArrayBuffer>;
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL): Bytes } } }).process.getBuiltinModule('node:fs');

interface AnalysisApi {
  header(): Bytes;
  needs(): Float64Array;
  putBlock(i: number, bytes: Uint8Array): Bytes;
  step(): Bytes;
  done(): boolean;
  finish(): Bytes;
  head(): Bytes;
  bands(): number;
  sample(): string;
  style(): string;
  lineValues(): Float64Array;
  lineMain(): Uint8Array;
  lineSizes(): Uint32Array;
  linePoints(): Float64Array;
  lineTexts(): string[];
  free(): void;
}
interface PointApi {
  header(): Bytes;
  step(): Bytes;
  done(): boolean;
  finish(): Bytes;
  head(): Bytes;
  grid(): Float64Array;
  bands(): number;
  style(band: number): string;
  notes(): string;
  crossPoint(): Uint32Array;
  crossObject(): Uint32Array;
  crossValues(): Float64Array;
  free(): void;
}
interface RasterWasm {
  initSync(o: { module: BufferSource }): unknown;
  AnalysisOpening: new (size: number) => { need(): Float64Array; put(offset: number, bytes: Uint8Array): void; analysis(spec: string): AnalysisApi; free(): void };
  PointAnalysis: new (objects: string, values: string, spec: string, lines: boolean) => PointApi;
}
interface FormatsWasm {
  initSync(o: { module: BufferSource }): unknown;
  RasterOpening: new (size: number) => { need(): Float64Array; put(offset: number, bytes: Uint8Array): void; open(world: string | undefined, budget: number): RasterFileApi; free(): void };
}
interface RasterFileApi {
  info(): string;
  needs(level: number, x: number, y: number, w: number, h: number): Float64Array;
  putBlock(i: number, bytes: Uint8Array): Bytes;
  sample(i: number, j: number): Float64Array | undefined;
  free(): void;
}

const rasterGlue = import.meta.glob<RasterWasm>('../io/raster/pkg/kentos_raster_wasm.js');
const formatsGlue = import.meta.glob<FormatsWasm>('../io/pkg/kentos_formats_wasm.js');
/** Whether both modules are built (the cases are skipped otherwise). */
export const surfaceModulesBuilt = Object.keys(rasterGlue).length > 0 && Object.keys(formatsGlue).length > 0;

let raster: RasterWasm | undefined;
let formats: FormatsWasm | undefined;

async function modules(): Promise<{ raster: RasterWasm; formats: FormatsWasm }> {
  if (!raster) {
    raster = await Object.values(rasterGlue)[0]();
    raster.initSync({ module: fs.readFileSync(new URL('../io/raster/pkg/kentos_raster_wasm_bg.wasm', import.meta.url)) });
  }
  if (!formats) {
    formats = await Object.values(formatsGlue)[0]();
    formats.initSync({ module: fs.readFileSync(new URL('../io/pkg/kentos_formats_wasm_bg.wasm', import.meta.url)) });
  }
  return { raster, formats };
}

/** A job run here as the analysis worker runs it (io/rasterAnalysisWorker.ts), over a TIFF's bytes. */
export async function analyzeHere(bytes: Uint8Array, spec: string): Promise<AnalysisResult> {
  const { raster: m } = await modules();
  const o = new m.AnalysisOpening(bytes.length);
  let a: AnalysisApi;
  try {
    for (;;) {
      const need = o.need();
      if (!need.length) break;
      o.put(need[0], bytes.slice(need[0], need[0] + need[1]));
    }
    a = o.analysis(spec);
  } finally {
    o.free();
  }
  try {
    const parts: Uint8Array[] = [a.header()];
    while (!a.done()) {
      const needs = a.needs();
      for (let i = 0; 2 * i < needs.length; i++) a.putBlock(i, bytes.subarray(needs[2 * i], needs[2 * i] + needs[2 * i + 1]));
      parts.push(a.step());
    }
    parts.push(a.finish());
    if (a.bands() > 0) {
      const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
      let at = 0;
      for (const p of parts) (out.set(p, at), (at += p.length));
      out.set(a.head(), 0);
      return { raster: { bytes: out, bands: a.bands(), sample: a.sample(), style: a.style() } };
    }
    return { lines: { values: a.lineValues(), main: a.lineMain(), sizes: a.lineSizes(), points: a.linePoints(), texts: a.lineTexts() } };
  } finally {
    a.free();
  }
}

/** A raster from points or lines (docs/adr/0232) made here as the analysis worker makes it. */
export async function analyzePointsHere(objects: string, values: string, spec: string, lines: boolean): Promise<PointResult> {
  const { raster: m } = await modules();
  const a = new m.PointAnalysis(objects, values, spec, lines);
  try {
    const parts: Uint8Array[] = [a.header()];
    while (!a.done()) parts.push(a.step());
    parts.push(a.finish());
    const bytes = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
    let at = 0;
    for (const p of parts) (bytes.set(p, at), (at += p.length));
    bytes.set(a.head(), 0);
    return {
      bytes,
      grid: Array.from(a.grid()),
      bands: a.bands(),
      styles: [a.style(1), a.style(2)],
      notes: a.notes(),
      crossPoint: a.crossPoint(),
      crossObject: a.crossObject(),
      crossValues: a.crossValues(),
    };
  } finally {
    a.free();
  }
}

/** A host whose linked rasters are `rasters` (a raster's file name → its bytes), keeping what a run writes in `written`. */
export function fixtureRasterHost(rasters: ReadonlyMap<string, Uint8Array>): { host: RasterRunHost; written: Map<string, Uint8Array> } {
  const written = new Map<string, Uint8Array>();
  const host: RasterRunHost = {
    source: (r) => {
      const bytes = r.file !== undefined ? rasters.get(r.file) : undefined;
      return bytes ? new Blob([bytes as Bytes]) : { refused: `${r.file}: not among the cases' rasters` };
    },
    analyze: async (blob, spec) => analyzeHere(new Uint8Array(await blob.arrayBuffer()), spec),
    analyzePoints: (objects, values, spec, lines) => analyzePointsHere(objects, values, spec, lines),
    keep: async (bytes, name) => {
      written.set(name, bytes);
      return { file: name, note: '' };
    },
  };
  return { host, written };
}

/** A written GeoTIFF's level 0 whole: its samples, bands interleaved (the formats module's reader). */
export async function level0(bytes: Uint8Array): Promise<number[]> {
  const { formats: m } = await modules();
  const o = new m.RasterOpening(bytes.length);
  let file: RasterFileApi;
  try {
    for (;;) {
      const need = o.need();
      if (!need.length) break;
      o.put(need[0], bytes.slice(need[0], need[0] + need[1]));
    }
    file = o.open(undefined, 64 * 1024 * 1024);
  } finally {
    o.free();
  }
  try {
    const { width: w, height: h } = JSON.parse(file.info()) as { width: number; height: number };
    const needs = file.needs(0, 0, 0, w, h);
    for (let i = 0; 3 * i < needs.length; i++) file.putBlock(i, bytes.subarray(needs[3 * i + 1], needs[3 * i + 1] + needs[3 * i + 2]));
    const out: number[] = [];
    for (let j = 0; j < h; j++) for (let i = 0; i < w; i++) out.push(...(file.sample(i, j) ?? []));
    return out;
  } finally {
    file.free();
  }
}

/** The units in the last place between two 32-bit values (NaN only with NaN). */
export function ulps(a: number, b: number): number {
  if (Number.isNaN(a) || Number.isNaN(b)) return Number.isNaN(a) && Number.isNaN(b) ? 0 : Infinity;
  const key = (v: number) => {
    const i = new Int32Array(new Float32Array([v]).buffer)[0];
    return i < 0 ? -2147483648 - i : i;
  };
  return Math.abs(key(a) - key(b));
}

/** The contour reference's lines (fixtures/contours/v1/cases.json, case `k`) as the tool's polylines on `layer`. */
export function contourObjects(cases: { cases: { texts: Record<string, string>; lines: { k: number; value: number; index: boolean; pts: [number, number][] }[] }[] }, k: number, layer: string) {
  const c = cases.cases[k];
  return c.lines.map((l) => ({
    kind: 'polyline',
    layerId: layer,
    attrs: { Kot: c.texts[String(l.k)], Tür: l.index ? 'Ana' : 'Ara' },
    pts: l.pts.map(([x, y]) => ({ x, y })),
    zs: l.pts.map(() => l.value),
    ...(l.index ? { lineWeight: 0.35 } : {}),
  }));
}
