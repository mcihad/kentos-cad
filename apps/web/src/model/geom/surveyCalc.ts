import type { Vec2 } from '../geometry';
import type { AngleUnit } from '../projectSettings';
import { op } from '../../wasm/core';

/**
 * Surveying computations of the Hesap menu (poligon, kutupsal alım,
 * aplikasyon, kestirmeler), computed by the geometry core
 * (crates/shared/geometry-core/src/survey). Y is east (`x`), X north (`y`);
 * bearings (semt) run clockwise from north, horizontal angles and direction
 * readings clockwise; angles in the project's unit, distances in metres.
 * A measurement the core cannot use throws an Error with a Turkish message;
 * a value the core has not (no closure, no height) is left out, not null.
 */

export interface TraverseInput {
  unit: AngleUnit;
  start: Vec2;
  back: Vec2;
  end: Vec2 | null;
  fore: Vec2 | null;
  /** At the start, at every new point with a leg after it and, with a fore point, at the end. */
  angles: number[];
  distances: number[];
}

export interface TraverseLeg {
  bearing: number;
  distance: number;
  dy: number;
  dx: number;
  vy: number;
  vx: number;
}

export interface TraverseResult {
  /** New points, adjusted (a known end point is not repeated). */
  points: Vec2[];
  legs: TraverseLeg[];
  /** Computed − known, and the correction added to each angle. */
  angleMisclosure?: number;
  angleCorrection?: number;
  fy?: number;
  fx?: number;
  linearMisclosure?: number;
  length: number;
}

/** Poligon hesabı: angles shared out equally, coordinates by the compass rule. */
export const surveyTraverse = op<(input: TraverseInput) => TraverseResult>('surveyTraverse');

export interface Shot {
  reading: number;
  /** Horizontal, or slope when `zenith` is given. */
  distance: number;
  zenith: number | null;
  targetHeight: number | null;
}

export interface PolarInput {
  unit: AngleUnit;
  station: Vec2;
  back: Vec2;
  backReading: number;
  stationZ: number | null;
  instrumentHeight: number | null;
  shots: Shot[];
  /** The refraction coefficient k of the heights (the project's, docs/adr/0169 §3); null: no curvature or refraction. */
  refraction?: number | null;
}

export interface PolarPoint {
  p: Vec2;
  z?: number;
  bearing: number;
  horizontal: number;
  dz?: number;
}

/** Kutupsal alım: points from direction readings on a station oriented on a back point. */
export const surveyPolar = op<(input: PolarInput) => PolarPoint[]>('surveyPolar');

export interface Stake {
  bearing: number;
  distance: number;
  /** Clockwise from the back point (read as 0), when there is one. */
  angle?: number;
}

/** Aplikasyon: bearing, distance and turning angle from a station to each target. */
export const surveyStakeout = op<(input: { unit: AngleUnit; station: Vec2; back: Vec2 | null; targets: Vec2[] }) => Stake[]>('surveyStakeout');

/** Önden kestirme: α at A clockwise from B to P, β at B clockwise from P to A (P right of A → B). */
export const surveyForward = op<(unit: AngleUnit, a: Vec2, b: Vec2, alpha: number, beta: number) => Vec2>('surveyForward');

/** Geriden kestirme: at P, α clockwise from A to B and β from B to C; `strength` near 0 near the danger circle. */
export const surveyResection = op<(unit: AngleUnit, a: Vec2, b: Vec2, c: Vec2, alpha: number, beta: number) => { p: Vec2; strength: number }>('surveyResection');

/** A field book's observation as the instrument wrote it (docs/adr/0169 §2): the zenith absent for a direction only. */
export interface FieldObservation {
  target: string;
  hz: number;
  zenith?: number;
  slope?: number;
  targetHeight?: number;
  code?: string;
  /** The line of the file it came from. */
  line?: number;
}

/** A station: its name, the instrument's height above it, its observations in order. */
export interface FieldStation {
  station: string;
  instrumentHeight?: number;
  observations: FieldObservation[];
}

