import { svgOp } from './core';
import type { RegionInput } from './pathBool';
import type { SubPath } from './pathData';

/**
 * Stroke outlines and offsets as fill regions, in the SVG core
 * (crates/shared/svg-core `stroke.rs`). A stroke is the union of simple
 * pieces: a ribbon along every smooth run of the flattened path, a join at
 * every corner (round disc, mitre within the limit, else bevel) and a cap
 * at every open end (round disc, square box, nothing for butt). Dashes cut
 * the path first. Inset and outset take the stroke of the region's outline
 * away from it or add it. The union's outline is fitted with cubics; round
 * joins and caps are exact arcs turned into cubics.
 */

export type Cap = 'butt' | 'round' | 'square';
export type Join = 'miter' | 'round' | 'bevel';

export interface StrokeStyle {
  width: number;
  cap: Cap;
  join: Join;
  /** SVG stroke-miterlimit (default 4). */
  miterLimit?: number;
  dash?: readonly number[];
}

/** The outline of a stroke as closed sub-paths (the area the stroke paints). */
export const strokeOutline = svgOp<(subs: readonly SubPath[], st: StrokeStyle) => SubPath[]>('strokeOutline');

/**
 * The fill region grown (d > 0) or shrunk (d < 0) by |d|: its outline's
 * stroke of width 2|d| added or taken away. Round joins round the corners
 * that open up (Inkscape's outset), mitre joins keep them sharp.
 */
export const offsetRegion = svgOp<(input: RegionInput, d: number, join?: Join) => SubPath[]>('offsetRegion');
