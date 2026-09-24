/**
 * The style engine's data model (docs/STYLE.md): symbols made of symbol
 * layers, layer renderers that pick a symbol for each object, and library
 * items. Everything is plain JSON so styles can be saved in a project,
 * kept in a user library, exported and shared. Nothing here draws.
 */

// ── Values ─────────────────────────────────────────────────────────────

/**
 * Size unit: "mm" is paper millimetres at the project's plot scale (1 mm is
 * scale/1000 m, so it grows with zoom like ink on the sheet), "px" screen
 * pixels (constant on screen), "m" map units.
 */
export type SizeUnit = 'mm' | 'px' | 'm';

/** A value that may come from an expression per object (docs/STYLE.md §3.5). */
export type DataDefined<T> = T | { readonly expr: string; readonly fallback?: T };

/**
 * "#RRGGBB", "#RRGGBBAA" or a theme token: "ink" (CAD colour 7: black on
 * paper, white on a dark screen), "paper" (the sheet: white on paper, the
 * canvas colour on a dark screen), "fg", "fg-dim".
 */
export type Color = string;

// ── Symbol layers ──────────────────────────────────────────────────────

interface LayerBase {
  /** Stable within its symbol (editor selection, undo). */
  readonly id: string;
  /** Hidden layers stay in the symbol but are not drawn; may depend on the object. */
  readonly enabled?: DataDefined<boolean>;
  /** 0–1, multiplies the layer's colours. */
  readonly opacity?: number;
  /** Unit of this layer's sizes and distances (default "mm"). */
  readonly unit?: SizeUnit;
}

export type ShapeName =
  | 'circle'
  | 'ring'
  | 'square'
  | 'rectangle'
  | 'diamond'
  | 'triangle'
  | 'pentagon'
  | 'hexagon'
  | 'octagon'
  | 'star'
  | 'cross'
  | 'x'
  | 'line'
  | 'arrow'
  | 'arrowhead'
  | 'chevron'
  | 'semicircle'
  | 'quartercircle'
  | 'gear'
  | 'arc';

export type Anchor = 'center' | 'top' | 'bottom' | 'left' | 'right' | 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right';

interface MarkerBase extends LayerBase {
  /** Width of the marker's box (height for text). */
  readonly size: DataDefined<number>;
  /** Degrees, counter-clockwise; added to the line's direction when the marker follows a line. */
  readonly rotation?: DataDefined<number>;
  /** Shift of the marker from its point, in the layer unit, before rotation (x right, y up). */
  readonly offset?: readonly [number, number];
  readonly anchor?: Anchor;
}

export interface ShapeMarker extends MarkerBase {
  readonly type: 'shape';
  readonly shape: ShapeName;
  /** Height for "rectangle" (size is the width). */
  readonly height?: number;
  readonly fill?: DataDefined<Color> | null;
  readonly stroke?: DataDefined<Color> | null;
  readonly strokeWidth?: number;
  /** Closed shapes: a round hole, as a share of the radius (0–0.95; a gear's axle, a washer). */
  readonly hole?: number;
  /** "gear": number of square teeth (default 12) and their depth as a share of the radius (default 0.2). */
  readonly teeth?: number;
  readonly teethDepth?: number;
  /** "arc": the opening in degrees, centred on the top (default 180). */
  readonly sweep?: number;
}

export interface SvgMarker extends MarkerBase {
  readonly type: 'svg';
  /** Library asset id. */
  readonly asset: string;
  /** Colours for param(fill) / param(stroke) in the drawing. */
  readonly fill?: DataDefined<Color>;
  readonly stroke?: DataDefined<Color>;
}

export interface TextMarker extends MarkerBase {
  readonly type: 'text';
  /** Fixed text or an expression ("Parsel", "'E=' || Emsal"). */
  readonly text: DataDefined<string>;
  /** ui: the interface face; sans: Arial-like (the regulation's lettering); narrow: Arial Narrow-like; serif: Times-like; mono. */
  readonly font?: 'ui' | 'sans' | 'narrow' | 'serif' | 'mono';
  /** 900 with the sans face is Arial Black. */
  readonly weight?: 400 | 500 | 600 | 700 | 800 | 900;
  readonly italic?: boolean;
  readonly color?: DataDefined<Color>;
  /** Outline around the letters for legibility over fills. */
  readonly halo?: { readonly color: Color; readonly width: number } | null;
}

