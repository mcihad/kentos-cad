# Web özellik envanteri: özet

Üretilmiş dosyadır, elle düzenlenmez. Yöntem ve alanlar: [README.md](README.md). Tam veri: [web.json](web.json).

| Bölüm | Toplam | implemented | partial | pending |
|---|---|---|---|---|
| Komutlar | 305 | 293 | 0 | 12 |
| Araçlar | 95 | 93 | 0 | 2 |
| İşlem araçları | 4 | 4 | 0 | 0 |
| İşlem modelleri | 1 | 1 | 0 | 0 |
| Proje türleri | 4 | 2 | 0 | 2 |
| Ayarlar | 81 | 81 | 0 | 0 |
| Tarayıcı depoları | 11 | 11 | 0 | 0 |
| `.kcad` alanları (v1 okunur, v2 yazılır) | 335 | 335 | 0 | 0 |
| Pencereler ve paneller | 96 | 96 | 0 | 0 |

## Kısmi (0)

Yok.

## Bekleyen (16)

- Komutlar: `analysis.slope` Eğim analizi…
- Komutlar: `analysis.volume` Hacim hesabı…
- Komutlar: `file.export.pdf` PDF pafta…
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
- Proje türleri: `disaster` Afet ve risk analizi
- Proje türleri: `plan3d` İmar planından 3D kent tasarımı

## Arayüzde yeri görünmeyen komutlar (27)

Menüde ve şeritte yoklar; kimlikleri `src/ui` altındaki hiçbir dosyada geçmiyor. Kısayolla, komut satırından ya da başka bir yoldan çalışıyor olabilirler. Her biri fareyle bulunabilirlik açısından gözden geçirilir.

`sheet.align.bottom`, `sheet.align.center`, `sheet.align.left`, `sheet.align.middle`, `sheet.align.right`, `sheet.align.top`, `sheet.alignTo.margins`, `sheet.alignTo.page`, `sheet.alignTo.selection`, `sheet.distribute.hCenters`, `sheet.distribute.hGaps`, `sheet.distribute.vCenters`, `sheet.distribute.vGaps`, `sheet.escape`, `sheet.export.kpafta`, `sheet.export.pdf`, `sheet.export.png`, `sheet.export.svg`, `sheet.matchSize.height`, `sheet.matchSize.width`, `sheet.model`, `sheet.nudge`, `sheet.print`, `sheet.redo`, `sheet.tool.hand`, `sheet.tool.select`, `sheet.undo`

## Masaüstü

Masaüstü sütunu şuralardan gelir, her biri öncekinin üstüne: `apps/desktop/equivalents.json`'ın bütün bir bölüm için dediği; masaüstü kabuğunun çalıştırdığı komutlar (`apps/desktop/ported.json`) ve onlarla araçları (`tool.<kimlik>`), işlem araçları ve modelleri (`processing.run.…`, `processing.model.…`), proje türleri (`workspace.<kimlik>`); şeması masaüstünü de barındıran tipli ayarlar; tablonun öğe öğe dediği (masaüstündeki yeri ya da orada neden anlamsız olduğu); en son `annotations.json`. Bilinmeyen `none`dır. Masaüstünde komutu olmayanlar şeritte soluk durur ve “masaüstüne henüz taşınmadı” der (docs/adr/0017).

| Bölüm | Masaüstünde | Kısmi | Yok | Bekliyor | Anlamsız | Toplam |
|---|---|---|---|---|---|---|
| Komutlar | 233 | 0 | 58 | 12 | 2 | 305 |
| Araçlar | 93 | 0 | 0 | 2 | 0 | 95 |
| İşlem araçları | 4 | 0 | 0 | 0 | 0 | 4 |
| İşlem modelleri | 1 | 0 | 0 | 0 | 0 | 1 |
| Proje türleri | 2 | 0 | 0 | 2 | 0 | 4 |
| Ayarlar | 78 | 0 | 2 | 0 | 1 | 81 |
| Tarayıcı depoları | 9 | 0 | 0 | 0 | 2 | 11 |
| `.kcad` alanları (v1 okunur, v2 yazılır) | 335 | 0 | 0 | 0 | 0 | 335 |
| Pencereler ve paneller | 78 | 2 | 15 | 0 | 1 | 96 |

Bekliyor: web'de de yapılmamış (`pending`); masaüstü onları web'in notuyla soluk gösterir, web gibi.

- `.kcad` alanları (v1 okunur, v2 yazılır), bütünüyle: implemented — Masaüstü .kcad'i web'le aynı Rust kodeğiyle (crates/shared/kcad) okur ve yazar: v2 yazılır, v1 okunur; belge (kentos-domain) göçü web'in örnek dosyasıyla aynı çıkarır (crates/native/domain/tests/all/snapshot_v2.rs, ADR 0025).

### Masaüstünde anlamsız (6)

