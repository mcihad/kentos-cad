# W3 — Pratik/hızlı plan çizimi: ürünler ve "hız" etkileşim desenleri

Araştırma ajanı W3 · 7 Kasım 2026 · Kapsam: hızlı plan uygulamaları (SketchUp 2026 + LayOut, Sweet Home 3D 7.x,
Rayon, Floorplanner, RoomSketcher, Planner 5D, Cedreo, magicplan, SmartDraw, Archisketch, Snaptrude, Arcol, Qonic,
Home Designer/Chief Architect, HomeByMe) ve profesyonel araçların hız desenleri (Archicad, Revit, Vectorworks,
BricsCAD, AutoCAD). AutoCAD/DWG alternatiflerinin ve BIM'in ayrıntı özellikleri ile Türkiye mevzuatı başka raporlarda.

## Okuma notları

- Her desende dört satır var: **Nasıl çalışır** (resmî kaynaktan; kısa alıntılar özgün dilde), **Kaynak**,
  **Görsel** (img src) ve **Kazanç**.
- **Görsel** etiketleri:
  - **[tam]**: sayfanın verdiği tam URL.
  - **[türetildi]**: sayfa göreli yol veriyordu; URL sayfanın klasöründen kuruldu, açılışı denenmedi.
  - **[imzalı]**: imzalı bağlantı, süresi dolabilir.
  - **[yok]**: resmî sayfada kullanılabilir görsel bulunamadı.
- **Kazanç** satırları ölçüm değildir. Ya üreticinin iddiasıdır (tırnak içinde, "üretici iddiası" diye işaretli) ya da
  benim akış sayımımdır ("tahmin"). KentOS'ta `kullan` kabul senaryolarıyla ölçülmelidir.
- "Arama özeti" diye işaretlenen bilgiyi yalnız arama sonucunun özetinden okudum, sayfanın kendisini açmadım (403, 404
  ya da bütçe). Bu bilgi ikinci derecedir.
- Dosya indirmedim. Yalnız bir görselin (Archicad `TrackerEditing.png`) gerçekten açıldığını denetlemek için URL'sini
  WebFetch ile açtım; araç onu kendi önbelleğine kaydetti. Başka görsel açmadım.
- KentOS'un bugünkü durumunu aynı klasördeki `M0-kendi-gozlemim.md` ve `arch_cmds.txt` dosyalarından okudum.
  3. bölümdeki öneriler bu iki dosyaya göre yazıldı.
- Yöntem: yaklaşık 60 WebSearch/WebFetch çağrısı. Üretici yardım ve belge sayfaları önce geldi, blog ve pazarlama
  sayfaları ancak onlardan sonra.

## 0. Çıkış noktası: M0'da gözlenen durum

- **Etkileşim yükü:** 2+1 daire kabul senaryosu yaklaşık 157 etkileşim tuttu (11 araç, 15 odak değişimi, 28 yazma,
  48 tuş, 38 tık). Kayıtlar kenetlemeyi kapatıyor ve araçları kimlikle başlatıyor; şerit hiç kullanılmıyor.
- **Değer girişi:** Resimlerde imleç yanında sayı kutusu görülmedi. Araç ayarları Öznitelikler panelinde, seçenekler
  ise komut satırında çip olarak duruyor.
- **Görünüm ve katmanlar:** Dikdörtgen duvar aracının varsayılanı **Stil: Yok**. 54 nesnenin hepsi tek katmanda.
- **Ölçü ve etiket:** Ölçü yalnız alttaki dış zincirde. Etiketler duvar ve merdiven çizgilerinin üstüne biniyor.
- **Arayüz:** Şeritte mobilya/vitrifiye kataloğu yok (yalnız Mutfak). İkonlar birbirine benziyor. 1100 px genişlikte bağlam
  sekmesi ekrana sığmıyor.
- **Güçlü taraf (ayrıntı):** Mimari komut kümesi zengin:
  - Mahal aracı içine tıklayınca odayı ve alanını buluyor; Toplu mahal bul var.
  - Duvarı kaydır, önizlemede iki yanın net mesafesini ve mahal alanlarını söylüyor.
  - Duvar birleşimi kuralları, kapı/pencere tipleri, çizelgeler, Pafta seti ve evreler var.
- **Eksik olan (hız katmanı):** İmleç yanında değer girişi, görünür kenetleme, nesnenin yanında eylem, akıllı
  varsayılanlar ve kendiliğinden belgeleme.

---

## 1. Desen kataloğu

### A. Oda mı önce, duvar mı önce: otomatik oda ve otomatik birleşim

**A1. Sürükleyerek dikdörtgen oda çizmek (oda önce)**
- **Floorplanner — nasıl çalışır:** Build → "Draw Room" sekmesinde "drag out a 4 sided room from corner to corner".
  Çok kenarlı odalar "Draw Wall" ile duvar duvar çizilir. Bütün yolların ortak kuralı: "Both options require a closed
  circuit of walls to achieve the automated creation of a floor and ceiling."
  - Kaynak: https://help.floorplanner.com/en/articles/2405826
  - Görsel [tam]: https://euw1.frontkb-cdn.com/attachments/9732503/313026/b6df431f-b928-4d3a-86f9-19f9aca203b8.gif
    (Draw Room) ·
    https://euw1.frontkb-cdn.com/attachments/9732503/313026/f628e6c0-5af1-47a7-8929-99417bfa6b80.gif (duvar duvar)
- **Archisketch — nasıl çalışır:**
  - Oda aracı (F): bir başlangıç, bir bitiş tıklaması. Kesin ölçü için bitişten önce yatay boy yazılır, Enter ya da
    Tab ile dikey boya geçilir, Enter bitirir.
  - Duvar aracı (L): referans (merkez/iç/dış), kalınlık (varsayılan 200), Snap ve Orthogonal kip seçilir. Zemin
    kendiliğinden oluşur, ama yalnız "when the start and end points meet".
  - Kaynak: https://docs.archisketch.com/guide/user-en/make-project/draw-floor-plan
  - Görsel [tam] (sayfadaki 1. görsel; alt metin yok, hangi adımı gösterdiği belli değil):
    https://1040484064-files.gitbook.io/~/files/v0/b/gitbook-x-prod.appspot.com/o/spaces%2Fs45pRyTQeyYi0T0toKii%2Fuploads%2Fgit-blob-dbf577cf317dbd4c464777e6f53139121cddc1a4%2Fdraw-floor-plan-1.png?alt=media
- **Planner 5D ve HomeByMe — nasıl çalışır:** Hazır oda biçimi yerleştirilir, sonra boyutlandırılır. Serbest kalem
  de var.
  - Planner 5D sayfası: "Move walls, resize rooms, place furniture" ve "Start from scratch or use a template".
  - HomeByMe rehberi: "room shape tool to insert rooms and then adjust the size".
  - Kaynak: https://planner5d.com/use/floor-plan-drawer · https://home.by.me/en/guide/how-to-draw-floor-plan
    (HomeByMe: arama özeti)
  - Doğrulanamadı: Planner 5D destek makalesindeki "predefined room forms … freehand pencil and a square room"
    ifadesi yalnız arama özetinde geçiyor; makale 404 verdi.
  - Görsel [tam] (genel arayüz):
    https://planner5d.com/assets/images/landing-seo/2d-floor-plan/intro_web@2x.webp?v=6.18.64
- **Kazanç (tahmin):** Dikdörtgen oda tek sürüklemeyle (isteğe bağlı iki sayıyla) çiziliyor; duvar duvar çizmek en
  az beş tık ve kapama ister. Zemin, tavan ve oda kendiliğinden oluşuyor.

**A2. Oda kutularından binaya geçmek ("Space Planning" → "Build House")**
- **Chief Architect / Home Designer — nasıl çalışır:**
  1. Tools > Space Planning > **Space Planning Assistant** kat sayısını, garajı ve her kattaki oda tiplerini sorar
     (her yatak odasına bir dolap da ekler). Cevaplardan oda kutuları üretilir.
  2. Kutular düzenlenir: ortadaki tutamaç taşır, kenar tutamacı boyutlandırır, köşedeki elmas biçimlendirir.
     "Add Break" kutuya dörtten fazla kenar verir; çakışan alanlar için ayrı araçlar var.
  3. Her katta **Build House** kutu kenarlarını varsayılan duvar tipleriyle duvara ve odaya çevirir.
  - X16 / Home Designer 2025 ve sonrasında kutunun çevresindeki çizgiler, oluşacak duvarların önizlemesidir.
  - Kaynak: https://www.chiefarchitect.com/support/article/KB-00349/using-the-space-planning-tools.html
  - Görsel [tam]:
    - https://d2haqc5836ials.cloudfront.net/live/KB-00349/349-Welcome-16.png?1720751144 (asistan)
    - https://d2haqc5836ials.cloudfront.net/live/KB-00349/349-DefaultAssistantBoxes-16.png?1720751144 (oda
      kutuları)
    - https://d2haqc5836ials.cloudfront.net/live/KB-00349/349-Walls-16.png?1720751144 (duvarlar üretildi)
- **Kazanç (tahmin):** İlk yerleşimde duvar, birleşim ve kalınlık düşünülmüyor. n oda için n kutu yerleştirmek ve
  tek komut çalıştırmak bütün katı duvara çeviriyor.

**A3. Önce program ve kütle, sonra yapı elemanı (Snaptrude, Arcol)**
- **Snaptrude 3.0 — nasıl çalışır:** Dört kip art arda kullanılıyor.
  - Program Mode ihtiyaç programını "live, Excel‑like table linked to the 3D model" yapar.
  - Design Canvas'ta yüzey itildikçe alan yeniden hesaplanır.
  - BIM Mode kütleleri "with a click" duvara, döşemeye ve Revit ailelerine çevirir.
  - Present Mode PDF/DWG'yi tek tıkla verir.
  - Program CSV/XLSX'ten içeri alınabiliyor ve alan panosu kendiliğinden güncelleniyor (arama özeti).
  - Kaynak: https://www.snaptrude.com/blog/snaptrude-3-0 · https://help.snaptrude.com/en/articles/10902764-import-program-data
    (arama özeti)
  - Görsel [tam]:
    https://cdn.prod.website-files.com/63ea37f826113a5029824f10/685bebdce493822916bc08f0_v3-thumb.avif
- **Arcol — nasıl çalışır:** Alan tiplerine adet ve büyüklük hedefi konur. Hedefler tasarımı "in real time while you
  model" denetler; eşik dışında kalan alan sayısı hedefin yanında görünür.
  - Kaynak: https://intercom.help/arcol/en/articles/10447961-targets
  - Görsel [imzalı; imza kaldırıldığı için açılmayabilir]:
    https://downloads.intercomcdn.com/i/o/uj7wtk23/1549134670/c19e2ae1323f052b5dcd3790198e/Screenshot+2025-05-30+at+3_21_13%E2%80%AFPM.png
- **Kazanç:** "Bu plan programı karşılıyor mu?" sorusu ayrı bir çizelge ya da hesap turu gerektirmiyor.

