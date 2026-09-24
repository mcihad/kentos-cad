import { BLACK, WHITE, along, crossHatch, grid, groupedHatch, hatch, label, rgb, shape, sheet, solid, stroke, text } from '../dsl';
import { code, codeMarks, gridDots, parenBelow, picto, pictoMarks } from './common';

/**
 * UİP (EK-1d s.3–5) › Konut alanları, Kentsel çalışma alanları. Fills are
 * the legend's RGB, hatches EK-1e's UİP rows; code sizes are measured from
 * EK-1d (see common.ts).
 */

export const konut = sheet('uip', ['Konut alanları'], 'konut', 30);

konut.area('yerlesik-konut-alani', 'Yerleşik konut alanı', [solid(rgb(140, 84, 26))], { ref: 'EK-1d s.3; EK-1e s.54', note: 'Düz dolgu 140/84/26.' });
konut.area('gelisme-konut-alani', 'Gelişme konut alanı', [solid(rgb(255, 250, 38))], { ref: 'EK-1d s.3; EK-1e s.54', note: 'Düz dolgu 255/250/38.' });

export const calisma = sheet('uip', ['Kentsel çalışma alanları'], 'calisma', 40);

/** "n mm ara ile karelaj": a square grid of 0.3 mm lines. */
const karelaj = (spacing: number, width = 0.3) => grid(spacing, spacing, width, BLACK);
/** "1 mm tarama çiftleri ile 45 derece karelaj": pairs of 0.2 mm lines 1 mm apart both ways, `period` apart. */
const pairs45 = (period: number) => [...groupedHatch(45, period, 2, 1, 0.2), ...groupedHatch(135, period, 2, 1, 0.2)];

calisma.area('ticaret-alani', 'Ticaret alanı', [solid(rgb(224, 0, 33)), karelaj(4), code({ expr: `'T' || varsayılan([Kademe], '')` }, 5)], {
  ref: 'EK-1d s.3; EK-1e s.58',
  note: '0.3 mm, 4 mm ara ile karelaj. AÇIKLAMA 9: kademelenme öngörülüyorsa T1 (en üst) … T3 (en alt), yoksa yalnız T. Kod "Kademe" alanından (1, 2, 3); boşsa "T".',
});
calisma.area('ticaret-konut-alani', 'Ticaret-konut alanı', [solid(rgb(227, 186, 69)), karelaj(4), code('TİCK', 3.5)], { ref: 'EK-1d s.3; EK-1e s.58', note: '0.3 mm, 4 mm ara ile karelaj.' });
calisma.area('ticaret-turizm-alani', 'Ticaret-turizm alanı', [solid(rgb(255, 115, 0)), karelaj(4), code('TİCT', 3.5)], { ref: 'EK-1d s.3; EK-1e s.59', note: '0.3 mm, 4 mm ara ile karelaj.' });
calisma.area('ticaret-turizm-konut-alani', 'Ticaret-turizm-konut alanı', [solid(rgb(255, 117, 0)), karelaj(3), code('TİCTK', 3.2, { font: 'narrow' })], {
  ref: 'EK-1d s.3; EK-1e s.59',
  note: '0.3 mm, 3 mm ara ile karelaj (öteki ticaret karışımları 4 mm). RGB 255/117/0 iki ekte de böyle yazılı (öteki turizm alanları 255/115/0).',
});
calisma.area('toptan-ticaret-alani', 'Toptan ticaret alanı', [solid(rgb(224, 0, 33)), karelaj(4), code('TT', 5)], { ref: 'EK-1d s.4; EK-1e s.60', note: '0.3 mm, 4 mm ara ile karelaj.' });
calisma.area('toplu-isyerleri', 'Toplu işyerleri (…)', [solid(rgb(224, 0, 33)), pairs45(3), label([...codeMarks('Ti', 5), parenBelow('Faaliyet')])], {
  ref: 'EK-1d s.4; EK-1e s.68',
  note: '0.2 mm, 1 mm tarama çiftleri ile 45° karelaj, çiftler arası 3 mm. Kod EK-1d\'deki gibi "Ti" (EK-1e "Tİ" yazar). AÇIKLAMA 12: planda öngörülen faaliyetin adı parantez içinde büyük harfle yazılır: "Faaliyet" alanı doluysa çerçevenin altına yazılır.',
});
calisma.area('belediye-hizmet-alani', 'Belediye hizmet alanı (…)', [solid(rgb(102, 153, 205)), gridDots(7, 1.2), label([...codeMarks('BHA', 4.1), parenBelow('Kullanım')])], {
  ref: 'EK-1d s.4; EK-1e s.61',
  note: '7×7 mm karolaj merkezlerinde 1.2 mm noktalama (EK-1d örneği daha küçük ve sık noktalar gösteriyor; metin esas). AÇIKLAMA 5: belirlenecek kullanım parantez içinde belirtilir: "Kullanım" alanı doluysa çerçevenin altına yazılır.',
});
calisma.area('idari-hizmet-alani', 'İdari hizmet alanı', [solid(rgb(102, 153, 205)), gridDots(7, 1.2), code('İHA', 4.1)], { ref: 'EK-1d s.4; EK-1e s.62', note: '7×7 mm karolaj merkezlerinde 1.2 mm noktalama. EK-1d örneği daha küçük ve sık noktalar gösteriyor; metin esas alındı.' });
calisma.area('resmi-kurum-alani', 'Resmi kurum alanı (…)', [solid(rgb(102, 153, 205)), gridDots(7, 1.2), label([...pictoMarks('resmi-kurum', { frame: 'rect', frameSize: 11.5, height: 9.5, size: 4.8 }), parenBelow('Kurum', 9.5)])], {
  ref: 'EK-1d s.4; EK-1e s.63',
  note: '7×7 mm karolaj merkezlerinde 1.2 mm noktalama (EK-1d örneği daha küçük ve sık noktalar gösteriyor; metin esas). Sembol: yarısı dolu daire, EK-1d\'deki gibi 11.5×9.5 mm yatay çerçevede (EK-1e kare çiziyor; ölçü verilmemiş). AÇIKLAMA 6: kurumun adı parantez içinde: "Kurum" alanı doluysa çerçevenin altına yazılır.',
});
calisma.area('akaryakit-ve-servis-istasyonu-alani', 'Akaryakıt ve servis istasyonu alanı', [solid(rgb(255, 56, 0)), karelaj(3), picto('akaryakit', { frame: 'rect', frameSize: 11.3, height: 9.8, size: 8.4, caption: 'A' })], {
  ref: 'EK-1d s.4; EK-1e s.64',
  note: '0.3 mm, 3 mm ara ile karelaj. Pompa piktogramı 11.3×9.8 mm çerçevede, altında "A" (ölçüler EK-1d\'den).',
});
calisma.area('sanayi-tesis-alani', 'Sanayi tesis alanı', [solid(rgb(170, 102, 205)), crossHatch(4, 0.2)], {
  ref: 'EK-1d s.4; EK-1e s.65 (Sanayi alanı)',
  note: '0.2 mm, 4 mm ara ile 45° çapraz tarama. EK-1e\'de "Sanayi alanı" adıyla.',
});

