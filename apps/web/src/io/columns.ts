import type { DocumentSnapshotV2 } from '../contracts/generated/DocumentSnapshotV2';
import type { Entity as ContractEntity } from '../contracts/generated/Entity';
import { KcadError, exactJson, projectHead, type Dropped } from './kcad';

/**
 * A drawing as typed columns (docs/adr/0030, TODOS.md FILE-15): how the page
 * hands a drawing to the formats worker to save it and takes one back when a
 * file is opened, without a JSON text or a structured copy of every object.
 * The layout is the shared Rust one (crates/shared/kcad/src/columns.rs, whose
 * comment has the table); this is the page's side of it, and the fixtures
 * hold the two to each other (io/columns.test.ts, io/kcad.wasm.test.ts).
 *
 * - `packDrawing`: the page's objects into six typed arrays (their buffers
 *   are transferred to the worker, not copied), the rest of the drawing as
 *   the contract's JSON; a field the contract does not know is counted in
 *   `dropped`, never written silently, and a value of the wrong type stops
 *   the save with its place.
 * - `ColumnsReader`: the columns a file was read into, back into objects,
 *   one at a time, so the page can check them and show progress in chunks.
 */

export interface DrawingColumns {
  kinds: Uint8Array;
  uids: Uint8Array;
  ints: Uint32Array;
  floats: Float64Array;
  /** UTF-16 code units of every text, one after another. */
  text: Uint16Array;
  textLengths: Uint32Array;
}

/** A drawing crossing to or from the worker: everything but the objects as JSON, the objects as columns. */
export interface PackedDrawing {
  head: string;
  columns: DrawingColumns;
}

/** The drawing without its objects, as `DocumentSnapshotV2` has it. */
export type DrawingHead = Omit<DocumentSnapshotV2, 'entities' | 'uids'>;

/** An object as the page holds it: the contract's fields, its slot and, in a drawing, its persistent id. */
export type PageEntity = ContractEntity & { uid?: string };

/** The kinds, numbered as `kinds` holds them. */
export const KINDS = ['point', 'line', 'polyline', 'polygon', 'circle', 'arc', 'ellipse', 'spline', 'xline', 'ray', 'text', 'dimension', 'hatch'] as const;
const KIND = new Map<string, number>(KINDS.map((k, i) => [k, i]));
const DIMENSION_STYLES = ['aligned', 'linear', 'angular', 'radius', 'diameter'] as const;
const HATCH_PATTERNS = ['solid', 'lines', 'cross'] as const;

const COLOR = 1;
const LABEL = 2;
const SYMBOL = 4;
const OPT = [1 << 8, 1 << 9, 1 << 10, 1 << 11] as const;
const HOLE_BULGES = 1;

/**
 * The fields each kind may have, besides the ones every object has: what the
 * contract holds. Anything else an object carries is reported, not written.
 */
const COMMON = ['kind', 'id', 'uid', 'layerId', 'color', 'attrs', 'label', 'symbol'];
const FIELDS: Record<string, ReadonlySet<string>> = Object.fromEntries(
  Object.entries({
    point: ['p', 'z'],
    line: ['a', 'b'],
    polyline: ['pts', 'bulges'],
    polygon: ['pts', 'bulges', 'holes'],
    circle: ['c', 'r'],
    arc: ['c', 'r', 'a0', 'a1'],
    ellipse: ['c', 'major', 'ratio', 't0', 't1'],
    spline: ['pts', 'closed'],
    xline: ['p', 'dir'],
    ray: ['p', 'dir'],
    text: ['p', 'text', 'height', 'rotation'],
    dimension: ['a', 'b', 'offset', 'height', 'text', 'style', 'angle', 'c'],
    hatch: ['ring', 'holes', 'pattern'],
  }).map(([k, f]) => [k, new Set([...COMMON, ...f])]),
);

/**
 * Orders text as UTF-8 bytes (code points) do: the contract's attribute map
 * is ordered so, and the columns list attributes in its order. JavaScript's
 * own order would differ for keys like "10" and "2" (an object lists
 * integer-like keys first) and for characters beyond U+FFFF.
 */
