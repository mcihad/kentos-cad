# ADR 0801: KentOS Mimari, KentOS Harita ve KentOS GIS ayrı programlar; CAD şeridi işe göre

- **Durum:** kabul edildi (2026-10-30).
- **İstek:** sahibin sözleri “Mimari CAD, Harita CAD ve GIS ayrımı” ve “Uygulamaların her biri ayrı binary olacak değil mi”
  (evet: her modül kendi çalıştırılabilir programıdır). Önerilen ve sahibin “başla”sıyla kabul ettiği öntanımlılar:
  - **CAD şeridi işe göre.** Mimari proje: Mimari sekmesi var, Plan yok, Ölçme'de yalnız *Sorgu* paneli. Harita / imar planı: Plan
    ve bütün Ölçme var, Mimari yok. Detay / imalat: Mimari de Plan da yok, Ölçme'de yalnız *Sorgu*. İşi bilinmeyen CAD projesi
    bugünkü gibi bütün CAD şeridini görür. CBS şeridi değişmez. Bağlam sekmeleri değişmez. Şeritte görünmeyen komut komut
    satırından ve Komut ara'dan yine çalışır (CLAUDE.md §4.12: tür bir güvenlik sınırı değildir).
  - **Detay / imalat KentOS Mimari'nindir.** Modüller: CBS projesi → KentOS GIS; CAD ve Mimari proje ya da Detay / imalat → KentOS
    Mimari; CAD ve Harita / imar planı → KentOS Harita; işi olmayan CAD projesi → hiçbir modül.
  - **İşi olmayan eski CAD dosyası masaüstünde bir kez sorulur** (`ask_type.rs` gibi): “Bu CAD çizimi hangi iş için?”, üç işle.
    Cevap projenin ayarıdır: çizim değişmiş sayılır, geri alma adımı değildir. Başsız sunucu, Python ve MCP sormaz.
