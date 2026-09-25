# ADR 0016: KentOS UI bu depoya taşındı (`kentos-rc` → `crates/ui`, `kentos-ui`)

- **Durum:** kabul edildi (2026-09-25, sahibin talimatı: “kentos-rc projesi artık bu projenin bir parçası olacak … ayrı git reposu olmayacak”)
- **Tarih:** 2026-09-25
- **Bağlam belgesi:** TODOS.md §6 (`UI-01..12`), ADR 0010; taşıma öncesi kayıt [docs/baseline/2026-09-25.md](../baseline/2026-09-25.md)
- **Kaynak:** `~/Projects/kentos-rc`, `master`, `dc2e8df`; 38 commit, 147 dosya, tek yazar

## Karar ve taşıma kaydı

- **Geçmişiyle birlikte, bu deponun sıradan dosyaları olarak.**
  - `git subtree` birleştirmesi (`100f6e3`) kaynağın 38 commit'ini bu deponun geçmişine kattı. `git blame` satırları asıl commit'lere bağlar.
  - Alt modül ya da ayrı depo yoktur.
  - Eski çalışma kopyası doğrulama bitene kadar diskte kalır (`UI-02`). Arşivleme ya da silme sahibin ayrı kararıdır.
- **Yerleşim:**
  - `crates/ui`: bileşen kütüphanesi. Tema, ikonlar, yazı, bileşenler, ekransız görüntü; `assets/fonts` altındaki OFL lisanslı yazı tipleri de burada.
  - `apps/ui-showcase`: vitrin uygulaması.
  - `include_bytes!` yolları kaynak dosyaya göreli olduğu için değişmedi (`UI-07`).
  - İç `[workspace]` bölümü ve ikinci `Cargo.lock` kaldırıldı (`UI-04`).
- **Adlar (`UI-03`):**
  - paket `kentos-ui`, crate `kentos_ui`;
  - vitrin paketi `kentos-ui-showcase`, ikili `kentos-ui-showcase`.

  Ad değişikliği yerleşimden ayrı bir commit'tir. Böylece çizimin taşımadan etkilenmediği adlar değişmeden önce kanıtlandı (aşağıda).
- **Çalışma alanı:**
  - UI crate'leri üyedir ama `default-members`'ta değildir. Web ve sunucu işi (`cargo build`, `cargo test`, `pnpm rust:test`) Iced ve wgpu derlemez (`UI-04`).
  - Masaüstü tarafı `pnpm rust:test:desktop` ve `make test-desktop` ile sınanır. `make check` ikisini de çalıştırır.
- **Bağımlılıklar (`UI-05`):**
  - `iced` 0.14.0, `iced_runtime` 0.14.0, `glam` 0.30.10, `bytemuck` 1.25.2, `png` 0.18.1 çalışma alanında `=` ile sabitlendi.
  - `kentos-rc`'nin kilit dosyası kök kilide katıldı; oradaki 291 paket sürümünün hiçbiri değişmedi (wgpu 27.0.1, naga 27.0.3, winit 0.30.13, cosmic-text 0.15.0, tiny-skia 0.11.4 …).
  - Cargo aynı ana sürümden tek sürüm tuttuğu için sunucu tarafında iki yama birleşti: `thiserror` 2.0.20 → 2.0.21 ve `zerocopy` 0.8.57 → 0.8.58. Sunucu testleri bu kilitle yeniden geçti.
  - Kilitte 625 paket var (önce 336); fazlası yalnız masaüstü derlemesine girer.
  - Lisanslar: yeni 291 paketin hepsi izin veren lisanslı. `self_cell` 1.3.0 seçimli lisanslıdır (“Apache-2.0 OR GPL-2.0-only”) ve Apache-2.0 ile kullanılır ([bağımlılık kaydı](../deps/README.md)).
