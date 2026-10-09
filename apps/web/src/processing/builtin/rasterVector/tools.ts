import { defineTool, type Shown } from '../../types';
import { ADD, CELL, EXTENT, GRID, layer as belowLayer, output } from '../interpolation/shared';
import { BAND, oneRaster } from '../rasterOps/shared';
import { count } from '../surface/shared';
import { BURN_KINDS, RASTER_VECTOR, SCANNED, objectsOf, runContourElevations, runRasterize, runVector, vectorLayer } from './shared';

/**
 * Raster ve vektör and Taranmış harita (docs/adr/0234 §3–§10): the seven tools, their parameters as the desktop's
 * (`builtin/raster_vector/tools.rs`); the vectorizing ones run the raster core's operation job, Rasterleştir its point
 * job, Eğrilere kot ver the geometry core's ordering.
 */

const HELP_EMPTY = "Rasterin nodata'sı, NaN ve alfası 0 olan pikseller değersizdir.";
const HELP_PLACE = 'Köşeler hücre uzayında (hücre köşeleri ya da merkezleri) bulunur, rasterin yeriyle (afin) çizime geçer.';
const HELP_THIN =
  'Çizgi hücreleri Zhang–Suen inceltmesiyle (Lü–Wang düzeltmesi: iki piksel kalınlığındaki çapraz çizgi kalır) tek piksele indirilir; iskeletin kavşaktan kavşağa yolları birer çoklu çizgidir. Kısa parçaları at: ucu boşta, kavşağa bağlı ve bu kadar adımdan kısa dallar, sonra bütün kısa yollar atılır. Sadeleştirme hücre cinsinden Douglas–Peucker toleransıdır.';
const FEATURES = (label: string) => [
  { name: 'objects', label, type: 'features' },
  { name: 'count', label: 'Nesne sayısı', type: 'number' },
] as const;

export const rasterize = defineTool({
  id: 'raster.rasterize',
  label: 'Rasterleştir',
  category: RASTER_VECTOR,
  icon: 'rasterize',
  description: 'Nesneleri rasterin hücrelerine yakar: alanlar merkezlerini, çizgiler geçtikleri hücreleri, noktalar içinde oldukları hücreyi.',
  help: [
    'Kapalı şekil (kapalı alan, daire, tam elips, kapalı eğri, tarama) merkezi içinde olan hücreleri, açık şekil (çizgi, çoklu çizgi, yay, elips yayı, açık eğri) bir noktası yarı açık karesine düşen bütün hücreleri yakar; yaylar 0,1 mm kirişlerle izlenir. Nokta içinde olduğu hücreyi yakar. Bir nesne bir hücreyi bir kez sayar.',
    'Değer: Sabit değer ya da seçilen alandaki sayı (ondalık nokta ya da virgül); sayı olmayan nesne alınmaz, söylenir. Çakışanlar: girdinin sırasında sonraki (Son çizilen), önceki, en büyük, en küçük, toplam ya da nesne sayısı.',
    "Tam sayıda yarımlar sıfırdan uzağa yuvarlanır; türe sığmayan değer reddedilir (Tam sayı 32 bit'te −2 147 483 648, Bayt'ta 255 değersizdir). Yanmayan hücre değersizdir.",
    "Hücre boyu 0 ise nesnelerin kutusunun kısa kenarının 250'de biri, 1, 2, 2,5 ya da 5 × 10ᵏ'ye yuvarlanır; ızgara bu boyun katlarına oturur. Rasterin ızgarası seçilirse sonuç o rasterle hücre hücre üst üste gelir.",
    "Sonuç karolu, Deflate'li GeoTIFF'tir; Çizime ekle açıksa girdinin katmanının hemen altındaki yeni katmana eklenir (nesneler üstte kalır).",
  ].join('\n\n'),
  keywords: ['rasterleştir', 'rasterize', 'feature to raster', 'polygon to raster', 'yak', 'burn', 'maske'],
  aliases: ['RASTERIZE', 'VEKTORRASTER'],
  targets: ['client'],
  parameters: [
    { name: 'input', label: 'Nesneler', type: 'features', kinds: BURN_KINDS, scopes: ['layer', 'selection', 'visible', 'all'], description: 'Yakılacak alanlar, çizgiler ve noktalar.' },
    {
      name: 'valueFrom',
      label: 'Değer',
      type: 'enum',
      options: [
        { value: 'constant', label: 'Sabit' },
        { value: 'field', label: 'Alandan' },
      ],
      default: 'constant',
    },
    { name: 'value', label: 'Sabit değer', type: 'number', default: 1, min: -1e15, max: 1e15, unit: '', visibleWhen: (v: Shown) => v.valueFrom !== 'field' },
    {
      name: 'field',
      label: 'Değer alanı',
      type: 'field',
      of: 'input',
      optional: true,
      description: 'Nesnenin bu alandaki sayısı; sayı olmayan nesne alınmaz.',
      visibleWhen: (v: Shown) => v.valueFrom === 'field',
    },
    {
      name: 'overlap',
      label: 'Çakışanlar',
      type: 'enum',
      options: [
        { value: 'last', label: 'Son çizilen' },
        { value: 'first', label: 'İlk çizilen' },
        { value: 'max', label: 'En büyük' },
        { value: 'min', label: 'En küçük' },
        { value: 'sum', label: 'Toplam' },
        { value: 'count', label: 'Sayı' },
      ],
      default: 'last',
      description: 'Bir hücreyi birden çok nesne yakarsa kalan değer; Sayı nesne sayısıdır.',
    },
    {
      name: 'sample',
      label: 'Sonuç türü',
      type: 'enum',
      options: [
        { value: 'f32', label: 'Ondalık 32 bit' },
        { value: 'f64', label: 'Ondalık 64 bit' },
        { value: 'i32', label: 'Tam sayı 32 bit' },
        { value: 'u8', label: 'Bayt (0–254)' },
      ],
      default: 'f32',
    },
    CELL,
    EXTENT,
    GRID,
    output('-raster'),
    ADD,
    belowLayer('Rasterleştirilmiş'),
  ] as const,
  outputs: [{ name: 'file', label: 'Sonuç dosyası', type: 'string' }],
  run: (v, ctx, feedback) => runRasterize(v, ctx, feedback),
});

