import type { ArrayLayout } from '../contracts/generated/ArrayLayout';
import type { CommandWarning } from '../contracts/generated/CommandWarning';
import type { EntitiesArray } from '../contracts/generated/EntitiesArray';
import type { EntitiesArrayed } from '../contracts/generated/EntitiesArrayed';
import type { EntitiesArrayPlan } from '../contracts/generated/EntitiesArrayPlan';
import type { Entity as PlannedEntity } from '../contracts/generated/Entity';
import type { CadDocument } from '../model/document';
import type { Entity, NewEntity } from '../model/entities';
import { arrayCopies, geometryIsFinite } from '../model/ops/transform';
import { checkRevision, checkUids, error, failed, findObjects, notFinite, notFiniteValue, validated, type Stop } from './checks';
import type { ProductCommand } from './command';

/**
 * `cad.entities.array` v1 (docs/adr/0047): copies of objects named by their
 * persistent ids laid out in rows and columns, or around a centre, as one
 * undo step. The web's handler over `CadDocument`; the desktop's is
 * `crates/native/application/src/array.rs`. Both pass the shared cases in
 * fixtures/commands/v1/cad.entities.array.json.
 *
 * The array tools (Dizi, Kutupsal dizi) make the selection explicit here
 * (TODOS.md CMD-07). The layout is the shared core's, as on the desktop:
 * the core lays the copies out from the layout's numbers (a polar array's
 * middle measured in the drawing's typeface) and moves every kind of
 * object, packed, with no JSON (`arrayCopies`); nothing is computed here.
 *
 * The checks, in order (the first that fails answers): at least one id,
 * each lowercase UUID text with hyphens; the layout's numbers finite in
 * their order, the counts in their ranges (whole numbers), a spacing for
 * each grid direction with more than one place, a polar fill that is not
 * zero and not past a full turn; the expected revision (checks.ts); each id
 * names an object; not every object on a locked layer; no copy carried past
 * the largest float64. A repeated id counts once.
 *
 * Objects on a locked layer are not copied (docs/adr/0037). The tools used
 * to copy them onto their locked layer; ADR 0047 records the change.
 */

/** Less than this, a spacing or a fill angle is none (the tools took a nanometre, or a nano-degree, as zero). */
const NONE = 1e-9;

interface Checked {
  /** The ids of the objects copied, in the input's order. */
  sources: string[];
  /** The copies as they would be written, place after place (each its original's slot until written). */
  copies: Entity[];
  /** The ids not copied on locked layers. */
  locked: string[];
  warnings: CommandWarning[];
}

const lockedMessage = (n: number) => `${n} nesne kilitli katmanda olduğu için atlandı. Kopyalamak için katmanın kilidini Katmanlar panelinden açın.`;

/** The undo step's name: the tool's (docs/adr/0047). */
export const arrayLabel = (layout: ArrayLayout): string => (layout.kind === 'grid' ? 'Dizi' : 'Kutupsal dizi');

const refuse = (code: string, message: string, path: string): Stop => failed(error(code, message, path));

