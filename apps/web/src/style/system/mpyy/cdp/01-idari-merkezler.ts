import { BLACK, circle, rgb, sheet, text } from '../dsl';

/**
 * ÇDP (EK-1c) › İdari merkezler: il, ilçe, belde ve köy merkezleri. EK-1c
 * tek bir sembol gösterir; EK-1e (s.35-38) kademeye göre ölçüleri verir:
 * 0,3 mm siyah daire içinde dolu siyah daire, arası sarı. AÇIKLAMA 1 gereği
 * yerleşmenin adı sembolün yanına yazılır ("Ad" özniteliği).
 */

const YELLOW = rgb(255, 255, 0);

export const idariMerkezler = sheet('cdp', ['İdari merkezler'], 'idari-merkez', 10);

/** A bullseye `outer` mm wide with a black disc of `inner` mm, the settlement's name to its right. */
const centre = (outer: number, inner: number) => [
  circle(outer, { fill: YELLOW, stroke: BLACK, strokeWidth: 0.3 }),
  circle(inner, { fill: BLACK }),
  text({ expr: `varsayılan([Ad], '')` }, 2.5, { font: 'sans', weight: 700, anchor: 'left', offset: [outer / 2 + 1, 0] }),
];

const note = (outer: number, inner: number, page: number) =>
  `EK-1e s.${page}: 0,3 mm, ${outer} mm çaplı daire içinde ${inner} mm çaplı içi dolu daire. Aradaki sarı yalnız çizimde (255/255/0 okundu), metinde renk yok. Ad, "Ad" özniteliğinden sembolün sağına yazılır (yazı boyu yazılı değil, 2,5 mm).`;

idariMerkezler.point('il-merkezi', 'İl merkezi', [centre(8, 5)], { ref: 'EK-1c s.1 (AÇIKLAMA 1); EK-1e s.35', note: note(8, 5, 35) });
idariMerkezler.point('ilce-merkezi', 'İlçe merkezi', [centre(6, 3)], { ref: 'EK-1c s.1 (AÇIKLAMA 1); EK-1e s.35', note: note(6, 3, 35) });
idariMerkezler.point('belde-merkezi', 'Belde merkezi', [centre(3, 1)], { ref: 'EK-1c s.1 (AÇIKLAMA 1); EK-1e s.36', note: note(3, 1, 36) });
idariMerkezler.point('koy-merkezi', 'Köy merkezi', [centre(3, 1)], {
  ref: 'EK-1c s.1 (AÇIKLAMA 1); EK-1e s.38',
  note: `${note(3, 1, 38)} Belde merkezi ile aynı ölçüde; ikisi adlarıyla ayrılır.`,
});
