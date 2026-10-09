import { defineTool, type Shown } from '../../types';
import {
  BAND,
  HELP_EMPTY,
  HELP_OUTPUT,
  IGNORE,
  OPS,
  STAT,
  STAT_COUNT,
  STATS,
  areas,
  ends,
  oneRaster,
  rasters,
  runHistogram,
  runRasterOp,
  runZonal,
} from './shared';

/**
 * Raster işlemleri and Raster istatistiği (docs/adr/0233 §3–§14): the nine tools, their parameters as the desktop's
 * (`builtin/raster_ops/tools.rs`), their settings handed to the raster core's operation job.
 */

const FILE = [{ name: 'file', label: 'Sonuç dosyası', type: 'string' }] as const;

export const rasterCalculator = defineTool({
  id: 'raster.calculator',
  label: 'Raster hesaplayıcı',
  category: OPS,
  icon: 'rasterCalculator',
  description: 'Rasterlerin hücrelerinden ifadeyle yeni bir raster yazar: aritmetik, koşul ve matematik işlevleri.',
  help: [
    'Örnekler: ([Ortofoto@4] - [Ortofoto@1]) / ([Ortofoto@4] + [Ortofoto@1]) (NDVI); durum eğer [Eğim] > 15 ise 1 yoksa 0 son; [DEM] - [DEM (2)].',
    'Sonuç, ifadenin ilk andığı rasterin ızgarasında, adıyla ve katmanının hemen üstündedir; yalnız ifadenin andığı rasterler açılır, öbürleri hücre merkezlerinde en yakın hücreleriyle okunur. Koşul doğruysa 1, yanlışsa 0; boş ve sayı olmayan değer değersizdir.',
    'Matematik işlevleri: ln, log10, log, üstel, sin, cos, tan, asin, acos, atan, atan2, derece, radyan, kök, mutlak, yuvarla, min, max.',
    HELP_EMPTY,
    HELP_OUTPUT,
  ].join('\n\n'),
  keywords: ['hesap', 'cebir', 'harita cebiri', 'raster calculator', 'map algebra', 'band math', 'ndvi', 'ifade'],
  aliases: ['RASTERHESAP', 'HARITACEBIRI', 'BANDARITMETIGI'],
  targets: ['client'],
  parameters: [
    rasters('İfadenin andığı rasterler; her biri katmanının adıyla anılır.', 'visible'),
    {
      name: 'expression',
      label: 'İfade',
      type: 'expression',
      returns: 'value',
      of: 'input',
      placeholder: '[DEM] * 2',
      description: 'Rasterler katmanlarının adıyla, bant @ ile: [Ortofoto@3]; [DEM] 1. bant. $y ve $x hücrenin merkezi, $alan hücrenin alanı.',
    },
    {
      name: 'empty',
      label: 'Değeri olmayan hücreler',
      type: 'enum',
      options: [
        { value: 'propagate', label: 'Değersiz kalır' },
        { value: 'expression', label: 'İfade karar verir' },
      ],
      default: 'propagate',
      description: 'İfade karar verirse değersiz hücre boş olarak görülür: aritmetik boş verir, boş ve eğer onu karşılar.',
    },
    {
      name: 'sample',
      label: 'Sonuç türü',
      type: 'enum',
      options: [
        { value: 'f32', label: 'Ondalık 32 bit' },
        { value: 'f64', label: 'Ondalık 64 bit' },
      ],
      default: 'f32',
    },
    ...ends('-hesap', 'Hesap'),
  ] as const,
  outputs: FILE,
  run: (v, ctx, feedback) =>
    runRasterOp(v, ctx, feedback, { kind: 'calculator', expression: v.expression?.source ?? '', empty: v.empty, sample: v.sample }, '-hesap', 'Raster hesaplanıyor', false, null),
});

