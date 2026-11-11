# ADR 8046: DWG/DXF katman profili — dış programda aynı katmanlar ve yazılar

- **Durum:** sürüyor (11 Kasım 2026); `plan/architecture-advanced.md` ADV-25, M46. 1. bölüm yapıldı: mahalin yazısı ve
  boşluğun çaprazı DXF'te ekrandaki gibi (§1). 2. bölüm yapıldı: KentOS'un yazı tipiyle yazılan yazının genişlik
  çarpanı, Arial'da KentOS'taki genişliği verecek biçimde (§2). Sırada: yazı yüksekliği — başka programlar DXF'in
  yüksekliğini büyük harf yüksekliği okuyor, KentOS em yazıyor (§3).
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

### 3. Yazı yüksekliği (sırada)

Başka programlar 40'ı büyük harf yüksekliği okuyor (yukarıdaki kaynaklar). Yazıcı TrueType yazının yüksekliğini o
programın yazı tipinin büyük harf oranıyla yazmalı (`40 = yükseklik × büyük harf oranı`, Standard için Arial'ın 0,7158'i),
okuyucu tersini yapmalı; SHX zaten büyük harf yüksekliğidir (ADR 1125). TEXT, MTEXT, ATTRIB/ATTDEF, ölçü yazısı, tablo,
kılavuz notu ve stil yükseklikleri birlikte; KentOS kendi dosyasını aynen geri okumalı.

## Doğrulama

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
