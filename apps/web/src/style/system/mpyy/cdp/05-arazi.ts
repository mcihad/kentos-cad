import { BLACK, circle, dashMarkers, hatch, pattern, rgb, sheet, solid } from '../dsl';
import { pairs } from './common';

/**
 * ÇDP (EK-1c) › Bugünkü arazi kullanımı devam ettirilerek korunacak
 * alanlar: tarım, organize tarım ve hayvancılık, sulama alanı (EK-1e s.98-99).
 */

export const arazi = sheet('cdp', ['Bugünkü arazi kullanımı devam ettirilerek korunacak alanlar'], 'arazi', 50);

arazi.area('tarim-alani', 'Tarım alanı', [solid(rgb(233, 250, 190)), pattern(circle(1, { stroke: BLACK, strokeWidth: 0.2 }), 5, 5)], {
  ref: 'EK-1c s.2; EK-1e s.98',
  note: '0,2 mm, 5 mm karelaj merkezlerinde 1 mm çapında içi boş daireler.',
});
arazi.area('organize-tarim-ve-hayvancilik-alani', 'Organize tarım ve hayvancılık alanı', [solid(rgb(233, 250, 190)), pairs(0, 3), pairs(90, 3)], {
  ref: 'EK-1c s.3; EK-1e s.98',
  note: '0,2 mm, 1 mm tarama çiftleri ile karelaj (yatay ve dikey), çiftler arası 3 mm.',
});
arazi.area(
  'sulama-alani',
  'Sulama alanı',
  [hatch(45, 3, 0.3, BLACK, { dash: [3, 2] }), dashMarkers(BLACK, 0.3, 5, 5, circle(3, { stroke: BLACK, strokeWidth: 0.3 }))],
  {
    ref: 'EK-1c s.3; EK-1e s.99',
    note: 'Alan şeffaf. Tarama: 0,3 mm, 3 mm aralıklı 45 derece kesik çizgi; çizgi ve boşluk boyu yazılı değil, 3 mm çizgi 2 mm boşluk alındı. Sınır: 0,3 mm; 5 mm düz çizgi, 1 mm boşluk, 3 mm çaplı boş daire, 1 mm boşluk.',
  },
);
