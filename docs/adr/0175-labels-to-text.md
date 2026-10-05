# ADR 0175: Etiketleri yazıya çevirme

- **Durum:** kabul edildi (2026-10-05). Sıra sahibin kararıdır: TODOS.md §16.0'ın on sekizinci işi `HYB-18`. Ayrıntılar bu ADR'nin
  varsayılanlarıdır.
- **Bağlam belgesi:** TODOS.md `HYB-18`, `CAD-06` (ilişkilendirme), `GIS-16` (etiket motoru); [araştırma kaydı](../research/2026-10-01-netcad-arcgis-qgis.md);
  ADR 0055 (çizimin yazıları), ADR 0145 (yazı ekleri: hiza, genişlik, zemin), ADR 0164 (pafta: etiketlerin kağıttaki kuralı), ADR 0060
  (yazı nesnesi); Netcad Etiketleri Üret ve CAD'e Çevir, ArcGIS Convert Labels To Annotation ve feature-linked annotation, QGIS Extract
  labels.

## Bağlam

Nesnenin etiketi (`label`: parsel numarası, nokta adı, yol adı) çizim nesnesi değildir: katmanın etiket stili (`LabelStyle`: yerleşim
merkez, köşe, yan ya da boyunca; ekran pikseliyle boyut, büyüme ve üst sınır; kalınlık; `{label}` şablonu; ölçek aralığı; en küçük nesne;
mürekkep) onu her karede ekranda çizer, birbirini örtenleri inceltir. Bu yüzden:

- DXF'e, GeoJSON'a ve Shapefile'a gitmez; AutoCAD'de ya da Netcad'de açılan çizimde parsel numarası yoktur.
- Yazının yeri elle düzeltilemez; tek bir etiket kaydırılamaz, döndürülemez.
- Pafta (ADR 0164) etiketleri kendi ölçeğinde kağıda yazar: bir etiketin kağıttaki boyu CSS pikseli başına 25,4/96 mm'dir, yerleşimin
  piksel kaydırmaları da öyle (`app/sheet/mapLabels.ts`, masaüstünde `sheets.rs`). Bu kural yalnız paftanın içindedir; çizime yazı olarak
  dönmez.

## Karar

### 1. Kural: etiketin yazısı

Bir etiket, seçilen ölçekte (1:N) paftanın onu kağıda yazdığı gibi yazı nesnesi olur:

- **Metin:** şablonun `{label}`'ı nesnenin etiketiyle (şablon yoksa etiketin kendisi); boş etiket yazı olmaz.
- **Yükseklik:** kağıtta `min(üst sınır ?? boyut, boyut + büyüme × px/m) × 25,4/96` mm, yerde bunun `N / 1000` katı metre (px/m = 96 /
  0,0254 / N, kağıdın ölçeği).
- **Yer ve hiza:** merkez yerleşimde nesnenin çapası, hiza ortanın ortası; köşede nesnenin kutusunun sol üst köşesi kağıtta 8 px sağa, 14 px
  aşağıya, hiza ortanın solu; yanda çapa 7 px sağa, 7 px yukarıya, hiza ortanın solu; boyunca köşelerin %35'indeki kenarın ortası, dönüş
  kenarın doğrultusu, 90°'yi aşan ya da −90°'nin altına inen doğrultu okunur kalsın diye yarım tur döner (pafta gibi: −90° ile 90° arası),
  hiza ortanın ortası. Kaydırmalar yerde `px × 25,4/96 × N / 1000` metredir. Çok parçalı alanın ve çoklu çizginin “boyunca” etiketi çapanın
  parçasındadır (en büyük alan, en uzun çizgi; ADR 0143, 0174), parçalar arasına düşmez; çizimin etiketi de böyle olur.
- **Orta:** yazı nesnesinin ortası, yüksekliğinin yarısı taban çizgisinin üstündedir (ADR 0145); paftanın kanvası yazıyı kendi yazı tipinin
  ortasına göre koyar. Çapa ve boy aynıdır; harfler satırın içinde yazı tipine göre birkaç yüzde kayabilir.
