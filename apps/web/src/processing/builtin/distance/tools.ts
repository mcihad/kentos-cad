import { defineTool, type Shown } from '../../types';
import { outputStyle } from '../geometry/shared';
import { ADD, layer, output } from '../rasterOps/shared';
import { BURN_KINDS } from '../rasterVector/shared';
import { count } from '../surface/shared';
import { DISTANCE, least3, runDistanceFromObjects, runDistancePaths, runDistanceRaster } from './shared';

/**
 * Uzaklık ve maliyet (docs/adr/0236 §3–§7): the four tools, their parameters as the desktop's
 * (`builtin/distance/tools.rs`); Uzaklık yüzeyi from objects runs the raster core's point job, the rest its operation
 * job over the cost raster (and the surface).
 */

const HELP_SIZE = 'Bütün raster bellekte çalışılır: en çok 2²⁵ hücre (5792 × 5792).';
const HELP_BURN =
  "Nesneler hücrelere Rasterleştir'in kuralıyla düşer: kapalı şekil merkezini içine alan hücrelere, açık şekil dokunduğu hücrelere, nokta içinde olduğu hücreye. Bir hücreye birden çok kaynak düşerse girdide önce gelenindir; hiçbir hücreye düşmeyen nesne söylenir.";
const HELP_COST =
  'Maliyet metre başınadır; değersiz hücre (nodata, NaN, alfası 0) engeldir. 0 ya da eksi değer reddedilir: geçilmeyecek yeri değersiz yapın, çok ucuz yere küçük bir artı değer verin.';
const HELP_STEP =
  "Hücre merkezleri 8 komşuya ya da at hamleleriyle 16 komşuya bağlanır. Dik ve çapraz adımın maliyeti iki hücrenin ortalaması çarpı adımın uzunluğu, at hamlesininki iki uçla geçtiği iki hücrenin ortalaması çarpı uzunluk (GRASS'ın r.cost'u gibi). Değersiz hücreye adım atılmaz; iki yanı da değersiz çapraz adım ve geçtiği hücrelerden biri değersiz at hamlesi atılmaz: köşeden bağlı engel geçilmez.";
const HELP_SURFACE =
  'Yükseklik modeliyle açıkken yükseklik modeli maliyet rasterinin ızgarasına çift doğrusal okunur: Yüzey uzunluğu açıkken adımın uzunluğu yükseklik farkıyla üç boyutta alınır; En büyük boyuna eğimden dik adım (iki yönde) atılmaz, yol yamaçta kıvrılır. Yükseklik modelinin değersiz hücresine adım atılmaz.';
const HELP_NET =
  'Ağın bozulması: sabit maliyette ağın maliyeti düz uzaklığın en çok %8,24 (8 komşu) ve %2,75 (16 komşu) fazlasıdır; her yolun maliyeti adımlarından yeniden hesaplanabilir.';
const ABOVE_COST = 'Bu adda katman yoksa oluşturulur; maliyet rasterinin katmanının hemen üstünde.';

const FILE = [{ name: 'file', label: 'Sonuç dosyası', type: 'string' }] as const;
const SCOPES = ['layer', 'selection', 'visible', 'all'] as const;
const RASTER_SCOPES = ['selection', 'layer'] as const;

const objects = <N extends string, L extends string>(name: N, label: L, description: string) =>
  ({ name, label, type: 'features', kinds: BURN_KINDS, scopes: SCOPES, description }) as const;
const raster = <N extends string, L extends string>(name: N, label: L, description: string) =>
  ({ name, label, type: 'features', kinds: ['raster'], scopes: RASTER_SCOPES, description }) as const;
const band = (description: string) => ({ name: 'band', label: 'Bant', type: 'number', default: 1, min: 1, max: 255, integer: true, unit: '', description }) as const;

const fromRaster = (v: Shown): boolean => v.from === 'raster';
const withSurface = (v: Shown): boolean => v.useSurface === true;

