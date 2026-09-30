import type { AppContext } from '../../app/context';
import type { EntitiesEdited } from '../../contracts/generated/EntitiesEdited';
import type { EntitiesPropertiesSet } from '../../contracts/generated/EntitiesPropertiesSet';
import type { EntitiesSetProperties } from '../../contracts/generated/EntitiesSetProperties';
import type { EntityGeometry as EditGeometry } from '../../contracts/generated/EntityGeometry';
import type { Entity } from '../../model/entities';
import { geometryOf } from '../../product/entitiesEdit';
import { entitiesSet } from '../../product/entitiesSet';
import { uidOf, writeEdit } from '../../tools/editCommand';

/**
 * How Öznitelikler, the in-place text editor and the symbol commands write:
 * through product commands, with the objects by persistent id and the value
 * explicit in the input (TODOS.md CMD-07). A layer, colour, symbol,
 * attribute or label goes through `cad.entities.set`; a value of the
 * geometry (a point's Y, a text, a hatch's spacing) through
 * `cad.entities.edit`, operation `properties`. A refusal or a warning of the
 * command is said in the log; the undo steps are named as before.
 */

/** The persistent ids of the objects in these slots, as the commands name them. */
export function uidsOf(ctx: AppContext, ids: Iterable<number>): string[] {
  return [...ids].map((id) => ctx.doc.uidOf(id)).filter((uid): uid is string => uid !== undefined);
}

/** Sets the objects' layer, colour, symbol, attributes or label; the command's answer, or null when it refused. */
export function setProperties(ctx: AppContext, input: EntitiesSetProperties): EntitiesPropertiesSet | null {
  const result = entitiesSet.execute({ doc: ctx.doc }, input);
  if (result.status !== 'completed') {
    if ('error' in result) ctx.log.warn(result.error.message);
    return null;
  }
  for (const w of result.warnings) ctx.log.warn(w.message);
  return result.output;
}

/**
 * `e` with some fields of its geometry changed (`{ text: 'Park' }`; a field
 * given undefined is taken away, as a dimension's own text), written in its
 * place as the step “Değiştir”; the command's answer, or null when it refused.
 */
export function setGeometry(ctx: AppContext, e: Entity, patch: Record<string, unknown>): EntitiesEdited | null {
  const geometry = { ...geometryOf(e as unknown as EditGeometry), ...patch } as unknown as EditGeometry;
  return writeEdit(ctx, 'properties', [{ kind: 'update', uid: uidOf(ctx, e), geometry }]);
}

/**
 * Several objects, each with some fields of its geometry changed, written in one step “Değiştir” (Öznitelikler's
 * rows over a selection, docs/adr/0145 §6); nothing when none changes. The command's answer, or null when it refused.
 */
export function setGeometries(ctx: AppContext, changes: readonly { e: Entity; patch: Record<string, unknown> }[]): EntitiesEdited | null {
  if (!changes.length) return null;
  return writeEdit(
    ctx,
    'properties',
    changes.map(({ e, patch }) => ({ kind: 'update', uid: uidOf(ctx, e), geometry: { ...geometryOf(e as unknown as EditGeometry), ...patch } as unknown as EditGeometry })),
  );
}
