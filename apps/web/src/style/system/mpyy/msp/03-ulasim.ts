import { BLACK, circle, rgb, sheet, solid, stroke, svg } from '../dsl';
import { pic } from '../pictograms';

/**
 * MSP (EK-1e bölüm I, s.3-5) › Ana ulaşım sistemi: koridorlar (%50 saydam
 * alanlar), ulaşım odağı, karayolu ve demiryolu ağları, limanlar,
 * havalimanları, lojistik merkezler.
 */

export const ulasim = sheet('msp', ['Ana ulaşım sistemi'], 'ulasim', 30);

/**
 * The rounded tiles are whole pictograms (tile and white figure); the
 * pictogram's viewBox keeps about 6 % margin on each side, so an 11.4 mm
 * drawing gives the ≈10 mm tile measured in the sample.
 */
const TILE = 11.4;

ulasim.area('uluslararasi-ve-ulusal-ulasim-koridorlari', 'Uluslararası ve ulusal ulaşım koridorları', [solid(rgb(240, 220, 220, 0.5))], { ref: 'EK-1e s.3', note: '240/220/220, %50 saydam. Sınır verilmemiş.' });
ulasim.area('bolgelerarasi-ulasim-koridorlari', 'Bölgelerarası ulaşım koridorları', [solid(rgb(215, 215, 215, 0.5))], { ref: 'EK-1e s.3', note: '215/215/215, %50 saydam. Sınır verilmemiş.' });
ulasim.point('ulasim-odagi', 'Ulaşım odağı', [circle(5.7, { fill: rgb(255, 0, 0), stroke: BLACK, strokeWidth: 0.35 })], {
  ref: 'EK-1e s.4',
  note: 'Kırmızı (255/0/0) daire, ince siyah dış çizgi; 5,7 mm çap çizimden.',
});
ulasim.line('karayolu-agi', 'Karayolu ağı', [stroke(BLACK, 0.7)], { ref: 'EK-1e s.4', note: '0,7 mm siyah düz çizgi.' });
ulasim.line('demiryolu-agi', 'Demiryolu ağı', [stroke(rgb(255, 0, 0), 0.7)], { ref: 'EK-1e s.4', note: '0,7 mm kırmızı (255/0/0) düz çizgi.' });
ulasim.point('limanlar', 'Limanlar', [svg(pic('msp-liman'), TILE, { fill: BLACK })], { ref: 'EK-1e s.5', note: 'Siyah yuvarlak köşeli karede beyaz çapa; kare ≈10 mm (çizimden).' });
ulasim.point('havalimanlari', 'Havalimanları', [svg(pic('msp-havalimani'), TILE, { fill: BLACK })], { ref: 'EK-1e s.5', note: 'Siyah yuvarlak köşeli karede beyaz uçak; kare ≈10 mm (çizimden).' });
ulasim.point('lojistik-merkezler', 'Lojistik merkezler', [svg(pic('msp-lojistik'), TILE)], {
  ref: 'EK-1e s.5',
  note: 'Kırmızı (255/0/0) yuvarlak köşeli karede beyaz kalın "L" harfi; kare ≈10 mm (çizimden).',
});
