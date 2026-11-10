# W1 — AutoCAD ve DWG uyumlu alternatiflerde mimari çizim yetenekleri

**Araştırma tarihi:** 7 Kasım 2026 · **Amaç:** KentOS CAD "Mimari" modülünün plan maddelerini iyileştirmek
(hedef: hem çok pratik/hızlı hem AutoCAD kadar ayrıntılı mimari çizim).

**Kapsam:** AutoCAD 2026/2027 (çekirdek 2B + Architecture toolset), AutoCAD LT 2026/2027, BricsCAD V26/V27
(Lite/Pro/BIM), ZWCAD 2026 (+2027 Beta, ZWCAD Architecture), GstarCAD 2026/2027 (+GstarCAD Architecture),
progeCAD 2026/2027 (mimari AEC eklentisi), IntelliCAD 14.x, DraftSight 2026, nanoCAD 26 (+Construction modülü).

**Yöntem:** Yalnız üretici yardım/ürün sayfaları ve üreticinin resmî blogları; yaklaşık 74 arama/getirme.
Bu dosyadaki **bütün ekran görüntüsü URL'leri** yalnız başlık (HEAD) isteğiyle denetlendi (dosya indirilmedi) ve
hepsi `200 image/*` döndü. Kaynak kimlikleri (A1, R5, B9 …) en alttaki **§6 Kaynaklar** listesindedir.

---

## 0. Okuma anahtarı ve sınırlar

| İşaret | Anlamı |
|---|---|
| ✔ | Resmî kaynakla doğrulandı (hücrede kaynak kimliği) |
| ◐ | Kısmen: yalnız belirli paket/sürümde, deneysel ya da benzer ama eşdeğer olmayan işlev |
| ✖ | Resmî kaynak yokluğu açıkça söylüyor (ya da hücre notunda gerekçe var) |
| (✔) | Ürünün bilinen çekirdek CAD işlevi, ama **bu çalışmada resmî sayfa açılarak doğrulanmadı → "doğrulanamadı" say** |
| ? | Doğrulanamadı; resmî kaynakta bilgi bulunamadı |

**Sınırlar (önemli):**
- `autodesk.com` ürün ve karşılaştırma sayfaları otomatik getirmede **HTTP 403** döndü. Autodesk için yalnız
  `help.autodesk.com` okundu; resmî AutoCAD ↔ LT karşılaştırma tablosu **okunamadı**.
- `bricsys.com` artık `bricscad.octave.com`'a 301 ile yönleniyor. Resmî sürüm karşılaştırma sayfasında
  (Lite / Pro / Mechanical / BIM / Ultimate) satır başlıkları okunabildi (2D Drafting, 3D Modeling, BIM tools …)
  ama **işaretler okunamadı**; paket bilgisi komut yardım sayfalarının başlık görsellerinden alındı.
- **ZWCAD Architecture** için güncel resmî ürün sayfası bulunamadı (eski basın bültenleri ve üçüncü taraf kaynaklar
  var) → ilgili hücreler "?".
- **DraftSight** paket tablosu (Standard/Professional/Premium/Enterprise) için güncel resmî tablo bulunamadı; bayi
  kaynakları birbiriyle çelişiyor. Yalnız 2026 yenilikleri resmî blogdan.
- **nanoCAD BIM Construction 26** (Rusya pazarı) incelenmedi; uluslararası `nanocad.com` kullanıldı.
- "arama özeti" notu: resmî sayfanın kendisi açılamadan, arama sonucundaki özetten alınan bilgi; güveni orta.

## 0.1 Ürün ve sürüm durumu (7 Kasım 2026)

| Ürün | Bu araştırmada görülen güncel sürüm | Not | Kaynak |
|---|---|---|---|
| AutoCAD | 2027 (2027.1 güncellemesi yayında); 2026.1.2 | Architecture toolset AutoCAD aboneliğine dahil | A1, A6, A20 |
| AutoCAD Architecture toolset | 2027 | 2027: taşıyıcı eleman koordinatları, IFC, grafik hızı, Geometry Cleanup | R1 |
| AutoCAD LT | 2026 (2026.1.2); 2027 yardım sayfaları da yayında | Toolset yok (bkz. §1E notu) | A12, A16 |
| BricsCAD | V27 ("available now"); ayrıntılı liste V26/V26.2 için | Lite · Pro · Mechanical · BIM · Ultimate | B1, B2, B3 |
| ZWCAD | 2026 (basın bülteni 8 Mayıs 2025); **2027 Beta** (resmî yerel sayfa başlıkları) | Standard · Professional · MFG | Z1, Z2, Z5 |
| GstarCAD | 2027 (resmî blog 11 Haziran 2026); 2026 | GstarCAD Architecture ayrı ürün; sayfada sürüm numarası yok | G1, G2, G3 |
| progeCAD Professional | 2027 (IntelliCAD 14.1 motoru); 2026 (IntelliCAD 13.1) | Mimari AEC eklentisi Professional'a ücretsiz dahil | P1, P2, P3 |
| IntelliCAD (ITC motoru) | 14.1 (readme: 10.12.2025); 14.0 (31.07.2025) | progeCAD, CMS IntelliCAD vb. bu motoru kullanır | P4, P5 |
| DraftSight | 2026 (duyuru 7 Kasım 2025) | BIM modülü: Premium, Enterprise Plus, Collaborative Designer | D1 |
| nanoCAD | 26 (Mart 2026) | Modüller: Construction, 3D, Raster, Mechanica, Topoplan | N1, N2 |

---

## 1. Özellik × ürün matrisi

> §1A–§1D'de "AutoCAD" sütunu çekirdek AutoCAD'i kapsar; ACA (Architecture toolset) aynı çekirdeğin üstündedir.
> §1A'daki AutoCAD/LT kaynakları LT 2026 yardım sayfalarıdır; AutoCAD için aynı konu sayfası ayrıca açılmadı.

### 1A. Kesin girdi

| Özellik | AutoCAD 2026/27 | LT 2026 | BricsCAD V26/27 | ZWCAD 2026 | GstarCAD 2026/27 | progeCAD 2027 · IntelliCAD 14.1 | DraftSight 2026 | nanoCAD 26 |
|---|---|---|---|---|---|---|---|---|
| Nesne kenetleme | ✔ A14 | ✔ A14 | ✔ ESNAP; QuickDraw xref çizgilerine de kenetlenir B5 | (✔) | (✔) | ✔ kenetlemeli çizgi %63'e kadar hızlı P4; kenetleme hızlandı P1 | (✔) | ✔ "smarter snaps" N1 |
| Kenetleme izleme (hizalama yolları) | ✔ edinilen nokta "+", 0/90/180/270° ya da polar açılar; F11 A14 | ✔ A14 | (✔) | (✔) | (✔) | (✔) | (✔) | ? |
| Polar izleme · Ortho | ✔ F10; Ortho (F8) ile birbirini dışlar A14 | ✔ A14 | (✔) | (✔) | (✔) | ◐ ISODRAFT 30/90/150° izometrik düzlemler (2027) P1 | (✔) | ? |
| Dinamik girdi (imleçte mesafe/açı, Tab) | ✔ F12; LINE/PLINE/ARC/CIRCLE/ELLIPSE ikinci noktada A13 | ✔ A13 | ✔ DYN alanları + Tab B5; V27 "Dynamic input at cursor" (yeni) B2 | (✔) | ◐ 2027: komut/metin arası otomatik giriş yöntemi geçişi G2 | (✔) | (✔) | ✔ imleçte ölçü girme N2 |
| FROM · geçici izleme noktası (TK) | ✔ A15 | ✔ A15 | ? | ? | ? | ? | ? | ? |
| Doğrudan mesafe girme | ✔ (izleme geçici tuşu basılıyken kapalı) A14 | ✔ A14 | (✔) | (✔) | (✔) | (✔) | (✔) | (✔) |
| Geometri ölçme (MEASUREGEOM) | (✔) | (✔) | ✔ V26: mesafe, yarıçap, açı, alan, hacim B1 | ✔ 2026: hacim, toplam alan/uzunluk Z2 | ? | ? | ? | ? |

### 1B. Düzenleme ve seçim

