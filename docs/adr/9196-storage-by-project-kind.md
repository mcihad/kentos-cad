# ADR 9196: Saklama modeli proje türündendir: CBS verisi veritabanında, CAD projesi proje dosyasında

- **Durum:** kabul edildi (2026-11-23); plan [plan/ayrim.md](../../plan/ayrim.md) AY-01 (karar), AY-05 ve AY-06
  (uygulama). ADR 9107'nin “CBS'de veri her hâlükârda veritabanında” sözünü kural yapar. ADR 9184'ün saklama
  kartlarını ve ADR 0031'in iki bulut biçiminin her türe açık olmasını değiştirir.
- **Güncelleme (2026-11-24, [ADR 9302](9302-cbs-new-project-simple.md)):** sihirbaz veri yerini ve proje dosyasının
  yerini sormaz, tablo kurmaz; CBS projesi boş açılır (yalnız Açıklama). Tablolar Veri ▸ Yeni katman ile kurulur
  (yeri sorulur), var olanlar Harita ▸ Veri ekle ile eklenir; proje dosyasının yeri ilk Kaydet'te sorulur. §2'nin
  sihirbazdan söz eden yerleri ve `kentos_project::cbs_tables` geçmiştir.
- **İstek:** sahibin sözleri (23 Kasım 2026), sırasıyla:
  - “gis projelerinde veriler tamamen veri tabanına kaydedilir öz nitelikler de veritabanına kaydedilir proje dosyası
    sadece katman özelliklerini projenin yapısını ve diğer ayarları tutar bu buluta da kaydedilebilir dosyaya da
    kaydedilebilir veri tabanına da kaydedilebilir proje dosyası. Ama tüm katman verileri ve öz nitelikleri
    veritabanında tutulur, ama Cad tarafında projenin tamamı proje dosyasında tutulur dosyaya da kaydedilebilir
    buluta da kaydedilebilir proje dosyası”;
  - açıklamalar nerede durur sorusuna: “Proje dosyasında ama proje dosyası qgis gibi veritabanına da kaydedilebilir
    olmalı”;
  - bulutta CAD sorusuna: “Yalnız proje dosyası”;
  - eski projeler sorusuna: “Şu anda halen development aşamasında uygulama henüz Prod çıkmadı”. Eski projelere
    taşıma yolu yazılmaz; örnekler ve sınamalar yeni kurala çevrilir.
- **Kural:** CLAUDE.md §25 madde 5.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** biçim değişmez. Kural, CBS projesinin kaydına ve bulut
  türlerine uygulanır (AY-05, AY-06).

## §1 Bağlam

**Bugün olanlar:**

- [ADR 9107](9107-project-in-a-source.md):
  - proje bir GeoPackage'ta ya da PostgreSQL şemasında saklanabilir (`kentos_data::ProjectStore`);
  - CBS'de yeni katman bir tablodur (`cad.layers.create`);
  - eski çizim katmanı kaynağa taşınabilir (`cad.layers.migrate`).
- [ADR 9129](9129-data-layers-in-source-status-and-table-picker.md): veri katmanının satırları kaynağındadır.

**Kurala uymayan yollar:**

- Sihirbazın Boş kent haritası (Taslak) ve Kadastro paftası ağacı proje dosyasına yazan çizim katmanlarıdır
  (`crates/native/project/src/new_project.rs`).
- Sihirbazın “Dosya (.kcad)” kartı ve notu, içe alınan verinin projeye kopyalandığını söyler (ADR 9184).
- CBS'de içe alma nesneleri projeye kopyalar (ADR 9192'nin “alınmış kopya”sı).
- CBS'de bağlı katman nesnelerini projede tutar ve kaynağa yazar ([ADR 0543](0543-source-status-and-way-out.md)).
- Bulutta CBS projesi dosya biçiminde (revizyon), CAD projesi canlı veritabanı biçiminde olabilir
  ([ADR 0031](0031-cloud-file-projects.md); dönüşüm [ADR 0039](0039-storage-conversion.md)).

