# ADR 0186: Tarama ekleri

- **Durum:** kabul edildi (2026-10-06). Sıra sahibin kararıdır: TODOS.md §16.1, `CAD-19`'un ardından `CAD-23`; sahibin 5 Ekim
  kararıyla madde tek parçada biter. Ayrıntılar bu ADR'nin varsayılanlarıdır. Örnekler AutoCAD'in Tarama ve Degrade'si (desen adları,
  ölçek ve açı, ilişkili tarama, adalar ve yazılar), Netcad'in Alan Taramaları (Tara, Çoklu Tara, içteki yazılar hariç), ArcGIS'in ve
  QGIS'in çizgi desenli dolgularıdır.
- **Bağlam belgesi:** TODOS.md `CAD-23` (ilgili `CAD-06`, `CAD-24`); ADR 0062 (Tarama aracı), ADR 0175 §4 (nesneye bağlı yazının iki
  belgede izlenmesi), ADR 0157 (dünyaya bağlı desenlerin evresi), ADR 0183 (yazının yüzü), ADR 0025 (`.kcad` v2).

## Bağlam

Tarama bugün dört hazır desenden birini alır: 45° çizgiler, 45° çapraz, yatay çizgiler, dolu (ADR 0062). Desen kitaplığı, ölçek, sınırı
izleyen tarama, içteki yazıları açık bırakma, birden çok alanı birlikte tarama ve degrade yok. DXF'in HATCH'i desen satırlarıyla gelir
ama ilk çizgi ailesine indirgenir, degrade düz dolguya döner (içe aktarma raporunda söylenir).

## Karar

### 1. Desen türleri

Taramanın deseni (`pattern`) şu türlerden biridir; `angle` derece, saat yönünün tersine:

- `solid` **Dolu** (bugünkü).
- `lines`, `cross` **Çizgili** ve **Çapraz** (kullanıcı tanımlı): `angle` çizgilerin açısı, `spacing` aralıkları (m); çapraz ikinci
  aileyi 90° döndürür. Bugünkü taramalar böyle kalır.
