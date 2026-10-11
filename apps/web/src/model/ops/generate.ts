import { op } from '../../wasm/core';
import type { EntityGeometry } from '../entities';
import type { Vec2 } from '../geometry';

/**
 * Geometri üretimi (docs/adr/0219): the points' Voronoi cells and Delaunay triangles, groups' convex hulls, boxes,
 * rotated boxes, least circles and concave hulls, areas' poles of inaccessibility, computed by the geometry core
 * (`ops::generate`; the desktop's tools call the same functions), a list a call. The independent reference is
 * scripts/fixtures/generate_cases.py.
 */

/** A run's results, or why the core refused it. */
export type Refusable<K extends string, T> = { readonly [k in K]: T } | { readonly refused: string };

/** Each site's cell in the frame grown by `margin` (a fraction): its ring, `'same'` in an earlier site's place, null for none. */
export const generateVoronoi = op<(sites: readonly Vec2[], margin: number) => Refusable<'cells', (Vec2[] | 'same' | null)[]>>('generateVoronoi');

/** The Delaunay triangles as the sites' numbers, each counter-clockwise from its earliest site, in their sites' order. */
export const generateDelaunay = op<(sites: readonly Vec2[]) => Refusable<'triangles', [number, number, number][]>>('generateDelaunay');

/** Which bounding geometry (§4). */
export type BoundKind = 'hull' | 'box' | 'rotated' | 'circle';

/** A group's bounding geometry and its measures (the hull's area, the box's sides, the rotated box's sides and bearing, the circle's radius). */
export interface Bound {
  readonly shape: EntityGeometry;
  readonly area?: number;
  readonly width?: number;
  readonly height?: number;
  readonly length?: number;
  readonly bearing?: number;
  readonly r?: number;
}

/** Results a group or an object each (null for none) and how many objects entered with curves taken as points. */
export interface Listed<T> {
  readonly list: (T | null)[];
  readonly curved: number;
}

/** Each group's bounding geometry (`groups[i]` object i's group, below `count`). */
export const generateBounding = op<(entities: readonly EntityGeometry[], groups: readonly number[], count: number, kind: BoundKind) => Listed<Bound>>('generateBounding');

/** Each group's concave hull and its net area. */
export const generateConcave = op<
  (entities: readonly EntityGeometry[], groups: readonly number[], count: number, ratio: number, holes: boolean) => Listed<{ shape: EntityGeometry; area: number }>
>('generateConcave');

/** Each area's pole of inaccessibility and its distance from the area's edges. */
export const generatePoles = op<(entities: readonly EntityGeometry[], precision: number) => Listed<{ p: Vec2; d: number }>>('generatePoles');
