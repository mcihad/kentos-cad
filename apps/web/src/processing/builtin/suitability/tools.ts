import { defineTool, type Shown } from '../../types';
import { ADD, AREA_KINDS, layer, output } from '../rasterOps/shared';
import { SUITABILITY, joined, runPairwise, runRoc, runSuitRaster, sayInvalid, sayOverlay } from './shared';

/**
 * Uygunluk analizi (docs/adr/0237 §3–§8): the six tools, their parameters as the desktop's
 * (`builtin/suitability/tools.rs`), their settings handed to the raster core's operation job.
 */

const HELP_EMPTY = "Rasterin nodata'sı, NaN ve alfası 0 olan pikseller değersizdir.";
const HELP_GRID =
  'Rasterler ortak alanlarında, hücresi en küçük olanın ızgarasında birleşir: sonucun hücreleri merkezi bütün rasterlerin içinde olanlardır; her raster hücre merkezinde en yakın hücresiyle okunur. Bir rasterin değersiz olduğu hücre sonuçta değersizdir.';
const HELP_ORDER = 'Rasterler Katmanlar panelinde üstten aşağı sırayla (aynı katmanda sonra eklenen önce) ve katmanlarının adlarıyla anılır; aynı katmandaki ikinci raster “Ad (2)”.';
const HELP_OUTPUT =
  "Sonuç karolu, Deflate'li ve önizleme katlı GeoTIFF'tir; çıktı dosyası boşsa ilk rasterin adının sonuna ek konur. Web'de 32 MB'a kadar olan sonuç projeye gömülür, büyüğü indirilir ve bu oturumda bağlı kalır. Çizime ekle açıksa raster ilk rasterin katmanının hemen üstündeki yeni katmana eklenir.";

const FILE = [{ name: 'file', label: 'Sonuç dosyası', type: 'string' }] as const;
const TABLE = { name: 'table', label: 'Tablo', type: 'table' } as const;

const oneRaster = (description: string) => ({ name: 'input', label: 'Raster', type: 'features', kinds: ['raster'], scopes: ['selection', 'layer'], description }) as const;
const rasters = <L extends string>(label: L, description: string) =>
  ({ name: 'input', label, type: 'features', kinds: ['raster'], scopes: ['visible', 'selection', 'layer', 'all'], description }) as const;
const BAND = { name: 'band', label: 'Bant', type: 'number', default: 1, min: 1, max: 255, integer: true, unit: '', description: "Değerlerin okunduğu bant (1'den); bütün rasterlerde aynı bant." } as const;
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
const ends = <S extends string, N extends string>(suffix: S, name: N) => [output(suffix), ADD, layer(name)] as const;

const fn = (v: Shown): string => String(v.function ?? 'linear');
const ends_ = (v: Shown): boolean => fn(v) === 'linear' || fn(v) === 'power';

