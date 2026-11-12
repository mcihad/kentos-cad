# 4460 — Ortofoto dikiş ölçümünün iki görüntünün dikiş çokgenlerinin ortak sınırı boyunca yapılması

Tarih: 2026-11-10. plan/map-cad.md MAP-12, sekizinci tur. Durum: taslak. ADR 2860'ın “Sonra”sındaki “Mozaiğin dikiş
çokgenlerinden hatların kendiliğinden okunması” (MAP-12'nin açığı: “mozaiğin dikiş çokgenleri”).

İlgili:
- [ADR 0200](0200-spatial-and-attribute-queries.md) §1: sınırda ya da değmede 1 mm;
- [ADR 2460](2460-orthophoto-seam-measured-in-the-images.md): dikiş ölçümü görüntülerin kendisinden;
- [ADR 2860](2860-orthophoto-seam-along-a-drawn-line.md): çizilmiş dikiş hattı boyunca ölçüm;
- [ADR 3260](3260-orthophoto-seam-bicubic-and-arcs.md), [ADR 3660](3660-orthophoto-seam-resampled-onto-the-first-grid.md),
  [ADR 4060](4060-orthophoto-seam-on-a-turned-first-grid.md): piksel altı, yeniden örnekleme, dönük birinci.

## Dayanak

- **ArcGIS Pro, mozaik veri kümesinin dikiş hatları**
  ([Mosaic dataset seamlines](https://doc.esri.com/en/arcgis-pro/latest/help/data/imagery/mosaic-dataset-seamlines-pro-.html),
  10 Kasım 2026'da okundu):
  - dikiş hatları ayak izlerine benzer: her görüntüyü bir çokgen temsil eder;
  - çokgenin biçimi görüntünün mozaikte kullanılan bölümüdür;
  - dikiş hatlı mozaikte görüntüler arasında örtüşme kalmaz;
  - dikiş hatları Seamline katmanındadır (SOrder, harmanlama genişliği ve türü alanlarıyla); elle düzenlenir ya da
    Import Mosaic Dataset Geometry ile kullanıcının çokgenleriyle değiştirilir.
- **ArcGIS Pro**, [Export Mosaic Dataset Geometry](https://doc.esri.com/en/arcgis-pro/latest/tool-reference/data-management/export-mosaic-dataset-geometry.html)
  (aynı gün okundu): mozaik veri kümesinin ayak izlerini, sınırını, **dikiş hatlarını** ya da çözünürlük düzeylerini
  gösteren bir nesne sınıfı yazar.
- **ArcGIS Pro**, [Build Seamlines](https://doc.esri.com/en/arcgis-pro/latest/tool-reference/data-management/build-seamlines.html)
  (aynı gün okundu): her ayak izine bir dikiş hattı; geometri, radyometri, kenar algılama, ayak izinden kopya, Voronoi
  ya da paralaks farkıyla hesaplanır; sıralama kuzeybatı, bakış noktasına yakınlık ya da bir alana göre.
- **ADR 2860'ın okuması:** iki görüntünün kayması en çok mozaiğin göründüğü yerde, dikiş hattında önemlidir.
- **KentOS bugün** (ADR 2860, 3260): dikiş ölçümü örtüşmenin ızgarasında ya da elle çizilmiş bir hat boyunca
  yapılıyordu. Mozaiğin kendi dikiş çokgenleri okunmuyordu: hattın elle yeniden çizilmesi gerekiyordu.

## Karar

1. **`seam.polygons`** (`cad.orthophoto.compare`, ek alan; komut sürümü değişmez): iki görüntünün mozaikteki dikiş
   çokgenlerinin kalıcı kimlikleri (sırası serbest). Her biri bir alan (dış halkaları, delikleri, parçaları) ya da
   kapalı bir çoklu çizgidir; kenarları düzdür. `line` ile birlikte verilmez.
2. **Ortak sınır:** birinci çokgenin halkalarının ikincininkilere 1 mm içinde (ADR 0200 §1'in sınırda sayma payı)
   oturan parçaları:
   - birincinin bir kenarı ile ikincinin bir kenarı, birinin iki ucu da öbürünün doğrusuna 1 mm'den yakınsa, birincinin
     kenarının ikincininkinin uçlarının izdüşümleri arasındaki parçasını paylaşır (birincinin kenarına kırpılır; tek
     noktada buluşan izdüşüm paylaşmaz);
   - bir kenarın parçaları birleştirilir (aralarında 1 mm'den az boşluk varsa bir);
   - parçalar birincinin halka sırasıyla izlenir: kenarının sonuna varan parça, sonraki kenarın başından başlayan
     parçasıyla köşeden geçerek sürer; halkanın sonuna varan son parça ile başından başlayan ilk parça halkanın başından
     geçerek birdir;
   - parçanın köşeleri birincinin kendi noktalarıdır; 1 mm'den kısa parça atılır.

   Çokgenler öbür sırayla verilince sınır ikincinin halka sırasıyla, yani çoğunlukla öbür yönde okunur.
3. **Yerler:** her parçanın başından `step` metrede bir (ADR 2860'ın kuralı); yerin `station`'ı parçası boyunca
   uzaklığı, `part`'ı parçanın sırası (0 ilki). Yalnız örtüşmedeki yerler ölçülür; dışındakiler sayılır.
4. **Cevap:** `seam.shared`: çokgenler, parçalar (köşeleriyle), toplam uzunluk, yer sayısı, örtüşme dışındaki yerler.
   Not: “Dikiş ölçümü: dikiş çokgenlerinin ortak sınırı boyunca (15.31 m, 1 parça) örtüşmede 9 yer, 7 yer örtüşmenin
   dışında; …”.
5. **Retler** (`invalid_input`; örtüşme ve piksel retleri önce gelir):
   - `line` ile `polygons` birlikte; aynı çokgen iki kez (`seam.polygons`);
   - çizimde olmayan, alan ya da kapalı çoklu çizgi olmayan, yaylı kenarlı çokgen (`seam.polygons[k]`);
   - ortak sınır yok: “İki dikiş çokgeninin ortak sınırı yok (1 mm içinde): mozaikte birbirine değmiyorlar; …”;
   - ortak sınırın hiçbir yeri örtüşmede değil (`seam.polygons`).
6. **Masaüstü** (Ortofoto karşılaştırması › Dikiş hattı):
   - **Seçili çokgenleri al:** çizimde seçili iki alan ya da kapalı çoklu çizgi, seçim sırasıyla. Başka seçim söylenir,
     hiçbir şey alınmaz. **Hattı kaldır** örtüşmenin bütününe döner; satırın yardımı üç yolu söyler;
   - özet ortak sınırın uzunluğunu ve parça sayısını söyler; dikiş tablosunun **Hat (m)** sütununda birden çok parça
     varsa uzaklık parçanın numarasıyla (“2 · 0.000”).
7. **Kapsam işareti:** ADR 1780'deki gibi **CAD + CBS** (raster bağlam sekmesi).

## Sınırlar

- KentOS dikiş çokgeni kurmaz (ArcGIS'in Build Seamlines'ı gibi Voronoi, radyometri ya da kenar algılamayla); mozaiğin
  kurduğu çokgenleri okur (ör. Export Mosaic Dataset Geometry ile yazılıp KentOS'a alınmış).
- Yaylı kenarlı dikiş çokgeni reddedilir.
- İkiden çok görüntülü mozaikte her çift ayrı ölçülür.
- Harmanlama genişliği kullanılmaz: ölçüm dikiş hattının kendisindedir.

## Kanıt

- **Çekirdek** (`orthophoto_mosaic::tests`): yan yana iki kare, halkanın başından geçen parça, köşede değme ve 1 mm
  payı, kısmi kenar.
- **Bağımsız başvuru** (`scripts/fixtures/orthophoto_compare_cases.py`; KentOS kodu yok), 11 yeni durum (67):
  - birincinin çokgeni kıvrımlı dikiş hattının batısında, ikincininki doğusunda: ortak sınır hattın kendisi, yerleri
    ADR 2860'ın hat durumunun yerleri;
  - öbür sırayla: hat güneyden kuzeye;
  - ikincisi hattın iki kenarının noktaları arasında doğuya çekilmiş (kapalı çoklu çizgi): iki parça, her biri kendi
    başından;
  - 0,4 mm kaymış ikinci: ortak sınır 1 mm içinde;
  - yedi ret (2 mm ayrık çokgenler, örtüşmede buluşmayanlar dahil).

  Eski 56 durum değişmedi. Rust ve Python geçer. Bozma denemesi: çokgenler öbür sırayla okununca dört durum yakalar.
- **Python** (`python/tests/test_orthophoto_seam_polygons.py`), testin kendi okumalarıyla:
  - ortak sınır iki halkanın ortak kenarları; yerler çizime eklenen aynı hattın yerleriyle aynı (her biri parçasıyla);
    öbür sırayla ikincinin sırası;
  - iki parçalı durumda parçalar birincinin sınırı santimetrede bir örneklenerek, ikincininkine 1 mm'den yakın örnekler
    tutularak okunur: uçlar ve uzunluklar 1 cm içinde;
  - 2 mm ayrık çokgenler `seam.polygons`'ta reddedilir.

  ADR 3660 ve 4060'ın Python testleri geçer.
- **MCP** (`crates/native/mcp/tests/orthophoto_seam_polygons.rs`): dört durumun ortak sınırı ve yerleri, 2 mm'nin reddi;
  dikiş ölçümünün MCP testi değişmeden geçer.
- **Masaüstü** (`ortho_seam_polygons_tests`): tek çokgen seçilince alınmaz; iki çokgenle cevap durumunki; iki parçalı
  durumda tablonun uzaklıkları parça numarasıyla; Hattı kaldır ızgaraya döner; hiçbir şey yazılmaz. Resimler
  `.run/gui-evidence/dikis-cokgenleri-4460/`.

## Sonra

- ~~KentOS'un kendi dikiş çokgenlerini kurması (Voronoi, radyometri; Build Seamlines'ın yöntemleri).~~ ADR 4700.
- Yaylı kenarlı dikiş çokgenleri; mozaikteki bütün görüntü çiftlerinin birlikte ölçülmesi.
- Görüntüden türeyen nesnenin kaynağa izi (MAP-01'in kararı).
- Web (TODOS §0.2).
