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
  - Sözleşmelerin belge yorumlarında bir kitaplığın adı 17 yerde geçiyordu (8 dosya) ve Python'un `help()` metnine
    14 satır olarak çıkıyordu. BK-16'nın yeniden üretiminde temizlendi (§8).
  - Bir ortak durumun örnek grup adı (`gis_import_cases.py`) arayüz metni değildir; üretecine dokunan işte değişir.

**Sınama.** `other_names_tests` (genişletilmiş hâliyle; düzeltmeden önce iki katalog açıklamasını ve GeoPDF ipucunu
gösterdi), `kentos-sheet-ui` sınamaları, dokunulan pencerelerin sınamaları.

#### 7.1 Bekçi her crate'in metnine bakar (28 Kasım 2026, ikinci tur)

**Bulgu.** Bekçi yalnız masaüstü programlarının, arayüz kitlerinin, kataloğun ve sözleşmelerin metinlerini
tarıyordu. Kişiye ulaşan metinlerin çoğu onların altında yazılır: komutun reddi ve notu (uygulama katmanı), okuyucunun
açılış raporu (biçim crate'leri), servisin hatası, işlem aracının açıklaması. NCZ okuyucusunun raporundaki iki ileti
böyle kaçmıştı (§9.2).

**Karar.** Bekçi bütün programların ve bütün crate'lerin metin sabitlerini tarar (vendored crate dışında). Ad listesi
17'den 31'e çıktı. Bir sözcüğün içindeki eşleşme ad sayılmaz (`SheetMapInfo`). ADR atfı, kişinin okuduğu metni yazan
yerlerde aranır: masaüstü, kitler, katalog, sözleşmeler ve artık uygulama, etkileşim ve işlem katmanları.

Çıkanlar düzeltildi:

- Başka program adı: stok hacminin notu ve sözleşmesinin açıklaması, arayüz vitrininin bir açıklaması, iki Python
  paketinin tanıtımı, iki sınamadaki ad.
- ADR atfı: Yol'un iki reddi (kübaj, Brükner), malzeme lejandı bloğunun açıklaması, iki işlem aracının açıklaması.
- İzin verilenler gerekçeleriyle listeye yazıldı: dosya imzaları (SHX, ikili DXF), GeoTIFF etiketinin ve bulut
  GeoTIFF başlığının kendi adları, servis türü ve coğrafi kodlama servisi olarak ArcGIS.

Dört komutun ortak durumlarındaki iletiler bağımsız üreteçlerinde değiştirildi. Üreteçlerden ikisi (kübaj, Brükner)
sabit ortamda dosyalarının son basamaklarını da yeniden yazıyor; bu, bu işten önceki bir ortam farkıdır. O dosyalarda
yalnız ileti satırları değiştirildi.

**Kalan.** Yorumlardaki adlar (kaynakta 800 kadar satır) dokunulan dosyada temizlenir; bekçi yorumları taramaz.

### 8. Hata kodu tablosu parçalara bölündü; kütlenin kendi kodu (BK-16)

**Bulgu.** `contracts/src/core/codes.rs` 3 387 satırdı ve boy sınırını aştığı için büyüyemiyordu. `cad.masses.create`
ile `cad.arch.edit` hatalı kütleyi bu yüzden mahalin koduyla (`invalid_space`) reddediyordu; gerekçe dosyanın başında
yazılıydı. Ajanlar ve betikler koda göre dallanır: kütlenin hatası mahalin hatası gibi görünüyordu.

**Karar.**

- **Tablo altı dosya** (`core/codes/`): çizim, servisler, düzenleme, pafta ve dosyalar, mimari, raster ve konak.
  Parçalar tablonun sırasını korur (katalog kodları bu sırayla listeler) ve derlenirken birleştirilir; sabitlerin
  yolları değişmedi. Bölmeden sonra yeniden üretilen katalog bayt bayt aynıydı.
- **`invalid_mass`:** halkası, yüksekliği, tabanı ya da adı hatalı kütle bu kodla reddedilir (`cad.masses.create` ve
  `cad.arch.edit`). Komut sürümü artırılmadı: kütle komutu yenidir (ADR 8178) ve ödünç kod belgesinde geçici olarak
  yazılıydı.
- **Aynı yeniden üretimde iki düzeltme:**
  - Sözleşmelerin belge yorumlarındaki kitaplık adı kaldırıldı (19 yer); bekçi artık bu adı da arar. BK-18'in açığı
    kapandı.
  - `cad.layers.migrate`'in bir ortak durumu `invalid_option` veriyordu, katalog bu kodu anmıyordu: yetenek sınaması
    bu dalda kırmızıydı (27 Kasım'daki işten beri). Kod komutun listesine eklendi.
- **Üretilen TypeScript dosyaları yenilenmedi.** Donmuş web'in içindeki dizin zaten eskiydi (başka işlerin tipleri
  eksik); `kentos-commands` ve `kentos-contracts` sınamaları her koşuda yeniden yazıyor, çalışma ağacı kirleniyor.
  Taşınması BK-25'tedir. `commandCatalog.json` ve Python SDK'sı yenilendi.

**Sınama.** `codes::tests`; katalog sınaması; ortak durumlar bağımsız üreteçten (`arch_command_cases.py`):
`cad.masses.create`'te 8, `cad.arch.edit`'te 2 beklenen kod değişti, öbür durumlar aynı kaldı. Yetenek sınaması
(düzeltmeden önce kırmızı) yeşil. Rust ve Python koşucuları üç komutta geçti; SDK `--check` temiz.

### 9. Tek yuvarlama kuralı ve kesin nokta-alan kararı (BK-17)

#### 9.1 Çizime yazılan sayılar gösterim kuralıyla yuvarlanır

**Bulgu.** Dört yer sayıyı Rust'ın kendi biçimlemesiyle yazıyordu (`format!("{v:.2}")`): tam yarım çift basamağa
yuvarlanır, 12,125 m² “12,12” olur. Gösterim kuralı (ADR 0149) aynı değeri “12,13” yazar; mahalin kendi etiketi de
öyle. Yerler: renk lejandının alanları, yangın yürüyüşlerinin uzunlukları, blok düğmelerinin uzunlukları, profil
denetiminin iletileri.

**Karar.** Dördü de `kentos_geometry_core::display::fixed` ile yazar. Beklenen değer kuralın kendi bağımsız
dosyasındandır (`fixtures/numeric/v1/display.json`: 12.125, iki basamak, 12.13). Masaüstündeki öbür `{:.N}`
biçimlemeleri BK-50'dedir.

#### 9.2 Bir nokta halkaların içinde mi: tek, kesin karar

**Bulgu.**

- Çekirdekte kesin bir sınama vardı (`point_in_polygon`: kesişimin noktanın sağında olup olmadığına yönelimle karar
  verir). Depoda aynı ışın sınamasını, kesişimi kayan noktayla hesaplayıp karşılaştırarak yapan yaklaşık otuz yerel
  kopya duruyordu (ölçüm: 52 ışın sınaması yerinden biri kesindi).
- Yuvarlanmış yazım, iki alanın ortak kenarının tam üstündeki noktayı iki alana birden ya da hiçbirine koyabiliyor.
  Çizim ölçeğindeki koordinatlarda (metre, santimetre basamaklı, başlangıca yakın) kenarın tam üstünde duran
  77 157 orta noktanın 3 353'ünde (%4,3) böyle oldu. Mimari'de bu, ortak duvar çizgisinin ortasındaki bir noktanın
  (kapı, kolon) iki mahalde birden ya da hiçbirinde sayılmasıdır.
- Transverse Mercator ölçeğinde iki yazım aynı sonucu verir: büyük sayıya yapılan toplamanın yuvarlaması kesişimi
  noktanın kendisine çeker. Sorun yerel koordinatlı çizimlerdedir.
- Kopyaların bir nedeni ikinci nokta türüdür: sözleşmelerin `Vec2`'si ile geometri çekirdeğinin `Vec2`'si ayrı
  türlerdir ve iki crate birbirini bilmez.

**Karar.**

- **Tek karar çekirdekte:** `ray_crosses(p, a, b)` (bir kenar için), `point_in_ring` (bir halka; `[x, y]`'sini
  veren her nokta türü için), `point_in_rings` (çok halka, çift-tek: delikli ve çok parçalı alan).
  Sözleşmelerin `Vec2`'si `[x, y]`'sini verir (`contracts/src/core/point_xy.rs`).
- **23 kopya ona bağlandı:** domain 4, application 4, interaction 4; geometri çekirdeğinin kendi içinde 4 (kesit,
  etiket motorunun `inside_rings` ve tek yürüyüşlü `signed_distance`'ı, iç ölçüler); stil çekirdeği 2, stil 1, baskı
  3, proje 1 (koordinat sisteminin alan sınırı; halkanın kendi kenarları korunarak), yol 1, model görünümü 1.
- **Bırakılanlar, gerekçesiyle:**
  - Shapefile'ın halka eşlemesi (`formats/src/shp/shape.rs`): ADR 0046'nın yazdığı kuraldır, bağımsız başvurusu
    aynı yazımı kullanır.
  - Nokta bulutunun sınırı (`pointcloud/src/ops/boundary.rs`): sınanan nokta yarım hücrededir, kenara hiç düşmez.
  - Arayüzün ekran pikselleri (`ui/src/spatial/query.rs`): ölçme değil, seçim.
  - NCZ okuyucusu (`ncz/src/emit.rs`) sonradan bağlandı: dosya başka bir programın adını 26 yerde anıyordu, ikisi
    açılış raporunda kişiye gösterilen iletideydi. Crate'in tamamındaki 61 atıf aynı işte temizlendi (iletiler
    “dosyayı yazan program” ve “dosyadaki stil” diyor).
  - Kesişimin kendi değerini kullanan tarama ve kesme hesapları: içerme sınaması değildir.
- Alan hesabının kopyaları §9.3'tedir.

**Bağımsız başvuru.** `scripts/fixtures/point_in_rings_cases.py` kuralı kesirlerle (`fractions`) uygular ve
`fixtures/geometry/v1/point_in_rings.json`'u yazar: 222 ortak kenar durumu (120'sinde yuvarlanmış yazım yanılıyor;
çizim ve TM ölçeği, yatay ve düşey kenar) ve 17 çok halkalı durum.

