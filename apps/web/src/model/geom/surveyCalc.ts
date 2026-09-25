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
