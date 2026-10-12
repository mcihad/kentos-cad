# ADR 0226: Sembol katmanı ekleri: geometri üretici, çizim efektleri, maskeler ve karışım kipleri

- **Durum:** kabul edildi (2026-10-12). Kapsamı ben belirledim (sahibin sözü: “Benim seçmeme gerek yok sen sıradan devam et”). İki
  platform (masaüstü ve web) ve bulut; ikonlar sorulmadan seçilir; ilke “Performance First”. Madde tek parçada biter.
- **Bağlam belgesi:** TODOS.md `GIS-30`; docs/STYLE.md §3 (sembol katmanları), §6 (çizim hattı; bilinen sınırlar: “bulanık gölge ve
  parıltı gibi çizim efektleri yok”), §7 (Sembol tasarımcısı, Katman stili); ADR 0090 (masaüstünün stilli çizimi), ADR 0094 (Sembol
  tasarımcısı), ADR 0120 (tutulan resim), ADR 0157 (karolar), ADR 0195 (görünüm kipleri), ADR 0212 (etiket motoru: yerleşen etiketler),
  ADR 0213 (ek işleyiciler), ADR 0225 (fotoğraf noktalarının yönü: kamanın tipik kullanımı).
- **Araştırma:** QGIS'in Geometry generator sembol katmanı (ifadenin döndürdüğü geometri, alt sembolü; `buffer`, `centroid`,
  `wedge_buffer`, `translate`, `rotate`, `bounds`, `convex_hull`, `extend`, `offset_curve`), Draw effects (Drop shadow: uzaklık, açı
  135°, bulanıklık, renk, saydamlık; Inner shadow; Outer glow ve Inner glow: yayılma, bulanıklık; Blur; Colorise; Transform; efektlerin
  sırası), Masks (etiketin ve maske sembol katmanının şekli, maskelenen katmanlar), Layer rendering (Opacity, Blending mode); ArcGIS
  Pro'nun Geometric effects'i (Buffer, Move, Rotate, Scale, Offset, Extension, Enclosing polygon, Circular sector, Regular polygon …;
  sıralı zincir, parametreleri veriyle tanımlı), Masking ve Feature Outline Masks, katmanın Blend mode ve Transparency'si; W3C
  Compositing and Blending Level 1 (ayrılabilir karışım kiplerinin `B(cb, cs)` tanımları, kaynak-üstü birleştirme).

## Bağlam

Sembol katmanları bugün nesnenin kendi geometrisini çizer: işaret noktanın, alanın iç noktasının ya da çizginin üstünde; çizgi yolun ya da
kenarın üstünde; dolgu alanın içinde. Saha haritalarında sık istenen çizimler bunun dışındadır: fotoğraf noktasının bakış yönünü gösteren
daire dilimi, direğin çevresinde sabit yarıçaplı tampon, parselin sınırlayıcı kutusu, bir yolun uçlarına konan işaretler. Kartografik
çizimde yapıların altına gölge, su yüzeyine iç parıltı, yazıların çevresinde alttaki çizgilerin boşaltılması (maske), gölgeli kabartmanın
renklerin üstüne çarpılarak bindirilmesi (karışım kipi) ve bir katmanın bütünüyle yarı saydam gösterilmesi yaygındır. KentOS'un tek “efekti”
basit çizginin `blur` ve `shift`'iyle yapılan kaba çizgi gölgesidir; alanlarda, işaretlerde ve katman düzeyinde hiçbiri yoktur. Eş yükselti
eğrisinin yazısının altındaki “maske” çizim alanının rengiyle boyanmış bir kutudur (ADR 0212), gerçek boşaltma değildir.

## Karar

### 1. Geometri üretici

