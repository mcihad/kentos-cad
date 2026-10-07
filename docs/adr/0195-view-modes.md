# ADR 0195: Görünüm kipleri

- **Durum:** kabul edildi (2026-10-07). Sıra sahibin kararıdır: TODOS.md §16.1, `CAD-32`'nin ardından `CAD-33`; madde tek parçada biter ve
  sahibin 6 Ekim gecesi sözüyle CAD-36'ya dek ara verilmez. Ayrıntılar bu ADR'nin varsayılanlarıdır. Örnekler Netcad'in Görünüm › Çizim
  Yöntemi (Renk Modu), Görünüm Ayarları (Alan Taramaları, Alan Sınırları) ve Belirginleştirme Tanımları'dır.
- **Bağlam belgesi:** TODOS.md `CAD-33`; ADR 0023 (tipli ayarlar), ADR 0090 (stilli çizim), ADR 0139 (çizgi kalınlığı), ADR 0051
  (çizim zemini), ADR 0029 ve 0120 (seçim ve üzerine gelme vurgusu).

## Bağlam

Çizimi kâğıda basmadan önce tek renk ya da gri görmek, kalabalık bir paftada dolguları ya da alan sınırlarını bir an kapatmak, yarı
saydam alanları tam görmek ve seçim vurgusunu ekrana ya da göze göre ayarlamak gerekir. Bugün yalnız Çizgi kalınlığı (`graphics.lineWeights`)
ve çizim zemini (`appearance.drawingBackground`) var.

## Karar

### 1. Ayarlar

Kullanıcının tercihleridir (proje verisi değil; ADR 0023 §4.4), iki platformda, kaydedilmez ama hatırlanır:

