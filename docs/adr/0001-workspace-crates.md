# ADR 0001: Rust çalışma alanı ve crate sınırları (Faz A)

- **Durum:** kabul edildi
- **Tarih:** 2026-09-23
- **Bağlam belgesi:** CLAUDE.md §14, §20 Faz A
- **Sonraki kararlar (2026-09-25):** masaüstü native Iced + `kentos-ui` + saf wgpu çizim alanıdır ([ADR 0010](0010-platform-boundaries.md)); planlı `crates/tiles` kapsamdan çıktı ([ADR 0012](0012-server-scope-project-cloud.md)). Aşağıdaki tarihsel metin korunur, değişen yerler işaretlidir.

## Bağlam

Tarayıcı uygulaması (`apps/web/src/`) TypeScript'tir ve olduğu gibi kalır. CLAUDE.md §13–14 ortak bir Rust çekirdeği istiyor. Bu çekirdek tarayıcıda WASM, sunucuda yerel kod olarak çalışacak. Yanında sürümlü sözleşmeler ve bir Rust API'si olacak.

Sunucu yığını kesindir: Axum, Tokio, SQLx (PostgreSQL + PostGIS) ve Tower. Faz A yalnızca en küçük çalışan dilimi kurar; crate sayısı gereksiz yere artırılmaz.

## Karar

- Depo kökünde tek bir Cargo çalışma alanı var (`Cargo.toml`, resolver 3, edition 2024). Faz A üyeleri:

  | Crate | Görev | Bağımlılık |
  |---|---|---|
  | `crates/shared/geometry-core` | Saf analitik geometri (f64), `apps/web/src/model/geom` ve `apps/web/src/model/geometry.ts`'in birebir karşılığı; §23 sayısal politikası (`numeric`); çağrı tablosu (`api`, ADR 0008) | Yalnızca saf crate'ler (rust_decimal, libm, serde, serde_json). Dosya sistemi, ağ, Tokio, SQLx, HTTP ya da WebGPU'ya bağlanamaz; wasm32 için de derlenir |
  | `crates/shared/contracts` | Sürümlü veri sözleşmeleri; TS tipleri buradan üretilir (ADR 0002) | serde, serde_json, ts-rs |
  | `crates/wasm/geometry-wasm` | Çekirdeğin tarayıcı sınırı: çağrı tablosu (JSON) ve düz `Float64Array` giriş-çıkışlı sıcak yollar (ADR 0008) | geometry-core, wasm-bindgen |
  | `apps/api` | HTTP API; Faz A'da yalnızca `GET /v1/health` | contracts, axum, tokio |

- Bağımlılıklar kullanıcı onayıyla eklendi ve tam sürüme kilitlendi (`=`). `Cargo.lock` depoya girer.

  | Crate | Sürüm |
  |---|---|
  | serde | 1.0.229 |
  | serde_json | 1.0.151 |
  | rust_decimal | 1.43.0 (yalnızca `std`) |
  | ts-rs | 12.0.1 |
  | wasm-bindgen | 0.2.128 |
  | libm | 0.2.16 (ADR 0008) |
  | axum | 0.8.9 |
  | tokio | 1.53.1 |

- Araç zinciri `rust-toolchain.toml` ile 1.96.0'a sabitlenir; `wasm32-unknown-unknown` hedefi zincirle birlikte kurulur.
- WASM paketi `wasm-bindgen-cli` 0.2.128 ile üretilir (`pnpm rust:wasm`, `--target web`, `apps/web/src/wasm/pkg`). Komutun sürümü `wasm-bindgen` crate'iyle aynı olmak zorundadır; paket depoya girmez. `apps/web/src/wasm/golden.wasm.test.ts` paketin çalışma alanı sürümüyle derlendiğini denetler, eski paket testte yakalanır.
- **Derleme sınırı:** `.cargo/config.toml` derlemeyi 4 işle sınırlar. Geliştirici makinesi tarayıcı, Vite ve başsız Chrome ile paylaşılıyor. Paralel ağır süreçler makineyi bir kez kilitledi. Cargo derlerken e2e ya da başka bir ağır iş çalıştırılmaz.
- ~~Ana `pnpm test` Rust araç zincirine bağımlı değildir.~~ **2026-09-24'te değişti (ADR 0008):** uygulama geometriyi Rust çekirdeğinden aldığı için `pnpm dev`, `test`, `build` ve `e2e` önce `scripts/wasm/ensure.mjs` ile WASM paketini (gerekirse) derler; Rust araç zinciri bunlar için de gereklidir. `serde_json` `float_roundtrip` özelliğiyle kullanılır; paket `wasm` profiliyle (`lto = "fat"`, `panic = "abort"`) derlenir. `pnpm test:rust` Rust testleri, clippy ve WASM testlerini birlikte çalıştırır.