export function byCodePoint(a: string, b: string): number {
  const n = Math.min(a.length, b.length);
  for (let i = 0; i < n; i++) {
    let x = a.charCodeAt(i);
    let y = b.charCodeAt(i);
    if (x === y) continue;
    // Surrogates (U+10000 and up) come after U+E000–U+FFFF in code point order.
    x = x >= 0xd800 && x <= 0xdfff ? x + 0x2000 : x >= 0xe000 ? x - 0x800 : x;
    y = y >= 0xd800 && y <= 0xdfff ? y + 0x2000 : y >= 0xe000 ? y - 0x800 : y;
    return x - y;
  }
  return a.length - b.length;
}

// ── Growable typed arrays ───────────────────────────────────────────────

class Grow<T extends Uint8Array | Uint16Array | Uint32Array | Float64Array> {
  a: T;
  n = 0;
  private readonly make: (n: number) => T;
  constructor(make: (n: number) => T, size = 1024) {
    this.make = make;
    this.a = make(size);
  }
  room(k: number): void {
    if (this.n + k <= this.a.length) return;
    const b = this.make(Math.max(this.a.length * 2, this.n + k));
    b.set(this.a.subarray(0, this.n) as never);
    this.a = b;
  }
  push(v: number): void {
    if (this.n === this.a.length) this.room(1);
    this.a[this.n++] = v;
  }
  /** Exactly what was written: a view when little room is left over, else a copy (the buffer is transferred whole). */
  done(): T {
    const view = this.a.subarray(0, this.n) as T;
    return this.n >= this.a.length * 0.8 ? view : (view.slice() as T);
  }
}

// ── Packing ─────────────────────────────────────────────────────────────

const unwritable = (code: string, where: string, what: string) => new KcadError(code, `Çizim KCAD 2 olarak yazılamıyor: ${where}: ${what}. Değeri düzeltip yeniden kaydedin.`);

const HEX = new Int8Array(128).fill(-1);
for (let i = 0; i < 16; i++) HEX['0123456789abcdef'.charCodeAt(i)] = i;

class Packer {
  readonly kinds = new Grow((n) => new Uint8Array(n));
  readonly uids = new Grow((n) => new Uint8Array(n), 16 * 1024);
  readonly ints = new Grow((n) => new Uint32Array(n));
  readonly floats = new Grow((n) => new Float64Array(n), 8192);
  /** Every text's UTF-16 code units. */
  readonly units = new Grow((n) => new Uint16Array(n), 8192);
  readonly lengths = new Grow((n) => new Uint32Array(n));
  readonly dropped: Dropped = {};
  /** The object being packed (its index), for messages: `entities/12/p/x`. Built only when one is needed. */
  index = 0;

  get where(): string {
    return `entities/${this.index}`;
  }

  drop(what: string): void {
    this.dropped[what] = (this.dropped[what] ?? 0) + 1;
  }

  int(v: number): void {
    this.ints.push(v);
  }

  /** Why `v` cannot be written as a float, or null. */
  private static badFloat(v: unknown): string | null {
    if (typeof v !== 'number') return 'sayı olmalı';
    return Number.isFinite(v) ? null : 'sayı NaN ya da sonsuz; yalnız sonlu sayılar yazılır';
  }

  float(v: unknown, field: string): void {
    if (typeof v !== 'number' || !Number.isFinite(v)) throw unwritable(typeof v === 'number' ? 'non_finite' : 'wrong_type', `${this.where}/${field}`, Packer.badFloat(v)!);
    this.floats.push(v);
  }

  /**
   * A point, `x` then `y`. `field` names it (`pts`, `holes.pts`) and `at` its
   * place in a list (-1: not in one); both only make a message, when needed.
   */
  point(p: unknown, field: string, kind: string, at = -1): void {
    const q = p as { x?: unknown; y?: unknown } | null;
    const x = typeof q === 'object' && q !== null ? q.x : undefined;
    const y = typeof q === 'object' && q !== null ? q.y : undefined;
    if (typeof x !== 'number' || typeof y !== 'number' || !Number.isFinite(x) || !Number.isFinite(y)) throw this.badPoint(q, field, at);
    const f = this.floats;
    if (f.n + 2 > f.a.length) f.room(2);
    f.a[f.n] = x;
    f.a[f.n + 1] = y;
    f.n += 2;
    for (const key in q) if (key !== 'x' && key !== 'y') this.drop(`${kind}.${field}.${key}`);
  }

