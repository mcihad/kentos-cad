import { op } from '../../wasm/core';
import type { Entity, HatchPattern, PatternLine } from '../entities';
import type { Vec2 } from '../geometry';
import type { Area } from '../geom/region';

/**
 * Hatch patterns and regions (docs/adr/0186): the core's `geom::hatch_pattern` and `ops::hatch_region`, one rule for
 * both platforms. The library's patterns, a pattern's families as the style engine's hatch paints, its lines and dots
 * cut to a region (previews), and a hatch's region: the closed object's area around its seed less its islands and the
 * boxes of the texts and inserts left open. The independent reference is scripts/fixtures/hatch_pattern_cases.py
 * (fixtures/hatch/v1/cases.json).
 */

/** A pattern of the library: its name, group, what it is drawn for and its families in paper millimetres. */
export interface LibraryPattern {
  readonly name: string;
  readonly group: 'ansi' | 'iso' | 'general';
  readonly description: string;
  readonly lines: readonly PatternLine[];
}

/** A family as a hatch paint: lines at `angle` (degrees) `spacing` apart, its phases, dashes and stagger (metres). */
export interface FamilyPaint {
  readonly angle: number;
  readonly spacing: number;
  readonly offset: number;
  readonly stagger: number;
  readonly dash?: readonly number[];
  readonly dashOffset: number;
}

/** A pattern's lines and dots cut to a region; `capped`: more than the budget, none given. */
export interface HatchPieces {
  readonly segments: readonly (readonly [Vec2, Vec2])[];
  readonly dots: readonly Vec2[];
  readonly capped: boolean;
}

/** A hatch's region: its ring and holes, arcs as chords; which islands and cutouts given reach into it, by place. */
export interface HatchRegion {
  readonly ring: readonly Vec2[];
  readonly holes: readonly (readonly Vec2[])[];
  readonly islands: readonly number[];
  readonly cutouts: readonly number[];
}

/** A base area cut by what reaches into it: the parts left, and which islands and cutouts reached in, by place. */
export interface HatchCut {
  readonly parts: readonly Area[];
  readonly islands: readonly number[];
  readonly cutouts: readonly number[];
}

/** One of Tarama's Desen choices (`tools::hatch::choices`): its name, menu label, typed form, icon and pattern type. */
export interface HatchChoice {
  readonly name: string;
  readonly label: string;
  readonly typed: string;
  readonly icon: string;
  /** Its wide sample in Desen's menus (docs/adr/0186 §11). */
  readonly preview: string;
  readonly kind: 'lines' | 'cross' | 'solid' | 'pattern' | 'gradient';
}

/** Tarama's pattern options and the drawing's plot scale (1:N). */
export interface HatchToolOptions {
  readonly choice: number;
  readonly scale: number;
  readonly angle: number;
  readonly color2: string;
  readonly inverted: boolean;
  readonly plotScale: number;
}

/** The block definitions a cutout insert needs (the contract's list), or null. */
type Blocks = readonly unknown[] | null;

export const hatchLibrary = op<() => LibraryPattern[]>('hatchPatterns');
export const hatchPaints = op<(pattern: HatchPattern) => FamilyPaint[]>('hatchPaints');
/** Whether a pattern's lines over a ring are too many to draw or explode (a family over the core's 20 000). */
export const hatchTooDense = op<(ring: readonly Vec2[], pattern: HatchPattern) => boolean>('hatchTooDense');
export const hatchPatternPieces = op<(ring: readonly Vec2[], holes: readonly (readonly Vec2[])[] | null, pattern: HatchPattern, budget: number) => HatchPieces>('hatchPatternPieces');
/** A hatch's region; the cutouts' boxes from `hatchCutout`. */
export const hatchRegion = op<(outer: Entity, islands: readonly Entity[], boxes: readonly (readonly Vec2[])[], seed: Vec2) => HatchRegion | null>('hatchRegion');
export const hatchCutout = op<(e: Entity, blocks: Blocks, font: string | null) => Vec2[] | null>('hatchCutout');
/** The tools' two halves of `hatchRegion`: the cut once, the part under the cursor each move. */
export const hatchCut = op<(base: Area, islands: readonly (readonly Area[])[], cutouts: readonly (readonly Vec2[])[]) => HatchCut>('hatchCut');
export const hatchPick = op<(parts: readonly Area[], seed: Vec2) => number | null>('hatchPick');
/** A point inside an area (Çoklu tara's seed). */
export const hatchSeed = op<(area: Area) => Vec2 | null>('hatchSeed');
export const hatchChoices = op<() => HatchChoice[]>('hatchChoices');
export const hatchChoiceNamed = op<(text: string) => number | null>('hatchChoiceNamed');
export const hatchColour = op<(text: string) => string | null>('hatchColour');
export const hatchToolPattern = op<(options: HatchToolOptions) => HatchPattern>('hatchToolPattern');
/** İkinci renk's names (`tools::hatch::COLOURS`), as the menus offer them. */
export const HATCH_COLOURS: readonly (readonly [string, string])[] = [
  ['Beyaz', '#FFFFFF'],
  ['Siyah', '#000000'],
  ['Kırmızı', '#E5484D'],
  ['Sarı', '#F2C94C'],
  ['Yeşil', '#5FBF77'],
  ['Camgöbeği', '#4CC3D9'],
  ['Mavi', '#4F8EF7'],
  ['Eflatun', '#C86DD7'],
  ['Gri', '#8C9AAA'],
];

let library: readonly LibraryPattern[] | null = null;

/** The library, read from the core once. */
export function hatchPatterns(): readonly LibraryPattern[] {
  return (library ??= hatchLibrary());
}

/** The library's pattern of this name, its case aside. */
export function libraryPattern(name: string): LibraryPattern | undefined {
  const n = name.toUpperCase();
  return hatchPatterns().find((p) => p.name === n);
}
