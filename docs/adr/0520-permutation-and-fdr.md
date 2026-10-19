# ADR 0520: Permütasyon çıkarımı, çoklu test düzeltmesi ve deney profili

- **Durum:** kabul edildi (2026-10-19). Madde plan/gis.md `CBS-25` (“Mevcut istatistiklere permutation/FDR ve tekrarlanabilir deney
  profili ekle”); web dururken (ADR 0228) yalnız masaüstünde, ortak çekirdekte ve `cad.processing.run` üzerinden Python ve MCP'de.
- **Bağlam belgesi:** ADR 0238 (Mekânsal istatistik: Moran I, Gi\*, komşuluk, `kentos.statistics/1`), ADR 0220 (tohumlu sayılar:
  SplitMix64 ve xoshiro256**), ADR 0508 (İşlemler'in araçları komut olarak), ADR 0510 (bağlı katmanın alan sınırlı çalışma kopyası).

## Bağlam

ADR 0238'in Moran I ve Sıcak nokta (Gi\*) araçları p değerini normal dağılım varsayımıyla (Cliff ve Ord'un rastgele dağıtım
varyansı, Ord ve Getis'in z'si) bulur; FDR düzeltmesi ve Anselin'in yerel Moran'ı (LISA) kapsam dışıydı. Bilimsel kullanımda (GeoDa,
PySAL/esda, ArcGIS Pro'nun Cluster and Outlier Analysis'i) p değeri çoğunlukla **permütasyonla** bulunur: değerler yerlere rastgele
dağıtılır, istatistiğin dağılımı varsayımsız kurulur. Yerel istatistikler her nesneyi ayrı sınadığından **çoklu test** sorunu vardır:
1 000 nesnede %5'le 50 nesne rastlantıyla anlamlı çıkar; ArcGIS “Apply False Discovery Rate (FDR) Correction”, GeoDa FDR ve
Bonferroni sunar. Permütasyon rastgele olduğundan sonuç **tohuma** bağlıdır: aynı sonucun her platformda yeniden üretilebilmesi
için tohum, permütasyon sayısı, komşuluk ve girdi kaydedilmelidir.

Araştırmada bulunanlar (19 Ekim):
- **esda** (PySAL) `Moran` ve `Moran_Local`, `G_Local`: global permütasyonda değerlerin tam bir karıştırması; yerelde **koşullu
  permütasyon** (nesnenin değeri yerinde, komşularınınkiler öbür n − 1 değerden yerine koymadan çekilir); esda'nın `crand`'i her
  permütasyon satırını bütün nesneler için yeniden kullanır, çekilen indeks nesnenin kendisini atlar. Sözde p **katlanmış** tek
  yanlıdır: `larger` = gözlenene eşit ya da büyük olanların sayısı; `R − larger < larger` ise `R − larger`; p = (larger + 1)/(R + 1).
- **Benjamini ve Hochberg (1995):** p'ler sıralanır, k'inci en küçüğün düzeltilmiş p'si min(1, min_{j≥k} p₍ⱼ₎·m/j); düzeltilmiş
  p ≤ q olanlar FDR'ı q'da tutar. Makalenin 15 p'lik örneğinde q = 0,05'te dört sınama reddedilir. **Bonferroni:** min(1, p·m).
- **Anselin (1995):** Iᵢ = (zᵢ / m₂) Σⱼ wᵢⱼ zⱼ, m₂ = Σ z² / n; çeyrekler Yüksek-Yüksek, Düşük-Düşük (küme), Yüksek-Düşük,
  Düşük-Yüksek (aykırı). GeoDa analitik varyans yerine yalnız koşullu permütasyonu kullanır.
- **Isı haritası** (ADR 0213) noktaların yoğunluğunu gösterir, hiçbir şey sınamaz; sıcak nokta bir değerin komşuluğunda
  beklenenden yüksek olup olmadığını sınar: ikisi aynı çıktı sayılmaz.

## Karar

### 1. Kapsam

- **Moran I** (`stats.moransI`): Çıkarım (Normal dağılım ya da Permütasyon), Permütasyon sayısı, Tohum.
- **Sıcak nokta (Gi\*)** (`stats.hotSpot`): Çıkarım (koşullu permütasyon), Permütasyon sayısı, Tohum, Çoklu test düzeltmesi
  (Yok, FDR, Bonferroni).
- **Yerel Moran (LISA)** (`stats.localMoran`), yeni araç: yalnız koşullu permütasyonla (GeoDa gibi; analitik yerel varyans yok),
  Çoklu test düzeltmesi, Anlamlılık düzeyi (%10, %5, %1).
