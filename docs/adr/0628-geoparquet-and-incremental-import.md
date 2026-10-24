# ADR 0628 — GeoParquet ve içe aktarmada artırımlı yükleme

Durum: kabul edildi (24 Ekim 2026). Kapsam: plan/gis.md `CBS-05`'in kalanı (GeoParquet, hazırlık ve artırımlı yükleme,
reddedilen satırlar); TODOS.md `FMT-03`, `FMT-06`, `PG-13`, `PG-14`'ün dosya içe aktarmasına düşen payı.

## Bağlam

CBS-05'in önceki dilimleri GeoPackage, Shapefile, CSV, FlatGeobuf, GML ve XYZ'yi iki yönde (ya da yazmada) taşıdı
(ADR 0502, 0514, 0522, 0544, 0545, 0563). Kalan üç iş:

1. **GeoParquet.** Bulut verisinin ve büyük dağıtımların biçimi (Overture, Source Cooperative, DuckDB, GDAL, geopandas).
   Kitaplık (Apache Arrow'un `parquet` crate'i, onlarca bağımlılıkla) sahibin onayını bekliyordu. Bu makinedeki GDAL'da Parquet
   sürücüsü yok; bağımsız başvuru pyarrow 26 ve geopandas 1.2'dir (`uv`).
2. **Artırımlı yükleme (upsert).** Aynı dosyanın yeni sürümü bugün çizime ikinci kez alınamaz (`entity_exists`) ya da var olan
   katmana kopya olarak eklenir. Kurumlar veriyi dönem dönem yeniden yükler: anahtar alanla eşleşen nesne güncellenmeli,
   yenisi eklenmeli, yinelenen anahtar açıkça ele alınmalı.
3. **Hazırlık ve reddedilen satırlar.** İçe aktarma kaç satır okuduğunu, yazdığını, atladığını ve nedenini söylemeli; alınmayan
   satırlar kurumun düzeltip yeniden yükleyebileceği bir dosyada durmalı.

## Karar

### 1. Parquet kodeği kendi kodumuzla

