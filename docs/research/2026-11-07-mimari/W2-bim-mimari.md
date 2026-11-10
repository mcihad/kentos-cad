# W2 — BIM ve mimariye özel yazılımlar (KentOS Mimari için)

Hazırlanma: 2026-11-07 · Kapsam: Revit 2027 (+ Revit LT), Archicad 29/30, Vectorworks Architect 2026/2027,
Allplan 2026, ARCHLine.XP 2026, Chief Architect Premier X17/X18, İdeCAD Mimari (v11), Edificius (kısa).

**Yöntem.** Yalnız üretici kaynakları: yardım/belge sayfaları, sürüm notları, üreticinin resmî forumu/blogu/eğitim
listeleri. Yaklaşık 65 arama ve sayfa okuması yapıldı. Hiçbir dosya indirilmedi; ekran görüntüsü adreslerinin hepsi
HTTP HEAD ile denetlendi (200, `image/*`).

**Erişim sınırları.** `autodesk.com` (blog, Revit/LT karşılaştırma), `allplan.com` (özellik ve sürüm notu sayfaları),
`idecad.com.tr` ürün sayfası okuma aracına **403** döndü; `forums.idecad.com.tr` ekleri de 403 (oturum istiyor olabilir).
Bu kaynaklardan gelen bilgiler **"(arama özeti)"** diye işaretlendi: içerik, resmî sayfanın arama motorundaki özetinden
geliyor, sayfa satır satır okunmadı.

**Kaynak kimlikleri** (R1, A8, V12 …) §6'daki listede URL ve sürümle çözülür. KentOS'un bugünkü durumuna yapılan
göndermeler çağıranın notlarından: **M0** = `M0-kendi-gozlemim.md`, **PLAN** = `A3-plan-durumu.md` (aynı klasör).

---

## 0. Sürüm durumu (7 Kasım 2026'da doğrulanan)

| Ürün | Güncel sürüm | Not | Kaynak |
|---|---|---|---|
| Revit | **2027** (Nisan 2026) → 2027.1 (Haziran), 2027.2 (Temmuz), 2027.3 | What's New sayfası 2027.3 maddelerini de listeliyor | R1 |
| Revit LT | **2027** | Revit'in alt kümesi; API, worksharing, görünüm filtreleri yok | R2, R2b, R11 |
| Archicad | **30** (Ekim 2026 başı planlı; yardımda AC30 "New Architectural Features" sayfası yayında) | 29 = 2025 sürümü | A1, A2, A21, A4 |
| Vectorworks Architect | **2027** (16 Eylül 2026) + Update 1 | 2026 Update 4 "Phasing" (tadilat evresi) getirdi | V1, V2, V7 |
| Allplan | **2026** (2026-1-x yamaları) | Allplan 2027 duyurusu: **doğrulanamadı** | L1, L2 |
| ARCHLine.XP | **2026** | | H1 |
| Chief Architect Premier | **X18** güncel; görevde istenen X17'nin halefi | X17 özellik listesi Haziran 2025 tarihli | C1, C2 |
| İdeCAD Mimari | **v11** (yardım: "ideCAD v11 ile Gelen Yenilikler"); Nisan 2025 duyurusunda "ideCAD Mimari 1115" yapı numarası | 2026 sürümü: **doğrulanamadı** | I2, I4, I17 |
| Edificius | **doğrulanamadı** | Yalnız mahal özellikleri bakıldı | E1 |

---

## 1. Özellik × ürün matrisi

**V** = var · **K** = kısmen · **Y** = yok · **D** = doğrulanamadı (bu çalışmada resmî kaynakta görülmedi;
"yok" demek değildir). Köşeli parantez = kaynak kimliği. Ayrıntı §2'de.

Sütunlar: **Rvt** Revit 2027 · **LT** Revit LT 2027 · **AC** Archicad 29/30 · **VW** Vectorworks Architect 2026/27 ·
**AL** Allplan 2026 · **AX** ARCHLine.XP 2026 · **CA** Chief Architect X17/X18 · **ID** İdeCAD Mimari v11

### 1A. Eleman araçları ve tip sistemi

| Özellik | Rvt | LT | AC | VW | AL | AX | CA | ID | Not |
|---|---|---|---|---|---|---|---|---|---|
| Tip/örnek ayrımı, stiller | V [R12] | V [R2] | K [A7, A8] | V [V3, V1] | D | K [H1] | K [C2, C3] | K [I8] | VW parametre başına "by style / by instance"; AC favori = bağsız ön ayar, malzeme/bileşik global öznitelik |
| Katmanlı (bileşik) duvar | V [R12, R13] | D | V [A7] | V [V9] | D | V [H1] | V [C3] | D | AX 2026: katman ekle (işlev, malzeme, kalınlık, konum) |
| Kapı/pencere tipleri, 2B sembol | V [R2] | V [R2] | V [A1] | V [V1, V5] | K [L1] | V [H1] | V [C2, C3] | V [I8] | AX 2026 özel 2B kapı sembolü; VW kapı/pencerede düşük/orta/yüksek ayrıntı |
| Kolon, kiriş, döşeme, çatı, merdiven, korkuluk | V [R1, R2] | K [R2] | V [A1, A7] | V [V1] | V [L1] | V [H1] | V [C3] | V [I3, I4] | LT'de eğik kolon, kafes, donatı yok |
| Giydirme cephe | D | D | V [A8, A12] | D | D | D | D | D | AC'de otomatik zon ve otomatik dış ölçü giydirme cepheyi tanıyor |
| Favoriler / hazır ön ayar paleti | D | D | V [A8, A16] | K [V3] | D | K [H1] | V [C2, C3] | K [I4] | CA Default Sets + X18 Style Palettes; ID v11 yeni proje sihirbazı |

### 1B. Parametrik ilişkiler

| Özellik | Rvt | LT | AC | VW | AL | AX | CA | ID | Not |
|---|---|---|---|---|---|---|---|---|---|
| Duvarı duvara barındırma (host) | V [R1] | D | D | D | D | D | D | D | Revit 2027 yeni |
| Mahal sınırlayan eleman ayarı | V [R3, R4, R1] | V [R3] | V [A8] | K [V4, V6] | D | V [H2] | D | K [I5] | Rvt 2027: duvarı çizmeden önce "mahal sınırlamaz" |
| Tıkla-mahal + duvar değişince güncelleme | V [R3, R5] | V [R2, R3] | V [A8] | V [V4, V6] | D | V [H2] | K [C1, C2] | V [I5] | VW yeniden üretimde mahal parametrelerini koruyor |
| Geçici ölçü → kalıcı ölçü, kilit, EQ | V [R6, R7] | V [R8] | D | D | D | D | K [C1] | D | |
| Sihirli değnek / kontur yakalama | D | D | V [A9] | K [V4] | D | D | D | K [I6] | ID: "Kontur seçerek mahal oluşturma" konusu var, içerik açılmadı |
| Kesişim önceliği / temiz birleşim | K [R1, R12] | D | V [A7] | D | D | K [H1] | K [C2] | D | AC malzeme önceliği 0–999 |

### 1C. Gösterim

| Özellik | Rvt | LT | AC | VW | AL | AX | CA | ID | Not |
|---|---|---|---|---|---|---|---|---|---|
| Ölçeğe/ayrıntı düzeyine göre gösterim | V [R12] | D | V [A5, A6] | V [V5, V1] | V [L3, L4] | D | D | D [I3] | AC: kısmi yapı gösteriminin her kipinin kendi ölçü takımı var |
| Tadilat / evre (mevcut-yıkılacak-yeni) | V [R9, R10] | V [R9] | V [A5, A11] | V [V7, V8] | D | D | D | D | VW'de evre 2026 Update 4 ile geldi |
| Kural tabanlı grafik geçersiz kılma | V [R11] | **Y** [R11] | V [A5, A6] | V [V8] | V [L1] | D | K [C2] | D [I3] | LT'de görünüm filtresi yok |
| Katman kombinasyonu / görünüm şablonu | V [R14] | V [R14] | V [A10, A5] | K [V1] | D | K [H3] | V [C2, C3] | D | VW 2027 "Non Plot" sınıf etiketi |
| Kalem seti / çizgi kalınlığı tablosu | V [R1] | D | V [A13] | D | D | D | K [C2] | V [I9] | ID: 20 kalem kalınlığı (mm), 120 renk |

### 1D. Otomatik açıklama ve çizelge

| Özellik | Rvt | LT | AC | VW | AL | AX | CA | ID | Not |
|---|---|---|---|---|---|---|---|---|---|
| Toplu/otomatik etiketleme | V [R15, R1] | V [R16] | V [A8, A1] | V [V1, V3] | D | D | K [C2] | V [I5, I7] | Rvt 2027.2 "Tag by Example" |
| Otomatik ölçülendirme | D | D | V [A12, A1] | V [V11] | V [L1] | V [H4] | V [C1, C3] | V [I10, I11, I4] | AC ve VW ölçüleri ilişkili (eleman değişince güncellenir) |
| Kural tabanlı numaralandırma | V [R1] | D | D | D | D | D | K [C2] | D | Rvt 2027 "Rule-Based Numbering" |
| Alan şeması / norm alan hesabı | V [R17] | V [R2] | K [A14, A8] | V [V1] | V [L5, L6] | V [H2] | K [C3, C2] | V [I12, I7] | AL DIN 277 + Wohnfläche; AX DIN 277, WoFlV, İtalyan; ID TAKS/KAKS + DIN 277 + II BV |
| Çizelgeden modeli düzenleme | K [R18] | D | V [A15] | V [V12] | D | K [H2] | K [C3] | K [I13] | Rvt'de yalnız model→çizelge yönü resmî cümleyle doğrulandı |

### 1E. Belge üretimi ve proje yönetimi