**Sınama.** Çekirdek başvuruyla 239 durumda aynı sonucu veriyor (`tests/all/point_in_rings.rs`). Üç kopyanın sınaması
düzeltmeden önce kırmızıydı (domain, application, interaction). Dokunulan on crate'in bütün sınamaları koşuldu;
geometri çekirdeğinin (800 + 377), etiket motorunun ve stilin altın dosyaları değişmedi.

#### 9.3 Bir halkanın alanı: koordinatlar ilk köşeye göre alınır

**Bulgu.** Çekirdeğin alan toplamı koordinatları ilk köşeye göre alır ve nedenini söyler: ham Transverse Mercator
koordinatlarının çarpımı parsel alanının dördüncü ondalığını yitirir. Depoda aynı toplamı ham koordinatlarla yapan
39 yer vardı (sınama dışı). Ölçüm (486 512,34; 4 420 187,52 çevresinde, kesirlerle hesaplanan kesin değere göre):

| Halka | Kesin | Ham koordinatla | Kaydırılmış |
|---|---|---|---|
| 10 × 10 m kare | 100,000 000 000 | 99,999 755 859 (−2,4·10⁻⁴) | 100,000 000 000 |
| ADR 0004'ün parseli | 748,515 093 505 | 748,515 136 719 (+4,3·10⁻⁵) | 748,515 093 505 |

