import { svgOp } from './core';
import type { Pt, SubPath } from './pathData';

/**
 * Node editing of a path (Inkscape's node tool, and the CAD corner tools):
 * node types (cusp, smooth, symmetric, auto), new nodes in the middle of
 * segments, deleting nodes while keeping the shape, joining and breaking,
 * deleting segments, segments to lines or curves, fillet and chamfer of a
 * corner, aligning and distributing nodes. A node is addressed by its
 * sub-path and index; every function returns new sub-paths and leaves its
 * input alone. Computed by the SVG core (crates/shared/svg-core `nodes.rs`).
 */

export interface NodeRef {
  sub: number;
  index: number;
}

export type NodeType = 'cusp' | 'smooth' | 'symmetric' | 'auto';

export const refKey = (r: NodeRef) => `${r.sub}:${r.index}`;

type Edited = { subs: SubPath[]; refs: NodeRef[] };

/** The node's type: stored, or read from its handles (in line → smooth, in line and equal → symmetric). */
export const nodeTypeOf = svgOp<(sp: SubPath, i: number) => NodeType>('nodeTypeOf');

const refreshCore = svgOp<(subs: readonly SubPath[]) => SubPath[]>('refreshAuto');

/** Recomputes the handles of auto nodes (after nodes moved), in the given list. */
export function refreshAuto(subs: SubPath[]): SubPath[] {
  subs.splice(0, subs.length, ...refreshCore(subs));
  return subs;
}

/** Sets the type of the chosen nodes, making their handles fit it. */
export const setNodeType = svgOp<(subs: readonly SubPath[], refs: readonly NodeRef[], type: NodeType) => SubPath[]>('setNodeType');

/** Moves the chosen nodes (their handles with them); auto nodes follow. */
export const moveNodes = svgOp<(subs: readonly SubPath[], refs: readonly NodeRef[], dx: number, dy: number) => SubPath[]>('moveNodes');

/** A new node in the middle (t = ½) of every segment between two chosen nodes; the new nodes join the choice. */
export const insertMidNodes = svgOp<(subs: readonly SubPath[], refs: readonly NodeRef[]) => Edited>('insertMidNodes');

/**
 * Deletes the chosen nodes. With `keepShape` the curve over each removed
 * run is replaced by one cubic fitted to it (end tangents kept), as
 * Inkscape does; otherwise the neighbours are joined as they are. A
 * sub-path left with too few nodes goes away.
 */
export const deleteNodes = svgOp<(subs: readonly SubPath[], refs: readonly NodeRef[], keepShape?: boolean) => SubPath[]>('deleteNodes');

/**
 * Joins two chosen end nodes: into one node at their middle (`merge`) or
 * with a straight segment between them. Ends of one sub-path close it;
 * ends of two sub-paths make one.
 */
export const joinEnds = svgOp<(subs: readonly SubPath[], refs: readonly NodeRef[], merge: boolean) => Edited | { error: string }>('joinEnds');

/** Breaks the path at the chosen nodes: a closed sub-path opens there, an open one splits in two. */
export const breakAtNodes = svgOp<(subs: readonly SubPath[], refs: readonly NodeRef[]) => SubPath[]>('breakAtNodes');

/** Removes the segments between chosen neighbours: the path opens or splits there. */
export const deleteSegments = svgOp<(subs: readonly SubPath[], refs: readonly NodeRef[]) => SubPath[] | { error: string }>('deleteSegments');

/** The segments between chosen neighbours as straight lines or as (straight-looking) curves ready to bend. */
export const segmentsTo = svgOp<(subs: readonly SubPath[], refs: readonly NodeRef[], kind: 'line' | 'curve') => SubPath[]>('segmentsTo');

// ── Corners ────────────────────────────────────────────────────────────

/** What a corner node offers a fillet or chamfer: its angle and the longest cut along each side. */
export interface Corner {
  /** Angle between the two sides (radians, 0..π; π is straight on). */
  angle: number;
  /** Unit directions from the node along the incoming and outgoing sides. */
  back: Pt;
  ahead: Pt;
  /** Longest distance the cut may reach along each side (the neighbours' distance). */
  max: number;
}

export const cornerAt = svgOp<(sp: SubPath, i: number) => Corner | null>('cornerAt');

/** Tangent distance (from the corner along each side) of a fillet of radius r. */
export const filletDistance = svgOp<(c: Corner, r: number) => number>('filletDistance');
/** Radius of the fillet reaching distance d along each side. */
export const filletRadius = svgOp<(c: Corner, d: number) => number>('filletRadius');

/**
 * Rounds (fillet: `size` is the radius) or cuts (chamfer: `size` is the
 * distance along each side) the corner at the chosen nodes. Sides may be
 * curves; the cut points are where the sides are that far from the
 * corner, and the fillet is the circular arc tangent to both.
 */
export const cornerNodes = svgOp<(subs: readonly SubPath[], refs: readonly NodeRef[], mode: 'fillet' | 'chamfer', size: number) => Edited | { error: string }>('cornerNodes');

// ── Align and distribute nodes ─────────────────────────────────────────

/** Moves the chosen nodes onto one line: their smallest, middle or largest x (or y). */
export const alignNodes = svgOp<(subs: readonly SubPath[], refs: readonly NodeRef[], axis: 'x' | 'y', to: 'min' | 'mid' | 'max') => SubPath[]>('alignNodes');

/** Spaces the chosen nodes evenly between the outermost ones along x (or y). */
export const distributeNodes = svgOp<(subs: readonly SubPath[], refs: readonly NodeRef[], axis: 'x' | 'y') => SubPath[]>('distributeNodes');
