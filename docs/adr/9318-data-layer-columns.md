# ADR 9318: Veri katmanının tablosu ve alanları birlikte; formdan eklenen alan tablonun sütunu olur

- **Durum:** kabul edildi (2026-11-26). Kaynak: CBS kullanım incelemesi, 26 Kasım 2026 (`gis-eksik-26-11-2026.md`, madde
  1, P1); sahibin sözü: “dosyadaki maddeleri yapalım, bitene kadar durma”.
- **İlgili:**
  - [ADR 9100](9100-data-drivers.md) (veri sürücüleri), [ADR 9101](9101-data-layers.md) (veri katmanları),
    [ADR 9103](9103-edit-session.md) (düzenleme oturumu);
  - [ADR 9107](9107-project-in-a-source.md) §7 (`cad.layers.create`: yeni katman yeni tablo);
  - [ADR 9132](9132-table-conditions-by-layer.md) (katmanda olmayan alanı adlandıran koşul);
  - [ADR 9141](9141-form-designer-new-field.md) (formda Yeni alan), [ADR 9161](9161-publish-check-kind-by-kind.md)
    (yayın denetimi türüne göre).

## Bağlam

- **Kusur:** CBS projesinde GeoPackage veri katmanına Formlar'da “Yeni alan” ile `saha_notu` eklendi.
  - Tasarımcı alanı yalnız katmanın alan listesine yazdı (`cad.layers.edit` Fields); tabloya sütun eklenmedi.
  - Ön denetim “sorun yok” dedi, form yayınlandı.
  - Veri tablosu “Kaynak okunamadı … saha_notu alanı yok” ile hiçbir satırı göstermedi. Okuyucu katmanın her alanını
    kaynaktan istiyordu; tabloda olmayan tek bir alan bütün okumayı düşürdü.
  - Bağımsız eklenen sütun katmana kendiliğinden gelmedi; yeniden okumanın bir yolu yoktu.
- **Eksik olan:** veri sürücülerinin tablo kurma ve silme işi vardı, var olan tabloya sütun eklemesi yoktu
  (`Transaction`).

## Araştırma

