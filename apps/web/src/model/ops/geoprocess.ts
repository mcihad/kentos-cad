import { op } from '../../wasm/core';
import type { EntityGeometry } from '../entities';
import type { Vec2 } from '../geometry';
import type { DatumChoice, System } from '../geom/crsTransform';

/**
 * Geometri işlemleri (docs/adr/0201): the processing tools' buffers, clipping, overlays, dissolving, validity and
 * repair, simplifying and reprojecting, computed by the geometry core (`ops::geoprocess`; the desktop's tools call the
 * same functions). Objects go in whole, as the drawing holds them; what comes back is each result's geometry (an area
 * with its parts and holes, a polyline with its parts, a point with its points) for the tool to put on its output
 * layer with attributes. The independent reference is scripts/fixtures/geoprocess_cases.py.
 */

/** A geometry the core writes: an area, a polyline or a point. */
export type GeoShape = Extract<EntityGeometry, { kind: 'polygon' | 'polyline' | 'point' }>;

/**
 * One piece Tampon writes: the object (its place in the list; none: every object's, Birleştir), its ring (1 the
 * first), its distance (k · d as a decimal text; none when the joined distances differ), its shape. The core writes
 * no field it has no value for.
 */
export interface BufferPiece {
  readonly source?: number;
  readonly ring: number;
  readonly distance?: string;
  readonly shape: GeoShape;
}

/** Tampon's answer: its pieces; the objects whose distance is not a number, those with a minus distance while there are rings, and those whose buffer is nothing. */
export interface Buffered {
  readonly pieces: readonly BufferPiece[];
  readonly unread: readonly number[];
  readonly inward: readonly number[];
  readonly empty: readonly number[];
}

/** Tampon: each object at its distance (a decimal text, null: none), `rings` rings, joined when `dissolve`. */
export const geoBuffer = op<(entities: readonly EntityGeometry[], distances: readonly (string | null)[], side: 'both' | 'left' | 'right', rings: number, dissolve: boolean) => Buffered>('geoBuffer');

/** Kes: each object's parts inside or on the cutting areas joined; null where nothing is left. */
export const geoClip = op<(entities: readonly EntityGeometry[], cut: readonly EntityGeometry[]) => (GeoShape | null)[]>('geoClip');

/** Which overlay. */
export type OverlayMode = 'intersection' | 'difference' | 'symDifference' | 'union';

/** An overlay's piece: its first-side and second-side objects (places in their lists; none: not of that side), its share of the first-side object, its shape. */
export interface OverlayPiece {
  readonly a?: number;
  readonly b?: number;
  readonly share?: number;
  readonly shape: GeoShape;
}

/** Kesişim, Fark, Simetrik fark, Birleşim: the pieces in their order (§5). */
export const geoOverlay = op<(a: readonly EntityGeometry[], b: readonly EntityGeometry[], mode: OverlayMode) => OverlayPiece[]>('geoOverlay');

/** Birleştir: each group's objects (groups numbered by their first object) as one, or each joined part on its own. */
export const geoDissolve = op<(entities: readonly EntityGeometry[], groups: readonly number[], multi: boolean) => { group: number; shape: GeoShape }[]>('geoDissolve');

/** A problem's kind (§6). */
export type ProblemKind = 'repeated' | 'zeroArea' | 'ringCrossing' | 'pathCrossing' | 'holeOutside' | 'holesOverlap';

/** Geçerliliği denetle: every object's problems, its place in the list, the kind, the report's text and the first place it shows. */
export const geoValidity = op<(entities: readonly EntityGeometry[]) => { index: number; problem: ProblemKind; text: string; at: Vec2 }[]>('geoValidity');

/** Onar's answer for an object: its shape (none: nothing is left; an object without problems as it is), its parts, holes, vertices and area (none for a path) before and after, its problems and their texts. */
export interface Repaired {
  readonly shape?: EntityGeometry;
  readonly parts: readonly [number, number];
  readonly holes: readonly [number, number];
  readonly vertices: readonly [number, number];
  readonly area?: readonly [number, number];
  readonly problems: readonly ProblemKind[];
  readonly texts: readonly string[];
}

export const geoRepair = op<(entities: readonly EntityGeometry[]) => Repaired[]>('geoRepair');

/** Sadeleştir's answer for an object: its shape (as it was when no vertex went), whether it changed, its vertices and area (null for a path) before and after, the largest deviation (m). */
export interface Simplified {
  readonly shape: GeoShape;
  readonly changed: boolean;
  readonly vertices: readonly [number, number];
  /** None for a path. */
  readonly area?: readonly [number, number];
  readonly deviation: number;
}

export const geoSimplify = op<(entities: readonly EntityGeometry[], tolerance: number) => Simplified[]>('geoSimplify');

/** Why a vertex has no value in the project's system. */
export type Unreached = 'outside' | 'noLink' | 'noGrid' | 'outsideGrid';

/** Koordinat sistemine dönüştür's answer for an object: its shape, or why not; the arcs that became chords. */
export interface Reprojected {
  readonly shape?: GeoShape;
  readonly error?: Unreached;
  readonly chorded: number;
}

export const geoReproject = op<(entities: readonly EntityGeometry[], from: System, to: System, choices: readonly DatumChoice[] | null) => Reprojected[]>('geoReproject');

/** An object as the geometry tools measure it (the shared cases' players). */
export interface GeoMeasure {
  readonly parts: number;
  readonly holes: number;
  readonly area?: number;
  readonly length?: number;
  readonly points?: readonly Vec2[];
}

export const geoMeasure = op<(entities: readonly EntityGeometry[]) => GeoMeasure[]>('geoMeasure');

/** Alan oranıyla paylaştır (§5): each value times the share, rounded half to even at two more digits; null for a value not read as a number. */
export const apportion = op<(values: readonly (string | null)[], share: number) => (string | null)[]>('apportion');
