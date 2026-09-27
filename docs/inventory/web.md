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
| Pencereler ve paneller | 62 | 62 | 0 | 0 |

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

| Bölüm | Masaüstünde | Kısmi | Yok | Anlamsız | Toplam |
|---|---|---|---|---|---|
| Komutlar | 138 | 0 | 29 | 0 | 167 |
| Araçlar | 56 | 0 | 2 | 0 | 58 |
| İşlem araçları | 4 | 0 | 0 | 0 | 4 |
| İşlem modelleri | 1 | 0 | 0 | 0 | 1 |
| Çalışma modları | 3 | 0 | 2 | 0 | 5 |
| Ayarlar | 27 | 0 | 38 | 0 | 65 |
| Tarayıcı depoları | 1 | 0 | 9 | 0 | 10 |
| `.kcad` alanları (v1 okunur, v2 yazılır) | 0 | 0 | 192 | 0 | 192 |
| Pencereler ve paneller | 1 | 0 | 61 | 0 | 62 |

### Masaüstünde anlamsız (0)

Yok.

### Web'de olup masaüstünde olmayanlar

Kısmi olanlar notlarıyla; bölüm bölüm.

#### Komutlar (29 / 167)

- `analysis.slope` Eğim analizi…
- `analysis.volume` Hacim hesabı…
- `cloud.history` Proje geçmişi…
- `cloud.share` Bulut projesini paylaş…
- `crs.query` Koordinat sorgula
- `crs.transform` Datum dönüşümü (ED50 ↔ TUREF)…
- `file.export.pdf` PDF pafta…
- `file.import.ncz` Netcad NCZ…
- `file.print` Yazdır ve pafta çıktısı…
- `map.contours` Eşyükselti üret…
- `map.parcelReport` Parsel alan çizelgesi
- `map.profile` Boy kesit al…
- `map.sheet` Pafta bölümlemesi…
- `processing.newModel` Yeni model…
- `style.assign` Seçili nesnelere sembol ver…
- `style.clearSymbol` Nesne sembolünü kaldır
- `style.legend` Lejant…
- `style.manager` Stil yöneticisi…
- `style.svgEditor` SVG çizim düzenleyicisi…
- `tool.stakeout` Aplikasyon
- `tool.subdivide` İfraz
- `view.keyTips` Şerit harf ipuçları
- `view.renderer.webgl2` WebGL2
- `view.renderer.webgpu` WebGPU
- `view.ribbon` Şerit arayüzü
- `view.toolbox` Araç kutusu
- `view.toolboxDock` Araç kutusunu kenara sabitle
- `workspace.disaster` Afet Analizi
- `workspace.plan3d` 3D Plan

#### Araçlar (2 / 58)

- `stakeout` Aplikasyon — Aplikasyon aracı hazır değil. Hesap menüsündeki `calc.stakeout` penceresi ayrıdır ve çalışır.
- `subdivide` İfraz — İfraz hesabı henüz yok. Alan ve hisse kuralları bağımsız referans ve kurum kabulü ister (CLAUDE.md §7, §23; TODOS.md GIS-06, GIS-13).

#### İşlem araçları (0 / 4)

Yok.

#### İşlem modelleri (0 / 1)

Yok.

#### Çalışma modları (2 / 5)

- `disaster` Afet ve risk analizi
- `plan3d` İmar planından 3D kent tasarımı

#### Ayarlar (38 / 65)

- `device.rendererPreference`
- `layout.bottomExpanded`
- `layout.bottomHeight`
- `layout.bottomTab`
- `layout.dockTab`
- `layout.dockWidth`
- `layout.layersFraction`
- `layout.processingFolded`
- `layout.processingTab`
- `layout.ribbonCollapsed`
- `layout.ribbonQuickAccess`
- `layout.ribbonSplits`
- `layout.ribbonTab`
- `layout.ribbonToolbox`
- `layout.rightVisible`
- `layout.theme`
- `layout.toolboxColumns`
- `layout.toolboxDocked`
- `layout.toolboxFolded`
- `layout.toolboxVisible`
- `layout.toolboxX`
- `layout.toolboxY`
- `project.angleUnit`
- `project.areaDecimals`
- `project.areaUnit`
- `project.drawingFont`
- `project.lengthDecimals`
- `project.plotScale`
- `project.srid`
- `project.workspace`
- `session.color`
- `session.lineType`
- `session.lineWeight`
- `user.accent`
- `user.crosshair`
- `user.shell`
- `user.uiFont`
- `user.uiScale`

#### Tarayıcı depoları (9 / 10)

