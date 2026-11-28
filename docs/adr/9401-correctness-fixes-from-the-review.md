# ADR 9401: İncelemenin doğruluk düzeltmeleri

- **Durum:** kabul edildi (2026-11-28). [plan/bakim.md](../../plan/bakim.md) Aşama 1, BK-10…BK-19.
- **Kural:** CLAUDE.md §7 (belge değişikliği geri alınabilir; düzenleme bütün yazılır ya da hiç), §8 (hata iletisi),
  §20.2 (geç kalan sonuç başka belgenin üstüne uygulanmaz), §23 (sayısal doğruluk), §25 kural 6 (veri yalnız komutla
  değişir).
- **İlgili:** [ADR 9400](9400-maintenance-plan.md) (bakım planı), [ADR 0253](0253-command-protocol.md)
  (komut adımı ve kayıt), [ADR 0560](0560-maplibre-style-exchange.md) (MapLibre stilinin okuma kuralı),
  [ADR 9113](9113-query-layers.md) (sorgu katmanı).
- **Ürün:** hata düzeltmeleri; her biri aşağıda. **`.kcad`:** değişmedi. **Sözleşme:** madde madde yazılı.
- **Bağımlılık:** yeni dış bağımlılık yok. `kentos-maplibre` artık `kentos-svg-core`'a bağlı (§3.5).

## Bağlam

28 Kasım 2026 incelemesi ([plan/bakim.md](../../plan/bakim.md) “A. Doğruluk”) on bir doğruluk bulgusu çıkardı. Her
düzeltme ayrı commit'tir ve düzeltmeden önce kırmızı olan bir sınamayla gelir. Bu ADR her birinin kararını tutar.

## Kararlar

### 1. İmar takımı bütün yazılır ya da hiç (BK-10)

**Bulgu.** `kentos_native_application::permit_set::execute` önce künyeyi ayarlara yazıyor, sonra sayfaları
planlıyordu. Plan reddedilince (kat yok, bina bulunamadı, şablon okunamadı) künye yazılı kalıyordu. Ayarlar geri alma
adımı değildir; masaüstünün açtığı grup da onları geri almaz.

**Karar.**

- Takım önce planlanır: sayfalar, tadilat görünümlerinin denetimi ve pafta işlemleri yalnız okur. Hiçbiri künyeye
  bakmaz. Plan reddedilirse hiçbir şey yazılmamıştır.
- Sonra künye ve görünümler tek `set_settings` ile, ardından paftalar `cad.sheets.edit` ile yazılır. Paftalar
  reddedilirse ayarlar eski hâline konur.
- Bütünü bir `Document::run_step` çerçevesine almak seçilmedi: çerçevenin içindeki komut kendi kayıt satırını yazmaz
  (ADR 0253), Komut geçmişi takımın `cad.sheets.edit` satırını kaybederdi. Kalıcı çözüm takımın kendi komutu
  olmasıdır (BK-19).

**Sınama.** `permit_set::tests::a_refused_set_writes_no_particulars`: katsız çizimde takım reddedilir; ayarlar ve
revizyon değişmez. Düzeltmeden önce kırmızıydı (künyenin değişkenleri yazılıyordu).

### 2. İfade ve süzgeç ayrıştırıcılarında derinlik ve işlem sınırı (BK-14)

**Bulgu.** İfade dilinin ayrıştırıcısı (`crates/shared/expression/src/parser.rs`) ve stil süzgeci dilbilgisi
(`crates/shared/formats/src/sld.rs` `parse_qgis`, `parse_filter`) iç içe yapıları kendini çağırarak okur; ağacı gezen
öbür işlevler de (çözümleme, derleme, kaynağa itme) öyle. Derinliği sayan yoktu. İçe alınan bir stil dosyasının süzgeci
ya da bir proje dosyasındaki ifade programı çökertebilirdi; yığın taşması yakalanamaz.

Ölçüm (28 Kasım 2026, bu Mac; 2 MB yığınlı iş parçacığı, bir işçi iş parçacığının boyu):

| Girdi | Hata ayıklama derlemesi | Sürüm derlemesi |
|---|---|---|
| iç içe parantez `(((…1…)))` | 150 geçer, 200 taşar | 700 geçer, 1 000 taşar |
| işaret zinciri `----1` | 300 geçer, 400 taşar | 2 500 geçer, 3 000 taşar |
| işlem zinciri `1 + 1 + …` | 300 geçer, 400 taşar | 2 500 geçer, 3 000 taşar |

