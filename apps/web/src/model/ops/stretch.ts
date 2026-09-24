import type { Entity, EntityGeometry } from '../entities';
import type { Bounds } from '../geometry';
import { entityOp } from './entityOp';

/**
 * Stretch ("Esnet"): vertices inside the crossing window move by (dx, dy),
 * the rest stay; an entity entirely inside simply moves. Returns null when
 * nothing of the entity lies in the window. Computed by the geometry core
 * (docs/adr/0008).
 */
export const stretchEntity = entityOp<(e: Entity, r: Bounds, dx: number, dy: number) => EntityGeometry | null>('stretchEntity');
