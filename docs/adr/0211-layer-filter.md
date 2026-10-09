# ADR 0211: Katman süzgeci (tanım sorgusu)

- **Durum:** kabul edildi (2026-10-09). Kapsamı sahibin GIS-10'daki sözüyle (“Benim seçmeme gerek yok sen sıradan devam et”) ben
  belirledim: Netcad'in Referans Özellikleri › Süzgeç'i, ArcGIS Pro'nun tanım sorgusu (definition query) ve Generate Definition Query From
  Selection'ı, QGIS'in Query Builder'ı. İki platform (masaüstü ve web), bulut, Python ve MCP; ikonlar sorulmadan seçilir; ilke
  “Performance First”. Madde tek parçada biter.
- **Bağlam belgesi:** TODOS.md `GIS-15`; ADR 0100 (ifade dili), ADR 0091 (kurallı işleyici), ADR 0199 (katman alanları, Öznitelik
  tablosu), ADR 0200 (sorgular), ADR 0178 (Veride ara), ADR 0210 (zaman sürgüsünün süzgeci), ADR 0014 (kalıcı kimlik).

## Bağlam

Büyük bir katmanın yalnız bir kısmıyla çalışmak sık iştir: yalnız “Arsa” nitelikli parseller, yalnız bir mahallenin binaları, yalnız
denetlenecek yüz nesne. Bugün kurallı işleyici ifadeye uymayan nesneyi çizmez (ADR 0091), ama nesne seçilir, kenetlenir, Öznitelik
tablosunda ve işlemlerde görünür; katman ağacı da süzüldüğünü söylemez. Netcad bunu referansın süzgeciyle, ArcGIS katmanın tanım
sorgusuyla (ve seçimden tanım sorgusu üretmeyle), QGIS sağlayıcının süzgeciyle yapar: süzülen nesneler o katmanda yokmuş gibidir.

## Karar

### 1. Kapsam

