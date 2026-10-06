# ADR 0183: Yazı ve ölçü stilleri

- **Durum:** kabul edildi (2026-10-06). Sıra sahibin kararıdır: TODOS.md §16.1, araştırma kaydının önerisiyle `CAD-20`'nin hemen
  ardından `CAD-21`; sahibin 5 Ekim kararıyla madde tek parçada biter. Ayrıntılar bu ADR'nin varsayılanlarıdır. Adlar AutoCAD'in
  Türkçe arayüzünden (“Yazı stili”, “Ölçü stili”); Netcad'in Yapınet › Yazı/Ölçü Stilleri, ArcGIS'in boyut stilleri ve QGIS'in yazı
  biçimleri örnektir.
- **Bağlam belgesi:** TODOS.md `CAD-21`; ADR 0055 (çizimin yazıları: projenin yazı tipi, tek satırlı yazı eğik), ADR 0145 (yazı
  ekleri: genişlik çarpanı), ADR 0147 (ölçü türleri ve zemin), ADR 0182 (çok satırlı yazı), ADR 0146 (kılavuzun ok başları), ADR 0177
  §4 (katman durumları: projenin adlı ayarı), ADR 0025 (`.kcad` v2).

## Bağlam

Yazının yazı tipi projenin tek ayarıdır (`drawingFont`); tek satırlı yazı eğik, çok satırlı yazı dik çizilir. Ölçünün görünüşü
sabittir: uçlarda eğik çentik (değer yüksekliğinin 0,6'sı), uzatma çizgisinin ölçülen noktadan boşluğu ve ölçü çizgisini aşması
0,5 h, değer çizginin 0,35 h üstünde; basamak ve birim projenin. Harita ve proje çizimlerinde yazılar türlerine göre (parsel
numarası, ada, yol adı, not) ayrı yazı tipi, boy ve eğiklikle; ölçüler oklu ya da çentikli, cm ya da mm'de, önek ve sonekle yazılır.
DXF'in STYLE ve DIMSTYLE tabloları okunurken düşüyordu.

## Karar

### 1. Stil bir önayardır; nesne görünüşünü kendisi taşır

- Adlı yazı ve ölçü stilleri projenin ayarıdır (`ProjectSettings.textStyles`, `.dimensionStyles`; `.kcad` şema 21): her biri bir
  kimlik (UUIDv7), ad ve değerler. Projeyle gider; bulut veritabanı projesinde projenin ayarlarıyla.
- Nesne stiline kimliğiyle bağlıdır (yazının `textStyle`'ı, ölçünün `dimStyle`'ı) ve stilin değerlerini **kendi alanlarında** taşır.
  Çizim, seçme, kutu, kayıt, DXF ve Python nesnenin alanlarını okur; stil tablosu çizim için okunmaz. Böylece çekirdek, depo ve bütün
  çiziciler nesneyle yetinir; DXF'in nesne başına ayarları (TEXT'in 41 ve 51'i, ölçünün DSTYLE'ı) doğrudan karşılanır; bulutta ayar ve
  nesneler ayrı gelse de çizim tutarlıdır.
