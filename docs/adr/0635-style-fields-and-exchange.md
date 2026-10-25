# ADR 0635: Stilin alanlara bağı ve stil alışverişinin genişlemesi

- **Durum:** kabul edildi (2026-10-25). `plan/gis.md` CBS-17: görünüşün sağlayıcının alanlarına bağı (kaynakta yeniden
  adlandırılan sütun, kitaplık simgesindeki alan adı), SLD, QML ve MapLibre alışverişinde etiketler, tematik işleyiciler,
  işlevli ifadeler, QGIS'in öbür sembol katmanları ve MapLibre sprite'ları; alışverişin açık profili; tek “Stil yükle / Stili
  kaydet”; yerel ve bağlı katmanda aynı sınıflar ve lejant. Kapsamı ben belirledim; QML ile MapLibre'nin kendi crate'lerine
  ayrılması sahibin isteği (“bağımsız geliştirebilelim ileride”). Sivas Belediyesi'nin gerçek MapLibre stiliyle sınama da
  sahibin isteği.
- **Bağlam belgesi:** ADR 0516 (SLD), 0555, 0565 ve 0571 (QML ve stil ekleri), 0560 (MapLibre), 0208 (harita servisleri ve
  vektör karolar), 0212 (etiket motoru), 0213 (ek işleyiciler), 0414 ve 0564 (alanın kullanıldığı yerler), 0503, 0505 ve 0581
  (bağlı katman, PostGIS), 0600 (QGIS'le denetlenen işlevler), 0121 (parçalı katman).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:** yakınlaştırmaya göre değişen değerin bütün eğrisi (bugün çizimin ölçeğindeki değeri alınır, §12);
  MapLibre'ye SVG işaretinin sprite'ı (bugün yalnız şekil işaretleri sprite olur); SLD'de tematik işleyiciler (SE'nin karşılığı
  yok); etiketin hale bulanıklığı, satır genişliği, `text-transform` ve yazı tipi (KentOS etiketi projenin yazı tipiyle çizer).

## Bağlam

ADR 0516, 0555 ve 0560 görünüşü üç biçimde iki yönde taşıdı; ama yalnız simgeleri ve dört temel işleyiciyi. Bir GIS'in
stili bundan fazlasıdır: etiketler, ısı haritası ve kümeleme gibi tematik çizimler, alanların işlevlerle okunduğu kurallar,
QGIS'in elips ya da şekil dolgulu işaretleri, MapLibre'nin sprite'tan ikonları. Aynı zamanda görünüş alanlara bağlıdır:
kaynakta bir sütunun adı değişince görünüş, etiket ve formlar sessizce boşa düşüyordu; projenin kitaplığındaki bir simge
alanı eski adıyla anmaya devam ediyordu. Neyin taşındığı da yalnız raporlarda, dağınık söyleniyordu.

## Karar

### 1. Kaynakta yeniden adlandırılan sütun

`cad.sources.sync`'in okuması tablonun sütunlarını katmanın alanlarıyla karşılaştırır (`sources::renamed_columns`): sayılar
aynıysa, aynı yerde katmanda olup tabloda olmayan bir alan ve tabloda olup katmanda olmayan bir sütun aynı türdense bu bir
yeniden adlandırmadır. SQLite'ın ve PostgreSQL'in `RENAME COLUMN`'u sütunun yerini ve türünü korur; araya sütun eklenmiş ya
da çıkarılmışsa yerler kayar ve hiçbir şey yeniden adlandırma sayılmaz (alan eskisi gibi kalır). Yeniden adlandırma aynı
adımda `cad.layers.edit`'in adlandırmasının yolundan gider: görünüş, etiketler, süzgeç, formlar, ilişkiler ve kurallar yeni
adı alır (ADR 0414, 0564). Cevabın `renamed`'i (`from`, `to`) ve `source_columns_renamed` uyarısı söyler. GeoPackage ve PostGIS
aynı yoldadır (`a_column_renamed_in_postgis_is_followed`).

### 2. Kitaplık simgesindeki alan adı

Görünüş projenin kitaplığından bir simgeyi başvuruyla alabilir (`{"ref": …}`); simge katmanlar arasında paylaşılır, bir
katmanın alanı yeniden adlandırılınca değiştirilmez. `field_refs::library_uses` görünüşün başvurduğu simgelerin alanları nerede
andığını bulur; `cad.fields.usage` bunları sayar, `cad.layers.edit`'in yeniden adlandırması `library_symbol_field` uyarısıyla
söyler (`fields_usage::library_warning`; adlandırmadan önce sorulur). Kaynakta yeniden adlandırılan sütunda da aynı uyarı
`cad.sources.sync`'ten gelir, yolu `layer`.

### 3. Etiketler üç biçimde iki yönde

