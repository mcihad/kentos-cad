# ADR 0260: Nesnenin görünüşü: kaynağı, çizgi tipi, saydamlık, yazdırma ve çizim sırası

- **Durum:** kabul edildi (2026-10-13). [plan/cad.md](../../plan/cad.md) §6 `CAD-DATA-21`. Kapsamı ben belirledim (sahibin
  sözleri: “CAD data bölümüne geç ve orayı tamamla”; “Araç = komut” kuralı, 13 Ekim).
- **Bağlam belgesi:** ADR 0089 (şeridin Özellikler'i), 0139 (nesnenin çizgi kalınlığı), 0144 (blok), 0009 (DXF), 0090 (stil
  motoru), 0226 (Katman çizimi), 0248 (katman ağacının komutları), 0253 (komut protokolü), 0025 ve `docs/specs/kcad-v2.md`.
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

- **Kaynak:** nesnenin kendi rengi ve kalınlığı vardı; yoksa çizimde katmanınki, bir bloğun içinde yerleştirmeninki çizilirdi
  (ADR 0139, 0144). Bu iki anlam yazılmamıştı; bloğun nesnesinin kendi katmanının görünüşünü izlemesinin yolu yoktu: DXF'in blokta
  BYLAYER nesnesi katmanının rengini sabit değer olarak alıyordu, katmanın rengi değişince izlemiyordu.
- **Çizgi tipi:** yalnız katmandaydı. Şeridin Özellikler'indeki Çizgi tipi oturumda tutuluyor, hiçbir araç okumuyordu (ADR 0089);
  DXF'in nesne çizgi tipi (6) ve ölçeği (48) okunmuyordu.
- **Saydamlık:** yalnız katmanın bütünüydü (Katman çizimi, ADR 0226); DXF'in 440'ı okunmuyordu.
- **Yazdırma:** her katman paftada çizilirdi; DXF'in katman yazdırma anahtarı (290) okunmuyordu.
- **Çizim sırası:** belgenin sırasıydı; değiştiren komut yoktu.
- **Planın isteği:** “Nesne görünüşünde ByLayer/ByBlock/explicit ayrımını, linetype, linetype scale, transparency, plot ve draw
  order alanlarını tasarla.”

## Karar

### 1. Görünüşün kaynağı

Görünüşün dört parçası vardır: **renk** (`color`), **çizgi tipi** (`lineType`), **kalınlık** (`lineWeight`), **saydamlık**
(`transparency`). Her parçanın kaynağı üçten biridir; değer yokluğu nesnenin yerine göre okunur:

| Kaynak | Çizimdeki nesne | Bir bloğun nesnesi |
|---|---|---|
| **Kendi** | değeri | değeri |
| **Katmana göre** | değer yok: katmanınınki | değer yok ve parça `byLayer`'da: kendi katmanınınki; katmanı yoksa (`""`, DXF'in 0'ı) ya da çizimde bulunmuyorsa yerleştirmenin katmanınınki |
| **Bloğa göre** | — | değer yok: yerleştirmenin o parçası (kendi değeri, yoksa yerleştirmenin kaynağı; iç içe bloklarda dıştan içe) |

- Bu bugünkü anlamdır (ADR 0139, 0144): çizimde değer yokluğu katmana, blokta yerleştirmeye göredir. Eski dosyaların, sunucudaki
  blok tanımlarının ve web'in nesneleri göç istemez; anlamları değişmez.
- **`byLayer`** yeni alandır, yalnız bir bloğun nesnesinde olur: görünüşün bu parçaları yerleştirmeninki yerine nesnenin kendi
  katmanınındır (AutoCAD'in blokta BYLAYER'ı). Değeri olan parça `byLayer`'da olmaz; çizimdeki nesnede alan yoktur. Liste
  `color`, `lineType`, `lineWeight`, `transparency` sırasıyla, tekrarsızdır.
- Bloğun nesnesinin görünürlüğü ve kilidi bugünkü gibi yerleştirmenin katmanınındır (ADR 0144); `byLayer` yalnız görünüşü izler.
- **Blok oluştur** nesneleri olduğu gibi alır: çizimde katmana göre olan parça blokta bloğa göre olur (bugünkü gibi).
- **Patlat** (ADR 0144): bloğa göre parçalar yerleştirmenin çözülmüş değerini alır (bugünkü gibi); `byLayer` parçaları değersiz
  kalır, çizime indiği katmana göre çizilir.
- Blokta olmayan bir nesnenin görünüşü katmana göredir; DXF'in çizimdeki BYBLOCK nesnesi bugünkü gibi ön plan rengini (`ink`) alır.

### 2. Çizgi tipi ve ölçeği

- **`lineType`**: `continuous`, `dashed`, `dashdot`, `dotted` (katmanın dört tipi, `LineType`); yoksa kaynağına göre.
- **`lineTypeScale`**: desenin nesneye özgü çarpanı (AutoCAD'in CELTSCALE'i): 0'dan büyük, en çok 1000; yoksa 1. Kaynağı yoktur,
  her zaman nesnenindir; deseni olmayan çizgide (sürekli) bir şey değiştirmez.
