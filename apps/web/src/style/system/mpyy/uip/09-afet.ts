import { BLACK, WHITE, along, edge, groupedHatch, label, rgb, shape, sheet, solid, stroke, text } from '../dsl';
import { BELT_FILL, FRAME, FRAME_LINE, RED, code, crossBoxMarks, staggeredDashes } from './common';

/**
 * UİP (EK-1d s.13–14) › Afet tehlikeli alanlar. EK-1d prints them as a
 * bold sub-row under "Açık ve yeşil alanlar"; they are a section of their
 * own here, as the hazard category they are. Numbers: EK-1e s.144–146,
 * 189–191.
 */

export const afet = sheet('uip', ['Afet tehlikeli alanlar'], 'afet', 90);

/** "60 derecelik 1 mm aralıklı üçlü çizgi, çizgiler arası 8 mm", 0.2 mm (the 8 mm read as group to group). */
const triple60 = () => groupedHatch(60, 8, 3, 1, 0.2);
/** "10 mm uzunluğunda 3 mm aralıklı … kesik şaşırtmalı çizgiler", 0.2 mm; the gap is not given (2 mm, from EK-1d's rhythm). */
const dashes = (angle: number) => staggeredDashes(angle, 3, 10, 2, 0.2);

/**
 * The hazard boundary (EK-1e s.145): a 0.3 mm red line through the apexes of
 * open equilateral triangles of 3 mm side (bases left out), 7 mm apart
 * (10 mm pitch). They hang on the right of the drawing direction as EK-1d
 * draws them under a line drawn left to right (on areas: outside).
 */
const H = (3 * Math.sqrt(3)) / 2;
// A chevron turned to point left of the line, moved back by half its length so its apex sits on the line
// (a turned marker's offset is in its own frame).
const hazardLine = () => [stroke(RED, 0.3), along(shape('chevron', H, { height: 3, stroke: RED, strokeWidth: 0.3, rotation: 90, offset: [-H / 2, 0] }), 10, { offsetAlong: 5 })];
const HAZARD_NOTE =
  'Sınır: 0.3 mm kırmızı, 7 mm aralıklı, 3 mm kenarlı tabanları eksik eşkenar üçgenler ve köşelerinden geçen düz çizgi; üçgenler EK-1d\'deki gibi çizim yönünün sağına (alan kenarında dışa) asılı. İki ekin çizimi üçlü gruplar gösteriyor (başka gösterimden kopya); metin esas: tek üçgenler, aralarında 7 mm.';

afet.area('yapi-yasakli-alan', 'Yapı yasaklı alan', [triple60(), edge(BLACK, 0.4, { dash: [2, 2] }), code('YYA', 3, { font: 'serif' })], {
  ref: 'EK-1d s.13; EK-1e s.144',
  note: 'Şeffaf. Tarama: 0.2 mm, 60° 1 mm aralıklı üçlü çizgi, çizgiler arası 8 mm. Sınır: 0.4 mm siyah, 2 mm çizgi, 2 mm ara.',
});
afet.area('taskina-maruz-alan', 'Taşkına maruz alan', [solid(BELT_FILL), dashes(0), hazardLine(), label(crossBoxMarks('TAŞKIN', { captionSize: 3.3 }))], {
  ref: 'EK-1d s.13; EK-1e s.145',
  note: `Alan 245/122/122. Tarama: 0.2 mm, 10 mm uzunluğunda 3 mm aralıklı yatay kesik şaşırtmalı çizgiler (boşluk verilmemiş: 2 mm). ${HAZARD_NOTE}`,
});
afet.area('heyelan-alani', 'Heyelan alanı', [triple60(), hazardLine(), code('HA', 4, { font: 'serif' })], {
  ref: 'EK-1d s.13; EK-1e s.146',
  note: `Şeffaf. Tarama: 0.2 mm, 60° 1 mm aralıklı üçlü çizgi, çizgiler arası 8 mm. ${HAZARD_NOTE} AÇIKLAMA 4: etüt raporlarındaki veriler plana işlenir.`,
});

