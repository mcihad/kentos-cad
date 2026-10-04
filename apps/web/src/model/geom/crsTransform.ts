import type { CrsDef } from '../../geo/crs';
import { op } from '../../wasm/core';
import type { Vec2 } from '../geometry';

/**
 * Points between coordinate systems (crates/shared/geometry-core/src/crs.rs, docs/adr/0167 §3, docs/adr/0168): the
 * grids' transverse Mercator both ways (of any origin), the Pseudo-Mercator, a local system's plane transform to its
 * base, the datum shift by EPSG's Helmert transformations (the ones PROJ takes by default for Türkiye), a datum of the
 * project's through WGS 84, the project's own choices; each answer with its accuracy and what it rests on, or why
 * there is none. Latitudes and longitudes in degrees, minutes and seconds or decimal degrees, written and read.
 */

/** Which way a Helmert transformation's rotations turn: EPSG's position vector (9606) or coordinate frame (9607). */
export type Convention = 'positionVector' | 'coordinateFrame';

/** Seven parameters: translations (m), rotations (″) in their convention, the scale difference (ppm), the accuracy (m). */
export interface Helmert {
  readonly translation: readonly [number, number, number];
  readonly rotation: readonly [number, number, number];
  readonly scale: number;
  readonly convention: Convention;
  readonly accuracy?: number;
}

/** An ellipsoid: its name, semi-major axis (m) and inverse flattening. */
export interface Ellipsoid {
  readonly name: string;
  readonly semiMajor: number;
  readonly inverseFlattening: number;
}

/** A datum the project defines (docs/adr/0168 §2): without a way to WGS 84 it stands alone. */
export interface CustomDatum {
  readonly name: string;
  readonly ellipsoid: Ellipsoid;
  readonly toWgs84?: Helmert;
}

export type RegistryDatum = 'TUREF' | 'ED50' | 'WGS84';
export type Datum = RegistryDatum | CustomDatum;

/** A local system's coordinates to its base's: base x = a·x + b·y + c, base y = d·x + e·y + f (a similarity turns counter-clockwise, in degrees). */
export type Plane =
  | { readonly kind: 'similarity'; readonly east: number; readonly north: number; readonly rotation: number; readonly scale: number }
  | { readonly kind: 'affine'; readonly a: number; readonly b: number; readonly c: number; readonly d: number; readonly e: number; readonly f: number };

/** A system as the core reads it: the registry's entry without its names (`systemOf`), or the project's definition. */
export type System =
  | { readonly kind: 'geographic'; readonly datum: Datum }
  | {
      readonly kind: 'tm';
      readonly datum: Datum;
      /** Degrees; 0 when left out. */
      readonly latitudeOfOrigin?: number;
      readonly centralMeridian: number;
      readonly scaleFactor: number;
      readonly falseEasting: number;
      readonly falseNorthing: number;
    }
  | { readonly kind: 'mercator' }
  | { readonly kind: 'local'; readonly base: System; readonly plane: Plane };

/**
 * The project's choice for a pair of the registry's datums instead of EPSG's way (docs/adr/0168 §3): seven parameters,
 * or an NTv2 grid loaded under `id` (its SHA-256, `crsLoadGrid`) with the accuracy the project gives it.
 */
export type DatumChoice = {
  readonly from: RegistryDatum;
  readonly to: RegistryDatum;
  /** What the values rest on: “ED50 → TUREF: Bölge 7”. */
  readonly name: string;
} & ({ readonly helmert: Helmert } | { readonly grid: { readonly id: string; readonly accuracy?: number } });

/** The core's system for a registry entry (the desktop's `transform_system`); null for the local one. */
export function systemOf(crs: CrsDef): System | null {
  if (crs.projection === 'Pseudo-Mercator') return { kind: 'mercator' };
  if (crs.datum === 'LOCAL') return null;
  if (crs.kind === 'geographic') return { kind: 'geographic', datum: crs.datum };
  if (crs.kind !== 'projected' || crs.centralMeridian === undefined) return null;
  return {
    kind: 'tm',
    datum: crs.datum,
    centralMeridian: crs.centralMeridian,
    scaleFactor: crs.scaleFactor ?? 1,
    falseEasting: crs.falseEasting ?? 0,
    falseNorthing: crs.falseNorthing ?? 0,
  };
}

/**
 * A point moved: where it falls, how far that can be off (m; 0 where only the projection changed; left out when a
 * step's is not known), what it rests on, and whether an EPSG operation of ED50 was used (not the official values).
 */
export interface Transformed {
  readonly point: Vec2;
  readonly accuracy?: number;
  /** The datum shift's operations and the project's names, “EPSG:1784”, “Bessel datumu + EPSG:5261”; empty when the datum stays. */
  readonly via: string;
  readonly unofficial: boolean;
}

