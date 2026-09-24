import type { Vec2 } from '../model/geometry';
import type { Anchor, Color, ShapeName } from '../model/style';

/**
 * What compiling a symbol on a geometry produces: drawing primitives in
 * world coordinates, independent of any backend. The GPU scene builder
 * packs them into batches; previews and legends draw them with Canvas2D.
 * Every size is already converted: "world" means metres (from mm or m),
 * "px" screen pixels drawn by the shader.
 */

export type PrimUnit = 'world' | 'px';

/** Paint shared by one batch: equal styles merge (see styleKey). */
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

/** Where compile sends its output. Equal styles are merged by the receiver. */
export interface PrimitiveSink {
  stroke(style: StrokeStyle, path: readonly Vec2[], closed: boolean): void;
  /** One area: outer ring first, holes after (any orientation). */
  fill(paint: FillPaint, rings: readonly (readonly Vec2[])[]): void;
  /** `angle`: radians, the direction the marker faces (0 = east). */
  marker(style: MarkerStyle, at: Vec2, angle: number): void;
}

/** A stable key for merging equal styles into one batch. */
export const styleKey = (s: StrokeStyle | FillPaint | MarkerStyle): string => JSON.stringify(s);

/** Collects primitives in order (tests, previews, legends). */
export class PrimitiveList implements PrimitiveSink {
  readonly strokes: { style: StrokeStyle; path: readonly Vec2[]; closed: boolean }[] = [];
  readonly fills: { paint: FillPaint; rings: readonly (readonly Vec2[])[] }[] = [];
  readonly markers: { style: MarkerStyle; at: Vec2; angle: number }[] = [];

  stroke(style: StrokeStyle, path: readonly Vec2[], closed: boolean): void {
    this.strokes.push({ style, path, closed });
  }

  fill(paint: FillPaint, rings: readonly (readonly Vec2[])[]): void {
    this.fills.push({ paint, rings });
  }

  marker(style: MarkerStyle, at: Vec2, angle: number): void {
    this.markers.push({ style, at, angle });
  }
}