**Karar.** Çekirdekte `ring_area` (her nokta türü için; `signed_area` onu çağırır). Çizimin alanını bildiren on kopya
ona bağlandı: Ölç kutusu, parselin alanı (imar hesabının girdisi), kaçış yolu, kapı denetimi, kesit, kolon birleşimi,
atlas, Yol'un korunan alan, tesviye ve yüzey analizi hesapları.

**Öbür 29 yer** tek tek okundu:

- **Alan bildiren ya da eşikle karşılaştıran yerler:** merdiven ve rampanın plan alanı (`stair_measure`,
  `ramp_measure`), yüzey onarımının “farkın sıfır olduğu alan”ı, çatının yüz eşikleri (`area2 / 2 > EPS`: ham
  toplamın yuvarlaması eşikten büyük olduğundan alansız yüz kalıyor ya da küçük yüz düşüyordu), kesitin kapak eşiği,
  blok kırpma sınırının “alanı yok” denetimi.
- **Yalnız yönü kullanan yerler:** halkanın hangi yöne döndüğünü toplamın işaretinden okurlar (duvar, döşeme, çatı,
  korkuluk, kolon tanıma, parsel, kütle, GeoParquet ve IFC halkaları, çizim motorunun alan kenarı…). Ham toplamın
  yuvarlaması küçük halkanın alanından büyük olabilir. Ölçüm: TM koordinatında 2–5 cm'lik 20 000 halkanın 10'unda
  işaret ters çıktı; 20–50 cm'liklerde hiç. Örnek: (486 280,859; 4 420 207,667)'deki 12 mm'lik, saat yönünün tersine
  çizilmiş karenin ham toplamı −0,000488, kesin değeri +0,000288.