- **Seçim kuralları ölçekte:** etiket stilinin ölçek aralığı dışındaki ve kağıtta en küçük nesneden küçük nesnenin etiketi yazı olmaz; boyu 0
  ya da daha küçük çıkan etiket de küçük sayılır.
- **İnceltme:** varsayılan olarak pafta gibi: kağıtta önceki bir etiketin hücrelerine değen etiket atlanır (çizimin sırasıyla, ilk gelen
  yerinde kalır; yazının kutusu projenin yazı tipindeki genişlikle, çekirdeğin ölçüsüyle). Hücreler kağıtta 8 px'tir ve çizimin
  başlangıcından sayılır; böylece aynı etiketler hangi seçimle çevrilirse çevrilsin aynı inceltilir. Seçenek “Örtüşenler de” hepsini çevirir.
- Genişlik çarpanı ve zemin yoktur (seçenek “Zemin” yazılara zemin verir); renk yoktur (katmanınki); etiket stilinin kalınlığı ve
  mürekkebi yazı nesnesinde karşılıksızdır ve söylenmez (yazı projenin yazı tipindedir).

Kural ortak çekirdektedir (`ops::label_text`): etiketler çizimin sırasıyla, her biri yeri (yerleşimi, çapası ya da kutusunun köşesi, boyunca
yerleşimde iki köşe), nesnenin kutusunun küçük kenarı, metni, metnin projenin yazı tipinde em olarak genişliği ve katmanının etiket stiliyle
verilir; yazılar ve atlananların sayıları (ölçek dışı, küçük, örtüşen) döner. Yeri depo bulur, çizimin etiketlerini bulduğu kuralla
(`ops::label_text::spot`, `Store::labels` de onu kullanır); `Store::label_texts` nesnelerin kimliklerini, etiketlerini ve stillerini alır,
yeri, metni (şablonla) ve genişliği bulup kuralı çağırır: masaüstü doğrudan, web WASM'la. Şablonun `{label}`'ı ilk geçtiği yerde harfi harfine
değişir (`fill_template`, web'de `fillTemplate`); çizimin etiketleri ve pafta da bu kuralla yazar: masaüstü bütün `{label}`'ları değiştiriyor,
boş şablonda boş yazıyordu, web'in `replace`'i etiketteki `$&` gibi dizileri yorumluyordu. Pafta kendi kuralını korur; web'in bir testi aynı etiketlerin paftadaki yerini, boyunu ve dönüşünü
çekirdeğin yazılarıyla karşılaştırır. Bağımsız Python başvurusu `label_text_cases.py` kuralın durumlarını (dört yerleşim, büyüme ve üst
sınır, ölçek aralığı, en küçük nesne, okunur yön, inceltme ve hücreleri) kesirle yazar; her durumun kutusu hücre sınırından en az 10⁻⁶ px uzaktır.

### 2. Komut

`cad.entities.create`'in yeni işlemi `labels` (adım “Etiketleri yazıya çevir”): yazılar tek adımda, tek katmana. Hesap çağıranındır
(çekirdeğin kuralı), komut yazıları verildiği gibi yazar; ortak durumlar `fixtures/commands/v1`'de, bağımsız denetimle.

Bağlı yazı (§4) için `NewObject`'in iki alanı: `labelOf` (nesnenin kalıcı kimliği, metin) ve `labelScale` (N). Redler: `invalid_link`
(yazı olmayan nesnede, biri öbürü olmadan, kalıcı kimlik yazımında olmayan kimlik, sonlu ve sıfırdan büyük olmayan ölçek; nesnenin
geometrisinden ve kalınlığından sonra) ve `link_not_found` (adlandırdığı nesne çizimde yok; katman ve bloklardan sonra, sırayla).
`cad.entities.set`'in `unlink`'i (işlem `unlink`, adım “Bağı kopar”) bağlı yazıların iki alanını siler; öbür nesneler değişmez.

### 3. Araç: Etiketleri yazıya çevir

Topolojik temizlik (ADR 0148) gibi: kapsamı başlarken alır, seçenekleri komut satırındadır, önizler, Enter yazar. Böylece iki platform aynı
izle (`fixtures/interaction/v1`) sınanır; pencere yoktur.

- **Yer:** CAD'de Açıklama › Yazı ▾, CBS'de Harita › Etiket; takma adlar `ETIKETYAZI`, `ETIKETCEVIR`, `LABELCONVERT`.
- **Kapsam:** seçim varsa seçili nesnelerin etiketleri, yoksa bütün çizimdekiler; gizli katmandaki nesne girmez. Etiketin stili katmanınki,
  yoksa türünün varsayılanıdır (çizimdeki gibi; etiket stili olmayan katmanda alan, daire, nokta, çoklu çizgi ve çizgi). Etiketi boş olan, yazı,
  ölçü, kılavuz ve stilsiz türler girmez; hiç etiket yoksa araç söyler ve çıkar.
- **Seçenekler**: Ölçek (Ö): `1:N` ya da `N` yazılır, her çalışmada projenin çizim ölçeğinden başlar; Örtüşenler de (R), Zemin (Z) ve
  Katman (K) ve Nesneye bağlı (B) oturum boyunca kalır. Örtüşenler de (R): kapalı başlar;
  Zemin (Z): yazılara zemin (ADR 0145), kapalı başlar; Katman (K): projenin standart yazı katmanı (`yazi`, yeni projedeki adıyla:
  CBS'de “Yazılar”, CAD'de “Yazı”; yoksa aynı adımda açılır, ADR 0067'nin `parsel` ve `kot` katmanları gibi) ya da etkin katman.
  Nesneye bağlı (B): kapalı başlar; açıkken yazılar nesnelerine bağlı yazılır (§4).
- Bağlı yazısı olan nesnenin etiketi kapsama girmez: etiketi zaten yazıdır (§4).
- **Önizleme:** yazılar yerlerinde, boylarında ve dönüşlerinde soluk; imlecin yanında sayılar: kaç etiketin yazı olacağı, kaçının örtüşme, ölçek
  ya da boyut yüzünden atlanacağı.
- **Sonuç:** Enter (ya da Uygula) yazıları `cad.entities.create` (`labels`) ile hedef katmana tek adımda yazar; yazı katmanı yoksa
  aynı adımda açılır. Yazılar seçili kalır, ileti sayıları söyler (“12 etiket yazıya çevrildi; 3 örtüşen atlandı.”); araç çıkar. Kilitli hedef
  katman komutun reddiyle söylenir.
- İki platformda aynı kurallarla, ortak izle.

### 4. Nesneye bağlı yazı

- “Nesneye bağlı” (B) açıkken yazı etiketlediği nesneyi bilir: yazının yeni alanları `labelOf` (nesnenin kalıcı kimliği, ADR 0014) ve
  `labelScale` (N). `.kcad` belge şeması 18 (yazının iki alanı; yazıcı 18'i yalnız bağlı yazı varken yazar); eski şemalı yükte bu alanlar
  bilinmeyen alandır (ADR 0174 gibi). `cad.entities.create`'in yeni nesnesi bağı taşır; `cad.entities.set` bağı koparabilir (Bağı kopar).
- **Güncelleme, iki belgede kayıttan hemen önce:** bir adım (hangi komut, araç, Python ya da MCP yaptıysa) bağlı bir yazının nesnesini
  değiştirdiyse (geometrisi, etiketi ya da katmanı), adım kaydedilmeden yazı §1'in kuralıyla yeniden yazılır ve aynı adıma girer: metni ve
  yeri nesnenin şimdiki etiketinden, katmanının şimdiki etiket stilinden (yoksa türünün varsayılanı), projenin yazı tipinden ve
  `labelScale`'den; inceltme yoktur (her yazı kendi nesnesini izler). Kural tek nesnelik çekirdek işlevidir (`label_text_of`, web'e
  `labelTextOf` işlemiyle): nesnenin biçimi, etiketi, stili, ölçek ve yazı tipini alır, yazıyı verir; yazmıyorsa hiçbir şey vermez (bağ
  kopar). Belge kuralını (web `CadDocument`'in `model/linkedTexts.ts`'i, masaüstü `kentos_domain::Document`'in `linked.rs`'i; iki belge bağlı
  yazıları nesnenin kalıcı kimliğiyle dizinler) ortak belge fixture'ları (`fixtures/document-ops/v1/linked-texts.json`) tutar; geri alma ve
  yineleme yazıyı nesneyle birlikte döndürür. Gruptaki (bir modelin, uzun bir işin) değişiklikler grubun sonunda izlenir; başka bir
  düzenleyiciden gelen değişiklik izlenmez (onu yapan istemci izlemiştir).
- **Bağın kopması:** nesnenin etiketi boşalır ya da kural onu artık yazmazsa (ölçek dışı, küçük) yazı yerinde kalır ve bağı kopar (iki alan
  silinir), aynı adımda; yazının kendisi düzenlenince (yeri, metni, boyu, dönüşü, hizası) bağı kopar: elle yerleştirilen yazı nesneyi izlemez.
- **Silme:** nesnesi silinince bağlı yazı da aynı adımda silinir. Patlat ve Parçalara ayır yazıyı nesnenin yerinde kalan parçasına bağlı
  bırakır (o parça nesnenin kimliğini taşır).
- **Çizimde:** bağlı yazısı olan nesnenin etiketi çizimde ve paftada çizilmez; yazı onun yerini alır. Bağsız yazıya çevrilen etiketler çizimde
  yazılarıyla üst üste görünür; katmanın etiketini kapatmak kullanıcınındır.
- Öznitelikler'de “Bağlı nesne” satırı (nesnenin türü ve etiketi) ve “Bağı kopar”; DXF ve GeoJSON bağı yazmaz (yazı düz yazıdır) ve bunu
  dışa aktarma penceresi söyler.
- Parçalar: 4a sözleşme ve şema 18 (kodek, sütunlar, bağımsız Python okuyucu ve yazıcısı, örnek dosya); 4b çekirdeğin `label_text_of`'u ve iki
  belgede kayıt öncesi izleme (ortak belge durumları); 4c araçta “Nesneye bağlı”, çizimde etiketin yerini alma, Öznitelikler, dışa aktarma.

### 5. Kapsam dışı

- Etiket motorunun kendisi (`GIS-16`: kurallar, çakışma önleme, kıvrık yazı); ölçü ve tarama ilişkileri (`CAD-06`'nın geri kalanı).
- Yazının etikete geri dönmesi.

## Adımlar

1. Çekirdek: `ops::label_text` (kural ve inceltme, yer), `Store::label_texts`, bağımsız başvuru `label_text_cases.py`; paftayla karşılaştırma
   testi.
2. Komut: `cad.entities.create`'in `labels` işlemi; ortak durumlar.
3. Araç iki platformda: kapsam, seçenekler, önizleme, yazı katmanı, ileti; ortak iz ve resimler.
4. Bağlı yazı (4a, 4b, 4c; §4): sözleşme ve `.kcad` şema 18; çekirdeğin `label_text_of`'u ve iki belgede kayıt öncesi izleme; araçta
   “Nesneye bağlı”, çizimde etiketin yerini alma, Öznitelikler, biçimlerin raporu.

Her adım iki platformda, ortak fixture'larla, kendi commit'inde ilerler.

## Doğrulama

- Kuralın değerleri bağımsız başvurudan (ölçek, piksel ve milimetre hesabı kesirle); paftanın etiketleriyle aynı yer ve boy.
- Komut ve araçlar ortak durumlar ve izlerle iki platformda; bağlı yazının güncellemesi geri alma ve yineleme ile.
