# ADR 9333: Hizmet alanı aralıkları ve kenar payını baştan gösterir, yaklaşık alanı ulaşılan yollardan ayırır; Tampon'un Yan'ı çizgisiz girdide kapanır

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 26 Kasım 2026 (`gis-eksik-26-11-2026.md`, madde
  17, P2). Sahibin sözü: “dosyadaki maddeleri yapalım, bitene kadar durma”.
- **İlgili:** [ADR 0209](0209-network-analysis.md) §6 (Hizmet alanı), [ADR 0201](0201-geometry-operations.md) §2
  (Tampon), [ADR 9179](9179-buffer-cases-and-the-run-in-words.md) (işin sözleri), [ADR 9332](9332-network-tools-see-the-network.md) (ağ
  araçlarının ayarları).

## Bağlam

- **Hizmet alanı:**
  - Yeni yol ağında A seçildi. Eşik Diğer › Aralıklar altındaydı (500/1000/1500 m); biçim disk, kenar payı 50 m, çizgi
    çıktısı kapalıydı.
  - 35 m girilince bir alan yazıldı. Alanın C'yi geçmesi ağdaki erişimi kanıtlamaz: 50 m kenar payı, 35 m eşiğin
    dışında kalan yerleri de kaplar.
- **Tampon:** `qa_noktalar`, 10 m, bir halka, birleştirme kapalı.
  - Yan: Sol seçilebildi; üç tam daire oluştu, yani çıktı noktalar için doğruydu.
  - İşlem geçmişi “Yan: Sol” yazdı: uygulanmayan bir seçeneği uygulanmış gibi gösterdi.
- **İncelemenin kabulü:**
  - Hizmet alanında maliyet ve eşikler önizlemeden önce görünsün.
  - Ağda ulaşılan çizgi parçaları ve onlardan türeyen yaklaşık alan ayrı adlarla yazılsın.
  - 35 m eşiğinde A–C'nin 30 m'si, C–B'nin ilk 5 m'si ve A–B'nin ilk 35 m'si ulaşılan çizgilerde doğrulansın.
  - Çizgisiz Tampon girdisinde Sol/Sağ kapansın; karışık girdide hangi kayıtların etkileneceği gösterilsin.
  - Geçmiş etkin seçenekleri yazsın. Üç noktanın tamponu 10 m yarıçaplı, üç kayıt olsun.

## Araştırma

