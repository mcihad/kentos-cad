import { op } from '../../wasm/core';
import type { LeaderEntity } from '../entities';
import type { Vec2 } from '../geometry';

/**
 * A leader's layout (docs/adr/0146 §2): where its arrowhead, its landing and its note go, every length in the
 * note's height (the landing 2h, the note h/2 past it); its arrowhead AutoCAD's kinds, its length the height times
 * the leader's arrowhead size (docs/adr/0205 §7). The geometry core lays it out (`geom::leader::layout`), for the
 * drawing, the pick and the tools alike.
 */

/** A leader's arrowhead, placed (`geom::arrowhead`): its filled areas (solid, never stroked) and its lines. */
export interface LeaderHead {
  readonly fills: readonly (readonly Vec2[])[];
  readonly lines: readonly { readonly pts: readonly Vec2[]; readonly closed: boolean }[];
  /** How far from the tip along the line its line starts, metres. */
  readonly back: number;
}

export interface LeaderLayout {
  readonly head: LeaderHead;
  /** Where its line starts: the tip, or on its first segment at the head's back. */
  readonly start: Vec2;
  /** The first of its vertices the line goes on to from `start`. */
  readonly first: number;
  /** 1: the landing runs along the note's direction (the last segment goes that way, or across it); −1: against it. */
  readonly side: number;
  /** From the last vertex to the landing's end; absent without a note. */
  readonly landing?: readonly [Vec2, Vec2];
  /** Where the note stands, and which point of it that is; absent without a note. */
  readonly notePoint?: Vec2;
  readonly noteAlign?: 'middleLeft' | 'middleRight';
}

/** A leader's layout; null for a leader without a vertex. */
export const leaderLayout = op<(e: LeaderEntity) => LeaderLayout | null>('leaderLayout');
