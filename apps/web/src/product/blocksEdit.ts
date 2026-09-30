import type { AttributeDefinition } from '../contracts/generated/AttributeDefinition';
import type { BlockEditOperation } from '../contracts/generated/BlockEditOperation';
import type { BlocksEdit } from '../contracts/generated/BlocksEdit';
import type { BlocksEdited } from '../contracts/generated/BlocksEdited';
import type { BlocksEditPlan } from '../contracts/generated/BlocksEditPlan';
import type { CommandWarning } from '../contracts/generated/CommandWarning';
import type { Entity as ContractEntity } from '../contracts/generated/Entity';
import type { BlockDefinition } from '../model/blocks';
import { Refusal, type CadDocument } from '../model/document';
import { sameJson } from '../model/sameJson';
import { checkBlockRules, checkName, checkReplaced, copies, noLayer, writeBlock, type Replaced } from './blocksDefine';
import { checkRevision, checkUids, error, failed, findObjects, isBlank, notFinite, notFiniteValue, validated, type Stop } from './checks';
import type { ProductCommand } from './command';

/**
 * `cad.blocks.edit` v1 (docs/adr/0144 §4): one change of the drawing's block
 * definitions as one undo step named after it: a new name, new objects, a new
 * base point or new attribute definitions (“Blok değiştir”), a definition
 * deleted (“Blok sil”) or every unused one (“Blokları temizle”). Every insert
 * shows a changed definition.
 * The web's handler over `CadDocument`; the desktop's is
 * `crates/native/application/src/blocks_edit.rs`. Both pass the shared cases
 * in fixtures/commands/v1/cad.blocks.edit.json.
 *
 * What would change nothing writes nothing and completes with nothing in
 * `changed` and `removed`.
 *
 * The checks, in order (the first that fails answers): the block given where
 * the operation needs one; what the operation needs from the input (a name,
 * objects and a base point, a layer, the attribute definitions); the expected revision; the block is
 * the drawing's; the operation's own: the block rules with the definition as
 * it would be, the objects (`redefine`), a definition an insert uses
 * (`remove`).
 */

/** The undo step's name (docs/adr/0144 §4). */
export const BLOCK_EDIT_LABEL: Record<BlockEditOperation, string> = {
  rename: 'Blok değiştir',
  redefine: 'Blok değiştir',
  rebase: 'Blok değiştir',
  remove: 'Blok sil',
  purge: 'Blokları temizle',
  attributes: 'Blok değiştir',
};

/**
 * The attribute definitions `attributes` writes (docs/adr/0144 §7): the list
 * given, then each in its order: a tag, not one an earlier one has (exactly),
 * a finite point and turn, a height above zero.
 */
function checkAttributes(list: readonly AttributeDefinition[] | undefined): Stop | null {
  if (!list) return failed(error('no_attributes', 'Öznitelik listesi verilmedi. Bloğun bütün özniteliklerini verin; boş liste hepsini kaldırır.', 'attributes'));
  for (const [i, a] of list.entries()) {
    const at = `attributes[${i}]`;
    if (isBlank(a.tag)) return failed(error('empty_tag', 'Öznitelik etiketi boş olamaz; her özniteliğe bir etiket verin.', `${at}.tag`));
    if (list.slice(0, i).some((b) => b.tag === a.tag)) return failed(error('duplicate_tag', `“${a.tag}” etiketi listede iki kez var; her etiket bir kez olmalı.`, `${at}.tag`));
    const stop = notFinite(a.p, `“${a.tag}” özniteliğinin yerinin`, `${at}.p`) ?? notFiniteValue(a.rotation, `“${a.tag}” özniteliğinin açısı`, 'Açıyı sonlu bir sayıyla verin.', `${at}.rotation`);
    if (stop) return stop;
    if (!(Number.isFinite(a.height) && a.height > 0))
      return failed(error('invalid_height', `“${a.tag}” özniteliğinin yazı yüksekliği sıfırdan büyük, sonlu bir sayı olmalı (metre, bloğun kendi ölçüsünde).`, `${at}.height`));
  }
  return null;
}

interface Checked {
  /** The definition as it will be, when it changes. */
  changed: BlockDefinition | null;
  /** The definitions to delete, in the order they can go. */
  removed: string[];
  /** With `replace`: the objects to delete and the insert. */
  replaced: Replaced | null;
  warnings: CommandWarning[];
}

const nothing = (): Checked => ({ changed: null, removed: [], replaced: null, warnings: [] });

/**
 * The definitions no insert uses, in the order they can go: first those no
 * object or other definition places, then those only they placed, and so on.
 */
function unused(doc: CadDocument): string[] {
  const gone: string[] = [];
  let left = [...doc.blocks.value];
  for (;;) {
    const used = new Set<string>();
    for (const e of doc.all()) if (e.kind === 'insert') used.add(e.block);
    for (const b of left) for (const e of b.entities) if (e.kind === 'insert') used.add(e.block);
    const round = left.filter((b) => !used.has(b.id)).map((b) => b.id);
    if (!round.length) return gone;
    left = left.filter((b) => !round.includes(b.id));
    gone.push(...round);
  }
}

/** `ids` in the order the drawing lists its definitions. */
const inDrawingOrder = (doc: CadDocument, ids: readonly string[]): string[] => doc.blocks.value.map((b) => b.id).filter((id) => ids.includes(id));

/** The block rules with `block` in its definition's place. */
const rulesWith = (doc: CadDocument, block: BlockDefinition): Stop | null => checkBlockRules(doc.blocks.value.map((b) => (b.id === block.id ? block : b)));

