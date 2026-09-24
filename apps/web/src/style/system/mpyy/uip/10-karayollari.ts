import { BLACK, WHITE, along, circle, grid, hatch, label, rgb, shape, sheet, solid, stroke, text } from '../dsl';
import { CAPTION, FRAME, FRAME_LINE, code, metresToMm, picto, seg, sidePair } from './common';

/**
 * UİP (EK-1d s.14–15) › Teknik altyapı › Ulaşım › Karayolları (EK-1e
 * s.147–156). Roads are drawn "at their real width": the object is the
 * road's axis and the widths are data (metres), turned into offsets on
 * paper at the drawing's scale ($ölçek). Only the lines are drawn; the
 * carriageway between them is left transparent.
 */

/** The legend's top sections; their children carry the items. */
export const teknikAltyapi = sheet('uip', ['Teknik altyapı'], 'teknik-altyapi', 100);
export const ulasim = sheet('uip', ['Teknik altyapı', 'Ulaşım'], 'ulasim', 10);

export const karayollari = sheet('uip', ['Teknik altyapı', 'Ulaşım', 'Karayolları'], 'karayollari', 10);

const KERB = rgb(255, 0, 0);
const MEDIAN = rgb(99, 186, 82);

/**
 * A road: cephe lines (black, `cephe` mm) on its edges, kerb lines (red
 * 0.3 mm) inside them by the sidewalk width, and, with `r0`, the refüj's
 * two green 0.3 mm lines.
 */
function road(cephe: number, w0: number, k0: number, r0?: number) {
  const W = `varsayılan([Genişlik], ${w0})`;
  const layers = [...sidePair(BLACK, cephe, W, w0), ...sidePair(KERB, 0.3, `${W} - 2 * varsayılan([Kaldırım], ${k0})`, w0 - 2 * k0)];
  if (r0 !== undefined) layers.push(...sidePair(MEDIAN, 0.3, `varsayılan([Refüj], ${r0})`, r0));
  return layers;
}

const ROAD_NOTE = (w0: number, k0: number, r0?: number) =>
  `Çizgi yolun eksenidir; yol gerçek genişliğinde çizilir: "Genişlik" (cepheden cepheye, m; boşsa ${w0}), "Kaldırım" (m; boşsa ${k0})${r0 !== undefined ? `, "Refüj" (m; boşsa ${r0})` : ''} alanlarından. Çizgiler eksenden çizim ölçeğine göre kaydırılır; araları şeffaftır. Lejanttaki "…" yazılı elips kesme (devam) işaretidir, çizgi tipine dahil değildir.`;

karayollari.line('erisme-kontrollu-karayolu-otoyol', 'Erişme kontrollü karayolu (otoyol)', [road(2.5, 16, 2.5, 2)], {
  ref: 'EK-1d s.14; EK-1e s.147',
  note: `Cephe çizgisi 2.5 mm siyah, kaldırım 0.3 mm kırmızı (255/0/0), refüj 0.3 mm yeşil (99/186/82) çift çizgi. ${ROAD_NOTE(16, 2.5, 2)}`,
});
karayollari.line('bolunmus-tasit-yolu', 'Bölünmüş taşıt yolu', [road(1.6, 14, 2.5, 1.5)], {
  ref: 'EK-1d s.14; EK-1e s.150',
  note: `Cephe çizgisi 1.6 mm siyah, kaldırım 0.3 mm kırmızı, refüj 0.3 mm yeşil çift çizgi. ${ROAD_NOTE(14, 2.5, 1.5)}`,
});
karayollari.line('tasit-yolu', 'Taşıt yolu', [road(1, 10, 2)], {
  ref: 'EK-1d s.14; EK-1e s.151',
  note: `Cephe çizgisi 1 mm siyah, kaldırım 0.3 mm kırmızı; refüj yok (metin refüj çizgisinden söz eder, lejant ve çizim göstermez). ${ROAD_NOTE(10, 2)}`,
});

const OTOPARK = rgb(178, 178, 178);
karayollari.area('genel-otopark-alani', 'Genel otopark alanı', [solid(OTOPARK), code('P', 6.5, { weight: 400 })], { ref: 'EK-1d s.14; EK-1e s.152', note: 'Düz dolgu 178/178/178, çerçevede ince "P".' });
karayollari.area('tir-kamyon-makine-parki-ve-garaj-alani', 'Tır, kamyon, makine parkı ve garaj alanı', [solid(rgb(255, 56, 0)), grid(3, 3, 0.2), picto('tir', { size: 9, caption: 'PG' })], {
  ref: 'EK-1d s.14; EK-1e s.153',
  note: '0.2 mm, 3 mm ara ile karolaj.',
});

