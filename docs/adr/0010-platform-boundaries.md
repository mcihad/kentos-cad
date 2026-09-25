# ADR 0010: Platform sınırları: bağımsız web, native masaüstü, ortak Rust hesabı

- **Durum:** kabul edildi (2026-09-25, sahibin kararı: CLAUDE.md §0, TODOS.md “Kesin web sınırı”)
- **Tarih:** 2026-09-25
- **Bağlam belgesi:** CLAUDE.md §0, §4, §14; TODOS.md §2, §6, §8.1, §23.1
- **Değiştirdiği:** ADR 0001 “Güncelleme (2026-09-24)”daki masaüstü notunu somutlaştırır; eski CLAUDE.md §24.4'teki (27f771d'ye kadar) “Tauri/Electron ve yerel veri adaptörü ayrıca ADR ile seçilir” notunu kapatır. ADR 0008'in web için çizdiği sınır aynen sürer.

## Bağlam

KentOS'ta üç çalıştırılabilir yüz var ya da olacak: tarayıcı uygulaması, sunucu ve masaüstü uygulaması.

- ADR 0001 ve 0008 hesabı tek Rust kaynağına topladı (`crates/shared/*`). Tarayıcı bu crate'leri dar WASM bağlayıcılarıyla çağırır, arayüz ve belge TypeScript'te kalır.
- ADR 0001'in 2026-09-24 güncellemesi masaüstünü yalnız “ileride bir wgpu masaüstü CAD/CBS uygulaması” diye anıyordu.
- Eski CLAUDE.md ise masaüstü/çevrimdışı paket için Tauri ya da Electron seçeneğini açık bırakıyordu.

Sahip 25 Eylül'de yönü kesinleştirdi. UI bileşen kütüphanesi `~/Projects/kentos-rc`'de Iced 0.14 ve wgpu 27 üzerinde hazırdır (`dc2e8df`). Bu depoya henüz taşınmadı.

## Karar

- **Web bağımsız bir TypeScript/DOM uygulamasıdır.**
  - Belge, komut, etkileşim, ayarlar ve çizim orkestrasyonu TypeScript'te kalır (`apps/web/src/`).
  - WebGPU ve WebGL2 arka uçları korunur.
  - Rust'tan web'e yalnız hesap ve kodek kütüphaneleri girer, dar WASM bağlayıcılarıyla: bugün `geometry-wasm`, `formats-wasm`, `svg-wasm`.
- **Masaüstü native bir Rust uygulamasıdır.** Iced, `kentos-ui` ve saf wgpu ana CAD çizim alanından oluşur.
  - `kentos-ui` `kentos-rc`'den `crates/ui` olarak alınacak (paket `kentos-ui`, crate `kentos_ui`).
  - Ana çizim alanı KentOS'un kendi wgpu hattıdır. Kütüphanenin Iced Canvas demo çizim alanı üretim motoru sayılmaz.
  - Masaüstü bir webview kabuğu değildir: Tauri/Electron seçeneği kapandı.
- **Masaüstünün uygulama çalışma zamanı web'e derlenmez.** Iced, native application/interaction/settings katmanları ve Rust wgpu renderer'ı tarayıcı paketine girmez. Uygulamanın tamamı WASM'a çevrilmez.
- **Ortak olanlar:**
  - saf Rust hesap crate'leri (`crates/shared/*`: geometri, stil ve ifade, SVG, sayısal politika, biçimler);
  - sürümlü veri ve komut sözleşmeleri (`crates/shared/contracts` ve oradan üretilen TS tipleri);
  - platformlar arası fixture'lar;
  - uygun WGSL shader kaynakları (hedef yol `shaders/wgsl/`, TODOS.md `REN-04`).

  Her platform kendi cihazını, pipeline'ını, buffer sahipliğini ve kare zamanlayıcısını tutar. Web'e Rust wgpu Device/Queue ya da Iced aktarılmaz.
