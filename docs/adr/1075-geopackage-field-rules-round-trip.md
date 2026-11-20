# 1075 — Alan kuralları GeoPackage gidiş-dönüşünde: KentOS belgesi, şema uzantısı ve kırılan kuralların bildirimi

Tarih: 2026-11-20. gis-usage-report.md GIS-009. Durum: kabul edildi (doğrulanmış teslim kipi açık).

İlgili: [ADR 0629](0629-field-value-kinds-and-lossless-providers.md) §4 (KentOS'un alan türleri belgesi), [ADR 0755](0755-bound-table-check-and-unique-constraints.md)
(bağlı tablonun kısıtları), [ADR 0758](0758-layer-keys-and-indexes.md) §5 (katman dizinleri).

## Sorun

Park sınırları katmanında tekil park kodu ve 0–20 aralıklı tam sayı bank sayısı vardı. Envanter GeoPackage'a yazılıp
ayrı projeye alınınca türler kaldı ama görünen adlar (“Park kodu”, “Bank sayısı”), tekillik ve aralık kayboldu: iki
ANK-01 ve 21 bank kabul edildi. Zorunlu `rota_kodu` boşken dışa aktarım başarılı sayıldı, sütun boş değere izin
verdi. Kurallar kullanıcıya söylenmeden kayboluyordu.

## Dayanak

- OGC GeoPackage şema uzantısı ([belgesi](https://www.geopackage.org/spec/#extension_schema)): `gpkg_data_columns`
  sütunun adı/başlığı/açıklaması ve kısıt adı; `gpkg_data_column_constraints` `range` (min, max, dahil mi) ve `enum`
  (değer, açıklama) kısıtları. İsteğe bağlıdır.
- GDAL 3.13 (QGIS 4.2 içindeki OGR Python ile 20 Kasım 2026'da denendi): alanın alternatif adını
  `gpkg_data_columns.name`'e, aralık ve kodlu alanları `gpkg_data_column_constraints`'e, zorunlu ve tekili tablo
  tanımına (`NOT NULL UNIQUE`) yazar ve aynı yerlerden okur.

## Karar

1. **KentOS'un belgesi** (`https://kentos.app/field-rules/1`, tablonun meta verisi): kuralı olan her alan, katmanın
   tuttuğu gibi (görünen ad, aralık ve uçları, zorunlu, tekil, liste, varsayılan, birim, proje listesi). KentOS geri
   okurken alanı bütün alır (sütunun adı korunur); türler belgesi (ADR 0629 §4) olduğu gibi kalır.
2. **Şema uzantısı** (başka programlar için; `kentos_geopackage::schema`): görünen ad `name`'e; sayının aralığı
   `range`, listesi `enum` (kod ve etiket) kısıtı olarak, `<tablo>_<sütun>` adıyla. Okurken KentOS belgesi yoksa
   görünen ad, aralık ve liste buradan alınır.
3. **Tablo tanımı:** zorunlu alanı her nesne doldurmuşsa sütun `NOT NULL` yazılır; tekil alan için
   `kentos_tekil_<tablo>_<sütun>` adlı tekil dizin yazılır, değerler yineleniyorsa düz dizin yazılır ve söylenir. Bu
   dizin geri okunurken katman dizini sayılmaz (alanın kuralıdır). Dosyanın kendi UNIQUE kısıtı eskisi gibi katman
   dizini (ADR 0758) ve bağlı tablonun kısıtı (ADR 0755) olur, alanın Tekil'i olmaz.
4. **Kırılan kurallar:** yazmadan önce her alan için zorunlu boşlar, kurala uymayan değerler (aralık dışı, listede
   olmayan, türüne uymayan) ve yinelenen tekil değerler sayılır; dışa aktarma `export_losses` uyarısıyla alan alan,
   ilk üç nesneyle söyler. Saha taslağı yine yazılır.

## Sınama

- `crates/native/application/tests/all/gpkg_field_rules.rs`: gidiş-dönüşte görünen adlar, tekil kod, 0–20 aralığı
  (0 ve 20 geçer; 21 ve “yirmi” ret) ve zorunlu rota kodu; şema uzantısı ve `NOT NULL`, tekil dizin; GDAL'ın yazdığı
  `fixtures/geopackage/v1/gdal-sema.gpkg` (görünen ad, 0–20 aralığı, NOT NULL, UNIQUE, kodlu liste) KentOS'ta alan
  kuralı ve katman dizini olur; iki ANK-01, 21 bank ve boş rota kodu alan alan söylenir, dosya bunları iddia etmez.
- GDAL KentOS'un yazdığı dosyada görünen adları, 0–20 aralık alanını, tekilliği ve `NOT NULL`'u okudu.

## Açık

- Doğrulanmış teslim kipi: kural kırılınca yazmayı reddeden seçenek (bugün saha taslağı olarak uyarıyla yazılır).