- Desenler kâğıt mm'sidir ve çizimin ölçeğiyle çizime iner (katmanlardaki gibi, `line_type_dash`); çarpan deseni büyütür.
- Sembolü (`symbol`) ya da katmanında işleyicisi olan nesnenin çizgileri sembolündür: çizgi tipi ve ölçeği yalnız katmanın düz
  görünüşüyle çizilen nesneye uygulanır (çizgi, çoklu çizgi, alan, daire, yay, elips, eğri, kılavuz, tablo).

### 3. Saydamlık

- **`transparency`**: 0 ile 0,9 arası pay (AutoCAD'in %0–90'ı, adımı serbest); yoksa kaynağına göre. Katmanın nesne saydamlığı
  yoktur (0): katmanın bütününün saydamlığı Katman çizimi'dir (ADR 0226) ve ayrıca uygulanır.
- Nesnenin çizgileri, dolguları, yazıları ve sembolleri bu oranda saydam çizilir; seçim ve üzerine gelme vurgusu saydam değildir.

### 4. Yazdırma

- **`LayerNode.plot`** (yalnız `false` yazılır): yazdırılmayan katman ya da grup ekranda çizilir; paftada, paftanın PDF'inde ve
  yazdırmada çizilmez. Grubunki altındakilerin hepsini kapsar (görünürlük gibi). AutoCAD'in katman yazdırma anahtarı, DXF'in 290'ı.