| Özellik | Rvt | LT | AC | VW | AL | AX | CA | ID | Not |
|---|---|---|---|---|---|---|---|---|---|
| Kesit, görünüş, iç görünüş | V [R2] | V [R2] | V [A16] | V [V1] | V [L1] | D | V [C3] | V [I1, I4] | CA dış ve iç oda görünüşlerini otomatik üretiyor |
| Pafta / yayın setleri | V [R1, R2] | V [R2] | V [A17] | V [V13] | V [L4] | D | V [C3] | V [I4, I17] | VW Title Block Manager: çok anteti toplu düzenleme |
| Revizyon yönetimi | V [R2] | V [R2] | V [A17, A18] | V [V13, V14] | D | K [H1] | D | D | AX yalnız dosya sürüm geçmişi |
| Ekip çalışması | V [R2, R1] | **Y** [R2] | V [A16, A2] | D | D | K [H1] | D | D | VW Project Sharing bu çalışmada açılmadı |
| IFC | V [R1] | V [R2] | V [A19, A2] | V [V12, V1] | D | V [H1] | D [C3] | D | CA özellik sayfasında IFC yok |
| Tasarım seçenekleri | V [R1, R2] | V [R2] | V [A20] | D | D | D | D | D | |
| Yapay zekâ / MCP / API | V [R1] | **Y** [R2] | V [A2, A4] | K [V2] | D | D | D | D | Rvt Public MCP Server (Tech Preview), Archicad MCP (Beta) |

### 1F. Türkiye'ye özgü işler (yalnız İdeCAD'de arandı)

| Özellik | ID | Kaynak | Not |
|---|---|---|---|
| TAKS/KAKS/emsal hesabı, eskizde canlı denetim | V | I12, I17 | Aşan kontrol önizlemede X ile işaretleniyor |
| Arsa poligonu koordinat tablosundan, çekme mesafeleri | V | I17 | "ideCAD Mimari 1115" ile geldi (Nisan 2025) |
| Mimari + statik tek model / tek dosya | V | I2, I3, I19 | Mimari plan modu ↔ kalıp planı modu (Shift+Tab) |
| Ruhsat paftaları | V (arama özeti) | I1 | "Otomatik oluşturulan kesit/görünüşlerle birlikte, çıktıya hazır ruhsat paftaları" |
| Otopark hesabı | D | — | Resmî kaynakta yalnız rampa modelleme soruları bulundu |
| Belediye onay süreci entegrasyonu | D | — | Hiçbir resmî kaynakta görülmedi |
| Türk standardına göre hazır tarama/sembol kitaplığı | D | — | Yalnız kullanıcı tanımlı kalem/renk tablosu görüldü [I9] |

Diğer ürünlerde Türkiye mevzuatına özgü araç **aranmadı** (kapsam dışı). Norm tabanlı alan hesabı örnekleri Almanya
(Allplan, ARCHLine) ve İtalya (ARCHLine) için var: "norm = değiştirilebilir profil" kalıbının kanıtı.

**Edificius (kısa):** mahal "otomatik alan algılama" ile oluşturuluyor, mahal otomatik ölçülendiriliyor, kapanmayan
mahal için tanı ekranı var (arama özeti) [E1]. Sürüm ve diğer özellikler: doğrulanamadı.

---

## 2. Özellik gruplarının açıklaması ve ekran görüntüleri

### 2.1 Eleman araçları ve tip sistemi: üç model

1. **Revit: aile → tip → örnek.** Tip özellikleri bütün örnekleri değiştirir; duvar tipinde katmanlar ve
   "Coarse Scale Fill Pattern / Color" tip düzeyinde durur [R12, R13]. 2027'de **duvarı duvara barındırma** geldi:
   barındırılan duvar ana duvarın geometrisini izliyor, öteleme ve yeniden barındırma alıyor, çizelgelenebiliyor [R1].
2. **Archicad: araç ayarı + Favoriler + global öznitelik.** Favoriler bağsız ön ayardır (zon etiketi bile Favoriler
   açılır listesinden seçiliyor [A8]). Tip benzeri bağ **malzeme ve bileşik özniteliklerinde**: bir Building
   Material'ın önceliği değişince onu kullanan her eleman etkilenir; tek elemanı ayırmak için malzeme çoğaltılır [A7].
3. **Vectorworks: stil + parametre başına kilit.** Eklenti nesnelerinde her parametre "by style" (stilden gelir,
   örnekte değişmez) ya da "by instance" (örnekte serbest) olarak işaretleniyor [V3]. 2027'de duvar, döşeme ve çatı
   stili **Object Info paletindeki örnekten** oluşturulabiliyor: "önce çiz, beğenince stile çevir" [V1].
4. **Chief Architect:** Custom Wall Types ve Default Sets; X18'de Style Palettes kapı, raf ve bölmeleri de kapsıyor;
   X18'de "mulled" (birleşik) kapı/pencere varsayılanları ve kitaplık desteği [C2, C3].
5. **ARCHLine.XP 2026:** sınırsız "object variants", içe alınan Revit ailelerinin varyantlara dönüşmesi, 2B kapı
   sembolünün (kontur, açılış çizgisi, sembol) kullanıcı tanımlı olması [H1]. **İdeCAD:** kapı/pencere tip listesi
   kapı ve pencere tiplerine göre klasörlenmiş; genişlik, yükseklik, kot ve etiket ayarları tipte [I8].

Ekran görüntüleri:
- VW "by style / by instance" simgeleri: https://app-help.vectorworks.net/2026/eng/assets/images/PIO_ByStyleInstance.png
- Revit 2027 duvarı duvara barındırma: https://help.autodesk.com/cloudhelp/2027/ENU/Revit-WhatsNew/images/GUID-D221F320-A58C-4232-B154-D736156ED8D2.png
- ARCHLine.XP 2026 kiriş böl/birleştir: https://www.archlinexp.com/images/2026/02/19/image-14.png

### 2.2 Parametrik ilişkiler

- **Mahal sınırlama.** Revit'te her model elemanının "Room Bounding" parametresi var; mahal alanı ve hacmi bu
  sınırlardan hesaplanıyor; bağlı modelin sınırları ana modelde ayrıca açılmalı, varsayılan kapalı [R3, R4]. 2027'de
  duvar **çizilmeden önce** "mahal sınırlamaz" yapılabiliyor ve "odaya göre / parçaya göre" duvar oluştururken yön
  çevrilebiliyor [R1]. Archicad'de duvarın "Relation to Zones" ayarı "Zone Boundary" ya da "No Effect on Zones" [A8].
- **Mahal oluşturma kipleri.** Archicad: **Inner Edge** (duvar iç yüzüne kadar) ve **Reference Line** (duvar
  eksenine/referans çizgisine kadar; planda Gross ya da Net gösterim). Tıklanan nokta sonraki güncellemenin başlangıcı
  olarak saklanıyor; kapı/pencere dışı boşluklar sınırı "taşırabiliyor" [A8]. Vectorworks: **Picked Walls** (mahal
  seçilen duvarlara bağlı kalır), **Inner Boundary** (duvarlar görünür, birleşik ve aynı katman ölçeğinde olmalı) ve
  toplu **Create Spaces from Walls** (her oda için ya da tek brüt mahal; en küçük alan eşiği; yeniden çalıştırınca
  mahaller **parametreleri korunarak** yeni duvarlara uyar) [V4, V6]. İdeCAD: kapalı alana tıkla → mahal ve yazı bloğu
  ortada; Ctrl ile yazının yeri seçilir; sınırlar hatasız birleşmezse mahal oluşmaz ya da hatalı oluşur [I5].
  Edificius: kapanmayan mahal için tanı (arama özeti) [E1].
- **Geçici ölçüler ve kısıtlar (Revit).** Eleman oluşturulurken/seçilince geçici ölçüler en yakın dik elemana
  bağlanıyor; kalıcıya çevrilebiliyor, şahit çizgisi başka elemana taşınabiliyor [R6]. Kalıcı ölçüdeki kilit bir kısıt
  oluşturuyor; çok parçalı ölçüde EQ eşitlik kısıtı; kısıtlar görünümden bağımsız, mavi kesik çizgiyle görünüyor;
  kilitli ölçüler çatışmada korunuyor, kilitsizler sessizce düşebiliyor [R7]. Revit LT'de de var [R8].
- **Sihirli değnek (Archicad).** Boşluk tuşuyla etkin; mevcut geometriden kapalı alanı ya da eleman zincirini izliyor [A9].
- **Kesişim önceliği (Archicad).** Malzemelere 0–999 öncelik; eşitlikte yerleşik sıra Morph > Column > Beam > Slab >
  Wall > Shell > Roof [A7]. Revit'te Coarse görünümde duvara birleşen kolon duvarın kaba dolgusunu alıyor [R12].

Ekran görüntüleri:
- Archicad otomatik zon: https://help.graphisoft.com/AC/29/INT/_AC29_Help/040_ElementsVB/ZoneAutomatic.png
- Archicad zonla otomatik etiket: https://help.graphisoft.com/AC/29/INT/_AC29_Help/040_ElementsVB/ZoneLabelAuto.png
- İdeCAD kapalı alana tıklayarak mahal: https://help.idecad.com.tr/__attachments/a_e0c666b9f5e33c80e5bcc5987cba6b8468926c4f15d5ec393c3e3e25e481591d/image-20201027-065937.png?cb=8dbd886f30093be63280f9a8db61fc01

### 2.3 Gösterim: bir model, birçok çıktı

- **Archicad View Settings** her görünümde dört şeyi birlikte saklıyor: Partial Structure Display, Model View
  Options, Graphic Override Combination ve Renovation Filter. Model View Options proje geneli; önce grafik geçersiz
  kılma kuralları, en son (en düşük öncelikle) tadilat filtresi uygulanıyor; "Drawing" tipli görünümde
  değiştirilemiyor [A5]. **Kısmi yapı gösteriminde** (Entire Model / Without Finishes / Core Only) her kipin **kendi
  ölçü takımı** var: kip değişince öbür kipte konmuş ölçüler gizleniyor, geri dönünce yeniden görünüyor; zonlar ve
  kaplama dolguları bu ayardan etkilenmiyor [A6]. Layer Combinations her görünümle saklanıyor [A10]. Çizim kendi kalem
  setini ya da kaynak görünümünkini kullanabiliyor [A13].
