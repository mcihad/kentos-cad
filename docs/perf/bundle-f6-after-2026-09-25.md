# Build envanteri: f6-after (2026-09-25, 55a0fff, çalışma ağacı temiz değil)

Ölçüm: `node scripts/perf/bundle.mjs --label f6-after`. Boyutlar ham / gzip (9) / brotli (11).

| Kapsam | Ham | gzip | brotli |
|---|---|---|---|
| İlk sayfa JS | 538.9 KB | 170.4 KB | 143.8 KB |
| İlk sayfa CSS | 114.4 KB | 19.2 KB | 16.8 KB |
| Bütün dist (97 dosya) | 5174.0 KB | 2562.9 KB | 2293.9 KB |

## Giriş chunk'ının en büyük modülleri (işlenmiş boyut)

| Modül | Boyut |
|---|---|
| `src/tools/catalog.ts` | 29.7 KB |
| `src/viewport/ViewportController.ts` | 21.9 KB |
| `src/app/commands.ts` | 18.7 KB |
| `src/render/webgl2/styledShaders.ts` | 16.1 KB |
| `src/tools/areaTools.ts` | 15.3 KB |
| `src/ui/properties/PropertiesPanel.ts` | 15.2 KB |
| `src/tools/modifyTools.ts` | 14.0 KB |
| `src/tools/curveTools.ts` | 13.9 KB |
| `src/render/webgl2/styledRenderer.ts` | 11.8 KB |
| `src/tools/cornerTools.ts` | 11.4 KB |
| `src/app/cloud/sync.ts` | 11.0 KB |
| `src/style/file.ts` | 10.6 KB |
| `src/tools/editTools.ts` | 10.2 KB |
| `src/tools/shapeTools.ts` | 10.1 KB |
| `src/app/fileIO.ts` | 9.4 KB |
| `src/viewport/overlay.ts` | 9.4 KB |
| `src/app/cloud/session.ts` | 9.4 KB |
| `src/ui/layers/LayersPanel.ts` | 9.3 KB |
| `src/tools/pathTool.ts` | 9.2 KB |
| `src/model/snapshot.ts` | 9.2 KB |
| `src/ui/processing/ProcessingPanel.ts` | 9.0 KB |
| `src/processing/runner.ts` | 8.8 KB |
| `src/tools/pathEditTools.ts` | 8.5 KB |
| `src/render/styledBatches.ts` | 8.4 KB |
| `src/tools/pointCalc.ts` | 8.2 KB |

## İsteğe bağlı (lazy) chunk'lar

