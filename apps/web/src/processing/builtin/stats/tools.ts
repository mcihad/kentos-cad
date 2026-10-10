import { statsCenters, statsDbscan, statsHotSpots, statsKMeans, statsMoran, statsNearest, type StatsConcept } from '../../../model/ops/spatialStats';
import { defineTool, type Shown } from '../../types';
import { GEO_KINDS, GEO_SCOPES, outputStyle } from '../geometry/shared';
import { CLUSTER_RENDERER, GEOGRAPHIC, HOT_RENDERER, attrOf, geographic, refusal, resultOf, shapesOf } from './shared';

/**
 * Mekânsal istatistik (docs/adr/0238; ArcGIS's Spatial Statistics toolbox, QGIS's Mean coordinates, Nearest neighbour
 * analysis, DBSCAN and K-means clustering): where objects lie and how their values sit among their neighbours. Every
 * run is the geometry core's (`ops::spatial_stats`), its texts written there; an object's place is its centroid.
 */

const input = (description: string) => ({ name: 'input', label: 'Nesneler', type: 'features', kinds: GEO_KINDS, scopes: GEO_SCOPES, description }) as const;
const weightField = { name: 'weightField', label: 'Ağırlık alanı', type: 'field', of: 'input', optional: true, description: 'Seçilirse her nesne bu alanın değeriyle ağırlıklıdır; okunamayan ve eksi değerli nesne alınmaz.' } as const;
const groupField = { name: 'groupField', label: 'Grup alanı', type: 'field', of: 'input', optional: true, description: 'Seçilirse her grubun sonucu ayrı yazılır; boş değerliler “(boş)” grubudur.' } as const;
const deviations = {
  name: 'deviations',
  label: 'Standart sapma',
  type: 'enum',
  options: [
    { value: '1', label: '1 kat', hint: 'Noktaların yaklaşık %63’ü (elipste)' },
    { value: '2', label: '2 kat', hint: 'Yaklaşık %98' },
    { value: '3', label: '3 kat', hint: 'Yaklaşık %99,9' },
  ],
  default: '1',
  description: 'Sonucun kaç standart sapma boyunda yazılacağı.',
} as const;
const valueField = { name: 'valueField', label: 'Değer alanı', type: 'field', of: 'input', description: 'Değerleri sayı olarak okunan alan; okunamayan nesne alınmaz.' } as const;
const concepts = (inverse: boolean) =>
  ({
    name: 'concept',
    label: 'Komşuluk',
    type: 'enum',
    options: [
      { value: 'band', label: 'Sabit bant', hint: 'Bant içindeki her nesne komşu, ağırlığı 1' },
      ...(inverse ? [{ value: 'inverse', label: 'Ters uzaklık', hint: 'Bant içinde ağırlık 1 / uzaklık; 1 m’den yakın 1 m sayılır' }] : []),
      { value: 'nearest', label: 'k en yakın', hint: 'Her nesnenin en yakın k komşusu, ağırlığı 1' },
    ],
    default: 'band',
    description: 'Nesnelerin komşularının nasıl bulunacağı.',
  }) as const;
const band = {
  name: 'band',
  label: 'Bant',
  type: 'number',
  optional: true,
  default: null,
  min: 0.001,
  unit: 'm',
  placeholder: 'Kendiliğinden',
  visibleWhen: (v: Shown) => v.concept !== 'nearest',
  description: 'Boş bırakılırsa en büyük en yakın komşu uzaklığı: her nesnenin en az bir komşusu olur.',
} as const;
const neighbors = {
  name: 'neighbors',
  label: 'Komşu sayısı (k)',
  type: 'number',
  integer: true,
  min: 1,
  max: 100,
  default: 8,
  visibleWhen: (v: Shown) => v.concept === 'nearest',
  description: 'Her nesnenin en yakın kaç komşusu; eşit uzaklıkta önce gelen.',
} as const;
const layer = (newName: string, color: string, cross = false) =>
  ({
    name: 'layer',
    label: 'Çıktı katmanı',
    type: 'layer',
    default: { newName },
    // Right above the input's layer: the results over the objects they come from.
    above: 'input',
    // A centre is drawn as a cross, apart from the places it sums.
    newLayerStyle: cross ? { ...outputStyle(color), point: { symbol: 'cross' as const, size: 12 } } : outputStyle(color),
    description: 'Bu adda katman yoksa oluşturulur.',
  }) as const;
