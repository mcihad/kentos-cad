import { BLACK, circle, hatch, pattern, rgb, sheet, solid } from '../dsl';
import { box, freeDots, picture } from './common';

/**
 * NİP (EK-1ç) › Bugünkü arazi kullanımı devam ettirilerek korunacak
 * alanlar: tarım alanı, makilik-fundalık alan, kumsal-plaj (EK-1e s.96-98).
 */

export const arazi = sheet('nip', ['Bugünkü arazi kullanımı devam ettirilerek korunacak alanlar'], 'arazi', 50);

arazi.area('tarim-alani', 'Tarım alanı', [solid(rgb(233, 250, 190)), pattern(circle(1, { stroke: BLACK, strokeWidth: 0.2 }), 5, 5)], {
  ref: 'EK-1ç s.5; EK-1e s.98',
  note: '0,2 mm, 5 mm karelaj merkezlerinde 1 mm çapında içi boş daireler. EK-1ç örneği daireleri gri basmış; EK-1e siyah.',
});
arazi.area('makilik-fundalik-alan', 'Makilik-fundalık alan', [solid(rgb(180, 215, 158)), hatch(0, 5, 0.3), box('MF', { weight: 900, heavy: true, max: 4.2 })], {
  ref: 'EK-1ç s.5; EK-1e s.96',
  note: '0,3 mm, 5 mm aralıklı yatay tarama.',
});
arazi.area('kumsal-plaj', 'Kumsal-plaj', [solid(rgb(255, 229, 207)), freeDots(0.3, 3), picture('plaj-semsiyesi', 'K/P')], {
  ref: 'EK-1ç s.6; EK-1e s.97',
  note: '0,3 mm serbest noktalama. Sıklık yazılı değil: EK-1ç örneğindeki gibi cm² başına yaklaşık 20 nokta.',
});
