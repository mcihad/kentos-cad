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
- [x] Duvar stili kaydı tabloyu ve izleyen duvar/açıklık/etiket/çizelge/ölçüleri
  tek geri alınabilir adımda tutar: [ADR0699](../docs/adr/0699-style-tables-in-undo.md)
  ile `cad.styles.edit` stil stil (kimlikle) aynı adımda yazar, yalnız ad değişimi de
  adımdır; 61 ortak durum (`cad.styles.edit.json`) ve gerçek GUI'de ad kaydı/tek
  Geri al resimleri (`.run/shots/arac-duvar-stili-*`). Aşağıdaki eski not
  kusurun bulunduğu kaydı anlatır.
- Eski kayıt: Duvar stili kaydı tabloyu ve izleyen duvar/açıklık/etiket/çizelge/ölçüleri
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

### ARCH-UX-04 — Etkin kat ve yeni nesnenin katı [x]

- [x] Etkin kat dilimi [ADR0713](../docs/adr/0713-active-level.md): Archicad'in
  geçerli katı (Home Story) gibi; çizim zemin katta açılır, durum çubuğunun Kat
  hücresi ve yeni nesne çizen araçların ilk ayar satırı katı seçer (Katsız da).
  Duvar/dikdörtgen duvar/bölme/kolon/merdiven/rampa/aks ağı etkin katla yazar;
  mahal, toplu mahal, ayırıcı ve iç ölçü zinciri o katta duvar varken izler
  (eski katsız çizimler bozulmaz), elle seçilen kat kalır. Daire yürüyüşünde
  Denetim'in dört “katı yok” bulgusu kalktı. Etkileşim/masaüstü testleri ve
  gerçek GUI resimleri (`.run/shots/arac-kat-*`).
- [x] Merdivenin yüksekliği katın (ADR0713 eki): Merdiven ve Döner merdiven'in
  Yükseklik'i yazılmadıkça etkin katın yüksekliği (“3.060 m · katın”), yazılan
  kalır, boş/“kat” döner; daire yürüyüşündeki `stair_rise_level` uyarısı kalktı.
  Etkileşim testi ve gerçek GUI (`arac-daire-merdiven`).

- [x] Kat görünümü [ADR0715](../docs/adr/0715-level-view.md): Bütün katlar /
  Yalnız etkin kat / Etkin kat ve alt kat (alt kat soluk, kenetlenir, seçilmez;
  Revit altlığı, Archicad izleme başvurusu). Açıklık duvarının, etiket nesnesinin,
  kaplama mahalin, ölçü bağlarının katıyla; üst üste nesnede tıklama etkin katı
  önce alır (kapı/pencere artık alt katın duvarına oturmuyor). Kat menüsü, hücre
  ikonu, Görünüm › Kat görünümü ▾. Testler ve gerçek GUI (`arac-kat-gorunumu-*`).
- [x] Paftada kat [ADR0716](../docs/adr/0716-sheet-map-level.md): pafta haritasının
  kendi Kat'ı (Revit'in kat planı görünümü ve altlığı; görünüm kopyası gerekmez):
  katın ve katsız nesneler, istenirse alt kat kâğıtta yarım ton; ekranın kat
  görünümü paftayı değiştirmez. `setViewport.level`, projede olmayan kata atomik
  ret, ön denetim bulgusu ve düzeltmesi, denetçide Kat ve Alt kat soluk, yerinde
  model düzenlemede seçim/kenet haritanın katıyla. Görünüm penceresinin kapsamı
  artık geometrik (soluk kat kenetlenmiyordu, düzeldi). Testler ve gerçek GUI
  (`arac-pafta-kat-*`).
