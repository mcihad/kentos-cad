import { BLACK, WHITE, along, circle, dashDots, groupedHatch, shape, solid, stroke, svg, text, rgb } from '../dsl';
import { pic } from '../pictograms';
import { BLUE, RED, SYMBOL, circleCross, code, gear, gridDots, heading, picto, sections, triangleOnLine, type Level } from './common';

/**
 * EK-1a (s.1–3) › Sınırlar: idari, planlama ve özel kanunlarla belirlenen
 * sınırlar, every plan level. Numbers are EK-1e s.12–36; where a level
 * differs, its row is in the tables below.
 */

export const HEAD = [
  ...heading([], 1000, 'EK-1a: her plan türünde kullanılan ortak gösterimler; ölçüler EK-1e, sembol çerçeveleri AÇIKLAMA 6 (ÇDP 5, NİP 7, UİP 10 mm)'),
  ...heading(['Sınırlar'], 1005),
];

// ── İdari sınırlar ─────────────────────────────────────────────────────

/**
 * Belediye and mücavir alan sınırı: a 10 mm line between two 2 mm dots
 * (dumbbell), then `dots` 1 mm dots, 1 mm clear gaps between all.
 */
function dumbbell(dots: number) {
  const period = 14 + 1 + dots * 2;
  return [
    // The line runs between the big dots' centres (1 → 13 mm of the period).
    stroke(BLACK, 0.8, { dash: [12, period - 12], dashOffset: period - 1 }),
    along(circle(2, { fill: BLACK }), period, { offsetAlong: 7, group: { count: 2, spacing: 12 } }),
    along(circle(1, { fill: BLACK }), period, { offsetAlong: 14.5 + dots, group: { count: dots, spacing: 2 } }),
  ];
}

export const IDARI = sections(['Sınırlar', 'İdari sınırlar'], 'idari', 1010, (s) => {
  s.line(
    'ulke-siniri',
    'Ülke sınırı',
    [
      stroke(BLACK, 1.2, { dash: [10, 5] }),
      along(shape('line', 5, { stroke: BLACK, strokeWidth: 1.2, rotation: 90 }), 15, { offsetAlong: 5, group: { count: 2, spacing: 10 } }),
      along(circle(3, { stroke: BLACK, strokeWidth: 0.3 }), 15, { offsetAlong: 12.5 }),
    ],
    {
      ref: 'EK-1a s.1; EK-1e s.12',
      note: '1.2 mm; 1 cm çizgi, iki ucunda 0.5 cm dik çizgi, aralarda 3 mm çaplı boş daire. Ara boyu verilmemiş: 5 mm (daire ve iki yanında 1 mm) çizimden; daire çizgisi 0.3 mm (çizimde ince).',
    },
  );
  s.line('il-siniri', 'İl sınırı', [dashDots(BLACK, 1, { dash: 10, gap: 1, dots: 1, dot: 1, dotGap: 1 })], { ref: 'EK-1a s.1; EK-1e s.12', note: '1 mm; 1 cm çizgi, 1 mm ara, 1 mm çaplı nokta.' });
  s.line('ilce-siniri', 'İlçe sınırı', [dashDots(BLACK, 0.8, { dash: 10, gap: 2, dots: 2, dot: 1, dotGap: 1 })], {
    ref: 'EK-1a s.1; EK-1e s.13',
    note: '0.8 mm; 1 cm çizgi, 2 mm ara, 1 mm çaplı 2 nokta. İki nokta arası verilmemiş: 1 mm çizimden.',
  });
  s.line('belediye-siniri', 'Belediye sınırı', [dumbbell(2)], {
    ref: 'EK-1a s.1; EK-1e s.13',
    note: '0.8 mm; iki ucunda 2 mm çaplı nokta olan 1 cm çizgi, 1 mm ara ile 2 adet 1 mm çaplı nokta. Büyükşehir belediye sınırını da içerir (EK-1a AÇIKLAMA 5).',
  });
  s.line('mucavir-alan-siniri', 'Mücavir alan sınırı', [dumbbell(3)], { ref: 'EK-1a s.1; EK-1e s.14', note: '0.8 mm; iki ucunda 2 mm çaplı nokta olan 1 cm çizgi, 1 mm ara ile 3 adet 1 mm çaplı nokta.' });
});

// ── Planlama sınırları ─────────────────────────────────────────────────

/** Plan onama and etaplama sınırı: circle diameter per level, 2.5 mm apart (EK-1e s.15–16). */
const CIRCLE: Record<Level, number> = { uip: 5, nip: 4, cdp: 3 };

