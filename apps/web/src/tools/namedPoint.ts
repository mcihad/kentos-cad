import type { AppContext } from '../app/context';
import type { Entity, PointEntity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { pointName } from './coordinateInput';

/**
 * `#ad` typed where a point is asked (docs/adr/0152 §4): the place of the point named so, given to the running tool
 * as if clicked (its `acceptPoint`, as the point calculator gives one). The name is matched with the points' labels,
 * the spaces round them dropped; hidden layers count too: a name is an identity. True when the text was a name (the
 * point taken, or why not said); false when it was none and is the tool's. The desktop's is `Session::input`.
 */
export function namedPoint(ctx: AppContext, text: string): boolean {
  const name = pointName(text);
  if (name === null) return false;
  const at = namedPointAt(ctx, name);
  if (typeof at === 'string') ctx.log.warn(at);
  else if (!ctx.tools.active.acceptPoint?.(at)) ctx.log.warn(`#${name}: bu adımda nokta istenmiyor.`);
  return true;
}

/**
 * The place of the one point named `name`, or why none is taken in `#ad`'s words (Nokta adından takes it too,
 * docs/adr/0188 §3). The desktop's is `session::named_point_at`.
 */
export function namedPointAt(ctx: AppContext, name: string): Vec2 | string {
  const found = [...ctx.doc.all()].filter((e: Entity): e is PointEntity => e.kind === 'point' && e.label?.trim() === name);
  if (!found.length) return `#${name}: bu adda nokta yok.`;
  if (found.length > 1) return `#${name}: bu adda ${found.length} nokta var; koordinatı yazın.`;
  return found[0].p;
}
