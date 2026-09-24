import { BLACK, WHITE, along, crossHatch, edge, framed, grid, hatch, pattern, rgb, shape, sheet, solid } from '../dsl';
import { S, box, boxMarks5, captionMark, gear, gridDots, pairs, pictureMarks5, speckle, stack, turned } from './common';

/**
 * ÇDP (EK-1c) › Kentsel çalışma alanları (EK-1e s.55-67, 174, 178). Turizm
 * bölgesinin alt türleri (G, ST, YT, KT, ET) tabloda çizili değil, AÇIKLAMA
 * 9 ile tanımlı: tarama ve alan rengi aynı, sembol rengi 255/170/0.
 */

const RED = rgb(224, 0, 33);
const LILAC = rgb(194, 158, 215);
const ORANGE = rgb(255, 115, 0);

/** Speckled 3 mm discs on a 9 mm grid, staggered: the rows half a column pitch apart as drawn. */
const discs = () => pattern(speckle(3), 9, 4.5, { stagger: true });
const discNote =
  '0,2 mm, 3 mm çapında serbest noktalama 9 mm karolaj merkezlerinde, şaşırtmalı sıra. Her benekli daire, daire içinde rastgele noktalardan oluşan tek bir çizim (benekli-daire); şaşırtmalı sıralar EK-1c çizimindeki gibi sütun aralığının yarısı (4,5 mm) arayla.';

/** A code in a 5 mm circle (T, GTB …); `fill` paints the circle (the turizm sub-types). */
const ring = (code: string, o: { fill?: string; weight?: 400 | 700; textSize?: number; strokeWidth?: number } = {}) =>
  stack(framed(code, { frame: 'circle', size: S, strokeWidth: o.strokeWidth ?? 0.3, background: o.fill ?? WHITE, textSize: o.textSize ?? 2.4, font: 'sans', weight: o.weight ?? 700 }));

export const calisma = sheet('cdp', ['Kentsel çalışma alanları'], 'calisma', 40);

calisma.area('merkezi-is-alani', 'Merkezi iş alanı (MİA) (1. derece merkez)', [solid(RED), grid(2, 2, 0.2), box('MİA', { heavy: true })], { ref: 'EK-1c s.1; EK-1e s.55', note: '0,2 mm, 2 mm ara ile karolaj.' });
calisma.area('tali-merkez', 'Tali merkez (2. ve 3. derece merkezler)', [solid(RED), grid(3, 3, 0.2), box('M', { heavy: true, max: 3.2 })], { ref: 'EK-1c s.1; EK-1e s.55', note: '0,2 mm, 3 mm ara ile karolaj.' });
calisma.area('lojistik-bolge', 'Lojistik bölge', [solid(LILAC), pairs(45, 3), pairs(135, 3), box('LB', { heavy: true })], {
  ref: 'EK-1c s.2; EK-1e s.56',
  note: '0,2 mm, 1 mm tarama çiftleri ile 45 derece çapraz karolaj, çiftler arası 3 mm.',
});
calisma.area('sanayi-ve-depolama-bolgesi', 'Sanayi ve depolama bölgesi', [solid(rgb(170, 102, 205)), crossHatch(2, 0.2)], { ref: 'EK-1c s.2; EK-1e s.56', note: '0,2 mm, 2 mm ara ile 45 derece çapraz tarama.' });
calisma.area(
  'depolama-alani',
  'Depolama alanı',
  [
    solid(LILAC),
    pairs(45, 3),
    pairs(135, 3),
    stack([
      ...boxMarks5('D', { weight: 900, max: 3.2 }),
      // The legend's frame is heavier on the bottom and the right (a drop shadow).
      shape('line', S, { stroke: BLACK, strokeWidth: 0.5, offset: [0.25, -S / 2 - 0.2] }),
      shape('line', S, { stroke: BLACK, strokeWidth: 0.5, rotation: 90, offset: turned(S / 2 + 0.2, -0.25, 90) }),
    ]),
  ],
  { ref: 'EK-1c s.2; EK-1e s.67', note: '0,2 mm, 1 mm tarama çiftleri ile 45 derece karelaj, çiftler arası 3 mm. Çerçevenin alt ve sağ kenarı EK-1c\'deki gibi gölgeli (kalın).' },
);
calisma.area(
  'endustriyel-gelisme-bolgesi',
  'Endüstriyel gelişme bölgesi',
  [solid(rgb(232, 190, 255)), pairs(45, 3), pairs(135, 3), edge(BLACK, 0.3, { dash: [7, 15] }), along(gear(5), 22, { offsetAlong: 14.5, group: { count: 2, spacing: 6 } }), box('EGB')],
  {
    ref: 'EK-1c s.2; EK-1e s.66',
    note: 'Tarama: 0,2 mm, 1 mm tarama çiftleri ile 45 derece çapraz tarama, çiftler arası 3 mm. Sınır: 0,3 mm; 5 mm dış çaplı içi dolu 1 mm aralıklı 2 dişli, 2 mm boşluk, 7 mm çizgi. Dişli 12 kare dişli, diş derinliği yarıçapın 0,22 katı (EK-1ç çiziminden ölçüldü; çizimde dişler raster yüzünden yuvarlak görünüyor). EK-1c taraması UİP aralığıyla (5 mm) basılmış; ölçü EK-1e ÇDP satırından.',
  },
);
calisma.area('turizm-bolgesi', 'Turizm bölgesi', [solid(ORANGE), discs(), ring('T')], { ref: 'EK-1c s.2 (AÇIKLAMA 9); EK-1e s.57', note: discNote });