- **QGIS, Hizmet alanı:** sonuç, eşik içinde ulaşılan kenarlar ya da kenar parçalarıdır (Service area (lines)); sınır
  düğümleri isteğe bağlıdır; çokgen üretilmez
  ([Network analysis](https://docs.qgis.org/latest/en/docs/user_manual/processing_algs/qgis/networkanalysis.html)).
- **ArcGIS Pro, Service Area:**
  - Çokgen yoldan Polygon Trim Distance kadar uzanır; belge bunu “çizgi tamponuna benzer” ve “kabaca” diye anlatır.
    Öntanımlı 100 m'dir.
  - Çıktı türü çokgen, çizgi ya da ikisidir. Belge, yalnız kırpma payı içini isteyenlere çizgiyi alıp tamponlamayı
    önerir
    ([Service area analysis layer](https://pro.arcgis.com/en/pro-app/latest/help/analysis/networks/service-area-analysis-layer.htm)).
- **ArcGIS Pro, Buffer:** Side Type yalnız çizgi ve alan girdisinde vardır; Left ve Right yalnız çizgidedir
  ([Buffer](https://pro.arcgis.com/en/pro-app/latest/tool-reference/analysis/buffer.htm)).
- **QGIS, Buffer:** tamponun yan seçeneği yoktur. Tek yanlı tampon (Single sided buffer) ayrı bir araçtır ve yalnız
  çizgi katmanı alır
  ([Vector geometry](https://docs.qgis.org/latest/en/docs/user_manual/processing_algs/qgis/vectorgeometry.html)).
- **KentOS kararı:**
  - Hizmet alanı çokgeni ve ulaşılan yolları birlikte üretmeye devam eder. Çokgen “yaklaşık” adını taşır; erişimin
    kendisi “Ulaşılan yollar”dadır.
  - Tampon tek araç kalır. Yan, girdinin çizgilerinde etkilidir; çizgi yokken kapanır ve neden söylenir.

## Karar

### 1. Hizmet alanının aralıkları ve kenar payı baştan görünür

- **Komut satırının sırası:** Aralıklar, Kenar payı, Ağ, Maliyet, Çizgiler, Engel, Biçim, Yön, Birleşik.
  - Dar satır son seçenekleri Diğer'e katlar; aralıklar ve kenar payı satırda kalır. Adım yazısı uzayınca (“1 tesis;
    3 çizgi, 1 alan; …”) satıra yalnız ilk seçenekler sığar.
  - Ağ ve maliyet ayarlarda da durur.
  - Engel, “sonraki tıklama” durumu görünür kalsın diye biçim seçeneklerinin önündedir.
- **Öznitelikler'in başı:** ADR 9332'nin satırlarının (Ağ, Maliyet, Tolerans, Bağlanma) altında şunlar gelir:
  - Aralıklar, birimiyle (“500 1000 1500 m”, “5 10 15 dk”);
  - Kenar payı;
  - Çizgiler (yazılır / yazılmaz);
  - alanın ne olduğu: “Alan, ulaşılan yolların 50.000 m çevresidir; yaklaşıktır, “Yaklaşık hizmet alanı” katmanına
    yazılır. Ağda ulaşılan yollar Çizgiler açıkken “Ulaşılan yollar” katmanına yazılır.”

### 2. Çıktıların adı ve sözü

- **Katmanlar:** alanlar “Yaklaşık hizmet alanı”na, çizgiler “Ulaşılan yollar”a yazılır.
  - Ad, işlem aracının kendi “Ulaşılan yollar” sözüyle aynıdır.
  - Hizmet alanları işlem aracının (`network.serviceAreas`) öntanımlı katmanları da bu adlardır.
- **İleti:**
  - Çizgiler açıkken: “Hizmet alanı: 1 yaklaşık alan ve 3 ulaşılan yol parçası yazıldı; alan, yolların 50.000 m
    çevresidir.”
  - Kapalıyken: “Hizmet alanı: 1 yaklaşık alan yazıldı; alan, ulaşılan yolların 50.000 m çevresidir. Ulaşılan yollar
    için Çizgiler (Ç) seçeneğini açın.”
- **Çizgilerin öntanımı:** kapalı kalır; ayarlardaki not ve ileti onu söyler.

### 3. Girdinin yalnız bazı nesnelerinde etkili seçenek

- **Tanım:** bir işlem parametresi, bir girdinin yalnız bazı nesnelerinde etkili olduğunu söyleyebilir (`ActsOn`):
  - girdinin adı ve nesne kuralı;
  - çizgi yokken söylenen neden, sayılı söz ve öbür nesnelerin aldığı (“iki yandan alınır”);
  - öbür nesnelerin seçeneği (Tampon'da İki yan).
- **Sayım:** pencerenin girdi özeti ve işin hazırlığı, kuralı tutan nesneleri sayar (`InputSummary::acting`).
- **Tampon'un Yan'ı:** çekirdeğin aldığı biçimde yollarda etkilidir: çizgi, çoklu çizgi, yay, açık elips ya da eğri.
  Alanlar ve noktalar iki yandan alınır.
- **Pencere:**
  - Girdide hiç çizgi yokken Sol ve Sağ kapalıdır, İki yan seçili görünür. Altında “Girdide çizgi yok; nesneler iki
    yandan alınır.” yazar.
  - Karışık girdide İki yan dışında bir seçim varken altında “Sol yalnız 1 çizgide; öbür 3 nesne iki yandan alınır.”
    yazar.
  - Seçilen değer silinmez: girdi çizgi taşıyınca yeniden görünür.

### 4. Geçmiş etkin seçenekleri yazar

- **Çizgisiz girdi:** işin sözlerinde (ADR 9179) “Yan: kullanılmadı (girdide çizgi yok; nesneler iki yandan alınır)”
  yazar; “Yan: Sol” yazılmaz.
- **Karışık girdi:** “Yan: Sol (1 çizgide; öbür 3 nesne iki yandan alınır)” yazar.
- **Sayılar** işin hazırlandığı andaki nesnelerdendir.
- **Yeniden aç** değerleri girildiği gibi doldurur.

### 5. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Hizmet alanının sırası, ayarları, çıktı adları ve iletisi | yok: Mimari'de ağ analizi yoktur | yok: ağ araçları yalnız CBS'dedir | yok: aynı gerekçe | var |
| Hizmet alanları işlem aracının öntanımlı katmanları | yok: araç yalnız CBS'dedir | yok: aynı gerekçe | yok: aynı gerekçe | var |
| Tampon'un Yan'ı: pencerede kapanma, sayılar, geçmiş | yok: Mimari'de Tampon yoktur | var | var | var |

### 6. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | var | `cad.processing.run` aynı Tampon'u çalıştırır; Yan noktalarda çekirdekte de kullanılmaz. `network.serviceAreas` aynı öntanımlı katmanlara yazar. Pencerenin kapanan parçaları ve geçmişin sözleri masaüstünün sunumudur. |
| MCP | var | Aynı komut. |
| Yapay zekâ ajanı | var | Aynı komut. |
| Eklenti | var | Aynı komut. Eklentinin kendi işlem aracı `ActsOn` bildiremez; tanım dosyasında karşılığı yoktur. |

## Açık

- **Eklenti araçları:** eklentinin tanım dosyası (`kentos-processing-tool`) bir parametrenin yalnız bazı nesnelerde
  etkili olduğunu bildiremez.
- **Uç biçimi:** Tampon'un tek yanlı şeridinin uçları düzdür, iki yanlınınki yuvarlaktır; ayrı bir uç seçeneği yoktur.
- **Gerçek pencere:** fare ve klavyeyle kabul (`scripts/usage/real/`) bu dilimde yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **İşlem çekirdeği:** `cargo test -p kentos-processing --test cases`: 38 geçti.
  - Ağ durumları (`the_network_cases_do_what_they_say`) ve araçların öntanımlıları yeni katman adlarıyla geçti.
  - Örnek dosyası (`fixtures/processing/v1/network.json`) elle güncellendi: üreteç shapely ister, bu makinede kurulu
    değil. Değişen yalnız iki ad ve onlardan türeyen kimliklerdir (`islem-yaklasik-hizmet-alani`,
    `islem-ulasilan-yollar`). Önce dosyanın üretecin JSON biçiminde olduğu denetlendi; üreteç aynı adları yazar ve
    kimlik kuralı ayrıca denendi.
- **Masaüstü:** `cargo test -p kentos-desktop --lib -- buffer_cases_tests`: 7 geçti.
  - `yan_is_off_for_points_and_the_record_says_it_was_not_used`:
    - üç noktada Yan'ın sayısı 0/3, yalnız İki yan açık, söz “Girdide çizgi yok; nesneler iki yandan alınır.”;
    - Sol ile koşunca üç kayıt, her biri noktasının çevresinde 10 m yarıçaplı daire;
    - geçmiş “Yan: kullanılmadı (girdide çizgi yok; nesneler iki yandan alınır)” yazıyor, “Yan: Sol” yazmıyor.
  - `yan_on_points_and_a_line_says_what_it_reaches`:
    - İki yan'da söz yok; Sol'da “Sol yalnız 1 çizgide; öbür 3 nesne iki yandan alınır.”;
    - dört kayıt; çizginin şeridi kuzeyinde (çizildiği yönün solu) 100 × 10 m;
    - geçmiş “Yan: Sol (1 çizgide; öbür 3 nesne iki yandan alınır)”.
- **CBS paketi:** `cargo test -p kentos-desktop-cbs -- service_area_tests prerequisite_tests`: 3 geçti.
  - `the_breaks_and_the_margin_stand_first_and_35_m_reach_what_the_network_reaches`:
    - komut satırı Aralıklar ve Kenar payı ile başlıyor;
    - ayarlarda Aralıklar, Kenar payı, Çizgiler ve alanın notu var;
    - 35 m ve Çizgiler açıkken A'dan ulaşılan yollar 5, 30 ve 35 m, “Aralık” “0–35”;
    - tek yaklaşık alan B'yi kaplıyor, ağ B'ye ulaşmıyor; ileti alanın ne olduğunu söylüyor.
- **İzler:** `fixtures/interaction/v1/network-service-area.json` yeni sıraya, katman adlarına ve iletiye çevrildi;
  `cargo test -p kentos-cad`.
- **Resim** (`.run/gui-evidence/kullanim-9333/`, açık ve koyu tema; büyütülerek bakıldı):
  - `01-hizmet-alani-baslangic`: komut satırında Aralıklar, ayarlarda aralıklar, kenar payı, çizgiler ve alanın notu;
  - `02-hizmet-alani-35m`: yaklaşık alanın içinde mavi ulaşılan yollar, kalan yollar ulaşılmamış;
  - `03-hizmet-alani-yazildi`: “Yaklaşık hizmet alanı” 1, “Ulaşılan yollar” 3 nesne;
  - `04-tampon-noktalar`: Sol ve Sağ sönük, neden altında;
  - `05-tampon-karisik`: “Sol yalnız 1 çizgide; …”;
  - `06-tampon-gecmis`: İşlemler › Geçmiş'te “Yan: kullanılmadı (…)”, üç daire.