Ortak kurallar `kentos_formats::style_labels`'tadır: tek etiket “Etiket” adlı sınıf olarak yazılır; şablonun parçaları
(`[alan]` ve metin) her biçimin birleştirmesine çevrilir (SLD `Concatenate`, QML `||`, MapLibre `concat` ya da `{alan}`);
ölçek sınırları kâğıdın 0,28 mm pikseliyle (paydası 4, piksel 5 anlamlı basamakla); okunan değer KentOS'un aralığına
sıkıştırılır ve söylenir; en çok 64 sınıf.

- **SLD:** `TextSymbolizer` (yazı tipi, boy, renk, hale, yerleşim; `followLine`, `repeat`, `maxAngleDelta`, `group`,
  `conflictResolution` satıcı seçenekleri). `Graphic` taşınmaz. QGIS 4.2 kuralları okur, `Concatenate`'i okumaz.
- **QML:** `<labeling>` (`simple` ya da `rule-based`), metin ifadesi, yerleşim, ölçek; QGIS 4.2'de `loadNamedStyle` ile
  denetlenir.
- **MapLibre:** `symbol` katmanı, `text-field`, yazı tipi yığını (kalın ve eğik etiketin yüzü), `minzoom` ve `maxzoom`.

İçe aktarma komutları görünüşü ve etiketleri tek kayıtlı adımda yazar (`cad.layers.renderer` ve `cad.layers.labels`). Cevapta
`labels` ve `label`; dışa aktarmada etiket sayısı.

### 4. Tematik işleyiciler

QML'de ısı haritası (`heatmapRenderer`), ters alan (`invertedPolygonRenderer`), kümeleme (`pointCluster`) ve yayma
(`pointDisplacement`) iki yönde; MapLibre'de ısı haritası `heatmap` katmanı olarak iki yönde. Rampa 2–16 renktir: yoksa
“Isı” rampası, tek renk ikilenir, 16'dan fazlasının ilk 16'sı. QGIS 4'ün durakları `:` ile ayrılır, renkler `rgb:` taşıdığı için
ofsetle başlamayan parçalar birleştirilir. Öbür türler (sürekli renk, orantılı sembol, iki değişkenli renk, nokta yoğunluğu,
grafik) üç biçimde de taşınmaz ve adıyla söylenir.

### 5. İşlevli ifadeler

`kentos_formats::style_expr` KentOS ile QGIS'in ifadesi arasında sözcük sözcük çevirir: işleçler ve anahtar sözcükler ortak,
alanın yazılışı (`[ad]` ve `"ad"`) ve 39 işlevin adı (yalnız iki dilde aynı anlama gelenler, ADR 0600'ün QGIS'le denetlediği:
`yuvarla`/`round`, `büyük`/`upper`, `varsayılan`/`coalesce` …). Başka işlev, `$` ya da `@` değeri olan ifade çevrilmez.
QML'in kural süzgeçleri, veriye bağlı değerleri, etiket metni ve süzgeci bu yoldan.

### 6. QGIS'in öbür sembol katmanları

`EllipseMarker` ve `FilledMarker` şekil işareti olarak okunur; `ShapeburstFill` düz dolgu olarak, notla; `ArrowLine` ve
başkaları adıyla kaybolur.

### 7. MapLibre sprite'ları

Daire olmayan şekil işareti (kare, yıldız, artı …) `circle` katmanıyla çizilemez: `symbol` katmanı olarak yazılır, `icon-image`
sprite'ın bir ikonunu adlandırır (`kentos-{şekil}-{fnv32}`), işaretin kendisi `metadata.kentos:marker`'dadır. Başsız sunucu
ikonları çizimin kendi şekil yollarıyla (`kentos_native_style::shapes`) tiny-skia ile `{ad}-sprite.png` ve `.json`'a, 1× ve 2×,
çizer ve stilin `sprite`'ını yazar. Geri okunurken KentOS'un ikonu işaretine döner; başka bir sprite'ın ikonu (yalnız
pikseller) adıyla kaybolur.

### 8. Ayrı crate'ler ve alışverişin açık profili

QML `crates/shared/qml` (`kentos-qml`), MapLibre `crates/shared/maplibre` (`kentos-maplibre`) oldu; SLD ve ortak kurallar
(`style_labels`, `style_expr`, `style_extras`) `kentos-formats`'ta kalır. Her biçim kendi başına geliştirilir ve sınanır.

Neyin taşındığı artık kodla denetlenen bir dosyadır: `catalog/v1/style-exchange.json`. Her işleyici (13) ve her sembol
katmanı (13) için her biçimde **both** (iki yönde), **write** (yazılır, başka türle geri gelir) ya da **lost** (taşınmaz); kısmi
kayıplar `notes`'ta. Her satırın örneği kendi biçiminin testinde yazılıp geri okunur (`kentos_formats::style_exchange::check`;
`sld_does_what_the_profile_says`, `qml_…`, `maplibre_…`): iddia ya da not kodla aynı değilse test düşer. Okunur belge
`docs/style/exchange.md`, `scripts/style/exchange.py` ile üretilir.

