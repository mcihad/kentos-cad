import type { AppContext } from '../app/context';
import type { CreateOperation } from '../contracts/generated/CreateOperation';
import type { EntitiesCreated } from '../contracts/generated/EntitiesCreated';
import type { EntityGeometry as NewGeometry } from '../contracts/generated/EntityGeometry';
import type { EntityGeometry } from '../model/entities';
import { entitiesCreate } from '../product/entitiesCreate';

/**
 * How the drawing tools write objects that have no command of their own
 * (docs/adr/0057): through the product command `cad.entities.create`, with
 * the geometry the shared core computed, on the active layer and in the
 * current colour, explicit in the input (TODOS.md CMD-07). The command's
 * refusal or warnings are the tool's messages (the locked and hidden layer
 * texts are the tools' own); one undo step, “Ekle” or the tool's name.
 */
export function writeObjects(ctx: AppContext, geometries: readonly EntityGeometry[], operation?: CreateOperation): EntitiesCreated | null {
  const color = ctx.settings.color.value;
  const objects = geometries.map((g) => ({ geometry: g as unknown as NewGeometry, ...(color !== null && { color }) }));
  const result = entitiesCreate.execute({ doc: ctx.doc }, { layerId: ctx.doc.layers.active.value, objects, ...(operation && { operation }) });
  if (result.status !== 'completed') {
    if ('error' in result) ctx.log.warn(result.error.message);
    return null;
  }
  for (const w of result.warnings) ctx.log.warn(w.message);
  return result.output;
}
