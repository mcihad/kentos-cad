# IFC okuma motoru denemesi — 16 Kasım 2026

ADR 8061 §7'nin ölçümleri (plan/ifc-uyum.md IFC-07, bim-plan B12). Motor: IFClite'ın Rust çekirdeği `ifc-lite-core` ve
`ifc-lite-geometry` 28.3.0 (sabit sürüm, `=28.3.0`), yalnız `kentos-ifc`'in `read` ve `geometry` özelliklerinin
arkasında.

- **Makine:** Apple M5, 32 GB, macOS 27.2.1; `rustc 1.100.0 (0ef1bc386 2026-11-09)`; `-j 10`.
- **Kod:** `codex/bim-ifc-uyum`, 3583b863c'nin üstündeki B12 işi (commit iletisinde).
- **Ölçen:** `crates/shared/ifc/examples/ifc_read.rs` (release), tepe bellek `/usr/bin/time -l`'in “maximum resident set
  size”ı. Büyük dosya `scripts/fixtures/ifc_large_case.py 700`: kesit binasının 700 kopyası, her kopyanın kayıt numaraları
  bir milyon kaydırılmış ve GlobalId'leri kendine ait (Git'e girmez, `.run/ifc-buyuk/`).

## Derleme ve boyut

| Ölçü | Değer |
|---|---|
| `read`'in eklediği paket | 12 (ifc-lite-core, nom, fast-float2, futures-*, twox-hash, rustc-hash…) |
| `geometry`'nin ayrıca eklediği | 35 (ifc-lite-geometry, nalgebra, i_overlay, rayon, png, jpeg-decoder…) |
| Lisanslar | iki motor paketi MPL-2.0; öbür 45'i MIT, Apache-2.0, ISC, Zlib, 0BSD (izin veren) |
| wasm32-unknown-unknown | `geometry` ile derleniyor, uyarısız |
| Release derleme (`kentos-ifc` + motor, motor önbellekte yokken) | 67 s duvar, 285 s işlemci |
| Örnek programın boyutu (release, arm64) | motorsuz 0,43 MB; `read` 1,34 MB (+0,91 MB); `geometry` 6,03 MB (+4,69 MB) |

## Süre ve bellek

| Dosya | Kayıt | Nesne | Yüz (özet, nesneler, ağaç, bulgular) | Gövdeler | Tepe bellek |
|---|---|---|---|---|---|
| kesit-binasi.ifc (156 KB) | 2 929 | 91 | 1–3 ms | 70 gövde, 2 892 üçgen, 13 ms | 9 MB (gövdelerle) |
| kesit-binasi-x700.ifc (135 MB) | 2 050 300 | 63 700 | 0,72 s | 49 000 gövde, 2 024 400 üçgen, 8,39 s | 322 MB (yüz), 913 MB (gövdelerle) |

## İptal

| Ne zaman istendi | Ne zaman durdu |
|---|---|
| Yüz okunurken, 100 ms'de | 0,100 s (≤ 1 ms gecikme) |
| Yüz okunurken, 400 ms'de | 0,401 s |
| Gövdeler yapılırken, 2 000 ms'de | 2,057 s (57 ms) |
| Gövdeler yapılırken, 5 000 ms'de | 5,226 s (226 ms) |

## Gerçek dünyadan örnekler

buildingSMART'ın sertifika veri setlerinin 35 IFC dosyası (CC BY 4.0, commit `80d976a9b`, 16,4 MB; ADR 8061 §11.7):
en büyüğü 2,3 MB (`Infra-Landscaping`, 27 419 kayıt). Her dosyada yüz 6 ms'nin, gövdeler 16 ms'nin altında okundu; 0 bulgu,
şekilli nesnelerin hepsi gövdeli.

Taban budur; hedef bu ölçümden sonra konur (ADR 8061 §7).
