import type { BlockDefined } from '../contracts/generated/BlockDefined';
import type { BlocksDefine } from '../contracts/generated/BlocksDefine';
import type { BlocksDefinePlan } from '../contracts/generated/BlocksDefinePlan';
import type { CommandWarning } from '../contracts/generated/CommandWarning';
import type { Entity as ContractEntity } from '../contracts/generated/Entity';
import type { Vec2 } from '../contracts/generated/Vec2';
import { uuidv7 } from '../core/uuid';
import { blockFaultMessage, definitionsFault, nameOk, type BlockDefinition } from '../model/blocks';
import { Refusal, type CadDocument } from '../model/document';
import type { Entity, NewEntity } from '../model/entities';
import { withoutLink } from '../model/linkedTexts';
import { checkLayer, checkRevision, checkUids, error, failed, findObjects, notFinite, validated, type Stop } from './checks';
import type { ProductCommand } from './command';

/**
 * `cad.blocks.define` v1 (docs/adr/0144 §4): a block definition made of a
 * drawing's objects, with the base point given, as one undo step (“Blok
 * tanımla”); with `replace` the objects are deleted and an insert of the new
 * block takes their place in the same step. The web's handler over
 * `CadDocument`; the desktop's is
 * `crates/native/application/src/blocks_define.rs`. Both pass the shared
 * cases in fixtures/commands/v1/cad.blocks.define.json.
 *
 * The objects are copied as they are: their geometry, layer, colour, line
 * weight, attributes, label and symbol, with local ids 1, 2, … in the order
 * the input first names them; an id given twice is one object.
 *
 * The checks, in order (the first that fails answers): a name that is not
 * blank; at least one id, each lowercase UUID text; a finite base point; with
 * `replace`, the insert's layer given; the expected revision; every id names
 * an object; the block rules with the new definition (its name once;
 * nesting); with `replace`, the insert's layer, then no object on a locked
 * layer.
 */

/** A nil id: the definition's and its insert's until execute gives the real one. */
export const NIL_BLOCK = '00000000-0000-0000-0000-000000000000';

/** A block's name: something besides white space (`empty_name`). */
/** None of the objects a block is defined from is a table: a block holds none (docs/adr/0184 §1), `table_in_block` at the first. */
export function checkNoTables(found: readonly { entity: Entity; at: number }[]): Stop | null {
  const table = found.find((f) => f.entity.kind === 'table');
  return table ? failed(error('table_in_block', 'Seçilenlerde tablo var; tablo bloğa konamaz. Tabloyu seçimden çıkarın.', `uids[${table.at}]`)) : null;
}

export function checkName(name: string): Stop | null {
  return nameOk(name) ? null : failed(error('empty_name', 'Blok adı boş olamaz; bir ad yazın.', 'name'));
}

/** `replace` without the layer the insert goes on (`no_layer`). */
export const noLayer = (): Stop =>
  failed(error('no_layer', 'Yerleştirmenin katmanı verilmedi. Seçilenleri blokla değiştirmek için bir katman verin (Blok oluştur etkin katmanı verir).', 'layerId'));

/**
 * The objects as a definition holds them: every field kept (their own
 * copies, in their own order), local ids 1, 2, … in order, no persistent id;
 * a linked text's copy writes no object's label (docs/adr/0175 §4).
 */
export function copies(objects: readonly Entity[]): Entity[] {
  return objects.map((e, k) => {
    const out: Record<string, unknown> = {};
    for (const [key, value] of Object.entries(structuredClone(withoutLink(e)) as unknown as Record<string, unknown>)) {
      if (key === 'uid') continue;
      out[key] = key === 'id' ? k + 1 : value;
    }
    return out as unknown as Entity;
  });
}

/**
 * The drawing's definitions as they would be, by the block rules
 * (docs/adr/0144): a name taken (`duplicate_block`), a definition holding
 * itself (`block_cycle`), nesting too deep (`block_too_deep`), in the
 * documents' words.
 */
export function checkBlockRules(next: readonly BlockDefinition[]): Stop | null {
  const fault = definitionsFault(next);
  if (!fault) return null;
  const message = blockFaultMessage(fault, (i) => next[i]?.name ?? '');
  const [code, path] =
    fault.kind === 'emptyName'
      ? ['empty_name', 'name']
      : fault.kind === 'duplicateId' || fault.kind === 'duplicateName'
        ? ['duplicate_block', 'name']
        : fault.kind === 'cycle'
          ? ['block_cycle', 'uids']
          : fault.kind === 'tooDeep'
            ? ['block_too_deep', 'uids']
            : fault.kind === 'badScale'
              ? ['invalid_scale', 'uids']
              : fault.kind === 'unknownBlock'
                ? ['unknown_block', 'uids']
                : // `attributes` checks its list first: never expected.
                  [fault.kind === 'emptyTag' ? 'empty_tag' : 'duplicate_tag', `attributes[${fault.attribute}].tag`];
  return failed(error(code, message, path));
}

/** What `replace` writes besides the definition: the objects to delete (slot and id) and the insert (slot 0). */
export interface Replaced {
  removed: { id: number; uid: string }[];
  insert: NewEntity;
  warnings: CommandWarning[];
}

