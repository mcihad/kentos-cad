import {
  AdjoinWork,
  angleDeg as wasmAngleDeg,
  arrayObjects as wasmArrayObjects,
  bearingGrad as wasmBearingGrad,
  callOp,
  cornerTexts as wasmCornerTexts,
  crsForgetGrid as wasmCrsForgetGrid,
  crsLoadGrid as wasmCrsLoadGrid,
  dist as wasmDist,
  distToSegment as wasmDistToSegment,
  exprEvaluate as wasmExprEvaluate,
  exprEvaluateIn as wasmExprEvaluateIn,
  ExprWorld as WasmExprWorld,
  FaceIndex,
  GeometryStore,
  StyleProgram,
  hatchLinesXY as wasmHatchLinesXY,
  initSync,
  NetworkGraph,
  offsetPathXY as wasmOffsetPathXY,
  opId,
  rasterLevelCount as wasmRasterLevelCount,
  rasterTiles as wasmRasterTiles,
  rasterTilesAt as wasmRasterTilesAt,
  rendererProblem as wasmRendererProblem,
  scratch as wasmScratch,
  scratchCentroid as wasmScratchCentroid,
  scratchPathLength as wasmScratchPathLength,
  scratchPointInPolygon as wasmScratchPointInPolygon,
  scratchSignedArea as wasmScratchSignedArea,
  TraceGraph,
  timeLayer as wasmTimeLayer,
  timeLayerMask as wasmTimeLayerMask,
  transformObjects as wasmTransformObjects,
  triangulateMany as wasmTriangulateMany,
} from './pkg/kentos_geometry_wasm.js';
import wasmUrl from './pkg/kentos_geometry_wasm_bg.wasm?url';

/**
 * The Rust geometry core in this page (crates/wasm/geometry-wasm, docs/adr/0008). It is
 * compiled once before the app starts (`initCore`), after which every call
 * is synchronous: snapping, drawing and tools run inside pointer events and
 * animation frames. The processing worker gets the compiled module with its
 * first job (`initCoreFrom`), so the file is not fetched twice.
 *
 * Calls go through `op(name)`: the arguments cross as a JSON array, the
 * result comes back as JSON with NaN and ±∞ kept as "#NaN" / "#Inf" /
 * "#-Inf" (api/json.rs). Hot paths have typed entry points instead.
 */

let compiled: WebAssembly.Module | null = null;
let faulted = false;
let onFault: ((err: Error) => void) | null = null;

/** Fetches, compiles and starts the core (the page, before the app mounts). */
export async function initCore(): Promise<void> {
  if (compiled) return;
  const res = await fetch(wasmUrl);
  if (!res.ok) throw new Error(`Geometri çekirdeği indirilemedi (${res.status}).`);
  // compileStreaming needs the application/wasm type; a server that sends another falls back to bytes.
  const module = await WebAssembly.compileStreaming(res.clone()).catch(async () => WebAssembly.compile(await res.arrayBuffer()));
  initCoreFrom(module);
}

/** The core's memory: the small ring measures write their points straight into it. */
let memory: WebAssembly.Memory | null = null;

/** Starts the core from a compiled module (the worker, the tests). */
export function initCoreFrom(module: WebAssembly.Module): void {
  if (compiled) return;
  memory = initSync({ module }).memory;
  compiled = module;
}

/** The compiled module, for a worker to start its own copy of the core. */
export function coreModule(): WebAssembly.Module | null {
  return compiled;
}

/**
 * Called once if the core stops with a trap (a bug in the core: it never
 * fails on user data by design). The page cannot start a second copy, so
 * the app tells the user to save and reload.
 */
export function onCoreFault(handler: (err: Error) => void): void {
  onFault = handler;
}

/** A trap stops the core for good: the app hears of it once. */
function fault(err: unknown): void {
  if (err instanceof WebAssembly.RuntimeError && !faulted) {
    faulted = true;
    onFault?.(err);
  }
}

/** Runs a typed entry point (a hot path), reporting a trap as `op` does. */
function typed<T>(run: () => T): T {
  if (!compiled) throw new Error('Geometri çekirdeği henüz başlatılmadı.');
  try {
    return run();
  } catch (err) {
    fault(err);
    throw err;
  }
}

const SPECIAL = new Map<string, number>([
  ['#NaN', NaN],
  ['#Inf', Infinity],
  ['#-Inf', -Infinity],
]);
const revive = (_key: string, v: unknown) => (typeof v === 'string' && v.charCodeAt(0) === 35 && SPECIAL.has(v) ? SPECIAL.get(v) : v);

/**
 * A core result read back: the special numbers are the strings "#NaN",
 * "#Inf" and "#-Inf". The reviver (a call per value, several times slower)
 * runs only when one of them is there: colours such as "#AA3300" start with
 * "#" too.
 */
export function readResult(text: string): unknown {
  return text.includes('"#NaN"') || text.includes('"#Inf"') || text.includes('"#-Inf"') ? JSON.parse(text, revive) : JSON.parse(text);
}

const special = (_key: string, x: unknown) => (typeof x !== 'number' || Number.isFinite(x) ? x : x !== x ? '#NaN' : x > 0 ? '#Inf' : '#-Inf');

/**
 * Arguments as JSON for the core. JSON.stringify writes NaN and ±∞ as
 * `null`, which the core would read as NaN (or as a missing optional
 * argument): a distance of `1 / 0` must stay infinite. `null` in the text
 * also stands for undefined and null, so only then is it worth the slower
 * pass that writes the special numbers as "#NaN" / "#Inf" / "#-Inf".
 */
export function writeArgs(value: unknown): string {
  const text = JSON.stringify(value);
  return text.includes('null') ? JSON.stringify(value, special) : text;
}

/**
 * A caller for the core operation `name`. `undef`: the TypeScript function
 * returned `undefined` (not `null`) for "nothing".
 */
export function op<F extends (...args: never[]) => unknown>(name: string, undef = false): F {
  let id = -1;
  const call = (...args: unknown[]): unknown => {
    if (id < 0) {
      if (!compiled) throw new Error('Geometri çekirdeği henüz başlatılmadı.');
      id = opId(name);
      if (id < 0) throw new Error(`Geometri çekirdeğinde “${name}” işlemi yok; WASM paketi eski olabilir (pnpm wasm).`);
    }
    let text: string;
    try {
      text = callOp(id, writeArgs(args));
    } catch (err) {
      fault(err);
      throw err;
    }
    const v = readResult(text);
    return undef && v === null ? undefined : v;
  };
  return call as unknown as F;
}

