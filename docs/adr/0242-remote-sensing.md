# ADR 0242: Uzaktan algılama

- **Durum:** kabul edildi (2026-10-10). Kapsam sahibin sözleridir (10 Ekim): “GIS-37, 38, 42, 43 maddelerini tamamla”, “Yeni branch
  içinde yap bunları”; önceki kararları: “yüksek performans ilk önceliğimiz”, simgeler sorulmadan seçilir, iki platform. Madde tek
  parçada biter; dal `gis-37-38-42-43`.
- **Bağlam belgesi:** TODOS.md `GIS-42` (ilgili `GIS-08`), ADR 0204 (raster katmanları, görünüş), ADR 0231 (raster çözümleme işi), ADR 0233
  (operasyon işi, Bölgesel istatistik'in alanları, Yeniden örnekle), ADR 0237 (çok rasterli araçların ortak ızgarası).

## Bağlam

Uydu ve hava görüntüleri (Landsat, Sentinel-2, ortofoto) çok bantlıdır: her bant bir dalga boyu aralığıdır (mavi, yeşil, kırmızı,
yakın kızılötesi, kısa dalga kızılötesi). Arazi örtüsü, bitki sağlığı, su yüzeyi, yapılaşma ve değişim bu bantlardan okunur. Netcad
Analist'in Raster Analizler'inde Renk Bandı Birleştir, Band Aritmetiği, Bitkisel İndis, Sınıflandır, Doğruluk Analizi, Değişiklik Analizi
ve Fusion; ArcGIS'te Composite Bands, Band Arithmetic, Iso Cluster Unsupervised Classification, Maximum Likelihood Classification,
Compute Confusion Matrix, Compute Change Raster, Create Pan-sharpened Raster Dataset; QGIS'te Raster calculator, SCP eklentisi ve GDAL'ın
Pansharpening'i vardır. KentOS'ta bugün raster katmanları, Raster hesaplayıcı ve Yeniden sınıflandır vardır (ADR 0204, 0233): bantları
birleştirmek, indisler, sınıflandırma, doğruluk ve görüntü birleştirme yoktur.

Araştırmada bulunanlar (10 Ekim):
- **İndisler:** NDVI = (YKÖ − K) / (YKÖ + K) (Rouse 1974); GNDVI = (YKÖ − Y) / (YKÖ + Y) (Gitelson 1996); SAVI = (1 + L)(YKÖ − K) /
  (YKÖ + K + L), L = 0,5 (Huete 1988); EVI = G (YKÖ − K) / (YKÖ + C₁K − C₂M + L), G = 2,5, C₁ = 6, C₂ = 7,5, L = 1 (Huete 2002; yansıma
  0–1 ister); NDWI = (Y − YKÖ) / (Y + YKÖ) (McFeeters 1996); MNDWI = (Y − KDK) / (Y + KDK) (Xu 2006); NDBI = (KDK − YKÖ) / (KDK + YKÖ)
  (Zha 2003). Sentinel-2 L2A'nın yansıması DN × 0,0001, Landsat Collection 2 L2'ninki DN × 0,0000275 − 0,2'dir.
- **Denetimli sınıflandırma:** eğitim alanlarının hücrelerinden sınıf başına ortalama vektör ve kovaryans; En yakın ortalama (öklit) ve
  En büyük olabilirlik (Gauss; eşit önsellerle gᵢ(x) = −ln|Σᵢ| − (x − μᵢ)ᵀΣᵢ⁻¹(x − μᵢ)); ArcGIS ve SCP böyle.
- **Denetimsiz sınıflandırma:** ISODATA ve k-ortalamalar; ArcGIS'in Iso Cluster'ı örnek hücrelerden kümeler, sonra bütün hücreleri en yakın
  merkeze atar. Başlangıç merkezleri bantların ortalama ± standart sapma köşegeni boyunca dağıtılır (ISODATA'nın alışılmış başlangıcı).
- **Doğruluk:** karışıklık matrisi (satırlar sınıflandırılan, sütunlar referans), genel doğruluk, üretici ve kullanıcı doğruluğu, Cohen'in
  kappası κ = (pₒ − pₑ) / (1 − pₑ) (Congalton 1991).
- **Değişim:** görüntü farkı, oranı, normalize farkı; sınıflandırma sonrası karşılaştırmada “neden neye” matrisi.
- **Görüntü birleştirme:** GDAL'ın Pansharpening'i ağırlıklı Brovey'dir: çok bantlı görüntü pankromatiğin ızgarasına (varsayılan kübik)
  örneklenir, her bant PAN / Σ wₖ MSₖ ile çarpılır; ArcGIS'te Brovey, IHS, Esri, Simple Mean, Gram-Schmidt.

## Karar

### 1. Kapsam

İşlemler'in yeni Uzaktan algılama kategorisinde sekiz araç:

| Araç | Kimlik | Sonuç |
|---|---|---|
| Bant birleştir | `remote.composite` | çok bantlı raster |
| Bantlara ayır | `remote.split` | bant başına raster |
| Spektral indis | `remote.index` | indis rasteri (NDVI, GNDVI, SAVI, EVI, NDWI, MNDWI, NDBI, oran, normalize fark) |
| Denetimli sınıflandırma | `remote.supervised` | sınıf rasteri ve sınıfların tablosu |
| Denetimsiz sınıflandırma | `remote.unsupervised` | sınıf rasteri ve kümelerin tablosu |
| Doğruluk analizi | `remote.accuracy` | karışıklık matrisi, genel doğruluk, kappa |
| Değişim tespiti | `remote.change` | fark, oran, normalize fark ya da sınıf değişimi rasteri; sınıf değişiminde matris |
| Görüntü birleştirme | `remote.pansharpen` | pankromatiğin ızgarasında çok bantlı raster |

**Kapsam dışı:** atmosfer düzeltmesi, bulut maskesi, radyometrik kalibrasyon ve meta veri dosyalarının (MTL, XML) okunması, ana bileşenler,
doku (GLCM), nesne tabanlı sınıflandırma, rastgele orman ve destek vektör makineleri, ISODATA'nın bölme ve birleştirmesi, IHS ve
Gram-Schmidt birleştirmesi, olasılık eşiği ve sınıflandırılmamış hücreler, piksel altı karışım.

### 2. Ortak kurallar

- **Raster işi:** her araç raster çekirdeğinin operasyon işidir (ADR 0233): 256 satırlık şeritler, satırlar iş parçacıklarında; sonuç
  karolu, önizleme katlı GeoTIFF girdinin yanında (ya da seçilen yolda), nesnesi ilk girdinin katmanının hemen üstündeki yeni katmanda.
- **Bant numaraları** 1'den başlar; raster'in bant sayısını aşan numara alanın sorunudur.
- **Boş hücre:** okunan bantlardan biri boşsa sonuç boştur; bölenin sıfır olduğu indis hücresi boştur.
- **Çok rasterli araçlar** (Bant birleştir, Değişim tespiti) girdilerin kesişiminde, en ince ızgarada çalışır (ADR 0237 §2'nin ortak
  ızgarası); Bant birleştir girdilerini Örnekleme'yle (en yakın ya da çift doğrusal), Değişim tespiti en yakın hücreleriyle okur.
- **Sınıf rasteri** 1, 2, … sınıf değerleridir, 0 boştur; 255 sınıfa kadar 8 bit, üstü 16 bit. Görünüşü Spektral rampa, el ile 0,5 …
  k + 0,5, en yakın örneklemeyle.
- **Metinler:** tabloları, özetin kuyruğunu ve uyarıları raster çekirdeği yazar (iki platform aynısını gösterir); sayılar gösterim
  kuralıyla (ADR 0149), özetteki sayımlar binlik noktalı, tablodaki sayılar noktasız.
- **Başvuru:** bağımsız başvurunun değerleriyle tamsayı ve 32 bit sonuçlar bit bit (“exact”), çift doğrusal ve kübik örneklemeli sonuçlar
  sonucun türünde en çok bir birim (“sum”); sınıflar, tablolar, metinler ve sayılar tam aynı; görüntü birleştirme GDAL'ın
  Pansharpening'iyle çapraz denetlenir.

### 3. Bant birleştir

- **Girdi:** rasterler (en az iki; girdinin sırasıyla, katman panelinde üstten alta); her rasterin bütün bantları sırayla sonucun bantları
  olur. Örnekleme (en yakın, çift doğrusal).
- **Örnek türü:** girdilerin hepsi aynı türdeyse ve değersiz değerleri aynıysa o tür ve o değer, değilse 32 bit kayan nokta; değeri
  olmayan hücre ADR 0233 §2'nin kuralıyla (değersiz değer, NaN ya da eklenen alfa).
- **Sonuç:** üç ve daha çok bantta görünüş renkli (1, 2, 3; 8 bitte olduğu gibi, öbürlerinde %2–98 gerdirme), azında gri; tablo: Bant,
  Kaynak, Kaynağın bandı; özet “n bant.”.

### 4. Bantlara ayır

- **Girdi:** tek raster. Her bant kendi GeoTIFF'ine yazılır: çıktı dosyası boşsa rasterin yanına adının sonuna `-b1`, `-b2` …, bir ad
  yazılırsa o adın sonuna; her bant bir çalıştırmadır, alfa bandında (son bant) durur. Örnek türü ve değersiz değeri kaynağınkidir;
  nesneleri yeni katmanda, sırayla. Görünüş gri, yüzde 2–98 gerdirme (8 bitte olduğu gibi); tablo: Bant, Dosya.

### 5. Spektral indis

- **Girdi:** tek raster; İndis (NDVI varsayılan); bantlar: Mavi, Yeşil, Kırmızı, Yakın kızılötesi (YKÖ), Kısa dalga kızılötesi (KDK),
  ya da oranda ve normalize farkta A ve B (yalnız indisin kullandıkları görünür; varsayılanları 1, 2, 3, 4, 5 ve 4, 3); Ölçek ve Öteleme
  (gelişmiş; her bant önce ρ = DN × ölçek + öteleme olur; varsayılan 1 ve 0); SAVI'nin L'si (0,5), EVI'nin G, C₁, C₂ ve L'si.
- **Hesap:** §Bağlam'daki tanımlar; oran A / B, normalize fark (A − B) / (A + B). Hesap float64'te, yazıldığı sırayla (çarpma-toplama
  birleştirilmez), sonuç 32 bit kayan nokta; bölen sıfırsa hücre boş kalır, sayısı uyarıdır. Özet “NDVI −0.5013 … 0.8195.”: sonucun en
  küçüğü ve en büyüğü dört basamakla.
- **Görünüş:** bitki indislerinde Arazi rampası ters çevrilmiş (yeşil yüksek), su indislerinde Mavi-kırmızı ters çevrilmiş (su mavi),
  yapı indisinde, oranda ve normalize farkta Mavi-kırmızı; yüzde 2–98 gerdirme.

### 6. Denetimli sınıflandırma

- **Girdi:** tek raster (bütün bantları); Eğitim alanları (alanlar) ve Sınıf alanı (metin); Yöntem: En büyük olabilirlik (varsayılan) ya da
  En yakın ortalama.
- **Eğitim:** sınıflar Sınıf alanının boş olmayan, kırpılmış (JavaScript'in boşlukları) metinleri, doğal sıralarıyla (ADR 0153) 1, 2, …
  değerleri; çekirdek metinlerden kurar, iki platform nesnelerin metinlerini verir. Bir sınıfın hücreleri merkezi o sınıfın alanlarından
  birinin içinde (Bölgesel istatistik'in kuralı, ADR 0233) ve hiçbir bandı boş olmayan hücrelerdir; iki alanının altındaki hücre bir kez.
  Her sınıfın ortalama vektörü ve örneklem kovaryansı (n − 1) float64'te: satır başına Welford, satırlar satır sırasıyla Chan'ın
  formülüyle (iş parçacığı sayısından bağımsız). İlk geçiş yalnız eğitim alanlarının altındaki blokları okur.
- **Ret:** sınıf yoksa; her sınıf sırayla: En büyük olabilirlikte hücresi bant sayısı + 1'den azsa, hücresi yoksa, kovaryansı tekilse
  (Cholesky'de bir köşegen kalanı kendi varyansının 10⁻¹²'sini aşmazsa), sınıfın adıyla.
- **Atama:** En yakın ortalama argminᵢ ‖x − μᵢ‖²; En büyük olabilirlik argmaxᵢ (−ln|Σᵢ| − (x − μᵢ)ᵀΣᵢ⁻¹(x − μᵢ)); eşitlikte küçük
  değer.
- **Sonuç:** sınıf rasteri; tablo: Sınıf, Değer, Eğitim hücresi, Hücre sayısı, Alan (m²); özet “k sınıf, n eğitim hücresi.”.

### 7. Denetimsiz sınıflandırma

- **Girdi:** tek raster (bütün bantları); Küme sayısı (2–50, varsayılan 8); En çok yineleme (1–100, varsayılan 20).
- **Örnek:** hiçbir bandı boş olmayan, i ve j'si s'nin katı olan hücreler, s = ⌈√(hücre sayısı / 250 000)⌉ (en az 1).
- **Başlangıç:** her bandın örnekteki ortalaması μ ve standart sapması σ; c'inci merkez (c = 0 … k − 1) μ + σ·(2(c + ½)/k − 1).
- **Yineleme:** her örnek en yakın merkeze (karesel uzaklık; eşitse küçük sıra); merkezler üyelerinin ortalaması (boş küme yerinde kalır);
  atamalar değişmeyince ya da en çok yinelemede durur.
- **Sıra:** kümeler merkezlerinin bant toplamına göre artan (eşitse ilk sıra) 1 … k değerlerini alır; bütün hücreler en yakın merkeze.
- **Sonuç:** sınıf rasteri; tablo: Küme, Hücre sayısı, Alan (m²), merkezin bantları (üç basamakla); özet “k küme, r yineleme (n örnek
  hücre).”. Örneklemin toplamları örneğin sırasıyla, en yakın merkezler iş parçacıklarında.

### 8. Doğruluk analizi

- **Girdi:** sınıf rasteri (tek raster, 1. bant); Referans nesneleri (noktalar ya da alanlar) ve Referans alanı (sınıfın değeri, tam sayı).
- **Hücreler:** noktada (çok noktalının her noktasında) noktanın düştüğü hücre; alanda merkezi alanın içindeki hücreler; her nesnenin
  hücreleri ayrı sayılır. Referans tam sayı olarak okunur (kentos.statistics/1, |değer| ≤ 2³¹ − 1); sınıf değeri hücrenin değerinin tam
  kısmı. Boş ya da rasterin dışındaki hücre ve okunamayan referans alınmaz, söylenir; değerlendirilecek hücre yoksa ret. Okuma yalnız
  nesnelerin altındaki blokları okur.
- **Sonuç:** tablo: satırlar sınıflandırılan, sütunlar referans değerler (ikisinin birleşimi, artan), Toplam ve Kullanıcı doğruluğu (%)
  sütunları, Toplam ve Üretici doğruluğu (%) satırları; özet “Genel doğruluk %87.50, kappa 0.8312 (240 hücre).” Çıktılar: tablo, genel
  doğruluk, kappa. Her pay ve kappa tamsayılardan tek bölmedir: κ = (n·Σd − Σrᵢcᵢ) / (n² − Σrᵢcᵢ).

### 9. Değişim tespiti

- **Girdi:** iki raster: Önceki ve Sonraki (ayrı parametreler); Bant (1); Yöntem: Fark (sonraki − önceki), Oran (sonraki / önceki),
  Normalize fark ((sonraki − önceki) / (sonraki + önceki)), Sınıf değişimi.
- **Sınıf değişimi:** sınıf rasterlerinde değer önceki × 1000 + sonraki (0'lar boş), 32 bit tam sayı; tablo: “neden neye” matrisi (satırlar
  önceki, sütunlar sonraki sınıflar), hücre sayıları; özet değişen hücrelerin payı.
- **Özet:** Fark, Oran ve Normalize farkta artan, azalan ve değişmeyen hücreler; Sınıf değişiminde “Değişen hücre n / m (%p).”.
- **Görünüş:** Fark, Oran ve Normalize fark Mavi-kırmızı rampa, değişimsizliğin (0; oranda 1) iki yanına en büyük değişime kadar eşit
  gerdirme (değişimsizlik beyaz): görünüş çalıştırma bitince değerlerden kurulur, ev sahibi onu iş bitince okur. Sınıf değişimi Spektral,
  en küçükten en büyüğe, en yakın örnekleme.

### 10. Görüntü birleştirme

- **Girdi:** Çok bantlı raster ve Pankromatik raster (ayrı parametreler); Yöntem: Brovey (ağırlıklı, varsayılan) ya da Basit ortalama;
  Ağırlıklar (Brovey; boş: eşit, 1/n); Örnekleme: kübik (varsayılan), çift doğrusal, en yakın.
- **Hesap:** çok bantlı görüntü pankromatiğin ızgarasına örneklenir (Yeniden örnekle'nin çekirdekleri). Brovey: MS'ₖ = MSₖ · PAN / Σ wⱼMSⱼ
  (Σ wⱼMSⱼ ≤ 0 ise MS'ₖ = 0); Basit ortalama: (MSₖ + PAN) / 2.
- **Ağırlıklar** sayılar ya da noktalı virgül veya boşlukla ayrılmış metin (kentos.statistics/1); sayısı bant sayısı olmalı.
- **Sonuç:** pankromatiğin ızgarasında, çok bantlının örnek türünde (tam sayılarda en yakın tam sayıya, yarımlar sıfırdan uzağa, türün
  aralığına kırpılarak; ADR 0233'ün kuralı); görünüşü çok bantlınınki. GDAL örneklenmiş bantları formülden önce türlerine yuvarlar; araç
  yuvarlamaz (GDAL'la fark en çok bir birim).

### 11. Arayüz

- **Araçlar** iki platformda aynı adlar, parametreler ve varsayılanlarla; **komutlar** `processing.run.<kimlik>`; tablo veren araçların
  tablosu çalıştırmadan sonra görünür olur (ADR 0237).
- **Şerit:** CBS'nin Raster sekmesinde Uzaktan algılama paneli (altı araç; Bant birleştir ve Bantlara ayır ▾'de, sekme geniş pencerede
  3200 px'in altında kalsın diye); sekme 1100 px'e sığsın diye Raster ve vektör ile Taranmış harita (ADR 0234) tek panelde.
- **Takma adlar:** BANTBIRLESTIR, COMPOSITEBANDS; BANTAYIR, SPLITBANDS; INDIS, NDVI; DENETIMLI, MAXLIKELIHOOD; DENETIMSIZ, ISOCLUSTER;
  DOGRULUK, CONFUSIONMATRIX; DEGISIM, CHANGEDETECTION; PANSHARPEN, FUSION.

### 12. Performans

- Raster çekirdeğinin operasyon işi: şeritler, satırlar iş parçacıklarında; iki geçişli araçlar (Denetimli: eğitim, atama; Denetimsiz:
  örnek, atama; Doğruluk: tek geçiş) aynı işin geçişleridir. Masaüstünde İşlemler'in iş parçacığında, web'de çözümleme işçisinde.
- **Bütçeler** (release, geliştirme makinesi, 4096², 4 bantlı 16 bit görüntü; masaüstü / web):

| İş | Masaüstü | Web |
|---|---|---|
| Bant birleştir (dört tek bantlı) | ≤ 1,5 s | ≤ 6 s |
| Bantlara ayır | ≤ 2 s | ≤ 8 s |
| NDVI | ≤ 1 s | ≤ 4 s |
| Denetimli, en büyük olabilirlik, 6 sınıf | ≤ 2 s | ≤ 8 s |
| Denetimsiz, 8 küme, 20 yineleme | ≤ 2 s | ≤ 8 s |
| Doğruluk analizi, 10 000 nokta | ≤ 0,5 s | ≤ 2 s |
| Değişim tespiti, fark | ≤ 1 s | ≤ 4 s |
| Görüntü birleştirme, Brovey, 2048² MS → 4096² PAN | ≤ 2 s | ≤ 8 s |

## Uygulama

- **Raster çekirdeği** (`crates/shared/raster/src/remote/`):
  - `spectral`: indislerin formülleri ve bantları (§5).
  - `classify`: eğitimin momentleri (Welford, Chan), Cholesky, iki sınıflandırıcı (§6).
  - `cluster`: örneğin adımı ve k-ortalamalar (§7).
  - `accuracy`: referansın okunuşu, karışıklık matrisi, payları ve kappa, metinleri (§8).
  - `work`: araçların ızgarası, sonucu ve görünüşü; okuma geçişleri (eğitim, örnek, referans), blokların hücreleri, notlar.
  - İşe (`ops.rs`) sekiz araç türü (`composite`, `band`, `index`, `supervised`, `unsupervised`, `accuracy`, `change`, `pansharpen`),
    `Work::Remote`, okuma geçişi ve ardından sonuç geçişi, `OpsFinished::Report`, notlarda `remote` (tablo, kuyruk, uyarılar, genel
    doğruluk, kappa), iş bitince görünüş (`style()` yeniden okunur); blokların planı en geniş geçişin okuduğuyla.
- **Raster okuyucusu** (`kentos-formats`, ADR 0204'ün düzeltmesi): TIFF'in ExtraSamples'ında yalnız 1 ve 2 alfadır, 0 veridir (GDAL'ın
  çok bantlı GeoTIFF'lerinin ek bantları); etiket yoksa RGB'nin dördüncü örneği alfa. Önce her ek örnek alfa sayılıyordu: dört bantlı
  16 bitlik uydu görüntüsünün yakın kızılötesi bandı maske oluyordu. `raster_cases.py` GDAL'ın son bandı okuyuşunu kaydeder; iki yeni
  dosya (RGBA ve dört bantlı 16 bit).
- **WASM:** notlarda `remote`; iş bitince görünüş.
- **Web:** araçlar `processing/builtin/remote/` (`tools.ts`, `shared.ts`); kategori `remoteSensing`; şeritte Uzaktan algılama paneli ve
  Raster ve vektör'ün birleşik paneli (`app/ribbon.ts`); sekiz ikon (`bandComposite`, `bandSplit`, `spectralIndex`, `classifySupervised`,
  `classifyUnsupervised`, `accuracyMatrix`, `changeDetect`, `pansharpen`).
- **Masaüstü:** `kentos-processing`'in `builtin/remote/` (`mod.rs`: ham raster, Bantlara ayır, Doğruluk analizi koşucuları; `tools.rs`);
  `drive` raporu alır ve görünüşü iş bitince okur; pencere ve resimler `apps/desktop` (`remote_scenes.rs`, `processing/remote_tests.rs`).
- **Ortak durumlar:** `fixtures/processing/v1/remote.json` (21 durum), çizim ve 12 raster; oynatıcılarda `remoteOf` (tam sayı sonucun
  “sum”u bir birim).

## Doğrulama

- **Bağımsız başvuru** `scripts/fixtures/remote_cases.py` (KentOS kodu yok; eğitim hücreleri ADR 0233'ün kesirli kuralıyla, ortalamalar ve
  kovaryanslar kesirlerle, Cholesky ve ayırıcılar 50 basamaklı mpmath'le; float64'te ayırt edilemeyecek kadar yakın iki sınıf başvuruda
  reddedilir; çift doğrusal ve kübik ağırlıklar kesin): `fixtures/remote/v1/cases.json`, 46 durum. Çekirdeğin testi
  (`tests/all/remote.rs`) hepsini bir ve üç iş parçacığıyla oynatır: rasterin boyu, yeri, türü ve değersiz değeri, örnekler kuralıyla,
  tablolar, kuyruklar, uyarılar, genel doğruluk ve kappa tam; iki çalıştırma bit bit aynı. Okuyucunun düzeltmesinden sonra 46'sı da geçti.
- **GDAL'la çapraz denetim:** ağırlıklı Brovey kübik örneklemeyle GDAL'ın VRTPansharpenedDataset'iyle, kenarlardan uzaktaki 384 örnekte:
  GDAL'ın örneklenmiş bantları türüne yuvarlama kuralıyla başvuru bire bir aynı; aracın kendisi (yuvarlamaz) en çok bir birim farklı.
- **İşlemler'in ortak durumları:** 21 durum iki platformda (yazılan rasterler başvurunun durumlarıyla); varsayılanlar ve pencerenin formları
  (`dialog.json`, yalnız eklemeler).
- **Masaüstü:** üç test (NDVI görüntünün katmanının hemen üstünde ve tek adımda geri alınır, beş sınıf adlarının sırasıyla, matris çizimi
  değiştirmez, her bandın rasteri).
- **Resimlerin sahnesinde** denetimli sınıflandırma referans noktalarıyla genel doğruluk %87.14, kappa 0.8393 (70 hücre; iki platformda
  aynı); tarım örtüsünün çıplak toprak parselleri yerleşimle karışır (üretici doğruluğu %35.71).
- **Süreler** (release, geliştirme makinesi, 4096² dört bantlı 16 bit görüntü, 8 iş parçacığı; web tarayıcının çözümleme işçisinde, tek
  iş parçacığı):

| İş | Masaüstü | Bütçe | Web | Bütçe |
|---|---|---|---|---|
| Bant birleştir (dört tek bantlı) | 0,78 s | 1,5 s | 3,06 s | 6 s |
| Bantlara ayır (dört bant) | 1,36 s | 2 s | 5,58 s | 8 s |
| NDVI | 0,44 s | 1 s | 1,74 s | 4 s |
| Denetimli, en büyük olabilirlik, 6 sınıf | 0,60 s | 2 s | 2,55 s | 8 s |
| Denetimsiz, 8 küme, 20 yineleme | 0,44 s | 2 s | 1,82 s | 8 s |
| Doğruluk analizi, 10 000 nokta | 0,019 s | 0,5 s | 0,14 s | 2 s |
| Değişim tespiti, fark | 0,59 s | 1 s | 2,24 s | 4 s |
| Görüntü birleştirme, Brovey, 2048² → 4096² | 1,26 s | 2 s | 5,35 s | 8 s |

- **Resimler:** masaüstünde `tools_screens`'in `ua-*`'ı, web'de `shots.mjs remote`; iki tema, iki boy.