- Varsayılanlar ADR 0238'inkiler kalır (Normal dağılım, düzeltme yok): eski çalıştırmalar aynı sonucu verir.
- **Kapsam dışı:** DBSCAN ve k-ortalamalar'ın kararlılık sınaması (yeniden örnekleme), En yakın komşu'nun permütasyonu, yerel
  Geary, iki değişkenli Moran, uzay-zaman permütasyonları, rastgele tohum (tohum her zaman yazılır; varsayılan 1).

### 2. Kural `kentos.spatial-inference/1`: sayılar ve çekilişler

- Bir çalıştırmanın tek dizisi: SplitMix64 tohumu dört 64 bitlik duruma yayar, xoshiro256** sözcükleri verir (ADR 0220'nin
  `Sequence`'i). m'den küçük indeks `below(m) = (sözcük × m) >> 64` (Lemire'nin çarpımı; reddetme yok, sapması m / 2⁶⁴'ten küçük).
- **Global** (Moran I): R permütasyon, her biri özdeşlikten (0 … n − 1) Fisher–Yates: i = n − 1'den 1'e, j = below(i + 1), takas.
  i'nci nesne `perm[i]`'nin değerini alır.
- **Koşullu** (Gi\*, Yerel Moran): R satır, her biri 0 … n − 2'den kmax çekiliş (yerine koymadan; özdeşlikten kısmi Fisher–Yates:
  t = 0 … kmax − 1, j = t + below(n − 1 − t)); kmax en kalabalık komşuluğun kendisi dışındaki komşu sayısı. i'nci nesne her satırın
  ilk kᵢ çekilişini okur; çekilen o, nesne o + (o ≥ i ? 1 : 0)'dır; çekilenler nesnenin komşu listesinin (komşuların sırasıyla)
  ağırlıklarıyla eşleşir. Satırlar bütün nesnelerce paylaşılır (esda'nın `crand`'i).
- **Karşılaştırma:** permütasyonun istatistiği, gözlenenden istatistiğin üst sınırının 10⁻⁹ katı kadar küçükse de “en az o kadar”
  sayılır (kesin aritmetikte eşit olanlar her platformda eşit sayılsın diye); üst sınır Moran I'da (n/S₀)·Σ|w|·max z²/Σz², yerelde
  |sabit| + |çarpan|·Σ|w|·max|v|.
- **Sözde p** katlanmış: larger, R − larger daha küçükse o; p = (larger + 1)/(R + 1) (float64'te bir bölme). **Sözde z** =
  (gözlenen − ortalama)/standart sapma (popülasyon sapması).
- **Yayılmayan dağılım:** permütasyonların standart sapması üst sınırın 10⁻⁹'undan küçükse p ve z yoktur (komşusu olmayan nesne,
  bütün öbür nesneleri eşit ağırlıkla komşusu olan nesne); nesne sınanmaz, sayısı söylenir.
- **Sınırlar:** R 9 … 99 999; R × (komşu çifti) en çok 4·10⁹ iş, R × kmax en çok 2,5·10⁷ çekiliş (≈100 MB); aşan ret, nedeniyle.

### 3. Moran I'nın permütasyonu

- Analitik satırlar (I, Beklenen I, Varyans, z, p) yazılmaya devam eder; Desen satırı seçilen çıkarımdan (permütasyon z'sinin
  işareti ve p'si, ADR 0238 §2'nin kuralıyla).
- Tabloya eklenenler: Permütasyon sayısı, Tohum, Permütasyon ortalaması (6 ondalık), Permütasyon z (4), Permütasyon p (6),
  **p'nin standart hatası** √(p(1 − p)/R) (belirsizlik: p bir Monte Carlo tahminidir).
- Özet: “Moran I 0.864645 (permütasyon z 6.2968, p 0.001000; 999 permütasyon, tohum 1): kümelenmiş.” Çıktılar: `zPermutation`,
  `pPermutation`, `profile`, `experiment`.

### 4. Gi\* ve çoklu test