export const rasterReclassify = defineTool({
  id: 'raster.reclassify',
  label: 'Yeniden sınıflandır',
  category: OPS,
  icon: 'reclassify',
  description: 'Rasterin değerlerini bir tabloyla yeni değerlere çevirir: aralıklar, tek değerler, değersiz hücreler.',
  help: ['Kurallar tablonun sırasıyla denenir; ilk tutan yeni değeri verir. Örnek: “* 5 1; 5 15 2; 15 * 3” eğimi üç sınıfa ayırır.', HELP_EMPTY, HELP_OUTPUT].join('\n\n'),
  keywords: ['sınıflandır', 'reclassify', 'tablo', 'sınıf', 'aralık'],
  aliases: ['YENIDENSINIFLA', 'RECLASS'],
  targets: ['client'],
  parameters: [
    oneRaster('Sınıflandırılacak raster.'),
    BAND,
    {
      name: 'table',
      label: 'Tablo',
      type: 'string',
      default: '',
      placeholder: '0 100 1; 100 200 2; 200 * 3',
      maxLength: 20000,
      description: 'Satır başına bir kural (satırlar ; ile de): “alt üst yeni”, “değer yeni” ya da “boş yeni”; açık uç *; yeni değer boş olabilir.',
    },
    {
      name: 'bounds',
      label: 'Sınırlar',
      type: 'enum',
      options: [
        { value: 'upperClosed', label: 'alt < değer ≤ üst' },
        { value: 'lowerClosed', label: 'alt ≤ değer < üst' },
      ],
      default: 'upperClosed',
    },
    {
      name: 'unmatched',
      label: 'Tabloda olmayanlar',
      type: 'enum',
      options: [
        { value: 'keep', label: 'Olduğu gibi kalır' },
        { value: 'empty', label: 'Değersiz olur' },
      ],
      default: 'keep',
    },
    {
      name: 'sample',
      label: 'Sonuç türü',
      type: 'enum',
      options: [
        { value: 'f32', label: 'Ondalık 32 bit' },
        { value: 'i32', label: 'Tam sayı 32 bit' },
        { value: 'u8', label: 'Bayt (0–254)' },
      ],
      default: 'f32',
      description: 'Tam sayıda değerler yarımlar sıfırdan uzağa yuvarlanır; değersiz −2 147 483 648, baytta 255.',
    },
    ...ends('-sinif', 'Sınıflar'),
  ] as const,
  outputs: FILE,
  run: (v, ctx, feedback) =>
    runRasterOp(
      v,
      ctx,
      feedback,
      { kind: 'reclassify', band: v.band ?? 1, table: v.table ?? '', bounds: v.bounds, unmatched: v.unmatched, sample: v.sample },
      '-sinif',
      'Sınıflandırılıyor',
      true,
      null,
    ),
});

export const rasterClipByMask = defineTool({
  id: 'raster.clipByMask',
  label: 'Maskeyle kırp',
  category: OPS,
  icon: 'clipRaster',
  description: 'Rasterin maskenin içindeki hücrelerini tutar, dışarıdakileri değersiz yapar; isterseniz maskenin kutusuna kırpar.',
  help: [
    'Hücre, merkezi bir maske nesnesinin içindeyse tutulur (nesnelerin birleşimi; delik dışarıdır). Sınırın üstündeki merkez yarı açık kuralla karar verir: sol ve alt kenar içeride.',
    "Hücreler yeniden örneklenmez: sonuç kaynağın hücreleridir. Kaynağın nodata'sı yoksa dışarıdaki hücreler ondalıkta NaN, RGB'de alfa 0 olur.",
    HELP_OUTPUT,
  ].join('\n\n'),
  keywords: ['kırp', 'maske', 'clip', 'mask', 'extract by mask', 'kes'],
  aliases: ['MASKEKIRP', 'RASTERKIRP'],
  targets: ['client'],
  parameters: [
    oneRaster('Kırpılacak raster (bütün bantları).'),
    areas('mask', 'Maske', false, 'Kapalı alanlar: kapalı alan, daire, elips, kapalı eğri, tarama; delikleri dışarıdır.'),
    {
      name: 'crop',
      label: 'Maskenin kutusuna kırp',
      type: 'boolean',
      default: true,
      description: 'Açıkken sonuç maskenin içindeki hücrelerin satır ve sütunlarıdır; kapalıyken rasterin bütün ızgarası.',
    },
    ...ends('-kirpik', 'Kırpılmış'),
  ] as const,
  outputs: FILE,
  run: (v, ctx, feedback) => runRasterOp(v, ctx, feedback, { kind: 'clipByMask', crop: v.crop !== false }, '-kirpik', 'Kırpılıyor', true, 'mask'),
});