Yeni bağımlılık eklenmez (FlatGeobuf, NetCDF, TIFF ve PDF'teki gibi). `kentos_formats::parquet`:

- **Kap:** Thrift compact protokolü (okuma ve yazma), `FileMetaData`, şema öğeleri, satır grupları, sütun parçaları, sayfa
  başlıkları; dosyanın sonundaki `PAR1` ve altbilgi boyu denetlenir, bozuk dosya panik değil açık hatadır.
- **Okuma:**
  - veri sayfası v1 ve v2, sözlük sayfası;
  - kodlamalar PLAIN, PLAIN_DICTIONARY ve RLE_DICTIONARY, RLE (evet/hayır ve düzeyler), DELTA_BINARY_PACKED,
    DELTA_LENGTH_BYTE_ARRAY, DELTA_BYTE_ARRAY, BYTE_STREAM_SPLIT;
  - sıkıştırma yok, Snappy, GZIP, ZSTD, LZ4_RAW ve Hadoop çerçeveli LZ4; Brotli ve LZO açık retle;
  - tekrar ve tanım düzeyleriyle iç içe sütunlar (Dremel): yaprak sütunlar düzeyleriyle verilir, kayıt kurmak okuyanın işidir.
- **Çözücüler** `kentos_formats::compress`: Snappy (çözme ve sıkıştırma), ZSTD (çözme; RFC 8878: çerçeveler, ham, RLE ve
  sıkıştırılmış bloklar, Huffman'lı ve ham değişmezler, FSE'li diziler, yinelenen uzaklıklar, xxHash64 sağlaması), LZ4 blok ve
  GZIP (başlık ve `miniz_oxide`). ZSTD ileride TIFF'in 50000 sıkıştırması için de kullanılabilir.
- **Yazma:** veri sayfası v1, PLAIN, düzeyler RLE; sıkıştırma Snappy (varsayılan) ya da yok; satır grubu 65 536 satır; sütun
  istatistikleri (en küçük, en büyük, boş sayısı); `created_by` KentOS.

### 2. Değer türleri

| Parquet | KentOS alanı |
|---|---|
| BOOLEAN | evet/hayır |
| INT32/INT64 (INT, işaretsiz dahil), INT96 değil | tam sayı (2⁵³'ü aşan her basamağıyla metin, söylenerek: ADR 0512) |
| FLOAT, DOUBLE | ondalık sayı |
| DECIMAL (INT32, INT64, FLBA, BYTE_ARRAY) | ondalık metni kesin (sayı alanına sığmıyorsa metin) |
| STRING, ENUM, JSON, UUID | metin |
| DATE | tarih |
| TIME | saat |
| TIMESTAMP (milli, mikro, nano), INT96 | tarih ve saat; UTC'ye bağlı olan dilimiyle |
| BYTE_ARRAY (türsüz), FLBA | alınmaz, söylenir |
| tek düzeyli LIST | metin (JSON dizisi) |
| yapı, iç içe liste, MAP | alınmaz, söylenir |

### 3. GeoParquet 1.1

- **Okuma:** dosyanın `geo` anahtarı (sürüm 1.0 ve 1.1), birincil geometri sütunu:
  - `WKB` (ISO ve EWKB'siz 2B/3B; Z kot olur, M ölçü);
  - yerli GeoArrow kodlamaları (`point` … `multipolygon`): yaprak sütunlar düzeyleriyle geometriye kurulur;
  - Parquet 2.11'in `GEOMETRY` mantıksal türü (`geo` anahtarı olmayan dosyada).

  Diğer geometri sütunları WKT metni olur ve söylenir. `covering` kutu sütunu türetilmiş veridir, alınmaz.
- **Sistem:** sütunun `crs`'i PROJJSON'dur:
  - `id`'si (EPSG ya da `OGC:CRS84`, iç içe `base_crs` değil) kodu verir;
  - `crs` anahtarı yoksa OGC:CRS84 (4326, boylam-enlem);
  - `null` sistem bilinmiyor demektir;
  - kimliksiz PROJJSON da bilinmiyor sayılır ve adıyla söylenir.

  Bilinmeyen sistemi girdinin `srid`'i bildirir (`crs_overridden`). Projeninkinden başka sistem ADR 0563'ün `reproject`
  onayıyla çevrilir.
- **Yazma:** WKB (ISO, Z kodları 1001–1007), yaylar 0,1 mm'lik kirişlerle (`kentos.chords/1`'in kuralı) ve söylenir, M atılır ve
  söylenir; `geometry_types` gerçekten bulunan türler; `bbox`; `covering` kutu sütunu; `orientation: counterclockwise` (dış halka
  saat yönü tersine yazılır); `crs` kayıttaki sistemin PROJJSON'u (`kentos_project::projjson`, WKT 2 yazıcısının tanımıyla,
  pyproj onu kodun sistemi okur); yerel projede `crs: null`. Nesnenin kalıcı kimliği `kentos_uid` sütunudur; okumada
  `kentos_uid` sütunu varsa kimlik odur, yoksa GeoPackage satırlarının kuralıyla `UUIDv5(gpkg:parquet/ad.parquet:satır)`
  (satır 1'den sayılır; aynı dosya yeniden okununca aynı kimlikler).

### 4. Komutlar

- `file.import.parquet` v1: GeoParquet'in satırları GeoPackage alma yolundan (`file_import_gpkg::run_read`) çizime; katman
  yönlendirmesi, sistem sorusu, kayıplar ve tek geri alma adımı “GeoParquet: ad.parquet” FlatGeobuf'unkiyle aynı.
- `file.export.parquet` v1: katmanların nesneleri tek dosyada (birden çok katmanda `katman` sütunu), `compression`
  (`snappy`, `none`), `covering` (kutu sütunu, varsayılan açık). Bütün ya da hiç, atomik yazma.

### 5. Artırımlı yükleme (`merge`)

Dosyadan içe aktaran komutların (GeoPackage, FlatGeobuf, GML, CSV, GeoParquet, Shapefile, GeoJSON, KML/KMZ) ortak isteğe
bağlı girdisi:

```json
"merge": { "key": "parsel_no", "targetKey": "parsel_no", "matched": "update", "geometry": true,
           "unmatched": "add", "duplicates": "stop" }
```

- Eşleme, tablonun gittiği **var olan katmanın** nesneleriyle yapılır (adıyla ya da `layers`/`layer` yönlendirmesiyle). Yeni
  açılan katmanda bütün satırlar eklenir.
- `key` yoksa eşleme **kalıcı kimlikle**dir: aynı dosyanın yeniden alınması (`entity_exists` yerine) güncellemedir. Satırları
  kalıcı kimlik taşımayan biçimlerde (Shapefile, GeoJSON, KML/KMZ) anahtarsız birleştirme `merge_key_required` ile reddedilir;
  GeoPackage, FlatGeobuf, CSV, GML ve GeoParquet'in satırlarının kimliği vardır.
- Anahtarın karşılaştırması Anahtarla birleştir'in kuralıdır (`ops::statistics::join_key`: kırpılır, sayılar sayı olarak).
- `matched`:
  - `update`: öznitelikler kaynağınkiler olur, kaynakta boş hücre hedefi boşaltır;
  - `fillEmpty`: yalnız hedefte boş olanlar dolar;
  - `keep`: eşleşen nesneye dokunulmaz.
- `geometry`: eşleşen nesne kaynağın geometrisini alır (varsayılan açık; türü uymayan reddedilir).
- `unmatched`: `add` (varsayılan) ya da `skip`.
- `duplicates` (kaynakta aynı anahtar birden çok satırda; ADR 0439 §4'ün sözleri):
  - `stop`: varsayılan, `duplicate_key` reddi, hiçbir şey yazılmaz;
  - `first`: ilk satır alınır, sonrakiler reddedilir;
  - `none`: o anahtarın hiçbir satırı alınmaz.
- Hedefte aynı anahtarı taşıyan birden çok nesne varsa o anahtarın satırları reddedilir (`target_ambiguous`).
- Satırın reddi şu nedenlerledir: anahtar boş, kaynakta yinelenen, hedefte birden çok, tipli alana uymayan değer (alanın
  kuralıyla, `cad.entities.set` gibi), katmanın türüne uymayan geometri. Reddedilen satır yazılmaz; içe aktarma geri kalanıyla
  sürer.
- Her satır şu sırayla sınanır, ilk tutan neden onun nedenidir:
  1. anahtar boş (`key_empty`);
  2. anahtar kaynakta yineleniyor (`key_duplicate`; `first`'te ilki geçer);
  3. geometri katmanın türüne uymuyor (`layer_type`; ADR 0622 §5);
  4. değer tipli alana uymuyor (`field_value`; alanın kuralı);
  5. anahtarlı eşlemede hedefte birden çok nesne (`target_ambiguous`);
  6. kimlikle eşlemede kimlik başka katmanın nesnesinin (`uid_elsewhere`);
  7. eşleşmeyen satır `add`'de boş bir kimlikle eklenir, `skip`'te alınmaz;
  8. eşleşen satır `keep`'te dokunulmaz; geometri güncellenecekse aynı ailede olmalıdır (nokta, çizgi, alan;
     `geometry_kind`); öznitelikler `update` ya da `fillEmpty` ile yazılır;
  9. sonuç var olan nesneyle aynıysa satır değişmeyen sayılır.

  Nedenlerin sözleri Türkçedir: “alan, çizgi katmanına konamaz”, “nokta, eşleşen alan nesnesinin yerine geçemez” gibi.
- Bütün ret nedenleri cevabın `rows_rejected` uyarısında sayılarıyla, kodların sırasıyla söylenir (“parsel.parquet: 3 satır
  alınmadı (1 satırın anahtarı boş, 2 satırın değeri alana uymuyor).”).
- Kilitli hedef katman bütün içe aktarmayı reddeder (her zamanki gibi).
- Tek geri alma adımı: eklenenler, güncellenenler ve dokunulmayanlar aynı adımda.

### 6. Reddedilen satırlar dosyası (`rejects`)

- Girdinin `rejects` yolu verilirse (`execute`'ta) reddedilen satırlar CSV olarak yazılır: `kaynak`, `satir`, `anahtar`,
  `neden`, kaynağın sütunları, `WKT` (geometri ISO WKT). GDAL'ın CSV'si gibi virgüllü, UTF-8, RFC 4180'in tırnaklamasıyla,
  satır sonu CRLF.
- Dosya birleştirme olmasa da her içe aktarmada yazılır: kaynağın bütün tablolarının sütunlarıyla, reddedilen yoksa yalnız
  başlıkla (bir iş akışı eski dosyayı yeni sanmasın). `plan` dosya yazmaz.
- Yol mutlak olmalı, `.csv` ile bitmeli ve klasörü var olmalıdır; yoksa `invalid_file_path` (`path: rejects`). Dosya önce
  yanındaki geçici adla yazılır ve yerine taşınır; yazılamazsa `file_unwritable` ve çizim değişmez.
- Sunucu işi (`project.import`, ADR 0561) `rejects` almaz: istek sunucunun diskindeki bir dosyayı adlandıramaz, işçi seçeneği
  `invalid` ile reddeder; reddedilenler işin sonucunda sayılır. Masaüstünün “Sunucuya içe aktar”ı seçeneği göndermez.
- Cevabın tablo satırlarında `added`, `updated`, `unchanged`, `skipped`, `rejected`; `reasons` (neden ve sayı); `rejects`
  (yazılan dosya ve satır sayısı).

### 7. Hazırlık

`plan` hazırlıktır: dosya çizimin kopyasında okunur, eşleme yapılır, sayılar ve nedenler döner; hiçbir şey yazılmaz. Onarım ve
sadeleştirme içe aktarmanın parçası değildir (Geçerliliği denetle, Onar ve Sadeleştir ayrı araçlardır, ADR 0201).

### 8. Masaüstü, Python ve MCP

- Dosya › Dışa aktar ▾'de GeoParquet dışa aktar, CBS'nin Harita › Katman ekle › Veri ekle ▾'sinde GeoParquet içe aktar
  (`catalog/v1/ribbon.json`);
  pencereler FlatGeobuf'unkiler gibi: dışa aktarmada satırlar ve satır grupları, sütunlar Parquet türleriyle, geometri türleri,
  sistem, boy, Sıkıştırma (Snappy, Sıkıştırmasız) ve Kutu sütununu (bbox) yaz; içe aktarmada planın sayıları, Başka sistemdeyse
  projenin sistemine çevir, birleştirme ve Sunucuya içe aktar. Kaynaklar `.parquet` dosyalarını GeoParquet olarak tanır ve
  içe aktarma penceresiyle açar. Sunucu işi GeoParquet'i de alır (`ImportFormat::Parquet`).
- İçe aktarma pencerelerinde (GeoPackage, FlatGeobuf, GML, CSV, GeoParquet, Shapefile, GeoJSON, KML/KMZ) **Var olan katmanla
  birleştir** bölümü:
  - Katman: adı eşleşen katman ya da seçilen katman (kilitli ve servis katmanları listede yok); seçilen katman komutun
    `layer`'ı (GeoPackage ve GML'de her tablo `layers` ile ona); adı eşleşen katman yoksa satırların yeni katmana gideceği
    söylenir;
  - Anahtar (satırları kalıcı kimlik taşıyan biçimlerde Kalıcı kimlik de) ve katmandaki alan; Eşleşen nesne (Güncelle, Yalnız
    boşları doldur, Dokunma), Geometriyi de güncelle, Eşleşmeyen satır (Ekle, Alma), Dosyada yinelenen anahtar (İçe aktarmayı
    durdur, İlkini al, Hiçbirini alma), Reddedilen satırları dosyaya yaz (`<dosya>-reddedilenler.csv`, dosyanın yanında);
  - planın sayıları canlı: katman başına eklenecek, güncellenecek, değişmeyecek, alınmayacak ve reddedilecek satırlar
    nedenleriyle; bitince “… birleştirildi: n nesne eklendi, m nesne güncellendi”.
- Python ve MCP katalogdan; iki komut MCP'nin yıkıcı listesinde (birleştirme var olan nesneleri değiştirir, dışa aktarma
  `overwrite` ile dosyanın üstüne yazar).

### 9. Alan değeri kuralına ek (ADR 0199 §1)

Ondalık alanda ölçeği aşan basamaklar yalnız sıfırsa kesilir, reddedilmez: ölçeği 2 olan alana “12.500” “12.50”, ölçeği 0
olana “100.000” “100” olarak yazılır; sıfır olmayan fazla basamak yine `scale` reddidir. Neden: Shapefile'ın `.dbf`'i ondalık
sayıları sütunun genişliğiyle (`N(24,15)`) sıfırlarla doldurur; kurumun yeniden yüklemesi bu yüzden her satırda reddedilirdi.
Kural `kentos_contracts::fields`'tadır, bütün yazma yollarına (komutlar, formlar, birleştirme) aynı uygulanır;
`layer_field_cases.py` ve alan durumlarının üreteçleri (`primitive_fields_cases.py`, `records_command_cases.py`) bağımsız
başvurularıyla yeniden yazıldı.

### 10. DBF'in boş sayısı

Sayı sütununda (N, F) yıldızla başlayan değer (`****`) shapelib'in kuralıyla NULL'dır; okuyucu (`shp::dbf`) ve bağımsız
okuyucu (`tools/formats/gis.py`) aynı kuralı izler. Önceden metin “****” olarak alınıp tipli alana yazılırken reddediliyordu.

## Sonuçlar

- GeoParquet okuma ve yazma; bulut verisi masaüstüne gelir, KentOS'un yazdığını pyarrow, geopandas ve GDAL (Parquet'li)
  okur.
- Aynı dosyanın yeni sürümü yeniden yüklenebilir; kurumun dönemsel yüklemesi tek adım ve açık rapordur.

## Doğrulama

Bağımsız başvurular (KentOS kodu olmadan; `.run/geo-py`: sistemin GDAL 3.12'li Python'u ve pyarrow 26):

- `compress_cases.py`: 61 sıkıştırma durumu; dosyaları başvuru kitaplıkları yazar (Snappy, GZIP, LZ4_RAW ve Brotli pyarrow'un
  codec'leri; ZSTD libzstd 1.5.7: düzeyler, sağlama, akışla bloklar, birden çok ve atlanacak çerçeve, uzun eşleşme, sözlük; LZ4
  çerçevesi, Hadoop çerçevesi ve çıplak blok `lz4`'le), bozukları açık hatadır.
- `parquet_cases.py`: pyarrow'un yazdığı 19 Parquet dosyası; kodlamalar, sayfa sürümleri, sözlükler, iç içe sütunlar ve düzeyler
  pyarrow'un okuduğuyla.
- `geoparquet_cases.py`: `file.import.parquet`'in 29 durumu geopandas'ın (WKB, GeoArrow, kutu sütunu) ve pyarrow'un dosyalarından;
  `--written` KentOS'un yazdığı 6 GeoParquet'i (`KENTOS_WRITE_PARQUET=1 cargo test -p kentos-headless --test headless
  geoparquet`) pyarrow, geopandas, shapely ve pyproj'la açar: `geo` anahtarı 1.1, PROJJSON kodun sistemi, geçerli geometriler,
  saat yönü tersine dış halkalar, her satırın kutusu, `geometry_types`.
- `import_merge_cases.py`: birleştirmenin 37 durumu sekiz biçimde (GeoPackage, GeoParquet, CSV, FlatGeobuf, GML, GeoJSON,
  Shapefile; dosyaları GDAL ve pyarrow yazar); eşleme, ret sırası, yinelenenler, reddedilenler dosyası (baytlarıyla) ve uyarıların
  sözleri betikteki kuraldan.
- Alan kuralının eki: `layer_field_cases.py`, `primitive_fields_cases.py`, `records_command_cases.py` yeniden yazıldı; DBF'in
  NULL'u: `gis_import_cases.py` (Shapefile 50, GeoJSON 33, KML 31) ve `gis_reference.py` yeni kuralla güncel.
- Sunucu işinin biçimleri `server_job_kinds_cases.py` (29 ad), Kaynaklar'ın listesi `source_list_cases.py`.

Rust: biçim çekirdeğinin `compress` ve `parquet` testleri; ortak komut durumları ve yetenek testi; birleştirmenin birim testleri
(RFC 4180 tırnaklaması, yazılamayan reddedilenler dosyası hiçbir şey bırakmaz); başsız sunucuda gidiş-dönüş (beş katman ve bütün
çizim; plan yazdığını söyler; kalıcı kimlikle geri birleştirme hiçbir şey eklemez); iş çalıştırıcısı `rejects`'i reddeder;
masaüstünde GeoParquet penceresinin beş testi (tek adım ve geri alma, sistemin onayı, kalıcı kimlikle yeniden alma, başka adlı
katmanın seçilmesi, sıkıştırmasız ve kutusuz dışa aktarma) ve sunucu işinin `rejects`'i göndermemesi.

Süreler (release; Intel i5-11300H, 8 iş parçacığı; `parquet_timing` ve `compress_timing`):

| İş | Süre |
|---|---|
| 100 000 parselin (12 köşe, beş tipli alan) GeoParquet'i, Snappy | 163 ms, 11,7 MB, 2 satır grubu |
| aynısı sıkıştırmasız | 143 ms, 33,8 MB |
| planı (yeni katmana) | 946 ms |
| içe aktarma (yeni katmana) | 929 ms |
| aynı satırların anahtarla birleştirme planı (10 000 güncellenen, 90 000 değişmeyen) | 1 053 ms |
| birleştirme | 1 034 ms |
| çözme hızı: Snappy, LZ4_RAW, Hadoop LZ4 | 1 268, 1 482, 1 450 MB/s |
| çözme hızı: ZSTD, GZIP | 274, 257 MB/s |
| Snappy yazma | 584 MB/s |

Birleştirmenin planı, 100 000 nesneli çizimde, boş çizimde yeni katmana alma planından ~100 ms uzundur.

## Sonraya kalan

- İçe aktarma pencerelerinin planı (FlatGeobuf, GML, CSV ve GeoParquet'te ortak) arayüz iş parçacığında yapılır: 100 000 satırlık
  dosyada seçenek başına yaklaşık bir saniye. Planın iş parçacığına taşınması ayrı iştir.
- OpenFileGDB, MapInfo (TAB, MIF) ve diğer uyumluluk biçimleri.
- Parquet'in sayfa dizini ve satır grubu süzgeciyle (istatistik, bbox `covering`) kısmi okuma; bölümlenmiş veri setleri
  (klasör, Hive yolları).
- ZSTD ve Brotli sıkıştırmasıyla yazma.
