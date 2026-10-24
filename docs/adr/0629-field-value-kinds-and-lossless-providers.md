# ADR 0629 — Alan değerlerinin kendi türleri ve sağlayıcılarda kayıpsız gidiş-dönüş

Durum: kabul edildi (24 Ekim 2026). Kapsam: plan/gis.md `CBS-06`'nın kalanı (öznitelik modelinin kendi türleri, boş değerin
anlamı, dosya ↔ sağlayıcı ↔ tablo ↔ Python yolu, PostGIS'te aynı denetim). Kapsamı ben belirledim. Bağlam belgeleri: ADR 0199
(katman alanları), 0412 (tarih ve saat, saat), 0439 §8 (Değerleri biçime çevir), 0502 (GeoPackage), 0512 (bağlı katmanda geniş
değerler), 0564 (görünüş alanın adını izler), 0627 (GeoPackage'ta KentOS belgeleri), 0628 (GeoParquet). Web: ertelendi (ADR 0228).

## Bağlam

Öznitelik değerleri metindir (`attrs: BTreeMap<String, String>`); alan (ADR 0199) değerin hangi metni alacağını ve tek biçimini
söyler. Bu model ondalığı zaten kesin taşır (30 rakam). Eksikler:

1. **Tam sayı 2⁵³ − 1 ile sınırlı.** GeoPackage'ın, Parquet'in, PostGIS'in 64 bitlik tam sayısı bu sınırı aşınca sütun metin alanı
   olur ve söylenir (ADR 0502); tür kaybolur, sıralama ve aralık denetimi metinle yapılır.
2. **Dilimli zaman yok.** Tarih ve saat alanı yereldir (ADR 0412); `+03:00` dilimli an ve saniyenin kesri alınırken atılır ve
   söylenir. PostGIS'in `timestamptz`'si, Parquet'in UTC'ye bağlı zaman damgası aynı kayıpla gelir.
3. **UUID, JSON ve ikili değer yok.** PostGIS'in `uuid`, `jsonb`, `bytea`'sı, Parquet'in UUID ve JSON mantıksal türleri,
   GeoPackage'ın BLOB'u metne düşer ya da hiç alınmaz.
4. **Boş değerin anlamı yazılı değil.** NULL'un özniteliğin yokluğu olduğu ADR 0502'de bir cümledir; boş metin, boş liste, sıfır ve
   false'un ayrımı her sağlayıcıda ayrı ayrı sınanmadı.

Alan adının yeniden adlandırılması formlarda (ADR 0414), görünüşte (0564), ifadelerde ve etiketlerde zaten izlenir; birleşik
görünümler alanı kimliğiyle (`key`) tutar. Bu kısım yapılmış sayılır, burada yalnız sınanır.

## Karar

### 1. Değer metindir, türü alan söyler; boşun anlamı

Değerler metin kalır: `.kcad`'in öznitelik biçimi, Python'un `dict[str, str]`'si ve ifade dilinin girdisi değişmez. Anlamlar:

| Değer | KentOS'ta | Anlamı |
|---|---|---|
| NULL (SQL), Parquet'in null'u, GeoJSON'un `null`'u ya da eksik özelliği | öznitelik yok | değer yok |
| boş metin | `""` | boş metin (metin alanında); zorunlu olmayan alanda “boş” |
| boş liste | `""` (çok değerli alan) ya da `[]` (JSON alanı) | boş liste |
| sıfır | `0` (sayı alanı) | sıfır |
| yanlış | `false` (evet/hayır alanı) | yanlış |

NULL ile özniteliğin yokluğu tek anlamdır: GeoJSON'un `null` özelliği ile eksik özelliği ayrılmaz (yazarken eksik özellik yazılmaz,
NULL'lar sütunlu biçimlerde NULL'dur). Sayı alanında boş metin yazılmaz: boş değer özniteliğin yokluğudur.