- [x] Kat planı paftaları [ADR0718](../docs/adr/0718-level-plan-sheets.md): her kat
  için pafta tek adımda (Revit'in New Plan'ı ve Archicad'in otomatik paftası gibi),
  haritası o katı çizer; bütün haritalar katların kutusunda aynı ölçekte, planlar
  kâğıtta üst üste oturur. Numara A-101'den, paftası olan kat atlanır.
  `cad.sheets.levels`, Pafta › Yeni ▾ ve Çıktı › Pafta › Kat planları; 31 ortak durum,
  testler ve gerçek GUI (`arac-pafta-kat-planlari*`).
- [x] Etkin kat dosyada [ADR0720](../docs/adr/0720-active-level-in-the-file.md): çizim
  kaydedildiği etkin katta ve kat görünümüyle açılır (Revit'in son görünümü; `arch.activeLevel`,
  `levelView`, Katsız `null`, KCAD 120 opt-in); kaydetme belgeyi değiştirmez (geri alma yok,
  kaydedilmemiş olmaz), her sekme kendi katını tutar. Kullanılan kat silinmez: her mimari tür
  ve pafta haritası sayılır, kimlikler verilir; Proje ayarları uyarıyı satırların altında
  gösterir, **Nesneleri seç** ile seçtirir, Kaydet bekler. Üst kat / Alt kat (Ctrl+↑/↓);
  çalışan araç (merdiven yüksekliği, mahal araçlarının Kat'ı) katı hemen izler.
- [x] Kat ver [ADR0717](../docs/adr/0717-assign-levels.md): `cad.levels.assign` (yer
  değişmez; Archicad'in Relink Home Story'si gibi). Denetim'in kat bulgularının Düzelt'i
  etkin katı verir; Kata ver (n) ▾: Seç ve göster (önizleme), sonra bir kat, hepsi tek
  adımda. Öznitelikler'de karışık mimari seçimde ortak Kat. Mahal yeni katında kapanmazsa
  uyarı; mahal ayırıcı da kat denetiminde. 32+41 ortak durum, testler ve gerçek GUI
  (`arac-denetim-kat*`).
- [x] Kabul (ADR0720): çok katlı plan kaydet/aç (Katsız ve sekmeler dahil), kat silme
  (her tür, harita, Nesneleri seç) ve yeniden adlandırma, eski katsız çizim (katsız projede
  açılış katı yazılmaz, baytlar aynı), kat değişince araçların ayar ve önizlemesi,
  Python/MCP girdisinde açık kat (`level_not_found`, `level_in_use` kimlikleriyle, açılış
  katı dosyada). 104 ortak durum (13 yeni), KCAD/Python okuyucusu, etkileşim/uygulama/
  masaüstü (1352)/MCP/Python (370) takımları, clippy, gerçek GUI (`arac-kat-dosya-*`,
  `arac-kat-silme*`, `arac-kat-merdiven-izler`).

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
- [x] Numara dilimi [ADR0714](../docs/adr/0714-opening-numbers-on-placement.md):
  Revit'in Mark'ı gibi yerleştirirken numara: kapı K1, K2…, pencere P1…, boşluk
  B1…; yazılan numaradan sırayla, kullanılan atlanır, Yok numarasız; numara komut
  girdisinde. Etiketin yazısı zeminli (duvar üstündeki kapı etiketi okunur).
  3 araç testi, tam takımlar, gerçek GUI (`arac-daire-kapi-numara`, `-etiketler`).
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
- [x] Kapalı ve çok kenarlı duvarın her düz kenarı: tarifin `edge`'i ve
  `around` ile bütün kenarlara tek adımda dizi; kenar köşe ekleme/silmede izlenir,
  izlenemezse dizi kopar (KCAD şema 114). [ADR0700](../docs/adr/0700-dimension-sets-on-wall-edges.md);
  `cad.dimensionsets.*` ortak durumları ve Kenarlar: Bu kenar / Bütün kenarlar,
  tek Enter, tek Geri al resimleri (`.run/shots/arac-olcu-*`). Yay kenar ve pahlı
  köşe açıklamalı rettir.
- [x] Kesen çizgi yöntemi (Archicad iç ölçülendirmesi) [ADR0711](../docs/adr/0711-interior-dimension-chains.md):
  `cad.dimensions.interior` kesen çizginin kestiği duvar yüzlerini (dış yüzler ya
  da bütün katmanlar) ve kolonları (kenarlar ya da merkez) sırayla ölçen, ölçüleri
  yüzlere `cross` kuralıyla bağlı zinciri tek adımda yazar; başka gövdenin içindeki
  kesişim görünmez yüz sayılır, yay yüz atlanıp söylenir. Duvar kayınca değer
  değişir, uzayıp kısalınca zincir çizgisi yerinde kalır (KCAD 118). 44 bağımsız
  durum (Kaydır ve silme izlemesi dahil), Python/MCP, 5 çekirdek+4 araç testi,
  gerçek GUI (`.run/shots/arac-ic-olcu-*`). Üyelik kalıcı değildir; açıklık, aks,
  kiriş ve duvar ekseni noktası açık.
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

### ARCH-EDIT-01 — Geçici ölçüyle doğrudan düzenleme [~]

- [x] Açıklık dilimi [ADR0701](../docs/adr/0701-opening-temporary-dimensions.md):
  seçili tek kapı/pencere/boşluğun imlecin yakın olduğu yüzde en yakın
  referanslara (yüz ucu, aynı kenardaki açıklık, kesen duvar yüzü) mesafeleri;
  sayıya basınca yazı kutusu, Enter `cad.openings.edit` `distance` ile taşır,
  referans sabit, Esc vazgeçer, tek Geri al. Salt okunur
  `cad.openings.distances` Python/MCP'de; 24+24 ortak durum bağımsız
  başvurudan; resimler `.run/shots/arac-gecici-olcu-*`. Duvar ve kolonun geçici
  ölçüsü, kilitli referans ve UCS kabulü açık.
- [x] Duvar dilimi [ADR0710](../docs/adr/0710-wall-temporary-distances.md):
  seçili duvarın imlecin yakın olduğu düz kenarının iki yanında en yakın paralel
  duvarın bakan yüzüne net mesafe; sayıya basıp yazınca kenar kendine dik kayar,
  köşedeki duvarın ucu, T ile saplanan duvarlar ve bitişik köşeler izler,
  açıklıklar yerinde kalır, tek Geri al. `cad.walls.edit` `shift`/`distance` ve
  salt okunur `cad.walls.distances` Python/MCP'de; 22+32 ortak durum bağımsız
  başvurudan, 4 araç testi, resimler `.run/shots/arac-duvar-mesafe-*`. Kolonun
  geçici ölçüsü, duvar boyu, kilitli referans ve UCS kabulü açık.
- [x] Kolon dilimi [ADR0712](../docs/adr/0712-column-temporary-distances.md):
  seçili kolonun dört yanında aynı kattaki en yakın paralel yüze (duvarın bakan
  yüzü ya da öbür kolonun yanı) net mesafe; sayıya basıp yazınca kolon o yana dik
  kayar, karşı yüz sabit, kolonu düşen mahal aynı adımda izler, tek Geri al.
  Akslara bağlı kolonda sayıya basınca neden ve çözüm söylenir. `cad.arch.edit`
  `distance` ve salt okunur `cad.columns.distances` Python/MCP'de; 22+23 ortak
  durum bağımsız başvurudan, 3 çekirdek+4 araç+2 SDK testi, resimler
  `.run/shots/arac-kolon-mesafe-*`. Üç geçici ölçüde sayı çizgiyi örter ve
  tıklama en yakın sayıya gider. Aksa mesafe, duvar boyu, kilitli referans ve
  UCS kabulü açık.

- Seçili kapı/duvar/kolonun yakındaki referansa mesafesini göster; sayıya basıp
  hedef mesafe yazılabilsin. Sabit kalan taraf/ankraj açık olsun.
- Ölçüyü değiştirme ve nesneyi değiştirme farklı eylemdir; mevcut ölçü değeri
  metnini sahte olarak override ederek model taşınmış gibi gösterme.
- Kabul: kilitli referans, iki aday, kaydırılmış UCS, büyük koordinat, Escape,
  tek undo; aynı anlamsal mesafe komutu Python/MCP'den çalışır.

## 7. P1/P2 — eksik mimari eleman ve donatı araçları

### ARCH-LIB-01 — Mobilya ve vitrifiye galerisi [~]

- [x] İçerik dilimi [ADR0702](../docs/adr/0702-furniture-and-fixture-library.md):
  KentOS kitaplığına gerçek boylu 21 blok (yatak, komodin, koltuk, kanepe, masalar,
  depolama, asma klozet, pisuvar, köşe duş, dolaplı lavabo, beyaz eşya, mutfak
  dolapları), vitrifiyelerde boşluk zarfı görünürlüğü (mevzuat iddiası değil),
  boyut parametreleri; Blok ekle'de Duvara hizala (H, yalnız duvarlı çizimde):
  taban yüzde, ön odaya dönük, dönüş “duvara göre”. Resimler
  `.run/shots/arac-mobilya-*`. DWG blok kategorisi, önizlemede en/boy etiketi
  ve m/mm/DXF kabul matrisi açık.

- Yatak, masa, sandalye, koltuk, dolap; klozet, lavabo, duş/küvet; evye, ocak,
  buzdolabı, çamaşır makinesi için gerçek boylu 2B kitaplık.
- Mevcut blok/dinamik blok ve kategori galerisi kullanılsın. Kapıdan büyük yatak
  üretecek belirsiz blok birimi yok; sembol birimi, fiziksel en/boy, yerleşim
  ankrajı ve dönüş önizlemede gösterilsin.
- Boyut varyantı, aynala, döndür, duvara hizala ve isteğe bağlı boşluk zarfı.
  Boşluk zarfı otomatik mevzuat uygunluğu iddiası değildir.
- Kabul: m/mm proje, dış DWG bloğu, iç içe INSERT, aynalı yerleşim, gerçek
  ölçüm, blok özellikleri, kaydet/aç ve DXF görünüşü.

### ARCH-LIB-02 — Mutfak/dolap dizisi ve tezgâh [~]

- [x] Düz dizi dilimi [ADR0706](../docs/adr/0706-kitchen-casework-runs.md):
  `cad.casework.run` çizgi boyunca modül enli dolaplar, 0,90 m evye modülü,
  ocak, isteğe bağlı üst dolap, kalan için görünür dolgu ve uyarı, tezgâh alanı;
  tek geri alma adımı, kitaplık blokları gerekirse aynı adımda. Araç başı, sonu
  ve tarafı tıklatır; Otomatik evye/ocak seçtiği modülü gösterir. 36 ortak durum
  bağımsız başvurudan (dolgu, sol/sağ, eğik, modül/derinlik retleri, çizimin
  kendi bloğu), araç testleri, Python/MCP, gerçek GUI resimleri
  (`.run/shots/arac-mutfak-*`). Köşe/L-U tek komut, cihaz modülleri, tezgâh
  deliği ve duvar izleme açık.

- Düz/L/U dizi; modül eni/adet, köşe çözümü, tezgâh derinliği; evye/ocak boşluğu,
  dolap açılma yönü ve yerleşim duvarına göre konum.
- Parametreli bileşen veya kaynaklı blok grubu; tür seçimi tabloyla izlenebilir.
- Kabul: toplam uzunluk/modül dağılımı, dar köşe, negatif kalan, çakışma,
  tek modül düzenleme ve kaynak duvar taşınması. İlk iş basit düz diziyle sınırlı.

### ARCH-ELEM-01 — Döşeme, balkon, teras ve şaft sınırı [x]

- [x] Şaft/döşeme boşluğu dilimi [ADR0708](../docs/adr/0708-space-voids.md):
  ikinci alan motoru yerine mahalin `void` işareti (KCAD şema 116, opt-in);
  duvarlardan bulunur ya da çizilir, planda kırpılmış çapraz, Metraj'da oda
  toplamı dışı ve ayrı boşluk toplamı, denetimde küçük/dar bulgusu yok. Mahal
  aracında Boşluk (V), Öznitelikler'de satır. Revit Shaft Opening ve Archicad
  Area List araştırıldı. Ortak durumlar bağımsız başvurulardan, KCAD/Python/
  araç testleri, gerçek GUI resimleri (`.run/shots/arac-saft-*`). Döşeme
  konturu/kot, balkon ve korkuluk, katlar arası şaft ve çizelge toplamı açık.
- [x] Kalan bütün alt maddeler ve kabul [ADR0722](../docs/adr/0722-slabs.md): yeni
  `slab` türü mahalin şekliyle (ikinci alan motoru yok): Döşeme/Balkon/Teras, kalınlık,
  kata göre kot farkı, malzeme, metraj dışı; sınır çizilir, duvarların dış yüzlerinden
  ya da odanın iç yüzlerinden bulunur (kat verilmişse o katın duvarları). Kenar ötelemesi
  dört başarısızlıkla atomik ret ve önizlemede kırmızı uyarı. Korkuluk/parapet yalnız
  serbest kenarlarda türer, duvarla izler. Metraj: döşeme alanı, şaft (boşluk mahali)
  düşülmüş net alan, hacim, üst kot, kenar boyu; mahalde süpürgelik (kapı ve döşemeye
  inen boşluk düşülür) ve kapıda kaplama geçişleri. Kes/Ekle ile ıslak hacim deliği ve
  avlu. KCAD 123, DXF (mimari INSERT), sunucu göçü 0034, Python/MCP; 47+38 ortak durum
  bağımsız başvurudan, gerçek GUI resimleri (`.run/shots/arac-doseme-*`). Katmanlı yapı,
  kesit/3B ve katlar arası tek şaft açık (ADR'nin Açık kalanları).

- Kapalı kontur, kenar ötelemesi, delik/boşluk, kat/kot ve malzeme/kaplama bilgisi.
- Mevcut polygon/hatch/mahal modelleriyle nelerin temsil edilebildiğini incele;
  gerekli değilse ikinci alan motoru veya yeni dosya türü kurma.
- Korkuluk/parapet 2B çizgisel sembolü; kaplama geçişi ve süpürgelik gibi
  çizgisel bitişler kaynak boyunca türesin.
- Kabul: L kontur, avlu, ıslak hacim deliği, şaft, offset başarısızlığı,
  metraj dahil/harici seçimi ve açık kayıp raporuyla temel CAD'e verme.

### ARCH-ELEM-02 — Niş, baca, tesisat boşluğu [x]

- [x] Bütün madde [ADR0719](../docs/adr/0719-niches-flues-service-holes.md): niş ve baca
  açıklığın kendi biçimleri (Archicad'in sınırlı açıklığı; yüz duvarın yüzü): niş bir
  yüzden `depth` içeri, baca duvarın içinde `inset` (yoksa ortada); kalınlığı aşan
  derinlik `recess_too_deep`, sonradan incelen duvarda `tooDeep`. Boşluk/niş/baca kesme
  yüksekliğini (1,10 m) kesiyorsa duvarı keser (nişin arkası ve kalan dolgu, bacanın
  çaprazı; yaylı duvarda merkeze göre yay), kesmiyorsa kesik çizgi; tesisat boşluğu türü.
  Kotları ayrı ve plan birini kesiyorsa aynı yerde iki açıklık (pencere altı kalorifer
  nişi). Metrajda niş/baca duvardan düşülmez, derinliği satırda; ölçü dizisi nişi kendi
  yüzünde ölçer. Boşluk ▾ (Boşluk/Niş/Baca), araç ayarları ve Öznitelikler; KCAD 119.
  Ortak durumlar (16 geometri, 33+17 komut, 2 ölçü, 2 metraj), testler ve gerçek GUI
  (`arac-nis-*`, `arac-baca-*`). Evre ve duvar taşıma açıklığın kuralıyla.

- Duvar içinde niş/kısmi oyuk ile tam geçişli boşluk ayrımı; genişlik, derinlik,
  yüz, kot ve gösterim. Mevcut kapı boşluğunu farklı anlamda sessizce kullanma.
- Kabul: kalınlığı aşan derinlik reddi, açıklıkla çakışma, evre, duvar taşıma ve
  2B görünüş/metraj sözleşmesi. 3B katı sonucu bu aşamada vaat edilmez.

### ARCH-ELEM-03 — Aks ve kolon yerleşimini güçlendir [~]

- [x] Aks ağı dilimi [ADR0707](../docs/adr/0707-grid-systems.md): `cad.grids.create`'in
  `system`'i iki yönde aralık listeleri, açı, ilk adlar, uzatma, balonlar ve
  kesişimlere akslarına bağlı kolonlar tek adımda; Aks ağı aracı (Aks ▾)
  imleçle gelir, tıklayınca yerleşir, İki nokta açıyı imleçten alır, “3*6”
  aralık yazımı. Archicad Grid System araştırıldı. 36 ortak durum bağımsız
  başvurudan, araç testleri, Python/MCP, gerçek GUI resimleri
  (`.run/shots/arac-aks-agi-*`). GUI'de Enter'ın aynı yere ikinci ağ yazması
  bulunup düzeltildi. Eğrisel ağ, kolon hizası, L kolon ve eşit bölme açık.

- Aks aralık listesiyle ortogonal/döndürülmüş ağ, iki uç balonu, kesik aralık;
  aks kesişimine kolon, dikdörtgen/daire/L varyant ve hiza.
- Mevcut aks/kolon/numaralama komutları genişletilir; kat bilgisi korunur.
- Kabul: sıra atlama, yinelenen aks, negatif yön, kolon dönüşü, kilitli katman,
  referans kopması ve ölçü/çizelge güncellemesi.

### ARCH-STAIR-01 — Kullanılabilir 2B merdiven tasarımı [x]

- [x] Yükseklikten tasarım dilimi [ADR0704](../docs/adr/0704-stairs-from-height.md):
  `cad.stairs.create` toplam yükseklik ve en yüksek rıhttan rıht sayısını
  (ya da verilen sayıdan rıht yüksekliğini), plan kesme yüksekliğinden kesmeyi
  bulur; son kol son rıhtta biter (`fit`). Kolları bir eni kadar aralıklı U'nun
  iki dönüşü tek sahanlık; kesmenin üstündeki rıhtlar kesikli. Araç bir
  tıklama ve yönle çalışır, değerler Öznitelikler'de görünür, önizleme komutun
  planıdır (rıhtlar, sahanlık, kesme, ok). Revit/Archicad/Vectorworks
  araştırıldı; öntanımlılar yönetmelik değeri değildir. 30 yeni ortak durum
  ve çekirdekte 14 merdiven durumu bağımsız başvurudan, 3 araç testi, gerçek
  GUI resimleri (`.run/shots/arac-merdiven-*`). Kat görünümü, dairesel/döner
  merdiven, kat yüksekliğine bağlı izleme ve kurum profili açık.
- [x] Döner merdiven dilimi [ADR0709](../docs/adr/0709-curved-stairs.md):
  iki noktalı yürüme çizgisi yay (`bulges`, KCAD şema 117 opt-in); rıhtlar yay
  boyunca radyal, bant iki eş merkezli yay, korkuluklar yay, kesme ve kesikli
  üst; `fit` yayı son rıhtta bitirir, bir turdan uzunu reddeder; ayna yayı
  çevirir. Döner merdiven aracı (Merdiven ▾ ve Biçim: Döner): merkez, başlangıç,
  imleçle birikerek dönen yön, yazılan dönüş; etikette ve Öznitelikler'de
  yarıçap, sola/sağa dönüş ve iç kenardaki basamak (bilgi). Revit Center-Ends
  Spiral ve Archicad Along Arc araştırıldı. 10 çekirdek + 26 komut durumu
  bağımsız başvurudan, KCAD/Python/MCP, 5 araç testi ve panelin her seçimini
  uygulayan regresyon testi (Sağ korkuluk/Tıklanan yerde/Yan: Sağ'ın panelden
  uygulanamaması bu dilimde bulunup düzeltildi), gerçek GUI resimleri
  (`.run/shots/arac-doner-merdiven-*`). Merkez direği, yay+düz karışık kol ve
  iç basamak denetimi açık.
- [x] Kat yüksekliği, yönetmelik profili, üst katta gösterim, merkez direği ve
  metraj [ADR0724](../docs/adr/0724-stair-levels-profiles-upper-plan.md):
  `heightFromLevel` merdiveni katının yüksekliğiyle tasarlar, kat yüksekliği ya
  da rıht sayısı değişince belge rıht yüksekliğini yazar (KCAD 125), araç Etkin
  kat'ı izlerken bağlar; döner merdivende merkez direği (`post`, M). İmar
  Yönetmeliği md. 31'in dört profili (`catalog/v1/arch-profiles/`, mevzuat onayı
  değil) ve Denetim'in `stairs` grubu: rıht, basamak, 2a+b, kol, sahanlık, dar
  uç, korkuluk; üçü Düzelt'li; `profiles` bir rampa ve bir merdiven profili
  alır. Alttaki katın kesmeli merdiveni etkin katta yukarıdan çizilir (seçilmez,
  kenetlenir); pafta haritası kendi katıyla çizer, ekranın kat görünümü paftayı
  değiştirmez. Metraj'da merdiven satırları ve Merdivenler sekmesi. 34+33+21
  ortak durum ve çekirdekte üst kat ile direk bağımsız başvurulardan; gerçek PDF
  (1. kat paftası merdiveni yukarıdan çizer, `pdftoppm`); gerçek GUI resimleri
  (`.run/shots/arac-merdiven-*`). Rıht sayısının yeniden hesaplanması, çok katlı
  merdiven ve kesit/3B açık.

- Düz/L/U ve sahanlık; toplam yükseklik, rıht sayısı, basamak derinliği, kol eni;
  değiştirilen parametre ve sabit tutulan kural açık olsun.
- Çıkış oku, basamak numarası, kesme çizgisi, kesme üstü kesik çizgi ve kat
  görünümü. Dairesel/döner merdiven ayrı sonraki dilim.
- Formül önerileri kurum/standart profili olarak adlandırılır; otomatik güvenlik
  veya ruhsat uygunluğu iddiası yok.
- Kabul: sayı/gerçek geometri tutarlılığı, başlangıç/bitiş yönü, aynala,
  sahanlık çakışması, pafta gösterimi ve 2B ölçülebilir çıktı.

### ARCH-STAIR-02 — Rampa ve korkuluk [x]

- [x] Eğim ve korkuluk dilimi [ADR0705](../docs/adr/0705-ramps-by-slope-and-railings.md):
  rampa eğimden tasarlanır (`fit`: son kol |kot farkı| ÷ eğim uzunluğunda biter,
  inen rampa da), üst üste binen sahanlıklar bir kez sayılır; merdiven ve rampa
  sol/sağ/iki yan korkuluk taşır (KCAD şema 115, opt-in), merdivende kesmenin
  üstü kesikli, iç korkuluk boşluğu dolanır. Araçlarda ve Öznitelikler'de
  görünür. 38 çekirdek, 22+6 komut durumu bağımsız başvurudan; KCAD/Python/
  araç testleri; gerçek GUI resimleri (`.run/shots/arac-rampa-*`).
- [x] Ara sahanlık, kata bağlı başlangıç, metraj ve erişilebilirlik profili
  [ADR0723](../docs/adr/0723-ramp-landings-levels-accessibility.md): `maxRun`
  (En uzun kol) aşan kola düz köşe ve sahanlık ekler; `baseOffset` başlangıcı
  katın kotu + fark yapar, kat değişince belge izler (KCAD 124), araçta
  Başlangıç kotu (Z) etkin katı izler; Metraj'da rampa satırları ve Rampalar
  sekmesi; sürümlü `erisilebilirlik-rampa/1` (TS 9111 özeti, mevzuat onayı
  değil) ile Denetim'de eğim bandı, uzun kol, kısa sahanlık ve dar dönüş, son
  ikisi Düzelt'li; planda okun yanında eğim yazısı. 31+27+22+24 ortak durum
  bağımsız başvurulardan, gerçek PDF (`pdftotext`: %5.00, kat yükselince
  +3.25 → +4.45), gerçek GUI resimleri (`.run/shots/arac-rampa-sahanlik-*`).
  Genişlik/küpeşte profili, döner rampa ve kesit/3B açık.

- Eğim, uzunluk, kot farkı ve sahanlık arasından açık sürücü parametreler;
  çıkış yönü, kot etiketi ve korkuluk kenarı.
- Mevcut Rampa'nın veri modeli/komutu kullanılır. Yerel erişilebilirlik
  yönetmeliği ayrı sürümlü profil; genel CAD aracı mevzuat onayı vermez.
- Kabul: ters yön, sıfır kot farkı, sınırdaki eğim, kaynak kot değişikliği,
  basılı gösterim ve metraj.

## 8. P2 — belgeleme, denetim ve çıktı

### ARCH-DOC-01 — Mimari semboller ve notlar [x]

- [x] Görünüm işaretinin pafta numarası [ADR0721](../docs/adr/0721-architectural-symbols.md) §1:
  kesit/görünüş/detay işaretinin balonu hedef paftanın çizim numarasını (yoksa adını) okur;
  yeniden numaralama ve kopya izler, pafta silinince `targetGone` ve Denetim. Plan işareti kesit
  geometrisi üretmez.
- [x] Kata bağlı kot (§2): `level {id, offset?}`, KCAD 121; kat yükselince izler, kullanılan kat
  silinmez; Kot aracında Kat ve Fark, Öznitelikler'de Kot/Kat/Fark.
- [x] Kuzey oku (§3): `cad.north.create`, kitaplığın bloğu aynı adımda; plan, coğrafi (meridyen
  yakınsaması; yerelde `no_true_north`) ya da açı; kâğıtta 15 mm.
- [x] Revizyon (§4): `cad.revisions.mark` bulut + üçgen (numara, tarih, açıklama); Revizyon işaretle
  aracı; Revizyon listesi sütunlu blok çizelgesi (`columns`, KCAD 122), belge her adımda yazar.
- [x] Malzeme lejandı (§5): `cad.legends.materials`, duvar bileşenlerinin taraması ve adı, yerinde
  yenilenir.
- [x] İnceleme hatası (§6): stil verilen duvarın yüzüne bağlı iç ölçü zinciri artık kopmuyor (yüzler
  ilk ve son çizgi).
- [x] Kabul: iki pafta, A-101 → A-201 yeniden numaralama, kopyalanmış işaret, PDF'te sözcükler ve
  gömülü yazı tipleri (`pdftotext`, `pdffonts`), silinen pafta; Python/MCP; gerçek GUI
  (`arac-isaret-*`). Pafta başına revizyon çizelgesi ve proje revizyon tablosu açık.

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

### ARCH-DOC-03 — Mimari proje denetimi [~]

- [x] İlk dilim [ADR0703](../docs/adr/0703-architectural-project-audit.md):
  salt okunur `cad.arch.audit` ve alt panelin Denetim sekmesi; birleşmeyen
  duvarlar, öbür duvara/uca birkaç santim kala biten uçlar (kat kat), kopuk
  açıklık/mahal/ölçü/dizi/kolon/işaret, çok küçük/dar mahal, yinelenen numara,
  katsız nesne. Düzeltmesi açık bulgu var olan komutun çağrısını taşır;
  satıra bas-yakınlaş-Düzelt-yeniden denetle-Geri al gerçek GUI resimleriyle
  (`.run/shots/arac-denetim-*`), 35 ortak durum bağımsız başvurudan, aynı
  rapor Python/MCP'de. Revit Review Warnings ve Archicad Update Zones
  araştırıldı. Kalıcı istisna, kat/seçim kapsamı ve merdiven denetimi açık.

- Açık duvar sınırı, çakışan açıklık, kopuk host/ölçü/etiket, yinelenen numara,
  eksik tip/kat, sıfır alan ve çok küçük artık parça için odaklanılabilir liste.
- Mevcut birleşim ve etiket denetimleri bir araya getirilir; otomatik onarım
  yalnız açık önizleme ve seçili bulgular için. Kaynak geometriye sessiz snap yok.
- Kabul: bir bulguya tıkla-yakınlaş-düzelt-yeniden denetle, undo ve aynı raporun
  Python/MCP'den alınması. “Hata yok” ruhsat/standart uygunluğu değildir.

### ARCH-DOC-04 — Pafta takımı ve ofis profili [~]

- [x] Kat planı paftaları [ADR0718](../docs/adr/0718-level-plan-sheets.md): katlar
  için A-101'den numaralı, “Mimari › Kat planları” alt kümesinde, ortak zeminli ve
  aynı ölçekli plan paftaları (sistem ya da projenin şablonundan, soruları bir kez).
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
| 2026-10-27 | ARCH-DIM-01 ve GUI düzeltmeleri / ADR0695–0698 | Başka ajanın dilimleri: kalıcı ilişkili duvar ölçü dizisi (KCAD 113), yakınlaştırmaya göre duvar ayrıntısı, pencereye sığan uygulama menüsü, dar pencerede Duvar stilleri. İnceleme: CLAUDE.md özetleri eksikti (eklendi); stil adının geri alınmaması ADR0699'da kapandı. | [~] Nihai kabul ADR'lerinde açık |
| 2026-10-27 | ARCH-BASE-01 / ADR0699 | Stil tablosu izleyen nesnelerle aynı geri alma adımında; yalnız ad değişimi de adım. 61 ortak durum, gerçek GUI ad kaydı ve tek Geri al resimleri. | [x] Stil geri alma alt maddesi |
| 2026-10-27 | ARCH-DIM-01 / ADR0700 | Kapalı/çok kenarlı duvarın her düz kenarına ölçü dizisi (`edge`, `around`), kenar köşe değişiminde izlenir; KCAD 114. Ortak durumlar ve Bu kenar/Bütün kenarlar resimleri. | [x] Kenar alt maddesi; yay kenar açık |
| 2026-10-27 | ARCH-EDIT-01 / ADR0701 | Seçili açıklığın geçici ölçüleri; sayıya bas-yaz-Enter tek adım, referans sabit. `cad.openings.distances` + 24+24 ortak durum, resimler. | [~] Açıklık dilimi; duvar/kolon açık |
| 2026-10-27 | ARCH-LIB-01 / ADR0702 | 21 gerçek boylu mobilya/vitrifiye/mutfak bloğu, boşluk zarfı, Blok ekle'de Duvara hizala (H). Kitaplık denetimi ve resimler. | [~] İçerik dilimi; DWG kategori ve etiket açık |
| 2026-10-27 | ARCH-DOC-03 / ADR0703 | `cad.arch.audit` ve Denetim sekmesi: bul-yakınlaş-Düzelt-yeniden denetle-Geri al; 35 ortak durum, Python/MCP, resimler. | [~] İlk dilim; istisna ve merdiven denetimi açık |
| 2026-10-27 | ARCH-STAIR-01 / ADR0704 | Yükseklikten rıht sayısı, kesme yüksekliği, son kol rıhtlara göre; dar U'da tek sahanlık; kesmenin üstü kesikli. 30+14 ortak durum, 3 araç testi, Python/MCP, clippy; gerçek GUI resimleri. | [~] Tasarım dilimi; kat görünümü ve döner merdiven açık |
| 2026-10-27 | ARCH-STAIR-02 / ADR0705 | Eğimden rampa (son kol eğime göre), sahanlıklar bir kez, merdiven/rampa korkuluğu (KCAD 115); 38+28 durum, KCAD/Python/araç testleri, gerçek GUI resimleri. | [~] Eğim/korkuluk dilimi; ara sahanlık ve profil açık |
| 2026-10-28 | ARCH-STAIR-01 / ADR0724 | Kata bağlı merdiven yüksekliği (`heightFromLevel`, KCAD 125, izleyici), merkez direği (`post`), İmar Yönetmeliği md. 31 profilleri ve Denetim'in `stairs` grubu (`profiles`), üst katta yukarıdan gösterim (ekran ve pafta), Metraj'da merdiven satırları ve Merdivenler sekmesi. 34+33+21 ortak durum, çekirdek üst kat/direk durumları, gerçek PDF (`pdftoppm`), uygulama/alan/etkileşim/komut/sözleşme/geometri/KCAD/başsız/pafta takımları, gerçek GUI. | [x] Bütün madde |
| 2026-10-28 | ARCH-STAIR-02 / ADR0723 | Ara sahanlık (`maxRun`), kata bağlı başlangıç (`baseOffset`, KCAD 124, izleyici), Metraj'da rampa satırları ve Rampalar sekmesi, Denetim'de erişilebilirlik profili (`erisilebilirlik-rampa/1`) ve Düzelt, planda eğim yazısı. 31+27+22+24 ortak durum, gerçek PDF, uygulama/alan/etkileşim/masaüstü (1356)/KCAD/Python (374)/MCP takımları, clippy, gerçek GUI. | [x] Bütün madde |
| 2026-10-27 | ARCH-LIB-02 / ADR0706 | `cad.casework.run`: düz mutfak dizisi (modüller, evye, ocak, üst dolap, dolgu, tezgâh) tek adımda, kitaplık blokları gerekirse aynı adımda; Mutfak dizisi aracı (baş, son, taraf). 36 durum, 3+1 araç testi, Python/MCP, gerçek GUI resimleri. Başlık tablosunun önceki eksikleri (0701, 0703) de kapandı. | [~] Düz dizi dilimi; köşe ve cihaz modülleri açık |
| 2026-10-27 | ARCH-ELEM-03 / ADR0707 | Aks ağı: `cad.grids.create` `system` (iki yön aralık, açı, ilk adlar, uzatma, kesişim kolonları akslara bağlı) tek adımda; Aks ağı aracı (tek tık ya da iki nokta). 36 durum, 3+1 araç testi, Python/MCP, gerçek GUI resimleri; Enter'ın ikinci ağ yazması düzeltildi. | [~] Dik ağ dilimi; eğrisel ağ, kolon hizası ve L kolon açık |
| 2026-10-27 | ARCH-EDIT-01 / ADR0710 | Duvarın geçici ölçüsü: iki yanda paralel duvara net mesafe, sayıya bas-yaz-Enter kenarı kaydırır, köşeler/T'ler/köşe noktaları izler, açıklıklar yerinde; `cad.walls.edit` shift/distance, `cad.walls.distances`. 22+32 durum, 4 araç testi, gerçek GUI resimleri. | [~] Duvar dilimi; kolon, duvar boyu ve kilitli referans açık |
| 2026-10-27 | ARCH-STAIR-01 / ADR0709 | Döner merdiven: yaylı yürüme çizgisi (KCAD 117), radyal rıhtlar, yay korkuluk, yayı son rıhtta bitiren `fit`, ayna; Döner merdiven aracı (merkez, başlangıç, imleçle yön). 10+26 durum, KCAD/Python/MCP, araç ve panel seçimi regresyon testleri; panelin Sağ/Tıklanan yerde seçimlerinin reddi düzeltildi; gerçek GUI resimleri. | [~] Döner dilimi; merkez direği ve karışık kol açık |
| 2026-10-27 | ARCH-ELEM-01 / ADR0708 | Mahal boşluğu (şaft, asansör, döşeme boşluğu): `void` (KCAD 116), çapraz, Metraj'da ayrı toplam, denetim muafiyeti; Mahal aracı ve Öznitelikler. Ortak durumlar, KCAD/Python/araç testleri, gerçek GUI resimleri. | [~] Boşluk dilimi; döşeme, balkon ve korkuluk açık |
| 2026-10-27 | ARCH-DIM-01 düzeltme / ADR0695 eki | Kullanıcı bildirimi: Duvar ölçüleri açıklıksız kenara aynı boyu iki kez yazıyordu (açıklık sırası = toplam). Bir ölçü, sıraların sırasıyla kendinden önce çizilenle aynı iki yeri ölçüyorsa çizilmez; bastırılan parça sayılmaz. 109 ortak durum yenilendi, belge ve araç testleri, gerçek GUI resmi (`arac-olcu-aciksiz-duvar`). | [x] Kusur kapandı |
| 2026-10-27 | ARCH-UX-04 / ADR0720 | Etkin kat dosyada: `arch.activeLevel`/`levelView` (Katsız `null`, KCAD 120), kaydet/kurtarma kopyası ekrandakini yazar, belge değişmez; açılış söyler; sekme başına kat. Kat silme her mimari türü ve pafta haritasını sayar, kimlikleri verir (zaten olmayan kat artık engellemiyor); Proje ayarları'nda uyarı + Nesneleri seç, Kaydet bekler. Üst kat/Alt kat (Ctrl+↑/↓), araçlar katı hemen izler (`Tool::level_changed`). 104 ortak durum (Rust, Python), KCAD, etkileşim/uygulama/masaüstü (1352)/MCP/Python (370), clippy, gerçek GUI. | [x] Bütün madde |
| 2026-10-27 | ARCH-ELEM-02 / ADR0719 | Niş, baca ve tesisat boşluğu: açıklığın `niche`/`flue` biçimleri, `depth`/`inset` (KCAD 119), `recess_too_deep`/`tooDeep`; kesme yüksekliği 1,10 m'ye göre kesik ya da kesikli çizim, çözücüde kısmi derinlik kesimi; kotları ayrı açıklıkların aynı yerde durması; metraj ve ölçü dizisi kuralları; Boşluk ▾, araç ayarları, Öznitelikler. Ortak durumlar bağımsız başvurulardan (geometri 41, oluşturma 112, düzenleme 102), çekirdek/etkileşim/uygulama/masaüstü (1348)/KCAD/Python (368)/MCP takımları, clippy, gerçek GUI. | [x] Bütün madde |
| 2026-10-27 | ARCH-UX-04, ARCH-DOC-04 / ADR0718 | Kat planı paftaları: pafta çekirdeği `levels::level_sheets` (katın paftası, ortak zemin ve ölçek, A-101'den numara, alt küme Mimari › Kat planları, paftası olan kat atlanır), `cad.sheets.levels` (`levels_have_sheets`, `level_not_found`, `not_fixed_map`, `bad_drawing_info`, `invalid_options`); Pafta › Yeni ▾ ve CAD'in Çıktı › Pafta › Kat planları, tek adım. 31 ortak durum (Rust, Python), çekirdek/tasarımcı/masaüstü (1347) testleri, bütün ortak durumlar, clippy, gerçek GUI. | [x] Pafta seti alt maddesi; pafta takımının kalanı açık |
| 2026-10-27 | ARCH-UX-04 / ADR0717 | Kat ver: `cad.levels.assign` (katsızlar ya da verilen mimari nesneler, kilitli atlanır, mahal yeni katında yeniden bulunur, kapanmazsa `arch_space_broken`); Denetim'de kat bulgusunun Düzelt'i etkin katı verir, Kata ver ▾ (Seç ve göster, tek adım); Öznitelikler'de karışık seçimde ortak Kat; ayırıcı kat denetiminde. Ortak durum koşucusunun yığın taşması düzeldi (komut başına çerçeve). 32+41 durum, uygulama/etkileşim/masaüstü (1346)/MCP takımları, clippy, Python, gerçek GUI. | [x] Toplu kat atama alt maddesi |
| 2026-10-27 | ARCH-UX-04 / ADR0716 | Paftada kat: haritanın kendi Kat'ı ve Alt kat soluk (çekirdek `MapItem.level`, `setViewport.level`, `level_not_found`/`bad_level`, PDF ve masaüstü kâğıdı, ön denetim `map_level_missing`, denetçi); yerinde model düzenleme haritanın katıyla; görünüm penceresinin kapsamı geometrik (soluk kat kenetlenmiyordu). 56 ortak durum (10 yeni), 2+2+1+1 test, etkileşim (771)/pafta çekirdeği/çıktı/tasarımcı takımları, clippy, gerçek GUI. | [x] Pafta katı alt maddesi; toplu kat atama açık |
| 2026-10-27 | ARCH-UX-04 / ADR0715 | Kat görünümü: Bütün katlar / Yalnız etkin kat / Etkin kat ve alt kat (soluk, kenetlenir, seçilmez); nesnenin katı sözleşmede (`Entity::level`: açıklık duvarın, etiket nesnenin, kaplama mahalin, ölçü bağlarının); depoda seçilmez küme; tıklamada etkin kat önce (iki katlı dairede pencereler alt kata oturuyordu, düzeldi). Kat menüsü, hücre ikonu, Görünüm şeridi. 1+3+1 test, etkileşim (770)/çekirdek (282)/masaüstü (1344) takımları, clippy, gerçek GUI. | [~] Ekran dilimi; paftanın katı ve toplu kat atama açık |
| 2026-10-27 | ARCH-UX-04 / ADR0713 eki | Merdivenin Yükseklik'i etkin katın yüksekliği (yazılmadıkça; “kat” döner): daire yürüyüşündeki 3,000/3,060 m `stair_rise_level` uyarısı kalktı. Etkileşim testi, merdiven testleri, gerçek GUI. | [x] Kat yüksekliği alt maddesi |
| 2026-10-27 | ARCH-DOC-02 düzeltme / ADR0344 eki | Kapı ve pencere çizelgesinin Duvar sütunu stilsiz duvarda boştu (Duvar aracı Stil: Yok başlar): artık duvarın kalınlığı çizim birimiyle (“0.250 m”). Yeni ortak durum (37), gerçek GUI (`arac-daire-cizelge-yazildi`). | [x] Kusur kapandı |
| 2026-10-27 | ARCH-DRAW-02 düzeltme / ADR0657 eki | Ölçülü bölme'nin yanı imleçle (Sol/Sağ seçilene dek), önizlemedeki uzaklık sayılı ölçü. 2 araç testi, gerçek GUI. | [x] Kullanılabilirlik kusuru kapandı |
| 2026-10-27 | ARCH-OPEN-01 / ADR0714 | Yerleştirirken numara (Otomatik K/P/B, yazılan numaradan sırayla, Yok) ve zeminli etiket yazısı; daire yürüyüşünde etiketler K1–K3/P1–P3 doldu. 3 araç testi, tam etkileşim/masaüstü takımları, gerçek GUI. | [x] Numara alt maddesi; odaya göre numara açık |
| 2026-10-27 | ARCH-UX-04 / ADR0713 | Etkin kat: zemin katta açılış, durum çubuğunda ve araç ayarlarında Kat menüsü; duvar, kolon, merdiven, rampa, aks ağı etkin kata; mahal/ayırıcı/iç zincir o katta duvar varken izler. Belgele'de Duvar ölçüleri ile İç ölçü zinciri tek “Ölçü ▾” (1440 px'te Proje paneli yazılı kaldı). 3+2 test, tam etkileşim (760) ve masaüstü (1343) takımı, gerçek GUI. | [~] İlk dilim; kat görünümü ve toplu kat atama açık |
| 2026-10-27 | ARCH-SPACE-02 düzeltme / ADR0344 eki | Daire yürüyüşü (kabuk, bölmeler, açıklıklar, toplu mahal, Hepsini etiketle, ölçüler, iç zincir, Metraj, Denetim) gerçek GUI'de: etiketli mahal kendi yazısını da çiziyordu (alan iki kez) → etiket onun yazısı; şablonun boş satırları atılır; numarasız açıklığın boş etiketi `tag_empty` ile ve önizlemede söylenir. 164+29 durum yenilendi, çekirdek/araç testleri, resimler. Kalan bulgular: araçlar kat atamıyor (Denetim her duvarı işaretliyor), açıklıklar numarasız başlıyor. | [x] Üç kusur kapandı |
| 2026-10-27 | ARCH-EDIT-01 / ADR0712 | Kolonun geçici ölçüsü: dört yanda paralel en yakın duvar yüzüne/kolon yanına net mesafe, sayıya bas-yaz-Enter kolonu kaydırır, mahal izler, akslara bağlı kolon nedenini hemen söyler; `cad.arch.edit` distance, `cad.columns.distances`. 22+23 durum, 3+4+2 test, Python/MCP, gerçek GUI resimleri; geçici ölçülerde maskeli sayı ve en yakın sayıya tıklama. | [~] Kolon dilimi; aksa mesafe, duvar boyu, kilitli referans açık |
| 2026-10-27 | ARCH-DIM-01 / ADR0711 | İç ölçü zinciri: kesen çizgi, duvar yüzleri/bütün katmanlar, kolon kenar/merkez, toplam, kat; `cross` kuralı (KCAD 118) ile duvar kayınca değer, uzayınca çizgi yerinde; duvar yüzü ve kolonda `along`/`center` yerleri. 44 durum, Python/MCP, 5+4 test, gerçek GUI resimleri. | [x] Kesen çizgi alt maddesi; kalıcı üyelik açık |
| 2026-10-28 | ARCH-DOC-01 / ADR0721 | Mimari işaretler: görünüm işareti paftanın çizim numarasını okur (A-101 → A-201, kopya, silinen pafta `targetGone`); kata bağlı kot (KCAD 121); kuzey oku (plan/coğrafi/açı); revizyon bulutu + üçgen ve Tarih/Açıklama sütunlu Revizyon listesi (KCAD 122); malzeme lejandı. İncelemede bulunan hata: stil verilen duvarın iç ölçü zinciri kopuyordu, düzeltildi. 27+35+20 yeni komut durumu, 6 tablo + 3 iç zincir durumu; PDF `pdftotext`/`pdffonts`; Python/MCP; masaüstü 1354, Python 374; gerçek GUI `arac-isaret-*`. | [x] Madde tamam; pafta başına revizyon çizelgesi açık |
| 2026-10-28 | ARCH-ELEM-01 / ADR0722 | Döşeme, balkon, teras: duvarların dış yüzünden ya da odadan döşeme, kenar ötelemesi ve dört başarısızlığı, serbest kenarda korkuluk/parapet, Kes/Ekle (ıslak hacim, avlu, şaft); Metraj'da döşeme net alanı/hacmi, süpürgelik ve kaplama geçişleri. 47+38 ortak durum, KCAD 123, DXF, Python/MCP, gerçek GUI resimleri. | [x] |