Üçüncü satır düşmanca girdi değildir: `a = 1 VEYA a = 2 …` biçiminde 3 000 terimli bir süzgeç sürüm programını
çökertiyordu. Zincir soldan derinleşen bir ağaçtır (`a + b + c`, `(a + b) + c`'dir): uzunluğu kadar derindir.

**Karar.**

- İfade dili: en çok **64** düzey iç içelik (`MAX_DEPTH`; parantez, işaret, argüman, dal) ve bir ifadede en çok
  **1 000** birleştirilen işlem (`MAX_OPERATIONS`). 64, JSON ve CBOR okuyucularının sınırıdır. Aşan ifade derlenmez;
  ileti yerini, nedenini ve çözümünü söyler (uzun değer listesi `içinde (…)` ile yazılır).
- Derinlik `unary`'de sayılır: dilin her iç içeliği oradan geçer. İşlemler `binary`'nin döngüsünde sayılır: ağacın
  özyinelemesiz derinleştiği tek yer orasıdır. Ağaç hiç kurulmadığı için reddin kendisi de yığını taşırmaz.
- Stil süzgeci: en çok 64 düzey; aşan süzgeç okunmaz (`None`, dilbilgisinin söyleyemediği süzgeç gibi). `VE` ve `VEYA`
  orada zaten düz listedir, zincir derinleşmez.
- Sınır derleme profiline göre değişmez: ortak durumlar her yerde aynı sonucu verir. 1 000 işlem sürümde 2 MB'ta 2,5 kat
  payla güvenlidir. Hata ayıklama derlemesi 2 MB'ta 300 işlemde taşar; cargo'nun koşturduğu her program 64 MB yığın alır
  (`.cargo/config.toml`, ADR 9400 §5), elle koşturulan hata ayıklama ikilisi almaz.
- Davranış değişikliği: 1 001–2 500 işlemli ifade sürümde derleniyordu, artık reddedilir.

**Sınama.** `crates/shared/expression/tests/language.rs` `an_expression_nested_past_the_limit_is_refused` ve
`an_expression_of_too_many_operations_is_refused` (100 000 parantez, işaret ve terim 2 MB'lık iş parçacığında
reddedilir; sınırın iki yanı); `sld::tests::a_filter_nested_past_the_limit_is_not_read`. Düzeltmeden önce aynı girdiler
süreci düşürüyordu (yukarıdaki ölçüm).

### 3. Sessiz kayıplar ve tek deyimli sorgu (BK-15)

Beş ayrı yerde okuyucu ya da yazıcı bir şeyi söylemeden bırakıyordu. Ortak karar: **okunamayan değer ya reddedilir ya
da rapora yazılır; yerine başka bir şey sessizce konmaz.**

#### 3.1 DXF: tarama deseni, geçiş ve resim adları çizimin kod sayfasında

**Bulgu.** DXF okuyucusunun düz metin okuması UTF-8 olmayan baytta boş döner (`formats/src/dxf/lexer.rs`). Tarama
deseninin adı, geçişin adı ve resmin dosyası bu yoldan okunuyordu, kod sayfası çözücüsünden geçmiyordu. Windows-1254
bir çizimde Türkçe harfli desen adı “_USER” oluyordu, resmin dosya adı kayboluyordu; rapor bir şey söylemiyordu.

**Karar.** Üç metin de katman ve blok adları gibi çizimin çözücüsünden (`Decoder`) geçer.

**Sınama.** `formats/tests/all/dxf_bridge.rs`: `a_hatch_pattern_s_name_is_read_in_the_drawing_s_code_page`,
`an_image_s_file_is_read_in_the_drawing_s_code_page`.

#### 3.2 WFS: koordinatı okunamayan nesne şekilsiz gelir ve söylenir

**Bulgu.** Servisin GML okuyucusu (`services/src/gml.rs`) sayı olmayan belirteci atlayıp okumayı sürdürüyordu:
`0 0 x 1 1 2 2` listesi (0,0), (1,1), (2,2) çizgisi oluyordu. Konumun ortasında biten listenin sonu da aynı yolla
düşüyordu. Sonraki bütün koordinatlar bir yer kayıyordu ve hiçbir şey söylenmiyordu.

**Karar.**

- Bir koordinatı okunamayan geometri verilmez: sayı olmayan ya da sonlu olmayan değer, konumun ortasında biten liste,
  eksik `X`/`Y`. Nesne şekilsiz gelir ve sayılır.
- Alımın raporu bunu servisin kendi boş gönderdiği nesnelerden ayrı söyler: “Koordinatı okunamayan nesne”. İkisinin
  nedeni ve çözümü başkadır (boş gelen nesne için başka koordinat sistemi önerilir).
- Dosyadan GML okuyucusu (`application/src/gis/gml_read.rs`) aynı durumda dosyayı reddeder. Servis sayfa sayfa gelir ve
  öbür nesneleri sağlamdır: alım sürer, kayıp söylenir.

**Sınama.** `gml::tests::a_coordinate_that_does_not_read_leaves_no_shape`,
`feed::tests::features_whose_coordinates_did_not_read_are_told_apart_from_empty_ones`.

#### 3.3 LAS: VLR'ye sığmayan kayıt taşınır ya da yazma reddedilir

**Bulgu.** LAS yazıcısı (`pointcloud/src/write.rs`) 65 535 baytı aşan değişken uzunluklu kaydı (VLR) söylemeden
atlıyordu.

**Karar.** LAS 1.4'te kayıt genişletilmiş kayıt (EVLR) olarak taşınır. 1.2 ve 1.3'te onun yeri yoktur: yazma
reddedilir; ileti kaydı, boyunu ve çözümü (LAS 1.4 olarak yazmak) söyler.

**Sınama.** `write::tests::a_record_too_large_for_a_vlr_is_carried_or_refused`.

#### 3.4 GeoPackage: sorgu katmanının metni SQLite'a tek deyim olarak gider

**Bulgu.** Üç katmanlıydı; her biri kodda gösterildi.

1. Sürücü sorgunun sütunlarını bulmak için geçici görünümü `execute_batch` ile kuruyordu
   (`native/data-gpkg/src/query.rs`). Bu çağrı metindeki bütün deyimleri çalıştırır.
2. Ortak denetim (`kentos_data::query::check`) yalnız `'…'` ve `"…"` tırnaklarını bilir. SQLite ayrıca `` `…` `` ve
   `[…]` biçimindeki adları, `$ad(…)` biçimindeki değişkeni de okur. ``SELECT 1 AS `a"`; …; --"`` metni denetimden
   tek deyim olarak geçiyordu, SQLite için iki deyimdi ve ikincisi çalışıyordu. Okuma bağlantısı salt okunurdur ama
   geçici tablo ve görünüm alır: bir tablonun adıyla kurulan geçici görünüm, saklanan bağlantıda o tabloyu 600 satır
   yerine 0 satır okutuyordu.
3. Çağrıyı tek deyimli olanla değiştirmek yetmiyordu. `rusqlite` ikinci deyimi reddetmek için onu derler; SQLite bazı
   ayarlarını (`PRAGMA`) deyim derlenirken uygular. Sınamada `PRAGMA case_sensitive_like` bağlantının karşılaştırmasını
   yalnız derlenerek değiştirdi.

**Karar.**

- Görünüm tek deyim alan çağrıyla kurulur. Birden çok deyim reddedilir, hiçbiri çalışmaz.
- Metin SQLite'a verilmeden önce onun kendi kurallarıyla okunur (`native/data-gpkg/src/statement.rs`,
  `one_statement`). Kurallar paketlenen SQLite'ın (3.54.1) ayrıştırıcısından (`sqlite3GetToken`) alındı: üç tırnak,
  köşeli parantezli ad, iki yorum biçimi; `;`'den sonra yalnız boşluk, yorum ve `;` gelebilir.
