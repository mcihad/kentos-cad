# ADR 9190: Türünü söylemeyen aracın tablosu gösterdiği hücrelerden türlenir

- **Durum:** kabul edildi (2026-11-23); adım VY-89. Plan: [plan/veri.md](../../plan/veri.md) VY-89.
- **İstek:** sahibin sözü (23 Kasım 2026): “Yarım kalan maddeleri tamamla”. Değerlendirmenin 23. maddesinin kalanı
  (ADR 9136, 9158): “öbür tablo veren araçların türleri (bölgesel istatistik, matrisler…)”. Kabul: “Alan tipleri,
  sayısal sıralama ve eşiğe göre sorgu bağımsız okuyucuda doğrulanmalı. PostGIS çıktı sağlayıcısına aynı tip
  sözleşmesi için test ekleyin.”
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** `cad.processing.run`'ın tablo
  çıktısı değişmedi; dosyaya yazılış biçimi değişti.

## §1 Bağlam

- Histogram (ADR 9136) ve Özet istatistik (ADR 9158) tablolarının sütun türlerini (`kinds`) ve düz değerlerini
  (`values`) veriyordu. Dosyaya ve veritabanına sayı yazılıyordu.
- Tablo veren otuz kadar araç ikisini de vermiyordu. Bölgesel istatistik, uzaklık ve başlangıç–varış matrisleri, komşuluk,
  karışıklık matrisi gibi araçların tabloları dosyada metin sütunlarıydı. `WHERE "Ortalama" > 100` sıfır satır
  veriyordu.
- Araçlar hücrelerini aynı biçimleyicilerle yazar:
  - tam sayıyı ayraçsız (`n.to_string()`, `fixed(x, 0)`);
  - ondalığı noktalı ve sabit basamakla (`fixed(x, n)`, `length_text`, `area_text`);
  - gruplu sayıyı (“1.024”, `count_words`) yalnız özet cümlelerinde.

## §2 Karar

1. **Hücrelerden okuma** (`kentos_processing::table_kinds`). Türünü ve değerini söylemeyen aracın tablosunun her
   sütunu yazılmadan önce gösterdiği hücrelerden okunur:
   - her hücresi tam sayı (rakam, isteğe bağlı eksi) olan sütun INTEGER;
   - her hücresi noktadan sonra aynı sayıda basamaklı sayı olan sütun (`fixed(x, n)`'nin yazdığı) REAL;
   - başka her sütun metin. Boş hücre değersizdir (NULL), karar vermez.
2. **Yanlış okumaya kapı yok.** Gruplu sayı (“1.024” ile “512” bir arada), birimli değer (“12 m”), tire, ondalık
   virgülü, artı işareti ve aradaki boşluk sayı sayılmaz; sütun metin kalır. Tam sayıyla ondalık bir sütunda karışıksa
   da metindir: tek bir biçimleme değildir.
   - Başında sıfır olan değer (“007”, “00.5”) bir koddur, sayının biçimlemesi onu yazmaz: sütun metin kalır, kod
     sıfırlarını korur. “0” ve “0.5” sayıdır.
   - 64 bite sığmayan tam sayı da metindir.
   - Sonucu tablolarda gruplu sayı yazan araç yok: araçların gruplu yazdığı sayılar (`count_words`, raster serilerinin
     `grouped`'ı) yalnız özet cümlelerindedir. Gösterilen ondalığı kırpılan sütun (ROC'un Eşik'i: “0.5”, “0.25”)
     basamak sayısı değiştiği için metin kalır.
3. **Kendi türünü söyleyen araç** (Histogram, Özet istatistik) eskisi gibi kendi `kinds`/`values`'ını kullanır,
   yuvarlanmamış değerlerle. Hücrelerden okunan değer araçta gösterilen basamaktadır: Bölgesel istatistik'te kişinin
   seçtiği ondalık sayısı.
4. **Aynı okuma her yerde.** Dosyaya (GeoPackage) ve veritabanına (PostGIS, aynı geçici dosyadan) yazım ile toplu
   çalıştırmanın sonucu bu okumayı kullanır. Pencerenin Kaydet ▾'i (XLSX, ODS, CSV) de sütunu aynı biçimde türler: sayı
   sütunu sayı, öbürü metin. Bir kod (“007”) metin sütunundadır, sayıya dönmez.

## §3 Kanıt

- **İşlem çekirdeği** (`table_kinds` birim sınamaları): tam sayı, sabit ondalık, gruplu sayı, birim, tire, karışık
  sütun, ondalık virgülü, artı, boşluk, başında sıfır olan kod ve 64 biti aşan sayı ayrı ayrı.
- **Uygulama** (`tests/all/processing_typed_tables.rs`):
  - `zonal_statistics_and_a_distance_matrix_go_to_a_geopackage_typed_from_their_cells`: SQLite'ın kendi `typeof`'u
    Bölgesel istatistik'te Nesne text, Sayı integer, Toplam/Ortalama/En küçük/En büyük/Standart sapma real.
    `"Ortalama" > 100` üç satır; hücresiz bölge NULL; `ORDER BY "Sayı" DESC` 35, 30, 23, 0. Uzaklık matrisinde Kaynak
    ve Hedef text, Sıra integer, Uzaklık real; `"Sıra" = 1` yedi satır.
  - `zonal_statistics_go_to_postgis_typed_from_their_cells`: geçici PostGIS'te (PostgreSQL 18, PostGIS 3.6) katalog
    Sayı'yı tam sayı, Ortalama'yı gerçek sayı, Nesne'yi metin veriyor. Histogram'ın PostGIS sınaması da aynı koşuda
    geçti.
- İşlem çekirdeği ve masaüstü işlem sınamaları geçiyor (masaüstü 137).
- Görünür arayüz değişmedi; resim çekilmedi.

## §4 Kalan

- Araçların kendi düz değerleri (yuvarlanmamış): her araç `kinds`/`values` verdikçe, ortak durumlarıyla birlikte.
- Web (TODOS.md §0.2).