/** Runs an operation by name on already-built JSON arguments (the golden fixtures). */
export function callNamed(name: string, args: unknown[]): unknown {
  return op(name)(...(args as never[]));
}

/**
 * Fill triangles of many polygons in one call (a layer's fills): `xy` holds
 * every ring's points one after another, `ringSizes` each ring's vertex
 * count and `polyRings` each polygon's ring count (its outer ring, then its
 * holes). Three vertex indices (into the points of `xy`) per triangle come
 * back, polygon after polygon; a renderer takes the coordinates from `xy`.
 */
export function triangulateMany(xy: Float64Array, ringSizes: Uint32Array, polyRings: Uint32Array): Uint32Array {
  return typed(() => wasmTriangulateMany(xy, ringSizes, polyRings));
}

/**
 * The times of a temporal layer's objects (docs/adr/0210 §4) in one call: `texts` each object's start and end values
 * one after another, `lens` their lengths (−1: the object lacks it). Per object `s, e, mode` (−1: timeless), then
 * `timed, timeless, unreadable, extent start, extent end`.
 */
export function coreTimeLayer(ranged: boolean, cumulative: boolean, texts: string, lens: Int32Array): Float64Array {
  return typed(() => wasmTimeLayer(ranged, cumulative, texts, lens));
}

/** Which of a temporal layer's objects show in a window (kind 1 the moment `a`, 2 from `a` up to `b`): 1 per object shown, a timeless one always. */
export function coreTimeLayerMask(ranged: boolean, cumulative: boolean, texts: string, lens: Int32Array, kind: number, a: number, b: number): Uint8Array {
  return typed(() => wasmTimeLayerMask(ranged, cumulative, texts, lens, kind, a, b));
}

/**
 * `model/geometry.ts` on plain numbers (docs/adr/0008, S3b). The tools call
 * these on every pointer move: no JSON, and no closure or array per call.
 * A trap is reported as `op` reports it.
 */
export function coreDist(ax: number, ay: number, bx: number, by: number): number {
  try {
    return wasmDist(ax, ay, bx, by);
  } catch (err) {
    fault(err);
    throw err;
  }
}

export function coreAngleDeg(ax: number, ay: number, bx: number, by: number): number {
  try {
    return wasmAngleDeg(ax, ay, bx, by);
  } catch (err) {
    fault(err);
    throw err;
  }
}

export function coreBearingGrad(ax: number, ay: number, bx: number, by: number): number {
  try {
    return wasmBearingGrad(ax, ay, bx, by);
  } catch (err) {
    fault(err);
    throw err;
  }
}

export function coreDistToSegment(px: number, py: number, ax: number, ay: number, bx: number, by: number): number {
  try {
    return wasmDistToSegment(px, py, ax, ay, bx, by);
  } catch (err) {
    fault(err);
    throw err;
  }
}

/**
 * Ring and path measures through the core's scratch buffer: the points go
 * straight into its memory and the answer comes back there, so a call
 * allocates nothing on either side (the style engine asks for a centroid
 * per object while a layer is built). Returns where the points start.
 */
function toScratch(pts: readonly { x: number; y: number }[]): number {
  const n = pts.length;
  // At least two numbers: the centroid comes back in the first two.
  const at = wasmScratch(Math.max(2, 2 * n));
  const view = new Float64Array(memory!.buffer, at, 2 * n);
  for (let i = 0; i < n; i++) {
    view[2 * i] = pts[i].x;
    view[2 * i + 1] = pts[i].y;
  }
  return at;
}

export function ringSignedArea(pts: readonly { x: number; y: number }[]): number {
  return typed(() => {
    toScratch(pts);
    return wasmScratchSignedArea(pts.length);
  });
}

export function ringPathLength(pts: readonly { x: number; y: number }[], closed: boolean): number {
  return typed(() => {
    toScratch(pts);
    return wasmScratchPathLength(pts.length, closed);
  });
}

export function ringCentroid(pts: readonly { x: number; y: number }[]): { x: number; y: number } {
  return typed(() => {
    const at = toScratch(pts);
    wasmScratchCentroid(pts.length);
    // The call may have grown the memory: a fresh view of the same place.
    const out = new Float64Array(memory!.buffer, at, 2);
    return { x: out[0], y: out[1] };
  });
}

/**
 * The tiles a view draws of a raster (docs/adr/0204 §5; the core's `level_for` and `visible`): how many, and their
 * numbers in the core's memory, 13 a tile (level, column, row, the texture's used share across and down, the corners'
 * xs and ys), nearest the view's centre first. The view of the numbers holds until the next call into the core.
 */
export function rasterTiles(
  x0: number,
  a: number,
  b: number,
  y0: number,
  c: number,
  d: number,
  width: number,
  height: number,
  view: readonly [number, number, number, number],
  pxPerM: number,
): { count: number; numbers: Float64Array } {
  return typed(() => {
    const count = wasmRasterTiles(x0, a, b, y0, c, d, width, height, view[0], view[1], view[2], view[3], pxPerM);
    const at = wasmRasterTilesAt();
    return { count, numbers: new Float64Array(memory!.buffer, at, count * 13) };
  });
}

/** How many levels a raster of `width` × `height` has (docs/adr/0204 §3). */
export function rasterLevelCount(width: number, height: number): number {
  return typed(() => wasmRasterLevelCount(width, height));
}

export function ringPointInPolygon(px: number, py: number, pts: readonly { x: number; y: number }[]): boolean {
  return typed(() => {
    toScratch(pts);
    return wasmScratchPointInPolygon(pts.length, px, py);
  });
}

/** Points as flat coordinates (x0, y0, x1, y1, …), for the typed entry points. */
export function toXY(pts: readonly { x: number; y: number }[]): Float64Array {
  const xy = new Float64Array(2 * pts.length);
  for (let i = 0; i < pts.length; i++) {
    xy[2 * i] = pts[i].x;
    xy[2 * i + 1] = pts[i].y;
  }
  return xy;
}

/** Flat coordinates back as points. */
export function fromXY(xy: Float64Array, from = 0, to = xy.length): { x: number; y: number }[] {
  const pts = new Array<{ x: number; y: number }>((to - from) >> 1);
  for (let i = 0, k = from; k + 1 < to; i++, k += 2) pts[i] = { x: xy[k], y: xy[k + 1] };
  return pts;
}

/**
 * Packed objects (./pack.ts) moved by one similarity and packed again, with
 * no store and no JSON (docs/adr/0037): the product command
 * `cad.entities.transform`. `kind` and `params` are its transform: `move`
 * (dx, dy), `rotate` (cx, cy, angle), `scale` (cx, cy, factor), `mirror`
 * (ax, ay, bx, by); the core builds the matrix, so −0 stays −0.
 */
