# Bağımlılık kaydı

Tarih: 25 Eylül 2026, `27f771d`. Politika CLAUDE.md §3'tedir. Bu kayıt TODOS.md `BASE-06`'nın karşılığıdır. Bugünkü 29 Rust bağımlılığı aşağıdadır. Lisans taramasını ve SBOM'u CI'a bağlamak ayrı iştir (`OPS-13`).

## Kurallar

- Yeni bir runtime bağımlılığı, sürüm yükseltmesi ya da mevcut bağımlılığın yeni bir hedefe taşınması sahibin onayını ister (CLAUDE.md §3).
- Onaylanan her bağımlılık bu tabloya bir satırla girer: tam sürüm, lisans, hedef (native / wasm32 / tarayıcı), kullanan paket ve kararın yazıldığı ADR. Bakım durumu ve ölçülmüş etki (paket boyutu, derleme süresi, sıcak yol) aynı ADR'de gerekçelenir.
- Rust sürümleri çalışma alanında `=` ile kilitlidir. `Cargo.lock` ve `pnpm-lock.yaml` depoya girer.
- Web runtime kütüphanesi küçük, tree-shaking'e uygun, MIT/BSD lisanslı ve bakımda olmalı; DOM'suz, worker'da çalışabilmeli (CLAUDE.md §3).
- Saf hesap crate'leri (`crates/shared/*`) DOM, Iced, SQLx, ağ ya da host runtime'a bağlanamaz (CLAUDE.md §14, ADR 0010).
- **Aday** tablosundaki bir satır kurulmuş ya da onaylanmış bağımlılık değildir. Karar ilgili fazın ADR'sinde verilir.

## Rust: çalışma alanı (`Cargo.toml` → `[workspace.dependencies]`)