**A4. Önce duvar; döngü kapanınca oda kendiliğinden**
- **Cedreo:** "Cedreo's room auto-detection kicks in as you close off areas"; ölçüler ve oda adları "as you go"
  geliyor (üretici blogu).
  - Kaynak: https://cedreo.com/blog/how-to-draw-floor-plans/
  - Görsel [yok]: sayfadaki görseller yer tutucu, gerçek URL yok.
- **Aynı kuralı izleyenler:**
  - Floorplanner: kapalı duvar devresi zemin ve tavanı üretir (A1).
  - Archisketch: uçlar buluşunca zemin oluşur (A1).
  - RoomSketcher: alan görünmüyorsa oda tam kapanmamıştır. Kaynak:
    https://help.roomsketcher.com/hc/en-us/articles/203472452-Display-the-Area-of-a-Room-PC-Mac- (403; arama özeti)
- **Kazanç:** Ayrı bir "oda oluştur" adımı gerekmiyor.

**A5. İçine tıklayınca (ya da çift tıklayınca) oda: duvarlardan oda**
- **Sweet Home 3D — nasıl çalışır:** Duvarlarla çevrili kapalı yüzeye çift tıklamak odayı kendiliğinden oluşturur;
  oda, duvarlarındaki kapıların "half doorstep"ini de içine alır.
  - 7.0'da oda oluşturma ipucuna köşegen düzenleme eklendi ("diagonal editing in the room creation tool tip").
  - 7.2'de plan bağlam menüsüne "Recompute room points" eklendi.
  - Kaynak: https://www.sweethome3d.com/userGuide.jsp · https://www.sweethome3d.com/history.jsp
  - Görsel [yok]: kılavuz görselleri tembel yükleniyor. Yalnız "figure 12. Creating with a double-click a room that
    includes an half doorstep" başlığı okunabildi.
- **Rayon — nasıl çalışır:** Zone (Z) aracında "Click inside a room enclosed by walls" yeterli; tek tıklama bir şekil
  ve bir etiketten oluşan bölge üretir.
  - Kapalı odanın dışına tıklamak sınırı çizdirir.
  - Çift tıklama düzenleme noktalarını açar.
  - Zone divider (DV) bir bölgeyi ikiye ayırır.
  - Kaynak: https://docs.rayon.design/documentation/design/elements-walls-zones-openings-etc.md
  - Görsel [tam]:
    https://2112124689-files.gitbook.io/~/files/v0/b/gitbook-x-prod.appspot.com/o/spaces%2FngVHKyaUSSJPTFck3wkz%2Fuploads%2Fhh1I4tRSUh9JouecAZk7%2Fcreate-zone.webp?alt=media&token=2c4f5bc7-e586-4fde-87f9-cd447aa595e0
- **Revit — nasıl çalışır:** Sınırlayıcı elemanların içine konan oda sınırlara kadar genişler ve alanı hesaplanır
  (2012 kılavuzu, arama özeti).
  - "Tag on Placement" odayı yerleştirirken etiketler. Sonradan "Tag All Not Tagged" eksik etiketleri topluca koyar.
  - "Place Rooms Automatically" ilgili görev olarak listeleniyor; adımlarını doğrulayamadım.
  - Kaynak: https://help.autodesk.com/cloudhelp/2026/ENU/Revit-ArchDesign/files/GUID-CB78C07F-5878-4053-8014-D23FAA45F001.htm
    · https://help.autodesk.com/cloudhelp/2025/ENU/Revit-ArchDesign/files/GUID-65668F88-4429-4992-AA0D-1C4FBDCFA650.htm
  - Görsel [yok].
- **Archicad Magic Wand — nasıl çalışır:** Space tuşuyla (ya da Design > Outline Polygon with Magic Wand) çalışır.
  - Kenara ya da düğüme tıklamak o elemanı ve zincirini izler.
  - Boş alana tıklamak "closed geometric area formed by the nearest elements"ı izler.
  - Sonuç, araç kutusunda seçili türde (dolgu, çatı, duvar…) yeni bir elemandır.
  - Pet Palette'teki "Subtract from Polygon" ile çokgenden alan çıkarılabilir.
  - Kaynak: https://help.graphisoft.com/AC/29/INT/_AC29_Help/030_Interaction/030_Interaction-53.htm
  - Görsel [türetildi] (sayfada "MW3.png": kesişen döşemelerin üstünde duvar):
    https://help.graphisoft.com/AC/29/INT/_AC29_Help/030_Interaction/MW3.png
- **Kazanç (tahmin):** n köşeli oda için n+1 tık yerine bir tık ya da çift tık.

**A6. Duvarlar kendiliğinden birleşsin, istisnası "ağ" ile ayrılsın**
- **Rayon — nasıl çalışır:** Duvarlar dokundukları duvarlarla kendiliğinden birleşir.
  - "Wall networks": farklı ağdaki duvarlar dokunsalar da birleşmez. Ağ sağ tık > "New wall network" ile kurulur;
    Katman panelinde listelenir.
  - Sağ tık > Convert > To free wall duvarın yolunu tamamen düzenlenebilir yapar. Belge şu uyarıyı veriyor:
    "will not transfer its style tokens".
  - Uzunluğu değiştirmek için duvarın yardımcı ölçüsüne tıklamak yeter (C3).
  - Kaynak: https://docs.rayon.design/documentation/design/elements-walls-zones-openings-etc.md
  - Görsel [tam]:
    - Birleşim (alt metin: aynı T birleşimi ağsız birleşik, ağla ayrık):
      https://2112124689-files.gitbook.io/~/files/v0/b/gitbook-x-prod.appspot.com/o/spaces%2FngVHKyaUSSJPTFck3wkz%2Fuploads%2F710p3iai2gissyf3FfT4%2Fwall-networks-join.png?alt=media
    - Duvar çizimi:
      https://2112124689-files.gitbook.io/~/files/v0/b/gitbook-x-prod.appspot.com/o/spaces%2FngVHKyaUSSJPTFck3wkz%2Fuploads%2FKSBO1Hc2aBCFD5F2qiFn%2Fcreate-wall.webp?alt=media&token=982c0c9a-106a-418a-a425-6d39ccec3e9a
- **Revit:** Options Bar'daki "Chain" açıkken çizilen duvarlar birbirine bağlı devam eder (Autodesk Learn, arama
  özeti).
- **Kazanç:** "Her şey kendiliğinden birleşir" hızı korunuyor. Bina derzi ya da mevcut/yeni duvar gibi istisnalar tek
  bir ayarla ayrılıyor.

### B. Çizerken sayı yazmak: imleç yanı HUD ve data bar

**B1. SketchUp Measurements box: yazmaya hemen başla**
- **Nasıl çalışır:** Çizerken sayı yazıp Enter'a basmak kesin uzunluğu verir; önce kutuya tıklamak gerekmez.
  - Köşeli parantez mutlak koordinattır: `[3',5',7']`.
  - Açılı parantez başlangıca göre göreli koordinattır: `<1.5m,4m,2.75m>`.
  - Uzunluk sonradan Entity Info'dan da düzeltilebilir.
- Kaynak: https://help.sketchup.com/en/sketchup/introducing-drawing-basics-and-concepts
- Görsel: bu desen için ayrı görsel yok; çıkarım görselleri D1'de.
- **Kazanç:** Değer girmek için odak değişmiyor (sıfır odak değişimi).