- `pattern` **desen**: `name` (ANSI31 …), `angle` desenin dönüşü, `scale` desen biriminin metresi (> 0), `lines` tanımı: çizgi
  aileleri, her biri `angle` (derece), `origin` (taban noktası), `offset` (`[x, y]`: bir sonraki çizginin çizgi boyunca kayması ve
  çizgiye dik uzaklığı, çizginin kendi doğrultusunda) ve `dashes` (çizgi, boşluk sırası: artı çizilir, eksi boşluk, 0 nokta), desen
  biriminde, dönmemiş. Tanım taramanın içindedir: kitaplıkta olmayan (DXF'ten gelen) desen de çizilir. En çok 64 aile, ailede en çok 16
  kesik; çizimde ailenin ilk 8 kesiği kullanılır.
- `gradient` **degrade**: `gradient` = `shape` (`linear` doğrusal, `cylinder` silindir, `spherical` küre), `inverted` (ters) ve
  `color2` (ikinci renk, `#RRGGBB`); ilk renk taramanın rengidir; `angle` degradenin doğrultusu.

`spacing` yalnız çizgili ve çapraz desenindir; ötekilerde 1 yazılır ve okunmaz.

### 2. Desen kitaplığı

Çekirdekte (`geom::hatch_pattern`), iki platformda aynı; adlar AutoCAD'in alışılmış adları, tanımlar bu ADR'nin (birim kâğıtta mm):

- **ANSI** (kesit taramaları): ANSI31 demir, tuğla, taş duvar; ANSI32 çelik; ANSI33 bronz, pirinç, bakır; ANSI34 plastik, kauçuk;
  ANSI35 ateş tuğlası; ANSI36 mermer, arduvaz, cam; ANSI37 kurşun, çinko, yalıtım; ANSI38 alüminyum. 45° aileler, 3,175 mm aralık
  (1/8 inç) ve katları; ANSI36 ve ANSI38'in kesikli aileleri satır satır kayar.
- **ISO** (ISO 128 çizgi tiplerinden, yatay, 5 mm aralık, W100: d = 1 mm): ISO02W100 kesikli (12, 3), ISO03W100 aralıklı kesikli (12, 18),
  ISO04W100 uzun kesikli noktalı (24, 3, 0,5, 3), ISO05W100 iki noktalı, ISO06W100 üç noktalı, ISO07W100 noktalı (0,5, 3), ISO08W100
  uzun ve kısa kesikli (24, 3, 6, 3), ISO09W100 uzun ve iki kısa kesikli, ISO10W100 kesikli noktalı (12, 3, 0,5, 3), ISO11W100 iki
  kesikli noktalı, ISO12W100 kesikli iki noktalı, ISO13W100 iki kesikli iki noktalı, ISO14W100 kesikli üç noktalı. Sekizden çok kesikli
  ISO15 kitaplıkta yoktur.
- **Genel:** LINE yatay çizgiler, NET kare ızgara, NET3 üçgen ızgara (0°, 60°, 120°), DASH kaydırmalı kesikli çizgiler, DOTS kaydırmalı
  noktalar, BRICK tuğla örgüsü, CROSS kaydırmalı artılar.

Her desenin Türkçe açıklaması ve ikonu (desenin kendisinden üretilen küçük resim) vardır.

### 3. Çizim

- Bir desenin her ailesi stil motorunun bir tarama boyası olur (iki platformda `hatchFill`): doğrultusu `angle` + desenin açısı,
  aralığı, dik evresi, kesikleri ve kesiklerin evresi; yeni `stagger` alanı satırdan satıra çizgi boyunca kaymadır (gölgelendiricide satır
  numarası kadar kaydırılır). Evreler ADR 0157'nin kuralıyla karoya göre katlanır; kayma katlanan satır sayısı kadar evreye eklenir.
- Boşlukla başlayan kesik dizisi çizgiyle başlayacak biçimde döndürülür, evre o kadar kayar; 0 uzunluklu kesik (nokta) en az bir piksel
  çizilir (çizgi tiplerinin noktaları da).
- Degrade yeni `gradientFill` boyasıdır: piksel başına `t` (doğrusal: `angle` doğrultusunda halkanın kutusunda 0'dan 1'e; silindir:
  ortada 1, kenarlarda 0; küre: kutunun ortasından en uzak köşeye 1'den 0'a; ters: 1 − t), renk ilk renkle ikinci arasında.
- Dolu desen bugünkü gibi %45 örtüyle çizilir; degrade tam örtüyle.

### 4. Tarama aracı

İstem: `Tarama: taranacak yerin içine tıklayın [Desen (D): ANSI31 / Ölçek (Ö): 1 / Açı (Ç): 0° / Sınır (B): kapalı nesne / Adalar
(A): taranmaz / İlişkili (İ): açık / Yazılar (Y): taranır]`; degradede Desen'den sonra `İkinci renk (R): #FFFFFF` ve `Ters (T)`.

- **Desen (D):** çipin menüsü bütün desenleri ikonlarıyla sıralar: Çizgili, Çapraz, Dolu, kitaplık, Degrade doğrusal, silindir ve küre;
  tuş sıradakine geçer, adı komut satırına yazılarak da seçilir (büyük küçük harf ve i ile ı ayrılmaz: `ansi31`; degrade biçimiyle de:
  `küre`, çünkü komut satırında boşluk Enter'dır).
- **Ölçek (Ö):** desenin kâğıttaki büyüklüğü (1: kitaplığın); taramanın ölçeği `ölçek × çizim ölçeği / 1000`. Çizgili ve Çapraz 3 mm
  × ölçek aralıklı ve 45° + açıdır.
- **Açı (Ç):** desenin dönüşü (derece).
- **İlişkili (İ):** kapalı nesneyle sınırda açık başlar (§6); çizgilerle sınırda tarama ilişkisizdir.
- **Yazılar (Y):** “boş bırakılır” iken bölgedeki görünen yazılar (yazı, çok satırlı yazı) ve blok yerleştirmeleri kutularıyla bölgeden
  çıkarılır; yazının kutusu çevresinde yüksekliğinin dörtte biri boşluk kalır. Noktaların sembolleri ekranda çizildiği için sayılmaz.
- **İkinci renk (R)** yazı kutusunda (`#RRGGBB` ya da renk adı), **Ters (T)** açılıp kapanır.
- Seçenekler oturum boyunca hatırlanır (masaüstünde `Memory`, web'de aracın durağan alanları). Yazma, istem, uyarılar, Enter ve Esc
  ADR 0062'deki gibidir; ileti desenin adını söyler: `ANSI31 tarama eklendi: 272.00 m², 1 ada taranmadı`.

**Çoklu tara** (`hatchSelected`; Netcad'in Çoklu Tara'sı): önce seçim; seçili her kapalı nesne (kapalı alan ve çok parçalının her
parçası, daire, tam elips, kapalı eğri) aynı seçeneklerle, içinden bir noktayla (tohumu) ayrı ayrı taranır, Adalar açıkken her birinin
içindeki kapalı nesneler ada olur. İstem taranacak bölge sayısını söyler, bölgeler kesikli gösterilir; Enter ya da kısa sağ tık hepsini
tek adımda yazar (“Tarama”), araç biter. Seçimde kapalı nesne yoksa söylenir.

### 5. Bölge çekirdekte

`ops::hatch_region`: dış halka (kapalı nesnenin ana hatları), adalar (kapalı nesnelerin ana hatları) ve çıkarılacak kutular (yazılar,
yerleştirmeler) verilince bölge: dış halkadan adalar ve kutular çıkarılır, tohum noktasını (tıklanan yer) içeren parça alınır; tohum
dışarıda kalmışsa en büyük parça. Tarama aracı, Çoklu tara ve ilişkili taramanın yenilenmesi aynı kuralı kullanır.

### 6. İlişkili tarama

Kapalı nesneyle yapılan ve İlişkili açık tarama nesnelerini bilir: `assoc` = `outer` (sınır nesnesinin kalıcı kimliği), `islands`
(adalar), `cutouts` (boş bırakılan yazılar ve yerleştirmeler) ve `seed` (tohum noktası). İki belge onu kayıttan önce izler (nesneye bağlı
yazı gibi, ADR 0175 §4):

- adımın değiştirdiği bir nesnesi olan taramanın bölgesi §5'in kuralıyla yeniden yazılır (aynı adımda, desen ve renk kalır); silinen ada
  ya da yazı listeden düşer;
- sınır nesnesi silinen, kapalı olmaktan çıkan ya da bölgesi boş kalan tarama yerinde kalır, ilişkisi kopar;
- adımın kendisi taramanın halkasını ya da deliklerini değiştirdiyse (tutamaç, köşe tablosu) ilişkisi kopar; nesneleri de aynı adımda
  değiştiyse kural yazar;
- taramayla nesneleri birlikte taşınınca, döndürülünce, ölçeklenince tohum da dönüşür.

Öznitelikler'de `İlişkili: Evet (n nesne)` ve **İlişkiyi kopar** (`cad.entities.set`'in `unlink`'i; adım, nesneye bağlı yazınınki
gibi “Bağı kopar”). Patlatılan ya da DXF'e yazılan tarama ilişkisizdir; DXF'in ilişkili
taraması ilişkisiz alınır ve içe aktarma raporunda söylenir.

### 7. Öznitelikler

Taramanın satırları: Desen (bütün desenler, ikonlarıyla), Açı, Ölçek (desen; çizim ölçeğine göre), Aralık (çizgili ve çapraz), İkinci
renk, Degrade biçimi ve Ters (degrade), İlişkili; Alan ve Çevre bugünkü gibi. Değişiklikler `cad.entities.edit`'in `properties`
işlemiyle yazılır.

### 8. Dönüşümler ve patlatma

Dönüşümler halkayı, delikleri ve tohumu taşır; desenin ve degradenin açısı dönüşün açısını, `scale` ve `spacing` boy ölçeğini alır;
yansımada desen tanımı kendi x eksenine göre aynalanır (`angle` → −`angle`, taban noktası ve kaymanın dik bileşeni ters), açı
`θ − açı` olur. Patlatma deseni kesikleriyle çizgilere çevirir (çekirdeğin `pattern_segments`'i, en çok 20 000 parça); degrade düz
dolgu gibi patlar.

### 9. DXF

- **Okuma:** desen satırları (53, 43–46, 79, 49) bütünüyle alınır: ad (2), açı (52) ve ölçek (41) ile dönmemiş tanıma çevrilir (DXF
  satırları dönmüş ve ölçeklenmiştir). Tek aile ve kesiksiz olan kullanıcı tanımlı desen (76 = 0) çizgili, iki dik aile çapraz olur.
  Degrade (450 = 1): ad (470: LINEAR, CYLINDER, INVCYLINDER, SPHERICAL, INVSPHERICAL; HEMISPHERICAL, CURVED ve tersleri en yakın biçime
  alınır ve söylenir), iki renk (63, 421) ve açı (460, radyan). İlişkililik (71 = 1) alınmaz, söylenir.
- **Yazma:** desen satırları dönmüş ve ölçeklenmiş yazılır (kitaplıkta adı olan desen AutoCAD'in hazır deseni, 76 = 1; başkası özel
  desen, 76 = 2; çizgili ve çapraz 76 = 0), 41 desenin ölçeği çizimin biriminde; degrade 450 = 1 ile, ilk rengi taramanın (yoksa
  katmanının) rengi; 71 = 0, ilişkili tarama söylenir. Desenin ve degradenin tam değerleri KentOS verisindedir; DXF'in grupları onu
  söyledikçe okuyucu onları alır.

### 10. `.kcad` şema 23

Taramanın sütunları desenin `name`, `scale`, `lines` ve `gradient`'ini, taramanın `assoc`'unu taşır (`FORMATS_VERSION` 33); bağımsız
Python okuyucu ve yazıcısı, örnek ve bozuk dosyalar, belge.

### 11. Arayüz

CAD projesinin Açıklama sekmesinde Tarama paneli: Tarama (büyük) ve Çoklu tara (CBS gizler, ADR 0165 §6). İkonlar: desenlerin ikonları
desenden üretilir (`scripts/ui/hatch_icons.py`); Çoklu tara, degrade biçimleri ve ilişkili tarama için çizilen ikonlar sahibin seçimidir.
Desen'in menülerinde (komut satırının ve sağ tıkın seçenek menüsü, Öznitelikler'in Desen'i) her seçeneğin ikonu yerine 56 × 24 piksellik
geniş örneği durur, satır 34 piksel (sahibin seçimi, 6 Ekim: “Geniş şerit”); örnekler de desenden üretilir, ikonun çizgileriyle.

## Kapsam dışı

Desen dosyası (.pat) alma; noktaların sembollerini boş bırakma; tek renkli degrade ve renk tonu; degradenin kaydırılması (AutoCAD'in
“ortalanmamış”ı); çizgilerle sınırda ilişkili tarama; sınırın yeniden oluşturulması; taramanın çizim sırası; büyük ölçekli harita
taramaları (`CAD-24`).

## Uygulama

Tek parçada (6 Ekim). Sözleşme `kentos_contracts::hatch` (`HatchPattern`'in `pattern` ve `gradient` türleri, `name`, `scale`, `lines`,
`gradient`; `PatternLine`, `HatchGradient`, `HatchAssoc`; `problem`'ler komutların ve okuyucunun ortak denetimi); `.kcad` şema 23
(`FORMATS_VERSION` 33; tipli sütunlarda OPT bitleri, bağımsız Python okuyucu ve yazıcısı, `hatches.kcad` ve on dokuz bozuk dosya, belge
§6.1 ve §6.6). Çekirdekte kitaplık ve boyalar `geom::hatch_pattern` (desenin aileleri, kesiklerin çizilişi, `pattern_pieces`, `carried`),
bölge `ops::hatch_region`, aracın seçimleri ve deseni `tools::hatch` (adlar büyük küçük harf ve i ile ı ayrılmadan, degrade biçimiyle de);
dönüşümler, Vektör oturtma, Esnet, tutamaçlar, patlatma ve karşılaştırma deseni ve tohumu taşır; depo yeni alanları paketler. Stil
motorunda `hatchFill`'in `stagger`'ı ve yeni `gradientFill` (çerçevesi alanın dış halkasından, partide köşeler gibi çapaya göre); WGSL
sözleşmesi 4. sürüm (degrade boru hattı; kayma `rect.x`'te), wgpu, WebGPU ve WebGL2'de, masaüstünün CPU çizicisinde ve web'in
sembol önizlemesinde. İki belgede ilişkili
taramanın izlenmesi kayıttan önce (masaüstü `kentos_domain` `hatch_ties.rs`, web `model/hatchTies.ts`; ortak belge durumları
`hatch-ties.json`); kopyalar (Kopyala, Dizi, blok tanımı, Yapıştır) bağı bırakır (`geometry::unlinked`, `withoutLink`).
`cad.entities.set`'in `unlink`'i (adım “Bağı kopar”). Araçlar: Tarama'nın seçenekleri iki platformda (masaüstü
`kentos_interaction::hatch_options`, web `tools/hatchOptions.ts`), Çoklu tara (`hatch_selected.rs`, `tools/hatchSelectedTool.ts`),
Öznitelikler'in satırları (`properties::hatch_rows`, `ui/properties/hatchRows.ts`); CAD şeridinin Açıklama'sında Tarama paneli, CBS
gizler. Desenlerin ikonları ve Desen menülerinin geniş örnekleri desenden üretilir (`scripts/ui/hatch_icons.py`: `HATCH_ICONS`,
`HATCH_PREVIEWS`; web `iconPreview`, `PopupMenu`'nün `preview`'ü; masaüstünde KentOS UI menüsünün `preview`'ü ve `Glyph::wide`'ı,
envanterden), Çoklu tara, degrade biçimleri ve ilişkili tarama ikonları sahibin seçtikleri. Sembol tasarımcısının modeli degrade
katmanını tanır (adı “Degrade”, yeni katmanı ve özeti; katman ekle menüsü onu sunmaz), iki platformda `designer.json`'la. DXF: HATCH'in desen satırları bütün olarak okunur ve dönmemiş tanıma çevrilir (aynalı ve eşit ölçeklenmeyen
yerleştirmede de), degrade adı, açısı ve iki rengiyle; yazılırken satırlar dönmüş ve ölçeklenmiş, kitaplığın adı hazır (76 1), başkası
özel (76 2), degrade 450–470 gruplarıyla, ilişki yazılmaz ve söylenir; desenin ve degradenin tam değerleri KentOS verisinde (`hatch`,
sözleşmenin JSON'u), gruplar onu söyledikçe (`dxf/emit/pattern.rs`, `writer/entities.rs`). Ortak izler `hatches.json` (yeni
seçeneklerle) ve `hatch-patterns.json`; oynatıcılara taramanın `pattern` ve `assoc` beklentisi, `points`'i ve `holes`'u. Uygulanırken
bulunan ve düzeltilen: web'in komut alanları taramanın bağını düşürüyordu (`FIELDS.hatch`, dönüşümlerde `SHAPE_FIELDS`), web aracı bağı
kalıcı kimlikle (`uidOf`) yazmıyordu, web'in çizim okuyucusu deseni ve degradeyi tanımıyordu (`model/hatchRules.ts`, masaüstününkiyle
sözcüğü sözcüğüne; sonlu olmayan geometri iki platformda `invalid_hatch`'ten önce `not_finite`), masaüstünün İkinci renk sorusu boş değerli seçenek yazıyordu, pafta vektör çıktısı tarama açısını derece
sanıyordu, degradenin çerçevesi partinin köşeleriyle aynı düzlemde değildi.

## Doğrulama

- `python3 scripts/fixtures/hatch_pattern_cases.py --check` (kitaplık, kesikler, boyalar, parçalar, taşıma, bölge; ADR'den, KentOS
  kodu olmadan); çekirdeğin `tests/all/hatch.rs`'i ve web `model/ops/hatchPatterns.test.ts` (WASM'dan aynı durumlar).
- `python3 scripts/fixtures/kcad_v2_reference.py --check` (şema 23, bozuk dosyalar); `kentos-kcad` testleri (`hatches.rs`, sütunlar),
  web `io/kcad.wasm.test.ts`.
- `python3 scripts/fixtures/dxf_write_reference.py --check` (`hatches.dxf`); `kentos-formats`'ın `hatch-patterns.dxf` okuma ve
  `hatches_read_back_as_they_were` testleri; web `io/dxf.wasm.test.ts`.
- `python3 scripts/fixtures/set_command_cases.py --check` (`unlink`); iki platformun komut testleri; ortak belge durumları
  `hatch-ties.json`.
- `python3 scripts/ui/hatch_icons.py --check`; stil motoru, WGSL sözleşmesi (`styled_contract`) ve CPU çizicisinin testleri;
  çekirdekte degradenin çapaya göre çerçevesi (`a_gradient_s_frame_is_written_from_the_anchor`).
- Ortak izler `hatches.json`, `hatch-patterns.json` ve `usage-parcel.json` iki platformda üç türde.
- Resimler: `(cd apps/web && node scripts/e2e/shots.mjs hatches)`,
  `KENTOS_SHOTS_ONLY=tarama-araci,tarama-desenler,tarama-coklu,tarama-iliskili,tarama-oznitelikler,tarama-desen-menusu,tarama-oznitelikler-desen cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`
  (`.run/shots/arac-tarama-*`).