### 2. Beş yeni alan türü

Her biri metindir ve tek biçimi (kanonik metni) vardır; `check_value` her yazmada onu yazar.

| Tür (`kind`) | Arayüzde | Alır | Tek biçimi |
|---|---|---|---|
| `bigint` | Büyük tam sayı | −9 223 372 036 854 775 808 … 9 223 372 036 854 775 807; işaret ve baştaki sıfırlar | `-123`, `9007199254740993` |
| `instant` | Zaman damgası | `YYYY-AA-GG` ya da `GG.AA.YYYY`, `T` ya da boşluk, `SS:DD[:SS[.k…]]` (kesir en çok 9 rakam) ve **dilim**: `Z`, `±SS:DD`, `±SSDD`, `±SS` (en çok ±18:00) | `2024-03-01T10:20:30.5+03:00`; sıfır dilim `Z`; kesrin sondaki sıfırları atılır |
| `uuid` | UUID | 32 onaltılık rakam: tireli (8-4-4-4-12) ya da tiresiz, büyük küçük harf, `{…}` ya da `urn:uuid:` önekiyle | küçük harfli, tireli |
| `json` | JSON | RFC 8259'un herhangi bir değeri; iç içe en çok 64 düzey, en çok 1 000 000 karakter | anlamsız boşlukları atılmış metin; sayılar ve dizgiler yazıldığı gibi (kayıpsız) |
| `binary` | İkili | RFC 4648 base64 (dolgulu ya da dolgusuz); çözülmüşü en çok 1 MiB | dolgulu base64 |

- `bigint` sayıdır: aralık (`min`, `max`, hariç uçlar) ve değer listesi alır, kesin karşılaştırılır (ondalığın kuralıyla).
- `instant` zamandır: aralık alır; karşılaştırması UTC'deki anıdır (aynı an farklı dilimle eşittir, aralığın ucuna göre).
- `uuid`, `json`, `binary` aralık ve değer listesi almaz; `length` yalnız metnindir, `scale` yalnız ondalığın.
- Çok değerli (`multiple`) yalnız metin alanıdır (değişmedi).

### 3. Nerede nasıl görünür, sıralanır

- Öznitelik tablosu: `bigint` kesin sayı sırasıyla (`number`), `instant` UTC'deki anıyla (yeni `instant` sırası; eşitte dilimin
  metniyle), ötekiler metin sırasıyla. İkili değer tabloda, Öznitelikler'de ve kartta “ikili, n bayt” görünür; düzenlenirken base64.
- İfade dili: beş tür **metin** olarak okunur, hiçbir zaman kayan noktalı sayıya çevrilmez (`[kimlik] = '9007199254740993'`).
  Tam sayı ve ondalık alanı eskisi gibi sayıdır; ondalığın 15 rakamı aşan kısmı ifadede yaklaşıktır, saklanan değer kesindir.
- İstatistik ve grafik: `bigint` metin gibi sayılır (değerler, tekiller); sayı istatistiği tam sayı ve ondalıkla sınırlıdır.
- Formlar: beş tür metin kutusuyla girilir, kuralı `check_value`'dur.
- Değerleri biçime çevir (`cad.fields.migrate`, ADR 0439 §8) yeni türlerle de çalışır: türü değiştirilen alanın değerleri planda
  (kuru çalıştırma) sayılır, uymayanlar örnekleriyle söylenir, yazma tek geri alma adımıdır.

### 4. Sağlayıcılar

Okuma (dosyanın sütunu → alan) ve yazma (alan → sütun):

