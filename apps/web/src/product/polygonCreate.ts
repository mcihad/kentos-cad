import type { CommandError } from '../contracts/generated/CommandError';
import type { CommandWarning } from '../contracts/generated/CommandWarning';
import type { PolygonCreate } from '../contracts/generated/PolygonCreate';
import type { PolygonCreated } from '../contracts/generated/PolygonCreated';
import type { PolygonPlan } from '../contracts/generated/PolygonPlan';
import type { Vec2 } from '../contracts/generated/Vec2';
import type { CadDocument } from '../model/document';
import type { NewEntity } from '../model/entities';
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
 * was prepared, the conflict comes before the layer's state.
 *
 * Nothing here is geometry (CLAUDE.md §4.8.1): the checks are counts,
 * finite numbers and the layer tree. Geometric validity (a ring crossing
 * itself, zero area, a hole outside the ring) is not checked, as the drawing
 * tools and the file reader do not check it either.
 */

const MIN_CORNERS = 3;
/** A revision as text: a whole decimal number, no sign, no leading zero (DOM-12). */
const REVISION_TEXT = /^(0|[1-9][0-9]*)$/;
const AXES = [
  ['x', 'doğu (Y)'],
  ['y', 'kuzey (X)'],
] as const;

/** Why a call stops before anything is written. */
type Stop = { status: 'failed'; error: CommandError } | { status: 'conflict'; error: CommandError };

const error = (code: string, message: string, path: string): CommandError => ({ code, message, path });
const failed = (e: CommandError): Stop => ({ status: 'failed', error: e });

/** Which ring of the input: the outer one (null) or hole `h` (0-based). */
type Ring = number | null;
/** The input path of one of the ring's fields: `pts`, `holes[0].bulges`. */
const pathOf = (ring: Ring, field: string) => (ring === null ? field : `holes[${ring}].${field}`);
/** Corner `i` (0-based) as a message names it. */
const cornerOf = (ring: Ring, i: number) => (ring === null ? `${i + 1}. köşenin` : `${ring + 1}. deliğin ${i + 1}. köşesinin`);
/** Edge `i` (0-based; the last closes the ring) as a message names it. */
const edgeOf = (ring: Ring, i: number) => (ring === null ? `${i + 1}. kenarın` : `${ring + 1}. deliğin ${i + 1}. kenarının`);

function checkRing(ring: Ring, pts: readonly Vec2[], bulges: readonly number[] | null | undefined): CommandError | null {
  const n = pts.length;
  if (n < MIN_CORNERS) {
    const message =
      ring === null
        ? `Kapalı alanın en az 3 köşesi olmalı; ${n} köşe verildi. Eksik köşeleri ekleyin.`
        : `${ring + 1}. deliğin en az 3 köşesi olmalı; ${n} köşe verildi. Eksik köşeleri ekleyin ya da deliği çıkarın.`;
    return error('too_few_corners', message, pathOf(ring, 'pts'));
  }
  for (let i = 0; i < n; i++)
    for (const [axis, name] of AXES)
      if (!Number.isFinite(pts[i][axis]))
        return error(
          'not_finite',
          `${cornerOf(ring, i)} ${name} değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin.`,
          `${pathOf(ring, 'pts')}[${i}].${axis}`,
        );
  if (bulges == null) return null;
  if (bulges.length !== n) {
    const whose = ring === null ? 'Yay değerleri' : `${ring + 1}. deliğin yay değerleri`;
    return error(
      'bulge_count',
      `${whose} köşe sayısı kadar olmalı, kapanış kenarı dahil her kenara bir değer: ${n} köşe, ${bulges.length} yay değeri verildi. Eksik ya da fazla değerleri düzeltin.`,
      pathOf(ring, 'bulges'),
    );
  }
  for (let i = 0; i < n; i++)
    if (!Number.isFinite(bulges[i]))
      return error(
        'not_finite',
        `${edgeOf(ring, i)} yay değeri sonlu bir sayı değil (NaN ya da sonsuz). Düz kenar için 0, yay için tan(açı/4) verin.`,
        `${pathOf(ring, 'bulges')}[${i}]`,
      );
  return null;
}

/** The checks in the contract's order: why nothing may be written, or the warnings when it may. */
function check(doc: CadDocument, input: PolygonCreate): Stop | CommandWarning[] {
  const outer = checkRing(null, input.pts, input.bulges);
  if (outer) return failed(outer);
  for (const [h, hole] of (input.holes ?? []).entries()) {
    const broken = checkRing(h, hole.pts, hole.bulges);
    if (broken) return failed(broken);
  }
  const expected = input.expectedRevision;
  if (expected != null) {
    if (!REVISION_TEXT.test(expected))
      return failed(
        error(
          'invalid_revision',
          `Beklenen sürüm “${expected}” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.`,
          'expectedRevision',
        ),
      );
    const current = String(doc.revision);
    if (expected !== current)
      return {
        status: 'conflict',
        error: {
          ...error(
            'revision_conflict',
            'Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.',
            'expectedRevision',
          ),
          revision: current,
        },
      };
  }
  const layers = doc.layers;
  const id = input.layerId;
  const node = layers.get(id);
  if (!node) return failed(error('layer_not_found', `“${id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.`, 'layerId'));
  if (node.type !== 'layer')
    return failed(error('not_a_layer', `“${node.name}” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.`, 'layerId'));
  // The closed-area tool's words (tools/targetLayer.ts), kept since the tool writes through here.
  if (layers.isLocked(id))
    return failed(error('layer_locked', `“${node.name}” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.`, 'layerId'));
  return layers.isVisible(id) ? [] : [{ code: 'layer_hidden', message: `“${node.name}” katmanı gizli; çizilen nesne görünmeyecek.`, path: 'layerId' }];
}

const copy = (p: Vec2): Vec2 => ({ x: p.x, y: p.y });

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
    const checked = check(cx.doc, input);
    return Array.isArray(checked) ? { status: 'completed', output: null, warnings: checked } : checked;
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
