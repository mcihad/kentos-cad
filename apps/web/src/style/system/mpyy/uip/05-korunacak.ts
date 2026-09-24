import { BLACK, WHITE, along, circle, crossHatch, edge, groupedHatch, hatch, label, pattern, rgb, shape, sheet, solid, stroke, text } from '../dsl';
import { RED, code, crossBoxMarks, freeDots, picto, pictoMarks } from './common';

/**
 * UİP (EK-1d s.6–8) › Korunacak alanlar and its first sub-group, bugünkü
 * arazi kullanımı devam ettirilerek korunacak alanlar. Numbers are EK-1e's
 * UİP rows (s.77–101).
 */

export const korunacak = sheet('uip', ['Korunacak alanlar'], 'korunacak', 60);

/** Tescilli yapı alanları: 0.2 mm, 1 mm ara ile çapraz tarama; SINIR 0.4 mm düz çizgi. */
const tescilHatch = () => crossHatch(1, 0.2);

korunacak.area('tescilli-anit-yapi', 'Tescilli anıt yapı', [tescilHatch(), edge(BLACK, 0.4), code('ANIT', 3.2, { weight: 900 })], {
  ref: 'EK-1d s.6; EK-1e s.77',
  note: 'Şeffaf; yapı alanında 0.2 mm, 1 mm ara ile 45° çapraz tarama; sınır 0.4 mm düz çizgi (yapı sınırı).',
});
korunacak.area('tescilli-bina', 'Tescilli bina', [tescilHatch(), edge(BLACK, 0.4), label(crossBoxMarks('TES', { outer: 0.5, inner: 0.25, captionFont: 'sans' }))], {
  ref: 'EK-1d s.6; EK-1e s.78',
  note: 'Şeffaf; 0.2 mm, 1 mm ara ile çapraz tarama; sınır 0.4 mm. Sembol: dışı kalın çift kare ve iç karenin köşegenleri, altında "TES" (Arial kalın). Ölçüler EK-1d\'den.',
});
korunacak.area('tescilli-parsel', 'Tescilli parsel', [hatch(0, 1, 0.2), edge(BLACK, 0.4)], {
  ref: 'EK-1d s.6; EK-1e s.78',
  note: 'Şeffaf; 0.2 mm, 1 mm aralıklı yatay tarama; sınır 0.4 mm düz çizgi (parsel sınırı). Sembol yok.',
});
korunacak.area('tescilli-tabiat-varligi', 'Tescilli tabiat varlığı', [hatch(0, 1, 0.2), edge(BLACK, 0.4), code('TTV', 3.5, { weight: 900 })], {
  ref: 'EK-1d s.7; EK-1e s.79',
  note: 'Şeffaf; 0.2 mm, 1 mm aralıklı yatay tarama; sınır 0.4 mm düz çizgi.',
});

/** "5.5 mm boş daire içerisinde 3 mm çaplı dolu daire" on a 0.3 mm red line with 7 mm pieces (12.5 mm period). */
const bullseye = () => [stroke(RED, 0.3, { dash: [7, 5.5] }), along([circle(5.5, { fill: WHITE, stroke: RED, strokeWidth: 0.3 }), circle(3, { fill: RED })], 12.5, { offsetAlong: 9.75 })];

