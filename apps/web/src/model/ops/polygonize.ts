import { op } from '../../wasm/core';
import type { Entity } from '../entities';
import type { Vec2 } from '../geometry';
import type { Area } from '../geom/region';

/**
 * Toplu alan (docs/adr/0151): the core's `ops::polygonize`. The regions the line work closes (the face index Tarama
 * and İçine tıklayarak alan use), smallest first, with the closed groups inside each as holes when `islands`; each
 * label goes to the smallest region whose outer ring holds it, unless it lies within 1 µm of a ring; a region whose
 * rings are an input area's names it; and the free ends of open paths are listed. The independent reference is
 * scripts/fixtures/polygonize_cases.py.
 */

/** A label: where it is (a text's box middle, a point) and its value. */
export interface PolyLabel {
  at: Vec2;
  value: string;
}

export interface PolyRegion {
  area: Area;
  /** The labels in it, by their places in the input. */
  labels: number[];
  /** The input area whose rings it repeats, by its place among the lines; absent when new. */
  existing?: number;
}

export interface PolyResult {
  regions: PolyRegion[];
  /** The labels within 1 µm of a region's or a group's ring. */
  onBoundary: number[];
  /** The ends of open paths that touch nothing. */
  freeEnds: Vec2[];
}

/** Toplu alan's finding over the line work `lines` (in the drawing's order; other kinds are passed over) and `labels`. */
export const polygonize = op<(lines: readonly Entity[], labels: readonly PolyLabel[], islands: boolean) => PolyResult>('polygonize');
