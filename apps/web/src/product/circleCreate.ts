import type { CircleCreate } from '../contracts/generated/CircleCreate';
import type { CircleCreated } from '../contracts/generated/CircleCreated';
import type { CirclePlan } from '../contracts/generated/CirclePlan';
import type { CommandWarning } from '../contracts/generated/CommandWarning';
import type { CadDocument } from '../model/document';
import type { NewEntity } from '../model/entities';
import { checkLayer, checkRadius, checkRevision, copy, notFinite, notFiniteValue, validated, type Stop } from './checks';
import type { ProductCommand } from './command';

/**
 * `cad.circle.create` v1 (docs/adr/0032): one circle by its centre and
 * radius on a named layer, as one undo step. The web's handler over
 * `CadDocument`; the desktop's is `crates/native/application/src/circle.rs`.
 * Both pass the shared cases in fixtures/commands/v1/cad.circle.create.json.
 *
 * The circle tool computes the circle of each of its methods through the
 * geometry core and gives the result here.
 *
 * The checks, in order (the first that fails answers): the centre finite (x
 * before y), then the radius finite, then above zero; the expected
 * revision; the layer (checks.ts, for the reasons polygonCreate.ts gives).
 */

/** The checks in the contract's order: why nothing may be written, or the warnings when it may. */
function check(doc: CadDocument, input: CircleCreate): Stop | CommandWarning[] {
  return (
    notFinite(input.c, 'Merkezin', 'c') ??
    notFiniteValue(input.r, 'Yarıçap', 'Yarıçapı sonlu bir sayıyla verin.', 'r') ??
    checkRadius(input.r) ??
    checkRevision(doc, input.expectedRevision) ??
    checkLayer(doc, input.layerId)
  );
}

/** The circle `input` describes, as the document stores it: its own copies, never the caller's objects. */
function circleOf(input: CircleCreate): NewEntity & { kind: 'circle' } {
  return {
    kind: 'circle',
    c: copy(input.c),
    r: input.r,
    layerId: input.layerId,
    ...(input.color != null && { color: input.color }),
    attrs: { ...input.attrs },
  };
}

export const circleCreate: ProductCommand<CircleCreate, CircleCreated, CirclePlan> = {
  id: 'cad.circle.create',
  version: 1,

  validate(cx, input) {
    return validated(check(cx.doc, input));
  },

  plan(cx, input) {
    const checked = check(cx.doc, input);
    if (!Array.isArray(checked)) return checked;
    // The slot is given when it is written: 0 until then.
    return { status: 'completed', output: { entity: { ...circleOf(input), id: 0 }, revision: String(cx.doc.revision) }, warnings: checked };
  },

  /** Writes through the document's own `add`: undo step “Ekle”, as the circle tool always wrote; into the open transaction or group, if one is. */
  execute(cx, input) {
    const checked = check(cx.doc, input);
    if (!Array.isArray(checked)) return checked;
    const e = cx.doc.add(circleOf(input));
    return { status: 'completed', output: { uid: e.uid, id: e.id, revision: String(cx.doc.revision) }, warnings: checked };
  },
};
