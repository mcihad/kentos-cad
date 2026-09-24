import { BLACK, WHITE, label, rgb, shape, sheet, solid, text } from '../dsl';
import { FRAME, FRAME_LINE, captionMark, code, gridDots, picto } from './common';

/**
 * UİP (EK-1d s.9–12) › Sosyal altyapı alanları: eğitim, sağlık, sosyal ve
 * kültürel tesisler, ibadet alanları. Hatches are EK-1e's "karolaj
 * merkezlerinde noktalama" (dot = pen width); symbols as EK-1d draws them.
 * AÇIKLAMA 8: when such a use is turned private, "ÖZEL" goes before the
 * use's name and "Ö" is added to the symbol.
 */

/** The legend's top section; its children carry the items. */
export const sosyalAltyapi = sheet('uip', ['Sosyal altyapı alanları'], 'sosyal-altyapi', 70);

export const egitim = sheet('uip', ['Sosyal altyapı alanları', 'Eğitim tesisleri alanı'], 'egitim', 10);

const EGITIM = rgb(0, 143, 255);
/** School triangles: equilateral, 10.5 mm side (EK-1d), caption in Times under the base. */
const TRI = 10.5;
/** An outline triangle of `side`, centred on its centroid. */
const tri = (side: number, strokeWidth: number) => shape('triangle', side, { stroke: BLACK, strokeWidth });
const triCaption = (c: string) => text(c, 3.8, { font: 'serif', weight: 400, offset: [0, -(TRI * Math.sqrt(3)) / 6 - 2.75], halo: { color: WHITE, width: 0.4 } });
const thin = (c: string) => label([tri(TRI, 0.3), triCaption(c)]);
const doubled = (c: string) => label([tri(TRI, 0.3), tri(TRI - 2 * 0.7 * Math.sqrt(3), 0.3), triCaption(c)]);
// The stroke is centred on the path (mitered): the path is drawn smaller so the outer edge stays 10.5 mm.
const thick = (c: string) => label([tri(TRI - 1.2 * Math.sqrt(3), 1.2), triCaption(c)]);
const TRI_NOTE = 'EK-1d örneğinin noktaları daha küçük ve sık; metin esas. Sembol: 10.5 mm eşkenar üçgen (EK-1d): anaokulu, ilkokul ve özel eğitimde ince, ortaokulda çift, lise ve meslek liselerinde kalın çizgili; altında Times yazı (EK-1e kalın Arial diye tarif eder, lejantın görünüşü esas alındı).';

egitim.area('anaokulu-alani', 'Anaokulu alanı', [solid(EGITIM), gridDots(5, 0.8), thin('ANA')], { ref: 'EK-1d s.9; EK-1e s.117', note: `5 mm karolaj merkezlerinde 0.8 mm noktalama. ${TRI_NOTE}` });
egitim.area('ilkokul-alani', 'İlkokul alanı', [solid(EGITIM), gridDots(5, 0.8), thin('İLK')], { ref: 'EK-1d s.9; EK-1e s.117', note: `5 mm karolaj merkezlerinde 0.8 mm noktalama. ${TRI_NOTE}` });
egitim.area('ortaokul-alani', 'Ortaokul alanı', [solid(EGITIM), gridDots(5, 0.8), doubled('ORTA')], { ref: 'EK-1d s.9; EK-1e s.118', note: `5 mm karolaj merkezlerinde 0.8 mm noktalama. ${TRI_NOTE} Çift çizgiler 0.7 mm aralı.` });
egitim.area('lise-alani', 'Lise alanı', [solid(EGITIM), gridDots(5, 0.8), thick('LİSE')], { ref: 'EK-1d s.9; EK-1e s.118', note: `5 mm karolaj merkezlerinde 0.8 mm noktalama. ${TRI_NOTE} Kalın çizgi 1.2 mm.` });
egitim.area('ozel-egitim-alani', 'Özel eğitim alanı', [solid(EGITIM), gridDots(5, 0.8), thin('Ö')], {
  ref: 'EK-1d s.9; EK-1e s.119',
  note: `5 mm karolaj merkezlerinde 0.8 mm noktalama. ${TRI_NOTE} AÇIKLAMA 7: ilkokul, ortaokul, lise birlikte ya da ayrı yapılabilir; eğitim tesisi türünün uygun sembolü işaretlenir.`,
});
egitim.area('mesleki-ve-teknik-ogretim-tesisi-alani', 'Mesleki ve teknik öğretim tesisi alanı', [solid(EGITIM), gridDots(5, 0.8), thick('MESLEK')], {
  ref: 'EK-1d s.10; EK-1e s.119',
  note: `5 mm karolaj merkezlerinde 0.8 mm noktalama. ${TRI_NOTE}`,
});
egitim.area('yuksek-ogretim-tesisi-alani', 'Yüksek öğretim tesisi alanı', [solid(EGITIM), gridDots(10, 0.8), code('Ü', 5)], {
  ref: 'EK-1d s.10; EK-1e s.120 (Yüksek öğretim alanı)',
  note: '10 mm karolaj merkezlerinde 0.8 mm noktalama (EK-1d örneği daha sık; metin esas). EK-1d adı "… tesisi alan" diye yazar.',
});
egitim.area('halk-egitim-merkezi', 'Halk eğitim merkezi', [solid(EGITIM), gridDots(5, 0.8), code('HEM', 3.2, { font: 'serif' })], { ref: 'EK-1d s.10; EK-1e s.120', note: '5 mm karolaj merkezlerinde 0.8 mm noktalama. EK-1d örneği daha küçük ve sık noktalar gösteriyor; metin esas alındı.' });