- **Archicad Renovation Filter:** eleman durumu Existing / To be Demolished / New; filtre her durumu gösterir, gizler
  ya da geçersiz kılar; görünümle saklanır [A11].
- **Revit:** görünümün Detail Level'ı Coarse / Medium / Fine; Coarse'ta bileşik eleman yalnız dış sınırıyla ve tipteki
  kaba dolguyla çiziliyor [R12]. Evre filtresi New / Existing / Demolished / Temporary durumlarının her biri için
  gösterim tanımlıyor [R9, R10]. View Template atanınca görünümün bütün görsel ayarları şablondan geliyor [R14].
  Revit LT'de görünüm filtreleri ve V/G "Filters" sekmesi yok [R11]. 2027'de bağlı modellerin çizgi kalınlığı ana
  modelin tablosundan alınabiliyor [R1].
- **Vectorworks:** kapı/pencere düşük/orta/yüksek ayrıntı (2026), sembolün 2B/3B bileşenlerinin ayrıntı düzeyi
  özelleştirme (2027), "Non Plot" sınıflar ekranda görünüp basılmıyor (2027) [V5, V1]. **Evre** (2026 Update 4):
  çoğu tadilat dosyası iki evreyle kuruluyor (mevcut; yıkım + yeni); varsayılan şablonda yıkılan/kaldırılan kırmızı
  kesik çizgi, yeni kendi öznitelikleriyle, mevcut beyaz dolgulu; mevcut ve kaldırılan duvarlar düşük ayrıntıda
  çizilerek yeni iş öne çıkarılabiliyor [V7, V8].
- **Allplan:** referans ölçek yazı, ölçü, desen ve taramanın modele göre boyunu belirliyor [L3]; desen öğeleri pafta
  yerleşiminde "sabit boy" ya da yerleşim ölçeğiyle büyüyen (1 m öğe 1:100'de 1 cm, 1:50'de 2 cm) [L4]. 2026'da
  özelliklere göre Graphic Override (arama özeti) [L1].

Ekran görüntüleri:
- Archicad "Core Only" kipine ait ölçü: https://help.graphisoft.com/AC/26/INT/_AC26_Help/050_ViewsVB/DimCoreOnly.png
- Archicad grafik geçersiz kılma örneği (yangın planı): https://help.graphisoft.com/AC/28/INT/_AC28_Help/050_ViewsVB/FirePlan.png
- Revit 2027 bağlı modelde çizgi kalınlığı: https://help.autodesk.com/cloudhelp/2027/ENU/Revit-WhatsNew/images/GUID-92909CEF-B3FD-4A30-8AE3-FF7C3CD0ACC2.png
- Archicad 30 "Show on Stories" (eşleşme sayfa düzeninden çıkarıldı): https://www.graphisoft.com/content/uploads/2026/10/Media-box-5-1-1.jpg

### 2.4 Otomatik açıklama ve etkileşimli çizelge

- **Etiket.** Revit "Tag All Not Tagged": seçim yoksa görünümdeki bütün etiketsiz elemanları etiketliyor; birden çok
  kategori, bağlı dosyalar, etiket çizgisi ve yön seçenekleri; görünürlük ya da görünüm şablonu engelliyorsa uyarıyor
  [R15]. 2027: çok kategorili etiket iyileştirmeleri, uçları serbest etiket çizgileri, **Tag by Example** (2027.2),
  **Isolate Annotations** (2027.3; açıklamaları yalıtılmış düzenleme kipi) [R1]. Vectorworks 2027 Data Tag'e
  "Coincident Picking", "Tagging Order" kipleri ve uygunluk ölçütü, yeni **Data Legend** aracı [V1]. Archicad zon
  yerleştirilince etiket otomatik ("Place Label" anahtarı) [A8]; AC30'da yeni etiket çerçeve biçimleri ve anahtar not
  lejandında satır kaydırma [A1].
- **Numaralandırma.** Revit 2027 "Rule-Based Numbering": numara şablonları paylaşılan parametre değerlerine göre
  kapı, ekipman, temel gibi elemanlara otomatik numara veriyor; çizelgeleri de etkiliyor [R1]. Chief X18
  çizelgelerde sayı, harf ve Roma rakamı biçimleri [C2].
- **Otomatik ölçü.** Archicad Automatic Exterior Dimensioning: seçili duvar/giydirme cepheye ilişkili ölçü; seçenekler
  toplam, dış geometri, duvar yapısı, açıklıklar ve "dört kenara yerleştir"; gereksiz zincir (açıklık yok, cephe düz)
  atlanıyor. Interior Dimensioning: çizilen çizgiyi kesen duvar, kolon, kiriş ve döşeme kenarlarını ölçüyor [A12].
  AC30 şahit/uzatma çizgisi özelleştirme [A1]. Vectorworks "Dimension Exterior Walls": duvara ya da çekirdeğe, dış yüze
  ya da eksene; kapı/pencere merkezine ya da kenarına; kaba açıklık (şim payı, kapıda kasa genişliği); iç duvar
  T-birleşimleri; ölçüler duvarla ilişkili kalabiliyor [V11]. Chief: Auto Exterior Dimensions, Automatic Elevation
  Dimensions; X17'de NKBA araçları genel otomatik ölçüye katıldı, ölçü zincirleri varsayılan ötelemelere kenetleniyor
  [C1, C3]. ARCHLine LT sayfası "bütün projeyi 3 tıkla ölçülendir" diyor (arama özeti) [H4]. İdeCAD: iç ölçüde
  çizilen hat üzerindeki elemanların ölçüsü otomatik, dış ölçüde dahil edilecek elemanlar ayarlardan seçiliyor; v11'de
  ayarlar Serbest / Dış / İç diye ayrıldı [I10, I11, I4]. Allplan 2026 görünüş ve kesitlere ilişkili otomatik ölçü
  (arama özeti) [L1].
- **Alan şemaları ve normlar.** Revit varsayılan iki alan şeması (Gross Building düzenlenemez, Rentable) ve sınırsız
  özel şema; alan şemaları tasarım seçeneklerine konamıyor, seçenek başına ayrı şema gerekiyor [R17]. Vectorworks 2027
  birden çok "detailed space area rule set": niş, kolon ve diğer nesnelerin alan/hacme etkisini belirliyor, IFC ve
  metraj raporlarına bağlanabiliyor [V1]. Allplan DIN 277 sekmesi: kapalılık tipi DIN 277-2021/2016 (R, S) ya da
  2005 (a, b, c, bc); ayrıca Wohnfläche ve "Land Act" sekmeleri [L5, L6]. ARCHLine oda defteri: DIN 277, WoFlV,
  İtalyan normu (arama özeti) [H2]. İdeCAD: §4.
- **Etkileşimli çizelge.** Archicad'de çizelgede yapılan değişiklik plana ve diğer görünümlere hemen yansıyor;
  düzenlenebilir alanlar **mavi**, salt okunur alanlar siyah [A15]. Vectorworks çalışma sayfasındaki veritabanı
  satırından kayıt ve IFC verisi düzenlenebiliyor, bağlı veri etiketleri kendiliğinden güncelleniyor; özetlenmiş satırı
  düzenlemek o satırın bütün nesnelerini değiştiriyor; hesaplanan alanlar, kilitli/başvurulu nesneler ve sınıf
  düzenlenemiyor (Design Series ürünü gerekiyor) [V12]. Revit: "A schedule is another view of the model" ve "All
  schedules update automatically when you modify the model" [R18]; çizelgeden modele yazma bu çalışmada resmî
  cümleyle **doğrulanamadı**. Chief: çizelgeden nesne özelliğini açma; CSV/TXT/XML dışa aktarım [C3].

Ekran görüntüleri:
- Revit 2027.2 Tag by Example: https://help.autodesk.com/cloudhelp/2027/ENU/Revit-WhatsNew/images/GUID-7F3D2318-FA7A-4280-AD2F-8F90627C9596.png
- Revit 2027 kural tabanlı numaralandırma: https://help.autodesk.com/cloudhelp/2027/ENU/Revit-WhatsNew/images/GUID-A3156137-AD74-411E-8725-40FB205C8BD3.png
- Revit 2027 çok kategorili etiket: https://help.autodesk.com/cloudhelp/2027/ENU/Revit-WhatsNew/images/GUID-C72DCC42-9326-41D9-9162-08642963AD14.png
- Vectorworks çalışma sayfasından nesne düzenleme: https://app-help.vectorworks.net/2026/eng/VW2026_Guide/Worksheets/ws_1.png
- Allplan DIN 277 sekmesi: https://help.allplan.com/Allplan/2025-0/1034/Allplan/31084.png
- Archicad 30 ölçü şahit çizgileri (eşleşme sayfa düzeninden çıkarıldı): https://www.graphisoft.com/content/uploads/2026/10/Media-box-9-1.jpg

### 2.5 Belge, pafta, revizyon, ekip, IFC

- **Archicad:** değişiklikler elemanlara bağlıysa ilgili paftanın revizyon numarası kendiliğinden artıyor; gönderilmiş
  revizyonlar kilitli, yalnız son revizyon düzenlenebilir; ana paftaya konan Revision History nesnesi her pafta için
  kendi verisini gösteriyor; şablonda varsayılan revizyon şemaları [A17, A18]. Publisher'da o anki gönderim setindeki
  paftalar ve onları içeren setler yeşil; "gönderim setinden etkilenenleri yayımla" seçeneği [A17].