  private badPoint(q: unknown, field: string, at: number): KcadError {
    const place = `${this.where}/${field.replaceAll('.', '/')}${at >= 0 ? `/${at}` : ''}`;
    if (typeof q !== 'object' || q === null) return unwritable('wrong_type', place, 'nokta ({x, y}) olmalı');
    const p = q as { x?: unknown; y?: unknown };
    const axis = Packer.badFloat(p.x) ? 'x' : 'y';
    const v = axis === 'x' ? p.x : p.y;
    return unwritable(typeof v === 'number' ? 'non_finite' : 'wrong_type', `${place}/${axis}`, Packer.badFloat(v)!);
  }

  /** A list of points: its length, then its coordinates. */
  points(list: unknown, field: string, kind: string): void {
    if (!Array.isArray(list)) throw unwritable('wrong_type', `${this.where}/${field.replaceAll('.', '/')}`, 'nokta listesi olmalı');
    this.int(list.length);
    this.floats.room(list.length * 2);
    for (let i = 0; i < list.length; i++) this.point(list[i], field, kind, i);
  }

  /** A list of numbers (bulges): its length, then the numbers. */
  numbers(list: unknown, field: string): void {
    if (!Array.isArray(list)) throw unwritable('wrong_type', `${this.where}/${field}`, 'sayı listesi olmalı');
    this.int(list.length);
    for (let i = 0; i < list.length; i++) {
      const v = list[i];
      if (typeof v !== 'number' || !Number.isFinite(v)) throw unwritable(typeof v === 'number' ? 'non_finite' : 'wrong_type', `${this.where}/${field}/${i}`, Packer.badFloat(v)!);
      this.floats.push(v);
    }
  }

  text(s: unknown, field: string): void {
    if (typeof s !== 'string') throw unwritable('wrong_type', `${this.where}/${field}`, 'metin olmalı');
    const t = this.units;
    t.room(s.length);
    for (let i = 0; i < s.length; i++) t.a[t.n + i] = s.charCodeAt(i);
    t.n += s.length;
    this.lengths.push(s.length);
  }

  uid(uid: unknown): void {
    const u = this.uids;
    u.room(16);
    const bad = () => unwritable('bad_value', `${this.where}/uid`, `kalıcı kimlik “${String(uid)}” küçük harfli, tireli bir UUID değil`);
    if (typeof uid !== 'string' || uid.length !== 36) throw bad();
    let byte = 0;
    for (let i = 0; i < 36; i++) {
      const c = uid.charCodeAt(i);
      if (i === 8 || i === 13 || i === 18 || i === 23) {
        if (c !== 0x2d) throw bad();
        continue;
      }
      const d = c < 128 ? HEX[c] : -1;
      if (d < 0) throw bad();
      if (byte & 1) u.a[u.n + (byte >> 1)] |= d;
      else u.a[u.n + (byte >> 1)] = d << 4;
      byte++;
    }
    u.n += 16;
  }

  object(e: PageEntity, index: number, layer: number): void {
    const kind = e.kind as string;
    const k = KIND.get(kind);
    this.index = index;
    if (k === undefined) throw unwritable('unknown_kind', this.where, `“${kind}” nesne türü bilinmiyor`);
    this.kinds.push(k);
    this.uid(e.uid);
    const known = FIELDS[kind];
    for (const key in e) if (!known.has(key) && (e as unknown as Record<string, unknown>)[key] !== undefined) this.drop(`${kind}.${key}`);
    const flagsAt = this.ints.n + 1;
    const attrs = e.attrs as Record<string, unknown>;
    if (typeof attrs !== 'object' || attrs === null) throw unwritable('wrong_type', `${this.where}/attrs`, 'öznitelikler metin → metin olmalı');
    const keys = Object.keys(attrs).sort(byCodePoint);
    this.int(layer);
    this.int(0);
    this.int(keys.length);
    let flags = 0;
    if (e.color !== undefined) (flags |= COLOR), this.text(e.color, 'color');
    if (e.label !== undefined) (flags |= LABEL), this.text(e.label, 'label');
    if (e.symbol !== undefined) (flags |= SYMBOL), this.text(e.symbol, 'symbol');
    for (const key of keys) {
      this.text(key, 'attrs');
      this.text(attrs[key], `attrs/${key}`);
    }
    flags |= this.geometry(e, kind);
    this.ints.a[flagsAt] = flags;
  }