export const rasterToPolygons = defineTool({
  id: 'raster.toPolygons',
  label: 'Rasterden alan',
  category: RASTER_VECTOR,
  icon: 'toPolygons',
  description: 'Rasterin değeri eşit komşu hücrelerini alan yapar; her alanın Değer özniteliği bölgenin değeridir.',
  help: [
    'Bölge, değeri tam eşit ve komşu (4 komşu: kenarla; 8 komşu: kenar ya da köşeyle) hücrelerin bağlı kümesidir; değersiz hücre bölge değildir.',
    'Halkalar hücre kenarlarındandır: dış halka saat yönünün tersine, delikler saat yönünde; bölgeyi bir noktadan dokunarak saran boşluk o noktada dış halkaya değen delik olur. Alanlar bölgelerin ilk hücresinin sırasıyla.',
    "En çok 8192 × 8192 hücre, 1 000 000 alan: çok değerli raster için önce Yeniden sınıflandır'la sınıflara ayırın.",
    HELP_EMPTY,
    HELP_PLACE,
  ].join('\n\n'),
  keywords: ['poligon', 'polygonize', 'raster to polygon', 'sınıf', 'bölge', 'vektörleştir'],
  aliases: ['RASTERDENALAN', 'RASTERALAN'],
  targets: ['client'],
  parameters: [
    oneRaster('Bölgeleri alan yapılacak raster (genellikle sınıflandırılmış).'),
    BAND,
    {
      name: 'connect',
      label: 'Komşuluk',
      type: 'enum',
      options: [
        { value: 'four', label: '4 komşu (kenar)' },
        { value: 'eight', label: '8 komşu (kenar ve köşe)' },
      ],
      default: 'four',
    },
    vectorLayer('Bölgeler', '#3E63DD'),
  ] as const,
  outputs: FEATURES('Alanlar'),
  run: (v, ctx, feedback) =>
    runVector(v, ctx, feedback, { kind: 'toPolygons', band: v.band ?? 1, connect: v.connect }, 'Bölgeler alan yapılıyor', (f, layerId) => {
      const add = objectsOf(f, layerId, (k) => ({ Değer: f.texts[k] }));
      const values = new Set(f.texts).size;
      return { add, summary: add.length ? `${count(values)} değerden ${count(add.length)} alan yazıldı.` : 'Rasterde değeri olan hücre yok.' };
    }),
});

