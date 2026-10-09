import { defineTool, type Shown } from '../../types';
import {
  DENSITY,
  DENSITY_POINTS,
  FIELD,
  HELP_CROSS,
  HELP_GRID,
  HELP_OUTPUT,
  HELP_POINTS,
  INTERPOLATION,
  LINES,
  POINTS,
  ends,
  endsCross,
  runPoints,
  weightField,
} from './shared';

/**
 * İnterpolasyon and Yoğunluk (docs/adr/0232 §1, §13): the seven tools, their parameters as the desktop's
 * (`builtin/interpolation/tools.rs`), their settings handed to the raster core's point job.
 */

const OUTPUTS_INTERP = [
  { name: 'file', label: 'Sonuç dosyası', type: 'string' },
  { name: 'table', label: 'Çapraz doğrulama', type: 'table' },
] as const;
const OUTPUTS_DENSITY = [{ name: 'file', label: 'Sonuç dosyası', type: 'string' }] as const;

const POWER = { name: 'power', label: 'Üs', type: 'number', default: 2, min: 0.1, max: 10, unit: '', description: 'Ağırlık uzaklığın bu üssüyle azalır: büyük üs yakın noktaları öne çıkarır.' } as const;
const points = (least: number, description: string) =>
  ({ name: 'points', label: 'Nokta sayısı', type: 'number', default: 12, min: least, max: 64, integer: true, unit: '', description }) as const;
const RADIUS = (description: string) => ({ name: 'radius', label: 'Arama yarıçapı', type: 'number', default: 0, min: 0, max: 1e7, unit: 'm', description }) as const;

export const interpolationIdw = defineTool({
  id: 'interpolation.idw',
  label: 'Ters uzaklık (IDW)',
  category: INTERPOLATION,
  icon: 'idw',
  description: 'Noktalardan yüzey: her hücre en yakın noktaların uzaklıklarının ters üssüyle ağırlıklı ortalamasıdır.',
  help: [
    "Hücrenin değeri Σ wᵢ zᵢ / Σ wᵢ, wᵢ = 1 / dᵢ^üs; en yakın Nokta sayısı kadar nokta (eşit uzaklıkta önce sırası küçük olan), Arama yarıçapı verilmişse onun içindekiler. Bir nokta hücrenin merkezindeyse değer onunkidir. GDAL'ın invdistnn'iyle aynı tanım.",
    HELP_POINTS,
    HELP_GRID,
    HELP_CROSS,
    HELP_OUTPUT,
  ].join('\n\n'),
  keywords: ['idw', 'ters uzaklık', 'ters mesafe', 'ağırlıklı ortalama', 'interpolasyon', 'enterpolasyon', 'yüzey', 'inverse distance'],
  aliases: ['IDW', 'TERSUZAKLIK', 'TERSMESAFE'],
  targets: ['client'],
  parameters: [
    POINTS,
    FIELD,
    POWER,
    points(1, 'Her hücrede en yakın bu kadar nokta.'),
    RADIUS('0: sınırsız; verilirse yalnız bu uzaklıktaki noktalar.'),
    { name: 'minPoints', label: 'En az nokta', type: 'number', default: 1, min: 1, max: 64, integer: true, unit: '', description: 'Yarıçapta bundan az nokta olan hücre boş kalır.' },
    ...endsCross('-idw', 'IDW'),
  ] as const,
  outputs: OUTPUTS_INTERP,
  run: (v, ctx, feedback) =>
    runPoints(v, ctx, feedback, { kind: 'idw', power: v.power, points: v.points ?? 12, radius: v.radius ?? 0, minPoints: v.minPoints ?? 1 }, '-idw', 'Ters uzaklıkla hesaplanıyor', 'points'),
});

export const interpolationNaturalNeighbor = defineTool({
  id: 'interpolation.naturalNeighbor',
  label: 'Doğal komşu',
  category: INTERPOLATION,
  icon: 'naturalNeighbor',
  description: 'Noktalardan yüzey: her hücre doğal komşularının, Voronoi hücrelerinden aldığı alanlarla ağırlıklı ortalamasıdır (Sibson).',
  help: [
    'Hücre noktalara eklenseydi Voronoi hücresinin her komşudan alacağı alan o komşunun ağırlığıdır; yüzey noktalardan geçer ve yumuşaktır. Noktaların dışbükey kabuğunun dışı boş kalır.',
    HELP_POINTS,
    HELP_GRID,
    HELP_CROSS,
    HELP_OUTPUT,
  ].join('\n\n'),
  keywords: ['doğal komşu', 'natural neighbor', 'sibson', 'voronoi', 'interpolasyon', 'yüzey'],
  aliases: ['DOGALKOMSU', 'SIBSON'],
  targets: ['client'],
  parameters: [POINTS, FIELD, ...endsCross('-dogalkomsu', 'Doğal komşu')] as const,
  outputs: OUTPUTS_INTERP,
  run: (v, ctx, feedback) => runPoints(v, ctx, feedback, { kind: 'naturalNeighbor' }, '-dogalkomsu', 'Doğal komşuyla hesaplanıyor', 'points'),
});

