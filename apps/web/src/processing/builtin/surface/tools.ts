import { RASTER_RAMPS } from '../../../model/rasterRules';
import { defineTool, type Shown } from '../../types';
import { ADD, BAND, CATEGORY, CONTOUR_LAYER, HELP_OUTPUT, HELP_SOURCE, METHOD, RASTER, Z_FACTOR, layer, output, runLines, runRaster } from './shared';

/**
 * Yüzey analizi (docs/adr/0231 §1, §10): the eight tools, their parameters as the desktop's
 * (`builtin/surface/tools.rs`), their settings handed to the raster core's job.
 */

export const surfaceSlope = defineTool({
  id: 'surface.slope',
  label: 'Eğim',
  category: CATEGORY,
  icon: 'slope',
  description: 'Yükseklik rasterinin her hücresinin eğimini derece ya da yüzde olarak yazar.',
  help: ['Eğim, hücrenin 3 × 3 komşuluğundan bulunan yüzey eğiminin büyüklüğüdür: derece atan|g|, yüzde 100·|g|.', HELP_SOURCE, HELP_OUTPUT].join('\n\n'),
  keywords: ['eğim', 'slope', 'dem', 'yüzey', 'gdaldem'],
  aliases: ['EGIMANALIZI', 'EGIMHARITASI'],
  targets: ['client'],
  parameters: [
    RASTER,
    BAND,
    METHOD,
    {
      name: 'unit',
      label: 'Birim',
      type: 'enum',
      options: [
        { value: 'degrees', label: 'Derece' },
        { value: 'percent', label: 'Yüzde' },
      ],
      default: 'degrees',
    },
    Z_FACTOR,
    output('-egim'),
    ADD,
    layer('Eğim'),
  ] as const,
  outputs: [{ name: 'file', label: 'Sonuç dosyası', type: 'string' }],
  run: (v, ctx, feedback) => runRaster(v, ctx, feedback, { kind: 'slope', method: v.method, unit: v.unit, zFactor: v.zFactor }, '-egim', 'Eğim hesaplanıyor'),
});

export const surfaceAspect = defineTool({
  id: 'surface.aspect',
  label: 'Bakı',
  category: CATEGORY,
  icon: 'aspect',
  description: 'Her hücrenin baktığı yönü, en dik inişin kuzeyden saat yönündeki açısı olarak yazar; düzlük −1.',
  help: ['Bakı 0–360 derecedir: 0 kuzey, 90 doğu, 180 güney, 270 batı. Eğimi sıfır olan hücre −1 alır.', HELP_SOURCE, HELP_OUTPUT].join('\n\n'),
  keywords: ['bakı', 'aspect', 'yön', 'dem', 'gdaldem'],
  aliases: ['BAKI'],
  targets: ['client'],
  parameters: [RASTER, BAND, METHOD, Z_FACTOR, output('-baki'), ADD, layer('Bakı')] as const,
  outputs: [{ name: 'file', label: 'Sonuç dosyası', type: 'string' }],
  run: (v, ctx, feedback) => runRaster(v, ctx, feedback, { kind: 'aspect', method: v.method, zFactor: v.zFactor }, '-baki', 'Bakı hesaplanıyor'),
});

export const surfaceHillshade = defineTool({
  id: 'surface.hillshade',
  label: 'Gölgeli kabartma',
  category: CATEGORY,
  icon: 'hillshade',
  description: 'Arazinin verilen yönden ışıklandırılmış gri görüntüsünü yazar (1–255, boş hücre 0).',
  help: ["Değer, yüzeyin ışığa dönüklüğüdür (gdaldem hillshade'in formülü): ışığa tam dönük yüzey 255, gölgede kalan 1.", HELP_SOURCE, HELP_OUTPUT].join('\n\n'),
  keywords: ['gölgeli kabartma', 'hillshade', 'rölyef', 'kabartma', 'dem'],
  aliases: ['GOLGEURET', 'KABARTMAURET'],
  targets: ['client'],
  parameters: [
    RASTER,
    BAND,
    { name: 'azimuth', label: 'Işığın açısı', type: 'number', default: 315, min: 0, max: 360, unit: '°', description: 'Işığın geldiği yön, kuzeyden saat yönünde (315: kuzeybatı).' },
    { name: 'altitude', label: 'Işığın yüksekliği', type: 'number', default: 45, min: 0, max: 90, unit: '°', description: 'Işığın ufuktan yüksekliği.' },
    Z_FACTOR,
    output('-golge'),
    ADD,
    layer('Gölgeli kabartma'),
  ] as const,
  outputs: [{ name: 'file', label: 'Sonuç dosyası', type: 'string' }],
  run: (v, ctx, feedback) => runRaster(v, ctx, feedback, { kind: 'hillshade', azimuth: v.azimuth, altitude: v.altitude, zFactor: v.zFactor }, '-golge', 'Gölgeli kabartma hesaplanıyor'),
});