export const fuzzyMembership = defineTool({
  id: 'suitability.fuzzyMembership',
  label: 'Bulanık üyelik',
  category: SUITABILITY,
  icon: 'fuzzyMembership',
  description: 'Rasterin değerlerini bir işlevle 0–1 arası üyeliğe çevirir: ölçütü ortak ölçeğe getirmenin bulanık yolu.',
  help: [
    'Doğrusal: alt değerde 0, üst değerde 1, arada doğrusal (alt değer büyükse azalan). Üslü: doğrusal üyelik üssüne yükseltilir. Gauss: e^(−yayılım·(x − orta)²). Büyük: 1 / (1 + (x / orta)^(−diklik)), 0 ve altı 0. Küçük: 1 / (1 + (x / orta)^diklik), 0 ve altı 1. Yakın: 1 / (1 + yayılım·(x − orta)²).',
    "Varsayılanlar ArcGIS'in Fuzzy Membership'iyle aynıdır: Gauss ve Yakın'da yayılım 0,1, Büyük ve Küçük'te diklik 5.",
    HELP_EMPTY,
    HELP_OUTPUT,
  ].join('\n\n'),
  keywords: ['bulanık', 'fuzzy', 'üyelik', 'membership', 'fuzzify', 'uygunluk', 'standartlaştır'],
  aliases: ['BULANIKUYELIK', 'FUZZYMEMBERSHIP'],
  targets: ['client'],
  parameters: [
    oneRaster('Üyeliğe çevrilecek raster (eğim, uzaklık …).'),
    BAND,
    {
      name: 'function',
      label: 'İşlev',
      type: 'enum',
      options: [
        { value: 'linear', label: 'Doğrusal', hint: "alt değerden üst değere doğru 0'dan 1'e" },
        { value: 'power', label: 'Üslü', hint: 'doğrusalın üssü: yavaş ya da hızlı yükselen' },
        { value: 'gaussian', label: 'Gauss', hint: 'orta noktada 1, iki yana çan eğrisiyle azalan' },
        { value: 'large', label: 'Büyük', hint: "orta noktadan büyük değerler 1'e yakın" },
        { value: 'small', label: 'Küçük', hint: "orta noktadan küçük değerler 1'e yakın" },
        { value: 'near', label: 'Yakın', hint: "orta noktaya yakın değerler 1'e yakın" },
      ],
      default: 'linear',
      description: 'Değer 0–1 üyeliğe çevrilir: 1 bütünüyle uygun, 0 hiç uygun değil.',
    },
    { name: 'low', label: 'Alt değer', type: 'number', default: 0, min: -1e12, max: 1e12, unit: '', description: "Doğrusal ve Üslü'de üyeliğin 0 olduğu değer; üst değerden büyükse üyelik azalır.", visibleWhen: ends_ },
    { name: 'high', label: 'Üst değer', type: 'number', default: 100, min: -1e12, max: 1e12, unit: '', description: "Doğrusal ve Üslü'de üyeliğin 1 olduğu değer.", visibleWhen: ends_ },
    {
      name: 'exponent',
      label: 'Üs',
      type: 'number',
      default: 2,
      min: 0.01,
      max: 100,
      unit: '',
      description: "Doğrusal üyelik bu üsse yükseltilir: 1'den büyükse yavaş, küçükse hızlı yükselir.",
      visibleWhen: (v: Shown) => fn(v) === 'power',
    },
    {
      name: 'midpoint',
      label: 'Orta nokta',
      type: 'number',
      default: 1,
      min: -1e12,
      max: 1e12,
      unit: '',
      description: "Gauss ve Yakın'da üyeliğin 1, Büyük ve Küçük'te 0,5 olduğu değer; Büyük ve Küçük'te 0'dan büyük.",
      visibleWhen: (v: Shown) => ['gaussian', 'large', 'small', 'near'].includes(fn(v)),
    },
    {
      name: 'spread',
      label: 'Yayılım',
      type: 'number',
      default: 0.1,
      min: 1e-9,
      max: 1e6,
      unit: '',
      description: 'Büyüdükçe üyelik orta noktadan uzaklaştıkça daha hızlı düşer.',
      visibleWhen: (v: Shown) => fn(v) === 'gaussian' || fn(v) === 'near',
    },
    {
      name: 'steep',
      label: 'Diklik',
      type: 'number',
      default: 5,
      min: 1e-9,
      max: 1e6,
      unit: '',
      description: 'Büyüdükçe üyelik orta noktada daha keskin değişir.',
      visibleWhen: (v: Shown) => fn(v) === 'large' || fn(v) === 'small',
    },
    SAMPLE,
    ...ends('-uyelik', 'Üyelik'),
  ],
  outputs: FILE,
  run: (v, ctx, feedback) =>
    runSuitRaster(
      v,
      ctx,
      feedback,
      {
        kind: 'fuzzyMembership',
        band: v.band ?? 1,
        function: v.function,
        low: v.low ?? 0,
        high: v.high ?? 100,
        exponent: v.exponent ?? 2,
        midpoint: v.midpoint ?? 1,
        spread: v.spread ?? 0.1,
        steep: v.steep ?? 5,
        sample: v.sample,
      },
      ['-uyelik', 'Üyelik hesaplanıyor'],
      true,
      () => '',
    ),
});

