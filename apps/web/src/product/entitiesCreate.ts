import type { CommandWarning } from '../contracts/generated/CommandWarning';
import type { CreateOperation } from '../contracts/generated/CreateOperation';
import type { EntitiesCreate } from '../contracts/generated/EntitiesCreate';
import type { EntitiesCreated } from '../contracts/generated/EntitiesCreated';
import type { EntitiesCreatePlan } from '../contracts/generated/EntitiesCreatePlan';
import type { Entity as PlannedEntity } from '../contracts/generated/Entity';
import type { NewObject } from '../contracts/generated/NewObject';
import type { CadDocument } from '../model/document';
import type { NewEntity } from '../model/entities';
import { checkLayer, checkRevision, error, failed, validated, type Stop } from './checks';
import type { ProductCommand } from './command';
import { checkGeometry, geometryOf } from './entitiesEdit';

/**
 * `cad.entities.create` v1 (docs/adr/0057): new objects of any kind on a
 * named layer, as one undo step named “Ekle” or after the drawing tool. The
 * web's handler over `CadDocument`; the desktop's is
 * `crates/native/application/src/create.rs`. Both pass the shared cases in
 * fixtures/commands/v1/cad.entities.create.json.
 *
 * Elips, Eğri, Yardımcı çizgi, Işın, Halka, Paralel çizgi, Dik in, Dik çık
 * and Böl compute the geometry with the shared core and write it here
 * (TODOS.md CMD-07); nothing is computed in this module.
 *
 * The checks, in order (the first that fails answers): at least one object;
 * every geometry, in order, by `cad.entities.edit`'s rules (enough points
 * for its kind, every number finite, a positive radius); the expected
 * revision, then the layer (checks.ts, for the reasons polygonCreate.ts gives).
 */

/** The undo step's name: the drawing tool's when it has its own, else the document's “Ekle”. */
export const CREATE_LABEL: Record<CreateOperation, string> = {
  parallel: 'Paralel çizgi',
  perpendicularIn: 'Dik in',
  perpendicularOut: 'Dik çık',
  divide: 'Böl',
  hatch: 'Tarama',
};

/** The checks in the contract's order: why nothing may be written, or the warnings when it may. */
function check(doc: CadDocument, input: EntitiesCreate): Stop | CommandWarning[] {
  if (!input.objects.length) return failed(error('no_objects', 'Eklenecek nesne verilmedi. En az bir nesne verin.', 'objects'));
  for (const [i, o] of input.objects.entries()) {
    const stop = checkGeometry(o.geometry, i, 'objects', 'nesnenin');
    if (stop) return stop;
  }
  return checkRevision(doc, input.expectedRevision) ?? checkLayer(doc, input.layerId);
}

/** An object as the document stores it: its own copies of every field, never the caller's objects. */
function entityOf(o: NewObject, layerId: string): NewEntity {
  return {
    ...geometryOf(o.geometry),
    layerId,
    ...(o.color != null && { color: o.color }),
    attrs: { ...o.attrs },
    ...(o.label != null && { label: o.label }),
  } as unknown as NewEntity;
}

export const entitiesCreate: ProductCommand<EntitiesCreate, EntitiesCreated, EntitiesCreatePlan> = {
  id: 'cad.entities.create',
  version: 1,

  validate(cx, input) {
    return validated(check(cx.doc, input));
  },

  plan(cx, input) {
    const checked = check(cx.doc, input);
    if (!Array.isArray(checked)) return checked;
    // The slots are given when they are written: 0 until then.
    const entities = input.objects.map((o) => ({ ...entityOf(o, input.layerId), id: 0 }) as unknown as PlannedEntity);
    return { status: 'completed', output: { entities, revision: String(cx.doc.revision) }, warnings: checked };
  },

  /**
   * Writes the objects in their order as one undo step through the
   * document's own `addMany`, named after the tool or “Ekle”; into the open
   * transaction or group, if one is.
   */
  execute(cx, input) {
    const checked = check(cx.doc, input);
    if (!Array.isArray(checked)) return checked;
    const label = input.operation ? CREATE_LABEL[input.operation] : 'Ekle';
    const written = cx.doc.addMany(
      input.objects.map((o) => entityOf(o, input.layerId)),
      label,
    );
    return {
      status: 'completed',
      output: { created: written.map((e) => e.uid), ids: written.map((e) => e.id), revision: String(cx.doc.revision) },
      warnings: checked,
    };
  },
};
