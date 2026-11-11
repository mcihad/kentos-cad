# ADR 8046: DWG/DXF katman profili — dış programda aynı katmanlar ve yazılar

- **Durum:** bitti (11 Kasım 2026); `plan/architecture-advanced.md` ADV-25, M46. 1. bölüm yapıldı: mahalin yazısı ve
  boşluğun çaprazı DXF'te ekrandaki gibi (§1). 2. bölüm yapıldı: KentOS'un yazı tipiyle yazılan yazının genişlik
  çarpanı, Arial'da KentOS'taki genişliği verecek biçimde (§2). 3. bölümün ilk adımı yapıldı: TrueType yazının
  yüksekliği DXF'te büyük harf yüksekliği — TEXT, öznitelik, tablo hücresi, eğri yazı ve stilin sabit yüksekliği (§3a);
  MTEXT (satır aralığı ve girintiler dahil) ve kılavuz (§3b); ölçü yazısı, tablo stili ve MULTILEADER (§3c); dikey
  hizalı yazının taban çizgisi (§3d).
- **Bağlam belgesi:** ADR 8008 (EK-4 katman anahtarı), ADR 0342 (mahal), ADR 0708 (boşluk mahal), ADR 0344 (etiket ve
  27 Ekim eki: etiketli mahalin yazısı etiketidir), ADR 8039 §4 (tadilat durumunun katmanları), ADR 0586 (DWG kodeği).

## Bağlam

Planın S5 tespiti: "DXF'te mahalin adı ve alanı yazılmıyor". ADV-25: "DXF/DWG: ADV-14'teki katman profiliyle; mahal
yazıları ve evre katmanları da gider." İlk kabul: "DXF dış programda aynı katmanlarla açılır." Ofisin DWG'si katmanları
ve yazılarıyla gidip gelmelidir.

## Araştırma

### Türkiye mevzuatı

- **2026 Dijital Proje Yönetmeliği EK-4** (Md. 7/1): katman adları `MM-ÖĞE-GÖSTERİM`; Tablo 2.3'te "Mahal" öğesi yoktur,
  mahal "Genel (XXXX)" öğesi ve MAHAL gösterimiyle MM-XXXX-MAHAL'dir (ADR 8008). Mahalin yazısı (numara, ad, alan) mahalin
  kendisidir: aynı katmanda.

### Bugünkü durum (ölçüldü)

- M9'un 2+1 DXF'i (`.run/gui-evidence/mimari-m9/2p1-ek4.dxf`) ezdxf 1.4.5 ile okundu: 12 katman, 10'unda nesne;
  MM-XXXX-MAHAL'de 5 LWPOLYLINE ve yazı yok. Mahaller etiketli olduğu için ad ve alan etiketin yazısı olarak
  MM-XXXX-ETIKET'te gidiyordu; **etiketsiz mahalin hiçbir yazısı yoktu**. ADR 0708 boşluğun çaprazının "DXF'in mimari
  bloklarında" olduğunu söylüyordu; yazıcı mahalin yalnız halkalarını yazıyordu, çapraz da yoktu.
- Evre (tadilat) katmanları zaten gidiyor (…-MEVCUT, -IPTAL kesikli, -ILAVE, -GECICI; ADR 8039 §4). DWG, aynı DXF'in
  OpenCADCodec ile çevrilmesidir (ADR 0586): DXF'te ne varsa DWG'de o var.
- **Yazı genişliği:** KentOS varsayılan yazıları Barlow ile çizer; DXF'in Standard stili `arial.ttf` der. Aynı satırlar
  Arial'da ortalama %7,1 geniş (Pencere 120 %8,4, yatak odası 1 %10,6; 0.900 %0,3); §2'nin konusu. M9'un dış okumasında
  çizelgenin hücreleri taşıyor (“Genişlik (m)Yükseklik (m)”): 2. bölümde ölçüldü, asıl sebep yükseklik (aşağıda, §3).

### Başka programlar (yalnız davranış için)

- **Revit, Export DWG › Text:** "Text treatment" — Exact (görünüşü korur) ya da Approximate (MTEXT bırakır).
- **Archicad, DWG Translator:** yazı tipi eşleme tablosu, eşlenen yazı tipine genişlik çarpanı.
- **AutoCAD:** yazının yazı tipini STYLE kaydından okur; TTF yoksa yerine başkasını koyar.

