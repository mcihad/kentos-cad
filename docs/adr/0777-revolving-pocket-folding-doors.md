# ADR 0777: Döner, cebe giren ve katlanır kapı; kayar kapının rayı ve bindirmesi

- **Durum:** kabul edildi (2026-10-31).
- **İş:** plan/architecture-advanced.md §3 `ADV-03`'ün “Açık” satırı (döner, cebe giren, katlanır kapı ve yatay kaydın
  çizimi); ADR 0772'nin üstüne.
- **Web:** web dururken (ADR 0228) yalnız masaüstü; web'e gerekenler TODOS.md §0.2'de.
- **`.kcad`:** şema 144 (opt-in; §7), `FORMATS_VERSION` 103.
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

ADR 0772 kapının üç açılışını bilir: kanatlı, sürme, sabit. Ofislerin projelerinde sık geçen üç kapı yoktur: girişlerin
döner kapısı, dar koridor ve ıslak hacimlerin cebe giren (duvarın içine kayan) kapısı, geniş açıklıkların katlanır
(akordeon) kapısı. Sürme kapının planı da eksiktir: iki kanadın ne kadar bindiği sabit (%5) ve rayı hiç çizilmez; duvarın
yüzünde rayla kayan tek kanat hiç yoktur. Cebe giren kapının asıl sorusu çizimin değil yapının sorusudur: kanat duvarın
içine sığıyor mu, cebi duvarın ucundan, köşesinden ya da öbür açıklığa taşıyor mu.

