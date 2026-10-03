# Pafta düzeni dalını `main`'e bağlama

Bu belge `feat/sheet-layouts` dalının `main`'e nasıl bağlanacağını anlatır. Sahibi başka bir
bilgisayarda `main` üzerinde çalışmayı sürdürüyor. Bu yüzden dal yeni kodu yeni dizinlerde tutar.
Paylaşılan dosyalardaki her değişikliği ayrı bir bağlama commit'inde toplar ve satır satır yazar
(§3). Çakışma ancak o commit'te çıkabilir; çıkarsa §3'teki tarif yeniden uygulanır.

## Kısa yol

1. `git fetch origin && git switch feat/sheet-layouts && git rebase origin/main`
2. Çakışma çıkarsa yalnız 6. ve 7. commit'tedir (§2). Paylaşılan dosyada `main`'in sürümünü al,
   §3'teki satırı yeniden uygula. Üretilen dosyaları yeniden üret (§1.4).
3. Migration ve ADR numarasını boş numaraya göre ayarla (§4.2, §4.3).
4. §5'teki doğrulamayı sırayla koş.
5. `git switch main && git merge --ff-only feat/sheet-layouts`.

## 1. İlke

1. **Yeni kod yeni dizinlerde.** Dalın içeriğinin neredeyse tamamı `main`'de olmayan dizinlerdedir:

   | Dizin | İçerik |
   |---|---|
   | `crates/shared/sheet/` | çekirdek (`kentos-sheet`) |
   | `crates/wasm/sheet-wasm/` | web bağlayıcısı (`kentos-sheet-wasm`) |
   | `crates/sheet-ui/` | masaüstü tasarımcı (`kentos-sheet-ui`) |
   | `apps/web/src/product/sheet/`, `render/sheet/`, `tools/sheet/`, `ui/sheet/`, `app/sheet/` | web |
   | `apps/web/src/contracts/generated/sheet/` | üretilmiş TS tipleri |
   | `apps/web/scripts/e2e/sheet-shots.mjs` | web görüntü betiği |
   | `fixtures/sheet/v1/` | ortak örnekler |
   | `scripts/fonts/sheet_metrics.py` | yazı ölçü tablosu üreticisi |
   | `docs/sheet/` | bu belgeler |

   Sunucu, masaüstü bulut istemcisi ve masaüstü uygulamasında da yeni dosyalar açılır:
   `crates/server/application/src/sheet_templates.rs` (ve `tests/sheet_templates.rs`),
   `apps/api/src/http/sheet_templates.rs` (ve `sheet_templates_tests.rs`),
   `crates/native/cloud/src/sheet_templates.rs`, bir migration dosyası, `apps/desktop/src/sheets.rs`.
2. **Paylaşılan dosyaya yalnız bağlantı noktasında dokunulur** (§3).
   - Toplam: izlenen 42 dosyada +1 204 / −113 satır (2026-10-03, son hâl; `Cargo.lock` hariç). Çoğu
     ekleme; her biri tabloda tarifiyle ve doğrulamasıyla yazılıdır.
   - Planda “birkaç tek satır” öngörülmüştü. Masaüstü ve web kabuğuna gerçek bağlanma bundan
     büyük çıktı; aşağıdaki risk tablosu bunu açıkça söyler.

   | Risk | Dosyalar | `main` aynı yeri değiştirdiyse |
   |---|---|---|
   | Düşük: liste girdisi, modül ya da bağımlılık satırı | `Cargo.toml`, `package.json`, `.gitignore`, `scripts/wasm/ensure.mjs`, `scripts/arch/deps.mjs`, `apps/api/Cargo.toml`, `crates/server/application/Cargo.toml`, `crates/native/cloud/Cargo.toml`, `apps/desktop/Cargo.toml`, `…/lib.rs` ve `mod.rs` modül satırları, `apps/desktop/src/main.rs`, `apps/web/src/app/createApp.ts`, `app/ribbon.ts`, `viewport/ViewportController.ts`, `app/cloud/api.ts`, `crates/native/cloud/src/api.rs`, `crates/server/application/src/commands.rs`, `apps/api/src/http/projects.rs` | `main`'in sürümünü al, satırı yeniden ekle |
   | Orta: bir işlevin içine giren değişiklik | `apps/web/src/ui/ribbon/Ribbon.ts` (W-3), `ui/shell/AppShell.ts` (W-2), `app/commands.ts` (W-10, W-17), `render/canvasShapes.ts` (W-14), `apps/desktop/src/app.rs` (D-3, D-8), `view.rs` (D-4, D-10), `input.rs` (D-5), `labels.rs` (D-11), `drawing_fonts.rs`, `style/scene.rs`, `crates/render/wgpu/src/styled/atlas.rs`, `raster.rs`, `crates/ui/src/widget/number.rs`, `attribute/number.rs`, `attribute/field.rs`, `widget/inspector.rs`, `widget/tabs.rs`, `style/text.rs` (U-1… ve 5–9. adımların satırları) | `main`'in sürümünü al, §3'teki tarifi o sürüme elle uygula; satırın doğrulamasını koş |
   | Geçici | `apps/desktop/src/drawing_menus.rs`, `marks.rs` (D-7) | ayrı commit'tedir (§2, 7); `main` ADR 0163'ün masaüstü adımını aldıysa bu commit atılır |
3. **Bağlantılar ayrı commit'tedir** (§2). Özellik commit'leri yalnız yeni dosya içerir ve
   hiçbir zaman çakışmaz. Bağlama commit'i çakışırsa §3'teki satırlar elle yeniden uygulanır.
4. **Üretilen dosyalar birleştirilmez, yeniden üretilir:** `Cargo.lock`, `pnpm-lock.yaml`, envanter,
   `pkg/` çıktıları. Çakışmada `main`'in sürümü alınır, sonra üretici çalıştırılır (§5).

## 2. Dalın commit düzeni

| Sıra | Commit | İçerik |
|---|---|---|
| 1 | `docs(sheet): reviews of PiriCAD and QGIS, the design, the tasks and this guide` | yalnız `docs/sheet/` |
| 2 | `feat(sheet): the sheet core` | `crates/shared/sheet/`, `fixtures/sheet/`, `scripts/fonts/sheet_metrics.py`, `scripts/geodesy/wmm_coefficients.py` |
| 3 | `feat(sheet,web): the WASM binding and the web sheet mode` | `crates/wasm/sheet-wasm/`, `apps/web/src/contracts/generated/sheet/`, `apps/web/src/{product,render,tools,ui,app}/sheet/`, `apps/web/scripts/e2e/sheet-{shots,cloud,pdf,north,a11y}.mjs` |
| 4 | `feat(sheet,server): the cloud template library` | `crates/server/application/src/sheet_templates.rs`, `tests/sheet_templates.rs`, `apps/api/src/http/sheet_templates.rs`, `sheet_templates_tests.rs`, `crates/server/postgres/migrations/0013_sheet_templates.sql`, `0014_org_sheet_templates.sql`, `crates/native/cloud/src/sheet_templates.rs`, `sheet_library.rs` |
| 5 | `feat(sheet,desktop): the desktop sheet designer` | `crates/sheet-ui/`, `apps/desktop/src/{sheets,sheet_library,sheet_library_tests,sheet_inputs,map_vectors,sheet_pdf}.rs`, `crates/render/wgpu/src/styled/cpu.rs` |
| 6 | `chore(sheet): connect the sheet layouts to the workspace and the apps` | **§3'teki bağlantı noktaları** (D-7 hariç), `docs/deps/README.md` ve `Cargo.lock` |
| 7 | `chore(desktop): temporary match arms for ADR 0163's new snap kinds` | yalnız D-7 (`drawing_menus.rs`, `marks.rs`); tek başına atılabilsin diye ayrı |

**Derlenme:**
- 1–5'teki dosyaların çoğu ancak 6'daki satırlarla derlemeye girer: çalışma alanı üyeliği,
  `mod` satırları, `installSheets` çağrısı.
- 1, 2, 4 ve 5 tek başına derlenir: yeni crate'ler 6'ya kadar çalışma alanı üyesi değildir, yeni
  `.rs` dosyaları 6'daki `mod` satırlarına kadar derlemeye girmez. 4'teki migration'lar
  `sqlx::migrate!` ile hemen gömülür; tabloları kurmaktan başka etkileri yoktur.
- 3 tek başına tip denetiminden geçmez. Web'in tip denetimi bağlanmamış dosyaları da tarar; bu
  dosyalar WASM paketine (`ensure.mjs` girdisi) ve kabuğun yeni yöntemlerine
  (`AppShell.sheetTabs`, `extendRibbon`) dayanır. İkisi de 6'da gelir.
- Dal bir bütün olarak birleştirilir (`git merge --ff-only`); ara commit'ler `git bisect` için
  derlenebilir değildir. 6'dan sonraki her commit tam derlenir ve sınanır.

Kökteki izlenmeyen `src/` dizini (24 Eylül tarihli eski WASM çıktısı) bu dala ait değildir, hiçbir
commit'e girmez.

## 3. Bağlantı noktaları

Bu tabloyu ajanlar, dokundukları her paylaşılan dosya için doldurur. Bir satır yalnız bu tablodaysa
yapılmıştır.

