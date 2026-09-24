import { BLACK, circle, hatch, pattern, sheet } from '../dsl';

/** MSP (EK-1e bölüm I, s.10) › Afetler açısından riskli alanlar: yalnız tarama, alan rengi yok. */

export const afet = sheet('msp', ['Afetler açısından riskli alanlar'], 'afet', 60);

afet.area('dogal-afetler-acisindan-riskli-alanlar', 'Doğal afetler açısından riskli alanlar', [pattern(circle(0.5, { fill: BLACK }), 5, 5)], {
  ref: 'EK-1e s.10',
  note: '0,5 mm, 5×5 mm ara ile karolaj merkezlerinde noktalama (nokta çapı çizgi kalınlığı).',
});
afet.area('yerlesme-acisindan-riskli-alanlar', 'Yerleşme açısından riskli alanlar', [hatch(90, 2, 0.2)], {
  ref: 'EK-1e s.10',
  note: '0,2 mm, 2 mm aralıklı tarama. Açı yazılı değil; çizimde dikey.',
});