korunacak.area(
  'sit-etkilesim-gecis-alani-siniri',
  'Sit etkileşim geçiş alanı sınırı',
  [
    bullseye(),
    // "SEG" standing on each 7 mm piece (inside), three short ticks under it (outside), as EK-1d draws it.
    along(text('SEG', 2.5, { font: 'sans', weight: 700, offset: [0, 0.3 + 0.35 * 2.5] }), 12.5, { offsetAlong: 3.5 }),
    along(shape('line', 1, { stroke: RED, strokeWidth: 0.2, rotation: 90, offset: [-0.65, 0] }), 12.5, { offsetAlong: 3.5, group: { count: 3, spacing: 0.8 } }),
    label(crossBoxMarks('SEG', { captionSize: 4.2 })),
  ],
  {
    ref: 'EK-1d s.7; EK-1e s.84',
    note: 'Şeffaf. Sınır: 0.3 mm kırmızı, 7 mm düz çizgi aralarında 5.5 mm boş daire içinde 3 mm dolu daire; "uygun aralıklarla sınır üzerine SEG yazılacak": EK-1d gibi her çizgi parçasının üstüne (alanın içine) 2.5 mm SEG ve altına üç kısa tırnak (tırnaklar metinde yok, çizimden). Sembol: çift kare, köşegenler, altında SEG.',
  },
);
korunacak.area('hassas-endemik-biyotop-alani', 'Hassas endemik biyotop alanı', [groupedHatch(45, 7, 2, 1, 0.4), code('EB', 5.5, { weight: 900, strokeWidth: 0.5 })], {
  ref: 'EK-1d s.7; EK-1e s.91',
  note: 'Şeffaf; 0.4 mm, 1 mm aralıklı 45° çift çizgi, çiftler arası 7 mm.',
});
korunacak.point('kentsel-goruntu-ogeleri-imgeleri', 'Kentsel görüntü öğeleri/imgeleri (vista-siluet-odak noktaları vb.)', [pictoMarks('manzara', { size: 9 })], {
  ref: 'EK-1d s.7; EK-1e s.91',
  note: 'Nokta: bakış konisi piktogramı ince çerçevede; ölçü verilmemiş, çerçeve 10 mm alındı.',
});
korunacak.line('yoresel-mimari-ozellikleri-korunacak-alan', 'Yöresel mimari özellikleri korunacak alan', [bullseye()], {
  ref: 'EK-1d s.7; EK-1e s.92',
  note: 'Şeffaf alan sınırı: 0.3 mm kırmızı, 7 mm aralıkla 5.5 mm daire içinde 3 mm dolu daire. EK-1d dış daireyi koyu kırmızı çiziyor; EK-1e\'nin tek RGB değeri (255/0/0) kullanıldı.',
});
korunacak.point('jeotermal-kaynak', 'Jeotermal kaynak', [pictoMarks('fiskiye', { size: 9, caption: 'JK' })], {
  ref: 'EK-1d s.7; EK-1e s.92',
  note: 'Nokta: fıskiye piktogramı ince çerçevede, altında "JK"; ölçü verilmemiş, çerçeve 10 mm alındı.',
});

export const bugunku = sheet('uip', ['Korunacak alanlar', 'Bugünkü arazi kullanımı devam ettirilerek korunacak alanlar'], 'bugunku', 10);

const TARIM = rgb(233, 250, 190);
/** "8 mm karelaj merkezlerinde 1 mm çapında içi boş daireler", 0.2 mm. */
const rings8 = () => pattern(circle(1, { stroke: BLACK, strokeWidth: 0.2 }), 8, 8);

bugunku.area('tarimsal-nitelikli-alan', 'Tarımsal nitelikli alan', [solid(TARIM), rings8(), picto('basak', { size: 9 })], {
  ref: 'EK-1d s.7; EK-1e s.100',
  note: '0.2 mm, 8 mm karelaj merkezlerinde 1 mm çapında içi boş daireler.',
});
bugunku.area(
  'ortu-alti-tarim-arazisi',
  'Örtü altı tarım arazisi',
  [
    solid(TARIM),
    hatch(0, 8, 0.2),
    hatch(90, 8, 0.2),
    // The circles sit on the grid's nodes and the lines run between them: filled with the area colour.
    pattern(circle(1, { fill: TARIM, stroke: BLACK, strokeWidth: 0.2 }), 8, 8, { offset: [4, 4] }),
    picto('sera', { size: 9 }),
  ],
  {
    ref: 'EK-1d s.7; EK-1e s.101',
    note: '0.2 mm, 8 mm karelaj düğümlerinde 1 mm çapında içi boş daireler, daireler arası yatay ve dikey tarama. EK-1d yatay çizgileri gri çiziyor; metin aynı 0.2 mm siyah çizgi der.',
  },
);
bugunku.area('organik-tarim-alani', 'Organik tarım alanı', [solid(TARIM), rings8(), picto('basak', { size: 9, caption: 'OTA' })], {
  ref: 'EK-1d s.7; EK-1e s.101',
  note: '0.2 mm, 8 mm karolaj merkezlerinde 1 mm çapında içi boş daireler (EK-1d çizimi daha sık gösteriyor; metin esas).',
});
bugunku.area('makilik-fundalik-alan', 'Makilik-fundalık alan', [solid(rgb(180, 215, 158)), hatch(0, 5, 0.3), code('MF', 6, { weight: 900, strokeWidth: 0.5, size: 13.5 })], {
  ref: 'EK-1d s.7; EK-1e s.96',
  note: '0.3 mm, 5 mm yatay tarama. Çerçeve EK-1d\'deki gibi 13.5 mm, kalın.',
});
bugunku.area('kumsal-plaj', 'Kumsal-plaj', [solid(rgb(255, 229, 207)), freeDots(0.4, 3), picto('plaj-semsiyesi', { size: 9, strokeWidth: 0.5, caption: 'K/P' })], {
  ref: 'EK-1d s.8; EK-1e s.97',
  note: '0.4 mm serbest noktalama; yoğunluk verilmemiş, EK-1d\'deki gibi seyrek (≈2.2 mm hücrede bir nokta).',
});