/** Bicycle (no pictogram in the set): two wheels and a line frame, 8.4 mm wide. */
const bicycle = () => [
  circle(3.3, { stroke: BLACK, strokeWidth: 0.25, offset: [-2.55, -0.9] }),
  circle(3.3, { stroke: BLACK, strokeWidth: 0.25, offset: [2.55, -0.9] }),
  seg(-2.55, -0.9, -0.2, -0.9, 0.25),
  seg(-0.2, -0.9, -1.1, 1.3, 0.25),
  seg(-1.1, 1.3, -2.55, -0.9, 0.25),
  seg(-1.1, 1.3, 1.7, 1.3, 0.25),
  seg(1.7, 1.3, -0.2, -0.9, 0.25),
  seg(1.7, 1.3, 2.55, -0.9, 0.25),
  seg(1.7, 1.3, 1.4, 2.1, 0.25),
  seg(1.4, 2.1, 2.2, 2.1, 0.25),
  seg(-1.7, 1.55, -0.6, 1.55, 0.35),
];
const bicycleMarks = () => [shape('square', FRAME, { fill: WHITE, stroke: BLACK, strokeWidth: FRAME_LINE }), ...bicycle()];

karayollari.line(
  'bisiklet-yolu',
  'Bisiklet yolu',
  [
    ...sidePair(BLACK, 0.3, 'varsayılan([Genişlik], 3)', 3),
    // Rungs across the whole path every 3 mm: a line marker as long as the width.
    along({ ...shape('line', 3, { stroke: BLACK, strokeWidth: 0.3, rotation: 90 }), size: { expr: metresToMm('varsayılan([Genişlik], 3)'), fallback: 3 } }, 3, { offsetAlong: 1.5 }),
    along(bicycleMarks(), 0, { placement: 'center', rotate: false }),
  ],
  {
    ref: 'EK-1d s.14; EK-1e s.153',
    note: 'Çizgi yolun eksenidir: 0.3 mm iki kenar çizgisi yolun gerçek genişliğinde ("Genişlik", m; boşsa 3) ve 3 mm aralıkla yola dik çizgiler. Lejantın sembol sütunundaki bisiklet çizginin ortasına konur (piktogram setinde bisiklet yok: şekillerden kuruldu).',
  },
);
karayollari.area('bisiklet-parki', 'Bisiklet parkı', [solid(rgb(255, 229, 207)), label(bicycleMarks())], {
  ref: 'EK-1d s.15; EK-1e s.154',
  note: 'Düz dolgu 255/229/207 ve çerçevede bisiklet (piktogram setinde yok: şekillerden kuruldu).',
});
karayollari.line('yaya-yolu-ve-bolgesi', 'Yaya yolu ve bölgesi', [sidePair(BLACK, 0.4, 'varsayılan([Genişlik], 5)', 5)], {
  ref: 'EK-1d s.15; EK-1e s.155',
  note: 'Çizgi yolun eksenidir: 0.4 mm iki cephe çizgisi gerçek genişlikte ("Genişlik", m; boşsa 5); arası şeffaf. Lejanttaki "…" yazılı elips kesme işaretidir.',
});

/**
 * Köprü and tünel (EK-1d s.15): each deck (or tunnel) edge is drawn as its
 * own line at its real place; both ends turn outward in a 3 mm wing at 45°.
 * The wings go to the left of the drawing direction, so each edge is drawn
 * with the outside on its left.
 */
const winged = (width: number) => [
  stroke(BLACK, width),
  along(seg(0, 0, -3 * Math.SQRT1_2, 3 * Math.SQRT1_2, width), 0, { placement: 'first' }),
  along(seg(0, 0, 3 * Math.SQRT1_2, 3 * Math.SQRT1_2, width), 0, { placement: 'last' }),
];
const WING_NOTE =
  '"Gerçek ölçüleri ile çizilir": her kenar ayrı çizgi olarak gerçek yerinde çizilir; iki ucu 45° dışa kırılır (3 mm, EK-1d\'den). Kanat çizim yönünün soluna döner: kenarı dışı solda kalacak yönde çizin. Kalınlık verilmemiş: EK-1d\'deki gibi 0.5 mm.';
karayollari.line('kopru', 'Köprü', [winged(0.5)], { ref: 'EK-1d s.15; EK-1e s.155', note: WING_NOTE });