| Dosya | gzip | En büyük modüller |
|---|---|---|
| `assets/AppMenu-D39rwsx9.js` | 5.1 KB | `appmenu/AppMenu.ts`, `styles/appmenu.css` |
| `assets/AppSettingsDialog-B4Txq0fO.js` | 4.2 KB | `settings/AppSettingsDialog.ts` |
| `assets/ConflictDialog-CFlBxqY8.js` | 1.1 KB | `cloud/ConflictDialog.ts` |
| `assets/CoordExportDialog-CRFm3aDS.js` | 1.7 KB | `io/CoordExportDialog.ts` |
| `assets/CoordImportDialog-K2bsER7s.js` | 3.3 KB | `io/CoordImportDialog.ts` |
| `assets/DxfExportDialog-DPNZHaOz.js` | 2.8 KB | `io/DxfExportDialog.ts` |
| `assets/DxfImportDialog-BkVNq0tM.js` | 2.2 KB | `io/DxfImportDialog.ts` |
| `assets/LayerStyleDialog-Bc64sgS0.js` | 5.8 KB | `style/LayerStyleDialog.ts`, `style/rulesEditor.ts`, `style/symbolSlot.ts` |
| `assets/LegendDialog-BnNme3mt.js` | 2.3 KB | `style/LegendDialog.ts`, `style/legend.ts` |
| `assets/LoginDialog-CVZhViT7.js` | 1.0 KB | `cloud/LoginDialog.ts` |
| `assets/ModelDesigner-CZYjgQzv.js` | 10.7 KB | `model/modelInspector.ts`, `model/ModelDesigner.ts`, `model/ModelCanvas.ts` |
| `assets/NewProjectDialog-CUvE_gjO.js` | 1.7 KB | `settings/NewProjectDialog.ts` |
| `assets/ProjectActions-Bgu9n7Cj.js` | 1.3 KB | `cloud/ProjectActions.ts` |
| `assets/ProjectSettingsDialog-CWmsdj-H.js` | 2.2 KB | `settings/ProjectSettingsDialog.ts` |
| `assets/ProjectsDialog-B8k1BJoJ.js` | 2.0 KB | `cloud/ProjectsDialog.ts` |
| `assets/Ribbon-BrwOz4_U.js` | 12.1 KB | `ribbon/Ribbon.ts`, `app/ribbon.ts`, `ribbon/controls.ts` |
| `assets/SettingsShell-CsbVfdfb.js` | 3.2 KB | `settings/crsPicker.ts`, `settings/SettingsShell.ts`, `settings/workspacePicker.ts` |
| `assets/StartScreen-Clmd7lGz.js` | 1.4 KB | `start/StartScreen.ts`, `styles/start.css` |
| `assets/StyleManager-DwVPHAvj.js` | 6.5 KB | `style/StyleManager.ts`, `style/managerDetails.ts` |
| `assets/SvgEditor-Bfn22_p0.js` | 48.7 KB | `svgedit/SvgEditor.ts`, `svgedit/svgCanvas.ts`, `svgedit/svgProps.ts` |
| `assets/SymbolDesigner-NtnYKRg4.js` | 9.5 KB | `style/layerForms.ts`, `style/SymbolDesigner.ts` |
| `assets/ToolDialog-CLaLWnmZ.js` | 8.7 KB | `processing/ToolDialog.ts`, `processing/paramFields.ts`, `processing/modelRunner.ts` |
| `assets/WebGPUBackend-C9fcW5gu.js` | 9.4 KB | `webgpu/styledShaders.ts`, `webgpu/styledRenderer.ts`, `webgpu/WebGPUBackend.ts` |
| `assets/appearancePickers-zRpYPubl.js` | 0.8 KB | `settings/appearancePickers.ts` |
| `assets/classify-C2udqnMe.js` | 1.4 KB | `style/classify.ts` |
| `assets/common-BGbu36dj.js` | 2.4 KB | `io/common.ts`, `io/client.ts`, `styles/io.css` |
| `assets/coords-OLfFaALF.js` | 0.7 KB | `io/coords.ts` |
| `assets/designerFields-DIkKqwJy.js` | 1.8 KB | `style/designerFields.ts` |
| `assets/recentList-DopqgN7V.js` | 0.5 KB | `start/recentList.ts`, `styles/recent.css` |
| `assets/sampleProject-BZSGOHy9.js` | 4.9 KB | `model/document.ts`, `model/sampleProject.ts`, `model/sameJson.ts` |
| `assets/scope-Cd1FEqaU.js` | 0.7 KB | `io/save.ts`, `io/scope.ts` |
| `assets/showcase-CGLPshhb.js` | 1.2 KB | `style/showcase.ts` |
| `assets/styleFiles-MTvMqefG.js` | 0.8 KB | `style/styleFiles.ts` |
| `assets/system-CzF_h3uS.js` | 59.9 KB | `mpyy/pictogramDrawings.ts`, `nip/09-teknik-altyapi.ts`, `ortak/02-sit.ts` |
| `assets/thumbs-DNt47K9y.js` | 4.4 KB | `render/symbolPreview.ts`, `style/thumbs.ts`, `style/compile.ts` |
| `assets/zoom-DlT2ybYU.js` | 1.1 KB | `io/apply.ts`, `io/zoom.ts` |
