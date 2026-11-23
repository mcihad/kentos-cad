# ADR 9196: Saklama modeli proje türündendir: CBS verisi veritabanında, CAD projesi proje dosyasında

- **Durum:** kabul edildi (2026-11-23); plan [plan/ayrim.md](../../plan/ayrim.md) AY-01 (karar), AY-05 ve AY-06
  (uygulama). ADR 9107'nin “CBS'de veri her hâlükârda veritabanında” sözünü kural yapar. ADR 9184'ün saklama
  kartlarını ve ADR 0031'in iki bulut biçiminin her türe açık olmasını değiştirir.
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

## §5 Kalan

AY-05: CBS'nin sihirbazı, tabloları, içe alma, kayıt kuralı. Bulutta CBS'nin yalnız veritabanı biçimi olması da
AY-05'tedir: buluta yüklemenin veri katmanlarının satırlarını aktarması gerekir. Her madde kanıtıyla bu ADR'ye ya da
kendi ADR'sine yazılır.
