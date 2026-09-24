import { BLACK, WHITE, along, crossHatch, edge, framed, grid, hatch, rgb, sheet, solid } from '../dsl';
import { S, box, gear, gridDots, pairs, picture, stack } from './common';

/**
 * NİP (EK-1ç) › Kentsel çalışma alanları. Renkler EK-1ç'den, tarama ölçüleri
 * EK-1e'nin NİP satırlarından. Alan içi kodlar 7 mm çerçevede; yazı
 * karakteri ve çerçeve kalınlığı EK-1ç çizimindeki gibi.
 */

const RED = rgb(224, 0, 33);

export const calisma = sheet('nip', ['Kentsel çalışma alanları'], 'calisma', 30);

calisma.area('merkezi-is-alani', 'Merkezi iş alanı (MİA)', [solid(RED), grid(2, 2, 0.2), box('MİA', { weight: 900, heavy: true })], {
  ref: 'EK-1ç s.3 (AÇIKLAMA 12); EK-1e s.55',
  note: '0,2 mm, 2 mm ara ile karolaj. EK-1ç örneği ızgarayı daha kalın ve seyrek basmış; ölçüler EK-1e\'den. MİA planda kademelendirilebilir.',
});

const TICARET = [
  { id: 't1', code: 'T1', name: 'Ticaret alanı (T1)' },
  { id: 't2', code: 'T2', name: 'Ticaret alanı (T2)' },
  { id: 't3', code: 'T3', name: 'Ticaret alanı (T3)' },
  { id: 't', code: 'T', name: 'Ticaret alanı (T)' },
] as const;
for (const t of TICARET) {
  calisma.area(`ticaret-alani-${t.id}`, t.name, [solid(RED), grid(3, 3, 0.3), box(t.code, { font: 'serif', max: 3.8 })], {
    ref: 'EK-1ç s.3 (AÇIKLAMA 14); EK-1e s.58',
    note: '0,3 mm, 3 mm ara ile karelaj. AÇIKLAMA 14: kademelenme öngörülüyorsa T1 (en üst) … T3 (en alt), öngörülmüyorsa yalnızca T kullanılır.',
  });
}

