import type { EntityGeometry as EditGeometry } from '../../contracts/generated/EntityGeometry';
import type { CadDocument } from '../../model/document';
import type { Entity, PointEntity } from '../../model/entities';
import type { Vec2 } from '../../model/geometry';
import { duplicatePoints, followPoint } from '../../model/ops/pointEditor';
import { textIncrement } from '../../model/textEdit';
import { elevatedPaths } from '../../product/elevation';
import { entitiesDelete } from '../../product/entitiesDelete';
import { entitiesEdit } from '../../product/entitiesEdit';
import { entitiesSet } from '../../product/entitiesSet';
import { parseNumber } from '../../tools/coordinateInput';
import { FOLLOW_LOCKED, PREFIX, inStep, withPaths } from './pointEdit';

/**
 * Nokta editörü's batch operations (docs/adr/0153 §5), apart from the DOM: Yeniden adlandır, Sıralı numara ver and
 * Katmana taşı over the table's target rows, written through cad.entities.set, and Çift noktaları ayıkla, through
 * cad.entities.edit and cad.entities.delete; each one undo step named after it. fixtures/point-editor/v1/batch.json and
 * dedupe.json hold both platforms to the same drawing, messages and steps; the desktop's is
 * `apps/desktop/src/points/batch.rs`.
 */

/** An operation as its window gives it. */
export type BatchOp =
  | { kind: 'rename'; mode: 'add' | 'remove'; prefix: string }
  | { kind: 'number'; start: string }
  | { kind: 'layer'; layer: string }
  | { kind: 'dedupe'; by: 'name' | 'place'; tolerance: string; keep: 'first' | 'last' | 'average' };

/** The undo steps (and the windows' titles), by the operations' kinds. */
export const BATCH_STEP = { rename: 'Yeniden adlandır', number: 'Sıralı numara ver', layer: 'Katmana taşı', dedupe: 'Çift noktaları ayıkla' } as const;

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
 * After İçe aktar from the table (docs/adr/0153 §5): the imported points' names (trimmed; one without a name is left
 * out) that two points or more of the drawing carry, and the points carrying them in the drawing's order, for Çift
 * noktaları ayıkla by Aynı ad (İlki keeps the drawing's, Sonuncusu the file's); null when there are none: no window.
 */
export function importTargets(doc: CadDocument, imported: readonly number[]): { ids: number[]; header: string } | null {
  const nameOf = (e: Entity | undefined) => (e?.kind === 'point' ? (e.label?.trim() ?? '') : '');
  const names = new Set(imported.map((id) => nameOf(doc.get(id))).filter((n) => n !== ''));
  if (!names.size) return null;
  // The table's points: a multi-point object is none of them (docs/adr/0174).
  const points = [...doc.all()].filter((e): e is PointEntity => e.kind === 'point' && !e.parts);
  const carried = new Map<string, number>();
  for (const e of points) {
    const name = nameOf(e);
    if (names.has(name)) carried.set(name, (carried.get(name) ?? 0) + 1);
  }
  const ids = points.filter((e) => (carried.get(nameOf(e)) ?? 0) > 1).map((e) => e.id);
  return ids.length ? { ids, header: `Aynı adlı ${ids.length} nokta` } : null;
}

/**
 * The names the points would take, in order (null: the point keeps its own), or why none can be given. Önek ekle puts
 * the trimmed prefix before every name (a point without one keeps none); Önek kaldır takes it from the names that start
 * with it, the rest trimmed (a name left empty stays); Sıralı numara ver gives the first the start, every next one the
 * one before's Artır.
 */
export function plannedNames(points: readonly PointEntity[], op: Extract<BatchOp, { kind: 'rename' | 'number' }>): (string | null)[] | string {
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
    return e?.kind === 'point' && !e.parts ? [e] : [];
  });

