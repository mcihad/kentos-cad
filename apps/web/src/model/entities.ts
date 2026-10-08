import { op } from '../wasm/core';
import type { Bounds, Vec2 } from './geometry';
import type { DimensionStyle } from './geom/dimension';
import type { DimensionLook, TextFace } from './annotationStyles';

export type EntityKind = 'point' | 'line' | 'polyline' | 'polygon' | 'circle' | 'arc' | 'ellipse' | 'spline' | 'xline' | 'ray' | 'text' | 'dimension' | 'hatch' | 'insert' | 'leader' | 'table' | 'image' | 'raster';

/**
 * Half-length (1000 km) used when an infinite line meets finite geometry on
 * the CPU. Far beyond any sheet in a TM zone, yet small enough that float64
 * keeps ~10⁻¹⁰ m at its ends (10⁴ km would already round to 2·10⁻⁹ m).
 * Rendering clips to the view instead.
 */
export const CONSTRUCTION_REACH = 1e6;

interface EntityBase {
  /**
   * The object's slot in the open drawing (docs/adr/0014): what selection,
   * picking, the geometry store, the GPU buffers and undo work with. Not
   * persistent: a drawing opened again numbers its objects anew.
   */
  id: number;
  /**
   * Persistent id (docs/adr/0014): a UUID, lowercase with hyphens. Every
   * object in a drawing has one (`DrawingEntity`): given when it is created
   * (v7) or derived from a v1 file when it is opened (v5). An edit, undo and
   * redo keep it; a copy or a new piece gets a new one; it is never reused.
   * An object outside a drawing (a preview, a clipboard copy, a file's object
   * before the drawing takes it) has none. Hot paths (drawing, picking,
   * snapping) never carry it.
   */
  uid?: string;
  layerId: string;
  /** Overrides the layer colour; undefined means "katmana göre" (ByLayer). */
  color?: string;
  /** Free-form GIS attributes (Ada, Parsel, Nitelik…). */
  attrs: Record<string, string>;
  /** Short label drawn at the entity's anchor (parcel number, point name). */
  label?: string;
  /** Library symbol drawn for this object, overriding its layer's style (docs/STYLE.md). */
  symbol?: string;
  /**
   * Its own line weight, paper millimetres as the layer's `lineWeight`, 0 the
   * thinnest line; undefined means "katmana göre" (docs/adr/0139). What a
   * DXF's group 370 and a Netcad pen give an object.
   */
  lineWeight?: number;
}

/** The heaviest line weight an object may have, mm (`MAX_LINE_WEIGHT` in the contracts). */
export const MAX_LINE_WEIGHT = 100;

export interface PointEntity extends EntityBase {
  kind: 'point';
  p: Vec2;
  z?: number;
  /**
   * A multi-point object's points past its first, whose own are the fields
   * above (docs/adr/0174); undefined for one point.
   */
  parts?: PointPart[];
}
/** A point of a multi-point object past its first: its place and elevation. */
export interface PointPart {
  p: Vec2;
  z?: number;
}
export interface LineEntity extends EntityBase {
  kind: 'line';
  a: Vec2;
  b: Vec2;
  /** The ends' elevations, m; undefined means none (docs/adr/0142). */
  za?: number;
  zb?: number;
}
export interface PolylineEntity extends EntityBase {
  kind: 'polyline' | 'polygon';
  pts: Vec2[];
  /**
   * Arc segments: bulges[i] = tan(θ/4) for the segment pts[i] → pts[i+1]
   * (the closing segment for a polygon), positive counter-clockwise.
   * Absent or all zero means straight segments only (DXF LWPOLYLINE bulge).
   */
  bulges?: number[];
  /**
   * Holes of a polygon ("adalı alan"): closed rings in the same vertex +
   * bulge form, lying inside the outer ring. Only polygons carry them;
   * CadDocument drops the field when an edit turns a polygon into
   * something else. Area, edges, picking and fills all respect them.
   */
  holes?: RingGeometry[];
  /**
   * Each vertex's elevation, m, as many as `pts`; null for a vertex without
   * one (not 0); undefined when no vertex has one (docs/adr/0142).
   */
  zs?: (number | null)[];
  /**
   * A multi-part area's or polyline's parts past its first, whose own are the
   * fields above (docs/adr/0143, 0174); undefined for one part. A polyline's
   * parts have no holes.
   */
  parts?: AreaPart[];
}
/** A part of a multi-part area or polyline past its first: its path, arcs, holes (an area's only) and elevations. */
export interface AreaPart {
  pts: Vec2[];
  bulges?: number[];
  holes?: RingGeometry[];
  zs?: (number | null)[];
}
/** A closed ring of vertices with DXF bulges (outer boundary or hole). */
export interface RingGeometry {
  pts: Vec2[];
  bulges?: number[];
  /** Each vertex's elevation, as a polyline's `zs`. */
  zs?: (number | null)[];
}
export interface CircleEntity extends EntityBase {
  kind: 'circle';
  c: Vec2;
  r: number;
}
/** Arc running counter-clockwise from a0 to a1 (radians from east). */
export interface ArcEntity extends EntityBase {
  kind: 'arc';
  c: Vec2;
  r: number;
  a0: number;
  a1: number;
}
/**
 * Ellipse or elliptical arc (DXF ELLIPSE): centre, major axis vector,
 * minor/major ratio and parameters t0 → t1 counter-clockwise; equal
 * parameters mean the whole ellipse. See model/geom/ellipse.
 */
