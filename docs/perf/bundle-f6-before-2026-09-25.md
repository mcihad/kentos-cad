# Build envanteri: f6-before (2026-09-25, 55a0fff, çalışma ağacı temiz değil)

Ölçüm: `node scripts/perf/bundle.mjs --label f6-before`. Boyutlar ham / gzip (9) / brotli (11).

| Kapsam | Ham | gzip | brotli |
|---|---|---|---|
| İlk sayfa JS | 844.5 KB | 263.5 KB | 216.9 KB |
| İlk sayfa CSS | 114.4 KB | 19.2 KB | 16.8 KB |
| Bütün dist (82 dosya) | 5170.4 KB | 2554.4 KB | 2281.3 KB |

## Giriş chunk'ının en büyük modülleri (işlenmiş boyut)

| Modül | Boyut |
|---|---|
| `src/style/system/mpyy/pictogramDrawings.ts` | 34.7 KB |
| `src/tools/catalog.ts` | 29.7 KB |
| `src/viewport/ViewportController.ts` | 21.9 KB |
| `src/app/commands.ts` | 18.7 KB |
| `src/style/system/mpyy/nip/09-teknik-altyapi.ts` | 16.5 KB |
| `src/render/webgl2/styledShaders.ts` | 16.1 KB |
| `src/ui/processing/ToolDialog.ts` | 16.0 KB |
| `src/tools/areaTools.ts` | 15.3 KB |
| `src/ui/properties/PropertiesPanel.ts` | 15.2 KB |
| `src/render/webgpu/styledShaders.ts` | 14.5 KB |
| `src/tools/modifyTools.ts` | 14.0 KB |
| `src/tools/curveTools.ts` | 13.9 KB |
| `src/ui/settings/AppSettingsDialog.ts` | 13.5 KB |
| `src/render/webgl2/styledRenderer.ts` | 11.8 KB |
| `src/tools/cornerTools.ts` | 11.4 KB |
| `src/ui/processing/paramFields.ts` | 11.3 KB |
| `src/model/document.ts` | 11.1 KB |
| `src/app/cloud/sync.ts` | 11.0 KB |
| `src/style/system/mpyy/ortak/02-sit.ts` | 10.9 KB |
| `src/style/file.ts` | 10.6 KB |
| `src/style/system/mpyy/ortak/01-sinirlar.ts` | 10.2 KB |
| `src/tools/editTools.ts` | 10.2 KB |
| `src/tools/shapeTools.ts` | 10.1 KB |
| `src/style/system/mpyy/uip/11-ulasim.ts` | 9.5 KB |
| `src/app/fileIO.ts` | 9.4 KB |

## İsteğe bağlı (lazy) chunk'lar

| Dosya | gzip | En büyük modüller |
|---|---|---|
| `assets/AppMenu-BI6DDMOl.js` | 5.1 KB | `appmenu/AppMenu.ts`, `styles/appmenu.css` |
| `assets/CoordExportDialog-Bij0hykO.js` | 1.7 KB | `io/CoordExportDialog.ts` |
| `assets/CoordImportDialog-BeIvxbt2.js` | 3.3 KB | `io/CoordImportDialog.ts` |
| `assets/DxfExportDialog-Bq9TW6Hv.js` | 2.8 KB | `io/DxfExportDialog.ts` |
| `assets/DxfImportDialog-DPDuQ2cc.js` | 2.2 KB | `io/DxfImportDialog.ts` |
| `assets/LayerStyleDialog-D4u5fFna.js` | 5.8 KB | `style/LayerStyleDialog.ts`, `style/rulesEditor.ts`, `style/symbolSlot.ts` |
| `assets/LegendDialog-DTQxPq13.js` | 2.3 KB | `style/LegendDialog.ts`, `style/legend.ts` |
| `assets/ModelDesigner-VOPtp_KW.js` | 10.7 KB | `model/modelInspector.ts`, `model/ModelDesigner.ts`, `model/ModelCanvas.ts` |
| `assets/Ribbon-CB3-uKLD.js` | 12.1 KB | `ribbon/Ribbon.ts`, `app/ribbon.ts`, `ribbon/controls.ts` |
| `assets/StartScreen-EjEH634P.js` | 1.4 KB | `start/StartScreen.ts`, `styles/start.css` |
| `assets/StyleManager-DzJmkV6q.js` | 6.5 KB | `style/StyleManager.ts`, `style/managerDetails.ts` |
| `assets/SvgEditor-BW303XJX.js` | 48.7 KB | `svgedit/SvgEditor.ts`, `svgedit/svgCanvas.ts`, `svgedit/svgProps.ts` |
| `assets/SymbolDesigner-CI2uWDIH.js` | 9.5 KB | `style/layerForms.ts`, `style/SymbolDesigner.ts` |
| `assets/classify-DiDCsToC.js` | 1.4 KB | `style/classify.ts` |
| `assets/common-2DDtsXFG.js` | 2.4 KB | `io/common.ts`, `io/client.ts`, `styles/io.css` |
| `assets/coords-OLfFaALF.js` | 0.7 KB | `io/coords.ts` |
| `assets/designerFields-CZqPS9sZ.js` | 1.8 KB | `style/designerFields.ts` |
| `assets/recentList-DopqgN7V.js` | 0.5 KB | `start/recentList.ts`, `styles/recent.css` |
| `assets/scope-Cd1FEqaU.js` | 0.7 KB | `io/save.ts`, `io/scope.ts` |
| `assets/styleFiles-CNU6XYLE.js` | 0.8 KB | `style/styleFiles.ts` |
| `assets/thumbs-B8IaJmtj.js` | 4.4 KB | `render/symbolPreview.ts`, `style/thumbs.ts`, `style/compile.ts` |
| `assets/zoom-3ziHSqq2.js` | 1.1 KB | `io/apply.ts`, `io/zoom.ts` |
