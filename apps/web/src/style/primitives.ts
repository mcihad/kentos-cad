import type { Vec2 } from '../model/geometry';
import type { Anchor, Color, ShapeName } from '../model/style';

/**
 * What compiling a symbol on a geometry produces, as the style core writes
 * it (crates/shared/style-core/src/style/prim.rs): drawing primitives in
 * world coordinates, independent of any backend. A layer's come packed in
 * GPU batches (render/styledBatches.ts); previews and legends get them one
 * by one (`compileSymbol`) and draw them with Canvas2D. Every size is
 * already converted: "world" means metres (from mm or m), "px" screen
 * pixels drawn by the shader.
 */

export type PrimUnit = 'world' | 'px';

/** A CSS pixel on paper (96 dpi), where a length in px must become geometry. */
export const MM_PER_PX = 25.4 / 96;

/** Paint shared by one batch: equal styles merge. */
export interface StrokeStyle {
  readonly color: Color;
  readonly opacity: number;
  readonly width: number;
  readonly unit: PrimUnit;
  /** On/off lengths in `unit`, or null. */
  readonly dash: readonly number[] | null;
  readonly dashOffset: number;
  readonly cap: 'butt' | 'round' | 'square';
  readonly join: 'miter' | 'round' | 'bevel';
  /** Soft edge width in `unit` (0 = crisp). */
  readonly blur: number;
  /** Symbol layer order: lower draws first (symbol levels). */
  readonly level: number;
}

export type FillPaint =
  | { readonly kind: 'solid'; readonly color: Color; readonly opacity: number; readonly level: number }
  | {
      readonly kind: 'hatch';
      readonly color: Color;
      readonly opacity: number;
      /** Radians, counter-clockwise from east. */
      readonly angle: number;
      readonly spacing: number;
      readonly width: number;
      readonly offset: number;
      readonly dash: readonly number[] | null;
      readonly dashOffset: number;
      readonly unit: PrimUnit;
      readonly level: number;
    }
  | {
      /**
       * One shape on a grid, drawn by the shader (sharp at every zoom, no
       * texture): pattern fills of shape markers, one paint per marker layer.
       */
      readonly kind: 'pattern';
      /** The shape, its sizes in the paint's `unit`. */
      readonly mark: ShapeMarkStyle;
      /** Cell size in `unit` (width, height). */
      readonly size: readonly [number, number];
      readonly stagger: boolean;
      readonly angle: number;
      readonly offset: readonly [number, number];
      readonly jitter: number;
      readonly coverage: number;
      readonly seed: number;
      readonly opacity: number;
      readonly unit: PrimUnit;
      readonly level: number;
    }
  | {
      /** A tile repeated over the area: a marker pattern or an image asset. */
      readonly kind: 'tile';
      readonly tile: TileSource;
      /** Tile size in `unit` (width, height). */
      readonly size: readonly [number, number];
      readonly angle: number;
      readonly offset: readonly [number, number];
      readonly opacity: number;
      readonly unit: PrimUnit;
      readonly level: number;
    };

export type TileSource =
  | { readonly kind: 'asset'; readonly asset: string }
  /** Markers at the tile's grid points (and half-shifted when staggered), drawn into a tile. */
  | { readonly kind: 'markers'; readonly markers: readonly MarkerStyle[]; readonly stagger: boolean };

export type MarkerStyle =
  | {
      readonly kind: 'shape';
      readonly shape: ShapeName;
      /** Width, and height for rectangles, in `unit`. */
      readonly size: number;
      readonly height: number;
      readonly fill: Color | null;
      readonly stroke: Color | null;
      readonly strokeWidth: number;
      /** Hole (share of the radius), teeth, arc opening (radians), tooth depth (share of the radius). */
      readonly params: ShapeParams;
      readonly common: MarkerCommon;
    }
  | { readonly kind: 'svg'; readonly asset: string; readonly size: number; readonly fill: Color | null; readonly stroke: Color | null; readonly common: MarkerCommon }
  | { readonly kind: 'raster'; readonly asset: string; readonly size: number; readonly common: MarkerCommon }
  | {
      readonly kind: 'text';
      readonly text: string;
      /** Letter height in `unit`. */
      readonly size: number;
      readonly font: 'ui' | 'sans' | 'narrow' | 'serif' | 'mono';
      readonly weight: number;
      readonly italic: boolean;
      readonly color: Color;
      readonly halo: { readonly color: Color; readonly width: number } | null;
      readonly common: MarkerCommon;
    };

export type ShapeMarkStyle = Extract<MarkerStyle, { kind: 'shape' }>;

/** Shape parameters as the shaders read them: hole, teeth, opening (radians), tooth depth. */
export type ShapeParams = readonly [number, number, number, number];
export const NO_SHAPE_PARAMS: ShapeParams = [0, 12, Math.PI, 0.2];

export interface MarkerCommon {
  readonly unit: PrimUnit;
  readonly opacity: number;
  /** Shift in `unit` before rotation (x right, y up). */
  readonly offset: readonly [number, number];
  readonly anchor: Anchor;
  /** Extra rotation of this marker, radians (added to the placement angle). */
  readonly rotation: number;
  readonly level: number;
}

/** A symbol's primitives on one object, in the order compiled (`compileSymbol`). */
export interface Primitives {
  readonly strokes: readonly { readonly style: StrokeStyle; readonly path: readonly Vec2[]; readonly closed: boolean }[];
  readonly fills: readonly { readonly paint: FillPaint; readonly rings: readonly (readonly Vec2[])[] }[];
  readonly markers: readonly { readonly style: MarkerStyle; readonly at: Vec2; readonly angle: number }[];
}
