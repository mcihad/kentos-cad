import { BLACK, along, circle, double, rgb, shape, solid, stroke, text } from '../dsl';
import { RED, crossBoxMarks, sections, stack } from './common';

/**
 * EK-1a (s.6) › Korunacak alanlar › Yapı sınırlaması getirilerek
 * korunacak alanlar, every plan level (EK-1e s.102–113).
 *
 * Sulak alan boundaries are 5 mm sine waves with dots between them. EK-1e
 * gives no pen for them: 0.3 mm, as EK-1a draws them, and a wave height of
 * ±0.22 mm (EK-1a/EK-1e drawings). The waves start at the path's start
 * (`wave.offsetAlong`), so the dots, placed with the same period, stay in
 * the gaps between them.
 */

const WATER = rgb(115, 223, 235);

/** İçme ve kullanma suyu koruma alanları: 7 mm line, 2 mm, `n` tangent 2 mm circles with a centre dot, 2 mm. */
function wellLine(n: number) {
  const period = 7 + 2 + 2 * n + 2;
  return [
    stroke(RED, 0.3, { dash: [7, period - 7] }),
    along([circle(2, { stroke: RED, strokeWidth: 0.3 }), circle(0.6, { fill: RED })], period, { offsetAlong: 9 + n, group: n > 1 ? { count: n, spacing: 2 } : undefined }),
  ];
}

const wellNote = (n: number) =>
  `Şeffaf. 0.3 mm kırmızı: 2 mm çapında içi noktalı ${n > 1 ? `teğet ${n} adet ` : ''}daire, 2 mm boşluk, 7 mm düz çizgi. Daire içindeki nokta 0.6 mm (çizimden). Daire sayısı 4/3/2/1 mutlak/kısa/orta/uzun mesafeyi gösterir.`;

const dot = () => circle(1, { fill: RED });

/**
 * A sulak alan boundary: `waves` 5 mm waves back to back, then a gap of
 * `gap` holding `dots` 1 mm dots `dotStep` apart (centre to centre).
 */
function wetLine(waves: number, gap: number, dots: number, dotStep: number) {
  const run = 5 * waves;
  const period = run + gap;
  return [
    ...Array.from({ length: waves }, (_, k) => stroke(RED, 0.3, { wave: { shape: 'sine', length: 5, amplitude: 0.22, spacing: period, connect: false, offsetAlong: 5 * k } })),
    along(dot(), period, { offsetAlong: run + gap / 2, rotate: true, group: dots > 1 ? { count: dots, spacing: dotStep } : undefined }),
  ];
}

/** Sınırı and bölgesi: two dots 1 mm apart with 2 mm clear on each side (EK-1a/EK-1e drawings; see note). */
const wetEdge = () => wetLine(1, 2 + 3 + 2, 2, 2);
const wetEdgeNote =
  '"5 mm uzunluğunda dalgalı çizgi, çizgiler arası boşluk 2 mm, boşluklarda 1 mm çapında 2 nokta": iki 1 mm nokta 2 mm boşluğa ancak birbirine ve dalgalara değerek sığar; iki çizim de noktaların iki yanında boşluk bırakır. 2 mm dalga ile nokta çifti arasındaki boşluk, noktalar arası 1 mm (çizimden) alındı. Çizgi kalınlığı verilmemiş: 0.3 mm.';

