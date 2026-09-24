import type { Entity, EntityGeometry } from '../entities';
import { layoutDimension, type DimensionLayout } from '../geom/dimension';
import { entityOp } from './entityOp';

export type ExplodeResult = { pieces: EntityGeometry[] } | { error: string };

const explode = entityOp<(e: Entity, valueText: string) => ExplodeResult>('explodeEntity');

/**
 * Breaks a compound entity into simple ones (computed by the geometry core,
 * docs/adr/0008):
 *   polyline / polygon → lines and arcs (one per segment, holes included)
 *   spline → polyline through its tessellated curve (so it can be trimmed)
 *   dimension → lines and the value text · patterned hatch → lines
 * `valueText` renders a dimension's measured value (project units); the
 * core takes the text, so it is asked only for a dimension without its own.
 */
export function explodeEntity(e: Entity, valueText: (l: DimensionLayout) => string): ExplodeResult {
  const l = e.kind === 'dimension' && !e.text ? layoutDimension(e) : null;
  return explode(e, l ? valueText(l) : '');
}
