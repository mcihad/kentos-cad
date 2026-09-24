import { BLACK, along, groupedHatch, hatch, rgb, shape, sheet, solid, stroke, text, ticks } from '../dsl';
import { code, metresToMm, picto, sidePair } from './common';

/**
 * UİP (EK-1d s.18–20) › Enerji üretim-dağıtım ve depolama; su-atıksu ve
 * atık sistemleri (EK-1e s.173–186, 99). EK-1d prints the water systems
 * as a bold sub-row after the energy uses; they are a section of their own
 * here.
 */

export const enerji = sheet('uip', ['Enerji üretim-dağıtım ve depolama'], 'enerji', 110);

const SANTRAL = rgb(171, 171, 200);
const TESIS = rgb(194, 158, 215);
/** "6 mm aralıklı yatay çizgiler ve 2 mm aralıklı 45 derecelik çizgiler", 0.2 mm. */
const energyHatch = () => [hatch(0, 6, 0.2), hatch(45, 2, 0.2)];
const HATCH = 'Tarama: 0.2 mm, 6 mm aralıklı yatay çizgiler ve 2 mm aralıklı 45° çizgiler.';
const bolt = (caption: string, font: 'serif' | 'sans' = 'serif') => picto('simsek', { size: 8.5, caption, captionFont: font });

enerji.area('nukleer-enerji-santrali-alani', 'Nükleer enerji santrali alanı', [solid(SANTRAL), energyHatch(), bolt('NES')], { ref: 'EK-1d s.18; EK-1e s.173', note: HATCH });
enerji.area('termik-santral-alani', 'Termik santral alanı', [solid(SANTRAL), energyHatch(), bolt('TS')], { ref: 'EK-1d s.18; EK-1e s.174', note: HATCH });
enerji.area('yenilenebilir-enerji-kaynaklarina-dayali-uretim-tesisi-alani', 'Yenilenebilir enerji kaynaklarına dayalı üretim tesisi alanı', [solid(SANTRAL), energyHatch(), bolt('YED')], {
  ref: 'EK-1d s.18; EK-1e s.174',
  note: HATCH,
});
enerji.area('rafineri-petrokimya-tesisi-alani', 'Rafineri-petrokimya tesisi alanı', [solid(TESIS), energyHatch(), code('R', 5)], { ref: 'EK-1d s.18; EK-1e s.179', note: HATCH });
enerji.area('enerji-depolama-alani', 'Enerji depolama alanı', [solid(TESIS), energyHatch(), bolt('EDA', 'sans')], { ref: 'EK-1d s.18; EK-1e s.178', note: HATCH });
enerji.line('iletim-tuneli', 'İletim tüneli', [stroke(BLACK, 0.8, { dash: [2, 2], cap: 'butt' })], {
  ref: 'EK-1d s.19; EK-1e s.175',
  note: '0.8 mm, 2 mm aralıklı kesikli çizgi; çizgi boyu verilmemiş, EK-1d\'deki gibi araya eşit (2 mm).',
});
enerji.line('cebri-boru-hatti', 'Cebri boru hattı', [stroke(BLACK, 0.8)], { ref: 'EK-1d s.19; EK-1e s.176', note: '0.8 mm düz çizgi.' });
enerji.area('regulator-alani', 'Regülatör alanı', [solid(TESIS), energyHatch(), bolt('R')], { ref: 'EK-1d s.19; EK-1e s.176', note: HATCH });
enerji.area('turbin-alani', 'Türbin alanı', [solid(TESIS), energyHatch(), bolt('T')], { ref: 'EK-1d s.19; EK-1e s.177', note: HATCH });
enerji.area('dogalgaz-dagitim-tesisi-alani', 'Doğalgaz/dağıtım tesisi alanı', [solid(TESIS), energyHatch(), bolt('DT')], {
  ref: 'EK-1d s.19; EK-1e s.177 (Doğalgaz iletim/dağıtım tesisi alanı)',
  note: HATCH,
});
enerji.area(
  'yanici-parlayici-ve-patlayici-maddeler-uretim-ve-depo-alani',
  'Yanıcı parlayıcı ve patlayıcı maddeler üretim ve depo alanı',
  [solid(TESIS), groupedHatch(45, 12, 2, 1, 0.2), hatch(135, 12, 0.2), picto('dinamit', { size: 9 })],
  { ref: 'EK-1d s.19; EK-1e s.178', note: 'Tarama: 0.2 mm, 45° yönde 1 mm tarama çiftleri, 135° yönde tek tarama, ara mesafe 12 mm.' },
);
enerji.area('akaryakit-urunleri-depolama-alani', 'Akaryakıt ürünleri depolama alanı', [solid(TESIS), energyHatch(), bolt('A', 'sans')], { ref: 'EK-1d s.19; EK-1e s.179', note: HATCH });

/** A filled equilateral triangle of 3 mm side pointing along the line (or back when `back`). */
const arrowTri = (back = false) => shape('arrowhead', (3 * Math.sqrt(3)) / 2, { height: 3 / 0.8, fill: BLACK, rotation: back ? 180 : 0 });

