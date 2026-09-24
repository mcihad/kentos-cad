import { rgb, solid } from '../dsl';
import { SYMBOL, code, freeDots, gridDots, picto, sections } from './common';

/**
 * EK-1a (s.7) › Kentsel çalışma alanları, sosyal altyapı alanları and
 * afet hizmet alanları: the common items of those groups, every plan level
 * (EK-1e s.93, 136–137, 192). Sokak hayvanları barınağı has EK-1e rows for
 * UİP and NİP only.
 */

export const CALISMA = sections(['Kentsel çalışma alanları'], 'calisma', 1070, (s) => {
  s.area('tuzla-alani', 'Tuzla alanı', [solid(rgb(184, 242, 248)), gridDots(5, 0.4)], {
    ref: 'EK-1a s.7; EK-1e s.93',
    note: 'Alan 184/242/248; 5×5 mm karolaj merkezlerinde 0.4 mm nokta (EK-1a koyu gri çizer; EK-1e siyah). Sembol yok.',
  });
});

export const SOSYAL = sections(['Sosyal altyapı alanları'], 'sosyal', 1080, (s, lv) => {
  // "Serbest noktalama": no density given; EK-1a shows about 90 dots per cm², one per 1.05 mm cell.
  s.area('millet-bahcesi', 'Millet bahçesi', [solid(rgb(36, 156, 34)), freeDots(0.3, 1.05), code(lv, 'MB', { font: 'serif', weight: 700, heavy: true })], {
    ref: 'EK-1a s.7; EK-1e s.137',
    note: 'Alan 36/156/34; 0.3 mm serbest noktalama (yoğunluk verilmemiş: EK-1a örneğindeki gibi cm² başına yaklaşık 90 nokta). Sembol: kalın çerçevede kalın Times "MB" (EK-1a; EK-1e düz yazı çizer).',
  });
  if (lv !== 'cdp')
    s.area('sokak-hayvanlari-barinagi-alani', 'Sokak hayvanları barınağı alanı', [solid(rgb(154, 207, 25)), gridDots(10, 1.2), picto(lv, 'hayvan-barinagi')], {
      ref: 'EK-1a s.7; EK-1e s.192',
      note: 'Alan 154/207/25; 10 mm karolaj merkezlerinde 1.2 mm nokta. Sembol: çerçevede köpek kulübesi. EK-1e yalnızca UİP ve NİP satırı verir.',
    });
});

export const AFET = sections(['Afet hizmet alanları'], 'afet', 1090, (s, lv) => {
  s.area('afet-arama-kurtarma-hizmet-lojistik-merkezi', 'Afet arama kurtarma hizmet lojistik merkezi', [solid(rgb(221, 221, 221)), picto(lv, 'can-simidi', { scale: 0.86, heavy: true })], {
    ref: 'EK-1a s.7; EK-1e s.136',
    note: `Alan 221/221/221. Sembol: kalın çerçevede can simidi (yeşil 84/130/53, kırmızı 255/0/0, "A-K-L"); "ilgili alana uygun büyüklükte": ${SYMBOL[lv]} mm (AÇIKLAMA 6). EK-1e sembolün altına RGB 99/186/82 ve 224/0/33 yazar; çizim EK-1a'daki renklerle yapıldı.`,
  });
});