- Masaüstünde Katmanlar'ın sağ tık menüsünde **Yazdır / Yazdırma**; pafta yazdırılmayan katmanları atlar.
- Nesnenin kendi yazdırma alanı yoktur (AutoCAD'de de katmanındır); yazdırma stilleri (CTB, STB) sonraya (§8).

### 5. Çizim sırası

- Bir katmanın nesneleri belgenin sırasıyla çizilir. Nesnenin **`order`** alanı (tam sayı; yoksa 0) bu sırayı katmanının içinde
  değiştirir: önce `order`'ı küçük olan, eşitlerde belgenin sırası. Katmanların sırası ağacın sırasıdır; çizim sırası katmanı
  aşmaz (KentOS'un çizimi katmanlıdır).
- **Öne getir**: her nesnenin `order`'ı kendi katmanındaki öbür nesnelerin en büyüğünden büyük olur; **Arkaya gönder**: en
  küçüğünden küçük. Taşınanların kendi aralarındaki sırası korunur. Şeritte CAD › Giriş ve CBS › Düzenle'nin Değiştir panelinde
  (`edit.bringToFront`, `edit.sendToBack`).
- **Çizimde bantlar:** stil motoru bir katmanın dolgularını çizgilerinin altına koyar (aynı stilin nesneleri tek partidir). Bir
  nesnesinin `order`'ı olan katmanda her `order` değeri bir **bant**tır (`LayerObjects::bands`, `BatchSink::set_band`): bantlar
  küçükten büyüğe, her biri kendi dolguları altta, çizgileri üstte çizilir; öne getirilen taramanın dolgusu arkasındaki çizgilerin
  üstüne çıkar. `order`'ı olmayan katmanın partileri ve sırası değişmez. Böyle bir katman parçalara bölünmeden bütün kurulur
  (ADR 0121'in parçaları belgenin yerine göredir).
- Bloğun nesneleri tanımın sırasıyla çizilir; `order` çizimdeki nesnenindir.
- Tıklamanın adayları değişmez: aynı yerdeki nesneler arasında Sıradakini seç (ADR 0187) kullanılır.

### 6. Komutlar

- **`cad.entities.set`**: `lineType`, `lineTypeScale`, `transparency` (null: kaynağına göre; ölçekte 1). İşlemler `lineType`
  (“Çizgi tipi değiştir”), `lineTypeScale` (“Çizgi tipi ölçeği”), `transparency` (“Saydamlık değiştir”). Ret:
  `invalid_line_type_scale`, `invalid_transparency`.
- **Oluşturan komutlar** (`cad.line.create`, `cad.polyline.create`, `cad.polygon.create`, `cad.point.create`, `cad.circle.create`,
  `cad.arc.create`, `cad.entities.create` ve onlardan yazan anlamsal komutlar): `lineType`. Şeridin Özellikler'indeki Çizgi tipi
  yeni nesnelere yazılır (ADR 0089'un “hiçbir araç okumaz”ı kalkar).
- **`cad.entities.order`** (yeni): `uids`, `to` (`front`, `back`). Kilitli katmandaki nesne taşınmaz (`layer_locked`); zaten
  yerinde olan değişmez.
- **`cad.layers.set`**: `plot` işlemi (`layer`, `value`), katman ağacının kendi değişikliği (görünürlük gibi, geri alma adımı değil).

### 7. Dosyalar ve sunucu

- **KCAD v2 şema 48**: nesnenin `lineType`, `lineTypeScale`, `transparency`, `order` (çizimin nesnesinde) ve `byLayer` (bloğun
  nesnesinde); katmanın `plot`'u. Yazıcı 48'i yalnız bunlardan biri varken yazar; tipli sütunlarda bayrak 64 (`columns.rs`), `FORMATS_VERSION` 61.
- **DXF okuma**: nesnenin 6'sı (BYLAYER, BYBLOCK ya da LTYPE tablosunun tipi, katmanlardaki gibi dörde indirgenerek), 48'i, 440'ı
  (BYLAYER, BYBLOCK ya da 0x02000000 ile alfa); katmanın 290'ı. Blokta BYLAYER nesne bugünkü gibi katmanının değerini sabit
  alır; `byLayer` üretmek sonraya (§8). Nesneler dosyanın sırasıyla gelir.
- **DXF yazma**: aynı gruplar; nesneler çizim sırasıyla yazılır (`order`'a göre, eşitlerde belgenin sırası; AutoCAD dosyanın
  sırasıyla çizer): yeniden okunan dosyada sıralar 0'dır, katmanların içindeki sıra aynıdır.
- **Sunucu**: göç 0018, `kentos.feature.look` (jsonb: `lineType`, `lineTypeScale`, `transparency`, `order`; varsayılanda null);
  katmanın `plot`'u katman ağacında, bloğun nesnelerinin `byLayer`'ı tanımın JSON'unda gelir ve gider.
- **Python ve MCP**: katalogdan.

### 8. Sonraya kalanlar

- **Adlı çizgi tipleri:** DXF'in LTYPE desenleri, ISO ve ANSI kitaplığı; bugün dörde indirgenir (ADR 0009).
- **Yazdırma stilleri** (CTB, STB) ve nesnenin yazdırma stili.
- **Katmanın nesne saydamlığı** (DXF'in katman saydamlığı); bugün katmanın saydamlığı Katman çizimi'dir.
- **Bloğun nesnesinin kendi katmanının görünürlüğü ve kilidi.**
- **`byLayer`'ı üretmek:** Blok oluştur'da “katmanlarına göre kalsın” seçeneği ve DXF'in blokta BYLAYER nesnesinin `byLayer`
  olarak okunması; bugün alan sözleşmede, KCAD'de, çizimde ve DXF yazıcısında vardır, `.kcad` ve komutlarla gelir.
- **Nesnenin üstüne ve altına** (AutoCAD'in DRAWORDER'ının Above ve Under'ı) ve katmanları aşan çizim sırası.
- **Yazıların ve taramaların toplu sırası** (Yazıları öne getir, Taramaları arkaya gönder).
- **DXF'in SORTENTSTABLE'ı:** okuyucu OBJECTS bölümünü okumaz; nesneler dosyanın sırasıyla gelir.

## Sonuçlar

- CAD çiziminin nesnesi AutoCAD'deki gibi kendi çizgi tipini, ölçeğini ve saydamlığını taşır; şeridin Çizgi tipi yeni nesnelere
  yazılır. DXF'in nesne görünüşü ve katmanın yazdırılması gelir ve gider.
- Blokta katmana göre içerik katmanını izler; eski dosyalar ve sunucudaki tanımlar anlamlarını korur.
- Çizim sırası komutla değişir, dosyada ve sunucuda kalır.
- Web'in okuyucusu ve çizimi bu alanları bilmez; web dönünce TODOS.md §0.2'deki satırla gelir.

## Doğrulama

- Sözleşme: `look_problem` ve `LayerNode.plot`'un testleri.
- Komutlar: `fixtures/commands/v1/cad.entities.order.json` (bağımsız başvuru `scripts/fixtures/order_command_cases.py`),
  `cad.entities.set.json`'un çizgi tipi, ölçek ve saydamlık durumları (`set_command_cases.py`), `cad.layers.set.json`'un yazdırma
  durumları (`layer_tree_command_cases.py`), oluşturan komutların çizgi tipi durumları.
- KCAD: `python3 scripts/fixtures/kcad_v2_reference.py --check`, örnek `look.kcad` ve bozuk dosyalar, Rust kodeğinin testleri.
- DXF: `cargo test -p kentos-formats --test dxf_write an_objects_look_and_a_layers_plot_go_out_and_come_back` (gruplar 6, 48,
  440 ve 290 gider ve gelir; 0,9 saydamlık 440'ta 33554457, 0,5 33554560).
- Sunucu: `an_objects_look_is_a_column_of_its_own` (geçici veritabanında, `KENTOS_TEST_DB=required`).
- Çizim: `kentos-style-core`'un `an_object_brought_to_the_front_draws_its_fill_over_the_strokes_behind`'ı; stil programının çizgi
  tipi, ölçek ve saydamlık testleri.
- Masaüstü resimleri: `KENTOS_SHOTS_ONLY=gorunus-cizim,gorunus-oznitelikler,gorunus-one-getir cargo test -p kentos-desktop
  tools_screens -- --ignored --nocapture` (`.run/shots/arac-gorunus-*`) ve Katmanlar'ın menüsünde Yazdırma
  (`layering::screens`, `katman-secimi-menu-*`).
