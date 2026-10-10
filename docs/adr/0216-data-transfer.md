# ADR 0216: Anahtarla veri aktarma

- **Durum:** kabul edildi (2026-10-10). Kapsamı ben belirledim (sahibin sözü: “Benim seçmeme gerek yok sen sıradan devam et”). İki
  platform (masaüstü ve web); `.kcad` şeması değişmez; ikonlar sorulmadan seçilir; ilke “Performance First”. Madde tek parçada biter.
- **Bağlam belgesi:** TODOS.md `GIS-20`; ADR 0199 (katman alanları: tür, zorunlu, varsayılan, değerin kanonik metni), ADR 0200 §6–§7
  (Anahtarla birleştir: anahtarların kuralı `join_plan`, dosya parametresi, tablo çıktısı, aracın reddi), ADR 0084 (İşlemler'in
  tanımdan üretilen penceresi), ADR 0237 §9 (satırlı parametre alanları: Rasterlere değer).
- **Araştırma:** ArcGIS Pro'nun Append'i (çıktı alanı başına kaynak alanla alan eşleme; Matching Fields For Updates ile eşleşeni
  güncelleme, eşleşmeyeni ekleme; Update Geometry), Join Field; QGIS'in Join attributes by field value ve Refactor fields (alan başına
  kaynak, ad, tür); Netcad'in Veri Aktar'ı (tablodan nesnelere anahtarla).

## Bağlam

Anahtarla birleştir (ADR 0200 §6) kaynağın alanlarını hedefe aynı adlarla (önekle) yazar. Alan adları farklıysa, hedefin alanı
tipliyse, eşleşmeyen kaynak satırları yeni nesne olmalıysa ya da geometri de güncellenecekse yetmez. Kadastro ve belediye işlerinde
tapu listesi, imar durumu tablosu ya da başka bir ekipten gelen güncel katman bir anahtarla (parsel, ada, kimlik) mevcut katmana
aktarılır: alanlar eşlenir, eksik değerler varsayılanla dolar, yeni kayıtlar eklenir ve ne olacağı önce görülür.

## Karar

### 1. Kapsam

İşlemler'in Öznitelik kategorisinde iki platformda yeni araç **Veri aktar** (`attributes.transfer`) ve İşlemler'e yeni parametre türü
**Alan eşleme** (`fieldMap`):

