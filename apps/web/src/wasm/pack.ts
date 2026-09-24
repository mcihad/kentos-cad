/**
 * Objects packed as numbers for the Rust geometry store (docs/adr/0008:
 * points cross as Float64Array; crates/shared/geometry-core/src/store/pack.rs has
 * the layout). A JSON array of 80 000 parcels is 26 MB and made the core
 * build millions of small values before reading one object; packed, it is
 * one run of float64s (ids and coordinates bit for bit, −0 and NaN
 * included) and a short list of strings (layer ids, texts). The store
 * answers a move, copy or paste in the same layout (`unpackEntities`).
 * This only packs and reads: no coordinate is computed here.
 */

const KIND: Record<string, number> = { point: 0, line: 1, polyline: 2, polygon: 3, circle: 4, arc: 5, ellipse: 6, xline: 7, ray: 8, spline: 9, text: 10, dimension: 11, hatch: 12 };
/** Kinds by their number (`KIND` the other way). */
const KINDS = ['point', 'line', 'polyline', 'polygon', 'circle', 'arc', 'ellipse', 'xline', 'ray', 'spline', 'text', 'dimension', 'hatch'] as const;

interface XY {
  x: number;
  y: number;
}

/** What the packer reads of an object (the model's Entity; this layer cannot import it). */
type Fields = Record<string, unknown> & { id: number; kind: string };

/** An object's geometry: its kind and the kind's fields (the model's EntityGeometry). */
export type Geometry = Record<string, unknown> & { kind: string };

export interface Packed {
  nums: Float64Array;
  /** JSON array of the strings the numbers point at. */
  strings: string;
}

export function packEntities(list: Iterable<object>): Packed {
  const out: number[] = [];
  const strings: string[] = [];
  const index = new Map<string, number>();
  const str = (s: unknown): number => {
    if (typeof s !== 'string') return -1;
    let i = index.get(s);
    if (i === undefined) {
      i = strings.length;
      strings.push(s);
      index.set(s, i);
    }
    return i;
  };
  const num = (v: unknown) => out.push(typeof v === 'number' ? v : NaN);
  const pt = (p: unknown) => {
    const q = p as XY | null | undefined;
    num(q?.x);
    num(q?.y);
  };
  const points = (ps: unknown) => {
    const a = Array.isArray(ps) ? (ps as XY[]) : [];
    out.push(a.length);
    for (const p of a) {
      num(p?.x);
      num(p?.y);
    }
  };
  const values = (vs: unknown) => {
    if (!Array.isArray(vs)) {
      out.push(-1);
      return;
    }
    out.push(vs.length);
    for (const v of vs) num(v);
  };
  const path = (e: Fields) => {
    points(e.pts);
    values(e.bulges);
    const holes = e.holes as { pts?: unknown; bulges?: unknown }[] | undefined;
    if (!Array.isArray(holes)) {
      out.push(-1);
      return;
    }
    out.push(holes.length);
    for (const h of holes) {
      points(h?.pts);
      values(h?.bulges);
    }
  };
  for (const e of list as Iterable<Fields>) {
    const kind = KIND[e.kind];
    if (kind === undefined) throw new Error(`Geometri deposu “${String(e.kind)}” türünü tanımıyor.`);
    num(e.id);
    out.push(str(e.layerId), e.label ? 1 : 0, kind);
    switch (kind) {
      case 0:
        pt(e.p);
        out.push(e.z == null ? 0 : 1);
        num(e.z);
        break;
      case 1:
        pt(e.a);
        pt(e.b);
        break;
      case 2:
      case 3:
        path(e);
        break;
      case 4:
        pt(e.c);
        num(e.r);
        break;
      case 5:
        pt(e.c);
        num(e.r);
        num(e.a0);
        num(e.a1);
        break;
      case 6:
        pt(e.c);
        pt(e.major);
        num(e.ratio);
        num(e.t0);
        num(e.t1);
        break;
      case 7:
      case 8:
        pt(e.p);
        pt(e.dir);
        break;
      case 9:
        points(e.pts);
        out.push(e.closed ? 1 : 0);
        break;
      case 10:
        pt(e.p);
        num(e.height);
        num(e.rotation);
        out.push(str(e.text));
        break;
      case 11:
        pt(e.a);
        pt(e.b);
        num(e.offset);
        num(e.height);
        out.push(str(e.text), str(e.style), e.angle == null ? 0 : 1);
        num(e.angle);
        out.push(e.c == null ? 0 : 1);
        pt(e.c);
        break;
      case 12: {
        points(e.ring);
        const holes = e.holes as unknown[] | undefined;
        if (!Array.isArray(holes)) out.push(-1);
        else {
          out.push(holes.length);
          for (const h of holes) points(h);
        }
        const pattern = e.pattern as { type?: unknown; angle?: unknown; spacing?: unknown } | undefined;
        out.push(str(pattern?.type));
        num(pattern?.angle);
        num(pattern?.spacing);
        break;
      }
    }
  }
  return { nums: Float64Array.from(out), strings: JSON.stringify(strings) };
}