const tableOutputs = (first: { name: string; label: string }) =>
  [
    { name: 'table', label: 'Sonuç tablosu', type: 'table' },
    { name: first.name, label: first.label, type: 'number' },
    { name: 'z', label: 'z', type: 'number' },
    { name: 'p', label: 'p', type: 'number' },
  ] as const;

const CENTER_HELP = [
  'Nesnenin yeri ağırlık merkezidir: noktanın kendisi (çok noktalının ortalaması), alanın, dairenin ve bütün elipsin ağırlık merkezi, çizginin ortası, çoklu çizginin ortadaki köşesi; ifadelerin $merkez_y ve $merkez_x’i.',
  'Ağırlık alanı seçilirse değerler sayı olarak okunur (ondalık nokta ya da virgül); okunamayan, boş ve eksi değerli nesneler alınmaz ve söylenir. Grup alanı seçilirse gruplar adlarının doğal sırasıyla, boş değerliler “(boş)” grubu olarak en sonda yazılır.',
  'Coğrafi koordinatlı projede çalışmaz: uzaklıklar metre olmalıdır.',
].join('\n\n');

export const statsMeanCenter = defineTool({
  id: 'stats.meanCenter',
  label: 'Ortalama merkez',
  category: 'spatialStats',
  icon: 'statsMeanCenter',
  description: 'Nesnelerin (isterseniz ağırlıklı) ortalama merkezini, gruplara göre de, nokta olarak yazar.',
  help: CENTER_HELP,
  keywords: ['ortalama merkez', 'merkez', 'ağırlık merkezi', 'mean center', 'mean coordinates', 'centroid', 'istatistik'],
  aliases: ['ORTALAMAMERKEZ', 'MEANCENTER'],
  targets: ['client', 'worker'],
  parameters: [input('Merkezi bulunacak noktalar, çizgiler ve alanlar.'), weightField, groupField, layer('Ortalama merkez', '#D6336C', true)] as const,
  outputs: [
    { name: 'centers', label: 'Merkezler', type: 'features' },
    { name: 'count', label: 'Merkez sayısı', type: 'number' },
  ],
  run: (v, ctx, feedback) => {
    if (geographic(ctx)) return { refused: GEOGRAPHIC };
    const list = v.input.entities;
    const w = v.weightField ?? '';
    const g = v.groupField ?? '';
    try {
      const run = statsCenters(shapesOf(list), 'mean', w ? list.map((e) => attrOf(e, w)) : null, g ? list.map((e) => attrOf(e, g)) : null, w, 1);
      return resultOf(run, feedback, list, v.layer.id);
    } catch (e) {
      return refusal(e);
    }
  },
});

export const statsMedianCenter = defineTool({
  id: 'stats.medianCenter',
  label: 'Ortanca merkez',
  category: 'spatialStats',
  icon: 'statsMedianCenter',
  description: 'Nesnelere uzaklıklarının (isterseniz ağırlıklı) toplamını en küçük yapan noktayı, gruplara göre de, yazar.',
  help: [
    'Ortanca merkez (geometrik ortanca) aykırı nesnelerden ortalama merkez kadar etkilenmez. Weiszfeld’in yinelemesiyle ortalama merkezden başlanarak bulunur; ortanca bir nesnenin yerindeyse o yer yazılır.',
    CENTER_HELP,
  ].join('\n\n'),
  keywords: ['ortanca merkez', 'medyan merkez', 'geometrik ortanca', 'median center', 'weber', 'istatistik'],
  aliases: ['ORTANCAMERKEZ', 'MEDIANCENTER'],
  targets: ['client', 'worker'],
  parameters: [input('Merkezi bulunacak noktalar, çizgiler ve alanlar.'), weightField, groupField, layer('Ortanca merkez', '#7048E8', true)] as const,
  outputs: [
    { name: 'centers', label: 'Merkezler', type: 'features' },
    { name: 'count', label: 'Merkez sayısı', type: 'number' },
  ],
  run: (v, ctx, feedback) => {
    if (geographic(ctx)) return { refused: GEOGRAPHIC };
    const list = v.input.entities;
    const w = v.weightField ?? '';
    const g = v.groupField ?? '';
    try {
      const run = statsCenters(shapesOf(list), 'median', w ? list.map((e) => attrOf(e, w)) : null, g ? list.map((e) => attrOf(e, g)) : null, w, 1);
      return resultOf(run, feedback, list, v.layer.id);
    } catch (e) {
      return refusal(e);
    }
  },
});