| # | Dosya | Değişiklik | Neden | Doğrulama |
|---|---|---|---|---|
| R-1 | `Cargo.toml` | `members` ve `default-members` listelerine `crates/shared/sheet`, `crates/wasm/sheet-wasm`; 4. adımda `crates/sheet-ui` yalnız `members`'a, `crates/ui`'nin ardına (yorumuyla; masaüstüne özgü, `crates/ui` gibi `default-members`'ta değil) | çalışma alanı üyeleri açık listelenir | `cargo metadata --locked` |
| R-2 | `Cargo.lock` | yeniden üretilir | — | `cargo metadata --locked` |
| R-3 | `scripts/arch/deps.mjs` | yeni grup `{ name: 'sheet-ui', path: 'crates/sheet-ui/', targets: [HOST], uses: ['shared', 'ui'], forbid: [...RUNTIMES, ...BROWSER, 'pyo3*'] }` (yorumuyla, `render` grubunun önüne); `apps/desktop/` `desktop` grubunun `uses` listesinin sonuna `'sheet-ui'` | ADR 0010: her crate'in grubu olmalı | `pnpm arch:deps`: “33 crate, 42 crate × hedef” |
| R-4 | `package.json` (kök) | `"rust:wasm:sheet": …` betiği (öbür `rust:wasm:*` satırlarının kalıbında) | `ensure.mjs` paketi bu betikle kurar | `pnpm rust:wasm:sheet` |
| R-5 | `scripts/wasm/ensure.mjs` | `PACKAGES` listesine `{ label: 'Pafta çekirdeği', script: 'rust:wasm:sheet', out: 'apps/web/src/product/sheet/pkg', lib: 'kentos_sheet_wasm', sources: ['crates/shared/sheet', 'crates/wasm/sheet-wasm', 'crates/shared/expression', 'crates/shared/geometry-core', 'crates/shared/contracts', ...PINS] }` (yorum satırıyla); dosyanın baş yorumunda paketin adı (bir yan cümle) | web'in her `pnpm dev/test/build`'i paketi tazeler | `pnpm wasm` |
| W-1 | `apps/web/src/app/createApp.ts` | `import { installSheets } from './sheet/install';` ve `shell = new AppShell(ctx); root.replaceChildren(shell.el);` satırlarının hemen ardından tek satır `installSheets(ctx, shell, frontHistory);` (yorumuyla). B bölümünde W-10 için: `Signal` ve `type FrontHistory` içe aktarımı; `registerCoreCommands`'tan önce `const frontHistory = new Signal<FrontHistory \| null>(null, () => false);` (yorumuyla) ve kancalara `frontHistory,` (+8 / −1) | pafta kipinin tek kurulum kancası: sekmeler, Pafta sekmesi, komutlar, kısayollar; öndeki paftanın geri alma yığını | `pnpm typecheck`; uygulama açılınca çizim alanının altında “Model \| +” |
| W-2 | `apps/web/src/ui/shell/AppShell.ts` | (1) `readonly sheetTabs` yuvası (`div.shell__sheet-tabs`), `shell__center` içinde `viewportHost` ile `bottom.el` arasına; (2) `readonly status: StatusBar` (önceden yerel `const status`); (3) `extendRibbon(ext: RibbonExtension): Disposable` → `this.ribbon.extend(ext)`; (4) baştaki düzen yorumuna sekme satırı; `type Disposable` ve `type RibbonExtension` içe aktarımları | sekme şeridinin yeri; pafta kipinin durum hücreleri (`status.el`'in başına eklenir); bağlamsal sekme | `pnpm typecheck`; `node apps/web/scripts/e2e/sheet-shots.mjs` |
| W-3 | `apps/web/src/ui/ribbon/Ribbon.ts` | `export interface RibbonExtension` ve `extend(ext)`: başka bir parçanın bağlamsal sekmesi. `derive()` onun sekmelerini ekler; `updateExtension()` `shown` doğruyken gösterir, görününce açar, gidince son olağan sekmeye döner; sekme ipucu uzantıdan; sayı hapı yalnız `'selection'` sekmesinde; `rebuildAll()` açık uzantı sekmesini korur (+ `type Disposable`, `type ReadonlySignal` içe aktarımı; ~50 satır) | Pafta sekmesi `RIBBON_TABS`'ta değildir: içeriği kip profilinden gelir (§11a) ve yalnız bir pafta öndeyken görünür | `pnpm typecheck`; `vitest run src/app/ribbon.test.ts src/ui/ribbon` |
| W-4 | `apps/web/src/app/ribbon.ts` | `RibbonTab.contextual` tipi `'selection'` → `'selection' \| 'sheet'` (yorumuyla) | Pafta sekmesi bağlamsal sekmedir | `pnpm typecheck` |
| W-5 | (dosya değişmez) `apps/web/src/ui/icons.ts` | pafta simgeleri `ui/sheet/icons.ts`'te; kurulumda `registerSheetIcons()` onları `ICONS`'a ekler, var olan bir adın üstüne yazmaz | şerit, menü ve paneller onları `icon()` ile çizer | birleştirmede istenirse simgeler `icons.ts`'e taşınır (envanter de görür) |
| W-6 | (dosya değişmez) komut ve kısayol kaydı | `sheet.*` komutları ve pafta kipinin tuşları `installSheets` içinden `ctx.commands.registerAll` ve `ctx.keymap.bind` ile; `app/keybindings.ts` değişmedi; `app/commands.ts`'te yalnız W-10'un kancası var | kayıt tek yerde, yeni dizinde | F1 kısayol listesinde “Pafta” kategorisi |
| W-7 | üretilen: `docs/inventory/web.{json,md}` | bu dalda yeniden üretilmedi (paylaşılan, üretilmiş dosya) | yeni komutlar (B ile `sheet.add.<araç>[.<hazır>]`, `sheet.zoomSelection`, `sheet.export.*`, `sheet.importKpafta` …), pencereler ve IndexedDB `kentos.sheets.v1` (sürüm 2) envantere girer | birleştirmede §5 `pnpm inventory` |
| W-8 | (dosya değişmez) kabuğun sınıf adları | `ui/sheet/sheet.css`, `.shell[data-sheet-mode]` altında `.shell__right`'ı ve durum çubuğunun `.status__coords`, `.status__sel`, `.status__toggles`, `.status__zoom` hücrelerini gizler; `data-sheet-mode`'u `installSheets` koyar ve kaldırır | pafta öndeyken denetçi sağ dokun, pafta hücreleri çizimin hücrelerinin yerini alır | bu sınıflar `main`'de yeniden adlandırılırsa `sheet.css` de değişir |
| W-9 | `CLAUDE.md` §2 (birleştirmede) | yerel depolar listesine IndexedDB `kentos.sheets.v1`, sürüm 2 (`books`: projenin pafta kitabı, anahtar `bulut/…`, `proje/…`, `dosya/…`, `oturum/…`; `templates`: bu cihazdaki şablonlar; `assets`: paftalardaki resimlerin baytları, anahtar SHA-256, yazılırken özeti denetlenir) eklenmeli. Sürüm 1'den yükseltme yalnız eksik depoyu açar, kayıtlara dokunmaz. C bölümüyle `templates` hesabın bulut kitaplığının kopyalarını da tutar: her kayıtta hesap, dayandığı revizyon, değişti mi, silindi mi, rol, sahip, eski kimlikler (`TemplateCloudState`) | depolar orada listelenir | bu dalda `CLAUDE.md`'ye dokunulmadı |
| W-10 | `apps/web/src/app/commands.ts` | `CommandHooks.frontHistory?: ReadonlySignal<FrontHistory \| null>` (yorumuyla) ve `export interface FrontHistory { undo; redo; canUndo; canRedo }`; `edit.undo` / `edit.redo`: `run` önce öndeki yığına gider, `isEnabled` onun `canUndo()` / `canRedo()`'su, `watch` sinyali de izler; `ReadonlySignal` içe aktarımı (+23 / −5). Sinyali `installSheets` doldurur: pafta öndeyken paftanın `SheetHistory`'si, Model öndeyken `null` | inceleme notu 4: hızlı erişimin Geri al / Yinele'si ve Ctrl+Z / Ctrl+Y pafta öndeyken paftanın geçmişini kullanır; en küçük bağlantı: tek isteğe bağlı kanca, `null`'da davranış eskisiyle aynı | `sheet-shots.mjs` `tuslar` sahnesi (hızlı erişimdeki Geri al, Ctrl+Y, Ctrl+Z paftada; yanlışsa koşu düşer); `pnpm test` |
| W-11 | `apps/web/src/viewport/ViewportController.ts` | salt okunur `get geometry(): PickIndex { return this.picker; }` (yorumuyla, +8) | harita çerçeveleri (`app/sheet/mapFrames.ts`) çizimi mevcut çizim hattıyla ekran dışında çizer: katmanlar ve yazılar görünüm penceresinin kendi geometri deposundan kurulur, ikinci bir kopya yapılmaz | `pnpm typecheck`; `b1-ifraz-sistem-sablonu` resminde harita çerçevesinde çizim ve yazıları |
| W-12 | `apps/web/src/app/cloud/api.ts` | `HttpCloudApi`'ye iki genel yöntem (yorumlarıyla, +11): `tenantCommand<T>(envelope)`: `POST /v1/tenants/{t}/commands` (bir alanın kendi komutu, projesiz); `read<T>(path, signal?)`: proje dışı bir okuma (`/v1/me/sheet-templates…`, `/v1/sheet-templates/…`). `CloudApi` arayüzü değişmedi (`FakeServer`'a dokunulmadı) | koordinatörün notu: kişisel alanın `sheet.template.*` komutları ve şablon kitaplığının okumaları, istemcinin kurallarıyla (oturum çerezi, `x-kentos-client: web`, zaman aşımı, sunucunun Türkçe iletisiyle `ApiFailure`). Uçların bilgisi `app/sheet/cloudApi.ts`'te kalır | `node apps/web/scripts/e2e/sheet-cloud.mjs` (gerçek kentosd); `pnpm typecheck`; `pnpm e2e` |
| W-13 | yeni dosya `apps/web/scripts/e2e/sheet-cloud.mjs` | paylaşılan dosya değil; A'daki listede olmadığı için yazılı. `cloud.mjs`'in kalıbında, ama geliştirme veritabanı yerine geçici bir docker kabında (boş port, rastgele parolalar). Sunucunun ayarları geçici dizinde (`KENTOS_ENV_FILE`); kökteki `.env.local`'e ve 5432'deki veritabanına dokunmaz; kap sonunda durdurulup silinir | C bölümünün uçtan uca denetimi ve görüntüleri; D+'da görüntüler iki boyda (1440×900, 1100×650); F'de kurum senaryosu (yönetici yayımlar; düz üye görür, kullanır, düzenleyemez; misafir ve ayrılan görmez; 11–16. görüntüler) | `node apps/web/scripts/e2e/sheet-cloud.mjs` (docker ve `postgis/postgis:18-3.6` gerekir; `target/debug/kentosd` derlenmiş olmalı) |
| W-14 | `apps/web/src/render/canvasShapes.ts` | `export type PathSink = Pick<Path2D, 'moveTo' \| 'lineTo' \| 'arc' \| 'rect' \| 'closePath'>` ve `shapeOutline(p, shape, hw, hh, params)`: `shapePath`'in gövdesi buraya taşındı, verilen hedefe çizip onu döndürür; `shapePath` artık `shapeOutline(new Path2D(), …)`; `gearPath` `PathSink` alır (yorumlarıyla, +9 / −2). Davranış aynı | D bölümü: PDF'in vektör haritası nokta simgelerini ekrandaki şekillerle aynı çizer (`app/sheet/mapVectors.ts`, yayları kirişle kıran bir hedefe); şekillerin ikinci bir kopyası yazılmaz | `pnpm test` (simge ve fixture testleri); `pnpm e2e`; `sheet-pdf.mjs` |
| W-15 | yeni dosya `apps/web/scripts/e2e/sheet-pdf.mjs` | paylaşılan dosya değil; A'daki listede olmadığı için yazılı (W-13 gibi). Demo çizimde ifraz paftasının PDF'ini uygulamada yazar ve `pdfinfo`, `pdffonts`, `pdftotext`, `gdalinfo`, `pdfimages`, `pdftoppm` ile denetler; çekirdeğin altın PDF'ini tarayıcıda yazar; dışa aktarma penceresinin Yazdır, Yeni sekmede aç, Kaydet ve Hepsi yollarını sınar | D bölümünün uçtan uca denetimi; tasarım §9a “Kabul ölçütleri”; F'de dış çizgi denetimleri (delikli simge, dolu daire; haritanın resminde ve PDF'te), SVG resim (pencerenin notu, gömülü 709 × 355 resim) ve yedek harita resminin yumuşak maskesi | `node apps/web/scripts/e2e/sheet-pdf.mjs` (poppler-utils ve GDAL'ın PDF sürücüsü gerekir) |
| W-16 | (dosya değişmez) `apps/desktop/assets/fonts/drawing/*.ttf` | web bu 22 dosyayı `app/sheet/pdfFonts.ts`'te `import.meta.glob(…, { query: '?url' })` ile tembel varlık olarak alır: derlemede her biri ayrı bir varlık (toplam 1,8 MB), yalnız PDF yazılırken indirilir | tasarım §9a: web masaüstünün gönderdiği dosyaların aynısını gönderir; kopya yok | dosyalar taşınır ya da adı değişirse `pdfExport.test.ts` düşer (motorun her yüzü için dosya aranır); `sheet-pdf.mjs` altın PDF'i bayt bayt denetler |
| W-17 | `apps/web/src/app/commands.ts`, `apps/web/src/app/createApp.ts` | (D+) `CommandHooks.frontPrint?: ReadonlySignal<(() => void) \| null>` (yorumuyla). `file.print` (Yazdır ve pafta, Ctrl+P, uygulama menüsü) öndekinin yazdırması varsa onu çalıştırır; `pending` bir getter olur, pafta öndeyken “Geliştirme aşamasında” demez; `watch` sinyali izler (`commands.ts` +17 / −1). `createApp.ts`: `const frontPrint = new Signal…`, hook'a ve `installSheets`'e verilir (+3). Sinyali `installSheets` doldurur: pafta öndeyken `sheet.print`, Model öndeyken `null` | koordinatörün D+ notu 2: pafta öndeyken genel Yazdır paftayı yazdırır; W-10'un kalıbı, en küçük bağlantı. `null`'da davranış eskisiyle aynı | `sheet-pdf.mjs` “file.print” denetimi (pafta öndeyken pencere “Yazdır: …” açılır, `pending` yanlış; Model öndeyken doğru); `pnpm e2e` |
| W-18 | yeni dosya `apps/web/scripts/e2e/sheet-a11y.mjs` | paylaşılan dosya değil; W-13 gibi yazılı. Chrome'un erişilebilirlik ağacından (CDP `Accessibility.getFullAXTree`; paket yok) pafta kipini, galeriyi, paylaşım ve dışa aktarma pencerelerini denetler; dışa aktarma penceresinde ve galeride Tab'la gezer | koordinatörün D+ notu 3 | `node apps/web/scripts/e2e/sheet-a11y.mjs [--strict]` |
| W-19 | yeni dosya `apps/web/scripts/e2e/sheet-north.mjs` | paylaşılan dosya değil; W-13 gibi yazılı. Demo çizimin ifraz paftasında manyetik kuzey okunu denetler: denetçinin sapması ve kaynağı, “Sapmayı elle gir”, kuzey çizelgesi (kâğıtta ve PDF'te), iki ön denetim bulgusunun düzeltmeleri | E bölümünün uçtan uca denetimi; F'de değerler `northInfo`'dan, açılar `'` ile | `node apps/web/scripts/e2e/sheet-north.mjs` (poppler-utils gerekir) |
| R-6 | `.gitignore` | `apps/web/src/product/sheet/pkg/` satırı, öbür `pkg/` satırlarının ardına | üretilen WASM paketi depoya girmez (öbür paketler gibi; §1.4) | `pnpm wasm` sonrası `git status`'ta `pkg/` görünmez |
| R-7 | `crates/server/application/Cargo.toml` | `[dependencies]`: `kentos-sheet = { path = "../../shared/sheet", default-features = false }`; `[dev-dependencies]`: aynısı `features = ["schema"]` ile (ikisi de yorumlu) | sunucu şablonu cihazın okuduğu gibi okur ve denetler; komut tipleri web ve masaüstünün gönderdiği tiplerdir; test, katalog girdilerini karşılaştırır | `cargo test -p kentos-application --test sheet_templates` |
| R-8 | `crates/server/application/src/lib.rs` | `pub mod sheet_templates;` (yorumuyla), `sharing` ile `snapshot` arasına | şablon kitaplığının kullanım durumları | `cargo build -p kentos-application` |
| R-9 | `crates/server/application/src/commands.rs` | (1) `CommandOutcome::SheetTemplate(kentos_sheet::cloud::SheetTemplateChanged)`; `committed()` onun için `false` (bir projeye olay gitmez), `to_json()` onu yazar; (2) `run_in_tenant`'ın başında `if crate::sheet_templates::handles(&envelope.command_name) { return … run(…).map(CommandOutcome::SheetTemplate); }` (~10 satır) | beş `sheet.template.*` komutu kişisel alanın komut ucuna gelir (ADR 0013 zarfıyla) | `cargo test -p kentos-application` (bütün testler) |
| R-10 | `apps/api/Cargo.toml` | `kentos-sheet = { path = "../../crates/shared/sheet", default-features = false }` (yorumuyla) | uçların yanıt tipleri | `cargo build -p kentos-api` |
| R-11 | `apps/api/src/http/mod.rs` | `pub mod sheet_templates;` ve `#[cfg(test)] mod sheet_templates_tests;`; `router`'da `/v1/invitations/accept` satırının ardına beş satır: `GET /v1/me/sheet-templates`, `GET /v1/me/sheet-templates/events`, `GET /v1/sheet-templates/{id}`, `GET …/{id}/access`, `GET …/{id}/access/candidates` | okuma uçları (design §13) | `cargo test -p kentos-api sheet_templates` |
| R-12 | `apps/api/src/http/projects.rs` | `tenant_command`: yanıttan önce tek satır `super::sheet_templates::announce(&state.hub, &o);` (yorumuyla; `map` kapanışı bloğa döndü) | değişen bir şablon, bu süreçte bekleyen şablon olayı isteklerini hemen uyandırır (proje olaylarının `hub` kalıbı; sinyal `(nil, nil)`, hiçbir projeninki değildir) | `the_desktop_client_keeps_a_template_library`: bekleyen uzun yoklama silmeyi 3 sn içinde duyar (sinyal kapatılınca test düşüyor, denendi) |
| R-13 | `crates/native/cloud/Cargo.toml` | `kentos-sheet = { path = "../../shared/sheet", default-features = false }` (yorumuyla) | masaüstü istemcisinin tipleri, `plan_sync`'in uzak tarafı | `cargo build -p kentos-cloud` |
| R-14 | `crates/native/cloud/src/lib.rs` | `pub mod sheet_templates;` (yorumuyla), `saving` ile `sync` arasına | masaüstünün şablon kitaplığı istemcisi | `cargo test -p kentos-cloud` |
| R-15 | `crates/native/cloud/src/api.rs` | `Cloud::get_at(path, query, timeout)` (`pub(crate)`, ~25 satır), `me()`'nin önüne: bir yolu sorgusuyla `GET` eder, JSON okur; uzun yoklama kendi süre sınırını verir | yeni modül `Inner`'a dokunmadan uçları çağırır | `cargo test -p kentos-api sheet_templates` (gerçek soket üzerinde) |
| R-16 | yeni dosya `crates/server/postgres/migrations/0013_sheet_templates.sql` | dalda ilk boş numara 0013'tü | `sqlx::migrate!` dizini numarasıyla okur | birleştirmede §4.2 |
| D-1 | `apps/desktop/Cargo.toml` | `kentos-render-wgpu` satırının ardına `kentos-sheet-ui = { path = "../../crates/sheet-ui" }` ve `kentos-sheet = { path = "../../crates/shared/sheet", default-features = false }` (yorumuyla) | pafta kipi ve ona verilen çekirdek tipleri (`sheets.rs`) | `cargo build -p kentos-desktop` |
| D-2 | `apps/desktop/src/main.rs` | `mod sheets;` (`mod shortcuts;` önüne); başlangıçta `app.cloud.replicas = …` satırının ardına `app.sheet_store = sheets::default_store();` (yorumuyla) | kitaplar uygulama veri klasöründe, projenin anahtarıyla (design §10) | uygulama açılınca `~/.local/share/kentos-cad/pafta/` |
| D-3 | `apps/desktop/src/app.rs` | (1) `App`'in sonuna 8 alan: `sheets`, `sheet_maps`, `sheet_tab`, `sheet_store`, `sheet_project`, `sheet_session`, `sheet_generation`, `sheet_attributes`; kurucuda karşılıkları (`viewport: Viewport::new(),` ardına); (2) `Message`'a `Sheet(kentos_sheet_ui::Message)` ve `SheetExportTo(ExportKind, Option<PathBuf>)` (`RibbonTab` ardına); (3) `handle`'da `RibbonTab` kolu: `sheet_tab = id == SHEET_TAB`, başka sekmede eski `tab_clicked`; iki yeni kol; (4) `update`'te `follow_document()` ardına `self.follow_sheets();` | pafta kipinin durumu, mesajları, Pafta sekmesi | `cargo test -p kentos-desktop sheets::` |
| D-4 | `apps/desktop/src/view.rs` | (1) `view()` başında: pafta öndeyse `self.sheet_view()` döner; çizim alanının sütununa `self.sheets.tabs().map(Message::Sheet)` (çizim ile alt panel arası); uygulama menüsünün ardına pafta penceresi katmanı; (2) `ribbon()`: Pafta açıkken `shown = SHEET_TAB`, pafta öndeyken bağlamsal “Pafta” sekmesi (`tab_tips`'e `None`), açıkken grupları `self.sheets.ribbon_groups(Message::Sheet)`; (3) `status_bar()` başında: pafta öndeyse paftanın hücreleri; (4) `ribbon`, `status_bar`, `dialog_view` `pub(crate)` oldu (`sheets.rs` kullanır) | design §11: sekmeler, bağlamsal sekme, kip, hücreler | `sheets::tests::a_sheet_opens_over_the_drawing_and_the_model_comes_back` (pencere yazılımsal çiziciyle çizilir) |
| D-5 | `apps/desktop/src/input.rs` | `key()`'de Esc/şerit bloğunun ardına: pafta öndeyken önce paftanın tuşları (`self.sheets.key`), sonra Ctrl/Alt'sız yazı tuşları (karakter, Enter, Boşluk, Tab, Esc) tutulur | gizli çizimin komut satırına yazılmasın, aracı başlamasın | aynı test: “c” tuşu çember aracını başlatmıyor (koruma kaldırılınca test düşüyor; denendi) |
| D-6 | yeni dosya `apps/desktop/src/sheets.rs` | bağlantının hepsi: `SheetMaps` (`MapPainter`, kentos-render-wgpu sahnesinden), `sheet_view`, `sheet_context`, `sheet_key`, `follow_sheets`, `sheet_message` (efektler: dosya pencereleri `rfd` ile), `sheet_export_to`; testler ve elle koşulan `sheet_screens` | — | `cargo test -p kentos-desktop sheets::` |
| D-7 | **geçici:** `apps/desktop/src/drawing_menus.rs`, `apps/desktop/src/marks.rs` | `SnapKind` eşleşmelerine `_ =>` kolları (5 yer: ad “Yakalama”, ikon `snapNearest`, iz adı `nearest`, işaret küçük daire) | **dalın HEAD'i (e21b094f, ADR 0163 1. adım) masaüstünü derlenmez bırakıyor**: çekirdeğe dört yeni yakalama türü eklendi, masaüstünün eşleşmeleri eksik (masaüstü adımı gelmedi). Bu kollar yalnız bağlantıyı derleyip sınamak için | birleştirmede ya da ADR 0163'ün masaüstü adımı geldiğinde **`main`'in sürümü alınır, bu kollar atılır** |
| R-17 | `crates/native/cloud/src/lib.rs` (4b) | `pub mod sheet_library;`, `pub mod sheet_templates;` satırının önüne, aynı yorumun altına (yorum genişledi: “the device's sync with it, its endpoints”); yeni dosya `crates/native/cloud/src/sheet_library.rs` | cihazdaki şablon kitaplığının buluta eşitlenmesi (`sync_once`, `DeviceLibrary`, web'in `SYNC_TEXTS`'i) masaüstünden bağımsız sınanabilsin diye istemcide; `kentos-sheet-ui` `kentos-cloud`'a bağlanamaz (`sheet-ui` grubu yalnız `shared` ve `ui`'yi kullanır) | `cargo test -p kentos-cloud sheet_library`; `cargo test -p kentos-api sheet_templates` (`the_desktop_syncs_a_template_library_across_devices_and_accounts`) |
| U-1 | `crates/ui/src/attribute/number.rs` (4b) | `set_point_rule(fn(f64, usize) -> String)` (bir kez, `OnceLock`) ve `point(value, decimals)` (kural verilmemişse eksi sıfırsız `format!`); test `a_point_number_has_no_grouping_and_no_minus_zero` (+38) | ADR 0149 kural 5 (ondalık ayırıcı nokta) pafta kipinde; kural `kentos-geometry-core::display::fixed`'tir, `kentos-ui` ona bağlanmaz, kipi kuran verir | `cargo test -p kentos-ui` |
| U-2 | `crates/ui/src/attribute/field.rs` (4b) | `Field.point: bool` (varsayılan `false`) ve `.point()`; `real_text()`; `format()` gerçel sayıyı onunla yazar (+22 / −3) | denetçinin alanı sayıyı noktalı yazsın; varsayılan değişmedi, başka kullanan etkilenmez | `cargo test -p kentos-ui` |
| U-3 | `crates/ui/src/widget/inspector.rs` (4b) | `point` alanında düzenlenen yazı `field.format(value)` (+4) | yazarken de nokta | `cargo test -p kentos-ui` |
| U-4 | `crates/ui/src/widget/number.rs` (4b) | `NumberInput::point()` (sayı nokta kuralıyla) ve `on_drag(msg)` (sürüklemenin ilk adımında bir kez yayınlanır); `plain()`, `begin()` `point` alır; testler güncellendi (+70 / −13) | denetçide sürüklenen sayı tek geri alma adımı olsun: ev sahibi sürüklemenin başını ve sonunu (`on_release`) bilir; varsayılan davranış aynı | `cargo test -p kentos-ui`; `a_dragged_number_is_one_undo_step_and_typed_ones_are_one_each` |
| D-8 | `apps/desktop/src/app.rs` (4b) | `App`'e iki alan, `sheet_library` ve `sheet_data_key` (kurucuda karşılıkları); `Message::SheetLibrary(LibraryMsg)` ve kolu; güncelleme toplu işinde `self.follow_sheet_library(Instant::now())`; `sheet_library.wants_ticks()` doğruyken `Subscription::run(sheet_library::ticks)` (~20 satır) | şablon kitaplığının oturumu, bağlantıyı ve olayları izlemesi (uzun yoklama, yeniden deneme, olaydan sonra eşitleme); paftanın tabloları ve lejantlarının verisi yalnız değişince yeniden hesaplanır | `cargo test -p kentos-desktop sheet_library::` (sunucusuz test), canlı test (aşağıda) |
| D-9 | `apps/desktop/src/main.rs` (4b) | `mod sheet_inputs; mod sheet_library;` (`mod sheets;` önüne) | yeni modüller | `cargo build -p kentos-desktop` |
| D-10 | `apps/desktop/src/view.rs` (4b) | pafta pencereleri katmanı boyayıcıyla: `self.sheets.window(std::rc::Rc::new(self.sheet_painter()))`; yorum tek satıra indi | galerinin küçük resimleri ve şablon önizlemesi projenin çizimini `MapPainter`'la çizer (gri “Harita” yer tutucusu değil) | `sheet_comparison_screens` (`b4-galeri-*` resimleri) |
| D-11 | `apps/desktop/src/labels.rs` (4b) | `Labels.fence: Option<Fence>` (çizim alanında `None`), `pub struct Fence` (kameradan harita çerçevesine: orta, boy, dönüş, ölçek), `pub fn paint_in_map(...)`; `Labels::draw` çerçeveyi kesen yazıyı çizmez; çitliyken kuzey oku ve ölçek çubuğu çizilmez, yer kameranın boyudur; `slots_of`'tan `pub fn slot_of(spot)` ayrıldı; başarım testine `fence: None` (+135 / −25) | harita çerçevelerinde çizimin yazıları (parsel numaraları, ada adları, etiketler) çizim alanının kendi etiket motoruyla, aynı yerleşim ve yazı tipiyle; web de çizim alanının yazılarını haritanın resmine çizer | `sheets::tests::a_map_keeps_inside_its_frame_and_writes_the_drawing_s_text`; `cargo test -p kentos-desktop labels::` |
| D-12 | yeni dosyalar `apps/desktop/src/sheet_library.rs`, `sheet_library_tests.rs`, `sheet_inputs.rs` (4b) | paylaşılan dosya değil; D-6 gibi yazılı. `sheet_library.rs`: kitaplığın ev sahibi (Iced görevleri, uzun yoklama, yeniden deneme, istekler); `sheet_inputs.rs`: paftanın tabloları, koordinat listeleri, lejantları ve koordinat sisteminin TM değerleri çizimden | — | `cargo test -p kentos-desktop sheet_library:: sheet_inputs::` |
| R-18 | `Cargo.toml` (5) | `[workspace.dependencies]`'e, `pollster` satırının ardına üç satır (yorumlarıyla): `pdf-writer = { version = "=0.15.0" }`, `subsetter = { version = "=0.2.6", default-features = false }`, `ttf-parser = { version = "=0.25.1", default-features = false, features = ["std"] }` (+11) | design §9a: PDF yazıcısı, yazı tipi alt kümesi (sahibin onayı, 3 Ekim), yazı tipi okuma (kilitteki paket) | `cargo metadata --locked`; `pnpm rust:wasm:sheet` |
| R-19 | `Cargo.lock` (5) | R-2'nin devamı: kilide yalnız `pdf-writer` 0.15.0 ve `subsetter` 0.2.6 girdi (cargo yazdı); `ttf-parser` zaten kilitliydi | — | `cargo metadata --locked` |
| R-20 | `docs/deps/README.md` (5) | başlıkta sayı 36 → 39 ve tarih; `pdf-writer`, `subsetter`, `ttf-parser` satırları; `miniz_oxide`, `sha2`, `libm`, `png` satırlarının “Kullanan”ına pafta crate'leri; geçişli sayı 652 → 657 | bağımlılık kaydının kuralı (README “Kurallar”) | belge |
| R-21 | `crates/render/wgpu/src/styled/raster.rs` (5) | `pub fn shape_outline(shape, hw, hh, params) -> Vec<(Vec<[f64; 2]>, bool)>`: `shape_path`'in yolu, eğrileri π/32'lik kirişlerle çoklu çizgi olarak, her parçanın kapalı olup olmadığıyla; test `every_shape_has_polylines_for_a_pdf` (+119) | masaüstünün PDF haritası nokta simgelerini işaretlerle aynı şekillerden yazar (web'in `canvasShapes.ts` `shapeOutline`'ı); şekillerin ikinci kopyası yazılmaz | `cargo test -p kentos-render-wgpu --lib raster` |
| U-5 | `crates/ui/src/widget/tabs.rs` (5) | `Tab::note(bool)`: başlığın yanında 6 piksellik bilgi mavisi nokta (`note_dot`); varsayılan kapalı (+28) | “Yeni sürüm var”: web'in `.sheet-tab__newer`'ı | `sheet_step5_screens` (`c5-yeni-surum`) |
| U-6 | `crates/ui/src/style/text.rs` (5) | `pub fn info(theme)`: bilgi mavisi yazı (`Tokens::info`) (+7) | “Yeni sürüm var” yazısı (web'in `tbadge--newer`'ı) | `cargo build -p kentos-ui` |
| D-13 | `apps/desktop/src/labels.rs` (5) | (1) `pub fn paper_colors()`: kâğıdın renkleri çekirdekten (`#111111` yazı, beyaz hale); `paint_in_map` ve `texts_in_map` artık `canvas` + `palette` yerine `Colors` alır; (2) `Fence::holds` artık yazının çapasına bakar: çapası çerçevede olan yazı çizilir (kenarda kesilir), `piece_box` kalktı; (3) `trait Ink` (çerçeve ya da liste) ve `struct Collect`: PDF için yazılar `MapLabel` (yerde çapa, `TextAnchor`, derece, kâğıt mm, renk, hale, maske kutusu); `Labels.current` yazının nesnesini (katmanı için) taşır (~+190 / −40) | PDF'in harita yazıları ekrandakinin yerleşimiyle, web'in çapalarıyla; koordinatörün 3. ve 4. maddesi | `sheets::tests::a_map_keeps_inside_its_frame_and_writes_the_drawing_s_text`; `sheet-pdf.mjs --masaustu` (99/99 sözcük, 0,00 mm) |
| D-14 | `apps/desktop/src/style/scene.rs` (5) | `shown_layers` ve `names` `pub(crate)` oldu; yeni `pub(crate) fn build_whole(store, library, node, entities, look, clip, names) -> StyledLayer` (bir katmanı stil motorundan bütün olarak geçirir; +37) | paftanın haritası (ekran ve PDF) çizim alanının stil motoruyla, haritanın ölçeğinde ve kâğıdın paletiyle kurulur (web'in `styleAt` + `buildStyledLayer`'ı) | `cargo test -p kentos-desktop sheet` |
| D-15 | `apps/desktop/src/main.rs` (5) | `mod map_vectors;` (`mod marks;` önüne) ve `mod sheet_pdf;` (`mod sheets;` önüne) | yeni modüller | `cargo build -p kentos-desktop` |
| D-16 | `apps/desktop/src/drawing_fonts.rs` (5) | `FACE_KEYS` (22 yüzün çekirdekteki adı, `FACES` sırasıyla), `pub fn pdf_fonts() -> Vec<PdfFont>` (gömülü TTF'ler), `pub fn sheet_font(DrawingFont) -> &str`; test `every_face_is_named_as_the_sheet_core_names_it` (+95) | PDF yazı tiplerini masaüstünün kendi dosyalarından alır | `cargo test -p kentos-desktop drawing_fonts` |
| D-17 | yeni dosyalar `apps/desktop/src/sheet_pdf.rs`, `map_vectors.rs` (5); `sheets.rs`, `sheet_inputs.rs`, `sheet_library_tests.rs` değişti (D-6, D-12) | paylaşılan dosya değil; D-12 gibi yazılı. `sheet_pdf.rs`: haritanın vektörleri, PDF girdileri, Yazdır; `map_vectors.rs`: web'in `mapVectors.ts`'inin karşılığı; `sheet_inputs.rs`: `pdf_crs`; `sheets.rs`: boyayıcı stilli katmanları çizer, PDF ve Yazdır efektleri | — | `cargo test -p kentos-desktop sheet map_vectors` |
| G-1 | üretilen: `apps/web/src/contracts/generated/sheet/` (5) | yeni tipler `PdfInputs`, `PdfOptions`, `PdfFont`, `PdfFace`, `PdfAsset`, `PdfMap`, `PdfCrs`, `MapContent`, `VectorMap`, `RasterMap`, `MapLayerContent`, `MapPath`, `MapStroke`, `MapText`, `TextAnchor`, `TmCrs`, `TextRun`, `PaperPalette`; `Variable.ts`'in yorumu ‹ad?› | ts-rs (`cargo test -p kentos-sheet`) | `pnpm typecheck` |
| R-22 | yeni dosya `scripts/geodesy/wmm_coefficients.py` (6) | paylaşılan dosya değil; yeni betik (W-15 gibi yazılı). NOAA'nın resmî `WMM.COF`'undan `crates/shared/sheet/data/wmm2025.json`'u üretir; `--check` (tablo güncel mi), `--fetch` (NOAA'nın zip'i tutulan dosyayla aynı mı, SHA-256) | design §8a: veri resmî dosyadan betikle üretilir | `python3 scripts/geodesy/wmm_coefficients.py --check` |
| R-23 | `docs/deps/README.md` (6) | yeni bölüm “Gömülü veri”: WMM2025 satırı (yayın 17 Aralık 2024, geçerlilik 2025,0–2030,0, kamu malı, kaynak adresi ve SHA-256) | koordinatörün 6. adım notu: kaynak, tarih, geçerlilik ve lisans `docs/deps`'te de yazılı | belge |
| R-24 | `docs/sheet/design.md` (5, 6, 6b, 7, 8, 9) | §6 “Eksik karakter”; §7 ve §8 işaret `‹ad?›`; §8a “Uygulama” ve “Sığdırma ve bilgi” (6b: ASCII açı işaretleri, yazının sığması, `northInfo`); §9 ön denetim listesi; §9a “İki platformun haritası aynıdır”; §13 “Kurum şablonları”nın “Uygulama”sı (7); açık soru 6'nın “Uygulandı”sı (8); §8a denetçi, §9a SVG resim ve stilli harita resmi (9) | kararların belgede yazılı olması (koordinatörün notları) | belge |
| R-25 | yeni dosya `crates/server/postgres/migrations/0014_org_sheet_templates.sql` (7) | kurum şablon kitaplığı: `sheet_template.published_from`; `sheet_template_can_publish(tenant)`; `sheet_template_role(şablon, sahip, alan)` kurum koluyla (eski iki girdili sürüm kaldırılır); `sheet_template_audience` ve `sheet_template_audit` yeni rolle; 0013'ün rolü soran bütün politikaları yeniden kurulur; paylaşım politikası yalnız kişisel alan şablonları için; yayımlayanın adı görünürlüğü (`sheet_template_publisher_seen`) | 0013 değiştirilmedi: saklanan bir geliştirme veritabanına uygulanmış olabilir, sqlx değişmiş bir göçü reddeder (bkz. `released_migrations_are_unchanged`) | birleştirmede §4.2: `main` 0013/0014'ü almışsa ikisi birlikte, bu sırayla ilk boş numaralara taşınır |
| G-2 | üretilen: `apps/web/src/contracts/generated/sheet/` (6b, 7) | yeni tipler `NorthInfo`, `DateSource`, `DeclinationSource`, `NorthMissing` (6b); `CloudOrganization`, `OrganizationTemplates`, `SheetTemplatePublish` (7); `TemplateRole` artık `"admin"` da; `SheetTemplateList.organizations`; `SheetTemplateSummary.organization?`, `.publishedFrom?` | ts-rs (`cargo test -p kentos-sheet --features ts`) | `pnpm typecheck` (web'in `templateStore.ts`'indeki `ROLES` dizisi alt küme olduğundan derleme bozulmaz) |
| R-26 | `Cargo.toml` (8) | `[workspace.dependencies]`'e `resvg = { version = "=0.45.1", default-features = false }`, yorumuyla (+4) | sahibin onayı, 3 Ekim (design açık soru 6, “resvg/usvg”): eklenen SVG resminin boyu; Iced'in SVG desteğinin çizdiği sürüm | `cargo metadata --locked` |
| R-27 | `Cargo.lock` (8) | +30 paket: Iced'in `image-without-codecs` ve `svg` özellikleri (sheet-ui) ve `resvg`; listesi ve lisansları `docs/deps`'te | aynı onay; özellikler sheet-ui'nin `Cargo.toml`'ında (bu dalın kendi crate'i) | `cargo metadata --locked`; `node scripts/arch/deps.mjs` |
| R-28 | `docs/deps/README.md` (8, 9) | `iced` satırına iki özellik, yeni `resvg` satırı, “Geçişli bağımlılıklar”a 30 paketin lisanslarıyla dökümü; başlıktaki sayı 40; 9. adımda `resvg` satırına sheet-ui'de adı yazılan `text`, `system-fonts` (kilit değişmedi) | kayıt kuralı (`docs/deps` README'si) | belge |
| R-29 | yeni dosya `crates/render/wgpu/src/styled/cpu.rs` (9) | stilli gölgelendiricilerin CPU ikizi: `CpuView`, `paint`, `paint_layer`; `stroke`/`fill`/`marker`/`shapes.wgsl`'in piksel fonksiyonları satır satır, `style_block` ile; sınamaları (9) | koordinatörün 9. adım notu 3: masaüstünün harita resmi yedeği stilsizdi. Paylaşılan crate'te çünkü gölgelendiriciler ve atlas resimleri orada; dalın crate'i (sheet-ui) Iced'e bağlıdır, tiny-skia'ya değil | `cargo test -p kentos-render-wgpu`; masaüstü `a_map_with_a_pattern_fill…` (eski yolla düşer) |
| R-30 | `crates/render/wgpu/src/styled/{mod.rs, raster.rs, atlas.rs}` (9) | `mod.rs`: `pub mod cpu;` ve belge satırı; `raster.rs`: `pub fn image(image, step, source)`, atlas resmini tek yerden çizer; `atlas.rs`: `paint` onu çağırır (aynı davranış, −22 satır) | atlas ile CPU ikizi aynı resmi çizsin | `cargo test -p kentos-render-wgpu` (atlas sınamaları değişmeden geçer) |
| G-3 | üretilen: `apps/web/src/contracts/generated/sheet/` (9) | yeni tip `PdfSvgSize`; `PdfAsset.raster?: string` | ts-rs | `pnpm typecheck` |
Tablo tamdır (2026-10-03, son hâl): `git diff --name-only`'deki 42 izlenen dosyanın (`Cargo.lock`
hariç) her biri bir satırda geçer. Koordinatör bunu dosya dosya karşılaştırdı. 5. adımdan sonra 41 dosya: yeni yedisi R-20,
R-21, U-5, U-6, D-14, D-16 ve W-14 satırlarındadır (Rust ajanı `git diff --name-only` ile denetledi).
6b., 7. ve 8. adımdan sonra da 41 dosya: bu adımlar izlenen yeni bir dosyaya dokunmadı. Yeni
dosyaları 0014 göçü ve dalın kendi crate'leridir; `Cargo.toml`, `Cargo.lock` ve `docs/deps`
R-26 – R-28'de yeniden yazılı (Rust ajanı denetledi, 3 Ekim). 9. adımdan sonra 43 dosya: yeni
ikisi render-wgpu'nun `styled/mod.rs` ve `styled/atlas.rs`'idir (R-30); yeni `styled/cpu.rs` R-29'da.

## 4. Bağlama adımları

Sahibin `main`'deki son durumuyla:

```sh
git fetch origin
git switch feat/sheet-layouts
git rebase origin/main            # ya da: git merge origin/main
```

1. **Çakışma çıkarsa** yalnız bağlama commit'lerinde (6, 7) çıkar. Dosyaya göre:
   - Düşük riskli dosyalar (§1.2): `main`'in sürümünü al, §3'teki satırları yeniden ekle.
   - Orta riskli dosyalar (§1.2): `main`'in sürümünü al, §3'teki tarifi o sürüme elle uygula,
     satırın “Doğrulama” sütunundaki komutu koş.
   - `Cargo.lock`, `pnpm-lock.yaml`, envanter dosyaları: `main`'in sürümünü al, §5'teki üreticileri
     çalıştır.
   - D-7 (7. commit): `main` ADR 0163'ün masaüstü adımını aldıysa commit'i at
     (`git rebase -i` yerine `git rebase --onto` ya da commit'i `git revert` et). Almadıysa commit
     kalır, `main`'in masaüstü derlemesini de onarır.
2. **Migration numarası:** `main`'de dalın kullandığı numara alındıysa dosyayı ilk boş numaraya
   yeniden adlandır. Dosya kendi içinde bağımsızdır; numarası başka yerde geçmez. Dalın dosyası
   `crates/server/postgres/migrations/0013_sheet_templates.sql`'dir (R-16). Yalnız `main`'in son
   migration'ına dayanır (`kentos.tenant`, `kentos.app_user`, `kentos.audit_event`,
   `kentos.current_user_id()`, `kentos.current_tenant()`); bunların adı `main`'de değiştiyse dosya
   da değişir. Dalın 0013'ünü çalıştırmış bir geliştirme veritabanı, yeniden adlandırmadan sonra
   yeniden kurulur (`sqlx` uygulanmış sürümü numarasıyla tanır). Üretim veritabanında dal hiç
   çalışmadı.
3. **ADR numarası:** `docs/sheet/design.md` → `docs/adr/NNNN-sheet-layouts.md` (ilk boş numara).
   Başlığa numarayı yaz, `docs/sheet/README.md`'deki bağlantıyı güncelle. Diğer belgeler
   `docs/sheet/`'te kalabilir.
4. **TODOS.md:** `CAD-07`, `CAD-08`, `OUT-01`, `OUT-02`, `OUT-03` satırlarına ADR numarasını ve
   yapılanları yaz (design.md “Sonuçlar”).
5. **CLAUDE.md §2:** yerel depolar listesine IndexedDB `kentos.sheets.v1`'i ve masaüstünün
   `~/.local/share/kentos-cad/pafta/` klasörünü ekle (W-9, D-2).
6. Doğrula (§5), sonra `main`'e al:

   ```sh
   git switch main
   git merge --ff-only feat/sheet-layouts
   ```

## 5. Doğrulama

Sırayla. Ağır işler birbirinin ardından çalışır (CLAUDE.md: cargo, tarayıcı ve ölçüm koşuları aynı
anda yapılmaz).

```sh
cargo metadata --locked > /dev/null      # kilit dosyası güncel mi
pnpm install                             # pnpm-lock.yaml
pnpm wasm                                # pafta paketi dahil bütün WASM paketleri
node crates/wasm/sheet-wasm/tests/smoke.mjs   # WASM sonuçları = Rust testlerinin beklediği
pnpm inventory                           # komut/pencere envanteri (yeni sheet.* komutları)
pnpm typecheck && pnpm test && pnpm build
pnpm arch:deps                           # “33 crate”
cargo test -p kentos-sheet -p kentos-sheet-wasm -p kentos-sheet-ui -p kentos-cloud -p kentos-ui
cargo test -p kentos-desktop
cargo clippy -p kentos-sheet -p kentos-sheet-wasm -p kentos-sheet-ui -p kentos-cloud -p kentos-ui \
  -p kentos-desktop -p kentos-api -p kentos-application --all-targets -- -D warnings
# Veritabanlı testler: geçici kap tarifi §7 Rust raporunda ("3." ve "4b."); 5432'ye dokunmaz.
KENTOS_TEST_DB=required cargo test -p kentos-postgres -p kentos-application -p kentos-api -p kentos-cloud
# Görüntüler ve uçtan uca:
node apps/web/scripts/e2e/sheet-shots.mjs     # → apps/web/scripts/e2e/out/shots/sheet/
node apps/web/scripts/e2e/sheet-cloud.mjs     # docker + target/debug/kentosd → …/shots/sheet-cloud/
cargo test -p kentos-sheet-ui --test screens -- --ignored          # → .run/shots/sheet-desktop/
cargo test -p kentos-desktop sheet_comparison_screens -- --ignored # gerçek uygulama, web'in örnek çizimi
```

`pnpm rust:test:desktop` betiği `kentos-sheet-ui`'yi içermez (paket listesi açık yazılıdır); yukarıdaki
`cargo test` satırı onu kapsar. İstenirse birleştirmede betiğe `-p kentos-sheet-ui` eklenir.

## 6. Birleştirmeden sonra `main`'de

Dalda bilerek yapılmayanlar. `main`'in merkezi biçimlerine dokundukları için ayrı adımlardır:

1. **`.kcad` 2.x `sheets` alanı.** Pafta kitabı proje dosyasına girer.
   - `minReaderMinor` yükselir; eski okuyucu açık hata verir.
   - `fixtures/kcad/v2`'ye geçerli ve bozuk örnekler eklenir.
   - Web ve masaüstü belgesi kitabı taşır.
   - 1. aşamanın yerel deposundan (IndexedDB, uygulama veri klasörü) tek seferlik taşıma yapılır.
2. **Veritabanı projeleri:** sunucuda proje başına pafta kitabı (revizyonlu, `expectedRevision`).
3. **Ürün komutu kataloğu:** pafta işlemleri `sheet.*` komutları olarak Python ve MCP'ye açılır
   (ADR 0013).
4. **PDF ve GeoPDF, yazı tipi gömme, manyetik model, kurum şablonları, e-postayla şablon daveti:**
   design.md “Açık sorular”.
5. **Masaüstünün web'den geri kalan üç yeri:** sekmede “Yeni sürüm var” işareti; “Kullan”da şablonun
   sorularını sormak; resimlerin büyütülünce yumuşatılması ve SVG resim (sahibin kararı: §7 Rust
   4b “Açık kararlar”, `image` paketi seçenekleri).
6. **Sunucu:** şablon olayları uzun yoklamayla gelir, WebSocket'e taşınabilir (sapma 30). Beş
   `sheet.template.*` komutu ürün kataloğu dosyasına yazılmalı (sapma 33).
7. **Pafta özellikleri:** atlas düzenleyicisi; ana sayfanın öğelerini yerinde düzenleme; başka
   koordinat sisteminde karelaj; derece-dakika-saniye yazıları; kerning; şablonun yeni revizyonunu
   var olan paftaya uygulama.
8. **Uzman incelemesi:** sistem şablonları bir harita mühendisi ve bir şehir plancısı tarafından
   incelenmeli; mevzuata bağlanacak olanlar veri sürümü olarak gelmeli.

### `main`'de bulunan, bu dalla ilgisi olmayan sorunlar

| Sorun | Neden | Önerilen düzeltme |
|---|---|---|
| Şeridin büyük düğmelerinde Ö, Ü noktaları ve İ'nin noktası kırpılıyor (“Olçülendirme”, “Otele”) | `apps/web/src/styles/ribbon.css` `.rbtn--large .rbtn__label`: `line-height: 1.18` + `overflow: hidden` (12 px'te satır kutusu 14,16 px, noktalar 15 px'e çıkıyor) | `.rbtn--large .rbtn__label { padding-top: 0.125em; margin-top: -0.125em; }` (1× ve 2×'te denendi; karşılaştırma: `apps/web/scripts/e2e/out/shots/sheet/zz-serit-o-kirpmasi-main-ve-duzeltme.png`) |
| `main`'in `e21b094f`'inde masaüstü derlenmiyor | ADR 0163 1. adım çekirdeğe dört `SnapKind` ekledi; `drawing_menus.rs` ve `marks.rs` eşleşmeleri eksik | ADR 0163'ün masaüstü adımı; o gelene dek bu dalın 7. commit'i (D-7) |
| Masaüstünde pafta kipi dışındaki pencereler ondalığı virgülle yazıyor | ADR 0149 kural 5 “ondalık ayırıcı noktadır” diyor; `kentos-ui`'nin varsayılanı virgül | `kentos-ui`'nin nokta kuralını (U-1) varsayılan yapmak, ayrı bir karar |
| Web'de sekmeli bir panelin daraltma düğmesinin erişilebilir adı yok (sağ dok: Katmanlar, İşlemler, Bloklar) | `apps/web/src/ui/dock/Panel.ts`: sekmeler gösterilince başlık gizleniyor, düğmenin adı başlıktan geliyordu | düğmeye panelin başlığını `aria-label` olarak vermek (tek satır); `apps/web/scripts/e2e/sheet-a11y.mjs` bulguyu gösterir |

## 7. Ajan raporları

Her ajan işini bitirince bu bölüme kısa bir rapor ekler:

- yaptıkları;
- dokunduğu bağlantı noktaları (§3'e de yazılır);
- çalıştırdığı komutlar ve sonuçları;
- sapmalar ve nedenleri;
- yapılmayanlar.

### Rust

#### Son hâl (3 Ekim, 9. adımdan sonra): ne var, nerede, nasıl kanıtlandı

**Ne var, nerede.**

| Parça | Yer | İçerik |
|---|---|---|
| Çekirdek `kentos-sheet` | `crates/shared/sheet` (makine + wasm32) | model, 41 işlem ve tersleri, kısıtlarla yerleşim, yapışma ve isabet, yazı ölçüleri, `[% … %]` ifadeleri, 13 öğe türünün çizim planı, SVG yazıcısı, 10 sistem şablonu ve kip profilleri, ön denetim, eşitleme planı, `.kpafta`, PDF ve GeoPDF (`pdf/`; SVG resim ev sahibinin PNG'siyle), WMM2025 (`wmm.rs`, `data/wmm2025.json`) ve kuzey bilgisi (`display::north_info`), bulut tipleri (`cloud.rs`: kişisel, paylaşılan, kurum şablonu) |
| WASM | `crates/wasm/sheet-wasm` → `apps/web/src/product/sheet/pkg` | çekirdeğin her işlevi zarflı JSON ile (`toPdf` bayt döner); TS tipleri `apps/web/src/contracts/generated/sheet/` |
| Masaüstü pafta kipi | `crates/sheet-ui` | tasarımcı, cetvel ve kılavuzlar, denetçi (kuzey okunun sapma bölümü dahil), öğe ağacı, galeri (Sistem, Benim, Kurumum, Benimle paylaşılanlar; Paylaş…, Kuruma yayımla…), şablon olarak kaydet, sorular, değişkenler, ön denetim, resimler (doku, SVG), dışa aktarma (PNG, SVG, PDF, `.kpafta`, Yazdır) |
| Masaüstü bağlantısı | `apps/desktop/src/{sheets.rs, map_vectors.rs, sheet_pdf.rs, sheet_library.rs, sheet_inputs.rs, drawing_fonts.rs}` | haritaların boyacısı (vektör ya da stilli resim), PDF ev sahibi, bulut kitaplığı ve eşitleme, çizim yüzleri |
| Stilli gölgelendiricilerin CPU ikizi | `crates/render/wgpu/src/styled/cpu.rs` | vektör biçimi olmayan haritanın resmi, ekranda ve PDF'te |
| Sunucu | göç `0013_sheet_templates.sql`, `0014_org_sheet_templates.sql`; `crates/server/application/src/sheet_templates.rs`; `apps/api/src/http/sheet_templates.rs` | şablon kitaplığı: komutlar (`create`, `update`, `delete`, `share`, `unshare`, `publish`), liste, olaylar, satır güvenliği |
| Bulut istemcisi | `crates/native/cloud/src/{sheet_templates.rs, sheet_library.rs}` | uçlar, eşitleme planı ve koşusu, uzun yoklama |
| Veri ve betikler | `scripts/fonts/sheet_metrics.py`, `scripts/geodesy/wmm_coefficients.py`, `fixtures/sheet/v1/` | ölçü tablosu, WMM katsayıları, iki platformun fixture'ları |
| Web'e notlar | `.run/sheet-engine-ready` | her uç, kod ve kullanıcı sözcüğü |

**Nasıl kanıtlandı** (son koşu, 3 Ekim, ağır işler `.run/heavy.lock` altında):

| Ne | Komut | Sonuç |
|---|---|---|
| Çekirdek | `cargo test -p kentos-sheet` | birim 361; fixtures 12, ops 7, pdf 8 (+1 elle), templates 8 (+3 elle), wmm 8 |
| 100 kâğıt matrisi | `cargo test -p kentos-sheet --test templates every_template_on_every_paper` | 2/2: on şablon, A4…A0, iki yön; düz ve kuzey çizelgeli+manyetik; bulgusuz |
| WASM | `cargo test -p kentos-sheet-wasm`; `pnpm wasm` + `node crates/wasm/sheet-wasm/tests/smoke.mjs` | 4; bütün aileler Rust'ın beklediği: ops 61, ops (hata) 22, display 2, sync 11, relayout 7, snap 14, hit 13, atlas 4, preflight 21, templates 14, kpafta 12, profiles 9, pdf 9, wmm 117, glyphs 12 |
| Altın PDF | `tests/pdf.rs`, `smoke.mjs` | `6c194fa0…9234`, 40 064 bayt; WASM aynı baytları verir |
| Masaüstü pafta kipi | `cargo test -p kentos-sheet-ui` | birim 42, designer 27, library 7 |
| CPU ikizi ve çizici | `cargo test -p kentos-render-wgpu` | 57 (`styled::cpu` 9) |
| Masaüstü | `cargo test -p kentos-desktop` | 690 (88 elle koşulan atlandı) |
| Sunucu ve bulut | geçici kapta, `KENTOS_TEST_DB=required`: `cargo test -p kentos-postgres -p kentos-application -p kentos-api -p kentos-cloud` | postgres 3; application birim 34 + 20 dosyada 77 (`sheet_templates` 7, ADR 0024 düzeyinde negatifler dahil); api 42 (+1 eski, atlanan); cloud 81 + 1 |
| Canlı masaüstü | gerçek `kentosd serve`, iki hesap, iki cihaz | 4b: paylaşım 48 ms'de, ikinci cihazın kaydı 39 ms'de duyuldu; 7: kurum yayımı 29 ms'de duyuldu |
| Görüntüler | `sheet_screens`, `sheet_step5/6/8/9_screens`, `sheet_comparison_screens`, canlı iki test; çekirdeğin `template_shots`, `north_diagram_shots`, `template_matrix_shots` | hepsi geçti; `.run/shots/sheet-desktop/`, `.run/shots/sheet/` |
| PDF kabulü | `gdalinfo`, `pdffonts`, `pdftotext`, `pdfinfo`, `pdftoppm` (5. adım; 9. adımda `pdftoppm` kesitleri) | TM33/TM36 köşeleri 0,025 mm içinde; web ile 23/23 denetim, 99/99 sözcük |
| Kod denetimi | `cargo clippy` dokunulan 9 crate `--all-targets -- -D warnings`, `kentos-sheet` ayrıca `--features ts,schema` | temiz |
| Bağımlılık | `pnpm arch:deps`, `cargo metadata --locked` | 33 crate, 42 crate × hedef temiz; kilit 687 paket |

Veritabanı: geçici kap `kentos-sheet-testdb-9` (`127.0.0.1:55442`), sunucu `55443`. Sonunda ikisi
de durduruldu, kap silindi, portlar boş. 5432'deki `database-postgis-1`'e hiç dokunulmadı.

**Bilinen sınırlar** (tasarım gereği, açık iş değil):

- CPU ikizi sRGB'de karıştırır, GPU doğrusal ışıkta: yalnız yumuşak kenarlarda ayrışır.
- Katmanlar arası sıra PDF vektörlerininki gibidir, sembol düzeyi sırası değil (sapma 120).
- Web'in yapacakları `.run/sheet-engine-ready`'de ve 6b/7/9. adımın raporlarında:
  - KURUM: `TemplateCloudState` ve `ROLES` (7);
  - SVG'yi tarayıcıda PNG'ye çizmek (9);
  - `'` arayan sınamalar (6b).

**Adım adım raporlar.** 1., 1b., 2., 3., 4., 4b., 5., 6., 6b., 7., 8. ve 9. adım tamam (2–3
Ekim). `.run/sheet-engine-ready` yazıldı (bulut uçları, komutları, `.kpafta` kodeki, PDF, WMM ve
KURUM dahil). 4b'nin raporu 4. adımınkinin ardında; 5.–9. adımın raporları bu bölümün sonunda,
kendi başlıklarında. **Dikkat:** dalın HEAD'i (e21b094f, ADR 0163 1. adım)
masaüstünü derlenmez bırakıyor; masaüstü bağlantısı geçici bir köprüyle (§3 D-7) derlenip sınandı.

**Yapılanlar.**

- `crates/shared/sheet` (`kentos-sheet`, `shared` grubu, makine + wasm32): ~15 200 satır kaynak
  (birim testleri içinde), 1 300 satır bütünleşik test.
  - Model, doğrulama ve normalleştirme (bilinmeyen alan hatadır), 41 işlem ve tersleri, kısıtlarla
    yeniden yerleşim, ana sayfa, varlık üst verisi, ƒ bağları.
  - Yapışma (`SnapSession`: öğe, kılavuz, sayfa, kenar boşluğu, ızgara, eşit aralık, mesafe
    rozetleri, boyutlandırma ve boy eşleme, dönüş), isabet (döndürülmüş kutu, çizgi yolu,
    tutamaçlar).
  - Yazı ölçüleri: `scripts/fonts/sheet_metrics.py` → `data/font-metrics.json` (22 yüz, 349
    karakter, `--check`); satır kırma, hizalama, `ShrinkToFit`.
  - İfadeler `kentos-expression` ile; `[% … %]`, hazır değişkenler, eksik değer işareti (5. adımdan
    beri `‹ad?›`; önce `⟨ad?⟩` idi).
  - Çizim planı: 13 öğe türünün hepsi; karelaj (artı, çizgi, çentik, yalnız çerçeve; zebra ve
    çentikli çerçeve; Y/X yazıları, yazı bandı), 1-2-5 ölçek çubuğu, ADR 0110'un `KentosK` oku,
    grid / coğrafi / manyetik kuzey ve yakınsama notu, lejant, tablo (devam çerçevesi), koordinat
    listesi, antet, pafta çerçevesi (bölge işaretleri, ortalama işaretleri).
  - SVG yazıcısı (sabit sayı biçimi, µm kullanıcı birimi, haritasız çerçevede gri yer tutucu).
  - Şablonlar: biçim, `validate`, `migrate`, `instantiate`, `extract`; **10 sistem şablonu**.
  - Kip profilleri veri olarak (`data/profiles.json`): `profile_for`, `tool_availability`,
    `rank_templates`, `new_item`, `item_note`.
  - Eşitleme planı, ön denetim (düzeltmeleriyle), atlas planı, kâğıt boyları ve standart ölçekler
    (veri), `cloud` DTO'ları, `EngineInfo`.
- `crates/wasm/sheet-wasm` (`kentos-sheet-wasm`): JS API (aşağıda), cevaplar JSON zarfı. Çıktı
  `apps/web/src/product/sheet/pkg` (3,3 MB; gzip -9 ile 0,94 MB).
- TS tipleri: `apps/web/src/contracts/generated/sheet/` (262 dosya, ts-rs; `cargo test -p
  kentos-sheet` yazar).
- Fixture'lar: `fixtures/sheet/v1/` (39 dosya, `README.md` biçimi ve beklenenlerin kaynağını
  anlatır): `ops`, `relayout`, `snap`, `hit`, `display`, `svg`, `templates/{valid,invalid}`,
  `sync`, `preflight`, `atlas`, `profiles`. Kayıt olmayan ailelerin beklenenleri elle (ya da
  çekirdekten bağımsız bir hesapla) yazıldı ve ilk koşuda tuttu.
- Node duman koşusu: `crates/wasm/sheet-wasm/tests/smoke.mjs`.

**JS API** (`apps/web/src/product/sheet/pkg/kentos_sheet_wasm.js`; girdiler ve `value`
`contracts/generated/sheet/` tipleri):

| İşlev | Döndürdüğü `value` |
|---|---|
| `engineInfo()`, `paperSizes()`, `standardScales()`, `bindableProperties()` | `EngineInfo`, `PaperSize[]`, `number[]`, `Bindable[]` |
| `readBook(book)`, `bookDigest(book)` | `SheetBook` (denetlenmiş, normalleştirilmiş), SHA-256 |
| `systemTemplates()`, `validateTemplate(t)`, `instantiateTemplate(t, ids, options)`, `extractTemplate(book, sheet, meta, assets)` | `Template[]`, `Template`, `Instance`, `Template` |
| `applyOp(book, op)`, `applyOps(book, ops)` | `Applied` (`book`, `inverse`, `label`) |
| `hitTest(book, sheet, query)` | `Hits` |
| `new SnapSession(book, sheet, moving, options)`: `query(dx, dy, tol)`, `queryResize(handle, x, y, tol, keepAspect)`, `movingBox()`, `free()` | `SnapResult`, `ResizeSnap`, `RectUm` |
| `snapRotation(angle, step)` | sayı (doğrudan) |
| `displayList(book, sheet, inputs)`, `toSvg(list, options)` | `DisplayList`, SVG metni |
| `preflight(book, sheet, inputs)`, `atlasPlan(book, sheet, features)`, `checkExpression(src)` | `Finding[]`, `AtlasPlan`, `null` |
| `planSync(local, remote)` | `SyncPlan` |
| `profileFor(ws, caps)`, `toolAvailability(ws, caps)`, `rankTemplates(metas, ws, projectType)`, `newItem(ws, caps, req)`, `itemNote(ws, item)` | `Profile`, `ToolInfo[]`, `RankedTemplate[]`, `Item`, `string \| null` |

Her işlev bir JSON metni döndürür: `{"ok":true,"value":…}` ya da
`{"ok":false,"error":{"code","message","path"?}}`. `ws` JSON'dur (`"cad"` ya da `null`). Yalnız
`SnapSession` kurucusu fırlatır (`code: message`).

**Bağlantı noktaları:** R-1 (`Cargo.toml` üyeleri), R-2 (`Cargo.lock`, +25 satır, cargo yazdı),
R-4 (`package.json` `rust:wasm:sheet`), R-5 (`scripts/wasm/ensure.mjs`), R-6 (`.gitignore`, yeni
satır). R-3 6. adımındır, dokunulmadı.

**Komutlar** (ağır olanlar `flock .run/heavy.lock` ile; son hâlde):

| Komut | Sonuç |
|---|---|
| `cargo test -p kentos-sheet -p kentos-sheet-wasm` | kentos-sheet: birim 297, `fixtures` 10, `ops` 6, `templates` 6 geçti (1 yok sayılan: `template_shots`); kentos-sheet-wasm: 3 geçti; başarısız yok |
| `cargo clippy -p kentos-sheet -p kentos-sheet-wasm --all-targets -- -D warnings` | temiz (ilk koşularda 19 bulgu çıktı: `large_enum_variant`, `too_many_arguments`, `type_complexity` …; hepsi düzeltildi, susturma yok) |
| `cargo build -p kentos-sheet --target wasm32-unknown-unknown` | temiz |
| `pnpm arch:deps` | “Bağımlılık yönü temiz: 32 crate, 41 crate × hedef denetlendi.” |
| `python3 scripts/fonts/sheet_metrics.py --check` | “Pafta yazı ölçüleri güncel: 22 yüz, 349 karakter.” |
| `cargo metadata --locked` | temiz |
| `pnpm wasm` | yalnız “Pafta çekirdeği” derlendi (39 s), öbür paketler güncel |
| `node crates/wasm/sheet-wasm/tests/smoke.mjs` | ops 58 + 18 hata durumu (kitabın özeti, ters işlem, etiket; ters işlem kitabı geri verir), display 2, sync 11, relayout 4, snap 14, hit 13, atlas 4, preflight 21 bulgu, templates 14, profiles 9: hepsi Rust'ın beklediğiyle aynı |
| `cargo test -p kentos-sheet --test templates template_shots -- --ignored` + başsız Chrome | 40 PNG |
| Başarım (Node, depoya girmeyen bir ölçüm betiği) | 200 öğede yapışma sorgusu ortalama 8 µs (hedef ≤ 0,5 ms), oturum açma 6,7 ms; A0, 50 öğe, 2 karelaj çizim planı 1,27 ms, JSON çözme dahil (hedef ≤ 5 ms) |

**Görüntüler:** `.run/shots/sheet/templates/<id>.png` (örnek cevaplar ve yapay harita resmiyle),
`<id>-bos.png` (boş şablon), iki başka kâğıtta `<id>-<kâğıt>-<yön>.png`: 10 şablon × 4 = 40 PNG.

**Sapmalar:** `tasks-rust.md` “Sapmalar” 1–23.

**1b. Yerleşim düzenleri (2 Ekim akşamı, koordinatörün isteği; design §3.2a).**

- Model: `Sheet.variants`, `Master.variants` (`LayoutVariant { id, name, when, reference, frames }`),
  `activeVariant`, `baseLayout`. `crates/shared/sheet/src/variants.rs`: seçim, geçiş (her öğe onu
  anan düzenden, anılmayan temel düzenden), düzene yazma, kimlik eşleme. `setPage` ve `instantiate`
  düzeni seçer; ana sayfa her paftanın kâğıdına kendi düzeniyle çizilir.
- İşlemler: `addVariant`, `removeVariant`, `setVariant`. Düzeni olan paftada her işlemin tersi
  paftanın önceki hâlidir; kesinliği `tests/ops.rs` (83 durum, 22'si hata) ve
  `a_variant_keeps_its_own_arrangement` sınar: yatay kâğıda dönüşte kitap bayt bayt aynı, dikeyde
  yapılan düzeltme dikeyde kalır.
- WASM: `variantFor(book, owner, page)`; işlemler `applyOp` ile. TS tipleri yeniden üretildi (269).
- Sistem şablonlarının hepsinde yatay ve dikey düzen var; A4/A3 için boyla sınırlı düzenler.
  CAD teknik dikey A4: ISO 7200 anteti altta çerçevenin tam genişliğinde, revizyon tablosu üstünde,
  notlar, görünüm penceresi. Genel A3 yatay dikey: harita üstte tam genişlikte, altta başlık ve
  lejant, kuzey, ölçek ve antet.
- Kabul testi `every_template_on_every_paper_both_ways`: 10 şablon × A4, A3, A2, A1, A0 × iki yön
  = 100 durum, örnek cevap ve verilerle. Bulgu yok: hata yok, uyarı da yok (çakışma, taşma, sayfa
  ya da kenar boşluğu dışı, lejant ve tablo taşması dahil).
- Görüntüler: `.run/shots/sheet/templates/matrix/<şablon>-<kâğıt>-<dikey|yatay>.png` (100) ve
  şablon başına bir kontak sayfası `matrix/<şablon>-kontakt.png` (10; her karenin üstünde geçerli
  düzenin adı).
- Komutlar (son hâl): `cargo test -p kentos-sheet -p kentos-sheet-wasm` (birim 308, fixtures 10,
  ops 7, templates 7 + 2 yok sayılan, wasm 3; başarısız yok); clippy `-D warnings` temiz;
  `pnpm wasm` temiz; `node crates/wasm/sheet-wasm/tests/smoke.mjs`: ops 61 + 22 hata, display 2,
  sync 11, relayout 7, snap 14, hit 13, atlas 4, preflight 21, templates 14, profiles 9, hepsi aynı.
- Sapmalar: `tasks-rust.md` 24–29.
- Zayıf yerler: küçük kâğıtta bazı bloklar gizli (sapma 29); büyük kâğıtlarda bant öğeleri
  orantıyla genişler, yükseklikleri sabittir (A0 dikeyde lejant geniş bir şeritte kalır); düzene
  yazılan çerçeve kısıtların tersiyle geri taşındığı için `scale` kısıtında 1 µm'lik yuvarlama
  olabilir.

**3. Sunucu ve `kentos-cloud` (design §13).**

- Migration `crates/server/postgres/migrations/0013_sheet_templates.sql` (R-16, numara notu §4.2):
  `sheet_template`, değişmez `sheet_template_revision`, `sheet_template_grant` (viewer, editor),
  kişinin komut günlüğü `sheet_template_command`, alıcı başına olay `sheet_template_event`. Rolü tek
  yer hesaplar (`kentos.sheet_template_role`); her tabloda satır güvenliği ona dayanır, yani
  sunucunun kodundaki bir hata kimsenin şablonunu açmaz. Denetim kaydı ve olayın alıcıları
  `security definer` işlevlerle (`sheet_template_audit`, `sheet_template_audience`).
- `crates/server/application/src/sheet_templates.rs`: beş komut (`sheet.template.create`, `update`,
  `delete`, `share`, `unshare`, v1) ve okumalar (`mine`, `detail`, `access_list`, `candidates`,
  `events_after`). İçerik cihazın okuduğu gibi okunur ve denetlenir (`read_template`); sunucu kimliği,
  revizyonu, tarihleri yazar, SHA-256'yı hesaplar. Yazan komut şablonu kilitler (görüntüleyici
  kilitlemez, 403 alır). Bir kişinin oluşturmaları sırayla geçer: aynı anda iki istek 500 sınırını
  birlikte aşamaz (test, kilit kaldırılınca düşüyor; denendi). Kişi araması ADR 0024'ün kurallarıyla
  (`people::search_patterns`, sahibin etkin ve koltuklu üyesi olduğu kurumlar).
- `apps/api/src/http/sheet_templates.rs`: `GET /v1/me/sheet-templates`, `GET /v1/sheet-templates/{id}`
  (`ETag` revizyon, 304), `…/access`, `…/access/candidates?q=`, `GET /v1/me/sheet-templates/events`
  (uzun yoklama, en çok 25 sn; bu süreçteki şablon komutu bekleyeni hemen uyandırır). Bozuk kimlik,
  var olmayan ve görülmeyen şablon aynı 404 gövdesini alır. Komutlar kişisel alanın var olan
  `POST /v1/tenants/{alan}/commands` ucuna gelir (R-9, R-12).
- `crates/native/cloud/src/sheet_templates.rs`: masaüstünün istemcisi (`list`, `detail`, `access`,
  `candidates`, `events` (uzun yoklama), `personal_space`, `create`, `update`, `delete`, `share`,
  `unshare`, `remote` → `plan_sync`), ADR 0040'ın kalıbında (bulutun kendi çalışma zamanında,
  bırakılınca durur).
- `kentos-sheet` `cloud`: olay tipleri (`SheetTemplateEvent`, `…Kind`, `…EventPage`), `schema`
  özelliğiyle schemars türetmeleri ve `catalog_entries()` (beş komutun katalog girdisi). TS tipleri
  yeniden üretildi (272 dosya).
- Testler:
  - `crates/server/application/tests/sheet_templates.rs` (5): katalog = sunucunun komutları;
    kaydet, listele, oku, revizyon, 409 ve hiçbir şey yazmaması, idempotency (yineleme ve anahtarın
    başka istekte kullanılması); ADR 0024 düzeyinde redler (başkasının şablonu var olmayanla harfi
    harfine aynı 404; sahip olmayan paylaşamaz, silemez, paylaşımı kaldıramaz: 403; görüntüleyici
    yazamaz; kimse kendi rolünü yükseltemez; kendine paylaşım yok; kurum dışına, kurumsuz kişiye ve
    var olmayan hesaba paylaşım aynı iletiyle reddedilir); paylaşım listesi ve kişi araması yalnız
    sahibin; paylaşımın kaldırılması listeden düşürür ve o kişiye söylenir; olaylar alıcı başına;
    satır güvenliği (başka kişinin kapsamında dört tablo boş); silme ve listelerden düşme;
    sınırlar (ad, 8 MB, 500 ve aynı anda iki oluşturma), kurumun ucu, sürüm, kısa anahtar.
  - `apps/api/src/http/sheet_templates_tests.rs` (2): uçlar uçtan uca (201, ETag/304, aynı 404
    gövdeleri, 403, 409 ve `revision`, 422 ve `path`, uzun yoklama) ve `kentos-cloud` gerçek soket
    üzerinden (bekleyen uzun yoklama silmeyi 3 sn içinde duyar).
- Veritabanı: depoda `.env.local` yok; kullanıcının 5432'deki PostGIS'ine dokunulmadı. Testler
  geçici bir kapta koştu, sonra kap durduruldu ve silindi. Aynısını yeniden koşmak için:

  ```sh
  docker run -d --rm --name kentos-sheet-testdb -p 127.0.0.1:55439:5432 \
    -e POSTGRES_PASSWORD=<yönetici> postgis/postgis:18-3.6
  docker exec kentos-sheet-testdb psql -U postgres \
    -c "create role kentos_cad_owner login nosuperuser nocreatedb nocreaterole password '<sahip>'" \
    -c "create role kentos_cad_app login nosuperuser nocreatedb nocreaterole nobypassrls noinherit password '<uygulama>'"
  export KENTOS_TEST_ADMIN_URL=postgres://postgres:<yönetici>@127.0.0.1:55439/postgres
  export KENTOS_DATABASE_URL=postgres://kentos_cad_app:<uygulama>@127.0.0.1:55439/kentos_cad
  export KENTOS_DATABASE_OWNER_URL=postgres://kentos_cad_owner:<sahip>@127.0.0.1:55439/kentos_cad
  export KENTOS_TEST_DB=required
  flock .run/heavy.lock cargo test -p kentos-postgres -p kentos-application -p kentos-api -p kentos-cloud
  docker stop kentos-sheet-testdb
  ```
- Komutlar (son hâl, `KENTOS_TEST_DB=required` ile):

  | Komut | Sonuç |
  |---|---|
  | `cargo test -p kentos-postgres -p kentos-application -p kentos-api -p kentos-cloud` | postgres 3; application birim 34 + 20 test dosyasında 75 (yeni `sheet_templates` 5); api 40 (+1 yok sayılan ölçüm, eski); cloud birim 77 (yeni 1) + `download_resume` 1 (+1 yok sayılan ölçüm, eski); başarısız yok |
  | `cargo test -p kentos-sheet -p kentos-sheet-wasm` | birim 311, fixtures 10, ops 7, templates 7 (+2 yok sayılan), wasm 3 |
  | `cargo clippy -p kentos-postgres -p kentos-application -p kentos-api -p kentos-cloud -p kentos-sheet -p kentos-sheet-wasm --all-targets -- -D warnings` ve `-p kentos-sheet --features schema` | temiz |
  | `cargo check -p kentos-mcp` | temiz (`kentos-cloud`'a bağlı) |
  | `pnpm arch:deps` | “Bağımlılık yönü temiz: 32 crate, 41 crate × hedef denetlendi.” |
  | `cargo metadata --locked` | temiz; `Cargo.lock`'a yalnız çalışma alanının crate'leri girdi, dış paket yok |
  | `rustfmt --check` | 3. adımın yeni dosyaları biçimli; paylaşılan dosyalardaki satırlarım da (o dosyalardaki eski farklar benim değil) |

- Bağlantı noktaları: R-7 … R-16 (§3). Sapmalar: `tasks-rust.md` 30–38.
- Web için: uçlar, komut zarfı ve sınırlar `.run/sheet-engine-ready`'de “BULUT ŞABLON KİTAPLIĞI”
  başlığında. Web'in `api.ts`'inde kişisel alanın komut ucunu çağıran bir yöntem yok (`command`
  proje yolunu kurar); web ajanı ekler.
- Zayıf yerler: beş komut ürün kataloğu dosyasında değil (sapma 33); olay WebSocket'ten gelmez
  (sapma 30); başka bir sunucu sürecinin komutu bekleyen yoklamayı en geç 5 sn sonra uyandırır.

**4. Masaüstü (`kentos-sheet-ui`, design §11) ve tek `.kpafta` kodeki.**

- `crates/sheet-ui` (`kentos-sheet-ui`, yeni `sheet-ui` grubu, R-3; yalnız `shared` ve `ui`
  gruplarını kullanır). KentOS UI bileşenleriyle web'in yapısı:
  - Sahne: `Rulers` (mm cetvelleri, cetvelden çekilen kılavuzlar paftanın `AddGuide`/`MoveGuide`/
    `RemoveGuide` işlemleri), masada beyaz kâğıt ve gölgesi, kenar boşlukları ve yakalama ızgarası
    (yardımlar, basılmaz). Çizim planı Iced tuvaline boyanır (`paint.rs`: yollar çeyrek piksel
    duyarlıkla düzlenir, yazılar çekirdeğin ölçüleriyle taban çizgisine oturur, haleli yazı,
    döndürülmüş yazı). Plan haritalarda kesilir: katmanlar ve haritalar çizim sırasıyla üst üste
    durur, her harita kendi tuvalinde ve önbelleğinde (kâğıt değişince harita yeniden boyanmaz).
  - İşaretçi katmanı: tıkla seç (Shift ekler/çıkarır, Ctrl grubun içine), sürükle taşı, tutamaçla
    boyutlandır ve döndür, boş yerden kutuyla seç (sağa: içinde kalanlar, sola: değenler), araçla
    yeni öğe çiz (tıklamada türün boyu); sürüklerken çekirdeğin `SnapSession`'ı: yapışma çizgileri,
    uzaklık rozetleri, eşit aralık işaretleri; sürükleme boyunca kitabın sonraki hâli canlı çizilir
    (kısıtlar, yazılar, karelaj izler). Tekerlek imlecin altında yakınlaştırır, orta tuş kaydırır.
  - Sol panel: paftalar (sağ tık: aç, çoğalt, sil) ve öğe ağacı (`TreeView`: üstteki en üstte,
    grubun öğeleri altında; göz, kilit; sürükleyerek sıra; sağ tık menüsü).
  - Denetçi (`Segmented`: Öğe · Sayfa · Ön denetim): Konum ve boyut (`NumberInput`, ortak olmayan
    değer “—” ve `EditCell`), **Kısıtlar** (kare çizimli düzenleyici: kenar ve orta pimleri, Shift ile
    iki yan; Yatay, Düşey, Göre listeleri), türün kendi özellikleri (KentOS `Inspector`; harita,
    metin, ölçek çubuğu, kuzey oku, lejant, resim, şekil, çizgi, tablo, koordinat listesi, çerçeve),
    Görünüş (dolgu, saydamsızlık, iç boşluk, yazdırılır), Veri (ƒ bağları, okunur). Sayfa: kâğıt,
    yön, ölçüler, kenar boşlukları, **geçerli yerleşim düzeni** ve düzenlerin koşulları, ana sayfa,
    ızgara, kılavuz sayısı. Ön denetim: bulgular, “Göster” ve çözümleri.
  - Şerit: bağlamsal Pafta sekmesinin grupları ev sahibinin mesaj tipinde (Pafta: Yeni ▾ (Yeni
    pafta, Çoğalt, .kpafta dosyasından…), Şablondan, Sayfa ayarları, Değişkenler, Modele dön;
    kipin profilinden Araçlar, Ekle (kipin adlarıyla, hazır biçimli araçlar ▾ menüsüyle; kapalı
    aracın ipucu nedenini söyler), Harita, Düzen (Hizala ▾, Dağıt ▾, Sıra ▾, Grupla, Grubu çöz);
    Görünüm; Çıktı (Ön denetim, Dışa aktar ▾ SVG/PNG/.kpafta, Şablon olarak kaydet)).
  - Pencereler (`Dialog`, `overlay::modal`): şablon galerisi (kaynaklar, arama, kâğıt süzgeci,
    “Bütün kiplerin şablonları”, her şablonun kendi çizim planından küçük resmi, rozetler, ayrıntı,
    kullanılacak kâğıt ve yön, Kullan, Çoğalt, Sil), Şablon olarak kaydet, Değişkenler (paftanın ve
    projenin; ön denetimin bulduğu eksik adları ekler), dışa aktarma (PNG dpi'si; hata varken
    “Yine de aktar”), `.kpafta`'yı yanına ya da yerine alma sorusu.
  - Sekmeler (`Tabs::bottom`: “Model | Pafta 1 | +”), durum hücreleri (imleç Sol/Üst mm, seçimin
    boyu, yakınlaştırma, sayfa n/N, ön denetim), tuşlar (V, H, M, T, L; oklar 1/10/0,1 mm;
    Ctrl+Z/Y, G, Shift+G, D, ], [, 0, 1, A; Delete; Esc).
  - Kendi geri alma yığını (her değişiklik çekirdeğin işlemi ve tersi, 200 adım).
  - Saklama (`store.rs`): uygulama veri klasöründe proje anahtarıyla kitaplar, bu cihazın
    şablonları, resimler; web'in `projectKeyOf` ve `keyChange` kuralları.
  - Dışa aktarma: SVG (çekirdeğin yazıcısı, haritalar boyayıcının PNG'si, resimler kendi
    baytları), PNG (şeritlerle, dpi'li), `.kpafta`.
  - `MapPainter` (`painter.rs`): ev sahibi haritanın içini tuvale boyar; `DemoMaps` testlerin ve
    görüntülerin uydurma kasabası.
- **`.kpafta` kodeki çekirdekte** (koordinatörün isteği): `crates/shared/sheet/src/kpafta.rs`
  (`encode`, `decode`, `KpaftaFile`; biçim ve sürüm, kitap `read_book` gibi, resimler bilgilerine
  tutulur, yarım okuma yok), WASM `encodeKpafta`, `decodeKpafta`, TS tipi `KpaftaFile` (273 dosya),
  fixture'lar `fixtures/sheet/v1/kpafta/{valid,invalid}` (2 + 10, elle), `smoke.mjs` “kpafta 12”.
  `kentos-sheet-ui` bu kodeki kullanır; kendi sabitleri silindi.
- `apps/desktop` bağlantısı (§3 D-1 … D-6): pafta öndeyken şerit ve bağlamsal Pafta sekmesi,
  çizim alanı ile panellerin yerinde pafta kipi, altta sekmeler, durum çubuğunda paftanın
  hücreleri, pencereleri; tuşlar önce pafta kipine, yazı tuşları gizli çizime gitmez; projenin
  değerleri (ad, kullanıcı, bugün, koordinat sistemi, öznitelik, çizim ölçeği, çizim alanının
  merkezi) kipe verilir; haritaların içi `SheetMaps`: kentos-render-wgpu'nun sahnesi (çizim
  alanıyla aynı katmanlar, renkler ve eğriler, kâğıdın paletiyle) tuvale boyanır; dışa aktarma ve
  `.kpafta` için sistemin dosya pencereleri (`rfd`).
- Komutlar (son hâl):

  | Komut | Sonuç |
  |---|---|
  | `cargo test -p kentos-sheet -p kentos-sheet-wasm` | birim 313, fixtures 11, ops 7, templates 7 (+2 yok sayılan), wasm 4 |
  | `cargo test -p kentos-ui -p kentos-ui-showcase -p kentos-render-wgpu -p kentos-desktop -p kentos-sheet-ui` (`pnpm rust:test:desktop` kalıbı ve yeni crate) | kentos-sheet-ui birim 16, `designer` 15, `screens` 1 yok sayılan; kentos-desktop 675 (80 yok sayılan; yeni `sheets::` 2 + 1 elle koşulan); kentos-ui 264; vitrin 59; render-wgpu hepsi; başarısız yok |
  | `cargo clippy -p kentos-ui -p kentos-ui-showcase -p kentos-render-wgpu -p kentos-desktop -p kentos-sheet-ui -p kentos-sheet -p kentos-sheet-wasm --all-targets -- -D warnings` | temiz |
  | `pnpm arch:deps` | “Bağımlılık yönü temiz: 33 crate, 42 crate × hedef denetlendi.” |
  | `cargo metadata --locked` | temiz; kilide yalnız çalışma alanının crate'leri (`kentos-sheet-ui` dahil), dış paket yok |
  | `pnpm wasm`, `node crates/wasm/sheet-wasm/tests/smoke.mjs` | temiz; “… templates 14, kpafta 12, profiles 9 — hepsi Rust testlerinin beklediğiyle aynı.” |
  | `cargo test -p kentos-sheet-ui screens -- --ignored --nocapture` | 24 PNG |
  | `cargo test -p kentos-desktop sheet_screens -- --ignored --nocapture` | 4 PNG |

- Görüntüler `.run/shots/sheet-desktop/`: `pafta-…` (açık tema, önce) ve `grafit-…` (koyu), 1440 ve
  1100 genişlikte, altışar sahne: 1 genel pafta ve seçili başlık, 2 sürüklenirken yapışma çizgileri
  ve uzaklıklar, 3 Sayfa sekmesi ve geçerli yerleşim düzeni, 4 imar planı ve ön denetim, 5 CAD
  projesinde teknik pafta (kipin adları: “Görünüm penceresi”), 6 şablon galerisi; ayrıca
  `masaustu-{pafta,grafit}-{1440,1100}.png`: gerçek masaüstü uygulaması, örnek çizim, haritanın içi
  çizim hattının sahnesinden. Görüntüler yazılımsal çiziciyle alındı (GPU'lu anlık görüntü,
  kalabalık galeride iced_wgpu'nun hazırlık arabelleği hatasıyla düştü).
- Sapmalar: `tasks-rust.md` 39–51.
- Zayıf yerler: harita çerçevesinde çizimin yazıları yok; resimler ekranda ve PNG'de mozaik;
  döndürülmüş harita çerçevesinin kırpması dönmemiş kutusudur; denetçinin sürüklemesi birden çok
  geri alma adımı yazar; Boşluk ile geçici el yok; galerinin bulut kaynakları ve Eşitle/Paylaş
  masaüstünde bağlı değil; ön denetimin `project.crs` ve `map.placeFromView` eylemleri açılmıyor;
  ƒ bağları denetçide yalnız okunur; büyük bir çizimde harita yakınlaştıkça tuvale yeniden boyanır
  (önbellek görünüş değişmedikçe tutar); `Karelaj` ve `Pafta bölümleme` düğmeleri henüz bir şey
  yapmaz (kapalı görünür).
- *4b'de kapananlar:* harita çerçevesinde çizimin yazıları, mozaik, döndürülmüş kırpma, sürüklemenin
  geri alma adımları, Boşlukla el, galerinin bulutu ile Eşitle ve Paylaş, `project.crs` ve
  `map.placeFromView`, ƒ bağlarının düzenlenmesi. Kalanlar: büyük çizimde yakınlaştıkça yeniden
  boyama, `Karelaj` ve `Pafta bölümleme` düğmeleri.

**4b. Masaüstünün web'e denkliği, masaüstünde bulut (3 Ekim).** Koordinatörün altı maddesi; web'in
C bölümünün kodu ve sözleri (`apps/web/src/app/sheet/`, `ui/sheet/`) örnek alındı.

1. **Bulut masaüstünde.**
   - Eşitleme sürücüsü `kentos-cloud`'da (`crates/native/cloud/src/sheet_library.rs`, R-17), web'in
     `TemplateSync.once` / `act`'ının karşılığı. Çekirdeğin `plan_sync`'i cihazın kayıtlarıyla
     hesabın listesinden eylemleri çıkarır, her biri web'inki gibi yapılır:
     - Oluşturma: bulutun kimliği alınır; eski kimlik `formerIds`'e, cihazdaki revizyon
       `formerRevision`'a yazılır (o şablondan yapılmış paftalar onu tanımaya devam eder).
     - Yükleme 403 alırsa (rol düştü): değişiklik “… (bu cihazdaki kopya)” adıyla bu cihazda kalır,
       bulutun sürümü yeniden iner, web'in `read_only` cümlesi söylenir.
     - Çakışma: yerel sürüm kopya olarak yüklenir (`conflictOf`), bulutunki iner.
     - Silme: 404 yok sayılır, 403 sonraki koşuya kalır.
     - Geçici hata ve 401 koşuyu bitirir; 409 bir koşu daha ister; öbürleri web'in `action_failed`
       cümlesiyle söylenir.
     - Yineleme anahtarları web'in kalıbında (`sheet-tpl-{update,create,copy,delete}-…`); masaüstü
       metnin SHA-256'sından bir UUID yapar.
   - Masaüstünün ev sahibi (`apps/desktop/src/sheet_library.rs`, D-8, D-12):
     - oturum açılınca kitaplığı eşitler;
     - olayları uzun yoklamayla dinler: `/v1/me/sheet-templates/events`, 25 sn bekleme, sayfa 500,
       en yeni imleçten;
     - bir olaydan 300 ms sonra eşitler; hata olursa 5, 15, 30 ve 60 sn sonra yeniden dener;
     - oturum bu açılışta kapanırsa hesabın kopyaları galeriden düşer;
     - oturum yokken (çevrimdışı açılış) son hesabın kopyaları görünür ve bulut “giriş yapın” der.
   - `kentos-sheet-ui`'de web'in sözleri ve davranışı (`library.rs`, `gallery.rs`,
     `share_template.rs`, `save_template.rs`):
     - Galerinin kaynakları: Sistem, Benim, Kurumum, Benimle paylaşılanlar; sayıları ve boşken
       nedenleri.
     - Rozetler (`libraryBadges`): Bu cihazda, Eşitlendi, Eşitleniyor, Değişti/eşitlenmedi,
       Çakışma; Düzenleyebilir / Görüntüleyebilir, Paylaşıldı.
     - Eylemler (`actionsOf`), etiketleri ve kapalıyken nedenleriyle: Şablonlarıma kopyala /
       Çoğalt, Kopyasını düzenle / Düzenle, Paylaş…, Buluta eşitle / Şimdi eşitle, Sil.
     - Şablonu paylaş: kişi arama (ortak kurumların etkin üyeleri), Görüntüleyebilir /
       Düzenleyebilir ve açıklamaları, erişimi olanlar (sahip başta, kilitli), rol değiştirme,
       Kaldır ve sorusu. Sonuç cümleleri web'inkiler: “Bora Tan artık bu şablonu düzenleyebilir.”,
       “… artık bu şablonu göremiyor.”
     - Bu cihazdaki şablonda Paylaş… yine açılır: pencere “Önce eşitleyin. Paylaşmak için önce
       buluta eşitleyin. …” der ve “Buluta eşitle”yi sunar; bulut kimliği verince pencere yeni
       kimliği izler, kişiler ve roller görünür.
     - Şablon olarak kaydet: kaynak şablon kişinin kendisininse ya da onunla düzenleyici olarak
       paylaşılmışsa (eski kimliğiyle de bulunur) “Güncelle” seçilir; bulut kopyası değişti diye
       işaretlenir ve hemen eşitlenir.
     - Cihazın kaydı: `kentos.sheets.deviceTemplate` v1 zarfı (şablon ve bulut durumu; durumun
       tipi çekirdekte, `DeviceCloudState`). 4. adımın düz şablon dosyaları da okunur.
   - Uçtan uca, gerçek API ve geçici veritabanıyla:
     - `kentos-api` `the_desktop_syncs_a_template_library_across_devices_and_accounts`
       (`kentos-cloud`'un sürücüsü gerçek sunucuya, gerçek soketle):
       - A1'de yapılan şablon buluta çıkar, A2'ye iner.
       - Bora'ya düzenleyici olarak paylaşılır; sahibin adı “Ayşe Yılmaz” görünür.
       - Uzun yoklama bir düzenlemeyi 3 sn içinde duyar.
       - Çakışmada iki kopya da kalır.
       - Salt okunura düşen rolde değişiklik kopya olarak kalır.
       - Silme şablonu her yerden kaldırır.
     - Masaüstünün canlı testi
       `sheet_library::tests::a_template_made_on_the_desktop_reaches_another_account_and_another_device`:
       gerçek `kentosd serve`, iki hesap, üç cihaz, gerçek masaüstü uygulaması (`App`) ve web'in
       örnek çizimi. Adımlar:
       1. Ayşe A1'de ifraz sistem şablonundan pafta yapar. Çizimin öznitelikleri olduğu için galeri
          bir şey sormaz.
       2. “Belediye ifraz paftası” adıyla şablon kaydeder; önce “Bu cihazda”dır.
       3. Buluta eşitle: kimliği bulutunkine döner, rozet “Eşitlendi” olur.
       4. Şablonu paylaş'ta “bora” diye arar, Bora Tan'ı düzenleyici yapar. Pencere “Bora Tan artık
          bu şablonu düzenleyebilir.” der; Ayşe'nin kartı “Eşitlendi · Paylaşıldı” olur.
       5. Bora'nın bekleyen uzun yoklaması paylaşımı duyar (beş koşuda 9–41 ms; sınır 3 sn).
          Galerisinde “Benimle paylaşılanlar” altında “Düzenleyebilir · Eşitlendi” görünür.
       6. Ayşe'nin ikinci cihazı A2 şablonu bulur.
       7. A1'de açıklama değişip kaydedilince revizyon 2 olur. A2'nin uzun yoklaması bunu duyar (beş
          koşuda 34–42 ms). A2'de ve Bora'da revizyon 2 ve yeni açıklama vardır; Bora'nın rolü
          düzenleyicidir.
     - Sunucusuz `sheet_library::tests::without_the_server_the_library_waits_and_says_so`:
       - kitaplık “Çevrimdışı” der ve yeniden denemeyi bekler;
       - bulutun kendi bağlantı durumuna dokunmaz;
       - oturum kapanınca hesabın kopyaları gizlenir.
   - Veritabanı: kap `kentos-sheet-testdb-desktop` (`127.0.0.1:55440`, `postgis/postgis:18-3.6`),
     sunucu `127.0.0.1:55441`. Parolalar rastgeleydi ve oturumun geçici dizinindeydi. Sonunda
     sunucu durduruldu, kap durduruldu ve silindi; port 55440 ve 55441 boş. Kullanıcının 5432'deki
     `database-postgis-1`'ine dokunulmadı.
     - Bir kaza: `kentosd` bir kez yanlışlıkla argümansız çalıştı (argümansız hâli `serve`'dür).
       Depoda `.env.local` ve ortamda `KENTOS_*` olmadığından veritabanı adresi yoktu, yalnız
       `/v1/health` açıktı. Yaklaşık 2 dakika sonra durduruldu; hiçbir veritabanına bağlanmadı
       (`serve`, adres yoksa bağlanmaz).
     - Yeniden koşmak için (parolaları siz verin):

     ```sh
     docker run -d --rm --name kentos-sheet-testdb-desktop -p 127.0.0.1:55440:5432 \
       -e POSTGRES_PASSWORD=<yönetici> postgis/postgis:18-3.6
     docker exec kentos-sheet-testdb-desktop psql -U postgres \
       -c "create role kentos_cad_owner login nosuperuser nocreatedb nocreaterole password '<sahip>'" \
       -c "create role kentos_cad_app login nosuperuser nocreatedb nocreaterole nobypassrls noinherit password '<uygulama>'"
     # API testleri (kendi veritabanlarını açarlar):
     export KENTOS_TEST_ADMIN_URL=postgres://postgres:<yönetici>@127.0.0.1:55440/postgres
     export KENTOS_DATABASE_URL=postgres://kentos_cad_app:<uygulama>@127.0.0.1:55440/kentos_cad
     export KENTOS_DATABASE_OWNER_URL=postgres://kentos_cad_owner:<sahip>@127.0.0.1:55440/kentos_cad
     export KENTOS_TEST_DB=required
     flock .run/heavy.lock cargo test -p kentos-api sheet_templates
     flock .run/heavy.lock cargo test -p kentos-application --test sheet_templates
     # Canlı masaüstü testinin sunucusu, kendi veritabanında; ayarları geçici dizinde:
     docker exec kentos-sheet-testdb-desktop psql -U postgres -c "create database kentos_e2e owner kentos_cad_owner"
     docker exec kentos-sheet-testdb-desktop psql -U postgres -d kentos_e2e \
       -c "create extension if not exists postgis" -c "create extension if not exists pgcrypto" \
       -c "revoke connect, temporary on database kentos_e2e from public" \
       -c "grant connect on database kentos_e2e to kentos_cad_owner, kentos_cad_app" \
       -c "grant usage on schema public to kentos_cad_app"
     export KENTOS_ENV_FILE=<geçici>/kentos.env          # boş dosya; kökteki .env.local okunmaz
     export KENTOS_DATABASE_URL=postgres://kentos_cad_app:<uygulama>@127.0.0.1:55440/kentos_e2e
     export KENTOS_DATABASE_OWNER_URL=postgres://kentos_cad_owner:<sahip>@127.0.0.1:55440/kentos_e2e
     export KENTOS_API_BIND=127.0.0.1 KENTOS_API_PORT=55441 KENTOS_BLOB_DIR=<geçici>/blobs E2E_PW=<parola>
     K=target/debug/kentosd     # flock .run/heavy.lock cargo build -p kentos-api --bin kentosd
     $K migrate && $K tenant add --slug buro --name "Harita Bürosu" --seats 6
     $K user add --login ayse --name "Ayşe Yılmaz" --email ayse@buro.gov.tr --password-env E2E_PW
     $K member add --tenant buro --user ayse --role project_manager --seat
     $K user add --login bora --name "Bora Tan" --email bora@buro.gov.tr --password-env E2E_PW
     $K member add --tenant buro --user bora --role editor --seat
     $K serve &
     KENTOS_E2E_SERVER=http://127.0.0.1:55441 KENTOS_E2E_PASSWORD=$E2E_PW \
       KENTOS_DEMO_DRAWING=$PWD/.run/demo/ornek-1244-1249-ada.json KENTOS_E2E_SHOTS=$PWD/.run/shots/sheet-desktop \
       flock .run/heavy.lock cargo test -p kentos-desktop sheet_library::tests -- --ignored --nocapture
     kill %1; docker stop kentos-sheet-testdb-desktop    # --rm: kap silinir
     ```

2. **Ondalık ayırıcı nokta** (ADR 0149 kural 5). Bütün `kentos-sheet-ui`'de kural
   `kentos_geometry_core::display::fixed`:
   - denetçinin mm ve derece alanları ve sayı kutuları (U-1 … U-4: `point`, kipi kuran kuralı
     `set_point_rule` ile verir);
   - salt okunur sayılar, durum hücreleri (“Seçim 10.0 × 10.0 mm”), sürüklerken boy rozeti;
   - Değişkenler penceresi (sayılar `fixed(n, 2)`; yazarken virgül nokta sayılır);
   - pencerelerin iletileri (“4.0 MB”).

   `kentos-ui`'nin varsayılanı değişmedi: masaüstünün pafta dışındaki pencereleri eskisi gibi
   yazar (aşağıda, “4b'de yapılmayanlar”). Test: `status::tests::millimetres_have_a_point`,
   `a_point_number_has_no_grouping_and_no_minus_zero`, resimler.
3. **Harita çerçeveleri.**
   - Çizimin yazı nesneleri (parsel numaraları, ada adları, kotlar, etiketler) çerçevenin içinde,
     çizim alanının kendi etiket motoruyla (D-11): aynı yerleşim, yazı tipi ve biçim. Yazılar
     haritanın kamerasında en az 640 piksellik bir kenarla yerleşir, sonra çerçeveye küçülür
     (web'in harita resmi gibi). Kenarı kesen yazı çizilmez.
   - Paftanın tabloları, koordinat listeleri ve lejantları çizimden gelir (`sheet_inputs.rs`).
     İfraz paftasının parsel tablosu dolu çıkar. Koordinat sisteminin TM değerleri
     `fixtures/crs/v1/registry.json`'dan okunur; çekirdek meridyen yakınsamasını onlardan hesaplar
     (değer web'inkiyle karşılaştırılmadı).
   - Galerinin küçük resimleri ve şablon önizlemesi projenin çizimini `MapPainter`'la çizer (D-10).
     Küçük resim web'inki gibi ince çizilir: çizgi ve işaretler küçülme oranında incelir.
   - Nokta işaretleri çizim alanının şekilleriyle (halka, artı, üçgen; noktalı).
4. **Kalan açıklar.**
   - Döndürülmüş harita çerçevesi döndürülmüş kutusuna kesilir. Haritanın içini boyayıcı kendi
     kutusuna keser, çerçeve onu döndürür. Kâğıttaki karelaj ve genel bakış da artık dönmüş kutuya
     kesilir: çekirdeğin dönmüş `PushClip`'i dönüşüyle uygulanır (`paint::Clip`); önceden dönmemiş
     dikdörtgeni uygulanıyordu (sapma 41). Çizgi ve dolgu tam kesilir; ayağı kutunun dışındaki yazı
     çizilmez. Testler:
     `paint::tests::a_turned_clip_cuts_to_the_turned_box`,
     `painter::tests::the_demo_town_keeps_inside_the_content_s_box`,
     `sheets::tests::a_map_keeps_inside_its_frame_and_writes_the_drawing_s_text`; resim
     `{pafta,grafit}-{1440,1100}-7-donuk-harita.png`.
   - Boşluk basılıyken geçici el: işaretçi kâğıdın üstündeyse (`Space(true/false)`, imleç el).
     Test: `space_held_over_the_paper_is_the_hand_for_a_moment`.
   - Denetçide sürüklenen sayı tek geri alma adımıdır (`GestureStart` / `GestureEnd`,
     `commit_as` jesti birleştirir); yazılan her değer kendi adımıdır. Test:
     `a_dragged_number_is_one_undo_step_and_typed_ones_are_one_each`.
   - ƒ bağları düzenlenir: denetçinin “Veri (ƒ)” bölümünde bağlanabilen her özelliğin ƒ düğmesi
     “Veriye bağla” penceresini açar; hazır adlar eklenir. Denetim çekirdeğin
     `kentos_sheet::expr::check`'idir (web'in `checkExpression`'ı). Kaydet ya da Bağı kaldır, tek
     geri alma adımı. Test: `a_binding_is_written_checked_by_the_core_and_taken_off`.
   - Ön denetimin eylemleri:
     - `project.crs` projenin ayarlarını koordinat sisteminde açar (`crs.set`). Test:
       `the_preflight_s_coordinate_system_fix_opens_the_project_s_settings`.
     - `map.placeFromView` haritanın merkezini çizim alanının merkezine taşır, tek adım. Çizim
       yoksa nedenini söyler. Test: `the_preflight_takes_a_map_s_place_from_the_drawing_area`.
   - Ek olarak web'deki iki şey:
     - Seçim (`sheet.zoomSelection`): seçilenler sahneyi 72 piksel boşlukla doldurur. Sahne
       boyunu her çizimde bildirir (`StageEvent::Resize`), böylece Sığdır, Gerçek ve Seçim
       sahnenin o anki boyunu kullanır. Testler: `the_chosen_item_fills_the_stage_at_its_size`,
       `a_stage_drawn_at_a_new_size_says_so`.
     - Galeri, web'inki gibi pencereye sığar: pencereden 24 piksel içeride; dar pencerede kartlar
       ikişer sıralanır (`Fitted`). Önceden 1100 × 720'de pencereden taşıyordu. Test:
       `the_gallery_fits_the_window`.
   - Galeri, açıkken projenin özniteliklerini de okur; projenin değerleri değişince kartların
     eksik listesi yenilenir. Test:
     `the_gallery_asks_for_what_the_project_lacks_until_the_host_says_it_has_it`.
5. **Resimler (logo mozaiği).** Yeni paket eklenmeden düzgün gösterilebildi:
   - PNG `png` ile, JPEG `zune-jpeg` ile çözülür (ikisi de kilitte vardı). Resim tuvale piksel
     piksel çizilir: büyütülünce her hücre resmin bir pikseli, küçültülünce ekranın bir pikseli ve
     kapladığı piksellerin ortalaması. Renkler nicelenmez, mozaik yok. İki çizicide de aynıdır.
   - Test: `paint::tests::a_picture_is_drawn_pixel_by_pixel`. Resim `masaustu-resim-logo-*`:
     512 × 512 piksellik bir kurum amblemi antetin başlık satırında, Seçim ile %1471 ve %743.
   - Masaüstünde “Resim seç…” yoktu; 4. adımın “Resim” aracı boş çerçeve bırakıyordu. Web'inki gibi
     eklendi:
     - denetçide “Resim” (ad · W × H px) ve “Dosya: Resim seç… / Başka resim seç…”;
     - sistemin dosya penceresi (PNG, JPEG);
     - baytlar kitapla saklanır, seçili çerçeveler özeti alır, tek adım “Resim: ad”;
     - reddedilen dosya web'in sözleriyle söylenir.

     Test: `a_picture_file_is_kept_and_shown_in_one_step`.
   - Sığdırma seçenekleri web'in sözleriyle: İçine sığdır, Kapla, Ger, Özgün boy; Çerçevede kırp.
   - Açık kararlar aşağıda.
6. **Web'le yan yana resimler** (`.run/shots/sheet-desktop/`; web'inkiler
   `apps/web/scripts/e2e/out/shots/sheet/` ve `…/sheet-cloud/`):
   - Hepsi gerçek masaüstü uygulamasından, web'in örnek çiziminden: `Ornek_1244-1249_Ada.kcad`,
     `apps/web/scripts/style/demo-drawing.test.ts`'in yazdığı `.run/demo/ornek-1244-1249-ada.json`.
     CAD kipinde, çizim alanı çizimin ev görünüşünde. Önce Pafta (açık), sonra Grafit; 1440 ve 1100.
   - `masaustu-b1-ifraz-sistem-sablonu-*`: ifraz sistem şablonu, web'in değerleri ve parsel tablosu
     sütunları. Web'in `b1`'iyle yan yana; ön denetim burada 2 hata, web'de 3. Fark `@kullanici`:
     masaüstü onu işletim sisteminin kullanıcısından doldurur.
   - `masaustu-b4-galeri-sistem-*`, `masaustu-b4-galeri-benim-*`: web'in `b4`'ü.
   - `masaustu-resim-logo-*`: resim, yalnız masaüstü.
   - Bulut sahneleri, canlı testten:
     - `masaustu-bulut-paylas-*`: Şablonu paylaş, Bora eklendi;
     - `masaustu-bulut-esitlendi-*`: Benim, “Eşitlendi · Paylaşıldı”;
     - `masaustu-bulut-benimle-paylasilanlar-*`: Bora'nın galerisi.

     Web'in `sheet-cloud/4-paylasildi`, `5-benimle-paylasilanlar` ve `b4-galeri-benim`'iyle yan yana.
   - Ayrıca `{pafta,grafit}-{1440,1100}-{1…7}-*.png`: `kentos-sheet-ui`'nin uydurma kasabayla
     sahneleri; 7. sahne yeni, döndürülmüş harita.

**4b'nin bağlantı noktaları:** R-17, U-1 … U-4, D-8 … D-12 (§3). R-2: `Cargo.lock`'ta yalnız
`kentos-sheet-ui`'nin bağımlılık listesi değişti; çalışma alanındaki `kentos-geometry-core`,
`serde` ve `uuid` eklendi. Yeni dış paket yok. 4. adımın D-7 köprüsü duruyor: ADR 0163'ün masaüstü
adımı gelince atılır.

**4b'nin komutları** (son hâl, ağır olanlar `flock .run/heavy.lock` ile):

| Komut | Sonuç |
|---|---|
| `cargo test -p kentos-sheet -p kentos-sheet-ui -p kentos-cloud -p kentos-ui` | kentos-sheet: birim 314, fixtures 11, ops 7, templates 7 (+2 yok sayılan). kentos-sheet-ui: birim 31, `designer` 23, `library` 5, `screens` 1 yok sayılan. kentos-cloud: birim 80, `download_resume` 1 (+1 yok sayılan, eski). kentos-ui 265. Başarısız yok |
| `cargo test -p kentos-sheet-wasm`; `cargo build -p kentos-sheet --target wasm32-unknown-unknown` | 4 geçti; temiz |
| `cargo test -p kentos-desktop` | 682 geçti, 82 yok sayılan, başarısız yok |
| `KENTOS_TEST_DB=required cargo test -p kentos-api sheet_templates` | 3 geçti (yeni `the_desktop_syncs_a_template_library_across_devices_and_accounts`) |
| `KENTOS_TEST_DB=required cargo test -p kentos-application --test sheet_templates` | 5 geçti |
| canlı test (yukarıdaki tarif) | geçti; 12 bulut resmi |
| `cargo test -p kentos-desktop sheet_comparison_screens -- --ignored` | geçti; 16 resim |
| `cargo test -p kentos-sheet-ui --test screens -- --ignored` | geçti; 28 resim |
| `cargo clippy -p kentos-desktop -p kentos-sheet-ui -p kentos-cloud -p kentos-sheet -p kentos-ui -p kentos-api --all-targets -- -D warnings`; `-p kentos-sheet-wasm` | temiz |
| `pnpm arch:deps` | “Bağımlılık yönü temiz: 33 crate, 42 crate × hedef denetlendi.” |
| `cargo metadata --locked` | temiz |
| `rustfmt --edition 2024 --check` | 4b'nin dosyaları ve kendi crate'lerim biçimli. Paylaşılan dosyalardaki eski farklar benim değil (ör. `view.rs` şeridi, `cloud/api.rs` içe aktarımı) |

**4b'nin sapmaları:** `tasks-rust.md` 52–70.

**Açık kararlar (resimler, sahibin kararı).** Şimdiki çözüm yeni paket istemiyor ve iki çizicide
aynı. Ama üç sınırı var:

- büyütülünce yumuşatma yok (en yakın komşu);
- bir resim ekranda en çok 400 000 hücreyle çizilir; ~630 × 630 pikselden büyük gösterilen büyük
  bir resmin hücresi birkaç piksel olur;
- **SVG resim masaüstünde çizilemiyor.** “Resim seç…” SVG'yi nedenini söyleyerek reddediyor; web'de
  eklenmiş bir SVG masaüstünde boş resim kutusu görünür.

Seçenekler:

| Seçenek | Kazanç | Bedeli |
|---|---|---|
| A. Iced'in `image-without-codecs` özelliği: çözmeyi yine `png` ve `zune-jpeg` yapar, Iced RGBA'yı `Frame::draw_image` ile çizer | yumuşatma; büyük resim tek dokuyla, hızlı; iki çizicide de (tiny-skia'nın resim boru hattı var), PNG dışa aktarma dahil | yeni paketler: `image` 0.25 (codec'siz) ve `kamadak-exif` 0.6, ve onların küçük bağımlılıkları. Iced'in `image` özelliği bunlara ek olarak `image`'in bütün codec'lerini getirir |
| B. Kendi wgpu dokumuz, `shader` aracıyla | yeni paket yok; GPU'da yumuşatma | yalnız GPU'da; ~300 satır kod. Dışa aktarma ve testler yazılımsal çiziciyle olduğundan yine hücrelerle: iki ayrı yol |
| C. Iced'in `svg` özelliği (SVG resimler için) | SVG resim masaüstünde iki çizicide de (`Frame::draw_svg`) | yeni paketler: `resvg`, `usvg` ve bağımlılıkları (çalışma alanının kilidinde yoklar) |
| Şimdiki: hücreler | paket yok; iki çizici ve dışa aktarma aynı | yukarıdaki üç sınır |

Bu tablo `iced` 0.14, `iced_graphics` ve `iced_tiny_skia` 0.14'ün `Cargo.toml` ve kaynağından
okundu. Hiçbir özellik açılıp denenmedi: açmak kilit dosyasını değiştirirdi.

**4b'de yapılmayanlar:**

- Masaüstünün pafta sekmelerinde web'in “Yeni sürüm var” işareti yok (web C: `8-yeni-surum-var`).
- Kullan, şablonun sorularını masaüstünde sormuyor (web: `sablon-sorulari`, `6-sorular`). Yalnız
  projenin eksiklerini soruyor.
- Masaüstünün pafta dışındaki pencereleri ondalığı eskisi gibi yazıyor; kural yalnız pafta
  kipinde uygulandı (madde 2).
- Kenarı kesen harita yazısı yarım çizilmiyor, hiç çizilmiyor (sapma 58).

**Yapılmayanlar ve zayıf yerler:**

- **WASM sınırının web vitest'i** (§14) web ajanınındır; şimdilik Node duman koşusu var.
- **Paket büyük:** 3,3 MB (gzip 0,94 MB), geometri paketinden (2,3 MB) büyük. Yalnız pafta kipi
  açılınca yüklenir; `wasm-opt` ve boyut profili denenmedi.
- **Kayıt fixture'ları** (`ops/cases.json`, `display/*.json`, `svg/*.svg`) çekirdeğin çıktısıdır;
  gözle ve birkaç değer elle denetlendi, bağımsız türetilmedi. Gerilemeyi yakalar, ilk doğruluğu
  kanıtlamaz.
- **Şablonlar bir harita mühendisi ya da şehir plancısı tarafından incelenmedi.** Ölçüler ve
  hücreler mevzuat kaynağına bağlanmadı; imar planı şablonunun lejantı ve notları örnektir.
- **Başka koordinat sisteminde karelaj, derece-dakika-saniye yazıları, kerning, manyetik model**
  yok (ön denetim söyler; manyetik sapma elle girilir).
- **Önermediği kâğıtta şablon sıkışır:** A1/A0 şablonları A4'te uyarı verir (hata değil).
- **Yazı ölçüsü** yalnız ilerleme genişliğidir; `0°05′ 47″` gibi üs işaretlerinde aralık göze
  biraz geniş görünür.
- **`design.md` “Adımlar” tablosu** hâlâ “6 sistem şablonu” diyor (düzeltme 10 dedi); dokunulmadı,
  belge koordinatörün.

#### 5. adım: PDF ve GeoPDF (3 Ekim)

**Yapılanlar.** Koordinatörün sekiz maddesi ve sonradan gelen iki notu (eksik değer işareti ile eksik
karakter; harita içinin web'le aynılığı).

- **Çekirdek** (`kentos-sheet::pdf`, ~2 900 satır, testler içinde):
  - `to_pdf(book, &PdfInputs, &PdfOptions)`: pafta başına kâğıdın tam boyunda bir sayfa. Çekirdeğin
    çizdiği her şey vektördür.
  - Yazı tipleri CID alt kümesi olarak gömülür (`subsetter`), `ToUnicode` ile; genişlikler
    çekirdeğin ölçü tablosundandır.
  - PNG Flate ile ve SMask'la, JPEG DCT olarak gider. PNG okuyucusu çekirdeğindir (sapma 71).
  - Harita içi `MapContent::Vector` (katman başına bir OCG, çerçeveye kırpılır) ya da
    `MapContent::Raster`.
  - GeoPDF: harita başına `VP` + `Measure /GEO`; TM/UTM köşelerinin enlem-boylamını çekirdek
    hesaplar (`geodesy.rs`), `GPTS` on ondalıkla (sapma 72). `tm_wkt` koordinat sistemi kaydının
    yazışlarını okur.
  - Belirleyici: saat okunmaz, belge kimliği içeriğin özetidir. Üst veri değişkenlerden, üretici
    “KentOS”.
  - `fonts_needed`, `export_name`, `paper_palette`; `text::glyph`, `text::runs`, `text::text_runs`.
- **Eksik değer işareti `‹ad?›`** (U+2039/U+203A, 22 yüzün hepsinde var), önceki `⟨ad?⟩` yerine:
  çekirdek, ön denetim iletisi, masaüstünün metinleri, `design.md` §6–§9 ve fixture'lar.
- **Eksik karakter** (design §6 “Eksik karakter”):
  - Yüzde olmayan karakter, tablonun sırasıyla onu içeren ilk çizim yüzünden yazılır; hiçbir yüzde
    olmayan “?” yazılır. Kapsam ölçü tablosudur (sapma 78).
  - Çizim planı “?”yu yazar; ön denetim `glyph_missing` verir: başka yüzle yazılan bilgi, “?” uyarı.
  - PDF parçaları kendi yüzleriyle yazar ve gömer. SVG başka yüzün parçasını `<tspan>` ile
    adlandırır. Masaüstü ekranı parçaları tek tek çizer.
  - Fixture ailesi `glyphs/`: 9 parça durumu ve bir pafta, elle; Rust'ta `fixtures::glyphs`,
    WASM'da `smoke.mjs` “glyphs 12”. Görüntü: `.run/shots/sheet-pdf/eksik-karakter-1.png`.
- **WASM:** `toPdf` (ikili, hata fırlatır), `pdfFonts`, `tmWkt`, `textRuns`, `exportName`,
  `paperPalette`; tipler ts-rs ile üretildi (G-1). Altın PDF `fixtures/sheet/v1/pdf/ifraz.json`
  (40 101 bayt, `4aeed668…`); WASM aynı baytı verir.
- **Masaüstü:**
  - Dışa aktar → PDF olarak… penceresi web'in sözleriyle: Bu pafta / Seçtiklerim / Hepsi, GeoPDF,
    çizim katmanları PDF katmanı, haritaların nasıl gideceği, yedek dpi, dosya adı.
  - Dosya adları web'inki (sapma 84).
  - Yazdır: PDF geçici dosyaya yazılır, sistemin görüntüleyicisinde açılır (sapma 85).
  - Harita içi stilli katmanlardandır: stil motoru haritanın ölçeğinde, kâğıdın paletiyle kurar;
    yollar web'in `mapVectors.ts`'inin karşılığıdır (`map_vectors.rs`). Katmanın kalınlığı, deseni ve
    nokta simgesi stildeki gibidir. Çizimin yazıları masaüstünün etiket motorundandır, web'in
    çapalarıyla (D-13).
  - Vektör biçimi olmayan katmanlı harita resim olarak gider; pencere hangi katman ve neden olduğunu
    yazar (test: Bina'ya desen dolgusu, “Bina (desen dolgusu)”).
  - Ekrandaki paftanın haritası da PDF'inkini çizer (sapma 81).
- **İki eşitlik açığı kapandı:**
  - “Yeni sürüm var”: sekmede nokta, paftalar listesinde yazı, denetçide “Şablon” bölümü (sapma 83).
  - Kullan ve Yeni pafta şablonun sorularını sorar: “Paftanın bilgileri” penceresi (web'in
    `TemplateQuestions`'ı; sapma 82). Galerinin üstünde açılır, Vazgeç galeriye döner.
- **Koordinatörün harita içi maddeleri** (1–5): hepsi yapıldı.
  - Çizgi kalınlığı ve deseni stilden.
  - Renkler kâğıdın paletinden; paletin değerleri artık çekirdekte (`display::paper`, WASM
    `paperPalette`).
  - Çapa kuralı çekirdekte: çapası içeride olan yazı yazılır ve kesilir, dışarıdaki yazılmaz. Masaüstü
    ekranı da aynı kuralı izler. Altın PDF bu yüzden değişti (sapma 77).
  - Nokta simgeleri stildeki gibi, kâğıt mm boyunda; ortada nokta yok.

**Kabul ölçütleri** (iki dosya: çekirdeğin testi `cekirdek-ifraz.pdf`, gerçek masaüstünün dışa
aktarma yolu `masaustu-ifraz.pdf`; ikincisi demo çizimde, `App::update` ve `sheet_export_to` ile,
`sheet_pdf_files` testi). Çekirdeğin testi demo çizimi okuyamaz (çekirdek belgeye bağlı değil); onun
haritası testin yapay katmanlarıdır.

| Araç | `cekirdek-ifraz.pdf` | `masaustu-ifraz.pdf` |
|---|---|---|
| `pdfinfo` | 1190.55 × 841.89 pt (A3); Title “İfraz paftası”, Producer KentOS | aynı |
| `pdffonts` | Barlow 400, 400 italik, 500, 600: gömülü, alt küme, uni | aynı |
| `pdftotext` | “İFRAZ / TEVHİT PAFTASI”, Ankara, Çankaya, Kızılay, 1234, tablonun değerleri | “İFRAZ / TEVHİT PAFTASI”, Sivas, Suşehri, Kızılırmak, 1245, P-12, 27 tablo değeri, ‹kontrol_eden?› |
| `gdalinfo` | TUREF / TM33 (EPSG:5255); NEATLINE köşeleri beklenenden en çok 0,025 mm | TUREF / TM36 (EPSG:5256); en çok 0,021 mm |
| `gdalinfo -mdd LAYERS` | 5 katman | 12 katman |
| `pdftoppm -r 150` | `.run/shots/sheet-pdf/cekirdek-ifraz-1.png` | `masaustu-ifraz-1.png` |

Beklenen köşeler çekirdeğin hesabından değil, haritanın merkezi, ölçeği ve içerik kutusundan
bağımsızca hesaplandı (`*-haritalar.json`). Ayrıca `eksik-karakter.pdf` (Arimo ve Courier Prime Bold
gömülü; “TAKS ? 0,30”).

**Web'le karşılaştırma** (`node apps/web/scripts/e2e/sheet-pdf.mjs --masaustu
.run/shots/sheet-pdf/masaustu-ifraz.pdf`, ağır kilitle): **23 denetimin hepsi geçti.**

- Çekirdeğin çizdikleri: 69 sözcük aynı yerde (0,000 mm).
- Harita: 99 / 99 sözcük, ortak sözcükler 0,00 mm. Yalnız bir tarafta olan yazı yok.
- 150 dpi sayfa resimlerinde harita alanı piksel piksel aynı: bir pikselde bir gri seviyesi fark.
  Sayfanın kalan farkları imza hücreleri ve sayfa numarasıdır, yani paftanın değerleri.
- Kalan farklar:
  - Masaüstünün yedek resmi stilsizdir (sapma 80).
  - Web'in PDF'inde dolu daire ve halka simgelerinin dolgusu kaybolur, delikli simgenin deliği
    dolar (sapma 75). Demo çizimde görünmez.

**Web'in değiştirmesi gerekenler** (koordinatöre):

1. Eski işareti yazan arayüz metinleri:
   - `apps/web/src/ui/sheet/inspector/pageTab.ts:88` (“kâğıda ⟨ad?⟩ yazılıyor”);
   - `apps/web/src/ui/sheet/VariablesDialog.ts:18` (yorum) ve `:72` (“kâğıda ⟨ad?⟩ yazılır”);
   - `apps/web/src/ui/sheet/TemplateQuestions.ts`: `QUESTION_TEXTS.intro` ve baş yorumu “(ad?)”
     diyor.
2. `app/sheet/mapVectors.ts` `Outline`: tam daire (`circle`, `ring`) kapanmıyor, dolgusu
   `corePaths`'te düşüyor. `shapeOutline`'da tam yaya `closePath` eklenmeli ya da `Outline.arc` 2π'yi
   kapalı saymalı. Çift-tek kuralında başka halkanın içindeki halka delik olmalı.
3. İsteğe bağlı: dosya adı için `engine.exportName` (geçici metin yolu kalkar), kâğıdın renkleri için
   `engine.paperPalette`, tuvalin yazısı için `engine.textRuns`. Ekranın yazısı parçalara bölünürse
   PDF'le aynı yüzlerden çizilir. Çizim planı “?”yu zaten yazıyor.
4. Çapa kuralı için iş yok: çekirdek PDF'te uygular, web'in `frameLabels.ts`'i ekranda zaten çapaya
   bakıyor.

**Bağlantı noktaları:** R-18 … R-21, U-5, U-6, D-13 … D-17, G-1 (§3).

**Komutlar** (son hâl; ağır olanlar `flock .run/heavy.lock` ile):

| Komut | Sonuç |
|---|---|
| `cargo test -p kentos-sheet -p kentos-sheet-wasm -p kentos-sheet-ui -p kentos-render-wgpu -p kentos-ui -p kentos-desktop` | 1 447 geçti, 0 düştü, 158 elle koşulan atlandı |
| `cargo clippy` (aynı altı crate) `--all-targets -- -D warnings` | temiz |
| `pnpm wasm` + `node crates/wasm/sheet-wasm/tests/smoke.mjs` | “pdf 6, glyphs 12”, hepsi Rust'ın beklediği; altın PDF bayt bayt |
| `node scripts/arch/deps.mjs` | temiz: 33 crate, 42 crate × hedef |
| `cargo metadata --locked` | temiz |
| `rustfmt --edition 2024` (benim dosyalarım) | biçimli. `main.rs` ve `style/scene.rs`'teki eski farklar benim değil |
| `node apps/web/scripts/e2e/sheet-pdf.mjs --masaustu …` | 23 / 23 |
| `sheet_pdf_files`, `pdf_files`, `sheet_step5_screens`, `sheet_comparison_screens` (elle) | dosyalar ve görüntüler yazıldı |

**Görüntüler:**

- `.run/shots/sheet-pdf/`: `cekirdek-ifraz-1.png`, `masaustu-ifraz-1.png`, `eksik-karakter-1.png`.
- `.run/shots/sheet-desktop/masaustu-c5-{pdf-penceresi,sorular,yeni-surum}-{pafta,grafit}.png`.
- `masaustu-b1…`, `b4…`, `resim-logo…` yeniden çekildi: ekrandaki harita artık stilli.

**Sapmalar:** `tasks-rust.md` 71–86.

**Yapılmayanlar:**

- Yazdır'ın gerçek görüntüleyiciyi açması elle denenmedi; testte açıcı taklittir. Kullanıcının
  masaüstünde pencere açılmasın diye.
- Masaüstünün yedek resmi stilli değil (sapma 80).
- PDF'te SVG resmi boş kutudur (sapma 73).
- Ölçü tablosunun dışındaki karakterler (Yunanca, Kiril) “?” yazılır (sapma 78).

#### 6. adım: WMM, manyetik kuzey (3 Ekim)

**Yapılanlar.**

- **Veri:** NOAA/NCEI'nin resmî `WMM.COF`'u (`WMM2025COF.zip`'ten) ve iki sınama dosyası
  `fixtures/sheet/v1/wmm/`'de olduğu gibi duruyor.
  - `scripts/geodesy/wmm_coefficients.py` bunlardan `crates/shared/sheet/data/wmm2025.json`'u
    üretir: 90 katsayı satırı; kaynak adresi, yayın (17 Aralık 2024), geçerlilik 2025,0–2030,0,
    kamu malı, dosyanın SHA-256'sı.
  - `--check` tablonun güncelliğini, `--fetch` NOAA'daki dosyanın tutulanla aynılığını denetler
    (koşuldu: aynı).
  - `docs/deps`'te “Gömülü veri” satırı (R-23).
- **Çekirdek** (`kentos-sheet::wmm`, `libm`):
  - `field(lat, lon, km, year) -> MagneticField` (D, I, X, Y, Z, H, F): WGS84 jeodezikten
    geosentriğe, 12. dereceye kadar küresel harmonikler, yıllık değişim; NOAA'nın `geomag` biçimi
    (sapma 87).
  - Yanında `declination`, `decimal_year`, `valid`, `info`.
- **NOAA'nın değerleriyle sınama** (`tests/wmm.rs`): sayfadaki 12 nokta ve katsayı paketindeki 100
  nokta (2025,0–2029,5; 0–100 km).
  - Açılar en çok 0,005° farklı. Fark değerlerin iki ondalığa yuvarlanmasıdır; ölçüt 0,01°.
  - Bileşenler en çok 0,0007 nT farklı; ölçüt 0,01 nT.
  - WASM aynı noktalarda aynı (`smoke.mjs` “wmm 114”).
- **Kuzey oku:**
  - `north: magnetic` sapmayı haritanın merkezinden ve paftanın tarihinden (`@tarih`) hesaplar.
    Notu: “Manyetik sapma 6°19′ D (WMM2025, 2026-10)”.
  - Yeni alan `declinationHand`: elle girilen değer hesabın yerine geçer ve “(elle)” ya da
    “(elle, 2024)” yazılır. Eski kitapla uyum sapma 88'de.
  - Yeni biçim `diagram` (“Kuzey çizelgesi”): GK, CK ve MK tek noktadan, aralarında yaylar,
    altında üç açı (sapma 91).
- **Ön denetim:**
  - `magnetic_out_of_model` (uyarı, 2025,0–2030,0 dışı).
  - `magnetic_no_place` (hata; düzeltmeler “Koordinat sistemi seç” ve “Sapmayı elle gir”).
  - `magnetic_no_date` (hata).
  - Projenin hiç sistemi yoksa yalnız eski `needs_crs`.
- **WASM:** `magneticField`, `wmmInfo`, `decimalYear`; tipler `MagneticField`, `WmmInfo`
  (ts-rs). `.run/sheet-engine-ready`'ye “WMM” notu eklendi.
- **Masaüstü:** denetçide “Biçim → Kuzey çizelgesi”, “Sapmayı elle gir”, “Elle sapma” (açıklamasıyla).
  Şeridin yakınlaştırmaları kâğıdı yeniden çiziyor (bulunan hata, sapma 92).

**Görüntüler** (gerçek uygulama, demo çizim, ifraz paftasının kuzey oku çizelge biçiminde):

- `.run/shots/sheet-desktop/masaustu-c6-kuzey-cizelgesi-{pafta,grafit}.png`: pafta.
- `…-c6-kuzey-cizelgesi-yakin-{pafta,grafit}.png`: çizelge yakından.
- Çizelgenin metni: “GK–CK yakınsama −0°06′”, “CK–MK manyetik sapma 6°19′ D (WMM2025, 2026-10)”,
  “GK–MK açısı 6°25′ D”, “Açılar ölçekli değildir.”
- Demo çizim TM36'nın orta meridyenine yakındır (≈35,85° D), bu yüzden yakınsama küçük.

**Komutlar** (son hâl, ağır olanlar kilitle):

| Komut | Sonuç |
|---|---|
| `cargo test --no-fail-fast -p kentos-sheet -p kentos-sheet-wasm -p kentos-sheet-ui -p kentos-desktop` | 1 146 geçti, 0 düştü, 89 elle koşulan atlandı |
| `cargo clippy` (aynı dört crate) `--all-targets -- -D warnings` | temiz |
| `pnpm wasm` + `smoke.mjs` | “wmm 114”, bütün aileler Rust'ın beklediği |
| `python3 scripts/geodesy/wmm_coefficients.py --check` ve `--fetch` | güncel; NOAA'nın dosyası tutulanla aynı |
| `node scripts/arch/deps.mjs`, `cargo metadata --locked` | temiz |
| `sheet_step6_screens` (elle) | dört görüntü |

**Bağlantı noktaları:** R-22, R-23, R-24 (§3). Yeni paket yok.

**Sapmalar:** `tasks-rust.md` 87–92.

**Yapılmayanlar:**

- Web'in denetçisi ve ekranı web ajanınındır: `declinationHand` anahtarı, “Kuzey çizelgesi”
  seçeneği. Çizelgenin kendisi çizim planında, web onu zaten çizer.
- Yıllık değişim (sapmanın yıllık artışı) çizelgede yazılmıyor; tasarım istemiyor, model
  verebilir.

#### 6b. adım: kuzey çizelgesinin sığması, `northInfo` (3 Ekim)

Koordinatörün 6. adımı gözden geçirmesinden dört iş.

**Yapılanlar.**

1. **Taşma (hata).** İfraz şablonunun 36 mm'lik okunda “CK–MK manyetik sapma … (WMM2025,
   2026-10)” satırı çerçevenin iki yanından taşıyordu ve ön denetim susuyordu.
   - Neden: her satır ayrı ayrı yarı boyuna kadar küçülüyordu (0,9 mm), sonra sessizce taşıyordu.
   - Şimdi çizelgenin açıları ve okun notu tek blok:
     - satır önce kaynağından önce kırılır, sonra boşluklardan;
     - yetmezse yazı küçülür, en çok 1,5 mm'ye (`text::LEGIBLE_MIN`, `text::layout_down_to`);
     - geniş çerçevede açılar çizelgenin yanına yazılır, öbür türlü altına; biri sığmazsa öbürü
       denenir;
     - yine sığmazsa `text_overflow` (öğeyle).
   - Çekirdeğin yazdığı öbür yazılar da artık sığmadığında `text_overflow` verir:
     - ölçek çubuğu: sayısal yazısı, “Ölçek 1/…” satırı, çerçeveden yüksek olması;
     - lejant: başlık ve tek bir sözcüğü sütundan geniş etiket ya da grup adı;
     - ızgara yazıları zaten sığmayanı yazmıyor; bant derinliği `grid_label_band`'dı (değişmedi).
   - 100 kâğıtlık matris (on şablon × A4…A0 × iki yön) yeni sınamada iki kez daha koşulur:
     her kuzey oku (1) manyetik + notlu ve (2) çizelge + manyetik.
     - İlk koşuda dört şablonun (aplikasyon, genel A4, GIS atlası, rapor) 9–11 mm'lik okları
       taşıyordu; bu boyda çizelge 1,5 mm'de bile sığmıyor.
     - Bu dört okun çerçevesi sola genişledi (26–28 mm; boy ve sağ kenar aynı; iki yön
       değişkesinde de). Sapma 96'da gerekçesi ve görünen sonucu var: beyaz kutu genişledi, K oku
       aynı boyda kutunun ortasında.
     - Şimdi iki koşu da bulgusuz.
2. **`northInfo(book, sheetId, itemId, inputs)`** (WASM; Rust `display::north_info`), tipi
   `NorthInfo`. Döndürdükleri:
   - haritanın merkezinin enlem-boylamı (çekirdek çevirir);
   - yakınsama;
   - sapma ve kaynağı (`model`/`hand`) ile elle değerin yılı;
   - sapma bilinmiyorsa nedeni (`noPlace`/`noDate`);
   - model ve geçerlilik yılları;
   - kullanılan tarih ve kaynağı (`sheet`/`project`/`today`), ondalık yılı, modelin içinde mi.

   Web kitabın kopyasından okumayı bırakabilir. “WMM” notu güncellendi.
3. **`magnetic_out_of_model`'in düzeltmeleri** artık çekirdekte: “Değişkenleri aç” (action
   `sheet.variables`) ve “Sapmayı elle gir” (`setItemProps` `declinationHand: true`; değer korunur).
   Web'in elle eklediği düğmeler kalkabilir.
4. **“6°19′  D”'deki çift boşluk** biçimlendiricinin değil, Barlow'un ′ glifinin tasarımıydı:
   0,60 em genişlik, 0,46 em'i boş. Açılar artık ASCII `'` ve `"` ile yazılır, tek normal boşlukla
   (“6°19' D”, “−0°05'47"”; sapma 93).

**Görüntüler** (çekirdeğin SVG'si, Chrome; çizelge + manyetik): `.run/shots/sheet/north-diagram/`
- `ifraz-paftasi.png`: açılar çizelgenin altında, 36 mm.
- `aplikasyon-krokisi.png`, `gis-atlas-a4-dikey.png`, `genel-a4-dikey.png`, `rapor-sayfasi.png`:
  açılar çizelgenin yanında, 26–28 × 16–19 mm.
- `imar-plani.png`.
- Bütün şablonların sayfaları: `.run/shots/sheet/templates/north-diagram/*.html`
  (`north_diagram_shots`, elle).

**Değişen altın dosyalar:**

- SVG'ler: ifraz, imar, genel A3 (yakınsamanın işaretleri) ve dört okun çerçevesi.
- `display/every-kind-*.json`: yalnız yakınsama satırı.
- `ops/cases.json`: genel A4'ün şablonunu taşıyan durum.
- Altın PDF: `6c194fa0…9234`, 40 064 bayt.

Her farkı okudum. Okların şekilleri ve öbür bütün çizimler aynı; değişen yalnız açı yazıları,
notun 1 µm'lik yuvarlaması ve dört okun yeri.

**Komutlar** (ağır olanlar kilitle):

| Komut | Sonuç |
|---|---|
| `cargo test --no-fail-fast -p kentos-sheet -p kentos-sheet-wasm` | birim 360, fixtures 12, ops 7, pdf 7 (+1 elle), templates 8 (+3 elle), wmm 8, wasm 4; düşen yok |
| `cargo clippy -p kentos-sheet -p kentos-sheet-wasm … --all-targets -- -D warnings` ve `--features ts,schema` | temiz |
| `pnpm rust:wasm:sheet` + `smoke.mjs` | “wmm 117” (yeni üçü `northInfo` ve açı işaretleri), bütün aileler Rust'ın beklediği |

**Web'in değiştirmesi gerekenler** (koordinatöre):

- `apps/web/scripts/e2e/sheet-north.mjs` 4., 87., 98. ve 116. satırlarda “′” arıyor; artık `'`.
- `apps/web/src/ui/sheet/inspector/northSection.ts` (18., 43. satır) açıyı kendisi “6°19′ D” diye
  yazıyor. Kâğıt gibi `'` yazmalı ya da `northInfo`'yu okumalı; o zaman kitabın kopyası da gider.
- `magnetic_out_of_model`'in düğmeleri çekirdekten gelir. “Sapmayı elle gir”, web'in anahtarı gibi
  modelin değeriyle başlar (sapma 100).
- İfraz paftasının PDF'i değişti. `sheet-pdf.mjs` altın değeri fixture'dan (`pdf/ifraz.json`)
  okur; yeni WASM'la (`pnpm wasm`) kendiliğinden tutar.

**Bağlantı noktaları:** R-24, G-2 (§3). Yeni paket yok. **Sapmalar:** `tasks-rust.md` 93–99 (99:
disk doldu; altı saatten eski artımlı derleme önbellekleri silindi, 49 GB).

**Yapılmayanlar:** Masaüstü denetçisi web'inki gibi hesaplanan sapmayı ve tarihin kaynağını salt
okunur göstermiyor. Yalnız “Sapmayı elle gir” ile “Elle sapma” var; `north_info` ile eklenebilir.

#### 7. adım: kurum şablonları (3 Ekim)

**Yapılanlar.**

- **Sunucu, göç `0014_org_sheet_templates.sql`** (R-25; 0013 değişmedi, sapma 101). Rolü tek yer
  hesaplar, `sheet_template_role(şablon, sahip, alan)`. Kurum şablonunda:
  - `owner`: yayımlayan, yayımlama yetkisi sürdükçe;
  - `admin`: kurumun sahibi ve yöneticileri (yeni rol, sapma 102);
  - `viewer`: öbür etkin, koltuklu üyeler;
  - misafir, ayrılan, üyeliği kapalı ya da koltuksuz kişi ve askıdaki kurum: hiçbir şey.

  Ayrıca: yayımlama yetkisi `sheet_template_can_publish`; satır güvenliğinin bütün politikaları
  yeni rolle; olayların kitlesi kurumun etkin, koltuklu üyeleri; yayımlayanın adı üyelere
  görünür (sapma 107).
- **Uygulama** (`kentos-application::sheet_templates`):
  - beş komut kurumun yolunda da çalışır;
  - yeni `sheet.template.publish` v1 `{templateId, tenantId}`: kendi kişisel şablonunu, bulutun son
    revizyonuyla ve `publishedFrom` ile kuruma kopyalar;
  - komut şablonun alanının yoluna gider (sapma 104);
  - kurum şablonu paylaşılmaz (sapma 103);
  - liste `organizations: [{ tenantId, name, canPublish, templates }]` döner;
  - çekirdeğin kataloğunda yeni komut, adı ve izni (`project.create`).
- **Çekirdeğin tipleri** (`kentos-sheet::cloud`, G-2): `TemplateRole::Admin`, `CloudOrganization`,
  `OrganizationTemplates`, `SheetTemplatePublish`, `SheetTemplateSummary.organization`/`.publishedFrom`,
  `DeviceCloudState.organization`/`.published_from`, `DeviceOrganization`,
  `SheetTemplateList::every()`.
- **`kentos-cloud`:**
  - komutlar alanın yoluna (`space`) gider; `publish` eklendi;
  - eşitleme kurum şablonlarını da planlar;
  - her komut kaydın kurumunun yoluna gider; çakışma kopyası kişisel alana gider (sapma 106);
  - listeden düşen kurum şablonu için yeni ileti (`left_organisation`);
  - rapor kurumları taşır.
- **Masaüstü ve `kentos-sheet-ui`:**
  - “Kurumum” canlı: kartlar kurumun adı başlığı altında; rozetler “Kurum: <ad>”, “Yayımladınız”,
    “Yöneticisiniz”; bölüm oturum yokken ve kurum yokken nedenini söyler.
  - Eylemler: “Şablonlarıma kopyala”; yöneticiye ve yayımlayana “Düzenle” ve “Sil”, öbürlerine
    “Kopyasını düzenle”; Paylaş… yok.
  - Kendi şablonunda “Kuruma yayımla…” (yeni `publish_template` penceresi): kurum seçilir,
    Yayımla. Kurumda bu şablondan bir kopya varsa birincil düğme “Kurumdakini güncelle” olur
    (sapma 105).
  - Masaüstü yayımlar, ardından eşitler; galeri Kurumum'a geçer ve yeni şablonu seçer.
  - Şablon olarak kaydet → “Bu şablonu güncelle”de yönetici de yazar.
- **`.run/sheet-engine-ready`'ye “KURUM” notu:** uçlar, kurallar, eşitleme ve masaüstünün bütün
  sözcükleri, web aynısını yapsın diye. Web'in cihaz kaydı (`TemplateCloudState`) iki alan almalı ve
  `ROLES`'e `'admin'` eklenmeli.

**Negatif sınamalar** (ADR 0024 düzeyi, `tests/sheet_templates.rs`
`an_organisation_s_library_refuses_at_the_level_of_adr_0024` ve API testi):

- **Üye olmayan ve misafir:** Ece (kurumsuz), Can (başka kurum) ve Fatma (Büro'nun bir projesinde
  misafir) Büro'nun yolunda 404 alır. Şablonun ayrıntısında da 404 alırlar, uydurulmuş bir kimlikle
  sözcüğü sözcüğüne aynı. Listelerinde Büro yok.
- **Koltuksuz üye:** Hakan yolda 403 alır, şablonu görmez.
- **Yayım yetkisi olmayan üye:** Bora (düzenleyici) ve Dilek (görüntüleyici) şunlarda 403 alır:
  yayımlama, kurumda oluşturma, düzenleme ve silme. HTTP'de de: publish 403, update 403.
- **Yayımlanamayanlar:**
  - paylaşılan şablon ve kurum şablonu: 403;
  - görünmeyen şablon: 404 (sözcüğü sözcüğüne);
  - kişisel yol ya da başka kurum: 422.
- **Yetkisi giden yayımlayan:** proje yöneticisinden düzenleyiciye inen Ayşe yalnız kullanır
  (update 403, `canPublish` yanlış).
- **Kurumdan ayrılmak:** üyelik kapanınca, koltuk alınınca ve üyelik silinince şablon 404 olur,
  Büro listeden düşer, sonraki değişikliğin olayı gelmez. Askıdaki kurumu kimse görmez.
- **Satır güvenliği:** başka kurumun kapsamında, Can'ın ve misafirin kapsamında Büro'nun
  şablonları ve revizyonları görünmez; üye görür. Yayım yetkisi olmayanın elle yazdığı satırı
  satır güvenliği reddeder.

**Uçtan uca** (gerçek `kentosd serve`, gerçek masaüstü uygulaması, iki kişi, demo çizim;
`sheet_library::tests::an_organisation_s_library_is_live_for_two_people`):

1. Ayşe (Büro'da proje yöneticisi) sistem ifraz şablonunu kendine kopyalar ve buluta eşitler.
   “Kuruma yayımla…” → Harita Bürosu → Yayımla. Günlük “… kurumunda yayımlandı” der.
2. Ayşe'nin Kurumum'unda kurumun kopyası görünür: “Kurum: Harita Bürosu”, “Yayımladınız”,
   “Eşitlendi”.
3. Bora'nın (düzenleyici) uzun yoklaması yayımı **25 ms**'de duyar. Kurumum'da şablon
   “Kurum: Harita Bürosu”, “Eşitlendi” rozetleriyle görünür.
4. Bora için Sil'in gerekçesi “yalnız yayımlayan ve kurum yöneticileri siler”dir.
   “Şablonlarıma kopyala” şablonu onun cihazına kendi şablonu olarak alır.
5. Ayşe kendi şablonunu değiştirir ve eşitler (2. revizyon). Pencere bu kez “Kurumdakini güncelle”
   der. Kurumun kopyası 2. revizyon olur; Bora'nın cihazı bunu duyup indirir (görüntüleyici olarak).

**Görüntüler** (`.run/shots/sheet-desktop/`, Pafta ve Grafit, 1440 ve 1100):

- `masaustu-c7-kuruma-yayimla-*`: pencere;
- `masaustu-c7-kurumum-ayse-*`: Kurumum, yayımlayan;
- `masaustu-c7-kurumum-bora-*`: Kurumum, öbür üye;
- `masaustu-c7-kurumdakini-guncelle-*`: kopya varken pencere;
- `masaustu-c7-kurumum-bora-guncel-*`: güncellenen kopya Bora'da.

**Veritabanı:**

- Testler, geçici kap `kentos-sheet-testdb-org` üzerinde (`127.0.0.1:55442`,
  `postgis/postgis:18-3.6`, rastgele parolalar oturumun geçici dizininde). Bütün komutlar
  `KENTOS_TEST_ADMIN_URL` ile o kaba gitti.
- Canlı sunucu aynı kapta `kentos_e2e` veritabanında, `127.0.0.1:55443`'te, ayarları geçici
  dizinde (`KENTOS_ENV_FILE`).
- Sonunda sunucu durduruldu, kap durduruldu ve silindi; 55442 ve 55443 boş.
- Kullanıcının 5432'deki `database-postgis-1`'ine dokunulmadı.
- Yeniden koşmak için 4b'deki tarif geçerlidir (kap adı ve portlar başka).

**Komutlar** (son hâl, ağır olanlar kilitle, veritabanlılar `KENTOS_TEST_DB=required` ile):

| Komut | Sonuç |
|---|---|
| `cargo test -p kentos-postgres -p kentos-application -p kentos-api -p kentos-cloud` | postgres 3; application birim 34 + 20 test dosyasında 77 (`sheet_templates` 7: yeni ikisi kurumun); api 42 (+1 yok sayılan, eski; yeni `an_organisation_s_library_over_http_and_on_a_device`); cloud 81 + 1 (+1 yok sayılan, eski); düşen yok |
| `cargo test -p kentos-sheet-ui -p kentos-desktop` | sheet-ui birim 33, designer 26, library 7 (yeni: Kurumum ve yayım); masaüstü 689 (86 elle koşulan atlandı) |
| canlı masaüstü testi (yukarıdaki) | geçti, 141 sn (görüntülerle) |
| `cargo clippy` (bu adımın yedi crate'i) `--all-targets -- -D warnings` | temiz |
| `node scripts/arch/deps.mjs`, `cargo metadata --locked` | temiz (33 crate, 42 crate × hedef) |

**Bağlantı noktaları:** R-24, R-25, G-2 (§3). Yeni paket yok. **Sapmalar:** `tasks-rust.md` 100–110
(100, 6b'ye aittir).

**Yapılmayanlar:**

- Web'in Kurumum'u web ajanınındır; “KURUM” notunda her şey var.
- Kişiye tek kurum şablonu paylaşmak ve kurumun kendi şablon sınırı tasarımda yok.

#### 8. adım: masaüstünde resim kalitesi (3 Ekim)

**Yapılanlar.**

- **Paketler** (sahibin onayı, 3 Ekim; R-26 – R-28, `docs/deps`):
  - Iced'in `image-without-codecs` ve `svg` özellikleri (sheet-ui) ve doğrudan `resvg` 0.45.1:
    eklenen SVG'nin boyu `usvg` ile ölçülür.
  - Kilide 30 paket girdi; lisansları tarandı, hepsi izin veren lisanslar.
  - `image` kod çözücüsüzdür, resimleri sheet-ui kendisi çözer. resvg'nin `raster-images`'ı
    kapatılamadı (sapma 115).
- **Doku** (`pictures.rs`): resim, kendi boyuyla ve yarılanmış hâlleriyle dokudur. Yarılamada
  renkler opaklıklarıyla ağırlıklanır.
  - Küçük gösterilen resim boyuna yakın yarılamadan çizilir, titremez.
  - Büyütülen resim tam katlı, yumuşatılmış (çift doğrusal) dokudan çizilir (sapma 111, 112).
  - Hücrelerle çizim ve **400 000 hücre sınırı kalktı**.
- **Katman** (`paint::plan`, `stage::paper_layers`): her resim, harita gibi iki kâğıt katmanının
  arasında kendi tuvalindedir. Çizim sırası korunur, resimden sonra gelen şekil üstte kalır
  (sapma 113).
- **SVG resimleri** artık masaüstünde de eklenir ve çizilir (resvg):
  - boyu dosyanın söylediğidir;
  - “Resim seç…” süzgeci ve uyarı sözcükleri “PNG, JPEG ya da SVG” der;
  - PDF'te SVG resmi eskisi gibi boş kutudur, iki platformda aynı (sapma 114).
- **PNG çıktısı ekranla aynıdır:** aynı katmanlar, aynı dokular. Yazılım çizicisinin büyütülmüş
  dokuyu kaydırması tam katlı büyütmeyle giderildi (sapma 112, test
  `a_turned_picture_turns_as_the_shapes_do`).

**Sınamalar:**

- `paint::tests`:
  - `a_picture_is_smoothed_when_magnified_and_averaged_when_shrunk`: dört renkli 4×4 resim 40 px'e
    büyütülünce dörtte birlerin sınırında iki renk karışır, kare yok; 64×64 dama 8 px'te gri olur.
  - `a_large_picture_keeps_its_detail`: 1000 tek piksellik şerit kendi boyunda tek tek kalır.
    Hücreli çizim bu boyda griye dönerdi.
  - `an_svg_picture_is_drawn`.
  - `a_turned_picture_turns_as_the_shapes_do`.
  - `a_picture_has_a_layer_of_its_own`.
- `pictures::tests`: yarılamalar ve doku seçimi, tam katlı büyütme ve en çok dört tutulması, SVG'nin
  boyu (piksel ve mm).
- `tests/designer.rs`: SVG “Resim seç…” ile eklenir.

**Görüntüler** (gerçek masaüstü uygulaması, demo çizim, ifraz paftası; `sheets::tests::sheet_step8_screens`):

- `.run/shots/sheet-desktop/masaustu-c8-resim-yakin-{pafta,grafit}.png`: başlık satırında 64
  piksellik logo ve KentOS işareti (web'in `favicon.svg`'si) %849'da. Logo yumuşak, işaret keskin.
- `masaustu-c8-resim-disa-aktarim-kesit.png`: aynı paftanın 400 dpi PNG çıktısından (6614×4677)
  iki resmin kesiti; ekrandakiyle aynı.

**Komutlar** (ağır olanlar kilitle):

| Komut | Sonuç |
|---|---|
| `cargo test -p kentos-sheet-ui -p kentos-desktop` | sheet-ui birim 39, designer 26, library 7; masaüstü 689 (87 elle koşulan atlandı); düşen yok |
| `cargo clippy -p kentos-sheet-ui -p kentos-desktop --all-targets -- -D warnings` | temiz |
| `sheet_step8_screens` (elle) | geçti, üç görüntü |
| `node scripts/arch/deps.mjs`, `cargo metadata --locked` | temiz; kilitte 687 paket |

**Bağlantı noktaları:** R-24, R-26, R-27, R-28 (§3). **Sapmalar:** `tasks-rust.md` 111–115.

**Yapılmayanlar:**

- Galerinin küçük resimleri tek tuvaldir; orada resim şekillerin üstündedir (sapma 113). 9. adımda
  kapandı.
- PDF'e SVG resmi çizmek çekirdekte SVG çizici ister (design §9a). 9. adımda ev sahibinin PNG'siyle
  kapandı.

#### 9. adım: dört açık iş kapandı (3 Ekim)

Koordinatörün 9. adım notu. Sahibin isteği: her şey tamamen bitsin.

**Yapılanlar.**

1. **Masaüstü denetçisinde kuzey okunun sapması** (`sheet-ui` `inspector_view.rs` `north_part`,
   `inspect.rs`). Web'in `northSection` sözcükleri ve sırası, çekirdeğin `north_info`'sundan:
   - “Manyetik sapma”: “6°19' D · WMM2025 · 2026-10” ve altında nereden hesaplandığı: “Haritanın
     merkezinde (39.9148° K, 35.8454° D), paftanın tarihinde; NOAA'nın Dünya Manyetik Modeli
     (WMM2025: 2025–2030).”
   - “Hesap tarihi”: “2026-10-03”, “Değişkenler…” düğmesi (değişkenler penceresini açar) ve kaynağı:
     “bugün: paftada ve projede “tarih” değişkeni yok.” Tarih modelin yıllarının dışındaysa
     “değer yaklaşıktır”.
   - “Sapmayı elle gir” (anahtar). Elle girilirken “Elle sapma” (°, iki ondalık), “Sapmanın yılı”
     ve “Doğu artı, batı eksi; yıl kâğıtta “elle, 2024” diye yazılır.” Elle girilen değerde
     “Modelin değeri: …”. Konum ya da tarih yoksa “Hesaplanamıyor: …”.
   - Birden çok ok seçiliyse: “Manyetik sapma tek kuzey oku seçiliyken gösterilir.”
   - Anahtar açılınca değer modelinkiyle başlar: dakikaya yuvarlanır ve yalnız hiçbir değer
     yazılmamışsa yazılır, web gibi. Çekirdeğin `magnetic_out_of_model` düzeltmesi buna hizalandı
     (sapma 118). “Renk” alanı da eklendi.
2. **SVG resim PDF'te** (sapma 116). Çekirdeğe paket eklenmedi.
   - Çekirdek: `PdfAsset.raster` (ev sahibinin PNG'si), `pdf::svg_sizes` / WASM `pdfSvgSizes`
     (boyut: en büyük çerçeve, dışa aktarma dpi'si), `pdf::findings` / WASM `pdfFindings`.
     `svg_as_picture` bilgisi “SVG resim PDF'e resim olarak gömülür (… piksel)” der; PNG yoksa
     `svg_not_in_pdf` uyarısı gelir: boş kutu basılır, ama sessizce değil.
   - Masaüstü PNG'yi resvg ile çizer (`pictures::svg_png`, sistemin yazı tipleriyle) ve bulguları
     günlüğe yazar (Kaydet ve Yazdır).
   - Web'in işi (tarayıcıda çizmek) `.run/sheet-engine-ready`'nin PDF bölümünde (“SVG RESİM”).
3. **Masaüstünün harita resmi stilli** (sapma 120, 121). Yedek yol ulaşılabilirdi: vektör biçimi
   olmayan bir katman (desen dolgulu Bina) haritayı resme çevirir. Bu yüzden kaldırılmadı, stillendirildi.
   - Yeni: render-wgpu `styled::cpu`, stilli WGSL gölgelendiricilerinin CPU ikizi (R-29). Çizgi
     (yumuşak kenar dahil), düz / taralı / resimli / desenli dolgu, şekil ve resim/yazı simgeleri,
     aynı stil bloklarıyla, aynı görünürlük kurallarıyla ve atlasın aynı resimleriyle.
   - Masaüstü böyle bir haritayı ekranda da PDF'in resminde de bununla çizer (`paint_picture`).
   - Ayrıca bulunan hata: harita resimleri temanın açık gri zeminiyle çıkıyordu (5. adımdan beri).
     Şimdi zemin saydam, PDF'te kâğıdın üstüne oturur.
4. **Galeri kartlarının resmi çizim sırasını korur** (sapma 119). Kartın resmi plandaki katmanlar
   ve aradaki her harita ve resim için ayrı tuvallerdir (`gallery.rs` `thumb_stack`).

**Sınamalar:**

- `inspect::tests::the_declination_is_written_as_the_web_writes_it`: web'in sözcükleri.
- `tests/designer.rs` `the_inspector_shows_the_north_arrow_s_declination_and_types_it_by_hand`:
  `north_info`, alanlar, dakikalık başlangıç, değer ve yıl, kapat-aç.
- `tests/wmm.rs`: düzeltmenin dakikalık değeri.
- `tests/pdf.rs` `an_svg_picture_goes_in_as_the_png_the_host_drew`: boyut, uyarı, gömme (119 px ve
  maskesi), bilgi, JSON'da base64.
- `smoke.mjs` “pdf 9”: `pdfSvgSizes` ve `pdfFindings` WASM'da da aynı.
- `pictures::tests::an_svg_is_drawn_as_a_png_for_a_pdf`.
- Masaüstü `an_svg_picture_goes_into_the_pdf_as_a_picture_and_the_log_says_so`.
- render-wgpu `styled::cpu` 9 sınama: düz dolgu, desen (hücreler ve uzak ton), tarama, yumuşak
  çizgi, şekil simgesi, resim simgesi, resimli dolgu, ölçek aralığı, basamaklar.
- Masaüstü `a_map_with_a_pattern_fill_goes_as_a_picture_and_the_window_says_why`: haritanın
  resmi Bina'nın mavi noktalarını ve aradaki kâğıdı taşır, PDF resmi yumuşak maskeli. Eski yolla
  koşturuldu: 0 mavi, sınama düşer.
- `gallery::tests::a_card_s_picture_keeps_what_is_drawn_after_a_picture_over_it`: kartta kare
  resmin üstünde; aynı liste tek tuvalde çizilince resim karenin üstünde, yani sınama farkı yakalar.

**Görüntüler** (`.run/shots/sheet-desktop/`, gerçek uygulama, demo çizim; `sheet_step9_screens`):

- `masaustu-c9-kuzey-denetci-{pafta,grafit}.png`: kuzey okunun sapma bölümü.
- `masaustu-c9-desenli-harita-{pafta,grafit}.png`: Yapı katmanı desen dolgulu, ekranda CPU ikiziyle.
- `masaustu-c9-desenli-harita-pdf.png`: aynı haritanın PDF'teki resmi, 200 dpi kesit; çarpılar,
  dolgu, çizgiler, yazılar, beyaz kâğıt.
- `masaustu-c9-svg-pdf.png`: KentOS işareti (SVG) PDF'te resim olarak.
- `masaustu-c9.pdf`: belgenin kendisi.

**Bağlantı noktaları:** R-24, R-28, R-29, R-30, G-3 (§3). Yeni paket yok; resvg'nin `text` ve
`system-fonts` özelliklerinin adı yazıldı (kilit aynı). **Sapmalar:** `tasks-rust.md` 116–121.

### Web

**A, B, C, D, D+, E ve F bölümleri tamam (2–3 Ekim).** Yeniden eskiye: F'nin ve E'nin raporları,
D+'nın notu, D'nin, C'nin, B'nin ve A'nın raporları (tarihçe; sonraki bölümde değişenler o
bölümün raporunda yazılı).

#### F bölümü (3 Ekim)

**Yapılanlar:** kurum şablonları (tasarım §13 “Kurum şablonları”; motorun “KURUM” notu, 7. adım),
`northInfo`, Rust'ın 5–8. adımlarından web'e kalan bulgular ve 9. adımın SVG resmi (“SVG RESİM”).

- **Kurumum** (`app/sheet/providers.ts`, `templateShelf.ts`; `ui/sheet/TemplateGallery.ts`):
  - Bulut hesabıyla canlıdır. Hesabın etkin üyesi olduğu her kurumun şablonları kurumun adının
    altında durur (`role=group`, adı kurumun adı).
  - Kartın rozetleri sırayla: “Kurum: <ad>”; hesabın payı (“Yayımladınız”: yayımlayan;
    “Yöneticisiniz”: kurumun sahibi ve yöneticileri; düz üyede yok); eşitleme. Sıra ve sözler
    masaüstününkiler (`library.rs`).
  - Boşsa nedeni yazar: oturum yok; hesap bir kurumun etkin üyesi değil (“Bir kurumun etkin üyesi
    değilsiniz: …”); sunucuya ulaşılamıyor.
- **Kuruma yayımla…** (yeni `ui/sheet/PublishTemplateDialog.ts`; `app/sheet/cloudLibrary.ts`):
  - Kendi şablonunda görünür. Kapalıysa ipucunda nedeni yazar:
    - bu cihazdaysa “Kuruma yayımlamak için önce buluta eşitleyin.”;
    - yayımlanabilecek kurum yoksa kimin yayımladığı (kurum sahibi, yöneticiler, proje açabilen
      üyeler: `canPublish`);
    - bu cihazda eşitlenmemiş değişiklik varsa önce onun eşitlenmesi (kuruma bulutun son sürümü
      kopyalanır).
  - Kurumu kullanıcı seçer; listede yalnız yayımlayabildiği kurumlar vardır. `sheet.template.publish`
    kurumun yoluna gider. Şablon kurumda yeni kimlikle, 1. revizyon olur; `publishedFrom` kendi
    şablonudur.
  - Bu şablondan o kurumda yayımlanmış bir kopya varsa pencere bunu söyler. Birincil düğme
    “Kurumdakini güncelle” olur: bu şablonun içeriği o kopyanın yeni revizyonu olarak kurumun
    yoluyla eşitlenir.
  - Sunucunun reddi pencerede sunucunun sözleriyle yazar, pencere açık kalır. Bitince galeri
    Kurumum'u açar, şablon seçili gelir.
- **Rollere göre eylemler** (`ui/sheet/galleryPlan.ts`, `galleryDetails.ts`;
  `app/sheet/templateActions.ts`). Kurallar masaüstünün `library.rs`'indekilerle aynı.
  - Her üye: Kullan, “Şablonlarıma kopyala”, “Şimdi eşitle”.
  - Yayımlayan ve kurum yöneticileri: Düzenle (kurum şablonunun yeni revizyonu); Sil (“… “<kurum>”
    kurumunun şablonlarından silinsin mi?”, “Kurumun bütün üyelerinin listesinden kalkar.”).
  - Düz üye: “Kopyasını düzenle” (kendi şablonları arasında bir kopya; kurumunki değişmez). Sil
    kapalı; ipucunda “Kurum şablonunu yalnız yayımlayan ve kurum yöneticileri siler.”
  - Kurum şablonunda Paylaş yok: sunucu kurum şablonunun paylaşımını 422 ile reddeder.
- **Eşitleme** (`app/sheet/templateSync.ts`, `templateSyncParts.ts`). Kurallar `planSync`'inkiler.
  - Uzak liste: `templates` ve her `organizations[].templates`.
  - Kurum şablonunun yüklemesi ve silmesi kurumun yoluna gider
    (`POST /v1/tenants/{kurum}/commands`). Yeni şablon ve çakışmanın kopyası kişisel alana gider.
  - Listeden düşen kurum şablonu (kurumda silindi ya da hesap kurumdan ayrıldı) cihazdan kalkar ve
    söylenir: “… artık “<kurum>” kurumunun şablonları arasında görünmüyor …”. Ondan yapılmış
    paftalar kalır.
- **Cihaz kaydı** (`product/sheet/templateStore.ts`): `organization?` (`tenantId`, `name`),
  `publishedFrom?`; `ROLES`'ta `admin`.
- **Sahte bulut** (`app/sheet/templateCloudTesting.ts`) sunucunun KURUM kurallarıyla çalışır:
  - roller; yanlış yola 422, üye olmayana 404;
  - yayımlama; kurum şablonunun paylaşımına 422;
  - `{ templates, organizations }`; olaylar kurumun bütün üyelerine.
- **`northInfo`** (`product/sheet/engine.ts`, `app/sheet/paint.ts`,
  `ui/sheet/inspector/northSection.ts`):
  - Denetçi sapmayı ve kaynağını, tarihi ve nereden geldiğini, haritanın merkezinin enlem ve
    boylamını, modeli ve yıllarını motorun `northInfo(book, sheetId, itemId, inputs)`'undan okur.
    Kitabın kopyasından okuma yolu silindi (Sapmalar 56 kapandı).
  - Satır çekirdeğin yeni yazımıyla: “6°19' D · WMM2025 · 2026-10”. Yer ya da tarih yoksa nedeni
    yazar. Tarih modelin yılları dışındaysa uyarı `inModel`'den gelir.
  - Elle girilirken ipucu modelin değerini `magneticField(lat, lon, 0, year)` ile verir: enlem ve
    boylamı artık motor veriyor.
- **Ön denetim:** `magnetic_out_of_model`'in düğmeleri artık çekirdeğin. Web'in eklediği iki düğme
  silindi (Sapmalar 57 kapandı).
- **Rust'ın bulguları (5–8. adım):**
  1. **Eksik değer işareti** artık ‹ad?›: `pageTab.ts`, `VariablesDialog.ts` (yorum ve uyarı),
     `TemplateQuestions.ts` (yorum ve giriş). İşaret tek yerde yazılı: `product/sheet/marks.ts`
     `missingMark`. `marks.test.ts` çekirdeğin kâğıda yazdığıyla aynı olduğunu sınar. Motor işareti
     vermiyor (Sapmalar 62).
  2. **Harita simgelerinin dış çizgileri** (`app/sheet/mapVectors.ts`, `pdfExport.ts` `corePaths`):
     - Yay olarak çizilen tam daire (`circle`, `ring`) artık kapalı halkadır; dolgusu PDF'te kalır
       (`Outline.rings`).
     - Çift-tek kuralında her simge bir yoldur: ilk halkası dış, gerisi delikleri (`VecPath.groups`).
       Çekirdek böyle bir yolu çift-tek doldurur. Delikli simgenin deliği boş kalır; üst üste binen
       iki simge iki dolu yoldur.
     - Ekran, küçük resimler, PNG ve SVG haritayı çizim hattının resminden alır; orada hata yoktu.
       `sheet-pdf.mjs` artık ikisini de denetler (Sapmalar 63).
  3. **Açılar:** `′` ve `″` kalmadı. `northSection.ts` ve `sheet-north.mjs` `'` ve `"` yazar, arar.
  4. **SVG resmi PDF'te** (motorun “SVG RESİM” notu; `app/sheet/pdfExport.ts`, `windows.ts`;
     `ui/sheet/exportPdf.ts`, `ExportDialog.ts`):
     - Tarayıcı her SVG resmi motorun `pdfSvgSizes`'ının verdiği pikselde çizer: `Image` →
       `OffscreenCanvas.drawImage` → PNG (`svgPng`). Resim, kâğıtta çizildiği gibi dikdörtgenine
       gerilir. PNG `PdfAsset.raster`'a girer; çekirdek onu resmin yerine gömer.
     - Dışa aktarma penceresi, yazmadan önce çekirdeğin `pdfFindings`'ini gösterir: “… SVG resim PDF'e
       resim olarak gömüldü (709 × 355 piksel). Daha keskin bir resim için PDF'i daha yüksek
       çözünürlükle yazın.” Çizilemeyen resim için uyarıyı ve çözümünü gösterir. Pencerenin
       çizdiği PNG'ler PDF'te yeniden kullanılır (resim ve boyuyla saklanır).
     - Yazınca bulgular masaüstündeki gibi günlüğe de yazılır: not bilgi olarak; uyarı çözümüyle
       birlikte.
     - Pencerenin harita yolları önbelleği modül düzeyindeydi ve anahtarında çizim yoktu. Artık her
       pencerenin kendi önbelleği var; notlarınki de öyle.
  5. **Altın PDF** (40 064 bayt, `6c194fa0f21900c1…`) tarayıcıda bayt bayt aynı çıkıyor.
  6. **Yedek harita resminin altında kâğıt yok** (`app/sheet/mapFrames.ts`): masaüstünün resmi gibi
     düz alfalı PNG'dir, PDF'te yumuşak maskeyle gömülür. Çizim arka ucunun tuvalinde alfa yok
     (`WebGL2Backend`, paylaşılan; dokunulmadı). Bu yüzden her parça bir beyaz, bir siyah zeminde
     çizilir; her pikselin ne kadar örttüğü ve rengi iki zeminin farkından çıkar (`unpapered`).
     Ekranın, küçük resimlerin, PNG'nin ve SVG'nin resmi beyaz kâğıtlı kalır.

**Notlar:**

| # | Not | Durum |
|---|---|---|
| 1 | Kurumum canlı, kurumun adıyla gruplu, “Kurum: <ad>” rozeti | tamam |
| 2 | `canPublish` olan kurum varken “Kuruma yayımla…”; kurumu kullanıcı seçer | tamam; ayrıca “Kurumdakini güncelle” |
| 3 | üyeye “Şablonlarıma kopyala” | tamam |
| 4 | düzenleme ve silme yalnız yayımlayana ve yöneticiye; görüntüleyen nedenini görür | tamam: “Kopyasını düzenle”; Sil kapalı, nedeni ipucunda; sunucu da reddediyor (403) |
| 5 | aynı `planSync` kuralları; kurumun yolu; çakışmanın kopyası kişisel alana | tamam |
| 6 | `sheet-cloud.mjs`: yönetici yayımlar; düz üye görür, kullanır, düzenleyemez; misafir ve ayrılan görmez | tamam: 17 yeni denetim, toplam 44 |
| 7 | görüntüler iki temada, iki boyda, açık önce | tamam: 11–16 |
| 8 | `northInfo`; web'in elle eklediği düğmeler kalkar | tamam |
| 9 | eksik değer işareti, tercihen motordan | tamam; motor vermediği için tek yerde, testle çekirdeğe bağlı (Sapmalar 62) |
| 10 | dış çizgi hataları, testleriyle | tamam: PDF'te düzeltildi; ekran, küçük resim, PNG ve SVG zaten doğruydu, artık denetleniyor (Sapmalar 63) |
| 11 | `'` ve `"` | tamam |
| 12 | SVG resmi PDF'te: tarayıcıda çiz, `raster`'a koy, `pdfFindings` pencerede, test ve `sheet-pdf.mjs` | tamam: 709 × 355 resim gömülü, boş kutu değil; pencere ve günlük söylüyor |
| 13 | altın PDF | tamam |
| 14 | yedek harita resmi saydam (masaüstüyle aynı) | tamam: PDF'te yumuşak maske |

**Bağlantı noktaları:** §3'te W-13 (kurum senaryosu), W-15 (dış çizgi, SVG resim ve saydam yedek
harita denetimleri), W-19 (açılar). Paylaşılan dosyaya dokunulmadı.

**Komutlar** (hepsi `flock .run/heavy.lock` ile; son hâlde):

| Komut | Sonuç |
|---|---|
| `pnpm typecheck` | temiz |
| `pnpm test` | 251 dosya geçti, 21 atlandı; 3 290 test geçti, 21 atlandı |
| `pnpm -C apps/web exec vitest run src/{product,render,tools,ui,app}/sheet` | 18 dosya, 134 test geçti. Yeniler: `templateOrgs.test.ts` (6), `marks.test.ts` (1), `mapFrames.test.ts` (1); `pdfExport.test.ts`'te dış çizgiler (2) ve SVG resim (1) |
| `pnpm build` | Giriş parçası 985,70 kB; pafta kodu 100,8 kB (E'de 98,4). Artış girişteki pafta modüllerinde: kurum sağlayıcısı ve rozetleri (`providers.ts`, `templates.ts`), kaydın alanları (`templateStore.ts`), motorun yeni sarmalayıcıları (`engine.ts`). Galeri, yayımla penceresi, eşitleme ve PDF tembel parçalarda |
| tembel yükleme: `dist/` | değişmedi: açılışta 47 kaynak; pafta parçası, `windows` ve `.ttf` yok |
| `pnpm e2e` | tüm kontroller geçti |
| `node apps/web/scripts/e2e/sheet-cloud.mjs` | 44 denetim geçti (aşağıda); geçici veritabanı durduruldu ve silindi |
| `cargo test -p kentos-desktop sheet_pdf_files -- --ignored` (`KENTOS_DEMO_DRAWING` mutlak yolla) | masaüstünün PDF'i yeniden yazıldı (12.20) |
| `node apps/web/scripts/e2e/sheet-pdf.mjs --masaustu .run/shots/sheet-pdf/masaustu-ifraz.pdf` | 30 denetim geçti (E'de 25); masaüstüyle harita 99 / 99 sözcük, 0,00 mm. 150 dpi sayfa resimlerinde harita alanında bir piksel bir gri seviyesi farklı; üstte, antette ve tabloda fark yok |
| `node apps/web/scripts/e2e/sheet-north.mjs` | 12 denetim geçti |
| `node apps/web/scripts/e2e/sheet-shots.mjs` | 1440 açıkta 30 sahne; `galeri-kurumum` iki temada, iki boyda |
| `node apps/web/scripts/e2e/sheet-a11y.mjs --strict` | bulgu yok; 452 etkileşimli öğe (E'de 451) |

**Uçtan uca: PDF** (`sheet-pdf.mjs`, yeni beş denetim):

- Delikli simge: haritanın resminde ve PDF'te ortası beyaz, halkası kırmızı. Dolu daire: ikisinde de
  ortası dolu.
- SVG resim (200 × 100, kırmızı dikdörtgen, mavi daire; 60 × 30 mm çerçeve):
  - Dışa aktarma penceresi yazmadan önce “… SVG resim PDF'e resim olarak gömülür (709 × 355 piksel).
    …” der.
  - `pdfimages -list` 709 × 355 resmi gösterir. 150 dpi sayfada çerçevenin ortası mavi, sol ucu
    kırmızı. Günlükte aynı bilgi satırı var.
- Yedek harita (desen dolgulu katman): resmin yanında aynı boyda `smask` var.

**Uçtan uca: kurum** (`sheet-cloud.mjs`). Kurulum: geçici veritabanında geliştirme tohumu.
“Örnek Harita Bürosu” (`ornek-buro`): Zeynep yönetici, Ayşe proje yöneticisi, Mehmet düz üye
(editör). Misafir `kentosd user add` ile eklenir, kurumu yok.

- Zeynep kendi şablonunu kaydeder, buluta eşitler, “Kuruma yayımla…”yı açar; pencere yalnız
  “Örnek Harita Bürosu”nu sunar.
- Pencere açıkken Zeynep'in kurumdaki rolü veritabanında “editor”e indirilir. “Yayımla”ya sunucu
  hayır der; pencere açık kalır ve sunucunun sözünü yazar: “Kuruma yayımlanamadı: Bu kurumun şablon
  kitaplığına yalnız kurum sahibi, yöneticiler ve proje açabilen üyeler şablon yayımlar; kurum
  yöneticinize başvurun.”
- Rolü geri verilince aynı pencere yayımlar:
  - Şablon Kurumum'da kurumun adı altında; rozetleri “Kurum: Örnek Harita Bürosu”, “Yayımladınız”,
    “Eşitlendi”.
  - Sunucu onu kurumun kitaplığında listeler, `publishedFrom`'u Zeynep'in kendi şablonudur.
  - Günlükte “… kurumunda yayımlandı …”.
- Mehmet onu kurumun adı altında rolsüz görür (“Kurum: …”, “Eşitlendi”) ve Kullan'la pafta yapar.
  - Düğmeleri: “Şablonlarıma kopyala”, “Kopyasını düzenle”, “Şimdi eşitle”; Sil kapalı; Paylaş yok.
  - Sunucuya değişiklik gönderirse 403 alır.
- Zeynep kendi şablonunu bu cihazda değiştirir: “Kuruma yayımla…” kapanır. “Şimdi eşitle”den sonra
  açılır.
- Zeynep “Kurumdakini güncelle” der: kurumdaki kopya 2. revizyon olur, Mehmet sormadan duyar.
- Misafir'in Kurumum'u boştur ve nedenini söyler.
- Ayşe'nin üyeliği kapatılır (`membership.status = 'disabled'`): kurum şablonu cihazından kalkar,
  ona söylenir.
- Beş tarayıcıda da yakalanmamış hata yok.

**Çekirdeğe notlar:**

- **Eksik değer işareti motordan okunabilsin:** `EngineInfo`'da bir alan, ör. `missingMark`. Web
  şimdilik işareti tek yerde yazıyor ve testle çekirdeğe bağlıyor (Sapmalar 62).
- **`svg_as_picture`'ın cümlesi geçmiş zamanda** (“gömüldü”). Web onu pencerede, yazmadan önce de
  gösteriyor. İki yere de uyan bir söz (ör. “resim olarak gömülür”) önerilir; web metni olduğu gibi
  alır, kendisi yazmaz.
- **`sheet_pdf_files`'ın yorumundaki komut göreli yolla düşüyor:** `cargo test` paketin dizininde
  (`apps/desktop`) koşar, `KENTOS_DEMO_DRAWING=.run/demo/…` “No such file or directory” verir.
  Mutlak yolla geçti. Yorum mutlak yol demeli, ya da test yolu çalışma alanının köküne göre
  çözmeli.

**Görüntüler:**

- `apps/web/scripts/e2e/out/shots/sheet-cloud/<n>-<sahne>-<light|dark>-<1440|1100>.png`, n 11–16:
  - `11-kuruma-yayimla` (pencere);
  - `12-kurumum-yayimlayan`, `13-kurumum-uye`, `14-kurumum-misafir`;
  - `15-kurumdan-ayrilan` (durum satırında iletisi);
  - `16-kurumdakini-guncelle`.
- `…/out/shots/sheet/galeri-kurumum-…`: sunucusuz Kurumum (“Sunucuya ulaşılamıyor”).
- `…/out/shots/sheet-pdf/pul-washer.pdf`, `pul-disc.pdf`: dış çizgi denetimlerinin PDF'leri;
  `ifraz-svg-resim.pdf` ve `.ppm`'i: SVG resimli sayfa; `ifraz-resimli-harita-1.png`: saydam yedek
  haritalı sayfa.

**Sapmalar:** `tasks-web.md` “Sapmalar” 62–64.

**Bekleyen:** yok.

#### E bölümü (3 Ekim)

**Yapılanlar:** manyetik kuzey (tasarım §8a; motorun “WMM” notu, 6. adım).

- **Kuzey okunun denetçisi** (`ui/sheet/inspector/northSection.ts`):
  - Kuzey: Grid, Coğrafi, Manyetik.
  - Biçim: KentOS “K”, Basit ok, Pusula, Kuzey çizelgesi (`diagram`).
  - Ok manyetik kuzeyi gösterdiğinde (çizelge hep gösterir) **Manyetik sapma** satırı kâğıdın
    yazdığını kaynağıyla söyler: “6°19′ D · WMM2025 · 2026-10”. Değer çekirdeğin yazısından
    okunur (Sapmalar 56). Altında modelin adı ve geçerlilik yılları yazar.
  - **Hesap tarihi:** tarih ve nereden geldiği: paftanın “tarih” değişkeni, projeninki, ya da
    “bugün: paftada ve projede tarih değişkeni yok”. Yanında “Değişkenler…” düğmesi değişkenleri
    açar. Tarih modelin yılları dışındaysa “değer yaklaşıktır” yazar (`decimalYear`, `wmmInfo`).
  - **Sapmayı elle gir:** açılınca değer modelden başlar (Sapmalar 58). “Elle sapma” derece,
    “Sapmanın yılı” isteğe bağlı. Elle girilirken satır “6°19′ D · elle” der, ipucu modelin
    değerini verir. B'deki alanın birim hatası düzeldi (Sapmalar 59).
- **Ön denetim:**
  - `magnetic_no_place`: çekirdeğin iki düzeltmesi. “Koordinat sistemi seç” projenin ayarlarını
    koordinat sistemiyle açar (`crs.set`); “Sapmayı elle gir” işlemi uygular.
  - `magnetic_out_of_model`: çekirdek düzeltme vermediği için web iki düğme ekler: “Değişkenleri
    aç”, “Sapmayı elle gir” (Sapmalar 57).
  - `magnetic_no_date`: çekirdeğin “Değişkenleri aç”ı (eski yol).
- **Motor sarmalayıcıları** (`product/sheet/engine.ts`): `magneticField`, `wmmInfo`, `decimalYear`;
  ayrıca motora yeni gelen `exportName`. PDF'in dosya adı artık ondan gelir (Sapmalar 60).
- **Görüntü sahneleri:** `e-kuzey-cizelgesi`, `e-manyetik-on-denetim`; iki temada, iki boyda,
  açık tema önce.

**Notlar:**

| # | Not | Durum |
|---|---|---|
| 1 | denetçi: kuzeyin türü, hesaplanan sapma ve kaynağı | tamam: “6°19′ D · WMM2025 · 2026-10”, kâğıtla aynı (Sapmalar 56) |
| 2 | “Sapmayı elle gir”, “Elle sapma” ve yıl | tamam |
| 3 | biçim, “Kuzey çizelgesi” dahil | tamam |
| 4 | hangi tarihe göre ve nasıl değişir | tamam: kaynak yazılı, “Değişkenler…” |
| 5 | iki bulgu, gerçek eylemlerle | tamam: dört düğme de uçtan uca tıklandı; biri web'in eki (Sapmalar 57) |
| 6 | sahne, açık önce | tamam |

**Bağlantı noktaları:** §3'te W-19 (yeni betik). Paylaşılan dosyaya dokunulmadı.

**Komutlar** (hepsi `flock .run/heavy.lock` ile; son hâlde):

| Komut | Sonuç |
|---|---|
| `pnpm typecheck` | temiz. Arada Rust tarafı `SheetTemplateList`'e `organizations` ekledi; sahte bulut uyarlandı (Sapmalar 61) |
| `pnpm test` | 248 dosya geçti, 21 atlandı; 3 279 test geçti, 21 atlandı |
| `pnpm -C apps/web exec vitest run src/{product,render,tools,ui,app}/sheet` | 15 dosya, 123 test geçti; yeni `north.test.ts` 6 test |
| `pnpm build` | Giriş parçası 983,76 kB; pafta kodu 98,4 kB (D+'da 98,0). Artış `engine.ts`'teki dört sarmalayıcı. Denetçi ve ön denetim tembel parçalarda |
| tembel yükleme: `dist/` | değişmedi: açılışta 47 kaynak, pafta parçası, `windows` ve `.ttf` yok |
| `pnpm e2e` | tüm kontroller geçti |
| `node apps/web/scripts/e2e/sheet-north.mjs` | 12 denetim geçti (aşağıda) |
| `node apps/web/scripts/e2e/sheet-pdf.mjs --masaustu …` | 25 denetim geçti; masaüstüyle harita 99 / 99 sözcük, 0,00 mm |
| `node apps/web/scripts/e2e/sheet-shots.mjs` | 1440 açıkta 30 sahne, hata yok; yeni iki sahne iki temada, iki boyda |
| `node apps/web/scripts/e2e/sheet-a11y.mjs --strict` | bulgu yok; yeni sahne “manyetik kuzey oku seçili” (80 öğe), toplam 451 |

**Uçtan uca** (`sheet-north.mjs`). Kurulum: demo çizimde ifraz paftası; kuzey oku manyetik,
paftanın “tarih”i 2026-10-03.

- Denetçi “6°19′ D · WMM2025 · 2026-10” der; tarihi “2026-10-03 · paftanın “tarih” değişkeni”.
- “Sapmayı elle gir” açılınca kitapta `declinationHand: true` olur, değer 6317 milliderece. Kâğıt “Manyetik
  sapma 6°19′ D (elle)” yazar.
- Kuzey çizelgesinde kâğıt şunları yazar, PDF'te de (`pdftotext`):
  - GK, CK, MK;
  - “GK–CK yakınsama −0°06′”;
  - “CK–MK manyetik sapma 6°19′ D (WMM2025, 2026-10)”;
  - “GK–MK açısı 6°25′ D”.
- Tarih 2031-03-01 olunca uyarı ve iki düğmesi çıkar:
  - “Değişkenleri aç” Değişkenler penceresini açar;
  - “Sapmayı elle gir” uyarıyı kaldırır.
- Proje sistemi TUREF coğrafi olunca hata ve iki düğmesi çıkar:
  - “Koordinat sistemi seç” Proje ayarlarını açar;
  - “Sapmayı elle gir” hatayı kaldırır.
- Konsolda hata yok.

**Çekirdeğe notlar:**

- **Kuzey çizelgesinin yazıları dar kalıyor.** İfraz şablonunun 36 mm'lik kuzey oku çerçevesine
  sığmıyor: “CK–MK manyetik sapma 6°19′ D (WMM2025, 2026-10)” satırı sağ kenara taşıyor (ekranda
  ve PDF'te). Ön denetim bunu söylemiyor.
- **Önerilen motor işlevi:** `northValues` (Sapmalar 56).
- **Önerilen çekirdek düzeltmeleri:** `magnetic_out_of_model` için düzeltmeler (Sapmalar 57).

**Görüntüler:**

- `apps/web/scripts/e2e/out/shots/sheet/e-kuzey-cizelgesi-<light|dark>-<1440|1100>.png`;
- `…/sheet/e-manyetik-on-denetim-…`;
- `…/shots/sheet-north/`:
  - `denetci-model.png`, `denetci-elle.png`;
  - `on-denetim-model-disi.png`, `on-denetim-yer-yok.png`;
  - `kuzey-cizelgesi-1.png` (PDF'ten, 150 dpi).

**Sapmalar:** `tasks-web.md` “Sapmalar” 56–61.

**Bekleyen:** F bölümü `.run/sheet-engine-ready`'de “KURUM” notunu bekliyor.

#### D+ (3 Ekim)

Koordinatörün motordan bağımsız dört işi.

1. **Harita yazısının çerçeve kuralı.** Dayanağı çerçevenin içindeki yazı yazılır ve çerçevede
   kesilir; dışındaki yazılmaz. Dayanağın tanımı: Sapmalar 52.
   - Kural tek yerdedir: `app/sheet/frameLabels.ts`.
   - Aynı kayıtları alanlar: ekrandaki harita çerçeveleri, galerinin küçük resimleri, PNG ve SVG
     resimleri, PDF'in vektör yazıları (`labelSpots`).
   - Test: `frameLabels.test.ts`. Dönmüş çerçeveyi sınar; metin kendi noktasıyla, çizginin adı
     parçanın ortasıyla yazılır; katman listesi önce süzer.
   - Uçtan uca: harita 30° döndürüldü. Çekirdeğin NEATLINE köşelerinden 1 cm içerisi
     `insideFrame`'e göre içeride, 1 cm dışarısı dışarıda. Dönüşün yönü de doğru.
   - Masaüstünün 08.11'de yazılan PDF'iyle harita yazıları aynıdır: 99 / 99 sözcük, ortak 35 yazı
     0,00 mm farkla. 150 dpi'da harita kutusunda farklı piksel yok. D raporundaki altı harita
     farkı kapandı.
2. **Ctrl+P / Yazdır ve pafta** (W-17):
   - Pafta öndeyken `file.print` paftanın Yazdır penceresini açar ve “Geliştirme aşamasında”
     demez. Model öndeyken eskisi gibidir.
   - Pafta kipindeki Ctrl+P bağı (`sheet.print`) duruyor; şeritteki Yazdır'ın ipucu kısayolu
     gösterir.
3. **Erişilebilirlik:** `sheet-a11y.mjs` (W-18).
   - Sahneler:
     - Model | Pafta sekmeleri;
     - pafta kipi dört hâlde: seçimsiz, harita seçili, Sayfa, Ön denetim;
     - Pafta şeridi ve durum hücreleri;
     - dışa aktarma penceresi;
     - galeri iki hâlde: Sistem; Benim, şablon seçili;
     - paylaşım penceresi.
   - Toplam 371 etkileşimli öğe.

   | Bulgu | Önce | Sonra |
   |---|---|---|
   | adsız etkileşimli öğe | 4: denetçinin katlama düğmesi, dört sahnede | 0 |
   | birleşik ya da açıklama taşıyan ad | 12: paftalar listesinin satırları (resmin adı ve yazı, iki kez); galerinin iki kaynağı, notlarıyla | 0 |
   | odaklanıp rolsüz | 0 | 0 |
   | yalnız işaretçiyle | 0 | 0 |
   | Tab'da bileşik denetimde fazladan durak | 10: galerinin 4 kaynağı ve 8 kartı | 0 |
   | Tab'da pencere dışına, görünmeyene | 0 | 0 |

   - “Önce”nin ad sütunu, ilk koşunun kaydettiği adlardan son kuralla sayıldı. Kuralın kendisi
     sezgiseldir: harfle bitişik rakam ya da büyük harf, 60 karakterden uzun ad.
   - Tab gezintileri:
     - dışa aktarma penceresi 8 durak: Kapat, Biçim, Hatalar varken yine de aktar, Paftalar,
       GeoPDF, Katmanlar, Çözünürlük, Vazgeç. Hatalar varken Yeni sekmede aç, Yazdır ve Kaydet
       kapalıdır, bu yüzden durak değildir;
     - galeri 18 duraktan 8'e indi.
   - Oklar galerinin kaynaklarında, kartlarında ve denetçinin sekmelerinde çalışıyor.
   - Düzeltilen dosyalar: `SheetInspector.ts`, `TemplateGallery.ts`, `SheetsPanel.ts`,
     `sheet.css`.
   - **Paylaşılan dosya, düzeltilmedi:** `ui/dock/Panel.ts`. Sekmeli bir panelin katlama düğmesi
     adını kaybeder, çünkü başlık `styles/panels.css`'te `display: none` olur. Aynı durum ana
     uygulamanın sağ dokunda da var: Katmanlar, İşlem, Bloklar. Tek satırlık düzeltme: düğmeye
     `aria-label: title`.
   - Ekran okuyucuyla deneme yapılmadı.
4. **Bulut görüntüleri iki boyda:** `sheet-cloud.mjs` her sahneyi 1440×900 ve 1100×650'de, açık
   tema önce alır (Sapmalar 54). 26 denetim geçti, 40 resim çıktı. Kap durduruldu ve silindi.

**Komutlar** (hepsi `flock .run/heavy.lock` ile; son hâlde):

| Komut | Sonuç |
|---|---|
| `pnpm typecheck` | temiz |
| `pnpm test` | 247 dosya geçti, 21 atlandı; 3 273 test geçti, 21 atlandı |
| `pnpm -C apps/web exec vitest run src/{product,render,tools,ui,app}/sheet` | 14 dosya, 117 test geçti |
| `pnpm build` | Giriş parçası 983,54 kB (D'de 983,25 kB). İçindeki pafta kodu 98,0 kB (D'de 97,8 kB); artış `install.ts`'teki `frontPrint` bağı. `frameLabels.ts` tembel `templateActions` parçasında |
| tembel yükleme: `dist/` vite preview ile | değişmedi: açılışta 47 kaynak; pafta parçası, `windows` ve `.ttf` yok. Galeri açılınca da `windows`, PDF kodu ve `.ttf` yok |
| `pnpm e2e` | tüm kontroller geçti |
| `node apps/web/scripts/e2e/sheet-pdf.mjs --masaustu .run/shots/sheet-pdf/masaustu-ifraz.pdf` | 25 denetim geçti: D'nin 21'i; dönmüş harita; `file.print`; masaüstüyle 2 karşılaştırma. Altın PDF yeni özetiyle tuttu (40 101 bayt) |
| `node apps/web/scripts/e2e/sheet-shots.mjs --themes light --sizes 1440x900` | 28 sahne, hata yok |
| `node apps/web/scripts/e2e/sheet-a11y.mjs --strict` | bulgu yok |
| `node apps/web/scripts/e2e/sheet-cloud.mjs` | 26 denetim geçti |

**Görüntüler:**

- `apps/web/scripts/e2e/out/shots/sheet-cloud/<n>-<sahne>-<light|dark>-<1440|1100>.png`;
- `…/shots/sheet-pdf/ifraz-donuk-harita-1.png`: 30° dönmüş harita, yazılar dayanaklarıyla;
- `…/out/a11y/sheet-a11y.json`: her sahnenin adları ve Tab yolları.

**Sapmalar:** `tasks-web.md` “Sapmalar” 52–55.

**Bekleyen:** E ve F bölümleri `.run/sheet-engine-ready`'de “WMM” ve “KURUM” notlarını bekliyor.

#### D bölümü (3 Ekim)

**Yapılanlar.** Paftalar web'de PDF ve GeoPDF olarak kaydedilir, yazdırılır ya da yeni sekmede
açılır.

- Yazıcı çekirdeğinkidir (`toPdf`; masaüstü de onu kullanır).
- Yazı tipleri masaüstünün dosyalarıdır.
- Haritalar web'in kendi çizim hattından vektör olarak gider.

Yeni dizinlerde 7 yeni dosya, 1 435 satır (196'sı test); `ExportDialog.ts` yeniden yazıldı. Ayrıca
yeni betik `sheet-pdf.mjs` (W-15).

- `app/sheet/pdfExport.ts`: PDF'i toplar ve çekirdeğe yazdırır.
  - Girdiler: seçilen her paftanın dışa aktarma girdileri, tek `RenderInputs`'ta birleşir.
  - Haritalar: her harita çerçevesi bir kez gider (ana sayfanınki de), vektör olarak. Bir katman
    vektörle yazılamıyorsa harita seçilen dpi'da PNG olur (Sapmalar 45).
  - Resimler: kitabın resimleri SHA-256'larıyla, lejant satırlarının simgeleri boyayıcının
    PNG'leriyle.
  - GeoPDF: projenin sistemi WKT olarak gider (`tmWkt`). TM ve UTM dışında kapalıdır (Sapmalar 46).
  - Yazı tipleri: çekirdeğin istediği yüzler (`pdfFonts`), yalnız onların dosyaları. Dosyası
    gelmeyen yüzü çekirdek adıyla söyler (`pdf_font_missing`).
  - Sonuç üç yoldan biriyle gider: Kaydet (dosya penceresi), Yazdır (gizli çerçeve), Yeni sekmede
    aç.
  - Dosya adı paftanın dışa aktarma varsayılanından gelir, motorun yazısıyla (Sapmalar 41).
- `app/sheet/mapContent.ts`, `mapVectors.ts`, `mapLabels.ts`: haritanın vektörleri, kâğıt
  ölçüleriyle.
  - Kaynak, harita çerçevelerinin stilli katmanlarıdır (`buildStyledLayer`).
  - Çizgiler yol ve halka olur, alanlar delikleriyle. Taramalar alana kırpılır.
  - Şekil simgeleri gölgelendiricinin koyduğu yerde çizilir, `render/canvasShapes.ts`'in
    şekilleriyle (W-14).
  - Yazılar ve maskeleri `drawLabels`'ın yerleşimini ve seyreltmesini izler.
  - Önce bütün çizgiler, sonra bütün yazılar gider. Her çizim katmanı bir PDF katmanıdır
    (Sapmalar 43).
- `app/sheet/pdfFonts.ts`: masaüstünün 22 TTF'i tembel varlıklardır (W-16). Yalnız PDF yazılırken
  ve yalnız istenenler indirilir.
- `product/sheet/engine.ts`: `toPdf`, `pdfFonts` ve `tmWkt` sarmalayıcıları. `toPdf`'in fırlattığı
  “kod: ileti” `SheetEngineError` olur.
- `ui/sheet/ExportDialog.ts` + `exportPdf.ts`: dışa aktarma penceresi.
  - Biçim: PDF (varsayılan), SVG, PNG, .kpafta.
  - Ön denetim biçimin hemen altındadır; gidecek bütün paftaları kapsar (Sapmalar 51).
  - Paftalar: Bu pafta, Seçtiklerim, Hepsi. Her pafta bir sayfadır.
  - GeoPDF (TM ve UTM'de açık); çizim katmanlarını PDF katmanı yapma seçeneği.
  - Her harita için nasıl gideceği: vektör, ya da kaç dpi resim, hangi katman ve neden.
  - Dosya adı ve yedek resimlerin çözünürlüğü (Sapmalar 50).
  - Düğmeler: Vazgeç, Yeni sekmede aç, Yazdır, Kaydet. Yazdır'la açılınca birincil düğme Yazdır'dır.
- Komutlar:
  - `sheet.export.pdf` (“PDF olarak…”), Dışa aktar menüsünün başında;
  - `sheet.print` (“Yazdır…”), Çıktı grubunda büyük düğme; pafta öndeyken Ctrl+P (Sapmalar 48).
- `app/sheet/mapFrames.ts`: piksel birimleri artık kâğıdın CSS pikselidir; vektör ve resim harita
  aynı boyu verir (Sapmalar 42).

**Notlar:**

| # | Not | Durum |
|---|---|---|
| 1 | dışa aktarma penceresi | tamam: paftalar, yedek dpi, GeoPDF, katmanlar, önce ön denetim, dosya adı varsayılandan |
| 2 | yazı tipleri | tamam: masaüstünün dosyaları; yalnız dışa aktarırken ve yalnız istenenler indirilir; altın PDF tarayıcıda bayt bayt tutar |
| 3 | vektör harita | tamam: katman başına PDF katmanı. Yedek PNG ve penceredeki açıklaması uçtan uca denendi |
| 4 | Yazdır ve yeni sekme | tamam: şerit ve Ctrl+P. Başsız tarayıcıda yazdırma penceresinin kendisi görülemez (“Yapılmayanlar”) |
| 5 | testler, araçlarla denetim, masaüstüyle karşılaştırma, görüntüler | tamam (aşağıda) |
| 6 | giriş parçasına PDF kodu girmez | tamam |

**Bağlantı noktaları:** §3'te üç yeni satır:

- W-14: `canvasShapes.ts`, +9 / −2;
- W-15: yeni betik;
- W-16: masaüstünün yazı tipi dosyaları (değişmedi).

**Komutlar** (hepsi `flock .run/heavy.lock` ile; son hâlde):

| Komut | Sonuç |
|---|---|
| `pnpm typecheck` | temiz |
| `pnpm test` | 246 dosya geçti, 21 atlandı; 3 271 test geçti, 21 atlandı |
| `pnpm -C apps/web exec vitest run src/{product,render,tools,ui,app}/sheet` | 13 dosya, 115 test geçti (PDF 10) |
| `pnpm build` | Vite'ın giriş parçası (983 kB) için büyük parça uyarısı dışında temiz. Giriş parçasındaki pafta kodu 97,8 kB (küçültmeden önce, %7,0; C'de ~97 kB). Artış iki komutun bildirimi, şerit satırı ve Ctrl+P'dir. PDF kodu `windows` parçasındadır: `pdfExport`, `mapContent`, `mapVectors`, `mapLabels`, `pdfFonts`, `ExportDialog`, `exportPdf`. `mapFrames` `templateActions` parçasındadır (rollup modül listesiyle denetlendi). 22 TTF ayrı varlıktır, toplam 1,8 MB |
| tembel yükleme: `dist/` vite preview ile, başsız Chrome | açılışta 47 kaynak; pafta parçası, `windows` ve `.ttf` yok. Galeri açılınca da `windows`, PDF kodu ve `.ttf` yok |
| `pnpm e2e` | tüm kontroller geçti |
| `node apps/web/scripts/e2e/sheet-shots.mjs` | 1440×900 açık temada bütün sahneler (28) hatasız. Yeni iki sahne iki temada, iki boyda |
| `node apps/web/scripts/e2e/sheet-pdf.mjs --masaustu .run/shots/sheet-pdf/masaustu-ifraz.pdf` | 23 denetim geçti: 21 PDF denetimi ve masaüstüyle 2 karşılaştırma (aşağıda) |

**PDF denetimleri** (`sheet-pdf.mjs`). Demo çizimde ifraz sistem şablonu kullanıldı; il, ilçe,
mahalle, ada, pafta, kontrol eden ve onaylayan dolu.

- `pdfinfo`: sayfa 1190.55 × 841.89 pt (A3 yatay); Title “İfraz paftası”, Producer KentOS.
- `pdffonts`: Barlow 400, 400 italik, 500, 600; hepsi gömülü, alt küme ve ToUnicode'lu.
- `pdftotext`: “İFRAZ / TEVHİT PAFTASI”, başlıklar ve yerin değerleri çıkar. Parsel tablosunun 27
  değerinin hepsi de çıkar.
- `gdalinfo`:
  - TUREF / TM36 (EPSG:5256);
  - NEATLINE harita çerçevesinin yeri üzerinde, 0,021 mm içinde; sayfa köşeleri 0,00 cm;
  - `-mdd LAYERS`: 12 çizim katmanı.
- Yazı tipleri: ilk dışa aktarmadan önce hiç `.ttf` istenmedi. Sonra yalnız gömülen dört yüzün
  dosyası indirildi.
- Altın PDF (`fixtures/sheet/v1/pdf`): tarayıcıda, uygulamanın indirdiği dosyalarla, Rust'ınkiyle
  bayt bayt aynı (40 245 bayt). Altın dosya koşular arasında Rust ajanınca güncellendi; web
  değişmeden yine tuttu.
- Pencere:
  - Pafta öndeyken klavyeden Ctrl+P pencereyi “Yazdır: İfraz paftası” olarak açar. Birincil düğme
    Yazdır'dır; GeoPDF açık, harita vektör.
  - Yazdır'ın çerçevesindeki, yeni sekmedeki ve Kaydet'in yazdığı PDF, doğrudan yazılanla aynı
    bayttır. Adı “İfraz paftası.pdf”.
  - Hepsi: iki sayfa (A3 yatay, A4 dikey), adı “Ornek_1244-1249_Ada paftaları.pdf”.
- Yedek resim: Yapı katmanına desen dolgusu verildi.
  - Pencere şunu der: “Harita” 300 dpi resim olarak gider. Vektörle yazılamayanlar: Yapı (desen
    dolgusu).
  - PDF'te harita 3047 × 2953 piksellik tek resimdir; NEATLINE yerinde.
- Coğrafi sistemde (TUREF, 5252) GeoPDF kapalıdır ve nedeni yazılıdır.
- Lejant: satırların simgeleri PDF'te resimdir (11 resim). Katman adları yazıdır.
- Sayfanın konsolunda hata yok.

**Masaüstüyle karşılaştırma.** Kaynak `.run/shots/sheet-pdf/masaustu-ifraz.pdf`: Rust ajanının
07.37'de yazdığı, aynı çizimde aynı şablon. Karşılaştırma `sheet-pdf.mjs --masaustu` ile
yinelenebilir.

- **Çekirdeğin çizdikleri aynı.**
  - Sayfa, yazı yüzleri, NEATLINE, 12 PDF katmanı ve sıraları aynı.
  - Haritanın dışındaki 67 sözcük aynı yerde (0,000 mm): antet, tablo, koordinat yazıları, kuzey
    oku, ölçek çubuğu.
  - 150 dpi'da haritanın dışında yalnız imza hücreleri ve sayfa numarası farklı (değerler). Antet,
    kuzey oku, ölçek çubuğu, parsel tablosu ve çerçevenin koordinat bantlarında farklı piksel yok.
- **Farklı olan yalnız paftanın değerleri** (girdi, yazıcı değil):
  - masaüstünde `@kullanici` “cihad”, kontrol eden ve onaylayan boş, “1 / 1”, Author “cihad”;
  - web koşusunda kontrol eden ve onaylayan dolu, oturum yok (‹kullanici?›), kitapta iki pafta
    (“1 / 2”), Author boş.
- **Harita içi farklar.** İki uygulamanın harita çiziminden gelir:
  1. **Çerçeveyi kesen yazılar:** web kesilmiş olarak çizer (ekranındaki gibi); masaüstü hiç
     çizmez. Örnekler: “1244 ada”, “1428. Sokak”, “898.97”, “899.11”, kenardaki parsel
     numaraları.
  2. **Ortak yazıların yeri:** 33 yazı aynı boyda; yerleri en çok 0,29 mm (ada yazıları,
     dikeyde), ortanca 0,14 mm farklı.
  3. **Yazı rengi:** web #111111 (kâğıt paleti); masaüstü #36414d (parsel numaraları) ve #1e2833
     (ada yazıları).
  4. **Çizgi kalınlığı:** web katmanın kalınlığını kullanır; ada sınırı 300 dpi'da 4 piksel.
     Masaüstü her çizgiyi 0,18 mm çizer (`sheet_pdf.rs` `LINE_MM`); ada sınırı 2 piksel.
  5. **Çizgi tipi:** web katmanın desenini çizer (Yol ekseni nokta-çizgi); masaüstü düz çizer.
  6. **Nokta işaretleri:** masaüstü üçgenin ve halkanın ortasına nokta koyar, daha küçük çizer. Web
     yalnız dış çizgiyi çizer.

  4 ve 5 çizimin stilidir; masaüstünün katman kalınlığını ve desenini alması önerilir. Karar
  koordinatörün.

**Çekirdeğe notlar:**

- **Yer tutucunun ayraçları.** İlk PDF'lerde ‹ad?› yerine ⟨ad?⟩ (U+27E8/9) yazılıyordu. Bu
  karakterler çizim yüzlerinde olmadığından boş kutu çıkıyordu. Çekirdek artık ‹› (U+2039/A)
  yazıyor (`expr.rs`). Son PDF'te doğru çıkıyor (`pdftotext`: “‹kullanici?›”); iş yok.
- **Dosya adı:** motorda tek başına `exportName` işlevi olsa web'in geçici metin yolu kalkar
  (Sapmalar 41).

**Görüntüler:**

- `apps/web/scripts/e2e/out/shots/sheet/d-pdf-disa-aktar-<light|dark>-<1440|1100>.png`: Yazdır'la
  açılan pencere.
- `…/sheet/d-pdf-resim-yedegi-…`: Seçtiklerim, iki pafta, yedek resim uyarısı.
- `…/shots/sheet-pdf/` (`pdftoppm -r 150`):
  - `ifraz-paftasi-1.png`;
  - `ifraz-resimli-harita-1.png`;
  - `ifraz-lejantli-1.png`;
  - `masaustu-1.png`.

  PDF'ler de bu dizinde.

**Sapmalar:** `tasks-web.md` “Sapmalar” 41–51.

**Yapılmayanlar:**

- **Tarayıcının yazdırma penceresi** başsız Chrome'da görülemez. Denetlenen: çerçevedeki PDF aynı
  bayt, yedek sekme gerekmedi.
- **Gerçek tarayıcılarda elle deneme** yapılmadı: Chrome, Firefox, Safari.
- **`pnpm inventory`**: W-7; iki yeni komut.
- **`file.print`'in pafta yazdırmaya bağlanması** birleştirmeye kaldı (Sapmalar 48).
- **GeoPDF TM ve UTM dışında** yazılmıyor (Sapmalar 46).
- **E ve F bölümleri:** `.run/sheet-engine-ready`'de “WMM” ve “KURUM” notları yok (son bakış 3 Ekim
  07.54). Notlar gelince başlanır.
- **Ekran okuyucu**, B'deki gibi.

#### C bölümü (3 Ekim)

**Yapılanlar.** Şablon kitaplığı buluta bağlandı: eşitleme, canlı paylaşım, çevrimdışı kullanım,
kullanırken sorulan bilgiler. Yeni dizinlerde 100 dosya, ~16 000 satır (1 766'sı test).

- `app/sheet/cloudApi.ts`: kitaplığın uçları ve komut zarfı. Komutlar kişisel alana gider,
  projesiz. İstekler uygulamanın bulut istemcisinden (W-12).
- `app/sheet/templateSync.ts`: eşitleme.
  - Plan motorun `planSync`'i; eylemleri sırayla burada yapılır: indir, yükle (`expectedRevision`
    ile), oluştur, çakışmada kopyayı yükle, yerelden kaldır, “bu cihazda” bırak, bulutta sil, bulutu
    geri getir.
  - Bulutun verdiği kimlik alınır; eski kimlik ve revizyonu kayıtta kalır (Sapmalar 33).
  - Komut anahtarı içerikten türer: cevabı kaybolan istek aynı anahtarla gider, çift şablon olmaz.
  - Ağ hatası koşuyu durdurur; yarım kalan eylem hiçbir şeyi değiştirmez. 5, 15, 30, 60 sn sonra
    yeniden dener.
  - Değişiklik olayları uzun yoklamayla dinlenir (`…/events`, 8 sn; Sapmalar 31). Gelen olay bir
    koşu başlatır, içerik taşımaz.
- `app/sheet/cloudLibrary.ts`: eşitlemeyi uygulamaya bağlar.
  - Girişte koşar ve dinler; çıkışta durur; sunucu yeniden cevap verince koşar.
  - “Buluta eşitle”, burada kaydedilen değişiklik, silme (hiç yüklenmemişse hemen; yüklenmişse
    işaretlenir, bulut duyunca kalkar), erişim listesi, kişi araması, paylaşma, paylaşımı kaldırma.
- `app/sheet/templateShelf.ts`, `providers.ts`: galerinin kaynakları.
  - Benim: bu cihazın ve hesabın kendi bulut şablonları. Benimle paylaşılanlar: rolüyle ve sahibiyle.
  - Kopyalar hesabındır (Sapmalar 32).
  - Rozetler: Bu cihazda, Eşitlendi, Eşitleniyor, Değişti/eşitlenmedi, Çakışma, Paylaşıldı,
    Görüntüleyebilir, Düzenleyebilir.
  - Bir paftanın şablonu eski kimliğiyle de bulunur, “Yeni sürüm var” için.
- `product/sheet/templateStore.ts`: cihazdaki şablon deposu `store.ts`'ten ayrıldı. Her kayıtta bulut
  durumu (`TemplateCloudState`) var; okunamayan durum kaydı yalnız bu cihazda bırakır, içeriği korur.
- `ui/sheet/ShareTemplateDialog.ts` + `templateFinder.ts`: paylaşım penceresi canlı.
  - Kişi sunucunun aramasıyla bulunur (ADR 0024; en az iki harf; oklar, Enter, Esc).
  - Roller: Görüntüleyebilir, Düzenleyebilir. Satırda rol değişir; Kaldır önce sorar.
  - Erişimi olanların listesi var.
  - Sunucunun reddi kendi sözüyle yazılır (`failureText`).
  - Yalnız bu cihazdaki şablonda “Paylaşmak için önce buluta eşitleyin.” ve “Buluta eşitle”;
    eşitlenince pencere eşitlenen şablonla canlı devam eder.
  - Sahibi olmayan için “Yalnız şablonun sahibi paylaşır.”
- `ui/sheet/TemplateQuestions.ts`: şablonun soruları, pafta yapılmadan önce (koordinatörün notu 6).
  - Her `Template.variables` için türüne göre bir alan; şablonun değeriyle dolu.
  - Kullan'da ve Yeni pafta'da açılır. Vazgeç pafta yapmaz; boş bırakılan “(ad?)” olur.
- Galeri:
  - durum satırı kitaplığın durumunu söyler (eşitlendi ve saati, eşitleniyor, çevrimdışı);
  - ayrıntıda “Saklandığı yer”; çakışma kopyasında açıklama;
  - eşitlenen şablon kimlik değiştirse de seçili kalır;
  - tuşları `galleryKeys.ts`'e taşındı.
- Durum çubuğu: eşitleme yolunda değilken bir hücre (Sapmalar 38). Paftalar listesinde ve Sayfa
  sekmesinde “Yeni sürüm var”.
- `@kullanici` girişliyken kullanıcının adıdır (B'den beri; uçtan uca denetlendi).
- **Tembel yükleme (not 4):** `withEngine.ts` motorla birlikte yüklenir. Giriş parçasından
  çıkanlar:
  - `paint.ts`, `mapFrames.ts`, `inputs.ts`, `templateActions.ts`, `pictures.ts`,
    `render/sheet/painter.ts`;
  - bulut kitaplığı (`cloudLibrary`, `templateSync`, `cloudApi`).

  Servis boyayıcıyı ve eylemleri motor gelince kurar. O zamana dek kâğıdın kaynakları boştur; bir
  şey çizilmez de.
- **`.kpafta` tek kodek (not 5):** çekirdeğin `encodeKpafta` / `decodeKpafta`'sı. TypeScript'teki
  biçim (`KPAFTA_FORMAT`, `KpaftaFile`, `readKpafta`) silindi. WASM sınırında `fixtures/sheet/v1/kpafta/`
  sınanır: geçerliler okunup yazılınca aynı; bozuk 10 dosya kendi koduyla reddedilir.
- **Galeri cilası:**
  - (a) “Kullanılabildiği kâğıtlar” yerine “Önerilen kâğıt(lar)”; yeni “Yerleşim düzenleri” satırı
    motorun verisinden: her düzenin adı ve koşulu, örneğin “A4 yatay · yatay; genişlik en çok
    300 mm”.
  - (b) “Benim'e kopyala” yerine “Şablonlarıma kopyala” (Sapmalar 39).

**Notlar:**

| # | Not | Durum |
|---|---|---|
| 1 | eşitleme, rozetler, çakışma, çevrimdışı, yeniden bağlanma, olaylar, “Yeni sürüm var”, sessiz kayıp yok | tamam: birim testleri (her §13 satırı, kayıp cevap, çevrimdışı, hesaplar, olaylar) ve uçtan uca |
| 2 | canlı paylaşım | tamam: arama, roller, erişim listesi, kaldırma, Benimle paylaşılanlar, eşitlenmemişte uyarı. Ret sunucunun sözüyle yazılır (`failureText`, projenin penceresindeki gibi); uçtan uca bir ret denenmedi |
| 3 | gerçek sunucuyla uçtan uca | tamam: geçici docker veritabanı; iki kullanıcı, iki cihaz, çakışma, çevrimdışı ve dönüş; iki temada 20 resim |
| 4 | boyama kodu tembel | tamam: aşağıdaki “Komutlar” |
| 5 | tek `.kpafta` kodeği | tamam: çekirdeğinki; TS kopyası silindi |
| 6 | kullanırken sorular, `@kullanici` | tamam |
| a, b | galeri cilası | tamam |

**Bağlantı noktaları:** §3'te W-12 (yeni, `api.ts` +11) ve W-13 (yeni betik); W-9 güncellendi.

**Komutlar** (hepsi `flock .run/heavy.lock` ile; son hâlde):

| Komut | Sonuç |
|---|---|
| `pnpm typecheck` | temiz |
| `pnpm test` | 245 dosya geçti, 21 atlandı; 3 261 test geçti, 21 atlandı |
| `pnpm -C apps/web exec vitest run src/{product,render,tools,ui,app}/sheet` | 12 dosya, 105 test geçti (eşitleme 11, kitaplık 4, `.kpafta` sınırı 1) |
| `pnpm build` | temiz. Giriş parçasındaki pafta kodu ~122 kB'tan ~97 kB'a indi (küçültmeden önce, parçanın %6,9'u). Tembel `withEngine` 25,9 kB (gzip 9,7 kB). Boyama, harita çerçeveleri, girdiler, şablon eylemleri ve bulut kitaplığı giriş parçasında yok (rollup modül listesiyle denetlendi) |
| tembel yükleme: `dist/` vite preview ile, başsız Chrome | açılışta 47 kaynak, tek WASM geometri çekirdeği; pafta parçası yok. İlk “Şablondan pafta…”: `TemplateGallery`, `paperPainter`, `painter`, `withEngine`, `kentos_sheet_wasm.js` ve `.wasm` |
| `pnpm e2e` | tüm kontroller geçti; konsolda hata yok |
| `node apps/web/scripts/e2e/sheet-shots.mjs` | 104 resim, hata yok; yeni sahne `sablon-sorulari` |
| `cargo build -p kentos-api --bin kentosd` | temiz (uçtan uca için; kaynak değişmedi) |
| `node apps/web/scripts/e2e/sheet-cloud.mjs` | 26 denetim geçti; kap durduruldu ve silindi (aşağıda) |

**Uçtan uca** (`sheet-cloud.mjs`):

- Kurulum:
  - gerçek kentosd; geçici `postgis/postgis:18-3.6` kabı (boş port);
  - `kentosd db-setup / migrate / dev-seed`, `KENTOS_ENV_FILE` geçici dizinde;
  - üç başsız Chrome: Ayşe'nin iki cihazı ve Mehmet.
- Denetlenenler:
  - bu cihazdaki şablon “Bu cihazda”dır; Paylaş önce eşitlemeyi ister;
  - pencereden “Buluta eşitle”: şablon bulutun kimliğiyle Ayşe'nin listesinde, pencere canlı;
  - Mehmet adıyla bulunur ve paylaşılır; rolü satırında değişir;
  - Mehmet şablonu “Benimle paylaşılanlar”da rolüyle görür;
  - Kullan soruları sorar; pafta cevaplarla yapılır; `@kullanici` onun adıdır;
  - Ayşe'nin revizyon 2'sini Mehmet sormadan duyar; sekmede ve Sayfa'da “Yeni sürüm var”;
  - Ayşe'nin ikinci cihazı kitaplığı indirir;
  - çevrimdışı cihazda değişiklik bekler (“Değişti, eşitlenmedi”); önbellekteki kopya ve sistem
    şablonları kullanılır;
  - öbür cihaz revizyon 3'ü kaydeder; geri gelince ikisi de kalır: bulutunki indirilir, bu
    cihazınki “(bu cihazdaki kopya)” olur ve “Çakışma” rozeti alır; kullanıcıya söylenir;
  - bulutta ikisi de var; öbür cihaz kopyayı duyar;
  - paylaşım kaldırılınca şablon Mehmet'in listesinden çıkar, paftası kalır, ona söylenir;
  - üç sayfada yakalanmamış hata yok.
- Kap her koşunun sonunda durduruldu ve silindi; `docker ps -a` boş döndü. 5432'deki veritabanına ve
  kökteki `.env.local`'e dokunulmadı.

**Görüntüler:** `apps/web/scripts/e2e/out/shots/sheet-cloud/<n>-<sahne>-<light|dark>.png`, 1440×900.

1. `1-bu-cihazda`;
2. `2-once-esitleyin`;
3. `3-kisi-ara`;
4. `4-paylasildi`;
5. `5-benimle-paylasilanlar`;
6. `6-sorular`;
7. `7-paylasilandan-pafta`;
8. `8-yeni-surum-var`;
9. `9-cevrimdisi`;
10. `10-cakisma`.

Ayrıca `out/shots/sheet/sablon-sorulari-*` (iki tema, iki boy).

**Sapmalar:** `tasks-web.md` “Sapmalar” 29–40.

**Yapılmayanlar:**

- **2. aşamanın işleri** (tasarım §12, §13):
  - kurum şablonları (Kurumum “yakında” der);
  - kurum dışına e-postayla davet;
  - sahiplik devri;
  - şablonun yeni sürümünü paftaya uygulamak (“Yeni sürüm var” yalnız bildirir).
- **`pnpm e2e:cloud`** (`cloud.mjs`) koşulmadı. O betik kökteki `.env.local`'in geliştirme
  veritabanını ister; bu makinede yok, istenen de 5432'ye dokunmamaktı. Kalıbı `sheet-cloud.mjs`'te
  kullanıldı.
- **`pnpm inventory`**: W-7; paylaşım ve sorular pencereleri yeni.
- **1100×650'de bulut sahneleri:** yalnız 1440×900'de alındı; sorular penceresi iki boyda da var.
- **Ekran okuyucu** ve **gerçek Chrome penceresinde elle deneme**: B'deki gibi.

#### B bölümü (2–3 Ekim)

**Yapılanlar.** A'nın bütün yer tutucuları motorla değişti; TypeScript'te çekirdek kuralı kalmadı.
Yeni dizinlerde 86 dosya, ~13 600 satır (1 325'i test); `sheet-shots.mjs` 494 satır. En uzun dosya
`TemplateGallery.ts` (400).

- `product/sheet/` (DOM bilmez):
  - `engine.ts`: WASM paketinin tipli sarmalayıcısı. Her `ok:false` cevabı `SheetEngineError`
    (kod, yol, Türkçe ileti) olarak fırlar; arayüz onu “ileti (kod)” diye yazar. Paket ilk pafta
    girişinde ya da galeri ilk açıldığında yüklenir; yükleme düşerse sonraki deneme yeniden yükler.
  - `history.ts`: `SheetHistory`. Her eylem tek `applyOps` adımı; motorun verdiği ters işlemler
    saklanır; adlı girdiler; 200 adım.
  - `adapter.ts`: motorun kitabından arayüzün görünüş modeli (µm → mm, milidereceler → derece).
    Motorun öğe notu (`itemNote`) yalnız öndeki paftaya sorulur; etkin yerleşim düzeninin adı.
  - `ops.ts` (kopyala, grupla, boş ad); `store.ts` sürüm 2 (`assets`, W-9).
  - `profile.ts` ve `templates.ts` motorun `profileFor`, `toolAvailability` ve `rankTemplates`
    cevaplarını gösterir. `preview.ts`, ortak araç listesi, galerinin TS sırası ve
    `tools/sheet/pick.ts` silindi.
- `render/sheet/`:
  - `painter.ts`: çizim planını Canvas2D ile boyar. Yollar ve yaylar; yazılar motorun ölçtüğü
    genişliğe uydurulur; resimler varlık deposundan; kırpma; grup saydamlığı. Haritası olmayan
    çerçevede gri kutu ve “harita / yeri seçilmedi”.
  - `paperPainter.ts`, `snapPainter.ts`: seçim, tutamaçlar, döndürme tutamacı, ızgara, kenar
    boşlukları, kılavuzlar, akıllı kılavuz çizgileri, mesafe rozetleri (kısa aralıkta çizginin
    yanında), eşit aralık işaretleri, yeni öğenin çerçevesi ve boyu, sürüklenen kılavuz.
  - `thumbPainter.ts`: küçük resim aynı boyayıcıyla.
- `tools/sheet/`:
  - `selectTool.ts`: tıklama motorun `hitTest`'iyle (kilitli öğe atlanır). Taşı, boyutlandır
    (Shift oran, Alt merkezden), döndür (Shift 15°), alan seçimi (sağdan sola kesişen). Yapışma
    `SnapSession` ve `snapRotation`'dan. Sürüklerken önizleme kitabın kopyasında; bırakınca tek
    adım.
  - `addTool.ts`: `newItem`. Tıklayınca hazır boy, sürükleyince çerçeve; pafta çerçevesi kenar
    boşluklarını doldurur. Yeni harita görünümün merkeziyle; haritayı okuyan öğeler ilk haritaya
    bağlanır.
  - `guideTool.ts`: cetvelden kılavuz (0,1 mm), sürükleyip taşıma, cetvele bırakınca silme.
  - Klavye: oklar motorun `nudge` adımlarıyla (düz, Shift, Alt).
- `ui/sheet/`:
  - `SheetStage.ts` (369) işaretçiyi `stageInput.ts`'e, boyamayı `stagePaint.ts`'e verir;
    “Seçime yakınlaş” eklendi.
  - Denetçi üç sekme: Öğe, Sayfa, Ön denetim (bulgu sayısıyla).
  - Öğe sekmesi (`inspector/`): her türün bölümü.
    - Harita: standart ya da elle ölçek, merkez ve “Görünümden al”, dönüş, katmanlar, karelaj,
      yazı bandı, genel bakış.
    - Lejant, ölçek çubuğu, kuzey oku, metin, resim, şekil, çizgi, pafta çerçevesi.
    - Tablo (sabit satırlar, katman, süzgeç, yalnız haritadakiler, sütunlar), koordinat listesi,
      antet hücreleri ve imza.
    - Konum ve boyut, Kısıtlar, Görünüş, Veri (ƒ bağları, “Bağ ekle”).
    - Çoklu seçimde ortak değer ya da “—”; başka kipin öğesinde motorun notu.
  - Sayfa sekmesi: kâğıt; yerleşim düzenleri; ana sayfa (seç, Yok, “Seçili öğelerden ana sayfa
    yap”, “Ana sayfadan ayır”); ızgara ve kılavuzlar; dışa aktarma ayarları; değişkenler; şablon.
  - Pencereler: Sayfa ayarları, Değişkenler, İfade (ƒ, canlı `checkExpression`), Şablon olarak
    kaydet, Dışa aktar, `.kpafta` içe aktarma sorusu.
  - Galeri:
    - motorun sırası;
    - kartlarda canlı küçük resim: şablon açık çizimin üstünde motorla kurulur, çizim planından
      boyanır;
    - kâğıt seçimi; “Kullan”dan önce eksik yetenek sorusu.
  - Öğeler listesi: göz, kilit, sürükle-bırak sıra (`setOrder`).
- `app/sheet/`:
  - `service.ts`: `SheetHost`.
  - `projectBooks.ts`: projenin kitabı, değişiklikten 400 ms sonra yazılır.
  - `paint.ts`: çizim planı önbelleği, resimler, lejant simgeleri, yazı tipleri.
  - `mapFrames.ts`: harita çerçeveleri (aşağıda).
  - `inputs.ts`: motorun girdileri: tablo satırları, koordinat listeleri, lejant, CRS bilgisi.
  - `exporting.ts`: SVG, PNG ve `.kpafta`.
  - `templateActions.ts`, `providers.ts`: Sistem motordan, Benim bu cihazdan.
  - `commands.ts` ve `toolCommands.ts`: CAD ve CBS kipinde 81, Hibrit'te 90 `sheet.*` komutu.
    Ekleme komutları kip profilinden gelir.
  - `keys.ts`, `ribbonTab.ts` (gruplar profilden), `install.ts` (W-10'un sinyali).
- `scripts/e2e/sheet-shots.mjs`: 25 sahne × iki tema × iki boy = 100 resim. İçeriğin tamamı motorun.

**Harita çerçeveleri** (inceleme notu 6): `mapFrames.ts` çizimi mevcut çizim hattıyla ekran
dışında çizer:

- araçlar: `buildStyledLayer`, `WebGL2Backend`, `drawLabels`; kâğıdın açık paletiyle;
- haritanın merkez, ölçek ve dönüşüyle.

Önbellek:

- yuva öğe ve ölçek basamağına göredir; en çok 24 resim tutulur;
- anahtar: merkez, ölçek, dönüş, boy, katmanlar, çizimin revizyonu, stiller;
- yakınlaştırırken eski resim yerinde kalır; yenisi 140 ms sonra çizilir.

Galeri ve Paftalar küçük resimleri ile dışa aktarma aynı yolu kullanır. Dışa aktarmada harita
resmi, dpi'ye göre 2048 piksellik karolarla çizilir; SVG'de PNG olarak gömülür.

**Yerleşim düzenleri** (koordinatörün notu): Sayfa sekmesinde gösterilir.

- “Yerleşim düzeni: <ad> (kendiliğinden)”.
- Her düzenin adı, koşulu ve “geçerli” rozeti.
- Satır düğmeleri: yukarı taşı (öncelik), yeniden adlandır, sil.
- “Bu kâğıt için ayrı düzen oluştur”: bu kâğıdın yönü ve boyuyla, en öne.
- Sekmenin kâğıt ipucunda “· düzen: <ad>”.

**İnceleme notları:**

| # | Not | Durum |
|---|---|---|
| 1 | yer tutucular yerine motor | tamam: `hitTest`, `rankTemplates`, `profileFor` / `toolAvailability`, `newItem`, `itemNote` |
| 2 | görüntüler gerçek | tamam: demo çizimde sistem şablonlarından motorla; haritada çizim; elle yazılmış kitap yok |
| 3 | tembel yükleme | tamam: aşağıdaki “Komutlar” |
| 4 | Geri al / Yinele | tamam: W-10; `tuslar` sahnesi denetler |
| 5 | işaretçi işleri `tools/sheet/`'te, 400 satır | tamam |
| 6 | harita çerçeveleri | tamam: yukarıda |
| 7 | WASM sınırında vitest | tamam: `engine.wasm.test.ts`, `fixtures/sheet/v1/`'i Node'da `initSync` ile paketten geçirir |
| 8 | “On denetim” | `main`'in kusuru, paylaşılan CSS: aşağıda; bu dalda değiştirilmedi |
| 9 | motor hataları | tamam: reddedilen işlem ileti alanında “<eylem> uygulanmadı: <ileti> (<kod>)” der (uygulama testi denetler); okunamayan `.kpafta` için ileti alanında dosyanın adı ve nedeni yazar; ƒ penceresi ifadenin hatasını alanın altında söyler |
| 10 | görüntüler iki temada | tamam: önce açık tema gözden geçirildi |

**“On denetim” (not 8) `main`'in kusurudur.**

- Neden paylaşılan `apps/web/src/styles/ribbon.css`'teki kural:
  `.rbtn--large .rbtn__label { -webkit-line-clamp: 2; overflow: hidden; line-height: 1.18; }`.
- Plus Jakarta Sans 12 px'te yazının yüksekliği 15 px (12 + 3), satır ise 14,16 px.
  - Satırın üst kenarı taban çizgisinin 11,58 px üstünde.
  - Ö ve Ü'nün noktaları 13 px'e, İ'ninki 12 px'e çıkar.
  - `overflow: hidden` taşanı keser; piksel oranı 1'de Ö'nün noktaları bütünüyle gider.
- `main`'den değişmemiş CSS ve `main`'in kendi etiketleriyle denendi:
  - Çizim sekmesinde “Ölçülendirme” “Olçülendirme”, Değiştir'de “Ötele” “Otele” görünür;
  - “İçe aktar” ve “İçine tıklayarak alan”da İ'nin noktası kısmen kesilir;
  - Ç ve Ş'nin çengeli hesaba göre son satırın altından 0,4 px taşar (düzeltme üst kenar içindir).
- En küçük düzeltme: yazının yeri değişmez, kutu üstteki 4 px'lik boşluğa 1,5 px uzar.

  ```css
  .rbtn--large .rbtn__label {
    padding-top: 0.125em;
    margin-top: -0.125em;
  }
  ```

  Sayfaya eklenerek 1× ve 2×'te denendi: noktalar geri gelir; düğmenin boyu ve yazının yeri
  aynı kalır. Karşılaştırma resmi: `out/shots/sheet/zz-serit-o-kirpmasi-main-ve-duzeltme.png`.
  Solda `main`, sağda düzeltme; üst dört satır 1×, alttakiler 2×.

**Bağlantı noktaları:** §3'te W-1, W-6, W-7 ve W-9 güncellendi; W-10 ve W-11 yeni. Paylaşılan
dosyalarda `main`'e göre değişen satırlar:

| Dosya | Eklenen / silinen satır (A + B) |
|---|---|
| `app/commands.ts` | +23 / −5 (W-10, yalnız B) |
| `app/createApp.ts` | +8 / −1 |
| `viewport/ViewportController.ts` | +8 (W-11, yalnız B) |
| `ui/shell/AppShell.ts`, `ui/ribbon/Ribbon.ts`, `app/ribbon.ts` | A'daki gibi (+14 / −4, +70 / −10, +2 / −1) |

**Komutlar** (hepsi `flock .run/heavy.lock` ile; son hâlde):

| Komut | Sonuç |
|---|---|
| `pnpm typecheck` | temiz |
| `pnpm test` | 243 dosya geçti, 21 atlandı; 3 243 test geçti, 21 atlandı |
| `pnpm -C apps/web exec vitest run src/{product,render,tools,ui,app}/sheet` | 10 dosya, 87 test geçti |
| `pnpm build` | temiz. Pafta paketi ayrı parça: `kentos_sheet_wasm` 10,3 kB ve `.wasm` 3,55 MB (gzip 1,0 MB); `index.html` onları ön yüklemez. Tembel parçalar: `SheetWorkspace` 89 kB, `TemplateGallery` 18,2 kB, `windows` 18,8 kB, `statusCells` 2,6 kB. Giriş parçasında pafta kodu ~122 kB, küçültmeden önce (parçanın %8,6'sı): kurulum, komutlar, servis, depo, boyama. 500 kB üstü ana parça uyarısı işten öncedir |
| tembel yükleme: `dist/` vite preview ile, başsız Chrome'da `performance` kayıtları | açılışta 47 kaynak; tek WASM `kentos_geometry_wasm`; pafta parçası yok. İlk “Şablondan pafta…” açılınca `TemplateGallery`, `kentos_sheet_wasm.js` ve `.wasm` gelir; galeride 10 kart. Kayıtlı paftası olan bir proje yeniden açılınca sekmeler adları motorsuz gösterir, motor sekmeye girilince gelir: bunu uygulama testi denetler; tarayıcıda denenmedi (demo çizim oturum anahtarıyla açılır, yeniden yüklenince kitabı yeni oturumda aranır) |
| `pnpm e2e` | tüm kontroller geçti; konsolda hata yok |
| `node apps/web/scripts/e2e/sheet-shots.mjs` | 100 resim, hata yok; `tuslar`ın denetimleri geçti |

Tarayıcıda ayrıca denenenler (geçici yoklama betiğiyle; betik silindi):

- Öğeler'de sürükle-bırak: sıra değişir, Geri al eski sıraya döner.
- “Seçili öğelerden ana sayfa yap”: üç öğe ana sayfaya taşınır; pafta onu kullanır; Öğeler'de
  kilitli “Ana sayfa” satırı.
- Ekle aracıyla sürükleme: yeni çerçeve ve boyu gösterilir.

Çalıştırılmayanlar:

- `pnpm inventory`: paylaşılan, üretilmiş dosya (W-7).
- `e2e:layout`, `e2e:interaction`: yeni pencereler taşma denetçisinden geçmedi; 1100×650
  resimlere gözle bakıldı.
- Rust komutları: Rust ajanınındır.

**Görüntüler:** `apps/web/scripts/e2e/out/shots/sheet/<sahne>-<light|dark>-<1440|1100>.png`.

- B sahneleri:
  - `b1-ifraz-sistem-sablonu`: demo çizimde sistem şablonundan ifraz paftası;
  - `b2-surukle-kilavuzlar`: kuzey oku sürüklenirken akıllı kılavuzlar ve rozetler;
  - `b3-coklu-secim-kisit`: üç öğe seçiliyken denetçi ve kısıt düzenleyicisi;
  - `b4-galeri-sistem`, `b4-galeri-benim`;
  - `b5-on-denetim`, `b6-disa-aktar`.
- A sahneleri motorla yeniden: `model-sekmeler`, `model-arti-menu`, `harita-secili`,
  `kisit-ipucu`, `sayfa`, `baska-kip`, `sekme-menusu`, `serit-ipucu`, `gercek-boyut`, `hidpi`,
  `tuslar`, `galeri-butun-kipler`, `galeri-kurumum`, `sablon-paylas`.
- Pencereler: `degiskenler`, `sayfa-ayarlari`, `fx-ifade`, `arac-ekle`.
- A'nın önizleme verisiyle alınmış resimleri (`onizleme-*`, `galeri-motorsuz-*`)
  `out/shots/sheet/bolum-a/`'ya taşındı.

**Sapmalar:** `tasks-web.md` “Sapmalar” 13–28.

**Yapılmayanlar:**

- **C bölümü** (bulut).
- **Atlas düzenleyicisi ve önizlemesi.** `sheet.atlas` devre dışı ve nedenini söyler; motorun
  `atlasPlan`'ı hazır.
- **Ana sayfanın öğelerini yerinde düzenleme.** Ayırıp düzenleyip yeniden ana sayfa yapılır.
- **Boyama kodunun tembel yüklenmesi.** `paint.ts`, `mapFrames.ts`, `inputs.ts`,
  `templateActions.ts` ve `render/sheet/painter.ts` (~31 kB, küçültmeden önce) giriş
  parçasındadır; motorun `import()`'una katılabilirler.
- **Ekran okuyucu denemesi.**
- **Gerçek Chrome penceresinde elle deneme.** Bütün etkileşimler CDP olaylarıyla denendi. CDP
  tarayıcının kendi kısayollarını (Ctrl+1, Ctrl+0) atlar.

#### A bölümü (2 Ekim)

**Yapılanlar.** Yeni dizinlerde 48 dosya, ~7 400 satır (testler dahil).

- `product/sheet/` (DOM bilmez):
  - `view.ts`: arayüzün görünüş modelleri (kâğıt mm, Sol/Üst/Genişlik/Yükseklik).
  - `state.ts`: `SheetState`.
    - Kitap, öndeki sekme (null = Model), seçim, kâğıt aracı, Hizala'nın hedefi, motorun durumu.
    - Silinen paftada soldaki sekme öne gelir.
    - Neden yapılamıyor metinleri.
  - `store.ts`: IndexedDB `kentos.sheets.v1`.
    - `books`: proje anahtarıyla kitap; `templates`: bu cihazın şablonları.
    - Zarfı denetlenir; okunamayan kayıt `…#okunamadi-<zaman>` altında saklanır.
    - Kitap taşıma ve kopyalama; bellekte yedek depo.
  - `templates.ts`: §11a'nın galeri kuralı.
    - Kart modeli, sağlayıcı arayüzü, uygunluk sırası (tür → kip → ortak → başka kip).
    - Eksik yetenekler, arama (Türkçe katlama), kâğıt ve tür süzgeçleri.
  - `profile.ts`: kip profilinin arayüz modeli ve motor gelene dek kullanılan ortak liste.
  - `preview.ts`: görüntüler ve testler için elle yazılmış önizleme kitabı ve şablon kartları.
    Uygulama bu dosyayı içe aktarmaz.
- `render/sheet/` (Canvas2D):
  - `paperView.ts`: sığdır, gerçek boyut (96 dpi), imleç çevresinde yakınlaştırma, kaydırma.
  - `rulerTicks.ts`: 1-2-5 cetvel işaretleri.
  - `paperPainter.ts`: masa, gölgeli beyaz kâğıt, kenar boşluğu, öğe çerçeveleri, seçim,
    tutamaçlar, döndürme tutamacı, pencere/kesişim kutusu.
  - `rulerPainter.ts`: kâğıdın uzunluğu, seçim bandı, imleç çizgisi.
  - `thumbPainter.ts`: küçük resim.
  - Hepsi aygıt pikseliyle çizer; piksel oranı değişince yeniden çizer.
- `tools/sheet/pick.ts`: tıklama ve kutuyla seçme. Döndürülmüş çerçeve dahil; grup bütün seçilir;
  ana sayfa ve gizli öğe seçilmez.
- `ui/sheet/`:
  - `SheetTabs.ts`: “Model \| … \| +”. Sağ tık, menü tuşu ve Shift+F10 menü açar; ←/→, Home ve
    End sekmeler arasında gezer; şablonu yeni sürümlü sekmede nokta vardır.
  - `SheetWorkspace.ts`: sol sütun Paftalar ve Öğeler, ortada masa, sağda Denetçi. Sütunlar
    sürüklenir.
  - `SheetStage.ts`: cetveller ve yakınlaştırma kutusu.
    - Ctrl+tekerlek yakınlaştırır; tekerlek kaydırır, Shift ile yatay.
    - Orta tuş, Boşluk ya da El sürükleyerek kaydırır.
    - Seçim, kutu, üzerine gelme, sağ tık menüleri.
  - `SheetsPanel.ts`: küçük resimli pafta listesi.
  - `ItemTree.ts`: TreeView ile; göz, kilit, gruplar ve ana sayfa satırı. Sürükle-bırak sıra ve
    gruplama motor gelince açılır.
  - `SheetInspector.ts`: Öğe ve Sayfa sekmeleri.
    - Bölümler: Konum ve boyut, Kısıtlar, türün kendisi, Görünüş, Veri.
    - Çoklu seçimde ortak değer, farklıysa “—”; bağlanabilen her alanda ƒ.
    - Salt okunurluk notu; başka kipin öğesinde §11a notu.
  - `widgets/ConstraintEditor.ts`: kare çizim.
    - Dört kenar ve iki orta çizgi; Shift ile iki kenar.
    - Yatay, Düşey ve Göre açılır listeleri, karışık değer kesikli.
  - `widgets/fields.ts`: sayı alanı ve ƒ düğmesi.
  - `TemplateGallery.ts`: Stil yöneticisinin ailesinde.
    - Bölümler: Sistem, Benim, Kurumum, Benimle paylaşılanlar.
    - Arama, kâğıt ve tür süzgeçleri, “Bütün kiplerin şablonları”.
    - Kartlar ve rozetler, ayrıntı ve eylemler; Kullan'dan önce eksik yetenek sorusu.
  - `ShareTemplateDialog.ts`: ShareDialog'un kalıbında iskelet. Eşitlenmemiş şablonda “Paylaşmak
    için önce buluta eşitleyin.” der.
  - `statusCells.ts`: Sol/Üst mm, seçimin boyu, %zum, Sayfa n/m, ön denetim.
  - Diğerleri: `icons.ts` (36 simge), `menus.ts`, `sheet.css`.
  - Kararlar DOM'suz modüllerde: `*Plan.ts`, `widgets/anchors.ts`.
- `app/sheet/`:
  - `install.ts`: `installSheets(ctx, shell)`, `sheetsOf`.
  - `service.ts`: `SheetService`, arayüzün ev sahibi.
    - Proje anahtarını izler: bulut, proje kimliği, dosya ya da oturum.
    - Taşı, kopyala ya da yükle.
  - `providers.ts`: şablon sağlayıcıları.
  - `commands.ts`: 67 `sheet.*` komutu. Motor gerekenler devre dışıdır; nedeni ipucunda ve
    menüde yazar.
  - `keys.ts`: pafta kipinin tuşları.
    - Çizim alanının kısayolları paftada tutulur ve nedeni söylenir.
    - Şeritten ya da komut satırından çalışan bir çizim komutu önce Model'i öne getirir.
  - `ribbonTab.ts`: Pafta sekmesi. Panelleri Pafta, Araç, Ekle, Düzen, Görünüm, Çıktı; Ekle
    profilden gelir.
  - `projectKey.ts`: proje anahtarı.
- `scripts/e2e/sheet-shots.mjs`:
  - `cdp.mjs` ile çalışır, `shots.mjs`'e dokunmaz.
  - 19 sahne, iki tema, 1440×900 ve 1100×650.
  - Tuşlar sahnesi H, V, Ctrl+1, Ctrl+0, C ve M'yi denetler; yanlışsa koşu düşer.

**Bağlantı noktaları:** §3'te W-1 … W-9. Paylaşılan dosyalarda yalnız şu değişiklikler var:

| Dosya | Eklenen / silinen satır |
|---|---|
| `app/createApp.ts` | +3 |
| `ui/shell/AppShell.ts` | +14 / −4 |
| `ui/ribbon/Ribbon.ts` | +70 / −10 (uzantı noktası) |
| `app/ribbon.ts` | +2 / −1 (tip) |

**Komutlar** (hepsi `flock .run/heavy.lock` ile):

| Komut | Sonuç |
|---|---|
| `pnpm typecheck` (ilk) | **düştü**: eski WASM paketleri (`src/wasm/core.ts`, `src/io/formatsWorker.ts`), işten önceki durum |
| `pnpm wasm` | 5 paket yeniden derlendi (54 s) |
| `pnpm typecheck` | temiz; son hâlde de temiz |
| `pnpm -C apps/web exec vitest run src/{product,render,tools,ui,app}/sheet` | 7 dosya, 57 test geçti |
| `pnpm -C apps/web exec vitest run src/app/ribbon.test.ts src/ui/ribbon src/app/menus.test.ts src/app/workspaces.test.ts` | 31 test geçti |
| `pnpm test` | 240 dosya geçti, 21 atlandı; 3213 test geçti, 21 atlandı |
| `pnpm build` | temiz. Tembel parçalar: `SheetWorkspace` 40 kB, `TemplateGallery` 19.7 kB, `statusCells` 2.3 kB. 500 kB üstü ana parça uyarısı işten öncedir |
| `node apps/web/scripts/e2e/sheet-shots.mjs` | 76 resim, hata yok (2 dk 47 s) |

Çalıştırılmayanlar:

- `pnpm inventory`: paylaşılan, üretilmiş dosya (W-7).
- `pnpm e2e`, `e2e:layout`: yeni pencereler taşma denetçisinden geçmedi; 1100×650 resimlere gözle
  bakıldı.
- Rust komutları: Rust ajanınındır.

**Görüntüler:** `apps/web/scripts/e2e/out/shots/sheet/<sahne>-<dark|light>-<1440|1100>.png`.

- Motorsuz gerçek hâl: `model-sekmeler`, `model-arti-menu`, `galeri-motorsuz`.
- Önizleme verisiyle (B'de `onizleme-*` ve `galeri-motorsuz-*` `bolum-a/` altına taşındı):
  - çalışma alanı: `onizleme-calisma-alani`, `-harita-secili`, `-coklu-secim`, `-kisit-ipucu`,
    `-sayfa`, `-baska-kip`, `-sekme-menusu`, `-serit-ipucu`, `-gercek-boyut`;
  - `-hidpi`: piksel oranı 2, masanın köşesi;
  - `-tuslar`;
  - galeri: `-galeri-sistem`, `-galeri-benim`, `-galeri-butun-kipler`, `-galeri-kurumum`;
  - `-sablon-paylas`.

**Sapmalar:** `tasks-web.md` “Sapmalar” 1–12.

**Yapılmayanlar:**

- **B bölümünün tamamı.** Motorla gelecek olanlar:
  - motor sarmalayıcısı; işlemler ve geri alma;
  - çizim planından boyama, harita çerçeveleri dahil;
  - taşı, boyutlandır, döndür, akıllı kılavuz, cetvelden kılavuz, okla kaydırma;
  - türlere göre denetçi, ƒ ile ifade oluşturucu;
  - şablon kullanma ve kaydetme;
  - ön denetim, dışa aktarma;
  - fixture testleri ve altı B sahnesi.
- **C bölümü** (bulut).
- **Paftanın ve öğenin yerinde adlandırılması.** Komutları var, devre dışı.
- **Öğeler'de sürükle-bırak.** Yazıldı ama tarayıcıda denenmedi; motor yokken satırlar sürüklenmez.
- **Hızlı erişimin Geri al ve Yinele'si** paftada hâlâ modeli geri alır, önce Model'e geçer. B'de
  bir bağlantı noktası gerekir.
- **Proje türü yok:** galeri yalnız kipe göre sıralar.
- **Ctrl+1 ve Ctrl+0** CDP olaylarıyla denendi. CDP tarayıcının kendi kısayollarını atlar; gerçek
  Chrome penceresinde elle bakılmadı.
- **Ekran okuyucu denemesi** yapılmadı.
- **400 satırı aşan iki dosya:**
  - `SheetStage.ts` (421): işaretçi işleri B'de `tools/sheet/`'e taşınacak.
  - `TemplateGallery.ts` (441): ayrıntı paneli `managerDetails.ts` gibi ayrılabilir.

## 8. Bağlantıyı geri almak

Bağlama commit'i (6) geri alınırsa (`git revert`) dalın bütün kodu yerinde durur ama derlemeye,
uygulamaya ve web'e bağlı değildir. Özellik `main`'de görünmez, hiçbir şey kırılmaz.
