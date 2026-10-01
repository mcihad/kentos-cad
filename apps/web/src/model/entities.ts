import { op } from '../wasm/core';
import type { Bounds, Vec2 } from './geometry';
import type { DimensionStyle } from './geom/dimension';

export type EntityKind = 'point' | 'line' | 'polyline' | 'polygon' | 'circle' | 'arc' | 'ellipse' | 'spline' | 'xline' | 'ray' | 'text' | 'dimension' | 'hatch' | 'insert' | 'leader';

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
   * A multi-part area's parts past its first, whose own are the fields above
   * (docs/adr/0143); undefined for a one-part area, never on a polyline.
   */
  parts?: AreaPart[];
}
/** A part of a multi-part area past its first: its ring, arcs, holes and elevations, as a polygon's own. */
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
 * measured value when set.
 */
export interface DimensionEntity extends EntityBase {
  kind: 'dimension';
  a: Vec2;
  b: Vec2;
  offset: number;
  height: number;
  text?: string;
  style?: DimensionStyle;
  /** linear: measured direction, degrees CCW from east (0 = ΔY, 90 = ΔX). */
  angle?: number;
  /** angular: the vertex. */
  c?: Vec2;
}
export type HatchPatternType = 'solid' | 'lines' | 'cross';
export interface HatchPattern {
  type: HatchPatternType;
  /** Line direction in degrees, CCW from east. */
  angle: number;
  /** Line spacing in metres (world units). */
  spacing: number;
}
/** Filled or line-patterned area inside a boundary ring (not associative). */
export interface HatchEntity extends EntityBase {
  kind: 'hatch';
  ring: Vec2[];
  /** Islands left unhatched (the holes of a holed polygon). */
  holes?: Vec2[][];
  pattern: HatchPattern;
}
export interface TextEntity extends EntityBase {
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
  /** The arrowhead; absent: a filled arrow. */
  arrow?: LeaderArrow;
  /** The note's box filled with the drawing area's colour, as a text's mask (docs/adr/0145). */
  mask?: boolean;
}

/** A leader's arrowhead other than the filled arrow, which is the field's absence (docs/adr/0146 §1). */
export type LeaderArrow = 'open' | 'dot' | 'none';

/** Every arrowhead name, in the contract's order (`LeaderArrow::ALL`). */
export const LEADER_ARROWS: readonly LeaderArrow[] = ['open', 'dot', 'none'];

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
  | LeaderEntity;

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
};

export const HATCH_PATTERN_LABEL: Record<HatchPatternType, string> = {
  solid: 'Dolu',
  lines: 'Çizgili',
  cross: 'Çapraz',
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
export const textBox = op<(e: { p: Vec2; text: string; height: number; rotation: number; font?: string }) => Vec2[]>('textBox');
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
  return e.kind !== 'point' && e.kind !== 'text' && e.kind !== 'dimension' && e.kind !== 'hatch' && e.kind !== 'insert';
}

/** Geometry-only view of an entity (drops ids, layer, colour, attributes, label, symbol and line weight). */
export function entityGeometry(e: Entity): EntityGeometry {
  const { id: _i, uid: _u, layerId: _l, attrs: _a, color: _c, label: _t, symbol: _s, lineWeight: _w, ...g } = e;
  return g as EntityGeometry;
}
