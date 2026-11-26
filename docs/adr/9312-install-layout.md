# ADR 9312: Kurulum paketinin dizinleri (`paket/`, `kentos_launch::install`)

- **Durum:** kabul edildi (2026-11-26). Sahibin isteği (26 Kasım 2026): uygulama henüz paketlenmedi. Örnek Python
  işlem aracının, eklentilerin ve Python betiklerinin hangi dizinde olacağı belirlensin; paketlerken bir şey
  kaçırılmasın diye kurulum paketine girecek dizinler standarda bağlansın ve CLAUDE.md'ye yazılsın. Yazı tipleri de
  paketin `fonts` dizinindedir.
- **İlgili:** [ADR 0800](0800-kentos-launcher.md) (başlatıcı, programlar yan yana), [ADR 8057](8057-phase-3-extension-packages.md)
  (eklenti klasörü, ilke dosyası), [ADR 9310](9310-processing-tools-by-module.md) (örnek araç),
  [ADR 9311](9311-arial-times-point-names-labels-off.md) (yazı tipleri), [ADR 9313](9313-fonts-folder-listing-upright-texts.md)
  (bütün yazı tipleri `fonts`'ta, listeler oradan).
- **Kural:** CLAUDE.md §26 (yeni bölüm).

## Bağlam

Programın çalışırken okuduğu dosyalar dört ayrı kurala dağılmıştı. Kurulum paketi henüz yoktu.

| Ne | Önceki yeri | Kuralı nerede |
|---|---|---|
| Python SDK tekerleği | programın yanındaki `tools/`; macOS'ta `Contents/Resources/python` | `kentos_python_env::sdk::dirs_for` |
| uv | `tools/`; macOS'ta `Contents/MacOS` | `kentos_python_env::uv::bundled_dirs_for` |
| Standart kural paketleri | programın sekiz üst klasöründen birinde `catalog/standards` | `kentos_standards::package` |
| Kullanıcının eklentileri | `<uygulama verisi>/kentos-cad/eklentiler` | `kentos_extension_host::places` |
| Yazı tipleri, örnek eklenti, betikler | yoktu | — |

macOS paketinde (`Contents/MacOS/kentos-cad`) standart paketlerin araması `Contents/catalog/standards`'ı buluyordu.
Kaynak klasörü (`Contents/Resources`) aranmıyordu. Paketçinin bilmesi gereken kural kodun dört yerinde gizliydi.

### Araştırma (26 Kasım 2026'da okundu)

- **macOS:** uygulama paketinin tek `Contents` klasörü vardır. Çalışan dosya `Contents/MacOS/`, kaynaklar
  `Contents/Resources/` altındadır ([Placing content in a bundle](https://developer.apple.com/documentation/bundleresources/placing-content-in-a-bundle),
  [Bundle Structures](https://developer.apple.com/library/prerelease/content/documentation/CoreFoundation/Conceptual/CFBundles/BundleTypes/BundleTypes.html)).
- **Linux:** kullanıcının verisi `$XDG_DATA_HOME` altındadır, tanımsızsa `$HOME/.local/share`
  ([XDG Base Directory Specification](https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest)).
- **Windows:** kullanıcının dolaşan verisi `%APPDATA%`'dadır. KentOS eklenti klasörünü zaten oraya koyuyor
  (ADR 8057 §2).
- **KentOS kararı:** üç yer vardır: programlar, salt okunur kaynaklar, kullanıcının klasörü. Depoda kaynakların tek
  kaynağı `paket/` klasörüdür; paketçi onu olduğu gibi kopyalar.

## Karar

### 1. Üç yer

- **`<program>`:** çalışan programın klasörü. Windows ve Linux'ta kurulum klasörü, macOS'ta `KentOS.app/Contents/MacOS`.
- **`<kaynak>`:** programın yalnız okuduğu dosyalar. Windows ve Linux'ta kurulum klasörü, macOS'ta
  `KentOS.app/Contents/Resources`.
- **`<kullanıcı>`:** kullanıcının kurduğu ve yazdığı her şey:
  - macOS: `~/Library/Application Support/kentos-cad`;
  - Windows: `%APPDATA%\kentos-cad`;
  - Linux: `$XDG_DATA_HOME/kentos-cad`, tanımsızsa `~/.local/share/kentos-cad`.

### 2. Tablo (CLAUDE.md §26'nın tablosu)

| Depoda | Kurulumda | İçerik |
|---|---|---|
| derlenen programlar | `<program>/` | `kentos-cad`, `kentos-launcher`, modül adları (`kentos-mimari`, `kentos-harita`, `kentos-gis`, `kentos-yol`), `kentos`, `kentos-mcp` |
| `paket/fonts/` | `<kaynak>/fonts/` | bütün yazı tipleri, lisansları yanlarında: `cizim/`, `arayuz/`, `yedek/` (ADR 9313). Arial'ın ve Times New Roman'ın lisanslı kopyası buraya konabilir; depoya konmaz (ADR 9311) |
| `paket/eklentiler/` | `<kaynak>/eklentiler/` | KentOS ile gelen eklentiler, her biri bir klasör; örnek Python işlem aracı `ornek-isleme/` |
| `paket/betikler/` | `<kaynak>/betikler/` | örnek Python betikleri |
| `catalog/standards/` | `<kaynak>/catalog/standards/` | standart kural paketleri (Python) |
| SDK tekerleği, uv (üretilir) | `<program>/tools/`; macOS'ta tekerlek `<kaynak>/python/`, uv `<program>/` | Python ortamı |
| — | `<kullanıcı>/eklentiler/`, `fonts/`, `betikler/`, `python/` | kullanıcının kurdukları ve yazdıkları |

### 3. Tek modül

- **`kentos_launch::install`** yerleri söyler:
  - `program_dir`, `resources`, `resources_for(os, dir)`;
  - `fonts`, `bundled_extensions`, `scripts`;
  - `user_root`, `user_root_for(os, …)`, `user_fonts`, `user_scripts`.
- **Yeni kod** kurulumda bir dosya okuyacaksa yerini buradan sorar. Yeni bir dizin önce tabloya ve bu modüle eklenir.
- **Depodan çalışan program** (`target/<profil>/`) kaynakları üst klasördeki `paket/`'ten bulur. İşareti
  `paket/README.md`'dir.
- **Ortam değişkenleri:** `KENTOS_RESOURCES_DIR` ve `KENTOS_USER_DIR` başka yeri gösterir (sınamalar, gerçek pencere
  oturumları).
- **Var olan kurallar korundu:**
  - Python ortamının `tools/` kuralı tabloya yazıldı.
  - Standart paketlerin araması önce `<kaynak>/catalog/standards`'a bakar; macOS paketinde de bulunur.
  - Kullanıcının eklenti klasörü `places::user_root`'tadır ve aynı yeri söyler.
- **Sınama:** `the_checkout_holds_the_resources_folders`, depodaki `paket/`'te tablonun klasörlerinin olduğunu sınar.

### 4. Paketçinin adımları

1. Programları `<program>`'a koy.
2. `paket/`'in içini `<kaynak>`'a kopyala.
3. `catalog/standards/`'ı `<kaynak>/catalog/standards/`'a kopyala.
4. SDK tekerleğini ve uv'yi tablonun yerine koy.

Yazı tiplerinin lisansları `paket/fonts/`'ta yüzlerin yanındadır; ikinci adım onları da kopyalar (ADR 9313: bütün
yazı tipleri o klasöre taşındı, ayrı `lisanslar/` klasörü kalktı).

Yüklü bir programda `kentos_launch::install::resources()` `<kaynak>`'ı vermelidir; paketçinin denemesi budur.

## Doğrulama (26 Kasım 2026)

- `cargo test -p kentos-launch`: 19 sınama geçti. Kurulumun kaynakları, kullanıcının klasörü (üç sistem) ve depodaki
  `paket/`'in klasörleri sınandı.
- `kentos-standards`: sınamaları geçti. Standart paketler önce `<kaynak>/catalog/standards`'ta aranır.
- `kentos kek verify paket/eklentiler/ornek-isleme`: doğrulandı.
- Paket kurulmadı. Yüklü bir programda yerlerin denenmesi paketleme işindedir (TODOS OPS-10).

## Açık kalanlar

- **Kurulum paketinin kendisi:** macOS `.app`/`.dmg`, Windows kurucusu ve Linux arşivi yapılmadı (TODOS OPS-10).
  Bu karar onun girdisidir.
- **KentOS ile gelen eklentiler:** kayıt `<kaynak>/eklentiler`'i henüz kendi kaynağı olarak açmaz. Kullanıcı paketi
  kurar (ADR 9310 §Açık kalanlar).
- **Python ortamının yerleri:** `kentos_python_env`'in iki işlevi aynı yerleri söyler ama tek modülden sormaz;
  paketleme işinde oraya bağlanır.