export const statsStandardDistance = defineTool({
  id: 'stats.standardDistance',
  label: 'Standart uzaklık',
  category: 'spatialStats',
  icon: 'statsStandardDistance',
  description: 'Nesnelerin ortalama merkezin çevresinde ne kadar yayıldığını, standart uzaklık yarıçaplı daire olarak yazar.',
  help: [
    'Standart uzaklık, nesnelerin ortalama merkeze uzaklıklarının karelerinin (ağırlıklı) ortalamasının köküdür. Daire Standart sapma kadar katıyla çizilir; tek nesneli ya da çakışık nesneli grubunki sıfırdır, yazılmaz.',
    CENTER_HELP,
  ].join('\n\n'),
  keywords: ['standart uzaklık', 'yayılım', 'dağılım', 'standard distance', 'dispersion', 'istatistik'],
  aliases: ['STANDARTUZAKLIK', 'STANDARDDISTANCE'],
  targets: ['client', 'worker'],
  parameters: [input('Yayılımı ölçülecek noktalar, çizgiler ve alanlar.'), weightField, groupField, deviations, layer('Standart uzaklık', '#1C7ED6')] as const,
  outputs: [
    { name: 'circles', label: 'Daireler', type: 'features' },
    { name: 'count', label: 'Daire sayısı', type: 'number' },
  ],
  run: (v, ctx, feedback) => {
    if (geographic(ctx)) return { refused: GEOGRAPHIC };
    const list = v.input.entities;
    const w = v.weightField ?? '';
    const g = v.groupField ?? '';
    try {
      const run = statsCenters(shapesOf(list), 'distance', w ? list.map((e) => attrOf(e, w)) : null, g ? list.map((e) => attrOf(e, g)) : null, w, Number(v.deviations));
      return resultOf(run, feedback, list, v.layer.id);
    } catch (e) {
      return refusal(e);
    }
  },
});

export const statsDirectionalDistribution = defineTool({
  id: 'stats.directionalDistribution',
  label: 'Yön dağılımı',
  category: 'spatialStats',
  icon: 'statsEllipse',
  description: 'Nesnelerin hangi doğrultuda yayıldığını standart sapma elipsi olarak yazar: büyük ve küçük yarı eksenleri, doğrultusu.',
  help: [
    'Elipsin eksenleri koordinatların (ağırlıklı) kovaryansının özvektörleridir; yarı eksenler √2 düzeltmesiyle yazılır, bir standart sapmalık elips noktaların yaklaşık %63’ünü alır. Doğrultu büyük eksenin kuzeyden saat yönünde açısıdır (0–180°).',
    'Nesneleri bir doğru üzerinde ya da çakışık olan grubun elipsi yazılmaz.',
    CENTER_HELP,
  ].join('\n\n'),
  keywords: ['yön dağılımı', 'standart sapma elipsi', 'elips', 'directional distribution', 'standard deviational ellipse', 'istatistik'],
  aliases: ['YONDAGILIMI', 'DIRECTIONALDISTRIBUTION'],
  targets: ['client', 'worker'],
  parameters: [input('Doğrultusu bulunacak noktalar, çizgiler ve alanlar.'), weightField, groupField, deviations, layer('Yön dağılımı', '#0CA678')] as const,
  outputs: [
    { name: 'ellipses', label: 'Elipsler', type: 'features' },
    { name: 'count', label: 'Elips sayısı', type: 'number' },
  ],
  run: (v, ctx, feedback) => {
    if (geographic(ctx)) return { refused: GEOGRAPHIC };
    const list = v.input.entities;
    const w = v.weightField ?? '';
    const g = v.groupField ?? '';
    try {
      const run = statsCenters(shapesOf(list), 'ellipse', w ? list.map((e) => attrOf(e, w)) : null, g ? list.map((e) => attrOf(e, g)) : null, w, Number(v.deviations));
      return resultOf(run, feedback, list, v.layer.id);
    } catch (e) {
      return refusal(e);
    }
  },
});

