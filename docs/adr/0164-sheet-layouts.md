# ADR 0164: Pafta düzeni: model alanından kâğıda

- **Durum:** kabul edildi (2026-10-03). `CAD-07`, `CAD-08`, `OUT-01`, `OUT-02`, `OUT-03`. Dal
  `feat/sheet-layouts` (PR #17) `main`'e birleştirilirken `docs/sheet/design.md`'den buraya taşındı
  ([birleştirme](../sheet/integration.md) §4).
- **Tarih:** 2026-10-02
- **Bağlam belgeleri:**
  - TODOS.md: `CAD-07`, `CAD-08`, `OUT-01`, `OUT-02`, `OUT-03`, `OUT-05`.
  - Araştırma notu: `docs/research/2026-10-01-netcad-arcgis-qgis.md`.
  - İncelemeler: [PiriCAD](../sheet/piricad-review.md), [QGIS 4.2](../sheet/qgis-review.md).
  - ADR'ler:
    - 0008 (ortak çekirdek), 0010 (platform sınırları), 0013 (ürün komutu sözleşmesi);
    - 0015, 0024, 0035 (sahiplik, paylaşım, davet), 0040 (masaüstü bulut istemcisi);
    - 0055 (çizimin yazı tipleri), 0093 (lejant), 0110 (kuzey oku ve ölçek çubuğu).
- **Adlar:** kodda **sheet**, arayüzde **pafta**.
  - KentOS'ta “layout” zaten çalışma tezgâhının yerleşimidir (`app/layoutPlan.ts`,
    `apps/desktop/src/layout.rs`, `pnpm e2e:layout`); bu sistem o adı kullanmaz.
  - Arayüzde: “Pafta”, “Pafta tasarımcısı”, “Pafta şablonları”.

## Bağlam

KentOS model alanında çizer, ölçer, düzenler; ama işin teslim edilen yüzü olan paftayı üretemez.
Lejant (ADR 0093), kuzey oku ve ölçek çubuğu (ADR 0110) çizim alanında var. Kâğıt, ölçekli harita
çerçevesi, antet, karelaj, koordinat listesi, çok sayfa, atlas ve şablon yok (`CAD-07`).

İki örnek incelendi:

- **PiriCAD:** verisi ve denetimleri sağlam; çizimi Qt'ye gömülü, öğe modeli tek ve şişkin,
  yerleşimi sabit koordinatlı.
- **QGIS 4.2:** yetenekleri geniş; ama kâğıt değişince bozulan yerleşim, kesilen karelaj yazıları,
  standart dışı ölçekler, yapısız antet ve yalnız dosya olan şablonlar kullanıcıyı yalnız bırakıyor.

Bu karar ikisinin iyi yanlarını alıp şu ölçütlerde ikisini de geçen bir sistem kurar:

| # | Ölçüt | Nasıl ölçülür |
|---|---|---|
| H1 | **Bir şablon, her kâğıt:** kâğıt değişince öğeler kısıtlarıyla yeniden yerleşir | `fixtures/sheet/v1/relayout/*`: A3 şablonu A1, A4 ve özel boya alınır; antet sağ kenarda kalır, harita büyür |
| H2 | **Ana sayfa:** antet ve çerçeve bir kez tanımlanır | çok sayfalı fixture; ana sayfadaki değişiklik her sayfada |
| H3 | **Akıllı kılavuz:** kenar, merkez, eşit aralık, mesafe rozeti, cetvel kılavuzu, ızgara | `fixtures/sheet/v1/snap/*` |
| H4 | **Çoklu seçim denetçisi**; karışık değer “—” | web ve masaüstü ekran görüntüleri |
| H5 | **Türk ölçmecilik dili:** standart ölçek listesi, Y/X karelajı, yazı bandı çerçeveye dahil, “K” kuzey oku, hücreli antet, koordinat listesi | display fixture'ları, örnek paftaların görüntüleri |
| H6 | **İki platformda aynı sonuç:** bütün hesap tek Rust çekirdeğinde; platformlar yalnız boyar | aynı fixture'lar Rust'ta, WASM üzerinden web'de, masaüstünde |
| H7 | **Ön denetim:** dışa aktarmadan önce, neden ve çözümüyle | `fixtures/sheet/v1/preflight/*` |
| H8 | **Şablon kitaplığı:** sistem, bu cihaz, bulut, paylaşılan; kaynak ve eşitleme rozetleri | sunucu testleri, eşitleme fixture'ları, galeri görüntüleri |
| H9 | **Her düzenleme bir işlem:** geri alınır, tersi kesindir, betik ve ajan aynı yoldan gelir | `ops` testleri: her işlem ve tersi özdeş kitaba döner |

## Karar

### 1. Parçalar ve sınırlar

| Parça | Yer | Paket | Mimari grup (`scripts/arch/deps.mjs`) |
|---|---|---|---|
| Çekirdek | `crates/shared/sheet` | `kentos-sheet` | `shared`: makine + wasm32; yalnız `shared` crate'leri; çalışma zamanı, tarayıcı, masaüstü yok |
| WASM bağlayıcı | `crates/wasm/sheet-wasm` | `kentos-sheet-wasm` | `wasm` |
| Masaüstü tasarımcı | `crates/sheet-ui` | `kentos-sheet-ui` | **yeni grup** `sheet-ui`: `shared` + `ui`; Iced var, tarayıcı ve sunucu çatısı yok |
| Sunucu | `crates/server/application/src/sheet_templates.rs`, `crates/server/postgres/migrations/NNNN_sheet_templates.sql`, `apps/api/src/http/sheet_templates.rs` | mevcut paketler | `server`, `api` |
| Masaüstü bulut istemcisi | `crates/native/cloud/src/sheet_templates.rs` | `kentos-cloud` | `native` |
| Web | `apps/web/src/{product,render,tools,ui,app}/sheet/` | `@kentos/web` | web katmanları: `product → render → tools → ui → app` |
| Ortak örnekler | `fixtures/sheet/v1/` | — | iki platformun sınadığı veri |
| Belgeler | `docs/sheet/` | — | birleştirmede ADR'ye ve kullanıcı belgelerine taşınır |

Masaüstü tasarımcı, haritanın içini kendisi çizmez. Ev sahibi uygulama bir `MapPainter` verir
(§11), böylece `kentos-sheet-ui` çizim modeline ve wgpu hattına bağlanmaz. Uygulama
(`apps/desktop`) onu bağlantı adımında kendi çizimi ve `kentos-render-wgpu` ile besler.

**Çakışmasızlık ilkesi:** dalın neredeyse bütün içeriği **yeni dizinlerdedir**. Paylaşılan dosyalara
yalnız [bağlantı noktalarında](../sheet/integration.md) dokunulur. Her biri tek satır ya da küçük bir
blokdur, listelenir ve ayrı bir “bağlama” commit'inde durur.

### 2. Birimler

| Nicelik | Tip | Not |
|---|---|---|
| Kâğıt uzunluğu | `Um = i32` mikrometre | ±2 147 m; A0 ve şerit paftalar rahat sığar. Kâğıt koordinatı **sol üst köşeden, aşağı doğru** (masaüstü yayıncılığının ve ekranın yönü) |
| Açı | `Mdeg = i32` binde bir derece, `[0, 360 000)` | dönüş saat yönünde pozitif (ekranın yönü) |
| Harita ölçeği | `u32` payda | 1000 → 1/1000 |
| Yer koordinatı | `f64` metre, projenin koordinat sisteminde | Y sağa (doğu), X yukarı (kuzey): KentOS'un saha dili |
| Renk | `#rrggbb` ya da `#rrggbbaa` metni | stil çekirdeğindeki renk tipi varsa o kullanılır |
| Yazı boyu | `Um` | arayüz pt ya da mm gösterir (1 pt = 352,778 µm) |

**Arayüzde kâğıt konumu “Sol / Üst / Genişlik / Yükseklik” diye adlanır, “X / Y” değil:** KentOS'ta
Y ve X yer koordinatlarının adıdır (Y doğu, X kuzey). Kâğıtta bu adlar karışıklık yaratır.

### 3. Belge modeli (`kentos.sheet/1`)

Model serde ile JSON olur ve ts-rs ile `apps/web/src/contracts/generated/sheet/` altına TypeScript tipi
olarak üretilir (elle düzenlenmez). Aşağıdaki tanımlar sözleşmenin çekirdeğidir. Alan eklemek
serbesttir; var olan alanın anlamı değişmez.

