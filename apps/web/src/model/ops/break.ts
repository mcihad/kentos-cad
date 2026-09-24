import type { Entity, EntityGeometry } from '../entities';
import type { Vec2 } from '../geometry';
import { entityOp } from './entityOp';

export type BreakResult = { pieces: EntityGeometry[] } | { error: string };

/**
 * Removes the part between two picked points ("Kır"). With p2 = p1 the
 * object is split at that point and nothing is removed. On closed shapes
 * the removed part runs counter-clockwise from p1 to p2, as on circles;
 * breaking a polygon at a single point opens it there. Computed by the
 * geometry core (docs/adr/0008).
 */
export const breakEntity = entityOp<(e: Entity, p1: Vec2, p2: Vec2) => BreakResult>('breakEntity');
