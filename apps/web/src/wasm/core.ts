import {
  angleDeg as wasmAngleDeg,
  bearingGrad as wasmBearingGrad,
  callOp,
  cornerTexts as wasmCornerTexts,
  dist as wasmDist,
  distToSegment as wasmDistToSegment,
  FaceIndex,
  GeometryStore,
  hatchLinesXY as wasmHatchLinesXY,
  initSync,
  offsetPathXY as wasmOffsetPathXY,
  opId,
  scratch as wasmScratch,
  scratchCentroid as wasmScratchCentroid,
  scratchPathLength as wasmScratchPathLength,
  scratchPointInPolygon as wasmScratchPointInPolygon,
  scratchSignedArea as wasmScratchSignedArea,
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

/** A core result read back: the special numbers are strings starting with "#". */
export function readResult(text: string): unknown {
  return text.includes('"#') ? JSON.parse(text, revive) : JSON.parse(text);
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
 * Where the texts beside numbered corners go (processing, docs/adr/0008 S4):
 * four numbers per corner in `corners` (x, y, outward x and y), its text's
 * character count in `chars`; x, y per corner come back.
 */
export function cornerTexts(corners: Float64Array, chars: Float64Array, height: number): Float64Array {
  return typed(() => wasmCornerTexts(corners, chars, height));
}

/**
 * The Rust geometry store (docs/adr/0008, S1): a copy of the drawing's
 * objects that picking, snapping and selection query on every pointer move.
 * `src/viewport/picking.ts` keeps one in step with the document. Objects go
 * in as JSON; queries take numbers and give flat arrays (ids are numbers).
 * Every call reports a trap as `op` does.
 */
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

  /** `[{ id, visible, locked, pickInterior }]` for every layer node, ancestors resolved. */
  setLayers(json: string): void {
    typed(() => this.raw.setLayers(json));
  }

  /** Label rules by kind for layers without a label style. */
  setLabelDefaults(json: string): void {
    typed(() => this.raw.setLabelDefaults(json));
  }

  /** What the overlay draws in the view: eight numbers per record (geometry-core store/labels.rs). */
  labels(minX: number, minY: number, maxX: number, maxY: number, scale: number, editing: number | null): Float64Array {
    return typed(() => this.raw.labels(minX, minY, maxX, maxY, scale, editing !== null, editing ?? 0));
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

  /** `[kind bit number, x, y, id]` or empty; `from` is the command's last point. */
  snap(x: number, y: number, tol: number, kinds: number, from: { x: number; y: number } | null): Float64Array {
    return typed(() => this.raw.snap(x, y, tol, kinds, !!from, from?.x ?? 0, from?.y ?? 0));
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

  /** Packed edges: `0, ax, ay, bx, by` (segment) or `1, cx, cy, r, a0, sweep` (arc). */
  edgesIn(minX: number, minY: number, maxX: number, maxY: number, except?: number): Float64Array {
    return typed(() => this.raw.edgesIn(minX, minY, maxX, maxY, except !== undefined, except ?? 0));
  }

  /** Frees the Rust side; the store must not be used afterwards. */
  dispose(): void {
    this.raw.free();
  }
}
