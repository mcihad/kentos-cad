import { BLACK, WHITE, along, circle, dashDots, dashMarkers, double, edge, framed, rgb, sheet, shape, stroke, text, ticks } from '../dsl';
import { stack } from './common';

/**
 * NİP (EK-1ç) › Sınırlar: idari, planlama, özel kanunlarla belirlenen ve
 * yapı sınırlaması getirilen sınırlar. Ölçüler EK-1e'nin NİP satırlarından;
 * alan içi semboller EK-1a AÇIKLAMA 6 gereği 7 mm.
 */

const RED = rgb(255, 0, 0);
const BLUE = rgb(0, 92, 230);

/** Text written on a boundary at intervals ("uygun aralıklarla sınır üzerine RA yazılacak"): four 5 mm dashes 1 mm apart, then the word in a gap. */
function wordOnLine(word: string) {
  const size = 2.5;
  // Bold Arial capitals are about 0.72 em wide (İ about 0.3): the gap leaves 1 mm clear on each side.
  const w = [...word].reduce((s, c) => s + (c === 'İ' || c === 'I' ? 0.3 : 0.72), 0) * size;
  const gap = w + 2;
  return [
    stroke(BLACK, 0.5, { dash: [5, 1, 5, 1, 5, 1, 5, gap] }),
    along(text(word, size, { font: 'sans', weight: 700 }), 23 + gap, { offsetAlong: 23 + gap / 2 }),
  ];
}

/** A letter in a 7 mm circle inside the area (K, Y, S). */
const circled = (letter: string) => stack(framed(letter, { frame: 'circle', size: 7, strokeWidth: 0.3, textSize: 3.6, font: 'sans', weight: 700, background: WHITE }));

export const sinirlar = sheet('nip', ['Sınırlar'], 'sinirlar', 10);

export const idari = sheet('nip', ['Sınırlar', 'İdari sınırlar'], 'idari', 11);

idari.line('koy-siniri', 'Köy sınırı', [dashDots(BLACK, 0.7, { dash: 10, gap: 2, dots: 3, dot: 1, dotGap: 1 })], {
  ref: 'EK-1ç s.1; EK-1e s.14',
  note: 'Noktalar arası 1 mm çizimden ölçüldü (metinde yok).',
});

export const planlama = sheet('nip', ['Sınırlar', 'Planlama sınırları'], 'planlama', 12);

const planNote = 'Sınır rengi EK-1e\'deki 0/92/230; EK-1ç örneği daha koyu mavi basılmış. Harfli daire alanın iç noktasına konur; çapı EK-1a AÇIKLAMA 6 gereği 7 mm (EK-1e ölçü vermiyor).';
planlama.area('mevcut-plandaki-durumu-korunacak-alan-siniri', 'Mevcut plandaki durumu korunacak alan sınırı', [edge(BLUE, 0.5, { dash: [10, 2] }), circled('K')], { ref: 'EK-1ç s.1; EK-1e s.17', note: planNote });
planlama.area('yeniden-duzenlenecek-alan-siniri', 'Yeniden düzenlenecek alan sınırı', [edge(BLUE, 0.5, { dash: [10, 2] }), circled('Y')], { ref: 'EK-1ç s.1 (AÇIKLAMA 2); EK-1e s.18', note: planNote });
planlama.area('sagliklastirma-alani-siniri', 'Sağlıklaştırma alanı sınırı', [edge(BLUE, 0.5, { dash: [10, 2] }), circled('S')], { ref: 'EK-1ç s.1 (AÇIKLAMA 3); EK-1e s.18', note: planNote });
planlama.line(
  'imar-hakki-aktarim-alani-siniri',
  'İmar hakkı aktarım alanı sınırı',
  [dashMarkers(RED, 0.3, 7, 4.5, [circle(4.5, { stroke: RED, strokeWidth: 0.3 }), circle(2.5, { stroke: RED, strokeWidth: 0.3 })])],
  { ref: 'EK-1ç s.1; EK-1e s.19', note: 'NİP: 4,5 mm boş daire içinde 2,5 mm boş daire, 7 mm çizgi; çizgi dış daireye değer.' },
);
planlama.line(
  'kirsal-yerlesik-alan-siniri',
  'Kırsal yerleşik alan sınırı',
  [along(circle(4, { fill: BLACK }), 13, { offsetAlong: 2 }), along(circle(4, { stroke: BLACK, strokeWidth: 0.3 }), 13, { offsetAlong: 8.5 })],
  { ref: 'EK-1ç s.1; EK-1e s.19', note: '4 mm daireler 2,5 mm ara ile bir dolu bir boş; bağlayan çizgi yok.' },
);

export const ozelKanun = sheet('nip', ['Sınırlar', 'Özel kanunlarla belirlenen alan ve sınırlar'], 'ozel-kanun', 13);

