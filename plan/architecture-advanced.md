# KentOS Mimari — ileri 2B tasarım ve üretim planı

İnceleme: **7 Kasım 2026** (ikinci tur; ilki 30 Ekim, `dc7b16d6c`). Kod tabanı: `e40e6dae0` (main). Dal: `arch-adv-ileri`.

Sahibin hedefi (7 Kasım 2026): bir mimari çizim **çok pratik** yapılabilsin, **aynı zamanda AutoCAD kadar ayrıntılı**
yapılabilsin. Yeni düğme sayısı başarı ölçüsü değildir. Kullanıcı bir odayı büyüttüğünde duvarları, açıklıkları,
donatıyı, mahalleri, ölçüleri ve paftaları elle onarmak zorunda kalmamalı. Çizim ilk duvardan itibaren bir mimari pafta
gibi görünmeli. Türkiye'de ruhsat ve uygulama projesinin istediği çizimler aynı modelden çıkmalı.

Bu tur **araştırma ve plandır**; uygulama kodu değişmedi. ADV-01…07'nin durumu kodla yeniden sınandı (§3). Yeni paketler
(ADV-08…ADV-26, §4) ve iş sırası (M1…M51, §5) eklendi. Uygulama, plan sahiple görüşüldükten sonra başlar. Her madde §6'daki
talimatla yapılır.

## 0. Bu turun yöntemi

- **Kod:** üç okuma denetimi yapıldı:
  - araçlar ve arayüz (A1),
  - çizimdeki öğeler (A2),
  - plan maddelerinin gerçek durumu (A3).
- **Uygulama:** güncel ikiliyle beş kabul senaryosu oynatıldı (`kentos-cad kullan acceptance-flat|house|change|renovation|curved`,
  hepsinde "beklentilerin hepsi tuttu"). Mimari sekmesi ve araç ayarları `kentos-cad snapshot` ile çekildi (M0).
- **İnternet:** dört konu araştırıldı:
  - AutoCAD ve DWG uyumlu alternatifleri (W1),
  - BIM ve mimari yazılımlar (W2),
  - pratik plan uygulamaları ve hız desenleri (W3),
  - Türkiye mevzuatı ve çizim standartları (W4).

  Yalnız üreticilerin ve kurumların resmî sayfaları kullanıldı.
- **Notlar:** raporların tamamı [docs/research/2026-11-07-mimari/](../docs/research/2026-11-07-mimari/) altındadır. Kaynak
  adresleri ve başka uygulamaların ekran görüntüsü adresleri oradadır. Plana giren kritik iddialar ayrıca kodda ya da
  resmî metinde doğrulandı; aşağıda ✓ ile işaretlidir.
- **Resimler:** `.run/shots/kullanim/masaustu-acceptance-*` ve `.run/gui-evidence/mimari-durum/` altındadır; Git'e girmez.

## 1. Bugünkü durum (7 Kasım 2026)

### 1.1 Güçlü taban

- **Nesneler:**
  - duvar (düz, yaylı, en çok 16 bileşen, düğüm kararı);
  - kapı ve pencere (6 işleyiş, tip/örnek, net geçiş); boşluk, niş, baca;
  - mahal (net/eksen/brüt, şaft), döşeme/balkon/teras;
  - aks, kolon, merdiven (düz, L, U, döner), rampa, mutfak dizisi;
  - etiket, görünüm işaretleri, çizelge ve metraj.
- **Bağlantılı işler:** bağlar, oda/daire modülleri, tasarım seçenekleri, bina programı ve bağımsız bölümler, görünüş
  tarifleri, proje gezgini.
- **Komutlar:** katalogda 63 mimari komut var; Python ve MCP katalogdan üretiliyor. Mimari ortak durumlar: 136 dosya,
  4 789 durum.
- **Çekirdek CAD'de olanlar:** kenetleme ve izleme, imleç yanı değer girişi, tutamaçlar, tarama, ölçü/yazı/tablo stilleri, blok
  ve dinamik blok, dış başvuru, pafta ve görünüm penceresi, kalem tabloları, yayın.
- **Değişim:** DWG kısmen var; mimari IFC yok.

### 1.2 Ana sorunlar

Önem: K kritik, Y yüksek, O orta. Kanıt sütununda ✓ bu turda kodda ya da resimde ayrıca doğrulananlar.