calisma.area('ticaret-konut-alani', 'Ticaret-konut alanı', [solid(rgb(227, 186, 69)), grid(3, 3, 0.3), box('TİCK', { font: 'serif' })], { ref: 'EK-1ç s.3; EK-1e s.58', note: '0,3 mm, 3 mm ara ile karelaj.' });
calisma.area('ticaret-turizm-alani', 'Ticaret-turizm alanı', [solid(rgb(255, 115, 0)), grid(3, 3, 0.3), box('TİCT', { font: 'serif' })], { ref: 'EK-1ç s.3; EK-1e s.59', note: '0,3 mm, 3 mm ara ile karelaj.' });
calisma.area('ticaret-turizm-konut-alani', 'Ticaret-turizm-konut alanı', [solid(rgb(255, 117, 0)), grid(3, 3, 0.3), box('TİCTK')], {
  ref: 'EK-1ç s.4; EK-1e s.59',
  note: '0,3 mm, 3 mm ara ile karelaj. Renk 255/117/0 (ticaret-turizmden 2 birim farklı; iki ekte de böyle).',
});
calisma.area('belediye-hizmet-alani', 'Belediye hizmet alanı', [solid(rgb(102, 153, 205)), gridDots(5, 0.4), box('BHA', { font: 'serif' })], {
  ref: 'EK-1ç s.4; EK-1e s.61',
  note: '0,4 mm, 5×5 mm karolaj merkezlerinde noktalama (nokta çapı çizgi kalınlığı).',
});
calisma.area('kamu-hizmet-alani', 'Kamu hizmet alanı', [solid(rgb(102, 153, 205)), gridDots(5, 0.4), box('KHA', { font: 'serif' })], {
  ref: 'EK-1ç s.4 (AÇIKLAMA 13); EK-1e s.62',
  note: '0,4 mm, 5×5 mm karolaj merkezlerinde noktalama. Kamu kurum ve kuruluşu, resmi kurum ve idari tesis alanlarını içerir.',
});
calisma.area('konut-disi-kentsel-calisma-alani', 'Konut dışı kentsel çalışma alanı', [solid(RED), hatch(45, 2, 0.3), hatch(135, 5, 0.3), box('KDKÇA', { weight: 900, heavy: true })], {
  ref: 'EK-1ç s.4; EK-1e s.63',
  note: '0,3 mm, 2 ve 5 mm ara ile 45 derece çapraz tarama: 2 mm aralık sağa yükselen (45°), 5 mm aralık sağa inen (135°) çizgilerde; hangisinin hangisi olduğu çizimden.',
});
calisma.area('akaryakit-ve-servis-istasyonu-alani', 'Akaryakıt ve servis istasyonu alanı', [solid(rgb(255, 56, 0)), grid(2, 2, 0.3), picture('akaryakit', 'A')], {
  ref: 'EK-1ç s.4; EK-1e s.64',
  note: '0,3 mm, 2 mm ara ile karelaj.',
});
calisma.area('sanayi-alani', 'Sanayi alanı', [solid(rgb(170, 102, 205)), crossHatch(3, 0.2)], { ref: 'EK-1ç s.4; EK-1e s.65', note: '0,2 mm, 3 mm ara ile 45 derece çapraz tarama.' });
calisma.area(
  'endustriyel-gelisme-bolgesi',
  'Endüstriyel gelişme bölgesi',
  [
    solid(rgb(232, 190, 255)),
    pairs(45, 3),
    pairs(135, 3),
    edge(BLACK, 0.3, { dash: [7, 17] }),
    along(gear(6), 24, { offsetAlong: 15.5, group: { count: 2, spacing: 7 } }),
    box('EGB'),
  ],
  {
    ref: 'EK-1ç s.4; EK-1e s.66',
    note: 'Tarama: 0,2 mm, 1 mm tarama çiftleri ile 45 derece çapraz tarama, çiftler arası 3 mm. Sınır: 0,3 mm; 6 mm dış çaplı içi dolu 1 mm aralıklı 2 dişli, 2 mm boşluk, 7 mm çizgi. Dişli 12 kare dişli, diş derinliği yarıçapın 0,22 katı (EK-1ç çiziminden ölçüldü; çizimde dişler raster yüzünden yuvarlak görünüyor).',
  },
);
calisma.area('kucuk-sanayi-alani', 'Küçük sanayi alanı', [solid(rgb(170, 102, 205)), pairs(45, 4), hatch(135, 4, 0.2), box('KSA', { weight: 900, heavy: true })], {
  ref: 'EK-1ç s.4; EK-1e s.66',
  note: '0,2 mm, 4 mm aralıklı: 45 derecede 1 mm tarama çiftleri, 135 derecede tek tarama.',
});
calisma.area('depolama-alani', 'Depolama alanı', [solid(RED), pairs(45, 3), pairs(135, 3), box('D', { weight: 900, heavy: true, max: 4.4 })], {
  ref: 'EK-1ç s.4; EK-1e s.67',
  note: '0,2 mm, 1 mm tarama çiftleri ile 45 derece karelaj, çiftler arası 3 mm. Renk EK-1ç\'den (224/0/33); EK-1e bu satırda 194/158/215 veriyor (ÇDP rengi).',
});
calisma.area('lojistik-tesis-alani', 'Lojistik tesis alanı', [solid(RED), pairs(45, 3), pairs(135, 3), box('LTA', { font: 'serif' })], {
  ref: 'EK-1ç s.4; EK-1e s.67',
  note: '0,2 mm, 1 mm tarama çiftleri ile 45 derece karelaj, çiftler arası 3 mm. Renk EK-1ç\'den (224/0/33); EK-1e bu satırda 194/158/215 veriyor.',
});
calisma.area('toplu-isyerleri', 'Toplu işyerleri', [solid(RED), pairs(45, 3), pairs(135, 3), box('Tİ', { max: 4.2 })], {
  ref: 'EK-1ç s.4; EK-1e s.68',
  note: '0,2 mm, 1 mm tarama çiftleri ile 45 derece karelaj, çiftler arası 3 mm.',
});
calisma.area('tarim-ve-hayvancilik-tesis-alani', 'Tarım ve hayvancılık tesis alanı', [solid(rgb(233, 250, 190)), pairs(0, 3), pairs(90, 3)], {
  ref: 'EK-1ç s.4; EK-1e s.69',
  note: '0,2 mm, 1 mm tarama çiftleri ile karelaj (yatay ve dikey), çiftler arası 3 mm.',
});
calisma.area('askeri-alan', 'Askeri alan', [solid(rgb(168, 168, 0)), pairs(90, 5), picture('tufekler')], {
  ref: 'EK-1ç s.5; EK-1e s.70',
  note: '0,2 mm, 1 mm aralıklı dikey çift çizgiler, çift çizgiler arası 5 mm.',
});
calisma.area('pazar-alani', 'Pazar alanı', [solid(RED), crossHatch(6, 0.3), box('P', { weight: 900, heavy: true, max: 4.4 })], { ref: 'EK-1ç s.5; EK-1e s.68', note: '0,3 mm, 6 mm ara ile 45 derece karolaj.' });
calisma.area('su-urunleri-uretim-ve-yetistirme-tesisi', 'Su ürünleri üretim ve yetiştirme tesisi', [edge(rgb(0, 92, 230), 0.5, { dash: [10, 4] }), picture('balik', 'SÜA')], {
  ref: 'EK-1ç s.5; EK-1e s.69',
  note: 'Sınır: 0,5 mm, 4 mm aralıklı kesikli çizgi, 0/92/230. Çizgi boyu yazılı değil, 10 mm alındı (çizimde çizgi ≈ 2,5 × boşluk). EK-1e alan rengi vermiyor, 0/92/230 sınır rengi; EK-1ç bu rengi ALAN RENK sütununa yazmış. Alan dolgusuz bırakıldı.',
});
calisma.area('beton-santrali', 'Beton santrali', [solid(rgb(170, 102, 205)), crossHatch(6, 0.3), box('BS')], { ref: 'EK-1ç s.5; EK-1e s.70', note: '0,3 mm, 6 mm ara ile 45 derece karelaj.' });
calisma.area(
  'organize-sanayi-bolgesi-hizmet-ve-destek-alanlari',
  'Organize sanayi bölgesi hizmet ve destek alanları',
  [solid(rgb(224, 190, 230)), grid(2, 2, 0.2), stack([...framed('', { size: S, strokeWidth: 0.35, background: WHITE }), ...['OSB', 'HDA'].map((t, i) => framed(t, { frame: 'none', textSize: 2.4, font: 'sans', weight: 700, offset: [0, i ? -1.4 : 1.4] })).flat()])],
  { ref: 'EK-1ç s.5; EK-1e s.23', note: '0,2 mm, 2 mm ara ile karolaj. Çerçevede iki satır: OSB / HDA.' },
);
