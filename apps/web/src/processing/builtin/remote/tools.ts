import { defineTool, type Shown } from '../../types';
import { ADD, AREA_KINDS, layer, output } from '../rasterOps/shared';
import { REMOTE, objectsWith, runAccuracy, runRemoteRaster, runSplit } from './shared';

/**
 * Uzaktan algılama (docs/adr/0242 §3–§10): the eight tools, their parameters as the desktop's
 * (`builtin/remote/tools.rs`), their settings handed to the raster core's operation job.
 */

const HELP_EMPTY = "Rasterin nodata'sı, NaN ve alfası 0 olan pikseller değersizdir.";
const HELP_OUTPUT =
  "Sonuç karolu, Deflate'li ve önizleme katlı GeoTIFF'tir; çıktı dosyası boşsa ilk rasterin adının sonuna ek konur. Web'de 32 MB'a kadar olan sonuç projeye gömülür, büyüğü indirilir ve bu oturumda bağlı kalır. Çizime ekle açıksa raster ilk rasterin katmanının hemen üstündeki yeni katmana eklenir.";

const FILE = { name: 'file', label: 'Sonuç dosyası', type: 'string' } as const;
const TABLE = { name: 'table', label: 'Tablo', type: 'table' } as const;

const raster = <N extends string, L extends string>(name: N, label: L, description: string) =>
  ({ name, label, type: 'features', kinds: ['raster'], scopes: ['selection', 'layer'], description }) as const;
const band = <N extends string, L extends string>(name: N, label: L, value: number, description: string) =>
  ({ name, label, type: 'number', default: value, min: 1, max: 255, integer: true, unit: '', description }) as const;
const ends = <S extends string, N extends string>(suffix: S, name: N) => [output(suffix), ADD, layer(name)] as const;
const objects = <N extends string, L extends string, K extends readonly string[]>(name: N, label: L, kinds: K, description: string) =>
  ({ name, label, type: 'features', kinds, scopes: ['layer', 'selection', 'visible', 'all'], description }) as const;

export const composite = defineTool({
  id: 'remote.composite',
  label: 'Bant birleştir',
  category: REMOTE,
  icon: 'bandComposite',
  description: 'Rasterlerin bantlarını tek çok bantlı rasterde birleştirir: her rasterin bütün bantları sırayla sonucun bantları olur.',
  help: [
    'Rasterler Katmanlar panelinde üstten aşağı sırayla (aynı katmanda sonra eklenen önce) birleşir; tabloda her bandın kaynağı yazılır.',
    'Rasterler ortak alanlarında, hücresi en küçük olanın ızgarasında birleşir: sonucun hücreleri merkezi bütün rasterlerin içinde olanlardır. Örnekleme en yakın hücre ya da çift doğrusal (dört komşu hücre merkezi).',
    'Rasterlerin hepsi aynı türdeyse sonuç o türde (ve aynı nodata ile), değilse ondalık 32 bit. Üç ve daha çok bantta görünüş renkli (1, 2, 3).',
    HELP_EMPTY,
    HELP_OUTPUT,
  ].join('\n\n'),
  keywords: ['bant', 'birleştir', 'composite', 'kompozit', 'çok bantlı', 'multiband', 'stack'],
  aliases: ['BANTBIRLESTIR', 'COMPOSITEBANDS'],
  targets: ['client'],
  parameters: [
    { name: 'input', label: 'Rasterler', type: 'features', kinds: ['raster'], scopes: ['visible', 'selection', 'layer', 'all'], description: 'Birleştirilecek rasterler (en az iki).' },
    {
      name: 'sampling',
      label: 'Örnekleme',
      type: 'enum',
      options: [
        { value: 'nearest', label: 'En yakın' },
        { value: 'bilinear', label: 'Çift doğrusal' },
      ],
      default: 'nearest',
      description: 'Rasterler sonucun ızgarasına böyle okunur; en yakın değerleri değiştirmez.',
    },
    ...ends('-birlesik', 'Bant birleştir'),
  ],
  outputs: [TABLE, FILE],
  run: (v, ctx, feedback) =>
    runRemoteRaster(v, ctx, feedback, { kind: 'composite', sampling: v.sampling }, [false, null], [], ['-birlesik', 'Bantlar birleştiriliyor']),
});