export const saglik = sheet('uip', ['Sosyal altyapı alanları', 'Sağlık tesisleri alanı'], 'saglik', 20);

const SAGLIK = rgb(0, 169, 230);
/** Two concentric squares (the inner 60 %), caption under them. */
const health = (c: string) => label([shape('square', FRAME, { fill: WHITE, stroke: BLACK, strokeWidth: 0.4 }), shape('square', FRAME * 0.6, { stroke: BLACK, strokeWidth: 0.3 }), captionMark(c)]);
const HEALTH_NOTE = '7 mm karolaj merkezlerinde 0.8 mm noktalama (EK-1d örneği daha küçük ve sık; metin esas). Sembol: iç içe iki kare (içteki %60), altında yazı.';

saglik.area('saglik-tesisi-alani', 'Sağlık tesisi alanı', [solid(SAGLIK), gridDots(7, 0.8), health('S')], { ref: 'EK-1d s.10; EK-1e s.121', note: HEALTH_NOTE });
saglik.area('ozel-saglik-tesisi-alani', 'Özel sağlık tesisi alanı', [solid(SAGLIK), gridDots(7, 0.8), health('ÖZEL')], { ref: 'EK-1d s.10; EK-1e s.122', note: `${HEALTH_NOTE} AÇIKLAMA 8.` });
saglik.area('hastane', 'Hastane', [solid(SAGLIK), gridDots(7, 0.8), health('HAST')], { ref: 'EK-1d s.10; EK-1e s.122', note: HEALTH_NOTE });
saglik.area('aile-sagligi-merkezi', 'Aile sağlığı merkezi', [solid(SAGLIK), gridDots(7, 0.8), health('ASM')], { ref: 'EK-1d s.10; EK-1e s.123', note: HEALTH_NOTE });

export const sosyalKulturel = sheet('uip', ['Sosyal altyapı alanları', 'Sosyal ve kültürel tesis alanı'], 'sosyal-kulturel', 30);

const SOSYAL = rgb(115, 212, 255);
const SPOR = rgb(137, 205, 102);
const DOTS_NOTE = '7 mm karolaj merkezlerinde 1.2 mm noktalama (EK-1d örneği daha küçük ve sık noktalar gösteriyor; metin esas).';

