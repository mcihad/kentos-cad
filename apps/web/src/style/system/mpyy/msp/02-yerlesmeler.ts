import { BLACK, WHITE, circle, rgb, shape, sheet, solid, stroke } from '../dsl';

/**
 * MSP (EK-1e bölüm I, s.1-3) › Yerleşmeler sistemi ve şehirleşme:
 * yerleşmelerin kademelenmesi (üç alan rengi, üç kademe sembolü),
 * yerleşmeler arası ilişkiler, kırsal gelişim odakları, yeni şehirler.
 */

export const yerlesmeler = sheet('msp', ['Yerleşmeler sistemi ve şehirleşme'], 'yerlesmeler', 20);

const TIER_COLOURS = [
  { id: '1', rgb: [225, 165, 100] },
  { id: '2', rgb: [180, 110, 0] },
  { id: '3', rgb: [140, 85, 25] },
] as const;
for (const t of TIER_COLOURS) {
  yerlesmeler.area(`yerlesme-alani-${t.id}`, `Yerleşmeler ve kademelenmesi, alan rengi ${t.id} (${t.rgb.join('/')})`, [solid(rgb(t.rgb[0], t.rgb[1], t.rgb[2]))], {
    ref: 'EK-1e s.1',
    note: 'Kademelenme için üç alan rengi verilmiş (225/165/100, 180/110/0, 140/85/25); hangi rengin hangi kademeye ait olduğu yazılı değil, sıra EK-1e\'deki sıradır. Sınır verilmemiş.',
  });
}

/**
 * The tier symbols: orange (255/170/0, sampled from the drawing) and white
 * rings in a thin black outline; the ring widths are measured from the
 * drawing (diameters of the filled circles, outside in).
 */
const ORANGE = rgb(255, 170, 0);
const bullseye = (outer: number, rings: readonly number[]) => [
  circle(outer - 0.25, { fill: ORANGE, stroke: BLACK, strokeWidth: 0.25 }),
  ...rings.map((d, i) => circle(d, { fill: i % 2 ? ORANGE : WHITE })),
];
const TIERS = [
  { id: '1', outer: 11.1, rings: [8.37, 6.63, 4.47, 2.77] },
  { id: '2', outer: 8.5, rings: [6.3, 5.0, 3.3, 1.94] },
  { id: '3', outer: 6.5, rings: [4.5, 3.6] },
] as const;
for (const t of TIERS) {
  yerlesmeler.point(`yerlesme-kademesi-${t.id}`, `Yerleşmeler ve kademelenmesi, ${t.id}. kademe`, [bullseye(t.outer, t.rings)], {
    ref: 'EK-1e s.1',
    note: `${t.outer} mm; turuncu (255/170/0) ve beyaz halkalar, ince siyah dış çizgi. Ölçüler ve turuncu renk çizimden (MSP satırı ölçü ve sembol rengi vermiyor). Kademe büyükten küçüğe: 1, 2, 3.`,
  });
}

yerlesmeler.line('yerlesmeler-arasi-iliskiler', 'Yerleşmeler arası ilişkiler', [stroke(BLACK, 0.7, { dash: [2, 2] })], { ref: 'EK-1e s.2', note: '0,7 mm siyah; 2 mm düz çizgi, 2 mm boşluk.' });
yerlesmeler.point(
  'kirsal-gelisim-odaklari',
  'Kırsal gelişim odakları',
  [circle(5, { fill: BLACK }), circle(3.4, { fill: rgb(205, 255, 50) }), circle(2, { fill: BLACK })],
  { ref: 'EK-1e s.2', note: 'Sarı-yeşil (205/255/50) daire, kalın siyah dış halka ve siyah merkez. Ölçüler çizimden: dış 5 mm, sarı-yeşil halka 3,4 mm, merkez 2 mm.' },
);

/** A solid eight-armed asterisk with round arm ends. */
const RED = rgb(255, 0, 0);
yerlesmeler.point('yeni-sehirler', 'Yeni şehirler', [[0, 45, 90, 135].map((a) => shape('line', 5.3, { stroke: RED, strokeWidth: 1.2, rotation: a }))], {
  ref: 'EK-1e s.3',
  note: 'Kırmızı (255/0/0) sekiz kollu yıldız işareti; boyu (6,5 mm) ve kol kalınlığı (1,2 mm) çizimden.',
});