export interface RasterMarker extends MarkerBase {
  readonly type: 'raster';
  readonly asset: string;
}

export type MarkerLayer = ShapeMarker | SvgMarker | TextMarker | RasterMarker;

export interface SimpleLine extends LayerBase {
  readonly type: 'simpleLine';
  readonly color: DataDefined<Color>;
  readonly width: DataDefined<number>;
  /** On/off lengths in the layer unit; empty or absent = continuous. */
  readonly dash?: readonly number[] | null;
  readonly dashOffset?: number;
  readonly cap?: 'butt' | 'round' | 'square';
  readonly join?: 'miter' | 'round' | 'bevel';
  /**
   * Parallel shift. On lines: positive to the left of the drawing direction.
   * On area edges: positive into the area. Data-defined for road edges set
   * from a width field ([Genişlik] / 2 in metres, or with $ölçek in mm).
   */
  readonly offset?: DataDefined<number>;
  /** Area edges only: which rings. */
  readonly rings?: 'all' | 'exterior' | 'interior';
  /** Draw the line as waves (sulak alan, enerji nakil hattı …) instead of straight. */
  readonly wave?: LineWave;
  /** Soft edges over this width (a shadow); 0 or absent = crisp. */
  readonly blur?: number;
  /** Fixed shift on the page, whatever the line's direction: [right, up] (a drop shadow). */
  readonly shift?: readonly [number, number];
}

/**
 * A wavy line, in the layer unit: one wave of `length` along the line and
 * `amplitude` to each side, repeated every `spacing` (default: `length`,
 * back to back). Between waves the line runs straight when `connect`,
 * otherwise it is left out (a dashed wave).
 */
export interface LineWave {
  readonly shape: 'sine' | 'zigzag' | 'square';
  readonly length: number;
  readonly amplitude: number;
  readonly spacing?: number;
  readonly connect?: boolean;
  /**
   * Where the first wave starts, from the path's start. When absent the
   * waves are centred on the path; when set, marker lines with the same
   * interval and offset stay in step with the waves (sulak alan dots).
   */
  readonly offsetAlong?: number;
}

export type MarkerPlacement = 'interval' | 'vertex' | 'innerVertex' | 'first' | 'last' | 'center' | 'segmentCenter';

export interface MarkerLine extends LayerBase {
  readonly type: 'markerLine';
  readonly marker: MarkerSymbol;
  readonly placement: MarkerPlacement;
  /** Distance between markers ("interval"), in the layer unit. */
  readonly interval?: number;
  /** Distance of the first marker from the start ("interval"). */
  readonly offsetAlong?: number;
  /** Perpendicular shift (same sign rule as SimpleLine.offset). */
  readonly offset?: DataDefined<number>;
  /** Turn markers with the line's direction; text is kept upright (turned half a turn where it would read backwards). */
  readonly rotate?: boolean;
  readonly rings?: 'all' | 'exterior' | 'interior';
  /** Several markers at each place, `spacing` apart along the line (dots in a dash gap). */
  readonly group?: { readonly count: number; readonly spacing: number };
}

export type LineLayer = SimpleLine | MarkerLine;

export interface SimpleFill extends LayerBase {
  readonly type: 'simpleFill';
  readonly color: DataDefined<Color>;
}

export interface HatchFill extends LayerBase {
  readonly type: 'hatchFill';
  /** Degrees counter-clockwise from east. */
  readonly angle: number;
  readonly spacing: number;
  readonly width: number;
  readonly color: DataDefined<Color>;
  /** Shift of the lines across their direction. */
  readonly offset?: number;
  /** Dashed hatch lines (on/off lengths). */
  readonly dash?: readonly number[] | null;
  readonly dashOffset?: number;
}

export interface PatternFill extends LayerBase {
  readonly type: 'patternFill';
  readonly marker: MarkerSymbol;
  readonly spacingX: number;
  readonly spacingY: number;
  /** Every other row shifted by half a spacing. */
  readonly stagger?: boolean;
  readonly angle?: number;
  readonly offset?: readonly [number, number];
  /**
   * Scatter (kumsal, serbest noktalama): 0–1, how far each shape may move
   * at random inside its cell. Shape markers only.
   */
  readonly jitter?: number;
  /** 0–1: share of the cells that get a shape, at random (default 1). Shape markers only. */
  readonly coverage?: number;
  /** Varies the random layout between symbols. */
  readonly seed?: number;
}

