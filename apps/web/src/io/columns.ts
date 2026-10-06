import type { DocumentSnapshotV2 } from '../contracts/generated/DocumentSnapshotV2';
import type { Entity as ContractEntity } from '../contracts/generated/Entity';
import type { HatchPattern } from '../contracts/generated/HatchPattern';
import type { PatternLine } from '../contracts/generated/PatternLine';
import type { TextRun } from '../contracts/generated/TextRun';
import { KcadError, OBJECT_FIELDS, exactJson, projectHead, type Dropped } from './kcad';

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
export const KINDS = ['point', 'line', 'polyline', 'polygon', 'circle', 'arc', 'ellipse', 'spline', 'xline', 'ray', 'text', 'dimension', 'hatch', 'insert', 'leader', 'table', 'image'] as const;
const KIND = new Map<string, number>(KINDS.map((k, i) => [k, i]));
/** The dimension's kinds in the contract's order (`DimensionStyle::ALL`); KCAD schema 9 added the last five (docs/adr/0147). */
const DIMENSION_STYLES = ['aligned', 'linear', 'angular', 'radius', 'diameter', 'ordinate', 'arcLength', 'jogged', 'azimuth', 'slope'] as const;
/** A text's alignments, numbered as the columns hold them (the contract's `TextAlign::ALL`). */
const TEXT_ALIGNS = ['baselineCenter', 'baselineRight', 'bottomLeft', 'bottomCenter', 'bottomRight', 'middleLeft', 'middleCenter', 'middleRight', 'topLeft', 'topCenter', 'topRight'] as const;
const HATCH_PATTERNS = ['solid', 'lines', 'cross', 'pattern', 'gradient'] as const;
/** A hatch pattern's fields (docs/adr/0186 §1); another is counted as dropped. */
const PATTERN_FIELDS = new Set(['type', 'angle', 'spacing', 'name', 'scale', 'lines', 'gradient']);
/** A gradient's shapes, numbered as the columns hold them (`columns.rs`'s `GRADIENT_SHAPES`, docs/adr/0186). */
const GRADIENT_SHAPES = ['linear', 'cylinder', 'spherical'] as const;
/** A leader's arrowheads, numbered as the columns hold them (the contract's `LeaderArrow::ALL`; the filled arrow is none). */
const LEADER_ARROWS = ['open', 'dot', 'none'] as const;
/** The drawing typefaces, numbered as the columns hold them (the contract's `DrawingFont::ALL`, docs/adr/0183). */
const FONTS = ['barlow', 'arimo', 'overpass', 'quicksand', 'architects-daughter', 'courier-prime', 'plex-mono'] as const;
/** A dimension's arrowheads, numbered as the columns hold them (the contract's `DimensionArrow::ALL`; the tick is none). */
const DIMENSION_ARROWS = ['closed', 'open', 'dot', 'none'] as const;
/** A dimension's units, numbered as the columns hold them (`columns.rs`'s `UNITS`). */
const UNITS = ['mm', 'cm', 'm'] as const;
/** A table's alignments and lines, numbered as the columns hold them (the contract's `TableAlign::ALL`, `TableGrid::ALL`). */
const TABLE_ALIGNS = ['left', 'center', 'right'] as const;
const TABLE_GRIDS = ['outer', 'rows', 'none'] as const;
/** A table's source's kinds, numbered as the columns hold them (docs/adr/0184 §5). */
const SOURCE_KINDS = ['coordinates', 'areas', 'attributes', 'file'] as const;

const COLOR = 1;
const LABEL = 2;
const SYMBOL = 4;
/** The object's own line weight: its first float (docs/adr/0139). */
const WEIGHT = 8;
/**
 * A kind's optional fields from bit 8 up, in the order the Rust module's table
 * names them (point: z; line: za, zb; polyline and polygon: bulges, holes, zs;
 * polygon: parts; text: align, width factor, mask (docs/adr/0145), link, box
 * width, line spacing, runs, text style, font, bold, italic, oblique
 * (docs/adr/0183); dimension: text, style, angle, c, mask, za, zb, dimension
 * style, arrow, arrow size, ext offset, ext beyond, text gap, centred,
 * decimals, unit, prefix, suffix, font; hatch: holes, name, scale, families,
 * gradient, tie (docs/adr/0186); insert: mirror; table:
 * merges, aligns, header, grid, text style, font, bold, italic, oblique,
 * source, frame, docs/adr/0184). A block's definitions travel in the head,
 * not here (docs/adr/0144).
 */
