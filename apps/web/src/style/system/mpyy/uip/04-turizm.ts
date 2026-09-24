import { BLACK, WHITE, circle, label, rgb, shape, sheet, solid, text } from '../dsl';
import { FRAME_LINE, code, picto, speckleBlobs } from './common';

/**
 * UİP (EK-1d s.5–6) › Turizm alanları. Every item has the same hatch
 * (EK-1e s.71–77): 0.3 mm, 6 mm serbest noktalama at the centres of an
 * 18 mm karolaj, staggered rows.
 */

export const turizm = sheet('uip', ['Turizm alanları'], 'turizm', 50);

const TURIZM = rgb(255, 115, 0);
const GUNUBIRLIK = rgb(255, 173, 0);
const HATCH = 'Tarama: 0.3 mm, 6 mm çapında serbest noktalama, 18 mm karolaj merkezlerinde, şaşırtmalı sıra (iki ekin çiziminde de sıralar 9 mm aralı).';
const blobs = () => speckleBlobs(6, 18);

turizm.area('pansiyon-alani', 'Pansiyon alanı', [solid(TURIZM), blobs(), code('PA', 5, { weight: 900 })], { ref: 'EK-1d s.5; EK-1e s.71', note: HATCH });
turizm.area('apart-otel-alani', 'Apart otel alanı', [solid(TURIZM), blobs(), code('AO', 5, { weight: 900 })], { ref: 'EK-1d s.5; EK-1e s.72', note: HATCH });
turizm.area('otel-alani', 'Otel alanı', [solid(TURIZM), blobs(), code('OTEL', 2.8)], { ref: 'EK-1d s.5; EK-1e s.72', note: HATCH });
turizm.area('motel-alani', 'Motel alanı', [solid(TURIZM), blobs(), code('MOT', 4.1)], { ref: 'EK-1d s.5; EK-1e s.73', note: HATCH });
turizm.area('hostel-alani', 'Hostel alanı', [solid(TURIZM), blobs(), code('HOS', 4.1)], { ref: 'EK-1d s.5; EK-1e s.73', note: HATCH });
turizm.area('tatil-koyu-alani', 'Tatil köyü alanı', [solid(TURIZM), blobs(), code('TKA', 4.2)], { ref: 'EK-1d s.6; EK-1e s.74', note: HATCH });
turizm.area('saglik-odakli-tatil-koyu', 'Sağlık odaklı tatil köyü', [solid(TURIZM), blobs(), code('SOTK', 2.8, { font: 'serif' })], { ref: 'EK-1d s.6; EK-1e s.74', note: HATCH });
turizm.area('termal-turizm-alani', 'Termal turizm alanı', [solid(TURIZM), blobs(), code('TTA', 4.2)], {
  ref: 'EK-1d s.6; EK-1e s.75',
  note: `${HATCH} Renk EK-1d'deki gibi 255/115/0; EK-1e'nin RGB hücresinde 255/173/0 yazıyor ama rengi 255/115/0 boyalı.`,
});
turizm.area('kamping-alani', 'Kamping alanı', [solid(GUNUBIRLIK), blobs(), picto('cadir', { size: 9 })], { ref: 'EK-1d s.6; EK-1e s.75', note: HATCH });
turizm.area('gunubirlik-tesis-alani', 'Günübirlik tesis alanı', [solid(GUNUBIRLIK), blobs(), code('G', 5, { weight: 900, frame: 'rect', size: 11.5, height: 10, strokeWidth: 0.5 })], {
  ref: 'EK-1d s.6; EK-1e s.76',
  note: `${HATCH} Çerçeve EK-1d'deki gibi 11.5×10 mm kalın dikdörtgen.`,
});
turizm.area(
  'golf-turizm-tesis-alani',
  'Golf turizm tesis alanı',
  [solid(rgb(161, 194, 114)), blobs(), label([circle(12.5, { fill: WHITE, stroke: BLACK, strokeWidth: 0.5 }), text('GTT', 4, { font: 'sans', weight: 400 })])],
  {
    ref: 'EK-1d s.6; EK-1e s.57 (Golf turizmi)',
    note: `${HATCH} Sembol: 12.5 mm kalın daire içinde "GTT" (EK-1d). EK-1e "ölçeğine göre sembol" diyerek GTB, GTA, GTT verir; UİP lejantı GTT kullanır.`,
  },
);
turizm.area('kis-sporlari-ve-kayak-tesisi-alani', 'Kış sporları ve kayak tesisi alanı', [solid(TURIZM), blobs(), picto('kayakci', { size: 9 })], { ref: 'EK-1d s.6; EK-1e s.76', note: HATCH });
turizm.area(
  'ekoturizm-kirsal-turizm-tesis-alani',
  'Ekoturizm / kırsal turizm tesis alanı',
  [solid(TURIZM), blobs(), label([shape('rectangle', 15, { height: 14, fill: WHITE, stroke: BLACK, strokeWidth: FRAME_LINE }), text('ET/K', 4.8, { font: 'sans', weight: 700, offset: [0, 1.4] })])],
  { ref: 'EK-1d s.6; EK-1e s.77 (Ekoturizm/kırsal turizm)', note: `${HATCH} Çerçeve EK-1d'deki gibi 15×14 mm, yazı üst yarıda.` },
);
