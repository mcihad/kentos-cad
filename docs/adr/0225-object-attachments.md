# ADR 0225: Nesne ekleri, eylemler ve bilgi kartı

- **Durum:** kabul edildi (2026-10-11). Kapsamı ben belirledim (sahibin sözü: “Benim seçmeme gerek yok sen sıradan devam et”). İki
  platform (masaüstü ve web) ve bulut; `.kcad` şema 40; ikonlar sorulmadan seçilir; ilke “Performance First”. Madde tek parçada biter.
- **Bağlam belgesi:** TODOS.md `GIS-29` (“nesneye fotoğraf ve belge ekleme, Öznitelikler'de ve üzerine gelme kartında gösterme, konumlu
  fotoğraflardan nokta katmanı, nesneden adres ya da belge açan eylemler, bilgi kartı (balon) şablonu”); ADR 0192 (resim nesnesi: gömülü
  ve bağlı resim, kitaplığın resim varlığı), ADR 0068 (üzerine gelme kartı), ADR 0169 §6 ve 0222 (GNSS içe aktar, WGS 84'ten projenin
  sistemine açık dönüşüm), ADR 0100 ve 0214 (ifade dili), ADR 0177 §5 (Kullanılmayanları temizle), ADR 0212 §2 (nesnenin etiket iğneleri:
  nesne başına alan, sunucuda sütun). Projenin ekleri (rapor, pafta, kaynak dosya) `CLOUD-06`'dadır, burada değil.
- **Araştırma:** ArcGIS Pro'nun ekleri (attachments: ilişkili tabloda fotoğraf ve belgeler, Pop-up'ta gösterim), GeoTagged Photos To
  Points (EXIF'in GPS'i, çekim zamanı ve yönü; fotoğrafı ek ya da yol olarak), Pop-ups (başlık, alan listesi, metin, ekler; `{alan}`
  yer tutucuları), Actions'ın URL'leri; QGIS'in External resource düzenleyicisi (alanda dosya yolu, önizleme), Import geotagged photos
  (`photo`, `filename`, `altitude`, `direction`, `timestamp`), Actions (URL, dosya aç; `[% ifade %]`), Map Tips (HTML ve ifade);
  Netcad'in Referans Özellikleri › Balon İçeriği; EXIF 2.32 (CIPA DC-008): APP1'in TIFF başlığı, IFD0, Exif IFD (`DateTimeOriginal`,
  `OffsetTimeOriginal`), GPS IFD (`GPSLatitudeRef`/`GPSLatitude`, `GPSLongitudeRef`/`GPSLongitude`, `GPSAltitudeRef`/`GPSAltitude`,
  `GPSImgDirectionRef`/`GPSImgDirection`, `GPSDateStamp`, `GPSTimeStamp`); RFC 3986 (yüzde kodlama).

## Bağlam

Saha işinde nesne kendi belgesiyle birlikte anlam kazanır: parselin tapu fotoğrafı, direğin bakım kaydının PDF'i, rögarın içinin
fotoğrafı, yolun kamulaştırma dosyası. KentOS'ta bunlar yalnız öznitelikteki bir metin olabiliyordu. Sahada çekilen konumlu fotoğraflar
elle nokta olarak çizilip adları öznitelik yazılıyordu. Bir parseli kurumun sorgu sayfasında ya da haritada açmak için adres elle
yazılıyordu. Tıklanan nesnenin özeti, Öznitelikler'deki bütün satırlar ve geometri değerleri arasında aranıyordu; Netcad'in balonu
ve ArcGIS'in pop-up'ı gibi katmana göre düzenlenen bir kart yoktu.

## Karar

### 1. Ekler

- **Nesne başına:** her nesnenin `attachments`'ı (`EntityBase`), en çok 64 ek, sırasıyla. Ek: `name` (gösterilen ad; 1–256 harf, satır
  sonu ve denetim karakteri yok), kaynağı yalnız biri: `asset` (**gömülü** fotoğraf: projenin kitaplığındaki PNG ya da JPEG varlığının
  kimliği, resim nesnesininki gibi, ADR 0192 §2), `file` (**bağlı** dosya: yolu; göreliyse çizim dosyasının klasörüne göre) ya da `url`
  (**adres**: `http` ya da `https`); isteğe bağlı `note` (en çok 1 000 harf). Yol ve adres en çok 4 096 harf.