const OPT = Array.from({ length: 19 }, (_, i) => 1 << (8 + i));
/** A multi-line text's run's flags (docs/adr/0182; `columns.rs`'s `RUN_*`): its format, and whether its colour follows. */
const RUN_BOLD = 1;
const RUN_ITALIC = 2;
const RUN_UNDERLINE = 4;
const RUN_SCRIPT = 8 | 16;
const RUN_SUPER = 8;
const RUN_SUB = 16;
const RUN_COLOR = 32;
const runFlags = (r: TextRun): number =>
  (r.bold ? RUN_BOLD : 0) | (r.italic ? RUN_ITALIC : 0) | (r.underline ? RUN_UNDERLINE : 0) | (r.script === 'super' ? RUN_SUPER : r.script === 'sub' ? RUN_SUB : 0) | (r.color !== undefined ? RUN_COLOR : 0);
/** A hole's flags: its bulges, its elevations (docs/adr/0142). */
const HOLE_BULGES = 1;
const HOLE_ELEVATIONS = 2;
/** A part's flags: its bulges, its elevations, its holes (docs/adr/0143). */
const PART_BULGES = 1;
const PART_ELEVATIONS = 2;
const PART_HOLES = 4;

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

  /** `v`'s place in `list` (a name the contract knows); `what` names it in the message when it is not one of them. */
  place(list: readonly string[], v: unknown, field: string, what: string): number {
    const at = typeof v === 'string' ? list.indexOf(v) : -1;
    if (at < 0) throw unwritable('bad_value', `${this.where}/${field}`, `“${String(v)}” ${what} bilinmiyor`);
    return at;
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

  /**
   * A list of elevations, one per vertex (`vertices`, docs/adr/0142): its
   * length, then the numbers, NaN for a vertex without one (`null`; a real
   * elevation is always finite, so the two cannot meet). The file's rules are
   * checked here, so the page says where: as many as the vertices, each a
   * finite number or null.
   */
  /**
   * A polygon's or a part's holes (`field`: `holes`, `parts/0/holes`): their count, then each hole's
   * flags, vertices, bulges and elevations.
   */
  holes(list: unknown, field: string, kind: string): void {
    if (!Array.isArray(list)) throw unwritable('wrong_type', `${this.where}/${field}`, 'ada listesi olmalı');
    const dotted = field.replace(/\/\d+\//g, '.');
    this.int(list.length);
    (list as { pts: unknown[]; bulges?: unknown; zs?: unknown }[]).forEach((h, i) => {
      for (const key in h) if (key !== 'pts' && key !== 'bulges' && key !== 'zs') this.drop(`${kind}.${dotted}.${key}`);
      this.int((h.bulges !== undefined ? HOLE_BULGES : 0) | (h.zs !== undefined ? HOLE_ELEVATIONS : 0));
      this.points(h.pts, `${dotted}.pts`, kind);
      if (h.bulges !== undefined) this.numbers(h.bulges, `${field}/${i}/bulges`);
      if (h.zs !== undefined) this.elevations(h.zs, `${field}/${i}/zs`, h.pts.length);
    });
  }

  elevations(list: unknown, field: string, vertices: number): void {
    if (!Array.isArray(list)) throw unwritable('wrong_type', `${this.where}/${field}`, 'kot listesi olmalı');
    if (list.length !== vertices) throw unwritable('bad_value', `${this.where}/${field}`, `${list.length} kot var ama ${vertices} köşe var; her köşenin bir kotu olmalı (kotsuz köşe için null)`);
    this.int(list.length);
    this.floats.room(list.length);
    for (let i = 0; i < list.length; i++) {
      const z: unknown = list[i];
      if (z === null) this.floats.push(Number.NaN);
      else if (typeof z === 'number' && Number.isFinite(z)) this.floats.push(z);
      else throw unwritable(typeof z === 'number' ? 'non_finite' : 'wrong_type', `${this.where}/${field}/${i}`, typeof z === 'number' ? Packer.badFloat(z)! : 'kot sayı ya da null olmalı');
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
    const known = OBJECT_FIELDS[kind];
    for (const key in e) if (!known.has(key) && (e as unknown as Record<string, unknown>)[key] !== undefined) this.drop(`${kind}.${key}`);
    const flagsAt = this.ints.n + 1;
    const attrs = e.attrs as Record<string, unknown>;
    if (typeof attrs !== 'object' || attrs === null) throw unwritable('wrong_type', `${this.where}/attrs`, 'öznitelikler metin → metin olmalı');
    const keys = Object.keys(attrs).sort(byCodePoint);
    this.int(layer);
    this.int(0);
    this.int(keys.length);
    let flags = 0;
    if (e.lineWeight !== undefined) (flags |= WEIGHT), this.float(e.lineWeight, 'lineWeight');
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
        // A multi-point object's other points (docs/adr/0174): each its flag, place and elevation.
        if (e.parts !== undefined) {
          if (!Array.isArray(e.parts)) throw unwritable('wrong_type', `${this.where}/parts`, 'nokta listesi olmalı');
          flags |= OPT[1];
          this.int(e.parts.length);
          e.parts.forEach((q, i) => {
            for (const key in q) if (key !== 'p' && key !== 'z') this.drop(`${kind}.parts.${key}`);
            this.int(q.z !== undefined ? 1 : 0);
            this.point(q.p, `parts/${i}/p`, kind);
            if (q.z !== undefined) this.float(q.z, `parts/${i}/z`);
          });
        }
        break;
      case 'line':
        this.point(e.a, 'a', kind);
        this.point(e.b, 'b', kind);
        if (e.za !== undefined) (flags |= OPT[0]), this.float(e.za, 'za');
        if (e.zb !== undefined) (flags |= OPT[1]), this.float(e.zb, 'zb');
        break;
      case 'polyline':
      case 'polygon': {
        this.points(e.pts, 'pts', kind);
        if (e.bulges !== undefined) (flags |= OPT[0]), this.numbers(e.bulges, 'bulges');
        if (e.zs !== undefined) (flags |= OPT[2]), this.elevations(e.zs, 'zs', e.pts.length);
        // A polyline's holes are not the contract's: counted as dropped above, never written.
        const holes = e.kind === 'polygon' ? e.holes : undefined;
        if (holes !== undefined) {
          flags |= OPT[1];
          this.holes(holes, 'holes', kind);
        }
        // A multi-part area's or polyline's other parts (docs/adr/0143, 0174): after the holes, each part's flags and lists.
        const parts = e.parts;
        if (parts !== undefined) {
          if (!Array.isArray(parts)) throw unwritable('wrong_type', `${this.where}/parts`, 'parça listesi olmalı');
          flags |= OPT[3];
          this.int(parts.length);
          parts.forEach((p, i) => {
            for (const key in p) if (key !== 'pts' && key !== 'bulges' && key !== 'holes' && key !== 'zs') this.drop(`${kind}.parts.${key}`);
            this.int((p.bulges !== undefined ? PART_BULGES : 0) | (p.zs !== undefined ? PART_ELEVATIONS : 0) | (p.holes !== undefined ? PART_HOLES : 0));
            this.points(p.pts, 'parts.pts', kind);
            if (p.bulges !== undefined) this.numbers(p.bulges, `parts/${i}/bulges`);
            if (p.zs !== undefined) this.elevations(p.zs, `parts/${i}/zs`, p.pts.length);
            if (p.holes !== undefined) this.holes(p.holes, `parts/${i}/holes`, kind);
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
      // docs/adr/0145: the alignment its place in TEXT_ALIGNS, the width factor a float, the mask a flag (only true is written).
      case 'text':
        this.point(e.p, 'p', kind);
        this.float(e.height, 'height');
        this.float(e.rotation, 'rotation');
        this.text(e.text, 'text');
        if (e.align !== undefined) {
          const at = TEXT_ALIGNS.indexOf(e.align);
          if (at < 0) throw unwritable('bad_value', `${this.where}/align`, `“${String(e.align)}” yazı hizası bilinmiyor`);
          flags |= OPT[0];
          this.int(at);
        }
        if (e.widthFactor !== undefined) (flags |= OPT[1]), this.float(e.widthFactor, 'widthFactor');
        if (e.mask === true) flags |= OPT[2];
        else if (e.mask !== undefined) throw unwritable('bad_value', `${this.where}/mask`, 'zemin yalnız true yazılır; zeminsiz yazıda alan yoktur');
        // A linked text's object, as its text, and scale (docs/adr/0175 §4); the codec checks the pair.
        if (e.labelOf !== undefined || e.labelScale !== undefined) {
          flags |= OPT[3];
          this.text(e.labelOf ?? '', 'labelOf');
          this.float(e.labelScale ?? Number.NaN, 'labelScale');
        }
        // A multi-line text's box, line spacing and runs (docs/adr/0182): each run its range, its flags
        // (`runFlags`) and, with RUN_COLOR, its colour; the codec checks them.
        if (e.boxWidth !== undefined) (flags |= OPT[4]), this.float(e.boxWidth, 'boxWidth');
        if (e.lineSpacing !== undefined) (flags |= OPT[5]), this.float(e.lineSpacing, 'lineSpacing');
        if (e.runs !== undefined && e.runs.length > 0) {
          flags |= OPT[6];
          this.int(e.runs.length);
          for (const r of e.runs) {
            this.int(r.start);
            this.int(r.end);
            this.int(runFlags(r));
            if (r.color !== undefined) this.text(r.color, 'runs');
          }
        }
        // Its face (docs/adr/0183 §2): the style's id, the typeface's place, bold and italic as flags (only true is
        // written), the slant; the codec checks them.
        if (e.textStyle !== undefined) (flags |= OPT[7]), this.text(e.textStyle, 'textStyle');
        if (e.font !== undefined) (flags |= OPT[8]), this.int(this.place(FONTS, e.font, 'font', 'yazı tipi'));
        if (e.bold === true) flags |= OPT[9];
        else if (e.bold !== undefined) throw unwritable('bad_value', `${this.where}/bold`, 'kalın yalnız true yazılır; alan yoksa yazı kalın değildir');
        if (e.italic === true) flags |= OPT[10];
        else if (e.italic !== undefined) throw unwritable('bad_value', `${this.where}/italic`, 'italik yalnız true yazılır; alan yoksa yazı italik değildir');
        if (e.oblique !== undefined) (flags |= OPT[11]), this.float(e.oblique, 'oblique');
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
        // docs/adr/0147: the mask a flag (only true is written), a slope's elevations.
        if (e.mask === true) flags |= OPT[4];
        else if (e.mask !== undefined) throw unwritable('bad_value', `${this.where}/mask`, 'zemin yalnız true yazılır; zeminsiz ölçüde alan yoktur');
        if (e.za !== undefined) (flags |= OPT[5]), this.float(e.za, 'za');
        if (e.zb !== undefined) (flags |= OPT[6]), this.float(e.zb, 'zb');
        // Its look (docs/adr/0183 §3), field by field in the contract's order; the codec checks them.
        if (e.dimStyle !== undefined) (flags |= OPT[7]), this.text(e.dimStyle, 'dimStyle');
        if (e.arrow !== undefined) (flags |= OPT[8]), this.int(this.place(DIMENSION_ARROWS, e.arrow, 'arrow', 'ölçü oku'));
        if (e.arrowSize !== undefined) (flags |= OPT[9]), this.float(e.arrowSize, 'arrowSize');
        if (e.extOffset !== undefined) (flags |= OPT[10]), this.float(e.extOffset, 'extOffset');
        if (e.extBeyond !== undefined) (flags |= OPT[11]), this.float(e.extBeyond, 'extBeyond');
        if (e.textGap !== undefined) (flags |= OPT[12]), this.float(e.textGap, 'textGap');
        if (e.textPlace === 'centre') flags |= OPT[13];
        else if (e.textPlace !== undefined) throw unwritable('bad_value', `${this.where}/textPlace`, `“${String(e.textPlace)}” değerin yeri bilinmiyor`);
        if (e.decimals !== undefined) {
          if (!Number.isInteger(e.decimals) || e.decimals < 0) throw unwritable('bad_value', `${this.where}/decimals`, `basamak sayısı ${e.decimals}; negatif olmayan bir tam sayı olmalı`);
          flags |= OPT[14];
          this.int(e.decimals);
        }
        if (e.unit !== undefined) (flags |= OPT[15]), this.int(this.place(UNITS, e.unit, 'unit', 'ölçü birimi'));
        if (e.prefix !== undefined) (flags |= OPT[16]), this.text(e.prefix, 'prefix');
        if (e.suffix !== undefined) (flags |= OPT[17]), this.text(e.suffix, 'suffix');
        if (e.font !== undefined) (flags |= OPT[18]), this.int(this.place(FONTS, e.font, 'font', 'yazı tipi'));
        break;
      }
      case 'hatch': {
        this.points(e.ring, 'ring', kind);
        const p = e.pattern;
        if (typeof p !== 'object' || p === null) throw unwritable('wrong_type', `${this.where}/pattern`, 'desen olmalı');
        for (const key in p) if (!PATTERN_FIELDS.has(key)) this.drop(`${kind}.pattern.${key}`);
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
        // A pattern's name, scale and families, a gradient, the objects it follows (docs/adr/0186).
        if (p.name !== undefined) (flags |= OPT[1]), this.text(p.name, 'pattern/name');
        if (p.scale !== undefined) (flags |= OPT[2]), this.float(p.scale, 'pattern/scale');
        if (p.lines !== undefined) {
          if (!Array.isArray(p.lines)) throw unwritable('wrong_type', `${this.where}/pattern/lines`, 'çizgi ailesi listesi olmalı');
          flags |= OPT[3];
          this.int(p.lines.length);
          p.lines.forEach((l, i) => {
            const at = `pattern/lines/${i}`;
            for (const key in l) if (key !== 'angle' && key !== 'origin' && key !== 'offset' && key !== 'dashes') this.drop(`${kind}.pattern.lines.${key}`);
            const dashes = l.dashes ?? [];
            if (!Array.isArray(dashes)) throw unwritable('wrong_type', `${this.where}/${at}/dashes`, 'kesik listesi olmalı');
            this.int(dashes.length);
            this.float(l.angle, `${at}/angle`);
            for (const [field, pair] of [['origin', l.origin], ['offset', l.offset]] as const) {
              if (!Array.isArray(pair) || pair.length !== 2) throw unwritable('wrong_type', `${this.where}/${at}/${field}`, 'iki sayı olmalı');
              this.float(pair[0], `${at}/${field}/0`);
              this.float(pair[1], `${at}/${field}/1`);
            }
            dashes.forEach((d, k) => this.float(d, `${at}/dashes/${k}`));
          });
        }
        if (p.gradient !== undefined) {
          const g = p.gradient;
          for (const key in g) if (key !== 'shape' && key !== 'inverted' && key !== 'color2') this.drop(`${kind}.pattern.gradient.${key}`);
          flags |= OPT[4];
          this.int(this.place(GRADIENT_SHAPES, g.shape, 'pattern/gradient/shape', 'degrade biçimi'));
          this.int(g.inverted === true ? 1 : 0);
          this.text(g.color2, 'pattern/gradient/color2');
        }
        if (e.assoc !== undefined) {
          const a = e.assoc;
          for (const key in a) if (key !== 'outer' && key !== 'islands' && key !== 'cutouts' && key !== 'seed') this.drop(`${kind}.assoc.${key}`);
          const islands = a.islands ?? [];
          const cutouts = a.cutouts ?? [];
          flags |= OPT[5];
          this.int(islands.length);
          this.int(cutouts.length);
          this.text(a.outer, 'assoc/outer');
          islands.forEach((id, i) => this.text(id, `assoc/islands/${i}`));
          cutouts.forEach((id, i) => this.text(id, `assoc/cutouts/${i}`));
          this.point(a.seed, 'assoc/seed', kind);
        }
        break;
      }
      // docs/adr/0144: p, scale, turn; the block's id as text; mirror a flag (only true is written).
      case 'insert':
        this.point(e.p, 'p', kind);
        this.float(e.scale, 'scale');
        this.float(e.rotation, 'rotation');
        this.text(e.block, 'block');
        if (e.mirror === true) flags |= OPT[0];
        else if (e.mirror !== undefined) throw unwritable('bad_value', `${this.where}/mirror`, 'aynalama yalnız true yazılır; aynalı olmayanda alan yoktur');
        break;
      // docs/adr/0192: p, width, height, turn; the mirror a flag (only true is written); the asset and the file texts;
      // the clip's corners; the opacity.
      case 'image':
        this.point(e.p, 'p', kind);
        this.float(e.width, 'width');
        this.float(e.height, 'height');
        this.float(e.rotation, 'rotation');
        if (e.mirror === true) flags |= OPT[0];
        else if (e.mirror !== undefined) throw unwritable('bad_value', `${this.where}/mirror`, 'aynalama yalnız true yazılır; aynalı olmayanda alan yoktur');
        if (e.asset !== undefined) (flags |= OPT[1]), this.text(e.asset, 'asset');
        if (e.file !== undefined) (flags |= OPT[2]), this.text(e.file, 'file');
        if (e.clip !== undefined) (flags |= OPT[3]), this.points(e.clip, 'clip', kind);
        if (e.opacity !== undefined) (flags |= OPT[4]), this.float(e.opacity, 'opacity');
        break;
      // docs/adr/0146: the vertices, height and turn; the note a text, the arrowhead its place in LEADER_ARROWS, the mask a flag.
      case 'leader':
        this.points(e.pts, 'pts', kind);
        this.float(e.height, 'height');
        this.float(e.rotation, 'rotation');
        if (e.text !== undefined) (flags |= OPT[0]), this.text(e.text, 'text');
        if (e.arrow !== undefined) {
          const at = LEADER_ARROWS.indexOf(e.arrow);
          if (at < 0) throw unwritable('bad_value', `${this.where}/arrow`, `“${String(e.arrow)}” kılavuz oku bilinmiyor`);
          flags |= OPT[1];
          this.int(at);
        }
        if (e.mask === true) flags |= OPT[2];
        else if (e.mask !== undefined) throw unwritable('bad_value', `${this.where}/mask`, 'zemin yalnız true yazılır; zeminsiz kılavuzda alan yoktur');
        break;
      // docs/adr/0184: p, turn, height; the rows' and columns' lists; every cell row with its length and its words;
      // the ranges, the alignments' places, the heading a flag, the lines' place, its face as a text's, the source
      // (its kind's place, then its objects or a file's sheet flag, name and sheet), the frame's width.
      case 'table': {
        this.point(e.p, 'p', kind);
        this.float(e.rotation, 'rotation');
        this.float(e.height, 'height');
        this.numbers(e.rows, 'rows');
        this.numbers(e.columns, 'columns');
        if (!Array.isArray(e.cells)) throw unwritable('wrong_type', `${this.where}/cells`, 'hücre satırları listesi olmalı');
        this.int(e.cells.length);
        e.cells.forEach((row, i) => {
          if (!Array.isArray(row)) throw unwritable('wrong_type', `${this.where}/cells/${i}`, 'hücre listesi olmalı');
          this.int(row.length);
          row.forEach((words, j) => this.text(words, `cells/${i}/${j}`));
        });
        if (e.merges !== undefined && e.merges.length) {
          flags |= OPT[0];
          this.int(e.merges.length);
          for (const m of e.merges) {
            this.int(m.row);
            this.int(m.col);
            this.int(m.rows);
            this.int(m.cols);
          }
        }
        if (e.aligns !== undefined) {
          flags |= OPT[1];
          this.int(e.aligns.length);
          for (const a of e.aligns) this.int(this.place(TABLE_ALIGNS, a, 'aligns', 'sütun hizası'));
        }
        if (e.header === true) flags |= OPT[2];
        else if (e.header !== undefined) throw unwritable('bad_value', `${this.where}/header`, 'başlık yalnız true yazılır; başlıksız tabloda alan yoktur');
        if (e.grid !== undefined) (flags |= OPT[3]), this.int(this.place(TABLE_GRIDS, e.grid, 'grid', 'tablo çizgisi'));
        if (e.textStyle !== undefined) (flags |= OPT[4]), this.text(e.textStyle, 'textStyle');
        if (e.font !== undefined) (flags |= OPT[5]), this.int(this.place(FONTS, e.font, 'font', 'yazı tipi'));
        if (e.bold === true) flags |= OPT[6];
        else if (e.bold !== undefined) throw unwritable('bad_value', `${this.where}/bold`, 'kalın yalnız true yazılır; alan yoksa yazı kalın değildir');
        if (e.italic === true) flags |= OPT[7];
        else if (e.italic !== undefined) throw unwritable('bad_value', `${this.where}/italic`, 'italik yalnız true yazılır; alan yoksa yazı italik değildir');
        if (e.oblique !== undefined) (flags |= OPT[8]), this.float(e.oblique, 'oblique');
        if (e.frame !== undefined) (flags |= OPT[10]), this.float(e.frame, 'frame');
        if (e.source !== undefined) {
          const source = e.source;
          flags |= OPT[9];
          this.int(this.place(SOURCE_KINDS, source.kind, 'source/kind', 'tablo kaynağı'));
          if (source.kind === 'file') {
            this.int(source.sheet !== undefined ? 1 : 0);
            this.text(source.name, 'source/name');
            if (source.sheet !== undefined) this.text(source.sheet, 'source/sheet');
          } else {
            if (!Array.isArray(source.objects)) throw unwritable('wrong_type', `${this.where}/source/objects`, 'nesne kimlikleri listesi olmalı');
            this.int(source.objects.length);
            source.objects.forEach((id, i) => this.text(id, `source/objects/${i}`));
          }
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

  /** The name at the next int's place in `list`; a place past them is broken columns (`what` names it). */
  private at<T extends string>(list: readonly T[], what: string): T {
    const v = this.readInt();
    if (v >= list.length) throw broken(`${what} ${v}`);
    return list[v];
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

  /** A polygon's or a part's holes (their count, then each hole's flags and lists). */
  private holes(): Record<string, unknown>[] {
    const h = this.readInt();
    const holes = [];
    for (let k = 0; k < h; k++) {
      const hf = this.readInt();
      const ring: Record<string, unknown> = { pts: this.pts() };
      if (hf & HOLE_BULGES) ring.bulges = this.nums();
      if (hf & HOLE_ELEVATIONS) ring.zs = this.elevs();
      holes.push(ring);
    }
    return holes;
  }

  /** A list of elevations (its length, then the numbers): NaN is a vertex without one, null. */
  private elevs(): (number | null)[] {
    const n = this.readInt();
    if (this.float + n > this.c.floats.length) throw broken('kot listesi sayılardan uzun');
    const out = new Array<number | null>(n);
    for (let i = 0; i < n; i++) {
      const z = this.c.floats[this.float + i];
      out[i] = Number.isNaN(z) ? null : z;
    }
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
    if (flags & WEIGHT) e.lineWeight = this.num();
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
        if (has(1)) {
          const q = this.readInt();
          const parts = [];
          for (let k = 0; k < q; k++) {
            const pf = this.readInt();
            const part: Record<string, unknown> = { p: this.pt() };
            if (pf & 1) part.z = this.num();
            parts.push(part);
          }
          e.parts = parts;
        }
        break;
      case 'line':
        e.a = this.pt();
        e.b = this.pt();
        if (has(0)) e.za = this.num();
        if (has(1)) e.zb = this.num();
        break;
      case 'polyline':
      case 'polygon':
        e.pts = this.pts();
        if (has(0)) e.bulges = this.nums();
        if (has(2)) e.zs = this.elevs();
        if (has(1)) e.holes = this.holes();
        if (has(3)) {
          const q = this.readInt();
          const parts = [];
          for (let k = 0; k < q; k++) {
            const pf = this.readInt();
            const part: Record<string, unknown> = { pts: this.pts() };
            if (pf & PART_BULGES) part.bulges = this.nums();
            if (pf & PART_ELEVATIONS) part.zs = this.elevs();
            if (pf & PART_HOLES) part.holes = this.holes();
            parts.push(part);
          }
          e.parts = parts;
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
        if (has(0)) e.align = TEXT_ALIGNS[this.readInt()];
        if (has(1)) e.widthFactor = this.num();
        if (has(2)) e.mask = true;
        if (has(3)) {
          e.labelOf = this.readText();
          e.labelScale = this.num();
        }
        if (has(4)) e.boxWidth = this.num();
        if (has(5)) e.lineSpacing = this.num();
        if (has(6))
          e.runs = Array.from({ length: this.readInt() }, () => {
            const r: TextRun = { start: this.readInt(), end: this.readInt() };
            const bits = this.readInt();
            if (bits & RUN_BOLD) r.bold = true;
            if (bits & RUN_ITALIC) r.italic = true;
            if (bits & RUN_UNDERLINE) r.underline = true;
            if ((bits & RUN_SCRIPT) === RUN_SUPER) r.script = 'super';
            else if ((bits & RUN_SCRIPT) === RUN_SUB) r.script = 'sub';
            if (bits & RUN_COLOR) r.color = this.readText();
            return r;
          });
        // docs/adr/0183 §2: the style, the typeface, bold, italic, the slant.
        if (has(7)) e.textStyle = this.readText();
        if (has(8)) e.font = this.at(FONTS, 'yazı tipi');
        if (has(9)) e.bold = true;
        if (has(10)) e.italic = true;
        if (has(11)) e.oblique = this.num();
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
        if (has(4)) e.mask = true;
        if (has(5)) e.za = this.num();
        if (has(6)) e.zb = this.num();
        // docs/adr/0183 §3: the style and the look.
        if (has(7)) e.dimStyle = this.readText();
        if (has(8)) e.arrow = this.at(DIMENSION_ARROWS, 'ölçü oku');
        if (has(9)) e.arrowSize = this.num();
        if (has(10)) e.extOffset = this.num();
        if (has(11)) e.extBeyond = this.num();
        if (has(12)) e.textGap = this.num();
        if (has(13)) e.textPlace = 'centre';
        if (has(14)) e.decimals = this.readInt();
        if (has(15)) e.unit = this.at(UNITS, 'ölçü birimi');
        if (has(16)) e.prefix = this.readText();
        if (has(17)) e.suffix = this.readText();
        if (has(18)) e.font = this.at(FONTS, 'yazı tipi');
        break;
      case 'hatch': {
        e.ring = this.pts();
        const type = this.at(HATCH_PATTERNS, 'tarama deseni');
        const pattern: HatchPattern = { type, angle: this.num(), spacing: this.num() };
        e.pattern = pattern;
        if (has(0)) {
          const h = this.readInt();
          const holes = [];
          for (let k = 0; k < h; k++) holes.push(this.pts());
          e.holes = holes;
        }
        // docs/adr/0186: a pattern's name, scale and families, a gradient, the objects it follows.
        if (has(1)) pattern.name = this.readText();
        if (has(2)) pattern.scale = this.num();
        if (has(3)) {
          const n = this.readInt();
          const lines: PatternLine[] = [];
          for (let k = 0; k < n; k++) {
            const d = this.readInt();
            const angle = this.num();
            const origin: [number, number] = [this.num(), this.num()];
            const offset: [number, number] = [this.num(), this.num()];
            if (this.float + d > this.c.floats.length) throw broken('kesik listesi sayılardan uzun');
            const line: PatternLine = { angle, origin, offset };
            if (d > 0) line.dashes = Array.from({ length: d }, () => this.num());
            lines.push(line);
          }
          pattern.lines = lines;
        }
        if (has(4)) {
          const shape = this.at(GRADIENT_SHAPES, 'degrade biçimi');
          const inverted = this.readInt();
          if (inverted > 1) throw broken(`degradenin ters bayrağı ${inverted}`);
          pattern.gradient = { shape, ...(inverted === 1 && { inverted: true }), color2: this.readText() };
        }
        if (has(5)) {
          const i = this.readInt();
          const k = this.readInt();
          const outer = this.readText();
          const islands = Array.from({ length: i }, () => this.readText());
          const cutouts = Array.from({ length: k }, () => this.readText());
          e.assoc = { outer, ...(i > 0 && { islands }), ...(k > 0 && { cutouts }), seed: this.pt() };
        }
        break;
      }
      case 'insert':
        e.p = this.pt();
        e.scale = this.num();
        e.rotation = this.num();
        e.block = this.readText();
        if (has(0)) e.mirror = true;
        break;
      case 'image':
        e.p = this.pt();
        e.width = this.num();
        e.height = this.num();
        e.rotation = this.num();
        if (has(0)) e.mirror = true;
        if (has(1)) e.asset = this.readText();
        if (has(2)) e.file = this.readText();
        if (has(3)) e.clip = this.pts();
        if (has(4)) e.opacity = this.num();
        break;
      case 'leader':
        e.pts = this.pts();
        e.height = this.num();
        e.rotation = this.num();
        if (has(0)) e.text = this.readText();
        if (has(1)) e.arrow = LEADER_ARROWS[this.readInt()];
        if (has(2)) e.mask = true;
        break;
      case 'table': {
        e.p = this.pt();
        e.rotation = this.num();
        e.height = this.num();
        e.rows = this.nums();
        e.columns = this.nums();
        const r = this.readInt();
        const cells: string[][] = [];
        for (let i = 0; i < r; i++) {
          const n = this.readInt();
          const row: string[] = [];
          for (let j = 0; j < n; j++) row.push(this.readText());
          cells.push(row);
        }
        e.cells = cells;
        if (has(0)) {
          const q = this.readInt();
          const merges = [];
          for (let k = 0; k < q; k++) merges.push({ row: this.readInt(), col: this.readInt(), rows: this.readInt(), cols: this.readInt() });
          e.merges = merges;
        }
        if (has(1)) {
          const a = this.readInt();
          const aligns = [];
          for (let k = 0; k < a; k++) aligns.push(this.at(TABLE_ALIGNS, 'sütun hizası'));
          e.aligns = aligns;
        }
        if (has(2)) e.header = true;
        if (has(3)) e.grid = this.at(TABLE_GRIDS, 'tablo çizgisi');
        if (has(4)) e.textStyle = this.readText();
        if (has(5)) e.font = this.at(FONTS, 'yazı tipi');
        if (has(6)) e.bold = true;
        if (has(7)) e.italic = true;
        if (has(8)) e.oblique = this.num();
        if (has(10)) e.frame = this.num();
        if (has(9)) {
          const kind = this.at(SOURCE_KINDS, 'tablo kaynağı');
          if (kind === 'file') {
            const sheet = this.readInt() === 1;
            const source: Record<string, unknown> = { kind, name: this.readText() };
            if (sheet) source.sheet = this.readText();
            e.source = source;
          } else {
            const k = this.readInt();
            const objects = [];
            for (let i = 0; i < k; i++) objects.push(this.readText());
            e.source = { kind, objects };
          }
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
