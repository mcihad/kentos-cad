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
