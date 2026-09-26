import type { CommandWarning } from '../contracts/generated/CommandWarning';
import type { PolylineCreate } from '../contracts/generated/PolylineCreate';
import type { PolylineCreated } from '../contracts/generated/PolylineCreated';
import type { PolylinePlan } from '../contracts/generated/PolylinePlan';
import type { CadDocument } from '../model/document';
import type { NewEntity } from '../model/entities';
import { checkLayer, checkRevision, copy, error, failed, notFinite, validated, type Stop } from './checks';
import type { ProductCommand } from './command';

/**
 * `cad.polyline.create` v1 (docs/adr/0027): one open polyline on a named
 * layer, as one undo step. The web's handler over `CadDocument`; the
 * desktop's is `crates/native/application/src/polyline.rs`. Both pass the
 * shared cases in fixtures/commands/v1/cad.polyline.create.json.
 *
 * The checks, in order (the first that fails answers): at least 2 points,
 * finite coordinates (x before y), one bulge per edge when bulges are given
 * (one fewer than the points), finite bulges; the expected revision; the
 * layer (checks.ts, for the reasons polygonCreate.ts gives).
 *
 * Bulges are given one per edge; the document keeps one per point, as DXF's
 * LWPOLYLINE does, so the polyline is stored with a 0 after them for the
 * closing edge it does not have: what the polyline tool always wrote.
 * Otherwise the input is stored as given (zero bulges too).
 */

const MIN_POINTS = 2;

/** The checks in the contract's order: why nothing may be written, or the warnings when it may. */
function check(doc: CadDocument, input: PolylineCreate): Stop | CommandWarning[] {
  const n = input.pts.length;
  if (n < MIN_POINTS) return failed(error('too_few_points', `Çoklu çizginin en az 2 noktası olmalı; ${n} nokta verildi. Eksik noktaları ekleyin.`, 'pts'));
  for (let i = 0; i < n; i++) {
    const broken = notFinite(input.pts[i], `${i + 1}. noktanın`, `pts[${i}]`);
    if (broken) return broken;
  }
  const bulges = input.bulges;
  if (bulges != null) {
    const edges = n - 1;
    if (bulges.length !== edges)
      return failed(
        error(
          'bulge_count',
          `Yay değerleri kenar sayısı kadar olmalı, her kenara bir değer: ${n} nokta, ${edges} kenar, ${bulges.length} yay değeri verildi. Eksik ya da fazla değerleri düzeltin.`,
          'bulges',
        ),
      );
    for (let i = 0; i < edges; i++)
      if (!Number.isFinite(bulges[i]))
        return failed(error('not_finite', `${i + 1}. kenarın yay değeri sonlu bir sayı değil (NaN ya da sonsuz). Düz kenar için 0, yay için tan(açı/4) verin.`, `bulges[${i}]`));
  }
  return checkRevision(doc, input.expectedRevision) ?? checkLayer(doc, input.layerId);
}

/** The polyline `input` describes, as the document stores it (bulges one per point): its own copies. */
function polylineOf(input: PolylineCreate): NewEntity & { kind: 'polyline' } {
  return {
    kind: 'polyline',
    pts: input.pts.map(copy),
    ...(input.bulges != null && { bulges: [...input.bulges, 0] }),
    layerId: input.layerId,
    ...(input.color != null && { color: input.color }),
    attrs: { ...input.attrs },
  };
}

export const polylineCreate: ProductCommand<PolylineCreate, PolylineCreated, PolylinePlan> = {
  id: 'cad.polyline.create',
  version: 1,

  validate(cx, input) {
    return validated(check(cx.doc, input));
  },

  plan(cx, input) {
    const checked = check(cx.doc, input);
    if (!Array.isArray(checked)) return checked;
    // The slot is given when it is written: 0 until then.
    return { status: 'completed', output: { entity: { ...polylineOf(input), id: 0 }, revision: String(cx.doc.revision) }, warnings: checked };
  },

  /** Writes through the document's own `add`: undo step “Ekle”, as the polyline tool always wrote it; into the open transaction or group, if one is. */
  execute(cx, input) {
    const checked = check(cx.doc, input);
    if (!Array.isArray(checked)) return checked;
    const e = cx.doc.add(polylineOf(input));
    return { status: 'completed', output: { uid: e.uid, id: e.id, revision: String(cx.doc.revision) }, warnings: checked };
  },
};
