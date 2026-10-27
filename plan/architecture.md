# 2B mimari üretim — araştırma, araçlar ve uygulama planı

Tarih: 24 Ekim 2026. Başlangıç: `main` / `bb1f3ffd0`.
Çalışma dalı: `codex/architecture-2d-workflows`.

27 Ekim 2026: mimari dal `3ad0decc7` ile main'e birleşti; çalışma **main**
üzerinde sürer. Her yeni adımda resmî üretici belgeleri araştırılır; kullanım
akışı görünür ayar, anlaşılır önizleme ve az işlem hedefiyle tasarlanır.
Gerçek GUI kontrolü sürer; ham ekran görüntüleri yalnız `.run/gui-evidence/`
altında yerel tutulur ve Git'e eklenmez. Metin sonuçları, model karşılaştırmaları
ve gerekli küçük yeniden üretilebilir fixture'lar repoda kalır.

Amaç: bir kullanıcının konut kat planını ölçülü ve rahat çizebilmesi; duvar,
açıklık, mahal, ölçü, etiket, çizelge ve paftanın düzenleme boyunca birlikte
kalması. Bu belge özellik sayısını artırma listesi değildir. Mevcut yeteneğin
bulunabilirliği, çizim hızı ve hatayı açıklaması da kabul kapsamındadır.

Durum işaretleri: `[ ]` yapılacak/doğrulanacak, `[~]` sürüyor, `[x]` kanıtlarıyla
kabul edildi. Kaynak kodun varlığı GUI kabulü değildir. Araştırılan ticari
uygulamalar yerelde çalıştırılmış sayılmaz; üretici kılavuzları referanstır.

## 1. Korunacak güncel taban

- Kullanıcı başka bilgisayarda geliştirmiş ve güncel kodu çekmiştir. Önceki
  oturumun değişiklikleri veya stash'i bu dala geri uygulanmayacak.
- [ADR 0626](../docs/adr/0626-units-scale-and-true-size-texts.md) güncel kullanıcı
  kararıdır: Mimari proje **metre / 1:100**, tek ad **Çizim ölçeği**, işe göre
  sihirbaz; mevcut `opened_scale`, gerçek boy, 3 px altı yazı şeridi, pan/zoom
  yerleşim ve resim önbelleği korunur. Mimari araç işi bu sistemleri değiştirmez.
- Önceki [mimari inceleme](../docs/cad/architecture-audit.md) araştırma kaynağıdır.
  Oradaki eski birim/ölçek önerileri ADR 0626'nın yerine geçmez. Örneğin yeni CAD'i
  mm/1:1'e döndürmek veya kullanıcının kabul ettiği açılış ölçeği kuralını kaldırmak
  bu planın işi değildir.
- CAD/CBS proje ayrımı, seçim/kenet, katman tipi/kilidi, serbest Duvar, mevcut
  Dikdörtgen, Açıklık, Mahal ve ölçü araçları yerinde kalır. Yeni iş ayrı komut ve
  araç olarak eklenir; çalışan araç ailesi topluca yeniden yazılmaz.
- Kaynak DWG/DXF/NCZ, özgün kullanıcı projesi ve kaydedilmiş tercih korunur.
  Testler ayrı örnek dosyalarda yapılır; içe alma veya şablon adına mevcut çizim
  otomatik yeniden ölçeklenmez.
- Web arayüzü ADR 0228 gereği bekler. İş masaüstü, ortak hesap, native uygulama,
  Python ve MCP'dedir. Gerekli ortak tür/katalog üretimi sürer; web'de ertelenen
  görünüm TODOS.md §0.2'ye yazılır.

## 2. Başlangıç envanteri ve ilk bulgular (`bb1f3ffd0`)

Başlangıçta `catalog/v1/ribbon.json` Mimari sekmesi sekiz panel içeriyordu.
Aşağıdaki tablo ilk uygulama öncesini kaydeder; eklenen araçlar çalışma günlüğündedir:

| Panel | Görünür araçlar | Açılır menüdeki mevcut işler |
| --- | --- | --- |
| Duvar | Duvar, Çizgiden duvar, Birleşim, Duvar stilleri | Böl, birleştir, uzat, buda, çevir, stil değiştir, birleşim denetimi |
| Açıklık | Kapı, Pencere, Boşluk | Çevir, taşı, numarala |
| Mahal | Mahal, Metraj | Böl, birleştir, döşeme/tavan kaplaması |
| Taşıyıcı | Aks, Kolon, Aks numarala | — |
| Merdiven | Merdiven, Rampa | — |
| Görünüm | Kesit, Görünüş, Detay işareti | Referanslar, pafta takımı |
| Etiket ve çizelge | Etiket, Toplu etiket, Açıklık çizelgesi, Mahal çizelgesi | Etiket denetimi |
| Proje | Mimari ayarlar, Şablon, Evre | — |

İlk kod incelemesinde:

- Araç sayısının az görünmesinin bir nedeni temel düzenleme işlerinin overflow'a
  gömülmesi. Bunları yeniden yazmak yerine bağlamında görünür kılmak gerekir.
- Duvar/açıklık/mahal parametreleri çoğunlukla alt komut isteminin seçeneklerinde.
  Kullanıcı stil, kalınlık, referans yüz ve yerleşim mesafesini tıklamadan önce
  okuyabilmeli; seçenek harflerini ezberlemek zorunda kalmamalı.
- Mevcut serbest Duvar ve Dikdörtgen var; iki köşeyle **anlamsal duvar çerçevesi**
  oluşturma ayrı bir araç olarak başlangıçta yoktu. Normal alan çizmek bu ihtiyacı karşılamaz.
- `catalog/v1/templates/mimari.json` hâlâ cm/1:50 tanımlıyor. `arch.template`
  doğrudan uygulandığında boş çizimin birimini, dolu çizimin de çizim ölçeğini
  değiştirebiliyor. Yeni m/1:100 başlangıcıyla uyumu ayrıca ele alınmalı; bu
  uyuşmazlık render veya sihirbaz kararlarını geri almak için gerekçe değildir.
- Mevcut mimari nesne, ilişki, canlı çizelge, kat/evre ve tutamaç altyapısı
  kullanılacak. Kodda bulunmaları bu oturumda kusursuz olduklarını kanıtlamaz.

İlgili kaynaklar: `interaction/{wall,path,rectangle,opening_tools,space_tools,
arch_tools,wall_edit_tools}.rs`, `application/{walls_create,walls_edit,
openings_create,openings_edit,spaces_create,spaces_edit,schedules,
templates_apply}.rs`, `properties/rows/arch.rs`, mevcut mimari scene/trace'lar.

## 3. Benzer uygulamalardan alınacak iş akışları

Aşağıdaki sonuçlar üretici belgelerine dayanır. Bir ürünün 3B özelliği, KentOS'ta
2B karşılığı tamamlanmış gibi sunulmaz. Esas soru: kullanıcı hangi işi kaç
adıma, hangi görsel geri bildirimle tamamlıyor?

