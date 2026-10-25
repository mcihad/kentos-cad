# ADR 0638: Atlas başka sistemde, nesneye kırpılmış, kaynağından; alanın kalıcı “dışarı verilmez” izni

- **Durum:** kabul edildi (2026-10-25). `plan/gis.md` CBS-21'in kalanı (“Mevcut atlas/GeoPDF'yi provider revision'ı ve export
  yetkisine bağla”): başka sistemde basma, nesneye kırpma, kapsam katmanının sağlayıcıdan sayfa sayfa okunması ve büyük
  katman ölçeği, haritaların yazıları, kalıcı alan izni. Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0534 (atlas PDF'i, sabitlenen sürüm), 0583 (harita sistemi `View.map`), 0504 (kutunun dönüşümü),
  0592 (kaynaktan okuma `sources_query::stage`), 0510 (parça), 0443 §2 (formun `redact`'ı), 0550 (tek erişim çözücüsü),
  0546 (dışarı çıkanlar), 0265 (yeni şemanın kuralı), 0164 (pafta düzeni, kırpma sınırı `Clip`).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **`.kcad`:** şema 107 (alanın `withheld`'i).
- **Sonraya kalan:**
  - haritanın kendi `crs`'i (paftada tek bir haritanın başka sistemi): atlas bütün sayfayı tek sistemde basar;
  - delikli ve çok parçalı atlas nesnesine kırpmada delikler ve küçük parçalar: kırpma en büyük parçanın dış sınırıdır;
  - kaynaktan okumada sunucu işi (sunucu bu bilgisayarın dosyasını okumaz) ve veritabanı kaynağının sürümü (dosyanın
    SHA-256'sı gibi sabitlenemez);
  - çıktının yan kayıt dosyası, sayfa küçük resimleri ve sayfa başına PDF (ADR 0534'ün listesinden);
  - haritaların yazıları ayrı bir iş değil: başsız çıktıda çizici `kentos_plot::labels`'tan yazılar zaten yazılıyor
    (CAD-SHEET-06); bu ADR yalnız onları başka sisteme taşır (§1).

## Karar

### 1. Başka koordinat sisteminde basma

`file.export.atlas`'ın `crs`'i artık projenin sisteminden başka, kayıttaki metre birimli bir sistem olabilir (TM, UTM,
Pseudo-Mercator). Proje sistemi olmayan proje, kayıtta olmayan kod ve derece birimli sistem `crs_unsupported` ile, nedeni
söylenerek reddedilir. Sözler `kentos_project::systems::paper_projection`'dan gelir.

- **Sayfalar:** her kapsam nesnesinin kutusu çıkış sistemine taşınır (`crs::transform_bounds`, kenar başına 21 iç nokta,
  PROJ'un `densify_pts`'i gibi). Planlayıcı ortayı ve ölçeği bu kutuya göre seçer. Cevapta sayfanın `center`'ı ve ölçeği
  çıkış sisteminde, `bbox`'u projenin sistemindedir (nesnenin kendisi). Sistemin kapsamadığı nesne sayfa olmaz ve söylenir.
- **Haritalar:** stil motoru çizimi çıkış sistemine taşıyarak kurar (`View.map`, ADR 0583 §3): kenarlar eğildikçe bölünür.
  Nesneler projenin sisteminde, haritanın kutusunun geri taşınmış hâliyle aranır (`maps::project_box`).
- **Duran haritalar:** paftanın kendi ortası olan haritaları da çıkış sistemine taşınır.
- **Yazılar:** projenin sisteminde yerleşir, noktası ve halkaları taşınır. Dönüşüne grid yakınsaması (doğunun çıkış
  sistemindeki yönü) eklenir (`labels::moved`).
- **Pafta:** antet, ızgara ve kuzey çıkış sistemini söyler. GeoPDF görüntü pencereleri o sistemdedir. Cevabın
  `bindings`'i `srid` (çıkış) ve `projectSrid`'yi (proje) verir. `Keywords`'te `koordinat=EPSG:…; proje=EPSG:…` yazar.

### 2. Atlas nesnesine kırpma

Haritanın “atlas nesnesine kırp” seçeneği (`clipToAtlasFeature`) artık uygulanır. Kırpma pafta çekirdeğinin kendi
çiziminde olur, PDF, SVG ve masaüstünün önizlemesi aynı sınırı kullanır:

- atlas sayfasının nesnesi (`AtlasFeature.outline`) dış sınırını taşır: alan, kapalı çoklu çizgi, daire ya da elips;
  yaylar kâğıtta küçük kirişlerle;
- `display::map::feature_clip` sınırı içerik çerçevesinin milyonda birlerine çevirir ve çerçeveye kırpar
  (Sutherland–Hodgman);
- en çok 256 köşeye kadar inceltir (Douglas–Peucker, tolerans çerçevenin binde birinden başlayıp ikiye katlanır);
- kırpma kuralına uymazsa çerçeve kalır.

Kırpma, haritanın kendi biçimli kırpmasının yerine geçer. Alanı olmayan nesnenin sayfası kırpılmaz ve söylenir.

### 3. Kapsam katmanı kaynağından

`coverage: "source"` bağlı kapsam katmanının bütün tablosunu sayfa yapar. Bağlı olmayan katman `coverage_not_bound` ile
reddedilir.

- **Kapsam tablosu:** `sources_query::stage` ile, `cad.sources.query`'nin okuduğu gibi parça parça okunur. Katmanın
  süzgecinin ifadesi ve zamanı kaynağa gider, satırlar yine çizimin kuralıyla süzülür.
- **Satırların yeri:** okunan satırlar bir okuma kopyasında katmanın nesnelerinin yerini alır (`atlas::with_rows`). Çizim
  değişmez.
- **Parçalı katmanlar:** parçayla bağlı her diğer harita katmanı, her sayfada o sayfanın atlas haritalarının erişiminde
  kaynaktan okunur. Parçanın kendi nesneleri (yazılmamış düzenlemeler) kalır, aynı kimlikli satır alınmaz. Sayfa bitince
  satırlar çıkar ve geometri deposu günlükle izler (`Spatial::sync`). Bir sayfanın okuduğu sınırlıdır: yalnız o sayfanın
  kutusu.
- **Duran haritalar:** ilk sayfanın satırları gelmeden, çizimin hâliyle bir kez çizilir.
- **Bant:** kaynaktan okunan satırların yerine parçanın bandı (ADR 0636) eklenmez; satırlar kendileri vardır.
- **Cevap:** cevabın `read`'i okunanları söyler: kapsam tablosundan alınan satırlar ve her parça katmanından sayfa sayfa
  alınanlar.
- **Pin ve sunucu:** pin ADR 0534'teki gibi dosyanın SHA-256'sını tutar; sayfalar yapılırken dosya değişirse
  `source_changed`. Sunucu işi bu bilgisayarın dosyasını okumaz: masaüstü kaynaktan atlası sunucuya göndermez.

### 4. Büyük katmanda sayfalar

Her sayfa, haritasının kutusundaki nesneleri geometri deposunun R-ağacından alır (`Store::reaching`). Önceden her katmanın
bütün nesneleri her sayfada taranıyordu. PDF yazıcısında da üç iyileştirme var:

- **Çizim durumu:** renk, kalınlık, uç, köşe ve kesik değişmedikçe yeniden yazılmaz (`Page::fill_rgb`, `stroke_rgb`,
  `line_style`'ın belleği; `q`/`Q` ile saklanır).
- **GeoPDF yer tutucuları:** dosyada tek geçişte doldurulur. Önceden her sayfa için bütün dosya baştan aranıyordu.
- **Sıkıştırma:** sayfalar sekizer kurulur, içerikleri yan yana sıkıştırılır, sırayla yazılır. Dosya bayt bayt aynıdır.

### 5. Alanın kalıcı izni: “Dışarı verilmez”

Katman alanının `withheld`'i (Alanlar penceresinde “Dışarı verilmez”) değerleri çizimde tutar, form gerekmeden.
Formun `redact`'ı gibidir:

- tablo ve form gösterir;
- atlas, Öznitelik tablosunun kopyası ve dosyası, İstatistik ve Grafik, komut kaydının Python'u ve sunucunun kesilmiş
  kopyaları onu dışarıda bırakır.

Erişim çözücüsü bunu tek yerden yapar: `reach::withheld_names`, `Reach::withholding`, `python_call::redacted_names`.
`.kcad` şema 107 alanı yalnız `true` iken yazar. Şema 2–106'da `withheld` bilinmeyen alandır; `false` yazılmışsa
reddedilir.

## Doğrulama

- **Ortak durumlar:** `fixtures/atlas/v1/cases.json` 35 durum; yenileri:
  - TM30 ve UTM 35N'de sayfalar (ortalar PROJ'la, pyproj, milimetre içinde);
  - derece birimli ve kayıtta olmayan sistem;
  - kaynaktan 105 sayfa, çizimden parçanın sayfaları;
  - sayfa sayfa okunan bina satırları (bağımsız hesapla aynı sayı);
  - bağlı olmayan kapsam;
  - formsuz “dışarı verilmez”.

  Kaynağı GDAL'ın yazdığı `fixtures/atlas/v1/atlas.gpkg`. Hepsi KentOS kodu olmadan `atlas_cases.py`'den; Rust'ta
  `atlas_cases.rs` oynatır (`$near`, `copy`, `take`).
- **Erişim:** `fixtures/reach/v1` 19 durum (üçü `withheld`).
- **Şema 107:** `fixtures/kcad/v2/withheld-fields.kcad`, `broken/withheld-fields-in-schema-106.kcad`,
  `broken/withheld-false.kcad`, `broken/schema-version-108.kcad`. Bağımsız yazıcı `kcad_v2_reference.py`, okuyucu
  `tools/kcad/kcad.py`; taşıma matrisi alanın her anahtarıyla.
- **Kırpma:** pafta çekirdeğinde `atlas_clip.rs` (dörtgen, 2000 köşeli daire, çerçeveden taşan L); çizicide kırpma
  alanlarının küçüldüğü.
- **Masaüstü:** `the_window_draws_in_another_system_and_reads_the_coverage_from_its_source`. Resimler `atlas-*-5-sistem`,
  `atlas-*-6-kaynak` ve `arac-form-alan-disari-verilmez`; 1440×900 ve 1100×650, iki tema.
- **Python:** `test_atlas.py` başka sistem ve bağlı olmayan kapsam.
- **Süreler** (bu makinede, release, `atlas_timing.rs`), 100 sayfa, 320 × 320 = 102 400 parselin üstünde:

  | Durum | Plan | PDF |
  |---|---|---|
  | Çizimden, önce | 211 ms | 8 585 ms (13,0 MB) |
  | Çizimden, şimdi | 224 ms | 2 723 ms (12,2 MB) |
  | Kaynaktan (102 400 satırlık GeoPackage'tan) | 42 ms | 6 377 ms |

  Kaynaktan PDF'te her sayfa kendi parsellerini okudu: 100 sayfada 564 208 satır. Kaynak dosya GDAL'la
  `.run/perf/atlas-100k.gpkg`'ye yazılır: `parsel` 10 m'lik kareler, `pafta` 320 m'lik kareler.

  “Önce”den “şimdi”ye adım adım:

  | Adım | PDF |
  |---|---|
  | Çizim durumunun belleği | 8,1 s (dosya 13,0 → 12,2 MB) |
  | Yer tutucuların tek geçişte doldurulması | 5,4 s |
  | R-ağacından seçim (haritaların kurulumu 2,4 → 1,3 s) | 4,2 s |
  | Yan yana sıkıştırma | 2,7 s |