### 9. Stil yükle / Stili kaydet

Katmanlar'ın Görünüş ▸'ünde tek dosya: uzantı biçimi seçer (`.qml` QGIS, `.sld` ya da `.xml`, `.json` MapLibre), işi o biçimin
komutu yapar (`layer_style_file.rs`). Biçimlerin kendi öğeleri yerinde kalır.

### 10. Kısmi yeniden kurulum

Kaynakta tek satırın değişmesi tek nesneye dokunur (`a_record_changed_in_its_table_touches_only_its_object`); büyük katmanda
yalnız o nesnenin parçası yeniden kurulur (ADR 0121'in sahne testi).

### 11. Yerel ve bağlı katmanda aynı sınıflar ve lejant

Aynı tablo GeoPackage'a bağlı katman ve kopyası olarak açıldığında Değerlerden sınıfla aynı sınıfları, lejant aynı satırları
verir (`bound_legend.rs`); PostGIS'e bağlı katman aynı tablonun GeoPackage'ından aynı nesneleri okur
(`a_postgis_layer_reads_what_a_file_layer_reads`).

### 12. Gerçek bir belediye stili: Sivas Kent Rehberi

`https://harita.sivas.bel.tr/style/kentrehberi` (Martin'in vektör karoları, 44 stil katmanı) iki yoldan sınandı:

- **Harita servisi:** stil vektör servisi olarak TM36 projesinde (`servis-sivas` sahnesi). Parseller ve binalar (z16'dan)
  çizilmiyordu: vektör kaynağın `maxzoom`'u (22) 14'e kırpılıyordu (`source::tiles`'ın `zoom(14)`'ü verilen değeri de 14 ile
  sınırlıyordu). Artık kaynağın kendi `maxzoom`'u en çok 24'e (`VECTOR_MAX_ZOOM`, MapLibre'nin sınırı) kadar istenir; vermeyen
  kaynakta 14. MapLibre GL JS'in aynı yerdeki çizimiyle karşılaştırıldı.
- **İçe aktarma:** karoların verisi yerel katmanlara (parseller, binalar, sokaklar, mahalleler, parklar) alınıp stil
  `file.import.maplibre` ile uygulandı. Yakınlaştırmaya göre değişen kalınlık ve saydamlık “ilk durağın değeri” ile
  okunuyordu: sokak çizgisinin ilk durağı 0 piksel olduğu için sokaklar görünmüyordu. Artık eğri **çizimin ölçeğindeki**
  değeriyle okunur (`renderer_of_at`, projenin çizim ölçeği; verilmezse 1:1000): duraklar arasında eğrinin kendi kuralıyla
  (doğrusal ya da üslü), durağın değeri bir ifadeyse (Sivas'ın yolun türüne göre `case`'i) sayı sayı. `["case", ["==", ["get",
  f], v], …]` tek alana bakıyorsa onun `match`'idir: yolun türüne göre sınıflar. Bulgu adıyla söylenir. Ortak durum
  `egri-yol.json`.

Sivas'ın stili ve verisi depoya konmadı (kurumun verisi).

## Doğrulama

- Ortak durumlar: `cad.sources.sync.renamed` 21, `cad.fields.usage` 22, `file.import.sld` 30, `.qml` 38, `.maplibre` 36,
  etiketlerin `file.import.sld.labels` 27, `.qml.labels` 34, `.maplibre.labels` 25; hepsi KentOS kodu olmadan yazılmış
  başvurulardan (`source_rename_cases.py`, `field_usage_cases.py`, `sld_cases.py`, `qml_cases.py`, `maplibre_cases.py`,
  `style_labels_cases.py`; ifade çevirisi `style_expr_ref.py`).
- QGIS 4.2.3 (Linux) KentOS'un QML'lerini ve SLD'lerini `loadNamedStyle` ile okur: etiketler, kural süzgeçlerinde işlevler,
  tematik QML'ler; QGIS'in yazdığı 15 dosya da okunur. MapLibre'nin resmî doğrulayıcısı (`@maplibre/maplibre-gl-style-spec`
  26.4.4) KentOS'un bütün stillerini, sprite'lısını da, geçerli bulur.
- Profil: üç biçimin testi 26 satırı yazıp geri okur.
- Başsız sunucu: etiketler üç biçimde, tematik QML'ler, sprite ikonları (`style_labels.rs`); sprite PNG'si resimle denetlendi.
- Sivas: `servis-sivas` sahnesi ve MapLibre GL JS 4.7.1'in aynı yerdeki çizimi; içe aktarmanın görünüşü yerel katmanlarda.
- Süreler (release, bu makine): Sivas stilinin 44 stil katmanının 3146 nesneli beş katmana alınması
  (`file.import.maplibre`, Python'dan beş kez) 0,8–1,1 ms; profilin üç testi milisaniyenin altında.
