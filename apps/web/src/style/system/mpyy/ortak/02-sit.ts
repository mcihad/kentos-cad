import { along, circle, groupedHatch, rgb, solid, stroke, text, ticks } from '../dsl';
import { RED, code, heading, picto, sections, triangleOnLine, type Level } from './common';

/**
 * EK-1a (s.3–5) › Korunacak alanlar › Sit ve korunacak alanlar, every
 * plan level. Numbers are EK-1e s.31–33 and s.79–90. The boundaries are
 * red (255/0/0) 0.3 mm; EK-1a draws their connecting pieces a darker red,
 * EK-1e gives the one RGB, which is used. "Çift çizgiler arası" is taken
 * as the distance from pair to pair: EK-1a's hatches keep that ratio to
 * the 1 mm pair gap.
 */

export const HEAD = heading(['Korunacak alanlar'], 1035);

/** Pairs of 0.2–0.4 mm lines 1 mm apart (sit hatches). */
const pairs = (angle: number, spacing: number, width: number) => groupedHatch(angle, spacing, 2, 1, width);

/**
 * "Uygun aralıklarla sınır üzerine … yazılacak": the code, red, 2.5 mm,
 * on every sixth line piece, standing on the inner side (outside when the
 * inner side carries ticks).
 */
const boundaryCode = (value: string, period: number, piece: number, inside = true) =>
  along(text(value, 2.5, { font: 'sans', weight: 700, color: RED }), period * 6, { offsetAlong: piece / 2, offset: inside ? 1.9 : -1.9 });

/**
 * Sit boundary: 5 mm line, then `n` tangent 3 mm red discs (EK-1e: "5 mm
 * düz çizgi aralıklı 3 mm çaplı n adet dolu daire").
 */
function discLine(n: number, codeText: string | null, withTicks = false) {
  const period = 5 + 3 * n;
  return [
    stroke(RED, 0.3, { dash: [5, 3 * n] }),
    along(circle(3, { fill: RED }), period, { offsetAlong: 5 + 1.5 * n, rotate: true, group: n > 1 ? { count: n, spacing: 3 } : undefined }),
    // Doğal sit: 2.5 mm ticks at 45° every 3 mm, inside the area.
    ...(withTicks ? [ticks(RED, 0.3, 2.5, 3, 1, { angle: 135 })] : []),
    ...(codeText ? [boundaryCode(codeText, period, 5, !withTicks)] : []),
  ];
}

/** Doğal sit hatch weight per level (EK-1e s.31–33). */
const NATURAL_W: Record<Level, number> = { uip: 0.4, nip: 0.3, cdp: 0.2 };
/** 1. derece arkeolojik sit: pair spacing per level (EK-1e s.79). */
const A1_SPACING: Record<Level, number> = { uip: 4, nip: 3, cdp: 3 };
/** Kentsel sit: hatch weight per level (EK-1e s.82). */
const KS_W: Record<Level, number> = { uip: 0.3, nip: 0.4, cdp: 0.4 };
/** Triangle side (and the flora/fauna dot) per level: UİP and NİP 4 mm, ÇDP 3 mm (EK-1e s.85–90). */
const SIDE: Record<Level, number> = { uip: 4, nip: 4, cdp: 3 };
/** Uluslararası sözleşmeler: the dot in the triangle (EK-1e s.85). */
const INTL_DOT: Record<Level, number> = { uip: 2, nip: 2, cdp: 1 };
/** ÖÇK circles per level (EK-1e s.87–89). */
const OCK_D: Record<Level, number> = { uip: 4, nip: 4, cdp: 3 };
/** ÖÇK hassas alan (C): pair spacing per level (EK-1e s.89). */
const OCK_C: Record<Level, number> = { uip: 5, nip: 3, cdp: 3 };

const PARK_FILL = rgb(36, 156, 34);