- **Türü kaynağından:** gömülü ek fotoğraftır; bağlı dosya ve adres uzantısı `.jpg`, `.jpeg` ya da `.png` ise fotoğraf, değilse belge
  (dosya) ya da bağlantı (adres). Belgeler gömülmez: projenin belgeleri `CLOUD-06`'nın işi; bağlı dosya ya da adres yeter.
- **Gömülü fotoğraf** kitaplığın varlığıdır (`kind: 'asset'`, kimliği içeriğinin SHA-256'sının ilk 16 onaltılık hanesiyle `resim-…`,
  `path` `Ekler`): aynı fotoğraf bir kez saklanır, resim nesnesiyle de paylaşılır. Kullanılmayanları temizle ekin varlığını kullanılıyor
  sayar. 32 MB'tan büyük dosya gömülmez.
- **Blok tanımının nesnelerinde ek yoktur** (tek tek seçilmezler; etiket iğnelerinin kuralı, ADR 0212 §2).
- **Düzenlemeler:** nesnenin yerine parça yazan düzenlemeler (`cad.entities.edit`'in `replace` ve `add`'i) ekleri öznitelikler ve
  etiketle birlikte taşır: `keepData` açıkken (öteleme kopyası, kırılan parça, patlatma) ekler de gelir, kapalıyken gelmez.
- **Kodek:** `.kcad` şema 40; sütunlarda nesnenin ortak alanlarından sonra; `FORMATS_VERSION` 52. Sunucuda `feature.attachments` (jsonb,
  göç 0016), kopyalama da alır.

### 2. Eklerin gösterimi

- **Öznitelikler:** tek nesne seçiliyken **Ekler** bölümü: fotoğraflar küçük resimleriyle, belgeler ve bağlantılar adıyla; satırda Aç
  ve Kaldır, bağlı fotoğrafta Göm; bölümün altında **Dosya ekle…** (fotoğraf ya da belge; masaüstünde birden çok dosya) ve **Adres
  ekle…**. Satırın menüsünde Görüntüle ya da Aç, Düzenle… (ad ve not), bağlı fotoğrafta Göm ve Kaldır; not satırın altında. Kilitli
  katmanın nesnesinde yalnız açmak vardır. Web dosyanın yolunu bilemez: Dosya ekle… fotoğrafları gömer, belgeler adresle eklenir;
  masaüstü seçilen dosyaları bağlar (çizimin klasörünün altındaysa göreli yolla, değilse bütün yoluyla) ve gömmeyi Göm'e bırakır.
