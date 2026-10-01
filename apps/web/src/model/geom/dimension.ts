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
 *   ordinate  a point's Y (`angle` 0) or X (90), its line across the axis to b
 *   arcLength the arc about `c` from a counter-clockwise to b's direction
 *   jogged    a radius (a the true centre, b on the arc) drawn from the centre shown `c`
 *   azimuth   the bearing from a to b, an arrow beside the edge
 *   slope     the slope from a to b between their elevations `za`, `zb`
 *             (the last five: docs/adr/0147)
 *
 * The layout is computed by the geometry core (docs/adr/0008).
 */
/** The last five came with KCAD schema 9 (docs/adr/0147): Koordinat, Yay uzunluğu, Kırıklı yarıçap, Semt, Eğim. */
export type DimensionStyle = 'aligned' | 'linear' | 'angular' | 'radius' | 'diameter' | 'ordinate' | 'arcLength' | 'jogged' | 'azimuth' | 'slope';

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
  /** linear: direction measured along, degrees CCW from east; ordinate: 0 its Y, 90 its X. */
  angle?: number;
  /** angular: the vertex; arc length: the arc's centre; jogged: the centre the line starts from. */
  c?: Vec2;
  /** slope: the two points' elevations, metres (docs/adr/0147). */
  za?: number;
  zb?: number;
}

/** What a dimension's value is: a length, an angle, a percentage (a slope) or a coordinate (a point's Y or X). */
export type DimensionUnit = 'length' | 'angle' | 'percent' | 'coordinate';

export interface DimensionLayout {
  /** Extension lines, dimension line (an arc as chords) and oblique ticks. */
  lines: [Vec2, Vec2][];
  /** Dimension line (or arc) end points. */
  d1: Vec2;
  d2: Vec2;
  /** Text anchor (centre of text baseline area) and readable rotation in degrees. */
  textAt: Vec2;
  rotation: number;
  /** Measured value: metres (a length, a point's Y or X), radians for an angle, a slope in percent. */
  value: number;
  unit: DimensionUnit;
  /** Written before the value: "R " for a radius, "Ø " for a diameter; "Y=", "X=", "t=", "%" (docs/adr/0147). */
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
  ordinate: 'Koordinat',
  arcLength: 'Yay uzunluğu',
  jogged: 'Kırıklı yarıçap',
  azimuth: 'Semt',
  slope: 'Eğim',
};

/** Extension and dimension lines, ticks, text place and pick edges of a dimension; null when degenerate. */
export const layoutDimension = op<(d: DimensionGeom) => DimensionLayout | null>('layoutDimension');

/** Signed perpendicular distance of p from the line a→b (positive = left). */
export const signedOffset = op<(a: Vec2, b: Vec2, p: Vec2) => number>('signedOffset');

/** The `offset` that puts the dimension line (arc, leader end) through p. */
export const dimensionOffsetAt = op<(d: DimensionGeom, p: Vec2) => number>('dimensionOffsetAt');

/** Why a dimension of docs/adr/0147 cannot be drawn (the core's `DimensionFault`). */
export type DimensionFault =
  | 'ordinateTooShort'
  | 'arcNoCentre'
  | 'arcNoRadius'
  | 'arcNoSweep'
  | 'arcInside'
  | 'joggedNoCentre'
  | 'joggedNoRadius'
  | 'joggedCentre'
  | 'edgeTooShort'
  | 'slopeNoElevations';

/** Why a new kind cannot be drawn, exactly when `layoutDimension` gives none (finite numbers); null when it can and for the older kinds. */
export const dimensionFault = op<(d: DimensionGeom) => DimensionFault | null>('dimensionFault');

/**
 * The text a dimension shows: its override, or prefix + value in project units; a coordinate is written as a length
 * without its unit (the formatter's `coord`), a slope as a percentage.
 */
export function dimensionLabel(text: string | undefined, l: Pick<DimensionLayout, 'prefix' | 'unit' | 'value'>, fmt: { length: (m: number) => string; angle: (rad: number) => string; percent: (v: number) => string }): string {
  if (text) return text;
  return `${l.prefix}${l.unit === 'angle' ? fmt.angle(l.value) : l.unit === 'percent' ? fmt.percent(l.value) : fmt.length(l.value)}`;
}

/**
 * What a style measures and writes before its value, as the core's layout says (`dimension_measure`, docs/adr/0147
 * §2): for a block's dimension pieces, whose label records carry no unit. An ordinate's axis is its `angle`: 0 (or
 * none) its Y, else its X. `dimension.test.ts` holds it to the core for every style.
 */
export function dimensionMeasure(style: DimensionStyle | undefined, angle: number | undefined): Pick<DimensionLayout, 'unit' | 'prefix'> {
  switch (style) {
    case 'angular':
      return { unit: 'angle', prefix: '' };
    case 'radius':
    case 'jogged':
      return { unit: 'length', prefix: 'R ' };
    case 'diameter':
      return { unit: 'length', prefix: 'Ø ' };
    case 'ordinate':
      return { unit: 'coordinate', prefix: (angle ?? 0) === 0 ? 'Y=' : 'X=' };
    case 'azimuth':
      return { unit: 'angle', prefix: 't=' };
    case 'slope':
      return { unit: 'percent', prefix: '%' };
    default:
      return { unit: 'length', prefix: '' };
  }
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
