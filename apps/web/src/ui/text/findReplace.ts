import type { AppContext } from '../../app/context';
import type { EntityEdit } from '../../contracts/generated/EntityEdit';
import type { EntityGeometry as EditGeometry } from '../../contracts/generated/EntityGeometry';
import type { TextEntity } from '../../model/entities';
import { textReplace, type FindHow } from '../../model/textEdit';
import { geometryOf } from '../../product/entitiesEdit';
import { uidOf, writeEdit } from '../../tools/editCommand';
import { textRunsRetext } from '../../model/paragraph';

/**
 * Bul ve değiştir's matches (docs/adr/0145 §6): the texts in scope whose words the core's rule changes
 * (`textReplace`, one call for all of them), each with its layer and whether it may be written. A text on a locked
 * layer is listed but not written; one that would become empty neither (a text is never empty). The desktop's
 * `find_replace` is the same.
 */
export interface Match {
  id: number;
  layer: string;
  old: string;
  /** What it becomes, trimmed. */
  new: string;
  /** Why it is not written, if it is not: its layer is locked, or it would become empty. */
  blocked: 'locked' | 'empty' | null;
}

export interface FindQuery extends FindHow {
  find: string;
  replace: string;
  /** Only the selected texts, else every text of the drawing. */
  selectionOnly: boolean;
}

export function findMatches(ctx: AppContext, q: FindQuery): Match[] {
  if (!q.find) return [];
  const { doc } = ctx;
  const ids = q.selectionOnly ? [...ctx.selection.ids.value] : null;
  const texts = (ids ? ids.map((id) => doc.get(id)) : [...doc.all()]).filter((e): e is TextEntity => e?.kind === 'text');
  if (!texts.length) return [];
  const results = textReplace(
    texts.map((t) => t.text),
    q.find,
    q.replace,
    q,
  );
  const out: Match[] = [];
  texts.forEach((t, i) => {
    // A text is kept trimmed, as the in-place editor and Öznitelikler keep it.
    const next = results[i]?.trim();
    if (next === undefined || next === t.text) return;
    const blocked = doc.layers.isLocked(t.layerId) ? 'locked' : next ? null : 'empty';
    out.push({ id: t.id, layer: doc.layers.path(t.layerId), old: t.text, new: next, blocked });
  });
  return out;
}

/** The matches written in one step “Bul ve değiştir” (`cad.entities.edit`'s `replaceText`); how many, or null when refused. */
export function replaceMatches(ctx: AppContext, matches: readonly Match[]): number | null {
  const changes: EntityEdit[] = [];
  for (const m of matches) {
    if (m.blocked) continue;
    const t = ctx.doc.get(m.id);
    if (t?.kind !== 'text') continue;
    // A multi-line text's letter formats follow its letters (docs/adr/0182 §4).
    const runs = t.runs?.length ? textRunsRetext(t.runs, t.text, m.new) : undefined;
    changes.push({ kind: 'update', uid: uidOf(ctx, t), geometry: { ...geometryOf(t as unknown as EditGeometry), text: m.new, ...(t.runs?.length && { runs }) } as unknown as EditGeometry });
  }
  if (!changes.length) return 0;
  return writeEdit(ctx, 'replaceText', changes) ? changes.length : null;
}