  /** The kind's own fields, in the layout's order; returns their flags. */
  private geometry(e: PageEntity, kind: string): number {
    let flags = 0;
    switch (e.kind) {
      case 'point':
        this.point(e.p, 'p', kind);
        if (e.z !== undefined) (flags |= OPT[0]), this.float(e.z, 'z');
        break;
      case 'line':
        this.point(e.a, 'a', kind);
        this.point(e.b, 'b', kind);
        break;
      case 'polyline':
      case 'polygon': {
        this.points(e.pts, 'pts', kind);
        if (e.bulges !== undefined) (flags |= OPT[0]), this.numbers(e.bulges, 'bulges');
        // A polyline's holes are not the contract's: counted as dropped above, never written.
        const holes = e.kind === 'polygon' ? e.holes : undefined;
        if (holes !== undefined) {
          if (!Array.isArray(holes)) throw unwritable('wrong_type', `${this.where}/holes`, 'ada listesi olmalı');
          flags |= OPT[1];
          this.int(holes.length);
          holes.forEach((h, i) => {
            for (const key in h) if (key !== 'pts' && key !== 'bulges') this.drop(`${kind}.holes.${key}`);
            this.int(h.bulges !== undefined ? HOLE_BULGES : 0);
            this.points(h.pts, 'holes.pts', kind);
            if (h.bulges !== undefined) this.numbers(h.bulges, `holes/${i}/bulges`);
          });
        }
        break;
      }
      case 'circle':
        this.point(e.c, 'c', kind);
        this.float(e.r, 'r');
        break;
      case 'arc':
        this.point(e.c, 'c', kind);
        this.float(e.r, 'r');
        this.float(e.a0, 'a0');
        this.float(e.a1, 'a1');
        break;
      case 'ellipse':
        this.point(e.c, 'c', kind);
        this.point(e.major, 'major', kind);
        this.float(e.ratio, 'ratio');
        this.float(e.t0, 't0');
        this.float(e.t1, 't1');
        break;
      case 'spline':
        this.points(e.pts, 'pts', kind);
        if (typeof e.closed !== 'boolean') throw unwritable('wrong_type', `${this.where}/closed`, 'doğru/yanlış olmalı');
        this.int(e.closed ? 1 : 0);
        break;
      case 'xline':
      case 'ray':
        this.point(e.p, 'p', kind);
        this.point(e.dir, 'dir', kind);
        break;
      case 'text':
        this.point(e.p, 'p', kind);
        this.float(e.height, 'height');
        this.float(e.rotation, 'rotation');
        this.text(e.text, 'text');
        break;
      case 'dimension': {
        this.point(e.a, 'a', kind);
        this.point(e.b, 'b', kind);
        this.float(e.offset, 'offset');
        this.float(e.height, 'height');
        if (e.text !== undefined) (flags |= OPT[0]), this.text(e.text, 'text');
        if (e.style !== undefined) {
          const at = DIMENSION_STYLES.indexOf(e.style);
          if (at < 0) throw unwritable('bad_value', `${this.where}/style`, `“${e.style}” ölçü türü bilinmiyor`);
          flags |= OPT[1];
          this.int(at);
        }
        if (e.angle !== undefined) (flags |= OPT[2]), this.float(e.angle, 'angle');
        if (e.c !== undefined) (flags |= OPT[3]), this.point(e.c, 'c', kind);
        break;
      }
      case 'hatch': {
        this.points(e.ring, 'ring', kind);
        const p = e.pattern;
        if (typeof p !== 'object' || p === null) throw unwritable('wrong_type', `${this.where}/pattern`, 'desen olmalı');
        for (const key in p) if (key !== 'type' && key !== 'angle' && key !== 'spacing') this.drop(`${kind}.pattern.${key}`);
        const at = HATCH_PATTERNS.indexOf(p.type);
        if (at < 0) throw unwritable('bad_value', `${this.where}/pattern/type`, `“${String(p.type)}” desen türü bilinmiyor`);
        this.int(at);
        this.float(p.angle, 'pattern/angle');
        this.float(p.spacing, 'pattern/spacing');
        if (e.holes !== undefined) {
          if (!Array.isArray(e.holes)) throw unwritable('wrong_type', `${this.where}/holes`, 'ada listesi olmalı');
          flags |= OPT[0];
          this.int(e.holes.length);
          for (const ring of e.holes) this.points(ring, 'holes', kind);
        }
        break;
      }
    }
    return flags;
  }

  columns(): DrawingColumns {
    return { kinds: this.kinds.done(), uids: this.uids.done(), ints: this.ints.done(), floats: this.floats.done(), text: this.units.done(), textLengths: this.lengths.done() };
  }
}

