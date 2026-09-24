import { op } from '../../wasm/core';
import type { Entity, EntityGeometry } from '../entities';
import type { Area, Source } from '../geom/region';
import { entityOp } from './entityOp';

/**
 * Entities ↔ areas for the area tools (Alan işlemleri), computed by the
 * geometry core (docs/adr/0008). Polygons and circles convert exactly (a
 * circle is two half-circle bulges); a full ellipse or a closed spline
 * becomes a fine polygon, within 1 mm of the curve, since the result is an
 * area of straight and circular edges.
 */

/** The area an entity encloses, or null for open or non-area entities. Hatches are fills, not areas. */
export const areaOfEntity = op<(e: EntityGeometry) => Area | null>('areaOfEntity');

/** Polygon geometry of an area (holes only when there are some). */
export const polygonOfArea = entityOp<(a: Area) => EntityGeometry>('polygonOfArea');

/** A polygon's rings as closed polylines (first point repeated at the end): outer first, then holes. */
export const polylinesOfPolygon = entityOp<(e: Extract<EntityGeometry, { kind: 'polyline' | 'polygon' }>) => EntityGeometry[]>('polylinesOfPolygon');

/** Line work as an overlay source (cut lines, boundaries of "click inside"). */
export const lineSource = op<(entities: readonly Entity[]) => Source>('lineSource');
