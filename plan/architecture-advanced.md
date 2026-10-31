# KentOS Mimari — ileri 2B tasarım ve üretim planı

İnceleme: **30 Ekim 2026**. Kod tabanı: `dc7b16d6c`.
Plan dalı: `codex/architecture-advanced-plugin-plans`.

Amaç, bir ev planını kolayca kurmak, tasarım değişikliklerini güvenle yapmak ve
aynı bilgiden okunabilir uygulama belgeleri üretmektir. Yeni düğme sayısı başarı
ölçüsü değildir. Kullanıcı bir odayı büyüttüğünde duvarları, açıklıkları, donatıyı,
mahalleri, ölçüleri ve paftaları elle onarmak zorunda kalmamalıdır.

Bu teslim **araştırma ve plandır**; uygulama kodu, `CLAUDE.md`, mevcut
[architecture.md](architecture.md) ve tamamlandı işaretleri değiştirilmemiştir.
Aşağıdaki yedi iş paketi uygulanmış sayılmaz. Kaynak incelemesi, geçmiş kabul
kayıtları ve bu oturumdaki sınırlı GUI gözlemi ayrı değerlendirilmiştir.

## 1. Mevcut sistemin gerçekten geldiği yer

[Önceki planın](architecture.md) başlangıç envanteri 24 Ekim'i anlatıyor;
sonraki ADR ekleri aynı başlıkları önemli ölçüde genişletiyor. İlk paragraftaki
bir “açık” notu, sonraki kabul kaydından bağımsız okunmamalı.

| Alan | Korunacak mevcut temel | İleri ihtiyaç |
| --- | --- | --- |
| Ölçülü çizim/düzenleme | Dikdörtgen duvar, düz/yaylı ölçülü bölme; duvar, kolon ve açıklık için geçici ölçü, referans değiştirme, sabit uç | Komşu odaları birlikte düzenleme ve isteğe bağlı kalıcı tasarım ilişkileri |
| Duvar birleşimi | Bileşen öncelikleri, köşe/T/çapraz birleşim, uç kipleri, teşhis ve düzeltme | Düğümde bileşen kararını anlaşılır düzenleme; açıklık dönüşleri ve bağlı yapı sınırları |
| Açıklıklar | Host ilişkisi, yerleştirme/çoğaltma, hazır tipler, yön, numara ve çizelge | Kullanıcı/ofis tipleri, tip-örnek ilişkisi ve gerçek net geçiş bilgisi |
| Mahal/ölçü/veri | Mahal bulma, kaynak izleme, kimlik/etiket, otomatik ilişkili ölçüler; farklı pafta ölçekleri | Tasarım programı, bağımsız bölüm ve bütün planın değişiklik etkisi |
| Donatı/eleman | Gerçek boylu blok kitaplığı; L/U mutfak ve cihazlar; döşeme, şaft, korkuluk; gelişmiş merdiven/rampa/aks | İlişkileri koruyan oda/daire modülleri, tip yönetimi ve kullanım boşlukları |
| Belgeleme | Canlı çizelgeler, kaynak hücre düzenleme, metraj, tablo dışa aktarma, kat paftaları, işaretler | Mimari görünüş tarifleri, proje gezgini, koordineli yayın ve gerçek türetilmiş görünüşler |
| Denetim | Mimari denetim, kalıcı istisna, etki önizlemesi ve toplu düzeltme | Net geçiş, kapı/donatı kullanım alanı ve tasarım hedefleri için açıklanabilir geri bildirim |

Bu temellerin kodda bulunması, bütün örneklerin kusursuz olduğunu veya bu
oturumda tekrar kabul edildiğini göstermez. Özellikle `ARCH-BASE-01`,
`ARCH-PERF-01` ve `ARCH-DOC-04` gibi açık kabul kapsamları yeni planla kapanmaz.

### Kod incelemesinin belirleyici kanıtları

Satır numaraları yukarıdaki commit'e aittir; dosya değişirse yeniden doğrulanır.