export function transformObjects(nums: Float64Array, strings: string, kind: string, params: Float64Array): { nums: Float64Array; strings: string } {
  return typed(() => {
    const r = wasmTransformObjects(nums, strings, kind, params);
    const out = r.strings;
    // Hands the numbers over and frees the answer: a large one is not copied twice.
    return { nums: r.intoNums(), strings: out };
  });
}

/**
 * Packed objects (./pack.ts) copied into an array and packed again, with no
 * store and no JSON (docs/adr/0047): the product command
 * `cad.entities.array`. `kind` and `params` are its layout as the core's
 * `array_transforms` takes it: `grid` (rows, cols, dx, dy) or `polar` (cx,
 * cy, count, fill, rotate 1 or 0); `font` is the drawing's typeface id,
 * which measures text for a polar array's middle. The copies come place
 * after place, each place in the order the objects were packed.
 */
export function arrayObjects(nums: Float64Array, strings: string, kind: string, params: Float64Array, font: string): { nums: Float64Array; strings: string } {
  return typed(() => {
    const r = wasmArrayObjects(nums, strings, kind, params, font);
    const out = r.strings;
    return { nums: r.intoNums(), strings: out };
  });
}

/** `offsetPath` on flat coordinates: the style engine offsets a path per object while a layer is built. */
export function offsetPathXY(xy: Float64Array, d: number, closed: boolean): Float64Array {
  return typed(() => wasmOffsetPathXY(xy, d, closed));
}

/**
 * `hatchLines` on flat coordinates (the hatch tool's preview, every frame):
 * holes one after another with `holeSizes` vertex counts. The first number
 * is 1 when the lines were capped, then `ax, ay, bx, by` per segment.
 */
export function hatchLinesXY(ring: Float64Array, holes: Float64Array, holeSizes: Uint32Array, angleDeg: number, spacing: number): Float64Array {
  return typed(() => wasmHatchLinesXY(ring, holes, holeSizes, angleDeg, spacing));
}

/**
 * Faces of line work kept in the core between calls (docs/adr/0008, S3):
 * built once per view, asked for the face under the cursor on every
 * pointer move. Results come back parsed.
 */
export class CoreFaceIndex {
  private raw: FaceIndex | null;

  private constructor(raw: FaceIndex) {
    this.raw = raw;
  }

  /** From overlay sources (a JSON array). */
  static of(sourcesJson: string): CoreFaceIndex {
    return new CoreFaceIndex(typed(() => new FaceIndex(sourcesJson)));
  }

  /** From the line work of entities (a JSON array). */
  static ofEntities(entitiesJson: string): CoreFaceIndex {
    return new CoreFaceIndex(typed(() => FaceIndex.ofEntities(entitiesJson)));
  }

  at(x: number, y: number, islands: boolean): unknown {
    return readResult(typed(() => this.get().at(x, y, islands)));
  }

  all(): unknown {
    return readResult(typed(() => this.get().all()));
  }

  /** Releases the core's copy now instead of when the object is collected. */
  free(): void {
    this.raw?.free();
    this.raw = null;
  }

  private get(): FaceIndex {
    if (!this.raw) throw new Error('Yüz dizini bırakıldı.');
    return this.raw;
  }
}

/**
 * İzle's graph of line work kept in the core between calls (docs/adr/0161 §5): built once per view, asked for the
 * way to the cursor on every pointer move. Results come back parsed.
 */
export class CoreTraceGraph {
  private raw: TraceGraph | null;

  private constructor(raw: TraceGraph) {
    this.raw = raw;
  }

  /** From the line work of entities (a JSON array). */
  static ofEntities(entitiesJson: string): CoreTraceGraph {
    return new CoreTraceGraph(typed(() => TraceGraph.ofEntities(entitiesJson)));
  }

  path(ax: number, ay: number, bx: number, by: number): unknown {
    return readResult(typed(() => this.get().path(ax, ay, bx, by)));
  }

  nearest(x: number, y: number, reach: number): unknown {
    return readResult(typed(() => this.get().nearest(x, y, reach)));
  }

  /** Releases the core's copy now instead of when the object is collected. */
  free(): void {
    this.raw?.free();
    this.raw = null;
  }

  private get(): TraceGraph {
    if (!this.raw) throw new Error('İzleme çizgesi bırakıldı.');
    return this.raw;
  }
}

/**
 * Bitişik alan's neighbours kept in the core between calls (docs/adr/0162 §5): taken once per view and drawing, asked
 * for the region on every pointer move. Results come back parsed.
 */
export class CoreAdjoinWork {
  private raw: AdjoinWork | null;

  private constructor(raw: AdjoinWork) {
    this.raw = raw;
  }

  /** From the entities (a JSON array) that enclose an area. */
  static ofEntities(entitiesJson: string): CoreAdjoinWork {
    return new CoreAdjoinWork(typed(() => AdjoinWork.ofEntities(entitiesJson)));
  }

  edgeCount(): number {
    return typed(() => this.get().edgeCount());
  }

  fill(xy: Float64Array, bulges: Float64Array): unknown {
    return readResult(typed(() => this.get().fill(xy, bulges)));
  }

  avoid(areaJson: string): unknown {
    return readResult(typed(() => this.get().avoid(areaJson)));
  }

  /** Releases the core's copy now instead of when the object is collected. */
  free(): void {
    this.raw?.free();
    this.raw = null;
  }

  private get(): AdjoinWork {
    if (!this.raw) throw new Error('Bitişik alanın komşuları bırakıldı.');
    return this.raw;
  }
}

/** One expression's values for a table of objects (crates/shared/expression/src/rows.rs). */
export interface ExprColumnData {
  /** Per object: 0 empty, 1 number, 2 text, 3 true/false. */
  readonly kinds: Uint8Array;
  /** Per object: the number, 1/0 for true/false, NaN otherwise. */
  readonly numbers: Float64Array;
  /** The text values one after another, and their lengths (UTF-16 code units). */
  readonly texts: string;
  readonly textLengths: Uint32Array;
}

/**
 * Evaluates the expression `source` for `n` objects in one call (docs/adr/0008
 * “İfade dili”): the objects cross as the table `model/expression/expression.ts`
 * builds, each value comes back as `want` asks (0 as it is, 1 a number, 2
 * text, 3 true/false, 4 the number its text reads as). An expression that
 * does not compile throws.
 */
export function exprEvaluate(source: string, n: number, texts: string, textLens: Int32Array, numbers: Float64Array, measures: Float64Array, scale: number, want: number): ExprColumnData {
  return typed(() => {
    const c = wasmExprEvaluate(source, n, texts, textLens, numbers, measures, scale, want);
    try {
      return { kinds: c.kinds, numbers: c.numbers, texts: c.texts, textLengths: c.textLengths };
    } finally {
      c.free();
    }
  });
}

