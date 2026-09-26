import type { ArcCreate } from '../contracts/generated/ArcCreate';
import type { ArcCreated } from '../contracts/generated/ArcCreated';
import type { ArcPlan } from '../contracts/generated/ArcPlan';
import type { CommandWarning } from '../contracts/generated/CommandWarning';
import type { CadDocument } from '../model/document';
import type { NewEntity } from '../model/entities';
import { checkLayer, checkRadius, checkRevision, copy, notFinite, notFiniteValue, validated, type Stop } from './checks';
import type { ProductCommand } from './command';

/**
 * `cad.arc.create` v1 (docs/adr/0032): one circular arc on a named layer, as
 * the document stores it (centre, radius, counter-clockwise from `a0` to
 * `a1`), as one undo step. The web's handler over `CadDocument`; the
 * desktop's is `crates/native/application/src/arc.rs`. Both pass the shared
 * cases in fixtures/commands/v1/cad.arc.create.json.
 *
 * The arc tool computes the arc of each of its methods through the geometry
 * core and gives the result here.
 *
 * The checks, in order (the first that fails answers): the centre finite (x
 * before y), the radius, `a0` and `a1` finite, the radius above zero; the
 * expected revision; the layer (checks.ts, for the reasons polygonCreate.ts
 * gives). The angles are stored as given: equal angles are a full turn, as
 * the document reads them, and no angle is refused for its size.
 */

/** The checks in the contract's order: why nothing may be written, or the warnings when it may. */
function check(doc: CadDocument, input: ArcCreate): Stop | CommandWarning[] {
  return (
    notFinite(input.c, 'Merkezin', 'c') ??
    notFiniteValue(input.r, 'Yarıçap', 'Yarıçapı sonlu bir sayıyla verin.', 'r') ??
    notFiniteValue(input.a0, 'Başlangıç açısı', 'Açıyı sonlu bir sayıyla verin.', 'a0') ??
    notFiniteValue(input.a1, 'Bitiş açısı', 'Açıyı sonlu bir sayıyla verin.', 'a1') ??
    checkRadius(input.r) ??
    checkRevision(doc, input.expectedRevision) ??
    checkLayer(doc, input.layerId)
  );
}

/** The arc `input` describes, as the document stores it: its own copies, never the caller's objects. */
function arcOf(input: ArcCreate): NewEntity & { kind: 'arc' } {
  return {
    kind: 'arc',
    c: copy(input.c),
    r: input.r,
    a0: input.a0,
    a1: input.a1,
    layerId: input.layerId,
    ...(input.color != null && { color: input.color }),
    attrs: { ...input.attrs },
  };
}

export const arcCreate: ProductCommand<ArcCreate, ArcCreated, ArcPlan> = {
  id: 'cad.arc.create',
  version: 1,

  validate(cx, input) {
    return validated(check(cx.doc, input));
  },

  plan(cx, input) {
    const checked = check(cx.doc, input);
    if (!Array.isArray(checked)) return checked;
    // The slot is given when it is written: 0 until then.
    return { status: 'completed', output: { entity: { ...arcOf(input), id: 0 }, revision: String(cx.doc.revision) }, warnings: checked };
  },

  /** Writes through the document's own `add`: undo step “Ekle”, as the arc tool always wrote; into the open transaction or group, if one is. */
  execute(cx, input) {
    const checked = check(cx.doc, input);
    if (!Array.isArray(checked)) return checked;
    const e = cx.doc.add(arcOf(input));
    return { status: 'completed', output: { uid: e.uid, id: e.id, revision: String(cx.doc.revision) }, warnings: checked };
  },
};
