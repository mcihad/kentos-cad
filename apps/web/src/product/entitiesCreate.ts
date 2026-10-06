import type { CommandWarning } from '../contracts/generated/CommandWarning';
import type { CreateOperation } from '../contracts/generated/CreateOperation';
import type { EntitiesCreate } from '../contracts/generated/EntitiesCreate';
import type { EntitiesCreated } from '../contracts/generated/EntitiesCreated';
import type { EntitiesCreatePlan } from '../contracts/generated/EntitiesCreatePlan';
import type { Entity as PlannedEntity } from '../contracts/generated/Entity';
import type { NewObject } from '../contracts/generated/NewObject';
import { isUuid } from '../core/uuid';
import type { CadDocument } from '../model/document';
import type { NewEntity } from '../model/entities';
import { checkLineWeight, checkLayer, checkRevision, error, failed, validated, type Stop } from './checks';
import type { ProductCommand } from './command';
import { checkBlocks, checkGeometry, checkStyles, geometryOf } from './entitiesEdit';

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
 * for its kind, every number finite, a positive radius), its line weight
 * from 0 to 100 mm when given (docs/adr/0139) and its link to the object
 * whose label it writes, when given: a text's, whole and well formed
 * (docs/adr/0175 §4); the expected revision, then the layer (checks.ts, for
 * the reasons polygonCreate.ts gives); every insert's block, then every
 * linked text's object, the drawing's.
 */

/** The undo step's name: the drawing tool's when it has its own, else the document's “Ekle”. */
export const CREATE_LABEL: Record<CreateOperation, string> = {
  parallel: 'Paralel çizgi',
  perpendicularIn: 'Dik in',
  perpendicularOut: 'Dik çık',
  divide: 'Böl',
  hatch: 'Tarama',
  boundary: 'Alan oluştur',
  traverse: 'Poligon hesabı',
  polarSurvey: 'Kutupsal alım',
  forwardIntersection: 'Önden kestirme',
  resection: 'Geriden kestirme',
  pointsBetween: 'Ara nokta',
  intersectPoint: 'Kesişim noktası',
  dimensionChain: 'Zincir ölçü',
  dimensionBaseline: 'Baz ölçü',
  textFile: 'Metin dosyası yerleştir',
  leader: 'Kılavuz',
  polygonize: 'Toplu alan',
  vertexPoints: 'Köşelere nokta',
  adjoin: 'Bitişik alan',
  labels: 'Etiketleri yazıya çevir',
  // Tablo ekle (docs/adr/0184 §6).
  table: 'Tablo',
  // Koordinat yaz (docs/adr/0185 §1).
  coordinates: 'Koordinat yaz',
};

/**
 * A new object's link to the object whose label it writes, when it has one (docs/adr/0175 §4): a text's, both
 * fields, the id a persistent id's text, the scale finite and over 0 (`invalid_link`).
 */
function checkLink(o: NewObject, i: number): Stop | null {
  const at = (field: string) => `objects[${i}].${field}`;
  const of = o.labelOf ?? undefined;
  const scale = o.labelScale ?? undefined;
  if (of === undefined && scale === undefined) return null;
  if (o.geometry.kind !== 'text')
    return failed(error('invalid_link', `Yalnız yazı bir nesnenin etiketine bağlanır; ${i + 1}. nesne yazı değil. Bağı kaldırın.`, at(of !== undefined ? 'labelOf' : 'labelScale')));
  if (of === undefined || scale === undefined)
    return failed(error('invalid_link', 'Bağlı yazının nesnesi ve ölçeği birlikte verilir. İkisini birden verin ya da hiçbirini vermeyin.', at(of === undefined ? 'labelOf' : 'labelScale')));
  if (!isUuid(of)) return failed(error('invalid_link', `Bağlı nesnenin kimliği küçük harfli, tireli bir UUID olmalı; “${of}” verildi.`, at('labelOf')));
  if (!(Number.isFinite(scale) && scale > 0))
    return failed(error('invalid_link', "Bağlı yazının ölçeği (1:N'deki N) sonlu ve sıfırdan büyük olmalı. Ölçeği düzeltin.", at('labelScale')));
  return null;
}

/** The checks in the contract's order: why nothing may be written, or the warnings when it may. */
function check(doc: CadDocument, input: EntitiesCreate): Stop | CommandWarning[] {
  if (!input.objects.length) return failed(error('no_objects', 'Eklenecek nesne verilmedi. En az bir nesne verin.', 'objects'));
  for (const [i, o] of input.objects.entries()) {
    const stop = checkGeometry(o.geometry, i, 'objects', 'nesnenin') ?? checkLineWeight(o.lineWeight, `objects[${i}].lineWeight`) ?? checkLink(o, i);
    if (stop) return stop;
  }
  const stop = checkRevision(doc, input.expectedRevision);
  if (stop) return stop;
  const layer = checkLayer(doc, input.layerId);
  if (!Array.isArray(layer)) return layer;
  // An insert's block is the drawing's (docs/adr/0144).
  const blocks = checkBlocks(
    doc,
    input.objects.map((o) => o.geometry),
    'objects',
  );
  if (blocks) return blocks;
  // A text's and a dimension's style is the project's (docs/adr/0183 §9).
  const styles = checkStyles(
    doc,
    input.objects.map((o) => o.geometry),
    'objects',
  );
  if (styles) return styles;
  // A linked text's object is the drawing's (docs/adr/0175 §4).
  for (const [i, o] of input.objects.entries())
    if (o.labelOf != null && !doc.byUid(o.labelOf))
      return failed(
        error('link_not_found', `“${o.labelOf}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Çizimdeki bir nesnenin kimliğini verin.`, `objects[${i}].labelOf`),
      );
  return layer;
}

/** An object as the document stores it: its own copies of every field, never the caller's objects. */
function entityOf(o: NewObject, layerId: string): NewEntity {
  return {
    ...geometryOf(o.geometry),
    layerId,
    ...(o.color != null && { color: o.color }),
    ...(o.lineWeight != null && { lineWeight: o.lineWeight }),
    attrs: { ...o.attrs },
    ...(o.label != null && { label: o.label }),
    ...(o.symbol != null && { symbol: o.symbol }),
    // A linked text knows its object (docs/adr/0175 §4).
    ...(o.geometry.kind === 'text' && o.labelOf != null && { labelOf: o.labelOf, labelScale: o.labelScale }),
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
