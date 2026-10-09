import { op } from '../../wasm/core';
import type { Entity, NewEntity } from '../entities';
import type { Vec2 } from '../geometry';

/**
 * Eğrilere kot ver (docs/adr/0234 §9): the core's `ops::contour_elevations`, one rule for both platforms. Each curve's
 * elevation by the order a cut from `start` to `end` crosses them (each curve at its crossing nearest the start; equal
 * places in the curves' order): the first `first`, each next `step` more; null for a curve the cut does not cross.
 * The independent reference is scripts/fixtures/raster_vector_cases.py.
 */
export const contourElevations = op<(curves: readonly (Entity | NewEntity)[], start: Vec2, end: Vec2, first: number, step: number) => (number | null)[]>(
  'contourElevations',
);
