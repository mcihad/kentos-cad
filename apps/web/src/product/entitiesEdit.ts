import type { EditOperation } from '../contracts/generated/EditOperation';
import type { EntitiesEdit } from '../contracts/generated/EntitiesEdit';
import type { EntitiesEdited } from '../contracts/generated/EntitiesEdited';
import type { EntitiesEditPlan } from '../contracts/generated/EntitiesEditPlan';
import type { Entity as PlannedEntity } from '../contracts/generated/Entity';
import type { EntityEdit } from '../contracts/generated/EntityEdit';
import type { EntityGeometry } from '../contracts/generated/EntityGeometry';
import { isUuid } from '../core/uuid';
import type { CadDocument } from '../model/document';
import type { Entity, NewEntity } from '../model/entities';
import { geometryIsFinite, SHAPE_FIELDS } from '../model/ops/transform';
import { checkRevision, error, failed, validated, type Stop } from './checks';
import type { ProductCommand } from './command';

/**
 * `cad.entities.edit` v1 (docs/adr/0047): objects named by their persistent
 * ids given a new geometry, replaced in their place, followed by new objects
 * made from them, or deleted, as one undo step named after the modify tool.
 * The web's handler over `CadDocument`; the desktop's is
 * `crates/native/application/src/edit.rs`. Both pass the shared cases in
 * fixtures/commands/v1/cad.entities.edit.json.
 *
 * The edge, corner and object tools (Ötele, Buda, Uzat, Köşe yuvarla, Pah,
 * Kır, Birleştir, Patlat, Uzat-kısalt, Köşe ekle/sil) and Esnet compute the
 * geometry with the shared core and write it here (TODOS.md CMD-07);
 * nothing is computed in this module.
 *
 * The checks, in order (the first that fails answers): at least one change,
 * each change's id lowercase UUID text with hyphens; every geometry, in
 * order: enough points for its kind, every number finite, a positive
 * radius; the expected revision (checks.ts); each id names an object; no
 * object changed twice; no object on a locked layer (an edit is written
 * whole or not at all).
 */

/** The undo step's name: the tool's (docs/adr/0047). */
export const EDIT_LABEL: Record<EditOperation, string> = {
  offset: 'Ötele',
  trim: 'Buda',
  extend: 'Uzat',
  fillet: 'Köşe yuvarla',
  chamfer: 'Pah',
  break: 'Kır',
  join: 'Birleştir',
  explode: 'Patlat',
  lengthen: 'Uzat-kısalt',
  vertexAdd: 'Köşe ekle',
  vertexRemove: 'Köşe sil',
  stretch: 'Esnet',
};

/** The contract's geometry fields by kind (`EntityGeometry`): what the command writes of a geometry. */
const FIELDS: Record<EntityGeometry['kind'], readonly string[]> = {
  point: ['p', 'z'],
  line: ['a', 'b'],
  polyline: ['pts', 'bulges'],
  polygon: ['pts', 'bulges', 'holes'],
  circle: ['c', 'r'],
  arc: ['c', 'r', 'a0', 'a1'],
  ellipse: ['c', 'major', 'ratio', 't0', 't1'],
  spline: ['pts', 'closed'],
  xline: ['p', 'dir'],
  ray: ['p', 'dir'],
  text: ['p', 'text', 'height', 'rotation'],
  dimension: ['a', 'b', 'offset', 'height', 'text', 'style', 'angle', 'c'],
  hatch: ['ring', 'holes', 'pattern'],
};

interface Checked {
  /** The objects updated or replaced: slot, id and the object as it will be, in the input's order. */
  changed: { id: number; uid: string; init: NewEntity }[];
  /** The new objects, in the order of their `add`s. */
  created: NewEntity[];
  /** The objects to delete: slot and id, in the input's order. */
  removed: { id: number; uid: string }[];
}

/** The id a change names and the field that holds it: `uid`, or `from` for an `add`. */
const named = (c: EntityEdit): [string, 'uid' | 'from'] => (c.kind === 'add' ? [c.from, 'from'] : [c.uid, 'uid']);