export function planBatch(doc: CadDocument, ids: readonly number[], op: Exclude<BatchOp, { kind: 'dedupe' }>): BatchPlan {
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
 * for a move, the command's warning (a hidden layer). Çift noktaları ayıkla takes Bağlı çizgiler izler (`follow`).
 */
export function runBatch(doc: CadDocument, ids: readonly number[], op: BatchOp, follow = true): BatchOutcome {
  if (op.kind === 'dedupe') return runDedupe(doc, ids, op, follow);
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

/** Çift noktaları ayıkla's plan: the groups, the points removed, the kept ones that move, and the window's summary. */
export interface DedupePlan {
  /** Why nothing may be done (a tolerance that is no number), or null. */
  error: string | null;
  /** The groups' members' ids, in the drawing's order. */
  groups: number[][];
  /** “6 grupta 15 nokta; 9 nokta silinecek.”, or “Çift nokta yok.”; null with an error. */
  summary: string | null;
  removed: PointEntity[];
  /** The kept points whose place or elevation changes: where to, and whether the elevation does. */
  moves: { e: PointEntity; to: Vec2; z: number | null; zChanged: boolean }[];
}

/**
 * The groups of duplicates among the target points (docs/adr/0153 §5), by the shared core (`duplicatePoints`): by name,
 * or by place within the tolerance (the window's text, metres, zero or more); the points taken in the drawing's order.
 */
export function planDedupe(doc: CadDocument, ids: readonly number[], op: Extract<BatchOp, { kind: 'dedupe' }>): DedupePlan {
  const tolerance = op.by === 'place' ? parseNumber(op.tolerance) : 0;
  if (tolerance === null || tolerance < 0) return { error: `${PREFIX}Tolerans sıfır ya da daha büyük bir sayı olmalı.`, groups: [], summary: null, removed: [], moves: [] };
  const order = new Map<number, number>();
  let k = 0;
  for (const e of doc.all()) order.set(e.id, k++);
  const points = pointsOf(doc, ids).sort((a, b) => (order.get(a.id) ?? 0) - (order.get(b.id) ?? 0));
  const found = duplicatePoints(
    points.map((e) => ({ p: e.p, z: e.z ?? null, name: e.label ?? null })),
    op.by,
    tolerance,
    op.keep,
  );
  const groups = found.groups.map((g) => g.members.map((i) => points[i].id));
  const removed = found.removed.map((i) => points[i]);
  const moves = found.groups.flatMap((g) => {
    const e = points[g.kept];
    const z = g.z ?? null;
    const zChanged = z !== (e.z ?? null);
    return g.p.x !== e.p.x || g.p.y !== e.p.y || zChanged ? [{ e, to: g.p, z, zChanged }] : [];
  });
  const total = groups.reduce((n, g) => n + g.length, 0);
  const summary = groups.length
    ? `${groups.length} grupta ${total} nokta; ${removed.length} nokta silinecek${moves.length ? `, ${moves.length} nokta ortalamaya taşınacak.` : '.'}`
    : 'Çift nokta yok.';
  return { error: null, groups, summary, removed, moves };
}

/**
 * Çift noktaları ayıkla written as one undo step (docs/adr/0153 §5): the kept points moved group by group (with
 * `follow`, the line work at a kept point's place moves with it and takes its new elevation when that changes), then
 * the others removed. The first point that would change, in the drawing's order, on a locked layer stops it all with
 * the edit command's words; so does a line work that would follow on one.
 */
function runDedupe(doc: CadDocument, ids: readonly number[], op: Extract<BatchOp, { kind: 'dedupe' }>, follow: boolean): BatchOutcome {
  const plan = planDedupe(doc, ids, op);
  if (plan.error) return { said: [plan.error], step: null };
  if (!plan.groups.length) return { said: [`${PREFIX}Çift nokta yok.`], step: null };
  const order = new Map<number, number>();
  let k = 0;
  for (const e of doc.all()) order.set(e.id, k++);
  const changing = [...plan.moves.map((m) => m.e), ...plan.removed].sort((a, b) => (order.get(a.id) ?? 0) - (order.get(b.id) ?? 0));
  const locked = changing.find((e) => doc.layers.isLocked(e.layerId));
  if (locked) {
    const name = doc.layers.get(locked.layerId)?.name ?? locked.layerId;
    return { said: [`“${name}” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın.`], step: null };
  }
  const step = BATCH_STEP.dedupe;
  const refused = inStep(doc, step, () => {
    for (const m of plan.moves) {
      const changes = [{ kind: 'update' as const, uid: doc.uidOf(m.e.id) ?? '', geometry: { kind: 'point', p: m.to, ...(m.z !== null && { z: m.z }) } as EditGeometry }];
      if (follow) {
        for (const other of doc.all()) {
          if (other.kind !== 'line' && other.kind !== 'polyline' && other.kind !== 'polygon') continue;
          const moved = followPoint(elevatedPaths(other), m.e.p, m.to, m.zChanged, m.zChanged ? m.z : null);
          const geometry = moved && withPaths(other, moved);
          if (!geometry) continue;
          if (doc.layers.isLocked(other.layerId)) return FOLLOW_LOCKED;
          changes.push({ kind: 'update', uid: doc.uidOf(other.id) ?? '', geometry });
        }
      }
      const r = entitiesEdit.execute({ doc }, { operation: 'properties', changes });
      if (r.status !== 'completed') return 'error' in r ? r.error.message : `${PREFIX}nokta taşınamadı.`;
    }
    const r = entitiesDelete.execute({ doc }, { uids: plan.removed.map((e) => doc.uidOf(e.id) ?? '') });
    if (r.status !== 'completed') return 'error' in r ? r.error.message : `${PREFIX}noktalar silinemedi.`;
    return null;
  });
  if (refused) return { said: [refused], step: null };
  const n = plan.groups.length;
  const done = `${PREFIX}${n} grupta ${plan.removed.length} nokta silindi`;
  return { said: [plan.moves.length ? `${done}, ${plan.moves.length} nokta ortalamaya taşındı.` : `${done}.`], step };
}