/**
 * "Dişli" (EK-1e s.66): a filled toothed disc of 7 mm outer diameter; EK-1d
 * draws 12 shallow teeth.
 */
const disli = (d = 7) => ({ ...shape('gear', d, { fill: BLACK }), teeth: 12, teethDepth: 0.12 });

calisma.area(
  'endustriyel-gelisme-bolgesi',
  'Endüstriyel gelişme bölgesi',
  [
    solid(rgb(232, 190, 255)),
    pairs45(5),
    // Boundary: 7 mm line, 2 mm blank, two 7 mm dişli 1 mm apart, 2 mm blank (26 mm).
    stroke(BLACK, 0.3, { dash: [7, 19] }),
    along(disli(7), 26, { offsetAlong: 16.5, group: { count: 2, spacing: 8 } }),
    code('EGB', 3.8, { strokeWidth: 0.5 }),
  ],
  {
    ref: 'EK-1d s.4; EK-1e s.66',
    note: 'Tarama: 0.2 mm, 1 mm tarama çiftleri ile 45° çapraz tarama, çiftler arası 5 mm. Sınır: 0.3 mm, 7 mm çizgi, 2 mm boşluk, 1 mm aralı 7 mm dış çaplı 2 dolu dişli, 2 mm boşluk (ikinci boşluk metinde yok, simetrik alındı). Dişli EK-1d\'deki gibi 12 sığ dişli dolu disk (diş derinliği yarıçapın %12\'si, çizimden).',
  },
);
calisma.area('kucuk-sanayi-alani', 'Küçük sanayi alanı', [solid(rgb(170, 102, 205)), groupedHatch(45, 7, 2, 1, 0.2), hatch(135, 7, 0.2), code('KSA', 3.4, { weight: 900 })], {
  ref: 'EK-1d s.4; EK-1e s.66',
  note: '0.2 mm, 7 mm aralı: 45° yönde 1 mm tarama çiftleri, 135° yönde tek tarama.',
});
calisma.area('depolama-alani', 'Depolama alanı', [solid(rgb(194, 158, 215)), pairs45(5), code('D', 6, { weight: 900, strokeWidth: 0.5 })], {
  ref: 'EK-1d s.4; EK-1e s.67',
  note: '0.2 mm, 1 mm tarama çiftleri ile 45° karelaj, çiftler arası 5 mm.',
});
calisma.area('imalathane-tesis-alani', 'İmalathane tesis alanı', [solid(rgb(241, 133, 233)), hatch(45, 5, 0.3), hatch(135, 10, 0.3)], {
  ref: 'EK-1d s.4; EK-1e s.24',
  note: '0.3 mm, 5 ve 10 mm ara ile 45° çapraz tarama: 45° yönü 5 mm, 135° yönü 10 mm (hangi yönün sık olduğu çizimden). AÇIKLAMA 11: konut dışı kentsel çalışma alanında çevre sağlığına tehlike oluşturmayan imalathaneler.',
});
calisma.area('lojistik-tesis-alani', 'Lojistik tesis alanı', [solid(rgb(194, 158, 215)), pairs45(5), code('LTA', 4.1)], {
  ref: 'EK-1d s.4; EK-1e s.67',
  note: '0.2 mm, 1 mm tarama çiftleri ile 45° karelaj, çiftler arası 5 mm.',
});
calisma.area('pazar-alani', 'Pazar alanı', [solid(rgb(224, 0, 33)), crossHatch(6, 0.3), code('P', 6, { weight: 900, strokeWidth: 0.5 })], { ref: 'EK-1d s.5; EK-1e s.68', note: '0.3 mm, 6 mm ara ile 45° karelaj.' });
calisma.area('tarim-ve-hayvancilik-tesis-alani', 'Tarım ve hayvancılık tesis alanı', [solid(rgb(233, 250, 190)), groupedHatch(0, 5, 2, 1, 0.2), groupedHatch(90, 5, 2, 1, 0.2)], {
  ref: 'EK-1d s.5; EK-1e s.69',
  note: '0.2 mm, 1 mm tarama çiftleri ile karelaj (yatay ve dikey), çiftler arası 5 mm.',
});
calisma.area('askeri-alan', 'Askeri alan', [solid(rgb(168, 168, 0)), groupedHatch(90, 5, 2, 1, 0.2), picto('tufekler', { size: 9 })], {
  ref: 'EK-1d s.5; EK-1e s.70',
  note: '0.2 mm, 1 mm aralıklı dikey çift çizgiler, çiftler arası 5 mm. EK-1d çizimi kalın-ince çizgi çiftleri gösteriyor; metin esas alındı.',
});
calisma.area('beton-santrali', 'Beton santrali', [solid(rgb(170, 102, 205)), crossHatch(6, 0.3), code('BS', 4.8)], { ref: 'EK-1d s.5; EK-1e s.70', note: '0.3 mm, 6 mm ara ile 45° karelaj.' });
calisma.area(
  'su-urunleri-uretim-ve-yetistirme-tesisi',
  'Su ürünleri üretim ve yetiştirme tesisi',
  [stroke(rgb(0, 92, 230), 0.5, { dash: [10, 4] }), picto('balik', { caption: 'SÜA', captionSize: 4 })],
  {
    ref: 'EK-1d s.5; EK-1e s.69',
    note: 'Sınır: 0.5 mm, 4 mm aralıklı kesikli çizgi, 0/92/230; çizgi boyu verilmemiş, çizimden 10 mm. EK-1e alanı şeffaf, 0/92/230\'u sınır rengi verir (EK-1d aynı rengi renk kodu sütununda gösteriyor); dolgu yok.',
  },
);
calisma.area('osb-hizmet-ve-destek-alanlari', 'Organize sanayi bölgesi hizmet ve destek alanları', [
  solid(rgb(224, 190, 230)),
  karelaj(2, 0.2),
  label([
    shape('rectangle', 16, { height: 14, fill: WHITE, stroke: BLACK, strokeWidth: 0.5 }),
    text('OSB', 4.2, { font: 'sans', weight: 700, offset: [0, 2.4] }),
    text('HDA', 4.2, { font: 'sans', weight: 700, offset: [0, -2.4] }),
  ]),
], {
  ref: 'EK-1d s.5; EK-1e s.23',
  note: '0.2 mm, 2 mm ara ile karolaj. Çerçeve EK-1d\'deki gibi 16×14 mm, iki satır "OSB / HDA".',
});