/**
 * `exprEvaluate` in a context (docs/adr/0214): `context` is the schema as JSON,
 * `{ variables, world }` (the `@` values; whether other layers are given). Without
 * a geometry store no call looks at other layers: one that would is refused.
 */
export function exprEvaluateIn(source: string, context: string, n: number, texts: string, textLens: Int32Array, numbers: Float64Array, measures: Float64Array, scale: number, want: number): ExprColumnData {
  return typed(() => {
    const c = wasmExprEvaluateIn(source, context, n, texts, textLens, numbers, measures, scale, want);
    try {
      return { kinds: c.kinds, numbers: c.numbers, texts: c.texts, textLengths: c.textLengths };
    } finally {
      c.free();
    }
  });
}

/**
 * The layers an expression's calls to other objects look at (docs/adr/0214 §3), as one table: every layer's
 * objects one after another (`exprTable` of the fields and values the calls read), their ids, each layer's name
 * and count. Their objects are in the store the expression is evaluated in.
 */
export interface ExprWorldTable {
  readonly names: readonly string[];
  readonly counts: Uint32Array;
  readonly ids: Float64Array;
  readonly texts: string;
  readonly lens: Int32Array;
  readonly numbers: Float64Array;
}

/**
 * Where the texts beside numbered corners go (processing, docs/adr/0008 S4):
 * four numbers per corner in `corners` (x, y, outward x and y), its text in
 * `texts`, measured in the drawing typeface `font`; x, y per corner come back.
 */
/**
 * Reads an NTv2 grid into the core under `id` (the file's SHA-256) for the coordinate transforms (docs/adr/0168 §4);
 * the core's JSON of what it says, or of why it is refused. `model/geom/crsGrid.ts` reads it.
 */
export function crsLoadGrid(id: string, bytes: Uint8Array): unknown {
  return typed(() => readResult(wasmCrsLoadGrid(id, bytes)));
}

/** Lets the grid kept under `id` go. */
export function crsForgetGrid(id: string): void {
  typed(() => wasmCrsForgetGrid(id));
}

export function cornerTexts(corners: Float64Array, texts: readonly string[], height: number, font: string): Float64Array {
  return typed(() => wasmCornerTexts(corners, [...texts], height, font));
}

/**
 * A layer's symbols and expressions for one styled build
 * (crates/shared/style-core/src/style/build.rs `Program`): the attribute names
 * and variables its expressions read, for the table of values the build takes.
 * Freed after the build.
 */
export class CoreStyleProgram {
  readonly raw: StyleProgram;
  /** The program as given (the style fixtures record it). */
  readonly json: string;
  /** The attribute names, in the table's order. */
  readonly fields: string[];
  /** The variables read, as bits: 1 geometry values, 2 corners, 4 kind, 8 layer, 16 label, 32 position, 64 id, 128 scale. */
  readonly needs: number;

  /** What a build depends on beyond its objects (docs/adr/0213 §3), as bits: 1 the view's scale, 2 the heat map's box, 4 the construction lines' box, 8 built whole. */
  readonly viewNeeds: number;

  constructor(json: string) {
    this.json = json;
    this.raw = typed(() => new StyleProgram(json));
    this.fields = JSON.parse(typed(() => this.raw.fields)) as string[];
    this.needs = typed(() => this.raw.needs);
    this.viewNeeds = typed(() => this.raw.viewNeeds);
  }

  free(): void {
    this.raw.free();
  }
}

/** Why a renderer cannot be a layer's (docs/adr/0213 §5; the style core's `renderer_problem`), or null. */
export function rendererProblem(renderer: unknown): string | null {
  const why = typed(() => wasmRendererProblem(JSON.stringify(renderer)));
  return why === '' ? null : why;
}

/** A picture a layer build made (a heat map's, docs/adr/0213 §2.6): RGBA, straight alpha, rows from the top. */
export interface MadePicture {
  readonly key: string;
  readonly width: number;
  readonly height: number;
  readonly rgba: Uint8Array;
}

/** A layer build's answer: its batches, the pictures it made and the dots it left out (docs/adr/0213 §2.4). */
export interface StyledOut {
  json: string;
  data: Float32Array;
  pictures: MadePicture[];
  dropped: number;
}

/**
 * The Rust geometry store (docs/adr/0008, S1): a copy of the drawing's
 * objects that picking, snapping and selection query on every pointer move.
 * `src/viewport/picking.ts` keeps one in step with the document. Objects go
 * in as JSON; queries take numbers and give flat arrays (ids are numbers).
 * Every call reports a trap as `op` does.
 */

/** A label the store placed (docs/adr/0212 §4): its object, class, state, middle (world), angle (degrees) and block (px). */
export interface StoreLabelHit {
  id: number;
  cls: number;
  state: number;
  at: { x: number; y: number };
  angle: number;
  w: number;
  h: number;
}

const labelHit = (r: ArrayLike<number>, i: number): StoreLabelHit => ({
  id: r[i],
  cls: r[i + 1],
  state: r[i + 2],
  at: { x: r[i + 3], y: r[i + 4] },
  angle: r[i + 5],
  w: r[i + 6],
  h: r[i + 7],
});

export class CoreStore {
  private readonly raw: GeometryStore;

  constructor() {
    this.raw = typed(() => new GeometryStore());
  }

  get size(): number {
    return typed(() => this.raw.size);
  }

  /** Adds or replaces objects (a JSON array of entities): new ids go last, known ones keep their place. */
  put(entitiesJson: string): void {
    typed(() => this.raw.put(entitiesJson));
  }

  /** Adds or replaces packed objects (./pack.ts), as `put`. */
  putPacked(nums: Float64Array, strings: string): void {
    typed(() => this.raw.putPacked(nums, strings));
  }

  remove(ids: Float64Array): void {
    typed(() => this.raw.remove(ids));
  }

  clear(): void {
    typed(() => this.raw.clear());
  }

  /** The drawing typeface (`DrawingFont` id): text boxes follow its measured letters. */
  setFont(id: string): void {
    typed(() => this.raw.setFont(id));
  }

  /** `[{ id, visible, locked, pickInterior }]` for every layer node, ancestors resolved. */
  setLayers(json: string): void {
    typed(() => this.raw.setLayers(json));
  }

  /** The drawing's block definitions (docs/adr/0144), the contract's JSON: every insert is placed again. */
  setBlocks(json: string): void {
    typed(() => this.raw.setBlocks(json));
  }