export const split = defineTool({
  id: 'remote.split',
  label: 'Bantlara ayır',
  category: REMOTE,
  icon: 'bandSplit',
  description: 'Çok bantlı rasterin her bandını ayrı bir rastere yazar.',
  help: [
    'Her bant kendi GeoTIFF\'ine yazılır: çıktı dosyası boşsa rasterin yanına adının sonuna “-b1”, “-b2” … eklenerek; bir ad yazılırsa o adın sonuna. Örnek türü ve nodata rasterinkidir; alfa bandı ayrılmaz.',
    'Çizime ekle açıksa bantlar sırayla rasterin katmanının hemen üstündeki yeni katmana eklenir; görünüşleri gri, %2–98 gerdirmeyle.',
  ].join('\n\n'),
  keywords: ['bant', 'ayır', 'split', 'band', 'tek bant'],
  aliases: ['BANTAYIR', 'SPLITBANDS'],
  targets: ['client'],
  parameters: [raster('input', 'Raster', 'Bantlarına ayrılacak raster.'), ...ends('-b', 'Bantlar')],
  outputs: [TABLE],
  run: (v, ctx, feedback) => runSplit(v, ctx, feedback),
});

const index = (v: Shown): string => String(v.index ?? 'ndvi');
const reads = (...uses: string[]) => (v: Shown): boolean => uses.includes(index(v));