export const rasterToLines = defineTool({
  id: 'raster.toLines',
  label: 'Rasterden çizgi',
  category: RASTER_VECTOR,
  icon: 'toLines',
  description: 'Rasterdeki ince çizgileri inceltip çoklu çizgi yapar: sıfır dışındaki hücreler, bir değer aralığı ya da bir renk.',
  help: [
    "Çizgi hücreleri: 0 ve değersiz dışındakiler, en küçük ile en büyük arasındaki değerler ya da üç bantlı rasterde #RRGGBB'ye Renk toleransı içindeki renkler ((ΔR² + ΔG² + ΔB²) ≤ tolerans²).",
    HELP_THIN,
    'Kapalı halka çoklu çizgisi ilk noktasını sonda yineler. En çok 8192 × 8192 hücre.',
    HELP_EMPTY,
    HELP_PLACE,
  ].join('\n\n'),
  keywords: ['çizgi', 'polyline', 'raster to polyline', 'iskelet', 'skeleton', 'inceltme', 'vektörleştir'],
  aliases: ['RASTERDENCIZGI'],
  targets: ['client'],
  parameters: [
    oneRaster('Çizgileri alınacak raster.'),
    BAND,
    {
      name: 'select',
      label: 'Çizgi hücreleri',
      type: 'enum',
      options: [
        { value: 'nonZero', label: '0 ve değersiz dışındakiler' },
        { value: 'range', label: 'Değer aralığı' },
        { value: 'color', label: 'Renk' },
      ],
      default: 'nonZero',
    },
    { name: 'min', label: 'En küçük', type: 'number', default: 1, min: -1e15, max: 1e15, unit: '', visibleWhen: (v: Shown) => v.select === 'range' },
    { name: 'max', label: 'En büyük', type: 'number', default: 1, min: -1e15, max: 1e15, unit: '', visibleWhen: (v: Shown) => v.select === 'range' },
    { name: 'color', label: 'Renk', type: 'string', default: '#000000', placeholder: '#RRGGBB', visibleWhen: (v: Shown) => v.select === 'color' },
    { name: 'tolerance', label: 'Renk toleransı', type: 'number', default: 60, min: 0, max: 442, unit: '', visibleWhen: (v: Shown) => v.select === 'color' },
    { name: 'spur', label: 'Kısa parçaları at', type: 'number', default: 0, min: 0, max: 10000, integer: true, unit: '', description: 'Hücre cinsinden; 0 hiçbirini atmaz.' },
    { name: 'simplify', label: 'Sadeleştirme', type: 'number', default: 1, min: 0, max: 1000, unit: '', description: 'Douglas–Peucker toleransı, hücre cinsinden; 0 yalnız doğrultudaki köşeleri atar.' },
    vectorLayer('Çizgiler', '#E5484D'),
  ] as const,
  outputs: FEATURES('Çizgiler'),
  run: (v, ctx, feedback) =>
    runVector(
      v,
      ctx,
      feedback,
      { kind: 'toLines', band: v.band ?? 1, select: v.select, min: v.min ?? 0, max: v.max ?? 0, color: v.color ?? '', tolerance: v.tolerance ?? 60, spur: v.spur ?? 0, simplify: v.simplify ?? 1 },
      'Çizgiler çıkarılıyor',
      (f, layerId) => {
        const add = objectsOf(f, layerId, () => ({}));
        return { add, summary: add.length ? `${count(add.length)} çizgi yazıldı.` : 'Çizgi hücresi yok ya da hepsi kısa parça olarak atıldı.' };
      },
    ),
});

