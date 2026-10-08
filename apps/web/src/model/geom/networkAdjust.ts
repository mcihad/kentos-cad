import { op } from '../../wasm/core';
import type { AngleUnit } from '../projectSettings';
import type { FieldStation, Grid, Tolerances } from './surveyCalc';

/**
 * Ağ dengelemesi ve kot ağı (docs/adr/0203), computed by the geometry core (crates/shared/geometry-core/src/survey/
 * adjust; the desktop calls the same functions): a horizontal network of directions and distances, a levelling network
 * of height differences, both by least squares with their statistics. Y is east, X north; directions are readings in
 * the project's unit; residuals and standard deviations in radians and metres. A network the core cannot adjust throws
 * an Error with the Turkish reason (a refusal names the table's line). The independent reference is
 * scripts/fixtures/network_adjust_cases.py.
 */

/** The a priori standard deviations: radians, metres, parts per million, m per √km. */
export interface Sigmas {
  direction: number;
  distance: number;
  ppm: number;
  centering: number;
  zenith: number;
  levelling: number;
}

/** A known point: fixed, or weighted with its σ (m). */
export interface KnownPoint {
  name: string;
  y: number;
  x: number;
  sigma?: number | null;
}

/** An observation row: a direction reading in the project's unit, a horizontal distance (m), either or both; its table line. */
export interface NetRow {
  station: string;
  target: string;
  direction?: number | null;
  distance?: number | null;
  line?: number | null;
}

export interface NetworkInput {
  unit: AngleUnit;
  sigma: Sigmas;
  grid?: Grid | null;
  known: KnownPoint[];
  /** The drawing's places of new points: where their approximations start. */
  approx: { name: string; y: number; x: number }[];
  rows: NetRow[];
}

/** An observation adjusted: its kind, its row (or known point), the residual (computed − observed), its σ, r, w, flag. */
export interface ObservationResult {
  kind: 'direction' | 'distance' | 'y' | 'x' | 'dh' | 'h';
  row: number;
  v: number;
  sigma: number;
  r: number;
  w?: number;
  flag: 'ok' | 'blunder' | 'uncontrolled';
}

export interface AdjustedPoint {
  name: string;
  y: number;
  x: number;
  sy: number;
  sx: number;
  sp: number;
  a: number;
  b: number;
  /** The error ellipse's major axis as a bearing, radians in [0, π). */
  theta: number;
}

/** The counts and tests both adjustments give. */
export interface Statistics {
  observations: ObservationResult[];
  worst?: number;
  n: number;
  u: number;
  f: number;
  omega: number;
  m0?: number;
  chi2?: number;
  passed?: boolean;
  iterations: number;
}

export interface NetworkResult extends Statistics {
  points: AdjustedPoint[];
  orientations: { station: string; z: number }[];
}

export interface KnownHeight {
  name: string;
  h: number;
  sigma?: number | null;
}

export interface LevelRow {
  from: string;
  to: string;
  dh: number;
  length: number;
  line?: number | null;
}

export interface LevelInput {
  kind: 'geometric' | 'trigonometric';
  sigma: Sigmas;
  known: KnownHeight[];
  rows: LevelRow[];
}

export interface LevelResult extends Statistics {
  points: { name: string; h: number; sh: number }[];
}

/** The horizontal network adjusted (the core's `survey::adjust::horizontal`). */
export const networkAdjust = op<(input: NetworkInput) => NetworkResult>('networkAdjust');

/** The levelling network adjusted (the core's `survey::adjust::levelling`). */
export const levelAdjust = op<(input: LevelInput) => LevelResult>('levelAdjust');

/** The χ² quantile the model test takes. */
export const chi2Quantile = op<(f: number, p: number) => number>('chi2Quantile');

/** Yatay ağ dengelemesi's rows from a book's stations, the directions in `to` (docs/adr/0203 §6). */
export const fieldNetwork = op<(stations: FieldStation[], unit: AngleUnit, k: number, tolerances: Tolerances | null, to: AngleUnit) => NetRow[]>('fieldNetwork');

/** Kot ağı dengelemesi's rows from a book's stations: the height differences with their horizontal distances. */
export const fieldLevels = op<(stations: FieldStation[], unit: AngleUnit, k: number, tolerances: Tolerances | null) => LevelRow[]>('fieldLevels');
