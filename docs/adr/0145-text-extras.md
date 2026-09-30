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

- **İki çizici** (web `viewport/overlay.ts`, masaüstü `labels.rs`) yazıyı çapadan, hizaya göre, kendi ölçtükleri genişlikle yerleştirir. Böylece ortalı yazı ekranda tam ortalıdır, tahmin yalnız seçim kutusunu etkiler.
- **Genişlik çarpanı** yazının kendi çerçevesinde yatay ölçektir: web'de `scale`, masaüstünde glif ana hatları.
- **Zemin** yazıdan hemen önce çizim alanının zemin rengiyle doldurulan kutudur. Kutu çekirdeğin kutusuna her yandan 0,1h pay eklenerek bulunur. İki çizici kutuyu depodan alır, yazı tipinin iki platformdaki küçük ölçü farkı zemini kaydırmaz.
- **Yerinde düzenleme** (ADR 0060): yazı kutusu, hizası ve çarpanıyla yazının yerinde açılır.

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

1. **Sözleşme ve `.kcad` şema 7.** Alanlar, tipli sütunlar, belirtim, Python okuyucu ve yazıcı, örnekler; iki belge alanları taşır.
2. **Çekirdek ve çizim.** Kutu, seçme, kenet, dönüşümler, blok parçaları; iki çizicide hiza, çarpan ve zemin; yerinde düzenleme. Okunur yap, artırma ve joker eşleme işlevleri ortak durumlarıyla.
3. **Komutlar.** `create` ve `edit`'in alanları, `readable` işlemi; ortak durumlar üç koşucuda.
4. **Araçlar ve arayüz.** Yazı aracının seçenekleri ve Artır, Öznitelikler, Okunur yap, Bul ve değiştir, Metin dosyası yerleştir; iki platformda izler ve resimler.
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