| Kanıt | Kaynak ve gözlenen sınır |
| --- | --- |
| A1 — Bölme bir oluşturma işlemidir | [walls_partition.rs](../crates/native/application/src/walls_partition.rs), 159–175 ve 229–234: mesafe/kaynak hesaplanıp normal duvar yazılır; bu oluşturma tarifi kalıcı mimari mesafe bağına dönüşmez. |
| A2 — Kısıt motoru zaten vardır | [constraints sözleşmesi](../crates/shared/contracts/src/constraints.rs) ve [çözücü modeli](../crates/shared/geometry-core/src/ops/constraints/model.rs), 74–100, 127–158: temel geometrik türler var; duvar yüzü/çekirdeği/açıklık için mimari değişken adaptörü ayrı iştir. |
| A3 — Açıklık tipi ile bağlı tip farklıdır | [opening.rs](../crates/shared/contracts/src/opening.rs), 587–617, 710–733: hazır tip aranır ve değerleri örneğe kopyalanır. Bu yol kullanıcı tip tablosu ve bütün örneklere canlı tip güncellemesi sağlamaz. |
| A4 — Birleşim yeniden yazılmayacak | [wall_join.rs](../crates/shared/geometry-core/src/geom/wall_join.rs), 1–23 ve 116–156; [walls_joins.rs](../crates/native/application/src/walls_joins.rs), 53–129: mevcut çözüm ve kontrollü retler ileri editörün tabanıdır. |
| A5 — Döşeme sınırının kaynağı saklanmıyor | [slabs_create.rs](../crates/native/application/src/slabs_create.rs), 128–143; [slab.rs](../crates/shared/contracts/src/slab.rs), 159–186: kaynak sınır kontura çevrilir. [slab_follow.rs](../crates/native/domain/src/slab_follow.rs), 1–5, 141–150 serbest korkuluk/parapet kenarlarını yeniler; döşeme çevresini duvara bağlı olarak büyütmez. |
| A6 — Bina organizasyonu sınırlı | [arch.rs](../crates/shared/contracts/src/arch.rs), 450–463, 997–1007: kat/evre/durum/disiplin; [space.rs](../crates/shared/contracts/src/space.rs), 188–239: mahal. Bağımsız bölüm/bina/kanat hiyerarşisi bu alanlarda yok. |
| A7 — Normal blok semantik daire modülü değildir | [blocks_define.rs](../crates/native/application/src/blocks_define.rs), 171–191, 257–267; [space_follow.rs](../crates/native/domain/src/space_follow.rs), 237–255: ölçü sahipliği ve mahal kaynağı için özel sınırlar var; blok içi içerik yeni ilişki grafiğinin yerine geçirilemez. |
| A8 — Kamera kaydı ile mimari görünüş farklı | [cad_views.rs](../crates/shared/contracts/src/cad_views.rs), 30–58: ad/kimlik, merkez, boy ve açı. Kat, evre, temsil ve anotasyon kapsamını birlikte saklayan görünüş tarifi değildir. |
| A9 — Kesit işareti kesit üretimi değildir | [viewmark.rs](../crates/shared/contracts/src/viewmark.rs), 154–196; [işaret geometrisi](../crates/shared/geometry-core/src/geom/viewmark.rs), 1–13: ok/işaret/hedef üretir. Gerçek cephe veya kesit geometrisi üretildiği sonucu çıkarılamaz. |
| A10 — UI ve denetim tabanı kullanılacak | [architecture_settings.rs](../apps/desktop/src/architecture_settings.rs), 29–133; [ribbon.json](../catalog/v1/ribbon.json); [arch_audit.rs](../crates/native/application/src/arch_audit.rs), 39–90: görünür araç ayarları, bağlamsal şerit ve çok sayıda denetim mevcut. |

### Bu oturumdaki arayüz gözlemi ve sınırı

- Yerelde bulunan **KentOS Architecture Live 3 QA** paketi açıldı. Duvar
  seçildiğinde Duvar bağlamsal sekmesi ve özellikleri; Dikdörtgen duvar etkinken
  görünür araç ayarları gerçekten görüldü. Eski `house-architecture-audit.kcad`
  örneği açıldı, Home ile sığdırıldı, araç etkinleştirilip Escape ile bırakıldı.
  Çizime geometri yazılmadı ve kaynak dosya kaydedilmedi.
- Bu paket **27 Ekim tarihli dondurulmuş test derlemesidir**. `target/debug`
  programının zamanı da güncel 30 Ekim kaynak tabanını doğrulamıyor. Dolayısıyla
  bu gözlem güncel modül ayrımının veya 28–30 Ekim araçlarının GUI kabulü değildir.
  Eski görüntüden bugünkü uygulamaya yeni kusur atfedilmemiştir.
- Ekran görüntüleri bu oturumda görsel olarak incelendi; repoya eklenmedi.
  Uygulamaya geçerken ilk iş, güncel commit'ten derlenmiş KentOS Mimari ile
  aşağıdaki konut senaryosunu yeniden kaydetmektir. Bu kapı şu an **bekliyor**.

