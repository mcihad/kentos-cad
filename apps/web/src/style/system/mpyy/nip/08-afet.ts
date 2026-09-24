import { BLACK, WHITE, along, edge, framed, groupedHatch, hatch, rgb, shape, sheet, solid, text } from '../dsl';
import { S, box, captionMark, crossBoxMarks, stack, turned } from './common';

/**
 * NİP (EK-1ç) › Afet tehlikeli alanlar (EK-1e s.144-146, 189-191). Kırmızı
 * sınırlar, tabanları kenara paralel eşkenar üçgenlerle çizilir: üçgenin
 * tepesi çizgide, gövdesi alanın içinde (çizim yönünün solunda).
 */

const RED = rgb(255, 0, 0);
const PINK = rgb(245, 122, 122);

/**
 * An equilateral triangle of side 3 mm without its base ("tabanı eksik"),
 * its apex on the line and its legs inside: a chevron 2.6 mm deep and 3 mm
 * wide turned to point at the line, shifted so the apex sits on it.
 */
const chevron = shape('chevron', 2.598, { height: 3, stroke: RED, strokeWidth: 0.3, rotation: -90, offset: turned(0, 1.299, -90) });
/** A filled equilateral triangle of side 3 mm, apex on the line, body inside. */
const tooth = shape('triangle', 3, { fill: RED, rotation: 180, offset: turned(0, 1.732, 180) });

/** "0.3 mm, 7 mm aralıklı … tabanları eksik eşkenar üçgenler, üçgenlerin köşesinden geçen … düz çizgi". */
const chevronLine = () => [edge(RED, 0.3), along(chevron, 7)];

/** Groups of three parallel lines 1 mm apart, the groups `period` apart ("60 derecelik 1 mm aralıklı üçlü çizgi"). */
const triple = (period: number) => groupedHatch(60, period, 3, 1, 0.2);

/** Staggered dashed lines ("10 mm uzunluğunda 3 mm aralıklı kesik şaşırtmalı çizgiler"): rows 3 mm apart, 10 mm dashes, every other row shifted half a period. */
function staggered(angle: number) {
  const dash = [10, 3];
  return [hatch(angle, 6, 0.2, BLACK, { dash }), hatch(angle, 6, 0.2, BLACK, { dash, offset: 3, dashOffset: 6.5 })];
}

/** A code with a bracket for the kind of risk written by the planner ("KRA (….)"). */
function riskCode(code: string) {
  return stack([
    ...framed('', { size: S, strokeWidth: 0.25, background: WHITE }),
    text(code, 2.3, { font: 'sans', weight: 700, offset: [0, 1.1] }),
    text({ expr: `'(' || varsayılan([Risk türü], '….') || ')'` }, 1.9, { font: 'sans', weight: 700, offset: [0, -1.3] }),
  ]);
}

const gapNote = 'Kesik çizgilerin boşluğu yazılı değil, 3 mm alındı (EK-1ç örneğinde çizgi ≈ 3 × boşluk).';
const chevronNote = 'Sınır: 0,3 mm kırmızı düz çizgi, 7 mm aralıklı, 3 mm kenarlı tabanı eksik eşkenar üçgenler; üçgenlerin tepesi çizgide, kolları alanın içinde (çizim yönünün solunda). EK-1ç çizimi üçgenleri üçerli gruplarla gösteriyor; metin (tek tek, 7 mm) esas alındı.';

export const afet = sheet('nip', ['Afet tehlikeli alanlar'], 'afet', 80);

