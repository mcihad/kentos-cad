# ADR 0530: CAD kaynağının GIS dataset'inde izlenebilir türevi

- **Durum:** kabul edildi (2026-10-19). `plan/gis.md` CBS-16'nın ilk dilimi: GeoPackage'a izlenebilir türev, türevin denetimi,
  türevin kaynağın yerine geçmemesi. Kapsam sahibin sözü: “CAD kaynağını GIS'e kontrollü eşler; geometriyi tek tip polyline'a
  indirgemez”, “kaynak geometriyi değiştirip eski türevi güncel bırakma”; ilke “Performance First”.
- **Bağlam belgesi:** `plan/gis.md` CBS-16, ADR 0006 (CAD ve PostGIS kaynak otoritesi), 0502 (GeoPackage), 0503 (bağlı katman),
  0511 (sonucun soyu: metadata uzantısı), 0514 (Shapefile dışa aktarma), 0522 (CSV), 0259 (NURBS), 0144 (bloklar), 0149 §5.3
  (eğrilerin kirişle hesabı), 0253 (komut protokolü), CLAUDE.md §15 (CAD ve PostGIS kaynak otoritesi), §23 (sayısal doğruluk).
- **Web:** ertelendi (ADR 0228); geri dönüşte gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - türevdeki GIS değişikliğinin kaynağa geri alınması: öznitelik değişikliği (kaynak da değişmediyse) ya da açık ilişki koparma
    (kaynak nesnenin yerine GIS geometrisi). Bu dilim yalnız söyler ve reddeder (§4); geri alma ayrı bir komut ister;
  - PostGIS türevi (CBS-04 dış tablosu), Shapefile ve GeoJSON türevi: aynı türetme (`gis_derive`) başka sürücülere;
  - ölçü, tarama, kılavuz, tablo ve mimari nesnelerin (mahal alanı, duvar yüzü) GIS karşılığı: bugün sayılıp nedeniyle söylenir;
  - spline'ın sapmasının bağımsız doğrulaması (Catmull–Rom'un bağımsız başvurusu yok; fit noktalarının köşe olduğu denetlenir);
  - büyük türevde bellek: dosya bellekte bir kez bütün okunur (`file.export.gpkg` gibi); akışla yazma CBS-42'nin işi.

## Bağlam

`file.export.gpkg` (ADR 0502) çizimi GeoPackage'a yazar; yayı eğri uzantısıyla (`CIRCULARSTRING`) kesin yazar, elips, eğri ve
NURBS'ı 0,1 mm'lik kirişlerle yazar ve “yaklaşık yazıldı” der. Plan CBS-16 bundan fazlasını ister:

- yaylı parsel ve bloklu bina `.kcad` ↔ GIS dışa aktarımı sonrasında düzenlenebilir kaynağını korusun; kaynak çizimde kalsın,
  GIS'teki geometri **türev** olsun, türev olduğu bilinsin;
- her GIS nesnesi kaynağını (kalıcı kimlik, tür) taşısın; yazı, ölçü, tarama ve blok kendiliğinden aynı anlamda GIS poligonu ya
  da noktası sayılmasın; blok öznitelikleri açıkça alan olsun;
- türev sapma toleransı ve algoritma ile üretilsin, sapma kullanıcıya ve otomasyona açık olsun; haritanın zoom'u nihai alan ve
  uzunluğu değiştirmesin;
- kaynak değişince eski türev güncel kalmasın; GIS'te türevi düzenlemek kaynağın yerine geçmesin.

Eğri uzantılı yazım (`file.export.gpkg`) QGIS ve GDAL'da çalışır ama her program onu okumaz (ArcGIS, bazı veritabanı araçları ve
basit okuyucular `CIRCULARSTRING`'i okuyamaz) ve türev olduğunu söylemez. Bu ADR ayrı bir komut ekler; eskisini değiştirmez.

## Karar

### 1. Kapsam

- **`file.export.gis`** (dosya komutu, başsız sunucuda): katmanları GeoPackage'a türev olarak yazar (§2, §3).
- **`cad.gis.derivation`** (salt okunur): bir türev dosyanın açık çizimle hâlâ eşleşip eşleşmediğini söyler (§4).
- **Politika** (§5): türevin tablosu bağlanmaz, katmana okunmaz, geri yazılmaz; başka çizime alınınca ne olduğu söylenir.
- Masaüstünde iki pencere (§7); Python ve MCP katalogdan.
- Kapsam dışı: yukarıdaki “Sonraya kalan”.
- `.kcad` şeması değişmez: türetim kaydı GeoPackage'ın içindedir, çizim hiçbir şey yazmaz.