export interface EllipseEntity extends EntityBase {
  kind: 'ellipse';
  c: Vec2;
  major: Vec2;
  ratio: number;
  t0: number;
  t1: number;
}
/** Construction line through p (xline: both ways, ray: towards dir only). dir is a unit vector. */
export interface ConstructionEntity extends EntityBase {
  kind: 'xline' | 'ray';
  p: Vec2;
  dir: Vec2;
}
/** Smooth curve through fit points (centripetal Catmull-Rom). */
export interface SplineEntity extends EntityBase {
  kind: 'spline';
  pts: Vec2[];
  closed: boolean;
}
/**
 * Dimension (aligned unless `style` says otherwise; see geom/dimension for
 * what a, b, offset, angle and c mean per style). `text` overrides the
 * measured value when set. Its look (arrowheads, sizes, the value's place and
 * writing; docs/adr/0183 §3) is its own, all absent the look dimensions always had.
 */
export interface DimensionEntity extends EntityBase, DimensionLook {
  kind: 'dimension';
  a: Vec2;
  b: Vec2;
  offset: number;
  height: number;
  text?: string;
  style?: DimensionStyle;
  /** linear: measured direction, degrees CCW from east (0 = ΔY, 90 = ΔX); ordinate: 0 its Y, 90 its X. */
  angle?: number;
  /** angular: the vertex; arc length: the arc's centre; jogged: the centre its line starts from. */
  c?: Vec2;
  /** The value over the drawing's background (docs/adr/0147); only `true` is written. */
  mask?: boolean;
  /** slope: the two points' elevations, metres (docs/adr/0147). */
  za?: number;
  zb?: number;
}
export type HatchPatternType = 'solid' | 'lines' | 'cross' | 'pattern' | 'gradient';
/**
 * A pattern's family (docs/adr/0186 §1), in its pattern's units, unturned: lines at `angle` (degrees) through `origin`,
 * each the next `offset` on (`[along, across]` the line), drawn as `dashes` say (plus drawn, minus a gap, 0 a dot).
 */
export interface PatternLine {
  angle: number;
  origin: [number, number];
  offset: [number, number];
  dashes?: number[];
}
export type GradientShape = 'linear' | 'cylinder' | 'spherical';
/** A gradient (docs/adr/0186 §1): its shape, whether it runs the other way, its second colour (`#RRGGBB`). */
export interface HatchGradient {
  shape: GradientShape;
  inverted?: boolean;
  color2: string;
}
export interface HatchPattern {
  type: HatchPatternType;
  /** Degrees, CCW from east: the lines' (lines, cross), the pattern's turn (pattern), the gradient's direction. */
  angle: number;
  /** Line spacing in metres (lines, cross); 1 and unread for the others. */
  spacing: number;
  /** A pattern's name (`ANSI31`), metres per unit of its definition and families (docs/adr/0186). */
  name?: string;
  scale?: number;
  lines?: PatternLine[];
  /** A gradient's shape and second colour. */
  gradient?: HatchGradient;
}
/**
 * The objects a hatch's region follows (docs/adr/0186 §6), by their persistent ids: the closed object, its islands,
 * the texts and inserts left open, and the point clicked inside.
 */
