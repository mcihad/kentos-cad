import { defineTool, type Shown } from '../../types';
import { BAND, ends } from '../rasterOps/shared';
import { count } from '../surface/shared';
import { HYDROLOGY, objectsLayer, runHydroObjects, runHydroRaster, trimmed, warnSkipped } from './shared';

/**
 * Hidroloji (docs/adr/0235 §3–§11): the eight tools, their parameters as the desktop's (`builtin/hydrology/tools.rs`);
 * each runs the raster core's operation job over the DEM.
 */

const HELP_EMPTY =
  "Rasterin nodata'sı, NaN ve alfası 0 olan pikseller değersizdir; kenardaki ve değersiz hücreye komşu hücreler çıkıştır: su rasterin dışına ya da değersiz hücreye akabilir.";
const HELP_FILL =
  "Çukurları doldur açıkken önce çukurlar taşma yüksekliğine doldurulur (en küçük eğim 0); düzlüklerde yön Barnes ve ark.'nın (2014) yöntemiyle verilir: alçak kenara yaklaşan ve yüksek kenardan uzaklaşan iki gradyan.";
const HELP_D8 = 'D8: hücre, eğimi (yükseklik farkı / merkezler arası uzaklık) en büyük alçak komşusuna akar; eşitlerde doğudan saat yönünde ilki.';
const HELP_SIZE = 'Bütün raster bellekte çalışılır: en çok 2²⁵ hücre (5792 × 5792).';

const FILE = [{ name: 'file', label: 'Sonuç dosyası', type: 'string' }] as const;
const OBJECTS = (label: string) => [
  { name: 'objects', label, type: 'features' },
  { name: 'count', label: 'Nesne sayısı', type: 'number' },
] as const;

const DEM = { name: 'input', label: 'Yükseklik modeli', type: 'features', kinds: ['raster'], scopes: ['selection', 'layer'], description: 'Yükseklik modeli (DEM) rasteri.' } as const;
const FILL_FIRST = { name: 'fill', label: 'Çukurları doldur', type: 'boolean', default: true, description: 'Önce çukurlar doldurulur; kapalıysa çukurlar akışı durdurur.' } as const;

const METHODS = [
  { value: 'd8', label: 'D8' },
  { value: 'dinf', label: 'D∞ (Tarboton)' },
  { value: 'mfd', label: 'Çoklu yön (MFD)' },
] as const;
const method = <F extends 'd8' | 'mfd'>(first: F) =>
  ({
    name: 'method',
    label: 'Yöntem',
    type: 'enum',
    options: [...METHODS.filter((m) => m.value === first), ...METHODS.filter((m) => m.value !== first)],
    default: first,
  }) as const;
const EXPONENT = {
  name: 'exponent',
  label: 'Çoklu yönün üssü',
  type: 'number',
  default: 0,
  min: 0,
  max: 100,
  unit: '',
  description: "0: uyarlanan (Qin ve ark. 2007, ArcGIS Pro'nunki); 0,1–100: sabit üs (1: Quinn ve ark. 1991).",
  visibleWhen: (v: Shown) => v.method === 'mfd',
} as const;
const POINTS = {
  name: 'points',
  label: 'Noktalar',
  type: 'features',
  kinds: ['point'],
  scopes: ['layer', 'selection', 'visible', 'all'],
  description: 'Döküm noktaları: nokta nesneleri (çok noktalının her noktası).',
} as const;
const SNAP = {
  name: 'snap',
  label: 'Yaklaştırma uzaklığı',
  type: 'number',
  default: 0,
  min: 0,
  max: 1e6,
  unit: 'm',
  description: '0: noktanın hücresi; artı bir uzaklıkta bu uzaklıktaki hücrelerden D8 birikimi en büyük olanı.',
} as const;
const THRESHOLD = {
  name: 'threshold',
  label: 'Eşik alanı',
  type: 'number',
  default: 0,
  min: 0,
  max: 1e15,
  unit: 'm²',
  description: 'D8 birikimi (alan) bu değerden küçük olmayan hücreler deredir; 0: en büyük birikimin yüzde biri.',
} as const;

