import { svgOp } from './core';
import { multiply, parsePathData, type Box, type Matrix, type SubPath } from './pathData';

/**
 * The drawing model of KentOS's own SVG editor (docs/STYLE.md §7): a
 * canvas (viewBox) and shapes on it, back to front. Rectangles, ellipses
 * and texts keep their kind (so they stay editable as such); everything
 * else is a path of Bézier nodes. Paints are "the symbol's colour"
 * (currentColor, recoloured by the symbol), "the second colour"
 * (param(stroke)), a fixed colour or none. Pure: the editor's view and the
 * SVG file are both made from this. The geometry (elements and the file's
 * numbers, boxes, transforms, shapes as paths) is the SVG core's
 * (crates/shared/svg-core `model.rs`).
 */

/** 'fill': the symbol's colour; 'stroke': the symbol's second colour; '#RRGGBB[AA]': fixed; 'none'. */
export type Paint = 'none' | 'fill' | 'stroke' | string;

interface ShapeBase {
  id: string;
  fill: Paint;
  stroke: Paint;
  strokeWidth: number;
  opacity?: number;
  /** Stroke dash lengths (on, off …); absent = continuous. */
  dash?: number[];
  /** Line ends and corners; absent = round (paths) or the SVG default (other shapes). */
  cap?: 'butt' | 'round' | 'square';
  join?: 'miter' | 'round' | 'bevel';
  /** Which parts of self-crossing or nested sub-paths are inside; absent = evenodd for paths. */
  fillRule?: 'nonzero' | 'evenodd';
  /** Shapes with the same group move and select together. */
  group?: string;
  hidden?: boolean;
  /** Not picked on the canvas nor moved by tools (the list and the panels still reach it). */
  locked?: boolean;
  name?: string;
}

export type SvgShape =
  | (ShapeBase & { kind: 'rect'; x: number; y: number; w: number; h: number; r?: number; rotate?: number })
  | (ShapeBase & { kind: 'ellipse'; cx: number; cy: number; rx: number; ry: number; rotate?: number })
  | (ShapeBase & { kind: 'path'; subs: SubPath[] })
  | (ShapeBase & { kind: 'text'; x: number; y: number; text: string; size: number; weight: 400 | 700 | 900; font: 'sans' | 'serif'; anchor: 'start' | 'middle' | 'end'; rotate?: number });

/** A guide: an endless line through (x, y) at `angle` degrees from the x axis (0 horizontal, 90 vertical). */
export interface Guide {
  id: string;
  x: number;
  y: number;
  angle: number;
}

export interface SvgDoc {
  width: number;
  height: number;
  shapes: SvgShape[];
  /** Intended width of the drawing on the map in mm (document properties; "1 birim = … mm"). */
  sizeMm?: number;
  /** Preview background (paper) colour kept with the drawing; absent = the theme's paper. */
  background?: string;
  /** Guide lines of the editor (snap targets); editing only, the SVG file does not keep them. */
  guides?: Guide[];
}

export const newDoc = (width = 100, height = 100): SvgDoc => ({ width, height, shapes: [] });

let seq = 0;
export const shapeId = () => `s${Date.now().toString(36)}${(seq++).toString(36)}`;

// ── To SVG ─────────────────────────────────────────────────────────────

/** A paint as an SVG attribute value (the symbol's colours as KentOS parameters). */
export const paintValue = (p: Paint): string => (p === 'fill' ? 'currentColor' : p === 'stroke' ? 'param(stroke) #000000' : p);

type Element = { tag: string; attrs: Record<string, string>; text?: string };
const elementCore = svgOp<(s: SvgShape, fill: string, stroke: string) => Element>('elementOf');

/** A shape as an element: tag and attributes (the view builds DOM from it, the file writes it). */
export function elementOf(s: SvgShape, paint: (p: Paint) => string = paintValue): Element {
  return elementCore(s, paint(s.fill), paint(s.stroke));
}

/** The drawing as an SVG file (groups become <g>, hidden shapes are left out). */
export const serializeDoc = svgOp<(doc: SvgDoc) => string>('serializeDoc');

// ── Geometry ───────────────────────────────────────────────────────────

/** A rectangle or ellipse as a path (for transforms they cannot keep). */
export const toPath = svgOp<(s: SvgShape) => Extract<SvgShape, { kind: 'path' }> | Extract<SvgShape, { kind: 'text' }>>('toPath');

export const shapeBox = svgOp<(s: SvgShape) => Box>('shapeBox');

export const shapesBox = svgOp<(shapes: readonly SvgShape[]) => Box | null>('shapesBox');

/**
 * A shape under an affine map. Translations and scalings without rotation
 * keep rectangles and ellipses; rotations keep them when uniform (the
 * rotate property grows); anything else turns them into paths. Text moves
 * and scales its size.
 */
export const transformShape = svgOp<(s: SvgShape, m: Matrix) => SvgShape>('transformShape');

/** Every shape under one affine map, in one call (a moving selection). */
export const transformShapes = svgOp<(shapes: readonly SvgShape[], m: Matrix) => SvgShape[]>('transformShapes');

export const translate = (dx: number, dy: number): Matrix => [1, 0, 0, 1, dx, dy];

/** Maps box `a` onto box `b` (scaling about their corners). */
export const boxToBox = svgOp<(a: Box, b: Box) => Matrix>('boxToBox');

/** A rotation by `deg` degrees about (cx, cy). */
export const rotation = svgOp<(deg: number, cx: number, cy: number) => Matrix>('rotation');
export { multiply };

/** A regular polygon (or star with an inner radius) as a closed path, first corner up. */
export const regularPolygon = svgOp<(cx: number, cy: number, r: number, sides: number, inner?: number) => SubPath>('regularPolygon');

/** Path data read into a path shape (imports). */
export const pathShape = (d: string, style: Omit<ShapeBase, 'id'>): SvgShape => ({ ...style, id: shapeId(), kind: 'path', subs: parsePathData(d) });