### Yazı yüksekliği başka programlarda (2. bölümde ölçüldü, §3'ün konusu)

KentOS yazının yüksekliğini yazı tipinin **em**'i olarak çizer (`text::shaping::placed`: glif `birim / em × yükseklik`;
Barlow'un büyük harfi yüksekliğin 0,700'ü). ADR 0914 DXF yüksekliğini "font metrikleri ve karşılaştırılabilir gerçek
çıktı ile ayrıca doğrulanacak" diye açık bırakmıştı. Kaynaklar aynı yönde:

- **Trimble Tekla Structures,** `XS_DRAWINGS_USE_CAP_HEIGHT_FOR_FONT_HEIGHT`: DGN/DWG/DXF'e aktarımda yazı yüksekliği
  büyük harf yüksekliğidir; "Tekla'daki yükseklik AutoCAD'de açıldığında aynıdır"; em ile çalışan çizimde aktarım "em'den
  CAP'e" çevirir.
- **BricsCAD** (Properties › Height): "the height of the uppercase text" (BricsCAD V5'ten beri AutoCAD'in TrueType
  yorumunu taklit eder; Bricsys forumu).
- **QCAD** (`RTextRenderer.cpp`): TrueType yazıyı 'A' harfi yüksekliğe eşit olacak biçimde ölçekler. **ezdxf**
  (`ttfonts.py`): aynı ('A'nın tepesi). **Esri** bilgi bankası 000013489: AutoCAD ve MicroStation'da yazı boyu "harflerin
  gerçek boyuna" göredir.
- **Autodesk** ("About Setting Text Height"): TrueType'ta yükseklik "büyük harf artı aksan payı"dır, oranı yazı tipine
  göre değişir. AutoCAD'in tam formülü yayımlanmamış; ölçülebilen bütün uyumlu uygulamalar büyük harf yüksekliğini alıyor.

Ölçüm (yazı tiplerinin dosyalarından, 'H' glifi): Arial 1466/2048 = 0,7158 em; Arimo (KentOS'un Arial ikizi) 0,688 —
**ilerlemeler Arial'la aynı, büyük harf %4,1 kısa**; Barlow 0,700. Yani KentOS'un `40 = em` yazdığı yazı AutoCAD'de büyük
harfi yüksekliğe eşit, em'i yüksekliğin 1/0,7158 = 1,397 katı çizilir: **%40 büyük**. M9'un çizelge taşması asıl bundandır
(aşağıda: 2. bölümden sonra da sürüyor).

## Karar

### 1. Mahal DXF'te ekrandaki gibi (1. bölüm)

- `formats::spaces::space_pieces_in`: mahalin parçaları ekranın `geom::space::pieces_with_view`'ı ile kurulur —
  halkaları, boşluğun çaprazı ve etiketsiz mahalin yazısı (numara, ad, alan; kâğıtta 2,5 mm, mahalin kutbunda ortalı;
  çok satırlı olduğu için MTEXT), hepsi mahalin katmanında ve görünüşünde.
- **Etiketli mahalin yazısı etiketidir** (ADR 0344 27 Ekim eki): bir etiketin `of`'u mahalse mahal kendi yazısını yazmaz;
  etiketin yazıları zaten gider. Ekranla aynı kural.
- KentOS verisi saklansa da (mimari bütünün bloğu) saklanmasa da (düz nesneler) aynı parçalar yazılır. DWG aynı DXF'ten
  çevrildiği için aynı yazılar DWG'de de var.
- Dışa aktarma raporu: "mahaller sınırlarının kapalı çoklu çizgileri, boşluğun çaprazı ve etiketsiz mahalin yazısı
  (numara, ad, alan) olarak yazıldı; ölçme yöntemleri ve kaplamaları taşınmaz, mahal olarak geri okunmaz".

### 2. Yazıların genişliği (2. bölüm)

KentOS'un kendi yazı tipiyle (stil yok, yazı tipi seçilmemiş) yazılan yazı DXF'te Standard'dır; Standard'ın dosyası
`arial.ttf`'tir, başka program onu Arial'la çizer. Aynı satır Arial'da Barlow'dakinden −%3,6 … +%29 geniş ("P1" %29,
"yatak odası 1" %8,6). Sabit bir çarpan bunu düzeltmez; **her satıra kendi çarpanı**:

- `41 = genişlik çarpanı × uyum`, `uyum = Barlow genişliği / Arial genişliği`, üç basamak (çarpım altı basamak).
  Barlow genişliği çekirdeğin yerleşimi (`text::width_em_in`: ekrandaki ilerlemeler, satır kalınsa kalın); Arial'ınki
  ölçü ikizi Arimo'nun **düz** ilerlemeleri (`text::shaping::advance_em`, harf aralığı yok: DXF okuyan programın glifleri
  art arda dizdiği gibi). Payda hep düz Arial: Standard'ın tek dosyası `arial.ttf`'tir; kalın yazı orada düz çizilir,
  kalınlığı değil genişliği korunur. Arimo'da glifi olmayan harfli satır çarpansız kalır.