```rust
pub struct SheetBook {            // bir projenin bütün paftaları
    pub schema: String,           // "kentos.sheet/1"
    pub sheets: Vec<Sheet>,
    pub masters: Vec<Master>,     // ana sayfalar
    pub assets: Vec<AssetMeta>,   // baytlar ayrı: AssetStore (§3.4)
    pub variables: Vec<Variable>, // proje düzeyi değişkenler (@proje_no, @idare…)
}
pub struct Sheet {
    pub id: SheetId, pub name: String,
    pub page: Page,
    pub master: Option<MasterId>,
    pub items: Vec<Item>,         // çizim sırası: ilk eleman en altta
    pub guides: Vec<Guide>,
    pub snap_grid: SnapGrid,
    pub atlas: Option<Atlas>,
    pub export: ExportDefaults,   // dpi, biçim, dosya adı ifadesi
    pub origin: Option<TemplateOrigin>, // hangi şablonun hangi revizyonundan
}
pub struct Page { pub paper: Paper, pub orientation: Orientation, pub size: SizeUm, pub margins: Margins, pub background: Option<String> }
pub enum Paper { Iso(IsoSize /* A0..A5, B0..B4 */), Custom }  // boyutlar tabloda, veri
pub struct Master { pub id: MasterId, pub name: String, pub items: Vec<Item>, pub guides: Vec<Guide> }

pub struct Item {
    pub id: ItemId,               // kalıcı anahtar; bağlar bununla tutulur
    pub name: String,             // pafta içinde tekil; komut ve betik bununla anar
    pub frame: RectUm,            // döndürülmemiş çerçeve
    pub rotation: Mdeg,           // çerçevenin merkezinde
    pub constraints: Constraints,
    pub locked: bool, pub hidden: bool, pub printable: bool,
    pub opacity: u8,              // 0..=100
    pub border: Option<Stroke>, pub fill: Option<String>, pub padding: Um,
    pub group: Option<ItemId>,    // bir Group öğesinin çocuğu
    pub bindings: Vec<Binding>,   // veriye bağlı özellikler (§7)
    pub kind: ItemKind,
}
pub enum ItemKind {
    Map(MapItem), Text(TextItem), ScaleBar(ScaleBarItem), NorthArrow(NorthArrowItem),
    Legend(LegendItem), Picture(PictureItem), Shape(ShapeItem), Line(LineItem),
    Table(TableItem), CoordinateList(CoordinateListItem), TitleBlock(TitleBlockItem),
    Border(BorderItem), Group,
}
pub struct Constraints { pub h: HConstraint, pub v: VConstraint, pub relative_to: ConstraintBox }
pub enum HConstraint { Left, Right, LeftRight, Center, Scale }
pub enum VConstraint { Top, Bottom, TopBottom, Center, Scale }
pub enum ConstraintBox { Page, Margins, Group }
pub struct Binding { pub property: String /* "frame.left", "text.content", "map.scale", "hidden"… */, pub expression: String }
pub struct Guide { pub id: String, pub axis: Axis, pub at: Um, pub locked: bool }
pub struct SnapGrid { pub spacing: Um, pub visible: bool, pub enabled: bool }
```

#### 3.1 Öğe türleri

| Tür | İçerik | Kural |
|---|---|---|
| **Harita** `Map` | `view` | `Fixed { center, scale, rotation }` ya da `Atlas { policy }` |
| | `layers` | `All`, `Theme(ad)` ya da katman listesi |
| | diğer | `crs`; `grids: Vec<MapGrid>`; `overview_of: Option<ItemId>`; `clip_to_atlas_feature`; `label_band: Um` |
| **Metin** `Text` | `content` | `[% ifade %]` parçalı metin |
| | biçim | yazı tipi, boy, renk, yatay ve düşey hizalama, satır aralığı, `wrap`, `fit: None \| ShrinkToFit` |
| **Ölçek çubuğu** `ScaleBar` | biçim ve bağ | `map` bağı; biçim: `SingleBox \| DoubleBox \| Ticks \| Stepped \| Hollow \| Numeric` |
| | parçalar | sol/sağ parça, alt bölüm, uzunluk `Auto \| Fixed(m)`, birim |
| | ek | “1/1000” sayısal ölçek; KentOS'un çizim alanındaki çubuğuyla aynı görünüş (ADR 0110) |
| **Kuzey oku** `NorthArrow` | bağ ve kuzey | `map` bağı; `north: Grid \| True \| Magnetic` |
| | biçim | `KentosK` (ADR 0110: yarısı dolu ok, üstünde “K”), `Simple`, `Compass` |
| | ek | `declination` (manyetik, elle); yakınsama notu |
| **Lejant** `Legend` | bağ ve süzgeç | `map` bağı; süzgeç: `All \| VisibleInMap \| AtlasFeature` |
| | düzen | başlık, sütun sayısı, kendiliğinden kaydırma genişliği, simge boyu, aralıklar |
| | girdiler | `Auto \| Custom(Vec<LegendEntryOverride>)`; içerik ev sahibinin lejant motorundan (ADR 0093) |
| **Resim** `Picture` | `asset` | gömülü varlık; `fit: Contain \| Cover \| Stretch \| Original`; `clip` |
| **Şekil** `Shape` | `shape` | `Rect { radius } \| Ellipse \| Triangle \| Polygon(Vec<[Um; 2]>)` |
| **Çizgi** `Line` | çizgi | çerçeveye göre noktalar, çizgi biçimi |
| | uçlar | `None \| Arrow \| Dot \| Bar` |
| **Tablo** `Table` | kaynak | `Fixed(satırlar) \| Layer { layer, filter, sort, only_in_map, atlas_filter }` |
| | biçim | sütunlar (başlık, değer ifadesi, genişlik `Auto \| Fixed`, hizalama, biçim); başlık ve hücre stili |
| | kenar durumları | boş tablo davranışı; taşma: `Clip`, sonra `ContinueIn(Vec<ItemId>)` |
| **Koordinat listesi** `CoordinateList` | kaynak | seçili nesneler ya da katman |
| | içerik | nokta adlandırma (sıra no ya da alan); sütunlar No, Y, X, (Z); ondalık; kapanış satırı; alan satırı |
| | veri | satırları ev sahibi verir |
| **Antet** `TitleBlock` | yapı | satırlar → hücreler (genişlik ağırlığı, etiket, değer ifadesi, yazı, hizalama, imza hücresi) |
| | davranış | tek öğe olarak taşınır ve yeniden boyutlanır |
| **Pafta çerçevesi** `Border` | çizgi | iç pay; tek ya da çift çizgi |
| | CAD'de | isteğe bağlı bölge işaretleri (sütunlar 1, 2, 3…; satırlar A, B, C…) ve ortalama işaretleri |
| | GIS'te | ince sınır çizgisi |
| **Grup** `Group` | çocuklar | çocuklar `group` alanıyla bağlanır |
| | çerçeve | çocukların birleşimi; normalleştirmede yeniden hesaplanır |

`MapGrid` (karelaj):

- `kind`: `Cross | Lines | Ticks | FrameOnly`.
- `interval`: metre `[Y, X]`; 0 ise ölçekten seçilir.
- `offset`, `stroke`, `cross_size`.
- `frame`: `None | Zebra | Ticks | Line`; `frame_width`.
- `labels`: kenarlar, iç ya da dış, yön (`Horizontal | AlongEdge`), biçim, yazı tipi.
  - Biçim: `Metres { decimals, group_thousands }`, `Dms`, `Expression`.
  - Varsayılan: tam metre; Y üst ve alt kenarda, X sol ve sağ kenarda kenar boyunca.
- `crs`: başka bir sistemde ızgara, örneğin coğrafi.

Dış yazılar **`label_band`'in içinde** kalır. Çerçeve bu bandı da kapsar; yazı komşu öğeye taşamaz
(QGIS Q2). Bant yetmezse ön denetim söyler.

#### 3.2 Kısıtlar ve yeniden yerleşim

Kâğıt boyu, yönü ya da kenar boşluğu değişince her öğe kısıtıyla yeni yerine taşınır. Kutu sayfa,
kenar boşluğu ya da grup çerçevesidir.

- `Left` / `Top`: sol ya da üst uzaklık korunur.
- `Right` / `Bottom`: sağ ya da alt uzaklık korunur.
- `LeftRight` / `TopBottom`: iki uzaklık da korunur; öğe esner.
- `Center`: merkezin kutunun merkezine uzaklığı korunur.
- `Scale`: konum ve boy oranla.

Sonuç tam mikrometreye yuvarlanır. Yuvarlama kuralı tektir: yarım değerler sıfırdan uzağa. En küçük
öğe boyu 1 mm'dir. Esneyen harita ölçeğini korur ve **kapsamını büyütür**; ölçeği değiştirmez.

#### 3.2a Yerleşim düzenleri (kâğıt yönüne ve boyuna göre)

Kısıtlar boy değişimini karşılar ama yön değişimini karşılamaz. A3 yatay bir paftanın sağdaki antet
şeridi A4 dikeyde haritayı dar bir şeride sıkıştırır; ISO 7200 anteti dikey A4'te tam genişliğe
yerleşmelidir. Bu yüzden pafta (ve ana sayfa) birden çok **yerleşim düzeni** taşıyabilir:

```rust
pub struct LayoutVariant {
    pub id: String, pub name: String,          // "dikey", "Dikey kâğıt"
    pub when: VariantCondition,                // yön; isteğe bağlı en az/en çok genişlik ve yükseklik
    pub reference: SizeUm,                     // düzenin tasarlandığı kâğıt
    pub frames: Vec<VariantFrame>,             // öğe → çerçeve, dönüş, kısıtlar, gizli mi
}
```

- **Kurallar:**
  - `Item.frame` her zaman o anki kâğıttaki çerçevedir; çizim yalnız onu okur.
  - Kâğıt değişince (`SetPage`, `instantiate`) koşulu uyan **ilk** düzen seçilir. Hiçbiri uymazsa
    temel düzen geçerlidir. Öğelerin çerçeveleri düzenin `reference` kâğıdından yeni kâğıda
    kısıtlarla taşınır.
  - Düzenin anmadığı öğe temel düzenden gelir.
- **Düzenleme:** etkin bir düzen varken taşıma ve boyutlandırma, o düzenin kaydına da yazılır. Kayıt
  kısıtların tersiyle düzenin `reference` kâğıdına çevrilerek tutulur. Böylece dikeyde yapılan
  düzeltme yatayı bozmaz.
- **Arayüz** (Sayfa denetçisi):
  - “Yerleşim düzeni: Dikey (kendiliğinden)”;
  - “Bu kâğıt için ayrı düzen oluştur”;
  - düzenin koşulu, adı; düzeni sil.
- **Sistem şablonları** en az bir yatay ve bir dikey düzen taşır. A4–A0'ın her boyunda ve her iki
  yönde ön denetimden çakışma, taşma ve sayfa dışı bulgusu olmadan geçmeleri testle kanıtlanır.

