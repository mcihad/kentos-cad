# ADR 0196: Eğri boyunca yazı

- **Durum:** kabul edildi (2026-10-07). Sıra sahibin kararıdır: TODOS.md §16.1, `CAD-33`'ün ardından `CAD-34`; madde tek parçada biter ve
  sahibin 6 Ekim gecesi sözüyle CAD-36'ya dek ara verilmez. Ayrıntılar bu ADR'nin varsayılanlarıdır. Örnekler ArcGIS'in açıklamasının
  “Follow feature”ı (eğri tabanlı açıklama), QGIS'in çizgi boyunca kıvrık etiketi ve AutoCAD Express Tools'un ARCTEXT'idir.
- **Bağlam belgesi:** TODOS.md `CAD-34` (etiket motorundaki karşılığı `GIS-16`); ADR 0145 (yazı ekleri: hiza, genişlik, zemin, Okunur
  yap), ADR 0182 (çok satırlı yazı: harflerin ilerlemeleri, satır kayıtları), ADR 0183 (yazı stilleri), ADR 0175 (nesneye bağlı yazı),
  ADR 0184 §7 (tablonun DXF'e adsız blok olarak yazılması), ADR 0055 (çizimin yazıları ortak depodan).

## Bağlam

Dere, yol ve kanal adları çizginin kıvrımını izler; bugün yazının tek dönüklüğü vardır. Ayrıca düz bir yazıyı bir yolun ya da sınırın
doğrultusuna çevirmek elle açı ölçmeyi gerektirir.

## Karar

### 1. Veri

- `TextEntity.path` (`TextPath`, `.kcad` şema 25): yazının taban eğrisi. Köşeleri yazının kendi çerçevesindedir: başlangıcı `p`, x
  ekseni `rotation` doğrultusunda, y ekseni ona dik ve solda; metre. İlk köşe `p`'nin kendisidir ve yazılmaz; `pts` ondan sonraki köşeler
  (en az bir); `bulges` her kenarın kavisi (kenar i, köşe i'den köşe i + 1'e; köşe 0 `p`): DXF'in tan(θ/4)'ü, saat yönünün tersi artı;
  yoksa bütün kenarlar düz, varsa `pts` kadar.
- Çerçeve sayesinde yazı taşınınca, döndürülünce ve ölçeklenince eğri de onunla gider; Öznitelikler'de yükseklik değişince eğri
  değişmez, dönüklük değişince eğri `p`'nin çevresinde döner, hiza değişince harfler eğri üstünde kayar.
- Eğrisi olan yazı tek satırdır: satır sonu, kutu genişliği ve satır aralığı olmaz; nesneye bağlı (`labelOf`) da olmaz. Harf
  biçimleri, stil ve yazının yüzü (yazı tipi, kalın, eğik, yatıklık), genişlik çarpanı ve zemin geçerlidir.
- Ret `invalid_path` (`cad.entities.create`, `cad.entities.edit`): köşesi yok; bir sayı sonlu değil; kavis sayısı köşe sayısından farklı;
  eğrinin uzunluğu sıfır; yazıda satır sonu, kutu genişliği, satır aralığı ya da `labelOf` var.

### 2. Yerleşim (ortak çekirdek `text::along`)

1. **Eğri dünyada:** köşe 0 `p`, köşe i `p + R(rotation)·pts[i − 1]`. Kavisli kenar yaydır: kiriş c, θ = 4·atan(b), yarıçap
   c / (2·|sin(θ/2)|), uzunluğu r·|θ|; düz kenarın uzunluğu kirişidir. S bütün kenarların uzunluğu.
2. **Harflerin ilerlemeleri** çok satırlı yazınınkilerdir (ADR 0182 §2): yazının yazı tipinin tablosu, kalın harfe (yazının ya da
   biçiminin) kalın tablo, üst ve alt simge 0,6 katı; metre olarak ilerleme / 1000 × yükseklik × genişlik çarpanı. L hepsinin toplamı.
3. **Başlangıç:** s0 = a·(S − L); a hizanın yatay payı (sol 0, orta ½, sağ 1; hizasız 0).
4. **Harf i'nin ortası** mᵢ = s0 + Σⱼ₍ⱼ₍ᵢ₎ Aⱼ + Aᵢ/2. Eğri üstündeki noktası P ve birim teğeti T: 0 ≤ m ≤ S iken m'yi içeren kenarda
   (başlangıcı m'ye eşit ya da küçük olan son kenar; uzunluğu sıfır olan kenar sayılmaz); m < 0 iken ilk kenarın başındaki teğet
   boyunca geriye, m > S iken son kenarın sonundaki teğet boyunca ileri, düz.
5. **Harfin yeri:** dönüklüğü T'nin açısı (derece, 0'dan 360'a); taban başlangıcı Bᵢ = P − T·Aᵢ/2 − N·(u·h), N sol dik (−T.y, T.x), u
   hizanın düşey payı (taban 0, alt −0,2, orta ½, üst 1; ADR 0145). Eğri böylece harflerin tabanından, altından, ortasından ya da
   üstünden geçer.
6. **Çizimin yazıları** (ADR 0055): harf başına bir `LABEL_LINE` (x, y Bᵢ; a harfin dönüklüğü; b yükseklik; c genişlik çarpanı; d, e
   i ve i + 1); zeminli yazıda her harften önce harfin zemini (`LABEL_PARAGRAPH_MASK`, tek satırlık yazınınki gibi, genişliği harfin
   ilerlemesi). İki ev sahibi bu kayıtları bugünkü gibi çizer; araçların önizlemesi de aynı kayıtlarla (`textLines`).
7. **Kutusu** (tıklama, pencere ve çokgenle seçme, taramanın yazı açıklıkları, Genel bakış): harflerin kutularının (taban başlangıcından
   ilerlemesi kadar; tabanın 0,23·h altından 1,15·h üstüne; yatık yazıda yatık) alt köşeleri sırayla, sonra üst köşeleri tersten: tek
   çokgen. Kapsamı bu çokgenin kutusudur.
8. **Doğrultusu** ilk harfin ortasından son harfin ortasına (tek harfte ya da iki nokta 1e−9 m içindeyse ilk harfin teğeti). Ters okunur:
   doğrultu 90°'den büyük ve en çok 270° (ADR 0145 §3'ün kuralı).

### 3. Ters çevirme, Okunur yap ve dönüşümler

- **Ters çevirme:** dünyadaki köşeler tersten (son köşe yeni `p`), kavisler tersten ve işaretleri dönük; dönüklük 180° artar (0'dan
  360'a); hizanın yatay payı değişir (sol ↔ sağ, orta kalır). Harfler eğrinin öbür yanına geçer.
- **Okunur yap** (ADR 0145 §3) ters okunan eğri boyunca yazıyı ters çevirir ve düşey payını da değiştirir (taban ve alt → üst, üst →
  taban, orta kalır): harfler eğrinin aynı yanında, aynı yerde kalır, okunur olur. Hizasız yazı (sol taban) üst sağ olur.
- **Benzerlik dönüşümleri** (taşı, kopyala, döndür, ölçekle, diziler, hizala ve dağıt): `p` taşınır, dönüklük döner (düz yazınınki gibi),
  `pts` ölçekle çarpılır, kavisler kalır.
- **Aynala:** dünyadaki köşeler aynalanır, kavislerin işareti döner; sonra yazı ters çevrilir (harfler eğrinin aynadaki yanına geçer);
  eğrisi ters okunuyorsa (ilk köşesinden son köşesine doğrultu; ikisi 1e−9 m içindeyse ilk kenarın başındaki teğet) bir kez daha, düşey
  payıyla birlikte (Okunur yap'ın kuralı). Yazı okunur kalır (AutoCAD'in MIRRTEXT = 0'ı gibi); dönüşüm yazı tipini bilmediği için eğrinin
  doğrultusuna bakılır.
- **Vektör oturtma ve Kauçuk levha** (ADR 0156, 0158): köşeler dönüşümle taşınır, kavisler kalır; dönüklük ve yükseklik düz yazınınki
  gibi.

### 4. Araçlar: Eğri boyunca yazı ve Yazıyı eğriye oturt

İki araç tek bölünmüş düğmededir (**Eğri boyunca yazı ▾**): CAD'de Açıklama › Yazı panelinde, CBS'de Harita › Etiket panelinde. Dört
satırın her biri kendi ikonuyla:

- **Eğri boyunca yazı** (`textAlong`): (1) eğriye tıklayın: çizgi, yay, daire, çoklu çizginin parçası, alanın halkası ya da deliği, elips,
  eğri (spline); (2) yazının yerine tıklayın: nokta eğrinin en yakın yerine iner, önizlemede son yazılan yazı (ilk kez “Yazı”) imleçle
  eğri boyunca kayar, seçilen eğri vurgulu kalır; (3) tıklanan yerde açılan kutuya yazıp Enter (kutu son yazıyla açılır). Sonra araç yeni
  bir eğri bekler; Esc yerleştirmeden eğri seçimine, oradan çıkışa döner. Seçenekler Yükseklik (Y, Yazı ile ortak), Hiza (H: Başı, Ortası,
  Sonu; tıklanan yer yazının başı, ortası ya da sonudur), Konum (K: Üstünde, Ortasında, Altında; alt, orta ya da üst düşey pay), CAD
  projesinde Stil (S). `cad.entities.create`'in `textAlong` işlemi, adım “Eğri boyunca yazı”.
- **Yazıyı eğriye oturt** (`textCurve`), değiştirme araçlarının tabanında (önce yazılar ya da seçim), yöntemlerinin harfleriyle:
  - **Eğriye oturt** (O): seçili yazılar tıklanan eğriye oturur: her yazının kutusunun ortası eğrinin en yakın yerine iner; hizası,
    yüksekliği ve yüzü kalır. Çok satırlı ve nesneye bağlı yazı atlanır, söylenir. `cad.entities.edit`'in `textPath` işlemi, adım
    “Eğriye oturt”.
  - **Doğrultuya döndür** (D): seçili düz yazılar tıklanan kenarın o yerdeki doğrultusuna döner (okunur: −90°'den büyük, en çok 90°), her
    yazı kendi noktasının çevresinde. Eğri boyunca yazı atlanır, söylenir. `textTurn` işlemi, adım “Doğrultuya döndür”.
  - **Düzleştir** (Z): seçili eğri boyunca yazılar hemen düz yazı olur: noktası ilk harfin taban başlangıcı, hizasız, dönüklüğü yazının
    doğrultusu (§2.8). Düz yazı atlanır, söylenir. `textStraighten` işlemi, adım “Düzleştir”.
  Seçimde hiç yazı yoksa araç söyler ve çıkar; yöntemin alacağı yazı yoksa ilk eyleminde söyler.

**Eğrinin parçası** (Eğri boyunca yazı ve Eğriye oturt): eğrinin yolları (çizgi, yay açık; daire kapalı; çoklu çizginin her parçası açık;
alanın her halkası ve deliği kapalı; elips ve eğri 0,1 mm'lik kirişleriyle) içinden tıklanan noktaya en yakın olanı; üstünde tıklanan
yerin uzaklığı s. Teğet s'de ters okunuyorsa yol ters çevrilir (s yolun uzunluğundan çıkarılır). Parça yazının uzunluğu L kadardır:
s − a·L'den başlar (Eğriye oturt'ta a = ½); açık yolda başı 0 ile S − L arasına çekilir (S < L ise bütün yol), kapalı yolda yolun başından
geçerek sürer (L ≥ S ise bütün halka bir kez). Parçanın ilk köşesi yazının `p`'si, ilk köşeden sonuncuya doğrultusu (ikisi 1e−9 m içindeyse,
bütün halkada, ilk teğeti) dönüklüğü, köşeleri bu çerçevede `pts`.

Öznitelikler'de eğri boyunca yazının **Eğri** satırı eğrinin uzunluğunu gösterir (salt okunur).

### 5. Dosyalar

- **DXF:** eğri boyunca yazı adsız blok (*U) olarak yazılır: her harf kendi TEXT'i (taban başlangıcında, kendi dönüklüğüyle; yazının
  yüksekliği, genişlik çarpanı ve stiliyle), bloğun çerçevesi yazınınkidir (`p` başlangıçta, dönüksüz); INSERT `p`'de, yazının
  dönüklüğüyle. INSERT'in KENTOS verisi (`curve`) yazının kendi alanlarıdır: KentOS yazıyı geri okur (INSERT'in yeri, dönüklüğü ve
  ölçeğiyle, tablo gibi). Başka programlar harfleri blok olarak gösterir; rapor söyler. Zemin başka programlarda yoktur.
- **GeoJSON ve Shapefile:** yazı bugünkü gibi yazılmaz (rapor söyler); eğri boyunca yazı da.
- **Sunucu:** yazının `cad_definition`'ı alanlarını taşır, eğri dahil.

## Kapsam dışı

Eğrinin köşelerini tutamaçla düzenlemek (Eğriye oturt yeniden oturtur), harf aralığı ve yazıyı eğriye sığdırma, harflerin eğriyle
birlikte dönmeyen (dik duran) yerleşimi, etiket motorunun kıvrık etiketi (`GIS-16`), NCZ'nin eğri yazısı.

## Uygulama

- **Sözleşme:** `TextEntity.path` ve `EntityGeometry::Text`'in `path`'i (`TextPath`, `entity.rs`), kuralları `TextPath::problem` ve
  `text_path_problem`; `EditOperation`'ın `TextPath`, `TextTurn`, `TextStraighten`'i; `CreateOperation::TextAlong`; ret `invalid_path`
  (dizinin yolununkiyle aynı ad, anlamı alanın). `FORMATS_VERSION` 35.
- **Çekirdek:** `text::along` (`Curve`, `Along`, `Letter`, `Placed`; yerleşim, kutu, kayıtlar, ters çevirme, `swapped`, `transformed`,
  `readable_turn`, `paths_of`, `piece`; işlemler `textAlongPiece`, `textAlongLength`, `textAlongReadable`, `textAlongStraight`,
  `textAlongTurn`, `textAlongCurveLength`). `TextPlace`'in `path`'i ve `along`'u: `origin`, `outline_grown`, `readable` (eğride yok),
  `realigned` (eğride nokta kalır); deponun `paragraph_records`'u eğri boyunca yazıya harf başına kayıt verir (`by_records`). Dönüşüm
  (`ops::transform`), çarpıtma (`ops::warp`), tutamaç, bölme karşılaştırması, veri karşılaştırma ve paketleyici (`store/pack.rs`, web
  `wasm/pack.ts`) eğriyi taşır.
- **`.kcad` şema 25:** kodek (`decode/objects.rs`, `encode/objects.rs`, `columns.rs`, web `io/columns.ts`), spesifikasyon §6.1 ve §6.6,
  bağımsız okuyucu `tools/kcad/kcad.py`, bağımsız yazıcı `scripts/fixtures/kcad_v2_reference.py`; örnek `text-paths.kcad` (belgede ve
  blok tanımında), dokuz bozuk dosya; desteklenmeyen sürüm dosyası `schema-version-26.kcad`.
- **DXF:** yazıcı `dxf/writer/entities/curved.rs` (harflerin bloğu, KENTOS verisinin `along`'u), okuyucu `emit.rs`'in `curved_of`'u;
  blok tanımındaki eğri boyunca yazı harf harf yazı olur.
- **Komutlar:** masaüstü `crates/native/application` (`create.rs`'in bağ ve eğri reddi, `edit.rs`'in denetimi ve adları), web
  `product/entitiesCreate.ts`, `entitiesEdit.ts` (`textPathProblem`, `model/entities.ts`).
- **Araçlar:** masaüstü `kentos_interaction::text_along` (`TextAlong`, `TextCurve`; oturumun `along_align`'ı), web
  `tools/textAlongTool.ts`, `model/textAlong.ts`; katalogda `textAlong` ve `textCurve` tek ailede; Okunur yap iki platformda eğriyi
  çevirir; Öznitelikler'de **Eğri** satırı (eğrinin uzunluğu). İkonlar `textAlong`, `textFit`, `textTurn`, `textStraighten` (sahip
  uyurken önerilen seçenekler; Doğrultuya döndür'ün ikincisi 16 pikselde daha okunaklı olduğu için, Eğri boyunca yazı'nın kemeri sonradan
  derinleştirildi).

## Doğrulama

- Bağımsız başvuru `scripts/fixtures/text_along_cases.py` (ADR'den, KentOS kodu olmadan; harflerin ilerlemeleri `metrics.rs`'ten veri
  olarak): yerleşim, kutu, kayıtlar, doğrultu, Okunur yap, dönüşümler (aynalar dahil), eğrinin parçası (çizgi, yay, daire, çoklu çizgi,
  çok parçalı çizgi, alanın halkası ve deliği), Düzleştir ve Doğrultuya döndür; ortak `fixtures/text/v1/along.json`. Çekirdek
  (`tests/all/text_along.rs`) ve web WASM'ı (`tools/textAlong.wasm.test.ts`) 1e−9 içinde geçer; çekirdek yazının çerçevesinde,
  başvuru dünyada hesaplar.
- `.kcad`: Rust kodeği bağımsız yazıcının baytlarını yazar ve okur (`fixtures/kcad/v2/expected.json`), Python okuyucu ve web aynı dosyaları.
- DXF: `fixtures/formats/v1/dxf-write/curved.dxf` yazıcının baytları; `scripts/fixtures/dxf_write_reference.py` her harfin yerini ve
  dönüklüğünü bağımsız yerleşimle denetler; `curved_texts_read_back_as_they_were` yazıları aynen geri okur.
- Komutlar: `fixtures/commands/v1/cad.entities.create.json` ve `.edit.json`'un eğri durumları (`create_command_cases.py`,
  `edit_command_cases.py`), iki platformda.
- Ortak iz `fixtures/interaction/v1/text-along.json` (`text-along.kcad`) iki platformda, üç varyantta; `shot` adımlarıyla resimler
  (`kentos-cad kullan text-along`, `pnpm -C apps/web e2e:use text-along`); ikon turunda `egri-yazi`.