- `kentos.cloud/drafts`
- `kentos.files/recent`
- `kentos.invitation`
- `kentos.prefs.v1`  — Eski tercih deposu. Tipli ayarlara (`kentos.settings.v1`) bir kez taşınır; uygulama ona bir daha yazmaz, yedek olarak olduğu gibi durur (ADR 0023).
- `kentos.processing.v1`
- `kentos.settings.v1`
- `kentos.settings.v1.backup`  — Okunamayan ya da geçersiz değer içeren ayar kaydının tam metni; kurtarma üstüne yazmadan önce buraya kopyalar (ADR 0023).
- `kentos.styles.v1`
- `kentos.ui.v1`

#### `.kcad` alanları (v1 okunur, v2 yazılır) (192 / 192)

- `AngleUnit`
- `ArcEntity.a0`
- `ArcEntity.a1`
- `ArcEntity.attrs`
- `ArcEntity.c`
- `ArcEntity.color`
- `ArcEntity.id`
- `ArcEntity.label`
- `ArcEntity.layerId`
- `ArcEntity.r`
- `ArcEntity.symbol`
- `AreaUnit`
- `Bounds.maxX`
- `Bounds.maxY`
- `Bounds.minX`
- `Bounds.minY`
- `CircleEntity.attrs`
- `CircleEntity.c`
- `CircleEntity.color`
- `CircleEntity.id`
- `CircleEntity.label`
- `CircleEntity.layerId`
- `CircleEntity.r`
- `CircleEntity.symbol`
- `ConstructionEntity.attrs`
- `ConstructionEntity.color`
- `ConstructionEntity.dir`
- `ConstructionEntity.id`
- `ConstructionEntity.label`
- `ConstructionEntity.layerId`
- `ConstructionEntity.p`
- `ConstructionEntity.symbol`
- `DimensionEntity.a`
- `DimensionEntity.angle`
- `DimensionEntity.attrs`
- `DimensionEntity.b`
- `DimensionEntity.c`
- `DimensionEntity.color`
- `DimensionEntity.height`
- `DimensionEntity.id`
- `DimensionEntity.label`
- `DimensionEntity.layerId`
- `DimensionEntity.offset`
- `DimensionEntity.style`
- `DimensionEntity.symbol`
- `DimensionEntity.text`
- `DimensionStyle`
- `DocumentSnapshotV1.activeLayer`
- `DocumentSnapshotV1.entities`
- `DocumentSnapshotV1.format`
- `DocumentSnapshotV1.homeView`
- `DocumentSnapshotV1.layers`
- `DocumentSnapshotV1.name`
- `DocumentSnapshotV1.origin`
- `DocumentSnapshotV1.settings`
- `DocumentSnapshotV1.styles`
- `DocumentSnapshotV1.version`
- `DocumentSnapshotV2.activeLayer`
- `DocumentSnapshotV2.entities`
- `DocumentSnapshotV2.format`
- `DocumentSnapshotV2.homeView`
- `DocumentSnapshotV2.layers`
- `DocumentSnapshotV2.migratedFrom`
- `DocumentSnapshotV2.name`
- `DocumentSnapshotV2.origin`
- `DocumentSnapshotV2.projectId`
- `DocumentSnapshotV2.settings`
- `DocumentSnapshotV2.styles`
- `DocumentSnapshotV2.uids`
- `DocumentSnapshotV2.version`
- `DrawingFont`
- `EllipseEntity.attrs`
- `EllipseEntity.c`
- `EllipseEntity.color`
- `EllipseEntity.id`
- `EllipseEntity.label`
- `EllipseEntity.layerId`
- `EllipseEntity.major`
- `EllipseEntity.ratio`
- `EllipseEntity.symbol`
- `EllipseEntity.t0`
- `EllipseEntity.t1`
- `Entity`
- `EntityId`
- `HatchEntity.attrs`
- `HatchEntity.color`
- `HatchEntity.holes`
- `HatchEntity.id`
- `HatchEntity.label`
- `HatchEntity.layerId`
- `HatchEntity.pattern`
- `HatchEntity.ring`
- `HatchEntity.symbol`
- `HatchPattern.angle`
- `HatchPattern.spacing`
- `HatchPattern.type`
- `HatchPatternType`
- `LabelInk`
- `LabelPlacement`
- `LabelStyle.grow`
- `LabelStyle.ink`
- `LabelStyle.maxScale`
- `LabelStyle.maxSize`
- `LabelStyle.minFeaturePx`
- `LabelStyle.minScale`
- `LabelStyle.placement`
- `LabelStyle.size`
- `LabelStyle.template`
- `LabelStyle.weight`
- `LayerNode.children`
- `LayerNode.expanded`
- `LayerNode.id`
- `LayerNode.locked`
- `LayerNode.name`
- `LayerNode.style`
- `LayerNode.type`
- `LayerNode.visible`
- `LayerNodeType`
- `LayerStyle.color`
- `LayerStyle.fill`
- `LayerStyle.label`
- `LayerStyle.lineType`
- `LayerStyle.lineWeight`
- `LayerStyle.pickInterior`
- `LayerStyle.point`
- `LayerStyle.renderer`
- `LineEntity.a`
- `LineEntity.attrs`
- `LineEntity.b`
- `LineEntity.color`
- `LineEntity.id`
- `LineEntity.label`
- `LineEntity.layerId`
- `LineEntity.symbol`
- `LineType`
- `MigrationSource.format`
- `MigrationSource.sourceSha256`
- `MigrationSource.version`
- `PathEntity.attrs`
- `PathEntity.bulges`
- `PathEntity.color`
- `PathEntity.holes`
- `PathEntity.id`
- `PathEntity.label`
- `PathEntity.layerId`
- `PathEntity.pts`
- `PathEntity.symbol`
- `PointEntity.attrs`
- `PointEntity.color`
- `PointEntity.id`
- `PointEntity.label`
- `PointEntity.layerId`
- `PointEntity.p`
- `PointEntity.symbol`
- `PointEntity.z`
- `PointStyle.size`
- `PointStyle.symbol`
- `PointSymbol`
- `ProjectId`
- `ProjectSettings.angleUnit`
- `ProjectSettings.areaDecimals`
- `ProjectSettings.areaUnit`
- `ProjectSettings.drawingFont`
- `ProjectSettings.lengthDecimals`
- `ProjectSettings.plotScale`
- `ProjectSettings.srid`
- `ProjectSettings.workspace`
- `ProjectStyles.categories`
- `ProjectStyles.items`
- `RingGeometry.bulges`
- `RingGeometry.pts`
- `SplineEntity.attrs`
- `SplineEntity.closed`
- `SplineEntity.color`
- `SplineEntity.id`
- `SplineEntity.label`
- `SplineEntity.layerId`
- `SplineEntity.pts`
- `SplineEntity.symbol`
- `TextEntity.attrs`
- `TextEntity.color`
- `TextEntity.height`
- `TextEntity.id`
- `TextEntity.label`
- `TextEntity.layerId`
- `TextEntity.p`
- `TextEntity.rotation`
- `TextEntity.symbol`
- `TextEntity.text`
- `Vec2.x`
- `Vec2.y`
- `Workspace`

