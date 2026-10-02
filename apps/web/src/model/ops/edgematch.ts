import { op } from '../../wasm/core';
import type { Vec2 } from '../geometry';
import type { PathElevations } from './warp';

/**
 * Kenar eşleme (docs/adr/0159): the core's `ops::edgematch`, one for both platforms. The line ends of two sheets that
 * meet across their common edge are found as links (the best continuation within a search distance and an angle;
 * junctions and connected ends left out) and put together: an end moved, a segment added, or the vertices adjusted by
 * a shift that fades along the line; at the adjacent end, in the middle or on a border. The independent reference is
 * scripts/fixtures/edgematch_cases.py (mpmath, 50 digits).
 */

/** An object of a set: its shape, its paths' elevations, its key for the criterion (a layer's name, an attribute). */
export interface EdgeMember<S> {
  shape: S;
  zs: PathElevations;
  key?: string | null;
}

/** What the links are found with: metres, degrees, the sheet's border, whether keys must match. */
export interface EdgeSettings<S> {
  distance: number;
  angle: number;
  border?: S | null;
  keyed: boolean;
}

export type EdgeEnd = 'first' | 'last';

/** A source end and the adjacent end it meets; the gap (m), the angle between their directions (degrees), the score. */
export interface EdgeLink {
  source: number;
  sourceEnd: EdgeEnd;
  adjacent: number;
  adjacentEnd: EdgeEnd;
  from: Vec2;
  to: Vec2;
  gap: number;
  angle: number;
  score: number;
}

/** The links; the free source ends near the edge left unmatched; the junction ends there; the objects taking no part. */
export interface EdgeFound {
  links: EdgeLink[];
  unmatched: { source: number; end: EdgeEnd }[];
  junctions: number;
  others: number;
}

export type EdgeMeet = 'adjacent' | 'middle' | 'border';
export type EdgeMethod = 'move' | 'segment' | 'adjust';

/** Every changed member's shape and elevations (null when unchanged), and the links not written (their indices). */
export interface EdgeApplied<S> {
  sources: (S | null)[];
  sourceZs: (PathElevations | null)[];
  adjacent: (S | null)[];
  adjacentZs: (PathElevations | null)[];
  refused: number[];
}

export const edgematchLinks = op<<S>(sources: readonly EdgeMember<S>[], adjacent: readonly EdgeMember<S>[], settings: EdgeSettings<S>) => EdgeFound>('edgematchLinks');

/** `links` written where they meet and by `method`; Sınırda without a border is `{ error: 'no_border' }`. */
export const edgematchApply = op<
  <S>(sources: readonly EdgeMember<S>[], adjacent: readonly EdgeMember<S>[], links: readonly EdgeLink[], meet: EdgeMeet, method: EdgeMethod, border: S | null) => EdgeApplied<S> | { error: 'no_border' }
>('edgematchApply');