/**
 * A drawing as the formats worker takes it to save (see the module comment):
 * `head` is the drawing without its objects, `entities` its objects in
 * document order, each with its persistent id. Runs in one turn, so it is the
 * drawing of that moment (its revision); the buffers are then the worker's.
 * A value the file cannot hold stops it with a KcadError that names its place.
 */
export function packDrawing(head: DrawingHead, entities: Iterable<PageEntity>): { drawing: PackedDrawing; dropped: Dropped } {
  const projected = projectHead(head);
  const p = new Packer();
  Object.assign(p.dropped, projected.dropped);
  const list = Array.isArray(entities) ? (entities as PageEntity[]) : [...entities];
  // The layer table: layer ids in the order the objects first use them, as the Rust side lists them.
  const table = new Map<string, number>();
  for (const e of list) if (!table.has(e.layerId)) table.set(e.layerId, table.size);
  p.int(table.size);
  for (const id of table.keys()) p.text(id, 'layerId');
  for (let i = 0; i < list.length; i++) p.object(list[i], i, table.get(list[i].layerId)!);
  return { drawing: { head: exactJson({ ...projected.head, entities: [], uids: [] }), columns: p.columns() }, dropped: p.dropped };
}

// ── Reading ─────────────────────────────────────────────────────────────

const broken = (what: string) =>
  new KcadError('bad_columns', `Dosya biçim modülünden bozuk bir çizim geldi (${what}); açık çizime dokunulmadı. Bu bir yazılım hatasıdır: sayfayı yenileyip yeniden deneyin ve durumu bildirin.`);

const hex = [...Array(256)].map((_, i) => i.toString(16).padStart(2, '0'));

/**
 * The objects of columns a file was read into, one at a time (`next`), as
 * the contract's JSON form has them: slots 1, 2, 3 … in file order, each with
 * its persistent id. The texts are decoded once; each object's are slices.
 */
export class ColumnsReader {
  readonly count: number;
  private readonly c: DrawingColumns;
  private readonly all: string;
  private readonly layers: string[] = [];
  private i = 0;
  private int = 0;
  private float = 0;
  private unit = 0;
  private texts = 0;

  constructor(columns: DrawingColumns) {
    this.c = columns;
    this.count = columns.kinds.length;
    if (columns.uids.length !== this.count * 16) throw broken('kimlikler nesne sayısıyla uyuşmuyor');
    const t = columns.text;
    this.all = new TextDecoder('utf-16le').decode(new Uint8Array(t.buffer, t.byteOffset, t.byteLength));
    const n = this.readInt();
    for (let i = 0; i < n; i++) this.layers.push(this.readText());
  }

  /** Whether every object was read and every stream to its end. */
  get done(): boolean {
    const c = this.c;
    return this.i === this.count && this.int === c.ints.length && this.float === c.floats.length && this.unit === c.text.length && this.texts === c.textLengths.length;
  }

  private readInt(): number {
    if (this.int >= this.c.ints.length) throw broken('tam sayılar erken bitti');
    return this.c.ints[this.int++];
  }

  private num(): number {
    if (this.float >= this.c.floats.length) throw broken('sayılar erken bitti');
    return this.c.floats[this.float++];
  }

  private pt(): { x: number; y: number } {
    const x = this.num();
    return { x, y: this.num() };
  }

  private pts(): { x: number; y: number }[] {
    const n = this.readInt();
    if (this.float + 2 * n > this.c.floats.length) throw broken('nokta listesi sayılardan uzun');
    const out = new Array<{ x: number; y: number }>(n);
    for (let i = 0; i < n; i++) out[i] = this.pt();
    return out;
  }

  private nums(): number[] {
    const n = this.readInt();
    if (this.float + n > this.c.floats.length) throw broken('sayı listesi sayılardan uzun');
    const out = Array.from(this.c.floats.subarray(this.float, this.float + n));
    this.float += n;
    return out;
  }

  private readText(): string {
    if (this.texts >= this.c.textLengths.length) throw broken('metinler erken bitti');
    const n = this.c.textLengths[this.texts++];
    if (this.unit + n > this.c.text.length) throw broken('metin uzunlukları metinden uzun');
    const s = this.all.slice(this.unit, this.unit + n);
    this.unit += n;
    return s;
  }

