import type { RubberLink } from '../../contracts/generated/RubberLink';
import { fixed } from '../../core/displayNumber';
import type { Fit, FitFailure, FitPair } from '../../model/ops/fit';
import { rubberSheet, type RubberFailure } from '../../model/ops/rubber';
import { mmText, summaryLine } from './common';

/**
 * Kauçuk levha in Vektör oturtma (docs/adr/0158 §5): the used pairs are the
 * sheet's links, a pair whose target is its source a fixed point (Sabit);
 * the core says whether they give a sheet (`rubberSheet`). The table's
 * residuals are Helmert's: at each link, the local correction the sheet
 * makes beyond the best similarity. The summary, the report and the reasons
 * there is no sheet. The desktop's is apps/desktop/src/calc/fit/rubber.rs.
 */

export const RUBBER_HINT = 'Yerel düzeltme: her bağ hedefine tam oturur, çevresi yakınlığıyla kayar; en az 3 bağ. Köşeler taşınır, kenarlar doğru kalır.';

/** The least links a sheet takes. */
const NEED = 3;

/** The used pairs as links, how many are fixed points, and why they give no sheet (null when they give one). */
export interface Links {
  links: RubberLink[];
  fixed: number;
  error: RubberFailure['error'] | null;
}

export function linksOf(pairs: readonly FitPair[]): Links {
  const links = pairs.filter((p) => p.used).map((p) => ({ from: p.source, to: p.target }));
  const fixed = links.filter((l) => l.from.x === l.to.x && l.from.y === l.to.y).length;
  const answer = links.length < NEED ? null : rubberSheet(links, []);
  return { links, fixed, error: answer === null ? 'too_few' : 'error' in answer ? answer.error : null };
}

const FAILURE: Record<Exclude<RubberFailure['error'], 'too_few'>, string> = {
  duplicate: "İki bağın kaynağı aynı nokta. Birini Kullan'dan çıkarın.",
  collinear: 'Bağların kaynakları bir doğru üstünde; levha kurulamaz. Doğrunun dışında bir bağ ekleyin.',
  singular: 'Bağların denklem takımının tek çözümü yok. Birbirine çok yakın kaynakları birleştirin.',
  too_many: 'En çok 1000 bağ alınır. Bağları azaltın.',
};

/** The local corrections at the used links (Helmert's residuals): the largest and whose, and their mean (metres). */
export interface Corrections {
  worst: number;
  who: string;
  mean: number;
}

/** The summary under the table: the links and that the sheet meets them, the local corrections and Helmert's m0; or why there is no sheet. */
export function rubberSummary(l: Links, fit: Fit | FitFailure | null, corrections: Corrections | null): HTMLElement[] {
  if (l.error === 'too_few')
    return [summaryLine('info', `Kauçuk levha için en az ${NEED} kullanılan bağ gerekir; şimdi ${l.links.length}. Koordinatları yazın, yapıştırın ya da çizimden seçin.`)];
  if (l.error) return [summaryLine('warn', FAILURE[l.error])];
  const lines = [summaryLine('ok', `${l.links.length} bağ (${l.fixed} sabit nokta); levha her bağdan tam geçer.`)];
  if (fit && !('error' in fit) && fit.m0 !== null && corrections) {
    lines.push(summaryLine('info', `Yerel düzeltme (Helmert'e göre) en çok ${mmText(corrections.worst)}: ${corrections.who}; ortalama ${mmText(corrections.mean)}, Helmert m0 = ±${mmText(fit.m0)}.`));
    lines.push(summaryLine('info', "Levha hatalı bir bağı da tam geçer: ölçü hatası olan bağı Kullan'dan çıkarın."));
  }
  return lines;
}

/**
 * Kauçuk levha's report, tab-separated: the method, the pairs with their local corrections (`rows`, as the table
 * has them), the links, the sheet or why there is none, the corrections and Helmert's m0 and parameters.
 */
export function rubberReport(title: string, rows: string[][], l: Links, fit: Fit | FitFailure | null, corrections: Corrections | null, parameters: string | null): string[][] {
  const lines: string[][] = [
    [title, 'kauçuk levha'],
    ['Yöntem', "İnce plaka eğrisi: levha her bağdan tam geçer; vY, vX ve v Helmert'e göre yerel düzeltmedir."],
    ['Kullan', 'Ad', 'Kaynak Y', 'Kaynak X', 'Hedef Y', 'Hedef X', 'vY (mm)', 'vX (mm)', 'v (mm)'],
    ...rows,
    ['Bağ', String(l.links.length), 'Sabit nokta', String(l.fixed)],
  ];
  if (l.error) lines.push(['Levha', l.error === 'too_few' ? `En az ${NEED} kullanılan bağ gerekir.` : FAILURE[l.error]]);
  if (corrections) {
    lines.push(['En büyük yerel düzeltme (mm)', fixed(corrections.worst * 1000, 1), corrections.who]);
    lines.push(['Ortalama yerel düzeltme (mm)', fixed(corrections.mean * 1000, 1)]);
  }
  if (fit && !('error' in fit)) {
    lines.push(['Helmert m0 (mm)', fit.m0 === null ? '—' : fixed(fit.m0 * 1000, 2)]);
    if (parameters) lines.push(['Helmert', parameters]);
  }
  return lines;
}