- Hizalı yazının 11'i aynı. Başlangıç (10), hizayı bilmeyen programlar için, Arial (Arimo) genişliğinin dosyanın 41'iyle
  çarpımı kadar geride: uyumlu yazıda bu KentOS'taki satırın genişliğidir, yazı 10'dan çizilip 11'de biter. İlk yazımda
  10 çarpansız Arial genişliğiyle hesaplanıyordu (başvuru yalnız tutarlılığa bakıyordu); başvuru artık bu genişliği
  kesin denetliyor (1e-8 m).
- Uygulanan yerler: TEXT, ATTRIB (öznitelik hep düz çizilir), tablo hücresi (kalınlığı KentOS'un çizdiği gibi: hücre
  stili, yoksa başlık satırı ya da yazı yüzü), eğri boyunca yazının harfleri. Görünmez ATTRIB (tanımı olmayan öznitelik,
  ADR 3980) KentOS'ta çizilmediği için çarpansızdır. Kendi stili ya da yazı tipi olan yazıya dokunulmaz.
- KentOS kendi çarpanını geri okur: TEXT'in KentOS verisinde `fit` öğesi (`1002 { 1000 fit 1040 <uyum> }`); okuyucu 41'i
  ona böler. Öznitelik, tablo ve eğri yazı KentOS'a kendi verisinden döner; çarpan onları değiştirmez.
- Uyum çarpanı em'i korur: §3'te yükseklik büyük harf yüksekliğine çevrildiğinde (başka programın em'i KentOS'unkine
  eşitlenince) aynen geçerlidir.

### 3. Yazı yüksekliği: DXF'te büyük harf yüksekliği

Başka programlar TrueType yazının 40'ını büyük harf yüksekliği okuyor (yukarıdaki kaynaklar). KentOS'un modeli em'de
kalır (ADR 0914); dönüşüm yalnız DXF/DWG sınırında (`formats::dxf::capitals`):