export const spectralIndex = defineTool({
  id: 'remote.index',
  label: 'Spektral indis',
  category: REMOTE,
  icon: 'spectralIndex',
  description: 'Bantlardan bitki, su ve yapılaşma indisi hesaplar: NDVI, GNDVI, SAVI, EVI, NDWI, MNDWI, NDBI, oran ya da normalize fark.',
  help: [
    'NDVI = (YKÖ − K) / (YKÖ + K); GNDVI = (YKÖ − Y) / (YKÖ + Y); SAVI = (1 + L)(YKÖ − K) / (YKÖ + K + L); EVI = G(YKÖ − K) / (YKÖ + C₁K − C₂M + L); NDWI = (Y − YKÖ) / (Y + YKÖ); MNDWI = (Y − KDK) / (Y + KDK); NDBI = (KDK − YKÖ) / (KDK + YKÖ); oran A / B; normalize fark (A − B) / (A + B). M mavi, Y yeşil, K kırmızı, YKÖ yakın kızılötesi, KDK kısa dalga kızılötesi.',
    "Her bant önce yansımaya çevrilir: ρ = sayı × ölçek + öteleme (Sentinel-2 L2A'da ölçek 0,0001; Landsat Collection 2 L2'de 0,0000275 ve −0,2). EVI yansıma ister.",
    'Hesap 64 bitte, sonuç 32 bit ondalıktır; bölenin sıfır olduğu hücre değersiz kalır, söylenir.',
    HELP_EMPTY,
    HELP_OUTPUT,
  ].join('\n\n'),
  keywords: ['ndvi', 'indis', 'index', 'bitki', 'vegetation', 'ndwi', 'su', 'ndbi', 'yapılaşma', 'savi', 'evi', 'bant aritmetiği'],
  aliases: ['INDIS', 'NDVI'],
  targets: ['client'],
  parameters: [
    raster('input', 'Raster', 'Çok bantlı görüntü (Landsat, Sentinel-2, ortofoto).'),
    {
      name: 'index',
      label: 'İndis',
      type: 'enum',
      options: [
        { value: 'ndvi', label: 'NDVI', hint: 'bitki örtüsü' },
        { value: 'gndvi', label: 'GNDVI', hint: 'yeşil bantla bitki' },
        { value: 'savi', label: 'SAVI', hint: 'toprak etkisi azaltılmış bitki' },
        { value: 'evi', label: 'EVI', hint: 'yoğun bitki örtüsü; yansıma ister' },
        { value: 'ndwi', label: 'NDWI', hint: 'açık su' },
        { value: 'mndwi', label: 'MNDWI', hint: 'su; yapı gölgesini ayırır' },
        { value: 'ndbi', label: 'NDBI', hint: 'yapılaşma' },
        { value: 'ratio', label: 'Oran', hint: 'A / B' },
        { value: 'normalized', label: 'Normalize fark', hint: '(A − B) / (A + B)' },
      ],
      default: 'ndvi',
      description: 'Hesaplanacak indis.',
    },
    { ...band('blue', 'Mavi bant', 1, "Mavi bandın numarası (1'den)."), visibleWhen: reads('evi') },
    { ...band('green', 'Yeşil bant', 2, "Yeşil bandın numarası (1'den)."), visibleWhen: reads('gndvi', 'ndwi', 'mndwi') },
    { ...band('red', 'Kırmızı bant', 3, "Kırmızı bandın numarası (1'den)."), visibleWhen: reads('ndvi', 'savi', 'evi') },
    { ...band('nir', 'Yakın kızılötesi bandı', 4, "Yakın kızılötesi bandın numarası (1'den)."), visibleWhen: reads('ndvi', 'gndvi', 'savi', 'evi', 'ndwi', 'ndbi') },
    { ...band('swir', 'Kısa dalga kızılötesi bandı', 5, "Kısa dalga kızılötesi bandın numarası (1'den)."), visibleWhen: reads('mndwi', 'ndbi') },
    { ...band('a', 'A bandı', 4, "Oranın payı, normalize farkın ilk bandı (1'den)."), visibleWhen: reads('ratio', 'normalized') },
    { ...band('b', 'B bandı', 3, "Oranın paydası, normalize farkın ikinci bandı (1'den)."), visibleWhen: reads('ratio', 'normalized') },
    { name: 'saviL', label: 'L', type: 'number', default: 0.5, min: 0, max: 10, unit: '', description: "SAVI'nin toprak düzeltmesi: yoğun bitkide 0, seyrekte 1.", visibleWhen: reads('savi') },
    { name: 'eviG', label: 'G', type: 'number', default: 2.5, min: 0, max: 100, unit: '', advanced: true, description: "EVI'nin kazancı.", visibleWhen: reads('evi') },
    { name: 'eviC1', label: 'C₁', type: 'number', default: 6, min: 0, max: 100, unit: '', advanced: true, description: "EVI'de kırmızının aerosol katsayısı.", visibleWhen: reads('evi') },
    { name: 'eviC2', label: 'C₂', type: 'number', default: 7.5, min: 0, max: 100, unit: '', advanced: true, description: "EVI'de mavinin aerosol katsayısı.", visibleWhen: reads('evi') },
    { name: 'eviL', label: "EVI'nin L'si", type: 'number', default: 1, min: 0, max: 100, unit: '', advanced: true, description: "EVI'nin örtü düzeltmesi.", visibleWhen: reads('evi') },
    {
      name: 'scale',
      label: 'Ölçek',
      type: 'number',
      default: 1,
      min: -1e9,
      max: 1e9,
      unit: '',
      advanced: true,
      description: 'Yansıma = sayı × ölçek + öteleme (Sentinel-2 L2A: 0,0001; Landsat C2 L2: 0,0000275).',
    },
    { name: 'offset', label: 'Öteleme', type: 'number', default: 0, min: -1e9, max: 1e9, unit: '', advanced: true, description: 'Yansıma = sayı × ölçek + öteleme (Landsat C2 L2: −0,2).' },
    ...ends('-indis', 'Spektral indis'),
  ],
  outputs: [FILE],
  run: (v, ctx, feedback) =>
    runRemoteRaster(
      v,
      ctx,
      feedback,
      {
        kind: 'index',
        index: v.index,
        bands: { blue: v.blue ?? 1, green: v.green ?? 2, red: v.red ?? 3, nir: v.nir ?? 4, swir: v.swir ?? 5, a: v.a ?? 4, b: v.b ?? 3 },
        scale: v.scale ?? 1,
        offset: v.offset ?? 0,
        saviL: v.saviL ?? 0.5,
        g: v.eviG ?? 2.5,
        c1: v.eviC1 ?? 6,
        c2: v.eviC2 ?? 7.5,
        eviL: v.eviL ?? 1,
      },
      [true, null],
      [],
      ['-indis', 'İndis hesaplanıyor'],
    ),
});

