# ADR 9191: Var olan GeoPackage'a yalnız bu tablolar ya da ekleme; teslimin Durdur'u

- **Durum:** kabul edildi (2026-11-23); adım VY-90. Plan: [plan/veri.md](../../plan/veri.md) VY-90.
- **İstek:** sahibin sözü (23 Kasım 2026): “Yarım kalan maddeleri tamamla”. Değerlendirmenin 6. maddesinin kalanı
  (ADR 9168 §4, 9178): “teslimin kendisinde ‘yalnız bu tablolar’ ve ‘ekle’ seçeneği (komut girdisi değişir), uzun
  teslimin Durdur'u.”
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi.
- **Komut:** `file.export.gpkg` v1'e isteğe bağlı `write` (`file`, `tables`, `append`) eklendi. Verilmezse eski davranış
  sürer. Sonuçta tablonun `replaced`, `appended`, `already` alanları ve dosyanın `kept`'i var. Yeni ret kodları:
  `geometry_mismatch`, `crs_mismatch`, `source_not_null`, `cancelled`. Katalog, TypeScript türleri ve `kentos.cad`
  yeniden üretildi; MCP aracı katalogdan gelir.

## §1 Bağlam ve araştırma

- Teslim var olan dosyanın yerine yenisini yazıyordu. Onay, silinen tabloları söylüyordu (ADR 9168). Öbür tabloları
  korumanın yolu her katmanı kendi dışa aktarmasıyla, tek tek yazmaktı (ADR 9178).
