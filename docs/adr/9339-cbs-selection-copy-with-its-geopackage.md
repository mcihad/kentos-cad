# ADR 9339: CBS'de Seçilenleri kaydet kopyanın türünü sorar; seçim GeoPackage'ıyla birlikte CBS projesi olur

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 27 Kasım 2026 (`gis-eksik-27-11-2026.md`, madde
  1, P1). Sahibin sözü: “Gis eksik 27 11 dosyasındaki maddeleri tamamla”.
- **İlgili:**
  - [ADR 0193](0193-drawing-exchange.md) (Seçilenleri kaydet, çizimler arası alışveriş);
  - [ADR 9196](9196-storage-by-project-kind.md) §2 (CBS verisi tablolardadır) ve
    [ADR 9321](9321-one-save-moves-every-held-layer.md) (kayıt nesne tutan katmanları taşıtır);
  - [ADR 9107](9107-project-in-a-source.md) §8 (`cad.layers.migrate`) ve
    [ADR 9106](9106-layer-export.md) (`cad.layers.export`'un `existing` seçenekleri).

## Bağlam

- **Gözlem:** CBS projesinde İfadeyle seç ile seçilen AB yolu Dosya › Seçilenleri kaydet ile KCAD'e yazıldı. Kopya yeni
  sekmede açıldı, bir GeoPackage veri katmanı olarak eklendi. Kaydet, seçimden kopyalanan çizgi katmanını önce veri
  deposuna taşımayı istedi. Kullanıcı taşıma için ayrı bir GeoPackage kurmak zorunda kaldı.
- **Kod:** `file.saveSelection` seçimin çizimini her projede nesneleri proje dosyasında tutan bir KCAD olarak
  yazıyordu. CBS projesi nesne tutan katmanı proje dosyasına yazmaz (CLAUDE.md §25 kural 5); bu yüzden kopya
  açıldığı gibi kaydedilemiyordu.
- **İncelemenin kabulü:**
  - CBS seçim kopyası için oluşturma anında GeoPackage hedefi ya da açık bir Çizim kopyası seçeneği olsun.
  - Kaynak bağlantıları ve proje aynı akışta yazılsın.
  - Tabloda iki kayıt varken ekleme, yeni tablo ve üzerine yazma ayrı doğrulansın.
  - Seçilen çizgi yeniden açılıp yeni katman eklenince normal Kaydet ve yeniden açma aynı satır kimliğini, öznitelik
    değerini ve CRS'yi versin.

## Araştırma

- **QGIS, seçilenleri kaydet:** Farklı kaydet penceresinde “yalnız seçili nesneleri kaydet” işaretlenir. Hedef bir
  GeoPackage ise katman var olan dosyaya yeni tablo olarak eklenir; aynı adlı tablo için “üzerine yaz” ya da “ekle”
  sorulur ([Creating new layers from an existing layer](https://docs.qgis.org/3.44/en/docs/user_manual/managing_data_source/create_layers.html)).
- **ArcGIS Pro, Export Features:** seçili nesneler yeni bir sınıf olarak dosya veritabanına yazılır; var olanın
  üzerine yazmak ortam ayarıyla açılır
  ([Export Features](https://pro.arcgis.com/en/pro-app/latest/tool-reference/conversion/export-features.htm)).
- **KentOS kararı:**
  - CBS'nin kopyası da CBS projesidir: verisi tablolarda, proje dosyası onları okur.
  - Tabloyu yazan ve katmanı ona bağlayan iş, kaydın kendi taşıma komutudur (`cad.layers.migrate`). Seçenekleri her
    kanalda aynı olsun diye “var olan tablo” seçeneği komuta eklendi.

## Karar

### 1. Kopyanın türü önce sorulur

- **Soru:** seçimin çiziminde CBS'nin tabloda tuttuğu bir katman varsa (`layer_kind::held_in_file`) Seçilenleri kaydet
  dosyayı sormadan önce “Seçilenleri kaydet” penceresini açar. Pencere seçili nesne sayısını ve tabloya yazılacak
  katmanları söyler.
- **CBS projesi ve GeoPackage** (öntanımlı):
  - Seçim, kaydedilecek proje dosyasıyla aynı adlı GeoPackage'a (`<ad>.gpkg`, yanında) yazılır: her katman
    `cad.layers.migrate` ile tablosuna taşınır, proje dosyası onları okur. Katmanın yolu proje dosyasından görelidir.
  - Kopya açılınca doğrudan kaydedilir; taşıma sorulmaz.
  - Bütün taşımalar önce planlanır: tabloya yazılamayan nesne (ölçü, blok, tarama) varsa hiçbir dosya yazılmaz.
  - Yazım yarıda kalırsa kopya için yapılan GeoPackage silinir.
- **Çizim kopyası:** nesneler proje dosyasında kalır (ADR 0193 §1). CAD modüllerinde açmak içindir; CBS'de açılınca ilk
  Kaydet taşımayı ister. Pencere bunu söyler.
- **CAD projeleri:** Mimari, Harita ve Yol'da pencere çıkmaz; kopya eskisi gibi proje dosyasıdır.

### 2. GeoPackage'da aynı adlı tablo

- **Yeni tablo** (öntanımlı): seçim adının sonuna `_2`, `_3` … eklenmiş yeni tabloya yazılır; var olan tablo değişmez.
- **Üzerine yaz:** tablo silinip seçimle yeniden yapılır; GeoPackage'ın öbür tabloları kalır.
- **Ekle:** satırlar tablonun satırlarının ardına eklenir; katman bütün tabloyu okur.

### 3. `cad.layers.migrate`'in `existing` seçeneği

- `existing`: `refuse` (yok sayılınca da), `replaceLayer`, `append`. `cad.layers.export`'un seçenekleridir ve aynı
  biçimde işler. Yazıların yanındaki tablosu da aynı yoldan gider.
- `overwriteFile` taşımada `invalid_option` ile reddedilir (sıra: `no_target`'tan sonra): kaynağın öbür tabloları
  silinirdi.
- `target_exists` yalnız `refuse`'ta söylenir.
- Var olan tabloya yazıldıktan sonra yazı tablosu yazılamazsa o tablo düşürülmez: önceki satırları da giderdi. Ret
  iletisi bunu söyler.

### 4. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Seçilenleri kaydet'in kopya türü sorusu ve GeoPackage'lı kopya | yok: CAD projesinin tamamı proje dosyasındadır | yok: aynı gerekçe | yok: aynı gerekçe | var |
| `cad.layers.migrate`'in `existing`'i | yok: taşıma yalnız CBS'dedir | yok | yok | var |

### 5. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | var | `cad.layers.migrate(existing=…)` (SDK yeniden üretildi). Seçilenleri kaydet'in kendisi masaüstünün dosya penceresidir; betik aynı kopyayı seçimin çizimi, `cad.layers.migrate` ve kayıtla kurar. |
| MCP | var | Aynı komut, aynı seçenek. |
| Yapay zekâ ajanı | var | Aynı komut. |
| Eklenti | yok | Dosya işleri ve kaynağa yazma eklentiye kapalıdır. |

## Açık

- **Veri katmanının seçili satırları:** Seçilenleri kaydet çizimin seçimini yazar. Tablonun seçtiği kaynak satırları
  (veri katmanında İfadeyle seç) kopyaya girmez.
- **Python ortak durumları:** `python/tests/test_command_cases.py` yeni durumlarla bu dilimde koşulmadı (yerel
  modülün yeniden derlenmesi gerekir). Rust'taki koşucu aynı dosyayı koştu.
- **Gerçek pencere:** fare ve klavyeyle kabul (`scripts/usage/real/`) bu dilimde yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Ortak durumlar:** `scripts/fixtures/layer_migrate_command_cases.py` 33 durum (dört yeni):
  - Ekle'de satırlar 1–6;
  - Üzerine yaz'da 1–3;
  - `refuse` `target_exists`;
  - `overwriteFile` `invalid_option`.
  - `KENTOS_CASES_ONLY=cad.layers.migrate cargo test -p kentos-native-application --test all every_command_case`
    geçti; `--check` aynı.
- **Katalog:** `KENTOS_WRITE_CATALOG=1 cargo test -p kentos-commands --lib catalog` (13 geçti), `python3
  scripts/python/sdk.py`, `python3 scripts/cad/manifest.py --check` (1096 satır tutarlı).
- **Masaüstü** (`shell/exchange/selection_save_tests.rs`). Örnek: CBS projesi, EPSG:5256, `yollar` katmanında
  `ad=AB` yolu.
  - `a_cbs_selection_is_a_project_with_its_geopackage_and_saves_at_once`:
    - pencere önce soruyor; `secim.gpkg`'de `yollar` 1 satır (`fid=1`, `AB`);
    - kopyanın proje dosyası nesne tutmuyor, katman tabloyu `secim.gpkg` göreli yoluyla ve 5256 ile okuyor;
    - açılan kopyaya GeoPackage veri katmanı eklenince Kaydet taşıma sormadan yazıyor;
    - yeniden açılınca satır 1, `AB`, sistem 5256, yeni katman yerinde.
  - `a_table_of_the_same_name_is_given_its_own_made_again_or_added_to`: Ekle 2 satır, Üzerine yaz 1 satır, Yeni tablo
    `yollar_2`'ye yazıyor ve kopya onu okuyor.
  - `a_drawing_copy_keeps_the_objects_in_its_file`: Çizim kopyası GeoPackage yapmıyor, nesne dosyada.
  - `a_layer_that_no_table_holds_writes_nothing`: ölçülü seçimde “Hiçbir dosya yazılmadı”, ne proje ne GeoPackage
    kalıyor.
  - `-- drawing_exchange layer_migrate selection_save source_export cbs_storage module_tests ribbon_tests history::text
    catalog settings`: 178 geçti. Kırmızı tek sınama bilinen bağımlılık tabanıdır (`shell/approved_layer.rs`,
    `shell/gpu_frame.rs`, `gis/data_selection.rs`; bu dilimden değil). Seçilenleri kaydet'in yazıcısı kabuğa taşındı;
    kabuktan CAD'e yeni bağ yok.
- **Resim** (`.run/gui-evidence/kullanim-9339/`, açık ve koyu tema; büyütülerek bakıldı): `01-secilenleri-kaydet`:
  Kopyanın türü, aynı adlı tablo seçenekleri, tabloya yazılacak katman.