export const YAPI_SINIRLAMASI = sections(['Korunacak alanlar', 'Yapı sınırlaması getirilerek korunacak alanlar'], 'yapi-sinirlamasi', 1060, (s, lv) => {
  s.line('icme-ve-kullanma-suyu-mutlak-koruma-alani', 'İçme ve kullanma suyu mutlak koruma alanı', [wellLine(4)], { ref: 'EK-1a s.6; EK-1e s.102', note: wellNote(4) });
  s.line('icme-ve-kullanma-suyu-kisa-mesafeli-koruma-alani', 'İçme ve kullanma suyu kısa mesafeli koruma alanı', [wellLine(3)], { ref: 'EK-1a s.6; EK-1e s.102', note: wellNote(3) });
  s.line('icme-ve-kullanma-suyu-orta-mesafeli-koruma-alani', 'İçme ve kullanma suyu orta mesafeli koruma alanı', [wellLine(2)], { ref: 'EK-1a s.6; EK-1e s.103', note: wellNote(2) });
  s.line('icme-ve-kullanma-suyu-uzun-mesafeli-koruma-alani', 'İçme ve kullanma suyu uzun mesafeli koruma alanı', [wellLine(1)], { ref: 'EK-1a s.6; EK-1e s.103', note: wellNote(1) });

  s.line('sulak-alan-siniri', 'Sulak alan sınırı', [wetEdge()], { ref: 'EK-1a s.6; EK-1e s.104', note: `Şeffaf. Kırmızı: ${wetEdgeNote}` });
  s.area('sulak-alan-bolgesi', 'Sulak alan bölgesi', [solid(WATER), wetEdge()], {
    ref: 'EK-1a s.6; EK-1e s.104',
    note: `Alan 115/223/235. Sınır sulak alan sınırı ile aynı: ${wetEdgeNote}`,
  });
  s.line('sulak-alan-tampon-bolgesi', 'Sulak alan tampon bölgesi', [wetLine(2, 2, 1, 0)], {
    ref: 'EK-1a s.6; EK-1e s.105',
    note: 'Şeffaf. Kırmızı: 5 mm uzunluğunda arka arkaya iki dalga, 2 mm boşluk ortasında 1 mm çapında nokta. Kalınlık verilmemiş: 0.3 mm.',
  });
  s.line('sulak-alan-ekolojik-etkilenme-bolgesi', 'Sulak alan ekolojik etkilenme bölgesi', [wetLine(1, 5, 3, 1.3)], {
    ref: 'EK-1a s.6; EK-1e s.105',
    note: 'Şeffaf. Kırmızı: 5 mm dalga, 5 mm boşlukta 1 mm çapında 3 nokta (merkezden merkeze 1.3 mm, çizimden). EK-1a ilk noktayı biraz büyük çizer; eşit alındı. Kalınlık verilmemiş: 0.3 mm.',
  });
  const threeDots = lv === 'cdp';
  s.line('sulak-alan-mutlak-koruma-bolgesi', 'Sulak alan mutlak koruma bölgesi', [threeDots ? wetLine(1, 4.6, 3, 1.3) : wetLine(1, 2, 1, 0)], {
    ref: 'EK-1a s.6; EK-1e s.106',
    note: threeDots
      ? 'Şeffaf. Kırmızı: 5 mm dalga; ÇDP metni "2 mm boşluklarda 1 mm çapında 3 nokta" der, çizim tek nokta gösterir. Üç nokta 2 mm\'ye sığmadığından boşluk 4.6 mm alındı (noktalar 1.3 mm arayla, iki yanda 0.5 mm). Kalınlık verilmemiş: 0.3 mm.'
      : 'Şeffaf. Kırmızı: 5 mm dalga, 2 mm boşluk ortasında 1 mm çapında nokta. UİP ve NİP\'te sulak alan özel hüküm bölgesi ile aynı gösterim. Kalınlık verilmemiş: 0.3 mm.',
  });
  s.line('sulak-alan-ozel-hukum-bolgesi', 'Sulak alan özel hüküm bölgesi', [wetLine(1, 2, 1, 0)], {
    ref: 'EK-1a s.6; EK-1e s.106',
    note: 'Şeffaf. Kırmızı: 5 mm dalga, 2 mm boşluk ortasında 1 mm çapında nokta. EK-1e bunu sulak alan mutlak koruma bölgesi (UİP, NİP) ile aynı verir; ayırt etmek öznitelikle olur. Kalınlık verilmemiş: 0.3 mm.',
  });

  // Yer altı su kaynakları: 7 mm red line, a 3 mm × between the pieces (the 4.4 mm gap is measured), YSK on the line.
  s.area(
    'yer-alti-su-kaynaklari-koruma-alanlari',
    'Yer altı su kaynakları koruma alanları',
    [
      solid(rgb(245, 122, 122)),
      stroke(RED, 0.3, { dash: [7, 4.4] }),
      along(shape('x', 3, { stroke: RED, strokeWidth: 0.3 }), 11.4, { offsetAlong: 9.2 }),
      along(text('YSK', 2.5, { font: 'sans', weight: 700, color: RED }), 11.4 * 5, { offsetAlong: 3.5, offset: 1.9 }),
      stack(crossBoxMarks(lv, 'YSK')),
    ],
    {
      ref: 'EK-1a s.6; EK-1e s.113',
      note: 'Alan 245/122/122. Sınır 0.3 mm kırmızı: 7 mm düz çizgi aralarında 3 mm uzunluğunda çarpı (ara 4.4 mm çizimden); "uygun aralıklarla" sınır üzerine YSK: her beşinci çizgi parçasının içine 2.5 mm. Sembol: çift kare, iç karenin köşegenleri, altında YSK. AÇIKLAMA 10: koruma kuşaklarının derece ve statüleri de belirtilir.',
    },
  );
  s.line('mania-plani-alani', 'Mania planı alanı', [double(BLACK, 0.4, 1.5)], {
    ref: 'EK-1a s.6; EK-1e s.109',
    note: 'Çizgi: 0.4 mm çift düz çizgi, "Mania Planı çizilir". İki çizgi arası verilmemiş: 1.5 mm (eksenden eksene) çizimden.',
  });
});
