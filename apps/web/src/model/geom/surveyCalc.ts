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
}

/** A station's reduction: its targets in order, the observations left out (a zenith that is no face). */
export interface Reduction {
  rows: Reduced[];
  problems: { observation: number; problem: 'zenith' }[];
}

/** A station's observations reduced in the unit with the refraction coefficient `k` (the core's `survey::fieldbook`). */
export const fieldReduce = op<(station: FieldStation, unit: AngleUnit, k: number) => Reduction>('fieldReduce');