export const fuzzyOverlay = defineTool({
  id: 'suitability.fuzzyOverlay',
  label: 'Bulanık çakıştırma',
  category: SUITABILITY,
  icon: 'fuzzyOverlay',
  description: 'Üyelik rasterlerini bulanık mantığın işleciyle birleştirir: Ve, Veya, Çarpım, Toplam, Gamma.',
  help: [
    'Ve en küçük üyeliği, Veya en büyüğünü alır. Çarpım üyeliklerin çarpımıdır (her ölçüt sonucu düşürür); Toplam 1 − Π(1 − μ)\'dür (her ölçüt artırır). Gamma Toplam^γ · Çarpım^(1 − γ) (Bonham-Carter).',
    'Üyelik 0 ile 1 arasında olmalıdır: dışındaki değerin hücresi değersiz bırakılır ve söylenir.',
    HELP_GRID,
    HELP_OUTPUT,
  ].join('\n\n'),
  keywords: ['bulanık', 'fuzzy', 'çakıştırma', 'overlay', 'gamma', 'uygunluk'],
  aliases: ['BULANIKCAKISTIR', 'FUZZYOVERLAY'],
  targets: ['client'],
  parameters: [
    rasters('Rasterler', "Üyelik rasterleri (0–1), örneğin Bulanık üyelik'in sonuçları."),
    BAND,
    {
      name: 'op',
      label: 'İşleç',
      type: 'enum',
      options: [
        { value: 'and', label: 'Ve (en küçük)' },
        { value: 'or', label: 'Veya (en büyük)' },
        { value: 'product', label: 'Çarpım' },
        { value: 'sum', label: 'Toplam' },
        { value: 'gamma', label: 'Gamma' },
      ],
      default: 'and',
      description: 'Ve: bütün ölçütler birlikte; Veya: en az biri; Çarpım ve Toplam ölçütlerin hepsini katar; Gamma ikisinin arası.',
    },
    { name: 'gamma', label: 'Gamma', type: 'number', default: 0.9, min: 0, max: 1, unit: '', description: "Toplam^γ · Çarpım^(1 − γ): 1'de Toplam, 0'da Çarpım.", visibleWhen: (v: Shown) => v.op === 'gamma' },
    SAMPLE,
    ...ends('-bulanik', 'Bulanık çakıştırma'),
  ],
  outputs: FILE,
  run: (v, ctx, feedback) =>
    runSuitRaster(
      v,
      ctx,
      feedback,
      { kind: 'fuzzyOverlay', band: v.band ?? 1, op: v.op, gamma: v.gamma ?? 0.9, sample: v.sample },
      ['-bulanik', 'Bulanık çakıştırılıyor'],
      false,
      (n, k, fb) => {
        sayInvalid(n, fb);
        return joined(k);
      },
    ),
});

export const weightedSum = defineTool({
  id: 'suitability.weightedSum',
  label: 'Ağırlıklı toplam',
  category: SUITABILITY,
  icon: 'weightedSum',
  description: 'Rasterleri ağırlıklarıyla çarpıp toplar: Σ ağırlık · değer, ondalık sonuç.',
  help: [
    "Değerler yeniden ölçeklenmez: ölçütleri önce ortak bir ölçeğe getirin (Yeniden sınıflandır ya da Bulanık üyelik). Ağırlıklar İkili karşılaştırma'dan alınabilir.",
    HELP_ORDER,
    HELP_GRID,
    HELP_OUTPUT,
  ].join('\n\n'),
  keywords: ['ağırlıklı', 'toplam', 'weighted sum', 'uygunluk', 'çok ölçütlü', 'mcda'],
  aliases: ['AGIRLIKLITOPLAM', 'WEIGHTEDSUM'],
  targets: ['client'],
  parameters: [
    rasters('Rasterler', 'Toplanacak ölçüt rasterleri.'),
    BAND,
    {
      name: 'weights',
      label: 'Ağırlıklar',
      type: 'rasterValues',
      of: 'input',
      cell: 'number',
      min: -1e9,
      max: 1e9,
      placeholder: '1',
      default: {},
      description: 'Her rasterin çarpanı; boş bırakılanınki 1. Eksi ağırlık ölçütü tersine çevirir.',
    },
    SAMPLE,
    ...ends('-agirlikli', 'Ağırlıklı toplam'),
  ],
  outputs: FILE,
  run: (v, ctx, feedback) =>
    runSuitRaster(
      v,
      ctx,
      feedback,
      { kind: 'weightedSum', band: v.band ?? 1, weights: v.weights ?? {}, sample: v.sample },
      ['-agirlikli', 'Ağırlıklı toplam hesaplanıyor'],
      false,
      (_n, k) => joined(k),
    ),
});