/** Maliyet rasteri and its band. */
const COST_RASTER = [raster('input', 'Maliyet rasteri', 'Metre başına maliyet; değersiz hücre engeldir.'), band("Maliyetin okunduğu bant (1'den).")] as const;

/** Komşuluk; Yükseklik modeliyle and, while it is on, Yükseklik modeli, Yüzey uzunluğu and En büyük boyuna eğim. */
const NETWORK = [
  {
    name: 'neighbours',
    label: 'Komşuluk',
    type: 'enum',
    options: [
      { value: '16', label: '16 (at hamleleriyle)' },
      { value: '8', label: '8' },
    ],
    default: '16',
    description: '16 komşuda ağın düz uzaklıktan sapması en çok %2,75, 8 komşuda %8,24.',
  },
  { name: 'useSurface', label: 'Yükseklik modeliyle', type: 'boolean', default: false, description: 'Yüzey uzunluğu ve En büyük boyuna eğim için bir yükseklik modeli okunur.' },
  { ...raster('surface', 'Yükseklik modeli', 'Maliyet rasterinin ızgarasına çift doğrusal okunur.'), optional: true, visibleWhen: withSurface },
  { name: 'surfaceLength', label: 'Yüzey uzunluğu', type: 'boolean', default: false, description: 'Adımın uzunluğu yükseklik farkıyla, üç boyutta.', visibleWhen: withSurface },
  { name: 'slope', label: 'En büyük boyuna eğim', type: 'number', default: 0, min: 0, max: 1000, unit: '', description: 'Yüzde; bundan dik adım atılmaz (iki yönde); 0: sınırsız.', visibleWhen: withSurface },
] as const;

const SAMPLE = {
  name: 'sample',
  label: 'Sonuç türü',
  type: 'enum',
  options: [
    { value: 'f32', label: 'Ondalık 32 bit' },
    { value: 'f64', label: 'Ondalık 64 bit' },
  ],
  default: 'f32',
} as const;

/** The network's settings as the core reads them (the surface's only while Yükseklik modeliyle is on). */
function networkOf(v: { neighbours?: string | null; useSurface?: boolean | null; surfaceLength?: boolean | null; slope?: number | null }) {
  const surface = v.useSurface === true;
  return { neighbours: v.neighbours === '8' ? '8' : '16', surfaceLength: surface && v.surfaceLength === true, slope: surface ? (v.slope ?? 0) : 0 };
}

