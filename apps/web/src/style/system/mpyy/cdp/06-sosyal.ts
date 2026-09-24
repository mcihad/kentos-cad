import { BLACK, framed, hatch, pattern, rgb, shape, sheet, solid, svg } from '../dsl';
import { pic } from '../pictograms';
import { S, box, freeDots, gridDots, picture, stack } from './common';

/**
 * ÇDP (EK-1c) › Sosyal altyapı alanları (AÇIKLAMA 2): açık ve yeşil
 * alanlar, eğitim alanları (EK-1e s.61, 123, 133, 138, 142).
 */

export const sosyal = sheet('cdp', ['Sosyal altyapı alanları'], 'sosyal', 60);
export const yesil = sheet('cdp', ['Sosyal altyapı alanları', 'Açık ve yeşil alanlar'], 'yesil', 61);

yesil.area(
  'agaclandirilacak-alan',
  'Ağaçlandırılacak alan',
  [solid(rgb(99, 186, 82)), pattern(shape('triangle', 3, { stroke: BLACK, strokeWidth: 0.3 }), 10, 10), picture('dort-konifer')],
  { ref: 'EK-1c s.3; EK-1e s.142', note: '0,3 mm, 10 mm karolaj merkezlerinde 3 mm kenarlı içi boş eşkenar üçgen. EK-1c örneği üçgenleri gri ve sık basmış; ölçüler EK-1e\'den.' },
);
yesil.area('fuar-panayir-ve-festival-gosteri-alani', 'Fuar, panayır ve festival/gösteri alanı', [solid(rgb(36, 156, 34)), freeDots(0.3, 4), picture('fuar', undefined, { heavy: true })], {
  ref: 'EK-1c s.3; EK-1e s.138',
  note: '0,3 mm serbest noktalama; sıklık yazılı değil, cm² başına yaklaşık 20 nokta.',
});
yesil.area('kentsel-ve-bolgesel-yesil-ve-spor-alani', 'Kentsel ve bölgesel yeşil ve spor alanı', [solid(rgb(36, 156, 34)), freeDots(0.2, 8)], {
  ref: 'EK-1c s.3; EK-1e s.133',
  note: '0,2 mm serbest noktalama; sıklık yazılı değil, cm² başına yaklaşık 20 nokta.',
});
yesil.area('kentsel-ve-bolgesel-sosyal-altyapi-alani', 'Kentsel ve bölgesel sosyal altyapı alanı', [solid(rgb(115, 212, 255)), gridDots(5, 0.4), box('D', { max: 3 })], {
  ref: 'EK-1c s.3 (AÇIKLAMA 2); EK-1e s.123',
  note: '0,4 mm, 5 mm karolaj merkezlerinde noktalama. EK-1c bu satırı "Açık ve yeşil alanlar" başlığı altına koymuş; EK-1e\'de sosyal ve kültürel tesis alanlarında.',
});
yesil.area(
  'mesire-alani',
  'Mesire alanı',
  [solid(rgb(170, 255, 0)), hatch(90, 5.5, 2, rgb(255, 170, 0)), stack([...framed('', { size: S, strokeWidth: 0.25, background: rgb(255, 115, 0) }), svg(pic('piknik-masasi'), S * 0.88, { fill: BLACK })])],
  {
    ref: 'EK-1c s.3; EK-1e s.138',
    note: '2 mm kalınlığında dikey turuncu tarama, renk 255/170/0; aralık yazılı değil, EK-1c çiziminden 5,5 mm (çizim bu satırda 1:1, çubuklar 2,0 mm ölçülüyor). Sembol turuncu karede piknik masası; EK-1e sembol rengini 255/115/0 yazıyor, çizimi 255/170/0\'a yakın: yazılı kod esas alındı.',
  },
);

export const egitim = sheet('cdp', ['Sosyal altyapı alanları', 'Eğitim alanları'], 'egitim', 62);

egitim.area('universite-alani', 'Üniversite alanı', [solid(rgb(122, 142, 245)), gridDots(3.5, 0.2), stack(framed('Ü', { size: S, strokeWidth: 0.25, background: rgb(52, 128, 255), textSize: 3.2, font: 'sans', weight: 900 }))], {
  ref: 'EK-1c s.3; EK-1e s.61',
  note: '0,2 mm, 3,5 mm karolaj merkezlerinde noktalama. Sembol mavi (52/128/255, EK-1e\'de yazılı) karede kalın siyah "Ü".',
});