const SUBTYPES = [
  { id: 'gunubirlik', name: 'Turizm bölgesi, günübirlik turizm', code: 'G' },
  { id: 'saglik', name: 'Turizm bölgesi, sağlık turizmi', code: 'ST' },
  { id: 'yayla', name: 'Turizm bölgesi, yayla turizmi', code: 'YT' },
  { id: 'kis', name: 'Turizm bölgesi, kış turizmi', code: 'KT' },
  { id: 'eko', name: 'Turizm bölgesi, ekoturizm', code: 'ET' },
] as const;
for (const t of SUBTYPES) {
  calisma.area(`turizm-bolgesi-${t.id}`, t.name, [solid(ORANGE), discs(), ring(t.code, { fill: rgb(255, 170, 0), textSize: t.code.length > 1 ? 2 : 2.4 })], {
    ref: 'EK-1c s.4-5 (AÇIKLAMA 9); EK-1e s.57',
    note: `${discNote} AÇIKLAMA 9: turizm türü çeşitlendirilirse tarama ve alan rengi aynı kalır, sembol rengi 255/170/0 olur. Sembolün biçimi tabloda yok: "T" ile aynı daire, içi 255/170/0 boyalı, harfler siyah (yorum).`,
  });
}

calisma.area('golf-turizm-bolgesi', 'Golf turizm bölgesi', [solid(rgb(161, 194, 114)), discs(), ring('GTB', { weight: 400, textSize: 1.7, strokeWidth: 0.4 })], {
  ref: 'EK-1c s.2; EK-1e s.57',
  note: `${discNote} EK-1e sembolü ölçeğe göre değiştirir (GTB, GTA, GTT); ÇDP'de GTB.`,
});
calisma.area('kentsel-servis-alani', 'Kentsel servis alanı', [solid(rgb(200, 100, 150)), crossHatch(2, 0.2)], { ref: 'EK-1c s.2 (AÇIKLAMA 6); EK-1e s.65', note: '0,2 mm, 2 mm ara ile 45 derece çapraz tarama.' });
calisma.area('tercihli-kullanim-bolgesi', 'Tercihli kullanım bölgesi', [solid(rgb(255, 85, 0)), hatch(90, 1.5, 0.2), hatch(0, 6, 0.2)], {
  ref: 'EK-1c s.2 (AÇIKLAMA 7); EK-1e s.60',
  note: '0,2 mm; 1,5 mm ara ile dikey, 6 mm ara ile yatay tarama.',
});
calisma.area('kamu-hizmet-alani', 'Kamu hizmet alanı', [solid(rgb(102, 153, 205)), gridDots(5, 0.4), box('KHA', { font: 'serif' })], { ref: 'EK-1c s.2; EK-1e s.62', note: '0,4 mm, 5×5 mm ara ile karolaj merkezlerinde noktalama.' });
calisma.area(
  'enerji-depolama-alani',
  'Enerji depolama alanı',
  [solid(LILAC), hatch(0, 6, 0.2), hatch(45, 2, 0.2), stack([...pictureMarks5('simsek'), captionMark('EDA', 1.9, S, 'sans', 400)])],
  { ref: 'EK-1c s.2; EK-1e s.178', note: '0,2 mm, 6 mm aralıklı yatay çizgiler ve 2 mm aralıklı 45 derecelik çizgiler. Alt yazı EK-1c\'deki gibi ince Arial.' },
);
calisma.area('yenilenebilir-enerji-kaynaklarina-dayali-uretim-tesisi-alani', 'Yenilenebilir enerji kaynaklarına dayalı üretim tesisi alanı', [solid(rgb(171, 171, 200)), stack(pictureMarks5('simsek', 'YED'))], {
  ref: 'EK-1c s.2; EK-1e s.174',
  note: 'Tarama yok (EK-1c). EK-1e ÇDP satırı bunu nokta sembolü olarak verir ("proje alanına uygun büyüklükte YED sembolü"); nokta hâli ayrı işaret.',
});
calisma.point('yenilenebilir-enerji-kaynaklarina-dayali-uretim-tesisi', 'Yenilenebilir enerji kaynaklarına dayalı üretim tesisi (nokta)', [pictureMarks5('simsek', 'YED')], {
  ref: 'EK-1e s.174',
  note: 'EK-1e ÇDP geometrisi nokta: proje alanına uygun büyüklükte YED sembolü. Varsayılan boyut 5 mm (AÇIKLAMA 4), gerekirse ölçeklenir.',
});
