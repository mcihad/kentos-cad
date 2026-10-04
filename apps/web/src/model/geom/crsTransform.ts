import type { CrsDef } from '../../geo/crs';
import { op } from '../../wasm/core';
import type { Vec2 } from '../geometry';

/**
 * Points between the registry's coordinate systems (crates/shared/geometry-core/src/crs.rs, docs/adr/0167 §3): the
 * grids' transverse Mercator both ways, the Pseudo-Mercator, the datum shift by EPSG's Helmert transformations (the
 * ones PROJ takes by default for Türkiye), each answer with its accuracy and the operations it rests on. Latitudes and
 * longitudes in degrees, minutes and seconds or decimal degrees, written and read (§1, §4).
 */

export type Datum = 'TUREF' | 'ED50' | 'WGS84';

/** A system as the core reads it: the registry's entry without its names (geo/crs.ts makes it, `systemOf`). */
export type System =
  | { readonly kind: 'geographic'; readonly datum: Datum }
  | {
      readonly kind: 'tm';
      readonly datum: Datum;
      readonly centralMeridian: number;
      readonly scaleFactor: number;
      readonly falseEasting: number;
      readonly falseNorthing: number;
    }
  | { readonly kind: 'mercator' };

/** The core's system for a registry entry (the desktop's `transform_system`); null for the local one. */
export function systemOf(crs: CrsDef): System | null {
  if (crs.projection === 'Pseudo-Mercator') return { kind: 'mercator' };
  if (crs.datum === 'LOCAL') return null;
  if (crs.kind === 'geographic') return { kind: 'geographic', datum: crs.datum };
  if (crs.kind !== 'projected' || crs.centralMeridian === undefined) return null;
  return {
    kind: 'tm',
    datum: crs.datum,
    centralMeridian: crs.centralMeridian,
    scaleFactor: crs.scaleFactor ?? 1,
    falseEasting: crs.falseEasting ?? 0,
    falseNorthing: crs.falseNorthing ?? 0,
  };
}

/** A point moved: where it falls, how far that can be off (m; 0 where only the projection changed), what it rests on. */
export interface Transformed {
  readonly point: Vec2;
  readonly accuracy: number;
  /** The EPSG operations of the datum shift, “EPSG:1784”; empty when the datum stays. */
  readonly via: string;
}

/** `p` of `from` in `to` (x east or longitude, y north or latitude); null where a projection cannot take or give it. */
export const crsTransform = op<(from: System, to: System, p: Vec2) => Transformed | null>('crsTransform');

/** A path or a ring: its vertices and its segments' bulges (none, or a missing one, straight). */
export interface PlaneRing {
  readonly pts: readonly Vec2[];
  readonly bulges?: readonly number[] | null;
}

/** What a path (its length) or an area (its perimeter and net area) measures in a plane, or why there is none (docs/adr/0167 §2). */
export type PlaneMeasures = { readonly length: number; readonly area: number } | { readonly why: 'geographic' | 'mercator' | 'unreachable' };

/**
 * A path (`closed` false: the first ring) or an area's rings (the first the outer, the others its holes), given in
 * `from`, measured in `to`'s plane: arcs as straight pieces within 0.1 mm, the points taken into `to` (crs::measure).
 */
export const crsPlaneMeasures = op<(from: System, to: System, rings: readonly PlaneRing[], closed: boolean) => PlaneMeasures>('crsPlaneMeasures');

/** A latitude or longitude as 40°45′12.3456″K (seconds with `decimals` places; K/G, D/B). */
export const formatDms = op<(deg: number, latitude: boolean, decimals: number) => string>('formatDms');

/** A latitude or longitude as 40.7534293°K. */
export const formatDd = op<(deg: number, latitude: boolean, decimals: number) => string>('formatDd');

/** A typed latitude or longitude in degrees (decimal, degrees and minutes, or DMS; a hemisphere letter); null otherwise. */
export const parseAngle = op<(text: string) => number | null>('parseAngle');
