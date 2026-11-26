# paket — kurulum paketinin kaynak dosyaları

Bu klasörün içi kurulum paketine olduğu gibi kopyalanır: macOS'ta `KentOS.app/Contents/Resources/`, Windows'ta ve
Linux'ta programın klasörü. Program bu dosyaları yalnız okur; yerlerini tek bir yerden sorar (`kentos_launch::install`).
Kural ve tablo CLAUDE.md §26'da, gerekçe ADR 9312'dedir. Geliştirmede program bu klasörü `target/` üstündeki depodan
bulur.

| Klasör | Ne | Kurulumda |
|---|---|---|
| `fonts/` | Programın okuduğu yazı tipi dosyaları ve lisansları | `<kaynak>/fonts/` |
| `eklentiler/` | KentOS ile gelen eklentiler, her biri bir klasör (örnek Python işlem aracı: `ornek-isleme/`) | `<kaynak>/eklentiler/` |
| `betikler/` | Örnek Python betikleri | `<kaynak>/betikler/` |

Buraya konmayanlar ve yerleri (CLAUDE.md §26): programlar `<program>/`; standart kural paketleri
`catalog/standards/` → `<kaynak>/catalog/standards/`; Python SDK tekerleği ve uv `<program>/tools/` (macOS'ta tekerlek
`<kaynak>/python/`, uv `<program>/`). Çizimin gömülü yazı tipleri (`apps/desktop/assets/fonts/drawing/`) programın
içindedir; lisans metinleri onların yanındadır ve pakette `<kaynak>/fonts/lisanslar/` altına da kopyalanır.