export const rasterMosaic = defineTool({
  id: 'raster.mosaic',
  label: 'Mozaik',
  category: OPS,
  icon: 'mosaic',
  description: 'Rasterleri tek rasterde birleştirir; çakışan hücrelerde üstteki, alttaki, ortalama, en küçük ya da en büyük değer.',
  help: [
    'Sonucun ızgarası en ince hücreli rasterin eksenleri ve hücre boyudur; kafesi bütün rasterleri kapsayacak kadar büyür. Öbür rasterler sonucun hücre merkezlerinde okunur.',
    "Rasterlerin bant sayısı ve türü aynı olmalıdır; nodata'sı olmayan RGB'de sonuç alfa bandı alır.",
    HELP_OUTPUT,
  ].join('\n\n'),
  keywords: ['mozaik', 'mosaic', 'birleştir', 'merge', 'pafta'],
  aliases: ['MOZAIK'],
  targets: ['client'],
  parameters: [
    rasters('Birleştirilecek rasterler: aynı bant sayısı ve türde.', 'selection'),
    {
      name: 'overlap',
      label: 'Çakışanlar',
      type: 'enum',
      options: [
        { value: 'top', label: 'Üstteki' },
        { value: 'bottom', label: 'Alttaki' },
        { value: 'mean', label: 'Ortalama' },
        { value: 'min', label: 'En küçük' },
        { value: 'max', label: 'En büyük' },
      ],
      default: 'top',
      description: 'Üstteki: çizimde üstte görünen rasterin değeri (Katmanlar panelinde üstteki katman; aynı katmanda sonra eklenen).',
    },
    {
      name: 'sampling',
      label: 'Örnekleme',
      type: 'enum',
      options: [
        { value: 'nearest', label: 'En yakın' },
        { value: 'bilinear', label: 'Çift doğrusal' },
        { value: 'cubic', label: 'Kübik' },
      ],
      default: 'nearest',
    },
    ...ends('-mozaik', 'Mozaik'),
  ] as const,
  outputs: FILE,
  run: (v, ctx, feedback) => runRasterOp(v, ctx, feedback, { kind: 'mosaic', overlap: v.overlap, sampling: v.sampling }, '-mozaik', 'Birleştiriliyor', false, null),
});