- `graphics.colorMode`: **Renkli** (`color`, varsayılan), **Tek renk** (`mono`), **Gri** (`gray`). Tek renkte her çizgi, dolgu, işaret
  ve yazı çizim zemininin karşıtı olan mürekkep rengindedir (temanın `ink`'i); grideki her renk kendi parlaklığıdır: `y = 0,2126 r +
  0,7152 g + 0,0722 b` (sRGB değerleri, Rec. 709 katsayıları), `(y, y, y)`. Alfa korunur. Yazıların halesi ve ölçü değerinin zemini zemin
  renginde kalır (yazı okunur kalsın); resim nesneleri ve raster görüntü dolguları kendi renkleriyle çizilir.
- `graphics.fills` (**Dolgular ve taramalar**, açık): kapalıyken alan dolguları, taramalar, desen, karo ve degrade dolgular çizilmez; resim
  nesneleri çizilir.
- `graphics.areaEdges` (**Alan sınırları**, açık): kapalıyken kapalı alanların (parsel, bina; çok parçalı ve delikli olanlar dahil)
  çizgileri çizilmez, dolguları çizilir. Daire, elips, çoklu çizgi ve tarama nesnesi etkilenmez.
- `graphics.transparency` (**Saydamlık**, açık): kapalıyken dolguların (yarı saydam renk ya da saydamlık değeri) alfası 1 olur; alfası 0
  olan (görünmez) dolgu görünmez kalır. Resmin saydamlığı da kalkar.
- `graphics.highlightColor` (**Vurgu rengi**): **Tema vurgusu** (`accent`, varsayılan), **Turuncu** `#F5A623`, **Kırmızı** `#E5484D`,
  **Yeşil** `#30A46C`, **Camgöbeği** `#05A2C2`, **Eflatun** `#D6409F`: seçimin ve üzerine gelmenin rengi.
- `graphics.highlightWidth` (**Vurgu kalınlığı**): 1–5 piksel, varsayılan 1 (bugünkü): seçimin ve üzerine gelmenin çizgi kalınlığı.

### 2. Nerede uygulanır

- **Dolgular ve alan sınırları** stil çekirdeğinin katman kuruluşundadır (`build_layer`'ın görünüşü): kapalıyken resim dışındaki dolgular
  ve kapalı alanların çizgileri partilere hiç girmez. İki platform aynı çekirdekle kurar.
- **Renk kipi ve saydamlık** sayfanın renk çözümündedir (partilerin GPU'ya giden renkleri; masaüstünde `batches::decode`, web'de
  `styledBatches`): çizgi, dolgu, işaret, yazı işareti, SVG sembollerin parametre renkleri ve desen karoları. Kural ikisinde aynıdır, ortak
  durumlar `fixtures/style/v1/batches.json`'un görünüş durumlarıdır.
- **Çizimin yazıları** (yazı nesnesi, çok satırlı yazı, ölçü değeri, kılavuzun notu, tablo hücreleri, etiketler) ev sahibinin üst
  katmanında çizilir: aynı renk kuralı orada da uygulanır.
- **Vurgu**: seçimin ve üzerine gelmenin rengi ve çizgi kalınlığı; seçimin dolgusu vurgu renginin aynı saydamlığıyla (0,13) kalır.
- Değişince çizim yeniden kurulur (Çizgi kalınlığı gibi); belge değişmez, geri alma adımı yoktur.

### 3. Arayüz

- Görünüm sekmesinde **Görünüm kipleri** paneli (iki proje türünde): **Renk kipi ▾** (Renkli, Tek renk, Gri; her biri kendi ikonuyla,
  seçili olan işaretli), **Dolgular**, **Alan sınırları**, **Saydamlık** (açık/kapalı düğmeler). Komutlar `view.colorMode.color`,
  `view.colorMode.mono`, `view.colorMode.gray`, `view.fills`, `view.areaEdges`, `view.transparency`.
- Uygulama ayarları › Çizim'de “Görünüm kipleri” grubu: altı ayarın hepsi (vurgu rengi ve kalınlığı yalnız burada).

## Kapsam dışı

Katman ya da nesne başına görünüm kipi (stilin işi), baskıda renk kipi (pafta çıktısının işi, §16.4), resimlerin griye çevrilmesi.

## Uygulama

- **Ayarlar** `crates/shared/contracts/src/settings/schema.rs`'te (`graphics.colorMode`, `graphics.fills`, `graphics.areaEdges`,
  `graphics.transparency`, `graphics.highlightColor`, `graphics.highlightWidth`); şema `settingsSchema.json`'a üretildi. Web'de
  tercihler `app/state.ts`'te, masaüstünde `ayarlar.json`'da; ikisinde de Uygulama ayarları › Çizim'in “Görünüm kipleri” grubu
  (web `ui/settings/engineSection.ts`, masaüstü `settings_sections.rs`).
- **Kuruluş:** stil çekirdeğinde `build::View { fills, area_edges }` ve `build_layer_with`; nesne başına `BatchSink::hide` (dolgular
  yalnız kapalı çizgilerde, resim dolgusu hariç; çizgiler yalnız `Shape::Polygon`'da). Web WASM'ın `buildStyled`'ına iki bayrakla
  (`crates/wasm/geometry-wasm/src/store.rs`), masaüstü `BuildOptions::view` ile verir.
- **Renk:** masaüstünde `kentos_native_style::color`'ın `ColorMode`, `ViewColors`, `view_rgba`, `view_hex`; `batches::decode`'un
  `DecodeOptions::view`'u çizgi, dolgu, işaret, sembol parametreleri ve desen karolarına uygular, yazı halesi kendi renginde kalır.
  Web'de `render/color.ts`'in aynı adlı işlevleri ve `styledBatches.ts`. Çizimin yazıları masaüstünde `labels.rs`'in `Colors::mode`'u,
  web'de `modedPalette` ile.
- **Vurgu:** masaüstünde `viewport::highlight_color` ve wgpu çizim hattının ikinci çerçeve bağlaması (`kentos.cad2d.highlight`:
  vurgu kalınlığıyla aynı çerçeve, stilli katmanlardan ve kaplamalardan sonra çizilen seçim ve üzerine gelme parçalarına); web'de
  `highlightHex` ve `sceneBuilder.ts`'in `widenLines`'ı (WebGL2 ve WebGPU'nun düz çizgileri bir piksel olduğundan kalın vurgu stilli
  piksel çizgisine çevrilir).
- **Komutlar ve şerit:** `view.colorMode.color`, `.mono`, `.gray`, `view.fills`, `view.areaEdges`, `view.transparency` (web
  `app/commands.ts`, `app/menus.ts`'in Görünüm kipleri bölümü; masaüstü `app.rs`, `style/mod.rs`'in `choose_color_mode`'u). İkonlar
  (`ui/icons.ts`): `colorModeColor`, `colorModeMono`, `colorModeGray`, `viewFills`, `viewAreaEdges`, `viewTransparency`; sahip uyurken
  önerilen seçenekler alındı (Tek renk için ikinci seçenek: birincinin yarım dairesi Tema'nın ikonudur).

## Doğrulama

- Renk kuralı ve kuruluş: `fixtures/style/v1/batches.json`'un beş yeni görünüş durumu (`view-mono`, `view-gray`, `view-no-fills`,
  `view-no-edges`, `view-opaque`), web kaydeder (`scripts/fixtures/record-batches.test.ts`), masaüstü yerli stil testi geçer
  (`crates/native/style/tests/batches.rs`); `render/viewColors.test.ts` ve `color.rs`'in testi gri dönüşümünü ve alfayı denetler
  (`#3E63DD` → `#646464`).
- Ortak iz `fixtures/interaction/v1/view-modes.json` (`view-modes.kcad`): turuncu ve üç piksel vurgulu seçim, üç renk kipi ve üç
  anahtar, belge değişmez; iki platformda geçer ve her kip resimlenir (`kentos-cad kullan view-modes`, `pnpm -C apps/web e2e:use
  view-modes`). Oynatıcılar görünüm tercihlerini her izin başında varsayılana döndürür.
- Ayarların şeması ve göçü: `kentos-contracts settings`, web `app/settings/store.test.ts`; masaüstünün ayar penceresi testleri
  (`settings_view`).
- İkon turu: `KENTOS_SHOTS_ONLY=renk-kipi cargo test -p kentos-desktop icon_tour -- --ignored --nocapture` (geniş pencerede, panel
  dar pencerede ikonlara iner).