export interface HatchAssoc {
  outer: string;
  islands?: string[];
  cutouts?: string[];
  seed: Vec2;
}
/** Filled, line-patterned or gradient area inside a boundary ring; associative when it has `assoc`. */
export interface HatchEntity extends EntityBase {
  kind: 'hatch';
  ring: Vec2[];
  /** Islands left unhatched (the holes of a holed polygon). */
  holes?: Vec2[][];
  pattern: HatchPattern;
  assoc?: HatchAssoc;
}
/** A text; its face (style, typeface, bold, italic, slant; docs/adr/0183 §2) is its own, all absent the project's typeface. */
export interface TextEntity extends EntityBase, TextFace {
  kind: 'text';
  p: Vec2;
  text: string;
  /** Text height in metres (paper-independent). */
  height: number;
  /** Degrees, counter-clockwise from east. */
  rotation: number;
  /** Where `p` is on the text; absent: the left of the baseline (docs/adr/0145). */
  align?: TextAlign;
  /** The letters' width times this, the height kept; absent: 1 (over 0, at most `MAX_WIDTH_FACTOR`). */
  widthFactor?: number;
  /** The text's box filled with the drawing area's colour before the text: what lies under it does not show. */
  mask?: boolean;
  /**
   * The object whose label this text writes (Etiketleri yazıya çevir's “Nesneye bağlı”, docs/adr/0175 §4): its
   * persistent id. The text follows the object as its label at `labelScale`; absent, a text of its own. Given with
   * `labelScale` or not at all.
   */
  labelOf?: string;
  /** The scale's denominator (1:N) the linked label is written at; finite, over 0. */
  labelScale?: number;
  /** Metres, over 0: the lines wrap word by word to this width (docs/adr/0182 §1); absent: they end only at line breaks. */
  boxWidth?: number;
  /** The baselines' distance in 5/3 of the height (DXF's group 44); absent: 1. From `MIN_LINE_SPACING` to `MAX_LINE_SPACING`. */
  lineSpacing?: number;
  /**
   * Its letters' formats (docs/adr/0182 §1): ranges of the text's Unicode scalar values (not UTF-16 units), in order,
   * apart, each with a format, touching runs of one format joined; absent: none.
   */
  runs?: TextRun[];
  /**
   * The curve its letters stand on (Eğri boyunca yazı, docs/adr/0196 §1), in its own frame: `p` its start, x along
   * `rotation`, metres; absent: a straight text.
   */
  path?: TextPath;
}

/**
 * A text's curve (docs/adr/0196 §1): its vertices after `p` in the text's frame and, when an edge bends, each edge's
 * bulge (DXF's tan(θ/4), counter-clockwise positive; as many as the vertices).
 */
export interface TextPath {
  pts: Vec2[];
  bulges?: number[];
}

/** A range of a text's letters and their format (docs/adr/0182 §1); a flag that is off is the field's absence. */
export interface TextRun {
  start: number;
  end: number;
  bold?: boolean;
  italic?: boolean;
  underline?: boolean;
  /** Raised or lowered, at 0.6 of the height. */
  script?: 'super' | 'sub';
  /** Its own colour, as an object's (`#E5484D` or a theme name); absent: the text's. */
  color?: string;
}

/** The least and the most a multi-line text's line spacing may be (AutoCAD's own; the contracts' `MIN_LINE_SPACING`). */
export const MIN_LINE_SPACING = 0.25;
export const MAX_LINE_SPACING = 4;

/** Whether a text is laid out in lines (docs/adr/0182): it has a line break, a box, a line spacing or letter formats. */
export const isParagraph = (t: Pick<TextEntity, 'text' | 'boxWidth' | 'lineSpacing' | 'runs'>): boolean =>
  t.boxWidth !== undefined || t.lineSpacing !== undefined || (t.runs?.length ?? 0) > 0 || t.text.includes('\n');

const hasFormat = (r: TextRun): boolean => r.bold === true || r.italic === true || r.underline === true || r.script !== undefined || r.color !== undefined;
const sameFormat = (a: TextRun, b: TextRun): boolean =>
  (a.bold ?? false) === (b.bold ?? false) && (a.italic ?? false) === (b.italic ?? false) && (a.underline ?? false) === (b.underline ?? false) && a.script === b.script && a.color === b.color;

/**
 * What is wrong with a multi-line text's fields for its `text` (docs/adr/0182 §1, the contracts' `Paragraph::problem`,
 * word for word): the field and the refusal's words; null when they may be written.
 */
/**
 * What is wrong with a text's curve for the text (docs/adr/0196 §1; the contract's `text_path_problem`, in its order):
 * no vertex, a number not finite, bulges not one an edge, no length; then a text of more than one line (a line
 * break, a box width or a line spacing), or one linked to an object. Null when it may be written.
 */
