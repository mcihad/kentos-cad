import { BLACK, rgb, shape, sheet } from '../dsl';

/**
 * MSP (EK-1e bölüm I, s.6) › Ana altyapı: enerji üretim alanları. Dört renkte
 * eşkenar üçgen; hangi rengin hangi enerji türü olduğu yazılı değil, her
 * renk ayrı işaret.
 */

export const altyapi = sheet('msp', ['Ana altyapı'], 'altyapi', 40);

const VARIANTS = [
  { id: 'yesil', name: 'yeşil', rgb: [0, 160, 0] },
  { id: 'sari', name: 'sarı', rgb: [255, 255, 0] },
  { id: 'mavi', name: 'mavi', rgb: [60, 170, 250] },
  { id: 'mor', name: 'mor', rgb: [130, 40, 255] },
] as const;
for (const v of VARIANTS) {
  altyapi.point(`enerji-uretim-alanlari-${v.id}`, `Enerji üretim alanları (${v.name}, ${v.rgb.join('/')})`, [shape('triangle', 7.5, { fill: rgb(v.rgb[0], v.rgb[1], v.rgb[2]), stroke: BLACK, strokeWidth: 0.3 })], {
    ref: 'EK-1e s.6',
    note: 'Tepesi yukarı eşkenar üçgen, ince siyah dış çizgi; kenar (7,5 mm) çizimden. MSP satırı dört renk verir (0/160/0, 255/255/0, 60/170/250, 130/40/255); renklerin enerji türleriyle eşleşmesi yazılı değil.',
  });
}