export const rasterResample = defineTool({
  id: 'raster.resample',
  label: 'Yeniden örnekle',
  category: OPS,
  icon: 'resample',
  description: 'Rasteri yeni bir hücre boyuyla yeniden örnekler: en yakın, çift doğrusal, kübik ya da örtüşen hücrelerin ortalaması, çoğunluğu, en küçüğü, en büyüğü.',
  help: [
    "Sonucun sol üst köşesi ve eksenleri rasterinkidir; rasteri bütünüyle kapsar. Kübik, Keys'in evrişimidir (a = −½, GDAL'ın cubic'i).",
    'Değeri olmayan komşular dışarıda kalır, kalanların ağırlıkları toplamlarına bölünür. Tam sayılı rasterde değerler yarımlar sıfırdan uzağa yuvarlanır.',
    HELP_OUTPUT,
  ].join('\n\n'),
  keywords: ['örnekle', 'resample', 'hücre boyu', 'çözünürlük', 'warp', 'aggregate'],
  aliases: ['YENIDENORNEKLE', 'RESAMPLE'],
  targets: ['client'],
  parameters: [
    oneRaster('Yeniden örneklenecek raster (bütün bantları).'),
    { name: 'cell', label: 'Hücre boyu', type: 'number', default: 0, min: 0, max: 1e6, unit: 'm', description: 'Yeni hücrenin kenarı; rasterin eksenleri boyunca.' },
    {
      name: 'method',
      label: 'Yöntem',
      type: 'enum',
      options: [
        { value: 'nearest', label: 'En yakın' },
        { value: 'bilinear', label: 'Çift doğrusal' },
        { value: 'cubic', label: 'Kübik' },
        { value: 'mean', label: 'Ortalama' },
        { value: 'mode', label: 'Çoğunluk' },
        { value: 'min', label: 'En küçük' },
        { value: 'max', label: 'En büyük' },
      ],
      default: 'nearest',
      description: 'En yakın, Çift doğrusal ve Kübik yeni hücrenin merkezinde okur; Ortalama, Çoğunluk, En küçük ve En büyük örtüştüğü hücrelerden (büyütürken).',
    },
    ...ends('-ornek', 'Örneklenmiş'),
  ] as const,
  outputs: FILE,
  run: (v, ctx, feedback) => {
    if (!((v.cell ?? 0) > 0)) return Promise.resolve({ refused: "Hücre boyunu yazın (0'dan büyük)." });
    return runRasterOp(v, ctx, feedback, { kind: 'resample', cell: v.cell, method: v.method }, '-ornek', 'Yeniden örnekleniyor', true, null);
  },
});

export const rasterZonalStatistics = defineTool({
  id: 'raster.zonalStatistics',
  label: 'Bölgesel istatistik',
  category: STATS,
  icon: 'zonalStats',
  description: 'Her bölgenin (kapalı alanın) içindeki hücrelerin istatistiğini alanına yazar ve tablo olarak verir.',
  help: [
    'Örnekler: parsellerin ortalama kotu, eğim rasterinden ada başına en büyük eğim, arazi örtüsü rasterinden mahalle başına çoğunluk sınıfı.',
    'Bölgenin hücreleri merkezi alanın içinde olanlardır; örtüşen bölgeler aynı hücreyi ikisi de sayar. Değeri olmayan hücre sayılmaz; hiç hücresi olmayan bölgeye Sayı 0 yazılır, öbürleri alanı boşaltır.',
    'Toplam ve ortalama çift-çift toplamla, bir kez yuvarlanarak; standart sapma örneklemindir (n − 1).',
  ].join('\n\n'),
  keywords: ['bölgesel', 'zonal', 'istatistik', 'parsel', 'ortalama kot', 'zonal statistics'],
  aliases: ['BOLGESELISTATISTIK', 'ZONAL'],
  targets: ['client'],
  parameters: [
    oneRaster('Değerleri okunan raster.'),
    BAND,
    areas('zones', 'Bölgeler', true, 'Kapalı alanlar; istatistik bunların alanına yazılır, kilitli katmandakiler alınmaz.'),
    {
      name: 'stat',
      label: 'İstatistik',
      type: 'enum',
      options: [
        { value: 'mean', label: 'Ortalama' },
        { value: 'count', label: 'Sayı' },
        { value: 'sum', label: 'Toplam' },
        { value: 'min', label: 'En küçük' },
        { value: 'max', label: 'En büyük' },
        { value: 'range', label: 'Aralık' },
        { value: 'std', label: 'Standart sapma' },
        { value: 'median', label: 'Ortanca' },
        { value: 'majority', label: 'Çoğunluk' },
        { value: 'minority', label: 'Azınlık' },
        { value: 'variety', label: 'Çeşit' },
        { value: 'area', label: 'Alan' },
      ],
      default: 'mean',
      description: 'Alan: değeri olan hücrelerin alanlarının toplamı. Tabloda her bölgenin sayısı, toplamı, ortalaması, en küçüğü, en büyüğü ve standart sapması da var.',
    },
    {
      name: 'output',
      label: 'Yazılacak alan',
      type: 'field',
      of: 'zones',
      allowNew: true,
      optional: true,
      default: 'Ortalama',
      description: 'Listeden var olan bir alanı seçin ya da yeni bir ad yazın; boşsa yalnız tablo.',
    },
    { name: 'decimals', label: 'Basamak', type: 'number', default: 3, min: 0, max: 12, integer: true, unit: '', description: 'Yazılan değerin ondalık basamağı (tipli ondalık alanda alanın kendi basamağı).' },
  ] as const,
  outputs: [
    { name: 'table', label: 'Bölgelerin istatistikleri', type: 'table' },
    { name: 'changed', label: 'Değişen nesneler', type: 'features' },
    { name: 'count', label: 'Yazılan nesne sayısı', type: 'number' },
  ],
  run: (v, ctx, feedback) => runZonal(v, ctx, feedback),
});