- Hedef bir katman; kaynak bir katman ya da bir dosya (CSV, TXT, XLSX; Anahtarla birleştir'in okuyucusu).
- Ortak anahtarla eşleşen hedef nesnelerin öznitelikleri alan eşlemesine göre güncellenir; isterse geometrisi de (kaynak katmansa).
- Eşleşmeyen kaynak satırları isterse hedef katmana yeni nesne olarak eklenir: katmandan geometrisiyle, dosyadan doğu ve kuzey
  sütunlarıyla nokta.
- Rapor tablosu her zaman; **Yalnız raporla** çizimi değiştirmeden önizler.
- Netcad'in adı “Veri Aktar” bu araca geçer: `VERIAKTAR` takma adı Anahtarla birleştir'den bu araca taşınır.
- **Kapsam dışı:** ifadeyle alan değeri (Refactor fields'ın ifadeleri; Öznitelik hesapla vardır), katmanın alanlarını yeniden
  düzenleme (ad, tür, sıra: Alanlar penceresi, ADR 0199), birden çok anahtar alanı, kaynakta olmayan hedefleri silme, veritabanından
  okuma (`PG-13`).

### 2. Alan eşleme (`fieldMap`)

- Değer satırların listesidir: **Hedef alan** (ad), **Kaynak alan** (kaynağın sütunu ya da yok) ve **Varsayılan** (metin). Boş liste:
  anahtardan başka bütün kaynak sütunları kendi adlarıyla (Anahtarla birleştir gibi).
- Tanımda `target` (hedefin özellik parametresi) ve `source` (kaynağın parametreleri, görünen ilki okunur: katman ya da dosya, ADR
  0200 §6'daki alan parametresi gibi).
- Bir satırın değeri: kaynak sütunun hücresi boş değilse (boşluklar atılınca) hücre; değilse varsayılan boş değilse varsayılan; o da
  yoksa değer yoktur.
- Pencere: satırlar tablo gibi; Hedef alan yazılır ya da hedefin alanlarından seçilir; Kaynak alan kaynağın sütunlarından ya da
  “(yok)”; Varsayılan yazılır; satırın ×'i; **Satır ekle**; **Aynı adlarla doldur**: var olan satırlar olduğu gibi kalır, kaynağın anahtar
  dışındaki, henüz hiçbir satırın kaynağı ya da hedefi olmayan her sütunu için kendi adıyla bir satır eklenir (sütunların sırasıyla; ekleyecek
  sütun yoksa düğme kapalıdır). Hedef adı boşsa, önceki bir satırda da hedefse ya da kaynağın sütunları belliyken kaynak sütunu kaynakta
  yoksa satır uyarı rengiyle çerçevelenir ve nedeni altında söylenir (bu sırayla ilki); parametrenin genel kırmızı çerçevesi sorunsuz
  satırlara geçmez. Çalıştırma ilk iki sorunla ve 64 karakterden uzun ya da köşeli parantezli adla başlamaz (alan parametresinin kuralı:
  “n. satırın hedef alanı boş”, ““X” iki satırda hedef”).
- Kaynağın sütunları bilinmiyorsa (dosya seçilmemiş) sütun listesi boştur ve not söylenir; modelin girdisi olamaz (dosya parametresi
  gibi), modelin adımında eşlemenin kendi değeri kullanılır.

### 3. Hedef katman

- **Hedef katman** katman kapsamlı özellik parametresidir; boş katman kabul edilir (yalnız ekleme). İşlemler'in özellik parametresine
  bunun için iki ek: boş girdiyi reddetmeyen `allowEmpty` ve katman kapsamında seçilen katmanın kimliği (`FeatureSet.layerId`). Böyle bir
  girdi boşken alanın altındaki satır uyarı üçgeniyle değil bilgi simgesiyle söylenir.
- Eklenen nesneler bir katmana yazılır: `allowEmpty` ve yazılan girdi bir grup ya da kilitli katman olamaz; pencerenin denetimi
  çalıştırmayı başlatmaz (“Hedef katman”: grup değil, bir katman seçin; eklenen nesneler bu katmana yazılır. / … katman kilitli. Kilidi
  Katmanlar panelinden açın ya da başka katman seçin.). Pencerenin listesi yalnız katmanları sunar; grup bir modelden, Python'dan ya da
  saklanan değerden gelebilir.

### 4. Değerlerin yazılması

- Anahtarlar Anahtarla birleştir'in kuralıyla eşlenir (`join_plan`: boşluklar atılır, iki taraf da sayıysa sayı olarak, `002` ile `2`
  aynı); kaynakta yinelenen anahtarın ilk satırı alınır; hedefte aynı anahtarlı nesnelerin hepsi güncellenir.
- **Var olan değerler:** Üzerine yaz (değeri olmayan satır alanı boşaltır) ya da Yalnız boşlara (hedefte dolu alana dokunulmaz).
- Hedef katmanın alanı tipliyse değer alanın kuralıyla okunur ve kanonik metniyle yazılır (ADR 0199); uymayan değer yazılmaz, satırın
  öbür değerleri yazılır ve raporun notu alanın kuralının iletisidir (çalıştırma reddedilmez). Zorunlu alan boşaltılmaz.
- **Geometriyi güncelle** (kaynak katmansa): eşleşen hedef kaynağın geometrisini alır (kimliği, katmanı, görünüşü ve öznitelikleri
  kalır); türleri farklıysa geometri güncellenmez ve söylenir.
- **Eşleşmeyenleri ekle:** hiçbir hedefin almadığı ilk satır hedef katmana yeni nesne olur, şu sırayla: anahtar hedef anahtarın
  adıyla, eşlemenin değerleri, sonra katmanın alanlarının varsayılanları; katman kaynağında nesnenin geometrisi, dosya kaynağında Doğu
  ve Kuzey sütunlarından nokta (ondalık sayı kuralıyla, ADR 0199). Anahtarı alanına uymayan, zorunlu alanı boş kalan ya da noktası
  okunamayan satır eklenmez ve nedeni söylenir.
- Hepsi tek geri alma adımı (“Veri aktar”).

### 5. Rapor

Tablo: **Anahtar**, **Sonuç**, **Değişiklikler**, **Not**. Satırlar önce hedeflerin sırasıyla (Güncellendi, Değişmedi, Eşleşmedi),
sonra hiçbir hedefin almadığı kaynak satırları kendi sıralarıyla (Eklendi, Eklenmedi, Kullanılmadı); en çok 1 000 000 satır.

- Bir değişiklik “ad: önceki → sonraki”dir, değer yoksa “(boş)”; geometri değiştiyse “geometri”; eklenen nesnede “ad: değer”
  (anahtar, eşlemenin değerleri, sonra katmanın varsayılanları). Değişiklikler “; ” ile ayrılır.
- Notlar: alanın kuralının iletisi, “Geometrinin türü farklı; geometri güncellenmedi.”, “Anahtarı boş.”, “Anahtar kaynakta
  yineleniyor; ilk satır alındı.”, “Doğu ya da kuzey okunamadı; nokta eklenmedi.”; birden çoksa boşlukla.
- Özet: “n nesne güncellendi, m eklendi; k hedefin eşi yok, s kaynak satırı kullanılmadı.” (eklenemeyenler de kullanılmamış sayılır);
  **Yalnız raporla** açıkken başına “Yalnız rapor, çizim değişmedi: ” gelir, tablo aynıdır.
- İletiler: yinelenen anahtarlar (“n anahtar kaynakta birden çok kez var; ilk satırları alındı.”), yazılmayan değerler (“n değer
  alanına uymadığı için yazılmadı; nedenleri raporda.”), eklenemeyen satırlar (“n kaynak satırı eklenemedi; nedenleri raporda.”).
- Ret: kaynakta anahtar ya da eşlemenin bir sütunu yok (“Kaynakta “X” alanı yok; alanları: …”), dosyadan eklemede Doğu ya da Kuzey
  seçilmemiş (“Dosyadan nesne eklemek için Doğu ve Kuzey sütunlarını seçin.”).

### 6. Arayüz

- Pencere tanımdan (ADR 0084); Alan eşleme'nin tablosu formun içinde, rapor tablosu formun altında (ADR 0200 §5).
- CBS'de Veri sekmesinin Öznitelik panelinde (Öznitelik hesapla ve İfadeyle seç'in yanında) ve Analiz sekmesinin Öznitelik panelinde
  (kategoriden); CAD'de İşlemler'den.
- **İkonlar** (sorulmadan): Veri aktar `dataTransfer`; Alan eşleme'nin Aynı adlarla doldur'u `fieldMapFill`.

### 7. Performans

Eşleme tek geçişte (`join_plan`), tipli alanın kuralı alan başına bir kez bulunur, nesne başına yalnız değişen alanlar yazılır.
Bütçeler (release, bu makine): 10 000 parsel ve 10 000 satırlık tapu listesinde (beş alan eşlenmiş, 100 eklenecek satır) aracın
bütün çalışması masaüstünde ≤ 120 ms, web'de ≤ 300 ms.

## Uygulama

- **Parametre** (`fieldMap`, `allowEmpty`, `FeatureSet.layerId`, `RunContext.fields`): web `processing/types.ts`, `parameters.ts`
  (`fieldMapProblem`, `firstShown`, `fieldMapKey`; `ValidationEnv.layerIsGroup`), `job.ts`, `runner.ts`; masaüstü `kentos_processing`'in
  `types.rs` (`ParamKind::FieldMap`, `ParamDef::allow_empty`, `Patch.geometry`: nesnenin alacağı şekil), `field_map.rs` (satırlar, görünüm,
  düzenlemeler, Aynı adlarla doldur, denetim; metinler web'in `DIALOG_TEXTS.fieldMap`'ine bir testle bağlı), `parameters.rs`
  (`field_map_key`; dosya satırları yerlerinde okunur: `file_rows`, `file_table` kopyalamaz), `runner.ts`/`runner.rs` (boş girdi, girdinin
  katmanı, şeklin yaması; masaüstünde yamanın parçaları nesneye kopyalanmadan taşınır), `web_param.rs`, `model_edit.rs` (modelin girdisi olamaz).
- **Araç:** web `processing/builtin/transfer.ts` ve `joinSource.ts` (kaynağın satırları ve eksik sütun iletisi, Anahtarla birleştir'le
  ortak); masaüstü `crates/native/processing/src/builtin/transfer.rs` ve `join_source.rs`; `VERIAKTAR` takma adı Anahtarla birleştir'den
  geçti.
- **Pencere:** web `ui/processing/fieldPlan.ts` (`fieldMapView`, `filledMap`, `withMapRow`, `withoutMapRow`, `withNewMapRow`;
  `featuresView`'in `count.allowed`'ı), `paramFields.ts`'in `fieldMapField`'ı, `dialogPlan.ts`, `dialogTexts.ts`, `styles/processing.css`;
  masaüstü `apps/desktop/src/processing/fields.rs`'in `field_map`'i ve `window.rs`; CBS'de Veri sekmesinin Öznitelik listesi (web
  `app/ribbon.ts`, masaüstüne envanterin `ribbonByMode`'uyla); `catalog.rs`'in `PORTED`'ı ve `ported.json`; ikonlar `ui/icons.ts`'te
  (`dataTransfer`, `fieldMapFill`; masaüstüne envanterle).
- **Bağımsız başvurular:** `scripts/fixtures/transfer_cases.py` (`fixtures/processing/v1/transfer.kcad`, `transfer.json`, `tapu.csv`,
  `noktalar.csv`: on durum) ve `scripts/fixtures/field_map_cases.py` (`field-map.json`: görünüm, doldurma, denetim ve düzenlemeler, 27 durum).
- **Sınamalar:** web `processing/cases.test.ts` (sayfada ve işçi yolunda; değişen geometri de karşılaştırılır), `ui/processing/fieldMap.test.ts`,
  `dialogFixture.test.ts` (formlar `dialog.json`'da); masaüstü `kentos-processing`'in `tests/cases.rs`'i
  (`the_transfer_cases_do_what_they_say`), `field_map` birim sınamaları, `apps/desktop/src/processing/transfer_tests.rs` (her ortak durum
  pencereden, eşlemenin denetimi alanın altında).
- **Resimler:** masaüstü `processing::transfer_tests::screens` (`islem-aktar-*`), web `shots.mjs transfer` (`aktar-*`); yerleşim denetimi
  `layout.mjs`'in `processing-transfer` ve `processing-transfer-map`'i.
- **Süre ölçümleri:** `cargo test --release -p kentos-processing --test cases transfer_timing -- --ignored --nocapture` (sahne
  `transfer_scene`), web `apps/web/scripts/perf/transfer.test.ts` (aynı sahne değer değer, sayfadaki çalıştırıcıyla).

## Doğrulama

10 Ekim 2026, geliştirme makinesinde (11. nesil Intel Core i5-11300H, 8 iş parçacığı, Linux; derleme 4 işle):

- **Bağımsız başvurular** (KentOS kodu olmadan, `--check` ile geçer): `transfer_cases.py` (`transfer.json`: tapu listesinden tipli
  alanlarla, Yalnız boşlara, yeni ölçümden geometri ve ekleme, Yalnız raporla, nokta dosyasından ekleme, eksik anahtar ve koordinat
  sütunları, boş hedef katman, kilitli katman, grup; anahtarlar ADR 0200'ün başvurusuyla, değerler ADR 0199'unkiyle) ve
  `field_map_cases.py` (`field-map.json`).
- **Platformlar:** on durum web'de sayfada ve işçi yolunda, masaüstünde doğrudan ve çizimin okuma kopyasında aynı sonuçları verir (değişen
  nesnelerin öznitelikleri ve geometrisi, eklenenler, rapor, iletiler, tek geri alma adımı); masaüstünün penceresinden de, dosya seçilerek
  (`transfer_tests`). Alan eşlemenin 27 durumu iki platformda; masaüstünün metinleri web'in kaydıyla aynı.
- **Resimler** iki platformda 1440×900 ve 1100×650'de, iki temada: Veri sekmesi, tapu listesinin eşlemesi, sorunlu satırlar, rapor, yeni
  ölçüm çizimde. Web'de sorunsuz satırın da parametrenin kırmızı çerçevesini aldığı görüldü ve düzeltildi (§2); boş hedef katmanın uyarı
  üçgeni bilgi simgesi oldu (§3). Yerleşim denetimi sekiz görünümde sorunsuz (uzun adlar kesilir, düğmeler kesilmez).
- **Süreler** (release; masaüstü 10 koşunun p50'si, web gönderilen WASM'la 10 koşunun p50'si; her koşu geri alınarak):

  | İş | Masaüstü | Bütçe | Web | Web bütçesi |
  |---|---|---|---|---|
  | 10 000 parsel, 10 000 satır, beş alan, 100 ekleme (denetim, araç, tek adım) | 70,1 ms (p95 79,8) | 120 | 109,9 ms (p95 120,9) | 300 |

  Masaüstünün ilk ölçümü 100,7 ms'ydi (hazırlık 15,0, araç 48,5, uygulama 41,8 ms): pencerenin denetimi dosyanın 10 000 satırını iki kez
  kopyalıyordu (uygunluk ve boşluk), kaynağın tablosu her hücreyi kopyalıyordu, çalıştırıcı her yamanın özniteliklerini iki kez
  kopyalıyordu. Satırlar yerlerinde okununca ve yamanın parçaları taşınınca 70,1 ms (hazırlık 4,0, araç 40,8, uygulama 27,2 ms). Çekirdek
  WASM değişmedi.