### 2. Türetme: kirişler ve sapmaları (`kentos_geometry_core::ops::derive`, algoritma `kentos.chords/1`)

Türev yalnız düz geometridir: `POINT`, `LINESTRING`, `POLYGON` ve çok parçalı biçimleri. Eğri uzantısı yazılmaz. Her yay
kirişe çevrilir ve her satır kaynağından en çok ne kadar saptığını söyler.

| Kaynak | Kirişler | Sapma |
|---|---|---|
| daire, yay, çoklu çizgi ve alanın yaylı kenarı (büküm) | eşit açılı; uçlar yayın üzerinde (bükümlü kenarın uçları kenarın kendi köşeleri) | **kesin**, `exact`: 2·r·sin²(φ/4), φ bir kirişin açısı |
| elips ve elips yayı | `geom::curve_outline`'ın bölmesi: parametre adımı h olan parça kirişinden en çok a·h²/8 uzakta | algoritmanın tuttuğu sınır, `bound` (= tolerans) |
| fit noktalı eğri (spline) | Bézier aralığı, iki iç denetim noktası kirişe toleransın içinde olana dek ikiye bölünür | sınır, `bound` |
| NURBS | `Nurbs::tessellate(tolerans)`: her parçada üç noktada denenir | örneklenmiş, `sampled` (kanıtlanmış değil) |
| düz kenar, nokta, kayıt | kendi noktaları | 0 (`none`) |

- **Kiriş sayısı:** yayın yarıçapı r ve süpürmesi θ için sagitta'sı toleransı tutan **en az** n; hiçbir kiriş çeyrek turdan
  geniş olmaz; bir yay en çok 65 536 kirişe bölünür (`MAX_CHORDS`). Sayı yetmezse sapma toleransın üstündedir ve söylenir
  (`export_losses`); sapma sütunu ve kayıttaki en büyük sapma gerçeği yazar.
- **Z ve M:** kaynağın köşesinde kendi değeri; bir yayın ara köşesinde, yayın iki ucunda değer varsa, süpürmenin payı k/n kadar
  doğrusal; uçlardan birinde yoksa ara köşede yok (GeoPackage'ta Z'li geometrinin değersiz köşesi NaN yazılır). Alanın M'si yok.
- **Belirlilik:** aynı girdi ve tolerans her platformda aynı noktaları verir (`libm`'in sin ve cos'u). Denetim bu yüzden yeniden
  türetip karşılaştırabilir (§4).
- **Aralık:** tolerans 0,000001 ile 1 metre arasında (`invalid_tolerance`), varsayılan 0,0001 (çizimin hesapta eğrilere tuttuğu
  sınır). Birim her zaman metre; çizim birimi mm ya da cm olsa da geometri metrededir.
- **Alan ve uzunluk:** türevin alanı kaynağın alanından en çok (yayların uzunluğu × sapma) kadar küçüktür; kirişler yayın içinde
  kalır ve her kirişin sliver'ı `r²/2 (φ − sin φ)`'dir. Harita yakınlaştırması türetmenin girdisi değildir: tolerans metredir ve
  çizimin hiçbir görünüm değeri okunmaz; aynı çizim aynı toleransta her zaman aynı dosyayı verir (zaman damgası ve sürüm
  dışında).

### 3. Dosya: tablolar, sütunlar, türetim kaydı

**Tablolar.** Her çizim katmanı bir özellik tablosudur (adı katmanın adı; aynı adda ikincisi `_2`). Yazı ve bloklar katman
tablolarına girmez; `annotations` seçilir:

- `table` (varsayılan): her blok tanımının yerleştirmeleri `blok_<ad>` tablosunda nokta olarak (yerleştirme noktası), bloğun
  öznitelikleri sütun (tanımın ATTDEF sırasıyla; değer yerleştirmenin özniteliği, yoksa tanımın varsayılanı), `scale` ve
  `rotation_deg` sütunları; yazılar `kentos_annotations` tablosunda nokta olarak (yazının noktası), `text`, `height`,
  `rotation_deg` sütunları;
