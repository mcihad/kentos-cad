import { op } from '../../wasm/core';
import type { EntityGeometry } from '../entities';
import type { Vec2 } from '../geometry';

/**
 * Mekânsal istatistik (docs/adr/0238): the processing tools' runs, computed by the geometry core
 * (`ops::spatial_stats`; the desktop's tools call the same functions). Objects go in as their geometry (only it is
 * read: an object's place is its `shape_centroid`), values and weights as attribute texts read by
 * `kentos.statistics/1`. What comes back is everything a tool writes and says, the numbers already written as texts
 * so both platforms show the same. The independent reference is scripts/fixtures/spatial_stats_cases.py.
 */

/** A shape the core writes: a centre, a standard distance's circle, a deviational ellipse. */
export type StatsShape =
  | { readonly kind: 'point'; readonly p: Vec2 }
  | { readonly kind: 'circle'; readonly c: Vec2; readonly r: number }
  | { readonly kind: 'ellipse'; readonly c: Vec2; readonly major: Vec2; readonly ratio: number; readonly t0: number; readonly t1: number };

/** A run's answer: its summary, notes, table, numbers, new objects, and each object's copy (its place in the input, the attributes added, the colour). */
export interface StatsRun {
  readonly summary: string;
  readonly infos: readonly string[];
  readonly warnings: readonly string[];
  readonly table?: { readonly columns: string[]; readonly rows: string[][] };
  readonly numbers: readonly { readonly name: string; readonly value: number }[];
  readonly objects: readonly { readonly shape: StatsShape; readonly attrs: readonly [string, string][] }[];
  readonly copies: readonly { readonly index: number; readonly attrs: readonly [string, string][]; readonly color: string }[];
}

/** Ortalama ve ortanca merkez, Standart uzaklık, Yön dağılımı (§3–§5); a refusal is thrown with its reason. */
export const statsCenters = op<
  (
    entities: readonly EntityGeometry[],
    kind: 'mean' | 'median' | 'distance' | 'ellipse',
    weights: readonly (string | null)[] | null,
    groups: readonly (string | null)[] | null,
    weightField: string,
    k: number,
  ) => StatsRun
>('statsCenters');

/** En yakın komşu (§6): `area` m², null for the places' box. */
export const statsNearest = op<(entities: readonly EntityGeometry[], area: number | null) => StatsRun>('statsNearest');

/** The neighbourhood's concept (§7). */
export type StatsConcept = 'band' | 'inverse' | 'nearest';

/** Moran I (§8): `band` null for the largest nearest-neighbour distance. */
export const statsMoran = op<
  (entities: readonly EntityGeometry[], values: readonly (string | null)[], field: string, concept: StatsConcept, band: number | null, k: number, standardize: boolean) => StatsRun
>('statsMoran');

/** Sıcak nokta, Getis-Ord Gi* (§9). */
export const statsHotSpots = op<
  (entities: readonly EntityGeometry[], values: readonly (string | null)[], field: string, concept: StatsConcept, band: number | null, k: number) => StatsRun
>('statsHotSpots');

/** DBSCAN (§10). */
export const statsDbscan = op<(entities: readonly EntityGeometry[], radius: number, minPoints: number, borderNoise: boolean) => StatsRun>('statsDbscan');

/** k-ortalamalar (§10). */
export const statsKMeans = op<(entities: readonly EntityGeometry[], k: number) => StatsRun>('statsKMeans');
