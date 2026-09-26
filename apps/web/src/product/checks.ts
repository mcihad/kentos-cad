import type { CommandError } from '../contracts/generated/CommandError';
import type { CommandResult } from '../contracts/generated/CommandResult';
import type { CommandWarning } from '../contracts/generated/CommandWarning';
import type { Vec2 } from '../contracts/generated/Vec2';
import { isUuid } from '../core/uuid';
import type { CadDocument } from '../model/document';
import type { Entity } from '../model/entities';

/**
 * What every create command checks after its own input (docs/adr/0022,
 * 0027): the expected revision, then the layer. The codes, paths and
 * messages are the same for every command, so the shared cases of each
 * (fixtures/commands/v1) hold them the same way. The desktop's counterpart
 * is `crates/native/application/src/checks.rs`.
 *
 * Nothing here is geometry (CLAUDE.md §4.8.1): counts, finite numbers and
 * the layer tree's state.
 */

/** A revision as text: a whole decimal number, no sign, no leading zero (DOM-12). */
const REVISION_TEXT = /^(0|[1-9][0-9]*)$/;
const AXES = [
  ['x', 'doğu (Y)'],
  ['y', 'kuzey (X)'],
] as const;

/** Why a call stops before anything is written. */
export type Stop = { status: 'failed'; error: CommandError } | { status: 'conflict'; error: CommandError };

export const error = (code: string, message: string, path: string): CommandError => ({ code, message, path });
export const failed = (e: CommandError): Stop => ({ status: 'failed', error: e });

/** The first coordinate of `p` that is NaN or ±∞ (x before y), named as a message names it (`whose`: “2. noktanın”). */
export function notFinite(p: Vec2, whose: string, path: string): Stop | null {
  for (const [axis, name] of AXES)
    if (!Number.isFinite(p[axis]))
      return failed(error('not_finite', `${whose} ${name} değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin.`, `${path}.${axis}`));
  return null;
}

/** A number that is NaN or ±∞ (a radius, an angle, an elevation), as its message names it: “Yarıçap sonlu bir sayı değil …”. */
export function notFiniteValue(value: number, what: string, fix: string, path: string): Stop | null {
  return Number.isFinite(value) ? null : failed(error('not_finite', `${what} sonlu bir sayı değil (NaN ya da sonsuz). ${fix}`, path));
}

/** A circle's or an arc's radius: above zero (`invalid_radius`), once it is finite. */
export function checkRadius(r: number): Stop | null {
  return r > 0 ? null : failed(error('invalid_radius', 'Yarıçap sıfırdan büyük olmalı. Pozitif bir yarıçap verin.', 'r'));
}

/** The expected revision, when given: decimal text, then the document's own (`conflict` when not, with the revision now). */
export function checkRevision(doc: CadDocument, expected: string | null | undefined): Stop | null {
  if (expected == null) return null;
  if (!REVISION_TEXT.test(expected))
    return failed(
      error(
        'invalid_revision',
        `Beklenen sürüm “${expected}” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.`,
        'expectedRevision',
      ),
    );
  const current = String(doc.revision);
  if (expected === current) return null;
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

/**
 * The layer the object goes on: known, a layer not a group, not locked by
 * itself or a group above. A hidden one takes the object with a warning.
 * The locked and hidden texts are the drawing tools' own words
 * (tools/targetLayer.ts), kept since the tools write through here.
 */
export function checkLayer(doc: CadDocument, id: string): Stop | CommandWarning[] {
  const layers = doc.layers;
  const node = layers.get(id);
  if (!node) return failed(error('layer_not_found', `“${id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.`, 'layerId'));
  if (node.type !== 'layer')
    return failed(error('not_a_layer', `“${node.name}” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.`, 'layerId'));
  if (layers.isLocked(id))
    return failed(error('layer_locked', `“${node.name}” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.`, 'layerId'));
  return layers.isVisible(id) ? [] : [{ code: 'layer_hidden', message: `“${node.name}” katmanı gizli; çizilen nesne görünmeyecek.`, path: 'layerId' }];
}

/**
 * The ids a command names (`cad.entities.delete`, `cad.entities.transform`):
 * at least one (`no_entities`, with the command's own sentence `nothing`),
 * each lowercase UUID text with hyphens (`invalid_uid`, the first that is not).
 */
export function checkUids(uids: readonly string[], nothing: string): Stop | null {
  if (!uids.length) return failed(error('no_entities', `${nothing} En az bir nesnenin kalıcı kimliğini verin.`, 'uids'));
  for (const [i, uid] of uids.entries())
    if (!isUuid(uid))
      return failed(
        error(
          'invalid_uid',
          `“${uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın.`,
          `uids[${i}]`,
        ),
      );
  return null;
}

/**
 * The objects the ids name, each once, in the input's order, with their ids;
 * `entity_not_found` for the first the document lacks.
 */
export function findObjects(doc: CadDocument, uids: readonly string[]): Stop | { entity: Entity; uid: string }[] {
  const seen = new Set<string>();
  const out: { entity: Entity; uid: string }[] = [];
  for (const [i, uid] of uids.entries()) {
    if (seen.has(uid)) continue;
    seen.add(uid);
    const entity = doc.byUid(uid);
    if (!entity)
      return failed(error('entity_not_found', `“${uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.`, `uids[${i}]`));
    out.push({ entity, uid });
  }
  return out;
}

/** `validate`'s answer from the checks: nothing, with the warnings, or why not. */
export const validated = (checked: Stop | CommandWarning[]): CommandResult<null> =>
  Array.isArray(checked) ? { status: 'completed', output: null, warnings: checked } : checked;

export const copy = (p: Vec2): Vec2 => ({ x: p.x, y: p.y });
