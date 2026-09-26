import type { CommandWarning } from '../contracts/generated/CommandWarning';
import type { EntitiesTransform } from '../contracts/generated/EntitiesTransform';
import type { EntitiesTransformed } from '../contracts/generated/EntitiesTransformed';
import type { EntitiesTransformPlan } from '../contracts/generated/EntitiesTransformPlan';
import type { Entity as PlannedEntity } from '../contracts/generated/Entity';
import type { Transform } from '../contracts/generated/Transform';
import type { CadDocument } from '../model/document';
import type { Entity, NewEntity } from '../model/entities';
import { geometryIsFinite, transformObjects } from '../model/ops/transform';
import { checkRevision, checkUids, error, failed, findObjects, notFinite, notFiniteValue, validated, type Stop } from './checks';
import type { ProductCommand } from './command';

/**
 * `cad.entities.transform` v1 (docs/adr/0037): objects named by their
 * persistent ids moved, rotated, scaled or mirrored as one undo step, in
 * place or as copies. The web's handler over `CadDocument`; the desktop's is
 * `crates/native/application/src/transform.rs`. Both pass the shared cases
 * in fixtures/commands/v1/cad.entities.transform.json.
 *
 * The modify tools (Taşı, Kopyala, Döndür, Ölçekle, Aynala) make the
 * selection explicit here (TODOS.md CMD-07). The geometry is the shared
 * core's, as on the desktop: the core builds the matrix from the
 * transform's numbers and moves every kind of object, packed, with no JSON
 * (`transformObjects`); nothing is computed here.
 *
 * The checks, in order (the first that fails answers): at least one id,
 * each lowercase UUID text with hyphens; the transform's numbers finite in
 * their order, a scale factor above zero, a mirror axis with a direction;
 * the expected revision (checks.ts); each id names an object; not every
 * object on a locked layer; no coordinate carried past the largest float64.
 * A repeated id counts once.
 *
 * Objects on a locked layer are neither changed nor copied. The tools used
 * to copy them onto their locked layer; ADR 0037 records the change.
 */

interface Checked {
  /** The objects to transform, with their ids, in the input's order. */
  sources: { entity: Entity; uid: string }[];
  /** Each of them as it would be written, in the same order (slot and persistent id its own). */
  moved: Entity[];
  /** The ids left alone on locked layers. */
  locked: string[];
  warnings: CommandWarning[];
}

const lockedMessage = (n: number) => `${n} nesne kilitli katmanda olduğu için atlandı. Değiştirmek için katmanın kilidini Katmanlar panelinden açın.`;

/** The undo step's name: the tool's (docs/adr/0037). */
export function transformLabel(t: Transform, copy: boolean): string {
  switch (t.kind) {
    case 'move':
      return copy ? 'Kopyala' : 'Taşı';
    case 'rotate':
      return 'Döndür';
    case 'scale':
      return 'Ölçekle';
    case 'mirror':
      return 'Aynala';
  }
}

/** The transform's own checks, in its fields' order: finite numbers, then what they mean. */
function checkTransform(t: Transform): Stop | null {
  switch (t.kind) {
    case 'move': {
      const fix = 'Kaydırmayı sonlu bir sayıyla verin.';
      return notFiniteValue(t.dx, 'Doğu (Y) yönündeki kaydırma', fix, 'transform.dx') ?? notFiniteValue(t.dy, 'Kuzey (X) yönündeki kaydırma', fix, 'transform.dy');
    }
    case 'rotate':
      return notFinite(t.center, 'Merkezin', 'transform.center') ?? notFiniteValue(t.angle, 'Dönme açısı', 'Açıyı sonlu bir sayıyla verin.', 'transform.angle');
    case 'scale':
      return (
        notFinite(t.center, 'Merkezin', 'transform.center') ??
        notFiniteValue(t.factor, 'Ölçek faktörü', 'Faktörü sonlu bir sayıyla verin.', 'transform.factor') ??
        (t.factor > 0 ? null : failed(error('invalid_factor', 'Ölçek faktörü sıfırdan büyük olmalı. Pozitif bir faktör verin.', 'transform.factor')))
      );
    case 'mirror': {
      const stop = notFinite(t.a, 'Eksenin ilk noktasının', 'transform.a') ?? notFinite(t.b, 'Eksenin ikinci noktasının', 'transform.b');
      if (stop) return stop;
      // The core's own measure of the axis (`mirror`): zero leaves it no direction.
      const dx = t.b.x - t.a.x;
      const dy = t.b.y - t.a.y;
      return dx * dx + dy * dy === 0 ? failed(error('invalid_axis', 'Simetri ekseninin iki noktası aynı; eksenin yönü yok. Birbirinden ayrı iki nokta verin.', 'transform.b')) : null;
    }
  }
}