- Komutlar: `view.renderer.webgl2` WebGL2 — Masaüstü çizimi wgpu ile yapar; arka ucu (Vulkan, Metal, DirectX 12, OpenGL) wgpu seçer (ADR 0019). WebGL2 ile WebGPU tarayıcının seçenekleridir.
- Komutlar: `view.renderer.webgpu` WebGPU — Masaüstü çizimi wgpu ile yapar; arka ucu (Vulkan, Metal, DirectX 12, OpenGL) wgpu seçer (ADR 0019). WebGL2 ile WebGPU tarayıcının seçenekleridir.
- Ayarlar: `device.rendererPreference`  — WebGL2 ya da WebGPU seçimi tarayıcınındır; masaüstünün arka ucunu wgpu seçer (ADR 0019).
- Tarayıcı depoları: `kentos.invitation`  — Davet bağlantısı web uygulamasının adresidir; belirteci yalnız o tarayıcı sekmesinde tutulur (ADR 0042). Masaüstü davet eder, bağlantıyı açmaz; kabul edilen proje masaüstünün kataloğunda da görünür.
- Tarayıcı depoları: `kentos.prefs.v1`  — Web'in tipli ayarlardan önceki deposudur; masaüstünün böyle eski bir deposu olmadı, ayarları baştan ayarlar.json'dadır.
- Pencereler ve paneller: `apps/web/src/ui/cloud/InvitationDialog.ts#openInvitationDialog` openInvitationDialog — Davet bağlantısı web uygulamasının adresidir: açılır ve orada kabul edilir; kabul edilen proje masaüstünün kataloğunda da görünür. Masaüstü davet eder, bağlantı açmaz.

### Web'de olup masaüstünde olmayanlar

Kısmi olanlar notlarıyla; bölüm bölüm.

#### Komutlar (58 / 305; ayrıca 12 iki platformda da bekliyor)