| Özellik | AutoCAD 2026/27 | LT 2026 | BricsCAD V26/27 | ZWCAD 2026 | GstarCAD 2026/27 | progeCAD 2027 · IntelliCAD 14.1 | DraftSight 2026 | nanoCAD 26 |
|---|---|---|---|---|---|---|---|---|
| Çok işlevli tutamaçlar | ✔ 2027.1: polyline tutamaç menüsü yenilendi, uçlara "Lengthen" A1 | (✔) | ✔ V26.2 tutamaç/nesne türüne göre bağlam menüsü (yalnız Windows) B1 | ✔ Dimension Grip Menu (2026) Z1 | ? | ✔ ölçü tutamacı bağlam menüsü (2027) P1 | ? | ◐ ölçü tutamacı ilişkiyi koparır; spline noktaları Ctrl ile N2 |
| OFFSET · FILLET/CHAMFER · TRIM/EXTEND | (✔) | (✔) | (✔) | (✔) | ✔ TRIM/EXTEND hızlandı (2026) G1 | ✔ OFFSET "Multiple"; Shift ile sıfır yarıçap FILLET/CHAMFER (2027) P1 | (✔) | ◐ iki yönde uzatma, iki noktayla kırma N2 |
| STRETCH · ALIGN | (✔) | (✔) | (✔) | (✔) | (✔) | ◐ dinamik blok Stretch eyleminde CPolygon (2027) P1 | (✔) | (✔) |
| İlişkisel dizi | ✔ 2027.1 yerinde dizi düzenleme hızı A1 | (✔) | ✔ ARRAYDETECT: tekrarlı deseni dizi tanır B7 | (✔) | (✔) | (✔) | (✔) | (✔) |
| Özellik eşleştirme (MATCHPROP) | (✔) | (✔) | ✔ BIM verisini de taşır (V26) B1 | ◐ Smart Match: döndürülmüş/ölçeklenmiş eşleri bulup toplu düzenleme (Pro) Z1, Z4 | ? | ? | ? | ? |
| Hızlı / özellik tabanlı seçim | ✔ 2027 Autodesk Assistant: doğal dille seçim A2 | ? | ✔ V26.2 kaydedilebilir özellik sorgusu B1 | ✔ Smart Select (SMARTSEL), Select Menu, Chain Select, Add Select Z2, Z3 | ? | ◐ Glow seçim önizleme (2027) P1; seçim döngüsü (14.1) P4 | ? | ? |
| Parametrik kısıtlar (geometrik/boyutsal) | ✔ (LT'de yok) A16 | ✖ kısıt tanımlanamaz A16 | ◐ dinamik blokta BCPARAMETER deneysel, ABD dışı (V26.2) B1 | ◐ 2026 Parametric Design; Standard'da yeni kısıt yok Z4 | ✔ 2026 boyutsal kısıt + Parameters Manager G1 | ? | ? | ? |
| Geometri temizliği / sağlık | ✔ 2027 Geometry Cleanup (kopuk nesne, boşluk) A4 | ? | ✔ V26.2 Health Advisor bildirimi B1 | ? | ✔ BATPURGE toplu temizlik (2026) G1 | ✔ OVERKILL: ışın, metin, tarama, ölçü, mleader (2027) P1, P4 | ? | ? |

### 1C. Açıklama (annotation)

| Özellik | AutoCAD 2026/27 | LT 2026 | BricsCAD V26/27 | ZWCAD 2026 | GstarCAD 2026/27 | progeCAD 2027 · IntelliCAD 14.1 | DraftSight 2026 | nanoCAD 26 |
|---|---|---|---|---|---|---|---|---|
| Annotative ölçekleme | ✔ CANNOSCALE; görünüm penceresi ölçeğiyle eşleme (ACA 2027 yardımı) R14 | (✔) | (✔) | (✔) | (✔) | (✔) | (✔) | ? |
| Ölçü stilleri / hızlı ölçü | ✔ ölçü stili CAD standardında denetlenir A19 | (✔) | ✔ QDIM Advanced: tek komutta çok yönlü zincir ölçü, AI destekli (V26.2) B1 | ✔ Smart Dimension: nesne türüne göre ölçü (2026) Z1 | ✔ DIMCORD koordinat ölçüsü yenilendi (2026) G1 | ✔ ELEVTEXT kot yazısı (2027) P1 | ? | ◐ başka CAD'den gelen ölçü/mleader ilişkisi korunur N2 |
| Çoklu kılavuz (MLEADER) | ✔ mleader stili CAD standardında A19 | (✔) | (✔) | (✔) | (✔) | ✔ OVERKILL mleader; LISP CAutoMLeader (14.1) P4 | (✔) | ✔ N2 |
| Yazı stilleri | ✔ A19 | (✔) | ◐ PDF'teki SHX metni düzenlenebilir metne (PDFSHXTEXT, AI) B1 | (✔) | ◐ PDF içe aktarmada metin → MText (2026) G1 | ✔ Unicode OTF; MTEXT otomatik yığma aç/kapa; TEXTORIENTATION (2027) P1 | (✔) | ✔ "comprehensive text tools" N1 |
| Tablolar | ✔ TABLELAYER (2026.1) A6 | ✔ TABLELAYER A12 | (✔) | ✔ Area Table (tüm sürümler) Z4 | ? | ✔ hücreleri blokla otomatik doldurma (2027 / 14.1) P1, P4 | ✔ DATAEXTRACTION: CSV başlık, formül sütunu D1 | ✔ Excel tarzı editör, formül/makro, öznitelikten tablo N2 |
| Alanlar (fields) | (✔) | (✔) | (✔) | (✔) | (✔) | (✔) | ✔ Field + DIESEL ifadeleri (2026) D1 | ? |
| Tarama / degrade | (✔) | (✔) | (✔) | (✔) | ✔ 2027 "Use Current Drawing": görünüm dışı sınır arama G2 | ✔ AEC nesnelerinde kesit dolgu ayarı (2026, arama özeti) P2 | ? (degrade yalnız bayi kaynağında) | ✔ araç paletinde tarama N2 |
| Revizyon bulutu | ✔ REVCLOUDLAYER, REVCLOUDSCALEMODE (2026.1) A6 | ✔ A12 | (✔) | (✔) | ✔ DWG Compare farkları bulutla işaretler (2026) G1 | ✔ varsayılan yay boyu görünüm boyutundan (2027) P1 | ? | ? |
| Wipeout | (✔) | (✔) | (✔) | (✔) | (✔) | (✔) | (✔) | ? |
| Merkez çizgisi / işareti | ✔ CENTERLINE, CENTERMARK + CENTERLAYER (2026) A6 | ✔ A12 | ? | ? | ✔ ilişkisel merkez çizgisi/işareti (2027) G2 | ? | ? | ? |

### 1D. Düzen, standartlar, çıktı

| Özellik | AutoCAD 2026/27 | LT 2026 | BricsCAD V26/27 | ZWCAD 2026 | GstarCAD 2026/27 | progeCAD 2027 · IntelliCAD 14.1 | DraftSight 2026 | nanoCAD 26 |
|---|---|---|---|---|---|---|---|---|
| Katman + katman durumları | (✔) | (✔) | (✔) | ✔ Toolbox'ta katman araçları Z1 | (✔) | (✔) | (✔) | ✔ "advanced layer controls" N1 |
| Nesne türüne göre otomatik katman | ✔ REVCLOUD/TABLE/VIEWPORT/CENTERLAYER A6; ACA Layer Key Styles R2 | ✔ A12 | ✔ BIM_SECTIONS, BIM_VIEWPORTS katmanları B9 | ? | ✔ Architecture: katman standartları özelleştirilebilir G3 | ? | ? | ? |
| CAD standart denetimi (.dws) | ✔ katman, ölçü stili, çizgi tipi, mleader, yazı stili A19 | ? | ? | ? | ? | ? | ? | ? |
| AIA / ISO 13567 katman standardı | ◐ ACA'da Layer Standards + Layer Key Styles var; AIA/ISO 13567 ayrıntısı doğrulanamadı R2 | ? | ? | ? | ? | ? | ? | ? |
| Bloklar + öznitelikler | ✔ 2027.1 öznitelik kutularında Ctrl+A A1 | (✔) | (✔) | (✔) | (✔) | ✔ Quick Properties'te blok öznitelikleri (2027) P1 | ? | ✔ öznitelikten tablo/şartname N2 |
| Dinamik bloklar | ✔ parametre + eylem + kısıt A16 | ◐ parametre/eylem var, kısıt yok A16, A17 | ◐ V26.2 ekleme ve palet özellikleri; BEDIT, eylem düzenleme, arama tablosu **deneysel, ABD dışı** B1 | ✔ Flexiblock tüm sürümler; Blok Editörü'nde kısıt yalnız Pro Z4 | ✔ dinamik blokta parametrik kısıt (2027) G2 | ◐ "Advanced (Dynamic) Blocks", BTESTBLOCK deneme penceresi P1, P4 | ? | ? |
| Araç paletleri | ✔ 2027: proje genelinde paylaşılan paletler A1 | (✔) | ✔ V27 model uzayından özelleştirme B2 | ◐ ZWCAD Toolbox (farklı kavram) Z1 | ? | ✔ görsel, kılavuz, xref destekli (2027 / 14.1) P1, P4 | ? | ✔ kurum standardı blok/tarama/betik paletleri N2 |
| İçerik tarayıcı / kütüphane | (✔) DesignCenter; ACA Content Browser R2 | (✔) | ? | ✔ Similar Search: dış dosyalarda benzer blok (Pro) Z1, Z4 | ✔ mobilya/peyzaj/vitrifiye kütüphanesi G3 | ✔ iCADLib 22.000 blok P3 | ? | ◐ Construction: parametrik standart parça kütüphanesi N2 |
| Dış başvurular (xref) | ✔ 2027 Connected References: kopuk yolu bul/onar A1 | (✔) | (✔) | (✔) | ✔ AUTOMERGE: xref ile böl/birleştir (2026) G1 | (✔) | ✔ 3DEXPERIENCE'tan DWG/PDF ekleme D1 | ✔ yeniden yüklemeden güncelleme bildirimi N2 |
| Pafta seti yöneticisi (SSM) | ✔ 2026.1.1 bulut pafta seti; 2027.1 çoklu pafta özelliği düzenleme A6, A1 | ✔ A12, A18 | ✔ BIM projesinde SheetSet yolları otomatik B9 | ✔ tüm sürümler Z4 | ? | ? | ✔ platformda SSM (Premium, Mechanical, CD) D1 | ? |
| Layout / görünüm pencereleri | ✔ 2027 layout geçişi hızlandı; VIEWPORTLAYER A1, A6 | ✔ A12 | (✔) | (✔) | ✔ layout geçişi hızlandı (2026) G1 | ✔ yüzen çizim pencereleri (2027) P1 | ✔ model uzayında boyutlanabilir görünüm karoları D1 | (✔) |
| CTB/STB kalem tabloları | (✔) (2026: GDI çizim .PC3 ayarı A6) | (✔) | (✔) | (✔) | (✔) | (✔) | (✔) | (✔) |
| Toplu çizim / yayın / paketleme | (✔) Publish, eTransmit | (✔) | (✔) | ✔ Smart Plot: çerçeve algılama, kâğıt eşleme, toplu PDF/DWF (tüm sürümler) Z3, Z4 | ✔ sıralı önizleme (2026), sayfa/set sıralı toplu çizim (2027) G1, G2 | ✔ PDF'e jeo-koordinat (2027) P1 | ✔ toplu baskı (Premium, Mechanical, CD) D1 | ✔ çok sayfalı toplu çizim N2 |
| Çizim karşılaştırma | ✔ Activity Insights "What's Changed" (2026) A9 | ✔ Activity Insights süzgeçleri A12 | ? | ✔ File Compare (tüm sürümler) Z4 | ✔ DWG Compare (2026) G1 | ? | ? | ✔ Drawing Comparison N2 |

### 1E. Mimari dikeyler

> **LT notu (✖\*):** Autodesk ürün sayfası Architecture toolset'i "included with AutoCAD 2027" diye tanımlıyor (A20,
> sayfa başlığı; içerik 403). LT için toolset ifadesi bulunmadı; LT'de mimari nesne yok kabul edildi.