- **Katman süzgeci:** katmanın bir ifadesi (İfadeyle seç'in dili, ADR 0100) ve ya da bir nesne listesi (nesnelerin kalıcı kimlikleri).
  Süzgeçten geçmeyen nesneler katmanda yokmuş gibidir:
  - çizilmez, etiketi ve yazısı çizilmez, Genel bakış'ta görünmez;
  - tıklamayla, pencereyle, çokgenle, Tümünü seç'le, Benzerini seç'le ve İfadeyle seç'le seçilmez, kenetlenmez;
  - Öznitelik tablosunda, İşlemler'in girdilerinde (bütün nesneler, görünenler, katman ve seçim kapsamları) ve Veride ara'da yoktur;
  - katman ağacının sayısı “geçen / bütün” olur.
- **Değişmeyenler:** çizimin kendisi (nesneler silinmez, kaydedilir); komutlar nesneyi kalıcı kimliğiyle bulur (Python ve MCP süzgeçten
  bağımsızdır); dosya alışverişi (`.kcad`, DXF, GeoJSON, Shapefile, koordinat listesi) bütün nesneleri yazar; Veri karşılaştır, Katman
  stili'nin ve Lejant'ın sayıları çizimin bütünüdür. Bunlar veri işidir; süzgeç katmanın görünümüdür.
- **Çizimler arası alışveriş:** Başka çizimden al ve Kaynaklar'ın Katman olarak ekle'siyle açılan katmana süzgecin koşulu gelir, nesne
  listesi gelmez (öbür çizimin nesnelerini adlandırır; gelen nesneler yeni kimlik alır); yalnız listesi olan süzgeç hiç gelmez. Senaryo
  bağlarının kuralıdır (ADR 0210 §9). Seçilenleri dosyaya kaydet süzgeci olduğu gibi yazar (nesneler kimlikleriyle gider).
- **Seçimden süzgeç:** seçili nesnelerin her katmanına o katmanın seçili nesnelerinin listesi yazılır (katmanın ifadesi kalır); tek adım.
- **Süzgeci kaldır.** Katman süzgeci penceresi; katman ağacında süzgeç işareti.
- **Otomasyon:** `cad.layers.filter` komutu, Python ve MCP; sunucu yeni alanın kuralını denetler.
- **Kapsam dışı:** grup süzgeci (bir grubun bütün katmanlarına aynı süzgeç), kayıtlı süzgeç listeleri, süzgeç başına stil, tabloya özel
  saklı süzgeçler (`GIS-01`'in notu), uzamsal süzgeç (bir alanın içi), ilişki üzerinden süzgeç (`GIS-18`'in işlevleri gelince ifade
  kendiliğinden genişler).

### 2. Veri modeli

`kentos_contracts` (katman ağacı):

- **`LayerNode.filter`** (`LayerFilter`, yalnız katmanda; servisten çizilen katmanda olmaz: nesnesi yoktur): `expression?` (metin) ve
  `objects?` (kalıcı kimlik dizisi). Kurallar (`LayerFilter::problem`): ikisinden en az biri; ifade kırpılmış, 1–10 000 karakter; liste
  1–100 000 kimlik, her biri bir kez. İfadenin derlenmesi kural değildir (dosya okuyucusu dili bilmez): komutlar derler.
- **`.kcad` şema 35** (`FORMATS_VERSION` 45): katman düğümünün `filter`'ı; yalnız varken şema 35 yazılır. Kimlikler 16 baytlık
  dizgilerdir (§6.8). Bağımsız Python okuyucusu ve yazıcısı, örnek `filters.kcad`, bozuk dosyalar.
- Sunucu `project.changes`'in ve proje açmanın katman ağacını aynı kuralla denetler.

### 3. Süzgecin değerlendirilmesi

- **İfade** İfadeyle seç gibi derlenir ve değerlendirilir: öznitelikler metindir, geometri değerleri (`$alan`, `$uzunluk`, `$merkez_y` …)
  nesnenin şeklinden; sonuç doğruysa nesne geçer, yanlış, boş ya da hatalıysa geçmez. `$sıra` ve `$ölçek` süzgeçte reddedilir (ikisi de
  çalıştırmaya göre değişir, süzgeç değişmez olmalı).
- **Liste:** nesnenin kalıcı kimliği listedeyse geçer. Çizimde olmayan kimlik hata değildir (nesne silinmiş olabilir).
- **İkisi birden:** ikisinden de geçmeli. Zaman sürgüsü açıkken nesne hem süzgeçten hem pencereden geçmeli (ADR 0210 §6).
- **Derlenmeyen ifade** (dosyadan ya da başka bir istemciden): hiçbir nesne geçmez; katman ağacında işaret uyarı rengindedir ve ipucu
  nedenini söyler. Komut derlenmeyen ifadeyi yazmaz.
- **Ne zaman:** katmanın süzgeci değişince bütün katman; nesneler değişince yalnız değişenler (öznitelik ve geometri), geometri
  deposunda, zamanların yanında bir işaret olarak tutulur. Sorgular (tıklama, kenet, etiket, Genel bakış) işarete bakar.
- **Yeni nesne süzgeçten geçmezse** (çizilen, yapıştırılan, içe aktarılan) görünmez; uygulama bir kez söyler: ““Parsel” katmanının
  süzgecinden geçmeyen 1 yeni nesne görünmüyor; Süzgeç… ya da Süzgeci kaldır.”

### 4. Arayüz

- **Katman süzgeci penceresi** (`layer.filter`): Katman (etkin katman), İfade (ε ile İfade oluşturucu), nesne listesi satırı (“Seçimden:
  12 nesne”, Listeyi kaldır), önizleme “n / m nesne süzgeçten geçiyor” ya da derleme hatası yerinde; Süzgeci kaldır, Vazgeç, Kaydet
  (`cad.layers.filter`). Tek geri alma adımı “Katman süzgeci”.
- **Seçimden süzgeç** (`layer.filterFromSelection`): seçili nesnelerin katmanlarına; seçim yoksa söylenir. **Süzgeci kaldır**
  (`layer.filterClear`): etkin katmanın ya da sağ tıklanan katmanın.
- **Katmanlar paneli:** süzgeçli katmanın huni işareti (ipucunda ifade ve liste), sayısı “geçen / bütün”; sağ tıkta Süzgeç…, Seçimden
  süzgeç, Süzgeci kaldır.
- **Şerit:** CBS'nin Veri sekmesinde Süzgeç paneli (Katman süzgeci, Seçimden süzgeç, Süzgeci kaldır). CAD şeridinde yoktur (komut
  aramasında ve Katmanlar panelinde vardır).