Yeni sembol katmanı türü `geometryGenerator` (arayüzde **Geometri üretici**; alan, çizgi ve işaret sembollerinde). Nesnenin geometrisi
**sıralı adımlardan** (`steps`) geçer, sonuç katmanın kendi sembolüyle (`symbol`) çizilir. Adımların sayıları veriyle tanımlıdır
(`DataDefined<number>`: sabit ya da nesne başına ifade, `{ expr, fallback }`); uzunluklar katmanın biriminde (kâğıt mm, harita m, ekran
px; px kâğıt mm'si gibi çizim ölçeğinde metreye çevrilir, uzunluğu değiştiren öbür değerler gibi).

| Adım (`kind`) | Arayüzdeki adı | Girdi → çıktı | Değerler |
|---|---|---|---|
| `centroid` | Ağırlık merkezi | her biri → nokta | — |
| `pointOnSurface` | İç nokta | her biri → nokta | — |
| `vertices` | Köşeler | çizgi, alan → noktalar | `which`: `all`, `inner`, `first`, `last`, `ends` |
| `boundary` | Sınır | alan → çizgiler | — |
| `offset` | Paralel | çizgi, alan → çizgiler | `distance` (artı solda, alanda içeri) |
| `extend` | Uzat | çizgi → çizgi | `start`, `end` |
| `buffer` | Tampon | her biri → alan | `distance` (alanda eksi içe), `side`: `both`, `left`, `right` (çizgide tek yan; düz yanı çizginin kendisi) |
| `envelope` | Sınırlayıcı kutu | her biri → alan | — |
| `hull` | Dışbükey örtü | her biri → alan | — |
| `circle` | Daire | her biri → alan | `radius` |
| `wedge` | Daire dilimi | her biri → alan | `direction`, `width` (derece), `radius`, `inner` |
| `polygon` | Düzgün çokgen | her biri → alan | `sides`, `radius`, `rotation` |
| `translate` | Öteleme | her biri → aynı | `dx`, `dy` |
| `rotate` | Döndür | her biri → aynı | `angle` |
| `scale` | Ölçekle | her biri → aynı | `x`, `y` |

Tanımlar (bağımsız başvuru bunları yazar):

- **Merkezler.** Ağırlık merkezi: alanda halkaların (delikler eksi) alan ağırlıklı merkezi, çok parçalıda parçaların birlikte; çizgide
  kenarların uzunluk ağırlıklı orta noktası; noktada kendisi; noktalarda ortalamaları. İç nokta: alanda etiketin konduğu iç nokta
  (`place::interior_point`), çizgide yol boyu ortası (`line_middle`), noktada kendisi. `circle`, `wedge`, `polygon`, `rotate` ve `scale`
  girdinin ağırlık merkezinde kurulur ya da döner; noktada noktanın kendisinde (her nokta ayrı). Düzgün çokgenin ilk köşesi `rotation`
  0'da kuzeydedir.
- **Köşeler.** `all` bütün köşeler (kapalı yolda son köşe ilkinin aynıysa bir kez); `inner` uçlar dışındakiler (kapalı yolda hepsi);
  `first`, `last`, `ends` açık yolun ilk, son ya da iki ucu (kapalı yolda ilk köşe).
- **Açılar.** `direction` derece, kuzeyden saat yönünde (Türk ölçmeciliğinin semti gibi derece; fotoğrafın `Yön`'ü doğrudan verilir);
  dilim `direction − width/2` ile `direction + width/2` arasıdır, `width` 0–360 (360: halka ya da daire). `rotation` ve `angle` derece,
  doğudan saat yönünün tersine (işaretin dönüşü gibi).
- **Yaylar.** Daire, dilim, düzgün çokgen dışındaki yaylı sonuçlar (tampon, dilim, daire) yaya oturan kirişlerle çizilir: yarıçapı `r`
  (metre) olan yay, `n = clamp(ceil(θ / (2·acos(1 − τ / r))), 1, 720·θ/2π)` parçaya bölünür; `τ = 0,05 mm` kâğıtta (`0,05 · ölçek / 1000`
  metre), `θ` yayın açısı; `r ≤ τ` ise bir kiriş; daire en az 8 kiriş. Tampon `ops::geoprocess::buffer`'dır (yaylar kesin), yayları bu
  kurala göre kirişlenir.
- **Paralel ve Uzat.** Paralel `geom::offset::offset_path`'tir (basit çizginin kaydırmasıyla aynı kural: köşeler uzatılarak birleşir,
  birleşme uzaklığın 4 katını aşınca kesilir). Uzat açık yolu ilk kenarının
  doğrultusunda başından, son kenarının doğrultusunda sonundan uzatır; kapalı yol değişmez; eksi değer kısaltmaz (sıfır sayılır).
- **Ölçekle.** `x` ve `y` çarpanları; biri boşsa öbürü; ikisi boşsa 1. Ölçekle ve Döndür kapalı halkaların yönünü korur (aynalayan
  eksi çarpan halkaları ters çevirir, sonuç yeniden yönlendirilir: dış halka saat yönünün tersine).
- **Sınıf uyumu.** Adımın girdisine uymayan sınıfta (noktaya `boundary`, `offset`, `extend`, `vertices`) sonuç boştur, çizilmez. Son
  sınıf katmanın sembolünün sınıfına uymazsa sembolün bugünkü uyum kuralı geçerlidir (alan sonucuna çizgi sembolü kenarlarını çizer,
  çizgi sonucuna işaret sembolü ortasına konur …). Boş, sonsuz ya da sayı olmayan değer adımı boş sonuç yapar; `radius ≤ 0`, `sides < 3`
  ya da `width ≤ 0` de.
- **Düzeyler.** Üreticinin katmanları kendi alt düzeylerini alır: `düzey + j/256` (çizgi boyunca işaretin katmanları gibi).
- **İfadeler.** Değerin ifadesi nesnenin özniteliğini adıyla okur: `[Yön]` ya da `Yön`; tırnaklı `'Yön'` ve `"Yön"` metindir (ifade dilinin
  kuralı). Değeri olmayan ya da sayı olmayan nesne `fallback`'i alır.
- Tasarımcı üreticinin sembolüne kendi işaret sembolü olan katmanları (çizgi boyunca işaret, desen, iç noktada işaret) ve iç içe üreticiyi
  koymaz (derinlik bir); derleyici dosyadan gelen her şeyi çizer.

### 2. Çizim efektleri

Sembolün üst düzey katmanları `effects` taşır: en çok beşi, türü birer kez, her biri `enabled` ile kapatılabilir. Geometri üreticinin ve
işaret sembollerinin içindeki katmanların efektleri yok sayılır: üreticinin kendi efektleri ürettiği çizimin bütününe uygulanır (Sembol
tasarımcısı bunu söyler). Uzunluklar katmanın
biriminde, ekranda aygıt pikseline çevrilir (kâğıt mm çizim ölçeğinde ve görünümün ölçeğinde; ekranda sabit sembol boyutunda mm doğrudan
px'e; px aygıt piksel oranıyla).

| Tür (`type`) | Adı | Değerler |
|---|---|---|
| `dropShadow` | Gölge | `distance`, `angle` (derece, kuzeyden saat yönünde; 135 sağ alt), `blur`, `color`, `opacity` |
| `outerGlow` | Dış parıltı | `spread`, `blur`, `color`, `opacity` |
| `innerShadow` | İç gölge | `distance`, `angle`, `blur`, `color`, `opacity` |
| `innerGlow` | İç parıltı | `spread`, `blur`, `color`, `opacity` |
| `blur` | Bulanıklaştır | `blur` |

Katmanın bütün nesnelerdeki ilkelleri bir **efekt grubunda** ayrı bir resme (önceden çarpılmış alfa) çizilir, sonra efektleriyle
birleştirilir. Sıra sabittir (listenin sırası değil): gölge, dış parıltı, katmanın kendisi (bulanıklaştırılmışsa bulanık), iç gölge,
iç parıltı. `A` katmanın alfası, `Bσ` standart sapması `σ = blur / 3` (piksel) olan Gauss bulanıklığı (çekirdek ±⌈3σ⌉, ağırlıklar
toplamı 1), `Dₛ` `s` piksellik kare yayma (en büyük süzgeç), `T(dx, dy)` kaydırma:

- Gölge `S = c · o · Bσ(T(A))`, dış parıltı `G = c · o · Bσ(Dₛ(A))`; katmanın altına: `R = K + (S ⊕ G) · (1 − K.a)` (`⊕` kaynak-üstü).
- İç gölge `I = c · o · (1 − Bσ(T(A))) · A`, iç parıltı `J = c · o · Bσ(Dₛ(1 − A)) · A`; katmanın üstüne kaynak-üstü.
- Bulanıklaştır katmanın kendisini (`K`, dört kanal) `Bσ` ile bulanıklaştırır.
- Büyük `σ`'da bulanıklık küçültülmüş resimde yapılır: `σ ≤ 8` tam, `8 < σ ≤ 32` yarım, `σ > 32` çeyrek çözünürlük (kutu süzgeciyle
  küçültme, çift doğrusal büyütme); `σ` en çok 64, `s` en çok 64 pikseldir.

Basit çizginin `blur` ve `shift`'i olduğu gibi kalır (ilkelin kendi yumuşak kenarı).

### 3. Maskeler

Katmanın **katman çizimine** (`LayerNode.compose`, aşağıda §4) **maske** (`mask`): `labels` (katmanın yerleşen etiketleri), `markers`
(katmanın işaret ilkelleri), `size` (ekranda mm, 0–10) ve `targets` (maskelenen katmanların kimlikleri: başka katmanlar, grup değil; en çok
64). Maskelenen katmanın, maskenin şeklinin altındaki pikselleri silinir (alfası `1 − m` ile çarpılır): yazının ve işaretin çevresinde
alttaki çizgiler boşalır, altındaki katmanlar görünür. Dosyada ağaçta olmayan hedef kalabilir (silinen, birleştirilen, başka çizimden gelen
katman); çizilirken atlanır. Komut hedefleri yazarken denetler.

- **Şekil.** Etiketin yerleşen kutusu (yönlü dikdörtgen; eğri boyunca etiketin her harfinin kutusu) ve işaretin çizilen kutusu, `size`
  kadar büyütülmüş, köşeleri `size` yarıçapıyla yuvarlanmış; kenar bir piksellik yumuşaklıkla. Etiket motorunun o kare yerleştirdikleri;
  gizlenen ve yerleşmeyen etiket (ve onun harfleri) maske değildir. Kutunun dönüşünün kosinüs ve sinüsü CPU'da çift duyarlıkta yapılır,
  pikselin kutudaki yeri parçanın kendi konumundan bulunur: dönük dörtgenin köşelerini rasterleyicinin alt piksel ızgarasına oturtması kapsamı
  kaydırmaz.
- Bir katman kendini maskelemez; aynı katmanı maskeleyen kaynakların şekilleri birleşir. Eş yükselti eğrisi yazısının zemin kutusu
  (ADR 0212) değişmez: zeminli etiketin zemini maskenin etkisini örter; maske için etiket eğri boyunca haleyle yazılır.

### 4. Katman saydamlığı ve karışım kipi

Katmanın katman çiziminde (`LayerNode.compose`) **saydamlık** (`opacity`, 0–1; yoksa 1; arayüzde Saydamlık yüzdesi `1 − opacity`) ve
**karışım kipi** (`blend`, yoksa `normal`). Görünüm kiplerinin Saydamlık'ı kapalıyken (ADR 0195) katmanın saydamlığı 1 sayılır. Saydamlığı
1'den küçük, kipi `normal` olmayan ya da maskelenen katman ayrı bir resme çizilir ve çizimin o ana kadarki resmine (zemin, opak) bu kurallarla
birleştirilir: `Cs` katmanın önceden çarpılmış rengi, `αs` alfası (saydamlık ve maskeyle çarpılmış), `cs = Cs/αs`, `cb` alttaki renk:
`co = (1 − αs) · cb + αs · B(cb, cs)`; toplamada `co = min(1, cb + Cs)`.

| Kip (`blend`) | Adı | `B(cb, cs)` |
|---|---|---|
| `normal` | Normal | `cs` |
| `multiply` | Çoğalt | `cb · cs` |
| `screen` | Ekran | `cb + cs − cb · cs` |
| `overlay` | Bindirme | `hardLight(cs, cb)` |
| `darken` | Koyulaştır | `min(cb, cs)` |
| `lighten` | Açıklaştır | `max(cb, cs)` |
| `colorDodge` | Renk soldurma | `cb = 0` ise 0; `cs = 1` ise 1; değilse `min(1, cb / (1 − cs))` |
| `colorBurn` | Renk yakma | `cb = 1` ise 1; `cs = 0` ise 0; değilse `1 − min(1, (1 − cb) / cs)` |
| `hardLight` | Sert ışık | `cs ≤ ½` ise `cb · 2cs`; değilse `screen(cb, 2cs − 1)` |
| `softLight` | Yumuşak ışık | W3C'nin tanımı (`D(cb)`'li) |
| `difference` | Fark | `|cb − cs|` |
| `exclusion` | Dışlama | `cb + cs − 2 · cb · cs` |
| `addition` | Toplama | (yukarıda) |

