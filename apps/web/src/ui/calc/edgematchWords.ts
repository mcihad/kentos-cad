import type { Entity } from '../../model/entities';
import type { EdgeFound, EdgeMeet, EdgeMethod } from '../../model/ops/edgematch';
import { mmText, summaryLine } from './common';

/**
 * Kenar eşleme's words (docs/adr/0159 §9): the choices with their names and
 * hints, an object as the links table names it, the summary under the table
 * and the report. The desktop's are apps/desktop/src/calc/edgematch/words.rs.
 */

export const TITLE = 'Kenar eşleme';

export const MEETS: { value: EdgeMeet; label: string }[] = [
  { value: 'adjacent', label: 'Komşunun ucunda' },
  { value: 'middle', label: 'Ortada' },
  { value: 'border', label: 'Sınırda' },
];

export const MEET_HINT: Record<EdgeMeet, string> = {
  adjacent: 'Komşu pafta yerinde kalır; kaynak ucu komşunun ucuna gider.',
  middle: 'İki uç aralarının ortasında buluşur; komşu çizgi de düzeltilir.',
  border: 'İki uç sınırın, ortalarına en yakın noktasında buluşur; komşu çizgi de düzeltilir.',
};

export const METHODS: { value: EdgeMethod; label: string }[] = [
  { value: 'move', label: 'Ucu taşı' },
  { value: 'segment', label: 'Parça ekle' },
  { value: 'adjust', label: 'Köşeleri ayarla' },
];

export const METHOD_HINT: Record<EdgeMethod, string> = {
  move: 'Yalnız uç taşınır; ucun kenarı doğru kalır.',
  segment: 'Uç yerinde kalır, buluşma yerine düz bir parça eklenir; çizgi çoklu çizgi olur.',
  adjust: 'Uçtaki kayma çizgi boyunca öbür uca doğru azalarak dağıtılır.',
};

/** The criterion that compares the layers' names (an attribute's name cannot hold this character). */
export const LAYER_KEY = '\u0000katman';

const KIND: Partial<Record<Entity['kind'], string>> = { line: 'çizgi', polyline: 'çoklu çizgi' };

/** An object as the links table names it: its layer, then its label, its first attribute's value or its kind. */
export function describe(e: Entity, layer: string): string {
  const named = e.label?.trim() || Object.values(e.attrs ?? {}).find((v) => typeof v === 'string' && v.trim())?.trim();
  return `${layer}: ${named || KIND[e.kind] || e.kind}`;
}

/** What the summary says: why nothing can be found, or what was found and what was left out. */
export interface Summary {
  problem: string | null;
  found: EdgeFound | null;
  used: number;
  worst: { gap: number; who: string } | null;
  mean: number | null;
  /** Objects of other kinds and closed or empty paths among the sources. */
  others: number;
  locked: number;
  /** The table's rows (1-based) of links that cannot be written. */
  refused: number[];
  bordered: boolean;
}

export function summaryLines(s: Summary): HTMLElement[] {
  if (s.problem) return [summaryLine('warn', s.problem)];
  const f = s.found;
  if (!f) return [];
  const lines: HTMLElement[] = [];
  if (!f.links.length)
    lines.push(
      summaryLine('info', `Arama uzaklığında devamı bulunan uç yok. Arama uzaklığını ya da açı toleransını büyütün${s.bordered ? '; uçlar sınıra yakın olmalı' : ''}.`),
    );
  else lines.push(summaryLine('ok', `${f.links.length} bağ bulundu; ${s.used} bağ kullanılacak.`));
  if (s.worst && s.mean !== null) lines.push(summaryLine('info', `En büyük aralık ${mmText(s.worst.gap)}: ${s.worst.who}; ortalama ${mmText(s.mean)}.`));
  if (f.unmatched.length) lines.push(summaryLine('info', `${f.unmatched.length} uç eşsiz kaldı: ${s.bordered ? 'sınıra' : 'komşu bir uca'} yakın ama devamı bulunamadı.`));
  if (f.junctions) lines.push(summaryLine('info', `${f.junctions} kavşak ucu eşlenmedi; yalnız biri taşınırsa kavşak bozulurdu.`));
  if (s.others) lines.push(summaryLine('info', `${s.others} nesne katılmadı: yalnız çizgiler ve açık çoklu çizgiler eşlenir.`));
  if (s.locked) lines.push(summaryLine('info', `${s.locked} nesne kilitli katmanda olduğu için katılmadı.`));
  if (s.refused.length)
    lines.push(summaryLine('warn', `${s.refused.length} bağ yazılamaz (${s.refused.join(', ')}. satır): nesnesinde sıfır boylu kenar kalırdı. Kullan'dan çıkarın ya da başka bir yöntem seçin.`));
  return lines;
}
