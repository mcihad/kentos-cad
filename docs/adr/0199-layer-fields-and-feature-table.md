# ADR 0199: Katman alanları, öznitelik tablosu ve veri kaynakları

- **Durum:** kabul edildi (2026-10-07). Sıra sahibin seçimidir (TODOS.md §16.2, `GIS-01`, 7 Ekim: “GIS-01”). Kapsam sahibin üç kararıdır
  (7 Ekim): kaynak kataloğu klasörler ve KentOS projeleriyle (dış PostGIS, WMS ve WFS `GIS-09`, `GIS-10`'da); değerler bugünkü gibi metin,
  katmanın şeması türü ve kuralları verir; form şemadan kendiliğinden kurulur (sürükle-bırak form tasarımcısı sonra). Madde tek parçada
  biter (sahibin 5 Ekim kuralı).
- **Bağlam belgesi:** TODOS.md `GIS-01` (ve araştırma notu), `DOM-09`–`DOM-11` (tipli şemanın bütünü), ADR 0163 §4 (katmanın kendi
  keneti: `LayerNode`'a alan ekleme yolu), ADR 0153 (Nokta editörünün tablosu), ADR 0066 (Öznitelikler ve `cad.entities.set`), ADR 0046
  (GeoJSON ve Shapefile), ADR 0193 (çizimler arası alışveriş).

## Bağlam

Nesnelerin öznitelikleri anahtar–metin çiftleridir; türleri, kuralları, varsayılanları yoktur. Bir katmanın bütün nesnelerinin
özniteliklerini birlikte görmek, sıralamak, süzmek ve düzenlemek için tablo yoktur; Öznitelikler paneli her değeri düz metin kutusunda
gösterir. Veriyi katman olarak eklemek için her biçimin kendi içe aktarma komutu vardır; sık kullanılan klasörleri ve KentOS'taki öbür
projelerin katmanlarını tek yerden görmenin yolu yoktur.

## Karar

### 1. Katmanın alanları

- Katmanın (grubun değil) isteğe bağlı **alanları** vardır (`LayerNode.fields`; yoksa şema yok, bugünkü gibi). Bir alan:
  - **Ad** (`name`): özniteliğin anahtarı; boş değil, başında ve sonunda boşluk yok, en çok 64 karakter, denetim karakteri yok; bir
    katmanda büyük küçük harf ve Türkçe harfler katlanarak (ADR 0178'in katlaması) bir kez.
  - **Takma ad** (`alias`, isteğe bağlı): tabloda ve formda gösterilen ad.
  - **Tür** (`kind`): Metin (`text`), Tam sayı (`integer`), Ondalık sayı (`decimal`), Tarih (`date`), Evet/hayır (`boolean`).
  - Metinde **Uzunluk** (`length`, 1–10 000 karakter), ondalıkta **Ondalık basamak** (`scale`, 0–15), sayılarda **Aralık** (`min`,
    `max`; ikisi de verilirse `min` ≤ `max`), metinde ve sayılarda **Değer listesi** (`values`: kod ve etiket çiftleri; kodlar türün
    tek biçiminde ve bir kez, etiketler boş değil ve katlanarak bir kez).
  - **Zorunlu** (`required`) ve **Varsayılan** (`default`, alanın kurallarına uyan tek biçim).
- Değerler metin olarak kalır. Bir değerin **tek biçimi** (canonical) türüne göre:
  - Metin: yazıldığı gibi; uzunluğu karakter sayısıyla.
  - Tam sayı: işaret ve rakamlar (çevresindeki boşluk atılır); `+` yazılmaz, baştaki sıfırlar atılır, `-0` `0`'dır; |n| ≤ 2⁵³ − 1.
  - Ondalık sayı: işaret, rakamlar, en çok bir ayırıcı (`.` ya da `,`), rakamlar; üs yok, en az bir rakam, en çok 30 rakam; tek
    biçimde ayırıcı `.`, `+` yazılmaz, tam kısmın baştaki sıfırları atılır (en az `0` kalır), kesir kısmı yazıldığı gibi (sondaki
    sıfırlar kalır: ölçünün hassasiyeti), kesiri olmayan ayırıcı atılır, bütün rakamları sıfır olan eksi işaretsizdir. Ondalık basamaktan
    çok kesir rakamı reddedilir; yuvarlanmaz.
  - Tarih: `YYYY-AA-GG` ya da `GG.AA.YYYY` (gün ve ay bir ya da iki rakam); takvimde var olan gün, yıl 1–9999; tek biçim `YYYY-AA-GG`.
    Gösterimi `GG.AA.YYYY`.
  - Evet/hayır: `evet`, `hayır`, `true`, `false`, `1`, `0` (katlanarak); tek biçim `true` ya da `false`, gösterimi Evet ya da Hayır.
  - Değer listesi olan alanda değer, türün tek biçiminde bir koddur; bir etiket yazılırsa (katlanarak eşleşen) onun kodu yazılır.
  - Aralık sayılarda kesin ondalık karşılaştırmasıyla, uçlar dahil.
- Boş değer (öznitelik yok ya da boş metin) zorunlu olmayan alanda geçerlidir. Var olan, kurallara uymayan değer silinmez ve
  değiştirilmez; tabloda ve formda uyarı rengiyle, nedeniyle gösterilir.

### 2. Komutlardaki denetim

- `cad.entities.set` ve `cad.entities.create`'in yazdığı öznitelikler, nesnenin katmanının alanlarıyla denetlenir: alanın anahtarına
  yazılan değer tek biçimine çevrilip yazılır; uymayan `invalid_attribute` ile, zorunlu alana boş yazmak `attribute_required` ile
  reddedilir (bütün yazma ya da hiç; yol `…attrs.<ad>`). Şemada olmayan anahtarlar bugünkü gibi serbesttir.
- `cad.entities.create` yeni nesnenin vermediği alanları katmanın varsayılanlarıyla doldurur. Zorunlu olup değeri ve varsayılanı
  olmayan alan yeni nesnede reddedilmez (çizim araçları alan bilmez); tablo ve form eksik diye gösterir.
- `cad.entities.edit` öznitelik yazmaz, taşır (öteleme kopyası, kırılan parça, patlatılan blok nesnesi kendi verisiyle): değerler
  denetlenmez, olduğu gibi kalır; uymayan tabloda ve formda gösterilir. Patlatmak ya da kırmak katmanın şeması yüzünden reddedilmez.
- Kurallar sözleşmededir (`kentos_contracts::fields`); web'in komutları aynı kuralları `model/layerFields.ts`'te uygular; iki taraf
  bağımsız başvurunun durumlarını (`scripts/fixtures/layer_field_cases.py`, `fixtures/layer-fields/v1`) ve ortak komut durumlarını geçer.

### 3. Alanlar penceresi

- Katmanlar panelinde katmanın sağ tık menüsünde ve öznitelik tablosunun başlığında **Alanlar…**. Tablo: Ad, Takma ad, Tür, Uzunluk ya
  da Ondalık basamak, Zorunlu, Varsayılan, En az, En çok, Değer listesi (satırın Liste… düğmesi kod ve etiket tablosunu açar); Alan
  ekle, Sil, Yukarı, Aşağı; **Verilerden al**: katmanın nesnelerindeki her anahtar bir alan olur, türü bütün dolu değerlerinin uyduğu
  ilk tür (Tam sayı, Ondalık sayı, Tarih, Evet/hayır sırasıyla, hiçbiri değilse Metin; ondalıkta basamak en çok kesir rakamıdır),
  anahtarlar doğal sırayla. Satırın hatası satırda söylenir; hatalıyken Kaydet kapalıdır.
- Kaydet tek geri alma adımıdır (“Alanlar”): şema, yeniden adlandırılan alanların anahtarları katmanın nesnelerinde (değerler korunur,
  aynı adda başka anahtar varsa reddedilir). Silinen alanın değerleri silinmez, şemasız öznitelik olarak kalır. Yeni türe uymayan
  değerlerin sayısı söylenir.

### 4. Öznitelik tablosu

- Alt panelin **Tablo** sekmesi (Veri › Öznitelik tablosu, CAD'de Yönet'te; `data.featureTable`): üstte Katman ▾ (açılışta seçili ilk
  nesnenin katmanı, yoksa etkin katman), Ara (`*` jokerli, katlanarak; ADR 0178'in eşleşmesi), Göster: Tümü, Seçililer, Görünümdekiler;
  İfade süzgeci (ε ile İfade oluşturucu; bugünkü metin öznitelikleriyle), sayaç “n / m”, Seçime yakınlaş, Alanlar….
- Satırlar katmanın nesneleri, çizim sırasıyla; sütunlar Sıra, Tür, şemanın alanları (takma adlarıyla, sırasıyla), sonra şemada olmayan
  anahtarlar (doğal sırayla). Çok satır sanal listeyle.
- Başlığa tık sıralar (artan, azalan, çizim sırası); türe göre: sayılar kesin ondalıkla, tarihler takvimle, evet/hayır, metin doğal
  sırayla; boş ve kurala uymayan değerler her yönde sonda; eşitler çizim sırasıyla. Sıralama, arama ve gösterilen metin çekirdektedir
  (`ops::feature_table`), bağımsız başvurusu `scripts/fixtures/feature_table_cases.py`.
- Satıra tık nesneyi seçer (Ctrl, Shift); çizimin seçimi satırlarda görünür ve ilk seçili satır görünür yere gelir; satıra çift tık
  yakınlaşır. Hücreye çift tık düzenler: metin ve sayı kutusu, değer listesinde ve evet/hayırda açılır liste (boş, Evet, Hayır); Enter yazar
  (`cad.entities.set`, adım “Değiştir”), Esc vazgeçer, Tab sağdakine geçer; kuralı kırılan değer hücrede söylenir, yazılmaz.

### 5. Form

- Öznitelikler panelinde, nesnenin katmanının şeması varsa Genel ve Geometri'den sonra **Alanlar** bölümü alanları (takma adlarıyla,
  zorunlular `*`'lı, sırasıyla) türüne göre düzenleyiciyle gösterir (değer listesi ve evet/hayır açılır liste, tarih ve sayı kutusu;
  uymayan değer uyarı rengiyle ve nedeniyle); öbür anahtarlar bugünkü bölümlerinde. Çoklu seçimde ortak değerler bugünkü kuralla.

### 6. İçe ve dışa aktarma

- Shapefile'ın DBF alan türleri katmanın alanları olur: C metin (uzunluğuyla), N ondalıksız tam sayı, N ondalıklı ve F ondalık sayı
  (basamağıyla), D tarih (`YYYYAAGG` tek biçime), L evet/hayır (`T`, `Y` evet; `F`, `N` hayır); değerler tek biçimlerinde.
- GeoJSON'ın özelliklerinden alanlar: bütün dolu değerleri tam sayı JSON sayısıysa tam sayı, sayıysa ondalık sayı, mantıksalsa
  evet/hayır, metinse metin; karışıksa metin. GeoJSON'a verirken şemalı katmanın sayıları JSON sayısı (tek biçimi tam olarak okunabildiği
  sürece; okunamayan metin olarak), evet/hayırı mantıksal yazılır.
- `.kcad` şema 26 alanları taşır (kodek, belirtim, bağımsız Python okuyucusu ve yazıcısı, örnekler). Sunucu ağacı denetlerken alanları da
  denetler; veritabanı projesi alanları katman ağacının JSON'unda taşır (göç yok).

### 7. Veri kaynakları

- Sağ dokta **Kaynaklar** paneli (Veri › Tablo ▾ › Veri kaynakları, CAD'de Yönet'te; `data.sources`), Katmanlar, İşlemler, Bloklar ve
  Şablonlar'ın yanında bir sekme. İki bölüm, bir ağaç:
  - **Klasörler**: kullanıcının eklediği klasörler (Klasör ekle; satırın menüsünde Listeden kaldır: klasör ve dosyaları silinmez).
    Liste cihazındır: web'de tarayıcının klasör tutamaçları IndexedDB `kentos.sources/folders`'ta (sayfa yeniden açılınca tarayıcı
    okuma iznini bir kez daha sorar), masaüstünde `$XDG_STATE_HOME/kentos-cad/kaynak-klasorleri.json`'da. Bir klasör açılınca okunur:
    klasörleri ve panelin eklediği dosyaları, noktayla başlayan adlar hariç, doğal sırayla; dosyanın türü uzantısından (büyük küçük
    harf önemsiz): GeoJSON (`.geojson`, `.json`), Shapefile (`.shp`), DXF, Netcad NCZ, GNSS (`.gpx`, `.nmea`, `.nma`), koordinat
    listesi (`.ncn`, `.txt`, `.csv`, `.xyz`, `.dat`, `.asc`); Shapefile `.shp`'si olarak, aynı adlı `.shx`, `.dbf`, `.prj` ve `.cpg`
    parçalarıyla (onlar ayrıca görünmez). Kural iki platformda `fixtures/sources/v1/cases.json`'un durumlarıyla
    (`scripts/fixtures/source_list_cases.py`). Kapatıp açmak yeniden okur.
  - **KentOS**: girişliyken Projelerim ve Benimle paylaşılanlar (adıyla sıralı ilk 200 proje; fazlası söylenir); giriş yoksa “Projeleri
    görmek için giriş yapın” satırı girişi açar. Proje açılınca çizimi indirilir (veritabanı projesinin anlık görüntüsü, dosya projesinin
    son revizyonu; SHA-256'sı denetlenir) ve katman ağacı nesne sayılarıyla listelenir; kapatıp açmak yeniden indirir. Koordinat
    sistemi bu çizimdekinden başkaysa söylenir, koordinatlar dönüştürülmez.
- Dosya ya da katman satırının **Katman olarak ekle**'si her zaman görünen düğmesinde, çift tıkta ve satırın menüsündedir.
  - Dosyada o biçimin içe aktarma penceresi dosyayla (Shapefile parçalarıyla) açılır; pencere koordinat sistemini ve öbür seçenekleri
    her zamanki gibi sorar.
  - Projenin katmanında katman nesneleriyle açık çizime **tek geri alma adımında** (“Katman olarak ekle”) alınır, ADR 0193'ün
    kurallarıyla (`layerTake`, `kentos_domain::exchange::layer_take`; bağımsız başvuru `exchange_cases.py`'nin `layers` durumları):
    katman yoluyla (bu çizimde aynı yolda katman varsa nesneler ona gider, görünüşü değişmez; yoksa grupları ve alanlarıyla açılır),
    nesnelerin bloklarının nesnelerinin katmanları da; yerleştirdikleri bloklar adıyla (aynı adlı blok bu çizimdekidir, öbürleri
    yerleştirdikleriyle gelir); yazı ve ölçü stilleri adıyla (olmayan eklenir); nesnelerin, resimlerin ve açılan katmanların
    görünüşünün çizdiği kitaplık öğeleri (bu çizimde olmayan). Nesneler yeni kimlik alır: bağlı yazının bağı, taramanın ilişkisi ve
    tablonun kaynağı düşer. Bu çizimdeki kilitli katmana alınmaz; `project.edit`'i olmayan veritabanı projesinde ağaç değişmez.
    Stiller ve kitaplık ayar olarak yazılır (geri alma adımı değil; ADR 0193 §2).
- Bir klasörün bütün dosyalarını tek seferde ekleme yoktur: her dosyanın kaynak koordinat sistemi kendi penceresinde sorulmalıdır
  (CLAUDE.md §5).

## Kapsam dışı

Değerlerin tipli saklanması, tarih-saat ve zaman damgası, alan kimliği ile adın ayrılması, benzersizlik, birim, ilişki ve hesaplanan
alan (`DOM-09`–`DOM-11`); ifadelerde tipli kullanıcı alanları (süzgeç bugünkü metin öznitelikleriyle); koşullu hücre biçimi, saklı
süzgeçler, sürükle-bırak form tasarımcısı, katman üst verisi (ISO 19115), alan hesaplayıcı (`GIS-02`); dış PostGIS, WMS, WFS (`GIS-09`,
`GIS-10`); bir klasörün bütün dosyalarını birden ekleme (§7).

## Uygulama

- **Sözleşme:** `kentos_contracts::fields` (`LayerField`, `LayerFieldKind`, `FieldChoice`; `check_value`, `field_problem`,
  `fields_problem`, `layer_fields_problem`, `infer_fields`, `display_value`), `LayerNode.fields`, `ImportLayer.fields` ve
  `GeoJsonLayer.fields` (`FORMATS_VERSION` 36); ret kodları `invalid_attribute`, `attribute_required`. Katalog, TypeScript ve Python
  tipleri yeniden üretildi.
- **`.kcad` şema 26:** kodek (`decode.rs`, `encode.rs`), belirtim, bağımsız Python okuyucusu ve yazıcısı, `layer-fields.kcad` ve on iki
  bozuk örnek.
- **Belge:** geri alma adımı “Alanlar” (`Op::LayerFields`, web'de `layerFields`; yeniden adlandırılan anahtarlar aynı adımda),
  `set_layer_fields` / `setLayerFields`, Verilerden al (`fields_from_data`, `inferFields`); ortak belge durumu `layers.json`'da.
  Sunucu ağacı denetlerken alanları da denetler (`projects.rs`). Kopyasını oluştur alanları da kopyalar, nesnelerden sonra.
- **Komutlar:** `cad.entities.set` ve `cad.entities.create` alanları denetler ve tek biçimi yazar (`checks.rs` `field_values`,
  `product/checks.ts` `fieldValues`).
- **Çekirdek:** `ops::feature_table` (sıralama, arama, gösterilen satırlar), web'e `featureTable`.
- **Masaüstü:** Tablo sekmesi `apps/desktop/src/features/` (`kentos_interaction::feature_table`), Alanlar `layer_fields.rs`,
  Öznitelikler'in alan satırları `properties/rows.rs`, Kaynaklar `sources/` (`kentos_interaction::sources`; katman alma
  `drawing_exchange.rs` `take_layer_into`, `kentos_domain::exchange::layer_take`).
- **Web:** `model/layerFields.ts`, `model/featureTable.ts`, `ui/bottom/FeatureTable.ts`, `ui/layers/LayerFieldsDialog.ts`,
  `ui/properties/fieldRows.ts`, `model/sources.ts`, `app/sources.ts`, `ui/sources/SourcesPanel.ts`, `model/exchange.ts` `layerTake`,
  `app/drawingExchange.ts` `takeLayerInto`.
- **Biçimler:** `formats::fields` (DBF ve GeoJSON türleri, tek biçim), `gis.rs`, GeoJSON yazıcısının `typed`'ı; bağımsız okuyucu
  `tools/formats/gis.py`.
- **İkonlar:** sahibin seçtikleri: Öznitelik tablosu tablo ve parsel, Alanlar… form kutuları, Verilerden al satırlardan başlığa ok,
  Veri kaynakları klasör ve veritabanı.
- **KentOS UI:** yalnız ikonu kalan dok sekmesi 28 px (ikonun iki yanında 7 px; önce 38): beş sekmeli sağ dok varsayılan genişliğe sığar.
- **İz biçimi:** `panel: "Tablo"` adımlarının `edit`'i ve adım düzeyinde `featureTable` beklentisi (nesnenin `table`'ıyla karışmasın);
  masaüstü oynatıcısı tablo hücresinin odağını izler.

## Doğrulama

- Değer kuralları bağımsız başvurudan (`scripts/fixtures/layer_field_cases.py`, `fixtures/layer-fields/v1`) iki platformda; komut
  durumları (`set_command_cases.py`, `create_command_cases.py`) iki işleyicide; belge durumları iki belgede.
- Tablonun sıralaması, araması ve gösterimi `scripts/fixtures/feature_table_cases.py`'den (`fixtures/feature-table/v1/cases.json`)
  çekirdekte yerli ve WASM'la.
- `.kcad` şema 26 Rust kodeğinde, bağımsız Python okuyucusunda ve yazıcısında (`kcad_v2_reference.py --check`); GeoJSON ve Shapefile'ın
  alanları ve türlü dışa aktarım bağımsız okuyucuyla (`gis_reference.py --check`, `export/alanli`).
- Katmanı nesneleriyle alma `exchange_cases.py`'nin `layers` durumlarıyla (bizde olan ve olmayan katman, iç içe ve aynı adlı blok, stiller,
  açılan katmanın görünüşünün simgesi, grup ve olmayan yol) iki platformda; klasör listesi `source_list_cases.py`'den
  (`fixtures/sources/v1/cases.json`) iki platformda.
- Ortak iz `fixtures/interaction/v1/feature-table.json` üç klavye düzeninde iki platformda geçer. Birim testler: masaüstünde
  `sources::tests` (diskten klasör satırları, saklanan liste, girişsiz KentOS, projenin katmanı tek adımda ve geri alınır),
  `layer_merge` (kopya alanlarıyla), `layout_tests` (Kaynaklar sekmesi saklanır); web'de `app/sources.wasm.test.ts`,
  `app/layerActions.test.ts`.
- Resimler 1440×900 ve 1100×650'de, koyu ve açık temada iki platformda: Tablo ve Alanlar (`kentos-cad kullan feature-table`,
  `pnpm -C apps/web e2e:use feature-table`), Öznitelikler'in alanları (`features::tests::screens`, `shots.mjs fields`), Kaynaklar
  (`sources::tests::screens`, `shots.mjs sources`, gerçek sunucu ve geçici veritabanıyla `cloud-shots.mjs`'in `sources-*`'ı).
