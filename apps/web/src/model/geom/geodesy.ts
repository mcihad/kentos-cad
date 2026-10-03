import { op } from '../../wasm/core';
import type { Vec2 } from '../geometry';

/**
 * Projection arithmetic of the geometry core (crates/shared/geometry-core/src/geodesy.rs, docs/adr/0165 §3): a
 * latitude and longitude on a transverse Mercator grid, the registry's TM3 and UTM zones, for a new project's start
 * view on a province's centre.
 */

/** A transverse Mercator projection with its ellipsoid. */
export interface Tm {
  /** Degrees east of Greenwich. */
  readonly centralMeridian: number;
  readonly scaleFactor: number;
  readonly falseEasting: number;
  readonly falseNorthing: number;
  /** The ellipsoid's semi-major axis, metres. */
  readonly semiMajor: number;
  readonly inverseFlattening: number;
}

/** The point at `lat`, `lon` (degrees) on the grid, east and north; null where the projection cannot reach. */
export const tmForward = op<(p: Tm, lat: number, lon: number) => Vec2 | null>('tmForward');