export const rasterToPoints = defineTool({
  id: 'raster.toPoints',
  label: 'Rasterden nokta',
  category: RASTER_VECTOR,
  icon: 'toPoints',
  description: 'Hücre merkezlerinde nokta yazar: adım adım, her hücrede ya da yalnız tepe ve çukurlarda; kotu hücrenin değeri.',
  help: [
    'Adımla: k hücrede bir (i mod k = ⌊k/2⌋ ve j mod k = ⌊k/2⌋ olan hücreler). Tepeler ve çukurlar: (2r + 1)² pencerede, değeri olan bütün komşularından kesin büyük olan tepe, kesin küçük olan çukur.',
    'Noktalar satır satır; öznitelikleri Değer (ve Tür: Tepe ya da Çukur). En çok 2 000 000 nokta.',
    HELP_EMPTY,
  ].join('\n\n'),
  keywords: ['nokta', 'point', 'raster to point', 'yükseklik noktaları', 'spot', 'tepe', 'çukur', 'dem'],
  aliases: ['RASTERDENNOKTA', 'YUKSEKLIKNOKTALARI'],
  targets: ['client'],
  parameters: [
    oneRaster('Değerleri nokta yapılacak raster.'),
    BAND,
    {
      name: 'mode',
      label: 'Biçim',
      type: 'enum',
      options: [
        { value: 'step', label: 'Adımla' },
        { value: 'all', label: 'Her hücre' },
        { value: 'extrema', label: 'Tepeler ve çukurlar' },
      ],
      default: 'step',
    },
    { name: 'step', label: 'Adım', type: 'number', default: 10, min: 1, max: 100000, integer: true, unit: '', description: 'Hücre cinsinden: k hücrede bir nokta.', visibleWhen: (v: Shown) => (v.mode ?? 'step') === 'step' },
    { name: 'radius', label: 'Pencere yarıçapı', type: 'number', default: 1, min: 1, max: 50, integer: true, unit: '', description: 'Hücre cinsinden: (2r + 1)² pencere.', visibleWhen: (v: Shown) => v.mode === 'extrema' },
    { name: 'elevation', label: 'Kot olarak yaz', type: 'boolean', default: true, description: 'Noktanın kotu hücrenin değeri.' },
    vectorLayer('Noktalar', '#30A46C'),
  ] as const,
  outputs: FEATURES('Noktalar'),
  run: (v, ctx, feedback) =>
    runVector(v, ctx, feedback, { kind: 'toPoints', band: v.band ?? 1, mode: v.mode, step: v.step ?? 10, radius: v.radius ?? 1 }, 'Noktalar çıkarılıyor', (f, layerId) => {
      const extrema = v.mode === 'extrema';
      const add = objectsOf(
        f,
        layerId,
        (k) => ({ Değer: f.texts[k], ...(extrema ? { Tür: f.tags[k] === 1 ? 'Tepe' : 'Çukur' } : {}) }),
        v.elevation !== false ? (k) => f.values[k] : undefined,
      );
      let summary = add.length ? `${count(add.length)} nokta yazıldı.` : 'Nokta çıkmadı: değeri olan hücre yok.';
      if (extrema && add.length) {
        const peaks = f.tags.reduce((n, t) => n + (t === 1 ? 1 : 0), 0);
        summary = `${count(peaks)} tepe, ${count(add.length - peaks)} çukur yazıldı.`;
      }
      return { add, summary };
    }),
});