/** A geometry's own fields, copied: nothing else a caller put beside them reaches the drawing. */
export function geometryOf(g: EntityGeometry): Record<string, unknown> {
  const src = g as unknown as Record<string, unknown>;
  const out: Record<string, unknown> = { kind: g.kind };
  for (const key of FIELDS[g.kind]) if (src[key] !== undefined) out[key] = structuredClone(src[key]);
  return out;
}

/**
 * `e` with another geometry: every field but its geometry kept (`update`),
 * in the object's own order, so an update that changes nothing is no edit
 * (`CadDocument.replace` compares as JSON writes, key order included).
 */
function reshaped(e: Entity, g: EntityGeometry): NewEntity {
  const src = e as unknown as Record<string, unknown>;
  const old = new Set(SHAPE_FIELDS[e.kind] ?? []);
  const geometry = geometryOf(g);
  const out: Record<string, unknown> = {};
  for (const key of Object.keys(src)) {
    if (key === 'kind') out.kind = geometry.kind;
    else if (key in geometry) out[key] = geometry[key];
    else if (!old.has(key)) out[key] = key === 'attrs' ? { ...e.attrs } : src[key];
  }
  for (const key of Object.keys(geometry)) if (!(key in out)) out[key] = geometry[key];
  return out as unknown as NewEntity;
}

/**
 * What a replacement or a new piece takes from its object: the layer and the
 * colour, the attributes and the label with `keepData`; not the symbol (the
 * tools' `inherit`).
 */
function inherited(e: Entity, g: EntityGeometry, keepData: boolean): NewEntity {
  return { ...geometryOf(g), layerId: e.layerId, color: e.color, attrs: keepData ? { ...e.attrs } : {}, label: keepData ? e.label : undefined } as unknown as NewEntity;
}

/**
 * The `i`-th geometry of the input's `list` (`changes`; `objects` of
 * `cad.entities.create`): enough points for its kind, every number finite, a
 * positive radius. `whose` names it in a message: “değişikliğin”.
 */
export function checkGeometry(g: EntityGeometry, i: number, list = 'changes', whose = 'değişikliğin'): Stop | null {
  const at = (field: string) => `${list}[${i}].geometry${field}`;
  if (g.kind === 'polyline' && g.pts.length < 2)
    return failed(error('too_few_points', `Çoklu çizginin en az 2 noktası olmalı; ${g.pts.length} nokta verildi. Eksik noktaları ekleyin.`, at('.pts')));
  if (g.kind === 'polygon') {
    if (g.pts.length < 3) return failed(error('too_few_corners', `Kapalı alanın en az 3 köşesi olmalı; ${g.pts.length} köşe verildi. Eksik köşeleri ekleyin.`, at('.pts')));
    for (const [h, ring] of (g.holes ?? []).entries())
      if (ring.pts.length < 3)
        return failed(error('too_few_corners', `${h + 1}. deliğin en az 3 köşesi olmalı; ${ring.pts.length} köşe verildi. Eksik köşeleri ekleyin ya da deliği çıkarın.`, at(`.holes[${h}].pts`)));
  }
  if (g.kind === 'hatch') {
    if (g.ring.length < 3) return failed(error('too_few_corners', `Taramanın en az 3 köşesi olmalı; ${g.ring.length} köşe verildi. Eksik köşeleri ekleyin.`, at('.ring')));
    for (const [h, hole] of (g.holes ?? []).entries())
      if (hole.length < 3)
        return failed(error('too_few_corners', `${h + 1}. deliğin en az 3 köşesi olmalı; ${hole.length} köşe verildi. Eksik köşeleri ekleyin ya da deliği çıkarın.`, at(`.holes[${h}]`)));
  }
  if (!geometryIsFinite(g as unknown as Entity))
    return failed(error('not_finite', `${i + 1}. ${whose} geometrisinde sonlu olmayan bir değer var (NaN ya da sonsuz). Geometriyi sonlu sayılarla verin.`, at('')));
  if ((g.kind === 'circle' || g.kind === 'arc') && !(g.r > 0)) return failed(error('invalid_radius', 'Yarıçap sıfırdan büyük olmalı. Pozitif bir yarıçap verin.', at('.r')));
  return null;
}