- **Bağlam belgesi:** ADR 0800 (KentOS Launcher, `crates/native/launch` sözleşmesi), 0165 (CAD ve CBS türleri), 0622 (masaüstünün
  şeridi), 0626 (sihirbazın üç işi), 0245 (Python ve MCP'de yeni çizim), 0050 (Başlangıç ve son dosyalar).
- **Web:** bir şey değişmez (ADR 0228); dönüşte gerekenler raporda ve TODOS.md §0.2'ye yazılacak.
- **`.kcad`:** şema 135 (opt-in): ayarların `cadWork`'ü. `FORMATS_VERSION` değişmez (tipli sütunlar değişmedi).
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

Bugün tek program vardır: `kentos-cad`. Proje CAD ya da CBS türündedir (ADR 0165) ve tür değişmez (ADR 0622 §1); CAD projesinin şeridi
her CAD projesinde aynıdır: bina çizen kullanıcı Plan'ı (yol, ada, parsel) ve bütün Ölçme'yi (karne, poligon, dengeleme), harita
çizen kullanıcı Mimari'yi görür. Sihirbaz yeni CAD projesine “Ne çiziyorsunuz?” diye sorar (ADR 0626 §1), ama cevap projeye
yazılmaz: yalnız koordinatları, birimi ve ölçeği getirir. KentOS Launcher (ADR 0800) programları modüllere göre gösterecek ve
başlatacaktır; ortak sözleşmesi `kentos-launch` modülleri (`mimari`, `harita`, `cbs`), her modülün programının adını
(`kentos-mimari`, `kentos-harita`, `kentos-gis`), komut satırını (`[--modul <id>] [dosya]`) ve son dosyaları (modülüyle) tutar.

Başvurulan yollar (30 Ekim 2026'da okundu):

- Autodesk, *AutoCAD included toolsets*: AutoCAD yedi sektör araç takımıyla gelir (Architecture, Mechanical, Map 3D, MEP,
  Electrical, Plant 3D, Raster Design); her biri AutoCAD'in üstünde kendi programıdır.
  <https://www-int.autodesk.com/in/products/autocad/included-toolsets>
- Autodesk, *Map 3D toolset › Workspaces*: Map 3D'nin hazır çalışma alanları (2D Drafting, Planning and Analysis, Maintenance, Map
  Classic) aynı programın şeritleridir; araç takımı ilk açılışta öntanımlı çalışma alanını sorar, çalışma alanı oturumda
  değiştirilebilir. <https://help.autodesk.com/cloudhelp/2025/ENU/MAP3D-Learn/files/GUID-390141F5-C3FB-4EEE-A872-B592BF3F9D48.htm>
- Autodesk, *About Customizing Startup*: komut satırı anahtarları masaüstü kısayolunun komut satırına eklenir; her biri başka
  başlangıç seçenekli birden çok kısayol kurulabilir; anahtarlar ayarları yalnız o oturum için ezer.
  <https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT/files/GUID-07B8B8AB-C6F7-4236-9D33-5343E4DC88F1.htm>
- AutoCAD Architecture'ın kısayolu aynı `acad.exe`'yi belgelenmemiş `/product "ACA"` anahtarıyla başlatır (topluluk kaynağı, Autodesk
  belgesi değil; yalnız bir programdan birden çok ürün çıkarmanın örneği olarak anıldı).
  <https://blog.jtbworld.com/2013/04/autocads-undocumented-language-and.html>
- QGIS, *QGIS Configuration › Running QGIS with advanced settings* ve *Working with user profiles*: tek kod tabanı komut satırı
  seçenekleriyle başlar (`--profile`, `--project`, `--profiles-path`); her kullanıcı profilinin ayarları, eklentileri ve geçmişi
  ayrıdır. <https://docs.qgis.org/latest/en/docs/user_manual/introduction/qgis_configuration.html>
- QGIS 2.18, *QGIS Browser*: “QGIS Browser is available as a standalone application and as a panel in QGIS Desktop”: aynı kod iki
  programda. <https://docs.qgis.org/2.18/en/docs/user_manual/qgis_browser/qgis_browser.html>
- QGIS, *Using processing from the command line*: `qgis_process` (QGIS Processing Executor) aynı kod tabanından ayrı bir programdır
  ve masaüstünü açmadan algoritma çalıştırır. <https://docs.qgis.org/latest/en/docs/user_manual/processing/standalone.html>

KentOS'un çıkardığı: AutoCAD'in araç takımları ve QGIS'in programları gibi **bir kitaplık, birden çok program**; modül hem projenin
özelliğidir (dosya söyler) hem programın (kısayol ya da program söyler). Şerit, Map 3D'nin çalışma alanları gibi tek kataloğun bir
kesitidir; ama kullanıcının seçtiği bir görünüm değil, projenin işine bağlıdır: aynı dosya her programda aynı şeritle açılır.

## Karar

### 1. Projenin işi

- Sözleşmeye `CadWork` (`architecture`, `map`, `detail`; Türkçe adları sihirbazın kartlarınınkiyle aynı: Mimari proje, Harita /
  imar planı, Detay / imalat) ve ayarlara `cadWork` eklenir. Yalnız CAD projesinin işi olur: `ProjectSettings::sanitized` başka
  projede onu düşürür, `cad_work()` yalnız CAD projesinde verir.
- **Modülü:** `kentos_launch::Module::of_project(tür, iş)` dosyanın yazdığı adlarla: `gis` → KentOS GIS; `cad` + `architecture` ya da
  `detail` → KentOS Mimari; `cad` + `map` → KentOS Harita; işi sorulmamış CAD projesi, türü sorulmamış proje ve duyurulmuş türler →
  modül yok. `Module::cad_works()` modülün işlerini (yeni projenin ilki önce), `project_type()` türünü verir. Kural kentos-launch'a
  yazıldı (sözleşmeyi genişletir, var olanı değiştirmez): Launcher da aynı kuralı okuyabilsin, kentos-launch serde'den başka bir şey
  bilmesin diye adlarla.
- **İş türü gibi bir kez seçilir:** işi olan projenin işi değişmez (`cad_work_locked`); Mimari ile Harita arasında geçiş ileride
  projeyi dönüştürerek olacak. Aynı iş yeniden verilirse bir şey değişmez.

### 2. Dosya biçimi: şema 135

Ayarların `cadWork`'ü (numaralı metin: `architecture`, `map`, `detail`), yalnız `workspace` `cad` iken; anahtarların sırasında 7 harfli
anahtarların ilki (`survey`'den sonra, `domains`'ten önce). Yazıcı 135'i yalnız işi olan projede seçer; öbür bütün çizimler bayt bayt
eskisi gibi kalır. Okuyucu: 134'te `unknown_field`; bilinmeyen değer, CBS projesinin ya da türü seçilmemiş projenin işi `bad_value`.
Örnek `cad-work.kcad` (bina çizimi, yerel, 1:100), önceki şema örneği `broken/cad-work-in-schema-134.kcad`, bozuk örnekler
`broken/cad-work-unknown.kcad`, `broken/cad-work-on-gis.kcad`, `broken/cad-work-no-type.kcad`; gelecek şemanın örneği
`broken/schema-version-136.kcad` oldu. Bağımsız okuyucu (`tools/kcad/kcad.py`) ve yazıcı (`scripts/fixtures/kcad_v2_reference.py`)
aynı kuralı taşır; spesifikasyon §6.1 ve §6.4.

### 3. Yeni proje, komutlar ve sunucu

- **Sihirbaz:** `kentos_project::wizard::Work`'ün her biri işini adlandırır (`work`); `Draft::cad_work()` taslağın işidir,
  `options()` onu `NewProject.cad_work` olarak verir, yeni proje onu ayarlara yazar (yalnız CAD projesinde). İş taslağın koordinat ve
  biriminden çıkar (ADR 0626): yerel metre Mimari proje, sistemli Harita / imar planı, yerel mm Detay / imalat; işi olmayan birim
  (cm, inç, ayak) işsiz proje verir.
- **Python ve MCP:** `Document.new` ve `drawing.new` aynı çekirdekten (`NewRequest` → `Draft::options`, ADR 0245) geçtiği için işi
  aynı kuralla yazar; istekte yeni alan yoktur. Başka bir iş `cad.settings.set` ile (işi olmayan çizimde) verilir.
- **`cad.settings.set`'in `cadWork`'ü:** CAD projesinde, işi yoksa bir kez; `projectType: cad` ile türü sorulmamış projeye ikisi
  birlikte. Retler, tür kilidinden sonra ve sırayla: `cad_work_needs_cad` (CAD olmayan proje), `cad_work_locked` (başka bir işi olan
  proje). Değişen anahtar `cadWork`, `projectType`'tan sonra. Ayardır, geri alma adımı değildir. Python SDK'sı katalogdan yeniden
  üretildi (`cad_work`), MCP katalogu izler.
- **Sunucu:** proje oluşturma ve `project.changes` aynı kuralı denetler (`projects::check_cad_work`: CAD olmayan projenin işi
  reddedilir). Türün kendisi gibi iş de sunucuda kilitlenmez; kilit komuttadır.

### 4. Şerit

- `catalog/v1/ribbon.json`'da sekmeler ve paneller isteğe bağlı `works` listesi taşır (`scripts/ui/ribbon.py` yazar): Plan sekmesi
  `map`, Mimari sekmesi `architecture`, CAD Ölçme'sinin Sorgu dışındaki panelleri `map`. `works`'süz sekme ve panel her işte görünür.
  CBS şeridi ve bağlam sekmeleri `works` taşımaz. Bütün işlerin şeritleri birlikte bütün CAD şeridinin komutlarını verir (CAD
  manifesti değişmez).
- Katalog her işin CAD şeridini bir kez kurar (`Catalog::tabs_for(tür, iş)`); `tabs_in(tür)` bütün şerittir.
- **Şeridin işi** (`App::ribbon_work`): çizimin işi; işi olmayan CAD çiziminde bir CAD modülünün programı kendi ilk işini (KentOS Mimari:
  Mimari proje, KentOS Harita: Harita / imar planı), kentos-cad hiçbirini (bütün CAD şeridi). Türü sorulmamış çizim ve çizim yokken
  modülün programı kendi türünü gösterir; kentos-cad bugünkü gibi CBS'yi.
- Durum çubuğunun tür hücresi işi olmayan CAD çiziminde işi sorar (türü sorulmamış projede türü sorduğu gibi); ipucu türün yanında işi
  de söyler (“Proje türü: CAD · Mimari proje”). Proje ayarları'nın Proje türü satırı işi de yazar.

### 5. Programlar

- `apps/desktop` bir kitaplık (`kentos_desktop`, `src/lib.rs`: bütün modüller, `all_in_one` ve `module_program`) ve programlarıdır.
  Kitaplığın belge yorumları test değildir (`doctest = false`); programların kendi testleri yoktur (`test = false`): bir test
  koşusu yine tek ikili bağlar.
- **`kentos-cad`** (`src/main.rs`): önce kendi sözcükleri (`snapshot`, `kullan`), sonra kentos-launch'ın komut satırı
  (`[--modul <id>] [dosya]`). Okunamayan komut satırı (bilinmeyen modül, iki dosya, değersiz `--modul`) stderr'e cümlesiyle yazılır,
  çıkış kodu 2. `--modul <id>` onu tam o modülün programı yapar.
- **`kentos-mimari`, `kentos-harita`, `kentos-gis`** (`src/bin/`): kitaplığı kendi modülüne sabitleyen birkaç satır. `--modul` yalnız
  kendi modülünü adlandırabilir; başkası çıkış kodu 2 ile reddedilir (“kentos-gis yalnız KentOS GIS olarak çalışır; KentOS Mimari için
  kentos-mimari programını ya da kentos-cad --modul mimari komutunu kullanın.”). `module-programs` özelliğiyle derlenir; sıradan
  `cargo build` ve `cargo test` yalnız `kentos-cad`'i bağlar.
- **Modülün programında:** pencerenin başlığı “KentOS Mimari — <çizim>” (çizimsiz “KentOS Mimari”); açılış iletisi programın adıyla;
  Başlangıç ekranı modülün adını, ne yaptığını ve son dosyalardan modülün kendilerini ve modülü bilinmeyenleri gösterir (uygulama
  menüsü de); Yeni proje modülün projelerini yapar: KentOS Mimari Mimari proje ya da Detay / imalat, KentOS Harita Harita / imar planı,
  KentOS GIS CBS projesi. Sihirbaz türü sormadan ikinci adımda (Çizim ya da Konum) açılır; türün sayfası yalnız modülün türünü ve
  öbürlerinin hangi programda yapıldığını gösterir; başka tür ya da iş seçilmez, programın türü “yeni projelerin türü” tercihine
  yazılmaz.

### 6. Açılış: yerleştirme, soru ve devir

Okunan çizim ekrana konmadan önce programın kuralından geçer (`App::opened_in_program`):

| Okunan | kentos-cad | KentOS Mimari / Harita | KentOS GIS |
|---|---|---|---|
| Bu programın modülünden | açılır | açılır | açılır |
| Başka modülden | açılır | devir sorusu | devir sorusu |
| İşi olmayan CAD (`.kcad`) | açılır, iş sorulur | iş sorulur, açılmadan önce | iki CAD programına devir sorusu |
| Türü sorulmamış (`.kcad`) | açılır, tür sonra iş sorulur | tür ve iş birlikte sorulur, CBS kartıyla | tür ve iş birlikte sorulur, CBS önde |
| DWG, DXF, NCZ | açılır, sorulmaz, işsiz | modülün işiyle açılır (mm çizim KentOS Mimari'de Detay) | iki CAD programına devir sorusu |
| Kurtarma kopyası, bulut projesi | açılır | açılır | açılır |

- **İş sorusu** (`project/ask_work.rs`, “Bu CAD çizimi hangi iş için?”; türsüzde “Bu proje hangi iş için?”): sihirbazın iş kartları;
  seçili kart çizimin görünüşünden tahmin edilir (sistemli Harita, mm Detay, öbürü Mimari; modülün programında modülün işlerinden).
  Seç işi (türsüzde türü de) projeye yazar: çizim değişmiş sayılır, geri alma adımı değildir. Modülün programında başka modülün işi
  seçilince düğme “KentOS Harita'da aç” olur ve dosya o programa verilir, burada hiçbir şey açılmaz. Sonra, Esc ve × çizimi olduğu gibi
  açar (modülün programında bir daha sorulmaz); iş sorulmamış kalır, sonraki açılış ve durum çubuğunun tür hücresi yine sorar. Ekrandaki
  çizime sorulan soru (kentos-cad, durum çubuğu) yalnız programın kendi işlerini önerir, devir yapmaz.
- **Devir sorusu** (`project/hand_over.rs`, “Başka programın dosyası”): ““Mahalle analizi.kcad” bir KentOS GIS projesi. KentOS GIS'te
  açılsın mı?” [Vazgeç] [KentOS GIS'te aç]. KentOS GIS'te işi bilinmeyen CAD çizimi iki programa sorulur; tek birincil düğme çizimin
  görünüşünden tahmin edilen programındır, öbürü solda (DESIGN.md §7.9). Devir `programs::find(module.program())`'ın bulduğu programı,
  bulunamazsa `kentos-cad`'i `--modul <id>` ile başlatır (`program::start_program`); bulunamazsa nedeni ve çözümü söylenir. Program
  arka planda kendi başına sürer; çıkışı bir iş parçacığında toplanır. Vazgeç, Esc ve × hiçbir şeyi değiştirmez.
- Kurtarma kopyası kaydedilmemiş iştir: hangi programda önerilirse orada açılır. Bulut projesi de açıldığı programda açılır; şeridi
  projenin türünü ve işini izler (devri ileride proje kimliğiyle).

### 7. Son dosyalar

Son dosyalara her ekleme çizimin modülüyle yapılır (`RecentFiles::add_in`; açılan DWG, DXF ve NCZ'nin kaynağı da). Modülün programı
kendi modülünün ve modülü bilinmeyen dosyaları gösterir; kentos-cad hepsini.

### 8. Sonraya kalanlar

- Her programı yalnız kendi koduna indirmek: bugün dört program aynı kitaplığı bağlar ve aynı boydadır; modülün kullanmadığı
  araçlar komut satırından çalışır.
- Paketleme: kurulum paketleri, masaüstü girdileri ve her modülün ikonu; programların ayrı sürümleri.
- Devrin cevabı taşıması: başka bir CAD modülüne verilen işsiz dosyayı alan program işi yeniden sorar (kendi işi seçili); cevap komut
  satırında taşınmadı, çünkü kentos-launch'ın `Launch` yapısını Launcher ile birlikte değiştirmek gerekir. Dosyaya kendiliğinden
  yazılmaz.
- Bulut projesinin başka programa proje kimliğiyle verilmesi.
- İşi değiştirmek (projeyi dönüştürme) ve işe göre öntanımlı katman ağacı, şablon ya da araç kutusu.

### 9. Performans

Sıcak yolda bir şey yoktur: işlerin şeritleri katalogla birlikte bir kez kurulur (üç CAD şeridi daha), açılışın kuralı yalnız
ayarları okur, soru pencereleri olay başına kurulur.

## Uygulama

- **Sözleşme:** `crates/shared/contracts/src/document.rs` (`CadWork`, `ProjectSettings.cad_work`, `cad_work()`, `Workspace::id()`,
  `sanitized`), kodlar `codes.rs` (`cad_work_needs_cad`, `cad_work_locked`). Komut `crates/shared/commands/src/cad_settings.rs`
  (`SettingsSet.cad_work`) ve `capabilities.rs`; katalog ve TS türleri üretildi (`CadWork.ts`, `ProjectSettings.ts`,
  `SettingsSet.ts`, `commandCatalog.json`); Python `kentos.cad` `scripts/python/sdk.py` ile yeniden yazıldı. Komutun işi
  `crates/native/application/src/settings_set.rs`.
- **Dosya:** `crates/shared/kcad` (`SCHEMA_WITH_CAD_WORK`, `encode.rs`, `decode.rs`), `tools/kcad/kcad.py`,
  `scripts/fixtures/kcad_v2_reference.py`, `fixtures/kcad/v2` (`cad-work.json` ve örnekleri, `expected.json`), `docs/specs/kcad-v2.md`.
- **Yeni proje:** `crates/native/project/src/wizard.rs` (`Work.work`, `Draft::cad_work`), `new_project.rs` (`NewProject.cad_work`),
  `fixtures/project/v1/wizard.json`. Modüllerin kuralı `crates/native/launch/src/module.rs` (`project_type`, `cad_works`,
  `of_project`).
- **Sunucu:** `crates/server/application/src/projects.rs` (`check_cad_work`), `changes.rs`.
- **Şerit:** `scripts/ui/ribbon.py` (`works`) ve ürettiği `catalog/v1/ribbon.json`; masaüstünün kataloğu `apps/desktop/src/catalog.rs`
  (`work_tabs`, `tabs_for`; bütün şerit `tabs_in` yalnız testlerde), `modes.rs` (şeridin sekmeleri, tür hücresi).
- **Programlar:** `apps/desktop/Cargo.toml` (`[lib]`, programlar, `module-programs`), `src/lib.rs` (`all_in_one`, `module_program`),
  `src/main.rs`, `src/bin/kentos-{mimari,harita,gis}.rs`, `src/program.rs` (modülün kuralı, başlık, son dosyalar, programı başlatma,
  açılışın kuralı); `opening.rs` (`Opened`, `place_opened`), `project/ask_work.rs`, `project/hand_over.rs`, `project/ask_type.rs`
  (CAD seçilince iş), `project/mod.rs`, `project/settings.rs` (türün satırında iş), `project/wizard/` (modülün sınırı), `start.rs`,
  `app_menu/`, `app.rs` (`start_in`, başlık), `cloud/opening.rs`.
- **Testler:** masaüstünün birim testleri kitaplıktadır: `cargo test -p kentos-desktop <süzgeç>` (önceki `--bin kentos-cad` yerine;
  masaüstünün kendi yorumları ve `apps/desktop/scripts/*-live.sh` `--lib` der). Modüllerin testleri `module_tests.rs` ve
  `program.rs`'in testleri, resimleri `module_screens.rs`:

  ```text
  cargo test -p kentos-desktop module_tests program::
  cargo test -p kentos-desktop module_screens -- --ignored --nocapture --test-threads=1   # .run/shots/moduller-*
  cargo build -p kentos-desktop --features module-programs   # target/debug/kentos-{cad,mimari,harita,gis}
  python3 scripts/fixtures/settings_set_command_cases.py --check
  ```

## Doğrulama

30 Ekim 2026, bu dalda:

- **Bağımsız başvurular (KentOS kodu olmadan):** `settings_set_command_cases.py --check` 121 ortak durum (16'sı `cadWork`: CAD
  projesinde ilk iş, aynı iş, kilit, CBS'de ve türsüzde ret, türle birlikte, `sanitized`); `kcad_v2_reference.py --check` 1365 dosya
  (yeni `cad-work.kcad` ve dört bozuk örnek, gelecek şemanın örneği 136); `tools/kcad/kcad.py validate` örneği geçerli, bozukları
  `unknown_field` ve `bad_value` ile reddeder; `scripts/ui/ribbon.py --check`, `scripts/cad/manifest.py --check` (922 satır, değişmedi),
  `scripts/python/sdk.py --check`, `node scripts/arch/deps.mjs` (50 crate).
- **Rust:** `cargo test -p kentos-desktop` 1401 geçti (245 elle koşulan; modüllerin 9 ve `program.rs`'in 3 testi dahil);
  `cargo clippy -p kentos-desktop --all-targets --features module-programs -- -D warnings` temiz; dört program
  `cargo build -p kentos-desktop --features module-programs` ile bağlandı. `kentos-commands` 952, `kentos-contracts` 1096 + 34 + 10,
  `kentos-kcad` 6 + 234 (taşıma matrisi, örnekler), `kentos-launch` 10, `kentos-project` 28, `kentos-native-application` 76 + 262
  (bütün komut durumları ve yetenekler) geçti; değişen test dosyalarıyla on üç crate `clippy --all-targets -D warnings` temiz.
  Sunucunun `projects` testleri 7 (yeni `a_cad_work_is_a_cad_projects`). `kentos-native-application`'da bir test düşer ama bu dalın
  değildir: `walls_partition::source_height_and_architecture_survive_and_curved_edges_are_explicitly_refused` yaylı kaynağın reddini
  bekler, ADR 0734 (`ad0a98880`) onu kabul etti ve testi değiştirmedi; main'de de düşer.
- **Programlar:** `kentos-gis --modul mimari`, `kentos-harita --modul=cbs x.kcad`, `kentos-mimari --modul`, `kentos-cad --modul yok`,
  `kentos-cad a.kcad b.kcad` cümlesiyle ve çıkış kodu 2 ile reddedildi; `kentos-cad snapshot` `cad-work.kcad`'i Mimari şeridiyle
  (Plan yok) çizdi.
- **Resimler** (`module_screens.rs`, `.run/shots/moduller-*`, 107 resim; koyu ve açık, 1440×900 ve 1100×650, sorular ve sihirbaz
  18 px yazıyla da): üç işin şeritleri ve Ölçme'si, üç modülün Başlangıç'ı, sınırlı sihirbaz ve tür adımı, iş sorusu (kentos-cad,
  KentOS Mimari, Harita'ya devreden seçim, türsüz proje), devir soruları (CBS projesi KentOS Mimari'de, işsiz CAD KentOS GIS'te),
  Proje ayarları'nın tür ve iş satırı. Dar pencerede ve büyük yazıda kesik düğme ya da taşan yazı görülmedi.
- **Koşulmadı:** `pnpm py:test` ve `kentos-headless`, `kentos-mcp`, `kentos-plot`, `kentos-domain`, `kentos-interaction` testleri
  (diskte yer kalmadığı için; değişiklikleri `clippy --all-targets` ile derlendi), veritabanlı sunucu testleri (`KENTOS_TEST_DB`), bir
  programın masaüstü oturumunda gerçekten başlatılması (devir testlerde sahte başlatıcıyla, `App::hand_over`), modül programlarının
  gerçek penceresi ve Launcher ile birlikte çalışma. Web dokunulmadı (ADR 0228).
