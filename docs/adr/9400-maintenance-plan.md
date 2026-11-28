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
`scripts/dev/module_deps.py --check`, `scripts/cad/manifest.py --check`, `cargo fmt --all -- --check`. Biri kırmızı
olsa da hepsi koşar. `make arch` aynı komuttur (eski ad); `make check` biçim denetimi yerine onu çağırır.

Başka uygulama adı bekçisi masaüstünün sınamasıdır (`apps/desktop/src/tests/other_names_tests.rs`) ve derleme ister;
`make guard`'a girmez, masaüstü dilimlerinin ortak sınamalarıyla koşar.

Claude Code'un `Stop` hook'u (`.claude/settings.json`, depoda; `scripts/dev/guard_hook.sh`) oturum dururken
`make guard`'ı koşar; kırmızıysa duruşu bir kez engeller ve bekçilerin çıktısını oturuma geri verir. HEAD ve çalışma
ağacı son yeşil koşudan beri değişmediyse koşmaz (`.run/guard.ok`). Ölçüm (28 Kasım 2026, bu Mac): yeşil koşu 25 sn,
değişiklik yokken 0,1 sn. Bu, 24 Kasım'ın “şimdilik otomatik denetim yok” kararını (todos-new.md F5) yalnız bu kadar
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

## Yapılanlar

- **BK-01 (28 Kasım 2026).** `cargo fmt --all` `27d390b97`: 3 008 dosya, +224 128 / −62 060 satır; üç dosya ikinci
  geçişte oturdu. `module_deps.py`'nin sayıları ve Rust kaynağını okuyan yedi üreteç ile `scripts/cad/manifest.py`,
  `scripts/crs/coverage.py` önce ve sonra aynı. Biçimlenince 2 488 satıra çıkan
  `apps/desktop/src/cad/surface_analysis_window.rs` üç dosyaya bölündü (pencere 1 412, `content.rs` 579, `chart.rs`
  516). Boy tabanı yeniden yazıldı: 69 → 87 dosya.
- **BK-02 (28 Kasım 2026).**
  - `kentos-standards → kentos-launch`: bağ doğrudur (paketlerin yeri `kentos_launch::install`'dan sorulur, CLAUDE.md
    §26); `scripts/arch/deps.mjs`'in `standards` grubu `launch`'ı kullanabilir.
  - Kabuk → cad ve gis → cad: yeni üç bağın hepsi `crate::labels`'ın `GlyphSlot`, `gpu_text` ve `text_run`'ınaydı. Bunlar
    etikete değil çizim alanının çizicisine aittir; `apps/desktop/src/shell/glyph_layer.rs`'e taşındı (yuva, metnin
    şekillendirilmesi, ölçüm boyu). `cad/labels` oradan alır ve eski adlarıyla yeniden dışa aktarır. `gpu_frame.rs`'in
    sınamaları kendi dosyasında (`gpu_frame_tests.rs`). Taban: kabuk → cad 77, gis → cad 54 (aşım yok), kabuk → gis
    42 → 41.
  - `shell/scene3d_sun.rs`'in kullanılmayan iki içe aktarımı kalktı (derleme uyarısızdır).
- **BK-03 (28 Kasım 2026).** `make guard`, `scripts/dev/guard_hook.sh`, `.claude/settings.json`. Hook dört yolda
  sınandı: yeşil, değişiklik yok, kırmızı (engeller, çıktıyı verir), zaten engellemiş (geçer).
