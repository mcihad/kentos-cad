import { BLACK, WHITE, pattern, rgb, shape, sheet, solid, text } from '../dsl';
import { S, box, freeDots, picture, stack, turned } from './common';

/**
 * NİP (EK-1ç) › Açık ve yeşil alanlar. Taramaların çoğu "serbest noktalama":
 * kalem kalınlığında rastgele noktalar; sıklık yazılı değil, EK-1ç örneği
 * gibi cm² başına yaklaşık 20 nokta (EK-1e s.108, 134-143).
 */

const KOYU = rgb(36, 156, 34);
const ACIK = rgb(99, 186, 82);

/** "10 mm karolaj merkezlerinde 3 mm kenarlı içi boş eşkenar üçgen". */
const triangles = () => pattern(shape('triangle', 3, { stroke: BLACK, strokeWidth: 0.3 }), 10, 10);

export const yesil = sheet('nip', ['Açık ve yeşil alanlar'], 'yesil', 70);

yesil.area('park-ve-yesil-alan', 'Park ve yeşil alan', [solid(KOYU), freeDots(0.3, 1), stack(text('PARK', 3, { font: 'sans', weight: 700, halo: { color: WHITE, width: 0.3 } }))], {
  ref: 'EK-1ç s.6; EK-1e s.134',
  note: '0,3 mm serbest noktalama. "PARK" çerçevesiz kalın Arial; yazı boyu yazılı değil, 3 mm alındı.',
});
yesil.area('pasif-yesil-alan', 'Pasif yeşil alan', [solid(ACIK), freeDots(0.3, 2)], { ref: 'EK-1ç s.7; EK-1e s.135', note: '0,3 mm serbest noktalama.' });
yesil.area('rekreasyon-alani', 'Rekreasyon alanı', [solid(KOYU), freeDots(0.3, 3), picture('rekreasyon', undefined, { heavy: true })], { ref: 'EK-1ç s.7; EK-1e s.137', note: '0,3 mm serbest noktalama.' });
yesil.area('fuar-panayir-ve-festival-gosteri-alani', 'Fuar, panayır ve festival/gösteri alanı', [solid(KOYU), freeDots(0.3, 4), picture('fuar', undefined, { heavy: true })], {
  ref: 'EK-1ç s.7; EK-1e s.138',
  note: '0,3 mm serbest noktalama.',
});
yesil.area('kent-ormani', 'Kent ormanı', [solid(ACIK), triangles(), picture('kent-ormani')], {
  ref: 'EK-1ç s.7; EK-1e s.141',
  note: '0,3 mm, 10 mm karolaj merkezlerinde 3 mm kenarlı içi boş eşkenar üçgen (tepesi yukarı). EK-1ç örneği üçgenleri gri ve sık basmış; ölçüler EK-1e\'den.',
});
yesil.area('agaclandirilacak-alan', 'Ağaçlandırılacak alan', [solid(ACIK), triangles(), picture('iki-konifer')], {
  ref: 'EK-1ç s.7; EK-1e s.142',
  note: '0,3 mm, 10 mm karolaj merkezlerinde 3 mm kenarlı içi boş eşkenar üçgen (tepesi yukarı).',
});

/** An open trapezoid "\_/" in the lower half of a frame (EK-1ç s.7). */
const bowl = { x0: 1.4, y0: -1.9, x1: 2.66, y1: -0.2 };
const bowlLen = Math.hypot(bowl.x1 - bowl.x0, bowl.y1 - bowl.y0);
const bowlDeg = (Math.atan2(bowl.y1 - bowl.y0, bowl.x1 - bowl.x0) * 180) / Math.PI;
const mezar = stack([
  shape('square', S, { fill: WHITE, stroke: BLACK, strokeWidth: 0.25 }),
  shape('line', 2 * bowl.x0, { stroke: BLACK, strokeWidth: 0.3, offset: [0, bowl.y0] }),
  shape('line', bowlLen, { stroke: BLACK, strokeWidth: 0.3, rotation: bowlDeg, offset: turned((bowl.x0 + bowl.x1) / 2, (bowl.y0 + bowl.y1) / 2, bowlDeg) }),
  shape('line', bowlLen, { stroke: BLACK, strokeWidth: 0.3, rotation: 180 - bowlDeg, offset: turned(-(bowl.x0 + bowl.x1) / 2, (bowl.y0 + bowl.y1) / 2, 180 - bowlDeg) }),
]);
yesil.area('mezarlik-alani', 'Mezarlık alanı', [solid(ACIK), freeDots(0.3, 5), mezar], { ref: 'EK-1ç s.7; EK-1e s.143', note: '0,3 mm serbest noktalama. Sembol çerçevede üstü açık yamuk.' });
yesil.area('mesire-yeri', 'Mesire yeri', [solid(KOYU), freeDots(0.4, 6), picture('piknik-masasi', undefined, { heavy: true })], { ref: 'EK-1ç s.7; EK-1e s.139', note: '0,4 mm serbest noktalama.' });
yesil.area('rekreaktif-alan', 'Rekreaktif alan', [solid(rgb(36, 186, 34)), freeDots(0.3, 7), box('RA', { heavy: true, max: 4 })], {
  ref: 'EK-1ç s.7 (AÇIKLAMA 20); EK-1e s.108',
  note: '0,3 mm serbest noktalama. Renk 36/186/34 (öteki yeşil alanların 36/156/34\'ünden farklı; iki ekte de böyle).',
});
