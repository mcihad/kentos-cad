import type { AppContext } from '../app/context';
import type { Entity, PointEntity } from '../model/entities';
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
  const found = [...ctx.doc.all()].filter((e: Entity): e is PointEntity => e.kind === 'point' && e.label?.trim() === name);
  if (!found.length) ctx.log.warn(`#${name}: bu adda nokta yok.`);
  else if (found.length > 1) ctx.log.warn(`#${name}: bu adda ${found.length} nokta var; koordinatı yazın.`);
  else if (!ctx.tools.active.acceptPoint?.(found[0].p)) ctx.log.warn(`#${name}: bu adımda nokta istenmiyor.`);
  return true;
}
