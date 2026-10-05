import { fixed } from '../core/displayNumber';
import { ENTITY_KIND_LABEL, type Entity } from '../model/entities';
import { COGO_ARC, COGO_LENGTH, COGO_RADIUS, COGO_SEMT, cogoCheck, cogoMeasure, type CogoField, type CogoFinding } from '../model/ops/cogo';
import { entitiesSet } from '../product/entitiesSet';
import { geometryOf } from '../product/entitiesEdit';
import type { AppContext } from './context';

/**
 * Kayıtlı ölçüler (docs/adr/0180; the desktop's `cogo.rs`): the recorded measurements of the drawing's lines and arcs
 * checked against the drawing (`cogo.check`) and written from it (`cogo.update`), by the core's rules
 * (`model/ops/cogo.ts`).
 */

export const COGO_STATUS_WORDS = { ok: 'Uyuyor', differs: 'Farklı', unreadable: 'Okunamadı' } as const;
export const COGO_FIELD_WORDS: Record<CogoField, string> = { semt: 'Semt', length: 'Uzunluk', radius: 'Yarıçap', arc: 'Yay uzunluğu' };

/** An object with recorded values, and what checking them found. */
export interface CogoRow {
  readonly entity: Entity;
  readonly finding: CogoFinding;
}

const isEdge = (e: Entity) => e.kind === 'line' || e.kind === 'arc';

/** The lines and arcs (every one, or the selected ones) that have recorded values, checked in the drawing's order. */
export function cogoRows(ctx: AppContext, selectionOnly: boolean, tolerances: { length: number; cc: number }): CogoRow[] {
  const { doc } = ctx;
  const ids = selectionOnly ? ctx.selection.ids.value : null;
  const out: CogoRow[] = [];
  for (const e of doc.all()) {
    if (!isEdge(e) || (ids && !ids.has(e.id))) continue;
    const finding = cogoCheck(geometryOf(e as never), e.attrs, tolerances);
    if (finding) out.push({ entity: e, finding });
  }
  return out;
}

/** “12 nesne denetlendi: 10 uyuyor, 1 farklı, 1 okunamadı.” */
export function cogoSummary(rows: readonly CogoRow[]): string {
  if (!rows.length) return 'Kayıtlı ölçüsü olan çizgi ya da yay yok.';
  const count = (s: CogoFinding['status']) => rows.filter((r) => r.finding.status === s).length;
  const parts = [
    [count('ok'), 'uyuyor'],
    [count('differs'), 'farklı'],
    [count('unreadable'), 'okunamadı'],
  ] as const;
  return `${rows.length} nesne denetlendi: ${parts
    .filter(([n]) => n > 0)
    .map(([n, w]) => `${n} ${w}`)
    .join(', ')}.`;
}

/** A recorded value's words: Durum, Katman, Tür, Ölçü, Kayıtlı, Çizimden, Fark (cc for the semt, mm for lengths). */
export function cogoItemWords(ctx: AppContext, row: CogoRow, i: number, decimalComma = false): string[] {
  const item = row.finding.items[i];
  const n = (v: number, d: number) => (decimalComma ? fixed(v, d).replace('.', ',') : fixed(v, d));
  const semt = item.field === 'semt';
  return [
    item.difference === undefined ? COGO_STATUS_WORDS.unreadable : item.over ? COGO_STATUS_WORDS.differs : COGO_STATUS_WORDS.ok,
    ctx.doc.layers.path(row.entity.layerId),
    ENTITY_KIND_LABEL[row.entity.kind],
    COGO_FIELD_WORDS[item.field],
    item.recorded,
    semt ? n(item.measured, 4) : n(item.measured, 3),
    item.difference === undefined ? '' : semt ? `${n(item.difference, 0)} cc` : `${n(item.difference * 1000, 1)} mm`,
  ];
}

export const COGO_REPORT_HEADER = ['Durum', 'Katman', 'Tür', 'Ölçü', 'Kayıtlı', 'Çizimden', 'Fark'] as const;

/**
 * Kayıtlı ölçüleri çizimden yaz (`cogo.update`): the selected lines and arcs take their measured values as their
 * recorded ones (semt with 4 decimals, lengths with 3; an arc's radius and arc length too), one undo step “Kayıtlı
 * ölçüleri yaz”; one on a locked layer is left out and said. Whether anything was written.
 */
export function cogoUpdate(ctx: AppContext, ids: Iterable<number>): boolean {
  const { doc, log } = ctx;
  const edges = [...ids].map((id) => doc.get(id)).filter((e): e is Entity => !!e && isEdge(e));
  if (!edges.length) return (log.warn('Kayıtlı ölçüsü çizimden yazılacak çizgi ya da yay seçin.'), false);
  const locked = edges.filter((e) => doc.layers.isLocked(e.layerId));
  const free = edges.filter((e) => !doc.layers.isLocked(e.layerId));
  let written = 0;
  if (free.length)
    try {
      doc.transact('Kayıtlı ölçüleri yaz', () => {
        for (const e of free) {
          const m = cogoMeasure(geometryOf(e as never));
          const uid = doc.uidOf(e.id);
          if (!m || uid === undefined) continue;
          const attrs: Record<string, string> = { [COGO_SEMT]: fixed(m.semt, 4), [COGO_LENGTH]: fixed(m.length, 3) };
          if (m.radius !== undefined) attrs[COGO_RADIUS] = fixed(m.radius, 3);
          if (m.arc !== undefined) attrs[COGO_ARC] = fixed(m.arc, 3);
          const r = entitiesSet.execute({ doc }, { uids: [uid], attrs, operation: 'attributes' });
          if (r.status !== 'completed') throw new Error('error' in r ? r.error.message : 'Kayıtlı ölçüler yazılamadı.');
          written++;
        }
      });
    } catch (e) {
      return (log.warn(e instanceof Error ? e.message : String(e)), false);
    }
  if (written) log.success(`${written} nesnenin kayıtlı ölçüleri çizimden yazıldı.`);
  if (locked.length) log.warn(`${locked.length} nesne kilitli katmanda: kayıtlı ölçüleri yazılmadı. Kilidini Katmanlar panelinden açın.`);
  return written > 0;
}
