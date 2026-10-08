# Bağımlılık kaydı

Tarih: 25 Eylül 2026, `27f771d`. Politika CLAUDE.md §3'tedir. Bu kayıt TODOS.md `BASE-06`'nın karşılığıdır. Bugünkü 42 Rust bağımlılığı aşağıdadır (26 Eylül'de `miniz_oxide`; 27 Eylül'de `tiny-skia`, `roxmltree` ve `zune-jpeg`; 28 Eylül'de `pyo3`; 3 Ekim'de pafta PDF'i için `pdf-writer`, `subsetter` ve `ttf-parser`, pafta resimleri için `resvg` ve Iced'in `image-without-codecs` ve `svg` özellikleri eklendi; 4 Ekim'de `roxmltree` biçim çekirdeğine, wasm32'ye de girdi; aynı gün elipsoit üstü ölçüler için `geographiclib-rs` eklendi; 8 Ekim'de nokta bulutunun LAZ'ı için `laz`). Lisans taramasını ve SBOM'u CI'a bağlamak ayrı iştir (`OPS-13`).

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
| libm | 0.2.16 | MIT | native, wasm32 | geometry-core, formats, ncz, sheet | ADR 0008 |
| rust_decimal | 1.43.0, yalnız `std` | MIT | native, wasm32 | geometry-core | ADR 0001, 0004 |
| geographiclib-rs | 0.2.7 (varsayılan: `accurate`), `libm`'li kopya `crates/shared/geographiclib-rs` | MIT | native, wasm32 | geometry-core (jeodezik uzunluk, ortası ve doğrultusu; köşelerin jeodezik çokgeninin alanı; `crs::ground`) | ADR 0171 (sahibin onayı, 4 Ekim): GeographicLib'in (Karney 2013) Rust kapısı, georust'ın bakımında. crates.io sürümünden `scripts/vendor/geographiclib.py` ile yazılan kopya (arşivi Cargo'nun SHA-256'sıyla denetlenir; `--check`): 51 aşkın yöntem çağrısı `libm`'den, masaüstü ve web aynı bitleri verir (sürüm glibc'ninkileri çağırıyordu); Karney'nin GeodTest'inde en büyük hata 11 nm (sürüm 7,5 nm). Çalışma alanının üyesi değil. Getirdikleri: `accurate` 0.3.1 (MIT OR Apache-2.0; `PolygonArea`'nın toplamları) ve `ieee754` 0.2.6 (MIT/Apache-2.0); `cfg-if` ve `num-traits` kilitliydi |
| laz | 0.13.0, varsayılan özellikler kapalı (`parallel` yok: rayon girmez) | Apache-2.0 | native (masaüstü); wasm32 için derlenebilir (web'in nokta bulutu modülü bekliyor) | pointcloud (LAZ'ın, yani LASzip'in parça sıkıştırıcıları ve çözücüleri: masaüstünün dizini, önizlemesi, İşlemler'in okuması ve yazması) | ADR 0207 (sahibin onayı, 8 Ekim): laz-rs, LASzip'in saf Rust kapısı, bakımda (tmontaigu). LAS'ın ve COPC'nin okuyucusu, dizini ve yazıcısı KentOS'undur; crate'ten yalnız parça sıkıştırma ve çözme kullanılır. Parçaları ev sahibi kendi iş parçacıklarında açar. Kilide tek paket girdi (`byteorder` ve `num-traits` kilitliydi) |
| ts-rs | 12.0.1, `serde-json-impl` | MIT | native (yalnız TS üretimi, `ts` özelliği) | contracts | ADR 0001, 0002 |
| wasm-bindgen | 0.2.128 | MIT OR Apache-2.0 | wasm32 | geometry-wasm, formats-wasm, dxf-wasm, ncz-wasm, svg-wasm | ADR 0001 (`wasm-bindgen-cli` aynı sürüm); ADR 0138 |
| schemars | 1.2.2, `derive`, `std` | MIT | native, wasm32 (derlenebilir; tarayıcı paketlerine girmez) | contracts (`schema` özelliği); katalog testlerinde application ve native-application (`schema` özelliği, yalnız geliştirme) | ADR 0013 (sahibin onayı, 25 Eylül). Getirdikleri: `schemars_derive` (MIT), `dyn-clone`, `ref-cast`, `ref-cast-impl`, `serde_derive_internals` (MIT OR Apache-2.0) |
| iced | 0.14.0 (varsayılan özellikler; `canvas`, `advanced`, vitrinde `debug`, `tokio`; sheet-ui'de `image-without-codecs` ve `svg`) | MIT | native (masaüstü) | ui, ui-showcase, desktop (`canvas`: araç önizlemesi, ADR 0021); sheet-ui (paftanın resimleri: doku ve SVG) | ADR 0016 (`kentos-rc`'nin test edilmiş sürümü). `image-without-codecs` ve `svg`: sahibin onayı, 3 Ekim (docs/sheet/design.md açık soru 6): resimler doku olarak çizilir, büyütülünce yumuşar; SVG resimleri resvg çizer. Kod çözücüsüz: resimleri sheet-ui kendisi çözer (`png`, `zune-jpeg`). Getirdikleri aşağıda |
| resvg | 0.45.1, varsayılan özellikler kapalı; sheet-ui'de `text`, `system-fonts` | Apache-2.0 OR MIT | native (masaüstü) | sheet-ui (eklenen SVG resminin boyu, `usvg` ile; PDF'e giden SVG resminin PNG'si, yazıları sistemin yazı tipleriyle, ekrandaki gibi) | Sahibin onayı, 3 Ekim (design açık soru 6, “resvg/usvg”). Iced'in SVG desteğinin çizdiği sürüm; Iced onu varsayılan özellikleriyle (`text`, `system-fonts`, `memmap-fonts`, `raster-images`) getirir, bu yüzden ikilide o özelliklerle derlenir. `text` ve `system-fonts`'un adı sheet-ui'de açıkça yazılı (9. adım); kilide paket girmedi |
| iced_runtime | 0.14.0 (isteğe bağlı: `snapshot`) | MIT | native | ui; desktop (iz oynatıcısı uygulamanın görevlerini çalıştırır) | ADR 0016, 0021; paket zaten Iced ve KentOS UI üzerinden ikilideydi, kilide yeni paket girmedi |
| glam | 0.30.10 (isteğe bağlı: `spatial`) | MIT OR Apache-2.0 | native | ui | ADR 0016 |
| bytemuck | 1.25.2, `derive` (isteğe bağlı: `spatial`) | Zlib OR Apache-2.0 OR MIT | native | ui, render-wgpu | ADR 0016, 0019 |
| png | 0.18.1 (isteğe bağlı: `snapshot`) | MIT OR Apache-2.0 | native | ui; desktop (stil kitaplığının PNG görüntüleri); sheet-ui (paftanın PNG çıktısı ve resimleri); sheet (yalnız test: PDF yazıcısının kendi PNG okuyucusu buna karşı sınanır) | ADR 0016, 0090 |
| tiny-skia | 0.11.4, varsayılan özellikler kapalı, `std`, `simd` | BSD-3-Clause | native (masaüstü) | render-wgpu (stilli çizimin atlası: SVG ve raster işaretler, yazılar, desen döşemeleri) | ADR 0090 (sahibin onayı, 27 Eylül); iced_tiny_skia üzerinden aynı özelliklerle ikilideydi, kilide yeni paket girmedi |
| roxmltree | 0.20.0 (varsayılan özellikler) | MIT OR Apache-2.0 | native (masaüstü); wasm32 (biçim çekirdeği) | desktop (kitaplığın SVG çizimleri; web'de tarayıcı okur); formats (Trimble JobXML karnesi, sonra GPX; iki platformda) | ADR 0090 (sahibin onayı, 27 Eylül); fontdb (fontconfig-parser) üzerinden aynı özelliklerle kilitliydi, kilide yeni paket girmedi. Ortak biçim çekirdeğine, yani web'in WASM'ına girişi ADR 0169 5c (sahibin onayı, 4 Ekim): biçim WASM'ı JobXML okuyucusuyla birlikte +77 KB (gzip +30 KB); ayrıştırıcısı öğe başına özyinelediğinden belgeler önce 256 düzey sınırıyla taranır (`formats::xml`) |
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
| sha2 | 0.10.9, varsayılan özellikler kapalı | MIT OR Apache-2.0 | native, wasm32 | contracts (sha256); kcad (dosya özeti, ADR 0025); application (yüklenen dosyanın özeti, ADR 0031); api testleri; sheet (varlıkların adı, PDF'in belge kimliği ve alt küme etiketi) | ADR 0014; zaten kilitliydi (sqlx). wasm32 hedefi sahibin onayıyla, 25 Eylül |
| jsonwebtoken | 11.1.0, `rust_crypto` | MIT | native | api (OpenID) | ADR 0007 |
| reqwest | 0.13.5, `rustls` | MIT OR Apache-2.0 | native | api (OpenID); cloud (masaüstünün bulut istemcisi) | ADR 0007, 0040; masaüstüne girişi kilide yeni paket getirmedi |
| bytes | 1.12.1 | MIT | native | cloud (bir yüklemenin baytları, her denemede kopyasız) | ADR 0040; reqwest ve hyper üzerinden zaten her ikilideydi, kilide yeni paket girmedi |
| time | 0.3.55 | MIT OR Apache-2.0 | native | application, api | ADR 0007 |
| tracing | 0.1.44 | MIT | native | api | ADR 0007 |
| tracing-subscriber | 0.3.23 | MIT | native | api | ADR 0007 |
| pyo3 | 0.29.2, varsayılan özellikler kapalı, `macros`, `abi3-py310` | MIT OR Apache-2.0 | native (CPython eklentisi) | python (`kentos._native`, Python SDK'sının yerel modülü; `default-members` dışında) | ADR 0131 (sahibin onayı, 28 Eylül). Kararlı ABI: CPython 3.10 ve sonrası için platform başına tek tekerlek. Eskimiş `extension-module` özelliği kullanılmaz; maturin `PYO3_BUILD_EXTENSION_MODULE`'ü kendisi koyar. Kilide girenler: `pyo3`, `pyo3-ffi`, `pyo3-build-config`, `pyo3-macros`, `pyo3-macros-backend` (MIT OR Apache-2.0) ve `target-lexicon` 0.13.5 (Apache-2.0 WITH LLVM-exception) |
| miniz_oxide | 0.9.1, varsayılan özellikler kapalı, `with-alloc` | MIT OR Zlib OR Apache-2.0 | native, wasm32 (saf Rust) | formats (zip'li Shapefile, `zip.rs`; yalnız inflate); sheet (PDF'in akışları, yazı tipleri ve resimleri deflate ile; resimlerin PNG'leri inflate ile) | ADR 0046 soru 4 (sahibin onayı, 26 Eylül), ADR 0053; `flate2` üzerinden zaten kilitliydi (`adler2` ile), kilide yeni paket girmedi. Biçim WASM modülü zip okumaz: +183 bayt. Pafta: docs/sheet/design.md §9a (sıkıştırma için kilitteki paket) |
| pdf-writer | 0.15.0 (varsayılan özellikler) | MIT OR Apache-2.0 | native, wasm32 (saf Rust) | sheet (pafta PDF'i ve GeoPDF'i: nesneler, içerik akışları, isteğe bağlı içerik grupları, `VP`/`Measure`) | Sahibin onayı, 3 Ekim (docs/sheet/design.md §9a). Typst'in yazıcısı, bakımda. Kilide tek paket girdi; bağımlılıkları (`bitflags`, `itoa`, `memchr`, `ryu`) zaten kilitliydi. Sayıları `f32` yazar: GeoPDF'in `GPTS` enlem-boylamları on ondalıkla yer tutucuya sonradan yazılır (tasks-rust.md Sapmalar) |
| subsetter | 0.2.6, varsayılan özellikler kapalı (`variable-fonts` yok) | MIT OR Apache-2.0 | native, wasm32 (saf Rust) | sheet (PDF'e gömülen yazı tiplerinin alt kümesi) | Sahibin onayı, 3 Ekim (design §9a). Typst'in alt kümeleyicisi. Kilide tek paket girdi (`rustc-hash` zaten kilitliydi); varsayılan özellik `skrifa` 0.42, `write-fonts` ve `kurbo` getirirdi, çizim yüzleri durağan TrueType olduğu için kapalı |
| ttf-parser | 0.25.1, varsayılan özellikler kapalı, `std` | MIT OR Apache-2.0 | native, wasm32 (saf Rust) | sheet (yazı tiplerinin glif kimlikleri, ilerlemeleri, adları, sınır kutusu) | design §9a (yazı tipi okuma için kilitteki paket; koordinatörün Adım 5 talimatı, sahibin onayladığı adım). fontdb ve cosmic-text üzerinden native için zaten kilitliydi; yeni hedef wasm32'dir (pafta WASM modülü), kilide yeni paket girmedi |

**Geçişli bağımlılıklar** (`Cargo.lock`): 687 paket (3 Ekim, pafta resimleri: +30, aşağıda), 33'ü çalışma alanının kendi crate'leri (28 Eylül; PyO3'ün beş paketi ve `target-lexicon` girdi; sonra `kentos-ncz`, `kentos-dxf-wasm` ve `kentos-ncz-wasm` eklendi, dış paket girmedi; 3 Ekim'de pafta düzeninin `kentos-sheet`, `kentos-sheet-wasm` ve `kentos-sheet-ui` crate'leri ile dış paket olarak `pdf-writer` ve `subsetter` girdi).

- Masaüstü arayüzü (Iced, wgpu, winit, cosmic-text, tiny-skia …) 291 paket getirdi (ADR 0016). Hepsi taranmıştır.
- **Pafta resimleri (3 Ekim, sahibin onayı; Iced'in `image-without-codecs` ve `svg` özellikleri, `resvg`):**
  30 paket. Lisanslar `~/.cargo/registry`'deki manifestlerden tarandı; hepsi izin veren lisanslardır.
  Yalnız masaüstü derlemesine girerler.
  - Iced'in resim desteği: `image` 0.25.10 (MIT OR Apache-2.0; kod çözücüsüz), `kamadak-exif` 0.6.1
    (BSD-2-Clause; resmin EXIF yönü), `mutate_once` 0.1.2 (BSD-2-Clause), `byteorder-lite` 0.1.0
    (Unlicense OR MIT). `image`'ın kendi bağımlılıkları da gelir: `moxcms` 0.8.1 ve `pxfm` 0.1.30
    (BSD-3-Clause OR Apache-2.0; renk profili).
  - Iced'in SVG desteği (resvg): `resvg`, `usvg` 0.45.1, `svgtypes` 0.15.3, `kurbo` 0.11.3,
    `simplecss` 0.2.2 (Apache-2.0 OR MIT); `rustybuzz` 0.20.1, `float-cmp` 0.9.0, `imagesize` 0.13.0,
    `xmlwriter` 0.1.0, `pico-args` 0.5.0, `rgb` 0.8.53 (MIT); `data-url` 0.3.2, `siphasher` 1.0.4
    (MIT OR Apache-2.0); `unicode-bidi-mirroring`, `unicode-ccc` 0.4.0, `unicode-vo` 0.1.0,
    `quick-error` 2.0.1 (MIT/Apache-2.0).
  - resvg'nin varsayılan `raster-images` özelliği (SVG'nin içine gömülü PNG/JPEG/GIF/WebP resimleri
    çizer; Iced resvg'yi varsayılan özellikleriyle aldığından kapatılamaz): `gif` 0.13.3,
    `image-webp` 0.2.4, `weezl` 0.1.12, `png` 0.17.16 (MIT OR Apache-2.0); `color_quant` 1.1.0
    (MIT); `zune-jpeg` 0.4.21, `zune-core` 0.4.12 (MIT OR Apache-2.0 OR Zlib).
  - İkinci sürümler: `png` 0.17.16 (0.18.1'in yanında) ve `zune-jpeg` 0.4.21 (0.5.15'in yanında).
    Bunları resvg ister; çalışma alanınınkiler değişmedi.
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

## Gömülü veri

Kod değil, derlemeye giren dış veri. Satırı bağımlılık gibi yazılır: kaynak, sürüm, lisans, kullanan.

| Veri | Sürüm ve geçerlilik | Lisans | Kaynak | Kullanan | Karar |
|---|---|---|---|---|---|
| Dünya Manyetik Modeli (WMM2025), NOAA/NCEI ve BGS | 17 Aralık 2024'te yayımlandı (dosyanın başlığı 13 Kasım 2024); geçerlilik 2025,0–2030,0; 12. dereceye kadar 90 katsayı satırı ve yıllık değişimleri | Kamu malı: ABD hükümetinin eseri, ABD'de telif yok (17 U.S.C. § 105); lisans gerekmez | <https://www.ncei.noaa.gov/sites/default/files/2024-12/WMM2025COF.zip> (`WMM.COF`, SHA-256 `dfa85978…582f`); modelin sayfası <https://www.ncei.noaa.gov/products/world-magnetic-model> | sheet (`crates/shared/sheet/data/wmm2025.json`, `scripts/geodesy/wmm_coefficients.py` resmî dosyadan üretir; resmî dosya ve NOAA'nın sınama değerleri `fixtures/sheet/v1/wmm/`'de) | docs/sheet/design.md §8a (sahibin onayı, 3 Ekim). Hesap çekirdektedir (`wmm.rs`, `libm`); yeni paket yok. 2030'dan sonra WMM2030 aynı betikle girer |

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
