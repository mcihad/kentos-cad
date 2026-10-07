import type { CommandWarning } from '../contracts/generated/CommandWarning';
import type { EntitiesPropertiesSet } from '../contracts/generated/EntitiesPropertiesSet';
import type { EntitiesSetProperties } from '../contracts/generated/EntitiesSetProperties';
import type { EntitiesSetPropertiesPlan } from '../contracts/generated/EntitiesSetPropertiesPlan';
import type { Entity as PlannedEntity } from '../contracts/generated/Entity';
import type { PropertiesOperation } from '../contracts/generated/PropertiesOperation';
import type { CadDocument } from '../model/document';
import type { Entity } from '../model/entities';
import { fieldValues, checkLineWeight, checkRevision, checkUids, error, failed, findObjects, isBlank, validated, type Stop } from './checks';
import type { ProductCommand } from './command';

/**
 * `cad.entities.set` v1: the layer, colour, line weight (docs/adr/0139),
 * symbol, attributes or label of objects named by their persistent ids, as one undo step named after the
 * operation. The web's handler over `CadDocument`; the desktop's is in
 * `crates/native/application`. Both pass the shared cases in
 * fixtures/commands/v1/cad.entities.set.json.
 *
 * Öznitelikler's Katman, Renk, Kalınlık and attribute rows, Sembol ver and Sembolü
 * kaldır write here. They put the objects and the value in the input
 * (TODOS.md CMD-07): the command reads no selection and no library.
 *
 * The checks, in order (the first that fails answers): at least one id,
 * each lowercase UUID text with hyphens; something to set (Bağı kopar's `unlink` too, docs/adr/0175 §4); no attribute
 * name empty or only white space; the line weight a number from 0 to 100
 * mm; the expected revision (checks.ts); each
 * id names an object; the layer given is one, not a group; no object on a
 * locked layer, then the layer given not locked. An object already as asked
 * is left alone; when none changes nothing is written.
 */

/** The undo step's name, the one Öznitelikler and the symbol commands wrote before the command. */
const LABEL: Record<Exclude<PropertiesOperation, 'symbol'>, string> = {
  layer: 'Katman değiştir',
  color: 'Renk değiştir',
  lineWeight: 'Kalınlık değiştir',
  attributes: 'Değiştir',
  label: 'Etiket değiştir',
  // Öznitelikler's Bağı kopar (docs/adr/0175 §4).
  unlink: 'Bağı kopar',
  // An object template given to objects (docs/adr/0176 §6).
  template: 'Şablonu uygula',
};

/** The step a call is named: its operation's, or for a symbol whether it is given or taken away. */
export function setLabel(input: Pick<EntitiesSetProperties, 'operation' | 'symbol'>): string {
  if (input.operation === 'symbol') return input.symbol === null ? 'Sembolü kaldır' : 'Sembol ata';
  return LABEL[input.operation];
}

interface Change {
  id: number;
  uid: string;
  /** What changes of the object, as `CadDocument.updateMany` takes it: a field undefined is removed. */
  patch: Partial<Entity>;
}

interface Checked {
  changes: Change[];
  warnings: CommandWarning[];
}

/** An attribute's value, or null when the object has none by that name (an own name only: “constructor” is one too). */
const attrOf = (e: Entity, key: string): string | null => (Object.hasOwn(e.attrs, key) ? e.attrs[key] : null);

/** What the input changes of `e`, its attributes `attrs` (the input's in their canonical text), or null when `e` already is as asked. */
function patchOf(e: Entity, input: EntitiesSetProperties, attrs: readonly (readonly [string, string | null])[]): Partial<Entity> | null {
  const patch: Record<string, unknown> = {};
  // A linked text follows its object no more (docs/adr/0175 §4).
  if (input.unlink && e.kind === 'text' && e.labelOf !== undefined) {
    patch.labelOf = undefined;
    patch.labelScale = undefined;
  }
  // An associative hatch follows its objects no more (docs/adr/0186 §6).
  if (input.unlink && e.kind === 'hatch' && e.assoc !== undefined) patch.assoc = undefined;
  if (input.layerId != null && e.layerId !== input.layerId) patch.layerId = input.layerId;
  for (const key of ['color', 'lineWeight', 'symbol', 'label'] as const) {
    const want = input[key];
    if (want !== undefined && (e[key] ?? null) !== want) patch[key] = want ?? undefined;
  }
  if (attrs.some(([key, value]) => attrOf(e, key) !== value)) {
    // A Map keeps the names in their order and takes any name (`__proto__` too) as a plain key.
    const next = new Map(Object.entries(e.attrs));
    for (const [key, value] of attrs) {
      if (value === null) next.delete(key);
      else next.set(key, value);
    }
    patch.attrs = Object.fromEntries(next);
  }
  return Object.keys(patch).length ? (patch as Partial<Entity>) : null;
}