export const distanceSurface = defineTool({
  id: 'distance.euclidean',
  label: 'Uzaklık yüzeyi',
  category: DISTANCE,
  icon: 'distanceSurface',
  description: 'Her hücreye en yakın kaynağın düz uzaklığını (m) ya da en yakın kaynağın numarasını yazar.',
  help: [
    'Kaynaklar nesneler (nokta, çizgi, alan) ya da bir rasterin bandının değerli hücreleridir. Hücrenin uzaklığı merkezinin en yakın kaynak hücrenin merkezine düz uzaklığıdır; kaynak hücrede 0. En yakın kaynak, eşitse sütunu, o da eşitse satırı küçük olan hücredir; En yakın kaynak sonucu nesnelerde nesnenin girdideki sırası, rasterde hücrenin değeridir.',
    "Hesap Felzenszwalb ve Huttenlocher'in kesin uzaklık dönüşümüdür: önce sütunlar, sonra satırlar, iş parçacıklarında; kare hücrede karşılaştırmalar tam sayılarla kesindir.",
    HELP_BURN,
    "Nesnelerde ızgara kaynakların kutusudur, her yanda Kenar payı kadar geniş; Hücre boyu 0 ise kutunun kısa kenarının 250'de biri, 1, 2, 2,5 ya da 5 × 10ᵏ'ye yuvarlanır. Rasterin ızgarası seçilirse sonuç o rasterle hücre hücre üst üste gelir. Rasterde ızgara rasterin kendisidir.",
    "Düz uzaklık engelin üstünden ölçülür; engelin çevresinden uzaklık için Birikimli maliyet'i 1 maliyetle kullanın. Izgaranın eksenleri dik olmalıdır; coğrafi ızgarada uzaklık metre olmadığından reddedilir.",
    "Sonuç 32 bit; Uzaklık'ta Viridis, En yakın kaynak'ta Spektral ve en yakın örnekleme. Çizime ekle açıksa nesnelerin sonucu kaynakların katmanının hemen altına, rasterinki rasterin katmanının hemen üstüne eklenir.",
    HELP_SIZE,
  ].join('\n\n'),
  keywords: ['uzaklık', 'mesafe', 'distance', 'proximity', 'euclidean', 'tampon', 'en yakın', 'allocation'],
  aliases: ['UZAKLIK', 'PROXIMITY', 'EUCDIST'],
  targets: ['client'],
  parameters: [
    {
      name: 'from',
      label: 'Kaynak',
      type: 'enum',
      options: [
        { value: 'objects', label: 'Nesneler' },
        { value: 'raster', label: 'Raster' },
      ],
      default: 'objects',
    },
    { ...objects('sources', 'Kaynaklar', 'Uzaklığın ölçüldüğü noktalar, çizgiler ve alanlar.'), visibleWhen: (v: Shown) => !fromRaster(v) },
    { ...raster('input', 'Kaynak raster', 'Bandının değerli hücreleri kaynaktır.'), visibleWhen: fromRaster },
    { ...band("Kaynak hücrelerin okunduğu bant (1'den)."), visibleWhen: fromRaster },
    { name: 'max', label: 'En büyük uzaklık', type: 'number', default: 0, min: 0, max: 1e9, unit: 'm', description: 'Bundan uzak hücreler değersiz; 0: sınırsız.' },
    {
      name: 'result',
      label: 'Sonuç',
      type: 'enum',
      options: [
        { value: 'distance', label: 'Uzaklık' },
        { value: 'allocation', label: 'En yakın kaynak' },
      ],
      default: 'distance',
    },
    {
      name: 'margin',
      label: 'Kenar payı',
      type: 'number',
      default: 100,
      min: 0,
      max: 1e7,
      unit: 'm',
      description: 'Kaynakların kutusu her yanda bu kadar genişletilir.',
      visibleWhen: (v: Shown) => !fromRaster(v) && v.extent !== 'raster',
    },
    {
      name: 'cellSize',
      label: 'Hücre boyu',
      type: 'number',
      default: 0,
      min: 0,
      max: 1e6,
      unit: 'm',
      description: "0: kendiliğinden, kutunun kısa kenarının 250'de biri yuvarlanarak (1, 2, 2,5, 5 × 10ᵏ).",
      visibleWhen: (v: Shown) => !fromRaster(v) && v.extent !== 'raster',
    },
    {
      name: 'extent',
      label: 'Kapsam',
      type: 'enum',
      options: [
        { value: 'objects', label: 'Kaynakların kutusu' },
        { value: 'raster', label: 'Rasterin ızgarası' },
      ],
      default: 'objects',
      description: 'Rasterin ızgarası: sonuç seçilen rasterle hücre hücre üst üste gelir.',
      visibleWhen: (v: Shown) => !fromRaster(v),
    },
    { ...raster('grid', 'Izgara rasteri', 'Izgarası (yeri, hücre boyu, boyu) alınan raster.'), optional: true, visibleWhen: (v: Shown) => !fromRaster(v) && v.extent === 'raster' },
    output('-uzaklik'),
    ADD,
    {
      ...layer('Uzaklık'),
      below: 'sources',
      description: 'Bu adda katman yoksa oluşturulur: nesnelerin sonucu kaynakların katmanının hemen altında, rasterinki rasterin katmanının hemen üstünde.',
    },
  ] as const,
  outputs: FILE,
  run: (v, ctx, feedback) => {
    const allocation = v.result === 'allocation';
    const result = allocation ? 'allocation' : 'distance';
    const max = v.max ?? 0;
    if (v.from !== 'raster') return runDistanceFromObjects(v, ctx, feedback, { kind: 'distance', max, result, margin: v.margin ?? 100 });
    const tool = { kind: 'distance', band: v.band ?? 1, max, result };
    return runDistanceRaster(v, ctx, feedback, tool, [], 'kaynak', '-uzaklik', 'Uzaklık yüzeyi hesaplanıyor', (d) => {
      let s = ` ${count(d.sources)} kaynak hücre`;
      if (!allocation && d.cells > 0) s += `; en uzak hücre ${least3(d.most)} m`;
      return `${s}.`;
    });
  },
});

