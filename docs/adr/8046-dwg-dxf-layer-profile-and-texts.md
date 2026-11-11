# ADR 8046: DWG/DXF katman profili — dış programda aynı katmanlar ve yazılar

- **Durum:** sürüyor (11 Kasım 2026); `plan/architecture-advanced.md` ADV-25, M46. 1. bölüm yapıldı: mahalin yazısı ve
  boşluğun çaprazı DXF'te ekrandaki gibi (§1). Sırada: yazıların dış programda KentOS'taki genişliğinde durması (§2).
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
  Arial'da ortalama %7,1 geniş (Pencere 120 %8,4, yatak odası 1 %10,6; 0.900 %0,3). M9'un dış okumasında çizelgenin
  hücreleri bu yüzden taşıyor (“Genişlik (m)Yükseklik (m)”). §2'nin konusu.

### Başka programlar (yalnız davranış için)

- **Revit, Export DWG › Text:** "Text treatment" — Exact (görünüşü korur) ya da Approximate (MTEXT bırakır).
- **Archicad, DWG Translator:** yazı tipi eşleme tablosu, eşlenen yazı tipine genişlik çarpanı.
- **AutoCAD:** yazının yazı tipini STYLE kaydından okur; TTF yoksa yerine başkasını koyar.

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

### 2. Yazıların genişliği (sırada)

Dış programda yazı KentOS'taki yerini ve genişliğini tutmalı. Barlow ile çizilen yazının Arial'daki genişliği her satır
için çekirdeğin harf genişliklerinden (Barlow ve Arial'ın ölçü ikizi Arimo) kesin hesaplanabilir. DXF yazıcısının bütün
yazı çıktısını ve bağımsız başvuru denetimini (`scripts/fixtures/dxf_write_reference.py`) değiştirdiği için ayrı adımdır.

## Doğrulama

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