export const PLANLAMA = sections(['Sınırlar', 'Planlama sınırları'], 'planlama', 1020, (s, lv) => {
  const d = CIRCLE[lv];
  const step = d + 2.5;
  s.line('plan-onama-siniri', 'Plan onama sınırı', [along(circle(d, { stroke: BLUE, strokeWidth: 0.3 }), step)], {
    ref: 'EK-1a s.1; EK-1e s.15',
    note: `0.3 mm mavi (0/92/230); ${d} mm çaplı içi boş daireler, 2.5 mm ara ile; çizgi yok.`,
  });
  s.line('plan-degisikligi-onama-siniri', 'Plan değişikliği onama sınırı', [dashDots(BLACK, 0.6, { dash: 10, gap: 1, dots: 1, dot: 0.6, dotGap: 1 })], {
    ref: 'EK-1a s.1; EK-1e s.16',
    note: '0.6 mm; 1 cm çizgi, 1 mm ara, 0.6 mm çaplı nokta.',
  });
  s.line(
    'etaplama-siniri',
    'Etaplama sınırı',
    [along(circle(d, { fill: BLUE }), 2 * step, { offsetAlong: step / 2 }), along(circle(d, { stroke: BLUE, strokeWidth: 0.3 }), 2 * step, { offsetAlong: 1.5 * step })],
    { ref: 'EK-1a s.1; EK-1e s.16', note: `0.3 mm mavi (0/92/230); ${d} mm çaplı daireler, 2.5 mm ara ile 1 dolu 1 boş; çizgi yok.` },
  );
  s.line('ozel-proje-alani-siniri', 'Özel proje alanı sınırı', [stroke(BLUE, 0.3), along(circle(4, { fill: BLUE }), 6.5)], {
    ref: 'EK-1a s.2; EK-1e s.20',
    note: '0.3 mm mavi (0/92/230) çizgi üzerinde 2.5 mm ara ile 4 mm çaplı dolu daireler. EK-1a alan rengini 0/92/230 yazar; EK-1e alanı şeffaf verir, EK-1e uygulandı.',
  });
});

// ── Özel kanunlarla belirlenen alan ve sınırları ───────────────────────

/** Turizm merkezi …: line weight and ⊕ diameter per level (EK-1e s.20). */
const TURIZM: Record<Level, { w: number; d: number }> = { uip: { w: 0.4, d: 5 }, nip: { w: 0.3, d: 4 }, cdp: { w: 0.3, d: 4 } };

/** TGB, serbest bölge: dot grid (EK-1e s.21); gear outer and inner diameters for OSB, EB, TGB and the serbest bölge's hollow gear. */
const DOTS: Record<Level, { spacing: number; dot: number }> = { uip: { spacing: 7, dot: 1.2 }, nip: { spacing: 5, dot: 0.4 }, cdp: { spacing: 5, dot: 0.4 } };
const GEAR: Record<Level, readonly [number, number]> = { uip: [7, 5], nip: [6, 4], cdp: [5, 3] };
/** OSB and endüstri bölgesi: distance between the hatch pairs (EK-1e s.22–23). */
const PAIRS: Record<Level, number> = { uip: 5, nip: 3, cdp: 3 };

const TECH_FILL = rgb(102, 153, 205);
const INDUSTRY_FILL = rgb(170, 102, 205);

/** A 7 mm line piece, then `count` gears 1 mm apart with 2 mm clear on each side. */
function gearLine(lv: Level, count: number) {
  const [o, i] = GEAR[lv];
  const gap = 2 + count * o + (count - 1) + 2;
  return [stroke(BLACK, 0.3, { dash: [7, gap] }), along(gear(o, i), 7 + gap, { offsetAlong: 7 + gap / 2, group: count > 1 ? { count, spacing: o + 1 } : undefined })];
}

/** The 45° double-line cross hatch of OSB and endüstri bölgesi: 0.2 mm lines in pairs 1 mm apart. */
const pairCross = (lv: Level) => [...groupedHatch(45, PAIRS[lv], 2, 1, 0.2), ...groupedHatch(135, PAIRS[lv], 2, 1, 0.2)];