- **Takma adlar:** KATMANSUZGECI, TANIMSORGUSU, LAYERFILTER; SECIMDENSUZGEC; SUZGECKALDIR (SUZGEC ve FILTER Seçim süzgecinindir,
  ADR 0187).
- **İkonlar:** katman ve huni (`layerFilter`); kesikli seçim kutusu, iki tutamaç ve huni (`layerFilterSelection`); huni ve çarpı
  (`layerFilterClear`); ağaçta dolgulu huni (`funnel`; Seçim süzgecinin çizgi hunisinden ayrı), koşul derlenmezse uyarı renginde.
- **Yeni nesne:** süzgeçten geçmeyen yeni nesnenin iletisi bu düzenleyicinin değişikliğinde söylenir; başka bir düzenleyicinin (bulut)
  nesnesi söylenmez.

### 5. Komutlar ve otomasyon

- `cad.layers.filter` v1: `{ layer, filter: LayerFilter | null }`; katmanın süzgecini yazar ya da kaldırır. Kodlar `invalid_filter`
  (kural), `invalid_expression` (derleme; yeri ile), `layer_not_found`, `not_a_layer` (grup), `service_layer`. Sonuç: `changed`, süzgeçten
  geçen ve katmanın bütün nesne sayısı. Geri alma adımı “Katman süzgeci”.
- Python `kentos.cad.layers.filter`; MCP'de komut.

### 6. Performans

- Süzgecin 100 000 nesnede değerlendirilmesi (öznitelik ifadesi) ≤ 50 ms, geometri değeri okuyan ifade ≤ 150 ms; nesne listesi ≤ 10 ms.
- Süzgeç değişince masaüstünde katman bütün olarak, web'de katman yeniden kurulur; nesne değişince yalnız değişenler değerlendirilir.
- Ölçümler Doğrulama'da.

## Uygulama

- **Sözleşme:** `kentos_contracts::layer_filter` (`LayerFilter`, `problem`, `filters_problem`; sınırlar `FILTER_EXPRESSION_MAX`,
  `FILTER_OBJECTS_MAX`), `LayerNode.filter`; `cad_layers`'ın `LayersFilter`, `LayersFiltered`, `LayersFilterPlan`'ı ve katalog kaydı;
  `FORMATS_VERSION` 45. Web'in kuralları `model/layerFilterRules.ts`.
- **KCAD:** `.kcad` şema 35 (`SCHEMA_WITH_FILTERS`), `encode/filter.rs`, `decode/filter.rs`; bağımsız Python okuyucusu
  (`tools/kcad/kcad.py`) ve yazıcısı (`kcad_v2_reference.py`), örnek `filters.kcad`, 11 bozuk dosya `broken/filter-*.kcad`;
  `docs/specs/kcad-v2.md` §6.5.
- **Belge:** masaüstünde `Document::set_layer_filter` ve `Op::LayerFilter` (`kentos_domain`), web'de `CadDocument.setLayerFilter`
  ve `layerFilter` işlemi; ikisi de tek geri alma adımı. Belgenin `by_layer_with_uids`'i katmanın nesnelerini kimlikleriyle tek
  geçişte verir.
- **Değerlendirme:** çekirdek `kentos_native_application::layer_filter` (`compile_filter`, `passes`, `condition`, `left_out`), web
  `model/layerFilter.ts` (`compileFilter`, `filterPasses`, `filterPassesIn`, `filterCondition`, `leftOut`); koşul İfadeyle seç'in sütun
  motoruyla, geometri değerleri nesnenin şeklinden. Geometri deposunun işareti `Store::set_filtered`, `filter_shown`, `view_shown`
  (zaman penceresiyle birlikte), etiketler ve Genel bakış ona bakar; web'de `setFiltered`, `viewMask`. Masaüstünde
  `kentos_interaction::Spatial` süzgeçleri izler (`sync_filters`: süzgeci değişen katman bütün, değişen nesneler tek tek; liste bir kez
  belgenin kimlik dizininden yuvalara çevrilir, koşul yalnız listedekilere sorulur, bütün katmanın sayısı değerlendirmeden), web'de
  `PickIndex` aynısını yapar (`markWhole`, baytlık işaret tablosu `model/idMarks.ts`). Çizim katmanları süzgeçli katmanı geçenlerle
  kurar (masaüstü `style/scene.rs`, web `ViewportController`).
