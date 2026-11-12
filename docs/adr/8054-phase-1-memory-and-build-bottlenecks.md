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

