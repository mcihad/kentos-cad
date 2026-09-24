import { BLACK, WHITE, circle, label, pattern, rgb, shape, sheet, solid, text } from '../dsl';
import { FRAME, FRAME_LINE, code, freeDots, gridDots, picto, seg } from './common';

/**
 * UİP (EK-1d s.12–13) › Açık ve yeşil alanlar. "Serbest noktalama" is
 * random 0.4 mm dots (EK-1e gives no density; the legend's is used).
 */

export const acikYesil = sheet('uip', ['Açık ve yeşil alanlar'], 'acik-yesil', 80);

const YESIL = rgb(36, 156, 34);
const PASIF = rgb(99, 186, 82);
const FREE = 'Tarama: 0.4 mm serbest noktalama; yoğunluk verilmemiş, EK-1d örneği gibi ≈2.2 mm hücrede bir nokta.';

acikYesil.area('park', 'Park', [solid(YESIL), freeDots(), label(text('PARK', 4.2, { font: 'sans', weight: 700 }))], { ref: 'EK-1d s.12; EK-1e s.134', note: `${FREE} Sembol: çerçevesiz "PARK".` });
acikYesil.area('cocuk-bahcesi-ve-oyun-alani', 'Çocuk bahçesi ve oyun alanı', [solid(YESIL), freeDots(0.4, 11), picto('tahterevalli', { size: 9, caption: 'ÇB' })], { ref: 'EK-1d s.12; EK-1e s.135', note: FREE });
acikYesil.area('pasif-yesil-alan', 'Pasif yeşil alan', [solid(PASIF), freeDots(0.4, 13)], { ref: 'EK-1d s.12; EK-1e s.135', note: `${FREE} Sembol yok.` });
acikYesil.area('rekreasyon-alani', 'Rekreasyon alanı', [solid(YESIL), freeDots(0.4, 17), picto('rekreasyon', { size: 9, strokeWidth: 0.5 })], { ref: 'EK-1d s.12; EK-1e s.137', note: FREE });
acikYesil.area('fuar-panayir-ve-festival-gosteri-alani', 'Fuar, panayır ve festival/gösteri alanı', [solid(YESIL), freeDots(0.4, 19), picto('fuar', { size: 9, strokeWidth: 0.5 })], { ref: 'EK-1d s.12; EK-1e s.138', note: FREE });
acikYesil.area('mesire-yeri', 'Mesire yeri', [solid(YESIL), freeDots(0.4, 23), picto('piknik-masasi', { size: 9, strokeWidth: 0.5 })], { ref: 'EK-1d s.12; EK-1e s.139', note: FREE });
acikYesil.area('hayvanat-bahcesi', 'Hayvanat bahçesi', [solid(YESIL), gridDots(10, 1.2), picto('hayvanat-bahcesi', { size: 9, strokeWidth: 0.5 })], {
  ref: 'EK-1d s.12; EK-1e s.139',
  note: '10 mm karolaj merkezlerinde 1.2 mm noktalama (EK-1d örneği daha sık; metin esas). Sembol lejanttaki gibi piknik masası ve konifer (hayvan değil; iki ekte de böyle).',
});
acikYesil.area('hipodrom', 'Hipodrom', [solid(YESIL), gridDots(10, 1.2), picto('at', { size: 9 })], { ref: 'EK-1d s.12; EK-1e s.140', note: '10 mm karolaj merkezlerinde 1.2 mm noktalama (EK-1d örneği daha sık; metin esas).' });

/** Meydan: four corner brackets round a small heavy square (EK-1d, scaled to the 10 mm frame). */
const bracket = (sx: number, sy: number) => [seg(sx * 3.4, sy * 3.4, sx * 3.4 - sx * 1.7, sy * 3.4, 0.3), seg(sx * 3.4, sy * 3.4, sx * 3.4, sy * 3.4 - sy * 1.7, 0.3)];
acikYesil.area(
  'meydan',
  'Meydan',
  [
    solid(rgb(255, 229, 207)),
    label([
      shape('square', FRAME, { fill: WHITE, stroke: BLACK, strokeWidth: 0.5 }),
      ...bracket(-1, 1),
      ...bracket(1, 1),
      ...bracket(-1, -1),
      ...bracket(1, -1),
      shape('rectangle', 3.2, { height: 3.4, stroke: BLACK, strokeWidth: 0.55 }),
    ]),
  ],
  { ref: 'EK-1d s.12; EK-1e s.140', note: 'Düz dolgu 255/229/207, tarama yok. Sembol: dört köşe çizgisi ortasında kalın küçük kare (şekillerden kuruldu, ölçüler EK-1d\'den).' },
);

