# ADR 9108: Tam sayının genişliği ve kaynakta gruplanmış sayım

- **Durum:** kabul edildi (2026-11-21). Plan: [plan/veri.md](../../plan/veri.md) VY-11 (ikinci tur).
- **İstek:** sahibin sözü (21 Kasım 2026): “Hepsini düzelt eksikleri tamamla.” ADR 9106 §7 (32 bitlik tam sayının
  GeoPackage türü) ve ADR 9105 §6 (kaynakta gruplanmış sayım) kalanları.
- **Sürücü sözleşmesi:** `FieldKind::Int { bits }` (önce yalnız `Int`); yeni `Source::groups`. **Komut, `.kcad`,
  web:** değişmedi. **Bağımlılık:** yeni dış kitaplık yok.

## §1 Bağlam ve araştırma

**Sorun:** sürücü sözleşmesinde tek tam sayı türü vardı (`FieldKind::Int`, “en çok 64 bit”). KentOS'un tam sayısı
(Alanlar'daki “Tam sayı”) 32 bitliktir, ama GeoPackage'a `INTEGER` yazılıyordu: başka bir CBS onu 64 bitlik tam sayı
okuyordu. Ölçüldü (bu işten önce, Python SDK ile `cad.layers.export`, kurulu masaüstü CBS kitaplığıyla okundu):
`no` alanı `INTEGER` yazıldı, `Integer64` okundu. Ayrıca bir alanın değerlerine göre kaç satır olduğunu sormanın
sözleşmede yolu yoktu; sınıf sayımı tabloyu baştan sona tarıyordu (ADR 9105 §6).

**Biçim ve öbür programlar** (21 Kasım 2026'da okundu):

- OGC GeoPackage 1.3, Table 1 (veri türleri): `TINYINT` 8, `SMALLINT` 16, `MEDIUMINT` 32, `INT`/`INTEGER` 64 bitlik
  işaretli tam sayıdır.
- GDAL'ın GeoPackage sürücüsü (`ogrgeopackageutility.cpp`, `GPkgFieldFromOGR` / `GPkgFieldToOGR`): 32 bitlik alanı
  `MEDIUMINT`, 64 bitlik alanı `INTEGER`, 16 bitlik alanı `SMALLINT` yazar. Okurken `INT…` ile başlayan her tür
  64 bitliktir, `MEDIUMINT` ve `TINYINT` 32 bitliktir, `SMALLINT` 16 bitliktir.
- Başka bir dönüştürücü (FME'nin GeoPackage eşlemesi) `mediumint` türünü `int32`, `int` türünü `int64` okur.
- PostgreSQL: `smallint` (`int2`), `integer` (`int4`), `bigint` (`int8`).
- Masaüstü CBS'nin sınıflandırması ayrık değerleri sağlayıcıdan alır. Sınıf başına nesne sayısını ise nesneleri
  gezerek kendisi sayar (istemcide).

**Türkiye:** TUCBS Genel Kavramsal Model (ÇŞB CBS Genel Müdürlüğü) §9.4.4: “TUCBS uygulama şemalarında TSE ISO/TS
19103 6.5'te belirtilen veri türleri kullanılacaktır.” ISO 19103'ün `Integer` türü sınırsızdır. Sınır, yazan
programın seçtiği sütundadır. Sütunun tutamayacağı bir değer ya reddedilir ya da sessizce başka bir sayıya döner.
Bu yüzden değer yazılmadan önce sınırına bakılır (§2).

## §2 Karar: tam sayının genişliği

- `FieldKind::Int { bits }`: 8, 16, 32 ya da 64 bit, kaynağın bildirdiği gibi. `FieldKind::INT` (64) ve
  `FieldKind::INT32` kısaltmalardır.
- **Okuma:** GeoPackage'ta bildirilen türden çıkar (`TINYINT` 8, `SMALLINT` 16, `MEDIUMINT` 32, öbürü 64). PostGIS'te
  `int2` 16, `int4` 32, `int8` ve `oid` 64 bittir.
- **Yazma** (yeni tablo: dışa aktarma, yeni katman, taşıma): GeoPackage'a genişliği tutan en dar tür yazılır
  (`TINYINT`, `SMALLINT`, `MEDIUMINT`, `INTEGER`). PostGIS'te `smallint`, `integer` ya da `bigint` olur. KentOS'un
  “Tam sayı” alanı 32 bit, “Büyük tam sayı” alanı 64 bit yazılır.
- **Değer:** sütunun aralığını aşan değer yazılmadan reddedilir (`coerce`): ““ada” alanı 32 bitlik tam sayı tutar
  (−2147483648…2147483647); “2147483648” sığmaz.”
- **Masaüstü:** Kaynaklar'dan eklenen veri katmanında 32 bite kadar olan sütun “Tam sayı”, 64 bitlik sütun “Büyük tam
  sayı” olur. Önce yalnız `bigint`/`int8` büyük sayılıyordu. Bir GeoPackage `INTEGER` sütunu artık “Büyük tam sayı”dır,
  başka bir CBS'nin de okuduğu gibi.

## §3 Karar: kaynakta gruplanmış sayım

- `Source::groups(table, field, condition, limit)`: koşulun tuttuğu satırları alanın değerine göre sayar
  (`GROUP BY`). Liste alanın sırasındadır: sayılar büyüklüğüne, metinler Türk alfabesinin sırasına göre dizilir
  (ADR 9102 §3), boş değer (`Value::Null`) en sondadır. En çok `limit` grup döner.
- GeoPackage sürücüsü dosyada, `kentos_order` ile sorar. PostGIS sürücüsü sunucuda, `tr-x-icu` ile sorar; sunucuda
  ICU yoksa çeviriyle sıralar.
- Sözleşme, kendi sorgusu olmayan bir sürücü için varsayılanı da verir: alanın değerleri okunur ve sayılır. Yeni bir
  veritabanı sürücüsü bunu yazmadan da çalışır.
- Ayrık değerler (`unique`) de artık Türk alfabesinin sırasıyla döner. Sınıflandır'ın listesi bu sırayla gelecek
  (VY-13).

## §4 Kanıt

- `cargo test -p kentos-data`: 18 test geçti. Değer denetimi: 32 bitlik sütun 2147483647'yi alır, 2147483648'i ve
  "-2147483649"u reddeder; 64 bitlik sütun 3 000 000 000'u alır.
- Uygunluk takımı iki sürücüde de geçti:
  - GeoPackage: `cargo test -p kentos-data-gpkg`;
  - PostGIS: yalıtılmış `kentos-portal-db`, `KENTOS_DATA_PG=… cargo test -p kentos-data-postgis`.

  Takımın denedikleri:
  - `ada` ve `parsel` 32 bit yazılıp 32 bit okunur;
  - `kullanim` dört grup ve boş grup olarak, Türk alfabesinin sırasıyla ve her satır bir kez sayılır;
  - `limit 2` ilk iki grubu verir;
  - koşullu `ada` grupları koşulun satırlarını sayar.
- Bütün `cad.layers*` ortak durumları geçti.
- Masaüstünün veri katmanı, tablo, düzenleme, dışa aktarma, oluşturma ve taşıma testleri geçti (179).
- **Başka bir CBS'nin okuması:** Python SDK ile `cad.layers.export` yazdı (`.run/gui-evidence/veri/vy11/`). SQLite'ta
  sütunlar `no MEDIUMINT`, `sayac INTEGER`, `ad TEXT` göründü. Kurulu masaüstü CBS kitaplığı bunları `no Integer`
  (32 bit), `sayac Integer64` diye okudu; `sayac`'ın 3 000 000 000 değeri korundu. Bu işten önce `no` da `INTEGER`
  yazılıyor ve `Integer64` okunuyordu.
- **Gerçek pencere:** `tam.gpkg` Kaynaklar'dan “Veri katmanı olarak eklendi”, Veri ▸ Alanlar… açıldı. Görülen:
  `no` Tam sayı, `sayac` Büyük tam sayı, `ad` Metin. Aynı senaryo eski ikiliyle `sayac`'ı da “Tam sayı” gösterdi.

## §5 Kalan

- Kaynakta gruplanmış sayımın sınıflara bağlanması VY-13'tedir. İfadenin koşul ağacına çevrilmesi VY-12'dedir.
- Yazılmış bir tablonun sütun genişliği değiştirilmez: var olan `INTEGER` sütunlar, dosya yeniden yazılana kadar
  “Büyük tam sayı” kalır.

## Uygulama

- Sözleşme: `crates/native/data/src/schema.rs` (`FieldKind::Int { bits }`, `INT`, `INT32`, `int_range`),
  `coerce.rs` (aralık denetimi), `driver.rs` (`Source::groups`, `group_order`), `conformance.rs`.
- GeoPackage: `crates/native/data-gpkg/src/convert.rs` (`int_bits`, `int_decl`), `source.rs` (`groups`, Türkçe
  sıralı `unique`).
- PostGIS: `crates/native/data-postgis/src/catalog.rs`, `write.rs`, `sql.rs`, `read.rs` (`groups`, Türkçe sıralı
  `unique`).
- Uygulama: `crates/native/application/src/gis/layer_export.rs` (Tam sayı 32, Büyük tam sayı 64). Masaüstü:
  `apps/desktop/src/gis/data_layers.rs`.