export const costDistance = defineTool({
  id: 'distance.cost',
  label: 'Birikimli maliyet',
  category: DISTANCE,
  icon: 'costDistance',
  description: 'Her hücreye en ucuz kaynaktan varmanın birikimli maliyetini ya da o kaynağın numarasını yazar.',
  help: [
    HELP_COST,
    HELP_STEP,
    HELP_SURFACE,
    "Birikimli maliyet kaynak hücrelerde 0, öbürlerinde komşunun maliyeti artı adımın maliyetinin en küçüğüdür (Dijkstra); toplamlar float64'te yapılır, hesabın sırası sonucu değiştirmez. Erişilemeyen ya da En büyük maliyeti aşan hücre değersizdir. En ucuz kaynak hücrenin geldiği komşunun kaynağıdır: girdideki sırası.",
    HELP_BURN,
    HELP_NET,
    "Sonuç Birikimli maliyet'te Viridis ve yüzde gerdirme, En ucuz kaynak'ta Spektral ve en yakın örnekleme; Çizime ekle açıksa maliyet rasterinin katmanının hemen üstüne eklenir.",
    HELP_SIZE,
  ].join('\n\n'),
  keywords: ['maliyet', 'cost distance', 'birikimli', 'accumulation', 'erişim', 'güzergâh', 'r.cost', 'hizmet alanı'],
  aliases: ['MALIYET', 'COSTDIST'],
  targets: ['client'],
  parameters: [
    ...COST_RASTER,
    objects('sources', 'Kaynaklar', 'Maliyetin başladığı noktalar, çizgiler ve alanlar.'),
    ...NETWORK,
    { name: 'max', label: 'En büyük maliyet', type: 'number', default: 0, min: 0, max: 1e300, unit: '', description: 'Bundan pahalı hücreler değersiz; 0: sınırsız.' },
    {
      name: 'result',
      label: 'Sonuç',
      type: 'enum',
      options: [
        { value: 'cost', label: 'Birikimli maliyet' },
        { value: 'allocation', label: 'En ucuz kaynak' },
      ],
      default: 'cost',
    },
    { ...SAMPLE, visibleWhen: (v: Shown) => v.result !== 'allocation' },
    output('-maliyet'),
    ADD,
    { ...layer('Birikimli maliyet'), description: ABOVE_COST },
  ] as const,
  outputs: FILE,
  run: (v, ctx, feedback) => {
    const allocation = v.result === 'allocation';
    const tool = {
      kind: 'costDistance',
      band: v.band ?? 1,
      ...networkOf(v),
      max: v.max ?? 0,
      result: allocation ? 'allocation' : 'cost',
      sample: v.sample === 'f64' ? 'f64' : 'f32',
    };
    return runDistanceRaster(v, ctx, feedback, tool, v.sources?.entities ?? [], 'kaynak nesnesi', '-maliyet', 'Birikimli maliyet hesaplanıyor', (d, cells) => {
      let s = ` ${count(d.sources)} kaynak hücre`;
      if (!allocation && d.cells > 0) s += `; en büyük birikimli maliyet ${least3(d.most)}`;
      s += '.';
      if (d.cells < cells) s += ` Değersiz ${count(cells - d.cells)} hücre (engel, erişilemeyen ya da sınırın ötesi).`;
      return s;
    });
  },
});