/**
 * Why a point has no value in another system: a projection cannot take or give it, a datum with no way to WGS 84 stands
 * between, the project's grid is not loaded here, or the point is outside it.
 */
export type Unreached = 'outside' | 'noLink' | 'noGrid' | 'outsideGrid';

/** `p` of `from` in `to` (x east or longitude, y north or latitude); null where a projection cannot take or give it. */
export const crsTransform = op<(from: System, to: System, p: Vec2) => Transformed | null>('crsTransform');

/** `p` of `from` in `to`, the project's datum choices taken where they apply; or why it has no value there. */
export const crsTransformIn = op<(from: System, to: System, p: Vec2, choices?: readonly DatumChoice[]) => Transformed | { readonly error: Unreached }>(
  'crsTransformIn',
);

/** A path or a ring: its vertices and its segments' bulges (none, or a missing one, straight). */
export interface PlaneRing {
  readonly pts: readonly Vec2[];
  readonly bulges?: readonly number[] | null;
}

/** What a path (its length) or an area (its perimeter and net area) measures in a plane, or why there is none (docs/adr/0167 §2). */
export type PlaneMeasures =
  | { readonly length: number; readonly area: number }
  | { readonly why: 'geographic' | 'mercator' | 'unreachable' | 'noLink' | 'noGrid' };

/**
 * A path (`closed` false: the first ring) or an area's rings (the first the outer, the others its holes), given in
 * `from`, measured in `to`'s plane: arcs as straight pieces within 0.1 mm, the points taken into `to` (crs::measure),
 * the project's datum choices taken where they apply.
 */
export const crsPlaneMeasures =
  op<(from: System, to: System, rings: readonly PlaneRing[], closed: boolean, choices?: readonly DatumChoice[]) => PlaneMeasures>('crsPlaneMeasures');

/**
 * What a path or an area measures in the project's plane, on its ellipsoid and on the ground at the project's mean
 * ellipsoidal height (docs/adr/0171 §1): a path's area values and a value without its ground (no height) or its plane
 * (a geographic system) are missing; `scale` is the plane's over the ellipsoid's (an area's ratio's square root),
 * `heightFactor` the ellipsoid's over the ground's; or why there is none.
 */
export type GroundMeasures =
  | {
      readonly planeLength?: number;
      readonly planeArea?: number;
      readonly ellipsoidLength: number;
      readonly ellipsoidArea?: number;
      readonly groundLength?: number;
      readonly groundArea?: number;
      readonly scale?: number;
      readonly heightFactor?: number;
    }
  | { readonly why: 'unreachable' };

/**
 * A path (`closed` false: the first ring) or an area's rings (the first the outer, the others its holes) in the
 * project's `system`, in its plane, on its ellipsoid (geodesics, GeographicLib's polygon and the arcs beside it) and,
 * with the project's mean ellipsoidal `height` (m), on the ground (crs::ground).
 */
export const crsGroundMeasures =
  op<(system: System, rings: readonly PlaneRing[], closed: boolean, height: number | null) => GroundMeasures>('crsGroundMeasures');

/** Whether the system has one scale at a point, so that lengths can be taken to its grid (docs/adr/0171 §3). */
export const crsHasPointScale = op<(system: System) => boolean>('crsHasPointScale');

/** The projection's scale at a point of the system's plane (docs/adr/0171 §3); null where it has no one scale. */
export const crsPointScale = op<(system: System, p: Vec2) => number | null>('crsPointScale');

/** A line's geodesic length, its scale (Simpson's of its ends' and middle's) and, with a height, R/(R + h) at its middle. */
export type LineFactors =
  | { readonly ellipsoidLength: number; readonly scale?: number; readonly heightFactor?: number }
  | { readonly why: 'unreachable' };

/** The line from `a` to `b` in the project's `system`: its factors between grid and ground (docs/adr/0171 §3). */
export const crsLineFactors = op<(system: System, a: Vec2, b: Vec2, height: number | null) => LineFactors>('crsLineFactors');

/** A latitude or longitude as 40°45′12.3456″K (seconds with `decimals` places; K/G, D/B). */
export const formatDms = op<(deg: number, latitude: boolean, decimals: number) => string>('formatDms');

/** A latitude or longitude as 40.7534293°K. */
export const formatDd = op<(deg: number, latitude: boolean, decimals: number) => string>('formatDd');

/** A typed latitude or longitude in degrees (decimal, degrees and minutes, or DMS; a hemisphere letter); null otherwise. */
export const parseAngle = op<(text: string) => number | null>('parseAngle');