export interface ImageFill extends LayerBase {
  readonly type: 'imageFill';
  readonly asset: string;
  /** Tile width in the layer unit; the height follows the image. */
  readonly tileSize: number;
  readonly angle?: number;
}

export interface CentroidMarker extends LayerBase {
  readonly type: 'centroidMarker';
  readonly marker: MarkerSymbol;
  /** Inside point (always in the area) or the centre of mass. */
  readonly position?: 'pointOnSurface' | 'centroid';
}

export type FillLayer = SimpleFill | HatchFill | PatternFill | ImageFill | CentroidMarker | SimpleLine | MarkerLine;

// ── Symbols ────────────────────────────────────────────────────────────

export interface MarkerSymbol {
  readonly type: 'marker';
  readonly layers: readonly MarkerLayer[];
}

export interface LineSymbol {
  readonly type: 'line';
  readonly layers: readonly LineLayer[];
}

export interface FillSymbol {
  readonly type: 'fill';
  readonly layers: readonly FillLayer[];
}

export type Symbol = MarkerSymbol | LineSymbol | FillSymbol;
export type SymbolType = Symbol['type'];
export type SymbolLayer = MarkerLayer | LineLayer | FillLayer;

/** A symbol from the library (by id) or written in place. */
export type SymbolRef = { readonly ref: string } | Symbol;

/** What a CAD layer (mixed geometry) draws for each geometry class. */
export interface SymbolSet {
  readonly marker?: SymbolRef;
  readonly line?: SymbolRef;
  readonly fill?: SymbolRef;
}

// ── Renderers ──────────────────────────────────────────────────────────

export interface Rule {
  readonly id: string;
  readonly label: string;
  /** Expression; absent = every object. */
  readonly filter?: string;
  /** Applies when no sibling rule matched. */
  readonly isElse?: boolean;
  /** Scale range as 1:N denominators; drawn when minScale ≤ N ≤ maxScale. */
  readonly minScale?: number;
  readonly maxScale?: number;
  readonly symbols?: SymbolSet;
  readonly children?: readonly Rule[];
  readonly enabled?: boolean;
}

export type LayerRenderer =
  | { readonly type: 'single'; readonly symbols: SymbolSet }
  | {
      readonly type: 'categorized';
      readonly expr: string;
      readonly categories: readonly { readonly value: string; readonly label: string; readonly symbols: SymbolSet; readonly enabled?: boolean }[];
      readonly other?: SymbolSet;
    }
  | {
      readonly type: 'graduated';
      readonly expr: string;
      /** min ≤ value < max; the last class includes its max. */
      readonly classes: readonly { readonly min: number; readonly max: number; readonly label: string; readonly symbols: SymbolSet }[];
    }
  | { readonly type: 'rules'; readonly rules: readonly Rule[] };

// ── Library ────────────────────────────────────────────────────────────

/** Where a library item lives: shipped (read-only), the user's own, or the project's. */
export type LibrarySource = 'system' | 'user' | 'project';

export interface LibrarySymbol {
  readonly kind: 'symbol';
  readonly id: string;
  readonly name: string;
  /** Category path, root first: ["MPYY", "Uygulama İmar Planı", "Sınırlar"]. */
  readonly path: readonly string[];
  readonly symbol: Symbol;
  readonly description?: string;
  readonly tags?: readonly string[];
  /** Where it comes from, e.g. "MPYY EK-1d, s. 1". */
  readonly reference?: string;
}

/** An SVG drawing or raster image used by markers and fills. */
export interface LibraryAsset {
  readonly kind: 'asset';
  readonly id: string;
  readonly name: string;
  readonly path: readonly string[];
  readonly format: 'svg' | 'png' | 'jpeg';
  /** SVG text, or a data: URL for rasters. */
  readonly data: string;
  /** Natural size in px (rasters) or the SVG's viewBox size. */
  readonly width: number;
  readonly height: number;
  readonly tags?: readonly string[];
}

export type LibraryItem = LibrarySymbol | LibraryAsset;

/** An item as the library hands it out: with its source. */
export type Sourced<T extends LibraryItem = LibraryItem> = T & { readonly source: LibrarySource };

/** Explicit categories: empty ones and the order of siblings. */
export interface LibraryCategory {
  readonly path: readonly string[];
  readonly order?: number;
  readonly description?: string;
}

/** A project's own library part, saved in the project file. */
export interface ProjectStyles {
  readonly items: readonly LibraryItem[];
  readonly categories: readonly LibraryCategory[];
}