export const rasterHistogram = defineTool({
  id: 'raster.histogram',
  label: 'Histogram',
  category: STATS,
  icon: 'histogram',
  description: 'Bandın değerlerini eşit aralıklara bölüp her aralıktaki hücre sayısını tablo olarak verir.',
  help: [
    'Aralık k = ⌊(x − alt) / (üst − alt) · n⌋ (son aralık üst sınırı da alır). Sınırların dışındaki değerler ayrıca sayılır.',
    'Tablo panoya kopyalanır ya da CSV olarak kaydedilir; çizim değişmez.',
  ].join('\n\n'),
  keywords: ['histogram', 'dağılım', 'frekans', 'aralık'],
  aliases: ['HISTOGRAM'],
  targets: ['client'],
  parameters: [
    oneRaster('Histogramı çıkarılacak raster.'),
    BAND,
    { name: 'bins', label: 'Aralık sayısı', type: 'number', default: 20, min: 1, max: 1000, integer: true, unit: '' },
    {
      name: 'min',
      label: 'En küçük',
      type: 'number',
      optional: true,
      default: null,
      unit: '',
      placeholder: 'Bandın',
      description: 'Boşsa bandın en küçük ve en büyük değeri; verilirse ikisi birlikte.',
    },
    { name: 'max', label: 'En büyük', type: 'number', optional: true, default: null, unit: '', placeholder: 'Bandın' },
  ] as const,
  outputs: [{ name: 'table', label: 'Histogram', type: 'table' }],
  run: (v, ctx, feedback) => runHistogram(v, ctx, feedback),
});

const rect = (v: Shown) => (v.shape ?? 'rect') === 'rect';
const round = (v: Shown) => (v.shape ?? 'rect') !== 'rect';
const ring = (v: Shown) => v.shape === 'ring';