/** The layout's own checks, in its fields' order: finite numbers, then the counts, then the spacing or the fill. */
function checkLayout(l: ArrayLayout): Stop | null {
  if (l.kind === 'grid') {
    const fix = 'Aralığı sonlu bir sayıyla verin.';
    const stop = notFiniteValue(l.dx, 'Doğu (Y) yönündeki aralık', fix, 'layout.dx') ?? notFiniteValue(l.dy, 'Kuzey (X) yönündeki aralık', fix, 'layout.dy');
    if (stop) return stop;
    const count = "Satır × sütun 2 ile 10 000 arasında olmalı; satır ve sütun en az 1'dir. Başka bir satır ve sütun sayısı verin.";
    if (!Number.isInteger(l.rows) || l.rows < 1) return refuse('invalid_count', count, 'layout.rows');
    if (!Number.isInteger(l.cols) || l.cols < 1) return refuse('invalid_count', count, 'layout.cols');
    const places = l.rows * l.cols;
    if (places < 2 || places > 10_000) return refuse('invalid_count', count, 'layout');
    if (l.cols > 1 && Math.abs(l.dx) < NONE)
      return refuse('invalid_spacing', 'Sütunlar arasındaki aralık (dY) sıfır; kopyalar üst üste düşer. Sıfırdan farklı bir aralık verin.', 'layout.dx');
    if (l.rows > 1 && Math.abs(l.dy) < NONE)
      return refuse('invalid_spacing', 'Satırlar arasındaki aralık (dX) sıfır; kopyalar üst üste düşer. Sıfırdan farklı bir aralık verin.', 'layout.dy');
    return null;
  }
  const stop = notFinite(l.center, 'Merkezin', 'layout.center') ?? notFiniteValue(l.fill, 'Doldurma açısı', 'Açıyı sonlu bir sayıyla verin.', 'layout.fill');
  if (stop) return stop;
  if (!Number.isInteger(l.count) || l.count < 2 || l.count > 1000)
    return refuse('invalid_count', 'Adet 2 ile 1000 arasında bir tam sayı olmalı. Başka bir adet verin.', 'layout.count');
  if (Math.abs(l.fill) < NONE || Math.abs(l.fill) > 360)
    return refuse('invalid_fill', 'Doldurma açısı 0 ile ±360 derece arasında olmalı; 0 olamaz. Başka bir açı verin.', 'layout.fill');
  return null;
}

/** The checks in the contract's order: why nothing may be written, or what may. */
function check(doc: CadDocument, input: EntitiesArray): Stop | Checked {
  const stop = checkUids(input.uids, 'Diziye alınacak nesne verilmedi.') ?? checkLayout(input.layout) ?? checkRevision(doc, input.expectedRevision);
  if (stop) return stop;
  const found = findObjects(doc, input.uids);
  if ('status' in found) return found;
  const originals: Entity[] = [];
  const sources: string[] = [];
  const locked: string[] = [];
  for (const f of found) {
    if (doc.layers.isLocked(f.entity.layerId)) locked.push(f.uid);
    else {
      originals.push(f.entity);
      sources.push(f.uid);
    }
  }
  if (!originals.length) return failed(error('layer_locked', lockedMessage(locked.length), 'uids'));
  const copies = arrayCopies(originals, input.layout, doc.settings.drawingFont.value);
  if (copies.some((c, i) => geometryIsFinite(originals[i % originals.length]) && !geometryIsFinite(c)))
    return failed(error('not_finite', 'Dönüşüm sonucunda sonlu olmayan bir değer çıktı (sayı taşması). Daha küçük bir değer verin.', 'layout'));
  const warnings: CommandWarning[] = locked.length ? [{ code: 'layer_locked', message: lockedMessage(locked.length), path: 'uids' }] : [];
  return { sources, copies, locked, warnings };
}

const isStop = (c: Stop | Checked): c is Stop => 'status' in c;

/** A copy as it is added: without its original's slot and persistent id. */
function added(e: Entity): NewEntity {
  const { id: _id, uid: _uid, ...rest } = e as Entity & { uid?: string };
  return rest as NewEntity;
}

/** A copy as the plan shows it: slot 0, given when it is written, and no persistent id. */
function planned(e: Entity): PlannedEntity {
  const { uid: _uid, ...rest } = e as Entity & { uid?: string };
  return { ...rest, id: 0 } as unknown as PlannedEntity;
}

export const entitiesArray: ProductCommand<EntitiesArray, EntitiesArrayed, EntitiesArrayPlan> = {
  id: 'cad.entities.array',
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
      output: { sources: checked.sources, entities: checked.copies.map(planned), locked: checked.locked, revision: String(cx.doc.revision) },
      warnings: checked.warnings,
    };
  },

  /**
   * Adds the copies as one undo step named after the tool (`addMany`, new
   * persistent ids), into the open transaction or group if one is.
   */
  execute(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    const created = cx.doc.addMany(checked.copies.map(added), arrayLabel(input.layout)).map((e) => e.uid);
    return {
      status: 'completed',
      output: { created, locked: checked.locked, revision: String(cx.doc.revision) },
      warnings: checked.warnings,
    };
  },
};
