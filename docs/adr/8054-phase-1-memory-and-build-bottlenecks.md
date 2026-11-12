# ADR 8054: Faz 1 — bellek, hız ve derleme darboğazları

- **Durum:** uygulanıyor (12 Kasım 2026). Sahibin isteği: "Argon tavsiye'deki ilk refactoring ve optimizasyon maddelerine
  başlayalım", ardından "Faz 1 tüm maddelerini bitir". Kaynak: `plan/argon-tavsiye.md` FAZ 1 (`ADIM 01` – `ADIM 12`).
- **Bağlam belgesi:** ADR 0029 (masaüstü deposu), ADR 0253 (komut kaydı ve plan), ADR 0654 (kaydın kopyasız yazımı).

Her adım kendi bölümünde: incelemenin iddiası koda karşı denetlenir, önce/sonra ölçülür, davranış değişmez.

## §1. Ayarlar ve stil kitaplığı paylaşımlı (`ADIM 01`)

**İddianın denetimi.** `Document::reading_copy()` 141 yerde çağrılıyor: her plan, tutamaçla sürüklemede izleyicilerin her
önizlemesi (`preview_followers`), her arka plan işi. Her çağrı `ProjectSettings`'i ve `ProjectStyles`'ı bütünüyle
kopyalıyordu. İncelemenin "TIN üçgenleri ayarlarda" sözü doğru değil: yüzeyin tanımı ayarlarda, üçgenleri çalışma anında
kurulur. Ağır olabilenler bağlı kaynakların özetleri (`SourceBase`: bir özetin alanları 8 MiB'a dek), parsel kütüğü,
GNSS arşivi, yol tabloları ve sembol kitaplığıdır.

**Karar.**
- `Document`'ın `settings`'i `Arc<ProjectSettings>`, `styles`'ı `Arc<ProjectStyles>`. Okuma kopyası ve adımın sakladığı
  önceki hâl (`Frame`) aynı değeri paylaşır; değişiklik yazınca kopyalar (`Arc::make_mut`, alan alan: öbür alanlar
  ödünç alınabilir kalır). Kopya yalnız değer paylaşılıyorken alınır.
- Adımın sonunda ve geri yüklemede karşılaştırma önce işaretçiye bakar (`Arc::ptr_eq`): ayara dokunmayan adım ayarları
  karşılaştırmaz.
- Dış API değişmedi (`settings()` ve `styles()` yine `&T` verir); alan `pub(crate)` olduğu için değişiklik
  `kentos-domain`'in içinde kaldı. Dosyaya yazılan anlık görüntü yine değeri taşır.
- Ayarların içindeki ağır parçaların ayrıca paylaşımlı olması (`Arc<RoadSettings>`, `Arc<Vec<SourceBase>>` …) sözleşmeyi
  ve geri alma işlemlerini birlikte değiştirir; §3'te geri almayla birlikte yapılır.

**Ölçüm** (`cargo run --release -p kentos-domain --example reading_copy -- MB 2000`; 4 bağlı kaynak özeti, 2 000 sembol):

| | Okuma kopyası (önce → sonra) | Ayar değişikliği | Çizgi ekleme |
|---|---|---|---|
| 16 MB özet | 5,336 → **0,000 ms** | 2,970 → 3,015 ms | 0,002 → 0,003 ms |
| 64 MB özet | 21,568 → **0,000 ms** | 13,761 → 13,930 ms | 0,002 → 0,003 ms |

Ayar değişikliği aynı: çağıran ayarların kopyasını alıp değiştirir (`set_settings`); bu §3'ün işidir.

**Doğrulama:** `cargo test -p kentos-domain` (159), `-p kentos-native-application` (812), `-p kentos-headless` (147),
`-p kentos-desktop --lib`.

## §2. Parça damgaları yalnız gerektiğinde (`ADIM 02`)

**İddianın denetimi.** Doğru. MCP, Python ve planlar özet ister (`state.summary || state.plan`); her komuttan önce altı
parçanın (katmanlar, ayarlar, stiller, paftalar, kısıtlar, ölçü kümeleri) JSON'u kurulup SHA-256'sı alınıyordu
(`PartStamps::of`). Özet bu damgalardan yalnız komutun değiştirdiği parçalarınkini kullanır. Masaüstünün kendi komutları
özet istemediği için etkilenmiyordu.

**Karar.**
- Komuttan önce parçalar saklanır, damgalanmaz (`Document::parts_at` → `PartsAt`): ad, paylaşımlı ayarlar, stiller,
  paftalar, kısıtlar, ölçü kümeleri ve katman ağacının kopyası. Damga istendiğinde ve bir kez alınır (`OnceLock`).
  Damgalar belgeninkiyle aynı işlevlerden gelir (`settings_stamp_of` …); planın kimliği değişmez.
- Çağrının beklediği parça damgaları (`expected.layers` …) da yalnız beklenen parça için alınır.
- Ayrıca belge düzeyinde bir damga önbelleği kurulmadı: önceki ve sonraki damgalar yalnız değişen parça için alındığından
  gerek kalmadı.

**Ölçüm** (aynı örnek): kayıtlı çağrı başına ek yük 16 MB'ta 40,5 → **0,000 ms**, 64 MB'ta 135,2 → **0,000 ms** (parçaların
saklanması); ayarı değiştiren komut yine iki damga öder.

**Doğrulama:** `cargo test -p kentos-domain --test all steps::parts_kept_before_a_write` (saklanan parçalar belge sonra
değişse de o anki damgayı verir); `-p kentos-native-application` (komut özetleri ve plan kimlikleri), `-p kentos-headless`.

## §3. Geri alma kaydı disiplinlere bölündü, ayar yükleri paylaşımlı (`ADIM 03`)

**İddianın denetimi.** Doğru: `history.rs` 2 182 satır, `Op` 72 varyant. 19 yol varyantı ve mimari/CBS listeleri
(`Domains`, `OpeningTypes`, `Materials`, `ArchModules`, `ArchOptions`, `OptionCombos`, `AuditExceptions`) önceki ve
sonraki hâli düz `Vec` tutuyordu; her geri alma ve yinelemede ters işlem (`inverse`) ikisini de kopyalıyordu. Harita
varyantları zaten `Arc` tutuyordu.

**Karar.**
- `history.rs` → `history/` klasörü: `mod.rs` (adım, grup, geri alma/yineleme; 629 satır), `op.rs` (çekirdek işlemler ve
  tersi), `apply.rs` (işlemin belgeye uygulanması), `forget.rs` (başka düzenleyicinin değişikliğiyle bırakılan adımlar)
  ve disiplin alt enum'ları: `road.rs` (`RoadOp`, 19), `map.rs` (`MapOp`, 6), `arch.rs` (`ArchOp`, 8), `gis.rs`
  (`GisOp`, 3). `Op` onları `Op::Road(..)`, `Op::Map(..)`, `Op::Arch(..)`, `Op::Gis(..)` olarak taşır; çekirdek işlemler
  (nesne, katman, blok, dizi, ölçü kümesi, pafta, kısıtlar) `Op`'ta kaldı.
- Yol, mimari ve CBS yükleri `Arc<Vec<T>>` (`RoadOp::Pay` `Arc<Option<_>>`, `ArchOp::BuildingProgram` `Arc<_>`): ters
  işlem hiçbir listeyi kopyalamaz. Uygulama değeri ayara bir kez kopyalar; ayar alanlarının kendilerinin paylaşımlı olması
  sözleşme değişikliğidir (`ProjectSettings`'in alanları), ayrı iş.
- Adım raporu (`report_of_ops`) ve geçmişi unutma, disiplin işlemlerini tek kolla sınıflar (`Op::Road(_)` …); davranış aynı.

**Ölçüm** (`cargo run --release -p kentos-domain --example undo_settings -- 200 500`: 200 değer alanı × 500 seçenek): bir
geri alma ya da yineleme 10,055 → **4,116 ms**. `history.rs` 2 182 → `history/mod.rs` 629 satır.

**Doğrulama:** `cargo test -p kentos-domain`, `-p kentos-native-application`, `-p kentos-headless`, `-p kentos-desktop --lib`
(2 312) değişmeden geçti.

## §4. Katalog, yetenek tablosu ve komut dosyaları bölündü (`ADIM 04`)

**İddianın denetimi.** Doğru: `pub fn catalog()` 14 600 satırlık tek işlevdi (içindeki tek `vec![…]` 438 komut);
`protocol_commands` ve testlerle dosya 19 043 satır. `capabilities.rs` 8 170 satır, ama çoğu sabit bir tablo
(`ENTRIES`, 481 kayıt): derleyicinin işlevi değil verisi. `commands/src` kökünde 206 modül düz duruyordu.

**Karar.**
- `catalog.rs` → `catalog/`: `mod.rs` (`catalog()` parçaları sırayla birleştirir, `protocol_commands`, testler) ve kataloğun
  sırasıyla 10 parça: `project`, `drawing`, `arch`, `data`, `arch_project`, `annotation`, `road`, `map`, `road_field`, `gis`.
  Sıra korunduğu için `commandCatalog.json` bayt bayt aynı (`cargo test -p kentos-commands`, 2 005).
- `capabilities.rs` → `capabilities/`: tablo sırasıyla 8 parça (`drawing`, `road`, `map`, `arch`, `data`, `annotation`,
  `gis`, `files`); `entries()` onları sırayla dolaşır.
- 204 komut modülü `core/` (71), `map/` (38), `road/` (35), `gis/` (32), `arch/` (28) klasörlerine taşındı. Modül yolları
  değişmedi (`lib.rs`'te `#[path]`): `kentos_commands::cad_walls::…` ve yeniden dışa verilen adlar aynı; taşınan dosyaların
  göreli `include_str!` yolları bir düzey derinleşti, `scripts/cad/manifest.py` yeni yolları okur.

**Ölçüm** (yalnız `kentos-commands`'ın yeniden derlenmesi, `/usr/bin/time -v`):

| | Önce | Sonra |
|---|---|---|
| Sürüm derlemesi | 110,6 s, en çok 13,8 GB | **93,9 s, en çok 9,1 GB** |
| Geliştirme derlemesi | 68,2 s, 6,7 GB | 64,9 s, 6,7 GB |

Geliştirme derlemesinin kalanı şema türetmelerinin (`schemars`, 1 000'i aşkın tip) genişlemesinde; tek işlevin bölünmesi
yalnız iyileştiricinin (LLVM) yükünü indirdi.

**Doğrulama:** `cargo test -p kentos-commands` (2 005: katalog JSON'u, birimler), `-p kentos-contracts`,
`-p kentos-native-application --test all capabilit|catalog`, `cargo check --workspace --tests` (kentos-mcp hariç).

## §5. Masaüstü kaynakları klasörlendi, testler ayrıldı (`ADIM 05`)

**İddianın denetimi.** Doğru: `apps/desktop/src` kökünde 650 dosya, 353'ü test, sahne ya da ekran dosyası
(`*_tests.rs`, `*_scenes.rs`, `*_screens.rs`), ayrıca 40 klasör.

**Karar.**
- Kökte yalnız `lib.rs` ve `main.rs` kaldı. Testler `tests/` (353); üretim kodu `shell/` (uygulama çerçevesi: pencere,
  şerit, ayarlar, belge, kaydet/aç; 252 dosya), `cad/` (çizim çekirdeği: blok, katman, yazı, ölçü, pafta, seçim, kenet;
  212), `gis/` (öznitelik, veri kaynağı, form, ağ, raster, nokta bulutu, süreç; 224), `map/` (aplikasyon, parsel,
  imar, ortofoto, ölçme ağı; 37), `road/` (güzergâh, boykesit, en kesit, kübaj; 45), `arch/` (mimari; 34). `cad/`
  klasörü inceleme notunda yok: çizim araçları uygulama çerçevesi değildir, `shell/`'e konmadı (benim kararım).
- Modül yolları değişmedi: `lib.rs` her modülü `#[path]` ile yeni yerinden yükler, `crate::labels::…` gibi bütün yollar
  aynı. Klasörü olan modüller (`labels.rs` + `labels/`) `cad/labels/mod.rs` biçimine geçti: `#[path]` ile yüklenen dosya
  `mod.rs` gibi arandığından iç modüller yerinde bulunur. Taşınan dosyalardaki bütün göreli `#[path]` ve `include_str!`
  yolları, hedefleri de taşındıysa onların yeni yerine göre yeniden hesaplandı. `apps/desktop/equivalents.json`'daki 132
  yol güncellendi; web'in ve betiklerin yorumlarındaki eski yollar tarihî kaldı.
- Testler yine tek test ikilisine derlenir: sahne testleri crate'in iç öğelerini (`pub(crate)`) kullanıyor; onları ayrı
  test hedeflerine (`tests/` crate'leri) taşımak dışa açık bir API ister, ayrı iş.

**Doğrulama:** `cargo check -p kentos-desktop --tests`, `cargo test -p kentos-desktop --lib` (2 312) değişmeden geçti;
1 157 dosya git'te yeniden adlandırma olarak görünür (`git log --follow`).

## §7. Masaüstünün büyük dosyaları konularına bölündü (`ADIM 07`)

**İddianın denetimi.** Doğru ama eksik: not dört dosyayı sayıyor; `apps/desktop/src`'de 2 000 satırı aşan **20** dosya
vardı, en büyüğü `tests/form_scenes.rs` (5 600). `rows.rs`'teki `entity_sections` tek başına 2 125 satırlık bir işlevdi:
dosyası bölünse de tek dosyaya sığmıyordu.

**Karar.** Her dosya konularına göre alt dosyalara bölündü (satır sayıları):

| Dosya | Önce | Sonra | Alt dosyalar |
|---|---|---|---|
| `tests/form_scenes.rs` | 5 600 | 414 | `buildings` 1 277, `lists` 1 051, `parcels` 813, `tests/workflow` 971, `tests/editing` 1 104 |
| `gis/forms/entry.rs` | 5 444 | 1 015 | `opening` 419, `steps` 817, `view` 614, `items` 703, `controls` 768, `related` 1 161 |
| `shell/app.rs` | 5 061 | 1 935 | `start` 437, `update` 933, `settings` 255, `commands` 1 139, `tests` 400 |
| `cad/properties/rows.rs` | 4 617 | 405 | `document` 205, `look` 345, `entity` 1 233, `fields` 155, `annotation` 887, `many` 429; `wall` 271, `space` 333, `slab` 198, `opening` 341 |
| `cad/labels/mod.rs` | 4 366 | 1 612 | `paint` 996, `placed` 284, `ink` 319, `turned` 316, `screens` 654, `tests` 227 |
| `cad/properties/tests.rs` | 4 290 | 294 | `objects` 570, `edits` 569, `elevations` 601, `annotations` 1 130, `parameters` 574, `arch` 604 |
| `cad/sheets/mod.rs` | 4 101 | 1 385 | `mode` 746, `tests` 929, `tests/screens` 1 049 |
| `cad/properties/mod.rs` | 2 975 | 653 | `panel` 920, `commit` 1 423 |
| `gis/forms/designer_view.rs` | 2 959 | 871 | `palettes` 489, `inspector` 1 265, `flow` 359 |
| `gis/forms/designer.rs` | 2 851 | 1 463 | `step` 1 396 |
| `tests/sheet_book_tests.rs` | 2 823 | 355 | `steps` 994, `model_scope` 634, `publishing` 886 |
| `shell/catalog.rs` | 2 766 | 851 | `ported` 842, `reading` 628, `tests` 460 |
| `shell/cloud/tests.rs` | 2 730 | 353 | `signing_in` 403, `sending` 674, `drafts` 559, `offline` 368, `renaming` 408 |
| `shell/view.rs` | 2 686 | 887 | `ribbon` 547, `area` 646, `status` 634 |
| `shell/viewport/mod.rs` | 2 573 | 1 111 | `state` 874, `tests` 593 |
| `road/volume_window.rs` | 2 496 | 1 531 | `reports` 524, `ledger` 463 |
| `shell/input.rs` | 2 113 | 1 070 | `fields` 589, `value_field_tests` 335, `cursor_settings_tests` 129 |
| `road/section_window.rs` | 2 069 | 1 342 | `editors` 508, `charts` 253 |
| `gis/sources/view.rs` | 2 013 | 587 | `lines` 693, `rows` 764 |
| `cad/layering.rs` | 2 004 | 947 | `menus` 539, `screens` 268, `tests` 265 |

- **Adlar kodun kendi konularından**, notun önerdiği kalıplardan değil (ör. `entry/{state,widgets,validation,…}` yerine
  `entry/{opening,steps,view,items,controls,related}`): not dosyaların içini değil adlarını görmüştü (benim kararım).
- **Taşınan kod harfi harfine aynı.** Değişen yalnız: özel öğe ve yöntem `pub(super)` oldu (erişimi aynı: üst modül ve
  altları), `pub(super)` olan `pub(crate)`; testler kendi görünürlüğünde kaldı. Taşınan koddaki `super::` yolları bir
  düzey uzadı, göreli `include_str!` yolları bir klasör derine göre yazıldı, dosyaya çıkan iç test modülleri 4 boşluk
  sola kaydı. Alt dosya üst modülü `use super::*` ile görür, üst modül alt dosyanın öğelerini `use alt::*` ile geri alır:
  `crate::catalog::PORTED`, `crate::forms::entry::…` gibi bütün yollar aynı. `column!` std'nin önsözünde de bulunduğundan
  `use super::*` ile gelen `iced` makrosuyla belirsiz kalır; onu kullanan alt dosya açıkça içe alır.
- **Dev işlevin kolları kendi dosyalarında.** `entity_sections`'ın duvar, mahal, döşeme ve boşluk kolları (1 078 satır)
  `rows/{wall,space,slab,opening}.rs`'te birer `rows` işlevi oldu (`rows/arch.rs`'in kalıbı). Ortak değerleri (belge,
  nesne, birim biçimi, kilit, alanı ve uzunluğu; `edit`, `number`, `metres`, `area`) tek bir `Kit` yapısında;
  `entity_sections`'ın kendi kapanışları da ondan okur, iki kopya yok. Form tasarımcısının 1 389 satırlık olay işleyicisi
  (`forms_step`) ve özellik panelinin `commit`'i (1 151) tek parça kendi dosyalarına sığdığı için bölünmedi.
- **Öğe sınırlarını derleyicinin sözdizimi ağacı (`syn`) verdi.** İlk deneme satır sayan bir ayrıştırıcıyla yapılmıştı;
  çok satırlı ham metinlerde ve `thread_local!` gibi makrolarda tökezledi, 30 derleme hatası bıraktı ve geri alındı.
- **Ad çakışması:** `viewport`'ta `scene` içe alınmış bir modülün adıydı; alt dosya `state.rs` oldu.
- **Bekçi (plan dışı, benim kararım):** `tests/source_size_tests.rs` `src` altında 2 000 satırı aşan dosya kalırsa düşer
  ve dosyaları sayar; 2 001 satırlık deneme dosyasıyla düştüğü görüldü.
- **Biçim:** taşınan satırlar olduğu gibi kaldı (depodaki bazı dosyalar rustfmt'nin 100 sütununa göre yazılmamış; ör.
  `road/volume_window.rs` önceden 1 669 fark satırı taşıyordu). Bölmenin yazdığı satırlar (modül başları, `use`, `mod`,
  genişleyen imzalar) rustfmt'ten geçti.
- **Sonuç:** 2 000'i aşan dosya 20 → 0; 77 yeni dosya; en büyük dosya artık `cad/tables/editor.rs` (1 995, dokunulmadı).
  Tek crate olduğu için derleme süresi değişmez; kazanç okunurluk ve birleştirme çakışmaları.

**Doğrulama:** `cargo check -p kentos-desktop` ve `--tests` uyarısız; `cargo test -p kentos-desktop --lib` 2 313 geçti
(önceki 2 312 + bekçi), 564 yok sayıldı — hiçbir test kaybolmadı ya da çiftlenmedi. Clippy'nin bölünen dosyalarda
gösterdiği 10 uyarının hepsi taşınan koddan; yazılan satırlarda uyarı yok.

## §9. Uygulama, etkileşim ve sözleşme crate'leri klasörlendi (`ADIM 09`)

**İddianın denetimi.** Doğru: `application/src` kökünde 587 dosya ve 9 klasör, `interaction/src` 213 dosya ve 4 klasör,
`contracts/src` 152 dosya; `contracts/src/road.rs` 4 187 satır; `registry.rs` 457 kollu tek bir eşleşme makrosu; test
koşucusu `tests/all/fixtures.rs` 7 674 satır.

**Karar.**
- Üç crate'in kökünde yalnız `lib.rs` ve klasörler kaldı: `application` → `core/` 189, `road/` 135, `gis/` 119, `arch/` 91,
  `map/` 52 dosya; `interaction` → `core/` 107, `arch/` 45, `viewer_tools/` 34, `map/` 15, `gis/` 9, `road/` 4;
  `contracts` → `core/` 64, `gis/` 53, `arch/` 21, `map/` 13, `road/` 6. Modül yolları `#[path]` ile aynı (ADIM 05'in
  yöntemi), göreli yollar yeni yerlerine göre yeniden hesaplandı.
- Etkileşimin salt-okunur araçları (seçim, çit, ölçü, açı, koordinat, km-ofset, gezinme, kenet adları, uzamsal indeks) ayrı
  `viewer_tools/` klasöründe: mobil görüntüleyicinin alacağı kümesi (ADIM 08) bir arada.
- `contracts/src/road.rs` → `road/`: `mod.rs` (ayarlar, sınırlar, ortak denetimler, testler), `terrain.rs`, `alignment.rs`,
  `profile.rs`, `corridor.rs` (şablon, kaplama, en kesit, dever, koridor, kübaj, taşıma), `structures.rs` (pafta takımı,
  bariyer, menfez, istinat, platform, kavşak, hakediş). Bütün adlar `kentos_contracts::road::…` ve kök yeniden dışa
  vermeleriyle aynı.
- Komut kaydı beş alt dağıtıcıya bölündü (`{core,arch,map,gis,road}/dispatch.rs`, `*_commands!`): her kol işleyicisinin
  klasörüne gider; `by_command!` onları sırayla zincirler, işleyici adları yine kaydın içinde çözülür. 457 kolun hepsi
  yerinde. Test koşucusu aynı biçimde bölündü: `tests/all/fixtures/` (`mod.rs` 2 514 satır, `run_*.rs` koşucu kolları,
  `inputs_*.rs` 455 girdi tipi).

**Doğrulama:** `cargo test -p kentos-contracts --all-features`, `-p kentos-interaction`, `-p kentos-native-application`
(veri güdümlü bütün komut durumları dahil), `-p kentos-headless`, `-p kentos-desktop --lib`; `cargo check --workspace
--tests` (kentos-mcp hariç).

