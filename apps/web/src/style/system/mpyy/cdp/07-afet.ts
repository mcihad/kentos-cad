import { groupedHatch, sheet } from '../dsl';
import { box } from './common';

/**
 * ÇDP (EK-1c) › Afet tehlikeli alanlar (AÇIKLAMA 3: deprem, jeolojik
 * sakıncalı alan, çığ, sel-taşkın gibi alanlar sembol ya da tarama ile
 * gösterilir; EK-1e s.144).
 */

export const afet = sheet('cdp', ['Afet tehlikeli alanlar'], 'afet', 70);

afet.area('afetler-acisindan-riskli-alan', 'Afetler açısından riskli alan', [groupedHatch(60, 5, 3, 1, 0.2), box('RİSK', { heavy: true })], {
  ref: 'EK-1c s.3 (AÇIKLAMA 3); EK-1e s.144',
  note: 'Alan şeffaf. 0,2 mm, 60 derecelik 1 mm aralıklı üçlü çizgi, üçlüler arası 5 mm.',
});