- **Vectorworks Title Block Manager:** işaretli setteki bütün antetlere birden düzenleme; proje verisi değişikliği
  bütün dosyaların antetlerine; yalnız etkin antetler otomatik pafta/sayfa numarasına giriyor; revizyon eklerken hangi
  katmanlara uygulanacağı ve başka dosyaların katmanları seçilebiliyor; revizyon/ihraç numarası elle ya da otomatik
  artan [V13, V14].
- **Revit:** revizyon izleme Revit ve LT'de; çok kullanıcılı çalışma (worksharing) yalnız Revit'te [R2]. 2027:
  "Toggle Auto Expand Sheet Views", görünüm başlıklarında kenetleme (2027.2) [R1].
- **Chief:** Layout Plan Sets; dış ve iç oda görünüşlerini otomatik üretme; arkadan kırpılmış kesit/görünüş;
  "CAD Detail From View" [C3].
- **IFC:** Revit 2027 IFC Parameter Mapping [R1]; Archicad IFC Open/Merge/Hotlink ve çevirmenler [A19], AC30 IFC4x3
  ifcSpatialZone eşlemesi [A2]; ARCHLine 2026 IFC 4 coğrafi konum [H1].

Ekran görüntüleri:
- Vectorworks antet yapısı: https://app-help.vectorworks.net/2026/eng/VW2026_Guide/Setup/TBlock_overall.png
- Revit 2027 merdiven basamak/rıht numarası: https://help.autodesk.com/cloudhelp/2027/ENU/Revit-WhatsNew/images/GUID-62B0E1C5-8314-4EA8-871B-86FAED2BDA20.png
- Revit 2027.3 Isolate Annotations: https://help.autodesk.com/cloudhelp/2027/ENU/Revit-WhatsNew/images/GUID-ABDFBC93-38A2-470C-9ABB-65A1D5976570.png
- Chief Architect X18 tanıtım görseli (2B değil): https://cloud.chiefarchitect.com/1/images/whats-new/x18/chief-architect-x18-hero-1612x756.jpg
- ARCHLine.XP 2026 IFC 4 coğrafi konum: https://www.archlinexp.com/images/2026/02/19/image-15.png

### 2.6 Bu yazılımları "hem pratik hem ayrıntılı" yapan desenler

| Desen | Kanıt | KentOS'a anlamı |
|---|---|---|
| **Önce otomatik, gerekince elle**: aynı nesneye iki yol | AC otomatik zon (Inner Edge / Reference Line) ve elle çokgen [A8]; CA X18 "otomatik ve elle duvar çerçevesi birlikte", "duvar otomatik bağlanmasını kapat" [C2]; AX oda defterinde hesaplanan ya da elle girilen değer [H2] | Her otomatik sonucun elle geçersiz kılınabilir alanı olsun; geçersiz kılma görünür işaretlensin |
| **Yeniden çalıştırılabilir otomasyon**: kullanıcının girdisini silmez | VW "Create Spaces from Walls" yeniden çalışınca mahalleri parametreleri koruyarak yeniler [V6]; AC otomatik dış ölçü gereksiz zinciri atlar [A12] | Toplu mahal bul, ölçü dizisi, etiketle: hepsi ikinci kez çalıştırıldığında ad/no/kaplama korunur |
| **Tek palet + arama** | Allplan 2026: duvar oluşturma ve düzenleme tek özellik paleti; sürüm notu amacını "yeni kullanıcının öğrenme eğrisini düzleştirmek, deneyimlinin verimini artırmak" diye yazıyor (arama özeti) [L1]; AC30 araç ayarlarında özellik araması, Property Manager'da aç/kapa [A1]; Revit 2027 Options Bar'ı kaldırdı [R1] | Araç ayarı ve öznitelik aynı bileşen; elemana özgü alanlar önce; arama kutusu |
| **Görünüm = ayar kombinasyonu** | AC View Settings: kısmi yapı + model görünüm seçeneği + grafik geçersiz kılma + tadilat filtresi + katman kombinasyonu [A5, A10] | Acemi tek "Ruhsat planı" görünümünü seçer; uzman kombinasyonun her parçasını ayrı ayarlar |
| **Ürün katmanı** | Revit LT: worksharing, görünüm filtresi, API, yerinde modelleme yok [R2, R11]; ARCHLine LT "3 tıkla ölçü" [H4]; VW ürünlere göre komut tablosu [V15] | KentOS'ta "Hızlı" ve "Uzman" çalışma alanı: aynı motor, farklı görünür komut kümesi |
| **Canlı denetim** | İdeCAD eskiz modunda TAKS/KAKS gerçek zamanlı, sınır aşımında X [I12] | Yönetmelik denetimi çizim sırasında rozet; denetim çizimin sonunda değil |
| **Ofis standardı tek yerde** | AC şablonda varsayılan revizyon şemaları [A18]; VW antetler dosyalar arası tek yöneticiden [V13]; Revit görünüm şablonu [R14]; İdeCAD v11 yeni proje sihirbazı [I4] | Ofis profili dosyası: katman, kalem, tip, antet, revizyon şeması birlikte |
| **Küçük UX işleri de sürüm maddesi** | Revit 2027 maddelerinin çoğu "community idea" etiketli: mahal sınırını önceden kapatma, etiket çizgisi, bağlı model çizgi kalınlığı, açıklamaları yalıtma, düzenleme rengi [R1] | Sürtünme listesi (M0) doğrudan plan maddesi olabilir |

---

## 3. 2B öncelikli KentOS Mimari'ye uyarlanabilecek en değerli 15 fikir

Sıra = önerilen öncelik. **[2B]** 2B çizim modeliyle yapılabilir · **[2,5B]** kat/eleman yükseklik verisi gerekir ·
**[3B]** genel 3B model gerekir.

1. **Ölçeğe göre ayrıntı düzeyi ve kesilen elemanın dolgusu (poché)** [2B]
   Öncül: AC kısmi yapı gösterimi (Entire / Without Finishes / Core Only) ve kip başına ölçü takımı [A6]; Revit Coarse
   = dış sınır + tipte tanımlı kaba dolgu [R12]; VW kapı/pencerede düşük/orta/yüksek [V5]; Allplan referans ölçek ve
   desenin yerleşim ölçeğiyle büyümesi [L3, L4].
   KentOS boşluğu: duvar ince çift çizgi, dolgu/tarama yok; dikdörtgen duvarın varsayılan stili "Yok" (M0).
   Öneri: görünüm ölçeğinden türeyen üç kip (1:200 dolu tek parça ve sade kapı; 1:100 çekirdek dolgusu + kaplama
   çizgisi; 1:50 bütün katmanlar ve tarama); kapı/pencere sembolü aynı kipe bağlı; ölçü zinciri hangi kipte konduysa
   orada görünür. Kabul: aynı 2+1 daire 1:200/1:100/1:50 paftalarında elle ayar yapılmadan üç ayrı ayrıntıda basılır.

2. **Evre (tadilat) çizgi dili ve görünüm başına tadilat filtresi** [2B]
   Öncül: Revit New/Existing/Demolished/Temporary [R9, R10]; AC göster/gizle/geçersiz kıl, en düşük öncelik [A5, A11];
   VW varsayılanı: yıkılan kırmızı kesik, mevcut beyaz dolgu, mevcut düşük ayrıntıda [V8].
   KentOS boşluğu: tadilat örneğinde yeni bölme ile mevcut duvar aynı görünüyor; araçlarda Evre açık (M0; PLAN §4/3).
   Öneri: her elemanda evre alanı + araç düzeyinde "yeni elemanın evresi"; üç hazır filtre (Mevcut durum, Yıkım,
   Yeni durum); grafik dili ofis profilinden (TR pratiği mevzuat ajanının bulgusuyla eşlenmeli).