/** The checks in the contract's order: why nothing may be written, or what may. */
function check(doc: CadDocument, input: EntitiesEdit): Stop | Checked {
  if (!input.changes.length) return failed(error('no_changes', 'Yapılacak değişiklik verilmedi. En az bir değişiklik verin.', 'changes'));
  for (const [i, c] of input.changes.entries()) {
    const [uid, field] = named(c);
    if (!isUuid(uid))
      return failed(
        error(
          'invalid_uid',
          `“${uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın.`,
          `changes[${i}].${field}`,
        ),
      );
  }
  for (const [i, c] of input.changes.entries()) {
    const stop = c.kind === 'remove' ? null : checkGeometry(c.geometry, i);
    if (stop) return stop;
  }
  const stop = checkRevision(doc, input.expectedRevision);
  if (stop) return stop;
  const found: Entity[] = [];
  for (const [i, c] of input.changes.entries()) {
    const [uid, field] = named(c);
    const e = doc.byUid(uid);
    if (!e)
      return failed(error('entity_not_found', `“${uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.`, `changes[${i}].${field}`));
    found.push(e);
  }
  // An object changes once: an `add` may come from one that changes.
  const targets = new Set<number>();
  for (const [i, c] of input.changes.entries()) {
    if (c.kind === 'add') continue;
    if (targets.has(found[i].id))
      return failed(error('repeated_entity', `“${c.uid}” kimlikli nesne birden çok değişiklikte değişiyor. Bir nesneye tek değişiklik verin.`, `changes[${i}].uid`));
    targets.add(found[i].id);
  }
  // Nothing on a locked layer is changed or copied from.
  for (const [i, c] of input.changes.entries()) {
    const layerId = found[i].layerId;
    if (!doc.layers.isLocked(layerId)) continue;
    const name = doc.layers.get(layerId)?.name ?? layerId;
    return failed(error('layer_locked', `“${name}” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın.`, `changes[${i}].${named(c)[1]}`));
  }
  const checked: Checked = { changed: [], created: [], removed: [] };
  for (const [i, c] of input.changes.entries()) {
    const e = found[i];
    if (c.kind === 'update') checked.changed.push({ id: e.id, uid: c.uid, init: reshaped(e, c.geometry) });
    else if (c.kind === 'replace') checked.changed.push({ id: e.id, uid: c.uid, init: inherited(e, c.geometry, c.keepData === true) });
    else if (c.kind === 'add') checked.created.push(inherited(e, c.geometry, c.keepData === true));
    else checked.removed.push({ id: e.id, uid: c.uid });
  }
  return checked;
}

const isStop = (c: Stop | Checked): c is Stop => 'status' in c;

/** An object as the plan shows it, without undefined fields: its slot, or 0 for a new one. */
const planned = (init: NewEntity, id: number): PlannedEntity => JSON.parse(JSON.stringify({ ...init, id })) as PlannedEntity;

export const entitiesEdit: ProductCommand<EntitiesEdit, EntitiesEdited, EntitiesEditPlan> = {
  id: 'cad.entities.edit',
  version: 1,

  validate(cx, input) {
    const checked = check(cx.doc, input);
    return validated(isStop(checked) ? checked : []);
  },

  plan(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    return {
      status: 'completed',
      output: {
        changed: checked.changed.map((c) => planned(c.init, c.id)),
        created: checked.created.map((init) => planned(init, 0)),
        removed: checked.removed.map((r) => r.uid),
        revision: String(cx.doc.revision),
      },
      warnings: [],
    };
  },

  /**
   * Writes one undo step named after the tool, through the document's own
   * edits: the deletions, the objects changed in their places (`replace`:
   * slot and persistent id kept), then the new objects (`addMany`). Into the
   * open transaction or group, if one is.
   */
  execute(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    const { doc } = cx;
    const label = EDIT_LABEL[input.operation];
    const created = doc.transact(label, () => {
      doc.remove(checked.removed.map((r) => r.id));
      for (const c of checked.changed) doc.replace(c.id, c.init, label);
      return doc.addMany(checked.created, label).map((e) => e.uid);
    });
    return {
      status: 'completed',
      output: { changed: checked.changed.map((c) => c.uid), created, removed: checked.removed.map((r) => r.uid), revision: String(doc.revision) },
      warnings: [],
    };
  },
};