| Özellik | ACA toolset 2026/27 | LT | BricsCAD BIM V26/27 | ZWCAD 2026 (Pro / Architecture) | GstarCAD Architecture | progeCAD AEC · IntelliCAD 14.1 | DraftSight 2026 (BIM modülü) | nanoCAD 26 (Construction) |
|---|---|---|---|---|---|---|---|---|
| Duvar nesnesi + stil (bileşen katmanları) | ✔ R2, R5 | ✖\* | ✔ QuickDraw: duvar genişliği/yüksekliği; 2B paralel çizgiden kalınlık B5 | ? | ✔ eksen ızgarasından toplu duvar G3 | ✔ AEC duvar P3; çizerken duvar çevirme (2027) P1 | ✖ yalnız RVT/IFC'den 2B üretim D1 | ✔ duvar + aks (eğitim 1) N5 |
| Duvar kesişim temizleme (öncelik, grup) | ✔ bileşen önceliği (düşük sayı = üstün), cleanup group R5, R6 | ✖\* | ◐ komşu oda duvarları "Auto adopt sizes"; kural ayrıntısı doğrulanamadı B5 | ? | ? | ? | ✖ | ? |
| Kapı / pencere / açıklık (duvara bağlı) | ✔ R2; perdede kapı/pencere montajı R11 | ✖\* | ✔ BIMWINDOWCREATE: kapalı konturdan parametrik pencere/açıklık B11 | ? | ✔ parametrik; çift tık / tutamaç G3 | ✔ Cycle Measure To (sol/orta/sağ), AECANCHOREDIT (2027 / 14.1) P1, P4 | ✖ | ✔ (eğitim 2) N5; SPLISTOFOPENINGS N4 |
| Giydirme cephe | ✔ ızgara, iç içe ızgara, 4 öğe tipi, düz/eğri R11 | ✖\* | ? | ? | ? | ✔ (progeCAD) P3 | ✖ | ? |
| Mahal (space) + alan hesabı | ✔ base/net/usable/gross sınırı, hesap düzenleyiciler R8 | ✖\* | ✔ BIMSPACE, sınırlayıcı öğe, BIMUPDATESPACE B10 | ◐ Area Table (tüm sürümler) Z4 | ✔ konut/kamu başvurusu için alan G3 | ◐ 14.1'de mahal tutamaçları (CMS, arama özeti) P6 | ? | ✔ mahal etiketi + mahal tablosu N5 |
| Merdiven / korkuluk | ✔ R2 | ✖\* | ? | ? | ✔ merdiven + korkuluk G3 | ✔ P3 | ✖ | ✔ SPSTAIRADD: düz ve yay parçalı N3 |
| Döşeme / çatı | ✔ slab, roof slab (ayrı stil kategorisi) R13 | ✖\* | ✔ QuickDraw: döşeme, kat kopyala, düz çatı B5 | ? | ✔ çok çatı tipi, plak G3 | ✔ slab, roof, roof slab P2, P3 | ✖ | ? |
| Taşıyıcı eleman | ✔ kiriş/kolon/çapraz, trim plane; 2027 başlangıç-bitiş koordinatı R12, R1 | ✖\* | ? | ? | ✔ kolon/kiriş/plak G3 | ? | ✖ | ? |
| Görünüm yönüne / ölçeğe göre gösterim | ✔ display configuration → set → representation; Top=Plan, Front=Section_Elev, diğer=Model R3, R4; Multi-View Block R2 | ✖\* | ◐ "Typed plans": katmandan bağımsız grafik B9 | ? | ◐ plandaki öğeler 3B'de eşzamanlı G3 | ? | ✖ | ? |
| Tematik renklendirme (renkli plan) | ✔ Display Themes R2 | ✖\* | ? | ? | ? | ? | ? | ? |
| Çizelge + etiket | ✔ schedule tag + schedule table + property set R2 | ✖\* | ◐ BIM etiketleri var; kesit güncellemesi etiketi otomatik koymaz B9 | ◐ Data Extraction (2026 yeniden yapılandı) Z2 | ✔ kapı-pencere listesi tek tuşla G3 | ◐ tabloyu blokla doldurma P1 | ◐ DATAEXTRACTION D1 | ✔ mahal/kapı/pencere tabloları N5 |
| Kesit / görünüş üretimi | ✔ Annotate > Callouts: bina kesiti, görünüşü R2 | ✖\* | ✔ BIMSECTION + BIMSECTIONUPDATE, senkron işareti, kesit göstergesi B9 | ◐ Section View: 3B/RVT/IFC'de gezinme (Pro) Z2 | ◐ plan ve kesit/görünüş çizgisinden (sayfa ifadesi belirsiz) G3 | ◐ Section Plane'de AEC dolgu (2026) P2 | ✔ RVT/IFC'den plan, kesit, görünüş D1 | ? |
| Tadilat modu (mevcut/yıkılacak/yeni) | ✔ R7 | ✖\* | ? | ? | ? | ? | ✖ | ? |
| Detay bileşen yöneticisi + keynote | ✔ R9 | ✖\* | ◐ BIMPROPAGATE: detayı benzer bağlantılara yayma B8 | ? | ? | ? | ✖ | ? |
| Proje gezgini | ✔ Project Navigator R10 | ✖\* | ✔ BIM Project Browser (Sheets segmenti) B9 | ◐ yalnız SSM Z4 | ? | ? | ◐ 3DEXPERIENCE'ta SSM D1 | ? |
| AEC ölçüleri | ✔ AEC Dimension R2 | ✖\* | ◐ QDIM Advanced (genel) B1 | ◐ Smart Dimension (genel) Z1 | ✔ kapı, pencere, duvar, merdiven için özel ölçü G3 | ? | ✖ | ? |
| Kot / aks / kesit sembolleri | (✔) | ✖\* | ✔ kesit göstergesi, kat ve aks balonları B9 | ? | ✔ kot işareti, gösterge, kuzey oku, bulut G3 | ✔ ELEVTEXT (2027) P1 | ? | ? |
| IFC / RVT | ✔ IFC iyileştirmeleri (2027) R1 | ? | ✔ IFC4x3, IFC dışa aktarma diyaloğu (V26.2), IFC referans (V26) B1 | ✔ RVT 2015–2025, IFC (Pro) Z2, Z4 | ? | ✔ Revit/IFC içe, IFC dışa P3; IFC içe aktarma sağlamlaştı (14.1) P4 | ✔ RVT/IFC → 2B D1 | ✔ IFC içe aktarma N2 |

### 1F. Pratik hızlandırıcılar ve yapay zekâ

| Özellik | AutoCAD 2026/27 | LT 2026 | BricsCAD V26/27 | ZWCAD 2026 | GstarCAD 2026/27 | progeCAD 2027 · IntelliCAD 14.1 | DraftSight 2026 | nanoCAD 26 |
|---|---|---|---|---|---|---|---|---|
| Akıllı blok yerleştirme (Placement) | ✔ 2024+: önceki yerleşime göre öneri, AUTOPLACEMENT, Shift+W bastırır A11 | ? | ? | ? | ? | ? | ? | ? |
| Blok değiştirme önerisi (Replacement) | ✔ ML önerisi / son kullanılan / elle (arama özeti) A21 | ? | ? | ? | ? | ? | ? | ? |
| Tekrarlı geometriyi bloğa çevirme | ✔ Search and Convert (metin farkları dahil), Detect and Convert (2026.1.1'de Tech Preview değil) A6, A7, A8 | ✔ Search and Convert A12 | ✔ BLOCKIFY (Lite/Pro/Mech/BIM; 3B katı yöntemi Lite'ta yok) B7 | ◐ Smart Match + Similar Search (Pro) Z1, Z4 | ? | ? | ? | ? |
| Hızlı plan (oda sürükle → duvar + döşeme) | ? | ✖\* | ✔ BIM QuickDraw (BIM, Ultimate) B4, B5 | ? | ? | ? | ✖ | ? |
| Detay yayma | ? | ✖\* | ✔ BIMPROPAGATE (BIM) B8 | ? | ? | ? | ? | ? |
| Bağlamsal imleç menüsü / jest | ? | ? | ✔ Quad (rollover özellikleri); V27 marking menu deneysel B6, B2 | ✔ Smart Mouse jestleri (SMARTMOUSE) Z3 | ◐ Architecture sağ-tık menüsü G3 | ? | ? | ? |
| İşaretlemeleri içe alma | ✔ Markup Import & Assist (PDF/JPG/PNG; metin, üstü çizili, sınır → bulut) A6, A10 | ? | ◐ PDFSHXTEXT (AI) B1 | ◐ Smart Voice sesli not (VOICEMAN) Z3 | ◐ PDFium içe aktarma (2026) G1 | ✔ PDF2CAD yenilendi (2027) P1 | ? | ? |
| Yapay zekâ asistanı | ✔ Autodesk Assistant (2027): standart denetimi, doğal dille seçim, sorgu; 2027.1 sayım, birim A2, A1 | ? | ✔ BricsCAD Assist (V26.2), V27'de geliştirildi B1, B2 | ? (resmî listede yok) | ? ("Smarter by AI" sloganı, somut özellik yok) G2 | ? | ? | ? |
| Çok kullanıcılı düzenleme | ✔ Checkout (2027) A1, A5 | ? | ◐ ACC / ProjectWise bağlantısı (BIM, V26.2) B1 | ? | ? | ? | ◐ 3DEXPERIENCE Collaborative Space D1 | ? |
| Yerel dilde komut takma adları | ? | ? | ? | ? | ? | ? | ? | ✔ native command aliases (26) N1 |
| Aranabilir ayarlar | ? | ? | ✔ V27: aranabilir, yalnız değişenleri gösterir (Pro) B2 | ? | ? | ? | ? | ? |
| Yüzen çizim pencereleri | ? | ? | ? | ? | ✔ (2027) G2 | ✔ (2027) P1 | ✔ (2026) D1 | ? |

