import type { ArrayLayout } from '../contracts/generated/ArrayLayout';
import type { CommandWarning } from '../contracts/generated/CommandWarning';
import type { EntitiesArray } from '../contracts/generated/EntitiesArray';
import type { EntitiesArrayed } from '../contracts/generated/EntitiesArrayed';
import type { EntitiesArrayPlan } from '../contracts/generated/EntitiesArrayPlan';
import type { Entity as PlannedEntity } from '../contracts/generated/Entity';
import type { CadDocument } from '../model/document';
import type { Entity, NewEntity } from '../model/entities';
import { withoutLink } from '../model/linkedTexts';
import { pathOf } from '../model/ops/path';
import { arrayCopies, geometryIsFinite, pathArrayTransforms } from '../model/ops/transform';
import { isUuid } from '../core/uuid';
import type { Affine } from '../model/geom/affine';
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
export const arrayLabel = (layout: ArrayLayout): string => (layout.kind === 'grid' ? 'Dizi' : layout.kind === 'polar' ? 'Kutupsal dizi' : 'Yol boyunca dizi');

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
  if (l.kind === 'path') {
    if (!isUuid(l.path))
      return refuse(
        'invalid_uid',
        `“${l.path}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın.`,
        'layout.path',
      );
    const spacing = l.spacing ?? null;
    if (spacing !== null) {
      const bad = notFiniteValue(spacing, 'Aralık', 'Aralığı sonlu bir sayıyla verin.', 'layout.spacing');
      if (bad) return bad;
    }
    if (!Number.isInteger(l.count) || l.count < 2 || l.count > 10_000)
      return refuse('invalid_count', 'Adet 2 ile 10 000 arasında bir tam sayı olmalı. Başka bir adet verin.', 'layout.count');
    if (spacing !== null && spacing < NONE)
      return refuse('invalid_spacing', 'Aralık sıfırdan büyük olmalı. Bir aralık verin ya da kopyaları yola eşit dağıtmak için aralığı boş bırakın.', 'layout.spacing');
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
  const maps = pathMaps(doc, input.layout);
  if (maps && 'status' in maps) return maps;
  if (!originals.length) return failed(error('layer_locked', lockedMessage(locked.length), 'uids'));
  const copies = arrayCopies(originals, input.layout, doc.settings.drawingFont.value, maps ?? undefined);
  if (copies.some((c, i) => geometryIsFinite(originals[i % originals.length]) && !geometryIsFinite(c)))
    return failed(error('not_finite', 'Dönüşüm sonucunda sonlu olmayan bir değer çıktı (sayı taşması). Daha küçük bir değer verin.', 'layout'));
  const warnings: CommandWarning[] = locked.length ? [{ code: 'layer_locked', message: lockedMessage(locked.length), path: 'uids' }] : [];
  return { sources, copies, locked, warnings };
}

/**
 * A path array's maps from its path in the document: `entity_not_found`, `invalid_path` and `invalid_spacing`
 * (places past its end) as the contract orders them; null for the other layouts.
 */
function pathMaps(doc: CadDocument, layout: ArrayLayout): Stop | Affine[] | null {
  if (layout.kind !== 'path') return null;
  const path = doc.byUid(layout.path);
  if (!path)
    return failed(
      error('entity_not_found', `“${layout.path}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.`, 'layout.path'),
    );
  const follows = path.kind === 'line' || path.kind === 'arc' || path.kind === 'circle' || path.kind === 'polyline';
  const length = follows ? (pathOf(path)?.length ?? 0) : 0;
  if (!(length > NONE))
    return refuse('invalid_path', 'Yol bir çizgi, yay, daire ya da çoklu çizgi olmalı ve bir uzunluğu olmalı. Başka bir nesneyi yol olarak seçin.', 'layout.path');
  const maps = pathArrayTransforms(path, layout.count, layout.spacing ?? null, layout.align);
  if (!maps)
    return refuse(
      'invalid_spacing',
      'Bu aralıkla bu kadar kopya yola sığmıyor: yerler yolun sonunu (kapalı yolda başını) geçiyor. Daha küçük bir aralık ya da adet verin.',
      'layout.spacing',
    );
  return maps;
}

const isStop = (c: Stop | Checked): c is Stop => 'status' in c;

/** A copy as it is added: without its original's slot and persistent id; a linked text's writes no object's label (docs/adr/0175 §4). */
function added(e: Entity): NewEntity {
  const { id: _id, uid: _uid, ...rest } = withoutLink(e) as Entity & { uid?: string };
  return rest as NewEntity;
}

/** A copy as the plan shows it: slot 0, given when it is written, and no persistent id or link. */
function planned(e: Entity): PlannedEntity {
  const { uid: _uid, ...rest } = withoutLink(e) as Entity & { uid?: string };
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