export const weightedOverlay = defineTool({
  id: 'suitability.weightedOverlay',
  label: 'Ağırlıklı çakıştırma',
  category: SUITABILITY,
  icon: 'weightedOverlay',
  description: 'Rasterleri ortak bir ölçeğe sınıflayıp etkileriyle birleştirir; sonuç ölçeğin tam sayı sınıfıdır.',
  help: [
    'Her raster tablosuyla ölçeğin bir sınıfına çevrilir (tablosu yoksa değeri sınıftır); sonuç sınıfların etkileriyle ağırlıklı toplamıdır, tam sayılarla kesin hesaplanıp yarımlar sıfırdan uzağa yuvarlanır. Kısıt sınıflı bir raster hücreyi kısıtlı yapar (ölçeğin alt ucunun bir eksiği).',
    'Kuralı tutmayan, ölçeğin dışında ya da tam sayı olmayan değer hücreyi değersiz bırakır ve söylenir; değersiz kısıtlıdan önce gelir.',
    HELP_ORDER,
    HELP_GRID,
    HELP_OUTPUT,
  ].join('\n\n'),
  keywords: ['ağırlıklı', 'çakıştırma', 'weighted overlay', 'uygunluk', 'arazi sentezi', 'yerleşilebilirlik', 'çok ölçütlü'],
  aliases: ['AGIRLIKLICAKISTIR', 'WEIGHTEDOVERLAY'],
  targets: ['client'],
  parameters: [
    rasters('Rasterler', 'Ölçüt rasterleri: sınıfları ya da değerleri.'),
    BAND,
    { name: 'low', label: 'Ölçeğin alt ucu', type: 'number', default: 1, min: -1e6, max: 1e6, integer: true, unit: '', description: 'Ortak ölçeğin en küçük sınıfı; kısıtlı hücreler bunun bir eksiğini alır.' },
    { name: 'high', label: 'Ölçeğin üst ucu', type: 'number', default: 9, min: -1e6, max: 1e6, integer: true, unit: '', description: 'Ortak ölçeğin en büyük, en uygun sınıfı.' },
    {
      name: 'influence',
      label: 'Etki (%)',
      type: 'rasterValues',
      of: 'input',
      cell: 'number',
      min: 0,
      max: 100,
      placeholder: '%',
      default: {},
      description: 'Her rasterin etkisi, yüzde; en çok dört ondalık, toplamları tam 100.',
    },
    {
      name: 'classes',
      label: 'Sınıflar',
      type: 'rasterValues',
      of: 'input',
      cell: 'text',
      placeholder: '* 5 9; 5 15 6; 15 * kısıt',
      default: {},
      optional: true,
      description: 'Her rasterin tablosu: “alt üst yeni”, “değer yeni” ya da “boş yeni”; yeni değer ölçekte bir tam sayı, boş ya da kısıt. Tablosu boş rasterin değerleri ölçek değeridir.',
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
    ...ends('-cakistirma', 'Ağırlıklı çakıştırma'),
  ],
  outputs: FILE,
  run: (v, ctx, feedback) =>
    runSuitRaster(
      v,
      ctx,
      feedback,
      { kind: 'weightedOverlay', band: v.band ?? 1, low: v.low ?? 1, high: v.high ?? 9, influence: v.influence ?? {}, classes: v.classes ?? {}, bounds: v.bounds },
      ['-cakistirma', 'Ağırlıklı çakıştırılıyor'],
      false,
      (n, k, fb) => `${joined(k)}${sayOverlay(n, fb)}`,
    ),
});

const writes = (v: Shown): boolean => v.write !== false;

export const pairwise = defineTool({
  id: 'suitability.pairwise',
  label: 'İkili karşılaştırma (AHP)',
  category: SUITABILITY,
  icon: 'pairwise',
  description: "Ölçütleri ikişer ikişer karşılaştırıp ağırlıklarını ve tutarlılığını bulur (Saaty'nin AHP'si); isterseniz ağırlıklı toplamı yazar.",
  help: [
    'Her çift için önemli olan ve kaç kat önemli olduğu seçilir: 1 eşit, 3 biraz, 5 açıkça, 7 çok, 9 son derece önemli (2, 4, 6, 8 arası). Ağırlıklar karşılaştırma matrisinin baş özvektörüdür, toplamları 1.',
    "Tutarlılık oranı CR = CI / RI; CI = (λ − n) / (n − 1), RI Saaty'nin rastgele indeksi. CR 0,10'dan büyükse karşılaştırmalar tutarsızdır: en çelişkili çiftleri yeniden gözden geçirin.",
    HELP_ORDER,
    "Ağırlıklı toplam Ağırlıklı toplam aracınınkiyle aynıdır: ölçütleri önce ortak bir ölçeğe getirin.",
    HELP_OUTPUT,
  ].join('\n\n'),
  keywords: ['ahp', 'ikili karşılaştırma', 'pairwise', 'analitik hiyerarşi', 'ağırlık', 'saaty', 'tutarlılık'],
  aliases: ['AHP', 'IKILIKARSILASTIRMA'],
  targets: ['client'],
  parameters: [
    rasters('Ölçütler', 'Karşılaştırılan ölçütler: 2 ile 15 raster.'),
    {
      name: 'comparisons',
      label: 'Karşılaştırmalar',
      type: 'rasterPairs',
      of: 'input',
      default: [],
      description: "Her çift için hangisinin kaç kat önemli olduğu (Saaty'nin 1–9 ölçeği); seçilmeyen çift eşittir.",
    },
    { name: 'write', label: 'Ağırlıklı toplamı yaz', type: 'boolean', default: true, description: 'Açıkken ölçütler bu ağırlıklarla toplanıp raster yazılır; kapalıyken yalnız ağırlıklar.' },
    { ...BAND, visibleWhen: writes },
    { ...SAMPLE, visibleWhen: writes },
    { ...output('-ahp'), visibleWhen: writes },
    { ...ADD, visibleWhen: writes },
    { ...layer('Ağırlıklı toplam (AHP)'), visibleWhen: (v: Shown) => writes(v) && v.add !== false },
  ],
  outputs: [TABLE, ...FILE],
  run: (v, ctx, feedback) => runPairwise(v, ctx, feedback),
});

const SAMPLE_KINDS = ['point', ...AREA_KINDS] as const;
const samples = <N extends string, L extends string>(name: N, label: L, description: string) =>
  ({ name, label, type: 'features', kinds: SAMPLE_KINDS, scopes: ['layer', 'selection', 'visible', 'all'], description }) as const;

export const rocValidation = defineTool({
  id: 'suitability.roc',
  label: 'ROC ile doğrulama',
  category: SUITABILITY,
  icon: 'rocCurve',
  description: 'Rasterin bilinen olayları ne kadar iyi ayırdığını ölçer: ROC eğrisinin tablosu, eğrinin altındaki alan (AUC) ve en iyi eşik.',
  help: [
    'Örnekler hücrelerdir: nokta içinde olduğu hücreyi, alan merkezini içine aldığı hücreleri verir; bir hücre bir kez sayılır. Değersiz hücreye ya da rasterin dışına düşen örnek atlanır, söylenir.',
    "Eşik büyükten küçüğe inerken doğru pozitif oranı yanlış pozitif oranına (bütün hücrelerde alan oranına) karşı çıkar; tablo varlık değerlerinin eşiklerindedir (en çok 200). AUC bir varlık hücresinin değerinin karşılaştırma hücresininkinden büyük olma olasılığıdır (eşitler yarım): 0,5 rastgele, 1 kusursuz. En iyi eşik Youden'in J'sidir (doğru pozitif oranı eksi yanlış pozitif oranı en büyük).",
    HELP_EMPTY,
  ].join('\n\n'),
  keywords: ['roc', 'auc', 'doğrulama', 'validation', 'başarı eğrisi', 'success rate', 'duyarlılık'],
  aliases: ['ROC', 'AUC'],
  targets: ['client'],
  parameters: [
    oneRaster('Doğrulanacak raster: duyarlılık ya da uygunluk.'),
    BAND,
    samples('presence', 'Varlık', 'Bilinen olaylar: noktalar ya da alanlar (heyelan, sel …).'),
    {
      name: 'background',
      label: 'Karşılaştırma',
      type: 'enum',
      options: [
        { value: 'cells', label: 'Bütün hücreler' },
        { value: 'absence', label: 'Yokluk nesneleri' },
      ],
      default: 'cells',
      description: 'Bütün hücreler başarı eğrisini verir (alan oranı); yokluk nesneleri olayın olmadığı bilinen yerlerdir.',
    },
    { ...samples('absence', 'Yokluk', 'Olayın olmadığı bilinen noktalar ya da alanlar.'), visibleWhen: (v: Shown) => v.background === 'absence' },
    { name: 'higher', label: 'Yüksek değer daha olası', type: 'boolean', default: true, description: 'Kapalıyken düşük değer olayı daha olası gösterir.' },
  ],
  outputs: [TABLE, { name: 'auc', label: 'AUC', type: 'number' }],
  run: (v, ctx, feedback) => runRoc(v, ctx, feedback),
});

export const SUITABILITY_TOOLS = [fuzzyMembership, fuzzyOverlay, weightedSum, weightedOverlay, pairwise, rocValidation] as const;