export const interpolationSpline = defineTool({
  id: 'interpolation.spline',
  label: 'Spline',
  category: INTERPOLATION,
  icon: 'splineSurface',
  description: "Noktalardan geçen en az eğrilikli yüzey (ArcGIS'in Spline'ı): düzenlemeli ya da gerilimli.",
  help: [
    "Her hücrede en yakın Nokta sayısı kadar noktadan geçen eğri (Mitas ve Mitasova 1988, ArcGIS'in tanımı); komşu kümesi değiştikçe yüzeyde küçük basamaklar olabilir. Doğru üzerindeki komşularla düzenlemeli spline çözülmez, hücre boş kalır.",
    HELP_POINTS,
    HELP_GRID,
    HELP_CROSS,
    HELP_OUTPUT,
  ].join('\n\n'),
  keywords: ['spline', 'eğri', 'ince plaka', 'thin plate', 'düzenlemeli', 'gerilimli', 'interpolasyon', 'yüzey'],
  aliases: ['SPLINEYUZEY', 'SPLINEINTERPOLASYON'],
  targets: ['client'],
  parameters: [
    POINTS,
    FIELD,
    {
      name: 'splineType',
      label: 'Tür',
      type: 'enum',
      options: [
        { value: 'regularized', label: 'Düzenlemeli' },
        { value: 'tension', label: 'Gerilimli' },
      ],
      default: 'regularized',
      description: 'Düzenlemeli: yumuşak, ağırlık büyüdükçe daha yumuşak; Gerilimli: sert, ağırlık büyüdükçe daha gergin.',
    },
    { name: 'weight', label: 'Ağırlık', type: 'number', default: 0.1, min: 0, max: 100, unit: '', description: "Düzenlemelide τ², 0 ile 5 arası (0: ince plaka); gerilimlide φ², 0'dan büyük." },
    points(3, 'Her hücrede en yakın bu kadar noktadan geçen eğri.'),
    ...endsCross('-spline', 'Spline'),
  ] as const,
  outputs: OUTPUTS_INTERP,
  run: (v, ctx, feedback) =>
    runPoints(v, ctx, feedback, { kind: 'spline', spline: v.splineType, weight: v.weight, points: v.points ?? 12 }, '-spline', "Spline'la hesaplanıyor", 'points'),
});

const manual = (v: Shown) => v.variogram === 'manual';