- `skip`: ikisi de yazılmaz ve sayılır.

**Yazılmayanlar** nedeniyle `left`'te ve `export_losses` uyarısındadır: ölçü (değeri çizimde kaynağından hesaplanır), tarama
(desen ve sınır, GIS alanı değil), kılavuz, tablo, resim, raster, nokta bulutu, yardımcı çizgi ve ışın, mimari nesneler (duvar,
açıklık, mahal, aks, kolon, merdiven, rampa, etiket, görünüm işareti). Hiçbiri sessizce poligon ya da nokta olmaz.

**Sütunlar** (her tabloda başta, bu sırayla): `kentos_uid` (kaynağın kalıcı kimliği; ayrıca ADR 0502'nin `kentos_uids`
uzantısında, QGIS'in öznitelik tablosunda da görünsün diye iki yerde), `kentos_kind` (kaynak türü: `polygon`, `circle`,
`arc`, `ellipse`, `nurbs` …), `kentos_deviation` (metre), `kentos_digest` (kaynağın özeti). Blok ve yazı tablolarında
ardından `kentos_layer` (nesnenin katman adı). Sonra rolün sütunları, katmanın alanları (`LayerField` türleriyle, ADR 0502 §2'nin
türleri), alan olmayan öznitelikler metin sütunu. Aynı adda (büyük küçük harf ayrı) bir sütun varsa sonraki `_2` alır ve
söylenir ve türetim kaydının `renamed`'ına yazılır (denetim sütunu kaynağın adıyla okur); `kentos_` önekli adlar türevindir,
kullanıcının özniteliği `kentos_uid_2` olur.

**Kaynağın özeti** (`kentos.derivation-digest/1`): nesnenin JSON'unun, bakışı çıkarıldıktan sonraki kanonik biçiminin (ADR 0253:
anahtarlar sıralı, sayılar en kısa) SHA-256'sı. Çıkarılanlar: `id` (yuva, kalıcı değil), `color`, `lineWeight`, `label`,
`symbol`, `labelPins`, `attachments`, `lineType`, `linePattern`, `lineTypeScale`, `transparency`, `order`, `byLayer`,
`wipeout`, `annotative`. Yerleştirmede bloğun adı ve öznitelik tanımları da girer (varsayılan değer değişince yerleştirmenin
satırı eskir). Katman kimliği girer: nesne başka katmana taşınırsa eski tablodaki satırı eskir ve yeni katmanın tablosunda
satırı yoktur.

**Türetim kaydı.** Her tablo, GeoPackage'ın metadata uzantısında (ADR 0511'in yolu: `gpkg_metadata`, `gpkg_metadata_reference`;
kapsam `dataset`, MIME `application/json`) standardı `https://kentos.app/derivation/1` olan bir belge taşır
(`kentos_contracts::DerivationRecord`, `kentos.derivation` 1):

| Alan | Anlamı |
|---|---|
| `table`, `role` | tablo; `layer`, `blocks` ya da `annotations` |
| `layer`, `block` | katman tablosunda `{id, name}`; blok tablosunda bloğun adı |
| `layers` | dışa aktarmanın aldığı katmanlar (kimlikle, grupların yaprakları): bir tablonun satırı olmayan nesneyi saymanın kapsamı |
| `algorithm`, `tolerance`, `unit` | `kentos.chords/1`, kirişlerin kesildiği tolerans, `m` |
| `source` | `{project, name, revision, srid}`: kaynak proje (kimliği varsa), ad, çizimin dışa aktarma andaki sürümü, EPSG |
| `rows`, `maxDeviation` | satır sayısı; tablonun en büyük sapması |
| `kinds` | kaynak türüne göre sapmanın nasıl bilindiği: `exact`, `bound`, `sampled`, `none` |
| `renamed` | çakışma yüzünden başka adla yazılan sütunlar: sütunun adı → kaynağın özniteliği ya da alanı (yoksa yazılmaz) |
| `digest`, `engine`, `made` | özet kuralı, `KentOS <sürüm>`, UTC zaman |

Çizimin sürümü yalnız bilgidir (açılışta sıfırdan başlar); eski olup olmadığını **özetler** söyler, sürüm değil.

### 4. `cad.gis.derivation`: türev kaynağıyla eşleşiyor mu