export const scanCaptureLine = defineTool({
  id: 'scan.captureLine',
  label: 'Çizgi yakala',
  category: SCANNED,
  icon: 'captureLine',
  description: 'Taranmış paftada tıklanan çizgiyi (ve ona bağlı aynı renkteki çizgileri) yakalayıp çoklu çizgi yapar; isterseniz kotuyla.',
  help: [
    'Noktanın çevresindeki 7 × 7 hücrenin en koyusu tohumdur; hedef renk onun rengidir. Hedefe Renk toleransı içindeki, tohuma köşe ya da kenarla bağlı hücreler çizgidir. Tıklanan çizgiye bağlı aynı renkteki bütün çizgiler yakalanır: yalnız bir eğri için toleransı daraltın.',
    HELP_THIN,
    'Okuma tohumun çevresindeki 1024 × 1024 hücreyle başlar, çizgi pencerenin kenarına değdikçe genişler (en çok 8192 × 8192). 4 194 304 hücreden büyük yakalama reddedilir. Kot verilirse çizgilerin bütün köşelerinin kotu odur (eş yükselti eğrisi).',
  ].join('\n\n'),
  keywords: ['yakala', 'hat', 'çizgi', 'taranmış', 'pafta', 'trace', 'vectorization', 'eş yükselti', 'kontur'],
  aliases: ['RASTERDENHATYAKALA', 'HATYAKALA', 'RASTERDENEGRIYAKALA'],
  targets: ['client'],
  parameters: [
    oneRaster('Taranmış pafta ya da harita rasteri.'),
    { name: 'at', label: 'Çizgi üzerinde nokta', type: 'point', description: 'Çizginin üstüne ya da yanına tıklayın.' },
    { name: 'tolerance', label: 'Renk toleransı', type: 'number', default: 60, min: 0, max: 442, unit: '' },
    { name: 'spur', label: 'Kısa parçaları at', type: 'number', default: 5, min: 0, max: 10000, integer: true, unit: '', description: 'Hücre cinsinden; 0 hiçbirini atmaz.' },
    { name: 'simplify', label: 'Sadeleştirme', type: 'number', default: 1, min: 0, max: 1000, unit: '', description: 'Douglas–Peucker toleransı, hücre cinsinden; 0 yalnız doğrultudaki köşeleri atar.' },
    { name: 'z', label: 'Kot', type: 'number', optional: true, default: null, min: -1e6, max: 1e6, unit: 'm', placeholder: 'Yok', description: 'Verilirse çizgilerin bütün köşelerinin kotu.' },
    vectorLayer('Yakalanan çizgiler', '#D6409F'),
  ] as const,
  outputs: FEATURES('Çizgiler'),
  run: (v, ctx, feedback) => {
    if (!v.at) return Promise.resolve({ refused: 'Çizgi üzerinde bir nokta seçin.' });
    return runVector(
      v,
      ctx,
      feedback,
      { kind: 'captureLine', x: v.at.x, y: v.at.y, tolerance: v.tolerance ?? 60, spur: v.spur ?? 5, simplify: v.simplify ?? 1 },
      'Çizgi yakalanıyor',
      (f, layerId) => {
        const z = v.z ?? undefined;
        const add = objectsOf(f, layerId, () => ({}), z !== undefined ? () => z : undefined);
        return { add, summary: add.length ? `${count(add.length)} çizgi yakalandı.` : 'Yakalanan hücreler çizgi vermedi: hepsi kısa parça olarak atıldı.' };
      },
    );
  },
});