## 2. Piyasa araştırması: hangi davranışlar değerli?

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

## 3. Yedi odaklı geliştirme paketi

### ADV-01 — Bağlantılı odaları net ölçüyle kur ve güvenle değiştir [~]

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

_Durum (31 Ekim 2026, [ADR 0772](../docs/adr/0772-door-window-types-instances-clear-width.md)):_ tipler, örnek istisnası,
dört en, yapı, takip raporu ve Kapılar denetimi yapıldı. Döner, cebe giren ve katlanır kapı, kayar kapının rayı ve bindirmesi
([ADR 0777](../docs/adr/0777-revolving-pocket-folding-doors.md)). Döner/katlanır/raylı sürme kapının çarpışma denetimi düzeltmesiyle,
yaylı duvarda cep ve cebin metrajdan düşülmesi ([ADR 0787](../docs/adr/0787-special-door-clashes-arc-pockets-pocket-quantities.md)).
Üç ve dört kanatlı sürme ve paftanın PDF metninin okunması ([ADR 0792](../docs/adr/0792-multi-leaf-sliding-doors-and-pdf-text.md)). Açık: bağımsız bölüm kapsamı (ADV-04), XOX ve eşit olmayan sürme kanatları, gerçek pencerede elle kabul.

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

_Durum (31 Ekim 2026, [ADR 0773](../docs/adr/0773-building-program-units-and-design-brief.md),
[ADR 0784](../docs/adr/0784-building-wing-window-and-common-area-shares.md)):_ bölüm/ortak/dış alan üyeliği, tasarım
hedefi ve gerçek/hedef/fark, net geçiş ve kapı/komşu/donatı denetimleri istisnalarıyla, böl/birleştir kararı ve
durdurulabilir denetim; bina/kanat penceresi ve katları, ortak alanların bağımsız bölümlere tam payı (`kentos.share/1`:
eşit, net alana, ağırlığa göre; kanat/bina/program kapsamı, en büyük kalan, paylar havuza tam eşit) yapıldı. Açık:
resmî alan profili ve arsa payı, bölüm çizelgesi türü, gerçek pencerede elle kabul.

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

_Durum (31 Ekim 2026, [ADR 0775](../docs/adr/0775-room-modules-and-controlled-reuse.md), [ADR 0779](../docs/adr/0779-room-modules-members-drift-and-flats.md), [ADR 0788](../docs/adr/0788-design-options.md)):_
tek kat, düz duvarlı oda ve bütün daire modülü; tanım ve örnek ayrı, yerleştirme, planlı kaynak güncelleme, yerel
istisna, bağımsızlaştırma, kaynağı silme; üye ekleme ve çıkarma; elle değişen örnek nesnesi; katları tekrarlama.
Tasarım seçenekleri: küme ve birincil seçenek, bağımsız kopya, birincili değiştirme, iki seçeneğin karşılaştırması
(sayı, alan, plan farkı), kabul (öbürleri silinir) ve silme; görünüş tarifiyle başka seçeneği gösteren pafta;
seçenekler birbirini görmez (birleşim, çakışma, mahal sınırı). Dört banyo, iki katta dört daire ve mutfak A/B kabulü
geçti. Açık: seçenek bileşimi, seçenekler arası eşleme/birleştirme, çok katlı tek modül, kattaki modülsüz nesnelerin
tekrarı, gerçek pencerede elle kabul.

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

_Durum (31 Ekim 2026, [ADR 0774](../docs/adr/0774-architectural-view-recipes.md)):_ adlı görünüm üstünde sürümlü görünüş tarifi
(kat, alt kat, evre, katman durumu, gizlenen türler, ölçek), bağlı paftanın tarifle çizmesi ve pafta setinin tarife göre etkisi
yapıldı. Proje gezgini ([ADR 0778](../docs/adr/0778-project-navigator.md)): `cad.navigator.read` ve Gezgin paneli; katlar,
görünümler, paftalar, çizelgeler ve seçili nesnenin göründüğü yerler. Tarifin duvar ayrıntısı ve görünümün alanına kırpma
([ADR 0781](../docs/adr/0781-view-recipe-wall-detail-and-crop.md)). Görünüş şablonu ve kalem seti, siyah beyaz
([ADR 0783](../docs/adr/0783-view-templates-and-pen-sets.md)). Kırpma çerçevesinin ekranda düzenlenmesi ve 1:20 PDF incelemesi
([ADR 0789](../docs/adr/0789-crop-frame-on-screen.md)). Açık: türetilmiş çizim, yayın/revizyon birlikteliği (paftanın yayın kaydı
ister), gerçek pencerede elle kabul. Alanı haritadan büyük görünümün ön denetimi ([ADR 0794](../docs/adr/0794-map-view-cut-preflight.md)).

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