export const statsNearestNeighbor = defineTool({
  id: 'stats.nearestNeighbor',
  label: 'En yakın komşu',
  category: 'spatialStats',
  icon: 'statsNearest',
  description: 'Nesnelerin en yakın komşularına ortalama uzaklığını rastgele dağılımınkiyle karşılaştırır: kümelenmiş mi, dağınık mı?',
  help: [
    'Gözlenen ortalama en yakın komşu uzaklığı aynı alanda rastgele dağılmış aynı sayıda nesnenin beklenen uzaklığıyla (0,5 / √(n / A)) karşılaştırılır (Clark ve Evans 1954). Oran 1’den küçük ve z eksiyse kümelenme, büyük ve artıysa dağınıklık vardır; p 0,05’ten küçükse desen anlamlıdır.',
    'Alan boş bırakılırsa nesnelerin yerlerini çevreleyen kutunun alanıdır; sonuç alana duyarlıdır, çalışma alanını biliyorsanız yazın. Çizim değişmez.',
  ].join('\n\n'),
  keywords: ['en yakın komşu', 'kümelenme', 'nearest neighbor', 'average nearest neighbor', 'clark evans', 'desen', 'istatistik'],
  aliases: ['ENYAKINKOMSU', 'NEARESTNEIGHBOR'],
  targets: ['client', 'worker'],
  parameters: [
    input('Deseni sınanacak noktalar, çizgiler ve alanlar.'),
    { name: 'area', label: 'Alan', type: 'number', optional: true, default: null, min: 0.001, unit: 'm²', placeholder: 'Kutudan', description: 'Çalışma alanı; boş bırakılırsa yerlerin kutusunun alanı.' },
  ] as const,
  outputs: tableOutputs({ name: 'ratio', label: 'En yakın komşu oranı' }),
  run: (v, ctx, feedback) => {
    if (geographic(ctx)) return { refused: GEOGRAPHIC };
    const list = v.input.entities;
    try {
      return resultOf(statsNearest(shapesOf(list), v.area ?? null), feedback, list, null);
    } catch (e) {
      return refusal(e);
    }
  },
});

export const statsMoransI = defineTool({
  id: 'stats.moransI',
  label: 'Moran I',
  category: 'spatialStats',
  icon: 'statsMoran',
  description: 'Bir değerin komşu nesnelerde benzer olup olmadığını Moran I ile sınar: kümelenmiş mi, dağınık mı, rastgele mi?',
  help: [
    'Moran I artıysa benzer değerler yan yana (kümelenme), eksiyse farklılar yan yana (dağınıklık); beklenen değer −1 / (n − 1). z ve p, rastgele dağıtım varsayımıyla bulunur (Cliff ve Ord); p 0,05’ten küçükse desen anlamlıdır.',
    'Komşuluk: sabit uzaklık bandı, bant içinde ters uzaklık ya da k en yakın komşu. Bant boş bırakılırsa her nesnenin en az bir komşusu olacak en küçük uzaklık alınır. Satır standartlaştırma her nesnenin komşularının ağırlıklarını toplamına böler. Çizim değişmez.',
  ].join('\n\n'),
  keywords: ['moran', 'otokorelasyon', 'mekânsal otokorelasyon', 'spatial autocorrelation', 'global moran', 'kümelenme', 'istatistik'],
  aliases: ['MORAN', 'MORANI'],
  targets: ['client', 'worker'],
  parameters: [
    input('Değeri sınanacak noktalar, çizgiler ve alanlar.'),
    valueField,
    concepts(true),
    band,
    neighbors,
    { name: 'standardize', label: 'Satır standartlaştırma', type: 'boolean', default: true, description: 'Her nesnenin komşu ağırlıkları toplamına bölünür (önerilen).' },
  ] as const,
  outputs: tableOutputs({ name: 'moransI', label: 'Moran I' }),
  run: (v, ctx, feedback) => {
    if (geographic(ctx)) return { refused: GEOGRAPHIC };
    const list = v.input.entities;
    try {
      const run = statsMoran(shapesOf(list), list.map((e) => attrOf(e, v.valueField)), v.valueField, v.concept as StatsConcept, v.band ?? null, v.neighbors, v.standardize);
      return resultOf(run, feedback, list, null);
    } catch (e) {
      return refusal(e);
    }
  },
});

