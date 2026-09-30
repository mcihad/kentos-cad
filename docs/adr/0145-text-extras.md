# ADR 0145: Yazı ekleri

- **Durum:** kabul edildi (2026-09-30). Yön sahibin kararıdır (biçim değişiklikleri özellikleriyle; sıra köşe kotu, çok parçalı alan, blok, yazı ekleri, lider, yeni ölçü türleri). Ayrıntılar bu ADR'nin varsayılanlarıdır.
- **Tarih:** 2026-09-30
- **Bağlam belgesi:** ADR 0025 (KCAD v2), ADR 0144 (şema 6: blok, §7 öznitelik tanımları), ADR 0055 (çizimin yazıları), ADR 0060 (Yazı aracı ve yerinde düzenleme), ADR 0066 (Öznitelikler), ADR 0009 ve 0138 (DXF, NCZ); `docs/specs/kcad-v2.md` §6.1, §6.6, §6.9; PiriCAD `netcad_plan.md` N-12.

## Bağlam

Netcad'in Yazı komutu boy ve açının yanında sıkışma (genişlik çarpanı), fon (yazının arkasında zemin), `++` ile ardışık artırma ve uygulama noktası (yazının hangi noktasından yerleştiği) sorar. Ayrıca metin dosyası yükleme, ters okunan yazıları çevirme (MAKE TEXTS READABLE) ve jokerli bul-değiştir vardır.

KentOS'un yazısı bugün `p`, `text`, `height`, `rotation`'dır: tek satır, sol alt noktasından (taban çizgisinin solu) yerleşir. Bunun bedeli:

- **Hizalı yazı kayar.** DXF'in ortalı ya da sağa yaslı TEXT'i, ATTDEF'i ve MTEXT'in satırları, NCZ'nin ortalı yazıları okunurken sol alt noktaya çevrilir. Çeviri yazının tahmini genişliğini kullanır (“Hizalı yazı … genişliği tahmin edilerek hesaplandı”). Yazı tipi değişince ya da tahmin gerçek genişlikten ayrılınca yazı hizasından kayar. DXF'e geri yazılan dosya hizayı kaybeder.
- **Genişlik çarpanı düşer.** DXF'in 41 grubu yalnız tahmin için okunur; sıkıştırılmış yazı KentOS'ta tam genişlikte görünür.
- **Zemin yok.** Çizginin üstündeki yazı okunmaz; kroki ve paftada yazının altı boşaltılamaz.

Sahibin sırası (29 Eylül, ADR 0143 Bağlam): bloktan sonra yazı ekleri. Her biri `.kcad` şemasını değiştiren kendi özelliği ve şema adımıyla gelir.

## Karar

### 1. Veri

Yazıya (`TextEntity`) üç isteğe bağlı alan eklenir. Hiçbiri yoksa yazı bugünkü yazıdır.