- `sheet.align.bottom` Alta hizala
- `sheet.align.center` Yatayda ortala
- `sheet.align.left` Sola hizala
- `sheet.align.middle` Düşeyde ortala
- `sheet.align.right` Sağa hizala
- `sheet.align.top` Üste hizala
- `sheet.alignTo.margins` Kenar boşluklarına göre
- `sheet.alignTo.page` Sayfaya göre
- `sheet.alignTo.selection` Seçime göre
- `sheet.atlas` Atlas
- `sheet.delete` Paftayı sil…
- `sheet.deleteItems` Sil
- `sheet.distribute.hCenters` Yatayda ortaları eşit dağıt
- `sheet.distribute.hGaps` Yatayda aralıkları eşitle
- `sheet.distribute.vCenters` Düşeyde ortaları eşit dağıt
- `sheet.distribute.vGaps` Düşeyde aralıkları eşitle
- `sheet.duplicate` Paftayı çoğalt
- `sheet.duplicateItems` Çoğalt
- `sheet.escape` Seçimi bırak
- `sheet.export.kpafta` .kpafta dosyası olarak…
- `sheet.export.pdf` PDF olarak…
- `sheet.export.png` PNG olarak…
- `sheet.export.svg` SVG olarak…
- `sheet.fromTemplate` Şablondan pafta…
- `sheet.grid` Karelaj
- `sheet.group` Grupla
- `sheet.hideItems` Gizle ya da göster
- `sheet.importKpafta` .kpafta dosyasından…
- `sheet.lockItems` Kilitle ya da aç
- `sheet.matchSize.height` Aynı yükseklik
- `sheet.matchSize.width` Aynı genişlik
- `sheet.moveLeft` Sola taşı
- `sheet.moveRight` Sağa taşı
- `sheet.new` Yeni pafta
- `sheet.nudge` Kaydır
- `sheet.open` Paftayı aç
- `sheet.order.back` Arkaya gönder
- `sheet.order.backward` Bir arkaya
- `sheet.order.forward` Bir öne
- `sheet.order.front` Öne getir
- `sheet.pageSetup` Sayfa ayarları…
- `sheet.preflight` Ön denetim
- `sheet.print` Yazdır…
- `sheet.redo` Yinele (pafta)
- `sheet.rename` Paftaya ad ver…
- `sheet.renameItem` Öğeye ad ver
- `sheet.saveTemplate` Şablon olarak kaydet
- `sheet.selectAll` Tümünü seç
- `sheet.tool.hand` El
- `sheet.tool.select` Seç
- `sheet.undo` Geri al (pafta)
- `sheet.ungroup` Grubu çöz
- `sheet.variables` Değişkenler…
- `sheet.zoomIn` Yakınlaştır
- `sheet.zoomOut` Uzaklaştır
- `sheet.zoomPage` Sayfayı sığdır
- `sheet.zoomReal` Gerçek boyut
- `sheet.zoomSelection` Seçime yakınlaş
- `analysis.slope` Eğim analizi… (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/catalog.rs (Standing::Pending: web'in notuyla soluk))
- `analysis.volume` Hacim hesabı… (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/catalog.rs (Standing::Pending: web'in notuyla soluk))
- `file.export.pdf` PDF pafta… (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/catalog.rs (Standing::Pending: web'in notuyla soluk))
- `file.print` Yazdır ve pafta çıktısı… (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/catalog.rs (Standing::Pending: web'in notuyla soluk))
- `map.contours` Eşyükselti üret… (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/catalog.rs (Standing::Pending: web'in notuyla soluk))
- `map.parcelReport` Parsel alan çizelgesi (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/catalog.rs (Standing::Pending: web'in notuyla soluk))
- `map.profile` Boy kesit al… (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/catalog.rs (Standing::Pending: web'in notuyla soluk))
- `map.sheet` Pafta bölümlemesi… (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/catalog.rs (Standing::Pending: web'in notuyla soluk))
- `tool.stakeout` Aplikasyon (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/catalog.rs (Standing::Pending: web'in notuyla soluk))
- `tool.subdivide` İfraz (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/catalog.rs (Standing::Pending: web'in notuyla soluk))
- `workspace.disaster` Afet Analizi (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/modes.rs (menüde “Yakında”))
- `workspace.plan3d` 3D Plan (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/modes.rs (menüde “Yakında”))

#### Araçlar (0 / 95; ayrıca 2 iki platformda da bekliyor)

- `stakeout` Aplikasyon (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/catalog.rs (Standing::Pending: web'in notuyla soluk)) — Aplikasyon aracı hazır değil. Hesap menüsündeki `calc.stakeout` penceresi ayrıdır ve çalışır.
- `subdivide` İfraz (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/catalog.rs (Standing::Pending: web'in notuyla soluk)) — İfraz hesabı henüz yok. Alan ve hisse kuralları bağımsız referans ve kurum kabulü ister (CLAUDE.md §7, §23; TODOS.md GIS-06, GIS-13).

#### İşlem araçları (0 / 4)

Yok.

#### İşlem modelleri (0 / 1)

Yok.

#### Proje türleri (0 / 4; ayrıca 2 iki platformda da bekliyor)

- `disaster` Afet ve risk analizi (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/modes.rs (menüde “Yakında”))
- `plan3d` İmar planından 3D kent tasarımı (iki platformda da bekliyor) (masaüstünde: apps/desktop/src/modes.rs (menüde “Yakında”))

#### Ayarlar (2 / 81)

- `session.overlapLast`
- `session.overlapLayers`

#### Tarayıcı depoları (0 / 11)

Yok.

#### `.kcad` alanları (v1 okunur, v2 yazılır) (0 / 335)

Yok.

#### Pencereler ve paneller (17 / 96)

- `apps/web/src/ui/settings/ProjectTypeDialog.ts#openProjectTypeDialog` openProjectTypeDialog
- `apps/web/src/ui/sheet/ExportDialog.ts#openExportDialog` openExportDialog
- `apps/web/src/ui/sheet/ExpressionDialog.ts#openExpressionDialog` openExpressionDialog
- `apps/web/src/ui/sheet/ItemTree.ts#ItemTree` ItemTree
- `apps/web/src/ui/sheet/PageSetupDialog.ts#openPageSetup` openPageSetup
- `apps/web/src/ui/sheet/PublishTemplateDialog.ts#openPublishDialog` openPublishDialog
- `apps/web/src/ui/sheet/SaveTemplateDialog.ts#openSaveTemplate` openSaveTemplate
- `apps/web/src/ui/sheet/ShareTemplateDialog.ts#openShareTemplateDialog` openShareTemplateDialog
- `apps/web/src/ui/sheet/SheetInspector.ts#SheetInspector` SheetInspector
- `apps/web/src/ui/sheet/SheetStage.ts#SheetStage` SheetStage
- `apps/web/src/ui/sheet/SheetTabs.ts#SheetTabs` SheetTabs
- `apps/web/src/ui/sheet/SheetWorkspace.ts#SheetWorkspace` SheetWorkspace
- `apps/web/src/ui/sheet/SheetsPanel.ts#SheetsPanel` SheetsPanel
- `apps/web/src/ui/sheet/TemplateGallery.ts#openTemplateGallery` openTemplateGallery
- `apps/web/src/ui/sheet/VariablesDialog.ts#openVariables` openVariables
- `apps/web/src/ui/svgedit/svgExport.ts#openExportDialog` openExportDialog (kısmi) (masaüstünde: apps/desktop/src/style/svgedit/files/export.rs (ADR 0095)) — PNG panoya kopyalanamaz: masaüstünün panosu yalnız metin tutar (SVG kopyalanır). PNG dosyaya yazılır.
- `apps/web/src/ui/svgedit/svgImport.ts#openImportDialog` openImportDialog (kısmi) (masaüstünde: apps/desktop/src/style/svgedit/files/import.rs, read.rs (ADR 0095)) — Katı XML olarak okunamayan ve onarılamayan dosya ayrıştırıcının nedeniyle (satır, sütun) reddedilir; web'in son çaresi tarayıcının hoşgörülü HTML ayrıştırıcısıdır.

## Test başvurusu

111 / 305 komutun kimliği hiçbir test dosyasında ya da e2e betiğinde geçmiyor. Kimliğin bir testte geçmesi davranışın sınandığını göstermez; kabul kanıtı değildir.
