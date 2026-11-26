# ADR 9310: İşlem araçlarının modülleri ve Python ile yazılan işlem araçları

- **Durum:** kabul edildi (2026-11-26). Sahibin isteği (26 Kasım 2026): işlem araçları CBS ile CAD arasında ayrılsın;
  bazıları CBS'nin değildir, bazıları ikisinindir, bazıları Yol'undur. Python ile de yazılabilsinler, Python desteği
  iyileşsin, örnek aracın yeri belli olsun.
- **İlgili:** [ADR 0508](0508-processing-tools-as-commands.md) (araçlar komut olarak), [ADR 0802](0802-strict-module-separation.md)
  (modül programlarının sunduğu), [ADR 9195](9195-module-separation.md) (modül ayrımı), [ADR 9199](9199-desktop-packages.md)
  (paketler), [ADR 8057](8057-phase-3-extension-packages.md) (eklentinin Python araçları),
  [ADR 9312](9312-install-layout.md) (örnek aracın yeri).
- **Kural:** CLAUDE.md §25 kural 2 (her yüzey her modül için açıkça var ya da yok), “Modül ve otomasyon denetimi”.
- **Web:** iptal edildi. **`.kcad`:** değişmedi. **Bağımlılık:** `kentos-processing` → `kentos-launch` (iç crate).

## Bağlam

İşlemler'in 157 hazır aracı vardı. Modüle göre ayrımları dağınıktı:

- **Şeritteki 137 araç** şeridin kararıyla ayrılıyordu: bir aracı hangi modülün şeridi tutuyorsa o modülde çıkıyordu
  (`catalog().module_has`).
- **Şeritsiz 20 araç**, araç kutusu olan her modüle düşüyordu (CBS, Harita, Yol). Örnekler: adres eşleme Yol'da,
  kenar yazıları CBS'de çıkıyordu.
- **Python, MCP ve yapay zekâ ajanı** (`cad.processing.tools`, `.run`, `.batch`) modüle hiç bakmıyordu. Mimari çizimde
  ağ analizi çalışıyordu.
- **Eklentinin Python araçları** hep Eklentiler kategorisindeydi. Modülleri yoktu.

### Araştırma (26 Kasım 2026'da okundu)

