import { op } from '../../wasm/core';
import type { ConstructionEntity, EllipseEntity, EntityGeometry } from '../entities';
import type { Vec2 } from '../geometry';
import type { EllipseGeom } from '../geom/ellipse';
import type { Edge } from '../geom/intersect';
import { entityOp } from './entityOp';

/**
 * Trim, break, extend and offset for the curves that are not paths of
 * segments and circular arcs: ellipses (cut in parameter space, pieces stay
 * elliptical arcs) and construction lines (pieces become rays or lines, as
 * AutoCAD does). Computed by the geometry core (docs/adr/0008).
 */

type Cut = { pieces: EntityGeometry[] } | { error: string };

/** Parameters where boundaries cross the curve `e` (exact for straight boundaries). */
export const ellipseCrossings = op<(e: EllipseGeom, boundaries: readonly Edge[]) => number[]>('ellipseCrossings');

export const trimEllipse = entityOp<(e: EllipseEntity, pick: Vec2, boundaries: readonly Edge[]) => Cut>('trimEllipse');

export const breakEllipse = entityOp<(e: EllipseEntity, p1: Vec2, p2: Vec2) => Cut>('breakEllipse');

/** Grows the end of an elliptical arc nearer to `pick` along its ellipse to the first boundary. */
export const extendEllipse = entityOp<(e: EllipseEntity, pick: Vec2, boundaries: readonly Edge[]) => { geometry: EntityGeometry } | { error: string }>('extendEllipse');

/**
 * Offset of an ellipse at distance d towards `through`. The true offset of
 * an ellipse is not an ellipse; like AutoCAD, the result follows it with a
 * dense polyline through exact offset points.
 */
export const offsetEllipse = entityOp<(e: EllipseEntity, d: number, through: Vec2) => { geometry: EntityGeometry } | { error: string }>('offsetEllipse');

/**
 * Trims a construction line. Line parameters (metres from p along dir)
 * where boundaries cross it are solved from p itself: the far ends of its
 * CPU edge would cost precision.
 */
export const trimConstruction = entityOp<(e: ConstructionEntity, pick: Vec2, boundaries: readonly Edge[]) => Cut>('trimConstruction');

export const breakConstruction = entityOp<(e: ConstructionEntity, p1: Vec2, p2: Vec2) => Cut>('breakConstruction');

export const offsetConstruction = entityOp<(e: ConstructionEntity, d: number, through: Vec2) => { geometry: EntityGeometry }>('offsetConstruction');