export const fillSinks = defineTool({
  id: 'hydrology.fill',
  label: 'Çukur doldur',
  category: HYDROLOGY,
  icon: 'fillSinks',
  description: 'Yükseklik modelinin çukurlarını taşma yüksekliğine doldurur: her hücreden rasterin dışına alçalan bir yol kalır.',
  help: [
    'Doldurulmuş yüzey f ≥ z olan ve her iç hücrenin f(n) + ε·d ≤ f(c) olan bir komşusu bulunan en küçük yüzeydir (ε en küçük eğim / 100). En küçük eğim 0\'da her hücre taşma yüksekliğine, kenara giden yolların en yüksek noktalarının en küçüğüne yükselir (Wang ve Liu 2006; Planchon ve Darboux 2002); artı eğimde çukurlar ve düzlükler bu eğimle çıkışa doğru alçalır.',
    'Hesap öncelik kuyruklu taşmadır (Barnes ve ark. 2014); birden çok iş parçacığında raster şeritlerde paralel doldurulur (Barnes 2016), sonuç aynıdır.',
    HELP_EMPTY,
    "Dolgu derinliği doldurulmuş yükseklik eksi DEM'dir. Sonuç 32 bit (DEM 64 bitse 64 bit) karolu GeoTIFF'tir; Çizime ekle açıksa DEM'in katmanının hemen üstüne eklenir.",
    HELP_SIZE,
  ].join('\n\n'),
  keywords: ['çukur', 'doldur', 'fill', 'sink', 'depression', 'wang liu', 'priority flood'],
  aliases: ['CUKURDOLDUR', 'FILLSINKS'],
  targets: ['client'],
  parameters: [
    DEM,
    BAND,
    {
      name: 'slope',
      label: 'En küçük eğim',
      type: 'number',
      default: 0,
      min: 0,
      max: 100,
      unit: '',
      description: 'Yüzde; 0 çukurları düz doldurur, artı değerde doldurulan yüzey çıkışa en az bu eğimle alçalır.',
    },
    {
      name: 'result',
      label: 'Sonuç',
      type: 'enum',
      options: [
        { value: 'filled', label: 'Doldurulmuş yükseklik' },
        { value: 'depth', label: 'Dolgu derinliği' },
      ],
      default: 'filled',
    },
    ...ends('-dolu', 'Doldurulmuş DEM'),
  ] as const,
  outputs: FILE,
  run: (v, ctx, feedback) => {
    const depth = v.result === 'depth';
    const tool = { kind: 'fill', band: v.band ?? 1, slope: v.slope ?? 0, result: depth ? 'depth' : 'filled' };
    return runHydroRaster(v, ctx, feedback, tool, '-dolu', 'Çukurlar dolduruluyor', (n) => {
      if (n.cells === 0) return ' Doldurulacak çukur yok.';
      let s = ` ${count(n.cells)} hücre dolduruldu.`;
      if (depth) s += ` En derin dolgu ${trimmed(n.most, 3)} m.`;
      return s;
    });
  },
});