/** Önlemli alan: pairs of filled 3 mm triangles, 1 mm apart, apex on the line, 7 mm between pairs (14 mm). */
afet.area(
  'onlemli-alan',
  'Önlemli alan',
  [
    stroke(RED, 0.3),
    along(shape('triangle', 3, { fill: RED, offset: [0, -H * (2 / 3)] }), 14, { offsetAlong: 7, group: { count: 2, spacing: 4 } }),
    label(crossBoxMarks({ expr: `'ÖA' || eğer(boş([No]), '', '-' || [No])` })),
  ],
  {
    ref: 'EK-1d s.13; EK-1e s.146',
    note: 'Şeffaf. Sınır: 0.3 mm kırmızı düz çizgi, köşeleri çizgide 1 mm aralı 3 mm kenarlı 2 dolu eşkenar üçgen, 7 mm boşluk; üçgenler çizim yönünün sağında (EK-1d). EK-1e sınır rengini "şeffaf" yazar, çizimler kırmızı. AÇIKLAMA 10: etüt raporu uyarınca numaralandırılabilir: "No" alanı doluysa sembol "ÖA-No" olur.',
  },
);

/** KRA / KHRA: the code over "(…)" in a thin frame; the parentheses carry the "Tür" field. */
const typed = (c: string) =>
  label([
    shape('square', FRAME, { fill: WHITE, stroke: BLACK, strokeWidth: FRAME_LINE }),
    text(c, 2.8, { font: 'sans', weight: 700, offset: [0, 1.5] }),
    text({ expr: `'(' || varsayılan([Tür], '….') || ')'` }, 2.4, { font: 'sans', weight: 700, offset: [0, -1.7] }),
  ]);

afet.area('kentsel-risk-alani', 'Kentsel risk alanı', [triple60(), edge(BLACK, 0.4, { dash: [2, 2] }), typed('KRA')], {
  ref: 'EK-1d s.13; EK-1e s.189',
  note: 'Şeffaf. Tarama: 0.2 mm, 60° 1 mm aralıklı üçlü çizgi, çizgiler arası 8 mm (EK-1d koyu gri çiziyor; siyah alındı). Sınır: 0.4 mm siyah, 2 mm çizgi, 2 mm ara. Parantez içine risk türü yazılır (yangın, orman yangını, endüstriyel/kimyasal kaza vs.): "Tür" alanından.',
});
afet.area('tsunami-riskli-alan', 'Tsunami riskli alan', [solid(rgb(225, 225, 225)), dashes(135), hazardLine(), label(crossBoxMarks('TSUNAMİ', { captionSize: 2.6, captionFont: 'sans', outer: 0.5, inner: 0.25 }))], {
  ref: 'EK-1d s.14; EK-1e s.189',
  note: `Alan 225/225/225. Tarama: 0.2 mm, 315° (sağa doğru inen) 3 mm aralıklı 10 mm uzunluğunda kesik şaşırtmalı çizgiler (boşluk verilmemiş: 2 mm). ${HAZARD_NOTE}`,
});
afet.area('kutle-hareketi-riskli-alan', 'Kütle hareketi riskli alan', [dashes(0), hazardLine(), typed('KHRA')], {
  ref: 'EK-1d s.14; EK-1e s.190',
  note: `Şeffaf. Tarama: 0.2 mm, 10 mm uzunluğunda 3 mm aralıklı yatay kesik şaşırtmalı çizgiler (boşluk verilmemiş: 2 mm). ${HAZARD_NOTE} Parantez içine hareket türü yazılır (kaya, çığ, heyelan, obruk vs.): "Tür" alanından.`,
});
afet.area('fay-sakinim-zonu-tampon-alani', 'Fay sakınım zonu/tampon alanı', [triple60(), hazardLine(), along(text('FTA', 2.5, { font: 'sans', weight: 700, offset: [0, 0.3 + 0.35 * 2.5] }), 40, { offsetAlong: 20 })], {
  ref: 'EK-1d s.14; EK-1e s.191',
  note: `Şeffaf. Tarama: 0.2 mm, 60° 1 mm aralıklı üçlü çizgi, çizgiler arası 8 mm. ${HAZARD_NOTE} Çizimdeki "FTA" yazısı (metinde yok) çizginin soluna (alanın içine) 40 mm'de bir 2.5 mm yazılır. Sembol yok.`,
});
