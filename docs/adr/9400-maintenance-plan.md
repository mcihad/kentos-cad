# ADR 9400: Bakım planı, biçim tabanı ve tek denetim komutu

- **Durum:** kabul edildi (2026-11-28). [plan/bakim.md](../../plan/bakim.md) BK-01…BK-05.
- **Kural:** CLAUDE.md §8 (kod kuralları, 2 000 satır sınırı), §8.1 (tamamlanma), §25 kural 3 (bağımlılık denetimi).
- **İlgili:** [ADR 8054](8054-phase-1-memory-and-build-bottlenecks.md) §7 (kaynak boyu), [ADR 9195](9195-module-separation.md) §6
  (bağımlılık tabanı), [ADR 9199](9199-desktop-packages.md) §5 (çekirdeğin katmanları bekliyor),
  [ADR 9303](9303-generator-environment.md) (üreteçlerin ortamı).
- **Ürün:** değişmedi. **`.kcad`:** değişmedi. **Sözleşme:** değişmedi.

## Bağlam

Sahip 28 Kasım 2026'da projenin yönetilebilir ve sürdürülebilir kalması için kodun incelenmesini ve bir refactor
planı istedi. İnceleme kodun okunmasıyla yapıldı; bulgular dosya ve satırlarıyla
[plan/bakim.md](../../plan/bakim.md)'dedir. Bu ADR planın zeminini (Aşama 0) ve yöntemini karara bağlar.

Zemini ilgilendiren ölçümler (`main` `59d451456`):

| Konu | Ölçüm |
|---|---|
| Rust | 6 581 dosya, 2 575 064 satır |
| `cargo fmt --all -- --check` | 3 010 dosyayı değiştirmek istiyor: +224 110 / −62 005 satır |
| 2 000 satırı aşan dosya | tabanda 69; 54'ü biçimsiz; biçimlenince 19 dosya daha sınırı geçiyor (88) |
| Bekçiler | üçü kırmızı: `module_deps.py`, `source_size.py`, `scripts/arch/deps.mjs` |
| Sınama ikilileri | `crates/native/mcp/tests` 166, `crates/server/application/tests` 41 kök dosya |

Boy sınırının yan etkisi kodda görülüyor: `crates/native/application/src/arch/masses.rs` kütle için mahalin hata
kodunu veriyor ve nedenini yazıyor (“the code table is at its size”); sınıra yakın dosyalarda satırlar
sıkıştırılmış (`crates/native/headless/src/file_commands.rs` `run_typed_with_progress`).

## Karar

### 1. İnceleme yöntemi

Bir inceleme ya da refactor planı kodun okunmasından çıkar. Bulgu dosyası ve satırıyla yazılır; ölçüm bütün depoda
betikle yapılır; okunmayan yer açıkça söylenir. Alt ajanın bulgusu ipucudur: plana girmeden önce anılan satır açılıp
doğrulanır, doğrulanmayan öyle işaretlenir.

### 2. Biçim tabanı

- Biçim rustfmt'ın varsayılanıdır; `rustfmt.toml` eklenmez. Genişlik 120 denendi: 5 750 dosyayı değiştirirdi,
  varsayılan 3 010'u.
- `cargo fmt --all` tek mekanik commit'tir: yalnız boşluk ve satır kırılımı değişir. Commit `.git-blame-ignore-revs`'e
  yazılır. Çalışma alanının üyesi olmayan dış kod (`crates/shared/geographiclib-rs`, `crates/vendor/block`)
  biçimlenmez.
- Zamanı: bütün dallar `main`'e girmişken ve itilmemiş commit yokken (28 Kasım 2026'da durum buydu).
- Sonrasında `cargo fmt --all -- --check` denetim komutunun parçasıdır (§4).

### 3. Boy tabanının bir kerelik yeniden yazımı

`scripts/dev/source_size.py`'nin tabanı “yalnız küçülür”. Biçim commit'i satır sayısını değiştirdiği için taban bu bir
kez yeniden yazılır: biçimlenmiş kodun gerçek sayıları. Bu, kuralın istisnasıdır ve yalnız biçim commit'inin hemen
ardından yapılır; sonrasında kural eskisi gibidir.

Masaüstü çekirdeğinin kendi sınaması (`apps/desktop/src/tests/source_size_tests.rs`) tabansız kalır: biçimlenince
sınırı geçen çekirdek dosyası aynı dilimde konularına göre bölünür.

Sınır sıkıştırılmış metni değil biçimlenmiş kodu ölçer. Bir dosyayı sınırın altında tutmak için satır birleştirmek,
kod tablosuna yanlış kod yazmak ya da başka bir dolanma kabul edilmez: dosya bölünür.

### 4. Tek denetim komutu ve hook

`make guard` derleme istemeyen bütün bekçileri koşar: `scripts/arch/deps.mjs`, `scripts/dev/source_size.py`,
`scripts/dev/module_deps.py --check`, başka uygulama adı bekçisi, `scripts/cad/manifest.py --check`,
`cargo fmt --all -- --check`. `make arch` ona bağlanır.

Claude Code'un `Stop` hook'u (`.claude/settings.json`, depoda) oturum biterken `make guard`'ı koşar ve kırmızıyı
oturuma geri verir. Bu, 24 Kasım'ın “şimdilik otomatik denetim yok” kararını (todos-new.md F5) yalnız bu kadar
değiştirir: CI ve git hook'u yine yoktur.

### 5. Sınama ikilileri

Bir crate'in bütünleşik sınamaları tek ikilide toplanır (`tests/all/main.rs` ve onun modülleri), bugün
`crates/native/application` ve `crates/shared/kcad`'in yaptığı gibi. `tests/` altına doğrudan konan her dosya ayrı bir
program olarak bağlanır; bu Mac'te her ikilinin açılışı ayrıca bekler.

### 6. Sıra ve numaralar

Planın sırası: zemin → doğruluk düzeltmeleri → tek kaynak → masaüstü kabuğu → dev işlevler → ortak alt düzey
yardımcılar → `.kcad` şema zinciri → harness ve belgeler. Refactor davranışı değiştirmez; doğruluk düzeltmesi ayrı
commit'tir ve düzeltmeden önce kırmızı olan bir sınamayla gelir.

Bu planın ADR'leri 9400–9449 aralığındadır.

## Modül ve kanal tablosu

Zemin (Aşama 0) hiçbir komutu, aracı ya da arayüz yüzeyini değiştirmez.

| | Mimari | Harita | Yol | CBS | MCP | Python | Eklenti | Yapay zekâ ajanı |
|---|---|---|---|---|---|---|---|---|
| Biçim tabanı, bekçiler, sınama ikilileri | değişmedi | değişmedi | değişmedi | değişmedi | değişmedi | değişmedi | değişmedi | değişmedi |

## Doğrulama

- Biçim commit'i: `cargo fmt --all -- --check` temiz; `module_deps.py --check` çıktısı önce ve sonra aynı;
  `cargo build -p kentos-cad`.
- `make guard` yeşil.
- Sınama ikilileri: MCP ve sunucu sınamalarının sayısı önce ve sonra aynı; bağlama süresi ve `target/` boyu ölçülür.