export const costPath = defineTool({
  id: 'distance.path',
  label: 'En düşük maliyetli yol',
  category: DISTANCE,
  icon: 'costPath',
  description: 'Her varış nesnesinden en ucuz başlangıca giden en düşük maliyetli yolu çoklu çizgi olarak yazar.',
  help: [
    'Varış hücresi, varış nesnesinin hücrelerinden birikimli maliyeti en küçük olanıdır (eşitse numarası küçük olan). Yol, hücrenin geldiği komşular boyunca başlangıca iner: geldiği komşu, maliyeti ve adımı hücrenin maliyetini veren komşulardan maliyeti, o da eşitse numarası küçük olanıdır. Çizgi başlangıçtan varışa doğru hücre merkezlerindendir; Sadeleştirme hücre cinsinden Douglas–Peucker toleransıdır.',
    'Öznitelikler: Yol (varış nesnesinin girdideki sırası), Kaynak (başlangıç nesnesinin sırası), Maliyet (varış hücresinin birikimli maliyeti), Uzunluk (m, adımların plan uzunlukları); yükseklik modeli verilince Yüzey uzunluğu (m) ve En büyük eğim (%). Erişilemeyen ya da rasterin dışındaki varış atlanır, söylenir.',
    HELP_COST,
    HELP_STEP,
    HELP_SURFACE,
    'Dijkstra varışların hücreleri kesinleşince durur. Yollar maliyet rasterinin katmanının hemen üstündeki yeni katmana yazılır.',
    HELP_SIZE,
  ].join('\n\n'),
  keywords: ['yol', 'güzergâh', 'güzergah', 'least cost path', 'en ucuz', 'optimal path', 'r.path', 'boru hattı'],
  aliases: ['GUZERGAHBUL', 'COSTPATH'],
  targets: ['client'],
  parameters: [
    ...COST_RASTER,
    objects('sources', 'Başlangıç', 'Yolların başladığı noktalar, çizgiler ve alanlar: her varış en ucuzuna bağlanır.'),
    objects('targets', 'Varış', 'Her varış nesnesinden bir yol: hücrelerinden birikimli maliyeti en küçük olanından.'),
    ...NETWORK,
    {
      name: 'simplify',
      label: 'Sadeleştirme',
      type: 'number',
      default: 0,
      min: 0,
      max: 1000,
      unit: '',
      description: 'Douglas–Peucker toleransı, hücre cinsinden; 0 yalnız doğrultudaki köşeleri atar.',
    },
    {
      name: 'layer',
      label: 'Çıktı katmanı',
      type: 'layer',
      default: { newName: 'En düşük maliyetli yol' },
      newLayerStyle: outputStyle('#E5484D'),
      above: 'input',
      description: ABOVE_COST,
    },
  ] as const,
  outputs: [
    { name: 'objects', label: 'Yollar', type: 'features' },
    { name: 'count', label: 'Nesne sayısı', type: 'number' },
  ] as const,
  run: (v, ctx, feedback) => {
    const sources = v.sources?.entities ?? [];
    const tool = { kind: 'costPath', band: v.band ?? 1, ...networkOf(v), simplify: v.simplify ?? 0, first: sources.length };
    return runDistancePaths(v, ctx, feedback, tool, [...sources, ...(v.targets?.entities ?? [])]);
  },
});

