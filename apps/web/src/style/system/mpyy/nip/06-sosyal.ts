import { BLACK, WHITE, framed, rgb, shape, sheet, solid, text } from '../dsl';
import { S, box, gridDots, picture, stack, turned } from './common';

/**
 * NİP (EK-1ç) › Sosyal altyapı alanları: eğitim, sağlık, sosyal ve kültürel
 * tesis, spor ve ibadet alanları. Hepsi "karolaj merkezlerinde noktalama":
 * kalem kalınlığında noktalar kare ızgaranın hücre ortalarında (EK-1e
 * s.116-130). AÇIKLAMA 15: özele dönüşen alanlarda kullanımın başına "ÖZEL",
 * açık/kapalı spor alanlarında "Ö" eklenir.
 */

const EGITIM = rgb(0, 143, 255);
const SAGLIK = rgb(0, 169, 230);
const TESIS = rgb(115, 212, 255);
const SPOR = rgb(137, 205, 102);

export const sosyal = sheet('nip', ['Sosyal altyapı alanları'], 'sosyal', 60);

sosyal.area('egitim-alani', 'Eğitim alanı', [solid(EGITIM), gridDots(5, 0.4), stack(shape('triangle', S, { fill: WHITE, stroke: BLACK, strokeWidth: 0.7 }))], {
  ref: 'EK-1ç s.6 (AÇIKLAMA 8); EK-1e s.116',
  note: '0,4 mm, 5 mm karolaj merkezlerinde noktalama. Sembol çerçevesiz kalın çizgili eşkenar üçgen (7 mm).',
});
sosyal.area('yuksek-ogretim-alani', 'Yüksek öğretim alanı', [solid(EGITIM), gridDots(8, 0.4), box('Ü', { max: 4.4 })], {
  ref: 'EK-1ç s.6; EK-1e s.120',
  note: '0,4 mm, 8 mm karolaj merkezlerinde noktalama.',
});
sosyal.area('saglik-alani', 'Sağlık alanı', [solid(SAGLIK), gridDots(4, 0.4), stack(framed('', { frame: 'double', size: S, strokeWidth: 0.3, background: WHITE }))], {
  ref: 'EK-1ç s.6 (AÇIKLAMA 9); EK-1e s.121',
  note: '0,4 mm, 4 mm karolaj merkezlerinde noktalama. Sembol iç içe iki kare.',
});

/** Two outline diamonds overlapping, their common part filled, in a frame (EK-1ç s.6). */
const sosyalTesis = stack([
  shape('rectangle', S, { height: 5.2, fill: WHITE, stroke: BLACK, strokeWidth: 0.25 }),
  shape('diamond', 3.6, { stroke: BLACK, strokeWidth: 0.35, offset: [-0.95, 0] }),
  shape('diamond', 3.6, { stroke: BLACK, strokeWidth: 0.35, offset: [0.95, 0] }),
  shape('diamond', 1.7, { fill: BLACK }),
]);
sosyal.area('sosyal-tesis-alani', 'Sosyal tesis alanı', [solid(TESIS), gridDots(4, 0.3), sosyalTesis], {
  ref: 'EK-1ç s.6; EK-1e s.124',
  note: '0,3 mm, 4 mm karolaj merkezlerinde noktalama. Sembol: yatay dikdörtgen çerçevede kesişen iki boş eşkenar dörtgen, ortak parçası dolu (çerçeve oranı EK-1ç çiziminden).',
});
sosyal.area('kulturel-tesis-alani', 'Kültürel tesis alanı', [solid(TESIS), gridDots(4, 0.4), picture('kitap')], {
  ref: 'EK-1ç s.6; EK-1e s.124',
  note: '0,4 mm, 4 mm karolaj merkezlerinde noktalama.',
});
sosyal.area('spor-alani', 'Spor alanı', [solid(SPOR), gridDots(4, 0.4), picture('bayrak-bos')], {
  ref: 'EK-1ç s.6; EK-1e s.125',
  note: '0,4 mm, 4 mm karolaj merkezlerinde noktalama. Özel spor alanında kullanıma "Ö" eklenir (AÇIKLAMA 15).',
});
sosyal.area('ozel-sosyal-altyapi-alani', 'Özel sosyal altyapı alanı', [solid(SAGLIK), gridDots(4, 0.4), box('Ö', { max: 4.4 })], {
  ref: 'EK-1ç s.6 (AÇIKLAMA 15); EK-1e s.125',
  note: '0,4 mm, 4 mm karolaj merkezlerinde noktalama. Renk sağlık alanının mavisi (0/169/230).',
});
sosyal.area('ibadet-alani', 'İbadet alanı', [solid(TESIS), gridDots(4, 0.4), box('İA', { max: 4 })], {
  ref: 'EK-1ç s.6 (AÇIKLAMA 10); EK-1e s.130',
  note: '0,4 mm, 4 mm karelaj merkezlerinde noktalama. AÇIKLAMA 10: mevcut ibadet alanlarında UİP sembollerinden uygun olanı, yeni alanlarda cami ve mescit sembolleri kullanılabilir.',
});

/**
 * A flag in a heavy frame with "SSA" in its pennant (EK-1ç s.6): pole on
 * the left, an outline pennant pointing right from its top.
 */
const semtSpor = stack([
  shape('square', S, { fill: WHITE, stroke: BLACK, strokeWidth: 0.5 }),
  shape('line', 4.6, { stroke: BLACK, strokeWidth: 0.25, rotation: 90, offset: turned(-2, -0.3, 90) }),
  shape('arrowhead', 4.4, { height: 2.2, stroke: BLACK, strokeWidth: 0.2, offset: [0.2, 1.1] }),
  text('SSA', 0.9, { font: 'sans', weight: 700, offset: [-0.85, 1.12] }),
]);
sosyal.area('semt-spor-alani', 'Semt spor alanı', [solid(SPOR), gridDots(7, 1.2), semtSpor], {
  ref: 'EK-1ç s.6; EK-1e s.126',
  note: '1,2 mm, 7 mm karolaj merkezlerinde noktalama (EK-1e NİP satırı; EK-1ç örneği ince noktalarla basılmış). Sembol kalın çerçevede bayrak, flamasında "SSA".',
});