_Durum (31 Ekim 2026, [ADR 0776](../docs/adr/0776-measured-quality-acceptance-of-an-apartment.md)):_ 2+1 daire gerçek
girdiyle iz ve bölüm ölçüsü, Python/MCP/masaüstü anlamsal eşitliği, büyük koordinat ve ölçek regresyonları, 1/20/100 daire
ölçümü, dört küçük düzeltme. B6 ve binanın çizelgeleriyle B7 kapandı ([ADR 0782](../docs/adr/0782-design-change-reads-only-what-it-reaches.md));
yazı katmanının artımlı kurulması (B13) de kapandı ([ADR 0786](../docs/adr/0786-text-layer-object-by-object.md): kare 351 → 163 ms, bellek +3 062 → +191 MB); etiket motorunun artımlı yerleşimi açık. Model alanında kalem kâğıttaki genişliğinde tutulur ve kopyalanan etiket kopyayı adlar ([ADR 0793](../docs/adr/0793-model-pen-widths-and-copied-links.md); B8 kapandı). Açık: gerçek
pencerede elle kabul, kabul korpusunun öbür dört örneği.

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

## 4. Uygulama sırası ve bitiş koşulu

| Sıra | İnce, tamamlanabilir teslim | Bağımlılık / bitiş kanıtı |
| --- | --- | --- |
| 0 | Güncel Mimari modülünde 2+1 temel akışını ölç, gerçek sorunları tekrar üret | Eski derleme ile güncel kaynak ayrımı kapanır; `ARCH-BASE-01` ve performans açıkları görünür kalır. |
| 1 | ADV-01: ortak bölmeyi net ölçüden değiştir, tek isteğe bağlı mesafe bağı | Mevcut araçlar korunur; komşu etkisi, çelişki, Undo ve API eşdeğerliği geçer. |
| 2 | ADV-02: bağlı döşeme sınırı ve bir bileşen birleşim/kapama örneği | Kaynak/bağımsız ayrımı, duvar değişimi ve 1:20 çıktı geçer. |
| 3 | ADV-03: kullanıcı kapı tipi, örnek istisnası ve açık ölçü referansı | 20 örnek güncellemesi, net geçiş ve çizelge aynı sonuç verir. |
| 4 | ADV-04 + ADV-06'nın ilk dilimi | Bağımsız bölüm/alan hedefi ve iki kayıtlı mimari görünüş; mevcut pafta sistemi kullanılır. |
| 5 | ADV-05, sonra gelişmiş türetilmiş görünüş/alternatif kapsamı | Dar modül kabul edilmeden bütün bina/alternatif sistemi açılmaz. |
| Her sıra | ADV-07 ve [plugins.md](plugins.md) ortak kapıları | Kullanılabilirlik, performans, veri/API ve gerçek GUI kanıtı olmadan `[x]` yok. |

Her alt teslimin ürüne geçişinde gerekli KCAD sürümü/uyumu, kilitli katman,
belge revizyonu, bulut değişiklik paketi ve yetki yeniden kontrol edilir.
İlgisiz renderer, birim veya etkileşim ailesi topluca yeniden yazılmaz.
Yeni davranış eski belgede varsayılan olarak yeni bağ/ölçek üretmez.

Mimari çizim kuralları, kurum/ofis standardı ve mevzuat denetimi ayrı
profiller olmalıdır. Standart adı/sürümü, kapsam, birim ve yorum kaynakları
kaydedilmeden “uluslararası standartlara tam uygun” veya “ruhsata uygun”
iddiası yapılmaz. Yönetmelik profili için uzman/kurum kabulü ayrıca gerekir.
Bu plan otomatik hukuki/proje onayı tanımlamaz.

Ham ekran görüntüleri Git'e girmez; yerel ignored kanıt alanında tutulur.
Repoda kısa sonuç, kaynak commit'i, yeniden üretim adımı ve gerekli küçük
örnek model bulunur. Bu planın kabulü uygulamanın üretim kabulü değildir.