const sitNote = (hatch: string, n: number, codeText: string | null) =>
  `Şeffaf. Tarama: ${hatch}. Sınır 0.3 mm kırmızı (255/0/0): 5 mm düz çizgi aralıklı ${n > 1 ? `${n} adet teğet ` : ''}3 mm çaplı dolu daire${codeText ? `; "uygun aralıklarla" sınır üzerine ${codeText}: her altıncı çizgi parçasının içine 2.5 mm kırmızı` : ''}.`;

export const SIT = sections(['Korunacak alanlar', 'Sit ve korunacak alanlar'], 'sit', 1040, (s, lv) => {
  const S = SIDE[lv];

  const a1 = A1_SPACING[lv];
  s.area('1-derece-arkeolojik-sit-alani', '1. derece arkeolojik sit alanı', [pairs(0, a1, 0.4), discLine(1, 'A1'), code(lv, 'A1')], {
    ref: 'EK-1a s.3; EK-1e s.79',
    note: sitNote(`0.4 mm, 1 mm aralıklı çift yatay çizgiler, çiftler arası ${a1} mm`, 1, 'A1'),
  });
  s.area('2-derece-arkeolojik-sit-alani', '2. derece arkeolojik sit alanı', [pairs(0, 6, 0.4), discLine(2, 'A2'), code(lv, 'A2')], {
    ref: 'EK-1a s.3; EK-1e s.80',
    note: sitNote('0.4 mm, 1 mm aralıklı çift yatay çizgiler, çiftler arası 6 mm', 2, 'A2'),
  });
  s.area('3-derece-arkeolojik-sit-alani', '3. derece arkeolojik sit alanı', [pairs(0, 6, 0.4), discLine(3, 'A3'), code(lv, 'A3')], {
    ref: 'EK-1a s.3; EK-1e s.80',
    note: sitNote('0.4 mm, 1 mm aralıklı çift yatay çizgiler, çiftler arası 6 mm', 3, 'A3'),
  });

  const nw = NATURAL_W[lv];
  const tickNote = ' İçeride çizgi eksenine 45° açılı, 3 mm aralıklı, 2.5 mm uzunluğunda tırnaklar; kod bu yüzden sınırın dışına yazıldı.';
  s.area('1-derece-dogal-sit-alani', '1. derece doğal sit alanı', [pairs(0, 3, nw), discLine(1, 'D1', true), code(lv, 'D1')], {
    ref: 'EK-1a s.3; EK-1e s.31',
    note: sitNote(`${nw} mm, 1 mm aralıklı çift yatay çizgiler, çiftler arası 3 mm`, 1, 'D1') + tickNote,
  });
  s.area('2-derece-dogal-sit-alani', '2. derece doğal sit alanı', [pairs(0, 6, nw), discLine(2, 'D2', true), code(lv, 'D2')], {
    ref: 'EK-1a s.3; EK-1e s.32',
    note: sitNote(`${nw} mm, 1 mm aralıklı çift yatay çizgiler, çiftler arası 6 mm`, 2, 'D2') + tickNote,
  });
  s.area('3-derece-dogal-sit-alani', '3. derece doğal sit alanı', [pairs(0, 6, nw), discLine(3, 'D3', true), code(lv, 'D3')], {
    ref: 'EK-1a s.3; EK-1e s.33',
    note: sitNote(`${nw} mm, 1 mm aralıklı çift yatay çizgiler, çiftler arası 6 mm`, 3, 'D3') + tickNote,
  });

  s.area('kesin-korunacak-hassas-alan', 'Kesin korunacak hassas alan', [pairs(0, 3, 0.4), discLine(1, 'KK'), code(lv, 'KK', { weight: 400 })], {
    ref: 'EK-1a s.3; EK-1e s.81',
    note: sitNote('0.4 mm, 1 mm aralıklı çift yatay çizgiler, çiftler arası 3 mm', 1, 'KK') + ' Sembol: çerçevede ince (normal) "KK".',
  });
  s.area('nitelikli-dogal-koruma-alani', 'Nitelikli doğal koruma alanı', [pairs(0, 6, 0.4), discLine(2, 'ND'), code(lv, 'ND', { weight: 400 })], {
    ref: 'EK-1a s.3; EK-1e s.81',
    note: sitNote('0.4 mm, 1 mm aralıklı çift yatay çizgiler, çiftler arası 6 mm', 2, 'ND'),
  });
  s.area('surdurulebilir-koruma-ve-kontrollu-kullanim-alani', 'Sürdürülebilir koruma ve kontrollü kullanım alanı', [pairs(0, 6, 0.3), discLine(3, 'SK'), code(lv, 'SK', { weight: 400 })], {
    ref: 'EK-1a s.3; EK-1e s.82',
    note: sitNote('0.3 mm, 1 mm aralıklı çift yatay çizgiler, çiftler arası 6 mm', 3, 'SK'),
  });

  const ks = KS_W[lv];
  s.area('kentsel-sit-alani', 'Kentsel sit alanı', [pairs(0, 3, ks), discLine(1, 'KS'), code(lv, 'K')], {
    ref: 'EK-1a s.4; EK-1e s.82',
    note: sitNote(`${ks} mm, 1 mm aralıklı çift yatay çizgiler, çiftler arası 3 mm`, 1, 'KS') + ' Alan sembolü "K", sınır kodu "KS" (EK-1e).',
  });
  s.area('kentsel-arkeolojik-sit-alani', 'Kentsel arkeolojik sit alanı', [pairs(0, 3, 0.4), discLine(1, 'KA'), code(lv, 'KA', { font: 'serif', weight: 700, heavy: true })], {
    ref: 'EK-1a s.4; EK-1e s.83',
    note: sitNote('0.4 mm, 1 mm aralıklı çift yatay çizgiler, çiftler arası 3 mm', 1, 'KA') + ' Sembol: kalın çerçevede kalın Times "KA" (EK-1a).',
  });
  s.area('tarihi-sit-alani', 'Tarihi sit alanı', [pairs(0, 3, 0.4), discLine(1, lv === 'cdp' ? null : 'TS'), code(lv, 'T')], {
    ref: 'EK-1a s.4; EK-1e s.84',
    note: sitNote('0.4 mm, 1 mm aralıklı çift yatay çizgiler, çiftler arası 3 mm', 1, lv === 'cdp' ? null : 'TS') + (lv === 'cdp' ? ' ÇDP satırında sınır kodu yok (EK-1e).' : ' Alan sembolü "T", sınır kodu "TS".'),
  });

  // Triangles 4 mm apart, a 2.7 mm dash in the middle of each interval (dash measured from EK-1a/EK-1e).
  const dot = INTL_DOT[lv];
  const ip = S + 4;
  const lift = (S * Math.sqrt(3)) / 6;
  s.line(
    'uluslararasi-sozlesmelerle-belirlenen-koruma-alan-siniri',
    'Uluslararası sözleşmelerle belirlenen koruma alan sınırı',
    [
      stroke(RED, 0.3, { dash: [2.7, ip - 2.7], dashOffset: ip - (S + 0.65) }),
      along([triangleOnLine(S, { stroke: RED, strokeWidth: 0.3 }), circle(dot, { fill: RED, offset: [0, Math.round(lift * 1000) / 1000] })], ip, { offsetAlong: S / 2 }),
    ],
    {
      ref: 'EK-1a s.4; EK-1e s.85',
      note: `Şeffaf. 0.3 mm kırmızı: 4 mm aralıklı ${S} mm kenarlı boş eşkenar üçgenler (tabanı çizgide, tepesi alanın içine), içlerinde ${dot} mm çaplı nokta; aradaki 2.7 mm çizgi parçası çizimden.`,
    },
  );

  // Milli park, tabiat parkı, yaban hayatı: triangles 1 mm apart, 1 mm gap, 5 mm line, 1 mm gap.
  const tri = (filled: boolean) => (filled ? triangleOnLine(S, { fill: RED }) : triangleOnLine(S, { stroke: RED, strokeWidth: 0.3 }));
  const triGroup = (n: number, filled: boolean) => {
    const gap = 1 + n * S + (n - 1) + 1;
    return [stroke(RED, 0.3, { dash: [5, gap] }), along(tri(filled), 5 + gap, { offsetAlong: 5 + gap / 2, group: n > 1 ? { count: n, spacing: S + 1 } : undefined })];
  };
  const triNote = (what: string) => `Alan 36/156/34. Sınır 0.3 mm kırmızı: ${what}; üçgenlerin tabanı çizgide, tepesi alanın içine; üçgenler arası 1 mm (çizimden).`;
  s.area('milli-park', 'Milli park', [solid(PARK_FILL), triGroup(3, true), picto(lv, 'uc-konifer', { caption: 'MP' })], {
    ref: 'EK-1a s.4; EK-1e s.85',
    note: triNote(`${S} mm kenarlı 3 dolu eşkenar üçgen, 1 mm boşluk, 5 mm düz çizgi, 1 mm boşluk`),
  });
  s.area('tabiat-parki-alani', 'Tabiat parkı alanı', [solid(PARK_FILL), triGroup(3, false), picto(lv, 'iki-katli-konifer', { caption: 'TP' })], {
    ref: 'EK-1a s.4; EK-1e s.86',
    note: triNote(`${S} mm kenarlı 3 boş eşkenar üçgen, 1 mm boşluk, 5 mm düz çizgi, 1 mm boşluk`),
  });
  const tkGap = 2 + S + 2;
  s.area(
    'tabiati-koruma-alani',
    'Tabiatı koruma alanı',
    [solid(PARK_FILL), stroke(RED, 0.3, { dash: [3, tkGap] }), along(tri(false), 3 + tkGap, { offsetAlong: 3 + tkGap / 2 }), picto(lv, 'uc-kucuk-konifer', { caption: 'TKA' })],
    { ref: 'EK-1a s.4; EK-1e s.86', note: `Alan 36/156/34. Sınır 0.3 mm kırmızı: ${S} mm kenarlı boş eşkenar üçgen, 2 mm boşluk, 3 mm düz çizgi, 2 mm boşluk. Sembol: üç küçük iki katlı çam, altında TKA.` },
  );
  s.area('yaban-hayati-koruma-ve-gelistirme-alani', 'Yaban hayatı koruma ve geliştirme alanı', [solid(PARK_FILL), triGroup(2, true), picto(lv, 'geyik')], {
    ref: 'EK-1a s.4; EK-1e s.87',
    note: triNote(`1 mm aralıkla ${S} mm kenarlı 2 dolu eşkenar üçgen, 1 mm boşluk, 5 mm düz çizgi (sonraki 1 mm boşluk ve üçgen sayısı çizimden)`),
  });

  // ÖÇK: three tangent circles, 1 mm gap, 5 mm line, 1 mm gap.
  const d = OCK_D[lv];
  const ockGap = 1 + 3 * d + 1;
  const ockPeriod = 5 + ockGap;
  const ockMid = 5 + ockGap / 2;
  const hollow = circle(d, { stroke: RED, strokeWidth: 0.3 });
  s.line('ozel-cevre-koruma-bolgesi', 'Özel çevre koruma bölgesi', [stroke(RED, 0.3, { dash: [5, ockGap] }), along(hollow, ockPeriod, { offsetAlong: ockMid, group: { count: 3, spacing: d } })], {
    ref: 'EK-1a s.4; EK-1e s.87',
    note: `Şeffaf. 0.3 mm kırmızı: ${d} mm çaplı teğet 3 boş daire, 1 mm boşluk, 5 mm düz çizgi, 1 mm boşluk (son boşluk çizimden).`,
  });
  const ockLine = [
    stroke(RED, 0.3, { dash: [5, ockGap] }),
    along(hollow, ockPeriod, { offsetAlong: ockMid, group: { count: 2, spacing: 2 * d } }),
    along(circle(d, { fill: RED, stroke: RED, strokeWidth: 0.3 }), ockPeriod, { offsetAlong: ockMid }),
  ];
  const ockNote = (hatch: string) => `Şeffaf. Tarama: ${hatch}. Sınır 0.3 mm kırmızı: ${d} mm çaplı, ortadaki dolu teğet 3 daire, 1 mm boşluk, 5 mm düz çizgi, 1 mm boşluk.`;
  s.area('ock-bolgesi-hassas-alan-a', 'ÖÇK bölgesi hassas alan (A)', [pairs(45, 3, 0.4), ockLine, code(lv, 'A')], {
    ref: 'EK-1a s.4; EK-1e s.88',
    note: ockNote('0.4 mm, 1 mm aralıklı 45° çift çizgi, çiftler arası 3 mm'),
  });
  s.area('ock-bolgesi-hassas-alan-b', 'ÖÇK bölgesi hassas alan (B)', [groupedHatch(0, 7, 3, 1, 0.4), ockLine, code(lv, 'B')], {
    ref: 'EK-1a s.4; EK-1e s.88',
    note: ockNote('0.4 mm, 1 mm aralıklı üç yatay çizgi, üçlüler arası 7 mm (açı metinde yok, çizimde yatay)'),
  });
  s.area('ock-bolgesi-hassas-alan-c', 'ÖÇK bölgesi hassas alan (C)', [pairs(135, OCK_C[lv], 0.4), ockLine, code(lv, 'C')], {
    ref: 'EK-1a s.4; EK-1e s.89',
    note: ockNote(`0.4 mm, 1 mm aralıklı 135° çift çizgi, çiftler arası ${OCK_C[lv]} mm`),
  });

  // Flora/fauna and ENK: three hollow triangles with a 2 mm dot, 1 mm apart, then a 7 mm line.
  const ffGap = 1 + 3 * S + 2 + 1;
  const ffLine = [
    stroke(RED, 0.3, { dash: [7, ffGap] }),
    along([triangleOnLine(S, { stroke: RED, strokeWidth: 0.3 }), circle(2, { fill: RED, offset: [0, Math.round(lift * 1000) / 1000] })], 7 + ffGap, { offsetAlong: 7 + ffGap / 2, group: { count: 3, spacing: S + 1 } }),
  ];
  const ffNote = `Şeffaf. Tarama: 0.4 mm, 1 mm aralıklı 45° çift çizgi, çiftler arası 7 mm. Sınır 0.3 mm kırmızı: 1 mm aralıkla ${S} mm kenarlı 3 eşkenar üçgen, içlerinde 2 mm çaplı nokta, 7 mm düz çizgi (üçgen grubunun iki yanındaki 1 mm boşluk çizimden).${lv === 'cdp' ? ' ÇDP: 3 mm üçgende 2 mm nokta metindeki gibi; üçgenin iç teğet dairesinden (1.7 mm) büyüktür.' : ''}`;
  s.area('korunmasi-gerekli-flora-ve-fauna-alani', 'Korunması gerekli flora ve fauna alanı', [pairs(45, 7, 0.4), ffLine, code(lv, 'FF')], { ref: 'EK-1a s.5; EK-1e s.89', note: ffNote });
  s.area('ekolojik-niteligi-korunacak-alan', 'Ekolojik niteliği korunacak alan', [pairs(45, 7, 0.4), ffLine, code(lv, 'ENK', { weight: 400 })], {
    ref: 'EK-1a s.5; EK-1e s.90',
    note: ffNote + ' Sembol: çerçevede normal Arial "ENK".',
  });
});

