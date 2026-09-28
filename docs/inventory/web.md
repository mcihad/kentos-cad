# Web özellik envanteri: özet

Üretilmiş dosyadır, elle düzenlenmez. Yöntem ve alanlar: [README.md](README.md). Tam veri: [web.json](web.json).

| Bölüm | Toplam | implemented | partial | pending |
|---|---|---|---|---|
| Komutlar | 167 | 152 | 0 | 15 |
| Araçlar | 58 | 56 | 0 | 2 |
| İşlem araçları | 4 | 4 | 0 | 0 |
| İşlem modelleri | 1 | 1 | 0 | 0 |
| Çalışma modları | 5 | 3 | 0 | 2 |
| Ayarlar | 65 | 65 | 0 | 0 |
| Tarayıcı depoları | 10 | 10 | 0 | 0 |
| `.kcad` alanları (v1 okunur, v2 yazılır) | 192 | 192 | 0 | 0 |
| Pencereler ve paneller | 63 | 63 | 0 | 0 |

## Kısmi (0)

Yok.

## Bekleyen (19)

- Komutlar: `analysis.slope` Eğim analizi…
- Komutlar: `analysis.volume` Hacim hesabı…
- Komutlar: `crs.query` Koordinat sorgula
- Komutlar: `crs.transform` Datum dönüşümü (ED50 ↔ TUREF)…
- Komutlar: `file.export.pdf` PDF pafta…
- Komutlar: `file.import.ncz` Netcad NCZ…
- Komutlar: `file.print` Yazdır ve pafta çıktısı…
- Komutlar: `map.contours` Eşyükselti üret…
- Komutlar: `map.parcelReport` Parsel alan çizelgesi
- Komutlar: `map.profile` Boy kesit al…
- Komutlar: `map.sheet` Pafta bölümlemesi…
- Komutlar: `tool.stakeout` Aplikasyon
- Komutlar: `tool.subdivide` İfraz
- Komutlar: `workspace.disaster` Afet Analizi — Yakında
- Komutlar: `workspace.plan3d` 3D Plan — Yakında
- Araçlar: `stakeout` Aplikasyon — Aplikasyon aracı hazır değil. Hesap menüsündeki `calc.stakeout` penceresi ayrıdır ve çalışır.
- Araçlar: `subdivide` İfraz — İfraz hesabı henüz yok. Alan ve hisse kuralları bağımsız referans ve kurum kabulü ister (CLAUDE.md §7, §23; TODOS.md GIS-06, GIS-13).
- Çalışma modları: `disaster` Afet ve risk analizi
- Çalışma modları: `plan3d` İmar planından 3D kent tasarımı

## Arayüzde yeri görünmeyen komutlar (0)

Menüde, şeritte ve araç kutusunda yoklar; kimlikleri `src/ui` altındaki hiçbir dosyada geçmiyor. Kısayolla, komut satırından ya da başka bir yoldan çalışıyor olabilirler. Her biri fareyle bulunabilirlik açısından gözden geçirilir.

Yok.

## Masaüstü

Masaüstü sütunu şuralardan gelir, her biri öncekinin üstüne: `apps/desktop/equivalents.json`'ın bütün bir bölüm için dediği; masaüstü kabuğunun çalıştırdığı komutlar (`apps/desktop/ported.json`) ve onlarla araçları (`tool.<kimlik>`), işlem araçları ve modelleri (`processing.run.…`, `processing.model.…`), çalışma modları (`workspace.<kimlik>`); şeması masaüstünü de barındıran tipli ayarlar; tablonun öğe öğe dediği (masaüstündeki yeri ya da orada neden anlamsız olduğu); en son `annotations.json`. Bilinmeyen `none`dır. Masaüstünde komutu olmayanlar şeritte soluk durur ve “masaüstüne henüz taşınmadı” der (docs/adr/0017).