**B2. Archicad Tracker**
- **Nasıl çalışır:** Tracker "follows your mouse movements".
  - Varsayılan parametre kalın yazılır. Doğrudan yazılan sayı o alana gider ("Enable direct input into Tracker
    fields", varsayılan açık).
  - Tab sonraki alana geçer. `X`, `Y`, `Z`, `D/R`, `A` ve ardından değer (ör. "X3 or Y-4") doğrudan o alana yazar.
  - `/` XY ile uzaklık/açı gösterimi arasında geçiş yapar.
  - Değerden sonra işleç yazılabilir ("2*", "3/").
  - "Expand Tracker Automatically" bütün alanları baştan açar.
  - Enter uygular ve çıkar, Esc vazgeçer, N Tracker'ı anında gösterir.
- Kaynak: https://help.graphisoft.com/AC/28/INT/_AC28_Help/030_Interaction/030_Interaction-40.htm · AC29 ayarları:
  https://help.graphisoft.com/AC/29/INT/_AC29_Help/140_UserInterfaceDialogBoxes/140_UserInterfaceDialogBoxes-8.htm
- Görsel:
  - [tam; açıldığı denendi] https://help.graphisoft.com/AC/28/INT/_AC28_Help/030_Interaction/TrackerEditing.png
  - [türetildi] https://help.graphisoft.com/AC/28/INT/_AC28_Help/030_Interaction/DistanceAngle.png
- **Kazanç:** Komut satırına gitmeden birden çok değer giriliyor; harf kısayolu alanı doğrudan seçiyor.

**B3. AutoCAD Dynamic Input**
- **Nasıl çalışır:** Belge onu "a command interface that displays near the cursor" diye tanımlıyor. Üç parçası var:
  komut istemleri, imleç girişi (mutlak ya da göreli; kutupsal ya da kartezyen) ve boyut girişi (uzunluk ve açı).
  - DYNMODE 0–3 arası değer alır; varsayılan 3.
  - Kartezyen girişte Tab, virgül yerine sonraki ipucuna geçer.
  - Tutamaç ve boyut istemlerinde Tab açı ile uzunluk arasında geçer; yazılan değer kilitlenir.
  - Uygun olduğunda boyut girişi imleç girişinin önüne geçer.
- Kaynak: https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-MEP/files/GUID-07AF5E9F-8A0D-434A-BD06-B80596591319.htm
  · https://help.autodesk.com/cloudhelp/2023/ENU/AutoCAD-Core/files/GUID-1ED138FF-2679-45C4-9C2C-332A821C9D12.htm
  · https://help.autodesk.com/cloudhelp/2023/ENU/AutoCAD-Core/files/GUID-38EC86CF-D96A-455F-A5DE-2CDA23C28FC4.htm
- Görsel [türetildi] ("Dimension input fields"):
  https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-MEP/images/GUID-6BC85909-DE19-4C8B-A990-9D147992ADB9.png

**B4. Vectorworks Data bar**
- **Nasıl çalışır:**
  - Tab ilk alana, Shift+Tab son alana götürür.
  - Bir alandayken Tab değeri işler ve sonraki alana geçer; Enter değeri işler ve odağı çizim alanına döndürür.
  - Çizim alanında Enter nesneyi ya da segmenti bitirir.
  - Backspace henüz işlenmemiş girişi siler.
  - "Instant activation" açıksa yazmaya başlamak ilk alanı açar. Değer, ilk tıklamadan önce de girilebilir.
  - Girilen değerin yeri kırmızı kesikli bir "vector lock" çizgisiyle gösterilir.
- Kaynak: https://app-help.vectorworks.net/2025/eng/VW2025_Guide/Basic1/Drawing_with_the_Data_bar.htm
- Görsel [türetildi]: https://app-help.vectorworks.net/2025/eng/VW2025_Guide/Basic1/draw_obj_with_Data_bar.png

**B5. Basit uygulamalarda aynı desen**
- **Sweet Home 3D:**
  - Duvar çizerken Enter'dan sonra uzunluk ve açı yazılır (Online ve Mobile sürümlerde yok).
  - Duvar açısı varsayılan olarak 15° katlarına mıknatıslanır; Alt (Windows) ya da cmd (macOS) basılıyken mıknatıs
    geçici kapanır.
  - 7.0 araç çubuğuna mıknatıs düğmesi ekledi.
  - Kaynak: https://www.sweethome3d.com/userGuide.jsp · https://www.sweethome3d.com/history.jsp
- **RoomSketcher:**
  - Çizerken duvar uzunluğu duvarın üstünde ve altında görünür. Duvar ucu seçilip kutuya kesin uzunluk yazılır
    (yardım makalesi 403 verdi; arama özeti).
  - Ürün sayfası: "Enter exact wall lengths as you draw".
  - Kaynak:
    https://help.roomsketcher.com/hc/en-us/articles/360000202509-How-Can-I-Draw-Walls-with-Exact-Lengths ·
    https://www.roomsketcher.com/features/draw-floor-plans/
  - Görsel [tam] ("active wall being drawn"):
    https://wpmedia.roomsketcher.com/content/uploads/2021/12/06101713/create-floor-plans.webp
- **Archisketch:** Oda aracında genişlik, Tab, derinlik, Enter (A1).
- **Kazanç (B grubu):** Uzunluk/açı/koordinat girerken göz ve fare imleçten ayrılmıyor; panel ya da komut satırına
  gidip gelme kalkıyor.

### C. Geçici ve düzenlenebilir ölçüler

**C1. Revit temporary dimensions**
- **Nasıl çalışır:** Eleman yerleştirilince ya da seçilince görünürler.
  - En yakın dik elemana kenetlenir ve tanımlı adımla değişirler ("in values of 6 cm").
  - Değere tıklayıp yeni uzaklığı yazmak elemanı taşır.
  - Yardımcı çizgi tutamacı sürüklenerek ölçü başka bir referansa bağlanır; Tab duvar yüzü, eksen gibi adaylar
    arasında döner.
  - Ölçünün yanındaki simgeye tıklamak onu kalıcı ölçüye çevirir.
  - Birden çok seçimde Options Bar'dan "Activate Dimensions" açılır.
  - Değer "=" ile başlayan bir formül olabilir (2025 yardımı, arama özeti).
- Kaynak:
  - https://help.autodesk.com/cloudhelp/2023/ENU/Revit-DocumentPresent/files/GUID-74FFB00B-FB29-44B7-8FEA-613DBE010AD4.htm
  - https://help.autodesk.com/cloudhelp/2022/ENU/RevitLT-GetStarted/files/GUID-CC0FF240-4F0E-4393-AA50-17FA1E4A58DF.htm
  - https://help.autodesk.com/view/RVT/2025/ENU/?guid=GUID-64E1FC74-2F79-42FF-8363-A1DF23448D98 (arama özeti)
- Görsel [yok]: sayfalarda görsel URL'si yok, yalnız gömülü video var.

**C2. Chief Architect temporary dimensions**
- **Nasıl çalışır:** Duvar seçilince boyu ve paralel duvara uzaklığı görünür.
  - Düzenlenebilir bir ölçünün üstünde imleç "pointing hand" olur.
  - "left click once on the dimension to edit it" → değer → Enter → "The selected wall will move until the specified
    distance is reached".
  - Auto Exterior Dimensions ve Auto Room Dimension'ın ürettiği ölçüler de "controlling dimension" olarak duvarı aynı
    yolla taşır.
  - İç duvarda ölçü, duvarın "primary wall side"ını esas alır.
- Kaynak: https://www.chiefarchitect.com/support/article/KB-01116/moving-walls-using-dimensions.html
- Görsel [tam]:
  - https://d2haqc5836ials.cloudfront.net/live/KB-01116/1116-fig2b-13.png?1779997192 (iki geçici ölçü)
  - https://d2haqc5836ials.cloudfront.net/live/KB-01116/1116-fig3b-13.png?1779997192 ("pointing hand")
  - https://d2haqc5836ials.cloudfront.net/live/KB-01116/1116-fig4b-13.png?1779997192 (giriş kutusu)

**C3. Rayon helper dimensions**
- **Nasıl çalışır:** "Click the wall's helper dimensions to change its length".
- Kaynak: A6 ile aynı sayfa.
- Görsel [tam]:
  https://2112124689-files.gitbook.io/~/files/v0/b/gitbook-x-prod.appspot.com/o/spaces%2FngVHKyaUSSJPTFck3wkz%2Fuploads%2F5rwHjaiPvKdjqLMpF69H%2Fedit-wall.webp?alt=media&token=5738442b-7535-4dbc-b1c3-82d092b3721b

**C4. SmartDraw: ölçüye uzunluğu yaz**
- **Nasıl çalışır:** Kendiliğinden görünen ölçüye uzunluk yazılır ve "the opposite wall will resize to the same
  dimension automatically".
  - Açı da yazılabilir; bağlı duvarlar ona uyar.
  - Açıklığın köşeye uzaklığı da aynı yolla düzeltilir.
- Kaynak: https://www.smartdraw.com/floor-plan/how-to-draw-a-floor-plan.htm
- Görsel [tam]:
  - https://waz.smartdraw.com/floor-plan/img/floor-plan-resize-dimensions.png?bn=1.7.2-202611031524
  - https://waz.smartdraw.com/floor-plan/img/floor-plan-adjust-angle.png?bn=1.7.2-202611031524

**C5. SketchUp Tape Measure: ölçerek ölçekle ve kılavuz koy**
- **Nasıl çalışır:**
  - Bilinen bir uzunluk ölçülür, yeni değer yazılır; SketchUp modeli yeniden boyutlandırmak isteyip istemediğini sorar
    → Evet (arama özeti).
  - 2026.1'de Ctrl (Option) üç kip arasında döner: Measure, Create Guide Lines, Create Guide Point.
  - Çift tıklama tek bir kılavuz noktası koyar.
  - Kılavuz noktaları çıkarım motoruyla birlikte çalışır.
- Kaynak:
  - https://help.sketchup.com/en/sketchup-desktop-20261
  - https://help.sketchup.com/en/sketchup/scaling-your-model-or-parts-your-model (arama özeti)
  - https://help.sketchup.com/en/using-guides (arama özeti)
- Görsel [tam] (Guide Point çıkarımı):
  https://images.ctfassets.net/yd21npj2ir9g/fe1d508bb45d3907292a7c/f99f6cace9b88131e7edc5cccefa61c0/SketchUp-Creating-Drawing-Basics17.png
- **Kazanç (C grubu, tahmin):**
  - "Taşı" komutu: komut + taban noktası + yön + mesafe = 4–5 etkileşim.
  - Geçici ölçüyle: seç + ölçüye tıkla + yaz + Enter = 3 etkileşim.
  - Ayrıca ölçü artık hem gösterge hem kumanda.

### D. Kenetleme ve çıkarım: geri bildirim nasıl görünüyor

**D1. SketchUp inference engine**
- **Nokta çıkarımları:** Endpoint, Midpoint (açık mavi nokta ve "Midpoint" ScreenTip'i), On Edge, On Face,
  Intersection, Center, Guide Point…
  - Grup ya da bileşen içindeki nokta çıkarımları magenta olur.
  - Belge diğer nokta türlerinin rengini vermiyor.
- **Doğrusal çıkarımlar:**
  - Kırmızı/yeşil/mavi eksene hizalanan çizgi o eksenin rengini alır.
  - Parallel ve Perpendicular magentadır.
  - "From Point" eksen renginde noktalı çizgidir; bir de "Through Point" var.
- **Kilit tuşları:**
  - ↑ mavi, ← yeşil, → kırmızı eksene kilitler.
  - ↓ paralel/dik kilididir (magenta).
  - Shift etkin doğrultuyu ya da düzlemi kilitler.
- **Kip döngüsü:** Line aracında ilk tıklamadan sonra Alt (Windows) / Cmd (macOS) şu kipler arasında döner: bütün
  çıkarımlar açık → kapalı → yalnız paralel/dik.
- Kaynak: https://help.sketchup.com/en/sketchup/introducing-drawing-basics-and-concepts
- Görsel [tam]:
  - Midpoint:
    https://images.ctfassets.net/yd21npj2ir9g/d186bc6c105256cd7ef5ea/1825a76b876b49871bd5323912adbc42/SketchUp-Creating-Drawing-Basics11.png
  - From Point:
    https://images.ctfassets.net/yd21npj2ir9g/bae6033c141e62abd5003a/655a818a92744a5f685468ec7e95d2a2/SketchUp-Creating-Drawing-Basics24.png
  - Parallel:
    https://images.ctfassets.net/yd21npj2ir9g/dee4805e4b89b9d3545ee0/e60e903c314eb0e32d233f90519a53df/SketchUp-Creating-Drawing-Basics26.png
  - Ok tuşuyla kilit (GIF):
    https://images.ctfassets.net/yd21npj2ir9g/de614a3dd5a3d8252edc5b/8be7e2a9e740d81409020143206285b3/su-3000083-locklinewitharrows.gif
- **SketchUp 2026.0'da çıkarımla ilgili yenilik:** Scale aracının tutamaçları Live Component'ı çıkarımla yerleştirip
  boyutlandırıyor; tutamaçlar başka geometrinin arkasında kalsa da görünüp tıklanabiliyor.
  - Kaynak: https://help.sketchup.com/en/release-notes/sketchup-desktop-20260
  - Görsel [tam] (GIF):
    https://images.ctfassets.net/yd21npj2ir9g/fa72ee0e3262f62f39f8cd/e5ca45babf0287d54d7378f5328f9b83/GIF-scale-live-components.gif

**D2. Revit snaps**
- **Nasıl çalışır:**
  - Kenetleme noktaları şekillerle (üçgen, kare, elmas…), kenetleme çizgileri kesikli çizgiyle gösterilir (2014
    yardımında yeşil kesikli).
  - Snaps iletişim kutusu nesne kenetlemelerinin kısayollarını listeler; ör. SR = Snap to Remote Objects (uzaktaki
    nesneye "jump snap").
  - Yoğun görünümde uzak kenetlemenin kapatılması öneriliyor.
  - Ayarlar oturum boyunca geçerlidir, projeye kaydedilmez.
- Kaynak: https://help.autodesk.com/view/RVT/2025/ENU/?guid=GUID-A3947840-1F30-41D7-AF75-B61687952A07 ·
  https://help.autodesk.com/cloudhelp/2027/ENU/Revit-Customize/files/GUID-3C6FDBDC-8FCC-4CEA-A887-C1FDD189D81C.htm
  (ikisi de arama özeti)
- Görsel [yok].

**D3. Vectorworks SmartCursor**
- **Nasıl çalışır:** İmlecin yanında metin ipuçları çıkar: "Endpoint", "Midpoint", "Tangent" ya da "Align H/Angle"
  gibi iki kenetlemenin birlikte geçerli olduğunu söyleyen ipuçları.
  - Yakında başka kenetleme noktası varsa ipucunun solunda bir simge belirir; belge "Zoom in or use the snap loupe"
    diyor.
  - Data bar'dan değer kilitlenmişse "Angle"/"Length" ipucu çıkar.
  - T tuşuyla "smart edge" edinilir (en çok iki tane); kenar noktalı uzantı çizgileriyle gösterilir, açıortay ve
    öteleme kenetlemeleri açılır.
  - Y tuşu ipuçlarını açıp kapar (2026 kısayol listesi, arama özeti).
- Kaynak:
  - https://app-help.vectorworks.net/2026/eng/VW2026_Guide/Basic2/SmartCursor_cues.htm
  - https://app-help.vectorworks.net/2026/eng/VW2026_Guide/Basic2/Snapping_to_a_smart_edge.htm
  - https://app-help.vectorworks.net/2026/eng/assets/docs/VWKeyboardShortcuts.pdf (arama özeti)
- Görsel [türetildi]:
  - https://app-help.vectorworks.net/2026/eng/VW2026_Guide/Basic2/Snap_NearbyIcon.png
  - https://app-help.vectorworks.net/2026/eng/VW2026_Guide/Basic2/Sedge_ExtLines.png

**D4. Basit uygulamalarda mesafe kılavuzu ve dik çizim**
- **SmartDraw:** Sürüklerken "The blue guidelines will show the distance to the nearest wall or nearest object"; duvar
  ucunda bağlanacağı yeri gösteren bir daire çıkar.
  - Kaynak: C4 sayfası.
  - Görsel [tam]: https://waz.smartdraw.com/floor-plan/img/floor-plan-guides.png?bn=1.7.2-202611031524
- **Archisketch:** Snap kipi sarı kılavuz çizgileri gösterir; Orthogonal kip duvarları 90°'ye kısıtlar (A1 kaynağı).
- **Sweet Home 3D:** 15° açı mıknatısı ve Alt ile geçici kapatma (B5).
- **Rayon:** Shift dik çizmeyi zorlar, Ctrl/Cmd kenetlemeyi yok sayar, Shift+P kenetlemeyi açıp kapar. Belge
  sürümleri arasında fark var; bir arama özeti Shift+S diyor. Kenetleme seçenekleri sağ alt paneldedir.
  - Kaynak: https://docs.rayon.design/documentation/rayons-basics/shortcuts
- **Kazanç:** "Kaç santim kaldı?" sorusu ayrıca ölçü almadan cevaplanıyor.

### E. Seçim ve bağlam: eylem imlecin yanında

**E1. Archicad Pet Palette**
- **Nasıl çalışır:** Seçili elemanın kenarına, düğümüne ya da yüzeyine tıklanınca yalnız o parçaya uygun düzenleme
  komutlarını gösterir.
  - Çoklu çizgi girişi gibi çizim sırasında da çıkar; işlem bitene kadar başka bir komuta geçilebilir.
  - F / Shift+F simgeler arasında dolaşır.
  - Ya imleci izler ("like a pet on a leash") ya da sabit bir yerde durur ("stay").
  - İçeriği özelleştirilemez.
- Kaynak: https://help.graphisoft.com/AC/27/INT/_AC27_Help/020_Configuration/020_Configuration-25.htm
- Görsel [türetildi]: https://help.graphisoft.com/AC/27/INT/_AC27_Help/020_Configuration/PetPalette1.png

**E2. BricsCAD Quad**
- **Nasıl çalışır:** Üstüne gelinen nesne vurgulanır ve Quad o nesne türü için en son kullanılan komutu gösterir.
  - QUADCOMMANDLAUNCH=1 iken sağ tık o komutu doğrudan çalıştırır.
  - Varsayılan komutun üstüne gelince Quad genişler ve son altı komutu gösterir (QUADWIDTH); diğer komutlar
    sekmelerdedir.
  - Rollover Tips nesnenin özelliklerini gösterir.
  - Üst üste binen nesneler arasında Tab ile geçilir.
  - Seçim yokken sağ tuşa uzun basmak oluşturma komutlarını açar.
  - Doğrulanamadı: rollover özelliklerinin yerinde düzenlenebildiğini yalnız BIM yardım sayfasının arama özeti
    söylüyor; getirdiğim sayfa bunu söylemedi.
- Kaynak:
  - https://help.bricsys.com/en-us/document/bricscad/getting-started-with-bricscad/exploring-the-interface/the-quad-cursor-menu
  - https://help.bricsys.com/en-us/document/bricscad-bim/getting-started/quad-cursor
- Görsel [tam]:
  - https://static.bricsys.com/help/images/V26.2/en_US/Quad_Default.png
  - https://static.bricsys.com/help/images/V26.2/en_US/Quad_Expanded.png
  - https://static.bricsys.com/help/images/V26.2/en_US/Quad_RollOverTips.png

**E3. Revit: Tab ile zincir seçimi**
- **Nasıl çalışır:** Zincirdeki bir duvarın üstüne gelinir; Tab bağlı elemanları vurgular, tıklama bütün zinciri
  seçer. Aynı noktada birden çok duvar birleşiyorsa zincir en küçük açı yapan duvardan devam eder.
- Kaynak: https://help.autodesk.com/cloudhelp/2015/ENU/Revit-Model/files/GUID-86A50B39-334D-4EE2-B2B8-F28DC6C2D26C.htm
  (arama özeti)
- Görsel [yok].

**E4. AutoCAD çok işlevli tutamaçlar**
- **Nasıl çalışır:** Çoklu çizginin köşe tutamacının üstünde beklemek bir menü açar (Add Vertex…); segmentin orta
  tutamacında "Convert to Arc" seçeneği çıkar.
- Kaynak: https://help.autodesk.com/cloudhelp/2020/ENU/AutoCAD-Core/files/GUID-6242DF04-BD00-4A5D-891C-B131184D0D36.htm
  (arama özeti)
- Görsel [yok].

**E5. Rayon: seçim araç çubuğu ve sağ tık dönüşümleri**
- **Nasıl çalışır:**
  - Eylül 2026'dan beri görsel seçilince AI araçları "in the selection toolbar" çıkıyor.
  - Sağ tık > Convert > To free wall ve sağ tık > New wall network bağlam menüsünde.
- Kaynak: https://docs.rayon.design/other-resources/product-updates/september-26-product-updates.md
- Görsel [tam]:
  https://2112124689-files.gitbook.io/~/files/v0/b/gitbook-x-prod.appspot.com/o/spaces%2FngVHKyaUSSJPTFck3wkz%2Fuploads%2F0UPKbbGwWdxIpgr2CTLo%2FVincent-F-11.png?alt=media&token=c8b8fb7d-2768-46e8-8356-91b84086ef60
- **Kazanç (E grubu):** Şeritte bağlam sekmesi aranmıyor. BricsCAD'de aynı türdeki son işlem tek sağ tıkla
  tekrarlanıyor.

### F. Katalog: arama, sürükle-bırak, akıllı yerleşim

**F1. Sweet Home 3D**
- **Nasıl çalışır:** Mobilya katalogdan plana ya da listeye sürüklenir.
  - Mıknatıs açıkken bırakılan parça, sırtı duvar boyunca uzanacak şekilde döner ("lies along the wall").
  - Daha büyük bir parçanın üstüne bırakılan parça, varsayılan kotu zemin ise onun üstüne yükselir.
  - Duvara bırakılan kapı ya da pencere duvara göre yönlenir ve boyutlanır.
  - Katalogda kategoriye ve birden çok kelimeye göre arama var. Sürüm geçmişini okuduğumda 7.3'te çıkıyordu; bir
    başka özet 7.4 / JS Online diyor.
  - Katalog ipuçlarında animasyonlu GIF simgesi gösterilebiliyor (7.0).
  - Özellik sayfası: "searchable and extensible catalog organized by categories".
- Kaynak: https://www.sweethome3d.com/userGuide.jsp · https://www.sweethome3d.com/history.jsp ·
  https://www.sweethome3d.com/features.jsp
- Görsel [yok].

**F2. SmartDraw**
- **Nasıl çalışır:** Kapı ya da pencere duvara sürüklenir.
  - Duvara tutunacağı zaman imleç "stamp"tan "anchor"a döner.
  - Sonra öğe duvar boyunca kaydırılıp mesafe kılavuzlarıyla konumlandırılır.
  - Sembol araması var.
- Kaynak: C4 sayfası.
- Görsel [tam]:
  - https://waz.smartdraw.com/floor-plan/img/floor-plan-drag-drop-door.png?bn=1.7.2-202611031524
  - https://waz.smartdraw.com/floor-plan/img/floor-plan-search-symbols.png?bn=1.7.2-202611031524

**F3. Floorplanner**
- **Nasıl çalışır:**
  - Arama süzgeçleri: yalnız boyutlandırılabilir öğeler, renk, marka. Varyantlar, benzer öğeler ve favoriler var.
  - "Lock furniture" ve "lock construction" kilitleri yanlışlıkla taşımayı önler.
  - **Magic Layout:** Odaya bir Styleboard bağlanır ve öğelere rol atanır (ör. "Dining Table" rolü yemek masasına).
    "Magic Layout" düğmesi öğeleri odaya kendiliğinden yerleştirir; düğmeye "as many times as desired" basılıp
    yerleşim yeniden dizilebilir.
- Kaynak: https://help.floorplanner.com/en/categories/579650-working-in-the-editor ·
  https://help.floorplanner.com/en/articles/2399106
- Görsel [tam] (Magic Layout küçük resmi):
  https://euw1.frontkb-cdn.com/attachments/9732503/313026/26771b09-2f44-4889-a2c3-d5372b674959.gif

**F4. Rayon**
- **Nasıl çalışır:**
  - Blok kitaplığından sürükle-bırak.
  - Opening kitaplığından kapı ya da pencere duvara sürüklenir; son adımda açılış yönü seçilir.
  - Parametrik bloklar var; kitaplıklar ve şablonlar ekiple paylaşılmak üzere yayımlanabilir.
- Kaynak: https://docs.rayon.design/documentation/rayons-basics/drawing · A6 sayfası · https://docs.rayon.design/llms.txt
- Görsel [tam]:
  https://2112124689-files.gitbook.io/~/files/v0/b/gitbook-x-prod.appspot.com/o/spaces%2FngVHKyaUSSJPTFck3wkz%2Fuploads%2FHNQjLLFoy3LEC5hhBxrM%2Fcreate-opening.webp?alt=media&token=2dfcc887-2769-42f3-8332-f17d59fc0263

**F5. LayOut 2026 Architecture Scrapbook**
- **Nasıl çalışır:** Kapı, pencere ve oda tipi gruplarını ölçekli grup olarak ekler.
- Kaynak: https://help.sketchup.com/en/release-notes/sketchup-desktop-20260
- Görsel [tam]:
  https://images.ctfassets.net/yd21npj2ir9g/bfabdefe327eaacf6debde/221123d456c6e90199f4091739b180b9/layout-scrapbook-libraries.png

**F6. Planner 5D**
- **Nasıl çalışır:** Sayfa yalnız şunları söylüyor: "over 8,000 furniture and decor items", "Automatic furniture
  placement", "Fully furnished room with Smart Wizard". Nasıl çalıştığı anlatılmıyor (üretici iddiası).
- Kaynak: https://planner5d.com/use/floor-plan-drawer
- **Kazanç (F grubu):** Sweet Home 3D ve SmartDraw'da yerleştirme, döndürme ve duvara yaslama tek bırakmada oluyor.
  Floorplanner'da bir odayı donatmak, Styleboard hazırsa yaklaşık beş tıklama (benim sayımım).

### G. Otomatik ölçü, etiket, alan ve çıktı

**G1. Floorplanner: ölçü türleri tek tek açılıp kapanıyor**
- **Nasıl çalışır:** Ayrı görünüm anahtarları var: "show auto dimension", dış otomatik ölçüler, iç ölçüler, kısa ölçü
  çizgileri, alan etiketi çerçevesi, yatay etiketler, etiket ölçeği. Odada "Show Surface Area" alanı gösterir.
- Kaynak: https://help.floorplanner.com/en/articles/2394370 ·
  https://help.floorplanner.com/en/categories/579650-working-in-the-editor ·
  https://help.floorplanner.com/en/categories/581122-room
- Görsel [yok].

**G2. Chief Architect: Auto Exterior Dimensions ve Auto Room Dimension**
- **Nasıl çalışır:** Dış ölçüler kendiliğinden üretilir; bu ölçüye tıklayıp duvar taşınabilir (C2).
  - Dış duvarlarda boşluk varsa ölçü üretilmeyebilir; çaresi Build > Wall > Fix Wall Connections.
  - Doğrulanamadı: Fix Wall Connections çaresi arama özetinden; hangi makalede geçtiğini eşleştiremedim.
- Kaynak: https://www.chiefarchitect.com/support/article/KB-01116/moving-walls-using-dimensions.html
- Görsel [tam]: https://d2haqc5836ials.cloudfront.net/live/KB-01116/1116-fig6b-13.png?1779997192

**G3. RoomSketcher**
- **Nasıl çalışır:**
  - "Show Zone Size" oda alanını gösterir.
  - **Measurement Wizards** sürükle-bırakla çalışır: Room Dimensions ve Room Measurements odanın içine, Outside
    Measurements planın dışına, Wall Measurements duvarın üstüne bırakılır. Pro ya da Team aboneliği ister.
  - Total Area kat ya da proje toplamını verir (GLA/GBA/GFA).
  - Blog: "The first six are automatic".
- Kaynak: https://www.roomsketcher.com/blog/the-7-measurement-types-in-roomsketcher/
- Görsel [tam]:
  - https://wpmedia.roomsketcher.com/content/uploads/2025/02/19112723/room-area-floor-plan-measurement.jpg
  - https://wpmedia.roomsketcher.com/content/uploads/2025/02/19110937/room-dimensions.jpg
  - https://wpmedia.roomsketcher.com/content/uploads/2025/02/19113454/outside-floor-plan-measurements.jpg

**G4. Cedreo**
- **Nasıl çalışır:** Otomatik ölçülerin üstüne üç tür ölçü daha var.
  - Wall-to-wall: iki tıklamayla çizilir; çizginin kestiği duvarlar "detected by the software, which gives you the
    option to choose the ones you are interested in"; duvar kalınlıklarını da gösterir.
  - Manual: sembole kenetlenebilir.
  - Elevation: kot 0'a göre yükseklik verir.
- Kaynak: https://help.cedreo.com/en/how-to-customize-the-measurements-on-the-2d-plan
- Görsel [tam]:
  - https://help.cedreo.com/hubfs/9-04%20wall%20to%20wall%20dimensions-gif-3.gif
  - https://help.cedreo.com/hubfs/9-04%20manual%20drawing%20dimensions-gif-2.gif

**G5. Rayon associative dimensions**
- **Nasıl çalışır:** Ölçü, çizilirken kenetlendiği noktaya bağlanır; eleman taşınınca ölçü onu izler.
  - Bağlantı kurulunca artı imlecin yanında asma kilit simgesi çıkar.
  - Özelliklerde dört durum görünür: Linked / Partially Linked / Unlinked / Mixed.
  - Bağlı ölçü elle taşınamaz.
  - Davranış Preferences > Snapping > Auto-link annotations ile kapatılabilir.
- Kaynak: https://docs.rayon.design/documentation/design/associative-dimensions.md
- Görsel [tam]:
  https://2112124689-files.gitbook.io/~/files/v0/b/gitbook-x-prod.appspot.com/o/spaces%2FngVHKyaUSSJPTFck3wkz%2Fuploads%2F2GGbB36AmRd8SoWZhKhT%2FFrame%201827%20(2).png?alt=media&token=49347597-4542-4ad9-b257-dabe02f0dc39

**G6. Revit: Tag on Placement ve Tag All Not Tagged**
- A5'te anlatıldı.

**G7. Tek tıkla çıktı**
- **Snaptrude:** Present Mode "export to PDF or DWG in one click" (A3 kaynağı).
- **Cedreo** (üretici iddiaları):
  - "one model generates all of these plans in sync (including the 3D version)".
  - "it only takes about 2 hours to draw a complete set of house plans".
  - Görünüşler "just 3 clicks".
  - Kaynak: https://cedreo.com/faq/how-to-draw-house-plans/
- **RoomSketcher:** 2B ve 3B "from the same project with just a click". Kaynak: B5'teki ürün sayfası
  (https://www.roomsketcher.com/features/draw-floor-plans/).
- **SmartDraw:** "You don't have to pick and stick to a scale"; ölçek Document Setup'tan her an değişir.
  - Görsel [tam]: https://waz.smartdraw.com/floor-plan/img/floor-plan-set-scale.png?bn=1.7.2-202611031524
- **Kazanç (G grubu):** Ölçü, etiket ve alan, çizim bittikten sonra yapılan ayrı bir iş olmaktan çıkıyor.

### H. Başlangıç deneyimi

**H1. Şablonlar ve örnek projeler**
- **Rayon:** "Templates" sekmesi topluluğun ve kişinin kendi şablonlarını bir arada gösteriyor; şablonu çoğaltmak yeni
  proje açıyor (Eylül 2026).
- **SmartDraw:** Dashboard'da "Floor Plans - Residential" gibi şablonlar.
- **RoomSketcher:** "start with a blank project or choose a floor plan template".
- **Planner 5D:** şablonlar.
- **SketchUp 2026:** Preferences'ta özel şablon klasörü tanımlanabiliyor (2026.0 notları).
- Görsel [tam]:
  - Rayon şablonları:
    https://2112124689-files.gitbook.io/~/files/v0/b/gitbook-x-prod.appspot.com/o/spaces%2FngVHKyaUSSJPTFck3wkz%2Fuploads%2F8wnc53rzv4RKAcGrs6sj%2FVincent-F-7%20(2).png?alt=media&token=480726a3-d3cb-4538-ae5b-9703ebec56a4
  - SmartDraw: https://waz.smartdraw.com/floor-plan/img/floorplan-dashboard-new.png?bn=1.7.2-202611031524

**H2. Sihirbaz**
- Chief Space Planning Assistant (A2).

**H3. Var olan plandan başlamak (yapay zekâ ve altlık)**
- **RoomSketcher AI Convert:** "detects walls, doors, and windows"; PNG/JPG/PDF kabul ediyor. Elle izleme de mümkün.
- **Rayon:** görüntüden "vector-based floor plan, with walls, openings and zones" üretiyor (Eylül 2026).
- **SmartDraw:** var olan binayı yapay zekâyla yeniden kurmak ve doğal dil tarifinden yerleşim üretmek.
- **Cedreo:** PDF, görüntü ya da DWG/DXF içeri alınıp üstünden çiziliyor.
- **HomeByMe:** 2B plan görüntüsü arka planda duruyor, duvarlar onun üstünden çiziliyor (yardım merkezi, arama özeti).
- Görsel [tam]:
  - https://waz.smartdraw.com/ai/img/recreate-existing-building.png?bn=1.7.2-202611031524
  - https://waz.smartdraw.com/ai/img/generate-floorplan-layout-ideas.png?bn=1.7.2-202611031524

**H4. Uygulama içinde öğretim**
- **SketchUp:** Durum çubuğu ve Instructor paneli etkin aracın adımlarını anlatıyor (arama özeti; menü yolu sürüme
  göre değişmiş olabilir).
- **Sweet Home 3D:** katalog ipucunda animasyonlu GIF (7.0).
- Kaynak: https://help.sketchup.com/en/sketchup/search-tools-commands-and-extensions · https://www.sweethome3d.com/history.jsp
- **Kazanç (H grubu):** İlk beş dakikada boş tuval yerine düzenlenecek bir plan karşılıyor.

### I. Klavyeyle çalışma, geri alma, hata önleme

**I1. Komut araması ve kavram araması**
- **SketchUp Search:** Shift+S ile açılır; sonuçlar yazdıkça güncellenir. Araç adı yerine amaç da yazılabilir:
  "transparent" saydamlık araçlarını getirir; web sürümünde "raise"/"lower" Move'u önerir (arama özeti).
- **Rayon komut satırı:** "Type anytime on your keyboard to search for a command"; Ctrl/Cmd+K komut satırını açar;
  Space/Enter onaylar, Esc vazgeçer.
- Kaynak:
  - https://help.sketchup.com/en/sketchup/search-tools-commands-and-extensions
  - https://help.sketchup.com/en/sketchup-web/search-tools-and-commands (arama özeti)
  - https://docs.rayon.design/documentation/rayons-basics/navigation-and-command-access.md
- Görsel [tam]:
  - SketchUp arama sonuçları:
    https://images.ctfassets.net/yd21npj2ir9g/212faa81f94da330cce707/30a4ce2b685698d11aba3a358424a8f9/su-search-results.png
  - Rayon komut erişimi ayarı:
    https://2112124689-files.gitbook.io/~/files/v0/b/gitbook-x-prod.appspot.com/o/spaces%2FngVHKyaUSSJPTFck3wkz%2Fuploads%2FprKBgfDh7evM3DRV58gD%2FVincent-F-7%20(2).png?alt=media&token=9c51ead7-8700-4b30-a61f-c1d72994cb95

**I2. Kısayol kümeleri ve komut erişim kipleri**
- **Rayon:**
  - İki kısayol kümesi var: Rayon'un kendi kümesi (PL, CI, RE, MM) ve "AutoCAD-like" küme (PLINE, C, REC, M).
    Kullanıcı ayarlarından değiştirilir.
  - İki erişim kipi var: kısayol kipi (onay tuşu yok) ve komut satırı kipi (Space/Enter ile onay).
  - Komutlarda ortak kural: `A` ana kip, `B` ikinci kip, `L` bir kez / döngü.
- Kaynak: https://docs.rayon.design/documentation/rayons-basics/shortcuts

**I3. Geri al, yinele ve geçmiş**
- **Floorplanner:** undo/redo, tasarım geçmişi ve silinen tasarımı geri getirme.
- **SketchUp 2026:** sahne oluşturma, adlandırma ve silme de geri alınabiliyor.
- **Sweet Home 3D:** bütün plan değişiklikleri geri alınabilir.
- Kaynak: Floorplanner kategori sayfası · SketchUp 2026.0 notları · Sweet Home 3D kullanıcı kılavuzu.

**I4. Çizim sırasında görünen hata önleme**
- **Archisketch:** "Unconnected ends appear in yellow"; zemin yalnız uçlar buluşunca oluşur.
- **magicplan:** Taramadan sonra "Green walls were scanned successfully"; turuncu duvar 2B planda çizilmez. Kullanıcı
  "Confirm Scan" ya da "Discard & Rescan" seçer.
- **RoomSketcher:** Alan görünmüyorsa oda kapalı değildir (arama özeti).
- **Rayon:** Bağlı ölçü elle taşınamaz; "free wall"a çevirmek stil kaybı uyarısı verir.
- **Chief Architect:** Dış duvarlardaki boşluk otomatik ölçüyü bozar; çaresi Fix Wall Connections (arama özeti).
- **SketchUp 2026:** Döndürme düzlemi kilitliyken döndürme tutamaçları gizlenir ("to avoid unexpected results").
- Görsel [tam] (magicplan):
  https://help.magicplan.app/hs-fs/hubfs/IncompleteScan.png?width=603&height=416&name=IncompleteScan.png
- **Kazanç:** Hata çizim anında görünüyor; ayrı bir denetim komutu çalıştırmak gerekmiyor.

### J. Mobil, tablet ve işbirliği (yalnız fikir olarak)

- **magicplan:** LiDAR Auto-Scan (Apple RoomPlan) tek oturumda birden çok oda tarıyor; duvar, pencere ve kapıları
  kendiliğinden buluyor, sonra "Generate Floor Plan". Manual-Scan'de Corner ve Wall kipleri var.
  - Kaynak: https://help.magicplan.app/auto-scan-your-floor-plan · https://help.magicplan.app/scan-a-room-in-seconds-using-lidar
    (arama özeti)
  - Görsel [tam]: https://help.magicplan.app/hs-fs/hubfs/AutoScanTips.png?width=603&height=416&name=AutoScanTips.png
- **Rayon mobil (Eylül 2026):** telefon ve tablette salt okunur; sahada ölçü alınabiliyor.
  - Görsel [tam]:
    https://2112124689-files.gitbook.io/~/files/v0/b/gitbook-x-prod.appspot.com/o/spaces%2FngVHKyaUSSJPTFck3wkz%2Fuploads%2FxGqqFhrD4EV1fa2SjK0L%2FVincent-F-7%20(6).png?alt=media&token=4f2243b0-51bd-4d1c-abf0-cd8f3d0b0207
- **SketchUp 2026:**
  - Collaboration Bar: özel davet ya da açık bağlantı; erişim düzeyleri View Scenes / Comment / View Model.
  - Sahneye ve kameraya bağlanabilen 3B yorum işaretleri.
  - İzleyiciler ölçü alabiliyor; değişiklikler gerçek zamanlı görünüyor.
  - Görsel [tam]:
    https://images.ctfassets.net/yd21npj2ir9g/47dc6a6ddc8684e361706b/ce779036424623cc6cae5c3f8c193784/desktop-comments-window.png
- **Arcol:** Tarayıcıda ortak çalışma, metrikler ve sunum panoları (Boards). Yalnız yardım merkezinin koleksiyon
  sayfasını gördüm (https://intercom.help/arcol/); ayrıntı doğrulanamadı.
- **Qonic:** Bağlamı anlayan "smart snapping" ve duvar araçları ticari basında anlatılıyor
  (https://aecmag.com/bim/qonic-edges-towards-bim-authoring/; ikincil kaynak). Resmî belge bulamadım.
- **KentOS için fikir:**
  1. Sahadan rölöve: telefonla LiDAR ya da elle köşe → taslak plan.
  2. Model üstünde salt okunur paylaşım: ölçü alma ve yorum (pafta değil).

---

## 2. "Pratik" ile "ayrıntılı"yı birlikte sunan ürünler bunu nasıl başarıyor?

1. **Aynı nesne, iki giriş yolu.** Fareyle kaba çizim (çıkarım/kenetleme) ve imleç yanında kesin sayı aynı aracın
   içinde:
   - SketchUp: çıkarım + Measurements box.
   - Archicad: Tracker.
   - Vectorworks: Data bar.
   - AutoCAD: Dynamic Input + komut satırı.

   Hız için ayrı bir "basit duvar" yok; hızlı da çizilse aynı duvar ayrıntılandırılıyor.
2. **Önce yaklaşık çiz, sonra ölçüyle düzelt.** Revit, Chief, Rayon ve SmartDraw'da geçici ölçüye tıklayıp değer
   yazmak yetiyor; ölçü hem gösterge hem kumanda. Ayrıntı isteyen kullanıcı geçici ölçüyü tek tıkla kalıcı yapıyor
   (Revit).
3. **İş akışı katmanları (aşamalı derinlik).**
   - Chief: kutu → Build House → duvar/oda düzenleme.
   - Snaptrude: Program → Design → BIM → Present.
   - Floorplanner: Draw Room → Draw Wall → böl/sürükle.
   - Rayon: kural duvarı → "free wall".
   - RoomSketcher: otomatik alan → sürükle-bırak ölçü sihirbazı → elle ölçü çizgisi.
   - Cedreo: otomatik ölçü → wall-to-wall → manual → elevation.

   Kullanıcı ihtiyacı olduğu derinliğe kadar iniyor.
4. **Akıllı varsayılan, bilinçli kaçış.**
   - Sweet Home 3D: mıknatıs açık, Alt basılıyken kapalı.
   - SketchUp: Alt ile çıkarım kipleri arasında döngü.
   - Rayon: Ctrl kenetlemeyi yok sayar; duvar ağı kendiliğinden birleşmenin istisnasını ayırır.
   - Revit: uzak kenetleme kapatılabiliyor.
5. **Kademeli açılan ayar.**
   - Archicad Tracker: varsayılan alan kalın; diğerleri Tab ile ya da "Expand automatically" ile açılıyor.
   - Vectorworks: instant activation.
   - AutoCAD: DYNMODE.
   - Floorplanner: her ölçü türü için ayrı anahtar.
   - Sweet Home 3D: ölçüyü sayıyla düzeltmek ayrı bir iletişim kutusunda ("Modify dimensions", 7.2).
6. **Bağlam kendini öğrenir.** BricsCAD Quad o nesne türünde en son kullanılan komutları öne alıyor; Archicad Pet
   Palette yalnız uygun komutları gösteriyor. Uzmana hız, yeni başlayana keşif sağlıyor.
7. **Erişim kişiye göre ayarlanır.**
   - Rayon: kısayol kipi ↔ komut satırı kipi; Rayon kümesi ↔ AutoCAD benzeri küme.
   - Archicad Favorites: seçili elemana uygulanır ya da varsayılan yapılır.
8. **Tek model, çok çıktı.** Cedreo "one model generates all of these plans in sync"; Snaptrude Present;
   SketchUp + LayOut Scrapbook. Pafta, çizelge ve ölçü modelden türüyor; hızlı çizilen plan sonradan yeniden
   çizilmiyor.
9. **Durum görünür, iş geri alınabilir.** Rayon'da bağlı/bağsız ölçü, magicplan'de yeşil/turuncu duvar,
   Archisketch'te sarı uç, Floorplanner'da tasarım geçmişi. Kullanıcı hızlı giderken neyin "tamam" olduğunu görüyor.

---

## 3. KentOS Mimari için en çok etki yaratacak 15 desen (öncelik sırasıyla)

Her maddede dört satır var: **neden** (M0 bulgusu), **örnek ürünler**, **öneri** ve **ölçüt**. Ölçütler `kullan`
kabul senaryolarıyla ölçülmeli: `acceptance-flat` ≈157, `change` 5 araç / 33 tık. Hepsi kenetleme açıkken ve şerit
kullanılırken yeniden kaydedilmeli; bugünkü kayıtlar ikisini de kapatıyor.

Uygulama ilkesi: HUD, bağlam çubuğu ve kenetleme işaretleri gibi görsel parçalar tek tek pencerelere değil, ortak
`kentos_ui` bileşenlerine yazılmalı. Her bileşenin belgesinde dayandığı standart adıyla anılmalı (ör. "Archicad
Tracker / AutoCAD Dynamic Input deseni"). Her madde bir plan satırı ve bir ADR olmalı.

1. **İmleç yanında sayı HUD'u, "yaz-hemen" girişi**
   - **Neden:** 15 odak değişimi ve 28 yazma; imleç yanında sayı kutusu yok; ayarlar Öznitelikler panelinde.
   - **Örnek:** SketchUp Measurements box, Archicad Tracker, AutoCAD Dynamic Input, Vectorworks Data bar.
   - **Öneri:** Bütün araçların kullandığı tek bir bileşen.
     - Etkin aracın varsayılan alanı kalın yazılır; çizim sırasında yazılan rakam doğrudan o alana gider.
     - Tab ve Shift+Tab aracın 2–3 ana alanı arasında dolaşır (ör. uzunluk → açı → kalınlık).
     - `/` kartezyen ile kutupsal arasında geçiş yapar; işleçler çalışır (`3.6+0.25`, `2*3`).
     - Elle yazılan alan kilit simgesiyle sabitlenir. Enter uygular, Esc geri alır.
     - Komut satırındaki harf seçenekleri (E, U, R…) HUD'da da aynı harfle çalışır; komut satırı uzman yolu olarak
       kalır.
   - **Ölçüt:** `acceptance-flat`'te odak değişimi 15 → ≤3, aynı sonuç geometrisiyle.

2. **Seçimde ve çizimde tıklanabilir geçici ölçüler**
   - **Neden:** Duvarı kaydır önizlemesi net mesafeyi ve alanı zaten hesaplıyor, ama kullanıcı bunu ancak komutu
     başlatarak görüyor.
   - **Örnek:** Revit, Chief Architect, Rayon, SmartDraw.
   - **Öneri:**
     - Duvar, açıklık ya da mahal seçilince boyu ve en yakın paralel duvara net uzaklığı geçici ölçü olarak çizilir.
     - Değere tıklamak HUD'u açar; yazıp Enter'a basmak Duvarı kaydır'ın aynı işlemini yapar (tek geri alma adımı).
     - Tab ölçünün referansını döndürür: İç net → Eksen → Dış ölçü. Bu adlar Dikdörtgen duvar aracındaki
       "Boy referansı" seçenekleriyle aynı.
     - Küçük bir simge ölçüyü kalıcı yapar; adım ayarı (ör. 5 cm) var.
   - **Ölçüt:** `change` senaryosu 5 araç / 33 tık → komut açmadan ≤10 tık.

3. **Görünür kenetleme dili**
   - **Neden:** Kayıtlar kenetlemeyi kapatıyor; kenetlemenin hıza katkısı bugün ölçülemiyor.
   - **Örnek:** SketchUp inference engine, Revit snaps, Vectorworks SmartCursor.
   - **Öneri:**
     - Nokta türü şekil ve renkle ayrılır (uç kare, orta üçgen, merkez daire, kesişim çarpı); grup içindeki noktalar
       ayrı renk.
     - İmleçte bir-iki kelimelik Türkçe ipucu: "Orta nokta", "Dik", "Paralel", "Uzantı", "Hizada: Y". İki koşul
       birlikte geçerliyse ikili ipucu ("Hizada Y / Dik").
     - Hizalama ve uzantı çizgileri kesikli.
     - Ok tuşları ekseni kilitler, Shift etkin doğrultuyu kilitler, Alt basılıyken kenetleme geçici olarak kapanır.
     - "T" ile kenar edinilir (Vectorworks smart edge); "Y" ipuçlarını açıp kapar; yoğun çizimde uzak kenetleme
       kapatılabilir.
   - **Ölçüt:** Kabul senaryoları `snap: true` ile yeniden kaydedilir; tık sayısı ve sonuç geometrisi karşılaştırılır.

4. **Oda-önce "Oda" aracı: bir sürüklemede duvar + mahal + ad + alan**
   - **Neden:** 2+1 için 11 araç kullanılıyor; Dikdörtgen duvar ile Mahal ayrı araçlar.
   - **Örnek:** Floorplanner Draw Room, Archisketch Room, Chief Space Planning, Planner 5D / HomeByMe oda biçimleri.
   - **Öneri:**
     - Kullanıcı sürükler ya da HUD'a "4.20 Tab 3.60" yazar. Sonuç: varsayılan stilde duvarlar, mahal (numara ve ad
       HUD'da sorulur, son kullanılan tip önerilir) ve alan etiketi.
     - Komşu odanın yanına çizilen odada "Ortak duvar" kendiliğinden uygulanır.
     - Hazır biçimler: dikdörtgen, L, U.
     - İkinci aşama: duvarsız "oda kutusu" kipi ve "Duvarlara çevir" (Chief Build House).
   - **Ölçüt:** `acceptance-flat` 11 araç → ≤6; 38 tık → ≤20.

5. **Kapanınca kendiliğinden mahal; boş odaya çift tık**
   - **Neden:** Mahal aracı içine tıklamayla çalışıyor, bu iyi; ama ayrı bir araç ve ayrı bir adım. Etiketler
     çizgilerin üstüne biniyor.
   - **Örnek:** Cedreo, Sweet Home 3D, Rayon Zone, Revit Tag on Placement.
   - **Öneri:**
     - Duvar aracı bir döngüyü kapatınca mahal önizlemesi çıkar; Enter ya da tık kabul eder (Toplu mahal bul'un tek
       odalık hâli).
     - Seçim aracındayken mahalsiz kapalı bir alana çift tıklamak mahal oluşturur.
     - Yerleştirirken etiketleme varsayılan olarak açık; etiket yeri duvar ve merdiven çizgilerinden kaçar.
   - **Ölçüt:** 2+1'de Mahal aracını açmadan bütün mahaller oluşur; etiketle çizgi çakışması 0.

6. **Canlı alan ve programa göre fark**
   - **Neden:** Bina programı var, ama M0'da çizimle canlı karşılaştırma görülmedi.
   - **Örnek:** KentOS'un kendi Duvarı kaydır önizlemesi, Arcol Targets ("in real time while you model"),
     Snaptrude Program Mode.
   - **Öneri:**
     - Her sürüklemede (duvar, oda köşesi) etkilenen mahallerin alanı HUD'da görünür.
     - Bina programında hedef varsa fark renkli gösterilir (+1,2 m² / −0,8 m²).
     - Alt panelde canlı bir "Mahal listesi": hedef / gerçek / fark.
     - Net, brüt ve emsal alan tanımları mevzuat raporuna bağlanmalı; bu raporda doğrulanmadı.
   - **Ölçüt:** Programlı örnekte hedef dışındaki mahal sayısı panelde görünür; ayrı çizelge komutu gerekmez.

7. **Bağlam mini araç çubuğu / imleç menüsü**
   - **Neden:** 1100 px genişlikte Duvar bağlam sekmesi ekrana sığmıyor; düzenleme komutlarının çoğu taşma menüsünde.
   - **Örnek:** Archicad Pet Palette, BricsCAD Quad, Rayon selection toolbar.
   - **Öneri:**
     - Seçili duvarın yanında 5–6 düğmelik bir çubuk: Böl, Uzat, Buda, Çevir, Stil, Birleşim, Açıklık ekle.
     - Düğüme ya da kenara tıklayınca yalnız o parçaya uyan komutlar çıkar (Pet Palette).
     - O nesne türünde en son kullanılan komut ilk sırada; sağ tık onu tekrarlar (Quad).
     - Tab üst üste binen nesneler arasında geçiş yapar.
   - **Ölçüt:** 1100×650'de taşma 0; Düzenle komutlarının bağlamdan çağrılma oranı izlenir.

8. **Komut arama ve kavram araması (Türkçe eşanlamlılar)**
   - **Neden:** Duvar ailesi ve Mahal ailesinin ikonları birbirine benziyor; kayıtlarda şerit hiç kullanılmıyor.
   - **Örnek:** SketchUp Search (amaçla arama), Rayon "Type anytime", AutoCAD/BricsCAD komut satırı.
   - **Öneri:**
     - Hiçbir komut etkin değilken harf yazmak komut aramasını açar.
     - Sonuçlar takma adlara (DUVAR, WALL… zaten var) ek olarak kavramlarla da eşleşir: "oda", "bölme", "kapı aç",
       "alan", "ölçü koy".
     - Her sonuçta kısayol ve şeritteki yeri görünür ("Mimari › Çiz"); Enter çalıştırır; son kullanılanlar üstte.
   - **Ölçüt:** Yeni kullanıcının aracı bulma süresi (kayıtla ölçülür) yarıya iner.

9. **Akıllı varsayılan, stil önayarı ve katmanlar**
   - **Neden:** Dikdörtgen duvarın varsayılanı Stil: Yok; 54 nesne tek katmanda; Öznitelikler'de genel CAD alanları
     önce geliyor.
   - **Örnek:** Archicad Favorites, Rayon'da etkin stil, Chief varsayılanları.
   - **Öneri:**
     - Mimari şablonuna varsayılan duvar stili (ör. tuğla + sıva) ve kategori katmanları (duvar, açıklık, mahal,
       ölçü, etiket, çizelge) konur.
     - "Favoriler": stil + kalınlık + yükseklik paketleri; seçime uygulanır ya da varsayılan yapılır. Son kullanılan
       stil hatırlanır.
     - Öznitelikler'de mimari alanlar en üstte: Stil, Kalınlık, Yükseklik, Hiza.
   - **Ölçüt:** M0 resimlerindeki dolgusuz, "mimari görünmeyen" duvar kalmaz; en az 6 kategori katmanı.

10. **Tek tıkla ölçülendirme ve ilişkisel ölçü göstergesi**
    - **Neden:** Bugün yalnız alttaki dış zincir var; iç ölçü ve diğer cepheler yok.
    - **Örnek:** Chief Auto Exterior / Auto Room Dimension, Floorplanner anahtarları, RoomSketcher Measurement
      Wizards, Cedreo wall-to-wall, Rayon asma kilidi.
    - **Öneri:**
      - Tek "Ölçülendir" komutu dört cepheye açıklık / duvar / toplam zincirlerini ve istenirse odalara iç ölçüleri
        koyar.
      - Görünüm anahtarları: dış, iç, açıklık, kot.
      - Cedreo gibi: iki tıklamayla çizilen bir çizgi, kestiği bütün duvarların ölçü zincirini üretir.
      - Ölçüler modele bağlı; durum asma kilit ve Bağlı / Kısmen / Bağsız ile görünür. Bağlı ölçüye tıklamak duvarı
        taşır (2. madde).
    - **Ölçüt:** 2+1'de belgeleme için ≤2 ölçü komutu; duvar taşındıktan sonra elle ölçü düzeltmesi 0.

11. **Arama ve sürükle-bırak katalog, akıllı yerleşim**
    - **Neden:** Şeritte mobilya ve vitrifiye kataloğu yok; yalnız Mutfak dizisi var.
    - **Örnek:** Sweet Home 3D mıknatısı, SmartDraw stamp→anchor imleci, Floorplanner süzgeçleri ve Magic Layout.
    - **Öneri:**
      - Kenar panelde aranabilir katalog: kategori ve çok kelime ("lavabo 60").
      - Plana sürüklenen nesne en yakın duvara sırtını verir, köşeye kenetlenir; duvara uzaklığı kılavuzla görünür.
      - Duvara bırakılan kapı ya da pencere Kapı aracının motoruyla duvar kalınlığına uyar.
      - Başka bir nesnenin üstüne bırakılan (ör. tezgâh üstü evye) onun kotunu alır.
      - İmleç, nesnenin tutunacağı yeri gösterir.
      - Sonra: oda tipine göre önerili yerleşim ("Islak hacmi donat", Magic Layout fikri).
    - **Ölçüt:** Banyo donatımı (klozet, lavabo, duş) ≤6 etkileşimde.

12. **Çizim anında hata önleme**
    - **Neden:** "Birleşimleri denetle" ve "Etiketleri denetle" ancak komutla çalışıyor; etiket binmesi ekranda kalıyor.
    - **Örnek:** Archisketch sarı uç, magicplan yeşil/turuncu duvar, RoomSketcher kapanmamış oda, Rayon kilitli ölçü.
    - **Öneri:**
      - Açık duvar uçları ve kapanmamış oda sınırları çizimde renkli işaretlenir (yalnız ekranda, çıktıya girmez).
      - Mahali olmayan kapalı alan soluk taranır ve "çift tıkla: mahal" ipucu verir.
      - Etiket çakışması anında kaydırılır ya da işaretlenir.
      - Durum çubuğunda sayaç ("3 açık uç · 1 çakışan etiket"); tıklamak ilk soruna götürür.
    - **Ölçüt:** Denetim komutlarının bulduğu her şey, komut çalıştırılmadan da ekranda görünür.

13. **İlk beş dakika: şablon, sihirbaz, örnek proje, araç yardımı**
    - **Neden:** "Bu CAD çizimi hangi iş için?" penceresi iyi bir kapı, ama ardından boş tuval geliyor.
    - **Örnek:** Rayon Templates, SmartDraw dashboard, Chief Space Planning Assistant, SketchUp Instructor.
    - **Öneri:**
      - Mimari seçilince kartlar: Boş · 2+1 daire · Müstakil konut · Rölöveden (PDF/resim altlık).
      - Sihirbaz kat sayısını ve oda listesini sorar, oda kutularını üretir (4. maddenin ikinci aşaması).
      - Her araç açılınca köşede 2–3 adımlık "Araç yardımı" ve kısa bir animasyon.
      - Her örnek proje bir `kullan` kabul senaryosuyla aynı olur.
    - **Ölçüt:** Kullanılabilirlik oturumunda yeni kullanıcı ilk mahali alanıyla birlikte ≤2 dakikada görür.

14. **Klavyeyle çalışma: Tab döngüsü, son komutu yineleme, eksen kilidi**
    - **Neden:** Klavye zaten yoğun kullanılıyor (48 tuş, 28 yazma), ama seçim ve tekrar fareyle yapılıyor.
    - **Örnek:** Revit Tab ile zincir seçimi, BricsCAD Tab, Rayon Space/Enter, SketchUp ok tuşları.
    - **Öneri:**
      - Duvarın üstünde Tab bağlı duvar zincirini vurgular, tıklama hepsini seçer (stil değiştirmek ve ölçülendirmek
        için).
      - Üst üste binen nesneler arasında Tab ile geçilir.
      - Komut yokken Enter ya da Space son komutu yeniden başlatır.
      - Ok tuşları ekseni kilitler.
      - CAD alışkanlığı olanlar için "AutoCAD benzeri" kısayol kümesi seçeneği.
    - **Ölçüt:** `change` senaryosunda tık sayısı düşer; tuş sayısındaki artış tıktaki düşüşten küçük kalır.

15. **Tek tıkla pafta ve plan çıktısı**
    - **Neden:** Çizelgeler model alanında plana yapışık, mahal çizelgesi görünümün altına taşıyor; Pafta seti var
      ama ayrı bir iş.
    - **Örnek:** Snaptrude Present, Cedreo'nun tek modelden bütün planları, RoomSketcher'ın tek tıkla 2B/3B'si,
      SmartDraw'un her an değişen ölçeği.
    - **Öneri:** "Planı paftala" komutu tek seferde şunları kurar:
      - 1/100 kat planı;
      - 10. maddedeki otomatik ölçü;
      - çakışmasız etiketler;
      - pafta alanında mahal çizelgesi;
      - antet ve kuzey oku.

      Ölçek paftanın üstünden değiştirilebilir; PDF/DWG tek çıkıştır. Komut yeniden çalıştırılınca paftayı baştan
      üretmez, günceller.
    - **Ölçüt:** Kabul senaryosunda çizimin bitişinden PDF'e ≤3 etkileşim.

**Sonraya kalanlar (15'in dışında):**
- Görüntü ya da PDF'ten yapay zekâyla plan çıkarma (RoomSketcher AI Convert, Rayon).
- Telefonla rölöve (magicplan).
- Model üstünde yorum ve salt okunur paylaşım (SketchUp 2026, Rayon mobil).
- Duvar ağıyla kendiliğinden birleşmeye istisna tanımlama (Rayon).

---

## 4. Doğrulanamayanlar ve sınırlar

- **Revit:** Geçici ölçü, kenetleme ve zincir seçimi sayfalarında görsel URL'si bulamadım. Zincir seçimi, snaps ve
  "=" formülü bilgisi arama özetinden.
- **Sweet Home 3D:**
  - Kullanıcı kılavuzunun görselleri tembel yükleniyor; URL alınamadı.
  - Katalog aramasının geldiği sürüm kaynaklar arasında farklı (7.3 / 7.4).
  - Duvar ipucunun ne gösterdiği kılavuzda yazmıyor.
- **RoomSketcher:** Yardım merkezi (help.roomsketcher.com) 403 verdi; o makalelerdeki ayrıntılar arama özetinden.
- **Planner 5D:** Destek makaleleri 404 verdi. "Smart Wizard" ve otomatik yerleşimin nasıl çalıştığı belgelenmemiş;
  yalnız pazarlama metni var.
- **Floorplanner:** 2019 tarihli otomatik donatım blog yazısı 404 verdi; Magic Layout yardım makalesinden doğrulandı.
  Duvarı sürükleyerek boyutlandırma ve kesin ölçü yazma kendi yardımında okunamadı (yalnız eski bir el kitabı
  kopyası söylüyor).
- **Chief Architect:** "Fix Wall Connections" çaresi arama özetinden; makale URL'si eşleştirilemedi.
- **Arcol ve Qonic:** Arcol için yalnız yardım merkezi koleksiyonları ve bir makale; görsel bağlantıları imzalı.
  Qonic için resmî belge bulamadım, yalnız ticari basın var.
- **Cedreo:** Oda bulma ve "as you go" etiketleme üretici blogundan; resmî yardımda yalnız ölçü türleri doğrulandı.
- **Görseller:** Archicad, Vectorworks ve AutoCAD görsellerinin çoğu göreli yoldan türetildi. Yalnız Archicad
  `TrackerEditing.png`'nin açıldığını denedim.
- **Kazanç sayıları:** Ölçüm değil; akış sayımı ya da üretici iddiası.
- **Mevzuat:** Alan türleri ve etiket içeriği gibi konular kapsam dışı; mevzuat raporuyla bağlanmalı.

---

## 5. Kaynaklar

SketchUp / LayOut
- https://help.sketchup.com/en/sketchup/introducing-drawing-basics-and-concepts
- https://help.sketchup.com/en/release-notes/sketchup-desktop-20260
- https://help.sketchup.com/en/sketchup-desktop-20261
- https://help.sketchup.com/en/sketchup/search-tools-commands-and-extensions
- https://help.sketchup.com/en/sketchup-web/search-tools-and-commands (arama özeti)
- https://help.sketchup.com/en/sketchup/scaling-your-model-or-parts-your-model (arama özeti)
- https://help.sketchup.com/en/using-guides (arama özeti)

Sweet Home 3D
- https://www.sweethome3d.com/userGuide.jsp
- https://www.sweethome3d.com/history.jsp
- https://www.sweethome3d.com/features.jsp

Rayon
- https://docs.rayon.design/llms.txt
- https://docs.rayon.design/documentation/rayons-basics/drawing
- https://docs.rayon.design/documentation/rayons-basics/shortcuts
- https://docs.rayon.design/documentation/rayons-basics/navigation-and-command-access.md
- https://docs.rayon.design/documentation/design/elements-walls-zones-openings-etc.md
- https://docs.rayon.design/documentation/design/associative-dimensions.md
- https://docs.rayon.design/other-resources/product-updates/september-26-product-updates.md

Floorplanner
- https://help.floorplanner.com/en/categories/579650-working-in-the-editor
- https://help.floorplanner.com/en/categories/581122-room
- https://help.floorplanner.com/en/articles/2405826
- https://help.floorplanner.com/en/articles/2394370
- https://help.floorplanner.com/en/articles/2399106

RoomSketcher
- https://www.roomsketcher.com/blog/the-7-measurement-types-in-roomsketcher/
- https://www.roomsketcher.com/features/draw-floor-plans/
- https://help.roomsketcher.com/hc/en-us/articles/360000202509-How-Can-I-Draw-Walls-with-Exact-Lengths (403, arama
  özeti)
- https://help.roomsketcher.com/hc/en-us/articles/203472452-Display-the-Area-of-a-Room-PC-Mac- (arama özeti)

Planner 5D · Cedreo · magicplan · SmartDraw · Archisketch · HomeByMe
- https://planner5d.com/use/floor-plan-drawer
- https://help.cedreo.com/en/how-to-customize-the-measurements-on-the-2d-plan
- https://cedreo.com/blog/how-to-draw-floor-plans/
- https://cedreo.com/faq/how-to-draw-house-plans/
- https://help.magicplan.app/auto-scan-your-floor-plan
- https://help.magicplan.app/scan-a-room-in-seconds-using-lidar (arama özeti)
- https://www.smartdraw.com/floor-plan/how-to-draw-a-floor-plan.htm
- https://docs.archisketch.com/guide/user-en/make-project/draw-floor-plan
- https://home.by.me/en/guide/how-to-draw-floor-plan (arama özeti)
- https://homebyme.zendesk.com/hc/en-us/articles/35277592775581-How-do-I-start-a-project (arama özeti)

Snaptrude · Arcol · Qonic
- https://www.snaptrude.com/blog/snaptrude-3-0
- https://www.snaptrude.com/llms.txt
- https://help.snaptrude.com/en/articles/10902764-import-program-data (arama özeti)
- https://intercom.help/arcol/
- https://intercom.help/arcol/en/collections/11458094-metrics
- https://intercom.help/arcol/en/articles/10447961-targets
- https://aecmag.com/bim/qonic-edges-towards-bim-authoring/ (ikincil, arama özeti)

Chief Architect / Home Designer
- https://www.chiefarchitect.com/support/article/KB-00349/using-the-space-planning-tools.html
- https://www.chiefarchitect.com/support/article/KB-01116/moving-walls-using-dimensions.html

Archicad
- https://help.graphisoft.com/AC/29/INT/_AC29_Help/030_Interaction/030_Interaction-53.htm
- https://help.graphisoft.com/AC/28/INT/_AC28_Help/030_Interaction/030_Interaction-40.htm
- https://help.graphisoft.com/AC/29/INT/_AC29_Help/140_UserInterfaceDialogBoxes/140_UserInterfaceDialogBoxes-8.htm
- https://help.graphisoft.com/AC/27/INT/_AC27_Help/020_Configuration/020_Configuration-25.htm
- https://help.graphisoft.com/AC/28/INT/_AC28_Help/020_Configuration/020_Configuration-56.htm (Favorites; arama özeti)

Revit
- https://help.autodesk.com/cloudhelp/2023/ENU/Revit-DocumentPresent/files/GUID-74FFB00B-FB29-44B7-8FEA-613DBE010AD4.htm
- https://help.autodesk.com/cloudhelp/2022/ENU/RevitLT-GetStarted/files/GUID-CC0FF240-4F0E-4393-AA50-17FA1E4A58DF.htm
- https://help.autodesk.com/view/RVT/2025/ENU/?guid=GUID-64E1FC74-2F79-42FF-8363-A1DF23448D98 (arama özeti)
- https://help.autodesk.com/view/RVT/2025/ENU/?guid=GUID-A3947840-1F30-41D7-AF75-B61687952A07 (arama özeti)
- https://help.autodesk.com/cloudhelp/2015/ENU/Revit-Model/files/GUID-86A50B39-334D-4EE2-B2B8-F28DC6C2D26C.htm
  (arama özeti)
- https://help.autodesk.com/cloudhelp/2026/ENU/Revit-ArchDesign/files/GUID-CB78C07F-5878-4053-8014-D23FAA45F001.htm
  (arama özeti)
- https://help.autodesk.com/cloudhelp/2025/ENU/Revit-ArchDesign/files/GUID-65668F88-4429-4992-AA0D-1C4FBDCFA650.htm
  (arama özeti)

Vectorworks
- https://app-help.vectorworks.net/2025/eng/VW2025_Guide/Basic1/Drawing_with_the_Data_bar.htm
- https://app-help.vectorworks.net/2026/eng/VW2026_Guide/Basic2/SmartCursor_cues.htm
- https://app-help.vectorworks.net/2026/eng/VW2026_Guide/Basic2/Snapping_to_a_smart_edge.htm
- https://app-help.vectorworks.net/2026/eng/assets/docs/VWKeyboardShortcuts.pdf (arama özeti)

BricsCAD
- https://help.bricsys.com/en-us/document/bricscad/getting-started-with-bricscad/exploring-the-interface/the-quad-cursor-menu
- https://help.bricsys.com/en-us/document/bricscad-bim/getting-started/quad-cursor (arama özeti)

AutoCAD
- https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-MEP/files/GUID-07AF5E9F-8A0D-434A-BD06-B80596591319.htm
- https://help.autodesk.com/cloudhelp/2023/ENU/AutoCAD-Core/files/GUID-1ED138FF-2679-45C4-9C2C-332A821C9D12.htm
  (arama özeti)
- https://help.autodesk.com/cloudhelp/2023/ENU/AutoCAD-Core/files/GUID-38EC86CF-D96A-455F-A5DE-2CDA23C28FC4.htm
  (arama özeti)
- https://help.autodesk.com/cloudhelp/2020/ENU/AutoCAD-Core/files/GUID-6242DF04-BD00-4A5D-891C-B131184D0D36.htm
  (arama özeti)
