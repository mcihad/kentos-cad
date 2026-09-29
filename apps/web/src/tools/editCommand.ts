import type { AppContext } from '../app/context';
import type { EditOperation } from '../contracts/generated/EditOperation';
import type { EntitiesEdited } from '../contracts/generated/EntitiesEdited';
import type { EntityEdit } from '../contracts/generated/EntityEdit';
import type { EntityGeometry as EditGeometry } from '../contracts/generated/EntityGeometry';
import type { Entity, EntityGeometry } from '../model/entities';
import { withoutElevations } from '../product/elevation';
import { entitiesEdit } from '../product/entitiesEdit';
import { mapElevations } from '../product/elevationValues';

/**
 * How the edge, corner and object tools write (docs/adr/0047): through the
 * product command `cad.entities.edit`, with the objects they picked by
 * persistent id and the geometry the shared core computed (TODOS.md CMD-07).
 * The command's refusal is the tool's warning; one undo step, the tool's name.
 */

/** Writes `changes` as one edit; the command's answer, or null when nothing was written. */
export function writeEdit(ctx: AppContext, operation: EditOperation, changes: EntityEdit[]): EntitiesEdited | null {
  const result = entitiesEdit.execute({ doc: ctx.doc }, { operation, changes });
  if (result.status !== 'completed') {
    if ('error' in result) ctx.log.warn(result.error.message);
    return null;
  }
  for (const w of result.warnings) ctx.log.warn(w.message);
  return result.output;
}

/** An object's persistent id, as the command names it (every object of the drawing has one; ADR 0014). */
export function uidOf(ctx: AppContext, e: Entity | number): string {
  return ctx.doc.uidOf(typeof e === 'number' ? e : e.id) ?? '';
}

/**
 * A geometry the core computed, as the command takes it: the command writes only its own fields, an area's parts
 * (docs/adr/0143) among them, and no elevations (the core has none): each vertex takes its own from the objects the
 * edit names (docs/adr/0142). The `zs` an object the core gave back still carries are the old ones.
 */
export const editGeometry = (g: EntityGeometry): EditGeometry => withoutElevations(g) as unknown as EditGeometry;

/** The slots of the objects an edit made, in its order. */
export function createdIds(ctx: AppContext, out: EntitiesEdited): number[] {
  return out.created.map((uid) => ctx.doc.byUid(uid)?.id).filter((id): id is number => id !== undefined);
}

/**
 * Writes vertex elevations (docs/adr/0142): each object with its own geometry and the elevation `f` gives each vertex
 * (its elevation and its place in `vertexElevations`' order), as one `cad.entities.edit` with the operation
 * `elevation`, the undo step “Kot ver”. Kot ver and Öznitelikler's Kot rows write here. The command's answer, or
 * null when nothing was written: it refused (its message is in the log) or no object takes elevations.
 */
export function writeElevations(ctx: AppContext, entities: readonly Entity[], f: (z: number | null, index: number) => number | null): EntitiesEdited | null {
  const changes: EntityEdit[] = [];
  for (const e of entities) {
    const geometry = mapElevations(e, f);
    if (geometry) changes.push({ kind: 'update', uid: uidOf(ctx, e), geometry });
  }
  return changes.length ? writeEdit(ctx, 'elevation', changes) : null;
}