  /** A block's pieces as `GROUP` records and piece labels number them (JSON), or undefined for an unknown block. */
  blockPieces(block: string): string | undefined {
    return typed(() => this.raw.blockPieces(block));
  }

  /** An insert's pieces as placed (JSON), or undefined for any other object. */
  insertPieces(id: number): string | undefined {
    return typed(() => this.raw.insertPieces(id));
  }

  /** Patlat of an insert (docs/adr/0144 §3): `{ pieces }` or `{ error }` as JSON. */
  explodeInsert(entityJson: string): string {
    return typed(() => this.raw.explodeInsert(entityJson));
  }

  /**
   * The label engine alone (docs/adr/0212 §3; the shared cases' fixtures/labels/v1): the labels of a window at `scale`
   * px/m around the drawing's texts' outlines `fixedJson` (`[[[x, y] …] …]`, world); `flags` as `labels`'. Their texts
   * are `placedTexts`'.
   */
  placeLabels(x0: number, y0: number, x1: number, y1: number, scale: number, fixedJson: string, flags = 0): Float64Array {
    return typed(() => this.raw.placeLabels(x0, y0, x1, y1, scale, fixedJson, flags));
  }

  /** The texts of the labels last asked, a line each. */
  placedTextsOf(): string[] {
    const t = typed(() => this.raw.placedTexts());
    return t ? t.split('\n') : [];
  }

  /** Label rules by kind for layers without a label style. */
  setLabelDefaults(json: string): void {
    typed(() => this.raw.setLabelDefaults(json));
  }

  /** The label engine's layers (docs/adr/0212 §3.1): `[{ id, rank, point?, label?, labels? }]` as JSON. */
  setLabelLayers(json: string): void {
    typed(() => this.raw.setLabelLayers(json));
  }

  /**
   * Objects' labels' texts (docs/adr/0212 §3.1): `ids[i]`'s are the entries `from[i]..from[i + 1]` of `classes` and
   * `texts` (`lens` their UTF-16 lengths), its height `zs[i]` (NaN none). An object with none forgets its own.
   */
  setObjectLabels(ids: Float64Array, from: Uint32Array, classes: Uint16Array, texts: string, lens: Uint32Array, zs: Float64Array): void {
    typed(() => this.raw.setObjectLabels(ids, from, classes, texts, lens, zs));
  }

  /** Objects' pins (docs/adr/0212 §3.7): `[[id, [LabelPin …]] …]` as JSON; an empty list forgets an object's. */
  setLabelPins(json: string): void {
    typed(() => this.raw.setLabelPins(json));
  }

  /** The texts of the labels last asked, a string each (`labels`' and `labelsShown`'s records name them by index). */
  placedTexts(): string[] {
    const t = typed(() => this.raw.placedTexts());
    return t ? t.split('\n') : [];
  }

  /** The label under (x, y) among the main view's last labels within `tol` px; `all` counts the unplaced and hidden. */
  labelAt(x: number, y: number, tol: number, all: boolean): StoreLabelHit | null {
    const r = typed(() => this.raw.labelAt(x, y, tol, all));
    return r.length ? labelHit(r, 0) : null;
  }

  /** The labels whose middle is in the box (x0, y0)–(x1, y1) among the main view's last labels (docs/adr/0212 §4). */
  labelsIn(x0: number, y0: number, x1: number, y1: number, all: boolean): StoreLabelHit[] {
    const r = typed(() => this.raw.labelsIn(x0, y0, x1, y1, all));
    const out: StoreLabelHit[] = [];
    for (let i = 0; i + 8 <= r.length; i += 8) out.push(labelHit(r, i));
    return out;
  }

  /** Where an object's labels are pinned from: its anchor (world), null for none (docs/adr/0212 §2). */
  labelAnchor(id: number): { x: number; y: number } | null {
    const r = typed(() => this.raw.labelAnchor(id));
    return r.length ? { x: r[0], y: r[1] } : null;
  }


  /** The objects whose label a text writes (docs/adr/0175 §4): `labels` leaves their own out. */
  setTextLabelled(ids: Float64Array): void {
    typed(() => this.raw.setTextLabelled(ids));
  }

  /** The objects' times (docs/adr/0210 §6): `s, e, mode` per id, mode −1 takes it away. */
  setTimes(ids: Float64Array, times: Float64Array): void {
    typed(() => this.raw.setTimes(ids, times));
  }

  /** A temporal layer's objects' times read from their values straight into the store (`texts`, `lens` as `coreTimeLayer`'s). */
  setLayerTimes(ids: Float64Array, ranged: boolean, cumulative: boolean, texts: string, lens: Int32Array): void {
    typed(() => this.raw.setLayerTimes(ids, ranged, cumulative, texts, lens));
  }

  clearTimes(): void {
    typed(() => this.raw.clearTimes());
  }

  /** The time slider's window: kind 0 none, 1 the moment `a`, 2 from `a` up to `b`; queries leave out what it does not show. */
  setTimeWindow(kind: number, a: number, b: number): void {
    typed(() => this.raw.setTimeWindow(kind, a, b));
  }

  /** How many objects have a time, then the extent of their starts and ends (NaN without one). */
  timeSummary(): Float64Array {
    return typed(() => this.raw.timeSummary());
  }

  /** For each id, 1 when it shows at the slider's window. */
  timeMask(ids: Float64Array): Uint8Array {
    return typed(() => this.raw.timeMask(ids));
  }

  /** The objects their layer's filter leaves out (docs/adr/0211 §3): 1 in `out` leaves one out, 0 lets it in again. */
  setFiltered(ids: Float64Array, out: Uint8Array): void {
    typed(() => this.raw.setFiltered(ids, out));
  }

  clearFiltered(): void {
    typed(() => this.raw.clearFiltered());
  }

  /** For each id, 1 when the view shows it: it passes its layer's filter and shows at the slider's window. */
  viewMask(ids: Float64Array): Uint8Array {
    return typed(() => this.raw.viewMask(ids));
  }

  /**
   * What the overlay draws in the view: nine numbers per record (geometry-core store/labels.rs); the objects' labels
   * placed by the label engine (docs/adr/0212 §3.8), their texts then `placedTexts`. `flags`: 1 the unplaced too,
   * 2 the hidden, 4 kept for `labelAt`.
   */
  labels(minX: number, minY: number, maxX: number, maxY: number, scale: number, editing: number | null, flags = 0): Float64Array {
    return typed(() => this.raw.labels(minX, minY, maxX, maxY, scale, editing !== null, editing ?? 0, flags));
  }