Başvurulan kaynaklar (31 Ekim 2026'da okundu):

- **2010 ADA Standards, 404.2.1 Revolving Doors, Gates, and Turnstiles:** “Revolving doors, revolving gates, and
  turnstiles shall not be part of an accessible route.” **404.2.3 Clear Width:** geçiş en az 815 mm; kanatlı kapıda
  “measured between the face of the door and the stop, with the door open 90 degrees”; Şekil 404.2.3'ün (b) açık sürme,
  (c) açık katlanır kapının net geçişini gösterir. <https://www.access-board.gov/ada/chapter/ch04/>
- **Graphisoft Archicad:** kütüphanenin cebe giren kapısı duvarın içindeki cebi kapının kendisiyle açar; “Reveal to Wall
  Face” ve cebin seçimiyle ayarlanır; kanattan kısa cep seçeneği yoktur (Graphisoft topluluğu, “Protruding Pocket Door”,
  “Multi-Panel Pocket Door stacking”). Katlanır kapı için kütüphane nesnesi zayıftır, kullanıcılar boş panelli kapı
  kullanır (“folding door representation”). <https://community.graphisoft.com/t5/Libraries-objects/Protruding-Pocket-Door/m-p/269483>,
  <https://community.graphisoft.com/t5/Libraries-objects/folding-door-representation/m-p/164387>
- **Autodesk Revit:** standart kütüphanede cebe giren kapı yoktur; ailenin açtığı boşluk (“opening cut”) yerine
  duvarda boşluk (void) çizilerek cep açılır (Autodesk forumu, “Pocket Door”); döner kapı iç içe ailelerle (kanat
  dönen gövdeye) kurulur. <https://forums.autodesk.com/t5/revit-architecture-forum/pocket-door/td-p/8286902>,
  <https://resources.imaginit.com/revit/advanced-creating-door-family-part-03>
- Üreticilerin döner kapı tamburları 1,8–3,0 m iç çaptadır; kanat sayısı 3 ya da 4'tür.

İki kaynaktan çıkan ders: cep, kapının kendi parametrelerinden duvarda açılan bir oyuktur, elle çizilen bir duvar
parçası değil; ve kimse cebin duvara sığmasını denetlemez. KentOS bunu kapının kendisinden hesaplar ve reddeder.

## Karar

### 1. Döner kapı

Açılış `revolving`, 3 ya da 4 kanat, yalnız kapı. Yapının `drum`'ı tamburun iç çapıdır; verilmezse ışık kadardır,
ışıktan büyük olamaz (`invalid_opening`, `build.drum`). Plan: ışığın ortasında, duvarın ortasında tambur (daire, r =
D/2); tamburun iki duvarı r + 3 cm yarıçaplı, duvarın doğrultusu çevresinde 360°/n genişliğinde iki yay; kanatlar
merkezden r uzunluğunda, ilki doğrultunun 90° − 180°/n ilerisinde, her biri 360°/n arayla. Her kanat tamburun
yarıçapıdır. Net geçiş tamburun boğazıdır: D·cos(180°/n) (dört kanatta D/√2, üçte D/2). Süpürme alanı tamburun
dairesidir (32 nokta, `geom::opening::swing_zones`). ADA 404.2.1 döner kapıyı erişilebilir yol saymaz;
KentOS bunu proje kuralı olarak bırakır (ofis seçer), bu dilimde ayrı uyarı yazmaz.

### 2. Cebe giren kapı

Açılış `pocket`, 1 ya da 2 kanat, yalnız kapı. Yapının `lip`'i açıkken cepten kalan paydır (öntanımlı 5 cm). Kanat
(tl kalın, öntanımlı 4 cm) menteşe yanındaki pervazdan duvarın içine kayar; iki kanat iki yana. Her cep:

- boyu: w − lip − fit − kasa + 1 cm (w kanadın eni);
- eni: tl + 2 × 1 cm boşluk, duvarın ortasında;
- duvar en az cebin eni + iki yanda 2,5 cm olmalı; değilse `opening_pocket_thin` (yol `operation`);
- cep kapıyla aynı kenarda, duvarın uçları ve köşeleri arasında olmalı; değilse `opening_pocket_short` (yol `at`);
- cebin uzunluğunda başka açıklık duramaz: `opening_pocket_short` (ret yeri kapının kendisi). Kapı ve cebi birlikte
  (`reach`) duvarın öbür açıklıklarıyla çakışma kuralına girer; açıklık izlemesi (`opening_follow`) duvar incelir ya da
  kısalırsa kapıyı `tooDeep` ya da `offWall` ile kopuk yapar.

Planda cep, duvarın içinde bacanınki gibi ortalanmış bir oyuktur (`geom::opening::holes`): kapının deliğinin o pervazında
duvar yalnız cebin iki yanında çizilir, cebin uzak ucu kendi çizgisiyle kapanır, iki yanı iç çizgidir; çok katmanlı
duvarda cebin kestiği bileşen çizgileri kesilir, bantlar düşülür. Kanat ışığın ortasında tl kalın bant(lar)dır. Net geçiş
ışık eksi her kanadın payıdır. Duvarın dönen katmanları (ADR 0771) cebin açıldığı pervazda dönmez.

### 3. Katlanır kapı

Açılış `folding`, 2–6 panel, yalnız kapı. Paneller ışık / n endedir; menteşe yanındaki pervaza (yapının `bothSides`'ı
açıksa iki yana, baş yana ⌊n/2⌋) açıldığı yüzde zikzakla toplanır: her panel doğrultu boyunca cos α, dışarı ±sin α
ilerler. `swing` verilmezse plan 30° çizer (kanatlıdaki 90° değil); `bothSides` yalnız çift panelde
(`invalid_opening`, `build.bothSides`). Net geçiş ışık eksi düz toplanmış panellerin kalınlığıdır: ışık − n·tl
(ADA Şekil 404.2.3 c). Süpürme alanı her grubun boyunca bir panel derinliğinde dikdörtgendir.

### 4. Sürme kapının bindirmesi ve rayı

`overlap` iki kanadın bindirmesidir (yalnız iki kanatlı sürme, ışıktan küçük); kanatlar (ışık + bindirme)/2 endedir,
net geçiş ışık − bir kanat. Verilmezse eski kural (%5) sürer, eski çizimler değişmez. `track` açıkken iki kanatta ışık
boyunca ortada ray çizilir; tek kanatta kanat açıldığı yüzün dışında, duvar kalınlığının dörtte biri öteye asılır ve
ray ışığın uzak kenarından menteşe yanını bir ışık geçecek kadar uzanır (duvar yüzünde kayan kapı). Sürme pencere aynı
bindirme ve rayı alır (net geçişi yoktur).

### 5. Komutlar, tipler, arayüz

- `cad.openings.create` ve `cad.openings.edit`'in `set`'i yeni açılışları, kanat sayısını ve yapının `drum`, `overlap`,
  `track`, `bothSides`, `lip` alanlarını alır; yapının alanı yalnız kendi açılışınındır (`invalid_opening`, “… yalnız
  döner açıklığındır”). Kapalı anahtar (`track: false`) yazılmaz: komut onu kaldırır, dosyada `bad_value`.
- Kanat sayısı açılışa göre: kanatlı/sürme/sabit 1–2 (eski ileti aynen), döner 3–4, cebe giren 1–2, katlanır 2–6.
- Kapı tipleri (`cad.openingtypes.edit`): beş alan tipin alanlarıdır (`OpeningTypeField`), istisna ve izleme raporu
  (applied/kept/skipped/refused) aynen çalışır; tipin değişikliği izleyen cebe giren kapıyı duvara sığdırmazsa
  `opening_type_refused` iletisi cebin nedenini söyler.
- Çizelge sütunları ve etiket yerleri: `operation` (Açılış), `leaves` (Kanat), `drum` (Tambur çapı), `pocket` (Cep
  boyu, en uzun cep); net geçiş ADR 0772'nin `clearWidth`'idir. `cad.openings.query`'nin `sizes`'ı `drum` ve
  `pockets`'i (sonda mı, boy, en) verir.
- Kapı aracı: Açılış (Ç: Türün, Kanatlı, Sürme, Sabit, Döner, Cebe giren, Katlanır), Kanat/Panel (K, açılışın aralığında),
  Tambur çapı (J), Cepten kalan pay (P), İki yana toplanır (İ), Kanat bindirmesi (S), Ray çizilir (H); hepsi komutun
  girdisidir (CMD-07), önizleme komutun planıdır, etiket tamburu, cebi ve net geçişi söyler, cebin sığmadığı yer
  tıklamadan önce söylenir. Tür verilmezse araç KentOS'un ilk kapısını gösterir ve yazar (önceden projenin ilk tipini
  gösterip KentOS'unkini yazıyordu).
- Öznitelikler: Açılış (pencerede özel açılışlar yok; seçilen açılış kanat sayısını aralığına alır), Kanat/Panel
  (aralığıyla), Tambur çapı, Cepten kalan pay ve Cep, İki yana toplanır, Kanat bindirmesi, Ray çizilir.
- Kapı ve pencere tipleri penceresi: açılışa göre kendi alanları; başka açılışa geçince eskisinin alanları kalkar,
  anahtarlar “Evet” ya da boş.
- İkonlar: `doorRevolving`, `doorPocket`, `doorFolding` (seçenek sayfasının A'ları).

### 6. Dosyalar

DXF: her kapı blok görünüşü ve KentOS verisiyle gider (ADR 0347): tambur `CIRCLE`, tamburun duvarları `ARC`, kanatlar
ve paneller `LINE`, cebe giren kanat kapalı çoklu çizgi, ray çizgi; duvar cebiyle. KentOS kapıyı verisinden geri okur;
yeni kayıp yok. PDF: pafta ve başsız kitap aynı şekilleri çizer (1:50'de tambur kâğıtta 20 mm, cebin iki yanı duvarın
içinde).

### 7. `.kcad` şema 144

Açılışın `revolving`, `pocket`, `folding` değerleri, 2'den çok kanat, yapının beş alanı, istisnada bu alanlar, projenin
tiplerinde aynısı ve çizelgenin dört sütunu şema 144'tür; yazıcı yalnız bunlardan biri olan çizimde 144 yazar.
140 ve öncesi `unknown_field` (çizelge ve istisna alanları) ya da `bad_value` ile reddeder; 141 ve 142 başka dilimlerin
numaralarıdır, bu kod tabanında okunmaz (`schema_version`). Kural ihlalleri (iki kanatlı döner, katlanır pencere,
kanatlıda tambur, `track: false`, tek sayıda iki yana panel) `bad_value`. Örnek `fixtures/kcad/v2/special-doors.json`.

## Doğrulama

- Bağımsız Python başvurusu `scripts/fixtures/special_doors_cases.py` (KentOS kodu yok; duvarlar kesirlerle, birleşimler
  `wall_join_cases.py`'den): 24 geometri durumu (`fixtures/walls/v1/special-doors.json`: pervazlar, simgeler, süpürme
  alanları, duvar çizgileri, uç çizgileri, bantlar, dolgu, cebin iç çizgileri); çekirdeğin testi
  `tests/all/openings.rs`.
- Komut durumları `scripts/fixtures/special_doors_command_cases.py`: `cad.openings.create.special` 27,
  `cad.openings.query.special` 21, `cad.openings.edit.special` 22, `cad.openingtypes.edit.special` 21 (91 durum).
- KCAD: başvuru yazıcısı ve okuyucusu (`kcad_v2_reference.py`, `tools/kcad/kcad.py`) şema 144'ü bilir; 1 geçerli,
  7 bozuk örnek; geçmiş sürüm örneği `schema-version-145.kcad`.
- Başsız: DXF'in bloklarında tambur ve duvarları, geri okunan dört kapı; 1:50 PDF'te tamburun dairesi ve cebin iki yanı
  (`pdftoppm`, `tests/headless/special_doors.rs`).
- Etkileşim: Kapı aracının döner, cebe giren, katlanır ve raylı sürme kapısı, Ç'nin döngüsü (`opening_special.rs`).
- Masaüstü: Öznitelikler'den tambur ve cep payı, tip penceresinin açılışa göre alanları; resimler `special_door_scenes`.

## Sınırlar [~]

- Döner kapının erişilebilirlik uyarısı, tamburun 3B'si ve kanatların dönüş yönü (saat yönü) yok.
- Cep yalnız düz duvar kenarında; yaylı duvarda cebin yayı ve cebin metrajdan düşülmesi açık.
- Katlanır kapının panel başına farklı eni ve ara mentesesi, sürme kapının üç ve dört kanadı (çift ray) yok.
- Döner ve katlanır kapının süpürme alanı hesaplanır ama kapıların denetimi (ADR 0772 §5) hâlâ yalnız kanatlı kapının
  kanadına bakar; tambur ve panellerin çarpışma denetimi açık.
- Gerçek pencerede elle denenmedi.
