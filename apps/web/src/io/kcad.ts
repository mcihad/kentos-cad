import type { DocumentSnapshotV2 } from '../contracts/generated/DocumentSnapshotV2';
import type { DrawingColumns, DrawingHead, PackedDrawing } from './columns';

/**
 * The binary project file, `.kcad` v2 (docs/specs/kcad-v2.md, docs/adr/0025,
 * 0030), as the browser runs it. The codec is the shared Rust one
 * (crates/shared/kcad in the formats WASM module); this module is what
 * surrounds it in the formats worker (and in process in tests):
 *
 * - `sniffDrawing`: what a file is by content (spec §8), so the page knows
 *   whether to hand it to the worker (v2) or read it as v1 text;
 * - `encodeWith`: a drawing the page packed (io/columns.ts) to bytes. The
 *   module reads them back and compares the objects with the columns as
 *   they came, every float bit for bit; here the head the file holds is
 *   compared with the head the page sent (a field the contract does not read
 *   would show there). Only bytes that read back to the same drawing leave
 *   the worker;
 * - `decodeWith`: bytes to the drawing (its head and its objects as columns),
 *   or the specification's error code and a Turkish message.
 *
 * Both report their stages (`KcadProgress`), which the worker posts to the page.
 */

/** A stage of a long read or write (the formats module's `KcadProgress`). */
export type KcadProgress =
  | { stage: 'checking' | 'reading' | 'writing' | 'verifying'; done: number; total: number }
  | { stage: 'project'; name: string; layers: number; objects: number };

/** What the module's KCAD calls give back (`Kcad` in crates/wasm/formats-wasm). */
export interface KcadResult {
  readonly ok: boolean;
  readonly code: string;
  readonly message: string;
  takeHead(): string;
  takeBytes(): Uint8Array;
  takeKinds(): Uint8Array;
  takeUids(): Uint8Array;
  takeInts(): Uint32Array;
  takeFloats(): Float64Array;
  takeText(): Uint16Array;
  takeTextLengths(): Uint32Array;
  free(): void;
}

/** The progress object the module calls (`KcadProgress` in crates/wasm/formats-wasm). */
interface Relay {
  step(stage: string, done: number, total: number): void;
  project(name: string, layers: number, objects: number): void;
}

/**
 * The `.kcad` v2 codec as the page uses it: the formats worker (client.ts),
 * or the same module in process in tests (testFormats.ts). `encode` takes
 * the drawing's buffers (they are the codec's once called).
 */
export interface KcadCodec {
  encode(drawing: PackedDrawing, progress?: (p: KcadProgress) => void): Promise<Uint8Array<ArrayBuffer>>;
  decode(bytes: Uint8Array, progress?: (p: KcadProgress) => void): Promise<PackedDrawing>;
  /** Stops what the codec is doing now: the waiting call fails with the code `cancelled` (the worker ends). */
  cancel?(): void;
}

/** The formats module's two KCAD calls (crates/wasm/formats-wasm). */
export interface KcadModule {
  encodeKcad(head: string, kinds: Uint8Array, uids: Uint8Array, ints: Uint32Array, floats: Float64Array, text: Uint16Array, textLengths: Uint32Array, progress: Relay): KcadResult;
  decodeKcad(bytes: Uint8Array, progress: Relay): KcadResult;
}

/** A file refused or a drawing not written: the specification's code (§9) and a Turkish message. */
export class KcadError extends Error {
  readonly code: string;
  constructor(code: string, message: string) {
    super(message);
    this.code = code;
  }
}

/** What a file is by content (docs/specs/kcad-v2.md §8). */
export type DrawingKind = 'kcad' | 'kcad-damaged' | 'json' | 'empty' | 'foreign';

const MAGIC = [0x89, 0x4b, 0x43, 0x41, 0x44, 0x0d, 0x0a, 0x1a, 0x0a] as const;

export function sniffDrawing(bytes: Uint8Array): DrawingKind {
  const starts = (n: number) => bytes.length >= n && MAGIC.slice(0, n).every((b, i) => bytes[i] === b);
  if (starts(9)) return 'kcad';
  if (starts(5)) return 'kcad-damaged';
  let i = bytes.length >= 3 && bytes[0] === 0xef && bytes[1] === 0xbb && bytes[2] === 0xbf ? 3 : 0;
  while (i < bytes.length && (bytes[i] === 0x20 || bytes[i] === 0x09 || bytes[i] === 0x0a || bytes[i] === 0x0d)) i++;
  if (i === bytes.length) return 'empty';
  return bytes[i] === 0x7b ? 'json' : 'foreign';
}

// ── The contract's fields ───────────────────────────────────────────────

type Obj = Record<string, unknown>;
const isObj = (v: unknown): v is Obj => typeof v === 'object' && v !== null && !Array.isArray(v);
type Pick = (v: unknown, where: string) => unknown;