#### Pencereler ve paneller (61 / 62)

- `apps/web/src/ui/appmenu/AppMenu.ts#openAppMenu` openAppMenu
- `apps/web/src/ui/bottom/BottomPanel.ts#BottomPanel` BottomPanel
- `apps/web/src/ui/bottom/CommandLine.ts#CommandLine` CommandLine
- `apps/web/src/ui/calc/IntersectionDialog.ts#openIntersection` openIntersection
- `apps/web/src/ui/calc/PolarDialog.ts#openPolar` openPolar
- `apps/web/src/ui/calc/StakeoutDialog.ts#openStakeout` openStakeout
- `apps/web/src/ui/calc/TraverseDialog.ts#openTraverse` openTraverse
- `apps/web/src/ui/cloud/AccessLostNotice.ts#openAccessLostNotice` openAccessLostNotice
- `apps/web/src/ui/cloud/CatalogDialog.ts#openCatalog` openCatalog
- `apps/web/src/ui/cloud/ConflictDialog.ts#openConflictDialog` openConflictDialog
- `apps/web/src/ui/cloud/HistoryForms.ts#openCheckpointDialog` openCheckpointDialog
- `apps/web/src/ui/cloud/HistoryForms.ts#openRestoreDialog` openRestoreDialog
- `apps/web/src/ui/cloud/InvitationDialog.ts#openInvitationDialog` openInvitationDialog
- `apps/web/src/ui/cloud/LoginDialog.ts#openLoginDialog` openLoginDialog
- `apps/web/src/ui/cloud/ProjectActions.ts#openDeleteDialog` openDeleteDialog
- `apps/web/src/ui/cloud/ProjectActions.ts#openRenameDialog` openRenameDialog
- `apps/web/src/ui/cloud/ProjectForms.ts#openConvertDialog` openConvertDialog
- `apps/web/src/ui/cloud/ProjectForms.ts#openDuplicateDialog` openDuplicateDialog
- `apps/web/src/ui/cloud/ProjectForms.ts#openMetadataDialog` openMetadataDialog
- `apps/web/src/ui/cloud/ProjectsDialog.ts#openProjectsDialog` openProjectsDialog
- `apps/web/src/ui/cloud/ShareDialog.ts#openShareDialog` openShareDialog
- `apps/web/src/ui/cloud/UploadDialog.ts#openUploadDialog` openUploadDialog
- `apps/web/src/ui/dialogs.ts#openAboutDialog` openAboutDialog
- `apps/web/src/ui/dialogs.ts#openShortcutsDialog` openShortcutsDialog
- `apps/web/src/ui/dock/RightDock.ts#RightDock` RightDock
- `apps/web/src/ui/io/CoordExportDialog.ts#openCoordExport` openCoordExport
- `apps/web/src/ui/io/CoordImportDialog.ts#openCoordImport` openCoordImport
- `apps/web/src/ui/io/DxfExportDialog.ts#openDxfExport` openDxfExport
- `apps/web/src/ui/io/DxfImportDialog.ts#openDxfImport` openDxfImport
- `apps/web/src/ui/io/GeoJsonExportDialog.ts#openGeoJsonExport` openGeoJsonExport
- `apps/web/src/ui/io/GisImportDialog.ts#openGeoJsonImport` openGeoJsonImport
- `apps/web/src/ui/io/GisImportDialog.ts#openShapefileImport` openShapefileImport
- `apps/web/src/ui/layers/LayersPanel.ts#LayersPanel` LayersPanel
- `apps/web/src/ui/menu/MenuBar.ts#MenuBar` MenuBar
- `apps/web/src/ui/processing/ProcessingPanel.ts#ProcessingPanel` ProcessingPanel
- `apps/web/src/ui/processing/ToolDialog.ts#openModelDialog` openModelDialog
- `apps/web/src/ui/processing/ToolDialog.ts#openToolDialog` openToolDialog
- `apps/web/src/ui/processing/model/ModelDesigner.ts#openModelDesigner` openModelDesigner
- `apps/web/src/ui/properties/PropertiesPanel.ts#PropertiesPanel` PropertiesPanel — Öznitelikler v1'de metindir; tipli öznitelik şeması henüz yok (CLAUDE.md §24.1, TODOS.md DOM-09..11).
- `apps/web/src/ui/ribbon/Ribbon.ts#Ribbon` Ribbon
- `apps/web/src/ui/settings/AppSettingsDialog.ts#openAppSettings` openAppSettings
- `apps/web/src/ui/settings/NewProjectDialog.ts#openNewProjectDialog` openNewProjectDialog
- `apps/web/src/ui/settings/ProjectSettingsDialog.ts#openProjectSettings` openProjectSettings
- `apps/web/src/ui/shell/AppShell.ts#AppShell` AppShell
- `apps/web/src/ui/shell/CommandBar.ts#CommandBar` CommandBar
- `apps/web/src/ui/shell/CursorInput.ts#CursorInput` CursorInput
- `apps/web/src/ui/shell/HoverCard.ts#HoverCard` HoverCard
- `apps/web/src/ui/shell/InlineTextEditor.ts#InlineTextEditor` InlineTextEditor
- `apps/web/src/ui/start/StartScreen.ts#openStartScreen` openStartScreen
- `apps/web/src/ui/statusbar/StatusBar.ts#StatusBar` StatusBar
- `apps/web/src/ui/style/LayerStyleDialog.ts#openLayerStyle` openLayerStyle
- `apps/web/src/ui/style/LegendDialog.ts#openLegend` openLegend
- `apps/web/src/ui/style/StyleManager.ts#openStyleManager` openStyleManager
- `apps/web/src/ui/style/SymbolDesigner.ts#openSymbolDesigner` openSymbolDesigner
- `apps/web/src/ui/svgedit/SvgEditor.ts#openSvgEditor` openSvgEditor
- `apps/web/src/ui/svgedit/svgDocProps.ts#openDocProps` openDocProps
- `apps/web/src/ui/svgedit/svgExport.ts#openExportDialog` openExportDialog
- `apps/web/src/ui/svgedit/svgImport.ts#openImportDialog` openImportDialog
- `apps/web/src/ui/svgedit/svgTrace.ts#openTraceDialog` openTraceDialog
- `apps/web/src/ui/toolbar/Toolbar.ts#Toolbar` Toolbar
- `apps/web/src/ui/toolbox/Toolbox.ts#Toolbox` Toolbox

## Test başvurusu

52 / 167 komutun kimliği hiçbir test dosyasında ya da e2e betiğinde geçmiyor. Kimliğin bir testte geçmesi davranışın sınandığını göstermez; kabul kanıtı değildir.