/**
 * With `replace`: the insert's layer (known, a layer, not locked; hidden with
 * a warning), then every object off locked layers; the objects to delete and
 * the insert of `block` at `base`.
 */
export function checkReplaced(doc: CadDocument, found: readonly { entity: Entity; uid: string; at: number }[], layerId: string, base: Vec2, block: string): Stop | Replaced {
  const warnings = checkLayer(doc, layerId);
  if (!Array.isArray(warnings)) return warnings;
  for (const { entity, at } of found) {
    if (!doc.layers.isLocked(entity.layerId)) continue;
    const name = doc.layers.get(entity.layerId)?.name ?? entity.layerId;
    return failed(
      error(
        'layer_locked',
        `“${name}” katmanı kilitli; üzerindeki nesne bloğa alınır ama yerine yerleştirme konamaz. Kilidi Katmanlar panelinden açın ya da nesneleri yerinde bırakın.`,
        `uids[${at}]`,
      ),
    );
  }
  return {
    removed: found.map(({ entity, uid }) => ({ id: entity.id, uid })),
    insert: { kind: 'insert', layerId, attrs: {}, block, p: { x: base.x, y: base.y }, scale: 1, rotation: 0 } as NewEntity,
    warnings,
  };
}

/**
 * Writes the definition (added, or in its place when the drawing has its id)
 * and, with `replace`, the deletion and the insert, as one undo step named
 * `label`: the insert as written. A refusal of the document (a block rule the
 * checks passed: never expected) is said in its words.
 */
export function writeBlock(doc: CadDocument, label: string, block: BlockDefinition, replaced: Replaced | null): Stop | { insert: { id: number; uid: string } | null } {
  try {
    let insert: { id: number; uid: string } | null = null;
    doc.transact(label, () => {
      if (doc.block(block.id)) doc.updateBlock(block);
      else doc.addBlock(block);
      if (!replaced) return;
      doc.remove(replaced.removed.map((r) => r.id));
      const [written] = doc.addMany([replaced.insert], label);
      insert = { id: written.id, uid: written.uid };
    });
    return { insert };
  } catch (e) {
    if (e instanceof Refusal) return failed({ code: 'block_refused', message: e.message });
    throw e;
  }
}

interface Checked {
  block: BlockDefinition;
  replaced: Replaced | null;
  warnings: CommandWarning[];
}

/** The checks in the contract's order. */
function check(doc: CadDocument, input: BlocksDefine): Stop | Checked {
  const stop = checkName(input.name) ?? checkUids(input.uids, 'Bloğa girecek nesne verilmedi.') ?? notFinite(input.base, 'Taban noktasının', 'base');
  if (stop) return stop;
  const replace = input.replace === true;
  if (replace && input.layerId === undefined) return noLayer();
  const revision = checkRevision(doc, input.expectedRevision);
  if (revision) return revision;
  const found = findObjects(doc, input.uids);
  if (!Array.isArray(found)) return found;
  const tables = checkNoTables(found);
  if (tables) return tables;
  const block: BlockDefinition = {
    id: NIL_BLOCK,
    name: input.name,
    base: { x: input.base.x, y: input.base.y },
    entities: copies(found.map((f) => f.entity)),
    ...(input.description !== undefined && { description: input.description }),
  };
  const rules = checkBlockRules([...doc.blocks.value, block]);
  if (rules) return rules;
  if (!replace || input.layerId === undefined) return { block, replaced: null, warnings: [] };
  const replaced = checkReplaced(doc, found, input.layerId, input.base, NIL_BLOCK);
  if ('status' in replaced) return replaced;
  return { block, replaced, warnings: replaced.warnings };
}

const isStop = (c: Stop | Checked): c is Stop => 'status' in c;

export const blocksDefine: ProductCommand<BlocksDefine, BlockDefined, BlocksDefinePlan> = {
  id: 'cad.blocks.define',
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
        block: structuredClone(checked.block) as unknown as BlocksDefinePlan['block'],
        ...(checked.replaced && { insert: { ...structuredClone(checked.replaced.insert), id: 0 } as unknown as ContractEntity }),
        removed: checked.replaced?.removed.map((r) => r.uid) ?? [],
        revision: String(cx.doc.revision),
      },
      warnings: checked.warnings,
    };
  },

  /** Writes the definition, and with `replace` the deletion and the insert, as one undo step “Blok tanımla”. */
  execute(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    const id = uuidv7();
    const block = { ...checked.block, id };
    const replaced = checked.replaced && { ...checked.replaced, insert: { ...checked.replaced.insert, block: id } as NewEntity };
    const written = writeBlock(cx.doc, 'Blok tanımla', block, replaced);
    if ('status' in written) return written;
    return {
      status: 'completed',
      output: {
        block: id,
        ...(written.insert && { insert: written.insert.uid, id: written.insert.id }),
        removed: checked.replaced?.removed.map((r) => r.uid) ?? [],
        revision: String(cx.doc.revision),
      },
      warnings: checked.warnings,
    };
  },
};
