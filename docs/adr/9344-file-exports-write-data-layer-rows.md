# ADR 9344: Dosya dışa aktarmaları veri katmanlarının satırlarını yazar; tablonun seçtiği satır Seçilenleri kaydet'e girer

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 27 Kasım 2026, ikinci tur
  (`gis-eksik-27-11-2026.md`, T1 P1 ve T2 P2). Sahibin sözü: “yeni maddeler ekledim onları düzelt komple”.
- **İlgili:**
  - [ADR 9338](9338-export-and-package-data-layer-rows.md) (GeoJSON ve Paketle kaynak satırlarını yazar);
  - [ADR 9339](9339-cbs-selection-copy-with-its-geopackage.md) (CBS'de Seçilenleri kaydet);
  - [ADR 1073](1073-geopackage-export-scope.md) ve [ADR 9191](9191-geopackage-delivery-into-a-file-that-is-there.md) (GeoPackage dışa aktar);
  - [ADR 9101](9101-data-layers.md) (veri katmanı).

## Bağlam

- **T1:** yalnız veri katmanları olan projede Dosya › Dışa aktar › GeoPackage › Tüm proje kaynak tablolarını boş saydı.
  - Başarı “3 tablo, 0 satır” dedi.
  - CSV, Shapefile, FlatGeobuf, KML/KMZ, GeoParquet ve GML yazılacak nesne bulamadı.
  - Aynı kısa adı taşıyan iki katmanın tablo adları yazılmadan önce söylenmiyordu.
  - Bu pencereler ve komutlar yalnız çizimin nesnelerini okuyordu. ADR 9338 bunu yalnız GeoJSON ve Paketle için
    çözmüştü.
- **T2:** İfadeyle seç bir kaynak satırını tablonun seçimine aldı; Denetçi `Veri satırı (1)` gösterdi. Seçilenleri
  kaydet yine kapalıydı, çünkü yalnız çizimin seçimine bakıyordu.

## Araştırma

- **QGIS, var olan katmandan yeni katman:** bir vektör katman, kaynağı ne olursa olsun Farklı kaydet ile her biçime
  yazılır; “yalnız seçili nesneler” seçeneği vardır
  ([Creating new layers from an existing layer](https://docs.qgis.org/latest/en/docs/user_manual/managing_data_source/create_layers.html#creating-new-layers-from-an-existing-layer)).
- **KentOS kararı:** satırları dışa aktarmanın tek yolu var: çizimin satırlarla birlikte kopyası. Pencere sayımı ve
  yazmayı bu kopya üstünden yapar; komut her kanal için aynısını kendisi kurar.

## Karar

### 1. Kaynak kopyası (masaüstü)

- **Kopya** (`shell/exchange/source_copy.rs`): çizim, her veri katmanının satırlarıyla birlikte kopyalanır.
  - Satırlar tabloların okuduğu gibidir (ADR 9130: kaynaktan bütün olarak, arayüzün iş parçacığı dışında, çizim
    değişmedikçe saklanır).
  - Satırlar kendi katmanlarına nesne olur; katmanların bağı kopyada kalkar.
  - Çizimin tuttuğu satırlar (alınmış, yeni) zaten nesnedir, iki kez konmaz.
  - Kopya çizim değişince ya da kaynağın satırları gelince yeniden kurulur.
- **Kullanan pencereler:** GeoPackage, Shapefile, CSV, FlatGeobuf, GeoParquet ve GML dışa aktar sayımı ve yazmayı
  bu kopyadan yapar. KML/KMZ ve GPX, GeoJSON'un kapsamlarını kullanır (`export_objects`, `export_by_layer`).
- **Okunurken ve okunamayınca:**
  - Kaynak okunurken plan “Kaynaktan okunuyor: …” der, yazılacak nesne yok demez.
  - Satırlar gelince planlar yeniden kurulur.
  - Okunamayan kaynak adıyla söylenir.

### 2. GeoPackage'da aynı adlı katmanlar

Yazıcı aynı adlı katmanlara `_2`, `_3` … ile ayrı tablolar verir (`geopackage::table_name`). Pencere bunu yazmadan
önce söyler: “Aynı adlı katmanların tabloları: Senaryo 1 / qa_yollar → qa_yollar_2.” Başarı yazılan tabloları ve
satırları söyler (3 tablo, 7 satır).

### 3. Komut her kanalda (Python, MCP, ajan)

`file.export.gpkg`, `shp`, `csv`, `fgb`, `parquet`, `gml` ve `xyz` veri katmanı olan çizimde satırları kaynaktan
okur (`package_data::typed_with_rows`). Kaynağı okunamayan katman komutu `source_unreachable` ile reddeder; ret
katmanın adını söyler ve hiçbir dosya yazılmaz.

### 4. Seçilenleri kaydet (T2)

- **Seçim:** çizimin seçimi ve bir veri katmanının tablosunda seçili satırlar birlikte seçimdir. Satır seçiliyken
  düğme açıktır.
- **Kopya:** seçili satırların nesneleri kaynak kopyasından alınır. CBS projesinde kopyanın türü sorulur (ADR 9339).
  GeoPackage'lı kopyada satır yeni GeoPackage'a adı, geometrisi ve sistemiyle yazılır.

### 5. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Dışa aktarmalarda kaynak satırları | yok: Mimari'de veri katmanı yoktur | var: CBS tabloları altlık olabilir | var: aynı gerekçe | var |
| Aynı adlı katmanların tablo adları | yok: Mimari'de GeoPackage dışa aktarma yoktur | var | var | var |
| Seçilenleri kaydet'te tablo satırları | yok: aynı gerekçe | var | var | var |

### 6. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | var | Dışa aktarma komutları satırları kaynaktan okur (§3). |
| MCP | var | Aynı komutlar. |
| Yapay zekâ ajanı | var | Aynı komutlar. |
| Eklenti | yok | Dosya işleri eklentiye kapalıdır. |

## Açık

- **Büyük tablolar:** kopya satırları bütün olarak okur. Milyonlarca satırlık bir tabloda dışa aktarmanın kaynaktan
  akan yolu daha uygundur (`cad.layers.export`).
- **Kanalların farkı:** komut düzeyinde okunamayan kaynak bütün dışa aktarmayı reddeder; masaüstü penceresi o katmanı
  söyleyip öbürlerini yazar.
- **Gerçek pencere:** fare ve klavyeyle kabul (`scripts/usage/real/`) bu dilimde yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Masaüstü:** `source_formats_tests`. Örnek: `qa_noktalar` 3, `qa_yollar` 3 satır ve Senaryo 1'de `qa_secim`'i
  okuyan ikinci `qa_yollar` 1 satır.
  - GeoPackage › Tüm proje “7 nesne, 3 katman” ve “Senaryo 1 / qa_yollar → qa_yollar_2” diyor. Dosyada `qa_noktalar`
    3, `qa_yollar` 3, `qa_yollar_2` 1 satır.
  - FlatGeobuf, GML ve CSV planları kuruluyor; Shapefile üç dosya planlıyor.
  - Tabloda seçilen satır Seçilenleri kaydet'i açıyor. CBS kopyasının GeoPackage'ında `qa_yollar` 1 satır, `AB`,
    `srs_id` 5256.
  - `file.export.gpkg` komutu çizimin kendisinde (kanalların yolu) 3, 3, 1 satır yazıyor.
  - `-- exchange:: drawing_exchange module_tests ribbon_tests history::text catalog settings`: 358 geçti. Kırmızı
    tek sınama bilinen bağımlılık tabanıdır.
- **Headless:** `cargo test -p kentos-headless --test headless -- package gpkg csv fgb shp`: 27 geçti.
- **Resim** (`.run/gui-evidence/kullanim-9344/`, açık ve koyu tema; büyütülerek bakıldı): `01-geopackage-kaynak`:
  Tüm proje (7), katmanların satırları, aynı adlı katmanın tablo adı.