const colors = (v: Shown) => (v.colors ?? 'ramp') as string;
const manual = (v: Shown) => colors(v) === 'ramp' && v.range === 'manual';

export const surfaceColorRelief = defineTool({
  id: 'surface.colorRelief',
  label: 'Renkli kabartma',
  category: CATEGORY,
  icon: 'colorRelief',
  description: 'Yükseklikleri bir rampayla ya da renk tablosuyla renklendirip RGBA raster yazar.',
  help: [
    'Rampa bandın en küçüğünden en büyüğüne (ya da elle verilen aralığa) yayılır. Renk tablosu “değer #RRGGBB” satırlarıdır; tablonun ilk değerinden küçük hücre ilk rengi, son değerinden büyüğü son rengi alır. Boş hücre saydamdır.',
    HELP_SOURCE,
    HELP_OUTPUT,
  ].join('\n\n'),
  keywords: ['renkli kabartma', 'color relief', 'hipsometri', 'renk', 'dem'],
  aliases: ['RENKLIKABARTMA', 'HIPSOMETRI'],
  targets: ['client'],
  parameters: [
    RASTER,
    BAND,
    {
      name: 'colors',
      label: 'Renkler',
      type: 'enum',
      options: [
        { value: 'ramp', label: 'Rampa' },
        { value: 'table', label: 'Renk tablosu' },
      ],
      default: 'ramp',
    },
    { name: 'ramp', label: 'Rampa', type: 'enum', options: RASTER_RAMPS.map((r) => ({ value: r, label: r })), default: 'Arazi', visibleWhen: (v: Shown) => colors(v) === 'ramp' },
    { name: 'invert', label: 'Ters', type: 'boolean', default: false, visibleWhen: (v: Shown) => colors(v) === 'ramp' },
    {
      name: 'range',
      label: 'Değer aralığı',
      type: 'enum',
      options: [
        { value: 'auto', label: 'Bandın en küçüğü–en büyüğü' },
        { value: 'manual', label: 'Elle' },
      ],
      default: 'auto',
      visibleWhen: (v: Shown) => colors(v) === 'ramp',
    },
    { name: 'min', label: 'En küçük', type: 'number', default: 0, min: -1e9, max: 1e9, unit: '', visibleWhen: manual },
    { name: 'max', label: 'En büyük', type: 'number', default: 1000, min: -1e9, max: 1e9, unit: '', visibleWhen: manual },
    {
      name: 'table',
      label: 'Renk tablosu',
      type: 'string',
      default: '',
      allowEmpty: true,
      maxLength: 20000,
      placeholder: '100 #2E7D32; 500 #FFF59D; 1000 #FAFAFA',
      description: "Satırlar “değer #RRGGBB” ya da “değer #RRGGBBAA”, satır başına ya da noktalı virgülle ayrılarak (gdaldem color-relief'in metni gibi).",
      visibleWhen: (v: Shown) => colors(v) === 'table',
    },
    {
      name: 'interp',
      label: 'Ara renkler',
      type: 'enum',
      options: [
        { value: 'linear', label: 'Doğrusal' },
        { value: 'nearest', label: 'En yakın' },
      ],
      default: 'linear',
    },
    output('-renkli'),
    ADD,
    layer('Renkli kabartma'),
  ] as const,
  outputs: [{ name: 'file', label: 'Sonuç dosyası', type: 'string' }],
  validate: (v) => {
    if (v.colors === 'table' && !String(v.table ?? '').trim()) return 'Renk tablosunu yazın: “değer #RRGGBB” satırları, ör. 100 #2E7D32; 500 #FFF59D.';
    if (manual(v) && !(typeof v.min === 'number' && typeof v.max === 'number' && v.min < v.max)) return 'En küçük değer en büyük değerden küçük olmalı.';
    return null;
  },
  run: (v, ctx, feedback) => {
    const table = v.colors === 'table';
    const byHand = !table && v.range === 'manual';
    return runRaster(
      v,
      ctx,
      feedback,
      {
        kind: 'colorRelief',
        ramp: table ? null : v.ramp,
        invert: !table && v.invert === true,
        min: byHand ? v.min : null,
        max: byHand ? v.max : null,
        table: table ? v.table : null,
        interp: v.interp,
      },
      '-renkli',
      'Renkli kabartma hesaplanıyor',
    );
  },
});