/** Bakı ve seyir terası: a small eye circle, two rays to the right and an arc across them near the eye (EK-1d). */
function viewpoint() {
  const eye: [number, number] = [-3.6, 0.2];
  // The arc: radius 3.9 about the eye, 70° wide, facing +2°. An arc shape opens on its top, so it is turned
  // by −88°; a turned marker's offset is in its own frame.
  const t = (-88 * Math.PI) / 180;
  const r3 = (v: number) => Math.round(v * 1000) / 1000;
  const off: [number, number] = [r3(eye[0] * Math.cos(t) + eye[1] * Math.sin(t)), r3(-eye[0] * Math.sin(t) + eye[1] * Math.cos(t))];
  return [
    shape('square', FRAME, { fill: WHITE, stroke: BLACK, strokeWidth: FRAME_LINE }),
    circle(1.2, { stroke: BLACK, strokeWidth: 0.25, offset: eye }),
    seg(eye[0] + 0.5, eye[1] + 0.35, 4.8, 4.2, 0.25),
    seg(eye[0] + 0.5, eye[1] - 0.35, 4.8, -3.8, 0.25),
    { ...shape('arc', 7.8, { stroke: BLACK, strokeWidth: 0.25, rotation: -88, offset: off }), sweep: 70 },
  ];
}
acikYesil.area('baki-ve-seyir-terasi', 'Bakı ve seyir terası', [solid(YESIL), freeDots(0.4, 29), label(viewpoint())], {
  ref: 'EK-1d s.12; EK-1e s.141',
  note: `${FREE} Sembol: bakış noktası (küçük daire), iki ışın ve aralarında yay (şekillerden kuruldu; "manzara" piktogramından farklı).`,
});

/** "10 mm karolaj merkezlerinde 3 mm kenarlı içi boş eşkenar üçgen", 0.3 mm. */
const forest = () => pattern(shape('triangle', 3, { stroke: BLACK, strokeWidth: 0.3 }), 10, 10);

acikYesil.area('kent-ormani', 'Kent ormanı', [solid(PASIF), forest(), picto('kent-ormani', { size: 9 })], { ref: 'EK-1d s.13; EK-1e s.141', note: '0.3 mm, 10 mm karolaj merkezlerinde 3 mm kenarlı içi boş eşkenar üçgen.' });
acikYesil.area('arboretum-botanik-parki', 'Arboretum - botanik parkı', [solid(YESIL), freeDots(0.4, 31), picto('arboretum', { size: 9 })], { ref: 'EK-1d s.13; EK-1e s.142', note: FREE });
acikYesil.area('agaclandirilacak-alan', 'Ağaçlandırılacak alan', [solid(PASIF), forest(), picto('iki-konifer', { size: 9 })], { ref: 'EK-1d s.13; EK-1e s.142', note: '0.3 mm, 10 mm karolaj merkezlerinde 3 mm kenarlı içi boş eşkenar üçgen.' });
acikYesil.area(
  'mezarlik-alani',
  'Mezarlık alanı',
  [
    solid(PASIF),
    freeDots(0.4, 37),
    // An open-topped trapezoid (EK-1d, scaled to the 10 mm frame).
    label([shape('square', FRAME, { fill: WHITE, stroke: BLACK, strokeWidth: FRAME_LINE }), seg(-4.4, 2.1, -2.5, -1.6, 0.45), seg(-2.5, -1.6, 2.5, -1.6, 0.45), seg(2.5, -1.6, 4.4, 2.1, 0.45)]),
  ],
  { ref: 'EK-1d s.13; EK-1e s.143', note: `${FREE} Sembol: üstü açık yamuk (şekillerden kuruldu). EK-1d çerçeveyi gri çiziyor; EK-1e siyah der.` },
);
acikYesil.area('korunacak-bahce', 'Korunacak bahçe', [solid(PASIF), freeDots(0.4, 41), code('KB', 4.9, { weight: 900, strokeWidth: 0.5 })], { ref: 'EK-1d s.13; EK-1e s.143', note: FREE });
acikYesil.area('rekreaktif-alan', 'Rekreaktif alan', [solid(rgb(36, 186, 34)), freeDots(0.4, 43), code('RA', 6)], {
  ref: 'EK-1d s.13; EK-1e s.108',
  note: `${FREE} Renk iki ekte de 36/186/34 (öteki yeşillerden farklı; olduğu gibi alındı). AÇIKLAMA 13: Kıyı Kanununun Uygulanmasına Dair Yönetmelikte tanımlanan kullanımı kapsar.`,
});
