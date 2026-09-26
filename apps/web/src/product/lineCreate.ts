import type { CommandWarning } from '../contracts/generated/CommandWarning';
import type { LineCreate } from '../contracts/generated/LineCreate';
import type { LineCreated } from '../contracts/generated/LineCreated';
import type { LinePlan } from '../contracts/generated/LinePlan';
import type { CadDocument } from '../model/document';
import type { NewEntity } from '../model/entities';
import { checkLayer, checkRevision, copy, notFinite, validated, type Stop } from './checks';
import type { ProductCommand } from './command';

/**
 * `cad.line.create` v1 (docs/adr/0027): one straight line on a named layer,
 * as one undo step. The web's handler over `CadDocument`; the desktop's is
 * `crates/native/application/src/line.rs`. Both pass the shared cases in
 * fixtures/commands/v1/cad.line.create.json.
 *
 * The line tool draws a chain and calls this once per segment: each
 * segment is its own object and its own undo step, as the tool always wrote
 * them.
 *
 * The checks, in order (the first that fails answers): both ends finite
 * (`a` then `b`, x before y); the expected revision; the layer (checks.ts,
 * for the reasons polygonCreate.ts gives). A line whose ends coincide is
 * written: geometric validity is not checked, as for the closed area; the
 * tool never gives one.
 */

/** The checks in the contract's order: why nothing may be written, or the warnings when it may. */
function check(doc: CadDocument, input: LineCreate): Stop | CommandWarning[] {
  return (
    notFinite(input.a, 'Başlangıç noktasının', 'a') ??
    notFinite(input.b, 'Bitiş noktasının', 'b') ??
    checkRevision(doc, input.expectedRevision) ??
    checkLayer(doc, input.layerId)
  );
}

/** The line `input` describes, as the document stores it: its own copies, never the caller's objects. */
function lineOf(input: LineCreate): NewEntity & { kind: 'line' } {
  return {
    kind: 'line',
    a: copy(input.a),
    b: copy(input.b),
    layerId: input.layerId,
    ...(input.color != null && { color: input.color }),
    attrs: { ...input.attrs },
  };
}

export const lineCreate: ProductCommand<LineCreate, LineCreated, LinePlan> = {
  id: 'cad.line.create',
  version: 1,

  validate(cx, input) {
    return validated(check(cx.doc, input));
  },

  plan(cx, input) {
    const checked = check(cx.doc, input);
    if (!Array.isArray(checked)) return checked;
    // The slot is given when it is written: 0 until then.
    return { status: 'completed', output: { entity: { ...lineOf(input), id: 0 }, revision: String(cx.doc.revision) }, warnings: checked };
  },

  /** Writes through the document's own `add`: undo step “Ekle”, as the line tool always wrote each segment; into the open transaction or group, if one is. */
  execute(cx, input) {
    const checked = check(cx.doc, input);
    if (!Array.isArray(checked)) return checked;
    const e = cx.doc.add(lineOf(input));
    return { status: 'completed', output: { uid: e.uid, id: e.id, revision: String(cx.doc.revision) }, warnings: checked };
  },
};