export const surfaceCurvature = defineTool({
  id: 'surface.curvature',
  label: 'Eğrilik',
  category: CATEGORY,
  icon: 'curvature',
  description: "Yüzeyin toplam, profil ya da plan eğriliğini (ArcGIS'teki gibi 1/100 m⁻¹) yazar.",
  help: [
    "Zevenbergen ve Thorne'un yüzeyinden: toplamda artı tepe, eksi çukur; profilde eksi dışbükey (akış yavaşlar), artı içbükey; planda artı sırt, eksi dere.",
    'Eğrilik dik pikselli raster ister (afinin eksenleri dik).',
    HELP_SOURCE,
    HELP_OUTPUT,
  ].join('\n\n'),
  keywords: ['eğrilik', 'eğrisellik', 'curvature', 'profil', 'plan', 'dem'],
  aliases: ['EGRILIK', 'EGRISELLIK'],
  targets: ['client'],
  parameters: [
    RASTER,
    BAND,
    {
      name: 'curvature',
      label: 'Eğrilik',
      type: 'enum',
      options: [
        { value: 'total', label: 'Toplam' },
        { value: 'profile', label: 'Profil' },
        { value: 'plan', label: 'Plan' },
      ],
      default: 'total',
      description: 'Toplam: yüzeyin eğriliği; profil: eğim yönünde; plan: eğriler boyunca.',
    },
    Z_FACTOR,
    output('-egrilik'),
    ADD,
    layer('Eğrilik'),
  ] as const,
  outputs: [{ name: 'file', label: 'Sonuç dosyası', type: 'string' }],
  run: (v, ctx, feedback) => runRaster(v, ctx, feedback, { kind: 'curvature', curvature: v.curvature, zFactor: v.zFactor }, '-egrilik', 'Eğrilik hesaplanıyor'),
});

export const surfaceRuggedness = defineTool({
  id: 'surface.ruggedness',
  label: 'Pürüzlülük',
  category: CATEGORY,
  icon: 'ruggedness',
  description: 'Arazinin pürüzlülüğünü TRI (Riley ya da Wilson), TPI ya da engebe olarak yazar.',
  help: [
    'TRI (Riley): komşuların merkezden farklarının karelerinin toplamının karekökü; TRI (Wilson): farkların mutlak değerlerinin ortalaması; TPI: merkez eksi komşuların ortalaması; Engebe: 3 × 3 pencerenin en büyüğü eksi en küçüğü. Yükseklikler z çarpanıyla çarpılmaz (gdaldem gibi).',
    HELP_SOURCE,
    HELP_OUTPUT,
  ].join('\n\n'),
  keywords: ['pürüzlülük', 'tri', 'tpi', 'engebe', 'ruggedness', 'roughness', 'dem'],
  aliases: ['PURUZLULUK', 'TRI', 'TPI'],
  targets: ['client'],
  parameters: [
    RASTER,
    BAND,
    {
      name: 'index',
      label: 'Ölçü',
      type: 'enum',
      options: [
        { value: 'triRiley', label: 'TRI (Riley)' },
        { value: 'triWilson', label: 'TRI (Wilson)' },
        { value: 'tpi', label: 'TPI' },
        { value: 'roughness', label: 'Engebe' },
      ],
      default: 'triRiley',
      description: 'TRI: komşuların merkezden farkı; TPI: merkezin komşuların ortalamasından farkı; Engebe: penceredeki en büyük ile en küçüğün farkı.',
    },
    output('-puruzluluk'),
    ADD,
    layer('Pürüzlülük'),
  ] as const,
  outputs: [{ name: 'file', label: 'Sonuç dosyası', type: 'string' }],
  run: (v, ctx, feedback) => runRaster(v, ctx, feedback, { kind: 'ruggedness', index: v.index }, '-puruzluluk', 'Pürüzlülük hesaplanıyor'),
});