export const statsHotSpot = defineTool({
  id: 'stats.hotSpot',
  label: 'Sıcak nokta (Gi*)',
  category: 'spatialStats',
  icon: 'statsHotSpot',
  description: 'Yüksek ve düşük değerlerin kümelendiği yerleri Getis-Ord Gi* ile bulur; her nesnenin kopyasını z, p ve güven sınıfıyla renkli yazar.',
  help: [
    'Her nesnenin ve komşularının değer toplamı, bütün nesnelerden beklenenle karşılaştırılır (Ord ve Getis 1995); sonuç bir z puanıdır. Büyük artı z sıcak nokta (yüksek değerler kümesi), büyük eksi z soğuk noktadır.',
    'Güven sınıfı z’nin işaretiyle 3 (%99), 2 (%95), 1 (%90) ya da 0’dır; kopyalar sınıflarının rengini alır (kırmızılar sıcak, maviler soğuk). Komşuluk sabit uzaklık bandı ya da k en yakın komşu; nesne kendi komşusudur. Bant boş bırakılırsa her nesnenin en az bir komşusu olacak en küçük uzaklık alınır.',
  ].join('\n\n'),
  keywords: ['sıcak nokta', 'soğuk nokta', 'getis', 'gi*', 'hot spot', 'hotspot analysis', 'kümelenme', 'istatistik'],
  aliases: ['SICAKNOKTA', 'HOTSPOT', 'GISTAR'],
  targets: ['client', 'worker'],
  parameters: [input('Değeri sınanacak noktalar, çizgiler ve alanlar.'), valueField, concepts(false), band, neighbors, { ...layer('Sıcak noktalar', '#E03131'), newLayerStyle: { ...outputStyle('#E03131'), renderer: HOT_RENDERER } }] as const,
  outputs: [
    { name: 'spots', label: 'Nesneler', type: 'features' },
    { name: 'count', label: 'Nesne sayısı', type: 'number' },
    { name: 'hot', label: 'Sıcak nokta', type: 'number' },
    { name: 'cold', label: 'Soğuk nokta', type: 'number' },
  ],
  run: (v, ctx, feedback) => {
    if (geographic(ctx)) return { refused: GEOGRAPHIC };
    const list = v.input.entities;
    try {
      const run = statsHotSpots(shapesOf(list), list.map((e) => attrOf(e, v.valueField)), v.valueField, v.concept as StatsConcept, v.band ?? null, v.neighbors);
      return resultOf(run, feedback, list, v.layer.id);
    } catch (e) {
      return refusal(e);
    }
  },
});