Salt okunur; dosyayı açar, her türev tablonun her satırını kalıcı kimliğiyle çizimdeki nesneye bağlar (`kentos_uids`, yoksa
`kentos_uid`):

| Durum | Anlamı |
|---|---|
| `current` | kaynağın özeti satırınki ve satır, kaydın algoritması ve toleransıyla yeniden türetilen satırla aynı (koordinatlar 1e-9 m içinde, öznitelik sütunları satırın okuma kuralıyla) |
| `stale` | kaynağın özeti satırınkinden başka (ya da satırda özet yok): kaynak dışa aktarmadan sonra değişti ya da artık o tablonun satırı olarak türetilemiyor |
| `edited` | özet aynı, satır yeniden türetilenden farklı: GIS'te değiştirilmiş; `fields` neyin farklı olduğunu söyler (`geometry`, sütun adları) |
| `source_gone` | kimliği çizimde yok |
| `not_exported` | tablonun katmanında (blok ve yazı tablosunda kaydın katmanlarında) olup türevde satırı olmayan nesne |

Kaynak da, satır da değişmişse satır `stale`'dir: özet eşit değilse satırın eski kaynağın türevi olup olmadığı bilinemez.
Cevap tabloların sayılarını, toplamları, dosyanın kaydettiği kaynağı, bu çizimle aynı proje olup olmadığını (`sameProject`:
projenin kimliği ikisinde de varsa o, yoksa ad) ve eşleşmeyen satırları en çok `limit` (varsayılan 100, en çok 1000) kadar
listeler; listelenmeyenler `unlisted`'dadır. Eski ya da kaynağı yok ya da satırı olmayan varsa `derivation_stale`, GIS'te
değişen varsa `derivation_edited` uyarısı verir. Hiçbir şey yazmaz: **stale ve edited satırlar çizimi değiştirmez ve GIS'teki
değişiklik çizime geri yazılmaz.** Retler: `invalid_file_path`, `file_not_found`, `import_unreadable`, `not_a_derivation`.

### 5. Politika: türev kaynağın yerine geçmez

Türevin tablosunda yay yoktur, kirişler vardır; onu katman olarak bağlamak, katmana okumak ya da geri yazmak çizimin yaylı
kaynağını kirişlerle ezerdi. Bu yüzden:

- `cad.sources.edit` `link` (yeni katmana ya da var olan katmana) ve `cad.sources.sync` (`write` ve `read`) türev tablo için
  **`derived_dataset`** ile reddedilir; ileti türetimi (algoritma, tolerans, kaynak, sürüm) ve kaynağın çizimde olduğunu söyler.
  Daha önce bağlanmış bir türev tabloya da (ayarda kalmış bir bağ) yazılmaz ve okunmaz;
- `file.import.gpkg` türev tabloyu alabilir (başka bir çizime, örneğin kirişleri bir hesapta kullanmak için) ama kaynağın
  nesneleri çizimdeyken ikinci kez alınamaz: kalıcı kimlikler zaten çizimin (`entity_exists`, ADR 0502). Başka bir çizime alınınca
  **`derived_import`** uyarısı alınan tabloların türev olduğunu söyler (kaynak, sürüm, tolerans): yaylar yay olarak değil çoklu
  çizgi ve alan olarak gelir;
- GIS'te yapılan değişiklikleri `cad.gis.derivation` gösterir; geri almak (öznitelik) ya da koparmak (geometri) bu dilimde yoktur
  (Sonraya kalan).

### 6. Komutlar ve otomasyon