| Biçim | `bigint` | `instant` | `uuid` | `json` | `binary` |
|---|---|---|---|---|---|
| GeoPackage | INTEGER; okumada 2⁵³'ü aşan değerli sütun | DATETIME, UTC'de `Z` ve kesriyle; okumada dilimli ya da kesirli değerli sütun | TEXT | TEXT | BLOB |
| GeoParquet | INT64; okumada 2⁵³'ü aşan değerli sütun | TIMESTAMP(MICROS, UTC); okumada UTC'ye bağlı zaman damgası | FIXED_LEN_BYTE_ARRAY(16) UUID | BYTE_ARRAY JSON | BYTE_ARRAY |
| PostGIS (okuma) | `bigint`, 2⁵³'ü aşan değerli | `timestamptz` | `uuid` | `json`, `jsonb` | `bytea` |
| FlatGeobuf | Long | DateTime (UTC'de) | String | String | Binary |
| CSV | `Integer64` | `DateTime` (UTC'de) | `String` | `String` | `String` (base64) |
| GML | `xs:long` | `xs:dateTime` (UTC'de) | `xs:string` | `xs:string` | `xs:base64Binary` |
| Shapefile | `N` (en çok 18 rakam; daha genişi metin, söylenir) | `C` (metin, söylenir) | `C` | `C` (metin, söylenir; 254 karakterde kesilir, söylenir) | `C` (base64 metin, söylenir) |
| GeoJSON | sayı (her basamağıyla); okumada 2⁵³'ü aşan tam sayılar | metin | metin | değerin kendisi | base64 metin |

- Zaman damgası dosyalara UTC'de yazılır (`instant_utc`): an aynıdır, dilimin metni dosyada kalmaz.
- KentOS'un yazdığı dosya türü geri okunurken kaybetmez: GeoPackage'ta tabloya bağlı KentOS belgesi (ADR 0627'nin yolu,
  `https://kentos.app/fields/1`, `{"kinds": {"sütun": "tür"}}`), GeoParquet'te dosyanın `kentos` anahtarı (`{"kinds": {…}}`) yeni
  türlerin sütunlarını söyler. Belgesi olmayan dosyada tür değerlerden ve sütunun türünden anlaşılır (tablodaki “okumada”;
  PostGIS'te sütunun bildirilen türünden): böyle bir dosyada kesirsiz UTC anı tarih ve saat, JSON ve UUID metin okunur, değer
  aynıdır.
- PostGIS'e katmanın eklediği yeni sütunlar şimdilik eski türlerle açılır (`timestamp`, `varchar(36)`, `text`, `bytea`); zaman
  damgası UTC'de yazıldığı ve katmanın alanı türünü bildiği için eşitleme anı aynı sayar (Sonraya kalan).
- Yazmada kayıp olursa yazmadan önce söylenir: zaman damgasının mikrosaniyeden ince kesri Parquet'te, Shapefile'da metin olarak
  yazılan türler.
- Bağlı katmanın eşitlemesi (ADR 0512) hücreyi sütunun okuma kuralıyla karşılaştırır: `instant` aynı anla, `uuid` küçük harfle,
  `json` tek biçimiyle, `binary` baytlarıyla eşittir; değişmeyen satır yeniden yazılmaz.

### 5. `.kcad`

Beş tür şema 104'ündür (`SCHEMA_WITH_FIELD_KINDS`): bunlardan birini taşıyan katman alanı olan çizim şema 104 yazılır, şema
103'te bu türler okunmaz (ADR 0265'in kuralı). Değerler metin olduğu için başka hiçbir şey değişmez.

### 6. Alanın adı

Yeniden adlandırmanın izlenmesi (formlar, ilişkiler, koşullu biçim, süzgeç, etiketler, zaman, bilgi kartı, eylemler, görünüş) ve
birleşik görünümlerin alan anahtarı değişmez; bu ADR'de yeni türdeki alanın yeniden adlandırılması da aynı yoldan sınanır.

## Sonuçlar

- 64 bitlik kimlikler, dilimli zaman damgaları, UUID'ler, JSON ve küçük ikili değerler dosya ↔ çizim ↔ dosya yolunda kayıpsız.
- İfade dili bu türleri metin olarak görür: büyük sayılarla aritmetik yapılamaz, karşılaştırma metinle yapılır (yazılı kural).

## Doğrulama

- Kurallar: `scripts/fixtures/layer_field_cases.py` (yeni türlerin denetimi, tek biçimi, aralığı, gösterimi ve verilerden tahmini;
  `fixtures/layer-fields/v1/cases.json`), `scripts/fixtures/field_kinds_cases.py` (32 `cad.entities.set` durumu,
  `fixtures/commands/v1/cad.entities.set.kinds.json`), `feature_table_cases.py` (zaman damgası UTC'deki anıyla, eşitte metniyle;
  büyük tam sayı kesin); hepsi KentOS kodu olmadan.
- `.kcad` şema 104: `kcad_v2_reference.py` (`field-kinds.kcad`, `broken/field-kinds-in-schema-103.kcad`, `broken/field-kind-unknown.kcad`,
  `schema-version-105.kcad`), `tools/kcad/kcad.py`, `kentos-kcad`'in testleri, `docs/specs/kcad-v2.md`.
- Sağlayıcılar: `field-kinds.kcad` GeoPackage, GeoParquet, FlatGeobuf, GML ve CSV olarak yazılıp yeni çizime okunur
  (`kentos-headless`'in `field_kinds` testi; GeoPackage ve GeoParquet bütün türleri, ötekiler değerleri tutar); yazılanlar GDAL'ın OGR'ı ve
  pyarrow'la (`field_kinds_cases.py --written`: türler, değerler, KentOS'un belgeleri, GeoPackage DATETIME'ı uyarısız milisaniyeyle);
  PostGIS'te `bigint`, `timestamptz`, `uuid`, `jsonb`, `bytea` ve `numeric(40, 10)` geçici docker veritabanında
  (`kentos-postgis`'in `wide_kinds_come_from_postgis`'i: türler, değerler, bağlamadan sonra değişiklik yok, yazılan an aynı);
  `genis.gpkg`'nin gidiş-dönüşü yeni kurallarla (`wide_values_cases.py`, `test_wide_values.py`).
- Önceki başvurular yeni okumalarla yeniden yazıldı: `gpkg_import_cases.py`, `geoparquet_cases.py`, `fgb_exchange_cases.py`,
  `csv_exchange_cases.py`, `gml_exchange_cases.py`, `bound_sources_cases.py`, `gis_import_cases.py` (GeoJSON'ın büyük tam sayısı),
  `shp_export_cases.py` (büyük tam sayı `N`); “2⁵³'ten büyük sayılar metin alanına” kayıp satırları kalktı.
- Masaüstü: Öznitelik tablosunun gösterimi ve sıralaması (`features::tests::the_table_shows_and_sorts_the_new_kinds`), türe göre sütun
  genişliği; resimler `features::tests::kinds_screens` (`.run/shots/alan-turleri-*`).
- Süreler (release, `kinds_timing`, 100 000 nokta, beş alan): değer denetimi bigint 4,6 ms, instant 45,6 ms, uuid 28,1 ms, json 13,1 ms,
  binary 11,9 ms; GeoPackage yazma 652 → 740 ms ve okuma 267 → 426 ms, GeoParquet yazma 111 → 192 ms ve içe aktarma 560 → 780 ms
  (aynı değerler metin alanlarında → yeni türlerle).

## Sonraya kalan

- Büyük tam sayı ve zaman damgasıyla ifadede aritmetik (kesin tam sayı ve an türleri ifade dilinde).
- PostGIS'te yeni sütunun `timestamptz`, `uuid`, `jsonb` olarak açılması; FlatGeobuf'un Json türü; GeoJSON'da JSON alanının
  okumada tanınması.
- 1 MiB'tan büyük ikili değerin projenin kitaplığına başvuru olarak alınması.
- Web'in alan kuralı (`model/layerFields.ts`) ve pencereleri: TODOS.md §0.2.