#### 3.3 Ana sayfa

Ana sayfanın öğeleri, ona bağlı her paftanın öğelerinin **altında** çizilir; kilitlidir ve seçilmez.
Anteti, çerçeveyi, logoyu ve kuzey okunu bir kez tanımlamaya yarar.

- Ana sayfadaki metin ve antet değerleri değişkenlerle her paftada ayrı dolar (`@pafta_adi`,
  `@sayfa`, atlas alanları).
- “Ana sayfadan ayır” işlemi öğeleri paftaya kopyalar.

#### 3.4 Varlıklar

Resimler (PNG, JPEG, SVG) içerik özetiyle (SHA-256) anılır. `SheetBook.assets` yalnız üst veriyi
taşır (özet, tür, ad, piksel boyu, bayt sayısı); baytlar `AssetStore`'dadır. Bu sayede işlemler ve
geri alma büyük baytları taşımaz. Şablon ve dışa aktarılan pafta dosyası baytları birlikte taşır;
şablon başka makinede boş kutu basmaz (PiriCAD Z11).

Sınırlar: varlık başına 4 MB, şablon başına toplam 8 MB.

### 4. İşlemler ve geri alma

Her düzenleme saf bir işlemdir:

```rust
apply(book, op) -> Result<Applied { book, inverse: Vec<Op> }, OpError>
```

- `inverse` kitabı **özdeş** hâline döndürür; özdeşlik testle kanıtlanır.
- Kimlikleri çekirdek üretmez, işlemle gelir. Ev sahibi üretir; fixture'lar sabit kimlik kullanır,
  bu yüzden belirleyicidir.

İşlemler:

| Konu | İşlemler |
|---|---|
| Kitap | `AddSheet`, `RemoveSheet`, `RenameSheet`, `MoveSheet`, `DuplicateSheet` |
| Sayfa | `SetPage` (kâğıt, yön, boy, kenar; kısıtlarla yeniden yerleşim) |
| Ana sayfa | `AddMaster`, `SetSheetMaster`, `DetachMaster` |
| Öğe | `AddItems`, `RemoveItems`, `MoveItems { ids, delta }`, `ResizeItem { id, handle, to, keep_aspect, from_center }`, `RotateItems`, `SetItemProps { id, patch }`, `RenameItem` |
| Sıra ve grup | `Reorder { ids, to: Front \| Back \| Forward \| Backward }`, `Group`, `Ungroup`, `Lock`, `Hide` |
| Düzen | `Align { ids, edge, to: Selection \| Page \| Margins \| KeyItem }`, `Distribute { ids, axis, mode: Centers \| Gaps }`, `MatchSize` |
| Kılavuz | `AddGuide`, `MoveGuide`, `RemoveGuide` |
| Şablon | `ApplyTemplate`, `SaveVariables` |

`SetItemProps` kısmi bir JSON yamasıdır. Çekirdek onu türün şemasına göre doğrular; bilinmeyen alan
ya da yanlış tip hatadır, sessizce atılmaz.

Web'de işlemler WASM üzerinden çalışır. Masaüstü aynı fonksiyonu doğrudan çağırır. Geri alma yığını
pafta kipine özgüdür (§10).

### 5. Akıllı kılavuzlar ve yapışma

Sürükleme oturumu başında adaylar bir kez hesaplanır:

```rust
SnapSession::new(sheet, moving_ids, enabled)
session.query(delta, tolerance_um) -> SnapResult
```

- **X adayları:** sayfa sol/orta/sağ; kenar boşluğu; düşey kılavuzlar; ızgara; öbür öğelerin
  sol/orta/sağ kenarları. Öbür öğeler görünür, kilitli ya da değil; ana sayfa öğeleri dahil;
  taşınanlar ve grupları hariç. Y için aynısı.
- **Seçim:** her eksende ayrı, uzaklığı tolerans içindeki en yakın aday. Eşitlikte öncelik:
  kılavuz > öğe > sayfa ve kenar boşluğu > ızgara; sonra aday sırası.
- **Eşit aralık:** taşınan kutuyla aynı satırdaki (ya da sütundaki) komşuların aralıkları
  ölçülür. Taşınan kutunun aralığı bunlardan birine tolerans içinde yaklaşırsa ona eşitlenir;
  aralık işaretleri döner.
- **Mesafe rozetleri:** her yönde en yakın komşuya uzaklık, mm cinsinden, bir ondalıkla.
- **Boyutlandırma:** tutamacın kenarları aynı kurallarla; ek olarak “aynı genişlik/yükseklik”.
- **Dönüş:** Shift ile 15° adım; 0/90/180/270'e 2° içinde yapışma.
- **Klavye:** ok tuşu 1 mm, Shift 10 mm, Alt 0,1 mm.
- Tolerans ekran pikselidir (varsayılan 6 px); çağıran yakınlaştırmaya göre µm'ye çevirir.

`SnapResult { delta, lines: Vec<SnapLine>, gaps: Vec<GapBadge>, spacing: Vec<SpacingMark> }`.
Çizgiler ve rozetler çekirdekte hesaplanır; platform yalnız boyar. Aynı sorgu iki platformda aynı
sonucu verir.

### 6. Metin ve yazı tipleri

Paftanın yazıları çizimin yazı tipleriyle yazılır (ADR 0055, `DRAWING_FONTS`). Lejant, tablo ve antet
yerleşimi için **genişlik çekirdekte hesaplanır**:

- `scripts/fonts/sheet_metrics.py`, aynı yazı tipi dosyalarından bir ölçü tablosu üretir:
  `crates/shared/sheet/data/font-metrics.json`. İçinde em birimi, yükseliş/iniş, satır aralığı ve
  Türkçe ile Latin karakterlerin ilerleme genişlikleri vardır.
  - Üretim fontTools ile yapılır; yeni Rust bağımlılığı gerekmez.
  - `--check` kipi, tablonun yazı tiplerine göre güncel olduğunu denetler.
- Satır kırma, hizalama, `ShrinkToFit`, tablo sütun genişliği ve lejant sütunları bu tabloyla
  hesaplanır. Çekirdek satırları konumlarıyla verir; platform her satırı aynı yazı tipiyle o noktaya
  yazar. Satır kırılması iki platformda aynıdır.
- Kerning ve karmaşık yazı düzenleri 1. aşamada yoktur. Türkçe ve Latin yazı için fark göze
  görünmez. Sınır belgede ve ön denetimde söylenir.
- **Eksik karakter** (2026-10-03): hiçbir çıktı yazı tipinde olmayan bir karakteri kutu olarak
  basmaz. Ölçü tablosu aynı zamanda yazı tiplerinin kapsamıdır; tablonun ölçmediği karakter hiçbir
  yazı tipinde yok sayılır.
  - Yazının yüzünde olmayan karakter, tablonun sırasıyla (Barlow, Arimo, Overpass, Quicksand,
    Architects Daughter, Courier Prime, IBM Plex Mono) onu içeren ilk çizim yazı tipinin aynı
    kalınlık ve eğiklikteki yüzüyle yazılır; genişliği o yüzün ilerlemesidir.
  - Hiçbir yüzde olmayan karakter “?” yazılır ve öyle ölçülür; denetim karakteri boşluk olur.
    Çizim planı yazıyı bu hâliyle verir, ekran da “?” gösterir.
  - Her iki durum da ön denetimde `glyph_missing` bulgusudur: karakter, öğe ve yüz adıyla. Başka
    yüzle yazılan bilgi, “?” olan uyarıdır.
  - PDF bu parçaları kendi yüzleriyle yazar ve o yüzleri de gömer; SVG başka yüzün parçasını
    `<tspan font-family>` ile adlandırır. Kural `text::glyph` ve `text::runs`'tadır; WASM'da
    `textRuns`.

### 7. Veriye bağlı özellikler ve değişkenler

Her `Binding`, `kentos-expression` ile derlenir ve değerlendirilir; dil yenisi yazılmaz. Kapsam
sırası:

- atlas nesnesinin alanları;
- pafta değişkenleri;
- proje değişkenleri;
- hazır değişkenler.

Hazır değişkenler:

| Değişken | Anlamı |
|---|---|
| `@proje_adi` | projenin adı |
| `@pafta_adi` | paftanın adı |
| `@sayfa`, `@sayfa_sayisi` | sayfa numarası ve sayfa sayısı |
| `@olcek` | bağlı haritanın ölçeği, “1/1000” |
| `@tarih` | bugünün tarihi, ISO; biçim ifadeyle |
| `@kullanici` | oturumdaki kişinin adı |
| `@koordinat_sistemi` | koordinat sisteminin adı |
| `@atlas_*` | atlas nesnesine ait değerler |