- **`file.export.gis`** v1 (`ExportGis`): `path`, `layers`, `tolerance` (m), `annotations` (`table`, `skip`), `overwrite`. Retler,
  sırasıyla: `invalid_file_path`, `file_exists`, `invalid_tolerance`, `layer_not_found`, `nothing_to_export`, `file_unwritable`;
  uyarı `export_losses`. Cevap: yol, boy, SHA-256, algoritma, tolerans, tablolar (rol, kaynak, satır, geometri türü, kirişli satır
  sayısı, en büyük sapma), `left` (tür, sayı, neden), toplam kirişli satır ve en büyük sapma, çizimin sürümü. Dosya geçici yerde
  kurulup okunur, bütün yazılır ya da hiç (ADR 0502'nin yolu); `plan` yazmadan söyler. Plan ve yürütmenin dosyaları zaman
  damgası yüzünden bayt bayt aynı değildir (`made`).
- **`cad.gis.derivation`** v1 (`GisDerivation`): `path`, `limit`. Masaüstü komutu (`DESKTOP_COMMANDS`), başsız sunucuda da.
- Yeni kodlar: `derived_dataset`, `derived_import`, `not_a_derivation`, `derivation_stale`, `derivation_edited`.
- Katalog (`catalog.rs`, `capabilities.rs`, `codes.rs`), kayıt (`registry.rs`, `lib.rs`), başsız sunucu (`gis_export.rs`),
  Python (`kentos.cad.file.export.gis`, `kentos.cad.gis.derivation`; `sdk.py` üretir), MCP (`file.export.gis` yıkıcı listede,
  yazılan dosyanın üzerine `overwrite` ile yazabildiği için; `cad.gis.derivation` salt okunur), manifest
  (`catalog/v1/operations.json`: `file.export.gis` `semantic`, `gis.derivation` `query`).
- Ortak durumlar: `fixtures/commands/v1/cad.gis.derivation.json` (33 durum) ve `fixtures/gis-derivation/v1/chords.json` (29 yay,
  8 bükümlü kenar, 5 yol); iki platformun Python ve Rust oynatıcıları aynı dosyayı çalıştırır.

### 7. Arayüz (masaüstü)

- **GIS türevi dışa aktar…** (`file.export.gis`; ikon `exportGis`: sayfa, çıkış oku, kesikli yay ve üstünde düz kirişler):
  tolerans (0,01 / 0,1 / 1 / 10 mm, hepsi tıklanabilir), yazılar ve bloklar (ayrı tablolara / atla); pencere yazmadan önce tabloları,
  kirişli satır sayısını, en büyük sapmayı ve yazılmayanları nedenleriyle önizler (plan ayrı bir iş parçacığında, seçim
  değişince yenilenir, eski cevap atılır); Dışa aktar… yolu sorar ve `file.export.gis` ile yazar. CBS'de Veri › Dosya
  alışverişi'nin ▾ listesinde (şerit 1100 px'e sığsın diye düğme olarak değil), CAD'de Yönet › GIS panelinde.
- **GIS türevini denetle…** (`gis.derivation`, komutu `cad.gis.derivation`; ikon `gisDerivation`: kesikli yay, kirişler ve onay
  işareti): GeoPackage seçtirir, tabloların sayılarını, eşleşmeyen satırları nedenleriyle ve uyarıları gösterir; Başka dosya… başka
  bir türevi açar.
- Seçilen ikonlar: `exportGis`, `gisDerivation` (sahibe sorulmadan seçildi; resimlerde gösterilir).

### 8. Performans

Her ağır iş arayüz iş parçacığında değildir: pencerenin planı ve yazması `off_thread`'de, komut başsız sunucuda kendi
çağrısında. Bütçeler (140 000 nesnede, 100 000 yaylı parsel, 20 000 daire, 20 000 elips; release):

- açılış ≤ 1 s, plan ≤ 15 s, dışa aktarma ≤ 15 s, denetim ≤ 10 s (nesne başına ~50 µs);
- bellek dosya boyunda bir kez (bkz. Sonraya kalan).

## Uygulama

- **Çekirdek:** `geometry-core`: `ops::derive` (`arc_chord_count`, `arc_chords`, `path_chords`, `sagitta`, `Basis`), `geom::curve_outline`'ın
  tolerans parametreli `ellipse_outline_tol` ve `spline_outline_tol`'u (var olan `*_outline` aynı sayıları verir).
- **Sözleşme:** `crates/shared/contracts/src/file_gis.rs` (`ExportGis`, `GisExported`, `GisTableExported`, `GisLeft`,
  `GisAnnotations`, `GisTableRole`, `DerivationRecord`, `DerivationSource`, `GisDerivation`, `GisDerived`, `DerivationTable`,
  `DerivationEntry`, `DerivationStatus`), `codes.rs`, `catalog.rs`, `capabilities.rs`, `file_export.rs` (`FILE_COMMANDS`);
  üretilen TS türleri ve `commandCatalog.json`.