const MONTHS = ['Ocak', 'Şubat', 'Mart', 'Nisan', 'Mayıs', 'Haziran', 'Temmuz', 'Ağustos', 'Eylül', 'Ekim', 'Kasım', 'Aralık'].map((label, i) => ({ value: String(i + 1), label }));
const MONTH_DAYS = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
const period = (v: Shown) => (v.period ?? 'year') as string;

/** A day of the year (1–365); why not (the raster core's `insolation::day_of_year`). */
function dayOfYear(month: number, day: unknown): number | string {
  if (typeof day !== 'number' || !Number.isInteger(day) || day < 1 || day > 31) return 'Gün 1 ile 31 arasında bir tam sayı olmalı.';
  if (!(month >= 1 && month <= 12)) return `Ay 1 ile 12 arasında olmalı; ${month} verildi.`;
  if (day > MONTH_DAYS[month - 1]) return `${MONTHS[month - 1].label} ayının günü 1 ile ${MONTH_DAYS[month - 1]} arasında olmalı; ${day} verildi.`;
  return MONTH_DAYS.slice(0, month - 1).reduce((a, b) => a + b, 0) + day;
}

/** The period's first and last day of the year; why not. */
export function periodDays(v: Shown): [number, number] | string {
  if (period(v) === 'year') return [1, 365];
  const first = dayOfYear(Number(v.startMonth ?? 1), v.startDay);
  if (typeof first === 'string') return first;
  if (period(v) === 'day') return [first, first];
  const last = dayOfYear(Number(v.endMonth ?? 1), v.endDay);
  if (typeof last === 'string') return last;
  return first > last ? 'Bitiş başlangıçtan önce; aynı yıl içinde bir aralık verin.' : [first, last];
}

export const surfaceInsolation = defineTool({
  id: 'surface.insolation',
  label: 'Güneşlenme',
  category: CATEGORY,
  icon: 'insolation',
  description: 'Dönemde eğimli yüzeye gelen doğrudan güneş enerjisini (açık gök, kWh/m²) yazar.',
  help: [
    "Güneşin yeri Spencer'ın formülleriyle, hava kütlesi Kasten ve Young'ınkiyle bulunur; ışınım 1367 W/m²·E₀·τ^m·max(0, n·s). Yalnız yüzeyin kendi gölgesi vardır; arazinin gölgesi ve dağınık ışınım hesaba girmez.",
    'Dönem 365 günlük yıldır; Gün aralığı ve Saat aralığı hesabın sıklığıdır.',
    HELP_SOURCE,
    HELP_OUTPUT,
  ].join('\n\n'),
  keywords: ['güneşlenme', 'güneş', 'radyasyon', 'ışınım', 'insolation', 'solar', 'dem'],
  aliases: ['GUNESLENME', 'GUNESRADYASYONU'],
  targets: ['client'],
  parameters: [
    RASTER,
    BAND,
    {
      name: 'period',
      label: 'Dönem',
      type: 'enum',
      options: [
        { value: 'year', label: 'Yıl' },
        { value: 'range', label: 'Tarih aralığı' },
        { value: 'day', label: 'Tek gün' },
      ],
      default: 'year',
    },
    { name: 'startDay', label: 'Başlangıç günü', type: 'number', default: 21, min: 1, max: 31, integer: true, unit: '', visibleWhen: (v: Shown) => period(v) !== 'year' },
    { name: 'startMonth', label: 'Başlangıç ayı', type: 'enum', options: MONTHS, default: '6', visibleWhen: (v: Shown) => period(v) !== 'year' },
    { name: 'endDay', label: 'Bitiş günü', type: 'number', default: 21, min: 1, max: 31, integer: true, unit: '', visibleWhen: (v: Shown) => period(v) === 'range' },
    { name: 'endMonth', label: 'Bitiş ayı', type: 'enum', options: MONTHS, default: '9', visibleWhen: (v: Shown) => period(v) === 'range' },
    {
      name: 'dayStep',
      label: 'Gün aralığı',
      type: 'number',
      default: 14,
      min: 1,
      max: 365,
      integer: true,
      unit: '',
      description: 'Dönem bu kadar günlük parçalara bölünür; her parçayı ortasındaki gün temsil eder.',
      visibleWhen: (v: Shown) => period(v) !== 'day',
    },
    {
      name: 'hourStep',
      label: 'Saat aralığı',
      type: 'enum',
      options: [
        { value: '0.25', label: '15 dakika' },
        { value: '0.5', label: '30 dakika' },
        { value: '1', label: '1 saat' },
        { value: '2', label: '2 saat' },
      ],
      default: '0.5',
    },
    { name: 'transmissivity', label: 'Geçirgenlik', type: 'number', default: 0.5, min: 0.01, max: 1, unit: '', description: 'Atmosferin dik gelen ışığı geçirme oranı (açık gök 0,5–0,7).' },
    {
      name: 'latitude',
      label: 'Enlem',
      type: 'number',
      default: null,
      min: -90,
      max: 90,
      unit: '°',
      optional: true,
      placeholder: 'Projenin sisteminden',
      description: 'Yalnız koordinat sistemi olmayan (yerel) projede gerekir; sistemi olan projede her satırın kendi enlemi kullanılır.',
    },
    Z_FACTOR,
    output('-gunes'),
    ADD,
    layer('Güneşlenme'),
  ] as const,
  outputs: [{ name: 'file', label: 'Sonuç dosyası', type: 'string' }],
  validate: (v) => {
    const days = periodDays(v);
    return typeof days === 'string' ? days : null;
  },
  run: (v, ctx, feedback) => {
    const days = periodDays(v);
    if (typeof days === 'string') return { refused: days };
    return runRaster(
      v,
      ctx,
      feedback,
      {
        kind: 'insolation',
        firstDay: days[0],
        lastDay: days[1],
        dayStep: v.period === 'day' ? 1 : (v.dayStep ?? 14),
        hourStep: Number(v.hourStep),
        transmissivity: v.transmissivity,
        latitude: v.latitude ?? null,
        zFactor: v.zFactor,
      },
      '-gunes',
      'Güneşlenme hesaplanıyor',
    );
  },
});