- **Fotoğraf görüntüleyici:** fotoğrafa tıklamak onu büyük gösterir: adı, notu (değiştirilir), sırası (“2/5”), Önceki ve Sonraki (←, →),
  Dışarıda aç (sistemin görüntüleyicisi ya da yeni sekme). Belge ve bağlantı Aç ile dışarıda açılır (§4'ün kuralları).
- **Üzerine gelme kartı:** eki olan nesnede satırların sonunda “Ekler: n ek” ve altında ilk fotoğrafın küçük resmi (kutuyu doldurur).
- Bağlı dosya bulunamazsa ya da web'de (tarayıcı yolu okuyamaz) fotoğrafın yerinde gri çerçeve ve “bulunamadı” ya da “tarayıcıda
  okunamaz” yazar (ADR 0192 §2'nin kuralı).

### 3. Konumlu fotoğraflar

- **Dosya › İçe aktar › Konumlu fotoğraflar** (`file.import.photos`): JPEG fotoğraflar seçilir; her birinin EXIF'i kendi okuyucumuzla
  (`formats::exif`) okunur: enlem ve boylam (yönleriyle, rasyonellerden kesin değer bir kez float'a), yükseklik (deniz düzeyinin altı
  eksi), resmin yönü (gerçek ya da manyetik kuzey), çekim zamanı (`DateTimeOriginal` ve varsa `OffsetTimeOriginal`; yoksa GPS'in UTC
  tarihi ve saati), kamera (Make, Model).
- Konum WGS 84'ten projenin sistemine Koordinat dönüştür'ün yoluyla çevrilir (ADR 0167, projenin datum seçimleri; doğruluk söylenir);
  sistemi olmayan proje fotoğraf almaz. GPS'i olmayan, bozuk ya da sistemin ulaşmadığı fotoğraf nedeniyle söylenir.
- Pencerede fotoğraflar tabloda (Ad, Tarih, Y, X, Yön, durum); **Katman** (yeni “Fotoğraflar” ya da çizimin bir katmanı) ve **Gömülü**
  (açıkken fotoğraflar kitaplığa gömülür; masaüstünde kapalı başlar ve fotoğraf dosyasına bağlanır, web'de hep gömülü). Fotoğrafı resim
  olarak çözülemeyen dosyanın konumu alınmaz, nedeni söylenir. Her fotoğraf bir nokta: öznitelikleri
  `Ad` (dosyanın adı), `Tarih` (ISO 8601), `Yön` (derece; manyetikse “M” ekiyle), `Yükseklik` (m), `Kamera`; eki fotoğrafın kendisi.
  Tek adım “Konumlu fotoğraflar”.
- EXIF'in yüksekliği deniz düzeyine göredir (dayanağı söylenmez); kot olarak değil öznitelik olarak yazılır (ADR 0169'un kuralı:
  dayanağı belli olmayan yükseklik kot sayılmaz).

### 4. Eylemler

- **Katman başına** `actions`: en çok 32 eylem, her biri `name` ve `target` şablonu (§6): `http`, `https` ya da `mailto` adresi, ya da
  dosya yolu (göreliyse çizim dosyasının klasörüne göre). Örnek: `https://www.google.com/maps?q={$enlem},{$boylam}`.
- Şablonun ilk düz metni (baştaki boşluklar atlanarak) `http://`, `https://` ya da `mailto:` ile başlıyorsa şablon adrestir ve her yer
  tutucunun değeri yüzde kodlanır (RFC 3986: harf, rakam ve `-_.~` dışındaki her bayt UTF-8'iyle); değilse değerler olduğu gibi konur.
  Doldurulan hedef `http`, `https` ya da `mailto` adresi değilse ve sürücü harfli ya da düz bir dosya yolu da değilse açılmaz
  (`javascript:`, `data:`, `file:`); bu, söylenir.
- **Nereden:** nesnenin sağ tık menüsünde **Eylemler ▸** (katmanının eylemleri varken), Öznitelikler'in **Eylemler** satırı ve bilgi
  kartı. Masaüstü adresi ve dosyayı sistemin açıcısıyla açar; dosya ancak belge ya da resim uzantılıysa açılır (`.pdf`, `.txt`, `.csv`,
  `.jpg`, `.jpeg`, `.png`, `.tif`, `.tiff`, `.doc`, `.docx`, `.xls`, `.xlsx`, `.odt`, `.ods`, `.dwg`, `.dxf`), çalıştırılabilir dosya
  açılmaz. Web adresi yeni sekmede açar; dosya yolu tarayıcıda açılamaz ve söylenir.

### 5. Bilgi kartı

- **Katman başına** `card`: `title` şablonu, `fields` (gösterilecek öznitelikler sırasıyla; boşsa hepsi), `text` şablonu (satırlar),
  `attachments` (ekleri göster; varsayılan açık), `actions` (eylemleri göster; varsayılan açık). Kartı olmayan katmanın nesnesinin kartı
  türü ve etiketiyle başlar, bütün öznitelikleri ve ekleri gösterir.
- **Bilgi** aracı (`tool.identify`; Netcad'in balonu, ArcGIS'in pop-up'ı): tıklanan nesnenin (tıklamanın adaylarından en üstteki,
  ADR 0187) kartı tıklanan yerin yanında, çizim alanının içinde açılır; başka nesneye tıklamak kartı ona taşır, boşluğa tıklamak ve Esc
  kapatır. Kartta başlık, alanlar (ad: değer), metin, ekler (fotoğrafa tıklamak görüntüleyiciyi açar) ve eylem düğmeleri.
- **Katman › Bilgi kartı ve eylemler…** (`layer.info`): kartın ve eylemlerin penceresi; seçili nesneyle (yoksa katmanın ilk nesnesiyle)
  canlı önizleme; şablonun hatası yerinde söylenir. Komutu `cad.layers.info` (tek adım “Bilgi kartı”).

### 6. Şablonlar

- Şablon düz metindir; `{…}` arası bir **ifadedir** (ADR 0100'ün dili: `{Ada}`, `{[Tapu alanı]}`, `{$alan}`, `{yuvarla($alan, 2)}`,
  `{@proje_adı}`), nesne üstünde değerlendirilir ve metin olarak yerine konur (boş değer boş metin). `{{` ve `}}` düz süslü ayraçtır;
  ifadenin içindeki tırnaklı metinde süslü ayraç sayılmaz. Kapanmayan ya da boş `{}` ve derlenmeyen ifade şablonu reddeder; ileti yerini
  söyler. Başka katmana bakan ifadeler (ADR 0214) şablonda yoktur.
- Eylem ve kart için iki yerleşik yer tutucu: tek başına `{$enlem}` ve `{$boylam}` (nesnenin etiket noktasının, yani etiketinin durduğu
  yerin: alanın ağırlık merkezi, çok parçalının en büyük parçasınınki, çizginin ortası, noktanın kendisi; WGS 84'te enlemi ve boylamı,
  Koordinat dönüştür'ün yoluyla, ondalık derece, sekiz basamak; projenin sistemi yoksa ya da sistem ulaşmıyorsa boş). İfade dilinin
  değişkeni değildirler, ifadenin içinde kullanılmazlar.
- Şablonun bölünmesi ve doldurulması (yüzde kodlama dahil) çekirdektedir (`kentos_expression::template`); ifadeleri her platformun
  kendi değerlendiricisi nesne üstünde değerlendirir.

### 7. Performans

Ekler nesnenin alanında taşınır; küçük resimler gerektikçe ve bir kez çözülür (kitaplığın varlığının ve dosyanın anahtarıyla), çizimi
yavaşlatmaz. Masaüstünde fotoğraf arayüz iş parçacığının dışında çözülür, uzun kenarı en çok 2048 piksele indirilip mip katmanlarıyla
kabuğun kendi wgpu boru hattında çizilir: küçük resim de görüntüleyici de aynı dokudan, yumuşak. Derlenen şablonlar metinleriyle
saklanır; kart ve eylemler her karede yeniden derlenmez. Bütçeler (release, bu makine): EXIF'in okunması 1 000 fotoğrafın başlığında ≤ 50 ms; 10 000 nesnenin kart metni (dört
yer tutuculu şablon) ≤ 200 ms; bir kartın açılması ≤ 16 ms; web'de ≤ 2,5 katı.

## Uygulama

- **Sözleşme:** `kentos_contracts::attachment` (`Attachment`, `AttachmentKind`, `attachments_problem`, `PhotoExif`) ve `layer_info`
  (`LayerAction`, `InfoCard`, `actions_problem`, `infos_problem`); `EntityBase.attachments`, `LayerNode.actions` ve `.card`;
  `cad.entities.set`'in `attachments` işlemi (`invalid_attachments`, `unknown_asset`; adım “Ekler”); `cad.layers.info` v1
  (`LayersInfo`; `invalid_actions`, `invalid_card`, `invalid_template`, `not_a_layer`, `service_layer`; adım “Bilgi kartı”).
- **Kodek:** `.kcad` şema 40 (`FORMATS_VERSION` 52): nesnenin `attachments`'ı ve katmanın `actions` ve `card`'ı yalnız varken; sütunlarda
  `ATTACHMENTS` işareti ve ek başına bitler (`ATT_ASSET`, `ATT_FILE`, `ATT_URL`, `ATT_NOTE`); bağımsız Python okuyucusu ve yazıcısı
  (`tools/kcad/kcad.py`, `scripts/fixtures/kcad_v2_reference.py`), örnek `fixtures/kcad/v2/attachments.kcad` ve 24 bozuk dosya.
- **Çekirdek:** `kentos_expression::template` (bölme, doldurma, yüzde kodlama, `target_kind`; web'e `templateParse`,
  `templateFillMany`, `templateTarget`); `kentos_formats::exif` (web'e `readExif`, `exifNeedsMore`); `kentos_interaction::photos`
  (konumlu fotoğrafların planı ve noktaları), `kentos_interaction::identify` (Bilgi aracı, `ViewChange::InfoCard`);
  `kentos_native_application::info_templates` ve `layers_info`; belgede `Op::LayerInfo` ve `set_layer_info`; Kullanılmayanları
  temizle ekin varlığını sayar (`layer_purge`).
- **Masaüstü:** `attachments.rs` (görünümler, yazmalar, Adres ekle ve Eki düzenle, fotoğraf görüntüleyici), `photo_view.rs` (çözme,
  mip zinciri, `Photos` deposu, wgpu gölgelendirici bileşeni), `object_info.rs` (kartın ve eylemlerin doldurulması, açma kuralları),
  `info_card.rs` (Bilgi'nin kartı), `layer_info.rs` (Bilgi kartı ve eylemler), `exchange/photo_import.rs` (Konumlu fotoğraflar),
  `properties/rows/attachments.rs` (Öznitelikler'in Ekler ve Eylemler'i, satırın resmi), üzerine gelme kartının satırı ve resmi,
  çizim alanının sağ tıkında Eylemler ▸, Katmanlar'ın sağ tıkında Bilgi kartı ve eylemler….
- **Web:** `app/attachments.ts`, `app/objectActions.ts`, `app/objectInfoCommands.ts`, `tools/identifyTool.ts`, `model/infoTemplates.ts`,
  `model/photoImport.ts`, `model/attachmentRules.ts`, `model/layerInfoRules.ts`, `product/layersInfo.ts`; `ui/attachments/`
  (`AttachmentDialog.ts`, `PhotoViewer.ts`), `ui/shell/InfoCard.ts`, `ui/layers/LayerInfoDialog.ts`, `ui/io/PhotoImportDialog.ts`,
  `ui/properties/attachmentRows.ts`; şeritte CAD'in Ekle'sinde ve CBS'nin Veri'sinde Ekler paneli.
- **Sunucu:** göç 0016 (`feature.attachments`, 1–64 ek; kopyalama da alır), `project.changes` ve içe aktarma ekleri yazar ve okur, katman
  ağacının kuralı `infos_problem`'i denetler. **Python** `kentos.cad.layers.info` ve `entities.set`'in `attachments`'ı; **MCP**'de araç.

## Doğrulama

- **Bağımsız başvurular:** `scripts/fixtures/template_cases.py` (şablonun bölünmesi, doldurulması, yüzde kodlaması ve hedefin türü;
  `fixtures/expression/v2/template.json`), `exif_cases.py` (EXIF'i KentOS kodu olmadan yazılmış JPEG ve TIFF başlıkları ve kesirlerle;
  `fixtures/exif/v1`), `photo_import_cases.py` (konumlu fotoğrafların noktaları PROJ'la; `fixtures/photos/v1/import.json`),
  `layer_info_command_cases.py` (`cad.layers.info`), `set_command_cases.py` ve `edit_command_cases.py`'nin ek durumları,
  `layer_purge_cases.py`'nin ek varlıkları, `kcad_v2_reference.py`'nin şema 40 örnekleri; iki platform aynı durumları geçer.
- **Ortak iz:** `fixtures/interaction/v1/identify.json` (Bilgi'nin kartı: başlık, satırlar, metin, eylemlerin adları ve PROJ'la
  hesaplanmış yeriyle doldurulmuş hedefleri, ek sayısı; boşluk ve Esc), yeni beklenti `infoCard`; sahne
  `scripts/fixtures/attachments_scene.py` (`attachments.kcad`, gömülü PNG fotoğraflar küçük bir tarama çizicisiyle, bağlı tapu PDF'i).
- **Masaüstü testleri:** `attachments_tests` (görünümler, Adres ekle, Düzenle, Kaldır ve geri alma, tek nesne ve kilitli katman, Dosya
  ekle'nin göreli bağı ve Göm, kartın ve eylemlerin doldurulması, açılmayan hedef ve dosya, Bilgi kartı ve eylemler penceresi, Eylemler ▸,
  Konumlu fotoğraflar ve Gömülü), `photo_view`'un mip boyları (GPU'nun `max(1, n >> i)`'si).
- **Süreler** (release, bu makine; bütçeler §7): EXIF 1 000 fotoğrafın başlığı 3,0 ms (web 5,4 ms); 10 000 nesnenin kart metni (dört yer
  tutucu) 4,7 ms (web 11,4 ms); bir kartın açılması 0,006 ms (web 0,064 ms). `cargo test --release -p kentos-formats --test all
  exif::timing`, `-p kentos-native-application --test all info_templates`, `-p kentos-desktop a_card_opens_in_a_frame` (hepsi
  `-- --ignored --nocapture`); web'inkiler `scripts/perf/attachments.test.ts`.
- **Resimler:** masaüstünde `tools_screens`'in `ekler-*`'ı (`KENTOS_SNAPSHOT_BACKEND=wgpu`: fotoğraflar kabuğun wgpu boru hattında),
  web'de `shots.mjs attachments`; iki temada 1440×900 ve 1100×650.
- **Atlananlar:** veritabanı isteyen sunucu testleri bu ortamda `.env.local` olmadığı için çalışmadı (`attachments_come_back_and_travel_with_a_copy`
  dahil); geçmiş sayılmaz.

## Doğrulama

(Madde bitince yazılır.)