- Teslim arayüzün iş parçacığında koşuyordu: büyük bir teslimde pencere donuyor, teslim durdurulamıyordu.
- **Araştırma.**
  - QGIS 3.44 kılavuzu, “Creating Layers” §11.2.2.3
    ([create_layers.html](https://docs.qgis.org/3.44/en/docs/user_manual/managing_data_source/create_layers.html),
    okundu 23 Kasım 2026). Var olan bir dosyaya kaydederken, biçimin yeteneğine göre (GeoPackage, SpatiaLite…) kişi
    dört yoldan birini seçer:
    - bütün dosyanın üstüne yazmak;
    - yalnız hedef katmanın üstüne yazmak;
    - var olan hedef katmana nesne eklemek;
    - yeni alanları da ekleyerek nesne eklemek.
  - OGC GeoPackage 1.4 ([spec140](https://www.geopackage.org/spec140/index.html), okundu 23 Kasım 2026):
    - **Gereklilik 32:** geometri sütunu, bildirilen türün ya da ona atanabilir türlerin geometrilerini tutar.
    - **Ek G, alt türler:**
      - GEOMETRY her türü alır;
      - CURVE: LINESTRING, CIRCULARSTRING, COMPOUNDCURVE;
      - CURVEPOLYGON: POLYGON;
      - MULTICURVE: MULTILINESTRING;
      - MULTISURFACE: MULTIPOLYGON;
      - GEOMETRYCOLLECTION: MULTIPOINT, MULTICURVE, MULTISURFACE.
    - `gpkg_geometry_columns`'un `z` ve `m` değerleri: 0 yasak, 1 zorunlu, 2 isteğe bağlı.
  - SQLite, “VACUUM” ([lang_vacuum.html](https://sqlite.org/lang_vacuum.html), okundu 23 Kasım 2026): `VACUUM INTO`
    tutarlı bir anlık görüntü yazar. Ama VACUUM, tamsayı birincil anahtarı olmayan tabloların ROWID'lerini
    değiştirebilir. Bu yüzden kopya SQLite'ın çevrimiçi yedeğiyle (backup API) yapılır (§2.2).

## §2 Karar

1. **`write`** (`kentos_native_application::gpkg_into`).
   - **`file`** (varsayılan): eskisi gibi, yerine yeni dosya.
   - **`tables`:** yalnız yazılan tablolar yeniden yazılır. Dosyanın öbür tabloları, stilleri ve ilişkileri kalır.
     - Dosyanın yeniden yazılan bir tabloya bağlı ilişkisi, eşlemesi eski satırlara gittiği için çıkar ve söylenir.
       Katmanların kendi ilişkileri yeniden yazılır.
     - Düşürülen tablonun alan şeması satırları (`gpkg_data_columns`) da gider; yeni tablonun satırları yazılır.
   - **`append`:** satırlar dosyanın aynı adlı tablolarına eklenir. Dosyada olmayan tablo yeni yapılır.
     - Tablonun sütunları, stili, dizinleri ve ilişkileri dosyanındır. Tabloda sütunu olmayan alan yazılmaz, uyarıyla
       söylenir.
     - Tablonun kalıcı kimliğini tuttuğu nesne yeniden eklenmez; sayısı söylenir.
     - Uymayan tablo baştan reddedilir, hiçbir şey yazılmaz:
       - geometri ailesi Ek G'ye göre atanabilir değilse; tek parçalı sütuna çok parçalı, doğru parçalı sütuna yaylı
         nesne eklenmez; GEOMETRYCOLLECTION ve CIRCULARSTRING sütununa katman satırı eklenmez (`geometry_mismatch`);
       - Z ya da M sütunun söylediğine uymuyorsa (`geometry_mismatch`);
       - tablonun sistemi çizimin sistemi değilse (`crs_mismatch`, iki sistem adıyla);
       - NOT NULL ve varsayılansız bir sütunun katmanda karşılığı yoksa ya da eklenecek nesnelerde boşsa
         (`source_not_null`).
     - Satırlar dosyanın kendi uzamsal dizinine, dosyanın tetikleyicileriyle girer. Tablonun kutusu büyür. Birincil
       anahtarın adı dosyadan okunur. GDAL'ın satır sayımı tablosu (`gpkg_ogr_contents`) varsa gerçek sayıya getirilir.
     - Bir aracın tablosuna (işlem sonucu) hiçbir zaman eklenmez: var olan tablo alınmış sayılır.
   - Dosya yoksa `write` ne derse desin yeni dosya yazılır.
2. **Bütün ya da hiç.**
   - Var olan dosya önce SQLite'ın çevrimiçi yedeğiyle geçici bir kopyaya alınır (`GeoPackage::copy_to`). Kopya sayfa
     sayfadır: başka bir programın write-ahead log'da tuttuğu yazılanlar da gelir, hiçbir tablonun ROWID'i değişmez.
   - Yazım kopyada tek işlemde yapılır. Kopya sonra tek adımla dosyanın yerine konur (ADR 9178'in yolu).
   - Durdurulan, reddedilen ya da başarısız yazım dosyayı olduğu gibi bırakır.
   - GeoPackage olarak okunamayan dosyaya `tables` ya da `append` istenirse nedeniyle reddedilir (`file_unwritable`).
   - `rusqlite`'ın `backup` özelliği yalnız `kentos-geopackage`'ta açıldı. Aynı sürümdür ve gömülü SQLite'ın kendi
     işlevleridir; yeni bağımlılık değildir.
3. **Durdur.** `export_gpkg_flow` her satır partisinden önce durup durmayacağını sorar (parti 5.000 satır) ve ilerlemeyi
   söyler. Durdurulan teslim `cancelled` ile reddedilir.
4. **Masaüstü.**
   - Teslim ayrı bir iş parçacığında, çizimin okuma kopyası üzerinde koşar. Ayak ilerlemeyi söyler (“15.000 / 20.007
     satır yazıldı (%75)”) ve çubukla gösterir. Durdur, “Dışa aktarma durduruldu; dosyaya hiçbir şey yazılmadı.” der,
     pencere açık kalır. Geç gelen cevap düşer.
   - Var olan dosya seçilince pencere “Dosyaya nasıl yazılsın?” diye sorar: “Yalnız bu tabloları yeniden yaz”, “Tablolara
     ekle”, “Dosyanın yerine yaz”. Varsayılan ilkidir: öbür tabloları silmeyen tek yol odur.
   - Satırlar seçilen yola göredir:
     - yeniden yazılan ya da satır eklenen tablolar, dosyadaki satır sayılarıyla;
     - yeni tablolar;
     - olduğu gibi kalan tablolar;
     - dosyanın yerine yazmada silinen tablolar (ADR 9168).
   - Düğme yolu söyler: Tabloları yeniden yaz, Ekle, Üstüne yaz.
   - Yazılınca kayıt yazılanı söyler: “teslim.gpkg” yazıldı: “Duraklar” tablosuna 3 satır eklendi; 4 tablo olduğu gibi
     kaldı.

## §3 Kanıt

- **GeoPackage çekirdeği:** `kentos-geopackage` sınamaları geçiyor.
- **Uygulama:** `gpkg_into` birim sınamaları (Ek G'ye göre uygunluk, Z ve M).
- **Başsız** (`tests/headless/gpkg_write_into.rs`):
  - `only_these_tables_are_written_again_and_the_files_others_stay`:
    - `tables` ile tablo 5'ten 7 satıra yeniden yazıldı; `eski_tablo` bir satırıyla kaldı;
    - tamsayı anahtarı olmayan, ara satırı silinmiş `notlar` tablosunun ROWID'leri (1, 3) korundu;
    - dosyanın ilişkisi çıktı ve söylendi; `integrity_check` ok;
    - GDAL (`ogrinfo`, GDAL 3.13.3) aynı sayıları okudu;
    - dosya yokken `append` yeni dosya yazdı.
  - `rows_are_added_to_the_files_table_and_an_object_it_has_is_not_added_again`:
    - `append` 3 satır ekledi, 5 nesne zaten vardı; tabloda olmayan alan için uyarı;
    - kalıcı kimlikler 8 ayrı; uzamsal dizinde 8 satır; kutu yeni noktalara büyüdü; GDAL 8 satır okudu;
    - ikinci ekleme 0 satır ekledi, 8 nesne zaten vardı.
  - `a_table_that_cannot_hold_the_rows_is_refused_and_the_file_stays`: POLYGON tabloya nokta (`geometry_mismatch`),
    EPSG:4326 tablosu (`crs_mismatch`), karşılığı olmayan NOT NULL sütun (`source_not_null`). Her birinde dosya bayt
    bayt aynı; aynı tablo `tables` ile yazılıyor. GeoPackage olmayan dosya reddedildi.
  - `a_delivery_stopped_between_two_batches_writes_nothing`: 12.003 satır, ilk partiden sonra durdurma. `file`,
    `tables` ve `append` üçünde de `cancelled`, dosya bayt bayt aynı.
- **Uygulama sınamaları:** GeoPackage, işlem sonucunun dosyaya yazılması ve yetenekler (32) geçiyor.
  - `processing_sink`'in tablo sınaması ADR 9158'den beri düşüyordu: Özet istatistik tablosunu metin bekliyordu. Aracın
    türlü değerlerini bekleyecek biçimde düzeltildi.
- **Masaüstü** (`exchange/gpkg_tests/write_into.rs`):
  - pencereden ekleme 4 satırı 7'ye çıkarıyor, kayıt eklenen ve zaten var olan nesneleri söylüyor;
  - Durdur (20.003 satır): pencere açık, durum sözü doğru, dosya aynı, Geri onayı kapatıyor.
- ADR 9168'in sınaması yeni varsayılana göre güncellendi: yalnız bu tablolar `eski_tablo`'yu koruyor, dosyanın yerine
  yazma siliyor. Teslim sınaması onayın ilk satırını yeni varsayılanla okuyor. Masaüstü GeoPackage sınamaları (31) ve
  ortak sınamalar (155) geçiyor.
- **Python:** `sdk.py --check` temiz. `write` ve yeni yanıt alanları türlerden gidip geliyor (elle denendi). Alışveriş
  sınamaları (10) var olan yerli modülle geçiyor. Yerli modül yeniden derlenmedi, `pnpm py:test` koşmadı.
- **Resimler:** `.run/gui-evidence/kullanim-9191`: üç yol, Tablolara ekle, Dosyanın yerine yaz, yazılırken Durdur
  (Git'e girmez).

## §4 Kalan

- Eklerken tabloya yeni alanları da eklemek (araştırmadaki dördüncü yol). Bugün sütunu olmayan alan yazılmaz, söylenir.
- Eklenen tablolar arasındaki ilişki eşlemesine satır eklemek. Bugün dosyanın ilişkileri kalır, katmanların ilişkileri
  eklenmez, söylenir.
- GDAL'ın yazdığı ve `gpkg_ogr_contents` tutan bir dosyaya ekleme sınanmadı.
- Gerçek pencerede deneme; web (TODOS.md §0.2).
