import type { CommandWarning } from '../contracts/generated/CommandWarning';
import type { PolygonCreate } from '../contracts/generated/PolygonCreate';
import type { PolygonCreated } from '../contracts/generated/PolygonCreated';
import type { PolygonPlan } from '../contracts/generated/PolygonPlan';
import type { Vec2 } from '../contracts/generated/Vec2';
import type { CadDocument } from '../model/document';
import type { NewEntity } from '../model/entities';
import { checkLayer, checkRevision, copy, error, failed, notFinite, validated, type Stop } from './checks';
import type { ProductCommand } from './command';

/**
 * `cad.polygon.create` v1 (docs/adr/0022): one closed area on a named
 * layer, as one undo step. The web's handler over `CadDocument`; the
 * desktop's is `crates/native/application/src/polygon.rs`. Both pass the
 * shared cases in fixtures/commands/v1/cad.polygon.create.json, which fix
 * the checks, their order, the codes, paths and messages.
 *
 * The checks, in order (the first that fails answers): the outer ring, then
 * each hole (at least 3 corners, finite coordinates, one bulge per edge
 * when bulges are given, finite bulges); the expected revision (decimal
 * text, then the document's own: `conflict` when not); the layer (known, a
 * layer not a group, not locked by itself or a group above). A hidden layer
 * is written with a warning. An input broken by itself is refused before
 * the document is looked at; once the document has changed since the input
 * was prepared, the conflict comes before the layer's state. The last two
 * are every create command's (checks.ts).
 *
 * Nothing here is geometry (CLAUDE.md §4.8.1): the checks are counts,
 * finite numbers and the layer tree. Geometric validity (a ring crossing
 * itself, zero area, a hole outside the ring) is not checked, as the drawing
 * tools and the file reader do not check it either.
 */

const MIN_CORNERS = 3;

/** Which ring of the input: the outer one (null) or hole `h` (0-based). */
type Ring = number | null;
/** The input path of one of the ring's fields: `pts`, `holes[0].bulges`. */
const pathOf = (ring: Ring, field: string) => (ring === null ? field : `holes[${ring}].${field}`);
/** Corner `i` (0-based) as a message names it. */
const cornerOf = (ring: Ring, i: number) => (ring === null ? `${i + 1}. köşenin` : `${ring + 1}. deliğin ${i + 1}. köşesinin`);
/** Edge `i` (0-based; the last closes the ring) as a message names it. */
const edgeOf = (ring: Ring, i: number) => (ring === null ? `${i + 1}. kenarın` : `${ring + 1}. deliğin ${i + 1}. kenarının`);

function checkRing(ring: Ring, pts: readonly Vec2[], bulges: readonly number[] | null | undefined): Stop | null {
  const n = pts.length;
  if (n < MIN_CORNERS) {
    const message =
      ring === null
        ? `Kapalı alanın en az 3 köşesi olmalı; ${n} köşe verildi. Eksik köşeleri ekleyin.`
        : `${ring + 1}. deliğin en az 3 köşesi olmalı; ${n} köşe verildi. Eksik köşeleri ekleyin ya da deliği çıkarın.`;
    return failed(error('too_few_corners', message, pathOf(ring, 'pts')));
  }
  for (let i = 0; i < n; i++) {
    const broken = notFinite(pts[i], cornerOf(ring, i), `${pathOf(ring, 'pts')}[${i}]`);
    if (broken) return broken;
  }
  if (bulges == null) return null;
  if (bulges.length !== n) {
    const whose = ring === null ? 'Yay değerleri' : `${ring + 1}. deliğin yay değerleri`;
    return failed(
      error(
        'bulge_count',
        `${whose} köşe sayısı kadar olmalı, kapanış kenarı dahil her kenara bir değer: ${n} köşe, ${bulges.length} yay değeri verildi. Eksik ya da fazla değerleri düzeltin.`,
        pathOf(ring, 'bulges'),
      ),
    );
  }
  for (let i = 0; i < n; i++)
    if (!Number.isFinite(bulges[i]))
      return failed(
        error(
          'not_finite',
          `${edgeOf(ring, i)} yay değeri sonlu bir sayı değil (NaN ya da sonsuz). Düz kenar için 0, yay için tan(açı/4) verin.`,
          `${pathOf(ring, 'bulges')}[${i}]`,
        ),
      );
  return null;
}

/** The checks in the contract's order: why nothing may be written, or the warnings when it may. */
function check(doc: CadDocument, input: PolygonCreate): Stop | CommandWarning[] {
  const outer = checkRing(null, input.pts, input.bulges);
  if (outer) return outer;
  for (const [h, hole] of (input.holes ?? []).entries()) {
    const broken = checkRing(h, hole.pts, hole.bulges);
    if (broken) return broken;
  }
  return checkRevision(doc, input.expectedRevision) ?? checkLayer(doc, input.layerId);
}

/** The polygon `input` describes, as the document stores it: its own copies, never the caller's arrays. */
function polygonOf(input: PolygonCreate): NewEntity & { kind: 'polygon' } {
  return {
    kind: 'polygon',
    pts: input.pts.map(copy),
    ...(input.bulges != null && { bulges: [...input.bulges] }),
    ...(input.holes != null && { holes: input.holes.map((h) => ({ pts: h.pts.map(copy), ...(h.bulges != null && { bulges: [...h.bulges] }) })) }),
    layerId: input.layerId,
    ...(input.color != null && { color: input.color }),
    attrs: { ...input.attrs },
  };
}

export const polygonCreate: ProductCommand<PolygonCreate, PolygonCreated, PolygonPlan> = {
  id: 'cad.polygon.create',
  version: 1,

  validate(cx, input) {
    return validated(check(cx.doc, input));
  },

  plan(cx, input) {
    const checked = check(cx.doc, input);
    if (!Array.isArray(checked)) return checked;
    // The slot is given when it is written: 0 until then.
    return { status: 'completed', output: { entity: { ...polygonOf(input), id: 0 }, revision: String(cx.doc.revision) }, warnings: checked };
  },

  /** Writes through the document's own `add`: undo step “Ekle”, as the closed-area tool always wrote it; into the open transaction or group, if one is. */
  execute(cx, input) {
    const checked = check(cx.doc, input);
    if (!Array.isArray(checked)) return checked;
    const e = cx.doc.add(polygonOf(input));
    return { status: 'completed', output: { uid: e.uid, id: e.id, revision: String(cx.doc.revision) }, warnings: checked };
  },
};