| Bölüm | Masaüstünde | Kısmi | Yok | Bekliyor | Anlamsız | Toplam |
|---|---|---|---|---|---|---|
| Komutlar | 147 | 0 | 0 | 15 | 5 | 167 |
| Araçlar | 56 | 0 | 0 | 2 | 0 | 58 |
| İşlem araçları | 4 | 0 | 0 | 0 | 0 | 4 |
| İşlem modelleri | 1 | 0 | 0 | 0 | 0 | 1 |
| Çalışma modları | 3 | 0 | 0 | 2 | 0 | 5 |
| Ayarlar | 56 | 0 | 0 | 0 | 9 | 65 |
| Tarayıcı depoları | 8 | 0 | 0 | 0 | 2 | 10 |
| `.kcad` alanları (v1 okunur, v2 yazılır) | 192 | 0 | 0 | 0 | 0 | 192 |
| Pencereler ve paneller | 57 | 2 | 0 | 0 | 4 | 63 |

Bekliyor: web'de de yapılmamış (`pending`); masaüstü onları web'in notuyla soluk gösterir, web gibi.

- `.kcad` alanları (v1 okunur, v2 yazılır), bütünüyle: implemented — Masaüstü .kcad'i web'le aynı Rust kodeğiyle (crates/shared/kcad) okur ve yazar: v2 yazılır, v1 okunur; belge (kentos-domain) göçü web'in örnek dosyasıyla aynı çıkarır (crates/native/domain/tests/snapshot_v2.rs, ADR 0025).

### Masaüstünde anlamsız (20)