- **BK-04 (28 Kasım 2026).**
  - MCP: 163 dosya `crates/native/mcp/tests/all/`'a taşındı, tek program (`main.rs`). Üç sınama kendi programında
    kaldı (`package_grids.rs`, `package_grids_found.rs`, `gnss_sessions.rs`): her biri `KENTOS_GRID_DIR`'i bütün süreci
    için değiştirir. Dosyaya göreli `include_str!` yolları bir düzey derinleşti; `CARGO_MANIFEST_DIR`'e göreli yollar
    değişmedi. Sonuç: 313 + 3 sınama, hepsi geçti; tek programın koşusu 126 sn, derlemesi 2 dk 2 sn.
    Ölçüm: bir MCP sınama programı 0,88 GB'tır (her biri bütün konağı bağlar); 166 program ≈ 146 GB, 4 program ≈ 3,5 GB.
  - Sunucu: 40 dosya ve `common` `crates/server/application/tests/all/`'a taşındı; `common` bir kez derlenir.
    `disk_full.rs` kendi programında kaldı (`scripts/ops/disk_full_drill.sh` onu tek başına derleyip koşar). Her
    veritabanı sınamasının kendi veritabanı olduğu için (`TestDb`) modüller durum paylaşmaz. Geçici bir PostGIS
    kabında (`KENTOS_TEST_DB=required`): 167 geçti, 4 yok sayıldı (süre ölçümleri), 62,7 sn.
  - Yığın: MCP'nin kabul sınaması 2 MB'lık sınama yığınını tek başına koşunca da taşırıyordu (birleştirmeden bağımsız;
    `pnpm rust:test` değişkeni vermediği için orada çökerdi). `.cargo/config.toml` `[env]` artık
    `RUST_MIN_STACK=67108864` verir; masaüstü sınamaları için de elle vermek gerekmez. Kök neden hata ayıklama
    derlemesinde dev işlevlerin yığın çerçeveleridir (BK-21, BK-40).
  - Kılavuzdaki 155 komut yeni hedefe çevrildi (`--test all <modül>::`). ADR'lerdeki ve `docs/baseline`'daki eski
    komutlar tarihsel kayıttır, değişmedi.
  - Kalan: 133 MCP sınama dosyasındaki aynı `fn call(…)` yardımcısı ortak bir modüle alınacak (BK-53).
- **BK-05 (28 Kasım 2026).** Ölçümler [docs/perf/build-bakim-2026-11-28.md](../perf/build-bakim-2026-11-28.md)'dedir.
  - Clippy `-D warnings` ile iki sınama komutunda da kırmızıydı: ortak ve yerel crate'lerde 15, masaüstü paketlerinde 26
    uyarı. Hepsi clippy'nin kendi önerisiyle giderildi; iki komut temiz.
  - Tek satır değişiklikte `cargo build -p kentos-cad`: modül paketi 3,7 sn, masaüstü çekirdeği 7,9 sn, uygulama
    katmanı 13,6 sn, geometri çekirdeği 36,4 sn; sıfırdan 176 sn.
  - Bulunan öbür kırmızılar (BK-02 ve BK-04'te giderildi): `other_names_tests` (iki metinde ADR atfı),
    `labels::gpu`'nun genel sayaca dayanan sınaması, MCP kabul sınamasının yığını.
  - `cargo test -p kentos-ui --lib` tek başına derlenmiyor: sınamaları `snapshot` özelliğini ister
    (`--features snapshot`; `pnpm rust:test:desktop` vitrinle birlikte koştuğu için derlenir). Olduğu gibi bırakıldı.
- **BK-20 (28 Kasım 2026).** Masaüstünün `history/text.rs`'indeki elle tutulan başlık tablosu (505 çift) silindi.
  Başlık komutun tanımında bir kez yazılır; katalog üretilirken başlıkların küçük dosyası da yazılır
  (`catalog/v1/command-titles.json`, 526 komut, 25 KB; `cargo test -p kentos-commands catalog` onu tanımlara tutar) ve
  masaüstü onu derleme zamanında gömüp ilk sorulduğunda okur. Kataloğun kendisi çalışma anında kurulmaz: şemalarıyla
  birlikte kurulması saniyeler sürer. Eski tablonun 505 çiftinin hepsi dosyadakiyle aynıydı. `text.rs` 1 226 → 704
  satır. BK-21'de komut tek tabloda bildirilince dosya o tablodan üretilir.

## Doğrulama

- Biçim commit'i: `cargo fmt --all -- --check` temiz; `module_deps.py --check` çıktısı önce ve sonra aynı;
  `cargo build -p kentos-cad`.
- `make guard` yeşil.
- Sınama ikilileri: MCP ve sunucu sınamalarının sayısı önce ve sonra aynı; bağlama süresi ve `target/` boyu ölçülür.
