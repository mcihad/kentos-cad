import type { CadDocument } from '../../model/document';
import type { PointEntity } from '../../model/entities';
import { textIncrement } from '../../model/textEdit';
import { entitiesSet } from '../../product/entitiesSet';
import { PREFIX, inStep } from './pointEdit';

/**
 * Nokta editörü's batch operations (docs/adr/0153 §5), apart from the DOM: Yeniden adlandır, Sıralı numara ver and
 * Katmana taşı over the table's target rows, each one undo step named after it, written through cad.entities.set.
 * fixtures/point-editor/v1/batch.json holds both platforms to the same drawing, messages and steps; the desktop's is
 * `apps/desktop/src/points/batch.rs`.
 */

/** An operation as its window gives it. */
export type BatchOp = { kind: 'rename'; mode: 'add' | 'remove'; prefix: string } | { kind: 'number'; start: string } | { kind: 'layer'; layer: string };

/** The undo steps (and the windows' titles), by the operations' kinds. */
export const BATCH_STEP = { rename: 'Yeniden adlandır', number: 'Sıralı numara ver', layer: 'Katmana taşı' } as const;

/** What came of an operation: what to say, and the undo step written (null: nothing). */
export interface BatchOutcome {
  said: string[];
  step: string | null;
}

/**
 * The rows an operation takes (docs/adr/0153 §5): the table's selected rows in its order, or every row when none of
 * them is selected; and the header naming them. A selected point the table does not show takes no part.
 */
export function batchTargets(shown: readonly number[], selected: (id: number) => boolean): { ids: number[]; header: string } {
  const picked = shown.filter((id) => selected(id));
  return picked.length ? { ids: picked, header: `${picked.length} seçili nokta` } : { ids: [...shown], header: `Tablodaki ${shown.length} nokta` };
}

/**
 * The names the points would take, in order (null: the point keeps its own), or why none can be given. Önek ekle puts
 * the trimmed prefix before every name (a point without one keeps none); Önek kaldır takes it from the names that start
 * with it, the rest trimmed (a name left empty stays); Sıralı numara ver gives the first the start, every next one the
 * one before's Artır.
 */
export function plannedNames(points: readonly PointEntity[], op: Exclude<BatchOp, { kind: 'layer' }>): (string | null)[] | string {
  if (op.kind === 'rename') {
    const prefix = op.prefix.trim();
    if (!prefix) return `${PREFIX}Önek yazılmalı.`;
    return points.map((e) => {
      const name = e.label?.trim() ?? '';
      if (op.mode === 'add') return name ? prefix + name : null;
      return name.startsWith(prefix) ? name.slice(prefix.length).trim() || null : null;
    });
  }
  const start = op.start.trim();
  if (!/[0-9]$/.test(start)) return `${PREFIX}Başlangıç adı sayıyla bitmeli.`;
  const names: string[] = [];
  let name = start;
  for (let i = 0; i < points.length; i++) {
    names.push(name);
    name = textIncrement(name) ?? name;
  }
  return names;
}

/** What an operation would change, for its window: why it may not, or the points that change (with their new name). */
export interface BatchPlan {
  error: string | null;
  changes: { e: PointEntity; name: string | null }[];
}

const pointsOf = (doc: CadDocument, ids: readonly number[]): PointEntity[] =>
  ids.flatMap((id) => {
    const e = doc.get(id);
    return e?.kind === 'point' ? [e] : [];
  });

export function planBatch(doc: CadDocument, ids: readonly number[], op: BatchOp): BatchPlan {
  const points = pointsOf(doc, ids);
  if (op.kind === 'layer') return { error: null, changes: points.filter((e) => e.layerId !== op.layer).map((e) => ({ e, name: null })) };
  const names = plannedNames(points, op);
  if (typeof names === 'string') return { error: names, changes: [] };
  return {
    error: null,
    changes: points.flatMap((e, i) => {
      const name = names[i];
      return name !== null && name !== e.label ? [{ e, name }] : [];
    }),
  };
}

/**
 * The operation written as one undo step named after it (docs/adr/0153 §5). Only the points that change are written,
 * names one by one in the targets' order, a layer move in one call; a refusal of the command is said as it is and
 * nothing is written. Then it is said what changed: for names, also how many of the new names another point has too;
 * for a move, the command's warning (a hidden layer).
 */
export function runBatch(doc: CadDocument, ids: readonly number[], op: BatchOp): BatchOutcome {
  const plan = planBatch(doc, ids, op);
  if (plan.error) return { said: [plan.error], step: null };
  const step = BATCH_STEP[op.kind];
  if (op.kind === 'layer') {
    if (!plan.changes.length) return { said: [`${PREFIX}Taşınacak nokta yok.`], step: null };
    let warnings: string[] = [];
    const refused = inStep(doc, step, () => {
      const r = entitiesSet.execute({ doc }, { uids: plan.changes.map((c) => doc.uidOf(c.e.id) ?? ''), layerId: op.layer, operation: 'layer' });
      if (r.status !== 'completed') return 'error' in r ? r.error.message : `${PREFIX}noktalar taşınamadı.`;
      warnings = r.warnings.map((w) => w.message);
      return null;
    });
    if (refused) return { said: [refused], step: null };
    const name = doc.layers.get(op.layer)?.name ?? op.layer;
    return { said: [`${PREFIX}${plan.changes.length} nokta “${name}” katmanına taşındı.`, ...warnings], step };
  }
  if (!plan.changes.length) return { said: [`${PREFIX}Adı değişen nokta yok.`], step: null };
  const refused = inStep(doc, step, () => {
    for (const c of plan.changes) {
      const r = entitiesSet.execute({ doc }, { uids: [doc.uidOf(c.e.id) ?? ''], label: c.name, operation: 'label' });
      if (r.status !== 'completed') return 'error' in r ? r.error.message : `${PREFIX}ad yazılamadı.`;
    }
    return null;
  });
  if (refused) return { said: [refused], step: null };
  // The new names are trimmed: one counted twice is another point's too.
  const count = new Map<string, number>();
  for (const e of doc.all()) {
    const name = e.kind === 'point' ? e.label?.trim() : undefined;
    if (name) count.set(name, (count.get(name) ?? 0) + 1);
  }
  const shared = plan.changes.filter((c) => (count.get(c.name ?? '') ?? 0) > 1).length;
  const done = `${PREFIX}${plan.changes.length} noktanın adı değişti`;
  return { said: [shared ? `${done}; ${shared} ad başka noktalarda da var.` : `${done}.`], step };
}
