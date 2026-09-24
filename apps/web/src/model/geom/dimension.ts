import { op } from '../../wasm/core';
import type { Vec2 } from '../geometry';
import type { Edge } from './intersect';

/**
 * Dimensions, drawn the way cadastral sheets do it: extension lines with a
 * small gap from the measured points, the dimension line (or arc) with
 * oblique ticks, and the value above it, always reading left-to-right.
 *
 *   aligned   a–b along their own direction (default)
 *   linear    a–b projected on a fixed direction (`angle`; 0 = ΔY yatay,
 *             90 = ΔX düşey), dimension line parallel to it
 *   angular   the angle at vertex `c` from the arm through a to the arm
 *             through b, counter-clockwise; `offset` is the arc radius
 *   radius    a = centre, b = point on the circle; `offset` extends the
 *             line past the circle (leader)
 *   diameter  as radius, the line running through the centre
 *
 * The layout is computed by the geometry core (docs/adr/0008).
 */
export type DimensionStyle = 'aligned' | 'linear' | 'angular' | 'radius' | 'diameter';

export interface DimensionGeom {
  /** Measured points (radius, diameter: a is the centre, b on the circle). */
  a: Vec2;
  b: Vec2;
  /**
   * aligned, linear: signed distance of the dimension line (positive =
   * left of the measured direction); angular: arc radius; radius,
   * diameter: leader length past the circle.
   */
  offset: number;
  /** Text height in metres; gaps and ticks are proportional to it. */
  height: number;
  style?: DimensionStyle;
  /** linear: direction measured along, degrees CCW from east. */
  angle?: number;
  /** angular: the vertex. */
  c?: Vec2;
}

export interface DimensionLayout {
  /** Extension lines, dimension line (an arc as chords) and oblique ticks. */
  lines: [Vec2, Vec2][];
  /** Dimension line (or arc) end points. */
  d1: Vec2;
  d2: Vec2;
  /** Text anchor (centre of text baseline area) and readable rotation in degrees. */
  textAt: Vec2;
  rotation: number;
  /** Measured value: metres, or radians for an angle. */
  value: number;
  unit: 'length' | 'angle';
  /** Written before the value: "R " for a radius, "Ø " for a diameter. */
  prefix: string;
  /** Edges that pick and snap: the dimension line or arc. */
  pick: Edge[];
  /** Where the grip that moves the dimension line sits. */
  handle: Vec2;
}

export const DIMENSION_STYLE_LABEL: Record<DimensionStyle, string> = {
  aligned: 'Hizalı',
  linear: 'Doğrusal',
  angular: 'Açı',
  radius: 'Yarıçap',
  diameter: 'Çap',
};

/** Extension and dimension lines, ticks, text place and pick edges of a dimension; null when degenerate. */
export const layoutDimension = op<(d: DimensionGeom) => DimensionLayout | null>('layoutDimension');

/** Signed perpendicular distance of p from the line a→b (positive = left). */
export const signedOffset = op<(a: Vec2, b: Vec2, p: Vec2) => number>('signedOffset');

/** The `offset` that puts the dimension line (arc, leader end) through p. */
export const dimensionOffsetAt = op<(d: DimensionGeom, p: Vec2) => number>('dimensionOffsetAt');

/** The text a dimension shows: its override, or prefix + value in project units. */
export function dimensionLabel(text: string | undefined, l: Pick<DimensionLayout, 'prefix' | 'unit' | 'value'>, fmt: { length: (m: number) => string; angle: (rad: number) => string }): string {
  if (text) return text;
  return `${l.prefix}${l.unit === 'angle' ? fmt.angle(l.value) : fmt.length(l.value)}`;
}

/**
 * Horizontal (ΔY, 0°) or vertical (ΔX, 90°) for a linear dimension placed
 * at p, as AutoCAD decides it: beside the measured points → vertical,
 * above or below them → horizontal.
 */
export const linearAngleFor = op<(a: Vec2, b: Vec2, p: Vec2) => 0 | 90>('linearAngleFor');

/**
 * The angle two lines make, chosen by where the arc goes: the two arm
 * directions bounding the sector around `p` (counter-clockwise order),
 * from the vertex `c`. Lines are given by a direction each.
 */
export const sectorArms = op<(c: Vec2, u1: Vec2, u2: Vec2, p: Vec2) => [Vec2, Vec2]>('sectorArms');