export const flowDirection = defineTool({
  id: 'hydrology.flowDirection',
  label: 'Akış yönü',
  category: HYDROLOGY,
  icon: 'flowDirection',
  description: 'Her hücrenin suyunu verdiği komşuyu D8 kodu olarak yazar.',
  help: [
    HELP_D8,
    "ESRI kodları doğudan saat yönünde 1, 2, 4, 8, 16, 32, 64, 128 (güneydoğu 2, kuzey 64); 1–8 TauDEM'inki: doğudan saat yönünün tersine. Çıkış hücresinin alçak komşusu yoksa dışarı akar. Yönsüz hücre 0, değersiz 255.",
    HELP_FILL,
    HELP_EMPTY,
    HELP_SIZE,
  ].join('\n\n'),
  keywords: ['akış yönü', 'flow direction', 'd8', 'drenaj', 'yön'],
  aliases: ['AKISYONU', 'FLOWDIR'],
  targets: ['client'],
  parameters: [
    DEM,
    BAND,
    FILL_FIRST,
    {
      name: 'coding',
      label: 'Kodlama',
      type: 'enum',
      options: [
        { value: 'esri', label: 'ESRI (1–128)' },
        { value: 'taudem', label: '1–8 (TauDEM)' },
      ],
      default: 'esri',
    },
    ...ends('-yon', 'Akış yönü'),
  ] as const,
  outputs: FILE,
  run: (v, ctx, feedback) => {
    const tool = { kind: 'flowDirection', band: v.band ?? 1, fill: v.fill !== false, coding: v.coding === 'taudem' ? 'taudem' : 'esri' };
    return runHydroRaster(v, ctx, feedback, tool, '-yon', 'Akış yönleri bulunuyor', (n) => {
      let s = '';
      if (n.cells > 0) s += ` ${count(n.cells)} düzlük hücresinin yönü verildi.`;
      if (n.empty > 0) s += ` ${count(n.empty)} hücre yönsüz kaldı: çukurlar ve çıkışsız düzlükler (Çukurları doldur açıkken kalmaz).`;
      return s;
    });
  },
});

export const flowAccumulation = defineTool({
  id: 'hydrology.flowAccumulation',
  label: 'Akış birikimi',
  category: HYDROLOGY,
  icon: 'flowAccumulation',
  description: 'Her hücreden geçen suyun geldiği alanı yazar: hücre sayısı, alan ya da özgül havza alanı.',
  help: [
    "Birikim hücrenin kendisi ve yukarısındaki hücrelerin paylarıdır (hücre kendini sayar; ArcGIS'in Flow Accumulation'ı saymaz, onunki bir eksiktir). Özgül havza alanı alan / hücrenin genişliğidir.",
    "D8: bütün pay yönüne. D∞ (Tarboton 1997): sekiz üçgen yüzün en dik alçalanı, pay iki komşu arasında açıyla. Çoklu yön: pay L·tᵖ ile orantılı (t eğim, L Quinn ve ark.'nın kontur uzunluğu: dikte 0,5, çaprazda 0,354); üs sabit ya da uyarlanan p = 8,9·min(e, 1) + 1,1 (Qin ve ark. 2007, e hücrenin en dik eğimi). Alçak komşusu olmayan hücrenin payı D8 yönüne.",
    HELP_FILL,
    HELP_EMPTY,
    HELP_SIZE,
  ].join('\n\n'),
  keywords: ['birikim', 'flow accumulation', 'akış', 'havza alanı', 'sca', 'd8', 'mfd', 'd-infinity'],
  aliases: ['AKISBIRIKIMI', 'FLOWACC'],
  targets: ['client'],
  parameters: [
    DEM,
    BAND,
    FILL_FIRST,
    method('d8'),
    EXPONENT,
    {
      name: 'unit',
      label: 'Birim',
      type: 'enum',
      options: [
        { value: 'cells', label: 'Hücre sayısı' },
        { value: 'area', label: 'Alan (m²)' },
        { value: 'sca', label: 'Özgül havza alanı (m)' },
      ],
      default: 'cells',
    },
    ...ends('-birikim', 'Akış birikimi'),
  ] as const,
  outputs: FILE,
  run: (v, ctx, feedback) => {
    const unit = v.unit === 'area' ? 'area' : v.unit === 'sca' ? 'sca' : 'cells';
    const tool = { kind: 'flowAccumulation', band: v.band ?? 1, fill: v.fill !== false, method: v.method ?? 'd8', exponent: v.exponent ?? 0, unit };
    const name = unit === 'area' ? 'm²' : unit === 'sca' ? 'm' : 'hücre';
    return runHydroRaster(v, ctx, feedback, tool, '-birikim', 'Akış birikimi hesaplanıyor', (n) => ` En büyük birikim ${trimmed(n.most, 3)} ${name}.`);
  },
});

