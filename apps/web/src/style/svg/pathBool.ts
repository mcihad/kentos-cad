import { svgOp } from './core';
import type { SubPath } from './pathData';

/**
 * Path booleans on the app's plane overlay engine, in the SVG core
 * (crates/shared/svg-core `boolean.rs`, docs/adr/0008 “SVG düzenleyicisi”).
 *
 * Curves are flattened to chords within a tolerance (a ten-thousandth of
 * the drawing's size) and every chord remembers the input segment and the
 * parameter range it came from. The overlay keeps input vertices bit for
 * bit, so each edge of a result ring lies on one input chord; runs of
 * edges on the same input segment are turned back into the exact part of
 * that Bézier (de Casteljau). **Curves are kept**: only the new corners
 * where outlines cross move to the crossing, within the tolerance.
 *
 * Fill rules: a nonzero shape is one overlay source as drawn. An even-odd
 * shape whose rings do not cross is re-oriented by nesting depth; one
 * whose rings cross is split into the faces of its line work and the faces
 * with odd winding kept.
 */

export type FillRule = 'nonzero' | 'evenodd';

export interface RegionInput {
  subs: readonly SubPath[];
  fillRule: FillRule;
}

export type BoolOp = 'union' | 'difference' | 'intersection' | 'exclusion' | 'division';

/**
 * A boolean of fill regions, bottom first. Union, intersection and
 * exclusion (odd count) use all inputs; difference takes the others away
 * from the first; division cuts the first along the others' outlines.
 * Every result is a list of sub-paths (division gives one per piece).
 */
export const booleanOp = svgOp<(op: BoolOp, inputs: readonly RegionInput[]) => SubPath[][]>('booleanOp');

/**
 * The bottom path's outline cut where the others' outlines cross it: open
 * pieces, each exactly the part of the curves it covers (Béziers split at
 * the crossings). A sub-path nothing crosses stays as it is.
 */
export const cutPath = svgOp<(target: readonly SubPath[], cutters: readonly (readonly SubPath[])[]) => SubPath[]>('cutPath');