/**
 * A target reduced (docs/adr/0169 §3): one face or two (the observations it came from), the reading and zenith in face
 * I, the faces' differences (the reading's, the index error, the distance's), the horizontal distance and the height
 * difference station mark → target mark (earth curvature and refraction applied).
 */
export interface Reduced {
  target: string;
  faces: 1 | 2;
  observations: number[];
  hz: number;
  zenith?: number;
  slope?: number;
  hzDiff?: number;
  index?: number;
  slopeDiff?: number;
  targetHeight?: number;
  horizontal?: number;
  dh?: number;
  /** The project's tolerances its differences are above. */
  over: ('faceHz' | 'index' | 'faceSlope')[];
}

/** The project's tolerances of a pair and of a traverse leg (docs/adr/0169 §3), radians and metres; an absent one is not checked. */
export interface Tolerances {
  faceHz?: number;
  index?: number;
  faceSlope?: number;
  twoWay?: number;
}

/** A station's reduction: its targets in order, the observations left out (a zenith that is no face), each observation's face (1, 2, 0 for a direction only, null for none). */
export interface Reduction {
  rows: Reduced[];
  problems: { observation: number; problem: 'zenith' }[];
  faces: (number | null)[];
}

/** A shot for Kutupsal alım: the target, its reading, slope distance and zenith in the project's unit, its target height. */
export interface PolarShot {
  name: string;
  reading: number;
  slope: number;
  zenith: number;
  targetHeight?: number;
}

/** Kutupsal alım's fields from a station's reduction (docs/adr/0169 §3): the back sight and its reading, the shots, the targets left out. */
export interface PolarTransfer {
  back: string;
  backReading: number;
  shots: PolarShot[];
  left: string[];
}

/** A station reduced and turned into Kutupsal alım's fields: the `back` row the back sight, the angles in `to` (the core's `polar_transfer`). */
export const fieldPolar = op<(station: FieldStation, unit: AngleUnit, k: number, tolerances: Tolerances | null, back: number, to: AngleUnit) => PolarTransfer | null>('fieldPolar');

/** A leg of a field book's traverse: its two stations, the horizontal distance measured from each end, their mean and difference (forward − backward), whether that is above the two-way tolerance. */
export interface BookLeg {
  from: string;
  to: string;
  forward?: number;
  backward?: number;
  mean?: number;
  diff?: number;
  over: boolean;
}

/** Whether a traverse's misclosures are above the project's tolerances (docs/adr/0169 §3); absent without a misclosure or a tolerance. */
export interface Closure {
  angleOver?: boolean;
  coordOver?: boolean;
}

/** The angular misclosure (in the unit) against `angle` (radians), the linear (fs) against `coord` (metres): the core's `traverse::closure`. */
export const surveyTraverseClosure = op<(unit: AngleUnit, angleMisclosure: number | null, linearMisclosure: number | null, angle: number | null, coord: number | null) => Closure>('surveyTraverseClosure');

/** Poligon hesabı's angles and legs from a field book's stations (docs/adr/0169 §3): the targets a station has no row for named. */
export interface TraverseTransfer {
  stations: string[];
  angles: (number | null)[];
  legs: BookLeg[];
  missing: { station: string; target: string }[];
}

/** A book's stations reduced and turned into Poligon hesabı's fields: ST1 oriented on `back`, STn on `fore` (null: none), the angles in `to` (the core's `traverse_transfer`). */
export const fieldTraverse = op<(stations: FieldStation[], unit: AngleUnit, k: number, tolerances: Tolerances | null, back: string, fore: string | null, to: AngleUnit) => TraverseTransfer>('fieldTraverse');

/** A station's observations reduced in the unit with the refraction coefficient `k`, its pairs checked against the tolerances (the core's `survey::fieldbook`). */
export const fieldReduce = op<(station: FieldStation, unit: AngleUnit, k: number, tolerances: Tolerances | null) => Reduction>('fieldReduce');