export const scanCloseArea = defineTool({
  id: 'scan.closeArea',
  label: 'Alan kapat',
  category: SCANNED,
  icon: 'closeArea',
  description: 'Taranmış paftada tıklanan noktanın çevresindeki çizgilerin kapattığı alanı bulup alan yapar.',
  help: [
    'Alan, noktanın hücresinin rengine Renk toleransı içindeki hücrelerin o hücreye kenarla bağlı kümesidir (çizgiler köşeyle bağlı olduğu için çapraz boşluktan sızmaz). Küme rasterin kenarına ulaşırsa alan kapanmıyordur: reddedilir.',
    "Halkalar hücre kenarlarındandır; Doldur'da delikler atılır. Halkalar Douglas–Peucker'la sadeleşir; sadeleşen halkalar kendine ya da birbirine değerse alan sadeleştirilmeden yazılır ve söylenir.",
  ].join('\n\n'),
  keywords: ['alan kapat', 'kapalı alan', 'flood', 'doldur', 'parsel', 'taranmış', 'pafta'],
  aliases: ['RASTERDENALANKAPAT', 'PAFTAALANKAPAT'],
  targets: ['client'],
  parameters: [
    oneRaster('Taranmış pafta ya da harita rasteri.'),
    { name: 'at', label: 'Alanın içinde nokta', type: 'point', description: 'Kapatılacak alanın içine tıklayın.' },
    { name: 'tolerance', label: 'Renk toleransı', type: 'number', default: 60, min: 0, max: 442, unit: '' },
    {
      name: 'holes',
      label: 'Delikler',
      type: 'enum',
      options: [
        { value: 'fill', label: 'Doldur' },
        { value: 'keep', label: 'Koru' },
      ],
      default: 'fill',
    },
    { name: 'simplify', label: 'Sadeleştirme', type: 'number', default: 1, min: 0, max: 1000, unit: '', description: 'Douglas–Peucker toleransı, hücre cinsinden; 0 yalnız doğrultudaki köşeleri atar.' },
    vectorLayer('Kapatılan alanlar', '#6E56CF'),
  ] as const,
  outputs: FEATURES('Alanlar'),
  run: (v, ctx, feedback) => {
    if (!v.at) return Promise.resolve({ refused: 'Alanın içinde bir nokta seçin.' });
    return runVector(
      v,
      ctx,
      feedback,
      { kind: 'closeArea', x: v.at.x, y: v.at.y, tolerance: v.tolerance ?? 60, holes: v.holes, simplify: v.simplify ?? 1 },
      'Alan kapatılıyor',
      (f, layerId) => {
        const add = objectsOf(f, layerId, () => ({}));
        if (f.tags[0] === 1) feedback.warn('Sadeleşen halkalar birbirine değdiği için alan sadeleştirilmeden yazıldı.');
        const holes = Math.max(0, (f.rings[0] ?? 1) - 1);
        const corners = f.sizes.reduce((n, s) => n + s, 0);
        return { add, summary: `Alan kapatıldı: ${count(corners)} köşe${holes ? `, ${count(holes)} delik` : ''}.` };
      },
    );
  },
});

export const scanContourElevations = defineTool({
  id: 'scan.contourElevations',
  label: 'Eğrilere kot ver',
  category: SCANNED,
  icon: 'contourElevations',
  description: 'Kesen bir çizginin geçtiği eğrilere sırayla kot verir: ilk kot, sonra her eğride bir aralık.',
  help: [
    'Her eğrinin yeri, kesen çizgiyle kesişimlerinden başlangıca en yakınıdır; eğriler bu yerlere göre (eşitse girdinin sırasıyla) sıralanır ve k. eğrinin kotu İlk kot + k · Aralık olur. Kesmeyen eğri değişmez, söylenir.',
    'Kot eğrinin bütün köşelerine (deliklerin ve parçaların dahil) yazılır; tek adımda. Eksi aralık kotları azaltır.',
  ].join('\n\n'),
  keywords: ['kot', 'eğri', 'eş yükselti', 'kontur', 'contour', 'elevation', 'taranmış'],
  aliases: ['EGRILEREKOTVER'],
  targets: ['client'],
  parameters: [
    {
      name: 'curves',
      label: 'Eğriler',
      type: 'features',
      kinds: ['line', 'polyline', 'polygon'],
      scopes: ['selection', 'layer', 'visible', 'all'],
      writes: true,
      description: 'Kot verilecek eğriler; kilitli katmandakiler alınmaz.',
    },
    { name: 'start', label: 'Başlangıç', type: 'point', description: 'Kesen çizginin ilk eğriden önceki ucu.' },
    { name: 'end', label: 'Bitiş', type: 'point', description: 'Kesen çizginin öbür ucu.' },
    { name: 'first', label: 'İlk kot', type: 'number', default: 0, min: -1e6, max: 1e6, unit: 'm' },
    { name: 'step', label: 'Aralık', type: 'number', default: 5, min: -1e6, max: 1e6, unit: 'm', description: '0 olamaz; eksi aralık kotları azaltır.' },
  ] as const,
  outputs: [
    { name: 'changed', label: 'Değişen eğriler', type: 'features' },
    { name: 'count', label: 'Kot verilen eğri sayısı', type: 'number' },
  ],
  run: (v, ctx, feedback) => Promise.resolve(runContourElevations(v, ctx, feedback)),
});

export const RASTER_VECTOR_TOOLS = [rasterize, rasterToPolygons, rasterToLines, rasterToPoints, scanCaptureLine, scanCloseArea, scanContourElevations];