---

## 2. Özellik grupları: ne işe yarar, kullanıcı nasıl kullanır (+ ekran görüntüleri)

### 2.1 Kesin girdi

**Ne işe yarar:** Mimari çizimde her nokta bir ölçüye bağlıdır: duvar ekseni, kapı kenarından 10 cm, kolon yüzüne
hizalı nokta. Kenetleme, kenetleme izleme ve polar izleme fareyle gösterilen noktayı geometrik bir kurala bağlar;
dinamik girdi ve doğrudan mesafe girme ise sayıyı imlecin yanında klavyeden alır.

**Kullanıcı nasıl kullanır:** AutoCAD/LT'de kullanıcı bir uç noktanın üstünde bekleyip onu "edinir" (küçük +),
imleci yatay, dikey ya da polar açıda kaydırınca kesikli hizalama yolu çıkar; yol üstünde mesafe yazıp Enter'a basar
(A14). Başka bir noktaya göre ofsetli nokta için `FROM` (taban noktası + göreli koordinat) ya da `TK` (bir dizi geçici
nokta) kullanılır (A15). Dinamik girdide ikinci nokta istenirken imleç yanındaki uzunluk ve açı kutuları arasında Tab
ile geçilir (A13). BricsCAD V27 dinamik girdi bilgisini imlecin hemen üstüne taşıdı (B2); BIM QuickDraw'da oda eni ve
boyu aynı DYN alanlarına yazılır (B5).

Ekran görüntüleri:
- BricsCAD V27 — Dynamic input at cursor: https://images.ctfassets.net/26961o1141cc/1K7kZ7DM0D0Rwp2llTsCmO/eac4ccef8d4de487d6344636f06fe6a8/Dynamic_input_at_cursor.png?w=2434&h=1368&fm=webp&q=100
- nanoCAD 26 — Dynamic input: https://nanocad.com/upload/iblock/1e9/3qxzzpbctb3f3zgel6fbs26uvq7ew2az/Dynamic%20Input%20(367x206).png

### 2.2 Düzenleme ve seçim

**Ne işe yarar:** Mimari çizimin çoğu yeni çizgi değil, var olanı düzeltmektir. Çok işlevli tutamaçlar, STRETCH,
OFFSET, FILLET/CHAMFER, TRIM/EXTEND, ALIGN, ilişkisel dizi ve MATCHPROP duvar, aks ve mobilya düzenini birkaç tıkla
değiştirir; özellik tabanlı seçim "bütün sarı çizgiler" ya da "bu katmandaki bütün yazılar" gibi kümeleri tek seferde
yakalar. Temizlik araçları dışarıdan gelen kirli DWG'yi (çakışık çizgi, kopuk uç) mimari nesneye dönüştürülebilir hâle
getirir.

**Kullanıcı nasıl kullanır:** AutoCAD 2027.1'de polyline uç tutamacının üstünde durunca açılan menü yenilendi ve
"Lengthen" eklendi (A1). progeCAD 2027'de Shift basılıyken FILLET/CHAMFER köşeyi sıfır yarıçapla kapatır, OFFSET
"Multiple" aynı mesafeyi tekrarlar (P1). ZWCAD `SMARTSEL` paneli tür, renk, katman, çizgi tipine göre süzüp seçimi
anında önizler; Chain Select bağlı gevşek çizgileri tek seferde seçer (Z2, Z3). BricsCAD V26.2 seçim sorgularını
kaydedip yeniden kullanmayı sağlar (B1); AutoCAD 2027 Autodesk Assistant doğal dille seçim yapar (A2). AutoCAD 2027
Geometry Cleanup kopuk nesneleri ve boşlukları tarayıp düzeltir (A4); progeCAD/IntelliCAD OVERKILL ölçü ve mleader'ı da
kapsar (P1, P4); BricsCAD Health Advisor arka planda öneri verir (B1).

Ekran görüntüleri:
- ZWCAD Smart Select (sarı nesneleri seçme örneği): https://statics.zwsoft.com/upload/default/20250723/edca044d1a522e7dc90f5be59ae19711.png
- ZWCAD 2026 Dimension Grip Menu: https://statics.zwsoft.com/upload/default/20250324/23d39aa9b8240ffbf420f22e9a5e5aa5.png
- AutoCAD 2027 Geometry Cleanup: https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-WhatsNew/images/GUID-AAE1A56F-6235-4C91-A4EA-F9620F940416.png

### 2.3 Açıklama (yazı, ölçü, tablo, tarama, bulut)

**Ne işe yarar:** Aynı model 1/200 vaziyet, 1/100 uygulama, 1/50 detay paftasına basılır. Annotative nesneler (yazı,
ölçü, tarama, etiket) her ölçekte kâğıtta aynı boyda çıkar; ölçü/yazı/mleader stilleri kurum standardını taşır;
tablolar, alanlar ve veri çıkarma metrajı ve pafta künyesini çizimden üretir; revizyon bulutu ve wipeout revizyon ile
okunurluk içindir.

**Kullanıcı nasıl kullanır:** Açıklama ölçeği model sekmesi ve her görünüm penceresiyle ilişkilidir (`CANNOSCALE`);
kullanıcı durum çubuğundan ölçeği seçer, annotative nesne o ölçeğin temsiline sahipse ona göre çizilir (R14). AutoCAD
2026.1 yeni revizyon bulutunu, tabloyu ve görünüm penceresini kendi katmanına koyar (`REVCLOUDLAYER`, `TABLELAYER`,
`VIEWPORTLAYER`) ve bulut yay boyunu DIMSCALE'e göre ölçekler (`REVCLOUDSCALEMODE`) (A6). BricsCAD V26.2 QDIM Advanced
tek komutta çok yönlü zincir ölçü atar (B1); ZWCAD 2026 Smart Dimension nesne türünü algılayıp doğru ölçü tipini seçer,
Dimension Grip Menu tutamaçtan ok çevirme, baseline başlatma ve metin konumu verir (Z1). DraftSight 2026 Field komutuna
DIESEL ifadeleri ekledi (D1); nanoCAD tablo editörü Excel benzeri formül destekler ve blok özniteliğinden tablo üretir
(N2).

Ekran görüntüleri:
- BricsCAD V26.2 QDIM Advanced: https://images.ctfassets.net/26961o1141cc/5AqXvMwfzl6pFSQOHhHoLS/237ff65bef19ebee47d0dfe428f4132f/QDIM_Advanced_mode_to_reduce_dimensioning_time.png?w=2434&h=1368&fm=webp&q=100
- GstarCAD 2026 DWG Compare (farklar revizyon bulutuyla): https://blog.gstarcad.net/wp-content/uploads/2025/03/DWG-compare.jpeg

### 2.4 Düzen ve standartlar (katman, blok, palet, xref, pafta seti, çıktı)

**Ne işe yarar:** Katman, katman durumu ve kalem tablosu neyin hangi kalınlıkta ve renkte basılacağını belirler.
Bloklar, dinamik bloklar ve öznitelikler tekrar eden içerikleri (mobilya, vitrifiye, künye) tek tanımdan yönetir.
Araç paletleri, içerik tarayıcı, xref ve pafta seti büyük projeyi dosyalara bölüp tutarlı tutar; CAD standart denetimi
kurum standardından sapmayı yakalar.

**Kullanıcı nasıl kullanır:** AutoCAD'de çizime `.dws` standart dosyası bağlanır, denetim katmanları, ölçü stillerini,
çizgi tiplerini, mleader ve yazı stillerini karşılaştırıp düzeltme önerir (A19). ACA'da "Layer Key Styles" nesne
türüne göre katmanı kendisi seçer (R2). Dinamik blokta kullanıcı Block Editor'da parametre ve eylem ekler; LT de yapar
ama kısıt tanımlayamaz (A16, A17). ZWCAD'in karşılığı Flexiblock'tur (tüm sürümler; Blok Editörü'nde kısıt yalnız Pro,
Z4); GstarCAD 2027 dinamik bloğa parametrik kısıt ekledi (G2); BricsCAD V26.2'de dinamik blok yazımı (BEDIT, eylem
düzenleme, arama tablosu) deneysel ve ABD dışı kullanıcılara açık (B1). Pafta setinde AutoCAD 2026.1.1 bulut pafta
seti ve eşzamanlı düzenleme uyarısı getirdi, 2027.1 birden çok paftanın ortak özelliklerini topluca düzenletiyor (A6,
A1). Çıktıda ZWCAD Smart Plot model uzayındaki pafta çerçevelerini katman ya da blokla tanır, kâğıt boyunu eşler ve
birden çok dosyayı toplu basar (Z3).

Ekran görüntüleri:
- AutoCAD 2026 Smart Blocks: Search and Convert: https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-WhatsNew/images/GUID-0B1C9EC6-C883-4051-850D-D98EF96F5BE1.png
- BricsCAD V26.2 dinamik blok arama tabloları (deneysel): https://images.ctfassets.net/26961o1141cc/MekuVmFRrHssQO8MXDQv0/eca69778165f468f42a1714887d584a1/Lookup_tables_for_Dynamic_Blocks.png?w=2434&h=1368&fm=webp&q=100
- ZWCAD Smart Batch Plot — çerçeve stili seçimi: https://statics.zwsoft.com/upload/default/20250723/cf4ddca8040d23032457b794c22a2386.png
- GstarCAD 2027 dinamik blok parametrik kısıtları: https://i0.wp.com/blog.gstarcad.net/wp-content/uploads/2026/06/image-1.png?resize=1024%2C673&ssl=1