  /**
   * The same as the view shows them under `size` (`graphics.annotationSize`, docs/adr/0205 §5): each record, its factor
   * and the point it grows about (store/legible.rs, `LABEL_SHOWN_STRIDE` numbers a record).
   */
  labelsShown(minX: number, minY: number, maxX: number, maxY: number, scale: number, editing: number | null, size: string, plotScale: number, flags = 0): Float64Array {
    return typed(() => this.raw.labelsShown(minX, minY, maxX, maxY, scale, editing !== null, editing ?? 0, size, plotScale, flags));
  }

  /** Etiketleri yazıya çevir (docs/adr/0212 §4): the objects' ids → `{ texts, callouts, outOfScale, small, overlapping }` as JSON. */
  labelTexts(ids: Float64Array, scale: number, every: boolean): string {
    return typed(() => this.raw.labelTexts(ids, scale, every));
  }

  /** Grips of these objects: `id, count, vertices`, then `x, y, segment` per grip. */
  grips(ids: Float64Array): Float64Array {
    return typed(() => this.raw.grips(ids));
  }

  /** `trimEntity` against the trim tool's boundaries (`chosen` ids, or the visible edges in the view). */
  trimPreview(target: string, x: number, y: number, except: number | null, chosen: Float64Array | null, minX: number, minY: number, maxX: number, maxY: number): unknown {
    return readResult(typed(() => this.raw.trimPreview(target, x, y, except !== null, except ?? 0, chosen, minX, minY, maxX, maxY)));
  }

  /** `extendEntity` against the extend tool's boundaries (as `trimPreview`). */
  extendPreview(target: string, x: number, y: number, except: number | null, chosen: Float64Array | null, minX: number, minY: number, maxX: number, maxY: number): unknown {
    return readResult(typed(() => this.raw.extendPreview(target, x, y, except !== null, except ?? 0, chosen, minX, minY, maxX, maxY)));
  }

  /** Outlines of objects moved by each affine (six numbers each): `flags, n, x0, y0, …` per path. */
  transformOutlines(ids: Float64Array, affines: Float64Array, limit: number): Float64Array {
    return typed(() => this.raw.transformOutlines(ids, affines, limit));
  }

  /** Outlines of a block placed as an insert would place it (docs/adr/0144), as `transformOutlines`; empty for an unknown block. */
  insertOutlines(block: string, x: number, y: number, scale: number, rotation: number, mirror: boolean): Float64Array {
    return typed(() => this.raw.insertOutlines(block, x, y, scale, rotation, mirror));
  }

  /** A drawing point in the own coordinates of the definition the insert `id` places (docs/adr/0144); empty when it is not an insert of a known block. */
  insertLocal(id: number, x: number, y: number): Float64Array {
    return typed(() => this.raw.insertLocal(id, x, y));
  }

  /**
   * These objects moved by each affine (six numbers each), affine after
   * affine, as `transformEntities` gives them: packed as `putPacked` reads
   * them (./pack.ts `unpackEntities` reads them back), not as JSON. Move,
   * copy, arrays and paste. Unknown ids are skipped.
   */
  transformPacked(ids: Float64Array, affines: Float64Array): { nums: Float64Array; strings: string } {
    return typed(() => {
      const r = this.raw.transformPacked(ids, affines);
      const strings = r.strings;
      // Hands the numbers over and frees the answer: a large one is not copied twice.
      return { nums: r.intoNums(), strings };
    });
  }

  /** Outlines of objects stretched by a window and a displacement. */
  stretchOutlines(ids: Float64Array, minX: number, minY: number, maxX: number, maxY: number, dx: number, dy: number): Float64Array {
    return typed(() => this.raw.stretchOutlines(ids, minX, minY, maxX, maxY, dx, dy));
  }

  /** `[length, area]` of these objects. */
  measure(ids: Float64Array): Float64Array {
    return typed(() => this.raw.measure(ids));
  }

  /** What is drawn of these objects, one record each (read by `style/geometry.ts` `readDrawn`); `clip`: the box construction lines are clipped to. */
  drawn(ids: Float64Array, oriented: boolean, clip: { minX: number; minY: number; maxX: number; maxY: number } | null): Float64Array {
    return typed(() => this.raw.drawn(ids, oriented, clip !== null, clip?.minX ?? 0, clip?.minY ?? 0, clip?.maxX ?? 0, clip?.maxY ?? 0));
  }

  /** Geometry values of these objects for expressions: `flags, length, area, anchor x, anchor y, 0` each. */
  measures(ids: Float64Array): Float64Array {
    return typed(() => this.raw.measures(ids));
  }

  /**
   * One expression's values for these objects (docs/adr/0100 §3): the table
   * of what it reads of their attributes and names as `exprEvaluate` takes it,
   * without `measures`; the geometry values (`$alan`, `$merkez_y`, `$genişlik` …)
   * are read here from the store's shapes, only those the expression reads.
   */
  evaluateExpression(source: string, ids: Float64Array, texts: string, textLens: Int32Array, numbers: Float64Array, scale: number, want: number): ExprColumnData {
    return typed(() => {
      const c = this.raw.evaluateExpression(source, ids, texts, textLens, numbers, scale, want);
      try {
        return { kinds: c.kinds, numbers: c.numbers, texts: c.texts, textLengths: c.textLengths };
      } finally {
        c.free();
      }
    });
  }

  /**
   * `evaluateExpression` in a context (docs/adr/0214): the schema as JSON (`{ variables, world }`), and the layers
   * its calls to other objects look at, whose objects are in this store too.
   */
  evaluateExpressionIn(source: string, context: string, ids: Float64Array, texts: string, textLens: Int32Array, numbers: Float64Array, scale: number, want: number, world?: ExprWorldTable): ExprColumnData {
    return typed(() => {
      // Handed over to the core, which frees it.
      const w = world ? new WasmExprWorld([...world.names], world.counts, world.ids, world.texts, world.lens, world.numbers) : undefined;
      const c = this.raw.evaluateExpressionIn(source, context, ids, texts, textLens, numbers, scale, want, w);
      try {
        return { kinds: c.kinds, numbers: c.numbers, texts: c.texts, textLengths: c.textLengths };
      } finally {
        c.free();
      }
    });
  }

