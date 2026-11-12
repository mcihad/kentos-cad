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

