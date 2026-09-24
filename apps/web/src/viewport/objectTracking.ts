import type { Vec2 } from '../model/geometry';
import { op } from '../wasm/core';

/**
 * Object snap tracking ("nesne izleme"), computed by the Rust core
 * (crates/shared/geometry-core/src/tools/object_tracking.rs, docs/adr/0008 S5).
 * Points acquired by resting the cursor on a snap emit alignment lines
 * (horizontal/vertical, plus polar steps when polar tracking is on). The
 * cursor locks onto the nearest line, or onto the crossing of two lines
 * from different origins — "above this corner and level with that one".
 * The viewport feeds it world coordinates and a world tolerance.
 */

export interface TrackLine {
  origin: Vec2;
  /** Degrees, counter-clockwise from east. */
  angle: number;
}

export interface TrackHit {
  point: Vec2;
  /** One line, or two when the point is where two alignments cross. */
  lines: TrackLine[];
}

/** Alignment directions: always 0/90/180/270, plus every polar step. */
export const trackAngles = op<(polarStep: number | null) => number[]>('trackAngles');

/**
 * Where the cursor `p` locks to, if anywhere within `tol`. `acquired` are
 * the tracking points; `from` (the command's last point) only takes part
 * in crossings, since polar tracking already covers lines through it.
 */
export const trackPoint = op<(p: Vec2, acquired: readonly Vec2[], from: Vec2 | null, angles: readonly number[], tol: number) => TrackHit | null>('trackPoint');

/** Point `distance` along a single tracking line from its origin (typed distance while tracking). */
export const alongTrack = op<(hit: TrackHit, distance: number) => Vec2 | null>('alongTrack');
