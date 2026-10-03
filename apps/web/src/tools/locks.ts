import type { Vec2 } from '../model/geometry';
import { op } from '../wasm/core';
import type { Tracking } from './tracking';

/**
 * The digitizing locks' arithmetic (docs/adr/0166), from the Rust core (tools/locks.rs): the locked point, the cursor
 * rules with the locks, the direction of a typed angle and of a deflection, and Dik kapat's corner. Lock text (`<45`)
 * is read here with the point grammar's number, as coordinateInput.ts reads point text; the shared cases in
 * fixtures/locks/v1/cases.json hold both readers to the same answers.
 */

/** A locked direction: its unit vector, and whether the cursor may take the other way along the line (Paralel, Dik). */
export interface LockDirection {
  readonly u: Vec2;
  readonly both: boolean;
}

/** The locked point from the reference `o` with the cursor (or a snapped point) at `c`; null when a length alone has no way. */
export const lockPoint = op<(o: Vec2, c: Vec2, length: number | null, u: Vec2 | null, both: boolean) => Vec2 | null>('lockPoint');

/** The cursor rules (ortho, polar tracking) with the locks; null when a length alone has no way to go. */
export const constrainLocked =
  op<
    (
      from: Vec2 | null,
      world: Vec2,
      exact: boolean,
      ortho: boolean,
      polarStep: number | null,
      tol: number,
      length: number | null,
      u: Vec2 | null,
      both: boolean,
    ) => { point: Vec2; tracking: Tracking | null } | null
  >('constrainLocked');

/** The unit direction of an angle typed in the project's way (ADR 0165 §4). */
export const lockDirection = op<(angle: number, fromNorth: boolean, grads: boolean) => Vec2>('lockDirection');

/** Sapma: the previous edge's direction turned by `angle` the way the project's angles run; null without an edge. */
export const lockDeflected = op<(prev: Vec2, from: Vec2, angle: number, fromNorth: boolean, grads: boolean) => Vec2 | null>('lockDeflected');

/** Dik kapat: the corner that closes a right-angled shape square; null when the first and last edge are parallel. */
export const squareCorner = op<(first: Vec2, second: Vec2, prev: Vec2, last: Vec2) => Vec2 | null>('squareCorner');

const LOCK_TEXT = /^<\s*([-+]?\d+(?:\.\d+)?)$/;

/** `<45`: the angle a typed direction lock names (in the project's way and unit); null for any other text. */
export function parseLockText(text: string): number | null {
  const m = text.trim().match(LOCK_TEXT);
  return m ? +m[1] : null;
}
