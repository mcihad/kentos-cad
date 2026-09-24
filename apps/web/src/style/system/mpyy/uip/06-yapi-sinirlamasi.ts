import { BLACK, along, circle, double, hatch, label, rgb, sheet, solid, stroke, svg, ticks } from '../dsl';
import { pic } from '../pictograms';
import { BELT_FILL, RED, beltEdge, crossBoxMarks, picto } from './common';

/**
 * UİP (EK-1d s.8–9) › Korunacak alanlar › Yapı sınırlaması getirilerek
 * korunacak alanlar: koruma kuşakları (EK-1e s.109–116, 175). The belts
 * share one boundary (0.3 mm red, 7 mm dash, 3 mm çarpı, the code written
 * on it) and one frame (double square with a cross, the code under it).
 */

export const yapiSinirlamasi = sheet('uip', ['Korunacak alanlar', 'Yapı sınırlaması getirilerek korunacak alanlar'], 'yapi-sinirlamasi', 20);

const BELT_NOTE =
  'Alan 245/122/122. Sınır: 0.3 mm kırmızı (255/0/0), 7 mm düz çizgi aralıklı 3 mm uzunluğunda çarpı; ara (4.4 mm) ≈1:1 çizimden. "Uygun aralıklarla sınır üzerine … yazılacak": kod her dördüncü çizgi parçasının üstüne (alanın içine) 2.5 mm siyah yazılır; aralık ve boyut verilmemiş. Sembol: çift kare, iç karenin köşegenleri, altında kod.';

const belt = (id: string, name: string, codeText: string, ref: string) =>
  yapiSinirlamasi.area(id, name, [solid(BELT_FILL), beltEdge(codeText), label(crossBoxMarks(codeText))], { ref, note: BELT_NOTE });

yapiSinirlamasi.area(
  'nukleer-enerji-uretim-alani-koruma-kusagi',
  'Nükleer enerji üretim alanı koruma kuşağı',
  [solid(BELT_FILL), along(svg(pic('radyasyon'), 6, { fill: RED }), 8), picto('radyasyon', { size: 8 })],
  {
    ref: 'EK-1d s.8; EK-1e s.109',
    note: 'Alan 245/122/122. Sınır: çizgi yok; 6 mm çaplı daireden üretilmiş kırmızı radyasyon işaretleri, aralarında 2 mm (8 mm adım). Sembol: siyah radyasyon işareti çerçevede.',
  },
);
yapiSinirlamasi.line('mania-plani', 'Mania planı', [double(BLACK, 0.4, 1.5)], {
  ref: 'EK-1d s.8; EK-1e s.109 (Mania planı alanı)',
  note: 'Çizgi: "Mania planı çizilir", 0.4 mm siyah çift çizgi; aralık verilmemiş, çizimden 1.5 mm. EK-1d çizgileri koyu gri gösteriyor.',
});
belt('havaalani-havalimani-koruma-kusagi', 'Havaalanı/havalimanı koruma kuşağı', 'HKK', 'EK-1d s.8; EK-1e s.110');
belt('karayollari-yol-kenari-koruma-kusagi', 'Karayolları yol kenarı koruma kuşağı', 'YKK', 'EK-1d s.8; EK-1e s.111');
belt('boru-hatti-koruma-kusagi', 'Boru hattı koruma kuşağı', 'BHK', 'EK-1d s.8; EK-1e s.111');
belt('su-kanallari-koruma-kusagi', 'Su kanalları koruma kuşağı', 'SKK', 'EK-1d s.8; EK-1e s.112');
belt('icme-suyu-ana-iletim-hatti-koruma-kusagi', 'İçme suyu ana iletim hattı koruma kuşağı', 'İSK', 'EK-1d s.8; EK-1e s.112');
belt('yer-alti-su-kaynaklari-koruma-kusagi', 'Yer altı su kaynakları koruma kuşağı', 'YSK', 'EK-1d s.8; EK-1e s.113');
belt('demiryollari-koruma-kusagi', 'Demiryolları koruma kuşağı', 'DKK', 'EK-1d s.8; EK-1e s.114');
yapiSinirlamasi.line(
  'jeotermal-koruma-kusagi',
  'Jeotermal koruma kuşağı',
  [
    // 2 mm circle with a centre dot, 1 mm blank, 7 mm double line (1 mm apart), 1 mm blank: 11 mm.
    double(RED, 0.3, 1, { dash: [7, 4] }),
    along([circle(2, { stroke: RED, strokeWidth: 0.3 }), circle(0.5, { fill: RED })], 11, { offsetAlong: 9 }),
  ],
  {
    ref: 'EK-1d s.8; EK-1e s.114',
    note: 'Şeffaf alan sınırı: 0.3 mm kırmızı, 2 mm çapında içi noktalı daire, 1 mm boşluk, çizgileri 1 mm aralı 7 mm çift çizgi. Merkez noktanın çapı verilmemiş (0.5 mm).',
  },
);
belt('yanici-parlayici-patlayici-maddeler-koruma-kusagi', 'Yanıcı parlayıcı ve patlayıcı maddeler koruma kuşağı (güvenlik mesafesi)', 'YPK', 'EK-1d s.9; EK-1e s.115');
belt('saglik-koruma-bandi', 'Sağlık koruma bandı', 'SKB', 'EK-1d s.9; EK-1e s.115');
belt('enerji-nakil-hatti-koruma-kusagi', 'Enerji nakil hattı koruma kuşağı', 'ENH', 'EK-1d s.9; EK-1e s.116');
yapiSinirlamasi.line('havaalani-hava-koridoru', 'Havaalanı hava koridoru', [stroke(BLACK, 0.4), ticks(BLACK, 0.4, 3, 7, 1)], {
  ref: 'EK-1d s.9; EK-1e s.110',
  note: 'Çizgi: 0.4 mm siyah, 7 mm aralıklı 3 mm dik çizgiler, koridorun içine doğru tek yanda: çizginin soluna (alan kenarında içe). AÇIKLAMA 15: mania planındaki yaklaşma-tırmanma yüzeyleri.',
});
yapiSinirlamasi.area('tunel-etki-alani', 'Tünel etki alanı', [hatch(45, 15, 0.2, rgb(160, 160, 160)), beltEdge('TEA', BELT_FILL, 5)], {
  ref: 'EK-1d s.9; EK-1e s.175',
  note: 'Şeffaf; tarama 0.2 mm, 15 mm aralıklı 45° gri (160/160/160) çizgi. Sınır 0.3 mm, 7 mm düz çizgi aralıklı 3 mm çarpı, rengi EK-1e\'nin sınır RGB değeri 245/122/122 (çizimler daha koyu kırmızı gösteriyor; EK-1d aynı değeri tek renk sütununda veriyor). "Uygun aralıklarla sınır üzerine TEA yazılır": her dördüncü parçaya. Çarpı çevresindeki ara verilmemiş (5 mm alındı).',
});