/** Yaya geçitleri: the walkway along its axis at real width, three stair rungs at each end. */
const stairs = (w0: number) =>
  ['first', 'last'].map((p) =>
    along({ ...shape('line', w0, { stroke: BLACK, strokeWidth: 0.25, rotation: 90 }), size: { expr: metresToMm(`varsayılan([Genişlik], ${w0})`), fallback: w0 } }, 0, {
      placement: p as 'first' | 'last',
      group: { count: 5, spacing: 0.8 },
    }),
  );
karayollari.line('yaya-ust-gecidi', 'Yaya üst geçidi', [...sidePair(BLACK, 0.5, 'varsayılan([Genişlik], 3)', 3), ...stairs(3)], {
  ref: 'EK-1d s.15; EK-1e s.156',
  note: 'Gerçek ölçüleri ile çizilir: çizgi geçidin eksenidir, iki kenar 0.5 mm gerçek genişlikte ("Genişlik", m; boşsa 3), iki uçta basamak çizgileri (0.8 mm aralı, beş; EK-1e 4–5 basamak der). Kalınlıklar verilmemiş, EK-1d\'den.',
});
karayollari.line('yaya-alt-gecidi', 'Yaya alt geçidi', [...sidePair(BLACK, 0.3, 'varsayılan([Genişlik], 3)', 3, { dash: [2, 1] }), ...stairs(3)], {
  ref: 'EK-1d s.15; EK-1e s.156',
  note: 'Gerçek ölçüleri ile çizilir; yeraltındaki izleri 2 mm çizgi, 1 mm ara. Çizgi geçidin eksenidir, iki kenar gerçek genişlikte ("Genişlik", m; boşsa 3), iki uçta basamak çizgileri. EK-1d giriş merdivenini ayrıca kutu olarak çizer; bu çizgi tipinde yok.',
});
karayollari.line('tunel', 'Tünel', [winged(0.5)], { ref: 'EK-1d s.15; EK-1e s.154', note: WING_NOTE });
karayollari.area(
  'katli-otopark',
  'Katlı otopark',
  [
    solid(OTOPARK),
    label([
      shape('rectangle', 15, { height: 14, fill: WHITE, stroke: BLACK, strokeWidth: FRAME_LINE }),
      text('P', 8, { font: 'sans', weight: 400, offset: [-1.2, 0.6] }),
      text('k', 4.5, { font: 'sans', weight: 400, offset: [2.4, -1.4] }),
    ]),
  ],
  { ref: 'EK-1d s.15; EK-1e s.152', note: 'Düz dolgu 178/178/178; 15×14 mm çerçevede "Pk" (ölçüler EK-1d\'den).' },
);
karayollari.area('elektrikli-arac-sarj-istasyonu-alani', 'Elektrikli araç şarj istasyonu alanı', [solid(OTOPARK), picto('sarj-istasyonu', { frame: 'rect', frameSize: 16, height: 13.5, size: 13, caption: 'EA', captionSize: CAPTION, captionFont: 'sans' })], {
  ref: 'EK-1d s.15; EK-1e s.64',
  note: 'Düz dolgu 178/178/178; renkli şarj piktogramı (112/185/71) 16×13.5 mm çerçevede, altında kalın Arial "EA".',
});
karayollari.area('elektronik-haberlesme-altyapi-alani', 'Elektronik haberleşme altyapı alanı', [solid(OTOPARK), hatch(0, 6, 0.2), hatch(45, 2, 0.2), picto('anten', { size: 9 })], {
  ref: 'EK-1d s.15; EK-1e s.182',
  note: '0.2 mm, 6 mm aralıklı yatay çizgiler ve 2 mm aralıklı 45° çizgiler.',
});
karayollari.area(
  'motokurye-park-alani',
  'Motokurye park alanı',
  [solid(rgb(255, 183, 185)), label([shape('rectangle', 24, { height: 18, fill: WHITE, stroke: BLACK, strokeWidth: 0.75 }), shape('rectangle', 9.8, { height: 12, stroke: BLACK, strokeWidth: FRAME_LINE })]), picto('motosiklet', { frame: 'none', size: 8 })],
  { ref: 'EK-1d s.15; EK-1e s.192', note: 'Düz dolgu 255/183/185. Sembol: kalın 24×18 mm çerçeve içinde 9.8×12 mm çerçevede motosiklet (ölçüler EK-1d\'den).' },
);

