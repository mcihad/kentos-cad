# ADR 9183: PostGIS kataloğu seçilen tablonun geometri sütununu, kapsamını ve ilişkili tablolarını söyler

- **Durum:** kabul edildi (2026-11-23); adım VY-82. Plan: [plan/veri.md](../../plan/veri.md) VY-82.
- **İstek:** sahibin sözü (23 Kasım 2026): “Docker çalışıyor”, “Devam”. Değerlendirmenin 7. maddesinin kalanı
  (ADR 9140, 9172): “ilişkili öznitelik tabloları, geometri sütunu adı ve kapsam, ayrı veritabanında
  rol/kesinti/çakışma sınamaları.” Maddenin kabulü: şema ağacı, geometrili tablolar, ilişkili öznitelik tabloları ve sık
  kullanılanlar kolay erişilsin; katman kimliğinde bağlantı, şema, tablo, geometri sütunu, birincil anahtar, SRID,
  okuma/yazma yetkisi ve kapsam sunulsun.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam ve araştırma

- PostGIS'e bağlan penceresi (ADR 9140, 9172) şema ağacını, sık kullanılanları ve seçilen tablonun kimliğini
  gösteriyordu: şema.tablo, anahtar, geometri türü, satır tahmini, SRID ve yazma hakkı. Geometri sütununun adı ve
  tablonun kapsamı yoktu.
- Geometrisiz tablolar varsayılan olarak listelenmez (bağlantının “Geometrisiz tabloları da listele” seçeneği). Bir
  parsel tablosunun malik tablosu gibi ilişkili öznitelik tablolarına ulaşmak için bu seçeneği açıp listeyi baştan
  aramak gerekiyordu.
- **Araştırma.** QGIS 4.2'nin PostGIS kaynak seçimi
  ([qgspgsourceselect.cpp](https://github.com/qgis/QGIS/blob/release-4_2/src/providers/postgres/qgspgsourceselect.cpp),
  okundu 23 Kasım 2026) her satırda şemayı, tabloyu, geometri sütununu, türünü, SRID'yi ve birincil anahtarı ayrı
  sütunlarda gösterir; geometrisiz tabloları bir seçenekle listeler. PostgreSQL'in kataloğu yabancı anahtarları
  `pg_constraint`'te tutar (`contype = 'f'`, `conrelid` → `confrelid`).

## §2 Karar

1. **Katalog** (`kentos_postgis::probe`, `Relation`):
   - `geometry_column`: ilk geometri ya da coğrafya sütununun adı (`geometry_columns`, `geography_columns`).
   - `related`: tabloya bir yabancı anahtarla, iki yönde de bağlı tablolar (`schema.tablo`). Tek sorguyla bütün
     katalog için okunur.
2. **Seçilen tablonun kimliği** şunu der: “kadastro.parsel · geometri sütunu geom · anahtar fid · çok parçalı alan ·
   ≈1.240 satır · EPSG:5256 · yazılabilir”. Birden çok geometri sütunu varsa hangisinin çizildiği söylenir (“3
   sütundan ilki”). Bağlantının adı pencerenin başındadır.
3. **Kapsam.** Tablo seçilince kapsamı veritabanına arayüzü bekletmeden sorulur (`Source::extent`): önce kataloğun
   tahmini (`ST_EstimatedExtent`), o yoksa ölçülen. Tablonun kendi sisteminde, ölçmeciliğin sırasıyla yazılır: “kapsam
   Y 486500.00–486585.00, X 4420200.00–4420225.00”. Coğrafi sistemde altı basamak. Sorulurken “kapsam okunuyor…”,
   okunamazsa nedeni.
4. **İlişkili tablolar.** Seçilen tablonun altında “İlişkili tablolar:” ve her biri için bir düğme durur. Basınca o
   tablo seçilir; geometrisiz tablolar listelenmiyor olsa da. Seçilmiş olanın düğmesi kapalıdır.

## §3 Kanıt

- **Masaüstü** (`shell/exchange/postgis_tests.rs`):
  - kimlik geometri sütununu söylüyor; kapsamın sözleri (metre, derece, okunuyor, okunamadı, geometrisiz tabloda yok);
  - **gerçek PostgreSQL** (`on_postgresql_the_catalogue_says_a_tables_column_extent_and_related_tables`, ayrı bir
    şemada):
    - parsel tablosunun geometri sütunu `geom`; parsel ile malik birbirinin ilişkili tablosu;
    - anahtarsız görünüm bağlanamıyor ve nedenini söylüyor;
    - pencerede parsel seçilince kapsamı sorulup geliyor; malik, geometrisiz tablolar listelenmezken düğmeyle
      seçiliyor;
    - yalnız okuyabilen hesap parseli salt okunur görüyor;
    - aktarıcı bağlantıyı kesince listeleme düşüp nedenini söylüyor, açılınca yeniden listeliyor.
- Maddenin öbür sınamaları ADR 9182'dedir: salt okunur rol (27, 28, 4), ağ kesintisi (30, 31), ikinci istemci
  çakışması (26).

## §4 Kalan

- Gerçek pencerede deneme; web (TODOS.md §0.2).