- **QGIS:** işlem algoritması iki yolla Python'da yazılır: `QgsProcessingAlgorithm` sınıfı ya da `@alg` süsü.
  - Süsle yazılan algoritma her zaman kullanıcının Processing Scripts sağlayıcısına düşer; bir eklentinin kendi
    sağlayıcısına konamaz.
  - Kaynak: [Writing new Processing algorithms as Python scripts](https://docs.qgis.org/3.10/ca/docs/user_manual/processing/scripts.html),
    [ProcessingAlgFactory](https://qgis.org/pyqgis/3.40/processing/ProcessingAlgFactory.html).
- **ArcGIS Pro:** bir aracın kullanılabilmesi lisansa ve uzantıya bağlıdır; eksikse araç hata vererek durur.
  - Python araç kutusunda `isLicensed` aracı çalıştırmadan kapatır.
  - Kaynak: [Licenses and extensions access in Python](https://pro.arcgis.com/en/pro-app/3.6/arcpy/geoprocessing_and_python/accessing-licenses-and-extensions-in-python.htm),
    [Control license behavior in a Python toolbox](https://pro.arcgis.com/de/pro-app/3.3/arcpy/geoprocessing_and_python/controlling-license-behavior-in-a-python-toolbox.htm).
- **KentOS kararı:**
  - Aracı kapatan ölçüt lisans değil projenin modülüdür (CLAUDE.md §25 kural 1). Kapatılan araç listelenmez,
    çalıştırılırsa nedeni söylenir.
  - Python aracı bir sağlayıcının değil, hazır araçla aynı sözleşmenin parçasıdır: kategorisi ve modülleri tanımındadır.
  - Kullanıcının kendi betik klasörü (QGIS'in Scripts sağlayıcısı gibi) ayrı bir karardır (§Açık kalanlar).

## Karar

### 1. Her araç her modül için açıkça var ya da yok

- **`ModuleSet`** (`kentos_launch`): dört modülün her biri için adlı bir evet/hayır (`architecture`, `map`, `gis`,
  `road`). Yeni bir modül yeni bir alandır; her küme onu yerleştirmeden derlenmez. “Her modül” öntanımı yoktur.
- **`Tool.modules`**: her araç tanımının alanıdır. Hazır araçlarınki tek tablodan gelir:
  `kentos_processing::modules::PLACED`. Tabloda 157 satır ve altı küme vardır.
- **Sınama:** `every_builtin_tool_is_placed_once` her aracın tabloda bir kez olduğunu sınar. Tabloda olup aracı olmayan
  kimlik de düşer.

| Küme | Mimari | Harita | CBS | Yol | Araç | Ne |
|---|---|---|---|---|---|---|
| `EVERY` | var | var | var | var | 2 | İfadeyle seç, Konuma göre seç (Mimari'nin şeridi tutar) |
| `SHARED` | yok | var | var | var | 70 | Öznitelik, dönüştürme, geometri, katman, doğrusal referans, nokta bulutu, yüzey (eğim, bakı, eş yükselti …), IDW ve TIN, raster kırpma/mozaik, taranmış harita, örnekleme (ağ, düzenli, hat boyunca), Özet istatistik, Koordinat sistemine dönüştür |
| `GIS_MAP` | yok | var | var | yok | 6 | Adres eşleme ve konumlara adres, Bant birleştir, Altyapı bağları (senaryo), Topoloji denetimi, İmar planı denetimi |
| `GIS_ROAD` | yok | yok | var | var | 12 | Hidroloji (8), Uzaklık ve maliyet (4): drenaj ve güzergâh koridoru |
| `CAD_DRAWING` | yok | var | yok | var | 2 | Kenar uzunluklarını yaz, Köşe noktalarını numarala: çizimin kendi yazısı |
| `GIS` | yok | yok | var | yok | 65 | Ağ analizi, mekânsal istatistik, uygunluk, uzaktan algılama, çok boyutlu veri, genelleştirme, yoğunluk, kriging/spline/doğal komşu, raster hesap ve istatistikleri, yakınlık, Voronoi, içbükey zarf |

- **Mimari'nin araç kutusu yoktur** (ADR 9195 §5). Şeridinin tuttuğu iki seçim aracı çalışır. Bina çizimi kendi
  koordinatındadır; geometri ve katman araçları gerçek koordinatın işidir.

### 2. Şeritle uyum ve değişen 20 araç

- **Şeritteki 137 aracın** kümesi, bugün onu tutan şeritlerin modülleridir. Bu araçlarda davranış değişmedi.
- **Matris sınaması:** `the_processing_tools_modules_and_ribbons_agree` her modülde her aracı sınar:
  - şerit aracı yalnız kümesinin modülünde tutar;
  - kümesinde olan araç şeritte ya da araç kutusundadır;
  - `module_has` kümenin söylediğini söyler.
- **Şeritsiz 20 aracın kararı:**

| Araç | Önce | Şimdi | Gerekçe |
|---|---|---|---|
| Adresleri eşle, Konumlara adres yaz | CBS, Harita, Yol | CBS, Harita | Adres komutları CBS'nin ve Harita'nındır; Yol'da adres yok |
| Kenar uzunluklarını yaz | CBS, Harita, Yol | Harita, Yol | Çizime yazı nesnesi koyar; CBS'de kenar ölçüsü etiketin işidir (ADR 9311) |
| Köşe noktalarını numarala | CBS, Harita, Yol | Harita, Yol | Köşe adını yazı nesnesi olarak yazar (ADR 9311); CBS'nin karşılığı Köşeleri noktaya çevir'dir |
| Öznitelik hesapla, Anahtarla birleştir, Veri aktar | CBS, Harita, Yol | aynı | Veri ve bağlı katmanlar üç modülde de var |
| Semt ve uzunluktan çizgi, XY'den çizgi, Tablodan elips, Tablodan nokta | CBS, Harita, Yol | aynı | Ölçme tablosundan çizim üçünün de işi |
| Koordinat sistemine dönüştür | CBS, Harita, Yol | aynı | `layer.reproject` üçünde de var |
| Topolojiyi kurallarla denetle, İmar planını denetle | CBS, Harita, Yol | CBS, Harita | Topoloji ve imar komutları Yol'da yok |
| Altyapı bağlarını çiz | CBS, Harita, Yol | CBS, Harita | Senaryo komutları CBS'nin ve Harita'nındır |
| Katman araçları (5) | CBS, Harita, Yol | aynı | Her gerçek koordinatlı katmanın işi |

### 3. Tek kapı: arayüz ve bütün kanallar aynı kümeye bakar

- **Masaüstü:** `App::offers` tek kapıdır: araç kutusu, komut satırı, Komut ara, kısayollar ve çizimin menüleri.
  Bir işlem aracında canlı kayda sorar (`Registry::modules_of`), yani hazır araçlarda ve eklenti araçlarında aynı.
  Başka modülün aracı için ileti, aracı sunan programları söyler (`not_offered`).
- **Katalog:** `catalog().module_has` hazır araçta tablonun kararını verir (modül kataloğu, şerit sınamaları).
- **Python, MCP, eklenti, ajan** (`cad.processing.tools`, `.run`, `.batch`) aynı kümeyi okur:
  - **Listeleme:** belgenin modülünün araçları listelenir (`modules::runs_on`).
  - **Ret:** başka modülün aracı `tool_not_in_module` koduyla reddedilir ve ileti aracın çalıştığı projeleri söyler:
    “Hizmet alanları” bu mimari projede yok; CBS projelerinde var.
  - **`modules`:** `ProcessingTool`'un yeni alanı her aracın modüllerini verir.
  - **Bilinmeyen modül:** modülü belli olmayan belge (türü sorulmamış eski çizim) bütün araçları görür; bu, masaüstünün
    “her şey” kuralıyla aynıdır.
- **Modeller:** model, bütün adımlarının ortak modüllerinde çıkar (`ModuleSet::and`).

### 4. Python ile yazılan işlem aracı

Python aracı eklentidir (ADR 8057): tanım `tools/<id>.tool.json`, iş `@ext.processing("<id>")`. Bu karar tanıma iki
alan ekler:

- **`modules`:** aracın çıktığı modüller (`cbs`, `harita`, `mimari`, `yol`), en az biri.
  - Her biri eklentinin manifestinde olmalı. Manifestte olmayan ya da KentOS'un bilmediği modül tanımı reddeder,
    nedenini söyler.
  - Yazılmazsa manifestin modülleri geçerlidir. Manifestin `modules` alanı zorunlu olduğu için öntanım yine açıktır.
- **`category`:** araç kutusundaki yeri, İşlemler'in kategorilerinden birinin kimliği (`analysis`, `geometry` …).
  - Başka bir değer ya da yokluğu aracı Eklentiler'de gösterir. Eski tanımlar alana eklentinin kendi adını yazıyordu
    (MEBİS: “Mezarlık BS”); onlar reddedilmez, önceki gibi Eklentiler'dedir.
  - Önce alan hiç okunmuyordu; Python aracı artık hazır araçların arasına girebilir.

Şema (`crates/shared/extensions/sdk/kentos-processing-tool.schema.json`) ve okuyucunun sınamaları güncellendi.

### 5. Örnek araç: Alan dilimleri

KentOS ile gelen örnek eklenti `paket/eklentiler/ornek-isleme/` klasöründedir (ADR 9312). Kurulumda
`<kaynak>/eklentiler/ornek-isleme/` olur.

- **Ne yapar:** kapalı alanların alanını hesaplar, artan sınırlara göre dilimini yazar, alanları yeni katmana kopyalar,
  dilimleri tabloya döker. Kütüphane istemez (alan, halkaların koordinatlarından).
- **Nerede çıkar:** Harita ve CBS (`modules`), Analiz kategorisi, masaüstünde ve başsız (`headless-python`).
- **README:** bir işlem aracının iki parçasını ve denemeyi anlatır.
- **Sınamalar:** Python sınamaları KentOS'suz koşar; Rust sınaması tanımın okunduğunu, modüllerini, kategorisini ve
  retleri sınar.

### 6. Modül kataloğu

`catalog/v1/modules.json`'a her modülün `processingTools` listesi eklendi. Masaüstü, bulut portalı ve ileride mobil
istemciler aynı listeyi okur (CLAUDE.md §25 kural 4).

### 7. Modüller

| Modül | Var / yok | Gerekçe |
|---|---|---|
| Harita | var | `SHARED`, `GIS_MAP`, `CAD_DRAWING` kümeleri: kadastro, imar, ölçme |
| CBS | var | `SHARED`, `GIS_MAP`, `GIS_ROAD`, `GIS` kümeleri: çözümleme ve veri |
| Yol | var | `SHARED`, `GIS_ROAD`, `CAD_DRAWING` kümeleri: arazi, drenaj, koridor, kamulaştırma parseli yazısı |
| Mimari | var (yalnız iki seçim aracı) | Araç kutusu yok; bina kendi koordinatında |

### 8. Kanallar

| Kanal | Var / yok | Nasıl |
|---|---|---|
| Python | var | `kentos.cad.processing.tools(doc)` belgenin modülünün araçlarını `modules` ile verir; `run` başka modülün aracını `tool_not_in_module` ile reddeder. SDK katalogdan üretildi. |
| MCP | var | Aynı komutlar (`cad.processing.tools`, `.run`, `.batch`), aynı ret. |
| Yapay zekâ ajanı | var | MCP ile aynı araç katmanı. |
| Eklenti | var | Eklenti işlem aracını aynı komutla çalıştırır, aynı rete uğrar. Kendi aracının modüllerini tanımında söyler (§4). |

## Uygulama

- **Ortak tanım:** `crates/native/launch/src/module.rs` (`ModuleSet`).
- **İşleme:**
  - `crates/native/processing/src/modules.rs` (kümeler, `PLACED`, `module_of`, `runs_on`, `refusal`, sınamalar);
  - `types.rs` (`Tool.modules`), `builtin/**` (her araç tanımı `modules: placed(id)`);
  - `model_runner.rs` (modelin kümesi), `registry.rs` (`modules_of`);
  - `command.rs` (liste süzgeci, ret, `modules` alanı);
  - `external.rs` ve `external/descriptor_schema.rs` (`modules`, `category`).
- **Sözleşme:** `codes.rs` (`TOOL_NOT_IN_MODULE`), `capabilities/gis.rs`, `cad_processing.rs` (`ProcessingTool.modules`);
  `commandCatalog.json` ve Python SDK yeniden üretildi.
- **Masaüstü:** `shell/program.rs` (`offers`, `module_offers`), `shell/catalog.rs` (`module_has`),
  `tests/module_catalog_tests.rs` (`processingTools`, matris sınaması).
- **Eklenti okuyucuları:** `apps/desktop/src/shell/extensions/mod.rs`, `crates/native/extension-tools/src/plan.rs`,
  `crates/native/headless/src/agent_tools.rs` manifestin modüllerini verir.
- **Örnek:** `paket/eklentiler/ornek-isleme/`; `crates/native/processing/tests/external.rs`.

## Doğrulama (26 Kasım 2026)

- **İşleme crate'i** (`kentos-processing`): lib 44 sınama (tablonun bütünlüğü, kümeler, Python/MCP yolundaki ret ve
  süzgeç), ortak durumlar 38, eklenti araçları 19 (örnek aracın tanımı dahil), toplu 18. Hepsi geçti.
- **Uygulama çekirdeği:** `cad.processing.tools`, `.run`, `.batch` komut durumları geçti; yeni ret durumları
  (`tool_not_in_module`) ve yetenek sınaması da.
- **Masaüstü:** `module_catalog` (matris) ve `module_tests` (modelin modülleri) geçti. Programlar arası sınamalar
  (`kentos-cad`) 37 geçti.
- **MCP ve ajan:** işleme sınamaları (83) geçti. Ajan sınamaları Mimari çizimdeydi, Harita çizimine alındı: Mimari
  yalnız iki seçim aracını görür.
- **Örnek araç:** `kentos kek verify paket/eklentiler/ornek-isleme` doğrulandı (0 uyarı, 1 katkı); Python sınamaları:
  `PYTHONPATH=paket/eklentiler/ornek-isleme/src .run/py/bin/python -m unittest discover -s paket/eklentiler/ornek-isleme/tests`.
- **Resimler:** `.run/gui-evidence/kullanim-9310/` (Harita'nın ve CBS'nin araç kutusu); üretici
  `cargo test -p kentos-cad --test programs tool_modules_screens -- --ignored --nocapture`.
- **Koşulmayanlar:** MCP'nin 166 sınama ikilisinin hepsi (yalnız işleme araçlarına dokunanlar), Python SDK'nın derlenmiş
  bağlayıcıyla ortak durum oynatıcısı (`pnpm py:test`).
- **Bu işten önce de düşenler** (main'de, bu değişiklikten bağımsız):
  - `module_deps` tabanı aşılmış (render-motoru birleştirmesi: shell→cad, gis→cad);
  - `source_size`'ın üç dosyası;
  - uygulamanın `file_digest` ve `module_features` sınamaları, `cad.blocks.edit`'in blok kitaplığı durumu;
  - çizimin `bands` sınaması; masaüstünün `open_report` sınaması; eklentilerin `icon_names` sınaması.

## Açık kalanlar

- **Kullanıcının betik klasörü:** eklenti paketlemeden, tek `.py` dosyasıyla işlem aracı (QGIS'in Scripts sağlayıcısı
  gibi) yapılmadı. Bugün Python aracı eklentidir; tek sözleşme korunur. Betik klasörü `<kullanıcı>/betikler`
  ayrıldı (ADR 9312); ondan araç okumak ayrı karardır.
- **KentOS ile gelen eklentinin kendiliğinden kurulması:** `<kaynak>/eklentiler` okunur yerdir, ama kayıt onu henüz
  kendi kaynağı olarak açmaz. Örnek, Eklentiler penceresinin “Paket yükle…” düğmesiyle (`kentos kek pack` ile paketlenip) kurulur.
