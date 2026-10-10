# ADR 0217: Katman bölme ve birleştirme

- **Durum:** kabul edildi (2026-10-11). Kapsamı ben belirledim (sahibin sözü: “Benim seçmeme gerek yok sen sıradan devam et”). İki
  platform (masaüstü ve web); `.kcad` şeması değişmez; ikonlar sorulmadan seçilir; ilke “Performance First”. Madde tek parçada biter.
- **Bağlam belgesi:** TODOS.md `GIS-21`; ADR 0177 (Katmanlar panelinin Katmanları birleştir, Katmana kopyala ve Kopyasını oluştur'u),
  ADR 0199 (katman alanları), ADR 0211 (katman süzgeci), ADR 0216 (Alan eşleme), ADR 0084 (İşlemler), ADR 0153 §6 (doğal sıra).
- **Araştırma:** QGIS'in Split vector layer, Merge vector layers (alanların birleşimi, `layer` alanı), Extract selected features, Delete
  duplicates by attribute ve Order by expression'ı; ArcGIS'in Split By Attributes (birden çok alan), Merge (alan eşleme, kaynak bilgisi)
  ve Delete Identical'ı (alanlar ve şekil, ilk kayıt kalır).

## Bağlam

Katmanlar panelinin Katmanları birleştir'i (ADR 0177) bütün nesneleri bir katmana taşır ve kaynakları siler; alan eşlemez, kaynağı
yazmaz. Bir katmanı bir özniteliğin değerlerine göre katmanlara ayırmak (mahalleye, paftaya, kullanıma göre), birkaç katmanı alanlarını
eşleyerek tek katmanda toplamak, seçilenleri bir katmana almak, yinelenen kayıtları temizlemek ve nesneleri bir ifadeye göre sıralı
yazmak İşlemler'in işidir: girdisi kapsamla seçilir, modelde adım olur, raporu vardır, tek geri alma adımıdır.

## Karar

### 1. Kapsam

İşlemler'in yeni **Katman** kategorisinde beş araç, iki platformda:

1. **Özniteliğe göre böl** (`layers.splitByAttribute`).
2. **Vektör katmanlarını birleştir** (`layers.merge`; QGIS'in Türkçe adı: panelin Katmanları birleştir'inden ayrı).
3. **Seçilenleri çıkar** (`layers.extract`).
4. **Yinelenenleri sil** (`layers.deleteDuplicates`).
5. **İfadeye göre sırala** (`layers.orderBy`).

İşlemler'e üç ek (§7): parametre türü **Katmanlar** (`layers`), aracın açtığı katmanlar ve gruplar (`RunResult.layers`) ve yeni çıktı
katmanının görünüşünü ve alanlarını bir katmandan alması (`RunResult.like`); masaüstünün yamasına katman (`Patch.layer`). Sıralamanın kuralı
çekirdekte, iki platformda aynı (§6). **Kapsam dışı:** dosyaya bölme (bir katman bir dosya; seçilenleri dosyaya kaydetmek ADR 0193'tür),
nesnelerin çizimdeki sırasını yerinde değiştirme, toleranslı geometri eşitliği, birleştirmede koordinat sistemi dönüşümü.

Kilitli katmandaki nesnenin kopyası yapılmaz (CLAUDE.md §7): beş aracın girdisi kilitli katmanın nesnelerini almaz (İşlemler'in yazılan
girdisi gibi, söylenir).

### 2. Özniteliğe göre böl

- **Girdi** nesneler (katman, seçim, tümü, görünen), **Alanlar** (bir ya da birkaç), **Nesneler**: Kopyala (varsayılan: girdi olduğu gibi
  kalır) ya da Taşı.
- Bir nesnenin değeri alanlarının değerleridir, boşluklar atılarak; değeri olmayan alan “(boş)”; birden çok alanda değerler “ – ” ile
  birleşir. Her değer bir katman olur, değerlerin girdide ilk görüldüğü sırayla; adı değerdir. Katmanlar yeni bir grupta: adı alanların
  adları (“Mahalle”, “Mahalle, Pafta”), girdinin ilk nesnesinin katmanının hemen üstünde; o yerde aynı adlı düğüm varsa “ (2)”, “ (3)”.
  Her yeni katman, değerinin ilk nesnesinin katmanının görünüşünü ve alanlarını alır.
- En çok 1000 değer: daha çoksa çalıştırma reddedilir (“n farklı değer var; en çok 1000 katmana bölünür. Daha az değerli bir alan
  seçin.”).
- Kopyalar nesnenin bütün özelliklerini taşır (bağlı yazı bağsız, ilişkili tarama ilişkisiz: ADR 0175, 0186'nın kopya kuralı); Taşı
  nesnelerin katmanını değiştirir.
- Tablo **Değer**, **Nesne** (değer katmanın adıdır); özet “n nesne k katmana bölündü (kopyalandı).” ya da “(taşındı)”.

### 3. Vektör katmanlarını birleştir

- **Katmanlar** (yeni `layers` parametresi; en az iki), **Alan eşleme** (ADR 0216 §2; hedef ve kaynak sütunlar girdilerin alanlarının
  birleşimi; boşsa bütün öznitelikler kendi adlarıyla), **Kaynak katman alanı** (metin; varsayılan “Kaynak katman”, boşsa yazılmaz),
  **Sonuç** katmanı (yeni ya da var olan; varsayılan yeni “Birleşik”).
- Nesneler kopyalanır (girdiler değişmez): katmanlar ağaçtaki sırasıyla, her katmanda çizim sırasıyla. Eşleme varsa kopyanın öznitelikleri
  yalnız eşlemenin hedefleridir (değer: kaynak sütunun hücresi boş değilse o, değilse varsayılan); sonra kaynak katman alanı katmanın
  adıyla.
- Sonuç var olan, alanları tipli bir katmansa değerler alanların kuralıyla yazılır (ADR 0216 §4): uymayan değer yazılmaz, sayılır;
  katmanın varsayılanları boş alanlara gelir; zorunlu alanı boş kalan nesne eklenmez, sayılır.
- Özet “k katmandan n nesne “X” katmanına birleştirildi.”; iletiler: uymayan değerler, eklenmeyen nesneler.

### 4. Seçilenleri çıkar

- **Girdi** nesneler (varsayılan kapsam seçim), **Nesneler** Kopyala ya da Taşı, **Sonuç** katmanı (varsayılan yeni “Seçilenler”). Yeni
  sonuç katmanı ilk nesnenin katmanının görünüşünü ve alanlarını alır. Taşı'da sonuç katmanındaki nesneler olduğu gibi kalır.
- Özet “n nesne “X” katmanına kopyalandı.” ya da “taşındı”.

### 5. Yinelenenleri sil

- **Girdi** nesneler (yazılır), **Alanlar** (boşsa bütün öznitelikler), **Geometri de aynı olsun** (varsayılan açık). Değerler boşluklar
  atılarak karşılaştırılır; olmayan değer boş sayılır; bütün özniteliklerde boş değerli alanlar yok sayılır. Geometri kesin eşitlikle:
  türü ve şekli (köşeler, yaylar, kotlar) aynı; sayılar değerleriyle (−0 ile 0 aynı sayıdır).
- Girdinin sırasıyla ilk nesne kalır, aynı anahtarlı sonrakiler silinir; tek geri alma adımı.
- Tablo **Değer**, **Silinen** (yinelenen grup başına, ilk yinelenenin sırasıyla; değer alanların değerleri “ – ” ile, alan
  seçilmemişse “ad: değer” çiftleri “; ” ile, hiç değer yoksa “—”); özet “k grupta n yinelenen nesne silindi.”, yinelenen yoksa
  “Yinelenen nesne yok.”

### 6. İfadeye göre sırala

- **Girdi** nesneler, **İfade** (değer), **Yön** Artan ya da Azalan, **Boşlar** Sonda ya da Başta, **Sıra alanı** (varsayılan “Sıra”;
  boşsa yazılmaz), **Sonuç** katmanı (varsayılan yeni “Sıralı”): nesnelerin kopyaları sıralı yazılır (Öznitelik tablosu ve çizim sırası
  onların sırasıdır), sıra alanına 1, 2, 3 …; yeni sonuç katmanı ilk nesnenin katmanının görünüşünü ve alanlarını alır (QGIS'in Order by
  expression'ı gibi yeni katman).
- Sıralamanın kuralı çekirdekte (`ops::order`, iki platformda aynı): değeri olmayan (boş, sonsuz ya da tanımsız sayı) Boşlar'ın yerinde;
  türler karışıksa evet/hayır, sayı, metin sırasıyla; evet/hayır'da hayır önce, sayılar değerle (−0 ile 0 aynı), metinler doğal sırayla
  (ADR 0153 §6); Azalan değerlerin sırasını çevirir; eşitler ve boşlar girdinin sırasıyla.

### 7. İşlemler'e ekler

- **Katmanlar** (`layers`): değer katman kimliklerinin listesidir; pencerede katmanlar (grupları değil) ağacın sırasıyla, yollarıyla ve
  nesne sayılarıyla işaretli liste; `min` en az işaretli sayı (denetim: “en az n katman seçin”). Çalıştırıcı listeyi ağacın sırasıyla
  katman katman nesnelere çözer (süzgecin dışarıda bıraktıkları ve kilitli katmanlar alınmaz, söylenir); olmayan katman “seçilen katman
  artık yok”tur. Modelin girdisi olamaz.
- **Aracın açtığı katmanlar** (`RunResult.layers`): her biri bir anahtar, ad, grup mu, üst düğümü (aynı çalıştırmanın grubu), yeri (bir
  katmanın hemen üstü) ve görünüşünü ve alanlarını aldığı katman. Nesneler ve yamalar katmanı anahtarla anar; çalıştırıcı tek adımda
  açar, kimliği adından verir (`islem-<ad>`, alınmışsa “-2” eklenerek: yeni katman parametresinin kuralı) ve adı yerinde alınmışsa “ (2)”.
- **`RunResult.like`**: çalıştırmanın katman parametreleriyle açılan yeni katman bu katmanın görünüşünü ve alanlarını alır.
- **`Patch.layer`** (masaüstü; web'in yaması nesnenin her alanını alır): nesnenin katmanı.

### 8. Arayüz

- Pencereler tanımdan; tablolar formun altında; Katmanlar listesi Girdi'de. CBS'de Veri sekmesinin Katman panelinde beş araç Yeni
  katman, Yeni grup ve Katmanları göster'in yanında (dar pencerede yalnız ikonlarıyla); Analiz'in İşlemler menüsünde değil (sekme 1100
  pikseli aşıyordu); araç kutusunda Katman kategorisi; CAD'de İşlemler'den.
- **İkonlar** (sorulmadan): §Uygulama.

### 9. Performans

Bütçeler (release, bu makine): 100 000 parselde bölme (50 değer, kopya) masaüstünde ≤ 300 ms, web'de ≤ 800 ms; yinelenenler (100 000,
iki alan ve geometri) ≤ 200 / 500 ms; sıralama (100 000, sayı ifadesi, kopya) ≤ 300 / 800 ms; iki 50 000'lik katmanın birleştirilmesi
≤ 300 / 800 ms (denetim, araç, tek adım).

## Uygulama

- **Çekirdek:** `ops::order` (`Key`, `order`; işlem `orderBy`) sıralamanın kuralı, doğal sıra `text::natural`'ın `tokens` ve `tokens_cmp`'siyle
  (`natural_cmp` de onları kullanır); web'de `model/ops/order.ts` (`orderOf`).
- **İşlemler (iki platform):** parametre türü `layers` (masaüstünde `ParamKind::Layers`, `LayerSet`; çözüm `features::resolve_layers`, özet
  `summarize_layers`; web'de `LayersParam`, `LayerSet`, `job.ts`'in `LayersRef`'i, `features.ts`'in `resolveLayers`'ı); aracın katmanları
  `ToolLayer` ve `RunResult.layers`, `RunResult.like`; çalıştırıcı (`runner.rs`, `runner.ts`) katmanları ve grupları tek adımda açar
  (`new_layer_id`/`newLayerId`, `free_name`/`freeName`), anahtarları kimliklere çevirir; masaüstünün yamasında `Patch.layer`. Kategori
  `layers` (ikon `layers`).
- **Araçlar:** masaüstünde `kentos-processing`'in `builtin/layers/`'ı (`split`, `merge`, `extract`, `dedupe`, `order_by`; ortakları
  `mod.rs`'te: kopya, alanların değeri, Kopyala ve Taşı), web'de `processing/builtin/layers/` (`shared.ts`). Kopya nesnenin kimliği
  dışında her alanıdır; bağlı yazının ve ilişkili taramanın kopyası bağsızdır (masaüstünde `unlinked`).
- **Yinelenenleri sil'in hızlı yolu:** nesneler önce değer sınıflarına ayrılır (anahtar metin ya da ödünç değerler); geometri açıkken eşit
  şekillerin ortak yanı (tür, köşe sayısı, ilk yer; −0 ile 0 aynı) ön süzgeçtir, şekil yalnız bunu başka bir nesneyle paylaşanlarda okunur.
  Masaüstünde şeklin JSON'u metin tutulmadan özetlenir (FNV-1a; “-0.0” “0.0” olarak), aynı özetliler eşitlikle ayrılır; web'de anahtarları
  sıralı JSON (`shapeKey`). Tablonun değer metni yalnız gruplar için yazılır.
- **Pencere:** masaüstünde `processing/fields.rs`'in `layers_list`'i (onay kutulu katmanlar, yolları, nesne sayıları; kilitli soluk ve
  kilitli), `window.rs`; web'de `ui/processing/paramFields.ts`'in `layersField`'ı, kuralları `fieldPlan.ts`'in `layersView` ve
  `withChosenLayer`'ı. Şerit `app/ribbon.ts` (envanterin `ribbonByMode`'u masaüstünün şerididir).
- **İkonlar** (sorulmadan seçildi): Özniteliğe göre böl `layerSplitAttr` (bir katman üçe ayrılır), Vektör katmanlarını birleştir
  `layersMergeVector` (iki katman birine), Seçilenleri çıkar `extractSelected` (seçili nesne katmana taşınır), Yinelenenleri sil
  `deleteDuplicates` (çizilmiş yinelenen kare), İfadeye göre sırala `orderByExpression` (ok altında sıralı çizgiler).

## Doğrulama

- **Bağımsız başvuru:** `scripts/fixtures/layer_ops_cases.py` (KentOS kodu olmadan ADR'den; doğal sıra, alan ve şekil eşitliği kendi
  yazımıyla) 16 durumu yazar: bölme (kopya, taşıma, alınmış adlar, kilitli katman), birleştirme (bütün öznitelikler, tipli hedef alanları,
  kilitli katman, tek katman), çıkarma (kopya ve taşıma), yinelenenler (bütün öznitelikler ve geometri, alanlar, yinelenen yok, kotun
  sıfırının işareti), sıralama (alan ve metin); `fixtures/processing/v1/layers.json` ve `layers.kcad`. İki platformun oynatıcıları
  (`cases.rs`'in `the_layer_tool_cases_do_what_they_say`'i, `cases.test.ts`) hepsini çizimde ve okuma kopyasında (web'de işçide) geçer;
  oynatıcılar yeni grupları, katmanın kimliğini, üst düğümünü ve alanlarını da denetler.
- **Pencere:** masaüstünde `processing::layer_tools_tests` (her durum pencereden kullanıcı gibi doldurulur); pencerenin formları ve
  metinleri `fixtures/processing/v1/dialog.json`'da iki platformda.
- **Resimler:** masaüstünde `cargo test -p kentos-desktop processing::layer_tools_tests::screens -- --ignored --nocapture
  --test-threads=1` (`.run/shots/islem-katman-*`), web'de `(cd apps/web && node scripts/e2e/shots.mjs layertools)`; yerleşim
  `(cd apps/web && node scripts/e2e/layout.mjs --only processing-layers-merge,processing-layers-split)` (1100×650 ve 1440×900, iki
  tema).
- **Süreler** (11 Ekim, bu makine, release; p50 / p95, denetim, araç ve tek adım):

  | İş | Masaüstü | Web | Bütçe |
  |---|---|---|---|
  | Özniteliğe göre böl, 100 000 parsel, 50 değer, kopya | 221,2 / 231,9 ms | 259,5 / 354,0 ms | 300 / 800 ms |
  | Yinelenenleri sil, 100 000 parsel, iki alan ve geometri | 125,5 / 128,1 ms | 133,7 / 135,4 ms | 200 / 500 ms |
  | İfadeye göre sırala, 100 000 parsel, `$alan`, kopya | 245,8 / 247,9 ms | 352,5 / 390,7 ms | 300 / 800 ms |
  | Vektör katmanlarını birleştir, iki 50 000'lik katman | 198,5 / 200,9 ms | 263,0 / 386,9 ms | 300 / 800 ms |

  Masaüstünde parçaları (hazırlık, araç, uygulama): bölme 29,6 / 111,7 / 79,4 ms, yinelenenler 29,8 / 93,8 / 1,5 ms, sıralama
  29,5 / 152,7 / 60,9 ms, birleştirme 9,8 / 123,6 / 65,7 ms. Yinelenenleri sil ilk yazımında bütçeyi aşıyordu (masaüstü 238,5 ms, web
  511,4 ms: her nesnenin anahtarı ve şekli metin); değer sınıfları, özetlenen şekil ve ön süzgeçle yukarıdaki sürelere indi. Komutlar:
  `cargo test --release -p kentos-processing --test cases layer_timing -- --ignored --nocapture`; web'de
  `KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm; LAYERS_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/layers.test.ts
  --disable-console-intercept; rm apps/web/src/wasm/pkg/.stamp`.
