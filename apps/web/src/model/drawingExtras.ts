import { op } from '../wasm/core';
import type { Entity } from './entities';
import type { Vec2 } from './geometry';

/**
 * Çizim ekleri (docs/adr/0197): the shared core's rules (`tools::drawing_extras`), as İki daireye teğet, Dördüncü köşe
 * and Menzil halkaları call them.
 */

/** Which common tangent: outer ones leave both circles on one side, inner ones cross between; left and right as seen from the first centre. */
export type TangentKind = 'outerLeft' | 'outerRight' | 'innerLeft' | 'innerRight';

/** A common tangent, from its point on the first circle to its point on the second. */
export interface Tangent {
  kind: TangentKind;
  a: Vec2;
  b: Vec2;
}

/** Two circles' or arcs' common tangents (§1): outer left, outer right, inner left, inner right, those there are; none for other kinds. */
export const commonTangents = op<(first: Entity, second: Entity) => Tangent[]>('commonTangents');

/** The tangent clicks at `p1` on the first and `p2` on the second choose (§1), its points nearest them; null when there is none. */
export const chosenTangent = op<(first: Entity, second: Entity, p1: Vec2, p2: Vec2) => Tangent | null>('chosenTangent');

/** A parallelogram's fourth corner (§2), opposite `b`, the middle of three. */
export const fourthCorner = op<(a: Vec2, b: Vec2, c: Vec2) => Vec2>('fourthCorner');

/** Range rings (§3): `count` rings `spacing` metres apart and `rays` rays to the outer one from north clockwise; null out of range. */
export const rangeRings = op<(center: Vec2, spacing: number, count: number, rays: number) => { radii: number[]; rays: Vec2[] } | null>('rangeRings');