- **GeoPackage crate'i:** `GeoPackage::metadata(table)` (tablonun metadata belgeleri).
- **Uygulama katmanı:** `gis_derive.rs` (`derive_entity`, `digest`, `write`, `check`, `record_of`, `derived_table`,
  `derived_tables`), `gis_derivation.rs` (komut), `sources.rs` (`link` ve `sync`'te `derived_dataset`), `file_import_gpkg.rs`
  (`derived_import`), `geopackage.rs`'in `fit` ve `srs_of`'u `pub(crate)`.
- **Başsız sunucu:** `gis_export.rs` (`make`, `export`), `file_commands.rs`'in `run_typed`, `version_of` ve `normalize`'ı.
- **Masaüstü:** `exchange/gis_export.rs`, `exchange/gis_check.rs`, `exchange/mod.rs`, `history/text.rs` (komut başlıkları; üç eksik
  başlık da yazıldı: `cad.rasters.georef`, `file.export.csv`, `file.import.csv`).
- **Otomasyon:** Python SDK (`sdk.py`), MCP (`tools.rs`), manifest, `catalog/v1/additions.json` (iki komut, iki ikon, şerit yerleri).
- **Bağımsız başvurular:** `scripts/fixtures/gis_derivation_drawing.py` (çizim `fixtures/gis-derivation/v1/drawing.json`),
  `scripts/fixtures/cad_gis_derivation_cases.py` (mpmath), `scripts/fixtures/gis_derivation_files.py` (OGR).
- **Sonraya kalan:** yukarıda.

## Doğrulama

- **Kiriş sayısı ve sapma** (`chords.json`, mpmath 50 basamak, KentOS kodu yok): 29 yay (altı yarıçap × dört tolerans, yaylar,
  tolerans yarıçaptan büyük, kapak), 8 bükümlü kenar, 5 yol. Rust'ın kapalı biçimli sayısı arama ile bulunanla her durumda aynı;
  sapma 1e-12 bağıl içinde; noktalar çemberde; bir kiriş çeyrek turdan geniş değil. `cargo test -p kentos-geometry-core --test all
  derive` (4 test).
- **Komutun durumları** (`cad.gis.derivation.json`, 33 durum): güncel, doğrulama ve plan, öznitelik değişen ve taşınan kaynak (eski),
  yarıçapı değişen daire, silinen nesne ve yazı (kaynağı yok), sonradan yapılan alan, yazı ve blok yerleştirmesi (türevde yok),
  bloğun öznitelik tanımı değişince iki yerleştirme eski, tek yerleştirmenin özniteliği değişince yalnız o, bakış değişikliği
  (renk, kalınlık, etiket, çizgi tipi) güncel, başka katmana taşınan nesne (eski ve yeni katmanda eksik), `limit` ve `limit 0`,
  GIS'te değişen öznitelik, geometri ve yazı (`edited`, `fields`), kaynağı da değişen düzenlenmiş satır (`stale`), GIS'te
  silinen satır, kimliği değişen satır, başka çizim (`sameProject` yanlış), altı ret (yol mutlak değil, klasör, dosya yok, GeoPackage
  değil, türetim kaydı yok), türevin yeni ve var olan katmana bağlanamaması, bağlı türev tabloya yazılamaması ve okunamaması, türevin
  aynı çizime alınamaması, başka çizime alınınca `derived_import`. Dosyanın boyu ve SHA-256'sı hashlib'in, her tablonun en büyük
  sapması yayların kesin sagitta'sı (1e-12 içinde). Aynı dosya Python oynatıcısında da geçer (`test_command_cases.py`).
- **Dosyanın OGR ile okunuşu** (`gis_derivation_files.py`, QGIS 4.2.3'ün GDAL 3.13'ü ve numpy; KentOS kodu yok): tablolar, sütunlar,
  fid ve kimlikler (`kentos_uid` ile `kentos_uids` aynı, hiçbiri iki kez değil), türler, blok sütunları (varsayılan değerle),
  yazı sütunları, türetim kayıtları; **her yay, daire ve bükümlü kenar için** kirişlerin sayısı ve köşeleri çember üzerinde;
  **her eğrisel satır gerçek eğri boyunca 20 000 noktadan örneklenip** kirişlerden en uzak örnek ölçüldü: sapmanın içinde, yaylarda
  sapmanın %99'undan fazla (yazılan sapma gerçekten büyük değil), elips ve NURBS çeyrek çemberinde tolerans içinde; spline'ın
  fit noktaları köşe; Z ve M (köşede kendi değeri, kirişte payı); halkalar kapalı, poligonlar geçerli; parselin alanı: OGR'ın
  yazılan poligonun alanı (1316,300080 m²), köşelerin shoelace'ı ve yayların sliver'larından bağımsız hesapla 1e-6 içinde aynı,
  analitik alandan (1316,301961 m²) 0,001882 m² küçük, sınırın (yay uzunluğu × sapma = 0,004139 m²) içinde; GDAL'ın
  `validate_gpkg.py`'si türev ve kopyaları için QGIS'in kendi dosyasından fazla bulgu vermez; düzenlenmiş kopyalar `ada.gpkg`'den
  tam değiştirilen satırlarda ayrılır.
- **Başsız sunucu** (`tests/gis.rs`): plan yazmaz; yazım 17 satır, 6 tablo, dokuz kirişli satır, en büyük sapma tolerans içinde,
  ışın ve yardımcı çizgi `left`'te; çizim değişmedi (sürüm, geri alma, nesneler aynı); `.kcad`'e kaydedip yeniden açınca
  yaylı parsel hâlâ yaylı (`bulges`) ve türev hâlâ güncel (17); retler; ince tolerans; `skip`; politika (bağlanmaz, aynı çizime
  alınmaz, başka çizime `derived_import` ve yollar kirişlerle gelir); adı çakışan sütunun (`kentos_uid_2`, `ADA_2`) kayıtla
  okunması. Üç test.
- **Masaüstü** (`exchange/gis_derivation_tests.rs`): pencere seçimlere göre planlar, tolerans ve yazı seçimi planı değiştirir, yazar;
  denetim kendi çiziminde 17 güncel, başka çizimde 17 kaynağı yok, GIS'te değişenlerde 3 `edited`, türev olmayan dosyada ret. Resimler
  `.run/shots/cbs-turev-*` (koyu ve açık tema, 1440×900 ve 1100×650; `screens`).
- **Python** (`test_gis_derivation.py`, `test_command_cases.py`): SDK ile dışa aktarma, `sqlite3` ile satırlar ve kayıt, denetim,
  değişen kaynak, politika.
- **Taşıma ve komut denetimleri:** `cargo test -p kentos-contracts` (katalog, birimler, kodlar), `-p kentos-native-application`
  (yetenek testi dahil: her kod bir durumda), `-p kentos-headless`, `-p kentos-geopackage`, `-p kentos-mcp`, `-p kentos-geometry-core`;
  `clippy -D warnings`; `node scripts/arch/deps.mjs`; `python3 scripts/cad/manifest.py --check`; `python3 scripts/python/sdk.py --check`.
- **Süreler** (release, Apple M5; 140 000 nesne: 100 000 yaylı parsel, 20 000 daire, 20 000 elips; `gis_timing`):

  | İş | Süre |
  |---|---|
  | açılış (v1 JSON) | 0,37 s |
  | `file.export.gis` plan | 6,8 s |
  | `file.export.gis` yazma | 7,1 s (dosya 975 MB) |
  | `cad.gis.derivation` | 4,0 s (140 000 satır) |

  Bütçelerin içinde. Dosya büyüktür çünkü 0,1 mm toleransta 3 m yarıçaplı bir daire 385, 6 m'lik bir elips yaklaşık 1 000 kirişe
  bölünür (nesne başına 6–16 KB); toleransı gevşetmek (1 mm) ~3 kat küçültür. Plan dosyayı yazıp okuduğu için yazma kadar sürer.
- **Ölçülmeyenler / bilinenler:** PostGIS, Shapefile ve GeoJSON türevi yok. Spline'ın sapması yalnız fit noktaları ve algoritmanın
  sınırıyla; bağımsız ölçüm yok. NURBS'ın sapması örneklenmiştir, kanıtlanmış değil (`sampled`). Çok parçalı geometride
  karışık Z (bir parçada var, ötekinde yok) yalnız parsel sahnesinin deliğiyle denendi. Masaüstünün dört testi bu maddeden önce de
  kırıktı ve bu madde onlara dokunmadı: `karsilastir` takma adı iki komutta (`data.compare`, `view.swipe`), `exportCsv` ikonunun
  web envanterinde de olması, İşlemler'in araç sayısı (151, 148 beklenir), CBS Veri sekmesinin tasarım genişliğinde sığması (4349 px,
  3800 beklenir; bu madde şerit panelini genişletmesin diye iki komutu ▾ listesine koydu).