  private uid(i: number): string {
    const u = this.c.uids;
    const o = i * 16;
    let s = '';
    for (let k = 0; k < 16; k++) {
      if (k === 4 || k === 6 || k === 8 || k === 10) s += '-';
      s += hex[u[o + k]];
    }
    return s;
  }

  /** The next object, with its slot and persistent id. */
  next(): ContractEntity & { uid: string } {
    if (this.i >= this.count) throw broken('nesneler bitti');
    const i = this.i++;
    const kind = KINDS[this.c.kinds[i]];
    if (!kind) throw broken(`${i + 1}. nesnenin türü ${this.c.kinds[i]}`);
    const layerId = this.layers[this.readInt()];
    if (layerId === undefined) throw broken(`${i + 1}. nesnenin katmanı tabloda yok`);
    const flags = this.readInt();
    const n = this.readInt();
    const e: Record<string, unknown> = { kind, id: i + 1, uid: this.uid(i), layerId };
    if (flags & COLOR) e.color = this.readText();
    if (flags & LABEL) e.label = this.readText();
    if (flags & SYMBOL) e.symbol = this.readText();
    const attrs: Record<string, string> = {};
    for (let k = 0; k < n; k++) {
      const key = this.readText();
      attrs[key] = this.readText();
    }
    e.attrs = attrs;
    const has = (bit: number) => (flags & OPT[bit]) !== 0;
    switch (kind) {
      case 'point':
        e.p = this.pt();
        if (has(0)) e.z = this.num();
        break;
      case 'line':
        e.a = this.pt();
        e.b = this.pt();
        break;
      case 'polyline':
      case 'polygon':
        e.pts = this.pts();
        if (has(0)) e.bulges = this.nums();
        if (has(1)) {
          const h = this.readInt();
          const holes = [];
          for (let k = 0; k < h; k++) {
            const hf = this.readInt();
            const ring: Record<string, unknown> = { pts: this.pts() };
            if (hf & HOLE_BULGES) ring.bulges = this.nums();
            holes.push(ring);
          }
          e.holes = holes;
        }
        break;
      case 'circle':
        e.c = this.pt();
        e.r = this.num();
        break;
      case 'arc':
        e.c = this.pt();
        e.r = this.num();
        e.a0 = this.num();
        e.a1 = this.num();
        break;
      case 'ellipse':
        e.c = this.pt();
        e.major = this.pt();
        e.ratio = this.num();
        e.t0 = this.num();
        e.t1 = this.num();
        break;
      case 'spline':
        e.pts = this.pts();
        e.closed = this.readInt() === 1;
        break;
      case 'xline':
      case 'ray':
        e.p = this.pt();
        e.dir = this.pt();
        break;
      case 'text':
        e.p = this.pt();
        e.height = this.num();
        e.rotation = this.num();
        e.text = this.readText();
        break;
      case 'dimension':
        e.a = this.pt();
        e.b = this.pt();
        e.offset = this.num();
        e.height = this.num();
        if (has(0)) e.text = this.readText();
        if (has(1)) e.style = DIMENSION_STYLES[this.readInt()];
        if (has(2)) e.angle = this.num();
        if (has(3)) e.c = this.pt();
        break;
      case 'hatch': {
        e.ring = this.pts();
        const type = HATCH_PATTERNS[this.readInt()];
        e.pattern = { type, angle: this.num(), spacing: this.num() };
        if (has(0)) {
          const h = this.readInt();
          const holes = [];
          for (let k = 0; k < h; k++) holes.push(this.pts());
          e.holes = holes;
        }
        break;
      }
    }
    return e as unknown as ContractEntity & { uid: string };
  }
}

/**
 * A packed drawing as the contract's JSON form (`DocumentSnapshotV2`): what
 * tests compare with the fixtures' drawings. The page reads a file in chunks
 * instead (app/drawingFile.ts).
 */
export function unpackSnapshot(d: PackedDrawing): DocumentSnapshotV2 {
  const head = JSON.parse(d.head) as DocumentSnapshotV2;
  const r = new ColumnsReader(d.columns);
  const entities: ContractEntity[] = [];
  const uids: string[] = [];
  for (let i = 0; i < r.count; i++) {
    const { uid, ...e } = r.next();
    entities.push(e as ContractEntity);
    uids.push(uid);
  }
  if (!r.done) throw broken('nesnelerden sonra fazladan değer var');
  return { ...head, entities, uids };
}