/** Whether two definitions are the same, as their JSON is (the document's own comparison, `updateBlock`). */
const same = (a: BlockDefinition, b: BlockDefinition): boolean => sameJson(a, b);

/** The checks in the contract's order. */
function check(doc: CadDocument, input: BlocksEdit): Stop | Checked {
  if (input.operation !== 'purge' && input.block === undefined) return failed(error('no_block', 'Değiştirilecek blok verilmedi. Bloğun kimliğini verin.', 'block'));
  const uids = input.uids ?? [];
  if (input.operation === 'rename') {
    const stop = checkName(input.name ?? '');
    if (stop) return stop;
  } else if (input.operation === 'redefine') {
    const stop = checkUids(uids, 'Bloğa girecek nesne verilmedi.') ?? (input.base ? notFinite(input.base, 'Taban noktasının', 'base') : null);
    if (stop) return stop;
    if (input.replace === true && input.layerId === undefined) return noLayer();
  } else if (input.operation === 'rebase') {
    if (!input.base) return failed(error('no_base', 'Yeni taban noktası verilmedi. Bloğun taban noktasını verin.', 'base'));
    const stop = notFinite(input.base, 'Taban noktasının', 'base');
    if (stop) return stop;
  } else if (input.operation === 'attributes') {
    const stop = checkAttributes(input.attributes);
    if (stop) return stop;
  }
  const revision = checkRevision(doc, input.expectedRevision);
  if (revision) return revision;
  if (input.operation === 'purge') return { ...nothing(), removed: unused(doc) };
  const old = input.block === undefined ? undefined : doc.block(input.block);
  if (!old)
    return failed(
      error(
        'unknown_block',
        `“${input.block ?? ''}” kimlikli blok çizimde tanımlı değil: silinmiş ya da başka bir çizimin olabilir. Çizimde tanımlı bir bloğun kimliğini verin.`,
        'block',
      ),
    );
  const checked = nothing();
  let next: BlockDefinition = structuredClone(old);
  switch (input.operation) {
    case 'rename': {
      next = { ...next, name: input.name ?? '' };
      const stop = rulesWith(doc, next);
      if (stop) return stop;
      break;
    }
    case 'rebase': {
      next = { ...next, base: { x: input.base!.x, y: input.base!.y } };
      const stop = rulesWith(doc, next);
      if (stop) return stop;
      break;
    }
    case 'attributes': {
      next = { ...next, attributes: structuredClone(input.attributes ?? []) };
      if (!next.attributes?.length) delete next.attributes;
      const stop = rulesWith(doc, next);
      if (stop) return stop;
      break;
    }
    case 'redefine': {
      const found = findObjects(doc, uids);
      if (!Array.isArray(found)) return found;
      next = { ...next, entities: copies(found.map((f) => f.entity)), base: input.base ? { x: input.base.x, y: input.base.y } : next.base };
      const stop = rulesWith(doc, next);
      if (stop) return stop;
      if (input.replace === true && input.layerId !== undefined) {
        const replaced = checkReplaced(doc, found, input.layerId, next.base, old.id);
        if ('status' in replaced) return replaced;
        checked.replaced = replaced;
        checked.warnings = replaced.warnings;
      }
      break;
    }
    case 'remove': {
      const reason = doc.blockRemovalRefused(old.id);
      if (reason) return failed(error('block_in_use', reason, 'block'));
      return { ...checked, removed: [old.id] };
    }
  }
  if (!same(next, old)) checked.changed = next;
  return checked;
}

const isStop = (c: Stop | Checked): c is Stop => 'status' in c;

/** Deletes the definitions in their order as one undo step named `label`. */
function remove(doc: CadDocument, label: string, ids: readonly string[]): Stop | null {
  try {
    doc.transact(label, () => {
      for (const id of ids) doc.removeBlock(id);
    });
    return null;
  } catch (e) {
    if (e instanceof Refusal) return failed({ code: 'block_refused', message: e.message });
    throw e;
  }
}

export const blocksEdit: ProductCommand<BlocksEdit, BlocksEdited, BlocksEditPlan> = {
  id: 'cad.blocks.edit',
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
      output: {
        changed: checked.changed ? [structuredClone(checked.changed) as unknown as BlocksEditPlan['changed'][number]] : [],
        removed: inDrawingOrder(cx.doc, checked.removed),
        ...(checked.replaced && { insert: { ...structuredClone(checked.replaced.insert), id: 0 } as unknown as ContractEntity }),
        deleted: checked.replaced?.removed.map((r) => r.uid) ?? [],
        revision: String(cx.doc.revision),
      },
      warnings: checked.warnings,
    };
  },

  /** Writes the change as one undo step named after the operation. */
  execute(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    const doc = cx.doc;
    const removed = inDrawingOrder(doc, checked.removed);
    let insert: { id: number; uid: string } | null = null;
    if (checked.removed.length) {
      const stop = remove(doc, BLOCK_EDIT_LABEL[input.operation], checked.removed);
      if (stop) return stop;
    } else if (checked.changed || checked.replaced) {
      // The objects replaced by an insert of an unchanged definition write it as it is.
      const block = checked.changed ?? doc.block(input.block ?? '');
      if (block) {
        const written = writeBlock(doc, 'Blok değiştir', block, checked.replaced);
        if ('status' in written) return written;
        insert = written.insert;
      }
    }
    return {
      status: 'completed',
      output: {
        changed: checked.changed ? [checked.changed.id] : [],
        removed,
        ...(insert && { insert: insert.uid, id: insert.id }),
        deleted: checked.replaced?.removed.map((r) => r.uid) ?? [],
        revision: String(doc.revision),
      },
      warnings: checked.warnings,
    };
  },
};