| Crate | Sürüm ve özellikler | Lisans | Hedef | Kullanan | Karar |
|---|---|---|---|---|---|
| serde | 1.0.229, `derive` | MIT OR Apache-2.0 | native, wasm32 | contracts, formats, formats-wasm, application, api | ADR 0001 |
| serde_json | 1.0.151, `float_roundtrip` | MIT OR Apache-2.0 | native, wasm32 | contracts, formats, formats-wasm, application, api; test (domain dahil) | ADR 0001, 0008 |
| libm | 0.2.16 | MIT | native, wasm32 | geometry-core, formats | ADR 0008 |
| rust_decimal | 1.43.0, yalnız `std` | MIT | native, wasm32 | geometry-core | ADR 0001, 0004 |
| ts-rs | 12.0.1, `serde-json-impl` | MIT | native (yalnız TS üretimi, `ts` özelliği) | contracts | ADR 0001, 0002 |
| wasm-bindgen | 0.2.128 | MIT OR Apache-2.0 | wasm32 | geometry-wasm, formats-wasm, svg-wasm | ADR 0001 (`wasm-bindgen-cli` aynı sürüm) |
| schemars | 1.2.2, `derive`, `std` | MIT | native, wasm32 (derlenebilir; tarayıcı paketlerine girmez) | contracts (`schema` özelliği) | ADR 0013 (sahibin onayı, 25 Eylül). Getirdikleri: `schemars_derive` (MIT), `dyn-clone`, `ref-cast`, `ref-cast-impl`, `serde_derive_internals` (MIT OR Apache-2.0) |
| iced | 0.14.0 (varsayılan özellikler; `canvas`, `advanced`, vitrinde `debug`, `tokio`) | MIT | native (masaüstü) | ui, ui-showcase | ADR 0016 (`kentos-rc`'nin test edilmiş sürümü) |
| iced_runtime | 0.14.0 (isteğe bağlı: `snapshot`) | MIT | native | ui | ADR 0016 |
| glam | 0.30.10 (isteğe bağlı: `spatial`) | MIT OR Apache-2.0 | native | ui | ADR 0016 |
| bytemuck | 1.25.2, `derive` (isteğe bağlı: `spatial`) | Zlib OR Apache-2.0 OR MIT | native | ui, render-wgpu | ADR 0016, 0019 |
| png | 0.18.1 (isteğe bağlı: `snapshot`) | MIT OR Apache-2.0 | native | ui | ADR 0016 |
| rfd | 0.17.2 (varsayılan: `xdg-portal`, `wayland`) | MIT | native (masaüstü) | desktop | ADR 0017 (sahibin onayı, 25 Eylül). Getirdiği tek yeni paket `pollster` (Apache-2.0 OR MIT) |
| wgpu | 27.0.1 (Iced'in kilitlediği sürüm, varsayılan özellikler) | MIT OR Apache-2.0 | native (masaüstü) | render-wgpu | ADR 0019; kilide yeni paket girmedi |
| naga | 27.0.3, `wgsl-in` (yalnız test) | MIT OR Apache-2.0 | native (test) | render-wgpu | ADR 0019 |
| pollster | 0.4.0 (yalnız test) | Apache-2.0 OR MIT | native (test) | render-wgpu GPU testi | ADR 0019 |
| axum | 0.8.9, `ws` | MIT | native | api | ADR 0001 |
| tokio | 1.53.1 | MIT | native | api, postgres; application testleri | ADR 0001 |
| sqlx | 0.9.0, `tls-none` | MIT OR Apache-2.0 | native | postgres, application, api | ADR 0006, 0007. TLS'siz yalnız yerel sunucu içindir; üretim TLS'i açıktır (`OPS-03`) |
| tower | 0.5.3 | MIT | native | api | ADR 0007 |
| tower-http | 0.7.1 | MIT | native | api | ADR 0007 |
| uuid | 1.26.1, `v4`, `v7` | Apache-2.0 OR MIT | native | postgres, application, api, domain | ADR 0007, 0020 |
| sha1 | 0.10.7, varsayılan özellikler kapalı | MIT OR Apache-2.0 | native, wasm32 | contracts (UUIDv5) | ADR 0014; zaten kilitliydi (axum). wasm32 hedefi sahibin onayıyla, 25 Eylül |
| sha2 | 0.10.9, varsayılan özellikler kapalı | MIT OR Apache-2.0 | native, wasm32 | contracts (sha256) | ADR 0014; zaten kilitliydi (sqlx). wasm32 hedefi sahibin onayıyla, 25 Eylül |
| jsonwebtoken | 11.1.0, `rust_crypto` | MIT | native | api (OpenID) | ADR 0007 |
| reqwest | 0.13.5, `rustls` | MIT OR Apache-2.0 | native | api (OpenID) | ADR 0007 |
| time | 0.3.55 | MIT OR Apache-2.0 | native | application, api | ADR 0007 |
| tracing | 0.1.44 | MIT | native | api | ADR 0007 |
| tracing-subscriber | 0.3.23 | MIT | native | api | ADR 0007 |

**Geçişli bağımlılıklar** (`Cargo.lock`): 628 paket, 14'ü çalışma alanının kendi crate'leri.

- Masaüstü arayüzü (Iced, wgpu, winit, cosmic-text, tiny-skia …) 291 paket getirdi (ADR 0016). Hepsi taranmıştır.
- Yalnız masaüstü derlemesine girerler; web ve sunucu derlemesi (`default-members`) onları derlemez.

- Lisanslar `~/.cargo/registry`'deki manifestlerden tarandı (25 Eylül). Hepsi izin veren lisanslardır: MIT, Apache-2.0, BSD-2/3-Clause, ISC, Zlib, Unicode-3.0, CC0-1.0, BSL-1.0, Unlicense ve bunların birleşimleri.
- Tek seçimli lisans `self_cell` 1.3.0'dır (“Apache-2.0 OR GPL-2.0-only”). Apache-2.0 ile kullanılır. Copyleft'e bağlı bir kullanım yoktur.
- Kalan 35'i yalnız başka işletim sistemlerinde derlenen paketlerdir (macOS `core-foundation`, Android `jni` …) ve Linux'ta indirilmediği için taranmadı.

## Web (`apps/web/package.json`)

- **Runtime bağımlılığı yok.** Uygulamanın kendi kodu ve Rust'tan üretilen WASM paketleri dışında tarayıcıya kütüphane gitmez.
- **Geliştirme araçları.** Aralıklar `package.json`'da, kilitli sürümler `pnpm-lock.yaml`'dadır:

  | Paket | Aralık | Kilitli | Lisans |
  |---|---|---|---|
  | typescript | `~6.0.2` | 6.0.3 | Apache-2.0 |
  | vite | `^8.3.0` | 8.3.0 | MIT |
  | vitest | `^5.0.1` | 5.0.1 | MIT |

- **Gömülü yazı tipleri** (`apps/web/src/assets/fonts/`): 13 aile. Architects Daughter, Arimo, Barlow, Courier Prime, IBM Plex Mono, IBM Plex Sans, Inter, Noto Sans, Overpass, Plus Jakarta Sans, Quicksand, Roboto ve Source Sans 3'ün hepsi SIL OFL 1.1'dir. Her ailenin `OFL.txt`'i yanındadır. CDN'den yazı tipi alınmaz (CLAUDE.md §8).

## Araçlar

| Araç | Sürüm | Kaynak |
|---|---|---|
| Rust | 1.96.0, `wasm32-unknown-unknown` hedefiyle | `rust-toolchain.toml` |
| wasm-bindgen-cli | 0.2.128 | crate sürümüyle aynı olmak zorunda (CLAUDE.md §2) |
| Node, pnpm | 24.16.0, 11.24.0 (referans makinede, 25 Eylül; depoda kilitli değil) | `docs/baseline/` |
| Python 3 | yalnız standart kitaplık (`decimal`, `fractions`, `math`, `json`, `random`, `re`) | `scripts/fixtures/*.py` bağımsız referans üreticileri |
| PostgreSQL + PostGIS | `postgis/postgis:18-3.6` (yerel geliştirme kabı) | ADR 0006 |

## Aday: kurulmadı, ilgili fazda karar verilecek

| Aday | Görev | Karar yeri |
|---|---|---|
| CBOR kodlayıcısı (ör. `ciborium`, `minicbor`), sıkıştırma kodeği | Binary `.kcad` | `FILE-01`, `FILE-07`, `FILE-11`; KCAD v2 ADR'si ([ADR 0011](../adr/0011-kcad-binary-snapshot.md)) |
| sqlx TLS özelliği (`rustls`) | Üretim veritabanı bağlantısı | `OPS-03` |
| S3 uyumlu nesne deposu istemcisi | Bulut dosya revizyonları | `SYNC-02`, `SYNC-03` |
| PROJ, GDAL bağlayıcıları | Dönüşüm ve biçimler (native/sunucu) | `NUM-05`, `NUM-06`, `FMT-04` |
| PyO3 + gömülü CPython; Pyodide | Python | `PY-01`, `PY-02`, `PY-18` |
| MCP uygulaması | AI yüzeyi | `AI-03`, `AI-04` |