export function textPathProblem(path: TextPath, text: string, p: Pick<TextEntity, 'boxWidth' | 'lineSpacing'>, linked: boolean): string | null {
  if (path.pts.length === 0) return 'Eğri boyunca yazının eğrisinde köşe yok. En az bir köşe verin ya da eğriyi kaldırın (düz yazı).';
  if (!path.pts.every((q) => Number.isFinite(q.x) && Number.isFinite(q.y)) || !(path.bulges ?? []).every(Number.isFinite))
    return 'Eğri boyunca yazının eğrisinde sonlu olmayan bir sayı var. Köşeleri ve kavisleri sonlu sayılarla verin.';
  if (path.bulges !== undefined && path.bulges.length !== path.pts.length)
    return `Eğri boyunca yazının kavis sayısı (${path.bulges.length}) köşe sayısından (${path.pts.length}) farklı. Her kenara bir kavis verin ya da kavisleri kaldırın (düz kenarlar).`;
  if (path.pts.every((q) => q.x === 0 && q.y === 0))
    return 'Eğri boyunca yazının eğrisinin uzunluğu sıfır: bütün köşeleri yazının noktasında. Köşeleri yazının noktasından ayırın.';
  if (text.includes('\n') || p.boxWidth != null || p.lineSpacing != null)
    return 'Eğri boyunca yazı tek satırdır: satır sonu, kutu genişliği ve satır aralığı olmaz. Yazıyı tek satır yapın ya da eğriyi kaldırın.';
  if (linked) return 'Nesneye bağlı yazının eğrisi olmaz. Önce bağı koparın ya da eğriyi kaldırın.';
  return null;
}

export function paragraphProblem(text: string, p: Pick<TextEntity, 'boxWidth' | 'lineSpacing' | 'runs'>): ['boxWidth' | 'lineSpacing' | 'runs', string] | null {
  const w = p.boxWidth;
  if (w !== undefined && !(Number.isFinite(w) && w > 0))
    return ['boxWidth', `Çok satırlı yazının kutu genişliği sıfırdan büyük olmalı; ${w} verildi. Genişliği metre olarak, pozitif verin ya da alanı kaldırın (satırlar yalnız satır sonlarında biter).`];
  const s = p.lineSpacing;
  if (s !== undefined && !(s >= MIN_LINE_SPACING && s <= MAX_LINE_SPACING))
    return ['lineSpacing', `Çok satırlı yazının satır aralığı ${MIN_LINE_SPACING} ile ${MAX_LINE_SPACING} arasında olmalı; ${s} verildi. Aralığı bu sınırlarda verin ya da alanı kaldırın (1).`];
  const letters = [...text].length;
  let before: TextRun | null = null;
  for (const [i, r] of (p.runs ?? []).entries()) {
    const n = i + 1;
    if (r.start >= r.end || r.end > letters)
      return ['runs', `${n}. biçim dilimi ${r.start}–${r.end}: başı sonundan önce olmalı, sonu yazının harf sayısını (${letters}) aşmamalı. Dilimi yazının harfleri içinde verin.`];
    if (!hasFormat(r)) return ['runs', `${n}. biçim diliminin biçimi yok; biçimsiz dilim yazılmaz. Dilime bir biçim verin ya da dilimi çıkarın.`];
    if (r.color === '') return ['runs', `${n}. biçim diliminin rengi boş. Bir renk verin ya da rengi kaldırın.`];
    if (before) {
      if (r.start < before.end)
        return ['runs', `${n}. biçim dilimi öncekiyle örtüşüyor ya da ondan önce başlıyor; dilimler sıralı ve ayrı olmalı. Dilimleri sırayla, örtüşmeden verin.`];
      if (r.start === before.end && sameFormat(r, before)) return ['runs', `${n}. biçim dilimi aynı biçimdeki öncekine bitişik; ikisi tek dilimdir. İki dilimi birleştirin.`];
    }
    before = r;
  }
  return null;
}

/**
 * Which point of a text its `p` is (docs/adr/0145 §1): left, centre or right;
 * on the baseline, at the bottom (0.2 of the height under it), the middle or
 * the top. The left of the baseline is the field's absence, no value.
 */
export type TextAlign = 'baselineCenter' | 'baselineRight' | 'bottomLeft' | 'bottomCenter' | 'bottomRight' | 'middleLeft' | 'middleCenter' | 'middleRight' | 'topLeft' | 'topCenter' | 'topRight';

/** Every alignment, in the contract's order (`TextAlign::ALL`). */
export const TEXT_ALIGNS: readonly TextAlign[] = ['baselineCenter', 'baselineRight', 'bottomLeft', 'bottomCenter', 'bottomRight', 'middleLeft', 'middleCenter', 'middleRight', 'topLeft', 'topCenter', 'topRight'];

