import { hatch, rgb, sheet, solid } from '../dsl';

/**
 * NİP (EK-1ç) › Konut alanları: brüt nüfus yoğunluğuna göre mevcut (dikey
 * tarama) ve gelişme (yatay tarama) konut alanları. Yoğunluk arttıkça renk
 * koyulaşır ve tarama sıklaşır: 0,2 mm çizgiler 2, 3, 4, 5, 6 mm aralıkla
 * (EK-1e s.49-53). Brüt yoğunluğun tanımı AÇIKLAMA 1.
 */

export const konut = sheet('nip', ['Konut alanları'], 'konut', 20);

export const mevcut = sheet('nip', ['Konut alanları', 'Mevcut konut alanı (brüt yoğunluğuna göre)'], 'mevcut-konut', 21);

const TIERS = [
  { id: 'cok-yuksek', name: 'çok yüksek' },
  { id: 'yuksek', name: 'yüksek' },
  { id: 'orta', name: 'orta' },
  { id: 'dusuk', name: 'düşük' },
  { id: 'seyrek', name: 'seyrek' },
] as const;

const MEVCUT = [
  { rgb: [140, 84, 26], range: '601 kişi/ha üstü', ref: 'EK-1ç s.2; EK-1e s.49' },
  { rgb: [181, 127, 0], range: '301-600 kişi/ha', ref: 'EK-1ç s.2; EK-1e s.49' },
  { rgb: [227, 186, 69], range: '151-300 kişi/ha', ref: 'EK-1ç s.2; EK-1e s.50' },
  { rgb: [255, 211, 127], range: '51-150 kişi/ha', ref: 'EK-1ç s.3; EK-1e s.50' },
  { rgb: [255, 223, 179], range: '50 kişi/ha altında', ref: 'EK-1ç s.3; EK-1e s.51', note: 'EK-1ç renk kodunu 225/223/179 yazıyor; hem EK-1ç renk kutusu hem EK-1e 255/223/179. EK-1ç\'deki 225 yazım hatası sayıldı.' },
] as const;

MEVCUT.forEach((m, i) => {
  const t = TIERS[i];
  const spacing = i + 2;
  mevcut.area(`mevcut-konut-${t.id}`, `Mevcut konut alanı, ${t.name} (${m.range})`, [solid(rgb(m.rgb[0], m.rgb[1], m.rgb[2])), hatch(90, spacing, 0.2)], {
    ref: m.ref,
    note: `0,2 mm, ${spacing} mm aralıklı dikey tarama.${'note' in m ? ' ' + m.note : ''}`,
  });
});

export const gelisme = sheet('nip', ['Konut alanları', 'Gelişme konut alanı (brüt yoğunluğuna göre)'], 'gelisme-konut', 22);

const GELISME = [
  { rgb: [242, 210, 0], range: '401 kişi/ha üstü', ref: 'EK-1ç s.3; EK-1e s.51' },
  { rgb: [255, 238, 69], range: '251-400 kişi/ha', ref: 'EK-1ç s.3; EK-1e s.52', note: 'Renk kodu iki ekte de 255/238/69 yazılı; EK-1ç renk kutusu 255/230/69 boyanmış. Yazılı kod esas alındı.' },
  { rgb: [255, 250, 112], range: '121-250 kişi/ha', ref: 'EK-1ç s.3; EK-1e s.52' },
  { rgb: [255, 254, 168], range: '51-120 kişi/ha', ref: 'EK-1ç s.3; EK-1e s.53' },
  { rgb: [255, 255, 209], range: '50 kişi/ha altında', ref: 'EK-1ç s.3; EK-1e s.53' },
] as const;

GELISME.forEach((g, i) => {
  const t = TIERS[i];
  const spacing = i + 2;
  gelisme.area(`gelisme-konut-${t.id}`, `Gelişme konut alanı, ${t.name} (${g.range})`, [solid(rgb(g.rgb[0], g.rgb[1], g.rgb[2])), hatch(0, spacing, 0.2)], {
    ref: g.ref,
    note: `0,2 mm, ${spacing} mm aralıklı yatay tarama.${'note' in g ? ' ' + g.note : ''}`,
  });
});
