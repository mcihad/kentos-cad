# ADR 0546: Öznitelik tablosu: dışarı çıkanlar, süzgeçle istatistik ve kaynak satırı

- **Durum:** kabul edildi (2026-10-20). `plan/gis.md` CBS-20'nin ikinci dilimi ve CBS-40'ın Öznitelik tablosu kısmı. Kapsamı ben
  belirledim.
- **Bağlam belgesi:**
  - ADR 0507 (toplu yazma; “Sonraya kalan”ı bu ADR'nin başlangıcı);
  - 0199 (Öznitelik tablosu);
  - 0223 (tablo alışverişi, Dışa aktar ▾);
  - 0227 (İstatistik ve Grafik);
  - 0443 (alanın `redact`'ı);
  - 0532 (`cad.records.page`);
  - 0503, 0505 (bağlı katman);
  - 0543 (Kaynak durumu).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - sıralamanın, süzgecin ve toplamın kaynağa itilmesi ve imleçli sayfalar (§4'te tasarlandı, yazılmadı);
  - sütun başlığının menüsü ve hücrede Ctrl+C;
  - alan yetkisinin ortak çözücüsü (CBS-38): bu ADR yalnız formun `redact`'ını izler;
  - bilgi kartında kaynak satırı (kart bağlı katmanın nesnesi için aynı durumu söylesin);
  - katman süzgecinin geçen sayısı ve bulut taslağının durumu aynı satırda (CBS-40).

## Bağlam

Öznitelik tablosu gösterdiği satırları dosyaya yazabiliyordu (XLSX, ODS, CSV), ama:
- dışa aktarma tablonun bütün sütunlarını yazıyordu; formda `redact` işaretli bir alan (ADR 0443) Python'a kopyalanan çağrıda
  `‹gizli›` olurken tablodan dosyaya düz yazıyla çıkıyordu;
- tablonun sütunu gizlenemiyordu, gösterilen satırların panoya kopyası yoktu;
- İstatistik ve Grafik tablonun süzgecini bilmiyordu: Göster ▾'leri yalnız bütün nesneler, seçililer ve görünümdekilerdi, tabloda
  Ara, Göster ve İfade süzgeci ile daraltılmış satırlar o sekmelerde yeniden daraltılamıyordu;
- bağlı katmanın tablosunda kaynağın durumu yoktu: katman bir GeoPackage ya da PostGIS tablosunun çalışma kopyasıysa kullanıcı bunu
  ve ne beklediğini (yaz, oku, uzlaştır, yeniden bağla) tablodan göremiyordu; Kaynak durumu penceresi (ADR 0543) ayrıydı.

Plan (CBS-20) bunları ister: “gizlenen kolonun değerleri kopyalama/export/AI üzerinden sızmamalı”, “grafik/istatistik aynı kaynak
filtresini kullansın”, “yüklü, seçili, filtreli ve tüm dataset kapsamlarını açık sayılarla ayır”.

## Karar

### 1. Tablodan çıkan: Sütunlar, Kopyala, Dışa aktar

- **Sütunlar ▾** tablonun sütunlarını tek tek gösterir ve gizler (menü açık kalır, Hepsini göster). Gizleme katman başına ve
  oturum içindir, çizime yazılmaz. Tür sütunu gizlenmez. Gizlenen sütun tabloda, kopyada, dosyada ve Yapıştır'ın sütun sırasında
  yoktur: Yapıştır panodaki başlıksız hücreleri tablonun gösterilen alan sütunlarına sırayla yazar.
- **Tabloya çıkan iki küme** (çekirdekte, `kentos_interaction::feature_table`):
  - `visible_columns(tablo, gizlenenler)`: tablonun gösterdiği sütunlar;
  - `outgoing_columns(tablo, gizlenenler, redact)`: gösterilenlerden formun `redact` işaretlediği alanlar (alanın anahtarı, adı ya da
    başlığı) çıkarılmış hâli ve çıkarılanların başlıkları (`withheld`). Gizlenmiş bir sütun sessizce dışarıdadır, `redact`'lı sütun
    tabloda görünür ama dışarı çıkmaz ve söylenir.
- **Kopyala** (`copy_text`): tablonun gösterilen satırlarını, çıkabilen sütunlarıyla, ilk satırı alan adları olan sekmeyle ayrılmış
  metin olarak panoya yazar; Yapıştır'la geri yapıştırılabilir. Tür sütunu kopyada yoktur (hiçbir alan taşımaz). Hücredeki sekme ve
  satır sonu boşluk olur.
- **Dışa aktar ▾** (`workbook_sheet_of`): aynı satırlar ve sütunlar, ADR 0223'ün biçimleriyle; önceden bütün sütunları yazan
  `workbook_sheet` bunun bütün sütunlarla çağrısıdır.
- “Gösterilen satırlar” tablonun Ara, Göster (Tümü, Seçililer, Görünümdekiler), İfade süzgeci ve sıralamasının sonucudur; kopya ve
  dosya onu olduğu gibi izler. Dışarıda bırakılan alan varsa bildirimle söylenir: “Formda gizli işaretli n alan dışarı yazılmadı:
  …”.
- Her formun taslağı ve yayınları taranır (`python_call::redacted_names`); işaret herhangi birinde varsa alan çıkmaz.

### 2. İstatistik ve Grafik tablonun süzgeciyle

- İstatistik ve Grafik'in Göster ▾'ine dördüncü seçenek: **Tablonun süzgeciyle** (`Show::Table`). Satırları, tablonun gösterdiği
  satırlardır; aynı değerlendirme (`feature_rows`, ADR 0120'nin önbelleğiyle), ikinci bir uygulama yok.
- Tablonun çubuğunda **İncele ▾**: İstatistik (tablonun süzgeciyle) ve Grafik (tablonun süzgeciyle). İkisi sekmeyi tablonun
  katmanında ve Göster = Tablonun süzgeciyle açar.
- Katman: seçenek tablonun katmanını okur. Tablonun katmanı değişirse sekmeler ona gelir; sekmenin kendi katmanı seçilirse Göster
  Tümü'ne döner. Tablo başka bir katmandaysa seçenek o katmanın bütün nesnelerine düşer.
- Önbellek anahtarı tablonun seçimlerini içerir (Ara, Göster, süzgeç değişince `FeaturesPanel.version`; tablonun Göster'i
  Seçililer ya da Görünümdekiler iken seçimin ya da görünümün sürümü), yani tablo değişince sekmeler yeniden hesaplanır, başka
  zaman değil.
- **Redact'lı alan** İstatistik ve Grafik'in veri kümesinden çıkarılır (özet tablosu, grafiğin sütunları, Panoya kopyala, PNG ve
  SVG): sayıların toplamı da gizli değeri sızdırır (CBS-38 “count/aggregate aynı projection'a tabi”).

### 3. Kaynak satırı

- Bağlı katmanın tablosunda çubuğun altında bir satır: **Kaynak** — kaynak (dosya › tablo, salt okunursa söylenir), durum, kapsam,
  gösterilen ve seçili sayısı, sistem; durumun istediği tek işin düğmesi; Yenile.
- Durum `cad.sources.describe`'ın durumudur: `sources::layer_state(doc, katman)` aynı `state_of`'u çağırır; kural
  (`SourceNext::of`) ve kelimeler (`source_words`, `state_words`, `scope_words`, `system_words`, `act_words`) Kaynak durumu
  penceresinin aynısıdır, bir daha yazılmadı. Düğmenin işi `source_status::way_out(next, katman)`: Kaynağa yaz, Kaynaktan oku,
  Uzlaştır…, Yeniden bağla… (veritabanında Adresi denetle). Salt okunur tabloda yazma düğmesi kapalıdır.
- **Dört kapsam ayrı sayılır:** katmanın tuttuğu nesneler ve satırlar ile bütün tablonun satırları (`scope_words`: “n nesne · yüklü /
  toplam”), süzgeçten geçip gösterilenler, seçililer.
- **Okuma tablonun dosyasını okur**, bu yüzden saklanır ve yalnız gerektiğinde yeniden okunur: tablodaki katman, bağın özeti ve
  sürümü (yazma ve okuma bunları değiştirir), çizimin oturumu ya da Yenile değişince; ve en çok `LIVE_MOST` (20 000) nesneli katmanda
  çizim her değiştiğinde (katmandaki düzenleme hemen “Katmanda yazılmamış” olur). Daha büyük katmanda son okuma tutulur ve
  “Katman değişti; durum eski” denir, Yenile'ye basılır: her düzenlemeden sonra milyon nesnenin özetini almaz.
- Düğme Kaynak durumu penceresinin olaylarını çalıştırır ve ardından durumu yeniden okur. Bağlı olmayan katmanda satır yoktur.

### 4. Kaynağa itilen sıralama, süzgeç ve sayfalar (tasarım; yazılmadı)

Bugün bağlı katman tablonun çalışma kopyasıdır: nesneleri çizimdedir (alan sınırlı kopyada alanın içindekiler), Öznitelik tablosu
sıralamayı, süzgeci ve sayımı bellekte o nesneler üzerinde yapar. `cad.records.page` da çalışma kopyasını okur (kalıcı kimlik
sırasıyla, kaynağın sürümüyle). Bu, katman tablonun hepsini tuttuğu sürece doğrudur ve milyon satırda bellekte çalışır
(`docs/perf/provider-2026-10-18.md`). Kaynağa itmek, katmanın tablonun yalnız bir penceresini tutması demektir; bu iş sağlayıcının
sayfa arayüzü gerektirir ve bu ADR'nin kapsamını aşar. Tasarım:

- **Sağlayıcı arayüzü:** `TableProvider::page(sıralama, yüklem, imleç, sınır) -> Page { satırlar, eşleşen, sonraki }`.
  - Sıralama sütun ve yön listesidir; her sıranın sonunda kalıcı kimlik (ya da `rowid`) bağı bozar, böylece sıra tam belirlidir.
  - Yüklem yalnız kaynağın deyimine çevrilebilen altküme: karşılaştırmalar, `and`, `or`, `not`, `in`, `boş`, `gibi`. Çevrilemeyen ifade
    istemcide süzülür ve “kaynakta süzülemedi: n satır istemciye indi” denir; sessizce eksik sonuç verilmez.
  - Metin sıralaması ve karşılaştırması Türkçe katlama kuralıyla olur: GeoPackage'ta ilgili sütunda tanımlı harmanlama, PostGIS'te
    `tr-TR` ICU harmanlaması; iki taraf tablonun gösterdiği sırayı vermezse itme açılmaz.
- **Kararlı imleç:** imleç sıralama anahtarlarının son satırdaki değerleri ve kimliğidir (keyset), `OFFSET` değil; kaynağın
  sürümüne (SHA-256 ya da PostGIS için işlem anlık görüntüsü) bağlıdır. Sürüm değişince sonraki sayfa okunmaz (`source_changed`),
  sayfa atlanmaz ya da tekrar etmez. `cad.records.page`'in `cursor`, `expectedSource`'u bunun bugünkü biçimidir.
- **Sayım:** `eşleşen` sağlayıcının `count(*)`'ıdır, aynı yükleme göre; tablo dört kapsamı kaynağın söylediğiyle gösterir.
- **Düzenleme:** penceredeki satır çizimde nesne olur, pencerenin dışındaki satıra yazılmaz (ADR 0510'un alan kuralı gibi); bekleyen
  değişiklik kaynağa yazılırken denetlenir.
- **Sıra:** önce sağlayıcının sayfa arayüzü ve PostGIS'te sıralama (CBS-04), sonra tablonun penceresi ve bu tasarım.

## Uygulama

- Çekirdek: `kentos_interaction::feature_table`: `visible_columns`, `outgoing_columns`, `Outgoing`, `copy_text`,
  `workbook_sheet_of`.
- Uygulama katmanı: `sources::layer_state`.
- Masaüstü:
  - `features/outgoing.rs` (Sütunlar, Kopyala, Dışa aktar, İncele);
  - `features/source.rs` (kaynak satırı ve önbelleği);
  - `source_status::way_out`;
  - `Show::Table` (İstatistik ve Grafik; `LayerData::rows`, `table_rows_stamp`, `redacted_fields`);
  - tablonun başlık, sıralama, Tab gezintisi ve Yapıştır'ı gösterilen sütunlarla.
- İkonlar (sahibe sorulmadan seçildi): Sütunlar `columns`, Kopyala `tableGridAll`, İncele `chartBar` (İstatistik `chartHistogram`, Grafik
  `chartPie`).
- ADR 0543'ün gpkg testi `a_table_bound_to_a_layer_is_written_back_and_read_again` Kaynak durumu'nun pencere olmasından beri bir
  günlük satırı arıyordu; pencerenin satırını denetler oldu.
- Python ve MCP: değişmedi (komut eklenmedi, `cad.sources.describe` aynı).

## Doğrulama

- `cargo test -p kentos-interaction feature_table`: gizleme, `redact`, kopya ve dosya sayfasının testleri.
- `cargo test -p kentos-desktop features::tests`:
  - `a_hidden_column_is_out_of_the_table_the_copy_and_the_file_and_a_redacted_field_is_out_of_the_last_two`;
  - `statistics_and_charts_over_the_tables_rows_follow_its_filter_and_leave_redacted_fields_out`.
- `cargo test -p kentos-desktop exchange::gpkg_tests::the_table_of_a_bound_layer_says_its_state_and_takes_the_way_out`: dört
  bağlı katmanda Uzlaştır, Yaz, Aynı, Yeniden bağla; Kaynağa yaz dosyayı yazar; katman düzenlemesi hemen Yaz olur; Uzlaştır… kendi
  penceresini açar; bağlı olmayan katmanda satır yok.
- Resimler (1440×900 ve 1100×650, koyu ve açık):
  - `cargo test -p kentos-desktop features::tests::outgoing_screens -- --ignored --nocapture`: `.run/shots/oznitelik-sutun-*`,
    `oznitelik-istatistik-*`, `oznitelik-grafik-*`;
  - `cargo test -p kentos-desktop exchange::gpkg_tests::table_source_screens -- --ignored --nocapture`:
    `.run/shots/gpkg-tablo-kaynak-*`.