/**
 * The alignment picker's rows (docs/adr/0145 §6): top, middle, bottom and the baseline, each left, centre and right;
 * null is the left of the baseline, a text without the field. The desktop's `text::ALIGN_ROWS` is the same.
 */
export const TEXT_ALIGN_ROWS: readonly (readonly (TextAlign | null)[])[] = [
  ['topLeft', 'topCenter', 'topRight'],
  ['middleLeft', 'middleCenter', 'middleRight'],
  ['bottomLeft', 'bottomCenter', 'bottomRight'],
  [null, 'baselineCenter', 'baselineRight'],
];

const ALIGN_NAME: Record<TextAlign | 'baselineLeft', string> = {
  topLeft: 'sol üst',
  topCenter: 'orta üst',
  topRight: 'sağ üst',
  middleLeft: 'sol orta',
  middleCenter: 'orta',
  middleRight: 'sağ orta',
  bottomLeft: 'sol alt',
  bottomCenter: 'orta alt',
  bottomRight: 'sağ alt',
  baselineLeft: 'sol taban',
  baselineCenter: 'orta taban',
  baselineRight: 'sağ taban',
};

/** An alignment's name as Yazı, its menu and Öznitelikler write it, in lower case: “sol taban”, “orta”, “sağ üst”. */
export const textAlignName = (a: TextAlign | null | undefined): string => ALIGN_NAME[a ?? 'baselineLeft'];

/** A name as typed, folded: lower case, Turkish letters without their marks, nothing but letters (“Sağ-üst” → “sagust”). */
const foldName = (s: string): string =>
  s
    .toLocaleLowerCase('tr-TR')
    .replace(/[çğıöşü]/g, (c) => ({ ç: 'c', ğ: 'g', ı: 'i', ö: 'o', ş: 's', ü: 'u' })[c] ?? c)
    .replace(/[^a-z]/g, '');

/**
 * The alignment a typed name is (Yazı's Hiza, docs/adr/0145 §6), written together or apart, with or without the
 * Turkish marks (“sağüst”, “sag-ust”; “orta” and “ortaorta” are the middle): null is the left of the baseline,
 * undefined no alignment's name. The desktop's `text::align_from_name` reads the same.
 */
export function textAlignFromName(typed: string): TextAlign | null | undefined {
  const folded = foldName(typed);
  if (folded === 'ortaorta') return 'middleCenter';
  for (const [align, name] of Object.entries(ALIGN_NAME)) if (foldName(name) === folded) return align === 'baselineLeft' ? null : (align as TextAlign);
  return undefined;
}

/** The widest a text's letters may be drawn, times their width (`MAX_WIDTH_FACTOR` in the contracts). */
export const MAX_WIDTH_FACTOR = 100;

/** Whether `f` may be a width factor: over 0, at most `MAX_WIDTH_FACTOR` (NaN is not). */
export const widthFactorOk = (f: number): boolean => f > 0 && f <= MAX_WIDTH_FACTOR;
/**
 * A block placed in the drawing (docs/adr/0144): its definition (`block`,
 * model/blocks.ts) moved from its base point to `p`, mirrored in its x axis
 * when `mirror`, scaled and turned about `p`: a similarity, so shapes keep
 * their kind. Its attributes carry the values of the definition's attribute
 * definitions.
 */
export interface InsertEntity extends EntityBase {
  kind: 'insert';
  /** The definition's persistent id (UUID). */
  block: string;
  /** Where the definition's base point goes. */
  p: Vec2;
  /** Positive; 1 is the definition's size. */
  scale: number;
  /** Radians, counter-clockwise from east (as an arc's angles, not a text's degrees). */
  rotation: number;
  /** Mirrored in the definition's x axis, before the turn; absent when not (a file never holds false). */
  mirror?: boolean;
}

/**
 * A leader (docs/adr/0146): an arrowhead at its first vertex, a line through its vertices and, with a note, a
 * landing from its last vertex along the note's direction and the note past the landing's end; one object. The
 * arrowhead and the landing are measured by the note's height.
 */