- Komutlar: `view.renderer.webgl2` WebGL2 — Masaüstü çizimi wgpu ile yapar; arka ucu (Vulkan, Metal, DirectX 12, OpenGL) wgpu seçer (ADR 0019). WebGL2 ile WebGPU tarayıcının seçenekleridir.
- Komutlar: `view.renderer.webgpu` WebGPU — Masaüstü çizimi wgpu ile yapar; arka ucu (Vulkan, Metal, DirectX 12, OpenGL) wgpu seçer (ADR 0019). WebGL2 ile WebGPU tarayıcının seçenekleridir.
- Komutlar: `view.ribbon` Şerit arayüzü — Sahibin kararı (27 Eylül): masaüstünde yalnız şerit arayüzü vardır; klasik arayüz (menü çubuğu, araç çubuğu, kayan araç kutusu) yalnız web'dedir. Şerit her aracı ve komutu taşır.
- Komutlar: `view.toolbox` Araç kutusu — Sahibin kararı (27 Eylül): masaüstünde yalnız şerit arayüzü vardır; klasik arayüz (menü çubuğu, araç çubuğu, kayan araç kutusu) yalnız web'dedir. Şerit her aracı ve komutu taşır.
- Komutlar: `view.toolboxDock` Araç kutusunu kenara sabitle — Sahibin kararı (27 Eylül): masaüstünde yalnız şerit arayüzü vardır; klasik arayüz (menü çubuğu, araç çubuğu, kayan araç kutusu) yalnız web'dedir. Şerit her aracı ve komutu taşır.
- Ayarlar: `device.rendererPreference`  — WebGL2 ya da WebGPU seçimi tarayıcınındır; masaüstünün arka ucunu wgpu seçer (ADR 0019).
- Ayarlar: `layout.ribbonToolbox`  — Sahibin kararı (27 Eylül): masaüstünde yalnız şerit arayüzü vardır; klasik arayüz (menü çubuğu, araç çubuğu, kayan araç kutusu) yalnız web'dedir. Şerit her aracı ve komutu taşır.
- Ayarlar: `layout.toolboxColumns`  — Sahibin kararı (27 Eylül): masaüstünde yalnız şerit arayüzü vardır; klasik arayüz (menü çubuğu, araç çubuğu, kayan araç kutusu) yalnız web'dedir. Şerit her aracı ve komutu taşır.
- Ayarlar: `layout.toolboxDocked`  — Sahibin kararı (27 Eylül): masaüstünde yalnız şerit arayüzü vardır; klasik arayüz (menü çubuğu, araç çubuğu, kayan araç kutusu) yalnız web'dedir. Şerit her aracı ve komutu taşır.
- Ayarlar: `layout.toolboxFolded`  — Sahibin kararı (27 Eylül): masaüstünde yalnız şerit arayüzü vardır; klasik arayüz (menü çubuğu, araç çubuğu, kayan araç kutusu) yalnız web'dedir. Şerit her aracı ve komutu taşır.
- Ayarlar: `layout.toolboxVisible`  — Sahibin kararı (27 Eylül): masaüstünde yalnız şerit arayüzü vardır; klasik arayüz (menü çubuğu, araç çubuğu, kayan araç kutusu) yalnız web'dedir. Şerit her aracı ve komutu taşır.
- Ayarlar: `layout.toolboxX`  — Sahibin kararı (27 Eylül): masaüstünde yalnız şerit arayüzü vardır; klasik arayüz (menü çubuğu, araç çubuğu, kayan araç kutusu) yalnız web'dedir. Şerit her aracı ve komutu taşır.
- Ayarlar: `layout.toolboxY`  — Sahibin kararı (27 Eylül): masaüstünde yalnız şerit arayüzü vardır; klasik arayüz (menü çubuğu, araç çubuğu, kayan araç kutusu) yalnız web'dedir. Şerit her aracı ve komutu taşır.
- Ayarlar: `user.shell`  — Sahibin kararı (27 Eylül): masaüstünde yalnız şerit arayüzü vardır; klasik arayüz (menü çubuğu, araç çubuğu, kayan araç kutusu) yalnız web'dedir. Şerit her aracı ve komutu taşır.
- Tarayıcı depoları: `kentos.invitation`  — Davet bağlantısı web uygulamasının adresidir; belirteci yalnız o tarayıcı sekmesinde tutulur (ADR 0042). Masaüstü davet eder, bağlantıyı açmaz; kabul edilen proje masaüstünün kataloğunda da görünür.
- Tarayıcı depoları: `kentos.prefs.v1`  — Web'in tipli ayarlardan önceki deposudur; masaüstünün böyle eski bir deposu olmadı, ayarları baştan ayarlar.json'dadır.
- Pencereler ve paneller: `apps/web/src/ui/cloud/InvitationDialog.ts#openInvitationDialog` openInvitationDialog — Davet bağlantısı web uygulamasının adresidir: açılır ve orada kabul edilir; kabul edilen proje masaüstünün kataloğunda da görünür. Masaüstü davet eder, bağlantı açmaz.
- Pencereler ve paneller: `apps/web/src/ui/menu/MenuBar.ts#MenuBar` MenuBar — Sahibin kararı (27 Eylül): masaüstünde yalnız şerit arayüzü vardır; klasik arayüz (menü çubuğu, araç çubuğu, kayan araç kutusu) yalnız web'dedir. Şerit her aracı ve komutu taşır.
- Pencereler ve paneller: `apps/web/src/ui/toolbar/Toolbar.ts#Toolbar` Toolbar — Sahibin kararı (27 Eylül): masaüstünde yalnız şerit arayüzü vardır; klasik arayüz (menü çubuğu, araç çubuğu, kayan araç kutusu) yalnız web'dedir. Şerit her aracı ve komutu taşır.
- Pencereler ve paneller: `apps/web/src/ui/toolbox/Toolbox.ts#Toolbox` Toolbox — Sahibin kararı (27 Eylül): masaüstünde yalnız şerit arayüzü vardır; klasik arayüz (menü çubuğu, araç çubuğu, kayan araç kutusu) yalnız web'dedir. Şerit her aracı ve komutu taşır.

### Web'de olup masaüstünde olmayanlar

Kısmi olanlar notlarıyla; bölüm bölüm.

#### Komutlar (0 / 167; ayrıca 15 iki platformda da bekliyor)