/**
 * Fields KCAD v2 cannot hold, by where they were found (`polyline.not`):
 * the v1 reader keeps what it does not know, and a save says what it left out.
 */
export type Dropped = Record<string, number>;

class Projection {
  readonly dropped: Dropped = {};

  /** `v` with only the fields in `fields` (each through its own pick); others are counted as dropped. */
  fields(v: unknown, fields: Record<string, Pick>, where: string): unknown {
    if (!isObj(v)) return v;
    const out: Obj = {};
    for (const key of Object.keys(v)) {
      const value = v[key];
      if (value === undefined) continue;
      const pick = fields[key];
      if (pick) out[key] = pick(value, `${where}.${key}`);
      else this.dropped[`${where}.${key}`] = (this.dropped[`${where}.${key}`] ?? 0) + 1;
    }
    return out;
  }

  list(item: Pick): Pick {
    return (v, where) => (Array.isArray(v) ? v.map((x) => item(x, where)) : v);
  }
}

const same: Pick = (v) => v;

/**
 * The drawing without its objects reduced to the contract's fields, fresh
 * objects all the way down; what else it held is counted in `dropped`
 * (`belge.layers.style.glow`). The objects are packed field by field
 * (io/columns.ts), which counts theirs. Opaque parts (project styles,
 * renderers) are the style engine's own and stay whole.
 */
export function projectHead(head: DrawingHead): { head: DrawingHead; dropped: Dropped } {
  const p = new Projection();
  const vec = (v: unknown, where: string) => p.fields(v, { x: same, y: same }, where);
  const label = (v: unknown, where: string) =>
    p.fields(
      v,
      { placement: same, size: same, grow: same, maxSize: same, weight: same, template: same, minFeaturePx: same, minScale: same, maxScale: same, ink: same },
      where,
    );
  const style = (v: unknown, where: string) =>
    p.fields(
      v,
      {
        color: same,
        lineType: same,
        lineWeight: same,
        fill: same,
        point: (x, w) => p.fields(x, { symbol: same, size: same }, w),
        label,
        pickInterior: same,
        renderer: same,
      },
      where,
    );
  const layer: Pick = (v, where) => p.fields(v, { id: same, name: same, type: same, visible: same, locked: same, expanded: same, style, children: (c, w) => p.list(layer)(c, w) }, where);
  const out = p.fields(
    head,
    {
      format: same,
      version: same,
      name: same,
      settings: (x, w) => p.fields(x, { srid: same, lengthDecimals: same, areaDecimals: same, areaUnit: same, angleUnit: same, plotScale: same, workspace: same, drawingFont: same }, w),
      origin: vec,
      homeView: (x, w) => p.fields(x, { minX: same, minY: same, maxX: same, maxY: same }, w),
      layers: p.list(layer),
      activeLayer: same,
      // The objects are the columns' (an empty list here, if any).
      entities: () => [],
      uids: () => [],
      styles: (x, w) => p.fields(x, { items: same, categories: same }, w),
      projectId: same,
      migratedFrom: (x, w) => p.fields(x, { format: same, version: same, sourceSha256: same }, w),
    },
    'belge',
  );
  return { head: out as DrawingHead, dropped: p.dropped };
}

// ── JSON the Rust contracts read bit for bit ────────────────────────────

/**
 * `value` as JSON text in which every number is exact: JSON.stringify
 * writes the shortest round-trip decimal (as serde_json does) but writes −0
 * as 0; here −0 stays −0 (`-0.0`), so a float64 crosses to the Rust
 * contracts bit for bit. A number that is not finite (NaN, ±∞) throws with
 * its place: KCAD v2 cannot hold it (spec §5.3).
 */
export function exactJson(value: unknown): string {
  const path: (string | number)[] = [];
  let negativeZero = false;
  const scan = (v: unknown): void => {
    if (typeof v === 'number') {
      if (!Number.isFinite(v)) throw new KcadError('non_finite', `Çizim KCAD 2 olarak yazılamıyor: ${path.join('/')}: sayı NaN ya da sonsuz; yalnız sonlu sayılar yazılır. Değeri düzeltip yeniden kaydedin.`);
      if (v === 0 && 1 / v < 0) negativeZero = true;
    } else if (Array.isArray(v)) {
      for (let i = 0; i < v.length; i++) {
        path.push(i);
        scan(v[i]);
        path.pop();
      }
    } else if (isObj(v)) {
      for (const key of Object.keys(v)) {
        path.push(key);
        scan(v[key]);
        path.pop();
      }
    }
  };
  scan(value);
  if (!negativeZero) return JSON.stringify(value);
  // A string no drawing holds (NUL and a random part), put where −0 is and then replaced by -0.0.
  const marker = `\u0000kcad-negative-zero-${Math.random().toString(36).slice(2)}\u0000`;
  const text = JSON.stringify(value, (_key, v: unknown) => (typeof v === 'number' && v === 0 && 1 / v < 0 ? marker : v));
  return text.split(JSON.stringify(marker)).join('-0.0');
}

