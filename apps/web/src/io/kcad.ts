import type { DocumentSnapshotV2 } from '../contracts/generated/DocumentSnapshotV2';

/**
 * The binary project file, `.kcad` v2 (docs/specs/kcad-v2.md, docs/adr/0025),
 * as the browser runs it. The codec is the shared Rust one (crates/shared/kcad
 * in the formats WASM module); this module is what surrounds it, and it runs
 * in the formats worker (and in process in tests), never on the page:
 *
 * - `sniffDrawing`: what a file is by content (spec §8), so the page knows
 *   whether to hand it to the worker (v2) or read it as v1 text;
 * - `encodeWith`: the drawing reduced to what the contract holds (anything
 *   else is reported, not dropped silently), written as JSON the Rust
 *   contracts read bit for bit (JSON.stringify writes −0 as 0; here it stays
 *   −0), encoded, decoded again and compared with what was sent: only bytes
 *   that read back to the same drawing leave the worker;
 * - `decodeWith`: bytes to the drawing, or the specification's error code and
 *   a Turkish message.
 */

/** The formats module's two KCAD calls (crates/wasm/formats-wasm). */
export interface KcadModule {
  encodeKcad(json: string): Uint8Array;
  decodeKcad(bytes: Uint8Array): Uint8Array;
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
 * A drawing reduced to the contract's fields (`DocumentSnapshotV2`), fresh
 * objects all the way down; what else it held is counted in `dropped`.
 * Opaque parts (project styles, renderers) are the style engine's own and
 * stay whole.
 */
export function project(snapshot: DocumentSnapshotV2): { snapshot: DocumentSnapshotV2; dropped: Dropped } {
  const p = new Projection();
  const vec = (v: unknown, where: string) => p.fields(v, { x: same, y: same }, where);
  const vecs = p.list(vec);
  const ring = (v: unknown, where: string) => p.fields(v, { pts: vecs, bulges: same }, where);
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
  const common = { kind: same, id: same, layerId: same, color: same, attrs: same, label: same, symbol: same };
  const kinds: Record<string, Record<string, Pick>> = {
    point: { p: vec, z: same },
    line: { a: vec, b: vec },
    polyline: { pts: vecs, bulges: same },
    polygon: { pts: vecs, bulges: same, holes: p.list(ring) },
    circle: { c: vec, r: same },
    arc: { c: vec, r: same, a0: same, a1: same },
    ellipse: { c: vec, major: vec, ratio: same, t0: same, t1: same },
    spline: { pts: vecs, closed: same },
    xline: { p: vec, dir: vec },
    ray: { p: vec, dir: vec },
    text: { p: vec, text: same, height: same, rotation: same },
    dimension: { a: vec, b: vec, offset: same, height: same, text: same, style: same, angle: same, c: vec },
    hatch: { ring: vecs, holes: p.list(vecs), pattern: (x, w) => p.fields(x, { type: same, angle: same, spacing: same }, w) },
  };
  const entity = (v: unknown) => {
    const kind = isObj(v) && typeof v.kind === 'string' ? v.kind : '?';
    return p.fields(v, { ...common, ...(kinds[kind] ?? {}) }, kind);
  };
  const s = snapshot as unknown as Obj;
  const out = p.fields(
    s,
    {
      format: same,
      version: same,
      name: same,
      settings: (x, w) => p.fields(x, { srid: same, lengthDecimals: same, areaDecimals: same, areaUnit: same, angleUnit: same, plotScale: same, workspace: same, drawingFont: same }, w),
      origin: vec,
      homeView: (x, w) => p.fields(x, { minX: same, minY: same, maxX: same, maxY: same }, w),
      layers: p.list(layer),
      activeLayer: same,
      entities: (x) => (Array.isArray(x) ? x.map(entity) : x),
      uids: same,
      styles: (x, w) => p.fields(x, { items: same, categories: same }, w),
      projectId: same,
      migratedFrom: (x, w) => p.fields(x, { format: same, version: same, sourceSha256: same }, w),
    },
    'belge',
  );
  return { snapshot: out as DocumentSnapshotV2, dropped: p.dropped };
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

const decoder = new TextDecoder();

type Envelope = { ok: true; document: DocumentSnapshotV2 } | { ok: false; code: string; message: string };

/** The drawing in a `.kcad` v2 file's bytes; a file the reader refuses throws a KcadError with its code. */
export function decodeWith(m: KcadModule, bytes: Uint8Array): DocumentSnapshotV2 {
  const r = JSON.parse(decoder.decode(m.decodeKcad(bytes))) as Envelope;
  if (!r.ok) throw new KcadError(r.code, r.message);
  return r.document;
}

/**
 * A drawing as a `.kcad` v2 file's bytes, read back and compared with the
 * drawing first (every number with Object.is, so −0 ≠ 0; only the objects'
 * slots, which the file does not keep, may differ). `dropped` counts what the
 * drawing held beyond the contract, which the file does not keep either.
 */
export function encodeWith(m: KcadModule, snapshot: DocumentSnapshotV2): { bytes: Uint8Array<ArrayBuffer>; dropped: Dropped } {
  const { snapshot: clean, dropped } = project(snapshot);
  const bytes = m.encodeKcad(exactJson(clean)) as Uint8Array<ArrayBuffer>;
  let back: DocumentSnapshotV2;
  try {
    back = decodeWith(m, bytes);
  } catch (e) {
    throw unverified((e as Error).message);
  }
  const where = difference(clean, back);
  if (where) throw unverified(where);
  return { bytes, dropped };
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