export const wetness = defineTool({
  id: 'hydrology.wetness',
  label: 'Topografik nemlilik indisi',
  category: HYDROLOGY,
  icon: 'wetness',
  description: 'Topografik nemlilik indisi TWI = ln(a / tan β): suyun toplandığı düz yerler yüksek, sırtlar düşük.',
  help: [
    "a özgül havza alanıdır (m; Akış birikimi'nin), tan β yüzeyin Horn yöntemiyle eğimi (ADR 0231); tan β en küçük eğimin altındaysa en küçük eğim alınır. Beven ve Kirkby'nin (1979) indisidir.",
    'Yöntem birikimin yöntemidir; Çoklu yön varsayılandır. Sonuç 32 bit; görünüş kuruda kırmızı, ıslakta mavi.',
    HELP_FILL,
    HELP_EMPTY,
    HELP_SIZE,
  ].join('\n\n'),
  keywords: ['twi', 'nemlilik', 'wetness', 'topographic wetness index', 'ıslak', 'beven kirkby'],
  aliases: ['NEMLILIK', 'TWI'],
  targets: ['client'],
  parameters: [
    DEM,
    BAND,
    FILL_FIRST,
    method('mfd'),
    EXPONENT,
    {
      name: 'slope',
      label: 'En küçük eğim',
      type: 'number',
      default: 0.1,
      min: 0.0001,
      max: 100,
      unit: '',
      description: "Yüzde; tan β bunun altındaysa bu kullanılır (CSIRO'nun DEM-H TWI'ında %0,1).",
    },
    ...ends('-twi', 'Nemlilik indisi'),
  ] as const,
  outputs: FILE,
  run: (v, ctx, feedback) => {
    const tool = { kind: 'wetness', band: v.band ?? 1, fill: v.fill !== false, method: v.method ?? 'mfd', exponent: v.exponent ?? 0, slope: v.slope ?? 0.1 };
    return runHydroRaster(v, ctx, feedback, tool, '-twi', 'Nemlilik indisi hesaplanıyor', (n) => (n.cells > 0 ? ` ${count(n.cells)} hücrede en küçük eğim kullanıldı.` : ''));
  },
});

export const pourPoint = defineTool({
  id: 'hydrology.pourPoint',
  label: 'Döküm noktası',
  category: HYDROLOGY,
  icon: 'pourPoint',
  description: 'Noktaları yakındaki en büyük akış birikimli hücreye taşır: havzanın çıkışı derenin üstüne gelir.',
  help: [
    "Yaklaştırma 0'da noktanın hücresi; artı bir uzaklıkta merkezi bu uzaklıkta olan hücrelerden D8 birikimi en büyük olanı (eşitse noktaya en yakın, o da eşitse satır satır ilki; ArcGIS'in Snap Pour Point'i).",
    'Her nokta hücresinin merkezine yazılır; öznitelikleri Nokta (girdideki sırası), Birikim (hücre), Alan (m²) ve Uzaklık (m). Rasterin dışındaki ya da değersiz hücreye düşen nokta atlanır.',
    HELP_FILL,
    HELP_SIZE,
  ].join('\n\n'),
  keywords: ['döküm noktası', 'pour point', 'snap', 'çıkış', 'outlet', 'havza'],
  aliases: ['DOKUMNOKTASI', 'SNAPPOUR'],
  targets: ['client'],
  parameters: [DEM, BAND, POINTS, FILL_FIRST, SNAP, objectsLayer('Döküm noktaları', '#E5484D')] as const,
  outputs: OBJECTS('Noktalar'),
  run: (v, ctx, feedback) => {
    const tool = { kind: 'pourPoint', band: v.band ?? 1, fill: v.fill !== false, snap: v.snap ?? 0 };
    return runHydroObjects(v, ctx, feedback, tool, v.points?.entities ?? [], 'Döküm noktaları bulunuyor', (f, n, fb) => {
      warnSkipped(n, fb);
      return `${count(f.values.length)} döküm noktası yazıldı.`;
    });
  },
});