// ── Encoding and decoding ───────────────────────────────────────────────

/** The buffers of columns, each once: what goes in a message's transfer list. */
export function transferables(c: DrawingColumns): ArrayBuffer[] {
  return [...new Set([c.kinds, c.uids, c.ints, c.floats, c.text, c.textLengths].map((a) => a.buffer as ArrayBuffer))];
}

/** The module's progress object, telling `progress`. */
const relay = (progress: (p: KcadProgress) => void): Relay => ({
  step: (stage, done, total) => progress({ stage: stage as 'checking', done, total }),
  project: (name, layers, objects) => progress({ stage: 'project', name, layers, objects }),
});

const quiet = () => {};

/**
 * A `.kcad` v2 file's drawing: its head (JSON) and its objects as columns. A
 * file the reader refuses throws a KcadError with its code. `progress`
 * hears the integrity check, the project and the objects as they are read.
 */
export function decodeWith(m: KcadModule, bytes: Uint8Array, progress: (p: KcadProgress) => void = quiet): PackedDrawing {
  const r = m.decodeKcad(bytes, relay(progress));
  try {
    if (!r.ok) throw new KcadError(r.code, r.message);
    const columns: DrawingColumns = { kinds: r.takeKinds(), uids: r.takeUids(), ints: r.takeInts(), floats: r.takeFloats(), text: r.takeText(), textLengths: r.takeTextLengths() };
    return { head: r.takeHead(), columns };
  } finally {
    r.free();
  }
}

/**
 * A packed drawing as a `.kcad` v2 file's bytes, verified: the module read
 * them back and compared the objects with the columns; here the head the
 * file holds is compared with the head sent (every number with Object.is, so
 * −0 ≠ 0). `progress` hears the objects written and the reading back.
 */
export function encodeWith(m: KcadModule, drawing: PackedDrawing, progress: (p: KcadProgress) => void = quiet): Uint8Array<ArrayBuffer> {
  const c = drawing.columns;
  const r = m.encodeKcad(drawing.head, c.kinds, c.uids, c.ints, c.floats, c.text, c.textLengths, relay(progress));
  try {
    if (!r.ok) throw new KcadError(r.code, r.message);
    const bytes = r.takeBytes() as Uint8Array<ArrayBuffer>;
    const where = diff(JSON.parse(drawing.head), JSON.parse(r.takeHead()), 'belge');
    if (where) throw unverified(where);
    return bytes;
  } finally {
    r.free();
  }
}

const unverified = (what: string) =>
  new KcadError(
    'verify_failed',
    `KCAD v2 baytları geri okununca çizimle aynı çıkmadı (${what}); dosya yazılmadı ve eski dosyaya dokunulmadı. Bu bir yazılım hatasıdır: çizimi başka bir yere kaydetmeyi deneyin ve durumu bildirin.`,
  );

/** Where two drawings differ (a path), or null. The objects' `id`s (slots) are not compared. */
export function difference(a: DocumentSnapshotV2, b: DocumentSnapshotV2): string | null {
  const x = a as unknown as Obj;
  const y = b as unknown as Obj;
  for (const key of new Set([...Object.keys(x), ...Object.keys(y)])) {
    if (key === 'entities') continue;
    const d = diff(x[key], y[key], key);
    if (d) return d;
  }
  if (a.entities.length !== b.entities.length) return 'entities';
  for (let i = 0; i < a.entities.length; i++) {
    const d = diff(a.entities[i], b.entities[i], `entities/${i}`, 'id');
    if (d) return d;
  }
  return null;
}

function diff(a: unknown, b: unknown, where: string, skip?: string): string | null {
  if (typeof a === 'number' || typeof b === 'number') return Object.is(a, b) ? null : where;
  if (Array.isArray(a) || Array.isArray(b)) {
    if (!Array.isArray(a) || !Array.isArray(b) || a.length !== b.length) return where;
    for (let i = 0; i < a.length; i++) {
      const d = diff(a[i], b[i], `${where}/${i}`);
      if (d) return d;
    }
    return null;
  }
  if (isObj(a) || isObj(b)) {
    if (!isObj(a) || !isObj(b)) return where;
    const keys = new Set([...Object.keys(a), ...Object.keys(b)]);
    for (const key of keys) {
      if (key === skip) continue;
      const d = diff(a[key], b[key], `${where}/${key}`);
      if (d) return d;
    }
    return null;
  }
  return a === b ? null : where;
}