  /**
   * A layer through the style engine (crates/shared/style-core/src/style/build.rs):
   * `objects` four numbers per id (how it is drawn, its set or symbol, the set
   * of its simple look, its colour), `pieces` the sets of every insert's pieces
   * in turn (docs/adr/0144), the program's table of values, the box
   * construction lines are clipped to, the batches' origin, the plot scale, and
   * whether symbol sizes are on the screen (paper mm drawn as px, steady while zooming).
   * The batches' descriptions (JSON) and their numbers one after another.
   */
  buildStyled(
    program: CoreStyleProgram,
    ids: Float64Array,
    objects: Int32Array,
    pieces: Int32Array,
    table: { texts: string; lens: Int32Array; numbers: Float64Array },
    clip: { minX: number; minY: number; maxX: number; maxY: number } | null,
    origin: { x: number; y: number },
    plotScale: number,
    screen = false,
    view = { fills: true, areaEdges: true },
    frame: { pxPerM: number; picture: string } | null = null,
  ): StyledOut {
    return typed(() => {
      const r = this.raw.buildStyled(program.raw, ids, objects, pieces, table.texts, table.lens, table.numbers, clip !== null, clip?.minX ?? 0, clip?.minY ?? 0, clip?.maxX ?? 0, clip?.maxY ?? 0, origin.x, origin.y, plotScale, screen, view.fills, view.areaEdges, frame?.pxPerM ?? 0, frame?.picture ?? '');
      const json = r.json;
      const sizes = JSON.parse(r.pictures) as { key: string; width: number; height: number }[];
      const pictures = sizes.map((p, i) => ({ ...p, rgba: r.pictureData(i) }));
      const dropped = r.dropped;
      return { json, data: r.intoData(), pictures, dropped };
    });
  }

  /** Ids of objects on every layer whose box overlaps the rectangle, in the document's order (the "visible" scope). */
  inBox(minX: number, minY: number, maxX: number, maxY: number): Float64Array {
    return typed(() => this.raw.inBox(minX, minY, maxX, maxY));
  }

  /**
   * Corner numbering of these objects (polygons: outer ring then holes;
   * polylines): five numbers per corner, `x, y, out x, out y, ref`.
   * `start`: 0 north-west, 1 north, 2 first vertex, 3 nearest `point`;
   * `existing`: x, y pairs of numbered points a corner may take.
   */
  numberCorners(ids: Float64Array, ccw: boolean, start: number, point: { x: number; y: number } | null, tolerance: number, shared: boolean, existing: Float64Array): Float64Array {
    return typed(() => this.raw.numberCorners(ids, ccw, start, !!point, point?.x ?? 0, point?.y ?? 0, tolerance, shared, existing));
  }

  /** Edge-length labels of these objects: skipped shared edges, then `id, x, y, rotation, length` per label. */
  edgeLengths(ids: Float64Array, height: number, minLength: number, inside: boolean, shared: boolean): Float64Array {
    return typed(() => this.raw.edgeLengths(ids, height, minLength, inside, shared));
  }

  /**
   * Pairs of input and reference objects in the relation (docs/adr/0200 §1; code: the core's `Relation::from_code`,
   * 0 Kesişen, 1 İçeren, 2 İçinde kalan, 3 Ayrık, 4 Uzaklıkta, 5 Merkezi içinde), `within` metres for Uzaklıkta:
   * flat (input position, reference position) pairs, inputs first, then references in their order.
   */
  relatePairs(inputs: Float64Array, references: Float64Array, relation: number, within: number): Float64Array {
    return typed(() => this.raw.relatePairs(inputs, references, relation, within));
  }

  /**
   * Each input's nearest targets (docs/adr/0215 §2.1): `k` of them (0: all) within `max` (Infinity: no bound), edge to
   * edge (0) or centre to centre (1); eight numbers each: input place, target place, distance, the input's nearest
   * point (x, y), the target's (x, y) and the bearing between them (radians, clockwise from north; NaN at 0 apart).
   */
  nearest(inputs: Float64Array, targets: Float64Array, k: number, max: number, measure: number): Float64Array {
    return typed(() => this.raw.nearest(inputs, targets, k, max, measure));
  }

  /** The areas' neighbours (docs/adr/0215 §2.2): five numbers each: place, neighbour's place, kind (0 edge, 1 corner, 2 overlap), shared length, overlapping area. */
  neighbors(ids: Float64Array, tolerance: number, corners: boolean, overlaps: boolean): Float64Array {
    return typed(() => this.raw.neighbors(ids, tolerance, corners, overlaps));
  }

  /** Ids in the document's order. */
  ids(): Float64Array {
    return typed(() => this.raw.ids());
  }

  /** An object as the store holds it (id, layer, label flag, geometry), as JSON; for tests. */
  itemJson(id: number): string | undefined {
    return typed(() => this.raw.itemJson(id));
  }

  /** `[minX, minY, maxX, maxY]` around these objects (all of them when `ids` is null), or empty. */
  extent(ids: Float64Array | null): Float64Array {
    return typed(() => this.raw.extent(ids ?? undefined));
  }

  /** `[minX, minY, maxX, maxY]`, or empty for an unknown id. */
  bounds(id: number): Float64Array {
    return typed(() => this.raw.bounds(id));
  }

  hit(x: number, y: number, tol: number): number | undefined {
    return typed(() => this.raw.hit(x, y, tol));
  }

  /** `id, distance` pairs, nearest first. */
  hitEdge(x: number, y: number, tol: number): Float64Array {
    return typed(() => this.raw.hitEdge(x, y, tol));
  }

  /** Every object a click could mean, the most specific first (Sıradakini seç, docs/adr/0187 §1). */
  hits(x: number, y: number, tol: number): Float64Array {
    return typed(() => this.raw.hits(x, y, tol));
  }

  /** `[kind bit number, x, y, id]` or empty; `from` is the command's last point. */
  snap(x: number, y: number, tol: number, kinds: number, from: { x: number; y: number } | null): Float64Array {
    return typed(() => this.raw.snap(x, y, tol, kinds, !!from, from?.x ?? 0, from?.y ?? 0));
  }

  /**
   * `snap` with what the drawing does not hold (docs/adr/0163): acquired extensions as records (`[0, endX, endY, dirX,
   * dirY]` a line, `[1, cx, cy, r, a0, sweep]` an arc), parallel directions (`[ux, uy]` each), the object being drawn
   * as an open path, Karelaj's spacings (0: none).
   */
  snapEx(
    x: number,
    y: number,
    tol: number,
    kinds: number,
    from: { x: number; y: number } | null,
    extensions: Float64Array,
    parallels: Float64Array,
    draftXy: Float64Array,
    draftBulges: Float64Array,
    gridX: number,
    gridY: number,
  ): Float64Array {
    return typed(() => this.raw.snapEx(x, y, tol, kinds, !!from, from?.x ?? 0, from?.y ?? 0, extensions, parallels, draftXy, draftBulges, gridX, gridY));
  }

  /** The extensions of object `id`'s edges ending at the point, as `snapEx` takes them. */
  extensionsAt(id: number, x: number, y: number): Float64Array {
    return typed(() => this.raw.extensionsAt(id, x, y));
  }

