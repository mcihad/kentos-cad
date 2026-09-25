import { svgOp } from './core';
import type { BoolOp } from './pathBool';
import type { SubPath } from './pathData';
import type { Join } from './pathStroke';
import { shapeId, type SvgShape } from './svgModel';

/**
 * The SVG editor's path operations (Inkscape's Path menu) on shapes:
 * booleans (union, difference, intersection, exclusion, division, cut
 * path), combine and break apart, object to path, stroke to path, inset
 * and outset, simplify, reverse, close and open, in the SVG core
 * (crates/shared/svg-core `ops.rs`). Booleans keep curves as curves;
 * stroke outlines and offsets are fitted with cubics within a
 * ten-thousandth of the drawing. Every operation returns the shapes to put
 * in place of the ones it used, or a message for the user.
 */

export type PathOp = BoolOp | 'cut' | 'combine' | 'breakApart' | 'split' | 'toPath' | 'strokeToPath' | 'reverse' | 'close' | 'open';

export type OpResult = { add: SvgShape[]; remove: string[]; note?: string } | { error: string };

/**
 * An operation's result with its new shapes' ids made here (time and a
 * counter, `shapeId`): the core names them "\u0001<k>", k the order the
 * ids are made in, and says how many it named.
 */
function withIds(r: { result: OpResult; ids: number }): OpResult {
  if (!r.ids || 'error' in r.result) return r.result;
  const ids = Array.from({ length: r.ids }, () => shapeId());
  const fresh = (v: string) => (v.charCodeAt(0) === 1 ? ids[Number(v.slice(1))] : v);
  for (const s of r.result.add) {
    s.id = fresh(s.id);
    if (s.group !== undefined) s.group = fresh(s.group);
  }
  return r.result;
}

type Named = { result: OpResult; ids: number };
const booleanCore = svgOp<(op: BoolOp, shapes: readonly SvgShape[]) => Named>('booleanShapes');
const cutCore = svgOp<(shapes: readonly SvgShape[]) => Named>('cutShapes');
const combineCore = svgOp<(shapes: readonly SvgShape[]) => Named>('combineShapes');
const breakCore = svgOp<(shape: SvgShape, keepHoles: boolean) => Named>('breakApart');
const toPathCore = svgOp<(shapes: readonly SvgShape[]) => Named>('shapesToPath');
const strokeCore = svgOp<(shapes: readonly SvgShape[]) => Named>('strokeToPath');
const offsetCore = svgOp<(shapes: readonly SvgShape[], d: number, join: Join) => Named>('offsetShapes');
const simplifyCore = svgOp<(shapes: readonly SvgShape[], tol: number) => Named>('simplifyShapes');
const reverseCore = svgOp<(shapes: readonly SvgShape[]) => Named>('reverseShapes');
const closeCore = svgOp<(shapes: readonly SvgShape[]) => Named>('closeShapes');
const openCore = svgOp<(shapes: readonly SvgShape[]) => Named>('openShapes');

/**
 * A boolean on shapes given bottom to top. The result takes the bottom
 * shape's style and place (Inkscape does the same); division gives one
 * shape per piece.
 */
export const booleanShapes = (op: BoolOp, shapes: readonly SvgShape[]): OpResult => withIds(booleanCore(op, shapes));

/** The bottom shape's outline cut where the others cross it (open pieces, no fill). */
export const cutShapes = (shapes: readonly SvgShape[]): OpResult => withIds(cutCore(shapes));

/** Every selected shape's sub-paths in one path (the bottom shape's style). */
export const combineShapes = (shapes: readonly SvgShape[]): OpResult => withIds(combineCore(shapes));

/**
 * Sub-paths as separate shapes. With `keepHoles` a hole stays with the
 * sub-path around it (the letter O stays one shape).
 */
export const breakApart = (shape: SvgShape, keepHoles: boolean): OpResult => withIds(breakCore(shape, keepHoles));

/** Rectangles and ellipses as editable paths (texts cannot: there are no letter outlines here). */
export const shapesToPath = (shapes: readonly SvgShape[]): OpResult => withIds(toPathCore(shapes));

/**
 * The stroke as a filled outline (its paint becomes the fill). A shape that
 * also has a fill keeps it as a shape below the outline, grouped with it.
 */
export const strokeToPath = (shapes: readonly SvgShape[]): OpResult => withIds(strokeCore(shapes));

/** Each shape's fill grown (d > 0) or shrunk (d < 0) by |d| drawing units. */
export const offsetShapes = (shapes: readonly SvgShape[], d: number, join: Join = 'round'): OpResult => withIds(offsetCore(shapes, d, join));

/**
 * Fewer nodes within `tol`: every sub-path flattened and fitted again with
 * cubics; corners sharper than `cornerDeg` stay corners.
 */
export const simplifySubPath = svgOp<(sp: SubPath, tol: number, cornerDeg?: number) => SubPath>('simplifySubPath');

export const simplifyShapes = (shapes: readonly SvgShape[], tol: number): OpResult => withIds(simplifyCore(shapes, tol));

export const reverseShapes = (shapes: readonly SvgShape[]): OpResult => withIds(reverseCore(shapes));

/** Open sub-paths closed (an end on the start merges into it). */
export const closeSubPath = svgOp<(sp: SubPath) => SubPath>('closeSubPath');

/** Closed sub-paths opened at their first node, keeping the shape (the closing segment stays). */
export const openSubPath = svgOp<(sp: SubPath) => SubPath>('openSubPath');

export const closeShapes = (shapes: readonly SvgShape[]): OpResult => withIds(closeCore(shapes));
export const openShapes = (shapes: readonly SvgShape[]): OpResult => withIds(openCore(shapes));

/** Puts an operation's result into the list: new shapes where the bottom-most removed one was. */
export function applyResult(list: readonly SvgShape[], r: Extract<OpResult, { add: SvgShape[] }>): SvgShape[] {
  const gone = new Set(r.remove);
  const replaced = new Map(r.add.filter((s) => gone.has(s.id)).map((s) => [s.id, s]));
  const fresh = r.add.filter((s) => !gone.has(s.id) || !replaced.has(s.id));
  // Shapes changed in place keep their place; new ones go where the bottom-most removed shape was.
  const firstGone = list.findIndex((s) => gone.has(s.id));
  const out: SvgShape[] = [];
  list.forEach((s, i) => {
    if (i === firstGone) out.push(...fresh.filter((f) => !replaced.has(f.id)));
    if (gone.has(s.id)) {
      const k = replaced.get(s.id);
      if (k) out.push(k);
    } else out.push(s);
  });
  if (firstGone < 0) {
    // Nothing removed (object to path): same-id shapes swap in place, the rest go on top.
    const byId = new Map(r.add.map((s) => [s.id, s]));
    return [...list.map((s) => byId.get(s.id) ?? s), ...r.add.filter((s) => !list.some((x) => x.id === s.id))];
  }
  return out;
}