export const statsDbscanTool = defineTool({
  id: 'stats.dbscan',
  label: 'DBSCAN kümeleme',
  category: 'spatialStats',
  icon: 'statsDbscan',
  description: 'Yakın nesneleri yoğunluklarına göre kümelere ayırır; seyrek kalanlar gürültüdür. Her nesnenin kopyası küme numarası ve kümenin rengiyle yazılır.',
  help: [
    'Yarıçap içinde (kendisi dahil) en az nokta kadar nesnesi olan nesne çekirdektir; birbirine yarıçaptan yakın çekirdekler ve onlara yakın nesneler bir kümedir (Ester ve arkadaşları 1996). Nesneler çizimdeki sırasıyla gezilir; iki kümeye yakın sınır nesnesi ilk ulaşan kümeye girer.',
    'Sınır noktaları gürültü açıkken (DBSCAN*) yalnız çekirdekler kümelenir. Gürültünün küme numarası 0’dır.',
  ].join('\n\n'),
  keywords: ['dbscan', 'kümeleme', 'yoğunluk', 'density based clustering', 'cluster', 'gürültü', 'istatistik'],
  aliases: ['DBSCAN'],
  targets: ['client', 'worker'],
  parameters: [
    input('Kümelenecek noktalar, çizgiler ve alanlar.'),
    { name: 'radius', label: 'Yarıçap', type: 'number', min: 0.001, default: 50, unit: 'm', description: 'Komşuluğun yarıçapı (ε).' },
    { name: 'minPoints', label: 'En az nokta', type: 'number', integer: true, min: 1, max: 1000, default: 5, description: 'Çekirdek olmak için yarıçap içinde, kendisi dahil, gereken nesne sayısı.' },
    { name: 'borderNoise', label: 'Sınır noktaları gürültü', type: 'boolean', default: false, description: 'Açıkken yalnız çekirdekler kümelenir (DBSCAN*).' },
    { ...layer('Kümeler (DBSCAN)', '#F08C00'), newLayerStyle: { ...outputStyle('#F08C00'), renderer: CLUSTER_RENDERER } },
  ] as const,
  outputs: [
    { name: 'members', label: 'Nesneler', type: 'features' },
    { name: 'count', label: 'Nesne sayısı', type: 'number' },
    { name: 'clusters', label: 'Küme sayısı', type: 'number' },
  ],
  run: (v, ctx, feedback) => {
    if (geographic(ctx)) return { refused: GEOGRAPHIC };
    const list = v.input.entities;
    try {
      return resultOf(statsDbscan(shapesOf(list), v.radius, v.minPoints, v.borderNoise), feedback, list, v.layer.id);
    } catch (e) {
      return refusal(e);
    }
  },
});

export const statsKMeansTool = defineTool({
  id: 'stats.kMeans',
  label: 'k-ortalamalar kümeleme',
  category: 'spatialStats',
  icon: 'statsKMeans',
  description: 'Nesneleri yerlerine göre k kümeye ayırır: her nesne en yakın küme merkezine. Her nesnenin kopyası küme numarası ve kümenin rengiyle yazılır.',
  help: [
    'Başlangıç merkezleri kendiliğinden ve hep aynı seçilir: ilki ortalama merkeze en yakın nesne, sonrakiler seçilmiş merkezlere en uzak nesneler. Ardından her nesne en yakın merkeze atanır ve merkezler üyelerinin ortalaması olur; atamalar değişmeyene dek (en çok 500 kez) sürer (Lloyd).',
    'Farklı yer sayısı küme sayısından azsa çalışmaz.',
  ].join('\n\n'),
  keywords: ['k-ortalamalar', 'k means', 'kmeans', 'kümeleme', 'cluster', 'gruplama', 'istatistik'],
  aliases: ['KORTALAMA', 'KMEANS'],
  targets: ['client', 'worker'],
  parameters: [
    input('Kümelenecek noktalar, çizgiler ve alanlar.'),
    { name: 'clusters', label: 'Küme sayısı', type: 'number', integer: true, min: 2, max: 100, default: 5, description: 'Kaç küme (k).' },
    { ...layer('Kümeler (k-ortalamalar)', '#F08C00'), newLayerStyle: { ...outputStyle('#F08C00'), renderer: CLUSTER_RENDERER } },
  ] as const,
  outputs: [
    { name: 'members', label: 'Nesneler', type: 'features' },
    { name: 'count', label: 'Nesne sayısı', type: 'number' },
    { name: 'clusters', label: 'Küme sayısı', type: 'number' },
  ],
  run: (v, ctx, feedback) => {
    if (geographic(ctx)) return { refused: GEOGRAPHIC };
    const list = v.input.entities;
    try {
      return resultOf(statsKMeans(shapesOf(list), v.clusters), feedback, list, v.layer.id);
    } catch (e) {
      return refusal(e);
    }
  },
});

export const STATS_TOOLS = [
  statsMeanCenter,
  statsMedianCenter,
  statsStandardDistance,
  statsDirectionalDistribution,
  statsNearestNeighbor,
  statsMoransI,
  statsHotSpot,
  statsDbscanTool,
  statsKMeansTool,
] as const;