| # | Sorun | Kanıt | Önem | Paket |
|---|---|---|---|---|
| S1 | Duvar zinciri ancak Enter'da yazılıyor; Esc çizilen parçaları sessizce atıyor | `apps/desktop/src/input.rs:472-489` ✓ | K | ADV-08 |
| S2 | "pencere" yazınca Pencere yakınlaştır başlıyor: `PENCERE` takma adı yakınlaştırmanın, pencere aracınınki `PENCEREKOY`; ad aramasında takma ad başlıktan önce eşleşiyor | `apps/desktop/src/typed_names.rs:35-44`, `catalog/v1/additions.json` ✓ | K | ADV-08 |
| S3 | 12 mimari pencere modal; karartılmış zemine tık kaydedilmemiş düzenlemeyi sormadan atıyor; seçimle çalışan pencereler açıkken seçim değişmiyor | A1 S3 | K/Y | ADV-09 |
| S4 | Plan mimari pafta gibi görünmüyor: aracın duvar stili varsayılanı "Yok", poşe yok, kesilen/görünen kalem ayrımı yok; ölçeğe göre ayrıntı yalnız duvarda; plan kesme yüksekliği sabit 1,10 m | M0 resimleri ✓, `contracts/src/arch.rs:94-125` ✓, `contracts/src/opening.rs:21` ✓ | Y | ADV-13, 14, 15 |
| S5 | Bütün mimari nesneler etkin katmana yazılıyor: 2+1 örneğinde 54 nesnenin hepsi "Çizim" katmanında. DXF'te mahalin adı ve alanı yazılmıyor | M0 ✓, `formats/src/dxf/writer/mod.rs:752-774` ✓ | Y | ADV-14, 25 |
| S6 | Mimari şerit 46 komutla kabul edildi, bugün 78 komut taşıyor; 42'si ▾ arkasında. Sık işler (Duvarı kaydır, Uzat, Buda, Hepsini etiketle) menüde. Gerçek `kentos-mimari`'de yalnız Duvar, Dikdörtgen duvar, Kapı, Pencere ve Proje ▾ büyük; Mahal ve donatı küçük (M0'ın `kentos-cad` resminde hiç büyük düğme yoktu) **(M15'te düzeldi: 37 → 27 görünen düğme; Duvar ▾, Kapı, Pencere, Mahal ▾, Donatı ▾, Ölçü ▾ büyük; Hepsini etiketle Etiket ▾'de; Duvarı kaydır, Uzat, Buda M17'de Duvar sekmesine)** | A1 §1.1, gerçek pencere ✓ | Y | ADV-09 |
| S7 | 78 mimari komutun hiçbirinde kısayol yok, en kısa takma ad 3 harf, F1 listesinde mimari yok | A1 S4 | Y | ADV-08 |
| S8 | Seçenek harfleri araçtan araca değişiyor; aracın seçeneği olmayan harf genel kısayolu başlatıyor | A1 S5 | Y | ADV-08 |
| S9 | Terim çakışmaları: duvar kalınlığı istemde "Genişlik", panelde "Kalınlık"; "Döşeme" hem döşeme hem kaplama deseni; "Boşluk" hem açıklık hem şaft; "Duvar birleşimi" ile "Duvarları birleştir" ayrı araçlar | A1 S6 | Y | ADV-08 |
| S10 | Duvar çizerken imleç etiketi CAD projesinde de "Semt" yazıyor | `crates/native/interaction/src/path.rs:2447` ✓ | Y | ADV-08 |
| S11 | Seçenekler komut satırına odaklanıp harfle giriliyor (`E` Enter `0.25`): 2+1 izinde 15 odak değişimi. İmleç yanı değer kutusu var ama aracın seçeneklerini taşımıyor | `fixtures/interaction/v1/acceptance-flat.json` ✓ | Y | ADV-11 |
| S12 | Başlangıç zayıf: Mimari, Giriş sekmesiyle açılıyor; yeni projede kat yok; şablon cm ve 1:50, proje başlangıcı m ve 1:100 (ADR 0626); örnek proje yok | `catalog/v1/templates/mimari.json:7-8` ✓, A1 S10 | Y | ADV-13 |
| S13 | Tek adımda "oda" (duvar + mahal + etiket) yok. Odayı sürükleyerek büyütme menüde. Mobilya ve vitrifiye Mimari sekmesinde yok; "lavabo" araması sonuç vermiyor **(M21'de Oda aracı geldi; M23'te oda döşemesinden seçilip kenar tutamacıyla büyüyor)** | A1 §4.1 | Y | ADV-11, 12 |
| S14 | Komut geçmişi ham parametre döküyor; Türkçe-İngilizce karışık ve kimlikli ("adopt: evet · frame: corners · wall 01a1b609-…") | M0 resimleri ✓ | O | ADV-08 |
| S15 | Sağ tık menüsü ve üzerine gelme kartı mimariye özgü değil (kart parsel bilgisi gösteriyor) **(M17'de düzeldi: sağ tıkın ilk bölümü bina sekmesinin kendi paneli; kartta duvarın stili, kalınlığı ve katı, mahalin numarası, adı ve alanı)** | A1 S11 | O | ADV-09 |
| S16 | İkonlar: üç duvar aracı bir ikonu, beş mahal aracı bir ikonu, Etiket ile Mahal numarala bir ikonu paylaşıyor; üç araç genel CAD ikonu ödünç alıyor; ikon turunda Mimari yok **(M16'da düzeldi: on yeni ikon, yedi çizelge yeniden çizildi; üç bekçi; ikon turunda 15 Mimari sahnesi)** | ikon tablosu ✓ (A1 §5) | O | ADV-10 |
| S17 | 1100 px genişlikte sekme satırı taşıyor; etkin bağlam sekmesi (Duvar) ekranda görünmüyor | M0 resmi ✓ | O | ADV-09 |
| S18 | Duvar seçiliyken Öznitelikler'de önce genel CAD alanları geliyor; duvarın kalınlığı, stili ve yüksekliği kaydırmadan görünmüyor | M0 resmi ✓ | O | ADV-09 |
| S19 | Etiketler duvar çizgisine, geçici ölçüler mahal etiketine biniyor; ölçü yalnız alttaki dış zincir | M0 resimleri ✓ | O | ADV-16 |
| S20 | Düşey model yok: çatı, kiriş, lento ve tavan nesnesi yok. Bina kesiti ve cephe türetilmiyor; yalnız oda iç görünüşü var (ADR 0799) | A2 §1, §3 | Y | ADV-18, 19, 20 |
| S21 | Tadilat: açıklığın evresi duvarından okunuyor, mevcut duvardaki yeni kapı gösterilemiyor; araçlarda evre seçilemiyor | `contracts/src/entity.rs:1999-2010` ✓, A1 S16 | Y | ADV-17 |
| S22 | Türk proje takımının büyük parçaları yok: vaziyet planı ve emsal/TAKS, metrekare cetveli, kat irtifakı tablosu, çatı planı, grafik doğrama listesi, Türk anteti ve pafta takımı | A2 §4, W4 §3 | Y | ADV-22, 23, 24 |
| S23 | Kabul açığı: 27 Ekim'den beri gerçek pencerede mimari kabul yok; 35 ADV ADR'sinin 32'si "elle denenmedi" diyor. Kabul izleri araçları kimlikle başlatıyor, kenetleme ve yardımları kapatıyor; `kullan` modül seçemiyor; `snapshot --sekme arch` "böyle bir şerit sekmesi yok" diyor | A3 §2, `apps/desktop/src/snapshot.rs:72-76` ✓ | Y | ADV-26 |
| S24 | 200 çizelgeli 100 dairede kare 163 ms (hedef 100 ms); etiket motoru artımlı yerleşmiyor | ADR 0786, A3 §4 | O | ADV-07 |

### 1.3 Ölçülen başlangıç değerleri

Bu sayılar her fazın sonunda yeniden ölçülür (§7).

| Ölçü | Bugün | Kaynak |
|---|---|---|
| 2+1 daire, gerçek pencerede (sahibin ayarları, kenetleme açık) | **147 etkileşim** (70 tık, 57 tuş, 14 değer / 71 harf, 3 hareket, 3 tekerlek; 11 araç); KLM **177,6 s** | `fixtures/interaction/real/daire-2p1.txt`, ADR 8000 ✓ |
| 2+1 daire kabul izi (kod içi) | 158 etkileşim: 11 araç, 15 odak, 28 yazma, 48 tuş, 38 tık, 17 hareket, 1 sayı tıkı; içinde fareyle yapılamayan adımlar var | `acceptance-flat.json` ✓ |
| İki katlı konut izi | 8 araç, 19 odak, 36 yazma, 52 tuş, 15 tık | `acceptance-house.json` ✓ |
| Mimari şerit | 78 komut; 42'si ▾ arkasında; `kentos-mimari`'de 4 büyük düğme + Proje ▾ | A1 §1.1, gerçek pencere ✓ |
| Kısayolu olan mimari komut | 0 / 78 | A1 S4 |
| Aynı ikonu paylaşan komut | duvar 3, mahal 5, etiket 2 | ikon tablosu ✓ |
| 2+1 örneğinde kullanılan katman | 1 (54 nesne) | M0 ✓ |
| Ölçeğe göre ayrıntısı olan nesne türü | 1 (duvar) | `arch.rs:94-125` ✓ |
| 100 daire + 200 çizelgede kare | 163 ms | ADR 0786 |

### 1.4 Planın kendi durumu

- `architecture.md`: 26 maddenin 18'i `[x]`, 8'i `[~]` (BASE-01, PERF-01, DRAW-03, OPEN-01, OPEN-02, SPACE-01, SPACE-02,
  DOC-04).
- **ADV-01…07:** hepsi `[~]`.
- **`[x]` olup kabulü bugünle uyuşmayanlar:**
  - UX-02: kabul 46 komutlu şeride aitti.
  - UX-01: 17 aracın panelindeki 56 satır komut satırında yok (ADR 0776 B9).
  - UX-03: şablon hâlâ cm ve 1:50.
  - DRAW-02: kalıcı bağ hâlâ bölme aracına bağlanmadı; `walls_partition.rs:229-235` sıradan duvar yazıyor.
  - DIM-01: ADR 0695'in durum satırı eski.
- **Bu turda düzeltilen belge tutarsızlıkları:** ADV-04'ün "Açık: bölüm çizelgesi" notu ADR 0795 ile kapanmıştı.
  `architecture.md`'deki eski KCAD numaraları (104–107 yerine 109–112) aynı commit'te düzeltildi.

### 1.5 Gerçek pencerede görülenler (M1, [ADR 8000](../docs/adr/8000-real-window-acceptance.md))

Sahibin uyarısıyla (7 Kasım: "mouse ile kullanmak ile kod testleri arasında fark oluşuyor") 2+1 daire gerçek
pencerede, gerçek fare ve klavye olaylarıyla, sahibin ayarlarının kopyasıyla çizildi (`scripts/usage/real/`). Kod
içi izin görmediği farklar:

| # | Gerçek kullanımda | Paket |
|---|---|---|
| G1 | Araç çalışırken komut satırını istem ve seçenek çipleri dolduruyor; yazı alanı görünmüyor. "Komut satırına tıkla" çipe düşüp menüsünü açıyor. Seçenek harfi çizim alanında küçük harfle yazılınca çalışıyor ("e" → "duvar kalınlığını yazın"), ama bunu ekranda bir şey söylemiyor | ADV-08, ADV-11 |
| G2 | Büyük harf (Shift+E) aracın seçeneği sayılmıyor: Dikdörtgen duvarı bırakıp Uzat'ı başlatıyor. Küçük "e" doğru çalışıyor (7 Kasım'da gerçek pencerede ikisi de denendi; ilk denemede sürücünün yazdığı büyük "E" yanlışlıkla "E bozuk" diye kaydedilmişti) | ADV-08 (M4) |
| G3 | Tablo yerleşince şerit kendiliğinden Tablo sekmesine geçiyor; sonraki Çizelge ▾ tıkı başka düğmeye gidiyor, ikinci çizelge sessizce oluşmuyor | ADV-09 |
| G4 | Duvar ölçüleri "Ölçü katmanı: Çizim", Hepsini etiketle "Etiket katmanı: Çizim (sabit)"; yeni projenin Ölçü ve Yazı katmanları kullanılmıyor **(M9'da düzeldi: MM-XXXX-OLCU ve MM-XXXX-ETIKET)** | ADV-14 (M9) |
| G5 | Dikdörtgen duvarın varsayılanı 0,20 m, Stil: Yok **(M8'de düzeldi: KentOS Mimari'de Dış duvar 25, 0,25 m)** | ADV-13 (M8) |
| G6 | Kenetleme ve ızgara açıkken fareyle konan pencereler 1–2 cm kayıyor (dış zincir 1.900 · 2.952 · 2.095 · 0.953; kod içi izde 1.910 · 2.940 · 2.100 · 0.950); kullanıcı bunu yerleştirirken görmüyor | ADV-11 (ölçüyle yerleştirme) |
| G7 | Görünen alanda çizelgeye yer yok; ikinci çizelgenin noktası birincinin üstüne düşünce tık tabloyu seçti, ileti yok | ADV-24 |
| G8 | Gerçek Mimari şeridi kod içi `kentos-cad` resimlerinden farklı (büyük düğmeler, katlanmış Proje ▾) | ADV-26 |
| G9 | Yeni proje Giriş sekmesinde açılıyor (S12 doğrulandı) **(M8'de düzeldi: yeni proje Mimari sekmesinde açılıyor)** | ADV-09, ADV-13 (M8) |
| G10 | Geçici ölçü yazısı mahal etiketinin, kapı ve pencere etiketleri duvar çizgisinin üstüne biniyor (S19 doğrulandı) | ADV-16 |
| G11 | F1 (Fare ve klavye kısayolları) penceresi Esc'yle kapanmıyor; Kapat düğmesi kapatıyor **(M7'de düzeldi: yazı alanının yuttuğu Esc pencereyi kapatıyor)** | ADV-09 (M7) |
| G12 | Durum çubuğu uyarıyı kesiyor: "Duvar sürüyor: “L”…" 1440 px'te okunmuyor, tam metin yalnız Komut geçmişi'nde | ADV-09 |
| G13 | Şerit etiket uzunluğuna duyarlı. Bir etiket 3 harf kısalınca Mimari şeridi 1440 px'te Proje ▾'yi açıp büyük düğmeleri küçültüyor; 4 harf uzayınca kentos-cad'in Mimari sekmesinde bir panel ikona iniyor (M4'te görüldü) **(M15'te düzeldi: 1440 px'te 13 px yazıyla bütün paneller açık ve sağda yaklaşık 120 px boş; yalnız simge basamağı 7 Kasım'da kalktı)** | ADV-09 (M15) |

## 2. Piyasa araştırması

### 2.1 30 Ekim turu

İncelenenler üreticilerin resmî teknik/kullanım belgeleridir; ticari yazılımlar
bu bilgisayarda çalıştırılmış sayılmaz. Kaynakların sürümleri bilerek yazılmıştır;
bir belgede anlatılan yetenek bütün sürümlere veya ücretsiz sürüme genellenmez.
Üç boyutlu BIM davranışları, KentOS için doğrudan uygulanabilir 2B kod değildir.

| Yazılım / incelenen yöntem | KentOS için sonuç |
| --- | --- |
| [Archicad 30 — Distance Guides](https://help.graphisoft.com/AC/30/INT/Topics/030_Interaction/030_Interaction-65.html): çevreye ölçü, referans kenarını değiştirme, boy yönünde bağlantıyı koruyarak uzama/kısalma | Mevcut geçici ölçüyü tekrar eklemek yerine oda değişikliğinin bağlantılı etkisini göster. Bu özelliğin kendisinden kalıcı kısıt garantisi çıkarma. |
| [Revit 2026 — ilişkiler](https://help.autodesk.com/cloudhelp/2026/ENU/RevitLT-GetStarted/files/GUID-04AAFF35-A495-48EC-AA5B-35E956A4855C.htm) ve [Chief Architect — ölçüyle duvar taşıma](https://www.chiefarchitect.com/support/article/KB-01116/moving-walls-using-dimensions.html) | Serbest taslağı sonradan kesinleştirme ile kilitli tasarım ilişkisini ayır; kullanıcı hangi elemanın hareket edeceğini bilsin. |
| [AutoCAD Architecture 2027 — bileşen öncelikleri](https://help.autodesk.com/cloudhelp/2027/CHT/AutoCAD-Architecture/files/GUID-A5F21978-FC03-4CFA-95F1-C027177A7A25.htm), [Vectorworks 2027 — bileşen birleşimleri](https://app-help.vectorworks.net/2027/eng/VW2027_Guide/Walls_edit/Joining_wall_components.htm), [Archicad 30 — duvar referansı](https://help.graphisoft.com/AC/30/INT/Topics/040_ElementsVB/040_ElementsVB-7.html) | Birleşim doğruluğu, kaplama sürekliliği ve kalınlık değişiminde sabit referans, yalnız tarama güzelliğinden daha önemlidir. |
| [Vectorworks 2027 — Door settings](https://app-help.vectorworks.net/2027/eng/VW2027_Guide/DoorWindow/Door_settings.htm), [Graphisoft — nominal boylar](https://gdl.graphisoft.com/tips-and-tricks/nominal-sizes-automatic-dimensioning/) | Kanat, kasa/ürün, kaba boşluk ve net geçiş farklı kavramlardır. “90 cm kapı” ifadesinin referansı görünür olmalı. |
| [Vectorworks 2027 — object styles](https://app-help.vectorworks.net/2027/eng/VW2027_Guide/Symbols/Concept_Plug-in_object_styles.htm) ve [wall closures](https://app-help.vectorworks.net/2027/eng/VW2027_Guide/Walls/Wall_closure_settings.htm) | Tipten, örnekten ve katalogdan gelen parametreleri ayır; açıklık çevresindeki katman dönüşünü host bağlamında tanımla. |
| [Vectorworks 2027 — Space Planning](https://app-help.vectorworks.net/2027/eng/VW2027_Guide/Spaces/Space_Planning.htm) ve [Space settings](https://app-help.vectorworks.net/2027/eng/VW2027_Guide/Spaces/Space_settings.htm) | Gerçek alanın yanında hedef program ve komşuluk göster; net/brüt hesabın yöntemi açık olsun. |
| [Revit 2026 — Groups of Elements](https://help.autodesk.com/cloudhelp/2026/ENU/Revit-Model/files/GUID-52612B0F-43AA-47AF-A76C-BB0E3DD24E34.htm) | Tekrarlanan banyo/oda/daireyi host ve anotasyon ilişkileriyle yönet. Normal mobilya bloğu ile aynı ürün kavramı sayma. |
| [Archicad 30 — View Settings](https://help.graphisoft.com/AC/30/INT/Topics/140_UserInterfaceDialogBoxes/140_UserInterfaceDialogBoxes-68.html), [Model View Options](https://help.graphisoft.com/AC/30/INT/Topics/050_ViewsVB/050_ViewsVB-90.html), [Renovation](https://help.graphisoft.com/AC/30/INT/Topics/050_ViewsVB/050_ViewsVB-101.html) | Aynı modelin uygulama, donatı ve tadilat görünüşünü tekrar kurulabilir tariflerle sakla. |
| [BricsCAD BIM QuickDraw V27](https://help.bricsys.com/en-us/document/command-reference/b/bimquickdraw-command?version=V27) | Bitişik oda çizerken komşu duvarı benimseme, yeniden çizmekten daha değerlidir. Bu kaynak arama dizininden doğrulandı; doğrudan sayfa açılışı başarısızdı, uygulama öncesi yeniden açılmalı. |
| [QCAD — property editor](https://qcad.org/doc/qcad/latest/reference/en/tutorials/properties/property_editor/property_editor_en.html), [LibreCAD — fundamentals](https://docs.librecad.org/en/latest/ref/fundamentals.html) | Tür/kapsam süzgeçli toplu düzeltme ve kesin CAD girdisi korunmalı. BIM yaklaşımı hassas 2B çizim alışkanlığının yerine geçmemeli. |

KentOS kararları aşağıdadır; üreticilerin algoritması, veri modeli veya kapalı
kaynak kodu biliniyormuş gibi bir öneri yapılmamaktadır.

### 2.2 7 Kasım turu: ayrıntıyı ve pratikliği ne getiriyor?

Kimlikler (R3, A12, B4 …) araştırma notlarındaki kaynak listelerine aittir. Ürün adları, sürümleri ve görüntü adresleri
orada; ticari ürünler bu bilgisayarda çalıştırılmadı.

**AutoCAD düzeyinde ayrıntı veren mekanizmalar** (W1, W2):

| Mekanizma | Kim nasıl yapıyor | KentOS karşılığı |
|---|---|---|
| Görünüm yönüne ve ölçeğe göre gösterim | AutoCAD Architecture: display configuration → set → representation [W1 R3, R4]. Revit: Detail Level [W2 R12]. Archicad: Model View Options ve Partial Structure Display [W2 A5, A6] | Yalnız duvarda üç düzey var → ADV-15 |
| Bileşen önceliği ve temizleme grupları | AutoCAD Architecture: düşük numara üstün, temizleme grupları [W1 R5, R6]. Archicad ve Vectorworks bileşen birleşimleri [W2 A7, V9] | Var (ADR 0771); kolon–duvar yok → ADV-18 |
| Mahal sınırları ve alan şemaları | AutoCAD Architecture: base/net/usable/gross [W1 R8]. Revit area schemes [W2 R17]. Allplan DIN 277 [W2 L5] | Net/eksen/brüt var; resmî alan profili yok → ADV-22 |
| Tadilat | AutoCAD Architecture renovation mode [W1 R7]. Revit phase filters [W2 R9, R10]. Archicad renovation filters [W2 A11]. Vectorworks phasing [W2 V7, V8] | Kısmen; açıklığın kendi evresi yok → ADV-17 |
| Katman anahtarı ve standart | AutoCAD Architecture layer key styles. AutoCAD 2026.1'de bulut, tablo ve görünüm penceresi kendi katmanına [W1 A6]. CAD Standards [W1 A19]. Archicad layer combinations [W2 A10] | Yok → ADV-14 |
| Otomatik ölçü | Archicad automatic exterior dimensioning [W2 A12]. Vectorworks exterior walls [W2 V11]. QDIM Advanced ve Smart Dimension [W1 B1, Z1] | Ölçü dizileri var; dört cephe tek komut yok → ADV-16 |
| Kesit ve görünüş üretimi | BricsCAD BIM generating drawings, eski paftayı kırmızı/turuncu işaretler [W1 B9]. AutoCAD Architecture | Yalnız oda iç görünüşü → ADV-19 |
| Çift yönlü çizelge | Archicad interactive schedule [W2 A15]. Vectorworks worksheets [W2 V12] | Kaynak hücreden düzenleme var; genişletilir |
| Revizyon ve yayın | Archicad revision management [W2 A17, A18]. Vectorworks title block manager ve revisions [W2 V13, V14] | Yayın var; pafta başına revizyon açık → ADV-24 |

**Pratikliği getiren desenler** (W3 §2'nin dokuz mekanizması):

- **Aynı nesne, iki giriş yolu:** fareyle kaba çizim ve imleç yanında kesin sayı aynı araçta. Örnekler: SketchUp
  Measurements, Archicad Tracker, Vectorworks Data bar, AutoCAD Dynamic Input. Hızlı çizilen duvar sonradan
  ayrıntılandırılan duvarın kendisidir.
- **Önce yaklaşık çiz, sonra ölçüyle düzelt:** geçici ölçüye tıkla ve değer yaz. Örnekler: Revit [W2 R6], Chief Architect
  KB-01116, Rayon, SmartDraw. Tek tıkla kalıcı ölçü ya da bağ olur.
- **Oda önce:**
  - BricsCAD BIM QuickDraw: oda sürükle → duvar + döşeme; komşu oda; L oda; kat ekle [W1 B4, B5].
  - Floorplanner ve Archisketch.
  - Chief Architect'in mekân planlama kutuları.
- **Duvardan oda:**
  - Sweet Home 3D: çift tık.
  - Rayon: tek tık.
  - Revit: oda ve etiketi birlikte koyar [W2 R16].
  - Vectorworks: duvarlardan mahal [W2 V6].
- **Görünür kenetleme:**
  - Vectorworks SmartCursor ipuçları;
  - SketchUp çıkarım renkleri ve ok tuşuyla eksen kilidi;
  - Revit'te nokta türüne göre şekil.
- **Eylem nesnenin yanında:** BricsCAD Quad imleci, o türde son kullanılan komutlar [W1 B6]. Archicad Pet Palette.
- **Katalog:**
  - Sweet Home 3D mobilyayı duvara yaslar, kendiliğinden döndürür.
  - AutoCAD Smart Blocks Placement önceki yerleşimden konum önerir [W1 A11].
- **Akıllı varsayılan, bilinçli kaçış:** mıknatıs açık, Alt ile kapalı; kademeli açılan ayar; kişiye göre kısayol kümesi;
  modelden kendiliğinden pafta, ölçü ve çizelge.

**Türk pazarı — İdeCAD Mimari** (W2 §4):

- TAKS ve KAKS imar durumundan girilir. Emsal harici %30 denetimi varsayılandır. Hesap eskizde canlı gösterilir; sınırı aşan
  denetim işaretlenir.
- Arsa poligonu XY koordinat tablosundan yapıştırılır; çekme mesafeleri görsel tanımlanır.
- Mahal kapalı alana tıklanarak oluşur. Mahal ayarlarında sıva payı, kolon/merdiven/duvar düşümü ve kaplama yazıları var.
- Mahal listesi kendiliğinden üretilir.
- Ölçülendirme: iç ölçü ve dış ölçü ayrı.
- 20 kalem kalınlığı tanımlanır.
- Mimari plan ile kalıp planı aynı dosyada, iki kip arasında geçilir.
- "Ruhsat paftaları" hazırdır.

### 2.3 Mevzuat ve standartlar (W4)

Madde numaraları ve sayılar resmî metinden okundu; ayrıntı ve bağlantılar W4 raporundadır. Okuma aracının
kısalttığı bentler raporda ◐/⚠ ile işaretlidir ve kural tablosuna girmeden önce aynen okunur.

| Kaynak | KentOS için önemli olan |
|---|---|
| **Planlı Alanlar İmar Yönetmeliği** (PAİY; RG 03.07.2017/30113, son değişiklik 01.07.2026/33297) | **Md. 4:** net, brüt, genel brüt ve toplam brüt alan; sıva payı 2,5 cm; merdiven altı 1,80 m.<br>**Emsal ve emsal harici:** 5(8) %30; 20(8), 21, 22.<br>**Ölçüler:** bahçe mesafeleri (23); kat ve iç yükseklikleri (28); piyes asgarileri (29); giriş ve rampa (30); merdiven (31: rıht 0,16/0,18, 2a+b = 60–64); ışıklık ve hava bacası (32); asansör (34); korkuluk 1,10 m ve kapılar (38–42).<br>**57(2)(a):** ruhsat eki mimari projenin listesi. **57(10):** ilk pafta bilgi tablosu. **57(27):** ölçek. **57(29):** mahal listesi. |
| **Mimarlık ve Mühendislik Projelerinin Dijital Olarak Hazırlanması Hakkında Yönetmelik** (RG 05.08.2026/33331, yürürlük 01.09.2027; geçici Md. 1 ile 2029–2033 arası kademeli) ✓ | **Md. 4:** çizimler PDF/A, model IFC olarak teslim edilir.<br>**Md. 7(1):** katman ve sembol adları, ölçü, yazı ve çizgi tipleri **EK-4**'e göre. **Md. 7(2):** mahal numarası kat + bölüm/fonksiyon + sıra no, **EK-3**'e göre.<br>**Md. 9(1):** CAD çizimleri BIM modelinden üretilir; model ile PDF/A çizimleri uyumlu olur.<br>EK-1…EK-9 (proje kodu, dosya adları, IFC sınıfları ve öznitelikleri, kalite formları) ayrı PDF'te; henüz okunamadı. |
| **e-PYS Yönetmeliği** (RG 05.08.2026/33331, yürürlük 01.09.2028) | Ruhsat süreci e-PYS'de yürür; yüklemeyi proje müellifi yapar; dosyalar yukarıdaki yönetmeliğe uygun olur. |
| **Binaların Yangından Korunması Hakkında Yönetmelik** | Yüksek bina eşikleri 21,50 / 30,50 m. Kaçış mesafesi duvardan 40 cm içeriden ölçülür. Ek-5/A kullanıcı yükleri, Ek-5/B ve Ek-14 kaçış uzaklıkları, çıkış genişlikleri; konutta yüksekliğe göre merdiven sayısı (48(5)). |
| **Otopark Yönetmeliği** | Park yeri 2,40 × 4,90 m; engelli 3,50 m; rampa %15/%20, genişlik ve iç yarıçap 2,75 m; ihtiyaç bağımsız bölüm alanından. |
| **Sığınak Yönetmeliği** (07.11.2025 değişikliği) | Oda sayısına göre 2/3/4 kişi; kişi başı 1 m²; en az 20 m²; 60 cm beton; 100 m²'yi aşınca iki çıkış. |
| **Kat Mülkiyeti Kanunu** Md. 2–4, 12 | Bağımsız bölüm, eklenti, ortak yer, arsa payı; kat irtifakı paylaşım tablosu. |
| **Bina Enerji Performansı Yönetmeliği ve TS 825** (Ekim 2024; 01.04.2025'ten beri zorunlu) | Katmanlı yapı bileşeninin U değeri; ısı yalıtım raporu ruhsat eki. |
| **Sivas Belediyesi İmar Yönetmeliği** (RG 10.11.2022/32009) | PAİY'den ayrılan yerel değerler: merdiven 1,30 m; asansör 3 kattan itibaren; ticari zemin kat 5,00 m; kar siperliği; kırmızı kiremit. Hava bacasında PAİY ile çelişki: 0,30 m / 0,36 m² ile 0,60 × 0,60 m. Sürümlü ve katmanlı kural motoru gerekir. |
| **Mimarlar Odası çizim ve sunuş esasları** (İstanbul Büyükkent Şb., 2015) | **Mahal etiketi:** elips içinde no, ad, net m².<br>**Kodlar:** doğrama kodları K/P/GK/CK; aks X harf, Y rakam; ±0,00 ve röper kot sembolü.<br>**Ölçü:** dış ölçü zinciri 3 (ön proje), 4 (uygulama); iç ölçü mahal başına 2+2.<br>**Pafta:** antet sağ altta; pafta kodları; A4 katlama. |
| **TSE standartları** (ücretli; kapsamla) | TS EN ISO 128 (çizgiler), TS EN ISO 7200 (antet), TS EN ISO 13567 (CAD katmanları), TS EN ISO 4157 (yapı bölümlerinin gösterimi), TS ISO 9836 (alan tanımları), TS 9111 (erişilebilirlik). "TS 3807" diye bir tarama standardı bulunamadı. |

Mevzuat denetimi resmî onay değildir. Kural profili kaynak, madde, Resmî Gazete tarihi ve yürürlük aralığıyla sürümlenir.
Hangi profilin uygulanacağına kullanıcı karar verir (§8).

## 3. Mevcut paketlerin durumu (ADV-01…07)

Bu paketlerin 30 Ekim–1 Kasım metni korunmuştur. Her birinin başına **7 Kasım** notu eklendi: kodla sınanmış bugünkü
durum ve kalan işin hangi yeni pakete ya da sıraya (§5) geçtiği.

### ADV-01 — Bağlantılı odaları net ölçüyle kur ve güvenle değiştir [~]

**7 Kasım:** bağlar, önizleme ve çelişki kartı kodda var. Kalanlar: ölçülü bölme oluştururken kalıcı bağ yazmıyor (`walls_partition.rs:229-235`; ARCH-DRAW-02'nin `[ ]`'i) ve bağların yay kenar, kolon, aks ve döşeme kapsamı (ADR 0770 sınırları) → M49; odayı sürükleyerek büyütme → M23 (yapıldı, ADR 8023); gerçek pencerede kabul → ADV-26.

_Durum (31 Ekim 2026, [ADR 0770](../docs/adr/0770-linked-rooms-and-architectural-bonds.md), [ADR 0780](../docs/adr/0780-bonds-in-the-way.md), [ADR 0790](../docs/adr/0790-bonds-while-dragging-and-together.md)):_ ortak duvarı benimseyen
Dikdörtgen duvar, kaydırmanın etkisi (önizleme kartı), Bu ölçüyü koru (kilit karesi), duvar çizgisi ve pervaz bağları
(mesafe, paralel, eşit aralık, sabit) ve şema 137 yapıldı. Çelişen bağ reddin `bonds`'unda (parçalar, değerler,
`frees`) ve Duvarın mesafesi'nde yerinde vurgulu; Bağı göster/kaldır Python ve MCP'de; 20 ms bütçesi ölçüldü (200/50: 2,4 ms,
400/100: 6,9 ms). Tutamaçla
sürüklemede, Duvarı kaydır'da, Boy'da ve açıklığın taşınmasında yoldaki bağlar yerinde işaretli; hiçbir bağ tek başına
geçirmiyorsa birlikte kaldırılınca geçen en küçük küme (`freesWith`) önerilir ve kartın düğmesiyle tek adımda kaldırılır.
Öznitelikler'den yazılan değerin kartı ve geç çelişkide kümelerin denklemlerle süzülmesi (400 duvarda 366 → 22 ms; [ADR 0797](../docs/adr/0797-properties-bond-card.md)). Açık: gerçek pencerede elle kabul.

**Kullanıcı sonucu:** “Bu odanın iç eni 3,20 m olsun; koridor 1,20 m kalsın”
işi, çok sayıda taşı/uzat/buda işlemine dönüşmeden yapılabilsin. Önce basit ve
açıklanabilir davranış teslim edilsin. Dayanak A1–A2; eski `ARCH-DRAW-03` ve
`ARCH-EDIT-01` ile birlikte yürütülür.

- Bitişik oda ekleme, ortak duvarı benimseme, bir oda yüzünü veya ortak bölmeyi
  net ölçüden değiştirme eylemleri mevcut araçların üzerinde çalışsın. Ortak
  sınırda ikinci duvar oluşmasın; mevcut açıklık ve mahal kimliği korunabilsin.
- “Şimdi bu ölçüye getir” ile “bu ölçüyü koru” ayrı ve anlaşılır olsun. Kalıcı
  bağ kullanıcı seçimiyle oluşsun; ölçülen değer ile tasarımı süren değer aynı
  simgeyle gösterilmesin. İlk kalıcı kapsam düz duvar, paralellik, net mesafe,
  açıklık uca mesafesi ve eşit aralıkla sınırlansın; yaylı kapsam ayrıca alınsın.
- Mevcut kısıt çözücüsüne mimari yüz/eksen/çekirdek adaptörü tasarla. Host,
  kısıt ve follower aynı geometri üzerinde ayrı otoriteler kurmasın. Sabit taraf,
  serbest taraf ve kopacak/korunacak bağlar komutun açık girdileri olsun.
- Önizleme yalnız yeni çizgileri göstermesin: iki yandaki net boy/alanı,
  etkilenen kapıları, donatıyı, ölçüyü ve kapsam dışındaki kilitli katmanı göster.
  Çelişkide ilgili bağlar yerinde işaretlensin; hangi bağı kaldırabileceği
  açıklansın. Otomatik olarak tolerans büyütülmesin veya bağı sessiz koparılmasın.
- Etkin araç ayarları, komut satırı ve geçici ölçü birbiriyle aynı taslağı
  kullansın. Tek Enter/sağ tuş Onayla bir işlem; Esc sıfır yazım; tek Undo bütün
  ilişkili sonucu geri alsın. “Bağı göster/kaldır” da Python/MCP'de erişilebilir olsun.

**İlk kabul:** 4 × 3 m odaya komşu 3 × 3 m oda ekle; ortak bölmeyi 40 cm kaydır.
Sonra 1,20 m koridoru koruyan bağı etkinleştirip dış duvarı değiştir. Alanlar,
kapılar, etiketler ve ölçüler izlesin. Çelişkili iki bağ atomik reddedilsin;
Undo/Redo ve kaydet–yeniden aç aynı modeli versin. İlk teslim bu örnekle
sınırlıdır; genel otomatik ev tasarımcısı değildir.

### ADV-02 — Yapı bileşenlerini ve bağlı sınırları uygulama kalitesine getir [~]

**7 Kasım:** Kalanlar: varsayılan duvar stilleri ve poşe → M8; kolon–duvar birleşimi ve kolon malzemesi → M41; IFC → M47; kitaplık kaynağının izlenmesi; gerçek pencerede kabul.

_Durum (31 Ekim 2026, [ADR 0771](../docs/adr/0771-building-parts-and-bound-boundaries.md)):_ düğüm kararı, sabit referans
ve etkisi, uçta ve sövede dönen katmanlar, kaynağa bağlı döşeme ve kopukluk yapıldı.
_ADR 0785 (ADV-02b, 31 Ekim 2026):_ malzeme kimliği (proje malzemeleri, bileşen ve döşeme bağı, Metinlerden al ile kayıpsız
göç, lejant ve Metraj kimlikle) ve kasanın yüzüne kadar dönen katmanlar yapıldı.
_ADR 0791 (ADV-02c, 31 Ekim 2026):_ ofis malzeme kitaplığı (KentOS temel kitaplığı, `.kmalzeme` klasörleri, koru/değiştir ile
alma, kitaplığa yazma), Duvar stilleri'nde malzeme seçicisi (bağlı bileşenin görünüşü malzemenin, `cad.styles.edit`
olmayan malzemeyi reddeder) ve döşemenin Öznitelikler'den mahale bağlanması yapıldı. Açık: IFC, kitaplık kaynağının
izlenmesi, gerçek pencerede elle kabul.

**Kullanıcı sonucu:** Duvar tipi veya dış kontur değiştiğinde çekirdek,
kaplamalar, açıklık çevresi ve bağlı döşeme birbirinden kopmasın. Dayanak A4–A5.

- Mevcut birleşim çözümünü koruyarak düğümde düzenleme tasarla: hangi bileşen
  hangisine bağlanır, hangisi durur/döner, uç nerede kapanır? Otomatik karar ve
  kullanıcı düzeltmesi ayırt edilsin; geçersiz kılma düğüm/alt nesne kimliğiyle
  saklansın, çizgilerle yapılmış görsel tamir olmasın.
- Tip/kalınlık değişiminde sabit kalacak bitmiş yüz, çekirdek yüzü veya eksen
  seçilsin. Oda net alanına etkisi ve açıklığın kasa derinliği önizlensin.
  Ters duvar yönü, eşit öncelik, kolon teması, dar açı ve eğrisel birleşim için
  destek sınırı açıklansın; desteklenmeyen birleşim bozuk geometri üretmesin.
- Açıklık çevresindeki sıva/yalıtım dönüşü, söve ve duvar uç kapaması ortak
  bileşen anlamını kullansın. İlk teslim birkaç yaygın çok katmanlı duvar ve
  kapı/pencereyle sınırlansın. Görsel tarama ile fiziksel bileşen hesabı ayrı sınansın.
- Döşeme/kaplama için **kaynağa bağlı sınır** ve **bağımsız kontur** açık seçenek
  olsun. Bağlı sınır kaynağı, yüz kuralı ve ötelemesi saklansın; duvar/mahal
  değişince yeniden üretilebilsin. Kaynak kaybında son geometri korunsun,
  kopukluk görünür olsun; yeniden bağla/ayır eylemi bulunsun.
- Malzeme kimliği, görünen ad, bileşen rolü ve hesap yöntemi ayrıştırılsın.
  Mevcut metin malzemeler kayıpsız taşınsın; yeni ortak tabloya geçiş, KCAD,
  metraj ve dışa aktarma sözleşmesiyle birlikte sürümlensin.

**Kabul:** Yalıtımlı dış duvarı 50 cm taşı; bağlı döşeme büyüsün, bağımsız balkon
değişmesin, mevcut serbest kenar korkuluğu izlesin. T birleşimi ve pencere
sövesini 1:20/1:50/1:100 ekran ve PDF'de incele. Tip değişimi ve tek Undo,
elle verilen düğüm kararını ve kaynak bağlarını geri getirsin.

### ADV-03 — Gerçek kapı/pencere tipleri, örnek kontrolü ve net kullanım [~]

**8 Kasım:** Doğrama kodları, 90/210, çizelgede Kod sütunu ve Mahale göre numara yapıldı (M28, [ADR 8028](../docs/adr/8028-joinery-codes-and-tag-places.md)).

**7 Kasım:** Kalanlar: kapı ve pencerenin ölçeğe göre temsili → M11; "90/210", doğrama kodları ve odaya göre numara → M28; grafik doğrama listesi → M38; Aynı tiptekileri seç'in bağımsız bölüm kapsamı; dört kanatta eşit olmayan kanat; gerçek pencerede kabul.

_Durum (31 Ekim 2026, [ADR 0772](../docs/adr/0772-door-window-types-instances-clear-width.md)):_ tipler, örnek istisnası,
dört en, yapı, takip raporu ve Kapılar denetimi yapıldı. Döner, cebe giren ve katlanır kapı, kayar kapının rayı ve bindirmesi
([ADR 0777](../docs/adr/0777-revolving-pocket-folding-doors.md)). Döner/katlanır/raylı sürme kapının çarpışma denetimi düzeltmesiyle,
yaylı duvarda cep ve cebin metrajdan düşülmesi ([ADR 0787](../docs/adr/0787-special-door-clashes-arc-pockets-pocket-quantities.md)).
Üç ve dört kanatlı sürme ve paftanın PDF metninin okunması ([ADR 0792](../docs/adr/0792-multi-leaf-sliding-doors-and-pdf-text.md)). XOX ve eşit olmayan iki kanatlı sürme ([ADR 0807](../docs/adr/0807-xox-and-unequal-sliding-leaves.md)). Açık: bağımsız bölüm kapsamı (ADV-04), dört kanatta eşit olmayan kanat, gerçek pencerede elle kabul.

**Kullanıcı sonucu:** Ofisin K01 kapısı tek yerde tanımlansın; kullanıcı hangi
özelliğin tipe, hangisinin bu kapıya ait olduğunu bilsin. Dayanak A3.

- Kullanıcı/ofis tip kitaplığı; tip kimliği/sürümü, örneğe özel değer,
  tipten ayırma, bu örneğin değerini tipe uygulama ve etkilenenleri önizleme
  birlikte tasarlansın. Aynı adlı farklı kaynak tipler karışmasın. Duvar
  stilleri ve dinamik bloklardaki mevcut kurallar tekrar kullanılabilirlik için incelensin.
- Kanat eni, kasa/ürün dış eni, kaba boşluk ve hesaplanan net geçiş farklı
  alanlardır. Mevcut `width` anlamı sessizce değiştirilmesin. Eski çizimler
  için uyum, yeni veri için sürümlü anlam ve çizelgedeki sütun adları birlikte kararlaştırılsın.
- Önce yaygın tek/çift kanat kapı, sürme kapı ve bölmeli pencere derinleştirilsin:
  eşit olmayan kanatlar, kasa derinliği, yerleşim, eşik, pervaz/söve, denizlik,
  doğrama bölmeleri ve duvar dönüşü. Daha az kullanılan pivot/cep/katlanır
  türleri aynı sözleşmeye sonradan eklenebilsin; sahte sembolle destek ilan edilmesin.
- “Aynı tiptekileri seç/değiştir” kat, bağımsız bölüm ve seçim kapsamı sunsun.
  Tip güncellemesi kilitli katman, örnek istisnası veya geçersiz host ile
  karşılaştığında uygulanacak/atlanacak/reddedilecek nesneleri açıkça raporlasın.
- Kapı kanadı ve donatı kullanım zarfı görünür/isteğe bağlı olsun. Fiziksel
  çarpışma, net geçiş ve kullanıcı proje kuralı ayrı bulgu türleri olsun.
  Mevcut mutfak çakışma uyarıları ve mimari denetim genişletilsin; ikinci
  bağımsız denetim sistemi kurulmasın.

**Kabul:** K01 tipindeki 20 kapının kasasını değiştir; tipten gelen 19 örnek
izlesin, örnek istisnası korunup gösterilsin. 90 cm kanat ve 90 cm kaba boşluk
aynı kabul edilmesin. Sahne, net geçiş sorgusu, etiket, çizelge, Python/MCP ve
PDF aynı bilgiyi versin. Kapı önü çakışması çizimde açıklanabilsin.

### ADV-04 — Mahalleri bina programı ve bağımsız bölüm bağlamında yönet [~]

**7 Kasım:** bölüm çizelgesi ADR 0795 ile yapılmıştı; aşağıdaki durum notu düzeltildi. Kalanlar: Türk alan motoru, emsal/TAKS ve kat irtifakı (resmî alan profili ve arsa payı) → M29–M31; mevzuat denetimi → M33–M35; gerçek pencerede kabul.

_Durum (31 Ekim 2026, [ADR 0773](../docs/adr/0773-building-program-units-and-design-brief.md),
[ADR 0784](../docs/adr/0784-building-wing-window-and-common-area-shares.md)):_ bölüm/ortak/dış alan üyeliği, tasarım
hedefi ve gerçek/hedef/fark, net geçiş ve kapı/komşu/donatı denetimleri istisnalarıyla, böl/birleştir kararı ve
durdurulabilir denetim; bina/kanat penceresi ve katları, ortak alanların bağımsız bölümlere tam payı (`kentos.share/1`:
eşit, net alana, ağırlığa göre; kanat/bina/program kapsamı, en büyük kalan, paylar havuza tam eşit) yapıldı. Bölüm çizelgesi de
yapıldı ([ADR 0795](../docs/adr/0795-unit-schedule.md)). Açık: resmî alan profili ve arsa payı, gerçek pencerede elle kabul.

**Kullanıcı sonucu:** Tasarımcı yalnız oluşan alanı okumakla kalmasın;
hangi odanın hangi daireye ait olduğunu ve tasarım hedefini karşılayıp
karşılamadığını görebilsin. Dayanak A6 ve mevcut mahal/denetim altyapısı.

- Bina, kanat, kat, bağımsız bölüm ve mahal kimliklerini/üyeliklerini tasarla.
  Ortak alan, dış alan ve iki katlı daire destek sınırları açık olsun. Katman
  adını bu hiyerarşinin veri modeli olarak kullanma; kat ile bağımsız bölüm
  ilişkisini zorunlu bire bir ağaç haline getirme.
- Mahal kullanımına göre hedef alan aralığı, net en, gerekli komşuluk ve erişim
  beklentisi tanımlanabilsin. Gerçek/hedef/fark çizimde ve tabloda görülsün.
  Bunlar ilk aşamada tasarım brief'idir; yasal sınırla karıştırılmasın.
- Açık mutfak, sanal ayırıcı, şaft, kolon, niş, bitmiş yüz ve çekirdek ölçüm
  yöntemi mevcut mahal motorundan gelsin. Net/brüt/kullanılabilir alanın adı
  ve hesap yöntemi gösterilsin; kayıtlı/resmî alanın üstüne yazılmasın.
- Bölme/birleştirmede hangi mahal kimliğinin devam edeceği, yeni numara ve
  bitiş/donatı verilerinin nereye taşınacağı kullanıcıya açıklansın. Yalnız
  seed yakınlığına dayanarak iki odanın iş verisi sessizce yer değiştirmesin.
- Koridor darboğazı, kapı bağlantısı ve donatı kullanım alanı denetimleri
  değişen çevreye göre güncellensin. Bulgu ölçülen yeri/değeri ve kural
  sürümünü göstersin; mevcut istisna mekanizması anlamlı geometri değişiminde
  yeniden değerlendirsin. Pahalı tarama durdurulabilir işlem olsun.

**Kabul:** İki bağımsız bölümlü 2+1 kat planında ortak koridor, açık mutfak,
şaft ve balkon oluştur. Bölme kayınca hedef alan ve net geçiş farkı güncellensin;
mahal birleşiminde kimlik/numara kararı açık olsun. Daire toplamları ortak
alanları iki kez saymasın; GUI/Python/MCP aynı hesap ve bulguları üretsin.

### ADV-05 — İlişkili oda/daire modülleri ve kontrollü tekrar kullanımı [~]

**7 Kasım:** Kalanlar: modüle kolon, merdiven, rampa, döşeme, mahal ayırıcı ve mutfak dizisi girmiyor, eğri duvar reddediliyor (`arch_modules.rs:224-235`) → M50; seçenekler arası eşleme ve birleştirme; gerçek pencerede kabul.

_Durum (31 Ekim 2026, [ADR 0775](../docs/adr/0775-room-modules-and-controlled-reuse.md), [ADR 0779](../docs/adr/0779-room-modules-members-drift-and-flats.md), [ADR 0788](../docs/adr/0788-design-options.md), [ADR 0796](../docs/adr/0796-multi-storey-modules-and-loose-level-repeat.md), [ADR 0808](../docs/adr/0808-design-option-combinations.md), [ADR 0809](../docs/adr/0809-comparing-option-combinations.md), [ADR 0810](../docs/adr/0810-loose-repeat-skips-what-is-there.md), [ADR 0811](../docs/adr/0811-single-storey-module-instances.md)):_
tek kat, düz duvarlı oda ve bütün daire modülü; tanım ve örnek ayrı, yerleştirme, planlı kaynak güncelleme, yerel
istisna, bağımsızlaştırma, kaynağı silme; üye ekleme ve çıkarma; elle değişen örnek nesnesi; katları tekrarlama, kattaki modülsüz nesnelerle (bağları kopyalarına çevrilir, hedefte aynısı olan yeniden kopyalanmaz); çok katlı modül (dubleks;
katlar kotlarına göre eşlenir, sığmayan kat reddedilir) ve onun tek katının yerleşmesi (en üst kata da; tuttuğu kattaki üyeleri izler).
Tasarım seçenekleri: küme ve birincil seçenek, bağımsız kopya, birincili değiştirme, iki seçeneğin karşılaştırması
(sayı, alan, plan farkı), kabul (öbürleri silinir) ve silme; görünüş tarifiyle başka seçeneği gösteren pafta;
seçenekler birbirini görmez (birleşim, çakışma, mahal sınırı); adlı bileşimler (kümelerin birer seçeneği; kaydet,
uygula, sil, tarifte göster) ve iki bileşimin küme küme karşılaştırması. Dört banyo, iki katta dört daire ve mutfak A/B ve dubleks
kabulü geçti. Açık: seçenekler arası eşleme/birleştirme, gerçek pencerede elle kabul.

**Kullanıcı sonucu:** Bir banyoyu veya daireyi dört kez çizmek yerine bağlı
örnekleri kullan; kaynak değişimini inceleyerek uygula. Dayanak A7.

- İlk kapsam tek katlı, düz duvarlı banyo/oda: duvar, açıklık, mahal, donatı ve
  ilişkili ölçüler birlikte kopyalanabilsin. Kaynak tanım ile her örneğin
  kalıcı kimlik eşlemesi ayrı olsun. Normal blok kitaplığı bozulmasın;
  bütün mimari varlıkları blok içine gömmek çözüm sayılmasın.
- Yerleştirmede taban noktası, yön/ayna, hedef kat, evre ve numara kuralları
  görünür olsun. Host'lar örnek içi kimliğe bağlansın; harici host/sınır
  bağımlılıkları açıkça yeniden eşlensin veya kapsam dışı diye reddedilsin.
- Kaynak güncellemesi, yerel istisna ve bağımsızlaştırma ayrı eylemler olsun.
  Etki farkı, çakışmalar ve güncellenecek örnekler önizlensin. Kaynak veya
  nesne silinince son geçerli örnek ve kırık bağ bilgisi korunabilsin.
- Anotasyonun model ilişkisi ile görünüşteki konumu ayrı taşınsın. Aynalanmış
  kapı, mahal numarası, etiket, ölçü sahipliği ve yerel yazı taşıması sınansın.
- Tekrar kat, bütün daire modülü ve tasarım alternatifleri sonraki dilimlerdir.
  Evre/mevcut-yıkım-yeni alanı alternatif kimliği olarak kullanılmasın.
  Önce bağımsız alternatif kopyası ve karşılaştırma yeterliyse, karmaşık
  otomatik alternatif birleştirme sistemi baştan zorunlu kılınmasın.

**Kabul:** Aynı banyoyu iki kata, biri aynalı dört örnek olarak yerleştir.
Kaynak enini değiştir; üç bağlı örnek güncellensin, ayrılmış örnek değişmesin.
Host/mahal/ölçü kimlikleri karışmasın; tek Undo, KCAD yeniden açma ve dışa
aktarmadaki kayıp raporu doğrulansın.

### ADV-06 — Mimari görünüşler, proje gezgini ve koordineli yayın [~]

**7 Kasım:** Kalanlar: bina kesiti ve görünüşü ile kapı/pencere açılımı ADV-19'a geçti → M42–M43; yayın/revizyon birlikteliği ve Türk pafta takımı ADV-24'e → M37; gerçek pencerede kabul.

_Durum (31 Ekim 2026, [ADR 0774](../docs/adr/0774-architectural-view-recipes.md)):_ adlı görünüm üstünde sürümlü görünüş tarifi
(kat, alt kat, evre, katman durumu, gizlenen türler, ölçek), bağlı paftanın tarifle çizmesi ve pafta setinin tarife göre etkisi
yapıldı. Proje gezgini ([ADR 0778](../docs/adr/0778-project-navigator.md)): `cad.navigator.read` ve Gezgin paneli; katlar,
görünümler, paftalar, çizelgeler ve seçili nesnenin göründüğü yerler. Tarifin duvar ayrıntısı ve görünümün alanına kırpma
([ADR 0781](../docs/adr/0781-view-recipe-wall-detail-and-crop.md)). Görünüş şablonu ve kalem seti, siyah beyaz
([ADR 0783](../docs/adr/0783-view-templates-and-pen-sets.md)). Kırpma çerçevesinin ekranda düzenlenmesi ve 1:20 PDF incelemesi
([ADR 0789](../docs/adr/0789-crop-frame-on-screen.md)). Alanı haritadan büyük görünümün ön denetimi ([ADR 0794](../docs/adr/0794-map-view-cut-preflight.md)). Oda iç görünüşü
türetilmiş çizim olarak ([ADR 0799](../docs/adr/0799-derived-room-interior-elevations.md)): modelin bildiği duvar, yükseklik ve
açıklıklarla, eskiliği ve yeniden üretimi; bilinmeyenler listelenir. Açık: kanat çizimi, döşeme/tavan kesiti, sınırlı 2,5B
kesit, yayın/revizyon birlikteliği (paftanın yayın kaydı ister), gerçek pencerede elle kabul.

**Kullanıcı sonucu:** Uygulama planı, mobilya planı ve tadilat planı aynı
modelden tekrar kurulabilsin; pafta için her defasında görünürlük ayarlamak
gerekmesin. Dayanak A8–A9; `ARCH-DOC-04` ile ortak teslim.

- Kamera kaydının üzerinde sürümlü mimari görünüş tarifi: kat/kapsam, evre ve
  durum filtresi, katman durumu, çizim yönü/kırpma, çıktı ölçeği, kalem seti,
  duvar ayrıntısı, ölçü/etiket temsili. Model özelliği, görünüş tercihi ve
  kişisel arayüz tercihi birbirine karıştırılmasın.
- Bina/kat/görünüş/çizelge/pafta arasında kaynak bağlantısını gösteren gezgin
  tasarla. Aynı nesnenin farklı görünüşteki temsiline gidilebilsin. Başlangıç
  için birkaç yararlı hazır görünüş yeterlidir; yeni onlarca ribbon ikonu açılmasın.
- Mevcut kat paftası üretimi, evre ve altlık özellikleri kullanılsın. Referansın
  hangi kat/evreye ait olduğu, güncelliği ve kenetlenme durumu açık olsun.
  Görünüşe özel gizleme diğer görünüşün veya modelin verisini silmesin.
- Kesit/görünüş işareti ile türetilmiş çizim ayrımı arayüzde de açık olsun.
  İlk gerçek üretim, mevcut kot/yükseklik ve host bilgilerinin yeterli olduğu
  kapı-pencere açılımı/oda iç görünüşü veya sınırlı 2,5B kesit olabilir.
  Desteklenmeyen elemanlar gösterilip raporlansın; yalnız 2B plandan bütün
  çatıyı/taşıyıcıyı bilen sahte otomatik kesit üretilmesin. Genel 3B model
  kaynağı [3d.md](3d.md) ile ayrıca tasarlanır.
- Kaynağa bağlı çizelge, pafta yayın seti, görünüş sürümü ve revizyon kaydı
  birlikte sınansın. Hesaplanan hücre salt okunur olsun; çizelgeden düzeltme
  mevcut ürün komutuna gitsin. Kullanıcının taşıdığı ölçü/etiket konumu korunsun.

**Kabul:** Aynı katın 1:50 uygulama ve 1:100 donatı görünüşünü oluştur,
kapatıp yeniden aç. Tek kapı değişikliği iki pafta ve çizelgede izlensin.
1:20 detayın taraması, yazı/ölçü boyu ve net geçişi PDF'de incelensin.
Kesit işareti bulunması, gerçek kesit üretimi kabulü yerine geçmesin.

### ADV-07 — Konut çizimini baştan sona ölçülen bir kalite teslimine dönüştür [~]

**7 Kasım:** beş kabul senaryosu güncel ikiliyle oynatıldı, bütün beklentiler tuttu; ama izler araçları kimlikle başlatıyor ve yardımları kapatıyor, şerit ve kenetleme ölçülmüyor (S23). Ölçüm ADV-26 (M1) ile her fazın kapısı olur. Kalanlar: etiket motorunun artımlı yerleşimi ve 163 ms → M51; araçlarda evre → M39; gerçek ofis DWG'si → M48.

_Durum (31 Ekim 2026, [ADR 0776](../docs/adr/0776-measured-quality-acceptance-of-an-apartment.md)):_ 2+1 daire gerçek
girdiyle iz ve bölüm ölçüsü, Python/MCP/masaüstü anlamsal eşitliği, büyük koordinat ve ölçek regresyonları, 1/20/100 daire
ölçümü, dört küçük düzeltme. B6 ve binanın çizelgeleriyle B7 kapandı ([ADR 0782](../docs/adr/0782-design-change-reads-only-what-it-reaches.md));
yazı katmanının artımlı kurulması (B13) de kapandı ([ADR 0786](../docs/adr/0786-text-layer-object-by-object.md): kare 351 → 163 ms, bellek +3 062 → +191 MB); etiket motorunun artımlı yerleşimi açık. Model alanında kalem kâğıttaki genişliğinde tutulur ve kopyalanan etiket kopyayı adlar ([ADR 0793](../docs/adr/0793-model-pen-widths-and-copied-links.md); B8 kapandı). Kabul korpusunun öbür dört örneği ([ADR 0798](../docs/adr/0798-acceptance-corpus-samples.md): ölçülü değişiklik, iki katlı konut, eğrisel/şaftlı, DXF altlığında tadilat; evreye göre mahal sınırı, döşemenin iç noktayla kesilmesi, kilitli altlıktan duvar, açıklığın boşluğuna tıklama düzeltildi). Açık: etiket motorunun artımlı yerleşimi, araçlarda Evre, gerçek
pencerede elle kabul, gerçek ofis DWG'si.

**Kullanıcı sonucu:** Özellikler tek tek gösterilirken iyi görünüp gerçek
bir daire çiziminde birbirini bozmamalı. Bu paket diğer altı paketin ortak
kabul çalışmasıdır; bütün geliştirmeleri bekleyen son temizlik işi değildir.

- Güncel derlemede bir acemi ve bir deneyimli kullanıcı akışını kaydet:
  dış kontur → bölmeler → kapılar/pencereler → mahal → donatı → ölçü → tasarım
  değişikliği → çizelge → pafta. Araç bulma süresi, tıklama/odak değişimi,
  tekrar yazılan değer, elle onarılan ilişki ve hatadan kurtulma sayısı ölçülsün.
- Başlangıçta yalnız bir kaç anlamlı eylem önerilsin; uzman sayısal girdi,
  kısayol ve komut satırını kullanabilsin. Etkin araç taslağı ile seçili nesne
  özellikleri ayrımı, neden pasif olduğu, host/referans ve kat/evre her zaman
  anlaşılır olsun. Mevcut beş panelli Mimari şerit yeniden kalabalıklaştırılmasın.
- Zoom/pan doğruluğu, büyük koordinatta küçük ayrıntı, dolgu/bileşen
  sürekliliği, yazı ve ölçü görünürlüğü regresyon senaryosuna alınsın. Model
  uzunluğu, proje birimi, çizim/pafta ölçeği ve ekran yakınlaştırması ayrı
  kalsın. Mevcut metre/1:100 başlangıcı ve ADR0626 bu planla değiştirilmez.
- Güncel referans donanım/derleme/sahneyle başlangıç ölçümü yap; sonra 1/20/100
  dairelik örneklerde p50/p95 kare süresi, araç önizlemesi, seçme, komut,
  yeniden hesap ve bellek ölç. Başlangıç hedefi etkileşimde 60 Hz bütçesi ve
  girdiye 100 ms içinde geri bildirimdir; ölçülmüş sonuç değildir. Büyük
  işlerde ilerleme/iptal sun; doğru geometriyi gizleyerek hız kazanma.
- Aynı anlamsal işi GUI, Python, MCP ve eklentiden yap. Önizleme/sonuç,
  birim/ret kodu, tek Undo, kalıcı kimlik ve yeniden açılmış model eşitliği
  kontrol edilsin. [Eklenti planındaki](plugins.md) tamlık kapısı her dilime uygulanır.

**Kabul korpusu:** Boş projeden 2+1 daire; ölçülü değişiklik; iki katlı konut;
eğrisel/şaftlı zor örnek; gerçek DWG/DXF altlığı üzerinde tadilat. Her biri
güncel binary/commit, gerçek fare/klavye, önce/sonra görüntü, bağımsız model
kontrolü ve çıktı ile kanıtlansın. Tek otomatik sahne görüntüsü GUI kabulü
değildir; eski ADR test sayıları yeni test çalıştırılmış gibi kullanılmaz.

## 4. Yeni paketler (ADV-08…ADV-26)

Her paket bir kullanıcı sonucu, kapsam, kaynak ve ilk kabul taşır. Kaynak kimlikleri araştırma notlarına aittir. Her dilim
başlamadan konunun güncel belgeleri yeniden okunur (§6). Paketler yeni olduğu için hepsi `[ ]`; işaret yalnız kanıtlanan
alt maddeyle ilerler.

### 4.1 Arayüz sağlığı ve hız

#### ADV-08 — Kritik kullanım hataları: Esc, adlar, terimler ve harfler [~]

_Durum (7 Kasım 2026):_
- **Esc yapıldı** (M2, [ADR 8001](../docs/adr/8001-escape-keeps-the-wall-chain.md)). Duvar zincirinde ilk Esc çizilen
  parçaları yazıyor ve araçta kalıyor, ikinci Esc çıkıyor. Gerçek pencerede önce/sonra resimleriyle doğrulandı.
- **Adlar yapıldı** (M3, [ADR 8002](../docs/adr/8002-mimari-typed-names.md)):
  - Mimari'ye özgü adlar: PENCERE → Pencere; DU, DD, OB, KP, PN, MH, DS, KL, AK, MR, RM, MD, ET; ODA, LAVABO …
  - Komut listesinde, Komut ara'da ve F1'de görünüyorlar.
  - AutoCAD profiline Architecture adları (WALLADD …) eklendi.
- **Terimler, harfler ve Açı yapıldı** (M4, [ADR 8003](../docs/adr/8003-option-letters-and-terms.md)):
  - İstem, araç paneli ve Öznitelikler aynı adı kullanıyor. Duvarın Kalınlık'ı ile çizginin Çizgi kalınlığı ayrıldı,
    "Duvarları birleştir" artık "Tek duvara çevir".
  - Sözleşmenin harfleri (E Kalınlık/Genişlik/En, G Geri, K Kapat, Y Yükseklik, S Stil, T Tür, L Kat, Ö Öteleme,
    D Açı) katalogdan üretilen testle tutuluyor.
  - Shift'li harf seçenektir; seçenek olmayan harf çalışan komutu bitirmez, uyarır.
  - İmleç CAD projesinde Açı yazıyor.
  - Sahibin isteğiyle kelimesi kelimesine çeviriler düz Türkçeye çevrildi (Benimse → Var olanı kullan …).
  - Plandan ayrılan iki nokta:
    - mahalin boşluğu "Şaft" değil "Döşeme boşluğu" oldu, çünkü galeri ve asansör boşluğunu da kapsıyor;
    - mahal sekmesindeki döşeme deseni "Döşeme kaplaması" değil "Döşeme deseni" oldu, çünkü kaplama mahalin
      özniteliği, araç onun desenini çiziyor.
- **Komut geçmişi yapıldı** (M5, [ADR 8004](../docs/adr/8004-readable-command-history.md)):
  - 18 Mimari komutu satırda okunur Türkçe özetle yazılıyor ("Dikdörtgen duvar · 3.000 × 3.000 m · iç net · ortak
    duvar: var olan kullanılır").
  - Komut kimliği, alanlar ve Python "Ayrıntı"da; alanlar da Türkçe ve kimliksiz.
- **Kalanlar:**
  - en sık araçlara tek tuş (sahibin kararı);
  - eş anlamlılar ("Bütün katlar"/"Tüm katlar");
  - "Temizlik", "düğüm" gibi sözcüklerin gözden geçirilmesi;
  - Harita ve kentos-cad araçlarının harfleri.

**Kullanıcı sonucu:** Çizilen iş Esc'de kaybolmaz. "pencere" yazınca pencere aracı açılır. Aynı şeyin her yerde tek adı ve
tek harfi vardır. Komut geçmişi okunur Türkçe yazar. Dayanak S1, S2, S7–S10, S14.

- **Esc:** çok parçalı duvar ve çoklu çizgiyle çalışan mimari araçlar (ölçülü bölme, döşeme, mahal ayırıcı) Esc'de
  tamamlanmış parçaları yazar ya da tek adımda "Geri getir" sunar, bunu da söyler. AutoCAD'in LINE ve PLINE'ı ile Revit
  duvar zincirinin Esc davranışı dilim başında resmî belgeden okunur.
- **Takma adlar ve kısayollar:**
  - Mimari'de "pencere" pencere aracını açar.
  - İki harfli Türkçe takma adlar (ör. DU, KA, PE, MA, OD) ve AutoCAD Architecture adları (WALLADD, DOORADD, WINDOWADD,
    SPACEADD …) birlikte tanınır.
  - En sık beş araca kısayol verilir; F1 mimari araçları listeler.
  - Takma ad çakışma testi yazılır.
- **Terim sözlüğü:** istem, araç ayarı ve Öznitelikler aynı adı kullanır. Ayrılan çiftler:
  - duvarın "Kalınlık"ı ile çizginin "Çizgi kalınlığı";
  - "Döşeme" ile "Döşeme kaplaması";
  - "Boşluk" ile "Şaft";
  - "Duvar birleşimi" ile "Duvarları birleştir".
- **Seçenek harfi sözleşmesi:** aynı anlam aynı harfi alır. Çalışan aracın seçeneği olmayan harf aracı bırakmaz, uyarır.
- **Açı:** duvar çizerken imleç "Semt" değil "Açı" yazar (doğudan saat yönünün tersine).
- **Komut geçmişi:** ham anahtar ve kimlik yerine Türkçe özet yazar ("Dikdörtgen duvar · 11 × 8 m · iç net · 25 cm ·
  Zemin kat"); ayrıntı açılınca görünür.

**İlk kabul:**
- 6 köşeli duvar zinciri + Esc: duvarlar çizimde kalır (ya da tek Ctrl+Z ile geri alınır) ve bildirim çıkar.
- "pencere", "oda" ve "lavabo" ilk sırada doğru komutu bulur.
- Terim ve harf tutarlılık testi katalogdan üretilir.
- Python ve MCP aynı sonucu verir.

#### ADV-09 — Mimari şerit, bağlam ve düzenleyici pencereler [~]

_Durum (8 Kasım 2026):_ Sekme satırı ve Öznitelikler yapıldı (M6, [ADR 8005](../docs/adr/8005-narrow-tabs-and-building-first.md)).
- Dar pencerede sığmayan sekmeler ⋯ menüsüne giriyor. Açık sekme, bağlam sekmesi ve programın kendi sekmesi görünür kalıyor.
- Yapı elemanının kendi bölümü Öznitelikler'de en üstte, ana satırları başında.
- Düzenleyiciler (M7, [ADR 8006](../docs/adr/8006-editors-ask-and-pick-from-drawing.md)):
  - Bina programı, Duvar stilleri, tipler ve Malzemeler değişiklik varken sorarak kapanıyor.
  - Oda modülleri ve Tasarım seçenekleri'nde Çizimden seç var.
  - Esc F1'i kapatıyor (G11).
- Şerit (M15, [ADR 8015](../docs/adr/8015-architecture-ribbon-rearranged.md)): 37 → 27 görünen düğme. Paneller Duvar ·
  Yerleştir · Mahal · Donatı · Düzenle · Belgele · Proje.
  - Büyükler Duvar ▾, Kapı, Pencere, Mahal ▾, Donatı ▾ (Mutfak, Kitaplık), Ölçü ▾.
  - Proje'de Kat ▾ (Üst kat, Alt kat, kat görünümü) ve Evre ▾; "Katlar" düğmesinin adı "Mimari ayarları".
- Bağlam (M17, [ADR 8017](../docs/adr/8017-building-context-tabs-right-click-card.md)): Duvar sekmesinde Duvarı kaydır, Stil
  ve Bağ; Mahal sekmesi Mahal · Kaplama · Belgele; sağ tıkın ilk bölümü bina sekmesinin kendi paneli; kartta mimari alanlar.
- Kalanlar:
  - pencerelerin yüzer ya da dok paneli olması;
  - kolon, merdiven ve rampanın kendi araçları; seçimin yanında eylem şeridi;
  - alt panel.

**Kullanıcı sonucu:** Ana araçlar ilk bakışta bulunur. Sık iş tek tık, seyrek iş bir menü uzağındadır. Düzenleyici
pencereler iş kaybettirmez. Dayanak S3, S6, S15, S17, S18.

- **Şerit:**
  - Duvar, Oda, Kapı, Pencere, Mahal ve Donatı büyük düğmedir.
  - Aileler bölünmüş düğmededir (Duvar ▾: duvar, dikdörtgen, ölçülü bölme, çizgiden).
  - Görünen düğme ≤ 30, menü başına ≤ 6 öğe.
  - Katalogdaki `lead`/`keep` kullanılır; panel adları korunur (DESIGN.md §7.3.1).
- **Açılış:** Mimari modülü Mimari sekmesiyle açılır; şeritte etkin kat ve evre seçicisi bulunur.
- **Bağlamsal sekmeler:**
  - Duvar sekmesinde Duvarı kaydır, bağlar ve stil/malzeme bulunur.
  - Mahal sekmesinin düğme adları düzeltilir.
  - Sağ tık menüsünün ilk bölümü bağlamın araçlarıdır.
  - Üzerine gelme kartı mimari alanları gösterir.
- **Pencereler:** Bina programı, Duvar stilleri, Tipler, Malzemeler, Modüller, Seçenekler ve Metraj yüzer ya da dok
  paneline geçer. Kaydedilmemiş değişiklik sorulur; "Çizimden seç" bulunur. Düzeltme ortak `kentos_ui` bileşenindedir.
- **Öznitelikler:** mimari nesnede önce kendi alanları gelir (kalınlık, stil, yükseklik, referans, kat, evre), genel CAD
  alanları sonra.
- **Sekme satırı:** 1100 px'te taşmaz; etkin bağlam sekmesi her zaman görünür (ortak bileşende).
- **Alt panel:** Mimari'de Mahal listesi ve Denetim bulguları bulunur; ilgisiz CBS sekmeleri gizlenir.

**Kaynak:** DESIGN.md §7.3.1, §7.9; BricsCAD Quad [W1 B6]; Archicad Pet Palette (W3 E).

**İlk kabul:**
- 1440×900 ve 1100×650, 13 ve 18 px, iki temada resimler.
- Şerit sayım testi.
- Pencere açıkken seçim değiştirilip modüle eklenebilir; zemine tık veri kaybettirmez.

#### ADV-10 — Mimari ikon seti [~]

_Durum (8 Kasım 2026):_ Paylaşılan ve ödünç ikonlar ayrıldı (M16, [ADR 8016](../docs/adr/8016-architecture-icon-set.md)).
- On yeni ikon; yedi çizelge solda kendi resmi, sağda liste; Düzenle ve Belgele panelleri ilk komutlarının ikonuyla katlanıyor.
- Bekçiler `arch_icons.rs`'te; ikon tablosu 16, 20 ve 28 px'te iki temada; ikon turunda 15 Mimari sahnesi.
- Kalanlar: birbirine benzeyen öbür aileler ve yeni araçların ikonları (Oda, Çatı, Kesit üret).

**Kullanıcı sonucu:** Her komutun kendi, anlaşılır ikonu vardır; aileler aynı görsel dili konuşur. Dayanak S16.

- **Paylaşılan ikonlar ayrılır:**
  - Duvar / Dikdörtgen duvar / Ölçülü bölme;
  - Mahal / Toplu mahal bul / Mahal ayırıcı / İç nokta seç / Mahal kaynakları;
  - Etiket / Mahal numarala;
  - birbirine benzeyen yedi çizelge ikonu.
- **Ödünç ikonlar:** Duvar ölçüleri, Açıklık dağıt ve Duvarı kaydır'ın ödünç aldığı genel CAD ikonları mimari ikonla
  değişir.
- **Yeni araçlar:** Oda, Donatı, Kot, Çatı, Kesit üret vb. aynı ızgarada çizilir (14, 16, 20 px, iki tema; DESIGN.md §6).
- **İkon turu ve gönderme:** `icon_tour.rs`'e Mimari sahnesi eklenir. Çizilen her ikon sahibine resim olarak gönderilir
  (sahibin 7 Kasım isteği).

**İlk kabul:**
- Mimari sekmesinde aynı ikonu taşıyan iki komut yoktur (test).
- İkon tablosu 16 ve 20 px'te, açık ve koyu temada resimlenir.

#### ADV-11 — Hızlı çizim: imleç yanı seçenekler, tıklanabilir ölçü, kenet ve Oda aracı [~]

_Durum (8 Kasım 2026):_ İmleç yanı seçenekler yapıldı (M18, [ADR 8018](../docs/adr/8018-tool-settings-beside-the-cursor.md)).
- Değer kutusunda Tab Uzunluk → Açı → aracın ayarları; Duvar'da Kalınlık ve Hiza, Dikdörtgen duvarda Boyutlar, Kalınlık ve Boy referansı.
- ↓ çizimin üstünde kutuyu ilk ayarda açar; yazma panelin yolundan (`configure`).
- Geçici ölçüden kalıcı ölçü (M19, [ADR 8019](../docs/adr/8019-temporary-distance-made-permanent.md)): sayının altındaki + mesafeyi iki yüze bağlı ölçü yapar; duvar kayınca değer güncellenir.
- Kenet dili (M20, [ADR 8020](../docs/adr/8020-snap-names-in-a-building-s-words.md)): kenet işareti duvarın çizgisini (Duvar yüzü, Çekirdek yüzü, Katman sınırı, Duvar ekseni, Taban çizgisi) ve açıklığın yerini (Açıklık ortası, kenarı) söyler.
- Oda aracı (M21, [ADR 8021](../docs/adr/8021-room-tool.md)): dikdörtgen duvarlar, mahal ve etiketi tek adımda; mahal adı listeden ya da yazılarak; Duvar panelinde büyük düğme.
- Kapanınca mahal ve canlı döngü alanı (M22, [ADR 8022](../docs/adr/8022-room-on-closing-and-live-loop-area.md)): Duvar'da Kapat mahali de yazar (M, isteğe bağlı); çizerken etikette döngünün alanı.
- Odayı kenarından büyütme (M23, [ADR 8023](../docs/adr/8023-room-edge-grips.md)): oda döşemesine tıklanınca seçilir; her duvar kenarında çift oklu tutamaç duvarı kaydırır, kartta iç ölçü ve alan; yazılan sayı yeni iç ölçü (+/− ile ne kadar).
- Kalanlar: çokgen oda, odaya çift tık, program hedefiyle fark; 2+1 izinin yeniden yazımı; açıklık ve kolonun geçici ölçüsünden kalıcı ölçü; mimari kenet süzgeci.

**Kullanıcı sonucu:** "4,20 × 3,60 bir yatak odası, yanına 1,20'lik koridor" fareyle ve birkaç sayıyla çizilir; komut
satırına odaklanmak gerekmez. Dayanak S11, S13.

- **İmleç yanı seçenekler:** imleç yanındaki değer kutusu aracın seçeneklerini de taşır: uzunluk ve açı, kalınlık, boy
  referansı. Alanlar arasında Tab ile geçilir; kutuya tıklamadan yazılır. Örnekler (W3 B): SketchUp Measurements, Archicad
  Tracker, Vectorworks Data bar, AutoCAD Dynamic Input.
- **Tıklanabilir geçici ölçü:** ölçüye tıklanır, değer yazılır, eleman taşınır. Tek tıkla kalıcı ölçü ya da "Bu ölçüyü
  koru" bağı olur. Örnekler: Revit [W2 R6], Chief KB-01116; ADV-01 ile birlikte.
- **Mimari kenet türleri:** duvar yüzü, ekseni ve çekirdeği; açıklık merkezi ve kenarı. Kenetin türü imlecin yanında yazı ve
  şekille görünür (Vectorworks SmartCursor ipuçları).
- **Oda aracı:** dikdörtgen ya da çokgen oda → duvarlar + mahal + etiket, tek adımda. Bitişik oda ortak duvarı benimser
  (bugünkü "Ortak duvar: Benimse"). Duvar stili ve mahal adı listeden seçilir. Örnekler: BricsCAD BIMQUICKDRAW [W1 B4, B5],
  Floorplanner, Archisketch.
- **Kendiliğinden mahal:** döngü kapanınca isteğe bağlı mahal oluşur; çift tık da mahal yapar (Sweet Home 3D, Rayon,
  Cedreo). Sürüklerken canlı alan ve program hedefiyle fark görünür (ADV-04).
- **Odayı büyütme:** mahalin kenar tutamacı odayı büyütür; Duvarı kaydır'ın planını bağlarla kullanır.

**İlk kabul:**
- 2+1 kabul izi Oda aracı ve imleç yanı seçeneklerle yeniden yazılır: etkileşim 157 → ≤ 80, odak değişimi 15 → ≤ 3.
- Esc, Undo, Python ve MCP aynı sonucu verir.

#### ADV-12 — Donatı kataloğu ve akıllı yerleşim [~]

_Durum (8 Kasım 2026):_ Donatı araması ve duvara yaslı yerleşim yapıldı (M24, [ADR 8024](../docs/adr/8024-fixture-search-and-wall-placement.md)).
- Menü araması kitaplığın donatılarını bulur (Türkçe eş anlamlılarla); Enter yalnız o bloğu alır, Blok ekle onunla başlar.
- KentOS Mimari'de Duvara hizala açık başlar; donatı MM-TEFR-MOBILYA / VITRIFIYE / MUTFAK katmanına yazılır.
- "lavabo"dan duvara yaslı lavaboya 3 eylem (önce 6).
- Kullanım alanı denetimi (M25, [ADR 8025](../docs/adr/8025-fixture-use-zones-in-the-audit.md)): Mimari denetim'de Donatılar grubu; alana giren duvar, kolon, donatı ve kapı kanadı (düzeltme: kapıyı çevir); izdüşüm gizli zarfı saymıyor (sahte kapı çarpışması gitti).
- Kalanlar: tezgâh önü, yerleştirirken canlı uyarı, Donatı ▾'de kategoriler, sürükle-bırak.

**Kullanıcı sonucu:** "lavabo" yazıp sürükleyerek banyo donatılır. Donatı duvara yaslanır; kapının önünü kesince bu
söylenir. Dayanak S13.

- **Yer ve kategoriler:** Mimari › Yerleştir › Donatı ▾. Islak hacim, yatak ve oturma, mutfak, çalışma, dış mekân.
- **Arama:** Türkçe eş anlamlılarla (lavabo, klozet, duş teknesi, karyola, gardırop).
- **Yerleştirme:** sürükle-bırak; duvara yaslama ve kendiliğinden döndürme; tipik ölçüler.
- **Kullanım alanı ve çakışma:** kapı kanadı, klozet önü, tezgâh önü. Bulgular bugünkü mutfak çakışma denetimiyle aynı
  sistemdedir.

**Kaynak:** Sweet Home 3D; AutoCAD Smart Blocks Placement [W1 A11]; SmartDraw (W3 F).

**İlk kabul:**
- "lavabo" aramasından duvara hizalı yerleşime ≤ 3 eylem.
- Banyo örneğinde kullanım alanı çakışması işaretlenir.

#### ADV-13 — Mimari başlangıç, şablon ve varsayılanlar [~]

_Durum (7 Kasım 2026):_ Başlangıç yapıldı (M8, [ADR 8007](../docs/adr/8007-mimari-start-ground-floor-hatched-wall.md)).
- Yeni Mimari projesi Zemin kat'la (3,00 m) ve Mimari sekmesinde açılıyor.
- Duvar araçları Dış duvar 25 ile başlıyor; ilk duvar 1:100'de poşeli.
- Şablon m ve 1:100. Ayarsız eski otomasyon çağrısı cm ve 1:50 yazmayı sürdürüyor.
- Kalanlar:
  - ilk duvarın kendi katmanı (M9);
  - malzemeleri ADR 0791 kitaplığından gelen stiller ve 13,5 cm'lik iç bölme;
  - mahal adı listesi, örnek 2+1 projesi, ilk beş dakikanın ipuçları.

**Kullanıcı sonucu:** Boş projede ilk duvar poşesiyle, katıyla ve kendi katmanında çıkar. Dayanak S4, S12.

- **Şablon:** proje başlangıcıyla aynı birim ve ölçekte olur (m, 1:100; ADR 0626). Bugün cm ve 1:50
  (`catalog/v1/templates/mimari.json:7-8`).
- **Kat:** Zemin kat ve kat yüksekliği hazır gelir.
- **Varsayılan duvar stilleri** ("Yok" yerine), malzemeleri ADR 0791 kitaplığından:
  - dış duvar (tuğla ya da bims + yalıtım + sıva);
  - iç bölme (10 ve 13,5 cm);
  - betonarme perde.
- **Diğer hazırlar:** katman profili (ADV-14), mahal adı listesi, örnek 2+1 projesi, ilk beş dakikanın ipuçları.

**İlk kabul:**
- Boş projeden ilk duvarın poşesiyle, katıyla ve kendi katmanında yazılması ≤ 3 eylem.
- Şablonun birimi ve ölçeği proje başlangıcıyla aynı.

### 4.2 Plan grafiği ve AutoCAD düzeyinde ayrıntı

#### ADV-14 — Türe göre katman ve kalem hiyerarşisi [~]

_Durum (7 Kasım 2026):_ Katman anahtarı yapıldı (M9, [ADR 8008](../docs/adr/8008-ek4-layer-key.md)).
- Mimari nesneler EK-4 adlı katmanlara yazılıyor: MM-DUVA-TUGLA, MM-KAPI-GENEL, MM-XXXX-OLCU …
- Katman ilk nesneyle, aynı geri alma adımında açılıyor. Araç panelinin Katman satırı hedefi söylüyor ve anahtarı
  kapatabiliyor.
- 2+1 gerçek pencerede 54 nesneyi 9 katmana dağıttı. DXF'te taramalar "-T" katmanında.
- Kalem hiyerarşisi (M10, [ADR 8009](../docs/adr/8009-pen-hierarchy-of-parts.md)):
  - açıklık parçaları rollü (kasa, kanat, yay, cam, denizlik …);
  - yay, cam ve malzeme sınırları ISO 128-23'ün ince çizgisiyle, nesnenin kaleminin yarısıyla çiziliyor;
  - 1:50 PDF'te 0,5 / 0,25 / 0,125 mm ölçüldü.
- Kalanlar:
  - ölçeğe göre çizgi grubu (sahibin kararı bekleniyor: ISO 128-23 hangi ölçeğe hangi grubun düştüğünü söylemiyor) ve
    kesme düzleminin önündeki çizgiler (M13);
  - ofis ve proje profili;
  - var olan nesneleri dağıtan komut;
  - donatı ve blok katmanları (M24'te yapıldı: MM-TEFR-MOBILYA, MM-TEFR-VITRIFIYE, MM-TEFR-MUTFAK);
  - ~~mahal yazısının DXF'e gitmesi~~ (M46'da yapıldı, ADR 8046 §1).

**Kullanıcı sonucu:** Çizim bir ofis çizimi gibi katmanlıdır. Paftada kesilen duvar kalın, görünen ince, üstteki kesikli
çizilir. Dayanak S4, S5.

- **Katman anahtarı:** nesne türü ve alt bileşen → katman. Araç ayarında "Katman: Otomatik"; ofis profili değiştirilebilir,
  proje profili saklanır.
  - **Temel:** 2026 Dijital Yönetmeliği'nin **EK-4**'ü (Md. 7(1)). EK-4 okunmadan katman adı uydurulmaz.
  - **EK-4 bir alana ad vermiyorsa:** TS EN ISO 13567'nin alan yapısı ve ÇŞİDB'nin CADD usul ve esasları.
  - **Örnekler:** AutoCAD Architecture layer key styles; AutoCAD 2026.1'in otomatik katmanları [W1 A6].
- **Kalem:**
  - Alt bileşen kalemleri: kasa, kanat, yay, eşik, cam, denizlik, tarama sınırı.
  - Kesilen, görünen ve üstteki çizgi sınıfları.
  - Ekranda çizgi kalınlığı gösterimi; siyah-beyaz baskı.
- **DXF:**
  - Alt bileşenler (tarama, kapı yayı) kendi katmanında gider.
  - Mahalin adı ve alanı yazı olarak gider; şaft çaprazı da gider. ADR 0708 §2 ile bugünkü yazıcı arasındaki çelişki
    böylece kapanır.

**İlk kabul:**
- 2+1 örneğinde nesneler türlerine göre en az 8 katmanda.
- DXF dış programda açılınca aynı katmanlar.
- 1:50 PDF'te kalem sınıfları ölçülür.

#### ADV-15 — Ölçeğe ve görünüşe göre temsil [~]

_Durum (7 Kasım 2026):_ Kapı ve pencere ölçeğe göre çiziliyor (M11, [ADR 8010](../docs/adr/8010-openings-by-scale.md)).
- Düzey duvarınki: paftada haritanın ölçeği ya da görünüş tarifi, ekranda yakınlık.
  - 1:200'de kapı kanadıyla, pencere camıyla çizilir.
  - 1:100'de yay ve pencerenin iki yüzü eklenir; kasası çizilmeyen pencere duvar ucundan ucuna uzanır.
  - 1:50'de kasa kesitleri, eşik, kayıt, denizlik ve pervaz da çizilir.
- Dayanak: Mimarlar Odası Mimari Proje Çizim ve Sunuş Standartları. Ön proje 1/100'de "kapıların açılış yönleri
  belirtilir"; uygulama projesi 1/50'de "doğramalar detayına uygun" çizilir.
- Aynı kat üç paftada PDF'e basıldı. Kapı ve pencere çizgisi 1:200'de 2, 1:100'de 5, 1:50'de 9; önce üçünde de 9'du.
- Kolon, merdiven, rampa, döşeme kenarı ve mobilya da ölçeğe göre çiziliyor (M12,
  [ADR 8011](../docs/adr/8011-elements-by-scale.md)).
  - 1:200 en sadedir: kolonda yazı, merdivende ve rampada korkuluk, merdivende numara, döşeme kenarında iç çizgi yok;
    mobilya dış çizgisiyle.
  - 1:50'de kolonun ölçüsü (30/50) ve merdivenin rıht ve basamağı (11 × 17.5/28) yazılır: Mimarlar Odası uygulama
    projesinde "kolon isimleri ve ölçüleri", merdivenin "basamak adedi … ve rıht yüksekliği" ister.
  - Aynı kat üç paftada PDF'e basıldı ve gerçek pencerede önceki ikiliyle karşılaştırıldı.
- Kesme yüksekliği ve üstteki çizgiler (M13, [ADR 8012](../docs/adr/8012-plan-cut-and-overhead.md)):
  - kesmenin üstündeki pencere duvarı kesmez, kesikli çizilir; yükseklik Proje ayarları › Mimari'de verilir (yeni
    projede 1,20 m, İTÜ'nün alt ucu). 1:50 PDF'te kesme 1,20'de yüksek pencere kesikli, 1,80'de kesik;
  - bir üst katın döşemeleri (balkon, çıkma) etkin katın planında ince kesikli, seçilmez (Mimarlar Odası §14.5.3 u).
- Ölçeğe göre çizgi grubu ([ADR 8013](../docs/adr/8013-line-groups-by-scale.md), sahibin onayı 8 Kasım):
  TS 88-23 / ISO 128-23 grupları, duvar 1:200'de 0,25, 1:100'de 0,35, 1:50'de 0,5, 1:20'de 0,7 mm, kapı-pencere yarısı.
  Yeni Mimari projesinde açık; Proje ayarları › Mimari › Çizgi kalınlıkları.
- TS 5319 taramaları ve yalıtım simgesi (M14, [ADR 8014](../docs/adr/8014-ts-5319-hatches-and-insulation.md)):
  SN46–SN58 kitaplıkta; ısı yalıtımı katmanın iki yüzü arasında SN55 zikzağı; kolon 1:20'de SN46, kolon aracıyla dolu
  (Mimarlar Odası §14.5.3 i). 1:20 PDF'te iki yalıtım zikzağı.
- Kalanlar:
  - 1:20 kasa profili; duvar katmanına ölçeğe göre ikinci tarama (perde 1:20'de SN46);
  - kiriş izi ve asma tavan (nesneleri yok);
  - mobilyanın parça parça ayrıntı düzeyi, ıslak hacim ve mutfak bloklarının sade gösterimi;
  - web çiziminin düzeyi ölçeğe göre seçmesi.

**Kullanıcı sonucu:** Aynı plan 1:100'de sade, 1:50'de uygulama, 1:20'de detay ayrıntısıyla çizilir. Dayanak S4.

- **Üç düzey bütün nesnelerde:** duvardaki Şematik/Uygulama/Ayrıntılı düzeyleri (`arch.rs:94-125`) bütün mimari nesnelere
  yayılır. Kapı ve pencere için:
  - 1:200: boşluk + kanat;
  - 1:100: kanat + yay;
  - 1:50: kasa, kanat, yay, eşik, denizlik;
  - 1:20: profil.

  Kolon, merdiven, döşeme, mobilya ve taramalar da düzey alır. Görünüş tarifi düzeyi geçersiz kılabilir.
- **Plan kesme yüksekliği:** ayarlanır (bugün sabit 1,10 m, `opening.rs:21`). Kesilen, görünen ve üstteki (kesikli)
  çizgiler ayrılır: kiriş izi, üst kat çıkması, asma tavan.
- **Tarama ve yalıtım:** malzeme tarama kitaplığı ve yalıtım simgesi. Taramanın yoğunluğu ölçeğe göre seçilir.
- **Ölçeklenen açıklama:** annotative ölçekleme bütün mimari açıklamalarda.

**Kaynak:** AutoCAD Architecture display configurations [W1 R3, R4]; Revit Detail Level [W2 R12]; Archicad MVO ve PSD
[W2 A5, A6]; Allplan reference scale [W2 L3].

**İlk kabul:**
- Aynı kat 1:100 ve 1:50 paftada farklı kapı, pencere, kolon ve merdiven çizimiyle basılır.
- 1:20 detayda yalıtım simgesi görünür.
- Siyah-beyaz PDF basılır.

#### ADV-16 — Otomatik ölçü, kot ve etiket [~]

_Durum (8 Kasım 2026):_ Planı tek komutla ölçülendirme (M26, [ADR 8026](../docs/adr/8026-dimension-the-plan-in-one-command.md)) ve EK-3 mahal numarasıyla elips etiket (M27, [ADR 8027](../docs/adr/8027-ek3-room-numbers-and-ellipse-tags.md)) yapıldı.
- Ölçü ▾ › Planı ölçülendir: dış duvarları kendisi bulur; dört cepheye ön projede 3, uygulamada 4 zincir (MO); her mahali enine ve boyuna ikişer kez kesen, binayı baştan başa geçen iç zincirler; önizleme, Enter ile tek adım.
- Mahal numarala'da EK-3 (E): Kat_Bölüm_Sıra, kat kodu Etkin kattan (K00, K01, B01, NNN), kuzeyden saat yönünde; tıklamadan Enter katın bütün mahallerini numaralar. Hepsini etiketle'de Çerçeve (B) "Elips içinde numara": numara elipste, ad ve alan altında (KCAD şema 219).
- Doğrama kodları (M28 1. bölüm, [ADR 8028](../docs/adr/8028-joinery-codes-and-tag-places.md)): Etiket ▾ › Doğrama kodları aynı tipteki açıklıklara aynı kodu verir (K01, GK01 dış kapı, P01, B01; kod tipin üstünde, KCAD 220) ve etiketler kodu ve 90/210'u yazar; etiket duvara, kapı yayına, oda etiketine ve ölçü yazısına binmeyen yere konur (Hepsini etiketle de).
- Mahale göre numara (M28 2. bölüm): Açıklıkları numarala'da M; kapı hizmet ettiği mahalin (koridor ile oda arasında odanın, iki oda arasında açıldığının), pencere bulunduğu mahalin numarasını alır (K01, P01-A); kapı-pencere çizelgesinde Kod sütunu.
- Kalanlar: yeniden çalışınca güncelleme, kesenin boşluktan kaçması, kot, ölçeğe göre etiket içeriği, ölçü zincirinde açıklık yüksekliği.

**Kullanıcı sonucu:** Tek komut planın dört cephesini ölçülendirir. Etiketler okunur ve hiçbir çizginin üstüne binmez.
Dayanak S19.

- **Dış ölçü:** tek komutla dört cephe dış ölçü zinciri; toplam, açıklıklar, duvar yapısı seçenekli; boş zincir üretmez.
  MO kuralı: ön projede 3, uygulamada 4 zincir. Örnekler: Archicad [W2 A12], Vectorworks [W2 V11], Chief.
- **İç ölçü:** mahal başına 2+2 zincir.
- **Kot işareti:** ±0,00 mutlak kotla ve röperle; mahalin döşeme kotu.
- **Mahal etiketi:** içerik ölçeğe göre değişir (no + ad + net alan; 1:50'de kaplama ve kot da). MO biçimi elips; numara
  EK-3'e göre kat + fonksiyon + sıra.
- **Etiket yerleşimi:** çakışmasız; örnekten etiketleme (Revit 2027 Tag by Example [W2 R1]).
- **Açıklık:** ölçü yazısı "90/210"; doğrama kodları K/P/GK/CK (MO); odaya göre numaralandırma.

**İlk kabul:**
- 2+1 planında dört cephe zinciri tek komutla çıkar.
- Etiketler duvar çizgisine binmez (M0'daki çakışmalar yok).
- Kapı ölçüsünde 90/210 yazar.

#### ADV-17 — Tadilat grafik dili [x]

_Durum (9 Kasım 2026):_ M39 1. bölüm yapıldı ([ADR 8039](../docs/adr/8039-renovation-graphic-language.md)): kavramın adı "Evre" değil Tadilat durumu (Mimarlar Odası: Mevcut, İptal, İlave; Geçici); kapı ve pencerenin kendi tadilat durumu (KCAD 230; Öznitelikler'de Duvarınki, Mevcut, İptal, İlave, Geçici); görünümün gizlediği duvar birleşmez, gizlediği açıklık kesmez (mevcut duvara açılan kapı Mevcut durumda delik bırakmaz, iptal pencerenin yeri Yeni durumda dolu); kendi görünümünü gösteren pafta haritası kendi kopyasından kesilir. Araçlarda tadilat durumu yapıldı (1b): yeni çizilen görünümün durumunu alır (Mevcut durum/Yıkım planı → Mevcut, Yeni durum → İlave); Yeni durumda mevcut duvara konan kapı İlave. DXF'te durum katmanları yapıldı (EK-4 4.14.3/4.14.5 kalıbıyla MM-DUVA-TUGLA-IPTAL kesikli, -ILAVE, -MEVCUT; DXF görünümün gösterdiğini alır). Tadilat projesi görünümü ve lejantı yapıldı (KCAD 231; MO: mevcut düz, ilave taralı, iptal kesikli; kapatılan boşluk taralı dolgu, açılan kapıda kaldırılan duvar kesikli). Tadilat takımı yapıldı (§6): Ruhsat takımı'nda Tadilat projesi kutusu (çizim tadilatsa işaretli); her kat üç pafta
(mevcut durum, tadilat projesi, yeni durum; EK-2 sıra 01–03), her biri takımın adlı görünümüne bağlı, vaziyet planı
tadilat projesi tekniğiyle; tadilatta değişen ama İptal yapılmamış odalar adıyla söylenir; tadilat projesinde iptal oda
ve etiketi gizli. **M39 yapıldı.**

**Kullanıcı sonucu:** Mevcut, yıkılacak ve yeni bir bakışta ayrılır; üç evre paftası aynı modelden çıkar. Dayanak S21.

- **Açıklığın kendi evresi:** bugün evre duvardan okunuyor (`entity.rs:1999-2010`). Mevcut duvarda yeni kapı ve kapatılan
  boşluk (dolgu) gösterilir; parçasal yıkım yapılır.
- **Araçlarda evre:** yeni nesnenin evresi araçta seçilir (ADR 0776 K6).
- **Evre görünüşleri:** mevcut durum, yıkım planı, yeni durum. Türkiye'de yerleşik çizgi ve renk dili dilim başında
  araştırılır; bulunamazsa ofis profili ayarı olur.
- **DXF:** evre katmanları.

**Kaynak:** AutoCAD Architecture renovation [W1 R7]; Revit phase filters [W2 R9, R10]; Archicad renovation filters
[W2 A11]; Vectorworks phasing [W2 V7, V8].

**İlk kabul:**
- Mevcut duvara yeni kapı açılır, bir pencere kapatılır.
- Üç evre paftası basılır.
- DXF'te evre katmanları bulunur.

### 4.3 Yapı modeli ve türetilmiş çizimler

#### ADV-18 — Düşey model ve taşıyıcı sistem [x]

_Durum (10 Kasım 2026):_ M40 yapıldı ([ADR 8040](../docs/adr/8040-vertical-model.md)): duvar betonarmeden betonarmeye,
katın yüksekliğini geçmez; kat yüksekliği değişince üstteki katlar ve duvar tepeleri izler; döşeme katmanları; mahalin
döşeme kotu, asma tavanı ve iç yüksekliği (PAİY 28/4–6). M41 yapıldı ([ADR 8041](../docs/adr/8041-beams-lintels-column-wall-joins.md)):
kiriş planda kesikli, numarası ve ölçüsü ölçeğe göre; duvar kirişin altında biter; kolonun malzemesi, duvar kolonun
yüzünde kesilir, perde + kolon tek kesit (TS EN ISO 128-3, 7.2), tuğla kolonda biter; lento (TBDY 11.5.3) ve hatıl
kirişin türleri; Metraj › Kirişler. Üç kabul de gerçek pencerede doğrulandı. **ADV-18 yapıldı.**

**Kullanıcı sonucu:** Kat yüksekliği değişince duvar tepeleri izler. Kiriş planda kesikli görünür. Kolon ile duvarın
taraması kesintisiz birleşir. Dayanak S20; Dijital Yönetmelik Md. 9(1) (çizim ile model uyumu).

- **Duvar ve döşeme:**
  - Duvarın taban/tepe kuralı kata ve üst döşemeye bağlanır.
  - Döşeme katmanları: kaplama, şap, döşeme, sıva.
  - Asma tavan ve mahalin döşeme kotu.
- **Taşıyıcı elemanlar:**
  - Kiriş, lento ve hatıl; planda kesikli izdüşümle.
  - Kolon–duvar birleşimi ve kolon malzemesi (ADR 0771 sınırı).
  - Kolon etiketi "25/50".

**İlk kabul:**
- İki katlı konutta kat yüksekliği değişince duvar tepeleri izler.
- Kiriş planda kesikli görünür.
- Perde + kolon + tuğla düğümünde tarama süreklidir.

#### ADV-19 — Kesit ve görünüş türetimi [ ]

_Durum (10 Kasım 2026):_ M42 1. bölüm yapıldı ([ADR 8042](../docs/adr/8042-section-derived-from-a-section-mark.md)):
Mimari › İşaret ▾ › Kesit çıkar, kesit işaretinden 1/50 kesiti modelden çıkarır (kesilen kalın ve EK-4 4.14.5 `-T`
katmanında taralı, görünen derinliğine göre `-1`…`-4`, merdiven kolu basamaklarıyla, döşeme katmanları, kotlar, akslar,
mahaller, tesviye zemini, başlık) ve işaretin bloğuna yazar; plan değişince durum çubuğunda "B-B kesiti eski", Öznitelikler
› Durum › Yeniden çiz; plandaki kesit çizgisi noktalı kesikli (EK-4 4.28). M42 2. bölüm yapıldı: Ruhsat takımında her
kesitin KS paftası (planlardan sonra, kesitin ölçeğinde; işaretin balonu paftanın numarasını yazar; vaziyet ve kat planı
pencereleri kesitleri basmaz), yayında eski kesit uyarısı, Denetim › Mevzuat'ta PAİY 57/2-a-6 (en az iki kesit, biri
ortak merdivenden). M43 1. bölüm yapıldı ([ADR 8043](../docs/adr/8043-elevation-derived-from-an-elevation-mark.md)): Mimari ›
İşaret ▾ › Görünüş çıkar, görünüş işaretinin baktığı cepheyi 1/50 çıkarır (kapı ve pencereler Türkiye'nin açılım
çizgileriyle, zeminin altı kesik, adı ve başlığı kuzey okuna göre "KUZEY GÖRÜNÜŞÜ"), eskir ve Yeniden çiz ile güncellenir,
Ruhsat takımında kesitlerden sonra GR paftası. M43 2. bölüm sürüyor (ADR 8043 §7–§8): aynı düzlemde ve aynı kaplamada
birleşen yüzlerin ek yeri çizilmez (sıvalı cephede kiriş ve döşeme alnı yok), görünüşün derinlik dilimi en yakın yüzden;
Görünüş çıkar › Tümü (T) çizilmemiş bütün görünüşleri yan yana tek adımda çıkarır, işaret yoksa binanın dört yanına koyar.
Denetim › Mevzuat görünüşü çıkarılmamış cepheyi söyler (`paiy_elevations_missing`, ADR 8043 §9); görünüş görünen cephe
kaplamasını yazar (§10). M43 yapıldı; renk alanı sonraya.

**Kullanıcı sonucu:** En az iki kesit ve dört görünüş (PAİY 57(2)(a)) plandan türer. Plan değişince eskidikleri görünür.
Dayanak S20, S22.

- **Kesit:** kesit çizgisinden 2,5B. Kesilen kalın ve taranmış, görünen ince çizilir. Döşeme katmanları, merdiven, kotlar ve
  akslar görünür; kesitlerden biri merdivenden geçer.
- **Görünüş:** doğramalar ve açılım işaretleri, kaplama. Doğal zemin kesik, yeni zemin sürekli çizilir; kotlar yazılır.
- **Eskime:** plan değişince "eski" işareti; yeniden üretim aynı çizimi verir. Kalıp ADR 0799; BricsCAD'in kırmızı ve
  turuncu işareti [W1 B9].
- **Bilinmeyenler:** modelin bilmediği elemanlar listelenir. Sahte otomatik kesit yapılmaz (ADR 0347 §3).

**İlk kabul:**
- 1:50 bir kesit ve bir cephe PDF'te basılır.
- Bir kapı değişince ikisi de "eski" olur.
- Yeniden üretim aynı çizimi verir.

#### ADV-20 — Çatı ve çatı planı [ ]

**Kullanıcı sonucu:** Çatı planı paftası (PAİY 57(2)(a)) modelden çıkar. Dayanak S20, S22.

- **Çatı türleri:** tek, beşik ve kırma eğimli çatı; düz çatı.
- **Çatı öğeleri:**
  - eğim yönü ve yüzdesi;
  - mahya, dere ve kalkan;
  - saçak, oluk ve iniş borusu;
  - baca ve havalandırma çıkışları, çatı penceresi;
  - kar siperliği (Sivas yerel kuralı).
- **Kesit:** çatı kesitlerde de çizilir.

**Kaynak:** AutoCAD Architecture slabs and roof slabs [W1 R13]; PAİY 40, 42.

**İlk kabul:**
- Kırma çatı + baca + çatı penceresi çizilir.
- Çatı planı paftası basılır.
- Çatı kesitte görünür.

#### ADV-21 — Merdiven, korkuluk, asansör ve özel elemanlar [ ]

**Kullanıcı sonucu:** Konutta karşılaşılan merdiven ve özel elemanlar hem plan sembolüyle hem denetimle gelir.

- **Merdiven:**
  - Balansmanlı merdiven ve karışık kollar.
  - Çok katlı merdiven: kat yüksekliği değişince rıht sayısı güncellenir (ADR 0724 açığı).
  - Plan etiketi "n × r / b".
- **Korkuluk:** galeri ve balkon için bağımsız korkuluk.
- **Asansör:** kuyu, kabin ve kapı sembolü; ölçüleri PAİY 34'te.
- **Diğer:** otopark yeri (2,40 × 4,90 m), giydirme cephe (2B ızgara), peyzaj kitaplığı.

**İlk kabul:**
- Balansmanlı L merdiven merdiven denetiminden geçer.
- Galeri korkuluğu çizilir.
- Asansör sembolü kapısıyla yerleşir.

### 4.4 Türkiye'ye uygun proje

#### ADV-22 — Alan hesabı, vaziyet planı ve imar tabloları [~]

_Durum (9 Kasım 2026):_ PAİY 4'ün bağımsız bölüm alanları yapıldı (M29 1. bölüm, [ADR 8029](../docs/adr/8029-regulation-areas-of-independent-units.md)).
- Bölüm çizelgesinde BB net, BB brüt (dış cephede dış yüz, komşuyla eksen; şaft ve galeri hariç), ortak alan payı (ortak alanların brütünden) ve BB genel brüt; eski "Brüt" (net + pay) "Net + pay" adını aldı; KCAD 221.
- Metrekare cetveli (M29 2. bölüm): Metraj ▾ › Metrekare cetveli, bölüm çizelgesi kat kat (`byLevel`): her katta bölümler ve ortak alanların o kattaki yönetmelik brüt ve net alanı, toplam binanın.
- Emsal ve TAKS (M30, [ADR 8030](../docs/adr/8030-emsal-taks-zoning-badge-and-table.md)): Proje ayarları › Mimari › İmar durumu (parsel, emsal, TAKS); mahalin emsal sınıfı kullanım listesinden ya da adından; durum çubuğunda canlı rozet (aşımda kırmızı), Öznitelikler › İmar durumu; Metraj ▾ › Emsal ve TAKS tablosu (PAİY 57/3'ün değerleri, kat kat alanlar, her adımda ve ayar değişince yenilenir); KCAD 222.
- Kat irtifakı (M31, [ADR 8031](../docs/adr/8031-condominium-units-annexes-and-land-shares.md)): eklenti grubu ve bölümü, bölümün niteliği, değer katsayısı ve arsa payı paydası; toplam brüt = brüt + eklenti, genel brüt = toplam + pay; arsa payı `kentos.land/1` (payda: program, yoksa parsel alanı); Metraj ▾ › Kat irtifakı tablosu; KCAD 223.
- Vaziyet planı (M32, [ADR 8032](../docs/adr/8032-site-plan-parcel-setbacks-and-datum.md)): parsel (kapalı çizgi), kenar türleri ve PAİY 23 mesafeleri (kat adedine göre artış), canlı yapı yaklaşma sınırı, oturumun kenarlara uzaklığı, ağaç sayısı, ±0,00 kotu; rozet, Öznitelikler ve Emsal ve TAKS tablosunda; çizimde cephe ortası bahçe ölçüleri, ±0,00 işareti ve kot yazısı; KCAD 224.
- Otopark ve sığınak (M36, [ADR 8036](../docs/adr/8036-parking-and-shelter-provisions.md)): Metraj ▾ › Otopark hesabı (Ek-1: konut brüt alana göre 80/120/180 m² eşikleri, dükkân 50 m²'ye 1 ve bölüm başına en az 1, büro 50 m²'ye 1; toplam bir üste, 8/4; Otopark/Garaj mahallerinin net alanı ÷ 20 m², 4/1-ç; 20 yerde %5 şarj, 5/1-j) ve Sığınak hesabı (8/a: oda sayısına göre 2/3/4 kişi, konut dışı brüt ÷ 20, kişi başına 1 m² net; 7/a–c muafiyetleri; tuvalet ve çıkış); eksikler Denetim › Mevzuat'ta; KCAD 227.
- Kalanlar: kat ara toplamı, gruba alınmamış mahallerin yapı inşaat alanı, merdiven altı 1,80 m, sıvasız projede sıva payı, emsal sınıfı arayüzü ve 22. maddenin %20/%50/70 m² sınırları, ortak yer sözlüğü ve elle arsa payı, vaziyet planında ağaç simgeleri ve bağlı ölçüler.

**Kullanıcı sonucu:** Metrekare cetveli, emsal/TAKS tablosu, kat irtifakı tablosu ve vaziyet planı aynı modelden çıkar.
Sınırı aşan değer çizim sırasında görünür. Dayanak S22; W4 §4/1–3, 8–10.

- **Türk alan motoru (PAİY Md. 4):**
  - Bağımsız bölümün net, brüt, genel brüt ve toplam brüt alanı.
  - Yapı inşaat alanı ve emsale esas alan.
  - Kontur kuralları: sıva payı 2,5 cm; kolon, şaft ve galeri düşümü; merdiven altı 1,80 m.
- **Emsal ve TAKS:** her mahal emsal ve taban durumunu madde koduyla taşır. %30 emsal harici sınırı ve öbür sınırlar
  denetlenir; aşan kısım emsale aktarılır. İmar durumu girdisinden canlı rozet ve tablo üretilir.
- **Kat irtifakı:** bağımsız bölüm, eklenti ve ortak yer modeli (KMK 2–4, 12); arsa payı; paylaşım tablosu.
- **Vaziyet planı:** parsel (Harita işinden ya da koordinat tablosundan); bahçe mesafeleri ve kat adedine göre artış
  (PAİY 23); bina oturumu, ±0,00 ve röper, kuzey; ağaç hesabı.
- **Otopark ve sığınak hesapları:** ilgili yönetmeliklerden, çizelge olarak.

**Kaynak:** İdeCAD kat alanı hesabı [W2 I12]; Revit area schemes [W2 R17]; PAİY 4, 5, 20–23, 57; Otopark ve Sığınak
yönetmelikleri.

**İlk kabul:**
- Tek parselde iki bağımsız bölümlü bina: metrekare cetveli, emsal/TAKS ve kat irtifakı tabloları bağımsız başvuruyla aynı.
- Resmî onay iddiası yapılmaz.

#### ADV-23 — Sürümlü mevzuat kural motoru ve denetimler [~]

_Durum (9 Kasım 2026):_ Denetim › Mevzuat grubu yapıldı (M33, [ADR 8033](../docs/adr/8033-regulation-rules-rooms-storeys-doors.md)): PAİY 29/1–3 piyesleri, eksik piyes ve koridor, 28/1-c kat yüksekliği, 39/1 kapı yüksekliği ve net genişlikleri; her bulgu maddesini ve Resmî Gazete'sini (`rule`) taşır. M34 ([ADR 8034](../docs/adr/8034-regulation-stairs-shafts-elevators-local-rules.md)): merdiven (31), rampa (30/12), korkuluk ve pencere (38/1), ışıklık ve hava bacası (32/2–3), asansör zorunluluğu, kabin ve sahanlık (34/1–2, 1.7.2026); belediye katmanı (Sivas İmar Yönetmeliği, projede seçilir): sıkı değer bağlar, gevşek değerin tuttuğu ölçü çelişki olarak söylenir (PAİY 69/3, idare karar verir). M35 ([ADR 8035](../docs/adr/8035-fire-escape-routes-distances-load-exits.md)): Binaların Yangından Korunması Hakkında Yönetmelik — kaçış uzaklığı (en uzak nokta duvardan 40 cm), tek yön/iki yön ve çıkmaz koridor (Ek-5/B), konutta 48/2 ve 48/4, kişi yükü (Ek-5/A) ile çıkış genişliği ve sayısı (33/1, 39/2), koridor, yüksek bina (33/2), konut merdiven sayısı (48/5); yağmurlama ayarı. Proje ▾ › Kaçış yolları rotaları plana çizer (sınırı aşan kırmızı). Kalanlar: başvuru tarihi ve bölge türü, iç yükseklik, plan notu katmanı, kapı açılış yönü ve yangın dayanımları.

**Kullanıcı sonucu:** Mimari denetim, çizimin hangi yönetmelik maddesini karşılamadığını ölçülen yer ve değerle söyler.
Kuralın kaynağı ve yürürlük tarihi görünür. Dayanak S22; W4 §4/4–7, 10.

- **Kural:** kaynak, madde, RG tarih/sayı, yürürlük başlangıcı/bitişi, kapsam (bina türü, yükseklik, bağımsız bölüm
  sayısı), ifade ve şiddet taşır. Kural seti ruhsat başvuru tarihine göre seçilir.
- **Katmanlar:** ulusal (PAİY, yangın, otopark, sığınak) → belediye (Sivas) → parsel plan notu. Çelişki görünür kılınır
  (ör. hava bacası).
- **Kurallar, sırasıyla:**
  1. Piyes asgarileri ve iç yükseklikler (28, 29).
  2. Koridor ≥ 1,20 m; kapılar 1,50/1,00/0,90 m.
  3. Işıklık ve hava bacası (32).
  4. Merdiven (31: rıht, 2a+b, kol genişliği, sahanlık, baş yüksekliği) ve rampa.
  5. Korkuluk 1,10 m; asansör zorunluluğu ve kabin ölçüleri.
  6. Yangın kaçışı: 40 cm içeriden rota, uzaklıklar, kullanıcı yükünden çıkış genişliği.
- **Bulgu:** yeri, ölçülen değeri ve maddeyi gösterir. Mevcut istisna mekanizması kullanılır; ikinci bir denetim sistemi
  kurulmaz (ADV-03, ADR 0732).

**İlk kabul:**
- 2+1 örneğinde her kural için bir geçen ve bir kalan durum.
- Bağımsız başvuru kuralları KentOS kodu olmadan sınar.
- Sivas ile PAİY arasındaki çelişki raporda görünür.

#### ADV-24 — Türk proje takımı: antet, paftalar ve listeler [~]

_Durum (9 Kasım 2026):_ PDF/A-2b yapıldı (M37 1. bölüm, [ADR 8037](../docs/adr/8037-turkish-title-block-sheet-sets-pdfa.md)): pafta yazıcısında XMP, sRGB çıktı niyeti ve kendi profili; projenin PDF okuyucusuyla PDF/A-2 denetimi; Paftaları toplu yayınla'da "PDF/A-2 arşiv biçimi" (`file.publish.sheets` `pdfa`), her dosya teslimden önce denetlenir. Ruhsat takımı yapıldı (M37 2. bölüm): MO anteti (`sys:mimari-ruhsat`; pafta başlığı sağ altta, revizyonlar aşağıdan yukarı, M harfi), künye çizimin değişkenlerinde (cevapsız pafta sorusu çizimin değerine yol verir), arsa alanı/emsal/kat adedi/BB/kullanım çizimden (`title_facts`); Mimari › Proje › Ruhsat takımı: proje bilgileri (PAİY 57/10 tablosu ve canlı pafta listesi), vaziyet planı, kat planları, UP-01…, en küçük A kâğıdı, kilitli haritalar; yayın penceresi takım için PDF/A ve dijital yönetmelik EK-2 dosya adlarıyla (`{@proje_kodu}_{@bina_kodu}_MM_XXXX_…`) denetlenmiş açılır. Hesap paftası yapıldı (M37 3. bölüm): metrekare cetveli, emsal ve TAKS, kat irtifakı, otopark ve sığınak tabloları çizimde yoksa kurulur ve "Mimari hesap raporu" paftasında (EK-2 HS/MIR) kilitli haritalarla gösterilir. Mahal listesi yapıldı (M38 1. bölüm, [ADR 8038](../docs/adr/8038-room-list-and-graphic-joinery-list.md)): mahalin denizlik ve parapet, merdiven basamak ve rıht, küpeşte ve korkuluk kaplaması (KCAD 228) ve Öznitelikler'de bütün kaplamalar; her kapı ve pencere hizmet ettiği mahalde doğrama koduyla ("K01 (2)"); Çizelge ▾ › Mahal listesi kat kat (en alttan, katın başlığıyla); ruhsat takımında planlardan sonra kendi kâğıdında "Mahal listesi" paftası (EK-2 LS/MHL). Grafik doğrama listesi yapıldı (M38 2. bölüm): kapı ve pencere tipine kasa ve kanat ile U değeri (KCAD 229; TS 825:2024 pencere ≤ 1,8 W/m²K Mevzuat'ta); Etiket ▾ › Doğrama listesi her tipi kodu, Türkiye'nin açılım çizgisiyle görünüşü, kaba ölçüsü, denizliği, açılımı, kasa-kanadı, U değeri ve adediyle tek canlı blokta (MM-XXXX-DOGRAMALISTESI) gösterir; takımda 1/50 kendi paftası (EK-2 LS/MMC). Kalanlar: vasistas ve çift açılım, ağaç ve toprak kazı hesabı, A4 katlama izleri, çatı planı ve kesitler (ADV-19/20).

**Kullanıcı sonucu:** "Ruhsat takımı" komutu, PAİY 57(2)(a)'nın istediği paftaları Türk antetiyle tek PDF/A takımı olarak
basar. Dayanak S22; W4 §3, §4/11–12.

- **Pafta takımları:** proje evresine göre hazır gelir:
  - ön proje 1/100;
  - ruhsat 1/100 (belediye isterse "1/50 tekniğinde");
  - uygulama 1/50;
  - detay 1/20–1/5.

  Pafta kodları ve A-serisi kâğıt; A4 katlama izleri.
- **Antet:** sağ altta; müellif, sicil no, ada/parsel, pafta no, ölçek, tarih; üstünde revizyon tablosu (TS EN ISO 7200,
  MO). İlk pafta bilgi tablosu (57(10)).
- **Listeler:**
  - Mahal listesi (57(29)): döşeme, duvar ve tavan kaplaması, süpürgelik, denizlik.
  - Grafik doğrama listesi: tip görünüşü, kaba ölçü, açılım, adet, U değeri.
  - Metrekare cetveli ve kat irtifakı tablosu (ADV-22).
- **Çıktı:** tek tıkla PDF/A takımı, her pafta ayrı dosya (Dijital Yönetmelik Md. 4).

**İlk kabul:** 2+1 örneğinden ruhsat takımı tek komutla basılır. Kesit ve görünüş ADV-19'la, çatı planı ADV-20'yle takıma
girer.

### 4.5 Değişim ve kabul

#### ADV-25 — Değişim: DWG/DXF, IFC ve eski planı tanıma [ ]

**Kullanıcı sonucu:** Ofisin DWG'si katmanları ve yazılarıyla gider gelir. 2029'dan sonra istenecek IFC modeli aynı
çizimden üretilir. Dayanak S5; Dijital Yönetmelik Md. 4, 9–12; W4 §4/14.

- **DXF/DWG:** ADV-14'teki katman profiliyle; mahal yazıları ve evre katmanları da gider.
- **IFC4 mimari dışa aktarma** (yönetmelik EK-5 §5.4: IFC 4.3; ADR 8047)**:**
  - Sınıflar: IfcWall, IfcDoor, IfcWindow, IfcSpace, IfcSlab, IfcStair; EK-5'teki zorunlu sınıflarla eşlenir.
  - Öznitelikler EK-6 ve EK-7'ye, proje ve dosya kodları EK-1 ve EK-2'ye göre.
  - Plan ile model arasında tutarlılık denetimi (Md. 9).
- **Eski DWG planından tanıma:** patlatılmış plandan duvar, açıklık ve donatı tanınır. Örnekler: BricsCAD BIMQUICKDRAW'ın
  2B plandan duvarı, BLOCKIFY [W1 B7], AutoCAD Smart Blocks Search and Convert [W1 A7].

**İlk kabul:**
- DXF dış programda aynı katmanlarla açılır.
- IFC'de duvar, kapı ve mahal sayısı çizimle eşit; bağımsız okuyucu sınar.
- Gerçek bir ofis DWG'siyle tadilat örneği yapılır.

#### ADV-26 — Kabul ve ölçüm altyapısı [~]

_Durum (7 Kasım 2026, [ADR 8000](../docs/adr/8000-real-window-acceptance.md), M1):_
- **Gerçek pencere düzeneği yapıldı** (`scripts/usage/real/`):
  - Xvfb'de gerçek X penceresi; XTEST ile gerçek fare ve klavye olayları; lavapipe;
  - her koşu temiz ev dizini ve sahibin ayarlarının kopyası;
  - KLM sayacı.
- **2+1 dairenin gerçek senaryosu** (`fixtures/interaction/real/daire-2p1.txt`): 147 etkileşim, 177,6 s; on fark (§1.5).
- **Açık:** `kullan --modul`, `snapshot --sekme`'nin CAD sekmeleri, sayaç kurallarının kod içi izlere uygulanması ve
  izlerde fareyle yapılamayan adımların ayıklanması.

**Kullanıcı sonucu:** Her dilimin kullanıcıya etkisi sayıyla ve sahibin göreceği resimle kanıtlanır. Dayanak S23.

- **Oynatıcı ve görüntü:** `kentos-cad kullan --modul mimari` izleri Mimari modülünde oynatır. `snapshot --sekme` CAD
  sekmelerine ulaşır.
- **Kabul izleri:** aracı şeritten başlatır, kenetleme ve yardımları açık tutar. Bugün araçlar kimlikle başlıyor ve
  yardımlar kapalı.
- **Sayaç:** izden etkileşim sayacı raporu üretilir (tık, tuş, yazma, odak, araç, menü).
- **Elle kabul:** her faz sonunda sahibin gerçek pencerede deneyeceği kısa liste.
- **Resim matrisi:** 1440 ve 1100 px × 13 ve 18 px × açık ve koyu.

**İlk kabul:** 2+1 daire gerçek pencerede, şeritten ve kenetle çizilir; sayaç taban değeri yazar (147 etkileşim, 177,6 s;
yapıldı).

## 5. Uygulama sırası (M1…M51)

Her satır bir ADR, bir commit, sahibe gönderilen resimler ve bir rapordur (§6). Sıra, sahibin önceliğini izler: önce
çizilen iş kaybolmasın ve ölçülsün; sonra çizim mimari görünsün; sonra hızlı çizilsin; ardından Türkiye'ye uygun proje,
yapı modeli ve değişim gelir. Her fazın sonunda kapı (§7) koşulur.

### Faz 0 — Güvenli ve ölçülebilir

| Sıra | İş | Paket | Görünen sonuç |
|---|---|---|---|
| M1 | Ölçüm altyapısı | ADV-26 | **(M1, ADR 8000: gerçek pencere düzeneği ve 2+1 tabanı 147 etkileşim / 177,6 s yapıldı; `kullan --modul`, `snapshot --sekme` kaldı)** |
| M2 | Esc taslağı korur | ADV-08 | Esc'de duvar zinciri kalır; bildirim **(M2, ADR 8001: yapıldı)** |
| M3 | Takma adlar ve kısayollar | ADV-08 | "pencere" → pencere aracı; iki harfli adlar; F1'de mimari **(M3, ADR 8002: yapıldı; tek tuş kısayolu sahibin kararına kaldı)** |
| M4 | Terim sözlüğü, seçenek harfleri, "Açı" | ADV-08 | Tek ad, tek harf; yanlış harf aracı bırakmaz **(M4, ADR 8003: yapıldı)** |
| M5 | Türkçe komut geçmişi | ADV-08 | Kimliksiz, okunur özet **(M5, ADR 8004: yapıldı)** |
| M6 | Öznitelikler önceliği ve sekme taşması | ADV-09 | Duvar alanları üstte; 1100 px'te bağlam sekmesi görünür **(M6, ADR 8005: yapıldı)** |
| M7 | Modalsız mimari düzenleyiciler | ADV-09 | Kaydedilmemiş değişiklik sorusu; Çizimden seç **(M7, ADR 8006: soru, Çizimden seç ve Esc yapıldı; yüzer/dok pencere kaldı)** |

### Faz 1 — Çizim mimari görünsün

| Sıra | İş | Paket | Görünen sonuç |
|---|---|---|---|
| M8 | Mimari başlangıç ve varsayılan duvar stilleri | ADV-13 | İlk duvar poşeli ve Zemin katta; şablon m ve 1:100 **(M8, ADR 8007: yapıldı; Mimari sekmesiyle açılış M15'ten alındı; malzeme bağı ve örnek proje kaldı)** |
| M9 | Katman anahtarı (EK-4) | ADV-14 | 2+1 örneği türlerine göre katmanlarda; DXF'te aynısı **(M9, ADR 8008: yapıldı; 54 nesne 9 EK-4 katmanında; profil ve blok katmanı kaldı)** |
| M10 | Kalem hiyerarşisi | ADV-14 | Kesilen kalın, görünen ince; ekranda çizgi kalınlığı **(M10, ADR 8009: yapıldı; yay, cam ve malzeme sınırı yarım kalem; 1:50 PDF'te ölçüldü; ölçeğe göre çizgi grubu M11–M12'ye)** |
| M11 | Kapı ve pencere ölçeğe göre | ADV-15 | 1:100 ile 1:50 paftalar farklı **(M11, ADR 8010: yapıldı; kapı ve pencere çizgisi 1:200'de 2, 1:100'de 5, 1:50'de 9; 1:20 kasa profili kaldı)** |
| M12 | Kolon, merdiven, döşeme, mobilya, tarama ölçeğe göre | ADV-15 | Aynı kat üç ölçekte **(M12, ADR 8011: yapıldı; 1:200 sade, 1:50'de kolon ölçüsü ve merdiven bilgisi; kolonun beton taraması M14'e, çizgi grubu sahibin kararına)** |
| M13 | Kesme yüksekliği ve üstteki çizgiler | ADV-15 | Kesikli izdüşümler **(M13, ADR 8012: yapıldı; yüksek pencere kesikli, kesme yüksekliği proje ayarı (yeni projede 1,20), üst katın döşemesi alt katta kesikli; kiriş ve asma tavan nesnesi yok)** |
| M13+ | Ölçeğe göre çizgi grubu (sahibin onayı) | ADV-15 | Paftanın ölçeğine göre kalem **(ADR 8013: yapıldı; PDF'te duvar 1:200/1:100/1:50/1:20'de 0,25/0,35/0,5/0,7 mm)** |
| M14 | Tarama kitaplığı ve yalıtım simgesi | ADV-15 | 1:20 detayda yalıtım **(M14, ADR 8014: yapıldı; TS 5319 SN46–SN58 kitaplıkta, yalıtım SN55 zikzağı katmana sığar, kolon 1:20'de SN46 ve kolon aracıyla dolu; perdenin 1:20 taraması kaldı)** |
| M15 | Mimari şeridin yeniden düzeni | ADV-09 | Büyük ana araçlar; ≤ 30 düğme (Mimari sekmesiyle açılış M8'de yapıldı) **(M15, ADR 8015: yapıldı; 27 düğme, altı büyük, Kat ▾; Oda büyük düğmesi M21'de)** |
| M16 | Mimari ikon seti | ADV-10 | Paylaşılan ikon yok; ikon turu; ikonlar sahibe **(M16, ADR 8016: yapıldı; 10 yeni ikon, 7 çizelge, 3 bekçi, 15 tur sahnesi)** |
| M17 | Bağlamsal sekmeler, sağ tık, kart | ADV-09 | Duvar sekmesinde Duvarı kaydır; mimari kart **(M17, ADR 8017: yapıldı; sağ tıkın ilk bölümü bina paneli, kart `arch_value`'dan, Mahal sekmesi üç panel, dört Türkçe düzeltme)** |

### Faz 2 — Hızlı çizilsin

| Sıra | İş | Paket | Görünen sonuç |
|---|---|---|---|
| M18 | İmleç yanında aracın seçenekleri | ADV-11 | Kalınlık, referans ve uzunluk Tab ile **(M18, ADR 8018: yapıldı; Duvar Kalınlık/Hiza, Dikdörtgen duvar Boyutlar/Kalınlık/Boy referansı; ↓ ile açılış)** |
| M19 | Tıklanabilir geçici ölçü | ADV-11 | Ölçüye yazılan değer duvarı taşır **(M19, ADR 8019: yazılan değerin taşıması ADR 0710'dan vardı; yeni olan sayının altındaki + ile iki yüze bağlı kalıcı ölçü)** |
| M20 | Mimari kenet türleri ve kenet dili | ADV-11 | Yüz, eksen, çekirdek; imleç yanında yazı **(M20, ADR 8020: yapıldı; ad noktadan bulunur, kenet değişmez; süzgeç sonraya)** |
| M21 | Oda aracı | ADV-11 | Duvar + mahal + etiket tek adımda **(M21, ADR 8021: yapıldı; dikdörtgen oda, tek geri alma, mahal adı açık seçim; çokgen oda M22'yle)** |
| M22 | Kendiliğinden mahal ve canlı alan | ADV-11 | Döngü kapanınca mahal; sürüklerken alan **(M22, ADR 8022: yapıldı; Kapanınca mahal M ile, canlı döngü alanı; çift tık ve hedef farkı sonraya)** |
| M23 | Odayı sürükleyerek büyütme | ADV-11, ADV-01 | Mahal kenarından **(M23, ADR 8023: yapıldı; oda döşemesinden seçilir, kenar tutamacı Duvarı kaydır'ın planıyla, yazılan iç ölçü)** |
| M24 | Donatı galerisi ve akıllı yerleşim | ADV-12 | "lavabo" → duvara yaslı yerleşim **(M24, ADR 8024: yapıldı; menü aramasında donatı ve eş anlamlılar, Mimari'de Duvara hizala açık, TEFR katmanları; 3 eylem)** |
| M25 | Kullanım alanları ve çakışma | ADV-12 | Kapı önü ve klozet önü bulguları **(M25, ADR 8025: yapıldı; Donatılar grubu, kanat ve donatı bulguları, kırmızı alan, sahte çarpışma düzeltildi)** |
| M26 | Dört cephe dış ölçü ve iç zincirler | ADV-16 | Tek komut; 3/4 dış, 2+2 iç zincir **(M26, ADR 8026: yapıldı; Planı ölçülendir, dış duvarlar kendiliğinden, MO zincirleri, paylaşılan iç kesenler)** |
| M27 | Kot, mahal etiketi, çakışmasız yerleşim | ADV-16 | Elips etiket, EK-3 numarası **(M27, ADR 8027: yapıldı; Mahal numarala EK-3 ile kat kodu ve saat yönü, Elips içinde numara çerçevesi, KCAD 219. Kot ve çakışmasız yerleşim kaldı: kapı etiketi M28'de)** |
| M28 | 90/210, doğrama kodları, odaya göre numara | ADV-16, ADV-03 | K01, P01 **(M28, ADR 8028: yapıldı; Doğrama kodları tipe kod verir, etiket kod ve 90/210, çakışmasız yer, çizelgede Kod sütunu; Açıklıkları numarala › Mahale göre)** |

### Faz 3 — Türkiye'ye uygun proje

| Sıra | İş | Paket | Görünen sonuç |
|---|---|---|---|
| M29 | Türk alan motoru ve metrekare cetveli | ADV-22 | PAİY 4 alanları çizelgede **(M29, ADR 8029: yapıldı; bölüm çizelgesinde BB net, brüt, ortak alan payı, genel brüt; Metrekare cetveli kat kat)** |
| M30 | Emsal/TAKS ve emsal harici | ADV-22 | Canlı rozet ve tablo **(M30, ADR 8030: yapıldı; imar durumu Proje ayarlarında, durum çubuğu rozeti ve Öznitelikler, Emsal ve TAKS tablosu)** |
| M31 | Bağımsız bölüm, eklenti, ortak yer, kat irtifakı | ADV-22 | Paylaşım tablosu **(M31, ADR 8031: yapıldı; eklenti, nitelik, değer katsayısı, arsa payı; Kat irtifakı tablosu)** |
| M32 | Vaziyet planı | ADV-22 | Parsel, bahçe mesafeleri, oturum, ±0,00 **(M32, ADR 8032: yapıldı; parsel, kenar türleri, canlı yaklaşma sınırı, oturum mesafeleri ve ölçüleri, ağaç, ±0,00)** |
| M33 | Kural motoru; piyes, kat ve kapı denetimi | ADV-23 | Madde ve ölçüyle bulgu **(M33, ADR 8033: yapıldı; Denetim › Mevzuat: PAİY 29/1–3 piyes ve koridor, 28/1-c kat yüksekliği, 39/1 kapı; bulgu maddesini ve RG'yi taşır)** |
| M34 | Merdiven, rampa, korkuluk, ışıklık, asansör denetimi | ADV-23 | Sivas ile PAİY çelişkisi görünür **(M34, ADR 8034: yapıldı; PAİY 30/12, 31, 32/2–3, 34/1–2, 38/1; yerel yönetmelik Sivas: sıkı değer bağlar, gevşeğin tuttuğu ölçü çelişki)** |
| M35 | Yangın kaçışı | ADV-23 | Kaçış rotası ve çıkış genişliği **(M35, ADR 8035: yapıldı; kaçış yolları motoru; BYKHY 32/4, 32/6, 4/i, 33/1–2, 33/6-b, 39/2, 48/1–5, Ek-5/A–B; yağmurlama ayarı; Proje ▾ › Kaçış yolları rotayı ölçüsü ve sınırıyla plana çizer, aşan kırmızı)** |
| M36 | Otopark ve sığınak hesapları | ADV-22 | Çizelgeler **(M36, ADR 8036: yapıldı; Metraj ▾ › Otopark hesabı ve Sığınak hesabı canlı tabloları; Otopark Yön. 4/1-ç, 5/1-j, 8/3–4, Ek-1; Sığınak Yön. 7/a–c, 8/a, 8/c; eksik yer ve alan Denetim › Mevzuat'ta)** |
| M37 | Türk anteti, pafta takımları, PDF/A | ADV-24 | Ruhsat takımı tek komutla **(M37, ADR 8037: PDF/A-2b ve kendi denetimi; MO anteti, künye çizimde, 57/10 tablosu çizimden; Proje › Ruhsat takımı: bilgi, vaziyet, kat planları ve hesap paftası, EK-2 adlı PDF/A yayın; yapıldı)** |
| M38 | Mahal listesi ve grafik doğrama listesi | ADV-24 | 57(29) listesi; tip görünüşleri **(M38, ADR 8038: mahalde denizlik, basamak ve korkuluk kaplaması; Çizelge ▾ › Mahal listesi kat kat, kapı ve pencere kodlarıyla; takımda LS/MHL paftası; tipte kasa-kanat ve U değeri, TS 825:2024 denetimi; Etiket ▾ › Doğrama listesi: Türkiye'nin açılım çizgisiyle görünüşler, canlı blok; takımda 1/50 LS/MMC paftası; yapıldı)** |
| M39 | Tadilat grafik dili | ADV-17 | Üç tadilat paftası **(M39, ADR 8039: "Evre" → Tadilat durumu (mevcut, iptal, ilave); açıklığın kendi tadilat durumu, görünüme göre birleşim ve kesim; araçlar görünümün durumunu yazar; DXF durum katmanları; Tadilat projesi görünümü ve lejantı; Ruhsat takımında Tadilat projesi: her kat mevcut durum, tadilat projesi, yeni durum paftası, kendi adlı görünümüne bağlı; yapıldı)** |

### Faz 4 — Yapı modeli ve türetilmiş çizimler

| Sıra | İş | Paket | Görünen sonuç |
|---|---|---|---|
| M40 | Düşey model | ADV-18 | Duvar tepeleri kata bağlı; döşeme katmanları **(M40, ADR 8040: duvar betonarmeden betonarmeye, katın yüksekliğini geçmez; kat yüksekliği değişince üstteki katlar ve duvar tepeleri izler; döşemenin kaplama, şap, yalıtım ve sıva katmanları, metrajı katman katman, `.kcad` 232; mahalin döşeme kot farkı, asma tavanı ve iç yüksekliği, PAİY 28/4–6 denetimi, `.kcad` 233; yapıldı)** |
| M41 | Kiriş, lento, kolon–duvar birleşimi | ADV-18 | Kesikli kiriş; sürekli tarama **(M41, ADR 8041: kiriş nesnesi, planda kesikli ve ince, 1:100'de numarası, 1:50'den "K101 25/50"; duvar kirişin altında biter; Mimari › Kiriş aracı, numara K101 → K102; `.kcad` 234; kolonun malzemesi, duvar kolonun yüzünde kesilir, kolon kendi malzemesindeki duvar katmanıyla tek kesit: perde + kolon kesintisiz, tuğla kolonda biter; etiketin `{size}`'ı "30/50"; `.kcad` 235; kirişin türü (lento, hatıl) ve kattan yüksekliği, lento açıklığa tıklanarak (TBDY 11.5.3), Metraj › Kirişler; `.kcad` 236; yapıldı)** |
| M42 | Kesit türetimi | ADV-19 | 1:50 kesit; eskime işareti **(M42 1. bölüm, ADR 8042: Kesit çıkar kesit işaretinden 1/50 kesiti işaretin bloğuna çizer — kesilen taralı, görünen derinlikle, merdiven, kotlar, akslar, mahaller; "B-B kesiti eski" ve Yeniden çiz; kesit çizgisi noktalı kesikli; 2. bölüm, ADR 8042 §6–§7: Ruhsat takımında KS paftası ve balonda pafta numarası, yayında eski kesit uyarısı, Mevzuat'ta PAİY 57/2-a-6; yapıldı)** |
| M43 | Görünüş türetimi | ADV-19 | Dört cephe **(1. bölüm, ADR 8043: Görünüş çıkar görünüş işaretinden 1/50 cepheyi işaretin bloğuna çizer — kapı ve pencereler açılım çizgileriyle, zeminin altı kesik, kuzey okuna göre adı; eskime ve Yeniden çiz; Ruhsat takımında GR paftası; yapıldı. 2. bölüm: ek yeri çizilmez, Tümü ile dört görünüş birden, Mevzuat'ta görünüşü olmayan cephe, cephe kaplaması yazısı; yapıldı)** |
| M44 | Çatı ve çatı planı | ADV-20 | Çatı planı paftası **(1. bölüm, ADR 8044: saçak çizgisinden ve kenar eğimlerinden yüzler, mahya, kırma, dere — çekirdek ve bağımsız başvuru; çatı nesnesi, saçağı katın üstünde, `.kcad` 237; planda saçak, kalkan, ince mahya ve kırmalar, her yüze ok ve eğim, mahya kotu; Mimari › Çatı: Binadan ya da Köşeleri çiz, Kırma, Beşik, Tek eğimli, Düz; PAİY 40 mahya uyarısı; yapıldı. 2. bölüm: kesit ve görünüşte çatı — eğik bant ağaç taramalı, saçak alnı, kırma ve mahya görünür, “%33”, “+6.00 Saçak”, “+7.78 Mahya”; çatı duvara oturur; yapıldı. 3. bölüm: baca ve çatı penceresi — çatının parçaları (`.kcad` 238), duman bacası mahyanın 0,80 m, hava bacası çatının 0,60 m üstünde, pencere yüzün düzleminde; Çatı ▾: Çatı bacası, Çatı penceresi; kesitte taramalı baca ve "+8.49 Baca"; çatı `cad.arch.edit` ile düzenlenir; yapıldı. 4. bölüm: çatı planı paftası — Ruhsat takımında "Çatı planı" (EK-2 PL/CAT/NNN), haritası bütün çatılar ve katsız nesneler, PDF/A basılır; yapıldı. 5. bölüm, ilk adım: oluk ve kar siperliği — eğimli saçaklar boyunca, planda oluk dışta, kar siperliği içte çentikli (Sivas 42/5), `.kcad` 239, Çatı aracında O ve K; yapıldı. İkinci adım: iniş borusu — oluğa bağlı, planda gerçek boyunda Ø100, `.kcad` 240, Çatı ▾ İniş borusu; yapıldı. Üçüncü adım: görünüşte ve kesitte oluk (yarım yuvarlak, saçak kotunda), iniş borusu (45° kaz boynuyla duvara, oradan ±0.00'a) ve kar siperliği çizgisi; yapıldı. Dördüncü adım: Denetim › Mevzuat'ta çatı kuralları — PAİY 40/4, 40/5, 42/1, BYKHY 58/2, Sivas 40/5-e, 40/5-g, 42/5 — ve düzeltmeleri; yapıldı. Sonraya (ADR 8044): içe dönen saçak, çatı katmanları, çatı bacasının Baca aracıyla bağı)** |
| M45 | Merdiven ve özel elemanlar | ADV-21 | Balansman, asansör, korkuluk, otopark yeri **(1. bölüm yapıldı, ADR 8045 §1–2: balansmanlı merdiven — L ve U'da dönüşün iki yanında dengelenen basamak, iç kenarda doğrusal daralma, planda dönen basamak çizgileri, `.kcad` 241, Merdiven aracında A, Öznitelikler, PAİY 31/2-c denetimi ve düzeltmesi; kesit ve görünüşte tek tek dengelenmiş basamaklar. 2. bölüm ilk adımı yapıldı, ADR 8045 §3: bağımsız korkuluk — `.kcad` 242, `cad.railings.create`, planda iki ince çizgi, Merdiven ▾ › Korkuluk (Çiz, Boşluktan: galeri boşluğunun serbest kenarları, merdiven çıkışı açık), Öznitelikler ve PAİY 38 sözü; kesit ve görünüşte küpeşte ve çubuklar; Mevzuat'ta PAİY 38/1, 38/4 — alçak korkuluk, geniş aralık, korkuluksuz boşluk, balkon ve teras kenarı. 3. bölüm yapıldı, ADR 8045 §5: asansör kitaplığın değiştirilebilir bloğu — kabin, karşı ağırlık, sürme kapı; Kabin eni, derinliği, Kapı genişliği ve Tip (Konut, Sedye, TSE Tip 2); MM-ASAN-GENEL; donatı aramasında; Mevzuat'ta blok asansör sayılır, kuyu ve içindeki kabin PAİY 34/2 ile, 10 kat ve fazlasında sedye asansörü 34/5 ile, düzeltmesi bloğun parametreleri. 4. bölüm yapıldı, ADR 8045 §6: otopark yeri kitaplığın bloğu — En, Boy, Tür (Binek, Engelli, Şarjlı, Kamyonet), Ölçü tablosu, numara; MM-TEFR-OTOPARK; Otopark hesabı çizilen yerleri sayar; Mevzuat'ta Otopark Yön. 5/1-h-8, 9 ölçü, 4/1-ç manevra alanı, 4/1-ı engelli yeri, 5/1-j şarj yeri. Kalan: 34/4'ün iki asansörü, kabinin kesitte görünmesi, bir sıra yeri tek seferde çizen araç, manevra yolu ve rampa ölçüleri)** |

### Faz 5 — Değişim ve kalanlar

| Sıra | İş | Paket | Görünen sonuç |
|---|---|---|---|
| M46 | DWG/DXF katman profili | ADV-25 | Dış programda aynı katmanlar ve yazılar **(1. bölüm yapıldı, ADR 8046 §1: mahal DXF'te ekrandaki gibi — sınırı, boşluğun çaprazı ve etiketsiz mahalin yazısı (numara, ad, alan) MM-XXXX-MAHAL'de, etiketli mahalin yazısı etiketinden; DWG aynı DXF'ten. Dış okuyucu ezdxf: önce 5 çizgi 0 yazı, sonra 8 yazı ve çapraz. 2. bölüm yapıldı, ADR 8046 §2: Standard (Arial) yazının genişlik çarpanı satır satır Barlow/Arial — 2+1'de fark −%3,6…+%29'dan ±%0,06'ya. 3. bölümün ilk adımı yapıldı, ADR 8046 §3: başka programlar TrueType yüksekliğini büyük harf yüksekliği okuyor (Tekla, BricsCAD, QCAD, ezdxf) — TEXT, öznitelik, tablo hücresi, eğri yazı ve stil yüksekliği `em × yüz tipinin büyük harf oranı` (Arial 1466/2048), okurken bölünür; 2+1'de 133 TEXT. İkinci adım yapıldı: MTEXT (satır aralığı ve girintiler de oranla) ve kılavuz. Üçüncü adım yapıldı: ölçü yazısı (DIMTXT, bloğun MTEXT'i), tablo stili, MULTILEADER; ezdxf'te 2+1'in etiketleri ve çizelgeleri artık taşmıyor. Dördüncü adım: dikey hizalı yazının taban çizgisi iki programda aynı; bitti)** |
| M47 | IFC4 mimari dışa aktarma | ADV-25 | Bağımsız okuyucuda sayılar eşit **(1. bölüm yapıldı, ADR 8047: IFC'nin ayrı paketi `kentos-ifc` (kullanıcının kuralı, CLAUDE.md), yolun hizalaması da oraya taşındı; bina IFC 4.3'te (EK-5 §5.4) — kat, duvar, kapı, pencere, açıklık, mahal, döşeme, kolon, kiriş, çatı, korkuluk, merdiven; adlar EK-5 §5.5'e göre (MM-DVR-Dış_duvar_25, MM-KAP-K01), EK-6'dan çizimin bildikleri (dışarıda, taşıyıcı, malzeme, net alanlar, emsal ve TAKS); GlobalId nesnenin kimliği. IfcOpenShell bölüm binasında: 0 doğrulama bulgusu, sayılar eşit (2 kat, 16 duvar, 5 kapı, 9 pencere, 10 mahal); ilk koşu 8 iç duvarın "dış" işaretlendiğini buldu, düzeltildi. 2. bölüm yapıldı, ADR 8047 §6: komut `cad.arch.ifc` (MCP ve Python'da da) ve Dosya › Dışa aktar › Bina modeli › "IFC dışa aktar (bina)…"; yazılamayan nesne ve rampa uyarıyla söylenir; 22 ortak durum; gerçek pencerede GTK kayıt penceresiyle yazılan dosya IfcOpenShell'de 0 bulgu; başlık DesignTransferView. 3. bölümün ilk adımı yapıldı, ADR 8047 §7: dosya adı EK-2'ye göre (260123_00_MM_XXXX_MD_XXX_NNN_01_000.ifc), EK-7'nin projesi, mimarı (sicil), bürosu (vergi no) ve rolü, EK-6'nın adresi, ada/parseli, TAKS/KAKS/çekmeleri ve bina kimliği künyeden; künyeye büro adı ve vergi no eklendi. İkinci adım yapıldı, ADR 8047 §8: koordinat sistemi EK-7'ye göre (TUREF dilimi, datum, Transverse Mercator, dilim, düşey datum; harita dönüşümü ±0.00'ın kotuyla), yolun IFC'si de aynı fonksiyondan. Üçüncü adım yapıldı, ADR 8047 §9: EK-9'un 21 sorusu modelden cevaplanıyor (`kentos_ifc::quality`); Mimari denetim › Mevzuat'ta Dijital Proje Yön. bulguları — proje bilgileri (Md. 13), proje kodu (Md. 6), adsız mahal (Md. 10), koordinat sistemi (EK-7), mahalsiz model, modele girmeyen nesne (Md. 9). 4. bölüm yapıldı, ADR 8047 §10: Mimari › Proje ▾ › Model kalite kontrol formu (EK-9) çizime yerleştirir; Ruhsat takımı onu yeniden cevaplayıp kendi paftasına koyar (EK-2 FR/MKF, A2); künye penceresinde büro adı ve vergi no. Açık: açıklığı gövdeden kesen Reference View çıktısı; bitti)** |
| M48 | Eski DWG planından tanıma | ADV-25 | Patlatılmış plandan duvar ve açıklık |
| M49 | Ölçülü bölmede kalıcı bağ; bağ kapsamı | ADV-01 | Yay kenar, kolon, aks bağı |
| M50 | Modül kapsamı | ADV-05 | Kolon, merdiven, döşeme, mutfak, eğri duvar modülde |
| M51 | Büyük projede akıcılık | ADV-07 | 163 → < 100 ms |

## 6. Bir maddeyi baştan sona yapma talimatı (mimari)

Sahibin 7 Kasım 2026 isteğiyle her madde şu sırayla yapılır. Hiçbir adım atlanmaz.

1. **Seç:** §5'te sıradaki ilk açık M satırını al. Atlanan satırın nedeni satıra yazılır.
2. **Araştır:** konunun güncel resmî belgelerini (üreticiler) ve Türkiye mevzuatını oku. Araştırma notları başlangıçtır,
   iddiaları yeniden doğrulanır. Değer uydurulmaz.
3. **Başka uygulamaların resimleri:** aynı işi yapan en az iki uygulamanın resmî belgelerindeki ekran görüntülerini
   indir. Yer: `.run/gui-evidence/mimari-mNN/referans/`; her biri kendi boş dizinine (güvenilmeyen veri). Kaynak adresiyle
   sahibe gönder.
4. **ADR:** mimari blokta sıradaki numara. Mimari ileri planın ADR'leri **3000'den başlar** (Yol 1042'den, Harita 1800'den
   20'şer ilerliyor; çakışmasın diye). Bölümler: Sorun, Araştırma (kaynaklar ve referans resimler), Karar, Ölçüm.
5. **Çekirdek ve sözleşme:** hesap `crates/shared/*`'ta, türler `kentos-contracts`'ta. Kalıcı yeni alan yeni KCAD şeması
   ister. Dalda geçici numara kullanılır (sıradaki 205); kesin numara birleşme sırasıyla verilir (MADDE-TARIFI §14).
6. **Komut:** araç = komut (kılavuz §9.2); katalog, Python, MCP ve manifest.
7. **Bağımsız başvuru:** `scripts/fixtures/*_cases.py` beklenen sonucu KentOS kodu olmadan kurar; en az 20 durum. Bir değeri
   bozup durumun düştüğünü göster.
8. **Arayüz:**
   - Düzeltme ortak `kentos_ui` bileşeninde yapılır.
   - Her araç şu kapılardan geçer: kendi ikonu, kısa takma ad, şeritteki yerin gerekçesi, terim sözlüğü, seçenek harfi
     sözleşmesi (ADV-08, ADV-10).
9. **İkon:** yeni ya da değişen her ikon çizilir, 16 ve 20 px'te iki temada resimlenir, sahibe gönderilir.
10. **Sına:** ilgili ortak durumlar ve masaüstü testleri; şema değiştiyse `kentos-kcad`; clippy. Ağır işler
    `scripts/dev/svc.sh heavy` altında koşar.
11. **Resim ve gerçek pencere kabulü (ADR 8000):**
    - Değişen iş, **gerçek pencerede** kullanıcı gibi sınanır: `scripts/usage/real/session.sh` ile, sahibin ayarlarının
      kopyasıyla, şeritten, kenetleme açık. Resimler buradan alınır, sayaç eski değerle karşılaştırılır.
    - Kod içi resimler (`kentos-cad kullan`, `snapshot`) ek güvencedir; yalnız onlarla kabul yapılmaz.
    - Resme bakılır, kusur düzeltilir, yeniden çekilir.
    - Referans resimlerle yan yana sahibe gönderilir.
12. **Planı işaretle:** satıra `(MNN, ADR NNNN: …)` yazılır. `[x]` yalnız bütün kabul sağlanınca konur. Ölçü §7'ye yazılır.
13. **Commit ve gönder:** commit iletisi ortak yazar ve oturum satırlarıyla biter. Çalışılan dal (şu an `arch-adv-01`)
    GitHub'a gönderilir.
    `main` yalnız sahip isteyince birleştirilir.
14. **Disk:** `df -h /` denetlenir. Doluysa `target/debug/deps`'teki eski test ikilileri silinir; her adın en yenisi kalır.

## 7. Kabul ölçüleri ve faz kapıları

| Ölçü | Bugün | Hedef | Faz |
|---|---|---|---|
| Esc'de kaybolan duvar parçası | bütün zincir → **0 (M2)** | 0 | 0 |
| Kısayolu ya da kısa adı olan mimari komut | 0 → **13 iki harfli ad (M3)** | en sık araçlara tek tuş (sahibin kararı) | 0 |
| Aynı harfe istemde ve panelde farklı ad | 49 → **0 (M4)** | 0 | 0 |
| İstemlerde birden çok harfte duran ad | 12 → **0 (M4)** | 0 | 0 |
| Komut geçmişinde kimlik ya da İngilizce alan adı (Mimari komutları) | her satırda → **0 (M5)** | 0 | 0 |
| İlk poşeli duvara eylem | ölçülecek (M1) | ≤ 3 | 1 |
| 2+1 örneğinde katman | 1 | türlerine göre (EK-4) | 1 |
| Aynı ikonu paylaşan komut | 10 | 0 | 1 |
| Görünen şerit düğmesi | 78 komut; 4 büyük + Proje ▾ | ≤ 30, ana araçlar (Mahal, Oda, Donatı dahil) büyük | 1 |
| Ölçeğe göre ayrıntısı olan nesne türü | 1 | ≥ 6 | 1 |
| 2+1 gerçek pencerede etkileşim | 147 | ≤ 80 | 2 |
| 2+1 gerçek pencerede KLM süresi | 177,6 s | ≤ 100 s | 2 |
| Ruhsat takımı | yok | tek komut, PDF/A | 3 |
| 100 daire + 200 çizelgede kare | 163 ms | < 100 ms | 5 |

**Faz kapısı:**
- Kabul izleri Mimari modülünde, şeritten ve kenetle oynar; sayaç raporu önceki fazla karşılaştırılır.
- Resim matrisi çekilir ve sahibe gönderilir.
- Sahip kısa listeyi gerçek pencerede dener.
- Fazın ADR'lerindeki "elle denenmedi" notları kapanır.

## 8. Kurallar

- **Değişikliğin sınırları:**
  - Her alt teslimin ürüne geçişinde KCAD uyumu, kilitli katman, belge revizyonu, bulut değişiklik paketi ve yetki
    yeniden kontrol edilir.
  - İlgisiz renderer, birim ya da etkileşim ailesi topluca yeniden yazılmaz.
  - Yeni davranış eski belgede varsayılan olarak yeni bağ, ölçek ya da katman üretmez.
- **Profillerin ayrılığı:** mimari çizim kuralları, kurum ve ofis standardı ve mevzuat denetimi ayrı profillerdir.
  - Standart adı ve sürümü, kapsam, birim ve yorum kaynakları kaydedilmeden "standarda tam uygun" ya da "ruhsata uygun"
    iddiası yapılmaz.
  - Mevzuat profili için uzman ya da kurum kabulü ayrıca gerekir. Bu plan otomatik hukuki ya da proje onayı tanımlamaz.
- **Belgeler ve resimler:** ham ekran görüntüleri ve indirilen referans resimler Git'e girmez; `.run/gui-evidence/`
  altında kalır. Repoda kısa sonuç, kaynak commit'i, yeniden üretim adımı ve gerekli küçük örnek model bulunur.
- **Kabulün anlamı:** bu planın kabulü uygulamanın üretim kabulü değildir.