- **Oran yazı tipinin kendisinden:** STYLE kaydının dosyasının (3; yoksa ACAD verisindeki aile ve kalın/eğik bayrakları)
  yüz tipinin büyük harf payı. Arial (Standard, `arial*.ttf`, Helvetica): **1466/2048** (dört dosyada aynı, ölçüldü);
  Courier New: 1170 (`cour`), 1171 (`couri`), 1213/2048 (`courbd`, `courbi`); KentOS'un kendi yüzleri ve Liberation Sans
  dosyalarının OS/2 büyük harf yüksekliği (Barlow 700/1000, Arimo 1409/2048, Courier Prime 1187/2048, Overpass ve
  Quicksand 0,7, IBM Plex Mono 0,698); KentOS'ta olmayan TrueType yüz (Times, Calibri …) KentOS'ta Barlow'la çizildiği için
  Barlow'unki — iki programda büyük harfleri aynı boyda. SHX dosyası ya da uzantısız ad (`txt`, `romans`) ve hiçbir şey
  söylemeyen stil: dönüşüm yok (SHX'te yükseklik iki tarafta da büyük harftir, ADR 1125). STYLE tablosu olmayan dosya
  AutoCAD'in Standard'ı, txt.shx: dönüşüm yok.
- **Yazıcı:** `40 = em × oran`; okuyucu `em = 40 / oran`. Bölme yüksekliği tam vermiyorsa TEXT'in ve ATTDEF'in KentOS
  verisinde `height` öğesi (`1002 { 1000 height 1040 <em> }`) kendi yüksekliğini tutar; başka program 40'ı değiştirdiyse
  dosya kazanır (öğe yalnız 40 hâlâ KentOS'un yazdığıyken okunur).
- **3a (yapıldı):** TEXT, ATTRIB, ATTDEF, tablo hücresi, eğri yazının harfleri ve proje stilinin sabit yüksekliği (STYLE 40;
  okurken stilin yüksekliği de bölünür). Hizalı (72 = 3) ve sığdırılmış (72 = 5) yazının yüksekliği iki noktasının
  arasından ve Arial genişliğinden çıkar: değişmedi (em zaten oradan). Tanımı olmayan özniteliğin görünmez ATTRIB'i
  (ISO 3098'in 2,5 mm'si) büyük harf yüksekliğidir: dönüşümsüz.
- **3b (yapıldı):** MTEXT ve kılavuz. Satır aralığı ve girinti/sekme durakları da yüksekliğe bağlı: AutoCAD'in satırları
  5/3 × 40 × 44 aralıklı, ezdxf: "girintiler ve sekme durakları varsayılan yazı yüksekliğinin katlarıdır"; KentOS'unkiler
  em'in katı (`paragraph::PITCH`). Yazıcı: `40 = em × oran`, `44 = aralık / oran` (aralık yoksa 1 / oran; 73 = 1), AutoCAD'in
  0,25–4 aralığında — aşan 4 yazılır ve söylenir; `\px` değerleri oranla bölünür. Okuyucu tersini yapar: 44'ü olmayan
  yabancı Arial MTEXT'in aralığı 0,7158 olur, satırları AutoCAD'deki gibi 5/3 × 40 aralıklı. KentOS'un kendi düzeni (`layout`)
  verisinden aynen; aralık bölmeyle tam dönmüyorsa verisinde `spacing` öğesi. Kılavuz: LEADER 40'ı ve notunun MTEXT'i
  Standard'ın oranıyla; notun KentOS verisinde `height`; ok boyunun oranı KentOS'un em'ine göre okunur.
- **3c (yapıldı):** ölçü yazısı: bloğundaki MTEXT (blok biçimli kılavuzun notu da aynı yazıcıdan) ve DIMTXT — ölçünün
  DSTYLE'ında değerin yazı tipinin oranıyla, DIMSTYLE kaydında DIMTXSTY'sinin (yoksa Standard'ın) oranıyla; okları,
  uzatma çizgileri ve boşluk em'in katı uzunluklardır, değişmez. Okurken DIMTXT, DIMTXSTY kaydının (yoksa Standard'ın)
  oranıyla bölünür: yabancı ölçünün yüksekliği ve görünüşünün oranları KentOS'un em'ine göre (stil bu yüzden içe
  alınmaz). Tablo stili (TABLESTYLE) hücre yükseklikleri hücrenin yazı stilinin oranıyla; MULTILEADER'ın notu MTEXT
  yolundan zaten çevriliyor, ok oranı artık notun em'ine göre. KentOS'un kendi ölçüsü ve tablo stili verisinden aynen.
- **3d (yapıldı): taban çizgisi.** Başka program taban çizgisini hiza noktasının, KentOS em'inin değil dosyanın 40'ının
  payı kadar altında bulur (0 taban, −0,2 alt, ½ orta, 1 üst; MTEXT'in iliştirme noktası da öyle: 1–3 üst, 4–6 orta, 7–9
  alt). Hiza noktası dosyaya `pay × (1 − oran) × em` aşağıda yazılır (`capitals::drop`, `lowered`): TEXT ve ATTRIB/ATTDEF'in
  11'i, MTEXT'in ve blok MTEXT'in (ölçü değeri) 10'u, kılavuz notunun 10'u. Satırların aralığı iki tarafta aynı olduğu
  için (3b) çok satırda da yalnız bu fark kalır. Okuyucu noktayı aynı kadar yükseltir; yabancı Arial yazının taban
  çizgisi de böylece AutoCAD'deki yerinde. KentOS'un kesin noktası verisinde `at` öğesiyle (TEXT ve ATTDEF'te bölme tam
  vermediğinde; MTEXT'te her zaman, çünkü okuyucunun dönüşü 11'in doğrultusundan gelir); dosyadaki nokta bir nanometreden
  çok oynamışsa dosya kazanır. Taban hizalı yazı (tablo hücreleri, sola yaslı etiketler) ve SHX değişmez.
  Bilinen sınır: AutoCAD'in TrueType'ta "alt" hizayı (inen harfler) tam nereden ölçtüğü yayımlanmamış; KentOS'un −0,2'si
  büyük harf yüksekliğine uygulandı.
- **Bilinen sınır:** KentOS'ta olmayan TrueType yüz için Barlow'un oranı alınır; projenin yazı tipi başka (ör. Courier
  Prime) olduğunda o projenin yabancı yazısı biraz farklı boyda okunur.

## Doğrulama

### 3. bölüm, dördüncü adım (taban çizgisi)

- 2+1 dairenin DXF'i ezdxf'le: ortalı etiketlerin (K/P kodları, mahal damgaları) taban çizgisi 3c'de yüksekliğin %14'ü
  kadar yukarıdaydı; şimdi KentOS'taki yerinde (`.run/gui-evidence/mimari-m46/3-yukseklik/m46-3d-yakin.png`).
- `an_aligned_text_s_baseline_is_where_another_program_draws_it`: Arial, 40 = 2.5, 11 = (0, 0): ortalının KentOS taban
  çizgisi −1.25 (AutoCAD'deki), üst hizalının −2.5; taban hizalı ve SHX yazı yerinde. `capitals` birim testi: ½ × (1 −
  0,7158) × 2.5 aşağı, saklı nokta aynen geri, oynatılmış nokta kazanır.
- Bağımsız başvuru: TEXT/ATTRIB/ATTDEF'in 11'i indirilmiş noktada (1e-9), 10'un 11'in altında dosyanın 40'ının payı kadar
  olduğu (1e-8), MTEXT'in 10'u ve `at` öğesi, kılavuz notunun 10'u; 11 dosyada tutarlı. Testler: DXF yazma 59 (taban dosyası
  38311 → 38312 bayt), biçim DXF/ölçü/CAD G/Ç 62, birim 50, headless 19, uygulama 7, masaüstü `layer_keys`.

### 3. bölüm, üçüncü adım (ölçü, tablo stili, MULTILEADER) ve dış çizim

- 2+1 dairenin DXF'inde 8 ölçünün değer MTEXT'i ve DSTYLE DIMTXT'i 0,7158 katı; tablo ve etiketler 3a'dan.
- Dış çizim (`.run/gui-evidence/mimari-m46/3-yukseklik/`, ezdxf 1.4.5 + gerçek Arial; ezdxf, AutoCAD gibi yüksekliği büyük
  harf yüksekliği okur): **önce** (1. bölümün yazıcısı) mahal etiketleri çerçevelerinden, kapı/pencere kodları
  altıgenlerinden ve çizelge yazıları hücrelerinden taşıyordu ("Genişlik (m)Yükseklik (m)", "Pencere 1201.200");
  **sonra** hepsi yerinde, yazılar KentOS'taki boyunda.
- Testler: DXF yazma 59 (ölçü bloğu testinde DSTYLE 140 = 0,75 × 1466/2048 = 0,536865234375; taban dosyası 38171 → 38311
  bayt), biçim DXF bütünleşme ve ölçü/CAD G/Ç 61 (yabancı KADASTRO ölçü stili ADA'nın Arial'ında: 6 mm → 6 / 0,7158 mm;
  ölçüsü 3 m → 4,191 m em, ok ve boşluk oranları buna göre), birim 49, headless DXF/DWG 19, uygulama DXF 7, masaüstü
  `layer_keys`. Bağımsız başvuru DSTYLE ve DIMSTYLE 140'ını denetler, 11 dosyada tutarlı; tablo stili dışa aktarmasının
  ezdxf denetimi Standard hücrelerinde 140'ı 1466/2048 katı bekler, "güncel".

### 3. bölüm, ikinci adım (MTEXT ve kılavuz)

- Bölüm binasının zemin katı (`dxf_rooms`): beş mahal yazısı MTEXT, 0,25 m'lik yazının 40'ı **0,178955078125** (0,25 ×
  1466/2048), 44'ü **1,397** (2048/1466): AutoCAD'in satır aralığı 5/3 × 0,179 × 1,397 = 5/3 × 0,25, KentOS'unkiyle aynı.
  2+1 dairenin DXF'inde değişen yok (mahaller etiketli: yazıları TEXT; 8 MTEXT ölçülerin bloğunda, 3c).
- `an_mtext_s_height_spacing_and_indents_are_its_capitals`: Arial MTEXT 40 = 2.5 → em 2.5 / 0,7158; 44'süz aralık 0,7158,
  44 = 2 ile 2 × 0,7158 (iki durumda da KentOS'un satır aralığı 5/3 × 2.5 × 44); `\pxi2;` girintisi 2 × 0,7158 em; SHX
  stilinde hiçbiri değişmez. `capitals` birim testleri: aralıklar 0,25 … 4 gidip aynen geliyor, 4'ün 4'te kesilip KentOS
  verisinde tutulduğu.
- Bağımsız başvuru: MTEXT'in 40'ını, 73/44'ünü ve `height`/`spacing` öğelerini, kılavuzun LEADER ve not 40'ını ve
  `height` öğesini denetler; 11 dosyada tutarlı. Testler: DXF yazma 59 (kılavuzların gidiş-dönüşü dahil), biçim DXF birim
  49 ve bütünleşme 43, ölçü/CAD G/Ç 21, headless DXF/DWG 19, uygulama DXF 7, masaüstü `layer_keys`.

### 3. bölüm, ilk adım (TEXT ailesi)

- 2+1 dairenin DXF'i (`layer_keys`): 521 nesne aynı; **133 TEXT'in 40'ı 0,71582 katı** (Arial'ın büyük harf oranı),
  MTEXT ve ölçüler değişmedi (3b, 3c).
- `a_truetype_text_s_height_is_its_capitals_height`: aynı 40 = 2.5 ile `arial.ttf` → em 2.5 / (1466/2048), `romans.shx` ve
  `txt` → 2.5, `times.ttf` → 2.5 / 0.7, `courbd.ttf` → 2.5 / (1213/2048); STYLE tablosu olmayan dosya 2.5.
- `capitals` birim testleri: her yüzün oranı; 2.5, 0.25, 1/3, 0.18, 3.5e-3, 1234.5678 gidip aynen geliyor; başka
  programın değiştirdiği 40 KentOS'un sakladığı yüksekliği yener.
- Bağımsız başvuru (`dxf_write_reference.py`, yüz dosyalarının OS/2 tablosunu standart kütüphaneyle okur): TEXT, ATTRIB,
  ATTDEF, tablo hücresi, eğri yazı harfi ve stil sabit yüksekliğinde `40 = em × oran`ı, TEXT ve ATTDEF'in `height`
  öğesinin tam gerektiğinde bulunduğunu denetler; eski fikstürleri 6 dosyada reddetti, yenileri 11 dosyada tutarlı.
- Testler: DXF yazma 59 (taban dosyası 38160 → 38171 bayt), biçimlerin DXF birim 48 ve bütünleşme 42 (karmaşık çizgi tipinin
  "Utility Text" stili Barlow'da: 4 → 4 / 0,7 mm) ve ölçü/CAD G/Ç 21, headless DXF ve DWG 19, uygulama DXF 7, masaüstü
  `layer_keys`; tablo stili dışa aktarma fikstürü yeniden yazıldı, ezdxf denetimi "güncel".

### 2. bölüm (yazı genişliği)

- 2+1 dairenin DXF'i (`layer_keys` testinin dosyası, KentOS verisi yazılmadan), önceki ve yeni yazıcıyla: iki dosya
  **yalnız 41 gruplarında ve `fit` öğelerinde** ayrışıyor (521 nesne ikisinde de). 133 yazının (15'i kalın başlık hücresi)
  Arial genişliği KentOS'unkinden: önce **−%3,64 … +%29,43, ortalama +%6,08**; sonra **−%0,05 … +%0,06** (çarpanın üç
  basamağı).
- Dış çizim (`.run/gui-evidence/mimari-m46/2-genislik/`, ezdxf 1.4.5 + gerçek Arial): çizelgede taşma azaldı
  ("Pencere 1201.200" → "Pencere 120 1.200") ama sürüyor — ezdxf yüksekliği büyük harf yüksekliği okuduğu için yazılar
  %40 büyük. §3.
- `a_text_in_kentos_s_own_face_is_as_wide_in_arial_as_kentos_draws_it`: "yatak odası 1" 0.921, kalın 0.931 (kalın Barlow /
  düz Arial), 1,2 çarpanla 1.1052, Arimo yazı tipli yazı çarpansız; tablo başlığı "Genişlik (m)" 0.961 (kalın), gövde
  "1.20" 0.892; KentOS çarpanları aynen geri okur. Beklenen değerler başvurunun yazı tipi dosyalarından hesabı.
- Bağımsız başvuru (`dxf_write_reference.py --check`, KentOS kodu yok; Arimo'nun `cmap`+`hmtx`'ini standart kütüphaneyle
  okur): TEXT, ATTRIB, tablo hücresi ve eğri yazı harfinin 41'ini, TEXT'in `fit` öğesini denetler; 11 fikstür tutarlı.
  Bozulmuş bir hücre çarpanını ("0.952" → "0.5") ve bozulmuş etiketi yakalıyor. Başvuru, tanımı olmayan özniteliklerin
  görünmez ATTRIB'ini (ADR 3980/4380) o ADR'den beri tanımıyordu (blocks.dxf HEAD'de de düşüyordu): artık sayı, etiket,
  görünmezlik, nokta, 2,5 mm yükseklik ve KentOS verisindeki ad (`tagname`) denetleniyor.
- Biçim testleri: DXF yazma 59 (yeni 1), biçimlerin DXF birim testleri 46, bütünleşme 41 (tablo stili dışa aktarma
  fikstürü yeniden yazıldı; ezdxf denetimi "güncel"), headless DXF 14 ve DWG 7, uygulama DXF 7, masaüstü `layer_keys`.

### 1. bölüm (mahal)

- `dxf_rooms` testi (bölüm binası, zemin kat, KentOS verisi yazılmadan): beş mahal beş yazı, MM-XXXX-MAHAL'de, satırları
  “001 / Yatak odası / 17.85 m²”, “002 / Salon / 17.85 m²”, “003 / Merdiven holü / 24.51 m²”, “004 / Banyo / 17.85 m²”,
  “005 / Mutfak / 17.85 m²”; boşluk mahal (Şaft) eklenince MM-XXXX-MAHAL'de 0 → 2 çizgi (çapraz) ve “Şaft / 3.36 m²”;
  Salon etiketlenince kendi yazısı yerine etiketin üç satırı (“002”, “Salon”, “17.85 m²”). Rapor mahalin neye
  dönüştüğünü söyler. Biçimlerin DXF yazma testleri 58, başvuru `--check` tutarlı; mahal fikstürü olmadığı için kayıtlı
  DXF'ler değişmedi. Mahal ayırıcı ve kaynak testlerinin DXF gidiş-dönüşleri (15) değişmedi.
- Dış program: aynı DXF'ler ezdxf 1.4.5 ile (KentOS kodu yok) okunup çizildi
  (`.run/gui-evidence/mimari-m46/`). Önceki yazıcı: MM-XXXX-MAHAL'de LWPOLYLINE 5, yazı 0. Yeni yazıcı: LINE 2,
  LWPOLYLINE 7, MTEXT 5, TEXT 3; sekiz yazı. Etiketin “17.85 m²” satırı kutusundan biraz taşıyor: §2.
- Gerçek pencere notu: Xvfb'de sistemin dosya penceresi (rfd 0.17, xdg-portal) açılmadığı için dosya, DXF dışa aktarma
  penceresinin kullandığı yazıcıdan test içinde yazıldı.