export const watershed = defineTool({
  id: 'hydrology.watershed',
  label: 'Noktadan havza',
  category: HYDROLOGY,
  icon: 'watershed',
  description: 'Her döküm noktasının havzasını alan olarak yazar: suyu o noktadan geçen bütün hücreler.',
  help: [
    "Hücre D8 yolundaki ilk döküm noktasının havzasındadır: yukarıdaki nokta aşağıdakinin havzasından kendi havzasını ayırır (ArcGIS'in Watershed'i). İki nokta aynı hücreye düşerse hücre girdide sonra gelenindir; öbürünün havzası boş kalır, söylenir.",
    "Yaklaştırma Döküm noktası'nınkidir. Alanlar hücre kenarlarındandır (8 komşu); öznitelikleri Havza (noktanın girdideki sırası) ve Alan (m²).",
    HELP_FILL,
    HELP_SIZE,
  ].join('\n\n'),
  keywords: ['havza', 'watershed', 'catchment', 'su toplama', 'menfez', 'noktaya göre havza'],
  aliases: ['HAVZABUL', 'NOKTAHAVZA', 'WATERSHED'],
  targets: ['client'],
  parameters: [DEM, BAND, POINTS, FILL_FIRST, SNAP, objectsLayer('Havzalar', '#30A46C')] as const,
  outputs: OBJECTS('Havzalar'),
  run: (v, ctx, feedback) => {
    const tool = { kind: 'watershed', band: v.band ?? 1, fill: v.fill !== false, snap: v.snap ?? 0 };
    return runHydroObjects(v, ctx, feedback, tool, v.points?.entities ?? [], 'Havzalar bulunuyor', (f, n, fb) => {
      warnSkipped(n, fb);
      if (n.emptyPoints.length) fb.warn(`${count(n.emptyPoints.length)} noktanın hücresini sonraki bir nokta aldığı için havzası boş.`);
      return `${count(f.values.length)} havza yazıldı.`;
    });
  },
});

export const basins = defineTool({
  id: 'hydrology.basins',
  label: 'Havzalar',
  category: HYDROLOGY,
  icon: 'basins',
  description: 'Bütün havzaları, dere kollarının alt havzalarını ya da bir güzergâhı kesen derelerin havzalarını alan olarak yazar.',
  help: [
    "Ana havzalar: her hücre D8 yolunun bittiği hücreye göre (dışarı akan çıkış ya da yönsüz hücre); tek hücrelik havza da havzadır (ArcGIS'in Basin'i). Alt havzalar: Dere ağı'nın her kolunun kendi havzası (Bağ, Sıra). Güzergâhı kesen dereler: güzergâhın geçtiği dere hücrelerinden dere boyunca en aşağıdakiler geçiştir; her geçişin bütün havzası (menfez ve köprünün beslenme alanı, örtüşebilir), Km güzergâhtaki yeri, Sıra kolun Strahler sırası.",
    'En küçük alandan küçük havza yazılmaz, sayısı söylenir. Alanlar hücre kenarlarındandır.',
    HELP_FILL,
    HELP_SIZE,
  ].join('\n\n'),
  keywords: ['havzalar', 'basin', 'alt havza', 'subbasin', 'güzergâh', 'menfez', 'köprü', 'dere'],
  aliases: ['HAVZALAR', 'ALTHAVZA', 'BASINS'],
  targets: ['client'],
  parameters: [
    DEM,
    BAND,
    FILL_FIRST,
    {
      name: 'mode',
      label: 'Biçim',
      type: 'enum',
      options: [
        { value: 'main', label: 'Ana havzalar' },
        { value: 'sub', label: 'Alt havzalar' },
        { value: 'route', label: 'Güzergâhı kesen dereler' },
      ],
      default: 'main',
    },
    { ...THRESHOLD, visibleWhen: (v: Shown) => v.mode !== 'main' },
    {
      name: 'routes',
      label: 'Güzergâh',
      type: 'features',
      kinds: ['line', 'polyline', 'arc'],
      scopes: ['selection', 'layer'],
      optional: true,
      description: 'Yol ya da kanal ekseni: çizgi, çoklu çizgi ya da yay.',
      visibleWhen: (v: Shown) => v.mode === 'route',
    },
    { name: 'least', label: 'En küçük alan', type: 'number', default: 0, min: 0, max: 1e15, unit: 'm²', description: 'Bundan küçük havza yazılmaz; 0 hepsi.' },
    objectsLayer('Havzalar', '#30A46C'),
  ] as const,
  outputs: OBJECTS('Havzalar'),
  run: (v, ctx, feedback) => {
    const mode = v.mode === 'sub' ? 'sub' : v.mode === 'route' ? 'route' : 'main';
    const tool = { kind: 'basins', band: v.band ?? 1, fill: v.fill !== false, mode, threshold: v.threshold ?? 0, least: v.least ?? 0 };
    const shapes = mode === 'route' ? (v.routes?.entities ?? []) : [];
    return runHydroObjects(v, ctx, feedback, tool, shapes, 'Havzalar bulunuyor', (f, n) => {
      let s = `${count(f.values.length)} havza yazıldı.`;
      if (n.dropped > 0) s += ` En küçük alandan küçük ${count(n.dropped)} havza yazılmadı.`;
      return s;
    });
  },
});