export interface LeaderEntity extends EntityBase {
  kind: 'leader';
  /** At least two: the arrow's tip first, where the landing starts last. */
  pts: Vec2[];
  /** The note, one line; absent: the arrow alone (no landing, no note). Never empty. */
  text?: string;
  /** The note's height in metres, over 0; the arrowhead and the landing are measured by it. */
  height: number;
  /** The note's and the landing's direction, degrees counter-clockwise from east. */
  rotation: number;
  /** The arrowhead; absent: a filled triangle. */
  arrow?: LeaderArrow;
  /** The arrowhead's length, times the note's height (docs/adr/0205 §7); absent: 1. */
  arrowSize?: number;
  /** The note's box filled with the drawing area's colour, as a text's mask (docs/adr/0145). */
  mask?: boolean;
}

/**
 * A leader's arrowhead other than the filled triangle (Dolu üçgen, AutoCAD's Closed filled), which is the field's
 * absence (docs/adr/0146 §1): the first leaders' three, then AutoCAD's others (docs/adr/0205 §7).
 */
export type LeaderArrow =
  | 'open'
  | 'dot'
  | 'none'
  | 'closed'
  | 'open30'
  | 'open90'
  | 'dotSmall'
  | 'dotBlank'
  | 'oblique'
  | 'archTick'
  | 'boxFilled'
  | 'boxBlank'
  | 'datumFilled';

/** Every arrowhead name, in the contract's order (`LeaderArrow::ALL`). */
export const LEADER_ARROWS: readonly LeaderArrow[] = ['open', 'dot', 'none', 'closed', 'open30', 'open90', 'dotSmall', 'dotBlank', 'oblique', 'archTick', 'boxFilled', 'boxBlank', 'datumFilled'];

/** The arrowheads as the interface names them (the contract's `LeaderArrow::label`); the filled triangle under ''. */
export const LEADER_ARROW_LABEL: Readonly<Record<LeaderArrow | '', string>> = {
  '': 'Dolu üçgen',
  open: 'Açık ok',
  dot: 'Dolu nokta',
  none: 'Yok',
  closed: 'Boş üçgen',
  open30: 'İnce açık ok',
  open90: 'Dik açık ok',
  dotSmall: 'Küçük nokta',
  dotBlank: 'Boş nokta',
  oblique: 'Eğik çizgi',
  archTick: 'Mimari çentik',
  boxFilled: 'Dolu kare',
  boxBlank: 'Boş kare',
  datumFilled: 'Dayanak üçgeni',
};

/** The shortest and the longest a leader's arrowhead may be, times its note's height (docs/adr/0205 §7). */
export const MIN_LEADER_ARROW = 0.1;
export const MAX_LEADER_ARROW = 10;

/** Whether `size` may be a leader's arrowhead size (the contract's `leader_arrow_holds`). */
export const leaderArrowHolds = (size: number): boolean => Number.isFinite(size) && size >= MIN_LEADER_ARROW && size <= MAX_LEADER_ARROW;

/** A merged range of a table: `rows` × `cols` cells from row `row`, column `col` (docs/adr/0184 §1). */
export interface CellRange {
  row: number;
  col: number;
  rows: number;
  cols: number;
}

/** A table column's alignment; absent alignments: all left. */
export type TableAlign = 'left' | 'center' | 'right';

/** Which of a table's lines are drawn when not all: its outline alone, with the lines between its rows, none. */
export type TableGrid = 'outer' | 'rows' | 'none';

/** Where a table's rows came from (docs/adr/0184 §5): objects by their persistent ids, or a file by its name and sheet. */
export type TableSource =
  | { kind: 'coordinates' | 'areas' | 'attributes'; objects: string[] }
  | { kind: 'file'; name: string; sheet?: string };

/**
 * A table (docs/adr/0184 §1): rows and columns of one-line cells hanging from its top left corner, turned with it;
 * its cells' face (docs/adr/0183 §2) its own.
 */
export interface TableEntity extends EntityBase, TextFace {
  kind: 'table';
  /** Its top left corner. */
  p: Vec2;
  /** Degrees counter-clockwise from east: its rows run along it. */
  rotation: number;
  /** Its cells' text height, metres. */
  height: number;
  /** Its rows' heights, top to bottom, metres. */
  rows: number[];
  /** Its columns' widths, left to right, metres. */
  columns: number[];
  /** The cells' words, row by row, one line each, empty for an empty cell. */
  cells: string[][];
  /** Merged ranges: each range's words in its top left cell, the others empty; absent: none. */
  merges?: CellRange[];
  /** Each column's alignment; absent: all left. */
  aligns?: TableAlign[];
  /** The first row is its heading: bold, centred. */
  header?: boolean;
  /** Which lines are drawn; absent: all. */
  grid?: TableGrid;
  /** Its frame's width, metres: the outline drawn as a band that wide inside it (Kalın çerçeve); absent: a line. */
  frame?: number;
  /** Where its rows came from (Tabloyu güncelle); absent: written by hand. */
  source?: TableSource;
}