  /** The direction `[ux, uy]` of the straight edge nearest the point within `tol`, or nothing. */
  directionAt(x: number, y: number, tol: number): Float64Array {
    return typed(() => this.raw.directionAt(x, y, tol));
  }

  near(x: number, y: number, tol: number): Float64Array {
    return typed(() => this.raw.near(x, y, tol));
  }

  overlapping(minX: number, minY: number, maxX: number, maxY: number, except?: number): Float64Array {
    return typed(() => this.raw.overlapping(minX, minY, maxX, maxY, except !== undefined, except ?? 0));
  }

  inRect(minX: number, minY: number, maxX: number, maxY: number, crossing: boolean): Float64Array {
    return typed(() => this.raw.inRect(minX, minY, maxX, maxY, crossing));
  }

  /** `[id, x0, y0, x1, y1, …]` or empty. */
  enclosing(x: number, y: number): Float64Array {
    return typed(() => this.raw.enclosing(x, y));
  }

  /** `[id, area, id, area, …]`: the closed shapes around the point, smallest first (docs/adr/0141). */
  containing(x: number, y: number): Float64Array {
    return typed(() => this.raw.containing(x, y));
  }

  /** `[id, part, hole, …]`: the areas with a hole around the point, the smallest hole first (docs/adr/0173 §5). */
  holesAt(x: number, y: number): Float64Array {
    return typed(() => this.raw.holesAt(x, y));
  }

  /** Ids of the objects the fence `[x0, y0, x1, y1, …]` crosses; a point within `tol` counts. */
  inFence(fence: Float64Array, tol: number): Float64Array {
    return typed(() => this.raw.inFence(fence, tol));
  }

  /** Ids of the objects wholly inside the circle, or also those it touches when `crossing`. */
  inCircle(x: number, y: number, r: number, crossing: boolean): Float64Array {
    return typed(() => this.raw.inCircle(x, y, r, crossing));
  }

  /** Çokgenle seç (docs/adr/0187 §2): `mode` 0 inside, 1 touching too, 2 not touching. */
  inPolygon(ring: Float64Array, mode: number): Float64Array {
    return typed(() => this.raw.inPolygon(ring, mode));
  }

  /** Ids of the objects lying far from the rest of the drawing (Kapsam denetimi). */
  extentOutliers(): Float64Array {
    return typed(() => this.raw.extentOutliers());
  }

  /** Genel bakış (docs/adr/0181 §3): the extent of what the visible layers hold, `minX, minY, maxX, maxY`; empty for nothing. */
  overviewExtent(): Float64Array {
    return typed(() => this.raw.overviewExtent());
  }

  /** The overview's picture: RGBA rows of ⌊width·dpr + 0.5⌋ pixels; `colorsJson` is `{ layerId: '#RRGGBB' }`; empty for nothing. */
  overviewPicture(width: number, height: number, dpr: number, colorsJson: string): Uint8Array {
    return typed(() => this.raw.overviewPicture(width, height, dpr, colorsJson));
  }

  /** Packed edges: `0, ax, ay, bx, by` (segment) or `1, cx, cy, r, a0, sweep` (arc). */
  edgesIn(minX: number, minY: number, maxX: number, maxY: number, except?: number): Float64Array {
    return typed(() => this.raw.edgesIn(minX, minY, maxX, maxY, except !== undefined, except ?? 0));
  }

  /**
   * Ağ analizi (docs/adr/0209 §3): the network of definition `defJson` over this store's objects: the edges (`ids`) with
   * their values (`[direction | null, [cost values], closed]` each, JSON) and the junctions with theirs (`[role, closed]`
   * each). The store must outlive nothing: the network keeps its own copy of what it read.
   */
  buildNetwork(defJson: string, edges: Float64Array, edgeValues: string, junctions: Float64Array, junctionValues: string): CoreNetwork {
    return CoreNetwork.wrap(typed(() => NetworkGraph.build(this.raw, defJson, edges, edgeValues, junctions, junctionValues)));
  }

  /** Frees the Rust side; the store must not be used afterwards. */
  dispose(): void {
    this.raw.free();
  }
}

/**
 * A network kept in the core between questions (docs/adr/0209 §10, §12): built once from a store, then asked for
 * places, routes, the way to the cursor, service areas, closest facilities, traces and its check. Answers come back as
 * the core writes them (JSON text, parsed by the caller), but the way to the cursor, which is numbers.
 */
export class CoreNetwork {
  private raw: NetworkGraph | null;

  private constructor(raw: NetworkGraph) {
    this.raw = raw;
  }

  /** @internal The store's `buildNetwork`. */
  static wrap(raw: NetworkGraph): CoreNetwork {
    return new CoreNetwork(raw);
  }

  summary(): string {
    return typed(() => this.get().summary());
  }

  locate(x: number, y: number, reach: number): string {
    return typed(() => this.get().locate(x, y, reach));
  }

  route(stops: string, barriers: string, reach: number, cost: number, reorder: string): string {
    return typed(() => this.get().route(stops, barriers, reach, cost, reorder));
  }

  treeFrom(x: number, y: number, reach: number, cost: number, barriers: string): boolean {
    return typed(() => this.get().treeFrom(x, y, reach, cost, barriers));
  }

  /** `[cost, n, x0, y0, …, bulge0, …]`; empty when there is no way. */
  pathTo(x: number, y: number, reach: number): Float64Array {
    return typed(() => this.get().pathTo(x, y, reach));
  }

  area(facilities: string, breaks: string, reach: number, cost: number, toward: boolean, separate: boolean, barriers: string, trim: number, rings: boolean, areas: boolean): string {
    return typed(() => this.get().area(facilities, breaks, reach, cost, toward, separate, barriers, trim, rings, areas));
  }

  /** `k` below 0: every target; `cutoff` NaN: none. */
  nearest(origins: string, targets: string, reach: number, k: number, cutoff: number, cost: number, reverse: boolean, barriers: string, paths: boolean): string {
    return typed(() => this.get().nearest(origins, targets, reach, k, cutoff, cost, reverse, barriers, paths));
  }

  trace(starts: string, barriers: string, reach: number, kind: string): string {
    return typed(() => this.get().trace(starts, barriers, reach, kind));
  }

  check(): string {
    return typed(() => this.get().check());
  }

  /** Releases the core's copy now instead of when the object is collected. */
  free(): void {
    this.raw?.free();
    this.raw = null;
  }

  private get(): NetworkGraph {
    if (!this.raw) throw new Error('Ağ bırakıldı.');
    return this.raw;
  }
}
