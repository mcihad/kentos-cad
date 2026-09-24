import { CoreFaceIndex, op, writeArgs } from '../../wasm/core';
import type { Entity } from '../entities';
import type { Vec2 } from '../geometry';
import type { Edge } from './intersect';
import type { Area, Ring, Source } from './overlay';

/**
 * Area algebra on top of the overlay engine: union, intersection,
 * difference, splitting by lines, and the faces that line work encloses.
 * Areas are exact (arcs stay arcs) and may have holes. Computed by the
 * geometry core (docs/adr/0008).
 */

export type { Area, Ring, Source };

export const ringArea = op<(r: Ring) => number>('ringArea');
export const ringEdges = op<(r: Ring) => Edge[]>('ringEdges');

/** The ring running counter-clockwise (`ccw`) or clockwise. */
export const orientRing = op<(r: Ring, ccw: boolean) => Ring>('orientRing');

/** Net area: outer ring minus holes (m²). */
export const netArea = op<(a: Area) => number>('netArea');

/** Whether p lies inside the area (inside the outer ring, outside every hole). */
export const insideArea = op<(a: Area, p: Vec2) => boolean>('insideArea');

/** Overlay source of areas: outer rings counter-clockwise, holes clockwise. */
export const areaSource = op<(list: readonly Area[]) => Source>('areaSource');

/** Everything covered by any of the areas. */
export const unionAreas = op<(list: readonly Area[]) => Area[]>('unionAreas');

/** What all the areas have in common. */
export const intersectAreas = op<(list: readonly Area[]) => Area[]>('intersectAreas');

/** `from` with everything covered by `cutters` removed. */
export const subtractAreas = op<(from: readonly Area[], cutters: readonly Area[]) => Area[]>('subtractAreas');

/**
 * The area cut along lines. A line has to cross the area (or meet another
 * line) to cut; a line ending inside leaves the area whole there.
 */
export const splitArea = op<(a: Area, cut: Source) => Area[]>('splitArea');

/** Faces of line work, computed once and queried many times (hover previews). */
export interface FaceIndex {
  /**
   * The face around `p`, or null when `p` is outside every closed shape.
   * With `islands`, closed groups inside the face become holes (a
   * building inside a parcel boundary).
   */
  at(p: Vec2, islands?: boolean): Area | null;
  /** Every bounded face, each with the groups inside it as holes. */
  all(): Area[];
  /** Releases the core's copy of the faces now (else when the index is collected). */
  free(): void;
}

function wrap(core: CoreFaceIndex): FaceIndex {
  return {
    at: (p, islands = true) => core.at(p.x, p.y, islands) as Area | null,
    all: () => core.all() as Area[],
    free: () => core.free(),
  };
}

export function faceIndex(lines: readonly Source[]): FaceIndex {
  return wrap(CoreFaceIndex.of(writeArgs(lines)));
}

/** Faces of the line work of entities (their `lineSource`), built in one step in the core. */
export function entityFaceIndex(entities: readonly Entity[]): FaceIndex {
  return wrap(CoreFaceIndex.ofEntities(writeArgs(entities)));
}

/** The face of the line work around `p` (see FaceIndex.at). */
export const faceAt = op<(lines: readonly Source[], p: Vec2, islands?: boolean) => Area | null>('faceAt');

/** Every bounded face of the line work, each with the groups inside it as holes. */
export const allFaces = op<(lines: readonly Source[]) => Area[]>('allFaces');