/**
 * A picture (docs/adr/0192 §1): its frame from its lower left corner `p`, `width` along and `height` up (metres), turned
 * `rotation` radians counter-clockwise about `p`, the picture upside down in it when `mirror`; its bytes the project
 * library's PNG or JPEG image `asset` (embedded) or the file `file` (linked), exactly one; `clip` the part shown in its
 * own fractions (0,0 its lower left, 1,1 its upper right); `opacity` 0.1–1.
 */
export interface ImageEntity extends EntityBase {
  kind: 'image';
  p: Vec2;
  width: number;
  height: number;
  rotation: number;
  mirror?: boolean;
  asset?: string;
  file?: string;
  clip?: Vec2[];
  opacity?: number;
}

/** A band's samples (docs/adr/0204 §2). */
export type RasterSample = 'u8' | 'i8' | 'u16' | 'i16' | 'u32' | 'i32' | 'f32' | 'f64';
/** How a raster's bands are drawn (docs/adr/0204 §4). */
export type RasterRender = 'rgb' | 'gray' | 'palette' | 'ramp' | 'hillshade' | 'rampShade';
/** How values are brought to colours; none is no field. */
export type RasterStretch = 'none' | 'minMax' | 'percent' | 'manual';

/** A raster's look (docs/adr/0204 §4): its fields' defaults are no fields. */
export interface RasterStyle {
  render: RasterRender;
  /** The bands drawn, from 1: three or four for `rgb`, one otherwise. */
  bands: number[];
  stretch?: RasterStretch;
  min?: number;
  max?: number;
  /** One of `RASTER_RAMPS`; absent: Gri. */
  ramp?: string;
  invert?: boolean;
  /** Gölgeli kabartma's light: degrees from north, clockwise; its height, degrees; heights multiplied. */
  azimuth?: number;
  altitude?: number;
  zFactor?: number;
  /** The value shown as nothing, in place of the file's. */
  nodata?: number;
  resampling?: 'bilinear' | 'nearest';
}

/**
 * A raster (docs/adr/0204 §2): an orthophoto, a scanned sheet or an elevation model whose pixels lie where `affine`
 * (`[x₀, a, b, y₀, c, d]`, GDAL's order: pixel corner (i, j) at x₀ + a·i + b·j, y₀ + c·i + d·j) puts them; its size,
 * bands and samples; its file the project library's `asset` (embedded) or `file` (linked), exactly one; the file's
 * system (0: the project's, taken by the user); its look and opacity (0.1–1). Its pixels are never in the drawing.
 */
export interface RasterEntity extends EntityBase {
  kind: 'raster';
  affine: [number, number, number, number, number, number];
  width: number;
  height: number;
  bands: number;
  sample: RasterSample;
  asset?: string;
  file?: string;
  srid: number;
  style: RasterStyle;
  opacity?: number;
}

export type Entity =
  | PointEntity
  | LineEntity
  | PolylineEntity
  | CircleEntity
  | ArcEntity
  | EllipseEntity
  | ConstructionEntity
  | SplineEntity
  | TextEntity
  | DimensionEntity
  | HatchEntity
  | InsertEntity
  | LeaderEntity
  | TableEntity
  | ImageEntity
  | RasterEntity;

/** An object of a drawing, as `CadDocument` gives it out: always with its persistent id. */
export type DrawingEntity = Entity & { uid: string };

/** Entity without slot and persistent id, as passed to CadDocument.add, which gives both. */
export type NewEntity = DistributiveOmit<Entity, 'id' | 'uid'>;

export const ENTITY_KIND_LABEL: Record<EntityKind, string> = {
  point: 'Nokta',
  line: 'Çizgi',
  polyline: 'Çoklu çizgi',
  polygon: 'Kapalı alan',
  circle: 'Daire',
  arc: 'Yay',
  ellipse: 'Elips',
  spline: 'Eğri',
  xline: 'Yardımcı çizgi',
  ray: 'Işın',
  text: 'Yazı',
  dimension: 'Ölçü',
  hatch: 'Tarama',
  insert: 'Blok',
  leader: 'Kılavuz',
  table: 'Tablo',
  image: 'Resim',
  raster: 'Raster',
};

export const HATCH_PATTERN_LABEL: Record<HatchPatternType, string> = {
  solid: 'Dolu',
  lines: 'Çizgili',
  cross: 'Çapraz',
  pattern: 'Desen',
  gradient: 'Degrade',
};

