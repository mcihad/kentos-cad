import { op } from '../../wasm/core';
import type { EntityGeometry } from '../entities';
import type { Zs } from './convert';

/**
 * Kartografik genelleştirme (docs/adr/0221): smoothing, squaring, building simplification, aggregation, part
 * removal, elimination and coverage simplification, computed by the geometry core (`ops::generalize`; the desktop's
 * tools call the same functions), a list a call. Results carry their vertices and net areas before and after, read by
 * the same functions the desktop reads them with. The independent reference is scripts/fixtures/generalize_cases.py.
 */

/** A result with its elevations, its vertices and net area (m²) before and after. */
export interface Measured {
  readonly shape: EntityGeometry;
  readonly zs?: (number | null)[][] | null;
  readonly vertices: readonly [number, number];
  readonly area: readonly [number, number];
}

/** Yumuşat (§2): each object's result, null for a kind it does not take. */
export const generalizeSmooth = op<
  (entities: readonly EntityGeometry[], zs: readonly (Zs | null)[], iterations: number, offset: number, maxAngle: number, shared: boolean) => (Measured | null)[]
>('generalizeSmooth');

/** Dik açılandır (§3): each object's result, whether it moved and its largest step (m). */
export interface Squared {
  readonly shape: EntityGeometry;
  readonly zs?: (number | null)[][] | null;
  readonly moved: boolean;
  readonly shift: number;
}

export const generalizeSquare = op<(entities: readonly EntityGeometry[], zs: readonly (Zs | null)[], threshold: number, iterations: number) => (Squared | null)[]>(
  'generalizeSquare',
);

/** What Bina sadeleştir did to an object: edges went; none could; the result grew past twice; it was not valid. */
export type BuildingCase = 'changed' | 'same' | 'grew' | 'invalid';

/** Bina sadeleştir (§4): each building's shape (as it was unless changed), the edges that went, what happened. */
export interface Built {
  readonly shape: EntityGeometry;
  readonly edits: number;
  readonly case: BuildingCase;
  readonly vertices: readonly [number, number];
  readonly area: readonly [number, number];
}

export const generalizeBuilding = op<(entities: readonly EntityGeometry[], minEdge: number, parallel: number, orthogonal: number) => (Built | null)[]>(
  'generalizeBuilding',
);

/** Küçük parçaları ayıkla (§6): each area's result (its shape only when something went), the parts and holes that went. */
export interface Trimmed {
  readonly shape?: EntityGeometry;
  readonly zs?: (number | null)[][] | null;
  readonly parts: number;
  readonly holes: number;
}

export const generalizeParts = op<
  (entities: readonly EntityGeometry[], zs: readonly (Zs | null)[], minArea: number, percent: number, parts: boolean, holes: boolean) => (Trimmed | null)[]
>('generalizeParts');

/** Ortak kenarları koruyarak sadeleştir (§8): each area's result. */
export const generalizeCoverage = op<(entities: readonly EntityGeometry[], zs: readonly (Zs | null)[], tolerance: number, keepBoundary: boolean) => (Measured | null)[]>(
  'generalizeCoverage',
);

/** Alanları kümele (§5): each result's shape, its objects (indices) and its group; how many were smaller than the least area. */
export interface Aggregated {
  readonly shapes: readonly EntityGeometry[];
  readonly members: readonly (readonly number[])[];
  readonly groups: readonly string[];
  readonly dropped: number;
}

export const generalizeAggregate = op<
  (entities: readonly EntityGeometry[], groups: readonly string[], distance: number, minArea: number, minHole: number, fillNotches: boolean) => Aggregated
>('generalizeAggregate');

/**
 * Küçükleri komşuya kat (§7): the objects that took others in (indices) and their shapes now; each joining's small
 * one, the one it joined, the boundary they shared (m) and the small one's area (m²); the small ones left.
 */
export interface Eliminated {
  readonly changed: readonly number[];
  readonly shapes: readonly (EntityGeometry | null)[];
  readonly small: readonly number[];
  readonly into: readonly number[];
  readonly shared: readonly number[];
  readonly smallArea: readonly number[];
  readonly left: readonly number[];
}

export const generalizeEliminate = op<(entities: readonly EntityGeometry[], minArea: number, largest: boolean) => Eliminated>('generalizeEliminate');