**Araştırma** (23 Kasım 2026'da okundu):

- **Esri, *Projects in ArcGIS Pro*:**
  - proje dosyası haritaları, sahneleri, sayfa düzenlerini ve verinin temsillerini tutar;
  - proje paketi veriye başvurur, verinin kendisini saklamaz;
  - verinin yeri projenin geodatabase'idir.
  <https://pro.arcgis.com/en/pro-app/latest/help/projects/what-is-a-project.htm>
- **QGIS, *Working with project files*** (ADR 9107 §1): proje PostgreSQL'de ve GeoPackage'ta saklanabilir
  (`qgis_projects` tablosu); katmanlar kaynaklarına başvurur.
- **QGIS API, `QgsAnnotationLayer`:** açıklama katmanı haritaya konan işaret, çizgi, alan ve yazı öğelerini tutar ve
  proje dosyasına yazılır. Açıklama, verinin değil projenin parçasıdır.
  <https://qgis.org/pyqgis/3.36/core/QgsAnnotationLayer.html>

## §2 Karar

1. **İki saklama türü vardır.** Her modül birini seçer; seçim ortak modül tanımında durur (ADR 9195 §2.5). Üçüncü
   bir biçim ADR ister.
2. **CBS (veri veritabanında).**
   - **Veri:** her katmanın nesneleri ve öznitelikleri bir veritabanının tablosudur: GeoPackage, PostgreSQL ya da
     KentOS Bulut.
   - **Proje dosyası:** katmanların yapısını ve stillerini, formları, ilişkileri, kuralları, ayarları ve açıklamaları
     (yazı, ölçü, pafta) tutar.
     - Proje dosyası `.kcad` olarak, veritabanının içinde (GeoPackage ya da PostgreSQL, `ProjectStore`) ya da
       bulutta saklanır.
     - Bulutta saklanan proje dosyası yalnız bulut verisiyle olur.
   - **Sihirbaz iki şey sorar:** verinin yeri (öntanımlı: proje adıyla bir GeoPackage) ve proje dosyasının yeri.
     Amaçlar (Boş kent haritası, Kadastro paftası, Kurum şablonu) katmanlarını veri deposunda tablo olarak kurar.
   - **İçe alma** veriyi projenin veri deposunda bir tabloya yazar ve veri katmanı olarak ekler (kaynak kaydı ADR
     9192). Bağlama dış tabloya veri katmanıdır.
   - **Bağlı katman CBS'de sunulmaz.** Çizim araçları yalnız veri katmanına (düzenleme oturumu) ya da açıklama
     katmanına çizer.
   - **Kayıt kuralı:** CBS projesinin kaydı, açıklama dışında nesne tutan katmanı proje dosyasına yazmaz. Nedenini
     ve Veri kaynağına taşı'yı söyler.
   - **Bulutta** CBS projesi yalnız veritabanı biçimidir.
3. **CAD (proje dosyasında): Mimari, Harita, Yol.**
   - **Projenin tamamı** (nesneler, öznitelikler, bloklar, paftalar, ayarlar) proje dosyasındadır.
   - **Saklama:** proje dosyası `.kcad` olarak ya da bulutta saklanır. Bulutta yalnız dosya revizyonudur: canlı
     veritabanı biçimi CAD türlerinde (`cad`, `architecture`, `zoningPlan`, `landReadjustment`, `subdivision`,
     `road`) reddedilir. Aynı anda iki kayıt dosyanın revizyon çakışmasıdır (ADR 0031).
   - **CBS tabloları** Harita ve Yol'da salt okunur altlık olabilir. Harita'nın bağlı katmanı teslimdir: nesneleri
     proje dosyasında durur, ayrıca kaynağa yazılır.
4. **Geliştirme aşaması.** Eski CBS projelerini taşıyan bir yol yazılmaz. Nesneli CBS çizim katmanı taşıyan örnekler
   ve sınamalar kurala çevrilir.

## §3 Uygulama sırası

- **AY-05:** sihirbaz, amaçların tabloları, içe alma, bağlı katman, çizim araçları, kayıt kuralı, CBS'nin buluta
  yüklenmesi, örnekler.
- **AY-06:** sunucu CAD türlerinde veritabanı biçimini reddeder; masaüstünde yükleme ve dönüşüm; Python ve MCP.

## §4 AY-06: CAD projesi bulutta yalnız dosya revizyonu (23 Kasım 2026)

Araştırma §1'dedir; bu madde kararı sunucuda ve istemcilerde uygular.

**Kuralın kaynağı:** ortak modül tanımı (`kentos_launch::Module::storage`, ADR 9195 §6).

- Sunucu proje türünü modüle çevirir (`Module::of_project_type`) ve saklama türünü oradan alır
  (`kentos_application::projects::storage_of_type`).
- `scripts/arch/deps.mjs`'te sunucu grubunun bağımlılıklarına `launch` eklendi; `kentos-launch` yalnız serde'dir.

**Sunucu** (`crates/server/application`):

- **`project.create`:**
  - Saklama biçimi verilmezse türünkü kullanılır: CAD türleri dosya, CBS veritabanı. Önceden her tür veritabanıydı.
  - CAD türünde veritabanı istenirse `storage` alanıyla reddedilir. İleti, ör. Yol için: “Yol” türündeki bir
    projenin tamamı proje dosyasındadır; bulutta yalnız dosya revizyonu olarak saklanır. Saklama biçimini dosya seçin.
  - Türün adı Bulut projeleri'nin adıdır (Genel CAD, 18 uygulaması, İmar planı, İfraz / tevhit, Yol, Mimari).
- **`project.convert`:** CAD türündeki dosya projesinin veritabanına dönüştürülmesi `to` alanıyla, aynı nedenle
  reddedilir. Veritabanındaki eski bir CAD projesi dosyaya dönüştürülebilir; çıkış yolu açık kalır.
- **`project.metadata.update`:** veritabanındaki bir projenin türü CAD türüne çevrilmez; `projectType` alanıyla
  reddedilir ve önce dosya projesine çevirmeyi söyler. Tür artık saklamadan bağımsız bir etiket değildir
  (`ProjectSummary.project_type`'ın belgesi).
- **Kopya ve geri yükleme** kaynağın biçimini korur. Eski CAD veritabanı projeleri olduğu gibi kalır; taşıma yolu
  yazılmaz (§2.4).
- **Sözleşme:** `ProjectCreate.storage`'ın açıklaması kuralı söyler. TS türü, komut kataloğu, OpenAPI ve Python
  SDK'sı yeniden üretildi.

**Masaüstü:**

- **Buluta yükle:**
  - CAD türünde yalnız Dosya seçeneği vardır; altında neden yazar.
  - Tür CAD'e çevrilince saklama dosyaya döner. Pencere veritabanı istense de CAD türünde dosyayla açılır
    (`cloud::kinds::storages_of`).
- **Bulut projeleri:** CAD türündeki dosya projesinde “PostGIS'e aktar…” kapalıdır ve nedenini söyler.

**Python ve MCP:** sunucunun kuralı onlara da uygulanır. SDK'nın `storage` belgesi kuralı söyler.

**Kanıt:**

- **Sunucu sınamaları:**
  - Yeni: `project_convert::a_cad_types_project_is_kept_as_its_file`. Altı CAD türü saklamasız dosya olur,
    veritabanında `storage` ile reddedilir; CBS veritabanıdır; Yol'un dosya projesi veritabanına dönüştürülmez.
  - Veritabanı biçimine dayanan sınamaların projesi CBS projesi oldu: ortak `new_project` ve altı sınama dosyası.
    Kabul setinin veritabanı koşusu da CBS projesiyle yapılır.
  - `project_convert`'in dosya projeleri CBS projesidir.
  - Katalog sınamaları genel CAD türünde dosya projesiyle kurar (`new_cad_project`); veritabanındaki CBS projesinin
    Yol türüne çevrilmesi reddedilir.
- **API sınamaları:** veritabanı projesi kuran 19 istek `"projectType": "gis"` ile kurar; sunucu işlerinin sınaması
  (`job_run_db`) da CBS projesiyle.
- **Masaüstü sınamaları:**
  - `kinds_tests`: Mimari'nin yüklemesi dosyadır, veritabanı seçilemez.
  - `drafts` ve `renaming`: genel CAD çiziminin yüklemesi dosyayla açılır; veritabanı yüklemesi CBS türüyle yapılır.
  - `catalog_forms_tests`: genel CAD dosya projesinde Dönüştür kapalıdır ve nedenini söyler; CBS dosya projesi
    veritabanına aktarılır.
  - Web'le ortak bölme örneği (`fixtures/cloud/v1/catalog.json`): imar planının dosya projesinde “PostGIS'e aktar…”
    nedeniyle kapalı; yeni “file-project-cbs” durumunda açık.
  - Canlı sınamaların yardımcısı da veritabanında CBS türünü seçer.
- **Ortama bağlı kalan:** `kentos-api`'nin üç sunucu denetim işi sınaması (`job_run::audit*`) macOS'ta denetim kutusu
  kurulamadığı için düşer (“Landlock yalnız Linux'ta”); saklama kuralıyla ilgisizdir.

## §5 AY-05: CBS projesinin verisi veritabanında (23 Kasım 2026)

Araştırma §1'dedir: ArcGIS Pro'nun projesi veriye başvurur, veri geodatabase'dedir; QGIS projeyi GeoPackage'ta ya da
PostgreSQL'de tutar; QGIS'in açıklama katmanı projenin parçasıdır. Bu madde kararı sihirbazda, kayıtta, içe almada,
bağlamada ve bulutta uygular.

**Sihirbaz** (`crates/native/project/src/wizard.rs`, `apps/desktop/src/shell/project/wizard/`):

- İki soru sorar:
  - **Veri:** GeoPackage (öntanımlı; proje adıyla, açık çizimin klasöründe ya da Belgeler'de), PostgreSQL (kayıtlı
    bağlantı ya da adres, şema; parola adrese yazılmaz) ya da KentOS Bulut.
  - **Proje dosyası:** `.kcad` (GeoPackage'ın yanında), GeoPackage'ın ya da şemanın içinde (ADR 9107) ya da bulutta.
    Bulut proje dosyası yalnız bulut verisiyle olur.
- Amaçların katmanları tablodur (`kentos_project::cbs_tables`):
  - Kadastro paftası: ada, parsel ve yapı alan; yol ekseni, kaldırım, eşyükselti ve ana eşyükselti çizgi; kot ve
    poligon nokta tablosudur. Yükseklik nesnenin Z'sidir. Nesnenin kendi etiketini yazan katman `etiket` sütununu
    okur (`{etiket}`).
  - Taslak, pafta, karelaj ve yazı Açıklama türündedir ve proje dosyasında kalır. Boş kent haritasının tek katmanı
    Taslak'tır, o da açıklamadır.
  - Kurum şablonunun türlü katmanı tablo, türsüz katmanı açıklamadır.
  - Açıklama türü yazı, ölçü, kılavuz, tablo, etiket, tarama, blok, resim ve görünüm işareti alır (ADR 0622 §5).
    Çizgi, alan ve nokta veri katmanına (düzenleme oturumu) çizilir; etkin katman açıklamaysa o araçlar nedenini
    söyleyerek kapalıdır.
- **Oluştur:**
  - tabloları, ekrandaki çizimden ayrılma sorusu yanıtlandıktan sonra ve proje açılmadan kurar
    (`layers_create::new_project_table`); Vazgeç tablo bırakmaz;
  - her tablo katmanının düğümüne onu okuyan veri katmanını koyar;
  - projeyi yerine hemen yazar: `.kcad`'e, GeoPackage'a ya da şemaya.
  - KentOS Bulut'ta tablo kurulmaz; Buluta yükle açılır, proje ve verisi birlikte gider.
- Özet “Veri” ve “Proje dosyası” satırlarını söyler. Ortak örnek (`fixtures/project/v1/wizard.json`) yenilendi.

**Katman türü ve kayıt kuralı** (ortak: `kentos_native_application::layer_kind`):

- Katmanın türünü tek yer söyler: grup, servis, veri, bağlı, açıklama, yalnız raster, yalnız nokta bulutu, çizim
  (`kentos_launch::LayerKind`, ADR 9197 §2.4). Katman menüsü (AY-03) ve her istemcinin kaydı onu sorar.
- CBS projesinin kaydı nesne tutan çizim ya da bağlı katmanı proje dosyasına yazmaz. Açıklama, veri ve servis
  katmanları, raster ve nokta bulutu (pikselleri ve noktaları kendi dosyalarındadır) proje dosyasındadır.
- **Masaüstü:**
  - Kaydet (dosyaya ya da veri kaynağına) reddedilir; ileti katmanları adıyla ve çıkış yolunu söyler.
  - İlk katmanda Veri kaynağına taşı… açılır.
  - Kopya kaydı ve kurtarma kopyası kuralın dışındadır: veri kaybettirmezler.
- **Python ve MCP:** `Session::save` ve `save_to_source` aynı kuralla `data_held_in_file` koduyla (Host) reddeder;
  ileti `cad.layers.migrate`'i söyler.

**İçe alma:**

- Masaüstünde CBS projesine içe alınan (içe alma penceresinden gelen) yeni katman aynı adımda projenin veri yerine tablo olur
  (`cad.layers.migrate`; her katman bir geri alma adımı). Veri yeri sırasıyla:
  - projenin saklandığı GeoPackage ya da şema;
  - yoksa ilk veri katmanının kaynağı;
  - yoksa proje dosyasının yanındaki GeoPackage.
- Yer bilinmiyorsa ya da katman taşınamıyorsa (kilitli, karışık türlü) katman kalır, nedeni söylenir, kayıt onu sorar.
- GeoPackage penceresinde CBS'de “Bağ” yoktur. Yerinde “Veri katmanı: Dosyada bırak, veri katmanı olarak ekle” vardır:
  tablolar dosyada kalır ve okunur (`cad.layers.data`). Kapalıyken İçe aktar tabloyu projenin veri yerine yazar.
  Başka sistemdeki tabloyu veri katmanı kendi sisteminden çevirerek çizer.

**Bağlı katman:**

- `cad.sources.edit`'in `link`'i CBS projesinde `bound_in_gis` koduyla reddedilir; kural komuttadır, her istemciye
  uyar. İleti `cad.layers.data`'yı söyler. Ortak durumu `cad.sources.edit.json`'dadır (üretici
  `scripts/fixtures/bound_sources_cases.py`; AY-08'in diliminde eklendi, yetenek sınaması onu istedi).
- Şerit: `source.link` CBS'nin Kaynak panelinden çıktı; Harita ve Yol'un “Ekle ▸ Harita” panelinin ▾'sindedir.
  Var olan bağlı katmanın komutları (yaz, oku, durum) kalır; eski projeye taşıma yolu yazılmaz (§2.4).

**Bulut:**

- **Sunucu** CBS projesini yalnız veritabanında tutar:
  - oluşturmada dosya biçimi `storage`,
  - dönüşümde dosyaya çevirmek `to`,
  - dosya projesinin türünü CBS'ye çevirmek `projectType` alanıyla, nedenini söyleyerek reddedilir.
  - Eski CBS dosya projesinin veritabanına dönüşümü açıktır (çıkış yolu).
- **Masaüstü:**
  - Buluta yükle CBS türünde yalnız veritabanını sunar ve nedenini yazar.
  - Bulut projeleri'nde CBS veritabanı projesinin “Dosya projesine çevir…”i kapalıdır, nedenini söyler.
- **Yükleme veri katmanlarının satırlarını aktarır** (`kentos_native_application::data_take`):
  - her veri katmanının bütün satırları kaynağından okunur ve yüklemenin kopyasında katmanın nesneleri olur;
  - değerleri özniteliktir, geometrisiz satır kayıttır, geometri projenin sistemine çevrilir;
  - katalogdaki katman ağacı dosyanınkiyle aynıdır (`data_take::plain`);
  - çizimde karşılığı olmayan satır yüklemeyi durdurur ve söylenir (sessiz kayıp yok);
  - bilgisayardaki proje değişmez.

**Kanıt:**

- **Masaüstü** (`RUST_MIN_STACK=67108864`):
  - `cbs_storage_tests`: kayıt reddedilir, Veri kaynağına taşı açılır, taşınınca kaydedilir; bağlama komutta
    reddedilir; buluta giden kopyada satırlar nesne olur.
  - `exchange::gis_tests::an_import_into_a_cbs_project_goes_into_its_tables`,
    `exchange::gpkg_tests::a_cbs_projects_geopackage_tables_are_added_as_data_layers`.
  - Sihirbaz sınamaları (veri yeri, tablolar, hemen kayıt). Modül, şerit, katman menüsü, bulut, katalog ve ayar
    takımları geçti.
  - Bulutta dosya projesinin yerel kaydı CAD projesiyle sınanır (`file_follow_tests`).
- **Ortak:** `layer_kind`'ın birim sınaması; başsız oturumun sınaması (`a_cbs_projects_file_takes_no_drawing_layers_objects`);
  paket durumlarının bağlı katmanları CAD çiziminde kurulur.
- **Bağlı katmanın ortak durumları Harita (CAD) projesindedir:**
  - 19 örnek dosyasının kurulumu (`cad.sources.*`, `cad.records.page`, `cad.forms.submit.commit`,
    `cad.topology.check`, `cad.topology.exceptions`) CAD'dir; dört dosyada (`cad.gis.derivation`, `cad.gis.detach`,
    `cad.processing.run.source`, `cad.processing.batch.sink`) yalnız bağlayan 8 durum kendi CAD kurulumunu taşır.
  - Üreteçler aynı değişikliği yapar ve `--check`'te tutarlıdır (Homebrew'un GDAL'lı Python'u, mpmath).
  - CAD projesinde işlemin yeni katmanı türlü alan taşımaz (ADR 1070 CBS'nindir): dışa yazılan sütunlar ad
    sırasıyladır; beklenen sıra buna göre.
  - Uygulamanın `sources`, `reconcile`, `source_relations` sınamaları ve Python'un üç sınaması da CAD belgesinde
    bağlar.
  - Sonuç: 22 475 ortak durumun hepsi geçer.
- **Ortak kabul senaryosu “Ada: parseller ve binalar” (CBS-44, `fixtures/acceptance/v1/parcels.json`) Harita'nın
  projesidir:** nesne olarak çizilen parseller ve GIS biçimlerine teslim Harita'nın işidir; senaryonun tek farkı proje
  türüdür (`cad`). Python, MCP ve masaüstü onu yeniden oynatır ve geçer; PostGIS'li oynatıcı (`#[ignore]`) koşulmadı.
- **Python ve MCP'nin kaydeden sınamaları** (`test_document`, `test_forms`'un iki sınaması, MCP'nin `mcp.rs` ve
  `source_relations.rs`'i) CAD belgesiyle kaydeder; CBS belgesinin kaydının reddi başsız oturumun ve `test_document`'ın
  sınamasındadır. Python'da ret `FileError`'dır (`data_held_in_file`).
- **Örneklerin gidiş-dönüşü** (`test_carriage`): nesneli CBS örneklerinin ve profil projelerinin kaydı reddedilir, baytları
  onları bütün taşır. Karşılaştırma veri katmanlarının kayıtta yazılan `relative` yolunu da ayıklar (ADR 9117 §3).
- **Python takımı** (877 sınama): bu işin dokunmadığı dört yerde hata kalır: Harita çalışma verisinin (`mapWork`)
  yedi örnekte kayıttan sonra farklı dönmesi, tipli katmandan geçen BIM, teslim, form yanıtı ve imar parseli
  durumları (253 durum; Rust'ta aynı örnekler geçer), profil listesine eklenen iki yerleşik profil ve panel
  kurucusunun `split` alanı.
- **Sunucu** (geçici PostGIS'te `kentos-application`'ın 43 sınama dosyası):
  - `project_convert`: CBS dosya olarak oluşmaz, dosyaya çevrilmez; eski CBS dosya projesi ve eski CAD veritabanı
    projesi sınamada SQL'le kurulur (çıkış yolları açık).
  - `project_catalog`: dosya projesi CBS türüne çevrilmez. AY-06'dan düşük kalan iki sınama (dosya projesine
    `project.changes` ile nesne yazıyordu) nesnelerini CBS veritabanı projesine yazar.
  - `project_duplicate`: veritabanı projesinin türü CAD'e çevrilmez; kopya CBS kalır.
  - `project_snapshot`'ın dosya projesi CAD türündedir.
- **Üretilenler:** sözleşme kodları (`bound_in_gis`, `data_held_in_file`), komut kataloğu, TS türleri, OpenAPI,
  Python SDK, `catalog/v1/modules.json` ve şerit. CAD manifestosu yeniden yazıldı; önceki bir işten (ADR 9127)
  güncellenmemiş kalan bina dışa aktarma ve kullanıcı koordinat sistemi satırları da CAD'e geçti.
- **Bağımlılık:** uygulama çekirdeği `kentos-launch`'a bağlandı (katman türü); `scripts/arch/deps.mjs`'te onu kullanan
  gruplara `launch` eklendi. Denetimin `crates/vendor/block` bulgusu önceden vardı.

## §6 Kalan

- Python ve MCP'de yeni CBS projesi (`NewDrawing`) veri yeri sormaz: katmanları çizim katmanıdır ve kayıt onların
  nesnelerini kaydetmez; `cad.layers.migrate` ile taşınır. Sihirbazın veri yerinin betik karşılığı ayrı bir dilimdir.
- Bağlı katmanın taşınması (`cad.layers.migrate`) bağını kaldırmaz; eski projelerdedir (§2.4).
- CBS projesinde bir işlemin (İşlemler) yeni çıktı katmanı proje dosyasına yazılır; kayıt onu sorar. Çıktı bugün de
  bir tabloya yazılabilir (çıktının yeri); içe almadaki gibi kendiliğinden veri yerine gitmesi ayrı bir dilimdir.
- `workspace: gis` taşıyan örnekler (378) yalnız kaydedildiklerinde kuralla karşılaşır; kaydedenlerin sınamaları
  çevrildi, öbürleri olduğu gibi okunur.