export const supervised = defineTool({
  id: 'remote.supervised',
  label: 'Denetimli sınıflandırma',
  category: REMOTE,
  icon: 'classifySupervised',
  description: 'Eğitim alanlarından öğrenip görüntünün her hücresini bir sınıfa atar: en büyük olabilirlik ya da en yakın ortalama.',
  help: [
    'Sınıflar eğitim alanlarının Sınıf alanındaki metinlerdir (kırpılır; boş olanlar alınmaz), doğal sırayla 1, 2, … değerlerini alır. Bir sınıfın eğitim hücreleri merkezi o sınıfın alanlarından birinin içinde olan ve hiçbir bandı değersiz olmayan hücrelerdir.',
    'En büyük olabilirlik her sınıfı ortalaması ve kovaryansıyla çok değişkenli normal dağılım sayar (eşit önsel olasılık): bir sınıfın en az bant sayısı + 1 eğitim hücresi olmalı, kovaryansı tekil olmamalı. En yakın ortalama hücreyi ortalamasına en yakın sınıfa verir.',
    'Sonuç 8 bit tam sayı (255 sınıftan çoğunda 16 bit), 0 değersiz; tabloda her sınıfın eğitim hücresi, hücre sayısı ve alanı.',
    HELP_EMPTY,
    HELP_OUTPUT,
  ].join('\n\n'),
  keywords: ['sınıflandırma', 'classification', 'denetimli', 'supervised', 'maximum likelihood', 'en büyük olabilirlik', 'arazi örtüsü', 'eğitim'],
  aliases: ['DENETIMLI', 'MAXLIKELIHOOD'],
  targets: ['client'],
  parameters: [
    raster('input', 'Raster', 'Sınıflandırılacak çok bantlı görüntü.'),
    objects('training', 'Eğitim alanları', AREA_KINDS, 'Sınıfları örnekleyen alanlar.'),
    { name: 'classField', label: 'Sınıf alanı', type: 'field', of: 'training', description: 'Eğitim alanının sınıfının adı (metin).' },
    {
      name: 'method',
      label: 'Yöntem',
      type: 'enum',
      options: [
        { value: 'likelihood', label: 'En büyük olabilirlik', hint: 'sınıfın ortalaması ve yayılımıyla (kovaryans)' },
        { value: 'distance', label: 'En yakın ortalama', hint: 'sınıfın ortalamasına en yakın' },
      ],
      default: 'likelihood',
      description: 'Hücrenin hangi sınıfa verileceği.',
    },
    ...ends('-siniflar', 'Denetimli sınıflandırma'),
  ],
  outputs: [TABLE, FILE],
  run: (v, ctx, feedback) => {
    const { shapes, texts } = objectsWith(v.training, v.classField);
    return runRemoteRaster(v, ctx, feedback, { kind: 'supervised', method: v.method, texts }, [true, null], shapes, ['-siniflar', 'Sınıflandırılıyor']);
  },
});

export const unsupervised = defineTool({
  id: 'remote.unsupervised',
  label: 'Denetimsiz sınıflandırma',
  category: REMOTE,
  icon: 'classifyUnsupervised',
  description: 'Görüntünün hücrelerini benzer spektral değerlerine göre kümelere ayırır (k-ortalamalar).',
  help: [
    "Kümeler hücrelerin bir örneğinden bulunur: hiçbir bandı değersiz olmayan, sütunu ve satırı s'nin katı olan hücreler (s, yaklaşık 250 000 hücre kalacak biçimde). Başlangıç merkezleri bantların ortalaması ± standart sapması boyunca dizilir; her yinelemede hücreler en yakın merkeze, merkezler üyelerinin ortalamasına gider; atamalar değişmeyince ya da en çok yinelemede durur.",
    'Kümeler merkezlerinin bant toplamına göre küçükten büyüğe 1 … k değerlerini alır; bütün hücreler en yakın merkeze atanır. Sonuç 8 bit, 0 değersiz; tabloda her kümenin hücre sayısı, alanı ve merkezi.',
    HELP_EMPTY,
    HELP_OUTPUT,
  ].join('\n\n'),
  keywords: ['kümeleme', 'clustering', 'denetimsiz', 'unsupervised', 'k-means', 'isodata', 'iso cluster', 'sınıflandırma'],
  aliases: ['DENETIMSIZ', 'ISOCLUSTER'],
  targets: ['client'],
  parameters: [
    raster('input', 'Raster', 'Kümelere ayrılacak çok bantlı görüntü.'),
    { name: 'clusters', label: 'Küme sayısı', type: 'number', default: 8, min: 2, max: 50, integer: true, unit: '', description: 'Bulunacak küme sayısı.' },
    { name: 'iterations', label: 'En çok yineleme', type: 'number', default: 20, min: 1, max: 100, integer: true, unit: '', description: 'Atamalar değişmeyince ya da bu kadar yinelemeden sonra durur.' },
    ...ends('-kumeler', 'Denetimsiz sınıflandırma'),
  ],
  outputs: [TABLE, FILE],
  run: (v, ctx, feedback) =>
    runRemoteRaster(v, ctx, feedback, { kind: 'unsupervised', clusters: v.clusters ?? 8, iterations: v.iterations ?? 20 }, [true, null], [], ['-kumeler', 'Kümeleniyor']),
});

