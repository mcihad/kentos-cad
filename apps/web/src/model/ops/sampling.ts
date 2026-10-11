import { op } from '../../wasm/core';
import type { EntityGeometry } from '../entities';
import type { Elevated, Zs } from './convert';

/**
 * Örnekleme ve ağ üretimi (docs/adr/0220): grids of cells and lines over the objects' box, regular and seeded random
 * points in areas and on paths, points along paths and objects cut into pieces of at most so many vertices, computed
 * by the geometry core (`ops::sampling`; the desktop's tools call the same functions), a list a call. Grids and points
 * come back as flat lists of numbers: a hundred thousand cells cross as four arrays, not as objects. The independent
 * reference is scripts/fixtures/sampling_cases.py.
 */

/** A run's results, or why the core refused it. */
export type Refusable<K extends string, T> = { readonly [k in K]: T } | { readonly refused: string };

/** Ağ oluştur's shapes (§2). */
export type GridShape = 'rect' | 'hexagon' | 'triangle' | 'lines';

/** A grid as flat lists: each piece's row and column (0: none), its number of points, then every point's x and y. */
export interface GridPieces {
  readonly rows: readonly number[];
  readonly cols: readonly number[];
  readonly counts: readonly number[];
  readonly xy: readonly number[];
}

/** Ağ oluştur (§2) over the objects' box; `keep` keeps the cells that meet them. */
export const sampleGrid = op<
  (entities: readonly EntityGeometry[], kind: GridShape, dx: number, dy: number, side: number, snap: boolean, keep: boolean) => Refusable<'grid', GridPieces>
>('sampleGrid');

/**
 * Points as flat lists: each point's object, its x and y; elevations, distances, bearings and paths (from 1 in an
 * object of more than one, 0 otherwise) where the tool gives them; Rastgele noktalar's points the least distance
 * refused, too thin areas and the points given up in them.
 */
export interface SampledPoints {
  readonly at: readonly number[];
  readonly xy: readonly number[];
  readonly z?: readonly (number | null)[];
  readonly s?: readonly (number | null)[];
  readonly bearing?: readonly number[];
  readonly path?: readonly number[];
  readonly tooClose?: number;
  readonly thin?: number;
  readonly thinLost?: number;
}

/** Düzenli noktalar (§3): each area's lattice points, shifted up to `shift` by the seed's numbers. */
export const sampleRegular = op<
  (entities: readonly EntityGeometry[], dx: number, dy: number, snap: boolean, shift: number, seed: number) => Refusable<'points', SampledPoints>
>('sampleRegular');

/** Rastgele noktalar (§4): `counts[i]` points for object i (null: unread), at least `min` apart. */
export const sampleRandom = op<
  (entities: readonly EntityGeometry[], zs: readonly (Zs | null)[], counts: readonly (number | null)[], min: number, tries: number, seed: number) => Refusable<'points', SampledPoints>
>('sampleRandom');

/** Hat boyunca nokta (§5): every `step` from `start` up to `end` before each path's end, or at `parts` equal parts. */
export const sampleAlong = op<
  (
    entities: readonly EntityGeometry[],
    zs: readonly (Zs | null)[],
    mode: 'interval' | 'parts',
    step: number,
    start: number,
    end: number,
    parts: number,
    ends: boolean,
  ) => Refusable<'points', SampledPoints>
>('sampleAlong');

/** Alt bölümlere ayır (§6): each object's pieces in order, with their elevations. */
export const sampleSubdivide = op<(entities: readonly EntityGeometry[], zs: readonly (Zs | null)[], max: number) => Elevated[][]>('sampleSubdivide');