## Sonuçlar

- `apps/web/src/` ağacı taşınmadı. Rust çekirdeği TypeScript'e parça parça girecek. Aynı anlam iki dilde uzun süre paralel geliştirilmez (§14).
- CLAUDE.md §14'teki diğer bileşenler ilgili fazda eklenir:
  - `apps/worker`;
  - `crates/server/application` (ortak kullanım durumları, yetki, iş sözleşmesi);
  - `crates/server/postgres` (SQLx, PostGIS SQL, migration);
  - ~~`crates/tiles` (Martin arkasında TileJSON/MVT, önbellek geçersizleştirme);~~ **2026-09-25'te kapsamdan çıktı ([ADR 0012](0012-server-scope-project-cloud.md)):** sunucunun görevi proje bulutudur, Martin rolü ve tile crate'i şimdiki kapsamda yoktur;
  - `crates/style-core`.
- SQLx ve Tower Faz B'de PostgreSQL ile birlikte girer. Faz A'da veritabanı yoktur.

## Güncelleme (2026-09-24): monorepo dizin düzeni

Kullanıcı kararı: geometri çekirdeği ve öteki ortak Rust kodu ileride bir wgpu masaüstü CAD/CBS uygulamasında da kullanılacak; depo bir ön yüz projesi gibi değil, üç istemcinin (web, sunucu, masaüstü) ortak kodunu taşıyan bir monorepo gibi düzenlenir. (2026-09-25: masaüstü Iced + `kentos-ui` + saf wgpu çizim alanıyla native bir Rust uygulamasıdır, web'e derlenmez; [ADR 0010](0010-platform-boundaries.md).)

- **Tarayıcı uygulaması `apps/web/`'e taşındı** (`src/`, `index.html`, `public/`, Vite ve TypeScript yapılandırması, yalnız web'e ait betikler: e2e, perf, showcase, TS fixture kaydedicileri). pnpm çalışma alanının tek paketidir (`@kentos/web`, `pnpm-workspace.yaml`). Kökteki `package.json` bütün komutları tutar ve web komutlarını `apps/web`'e iletir; `pnpm dev`, `test`, `build`, `e2e` eskisi gibi kökten çalışır.
- **Crate'ler rolüne göre gruplandı:**

  | Grup | Crate'ler | Kural |
  |---|---|---|
  | `crates/shared/` | `geometry-core`, `contracts`, `formats` | Platformdan bağımsız: DOM, wasm-bindgen, Tokio, SQLx, HTTP ya da wgpu'ya bağlanamaz. Web (WASM), sunucu ve masaüstü aynen kullanır |
  | `crates/wasm/` | `geometry-wasm` (eski `crates/wasm`, paket `kentos-wasm` → `kentos-geometry-wasm`), `formats-wasm` | Yalnız tarayıcı sınırı; hesap yapmaz |
  | `crates/server/` | `postgres`, `application` | Yalnız sunucu |
  | `apps/` | `api` (ileride `worker`, `desktop`) | Çalıştırılabilirler |

- **`contracts`'ta TS üretimi isteğe bağlı:** ts-rs `ts` özelliğinin arkasındadır (varsayılan açık, `cargo test` üretilen tipleri güncel tutar). Yalnız Rust tipleri isteyenler (`formats`, `formats-wasm`, `application`, `apps/api`, ileride masaüstü) `default-features = false` ile bağlanır ve ts-rs derlemez.
- Paylaşılan `fixtures/`, WASM derleme denetimi (`scripts/wasm/ensure.mjs`) ve Python referans üreticileri (`scripts/fixtures/*.py`) kökte kaldı. WASM paketleri `apps/web/src/wasm/pkg` ve `apps/web/src/io/pkg`'ye, ts-rs çıktısı `apps/web/src/contracts/generated`'e yazılır.
- Hiçbir hesap, API ya da sözleşme değişmedi; üretilen TS tipleri ve WASM paketlerinin içeriği aynı kaldı. Masaüstü uygulaması eklenirken `apps/desktop` bir Cargo üyesi olur ve `crates/shared/*`'ı doğrudan kullanır; tarayıcıya özgü `crates/wasm`'a bağlanmaz.
