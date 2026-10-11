import { op } from '../../wasm/core';
import type { EntityGeometry } from '../entities';
import type { Vec2 } from '../geometry';

/**
 * Geometri dönüştürme (docs/adr/0218): the Dönüştürme tools' points, paths and areas, computed by the geometry core
 * (`ops::convert`; the desktop's tools call the same functions). Objects go in whole, a list a call, with their
 * elevations path by path in `elevatedPaths`' order (null for an object without any); what comes back is each
 * result's geometry and elevations for the tool to put on its output layer with attributes. The independent
 * reference is scripts/fixtures/convert_cases.py.
 */

/** An object's elevations, path by path (`elevatedPaths`' order), a vertex's null when it has none. */
export type Zs = readonly (readonly (number | null)[])[];

/** A geometry the core writes and its elevations (none when no vertex has one). */
export interface Elevated {
  readonly shape: EntityGeometry;
  readonly zs?: (number | null)[][];
}

/** Nesneden nokta (§2): each object's centroid or interior point, each part's with `parts`. */
export const convertPoints = op<(entities: readonly EntityGeometry[], inside: boolean, parts: boolean) => Vec2[][]>('convertPoints');

/** A point Köşeleri noktaya çevir writes (§3): its number in the object (none for the middle), part, ring, distance along. */
export interface VertexPoint {
  readonly p: Vec2;
  readonly z?: number;
  readonly index?: number;
  readonly part: number;
  readonly ring: number;
  readonly distance: number;
}

/** Which vertices (§3). */
export type Which = 'all' | 'start' | 'end' | 'ends' | 'middle';

export const convertVertices = op<(entities: readonly EntityGeometry[], zs: readonly (Zs | null)[], which: Which) => VertexPoint[][]>('convertVertices');

/** Çizginin parçası (§8): each object's piece between its two distances; null where it has none. */
export const convertSubstring = op<(entities: readonly EntityGeometry[], zs: readonly (Zs | null)[], starts: readonly (number | null)[], ends: readonly (number | null)[]) => (Elevated | null)[]>(
  'convertSubstring',
);

/** Alanları çizgiye çevir (§9): each area's rings as lines, with their part and ring. */
export const convertRings = op<(entities: readonly EntityGeometry[], zs: readonly (Zs | null)[]) => (Elevated & { part: number; ring: number })[][]>('convertRings');

/** Çizgileri alana çevir (§10): each line's area, or why it has none. */
export type LineArea = Elevated | { readonly none: 'open' | 'empty' | 'parts' };
export const convertLineAreas = op<(entities: readonly EntityGeometry[], zs: readonly (Zs | null)[], close: boolean) => LineArea[]>('convertLineAreas');

/** Köşe ekle (§11): how an edge is divided. */
export type Density = { readonly interval: number } | { readonly count: number };

/** How many vertices Köşe ekle would add, read before anything is built. */
export const convertDensifyCount = op<(entities: readonly EntityGeometry[], by: Density) => number>('convertDensifyCount');

export const convertDensify = op<(entities: readonly EntityGeometry[], zs: readonly (Zs | null)[], by: Density) => (Elevated & { added: number })[]>('convertDensify');

/** A table row's object, or why it has none (an empty cell, no number, not more than zero, by its place; ends in one place). */
export type RowResult = { readonly shape: EntityGeometry } | { readonly fault: 'empty' | 'number' | 'positive' | 'same'; readonly column?: number };

/** Semt ve uzunluktan çizgi (§5): rows of east, north, bearing and length cells. */
export const convertBearingRows = op<(rows: readonly (readonly string[])[], unit: string) => RowResult[]>('convertBearingRows');

/** XY'den çizgi (§6): rows of the start's and the end's east and north cells. */
export const convertXyRows = op<(rows: readonly (readonly string[])[]) => RowResult[]>('convertXyRows');

/** Tablodan elips (§7): rows of east, north, the two semi-axes and the bearing cells. */
export const convertEllipseRows = op<(rows: readonly (readonly string[])[], unit: string) => RowResult[]>('convertEllipseRows');