/** The checks in the contract's order: why nothing may be written, or what changes and the warnings. */
function check(doc: CadDocument, input: EntitiesSetProperties): Stop | Checked {
  const stop = checkUids(input.uids, 'Özellikleri değişecek nesne verilmedi.');
  if (stop) return stop;
  const attrs = Object.keys(input.attrs ?? {});
  if (input.layerId == null && input.color === undefined && input.lineWeight === undefined && input.symbol === undefined && !attrs.length && input.label === undefined && !input.unlink)
    return failed({ code: 'nothing_to_set', message: 'Değişecek özellik verilmedi. Katman, renk, kalınlık, sembol, öznitelik ya da etiket verin.' });
  if (attrs.some(isBlank))
    return failed(error('invalid_attribute', 'Öznitelik adı boş olamaz; yalnız boşluktan oluşan ad da boştur. Özniteliğe bir ad verin.', 'attrs'));
  const weight = checkLineWeight(input.lineWeight, 'lineWeight');
  if (weight) return weight;
  const revision = checkRevision(doc, input.expectedRevision);
  if (revision) return revision;
  const found = findObjects(doc, input.uids);
  if ('status' in found) return found;
  const layers = doc.layers;
  const target = input.layerId == null ? null : layers.get(input.layerId);
  if (input.layerId != null) {
    if (!target) return failed(error('layer_not_found', `“${input.layerId}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.`, 'layerId'));
    if (target.type !== 'layer')
      return failed(error('not_a_layer', `“${target.name}” bir katman grubu; nesneler yalnız bir katmana taşınır. Grubun içinden bir katman seçin.`, 'layerId'));
  }
  // Nothing on a locked layer changes, and nothing moves onto one.
  for (const f of found) {
    if (!layers.isLocked(f.entity.layerId)) continue;
    const name = layers.get(f.entity.layerId)?.name ?? f.entity.layerId;
    return failed(error('layer_locked', `“${name}” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın.`, `uids[${f.at}]`));
  }
  if (target && layers.isLocked(target.id))
    return failed(error('layer_locked', `“${target.name}” katmanı kilitli; nesneler ona taşınamaz. Kilidi Katmanlar panelinden açın ya da başka bir katman seçin.`, 'layerId'));
  // The values given to a layer's fields, in their canonical text (docs/adr/0199 §2).
  const given: [string, string | null][][] = [];
  for (const f of found) {
    const values = attrs.length ? fieldValues(doc, input.layerId ?? f.entity.layerId, Object.entries(input.attrs ?? {}), 'attrs') : [];
    if ('status' in values) return values;
    given.push(values);
  }
  const changes: Change[] = [];
  for (const [i, f] of found.entries()) {
    const patch = patchOf(f.entity, input, given[i]);
    if (patch) changes.push({ id: f.entity.id, uid: f.uid, patch });
  }
  // On a hidden layer they vanish from the drawing: said when at least one moves there.
  const hidden = target && !layers.isVisible(target.id) && changes.some((c) => 'layerId' in c.patch);
  const warnings: CommandWarning[] = hidden ? [{ code: 'layer_hidden', message: `“${target.name}” katmanı gizli; taşınan nesneler görünmeyecek.`, path: 'layerId' }] : [];
  return { changes, warnings };
}

const isStop = (c: Stop | Checked): c is Stop => 'status' in c;

/** An object as the plan shows it: as it would be written, without its persistent id or fields left undefined. */
function planned(e: Entity, patch: Partial<Entity>): PlannedEntity {
  const { uid: _uid, ...rest } = JSON.parse(JSON.stringify({ ...e, ...patch })) as Entity & { uid?: string };
  return rest as unknown as PlannedEntity;
}

export const entitiesSet: ProductCommand<EntitiesSetProperties, EntitiesPropertiesSet, EntitiesSetPropertiesPlan> = {
  id: 'cad.entities.set',
  version: 1,

  validate(cx, input) {
    const checked = check(cx.doc, input);
    return validated(isStop(checked) ? checked : checked.warnings);
  },

  plan(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    const changed = checked.changes.map((c) => planned(cx.doc.get(c.id)!, c.patch));
    return { status: 'completed', output: { changed, revision: String(cx.doc.revision) }, warnings: checked.warnings };
  },

  /**
   * Writes one undo step named after the operation through the document's
   * own `updateMany`: every object keeps its slot and persistent id. Nothing
   * when no object changes. Into the open transaction or group, if one is.
   */
  execute(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    const { changes } = checked;
    if (changes.length)
      cx.doc.updateMany(
        changes.map((c) => ({ ...c.patch, id: c.id })),
        setLabel(input),
      );
    return {
      status: 'completed',
      output: { changed: changes.map((c) => c.uid), ids: changes.map((c) => c.id), revision: String(cx.doc.revision) },
      warnings: checked.warnings,
    };
  },
};