- **Süzgeci görmeyenler:** Tümünü seç ve Benzerini seç (`selecting.rs`, `select_similar.rs`; `selectionCommands.ts`,
  `selectSimilarTool.ts`), Öznitelik tablosu (`features/`, `FeatureTable.ts`), Veride ara (`search/`, `SearchPanel.ts`), İşlemler'in
  girdileri (`kentos-processing`'in `features.rs`'i, web `processing/features.ts`; `Scope::Ids` dışında).
- **Komut:** `cad.layers.filter` v1 masaüstünde `layers_filter.rs` (`kentos-native-application`), web'de `product/layersFilter.ts`,
  başsız sunucu `dispatch.rs`; durumlar `layer_filter_command_cases.py` (30). Python `kentos.cad.layers.filter` (üretilen), MCP'de araç
  (yıkıcı değil). Sunucu `check_tree`'de `filters_problem`.
- **Arayüz:** web `app/layerFilterCommands.ts` (`layer.filter`, `layer.filterFromSelection`, `layer.filterClear`; yeni nesnenin iletisi
  belgenin `touched` olayının `added`'ından), `ui/layers/LayerFilterDialog.ts` (ilk açılışta yüklenir), `LayersPanel` (huni, ipucu,
  “geçen / bütün”, sağ tık); CBS şeridinin Veri sekmesinde Süzgeç (`app/ribbon.ts`). Masaüstü `layer_filters.rs` (pencere, Seçimden
  süzgeç, Süzgeci kaldır, yeni nesnenin iletisi `Spatial::take_hidden_new`'dan), `layering.rs` (sağ tık, sayılar `layer_counted`),
  `view.rs` (huni ve sayı sütunu), İfade oluşturucunun `Target::LayerFilter`'ı.
- **Alışveriş:** `kentos_domain::exchange` ve `model/exchange.ts`'in açılan katmanı; bağımsız başvuru `exchange_cases.py`'nin iki yeni
  durumu (`theirsFilter`).
- **İzler:** `fixtures/interaction/v1/layer-filter.json` (sahne `feature-table.kcad`), yeni beklenti `layerCounts` iki oynatıcıda
  (`tracePlayer.mjs`, `traces/compare.rs`).

## Doğrulama

9 Ekim 2026, geliştirme makinesinde (11. nesil Intel Core i5-11300H, 8 iş parçacığı, Linux; derleme 4 işle):

- **Bağımsız başvurular** (KentOS kodu olmadan, `--check` ile hepsi geçer): `kcad_v2_reference.py` (484 dosya; örnek `filters.kcad` ve
  11 bozuk dosya `tools/kcad/kcad.py` ile), `layer_filter_command_cases.py` (`cad.layers.filter` 30 durum), `exchange_cases.py`
  (süzgeçli kaynağın iki yeni durumu); GIS-12'nin başvuruları değişmedi.
- **Platformlar:** komut durumları web'de, masaüstünde ve Python'da; ortak iz `layer-filter.json` iki platformda bütün varyantlarda;
  resimler iki platformda (`kentos-cad kullan layer-filter`, `e2e:use layer-filter`; `scripts/usage/compare.py`) 1440×900 ve 1100×650'de,
  iki temada.
- **Sunucu:** `check_tree`'nin birim testi ve geçici veritabanında (`KENTOS_TEST_DB=required`) HTTP testi (`http::filter_tests`): nesne
  listesi de ifadesi de olmayan süzgeçle proje açılmaz, geçerli süzgeç saklanır ve okunur, `project.changes`'te aynı nesneyi iki kez
  adlandıran liste reddedilir ve hiçbir şey değişmez. MCP'de araç, katmanın süzgeci ve sorgunun bütün nesneleri (`cargo test -p
  kentos-mcp`, 9).
- **Takım:** `pnpm typecheck`; `pnpm test` (347 dosya, 4242 test geçti, 23 atlandı); `pnpm rust:test` (2864 geçti, 29 yok sayıldı;
  API'nin veritabanı testleri geçici veritabanlarında, atlanan yok; clippy ve bağımlılık yönü temiz); `pnpm rust:test:desktop` (1255
  geçti, 194 yok sayıldı; clippy temiz); `pnpm py:test` (61); `pnpm e2e:interaction` (141 iz × 3 varyant) ve masaüstünde bütün izler
  bütün varyantlarda; `pnpm e2e` (209 denetim); `pnpm build` (Katman süzgeci penceresi ayrı parça: 4,5 kB); `pnpm inventory:check`.
  Önbelleği temizlenen `singleSource` denetimi `labelText.ts`'in metin birleştirmesini (ADR 0175'ten beri) yakaladı; şablon metne çevrildi.
- **Görünüş:** `e2e:layout`'ta Katman süzgeci penceresi, derlenmeyen koşulu ve süzgeçli katmanlı ağaç 1100×650 ve 1440×900'de, iki
  temada (12 görünüm, sorunsuz). İkonlar 16, 20 ve 28 pikselde iki temada denetlendi.
- **Süreler** (release; 99 856 ve 100 000 parsel, üçte biri “Arsa”):

  | İş | Süre | Bütçe |
  |---|---|---|
  | Masaüstü deposu: süzgeç yazılınca bütün katman, öznitelik ifadesi (`perf layer_filter`, yedinin ortancası; en yavaşı) | 13,4 ms (14,9) | 50 ms |
  | Masaüstü deposu: geometri ifadesi (`$alan > 310`) | 19,5 ms (20,5) | 150 ms |
  | Masaüstü deposu: nesne listesi (her ikinci parsel) | 7,6 ms (8,9) | 10 ms |
  | Çekirdek: komutun sayımı (`layers_filter::count`), öznitelik / geometri / liste | 12,0 / 31,1 / 8,0 ms | — |
  | Masaüstü: süzgeçli katmanda bir parselin özniteliği değişince (ortalama; en yavaşı) | 2,9 ms (3,9) | — |
  | Web deposu, gönderilen WASM (`scripts/perf/filter.test.ts`, p50 / p95): öznitelik | 20,9 / 25,4 ms | 50 ms |
  | Web deposu: geometri ifadesi | 22,7 / 26,8 ms | 150 ms |
  | Web deposu: nesne listesi | 7,3 / 11,9 ms | 10 ms |
  | Web: değerlendirmenin kendisi (`filterPassesIn`), öznitelik / geometri / liste (p50) | 18,0 / 19,8 / 4,9 ms | — |

  İlk ölçümde masaüstünün nesne listesi 18,1 ms'ydi: her nesnenin kimliği ikinci bir aramayla okunuyordu (100 000'de 4,3 ms), liste
  her nesne için kimlikle soruluyor ve bütün katman yeniden sayılıyordu. Kimlikler artık nesneyle birlikte okunur (`by_layer_with_uids`),
  liste bir kez yuvalara çevrilir, koşul yalnız listedekilere sorulur, sayı değerlendirmeden yazılır. Web'de ilk ölçüm üç süzgeçte de 36–37
  ms'ydi: kimlik dizisini kuran `Float64Array.from(liste, işlev)` 100 000'de 7 ms tutuyordu (artık döngü; GIS-12'nin zaman okuması da
  aynısını kullanıyordu), dışarıdakiler JS kümesindeydi (artık baytlık tablo), liste her nesne için metin kümesinde aranıyordu (artık bir
  kez kimliklere çevrilir, küme yalnız tek tek gelen nesneler için ve ilk sorulduğunda kurulur), sayılar ikinci geçişteydi ve süzgecin
  değişip değişmediği her seferinde JSON'la soruluyordu (artık önce nesnenin kendisiyle). Web'in liste yolunun p95'i (11,9 ms) bütçeyi
  aşar, ortancası (7,3 ms) içindedir; fark çöp toplamanın dalgalanmasıdır.
- **Bilinen sınırlar:** web süzgeçli katmanı bütün olarak yeniden kurar (parça yok); süzgeçli bir katmanda tek nesnenin değişmesi
  katmanın sayısını yeniden sayar (100 000'de 3 ms). §1'in kapsam dışısı.