export const streams = defineTool({
  id: 'hydrology.streams',
  label: 'Dere ağı',
  category: HYDROLOGY,
  icon: 'streams',
  description: 'Akış birikimi eşiği aşan hücrelerden dere ağını çoklu çizgi olarak yazar; her kolun Strahler ve Shreve sırasıyla.',
  help: [
    'Dere hücresi D8 birikimi (alan) eşikten küçük olmayan hücredir. Kol kaynakta ya da kavşakta başlar, bir sonraki kavşakta ya da yolun sonunda biter; kavşakta biten kol aktığı kolun ilk hücresine uzanır.',
    'Strahler: kaynak 1; aynı en büyük sıradan iki ya da daha çok kol birleşince bir artar. Shreve: gelenlerin toplamı. Öznitelikler: Bağ, Sıra, Shreve, Uzunluk (m), Düşü (m), Eğim (düşü / uzunluk), Alan (kolun sonundaki birikim, m²), Aşağı (aktığı kol).',
    'Sadeleştirme hücre cinsinden Douglas–Peucker toleransıdır; 0 yalnız doğrultudaki köşeleri atar.',
    HELP_FILL,
    HELP_SIZE,
  ].join('\n\n'),
  keywords: ['dere', 'stream', 'akarsu', 'drenaj ağı', 'strahler', 'shreve', 'kol', 'network'],
  aliases: ['DEREAGI', 'STREAMORDER'],
  targets: ['client'],
  parameters: [
    DEM,
    BAND,
    FILL_FIRST,
    THRESHOLD,
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
    objectsLayer('Dere ağı', '#0090FF'),
  ] as const,
  outputs: OBJECTS('Dereler'),
  run: (v, ctx, feedback) => {
    const tool = { kind: 'streams', band: v.band ?? 1, fill: v.fill !== false, threshold: v.threshold ?? 0, simplify: v.simplify ?? 0 };
    return runHydroObjects(v, ctx, feedback, tool, [], 'Dere ağı çıkarılıyor', (f, n) => {
      if (!f.values.length) return 'Eşiği aşan dere hücresi yok.';
      return `${count(f.values.length)} kol yazıldı; en büyük Strahler sırası ${trimmed(n.most, 0)}, eşik ${trimmed(n.threshold, 3)} m².`;
    });
  },
});

export const HYDROLOGY_TOOLS = [fillSinks, flowDirection, flowAccumulation, wetness, pourPoint, watershed, basins, streams] as const;