### 2.5 Mimari dikeyler

**Duvar stili ve temizleme.** ACA duvarı katmanlı bileşenlerden kurar; her bileşenin öncelik numarası vardır ve
**düşük numara yüksek önceliktir**: kesişimde düşük numaralı bileşen ötekini keser. ACA 2027 varsayılanları: Concrete
200 · CMU 300 · Precast Panel 400 · Rigid Insulation 404 · Stud 500 · Insulation 600 · Air Gap 700 · Brick 800 ·
Siding 900 · Metal Panel 1000 · Stucco 1100 · GWB / Glass 1200 · Casework 2000–2030 · Toilet Partition 3000 (R5).
Cleanup group tanımları hangi duvarların birbiriyle birleşeceğini (xref'ler dahil) belirler ve Style Manager'da
yönetilir (R6). BricsCAD QuickDraw'da duvar genişliği, yüksekliği ve döşeme kalınlığı komut panelinde ayarlanır;
"Auto adopt sizes" açıkken yeni oda yaslandığı duvarın ölçüsünü alır (B5).

**Kapı, pencere, açıklık.** ACA'da kapı/pencere blok değil, duvara bağlı nesnedir (R2). progeCAD 2027 / IntelliCAD
14.1'de yerleştirirken ölçü sol kenar, orta ve sağ kenar arasında döngüyle değişir (Cycle Measure To; varsayılanı
`AECDEFAULTMEASURETO`), `AECANCHOREDIT` ise açıklığın duvara göre konum, ölçek, ayna ve yön çapasını X/Y/Z'de ayarlar
(P1, P4). BricsCAD `BIMWINDOWCREATE` herhangi bir kapalı konturdan (polyline, sınır, ızgara) parametrik pencere ya da
açıklık üretir (B11).

**Giydirme cephe.** ACA curtain wall bir ızgaradır: her ızgara yatayda ya da düşeyde bölünür, ızgaralar iç içe olabilir,
dört öğe tipi vardır; hücrelere kapı, pencere ve kapı/pencere montajları konur; düz, eğri ya da karışık olabilir (R11).

**Mahal ve alan hesabı.** ACA space nesnesinin dört sınırı vardır: base, net, usable, gross. Hesap düzenleyiciler
uygulanmış "Calculated Area", çevre ve düzenleyici değerleri özellik setine akar, oradan etikete ve çizelgeye gider (R8).
BricsCAD `BIMSPACE` duvarlarla kapalı bölgeye tıklanınca mahal üretir; duvarsız bölgeler (otopark yeri) için çizgi ya da
tarama "space bounding" sınıflanabilir; `BIMUPDATESPACE` güncel olmayan mahali yeniden hesaplar (B10). GstarCAD
Architecture konut ve kamu yapısı başvurusu için alan hesabı sunar (G3); nanoCAD Construction mahal etiketi ve
mahal/kapı/pencere tabloları üretir (N5); ZWCAD'de Area Table tüm sürümlerde var (Z4).

**Görünüm yönüne ve ölçeğe göre gösterim (display system).** ACA'da zincir şöyledir: display configuration → display set
→ display representation. Görünüm penceresine atanan konfigürasyon görünüm yönüne göre set seçer; "Medium Detail"
örneğinde Top → Plan, Front/Right/Left/Back → Section_Elev, diğer yönler → Model. Nesne aynı kalır, çizimi değişir;
ölçeğe uygun ayrıntı düzeyi görünüm penceresine atanan konfigürasyonla seçilir; Display Manager eşlemeyi gösterir (R3,
R4). Multi-View Block'lar aynı fikri mobilya ve donatıya taşır (R2). BricsCAD BIM'de "Typed plans" model grafiklerini
katman ve katman durumundan bağımsız özelleştirir (B9). *(Low/High Detail, Reflected gibi diğer konfigürasyon adları
bu çalışmada doğrulanmadı.)*

**Çizelge ve etiket.** ACA'da schedule tag ve schedule table'lar property set definition'lardan beslenir (öznitelik
yerine) (R2). GstarCAD Architecture kapı ve pencereleri tek eylemle sayıp listeyi çizime ekler (G3).

**Kesit ve görünüş.** ACA Annotate > Callouts ile bina kesiti ve görünüşü üretir (R2). BricsCAD BIM'de `BIMSECTION`
kesit nesnesini koyar, `BIMSECTIONUPDATE` sonucu yeni çizime üretir; Project Browser'dan "Update Now / Update in
Background" yapılır; modelle senkron olmayan pafta görünümü **kırmızı**, hiç güncellenmemiş olan **turuncu**
işaretlenir; önceki sonuç donuk `BIM_SECTIONS_PREVIOUS` katmanında saklanır; kesit göstergesi, kat ve aks balonları
düzenlenebilir BIM etiketleridir (B9). DraftSight 2026 BIM modülü RVT/IFC'den plan, kesit ve görünüş üretir (D1).

**Tadilat modu.** ACA'da renovation mode açıkken eklenen nesne "New" olur, silinen nesne gerçekten silinmez "Demo"
kategorisine geçer; mevcut ve yeni duvarlar birbirini temizler, yıkılan ile yeni etkileşmez; yıkım planı mevcut +
yıkılanı, revizyon planı yeni + mevcudu gösterir, ayarlar Renovation Options'tadır (R7; arama özeti).

**Detay bileşenleri ve keynote.** Detail Component Manager detay bileşen veritabanlarını ağaç ve süzgeçle gezdirir; her
bileşenin Keynote alanı vardır; keynote veritabanları AecKeynotes ve AecKeynotes-Assemblies; ilişki değişirse önceden
konmuş keynote'lar kendiliğinden güncellenmez, yeniden keynote gerekir (R9).

**Proje gezgini.** Proje Project Browser'da seçilir, yapı ve dokümantasyon verisi Project Navigator'da yönetilir: sabit
kategori yapısı, construct / element / view / sheet; yeni proje kendi pafta setini alır; dosyalar Windows Gezgini'nden
taşınmamalıdır (R10). BricsCAD BIM Project Browser'ın Sheets segmenti üretilmiş paftaları listeler (B9).

**Taşıyıcı eleman, döşeme/çatı, merdiven, AEC ölçüsü.** ACA structural member bir profilin yol boyunca çekilmesidir
(kiriş, çapraz, kolon); birleşimler trim plane ile temizlenir; 2027'de başlangıç/bitiş koordinatları otomatik özellik
oldu ve Excel'e çıkar (R12, R1). Slab düzlemsel çokgen sınırlı 3B gövdedir; roof slab ayrı stil kategorisidir (R13).
nanoCAD Construction `SPSTAIRADD` düz ve yay parçalı merdiven çizer (N3). ACA "AEC Dimension" ölçüleri Annotate >
Dimensions'tadır (R2); GstarCAD Architecture kapı, pencere, duvar ve merdiven için özel ölçülendirme sunar (G3).

Ekran görüntüleri:
- ACA 2027 Display Manager (Medium Detail ve Standard): https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Architecture/images/GUID-8E268594-249D-4E84-9E67-06DB97E7204D.png
- ACA aynı model — Top/Plan: https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Architecture/images/GUID-4D1A52C0-B360-41AE-846B-2F2F83C64296.png
- ACA aynı model — Front/Elevation: https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Architecture/images/GUID-FA39CB01-5A28-44F2-9421-C63F3341CDF3.png
- ACA aynı model — SW Isometric/Model: https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Architecture/images/GUID-64970F1F-5868-4791-BD9F-C1DEC10C2FB4.png
- ACA 2027 duvar temizleme öncelikleri: https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Architecture/images/GUID-F1D7BE84-457B-4B64-9708-983B3B2D9A9B.png
- BricsCAD BIM — senkron dışı pafta işaretleri: https://static.bricsys.com/help/images/V27/en_US/GD_generatingdrawings_sheets.png
- BricsCAD BIM — kesit göstergeleri: https://static.bricsys.com/help/images/V27/en_INTL/Generating_Drawings_Section_Indicator.png
- GstarCAD Architecture — duvar/kapı/pencere: https://cdn-sg-gw.gstarcad.net/resource/images/jzgjd_12.jpg
- GstarCAD Architecture — mahal alanı: https://cdn-sg-gw.gstarcad.net/resource/images/zhushi_37.png
- GstarCAD Architecture — kapı-pencere tablosu: https://cdn-sg-gw.gstarcad.net/resource/images/zhushi_32.png
- DraftSight 2026 BIM modülü: https://blog-assets.solidworks.com/uploads/2025/12/picture1-4-1024x576.png.webp
- ACA 2027 taşıyıcı eleman koordinatları: https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Architecture-WhatsNew/images/GUID-70C78E28-B425-4530-9BBF-8F211403306C.png

### 2.6 Pratik hızlandırıcılar ve yapay zekâ

**BricsCAD BIM QuickDraw (BIM, Ultimate).** Kullanıcı `BIMQUICKDRAW`'ı başlatır, ilk köşeyi tıklar ve sürükler:
dikdörtgen oda duvar + döşemesiyle oluşur; en/boy DYN alanlarına yazılır, Tab ile geçilir. İmleç var olan binanın
içine konunca komşu oda çizilir ve alanlar çevre duvarlara uzaklığı gösterir. İç duvara kenetlenip dışarı sürüklenince
dış duvar parçası kırmızı olur, ikinci tıkla silinir ve L oda kalır. Binanın yanındaki simgeyle kat eklenir (tam kopya /
döşeme + dış duvar / düz çatı). İçe aktarılmış 2B planın dış duvar çizgilerinin üstünde gezinince duvar çizgilere
kenetlenir ve kalınlık paralel çizgi aralığından alınır; plan 3B'ye döner. Ctrl duvar yaslanmasını değiştirir. Komut
çıkılana kadar açık kalır (B4, B5).

