# ADR 9136: İşlemin tablosu dosyaya sütun türleriyle yazılır

- **Durum:** kabul edildi (2026-11-22); adım VY-35. Plan: [plan/veri.md](../../plan/veri.md) VY-35.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Belediye kullanım değerlendirmesinin
  23. maddesi (P1):
  - histogram doğru hesaplandı: 1.024 ve 2.048 geçerli hücre, ayrıca 1.024 NoData;
  - sonuç GeoPackage'a yazılınca FID dışındaki altı sütunun hepsi TEXT oldu;
  - `WHERE "Sayı" > 500` sıfır satır verdi, `CAST("Sayı" AS INTEGER) > 500` iki satır verdi.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi.
- **Komut:** `cad.processing.run`'ın tablo çıktısına isteğe bağlı `kinds` ve `values` eklendi; `rows` aynı kaldı.

## §1 Bağlam

- İşlemlerin tablo çıktısı `{columns, rows}` biçimindedir ve hücreler pencerenin gösterdiği metindir (“91.088”, “15.38”).
- Sonucun yeri dosya olunca bu metinler metin sütunlarına yazılıyordu (ADR 0551 §3). Sayıyla süzme, sıralama ve toplama
  dosyada çalışmıyordu.
- Veritabanına yazım önce geçici bir GeoPackage'a yazar, sonra onu sağlayıcıya aktarır. Bu yüzden düzeltmenin tek yeri
  GeoPackage yazıcısıdır.
- **Araştırma.** GeoPackage'ın öznitelik sütunları SQLite türleriyle bildirilir: INTEGER, REAL, TEXT. GDAL'ın
  GeoPackage sürücüsü bunları Integer64, Real ve String okur, sayı karşılaştırması sütunun türüyle yapılır.
  Kaynak: [GDAL GeoPackage sürücüsü](https://gdal.org/en/stable/drivers/vector/gpkg.html) (değerlendirmenin bağlantısı).

## §2 Karar

1. **Türler ve düz değerler.** Bir aracın tablo çıktısı gösterilen `rows`'un yanında iki şey verebilir:
   - `kinds`: her sütun için `integer`, `real` ya da `text`;
   - `values`: hücrelerin biçimlendirilmemiş değerleri (sayı, metin ya da boş).

   Pencere ve Panoya kopyala `rows`'u gösterir; ondalık görünüş sunumda kalır.
2. **Yazıcı** (`geopackage::TextTable`). Türü söylenmiş ve her satırının düz değeri olan sütun INTEGER ya da REAL olur,
   değerler biçimlendirilmeden yazılır.
   - Bir değeri türüne uymayan sütun metin kalır; uymayan değeri boş yazmak onu kaybettirirdi.
   - Türsüz tablo (serbest metin raporu) eskisi gibi metindir.
3. **Histogram.**
   - Aralık sıra numarası ve Sayı INTEGER.
   - Alt ve üst sınır, oran ve birikimli oran REAL, yuvarlanmadan.
4. **Veritabanı.** PostGIS'e yazım aynı geçici dosyadan geçtiği için aynı türleri alır.

## §3 Kanıt

- **Uygulama** (`tests/all/processing_typed_tables.rs`): ortak raster çiziminin `bolge` rasterinin dört aralıklı
  histogramı GeoPackage'a yazılıyor. SQLite'ın kendisine soruluyor:
  - her hücrenin `typeof`'u: integer, real, real, integer, real, real;
  - `WHERE "Sayı" > en küçük` sayısal süzüyor;
  - `ORDER BY "Sayı" DESC` sayısal sıralıyor;
  - oranların toplamı 100.
- Pencerenin satırları metin kaldı. Serbest metin raporu (Özet istatistik) dosyada hâlâ metin; eski sınaması geçiyor.
- **Bağımsız okuyucu:** aynı dosyayı GDAL'ın `ogrinfo`'su okudu:
  - Aralık ve Sayı Integer64; sınırlar, oran ve birikimli Real;
  - `sqlite3` ile `WHERE "Sayı" > 20 ORDER BY "Sayı" DESC` 38, 33, 32 verdi.
- **Ortak durum:** histogram durumunun beklenen tablosuna `kinds` ve `values` eklendi.
  - `scripts/fixtures/raster_ops_processing_cases.py` güncellendi. Bu makinede `mpmath` ve GDAL'ın Python bağları
    olmadığından üretici çalıştırılamadı.
  - Değerler bağımsız başvurudan (`fixtures/raster-ops/v1/cases.json`) üreticinin kuralıyla yazıldı. Rust'ta geçiyor.
- **PostGIS:** `KENTOS_DATA_PG` verildiğinde aynı tabloyu veritabanına yazıp sütun türlerini katalogdan okuyan sınama
  eklendi. Bu makinede Docker yanıt vermediği için koşturulmadı.
  - 23 Kasım 2026'da (ADR 9181 adımında) geçici bir PostGIS'te (PostgreSQL 18.4, PostGIS 3.6) koşturuldu ve iki
    sınama kusuru çıktı. Sınama, sonucu PostGIS'e yazan tablo sağlayıcısını kaydetmiyordu (“Bu ortamda PostGIS
    sağlayıcısı yok”); `kentos-postgis` sınama bağımlılığı oldu ve sağlayıcı çalıştırmadan önce kaydediliyor. Ayrıca
    raporun şemalı tablo adına şemayı bir kez daha ekliyordu.
  - Düzeltilince geçiyor: Sayı tam sayı, Alt sınır gerçek sayı olarak katalogda.
- İşlemler (masaüstü 126), işlem çekirdeği ve sonuç yeri sınamaları (15) geçiyor.
- Görünür arayüz değişmedi; resim çekilmedi.

## §4 Kalan

- Öbür tablo veren araçlar (Özet istatistik, bölgesel istatistik ve diğer 30 kadarı) türlerini henüz söylemiyor;
  her biri `kinds` ve `values` ekledikçe dosyaya sayı yazar.
- Kaydet ▾ (XLSX/ODS/CSV) hâlâ gösterilen metni yazar.
- Web (TODOS.md §0.2).