- **Dokunulmayan üç yer** yerel koordinatlıdır: SVG çekirdeğinin iki toplamı (belge pikseli) ve hacim hesabında
  önceden kaydırılmış koordinatlarla çalışan moment toplamı.

Yirmi altısı çekirdeğin `ring_area`'sına bağlandı. Sözleşmeler crate'i çekirdeği bilmediği için kırpma sınırının
denetimi aynı kaydırmayla yerinde yazıldı.

**Sınama.** Üç kopyanın sınaması düzeltmeden önce kırmızıydı (10 m'lik kare 99,999 756 çıkıyordu). Yön için bir
sınama eklendi (`tests/all/ring_direction.rs`): 12 mm'lik karenin yönü; düzeltmeden önce kırmızıydı. Çekirdeğin altın
dosyaları ve dokunulan crate'lerin bütün sınamaları değişmeden geçti.

#### 9.4 Bir halkanın ağırlık merkezi: aynı kayıp, metrelerle

**Bulgu.** Çekirdeğin ağırlık merkezi koordinatları ilk köşeye göre alır; nedeni kendi yorumunda yazılıdır (ham TM
koordinatlarında 25 × 18 m'lik parselin merkezi 5 m kaymıştı, ADR 0122). Aynı toplamı ham koordinatla yapan sekiz
yerel kopya duruyordu. Ölçüm (486 512,34; 4 420 187,52 çevresinde, kesirlerle hesaplanan kesin değere göre):

| Halka | Ham koordinatla merkezin kayması | İlk köşeye göre |
|---|---|---|
| 10 × 10 m kare | 3,62 m | 0 |
| ADR 0004'ün parseli | 8,6 cm | 0 |
| 4 × 3 m oda | 0,3 mm | 0 |
| 0,5 × 0,3 m kolon | 1 446 m | 0 |

Kopyalar ve ne yaptıkları:

- Yol'un koridor denetimi: örtüşen parçanın merkezinde iki yüzeyin yüksekliğini okur. Kayık merkez parçanın dışına
  düşer, okunan fark başka bir yerin farkıdır.
- Yol'un yüzey kapsamı çizimi: alanın yazısını merkeze koyar.
- Mimari denetimin mahal noktası, gölgenin ve çıkma izdüşümünün iç noktası, mahalin net genişliğinin merkezi.
- İşlem araçlarının denetim olguları: birkaç alanın ortak merkezi.
- Etiket motorunun merkezi (erişilmezlik kutbu aramasının ilk tahmini).

Üç yer zaten kaydırıyordu ve bulgu değildir: stok hacmi (bütün koordinatlar sınırın ilk köşesinden alınır), dikiş
çizgileri penceresi, formun konum alanı.

**Karar.** Çekirdekte `ring_centroid` (her nokta türü için; ilk köşeye göre). Sekiz kopya ona bağlandı ya da aynı
kaydırmayla yazıldı (birkaç alanın ortak merkezi ilk alanın ilk köşesinden; etiket motorununki kendi sırasını
koruyarak). Kopyaların alan eşikleri ve alansız halkadaki davranışları olduğu gibi kaldı.

**Sınama.** Üç kopyanın sınaması düzeltmeden önce kırmızıydı (10 m'lik karenin merkezi 3,6 m kayık). Çekirdeğin altın
dosyaları (800 + 377 sınama, etiket motoru dahil) ve dokunulan crate'lerin bütün sınamaları değişmeden geçti.

**Bu sırada bulunan, bu işten bağımsız kırmızı sınamalar.**

- `file_digest` (uygulama) ve `bands` (baskı) sınamaları bağlı katmanı CBS projesinde kuruyordu; CBS projesi bağlı
  katmanı 23 Kasım'dan beri reddediyor (ADR 9196 §2). İki sınama CAD projesine çevrildi.
- `module_features` (`gis/lineage` özelliğinin arkasında değil) ve `ifc_revisions` (saklanan BCF'de kameranın bir
  değeri 16. hanede farklı) uygulama katmanında kırmızı. İkisi de bu dalın değiştirmediği koddan, 27 Kasım'daki
  başka işlerden; dokunulmadı, sahibine bildirildi.
- `scene3d_building_tests` tam koşuda ara sıra düşüyor: iki sınama aynı geçici klasörü kullanıyor.

### 10. Komutun dışından yazan yerler kilitli katman kuralına uyar (BK-19)

Kural (CLAUDE.md §7): belge değişikliği kilitli katman kurallarına uyar. Belgenin kendi yazıcıları (`Document::remove`,
`add`, `update`) kilide bakmaz; kilide komutlar bakar. Komutun dışından, belgenin yazıcısını doğrudan çağıran yer
kuralı atlar.

**Ölçüm.** Uygulama katmanında 26 `doc.remove` çağrısı var; 24'ü bir komutun yazma adımında ve komutun denetiminden
sonra. Komutun dışından silen dört yer bulundu (§10.1). Uygulama katmanının dışında belgeye doğrudan yazan tek başka
yer plan ölçüleri aracıdır (`interaction/src/arch/plan_dimensions.rs`); o kilide kendi bakıyor.

#### 10.1 Aracın kendi işaretleri: eskisi kilitli katmandaysa yenisi çizilmez

**Bulgu.** Dört araç her çalıştığında kendi işaretlerini yeniden çizer ve önce eskilerini siler: renk planı lejandı,
kaçış yolları, gölge, vaziyet planının ölçüleri. Eskiler taşıdıkları öznitelikle bulunup doğrudan `doc.remove` ile
siliniyordu. İşaretler kilitli bir katmandaysa (kullanıcı katmanı kilitleyip başka katmanı etkin yaptıysa) kilide
rağmen siliniyor, yenileri etkin katmana çiziliyordu.

**Karar.**

- Tek yardımcı: `kentos_native_application::marks::take_away`. Dört yazıcı onu çağırır.
- Eski işaretlerden biri kilitli katmandaysa hiçbiri silinmez ve yenisi çizilmez; araç katmanın adını ve çözümü söyler
  (“Eski lejant kilitli “Lejant” katmanında; yerine yenisi çizilemez. Kilidi Katmanlar panelinden açın.”). Örnek alınan
  karar mahal kaplamasınınkidir (`cad.spaces.finish`: eski kaplama kilitli katmandaysa yenisi yazılmaz).
- Kilitli olanları bırakıp yenisini çizmek seçilmedi: planda iki takım işaret kalırdı.
- Silme, silme komutunun kendisiyle yapılır (`cad.entities.delete`'in işleyicisi): her nesne için baktığı ilişki
  kurallarına işaret için de bakılır.
- Dört araç da yazdıklarını zaten tek geri alma adımında topluyordu (`begin_group`); ret adımı geri alır.

**Kalan (BK-19).** Dört aracın kendi komutu yok: MCP, Python, eklenti ve yapay zekâ ajanı lejant, kaçış yolu, gölge ya
da vaziyet ölçüsü çizemez. Komuta çevrilmeleri (§9.2 zinciri) Aşama 2'den sonradır; komut eklemek o zaman ucuzlar.

**Sınama.** Dört sınama düzeltmeden önce kırmızıydı (işaretler kilitli katmandan silinip öbür katmana çiziliyordu):
uygulama katmanında lejant ve gölge, masaüstünde kaçış yolları aracı ve parsel aracı.

#### 10.2 Dış başvuruyu kaldırmak kilitli katmandaki yerleştirmeyi silmez

**Bulgu.** `cad.xrefs.edit`'in `detach` işlemi dış başvurunun tanımını ve çizimdeki bütün yerleştirmelerini siler
(ADR 0348). Komut yerleştirmelerin katmanına bakmıyordu: kilitli katmandaki yerleştirme de siliniyordu. Dosyada hiç
kilit denetimi yoktu.

**Karar.** Yerleştirmelerden biri kilitli katmandaysa (kendisi ya da üstündeki bir grup kilitli) kaldırma bütünüyle
reddedilir: `layer_locked`, katmanın ve başvurunun adıyla, çözümüyle. Kilitli olanı bırakıp öbürlerini silmek
olmaz: tanımı giden yerleştirme çizimde kalamaz. Denetim sürüm denetiminden sonradır (silme komutunun sırası).
Boşaltma, yeniden yükleme ve bağlama yerleştirmeyi değil tanımı değiştirir; onlara dokunulmadı.

**Sınama.** Ortak duruma bağımsız üreteçten bir durum eklendi (`xref_command_cases.py`: yerleştirme kilitli
katmanda; ret, hiçbir şey yazılmaz). Düzeltmeden önce kırmızıydı (“removed: 1”). Rust çalıştırıcısı ve Python
kanalı aynı durumdan geçiyor; yetenek sınaması yeşil (kod komutun listesinde zaten vardı: ekleme katmanı için).

#### 10.3 Yol çizimi, nesnesi kilitli katmandaysa yeniden üretilmez

**Bulgu.** `road.project.rebuild` bir çizimi yeniden üretirken onun nesnelerini siler, sonra çizim komutunu yeniden
çalıştırır. Çizim komutunun denetimi yalnız çizimin kendi katmanına bakar. Çizimin bir nesnesi elle başka bir
katmana alınmış ve o katman kilitlenmişse, `edited` ile istenen yeniden üretim onu kilide rağmen siliyordu
(`run_drawing`, doğrudan `doc.remove`).

**Karar.** Çizimin silinecek nesnelerinden biri kilitli katmandaysa çizim yeniden üretilmez: `layer_locked`, çizimin
ve katmanın adıyla. Denetim çizim komutunun kendi denetiminden sonradır; kilitli hedef katman eskisi gibi komutun
kendi sözüyle söylenir. Yalnız değişeni yazan yol (`regional`) aynı denetimden geçer: iki yol aynı cevabı verir.
Kendiliğinden yeniden üretimde ret, öbür retler gibi uyarı olur ve çizim olduğu gibi kalır.

**Sınama.** Ortak duruma bağımsız üreteçten bir durum eklendi (`road_project_cases.py`: nesne başka katmana alınır,
katman kilitlenir, `edited` ile istenir; ret, hiçbir şey yazılmaz). Düzeltmeden önce kırmızıydı (“removed: 2”). Rust
çalıştırıcısı ve Python kanalı geçiyor. Üretecin yazdığı `road.project.query.json` son basamaklarda eskimiş çıktı
(bu işten önce de; üreteç ortamının farkı): dokunulmadı.

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
| Kütle (`tool.mass`) ve kütlenin düzenlenmesi (BK-16) | var | yok: Mimari'nin işi | yok | yok | var: `cad.masses.create`, `cad.arch.edit` | var: `kentos.cad.masses.create` | var: `run`, `document.write` izniyle | var: aynı komut |
| Çizime yazılan sayılar ve nokta-alan kararı (BK-17) | var | var | var | var | var: aynı çekirdek işlevi | var: aynı | var: aynı | var: aynı |
| Renk planı lejandı, Kaçış yolları, Gölge, Parsel'in ölçüleri (BK-19, §10.1) | var | yok: Mimari'nin araçları | yok | yok | yok: araçların komutu yok (BK-19'un kalanı) | yok: aynı | yok: aynı | yok: aynı |
| Dış başvuruyu kaldır (BK-19, §10.2) | var | var | var | yok: profilinde yok (CAD dış başvurusu) | var: `cad.xrefs.edit` | var: `kentos.cad.xrefs.edit` | var: `run`, `document.write` izniyle | var: aynı komut |
| Yol çizimlerini yeniden üret (BK-19, §10.3) | yok: Yol'un işi | yok | var | yok | var: `road.project.rebuild` | var: `kentos.cad.road.project.rebuild` | var: `run`, `document.write` izniyle | var: aynı komut |

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