**Quad imleç (BricsCAD).** İmleç bir nesnenin üstüne gelince nesne vurgulanır, o nesne türünde en son kullanılan araç
simgesi ve birkaç özellik görünür; sağ tık o aracı başlatır, mavi alana gelince bütün araç grupları açılır. Rollover
özelliklerinde kenar uzunluğu, eğim, kot, yüzey alanı, sınır çevre ve alanı okunur, düzenlenebilir olanlar yerinde
değiştirilir (B6). V27 2B nesneler için "marking menu" jestlerini deneysel olarak ekledi (B2).

**Smart Blocks (AutoCAD).** Placement (2024+): blok eklenirken önceki yerleşimlere göre konum ve yön önerir;
`AUTOPLACEMENT` ile açılıp kapanır, Shift+W ya da Shift+[ geçici olarak bastırır (A11). Replacement: makine öğrenimi
önerisi, son kullanılanlar ya da elle seçilen blokla değiştirme (A21, arama özeti). Search and Convert: aynı geometrinin
bütün örneklerini bulup vurgular, kaynak nesneyi ya da seçilen örnekleri yeni bloğa çevirir; 2026'da metin farklarını
da bulur (oda adı/numarası özniteliğe aday), 2026.1'de blok içini Layer 0'a atama seçeneği geldi (A6, A7, A12). Detect
and Convert makine öğrenimiyle nesne algılar; 2026.1.1'de Tech Preview olmaktan çıktı (A6, A8).

**BLOCKIFY (BricsCAD).** Aynı 2B/3B nesne kümelerini bulup blok referansına çevirir; dört yöntem: seçili nesneyle eşle,
var olan bloklarla eşle (çizimden ya da klasörden), eşit 3B katılar, koleksiyonlar; tolerans (otomatik ya da %0–100) ve
parametrik koruma seçenekleri var; 3B katı yöntemi Lite'ta yok (B7).

**BIMPROPAGATE (BricsCAD BIM).** Seçili referans katılardaki detayı (katı, delik, bitiş geometrisi) modeldeki benzer
bağlantılara önerir; her öneride yeşil tik / soru işareti / kırmızı çarpı; blok ya da kopya olarak yayar; arama alanı
sınırlanabilir (B8).

**Activity Insights, Markup Import/Assist, Autodesk Assistant, Checkout (AutoCAD).** Activity Insights 2026'da "What's
Changed" ile kaydedilmiş iki an arasındaki düzenlemeleri özetler ve çizime bağlıdır, bütün ekip aynı içgörüleri görür
(A6, A9). Markup Assist, Autodesk Docs işaretlemelerini ve 2026'dan itibaren Docs dışı PDF'leri tanır; metni, üstü
çiziliyi ve sınır işaretini algılar, sınırı revizyon bulutu olarak ekler (A6, A10; ayrıntı arama özeti). Autodesk
Assistant (2027): çizimi standart dosyasına göre denetler, doğal dille seçim yapar, katman listesi ya da blok kullanımı
gibi sorguları yanıtlar; 2027.1'de nesne sayımı, seçime duyarlı analiz ve birim raporu eklendi (A2, A1). Checkout
(2027): başkasının açık tuttuğu çizimden nesneleri "check out" edip Trace içinde düzenleme ve birleştirilmek üzere
gönderme (A1, R1).

**ZWCAD Smart özellikleri.** Smart Mouse: sağ tuşla sürüklenen yön ya da harf jestine komut atanır (`SMARTMOUSE`,
`SMARTMOUSECONFIG`). Smart Voice: nesneye, noktaya ya da alana 1–30 sn sesli not; basılmaz, `VOICEMAN` sesi metne
çevirir. Smart Plot: çerçeve stili (katman/blok), toplu dosya, yazıcı ayarı ile toplu çıktı (`SMARTPLOT`). Smart Select:
`SMARTSEL` (Z3). Bunların hepsi Standard dahil bütün sürümlerde (Z4).

Ekran görüntüleri:
- BricsCAD BIM QuickDraw — genel: https://static.bricsys.com/help/images/V27/en_INTL/bimquickdraw.png
- BricsCAD BIM QuickDraw — L oda: https://static.bricsys.com/help/images/V27/en_INTL/Quickdraw_L_Shapedroom.png
- BricsCAD BIM QuickDraw — 2B plandan 3B model: https://static.bricsys.com/help/images/V27/en_INTL/Quickdraw_Hover_ThinWalls.png
- BricsCAD Quad + rollover: https://static.bricsys.com/help/images/V27/en_US/QUAD_RT.png
- BricsCAD Quad — açılmış araç grupları: https://static.bricsys.com/help/images/V27/en_US/QUAD_Highlight_Tools.png
- BricsCAD V27 Quad marking menu: https://images.ctfassets.net/26961o1141cc/3mQ9EKPVmeU0GxTwYqp0u6/6c17e8b7eae2b9063ed43d9e429df31d/Quad_Cursor_marking_menu.png?w=2434&h=1368&fm=webp&q=100
- BricsCAD BLOCKIFY sonuçları: https://static.bricsys.com/help/images/V27/en_US/CMD_blockify_matchentities_results.png
- BricsCAD BIMPROPAGATE: https://static.bricsys.com/help/images/V27/en_INTL/CMD_BIMPROPAGATE01.PNG
- AutoCAD 2026 Smart Blocks: Detect and Convert: https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-WhatsNew/images/GUID-449EB135-1500-461E-A75E-B1CB08DC0B4C.png
- AutoCAD 2026.1 Smart Blocks (Layer 0 seçeneği): https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-WhatsNew/images/GUID-3E11778D-76EF-4F48-9962-A8A204B9F460.png
- AutoCAD 2026 Activity Insights: https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-WhatsNew/images/GUID-0533AE70-8243-48D1-8C50-D7E41BEBB4FA.png
- AutoCAD 2026 Markup Import & Assist: https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-WhatsNew/images/GUID-582B28AD-3737-49FB-B504-33EDFB4281D5.png
- AutoCAD 2027 Autodesk Assistant: https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-WhatsNew/images/GUID-ED7F459A-0B44-4E44-AAAB-D2DAA525F41D.png
- ZWCAD Smart Mouse jest izi: https://statics.zwsoft.com/upload/default/20250723/93ffea3266b7a40106bb944f219097e7.png

---

## 3. KentOS Mimari için öncelik önerisi

Ölçüt: "AutoCAD (ACA) düzeyinde ayrıntılı mimari pafta" üretebilmek için neyin olmazsa olmaz olduğu. KentOS'un bugünkü
uygulamasını bu çalışma incelemedi; maddeler "yoksa ekle, varsa bu ayrıntıya genişlet" diye okunmalı. Her maddenin
yanında kanıt olan ürün/kaynak var.

### ZORUNLU (bunlar olmadan AutoCAD düzeyi yok)

| # | Madde | Ne kadar ayrıntı | Kanıt |
|---|---|---|---|
| Z1 | **Kesin girdi zinciri** | Kenetleme (uç, orta, kesişim, dik, teğet, merkez, en yakın, uzantı, paralel); kenetleme izleme (edinilen nokta, hizalama yolu); polar izleme (artım + ek açılar) / Ortho; doğrudan mesafe; imleçte dinamik girdi (uzunluk/açı, Tab); göreli/kutupsal koordinat; FROM ve geçici izleme noktası | A13, A14, A15, B2 |
| Z2 | **Düzenleme çekirdeği** | Çok işlevli tutamaç (uzat, köşe ekle/sil, yay'a çevir), STRETCH, OFFSET (çoklu), FILLET/CHAMFER (Shift = 0), TRIM/EXTEND, ALIGN, ilişkisel dizi, MATCHPROP, özellik tabanlı seçim | A1, P1, Z2, B1 |
| Z3 | **Annotative ölçekleme** | Yazı, ölçü, tarama, etiket, mleader modelde bir kez; 1/50–1/100–1/200 paftada kâğıt boyunda; görünüm penceresi ölçeğiyle eşleme | R14 |
| Z4 | **Stil sistemi** | Ölçü, yazı, mleader, tablo stilleri + kurum şablonu; standarttan sapma denetimi (.dws benzeri) | A19 |
| Z5 | **Katman + kalem tablosu + pafta** | Katman durumları; nesne türü → katman anahtarı (Layer Key Style; REVCLOUD/TABLE/VIEWPORT/CENTERLAYER mantığı); CTB/STB eşdeğeri; layout + görünüm pencereleri; pafta seti | R2, A6, A18 |
| Z6 | **Blok + öznitelik + dinamik blok** | Görünürlük, uzatma, döndürme, arama tablosu; öznitelikten tablo; çok görünümlü blok (plan/görünüş) | A16, R2, Z4, G2 |
| Z7 | **Duvar stili + temizleme** | Katmanlı bileşen (sıva / tuğla / yalıtım / beton) + bileşen öncelik numarası (düşük = üstün) + temizleme grupları (ör. mevcut / yeni ayrı grup) | R5, R6 |
| Z8 | **Duvara bağlı kapı/pencere/açıklık** | Duvarla birlikte hareket; yerleştirirken sol kenar / orta / sağ kenar ölçüsü döngüsü; çapa (konum, ayna, yön) düzenleme; stil (kasa, kanat, açılım yayı, denizlik) | R2, P1, P4 |
| Z9 | **Görünüm yönüne ve ölçeğe göre gösterim** | Aynı nesnenin plan / tavan / kesit / görünüş temsilleri; görünüm penceresine atanan ayrıntı düzeyi (ör. 1/200'de dolu tek çizgi duvar, 1/50'de bileşen taramalı) | R3, R4 |
| Z10 | **Mahal + alan hesabı + çizelge** | Brüt / net / kullanılabilir sınırlar + hesap düzenleyiciler; mahal etiketi; özellik setinden çizelge (mahal, kapı, pencere listesi) | R8, R2, B10, G3, N5 |
| Z11 | **Kesit ve görünüş üretimi** | Kesit çizgisinden 2B kesit/görünüş; modelle ilişkili; "güncel değil" işareti; kesit göstergesi, kot ve aks balonları | R2, B9 |

### ÖNEMLİ (ayrıntı ve verim için yüksek değer)

| # | Madde | Kanıt |
|---|---|---|
| Ö1 | **Tadilat modu**: mevcut / yıkılacak / yeni; silme = yıkılacak kategorisi; yıkım planı ve revizyon planı ayrı gösterim | R7 |
| Ö2 | **Merdiven + korkuluk stilleri**, **döşeme/çatı** (kenar profili, eğim), **giydirme cephe ızgarası**, **kolon/kiriş** 2B gösterimi | R11, R12, R13, G3, N3 |
| Ö3 | **AEC ölçüsü**: duvar bileşenlerine ve açıklıklara bağlı otomatik zincir; tek komutta çok yönlü zincir (QDIM Advanced); nesne türüne göre akıllı ölçü; kot yazısı | R2, B1, Z1, P1 |
| Ö4 | **Detay bileşen kütüphanesi + keynote veritabanı** | R9 |
| Ö5 | **Proje gezgini**: yapı / kat / görünüm / pafta yapısı; pafta seti alanları; toplu pafta özelliği | R10, A1, B9 |
| Ö6 | **Dış başvurular** + kopuk yol onarımı; xref'te duvar temizleme kuralı | A1, R6 |
| Ö7 | **Hızlı plan aracı** (QuickDraw benzeri) ve **2B DWG planından duvara dönüştürme** — eski arşiv paftaları için | B5 |
| Ö8 | **Toplu çıktı**: model uzayındaki çerçeveyi tanıyan toplu PDF (Smart Plot), pafta seti yayını, paketleme | Z3, G2, D1 |
| Ö9 | **Revizyon bulutu** (otomatik katman, ölçeğe göre yay) + **çizim karşılaştırma** (fark → bulut) | A6, G1, A9 |
| Ö10 | **Geometri temizliği**: OVERKILL, kopuk uç/boşluk taraması (içe aktarılan DWG için) | A4, P1, B1 |
| Ö11 | **Tematik renklendirme**: mahal kullanım türüne göre renkli plan | R2 |

### SONRA (farklılaştırır ama çekirdek değil)

- Yapay zekâ asistanı (doğal dille seçim, sorgu, standart denetimi — A2; BricsCAD Assist — B1).
- Makine öğrenimiyle blok algılama/önerme (Detect and Convert, Placement, Replacement — A8, A11, A21).
- Markup Assist (PDF işaretlemesini içe alma — A10), Activity Insights (A9), sesli not / fare jesti (Z3).
- Çok kullanıcılı düzenleme (Checkout — A1).
- IFC/RVT içe aktarma, BIMPROPAGATE türü detay yayma, nokta bulutu izleme (B1, B8, Z2).

---

## 4. Pratiklik: bu ürünlerden alınacak en iyi 10 fikir

1. **Oda sürükleyerek plan (BricsCAD BIM QuickDraw).** Dikdörtgen sürükle → duvar + döşeme; odanın içine imleç → komşu
   oda; dış duvar parçasını sürükle-sil → L oda; Tab ile en/boy; "kat ekle" (tam kopya / döşeme + dış duvar / düz çatı).
   Ek: 2B DWG planının çizgilerinin üstünde gezinerek duvar üretme, kalınlığı paralel çizgi aralığından alma (B5).
2. **Quad imleç (BricsCAD).** Nesnenin üstünde son kullanılan araç + düzenlenebilir özellikler (uzunluk, alan, kot);
   komut ezberlemeden çalışma. V27'de jest menüsü (B6, B2).
3. **Açıklığı ölçerek yerleştirme (progeCAD 2027 / IntelliCAD 14.1).** Kapı/pencere yerleştirirken ölçü sol kenar → orta
   → sağ kenar döngüsü ve duvara göre çapa düzenleme; mimarın "pencere kenarı duvar köşesinden 60 cm" alışkanlığına
   birebir uyar (P1, P4).
4. **Akıllı blok yerleştirme ve değiştirme (AutoCAD Smart Blocks).** Mobilya/vitrifiye eklerken önceki yerleşime göre
   konum ve yön önerisi; tek tuşla bastırma; benzer blok önerisiyle değiştirme (A11, A21).
5. **Tekrarlı geometriyi bloğa çevirme (Search and Convert / BLOCKIFY / Smart Match).** İçe aktarılan "patlatılmış"
   planda aynı kapıları, lavaboları bulup tek bloğa bağlama; metin farklarını (oda adı/numarası) özniteliğe çevirme
   (A7, B7, Z1).
6. **Hızlı ölçü üçlüsü.** QDIM Advanced (tek komutta çok yönlü zincir — B1) + Smart Dimension (nesne türüne göre ölçü
   — Z1) + ölçü tutamacı menüsü (ok çevir, baseline/zincir sürdür — Z1, P1).
7. **Nesne türüne göre otomatik katman.** Bulut, tablo, görünüm penceresi, merkez çizgisi ve mimari nesneler kendi
   katmanına düşer; kullanıcı katman değiştirmeyi unutamaz (A6; ACA Layer Key Styles — R2).
8. **Çerçeveyi tanıyan toplu çıktı (ZWCAD Smart Plot).** Model uzayındaki pafta çerçevelerini katman ya da blokla tanıyıp
   kâğıt boyunu eşleyerek birden çok dosyayı tek seferde PDF'e basma (Z3).
9. **Seçim hızlandırıcıları.** Özellik süzgeçli seçim + anında önizleme (ZWCAD Smart Select), bağlı çizgileri tek seferde
   seçme (Chain Select), kaydedilebilir seçim sorguları (BricsCAD V26.2), doğal dille seçim (Autodesk Assistant) (Z2,
   Z3, B1, A2).
10. **Tadilat ve revizyonu tek akışa bağlamak.** Tadilat modunda "sil = yıkılacak" (R7) + iki kayıt arasındaki farkı
    gösteren özet ya da DWG karşılaştırmasının farkları revizyon bulutuyla işaretlemesi (A9, G1).

Ayrıca not: **nanoCAD 26'nın yerel dilde komut takma adları** (N1) — Türkçe bir ürün olan KentOS için Türkçe komut
adları + İngilizce AutoCAD takma adlarını birlikte tanımak, AutoCAD alışkanlığıyla gelen kullanıcıyı kaybetmemenin en
ucuz yolu. **progeCAD 2027'nin Shift ile sıfır yarıçaplı FILLET'i** ve **OFFSET Multiple** (P1) küçük ama sık kullanılan
kısayollar.

---

## 5. Doğrulanamayanlar ve açık sorular

- AutoCAD LT'de CAD Standards (STANDARDS/CHECKSTANDARDS), Layer Translator, Smart Blocks Placement/Replacement, Detect and
  Convert ve Markup Import/Assist bulunup bulunmadığı — resmî karşılaştırma tablosu 403 döndüğü için **doğrulanamadı**.
- BricsCAD'in tam paket matrisi (Lite/Pro/BIM/Ultimate) — karşılaştırma sayfasının işaretleri okunamadı. BricsCAD BIM'de
  merdiven (BIMSTAIR?), giydirme cephe, taşıyıcı eleman, tadilat ve çizelge araçları bu çalışmada **doğrulanamadı**.
- ZWCAD Architecture'ın güncel sürümü ve özellikleri — resmî güncel sayfa bulunamadı.
- ZWCAD 2027 Beta'nın ayrıntılı listesi — yalnız resmî yerel sayfa başlıkları görüldü ("Neu in ZWCAD 2027 Beta"); ayrıntı
  için açılan MEA sayfası 2026 içeriği gösterdi.
- GstarCAD Architecture sayfası sürüm numarası vermiyor (dipnot 2013–2021, gezinme menüsü GstarCAD 2027).
- DraftSight'ın paket bazında dinamik blok, Toolbox, kısıt ve Image Tracer dağılımı — yalnız birbiriyle çelişen bayi
  kaynakları var.
- ACA display configuration adlarının tam listesi (Low/High Detail, Reflected, Screened …) ve hazır AIA/ISO 13567 katman
  standardı içeriği — bu çalışmada açılan sayfalarda geçmedi.
- (✔) ile işaretli bütün hücreler "doğrulanamadı" sayılmalı.

---

## 6. Kaynaklar (kimlik → URL, sürüm)

**Autodesk — AutoCAD / LT**
- A1 — What's New in AutoCAD 2027 (2027, 2027.1): https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-WhatsNew/files/GUID-D52A51CA-BDA1-4A78-9E67-87B340C55490.htm
- A2 — Autodesk Assistant (2027): https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-WhatsNew/files/GUID-99E2D8F7-D4D9-4B3A-9E57-7B73D6BB12AD.htm
- A3 — Autodesk Assistant (2027.1): https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-WhatsNew/files/GUID-0BBFEE0C-6FBF-4067-B8C0-3CD67D9CEAE3.htm
- A4 — Geometry Cleanup (2027): https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-WhatsNew/files/GUID-BAA6379F-68A6-4D56-BE22-307170488818.htm
- A5 — Checkout (2027): https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-WhatsNew/files/GUID-806DD388-5A79-4AF0-9034-C4618DE86B3E.htm
- A6 — What's New in AutoCAD 2026 (2026 … 2026.1.2): https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-WhatsNew/files/GUID-FAB1960D-49C1-4A12-B128-5511F7889AB9.htm
- A7 — Smart Blocks: Search and Convert (2026): https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-WhatsNew/files/GUID-3B3A51B6-FC61-40AB-9EEA-597426339131.htm
- A8 — Smart Blocks: Detect and Convert (2026): https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-WhatsNew/files/GUID-7D3E7065-AD01-4720-B5DF-95971BAEFA9A.htm
- A9 — Activity Insights (2026): https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-WhatsNew/files/GUID-ABDBDD72-C2E7-4024-87F1-DF3E4FC0887E.htm
- A10 — Markup Import & Markup Assist (2026): https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-WhatsNew/files/GUID-7DEBC745-EA11-4A40-81B4-96EDC8DD8F1C.htm
- A11 — Smart Blocks: Placement (2024): https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-WhatsNew/files/GUID-60D2011A-FB12-4B45-B2FC-0A6E4159D294.htm
- A12 — What's New in AutoCAD LT 2026: https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT-WhatsNew/files/GUID-8123423F-32E3-4866-82DF-A71C79E71059.htm
- A13 — To Specify a Precise Point Location (LT 2026): https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT/files/GUID-392167BC-8032-44D9-B4A9-DF4AC00DF5C4.htm
- A14 — About Tracking Points Automatically Using Object Snaps (LT 2026): https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT/files/GUID-665DC37F-8C3E-414A-9369-72A13C0BE07A.htm
- A15 — About Specifying a Point Relative to an Existing Point (FROM / tracking, LT 2026): https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT/files/GUID-94C1330A-EA6E-4476-A7F8-67F7BEE30E57.htm
- A16 — About Dynamic Blocks (LT 2026; kısıt LT'de yok): https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT/files/GUID-3C2FB982-3AF6-437B-987F-4EDF81EA0662.htm
- A17 — Block Editor (LT 2026): https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT/files/GUID-B69D58AD-7920-4198-AB2A-0E24B944F6CD.htm
- A18 — About the Connected Sheet Set Manager (LT 2026): https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT/files/GUID-E6B489F6-C969-4EC8-8961-51DB5BDBD89C.htm
- A19 — About CAD Standards (AutoCAD 2027): https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Core/files/GUID-D64F8076-4978-44B7-B056-D921C77FEA88.htm
- A20 — "The Architecture toolset is included with AutoCAD 2027" (ürün sayfası başlığı; içerik 403): https://www.autodesk.com/products/autocad-architecture
- A21 — Introducing AutoCAD 2025 (Smart Blocks Replacement/Object Detection; arama özeti): https://www.autodesk.com/blogs/autocad/autocad-2025/

**Autodesk — Architecture toolset (ACA)**
- R1 — What's New in AutoCAD Architecture 2027 Toolset: https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Architecture-WhatsNew/files/GUID-D69B1019-FF16-4091-9F67-49AA26875088.htm
- R2 — A Quick Start for Transitioning From AutoCAD to AutoCAD Architecture 2026 Toolset: https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Architecture/files/GUID-0EC70E35-2A2C-4394-9461-561E50F48CF6.htm
- R3 — About Display Configurations (2027): https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Architecture/files/GUID-BA31AA3B-5E14-410B-9AD8-661A9E4D47A8.htm
- R4 — About Display System Structure (2027): https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Architecture/files/GUID-8512B736-FC8A-4559-BFF5-7F097517A1C2.htm
- R5 — Wall cleanup priorities (2027): https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Architecture/files/GUID-CFB380A0-837B-4199-91EF-006F072B0A4C.htm
- R6 — About Cleanup Group Definitions (2027): https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Architecture/files/GUID-82455A71-8BA8-4307-A958-B452ED145235.htm
- R7 — Renovation (2027): https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Architecture/files/GUID-34250742-1584-4319-8070-36B76781F020.htm · About Working in Renovation Mode (2027): https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Architecture/files/GUID-0F2C5EF6-CCCF-4DE5-B6B8-130D052814C6.htm
- R8 — About Spaces (2026): https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Architecture/files/GUID-84D8FD5B-9DC2-46DA-9F8C-620805271E76.htm · Space calculation properties (2026): https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Architecture/files/GUID-6AE1AF64-5B20-4022-AFAC-1875F021797C.htm
- R9 — To Specify Keynotes for Detail Components (2026): https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Architecture/files/GUID-FD40CF4F-D31C-4CC4-B0BD-A5D823CD9A90.htm · About Specifying Keynotes: https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Architecture/files/GUID-64A548F1-8BBD-4215-B48B-7FFCB04B5F6E.htm · Detail component databases: https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Architecture/files/GUID-188D014A-FEE6-45FA-9D43-8BC136189417.htm
- R10 — About Project Navigator (2026): https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Architecture/files/GUID-BDA8F9DF-B9C2-4014-B783-E12C9CAC90CD.htm · PROJECTNAVIGATORTOGGLE (2026): https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Architecture/files/GUID-2A238204-CE77-4F7B-9646-334149AEDCE9.htm
- R11 — About Curtain Walls (2026): https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Architecture/files/GUID-F8BEA145-7C35-4662-B8FE-0C2B08E5A676.htm
- R12 — About Structural Members (2026): https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Architecture/files/GUID-A90E26B3-13EB-4355-9838-F7E871C53D6F.htm
- R13 — About Slabs and Roof Slabs (2026): https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Architecture/files/GUID-76BFBB16-7916-4A74-B505-FEA66823E76F.htm
- R14 — About Annotation Scaling (2027): https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Architecture/files/GUID-77856F78-BCB0-4BA3-BD74-D112267825DD.htm

**BricsCAD (Bricsys / Octave)**
- B1 — BricsCAD V26 / V26.2 yenilikleri: https://bricscad.octave.com/bricscad/v26
- B2 — BricsCAD V27 yenilikleri: https://bricscad.octave.com/bricscad/v27
- B3 — Sürüm karşılaştırma (işaretler okunamadı): https://bricscad.octave.com/bricscad/compare
- B4 — BIMQUICKDRAW command: https://help.bricsys.com/en-us/document/command-reference/b/bimquickdraw-command
- B5 — Using BIM Quickdraw (V27 yardımı): https://help.bricsys.com/en-us/document/bricscad-bim/modeling-techniques/using-bim-quickdraw
- B6 — Quad cursor: https://help.bricsys.com/en-us/document/bricscad-bim/getting-started/quad-cursor
- B7 — BLOCKIFY command: https://help.bricsys.com/en-us/document/command-reference/b/blockify-command
- B8 — BIMPROPAGATE command: https://help.bricsys.com/en-us/document/command-reference/b/bimpropagate-command
- B9 — Generating drawings (BIM): https://help.bricsys.com/en-us/document/bricscad-bim/design-documentation/generating-drawings
- B10 — BIMSPACE command: https://help.bricsys.com/en-us/document/command-reference/b/bimspace-command · Creating and editing BIM spaces: https://help.bricsys.com/en-us/document/bricscad-bim/building-data/creating-and-editing-bim-spaces
- B11 — BIMWINDOWCREATE command: https://help.bricsys.com/en-us/document/command-reference/b/bimwindowcreate-command

**ZWCAD (ZWSOFT)**
- Z1 — Introducing ZWCAD 2026 (8 Mayıs 2025): https://www.zwsoft.com/news/products/zwcad-2026-design-with-speed-innovate-with-ease
- Z2 — ZWCAD What's New (2026 içeriği): https://www.zwsoft.com/mea/product/zwcad/whats-new
- Z3 — ZWCAD Smart Features kullanıcı kılavuzu: https://www.zwsoft.com/user-guide/zwcad-smart-features
- Z4 — ZWCAD fiyat ve sürüm farkları (Standard / Professional / MFG): https://www.zwsoft.com/product/zwcad/pricing
- Z5 — "Neu in ZWCAD 2027 Beta" (yalnız başlık görüldü): https://www.zwsoft.com/de/product/zwcad/whats-new

**GstarCAD (Gstarsoft)**
- G1 — Introducing GstarCAD 2026 Beta: https://www.gstarcad.net/news/introducing-gstarcad-2026-beta-faster-smarter-better?id=283
- G2 — GstarCAD 2027 (resmî blog, 11 Haziran 2026): https://blog.gstarcad.net/gstarcad-2027-unleashing-performance-and-innovation-for-modern-engineering/
- G3 — GstarCAD Architecture özellikleri: https://www.gstarcad.net/architecture/feature

**progeCAD / IntelliCAD**
- P1 — progeCAD 2027 What's New (IntelliCAD 14.1): https://www.progesoft.com/products/progecad-professional/progecad2027-whats-new
- P2 — progeCAD 2026 What's New (IntelliCAD 13.1; arama özeti): https://www.progesoft.com/products/progecad-professional/progecad2026-whats-new
- P3 — progeCAD mimari eklenti (EasyArch/AEC): https://progesoft.com/products/progecad-professional/easyarch-architecture
- P4 — IntelliCAD 14.1 Release Details (ITC): https://www.intellicad.org/intellicad-14.1-release-details
- P5 — IntelliCAD 14.1 Readme (yayın 10.12.2025; arama özeti): https://www.intellicad.org/hubfs/IntelliCAD%2014.1/IntelliCAD%2014.1%20Readme%202025-12-04.pdf
- P6 — What's new in CMS IntelliCAD 14.1 (ITC üyesi üretici; arama özeti): https://intellicadms.com/home/cms-intellicad-2d-3d-cad-software-autocad-dwg-compatible/41-in-detail/what-is-new/581-what-is-new-intellicad-14-1

**DraftSight (Dassault Systèmes)**
- D1 — What's New in DraftSight 2026 (SOLIDWORKS blog, 7 Kasım 2025): https://blogs.solidworks.com/products/draftsight/whats-new-in-draftsight-2026

**nanoCAD (Nanosoft)**
- N1 — nanoCAD 26 Release (Mart 2026): https://nanocad.com/company/news/nanocad-26-release-smarter-tools-enhanced-precision-and-a-more-intuitive-experience/
- N2 — nanoCAD 26 Features: https://nanocad.com/products/nanocad-platform/features/
- N3 — nanoCAD Platform Help — Stairs (SPSTAIRADD): https://nanocad.com/learning/online-help/nanocad-platform/stairs/
- N4 — nanoCAD Platform Help — List of openings (SPLISTOFOPENINGS): https://nanocad.com/learning/online-help/nanocad-platform/list-of-openings/
- N5 — nanoCAD Construction Module Tutorials: https://nanocad.com/learning/video/nanocad-construction-module-tutorials/