- **`align` — hiza:** `p`'nin yazının hangi noktası olduğu. Yatayda sol, orta, sağ; düşeyde taban çizgisi, alt, orta, üst. Yokken sol taban çizgisidir (bugünkü). Numaralı metin, on bir değer:
  - taban çizgisinde `baselineCenter`, `baselineRight`;
  - `bottomLeft`, `bottomCenter`, `bottomRight`;
  - `middleLeft`, `middleCenter`, `middleRight`;
  - `topLeft`, `topCenter`, `topRight`.
  - Sol taban çizgisi bir değer değildir, alanın yokluğudur (yerleştirmenin `mirror`'ı gibi). Böylece bir yazının tek bir yazılışı olur.
- **Düşey konumlar** yazının yüksekliği `h` cinsinden, taban çizgisine göre: alt −0,2h, orta 0,5h, üst h. Bunlar DXF okuyucusunun bugün kullandığı değerlerdir (`text_frame`); yeni bir ölçü icat edilmez.
- **`widthFactor` — genişlik çarpanı** (Netcad'in “sıkışma”sı, DXF'in 41'i): harflerin eni bununla çarpılır, yükseklik değişmez. Sonlu, 0'dan büyük, en çok 100. Yokken 1'dir; üreticiler 1'i yazmaz.
- **`mask` — zemin** (Netcad'in “fon”u): yazının kutusu, yazıdan önce çizim alanının zemin rengiyle doldurulur. Altındaki çizgi ve tarama ekranda ve çıktıda görünmez. Yalnız `true` yazılır.
- **Öznitelik tanımı** (`AttributeDefinition`, ADR 0144 §7) de `align` ve `widthFactor` alır. DXF'in hizalı ATTDEF'i böylece tahminsiz okunur. §7'nin “yazı yerleşimi nokta, yükseklik, açı ve hizadır” sözü bununla karşılanır.
- **Adlar:** sözleşmede (`contracts::entity`) `align` (`TextAlign`), `widthFactor`, `mask`; TypeScript'te aynı adlar.

### 2. `.kcad`: belge şeması 7

- **Yeni alanlar:**
  - `text`: `align` (numaralı metin), `widthFactor` (float), `mask` (bool);
  - öznitelik tanımı: `align`, `widthFactor`.
- **Yazıcı ne zaman 7 yazar:** yalnız bir yazıda ya da öznitelik tanımında bu alanlardan biri varken. Öbür çizimler şema 6 ya da öncesi olarak kalır, eskisiyle bayt bayt aynıdır.
- **Okuyucu reddi:**
  - şema 6 ve öncesinde bu alanlar bilinmeyen alandır (`unknown_field`);
  - bilinmeyen hiza `bad_value`'dur;
  - 0 ya da eksi, 100'den büyük `widthFactor` `bad_value`, sonlu olmayan `non_finite`'tir;
  - `mask: false` `bad_value`'dur.
- **Birlikte değişenler (ADR 0025'in kuralı):**
  - spesifikasyon §6.1, §6.6, §6.9;
  - Rust kodeği ve tipli sütunlar (`io/columns.ts` ↔ `kcad/src/columns.rs`, `FORMATS_VERSION`);
  - bağımsız Python okuyucusu ve yazıcısı;
  - yeni örnek dosyalar (`fixtures/kcad/v2`): hizalı, sıkıştırılmış ve zeminli yazılar, hizalı öznitelik tanımı; şema 6'da `align` reddi ve bozuk değerler.
- **Sunucu:** veritabanı projesi yazıyı `cad_definition` olarak saklar (§15). Yeni alanlar orada taşınır; izdüşüm noktadır ve değişmez.

### 3. Hesap (ortak çekirdek)

- **Yazının kutusu:** genişlik projenin yazı tipiyle ölçülür ve çarpanla çarpılır (`text::width_em` × `h` × `widthFactor`). Kutu hizaya göre kayar: çapa `p`'dir, sol taban çizgisi kutunun hizaya göre hesaplanan köşesidir. Seçme, pencere seçimi, kapsam ve Öznitelikler'in kutusu aynı kutuyu kullanır (`entity::text_box`).
- **Kenet:** yazının ekleme noktası kenedi `p`'dir, yani hizanın noktası (AutoCAD'deki gibi).
- **Dönüşümler:**
  - taşıma, döndürme ve ölçekleme `p`'yi taşır; dönüş eklenir, yükseklik ölçeklenir;
  - hiza, genişlik çarpanı ve zemin korunur;
  - aynalamada bugünkü kural sürer (çapa aynalanır, yazı okunur kalır: dönüşe 180° eklenir), hiza korunur.
- **Blok:** tanımın yazıları ve öznitelik tanımları açılırken hizalarını, çarpanlarını ve zeminlerini taşır. Patlat onları yazı nesnesine geçirir.
- **Okunur yap:** ters okunan yazı (dönüşü 90°'den büyük, en çok 270°) kutusunun ortası çevresinde 180° döner. Kutu yerinde kalır, hiza korunur: yeni çapa, hizanın döndürülmüş kutudaki noktasıdır. Çekirdekte tek bir işlev, iki platform ve bağımsız hesapla sınanır.
- **Ardışık artırma:** yazının sonundaki sayı bir artar, sıfır dolgusu korunur. `101` → `102`, `101/12` → `101/13`, `P9` → `P10`, `A-009` → `A-010`. Sonu sayı olmayan yazı değişmez. Kural çekirdekte, ortak durumlarla (`fixtures/text/v1/increment.json`).
- **Joker eşleme:** bul-değiştirin kalıbında `*` herhangi bir dizidir (önek, sonek, iç; birden çok olabilir). Büyük küçük harf ayrımı isteğe bağlıdır, Türkçe harf katlamasıyla (`I`/`ı`, `İ`/`i`). Değiştirme metni kalıbın yakaladığı parçaları yerinde bırakır: `*` ile yakalanan her parça, değiştirme metnindeki sırasıyla aynı `*`'a konur. Kural çekirdekte, ortak durumlarla (`fixtures/text/v1/pattern.json`).

### 4. Çizim

- **Tek ölçü.** Hizanın kaydırdığı yer, seçme kutusu ve zemin aynı genişlikten gelir: çekirdeğin, projenin yazı tipindeki harf genişlikleri çarpı çarpan (`TextPlace::width`; tablolar yazıların çizildiği yerde, Chrome'da paketli yazı tipleriyle ölçülmüştür). Depo yazının etiket kaydında taban çizgisinin başladığı noktayı (çapa, hizanın payı kadar geri), çarpanı ve zeminin genişliğini verir (`LABEL_STRIDE` 9). İki çizici (web `viewport/overlay.ts`, masaüstü `labels.rs`) yazıyı her zamanki gibi o noktadan çizer. Böylece iki platform aynı yere çizer, seçim kutusu çizilenle örtüşür. Kerning tablolarda yoktur; ortalı yazının görünen ortası bu kadar kayabilir.
- **Genişlik çarpanı** yazının kendi çerçevesinde yatay ölçektir: web'de `scale`, masaüstünde glif ana hatları (düz yazının glif önbelleği ölçeklenmez; çarpanlı yazı ana hatlarla çizilir).
- **Zemin** yazıdan hemen önce çizim alanının zemin rengiyle doldurulan kutudur: taban çizgisinin başından yazının genişliği boyunca, yüksekliğin 1,15'i üstte ve 0,23'ü altta, her yandan 0,1 yükseklik payla (`TextPlace::mask`). Yazılar sahnenin üstünde çizildiğinden zemin altındaki bütün çizgiyi ve taramayı örter.
- **Yerinde düzenleme** (ADR 0060): alan yazının çapasında, hizasının payı kadar kayarak açılır (web'de dönük ve çarpan kadar geniş; masaüstünde Iced'in alanı dönmez, çapaya göre hizalanır).

### 5. Komutlar

- `cad.entities.create`'in yazısı ve `cad.entities.edit`'in `properties` işlemi üç alanı alır: verilir, değiştirilir ya da kaldırılır. Varsayılan değer alanın kaldırılmasıdır: sol taban çizgisi, 1 ve zeminsiz.
- `cad.entities.edit`'e **`readable`** işlemi (Okunur yap): seçili yazıların ters okunanlarını çevirir. Hiçbiri ters değilse bir şey yazmaz ve bunu söyler. Adım “Okunur yap”.
- **Bul ve değiştir** yeni bir komut değildir. Pencere eşleşmeleri çekirdekten bulur, değişen yazıları `cad.entities.edit`'in `properties` işlemiyle tek adımda (“Bul ve değiştir”) yazar. Kilitli katmandaki yazı listelenir ama değiştirilmez ve söylenir.
- **Ortak durumlar:** bağımsız Python üreticisiyle (`create`, `edit` ve yeni `readable`), üç koşucuda (web, masaüstü, Python SDK).

### 6. Araçlar ve arayüz

- **Yazı aracı:** seçeneklerine Hiza, Genişlik ve Zemin eklenir. Hiza üç sütun dört satırlık nokta seçicisidir, komut satırında adıyla yazılır. Genişlik çarpan, Zemin açık ya da kapalıdır.
- **Artır** seçeneği açıkken bir sonraki yazı kutusu, son yazının artırılmışıyla dolu açılır. Seçenekler oturumca hatırlanır (masaüstünde `Memory`).
- **Öznitelikler:** yazının satırlarına Hiza, Genişlik çarpanı ve Zemin eklenir. Çoklu seçimde ortak değer, farklıysa “Çeşitli” gösterilir.
- **Okunur yap:** Değiştir'de, seçimden önce seçen araç. Takma adlar `OKUNURYAP`, `MAKEREADABLE`, `OKY`.
- **Bul ve değiştir:** Düzen menüsünde ve şeridin Giriş sekmesinde; takma adlar `BUL`, `FIND`, `BULDEGISTIR`. Penceresinde:
  - Bul, Değiştir; joker, büyük küçük harf ve tam sözcük seçenekleri;
  - kapsam: seçim ya da bütün çizim;
  - eşleşmeler tablosu (katman, eski ve yeni metin), satırdan nesneye yakınlaştırma;
  - Seçili satırları değiştir ve Hepsini değiştir.
  - Bu adımda yalnız yazı nesneleri aranır; etiket ve öznitelik değeri sonraki bir karardır.
- **Metin dosyası yerleştir** (`METINDOSYASI`, `PLACETEXTFILE`, `MTD`): bir UTF-8 metin dosyası seçilir ve bir nokta alınır. Her satır, Yazı aracının seçenekleriyle bir yazı olur, satırlar alt alta yazı yüksekliğinin 1,5 katı aralıkla dizilir. Tek adımda yazılır.
  - Bozuk UTF-8, 1 MB'tan büyük dosya ve 10 000'den çok satır reddedilir, nedeni söylenir.
  - Boş satır yazı olmaz ama yerini tutar.

### 7. Değişim biçimleri

- **DXF okuma:**
  - TEXT'in 72/73'ü hizadır; hizalı yazının çapası 11 noktasıdır, genişlik tahmini ve “Hizalı yazı” notu kalkar.
  - 41 genişlik çarpanıdır (1 değilse).
  - 72 = 4 (“Middle”) `middleCenter` olur.
  - 72 = 3 (hizalı) ve 5 (sığdır) iki noktanın arasına yerleşir: sığdır genişlik çarpanını, hizalı yüksekliği tahmini genişlikten hesaplar; söylenir.
  - ATTDEF ve ATTRIB aynı kuralla okunur.
  - MTEXT'in yerleşim noktası (71) satırlarının hizasıdır; satırlar yine ayrı yazılar olur.
  - MTEXT'in zemin dolgusu (90) zemin olur.
- **DXF yazma:**
  - hiza 72/73 ve 11 noktasıyla, genişlik çarpanı 41 olarak yazılır; ATTDEF ve ATTRIB de öyle;
  - TEXT'in zemini yoktur: zemin KentOS verisidir (`mask`), başka programlar göstermez, KentOS geri okur; dışa aktarma penceresi ve rapor bunu söyler.
- **NCZ okuma:** Netcad yazısının çapası (orta, sağ, üst …) hizadır; tahmin ve “ortalı yazı” sayımı kalkar.
- **GeoJSON ve Shapefile** yazı taşımaz, değişmez.

### 8. Kapsam dışı

- **Çok satırlı yazı nesnesi** (paragraf, satır aralığı, satır içi biçim): ayrı karardır. Bugün her satır ayrı yazıdır.
- **Renkli zemin**, zeminin payı ve yazı stili tablosu: sonraki karar.
- **Ölçü yazısının hizası ve zemini:** yeni ölçü türlerinin adımında.
- **Etiketlerde ve öznitelik değerlerinde bul-değiştir:** sonraki karar.

### 9. İş sırası

1. **Sözleşme ve `.kcad` şema 7.** Alanlar, tipli sütunlar, belirtim, Python okuyucu ve yazıcı, örnekler; iki belge alanları taşır. *(30 Eylül: tamam. Sözleşmede `TextAlign` (on bir değer, `ALL`, `name`, `from_name`, `along`, `up`), `TextEntity.align`, `widthFactor`, `mask`, `AttributeDefinition.align`, `widthFactor`, `MAX_WIDTH_FACTOR` ve `width_factor_ok`. Kodek şema 7'yi yalnız bir yazıda (belgenin ya da bir tanımın) ya da öznitelik tanımında bu alanlardan biri varken yazar; öbür çizimlerin baytları değişmedi (bütün eski örnekler aynı). Okuyucu şema 6 ve öncesinde alanları bilinmeyen alan sayar, `baselineLeft` ve bilinmeyen hizayı, 0 ya da eksi ve 100'den büyük çarpanı, `mask: false`'u `bad_value`, sonlu olmayan çarpanı `non_finite` olarak, yeriyle reddeder; yazıcı çarpanı aynı kuralla reddeder. Tipli sütunlarda yazının isteğe bağlı alanları (hiza tam sayı, çarpan float, zemin bayrak; `FORMATS_VERSION` 16); öznitelik tanımının alanları başın JSON'unda. Bağımsız Python yazıcısı ve okuyucusu: `texts.kcad` (on bir hizadan dördü, 0,6 ve 1,5 çarpan, zeminli yazılar, tanımda hizalı yazı ve öznitelik tanımları) ve dokuz bozuk örnek; reddedilen şema artık 8. Web açılışta aynı kuralları denetler (`snapshot.ts`), sütunları aynı düzenle yazar ve okur; iki belge alanları taşır (`document-ops` senaryosu); sunucu yazının ve öznitelik tanımının çarpanını denetler, yazıyı alanlarıyla saklar. Blok öznitelikleri tablosu tanımın hizasını ve çarpanını (hücreleri yok) satırda tutar: `fixtures/blocks/v1/attribute-table.json`'a durum eklendi, iki platform geçer. Python SDK'sı `TextAlign`'ı alır. Çizim, komutlar ve araçlar sonraki adımlardır: bu adımda yeni alanlar yalnız taşınır.)*
2. **Çekirdek ve çizim.** Kutu, seçme, kenet, dönüşümler, blok parçaları; iki çizicide hiza, çarpan ve zemin; yerinde düzenleme. Okunur yap, artırma ve joker eşleme işlevleri ortak durumlarıyla. *(30 Eylül, 2a: çekirdek ve çizim tamam. Çekirdeğin yazı şekli hizayı, çarpanı ve zemini taşır (`Shape::Text`, JSON'u sözleşmeninki); `TextPlace` genişliği, taban çizgisinin başını, kutuyu ve zemin kutusunu verir: seçme, pencere ve çit seçimi, kapsam ve kutu hizalı yazıyı çizildiği yerde bulur, kenet çapadadır. Dönüşümler, tutamaç, esnetme ve bölme alanları korur; blok tanımının yazıları ve öznitelik tanımları (`block::Attribute`'un hizası ve çarpanı) parçalara, Patlat'a ve biçimlerin açılımına geçer. Depo paketinde yazı hiza, çarpan ve zemin sayılarını alır (`fixtures/store-records/v1`'e yeni yazı, iki paketleyici); etiket kaydı 9 sayıdır: yazınınki başlangıç noktası, çarpan ve zemin genişliği, blok parçasınınki de (kayıtlı depo yanıtları `store-v1.json` bu düzene çevrildi, sayılar aynı). İki çizici çarpanı ve zemini çizer; iki platformun yerinde düzenleyicisi hizaya göre açılır. Doğrulama: çekirdekte el hesabıyla 90°'de sağ üst hizalı yazının başlangıcı, kaydı ve seçilmesi; paket gidiş-dönüşü; iki platformda `fixtures/interaction/v1/text-extras.kcad` sahnesinin resimleri (on iki hiza, 30° dönük orta, 0,6 ve 1,5 çarpan, tarama ve çizginin üstünde zeminli ve zeminsiz yazı; masaüstü `labels::text_extras_screens`, web `shots.mjs texts`). Bilinen ara durum: komutların geometrisi (`EntityGeometry::Text`) yeni alanları 3. adıma dek taşımaz; yazıyı `cad.entities.edit` ile yazan her şey (Öznitelikler, yerinde düzenleme, tutamaç) şimdilik hizayı, çarpanı ve zemini düşürür, `cad.entities.transform` korur. Okunur yap, artırma ve joker eşleme 2b'dedir.)* *(30 Eylül, 2b: Okunur yap, artırma ve joker eşleme çekirdekte (`text::edit::increment`, `replace`, `Find`; `TextPlace::readable`), web'e üç işlemle (`textIncrement`, `textReplace` birçok yazıyı tek çağrıda, `textReadable`; cepheler `model/textEdit.ts`). Kurallar bağımsız Python başvurusundan `fixtures/text/v1`'de (`increment`, `pattern`, `readable`; 15, 21 ve 13 durum): jokerli eşlemede her `*` soldan en kısasını alır, kalıbın sabit parçaları en soldaki yerlerinde aranır (bir `*` her parçayı izlediğinden daha sağdaki yer kalanı eşleştiremeyen yerde de eşleştiremez); okunur yapmanın kaydırması kapalı biçimde, genişlik verilerek sınanır. Rust çağrı tablosundan ve web WASM'dan bütün durumları geçer. Araçlar ve komutlar 3. ve 4. adımlardadır.)*
3. **Komutlar.** `create` ve `edit`'in alanları, `readable` işlemi; ortak durumlar üç koşucuda. *(30 Eylül: tamam. Komutların yazı geometrisi (`EntityGeometry::Text`) hizayı, genişlik çarpanını ve zemini alır; verilmeyen alan kalkar, varsayılanlar alan olarak yazılmaz (1 çarpan ve zeminsizlik). Çarpan 0'dan büyük ve en çok 100 değilse `invalid_width_factor`, alanın yeriyle; sonlu olmayan çarpan önce `not_finite`. `cad.entities.edit`'in `readable` işlemi verilen geometriyi “Okunur yap” adımında yazar; hangi yazının ters okunduğunu ve nasıl döneceğini araç çekirdekten bulur (4. adım). Ortak durumlar bağımsız Python üreticilerinden: `cad.entities.create`'e iki (38), `cad.entities.edit`'e üç (76; okunur yapılan geometri `fixtures/text/v1`'in kuralından). Web, masaüstü ve Python SDK'sı geçer. 2a'nın ara durumu kapandı: Öznitelikler, yerinde düzenleme ve tutamaç yazının hizasını, çarpanını ve zeminini korur (masaüstünde `a_texts_extras_stay_when_its_words_change`, web'de `setGeometry` nesnenin bütün geometrisini yazar).)*
4. **Araçlar ve arayüz.** Yazı aracının seçenekleri ve Artır, Öznitelikler, Okunur yap, Bul ve değiştir, Metin dosyası yerleştir; iki platformda izler ve resimler. *(30 Eylül, 4a: Yazı'nın seçenekleri ve Öznitelikler tamam. Yazı'nın istemi `Hiza (H)`, `Genişlik (G)`, `Zemin (Z)`, `Artır (R)` alır; değerleri oturumca kalır (web'de aracın statik alanları, masaüstünde `Memory`). Hiza'nın nokta seçicisi seçeneğin çipinden açılan menüdür: dört satır (üst, orta, alt, taban) üçer nokta, her satırın ikonu yazının kutusunda o noktayı gösterir (`textAlign…` ikonları, web'in kümesinden envanterle masaüstüne); menü sağ tık menüsünde alt menüdür. Bir seçeneğin menüsü genel bir yoldur (web `Tool.optionChoices`, `chooseOption`; masaüstü `Tool::option_choices`, `choose_option`, `Message::PromptChoice`). Hiza adıyla da yazılır; Boşluk komut satırında onay olduğundan ad bitişik ya da tireyle yazılır, Türkçe imler isteğe bağlıdır (`sagust`, `sağ-üst`, `orta`). Genişlik 0'dan büyük, en çok 100; dışındaki söylenir, adım kalır. Artır açıkken sonraki kutu aracın bu çalıştırmasında yazılan son yazının artırılmışıyla, seçili açılır (sonu sayı olmayan aynen). Kutu yazının hizasına ve çarpanına göre açılır; önizleme kutusu hizaya göre yerleşir, noktası işaretlidir. Öznitelikler'de Hiza (ikonlu açılır liste), Genişlik çarpanı ve Zemin; çoklu seçimde “Yazı” ya da “Yazılar (n)” bölümü, farklıysa Çeşitli; hepsi tek “Değiştir” adımı. Hiza değişince yazı yerinde kalır, noktası kutunun yeni hizadaki noktasına geçer (AutoCAD'in JUSTIFYTEXT'i gibi; çekirdekte `TextPlace::realigned`, web'e `textRealign`, bağımsız başvuruyla `fixtures/text/v1/realign.json`, 7 durum). Altı seçenek dar komut satırına sığmadığından iki platformda genel bir kural geldi: satıra sığmayan seçenekler sondan başlayarak “Diğer” çipinin menüsüne geçer, adımın ilk harfleri kalır (web `CommandLine.fit`, masaüstü KentOS UI `Prompt::fit`, komut satırı kendi genişliğiyle kurulur). Doğrulama: iki platformda `fixtures/interaction/v1/text-options.json` izi (üç varyant), birim testleri (web `textTool.test.ts`, `textRows.test.ts`; masaüstü `tests/text.rs`, `properties/tests.rs`, KentOS UI `command_line`, `context_menu`), resimler (web `shots.mjs texts`, masaüstü `tools_screens` `yazi-*`). Okunur yap, Bul ve değiştir ve Metin dosyası yerleştir 4b–4d'dedir.)* *(30 Eylül, 4b: Okunur yap tamam. Değiştir › Nesne'de, Birleştir ve Patlat'ın tabanında (web `SelectionActionTool`, masaüstü `ObjectAction::readable`): seçimle hemen çalışır, seçim yokken önce seçtirir; seçimin ters okunan yazıları çekirdeğin `TextPlace::readable`'ıyla projenin yazı tipinde ölçülerek yarım döner, kutu yerinde kalır, hiza, çarpan ve zemin korunur; okunan yazı ve yazı olmayan nesne kalır, kilitli katmandaki atlanır ve söylenir; ters okunan yoksa bunu söyler ve yazmaz; tek adım “Okunur yap” (`cad.entities.edit`'in `readable`'ı). İkonu dönen okun içinde T. Doğrulama: iki platformda `fixtures/interaction/v1/readable.json` (`texts.kcad` üstünde; ortalı hizalar genişlikten bağımsız döner, beklenenler elle hesaplı; oynatıcılara yazının dönüşü beklentisi eklendi), birim testleri (web `readableTool.test.ts`, masaüstü `tests/text.rs`), resimler (web `readable-*`, masaüstü `okunur-yap-*`).)* *(30 Eylül, 4c: Bul ve değiştir tamam. Web'de Düzen menüsünde ve Giriş › Açıklama'da (`text.findReplace`), masaüstünde şeritte envanterden; takma adlar `BUL`, `FIND`, `BULDEGISTIR`. Pencerede Bul ve Değiştir; Joker (*), Büyük küçük harf eşleşsin (kapalıyken Türkçe katlama), Tam sözcük (joker açıkken geçmez), Yalnız seçimde (seçim varken açık gelir). Eşleşmeler her tuşta çekirdekten bulunur (web bütün yazılar için tek `textReplace` çağrısı, masaüstü `text::edit::replace`), sanal listede katman, metin ve yeni metinle; kilitli katmandaki ve boş kalacak yazı listelenir ama değiştirilmez ve söylenir; yeni metin kırpılır; satırın sözlerine tıklamak yazıyı seçip ona gider; Seçilenleri değiştir işaretli satırları, Hepsini değiştir hepsini tek adımda yazar, pencere açık kalır. §5 “properties işlemiyle” der; adım pencerenin adını taşısın diye `cad.entities.edit`'e kuralları `properties`'inki olan `replaceText` işlemi eklendi (ortak durum, edit 77). Ortak mantık masaüstünde `kentos_interaction::find_replace`, web'de `ui/text/findReplace.ts`. Doğrulama: iki platformda `fixtures/interaction/v1/find-replace.json` (üç varyant; masaüstü oynatıcısı pencereyi `find_replace_control` ile yanıtlar), birim testleri (web `findReplace.test.ts`, masaüstü `tests/find_replace.rs`), resimler (web `find-replace`, masaüstü `bul-degistir`; 1100×650'de kaydırmasız). Etiket ve öznitelik değerlerinde arama §8'deki gibi sonraki karardır.)*
5. **Biçimler.** DXF okuma ve yazma, NCZ okuma; örnek dosyalar ve bağımsız denetim.

Her adım iki platformda, ortak fixture'larla, kendi commit'inde ilerler.

## Sonuçlar

- Hizalı yazı DXF'ten ve NCZ'den tahminsiz gelir, DXF'e hizasıyla döner; yazı tipi değişse de ortalı yazı ortalı kalır.
- Sıkıştırılmış yazı ve zeminli yazı çizilir; kroki ve paftada yazının altı boşaltılabilir.
- Hizası, çarpanı ve zemini olmayan çizimin baytları öncekiyle aynıdır; eski okuyucu onu açmayı sürdürür.

## Doğrulama

- **Dosya:** şema 7 örnekleri bağımsız Python yazıcısıyla üretilir; bozuk değerler ve şema 6'daki yeni alan reddedilir.
- **Hesap:** hizalı kutunun köşeleri el hesabıyla (0°, 90° ve 30° dönüşte); okunur yapmanın kutuyu yerinde bıraktığı; artırma ve joker kuralları bağımsız Python başvurusuyla.
- **Çizim:** iki platformda aynı sahnenin resimleri: on iki hiza, 0,6 ve 1,5 çarpan, çizginin üstünde zeminli yazı; açık ve koyu temada, iki pencere boyunda.
- **Komutlar:** ortak durumlar üç koşucuda; tek geri alma adımı; kilitli katman reddi.
- **Biçimler:** hizalı TEXT, ATTDEF ve MTEXT'li DXF örneği Rust'ta ve WASM'da; DXF gidiş-dönüşü; bağımsız Python denetimi.
- **Kullanım:** yazı eklerinin kullanım senaryosu iki platformda, adım adım resimleriyle.
