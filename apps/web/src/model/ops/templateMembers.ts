import { op } from '../../wasm/core';
import type { Entity, EntityGeometry } from '../entities';
import type { Vec2 } from '../geometry';
import type { MemberSide } from '../objectTemplate';
import { entityOp } from './entityOp';

/** An offset member's parallels, or why there are none. */
export type MemberOffsets = { geometries: EntityGeometry[] } | { error: string };

/**
 * What a group template's offset member makes from the drawn shape (docs/adr/0176 §5): its parallels at `distance`,
 * one on `side`, two for “both” (the left or inside one first); the side is left or right of the drawing's direction
 * on a line or polyline, inside or outside on an area. Computed by the geometry core (`ops::template_members`).
 */
export const templateMemberOffsets = entityOp<(e: Entity, distance: number, side: MemberSide) => MemberOffsets>('templateMemberOffsets');

/** Where an Ağırlık merkezine member's point or text goes: the label's place (an area's centroid, a line's middle); null for none. */
export const templateMemberCentroid = op<(e: Entity) => Vec2 | null>('templateMemberCentroid');
