import { svgOp } from './core';
import type { Box, Matrix, Pt } from './pathData';
import { shapeId, type SvgShape } from './svgModel';

/**
 * Arranging shapes in the SVG editor: align and distribute (Inkscape's
 * Align and Distribute), the numeric transforms (move, scale, rotate,
 * skew, matrix; together or each separately) and arrays for pattern and
 * symbol design (rectangular, polar, mirror copy, as a CAD array). A
 * group moves as one. Everything here gives matrices or new shapes; the
 * editor applies them as one undo step. Computed by the SVG core
 * (crates/shared/svg-core `arrange.rs`); the stacking order is a list
 * reordered here.
 */

/** Things that move together: a group, or a shape on its own; in the order they were chosen. */
export interface Unit {
  ids: string[];
  box: Box;
}

/** The chosen shapes as units, in the order of `order` (the selection's order). */
export const unitsOf = svgOp<(shapes: readonly SvgShape[], order: readonly string[]) => Unit[]>('unitsOf');

export type AlignTo = 'selection' | 'first' | 'last' | 'biggest' | 'smallest' | 'canvas';
export type AlignSide = 'left' | 'hcenter' | 'right' | 'top' | 'vcenter' | 'bottom';

/** The box the units align against. One unit alone aligns to the canvas. */
export const alignReference = svgOp<(units: readonly Unit[], to: AlignTo, page: { width: number; height: number }) => Box>('alignReference');

/**
 * One translation per unit putting its edge or centre on the reference's.
 * `asOne` moves the whole selection as one block (useful against the canvas).
 */
export const alignMoves = svgOp<(units: readonly Unit[], side: AlignSide, to: AlignTo, page: { width: number; height: number }, asOne?: boolean) => Matrix[]>('alignMoves');

export type Distribute = 'left' | 'hcenter' | 'right' | 'hgap' | 'top' | 'vcenter' | 'bottom' | 'vgap';

/**
 * Even spacing between the outermost units (they stay): of edges or
 * centres, or equal gaps between neighbouring boxes.
 */
export const distributeMoves = svgOp<(units: readonly Unit[], how: Distribute) => Matrix[]>('distributeMoves');

// ── Transforms ─────────────────────────────────────────────────────────

/** A box point: corners, edge middles, centre. */
export type Anchor = 'tl' | 't' | 'tr' | 'l' | 'c' | 'r' | 'bl' | 'b' | 'br';

export const anchorPoint = svgOp<(b: Box, a: Anchor) => Pt>('anchorPoint');

export const scaleAbout = svgOp<(sx: number, sy: number, p: Pt) => Matrix>('scaleAbout');

/** Skew by angles (degrees): x leans with y by `ax`, y with x by `ay`, about p. */
export const skewAbout = svgOp<(ax: number, ay: number, p: Pt) => Matrix>('skewAbout');

/** Rotation by `deg`, counter-clockwise on screen when `ccw` (the drawing's y runs down). */
export const rotateAbout = svgOp<(deg: number, p: Pt, ccw: boolean) => Matrix>('rotateAbout');

export type TransformSpec =
  | { kind: 'move'; x: number; y: number; relative: boolean }
  | { kind: 'scale'; sx: number; sy: number; anchor: Anchor }
  | { kind: 'rotate'; deg: number; ccw: boolean; about: Anchor | Pt }
  | { kind: 'skew'; ax: number; ay: number; anchor: Anchor }
  | { kind: 'matrix'; m: Matrix };

/**
 * The matrix of every unit. Together, the selection's box is the frame;
 * separately, each unit's own box (a relative move then steps each unit
 * one move further than the one before, spreading them out).
 */
export const transformMoves = svgOp<(units: readonly Unit[], spec: TransformSpec, separately: boolean) => Matrix[]>('transformMoves');

/** A matrix is usable: finite and not flattening everything to a line. */
export const invertible = svgOp<(m: Matrix) => boolean>('invertible');

// ── Arrays ─────────────────────────────────────────────────────────────

export interface RectArraySpec {
  rows: number;
  cols: number;
  /** Column and row spacing: centre to centre (`step`) or the gap between boxes (`gap`). */
  dx: number;
  dy: number;
  mode: 'step' | 'gap';
}

/** Offsets of every copy of a rectangular array (the original, row 0 column 0, is left out). */
export const rectArray = svgOp<(box: Box, spec: RectArraySpec) => Matrix[]>('rectArray');

export interface PolarArraySpec {
  count: number;
  /** Angle the copies fill (360: a full turn, evenly spaced). */
  angle: number;
  centre: Pt;
  /** Copies turn with their place (as a wheel); off, they only move. */
  rotate: boolean;
  ccw: boolean;
}

export const polarArray = svgOp<(box: Box, spec: PolarArraySpec) => Matrix[]>('polarArray');

/** Reflection across a line through p: vertical, horizontal, or at `deg` from the x axis. */
export const mirrorMatrix = svgOp<(axis: 'v' | 'h' | 'angle', p: Pt, deg?: number) => Matrix>('mirrorMatrix');

const copiesCore = svgOp<(shapes: readonly SvgShape[], matrices: readonly Matrix[]) => { shapes: SvgShape[]; ids: number }>('copiesOf');

/**
 * Copies of the shapes under each matrix: new ids, and each copy's groups
 * its own. The core names the new ids "\u0001<k>" in the order they are
 * made; they are made here (`shapeId`).
 */
export function copiesOf(shapes: readonly SvgShape[], matrices: readonly Matrix[]): SvgShape[] {
  const r = copiesCore(shapes, matrices);
  const ids = Array.from({ length: r.ids }, () => shapeId());
  const fresh = (v: string) => (v.charCodeAt(0) === 1 ? ids[Number(v.slice(1))] : v);
  for (const s of r.shapes) {
    s.id = fresh(s.id);
    if (s.group !== undefined) s.group = fresh(s.group);
  }
  return r.shapes;
}

// ── Stacking order ─────────────────────────────────────────────────────

/** The chosen shapes one step up or down past a neighbour, or to the top or bottom (list is back to front). */
export function restack(list: readonly SvgShape[], ids: ReadonlySet<string>, op: 'raise' | 'lower' | 'top' | 'bottom'): SvgShape[] {
  const sel = list.filter((s) => ids.has(s.id));
  const rest = list.filter((s) => !ids.has(s.id));
  if (op === 'top') return [...rest, ...sel];
  if (op === 'bottom') return [...sel, ...rest];
  const out = [...list];
  const order = op === 'raise' ? [...out.keys()].reverse() : [...out.keys()];
  for (const i of order) {
    const j = op === 'raise' ? i + 1 : i - 1;
    if (ids.has(out[i].id) && j >= 0 && j < out.length && !ids.has(out[j].id)) [out[i], out[j]] = [out[j], out[i]];
  }
  return out;
}