enerji.line('dogalgaz-boru-hatti', 'Doğalgaz boru hattı', [stroke(BLACK, 0.4, { dash: [3, 1] }), along(arrowTri(), 15, { offsetAlong: 7.5 })], {
  ref: 'EK-1d s.19; EK-1e s.180',
  note: '0.4 mm, 3 mm çizgi 1 mm boşluk kesikli çizgi üzerinde 15 mm aralıklı 3 mm kenarlı eşkenar üçgenler (akış yönünü gösterir: çizim yönü).',
});
enerji.line('akaryakit-boru-hatti', 'Akaryakıt boru hattı', [stroke(BLACK, 0.4), along(arrowTri(), 15, { offsetAlong: 7.5 })], {
  ref: 'EK-1d s.19; EK-1e s.181',
  note: '0.4 mm düz çizgi üzerinde 15 mm aralıklı 3 mm kenarlı eşkenar üçgenler (akış: çizim yönü).',
});
enerji.area('trafo-alani', 'Trafo alanı', [solid(rgb(178, 178, 178)), energyHatch(), picto('trafo', { size: 9 })], { ref: 'EK-1d s.19; EK-1e s.181', note: HATCH });

export const su = sheet('uip', ['Su-atıksu ve atık sistemleri'], 'su', 120);

su.line(
  'atik-su-ana-kollektoru',
  'Atık su ana kollektörü',
  [stroke(BLACK, 0.4), along(shape('chevron', 2.5, { height: 3, stroke: BLACK, strokeWidth: 0.4 }), 10, { offsetAlong: 5 })],
  { ref: 'EK-1d s.19; EK-1e s.185', note: '0.4 mm düz çizgi, üzerinde 10 mm aralıkla 3 mm uzunluğunda açık oklar (">", akış yönünde; ok ölçüsü EK-1d\'den 2.5×3 mm).' },
);
su.line(
  'atik-su-derin-deniz-desarj-hatti',
  'Atık su derin deniz deşarj hattı',
  [
    // The line stops at each triangle: dash 10 − 2.6 mm, the triangle (3 mm side, 2.6 mm long) fills the gap.
    stroke(BLACK, 0.4, { dash: [10 - (3 * Math.sqrt(3)) / 2, (3 * Math.sqrt(3)) / 2], cap: 'butt' }),
    along(shape('triangle', 3, { stroke: BLACK, strokeWidth: 0.2, rotation: -90 }), 10, { offsetAlong: 10 - (3 * Math.sqrt(3)) / 2 + Math.sqrt(3) / 2 }),
  ],
  { ref: 'EK-1d s.20; EK-1e s.186', note: '0.4 mm düz çizgi, 10 mm aralıkla 3 mm kenarlı içi boş eşkenar üçgenler (akış yönünde); çizgi üçgende kesilir. Üçgen çizgisi verilmemiş, EK-1d\'deki gibi ince (0.2 mm).' },
);
su.line(
  'sogutma-suyu-alma-hatti',
  'Soğutma suyu alma hattı',
  [stroke(BLACK, 0.4), along(arrowTri(true), 15, { offsetAlong: 0 }), along(text('S', 5, { font: 'sans', weight: 400, rotation: 0 }), 15, { offsetAlong: 7.5 })],
  {
    ref: 'EK-1d s.20; EK-1e s.183',
    note: '0.4 mm düz çizgi üzerinde 15 mm aralıklı 3 mm kenarlı eşkenar üçgenler ve iki üçgenin ortasında 5 mm "S". Üçgenler EK-1d\'deki gibi çizim yönünün tersine (alma yönüne) bakar.',
  },
);

/**
 * Sulama hattı: the canal's two lines at its real width ("Genişlik", m),
 * the channel colour between them, and 3 mm ticks every 15 mm from each
 * line into the channel, staggered by half.
 */
const CANAL = rgb(115, 223, 235);
const W = 'varsayılan([Genişlik], 8)';
const HALF = `${W} / 2`;
const edgeTicks = (side: 1 | -1, offsetAlong: number) => ({
  // On the left line (side 1) the ticks point right, into the channel, and the other way round.
  ...ticks(BLACK, 0.3, 3, 15, side === 1 ? -1 : 1, { offsetAlong }),
  offset: { expr: `${side < 0 ? '-' : ''}${metresToMm(HALF)}`, fallback: side * 4 },
});
su.line(
  'sulama-hatti',
  'Sulama hattı',
  [
    { ...stroke(CANAL, 8, { cap: 'butt' }), width: { expr: metresToMm(W), fallback: 8 } },
    ...sidePair(BLACK, 0.3, W, 8),
    edgeTicks(1, 3.75),
    edgeTicks(-1, 11.25),
    along(text({ expr: `varsayılan([Durum], '')` }, 2.5, { font: 'sans', weight: 700 }), 0, { placement: 'center' }),
  ],
  {
    ref: 'EK-1d s.20; EK-1e s.99',
    note: 'Çizgi kanalın eksenidir: kanal genişliğindeki ("Genişlik", m; boşsa 8) 0.3 mm iki hat çizgisi, araları 115/223/235, hat çizgilerine 15 mm aralıklarla dik 3 mm karşılıklı şaşırtmalı çizgiler. Açık, kapalı, yeraltı, hemzemin ya da havai hat olduğu hat üstünde belirtilir: "Durum" alanı doluysa eksene yazılır. Genişlik çizim ölçeğine göre kâğıda çevrilir.',
  },
);
