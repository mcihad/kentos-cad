import { BLACK, WHITE, along, circle, dashDots, rgb, shape, sheet, stroke } from '../dsl';
import { RED } from './common';

/**
 * UİP (EK-1d s.1) › Sınırlar: idari, planlama ve özel kanunlarla belirlenen
 * sınırlar. Numbers are EK-1e's UİP rows; boundaries adapt to polygons
 * (drawn on the edges, left = inside).
 */

/** The legend's top section; its children carry the items. */
export const sinirlar = sheet('uip', ['Sınırlar'], 'sinirlar', 10);

export const idari = sheet('uip', ['Sınırlar', 'İdari sınırlar'], 'idari', 10);

idari.line('koy-siniri', 'Köy sınırı', [dashDots(BLACK, 0.7, { dash: 10, gap: 2, dots: 3, dot: 1, dotGap: 1 })], {
  ref: 'EK-1d s.1; EK-1e s.14',
  note: '0.7 mm çizgi: 10 mm çizgi, 2 mm ara, 1 mm çaplı 3 nokta, 2 mm ara. Noktalar arası 1 mm çizimden (metin vermiyor).',
});
idari.line('mahalle-siniri', 'Mahalle sınırı', [dashDots(BLACK, 0.7, { dash: 10, gap: 2, dots: 4, dot: 1, dotGap: 1 })], {
  ref: 'EK-1d s.1; EK-1e s.15',
  note: '0.7 mm çizgi: 10 mm çizgi, 2 mm ara, 1 mm aralı 1 mm çaplı 4 nokta, 2 mm ara.',
});

export const planlama = sheet('uip', ['Sınırlar', 'Planlama sınırları'], 'planlama', 20);

const KTP = rgb(0, 92, 230);
planlama.line(
  'kentsel-tasarim-projesi-siniri',
  'Kentsel tasarım projesi sınırı',
  [
    // 2.5 mm line pieces between the circles; circles alternate empty and filled every 7.5 mm.
    stroke(KTP, 0.3, { dash: [2.5, 5] }),
    along(circle(5, { stroke: KTP, strokeWidth: 0.3 }), 15, { offsetAlong: 5 }),
    along(circle(5, { fill: KTP, stroke: rgb(65, 113, 156), strokeWidth: 0.3 }), 15, { offsetAlong: 12.5 }),
  ],
  {
    ref: 'EK-1d s.1; EK-1e s.17',
    note: '0.3 mm, 5 mm çaplı daireler 2.5 mm ara ile, bir dolu bir boş; renk 0/92/230. Dolu dairelerin koyu mavi-gri kenarı (65/113/156) iki çizimde de var, metinde yok.',
  },
);

planlama.line(
  'imar-hakki-aktarim-alani-siniri',
  'İmar hakkı aktarım alanı sınırı',
  [
    stroke(RED, 0.3, { dash: [7, 5.5] }),
    along([circle(5.5, { stroke: RED, strokeWidth: 0.3 }), circle(3, { stroke: RED, strokeWidth: 0.3 })], 12.5, { offsetAlong: 9.75 }),
  ],
  {
    ref: 'EK-1d s.1; EK-1e s.19',
    note: '0.3 mm kırmızı (255/0/0): 7 mm çizgi, 5.5 mm boş daire içinde 3 mm boş daire; çizgi dış daireye değer. EK-1d çizimi daireleri koyu kırmızı çiziyor; EK-1e RGB değeri kullanıldı.',
  },
);

export const ozelKanun = sheet('uip', ['Sınırlar', 'Özel kanunlarla belirlenen alan ve sınırlar'], 'ozel-kanun', 30);

ozelKanun.line('gecekondu-onleme-bolgesi-siniri', 'Gecekondu önleme bölgesi sınırı', [along(shape('cross', 3, { stroke: RED, strokeWidth: 0.3 }), 5)], {
  ref: 'EK-1d s.1; EK-1e s.26',
  note: 'Yalnızca 3 mm genişliğinde kırmızı artılar, aralarında 2 mm boşluk (5 mm adım); bağlayan çizgi yok. EK-1e çizimi adımı ≈6.8 mm gösteriyor; metin esas alındı.',
});
ozelKanun.line(
  'toplu-konut-alani-siniri',
  'Toplu konut alanı sınırı',
  [along([circle(3, { stroke: RED, strokeWidth: 0.3 }), shape('cross', 3, { stroke: RED, strokeWidth: 0.3 })], 5)],
  { ref: 'EK-1d s.1; EK-1e s.27', note: '3 mm çaplı daire içinde 3 mm artı, 2 mm boşlukla (5 mm adım); bağlayan çizgi yok. Çizgi 0.3 mm kırmızı.' },
);
ozelKanun.line('sahil-seridi', 'Sahil şeridi', [stroke(RED, 0.6), along(circle(2, { fill: RED }), 0, { placement: 'vertex' })], {
  ref: 'EK-1d s.1; EK-1e s.27',
  note: 'Çizgi geometrisi: tespit edilen koordinat noktaları arası 0.6 mm düz kırmızı çizgi, her koordinat noktasında 2 mm dolu daire.',
});
ozelKanun.line('kiyi-kenar-cizgisi', 'Kıyı kenar çizgisi', [stroke(RED, 0.6), along(circle(2, { fill: WHITE, stroke: RED, strokeWidth: 0.2 }), 0, { placement: 'vertex' })], {
  ref: 'EK-1d s.1; EK-1e s.28',
  note: 'Çizgi geometrisi: koordinat noktaları arası 0.6 mm düz kırmızı çizgi, her koordinat noktasında 2 mm boş daire (içi kâğıt rengi, çizgiyi keser). Daire çizgisi kalınlığı verilmemiş; çizimdeki gibi ince (0.2 mm) alındı.',
});
