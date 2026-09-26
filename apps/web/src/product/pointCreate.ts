import type { CommandWarning } from '../contracts/generated/CommandWarning';
import type { PointCreate } from '../contracts/generated/PointCreate';
import type { PointCreated } from '../contracts/generated/PointCreated';
import type { PointPlan } from '../contracts/generated/PointPlan';
import type { CadDocument } from '../model/document';
import type { NewEntity } from '../model/entities';
import { checkLayer, checkRevision, copy, notFinite, notFiniteValue, validated, type Stop } from './checks';
import type { ProductCommand } from './command';

/**
 * `cad.point.create` v1 (docs/adr/0032): one point object on a named layer,
 * as one undo step. The web's handler over `CadDocument`; the desktop's is
 * `crates/native/application/src/point.rs`. Both pass the shared cases in
 * fixtures/commands/v1/cad.point.create.json.
 *
 * The point tool calls it once per point it places; the spot elevation tool
 * gives the elevation and the text beside the point too.
 *
 * The checks, in order (the first that fails answers): the point finite (x
 * before y), then the elevation when given; the expected revision; the
 * layer (checks.ts, for the reasons polygonCreate.ts gives).
 */

/** The checks in the contract's order: why nothing may be written, or the warnings when it may. */
function check(doc: CadDocument, input: PointCreate): Stop | CommandWarning[] {
  return (
    notFinite(input.p, 'Noktanın', 'p') ??
    (input.z != null ? notFiniteValue(input.z, 'Kot', 'Kotu sonlu bir sayıyla verin.', 'z') : null) ??
    checkRevision(doc, input.expectedRevision) ??
    checkLayer(doc, input.layerId)
  );
}

/** The point `input` describes, as the document stores it: its own copies, never the caller's objects. */
function pointOf(input: PointCreate): NewEntity & { kind: 'point' } {
  return {
    kind: 'point',
    p: copy(input.p),
    ...(input.z != null && { z: input.z }),
    layerId: input.layerId,
    ...(input.color != null && { color: input.color }),
    attrs: { ...input.attrs },
    ...(input.label != null && { label: input.label }),
  };
}

export const pointCreate: ProductCommand<PointCreate, PointCreated, PointPlan> = {
  id: 'cad.point.create',
  version: 1,

  validate(cx, input) {
    return validated(check(cx.doc, input));
  },

  plan(cx, input) {
    const checked = check(cx.doc, input);
    if (!Array.isArray(checked)) return checked;
    // The slot is given when it is written: 0 until then.
    return { status: 'completed', output: { entity: { ...pointOf(input), id: 0 }, revision: String(cx.doc.revision) }, warnings: checked };
  },

  /** Writes through the document's own `add`: undo step “Ekle”, as the point tool always wrote each point; into the open transaction or group, if one is. */
  execute(cx, input) {
    const checked = check(cx.doc, input);
    if (!Array.isArray(checked)) return checked;
    const e = cx.doc.add(pointOf(input));
    return { status: 'completed', output: { uid: e.uid, id: e.id, revision: String(cx.doc.revision) }, warnings: checked };
  },
};