- Permütasyonda nesnenin istatistiği wᵢᵢ(xᵢ − x̄) + Σⱼ≠ᵢ wᵢⱼ(x_π(j) − x̄) (payda nesne için sabit: sıralama z'ninkiyle aynı).
- Kopyalara eklenen alanlar: Permütasyon z, Permütasyon p (permütasyonda), Düzeltilmiş p (düzeltmede); Güven sınıfı **son p'den**
  (düzeltilmiş ya da seçilen çıkarımın p'si) ADR 0238'in eşikleriyle (p < 0,01, 0,05, 0,10), işaret analitik z'den: haritanın
  sınıfı tablodaki sayıyla tutar.
- **Düzeltme** yalnız p'si olan nesnelerin sayısıyla (m): Bonferroni min(1, p·m); Benjamini–Hochberg p'ler sıralanır (eşitlerde
  girdinin sırası), sondan başa q = p·m/(sıra) float64'te bu sırayla, en küçüğü taşınarak. Sınıf q < eşik.
- İletiler: “Güven sınıfları 999 koşullu permütasyonun p'sinden (tohum 1), FDR (Benjamini–Hochberg) düzeltmesiyle.”

### 5. Deney profili

Permütasyonlu ya da düzeltmeli her çalıştırma (Yerel Moran'da her zaman) bir profil yazar:

| Anahtar | Alan |
|---|---|
| `rule` | Kural (`kentos.spatial-inference/1`) |
| `tool` | Araç |
| `inference` | Çıkarım (Normal dağılım, Permütasyon, Koşullu permütasyon) |
| `permutations`, `seed`, `generator` | Permütasyon sayısı, Tohum, Sayı üreteci (`SplitMix64 + xoshiro256**`) |
| `correction` | Çoklu test düzeltmesi (Moran I'da “Yok (tek sınama)”) |
| `weights`, `standardize` | Komşuluk (kullanılan bantla), Satır standartlaştırma |
| `objects`, `field`, `alpha` | Nesne sayısı, Değer alanı, Anlamlılık düzeyi (Yerel Moran) |
| `scope` | Kapsam (§7) |
| `inputs` | Girdi özeti: SHA-256 |

- **Girdi özeti:** sınanan her nesne için bir satırın SHA-256'sı: `girdideki sırası \t doğu \t kuzey \t değer\n`; yer nesnenin
  merkezi 6 ondalıkla (`shape_centroid`), değer float64'ün en kısa yazımı (üssüz, Rust'ın `Display`'i). Aynı özet, aynı tohum ve
  aynı permütasyon sayısı aynı sonucu verir.
- Çıktılar: `profile` (Alan, Değer tablosu; pencerede gösterilir, özetin ilk 16 hanesiyle) ve `experiment` (aynısı anahtarlarla,
  profilin sırasında bir JSON nesnesinin metni; tam özetle). Python ve MCP `cad.processing.run`'ın çıktılarından okur.

### 6. Yerel Moran (LISA)

- **Hesap:** z = x − x̄, m₂ = Σ z²/n; komşuluk Moran I'nınki (kendisi komşu değil, satır standartlaştırma varsayılan açık; komşusu
  olmayan nesne sınanmaz); gecikme Σⱼ wᵢⱼ zⱼ; Iᵢ = (zᵢ/m₂)·gecikme.
- **Çeyrek:** zᵢ ≥ 0 Yüksek, gecikme ≥ 0 Yüksek: YY, DD, YD, DY.
- **Küme türü:** son p < Anlamlılık düzeyi ise çeyreğin adı (Yüksek-Yüksek, Düşük-Düşük, Yüksek-Düşük, Düşük-Yüksek), değilse
  “Anlamlı değil”, p'si olmayan “Sınanamadı”.
- **Kopyalar:** öznitelikler Yerel Moran I (6 ondalık), Çeyrek, Permütasyon z, Permütasyon p, (Düzeltilmiş p), Küme türü; renkleri
  YY #D7191C, DD #2C7BB6, YD #FDAE61, DY #ABD9E9, Anlamlı değil #D9D9D9, Sınanamadı #8C8C8C; katman “Yerel Moran” `[Küme türü]`'ne
  göre kategorili, girdinin katmanının hemen üstünde.
- Özet: “29 nesne yazıldı: 5 Yüksek-Yüksek, 5 Düşük-Düşük, 0 Yüksek-Düşük, 0 Düşük-Yüksek (p < 0.05).” Çıktılar: `highHigh`,
  `lowLow`, `highLow`, `lowHigh`, `count`, `profile`, `experiment`. En az dört nesne; permütasyonsuz çalışmaz.

### 7. Alt küme, parça ve sıfır komşu

- Girdi seçim, görünen nesneler ya da kimliklerle verilmişse uyarı: “Girdi bir alt küme (seçili nesneler): istatistik yalnız bu 18
  nesnenin; dışarıda kalan komşular hesaba girmedi.” ve profilin Kapsam'ı “Seçili nesneler (alt küme)”; katmanın bütünü “Katman: ad
  (bütün nesneleri)”. Görünenler bir harita parçasıdır: tam veri kümesinin istatistiği gibi sunulmaz.
- Girdinin katmanı alan sınırlı bağlı kaynaksa (ADR 0510'un `extent`'i) uyarı ve Kapsam'a “; kaynağın alan sınırlı çalışma kopyası”.
- Komşu listeleri seyrektir (k-d ağacından, ADR 0238 §7); sıfır komşulu nesne Moran I'da satırı 0 (ADR 0238), yerelde sınanmaz.

### 8. Performans

- Hesap çekirdekte (`ops::spatial_stats::inference`, `local`); masaüstünde İşlemler'in iş parçacığında. Permütasyonlar O(R·(n +
  çiftler)); koşullu satırlar bir kez çekilir.
- **Bütçeler** (release, geliştirme makinesi, 100 000 nokta, ~10 komşu, 999 permütasyon): Moran I ≤ 3 s, Sıcak nokta FDR'la ≤ 3 s,
  Yerel Moran FDR'la ≤ 3 s.

## Uygulama

- **Çekirdek** (`crates/shared/geometry-core/src/ops/spatial_stats/`): `inference` (kural, `Inference`, `Correction`, `below`,
  `each_permutation`, `conditional_rows`, `folded_p`, `moments`, `adjust`, `Profile`, `input_line`), `local` (`Draws`,
  `hot_spots_with`, `local_morans`, `LISA_TYPES`), `autocorrelation` (`morans_i_with`, `profile_of`); `StatsRun.profile`;
  web'in çağrıları `statsMoranWith`, `statsHotSpotsWith`, `statsLocalMoran` (`calls`).
- **Masaüstü** (`kentos-processing`'in `builtin/stats/`): parametreler Çıkarım, Permütasyon sayısı, Tohum, Çoklu test düzeltmesi,
  Anlamlılık düzeyi; `scope_of`, `tested_result_of` (alt küme uyarısı, profil tablosu, SHA-256), `lisa_renderer`; yeni araç
  `stats.localMoran`. Arayüz kaydı `catalog/v1/additions.json` (`processing.run.stats.localMoran`, ikon `statsLocalMoran`, CBS
  Analiz panelinin ▾'i); manifest `docs/cad/manifest.*`.
- Python ve MCP: araçlar `cad.processing.run` ile (ADR 0508); katalog değişmedi.

## Doğrulama

- **Bağımsız başvuru** `scripts/fixtures/spatial_inference_cases.py` (KentOS kodu ve paket yok: `fractions`, `decimal`, `hashlib`):
  yerler, uzaklıklar, ağırlıklar (ters uzaklık 60 basamaklı kökle), sapmalar ve gözlenen ve her permütasyonun istatistiği kesir;
  karekök ve erfc 60 basamak (π ve erf serisi); sayılar ve çekilişler kuralın kendisi Python'un tam sayılarıyla. Gözlenene eşit
  olmayan ama üst sınırın 10⁻⁷'si kadar yakın permütasyon, sınıra yakın metin ve eşiğe yakın analitik düzeltilmiş p reddedilir.
  Kendi denetimleri: beş değerin 120 sıralamasının tam permütasyon p'si 9 999 çekilişin p'siyle binom hatası içinde; 9 999
  permütasyonun I ortalaması ADR 0238'in E[I]'sıyla; SplitMix64'ün yayımlanmış ilk sözcüğü (0xE220A8397B1DCDAF).
  PySAL/esda bu makinede kurulu değil; kural esda'nınkiyle aynı tanımdır (sayı üreteci hariç).
- **İşlemler'in ortak durumları** `fixtures/processing/v1/spatial-inference.json`, 13 durum (Moran I üç komşulukla, Gi\* dört,
  Yerel Moran dört, alt küme, ret, geçersiz değer), çizimde ve okuma kopyasında; ADR 0238'in 18 durumu değişmeden geçer.
- **Çekirdek birim testleri:** Benjamini ve Hochberg'in makalesindeki 15 p (dört ret), Bonferroni, çekilişlerin permütasyon ve
  ayrık olması, katlanmış p, sayı üretecinin ilk sözcüğü.
- **Masaüstü:** pencerenin testi (Yerel Moran'ın türleri, profili, aynı tohumla aynı p'ler), resimler `tools_screens`'in
  `ist-moran-permutasyon`, `ist-sicak-fdr`, `ist-sicak-fdr-cizim`, `ist-lisa`, `ist-lisa-profil`, `ist-lisa-cizim`'i (iki tema, iki boy).
- **Süreler** (release, geliştirme makinesi, 100 000 nokta, ~10 komşu, 999 permütasyon): Moran I 1,60 s; Sıcak nokta (FDR) 1,01 s;
  Yerel Moran (FDR) 0,95 s.

## Web'e ertelenen

Web'in Moran I ve Sıcak nokta araçlarına Çıkarım, Permütasyon sayısı, Tohum, Çoklu test düzeltmesi; Yerel Moran aracı
(`processing/builtin/stats/`), çekirdeğin `statsMoranWith`, `statsHotSpotsWith`, `statsLocalMoran` çağrılarıyla; deney profili tablosu
ve `experiment`; Analiz panelinin ▾'inde Yerel Moran; web'in permütasyon süreleri WASM'la.