export const OZEL_KANUN = sections(['Sınırlar', 'Özel kanunlarla belirlenen alan ve sınırları'], 'ozel-kanun', 1030, (s, lv) => {
  const S = SYMBOL[lv];
  const { w, d } = TURIZM[lv];
  const tGap = 1 + 2 * d + 1;
  s.line(
    'turizm-merkezi-kultur-ve-turizm-koruma-ve-gelisim-alt-bolgesi',
    'Turizm merkezi, kültür ve turizm koruma ve gelişim alt bölgesi',
    [stroke(BLUE, w, { dash: [5, tGap] }), along(circleCross(d, w, BLUE), 5 + tGap, { offsetAlong: 5 + tGap / 2, group: { count: 2, spacing: d } })],
    { ref: 'EK-1a s.2; EK-1e s.20', note: `${w} mm mavi (0/92/230); ${d} mm çaplı içinde artı olan iki daire (birbirine değer), 1 mm boşluk, 5 mm çizgi. Alan şeffaf.` },
  );

  const dots = DOTS[lv];
  const [go, gi] = GEAR[lv];
  s.area('teknoloji-gelistirme-bolgesi', 'Teknoloji geliştirme bölgesi', [solid(TECH_FILL), gridDots(dots.spacing, dots.dot), gearLine(lv, 3), code(lv, 'TGB')], {
    ref: 'EK-1a s.2; EK-1e s.21',
    note: `Alan 102/153/205; ${dots.spacing}×${dots.spacing} mm karolaj merkezlerinde ${dots.dot} mm nokta. Sınır 0.3 mm: ${go} mm dış, ${gi} mm iç çaplı 1 mm aralıklı 3 dişli, 2 mm boşluk, 7 mm çizgi (diş sayısı ve biçimi çizimden: 12 kare diş). Sembol: çerçevede kalın "TGB".`,
  });

  const sbGap1 = 2 + go + 2;
  const sbPeriod = 7 + sbGap1 + 7 + 11;
  s.area(
    'serbest-bolge',
    'Serbest bölge',
    [
      solid(TECH_FILL),
      gridDots(dots.spacing, dots.dot),
      stroke(BLACK, 0.3, { dash: [7, sbGap1, 7, 11] }),
      along(gear(go, gi), sbPeriod, { offsetAlong: 7 + sbGap1 / 2 }),
      along(gear(7, null), sbPeriod, { offsetAlong: 7 + sbGap1 + 7 + 5.5 }),
      picto(lv, 'serbest-bolge-bayragi', { caption: 'SB' }),
    ],
    {
      ref: 'EK-1a s.2; EK-1e s.21',
      note: `Alan 102/153/205; ${dots.spacing}×${dots.spacing} mm karolaj merkezlerinde ${dots.dot} mm nokta. Sınır 0.3 mm: ${go} mm dış, ${gi} mm iç çaplı içi boş dişli, 2 mm boşluk, 7 mm çizgi, 2 mm boşluk, 7 mm dış çaplı içi dolu dişli (dolu dişlinin arkasındaki 2 mm boşluk çizimden). Sembol: çerçevede bayrak, altında "SB".`,
    },
  );

  const osbGap = 2 + go + 2;
  s.area(
    'organize-sanayi-bolgesi',
    'Organize sanayi bölgesi',
    [solid(INDUSTRY_FILL), pairCross(lv), stroke(BLACK, 0.3, { dash: [7, osbGap] }), along(gear(go, gi), 7 + osbGap, { offsetAlong: 7 + osbGap / 2 }), code(lv, 'OSB')],
    {
      ref: 'EK-1a s.2; EK-1e s.22',
      note: `Alan 170/102/205; 0.2 mm, 1 mm aralıklı çiftlerle 45° çapraz tarama, çiftler arası ${PAIRS[lv]} mm (çiftten çifte; EK-1a çiziminde çift aralığı ile çift arası oranı 1:5). Sınır 0.3 mm: ${go} mm dış, ${gi} mm iç çaplı 1 dişli, 2 mm boşluk, 7 mm çizgi.`,
    },
  );
  s.area('endustri-bolgesi', 'Endüstri bölgesi', [solid(INDUSTRY_FILL), pairCross(lv), gearLine(lv, 2), code(lv, 'EB', { heavy: true })], {
    ref: 'EK-1a s.2; EK-1e s.23',
    note: `Alan 170/102/205; OSB ile aynı tarama (çiftler arası ${PAIRS[lv]} mm). Sınır 0.3 mm: ${go} mm dış, ${gi} mm iç çaplı 1 mm aralıklı 2 dişli, 2 mm boşluk, 7 mm çizgi. Sembol: kalın çerçevede "EB".`,
  });

  // Askeri yasak: 7 mm red line pieces, a 3 mm × between them; the gap (4.5 mm) is measured from the drawing.
  s.area(
    'askeri-yasak-ve-guvenlik-bolgesi',
    'Askeri yasak ve güvenlik bölgesi',
    [
      solid(rgb(168, 168, 0)),
      groupedHatch(90, 5, 2, 1, 0.2),
      stroke(RED, 0.3, { dash: [7, 4.5] }),
      along(shape('x', 3, { stroke: RED, strokeWidth: 0.3 }), 11.5, { offsetAlong: 9.25 }),
      along(text('AYB', 2.5, { font: 'sans', weight: 700, color: RED }), 11.5 * 5, { offsetAlong: 3.5, offset: 1.9 }),
      picto(lv, 'tufekler', { caption: 'AYB' }),
    ],
    {
      ref: 'EK-1a s.2; EK-1e s.24',
      note: 'Alan 168/168/0; 0.2 mm, 1 mm aralıklı dik çift çizgiler, çiftler arası 5 mm. Sınır 0.3 mm kırmızı: 7 mm düz çizgi aralarında 3 mm uzunluğunda çarpı (ara 4.5 mm çizimden); "uygun aralıklarla" sınır üzerine AYB yazılır: her beşinci çizgi parçasının içine 2.5 mm. Sembol: çerçevede çapraz tüfekler, altında AYB (EK-1e; EK-1a altyazısız çizer). AÇIKLAMA 9: askeri yasak ve güvenlik bölgesi ayrı ayrı gösterilir.',
    },
  );

  const squares = (n: number) => [stroke(BLACK, 0.3), along(shape('square', 2, { fill: BLACK }), 7 + n * 3 - 1, { offsetAlong: 7 + (n * 3 - 1) / 2, group: n > 1 ? { count: n, spacing: 3 } : undefined })];
  const bogaz = 'Alan şeffaf. 0.3 mm düz çizgi üzerinde 7 mm çizgi aralıklı, 1 mm aralıklı 2 mm dolu kareler';
  s.line('bogazici-etkilenme-bolgesi-siniri', 'Boğaziçi etkilenme bölgesi sınırı', [squares(3)], { ref: 'EK-1a s.2; EK-1e s.25', note: `${bogaz} (3 adet).` });
  s.line('bogazici-geri-gorunum-bolgesi-siniri', 'Boğaziçi geri görünüm bölgesi sınırı', [squares(2)], { ref: 'EK-1a s.2; EK-1e s.25', note: `${bogaz} (2 adet).` });
  s.line('bogazici-on-gorunum-bolgesi-siniri', 'Boğaziçi ön görünüm bölgesi sınırı', [squares(1)], { ref: 'EK-1a s.2; EK-1e s.26', note: `${bogaz} (1 adet).` });

  s.point('sinir-kapisi', 'Sınır kapısı', [svg(pic('sinir-kapisi'), S)], {
    ref: 'EK-1a s.2; EK-1e s.36',
    note: `Kırmızı (255/0/0) bayrak; "ilgili alana uygun büyüklükte": ${S} mm (AÇIKLAMA 6).`,
  });

  // Two filled and one hollow 4 mm triangle standing on the line, 1 mm apart; 1 mm; 5 mm line; 1 mm.
  s.line(
    'diger-ozel-kanunlarla-belirlenen-alan',
    'Diğer özel kanunlarla belirlenen alan (……. sayılı kanun)',
    [
      stroke(RED, 0.3, { dash: [5, 16] }),
      along(triangleOnLine(4, { fill: RED }), 21, { offsetAlong: 13, group: { count: 2, spacing: 10 } }),
      along(triangleOnLine(4, { fill: WHITE, stroke: RED, strokeWidth: 0.3 }), 21, { offsetAlong: 13 }),
      along(text({ expr: `varsayılan([Kanun], '……') || ' sayılı kanun'` }, 2.5, { font: 'sans', weight: 700, color: RED }), 21 * 4, { offsetAlong: 2.5, offset: -1.9 }),
    ],
    {
      ref: 'EK-1a s.3; EK-1e s.34',
      note: 'Alan şeffaf. 0.3 mm kırmızı; 4 mm kenarlı 2 dolu 1 boş eşkenar üçgen (dolu, boş, dolu; tabanı çizgide, tepesi alanın içine), 1 mm boşluk, 5 mm düz çizgi, 1 mm boşluk; üçgenler arası 1 mm çizimden. AÇIKLAMA 11: sınır üzerine kanun numarası yazılır: "Kanun" alanından, her dördüncü çizgi parçasının dışına ("Kanun" boşsa "……").',
    },
  );
});
