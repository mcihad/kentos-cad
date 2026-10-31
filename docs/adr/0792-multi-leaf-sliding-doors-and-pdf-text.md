# ADR 0792: Üç ve dört kanatlı sürme; paftanın PDF metninin okunması

- **Durum:** kabul edildi (2026-10-31).
- **İş:** plan/architecture-advanced.md §3 `ADV-03`'ün “Açık” satırı (ADV-03d): üç-dört kanatlı sürme ve PDF metninin
  okunması; ADR 0772, 0777 ve 0787'nin üstüne.
- **Web:** web dururken (ADR 0228) yalnız masaüstü; web'e gerekenler aşağıda.
- **`.kcad`:** şema 150 (opt-in; §5; dalda geçici 157'ydi, birleşmede 150 oldu). **Sözleşme:** `OpeningBuild.tracks`, `OpeningTypeField::Tracks`,
  `OpeningSizes.tracks`/`panels`, `ScheduleField::Tracks`/`Panels`, etiket yerleri `tracks`, `panels`.
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

ADR 0777 sürme kapıya bindirme ve ray verdi ama kanat sayısını 1–2'de bıraktı; balkon ve teras kapılarının sık türü
olan üç ve dört kanatlı sürme (teleskop ya da ortadan iki yana açılan) çizilemiyor, çizelgede ve etikette kanatların
hangisinin sabit, hangisinin sürme olduğu söylenemiyordu. Ayrıca ADV-03'ün kabulü “PDF aynı bilgiyi versin” der: paftanın
PDF'inde açıklık numaralarının ve çizelge ölçülerinin bir okuyucuya (arama, kopyalama, erişilebilirlik) doğru göründüğü
hiç okunarak sınanmamıştı.

Başvurulan kaynaklar (31 Ekim 2026'da okundu):

- **Sektörün kanat gösterimi:** sürme kapı ve pencere yapılandırmaları dışarıdan bakılarak `X` (açılır, sürme) ve `O`
  (sabit) harfleriyle yazılır: OX, XO, OXO, XOX, OXXO (Florida Building Code ürün onayı belgelerinde “X: operable panel,
  O: fixed panel”; Loewen'in sürme kapı kataloğu OXXO, XOX'u standart sayar).
  <https://floridabuilding.org/upload/PR_Instl_Docs/FL40257_R0_II_PHT001%20SS%202021-12-13.pdf>,
  <https://www.loewen.com/wp-content/uploads/2025/08/Loewen-SlidingPatioDoors-v3c.pdf>
- **Graphisoft Archicad:** kütüphanenin “Sliding Folding Multipanel Door” nesnesi kanat sayısını ve toplanma yönünü
  parametre olarak alır; topluluk çok raylı, ortadan toplanan kapıları kütüphane nesnesinin sınırında taklitle (iki
  sürme kapıyı tek açıklıkta birleştirerek) yapar ve bunun çizelgeye geçmediğini söyler.
  <https://bimcomponents.com/GSM/Details/7395>,
  <https://community.graphisoft.com/t5/Modeling/Sliding-Stacking-Frameless-Doors/m-p/662953>
- **Autodesk Revit:** çok kanatlı sürme kapı üreticilerin ailesiyle gelir (Solar Innovations'ın “Multi-Track Sliding
  Glass Door: OXXO” ailesi; her yapılandırma ayrı aile), planda kanatların açıklık yüzdesi aile parametresidir (AUGI).
  <https://www.arcat.com/search/revit-Glass-Doors-family/bim>, <https://forums.augi.com/archive/index.php/t-2069.html>

İki üreticiden çıkan ders: yapılandırma (hangi kanadın sabit, kaç ray, hangi yana toplanır) ya aileye gömülü ya da
çizelgeye geçmeyen bir taklittir. KentOS bunu kapının kendi alanlarından hesaplar ve O/X düzenini, net geçişi ve ray
sayısını çizelgeye ve etikete verir.

## Karar

### 1. Üç ve dört kanatlı sürme

- Sürme açıklık (kapı ve pencere) 1–4 kanat alır (`leaves`); başka açılışların aralığı aynen. Ret iletisi “Sürme
  açıklığın kanat sayısı 5; 1 ile 4 arasında olmalı.”
- Yapının yeni alanı `tracks` (ray sayısı) 2 ya da 3, yalnız üç ya da dört kanatlı sürmede, yazılmazsa 2. `bothSides`
  (ADR 0777'nin katlanır kapısının alanı) dört kanatlı sürmede ortadan iki yana açılmadır; yalnız iki rayla.
- **Kanatların düzeni** (`kentos_contracts::slide_panels`, çekirdekte `geom::opening::slide_leaves`): kanatlar
  toplandıkları pervazdan (`hand`; iki yana: her pervazdan ikişer) sayılır; k'ıncı kanat k numaralı raydadır ve k < ray
  sayısıyken sürer (ilki pervazda durur, öbürleri onun üstüne toplanır), raylardan sonrakiler 0 numaralı rayda sabittir.
  Böylece üç kanat iki rayda OXO, üç rayda OXX (teleskop), dört kanat iki rayda OXOO, üç rayda OXXO (tek yana, sonuncusu
  sabit), iki yana OXXO; sona toplananlar ters okunur.
- **Ölçüler:** farklı raydaki komşular `overlap` kadar biner (yazılmazsa ışığın %5'i n − 1 buluşmaya bölünür), aynı
  raydakiler (iki yana açılanın ortadaki iki kanadı) alın alına durur; her kanat (ışık + binme sayısı · bindirme) / n
  endedir. Bindirme ışığın n'de birinden küçük olmalı (“… ışığın üçte birinden (0.767 m) küçük olmalı.”). **Net geçiş**
  sürme kanatlar toplanınca kalan en geniş boşluktur: tek yana ve sabit kanadı olmayan için ışık − bir kanat, sabit
  kanat varsa t·(w − o) − w (pervazdan ilk sabit kanada), iki yana ışık − iki kanat. Kanat kalınlığı (ADR 0777'nin iki
  kanatlısı gibi) geçişten düşülmez.
- **Plan:** raylar çerçevenin bandında eşit aralıklıdır (kapıda derinlik / (2 · ray), pencerede / (3 · ray); 0 numaralı
  ray açıldığı yüzün karşısında); her kanat kendi rayında, kalınlığı kadar (yazılmazsa 4 cm, aralığın beşte dördünü
  aşmaz) bir banttır; her sürme kanadın yanında, aralığın yarısı açıldığı yüze doğru, toplandığı pervaza bakan ok
  (gövde min(w/3, 0,3 m), başı gövdenin üçte biri); `track` açıkken her ray ışık boyunca bir çizgi. İki kanatlı ve tek
  kanatlı sürmenin çizimi bayt bayt eskisidir.
- **Çarpışma (ADR 0787):** çok kanatlı sürme kendi kasasının içinde kayar; `door_zones`'u boştur, Kapılar denetimi ona
  engel bulmaz (yalnız yüzde rayla asılı tek kanatlı kapının yolu alandır).

### 2. Komutlar, tipler, arayüz

- `cad.openings.create`, `cad.openings.edit`'in `set`'i ve `cad.openingtypes.edit` `tracks`'i ve sürmede `bothSides`'ı
  alır (yeni komut yok); `cad.openings.query`'nin `sizes`'ı `tracks` ve `panels`'ı ("OXXO") verir. Tek kanatlıda
  bindirmenin iletisi “Kanat bindirmesi çok kanatlı sürmenindir; tek kanatlıda kaldırın.” oldu (önceden “iki kanatlı”).
  Yeni retler: “Ray sayısı yalnız sürme açıklığındır …”, “Ray sayısı üç ya da dört kanatlı sürmenindir …”, “Ray sayısı
  4; 2 ya da 3 olmalı.”, “İki yana açılan dört kanat iki raydadır …”, “İki yana açılma dört kanatlı sürmenindir (3
  kanat) …” (hepsi `invalid_opening` / `invalid_opening_type`).
- Çizelge sütunları `tracks` (Ray sayısı) ve `panels` (Kanat düzeni), etiket yerleri `{tracks}`, `{panels}`.
- Kapı aracı: Sürme'de Kanat 1–4; üç ve dört kanatta Ray sayısı (V: 2/3), dört kanatta İki yana açılır (İ); bindirme iki
  ve daha çok kanatta. İmlecin yanındaki etiket “Sürme 4 kanat: OXXO · 2 ray · Net geçiş 1.498 m” der. Hepsi komutun
  girdisidir (CMD-07), önizleme komutun planıdır.
- Öznitelikler: üç ve dört kanatta Ray sayısı, İki yana açılır (dört), Kanat bindirmesi (öntanımlı değeriyle) ve Kanat
  düzeni. Kapı ve pencere tipleri penceresinde sürmede Ray sayısı ve İki yana; başka açılışa geçince kalkar.

### 3. Dosyalar

DXF: kapı blok görünüşü ve KentOS verisiyle gider (ADR 0347): kanatlar kapalı çoklu çizgi, oklar ve raylar `LINE`;
KentOS kapıyı verisinden geri okur (kanat, ray sayısı, iki yana). PDF: pafta ve başsız kitap aynı şekilleri çizer.

### 4. Paftanın PDF metninin okunması

Başsız sunucuda 1:50 A3 paftası (çizimin kapıları ve penceresi, etiketleri, açıklık çizelgesi), poppler'ın
`pdftotext -bbox`'ıyla okunur (`tests/headless/multi_slide.rs`):

- her açıklığın numarası iki kez geçer: kâğıtta etiketinin noktasının 3 mm içinde bir kez, çizelgenin kendi satırında bir
  kez; satırda kaba boşluk, net geçiş (kapıda), kanat, ray sayısı ve kanat düzeni sırasıyla ve çizimin
  `sizes()`'ındaki değerlerle;
- etiketin ve çizelgenin yazıları kâğıtta 2,5 mm (poppler'ın kutusu 3,0 mm, satır yüksekliği);
- görünümün alanına kırpan bağlı haritada (ADR 0781 §2, ADR 0789 §3) kırpmanın dışında kalan etiket dosyada yoktur, her
  sözcük kırpmanın içindedir;
- 1:100 haritada etiket kâğıt boyunu korur (2,5 mm), çizimdeki çizelge (yüksekliği projenin 1:50'sinden metre) yarı
  boyda basılır: paftanın ölçeği çizimdeki tabloyu büyütmez (bilinen davranış, test onu sabitler).

Okumada yeni bir hata bulunmadı: kırpmanın dışındaki yazının dosyaya yazılması ADR 0789 ile main'de düzeltilmişti; bu
test onu etiketler ve çizelgeyle de sabitler. Çok satırlı etiket (“{number}\n{clearWidth}\n{panels}”) de satır satır,
pencerede boş satırı atlanarak okundu.

### 5. `.kcad` şema 150

Sürmede 3–4 kanat, yapının `tracks`'i, dört kanatlı sürmede `bothSides`, istisnada `tracks`, projenin tiplerinde
aynısı ve çizelgenin `tracks`, `panels` sütunları şema 150'dir; yazıcı yalnız bunlardan biri olan çizimde 150 yazar.
148 ve öncesi `unknown_field`; kuralı kıran (beş kanat, iki kanatta ray sayısı, dört ray, iki yana üç ray ya da üç
kanat) `bad_value`. **Numara geçicidir:** 150–156 başka dilimlerindir ve bu kod tabanında okunmaz; şema testinin
“bir önceki şemada ret” kuralı için `broken/multi-slide-in-schema-156.kcad` `schema_version` ile reddedilir, bir
sonraki sürüm örneği `schema-version-158.kcad`. Birleştirmede numara sıradakine çekilirken bu iki örnek de yeniden
yazılır. Örnek `fixtures/kcad/v2/multi-slide.json` (tipini izleyen üç raylı OXX kapı, iki yana açılan raylı OXXO kapı,
iki raylı OXO pencere); bağımsız Python yazıcısı (`kcad_v2_reference.py`) ve okuyucusu (`tools/kcad/kcad.py`) şemayı
bilir.

## Doğrulama

- Bağımsız Python başvuruları (KentOS kodu yok):
  - `scripts/fixtures/special_doors_cases.py` → `fixtures/walls/v1/multi-slide.json`, 22 geometri durumu (üç ve dört
    kanat, iki ve üç ray, iki yana, sona, sağa, bindirmesiz, kasalı ve montaj boşluklu, dar açıklık, eğik ve katmanlı
    duvar, oda duvarı, yanında kapı, pencere); çekirdeğin `multi_leaf_sliding_doors_draw_as_the_reference_says` ve
    `a_multi_leaf_sliding_door_takes_no_room_in_front_of_its_wall` testleri.
  - `scripts/fixtures/special_doors_command_cases.py` → `cad.openings.create.multislide.json` 23,
    `cad.openings.query.multislide.json` 20, `cad.openings.edit.multislide.json` 22,
    `cad.openingtypes.edit.multislide.json` 21 durum (92); ADR 0777'nin dosyalarında yalnız değişen iletiler ve
    “KT1 sabit olunca” durumu (dört kanatlı sürme artık geçerli).
- KCAD: 1 geçerli, 8 ret örneği; `kentos-kcad`'in bütün testleri, `kentos-application`'ın taşıma testi, Python'un
  `test_carriage` ve ortak komut durumları.
- Etkileşim: `a_multi_leaf_sliding_door_takes_its_tracks_and_parts_both_ways` (V, İ, iki kanatta düşen alanlar).
- Başsız: DXF'e gidip gelen üç sürme, PDF metninin dört testi (`tests/headless/multi_slide.rs`).
- Masaüstü resimleri `multi_slide_scenes.rs` (iki tema, 1440×900 ve 1100×650): `surme-cok-plan`, `surme-cok-yakin`,
  `surme-cok-arac`, `surme-cok-oznitelikler`, `surme-cok-tipler`.

## Sınırlar [~]

- XOX (iki dış kanadın ortadaki sabit kanada toplanması) ve dört raylı teleskop yok; ortadaki sabit kanat ancak OXO
  biçiminde.
- Kanatların eşit olmayan enleri ve çok kanatlı sürmenin cebe giren biçimi (ADR 0777'nin cebi) yok.
- Kanat kalınlığı net geçişten düşülmez (ADR 0777'nin iki kanatlısıyla tutarlı).
- Pencere aracı çok kanatlı sürmeyi tipten alır; aracın Açılış seçeneği yalnız kapıda.
- PDF okuma A3 1:50 ve 1:100'de, döndürülmemiş haritada sınandı; döndürülmüş harita ve kesik çizgili yazılar sınanmadı.
- Gerçek pencerede elle denenmedi; resimler pencere açmadan.

## Belgelere eklenecek satırlar

- **CLAUDE.md §0:** “**Üç ve dört kanatlı sürme, paftanın PDF metni (31 Ekim 2026; [ADR 0792](docs/adr/0792-multi-leaf-sliding-doors-and-pdf-text.md), ADV-03d [~]).**
  Sürme kapı ve pencere 1–4 kanat; yapının `tracks`'i (2/3, yalnız 3–4 kanatta) ve dört kanatta `bothSides` (ortadan iki
  yana). Kanatlar toplandıkları pervazdan k'ıncı rayda, raylardan sonrakiler sabit (`slide_panels`, `geom::opening::slide_leaves`):
  OXO, OXX, OXOO, OXXO; komşu raylar `overlap` kadar biner, net geçiş toplanınca kalan en geniş boşluk. Planda kanat bantları,
  toplandığı pervaza oklar, raylar; `door_zones` boş. `sizes`'ın `tracks`, `panels`'ı, çizelge Ray sayısı ve Kanat düzeni,
  etiket `{tracks}`, `{panels}`; Kapı aracında Ray sayısı (V) ve İki yana (İ), Öznitelikler ve tip penceresinde aynısı.
  `.kcad` şema 150. 1:50 paftanın PDF'i `pdftotext -bbox` ile okunur: numara etiketinde ve satırında birer kez,
  2,5 mm, kırpmanın dışında yazı yok. 22 + 23 + 20 + 22 + 21 ortak durum (`special_doors_cases.py`,
  `special_doors_command_cases.py`). XOX, eşit olmayan kanat ve gerçek pencere kabulü açık.”
- **CLAUDE.md §2 (komutlar):** `python3 scripts/fixtures/special_doors_cases.py --check; python3 scripts/fixtures/special_doors_command_cases.py --check; cargo test -p kentos-geometry-core --test all openings; KENTOS_CASES_ONLY=multislide,special cargo test -p kentos-native-application --test all every_command_case; cargo test -p kentos-headless --test headless multi_slide; cargo test -p kentos-interaction --test all opening_special; KENTOS_SHOTS_ONLY=surme-cok-plan,surme-cok-yakin,surme-cok-arac,surme-cok-oznitelikler,surme-cok-tipler cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture   # ADR 0792`
- **plan/architecture-advanced.md ADV-03 durum satırı:** “… yaylı duvarda cep ve cebin metrajdan düşülmesi
  ([ADR 0787](../docs/adr/0787-special-door-clashes-arc-pockets-pocket-quantities.md)); üç ve dört kanatlı sürme ve
  paftanın PDF metninin okunması ([ADR 0792](../docs/adr/0792-multi-leaf-sliding-doors-and-pdf-text.md)). Açık: bağımsız
  bölüm kapsamı (ADV-04), XOX ve eşit olmayan sürme kanatları, gerçek pencerede elle kabul.”
- **TODOS.md §0.2 (Web'e ertelenenler):** “ADR 0792: sürmenin 3–4 kanadı, `tracks` ve `bothSides`'ı, kanat bantları ve
  oklar, çizelgenin Ray sayısı ve Kanat düzeni sütunları, Kapı aracının V ve İ seçenekleri; `.kcad` şema 150'nin web
  okuyucusu.”