export const interpolationKriging = defineTool({
  id: 'interpolation.kriging',
  label: 'Kriging',
  category: INTERPOLATION,
  icon: 'kriging',
  description: 'Noktalardan yüzey: sıradan kriging, variogram otomatik uydurulur ya da elle verilir; isteğe bağlı standart hata.',
  help: [
    'Her hücrede en yakın Nokta sayısı kadar noktanın sıradan kriging tahmini; variogram küresel, üstel ya da Gauss. Otomatik variogram ampirik variograma ağırlıklı en küçük karelerle uydurulur, değerleri özette söylenir. Hata yüzeyi tahminin standart hatasını ikinci bant olarak yazar.',
    HELP_POINTS,
    HELP_GRID,
    HELP_CROSS,
    HELP_OUTPUT,
  ].join('\n\n'),
  keywords: ['kriging', 'variogram', 'jeoistatistik', 'ordinary kriging', 'standart hata', 'interpolasyon', 'yüzey'],
  aliases: ['KRIGING'],
  targets: ['client'],
  parameters: [
    POINTS,
    FIELD,
    {
      name: 'model',
      label: 'Model',
      type: 'enum',
      options: [
        { value: 'spherical', label: 'Küresel' },
        { value: 'exponential', label: 'Üstel' },
        { value: 'gaussian', label: 'Gauss' },
      ],
      default: 'spherical',
      description: 'Variogramın biçimi: erime kadar artıp eşikte duran (pratik erim).',
    },
    {
      name: 'variogram',
      label: 'Variogram',
      type: 'enum',
      options: [
        { value: 'auto', label: 'Otomatik' },
        { value: 'manual', label: 'Elle' },
      ],
      default: 'auto',
      description: 'Otomatik: noktaların ampirik variogramına uydurulur; Elle: külçe, kısmi eşik ve erim yazılır.',
    },
    {
      name: 'lags',
      label: 'Aralık sayısı',
      type: 'number',
      default: 12,
      min: 3,
      max: 100,
      integer: true,
      unit: '',
      description: 'Ampirik variogramın eşit aralıkları (en büyük uzaklık kutunun köşegeninin yarısı).',
      visibleWhen: (v: Shown) => v.variogram !== 'manual',
    },
    { name: 'nugget', label: 'Külçe', type: 'number', default: 0, min: 0, max: 1e12, unit: '', description: "γ'nın 0'dan hemen sonraki sıçraması.", visibleWhen: manual },
    { name: 'sill', label: 'Kısmi eşik', type: 'number', default: 1, min: 0, max: 1e12, unit: '', description: 'Külçenin üstüne erimde eklenen.', visibleWhen: manual },
    { name: 'range', label: 'Erim', type: 'number', default: 100, min: 1e-6, max: 1e9, unit: 'm', description: 'Variogramın eşiğe ulaştığı uzaklık.', visibleWhen: manual },
    points(1, 'Her hücrede en yakın bu kadar nokta.'),
    RADIUS('0: sınırsız.'),
    { name: 'errorSurface', label: 'Hata yüzeyi', type: 'boolean', default: false, description: 'Tahminin standart hatası ikinci bant olarak yazılır ve kendi katmanında çizilir.' },
    ...endsCross('-kriging', 'Kriging'),
    {
      name: 'errorLayer',
      label: 'Hata katmanı',
      type: 'layer',
      default: { newName: 'Kriging standart hatası' },
      newLayerStyle: { color: '#7A6B5B', lineWeight: 0.25 },
      below: 'input',
      description: 'Bu adda katman yoksa oluşturulur.',
      visibleWhen: (v: Shown) => v.errorSurface === true && v.add !== false,
    },
  ] as const,
  outputs: OUTPUTS_INTERP,
  run: (v, ctx, feedback) => {
    const variogram = v.variogram === 'manual' ? { fit: 'manual', nugget: v.nugget, sill: v.sill, range: v.range } : { fit: 'auto', lags: v.lags ?? 12 };
    return runPoints(
      v,
      ctx,
      feedback,
      { kind: 'kriging', model: v.model, variogram, points: v.points ?? 12, radius: v.radius ?? 0, error: v.errorSurface === true },
      '-kriging',
      "Kriging'le hesaplanıyor",
      'points',
    );
  },
});

export const interpolationTin = defineTool({
  id: 'interpolation.tin',
  label: "TIN'den raster",
  category: INTERPOLATION,
  icon: 'tinRaster',
  description: 'Noktaların Delaunay üçgenlemesinden yüzey: her hücre içinde kaldığı üçgende doğrusal.',
  help: [
    "Noktalar Delaunay üçgenlerine bölünür; hücrenin değeri içinde kaldığı üçgenin üç köşesinden doğrusal hesaplanır. Kabuğun dışı boş kalır. GDAL'ın linear'ıyla aynı tanım; eş çemberli noktalarda (düzgün ızgara) üçgenleme tek değildir.",
    HELP_POINTS,
    HELP_GRID,
    HELP_CROSS,
    HELP_OUTPUT,
  ].join('\n\n'),
  keywords: ['tin', 'üçgenleme', 'delaunay', 'doğrusal', 'yüzey', 'dem', 'sayısal arazi modeli'],
  aliases: ['TINRASTER', 'TINDENRASTER'],
  targets: ['client'],
  parameters: [POINTS, FIELD, ...endsCross('-tin', 'TIN')] as const,
  outputs: OUTPUTS_INTERP,
  run: (v, ctx, feedback) => runPoints(v, ctx, feedback, { kind: 'tin' }, '-tin', 'Üçgenlemeyle hesaplanıyor', 'points'),
});

