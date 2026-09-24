/**
 * Objects packed as numbers for the Rust geometry store (docs/adr/0008:
 * points cross as Float64Array; crates/shared/geometry-core/src/store/pack.rs has
 * the layout). A JSON array of 80 000 parcels is 26 MB and made the core
 * build millions of small values before reading one object; packed, it is
 * one run of float64s (ids and coordinates bit for bit, −0 and NaN
 * included) and a short list of strings (layer ids, texts). This only
 * packs: no coordinate is computed here.
 */

const KIND: Record<string, number> = { point: 0, line: 1, polyline: 2, polygon: 3, circle: 4, arc: 5, ellipse: 6, xline: 7, ray: 8, spline: 9, text: 10, dimension: 11, hatch: 12 };

interface XY {
  x: number;
  y: number;
}

/** What the packer reads of an object (the model's Entity; this layer cannot import it). */
type Fields = Record<string, unknown> & { id: number; kind: string };

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