- Metin içinde `[% ifade %]`. Değerlendirilemeyen ifade kâğıda `‹ad?›` olarak yazılır, ön denetimde
  hata olur (PiriCAD'in “sessiz boş metin yok” kuralı).
  - İşaretin köşeli tırnakları (U+2039, U+203A) çizimin her yazı tipinde vardır (ölçü tablosu);
    önceki `⟨ ⟩` (U+27E8, U+27E9) hiçbirinde yoktu ve PDF'te kutu olarak basılıyordu (2026-10-03).
- Bağlanabilen özellikler:
  - konum, boy, dönüş, gizlilik, saydamlık;
  - metnin içeriği;
  - haritanın ölçeği, merkezi ve dönüşü; ızgara aralığı;
  - lejant başlığı;
  - tablo süzgeci.

  Liste çekirdekte tektir ve arayüzde “ƒ” düğmesi olarak görünür.

### 7a. Atlas

Atlas, kapsam katmanındaki her nesne için bir sayfa üretir. Çekirdek ev sahibinin verdiği nesne
listesinden belirleyici bir **atlas planı** kurar (kimlik, kutu, alanlar):

- süzgeç ve sıralama;
- sayfa adı ifadesi;
- ölçek politikası: `Fit { margin_pct, standard_scales }`, `Fixed`, `Predefined`;
- aynı adlara ek;
- her sayfanın harita görünüşü ve değişkenleri.

Ölçek politikası `standard_scales` ise sığan **en büyük standart ölçek** seçilir. Standart ölçekler
çekirdekte veridir: 1/100, 200, 250, 500, 1000, 2000, 2500, 5000, 10 000, 25 000, 50 000, 100 000.

1. aşamada plan ve önizleme vardır; toplu dışa aktarma 2. aşamadadır.

### 8. Çizim planı (display list)

Çekirdek bir sayfayı ilkel listesine çevirir:

```rust
display_list(book, sheet_id, inputs: &RenderInputs) -> DisplayList
```

İlkeller:

- `Rect`, `Path` (doğru ve yay parçaları; çizgi biçimi, kesik, uç, birleşim);
- `Text` (konumlu satırlar, yazı tipi, boy, renk, dönüş);
- `Image { asset, rect, rotation, opacity }`;
- `Map { item, clip, rotation, view, layers, crs }`;
- `PushClip` / `PopClip`, `PushGroup { opacity }` / `PopGroup`.

Her ilkel kaynağının `ItemId`'sini taşır; seçim ve vurgulama bununla yapılır.

- **Çekirdeğin hesapladıkları:**
  - karelaj çizgileri ve yazıları;
  - ölçek çubuğunun parçaları ve sözleri (ADR 0110'un 1-2-5 kuralı);
  - kuzey okunun şekli ve dönüşü;
  - lejant, tablo, koordinat listesi ve antet yerleşimi;
  - metin satırları;
  - ana sayfanın öğeleri;
  - bulunamayan değerin `‹ad?›` işareti.
- **Ev sahibinin verdikleri (`RenderInputs`):**
  - değişken değerleri;
  - her lejantın girdileri (simgeler küçük çizim listeleri ya da varlıklar olarak);
  - tablo satırları, koordinat satırları;
  - atlas nesnesi;
  - kullanılabilir yazı tipleri;
  - harita başına meridyen yakınsaması: çekirdek, `kentos-geometry-core`'un izdüşüm hesabıyla
    kendisi bulabiliyorsa o kullanılır, ev sahibinin girdisi yalnız yedektir.
- **`Map` ilkelinin içini ev sahibi boyar:** web kendi çizim hattıyla, masaüstü
  `kentos-render-wgpu` ile, aynı görünüş dönüşümüyle. Çerçeve, karelaj, ölçek ve kuzey çekirdekte
  hesaplandığı için iki platformda aynı yerdedir.

Liste belirleyicidir: aynı kitap ve aynı girdiler bayt bayt aynı JSON'u verir. Fixture'lar bunu iki
platformda sınar.

### 9. Dışa aktarma ve ön denetim

**1. aşama:**

- **SVG:** çekirdeğin yazıcısı. Sayılar sabit biçimlidir. Harita içi, ev sahibinin verdiği SVG
  parçası ya da PNG'dir.
- **PNG:** dpi seçilir; web tuvalde, masaüstü kendi çiziciyle üretir.
- **Pafta dosyası:** `.kpafta`: kitap ve varlıklar, JSON.

**2. aşama (bağımlılık kararı ister):**

- PDF ve GeoPDF: `pdf-writer` ve `subsetter`. Saf Rust, wasm32'de de çalışır, bu yüzden iki
  platformda aynı PDF.
- Toplu atlas çıktısı.

**Ön denetim** (`preflight(book, sheet_id, inputs) -> Vec<Finding>`): her bulgu önem derecesi, öğe,
kod, Türkçe ileti ve çözüm taşır. Bakılanlar:

- sayfa ya da basılabilir alan dışındaki öğe;
- başka öğenin tamamen örttüğü öğe;
- bağı kopuk lejant, ölçek çubuğu ya da kuzey oku;
- ölçeği standart olmayan harita (uyarı);
- yazı bandına sığmayan karelaj;
- değerlendirilemeyen ifade (`‹ad?›`);
- yazı tipinde olmayan karakter (`glyph_missing`, §6);
- eksik varlık; eksik yazı tipi;
- seçilen dpi'da 150 ppi altına düşen resim;
- çerçeveye sığmayan metin; taşan tablo;
- kapsam katmanı olmayan atlas.

Dışa aktarma ön denetimi gösterir; **hata** varken kullanıcı açıkça “yine de aktar” demelidir.

### 8a. Manyetik kuzey (WMM)

Sahibin onayıyla 2026-10-03'te kapsama alındı.

- **Veri:** NOAA/NCEI'nin WMM2025 katsayıları `crates/shared/sheet/data/wmm2025.json` dosyasında
  durur. Dosya resmî `.COF` dosyasından bir betikle üretilir.
  - Kaynak adresi, yayın tarihi, geçerlilik (2025.0–2030.0) ve lisans (ABD kamu malı) dosyada ve
    `docs/deps`'te yazılıdır.
  - Yeni dış paket yoktur; hesap `libm` ile yapılır.
- **Hesap çekirdektedir:** jeodezik konum geosentriğe çevrilir; 12. dereceye kadar küresel
  harmonikler; yıllık değişim tarihe uygulanır. Sonuç manyetik sapma (D) ve eğimdir (I).
  - NOAA'nın resmî WMM2025 sınama değerleriyle 0,01° içinde sınanır.
- **Kullanım:**
  - `north: Magnetic` olan kuzey oku sapmayı haritanın merkezinden ve paftanın tarihinden
    (`@tarih` ya da paftada seçilen tarih) kendisi hesaplar. Elle girilen değer hesabın yerine
    geçer ve “elle” diye belirtilir.
  - Yeni biçim **`Diagram`**, Türk topoğrafik paftalarındaki kuzey çizelgesidir: grid kuzeyi,
    coğrafi kuzey ve manyetik kuzey, aralarındaki açılarla birlikte. Yazısı örneğin
    “Manyetik sapma 5°41' D (WMM2025, 2026-10)”.
- **Ön denetim:** WMM2025'in geçerlilik dışındaki tarih uyarıdır. Koordinat sistemi bilinmiyorsa
  hata verilir (konum yoktur).
- **Uygulama (3 Ekim):**
  - Hesap `wmm.rs`'tedir; NOAA'nın 112 sınama noktasıyla sınanır (açılar 0,005° içinde).
  - Elle değer `declinationHand` alanıyla seçilir. Alanı olmayan eski kitapta sıfırdan farklı bir
    sapma elle girilmiş sayılır.
  - Kuzey çizelgesinde GK, CK (ucunda yıldız) ve MK (ucunda yarım ok) çizilir, aralarında yaylar
    vardır. Çizelgenin altında üç açı yazılır: yakınsama, kaynağıyla manyetik sapma ve grid–manyetik
    açısı. 12°'den dar açılar genişletilir ve “Açılar ölçekli değildir.” yazılır.
  - Ön denetimin kodları:
    - `magnetic_out_of_model`: uyarı.
    - `magnetic_no_place`: hata; düzeltmeleri “Koordinat sistemi seç” ve “Sapmayı elle gir”.
      Projenin hiç sistemi yoksa yalnız `needs_crs` gelir.
    - `magnetic_no_date`: hata; `@tarih` okunamıyor.
- **Sığdırma ve bilgi (3 Ekim, koordinatörün notu):**
  - Açı işaretleri ASCII'dir: dakika `'`, saniye `"` (“6°19' D”, “−0°05'47"”). Barlow'un ′
    (U+2032) glifi 0,60 em genişliğindedir, 0,46 em'i boştur; ′'den sonra hep çift boşluk
    görünüyordu. `'` ve `"` her çizim yüzünde dar bir çentiktir, klavyeden yazılır, PDF'te aranır.
  - Çizelgenin açıları ve okun notu tek blok yazılır. Satır önce kaynağından önce kırılır
    (“… 6°19' D” / “(WMM2025, 2026-10)”), sonra boşluklardan. Yetmezse yazı küçülür, ama
    1,5 mm'nin altına inmez (`text::LEGIBLE_MIN`).
  - Geniş çerçevede (en > boy × 1,2) açılar çizelgenin yanına, öbür türlü altına yazılır; biri
    sığmazsa öbürü denenir. Hiçbiri sığmazsa `text_overflow` verilir.
  - Çekirdeğin kendi yazdığı her yazı sığmadığında `text_overflow` verir: çizelge, okun notu, ölçek
    çubuğunun yazıları, lejantın başlığı ve etiketleri. Önceden sessizce taşıyordu.
  - `magnetic_out_of_model`'in düzeltmeleri “Değişkenleri aç” (tarih) ve “Sapmayı elle gir”dir.
  - `northInfo(book, sheet, item, inputs)` (WASM; Rust `display::north_info`) okun gösterdiğini
    verir: haritanın merkezinin enlem-boylamı, yakınsama, sapma ve kaynağı (model ya da elle),
    model, kullanılan tarih ve nereden geldiği (paftanın “tarih”i, projeninki, bugün), tarihin
    modelin yılları içinde olup olmadığı.
  - İki platformun denetçisi bunu aynı sözcüklerle gösterir (9. adım). Sıra şöyledir:
    - “Manyetik sapma 6°19' D · WMM2025 · 2026-10” ve nereden hesaplandığı;
    - “Hesap tarihi”, kaynağıyla (“bugün: paftada ve projede “tarih” değişkeni yok”) ve
      “Değişkenler…” düğmesiyle;
    - “Sapmayı elle gir”;
    - elle girilirken “Elle sapma” ile “Sapmanın yılı”.
  - Anahtar açılınca değer modelinkiyle başlar: dakikaya yuvarlanır ve yalnız hiçbir değer
    yazılmamışsa yazılır. `magnetic_out_of_model`'in düzeltmesi de böyle yapar.
  - Dört sistem şablonunda (aplikasyon krokisi, genel A4, GIS atlası, rapor sayfası) okun çerçevesi
    çizelgeye yer olsun diye sola genişledi (26–28 × 16–19 mm). On şablon, A4…A0'ın iki yönünde
    çizelge+manyetik ve not+manyetik ile bulgusuzdur.

### 9a. PDF ve GeoPDF

Sahibin onayıyla 2026-10-03'te kapsama alındı; “2. aşama” notunun yerini alır.

- **Yazıcı çekirdektedir:** `kentos-sheet::pdf`.
  - Yeni paketler `pdf-writer` ve `subsetter`; ikisinin de lisansı MIT/Apache-2.0. Sürüm ve lisans
    `docs/deps`'e yazılır.
  - Yazı tipi okuma ve sıkıştırma için kilitte zaten bulunan paketler kullanılır: `ttf-parser` ya da
    `skrifa`, `miniz_oxide` ya da `flate2`.
  - Arayüz: `to_pdf(book, &PdfInputs, &PdfOptions) -> Vec<u8>`; WASM'da `toPdf`.
  - Web ve masaüstü aynı yazıcıyı kullanır.
- **Sayfa:** her pafta bir sayfadır. Kâğıt boyu tam karşılanır (1 mm = 72/25,4 pt); ölçek 1/1000
  ise kâğıtta da 1/1000'dir. Seçilen paftalar tek dosyada çok sayfa olur.
- **Vektör:** çekirdeğin çizdiği her şey vektördür: çerçeveler, karelaj, antet, tablolar, lejant,
  ölçek çubuğu, kuzey oku, şekiller, çizgiler.
- **Yazı:**
  - Yazı tipi alt küme olarak gömülür.
  - Ev sahibi çizimin aynı TTF dosyalarını verir; masaüstü kendi dosyalarını, web dışa aktarırken
    aynı dosyaları yükler.
  - `ToUnicode` sayesinde yazı seçilebilir ve aranabilir; Türkçe harfler doğru çıkar.
  - Konumlar çekirdeğin ölçü tablosundan gelir, satırlar ekrandakiyle aynı yerde kırılır.
- **Resim:** JPEG olduğu gibi gömülür, PNG Flate ile sıkıştırılır. Saydamlık yumuşak maskeyle
  (SMask) korunur.
  - SVG resmi çekirdek çizmez (yeni paket yok). Ev sahibi PNG'sini verir (`PdfAsset.raster`):
    `pdfSvgSizes` her SVG resmin dışa aktarma çözünürlüğündeki boyunu söyler (en büyük
    çerçevesininki). Masaüstü resvg ile, web tarayıcıda çizer. Çekirdek PNG'yi resmin yerine
    PNG gibi gömer.
  - `pdfFindings` (Rust `pdf::findings`) bunu söyler: `svg_as_picture` (bilgi, “SVG resim PDF'e
    resim olarak gömülür”), PNG'si verilmemişse `svg_not_in_pdf` (uyarı; boş resim kutusu basılır,
    sessizce değil).
- **Harita içi**, harita başına iki yoldan biriyle:
  - `MapContent::Vector`:
    - Ev sahibi çizimin yollarını (yer koordinatında, Y/X metre; çizgi kalınlığı kâğıt mm) ve
      yazılarını verir.
    - Çekirdek bunları haritanın görünüşüyle kâğıda çevirir ve çerçeveye kırpar.
    - Her çizim katmanı bir isteğe bağlı içerik grubu (OCG) olur, yani katmanlı PDF.
  - `MapContent::Raster`: yedek yoldur; çerçeve, seçilen dpi'da bir PNG'dir.
  - İki platformun haritası aynıdır (3 Ekim, koordinatörün kararı):
    - Yollar çizimin stilli katmanlarından gelir: stil motoru haritanın ölçeğinde, kâğıdın
      paletiyle kurar (web `buildStyledLayer`, masaüstü `build_layer`). Çizgi kalınlığı ve deseni,
      nokta simgesi ve taraması stildeki gibidir; ölçüler kâğıt mm'sidir.
    - Kâğıdın renkleri çekirdektedir (`display::paper`, WASM `paperPalette`): kâğıt beyaz, mürekkep
      siyah, temanın mürekkebi (`fg`) ve çizimin yazıları `#111111`, hale beyaz. Stilin kendi rengi
      olduğu gibi kalır.
    - Çizimin yazıları kâğıdın CSS pikselinde (25,4/96 mm) yerleşir; çapası ekrandakiyle aynıdır
      (ortalı etiketin ortası, metnin taban çizgisi başı).
    - Çapası çerçevenin içinde olan yazı yazılır ve kenarda kesilir; dışında olan hiç yazılmaz. Kural
      çekirdektedir; ekrandaki harita da aynı kuralı izler.
    - Vektör biçimi olmayan bir şey çizen katman (desen ya da resim dolgusu, resimli ya da yazılı
      simge, yumuşak kenarlı çizgi) haritayı resme çevirir; dışa aktarma penceresi hangi katmanın
      neden olduğunu yazar.
    - Bu resim de stillidir (9. adım). Web onu stilli gölgelendiricilerle çizer. Masaüstü aynı WGSL
      gölgelendiricilerinin CPU ikiziyle çizer (render-wgpu `styled::cpu`): aynı stil blokları, aynı
      piksel fonksiyonları, aynı görünürlük kuralları ve atlas resimleri. Ekran da böyle bir haritayı
      aynı ikizle gösterir. Resmin zemini saydamdır: kâğıdın ya da çerçevenin dolgusunun üstüne
      oturur.
- **GeoPDF:**
  - Her harita çerçevesi için sayfada bir `VP` (Viewport) ve onun `Measure` sözlüğü yazılır
    (`/Subtype /GEO`, `GPTS`, `LPTS`, `GCS`); ISO 32000-2 ve OGC en iyi uygulaması.
  - Köşelerin enlem-boylamını TM/UTM için çekirdek kendisi hesaplar (`geodesy.rs`); başka sistemde
    ev sahibi verir. Koordinat sisteminin WKT'si ev sahibinden gelir.
  - Sonuç: GDAL, QGIS ve Acrobat haritadan koordinat okur.
- **Belirleyicilik:**
  - Tarih ve belge kimliği girdiden türetilir: kimlik içeriğin özetidir.
  - Aynı girdi bayt bayt aynı PDF'i verir; iki platform da aynı PDF'i verir.
  - Üst veri: başlık, yazar ve konu değişkenlerden gelir; üretici “KentOS”.
- **Yazdır:** yeni paket gerekmez.
  - Web PDF'i üretip tarayıcının yazdırma penceresinde açar.
  - Masaüstü PDF'i geçici dosyaya yazıp sistemin görüntüleyicisinde açar.
- **Kabul ölçütleri:**

  | Araç | Beklenen |
  |---|---|
  | `gdalinfo` | koordinat sistemi; köşe koordinatları 1 cm içinde |
  | `pdffonts` | yazı tipleri gömülü ve alt küme |
  | `pdftotext` | Türkçe başlık ve tablo değerleri doğru |
  | `pdfinfo` | sayfa boyu |
  | `pdftoppm` | görüntüler gözle denetlenir |
  | altın PDF | özeti fixture'da |
  | WASM ve Rust | aynı girdiden aynı bayt |
- **Atlas:** çok sayfalı atlas PDF'i, atlas düzenleyicisi gelince aynı yazıcıyı kullanır.

### 10. Kalıcılık

- **Pafta kitabı projeye aittir.** 1. aşamada, `main`'deki merkezi biçimlere dokunmamak için kitap
  ayrı tutulur:
  - web: IndexedDB, ev sahibinin verdiği proje anahtarıyla;
  - masaüstü: uygulama veri klasöründe aynı anahtarla;
  - her iki yerde `.kpafta` dışa ve içe aktarma vardır.
- Birleştirmeden sonra `main`'de ayrı adım olarak:
  - `.kcad` 2.x'e `sheets` alanı eklenir (`minReaderMinor` ile);
  - veritabanı projeleri için sunucu tablosu kurulur ([birleştirme](../sheet/integration.md) §6).

  Bunu dalda yapmak `.kcad` kodeki ve iki platformun belgesiyle çakışırdı.
- Geri alma yığını pafta kipine özgüdür: model alanının geri almasından ayrıdır. Her girdi bir
  işlem ve tersidir, adı vardır (“Haritayı taşı”).

### 11. Arayüz

Paftalar ayrı bir pencere değil, **ana pencerenin bir kipidir**. Şerit tek kabuktur (DESIGN.md);
QGIS gibi ikinci bir menü ve araç çubuğu dünyası açılmaz.

```text
┌ Şerit ── [Giriş] [Çiz] … [Pafta]*  (pafta seçiliyken bağlamsal sekme)                              ┐
│ Pafta:  Yeni ▾ · Şablondan… │ Seç · El │ Harita · Metin · Lejant · Ölçek · Kuzey · Tablo ·          │
│         Koordinatlar · Antet · Resim · Şekil ▾ · Çizgi │ Hizala ▾ · Dağıt ▾ · Sıra ▾ · Grupla │       │
│         Ön denetim · Dışa aktar ▾ · Şablon olarak kaydet                                          │
├──────────────┬────────────────────────────────────────────────────────────────┬──────────────────┤
│ Paftalar     │   0    50   100  150  200  250  300  350  400   (mm cetveli)    │ Denetçi          │
│ ▣ Pafta 1    │  ┌──────────────────────────────────────────────┐              │ [Öğe] [Sayfa]    │
│ ▢ Pafta 2    │  │ ┌──────────────────────────┐ ┌─────────────┐ │              │ Konum ve boyut   │
│ ──────────── │  │ │                          │ │ ANTET       │ │              │  Sol  Üst        │
│ Öğeler       │  │ │        Harita            │ │ lejant      │ │              │  Gen. Yük.  ⟲    │
│ ▾ Antet   🔒 │  │ │                          │ │ ölçek  K    │ │              │ Kısıtlar  ┌─┐    │
│   Harita  👁 │  │ └──────────────────────────┘ └─────────────┘ │              │           └─┘    │
│   Lejant  👁 │  └──────────────────────────────────────────────┘              │ Görünüş · ƒ      │
├──────────────┴────────────────────────────────────────────────────────────────┴──────────────────┤
│ [Model] [Pafta 1] [Pafta 2] [+]                                                                   │
│ Kâğıt: 132,4 · 86,0 mm │ Seçim: 280,0 × 200,0 mm │ %65 │ Sayfa 1/2 │ ⚠ 2 ön denetim               │
└──────────────────────────────────────────────────────────────────────────────────────────────────┘
```

- **Sekmeler:** çizim alanının altında “Model | Pafta 1 | Pafta 2 | +”. Model sekmesi her zaman
  ilktir, kapanmaz (vitrindeki `sheets.rs` kalıbı). Pafta sekmesi seçilince:
  - çizim alanının yerini pafta görünüşü alır (masa zemininde kâğıt, cetveller, kılavuzlar, tutamaçlar);
  - şeritte bağlamsal **Pafta** sekmesi açılır;
  - sol panel “Paftalar ve öğeler” olur;
  - sağda denetçi açılır.
- **Denetçi:** KentOS bileşenleri (`PropertyGrid`, `Inspector`). Bölümler:
  - Konum ve boyut;
  - **Kısıtlar** (kare çizimli kısıt düzenleyicisi);
  - türün kendi özellikleri;
  - Görünüş;
  - Veri (ƒ bağları).

  Çoklu seçimde ortak alanlar düzenlenir; karışık değer “—” gösterilir.
- **Öğe ağacı:** sıra, görünürlük ve kilit; sürükleyerek sıra ve grup; çift tıkla ad.
- **Şablon galerisi** (“Pafta şablonları”):
  - sol tarafta **Sistem · Benim · Kurumum · Benimle paylaşılanlar**;
  - arama; kâğıt ve tür süzgeçleri; varsayılan olarak projenin kipine ve türüne göre süzülür
    (§11a), “Bütün kiplerin şablonları” anahtarıyla öbürleri de görünür;
  - kartlarda şablonun kendi çizim planından üretilen canlı küçük resim;
  - rozetler: “Sistem”, “Bu cihazda”, “Eşitlendi”, “Eşitlenmedi”, “Paylaşıldı”, “Yeni sürüm”;
  - sağ tarafta ayrıntı ve eylemler: Kullan, Çoğalt, Düzenle, Paylaş…, Eşitle, Sil.
- **Klavye** (yalnız pafta kipinde):
  - araçlar: V seç, H el (Boşluk basılıyken geçici), M harita, T metin, L lejant;
  - ok tuşları kaydırır (§5);
  - Ctrl+G grupla, Ctrl+Shift+G grubu çöz, Ctrl+D çoğalt, Ctrl+] / Ctrl+[ sıra;
  - Ctrl+0 sayfayı sığdır, Ctrl+1 gerçek boyut;
  - Delete siler, F2 ad verir.
- **Temalar:**
  - Grafit ve Pafta temalarında kâğıt hep beyazdır; masa zemini temanın yüzey rengindedir.
  - Seçim ve kılavuzlar temanın tek vurgu rengiyle çizilir, “bir vurgu, bir anlam”.
- **Masaüstü:** `kentos-sheet-ui` aynı düzeni `kentos-ui` bileşenleriyle kurar.
  - Kullanılan bileşenler: `rulers`, `inspector`, `property_grid`, `tree_view`, `tabs`,
    `segmented`, `number`, `dropdown`, `dialog`.
  - Kâğıt Iced `canvas` ile çizilir. Harita çerçeveleri `MapPainter`'ın verdiği görüntülerle
    dolar; görünüş değişmedikçe önbellekte kalırlar.

### 11a. Proje kipine göre araçlar ve şablonlar

KentOS'ta kip (`Workspace`: `hybrid | cad | gis | plan3d | disaster`, proje ayarı) **yalnız
görünüşü** belirler: hangi menü, sekme ve araç görünür. Verinin anlamını belirlemez; her komut her
kipte çalışır (`contracts/document.rs`). Proje türü (`ProjectType`: `cad`, `gis`, `landReadjustment`,
`zoningPlan`, `subdivision`, `road`, `architecture`) bir etikettir. Pafta düzeni bu ilkeye uyar:

- **Kip profili veridir:** `crates/shared/sheet/data/profiles.json`, iki platformda aynı. Her kip
  için şunları söyler:
  - Pafta şerit sekmesinin araçları ve grupları;
  - araçların hazır biçimleri (presets);
  - aynı öğenin o kipteki adı: CAD'de “Görünüm penceresi”, GIS'te “Harita”;
  - yeni paftanın varsayılan şablonu;
  - galerinin varsayılan süzgeci ve sırası.

  Arayüz hangi aracı göstereceğini koda gömmez, profilden okur.
- **Yetenekler kipten değil projeden gelir:** `georeferenced` (projenin koordinat sistemi var mı),
  `attribute_layers` (öznitelikli katman var mı), `plot_scale` (proje ayarının çizim ölçeği; yeni
  harita/görünümün varsayılan ölçeği). Bir araç, gerektirdiği yetenek yoksa görünmez ya da nedeniyle
  devre dışıdır. Örnek: koordinat sistemi olmayan genel CAD projesinde karelaj ve kuzey oku yoktur;
  koordinat sistemli “harita CAD” projesinde vardır.
- **Araçlar** (✓ görünür; “—” o kipte eklenmez):

  | Araç | CAD | GIS | Not |
  |---|---|---|---|
  | Görünüm penceresi / Harita | ✓ | ✓ | aynı `Map` öğesi; ad ve varsayılan profilden |
  | Metin, resim, şekil, çizgi, grup | ✓ | ✓ | ortak |
  | Antet, pafta çerçevesi | ✓ (bölge işaretli) | ✓ (sınır çizgisi) | ortak; hazır biçim farklı |
  | Ölçek çubuğu | ✓ | ✓ | ortak |
  | Lejant | ✓ (katmanlar ve çizgi tipleri) | ✓ (tematik) | ortak; hazır biçim farklı |
  | Sabit tablo | ✓ (revizyon tablosu, çizim listesi) | ✓ | ortak; hazır biçim farklı |
  | Koordinat listesi | ✓ | ✓ | koordinat sistemi varsa Y/X, yoksa yerel X/Y |
  | Kuzey oku, karelaj | ✓ koordinat sistemi varsa | ✓ | `georeferenced` ister |
  | Öznitelik tablosu | — | ✓ | `attribute_layers` ister |
  | Genel bakış haritası | — | ✓ | `Map` + `overview_of` hazır biçimi |
  | Atlas | ✓ koordinat sistemi varsa (pafta bölümleme) | ✓ | |

- **Hibrit** bugün CAD ve GIS profillerinin birleşimidir. Hibrit kip kaldırılınca yalnız
  `profiles.json`'daki satırı silinir; pafta düzeninde başka hiçbir şey değişmez. `plan3d` ve
  `disaster` açılınca kendi profil satırlarını alır. Profili olmayan bir kip **ortak** profili
  kullanır: ortak araçlar ve bütün şablonlar.
- **Kip dönüşümü kayıpsızdır.** Kip değişince pafta verisi değişmez.
  - Öbür kipin öğeleri yerinde kalır, çizilir, denetçide tamamen düzenlenir. Yalnız *yeni*
    eklenmeleri gizlenir; denetçide bunu söyleyen bir not görünür: “Bu öğe GIS kipinin aracıdır;
    CAD kipinde yenisi eklenmez, var olan düzenlenir.”
  - Geri dönüşte geri getirilecek bir şey yoktur, çünkü hiçbir şey atılmamıştır.
  - Yetenek kaybı ön denetimde görünür. Örnek: projenin koordinat sistemi kaldırıldıysa karelaj
    “koordinat sistemi gerekir” hatası verir ve iki çözüm önerir: koordinat sistemi seç ya da
    karelajı kaldır.
- **Şablonlar kipini ve türünü söyler:** `TemplateMeta.workspaces` (`["cad"]`, `["gis"]` ya da
  ortak için `["cad", "gis"]`) ve `project_types`.
  - Galeri önce projenin türüne uyanları, sonra kipine uyanları, sonra ortakları gösterir.
  - “Bütün kiplerin şablonları” anahtarı öbürlerini “GIS şablonu” gibi rozetlerle açar.
  - Başka kipin şablonu kullanılabilir, çünkü veri kipten bağımsızdır. Eksik yetenek varsa
    (örneğin koordinat sistemi) kullanmadan önce söylenir.

### 12. Şablonlar

```rust
pub struct Template {
    pub schema: String,              // "kentos.sheet.template/1"
    pub meta: TemplateMeta,          // id, revision, name, description, category, tags, papers,
                                     // workspaces, project_types (§11a), created, updated, author
    pub sheet: Sheet,                // haritalar ölçek taşır, yer (center) taşımaz
    pub master: Option<Master>,
    pub assets: Vec<AssetWithBytes>,
    pub variables: Vec<Variable>,    // kullanırken sorulacaklar (ada, parsel, mahalle…)
}
```

- **Sistem şablonları** uygulamayla gelir.
  - Yer: `crates/shared/sheet/templates/*.json`; `include_str!` ile gömülür.
  - Kimlik `sys:` ile başlar; salt okunurdur.
  - Web ve masaüstünde **aynı dosyalardır**.
  - Yeni sürüm uygulamanın yeni sürümüyle gelir; revizyon artar.

  1. aşamanın şablonları (hepsi kısıtlı, her kâğıtta kullanılabilir):

  | Kimlik | Ad | Kip | Proje türü | Önerilen kâğıt |
  |---|---|---|---|---|
  | `sys:genel-a4-dikey` | Genel pafta | ortak | — | A4 dikey |
  | `sys:genel-a3-yatay` | Genel pafta, sağda antet şeridi | ortak | — | A3 yatay |
  | `sys:rapor-sayfasi` | Rapor sayfası (başlık, harita, metin, tablo) | ortak | — | A4 dikey |
  | `sys:cad-teknik` | Teknik çizim paftası (bölge işaretli çerçeve, antet, revizyon tablosu) | CAD | `cad`, `road` | A3 yatay |
  | `sys:cad-mimari` | Mimari pafta (sağda geniş antet şeridi, çizim listesi) | CAD | `architecture` | A1 yatay |
  | `sys:aplikasyon-krokisi` | Aplikasyon krokisi (koordinat listesiyle) | CAD | `subdivision`, `landReadjustment` | A4 dikey |
  | `sys:ifraz-paftasi` | İfraz / tevhit paftası (karelaj, parsel tablosu) | CAD | `subdivision`, `landReadjustment` | A3 yatay |
  | `sys:imar-plani` | İmar planı paftası (lejant, plan notları, onay hücreleri) | ortak | `zoningPlan` | A1 yatay |
  | `sys:gis-tematik` | Tematik harita (lejant paneli, genel bakış haritası, kaynak notu) | GIS | `gis` | A3 yatay |
  | `sys:gis-atlas` | Atlas sayfası (nesne bilgisi tablosu, genel bakış, sayfa no) | GIS | `gis` | A4 dikey |

  Bu şablonlar genel örneklerdir; bir yönetmeliğe uygunluk iddiası taşımazlar. Mevzuata bağlı
  şablonlar ilgili uzman onayıyla ayrı veri sürümü olarak gelir.
- **Kullanıcı şablonu:** “Şablon olarak kaydet” açık paftadan bir şablon çıkarır:
  - haritaların yer merkezi atılır; ölçek kalır;
  - kullanılan varlıklar toplanır;
  - değişkenler sorulacaklar listesine alınır.

  Önce **bu cihazda** saklanır: web'de IndexedDB, masaüstünde uygulama veri klasörü.
- **Kullanma:** şablon kopyalanarak pafta olur, yeni kimliklerle.
  - Kâğıt seçilirse kısıtlarla yerleşir.
  - Paftada `origin { template_id, revision }` kalır. Şablonun daha yeni revizyonu varsa sekme ve
    galeri “yeni sürüm var” der. 1. aşama yalnız bildirir; güncellemeyi uygulamak 2. aşamadadır.
- **Doğrulama ve göç:** `validate(json)` bütün kuralları uygular, `migrate(json)` eski sürümü
  yükseltir. Bozuk şablon sessizce yarım yüklenmez.

### 13. Şablon kitaplığı, bulut eşitlemesi ve paylaşım

**Kaynaklar:**

| Kaynak | Ne | Kim değiştirir |
|---|---|---|
| Sistem | uygulamayla gelen, iki platformda aynı | kimse (salt okunur) |
| Bu cihaz | kullanıcının buluta eşitlenmemiş şablonları | kullanıcı |
| Bulutum | kişisel alandaki (ADR 0015) eşitlenmiş şablonlar; web ve masaüstünde aynı | sahip; düzenleyici paylaşımı olanlar |
| Benimle paylaşılanlar | başkasının paylaştığı | görüntüleyici: kullanır, çoğaltır; düzenleyici: yeni revizyon kaydeder |
| Kurumum | kurum alanında yayımlanan, kurumun etkin ve koltuklu bütün üyelerinin gördüğü (misafir görmez) | yayımlayan ve kurum yöneticileri; öbür üyeler kullanır ve “Şablonlarıma kopyala” der |

**Kurallar:**

- **Eşitlemeden paylaşım yok.** Yalnız bu cihazdaki bir şablonda “Paylaş…” devre dışıdır; nedenini
  söyler: “Paylaşmak için önce buluta eşitleyin.”
- **Eşitleme çekirdekte planlanır.** `plan_sync(local, remote) -> SyncPlan` saf ve belirleyicidir.
  Ağ işini platform yapar. Kurallar:

  | Yerel | Uzak | Sonuç |
  |---|---|---|
  | temiz | daha yeni | indir |
  | değişmiş | yerelin dayandığı revizyon | yükle (`expectedRevision`) |
  | değişmiş | daha yeni | **çakışma:** ikisi de kalır; yerel “(bu cihazdaki kopya)” adıyla yeni şablon olarak yüklenir, uzak indirilir |
  | temiz | silinmiş | yerelden kalkar |
  | değişmiş | silinmiş | yerelde “bu cihazda” olarak kalır; kullanıcıya söylenir |
  | silinmiş | aynı revizyon | uzakta da silinir |
  | silinmiş | daha yeni | uzak kalır; kullanıcıya söylenir |

  **Hiçbir yol sessizce veri yitirmez.**
- **Paylaşım:**
  - Kişiler, proje paylaşımının arama kurallarıyla bulunur (ADR 0024): yalnız ortak kurumların
    etkin üyeleri; genel e-posta araması yok, yani hesap sayımı yok.
  - Roller: **görüntüleyebilir**, **düzenleyebilir**. Sahiplik devri 2. aşamadadır.
  - Kurum dışındaki kişiye e-postayla davet, ADR 0035'in kalıbıyla 2. aşamadadır.
  - Paylaşım geri alınınca şablon karşı tarafın “Benimle paylaşılanlar”ından kalkar. Ondan
    kullanılmış paftalar kalır; onlar zaten kopyadır.
- **Çevrimdışı:** Sistem ve bu cihazın şablonları her zaman kullanılır. Bulut şablonlarının son
  indirilen revizyonu yerelde önbellektedir. Bağlantı gelince eşitleme kendiliğinden çalışır;
  durum çubuğu ve galeri bunu söyler.

**Kurum şablonları** (sahibin onayıyla 2026-10-03'te kapsama alındı):

- **Yayımlama:** kurum sahibi, kurum yöneticileri ve kurumda `project.create` yetkisi olan
  üyeler yayımlar. İki yol vardır:
  - kurum alanında doğrudan oluşturmak;
  - kendi şablonundan “Kuruma yayımla…”: kurum alanına kopyalar, kaynağı `publishedFrom` olarak
    tutar.
- **Roller:** yayımlayan ve kurum yöneticileri düzenler ve siler. Öbür etkin, koltuklu üyeler
  görüntüleyicidir. Misafir ve kurumdan ayrılan kişi görmez. Rolü tek yer hesaplar
  (`sheet_template_role`'ün kurum kolu); satır güvenliği ona dayanır.
- **Komutlar:** var olan beş komut kurumun alanına da gider (`POST /v1/tenants/{kurum}/commands`).
  Yeni komut: `sheet.template.publish` v1 `{ templateId, tenantId }`.
- **Liste:** `GET /v1/me/sheet-templates` artık `organizations: [{ tenantId, name, canPublish,
  templates }]` da döner. Olaylar kurumun üyelerine gider.
- **Negatif testler:**
  - Üye olmayan ve misafir 404 alır.
  - Yayım yetkisi olmayan üye yayımlayamaz ve düzenleyemez (403).
  - Kurumdan ayrılınca erişim düşer.
  - Satır güvenliği başka kurumun şablonunu göstermez.
- **İstemciler:** web ve masaüstünde “Kurumum” bölümü canlıdır: kurum adına göre gruplar,
  “Kurum: <ad>” rozeti, “Kuruma yayımla…”, “Şablonlarıma kopyala”. Eşitleme kişisel şablonlarla
  aynı kurallarla yürür.
- **Uygulama (3 Ekim):**
  - Göç `0014_org_sheet_templates.sql`. 0013 değiştirilmedi, çünkü saklanan bir veritabanına
    uygulanmış olabilir.
  - Rol `sheet_template_role(şablon, sahip, alan)`. Kurum şablonunda:
    - `owner`: yayımlayan, yayımlama yetkisi sürdükçe; yetkisi giden yalnız kullanır;
    - `admin`: kurumun sahibi ve yöneticileri (yeni rol);
    - `viewer`: öbür etkin, koltuklu üyeler;
    - öbürleri hiçbir şey görmez.
  - Yayımlama yetkisi `sheet_template_can_publish(kurum)`: 0012'deki proje kopyalama kuralı
    (etkin üyelik, koltuk, sahip, yönetici ya da proje yöneticisi).
  - Komutun yolu şablonun alanıdır: kurum şablonununki kurumun yolu, kişisel alanınki kişisel
    alanın yolu. Yanlış yol 422 alır.
  - Kurum şablonunda paylaşım yoktur (422). Listedeki `templates` kişiseldir; kurumunkiler
    `organizations[].templates`'tedir.
  - `publish` yalnız kendi kişisel şablonunu ve onun bulutta son revizyonunu kopyalar.
    - Cihazda eşitlenmemiş değişiklik varsa önce eşitlenir.
    - Aynı kuruma ikinci kez yayımlamak ikinci bir kopya açar. İstemci bunun yerine “Kurumdakini
      güncelle”yi önerir: kopyanın yeni revizyonu, eşitlemenin `expectedRevision`'ıyla.
  - Kurum şablonunun çakışma kopyası kişisel alana gider.
  - Listeden düşen kurum şablonunun cihazdaki kopyası kalkar ve kullanıcıya söylenir.
  - Kurum şablonları kişi başına 500 sınırına sayılır.

**Sunucu** (mevcut kalıplar: tenant, satır güvenliği, komut günlüğü, idempotency, outbox, WebSocket):

```sql
kentos.sheet_template (id uuid pk, tenant_id → tenant, owner_user_id → app_user, name, description,
    category, tags text[], papers text[], revision int, sha256 bytea, size int,
    created_at, updated_at, deleted_at)
kentos.sheet_template_revision (template_id, revision, content bytea, sha256, size,
    author_user_id, created_at, pk (template_id, revision))           -- değişmez
kentos.sheet_template_grant (template_id, user_id, role ('viewer'|'editor'),
    granted_by, created_at, pk (template_id, user_id))
```

- **Okuma uçları:**
  - `GET /v1/me/sheet-templates`: benim ve benimle paylaşılan, üst veri ve rolüm.
  - `GET /v1/sheet-templates/{id}`: üst veri ve en yeni içerik; `ETag` revizyondur.
  - `GET …/{id}/access`: yalnız sahip.
  - `GET …/{id}/access/candidates?q=`.
- **Değiştiren komutlar** (`POST /v1/tenants/{alan}/commands`, ürün komutu sözleşmesiyle, ADR 0013):
  - `sheet.template.create` v1;
  - `sheet.template.update` v1 `{ templateId, expectedRevision, … }`;
  - `sheet.template.delete` v1;
  - `sheet.template.share` v1 `{ templateId, userId, role }`;
  - `sheet.template.unshare` v1.
- **Olay:** `sheet_template.changed` sahibe ve paylaşılanlara gider; öbür cihaz listesini yeniler.
- **Sınırlar:** içerik ≤ 8 MB (varlıklar dahil); ad ≤ 120 karakter; kişi başına ≤ 500 şablon.
- **Negatif testler** ADR 0024 düzeyindedir:
  - başkasının şablonu 404, var olmayanınkiyle aynı gövdeyle;
  - sahip olmayan paylaşamaz (403);
  - görüntüleyici yazamaz;
  - kimse kendi rolünü yükseltemez;
  - kurum dışına paylaşım yok.

İstek ve yanıt tipleri `kentos-sheet`'in `cloud` modülünde tanımlanır ve TypeScript'e üretilir. Web,
masaüstü ve sunucu aynı tipleri kullanır.

### 14. Sınama

| Kanıt | Yer |
|---|---|
| İşlem, tersi, normalleştirme | `crates/shared/sheet/tests/ops.rs` |
| Yapışma | `fixtures/sheet/v1/snap/*.json` (sorgu → beklenen sonuç) |
| Yeniden yerleşim | `fixtures/sheet/v1/relayout/*.json` |
| Çizim planı | `fixtures/sheet/v1/display/*.json` (kitap + girdiler → liste) |
| SVG | `fixtures/sheet/v1/svg/*.svg` (bayt bayt) |
| Şablon doğrulama | `fixtures/sheet/v1/templates/{valid,invalid}/*.json` |
| Eşitleme planı | `fixtures/sheet/v1/sync/*.json` |
| Ön denetim | `fixtures/sheet/v1/preflight/*.json` |
| WASM sınırı | web vitest: aynı fixture'lar `kentos-sheet-wasm` üzerinden |
| Sunucu | `crates/server/application/tests/sheet_templates.rs`, `apps/api/src/http/sheet_templates_tests.rs` |
| Görüntü | web: `node apps/web/scripts/e2e/sheet-shots.mjs`; masaüstü: `cargo test -p kentos-sheet-ui screens -- --ignored`; ikisi de Grafit ve Pafta temasında |

### 15. Başarım hedefleri

| İş | Hedef |
|---|---|
| Yapışma sorgusu, 200 öğe | ≤ 0,5 ms (WASM) |
| Çizim planı, A0, 50 öğe, 2 karelaj | ≤ 5 ms (WASM) |
| Sürüklerken kare | 60 fps; harita çerçeveleri bit eşlem önbelleğinden |
| Şablon galerisi, 100 kart | ilk görüntü ≤ 300 ms; küçük resimler sırayla |

## Adımlar

| Adım | Kim | İş | Kabul |
|---|---|---|---|
| 1 | Rust | `kentos-sheet`: model, işlemler, yapışma, yeniden yerleşim, metin ölçüsü, çizim planı, SVG, şablonlar (10 sistem şablonu, kip profilleri), eşitleme planı, ön denetim, atlas planı | `cargo test -p kentos-sheet`, clippy `-D warnings`, `pnpm arch:deps`, fixture'lar |
| 2 | Rust | `kentos-sheet-wasm`; TS tipleri `contracts/generated/sheet/`; `pnpm wasm` girdisi | `pnpm wasm` temiz; web'den bir fixture'ın WASM'da geçmesi |
| 3 | Web | pafta kipi: sekmeler, görünüş, cetvel, boyayıcı, harita çerçevesi, araçlar, akıllı kılavuz, denetçi, öğe ağacı, işlemler ve geri alma, SVG/PNG, galeri (sistem + bu cihaz), şablon olarak kaydet | `pnpm typecheck`, `pnpm test`, görüntüler (iki tema) |
| 4 | Rust | sunucu: tablolar, komutlar, uçlar, olay, negatif testler; `kentos-cloud` şablon istemcisi | `cargo test` (veritabanlı testler `KENTOS_TEST_DB` ile), clippy |
| 5 | Web | bulut eşitlemesi, paylaşım penceresi, rozetler, çevrimdışı | vitest + `e2e:cloud` kalıbında görüntüler |
| 6 | Rust | `kentos-sheet-ui` (Iced): aynı kip; `apps/desktop` bağlantısı; masaüstü eşitlemesi | `pnpm rust:test:desktop`, `screens` görüntüleri |
| 7 | birleştirmeden sonra, `main` | `.kcad` 2.x `sheets` alanı, veritabanı projesi tablosu, ürün komutu kataloğu (Python, MCP), PDF/GeoPDF, manyetik model, atlas toplu çıktı, kurum şablonları, e-postayla şablon daveti | her biri kendi ADR adımıyla |

## Sonuçlar

- `CAD-07`'nin çekirdeği ve `OUT-01`, `OUT-02`, `OUT-03` kapanır.
  - `OUT-01`: değişkenler ve dinamik metin.
  - `OUT-02`: karelaj.
  - `OUT-03`: atlas planı ve pafta adlandırma.
- `CAD-08` kısmen kapanır: mm ve ölçek doğruluğu, dpi. Kalem tabloları ve PDF/DXF eşdeğerliği
  7. adımdadır.
- Yeni dış bağımlılık yoktur. Bağımlılık isteyen PDF ve manyetik model, sahibin kararını bekler.
- Mimari denetime bir grup (`sheet-ui`) eklenir. Masaüstü grubunun `uses` listesine `sheet-ui`
  girer.

## Açık sorular (sahibin kararı)

1. ~~**PDF:** `pdf-writer` + `subsetter`.~~ **Karar verildi** (2026-10-03, sahip: “bunları da
   komple bitir”): PDF ve GeoPDF kapsamda (§9a).
2. **Yazı tipleri:** SVG ve PDF'te gömülsün mü, eğriye mi çevrilsin? Varsayılan gömme, alt küme.
3. ~~**Manyetik kuzey.**~~ **Karar verildi** (2026-10-03): WMM2025 veri olarak eklenir (§8a).
4. ~~**Kurum şablonları.**~~ **Karar verildi** (2026-10-03): kurum sahibi, yöneticiler ve
   `project.create` yetkisi olanlar yayımlar (§13).
6. **Masaüstünde resim kalitesi** (2026-10-03'te karar verildi): Iced'in görüntü ve SVG desteği
   açılır. Yeni paketler: `image` (kod çözücüsüz), `kamadak-exif`, `resvg`/`usvg`; `docs/deps`'e
   yazılır. Resimler büyütülünce yumuşar; SVG resim çizilir.
   **Uygulandı (3 Ekim):**
   - Resim doku olarak çizilir: kendi boyu ve yarılanmışları. Küçük gösterilende boyuna yakın
     yarılanmış, büyütülende tam katlı büyütülmüş ve yumuşatılmış doku kullanılır.
   - Her resim kendi katmanındadır, çizim sırası korunur. 400 000 hücre sınırı kalktı.
   - SVG resmini resvg çizer, masaüstünde de eklenir.
   - PNG çıktısı ekranla aynıdır.
   - PDF'e SVG resmi, ev sahibinin çizdiği PNG olarak gider (9. adım, §9a).
   - Şablon galerisinin küçük resimleri de çizim sırasını korur (9. adım). Her harita ve resim
     iki katman arasında kendi tuvalindedir, paftadaki gibi.
   - Ayrıntı `tasks-rust.md` Sapmalar 111–115 ve 116–121'de.
5. **`.kcad` 2.x `sheets`:** hangi sürümle gelsin?