export const densityKernel = defineTool({
  id: 'density.kernel',
  label: 'Çekirdek yoğunluğu',
  category: DENSITY,
  icon: 'kernelDensity',
  description: 'Noktaların yoğunluğu: her nokta yarıçap içindeki hücrelere çekirdeğiyle yayılır (ısı haritası).',
  help: [
    "Hücrenin değeri Σ ağırlık · K(d / r) / r², d < r olan noktalar üstünden; çekirdeğin düzlemdeki integrali 1'dir, yoğunluğun toplamı ağırlıkların toplamına yaklaşır. Yarıçap 0 ise ArcGIS'in Silverman kuralı: 0,9 · min(standart uzaklık, √(1/ln 2) · ortanca uzaklık) · n^−0,2.",
    "Kapsam noktaların kutusu yarıçap kadar büyütülerek; değeri 0 olan hücreler çizimde boştur.",
    HELP_OUTPUT,
  ].join('\n\n'),
  keywords: ['yoğunluk', 'çekirdek', 'kernel density', 'ısı haritası', 'heatmap', 'sıcak nokta'],
  aliases: ['YOGUNLUK', 'CEKIRDEKYOGUNLUGU', 'ISIHARITASI', 'HEATMAP'],
  targets: ['client'],
  parameters: [
    DENSITY_POINTS,
    weightField("Boş bırakılırsa her nokta 1 sayılır; seçilirse nesnenin bu alandaki sayısı (eksi değer alınmaz)."),
    { name: 'radius', label: 'Yarıçap', type: 'number', default: 0, min: 0, max: 1e7, unit: 'm', description: "0: kendiliğinden (Silverman'ın kuralı, ArcGIS'in varsayılanı)." },
    {
      name: 'kernel',
      label: 'Çekirdek',
      type: 'enum',
      options: [
        { value: 'quartic', label: 'Dörtlü' },
        { value: 'triangular', label: 'Üçgen' },
        { value: 'uniform', label: 'Düzgün' },
        { value: 'epanechnikov', label: 'Epanechnikov' },
        { value: 'triweight', label: 'Üçlü ağırlık' },
      ],
      default: 'quartic',
      description: "Noktanın etkisinin uzaklıkla azalışı; integrali 1.",
    },
    {
      name: 'unit',
      label: 'Birim',
      type: 'enum',
      options: [
        { value: 'squareKilometre', label: 'km² başına' },
        { value: 'hectare', label: 'Hektar başına' },
        { value: 'decare', label: 'Dönüm başına' },
        { value: 'squareMetre', label: 'm² başına' },
      ],
      default: 'squareKilometre',
    },
    ...ends('-yogunluk', 'Yoğunluk'),
  ] as const,
  outputs: OUTPUTS_DENSITY,
  run: (v, ctx, feedback) =>
    runPoints(v, ctx, feedback, { kind: 'kernel', radius: v.radius ?? 0, kernel: v.kernel, unit: v.unit }, '-yogunluk', 'Yoğunluk hesaplanıyor', 'weighted'),
});

export const densityLine = defineTool({
  id: 'density.line',
  label: 'Çizgi yoğunluğu',
  category: DENSITY,
  icon: 'lineDensity',
  description: 'Çizgilerin yoğunluğu: her hücre yarıçap içindeki çizgi uzunluğunun dairenin alanına oranıdır.',
  help: [
    'Hücrenin değeri Σ ağırlık · (çizginin dairenin içindeki uzunluğu) / (π r²). Doğru parçaları ve yaylar daireyle kesin kesilir; elips ve eğri 0,1 mm içinde doğru parçalarıyla girer.',
    "Kapsam çizgilerin kutusu yarıçap kadar büyütülerek; değeri 0 olan hücreler çizimde boştur.",
    HELP_OUTPUT,
  ].join('\n\n'),
  keywords: ['çizgi yoğunluğu', 'line density', 'yol yoğunluğu', 'dere yoğunluğu', 'yoğunluk'],
  aliases: ['CIZGIYOGUNLUGU'],
  targets: ['client'],
  parameters: [
    LINES,
    weightField('Boş bırakılırsa her çizgi 1 sayılır; seçilirse nesnenin bu alandaki sayısı.'),
    { name: 'radius', label: 'Yarıçap', type: 'number', default: 0, min: 0, max: 1e7, unit: 'm', description: "0: kendiliğinden (çizgilerin kutusunun kısa kenarının 30'da biri)." },
    {
      name: 'unit',
      label: 'Birim',
      type: 'enum',
      options: [
        { value: 'kilometrePerSquareKilometre', label: 'km/km²' },
        { value: 'metrePerHectare', label: 'm/ha' },
        { value: 'metrePerSquareMetre', label: 'm/m²' },
      ],
      default: 'kilometrePerSquareKilometre',
    },
    ...ends('-cizgiyogunlugu', 'Çizgi yoğunluğu'),
  ] as const,
  outputs: OUTPUTS_DENSITY,
  run: (v, ctx, feedback) =>
    runPoints(v, ctx, feedback, { kind: 'lineDensity', radius: v.radius ?? 0, unit: v.unit }, '-cizgiyogunlugu', 'Çizgi yoğunluğu hesaplanıyor', 'lines'),
});

export const INTERPOLATION_TOOLS = [
  interpolationIdw,
  interpolationNaturalNeighbor,
  interpolationSpline,
  interpolationKriging,
  interpolationTin,
  densityKernel,
  densityLine,
];