/** The checks in the contract's order: why nothing may be written, or what may. */
function check(doc: CadDocument, input: EntitiesTransform): Stop | Checked {
  const stop = checkUids(input.uids, 'Dönüştürülecek nesne verilmedi.') ?? checkTransform(input.transform) ?? checkRevision(doc, input.expectedRevision);
  if (stop) return stop;
  const found = findObjects(doc, input.uids);
  if ('status' in found) return found;
  const sources: Checked['sources'] = [];
  const locked: string[] = [];
  for (const f of found) {
    if (doc.layers.isLocked(f.entity.layerId)) locked.push(f.uid);
    else sources.push(f);
  }
  if (!sources.length) return failed(error('layer_locked', lockedMessage(locked.length), 'uids'));
  const moved = transformObjects(
    sources.map((s) => s.entity),
    input.transform,
  );
  if (sources.some((s, i) => geometryIsFinite(s.entity) && !geometryIsFinite(moved[i])))
    return failed(error('not_finite', 'Dönüşüm sonucunda sonlu olmayan bir değer çıktı (sayı taşması). Daha küçük bir değer verin.', 'transform'));
  const warnings: CommandWarning[] = locked.length ? [{ code: 'layer_locked', message: lockedMessage(locked.length), path: 'uids' }] : [];
  return { sources, moved, locked, warnings };
}

const isStop = (c: Stop | Checked): c is Stop => 'status' in c;

/** An object as the plan shows it: without its persistent id; a copy's slot is 0, given when it is written. */
function planned(e: Entity, copy: boolean): PlannedEntity {
  const { uid: _uid, ...rest } = e as Entity & { uid?: string };
  return (copy ? { ...rest, id: 0 } : rest) as unknown as PlannedEntity;
}

export const entitiesTransform: ProductCommand<EntitiesTransform, EntitiesTransformed, EntitiesTransformPlan> = {
  id: 'cad.entities.transform',
  version: 1,

  validate(cx, input) {
    const checked = check(cx.doc, input);
    return validated(isStop(checked) ? checked : checked.warnings);
  },

  plan(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    const copy = input.copy === true;
    return {
      status: 'completed',
      output: {
        sources: checked.sources.map((s) => s.uid),
        entities: checked.moved.map((e) => planned(e, copy)),
        locked: checked.locked,
        revision: String(cx.doc.revision),
      },
      warnings: checked.warnings,
    };
  },

  /**
   * Writes one undo step named after the tool, through the document's own
   * edits: the objects changed in place (`updateMany`), or their copies
   * added (`addMany`, new persistent ids). Into the open transaction or group, if one is.
   */
  execute(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    const copy = input.copy === true;
    const label = transformLabel(input.transform, copy);
    let changed: string[] = [];
    let created: string[] = [];
    if (copy) {
      const copies = checked.moved.map((e) => {
        const { id: _id, uid: _uid, ...rest } = e as Entity & { uid?: string };
        return rest as NewEntity;
      });
      created = cx.doc.addMany(copies, label).map((e) => e.uid);
    } else {
      cx.doc.updateMany(checked.moved, label);
      changed = checked.sources.map((s) => s.uid);
    }
    return {
      status: 'completed',
      output: { changed, created, locked: checked.locked, revision: String(cx.doc.revision) },
      warnings: checked.warnings,
    };
  },
};