- `analysis.slope` Eğim analizi… (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/catalog.rs (Standing::Pending: web'in notuyla soluk))
- `analysis.volume` Hacim hesabı… (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/catalog.rs (Standing::Pending: web'in notuyla soluk))
- `crs.query` Koordinat sorgula (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/catalog.rs (Standing::Pending: web'in notuyla soluk))
- `crs.transform` Datum dönüşümü (ED50 ↔ TUREF)… (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/catalog.rs (Standing::Pending: web'in notuyla soluk))
- `file.export.pdf` PDF pafta… (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/catalog.rs (Standing::Pending: web'in notuyla soluk))
- `file.import.ncz` Netcad NCZ… (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/catalog.rs (Standing::Pending: web'in notuyla soluk))
- `file.print` Yazdır ve pafta çıktısı… (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/catalog.rs (Standing::Pending: web'in notuyla soluk))
- `map.contours` Eşyükselti üret… (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/catalog.rs (Standing::Pending: web'in notuyla soluk))
- `map.parcelReport` Parsel alan çizelgesi (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/catalog.rs (Standing::Pending: web'in notuyla soluk))
- `map.profile` Boy kesit al… (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/catalog.rs (Standing::Pending: web'in notuyla soluk))
- `map.sheet` Pafta bölümlemesi… (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/catalog.rs (Standing::Pending: web'in notuyla soluk))
- `tool.stakeout` Aplikasyon (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/catalog.rs (Standing::Pending: web'in notuyla soluk))
- `tool.subdivide` İfraz (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/catalog.rs (Standing::Pending: web'in notuyla soluk))
- `workspace.disaster` Afet Analizi (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/modes.rs (menüde “Yakında”))
- `workspace.plan3d` 3D Plan (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/modes.rs (menüde “Yakında”))

#### Araçlar (0 / 58; ayrıca 2 iki platformda da bekliyor)

- `stakeout` Aplikasyon (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/catalog.rs (Standing::Pending: web'in notuyla soluk)) — Aplikasyon aracı hazır değil. Hesap menüsündeki `calc.stakeout` penceresi ayrıdır ve çalışır.
- `subdivide` İfraz (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/catalog.rs (Standing::Pending: web'in notuyla soluk)) — İfraz hesabı henüz yok. Alan ve hisse kuralları bağımsız referans ve kurum kabulü ister (CLAUDE.md §7, §23; TODOS.md GIS-06, GIS-13).

#### İşlem araçları (0 / 4)

Yok.

#### İşlem modelleri (0 / 1)

Yok.

#### Çalışma modları (0 / 5; ayrıca 2 iki platformda da bekliyor)

- `disaster` Afet ve risk analizi (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/modes.rs (menüde “Yakında”))
- `plan3d` İmar planından 3D kent tasarımı (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/modes.rs (menüde “Yakında”))

#### Ayarlar (0 / 65)

Yok.

#### Tarayıcı depoları (0 / 10)

Yok.

#### `.kcad` alanları (v1 okunur, v2 yazılır) (0 / 192)

Yok.

#### Pencereler ve paneller (2 / 63)

- `apps/web/src/ui/svgedit/svgExport.ts#openExportDialog` openExportDialog (kısmi) (masaüstünde: apps/desktop/src/style/svgedit/files/export.rs (ADR 0095)) — PNG panoya kopyalanamaz: masaüstünün panosu yalnız metin tutar (SVG kopyalanır). PNG dosyaya yazılır.
- `apps/web/src/ui/svgedit/svgImport.ts#openImportDialog` openImportDialog (kısmi) (masaüstünde: apps/desktop/src/style/svgedit/files/import.rs, read.rs (ADR 0095)) — Katı XML olarak okunamayan ve onarılamayan dosya ayrıştırıcının nedeniyle (satır, sütun) reddedilir; web'in son çaresi tarayıcının hoşgörülü HTML ayrıştırıcısıdır.

## Test başvurusu

52 / 167 komutun kimliği hiçbir test dosyasında ya da e2e betiğinde geçmiyor. Kimliğin bir testte geçmesi davranışın sınandığını göstermez; kabul kanıtı değildir.