- **Sunucu native Rust'tır** (`apps/api`, `crates/server/*`). Masaüstüyle uygun native domain ve servis kodunu paylaşabilir, web'le paylaşamaz.
- **Uyum ortak çalışma zamanıyla kurulmaz.** Sürümlü sözleşme, ortak fixture ve davranış (conformance) testleriyle kurulur. Aynı basit düzenleme web'de, masaüstünde ve sunucuda uyumlu sonuç ve hata üretmelidir (TODOS.md §2 kabulü).
- **Python ve AI**, bulundukları host'un sürümlü komut girişine bağlanır. Ayrıntılar kendi ADR'lerinde karara bağlanacak (TODOS.md §14–15).

## Sonuçlar

- Web'in bugünkü mimarisi ve çalışma akışı değişmez. CLAUDE.md §4 ve ADR 0008'in “yalnız hesap Rust'ta, arayüz TypeScript'te” kuralı web için aynen geçerlidir.
- Masaüstü yolları (`apps/desktop`, `apps/ui-showcase`, `crates/ui`, `crates/native/*`, `crates/render/wgpu`, `shaders/wgsl/`) hedeftir. İlk gerçek tüketicisi oluşunca açılır, boş crate ağacı kurulmaz (TODOS.md §2.1). Mevcut `crates/server/application` paketinin adı `kentos-application`'dır. Native application katmanı eklenirken bu çakışma ayrıca çözülür.
- **Denetim (bugün):** `apps/web/src/model/singleSource.test.ts` web cephelerinde hesabın TS'e geri kopyalanmasını yakalar.
- **Bağımlılık yönü (TODOS.md `ARCH-01`, 25 Eylül):** `scripts/arch/deps.mjs` her crate'i derlendiği hedefte `cargo tree` ile gezer. `pnpm rust:test`'in sonunda ya da `pnpm arch:deps` ile çalışır. Normal ve build bağımlılıkları sayılır, özellik ve hedef süzgeçleri uygulanır.

  | Grup (yol) | Hedef | Kullanabileceği gruplar | Altında bulunamayanlar |
  |---|---|---|---|
  | `shared` (`crates/shared/`) | makine, wasm32 | shared | tokio, sqlx, axum, hyper, tower, reqwest, wasm-bindgen, js-sys, web-sys, iced, wgpu, winit, naga, pyo3, gdal, proj |
  | `wasm` (`crates/wasm/`) | wasm32 | shared | sunucu çalışma zamanları (tokio … reqwest), iced, wgpu, winit, naga, pyo3 |
  | `native` (`crates/native/`, henüz yok) | makine | shared, native | sqlx, axum, tarayıcı bağları, iced, wgpu, winit, naga, pyo3 |
  | `server` (`crates/server/`), `api` (`apps/api/`) | makine | shared, native, server | tarayıcı bağları, iced, wgpu, winit, naga |

  - Hiçbir grubun kapsamadığı yoldaki bir crate denetimi durdurur. İlk UI, renderer ya da masaüstü crate'i kurallarıyla birlikte bilinçli olarak eklenir.
  - Port kuralı: port arayüzünü iç katman (domain/application) tanımlar, gerçekleştirmesini dış adapter (PostgreSQL, dosya sistemi, tarayıcı) sahiplenir (`ARCH-02`).
- **Denetim (sonra):** web paketinde Iced, native application ya da Rust renderer bulunmadığını bağımlılık ve varlık denetimiyle koruyan test ilk masaüstü crate'iyle eklenecek (TODOS.md `TEST-02`, `ARCH-01`).
- Yeni bağımlılıklar (Iced, wgpu, naga, winit …) bu depoya girerken sürüm, lisans, platform ve gerekçe kaydı ister (TODOS.md `BASE-06`, [bağımlılık kaydı](../deps/README.md)). `kentos-rc`'deki kilitli sürümler (Iced 0.14.0, wgpu 27.0.1) başlangıç noktasıdır, kesin seçim değildir (TODOS.md `UI-05`).
