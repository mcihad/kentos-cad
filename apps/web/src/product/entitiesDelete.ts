import type { CommandWarning } from '../contracts/generated/CommandWarning';
import type { EntitiesDelete } from '../contracts/generated/EntitiesDelete';
import type { EntitiesDeleted } from '../contracts/generated/EntitiesDeleted';
import type { EntitiesDeletePlan } from '../contracts/generated/EntitiesDeletePlan';
import type { CadDocument } from '../model/document';
import { checkRevision, checkUids, error, failed, findObjects, validated, type Stop } from './checks';
import type { ProductCommand } from './command';

/**
 * `cad.entities.delete` v1 (docs/adr/0029): objects named by their
 * persistent ids deleted as one undo step (“Sil”, the document's own
 * `remove`). The web's handler over `CadDocument`; the desktop's is
 * `crates/native/application/src/delete.rs`. Both pass the shared cases in
 * fixtures/commands/v1/cad.entities.delete.json.
 *
 * The erase tool (Sil, Delete) makes the selection explicit here (TODOS.md
 * CMD-07): it gives the selected objects' ids, or the one it picked. Objects
 * on a locked layer stay, as the tool always left them: with others to
 * delete, with a warning; alone, the answer is a refusal.
 *
 * The checks, in order (the first that fails answers): at least one id, each
 * lowercase UUID text with hyphens; the expected revision (checks.ts); each
 * id names an object; not every object on a locked layer. A repeated id
 * counts once.
 */

interface Checked {
  /** The objects to delete: slot and id, in the input's order. */
  removed: { id: number; uid: string }[];
  /** The ids that stay on locked layers. */
  locked: string[];
  warnings: CommandWarning[];
}

const lockedMessage = (n: number) => `${n} nesne kilitli katmanda olduğu için silinmedi. Silmek için katmanın kilidini Katmanlar panelinden açın.`;

/** The checks in the contract's order: why nothing may be deleted, or what may. */
function check(doc: CadDocument, input: EntitiesDelete): Stop | Checked {
  const stop = checkUids(input.uids, 'Silinecek nesne verilmedi.') ?? checkRevision(doc, input.expectedRevision);
  if (stop) return stop;
  const found = findObjects(doc, input.uids);
  if ('status' in found) return found;
  const removed: Checked['removed'] = [];
  const locked: string[] = [];
  for (const { entity: e, uid } of found) {
    if (doc.layers.isLocked(e.layerId)) locked.push(uid);
    else removed.push({ id: e.id, uid });
  }
  if (!removed.length) return failed(error('layer_locked', lockedMessage(locked.length), 'uids'));
  const warnings: CommandWarning[] = locked.length ? [{ code: 'layer_locked', message: lockedMessage(locked.length), path: 'uids' }] : [];
  return { removed, locked, warnings };
}

const isStop = (c: Stop | Checked): c is Stop => 'status' in c;

export const entitiesDelete: ProductCommand<EntitiesDelete, EntitiesDeleted, EntitiesDeletePlan> = {
  id: 'cad.entities.delete',
  version: 1,

  validate(cx, input) {
    const checked = check(cx.doc, input);
    return validated(isStop(checked) ? checked : checked.warnings);
  },

  plan(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    return {
      status: 'completed',
      output: { removed: checked.removed.map((r) => r.uid), locked: checked.locked, revision: String(cx.doc.revision) },
      warnings: checked.warnings,
    };
  },

  /** Deletes through the document's own `remove`: undo step “Sil”, as the erase tool always deleted; into the open transaction or group, if one is. */
  execute(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    cx.doc.remove(checked.removed.map((r) => r.id));
    return {
      status: 'completed',
      output: { removed: checked.removed.map((r) => r.uid), locked: checked.locked, revision: String(cx.doc.revision) },
      warnings: checked.warnings,
    };
  },
};
