# ADR 9304: Kullanıcının okuduğu metinde başka uygulama adı ve ADR atfı yok; bekçi sınaması

- **Durum:** kabul edildi (2026-11-24).
- **Kural:** CLAUDE.md, “Başka uygulamaya atıf yok” (10 Kasım 2026). todos-new.md A6 ve A7.
- **Web:** iptal edildi; yalnız donmuş envanter metinleri düzeltildi. **`.kcad`:** değişmedi. **Komut:** yok (metin).

## Bağlam

Araç ipuçlarında, Komut ara'da, `help()`'te ve hata açıklamalarında Netcad, AutoCAD, Civil 3D, ArcGIS, QGIS ve NETPRO
adları geçiyordu; iki katalog açıklaması ADR dosyasına atıf yapıyordu. Metinlerin kaynağı üç yerdeydi: donmuş envanter
(`docs/inventory/web.json`) ve `catalog/v1/additions.json`, Rust kaynaklarının metin sabitleri ve sözleşmelerin belge
yorumları (bunlardan Python SDK ve katalog şemaları üretilir).

## Karar

1. **Yeniden yazıldı.** Katalog metinleri (`file.import.ncz` adı artık “NCZ…”; `tool.arc`, `tool.coordinateLabel`,
   `tool.centerline`, `tool.edgeShift`, `tool.stationLabels`, işlem açıklamaları, `view.lineWeights`,
   `pointcloud.vpcSave`, `file.import.ncn`, `file.import.dxf`; `NETCAD` arama takma adı kaldırıldı). Kübaj kuralının
   yardımı ve katalog özeti, hata kodu, birim ve form şeması açıklamaları. `tool.hatch`, `tool.railing` ve planı
   tanıma komutunun açıklamalarındaki ADR atfı çıkarıldı. Sözleşme ve komut belge yorumlarındaki adlar çıkarıldı;
   `KENTOS_WRITE_CATALOG=1 cargo test -p kentos-commands catalog`, sözleşmelerin ts-rs dışa aktarımı,
   `python3 scripts/python/sdk.py` ve `python3 scripts/cad/manifest.py` yeniden koşturuldu.
2. **ADR atfı yalnız kullanıcının okuduğu metinde yasak:** katalog başlık ve açıklaması, Python belge dizgileri, masaüstü
   metin sabitleri (test kodu dışında). Test iletilerinde, geliştirici notlarında ve yorumlarda kalır; uygulama adı ise
   her yerde yasaktır.
3. **Bekçi:** `apps/desktop/src/tests/other_names_tests.rs`. Dört sınama: katalog metinleri (ad ve ADR), masaüstü
   uygulamalarının, `kentos-cad`, komut kataloğunun ve sözleşmelerin Rust metin sabitleri (ad her yerde; ADR yalnız test
   dışında; yorumlar taranmaz), `python/kentos` (yalnız ad: Python belge dizgilerindeki ~4400 ADR atfı sözleşme
   yorumlarından gelir, ayrı iştir). İzinli adlar `ALLOWED` tablosundadır, her biri gerekçesiyle.

## F4: bırakılan adlar (sahibin kararı beklenir)

| Yer | Ad | Neden bırakıldı |
|---|---|---|
| `crates/shared/contracts/src/gis/service.rs` (`ServiceKind::Arcgis`, `FeedKind::Arcgis`, `AuthKind::Arcgis`, satır 97, 278, 367, 584, 799) | ArcGIS REST, ArcGIS belirteci | servis ve kimlik doğrulama türü, protokolün adı |
| `crates/shared/commands/src/catalog/annotation.rs:1490` | ArcGIS REST | aynı servis türü |
| `apps/desktop/src/shell/services/feed_window.rs:49, 933`, `services/window.rs:68`, `services/info.rs:57, 191` | ArcGIS REST, `ArcGIS where` | servis türü adı; sorgu söz dizimi ipucu |
| `crates/shared/contracts/src/core/shape_resource.rs:79–94, 153, 184–185` | `AutoCAD-86 …` | SHX dosyasının ilk baytları (tanıma imzası) |
| `crates/shared/geometry-core/src/text/shx.rs:63–66, 162, 788, 907–909`, `crates/native/dwg/src/lib.rs:86`, `crates/shared/formats/src/dxf/{mod.rs:803,preflight.rs:48}` (`AutoCAD Binary DXF`) | imza baytları | biçimin kendi imzası |
| `docs/inventory/web.json` `file.export.dxf` açıklaması | “AutoCAD 2007 DXF'i” | DXF sürüm adı; “DXF (2007 sürümü)” yazılabilir |
| `crates/shared/contracts/src/core/settings/schema.rs:162` | ayar anahtarı `drafting.autocadAliases` | saklanan anahtar; yeniden adlandırma taşıma ister |
| `python/kentos/services.py`, `cad/layers.py`, `cad/types.py` | ArcGIS | yukarıdaki servis türünden üretilen belgeler |

Bu tarama kapsamı dışında kalanlar: `crates/shared/services/**` ve `crates/sheet-ui/src/export_pdf.rs:32` ileti metinleri,
`apps/ui-showcase/**` galeri metinleri, `catalog/v1/names-mimari.json` ve `style-exchange.json` notları, `uzlas-qgis.gpkg`
örnek dosya adları.

## Doğrulama

- `cargo test -p kentos-desktop --lib other_names_tests`: 4 sınama geçti (bekçi önce gerçek ihlalleri yakaladı).
- `cargo test -p kentos-commands catalog` ve sözleşme dışa aktarımı yeniden üretti; `scripts/python/sdk.py --check` ve
  `scripts/cad/manifest.py --check` temiz; `module_catalog` ve `catalog` sınamaları geçti.