Renkler sRGB kodlu değerlerle karıştırılır (tarayıcının ve QGIS'in yaptığı gibi).

### 5. Nerede

- **Çizim alanı** iki platformda (masaüstünde wgpu, web'de WebGPU ve WebGL2) ve aynı kurallarla: efekt grupları, maskeler, katman
  birleştirmesi. Efekti, maskesi ve kipi olmayan katman bugünkü tek geçişten çizilir; ayrı resim yalnız gerekene açılır.
- **Sembol önizlemeleri** (tasarımcı, Stil yöneticisi, Lejant): geometri üretici ve efektler; masaüstünde stilli boru hatları, web'de
  Canvas2D aynı tanımlarla (bulanıklık Gauss çekirdeğiyle, yayma kare süzgeçle). Katmanın saydamlığı, kipi ve maskesi katmana aittir,
  önizlemede yoktur; Katman stili penceresinin önizlemesi gösterir.
- **Kapsam dışı:** Büyüteç, Genel bakış, paftanın PDF'i ve yazdırma (geometri üretici çizilir, efektler, maske, saydamlık ve kip yok
  sayılır), DXF ve öbür dışa aktarmalar (geometri üreticinin şekilleri yazılmaz), seçimin ve üzerine gelmenin vurgusu.

### 6. Veri, komut ve arayüz

- **Sembol katmanları** (geometri üretici, `effects`) sembolün JSON'undadır: kitaplıkta ve işleyicide opaktır, `.kcad` şeması değişmez.
- **Katman düğümü** (`LayerNode`) `compose`'u (`opacity`, `blend`, `mask`) tipli taşır: `.kcad` şema 41 (`FORMATS_VERSION` 53), bağımsız
  Python okuyucusu ve yazıcısı, sunucuda katman ağacının kuralı. Bir şey söylemeyen yazılmaz: 1 saydamlığı, `normal` kip, kapalı kaynak, boş
  katman çizimi. Komutu `cad.layers.compose` v1 (`layer`, `opacity`, `blend`, `mask`; adım “Katman
  çizimi”; retler: katman yok, grup, aralık dışı saydamlık, bilinmeyen kip, maskenin boyu, kendini ya da grubu ya da bulunmayan katmanı
  maskeleme); iki platformda, başsız sunucuda, Python'da ve MCP'de.
- **Sembol tasarımcısı:** katman türlerine Geometri üretici (adımlar listesi: ekle, sil, yukarı, aşağı; her adımın değerleri ve ƒ ile
  ifadesi; sembolün sınıfı); her katmanın formunda **Efektler** (beş türün açma kutuları ve değerleri). Modeli iki platformda ortak
  (`fixtures/style/v1/designer.json`).
- **Katman stili:** **Katman çizimi** bölümü: Saydamlık, Karışım kipi, Maske (Etiketler, İşaretler, Boy, Maskelenen katmanlar).

### 7. Performans

Geometri üretici nesne başına derlemede bir kez çalışır (katman kurulumunun parçası; kirlenmeyen katman yeniden kurulmaz). Efekt grubu,
maske ve katman birleştirmesi yalnız çizim alanı yeniden çizilirken çalışır (tutulan resim, ADR 0120; web'in temel resmi), ayrı resimler
boyutları değişmedikçe bir kez açılır ve yeniden kullanılır, büyük bulanıklık küçültülmüş resimde yapılır. Efektlerin haritaları paketlidir
(bir dokuda bir satırın ya da sütunun dört pikseli: x boyunca geçiş sütunlara, y boyunca satırlara paketli haritayı okur, aynı dokunuşlar
dört pikseli birden süzer); bulanıklığın dokunuşları kenardan uzakta doğrusal süzgeçle ikişer okunur; bir grubun bütün efektleri tek
geçişte birleşir; çizim resmine giden ardışık geçişler tek render geçişidir ve çoklu örnekli resim yalnız kopyalanmadan önce ve sonda
çözülür. Bütçeler (release, bu makine):
10 000 nesnede iki adımlı geometri üretici (tampon, daire dilimi) katman kurulumuna ≤ 150 ms ekler; 1440×900'lük çizim alanında üç efektli
bir katman, maskelenen bir katman ve çoğaltan bir katman kareye ≤ 8 ms ekler (masaüstü; web'de ≤ 2,5 katı).

## Uygulama

- **Sözleşme ve dosya:** `kentos_contracts::layer_compose` (`LayerCompose`, `LayerMask`, `BlendMode`; `problem`, `composes_problem`,
  `mask_targets`), `LayerNode.compose`; `cad_layers::LayersCompose` (`cad.layers.compose` v1, katalogda); KCAD kodeki `encode/compose.rs`,
  `decode/compose.rs` (şema 41, `FORMATS_VERSION` 53), `docs/specs/kcad-v2.md` §6.5; bağımsız `tools/kcad/kcad.py` ve
  `scripts/fixtures/kcad_v2_reference.py` (`compose.kcad` ve sekiz bozuk dosya); sunucunun ağaç kuralı (`http/compose_tests.rs`).
- **Komut:** masaüstü `kentos_native_application::layers_compose`, web `product/` kaydı; başsız sunucu, Python (`kentos.cad.layers.compose`)
  ve MCP; ortak durumlar `fixtures/commands/v1/cad.layers.compose.json` (`layer_compose_command_cases.py`, 31 durum). Çizimler arası
  alışveriş katman çizimini taşır (`exchange_cases.py`).
- **Geometri üretici:** stil çekirdeği `style/generate.rs` (adımlar, `chords`), `style/model.rs` (`Step`), `style/compile.rs`
  (`emit_generator`, efekt grupları); web `style/primitives.ts` ve `render/styledBatches.ts` gruplarla; bağımsız başvuru
  `geometry_generator_cases.py` → `fixtures/style/v1/generator.json` (39 durum; stil çekirdeğinin `generator` testi).
- **Çizim alanı:** WGSL `shaders/wgsl/compose/compose.wgsl` ve sözleşmesi `compose.layout.json` (2. sürüm: 128 baytlık sayılar, altı
  resim; `packFs`, `rowsFs`, `columnsFs`, `hFs`, `vFs`, `downFs`, `downWideFs`, `blurWideFs`, `effectFs`, `overFs`, `blendFs`, `boxVs`/`boxFs`);
  masaüstü `crates/render/wgpu/src/compose.rs` (`Planner`, `Composer::record`; yürüyüşleri birleştiren kayıt, çözmeyi yalnız gerekince yapan
  `targets.rs`'in `attachment_kept`'i) ve `renderer.rs`'in `plan_composed`'u; web `render/composePlan.ts` (aynı plan),
  `webgpu/composer.ts`, `webgl2/composer.ts` ve GLSL ikizi `webgl2/composeShaders.ts`; etiketlerin maske kutuları masaüstünde
  `viewport.rs`'in `with_mask_labels`'ı, web'de `viewport/compositions.ts`.
- **Arayüz:** Sembol tasarımcısı masaüstünde `style/designer/` (`generator.rs`: adımlar ve “Adım ekle” menüsü ikonlarıyla; formun
  Efektler'i) ve stil yerlisi `designer/steps.rs`, `designer/effects.rs`; web'de `ui/style/layerForms.ts` (`STEP_ICON`), `SymbolDesigner.ts`,
  önizlemede efektler `render/effectsCanvas.ts`; modelin ortak durumları `fixtures/style/v1/designer.json`. Katman stili'nin Katman çizimi
  bölümü masaüstünde `style/layer_style/compose.rs`, web'de `LayerStyleDialog.ts`'in `composeSection`'ı. İkonlar `geometryGenerator`,
  `layerCompose` ve Sınırlayıcı kutu adımının `stepEnvelope`'u; öbür adımlar uygulamanın aynı işlemdeki ikonları.

## Doğrulama

- **Bağımsız başvurular:** `geometry_generator_cases.py --check` (39 durum) ve `effects_cases.py --check` (numpy; 8 efekt, 13 karışım
  kipi, 2 maske sahnesi; 8 bit saklama ve grubun efektlerinin tek geçişte birleşmesi GPU'nunki gibi) KentOS kodu olmadan; KCAD'in bağımsız
  okuyucusu 586 dosyada tutarlı; komutun 31 durumu iki platformda, başsız sunucuda, Python'da ve MCP'de.
- **Gerçek GPU (masaüstü):** `KENTOS_GPU_TESTS=1 cargo test -p kentos-render-wgpu --test gpu`, Intel UHD Graphics 770 (Mesa 26.0.8,
  Vulkan): 23 sahne 1× ve 4×, en büyük fark 1/255 (tolerans 3/255).
- **Tarayıcı:** `(cd apps/web && node scripts/e2e/compose.mjs)`: WebGPU (SwiftShader) ve WebGL2 (ANGLE Vulkan, aynı GPU), 1× ve 4×, 92
  denetim, en büyük fark 1/255.
- **Ortak iz:** `symbol-extras.json` iki platformda (`layerCompose` beklentisi); resimler 1440×900 ve 1100×650, açık ve koyu temada.
- **Süreler** (release, bu makine: Intel UHD Graphics 770, 1440×900):
  - Geometri üretici (`cargo test --release -p kentos-native-style --test all generator_timing -- --ignored --nocapture`): 10 000 noktada
    tampon ve daire dilimi kuruluma +38,1 ms ekler, 10 000 parselin ağırlık merkezinde düzgün altıgen +3,2 ms (bütçe 150 ms ✓).
  - Katman çizimi masaüstünde (`cargo test --release -p kentos-render-wgpu --test gpu composition_timing -- --ignored --nocapture`):
    çoğaltan kabartma, 300 etiketle maskelenen yollar ve gölge, dış parıltı, iç parıltılı 2 000 bina kareye 1×'te +6,03 ms, 4×'te +6,84 ms
    ekler (bütçe 8 ms ✓). İlk uygulama (her efekt ayrı haritalar, dokunuş başına üstel işlev, her geçişin kendi çözmesi) 19,1 ve 21,7 ms'ydi;
    zaman damgalarıyla geçiş geçiş ölçülerek indirildi: üstel işlev yinelemeye, haritalar paketlilere (x boyunca sütunlara, y boyunca
    satırlara), bulanıklığın dokunuşları ikişer okumaya, grubun efektleri tek geçişe, çizim resminin geçişleri tek render geçişine.
  - Web'de aynı sahne (`(cd apps/web && node scripts/e2e/compose.mjs --time)`, makinenin GPU'sunda): WebGPU 1×'te +5,7 ms, 4×'te +6,2 ms;
    WebGL2 1×'te +5,8 ms, 4×'te +6,6 ms (bütçe 20 ms ✓).