- Değişken (`?`, `:ad`, `@ad`, `$ad`, `#ad`) reddedilir. Katmanın sorgusuna değer verilmez; `$ad(…)` de bu okumanın
  izleyemeyeceği tek belirteçtir. Çalışan hiçbir sorgu bundan etkilenmez: SQLite görünümde değişkeni zaten reddeder.
- Ortak denetim lehçeden bağımsız kalır. PostGIS sürücüsü görünümü hazırlanmış deyimle kurar (`sqlx::query`,
  sqlx-postgres 0.9.1'in kaynağından doğrulandı): PostgreSQL hazırlanmış deyimde birden çok komutu çalıştırmadan
  reddeder.
- İleti aynıdır: “Tek bir sorgu yazın: noktalı virgülle ayrılmış birden çok deyim olmaz.”
  (`kentos_data::query::MANY_STATEMENTS`).

**Seçilmeyen.** SQLite'ın yetkilendiricisi (`sqlite3_set_authorizer`) metne değil yapılan işe bakar ve daha güçlü bir
korumadır. `rusqlite`'ın `hooks` özelliğini ve kullanıcı sorgusunun her derlemesinin çevresinde bir kapsam ister.
Bugünkü açık, metnin tek deyim olmasıyla kapanıyor; yetkilendirici ayrı bir iş olarak açık kaldı.

**Sınama.**

- `data-gpkg/tests/conformance.rs` `a_query_s_text_cannot_change_what_the_connection_reads`: düzeltmeden önce
  `parseller` 600 yerine 0 satır okuyordu.
- `query::tests::what_follows_the_statement_is_not_compiled`: tek deyimli çağrıdan sonra da kırmızıydı, okumadan
  sonra yeşil.
- `statement::tests::one_statement_is_told_as_sqlite_reads_it`: her biçim için örnekler.
- `statement::tests::the_reading_agrees_with_sqlite_s_own`: her tırnak biçiminden, içinde öbür biçimlerin işaretleri
  olan 4 000 metin üretilir; SQLite'a deyimleri tek tek sorulur ve okumayla karşılaştırılır. Okuyucu üç ayrı yerinden
  bilerek bozuldu; sınama üçünde de kırmızıya döndü.

#### 3.5 MapLibre: her CSS renk adı okunur, renk olmayan değer söylenir

**Bulgu.** [ADR 0560](0560-maplibre-style-exchange.md) §7 stilin renklerini CSS renkleri olarak tanımlar (“adlı
renkler” dahil) ve “renk okunamazsa katman alınmaz” der. Kod ikisini de tutmuyordu:

- Okuyucu 15 renk adı biliyordu. MapLibre'nin belirtimi CSS'in bütün adlarına izin verir.
- Okunamayan metin sabit değer olarak kalıyor ve söylenmeden siyah çiziliyordu. `"fill-color": "steelblue"` siyah
  geliyordu. Durakları adla yazılmış bir ısı haritasının rampası tamamen saydam geliyordu. `match`'in bir kolundaki
  sabit ifade (`["rgb", 1, 2, 3]`) de siyahtı.
- Etiketin okunamayan hale rengi söylenmeden siyah alınıyordu.

**Karar.**

- **Adlar.** CSS'in 148 renk adının hepsi okunur. Tablo SVG çekirdeğinindir (`kentos_svg_core::values::read_color`);
  MapLibre crate'i kendi tablosunu tutmaz. Değerler üç kaynakta aynıdır: depodaki tablo, Python'un görüntü kitaplığının
  (Pillow) tablosu ve MapLibre'nin resmî paketi (`@maplibre/maplibre-gl-style-spec` 26.4.4, `Color.parse`).
  `maplibre_cases.py --check` üretecin tablosunu her koşuda resmî paketle karşılaştırır.
- **Renk olmayan değer.** Bir renk anahtarının alabileceği her değer, katman okunurken bir kez denetlenir
  (`maplibre/src/colour.rs`, `settle`): sabit, `match`'in kolları ve yedeği, `step`'in tabanı ve durakları, alanın
  yedeği. Renk olmayan değer söylenir (“… rengi okunamadı; değer alınmadı”). Renk katmanın kendi rengiyse katman
  alınmaz (ADR 0560'ın kuralı); çerçevenin, halenin ya da kenarın rengiyse varsayılanı kalır.
- **Kolun sabit ifadesi.** `["rgb", …]`, `["rgba", …]`, `["to-color", "ad"]` ve `["literal", …]` bir kolda ya da
  durakta kendi rengidir.
- **Etiket ve ısı haritası.** Etiketin okunamayan hale rengi söylenir, siyah alınır (etiket okuyucusunun yazı rengindeki
  kuralı). Isı haritasının okunamayan durağı söylenir ve atlanır.
- **Bırakılan ilk yazım.** Düzeltme önce “siyah al ve söyle” diye yazıldı. ADR 0560'ın kuralıyla çeliştiği için
  bırakıldı: yanlış renk çizmek, katmanı almamaktan kötüdür.
- **Davranış değişikliği.** Renk adıyla yazılmış stil kendi renkleriyle gelir. Geçersiz renk metni taşıyan katman
  eskiden siyah geliyordu; artık alınmaz ve söylenir.
- **Kalan.** CSS rengini okuyan üç ayrı kod var (SVG çekirdeği, servislerin stil okuyucusu, MapLibre okuyucusunun
  `#…`, `rgb()` ve `hsl()` kolu). Birleştirilmeleri BK-52'dedir.

**Sınama.**

- `maplibre/src/colour.rs`: `every_colour_name_reads` (önceki 15 ad eskisi gibi okunur), `a_named_colour_is_drawn_as_itself`,
  `a_colour_that_does_not_read_is_told`, `a_halo_and_a_stop_that_do_not_read_are_told`. Düzeltmeden önceki kodda
  kırmızıydılar.
- Ortak durumlar: `file.import.maplibre.json` 36 → 39, `file.import.maplibre.labels.json` 25 → 27 durum; beş yeni stil
  dosyası. Var olan durumlar bayt bayt aynı kaldı. Bağımsız üreteç (`scripts/fixtures/maplibre_cases.py`) aynı kuralı
  kendi koduyla uygular; Rust ve Python koşucuları 66 durumda onunla aynı sonucu verdi. Bir beklenen renk bilerek
  bozulunca Rust koşucusu yakaladı.

### 4. Bir Kaydet: bütün düzenlemeler ya da hiçbiri (BK-11)

**Bulgu.** Dört pencerenin Kaydet'i değişiklikleri sırayla, her birini kendi komutuyla yazıyordu: İlişkiler
(`cad.relations.edit`), Birleşik görünümler (`cad.joins.edit`), Olay bağları (`cad.linearref.define`) ve Kılavuz
stilleri (`cad.styles.edit`). Bir düzenleme reddedilince öncekiler yazılı kalıyordu; ilk üçünde kod bunu yorumuyla
söylüyordu. Kılavuz stilleri yalnız `Failed` cevabına bakıyordu: çakışma ve eksik girdi “kaydedildi” sayılıyordu.

- Bu dört komut proje ayarlarını yazar. Ayarlar geri alma adımı değildir: `Document::transact` ve
  `begin_group`/`cancel_group` yalnız geri alma işlemlerini geri alır (`domain/src/history/mod.rs`). Planın önerdiği
  `document::in_step` (o da `transact`'tir) bu pencerelere yetmez.
- Değer listeleri penceresi (`gis/forms/domains.rs`) doğrudur ve dokunulmadı: `cad.domains.edit` listeyi geri alınabilen
  bir işlemle yazar, pencerenin açtığı grup ret gelince hepsini geri alır. Sınaması vardı.

**Karar.**

- Tek yardımcı: `App::save_together(label, command, inputs)` (`apps/desktop/src/cad/layer_commands.rs`).
  - Tek düzenleme kendi komutuyla yazılır; kayıttaki satırı eskisi gibi komutun kendisidir.
  - Birden çok düzenleme tek değişiklik olarak `cad.changeset.apply` ile yazılır: bir adım reddedilirse bütün adımlar,
    ayarlar da, geri alınır (ADR 0253 §8). Python, MCP ve ajan aynı işi aynı komutla yapar.
  - Ret, düzenlemenin kendi sözleriyle söylenir (`changeset::step_refusal`): kullanıcı düzenleme gönderdi, kümenin
    “2. adım:” öneki ona bir şey söylemez.
  - Ret gelince pencere olduğu gibi kalır; hiçbir şey yazılmamıştır.
- Bütünü bir `Document::run_step`'e sarmak seçilmedi: içteki komut kendi kayıt satırını yazmaz (§1'deki gerekçe). Küme
  kendi satırını adımlarıyla yazar.
- Davranış değişikliği:
  - reddedilen Kaydet hiçbir şey yazmaz;
  - birden çok düzenleme Komut geçmişi'nde tek satırdır (`cad.changeset.apply`, pencerenin adıyla);
  - Kılavuz stilleri çakışmada “kaydedildi” demez.

**Sınama.** Dördü de düzeltmeden önce kırmızıydı (reddedilen Kaydet'ten sonra ilk düzenleme yazılı kalıyordu):

- `joins_tests::a_refused_view_leaves_the_others_unwritten`; sözün düzenlemenin kendi sözü olduğunu da denetler;
- `relation_keys_tests::a_refused_relation_leaves_the_others_unwritten`;
- `event_links_window::tests::a_refused_link_leaves_the_others_unwritten`: Yeni'ye iki kez basıp aynı olay katmanını
  seçen kullanıcının durumu;
- `leader_tools::styles_tests::a_refused_style_leaves_the_others_unwritten`;
- uygulama katmanında `changeset::tests::a_refused_step_s_error_is_given_back_as_its_own`.

### 5. Arka plan yanıtı soran pencerenindir (BK-12)

**Bulgu.** Plan beş aktarımı sayıyordu; kod bunun ortak düzenek olduğunu gösterdi. Aktarım pencerelerinin işi arayüz
iş parçacığının dışında tek yardımcıyla koşar (`exchange::off_thread`, 30'dan çok çağrı). Yanıtın işleyicisi o anda
hangi pencere açıksa ona dokunuyordu:

- **Başarı:** `close_exchange()` koşulsuz çağrılıyordu. CSV yazılırken penceresi kapatılıp DXF dışa aktar açılınca,
  CSV'nin yanıtı DXF penceresini kapatıyordu (sınamayla gösterildi).
- **Hata:** 17 kol hatayı yalnız pencereye yazıyordu. Pencere kapatılmışsa hiçbir şey söylenmiyordu: yazılamayan dosya
  sessiz kalıyordu.
- **IFC revizyonlarını karşılaştır:** yanıt koşu kimliği taşımıyordu. Karşılaştır'a yeniden basılınca yavaş biten eski
  koşunun yanıtı yenisinin üstüne yazıyordu.

**Karar.**

- **Pencerenin numarası.** `exchange::Runs` (`App.exchange_runs`): pencere açılınca ve kapanınca yeni bir numara.
- **Yanıt numarayla döner.** Bir aktarım olayının başlattığı işin verdiği her ileti `Message::ExchangeReply { run, … }`
  olarak gelir (`exchange_scoped`; `update`'teki tek dağıtım noktasında).
- **Geç yanıt hiçbir pencereye dokunmaz.** Numara açık pencereninkiyse yanıt olduğu gibi işlenir. Değilse (pencere
  kapandı ya da başkası açıldı) şimdiki pencere, diyalog ve numara kenara alınır, yanıt işlenir, sonra yerlerine konur
  (`exchange_reply`). Yanıtın söylediği ileti günlüğüne düşer: geç biten yazma yine “… yazıldı” der.
- **Hata sessiz kalmaz.** 17 kol hatayı soran pencere açıksa onda gösterir, değilse ileti günlüğüne yazar.
- **IFC karşılaştırması.** Her koşu süreç genelinde tek bir numara alır; başka koşunun yanıtı yok sayılır.
- **Seçilmeyenler.**
  - Geç yanıtı atmak (`ScopedReply` başka çizimin yanıtını atar): aktarımın sonucu hâlâ kullanıcının haberidir.
  - Yazma sürerken pencereyi kapattırmamak: uzun içe aktarma pencere kapatılarak durdurulur.
- Pafta yayını kendi kimliğini zaten taşıyordu; dokunulmadı.

**Sınama.**

- `exchange::tests::a_late_answer_does_not_close_another_window`: kenara alma kapatılınca kırmızı (denendi).
- `exchange::tests::a_late_failure_is_said`: düzeltmeden önce kırmızı.
- `ifc_revisions_tests::an_earlier_comparison_s_answer_does_not_write_over_a_newer_one`: düzeltmeden önce kırmızı.
- İleti akışı bütün aktarım pencerelerinde değiştiği için masaüstünün aktarım, iz, kaynak ve nokta sınamaları (488) ve
  programlar arası iz sınaması koşuldu.

### 6. Ağır iş arayüz iş parçacığında koşmaz: BCF okuması ve form taslağı (BK-13)

**Bulgu.**

- **BCF sorunları** (`desktop-arch/src/bcf_window.rs`): Oku, BCF dosyasını ve andığı IFC modelini tıklamanın içinde
  okuyordu. IFC modeli onlarca megabayt olabilir; pencere okuma bitene kadar donuyordu.
- **Formlar** (`gis/forms/designer/step.rs`): pencere değişmişken her olayda taslak JSON'a çevrilip olayın içinde
  diske yazılıyordu. Yazma hatası yutuluyordu (`library.rs`): klasör yazılamazken kullanıcı taslağın korunduğunu
  sanıyordu.

**Karar.**

- **BCF:** okuma kendi iş parçacığında koşar (IFC karşılaştırmasının kalıbı: çizimin kopyası, 64 MB yığın, koşu
  numarası). Pencere “Okunuyor…” der; eski koşunun yanıtı yok sayılır.
- **Form taslağı** (`forms/library.rs` `DraftKeeper`, `designer/draft.rs`):
  - yazma arayüz iş parçacığının dışındadır ve aynı anda bir tanedir;
  - yeni taslak süren yazmayı bekler ve bekleyenin yerini alır: dosya pencerenin son hâliyle biter;
  - aynı taslak yeniden yazılmaz;
  - hata bir kez söylenir, yazma yeniden başarılı olana dek yinelenmez;
  - Vazgeç ya da Kaydet taslağı bırakınca bekleyen unutulur, süren yazma dosyayı geri getirmez.
- **Kalan:** `forms_step` her olayda pencerenin tamamını kopyalar (1 620 satırlık işlev). İşlev bölünürken ele alınır
  (BK-40).

**Sınama.**

- `ifc_revisions_tests::the_issues_are_read_off_the_interface_thread`: tıklama dosyayı beklemeden döner.
- `workflow::a_draft_that_cannot_be_kept_is_said_once`: düzeltmeden önce hiçbir şey söylenmiyordu.
- `workflow::a_draft_is_written_off_the_interface_thread_and_ends_as_the_window_is`: düzeltmeden önce taslak olayın
  içinde yazılıyordu; ayrıca art arda iki düzenlemeyi ve yazma sürerken Vazgeç'i denetler.
- Sınama yardımcısı (`form_scenes::designer`) artık taslağın yanıtını bekler; 390 form sınaması geçti.

### 7. Arayüz metninde başka programın adı (BK-18)

**Bulgu.** İnceleme üç metin saymıştı. Bekçi (`other_names_tests`) bunları görmüyordu, iki nedenle: listesinde yalnız
on iki ad vardı ve arayüz kitlerinin crate'lerini (`crates/ui`, `crates/sheet-ui`, `crates/model-view-ui`) taramıyordu.
Bekçi genişletilince sekiz metin çıktı:

| Yer | Ad |
|---|---|
| CSV dışa aktar'ın ayırıcı ipucu | bir tablo programı |
| Uygulama menüsünün KML satırları (iki) ve KML dışa aktar'ın ipucu | bir yer küresi görüntüleyicisi |
| GeoPDF'in ipucu (`sheet-ui`) | bir kitaplık, bir CBS programı ve bir PDF okuyucu |
| SVG dışa aktar'ın seçeneği | bir çizim programı |
| Tablo yaz'ın ODS seçeneği | bir ofis takımı |
| Katalogdaki iki açıklama (`file.export.kml`, `file.import.kml`; ipuçları ve Komut ara gösterir) | bir yer küresi görüntüleyicisi |

**Karar.**

- Sekiz metin, programın adı yerine ne yaptığını söyleyerek yeniden yazıldı.
- Bekçi dört ad daha arar ve arayüz kitlerinin üç crate'ini de tarar.
- **Açık bırakılanlar (sahibin kararı ya da sonraki iş):**
  - “Excel (.xlsx)” 27 yerde biçimin adı olarak yazılı. Kural dosya biçimi adlarını uygulama adı saymıyor; “.xlsx”in
    herkesin bildiği adı da budur. Dokunulmadı; karar sahibindir.
  - Sözleşmelerin belge yorumlarında bir kitaplığın adı 17 yerde geçiyor (8 dosya) ve Python'un `help()` metnine
    14 satır olarak çıkıyor. Sözleşmeleri değiştirmek kataloğu ve SDK'yı yeniden üretir; BK-16 ile birlikte yapılır.
  - Bir ortak durumun örnek grup adı (`gis_import_cases.py`) arayüz metni değildir; üretecine dokunan işte değişir.

**Sınama.** `other_names_tests` (genişletilmiş hâliyle; düzeltmeden önce iki katalog açıklamasını ve GeoPDF ipucunu
gösterdi), `kentos-sheet-ui` sınamaları, dokunulan pencerelerin sınamaları.

## Modül ve kanal tablosu

| İş | Mimari | Harita | Yol | CBS | MCP | Python | Eklenti | Yapay zekâ ajanı |
|---|---|---|---|---|---|---|---|---|
| İmar takımı (BK-10) | var: Mimari › Proje ▾ › Ruhsat takımı (`arch.permitSet`) | yok: Mimari'nin işi | yok | yok | yok: katalog komutu değil (BK-19) | yok (BK-19) | yok (BK-19) | yok (BK-19) |
| İfade ve süzgeç sınırları (BK-14) | var | var | var | var | var: aynı ayrıştırıcı | var: aynı ayrıştırıcı | var: aynı ayrıştırıcı | var: aynı ayrıştırıcı |
| DXF içe aktar (BK-15, §3.1) | var | var | var | var | var: `file.import.dxf` | var: `kentos.cad.file.import_.dxf` | yok: dosya işleri eklentiye kapalı | var: aynı komut |
| Servisten veri al, WFS (BK-15, §3.2) | yok: servis katmanı türü yok | var | var | var | var: `cad.layers.service` | var: `kentos.cad.layers.service` | var: `run`, `document.write` izniyle | var: aynı komut |
| Nokta bulutu işlem araçlarının yazdığı LAS (BK-15, §3.3) | yok: araçlar profilinde yok | var | var | var | var: `cad.processing.run` | var: `kentos.cad.processing.run` | var: `run`, `document.write` izniyle | var: aynı komut |
| GeoPackage sorgu katmanı (BK-15, §3.4) | yok: veri katmanı türü yok | var | var | var | var: `cad.layers.data`, `cad.layers.rows` | var: `kentos.cad.layers.data` | var: `run`, okuma ve yazma izinleriyle | var: aynı komut |
| MapLibre stilinden görünüş al (BK-15, §3.5) | yok: stil içe alma yalnız CBS'de (ADR 9316) | yok | yok | var | var: `file.import.maplibre` | var: `kentos.cad.file.import_.maplibre` | yok: dosya işleri eklentiye kapalı | var: aynı komut |
| İlişkiler'in Kaydet'i (BK-11) | yok: veri katmanı türü yok | var | yok: profilinde yok | var | var: `cad.relations.edit`; birden çoğu `cad.changeset.apply` | var: `kentos.cad.relations.edit` | var: `run`, `document.write` izniyle | var: aynı komut |
| Birleşik görünümler'in Kaydet'i (BK-11) | yok: veri katmanı türü yok | var | yok: profilinde yok | var | var: `cad.joins.edit`; birden çoğu `cad.changeset.apply` | var: `kentos.cad.joins.edit` | var: `run`, `document.write` izniyle | var: aynı komut |
| Olay bağları'nın Kaydet'i (BK-11) | yok: profilinde yok | var | var | var | var: `cad.linearref.define`; birden çoğu `cad.changeset.apply` | var: `kentos.cad.linearref.define` | var: `run`, `document.write` izniyle | var: aynı komut |
| Kılavuz stilleri'nin Kaydet'i (BK-11) | var | var | var | yok: profilinde yok (CAD açıklaması) | var: `cad.styles.edit`; birden çoğu `cad.changeset.apply` | var: `kentos.cad.styles.edit` | var: `run`, `document.write` izniyle | var: aynı komut |
| Aktarım pencerelerinin arka plan yanıtı (BK-12) | var | var | var | var | yok: pencere yok, komut cevabıyla döner | yok: aynı | yok: aynı | yok: aynı |
| IFC revizyonlarını karşılaştır'ın yanıtı (BK-12) | var | yok: Mimari'nin işi | yok | yok | yok: pencere yok, `cad.ifc.compare` cevabıyla döner | yok: aynı | yok: aynı | yok: aynı |
| BCF sorunları'nın okuması (BK-13) | var | yok: Mimari'nin işi | yok | yok | yok: pencere yok, `cad.bcf.read` cevabıyla döner | yok: aynı | yok: aynı | yok: aynı |
| Formlar'ın bu aygıtta tutulan taslağı (BK-13) | yok: profilinde yok | var | yok: profilinde yok | var | yok: taslak pencerenindir; kanal formu `cad.forms.edit` ile yazar | yok: aynı | yok: aynı | yok: aynı |

Düzeltmeler hiçbir hücreyi değiştirmedi. İfade dili her modülde ve her kanalda aynı crate'ten (`kentos-expression`)
derlenir; sınır hepsinde aynıdır. BK-15'in düzeltmeleri komutların altındaki okuyucu, yazıcı ve sürücüdedir: her kanal
aynı komutu çağırdığı için aynı sonucu alır.

Kaynak: modüller `catalog/v1/modules.json`, Python ve MCP adları `docs/cad/manifest.json`, eklentinin kuralı kılavuz
§9.2. BK-15 için koşulan kanal sınaması: ortak durumlar Rust koşucusunda (`file.import.maplibre`) ve Python
koşucusunda (`file.import.maplibre`, `cad.layers.data`, `cad.layers.rows`, `file.import.dxf`) geçti. MCP'nin ve
eklentinin kendi sınamaları bu dilimde koşulmadı: aynı işleyiciyi çağırırlar, sözleşmeleri değişmedi.

BK-11 kanalların yapabildiğini değiştirmedi: çok düzenlemeyi tek değişiklik olarak yazmak onlarda `cad.changeset.apply`
ile zaten vardı; masaüstünün pencereleri artık aynı komutu kullanıyor. Koşulan sınamalar pencerelerinkidir (masaüstü,
209 sınama: dört pencere, form akışı ve çapraz süzgeçler).