export const costCorridor = defineTool({
  id: 'distance.corridor',
  label: 'Maliyet koridoru',
  category: DISTANCE,
  icon: 'costCorridor',
  description: 'İki uçtan birikimli maliyetlerin toplamını yazar: en küçük toplam en ucuz yolun maliyetidir, koridor onun çevresidir.',
  help: [
    "Koridor K = A'dan birikimli maliyet + B'den birikimli maliyettir; ikisinden biri tanımlı değilse hücre değersizdir. En küçük K, en ucuz A–B yolunun maliyetidir ve yol bu hücrelerden geçer. Eşik: yok, en küçük toplamın yüzdesi (K ≤ en küçük · (1 + yüzde/100)) ya da bir birikimli maliyet; eşiği aşan hücre değersizdir (ArcGIS'in Least Cost Corridor'ı).",
    HELP_COST,
    HELP_STEP,
    HELP_SURFACE,
    'İki uçtan aramalar iki iş parçacığında çalışır. Sonuç Viridis; Çizime ekle açıksa maliyet rasterinin katmanının hemen üstüne eklenir.',
    HELP_SIZE,
  ].join('\n\n'),
  keywords: ['koridor', 'corridor', 'güzergâh', 'least cost corridor', 'maliyet', 'bant'],
  aliases: ['KORIDOR', 'CORRIDOR'],
  targets: ['client'],
  parameters: [
    ...COST_RASTER,
    objects('sources', 'Birinci uçlar', 'Koridorun bir ucu: noktalar, çizgiler ya da alanlar.'),
    objects('targets', 'İkinci uçlar', 'Koridorun öbür ucu.'),
    ...NETWORK,
    {
      name: 'threshold',
      label: 'Eşik',
      type: 'enum',
      options: [
        { value: 'none', label: 'Yok' },
        { value: 'percent', label: 'En küçük toplamın yüzdesi' },
        { value: 'value', label: 'Birikimli maliyet' },
      ],
      default: 'none',
      description: 'Eşiği aşan hücre değersiz: koridor en ucuz yolun çevresinde kalır.',
    },
    {
      name: 'percent',
      label: 'Yüzde',
      type: 'number',
      default: 10,
      min: 0,
      max: 1e6,
      unit: '',
      description: 'En küçük toplamın bu kadar yüzde fazlasına kadar.',
      visibleWhen: (v: Shown) => v.threshold === 'percent',
    },
    {
      name: 'value',
      label: 'Eşik değeri',
      type: 'number',
      default: 0,
      min: 0,
      max: 1e300,
      unit: '',
      description: 'Toplamı bundan büyük olmayan hücreler.',
      visibleWhen: (v: Shown) => v.threshold === 'value',
    },
    SAMPLE,
    output('-koridor'),
    ADD,
    { ...layer('Maliyet koridoru'), description: ABOVE_COST },
  ] as const,
  outputs: FILE,
  run: (v, ctx, feedback) => {
    const sources = v.sources?.entities ?? [];
    const threshold = v.threshold === 'percent' ? 'percent' : v.threshold === 'value' ? 'value' : 'none';
    const value = threshold === 'percent' ? (v.percent ?? 10) : threshold === 'value' ? (v.value ?? 0) : 0;
    const tool = {
      kind: 'costCorridor',
      band: v.band ?? 1,
      ...networkOf(v),
      first: sources.length,
      threshold,
      value,
      sample: v.sample === 'f64' ? 'f64' : 'f32',
    };
    const shapes = [...sources, ...(v.targets?.entities ?? [])];
    return runDistanceRaster(v, ctx, feedback, tool, shapes, 'uç nesnesi', '-koridor', 'Maliyet koridoru hesaplanıyor', (d) => {
      const least = least3(d.least);
      if (d.cells === 0) return ` En ucuz yolun maliyeti ${least}; eşiğin içinde hücre yok.`;
      return ` En ucuz yolun maliyeti ${least}; koridorda ${count(d.cells)} hücre.`;
    });
  },
});

export const DISTANCE_TOOLS = [distanceSurface, costDistance, costPath, costCorridor] as const;
