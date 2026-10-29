# ADR 0745: PostGIS'te geography sütunu ve geometrisi olmayan tablo

- **Durum:** kabul edildi (2026-10-29). Sahibin bildirimi: veritabanındaki tabloları katman olarak bağlıyor, ama çizimde
  hiçbir şey görünmüyordu (`kbs.ilceler`).
- **Web:** bir şey değişmez. **`.kcad`:** değişmez. **Sunucu:** değişmez.
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

Sahibin veritabanındaki 40 geometri sütunlu tablonun hepsinde geometri boştu. Örneğin `kbs.ilceler`'in 17 satırının ve
`kbs.iller`'in 81 satırının hiçbirinde `geom` yoktu. KentOS bu satırları doğru biçimde kayıt olarak aldı, ama bunu
söylemedi: katman “17 nesne” dedi ve çizim boş kaldı.

Aynı araştırmada ikinci bir kusur bulundu. PostGIS'in `geography` türündeki sütunu hiç geometri sayılmıyordu:
`geometry_columns`'a bakılıyordu, `geography_columns`'a değil. Bu yüzden `geography(Point,4326)` tablosu 20 kayıt
veriyor ama 0 nesne çiziyordu.

Üreticilerin yolu (29 Ekim 2026'da okundu):

- PostGIS 3.6 belgeleri, “Data Management”. Geography alanları `geography_columns` görünümünde kayıtlıdır. Sistemi
  verilmeyen geography sütunu WGS 84'tür (4326). Geometry ile geography arasında dönüşüm yapılabilir.
  <https://postgis.net/docs/manual-3.6/using_postgis_dbmanagement.html>
- QGIS'in PostgreSQL sağlayıcısı (`qgspostgresconn.cpp`). `geography_columns`'u da tarar ve sütunu `SctGeography`
  diye işaretler. Sistemi ve türü `::geometry` dönüşümüyle (`st_srid(sütun::geometry)`) okur.
  <https://github.com/qgis/QGIS/blob/master/src/providers/postgres/qgspostgresconn.cpp>

## Karar

### 1. Katalog geography sütunlarını da sayar

Kaynaklar'ın veritabanı ağacı (`probe::browse`) `geometry_columns` ile `geography_columns`'un birleşimini okur, bu yüzden
geography tablosu nesne tablosu olarak listelenir.

### 2. Geography okunur, yazılmaz

Bağlanan tablonun şekli (`table_shape`) önce `geometry` sütununu, yoksa `geography` sütununu alır. Sorgu katmanı
(`query_shape`) ise bildirilen türden okur. Sistemi verilmeyen geography 4326'dır.

Seçim ve kutu koşulu sütunu `sütun::geometry` olarak okur (`geom_of`); `&&` böylece geometrinin kutusuyla karşılaştırır.
Geography tablosu salt okunurdur: yazmak sütunun türüne geri dönüşüm ister. Bu dilimde yapılmadı; değişiklik “salt
okunur” diye reddedilir.

### 3. Hiçbir satırında geometri olmayan tablo söylenir

Geometri sütunu olan tablonun bütün satırları geometrisizse bağlama yeni `rows_without_geometry` uyarısını verir:

> “kbs.ilceler” tablosunun 17 satırının hiçbirinde “geom” sütununda geometri yok: çizimde görünecek bir şey yok,
> satırlar kayıt olarak Öznitelik tablosunda. Geometriyi veritabanında doldurun ya da başka bir geometri sütunu olan
> tabloyu bağlayın.

Uyarının yolu `edit.table`'dır. Bazı satırların geometrisiz olması olağandır (QGIS de bir şey demez); o zaman uyarı
verilmez. Kural GeoPackage ve PostGIS bağlamasında aynıdır (`sources::link_table`).

Masaüstünde bağlamanın bildirimi uyarıyı gövdesinde yazar: ne çizilmediği ve neden, günlüğe bırakılmaz.

## Doğrulama

- `cargo test -p kentos-postgis --lib`: `geometry_decl` geometry ve geography türlerini, sistemlerini okur.
- `cargo test -p kentos-native-application --test all sources::`: QGIS'in parsel tablosunun bütün geometrileri
  boşaltılınca bağlama uyarıyı verir. Tek satırı geometrisiz olan özgün dosyada uyarı yoktur.
- `KENTOS_PG_URL=… cargo test -p kentos-desktop --bin kentos-cad every_kind_of_table_is_drawn_once_bound -- --ignored --nocapture`:
  - Geçici bir docker PostGIS'inde (5432'ye dokunmaz) sekiz tablo Kaynaklar'dan bağlanır ve 1440 × 900'de resimlenir
    (`.run/shots/postgis-cizim-*`).
  - Sonuçlar:

    | Tablo | Sonuç |
    |---|---|
    | MultiPolygon | 20 çizilen |
    | tipsiz geometri | 20 çizilen |
    | WGS 84 çizgi | 20 çizilen |
    | PolygonZ | 20 çizilen |
    | tırnaklı ad | 20 çizilen |
    | geography | 20 çizilen (önceden 0) |
    | anahtarsız tablo | bağlanmaz |
    | boş geometri | 17 kayıt, uyarıyla |

  - Görünüm bağlanan nesnelere yakınlaşır.
  - Sahibin veritabanında `KENTOS_PG_TABLES=kbs.ilceler,kbs.iller` ile salt okuyarak denendi: 17 ve 81 kayıt, uyarıyla.

## Kalan

- Geography sütununa yazmak (`ST_GeomFromWKB(...)::geography`).
- Bir tablonun birden çok geometri sütunundan birini seçmek (bugün ilki alınır).