export const surfaceContours = defineTool({
  id: 'surface.contours',
  label: 'Eş yükselti eğrileri',
  category: CATEGORY,
  icon: 'contours',
  description: 'Yükseklik rasterinden kotlu eş yükselti eğrileri (ana ve ara) çıkarır.',
  help: [
    'Eğriler hücre merkezlerinin karelerinden (marching squares) çıkar; eyerde karenin ortası karar verir. Yüksek taraf eğrinin solundadır; boş hücrede eğri kesilir.',
    'Her eğri çoklu çizgidir: köşelerinin kotu düzeydir, öznitelikleri Kot ve Tür (Ana ya da Ara); ana eğri 0,35 mm kalınlıktadır.',
    "Raster tek bir yükseklik rasteridir (DEM); bandın nodata'sı ve NaN boş sayılır. 10 000'den çok düzey ya da 5 milyondan çok köşe reddedilir: aralığı büyütün.",
  ].join('\n\n'),
  keywords: ['eş yükselti', 'eşyükselti', 'kontur', 'contour', 'eğri', 'dem'],
  aliases: ['ESYUKSELTI', 'KONTUR', 'EGRIURET'],
  targets: ['client'],
  parameters: [
    RASTER,
    BAND,
    { name: 'interval', label: 'Aralık', type: 'number', default: 5, min: 0.001, max: 1e6, unit: 'm', description: 'İki eğri arasındaki yükseklik farkı.' },
    { name: 'base', label: 'Taban', type: 'number', default: 0, min: -1e6, max: 1e6, unit: 'm', description: 'Düzeyler tabandan aralık aralık sayılır.' },
    { name: 'indexEvery', label: 'Ana eğri her', type: 'number', default: 5, min: 1, max: 1000, integer: true, unit: '', description: 'Bu kadar eğride bir ana eğri (kalın çizilir).' },
    { name: 'simplify', label: 'Sadeleştir', type: 'number', default: 0, min: 0, max: 1e4, unit: 'm', description: 'Douglas-Peucker toleransı; 0 sadeleştirmez.' },
    layer('Eş yükselti eğrileri', CONTOUR_LAYER),
  ] as const,
  outputs: [{ name: 'lines', label: 'Eğri sayısı', type: 'number' }],
  run: (v, ctx, feedback) => runLines(v, ctx, feedback, { kind: 'contours', interval: v.interval, base: v.base, indexEvery: v.indexEvery ?? 5, simplify: v.simplify ?? 0 }),
});

/** The category's tools, in the toolbox's order. */
export const SURFACE_TOOLS = [surfaceSlope, surfaceAspect, surfaceHillshade, surfaceColorRelief, surfaceCurvature, surfaceRuggedness, surfaceInsolation, surfaceContours];