- **QGIS:** öznitelik tablosunun “New field”ı alanı veri kaynağına ekler
  ([Working with the Attribute Table](https://docs.qgis.org/3.40/en/docs/user_manual/working_with_vector/attribute_table.html)).
- **ArcGIS Pro:** veritabanı tablosuna alan yalnız tablonun sahibiyle bağlanınca eklenir; INSERT, UPDATE ve DELETE hakkı
  verilmiş başka hesap şemayı değiştiremez
  ([Alter the schema of a database table](https://pro.arcgis.com/en/pro-app/3.6/help/data/databases/alter-database-table-schema.htm),
  [Grant and revoke dataset privileges](https://pro.arcgis.com/en/pro-app/3.4/help/data/geodatabases/overview/grant-dataset-privileges.htm)).
- **PostgreSQL:** `ALTER TABLE` için tablonun sahibi olmak gerekir
  ([ALTER TABLE](https://www.postgresql.org/docs/current/sql-altertable.html)).
- **GeoPackage / SQLite:** GeoPackage'ın tabloları SQLite tablolarıdır; `ALTER TABLE … ADD COLUMN` yeni sütunu sona ekler,
  var olan satırlar onda NULL'dır ([SQLite ALTER TABLE](https://www.sqlite.org/lang_altertable.html)).
- **KentOS kararı:** veri katmanının alanı tablonun sütunudur. Sütun ekleme kaynağın ve hesabın hakkıdır: GeoPackage'ta
  yazılabilir dosya, PostgreSQL'de tablonun sahibi.

## Karar

### 1. Sürücü sözleşmesi

- `Transaction::add_columns(table, fields)`: boş bırakılabilir sütunlar, sırayla; satırlar onlarda NULL. Öntanımlısı
  “bu kaynağın tablosuna sütun eklenemez” der.
  - GeoPackage: `ALTER TABLE … ADD COLUMN`, türü tablo kurarkenki gibi (`column_of`).
  - PostGIS: `ALTER TABLE … ADD COLUMN`. Sahibi olmayan hesabı veritabanı reddeder (42501, `Denied`); işlem hiçbir şey
    yazmaz.
- `Capabilities::add_columns`: GeoPackage'ta yazılabilir dosyanın gerçek tablosu; PostGIS'te hesabın sahibi olduğu
  tablo (`pg_has_role(relowner)`). Sorgu ve görünüm hiçbir zaman.
- Ortak uyum takımı (`kentos_data::conformance`) iki sürücüde de sınar: geri alınan ekleme tabloyu değiştirmez; eklenen
  sütunlar sona gelir, boş bırakılabilir, her satırda boştur, değer alır.

### 2. `cad.layers.columns` v1

- `add` (Veri kaynağına sütun ekle): her alan tabloya boş bırakılabilir bir sütun olarak eklenir, kendi işleminde. Tablo
  yeniden okunup sütunlar görülünce katmanın alanları da yazılır. Alanın kuralları (değer listesi, yeni satırın
  varsayılanı) katmanda kalır.
- `refresh` (Kaynak şemasını yenile): katmanın alanları tablonun şimdiki sütunlarına getirilir.
  - Sütunu duran alan kalır, kuralları ve yeri korunur.
  - Tablonun öbür sütunları tablonun sırasıyla alan olarak eklenir.
  - Tabloda artık olmayan alan katmandan çıkar.
- **Geri alma:** katmanın alanları tek adımdır (“Kaynağa sütun ekle”, “Kaynak şemasını yenile”). Eklenen sütun tabloda
  kalır, `cad.layers.session`'ın yazdığı satır gibi. Değişen yoksa adım açılmaz.
- **Plan** tabloyu okur, hiçbir şey yazmaz; `rows` tablonun satır sayısıdır (eklenen sütunda boş kalacaklar).
- **Retler, sırasıyla:**
  - `no_layer`, `no_columns`;
  - `invalid_name` (boş, 63 bayttan uzun, denetim karakteri, kaynağın kendine ayırdığı ad);
  - `column_exists` (girdide iki kez; sonra katmanın alanı, tablonun anahtarı ya da geometrisi; sonra tablonun
    katmanda olmayan sütunu, Türkçe katlanarak);
  - `invalid_fields`; sürüm retleri; `layer_not_found`, `not_a_layer`, `not_a_data_layer`;
  - `data_layer_read_only` (add);
  - `source_unreachable` (tablo okunamadı ya da değişikliği reddetti);
  - `columns_not_permitted` (sahibi olunmayan PostgreSQL tablosu, salt okunur dosya, sorgu).
- İletiler katmanın adını söyler; sorgu katmanının tablo adı bir SQL metnidir.

### 3. Okuma bir alan yüzünden durmaz

- `cad.layers.rows` (`layers_rows::read`) tablonun şemasını okur. Katmanda olup tabloda olmayan alanı kaynaktan istemez:
  öbür sütunlar okunur, satır o alanda boştur.
- Cevabın `missing`'i o alanları adlandırır.
- Veri tablosunun alt satırı bunu uyarı rengiyle söyler: “Kaynak tablosunda olmayan alan: “kayip”; boş gösterilir. Alanlar ›
  Kaynak şemasını yenile ya da sütunu ekleyin.”
- Okuma, seçim, Python ve MCP aynı okuyucudan geçer.
- Harita yalnız stilin, etiketin ve süzgecin adını verdiği alanları okur; onlar değişmedi.

### 4. Masaüstü

- **Formlar › Yeni alan, veri katmanında:** önce komutun planıyla sorulur. “Veri kaynağına sütun ekle” kutusu tabloyu,
  sütunun adını ve türünü, var olan satırlardan kaçının boş kalacağını söyler. Tablodaki sütunun geri alınmadığını,
  katmanın alanının Ctrl+Z ile geri alındığını da yazar.
  - Sütunu ekle: komut yürür, alan forma konur.
  - Vazgeç: hiçbir şey yazılmaz.
  - Kaynak eklemeye izin vermiyorsa (yetki, salt okunur dosya, sorgu) nedeni söylenir; taslak ve katman olduğu gibi kalır.
- **Alanlar, veri katmanında:**
  - Yeni alan önce tabloya eklenir (`add`).
  - Yeniden adlandırma reddedilir: tablonun sütunu adını korur, katman tabloda olmayan bir sütunu okumaya kalkardı.
    Takma ad verilir ya da sütun kaynakta yeniden adlandırılıp yenilenir.
  - Katmandan kaldırılan alan artık okunmaz; sütun tabloda kalır.
  - “Kaynak şemasını yenile” düğmesi `refresh`'i çalıştırır, satırları yeniden okur ve neyin geldiğini, neyin çıktığını
    söyler.
- **Yayın ön denetimi:** forma konmuş ama veri katmanının tablosunda olmayan alan bir hatadır
  (`form_field_not_in_source`, “Kaynak tablosu” başlığı).
  - Yayınla kapanır.
  - Yayınla nereden istenirse istensin aynı denetimle durur.
  - İleti çıkış yolunu söyler.

### 5. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| `cad.layers.columns`, Yeni alan ve Alanlar'ın sütun yolu, okumanın `missing`'i, yayın denetimi | yok: veri katmanı CBS'nindir (ADR 9196) | yok: CAD projesi CBS tablolarını salt okunur altlık olarak gösterir (AY-06), onlara sütun eklemez | yok: aynı gerekçe | var |

### 6. Kanallar

| Kanal | Var mı | Nasıl |
|---|---|---|
| Python | var | `kentos.cad.layers.columns(doc, action="add", layer=…, fields=[…])`, `action="refresh"`; okumada `cad.layers.rows`'un `missing`'i |
| MCP | var | `gis.run` ile `cad.layers.columns`; var olanı değiştirdiği için önce plan |
| Yapay zekâ ajanı | var | MCP ile |
| Eklenti | var | `run`/`call` ile `cad.layers.columns`, `document.write` izni |

- Her kanal aynı komutu çağırır; denetim, plan, yetki ve adım aynıdır.

## Açık

- Değer listesinin kaynakta kısıt olarak yazılması (GeoPackage `gpkg_data_column_constraints`, PostgreSQL CHECK) ve
  tablo hücresinin listeli editörü incelemenin 4. maddesidir.
- Sütunu tablodan silmek ve yeniden adlandırmak sunulmaz. Başka bir programla yapılan değişiklik yenilemeyle görülür.

## Doğrulama

26 Kasım 2026'da koşuldu:

- **Sürücüler:**
  - `cargo test -p kentos-data -p kentos-data-gpkg -p kentos-data-postgis` geçti; GeoPackage uyum takımı yeni sütun
    denetimiyle geçti.
  - Geçici PostGIS'te (55432, `KENTOS_DATA_PG`) `cargo test -p kentos-data-postgis --test conformance`: 7 sınama geçti.
    Uyum takımı sütun denetimiyle geçti. Yeni sınama `a_column_is_added_by_the_tables_owner_alone`: sahibin yeteneği
    var. Yalnız SELECT ve UPDATE hakkı olan hesabın yeteneği yok; deneyince `Denied` alıyor, tablo değişmiyor.
- **Ortak durumlar:** `fixtures/commands/v1/cad.layers.columns.json`, 30 durum
  (`scripts/fixtures/layer_columns_command_cases.py`, `--check`). `KENTOS_CASES_ONLY=cad.layers.columns cargo test -p
  kentos-native-application --test all every_command_case` geçti. Bir beklenti bilerek bozulunca düştü, üreticiyle geri
  yazılınca geçti.
- **Çekirdek:** `cargo test -p kentos-native-application --test all -- layers_columns capabilities` geçti.
  - Salt okunur dosya `columns_not_permitted` alıyor, dosya değişmiyor.
  - Dosyası gitmiş tablo `source_unreachable` alıyor.
  - Eklenen sütun boş okunuyor; geri alınınca katmanın alanı gidiyor, sütun tabloda kalıyor.
  - Yetenek sınaması her kodu bir durumda ya da gerekçesiyle buluyor.
- **Komşu ortak durumlar:** `KENTOS_CASES_ONLY=cad.layers.rows,cad.layers.data,cad.layers.session,cad.layers.create,cad.layers.migrate,cad.layers.export`
  ile `every_command_case` geçti; okumanın şema denetimi onları değiştirmedi.
- **Masaüstü:** `cargo test -p kentos-desktop --lib -- source_columns_tests new_field_tests layer_fields row_target_tests
  history::text features::` geçti (49 sınama). Yeni sınamalar (`gis/forms/source_columns_tests.rs`):
  - `yeni_alan_on_a_data_layer_asks_then_adds_the_column_and_every_row_stays_read`: soru (tablo, sütun, 3 satır),
    Vazgeç'te hiçbir şey yazılmıyor; Sütunu ekle'de sütun dosyada, alan katmanda ve formda; üç satır okunuyor.
  - `alanlar_adds_a_new_field_to_the_table_refuses_a_rename_and_reads_the_table_again`: yeniden adlandırma reddi, yeni
    alanın sütunu, başka yerde eklenen sütunun yenilemeyle gelmesi ve iletisi.
  - `a_field_the_table_lacks_leaves_the_rest_read_is_named_and_keeps_the_form_back`: okuma sürüyor, `missing` ve
    tablonun alt satırı alanı adlandırıyor, yayın engelleniyor.
- **Python:** yerel modül yeniden derlendi. `KENTOS_CASES_ONLY=cad.layers.columns,cad.layers.rows,cad.layers.data,cad.layers.session`
  ile `test_command_cases` geçti; `test_layer_columns` (2 sınama) tipli SDK'yla ekleme, `missing` ve yenilemeyi sınıyor.
- **Üreticiler:** komut kataloğu, TypeScript türleri, Python SDK'sı (`--check`), CAD manifesti (`--check`) yeniden
  yazıldı; modül kataloğu değişmedi (komut kayıttaki bölümüyle CBS'nin).
- **Resimler** (`.run/gui-evidence/kullanim-9318/`, `source_columns_tests::screens`, büyütülerek bakıldı):
  `01-yeni-alan-sorusu` (Formlar'da soru kutusu), `02-kaynakta-olmayan-alan` (tablonun alt satırında uyarı, satırlar
  okunuyor), `03-alanlar-yenile` (Kaynak şemasını yenile düğmesi, “Tablodan alındı: “baska”.”); açık ve koyu tema.
- **Kırmızılar (bu dilimden değil):** masaüstünde bağımlılık tabanı (render birleşmesinin üç bağı); `kentos-plot`'ta
  `bands` sınaması (CBS'de bağlı katman, ADR 9196).
- **Koşulmayan:** gerçek pencerede fareyle deneme.