/** An object read back from the layout. */
export interface Unpacked {
  id: number;
  layerId: string;
  /** Whether it carries a label (the layout keeps the flag, not the text). */
  labelled: boolean;
  /**
   * Its kind and the kind's fields, in the order the core's JSON writes an
   * entity's geometry; a field left out is absent, as after JSON.
   */
  geometry: Geometry;
}

/**
 * Objects in the packed layout read back (`CoreStore.transformPacked`
 * answers with it): the other direction of `packEntities`. Numbers come
 * out as they went in, bit for bit, −0 and NaN included.
 */
export function unpackEntities(p: Packed): Unpacked[] {
  const nums = p.nums;
  const strings = JSON.parse(p.strings) as string[];
  const out: Unpacked[] = [];
  let at = 0;
  const num = () => nums[at++];
  const flag = () => nums[at++] !== 0;
  /** A string the layout points at; −1 is none. */
  const str = (): string | undefined => {
    const i = nums[at++];
    return i < 0 ? undefined : strings[i];
  };
  const pt = (): XY => {
    const x = nums[at++];
    return { x, y: nums[at++] };
  };
  const points = (): XY[] => {
    const n = nums[at++];
    const a = new Array<XY>(n);
    for (let i = 0; i < n; i++) a[i] = pt();
    return a;
  };
  const values = (): number[] | undefined => {
    const n = nums[at++];
    if (n < 0) return undefined;
    const v = Array.from(nums.subarray(at, at + n));
    at += n;
    return v;
  };
  const rings = (): { pts: XY[]; bulges?: number[] }[] | undefined => {
    const n = nums[at++];
    if (n < 0) return undefined;
    const list = new Array<{ pts: XY[]; bulges?: number[] }>(n);
    for (let i = 0; i < n; i++) {
      const ring: { pts: XY[]; bulges?: number[] } = { pts: points() };
      const bulges = values();
      if (bulges) ring.bulges = bulges;
      list[i] = ring;
    }
    return list;
  };
  while (at < nums.length) {
    const id = num();
    const layerId = str() ?? '';
    const labelled = flag();
    const code = num();
    const kind = KINDS[code];
    let g: Geometry;
    switch (kind) {
      case 'point': {
        const p = pt();
        const hasZ = flag();
        const z = num();
        g = hasZ ? { kind, p, z } : { kind, p };
        break;
      }
      case 'line': {
        const a = pt();
        g = { kind, a, b: pt() };
        break;
      }
      case 'polyline':
      case 'polygon': {
        const pts = points();
        const bulges = values();
        const holes = rings();
        g = { kind, pts };
        if (bulges) g.bulges = bulges;
        if (holes) g.holes = holes;
        break;
      }
      case 'circle': {
        const c = pt();
        g = { kind, c, r: num() };
        break;
      }
      case 'arc': {
        const c = pt();
        const r = num();
        const a0 = num();
        g = { kind, c, r, a0, a1: num() };
        break;
      }
      case 'ellipse': {
        const c = pt();
        const major = pt();
        const ratio = num();
        const t0 = num();
        g = { kind, c, major, ratio, t0, t1: num() };
        break;
      }
      case 'xline':
      case 'ray': {
        const p = pt();
        g = { kind, p, dir: pt() };
        break;
      }
      case 'spline': {
        const pts = points();
        g = { kind, pts, closed: flag() };
        break;
      }
      case 'text': {
        const p = pt();
        const height = num();
        const rotation = num();
        g = { kind, p, text: str() ?? '', height, rotation };
        break;
      }
      case 'dimension': {
        const a = pt();
        const b = pt();
        const offset = num();
        const height = num();
        const text = str();
        const style = str();
        const hasAngle = flag();
        const angle = num();
        const hasC = flag();
        const c = pt();
        g = { kind, a, b, offset, height };
        if (text !== undefined) g.text = text;
        if (style !== undefined) g.style = style;
        if (hasAngle) g.angle = angle;
        if (hasC) g.c = c;
        break;
      }
      case 'hatch': {
        const ring = points();
        const n = num();
        const holes = n < 0 ? undefined : Array.from({ length: n }, points);
        const type = str() ?? '';
        const angle = num();
        g = { kind, ring };
        if (holes) g.holes = holes;
        g.pattern = { type, angle, spacing: num() };
        break;
      }
      default:
        throw new Error(`Geometri deposunun yanıtı okunamadı: ${at - 1}. sayı bilinmeyen bir nesne türü (${code}).`);
    }
    out.push({ id, layerId, labelled, geometry: g });
  }
  return out;
}
