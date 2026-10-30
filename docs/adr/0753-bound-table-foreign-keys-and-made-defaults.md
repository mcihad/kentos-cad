# ADR 0753: Bağlı tablonun yabancı anahtarları ve veritabanının verdiği değerler

- **Durum:** kabul edildi (2026-10-30).
- **İstek:** sahibin sözü “formlarımız çok zayıf”; ADR 0750'nin “Kalan”ının iki maddesi. Sahip `kbs.ilceler`'i PostGIS'ten
  bağladı; `il_id` `kbs.iller`'e, `ust_id` `kbs.ilceler`'e başvurur, `genel_id` ve tarihler veritabanınca yapılır.
- **Web:** bir şey değişmez; TODOS.md §0.2'ye yazıldı. **`.kcad`:** şema 133, isteğe bağlı: alanın `column`'u.
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

ADR 0750'den sonra bağlı tablonun alanları NOT NULL'u ve sabit varsayılanı alıyordu. İki şey eksik kaldı:

- **Yabancı anahtar.** Sahibin tablosunda (salt okunur incelendi, `pg_constraint`) her anahtar başvurulan tablonun
  birincil anahtarına (`id`, bigint) gider: `ilceler.il_id → iller(id)`, `ilceler.ust_id → ilceler(id)`; `iller`'in 81
  satırı ve `ad` sütunu var. Formda `il_id` serbest bir sayı kutusuydu: kullanıcı ilin numarasını bilmeliydi, yanlış
  numara ancak veritabanında (yazarken) reddedilirdi.
- **Veritabanının verdiği değer.** `genel_id` (`uuidv7()`) ve tarihler (`now()`) yeni satırda tablonundur (ADR 0750 §2),
  ama yeni kayıtta boş bir kutu “doldurun” diye bakıyordu; neyin doldurulacağı söylenmiyordu.

Başvurulan tablonun anahtarı (`id`) bağlı katmanda alan değildir (ADR 0505: anahtar satırın kimliğidir). Bu yüzden
başvuru KentOS'un projedeki ilişkisi (ADR 0410'un `relations`'ı, alt kayıt üst kaydın kalıcı kimliğini tutar) olarak
kurulamaz: alanın değeri ilin kalıcı kimliği değil `id`'sidir. Karar, ADR 0750'nin NOT NULL → zorunlu yansımasının
aynısıdır: anahtarın başvurduğu satırlar alanın kendi değer listesi olur, tablo her okunduğunda yenilenir.

