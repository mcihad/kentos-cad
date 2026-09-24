import { op } from '../wasm/core';
import type { Bounds, Vec2 } from './geometry';
import type { DimensionStyle } from './geom/dimension';

export type EntityKind = 'point' | 'line' | 'polyline' | 'polygon' | 'circle' | 'arc' | 'ellipse' | 'spline' | 'xline' | 'ray' | 'text' | 'dimension' | 'hatch';

/**
 * Half-length (1000 km) used when an infinite line meets finite geometry on
 * the CPU. Far beyond any sheet in a TM zone, yet small enough that float64
 * keeps ~10⁻¹⁰ m at its ends (10⁴ km would already round to 2·10⁻⁹ m).
 * Rendering clips to the view instead.
 */
export const CONSTRUCTION_REACH = 1e6;

interface EntityBase {
  id: number;
  layerId: string;
  /** Overrides the layer colour; undefined means "katmana göre" (ByLayer). */
  color?: string;
  /** Free-form GIS attributes (Ada, Parsel, Nitelik…). */
  attrs: Record<string, string>;
  /** Short label drawn at the entity's anchor (parcel number, point name). */
  label?: string;
  /** Library symbol drawn for this object, overriding its layer's style (docs/STYLE.md). */
  symbol?: string;
}

export interface PointEntity extends EntityBase {
  kind: 'point';
  p: Vec2;
  z?: number;
}
export interface LineEntity extends EntityBase {
  kind: 'line';
  a: Vec2;
  b: Vec2;
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
}
/** A closed ring of vertices with DXF bulges (outer boundary or hole). */
export interface RingGeometry {
  pts: Vec2[];
  bulges?: number[];
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
  | HatchEntity;

/** Entity without id, as passed to CadDocument.add. */
export type NewEntity = Entity extends infer E ? (E extends Entity ? Omit<E, 'id'> : never) : never;

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
 * Approximate rotated box of a text entity (Barlow averages ~0.55 em per
 * glyph). Used for picking and bounds until real glyph metrics exist.
 */
export const textBox = op<(e: { p: Vec2; text: string; height: number; rotation: number }) => Vec2[]>('textBox');

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
export type EntityGeometry = DistributiveOmit<Entity, 'id' | 'layerId' | 'attrs' | 'color' | 'label'>;

/** Geometry-only view of an entity (drops id, layer, colour, attributes, label). */
export function entityGeometry(e: Entity): EntityGeometry {
  const { id: _i, layerId: _l, attrs: _a, color: _c, label: _t, ...g } = e;
  return g as EntityGeometry;
}