| Referans | Belgelenen davranış | KentOS kararı |
| --- | --- | --- |
| [Vectorworks 2026 — Creating walls](https://app-help.vectorworks.net/2026/eng/VW2026_Guide/Walls/Creating_walls.htm) | Serbest/yaylı duvarın yanında iki köşeli dikdörtgen modu; görünür yüz/eksen/öteleme ve stil seçenekleri. | Serbest Duvar'ı değiştirmeden Dikdörtgen duvar ekle. Referans anlamını önizlemede göster. |
| [BricsCAD BIM QuickDraw](https://help.bricsys.com/en-us/document/command-reference/b/bimquickdraw-command?version=V27) | Oda boy/en girişi, hizalama ve komşu duvarlarla hızlı yerleşim. | Önce bağımsız 2B dikdörtgen; komşu duvarı benimseme/otomatik silme daha sonraki ayrı kabul. |
| [AutoCAD Architecture — doğrudan düzenleme](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-Architecture/files/GUID-CDA9AF6E-AE7F-47A1-A19A-82EAA0196138.htm) | Sayısal tutamaç düzenlemesi ve nesneye ait eylemler. | Kapı yönünü değiştirme, duvarı kaydırma ve açıklık enini düzeltme görünür eylem olmalı. |
| [AutoCAD Architecture — duvar birleşimi](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Architecture/files/GUID-33F8B759-ECFE-4359-B186-C95C98B30D36.htm) | Birleşim grubu/yarıçapı, referans ve teşhis görünümü. | Mevcut çözücü korunur; sorunlu köşe yerinde gösterilir, onarım önizlenir. |
| [AutoCAD Architecture — AEC ölçüsü](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Architecture/files/GUID-80E8DF2E-1722-4957-BABF-910A0072EF81.htm) | Duvar ve açıklıklardan ilişkili ölçü dizileri. | Cephe/mahal üzerinden açıklık, duvar ve toplam boy için çok sıralı ölçü üretimi. |
| [Archicad — otomatik dış ölçü](https://helpcenter.graphisoft.com/user-guide/77042/) | Toplam boy, dış geometri, yapı ve açıklık dizilerinin seçimi. | Zincir/Baz araçlarından ayrı bir otomatik mimari ölçü işi; hangi yüzü ölçtüğü açık. Referans eski sürüm kılavuzudur. |
| [Archicad — Zones](https://help.graphisoft.com/AC/27/INT/_AC27_Help/040_ElementsVB/040_ElementsVB-137.htm) | Kapalı alandan mahal, referansa göre alan hesabı. | Toplu mahal bulma, sınır tanısı, kaynakla birlikte alan/etiket güncelleme. |
| [AutoCAD Architecture — çizelge](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Architecture/files/GUID-77F8B132-6A45-483B-AB92-AD3BD5D47660.htm) | Nesnenin verisini okuyan etiket ve canlı çizelge. | Mevcut tabloyu kullan; kapı verisini bağımsız metin kopyalarına çoğaltma. |
| [FreeCAD ArchWall / ArchWindow / ArchSpace](https://github.com/FreeCAD/FreeCAD/tree/7f643c369dee071a9fa5058c831572ec48bbdd50/src/Mod/BIM) | Kaynak yol, host ve yeniden hesap ilişkileri açık nesneler. | Mimari nesneler görünüş çizgilerinden ayrı kalmalı; Python aynı anlamsal ilişkiyle çalışmalı. |

Ek doğrudan araç karşılaştırmaları:

- [Archicad 29 — Distance Guides](https://help.graphisoft.com/AC/29/INT/_AC29_Help/030_Interaction/030_Interaction-65.htm):
  yerleştirme/düzenlemede çevredeki elemanlara mesafe ve yüz/referans çizgisi
  seçimi. KentOS karşılığı ARCH-DRAW-02 ve ARCH-EDIT-01; ilk dilim düz kaynak,
  yaylı kaynak ayrıca sınanacak.
- [AutoCAD Architecture 2026 — DOORADD](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Architecture/files/GUID-E891DA26-F25C-4461-BC7A-81779498689A.htm):
  mevcut kapıyı eşleme, menteşe/merkez/kilit referansı ve çoklu yerleşim.
  ARCH-OPEN-01/02'ye aynı tipten ekle ve özelliklerini al yöntemi eklenir.
- [Vectorworks 2026 — Door/window creation modes](https://app-help.vectorworks.net/2026/eng/VW2026_Guide/DoorWindow/Creating_DoorsWindowsAssemblies.htm):
  çizimle en/konum belirleme, mevcut özelliklerle giriş. KentOS'ta host/yön
  önizlemesi ve iki noktayla açıklık eni kendi kabulünü alacak.
- [Archicad 29 — Automatic Interior Dimensioning](https://help.graphisoft.com/AC/29/INT/_AC29_Help/070_Documentation/070_Documentation-50.htm):
  elemanları kesen geçici çizgiyle iç ölçü zinciri. ARCH-DIM-01'e cephe seçimi
  yanında kesen çizgi yöntemi, duvar yüzü ve kolon uç/merkez seçimi eklenir.
- [Vectorworks 2026 — Creating cabinets](https://app-help.vectorworks.net/2026/eng/VW2026_Guide/FurnFixt/Creating_cabinets.htm?agt=index):
  alt/üst/yüksek dolap, en çizerek yerleştirme ve çizgi boyunca çoğaltma.
  ARCH-LIB-02'de mevcut blok sistemiyle dar ve ölçülü bir ilk teslim yapılacak.
- [Archicad 29 — Stair floor-plan display](https://help.graphisoft.com/AC/29/INT/_AC29_Help/040_ElementsVB/040_ElementsVB-202.htm):
  kat bazlı görünür/gizli parça, kırılma işareti, çıkış çizgisi ve numara.
  ARCH-STAIR-01'de 2B plan gösterimi, fotogerçekçi 3B üretimden önce gelir.

Araştırma genişletilirken üreticinin ilgili araç sayfası maddeye eklenir. Tanıtım videosu
veya ekran görüntüsü veri modeli, undo ya da dosya desteği kanıtı sayılmaz.

### 2B çizim odaklı örnekler: QCAD ve LibreCAD

- [QCAD — Offsetting](https://qcad.org/en/tutorials/offsetting/) mesafe/adedi
  görünür seçenek çubuğundan alır; fare tarafına göre önizler ve tek parça/tüm
  konturu ayırır. [LibreCAD 2.2 çizim araçları](https://docs.librecad.org/en/2.2.0_a/ref/tools.html)
  mesafeyle ve noktadan paraleli belgeler. ARCH-DRAW-02/ARCH-UX-01 için sonuç:
  bölmenin net uzaklığı ve hangi yüzün referans alındığı çizimden önce görünmeli.
- [LibreCAD — Blocks](https://docs.librecad.org/en/2.2.0_a/guides/blocks.html)
  çizimdeki blok listesinden bağlı örneklemenin, kütüphaneden tekrar eklemede
  bağımsız tanımlar oluşturulmasından farkını açıklar. ARCH-LIB-01/02'de
  “aynı mobilya tipinin örneği” ile “bağımsız varyant” ayrımı ve gerçek fiziksel
  en/boy önizlemesi belirgin olmalı; mevcut blok modeli korunmalı.
- [QCAD — Layers, Blocks and Attributes](https://qcad.org/en/tutorials/layers/layers_blocks_attributes/)
  By Layer/By Block mirasını renk, çizgi tipi ve kalınlık için anlatır.
  ARCH-UX-03/ARCH-DOC-04'te yeni galeri ve ofis profili, var olan blok/katman
  mirasını gelişigüzel sabit değerlere çevirmemeli.
- [QCAD Professional — Baseline](https://qcad.org/en/manual/scripts/pro/draw/dimension/dimbaseline/doc/dimbaseline/)
  mevcut ölçüden sabit aralıklı sıralar üretir. [LibreCAD — Annotating](https://docs.librecad.org/en/2.2.0_a/guides/annotate.html)
  düz etiketin otomatik değeri durdurabildiğini, `<>` yer tutucusunun değeri
  koruduğunu açıklar. ARCH-DIM-01/ARCH-EDIT-01'de ölçü yazısını gerçek geometri
  yerine kullanmama ve ilişkileri düzenleme boyunca koruma kabulü gerekir.
  Bu sayfalardan tam host ilişkisi garantisi çıkarılmaz.
- Bu da belge araştırmasıdır; uygulamalar burada yerelde çalıştırılmadı.
  QCAD'in Professional kapsamındaki gelişmiş offset/ölçü araçları CE/açık kaynak
  yeteneği olarak sunulmaz. LibreCAD bağlantıları kılavuzun 2.2 sürümüne aittir.

## 4. Ortak araç ve veri sözleşmesi

- Her yeni çizim aracı işi kendi **anlamsal ürün komutuyla** yapar. Kullanıcıdan
  alınan köşe/boy/mesafe/referans komuta gider; geometriyi UI veya AI hesaplamaz.
- `validate`, yazmayan `plan`, atomik `execute`, sabit ret kodu, revision,
  idempotency, işlem kaydı ve geri al yolu birlikte gelir.
- Hesap mevcut ortak/native çekirdekte tek yerde kalır. Önizleme ve sonuç aynı
  girdiyi kullanır. Ekran pikseli kalıcı mimari uzunluk değildir; API SI metre
  sözleşmesini korur, arayüz projenin biriminde gösterir/okur.
- Kapı/pencere host kimliği ve yol konumu; mahal seed/sınır kaynağı; ölçü nesne/
  alt nesne bağı; etiket hedefi; çizelge kaynakları korunur. Çizgiye patlatma
  ancak açık eylem ve kayıp raporuyla olur.
- Yeni araç mevcut dosyalara varsayılan ekleyip geometriyi değiştirmez. Kalıcı
  model gerekirse ayrı şema/migration/round-trip işi açılır.
- Bir işlem bir anlamlı undo adımıdır. Başarısız önizleme, Escape veya sağ tuş
  yeni nokta/nesne yazmaz. Tekrarlı çizim ve Enter davranışı eski araçlarda değişmez.
- Python tipli SDK ve MCP aynı komut/plan/ret/veri sonucunu sunar. Ekrana tıklama
  otomasyonu API desteğinin yerine geçmez.

## 5. P0 — tabanı koruma ve kullanım altyapısı

### ARCH-BASE-01 — Güncel GUI tabanı [~]

- Güncel koddan çalıştırılan test uygulamasında Mimari sekmesi, serbest duvar,
  kapı, pencere, mahal, özellik düzenleme ve undo akışlarını kaydet.
- Eski derleme ile güncel kaynak karıştırılmasın; commit, derleme, pencere boyu,
  tema, yazı boyu ve test dosyası kayıtlı olsun.
- En az bir gerçek fare/klavye izi ve önce/sonra ekran görüntüsü. Otomatik GPU
  görüntüsü ek kanıttır; gerçek pencere kontrolünün yerine geçmez.
- Her yeni dilimden sonra aynı kısa senaryo tekrar geçsin. Pencere kilitliyse
  veya erişim yoksa bunu bekleyen kabul olarak açık bırak; tamamlandı işaretleme.
- ADR 0675: CAD salt okunur ölçü basamakları dar panelde elenmeden tam
  genişliğe açılır; gerçek mahal yüksekliği `3 m` eki kusuru dar kapsamda
  düzeltildi. Sekiz görünüm, tek Enter/ret/Escape/undo/redo, 22 ham görüntü ve
  11 bağımsız KCAD kontrolü [doğrulandı](../docs/baseline/2026-10-25-architecture-measurements/README.md).
  Diğer özellik adları/uzunluk girişleri ve tüm araç matrisi açıktır.
- ADR 0676: CAD duvar/açıklık özelliklerinin açık uzunluk ekleri düzeltildi;
  gerçek 25 cm/120 cm kusuru, negatif öteleme, imperial konum/kasa/lento ve
  denizlik boşaltma doğrulandı. 29 ham görüntü, 26 bağımsız kayıt ve 40 başarılı
  özellik testi; [kanıt](../docs/baseline/2026-10-25-architecture-property-units/README.md).
  Diğer mimari nesne uzunlukları ve tam matris açık kalır.
- ADR 0677: CAD kolon/merdiven/rampa fiziksel uzunluk özellikleri ortak
  okuyucuya bağlandı; gerçek kolon `40 cm` kusuru ve bağlı mahal düzeltmesi,
  ret/Escape/tek undo/redo/imperial boy, 10 bağımsız kayıt ve 41 özellik testi
  [doğrulandı](../docs/baseline/2026-10-25-architecture-element-units/README.md).
  Merdiven/rampa tasarım iş akışları ve genel GUI matrisi açık kalır.

- [x] Mac etkin araçta CommandLine/EditCell Cmd-S yönlendirmesi: ADR0682'te
  görülen `s` ekleme gerçek eski/yeni widget ve Mac GUI ile düzeltildi.
  Alan taslağı, çalışan önizleme ve mevcut Enter/odak kaybı korunur; Kaydet
  taslağı onaylamaz. 38 test, clippy/build, 15 ham görüntü ve dokuz bağımsız
  disk kopyası; doğrudan hücre/Tab/komut satırı, tek onay/undo/Escape.
  [ADR 0683](../docs/adr/0683-command-line-chords.md),
  [kanıt](../docs/baseline/2026-10-26-commandline-save/README.md).
- [ ] Diğer giriş bileşenlerinin bütün araç/alan odağı klavye matrisi ve
  §10'daki tam boyut/font/tema kabulü. Tab sırası bu dilimde yeniden
  tasarlanmadı; kusurun ilk hangi commit'te başladığı araştırılmadı.
- [ ] Duvar stili kaydı tabloyu ve izleyen duvar/açıklık/etiket/çizelge/ölçüleri
  tek geri alınabilir adımda tutmalı. 27 Ekim gerçek GUI 130'da ad kaydı doğru,
  131–135'te yalnız ad değişiminin geri alınması başarısızdır; bu mevcut kusur
  menü/form kaydırma kabulünden ayrı açık tutulur. Stil kimliği bazında geri
  uygulama ilişkisiz sonradan edinilmiş stilleri korumalı; daha yeni yerel/uzak
  aynı stil veya ad çakışmasını ezmemeli. Ret/iptal yarım kayıt veya geçmiş
  kaybı bırakmamalı; Python/MCP ve gerçek tek Undo/Redo tam modelle sınanmalı.
  [AutoCAD transaction rollback](https://help.autodesk.com/cloudhelp/2026/ENU/OARX-DevGuide-Managed/files/GUID-C3DE4301-EAC3-4745-A434-9F28EE87AB73.htm)
  ve [Qt komut grupları](https://doc.qt.io/qt-6/qundostack.html) tek işlem ve
  geri alma grubu için karşılaştırma dayanağıdır; KentOS kimlik/çatışma
  politikası kendi tasarım kararıdır. [Tanısal kayıt](../docs/baseline/2026-10-27-architecture-wall-dimensions/README.md).

### ARCH-UX-01 — Görünür mimari araç ayarları [x]

- Etkin araca ait stil, kalınlık, referans yüz, açıklık tipi/en/konum, mahal
  yöntem/ad/numara alanlarını çizimden önce görülebilir yap.
- Seçili nesnenin özellikleri ile yeni nesnenin araç ayarları açıkça ayrı başlıkta
  olsun. Mevcut seçili nesne paneli ve komut satırı kaybolmasın.
- Mevcut seçim/parametre doğrulama yolu kullanılsın. Sayı yazarken her tuş belgeye
  işlenmesin; Enter/odak kaybı uygulasın, Escape geri dönsün.
- Gerçek denemede görülen odak belirsizliğini gider: menüden seçenek seçtikten
  sonra komut alanı odakta değilse harfler genel kısayollara gidiyor. Canvas'ta
  `R` seçeneği anında açılırken ardından boş Enter aracı bitirebiliyor. Metin
  girişi ile tek harf kısayolu kullanıcıya açık olmalı; bütün araçlarda ayrı
  regresyonla ele alınmalı, bu ilk duvar diliminde ortak tuş sistemi değiştirilmemeli.
- Birimli giriş yardımında mevcut söz dizimini örnekle: `200mm`, `4m`, `8"`,
  `12',10'`. Tek uzunluk alanları boşluklu metrik ve ondalık `in`/`ft`
  kabul eder (ADR 0661); koordinat/çift girişinin eski söz dizimi korunur.
- Kabul: parametre adı/harfi ezberlenmeden 250 mm duvar, 900 mm kapı ve net mahal
  seçilebilir; büyük yazıda alanlar çakışmaz; klavye akışı korunur.
- Kabul edilen kapsam: Duvar, Dikdörtgen duvar, Kapı/Pencere/Boşluk, Mahal ve
  Ölçülü bölme için görünür ayarlar; sabit stillerde salt okunur kalınlık,
  geçersiz girişte önceki değeri koruma, gecikmiş eski araç olayını reddetme.
  Ortak klavye/sağ tuş sistemi değiştirilmedi. 18 px gerçek pencerede panel
  büyütülerek alanlar kullanıldı; küçük panelde mevcut kaydırma kullanılır.
  [ADR 0657](../docs/adr/0657-visible-architecture-settings-and-measured-partitions.md)
  ve [ekran görüntülü kabul](../docs/baseline/2026-10-24-architecture-tools/README.md).

- [x] GUI takip bulgusu (ARCH-PERF-01 denemesi): gösterilen `0.300 m` biçimiyle
  düzenleme ortak tek uzunluk okuyucusunda düzeltildi; boşluklu metrik ve ondalık
  in/ft, Enter/blur/iptal/ret gerçek pencerede doğrulandı. Koordinat/çift söz dizimi
  korunur. [ADR 0661](../docs/adr/0661-formatted-single-length-fields.md),
  [kabul ve ekranlar](../docs/baseline/2026-10-25-architecture-input/README.md).

- [x] Etiket için görünür şablon/çerçeve/kol/kağıt yazı boyu; yeni nesne
  ayarları seçim özelliklerinden ayrı. Gerçek GUI'deki ilk yapıştırma ve
  çok satırlı komut düğmesi kusurları giderildi; tek Enter/ret/undo/redo,
  kaynak takibi ve yeniden açma, 26 ham ekran/11 bağımsız KCAD ile doğrulandı.
  [ADR 0685](../docs/adr/0685-visible-tag-settings.md),
  [kanıt](../docs/baseline/2026-10-26-architecture-tag-settings/README.md).
- [x] GUI takip bulgusu: önceki nesne özelliklerinde aşağı kaydırılmış
  panelden yeni mimari araca geçildiğinde etkin ayarların başlangıcını göster.
  Aynı araçta alan düzenleme/önizleme sırasında kullanıcının kaydırmasını
  sıfırlama; mevcut seçim özellikleri ve panel yaşam döngüsü korunsun.
  Dar kapsam [ADR 0686](../docs/adr/0686-reveal-new-tool-settings.md) ile
  doğrulandı: eski widget 508 px hata, yeni başlangıç 0; gerçek seçili
  mahal/etiket/kapı, özellik düzenle/undo/iptal/kapat-aç; 62 test, 19 ham
  ekran/beş bağımsız tam KCAD. [Kanıt](../docs/baseline/2026-10-26-architecture-settings-scroll/README.md).
  Bütün docking yaşam döngüsü/yerleşim kalıcılığı ve tam matris açık kalır.

### ARCH-UX-02 — Mimari şerit ve bağlam [x]

- Çiz / Yerleştir / Düzenle / Belgele / Proje sırasını kullan; tek düğmeye yığılmış
  menüler yerine sık işlere yazılı erişim ver. Tüm komutları aynı anda sergileme.
- Seçili duvarda böl/birleştir/uzat/buda/stil; açıklıkta yön/menteşe/taşı/en;
  mahalde ad/numara/alan/etiket ve çizelge ilgili bağlamsal sekmede bulunsun.
- Taşıma, kopyalama ve genel ölçü komutlarını ikinci kez yazma. Arama, F1 ve
  menüler aynı katalog kimliğini kullansın.
- Kabul: 1440×900 ve 1100×650, 13 ve 18 px, açık/koyu; kesilen düğme, anlamsız
  ikon yığını veya erişilemez komut yok. Kullanım adımı sayısı tabanla karşılaştırılır.
- Kabul: beş grup, mimari sekmelerde adları koruyan daralma, bağlamsal
  Özellikler ve taslağı koruyan Yardımcılar menüsü. Gerçek pencerede dar menüden
  araç başlatma, seçili kapı paneli ve elle kaydırma sonrası Mimari dönüşü geçti.
  Kullanım adımı karşılaştırması, görüntü matrisi ve kapsam sınırları
  [kabul kaydında](../docs/baseline/2026-10-24-architecture-ribbon/README.md);
  karar [ADR 0658](../docs/adr/0658-architectural-ribbon-and-narrow-cad-status.md).

### ARCH-UX-03 — Güvenli mimari başlangıç/şablon [x]

- m/1:100 mimari başlangıcı ile cm/1:50 eski ofis şablonu farkını gideren açık
  uygulama tasarımı yap. Mevcut şablon sözleşmesi incelenmeden sessiz değişiklik yapma.
- Şablonun ekleyeceği katman/stil/sembol/pafta ve değiştireceği ayarlar önce gösterilsin.
- Var olan projede birim/ölçek korunması varsayılan olsun; alternatif ayar uygulanacaksa
  kullanıcı somut farkı görsün. Geometri yeniden ölçeklenmesin.
- Kabul: boş/dolu CAD, gizli/kilitli katman, aynı şablonu iki kez uygulama,
  mevcut kat/stil kimliği çakışması, Python/MCP ve kaydet/aç.
- 24 Ekim kaynak kontrolü: [AutoCAD — Drawings and Templates](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-Core/files/GUID-02979F86-385F-4A53-A3FB-7202F1225CDA.htm)
  yeni çizimin şablondan ayar/stil/katman/pafta aldığını belgeler.
  [QCAD — Drawing/Application Preferences](https://qcad.org/doc/qcad/latest/reference/en/qcad_reference_manual_en.pdf)
  çizime kaydedilen ayarla yeni çizimler için uygulama varsayılanını ayırır.
  KentOS tasarım çıkarımı: mevcut projeye paket eklemek yeni proje açmakla aynı
  eylem değildir; ayar değişimi açık fark ve kullanıcı seçimi gerektirir.

- Uygulama/kabul: [ADR 0659](../docs/adr/0659-safe-architectural-template-preview.md).
  Varsayılan koruma; açık önce/sonra ve kimlik/ad koruma listesi; bütün paket
  tek undo/redo; legacy v1 çağrısı korunur. Sistemli projede metre zorunludur.
  34 bağımsız yeni durum, Python/MCP ve gerçek yerel/projeksiyonlu GUI;
  [19 gerçek ekran ve 16 GPU görünümü](../docs/baseline/2026-10-24-architecture-template/README.md).

### ARCH-PERF-01 — Büyük belgede mimari önizleme [~]

- Mevcut duvar planı, komşuluk aramasında bütün belge nesnelerini tarıyor. İlk
  dilimin eşlenik ölçümünde yeni dikdörtgen ve mevcut kapalı Duvar planları aynı
  çıktı verdi; 100 bin ilgisiz nesnede ortak maliyet belirginleşti. Sonuçlar
  `docs/baseline/2026-10-24-architecture/preview-perf.json` içindedir.
- Önce ayrı ve kalabalık duvar düzenlerinde 1/10/100 bin nesne, uzak koordinat,
  tekrar hareket, stil ve birleşim değişikliği ölçülsün. Python/FFI plan süresi
  ile gerçek pencere olaydan-kareye gecikmesi ve GPU kare zamanı ayrı kaydedilsin.
- Sonra mevcut uzamsal dizini/önbelleği kullanma veya komşuluk sonucunu revision
  ile saklama değerlendirilir. Kilit/katman/evre/stil değişince eski sonuç
  kullanılmamalı; tüm duvar birleşim ve hosted açıklık sonuçları eşit kalmalı.
- Kabul: aynı veri, donanım ve release derlemede tekrarlı önce/sonra ölçüm;
  p50/p95, tahsis/bellek ve değişmez geometri kontrolü. FPS ve zoom titremesi
  bu Python ölçümünden geçmiş sayılmaz; render hattı bu ilk dilimde değişmez.

- Uygulanan ilk iyileştirme: türetilmiş duvar aday dizini; eski FP64 erişim
  kutusu ve belge sırası korunur. Yazma/undo/redo/iptal/dış edit ile yenilenir.
  24 aynı girdili release durumda tam plan/uyarı hash'leri eşit; 20 soğuk / 60
  sıcak örnek ve bellek; Iced olay/CPU/geri okuma ayrı. Uzakta 100 bin duvar / yoğun
  çevre sıcak medyanı 2.379→0.156 ms; soğuk medyanı 3.194→6.377 ms.
  [ADR 0660](../docs/adr/0660-architectural-preview-candidates.md),
  [ham veriler ve 19 gerçek ekran](../docs/baseline/2026-10-24-architecture-preview/README.md).
  Gerçek pencere olaydan-sunuma ve saf GPU zamanı ile stil/evre hız matrisi açık;
  madde bu ilk optimizasyonla kapatılmaz.

- Metrajın ayrı sorgu iyileştirmesi [ADR 0680](../docs/adr/0680-query-scoped-room-quantity-search.md):
  kat/kapsam/UUID ve belge sırası korunarak immutable sorguda kaynak/ağ paylaşımı.
  100 eski/yeni tam rapor eşit. Aynı 400 mahal/21.000 nesneli debug belgede
  sıcak ortanca 8646,943→83,765 ms; yeni süreç ilk sorgusu 815,623 ms.
  Soğuk süreçler eşlenik değildir; release/p95/bellek/UI/GPU kapısı kapanmaz.
  [Ham ölçüm, gerçek GUI ve dosya korunumu](../docs/baseline/2026-10-25-architecture-quantity-cache/README.md).

## 6. P1 — hızlı ve ölçülü plan çizimi

### ARCH-DRAW-01 — Dikdörtgen duvar [x] — ilk uygulama dilimi

- Serbest Duvar ve normal Dikdörtgen'e dokunmadan ayrı araç ekle.
- İki karşı köşe veya ilk köşe + sayısal en/boy + yönle kapalı duvar oluştur.
- Referans seçimi açık: **İç net**, **Dış ölçü**, **Eksen**. İç net 4×3 m,
  200 mm duvarda mahal içi gerçekten 12 m² olmalı. Bu, referans ekseninin 4×3
  olmasından farklıdır; isim kullanıcıyı yanıltmamalı.
- Yeni komuta özgü yol yönünü kararlı yap; dört tıklama çeyreğinde iç/dış ters
  dönmesin. Serbest Duvar'ın mevcut yön/hiza kuralı değişmesin.
- Stil ve kalınlık mevcut duvar kurallarından gelsin. Sabit bileşenli stilin
  eni ile geçersiz genişlik çatışması mevcut açık ret kuralını kullansın.
- Önizlemede gerçek duvar gövdesi ve seçilen referans dikdörtgeni anlaşılır olsun;
  yalnız merkez çizgisini gösterip sonradan farklı geometri yazma.
- Sonuç başlangıçta tek kapalı duvar nesnesidir; otomatik mahal/etiket üretimi,
  komşu duvar silme ve plan topolojisini yeniden yazma bu ilk dilimde yoktur.
- Anlamsal komut önerisi: `cad.walls.rectangle`; araç: `tool.wallRectangle`.
  Mevcut `WallsPlan` / `WallsCreated` ve duvar kuralları yeniden kullanılsın.
- Kabul: dört çeyrek; sayısal boy; döndürme; net/dış/eksen; stil; en az 20
  bağımsız komut örneği; sıfır/negatif/çok küçük boy; uzak koordinat; 5 birim;
  kilitli katman; revision; önizlemede yazmama; tek undo; Python/MCP; GUI.
- İşlevsel kabul ve gerçek ekranlar: [24 Ekim kaydı](../docs/baseline/2026-10-24-architecture/README.md).
  81 bağımsız durum; gerçek net oda/kapı/pencere/mahal, özellikten kalınlık
  düzenleme ve undo, kayıt/açma doğrulandı. Ortak gezinmenin tam GUI tabanı
  ARCH-BASE-01'de, kare zamanı ve büyük belge işi ARCH-PERF-01'de açık kalır.

### ARCH-DRAW-02 — Ölçülü bölme duvar [x] düz ilk dilim

- Bir mevcut duvar yüzünden net uzaklıkla paralel bölme; başlangıç/bitişi seçilen
  sınırlarla önizlemeli bulma. İç açıklık ile eksen aralığı ayrı seçenek olsun.
- Yakın duvarı otomatik benimseme kullanıcıya gösterilsin; büyük görünmez snap
  toleransı veya kaynak duvarı sessiz silme olmasın.
- Uca kadar, iki duvar arasında, tek yön ve iki yön seçenekleri; düz ilk dilim,
  yaylı kaynak ayrı kabul.
- Kabul: 3 m net oda, farklı duvar kalınlığı, T birleşimi, kapılı host, kilitli
  sınır, birden çok aday, bölme taşıma ve bağlı mahal/ölçü güncellemesi.
- Tamamlanan: `cad.walls.partition`; kaynak uçları / iki sınır / verilen boy,
  ileri–geri–iki yön, net / eksen ölçüsü, hiza/öteleme hesabı. Önizleme belgeyi
  yazmaz; tek onay tek undo adımıdır. Masaüstü, Python ve MCP aynı komutu kullanır.
  Gerçek GUI'de 8×6 m kapılı odada 150 mm bölme, 3 m net / 3.175 m eksen,
  48→24 m² mahal; tek sağ tuş onayı ve 48→24 undo/redo görüldü.
  Diğer kabuller ve bağımsız 90 durumun kapsamı
  [kabul kaydında](../docs/baseline/2026-10-24-architecture-tools/README.md).
- [ ] Yaylı kaynak/sınır dilimi: yaylı kenar şimdilik açıklamalı reddedilir.
- [ ] Kaynak düzenlenince uzaklığı koruyan kalıcı kısıt: ayrı veri modeli ve
  çakışma çözümü gerekir. Bu teslimde oluşan nesne normal, düzenlenebilir duvardır.

### ARCH-DRAW-03 — Oda/alan konturundan duvar [~]

- İlk uygulama dilimi: [ADR 0666](../docs/adr/0666-closed-contour-walls.md).
  `cad.walls.convert(mode=contours)` ve mevcut aracın görünür ayarları; bütün
  parçalar/delikler, analitik daire, yön normalleştirme ve atomik ret.
  Sayısal/etkileşim, tam native/desktop ve MCP kontrolleri geçti; gerçek
  koyu/18 px ayar/onay/undo/avlu/ters kaynak/ret/kaydet-aç ile 1100×650
  erişim denendi. [Kabul kaydı](../docs/baseline/2026-10-25-architecture-wall-contours/README.md).
  İlk dilimin üzerine ortak sınır ağı, açık hedef kat ve büyük/karışık ağ
  dilimleri eklendi; tam pencere matrisi ve uçtan uca performans kabulü açıktır.
- Mevcut Çizgiden duvar komutunu kullanarak kapalı konturu duvara dönüştür;
  iç/dış/eksen referansı ve kaynak koru/sil önizlemede açık olsun.
- Ortak sınır dilimi: [ADR 0667](../docs/adr/0667-shared-room-wall-boundaries.md).
  Aynı hedef katmanda, Eksen referansında tam/kısmi düz ve analitik yaylı ortak
  sınırlar bir kez üretilir; normal duvar ağı kapı/pencere host'u olabilir.
  Mevcut 0.000001 m eşleme toleransı ve ilk kaynaktan alınan ortak parça
  öznitelik/görünümü uyarıda açıklanır. İç net/Dış ölçü çelişkisi, asimetrik
  stil, yinelenen taraf ve kesişme atomik reddedilir. Önizleme mevcut sahnenin
  birleşim çözümünü kullanır; açık parçalara ayrılmış ağın tekrarı da reddedilir.
  Gerçek koyu/18 px pencerede düz/kısmi/yay, Koru/Sil, tek sağ tuş onayı,
  undo/redo, tekrar ret ve kaydet-aç sınandı; gerçek 1100×650 pencerede ayarlar
  ve ret açıklaması görüldü. [Kabul kaydı](../docs/baseline/2026-10-25-architecture-shared-wall-contours/README.md).
  Açık tema/13 px gerçek 1100×650 pencerede de yöntem/Eksen/Sil seçimi,
  ortak sınır uyarısı, tek onay ve undo ayrıca doğrulandı.
- Kat dilimi: [ADR 0668](../docs/adr/0668-architectural-wall-level-isolation.md).
  Çizgiden duvarın görünür Kat (L) seçimi ve `cad.walls.convert.arch` alanı,
  Each/Pairs/Contours çıktılarında mevcut semantik katı kullanır. Katsız
  varsayılan ve eski 93 durum korunur; 25 bağımsız durum eklendi. Ortak
  sınır/tekrar ve köşe/T/çapraz birleşimleri aynı kat kimliğiyle sınırlıdır;
  eşit kotta başka kimlik veya katsız duvarla karışmaz. Bilinmeyen/kaldırılmış
  hedef atomik ret verir, kaynak özniteliğinden kat tahmin edilmez.
  Gerçek açık13 ve koyu18, dar1100×650 kat seçimi; açık13 geniş pencerede
  önizleme/kat ayarları/tek onay/undo/redo/ret/kaydet-aç ve bağımsız eski
  kaynak/duvar/ayar korunumu doğrulandı.
  [Kabul kaydı](../docs/baseline/2026-10-25-architecture-wall-levels/README.md).
- Büyük ve karışık ağ dilimi: [ADR 0669](../docs/adr/0669-large-architectural-wall-networks.md).
  Aynı FP64 arrangement hesabında seyrek kaynak sahipliğiyle yoğun kenar×kaynak
  tablosu kaldırıldı. Mevcut overlay yolu/tolerans/ölçek korunur; açık sayım
  kotaları atomik ret verir. Kontur önizlemesi ilk 400 duvarda kesilmez.
  Avlu/dolgu, çok parçalı L oda, kısmi ortak kenar, daire ve yaylı poligonun
  48 birleşimi; 1.600 bitişik/10.000 ayrık oda; yoğun/seyrek birebir hesap ve
  Python/MCP sınandı. Gerçek koyu18 geniş pencerede 40×40 oda → 3.276 duvar,
  tüm kenarlar ve 11.480 m; karma planda 7→16/9, tek onay/undo/redo/ret/kayıt-aç,
  1100×650 dar pencerede görüntü/seçim doğrulandı. Bağımsız KCAD denetimi ve
  çekirdek release soğuk/sıcak süre/süreç tepe belleği
  [kabul kaydındadır](../docs/baseline/2026-10-25-architecture-wall-networks/README.md).
- Açık kabul: tam tema/font/pencere matrisi, native komut/GUI/GPU gecikmesi ve
  bunların uçtan uca bellek/süre ölçümü. Çekirdek ölçümü bu kapıları kapatmaz.
  Etkin/görünür kat yönetimi ve mahalın katı bu hedef kat dilimiyle tamamlanmış sayılmaz.
- Kabul: L oda, iç avlu, ortak kenar, ters halka, delik, eşleşmeyen parça,
  farklı kat ve aynı çizgiden tekrar üretim.

### ARCH-OPEN-01 — Kapı/pencereyi hassas yerleştirme [~]

- Mevcut köşeden mesafe ve ortala seçeneklerini görünür kıl; referans başlangıç/
  bitiş/merkez/menteşe tarafı açık seçilsin.
- Önizlemede yakın köşeye mesafe, en, açılma yönü ve host vurgusu göster.
- İki noktayla açıklık eni seçimi ve bir açıklıktan tipi/eni alma yeni yöntemleri;
  eski serbest yerleştirme korunur.
- Kabul: düz/yaylı/ters duvar; aynalı kapı; uçta taşma; çakışma; host değiştirme;
  tutamaç/sayısal en değişikliği; duvar kalınlığı/bölme/undo sonrası bağ korunumu.
- [x] Yerleştirme dilimi (25 Ekim, ADR 0662): görünür uç/referans, iki pervaz,
  örnekten tür/en, net pay/host önizlemesi; aynı native/Python/MCP komutu.
  40 yeni bağımsız ortak durum, mevcut durumların korunumu, gerçek düz/ters/yaylı
  GUI ve bağımsız kaydet/aç kontrolü. Kanıt: `docs/baseline/2026-10-25-architecture-openings/`.
- [x] Yeni yöntemlerle tutamaç/host değişimi/kalınlık/bölme/undo birleşik native kabulü
  (ADR 0663): iki yöntem × düz/ters host; merkez tutamacı gerçek fare, tek undo/redo
  ve kaydet/aç ile doğrulandı. Karışık sıcak seçim kısmi yazmadan reddedilir;
  genel çoklu mimari tutamaç düzenleme tamamlanmış sayılmaz.
  Kanıt: `docs/baseline/2026-10-25-architecture-grips/`.
- [ ] Tam 1440×900 / 1100×650 × 13/18 px × açık/koyu gerçek pencere kabul matrisi.
  Sekiz durumun GPU renderı ve 18 görüntü incelemesi ayrıca geçti (ADR 0663);
  gerçek geniş pencerenin iç alanı 1440×828 olduğundan tam native matris yerine geçmez.
- [x] Gerçek Kaydır takibi (ADR 0664): toplu native olayda basışın son konuma
  kayması düzeltildi; izleme kodu olmayan derlemede üç ileri/geri çiftinin her
  adımı ve başlangıca dönüşü görüntüde doğrulandı. Bu 1440×828 / açık / 13 px
  kabulüdür; tam gerçek pencere matrisinin yerine geçmez.
- [x] Tutamaçta kenet aday kimliği: 4,772352 m, #1 duvar yüzünün #8/#10
  açıklıkları arasındaki orta noktasıdır; 9,133 mantıksal px uzaklık mevcut
  11 px kenet açıklığı içindedir. App regresyonu adayın Midpoint/#1 kimliğini,
  beklenen konumu, yazmasız önizlemeyi ve undo'yu doğrular; bağımsız decimal
  hesap aynı sonucu verir. Kenet kapalıyken aynı hedef 5 m'dir.
  Davranış/tolerans değiştirilmedi; ADR 0664 kabul kaydında takip kapatıldı.
  İlk bakışta farklı sanılan özet ölçeği büyütülmüş görüntüde 1:100 doğrulandı.

### ARCH-OPEN-02 — Açıklık dağıt / çoğalt [~]

- Aynı duvarda adet + net ara mesafe veya toplam aralığa eşit dağıtma; uç payı
  ve merkez aralığı ayrı seçenek. Her açıklık ayrı host ilişkisi taşır.
- Kapı numaraları için başlangıç/adım/önek; yinelenen numara denetimi.
- Kabul: açık/yaylı duvar, köşe aşımı, mevcut açıklıkla çakışma, 1 ve çok adet,
  bir başarısızlıkta kısmi yazmama, tek undo ve çizelge güncellemesi.

- [x] `cad.openings.edit.array`: kaynak korunarak kopya adedi, net/merkez aralığı,
  yön veya iki uç payı arasında eşit dağıtım; analitik yay uzunluğu ve üç ankraj.
- [x] Yeni kopyalara isteğe bağlı önek/başlangıç/adım; aynı biçimde yinelenen
  numara ve geometrik çakışma bütün işlemi reddeder. Mevcut Copy/Renumber korunur.
- [x] Mimari/Açıklık şeridinde ayrı Açıklık dağıt; görünür ayar, aynı komuttan
  yazmasız önizleme, tek Onayla/Enter, iptal ve tek undo/redo; canlı çizelge.
- [x] Önceki 45 ortak durum değişmeden 40 bağımsız durum eklendi; native
  analitik/etkileşim, Python ve MCP kabulü; gerçek düz/yaylı yerleştirme ve
  kaydedilmiş KCAD'de bağımsız konum/kaynak/kimlik/ayar korunumu.
- [x] Ek gerçek 18 px koyu tema kabulü: numara ret/düzeltme/tek onay, tek
  undo/redo, canlı çizelge, özellik eni/undo, gerçek zoom/pan ve kaydet/aç.
  312 px panelde yeni aracın uzun seçenek etiketleri sadeleştirildi. Bu deneme
  1440×828 iç alandadır; hedef pencere matrisi yerine sayılmaz.
- [ ] §10'un bütün gerçek boyut/font/tema matrisi ve bu araçla birleşik
  zoom/pan/özellik düzenleme kabulü. Ana madde bunun için [~] kalır.
  Ayrıntılar: [ADR 0665](../docs/adr/0665-hosted-opening-arrays.md).

### ARCH-SPACE-01 — Toplu mahal bul ve tanı [~]

- Kat ayrımı önkoşulu: [ADR 0670](../docs/adr/0670-level-bound-architectural-spaces.md).
  Açık kat kimliğiyle mahalın ilk hesabı, yöntem/kat düzenleme planı ve duvarla
  güncellemesi aynı katın duvarlarıyla sınırlıdır; eşit kot kimlikleri karıştırmaz.
  Mevcut Mahal aracında görünür Kat (L) seçimi vardır; kaldırılmış hedef açık ret
  verir. Katı olmayan eski çağrılar **Tüm katlar** davranışını korur.
  Bu önkoşul tek başına diğer kabul koşullarını tamamlamaz.

- Toplu bulmanın ilk dilimi: [ADR 0671](../docs/adr/0671-bulk-room-discovery.md).
  Mimari/Çiz **Toplu mahal bul**, açık kat veya Tüm katlar ağının kapalı
  boşluklarını Net/Eksen/Brüt önizler. Bölge, temsil noktasıyla oda seçer;
  duvarları veya oda sınırını kesmez. Aynı boşluktaki mevcut mahal UUID'leri
  korunur; elle çizilmiş kısmi çakışma kırmızı gösterilir ve bütün yazmayı
  engeller. Onay tek komut/undo'dur, boş tekrar undo eklemez. Python ve MCP
  aynı `cad.spaces.discover` plan/execute yolunu kullanır. Ortak geçici sınır
  dizini çoklu oluşturma ve follower'da aynı kaynak kümesi için paylaşılır;
  eski yakın/tam arama ve katsız çağrı semantiği korunur.
  [Kabul kaydı](../docs/baseline/2026-10-25-architecture-space-discovery/README.md).

- Atanmamış kaynak kapsamı [ADR 0674](../docs/adr/0674-unassigned-space-sources.md)
  ile doğrulandı. Eski boş Kat / Tüm katlar davranışı korunur; yeni açık kapsam
  mahalda saklanır ve follower/kayıt-aç sonrasında da geçerlidir. Görünür İç nokta
  seç mevcut rebound komutuna bağlanır; duvar üstü seed kapsam değişiminde atomik
  ret verir. Son sınır/uyarı, tek onay/undo/redo, 90 bağımsız durum, Python/MCP,
  56 gerçek görüntü ve 29 bağımsız GUI kaydı doğrulandı. İki tema/yazı/boyut yeni
  ayarları kapsar; geniş istemci yaklaşık 1440×825'tir, tam 1440×900/genel iş akışı
  kabulü değildir. [Kabul](../docs/baseline/2026-10-25-architecture-space-scope/README.md).
  Kalıcı kaynak listesi ADR 0678, sanal ayırıcı ADR 0679 dilimindedir.

- Kalıcı kaynak alt kümesi [ADR 0678](../docs/adr/0678-persistent-room-source-selection.md):
  tek/toplu mahal görünür Seçilenler tercihiyle duvar/kolon UUID'lerini saklar.
  Mevcut mahalın listesi **Mahal kaynakları** ile önizlenip tek onayla değişir.
  Kayıp/uygunsuz kaynak son sınırı ve açık uyarıyı korur; bütün çizime sessiz
  genişleme olmaz. Tam kopya grubu yeni UUID'lere bağlanır; eksik grup son
  geometrisiyle bağımsız olur ve uyarır. Python/MCP, opt-in KCAD 105 ve DXF
  kimlik eşlemesi aynı sözleşmededir. 63 bağımsız ortak durum, 53 ham görüntü,
  30 bağımsız GUI kaydı; ret/Escape/tek onay/undo/redo/kayıt-aç kontrolü
  [kabul kaydında](../docs/baseline/2026-10-25-architecture-space-sources/README.md).
  Sanal ayırıcı aşağıdaki ADR 0679 dilimindedir; büyük belge performansı ve
  bütün iş akışlarının tam matrisi açıktır.

- Kalıcı sanal sınır [ADR 0679](../docs/adr/0679-virtual-room-separators.md):
  Mimari/Çiz **Mahal ayırıcı** genişliksiz analitik path rolüdür; sıradan çizgi
  veya görünüşünden rol tahmin edilmez. Kat/evre/açık UUID kaynak seçimi ve
  mahal→etiket→çizelge tek undo kullanır. Create/set/clear plan/execute,
  Python/MCP, opt-in KCAD 106, DXF kimlik eşlemesi ve destekli kopya ortaktır;
  uygunsuz dönüşüm/blok içine alma atomik reddir. Her kopuk mahal son sınırı
  Metraj/API/CSV'de açıkça bildirir. 105 bağımsız ortak durum, 300 Python,
  35 bağımsız GUI kaydı ve 76 ham görüntü; tek onay/undo/redo/kayıt-aç,
  gerçek CSV ve yeni ayarların sekiz görünümü
  [kabul kaydındadır](../docs/baseline/2026-10-25-architecture-space-separators/README.md).
  Metrajın sorgu paylaşımı ADR 0680 ile uygulandı. Genel ret/girdi denetimi,
  release uçtan uca süre/bellek ve tüm
  araç iş akışlarının tam matrisi açık kaldığından ana madde [~] kalır.

- Kolon düşümü dilimi [ADR 0672](../docs/adr/0672-space-column-deductions.md)
  ile doğrulandı: kalıcı Düşme/Yalnız net/Her yöntemde tercihi, gerçek analitik
  kolon izi, kat kimliği ve aks → kolon → mahal → etiket/çizelge sırası.
  Eski mahal ve alanlar varsayılan olarak korunur. Gerçek GUI'de bulunan daire
  önizleme dolgusu ve mevcut GPU konum kusuru düzeltildi; [ADR 0673](../docs/adr/0673-invariant-cad-gpu-positions.md)
  gerçek GPU, zoom/pan ve kaynak korunumu kontrolünden geçti. Tek Enter toplu
  oluşturma, boş tekrar, aksla kolon taşıma, mahal/etiket/çizelge ve tek
  undo/redo/kayıt-aç bağımsız KCAD hesabıyla doğrulandı. Dar koyu/18 px pencerede
  bulunan iki açıklama kırpılması son build’de düzeltildi ve gerçek fareyle
  tekrar doğrulandı. Uyarı paneli açık/koyu × dar/geniş × 13/18 px sekiz
  görünümde okunur; bu, bütün araç iş akışlarının tam matrisi yerine geçmez.
  [47 ham görüntü ve bağımsız KCAD kabulü](../docs/baseline/2026-10-25-architecture-space-columns/README.md).

- Seçili kat/bölge veya kapalı duvar ağında mahalleri bul; mevcut mahalı yineleme.
- Kapalı olmayan sınırı sahnede vurgula; kapı geçişi, kolon/delik ve açık plan
  sanal sınırı için açık kural sun. Net/eksen/brüt sınır önizlemesi olsun.
- Kabul: iki oda, açık koridor, kolon, iç avlu, çakışan seed, farklı kat;
  kaynak duvar değişince sınır, alan, etiket ve tablo birlikte değişir.
- Teknik kapılar ve kalanlar:
  - [x] Aynı birleşmiş duvar yüzlerinden kapalı boşlukları çıkar; duvar
    gövdesi hücrelerini yönlü analitik malzeme sınırıyla ele. Bölge seçiminin
    iç temsil noktası kuralı görünürdür; sınır kesilmez.
  - [x] Mevcut mahal kimliği/seed'iyle örtüşen öneriyi ayrı göster; ad veya
    numara eşitliğini geometri kimliği sayma. Çizilmiş mahalın kısmi çakışması
    sessiz atlama veya üstüne yazma yerine açık atomik rettir.
  - [x] Yeni toplu iş akışı kat tanımı olmayan basit planlarda da çalışır.
    Eski `arch.level` yokluğu bütün çizimi aradığı için, yeni işin yalnız
    atanmamış duvarları veya seçilmiş kaynakları kullanma kapsamı kalıcı ve
    açıkça temsil edilir; yalnız ilk plan isteğinde verilen, follower'ın
    sonraki düzenlemede kaybettiği bir süzgeç kullanılmaz (ADR 0674/0678).
  - [x] Kapı açıklığı/fiziksel kopukluk farklıdır; sanal ayırıcı kalıcı açık
    path rolüdür, rastgele çizgi/öznitelik adı değildir (ADR 0679). Kolonlar
    gerçek kaynak izi/katıyla düşer (ADR 0672). Blok içi kaynaklar ve bütün
    genel düzenleme üst araçlarının denetimi bu işaretin kapsamında değildir.
  - [x] Kolon aksı değişince aynı adımda aks → kolon → mahal → etiket/çizelge
    sırası kullanılır (ADR 0672). Kaynak bağı, eski/yeni etki alanıyla geçersiz
    kılma ve tek undo/redo native test ve gerçek GUI/bağımsız kayıtla doğrulandı.
  - [~] Çok mahalde aynı kaynak kümesi için ortak geçici sınır dizini plan ve
    follower'da paylaşılır; kaynağın immutable ödüncü dışında yaşayamaz.
    400 seed için eski/yeni tam plan eşitliği ve çekirdek süre ölçümü vardır.
    Sanal sınır/kolon bağımlılıkları ADR 0679 ile doğrulandı. Metraj da aynı
    immutable sorguda kaynak/ağı paylaşır (ADR 0680); 100 dondurulmuş tam rapor
    eşit, 400 mahal/20.000 ilgisiz çizgide sıcak debug ortanca 8,647s→83,765ms,
    yeni süreç ilk sorgusu 815,623ms. Release uçtan uca süre/bellek ve bütün iş
    akışlarının tam GUI matrisi açık kalır; bu satır [~] kalır.

### ARCH-SPACE-02 — Mahal kimliği, etiket ve numaralandırma [~]

- [x] Tıklama sırası veya açık iki noktayla yön izdüşümü; tam ilk numara,
  sıfır dolgusu/önek, 1–10000 farklı UUID. Kimlik numaradan ayrıdır.
  Seçim dışındaki aynı tam numara başka katta/gizliyken de atomik reddir;
  otomatik atlama yoktur. Görünür ayar/numara önizlemesi, geri alma, tek onay
  ve Escape; yalnız numara ve bağlı etiket/çizelge güncellenir, tek undo.
  `cad.spaces.number` plan/validate/execute, Python/MCP ve kayıt/tekrar ortaktır.
  [ADR 0682](../docs/adr/0682-explicit-room-numbering.md),
  [kabul](../docs/baseline/2026-10-26-architecture-space-number/README.md):
  68 bağımsız ortak durum, 300 Python, 270 application / 6 interaction /
  7 gerçek MCP / 3 desktop geçmiş testi; 52 ham görüntü ve 20 disk anlık
  görüntüsü (tanısal mevcut-disk kopyaları ayrıca açıklanır).
  Gerçek GUI'de bulunan boş iç seçim kusuru yalnız bu araçta düzeltildi;
  büyük yazıda eski numara tekrarı kaldırılarak uzak önizleme sadeleştirildi.
- [x] Projeyle saklanan kat öneki ve sıralanabilir ad/kullanım listeleri;
  ayrı öneri alanları serbest metni korur, kat değiştirme taslak numarayı
  değiştirmez. Açık önek seçimi, tek onay/undo/redo, atomik çakışma reddi;
  liste kaydı mevcut mahal/etiket/UUID/geometriyi değiştirmez.
  Python/MCP, opt-in KCAD107 ve DXF tercih kaybının yazmadan önce/raporda
  uyarılması doğrulandı. Gerçek GUI'nin bulduğu açıklama yeri düzeltildi;
  42 ham ekran/23 tam bağımsız kayıt, 385 ortak durum/329 Python/dört MCP/
  iki desktop, clippy/build/WASM/1203 KCAD ve eski 1170 bayt korunumu.
  [ADR0690](../docs/adr/0690-project-room-choices.md),
  [kabul](../docs/baseline/2026-10-26-architecture-room-choices/README.md).
- [x] Yinelenen ve numarasız mahal denetimi mevcut Etiketleri denetle'de
  görünür seçenekle çalışır. Ad/kat/UUID ve Göster; gizli/kilitli ve başka
  katlar dahil. API'nin yok/null/false tercihi eski sonuç/sırayı korur;
  tam metin numara ayrımı ve sorgunun salt okunurluğu değişmez.
  233 ortak durum (önceki 21 check aynen), 323 Python, 68 domain/697
  interaction/219 application/üç MCP/iki desktop; clippy/build/WASM/1170
  KCAD ve bağımlılık yönü geçti. Gerçek tek Enter/boşalt/undo/redo/Escape/
  yeniden açma; 18 ham ekran ve 12 tam dosya bağımsız okuyucuda doğrulandı.
  [ADR 0689](../docs/adr/0689-room-number-review.md),
  [kabul](../docs/baseline/2026-10-26-architecture-space-identity/README.md).
  10.000 mahal+20.000 çizgide debug sorgu ortancası 241,663 ms; 9.998 eksik
  numarada 452,363 ms. Genel ekran/performance veya tam matris kabulü değildir.
- [x] Etiket şablonu: ad, numara, net alan, döşeme/tavan kotu; sürüklenebilir yer,
  manuel metin kopyasına dönüşmeden kaynak güncellemesi. Ayrı kabul kayıtları
  ADR0684/0685/0687/0688 ve aşağıdaki görünür içerik dilimindedir. Tam mimari
  kat/görünüş/pafta matrisi bu işaretin kapsamında değildir.
- [x] Etiket ve toplu etikette sekiz Hazır içerik önerisi; serbest şablon/özel
  metin, kaynak katı ve Koru/Yenile korunur. [ADR0693](../docs/adr/0693-visible-tag-content-examples.md),
  [kabul](../docs/baseline/2026-10-26-architecture-tag-contents/README.md):
  63+708 interaction, desktop build/all-targets clippy; gerçek tek Enter,
  kaynak yüksekliği/undo/redo/toplu yenileme/özel metin/ret/iptal/kaydet-aç.
  Açık13 geniş/koyu18 geniş-dar; 34 ham ekran (yedi tanısal), 15 tam bağımsız
  model, önceki1810 KCAD baytı aynı. Yeni TS/şema/API/birim/render yoktur.
- [x] Gerçek etiket tutamacıyla sürükleme sonrası 40 cm duvar değişikliği;
  mahal/etiket/çizelge birlikte 20.16 m², elle konmuş yer korunur. Mahal/duvar
  silme ve görünür kopuk bağ uyarısı, şablon değiştirme, tek undo/redo,
  önizleme/Kaydet/sağ tuş Bırak/Escape ve yeni süreçte kayıt açma doğrulandı.
  [ADR 0684](../docs/adr/0684-tag-grip-placement.md),
  [kabul](../docs/baseline/2026-10-26-architecture-tag-follow/README.md):
  22 ham görüntü, 18 bağımsız disk kopyası; 692 interaction/219 application/
  303 Python/bir mevcut MCP testi. Sadece sıcak etiket UUID'lerinin mevcut
  anlamsal taşıma rotası düzeltildi; yeni TS/şema/birim/render değişikliği yok.
- [x] Yeni etiketin görünür şablon, çerçeve, kol ve kağıt mm boyu alanları
  mevcut `cad.tags.create` yolunu kullanır. 20 şablon/çerçeve ve 20 birim/ölçek
  bileşimi; 697 interaction, 40 UI/host, üç mevcut ayar/üç mevcut Python
  testi; clippy/build, gerçek açık18/koyu13 geniş/dar girişleri ve 11 tam
  dosya karşılaştırması. [ADR 0685](../docs/adr/0685-visible-tag-settings.md),
  [kabul](../docs/baseline/2026-10-26-architecture-tag-settings/README.md).
- [x] Kaynağa bağlı `{floorElevation}` / `{ceilingElevation}` şablonları:
  bilinen kat kotu + yalnız açık mahal yüksekliği; eksik veri boş, bilinen
  sıfır sıfırdır. Katlar arası yükseklik tavan sayılmaz. Mevcut çizim birimi,
  manuel yer/UUID, takip/ayar/geçmiş/iptal ve dış aktarım korunur.
  Gerçek GUI yüksekliğinin sayısal taşması düzeltildi; 19 ham ekran/on tam
  bağımsız kayıt, tek Enter/ret/undo/redo/kaydet-aç; 195 ortak durum,
  309 Python, 697 interaction/219 application/67 domain ve bir MCP testi.
  [ADR 0687](../docs/adr/0687-room-tag-elevations.md),
  [kabul](../docs/baseline/2026-10-26-architecture-tag-elevations/README.md).
- [x] Ayrı `{netArea}` şablonu Net/eksen/brüt seçimini değiştirmeden aynı
  kat/kapsam/kaynak/kolon politikasının iç yüzlerini hesaplar. Kaynak
  değişiminde brüt sınır aynı kalsa da metin izler; elle yer/UUID korunur.
  Kopuk veya kaynakları bilinmeyen çizilmiş brüt/eksen için sayı uydurulmaz.
  225 ortak durum/30 yeni, 318 Python, 68 domain/697 interaction/219
  application/iki MCP, clippy/build/WASM/1170 KCAD; gerçek tek Enter,
  kolon/duvar/ret/undo/redo/kaynak kaybı/iptal/kaydet-aç, 21 ham ekran ve15
  bağımsız tam dosya. [ADR 0688](../docs/adr/0688-room-tag-net-area.md),
  [kabul](../docs/baseline/2026-10-26-architecture-tag-net-area/README.md).
  Yasal net alan profili veya bütün mimari görünüm matrisi kabulü değildir.
- [x] Toplu etikette açık Koru/Yenile; hedef katmandaki mevcut etiketlerin
  kimliği/elle yeri/kolu ve belirtilmeyen alanları korunur. Başka etiket
  katmanları değişmez. Görünür şablon/çerçeve/kağıt mm boyu, sekiz kaynak
  türü, sayılı salt okunur önizleme; yeni+yenilenen birlikte tek undo.
  Klavyeyle etkin katman değişimi ilk onayda yalnız yeni hedefin planını
  gösterir; normal akış tek onaydır. Python/MCP ve eski yok/null/false aynı.
  31 yeni bağımsız durum/ilgili264, 335 Python, son63+702 interaction,
  51+221 application, 809 commands; clippy/build/WASM/SDK ve uyum geçti.
  35 ham ekran/24 tam beklenen model +bir tanısal disk kopyası; gerçek
  klavye hedef değişimi/ret/Escape/tek onay/undo/redo/kaydet-aç doğrulandı.
  [ADR0691](../docs/adr/0691-bulk-tag-refresh.md),
  [kabul ve kapsam](../docs/baseline/2026-10-26-architecture-tag-refresh/README.md).
- [x] Toplu etikette açık kaynak mimari kat kapsamı: Tüm katlar / Yalnız
  atanmamış / proje katı. API çoklu kat birleşimi; açıklık katı duvardan
  gelir, kayıp host atanmamış sayılmaz. Koru/Yenile ve açık tags aynı
  komutta korunur. 26 yeni bağımsız durum/ilgili290, 340 Python/75 MCP,
  809 commands, 52+221 application/63+704 interaction; clippy/build/WASM/
  SDK geçti. Gerçek tek Enter/sağ tuş Onayla/ret/Escape/undo/redo/kaydet-aç;
  43 ham görüntü (tanısallar dahil), 27 tam bağımsız kayıt ve önceki1781
  KCAD bayt korunumu. [ADR0692](../docs/adr/0692-bulk-tag-levels.md),
  [kabul ve sınırlar](../docs/baseline/2026-10-26-architecture-tag-levels/README.md).
- [ ] Çoklu mimari kat/görünüş/pafta oranı bağlamı ve tam mimari matris.
  Toplu araçta varsayılan tüm katlar korunur; `all.levels` açık kaynak
  mimari kat kapsamıdır. `all.layers` kaynak CAD katmanını süzer.
  `{area}` seçilmiş alan yöntemidir; ayrı `{netArea}` yukarıda doğrulandı.

### ARCH-DIM-01 — Otomatik mimari ölçü dizileri [~]

- [~] İlk kalıcı sıra dilimi: doğrusal tek duvarın sol/sağ yüzü, açıklık
  uçları/merkezleri ve toplam; görünür ayarlar, tek onay, kalıcı üyelik,
  sıra/metin yerleşimi, Python/MCP, KCAD ve atomik bulut eşitleme.
  [ADR0695](../docs/adr/0695-associative-wall-dimension-sets.md) uygulanıyor;
  otomatik test ve gerçek arayüz kabulü tamamlanmadan bitmiş sayılmaz.
  Kapalı/çok parçalı cephe, mahal/çekirdek ve çoklu pafta kapsamı açıktır.
- [~] Kullanıcının bildirdiği duvar stili görünürlüğü regresyonu:
  1:100 otomatik ayrıntıda yaklaşınca bileşenlerin gelmemesi ve yoğun
  GPU taramanın kaybolması [ADR0696](../docs/adr/0696-model-wall-detail-and-dense-hairlines.md)
  ile ele alınıyor. Model gösterimi/pafta ölçeği ayrılır; gerçek GUI,
  GPU ve pafta/PDF karşılaştırması tamamlanmadan kabul edilmiş sayılmaz.
- [x] İlk altyapı: açıklığın gerçek jamb köşeleri/başlangıç/merkez/bitiş
  noktalarına mevcut ilişkili ölçüyü bağla; kapı eni/konumu/host değişimi,
  kopuk bağ ve tek undo/redo. [ADR0694](../docs/adr/0694-opening-dimension-places.md)
  kabul edildi: gerçek kesilmiş duvar jambı kenedi açıklığa bağlanır,
  genişlik/konum/kalınlık/bölme/rehost/kopuk bağ aynı kaynağı izler.
  342 Python/76 MCP/809 commands, son native takımlar/build/clippy/WASM;
  36 ham ekran/21 tam bağımsız model ve dört ayrı tanısal kopya,
  önceki1828 KCAD aynı. [Kanıt ve sınırlar](../docs/baseline/2026-10-26-architecture-opening-dimensions/README.md).
  Otomatik ölçü dizileri ve kaynak üyeliği tamamlanmış değildir.
- Cephe/duvar grubu/mahal seç; açıklık, ara duvar, yapı ve toplam boy için
  seçilebilir sıralar. İç yüz, dış yüz, çekirdek ve açıklık merkezi/uçları açık.
- Ölçüler mevcut ilişkili ölçü modelinden üretilsin; Zincir/Baz/Hızlı ölçü
  davranışı değişmesin. Yeni araç görünüş çizgilerine ölçü bağlamasın.
- Metin sığmama ve sıra aralığı önizlensin; üst üste yazıları gizleyerek çözme.
- [ ] Otomatik sıranın kalıcı kaynak tarifi: seçilmiş duvar/mahal UUID'leri,
  mimari kat ve hangi yüz/merkez/uç politikasının kullanıldığı saklanmalı.
  Sadece tek sefer çok sayıda serbest ölçü eklemek otomatik sıra kabulü değildir.
- [ ] Kaynaklar değişince sıranın üyeliği de güncellensin: sonradan kapı ekleme,
  açıklık kaldırma, duvar bölme/rehost ve ters yön; aynı kaynak çiftinin ölçü
  kimliği ve kullanıcının taşıdığı sıra yeri korunmalı. Belirsiz topoloji
  değişimi sessiz yanlış ölçüye dönüşmemeli, görünür inceleme gerektirmeli.
- [ ] Her sıranın önizlemesi kaynak, ölçülen yüz ve sayıyı açıklamalı. Kağıt mm
  cinsinden sıra aralığı mevcut çizim ölçeğiyle dönüştürülmeli; kaynak model
  veya proje birimi sırf ölçü yerleştirmek için değiştirilmemeli. Yazı sığma
  denetimi mevcut belge yazı tipi/ölçü stiliyle yapılmalı.
- [ ] İlk otomatik üretimin desteklediği geometri açıkça daraltılmalı:
  doğrusal kaynak/yaylı kaynak ve kiriş/host boyunca mesafe aynı şey değildir.
  Desteklenmeyen yüz/kat/karma kaynağa tek işlemden önce açıklamalı ret verilmeli;
  kısmen yazılmış ölçü dizisi kalmamalı.
- [ ] Mevcut oluştur/yeniden bağlama ve yeni sıra yaşam döngüsünün Python/MCP
  karşılığı, salt okunur planı, tek undo'su, kaydet-aç ve yeni sürüm uyumu
  birlikte doğrulanmalı. Native kenet/ölçü altyapısı tek başına bu kapıyı kapatmaz.
  Güncel iş akışı karşılaştırması:
  [Archicad29 dış ölçü](https://help.graphisoft.com/AC/29/INT/_AC29_Help/070_Documentation/070_Documentation-49.htm)
  (seçilmiş duvar, yön, yer, toplam/dış geometri/yapı/açıklık, merkez-uç, yüz ve
  sıra aralığı); yerel uygulama testi yapılmış sayılmaz.
- Kabul: kapı eni ve konumu değişir, duvar uzar/bölünür/silinir; ölçü güncellenir
  veya kopuk bağ açık gösterilir. Toplam ile parçaların yuvarlama kuralı belirli.

### ARCH-EDIT-01 — Geçici ölçüyle doğrudan düzenleme [ ]

- Seçili kapı/duvar/kolonun yakındaki referansa mesafesini göster; sayıya basıp
  hedef mesafe yazılabilsin. Sabit kalan taraf/ankraj açık olsun.
- Ölçüyü değiştirme ve nesneyi değiştirme farklı eylemdir; mevcut ölçü değeri
  metnini sahte olarak override ederek model taşınmış gibi gösterme.
- Kabul: kilitli referans, iki aday, kaydırılmış UCS, büyük koordinat, Escape,
  tek undo; aynı anlamsal mesafe komutu Python/MCP'den çalışır.

## 7. P1/P2 — eksik mimari eleman ve donatı araçları

### ARCH-LIB-01 — Mobilya ve vitrifiye galerisi [ ]

- Yatak, masa, sandalye, koltuk, dolap; klozet, lavabo, duş/küvet; evye, ocak,
  buzdolabı, çamaşır makinesi için gerçek boylu 2B kitaplık.
- Mevcut blok/dinamik blok ve kategori galerisi kullanılsın. Kapıdan büyük yatak
  üretecek belirsiz blok birimi yok; sembol birimi, fiziksel en/boy, yerleşim
  ankrajı ve dönüş önizlemede gösterilsin.
- Boyut varyantı, aynala, döndür, duvara hizala ve isteğe bağlı boşluk zarfı.
  Boşluk zarfı otomatik mevzuat uygunluğu iddiası değildir.
- Kabul: m/mm proje, dış DWG bloğu, iç içe INSERT, aynalı yerleşim, gerçek
  ölçüm, blok özellikleri, kaydet/aç ve DXF görünüşü.

### ARCH-LIB-02 — Mutfak/dolap dizisi ve tezgâh [ ]

- Düz/L/U dizi; modül eni/adet, köşe çözümü, tezgâh derinliği; evye/ocak boşluğu,
  dolap açılma yönü ve yerleşim duvarına göre konum.
- Parametreli bileşen veya kaynaklı blok grubu; tür seçimi tabloyla izlenebilir.
- Kabul: toplam uzunluk/modül dağılımı, dar köşe, negatif kalan, çakışma,
  tek modül düzenleme ve kaynak duvar taşınması. İlk iş basit düz diziyle sınırlı.

### ARCH-ELEM-01 — Döşeme, balkon, teras ve şaft sınırı [ ]

- Kapalı kontur, kenar ötelemesi, delik/boşluk, kat/kot ve malzeme/kaplama bilgisi.
- Mevcut polygon/hatch/mahal modelleriyle nelerin temsil edilebildiğini incele;
  gerekli değilse ikinci alan motoru veya yeni dosya türü kurma.
- Korkuluk/parapet 2B çizgisel sembolü; kaplama geçişi ve süpürgelik gibi
  çizgisel bitişler kaynak boyunca türesin.
- Kabul: L kontur, avlu, ıslak hacim deliği, şaft, offset başarısızlığı,
  metraj dahil/harici seçimi ve açık kayıp raporuyla temel CAD'e verme.

### ARCH-ELEM-02 — Niş, baca, tesisat boşluğu [ ]

- Duvar içinde niş/kısmi oyuk ile tam geçişli boşluk ayrımı; genişlik, derinlik,
  yüz, kot ve gösterim. Mevcut kapı boşluğunu farklı anlamda sessizce kullanma.
- Kabul: kalınlığı aşan derinlik reddi, açıklıkla çakışma, evre, duvar taşıma ve
  2B görünüş/metraj sözleşmesi. 3B katı sonucu bu aşamada vaat edilmez.

### ARCH-ELEM-03 — Aks ve kolon yerleşimini güçlendir [ ]

- Aks aralık listesiyle ortogonal/döndürülmüş ağ, iki uç balonu, kesik aralık;
  aks kesişimine kolon, dikdörtgen/daire/L varyant ve hiza.
- Mevcut aks/kolon/numaralama komutları genişletilir; kat bilgisi korunur.
- Kabul: sıra atlama, yinelenen aks, negatif yön, kolon dönüşü, kilitli katman,
  referans kopması ve ölçü/çizelge güncellemesi.

### ARCH-STAIR-01 — Kullanılabilir 2B merdiven tasarımı [ ]

- Düz/L/U ve sahanlık; toplam yükseklik, rıht sayısı, basamak derinliği, kol eni;
  değiştirilen parametre ve sabit tutulan kural açık olsun.
- Çıkış oku, basamak numarası, kesme çizgisi, kesme üstü kesik çizgi ve kat
  görünümü. Dairesel/döner merdiven ayrı sonraki dilim.
- Formül önerileri kurum/standart profili olarak adlandırılır; otomatik güvenlik
  veya ruhsat uygunluğu iddiası yok.
- Kabul: sayı/gerçek geometri tutarlılığı, başlangıç/bitiş yönü, aynala,
  sahanlık çakışması, pafta gösterimi ve 2B ölçülebilir çıktı.

### ARCH-STAIR-02 — Rampa ve korkuluk [ ]

- Eğim, uzunluk, kot farkı ve sahanlık arasından açık sürücü parametreler;
  çıkış yönü, kot etiketi ve korkuluk kenarı.
- Mevcut Rampa'nın veri modeli/komutu kullanılır. Yerel erişilebilirlik
  yönetmeliği ayrı sürümlü profil; genel CAD aracı mevzuat onayı vermez.
- Kabul: ters yön, sıfır kot farkı, sınırdaki eğim, kaynak kot değişikliği,
  basılı gösterim ve metraj.

## 8. P2 — belgeleme, denetim ve çıktı

### ARCH-DOC-01 — Mimari semboller ve notlar [ ]

- Kot/seviye, kesit/görünüş/detay işareti, kuzey, revizyon ve malzeme lejandı;
  mevcut viewmark/etiket ilişkisi ve yazı stilleri kullanılır.
- Plan işareti kesit geometrisi üretildiği anlamına gelmez. Başvuru paftası
  silinirse bozuk referans bildirilir.
- Kabul: çoklu pafta, yeniden numaralama, kopyalanmış işaret, fontlar ve PDF.

### ARCH-DOC-02 — Çizelge ve metraj iş akışı [~]

- [x] Gerçek CSV takip kusuru [ADR 0681](../docs/adr/0681-selected-quantity-csv.md):
  CSV açık sekmeyi, XLSX/ODS üç tabloyu yazar; mevcut kat süzgeci korunur.
  Duvar sekmesinden yanlış mahal CSV'si ve 400/600 satır bildirimi düzeltildi.
  Gerçek CSV 200 duvar/7200m, 400 mahal/14488m², boş açıklık 0 satır; kitaplar
  üç sayfa/600 veri satırı. İptal, gerçek geçmiş, KCAD yeniden açma ve bütün
  model korunumu bağımsız okuyucuyla doğrulandı. Üç desktop testi ve clippy/build,
  [17 ham görüntü/gerçek çıktılar](../docs/baseline/2026-10-26-architecture-quantity-export/README.md).
- [ ] Mevcut ham hesap değerleri 15 anlamlı basamağı aşınca kitap yazıcısı
  metin tutup bildirir. Kullanıcı alanını koruma ile hesaplanan metrajın
  kontrollü sayı/yuvarlama tercihi tasarlansın; geometri hesabı veya kaynak
  değerler sessizce yuvarlanmasın. Bu CSV düzeltmesi o politikayı değiştirmez.
- Kapı/pencere, mahal kaplama, duvar ve donatı çizelgesi; kat/evre/tip filtresi,
  toplam ve gruplanmış satır. Mevcut canlı kaynaklı tablo korunur.
- Kullanıcı alanı ile hesaplanan alanı ayır; bir hücreyi değiştirmek kaynak
  nesneye yazacaksa hangi nesneleri değiştirdiğini göster.
- Kabul: duvar/kapı/mahal düzenleme, birim/yuvarlama, boş veri, çoklu kat,
  kilitli kaynak ve CSV/XLSX/PDF dışa verme.

### ARCH-DOC-03 — Mimari proje denetimi [ ]

- Açık duvar sınırı, çakışan açıklık, kopuk host/ölçü/etiket, yinelenen numara,
  eksik tip/kat, sıfır alan ve çok küçük artık parça için odaklanılabilir liste.
- Mevcut birleşim ve etiket denetimleri bir araya getirilir; otomatik onarım
  yalnız açık önizleme ve seçili bulgular için. Kaynak geometriye sessiz snap yok.
- Kabul: bir bulguya tıkla-yakınlaş-düzelt-yeniden denetle, undo ve aynı raporun
  Python/MCP'den alınması. “Hata yok” ruhsat/standart uygunluğu değildir.

### ARCH-DOC-04 — Pafta takımı ve ofis profili [ ]

- Mevcut sheet_set ile A3/A2/A1 plan, detay ve çizelge yerleşimi; antet,
  revizyon, ölçek ve görünüş ayrıntısı. Hazır takım uygulamadan önce fark göster.
- Model ve basılı anotasyon davranışı mevcut güncel kuralla sürer. Birim/render
  sistemi bu araç için yeniden tasarlanmaz.
- Kabul: 10 m uzunluk 1:50'de 200 mm, 1:100'de 100 mm; kâğıt boyu, font,
  kalem/tarama, viewport kırpma, PDF ve DXF/DWG kayıp raporu bağımsız kontrol edilir.

## 9. Standart ve kurumsal profil yaklaşımı

- [ISO 7519:2025](https://www.iso.org/standard/89718.html): mimari/inşaat çizim
  sunumu; [ISO 128-2:2022](https://www.iso.org/standard/83355.html): çizgi
  gösterimi; [ISO 129-1:2018](https://www.iso.org/standard/64007.html): ölçü sunumu.
- [ISO 3098-1](https://www.iso.org/standard/65679.html): teknik yazı;
  [ISO 5455](https://www.iso.org/standard/11500.html): ölçek;
  [ISO 5457](https://www.iso.org/standard/29017.html): sayfa;
  [ISO 7200](https://www.iso.org/standard/35446.html): antet verisi.
- Katman adlandırması, çizgi/tarama/yazı/ölçü stilleri ve semboller ofis
  profilinde sürümlenir. Var olan DWG katmanları zorla yeniden adlandırılmaz.
- Tam normatif metin ve madde bazında test olmadan genel “ISO uyumlu” sonucu
  verilmez. Türkiye/kurumun ruhsat, erişilebilirlik, yangın ve alan hesabı kuralları
  çizim sunumundan ayrı profildir; sayısal kapı eninden hukuki onay çıkarılmaz.
- Kaynak araştırmasının standart sürümleri geliştirme sırasında doğrulanır;
  eski/geri çekilmiş norm bir etiketle güncelmiş gibi sunulmaz.

## 10. Her dilimin kabul kapıları

1. **Kaynak ve sözleşme:** mevcut aile incelenir; küçük sorumluluklu dosyalar;
   hesap UI'ya kopyalanmaz. Girdi/çıktı/plan/ret ve yan etkiler açık.
2. **Bağımsız hesap:** en az 20 ortak komut durumu; KentOS kullanmayan başvuru;
   ters yön, sınır, büyük koordinat ve geçersiz girdi. Golden güncellemek doğrulama değildir.
3. **Durum ve kalıcılık:** preview yazmaz; tek undo/redo; revision/idempotency;
   kilitli/gizli/uyumsuz katman; KCAD tekrar açma ve mevcut dosyalarda değişmezlik.
4. **Yüzey eşitliği:** katalog/capabilities, desktop/native/headless dispatch,
   Python SDK, MCP ve işlem manifesti. Kullanıcının anlamsal girdisiyle aynı sonuç.
5. **GUI:** gerçek pencere, fare/klavye, önce/önizleme/sonra; açık/koyu,
   1440×900 ve 1100×650, 13/18 px; parametreye yaz, Escape, Enter, sağ tuş,
   zoom/pan ve özellik düzenleme. Görüntü dosyası ve gözle gözlenen sonuç kaydedilir.
6. **Regresyon:** mevcut serbest Duvar, Dikdörtgen, Açıklık, Mahal, ölçüler ve
   ortak izler geçer. Yeni iş, mevcut scale/render/selection kuralını değiştirmez.
7. **Teslim:** ilgili test/clippy, ortak WASM derlemesi, katalog/SDK/manifest drift;
   eski görevlerin testlerini “yeni test edildi” sayma. Yapılmayan kapı açık kalır.

## 11. Uçtan uca örnekler

- **Örnek A — oda:** net 4×3 m oda, 200 mm duvar, 900 mm kapı, 1200 mm pencere;
  net alan 12 m²; duvar/kapı değişince alan, ölçü ve çizelge izlesin.
- **Örnek B — konut katı:** 10×8 m dış boy, 250 mm dış duvar, 150 mm bölmeler;
  salon, mutfak, iki oda, banyo/WC ve koridor; dış/iç ölçü, kot, kapı/mahal
  numarası, donatı ve A3 1:100 pafta. Bu yazılım test verisidir, mimari tasarım önerisi değil.
- **Örnek C — düzenleme:** dış duvarı taşı/kalınlaştır, bir kapıyı kaydır ve
  enini değiştir, bir mahali böl; kopuk ilişki ve eski metraj bırakma.
- **Örnek D — alışveriş:** mevcut kullanıcı DWG'sini aç, mimari ekleme yap,
  KCAD kaydet/aç; kaynak dosyaya yazma. DXF/DWG sonuçlarında görünüş ve
  düzenlenebilirlik/ilişki kaybı ayrı raporlanır.
- Aynı fiziksel model m/mm/cm/inç/ayak gösteriminde ve yerel/uzak koordinatta
  kontrol edilir. Mevcut kullanıcı başlangıç ayarları değişmez.

## 12. Sıra ve çalışma günlüğü

1. Araştırma + güncel kaynak envanteri + gerçek GUI tabanı.
2. Bu planın kaydı; ilk küçük ve bağımsız uygulama: **ARCH-DRAW-01**.
3. Araç ayarları/şerit ve güvenli şablon; sonra bölme ve açıklık yerleşimi.
4. Mahal ve otomatik ölçüler; ardından donatı/merdiven ve belgeleme.
5. Her dilimde bu dosyanın durumu ve kanıtları güncellenir; topluca tamamlandı yapılmaz.

| Tarih | Madde | Yapılan / kanıt | Durum |
| --- | --- | --- | --- |
| 2026-10-24 | Başlangıç | `bb1f3ffd0` temiz main incelendi; yeni dal açıldı; ADR 0626 koruma sınırı belirlendi. | Kaynak incelemesi |
| 2026-10-24 | Araştırma/plan | Üretici referansları ve güncel araç envanteri eşlendi; GUI ve ilk araç kabulü ayrı tutuldu. | Plan yazıldı; uygulama henüz kabul edilmedi |
| 2026-10-24 | ARCH-BASE-01 | Güncel `bb1f3ffd0` derlemesinde gerçek pencerede m/1:100 başlangıç; serbest Duvar'da 4×3 eksen çevresi; fareyle kapı, pencere, mahal; 10.64 m² alan; geri al/yinele ve ayrı KCAD kaydı görüldü. Ekranlar `.run/architecture-ui/shots/baseline-*`. | İlk GUI tabanı kaydedildi; tam plan kabulü sürüyor |
| 2026-10-24 | ARCH-DRAW-01 | `cad.walls.rectangle` ve ayrı araç eklendi. Gerçek pencerede İç net 4×3 m / 200 mm duvar, kapı/pencere/mahal → 12.00 m²; tek duvar undo/redo; yarım taslağı sağ tuşta tek bitirme; ayrı kaydet/aç → aynı 4 nesne. Açık tema 18 px'de Diğer menüsü ve 30° / inç-ayak önizlemesi görüldü. Özellikten 200→250→undo 200 mm; kapı/pencere ve net alan korundu. | [x] İlk aracın işlevsel kabulü tamam; genel mimari yol haritası açık |
| 2026-10-24 | Görsel kanıt | Gerçek pencere 2880×1720–1726 piksel; 13 px koyu ve 18 px açık. Otomatik GPU sahneleri iki tema × 1440×900 / 1100×650. Seçilen gerçek ekranlar ve KCAD dosyaları `docs/baseline/2026-10-24-architecture/`. | Görüntüler gözle incelendi; FPS/titreme ölçümü değil |
| 2026-10-24 | ARCH-PERF-01 | Eşlenik SDK/FFI ölçümünde yeni ve mevcut kapalı duvar planları aynı; 100 bin duvarda yeni komut medyan/p95 20.269/21.099 ms. Ortak O(n) tarama, debug ve eşzamanlı test yükü sınırları kabul kaydında açıklandı. | [ ] Ortak performans iyileştirmesi ayrı iş |
| 2026-10-24 | Son kontroller | Desktop 1261, native entegrasyon 149, interaction 637, MCP 29, headless 1 ve kaynak boy sınırı 1 test geçti. Python tam küme 262; son native pakette yeni 3 test tekrar geçti. İlgili clippy, WASM, katalog/SDK/manifest/şerit ve bağımlılık yönü kontrolleri geçti. | Ayrıntı ve kapsam sınırları kabul kaydında |
| 2026-10-24 | ARCH-UX-01 | Mimari araçların alanları mevcut Öznitelikler panelinde ayrı başlıkla görünür; mevcut seçili nesne özellikleri korunur. Gerçek açık tema / 18 px giriş, menü, geçersiz değer ve Escape denemeleri; 12 GPU ekranı incelendi. | [x] Görünür ayarlar kabulü; genel şerit ARCH-UX-02 açık |
| 2026-10-24 | ARCH-DRAW-02 | Düz bölme komutu, araç, Python ve MCP tamam. Gerçek GUI'de kapılı 48 m² oda → 3 m net bölme → 24 m²; tek Onayla/bitir, undo ve redo. Bağımsız rasyonel referans 90 durum; beş birim, uzak koordinatlar, taşıma/ilişkili ölçü/mahal ve dosya testleri. | [x] Düz dilim; yay ve kalıcı kısıt açık |
| 2026-10-24 | ADR 0657 kontrolleri | Commands 792, native 153, interaction 642, desktop 1263, headless 51, MCP 30, Python 265 geçti. Clippy, WASM, SDK/katalog/manifest/şerit ve bağımlılık yönü geçti. | [Kabul ve kanıtlar](../docs/baseline/2026-10-24-architecture-tools/README.md) |
| 2026-10-24 | ARCH-UX-02 | Beş grup; bağlamsal Özellikler; dar başlıkta etkin sekmeyi açığa çıkarma; taslağı koruyan Yardımcılar menüsü. 46 komut korunur; gerçek GUI ve 20 GPU görünümü; 1269 desktop/278 UI, son 7 hedef testi ve clippy/WASM/tutarlılık kontrolleri. | [x] Sunum dilimi; [kanıt](../docs/baseline/2026-10-24-architecture-ribbon/README.md) |
| 2026-10-24 | ARCH-UX-03 | Şablon fark penceresi; mevcut ayar/font/tanım korunumu; sistemli CAD metre koruması; bütün paket undo/redo; Python/MCP/KCAD. 1274 desktop, 269 Python, 795 sözleşme ve native/host kontrolleri; 19 gerçek ekran/16 GPU. | [x] Güvenli şablon kabulü; diğer mimari maddeler açık |
| 2026-10-25 | ARCH-PERF-01 | Duvar aday dizini; 24 release durumunda eşit geometri/uyarı; 20 soğuk/60 sıcak örnek, bellek ve Iced CPU ayrımı; 100 bin nesnede gerçek önizleme/kalınlık/sağ tık/undo/redo/kayıt. Eski 100012 nesne bağımsız okumada birebir korundu. | [~] Aday araması dilimi; gerçek pencere gecikmesi/GPU ve stil/evre hız matrisi açık |
| 2026-10-25 | ARCH-UX-01 takip / ADR 0661 | Gösterilen birimle tek uzunluk girişi; 13 gerçek ekran, Enter/blur/Escape/ret, tek undo/redo ve kayıt. Bağımsız okumada 100013 eski nesne korundu, tek yeni duvar 0.25 m. Core 770 / interaction 706 / desktop 1274; clippy ve WASM geçti. | [x] Parser takip bulgusu; diğer mimari işler açık |
| 2026-10-25 | ARCH-OPEN-01 / ADR 0662 | Uç/referans, iki pervaz, örnekten tür/en; 40 yeni bağımsız ortak durum, native/Python/MCP; gerçek GUI ve 5 eski nesne/ayarın bağımsız KCAD korunumu. Son kaynakta interaction 712, hedef GUI testleri, clippy/WASM; geniş küme kapsamı kabul kaydında. | [~] Yerleştirme dilimi; birleşik host/tutamaç kabulü, tam boyut matrisi, kaydırma kabulü açık; özet ölçeğinin 1:100 olduğu büyütülmüş görüntüde doğrulandı |
| 2026-10-25 | ARCH-OPEN-01 / ADR 0663 | Sessizce yazmayan merkez tutamacı mevcut cad.openings.edit komutuna bağlandı; birleşik native kabul, 715 interaction / 1274 desktop; gerçek fare/undo/redo/kaydet/aç ve sekiz GPU durumu, bağımsız eski nesne korunumu. | [~] Gerçek tam pencere matrisi ve Kaydır kabulü açık |
| 2026-10-25 | ARCH-BASE-01 / ARCH-OPEN-01 / ADR 0664 | Gerçek olay kaydı ve başarısız eski testle saptanan toplu fare basış konumu düzeltildi; 1279 desktop, 14 viewport, toplu GPU; altı gerçek sürükleme ve mimari yerleşim/tek onay/undo/redo/kayıt-aç. | [~] Tam native matris ve genel taban açık; kenet aday kimliği bağımsız hesap ve App regresyonuyla doğrulandı |
| 2026-10-25 | ARCH-OPEN-01 kenet takibi | Aynı fare hedefindeki 4,772/5,000 m farkının geçerli duvar yüzü Midpoint/#1 adayı olduğu App regresyonu ve bağımsız 60 basamaklı hesapla doğrulandı; önizleme yazmaz, undo korur; üretim kodu/tolerans değişmedi. | [x] Aday kimliği takibi; ana madde [~] |
| 2026-10-25 | ARCH-OPEN-02 / ADR 0665 | Aynı komutta adet/net-merkez aralığı/fit payları ve yeni numaralar; görünür araç, 40 bağımsız durum, analitik native/Python/MCP, gerçek düz/yaylı önizleme/tek onay/undo/redo/çizelge/kayıt-aç; bağımsız KCAD konum ve eski nesne korunumu. | [~] İşlevsel dilim doğrulandı; tam gerçek boyut/font/tema ve birleşik gezinme kabulü açık |
| 2026-10-25 | ARCH-OPEN-02 koyu/18 px | Gerçek K10 dağıtımı, ret/tek onay/undo/redo/çizelge, en düzenle/undo, zoom/pan/kapat-aç; 312 px panelde kısalan yeni seçenek etiketleri sadeleştirildi ve yeniden derlenip fareyle doğrulandı. Bağımsız 12→13→14 dosya/UUID/ayar denetimi. | [~] Bu varyant doğrulandı; tam boyut/font/tema matrisi açık |
| 2026-10-25 | ARCH-DRAW-03 / ADR 0667 | Tam/kısmi düz ve analitik yaylı ortak sınır Eksen ile tek duvar ağı; 24 bağımsız vaka (önceki 69 aynı), beş birim/uzak koordinat/ters yön, host/Python/MCP; gerçek koyu18 ve açık13, dar1100×650, tek onay/undo/redo/KoruSil/ret/kaydet-aç; bağımsız KCAD uzunluk ve kaynak korunumu. | [~] Ortak sınır dilimi; semantik kat, büyük ağ ve tam GUI matrisi açık |
| 2026-10-25 | ARCH-DRAW-03 / ADR 0668 | Açık kat seçimi, katlar arası duvar birleşim ayrımı ve aynı katta tekrar denetimi; 25 yeni bağımsız durum, native/Python/MCP, gerçek açık13 ve koyu18/dar1100×650 seçim ve açık13 geniş tek onay/undo/redo/ret/kaydet-aç; bağımsız KCAD kaynak/duvar/ayar korunumu. | [~] Katlı dönüşüm dilimi; geniş karışık kaynak/büyük ağ/tam GUI matrisi ve diğer kat iş akışları açık |
| 2026-10-25 | ARCH-DRAW-03 / ADR 0669 | Aynı noding hesabında seyrek sahiplik ve 400 sınırında kesilmeyen kontur önizlemesi; 48 ölçülü karma birleşim, büyük ağ red/green, Python/MCP; gerçek 1600 oda→3276 duvar, 11480 m ve karma 7→16/9, tek onay/undo/redo/ret/kayıt-aç, bağımsız KCAD. Çekirdek release soğuk/sıcak süre ve süreç tepe belleği ayrı raporlandı. | [~] Büyük/karışık ağ dilimi; tam GUI matrisi, uçtan uca performans ve diğer kat iş akışları açık |
| 2026-10-25 | ARCH-SPACE-01 / ADR 0670 | Açık kat kimliğine göre mahal sınırı ve görünür kat seçimi; eşit kotta 76,44/22,04 m² ayrımı, kat değişimi plan/follower/tek undo, kayıp sınır; 3210 Rust ve 282 Python, MCP/87 bağımsız durum; 25 gerçek görüntü, dar 1100×650 ve bağımsız KCAD korunumu. | [~] Kat önkoşulu doğrulandı; toplu bulma/kolon/sanal sınır ve tam GUI matrisi açık; [kanıt](../docs/baseline/2026-10-25-architecture-space-levels/README.md) |
| 2026-10-25 | ARCH-SPACE-01 / ADR 0671 | Önizlemeli toplu mahal, açık kat/bölge ve mevcut/çakışan oda ayrımı; tek onay/undo, 34 bağımsız ortak durum, Python/MCP. 4007 Rust başarılı/279 atlanan, 285 Python; 39 gerçek görüntü, 400 oda tek adım, bağımsız KCAD korunumu; 400 seed tam eski/yeni plan eşitliği ve ayrı release çekirdek sorgu ölçümü. | [~] Toplu bulma dilimi doğrulandı; kolon/sanal sınır, uçtan uca performans ve tam GUI matrisi açık; [kanıt](../docs/baseline/2026-10-25-architecture-space-discovery/README.md) |

| 2026-10-25 | ARCH-SPACE-01 / ADR 0672–0673 | Kalıcı kolon düşümü, analitik iz/kat ayrımı, aks→kolon→mahal→etiket/çizelge, tek undo/redo ve kayıt/aç; gerçek GUI’de saptanan daire dolgu ve GPU konum kusuru düzeltildi. Uyarı panelinin sekiz tema/boyut/yazı varyantı ve 47 ham ekran; bağımsız KCAD alan/kimlik/ayar korunumu. | [~] Kolon dilimi doğrulandı; kalıcı sanal sınır, kaynak alt kümesi, uçtan uca performans ve tüm iş akışlarının tam matrisi açık |
| 2026-10-25 | ARCH-SPACE-01 / ADR 0674 | Kalıcı atanmamış duvar/kolon kapsamı ve görünür İç nokta seç; duvar gövdesi seed reddi, kaynak katı/follower, tek onay/undo/redo, opt-in KCAD 104, Python/MCP. 90 bağımsız durum, 292 Python, 56 ham görüntü ve 29 bağımsız GUI kaydı; dar 1100×650, geniş yaklaşık 1440×825, iki tema/yazı. | [~] Bu kapsam dilimi doğrulandı; kalıcı kaynak UUID listesi/sanal ayırıcı, performans ve tüm iş akışlarının tam matrisi açık; [kanıt](../docs/baseline/2026-10-25-architecture-space-scope/README.md) |

| 2026-10-25 | ARCH-BASE-01 / ADR 0675 | Dar CAD ölçülerinde tam basamaklar; gerçek GUI’de bulunan mahal yüksekliği birim eki düzeltmesi. 22 ham görüntü, sekiz görünüm, 11 bağımsız KCAD, tek Enter/ret/Escape/undo/redo; son özellikler 39 başarılı/4 atlanan, odaklı UI 3 başarılı, derleme/clippy geçti. | [~] Bu dar dilim doğrulandı; diğer alanların birim/ad denetimi ve tam mimari matris açık; [kanıt](../docs/baseline/2026-10-25-architecture-measurements/README.md) |
| 2026-10-25 | ARCH-BASE-01 / ADR 0676 | Duvar/açıklık özelliklerinde açık metrik/imperial uzunluk; gerçek eski/yeni GUI, 29 ham görüntü, 26 bağımsız KCAD, tek Enter/ret/Escape/undo/redo/boşaltma; 40 özellik başarılı/4 atlanan, derleme/clippy geçti. | [~] Diğer mimari nesne uzunluk alanları ve tam matris açık; [kanıt](../docs/baseline/2026-10-25-architecture-property-units/README.md) |
| 2026-10-25 | ARCH-SPACE-01 / ADR 0678 | Kalıcı duvar/kolon UUID alt kümesi; tek/toplu seçim ve mevcut mahal kaynak editörü, tek onay/undo, kaynak kaybında son sınır, kopya/dizi/pano yeniden eşleme. Python/MCP, opt-in KCAD 105 ve gerçek DXF import eşlemesi; 63 bağımsız ortak durum, 294 Python, 53 ham görüntü ve 30 bağımsız GUI kaydı. | [~] Kaynak seçimi dilimi doğrulandı; sanal ayırıcı, uçtan uca performans ve genel iş akışlarının tam matrisi açık; [kanıt](../docs/baseline/2026-10-25-architecture-space-sources/README.md) |

| 2026-10-25 | ARCH-SPACE-01 / ADR 0679 | Kalıcı sıfır genişlikli Mahal ayırıcı; kat/kaynak/follower, tek onay/undo, Python/MCP, KCAD 106 ve DXF. Gerçek GUI’de kopuk mahalın yeşil Metraj uyarısı ve büyük yazı sütun çakışması düzeltildi; 105 bağımsız durum, 300 Python, 35 GUI kaydı, 76 ham görüntü ve iki gerçek CSV. | [~] İşlev dilimi doğrulandı; ölçülmüş metraj darboğazı, genel araç/girdi denetimi ve tam iş akışı matrisi açık; [kanıt](../docs/baseline/2026-10-25-architecture-space-separators/README.md) |
| 2026-10-26 | ARCH-SPACE-01 / ARCH-PERF-01 / ADR 0680 | Aynı immutable metraj sorgusunda kaynak/ağ paylaşımı, 100 tam eski/yeni cevap eşit; 300 Python, 268 application/103 domain/6 MCP. 400 mahal/21.000 nesnede sıcak debug ortanca 8646,943→83,765 ms, ilk yeni sorgu 815,623 ms. Gerçek Metraj genişlik/sağ-alt kenar düzeltmesi, 22 ham görüntü ve bağımsız CSV/KCAD. | [~] Bu sorgu/yerleşim dilimi doğrulandı; release/bellek/UI/GPU ve genel tam matris açık; [kanıt](../docs/baseline/2026-10-25-architecture-quantity-cache/README.md) |
| 2026-10-26 | ARCH-DOC-02 / ADR 0681 | CSV seçili sekmeyi yazar; 200/400/0 satır, XLSX/ODS üç sayfa/600. Gerçek iptal/geçmiş/kayıt-aç, bağımsız CSV/XML/KCAD, 17 ham görüntü; üç desktop testi, clippy/build. Eski ham metraj değerlerinin uzun sayı koruma notu kaydedildi. | [~] Bu çıktı dilimi doğrulandı; gruplama/evre/tip, sayı biçimi ve genel tam matris açık; [kanıt](../docs/baseline/2026-10-26-architecture-quantity-export/README.md) |
| 2026-10-26 | ARCH-SPACE-02 / ADR 0682 | Kimliği koruyan tıkla/yönle numaralandırma, görünür önizleme, tek onay/undo, etiket/çizelge ve Python/MCP/kayıt-tekrar; 68 bağımsız durum, 300 Python. Gerçek iç seçim kusuru ve uzun önizleme yazısı düzeltildi; 52 ham ekran, 20 disk anlık görüntüsü bağımsız KCAD kontrolünde. | [~] Numaralandırma dilimi doğrulandı; numara/ad listeleri, daha zengin etiketler ve tam matris açık; [kanıt](../docs/baseline/2026-10-26-architecture-space-number/README.md) |
| 2026-10-26 | ARCH-BASE-01 / ADR 0683 | CommandLine ve EditCell odaktayken Cmd-S harf eklemeden mevcut Kaydet yoluna ulaşır; onaylanmamış tampon/araç önizlemesi korunur. Gerçek GUI ikinci giriş yolunu yakaladı; 38 test, clippy/build, 15 ham görüntü, dokuz bağımsız disk kopyası geçti. | [~] İki bileşenli düzeltme doğrulandı; diğer girişler ve tam matris açık; [kanıt](../docs/baseline/2026-10-26-commandline-save/README.md) |
| 2026-10-26 | ARCH-SPACE-02 / ADR 0684 | Gerçek etiket tutamacının boş geometri yazımı mevcut anlamsal Move'a yönlendirildi; sıcak alt küme ve kaynak bağı korunur. Duvar/mahal silme, 40 cm→20.16 m² zinciri, şablon/undo/redo/iptal/kaydet-aç gerçek fareyle doğrulandı; 692 interaction/219 application/303 Python, 22 ham görüntü ve 18 bağımsız dosya. | [~] Etiket yeri/izleme dilimi doğrulandı; listeler, Koru/Yenile, kot şablonları ve tam matris açık; [kanıt](../docs/baseline/2026-10-26-architecture-tag-follow/README.md) |
| 2026-10-26 | ARCH-UX-01 / ARCH-SPACE-02 / ADR 0685 | Görünür yeni etiket ayarları; GUI’deki ilk yapıştırma ve uzun komut düğmesi kusuru düzeltildi. 697 interaction/40 UI-host/üç mevcut ayar/üç mevcut Python; clippy/build, 26 ham ekran/11 bağımsız tam KCAD. | [~] Panel dilimi doğrulandı; eski panel kaydırması için yeni takip, zengin etiketler ve tam matris açık; [kanıt](../docs/baseline/2026-10-26-architecture-tag-settings/README.md) |
| 2026-10-26 | ARCH-UX-01 takip / ADR 0686 | Yeni ayarlı araç başlarken Özellikler başlangıcı gösterilir; aynı araçta düzenleme/önizleme kaydırmayı korur. Eski 508 px hata, 62 ilgili test, clippy/build; 19 ham ekran/beş bağımsız tam dosya. | [x] Kaydırma takip işi doğrulandı; genel docking/matris açık; [kanıt](../docs/baseline/2026-10-26-architecture-settings-scroll/README.md) |
| 2026-10-26 | ARCH-SPACE-02 / ADR 0687 | Bilinen kat kotu ve yalnız açık mahal yüksekliğinden tavan şablonu; eksik veri boş, mevcut birim/elle konum korunur. Gerçek GUI sayısal yükseklik taşması düzeltildi; 195 ortak durum, 309 Python, 697 interaction/219 application/67 domain/bir MCP, clippy/build/WASM; 19 ham ekran/on bağımsız tam kayıt. | [~] Kot dilimi doğrulandı; ayrı net alan, listeler, Koru/Yenile ve tam matris açık; [kanıt](../docs/baseline/2026-10-26-architecture-tag-elevations/README.md) |
| 2026-10-26 | ARCH-SPACE-02 / ADR 0688 | Seçilmiş yöntemden ayrı net alan; aynı kaynak/kat/kolon tercihi, kaynak değişimi ve güncel ayarda metin takibi; elle yer korunur. 225 ortak durum/30 yeni, 318 Python, 68 domain/697 interaction/219 application/iki MCP, clippy/build/WASM/1170 KCAD; 21 ham ekran/15 bağımsız tam kayıt. | [~] Net etiket dilimi doğrulandı; listeler, Koru/Yenile ve tam matris açık; [kanıt](../docs/baseline/2026-10-26-architecture-tag-net-area/README.md) |
| 2026-10-26 | ARCH-SPACE-02 / ADR 0689 | Mevcut etiket denetiminde görünür numarasız mahal seçeneği; ad/kat/UUID, tam metin yinelenen gruplar, eski API varsayılanı ve salt okunurluk korunur. 233 ortak durum/323 Python/üç MCP; gerçek tek Enter/boşalt/undo/redo/Escape/kaydet-aç, 18 ham ekran/12 tam bağımsız dosya; 30.000 nesneli debug sorgu ölçümü. | [~] Numara denetimi doğrulandı; kat öneki/ad/kullanım listeleri, Koru/Yenile ve tam matris açık; [kanıt](../docs/baseline/2026-10-26-architecture-space-identity/README.md) |
| 2026-10-26 | ARCH-SPACE-02 / ADR0690 | Projeyle sıralı ad/kullanım listeleri ve açık kat öneki; serbest metin/kimlik/tek mahal undo korunur. Opt-in KCAD107, eski 1170 dosya aynı; DXF kaybı önceden ve raporda uyarılır. GUI açıklama yerini düzeltti; 385 ortak durum/329 Python/dört MCP/iki desktop, 42 ham ekran/23 tam bağımsız kayıt, yoğun belge kontrolü. | [~] Liste/önek dilimi doğrulandı; Koru/Yenile, çoklu pafta ve tam matris açık; [kanıt](../docs/baseline/2026-10-26-architecture-room-choices/README.md) |
| 2026-10-26 | ARCH-SPACE-02 / ADR0691 | Toplu etiket Koru/Yenile, elle yer/kimlik korunumu ve karma yeni+yenile tek undo; Python/MCP. Gerçek GUI açıklama yerleşimini ve klavye hedef katmanı açığını yakaladı; son ikiliyle tekrar, 35 ham ekran/24 tam model +bir tanısal disk, 31 yeni bağımsız durum/ilgili264, 335 Python. Eski107 durum ve1756 KCAD baytı aynı; clippy/build/WASM/SDK geçti. | [~] Koru/Yenile alt maddesi doğrulandı; çoklu kat/görünüş/pafta ve tam mimari matris açık; [kanıt](../docs/baseline/2026-10-26-architecture-tag-refresh/README.md) |
| 2026-10-26 | ARCH-SPACE-02 / ADR0692 | Toplu etikette açık kaynak katı, host mirası/bilinmeyen ayrımı; Koru/Yenile/önizleme ve tek undo. 26 yeni bağımsız durum/ilgili290, 340 Python/75 MCP; gerçek iki dolu kat ve atanmamış, tek Enter/sağ tuş Onayla/ret/Escape/undo/redo/yeni süreçte açma. GUI seçili değer kesilmesini düzeltti; 43 ham ekran (tanısallar dahil)/27 tam bağımsız model, eski1781 KCAD aynı. | [~] Kaynak katı alt maddesi doğrulandı; çoklu görünüş/pafta ve tam matris açık; [kanıt](../docs/baseline/2026-10-26-architecture-tag-levels/README.md) |
| 2026-10-26 | ARCH-DIM-01 ilk altyapı / ADR0694 | Açıklığın gerçek jamb/başlangıç/bitiş/merkez başvuruları; GUI'de bulunan kesilmiş duvar ucu sahipliği düzeltildi. Gerçek tek onay/undo/redo/genişlik/konum/kalınlık/kopuk bağ/Esc/yeni süreçte açma; 36 ham ekran/21 tam model+dört tanısal kopya, önceki1828 KCAD aynı. 342 Python/76 MCP/809 commands, native takımlar/build/clippy/WASM/SDK geçti. | [~] Nokta/izleme alt maddesi doğrulandı; otomatik sıra üretimi/üyelik/çoklu pafta ve tam matris açık; [kanıt](../docs/baseline/2026-10-26-architecture-opening-dimensions/README.md) |