3. **Katman standardı, görünüm kombinasyonu ve kural tabanlı geçersiz kılma** [2B]
   Öncül: AC Layer Combinations + View Settings + Graphic Override kuralları [A10, A5, A6]; Revit görünüm şablonu ve
   filtreler (LT'de filtre yok: profesyonel katman ayrımı) [R14, R11]; VW Non Plot sınıf [V1]; CA Layer Sets + Saved
   Plan Views [C3].
   KentOS boşluğu: 54 nesnenin hepsi "Çizim" katmanında (M0); ofis standardı 10 katman (PLAN ARCH-UX-03).
   Öneri: araç başına otomatik katman (duvar, açıklık, mahal, ölçü, etiket, çizelge, basılmaz yardımcı); adlandırılmış
   kombinasyonlar (Ruhsat planı, Uygulama planı, Mobilya planı, Tadilat); "yangın planı" gibi kural tabanlı geçersiz
   kılma (AC FirePlan örneği).

4. **Mevzuat alan profili = adlandırılmış alan şemaları** [2B]
   Öncül: Revit Area Schemes, seçenek başına ayrı şema [R17]; VW 2027 ayrıntılı mahal alanı kural setleri [V1];
   Allplan DIN 277 sürümleri [L5]; ARCHLine norm seçimi [H2]; İdeCAD TAKS/KAKS + DIN 277 + II BV, mahali rapordan
   dışlama [I7, I12].
   KentOS boşluğu: resmî alan profili, arsa payı açık (PLAN §4/14).
   Öneri: norm = veri dosyası (mahal türü/etiketi → dahil/hariç/katsayı; sıva payı; kolon/merdiven/şaft düşümü);
   aynı modelde birden çok şema (Emsal, Brüt inşaat, Bağımsız bölüm net); renkli alan planı + paftaya tablo; kurala
   uymayan mahal tanı listesinde.

5. **Arsa ve imar denetimi: parsel, çekme hatları, canlı TAKS/KAKS** [2B]
   Öncül: İdeCAD 1115: XY koordinat tablosunu Excel/Word'den yapıştırma, koordinat kilidi, çekme mesafeleri görsel;
   TAKS/KAKS imar durumu belgesinden; emsal harici alan kontrolü %30, kat net alanına göre %20 varsayılan; "benzer kat";
   eskiz modunda gerçek zamanlı; aşan kontrol X ile; Excel/TXT; paftaya yerleştirme [I12, I17].
   KentOS fırsatı: aynı uygulamada Harita/Yol işleri var (M0: "Bu CAD çizimi hangi iş için?"); parsel poligonu harita
   tarafından doğrudan alınabilir. Öneri: durum çubuğunda canlı rozet (TAKS, KAKS, emsal dışı oranı); oranlar Türk
   mevzuat ajanının bulgularıyla.

6. **Dört cephe otomatik ölçü: ilişkili, katmanlı zincirler** [2B]
   Öncül: AC otomatik dış ölçü (toplam, dış geometri, duvar yapısı, açıklıklar, dört kenar; boş zincir atlanır) [A12];
   VW çekirdek/yüz, açıklık merkezi/kenarı, kaba açıklık, iç duvar T-birleşimi [V11]; ARCHLine "3 tık" [H4]; İdeCAD
   iç/dış/serbest [I10, I11, I4]; AC30 şahit çizgisi özelleştirme [A1].
   KentOS boşluğu: ölçü dizileri kodda bitmiş (PLAN ARCH-DIM-01) ama kabul akışında yalnız alt dış zincir üretilmiş;
   iç ölçü ve diğer cepheler yok (M0); eğik/yay duvar reddediliyor (PLAN).
   Öneri: tek komutun varsayılanı dört cephe × üç zincir (açıklıklar, kırılım/aks, toplam); "çekirdeğe/yüze" seçimi
   1. fikirdeki ayrıntı kipinden; boş zincir üretilmez; yeniden çalışınca elle taşınmış ölçüler korunur.

7. **Çakışmasız etiket yerleşimi, örnekten etiketle, açıklamaları yalıt** [2B]
   Öncül: Revit Tag All Not Tagged [R15]; 2027.2 Tag by Example, serbest uçlu etiket çizgisi, 2027.3 Isolate
   Annotations [R1]; VW 2027 Coincident Picking / Tagging Order / uygunluk ölçütü [V1].
   KentOS boşluğu: "Etiketle (hepsi)" sonrası etiket kutuları duvar çizgilerinin üstüne biniyor; merdiven etiketleri
   çakışıyor (M0).
   Öneri: yerleşimde duvar dolgusundan, açılış yayından ve diğer etiketlerden kaçınma; seçili etiketin tipini ve göreli
   konumunu benzerlerine uygulama; yalnız açıklamaların seçilebildiği düzenleme kipi.

8. **Kural tabanlı numaralandırma (odaya göre kapı/pencere numarası)** [2B]
   Öncül: Revit 2027 Rule-Based Numbering [R1]; Chief X18 sayı/harf/Roma biçimleri [C2].
   KentOS boşluğu: odaya göre numara yok (PLAN ARCH-OPEN-01).
   Öneri: numara şablonu (ör. kat, mahal numarası, sıra alanları); değişiklikte yeniden numaralandırma önizlemesi;
   çizelge ve etiket aynı numarayı okur.

9. **Çift yönlü etkileşimli çizelge ve paftada çizelge görünümü** [2B]
   Öncül: AC Interactive Schedule (mavi = düzenlenebilir) [A15]; VW çalışma sayfası satırından nesne düzenleme, özet
   satırdan toplu düzenleme [V12]; Revit "çizelge modelin bir görünüşüdür" [R18].
   KentOS boşluğu: çizelgeler model alanında plana yapışık, mahal çizelgesi görünümün altına taşıyor (M0); paftada ayrı
   çizelge görünümü açık (PLAN ARCH-DOC-02).
   Öneri: hücre düzenle → model; özet satır toplu düzenler; hesaplanan/kilitli hücre ayrı renkte; paftadaki çizelge
   model alanına bir şey yazmaz.

10. **Tip/örnek kilitleri, favoriler ve "örnekten stil"** [2B]
    Öncül: VW by style / by instance [V3] ve 2027'de örnekten duvar/döşeme/çatı stili [V1]; AC Favoriler [A8]; Chief
    Style Palettes [C2]; ARCHLine nesne varyantları [H1].
    KentOS boşluğu: varsayılan duvar stili "Yok"; duvar seçiliyken Öznitelikler'de önce genel CAD alanları, duvarın
    kalınlık/stil/yüksekliği aşağıda (M0); duvar genişliğinin araç başına belleği açık (PLAN §4/3).
    Öneri: tipteki her parametre "tipten (kilitli)" ya da "örnekte serbest"; ofis profilinin doldurduğu favori paleti;
    seçili örnekten tip oluştur; Öznitelikler'de elemana özgü alanlar önce.

11. **Mahal sınırı anahtarı, sınır kipi ve parametre koruyan yeniden üretim** [2B]
    Öncül: Revit 2027 çizmeden önce "mahal sınırlamaz" [R1]; AC Inner Edge / Reference Line ve "No Effect on Zones"
    [A8]; VW yeniden üretimde parametre koruma ve en küçük alan eşiği [V6]; Edificius kapanmayan mahal tanısı [E1].
    KentOS durumu: toplu mahal bul ve tanı, ayırıcı ve kolon desteği var (PLAN ARCH-SPACE-01).
    Öneri (eksik halkalar): araç çubuğunda yeni duvarın "mahal sınırı" anahtarı (alçak duvar, korkuluk duvarı);
    mahal başına sınır kipi (iç yüz / eksen / dış yüz) alan şemasına girdi (4. fikir); duvar taşınınca ad, numara ve
    kaplama korunarak sınır yenilenir.

12. **Malzeme kesişim önceliğiyle temiz birleşimler** [2B]
    Öncül: AC malzeme önceliği 0–999 ve yerleşik sıra [A7]; Revit katman sarma ve kolonun duvar dolgusunu alması
    [R1, R12]; ARCHLine kolon yalıtımı yalnız duvar katmanları dışında [H1].
    Öneri: duvar katmanı malzemesine öncelik; T/L/X ve kolon-duvar birleşiminde dolgu ve çizgi temizliği önceliğe göre;
    tek elemanı ayırmak için malzemeyi çoğalt. (1. fikrin dolgusuyla birlikte anlam kazanır.)

13. **Pafta yöneticisi, yayın setleri ve değişiklik → revizyon zinciri** [2B]
    Öncül: AC elemana bağlı değişiklik → pafta revizyonu, Revision History nesnesi, Publisher Sets [A17, A18]; VW Title
    Block Manager [V13, V14]; Revit revizyon izleme [R2].
    KentOS boşluğu: pafta başına revizyon çizelgesi, proje revizyon tablosu, yayın/revizyon birlikteliği açık
    (PLAN §4/8; ADR 0721).
    Öneri: "Ruhsat seti", "Uygulama seti" yayın setleri; revizyon bulutu ↔ değişiklik kaydı ↔ antet revizyon tablosu
    otomatik; antetlerin toplu düzenlenmesi; yalnız son revizyon düzenlenebilir.

14. **Kat izleri ve basılmaz yardımcılar** [2B]
    Öncül: AC30 duvarda genişletilmiş "Show on Stories" (ana katın üstü/altı için özel seçenek) [A1, A2]; ARCHLine'da
    diğer katlardaki duvarlar kat bazında süzülerek görünüyor (sürümsüz sayfa, arama özeti) [H3]; VW Non Plot sınıf
    [V1]; Revit 2027 özelleştirilebilir düzenleme rengi [R1].
    Öneri: alt kat izi (soluk) ve üst kat çıkması (kesik) görünüm ayarı; eleman başına "görüneceği katlar"; basılmaz
    katman/sınıf etiketi.

15. **Özellik araması, tek palet ve dışa açık komut arayüzü (MCP)** [2B]
    Öncül: AC30 araç ayarlarında özellik araması [A1]; Allplan 2026 tek özellik paleti (arama özeti) [L1]; Revit 2027
    Options Bar'ı kaldırdı [R1]; Revit Public MCP Server (Tech Preview) ve Archicad MCP (Beta) [R1, A2].
    KentOS boşluğu: komut geçmişi ham, Türkçe-İngilizce karışık parametre döküyor (M0); panel ↔ komut satırı eşitliği
    17 araçta eksik (PLAN B9).
    Öneri: araç ayarı ve öznitelik tek bileşen + arama; komut geçmişinde Türkçe insan dili; KentOS'un komut sözleşmeleri
    dış ajanlara MCP benzeri bir yüzeyle açılabilir.

**3B ya da 2,5B gerektirenler (listenin dışında, sonra):**
- Oda bazlı otomatik dış görünüş ve kesit — Chief dış/iç oda görünüşleri ve arkadan kırpılmış kesit [C3], Archicad
  iç görünüş [A16]. KentOS'ta oda iç görünüşü var (PLAN: ADR 0799); dış görünüş ve sınırlı kesit duvar boyu, lento,
  parapet ve döşeme kalınlığıyla **[2,5B]** yapılabilir.
- Otomatik çatı (Chief Automatic Roofs [C3]) **[3B]**.
- Duvarı duvara barındırma (Revit 2027 [R1]) **[2,5B]**: düşey ilişki.
- Giydirme cephe ızgarası **[2,5B]**; plandaki kayıt/çerçeve gösterimi 2B yapılabilir.
- Yapay zekâ ile görselleştirme (Archicad, Vectorworks AI Visualizer [A2, V2]) **[3B]**.

---

## 4. İdeCAD Mimari: Türk kullanıcı için öne çıkan işler ve KentOS'taki karşılığı

"KentOS'ta bugün" sütunu çağıranın PLAN/M0 notlarından; burada yeniden doğrulanmadı.

| İdeCAD işi | Resmî kaynaktaki davranış | Kaynak | KentOS'ta bugün | Önerilen karşılık |
|---|---|---|---|---|
| **TAKS/KAKS/emsal** | Araçlar → Kat Alanı Katsayısı. TAKS ve KAKS imar durumu belgesinden girilir. Emsal harici alan kontrol oranı %30, kat net alanına göre kontrol %20 varsayılan ("mevcut imar yönetmeliğindeki değer"); kat net kontrolü bağımsız bölüm değil baz kat bazında. "İle benzer" kat; taban alanına girecek katlar işaretlenir. Excel/TXT dışa aktarım. "Eskiz modunda gerçek zamanlı göster" | I12 | Resmî alan profili açık (PLAN §4/14) | 4. ve 5. fikir: alan şeması + canlı rozet |
| **Arsa poligonu** | 1115 ile: XY koordinat tablosu (Excel/Word'den kopyala), elle çizim de var; koordinatlar döndürme/taşımada değişmez; arsa sınırı ve çekme mesafeleri görsel; merdiven, balkon, kat bahçesinin emsale dahil/hariç seçimi; hesap paftaya yerleştirilebilir | I17 | Harita işi aynı uygulamada (M0) | Parseli harita tarafından al; çekme hatları |
| **Kat alanı poligonları** | "Kat alanı tanımla" ile taban ve kat alanı poligonları; hesaba girmeyecek alanlar poligonun içinde tanımlanıp hesap dışı bırakılır; sınır aşımı önizlemede X (arama özeti) | I12 | Mahal ve bağımsız bölüm modülleri var (görev metni) | Alan şemasında "kat alanı" türü, mahalden türetilen ya da elle çokgen |
| **Tıkla-mahal** | Kapalı alana tıkla → mahal + yazı bloğu ortada; Ctrl ile yazı yeri; sınırlar hatasız birleşik olmalı | I5 | Toplu mahal bul ve tanı var (PLAN ARCH-SPACE-01) | Tek tık ve toplu birlikte; 11. fikir |
| **Mahal ayarları** | Beş sekme (Genel, Yazılar, Raporlar, Birimler, Yapı Bileşenleri). Alan hesabında sıva payı (otomatik/yüzde/kalınlık); kolon, merdiven ve duvar alanını netten düşme; taban/tavan/duvar kaplama adları plana yazılır; DIN 277 mahal sınıfı (HFN1–6, NNF, VF, FF); II BV faktörü; mahali DIN 277, II BV, hacim, TAKS veya KAKS raporundan dışlama; malzemeyle metraj | I7 | Mahal kimliği, etiket, numara var (PLAN ARCH-SPACE-02) | Mahalde "sıva payı", "düşülecekler", "kaplamalar" alanları; alan şemasına girdi |
| **Mahal listesi** | Kat bazında ya da tüm katlar; konut adı, mahal adı, alan, tavan, taban ve duvar özellikleri isteğe göre; çizime tıklayarak yerleştirme; v11'de arka plan renklendirme | I13, I4 | Mahal çizelgesi var ama model alanında, plana yapışık (M0) | "Mahal listesi (TR)" hazır çizelge şablonu, bağımsız bölüm alt toplamları; 9. fikir |
| **Kapı/pencere tipleri** | Genişlik, yükseklik, kot, etiket; tip listesi kapı ve pencere klasörlerinde | I8 | Kapı/pencere tipleri var (PLAN: Proje → Kapı/pencere tipleri) | Tip grupları + etiket biçimi; 8. fikirle numara |
| **Ölçülendirme** | İç: çizilen hat üzerindeki elemanlar otomatik; dış: dahil edilecek eleman ayardan; v11: Serbest/Dış/İç ayarları ayrı, ölçü komutları şeritte | I10, I11, I4 | Ölçü dizileri ve iç ölçü var (PLAN ARCH-DIM-01) | 6. fikir: dört cephe varsayılanı |
| **Kalem/renk** | 20 kullanıcı tanımlı kalem kalınlığı (mm), 120 renklik palet; Ayarlar ya da Genel Ayarlar sekmesinden (sayfa yaklaşık 10 yıllık, arama özeti) | I9 | Kalem seti ADV-06 kapsamında (PLAN) | Renk → kalınlık tablosu ofis profilinde ("ruhsat" profili) |
| **Mimari + statik tek dosya** | Mimari plan modu ↔ kalıp planı modu (Shift+Tab); duvar/kapı/pencere yalnız mimari modda; kiriş/döşeme komutu kalıp moduna geçirir; v11 statik ve mimari veriyi aynı dosyada tutar; "Mimari/Statik ile Uyumlu Yap" | I3, I2, I19 | Aks ve kolon var (PLAN ARCH-ELEM-03) | Statik altlık (aks, kolon, perde) içe alma + çakışma denetimi; tam statik kapsam dışı |
| **Metraj** | Duvar ve iç sıva metrajı otomatik, kapı/pencere boşlukları düşülür; tavan sıvası için mahal gerekli (arama özeti) | I18 | Çizelge ve metraj var (PLAN ARCH-DOC-02) | Mahalden sıva/boya/tavan metrajı |
| **Ruhsat paftaları** | "Otomatik oluşturulan kesit/görünüşlerle birlikte, çıktıya hazır ruhsat paftaları" (ürün sayfası, arama özeti) | I1 | Kesit yok, pafta takımı kısmen (PLAN §2/6, §4/8) | 13. fikir (ruhsat yayın seti) + [2,5B] kesit |
| **Türkçe yardım** | Bilgi Bankası (help.idecad.com.tr); v11'de F1 tuşu komuta özel sayfayı açıyor; resmî forum; YouTube oynatma listeleri (Mimari Eğitim Videoları, Yeni Özellikler) | I4, I20 | — | Komut başına F1 → Türkçe yardım sayfası |

**İdeCAD'de doğrulanamayanlar:** otopark hesabı (yalnız rampa modelleme forum soruları), belediye/e-onay
entegrasyonu, Türk standardına göre hazır tarama/sembol kitaplığı, yangın/kaçış hesapları, 2026 sürüm numarası, IFC.

---

## 5. Doğrulanamayanlar ve açık sorular

- **Revit:** çizelgeden modele yazma (hücre düzenle → eleman) resmî metinde görülmedi; yalnız model → çizelge
  güncellemesi alıntılandı [R18]. Revit'te otomatik ölçülendirme, favori/ön ayar paleti ve giydirme cephe bu
  çalışmada açılmadı.
- **Revit LT:** karşılaştırma tablosunda onay işaretleri aynı resim; sütunlar sıra ve sayıdan çıkarıldı [R2].
  Giydirme cephe satırı tabloda yok.
- **Archicad:** Teamwork, iç görünüş ve Favoriler'in varlığı göç kılavuzundaki başlıklardan ve zon sayfasından
  çıkarıldı [A16, A8]; ayrı sayfaları açılmadı. AC29 yeni özellik listesi Graphisoft topluluk sayfasının arama özetinden.
- **Vectorworks:** Project Sharing, kaydedilmiş görünümler (saved views), iç görünüş üretimi açılmadı.
- **Allplan:** allplan.com 403 verdiği için 2026 özellikleri arama özetinden; tip sistemi, katmanlı duvar, revizyon,
  ekip çalışması, IFC açılmadı. Allplan 2027 duyurusu doğrulanamadı.
- **ARCHLine.XP:** oda defteri, otomatik ölçü ve kat filtreleme bilgisi sürümsüz ya da arama özetinden; tadilat evresi
  doğrulanamadı.
- **Chief:** X18 güncel; X17 maddeleri PDF'lerin arama özetinden. IFC özellik sayfasında yok.
- **Ekran görüntüleri:** Graphisoft pazarlama görsellerinin hangi özelliğe ait olduğu sayfa düzeninden çıkarıldı;
  İdeCAD forum eki 403 veriyor (listeye alınmadı).

---

## 6. Kaynaklar

### Revit / Revit LT (Autodesk)
- **R1** What's New in Revit 2027 (2027–2027.3): https://help.autodesk.com/cloudhelp/2027/ENU/Revit-WhatsNew/files/GUID-C81929D7-02CB-4BF7-A637-9B98EC9EB38B.htm
  - Alt sayfalar (aynı klasör, `…/Revit-WhatsNew/files/<GUID>.htm`): duvarı duvara barındırma `GUID-167B3F56-3CAA-48BB-96D6-966296B36EA8`;
    çizmeden önce mahal sınırlamaz `GUID-1AEAFD34-194F-4913-8020-971FC9A0A218`; kural tabanlı numara `GUID-B0BC7A1A-1EE8-4034-9793-98D627119B90`;
    Tag by Example (2027.2) `GUID-0C4E7BAC-14BE-4C6E-BC12-61BF01307814`; etiket çizgisi `GUID-75C0FC63-4624-411F-8D4F-3BFB68C0ABAF`;
    çok kategorili etiket `GUID-A143BF7D-8D6E-4EB5-8FC0-8968FF515DCA`; bağlı modelde çizgi kalınlığı `GUID-24B815B9-BB04-4BA9-8C90-83EDF219609F`;
    merdiven numarası `GUID-1121752C-EAB2-411F-A549-66E599A820B8`; tasarım seçenekleri `GUID-435A552A-BF5E-437E-B42A-F38557121639`;
    Isolate Annotations (2027.3) `GUID-A92E300F-6794-4409-9431-C3927C52ABA5`; Options Bar kaldırılması `GUID-049762B4-F052-4F87-ACBD-BF757A26178E`;
    Auto Expand Sheet Views `GUID-5C3B1218-BB7B-47DC-AB47-94B467B689A1`; IFC Parameter Mapping `GUID-C057D715-1571-4D11-8F02-417F3B385577`;
    Public MCP Server `GUID-97697CBF-0E11-484E-96E5-4277E3E8D61F`; Autodesk Assistant `GUID-68D8FE6D-C5B0-4503-AE27-02C715BAC25B`.
- **R2** What Is Revit LT? (2025, Revit/LT karşılaştırması): https://help.autodesk.com/cloudhelp/2025/ENU/RevitLT-GetStarted/files/GUID-186DDFB1-4508-4FE0-881C-512453501C9F.htm
- **R2b** What's New in Revit LT 2027: https://help.autodesk.com/cloudhelp/2027/ENU/RevitLT-WhatsNew/files/GUID-C81929D7-02CB-4BF7-A637-9B98EC9EB38B.htm
- **R3** Room Boundaries (LT 2024): https://help.autodesk.com/cloudhelp/2024/ENU/RevitLT-ArchDes/files/GUID-C7338362-6D35-471B-90B0-6A893534FFE2.htm
- **R4** Use Room Boundaries in a Linked Model (LT 2026): https://help.autodesk.com/cloudhelp/2026/ENU/RevitLT-ArchDes/files/GUID-64AF3A48-C121-4F26-B0BF-CDF93B389F75.htm
- **R5** About Room Area (2027): https://help.autodesk.com/cloudhelp/2027/ENU/Revit-ArchDesign/files/GUID-9348929C-18CA-40AC-BB64-71D01CC52F2B.htm
- **R6** About Temporary Dimensions (2022): https://help.autodesk.com/cloudhelp/2022/ENU/Revit-DocumentPresent/files/GUID-74FFB00B-FB29-44B7-8FEA-613DBE010AD4.htm · Temporary Dimensions (2021): https://help.autodesk.com/cloudhelp/2021/ENU/Revit-DocumentPresent/files/GUID-3BE4F02E-3861-4B6D-8929-97F9ABFBDB91.htm
- **R7** About Constraints (2021): https://help.autodesk.com/cloudhelp/2021/ENU/Revit-Model/files/GUID-91CBCCF3-66D1-496B-80B3-D893065D1A50.htm
- **R8** Applying Constraints with Dimensions (Revit LT 2014): https://help.autodesk.com/cloudhelp/2014/ENU/Revit-LT/files/GUID-836F504C-3AC6-4A4C-BA78-DD65DF734F20.htm
- **R9** Create Phase Filters (LT 2026): https://help.autodesk.com/cloudhelp/2026/ENU/RevitLT-DocumentPresent/files/GUID-66829195-1E29-493D-B24C-B25A5A099081.htm
- **R10** Define the Graphic Display for Phase Filters (2026): https://help.autodesk.com/cloudhelp/2026/ENU/Revit-DocumentPresent/files/GUID-92CF3963-780B-42FA-BAED-1265E3E57CA7.htm
- **R11** About Views and Visibility/Graphics in Revit LT (2019): https://help.autodesk.com/cloudhelp/2019/ENU/Revit-Collaborate/files/GUID-80516EFA-1CC3-4C2F-98C6-EF5F303663D2.htm
- **R12** Detail Level (2014): https://help.autodesk.com/cloudhelp/2014/ENU/Revit/files/GUID-0BB861D5-27CB-4276-93AE-F2E949B0D466.htm · Wall Type Properties (2023): https://help.autodesk.com/cloudhelp/2023/ENU/Revit-ArchDesign/files/GUID-C6A3DD86-FFAF-4781-8E50-5714965C24BE.htm
- **R13** Compound Structure (2020): https://help.autodesk.com/cloudhelp/2020/ENU/Revit-Model/files/GUID-08E47F82-BFA1-4CAB-B2EA-6C8DC9049AD8.htm
- **R14** View Templates (LT 2026): https://help.autodesk.com/cloudhelp/2026/ENU/RevitLT-Customize/files/GUID-A9C36610-6853-433D-AFB5-639090BD017F.htm
- **R15** Tag All Not Tagged (2025): https://help.autodesk.com/cloudhelp/2025/ENU/Revit-DocumentPresent/files/GUID-13EA386B-E613-4126-B634-67BEC622177E.htm
- **R16** Tag a Room (LT 2025): https://help.autodesk.com/cloudhelp/2025/ENU/RevitLT-ArchDes/files/GUID-5FBF2AA2-B811-4443-9DBB-9DE62191F332.htm
- **R17** Area Schemes (2022): https://help.autodesk.com/cloudhelp/2022/ENU/Revit-Model/files/GUID-194A8E37-06C5-426D-964C-D9148A1FEFE3.htm
- **R18** About Schedules (2026): https://help.autodesk.com/cloudhelp/2026/ENU/Revit-DocumentPresent/files/GUID-73090B70-8A13-4E12-909C-F25D724D5BA7.htm · Schedules: https://help.autodesk.com/cloudhelp/2026/ENU/Revit-DocumentPresent/files/GUID-F50D6FF4-859E-43A2-A2F6-81C84A1BA0EB.htm

### Archicad (Graphisoft)
- **A1** Archicad 30 New Architectural Features: https://help.graphisoft.com/AC/30/INT/Topics/005_NewFeatures/WelcometoAC30_Beta.html
- **A2** New Features in Graphisoft 2026 Product Lineup: https://www.graphisoft.com/en-us/plans-and-products/whats-new/
- **A21** Release Timeline for 2026 (Graphisoft Community; arama özeti): https://community.graphisoft.com/t5/Graphisoft-Insights/Release-Timeline-for-2026/ba-p/691819
- **A4** Archicad 29 at a glance (arama özeti): https://community.graphisoft.com/t5/Getting-started/Archicad-29-A-glance-at-the-new-features/ta-p/672706 · AI Assistant in Archicad: https://community.graphisoft.com/t5/Getting-started/AI-Assistant-in-Archicad/ta-p/668983
- **A5** View Settings Dialog Box (AC28): https://help.graphisoft.com/AC/28/INT/_AC28_Help/140_UserInterfaceDialogBoxes/140_UserInterfaceDialogBoxes-77.htm
- **A6** Partial Structure Display Settings: Effect on Elements (AC26): https://help.graphisoft.com/AC/26/INT//_AC26_Help/050_ViewsVB/050_ViewsVB-117.htm · Graphic Overrides (AC28): https://help.graphisoft.com/AC/28/INT/_AC28_Help/050_ViewsVB/050_ViewsVB-137.htm
- **A7** Specific Element Intersections and Junctions (AC29): https://help.graphisoft.com/AC/29/INT/_AC29_Help/040_ElementsVB/040_ElementsVB-272.htm
- **A8** Creating Zones (AC29): https://help.graphisoft.com/AC/29/INT/_AC29_Help/040_ElementsVB/040_ElementsVB-137.htm · Zone Stamps (AC28): https://help.graphisoft.com/AC/28/INT/_AC28_Help/040_ElementsVB/040_ElementsVB-142.htm
- **A9** Magic Wand (AC29): https://help.graphisoft.com/AC/29/INT/_AC29_Help/030_Interaction/030_Interaction-53.htm
- **A10** Layer Combinations (AC29): https://help.graphisoft.com/AC/29/INT/_AC29_Help/025_Attributes/025_Attributes-6.htm
- **A11** Renovation Filters (AC24): https://help.graphisoft.com/AC/24/INT/_AC24_Help/050_ViewsVB/050_ViewsVB-118.htm
- **A12** Automatic Exterior Dimensioning (AC27): https://help.graphisoft.com/AC/27/INT/_AC27_Help/070_Documentation/070_Documentation-49.htm · Automatic Interior Dimensioning (AC19): https://help.graphisoft.com/AC/19/INT/AC19Help/04_Documentation/04_Documentation-57.htm
- **A13** Drawing Settings (AC23, kalem seti): https://help.graphisoft.com/AC/23/INT/_AC23_Help/140_UserInterfaceToolSettings/140_UserInterfaceToolSettings-29.htm
- **A14** Zone Categories (AC26): https://help.graphisoft.com/AC/26/INT/_AC26_Help/040_ElementsVB/040_ElementsVB-139.htm
- **A15** Edit the model using Interactive Schedule (AC29): https://help.graphisoft.com/AC/29/INT/_AC29_Help/055_InteractiveSchedule/055_InteractiveSchedule-26.htm
- **A16** Migrating to Archicad 29: https://help.graphisoft.com/AC/29/INT/_AC29_Help/010_MigrationGuide/010_MigrationGuide-1.htm
- **A17** Revision Management (AC28): https://help.graphisoft.com/AC/28/INT/_AC28_Help/070_Documentation/070_Documentation-97.htm · Revisions in the Publisher (AC28): https://help.graphisoft.com/AC/28/INT/_AC28_Help/070_Documentation/070_Documentation-112.htm · Publishing Process (AC26): https://help.graphisoft.com/AC/26/INT/_AC26_Help/070_Documentation/070_Documentation-126.htm
- **A18** Revision Management Workflow (AC27): https://help.graphisoft.com/AC/27/INT/_AC27_Help/070_Documentation/070_Documentation-98.htm · Revision History Data on Layout (AC25): https://help.graphisoft.com/AC/25/INT/_AC25_Help/070_Documentation/070_Documentation-116.htm · Default Schemes in the Template (AC27): https://help.graphisoft.com/AC/27/INT/_AC27_Help/070_Documentation/070_Documentation-99.htm
- **A19** Import IFC Model (AC29): https://help.graphisoft.com/AC/29/INT/_AC29_Help/121_IFC/121_IFC-5.htm
- **A20** Archicad 29.1.0 güncelleme notu (arama özeti): https://www.graphisoft.com/en-us/downloads/archicad/updates/archicad-29-1-0/

### Vectorworks
- **V1** Vectorworks 2027 New features: https://app-help.vectorworks.net/2027/eng/VW2027_Guide/LandingPage/New_features.htm
- **V2** Vectorworks 2027 basın bülteni (arama özeti): https://www.vectorworks.net/en-US/newsroom/vectorworks-2027 · Update 1: https://www.vectorworks.net/en-US/newsroom/2027-update-1
- **V3** Data tag settings (2026; by style / by instance simgeleri): https://app-help.vectorworks.net/2026/eng/VW2026_Guide/Annotation2/Data_tag_settings.htm
- **V4** Creating spaces with the Space tool (2026): https://app-help.vectorworks.net/2026/eng/VW2026_Guide/Spaces/Creating_spaces_with_the_Space_tool.htm
- **V5** Vectorworks 2026 New features: https://app-help.vectorworks.net/2026/eng/VW2026_Guide/LandingPage/New_features.htm
- **V6** Creating spaces from walls (2024): https://app-help.vectorworks.net/2024/eng/VW2024_Guide/Spaces/Creating%20spaces_from_walls.htm
- **V7** Concept: Phasing (2026): https://app-help.vectorworks.net/2026/eng/VW2026_Guide/Structure/Concept_Phasing.htm · Demolishing or removing a phased object: https://app-help.vectorworks.net/2026/eng/VW2026_Guide/Structure/Demolishing_removing_phased_object.htm
- **V8** Controlling visibility and appearance with phasing (2026): https://app-help.vectorworks.net/2026/eng/VW2026_Guide/Structure/Controlling_visibility_and_appearance_with_phasing.htm
- **V9** Creating wall components (2026): https://app-help.vectorworks.net/2026/eng/VW2026_Guide/Walls/Creating_wall_components.htm
- **V11** Dimensioning exterior walls (2026): https://app-help.vectorworks.net/2026/eng/VW2026_Guide/Dimensions/Dimensioning_exterior_walls.htm
- **V12** Editing drawing objects from a worksheet (2026): https://app-help.vectorworks.net/2026/eng/VW2026_Guide/Worksheets/Editing_drawing_objects_from_a_worksheet.htm
- **V13** Title Block Manager (2026): https://app-help.vectorworks.net/2026/eng/VW2026_Guide/Setup/Title_Block_Manager.htm · Concept: Title block borders: https://app-help.vectorworks.net/2026/eng/VW2026_Guide/Setup/Concept_title_block_borders.htm
- **V14** Adding and editing drawing revisions (2026): https://app-help.vectorworks.net/2026/eng/VW2026_Guide/Setup/Adding_and_editing_drawing_revisions.htm · issues: https://app-help.vectorworks.net/2026/eng/VW2026_Guide/Setup/Adding_and_editing_drawing_issues.htm
- **V15** Comparison of commands and tools by product for Vectorworks 2026 (PDF; açılmadı): https://app-help.vectorworks.net/2026/eng/assets/docs/Commands_Tools2026.pdf

### Allplan
- **L1** Release ALLPLAN 2026 ve Features ALLPLAN 2026 (403; arama özeti): https://www.allplan.com/system/releasenotes/2026/allplan-2026/ · https://www.allplan.com/products/allplan-2026-features/
- **L2** ALLPLAN 2026-1-6 ve 2026-1-3 yamaları (arama özeti): https://www.allplan.com/us_en/system/releasenotes/2026/allplan-2026-1-6/ · https://www.allplan.com/us_en/system/releasenotes/2026/allplan-2026-1-3/
- **L3** Reference Scale (2021): https://help.allplan.com/Allplan/2021-0/1033/Allplan/7542.htm
- **L4** Pattern lines and scale (2026): https://help.allplan.com/Allplan/2026-0/1033/Allplan/5870.htm
- **L5** Parameters on the "DIN 277, Floor Area" tab (2025): https://help.allplan.com/Allplan/2025-0/1034/Allplan/10149.htm
- **L6** "DIN277, Land Act" tab (2023): https://help.allplan.com/Allplan/2023-0/1033/Allplan/10172.htm · "DIN 277, Wohnfl." (2022, DE): https://help.allplan.com/Allplan/2022-1/1031/Allplan/10149.htm

### ARCHLine.XP
- **H1** New in ARCHLine.XP 2026: https://www.archlinexp.com/products/new-in-archline-xp-2026
- **H2** Help Center Overview (oda, oda defteri; arama özeti): https://help.archlinexp.com/hc/en-us/articles/4412202495889-Overview
- **H3** New in ARCHLine.XP (sürümsüz; arama özeti): https://www.archlinexp.com/overview/new-in-archline-xp
- **H4** ARCHLine.XP LT (arama özeti): https://www.archlinexp.com/products/archline-xp-lt

### Chief Architect
- **C1** X17 New Feature List (PDF; arama özeti): https://cloud.chiefarchitect.com/1/pdf/marketing/x17-new-features.pdf · X17 Detailed Features List: https://cloud.chiefarchitect.com/1/pdf/marketing/x17-detailed-features-list.pdf
- **C2** New Features in X18: https://www.chiefarchitect.com/whats-new/
- **C3** Detailed Feature List: https://www.chiefarchitect.com/products/features/
- **C4** X18 Migration Guide (PDF; arama özeti): https://cloud.chiefarchitect.com/1/pdf/documentation/chief-architect-x18-migration-guide.pdf

### İdeCAD (İdeYapı)
- **I1** ideCAD Mimari ürün sayfası (403; arama özeti): https://www.idecad.com.tr/portfolio-type/idecad-mimari-ozet/daha-fazlasi/ · https://www.idecad.com.tr/portfolio-type/idecad-mimari-ozet/
- **I2** What's new in ideCAD v11: https://idecad.com/architecture-engineering-construction/building-design/what-s-new-in-idecad-v11
- **I3** Mimari Plan Modu: https://help.idecad.com.tr/ideCAD/mimari-plan-modu
- **I4** Mimari Geliştirmeler (v11): https://help.idecad.com.tr/ideCAD/mimari-gelistirmeler
- **I5** Mahal Oluşturma: https://help.idecad.com.tr/ideCAD/mahal-olusturma
- **I6** Kontur Seçerek Mahal Oluşturma (açılmadı): https://help.idecad.com.tr/ideCAD/kontur-secerek-mahal-olusturma
- **I7** Mahal Ayarları: https://help.idecad.com.tr/ideCAD/mahal-ayarlari
- **I8** Kapı/Pencere Ayarları (arama özeti): https://help.idecad.com.tr/ideCAD/kapi-pencere-ayarlari
- **I9** Kalem Kalınlıkları ve Renk Ayarları (arama özeti): https://help.idecad.com.tr/ideCAD/kalem-kalinliklari-ve-renk-ayarlari
- **I10** İç Ölçülendirme (arama özeti): https://help.idecad.com.tr/ideCAD/ic-oelcuelendirme
- **I11** Dış Ölçülendirme (arama özeti): https://help.idecad.com.tr/ideCAD/dis-oelcuelendirme
- **I12** Kat Alanı Hesabı Ayarları: https://help.idecad.com.tr/ideCAD/kat-alan-hesab-ayarlar · Kat Alanı Tanımlama (arama özeti): https://help.idecad.com.tr/ideCAD/kat-alan-tanmlama · Örnek Katlar Alanı Hesabı: https://help.idecad.com.tr/ideCAD/ornek-katlar-alan-hesab · Arsa Tanımlama: https://help.idecad.com.tr/ideCAD/arsa-tanmlama
- **I13** Otomatik Mahal Listesi Oluşturma: https://help.idecad.com.tr/ideCAD/otomatik-mahal-listesi-olusturma · Mahal Listesi Ayarları: https://help.idecad.com.tr/ideCAD/mahal-listesi-ayarlari
- **I17** Resmî forum, "29.04.2025 ideCAD Mimari ile TAKS-KAKS (Emsal) Alanı Hesaplama & Arsa Sınırlarının Belirlenmesi" (1115): https://forums.idecad.com.tr/forum/threads/video-yay%C4%B1nland%C4%B1-29-04-2025-idecad-mimari-ile-taks-kaks-emsal-alan%C4%B1-hesaplama-arsa-s%C4%B1n%C4%B1rlar%C4%B1n%C4%B1n-belirlenmesi.18628/
- **I18** Duvar, İç Sıva ve Tavan Metrajı (arama özeti): https://help.idecad.com.tr/ideCAD/duvar-ic-siva-ve-tavan-metraji
- **I19** Mimari/Statik ile Uyumlu Yap (açılmadı): https://help.idecad.com.tr/ideCAD/mimari-statik-ile-uyumlu-yap
- **I20** Resmî YouTube oynatma listeleri (forumdan bağlantılı): Mimari Eğitim Videoları https://www.youtube.com/watch?v=_oX2mKPE18M&list=PLxqm7PJuC2PtXDUob0OV0qM8zyF7Y2iA_ · Yeni Özellikler ve Geliştirmeler https://www.youtube.com/playlist?list=PLxqm7PJuC2PutXSmlh50rhDtjpwIF3OSR
- Ek: forumda "ideCAD Mimari'nin ayırt edici özellikleri" (Administrator, 1 Temmuz 2024; genel tanıtım, teknik ayrıntı yok): https://forums.idecad.com.tr/forum/threads/idecad-mimarinin-ay%C4%B1rt-edici-%C3%B6zellikleri-nelerdir.16969/

### Edificius (ACCA)
- **E1** Rooms video tutorials (arama özeti): https://video.accasoftware.com/en/edificius/id9/index.html · BibLus, Defining rooms and levels: https://biblus.accasoftware.com/en/defining-rooms-and-levels-in-a-bim-software/ · Architecture Design Software: https://www.accasoftware.com/en/architecture-design-software

### İdeCAD ekran görüntüleri (HEAD 200)
- Kat alanı hesabı ayarları: https://help.idecad.com.tr/__attachments/a_f96d08b18f4663767e66bafdc03961a916c4b090a295f292d17fd2d0f8a5bb0c/image-20250320-085316.png?cb=7f5ab9ec36bd2ef5db9c612f270dd553
- Mahal listesi: https://help.idecad.com.tr/__attachments/a_5fb7f96005edf5aeb7d2b23a14755a88605434a700d8c1d4d2302edcae69cb85/image-20201022-130405.png?cb=240dc25ea622da6cdf433a38ad99153d
- Mahal ayarları (ilk görsel): https://help.idecad.com.tr/__attachments/a_601d840d3a33e6378a787e78a2fdb4dccd55f80a74bd11d9b084ac27f743e8da/image-20201026-135511.png?cb=387f5f9e4c40280f20df97cc76e6f906