Başvurulan yollar (30 Ekim 2026'da okundu):

- QGIS'in kullanıcı kılavuzu, *Connecting and Editing Data Across Layers* §12.4.2: “QGIS is able to discover existing
  relations from supported database formats” (PostgreSQL, GeoPackage …); proje özelliklerinde *Discover Relations*.
  <https://docs.qgis.org/latest/en/docs/user_manual/working_with_vector/joins_relations.html>
- QGIS, *Attributes Form Properties › Edit widgets*: *Relation Reference* “is the default widget assigned to the
  referencing field (i.e., the foreign key in the child layer) when a relation is set”; *Value Relation*: “The key column
  is the stored value in the attribute table, while the value column is the label shown in the combobox.” KentOS'un
  listesi bunun gibidir: kod saklanan değer, etiket gösterilen.
  <https://docs.qgis.org/latest/en/docs/user_manual/working_with_vector/vector_properties.html>
- PostgreSQL `pg_constraint`: `contype` `f` yabancı anahtar; `conkey` kısıtlanan, `confkey` başvurulan sütunlar,
  `confrelid` başvurulan tablo. `pg_type.typcategory` `S`: metin türleri.
  <https://www.postgresql.org/docs/current/catalog-pg-constraint.html>,
  <https://www.postgresql.org/docs/current/catalog-pg-type.html>
- SQLite: `PRAGMA foreign_key_list`; “Attaching a REFERENCES <parent-table> clause to a column definition creates a
  foreign key constraint that maps the column to the primary key of <parent-table>”; başvurulan sütunlar birincil anahtar
  ya da UNIQUE olmalı. Sütun yakınlığının kuralları (*Determination Of Column Affinity*).
  <https://www.sqlite.org/foreignkeys.html>, <https://www.sqlite.org/pragma.html#pragma_foreign_key_list>,
  <https://www.sqlite.org/datatype3.html>

## Karar

### 1. Alanın `column`'u (`.kcad` şema 133)

Katmanın alanına isteğe bağlı `column` eklendi (`kentos_contracts::FieldColumn`):

- `made`: tablonun yeni satıra kendisinin verdiği değerin SQL metni (`now()`, `uuidv7()`, `CURRENT_TIMESTAMP`);
- `refers`: yabancı anahtarın başvurduğu tablo (`table`, kaynağın verdiği adıyla: `kbs.iller`), sütunu (`column`) ve
  satırları adlandıran sütun (`label`; yoksa kodlar).

İkisi de boşsa `column` yazılmaz. Metinler boş olamaz, boşlukla başlayıp bitmez, denetim karakteri içermez; `made` en
çok 1000, adlar en çok 256 karakterdir (`field_problem`). Bilgi bağlamanın ve okumanın yazdığıdır; kullanıcı yazmaz,
Alanlar penceresi onu olduğu gibi taşır. Bağlı olmayan katmanda da kalabilir (kopya, paket); anlamı “tablo böyle dedi”dir.

### 2. Bağlarken ve okurken

Bağlama (`cad.sources.edit` link, yeni katmana; GeoPackage ve PostGIS) ve okuma (`cad.sources.sync` read) tablonun
sütunlarından şunu çıkarır (`sources::references`):

- **Veritabanının verdiği değer.** Varsayılanı ne sabit (ADR 0750) ne NULL olan sütunun varsayılanı alanın `made`'idir:
  denetim karakteri boşluk, kırpılmış; 1000 karakteri aşan 999'a kesilip “…” alır.
- **Yabancı anahtar.** Tek sütunlu her yabancı anahtar (`PRAGMA foreign_key_list`; PostgreSQL'de `pg_constraint`'in
  `contype = 'f'` ve tek `conkey`'li satırları) alanın `refers`'idir. Sütun adı vermeyen anahtar başvurulan tablonun tek
  sütunlu birincil anahtarınadır. Birden çok sütunlu anahtar, tablosu olmayan anahtar alınmaz.
- **Etiket sütunu.** Başvurulan tablonun metin sütunları (SQLite'ta metin yakınlığı: INT içermeyen ve CHAR, CLOB ya da
  TEXT içeren tür; PostgreSQL'de `typcategory = 'S'`), başvurulan sütun dışında, sırasıyla: adı, küçük harfle ve ı ş ğ ü ö
  ç i s g u o c okunarak, `ad`, `adi`, `isim`, `name`, `baslik`, `unvan`, `title`, `label` ya da `tanim` olan ilki; yoksa
  ilk metin sütunu; o da yoksa etiketler kodlardır. Sahibin `iller`'inde `ad`, `Adı` adlı sütun da (`adi`).
- **Değer listesi.** Metin ya da sayı alanı (çok değerli olmayan) başvurulan satırları listesi olarak alır, en çok
  10 000 satır (daha çoğunda liste kurulmaz, alan serbest kalır, söylenir):
  - satırlar (anahtarı NULL olmayanlar) anahtarlarının, sonra etiketlerinin doğal sırasıyla okunur;
  - her anahtar alanın türünde yazılır (`check_value`); türe uymayan listeye girmez ve söylenir, aynı koda inen ikinci
    anahtar bir kez;
  - etiket kırpılır, boşsa ya da yoksa kodun kendisidir;
  - Türkçe katlanınca başka bir etiketle aynı olan etiket yanında kodunu söyler (“Merkez (1)”, “Merkez (3)”), o da
    alınmışsa bir kez daha; tekrar etmeyen etiket olduğu gibi kalır;
  - liste etiketlerin, sonra kodların doğal sırasıyla (ADR 0153 §6).
- **Okuma** (`follow`): `made` tablonun dediğidir (konur ya da kalkar). Liste, alanda bağlı liste varsa ya da alanın
  kendi listesi (ve projenin listesi, ADR 0430) yoksa tablonunkiyle yenilenir; anahtar kalkınca bağı ve listesi
  kalkar; alanın kendi listesi kalır. Başvurulan tablolar okunamazsa alanın bağı ve listesi olduğu gibi kalır.
  Yansıtma alanı geçersiz yapacaksa (türü değişmiş alan) alan olduğu gibi kalır.

Liste eksikse ya da kurulamadıysa uyarı `reference_values_missing` (bağlarken `edit.table`, okurken `layer`).

### 3. Değer listesiyle yazmak

Alanın listesi olduğundan Formla düzenle, Öznitelikler ve form ilin adını gösterir, adla seçtirir; listede olmayan değer
nesneye yazılmaz (`invalid_attribute`, ADR 0199). Yazılan kod tabloya gider, anahtar veritabanında da tutar.

### 4. Formda “Veritabanı verir”

Yeni kayıtta `made`'i olan alanın boş kutusu “Veritabanı verir: uuidv7()” der; tarih ve saat kutuları da biçim ipucu
yerine bunu söyler (`or_shape`). Değer yazılınca tablodan aynı adımda nesneye döner (ADR 0750 §3).

### 5. Alanlar penceresi

Seçilen alanın altında sütunun söyledikleri yazılır (`layer_fields::column_said`): “Değer listesi “kbs.iller” tablosunun
satırları (kodu “id”, etiketi “ad” sütunundan); tablo her okunduğunda yenilenir.” Pencerenin gövdesi alçak pencerede
kayar, düğmeler görünür kalır (`Dialog::scroll`; ADR 0749'un kuralı Alanlar'a da).

### 6. Sonra: değer listesinin denetimi doğrusal

`field_problem` kodların ve etiketlerin tekrarını her değeri öncekilerin hepsiyle karşılaştırarak arıyordu (10 000
değerde yaklaşık 50 milyon katlanmış metin). Şimdi kümelerle tek geçiştir; söylediği aynıdır.

## Doğrulama

- `python3 scripts/fixtures/source_references_cases.py --check`: `fixtures/commands/v1/cad.sources.references.json`'un
  24 durumu, KentOS kodu olmadan. Tablonun sütunları, anahtarları ve başvurulan satırlar sqlite3'le okunur; değerin
  biçimi `layer_field_cases.py`'nin, doğal sıra `feature_table_cases.py`'nin kuralıyla. Örnek dosyaları `baglar.gpkg`,
  `baglar-2.gpkg` (bir il fazla), `baglar-3.gpkg` (`il_id`'nin anahtarı yok) `--files` GDAL ve sqlite3 ile yazar.
  - Bağlamanın on üç kuralı: liste, Türkçe adlı etiket sütunu, boş etiket, kendine başvuran sütun ve aynı adlar, ilk
    metin sütunu, metin sütunsuz tablo ve aynı koda inen anahtarlar, türe uymayan anahtar (uyarı), 10 001 satırlı tablo
    (uyarı), adsız anahtar, iki sütunlu anahtar, hesaplanan varsayılanlar, sabit ve NULL varsayılan, alanla bağlanan parça.
  - Planın yazmaması; okumanın bir şey değiştirmemesi, tablo değişince yenilenmesi, anahtar kalkınca kalkması, alanın
    kendi listesini koruması, elle değiştirilen bağlı listeyi geri alması, geri alınıp yinelenmesi, planı.
  - Listede olmayan değerin reddi; listeden değerin tabloya yazılması; yeni nesnenin veritabanının değerlerini alması.
  - Rust'ta (`KENTOS_CASES_ONLY=cad.sources.references cargo test -p kentos-native-application --test all
    every_command_case`) ve Python'da (`test_command_cases`) aynı sonuç.
- `python3 scripts/fixtures/source_defaults_cases.py --check`: ADR 0750'nin 21 durumu; hesaplanan iki varsayılan artık
  alanın `column.made`'inde.
- `python3 scripts/fixtures/kcad_v2_reference.py --check`: şema 133'ün örneği `field-columns.kcad` ve dört bozuk
  örnek, bağımsız yazıcı ve okuyucuyla; Rust'ta `cargo test -p kentos-kcad` (taşıma matrisi `column`'u da taşır).
- `/usr/bin/python3 scripts/fixtures/postgis_cases.py --only a_tables_foreign_keys_list_their_rows`: geçici docker
  PostGIS'inde (5432'ye dokunulmaz) sahibinkine benzeyen `kbs.iller` ve `kbs.ilceler`: il listesi adlarıyla, kendine
  başvuran ilçeler kodlarıyla ayrılmış, `uuidv7()` ve `now()` alanda; veritabanına eklenen il sonraki okumada listede;
  listede olmayan değer reddedilir, listedeki tabloya yazılır. ADR 0750'nin testi de yeniden geçti.
- Birim testleri: `foreign::tests` (GeoPackage), `sources::references::tests` (liste, etiketler, `made`, okuma).
- Süreler (release, bu makine, 20 denemenin en iyisi): anahtarlar ve 10 000 satırın GeoPackage'tan okunması
  1,74 ms; 10 000 satırın listesi 5,11 ms; listenin alan denetimi 1,33 ms.
- Dokunulan sandıkların testleri ve masaüstünün bütün testleri (1390) geçer. Mimari ajanın ADR 0734'ünden kalan
  `walls_partition` testi bu değişiklikle ilgisizdir. `clippy -D warnings`.
- `cargo test -p kentos-desktop --bin kentos-cad district_form_screens -- --ignored --test-threads=1`: sahibin ilçe
  formu, `.run/shots/form-ilce-*`: yeni kayıtta “Veritabanı verir”, `il_id`'nin açık listesi (`3b-il-listesi`), Alanlar
  penceresinde bağlantının sözü (`6-alanlar-baglanti`, `6b-alanlar-veritabani`), 1100×650'de düğmeler görünür.

## Kalan

- Kendine başvuran listenin (`ust_id`) KentOS'tan eklenen yeni satırla yazmadan sonra yenilenmesi: şimdilik sonraki
  okumada.
- Kimlik (identity) ve hesaplanan (generated) sütunlar: alan olmamaları ya da salt okunur olmaları. ADR 0754'te
  kapandı: alan olurlar, değerleri tablonundur, yazılmazlar.
- Birden çok sütunlu yabancı anahtar; 10 000'den çok satırlı tablo için aranan liste (ADR 0439'un başka katmandan
  listesi gibi).
- Başvurulan tablo projede katmansa ilişki olarak önerilmesi (QGIS'in *Discover Relations*'ı gibi).