afet.area('yapi-yasakli-alan', 'Yapı yasaklı alan', [triple(5), edge(BLACK, 0.4, { dash: [2, 2] }), box('YYA', { font: 'serif' })], {
  ref: 'EK-1ç s.7; EK-1e s.144',
  note: 'Alan şeffaf. Tarama: 0,2 mm, 60 derecelik 1 mm aralıklı üçlü çizgi, çizgiler (üçlüler) arası 5 mm. Sınır: 0,4 mm, 2 mm çizgi 2 mm ara.',
});
afet.area('afete-maruz-bolge', 'Afete maruz bölge', [solid(PINK), edge(RED, 0.3), along(chevron, 18, { offsetAlong: 9, group: { count: 3, spacing: 4 } }), stack(crossBoxMarks('AMB'))], {
  ref: 'EK-1ç s.7; EK-1e s.145',
  note: 'Sınır: 0,3 mm kırmızı düz çizgi; 1 mm aralıkla 3 adet 3 mm kenarlı tabanı eksik eşkenar üçgen, 7 mm boşluk; üçgenlerin tepesi çizgide, kolları alanın içinde.',
});
afet.area('taskina-maruz-alan', 'Taşkına maruz alan', [solid(PINK), staggered(0), chevronLine(), stack(crossBoxMarks('TAŞKIN'))], {
  ref: 'EK-1ç s.7; EK-1e s.145',
  note: `Tarama: 0,2 mm, 10 mm uzunluğunda 3 mm aralıklı yatay kesik şaşırtmalı çizgiler. ${gapNote} ${chevronNote}`,
});
afet.area('onlemli-alan', 'Önlemli alan', [edge(RED, 0.3), along(tooth, 14, { offsetAlong: 7, group: { count: 2, spacing: 4 } }), stack(crossBoxMarks('ÖA'))], {
  ref: 'EK-1ç s.8 (AÇIKLAMA 11, 16); EK-1e s.146',
  note: 'Alan şeffaf. Sınır: 0,3 mm düz çizgi; 1 mm aralıkla 2 adet 3 mm kenarlı dolu eşkenar üçgen, 7 mm boşluk; tepeler çizgide, gövdeler alanın içinde. EK-1e sınır rengini "ŞEFFAF" yazıyor, çizim kırmızı (255/0/0): renk çizimden. Önlemli alanlar etüt raporuna göre numaralandırılabilir.',
});
afet.area('kentsel-risk-alani', 'Kentsel risk alanı', [triple(5), edge(BLACK, 0.4, { dash: [2, 2] }), riskCode('KRA')], {
  ref: 'EK-1ç s.8; EK-1e s.189',
  note: 'Alan şeffaf. Tarama: 0,2 mm, 60 derecelik 1 mm aralıklı üçlü çizgi, üçlüler arası 5 mm. Sınır: 0,4 mm, 2 mm çizgi 2 mm ara. Parantez içine risk türü yazılır (yangın, orman yangını, endüstriyel/kimyasal kaza vb.): "Risk türü" özniteliği.',
});
afet.area(
  'tsunami-riskli-alan',
  'Tsunami riskli alan',
  [solid(rgb(225, 225, 225)), staggered(315), chevronLine(), stack([...crossBoxMarks(''), captionMark('TSUNAMİ', 1.9, S, 'sans', 400)])],
  {
    ref: 'EK-1ç s.8; EK-1e s.189',
    note: `Tarama: 0,2 mm, 315 derecelik (sağa inen) 3 mm aralıklı 10 mm uzunluğunda kesik şaşırtmalı çizgiler; EK-1ç örneği daha dik çizilmiş, açı metinden. ${gapNote} ${chevronNote}`,
  },
);
afet.area('kutle-hareketi-riskli-alan', 'Kütle hareketi riskli alan', [staggered(0), chevronLine(), riskCode('KHRA')], {
  ref: 'EK-1ç s.8; EK-1e s.190',
  note: `Alan şeffaf. Tarama: 0,2 mm, 10 mm uzunluğunda 3 mm aralıklı yatay kesik şaşırtmalı çizgiler. ${gapNote} ${chevronNote} Parantez içine kaya, çığ, heyelan, obruk vb. yazılır: "Risk türü" özniteliği.`,
});
afet.area(
  'fay-sakinim-zonu-tampon-alani',
  'Fay sakınım zonu/tampon alanı',
  [triple(5), chevronLine(), along(text('FTA', 2.2, { font: 'sans', weight: 700, offset: [0, -1.6] }), 28, { offsetAlong: 14 })],
  {
    ref: 'EK-1ç s.8; EK-1e s.191',
    note: `Alan şeffaf. Tarama: 0,2 mm, 60 derecelik 1 mm aralıklı üçlü çizgi, üçlüler arası 5 mm. ${chevronNote} "FTA" yazısı yalnız çizimde var: siyah, çizginin dışında, 28 mm arayla (aralık ve boy çizimden kestirildi).`,
  },
);
