import { op } from '../../wasm/core';
import type { LeaderEntity } from '../entities';
import type { Vec2 } from '../geometry';

/**
 * A leader's layout (docs/adr/0146 §2): where its arrowhead, its landing and its note go, every length in the
 * note's height (the arrowhead h long and h/3 wide, a dot h/2 across, the landing 2h, the note h/2 past it). The
 * geometry core lays it out (`geom::leader::layout`), for the drawing, the pick and the tools alike.
 */

/** A leader's arrowhead, placed: a filled triangle (the tip, then its base's corners), an open one's sides, a dot. */
export type LeaderHead =
  | { readonly kind: 'filled'; readonly triangle: readonly [Vec2, Vec2, Vec2] }
  | { readonly kind: 'open'; readonly lines: readonly [Vec2, Vec2, Vec2] }
  | { readonly kind: 'dot'; readonly center: Vec2; readonly radius: number }
  | { readonly kind: 'none' };

export interface LeaderLayout {
  readonly head: LeaderHead;
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