export const accuracy = defineTool({
  id: 'remote.accuracy',
  label: 'Doğruluk analizi',
  category: REMOTE,
  icon: 'accuracyMatrix',
  description: 'Sınıflandırılmış rasteri referans nesneleriyle karşılaştırır: karışıklık matrisi, genel doğruluk, üretici ve kullanıcı doğruluğu, kappa.',
  help: [
    'Nokta düştüğü hücreyi, alan merkezini içine aldığı hücreleri verir; her nesnenin hücreleri ayrı sayılır. Referans alanının değeri tam sayı olarak okunur (sınıfın değeri); okunamayan nesne ve değersiz ya da rasterin dışındaki hücre alınmaz, söylenir.',
    'Matrisin satırları sınıflandırılan, sütunları referans değerlerdir. Kullanıcı doğruluğu satırın, üretici doğruluğu sütunun köşegendeki payıdır; genel doğruluk köşegenin toplamdaki payı; kappa (Cohen) şansla uyuşmanın ötesindeki uyum: (pₒ − pₑ) / (1 − pₑ).',
  ].join('\n\n'),
  keywords: ['doğruluk', 'accuracy', 'karışıklık matrisi', 'confusion matrix', 'kappa', 'hata matrisi', 'doğrulama'],
  aliases: ['DOGRULUK', 'CONFUSIONMATRIX'],
  targets: ['client'],
  parameters: [
    raster('input', 'Raster', 'Sınıflandırılmış raster.'),
    band('band', 'Bant', 1, "Sınıfların okunduğu bant (1'den)."),
    objects('reference', 'Referans nesneleri', ['point', ...AREA_KINDS] as const, 'Sınıfı bilinen noktalar ya da alanlar.'),
    { name: 'referenceField', label: 'Referans alanı', type: 'field', of: 'reference', description: 'Nesnenin gerçek sınıfının değeri (tam sayı).' },
  ],
  outputs: [TABLE, { name: 'overall', label: 'Genel doğruluk (%)', type: 'number' }, { name: 'kappa', label: 'Kappa', type: 'number' }],
  run: (v, ctx, feedback) => runAccuracy(v, ctx, feedback),
});

export const change = defineTool({
  id: 'remote.change',
  label: 'Değişim tespiti',
  category: REMOTE,
  icon: 'changeDetect',
  description: 'İki tarihin rasterlerini karşılaştırır: fark, oran, normalize fark ya da sınıf değişimi.',
  help: [
    'Fark sonraki − önceki, oran sonraki / önceki, normalize fark (sonraki − önceki) / (sonraki + önceki); sonuç 32 bit ondalık, bölenin sıfır olduğu hücre değersiz. Özette artan, azalan ve değişmeyen hücreler sayılır.',
    "Sınıf değişimi sınıf rasterleri içindir: değer önceki × 1000 + sonraki (örneğin 2003: 2'den 3'e), 0 değerli ya da değersiz hücre değersiz; tablo “neden neye” matrisidir, özet değişen hücrelerin payı.",
    'Rasterler ortak alanlarında, hücresi en küçük olanın ızgarasında, en yakın hücreleriyle okunur.',
    HELP_EMPTY,
    HELP_OUTPUT,
  ].join('\n\n'),
  keywords: ['değişim', 'change detection', 'fark', 'difference', 'zaman', 'karşılaştırma', 'sınıf değişimi'],
  aliases: ['DEGISIM', 'CHANGEDETECTION'],
  targets: ['client'],
  parameters: [
    raster('input', 'Önceki raster', 'Önceki tarihin rasteri.'),
    raster('after', 'Sonraki raster', 'Sonraki tarihin rasteri.'),
    band('band', 'Bant', 1, "Karşılaştırılan bant (1'den); iki rasterde de aynı bant."),
    {
      name: 'method',
      label: 'Yöntem',
      type: 'enum',
      options: [
        { value: 'difference', label: 'Fark', hint: 'sonraki − önceki' },
        { value: 'ratio', label: 'Oran', hint: 'sonraki / önceki' },
        { value: 'normalized', label: 'Normalize fark', hint: '(sonraki − önceki) / (sonraki + önceki)' },
        { value: 'classes', label: 'Sınıf değişimi', hint: 'sınıf rasterleri: önceki × 1000 + sonraki' },
      ],
      default: 'difference',
      description: 'İki tarihin nasıl karşılaştırılacağı.',
    },
    ...ends('-degisim', 'Değişim'),
  ],
  outputs: [TABLE, FILE],
  run: (v, ctx, feedback) =>
    runRemoteRaster(v, ctx, feedback, { kind: 'change', band: v.band ?? 1, method: v.method }, [true, ['after', 'Sonraki raster']], [], ['-degisim', 'Değişim hesaplanıyor']),
});