- **Stili uygulamak** (araçta seçmek, Öznitelikler'de değiştirmek) nesnenin bağını ve görünüş alanlarını stilinkilere eşitler.
- **Stil değişince** (pencerede Kaydet) stili kullanan nesnelerde, alanı stilin **eski** değerinde olanlar yeni değeri alır; başka
  değerde olan alan nesnenin kendi ayarıdır, kalır (AutoCAD'in nesne başına geçersiz kılmaları gibi). Bu nesne değişikliği tek geri
  alma adımıdır (“Yazı stili”, “Ölçü stili”); stillerin kendisi projenin ayarıdır, geri alma adımı değildir (katman
  durumları gibi, ADR 0177 §4). Blok tanımlarının içindeki yazılar ve ölçüler tanımla değişir; stil değişikliği onlara dokunmaz.
- **Yeniden adlandırma** yalnız stili değiştirir (bağ kimliktir). **Silme:** stili kullanan nesne yoksa siler; varsa sorar (“N
  nesne bu stili kullanıyor; görünüşleri kalır, stilsiz olurlar”); Kaydet'te nesnelerin bağı aynı adımda (“Yazı stili”, “Ölçü
  stili”) kalkar, kilitli katmandakiler olduğu gibi kalır ve sayılır.
- Bağı tabloda olmayan bir kimliği gösteren nesne (başka projeden yapıştırılmış, bulutta silinmiş stil) stilsiz sayılır ve kendi
  alanlarıyla çizilir; komutlar bilinmeyen kimliği reddeder (`unknown_style`).
- **Standart:** stilsiz yazı ve ölçü. Yazıda projenin yazı tipi ve bugünkü görünüş (ADR 0055); ölçüde bugünkü görünüş ve 2,5 mm
  değer. Pencerede ilk satırdır, düzenlenmez (yazı tipi Proje ayarları'nda).

### 2. Yazı stili

- Alanlar: `name`; `font` (yedi çizim yazı tipinden biri); `bold`, `italic`; `oblique` (eğiklik, derece, −85 ile 85 arası, 0
  yazılmaz; artı harflerin üstünü sağa yatırır); `height` (kâğıtta mm; yoksa yükseklik stilin değil, aracın); `widthFactor` (yazının
  aralığında, 1 yazılmaz); `fontFile` (DXF'ten gelen yazı tipi dosyası; yazılırken aynen geri yazılır).
- Yazının yeni alanları: `textStyle`, `font`, `bold`, `italic`, `oblique`. **Yazı tipi olan yazı stilin görünüşüyle çizilir:** o
  aile, dik 400 ağırlık; `bold` 600, `italic` eğik yüz (eğik yüzü olmayan ailede ADR 0055'teki kural); `oblique` harfleri o açıyla
  yatırır. Çok satırlı yazıda dilimlerin kalın ve eğiği bunlara eklenir. **Yazı tipi olmayan yazı bugünkü gibidir** (projenin yazı
  tipi; tek satırlı eğik, çok satırlı dik); `bold`, `italic` ve `oblique` yazı tipi olmadan reddedilir (`invalid_style`).
- Ölçü: kalın yazının ilerlemesi kalın tablodan (ADR 0182); eğiklik yazının kutusunu yatırır: dış çizgi, zemin, seçme ve kapsam harflerin
  yattığı paralelkenardır. Hiza ilerlemelerle hesaplanır, eğiklik onu değiştirmez. Çok satırlı yazı kutusuyla yatar: her satırın
  başı taban çizgisinin yüksekliği kadar kayar. Eğik yüzü olmayan ailede (Barlow ve Courier Prime dışındakiler) eğik, harfleri
  0,25 (yaklaşık 14°) yatırarak çizilir; web'de tarayıcının yaptığı gibi.
- Uygulamak: `textStyle`, `font`, `bold`, `italic`, `oblique`, `widthFactor` stilinkiler olur; stilin yüksekliği varsa yazının
  yüksekliği `height / 1000 × ölçek` olur, yoksa kalır. Standart'ı uygulamak bu alanları kaldırır (yükseklik kalır).

### 3. Ölçü stili

- Alanlar: `name`; `height` (değerin kâğıttaki yüksekliği, mm, zorunlu); `arrow` (Dolu ok `closed`, Açık ok `open`, Nokta `dot`, Yok
  `none`; yoksa Çentik); `arrowSize`, `extOffset` (uzatma çizgisinin ölçülen noktadan boşluğu), `extBeyond` (ölçü çizgisini aşması),
  `textGap` (değerin çizgiden yüksekliği) mm; `textPlace` (`centre`: değer çizginin ortasında; yoksa üstünde); `decimals` (0–8; yoksa
  projenin uzunluk basamakları); `unit` (`m`, `cm`, `mm`; yoksa projenin birimi); `prefix`, `suffix` (en çok 32 karakter, satır sonu
  yok); `font` (değerin yazı tipi; yoksa projenin).
- Ölçünün yeni alanları: `dimStyle`, `arrow`, `arrowSize`, `extOffset`, `extBeyond`, `textGap`, `textPlace`, `decimals`, `unit`,
  `prefix`, `suffix`, `font`. Boylar **değer yüksekliğinin katıdır** (stilde mm / stilin yüksekliği): Ölçekle'de yükseklikle birlikte
  büyür, ölçünün bütün parçaları tek boydan gelir. Olmayan alan bugünkü değerdir: Çentik 0,6; ok ve nokta 1; boşluk ve aşma 0,5;
  değer 0,35.
- Uçlar: hizalı, doğrusal, açı, yarıçap, çap, yay uzunluğu ve kırıklı yarıçapın ölçü çizgisinin (yayının) uçlarında, bugün çentiğin
  durduğu yerde. Dolu ok ve açık ok Kılavuz'unkiyle aynı biçimde (boy s, eni s/3), ucu ölçü çizgisinin ucunda, gövdesi çizginin
  içinde; nokta s/2 çaplı dolu daire. Koordinat, semt ve eğim kendi işaretlerini korur (okları anlamlarıdır).
- `textGap` değerin taban çizgisinin ölçü çizgisinden uzaklığıdır (koordinat dışında). `centre`: değerin ortası çizginin üstünde
  (taban çizgisi 0,35 h aşağıda), çizgi değerin altında görünmez (değerin kutusu zemin gibi doldurulur, yanlarda paylı).
- Değer: `‹önek›‹türün öneki›‹sayı›‹sonek›`. Uzunluk ve koordinat `unit`'te `decimals` basamakla; eğim `decimals` (yoksa 2) basamakla;
  açılar projenin açı yazımıyla (basamak onlara uygulanmaz).
- Uygulamak: `dimStyle` ve görünüş alanları stilinkiler (boylar mm / yükseklik) olur; ölçünün yüksekliği `height / 1000 × ölçek`.
  Standart'ı uygulamak alanları kaldırır, yükseklik 2,5 mm'ninki olur.
- Patlat: çizgiler, dolu uçlar düz taramalı alan (Kılavuz'unki gibi), değer ölçünün yazı tipiyle yazı.

### 4. Araçlar

Stillerin arayüzü CAD'indir (sahibin 6 Ekim sözü: “CAD sadece CAD arayüzünde”; ADR 0165 §6): araçların Stil seçeneği,
Öznitelikler'in stil satırları ve pencerenin komutları yalnız CAD projesinde görünür. CBS projesinde araçlar ve Öznitelikler bugünkü
gibidir; CBS projesine gelen stilli nesneler (DXF, yapıştırma) kendi görünüşleriyle çizilir.

- Yazı ve Çok satırlı yazı'da **Stil (S)**: çipin menüsünde Standart ve projenin yazı stilleri, en altta “Yazı stilleri…”. Seçmek
  aracın Yükseklik'ini (stilin yüksekliği varsa) ve Genişlik'ini stilinkine getirir; yazılan yazı stilin görünüşünü alır. Çok satırlı
  yazı'nın Satır aralığı (R) olur (S artık Stil'dir; araçların hepsinde aynı harf).
- Ölçülendirme'nin bütün yöntemlerinde, Hızlı ölçü'de, Zincir ve Baz ölçü'de **Stil (S)**: ölçü stilleri ve “Ölçü stilleri…”;
  ölçünün yüksekliği ve görünüşü stilden.
- Metin dosyası yerleştir Yazı'nın stilini kullanır. Seçilen stiller oturumda hatırlanır (Yükseklik gibi); projede olmayan stil
  Standart'a döner.
- Zincir ve Baz ölçü yeni ölçüleri, AutoCAD'in DIMCONTINUEMODE 1'i gibi, taban ölçünün stili ve görünüşüyle yazar; kendi Stil
  seçenekleri yoktur (S, Ölçü seç'tir).
- Stilin adı büyük küçük harf ayırmadan yazılır; ad boşluk taşıyabildiği için stil adının adımında Boşluk komut satırında harftir,
  yalnız Enter onaylar (araçların `takes_words`'ü, web'de `takesWords`; başka her adımda Boşluk AutoCAD'deki gibi ikinci Enter'dır).

### 5. Yazı ve ölçü stilleri penceresi

- `style.textStyles` “Yazı stilleri…” (STYLE, ST, YAZISTILI) ve `style.dimensionStyles` “Ölçü stilleri…” (DIMSTYLE, DST, OLCUSTILI)
  aynı pencerenin iki hâlidir (biri yazı, biri ölçü stillerini tutar). STYLE takma adı AutoCAD'deki anlamıyla Stil yöneticisinden
  Yazı stilleri'ne geçti. Solda stiller (Standart ilk; her birinin kullanan nesne sayısı), Yeni (seçilinin kopyası) ve
  Sil; ad formda değişir; sağda seçili stilin formu ve canlı önizlemesi (örnek yazı ya da hizalı bir ölçü, iki platformun kendi
  çizicisiyle, çizimin zemininde ve mürekkebiyle). Form taslaktır: Kaydet projeye yazar ve §1'in kuralıyla nesneleri tek adımda günceller; Vazgeç, Esc, × bırakır.
- Ad: kırpılmış, 1–64 karakter, satır sonu ve denetim karakteri yok, büyük küçük harf ayırmadan tek; “Standart” ayrılmıştır.
- Yerleri: CAD şeridinin Açıklama sekmesinde Yazı ▾ ve Ölçü ▾, Araçlar menüsünün Stil bölümü (CAD'de Yönet); CBS'nin gizleme
  listesinde (`app/workspaces.ts`).

### 6. Öznitelikler

- CAD projesinde yazıda **Yazı stili**, ölçüde **Ölçü stili** satırı: menüde Standart ve stiller; çoklu seçimde farklıysa “Çeşitli”. Seçmek stili
  uygular (tek adım “Değiştir”). Bağı kopuk nesnede Standart görünür. Görünüş alanlarının ayrı satırı yoktur.

### 7. DXF

- **Okuma:** içe aktarılan yazıların ve ölçülerin kullandığı STYLE ve DIMSTYLE kayıtları projenin stili olur. Okuyucu onları
  karşılaştıkça `dxf-text-1`, `dxf-dim-1` … diye numaralar (`ImportResult`'ın `textStyles`'ı ve `dimensionStyles`'ı; boyları
  okuma seçeneklerinin `scale`'inde kâğıtta mm, eksik yazı tipi `drawingFont`); uygulama içe aktarırken projede aynı adda (büyük
  küçük harf ayırmadan) stil varsa onu kullanır, dosyanın değerleri nesnelerin kendi alanlarında kalır; yoksa stili yeni bir
  kimlikle (UUIDv7) projeye ekler; projenin tutamayacağı stil (adı “Standart”, değerleri kural dışı) stilsiz kalır, nesnesi
  görünüşüyle. Yeni stiller nesneler girdikten sonra eklenir (büyük dosyada son kare); ayardır, içe aktarmayı geri almak onları
  bırakır. Pencerenin iletisi sayar (“2 yazı stili ve 1 ölçü stili projeye eklendi”).
  - **Standard, Standart'tır** (KentOS'unki de başka programınki de): stil olmaz. Standard'daki yazı projenin yazı tipiyle,
    yüzsüz gelir (Standart'ın yazı tipi projenin ayarıdır); Standard'daki ölçünün görünüşü kaydın değerleri ve kendi DSTYLE'ıyla
    ölçünün kendi alanlarına gelir. DIMTXSTY Standard'ı gösteriyorsa değerin yazı tipi projenin olur. Böylece stilsiz çizimin
    dosyası bugünküyle bayt bayt aynıdır (ADR 0142'nin testi).
  - STYLE: 3 yazı tipi dosyası (ya da ACAD verisindeki aile adı) aileye: `arial*`, `arimo*` Arimo; `cour*` Courier Prime; KentOS'un
    yedi ailesinin adları kendileri; başkası (SHX yazı tipleri, Times, Calibri…) projenin yazı tipi, rapor dosyanın adıyla söyler.
    Kalın ve eğik ACAD verisinin bayraklarından ya da dosya adından (`arialbd`, `ariali`, `arialbi`, `courbd`, `-Bold`, `-Italic` …).
    40 sabit yükseklik (mm'ye çevrilir; 0 yok), 41 genişlik, 50 eğiklik; `fontFile` dosyanın adı. KentOS'un yazdığı kayıt stilin
    kendisini KENTOS verisinde (`face`) taşır ve aynen geri gelir; stilsiz yüzler için yazdığı `KENTOS_‹AİLE›` kaydı (`styleless`)
    stil olmaz, yüzünü verir.
  - TEXT: 7 stil; 41 kendi genişliği, 51 kendi eğikliği. MTEXT: 7 stil; genişlik ve eğiklik stilin (`\W` ADR 0182'deki gibi).
  - DIMSTYLE: 140 DIMTXT, 41 DIMASZ (0 ise ok yok, Yok), 142 DIMTSZ (sıfırdan büyükse çentik, boyu `√2 × DIMTSZ`), DIMBLK adından ok
    (`""` ve `_CLOSEDFILLED` dolu ok; `_OPEN`, `_OPEN30`, `_OPEN90`, `_CLOSEDBLANK`, `_CLOSED` açık ok; `_DOT`, `_DOTSMALL`, `_DOTBLANK`,
    `_SMALL` nokta; `_NONE` yok; `_OBLIQUE`, `_ARCHTICK` çentik; başkası dolu ok, söylenerek), 42 DIMEXO, 44 DIMEXE, 147 DIMGAP
    (mutlak değeri), 77 DIMTAD (0 orta, başkası üst), 271 DIMDEC, 3 DIMPOST (`<>`'nin önü önek, arkası sonek; `<>` yoksa sonek), 144
    DIMLFAC (dosyanın biriminden değerin birimine: metrede 100 cm, 1000 mm; başkası söylenir), 40 DIMSCALE (boylar onunla
    çarpılır), 340 DIMTXSTY'nin yazı tipi. Ölçünün DSTYLE verisi stilin üstüne gelir. Başka programın ölçüsünden KentOS ölçüsü
    olarak yalnız koordinat, yay uzunluğu ve kırıklı yarıçap gelir (ADR 0147 §8); öbürlerini blokları çizer.
- **Yazma:** STYLE tablosuna Standard (bugünkü gibi, bayt bayt) ve projenin her yazı stili: 3 `fontFile` ya da ailenin dosyası
  (Arimo `arial.ttf`, kalın `arialbd.ttf`, eğik `ariali.ttf`, ikisi `arialbi.ttf`; Courier Prime `cour.ttf`, `courbd.ttf`,
  `couri.ttf`, `courbi.ttf`; öbürleri `‹Aile›.ttf`), ACAD verisinde aile adı (Arimo Arial, Courier Prime Courier New) ve
  TrueType, kalın, eğik bayrakları; 40 sabit yükseklik (çizimin biriminde), 41, 50; KENTOS verisinde stilin kendisi. Stili
  olmayan ama yazı tipi olan yazı ve kendi yazı tipi olan ölçü değeri için `KENTOS_‹AİLE›[_B][_I]` kayıtları (`styleless`). TEXT
  stilini 7'de (Standard'da yazmaz, DXF'in varsayılanıdır), eğikliğini 51'de; MTEXT 7'de adlandırır. DIMSTYLE tablosuna Standard
  (bugünkü gibi) ve projenin her ölçü stili: boylar çizimin biriminde, DIMSCALE 1; çentikte DIMTSZ, okta DIMASZ (yok için 0),
  DIMTAD, DIMDEC, bayraklardan sonra DIMPOST, DIMLFAC (değerin birimi dosyanınkinden başkaysa), KENTOS verisinde stilin kendisi.
  DIMENSION 3'te stilin adını, DSTYLE'da ölçünün kendi görünüşünü (değer yüksekliği, çentik ya da ok boyu, boşluklar, değerin
  yeri, basamak, DIMPOST, DIMLFAC; stilsiz ölçününki bugünkü gibi) taşır; ölçünün bloğu dolu okları SOLID, noktaları DONUT (iki
  yarım yaylı, yarıçapı kalınlığında kapalı çoklu çizgi), açık okları iki çizgi olarak çizer ve değerini yazı tipinin kaydıyla
  yazar. Açık ok ve nokta DXF'in ok bloğu olmadan yazılır (DIMSTYLE'da dolu ok): başka program ölçüyü yeniden çizerse dolu ok
  gösterir, KentOS kendi görünüşünü KENTOS verisinden geri okur; rapor ve iki dışa aktarma penceresi bunu bir kez söyler.

### 8. `.kcad` şema 21

- Şema 21 yalnız projenin bir stili ya da bir yazının veya ölçünün bu alanlarından biri varken. Ayarlarda `textStyles`,
  `dimensionStyles` (kimlik, ad ve değerler; redler yeriyle); tipli sütunlarda yazının ve ölçünün yeni alanları (`FORMATS_VERSION`
  31). Bağımsız Python okuyucu ve yazıcı, `styles.kcad` ve bozuk örnekler.

### 9. Komutlar

- `cad.entities.create`'in yazı ve ölçü geometrisi yeni alanları alır; `cad.entities.edit`'in `properties`'i Öznitelikler'in stil
  satırlarını yazar. Redler: `unknown_style` (stilin kimliği projede yok), `invalid_style` (yazı tipi olmadan kalın, eğik ya da yatık;
  eğiklik aralık dışında; boy, basamak, önek ya da sonek kural dışında). Stillerin kendisi projenin ayarıdır, komut değildir.

## Kapsam dışı

Kılavuzun notu, blok öznitelikleri ve katman etiketleri projenin yazı tipiyle kalır; Etiketleri yazıya çevir stilsiz yazar; nesne
şablonlarında stil; Kullanılmayanları temizle'de stiller; açılar için ayrı basamak; DXF'in ok blokları (açık ok, nokta); paftanın
harita çerçevesinde yazının yatıklığı (yazı tipi, kalınlık ve eğiklik çizilir); yazının ve ölçünün görünüş alanlarının
Öznitelikler'de tek tek düzenlenmesi.

## Uygulama

Tek parçada (6 Ekim): sözleşme (`TextFace`, `DimensionLook`, `TextStyleDef`, `DimensionStyleDef`, kurallar ve uygulama/izleme
işlemleri `annotation.rs`; `ProjectSettings`'in iki tablosu; `cad.entities.edit`'in `textStyle` ve `dimensionStyle` işlemleri,
adımları “Yazı stili” ve “Ölçü stili”; `unknown_style`, `invalid_style`; katalog, TS tipleri ve Python SDK'sı yeniden üretildi);
`.kcad` şema 21 (kodek, sütunlar iki yanda, `FORMATS_VERSION` 31, bağımsız Python okuyucu ve yazıcı, `styles.kcad` ve 26 bozuk
örnek, belge §6.4.4); çekirdekte ölçünün uçları (dolu ok, açık ok, nokta, yok; `fills`), değerin yeri ve boşlukları, yazının
yüzü ve yatıklığı (kutu, seçme, kapsam), paket kayıtları (`store/pack.rs` ↔ `wasm/pack.ts`; ortak `fixtures/store-records/v1`'de
stilli yazı ve ölçü), Patlat'ta dolu uçlar; iki platformda çizim (yüzler, yatıklık, ölçünün yazı tipi; masaüstünde eğik yüzü
olmayan aile 0,25 yatar), araçlar (Yazı, Çok satırlı yazı, Ölçülendirme, Hızlı ölçü, Metin dosyası yerleştir; Zincir ve Baz
taban ölçüyü izler), Yazı stilleri ve Ölçü stilleri penceresi (masaüstü `annotation_styles.rs`, web
`ui/annotation/StylesDialog.ts`; kaydetme `style_tables`, `app/styleTables.ts`), Öznitelikler'in stil satırları, CAD şeridinin
Açıklama'sı ve Araçlar menüsü (CBS gizler), ikonlar (`textStyle`, `dimensionStyle`); DXF okuma ve yazma ve içe aktarmanın stil
birleştirmesi (`exchange/apply.rs`, `io/apply.ts`); değer yazımı (`format.rs`'in `dimension_in`'i, `model/dimensionValue.ts`; ortak
durumlar); ortak izler `text-styles.json`, `dimension-styles.json` (oynatıcılara `face` ve `look` beklentileri) ve
`paragraph-text.json`'da R; envanter.

Yan düzeltmeler: web penceresi ilk hâlinde Stil yöneticisinin `dialog--styles` sınıfını paylaşıyordu (ekran boyuna uzuyor, küçük
pencerede kayamıyordu); kendi sınıfı (`dialog--annotation`) verildi.

## Doğrulama

- `python3 scripts/fixtures/annotation_style_cases.py --check` (76 durum: kurallar, uygulama, izleme, değer yazımı); sözleşme
  `tests/all/annotation.rs`, web `model/annotationStyles.test.ts`, masaüstü `kentos_interaction` `dimension_values`.
- `python3 scripts/fixtures/kcad_v2_reference.py --check`; `cargo test -p kentos-kcad`; web `io/columns.test.ts`,
  `io/kcad.wasm.test.ts`.
- `python3 scripts/fixtures/dxf_write_reference.py --check` (`styles.dxf`: STYLE ve DIMSTYLE kayıtları, KENTOS verisi, yüzler,
  görünüşler, DSTYLE, SOLID ve DONUT; stilsiz örnekler bayt bayt aynı); `cargo test -p kentos-formats` (`styles.dxf` başka
  programın stilleri elle çıkarılmış değerlerle; kendi stillerinin gidiş-dönüşü); web `io/dxf.wasm.test.ts`.
- `python3 scripts/fixtures/create_command_cases.py --check`; iki platformun komut testleri; içe aktarmanın stil birleştirmesi
  (`exchange::apply`, `io/apply.test.ts`).
- Ortak izler `text-styles.json`, `dimension-styles.json`, `paragraph-text.json` iki platformda üç türde.
- Resimler: `node apps/web/scripts/e2e/shots.mjs styles`, `KENTOS_SHOTS_ONLY=stil-cizim,stil-yazi-penceresi,stil-olcu-penceresi,stil-yazi-araci,stil-oznitelikler,stil-menusu cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`
  (`.run/shots/arac-stil-*`).