export const rasterFocalStatistics = defineTool({
  id: 'raster.focalStatistics',
  label: 'Komşuluk istatistiği',
  category: STATS,
  icon: 'focalStats',
  description: 'Her hücreye çevresindeki pencerenin istatistiğini yazar: ortalama, toplam, en küçük, en büyük, aralık, standart sapma, ortanca, çoğunluk, azınlık, çeşit.',
  help: [
    'Pencere dikdörtgen (tek kenarlı), daire (merkezi r hücre içinde olanlar) ya da halkadır (iç ve dış yarıçap arası). Rasterin dışı pencerede sayılmaz.',
    "Değersizleri yok say açıkken değersiz merkez de çevresinde değer varsa değer alır (ArcGIS'in varsayılanı).",
    HELP_OUTPUT,
  ].join('\n\n'),
  keywords: ['komşuluk', 'odak', 'focal', 'filtre', 'pencere', 'neighborhood', 'yumuşat'],
  aliases: ['KOMSULUK', 'ODAKISTATISTIK', 'FOCAL'],
  targets: ['client'],
  parameters: [
    oneRaster('Değerleri okunan raster.'),
    BAND,
    {
      name: 'shape',
      label: 'Komşuluk',
      type: 'enum',
      options: [
        { value: 'rect', label: 'Dikdörtgen' },
        { value: 'circle', label: 'Daire' },
        { value: 'ring', label: 'Halka' },
      ],
      default: 'rect',
    },
    { name: 'width', label: 'Genişlik', type: 'number', default: 3, min: 1, max: 255, integer: true, unit: '', description: 'Hücre; tek sayı.', visibleWhen: rect },
    { name: 'height', label: 'Yükseklik', type: 'number', default: 3, min: 1, max: 255, integer: true, unit: '', description: 'Hücre; tek sayı.', visibleWhen: rect },
    {
      name: 'radius',
      label: 'Yarıçap',
      type: 'number',
      default: 3,
      min: 1,
      max: 127,
      integer: true,
      unit: '',
      description: 'Hücre: merkezi bu uzaklıkta olan hücreler (halkada dış yarıçap).',
      visibleWhen: round,
    },
    { name: 'inner', label: 'İç yarıçap', type: 'number', default: 1, min: 1, max: 126, integer: true, unit: '', description: 'Hücre; dış yarıçaptan küçük.', visibleWhen: ring },
    STAT,
    IGNORE,
    ...ends('-komsuluk', 'Komşuluk'),
  ] as const,
  outputs: FILE,
  run: (v, ctx, feedback) =>
    runRasterOp(
      v,
      ctx,
      feedback,
      {
        kind: 'focalStatistics',
        band: v.band ?? 1,
        shape: v.shape,
        width: v.width ?? 3,
        height: v.height ?? 3,
        radius: v.radius ?? 3,
        inner: v.inner ?? 1,
        stat: v.stat,
        ignore: v.ignore !== false,
      },
      '-komsuluk',
      'Komşuluk hesaplanıyor',
      true,
      null,
    ),
});

export const rasterCellStatistics = defineTool({
  id: 'raster.cellStatistics',
  label: 'Hücre istatistiği',
  category: STATS,
  icon: 'cellStats',
  description: 'Rasterlerin her hücredeki değerlerinin istatistiğini yeni bir rastere yazar: yılların ortalaması, en büyüğü, sayısı.',
  help: [
    "Sonucun ızgarası Mozaik'inki gibidir: en ince hücreli rasterinki, bütün rasterleri kapsar; rasterler hücre merkezlerinde en yakın hücreleriyle okunur.",
    HELP_EMPTY,
    HELP_OUTPUT,
  ].join('\n\n'),
  keywords: ['hücre', 'yığın', 'cell statistics', 'ortalama', 'zaman serisi'],
  aliases: ['HUCREISTATISTIK', 'CELLSTATS'],
  targets: ['client'],
  parameters: [rasters('İstatistiği alınacak rasterler (en az iki).', 'selection'), BAND, STAT_COUNT, IGNORE, ...ends('-hucre', 'Hücre istatistiği')] as const,
  outputs: FILE,
  run: (v, ctx, feedback) =>
    runRasterOp(v, ctx, feedback, { kind: 'cellStatistics', band: v.band ?? 1, stat: v.stat, ignore: v.ignore !== false }, '-hucre', 'Hücre istatistiği hesaplanıyor', false, null),
});

export const RASTER_OPS_TOOLS = [
  rasterCalculator,
  rasterReclassify,
  rasterClipByMask,
  rasterMosaic,
  rasterResample,
  rasterZonalStatistics,
  rasterHistogram,
  rasterFocalStatistics,
  rasterCellStatistics,
];