ozelKanun.line('gecekondu-onleme-bolgesi-siniri', 'Gecekondu önleme bölgesi sınırı', [along(shape('cross', 3, { stroke: RED, strokeWidth: 0.3 }), 5)], {
  ref: 'EK-1ç s.1; EK-1e s.26',
  note: '3 mm artılar 2 mm ara ile; bir kolu çizgi doğrultusunda. EK-1ç örneği soluk kırmızı basılmış, renk EK-1e\'deki 255/0/0.',
});
ozelKanun.line('toplu-konut-alani-siniri', 'Toplu konut alanı sınırı', [along([circle(3, { stroke: RED, strokeWidth: 0.3 }), shape('cross', 3, { stroke: RED, strokeWidth: 0.3 })], 5)], {
  ref: 'EK-1ç s.1; EK-1e s.27',
  note: '3 mm daire içinde 3 mm artı, 2 mm ara ile; bağlayan çizgi yok.',
});
ozelKanun.line('sahil-seridi', 'Sahil şeridi', [stroke(RED, 0.6), along(circle(2, { fill: RED }), 1, { placement: 'vertex' })], {
  ref: 'EK-1ç s.2; EK-1e s.27',
  note: 'Tespit edilen koordinat noktaları arası düz çizgi; her köşeye (koordinat noktasına) 2 mm dolu daire.',
});
ozelKanun.line('kiyi-kenar-cizgisi', 'Kıyı kenar çizgisi', [stroke(RED, 0.6), along(circle(2, { fill: 'paper', stroke: RED, strokeWidth: 0.3 }), 1, { placement: 'vertex' })], {
  ref: 'EK-1ç s.2; EK-1e s.28',
  note: 'EK-1e metnine göre: koordinat noktaları arası düz çizgi, her köşede 2 mm boş daire (içinden çizgi geçmez). EK-1ç örneği kesikli çizgi ve boşluklarda küçük daireler gösteriyor; metin esas alındı. Daire çizgi kalınlığı yazılı değil, 0,3 mm alındı.',
});
ozelKanun.line(
  'kentsel-donusum-ve-gelisim-proje-alani-siniri',
  'Kentsel dönüşüm ve gelişim proje alanı sınırı',
  [stroke(RED, 0.3, { dash: [5, 4] }), along(circle(4, { fill: RED }), 18, { offsetAlong: 7 }), along(circle(4, { stroke: RED, strokeWidth: 0.3 }), 18, { offsetAlong: 16 })],
  { ref: 'EK-1ç s.2 (AÇIKLAMA 6); EK-1e s.28', note: '4 mm daireler bir dolu bir boş, aralarında 5 mm çizgi. EK-1ç örneğinde boş dairenin çizgisi koyu kırmızı; EK-1e rengi 255/0/0.' },
);
ozelKanun.line('riskli-alan-siniri', 'Riskli alan sınırı', [wordOnLine('RA')], {
  ref: 'EK-1ç s.2 (AÇIKLAMA 4); EK-1e s.29',
  note: '5 mm çizgi 1 mm ara; "RA" uygun aralıklarla sınır üzerine yazılır. Yazı yüksekliği ve aralığı yazılı değil: 2,5 mm yazı, her dört çizgiden sonra (EK-1ç örneği "_ _RA_ _"). Yazı çizgi boyunca, okunur yönde durur.',
});
ozelKanun.line('rezerv-yapi-alani-siniri', 'Rezerv yapı alanı sınırı', [wordOnLine('RYA')], {
  ref: 'EK-1ç s.2 (AÇIKLAMA 4); EK-1e s.29',
  note: '5 mm çizgi 1 mm ara; "RYA" uygun aralıklarla sınır üzerine yazılır. Yazı yüksekliği ve aralığı yazılı değil: 2,5 mm yazı, her dört çizgiden sonra. Yazı çizgi boyunca, okunur yönde durur.',
});
ozelKanun.line('yenileme-alani-siniri', 'Yenileme alanı sınırı', [wordOnLine('YENİLEME')], {
  ref: 'EK-1ç s.2 (AÇIKLAMA 5); EK-1e s.30',
  note: '5 mm çizgi 1 mm ara; "YENİLEME" uygun aralıklarla sınır üzerine yazılır. Yazı yüksekliği ve aralığı yazılı değil: 2,5 mm yazı, her dört çizgiden sonra. Yazı çizgi boyunca, okunur yönde durur.',
});
ozelKanun.line(
  'statusu-ozel-kanunlarla-belirlenen-alan-siniri',
  'Statüsü özel kanunlarla belirlenen alan sınırı',
  [dashMarkers(RED, 0.3, 7, 8.5, [circle(4.5, { stroke: RED, strokeWidth: 0.3 }), shape('triangle', 3, { stroke: RED, strokeWidth: 0.3 })])],
  { ref: 'EK-1ç s.2 (AÇIKLAMA 7); EK-1e s.30', note: '4,5 mm daire içinde 3 mm boş eşkenar üçgen (tepesi çizginin soluna), 2 mm ara, 7 mm çizgi.' },
);

export const yapiSinirlamasi = sheet('nip', ['Sınırlar', 'Yapı sınırlaması getirilerek korunacak alanlar'], 'yapi-sinirlamasi', 14);

yapiSinirlamasi.line('mania-plani', 'Mania planı', [double(BLACK, 0.4, 1.5)], {
  ref: 'EK-1ç s.2; EK-1e s.109',
  note: '0,4 mm çift çizgi; iki çizgi arası (1,5 mm, eksenden eksene) yazılı değil, çizimden ölçüldü. "Mania planı çizilir."',
});
yapiSinirlamasi.line('havaalani-hava-koridoru', 'Havaalanı hava koridoru', [stroke(BLACK, 0.4), ticks(BLACK, 0.4, 3, 7, 1)], {
  ref: 'EK-1ç s.2 (AÇIKLAMA 21); EK-1e s.110',
  note: '0,4 mm çizgi, 7 mm aralıklı 3 mm dik çizgiler tek yanda: çizim yönünün solunda, alan kenarında koridorun içinde.',
});