sosyalKulturel.area(
  'sosyal-tesis-alani',
  'Sosyal tesis alanı',
  [
    solid(SOSYAL),
    gridDots(7, 1.2),
    label([
      shape('square', FRAME, { fill: WHITE, stroke: BLACK, strokeWidth: FRAME_LINE }),
      // Two outline diamonds side by side, overlapping; the overlap is filled (measured from EK-1d).
      shape('diamond', 5.8, { height: 6.2, stroke: BLACK, strokeWidth: 0.35, offset: [-1.65, 0] }),
      shape('diamond', 5.8, { height: 6.2, stroke: BLACK, strokeWidth: 0.35, offset: [1.65, 0] }),
      shape('diamond', 2.5, { height: 2.67, fill: BLACK }),
    ]),
  ],
  { ref: 'EK-1d s.10; EK-1e s.124', note: `${DOTS_NOTE} Sembol: kesişen iki eşkenar dörtgen, kesişimi dolu (şekillerden kuruldu).` },
);
sosyalKulturel.area('kulturel-tesis-alani', 'Kültürel tesis alanı', [solid(SOSYAL), gridDots(7, 1.2), picto('kitap', { size: 9 })], { ref: 'EK-1d s.10; EK-1e s.124', note: DOTS_NOTE });
sosyalKulturel.area('kres-gunduz-bakimevi', 'Kreş, gündüz bakımevi', [solid(SOSYAL), gridDots(7, 1.2), code('KREŞ', 2.6, { font: 'serif' })], { ref: 'EK-1d s.10; EK-1e s.127', note: DOTS_NOTE });
sosyalKulturel.area('acik-spor-tesisi-alani', 'Açık spor tesisi alanı', [solid(SPOR), gridDots(7, 1.2), picto('bayrak-bos', { size: 9 })], {
  ref: 'EK-1d s.10; EK-1e s.126',
  note: `${DOTS_NOTE} AÇIKLAMA 8: açık ve kapalı spor alanları "Ö" harfi eklenerek özel nitelikle yapılabilir.`,
});
sosyalKulturel.area('kapali-spor-tesisi-alani', 'Kapalı spor tesisi alanı', [solid(SPOR), gridDots(7, 1.2), picto('bayrak-dolu', { size: 9 })], { ref: 'EK-1d s.11; EK-1e s.127', note: `${DOTS_NOTE} AÇIKLAMA 8.` });
sosyalKulturel.area('yurt-alani', 'Yurt alanı', [solid(SOSYAL), gridDots(7, 1.2), picto('yurt', { size: 9 })], { ref: 'EK-1d s.11; EK-1e s.128', note: DOTS_NOTE });
sosyalKulturel.area('kongre-ve-sergi-merkezi-alani', 'Kongre ve sergi merkezi alanı', [solid(SOSYAL), gridDots(7, 1.2), code('KSM', 3.7, { weight: 900, strokeWidth: 0.5 })], { ref: 'EK-1d s.11; EK-1e s.128', note: DOTS_NOTE });
sosyalKulturel.area('yasli-bakimevi-alani', 'Yaşlı bakımevi alanı', [solid(SOSYAL), gridDots(7, 1.2), code('YBA', 4.2, { font: 'serif' })], { ref: 'EK-1d s.11; EK-1e s.129', note: DOTS_NOTE });
sosyalKulturel.area('sefkat-evleri-alani', 'Şefkat evleri alanı', [solid(SOSYAL), gridDots(7, 1.2), code('ŞEA', 4.2, { font: 'serif' })], { ref: 'EK-1d s.11; EK-1e s.129', note: DOTS_NOTE });
sosyalKulturel.area(
  'semt-spor-alani',
  'Semt spor alanı',
  [
    solid(SPOR),
    gridDots(7, 1.2),
    label([
      shape('rectangle', 14.4, { height: 11.3, fill: WHITE, stroke: BLACK, strokeWidth: 0.5 }),
      // The flag: pole on the left, outline pennant pointing right with "SSA" in it (measured from EK-1d).
      // A turned marker's offset is in its own frame: [along the pole, across it].
      shape('line', 8.3, { stroke: BLACK, strokeWidth: 0.3, rotation: 90, offset: [-0.25, 4] }),
      shape('arrowhead', 9.8, { height: 3.1, stroke: BLACK, strokeWidth: 0.3, offset: [0.9, 2.35] }),
      text('SSA', 1.6, { font: 'sans', weight: 700, offset: [-1.6, 2.45] }),
    ]),
  ],
  { ref: 'EK-1d s.11; EK-1e s.126', note: `${DOTS_NOTE} Sembol: 14.4×11.3 mm kalın çerçevede bayrak, flamada "SSA" (şekillerden kuruldu, ölçüler EK-1d'den).` },
);
sosyalKulturel.area(
  'cemevi-alani',
  'Cemevi alanı',
  [solid(rgb(102, 153, 205)), gridDots(7, 1.2), label([shape('square', 12.5, { fill: WHITE, stroke: rgb(0, 32, 96), strokeWidth: FRAME_LINE }), text('CEMEVİ', 2.8, { font: 'sans', weight: 700 })])],
  { ref: 'EK-1d s.11; EK-1e s.130', note: `${DOTS_NOTE} Alan 102/153/205 (öteki sosyal tesislerden koyu). Çerçeve EK-1d'deki gibi lacivert (0/32/96), 12.5 mm; yazı siyah.` },
);

export const ibadet = sheet('uip', ['Sosyal altyapı alanları', 'İbadet alanları'], 'ibadet', 40);

const IBADET_NOTE = '7 mm karolaj merkezlerinde 1.2 mm noktalama (EK-1d çizimi daha sık; metin esas).';
ibadet.area('cami', 'Cami', [solid(SOSYAL), gridDots(7, 1.2), picto('cami', { size: 9 })], { ref: 'EK-1d s.11; EK-1e s.131', note: IBADET_NOTE });
ibadet.area('mescit', 'Mescit', [solid(SOSYAL), gridDots(7, 1.2), picto('mescit', { size: 9 })], { ref: 'EK-1d s.11; EK-1e s.131', note: IBADET_NOTE });
ibadet.area('kilise', 'Kilise', [solid(SOSYAL), gridDots(7, 1.2), picto('hac', { size: 9 })], { ref: 'EK-1d s.11; EK-1e s.132', note: IBADET_NOTE });
ibadet.area('sapel', 'Şapel', [solid(SOSYAL), gridDots(7, 1.2), picto('hac-bos', { size: 9 })], { ref: 'EK-1d s.11; EK-1e s.132', note: IBADET_NOTE });
ibadet.area('sinagog-havra', 'Sinagog (havra)', [solid(SOSYAL), gridDots(7, 1.2), picto('davut-yildizi', { size: 9 })], { ref: 'EK-1d s.12; EK-1e s.133', note: IBADET_NOTE });
