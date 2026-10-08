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

const KIND: Record<string, number> = { point: 0, line: 1, polyline: 2, polygon: 3, circle: 4, arc: 5, ellipse: 6, xline: 7, ray: 8, spline: 9, text: 10, dimension: 11, hatch: 12, insert: 14, leader: 15, table: 18, image: 19, raster: 20 };
/** A text's alignments, numbered as the store numbers them (`TextAlign::ALL`, docs/adr/0145). */
const TEXT_ALIGNS = ['baselineCenter', 'baselineRight', 'bottomLeft', 'bottomCenter', 'bottomRight', 'middleLeft', 'middleCenter', 'middleRight', 'topLeft', 'topCenter', 'topRight'] as const;
/** The drawing typefaces, numbered as the core's tables number them (`FONTS`, the contract's `DrawingFont` order; docs/adr/0183). */
const FONTS = ['barlow', 'arimo', 'overpass', 'quicksand', 'architects-daughter', 'courier-prime', 'plex-mono'] as const;
/** A dimension's arrowheads, numbered as the core numbers them (`Arrow::ALL`); −1 the tick (docs/adr/0183 §3). */
const ARROWS = ['closed', 'open', 'dot', 'none'] as const;
/** A multi-line text's run's flags (docs/adr/0182; the store's `RUN_*`). */
const RUN_BOLD = 1;
const RUN_ITALIC = 2;
const RUN_UNDERLINE = 4;
const RUN_SUPER = 8;
const RUN_SUB = 16;

/** A run as the packer reads it (the model's TextRun). */
interface RunFields {
  start: number;
  end: number;
  bold?: boolean;
  italic?: boolean;
  underline?: boolean;
  script?: 'super' | 'sub';
  color?: string;
}

/** `v`'s place in `list`; −1 when it is not one of them (absent). */
const place = (list: readonly string[], v: unknown): number => (typeof v === 'string' ? list.indexOf(v) : -1);

const runBits = (r: RunFields): number =>
  (r.bold ? RUN_BOLD : 0) | (r.italic ? RUN_ITALIC : 0) | (r.underline ? RUN_UNDERLINE : 0) | (r.script === 'super' ? RUN_SUPER : r.script === 'sub' ? RUN_SUB : 0);

/** Kinds by their number (`KIND` the other way). */
const KINDS = ['point', 'line', 'polyline', 'polygon', 'circle', 'arc', 'ellipse', 'xline', 'ray', 'spline', 'text', 'dimension', 'hatch'] as const;
/**
 * A multi-part area (docs/adr/0143): its first part as a polygon's fields, then the count of the other
 * parts and each of them as a path. A one-part area stays the polygon's number, laid out as it always was.
 */
const MULTI_PART = 13;
/**
 * A block's insert (docs/adr/0144): its point, scale, turn, mirror flag, the block's id and its attributes' texts
 * (a count, −1 for none, of tag and value strings: what its block's attribute texts show, §7).
 */
const INSERT = 14;
/**
 * A leader (docs/adr/0146): its vertices, height and turn, its note and arrowhead's name (−1 none), its mask flag, its
 * arrowhead's size (NaN none, docs/adr/0205 §7).
 */
const LEADER = 15;
/** A multi-part polyline (docs/adr/0174): laid out as a multi-part area. */
const MULTI_PART_LINE = 16;
/** A multi-point object (docs/adr/0174): its first point as a point's fields, then the count of the others and each one's. */
const MULTI_POINT = 17;
/**
 * A table (docs/adr/0184): its corner, turn and height; its rows' heights and columns' widths; each cell's words, row
 * by row (as many as its rows times its columns); the merged ranges (a count, four numbers each); the alignments (−1
 * none, else a count and each one's place in TABLE_ALIGNS); the heading flag; the lines' place in TABLE_GRIDS (−1 all);
 * the frame's width (NaN none); its face as a text's. Its source is not packed: the store draws, picks and snaps.
 */
