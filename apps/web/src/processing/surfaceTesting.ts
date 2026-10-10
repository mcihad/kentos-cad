import type { AnalysisResult, MultidimRequest, MultidimResult, OpsResult, PointResult } from '../io/rasterAnalysisProtocol';
import type { RasterRunHost } from './rasterHost';

/**
 * What the raster cases need in a test (processing/cases.test.ts, fixtures/processing/v1/surface.json, interpolation.json,
 * raster-ops.json and raster-vector.json; docs/adr/0231–0234):
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
  sample(): string;
  style(band: number): string;
  notes(): string;
  crossPoint(): Uint32Array;
  crossObject(): Uint32Array;
  crossValues(): Float64Array;
  free(): void;
}
interface OpsApi {
  header(): Bytes;
  needs(): Float64Array;
  putBlock(i: number, bytes: Uint8Array): Bytes;
  step(): Bytes;
  done(): boolean;
  finish(): Bytes;
  head(): Bytes;
  grid(): Float64Array;
  bands(): number;
  sample(): string;
  style(): string;
  notes(): string;
  zones(): Float64Array;
  histogram(): string;
  roc(): string;
  featureKind(): string;
  featureValues(): Float64Array;
  featureTexts(): string[];
  featureTags(): Uint8Array;
  featureRings(): Uint32Array;
  featureSizes(): Uint32Array;
  featureXy(): Float64Array;
  featureFields(): string[];
  featureNumbers(): Float64Array;
  free(): void;
}
type HeaderOpening = { need(): Float64Array; put(offset: number, bytes: Uint8Array): void; analysis(spec: string): AnalysisApi; free(): void };
interface MultidimApi {
  needs(): Float64Array;
  putBlock(i: number, bytes: Uint8Array): Bytes;
  step(): void;
  done(): boolean;
  finish(): string;
  file(): Bytes;
  output(): string;
  free(): void;
}
type OpsOpeningApi = { addTiff(o: HeaderOpening): void; addCube(o: HeaderOpening): void; start(spec: string, shapes: string): OpsApi; free(): void };
interface RasterWasm {
  initSync(o: { module: BufferSource }): unknown;
  AnalysisOpening: new (size: number) => HeaderOpening;
  CubeOpening: new (size: number, part: string) => HeaderOpening;
  PointAnalysis: new (objects: string, values: string, spec: string, lines: boolean) => PointApi;
  OpsOpening: new () => OpsOpeningApi;
  MultidimAnalysis: {
    profile(o: OpsOpeningApi, affine: Float64Array, nodata: number | undefined, lines: string, step: number, band: number, axes: string): MultidimApi;
    bandSeries(o: OpsOpeningApi, affine: Float64Array, nodata: number | undefined, points: string): MultidimApi;
    timeSeries(c: HeaderOpening, affine: Float64Array, nodata: number | undefined, points: string): MultidimApi;
    meshCalc(c: HeaderOpening, spec: string): MultidimApi;
  };
  opsReads(spec: string): Uint32Array;
}

/** A header read in the pieces an opening asks for (a TIFF's, or a NetCDF's and its slice's). */
function opened<T extends HeaderOpening>(o: T, bytes: Uint8Array): T {
  for (;;) {
    const need = o.need();
    if (!need.length) return o;
    o.put(need[0], bytes.slice(need[0], need[0] + need[1]));
  }
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

/** A job run here as the analysis worker runs it (io/rasterAnalysisWorker.ts), over a TIFF's bytes or a NetCDF's slice `part`. */
export async function analyzeHere(bytes: Uint8Array, spec: string, part: string | null = null): Promise<AnalysisResult> {
  const { raster: m } = await modules();
  const o = opened(part ? new m.CubeOpening(bytes.length, part) : new m.AnalysisOpening(bytes.length), bytes);
  let a: AnalysisApi;
  try {
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
      sample: a.sample(),
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

/**
 * A raster operation (docs/adr/0233) run here as the analysis worker runs it, over TIFFs' bytes in the run's order (a
 * text: why that input cannot be read): only the inputs the run reads are opened.
 */
export async function analyzeOpsHere(sources: (Uint8Array | string)[], spec: string, shapes: string, parts: (string | null)[] = []): Promise<OpsResult> {
  const { raster: m } = await modules();
  const reads = Array.from(m.opsReads(spec));
  const inputs = reads.map((k) => {
    const s = sources[k];
    if (typeof s === 'string') throw new Error(s);
    return s;
  });
  const o = new m.OpsOpening();
  let a: OpsApi;
  try {
    for (const [k, bytes] of inputs.entries()) {
      const part = parts[reads[k]];
      const t = opened(part ? new m.CubeOpening(bytes.length, part) : new m.AnalysisOpening(bytes.length), bytes);
      try {
        if (part) o.addCube(t);
        else o.addTiff(t);
      } finally {
        t.free();
      }
    }
    a = o.start(spec, shapes);
  } finally {
    o.free();
  }
  try {
    const parts: Uint8Array[] = [a.header()];
    while (!a.done()) {
      const needs = a.needs();
      for (let i = 0; 3 * i < needs.length; i++) {
        const bytes = inputs[needs[3 * i]];
        a.putBlock(i, bytes.slice(needs[3 * i + 1], needs[3 * i + 1] + needs[3 * i + 2]));
      }
      parts.push(a.step());
    }
    parts.push(a.finish());
    const result: OpsResult = { reads, grid: Array.from(a.grid()), bands: a.bands(), sample: a.sample(), style: a.style(), notes: a.notes() };
    if (a.bands() > 0) {
      const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
      let at = 0;
      for (const p of parts) (out.set(p, at), (at += p.length));
      out.set(a.head(), 0);
      return { ...result, bytes: out };
    }
    const kind = a.featureKind();
    if (kind) {
      return {
        ...result,
        features: {
          kind: kind as 'polygons' | 'lines' | 'points',
          values: a.featureValues(),
          texts: a.featureTexts(),
          tags: a.featureTags(),
          rings: a.featureRings(),
          sizes: a.featureSizes(),
          xy: a.featureXy(),
          fields: a.featureFields(),
          numbers: a.featureNumbers(),
        },
      };
    }
    return { ...result, zones: a.zones(), histogram: a.histogram(), roc: a.roc() };
  } finally {
    a.free();
  }
}

/** A Çok boyutlu veri tool (docs/adr/0243 §8–§10) run here as the analysis worker runs it. */
export async function analyzeMultidimHere(job: Omit<MultidimRequest, 'type' | 'blob'>, bytes: Uint8Array): Promise<MultidimResult> {
  const { raster: m } = await modules();
  const affine = Float64Array.from(job.affine);
  const nodata = job.nodata ?? undefined;
  let a: MultidimApi;
  if (job.kind === 'timeSeries' || job.kind === 'meshCalc') {
    if (!job.part) throw new Error('Bu raster NetCDF veri seti göstermiyor.');
    const c = opened(new m.CubeOpening(bytes.length, job.part), bytes);
    try {
      a = job.kind === 'timeSeries' ? m.MultidimAnalysis.timeSeries(c, affine, nodata, job.objects) : m.MultidimAnalysis.meshCalc(c, job.spec ?? '{}');
    } finally {
      c.free();
    }
  } else {
    const o = new m.OpsOpening();
    try {
      const t = opened(job.part ? new m.CubeOpening(bytes.length, job.part) : new m.AnalysisOpening(bytes.length), bytes);
      try {
        if (job.part) o.addCube(t);
        else o.addTiff(t);
      } finally {
        t.free();
      }
      a =
        job.kind === 'profile'
          ? m.MultidimAnalysis.profile(o, affine, nodata, job.objects, job.step ?? 1, job.band ?? 1, job.axes ?? 'Y,X')
          : m.MultidimAnalysis.bandSeries(o, affine, nodata, job.objects);
    } finally {
      o.free();
    }
  }
  try {
    while (!a.done()) {
      const needs = a.needs();
      for (let i = 0; 2 * i < needs.length; i++) a.putBlock(i, bytes.slice(needs[2 * i], needs[2 * i] + needs[2 * i + 1]));
      a.step();
    }
    const result = a.finish();
    const file = a.file();
    return { result, file: file.length ? file : undefined, output: a.output() || undefined };
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
    analyze: async (blob, spec, _watch, part) => analyzeHere(new Uint8Array(await blob.arrayBuffer()), spec, part ?? null),
    analyzePoints: (objects, values, spec, lines) => analyzePointsHere(objects, values, spec, lines),
    analyzeOps: async (sources, spec, shapes, _watch, parts) =>
      analyzeOpsHere(await Promise.all(sources.map(async (b) => (typeof b === 'string' ? b : new Uint8Array(await b.arrayBuffer())))), spec, shapes, parts ?? []),
    analyzeMultidim: async ({ blob, ...job }) => analyzeMultidimHere(job, new Uint8Array(await blob.arrayBuffer())),
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