const brovey = (v: Shown): boolean => String(v.method ?? 'brovey') === 'brovey';

export const pansharpen = defineTool({
  id: 'remote.pansharpen',
  label: 'Görüntü birleştirme',
  category: REMOTE,
  icon: 'pansharpen',
  description: 'Çok bantlı görüntüyü pankromatik görüntünün ince çözünürlüğüne taşır (pansharpening): ağırlıklı Brovey ya da basit ortalama.',
  help: [
    'Çok bantlı görüntü pankromatiğin ızgarasına seçilen örneklemeyle okunur. Brovey: her bant × PAN / Σ wⱼ·bantⱼ (toplam sıfır ya da eksiyse 0); ağırlıklar boşsa eşittir (1/n). Basit ortalama: (bant + PAN) / 2.',
    'Sonuç pankromatiğin ızgarasında, çok bantlının örnek türündedir (tam sayıda en yakın tam sayıya yuvarlanır, türün aralığında tutulur); görünüşü çok bantlınınki.',
    'Ağırlıkları noktalı virgül ya da boşlukla ayırarak yazın (0,1; 0,3; 0,3; 0,3); sayısı çok bantlının bant sayısı olmalı.',
    HELP_EMPTY,
    HELP_OUTPUT,
  ].join('\n\n'),
  keywords: ['pansharpening', 'pankromatik', 'görüntü birleştirme', 'fusion', 'brovey', 'çözünürlük', 'keskinleştirme'],
  aliases: ['PANSHARPEN', 'FUSION'],
  targets: ['client'],
  parameters: [
    raster('input', 'Çok bantlı raster', 'Renkli ya da çok bantlı görüntü (kaba çözünürlük).'),
    raster('pan', 'Pankromatik raster', 'Tek bantlı, ince çözünürlüklü görüntü.'),
    {
      name: 'method',
      label: 'Yöntem',
      type: 'enum',
      options: [
        { value: 'brovey', label: 'Brovey', hint: 'ağırlıklı; renkleri pankromatiğin parlaklığıyla ölçekler' },
        { value: 'mean', label: 'Basit ortalama', hint: 'bant ile pankromatiğin ortalaması' },
      ],
      default: 'brovey',
      description: 'Bantların pankromatikle nasıl birleşeceği.',
    },
    {
      name: 'weights',
      label: 'Ağırlıklar',
      type: 'string',
      default: '',
      allowEmpty: true,
      optional: true,
      placeholder: 'eşit',
      description: 'Bantların ağırlıkları, noktalı virgülle ayrılmış (0,1; 0,3; 0,3; 0,3); boşsa eşit.',
      visibleWhen: brovey,
    },
    {
      name: 'sampling',
      label: 'Örnekleme',
      type: 'enum',
      options: [
        { value: 'cubic', label: 'Kübik' },
        { value: 'bilinear', label: 'Çift doğrusal' },
        { value: 'nearest', label: 'En yakın' },
      ],
      default: 'cubic',
      description: 'Çok bantlı görüntü pankromatiğin ızgarasına böyle okunur.',
    },
    ...ends('-birlesim', 'Görüntü birleştirme'),
  ],
  outputs: [FILE],
  run: (v, ctx, feedback) =>
    runRemoteRaster(
      v,
      ctx,
      feedback,
      { kind: 'pansharpen', method: v.method, weights: v.weights ?? '', sampling: v.sampling },
      [true, ['pan', 'Pankromatik raster']],
      [],
      ['-birlesim', 'Görüntüler birleştiriliyor'],
    ),
});

export const REMOTE_TOOLS = [composite, split, spectralIndex, supervised, unsupervised, accuracy, change, pansharpen] as const;
