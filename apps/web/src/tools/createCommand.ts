import type { AppContext } from '../app/context';
import type { CreateOperation } from '../contracts/generated/CreateOperation';
import type { EntitiesCreated } from '../contracts/generated/EntitiesCreated';
import type { EntityGeometry as NewGeometry } from '../contracts/generated/EntityGeometry';
import { drawsLines, type EntityGeometry } from '../model/entities';
import { entitiesCreate } from '../product/entitiesCreate';
import { stamp } from './templateStamp';

/**
 * How the drawing tools write objects that have no command of their own
 * (docs/adr/0057): through the product command `cad.entities.create`, with
 * the geometry the shared core computed, on the active layer and in the
 * current colour and line weight, explicit in the input (TODOS.md CMD-07). The command's
 * refusal or warnings are the tool's messages (the locked and hidden layer
 * texts are the tools' own); one undo step, “Ekle” or the tool's name.
 */
export function writeObjects(ctx: AppContext, geometries: readonly EntityGeometry[], operation?: CreateOperation, attrs?: Record<string, string>): EntitiesCreated | null {
  // Blok ekle's values (docs/adr/0144 §7): the objects' attributes, when given.
  return writeObjectsEach(
    ctx,
    geometries.map((geometry) => ({ geometry, attrs })),
    operation,
  );
}

/** `writeObjects` with each object's own attributes (Km yaz's `Km`, docs/adr/0189 §3); the desktop's `write_objects_each`. */
export function writeObjectsEach(
  ctx: AppContext,
  items: readonly { geometry: EntityGeometry; attrs?: Record<string, string> }[],
  operation?: CreateOperation,
): EntitiesCreated | null {
  const color = ctx.settings.color.value;
  // The current weight goes to what is drawn with lines (docs/adr/0139).
  const lineWeight = ctx.settings.lineWeight.value;
  const objects = items.map(({ geometry: g, attrs }) => ({
    geometry: g as unknown as NewGeometry,
    ...(color !== null && { color }),
    ...(lineWeight !== null && drawsLines(g) && { lineWeight }),
    // An object template's symbol, attributes and label (docs/adr/0176 §3), the tool's own attributes over its.
    ...stamp(ctx, attrs),
  }));
  const result = entitiesCreate.execute({ doc: ctx.doc }, { layerId: ctx.doc.layers.active.value, objects, ...(operation && { operation }) });
  if (result.status !== 'completed') {
    if ('error' in result) ctx.log.warn(result.error.message);
    return null;
  }
  for (const w of result.warnings) ctx.log.warn(w.message);
  return result.output;
}