- **Bağımlılık yönü (ADR 0010):**
  - `ui` grubu (`crates/ui/`) hiçbir çalışma alanı grubuna bağlanamaz. Sunucu çalışma zamanları (tokio dahil) ve tarayıcı bağları altında bulunamaz.
  - `desktop` grubu (`apps/desktop/`, `apps/ui-showcase/`) shared, native, ui ve render gruplarını kullanabilir. Yasakları axum ve tarayıcı bağlarıdır; Iced'in yürütücüsü tokio olabilir.

## Doğrulama

- **Görüntüler (`scripts/ui/snapshots.mjs`, tiny-skia):**
  - Adlar değişmeden önce, bu çalışma alanında derlenen vitrin 35 sahnenin **32'sini baseline ile bayt bayt aynı** çizdi.
  - Kalan üçü yalnız duvar saatini gösteren bölgelerde ayrılıyor:
    - `onay`: yeni nesnenin oluşturma zamanı 15:45 → 17:04:15;
    - `galeri-yerlesim` ve `galeri-geri-bildirim`: geçen süreyle ilerleyen çubuklar, 44 ve 8 piksel.
  - Betik bu üç sahneyi ayrı raporlar.
- **Ad değişikliğinden sonra** aynı karşılaştırmanın sonucu:
  - 22 sahne bayt bayt aynı.
  - 9 galeri sayfası yalnız ekrana yazdıkları crate yolu etiketlerinde ayrılıyor (`kentos_rc::theme::Mode` → `kentos_ui::theme::Mode`).
  - 4 sahne saat gösteriyor. Dördüncüsü `bildirimler`: bildirimin kalan süre çubuğu bir piksel uzadı.
  - Bundan sonraki karşılaştırmaların başvurusu `apps/ui-showcase/snapshots.json`'dır. `scripts/ui/snapshots.mjs --write` onu farkı okuduktan sonra yeniden yazar; taşıma öncesi kayıt `docs/baseline`'da kalır.
- **Testler:** paralel koşuda 170 kütüphane ve 55 vitrin testi geçti; 48 doctest `ignore` işaretli.
- **Baseline'daki paralel SIGSEGV'nin nedeni:**
  - Bileşen testleri ekransız çiziciyi aynı anda birçok iş parçacığında açıyor. Aynı anda açılan wgpu aygıtları GPU sürücüsünü çökertiyor.
  - Testler yazılım çiziciye zorlanınca 4 koşunun 4'ü geçti.
  - Çözüm: birim testlerinde varsayılan çizici tiny-skia (`cfg(test)`); `KENTOS_SNAPSHOT_BACKEND` yine önceliklidir. Testler böylece makinenin GPU'suna da bağlı değil.
- **Clippy (`-D warnings`, Rust 1.96):** tek bulgu, `docking.rs`'te elle yazılmış bir aralık denetimi (`manual_range_contains`). Önerilen biçim NaN'da farklı davranır, ama oran en az 1 olan bir paydadan geldiği için NaN olamaz; öneri uygulandı.

## Açık işler

- **Yorum dili:** `kentos-ui`'nin kod yorumları Türkçedir, CLAUDE.md §1 kod yorumlarını İngilizce ister. Çevirisi ayrı, mekanik bir dilimdir. O dilime kadar yeni yorumlar İngilizce yazılır.
- **Sabit saat:** vitrinin görüntü kipinde saat sabitlenmeli; üç sahne o zaman da birebir karşılaştırılabilir.
- **Tasarım jetonları (`UI-08`):** web'in `DESIGN.md` jetonları ile Rust temasının tek kaynağa bağlanması.
- **Tanıtım modeli (`UI-06`, `UI-10`):** `spatial` modülünün LonLat/Mercator örnek modeli ve vitrinin örnek verisi masaüstü uygulamasının alan modeli değildir. `apps/desktop` bunları kullanmaz; KentOS belgesine bağlanır.
- **Masaüstü kabuğu (`UI-11`):** vitrin sayfalarına bakarak kurulur. Web'in menü ve komut envanteri (`docs/inventory/web.json`) adım adım masaüstüne taşınır (sahibin talimatı, 25 Eylül).
