# ADR 0563: Shapefile, GeoJSON ve KML/KMZ içe aktarma komutları; sunucunun iki köşeli yaylı alan kuralı

- **Durum:** kabul edildi (2026-10-21). `plan/gis.md` CBS-05'in (dosya alışverişi) ve CBS-39'un (sunucu işleri) devamı; koordinatörün
  isteği: [ADR 0561](0561-import-atlas-and-mcp-jobs.md)'in “Sonraya kalan”ındaki Shapefile, GeoJSON ve KML içe aktarma işleri ve bu dilimde
  görülen sunucu hatası. Kapsamı koordinatörün maddelerine uyarak ben belirledim.
- **Bağlam belgeleri:** ADR 0046 ve 0053 (GeoJSON ve Shapefile okuyucuları, pencereler), 0222 (KML/KMZ, çevirmenin açık onayı), 0165 §2 (yerel proje),
  0502, 0522, 0544, 0545 (`file.import.gpkg`, `.csv`, `.fgb`, `.gml`), 0257 (`file.import.dxf`: aynı yönlendirme kuralları), 0253 (komut protokolü),
  0069 (iki köşeli yaylı alan), 0561 (`project.import` işi), CLAUDE.md §9.2 (araç = komut).
- **Web:** ertelendi (ADR 0228); web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - GeoJSON ve KML için “Sunucuya içe aktar” düğmesi masaüstü pencerelerinde (iş türü hazır; ADR 0561'in ortak penceresi bu üç pencereye bağlanmadı);
  - Shapefile'ın yan dosyalarının ayrı hazırlanması (iş yalnız `.zip` arşivini alır);
  - KML'in NetworkLink ve resim kaplamaları (okuyucu söyler, almaz), `.kmz` içindeki öbür `.kml` dosyaları;
  - dışa aktarma tarafı: `file.export.kml` ve GeoJSON'un komutu yok (pencere dosyayı kendisi yazıyor, §9.2 borcu; `file.export.shp` var).

## Bağlam

Masaüstünde GeoJSON, Shapefile ve KML/KMZ pencereleri dosyayı bir kez okuyup nesneleri **kendileri** çizime yazıyordu
(`gis_import.rs`'in `apply_import`'u): ürün komutu yoktu. CLAUDE.md §9.2 (“araç = komut”) bunu borç sayar: Python ve MCP aynı işi yapamaz, sunucu işi
(`project.import`, ADR 0561) bu üç biçimi alamaz, günlük pencerenin yazdığını komutun adıyla göstermez, izlenen kural (sistemin sorusu, katmanların yeri)
bir pencerenin içinde kalır. GeoPackage, FlatGeobuf, CSV, GML ve DXF'in komutu vardı; bunlar kalmıştı. Ayrıca ADR 0561'in canlı denemesinde sunucunun
masaüstünün kabul ettiği bir alanı reddettiği görüldü: yaylı kenarla iki köşeli alan (daire, mercek; ADR 0069) için “Alan en az 3 köşe ister”.

## Karar

### 1. `file.import.shp`, `file.import.geojson`, `file.import.kml`

Üçü DXF içe aktarmanın biçimindedir (`ImportResult` → ortak yönlendirme → `import_apply`), GeoPackage ailesinin sistem kuralıyla; kod `file_import_gis.rs`'te, her komutun
kendi dosyası (`file_import_shp.rs` …) yalnız yolu ve biçimi söyler. Okuyucular değişmedi (`kentos_formats::{shp, geojson, kml}`): nesneler, öznitelikler metin, katmanların alanları türleriyle
(ADR 0199 §6), dosyanın bildirdiği sistem (`DeclaredCrs`), alınamayanların raporu.

**Girdi** (ortak): `path` (mutlak; Shapefile için `.shp` ya da `.zip`), `srid`, `reproject`, `skipLayers`, `layers` (dosyanın katmanı → çizimin katman kimliği), `layer` (başka
adlanmayan her katmanın gittiği katman kimliği), `group`, `expectedRevision`; Shapefile'ın ayrıca `entry`'si. **Dosya nasıl bulunur:** `.shp`'nin yanındaki `.shx`, `.dbf`, `.prj`, `.cpg` adından, büyük küçük harfe
bakmadan (pencerenin “birlikte seçilen dosyalar” kuralı); bulunanlar raporun “Dosyalar” satırındadır, eksik `.dbf` öznitelik yok demektir. `.zip` arşivinin her `.shp`'si bir katmandır; `entry` arşivdeki yolla
(`veri/yol` ya da `veri/yol.shp`) ya da adla (aynı adlı iki katmanda yolla) birini seçer, verilmezse hepsi alınır (raporlar toplanır, kutu birleşir, sistem ilk katmanınki,
farklıysa not düşer). Arşivde aynı adlı iki katman ve `entry` yoksa `entry_not_found`.

**Sistemin sorusu veri olarak** (ADR 0222'nin kuralı):
1. Koordinatların sistemi: `srid` verilmişse o (dosyanın dediğinden başkaysa `crs_overridden` uyarısı, dönüştürülmez); yoksa dosyanın bildirdiği (`.prj`, `crs` üyesi, RFC 7946'nın ve KML'in WGS 84'ü);
   yoksa projenin (`crs_undeclared`). Bildirim okunamıyorsa ya da KentOS'ta tanımlı olmayan bir EPSG kodunu söylüyorsa `crs_unknown` (`srid` ile cevap verilir).
2. Projenin sistemindeyse olduğu gibi. **Yerel proje** (sistemsiz) her dosyanın koordinatlarını olduğu gibi alır (ADR 0165 §2); orada `reproject` çevrilecek sistem olmadığından `crs_mismatch`.
3. Başka sistemdeyse yalnız `reproject` ile: köşeler Koordinat dönüştür'ün yoluyla (`kentos_formats::reproject`: projeksiyonlar, datum kayması, projenin datum seçimleri), kotlar olduğu gibi. İstenmemişse
   `crs_mismatch` (`reproject` yolunda). Çevrilemezse (sistem kayıtta yok, ortaları ulaşılamıyor) yine `crs_mismatch`, nedeniyle.
4. **Cevabın `moved`'ı** çevirmenin yolunu ve doğruluğunu söyler: `from`, `to` (projenin kendi tanımlı sistemiyse 0), `via` (datum işlemleri, projeksiyonda boş), `accuracy` (m, adımın doğruluğu yazılı değilse yok),
   `unofficial`, `text` (“±2.1 m, EPSG:1783 + EPSG:5260; resmî dönüşüm değil”, “kesin, yalnız projeksiyon”: pencerelerin bugüne dek gösterdiği cümle), çevrilen ve çevrilemeyip dışarıda kalan nesne sayısı. Dosyanın kutusunun
   ortasındaki noktanın yolu esas alınır (pencerede olduğu gibi). Resmî olmayan işlem `crs_unofficial` uyarısıdır. Çevrilemeyen nesneler alınmaz ve raporda yazılır (`import_losses`).

**Katmanların yeri** DXF'inkiyle ortak bir işlevden (`file_import::route_layers`): `skipLayers` bırakır, `layers` ve `layer` kimlikle yönlendirir (olmayan `layer_not_found`, grup `not_a_layer`,
kilitli `layer_locked`; yol `layers.<ad>` ya da `layer`), kalan her katman çizimdeki aynı adlı katmana (büyük küçük harf ve Türkçe işaretler ayrılmadan; kilitliyse `layer_locked`), o da yoksa dosyanın alanlarını
taşıyan yeni katmana gider; yeni katmanlar `group`'ta (verilmezse dosyanın adıyla bir grup, boşsa ağacın üstü) açılır. Alınacak nesne kalmazsa `nothing_to_import`. DXF'in yerleşik `plan_of`'u aynı işlevi kullanır
(davranışı değişmedi).

**Adım ve günlük:** tek geri alma adımı “Shapefile: ad.shp”, “GeoJSON: ad.geojson”, “KML: ad.kml” (`call::recorded`); günlükte komutun kendi satırı ve girdisi, `kentos.replay/1` başka çizimde aynı sonucu verir
(sınanır). Nesnelerin kalıcı kimliği her seferinde yenidir (aynı dosya iki kez alınabilir; GeoPackage ailesinin satır kimliği burada yok: Shapefile ve GeoJSON'un kalıcı satır kimliği yok).

`plan` aynı yolu bir okuma kopyasında yürütür; `validate` aynısını sonucu atarak.

### 2. Pencereler komutla yazar

Üç pencere de (tek `gis_import.rs`) dosyayı kendi iş parçacığında okur, sistemin sorusunu ve katman listesini gösterir; **İçe aktar** artık `execute_read` ile yazar (DXF penceresinin yolu): girdi pencerenin
seçimlerinden kurulur (`GisInput`: yol, zip'te katmanın `entry`'si, kullanıcının dosyanın dediğinden başka bir sistem seçmişse `srid`, “Projenin sistemine çevir” ise `reproject`, kaldırılan ve aynı adlı katmanı kilitli olan
katmanlar `skipLayers`), okunan dosya komuta verilir; çevirme pencerede iş parçacığında yapılmışsa komut onu yapılmış sayar (`Premoved`: hangi sistemden, kutunun eski hâli, dışarıda kalanlar) ve bir daha çevirmez. Böylece pencere
ile betik aynı yazar, günlük komutun adını taşır, arayüzün mesajları (“… nesne … katmana alındı; … yeni katman … grubunda”) ve geri alma değişmez. Kabul edilmeyen, bozuk sayı taşıyan nesneler
komuttan önce pencerede zaten engellenir (`unusable`), komutta `import_unreadable`.

### 3. Sunucu işi (`project.import`) üç biçimi alır

`ImportFormat` üç değer kazandı: `shp` (arşivin kendisi: `.zip`; `import.zip` olarak çocuğa bağlanır, yan dosyalar arşivin içindedir), `geojson` (`.geojson`, `.json`), `kml` (`.kml`, `.kmz`; KMZ'yi komut
baytından anlar). Dosya adından biçim çıkarımı (`of_name`) buna göre; `.shp` tek başına ve `.dbf` gibi yan dosyalar hâlâ biçim değildir (iletide kabul edilen uzantılar sayılır). İşin seçenekleri komutun girdisidir
(`entry`, `srid`, `reproject`, `layers` …; `path` yine reddedilir), komutun reddi işin kodudur (`crs_mismatch`, `entry_not_found`, `crs_unknown` …), uyarılar sonucun notlarındadır. Çocuk süreç, bellek kestirimi ve
kesinleşme ADR 0559 ve 0561'inkiyle aynıdır; bağımsız sunucu mantığı eklenmedi.

### 4. Sunucunun iki köşeli yaylı alanı (ADR 0069)

Sunucunun nesne doğrulaması (`cad.rs`) alan halkasını (dış halka, ada, parça ve parçanın adası) **en az 3 köşe** istiyordu; masaüstü ve web komutları iki köşeli halkayı, iki kenarından biri yaysa (yay değeri 0 değil) kabul eder:
daire, mercek, dairesel kesit. GeoPackage'ın `egri` tablosunun dairesi masaüstünde alınıyor, sunucuya komitlenirken “Alan en az 3 köşe ister” ile düşüyordu (ADR 0561'in sonunda görüldü).
- **Doğrulama** masaüstüyle aynı kuraldır (`check_ring_len`): iki köşe ve ilk iki yay değerinden biri ≠ 0 geçer; iki köşe iki düz kenar “… en az 3 köşe ister; kenarlarından biri yaysa 2 yeter, bu ikisi de düz” ile reddedilir.
  Hatalar `Invalid` olarak eskisi gibi (yol ve ileti).
- **PostGIS izdüşümü:** halka `bulge_path` ile yayın ara noktalarıyla çoğalır (iki köşeli halkada iki kenar da); yay dilimi `tolerance`'tan düzse (yay değeri 1e-12) halkada iki nokta kalır ve PostGIS çokgeni olmaz:
  bu durumda `geom` boştur (nesne `cad_definition` ile saklanır, “çizilecek sonlu bir şey yok” kuralı: `projection` → `None`). Başka hiçbir şey değişmedi; üç ve daha çok köşeli halka ve
  çok parçalı alanın izdüşümü aynı.
- Sınama: geçici PostGIS'te `changes` testi (yarım daire: geçerli çokgen, alan yarım dairenin alanı ±0,2 m², köşe sayısı; mercek; iki köşeli ada ve parça; dört ret yolu; düz yay boş `geom`),
  zincirde gerçek GeoPackage'ın dairesinin işle komitlenmesi ve `st_isvalid`.

## Doğrulama

- **Ortak durumlar:** `fixtures/commands/v1/file.import.{shp,geojson,kml}.json` (50, 33 ve 31 durum), KentOS kodu olmadan `scripts/fixtures/gis_import_cases.py` (pyproj'lu Python) ve
  `scripts/fixtures/gis_import_ogr.py` (GDAL/OGR, QGIS 4.2'nin Python'u ya da Homebrew'in GDAL 3.13'ü) ile: dosyaları GDAL yazar (QGIS GeoPackage'ının tabloları Shapefile, GeoJSON ve KML/KMZ olarak, TM36'da ve OGR'ın
  PROJ ile çevirdiği WGS 84'te; zip'ler, `.prj`'siz, boş, bozuk, okunamayan `crs`, kentos üyeli dosyalar), beklenen nesneleri Shapefile ve GeoJSON için `tools/formats/gis.py` (ADR 0046), KML için `scripts/fixtures/kml_cases.py`
  (ADR 0222) okur, çevirmeyi PROJ yapar (±1e-6 m; yol ve doğruluk ADR 0167'nin tablosundan), `--check` her dosyayı OGR'a bir daha okutup köşelerini başvuruların okuduklarıyla karşılaştırır
  ve OGR'ın WGS 84'e çevirdiği parselleri KentOS'un TM36'ya geri çevirişinin GeoPackage'ın köşelerine 2 cm içinde (GeoJSON 7 ondalık) düştüğünü denetler.
  Sınadıkları: tek adım ve geri alma adı, plan/validate, var olan katmana, aynı adlı katman, grup, `skipLayers`, `entry`, çevirme (WGS 84, UTM 36N, ED50 resmî olmayan yol ±2,1 m), `srid` ezmesi, okunamayan sistem, yerel proje,
  her ret ve uyarı kodu.
  Koşucu: masaüstü `fixtures.rs` (yeni beklenti: `fields`'te `within`), Python `test_command_cases.py` (tipli katman üzerinden). Yetenek testi her kodu bir durumda buldu; KML için ulaşılamayan iki kod (`crs_unknown`,
  `crs_undeclared`) ve GeoJSON'un `crs_undeclared`'ı listeden çıkarıldı (KML hep WGS 84, GeoJSON hep bir sistem bildirir).
- Başsız sunucu: `headless` `gis_files_are_taken_in_by_their_commands` (beş dosya, plan = execute, `crs_mismatch`, günlük satırları komutun adıyla, tekrar oynatma aynı).
- Python `test_gis_import.py` (arşiv, `entry`, çevirme cevabı, `srid`, yerel proje), MCP araçları katalogdan gelir (yıkıcı listeye üç komut girdi).
- Sunucu işi: `job_run` (gerçek çocuk: zip'in tek katmanı, GeoJSON, KML'in `crs_mismatch`'i ve `reproject`'i), `job_run_db` (zincir: beş dosya sırayla komitlenir, sürümler ve nesne sayısı; GeoPackage dairesi `st_isvalid`),
  `job_kinds_cases` (25 biçim adı durumu).
- Pencereler: `exchange::gis_tests` ve `kml_tests` aynı, geçerek (günlükte komutun adıyla satır testi eklendi); resimler `.run/shots/gis-*`, `kml-*` (koyu ve açık, 1440×900 ve 1100×650).
- Manifest: üç pencere `semantic` (`catalog/v1/operations.json`), `docs/cad/manifest.{json,md}`, kapsam `docs/gis/coverage.{json,md}`.

## Performans

100 000 noktalı, 14 MB'lık GeoJSON, `reproject` ile (WGS 84 → TM36), hata ayıklama derlemesinde Python'dan: 1,95 sn (okuma, çevirme ve tek adımda yazma). Pencere aynı yolu dosyayı bir kez okuyup çeviriyi
iş parçacığında yaparak kullanır; komuta yalnız yazma düşer.
