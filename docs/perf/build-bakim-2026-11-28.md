# Derleme ve denetim ölçümü: bakım planının tabanı (2026-11-28, 4342f25b6)

Apple M5, 10 çekirdek, 32 GB; macOS 27.2.1; rustc 1.100.0; hata ayıklama profili (`debug = "line-tables-only"`,
bağımlılıklarda hata ayıklama bilgisi yok); `-j 10`. Her ölçüm bir kez; tek satır değişikliği iki kez (yaz ve geri al).
Plan: [plan/bakim.md](../../plan/bakim.md) BK-05; karar: [ADR 9400](../adr/9400-maintenance-plan.md).

## Derleme: `cargo build -p kentos-cad`

Tek satır değişikliği: dosyanın ortasındaki bir öğenin ardına bir yorum satırı. Gövdeyi değiştiren bir düzenleme daha
uzun sürer; bu, alt sınırdır.

| Değişen dosya | Yeniden derlenen | Süre |
|---|---|---|
| `apps/desktop-road/src/sight_window.rs` | Yol paketi, program | 3,7 sn / 3,6 sn |
| `apps/desktop/src/cad/flatten_window.rs` | masaüstü çekirdeği, dört modül paketi, program | 7,9 sn / 8,0 sn |
| `crates/native/application/src/core/delete.rs` | uygulama katmanı ve ona bağlı 16 crate | 13,6 sn / 13,6 sn |
| `crates/shared/geometry-core/src/geometry.rs` | geometri çekirdeği ve ona bağlı 56 crate | 36,4 sn / 36,3 sn |
| sıfırdan (kendi `target` dizini) | her şey | 176,4 sn; `target` 16 GB |

## Denetim ve sınama

| Ölçüm | Sonuç |
|---|---|
| `make guard` (derleme istemez) | 25 sn; değişiklik yokken hook 0,1 sn |
| `cargo fmt --all` | 12 sn |
| `cargo clippy --all-targets -- -D warnings` (varsayılan üyeler) | 2 dk 7 sn; 15 uyarı vardı, şimdi temiz |
| aynı, masaüstü paketleri | 2 dk 3 sn; 26 uyarı vardı, şimdi temiz |
| `cargo test -p kentos-mcp --test all` | 313 sınama, 126 sn; programın derlemesi 2 dk 2 sn |
| `cargo test -p kentos-application --test all` (geçici PostGIS) | 167 geçti, 4 yok sayıldı, 63 sn |
| masaüstü sınama programının derlemesi (`-p kentos-desktop --lib`) | 58 sn – 1 dk 41 sn (önceki derlemeye göre) |

## Boyutlar

| Ölçüm | Sonuç |
|---|---|
| Rust | biçim tabanından önce 6 581 dosya, 2 575 064 satır; bu commit'te 6 587 dosya, 2 737 446 satır |
| 2 000 satırı aşan dosya (biçimlenmiş) | 87 |
| Bir MCP sınama programı | 0,88 GB; 166 program ≈ 146 GB, 4 program ≈ 3,5 GB |
| `target/debug` | 186 GB (ölçümden önce) |

Yorum: tek satır değişikliğin maliyeti crate'in büyüklüğünden çok ona bağlı crate sayısıyla artıyor. Masaüstü
çekirdeğinde 8 sn, çekirdeği crate'lere bölme kararını (ADR 9199 §5) öne almayı gerektirmiyor; geometri çekirdeğine ve
sözleşmelere dokunan iş 36 sn öder.