const TABLE = 18;
/**
 * A picture (docs/adr/0192): its corner, width, height and turn; the mirror flag; its asset and file; its clip (−1
 * none, else a count and the corners); its opacity (NaN none).
 */
const IMAGE = 19;
/**
 * A raster (docs/adr/0204): its affine (six numbers), width, height and bands; its samples, asset and file; its system;
 * its look (the contract's JSON text); its opacity (NaN none).
 */
const RASTER = 20;
/** A table's alignments and lines, numbered as the store numbers them (`TABLE_ALIGNS`, `TABLE_GRIDS`). */
const TABLE_ALIGNS = ['left', 'center', 'right'] as const;
const TABLE_GRIDS = ['outer', 'rows', 'none'] as const;

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
  const path = (e: Record<string, unknown>) => {
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
    let kind = KIND[e.kind];
    if (kind === undefined) throw new Error(`Geometri deposu “${String(e.kind)}” türünü tanımıyor.`);
    // An area, a polyline or a point with parts past its first has a number of its own (the others are as they always were).
    const parts = (kind === 0 || kind === 2 || kind === 3) && Array.isArray(e.parts) && e.parts.length ? (e.parts as Record<string, unknown>[]) : null;
    if (parts) kind = kind === 3 ? MULTI_PART : kind === 2 ? MULTI_PART_LINE : MULTI_POINT;
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
      case MULTI_PART:
      case MULTI_PART_LINE:
        path(e);
        out.push(parts!.length);
        for (const part of parts!) path(part);
        break;
      case MULTI_POINT:
        pt(e.p);
        out.push(e.z == null ? 0 : 1);
        num(e.z);
        out.push(parts!.length);
        for (const q of parts!) {
          pt(q.p);
          out.push(q.z == null ? 0 : 1);
          num(q.z);
        }
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
      // docs/adr/0145: the alignment's place in TEXT_ALIGNS (−1 none), the width factor (NaN none), the mask a flag;
      // docs/adr/0182: the box width and line spacing (NaN none), then the runs: a count, each its start, end, flags
      // (RUN_*) and colour (a string, −1 none).
      case 10: {
        pt(e.p);
        num(e.height);
        num(e.rotation);
        out.push(str(e.text), typeof e.align === 'string' ? TEXT_ALIGNS.indexOf(e.align as (typeof TEXT_ALIGNS)[number]) : -1);
        num(e.widthFactor);
        out.push(e.mask === true ? 1 : 0);
        num(e.boxWidth);
        num(e.lineSpacing);
        const runs = Array.isArray(e.runs) ? (e.runs as RunFields[]) : [];
        out.push(runs.length);
        for (const r of runs) out.push(r.start, r.end, runBits(r), str(r.color));
        // docs/adr/0183: the style (a string, −1 none), the typeface's place (−1 the project's), bold and italic flags,
        // the slant (NaN none).
        out.push(str(e.textStyle), place(FONTS, e.font), e.bold === true ? 1 : 0, e.italic === true ? 1 : 0);
        num(e.oblique);
        // docs/adr/0196: the curve's vertex count (0 none), each vertex x, y and its edge's bulge.
        const path = e.path as { pts: { x: number; y: number }[]; bulges?: number[] } | undefined;
        out.push(path ? path.pts.length : 0);
        if (path) path.pts.forEach((q, i) => out.push(q.x, q.y, path.bulges?.[i] ?? 0));
        break;
      }
      // docs/adr/0147: the mask a flag, the slope's elevations (NaN none).
      case 11:
        pt(e.a);
        pt(e.b);
        num(e.offset);
        num(e.height);
        out.push(str(e.text), str(e.style), e.angle == null ? 0 : 1);
        num(e.angle);
        out.push(e.c == null ? 0 : 1);
        pt(e.c);
        out.push(e.mask === true ? 1 : 0);
        num(e.za);
        num(e.zb);
        // docs/adr/0183: the style, the arrowhead's place (−1 the tick), the sizes (NaN none), the centre flag, the
        // decimals (NaN none), the unit, prefix and suffix (strings, −1 none), the typeface's place.
        out.push(str(e.dimStyle), place(ARROWS, e.arrow));
        num(e.arrowSize);
        num(e.extOffset);
        num(e.extBeyond);
        num(e.textGap);
        out.push(e.textPlace === 'centre' ? 1 : 0);
        num(e.decimals);
        out.push(str(e.unit), str(e.prefix), str(e.suffix), place(FONTS, e.font));
        // docs/adr/0205 §6: its lines, colours and types as strings (−1 none), weights NaN none.
        out.push(str(e.dimLineColor));
        num(e.dimLineWeight);
        out.push(str(e.dimLineType), str(e.extColor));
        num(e.extWeight);
        out.push(str(e.extLineType), str(e.textColor));
        break;
      case 12: {
        points(e.ring);
        const holes = e.holes as unknown[] | undefined;
        if (!Array.isArray(holes)) out.push(-1);
        else {
          out.push(holes.length);
          for (const h of holes) points(h);
        }
        const pattern = e.pattern as
          | { type?: unknown; angle?: unknown; spacing?: unknown; name?: unknown; scale?: unknown; lines?: unknown; gradient?: { shape?: unknown; inverted?: unknown; color2?: unknown } }
          | undefined;
        out.push(str(pattern?.type));
        num(pattern?.angle);
        num(pattern?.spacing);
        // A pattern's name, scale, families and gradient, the objects it follows (docs/adr/0186; pack.rs).
        out.push(str(pattern?.name));
        num(pattern?.scale);
        const lines = pattern?.lines as { angle?: unknown; origin?: unknown[]; offset?: unknown[]; dashes?: unknown }[] | undefined;
        if (!Array.isArray(lines)) out.push(-1);
        else {
          out.push(lines.length);
          for (const l of lines) {
            num(l.angle);
            num(l.origin?.[0]);
            num(l.origin?.[1]);
            num(l.offset?.[0]);
            num(l.offset?.[1]);
            values(l.dashes);
          }
        }
        const gradient = pattern?.gradient;
        if (!gradient) out.push(-1);
        else out.push(str(gradient.shape), gradient.inverted === undefined ? -1 : gradient.inverted ? 1 : 0, str(gradient.color2));
        const assoc = e.assoc as { outer?: unknown; islands?: unknown; cutouts?: unknown; seed?: unknown } | undefined;
        if (!assoc) out.push(-1);
        else {
          out.push(str(assoc.outer));
          for (const ids of [assoc.islands, assoc.cutouts]) {
            if (!Array.isArray(ids)) out.push(-1);
            else {
              out.push(ids.length);
              for (const id of ids) out.push(str(id));
            }
          }
          pt(assoc.seed);
        }
        break;
      }
      case INSERT: {
        pt(e.p);
        num(e.scale);
        num(e.rotation);
        out.push(e.mirror === true ? 1 : 0, str(e.block));
        const attrs = e.attrs && typeof e.attrs === 'object' ? Object.entries(e.attrs).filter((kv): kv is [string, string] => typeof kv[1] === 'string') : null;
        if (!attrs) out.push(-1);
        else {
          out.push(attrs.length);
          for (const [tag, value] of attrs) out.push(str(tag), str(value));
        }
        break;
      }
      case LEADER:
        points(e.pts);
        num(e.height);
        num(e.rotation);
        out.push(str(e.text), str(e.arrow), e.mask === true ? 1 : 0);
        num(e.arrowSize);
        break;
      case RASTER: {
        const affine = Array.isArray(e.affine) ? (e.affine as unknown[]) : [];
        for (let k = 0; k < 6; k++) num(affine[k]);
        num(e.width);
        num(e.height);
        num(e.bands);
        out.push(str(e.sample), str(e.asset), str(e.file));
        num(e.srid);
        out.push(str(JSON.stringify(e.style ?? {})));
        num(e.opacity);
        break;
      }
      case IMAGE:
        pt(e.p);
        num(e.width);
        num(e.height);
        num(e.rotation);
        out.push(e.mirror === true ? 1 : 0, str(e.asset), str(e.file));
        if (Array.isArray(e.clip)) points(e.clip);
        else out.push(-1);
        num(e.opacity);
        break;
      case TABLE: {
        pt(e.p);
        num(e.rotation);
        num(e.height);
        const rows = Array.isArray(e.rows) ? (e.rows as unknown[]) : [];
        const columns = Array.isArray(e.columns) ? (e.columns as unknown[]) : [];
        values(rows);
        values(columns);
        const cells = Array.isArray(e.cells) ? (e.cells as unknown[][]) : [];
        for (let i = 0; i < rows.length; i++) for (let j = 0; j < columns.length; j++) out.push(str(cells[i]?.[j]));
        const merges = Array.isArray(e.merges) ? (e.merges as Record<string, unknown>[]) : [];
        out.push(merges.length);
        for (const m of merges) {
          num(m.row);
          num(m.col);
          num(m.rows);
          num(m.cols);
        }
        const aligns = Array.isArray(e.aligns) ? (e.aligns as unknown[]) : null;
        if (!aligns) out.push(-1);
        else {
          out.push(aligns.length);
          for (const a of aligns) out.push(Math.max(0, place(TABLE_ALIGNS, a)));
        }
        out.push(e.header === true ? 1 : 0, place(TABLE_GRIDS, e.grid));
        num(e.frame);
        out.push(str(e.textStyle), place(FONTS, e.font), e.bold === true ? 1 : 0, e.italic === true ? 1 : 0);
        num(e.oblique);
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
  /** A path of a multi-part area past its first: points, bulges and holes. */
  const part = (): { pts: XY[]; bulges?: number[]; holes?: { pts: XY[]; bulges?: number[] }[] } => {
    const pts = points();
    const bulges = values();
    const holes = rings();
    const out: { pts: XY[]; bulges?: number[]; holes?: { pts: XY[]; bulges?: number[] }[] } = { pts };
    if (bulges) out.bulges = bulges;
    if (holes) out.holes = holes;
    return out;
  };
  while (at < nums.length) {
    const id = num();
    const layerId = str() ?? '';
    const labelled = flag();
    const code = num();
    const kind =
      code === MULTI_PART
        ? 'polygon'
        : code === MULTI_PART_LINE
          ? 'polyline'
          : code === MULTI_POINT
            ? 'point'
            : code === INSERT
              ? 'insert'
              : code === LEADER
                ? 'leader'
                : code === TABLE
                  ? 'table'
                  : code === IMAGE
                    ? 'image'
                    : code === RASTER
                      ? 'raster'
                      : KINDS[code];
    let g: Geometry;
    switch (kind) {
      case 'point': {
        const p = pt();
        const hasZ = flag();
        const z = num();
        g = hasZ ? { kind, p, z } : { kind, p };
        // A multi-point object: the count of its other points, then each of them.
        if (code === MULTI_POINT)
          g.parts = Array.from({ length: num() }, () => {
            const q = pt();
            const qz = flag();
            const z = num();
            return qz ? { p: q, z } : { p: q };
          });
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
        // A multi-part area or polyline: the count of its other parts, then each of them.
        if (code === MULTI_PART || code === MULTI_PART_LINE) g.parts = Array.from({ length: num() }, part);
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
        const align = num();
        if (align >= 0) g.align = TEXT_ALIGNS[align];
        const widthFactor = num();
        if (!Number.isNaN(widthFactor)) g.widthFactor = widthFactor;
        if (flag()) g.mask = true;
        const boxWidth = num();
        if (!Number.isNaN(boxWidth)) g.boxWidth = boxWidth;
        const lineSpacing = num();
        if (!Number.isNaN(lineSpacing)) g.lineSpacing = lineSpacing;
        const count = num();
        if (count > 0)
          g.runs = Array.from({ length: count }, () => {
            const r: RunFields = { start: num(), end: num() };
            const bits = num();
            if (bits & RUN_BOLD) r.bold = true;
            if (bits & RUN_ITALIC) r.italic = true;
            if (bits & RUN_UNDERLINE) r.underline = true;
            if (bits & RUN_SUPER) r.script = 'super';
            else if (bits & RUN_SUB) r.script = 'sub';
            const color = str();
            if (color !== undefined) r.color = color;
            return r;
          });
        const textStyle = str();
        if (textStyle !== undefined) g.textStyle = textStyle;
        const font = num();
        if (font >= 0) g.font = FONTS[font];
        if (flag()) g.bold = true;
        if (flag()) g.italic = true;
        const oblique = num();
        if (!Number.isNaN(oblique)) g.oblique = oblique;
        // docs/adr/0196: the curve.
        const vertices = num();
        if (vertices > 0) {
          const pts: XY[] = [];
          const bulges: number[] = [];
          for (let i = 0; i < vertices; i++) {
            pts.push(pt());
            bulges.push(num());
          }
          g.path = bulges.some((b) => b !== 0) ? { pts, bulges } : { pts };
        }
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
        const mask = flag();
        const za = num();
        const zb = num();
        const dimStyle = str();
        const arrow = num();
        const sizes = [num(), num(), num(), num()];
        const centre = flag();
        const decimals = num();
        const unit = str();
        const prefix = str();
        const suffix = str();
        const font = num();
        const lineColor = str();
        const lineWeight = num();
        const lineType = str();
        const extColor = str();
        const extWeight = num();
        const extLineType = str();
        const textColor = str();
        g = { kind, a, b, offset, height };
        if (text !== undefined) g.text = text;
        if (style !== undefined) g.style = style;
        if (hasAngle) g.angle = angle;
        if (hasC) g.c = c;
        if (mask) g.mask = true;
        if (!Number.isNaN(za)) g.za = za;
        if (!Number.isNaN(zb)) g.zb = zb;
        if (dimStyle !== undefined) g.dimStyle = dimStyle;
        if (arrow >= 0) g.arrow = ARROWS[arrow];
        (['arrowSize', 'extOffset', 'extBeyond', 'textGap'] as const).forEach((k, i) => {
          if (!Number.isNaN(sizes[i])) g[k] = sizes[i];
        });
        if (centre) g.textPlace = 'centre';
        if (!Number.isNaN(decimals)) g.decimals = decimals;
        if (unit !== undefined) g.unit = unit;
        if (prefix !== undefined) g.prefix = prefix;
        if (suffix !== undefined) g.suffix = suffix;
        if (font >= 0) g.font = FONTS[font];
        if (lineColor !== undefined) g.dimLineColor = lineColor;
        if (!Number.isNaN(lineWeight)) g.dimLineWeight = lineWeight;
        if (lineType !== undefined) g.dimLineType = lineType;
        if (extColor !== undefined) g.extColor = extColor;
        if (!Number.isNaN(extWeight)) g.extWeight = extWeight;
        if (extLineType !== undefined) g.extLineType = extLineType;
        if (textColor !== undefined) g.textColor = textColor;
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
        const pattern: Record<string, unknown> = { type, angle, spacing: num() };
        g.pattern = pattern;
        // docs/adr/0186: a pattern's name, scale, families and gradient, the objects it follows.
        const name = str();
        if (name !== undefined) pattern.name = name;
        const scale = num();
        if (!Number.isNaN(scale)) pattern.scale = scale;
        const families = num();
        if (families >= 0)
          pattern.lines = Array.from({ length: families }, () => {
            const angle = num();
            const origin = [num(), num()];
            const offset = [num(), num()];
            const dashes = values();
            return dashes ? { angle, origin, offset, dashes } : { angle, origin, offset };
          });
        const shape = str();
        if (shape !== undefined) {
          const inverted = num();
          const color2 = str() ?? '';
          pattern.gradient = inverted < 0 ? { shape, color2 } : { shape, inverted: inverted === 1, color2 };
        }
        const outer = str();
        if (outer !== undefined) {
          const ids = () => {
            const k = num();
            return k < 0 ? undefined : Array.from({ length: k }, () => str() ?? '');
          };
          const islands = ids();
          const cutouts = ids();
          g.assoc = { outer, ...(islands && { islands }), ...(cutouts && { cutouts }), seed: pt() };
        }
        break;
      }
      case 'insert': {
        const p = pt();
        const scale = num();
        const rotation = num();
        const mirror = flag();
        g = { kind, block: str() ?? '', p, scale, rotation };
        if (mirror) g.mirror = true;
        // Its attributes are the object's data, not its geometry: read past.
        for (let n = num(); n > 0; n--) at += 2;
        break;
      }
      case 'leader': {
        const pts = points();
        const height = num();
        const rotation = num();
        const text = str();
        const arrow = str();
        const mask = flag();
        const arrowSize = num();
        g = { kind, pts };
        if (text !== undefined) g.text = text;
        g.height = height;
        g.rotation = rotation;
        if (arrow !== undefined) g.arrow = arrow;
        if (!Number.isNaN(arrowSize)) g.arrowSize = arrowSize;
        if (mask) g.mask = true;
        break;
      }
      case 'raster': {
        const affine = [num(), num(), num(), num(), num(), num()];
        const width = num();
        const height = num();
        const bands = num();
        const sample = str() ?? 'u8';
        const asset = str();
        const file = str();
        const srid = num();
        const style = JSON.parse(str() ?? '{}') as unknown;
        const opacity = num();
        g = { kind, affine, width, height, bands, sample, srid, style };
        if (asset !== undefined) g.asset = asset;
        if (file !== undefined) g.file = file;
        if (!Number.isNaN(opacity)) g.opacity = opacity;
        break;
      }
      case 'image': {
        const p = pt();
        const width = num();
        const height = num();
        const rotation = num();
        const mirror = flag();
        const asset = str();
        const file = str();
        const n = num();
        const clip = n < 0 ? undefined : Array.from({ length: n }, () => pt());
        const opacity = num();
        g = { kind, p, width, height, rotation };
        if (mirror) g.mirror = true;
        if (asset !== undefined) g.asset = asset;
        if (file !== undefined) g.file = file;
        if (clip) g.clip = clip;
        if (!Number.isNaN(opacity)) g.opacity = opacity;
        break;
      }
      case 'table': {
        const p = pt();
        const rotation = num();
        const height = num();
        const rows = values() ?? [];
        const columns = values() ?? [];
        const cells = rows.map(() => columns.map(() => str() ?? ''));
        const merges = Array.from({ length: num() }, () => ({ row: num(), col: num(), rows: num(), cols: num() }));
        const n = num();
        const aligns = n < 0 ? undefined : Array.from({ length: n }, () => TABLE_ALIGNS[num()] ?? 'left');
        const header = flag();
        const grid = num();
        const frame = num();
        g = { kind, p, rotation, height, rows, columns, cells };
        if (merges.length) g.merges = merges;
        if (aligns) g.aligns = aligns;
        if (header) g.header = true;
        if (grid >= 0) g.grid = TABLE_GRIDS[grid];
        if (!Number.isNaN(frame)) g.frame = frame;
        const textStyle = str();
        if (textStyle !== undefined) g.textStyle = textStyle;
        const font = num();
        if (font >= 0) g.font = FONTS[font];
        if (flag()) g.bold = true;
        if (flag()) g.italic = true;
        const oblique = num();
        if (!Number.isNaN(oblique)) g.oblique = oblique;
        break;
      }
      default:
        throw new Error(`Geometri deposunun yanıtı okunamadı: ${at - 1}. sayı bilinmeyen bir nesne türü (${code}).`);
    }
    out.push({ id, layerId, labelled, geometry: g });
  }
  return out;
}
