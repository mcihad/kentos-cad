import { svgOp } from './core';

/**
 * SVG path data as editable nodes: every command (lines, cubic and
 * quadratic Béziers, smooth variants, elliptic arcs) becomes nodes with
 * optional cubic handles, in absolute coordinates. A segment from node i
 * to i+1 is a cubic with controls (i.out ?? i) and (i+1.in ?? i+1): both
 * missing makes it a straight line. Computed by the SVG core
 * (crates/shared/svg-core `path.rs`, docs/adr/0008 “SVG düzenleyicisi”).
 */

export type Pt = readonly [number, number];

export interface PathNode {
  x: number;
  y: number;
  /** Incoming handle (control point before this node), absolute. */
  in?: Pt;
  /** Outgoing handle (control point after this node), absolute. */
  out?: Pt;
  /**
   * How the node editor keeps its handles: cusp (free), smooth (in line),
   * symmetric (in line, equal), auto (from the neighbours). Absent: read
   * from the handles. Editing state only; the file does not keep it.
   */
  type?: 'cusp' | 'smooth' | 'symmetric' | 'auto';
}

export interface SubPath {
  nodes: PathNode[];
  closed: boolean;
}

/** An affine matrix as SVG writes it: x' = a·x + c·y + e, y' = b·x + d·y + f. */
export type Matrix = readonly [number, number, number, number, number, number];

export const IDENTITY: Matrix = [1, 0, 0, 1, 0, 0];

/** m after n (n applied first). */
export const multiply = svgOp<(m: Matrix, n: Matrix) => Matrix>('multiply');

export const apply = svgOp<(m: Matrix, x: number, y: number) => [number, number]>('apply');

/** Nodes of every subpath in `d`; unknown text is skipped, a broken tail ends the path. */
export const parsePathData = svgOp<(d: string) => SubPath[]>('parsePathData');

/**
 * Cubic segments approximating an SVG arc (endpoint form, F.6.5 of the SVG
 * spec), each spanning at most 90°: [c1x, c1y, c2x, c2y, x, y] each.
 */
export const arcToCubics = svgOp<(x1: number, y1: number, rx: number, ry: number, rotDeg: number, large: boolean, sweep: boolean, x2: number, y2: number) => number[][]>('arcToCubics');

/** Path data of subpaths, absolute, lines as L and curves as C. */
export const pathDataOf = svgOp<(subs: readonly SubPath[], digits?: number) => string>('pathDataOf');

export const transformSubPaths = svgOp<(subs: readonly SubPath[], m: Matrix) => SubPath[]>('transformSubPaths');

export interface Box {
  minX: number;
  minY: number;
  maxX: number;
  maxY: number;
}

/** Points along every segment (curves sampled), for bounds and hit tests. */
export const flattenSubPath = svgOp<(sp: SubPath, steps?: number) => [number, number][]>('flattenSubPath');

export const subPathsBox = svgOp<(subs: readonly SubPath[]) => Box>('subPathsBox');
