# Bağımlılık kaydı

Tarih: 25 Eylül 2026, `27f771d`. Politika CLAUDE.md §3'tedir. Bu kayıt TODOS.md `BASE-06`'nın karşılığıdır. Bugünkü 36 Rust bağımlılığı aşağıdadır (26 Eylül'de `miniz_oxide`; 27 Eylül'de `tiny-skia`, `roxmltree` ve `zune-jpeg`; 28 Eylül'de `pyo3` eklendi). Lisans taramasını ve SBOM'u CI'a bağlamak ayrı iştir (`OPS-13`).

## Kurallar

- Yeni bir runtime bağımlılığı, sürüm yükseltmesi ya da mevcut bağımlılığın yeni bir hedefe taşınması sahibin onayını ister (CLAUDE.md §3).
- Onaylanan her bağımlılık bu tabloya bir satırla girer: tam sürüm, lisans, hedef (native / wasm32 / tarayıcı), kullanan paket ve kararın yazıldığı ADR. Bakım durumu ve ölçülmüş etki (paket boyutu, derleme süresi, sıcak yol) aynı ADR'de gerekçelenir.
- Rust sürümleri çalışma alanında `=` ile kilitlidir. `Cargo.lock` ve `pnpm-lock.yaml` depoya girer.
- Web runtime kütüphanesi küçük, tree-shaking'e uygun, MIT/BSD lisanslı ve bakımda olmalı; DOM'suz, worker'da çalışabilmeli (CLAUDE.md §3).
- Saf hesap crate'leri (`crates/shared/*`) DOM, Iced, SQLx, ağ ya da host runtime'a bağlanamaz (CLAUDE.md §14, ADR 0010).
- **Aday** tablosundaki bir satır kurulmuş ya da onaylanmış bağımlılık değildir. Karar ilgili fazın ADR'sinde verilir.

## Türetilmiş kod ve dağıtım koşulu

| Crate | Kaynak | Lisans | Koşul |
|---|---|---|---|
| `kentos-ncz` (`crates/shared/ncz`), `kentos-ncz-wasm` (`crates/wasm/ncz-wasm`) | Erdinç Örsan ÜNAL, QGIS eklentisi NCZ Reader 1.4.3 (`ncz_pure.py`), <https://github.com/erdincunal/Jeomatik-NCZ-Reader> | GPL-2.0-or-later (kaynağınki) | Yazarın izni (MIT, Apache-2.0 ya da yazılı izin) gelene dek NCZ okuyucusunu içeren hiçbir derleme dağıtılmaz; sahibin kararı, 28 Eylül. TODOS.md `NCZ-01`, ADR 0138 §2 |

Dış bağımlılık değildir: kod bu depodadır ve kilide dış paket getirmez. Masaüstü ikilisi bu crate'i içerdiğinden koşul masaüstünün dağıtımını da bağlar; web'de koşul yalnız NCZ modülünü (`apps/web/src/io/ncz/pkg`) bağlar, çünkü o ayrı yüklenir.

## Rust: çalışma alanı (`Cargo.toml` → `[workspace.dependencies]`)

| Crate | Sürüm ve özellikler | Lisans | Hedef | Kullanan | Karar |
|---|---|---|---|---|---|
| serde | 1.0.229, `derive` | MIT OR Apache-2.0 | native, wasm32 | contracts, formats, formats-wasm, dxf-wasm, application, api, kcad; processing (bir modelin adımlarındaki değerler yazıldığı sırayla okunur; kilide yeni paket girmedi, ADR 0116) | ADR 0001 |
| serde_json | 1.0.151, `float_roundtrip` | MIT OR Apache-2.0 | native, wasm32 | contracts, formats, formats-wasm, dxf-wasm, ncz-wasm, application, api; ncz (yalnız test); processing (işlem araçlarının değerleri, ADR 0084); test (domain, native-application dahil), kcad | ADR 0001, 0008 |
| libm | 0.2.16 | MIT | native, wasm32 | geometry-core, formats, ncz | ADR 0008 |
| rust_decimal | 1.43.0, yalnız `std` | MIT | native, wasm32 | geometry-core | ADR 0001, 0004 |
| ts-rs | 12.0.1, `serde-json-impl` | MIT | native (yalnız TS üretimi, `ts` özelliği) | contracts | ADR 0001, 0002 |
| wasm-bindgen | 0.2.128 | MIT OR Apache-2.0 | wasm32 | geometry-wasm, formats-wasm, dxf-wasm, ncz-wasm, svg-wasm | ADR 0001 (`wasm-bindgen-cli` aynı sürüm); ADR 0138 |
| schemars | 1.2.2, `derive`, `std` | MIT | native, wasm32 (derlenebilir; tarayıcı paketlerine girmez) | contracts (`schema` özelliği); katalog testlerinde application ve native-application (`schema` özelliği, yalnız geliştirme) | ADR 0013 (sahibin onayı, 25 Eylül). Getirdikleri: `schemars_derive` (MIT), `dyn-clone`, `ref-cast`, `ref-cast-impl`, `serde_derive_internals` (MIT OR Apache-2.0) |
| iced | 0.14.0 (varsayılan özellikler; `canvas`, `advanced`, vitrinde `debug`, `tokio`) | MIT | native (masaüstü) | ui, ui-showcase, desktop (`canvas`: araç önizlemesi, ADR 0021) | ADR 0016 (`kentos-rc`'nin test edilmiş sürümü) |
| iced_runtime | 0.14.0 (isteğe bağlı: `snapshot`) | MIT | native | ui; desktop (iz oynatıcısı uygulamanın görevlerini çalıştırır) | ADR 0016, 0021; paket zaten Iced ve KentOS UI üzerinden ikilideydi, kilide yeni paket girmedi |
| glam | 0.30.10 (isteğe bağlı: `spatial`) | MIT OR Apache-2.0 | native | ui | ADR 0016 |
| bytemuck | 1.25.2, `derive` (isteğe bağlı: `spatial`) | Zlib OR Apache-2.0 OR MIT | native | ui, render-wgpu | ADR 0016, 0019 |
| png | 0.18.1 (isteğe bağlı: `snapshot`) | MIT OR Apache-2.0 | native | ui; desktop (stil kitaplığının PNG görüntüleri) | ADR 0016, 0090 |
| tiny-skia | 0.11.4, varsayılan özellikler kapalı, `std`, `simd` | BSD-3-Clause | native (masaüstü) | render-wgpu (stilli çizimin atlası: SVG ve raster işaretler, yazılar, desen döşemeleri) | ADR 0090 (sahibin onayı, 27 Eylül); iced_tiny_skia üzerinden aynı özelliklerle ikilideydi, kilide yeni paket girmedi |
| roxmltree | 0.20.0 (varsayılan özellikler) | MIT OR Apache-2.0 | native (masaüstü) | desktop (kitaplığın SVG çizimleri; web'de tarayıcı okur) | ADR 0090 (sahibin onayı, 27 Eylül); fontdb (fontconfig-parser) üzerinden aynı özelliklerle kilitliydi, kilide yeni paket girmedi |
| zune-jpeg | 0.5.15, varsayılan özellikler kapalı, yalnız `std` (SIMD yolları `x86`, `neon` kapalı) | MIT OR Apache-2.0 OR Zlib | native (masaüstü) | desktop (kitaplığın JPEG görüntüleri: stilli çizimin atlası ve Stil yöneticisinin içe alması; web'de tarayıcı çözer) | ADR 0092 (sahibin onayı, 27 Eylül). Saf Rust, bakımda; image crate'inin JPEG çözücüsüdür, bakım kipindeki `jpeg-decoder`'ın yerini almıştır. Kilide iki paket girdi: `zune-jpeg` ve `zune-core` 0.5.3 (aynı lisans). Kenar başına 16 384 piksel sınırı çözücünündür |
| rfd | 0.17.2 (varsayılan: `xdg-portal`, `wayland`) | MIT | native (masaüstü) | desktop | ADR 0017 (sahibin onayı, 25 Eylül). Getirdiği tek yeni paket `pollster` (Apache-2.0 OR MIT) |
| wgpu | 27.0.1 (Iced'in kilitlediği sürüm, varsayılan özellikler) | MIT OR Apache-2.0 | native (masaüstü) | render-wgpu | ADR 0019; kilide yeni paket girmedi |
| naga | 27.0.3, `wgsl-in` (yalnız test) | MIT OR Apache-2.0 | native (test) | render-wgpu | ADR 0019 |
| pollster | 0.4.0 (yalnız test) | Apache-2.0 OR MIT | native (test) | render-wgpu GPU testi | ADR 0019 |
| axum | 0.8.9, `ws` | MIT | native | api | ADR 0001 |
| http-body | 1.1.0 | MIT | native | api (akışla indirilen dosyanın gövdesi, yalnız `Frame` türü) | ADR 0031; zaten kilitliydi (axum, hyper), kilide yeni paket girmedi |
| tokio | 1.53.1 | MIT | native | api, postgres; application (`fs`, `io-util`: dosya projelerinin nesne deposu, ADR 0031); cloud (masaüstü bulut istemcisinin kendi çalışma zamanı, ADR 0040) | ADR 0001 |
| sqlx | 0.9.0, `tls-none` | MIT OR Apache-2.0 | native | postgres, application, api | ADR 0006, 0007. TLS'siz yalnız yerel sunucu içindir; üretim TLS'i açıktır (`OPS-03`) |
| tower | 0.5.3 | MIT | native | api | ADR 0007 |
| tower-http | 0.7.1 | MIT | native | api | ADR 0007 |
| uuid | 1.26.1, `v4`, `v7` | Apache-2.0 OR MIT | native | postgres, application, api, domain, cloud | ADR 0007, 0020 |
| sha1 | 0.10.7, varsayılan özellikler kapalı | MIT OR Apache-2.0 | native, wasm32 | contracts (UUIDv5) | ADR 0014; zaten kilitliydi (axum). wasm32 hedefi sahibin onayıyla, 25 Eylül |
| sha2 | 0.10.9, varsayılan özellikler kapalı | MIT OR Apache-2.0 | native, wasm32 | contracts (sha256); kcad (dosya özeti, ADR 0025); application (yüklenen dosyanın özeti, ADR 0031); api testleri | ADR 0014; zaten kilitliydi (sqlx). wasm32 hedefi sahibin onayıyla, 25 Eylül |
| jsonwebtoken | 11.1.0, `rust_crypto` | MIT | native | api (OpenID) | ADR 0007 |
| reqwest | 0.13.5, `rustls` | MIT OR Apache-2.0 | native | api (OpenID); cloud (masaüstünün bulut istemcisi) | ADR 0007, 0040; masaüstüne girişi kilide yeni paket getirmedi |
| bytes | 1.12.1 | MIT | native | cloud (bir yüklemenin baytları, her denemede kopyasız) | ADR 0040; reqwest ve hyper üzerinden zaten her ikilideydi, kilide yeni paket girmedi |
| time | 0.3.55 | MIT OR Apache-2.0 | native | application, api | ADR 0007 |
| tracing | 0.1.44 | MIT | native | api | ADR 0007 |
| tracing-subscriber | 0.3.23 | MIT | native | api | ADR 0007 |
| pyo3 | 0.29.2, varsayılan özellikler kapalı, `macros`, `abi3-py310` | MIT OR Apache-2.0 | native (CPython eklentisi) | python (`kentos._native`, Python SDK'sının yerel modülü; `default-members` dışında) | ADR 0131 (sahibin onayı, 28 Eylül). Kararlı ABI: CPython 3.10 ve sonrası için platform başına tek tekerlek. Eskimiş `extension-module` özelliği kullanılmaz; maturin `PYO3_BUILD_EXTENSION_MODULE`'ü kendisi koyar. Kilide girenler: `pyo3`, `pyo3-ffi`, `pyo3-build-config`, `pyo3-macros`, `pyo3-macros-backend` (MIT OR Apache-2.0) ve `target-lexicon` 0.13.5 (Apache-2.0 WITH LLVM-exception) |
| miniz_oxide | 0.9.1, varsayılan özellikler kapalı, `with-alloc` (yalnız inflate kullanılır) | MIT OR Zlib OR Apache-2.0 | native, wasm32 (saf Rust) | formats (zip'li Shapefile, `zip.rs`) | ADR 0046 soru 4 (sahibin onayı, 26 Eylül), ADR 0053; `flate2` üzerinden zaten kilitliydi (`adler2` ile), kilide yeni paket girmedi. Biçim WASM modülü zip okumaz: +183 bayt |

**Geçişli bağımlılıklar** (`Cargo.lock`): 652 paket, 30'u çalışma alanının kendi crate'leri (28 Eylül; PyO3'ün beş paketi ve `target-lexicon` girdi; sonra `kentos-ncz`, `kentos-dxf-wasm` ve `kentos-ncz-wasm` eklendi, dış paket girmedi).

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
| Python 3 | yalnız standart kitaplık (`decimal`, `fractions`, `math`, `json`, `random`, `re`) | `scripts/fixtures/*.py` bağımsız referans üreticileri; `scripts/python/sdk.py` (Python SDK'sının üreticisi) |
| CPython | 3.10 ve sonrası (kararlı ABI); SDK'nın çalışma zamanı yalnız standart kitaplık | `python/` paketi `kentos` (ADR 0131) |
| maturin | 1.15.0 (`>=1.9.4,<2`; MIT OR Apache-2.0), `.run/py` ortamında | `python/pyproject.toml`'un derleme arka ucu; `scripts/python/test.sh` kurar (ADR 0131, sahibin onayı, 28 Eylül) |
| PostgreSQL + PostGIS | `postgis/postgis:18-3.6` (yerel geliştirme kabı) | ADR 0006 |

## Aday: kurulmadı, ilgili fazda karar verilecek

| Aday | Görev | Karar yeri |
|---|---|---|
| CBOR kodlayıcısı (ör. `ciborium`, `minicbor`), sıkıştırma kodeği | Binary `.kcad` | `FILE-01`, `FILE-07`, `FILE-11`; KCAD v2 ADR'si ([ADR 0011](../adr/0011-kcad-binary-snapshot.md)) |
| sqlx TLS özelliği (`rustls`) | Üretim veritabanı bağlantısı | `OPS-03` |
| S3 uyumlu nesne deposu istemcisi | Bulut dosya revizyonları | `SYNC-02`, `SYNC-03` |
| PROJ, GDAL bağlayıcıları | Dönüşüm ve biçimler (native/sunucu) | `NUM-05`, `NUM-06`, `FMT-04` |
| Masaüstüne gömülü CPython (konsol, betik paneli); Pyodide | Python | `PY-01`, `PY-02`, `PY-18` (PyO3'ün kendisi ADR 0131 ile kurulu) |
| MCP uygulaması | AI yüzeyi | `AI-03`, `AI-04` |