// Measures and outlines of entities, computed by the geometry core
// (docs/adr/0008, S3b). Calls that go over many objects (zoom to extents,
// the clipboard's base point, drawing, snapping) ask the geometry store,
// which holds the drawing already; these are for one object at a time.

export const tessellateCircle = op<(c: Vec2, r: number, segments?: number) => Vec2[]>('tessellateCircle');

/** Characteristic vertices — used for grips, snapping and coordinate tables. */
export const entityVertices = op<(e: EntityGeometry) => Vec2[]>('entityVertices');

/** Outline as a point list (curves tessellated, 72 segments a turn by default) — for previews, bounds and hit tests. */
export const entityOutline = op<(e: EntityGeometry, segments?: number) => Vec2[]>('entityOutline');

const ringOutline = op<(e: RingGeometry) => Vec2[]>('polygonRing');

/**
 * Closed ring of a polygon, arcs tessellated — for fills, hit tests and
 * hatching. A ring without arcs is its own vertices, and comes back as
 * they are without a call.
 */
export const polygonRing = (e: RingGeometry): Vec2[] => (e.bulges?.some((b) => b !== 0) ? ringOutline(e) : e.pts);

/** Hole rings of a polygon (arcs tessellated) or a hatch; empty for anything else. */
export const polygonHoles = op<(e: EntityGeometry) => Vec2[][]>('polygonHoles');

/** Whether p is inside a polygon's outer ring but not inside one of its holes (tessellated test). */
export const insidePolygon = op<(e: Extract<EntityGeometry, { kind: 'polyline' | 'polygon' }>, p: Vec2) => boolean>('insidePolygon');

/**
 * Rotated box of a text: its letters' advances measured in the drawing typeface
 * (`font`, Barlow without one; geometry-core `text`), one line tall.
 */
export const textBox = op<(e: { p: Vec2; text: string; height: number; rotation: number; align?: TextAlign; widthFactor?: number; font?: string; bold?: boolean; oblique?: number }) => Vec2[]>('textBox');
/**
 * Where a text's `p` is on it (the core's `TextAlign::along` and `up`, docs/adr/0145): [a share of its width along
 * it, a share of its height over its baseline]; [0, 0] without an alignment.
 */
export const textAlignShares = op<(align: TextAlign | null) => [number, number]>('textAlignShares');

/** Whether the outline is a closed ring. */
export const isClosedOutline = op<(e: EntityGeometry) => boolean>('isClosedOutline');

/** Box of an entity; construction lines count by their base point only (zoom extents ignores their reach). */
export const entityBounds = op<(e: Entity) => Bounds>('entityBounds');

/** Where a label or a value ($y, $x) sits: a polygon's centroid, a line's middle, a circle's centre…; null for a path without vertices. */
export const entityAnchor = op<(e: Entity) => Vec2 | null>('entityAnchor');

/** Length (a polygon's perimeter includes its holes, as in GIS), or null. */
export const entityLength = op<(e: Entity) => number | null>('entityLength');

/** Area (holes taken off), or null for what encloses none. */
export const entityArea = op<(e: Entity) => number | null>('entityArea');

type DistributiveOmit<T, K extends PropertyKey> = T extends unknown ? Omit<T, K> : never;

/** Pure geometry of an entity (what modify operations produce). */
export type EntityGeometry = DistributiveOmit<Entity, 'id' | 'uid' | 'layerId' | 'attrs' | 'color' | 'label' | 'symbol' | 'lineWeight'>;

/**
 * Whether an object is drawn with lines, so its own line weight shows and a
 * new one takes the current weight: not a point, a text, a dimension, a
 * hatch or a block (docs/adr/0139; the desktop's `Entity::draws_lines`).
 */
export function drawsLines(e: { kind: Entity['kind'] }): boolean {
  // A picture and a raster draw their frames as hairlines (docs/adr/0192 §3, 0204 §5).
  return (
    e.kind !== 'point' &&
    e.kind !== 'text' &&
    e.kind !== 'dimension' &&
    e.kind !== 'hatch' &&
    e.kind !== 'insert' &&
    e.kind !== 'image' &&
    e.kind !== 'raster'
  );
}

/** Geometry-only view of an entity (drops ids, layer, colour, attributes, label, symbol and line weight). */
export function entityGeometry(e: Entity): EntityGeometry {
  const { id: _i, uid: _u, layerId: _l, attrs: _a, color: _c, label: _t, symbol: _s, lineWeight: _w, ...g } = e;
  return g as EntityGeometry;
}
