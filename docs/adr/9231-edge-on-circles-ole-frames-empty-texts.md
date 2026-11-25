# ADR 9231: Yandan görülen çember çizgi, OLE nesnesi çerçeve, boş yazı bilgi notu olur

- **Durum:** kabul edildi (2026-11-25). Plan: [plan/render.md](../../plan/render.md) RM-22, 5. ve 6. adım.
- **İstek:**
  - planın RM-22 maddesi: alınmayanlar ele alınır;
  - kabul ölçütleri: “alımda düşen her nesnenin raporda bir nedeni var”, “görüntüde sessizce düşen nesne 0”;
  - 25 Kasım: “Devam et, asla durma, bitene kadar tüm maddeler.”
- **Sözleşmeler:** değişmedi. `.kcad`, komut ve ayar aynı.

## §1 Bulgu

10 DWG örneğinin açılış raporlarında ([ADR 9229](9229-dgn-line-styles-said-by-what-they-draw.md) §3) üç tür “alınmadı”
satırı, kaynak CAD'in gösterdiği ya da hiçbir şey kaybettirmeyen nesnelerdi:

| Satır | Örnek | Ne |
|---|---|---|
| CIRCLE · “görüş doğrultusuna dik” | TEKNOKENT MİMARİ, 28 | Planda bir doğru parçası; atılıyordu |
| OLE2FRAME · “gömülü nesneler alınmaz” | iki YILTAŞ 3 ve 2, TEKNOKENT 2 | Gömülü Excel sayfaları, yerleri de yok |
| TEXT · “boş yazı” | TEKNOKENT, 303 | Çizilecek bir şeyi yok, ama “alınmadı” sayılıyordu |

**TEKNOKENT'in 28 çemberi:** “Abellia grandifoliaa” adlı üç boyutlu bitki bloğunun yaprakları. Yarıçapları 1,2 cm,
normalleri yatay; planda 2–3 cm'lik çizgiler.

## §2 Karar

### §2.1 Yandan görülen çember, yay ve elips planda göründüğü doğru parçasıdır

Okuyucu, izdüşümü genişliksiz kalan çember, yay ve elipsi (`ellipse_from` → yok) atmak yerine onun planda kapladığı
doğru parçası olarak alır (`crates/shared/formats/src/geom.rs`, `edge_on`):

- **Matematik:** yayın noktası `c + u·cos t + w·sin t`. `u` ile `w` aynı doğru üzerindedir; doğru boyunca nokta
  `R·cos(t − φ)` olur.
- **Uçlar:** parçanın uçları bu değerin yayın iki ucundaki ve aradaki dönüm noktalarındaki (`φ + kπ`) en küçüğü ve en
  büyüğüdür. Tam çember `±R`'dir.
- **Rapor:** “görüş doğrultusuna dik; planda göründüğü gibi doğru parçası olarak alındı”.
- **Bir noktaya inen** atlanır; yarıçapı sıfır olan zaten kendi nedeniyle atlanıyor.

### §2.2 OLE nesnesi yerini gösteren çerçevesiyle gelir

OLE2FRAME'in sol üst (10) ve sağ alt (11) köşeleri kapalı bir çerçeve olur, görüntünün çerçevesi gibi
([ADR 9227](9227-dwg-regions-wipeouts-and-images.md) §2.4).

- **İçerik** (tablo, resim, metin) başka bir uygulamanın verisi; alınmaz.
- **Rapor:** “OLE nesnesi (OLE2FRAME) · gömülü belgenin (tablo, resim, metin) içeriği alınmadı; yerini gösteren
  çerçevesi alındı”.
- **Eski OLEFRAME** (R13) eskisi gibi desteklenmeyen türdür.

### §2.3 Boş yazı bilgi notudur

Boş TEXT ve MTEXT, boş öznitelik gibi (ADR 9227 §2.5), “alınmadı” değil bilgi notudur:

- “Yazı (TEXT) · boş; çizilecek yazısı yok”
- “Çok satırlı yazı (MTEXT) · boş; çizilecek yazısı yok”

## §3 Kanıt

**Örnekler, son kod:**

| Dosya | Önce “alınmadı” | Sonra “alınmadı” |
|---|---|---|
| TEKNOKENT MİMARİ | 31 ATTDEF, 3 ARC, 28 CIRCLE, 303 TEXT, 2 OLE2FRAME | 31 görünmez ATTDEF, yarıçapı sıfır 3 ARC |
| MİMARİ YILTAŞ | 130 HATCH, 16 LWPOLYLINE, 3 OLE2FRAME | iki köşesi olmayan 16 LWPOLYLINE |
| YILTAŞ İRTİFAK | 130 HATCH, 13 LWPOLYLINE, 2 OLE2FRAME | iki köşesi olmayan 13 LWPOLYLINE |

(Taramalar [ADR 9230](9230-hatch-boundaries-of-pieces.md) ile.) Kalan her satır çizilmeyen bir şey.

**Sınamalar** (`kentos-formats`):

- `a_circle_seen_edge_on_is_the_segment_it_is_in_plan`:
  - normali x boyunca çember, planda (3, −2)–(3, 2);
  - çeyrek yayı (3, 0)–(3, 2);
  - yarıçapı sıfır olan atlanır.
- `an_ole_frame_is_its_outline_and_an_empty_text_a_note`: çerçevenin dört köşesi, iki not, atlanan yok.
- `an_mtext_is_one_text_with_its_lines_and_formats`: boş MTEXT artık not.

Bütün sınamalar geçti (221 + 143 + 63 + 6); `kentos-native-application` alım sınamaları (14 + 5).

**Gerçek pencere** (`.run/gui-evidence/render/rm22/once-sonra-ole.png`, YILTAŞ İRTİFAK): önce boş, sonra iki gömülü
Excel sayfasının sarı çerçevesi.

Yandan görülen çemberler 2–3 cm'lik yaprak çizgileri; ekran görüntüsünde seçilemiyorlar, kanıtları sınama.

## Uygulama

- `crates/shared/formats/src/geom.rs`: `edge_on`.
- `crates/shared/formats/src/dxf/emit.rs`: `edge_on` (çember, yay, elips); `Kind::OleFrame`; boş TEXT ve MTEXT notu.
- `crates/shared/formats/src/dxf/entity.rs`: `Kind::OleFrame`.
- `crates/shared/formats/tests/all/dxf.rs`: sınamalar.
