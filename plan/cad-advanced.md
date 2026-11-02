# KentOS — ileri genel 2B CAD planı

İnceleme: **31 Ekim 2026**. Kod dayanağı:
`72728b04c33fc9c64962d64903ef81e6449f310e` (`main`).

Bu belge, genel çizim ve belgeleme yeteneklerini geliştiren **20 ana madde**
içerir. Hedef; mimari, harita ve diğer teknik çizimlerde kullanılan ortak
**2B CAD** altyapısının güvenilir, hızlı ve kolay kullanılabilir olmasıdır.
Katı modelleme, 3B sahne, BIM modelleme, 3B modelden kesit/cephe türetme,
LiDAR işleme ve yol koridoru bu planın kapsamı değildir. Dış dosyadaki Z/M
bilgisinin kaybolmadan taşınması ise 2B ürünün veri bütünlüğü sorumluluğudur.

31 Ekim araştırmasında yalnız bu plan dosyası oluşturulmuştur. Başka ajanın çalışma alanındaki değişiklikleri incelemeye
karıştırmamak için kod bağlantıları sabit commit'e verilmiştir. Yeni ana
maddelerin uygulama ilerlemesi aşağıda ayrıca tutulur; var olan bir temelin listelenmesi yeni kabul
kapsamının tamamlandığı anlamına gelmez.

**Uygulama başlangıcı:** 1 Kasım 2026, `main`, `4f1d11fde` üzerine.
`[~]` kısmen uygulanmış ve kalan kabulü açık, `[x]` bütün kabulü doğrulanmış demektir.

## Yaklaşım ve diğer planlarla ilişki

- [cad.md](cad.md) ilk kapsam envanteridir. Oradaki çok sayıda araç artık
  uygulanmıştır; eski başlangıç açıklamaları bugünün eksik listesi sayılamaz.
  Bu plan, aynı araçları ikinci kez yazdırmak yerine kapsam sınırlarını ve
  birden fazla aracın birlikte çalışması gereken üretim senaryolarını ele alır.
- [architecture.md](architecture.md) ve
  [architecture-advanced.md](architecture-advanced.md) duvar/oda/açıklık gibi
  mimari anlamın sahibidir. [map-cad.md](map-cad.md) ölçme, arazi, kadastro ve
  harita üretiminin sahibidir. Genel kenetleme, blok, tarama, ölçü, yazı ve
  pafta motorları ortak kalır. Mimari Gezgin ve görünüş tarifleri gibi yeni
  mevcut işler yeniden yapılacak diye yazılmaz.
- KentOS Mimari, Harita ve GIS programlarının mevcut ayrımı korunur.
  Bu belge dördüncü bir ürün veya CAD↔GIS mod değiştirme düğmesi oluşturmaz.
  GIS'te kullanılan ortak geometrik araçlar tipli katman ve erişim kurallarına uyar.
- [ADR 0626](../docs/adr/0626-units-scale-and-true-size-texts.md) kapsamındaki
  birim, tek çizim ölçeği ve CAD'de gerçek boylu metin kararları korunur.
  Rakip bir yazılım milimetreyle başlıyor diye KentOS'un mevcut proje
  varsayılanı yeniden değiştirilmez.
- [ADR 0228](../docs/adr/0228-web-pause-desktop-and-cloud-first.md) gereği
  masaüstü, ortak Rust, gereken sunucu yolları, Python ve MCP hedeflenir.
  Bu plan web geliştirmesini başlatmaz. Python/eklenti ayrıntıları için
  [python.md](python.md) ve [plugins.md](plugins.md) izlenir.
- `CAD-IO-15` içindeki genel afin/yay/genişlik modelinin açık bölümü,
  bu planla tamamlanmış sayılmaz. `CAD-IO-20` gerçek AutoCAD kabulünün
  sahibi tarafından atlandığı kararı da korunur; yapılmış test diye gösterilmez.
- Ürün davranışı güncel 2B üretim ihtiyaçlarına göre tasarlanır. Rakiplerin
  bütün eski komutlarını, COM/ActiveX altyapılarını veya dosya geçmişini
  aynen çoğaltmak hedef değildir. Mevcut verinin korunması ve günlük
  DWG/DXF uyumluluğu bu ilkeden ayrı, gerekli ürün sorumluluğudur.

## Araştırma ve kod incelemesi

**25 birincil internet kaynağının içeriği** incelendi: AutoCAD 2027,
BricsCAD'in güncel/V27 belgeleri, DraftSight'ın 2026 tarihli 2B araç rehberi,
QCAD ve LibreCAD belgeleri, QCAD kaynak kodu, Unicode ve HarfBuzz belgeleri.
Kaynakça aşağıdadır. Ürünlerin ücretli sürümleri bu bilgisayarda çalıştırılmış
sayılmaz; sürüme/lisans türüne bağlı özellikler bütün ürünlere genellenmez.

QCAD'in `Trim.js` dosyasında araç durumu, iptal/geri adım, önizleme ve
uygulama ayrımı; `RDocument.cpp` içinde işlem grupları, Undo ve blok başına
uzamsal dizin yolları da incelendi. Açık kaynak araştırması doğrudan kod
kopyalama veya yeni bağımlılık kararı değildir. Aşağıdaki çözümler KentOS
için tasarım önerileridir; kapalı kaynak rakip motorların iç uygulaması
biliniyormuş gibi yazılmamıştır.

Bu oturumda güncel uygulama derlenip GUI kabulü yapılmadı. Aşağıdaki kod
bulguları gerçek dosya/işlev okumalarına dayanır; mevcut ekranda yeni bir
bozukluk tespit edildiği iddiası içermez. Her işin sonunda gerçek etkileşim
ve ekran görüntüsüyle kabul ayrıca zorunludur.

### Koddan görülen önemli temeller ve sınırlar

Bağlantılar yukarıdaki commit'e aittir. Bir satırın sınırı, belirtilen
işlev/veri yolunun sınırıdır; bütün depoda başka destek olmadığı iddiası değildir.

| Kanıt | Mevcut uygulama ve ileri iş için anlamı |
| --- | --- |
| C01 | [units_change.rs][C01]: birimi değiştirirken geometriyi koruma/ölçekleme ayrımı, plan ve tek Undo zaten var. Yeni birim motoru gerekmiyor. |
| C02 | [changeset.rs][C02]: komutlar tek değişiklikte birleştiriliyor; plan okuma kopyasında çalışıyor. Yeni araç makrosu bu yolu kullanmalı. |
| C03 | [followers.rs][C03]: bağlı nesnelerin ekleme/güncelleme/silme etkisi `FollowerDelta` ve önizleme yollarıyla mevcut. Yeni bir ilişki motoru varsayılmamalı. |
| C04 | [entities_select.rs][C04]: geometrik seçim sorgusu ve görünüş kapsamı var. Katman ifade/listesi süzgeçleri de mevcut; ileri iş seçim ve özellikleri anlaşılır tek akışa bağlamak. |
| C05 | [properties/mod.rs][C05]: panel belge/kuşak/seçim sürümüyle önbellekleniyor; hücre Enter/odaktan çıkışla komut gönderiyor. “Her tuşta bütün model yazılıyor” varsayımı doğru değil. |
| C06 | [trim.rs][C06]: delikli polygon için açık ret var; genel budama iç halkaları kaybedeceğinden uygulanmıyor. Güvenli ret destek tamamlığı değildir. |
| C07 | [fillet.rs][C07]: iki düz çizgiyi bağlayan temel ve path köşesi mevcut; yaylı köşeler için açık sınır var. Çizgi–yay/yay–yay ailesi ayrı geometrik iş gerektiriyor. |
| C08 | [offset.rs][C08], `offset_noted`: kesin/yaklaşık sonuç ve kesişim raporu temeli var; fit/NURBS ötelemesi kirişlerden türetiliyor. Genel eğri ötelemesi analitikmiş gibi sunulmamalı. |
| C09 | [constraints/model.rs][C09]: nokta, çizgi, daire, yay ve path köşeleri değişkenlere çevriliyor; path bulge sabit. Elips/NURBS için genel değişken adaptörü bu modelde yok. |
| C10 | [constraints_query.rs][C10]: parametre, ölçülen değer, tutan/çelişen bağ ve çözüm teşhisi zaten sorgulanıyor. İleri hedef, teşhisi ve serbestliği kullanıcıya eyleme dönük göstermektir. |
| C11 | [block_dynamic.rs][C11]: doğrusal/kutupsal/açı/nokta/flip/görünürlük/lookup parametreleri ve eylemler var. “Dinamik blok ekle” yeni iş olarak yazılmamalı. |
| C12 | [blocks_sync.rs][C12]: öznitelik eşleme ve güncelleme var; bu eşleme çizimdeki yerleştirmeler içindir, başka tanım içindeki yerleştirmeler kendi tanım düzenlemesine bırakılıyor. |
| C13 | [assoc_array.rs][C13]: ilişkili grid/kutupsal/doğru/yol dizisi, kalıcı öğe kimliği, silinen/elle değiştirilen öğe semantiği var; en çok 10.000 yer sınırı konmuş. |
| C14 | [hatch_boundaries.rs][C14], `settled`: kesin sınır ve çizilen ring birlikte yönetiliyor; ring elle değişirse veya sınır geçersizleşirse sınır tanımı bırakılabiliyor. |
| C15 | [hatch_ties.rs][C15]: ilişkili tarama yeniden hesaplanıyor; kaynak kapanmazsa veya yok olursa yerinde kalıp bağı bırakabiliyor. Bu durumun kullanıcıya teşhisi ayrıca ele alınmalı. |
| C16 | [dimensions_query.rs][C16]: gerçek ölçü, gösterilen metin, override ve tam/kısmi/kopuk bağ ayrımı mevcut. Ölçü denetimi bunları yeniden hesaplayan ikinci motor olmamalı. |
| C17 | [paragraph.rs][C17]: girinti, tab, sütun, kesir ve satır kırma var; düzen karakter ilerleme değerleri ve Unicode scalar aralıklarıyla çalışıyor. Ortak şekillendirme/karakter kümesi tutarlılığı ayrı inceleme alanı. |
| C18 | [plot/images.rs][C18]: çıktıdaki bir metin outline yolunda `rustybuzz::shape` zaten kullanılıyor. “Üründe shaping hiç yok” denemez; bütün ekran/ölçüm/çıktı yollarının eşdeğerliği kanıtlanmalı. |
| C19 | [line_types.rs][C19], `dashes_of`: karmaşık yazı/şekil öğeleri çizimden atlanıyor; sekizi aşan dash/gap listesi kesiliyor ve `lossy` işaretleniyor. Somut görüntü tamlık açığı. |
| C20 | [standards.rs][C20]: `cad.standards.check/apply` mevcut. Katman çevirisi, font ve stil uygulaması yeniden yazılmayacak; kapsam ve kurum paketi geliştirilecek. |
| C21 | [table.rs][C21]: kaynaklı CAD tablosu, hücre stilleri, formüller, fields ve kaynak türleri var; hücrenin kalıcı içeriği tek satırlı metin tablosu. Tam veri çıkarım/uzlaştırma sistemiyle aynı şey değil. |
| C22 | [xref.rs][C22]: dosya özeti, göreli yol, kaynak içerik ve iç içe referans bilgisi var. Kaynak değişikliği sessizce alınmıyor. |
| C23 | [assets_check.rs][C23]: bulunamayan/değişen bağlı kaynaklar ve aranan yollar tek yapıda sorgulanıyor. Taşınmış kaynağı bulma ve farkı açıklama bu temelin üzerine kurulmalı. |
| C24 | [drawing_compare.rs][C24], `member/settings`: eşleme UUID ile; karşılaştırma geometrisi ve öznitelik/katman bilgisi hazırlanıyor. Stil tanımı, blok içeriği ve pafta değişimini kapsadığı varsayılmamalı. |
| C25 | [cad_views.rs][C25]: adlı görünüş artık yalnız kamera değil; `recipe` ile kat/evre/katman durumu/kategori/ölçek de taşıyor. Yeni Gezgin ve mimari görünüş işleri korunmalı. |
| C26 | [sheet/preflight.rs][C26]: pafta bulgusu önem, kararlı kod ve düzeltme bilgisi taşıyor. Yeni teslim denetimi bu sisteme bağlanmalı. |
| C27 | [file_import_dwg.rs][C27]: DWG komutu ortak dosya içe alma/doğrulama/plan yoluna delege ediyor. Birim, rapor ve Undo için ikinci yol açılmamalı. |
| C28 | [cleanup_cmd.rs][C28]: yinelenen/boş nesne, doğruda birleşme ve gereksiz köşe temizliği ile kayıp raporu var. Yeni hedef, boşluk/T birleşimi/açı sapması ve işlemlerin kontrollü birleştirilmesi. |
| C29 | [audit_cmd.rs][C29]: bozuk ilişki ve kullanılmayan kaynak taraması mevcut; kilitli olanlar onarılmıyor. İş “audit/purge ekle” değildir. |
| C30 | [recovery.rs][C30]: ayrı klasörde doğrulanan KCAD kopyası, geçici dosyadan rename, arka plan ve çökme sonrası kurtarma teklifi var. Kurtarma sıfırdan yapılmayacak. |
| C31 | [precision.rs][C31]: CPU f64 → yerel orijin + hi/lo f32 ve uzun doğruyu yalnız gösterimde bölme mevcut. Yüksek hassasiyet yeni renderer yazılarak aranmayacak. |
| C32 | [headless/dispatch.rs][C32]: validate/plan/execute ortak katalog işleyicisine gidiyor. Python ve MCP için yeni hesap kopyaları yazılmamalı. |

### Piyasadan alınacak esas davranışlar

| Örnek | KentOS için çıkarım |
| --- | --- |
| AutoCAD 2027: Geometry Cleanup, Connected References ve proje destek dosyaları. [R01] | Çizim kalitesi ve kaynak sürekliliği ürün akışının parçası olmalı. Bu sayfa iç algoritmanın veya KentOS hızının kanıtı değildir. |
| BricsCAD: BLOCKIFY, ARRAYDETECT ve parametre paneli. [R11], [R12], [R13] | Tekrarlanan çizimi tanıyıp ilişkili yapıya dönüştür; eşleşme ve parametre etkisini kullanıcı inceleyebilsin. |
| BricsCAD: Drawing Health, Optimize ve zaman sınırlı Health Advisor. [R14], [R15], [R16] | Dağınık temizlik araçlarını tarifli, önizlemeli ve durdurulabilir bir kalite akışında birleştir. |
| DraftSight: hızlı budama, özel bloklar, veri çıkarımı ve pafta seti. [R17] | Sık yapılan düzenlemeyi az adımla tamamla; çizim–tablo–teslim bağlantısını koru. Rehberdeki 3B özellikler bu plana alınmadı. |
| QCAD: özellikte türe göre daraltma, kaynak renk/görünen renk ayrımı; gerçek Trim kodunda açık durumlar. [R18], [R19], [R20] | Kalabalık seçimi güvenle düzenleme ve açık araç durumu, ek komut sayısından daha değerlidir. |
| LibreCAD: blok tanımı/örnek farkı ve baskı önizlemesi. [R22], [R23] | Kaynak ile yerleştirme, model ile kâğıt ölçeği açık olmalı. `latest` belgeleri ön sürüm belgesi olduğunu söylüyor; sürüm garantisi çıkarılmadı. |

## 20 ana geliştirme maddesi

### CAD-ADV-01 — Komut, önizleme, onay ve iptali öngörülebilir hale getir [x]

**Öncelik:** P0. **Kod:** [C02], [C03], [C32].
**Dayanak:** QCAD'in Trim durumları ve işlem grupları; DraftSight hızlı
budama akışı. [R20], [R21], [R17]

**İleri kapsam:** mevcut araçları değiştirmeden bütün genel CAD komutlarında
aynı anlaşılır etkileşim sözleşmesini tamamla.

- Araç aşaması, beklenen girdi, etkin seçenekler, seçilmiş kaynak ve
  sonlandırma yolu görünür olsun. Nokta/nesne/mesafe isteyen aşamalar
  birbirine karışmasın; komut satırı ve etkin araç ayarları aynı taslağı göstersin.
- Enter ve sağ tuş menüsündeki Onayla geçerli son aşamada **bir kez**
  bitirsin. Önceki aşamaya dönme, seçimi tamamlama ve işlemi uygulama
  ayrı geçişler olsun; ikinci kez onay isteme hatası senaryolarla yakalansın.
- Esc, seçenek değişimi, Undo, belge değiştirme ve odak kaybı için
  davranış tanımlansın. Önizleme belgeye yazmasın; eski önizleme veya geç
  arka plan cevabı başka çizime uygulanmasın.
- Kaynak ve bağlı nesnelerin etkisi mevcut `FollowerDelta` ile
  önizlensin. Kilitli hedef, desteklenmeyen eğri ve geçersiz değer
  yalnız “başarısız” demesin; nedenini ve değiştirilebilecek girdiyi göstersin.
- Son komutu yineleme, son kullanılan seçenekler ve semantik komut
  geçmişi mevcut kayıtlarla bütünleşsin. Python çıktısı ayarı açıksa
  gerçekten uygulanan komut yazılsın; fare koordinatı kaydı API yerine geçmesin.

**Kabul:** Taşı, Kopyala, Buda/Uzat, Köşe yuvarla, Tarama, Blok ekle ve
Ölçü'de fare + klavye + sağ tuş akışlarını çalıştır. Tek onay, Esc'de sıfır
yazım, tek mantıksal Undo ve önizleme/sonuç eşitliği gerçek pencerede doğrulansın.

**1 Kasım ilk dilim — [ADR 0812](../docs/adr/0812-stage-specific-command-confirmation.md):**

- [x] Sağ tuş komut menüsü, aracın mevcut `Prompt` verisindeki Enter eylemini
  gösterir. Seçimi tamamlama, koordinat farkı girme, son farkı uygulama,
  kopya dizisinden dönme ve taşımadan bitirme birbirinden ayrıldı.
  Aynı eylemler komut satırında da görünür; onay/iptal ve geometri hesapları değiştirilmedi.
- [x] Gerçek masaüstünde Taşı/Kopyala/Buda: tek menü tıklamasıyla ilgili
  aşamaya geçiş/bitirme, hedef tıklamasıyla taşıma, kopyayı fazladan nesne
  oluşturmadan bitirme, Esc ve tek Undo denendi. Geniş ve yaklaşık 1100×680
  mantıksal piksel pencerede menüler incelendi; görüntüler Git dışında.
  825 etkileşim, 7 menü ve 5 anlamsal komut testi geçti; 5 isteğe bağlı test atlandı.
- [x] İkinci dilim, [ADR 0813](../docs/adr/0813-readable-command-stage-and-default-value.md):
  dar satır aşama metnine ölçülmüş yer ayırır, ikincil seçenekleri mevcut
  Diğer menüsüne taşır. Hâlâ kısalan uzun metin üzerine gelince tam açıklama
  görünür. Yaklaşık 1100×680 pencerede Taşı temel nokta istemi tamamen
  okunur; Diğer içindeki Enter seçeneği doğru aşamayı açar.
- [x] Köşe yuvarla sağ tuş menüsü uygulanacak son yarıçapı birimiyle gösterir.
  Gerçek pencerede 1 m yarıçap yazma, sonraki köşeye son yarıçapı tek
  menü tıklamasıyla uygulama, tek Undo ve bekleyen köşeden Esc denendi.
  Uzat'ta seçilen sınırı tek menü tıklamasıyla kullanma, çizgiyi uzatma,
  tek Undo ve sınır seçimini Esc ile yazmadan iptal etme doğrulandı.
  Bu dilimde 321 UI ve 8 masaüstü menü testi geçti; derleme başarılı.
- [x] Üçüncü dilim, [ADR 0814](../docs/adr/0814-hatch-values-and-placement-confirmation.md):
  Tarama sayı/renk beklerken sahneye tıklamak artık eski değerle tarama
  üretmez; aşama korunur ve ne girileceği açıklanır. Bu aşamada bölge
  önizlemesi gösterilmez; başlangıç noktası ve özellik alma tıklamaları korunur.
  Değerden dönme, nokta seçmeden dönme ve özellik almadan dönme ayrı açıklanır.
- [x] Ölçü yerleşiminde Enter **Ölçü çizmeden başa dön**, Blok ekle'de
  **Yerleştirmeden bitir** olarak komut satırı ve sağ tuşta görünür.
  Mevcut yerleştirme matematiği ve Enter davranışı korundu. Gerçek pencerede
  tarama önizlemesi/uygulaması/tek Undo, sıfır ölçek reddi ve Esc;
  hizalı ölçü yerleştirme, klavyeden uzaklık ve yazmadan başa dönme;
  blok ekleme, fazladan kopyasız tek bitirme, tek Undo ve Esc doğrulandı.
  830 etkileşim, 8 menü ve 5 anlamsal komut testi geçti; 5 isteğe bağlı test atlandı.
- [x] Dördüncü dilim, [ADR 0815](../docs/adr/0815-document-boundaries-discard-command-drafts.md):
  dosya açma, sekme değiştirme ve kapatma normal Esc yerine bütün bekleyen
  etkileşimi bırakır. Tarama sayı aşamasından başka dosya açıldığında
  komutun yeni çizimde sürmesi gerçek pencerede bulunup düzeltildi.
  Askıdaki komut, tutamaç/seçim kutusu, kilit/izleme, yarım girdi ve
  yerinde yazı/paragraf taslağı başka belgeye taşınmaz; normal Esc korunur.
- [x] Gerçek pencerede dosya açma, Taşı/tutamaç önizlemesi ve onaysız yazıyla
  sekme değiştirme; yeni çizgi ve sekmelerin bağımsız tek Undo'su;
  değer aşamasından Esc ve bekleyen komutla kapatma doğrulandı. Görüntüler
  Git dışında. 832 etkileşim testi geçti. Geniş masaüstü: 1551 geçti,
  256 atlandı; eski iptal çağrılarıyla da aynı koordinatlarda tekrarlayan
  bir geçici ölçü testi başarısız kaldı. Bütün takım geçmiş sayılmıyor.
  Önceki Enter açıklamalarının 18 fixture'daki metin/seçenek beklentileri
  güncellendi; geometri ve Undo beklentileri korundu, senaryo çeşitleri geçti.
- [x] Son kabul, [ADR 0816](../docs/adr/0816-command-lifecycle-and-preview-acceptance.md):
  OS odak kaybı yarım fare hareketini yazmadan bırakır; açıkça girilmiş
  komut noktalarını korur. Belge/araç oturumuna ait dosya, nokta bulutu ve
  ağ yanıtları eski hedefe aitse reddedilir. Blok tanımı/öznitelik taslağı
  başka belgeye geçmez; odaklı öznitelik alanında tek Esc iptal eder.
- [x] Kilitli nesne gölgesi artık gerçek işlem gibi sabit kalır; bağlı
  tarama boşluğu/deseni kaynakla birlikte önizlenir. Ölçünün gerçek yazı
  konumu/yönü/yüksekliği yerleştirmeden önce görünür. Fillet geçersiz
  değer ve kilit/eğri gerekçesini açıklar; fare önizlemesi ile Enter'ın son
  yarıçapı ayrı gösterilir. Alt+Q odaklı komut girdisinde de çalışır.
- [x] Gerçek pencerede iç adalı/çizgi ağından tarama, öznitelikli/dinamik
  blok, yarıçap/açı/doğrusal/ordinat/yay uzunluğu ölçüsü, bağlı/kilitli
  taşıma, kısa/basılı sağ tık ve Python çıktısı açık/kapalı denendi.
  Ana araçların tek onay, yazmadan Esc, tek mantıksal Undo ve önizleme/sonuç
  kabulü tamamlandı; ayrıntılı matris ADR 0816'da. Bu, bütün geometrik
  kiplerin veya diğer CAD-ADV maddelerinin tamamlandığı anlamına gelmez.
- [x] Son doğrulama: 840 etkileşim + 1557 masaüstü + masaüstü profiliyle
  286 UI testi geçti; 258 isteğe bağlı test atlandı, başarısız test yok.
  Derleme ve Clippy başarılı. Önceki geçici ölçü testinin iki referans +
  iki bağ kontrolü ayrıştırılarak doğrulandı; renderer değişmedi. Son dilimin
  49 ekran görüntüsü ve günlükleri Git dışında tutuldu.

### CAD-ADV-02 — Birim, 2B UCS ve hassasiyeti kullanıcının öğrenmek zorunda kalmadığı bir sisteme dönüştür [x]

**Öncelik:** P0. **Kod:** [C01], [C27], [C31].
**Dayanak:** model/kâğıt ayrımı ve kaynak yerleştirme davranışları. [R22], [R23]

- Mevcut metre tabanlı kaynak modelini, birimli girişi, ayak/inç ve 2B
  UCS'yi koru. UI birimi, dosyanın bildirilen birimi, blok birimi, çizim
  ölçeği, viewport oranı ve zoom ayrı anlamlar taşısın; birbirini ikinci kez çarpmasın.
- Dosya açma/ekleme/yapıştırma/blok yerleştirme aynı birim kararını
  kullansın. Belirsizlikte açık karar ve önizleme olsun; dosyadaki gerçek
  ölçüyü doğrulamadan “bu ev olmalı, santimetreye çevir” tahmini yapılmasın.
- Birim değişikliği öncesi neyin boyunun değişeceği özetlensin:
  geometriyi koruma ile sayıları koruyup ölçekleme ayrı kalsın. Sonuç;
  bloklar, yazılar, ölçüler, diziler, taramalar ve paftalarda birlikte sınansın.
- Dünya koordinatı, döndürülmüş/ötelenmiş UCS ve kamera dönüşü tutarlı
  olsun. API'de world/UCS bağlamı açık; görüntüden yuvarlanmış değer
  tekrar hesap kaynağı olmasın. Ondalık virgül, negatif değer ve birimli ifade destekleri korunmalı.
- Büyük koordinatta çok küçük ayrıntının ekran ve ölçü doğruluğu
  bağımsız sınansın. Hi/lo ve yerel orijin gösterim içindir; orijin
  yenilemek kaynak koordinatlarını veya ilişki referanslarını değiştirmesin.

**Kabul:** aynı 1 m elemanı mm/m/inç girdileri ve farklı kaynak blok
birimleriyle yerleştir; fiziksel boy değişmesin. 4 × 3 m örnek her yolda
12 m² ölçülsün. Dönük UCS, uzak koordinat, clipboard, Undo ve kaydet–aç
aynı sonucu versin; yeni bir ölçek seçici eklemek çözüm sayılmasın.

**1 Kasım ilk dilim — [ADR 0817](../docs/adr/0817-unit-correction-keeps-coordinate-frames.md):**

- [x] “Sayıları koru” birim düzeltmesinde geçerli/kayıtlı UCS başlangıçları
  çizimle birlikte ölçeklenir; eksen yönleri korunur. Tek Undo/Redo hepsini
  birlikte geri getirir. Boyları koruyan birim değişimi, sonraki bağımsız
  UCS seçimini Undo ile ezmez.
- [x] Gerçek GUI testinin yakaladığı ikinci hata giderildi: Proje ayarları
  taslağı dönüştürülmüş UCS'yi eski değerle geri yazmıyor. Sınır dışı UCS
  plan aşamasında açıklanıp yazmadan reddediliyor; onayda doğru nesne sayısı
  ve çarpan, UCS etkisi gösteriliyor. 4×3m oda boyları korununca 12m² kalıyor.
- [x] 270 uygulama + 1560 masaüstü testi, 15 Python testi, 3 karma blok birimi
  ve 7 sayısal hassasiyet testi geçti; gerçek Apple M5/Metal GPU kontrolü
  çalıştırıldı. 293 isteğe bağlı test atlandı. Python dokümanı/katalog ve
  üretim kontrolü güncel; görüntüler repo dışında.
- [x] Aynı 1m eleman ve 4×3m örneği için mm/m/inç, karma kaynak blok
  birimleri, açma/ekleme/pano/yerleştirme kabulü tamamlandı. Gerçek GUI'de
  mm→m import 12m², inç→mm pano ve 1000mm blok, tek Undo, kaydet–aç,
  (500000,4400000) koordinatında 30° UCS/kamera ve 2mm ayrıntı doğrulandı.
  3×3 kaynak/hedef matrislerinin tamamı Python komut hostunda sınandı;
  her matris hücresi için ayrı GUI testi yapıldığı iddia edilmez.
- [x] Birimsiz açmada seçimsiz onay kapalı, açık birimle fiziksel boyut
  önizlemesi var; Python/MCP aynı kararı zorunlu tutuyor. Çelişkili kaynak
  birimi içe aktarma önizlemesinde açık. İlişkili dizi/ölçü tarifi ve
  dinamik blok kuralları, ifadeler, anotasyon konumları birlikte düzeltildi.
  Tarama model aralıkları dönüşür; pafta kâğıt geometrisi korunur ve
  model boyu değiştirilince görünüşleri ayarlama uyarısı verilir.

**Kapanış — [ADR 0818](../docs/adr/0818-unit-source-decisions-and-live-recipes.md):**
9 yeni bütünlük kabul testi; mevcut Python birim/UCS/açma 15, MCP 33,
native uygulama 270, masaüstü 1561; gerçek Metal GPU ve hassasiyet
kontrolleri geçti. Native/web ayrıntı zoom sınırı 10000px/m; mevcut GPU
hata eşikleri korundu. Derleme, Clippy ve SDK kontrolü geçti. Görüntüler
repo dışında. Web'in eski koordinat okuyucusunda başlangıçta da bulunan
32 UCS/birimli ifade/ayak-inç test hatası ADR'de ayrı kayıtlıdır; masaüstü
kabulü bunları tamamlanmış web eşliği olarak saymaz.

### CAD-ADV-03 — Kalabalık çizimde seçimden toplu özellik düzenlemeye güvenli geçiş sağla [x]

**Öncelik:** P0. **Kod:** [C04], [C05].
**Dayanak:** QCAD tür süzgeci ve kaynak/görünen renk ayrımı. [R18], [R19]

- Mevcut pencere/kesişen/çit/benzerini seç, ifade süzgeci ve alt nesne
  seçimini aynı akışta sun. Tüm çizim, görünür kapsam, katman ve mevcut
  seçim içinde arama açık olsun; kalıcı katman süzgeci geçici seçimle karışmasın.
- Renk gibi özelliklerde “atanan değer” ile ByLayer/ByBlock çözülmüş
  “görünen değer” ayrı sorgulanabilsin. İç içe blok/xref parçası için
  seçimin tanımı mı yerleştirmeyi mi hedeflediği görünür olsun.
- Karma seçimde özellik paneli türle daraltılabilsin; değişikliğin kaç
  nesneye gideceği, hangi kilitli nesnelerin atlanacağı ve ortak/çeşitli
  değerler gösterilsin. Boş bırakılan “Çeşitli” alan toplu boş değer yazmasın.
- Mevcut hücre taslağı ve panel önbelleğini kullanarak düşük gecikmeyi
  koru. Geometri ölçümü her klavye olayında tekrarlanmasın; seçim/belge
  değişince eski taslak yanlış hedefe yazılmasın. Geçersiz giriş açıklanabilsin.
- Sık kullanılan seçim tarifleri mevcut sorgu dilini tekrar kullanarak
  paylaşılabilsin. Sonuç listesinden sahneye gitme, seçime ekle/çıkar/kesiştir
  ve kısa süreli izole etme, görünürlük/Undo semantiğini korusun.

**Kabul:** 50.000 nesneli karma çizimde yalnız belirli yükseklikteki
yazıları seçip değiştir; blok içi ByLayer ve explicit renk örneklerini ayır.
Yazarken gecikme, ilk karakter, Enter/Esc, seçim değişimi ve kilitli hedef
gerçek etkileşimle sınansın. Aynı sorgu Python/MCP'de aynı hedefleri bulsun.

**İlerleme — 2026-11-01:** Türle daralan özellik kapsamı, güncel hedef anahtarıyla
eski hücre taslağı koruması, tek Undo ile toplu yazı yüksekliği, geometrik
kaynağa ek CAD özellik sorgusu, iç içe blokta ByLayer/ByBlock/explicit renk
kaynağı ve yerleştirme/tanım ayrımı, mevcut Processing modeliyle paylaşılır
seçim tarifi uygulandı. 50.000 nesneli gerçek arayüz kabulünde 5.000 yazı
bulundu; yalnız 4.705 kilitsiz yazının yüksekliği değişti. Kaydedilen dosya ve
arayüz Undo'su bağımsız karşılaştırıldı. Python/MCP eşliği ve otomatik
koruma testleri geçti. Ayrıntı: [ADR 0820](../docs/adr/0820-selection-property-scope-and-batch-text-height.md).
Renk girişi ortak görsel renk seçici, adlandırılmış örnekler ve Katmana/Bloğa
göre seçenekleriyle düzenlendi; hex yazmak gerekmiyor. 1100 piksel genişlikte
panel ve sorgu formu gerçek uygulamada denetlendi.
**Tamamlama — 2026-11-01:** Kalıcı katman süzgecinden ayrı, kaydedilmeyen
geçici nesne yalıtımı/bitirme; çizim/etiket/picking/Processing görünürlüğü,
Undo/Redo'nun ve PDF/pafta çıktısının korunması uygulandı. Aynı komut
Python ve MCP'de kullanılabilir. 50.000 nesnede 5.000 hedefin yalıtılması
ve 45.000 nesnenin gizlenmesi otomatik sınandı. Açık hücreden başka nesneye
geçiş gerçek uygulamada tekrarlandı: 77 cm yalnız eski hedefe 0,77 m
uygulandı; yeni hedef ve kilitli yazı değişmedi. Tekli yükseklik alanının
birim eki okuması da aynı uzunluk okuyucusuna geçirildi. Dosya karşılaştırması,
gizli hedefe tıklama, yalıtım sırasında Undo ve yalıtım bitince Redo doğrulandı.
AutoCAD'in resmî ekran örneği incelenerek palet; tek tür/sayı kutusu ve yanındaki
küçük sorgu/yalıtım simgeleriyle sadeleştirildi. Ayrı büyük düğmeler, ikinci
menü ve tekrar eden özet kaldırıldı. Gerçek arayüzde karma tür seçimi, sorgu
kapsamı, minimum 240 mantıksal piksel panelde taşmama ve seçim boşken yalıtımı
tek tıkla bitirme sınandı. Günlükler ve ekran görüntüleri repo dışındadır.

### CAD-ADV-04 — Eğrisel çizim ve düzenlemenin gerçek geometrik sınırlarını kapat [x]

**Öncelik:** P1. **Kod:** [C06], [C07], [C08].
**Dayanak:** QCAD Trim'in destek denetimi/işlem ayrımı ve hızlı budama
örneği; her aracın desteklemediği türü açık söylemesi. [R20], [R17]

- Mevcut çizgi/yay/elips/NURBS araçlarını yeniden yazmadan ortak
  destek matrisi oluştur: hedef tür, sınır türü, delik/çoklu parça,
  değişken genişlik ve ilişki taşıma. Güvenli ret, eksik desteği gizlemesin.
- Köşe yuvarlamada çizgi–yay, yay–yay ve uygun daire teğetlerini
  aşamalı ekle. Birden fazla çözüm varsa tıklanan taraf ve önizleme
  seçimi belirlesin; ters yay, dar açı, sıfır yarıçap ve çökme açık ele alınsın.
- Buda/Uzat ve bölme/birleştirmede delik, çoklu parça, analitik yay,
  knot aralığı, genişlik ve alt nesne kimliğini koru. Desteklenmeyen
  alanı patlatmaya zorlamak yerine uygun sınır/region işlemi sunulsun.
- Elips/NURBS ötelemesinde kapalı biçimde kesin eğri her zaman mümkün
  değildir. Kullanıcı toleransı, hata üst sınırı, kaynak bağlantısı ve
  self-intersection/yarıçap çökmesi raporu olsun; ekran LOD'u hesap toleransı olmasın.
- Kontrol/fit noktaları, uç teğeti, süreklilik ve açık/kapalı durumunu
  sahnede düzenleme geliştirilsin. Yaklaşıklaştırma yalnız açık seçenekle
  olsun; kaynak eğriyi habersiz polyline'a çevirmesin.

**Kabul:** yaylı kapalı yol, delikli alan, elips ve ağırlıklı NURBS
üzerinde aynı düzenlemeleri yap. Teğetlik, uç sürekliliği, alan/uzunluk ve
hata bütçesi bağımsız başvuruyla doğrulansın; bağlı ölçü/tarama ve Undo
korunsun. Destek genişlemeden mevcut retler kaldırılmasın.

**Kabul — 2026-11-02, masaüstü ve yayımlanmış destek matrisi kapsamında:** Bağımsız çizgi/yay/daire
çiftleri için analitik dairesel yuvarlama, tıklanan taraf, yönlü G1,
uç dışına tıklama, sıfır yarıçap ve büyük koordinat kontrolleri eklendi.
İlişkili taramanın yeni bağlayıcıyı sınırına katması ve tek Undo doğrulandı.
İki köşeli değişken genişlikli yolun Buda/Uzat genişlik kaybı giderildi.
Delikli alan retleri korunarak Alan böl öneriliyor. Native/Python/MCP
aynı komutu kullanıyor. Gerçek arayüzde kaydedilen geometri ve Undo
bağımsız dosya karşılaştırmasıyla kontrol edildi; görseller repo dışındadır.
[Destek matrisi ve açık kabul işleri](../docs/cad/curve-editing-support.md),
[ADR 0821](../docs/adr/0821-analytic-circular-fillets.md).
Pahın iki kenar mesafesi ayrı olmalı; sahnede kenar sırası açık gösterilir.
Masaüstü değer kutusu gerçek metin alanına geçirildi: tıklama, seçme,
yapıştırma, tek Enter ve Esc iptali gerçek arayüzde doğrulandı. Kutunun
sabit konumu ve geçici kimliği yanlışlıkla sahneye uygulamayı önler.
Pahın iki mesafesi ve yuvarlama yarıçapı yazılırken aynı salt okunur komut
planıyla canlı gösterilir. Geçersiz değer kutuda kalır; fare eski değeri
uygulamaz. Tek Enter, sahne tıklaması ve sağ tuş menüsü aynı yazılı değeri
uygular. Menü Esc'si değer kutusunu kapatmaz. Gerçek GUI sonucu, kaynak
kimlikleri ve tek Undo bağımsız dosya okuyucusuyla doğrulandı.
[ADR 0823](../docs/adr/0823-live-corner-value-previews.md).
Dairesel yuvarlama tek yarıçaptır; farklı kenar mesafeli eliptik/geçiş
eğrisi ayrı sözleşmeyle araştırılacak, mevcut daire hesabı bozulmayacak.
Elips/fit/NURBS ötelemesinde açık yaklaşım/tolerans seçimi, rasyonel Bézier
paralelinin türev sınırı + f64 payı, kaynak/kopya raporu ve güvenli
tekillik/kesişim denetimi eklendi. Yeni sağ uç düğüm ekleme düzeltmesi
ve büyük koordinatlar bağımsız analitik eğrilerle doğrulandı. GUI'de
20 cm içe öteleme/0,01 mm tolerans, 60 cm katlanma reddi, yeniden
seçmeden ayar düzeltme ve tek Undo sınandı. [ADR 0822](../docs/adr/0822-user-toleranced-curve-offsets.md)
sayısal sınırları ve tam IEEE ispatı olmadığını açıklar; [Python/MCP örneği](../docs/cad/curve-offset-python.md)
aynı sözleşmeyi kullanır. Eğri düzenlemede görünür geçiş/denetim ayarları,
doğrudan nokta seçimi, koordinat girişi, uç teğeti önizlemesi, ağırlık
düzeltmesi ve Aç/Kapat gerçek GUI'de doğrulandı. G2 eşlemesi olmadığı
açıkça gösterilir. Fare paneldeyken Undo/Redo işaretlerinin eski kalması
giderildi. [ADR 0824](../docs/adr/0824-visible-curve-editing-and-handles.md).
Elips yayı uzatma çekirdekte varken native uç denetiminde engelleniyordu;
tıklama/çit/pencere aynı analitik hesaba bağlandı. Katlanmış yolda yanlış
uç kaydı düzeltildi. GUI/Python/MCP ve tek Undo doğrulandı.
[ADR 0825](../docs/adr/0825-native-elliptical-arc-extension.md).
Ağırlıklı NURBS ölçü bağları gerçek knot domain'inde izlenir; Kır'da
bitiş yeni parçaya aktarılır. Native seçim filtresindeki eksik destek
GUI testiyle bulunup giderildi. Denetim noktası taşıma, gerçek GUI
bölme, Python/MCP, bağımsız De Boor ve tek Undo doğrulandı.
[ADR 0826](../docs/adr/0826-nurbs-measurement-references.md).
Yaylı/delikli alan bölmede taramanın yanlış parçaya sıçraması giderildi;
ölçüler kaynak köşenin yeni parçasını izler. Çok parçalı yaylı çizgide
dokunulmayan parça, toplam uzunluk ve başvuruların yeni indeksi korunur.
Gerçek GUI, bağımsız Green integrali, kaydedilmiş UID/Undo, Python/MCP
ve regresyonlar geçti. [ADR 0827](../docs/adr/0827-curved-area-split-reference-identity.md).
Bu kabul destek matrisindeki sınırları silmez: genel elips/NURBS
Birleştir, NURBS Uzat ve asimetrik Blend uygulanmış özellik sayılmaz.
Asimetrik geçişin [ayrı geometrik sözleşme araştırması](../docs/cad/asymmetric-curve-blends.md)
tamamlandı; daire yuvarlamaya sahte ikinci yarıçap eklenmedi. Web, ADR 0228
uyarınca ertelidir; masaüstü maddesini kapatmak için web geliştirilmez.

### CAD-ADV-05 — Parametrik 2B tasarımı açıklanabilir ve kapsamlı hale getir [x]

**Öncelik:** P1. **Kod:** [C09], [C10].
**Dayanak:** AutoCAD geometrik/ölçüsel kısıt ayrımı ve BricsCAD parametre
bağımlılığı inceleme yaklaşımı. [R03], [R13]

- Mevcut çözücü, parametre ifadeleri ve teşhis sorgusunu kullan.
  “Bu değere getir” ile “bu bağı koru”; süren ölçü ile sadece ölçen
  referans ölçüsü arayüzde açıkça ayrılsın.
- Serbestlik, tutan/çelişen/gereksiz/kopuk bağlar sahnede ilgili
  geometriye bağlansın. Kullanıcı bir parametrenin neyi değiştirdiğini
  ve değerin hangi ifadeden geldiğini bulabilsin; teşhis yeni motorla hesaplanmasın.
- Destek genişlemesini aşamalı yap: önce path yayının değişken bulge/
  yarıçap davranışı ve elips; sonra ihtiyaca göre NURBS uç/kontrol/teğet
  adaptörleri. Tüm eğriyi sınırsız çözen sistem vaadi yerine destek matrisi yayımlansın.
- Aynalı/dönük blok yerleştirmeleri, kilitli referans ve geçici
  sürükleme hedefinin mevcut kısıtlarla etkileşimi tasarlansın. Bir bağ
  çözülemiyorsa neden ve etkiler gösterilsin; tolerans büyütülerek çözülmüş sayılmasın.
- Parametre adı değiştirme, birim boyutu, döngü, sıfıra bölme ve
  çözüm dalları yönetilsin. Kullanıcı değişikliği kısıt çatışmasına
  yol açtığında eski geçerli model atomik korunsun; bağı geçici gevşetme açık seçim olsun.

**Kabul:** delikli 2B bağlantı plakası veya genel teknik detay üzerinde
ölçü/teğet/eşitlik/konum bağları kur. Parametre değiştir, kısmi kısıtlı
şekli sürükle ve çelişki oluştur. UI/Python/MCP aynı teşhis ve geometriyi
versin; çözüm dalı kullanıcı niyetine aykırı sessiz sıçramasın.

**Kabul — 2026-11-02:** Parametre penceresi ortak
query teşhisini ve gerçek serbestlikleri gösterir; bağımlılık inceleme,
rename ve ilgili geometriyi seçme, uzunluk/açı/sayı tür seçimi eklendi.
Tuşlarda çözücü tekrar çalışmaz; Undo teşhisi yeniler. Hata/sonuç görünür
üst bölümdedir. Gerçek GUI'de iki delikli plakada rename/değer/atomik
hata ve Python eşitliği doğrulandı. [ADR 0828](../docs/adr/0828-explainable-parameter-window.md),
[kodla doğrulanmış destek ve açık işler](../docs/cad/parametric-support.md).
Elips merkezi ve gerçek Start/End için mevcut çözücü adaptörü eklendi;
yarı eksen/açı bilinmeyenleri, dual türevler, tam elipsin kapalı kalması
ve dal sınırları test edildi. Gerçek GUI'de dört bağ fareyle kuruldu;
4→6 m, bağlı çizgi, tek Undo ve Python ile nesne/UID/teşhis eşitliği
doğrulandı. [ADR 0829](../docs/adr/0829-native-ellipse-point-constraints.md).
Kısıt başarısının hemen tekrar hover hatasıyla örtülmesi düzeltildi;
gerçek GUI’de mm/grad sonuçları ve sıfır olmayan küçük sayı dar/geniş
pencerede doğrulandı. [ADR 0830](../docs/adr/0830-constraint-success-feedback-and-parameter-units.md).
Elips merkez–uç yönünde simetrik 90° başlangıcın yanlış çelişki/tekillik
üretmesi düzeltildi. Gerçek arayüzde 4/2 m eksenler, bağlı çizgi,
query/Python eşitliği ve tek Undo doğrulandı. [ADR 0831](../docs/adr/0831-ellipse-ray-direction-without-axis-collapse.md).
Sınırlı açık NURBS Start/End adaptörü eklendi: dört uç bilinmeyeni;
iç kontrol noktaları, derece, düğüm ve ağırlıklar korunur. Gerçek GUI’de
dört bağ, 6→8 m ve izleyen çizgi, kapsam notu dar/geniş pencere, açıklamalı
destek dışı ret, dosya/Python eşitliği ve tek Undo doğrulandı.
[ADR 0832](../docs/adr/0832-bounded-nurbs-endpoint-constraints.md).
Aynalı/dönük ve ayrı X/Y ölçekli blokta Node(0) yerleştirme adaptörü
kabul edildi: gerçek GUI'de 6→8 m, izleyen çizgi, fare/ölçülü grip,
öznitelik konumları, kapsam notu ve tek Undo; Python/MCP ortak komut
kabulü geçti. [ADR 0833](../docs/adr/0833-block-insertion-anchor-constraints.md).
Geçici ilk-nesne/grip tutuşunun kalıcı Fix'i gereksiz sayması ve serbestlik
sayısını azaltması düzeltildi. Gerçek hover/tek tıklama/query/Python/MCP
ve tek Undo kabulü geçti. [ADR 0834](../docs/adr/0834-permanent-diagnosis-after-temporary-holds.md).
Elips yayı Start/End grip hedefi, çözülmüş yay/izleyen çizgi önizlemesi,
büyük koordinat doğruluğu ve kalıcı Fix çatışması gerçek Mac arayüzünde
kabul edildi; ölçülü tek Enter, dosya/Python/MCP ve tek Undo geçti.
[ADR 0835](../docs/adr/0835-elliptical-endpoint-grip-targets.md).
Sahne kısıt etiketleri panel sınırında tutuldu; ortak nokta grubu satıra
sığdırıldı. Sahne/pencere birim gösterimi ortaklaştırıldı; gerçek GUI'de
m, mm/gon, hover ve ikinci çizimde temiz önbellek doğrulandı.
[ADR 0836](../docs/adr/0836-constraint-label-viewport-and-units.md).
Yaylı path kenarı için ArcEdge adaptörü kabul edildi: değişken eğrilik,
korunan uç/genişlik/çözüm dalı ve ortak uçta teğet. Gerçek arayüzde büyük
koordinatta altı bağ, 2.5→3 m, altı geçerli bağ, dosyayı yeniden açma,
baytları tam geri getiren tek Undo ve 1.5 m atomik ret doğrulandı;
UI/Python geometri farkı 0. [ADR 0837](../docs/adr/0837-variable-circular-path-edge-constraints.md),
[saklama doğrulaması](../docs/adr/0838-stored-constraint-roundtrip-certificate.md).
İmkânsız yarıçap/çap, iki açık Sabit köşesi varsa kiriş ve asgari
ölçüyle açıklanır. Gerçek mm GUI'de hata/üç ilgili bağ, geçerli değere
dönüş, UI/Python eşitliği ve tam tek Undo kabulü geçti.
[ADR 0839](../docs/adr/0839-actionable-fixed-arc-radius-refusals.md).
Boyut cebiri ve eski ifade uyumluluğu eklendi: yeni/açıkça değişen ifadede
uzunluk/açı/sayı, çarpım/üs/karekök ve değişen üs bağımlısı denetlenir.
Eski pi yarım tur değerini korur; ortak query ve arayüz uyumsuzluğu
açıklar. Gerçek mm GUI'de yanlış tür reddi, 2500→3000 mm, altı geçerli
bağ, tam tek Undo, dar pencere, açık pi onarımı ve yeniden açma geçti;
UI/Python dosyaları tam aynı. [ADR 0840](../docs/adr/0840-parameter-dimensions-without-legacy-reinterpretation.md).
Tam elips eksen tutamacının kaynak dikiş açısını sıfırlaması düzeltildi;
eksen sırası değiştiğinde aynı bağlı nokta yeni baza taşınır. Gerçek
arayüzde 4→6 ve 4→1.5 m, bağlı çizginin doğru koordinatı, iki geçerli
bağ, beş serbestlik, tek Undo, yeniden açma ve tam GUI/Python dosya
eşitliği doğrulandı. [ADR 0841](../docs/adr/0841-full-ellipse-seam-through-axis-grips.md).
Tam elipste sürüklenen eksen artık açık geçici hedeftir; diğer serbest
boyutlar kalıcı ölçüye uyum sağlar. Gerçek GUI'de 2→3 m, eksen sırası
değişimi, üç geçerli bağ/dört serbestlik, önizleme iptali, açıklamalı Fix
çatışması, yeniden açma, tam GUI/Python dosya eşitliği ve tek Undo geçti.
[ADR 0842](../docs/adr/0842-explicit-full-ellipse-axis-grip-targets.md),
[Python/MCP rehberi](../docs/cad/ellipse-axis-grips-python.md).
Birleşik büyük koordinat teknik plakasında dokuz nesne/24 bağ/yedi
parametre kabul edildi: eşit delikler, teğet kılavuz, bağımlı genişlik,
yalnız ölçen köşegen, kısmi elips sürüklemesi, atomik sabit/kilit reddi ve
açık gevşetme. UI/Python dosyaları tam aynı; MCP, kaydet/aç ve tek Undo
geçti. Yakın bağımsız kısıt çubuklarının ölçü yazılarını örtmesi düzeltildi;
gerçek hover, dar pencere ve zoom kontrol edildi. Temiz adayda masaüstü
1583, MCP 49, Python kısıt 19 test ve Clippy geçti.
[ADR 0843](../docs/adr/0843-combined-parametric-plate-and-readable-bars.md),
[tekrar çalışma rehberi](../docs/cad/technical-plate-constraints.md).
Kabul destek matrisindeki aşamalı kapsama aittir; genel NURBS/G2 ve
kalıcı bağımsız elips eksen başvurusu uygulanmış sayılmaz.

### CAD-ADV-06 — Parametrik blokları sürdürülebilir 2B bileşenlere dönüştür [~]

**Öncelik:** P1. **Kod:** [C11], [C12].
**Dayanak:** Smart Blocks ve parametre/design-table düzeni; blok kaynağı
ile örnek değerinin ayrılması. [R02], [R13], [R22]

- Mevcut dinamik blok parametre/eylem/görünürlük/lookup ve gerçek boy
  kitaplığını koru. İleri kapsam: bileşen türü/sürümü, tip–örnek
  parametresi, okunabilir parametre grubu ve değişiklik etki önizlemesi.
- Aynı bileşenin katalogdan gelen değerleri, kullanıcının değiştirdiği
  örnekleri ve formülle hesaplanan alanlar ayrışsın. Aynı etiketin
  yeniden adlandırılması kullanıcı verisini yeni boş varsayılanla ezmesin.
- İç içe bloklarda tanım güncellemesi ve öznitelik eşitleme politikası
  açık olsun: hangi seviyeler değişecek, hangi örnek override'ları
  korunacak? Mevcut sadece çizim yerleştirmelerini eşitleyen yol sessizce genelleştirilmesin.
- Kitaplık güncellemesi önce kullanılan örnekleri, ölçü/tarama bağını
  ve tablo sonuçlarını önizletsin. Kullanıcı sürümü tutabilsin veya
  seçilmiş kapsamı yükseltebilsin; her dosya açılışında sessiz güncelleme yapılmasın.
- Blok düzenleyicide test yerleştirmesi ve parametre sınırları
  görülebilsin. Dışa aktarımda düzenlenebilir parametre kaybı ile statik
  görüntü koruması farklı raporlansın; proxy'nin doğru çizilmesi davranış uyumluluğu sayılmasın.

**Kabul:** iki görünürlük durumu, lookup ve birimli formülü olan genel
2B bileşeni iç içe yerleştir. Bir özniteliği yeniden adlandırıp tip
güncellemesi yap; örnek değerleri ve bağlı ölçü korunsun. Kaydet–aç,
geri al ve desteklenmeyen hedefe raporlu statik aktarımı sınanabilsin.

**İlerleme — 2026-11-02, tamamlanmadı:** Masaüstü öznitelik satırı
kaynak etiketini korur; yeniden adlandırma mevcut `renames` komutuna geçer.
R-1/R-2/R-3 değerlerini silen hata gerçek eski GUI'de üretildi ve düzeltildi.
Tanım değişiminde başka hedef verisi korunur; eşzamanlı değer/özel konum
swap'ı ve eski açık sync sözleşmesi geçti. Gerçek GUI/Python dosyaları
tam aynı; tek Undo, yeniden açma, dar pencere ve çakışma/iptal kabul edildi.
Temiz adayda 1584 masaüstü, 317 application, 50 MCP, dört Python testi,
Clippy ve dosya doğrulaması geçti.
[ADR 0844](../docs/adr/0844-block-attribute-rename-identity.md),
[Python/MCP rehberi](../docs/cad/block-attribute-renames.md).
**İlerleme — 2026-11-02, birim paneli:** Doğrusal/nokta parametreleri
proje uzunluk birimiyle, açı derece/grad ayarıyla okunur ve düzenlenir.
Fiziksel/birimsiz tanım ayrımı ve ilave örnek ölçeği korunur; mm/cm/m/in/ft,
formül, lookup, sınır dışı/bozuk giriş ve tek Undo kabulü geçti. Gerçek
GUI'de 900→1050 mm düzenlemesinin dosyası Python ile tam aynı, GUI tek
Undo dosyası ilk durumla tam aynı; geniş ve dar pencere görüldü.
1588 masaüstü ve 50 MCP testi, derleme ve Clippy geçti.
[ADR 0845](../docs/adr/0845-block-parameter-project-units.md).
Bu, aşağıdaki ileri kapsamın bittiği anlamına gelmez.

**İlerleme — 2026-11-02, kayıt öncesi test:** Blok düzenleyicide **Test et**,
kaydedilmemiş güncel geometriyi ayrı yerleştirmede dener. **Düzenleyiciye dön**
taslak/geçmiş/seçimi korur; deneme parametreleri asıl çizime sızmaz. Panelde
varsayılan–örneğe özel–hesaplanan ayrımı ve birimli min/max/adım görünür;
uzun açıklamalar tam genişlikte satıra yayılır. Testte pencere küçültülüp
dönüldüğünde kamera güncel alan boyutuna uyar. Gerçek GUI'de çizilen kol,
1050 mm ve Sade denemesi, dar pencere, tekrar test, kayıt engeli, normal
Kaydet/Python dosya eşitliği, tam tek Undo ve iptal dosyası doğrulandı.
1591 masaüstü testi, derleme ve Clippy geçti; belgeli bağımsız Python test
örneği çalıştırıldı. [ADR 0846](../docs/adr/0846-block-editor-test-placement.md),
[kullanım/API rehberi](../docs/cad/block-test-placement.md).

**İlerleme — 2026-11-02, değişiklik etkisi:** Düzenleyicide **Etkiyi gör** güncel
taslağın `cad.blocks.edit` ortak planını ayrı iş parçacığında çalıştırır;
gerçek değişen/bağlı kayıtları, doğrudan/iç içe yerleştirmelerden ayrı gösterir.
Kilitli katman, üst tanım adları ve bağlı ölçü/tarama/çizelge okuyucuları görünür.
Kapat/Esc yazmaz; eski istek ve değişmiş taslak sonucu reddedilir. Gerçek GUI'de
3 doğrudan/2 iç içe/2 kilitli, 1 ölçü/1 tablo kabulü ve dar pencere geçti;
GUI/Python kayıtları tam aynı, GUI tek Undo başlangıçla tam aynı. 1595 masaüstü,
51 MCP ve Python kabulü geçti. [ADR 0847](../docs/adr/0847-block-editor-update-impact.md),
[kullanım/API](../docs/cad/block-update-review.md).

**İlerleme — 2026-11-02, parametre grupları:** Boyut/konum, görünüş, katalog
ve hesaplanan grupları bağımsız kapanır; tanım/sürüm ve örnek kapsamı açıklanır.
Değerler menüsü tek parametreyi veya katalog girdilerini ortak `reset/names`
komutuyla sıfırlar; diğer override'lar korunur. Hesaplanan sonuç değişiklik
sayacına eklenmez. 1598 masaüstü, 52 MCP, Python testi, derleme ve Clippy geçti.
Gerçek GUI/Python önce–sonra dosyaları ve tek Undo tam aynı; dar pencere,
açık/koyu tema, Büyük yazı ve menüler görüldü.
[ADR 0848](../docs/adr/0848-block-parameter-groups-and-named-reset.md).

**İlerleme — 2026-11-02, güvenli tanım değiştirme:** Panelin Blok alanı artık
ortak `cad.blocks.replace` yolunu kullanır. Aynı ad/türdeki kullanıcı girdileri
korunur, hedef formülleri hesaplanır; uyumsuz hedef atomik reddedilir. Eski
başarılı görünüp kaydedilemeyen dinamik→düz blok hatası kapatıldı. 50 ortak
durum, 317 application, 1599 masaüstü, 53 MCP, dokuz Python blok testi,
derleme ve Clippy geçti. Gerçek GUI'de 1050 mm/Sade/K-1/bağlı ölçü korundu;
GUI/Python kayıtları ve tek Undo tam aynı, yeniden açma geçti.
[ADR 0849](../docs/adr/0849-block-replacement-keeps-compatible-parameters.md).
Bu, aile/sürüm tanıyan kitaplık yükseltme arayüzünü tamamlamaz.

**İlerleme — 2026-11-02, seçilmiş değiştirme önizlemesi:** Bloklar sağ menüsü
hemen bütün örneklere yazmaz; seçili/tüm doğrudan kapsamı ve gerçek komut
etkisini gösterir. Onay aynı plan kimliğiyle yürür; eski sonuç veya değişmiş
belge reddedilir. Gerçek GUI'de 1/3 kapsam, uyumsuz hedef, Esc, dar pencere,
Python ile tam kayıt eşitliği ve her iki kapsamın tam tek Undo'su geçti.
1603 masaüstü ve 429 Python testi, derleme ve Clippy geçti.
[ADR 0850](../docs/adr/0850-block-replacement-scope-and-guarded-preview.md).
Bu seçim mevcut çizim tanımları arasındadır; harici kitaplığın aile/yayın
sürümünü izleyen yükseltme halen açıktır.

**İlerleme — 2026-11-02, iç içe statik aktarım:** Varsayılan ilk görünüşün
gizli parçaları özel değer yokken de gizlidir. Varyant parça sırası değişince
etiketin kaybolması gerçek GUI tekrarında yakalandı ve aynı varyant önbelleğiyle
düzeltildi. DXF bütün iç içe referansları kendi değerlerinde sabitler; aynı
çocuk varyantını paylaşır, kullanılmayan kitaplık tanımlarını tutar ve
parametre/eylem kaybını ayrı uyarıyla bildirir. 315 çekirdek, 317 application,
881 etkileşim, 82 headless, 54 MCP, 1603 masaüstü ve 431 Python testi;
derleme ve Clippy geçti. DXF penceresinin birim açıklaması gerçek çıktı
birimiyle eşitlendi. Gerçek GUI'de 900/1200 mm ve iki görünüş, korunmuş etiketler,
DXF tekrar açılışı görüldü. GUI/Python DXF baytları ve aktarım öncesi/sonrası
kaynak KCAD tam aynı; görünüş değişimi/tek Undo ayrıca kayıttan denetlendi.
[ADR 0851](../docs/adr/0851-dynamic-block-default-visibility-and-nested-dxf.md),
[aktarım/API rehberi](../docs/cad/block-static-dxf.md). Bu dilim aile/yayın
sürümü veya kalıcı tip–örnek kapsamını tamamlamaz.

**İlerleme — 2026-11-02, kitaplık güncellemesinde kaynak güvenliği:**
Kitaplık veya dosyadan yeniden tanımlama, sahnedeki mevcut parametre ve
özel öznitelik konumlarını da birleşik son tanımlara göre doğrular.
Önceden başarılı görünüp sonraki kaydı bozan dinamik→düz değişim,
daraltılmış değer aralığı ve artık taşınamayan özel konum atomik reddedilir.
317 application, 55 MCP, 1603 masaüstü, 436 Python testi, Clippy ve derleme
geçti. Gerçek GUI konsolundan iki import yolunun reddi, uyumlu geometri
uygulama ve gerçek tek Undo denetlendi; ret/Undo kayıtları başlangıçla,
uyumlu GUI kaydı bağımsız Python ile bayt düzeyinde aynı.
[ADR 0852](../docs/adr/0852-block-library-replacement-validates-scene-inserts.md),
[API rehberi](../docs/cad/block-library-update-safety.md). Değişen formül ve
birim semantiği ayrıca ele alınacak; bu koruma aile/yayın yükseltmesi değildir.

**İlerleme — 2026-11-02, bağlı kitaplık tanımlarını birlikte uygulama:**
Var olan ana tanım yeni çocukla veya çocuktaki parametre adı değişimiyle
aynı işlemde güncellenebilir. Ara tanımları tek tek doğrulayan sıra hatası
kapatıldı; birleşik son grafik denetlenip tek Undo/Redo olarak yazılır.
134 domain, 317 application, 56 MCP, 1603 masaüstü ve 440 Python testi,
Clippy ve derleme geçti. Eski GUI'de hem konsol planı hem sağ tuş dosyadan
yeniden tanımlama hatası üretildi; yeni GUI aynı dosya/menüyle geçti.
Her iki GUI yolunun kayıtları Python ile, gerçek tek Undo kayıtları
başlangıçla tam aynı. Araya farklı işlem giren tekrar kabul dışı bırakılıp
temiz tekrar yapıldı. [ADR 0853](../docs/adr/0853-related-block-library-definitions-apply-together.md).
Geçmiş kartının yalnız tanım değişince “değişiklik yok” özeti vermesi
GUI'de tespit edildi ve ayrıca düzeltilecek; kayıt doğruluğu bununla karıştırılmadı.

**İlerleme — 2026-11-02, geçmişte doğru sonuç:** Nesne listesi değişmeden
blok tanımı değiştiğinde kart artık “Çizim güncellendi” der; gerçek no-op'un
“değişiklik yok” ve Undo'nun durum eki korunur. Aynı dosyanın gerçek sağ tuş
menüsüyle iki kez alınması ve tek Undo ekranlarıyla doğrulandı. 1604 masaüstü
testi, Clippy ve derleme geçti. [ADR 0854](../docs/adr/0854-history-revision-fallback-for-definition-writes.md).

**İlerleme — 2026-11-02, kitaplık formüllerinin güncellenmesi:** Yeni tanım
mevcut örneklerin ve değişmeyen ana bloklardaki etkilenen iç yerleştirmelerin
hesaplanan değerlerini kendi literal girdileriyle yeniden hesaplar. Tek
örneğin hesabı tanımsızsa bütün güncelleme yazmadan reddedilir. Gerçek eski
GUI'de kalan 450/500/450 mm önbelleği üretildi; yeni GUI aynı kitaplıkla
300/333,333…/300 mm ve doğru geometri verdi. GUI/Python kayıtları tam aynı;
gerçek tek Undo ve ret kayıtları başlangıçla tam aynı, gerçek yeniden açılış
korundu. 318 application, 57 MCP, 1604 masaüstü ve 444 Python testi, Clippy
ve derleme geçti. [ADR 0855](../docs/adr/0855-block-library-refreshes-formula-values.md).
Birim semantiği ve aile/yayın yükseltmesi bu düzeltmeyle tamamlanmaz.

**İlerleme — 2026-11-02, aile/tip/yayın kimliği:** Blok kartı isteğe bağlı
kalıcı aile ve tip UUID'si, okunabilir tip adı ve yayın numarası taşır;
yerel değişiklik sayacından ayrıdır. KCAD 158, kitaplık sürüm 2, Python ve
MCP aynı kimliği korur; kimliksiz eski dosya/kitaplık davranışı korunur.
Gerçek GUI'de tip/yayın düzenleme, sıfır numara reddi, minimum pencerede
ayrıntılar ve sabit alt düğmeler, kitaplıkta tip/aile araması doğrulandı.
GUI/Python kayıtları tam aynı; tek Undo başlangıçla, Redo/Vazgeç yeni
kayıtla tam aynı, gerçek yeniden açılışta yayın 3/yerel sayaç 7 korundu.
1163 contracts, 234 KCAD, 319 application, 58 MCP, 1605 masaüstü ve 448
Python testi, Clippy ve derleme geçti. [ADR 0856](../docs/adr/0856-block-component-publication-identity.md),
[kimlik/API rehberi](../docs/cad/block-publications.md). Bu kimlik temeli
seçilmiş yayın yükseltmesini veya tip kapsamındaki parametreyi uygulamaz.

**İlerleme — 2026-11-02, seçilmiş kitaplık yayını yükseltmesi:** Kitaplık
sağ menüsündeki karşılaştırma aynı aile/tipin daha yeni yayınını, bağımlılık
grafiğiyle ayrı alır. Eski tanımlar korunur; aynı yayının ikinci kapsamı
mevcut hedefi kullanır. Ortak `cad.blocks.replace.library` korumalı planı
Python ve MCP'de de aynı işlemi yürütür. Gerçek GUI'de 1/3 ve 3/3 önizleme,
Esc, tek tıkla yalnız ortadaki kapı, eski/yeni sayıları 2/1, gerçek yeniden
açılış, minimum pencere ve ikinci kapsamın aynı hedefi kullanması doğrulandı.
GUI/Python kayıtları tam aynı; gerçek tek Undo/Redo ve ret sonrası kayıtlar
tam eşit. Uyumsuz yayın 4 gerçek menüde genişlik girdisini kaybetmek yerine
reddedildi; Değiştir pasifti. 319 application, 59 MCP, 1606 masaüstü, 452
Python testi, ilgili çekirdek/pencere testleri, Clippy ve derleme geçti.
[ADR 0857](../docs/adr/0857-selected-block-publication-upgrade.md),
[kullanım ve API](../docs/cad/block-publication-upgrade.md). Bu dilim etiket
veya parametre yeniden adlandırmasını kendiliğinden tahmin etmez.

**İlerleme — 2026-11-03, kalıcı kullanıcı parametre grupları:** Dinamik
blok sağ menüsünde başlıklar yazılır veya mevcut ad listesinden seçilir;
otomatik başlığa dönüş ve grup sırası açıktır. Ortak
`DynamicBlock.parameterGroups` grup/üye sırasını KCAD 159, kitaplık ve
Python/MCP'de korur. Hesaplama sırası, geometri, yerleştirme değerleri ve
yerel geometri sayacı değişmez. Özel başlıklar bağımsız kapanır; kalan
otomatik başlıklar ayırt edilir. Gerçek GUI'de ret, iptal, dar pencere,
yeniden açma, tek Undo/Redo ve grup içinden 1→1.05 m ölçü düzenleme kabul
edildi. GUI/Python dosyaları üç işlemde tam aynı; gerçek Undo kayıtları
önceki baytlarla tam aynı. 1612 masaüstü, 319 application, 60 MCP, 457
Python, 234 KCAD testi, bağımsız 1501 dosya, Clippy ve derleme geçti.
[ADR 0858](../docs/adr/0858-user-defined-block-parameter-groups.md),
[gruplama ve sıfırlama/API](../docs/cad/block-parameter-groups.md).

**İlerleme — 2026-11-03, dinamik blok birimi düzeltmesi:** Gerçek GUI'de
üretilen “geometri küçülüyor, tutamaç ve kayıtlı genişlik eski kalıyor”
hatası giderildi. `Sayıları koru` statik/dinamik koordinat, sınır/adım,
katalog değeri, formül, kök/iç yerleştirme girdisi, taşınmış öznitelik
yeri ve kırpmayı aynı işlemde dönüştürür. Üst/dünya konumu ve ilgisiz
çocuk tanımı korunur. Etkilenen kilitli doğrudan/üst kullanım, sayı taşması
ve geçersizleşen kırpma yazısız reddedilir. Sadece birim etiketi veya
aynı birim seçimi dinamik geometrinin sayısını değiştirmez. Gerçek
menüden iki boyut aralığında dönüşüm, tek Undo/Redo, 1→1.05 m düzenleme
ve kaydet–aç yapıldı; GUI/Python dosyaları tam aynı. 330 application,
1612 masaüstü, 61 MCP ve 459 Python testi, Clippy ve derleme geçti. Tip–örnek kapsamı
bu hata düzeltmesiyle tamamlanmış sayılmaz.
[ADR 0859](../docs/adr/0859-scoped-dynamic-block-unit-correction.md),
[kullanım/API](../docs/cad/block-unit-correction.md).

**İlerleme — 2026-11-03, formül ve kayıt güvenliği:** Kutupsal uzaklık
formülünün sayı saklayıp gerçek Kaydet/Kurtarma işlemini bozduğu hata
giderildi; sonuç başlangıç ve yönü koruyan noktadır. Formül veya okuduğu
varsayılan değişince boş kökler ve üst tanımların iç kullanımları da
hesaplanır. Eski hesap önbelleği girdi sayılmaz; bir örneğin hesabı
geçersizse literal genişlik/yön silinmeden bütün işlem reddedilir. Gerçek
GUI'de 2.1→2.2 m hesap, 1.05→1.10 m tek Enter, kaydet–aç, tek Undo/Redo ve
sıfıra bölen formülde yazısız ret doğrulandı. GUI/korumalı Python dosyaları
ve geri alma kayıtları bayt düzeyinde aynı; 10 dosya bağımsız okuyucudan
geçti. 339 application + ayrı çekirdek çizim, 1612 masaüstü, 62 MCP,
462 Python testi, paylaşılan 14801 komut senaryosu, Clippy ve derleme geçti.
Bu dilim kalıcı tip–örnek kapsamının yerine geçmez.
[ADR 0860](../docs/adr/0860-polar-formulas-and-complete-reference-refresh.md),
[formül/API rehberi](../docs/cad/block-parameter-formulas.md).

**İlerleme — 2026-11-03, formül zinciri:** İleri başvurudaki geçerli
formülün ara varsayılandan yanlış reddi ve döngünün kabulü gerçek GUI'de
üretildi. Formüller artık gerçek lexer ve motorun Türkçe/harf eşlemesiyle
bağımlılık sırasına konur; her son sonuç bir kez denetlenir. Döngü
adlarıyla yazısız reddedilir. Varsayılan değişimi ve blok/proje birim
uyarlaması aynı girdi eşlemesini kullanır. Gerçek GUI'de korumalı ekleme,
1.10→1.15 m tek Enter, döngüde veri koruma, çizim odağında Undo/Redo,
kaydet–aç ve menüden `@genislik` formüllü mm→cm sayı koruma doğrulandı.
GUI/Python dosyaları tam aynı; 16 dosya bağımsız okuyucudan geçti.
348 application, 1612 masaüstü, 63 MCP ve 464 Python testi, son kaynakta
hedefli tekrarlar, tüm hedeflerde Clippy ve derleme geçti.
[ADR 0861](../docs/adr/0861-block-formula-dependency-order.md).

Kalıcı tip–örnek parametre kapsamı ve birleşik blok kabulü açıktır. Web öznitelik penceresinin native eşitleme kapsamıyla
aynı olmadığı kodda görüldü; bu dilim web eşitliğini tamamlanmış saymaz.

### CAD-ADV-07 — Tekrarları tanı, blok ve ilişkili diziye kontrollü dönüştür [ ]

**Öncelik:** P1. **Kod:** [C11], [C13], [C03].
**Dayanak:** BLOCKIFY ve ARRAYDETECT, bulunan adayları kullanıcıya
inceletip seçilmiş dönüşüm sunuyor. [R11], [R12]

- Mevcut ilişkili dizi modelini tekrar kurma. Bağımsız çizilmiş veya
  patlatılmış tekrarlı 2B geometriyi blok/diziye dönüştürebilen aday
  bulma katmanı ekle; önce kesin/dönüşümle eşdeğer eşleşme teslim edilsin.
- Döndürme, aynalama, ölçek ve öznitelik/metin varyantları ayrı
  eşleşme ölçütleri olsun. Geometrik benzerlik, anlamca aynı bileşen
  olduğunun garantisi sayılmasın; hangi özelliklerin göz ardı edildiği belli olsun.
- Grid/kutupsal/yol örüntüsünde eksik veya elle değiştirilmiş öğeler
  mevcut `ArrayItem` semantiğine eşlensin. Yol yönü, başlangıç, hizalama,
  boşluk ve öğe kimliğinin değişimde nasıl korunduğu gösterilsin.
- Dönüşüm öncesi kaynak ID→yeni yerleştirme/alt nesne eşlemesi üret.
  Bağlı ölçü, field, tarama, grup ve tablo kaybı olacaksa plan bunu
  açıklasın; yalnız dosya boyu azalıyor diye bağlar silinmesin.
- AI tabanlı benzerlik sonradan opsiyonel eklenebilir. Aynı veriyle
  yeniden üretilebilir geometrik temel ve kabul/ret incelemesi her
  zaman bulunsun; eşleştirme görevi arka planda ve durdurulabilir çalışsın.

**Kabul:** dönük/aynalanmış 40 teknik sembol ve iki eksik öğeli sıra
üzerinde tanıma yap. Birbirine benzeyen farklı bileşen yanlış birleşmesin.
Seçilmiş adayları dönüştür, sayıyı/aralığı değiştir, bir öğeyi koru/sil/resetle;
kimlik eşlemesi, bağlı ölçü ve tek Undo doğrulansın.

### CAD-ADV-08 — Sınır, tarama ve maskeyi hatasını açıklayabilen bir üretim sistemine getir [ ]

**Öncelik:** P0 ilişki/görünürlük; P1 kapsam. **Kod:** [C14], [C15], [C03].
**Dayanak:** güncel CAD'lerde tarama ve hızlı budama iş akışı; KentOS'ta
tamamlanacak sınır teşhisi aşağıdaki tasarım önerisidir. [R01], [R17]

- Mevcut analitik sınır, linework yüz bulma, ada, metin/blok boşluğu,
  PAT/desen/degrade ve ilişkili tarama korunur. İleri iş sınırın neden
  bulunamadığını veya bağın neden bırakıldığını görünür kılmaktır.
- Küçük açıklık, yinelenen kenar, self-intersection, teğet temas ve
  açık uçlar sahnede vurgulansın. Geçici gap toleransı geometriyi sessiz
  düzeltmesin; kaynak onarımı ayrı komut/önizleme olsun.
- Delik/ada ve çoklu bölge davranışını açık tanımla. Sınır budanıp
  parçalandığında taramanın bölünmesi/yeniden bağlanması kontrollü olsun;
  kaynak kimliği ve desen orijini gereksiz yere değişmesin.
- Bir sınır silindiğinde taramanın yerinde kalması ile ilişkisinin
  kopması farklı durumdur. Mevcut sabitleme davranışını veri kaybetmeden
  teşhis/yeniden bağla eylemleriyle tamamla; çok sayıda tarama birlikte incelenebilsin.
- Desen yoğunluğunda sadeleştirme yalnız gösterime uygulansın.
  Yakınlaştırmada desenin kaybolup gelmesi, maske/çizim sırası ve
  PDF–ekran farkı denetlensin. Desen orijini, açı ve ölçek sahnede ölçülü düzenlenebilsin.

**Kabul:** yaylı dış sınır, iki ada, metin boşluğu ve küçük açıklık
içeren çizimi tara. Kaynağı buda/sil/düzelt, sonra yeniden bağla. Analitik
sınır, alan, desen orijini ve tek Undo korunmalı; durum değişimi kullanıcı
tarafından anlaşılmalı. Yakın/uzak görünüm ve PDF karşılaştırılmalı.

### CAD-ADV-09 — Ölçü, kılavuz ve teknik anotasyonun doğruluğunu görünür kıl [ ]

**Öncelik:** P0. **Kod:** [C16], [C03], [C26].
**Dayanak:** AutoCAD'in tam/kısmi/kopuk ilişki ve Annotation Monitor
ayrımı. [R04]

- Mevcut ölçü türleri, ilişki/yeniden bağlama, çok ölçekli temsil,
  merkez işaretleri ve çok kollu kılavuz korunur. Mevcut gerçek ölçü /
  gösterilen override ayrımı UI, tablo, API ve ön denetimde birlikte kullanılsın.
- Kopuk veya kısmi bağı sahnede bulup kaynağa gitme ve uygun alt
  nesneye yeniden bağlama sunulsun. Bildirim yoğunluğu çizimi
  okunmaz hale getirmesin; seçili nesne, kapsam ve toplu inceleme desteklensin.
- Genel teknik detaylarda paralel/zincir/baz/koordinat ölçüsü
  yerleşimi, ortak referans, metin sığması ve çakışma önizlemesi geliştirilsin.
  Mimari mahal/duvar ölçü tarifleri ikinci kez yazılmasın.
- Birincil/alternatif birim, limit/çift taraflı tolerans, ölçü
  yuvarlama, çap/yarıçap ve teknik çerçevelerin stil profilleri tutarlı
  olsun. Bir standarda uygunluk, sembolün çizilmesine bakılarak iddia edilmesin;
  seçilen sürüm ve gerekli kural kapsamı ayrıca doğrulansın.
- Kılavuz ucu nesneye/alt nesneye bağlı kalsın; metin taşırken
  ok/dirsek ve iniş çizgisi anlaşılır davransın. Blok değişimi ve
  kırpılmış viewport ölçüsünde yanlış referans veya çift ölçek hesabı önlensin.

**Kabul:** aynı genel 2B detayda ilişkili, kısmi ve override ölçüler
oluştur. Kaynağı böl, blok tanımını değiştir ve bir referansı sil; teşhis
ve yeniden bağlamayı dene. 1:20/1:50 paftalarda gerçek değer, kâğıt boyu,
birim/tolerans ve metin konumu tutarlı olsun; zoom gerçek değeri etkilemesin.

**Kodla doğrulanan ek açık — 2026-11-02:** `headless/query.rs::item`,
`entity_bounds(Shape::Insert)` yoluyla sadece ekleme noktasını döndürüyor;
`drawing.measure` blok kutusu görünen geometriyi kapsamıyor. İç içe, dönmüş
ve farklı birimli blokta etkin varyant/kırpma/katman kapsamını esas alan
kutuyu API ve GUI'de aynı kuralla doğrula. ADR 0851 aktarım testi bu eksik
kutuyla değil gerçek parça uçlarıyla doğrulanmıştır.

### CAD-ADV-10 — Yazı ve field üretimini ekran ile çıktıda aynı hale getir [ ]

**Öncelik:** P1. **Kod:** [C17], [C18], [C16].
**Dayanak:** Unicode karakter kümeleri ve HarfBuzz şekillendirme ayrımı;
shaping tek başına satır yerleşimi değildir. [R24], [R25]

- Mevcut MTEXT paragrafı, girinti/tab/sütun/kesir, text field ve font
  yollarını koru. Satır kırma, glyph şekillendirme, ölçülen kutu,
  hit-test ve PDF yerleşiminin aynı metni aynı biçimde ele aldığını kanıtla.
- Türkçe, birleşik Unicode karakterleri, gerekli semboller ve farklı
  yazı yönleri için açık destek matrisi oluştur. Düzenleme imleci ve
  silme görünür karakteri parçalamamalı; Unicode scalar indeksli kalıcı
  aralıkların göçü düşünülmeden değişik indeks sistemi eklenmemeli.
- Bir çıktı yolunda zaten kullanılan Rustybuzz desteğini bütün metin
  yollarında eşdeğerlik açısından değerlendir. Font fallback ve gerçek
  glyph ölçüleri ortak sözleşmeye bağlansın; ikinci bir rastgele font/ölçü motoru açılmasın.
- Eksik font, farklı metrikli fallback, gizli/bozuk field ve üstüne
  yazılan sabit metin anlaşılır gösterilsin. Field'ın kaynak değeriyle
  önbellekteki son metni ayırt edilsin; kaynak yoksa eski değer güncelmiş gibi basılmasın.
- Çoklu yazıda bul/değiştir, stil/biçim aktarımı, hizalama ve
  okunurluğu koruyan dönüşümler kontrollü toplu işlem olsun. CAD gerçek
  boylu metin davranışı, ekran okunurluğu uğruna gizli minimum boy kurallarıyla bozulmasın.

**Kabul:** Türkçe, birleşik aksan, teknik sembol, karışık yazı yönü,
sütun ve field içeren metin dosyasını aç/düzenle/yansıt/yazdır. Ekran,
ölçülen kutu ve PDF satır sonları karşılaştırılsın. Eksik font ve kaynak
silinmesi ön denetimde görülsün; eski dosyanın metni habersiz değişmesin.

### CAD-ADV-11 — Karmaşık çizgi tiplerini ve çizgi görünüşünü eksiksiz çiz [ ]

**Öncelik:** P1; yanlış teslimi önleyen uyarılar P0. **Kod:** [C19], [C31].
**Dayanak:** AutoCAD özel çizgi tiplerinde font, metin yönü, ölçek ve
ötelemenin ayrı anlamları var. [R05]

- `dashes_of` içindeki iki somut sınırı kapat: yazı/şekil öğesinin
  çizilmemesi ve sekizi aşan dash/gap listesinin kesilmesi. Önce
  desteklenmeyen görünüşün UI ve çıktı raporunda kaybolmadığını doğrula.
- GPU/CPU/PDF için ortak desen tarifi ve yürütme kuralları oluştur;
  görünüş çözümlemesini yeniden yazma. Uzun desenleri mevcut sabit
  uniform sınırına sessiz kesmek yerine ölçülmüş buffer/önbellek düzeniyle işle.
- Yazılı/şekilli desende font kaynağı, glyph/shape, açı, upright/
  teğet/absolute yönü, başlangıç fazı ve tekrar mesafesi korunabilsin.
  Bir font/shape bulunamazsa yerine boşluk çizilip başarılı teslim sayılmasın.
- Düz/yaylı/kapalı path'te faz devamı, kısa parça, köşe, path yönü,
  değişken genişlik ve aynalı/nonuniform blok örneklerini kapsa.
  Model uzunluğu ile kâğıt lineweight ve viewport desen ölçeği ayrı kalsın.
- Çizim sırası, maskeler, saydamlık ve saç teli çizgilerinde model,
  viewport ve PDF eşdeğerliği aransın. Ekran temasını değiştirmek
  çizimin atanmış veya basılacak rengini değiştirmesin.

**Kabul:** on ikiden fazla dash/gap öğesi, yazı ve şekil içeren desenleri
doğru/yay/path/blok üzerinde kullan. 1:1/1:20/1:100 çıktılarda faz, sembol
yönü ve kâğıt boyu karşılaştırılsın. Kaynak tanım round-trip'te korunsun;
desteklenmeyen kaynak için açık bulgu üretilsin.

### CAD-ADV-12 — Katman, stil, şablon ve araç paletini sürümlü ofis standardına bağla [ ]

**Öncelik:** P1. **Kod:** [C20], [C05], [C25].
**Dayanak:** AutoCAD adlandırılmış kaynak denetimi ve proje araç
paletlerinin kapsamı. [R08], [R06]

- Mevcut katman durumları, filtreler, çevirici, ByLayer/ByBlock,
  stil kitaplığı ve standart check/apply kullanılsın. İleri kapsam
  bunları ayrı dosyalar yerine sürümlü, açıklamalı ofis/proje paketiyle koordine etmektir.
- Paket; katman/adlandırma, yazı/ölçü/kılavuz/tablo stili, çizgi
  tipi, tarama, blok ve pafta şablonu referanslarını taşısın. Kaynak
  kökeni ve sürümü görünür olsun; kullanıcı tercihi belge standardıyla karışmasın.
- Özellik paneli bir değerin nereden geldiğini açıklasın: nesne,
  katman, blok veya viewport override. “Neden bu renkte/boyda basılıyor?”
  sorusu tek yerden çözülebilsin; override sıfırlama kapsamı önizlensin.
- Standart denetimi yalnız adları değil desteklenen stil/bağlantı
  ve içerik kurallarını da tarasın. Geçerli istisna gerekçeyle
  saklanabilsin; eski projeyi açmak standardı uygulama izni sayılmasın.
- Parametreleri hazır bir aracı veya mevcut nesneyi KentOS araç
  paletine kaydetme, açıklama/önizleme, arama ve favori akışı tasarla.
  Yeni yüzlerce şerit ikonu yerine mevcut komut kaydına bağlı araç tarifleri kullanılsın.
- Paket güncellemesi hangi kaynakları yeniden adlandıracak,
  birleştirecek veya değiştirecek gösterilsin. Aynı adlı fakat farklı
  içerikli kaynakta kullanıcı eşlemesi korunsun; kullanılan tanım purge edilmesin.

**Kabul:** iki ofis profili arasında katman/stil eşle; aynı adlı
farklı blok ve çizgi tipini dene. Önizleme ile sonuç aynı olsun, nesne
override'ı ve iki viewportun farklı stilleri korunabilsin. Yeni/yeniden
açılmış projelerde aynı profil tarifi aynı üretim davranışını versin.

### CAD-ADV-13 — Çizim verisinden güvenilir tablo ve metraj üret [ ]

**Öncelik:** P1. **Kod:** [C21], [C16], [C11].
**Dayanak:** AutoCAD veri çıkarımı; çizim/kaynak seçimi, dış tabloyla
eşleme ve güncelliğini yitirmiş sonuç ayrımı. [R07]

- Mevcut CAD tabloları, formüller, field ve mimari çizelgeleri koru.
  Genel veri çıkarımı tarifi; çizim/kapsam, tür, alan, hesaplanan
  kolon, grup/sıralama ve toplama kurallarını yeniden kullanılabilir biçimde saklasın.
- İç içe blok, dinamik varyant, dizi, görünmez parça ve xref için
  sayım kapsamı açık olsun. Aynı nesneyi hem ana çizimde hem referans
  içinde sayarak toplamı ikiye katlama; kaynak/örnek kimliği raporda izlenebilsin.
- Görsel metin ile sayı/birim ve kaynak değerini ayır. Bir ölçünün
  override yazısı gerçek uzunluk gibi toplanmasın. Alan/uzunluk,
  adet ve kullanıcı katsayısı farklı anlamlar taşısın; toplam yuvarlamadan önce hesaplansın.
- Kaynak güncelliği ve kullanıcı hücre düzeltmesi ayrışsın.
  Yenilemede elle girilmiş açıklama/maliyet kaybolmasın; dış CSV/tablo
  eşlemesi satır numarası yerine tanımlı anahtar ve uyuşmazlık sonucu kullansın.
- Çok satırlı içerik, sığmayan hücre, başlık tekrarı ve paftalar
  arası tablo bölme geliştirilsin. Kalıcı tek satırlı hücre modelinin
  genişlemesi açık şema göçüyle yapılsın; yalnız ekranda farklı yorumlanmasın.
- Çıktı aynı veri modelinden CAD tablosu ve uygun harici dosya olarak
  üretilebilsin. Kaynak çizimleri okumak onları sessiz kaydetme veya
  güncelleme yetkisi oluşturmasın.

**Kabul:** iç içe bloklar ve diziler içeren iki çizimden parça/simge
listesi çıkar; gerçek ölçü ve override örneği ekle. Dış açıklama tablosunu
anahtarla eşle, kaynağı değiştirip yenile. Tekil sayım, birimler, toplam,
kullanıcı düzeltmeleri ve çok sayfalı çıktı bağımsız kontrol edilsin.

### CAD-ADV-14 — Dış referans ve altlık bağımlılıklarını kaybolmadan yönet [ ]

**Öncelik:** P1. **Kod:** [C22], [C23], [C03].
**Dayanak:** AutoCAD 2027 Connected References; kaynak bütünlüğü
çizim işinin parçası olarak ele alınıyor. [R01]

- Mevcut attach/overlay/reload/unload/repath/bind, kırpma, PDF/resim
  altlığı ve paketleme korunur. İleri iş; kaynak ağını, kullanılan
  revizyonu, etkileri ve çözüm yolunu tek görünümde incelemektir.
- Taşınan/yeniden adlandırılan kaynağı içerik özeti ve izinli arama
  yerleriyle aday olarak bul. Aynı ada güvenerek farklı dosyayı
  bağlama; aday eşleme incelemesi ve paket içi göreli yol çözümü olsun.
- Yenilemeden önce kaynak farkı ve bağlı ölçü/tarama/field etkisi
  önizlensin. Kaynakta nesne silindiğinde yerel alt nesne bağı
  rastgele aynı index'e yeniden bağlanmasın; mevcut kimlik eşlemesi kullanılsın.
- Kaynağı ayrı belgede açma ile bu çizimdeki yerleştirmeyi düzenleme
  açık ayrılsın. Bir dosyanın salt okunur referansını seçmek kaynak
  dosyaya yazma yetkisi vermesin. Yerel override'ların tutulması veya sıfırlanması kapsamlı olsun.
- PDF altlığında snap ile doğrulanmış CAD ölçüsünün farkı
  açıklansın. Raster/PDF görünüşünde soldurma, kırpma, maske, seçim ve
  plot davranışı tutarlı olsun; kaynak dosyası yokken önbellekteki içerik durumu bilinsin.
- Taşınabilir paket eksik kaynakları, kullanılan font/stil/deseni
  ve aktarım kayıplarını raporlasın. Paket üzerinde yeniden bağlama,
  çevrimdışı açılış ve üçüncü kişiye teslim edilebilirlik sınansın.

**Kabul:** iç içe üç referans, aynı adlı farklı iki dosya ve bir PDF
altlığı kur. Dosyayı taşı/değiştir, sonra yeniden bağla/yenile; doğru
kaynak bulunmalı, eski bağların durumu açıklanmalı. Paketi başka dizinde
aç, kırpmayı ve iki paftadaki override'ları doğrula; tek Undo geçerli modeli geri getirsin.

### CAD-ADV-15 — Revizyon farkını semantik karşılaştırma ve 2B inceleme notlarına bağla [ ]

**Öncelik:** P1. **Kod:** [C24], [C22], [C25].
**Dayanak:** AutoCAD Trace yorumları ana çizimden ayırır; DWG Compare
belgesi bütün türlerin aynı düzeyde desteklenmediğini açıklar. [R09], [R10]

- Mevcut UUID temelli karşılaştırma korunur. Geometri/öznitelik
  yanında görünüş, blok tanımı/parametresi, bağlı kaynak, ölçü
  override'ı, tarama bağı ve pafta/görünüş değişiklikleri ayrı kategorilerle kapsansın.
- Aynı kimliği taşımayan dış dosyalarda eşleme açık bir aday
  süreci olsun: kaynak ID/handle varsa kullan, yoksa geometri ve
  özellik benzerliğiyle öner. Yakın çizgileri kesin aynı nesne sayma;
  belirsiz eşleme raporlanabilsin.
- Karşılaştırma kapsamı, tolerans ve birim bilinsin. Sadece renk
  değişimi gerçek geometri farkından ayrışsın; blok tanımı değişip
  yerleştirme ID'si aynı kaldığında görüntü farkı görünmez kalmasın.
- Revizyon bulutu/notu ve inceleme kaydı nesne/alt nesne,
  görünüş ve kaynak revizyona bağlanabilsin. Açık/yanıtlandı/çözüldü
  durumu, açıklama ve değişen kaynak nedeniyle eskime görülsün.
- 2B işaretleme katmanı ana tasarımdan ayrı kalabilsin; PDF
  notları veya AI yorumları otomatik tasarım değişikliği sayılmasın.
  Yorumdan komuta geçiş gerçek önizleme ve değişiklik kaydıyla yapılsın.
- Seçilmiş farkları alma, aynı nesnenin iki tarafta değişmesi ve
  yetkisiz kapsam için mevcut revizyon/çatışma kuralları kullanılsın.
  Eski bir not yeni geometriye bağlanırken belirsizlik saklanmasın.

**Kabul:** yalnız blok tanımı, yalnız stil, yalnız ölçü override'ı,
yalnız viewport ve gerçek geometri değişmiş örnekleri karşılaştır.
Kimliği farklı dış sürümde belirsiz eşlemeyi göster. Nota bağlı kaynak
değişsin; inceleme durumu ve seçilmiş farkı alma/Undo doğru çalışsın.

### CAD-ADV-16 — Genel 2B pafta ve yayın işini tek güvenilir üretim akışına bağla [ ]

**Öncelik:** P1; yanlış ölçek/çıktı P0. **Kod:** [C25], [C26], [C03].
**Dayanak:** pafta seti ve baskı önizlemesinin ölçekle üretim ilişkisi.
[R17], [R23]

- Mevcut pafta kitabı, analitik paper-space, viewport, override,
  plot profili, atlas, şablon ve Gezgin korunur. Mimari kat/evre
  tarifleri üzerine paralel bir genel görünüş sistemi yazılmasın.
- Genel 2B detay/uygulama paftalarında görünüş kaynağı, kapsam,
  kâğıt oranı, katman/stil durumu ve anotasyon kapsamı tekrar kurulabilir
  olsun. Genel çizim görünüşü mimari kat verisi gerektirmesin.
- Viewport kilidi, kırpma ve model/kâğıt düzenleme bağlamı açık
  olsun. Ekranda zoom yapmak yanlışlıkla baskı ölçeğini değiştirmesin;
  paftadan kaynak geometriyi düzenleme varsa görünüş kapsamı tutarlı uygulansın.
- Kaynak revizyonu ile son yayın revizyonu ayrı tutulsun. Değişen
  nesnenin hangi paftaları/tabloları etkilediği mevcut bağlardan bulunsun;
  yalnız etkilenen çıktının yeniden üretimi doğrulanabilir olsun.
- PDF vektör çizgi, metin aranabilirliği, font, hairline, maske,
  saydamlık ve tarama görünüşünü birlikte denetlesin. Baskı profili
  aynı projede başka kullanıcının temasından etkilenmesin.
- Toplu yayın için ön denetim, kapsam özeti, iptal, başarısız
  sayfadan devam ve çıktı manifesti mevcut job/pafta akışına bağlansın.
  Dosyanın oluşması bütün sayfaların doğru olduğu anlamına gelmesin.

**Kabul:** aynı modelin 1:1/1:5/1:20 ve 1:100 detaylarını bir sette
üret. İki viewport farklı katman/ölçü temsili kullansın; birini değiştir,
öbürü korunmalı. Bağımsız PDF okuyucuda ölçü/vektör/metin kontrolü yap;
güncelliğini yitiren pafta ve durdurulan yayın açık raporlansın.

### CAD-ADV-17 — Dosya alışverişinde görüntü, geometri ve düzenlenebilirliği ayrı güvenceye al [ ]

**Öncelik:** P0. **Kod:** [C27], [C01], [C11], [C22].
**Dayanak:** rakiplerde de özelleşmiş nesne ve ilişki taşımada kapsam
sınırları var; doğru görünen aktarım tam davranış uyumluluğu değildir. [R10]

- DWG/DXF doğrudan açma ve ortak içe alma yolunu koru; yeni
  aktarım motoru açma. Dosyanın birim/OCS/blok yerleştirme kararı,
  katman/stil ve kâğıt uzayı aynı rapora bağlansın.
- Destek matrisi nesne ve özellik düzeyinde olsun: analitik eğri,
  genişlik, afin/nonuniform dönüşüm, tarama sınırı, metin/font,
  dinamik blok, ölçü/field ve pafta ilişkisi. “DWG destekleniyor” tek başına kapsam anlatmasın.
- 2B nonuniform dönüşümde daire→elips ve genişlikli path gibi
  gerçek model ihtiyaçları `CAD-IO-15` ile tek iş listesinde çözülsün.
  Eğik 3B kaynak desteği bu planın tamamlanma koşuluna dönüştürülmesin;
  korunamayan veri mevcut güvenli ret/raporlu kabul yolunu izlesin.
- Z/M veya başka bilgi taşıyan dış çizimi açmak o değerleri
  otomatik sıfırlamasın. Gerçekten gereken “2B düzleme indir”
  dönüşümü açık kaynak/düzlem/kayıp önizlemesiyle ayrı, geri alınabilir işlem olsun.
- OpenCADStudio üzerinden kullanılan DWG okuma yolunda sürüm
  güncellemesi sabit kaynak revizyonu ve gerçek dosya korpusu ile
  doğrulansın. Okuyucu desteği tam DWG yazıcısı diye sunulmasın.
- Bağımsız okuyucu/yazıcı ve sayısal başvuruyla test et;
  görüntü korunumu ile analitik/düzenlenebilir ilişki korunumu ayrı
  sonuçlansın. NCZ için mevcut `NCZ-01` lisans/dağıtım kapısı korunur.

**Kabul:** birimli iç içe blok, yaylı/genişlikli path, elips/NURBS,
ilişkili ölçü, karmaşık desen ve iki pafta içeren 2B dosya kümesi kullan.
Aç–düzenle–kaydet–yeniden oku akışında her kayıp raporlansın; kaynak dosya
habersiz ezilmesin. Atlanan gerçek AutoCAD testi yapılmış gibi yazılmasın.

### CAD-ADV-18 — Çizim sağlığını kontrollü onarım ve kanıtlı kurtarmayla tamamla [ ]

**Öncelik:** P0 bütünlük/kurtarma; P1 ileri temizlik. **Kod:** [C28]–[C30].
**Dayanak:** BricsCAD Drawing Health, Optimize ve zaman sınırlı arka
plan teşhisi. [R14], [R15], [R16]

- Mevcut cleanup, audit/purge ve kurtarma kopyasını koru.
  Kullanıcıya teşhis→incele→onar→yeniden denetle zinciri sun; farklı
  araçlar için yeni, çelişen tolerans/kimlik politikaları açılmasın.
- Küçük boşluk, T birleşiminde taşma/kısa kalma, istenmeyen açı
  sapması, uzak gereksiz nesne ve yinelenen blok tanımı için
  aday denetim ekle. Ölçülmüş veya kısıtlı geometriyi otomatik düzelten genel rutin olmasın.
- Onarım tarifi sıra, eşik, kapsam, göz ardı edilen özellik ve
  hangi ilişkilerin korunacağını kaydetsin. Maksimum yer değişimi,
  alan/uzunluk farkı ve silinen/yeniden eşlenen kaynaklar raporlansın.
- Dosya açılışındaki isteğe bağlı sağlık kontrolü önce salt
  okunur olsun; süre/bellek bütçesi ve iptal taşısın. Önceki aynı
  kaynak revizyonunun sonucunu tekrar kullanabilsin; açılışı zorunlu tam taramayla yavaşlatmasın.
- Kurtarma; model, pafta, kaynak ilişkileri, yeni dosya sürümü ve
  aktif belge için gerçek çökme/disk dolu/kesilen kayıt senaryolarıyla
  doğrulansın. Onarılmış sonuç ayrı kurtarma kopyası olsun; sağlam kaynak ezilmesin.
- Kısmi okunabilen dosya için veri çıkarma yolu geliştirilecekse
  normal okuyucunun katı doğrulamasından ayrı olsun; kurtarılan ve
  kaybolan kayıtlar açık raporlansın. Bozuk ilişkiyi görünmez silmek başarı sayılmasın.

**Kabul:** bilerek sorunlu ama sınırları bilinen çizimde önce sıfır
yazımlı tarama, sonra seçilmiş onarım yap. Gerçek test sürecini kayıt
sırasında sonlandırıp kurtar; bütün kaynak/pafta bağlarını karşılaştır.
Disk dolması ve iptalde önceki geçerli dosya ve Undo zinciri korunmalı.

### CAD-ADV-19 — Büyük 2B çizimde akıcılığı ve görsel kararlılığı ölçerek koru [ ]

**Öncelik:** P0; diğer maddeler boyunca uygulanır. **Kod:** [C05], [C13], [C31].
**Dayanak:** QCAD kaynak kodundaki blok başına uzamsal dizin ve işlem
sonucuna bağlı güncelleme örneği; KentOS için yöntem aşağıdaki öneridir. [R21]

- Mevcut wgpu, parça önbelleği, uzamsal dizin, hi/lo ve eğri LOD
  yaklaşımını kullan. Tek küçük düzenleme bütün çizimi, bütün metni
  veya bütün blok tanımlarını yeniden kurmasın; etki kapsamı ölçülsün.
- Pan/zoom sırasında kamera ve LOD geçişi kararlı olsun. Kaynak
  eğri aynı kalırken çizgi, tarama, yazı ve ölçü sıçramasın;
  sadeleştirme seçimi/snap'i veya hesap sonucunu değiştirmesin.
- Yoğun blok/dizi/linetype/hatch için batching, ortak geometri ve
  görünür kapsam önceliği değerlendirilsin. Her yerleştirmeyi her
  karede açmak yerine tanım/parametre varyantı önbellekleri bütçeyle yönetilsin.
- Ekran önizlemesi, tam kaynak sorgusu ve vektör PDF ayrı
  performans/kalite yolları olsun. GPU FP32 hızlandırma kaynak
  geometrinin hassasiyetini düşürmesin; kâğıt çıktısına ekran LOD'u kopyalanmasın.
- 10.000/100.000/1.000.000 karma nesne, çok yazı, uzun çizgi,
  iç içe blok ve yoğun tarama için sabit veri setleri kullan.
  Açılış, ilk etkileşim, p95/p99 kare, yazı girişi, komut süresi,
  RAM/VRAM ve iptal gecikmesini ayrı raporla; veri sayısı tek başına hız vaadi olmasın.
- İlk hedefler uygulama öncesi referans cihazla sabitlensin:
  hafif etkileşimde 60 Hz kare bütçesi, özellikte yerel tuş
  yankısında p95 ≤50 ms ve iptal isteğinin UI'da ≤100 ms içinde
  görünmesi başlangıç ürün hedefidir. Tam işin durma süresi ayrıca ölçülür;
  bu sayılar elde edilmiş sonuçlar değildir.

**Kabul:** aynı sabit kamera/fare/klavye iziyle önce/sonra ölçüm yap;
yeni özellik açık/kapalı karşılaştırılsın. Yakın ve uzak görünümde
milimetrik ayrıntı, yazı, ölçü ve desen yerleşimini gerçek GPU'da görüntüle.
UI iş parçacığı süresiyle toplam GPU/kare süresi ayrı raporlansın.

### CAD-ADV-20 — Her 2B yeteneği Python, MCP, AI ve eklenti üzerinden aynı doğrulukla sun [ ]

**Öncelik:** P0; sona bırakılacak ayrı ek iş değildir. **Kod:** [C02], [C03], [C32].
**Dayanak:** AutoCAD 2027'de bağlamsal AI/MCP ve proje kaynaklarına
erişim ürün yönü olarak görülüyor; KentOS sözleşmesi aşağıda tanımlıdır. [R01]

- Her yeni araç/araç seçeneği için UI eylemi→komut/version→girdi
  şeması→hesap işleyicisi→sonuç eşlemesi tamam olsun. Kaynak geometriyi
  script'in hesaplayıp genel create/edit'e göndermesi semantik API'nin yerine geçmesin.
- `validate` ve `plan` hiçbir kalıcı durum yazmasın. `execute`,
  beklenen revizyon ve erişim denetimiyle bütün mantıksal değişikliği
  uygulasın. Seçim/viewport/UCS gibi oturum durumlarının kapsamı açık olsun.
- Girdi birimleri, kaynak/alt nesne referansları, yaklaşık sonuç,
  kayıp/uyarı, etkilenen nesneler, ilişkiler, pafta ve Undo bilgisi
  yapılandırılmış sonuçta bulunsun. AI kullanıcıya başarıyı gerçek sonuçtan anlatsın.
- Python tipleri, docstring, örnekler, birim açıklaması ve ret
  kodları komut kataloğundan güncel üretilsin. Aynı senaryo başsız
  belge ve açık masaüstü oturumunda doğru belge/revizyonla çalışsın.
- Ağır tanıma/karşılaştırma/temizlik/çıktı işi mevcut job/Processing
  yoluna bağlansın. Gerçek iptal, ilerleme, kaynak kotası, geç cevap
  reddi ve atomik sonuç şart olsun; UI kilitleyen Python callback'i araç teslimi sayılmasın.
- Eklenti katkısı `.kek`, KentOS UI bileşenleri ve isteğe bağlı
  uv/Python ortamı sözleşmesini kullansın. Ana giriş Eklentiler'de
  kalsın; komut/panel/Processing katkıları aynı kayıt üzerinden erişilsin.
- AI sağlayıcısı kullanıcı seçimiyle bağlansın. İşin kapsamını ve
  değişiklik planını keşfedebilsin; başka belgenin, kilitli nesnenin
  veya erişilemeyen verinin kuralını atlamasın. Yeni araçlar için
  AI ile eklenti üretmeye yetecek gerçek örnek ve keşif metadatası yayımlansın.

**Kabul:** aynı genel detayı GUI, Python ve MCP üzerinden oluştur,
ilişkili ölçü/tarama ekle, parametre değiştir ve pafta çıkar. Kaynak
geometri, bağlar, retler ve kalıcı sonuç eşdeğer olsun. İptal, tekrar
gönderim, yanlış belge/revizyon ve script çökmesinde yarım iş kalmasın.

## Teslim sırası ve ortak kabul

Ana görev sayısı 20'dir. Aşağıdaki aşamalar bu maddelerin uygulanma
sırasıdır; yeni görev numarası veya mevcut planın kapanış işareti değildir.

| Aşama | Odak | Çıkış ölçütü |
| --- | --- | --- |
| Temeli koru | CAD-ADV-01/02/03; 09/17/18'in veri doğruluğu; 19/20 bütün süreçte | Tek onay/iptal, birim, seçim/özellik, ölçü, kayıt ve kaynak doğruluğu güncel masaüstünde kabul edilmiş. |
| Gerçek kapsam açıkları | CAD-ADV-04/05/08/10/11 | Eğrisel düzenleme ve kısıt kapsamı açık; tarama bağı, metin ve karmaşık çizgi tipi uçtan uca doğru. |
| Tekrar kullanım ve ofis üretimi | CAD-ADV-06/07/12/13/14 | Blok/dizi, standart, tablo ve dış kaynak ilişkileri birlikte korunuyor. |
| Koordineli teslim | CAD-ADV-15/16/17; 18'in ileri onarımı | Kaynak revizyonu, fark/inceleme, pafta/PDF ve aktarım kaybı aynı üretim kaydından izleniyor. |

Her madde ancak şu koşullarla tamamlanmış işaretlenir:

- **Başlangıç kanıtı:** güncel commit'ten üretilmiş uygulamada ilgili
  mevcut davranış kontrol edilir. Bu rapordaki kod okuması veya eski
  bir test derlemesinin ekranı güncel GUI kabulü sayılmaz.
- **Bağımsız geometri:** beklenen sonuç aynı fonksiyondan yeniden
  üretilmez. Analitik örnek, uygun bağımsız CAD/geometri okuyucusu ve
  hata bütçesi kullanılır. Kesin sonuçla yaklaşık/görsel sonuç ayrı değerlendirilir.
- **Etkileşim:** fare, klavye, ilk karakter, negatif/virgüllü giriş,
  Enter/Tab/Esc, sağ tuş, art arda zoom/pan, dar/geniş pencere ve
  desteklenen ekran ölçekleri denenir. Önizleme–sonuç ve tek Undo kanıtlanır.
- **Kalıcılık:** validate/plan/ret/iptal çizimi, kirli durumu veya
  Undo/Redo'yu değiştirmez. Kaydet–aç ve uygun bulut/taşınabilir
  aktarım kaynak parametreleri, kimlikleri ve ilişkileri korur.
- **Otomasyon:** komut seçeneği UI/Python/MCP'de aynı anlamı taşır.
  Uygun ağır araç Processing/job olarak erişilir; doküman ve örnek
  aynı teslimde tamamlanır. CAD-ADV-20 sonradan topluca kapatılmaz.
- **Regresyon:** ortak motor değişikliği Mimari ve Harita'nın ilgili
  örneklerinde sınanır; GIS tarafından kullanılan yol varsa tipli
  katman ve erişim kuralları da denetlenir. Diğer ajanın yeni görünüş/
  Gezgin işinin kapsamı uygulama başlamadan yeniden okunur.
- **Görüntü ve kanıt:** gerçek pencerenin önce/sonra ekran görüntüleri
  ve işlem adımları kaydedilir; görüntüler **repo dışında** tutulur.
  Bir screenshot tek başına hesap veya etkileşim kanıtı değildir.
- **Performans:** veri, cihaz, CPU/GPU yolu, ilk/ısınmış çalışma ve
  bellek tepe değeriyle raporlanır. Başlangıç hedefi tutmazsa madde
  tamamlanmış denmez; teknik sınır açıkça kaydedilir.

## İncelenen 25 internet kaynağı

İnceleme tarihi **31 Ekim 2026**. Arama sonucu tek başına, boş uygulama
kabuğu, açılamayan bağlantı ve tekrar açılan sayfa bu sayıya katılmadı.
AutoCAD/BricsCAD'in 3B bölümleri KentOS hedefi olarak alınmadı.

| No | Kaynak | Kullanılan bilgi / sınır |
| --- | --- | --- |
| R01 | [AutoCAD 2027 — Product details][R01] | Geometri temizliği, bağlı referans/destek kaynakları ve AI/MCP ürün yönü; üretici beyanı, kapalı motor veya KentOS performansı kanıtı değil. |
| R02 | [AutoCAD 2027 — Smart Blocks][R02] | Yerleştirme/yerine koyma/tanıma/dönüştürme ailesi; LT kapsamı farklı. |
| R03 | [AutoCAD 2027 — Parametric Drawing and Constraints][R03] | Geometrik/ölçüsel kısıt, kısmi/tam kısıt ve düzenleme semantiği. |
| R04 | [AutoCAD 2027 — Associative Dimensions][R04] | Tam/kısmi/kopuk bağ ve anotasyon izleme; destek türlerinin sınırları. |
| R05 | [AutoCAD LT Mac 2027 — Text in Custom Linetypes][R05] | Font/stil, yazı yönü, ölçek ve offset. Bu kaynak bütün platform/sürümlerin aynı davranışta olduğu iddiası değil. |
| R06 | [AutoCAD 2027 — Creating Tool Palettes][R06] | Araç tarifleri, içerik grupları ve proje kaynaklarının kapsamı. |
| R07 | [AutoCAD 2027 — Data Extraction][R07] | Yeniden kullanılabilir çıkarım tarifi, çizim/tablo eşlemesi ve güncellik. |
| R08 | [AutoCAD 2027 — CAD Standards][R08] | Adlandırılmış kaynak ve özellik denetimi; katman denetiminin kapsamadığı bayraklar da belirtiliyor. |
| R09 | [AutoCAD 2027 — Trace][R09] | Yorum/işaretleme ile tasarımın ayrılması. Sayfa platform metni ve tablosunda çelişki barındırıyor; platform bazlı destek matrisi buradan türetilmedi. |
| R10 | [AutoCAD 2027 — DWG Compare Limitations][R10] | Karşılaştırma/aktarımı desteklenmeyen türler ve özel ilişkilerin kayıp ihtimali. |
| R11 | [BricsCAD — BLOCKIFY][R11] | Eşleşme kapsamı, parametreyi koruma ve adayları inceleyerek dönüştürme; belgedeki 3B modlar kapsam dışı. |
| R12 | [BricsCAD V27 — ARRAYDETECT][R12] | Tekrarlı örüntüyü ilişkili diziye dönüştürmeden önce inceleme. |
| R13 | [BricsCAD — Parameters Manager][R13] | İfade/değer, bağımlı/tanımlayan parametre, tasarım tablosu ve bağlam eylemleri; yalnız 2B'ye uygulanabilir kısmı alındı. |
| R14 | [BricsCAD V27 — OPTIMIZE][R14] | 2B boşluk, açı ve T birleşimi düzeltmesi; eşik ve önizleme. Otomatik koordinat yuvarlama KentOS varsayılanı olarak alınmadı. |
| R15 | [BricsCAD V27 — DWGHEALTH][R15] | Çoklu sağlık/temizlik işini rutin, etkileşim ve raporla birleştirme. |
| R16 | [BricsCAD V27 — Health Advisor Settings][R16] | Arka plan teşhisi için zaman sınırı, sıra ve aynı dosyayı gereksiz tekrar taramama. |
| R17 | [DraftSight — 2D CAD Top 15 Features, Mayıs 2026][R17] | Hızlı budama ve ilişkili üretim örnekleri. PDF içindeki BIM/STEP/3B projeksiyon özellikleri hedefe alınmadı. |
| R18 | [QCAD — Property Editor][R18] | Karma seçimde tür süzgeci ve toplu özellik düzenleme. |
| R19 | [QCAD — Selection Filter][R19] | Sorguyu ekle/çıkar/kesiştir; atanmış ve görünen renk ayrımı. Sayfa Professional eklentisi olarak işaretli. |
| R20 | [QCAD — Trim.js kaynak kodu][R20] | Durumlar, Esc/geri adım, destek denetimi, preview/getOperation ve applyOperation yolları gerçekten okundu. `master` değişebilir. |
| R21 | [QCAD — RDocument.cpp kaynak kodu][R21] | `startTransactionGroup`, Undo/Redo ve `getSpatialIndexForBlock` yolları incelendi; bütün motor denetlenmiş sayılmaz. |
| R22 | [LibreCAD — Blocks][R22] | Blok tanımı, yerleştirme ve kitaplıktan kopyalama davranışlarının ayrımı. `latest` ön sürüm/WIP belgesi. |
| R23 | [LibreCAD — Completing and Printing][R23] | Gerçek boylu model, kâğıt ölçeği, kilitli baskı oranı ve önizleme. `latest` ön sürüm/WIP belgesi. |
| R24 | [Unicode UAX #29 — Text Segmentation][R24] | Görünür karakter kümesi ile codepoint/scalar sınırının farkı. Ürünün seçtiği Unicode sürümü uygulamada sabitlenecek. |
| R25 | [HarfBuzz — What is text shaping?][R25] | Unicode dizisinden glyph seçimi/yerleşimi; satır kırma ve belge düzeniyle farklı sorumluluk. Bağımlılık ekleme kararı alınmadı. |

DraftSight'ın bazı sürümlü HTML yardım sayfaları açılışta yalnız JavaScript
kabuğu verdi veya erişilemedi; bunlar okunan kaynak diye sayılmadı. İlgili
2B örnekleri üreticinin okunabilir PDF rehberinden incelendi. Açık kaynak
dallarında ve sürümsüz yardım sayfalarında uygulama aşamasında seçilen
commit/sürüm yeniden sabitlenecek; bu araştırma kaydı kalıcı sürüm garantisi değildir.

[R01]: https://www.autodesk.com/products/autocad/product-details
[R02]: https://help.autodesk.com/cloudhelp/2027/ENG/AutoCAD-Core/files/GUID-8D87AF53-BD0C-4E01-A015-42E92E6AA715.htm
[R03]: https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Core/files/GUID-899E008D-B422-4DF2-AC8D-1A4F5701ED4E.htm
[R04]: https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Core/files/GUID-8B7E8C96-C30D-409E-881E-7942871E80DA.htm
[R05]: https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-LT-MAC-Customization/files/GUID-FEDCE7EB-4919-43AE-A54E-F3A293DD60CA.htm
[R06]: https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Core/files/GUID-6B20D7DF-447A-4A8D-8FF1-8339CF54FFBD.htm
[R07]: https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Core/files/GUID-B0D32260-45E3-4643-B574-7F6C31579B68.htm
[R08]: https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Core/files/GUID-D64F8076-4978-44B7-B056-D921C77FEA88.htm
[R09]: https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Platform/files/GUID-5D5E0BA1-10E7-4662-8118-E046EC579B08.htm
[R10]: https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Core/files/GUID-3D918CC7-4FB7-4951-8F04-C2434DFD04A2.htm
[R11]: https://help.bricsys.com/en-us/document/command-reference/b/blockify-command
[R12]: https://help.bricsys.com/en-us/document/command-reference/a/arraydetect-command?version=V27
[R13]: https://help.bricsys.com/en-us/document/panels/parameters-manager-panel
[R14]: https://help.bricsys.com/en-us/document/command-reference/o/optimize-command?version=V27
[R15]: https://help.bricsys.com/en-us/document/command-reference/d/dwghealth-command?version=V27
[R16]: https://help.bricsys.com/en-us/document/dialog-boxes/h/health-advisor-settings-dialog-box?version=V27
[R17]: https://www.draftsight.com/sites/draftsight/files/2026-05/DraftSight_2D_CAD_Top_15_Features_eBook.pdf
[R18]: https://qcad.org/doc/qcad/latest/reference/en/tutorials/properties/property_editor/property_editor_en.html
[R19]: https://www.qcad.org/en/manual/scripts/pro/widgets/selectionfilter/doc/selectionfilter/
[R20]: https://raw.githubusercontent.com/qcad/qcad/master/scripts/Modify/Trim/Trim.js
[R21]: https://raw.githubusercontent.com/qcad/qcad/master/src/core/RDocument.cpp
[R22]: https://docs.librecad.org/en/latest/guides/blocks.html
[R23]: https://docs.librecad.org/en/latest/guides/completion.html
[R24]: https://unicode.org/reports/tr29/
[R25]: https://harfbuzz.github.io/what-is-harfbuzz.html

[C01]: https://github.com/mcihad/kentos-cad/blob/72728b04c33fc9c64962d64903ef81e6449f310e/crates/native/application/src/units_change.rs#L89
[C02]: https://github.com/mcihad/kentos-cad/blob/72728b04c33fc9c64962d64903ef81e6449f310e/crates/native/application/src/changeset.rs#L73
[C03]: https://github.com/mcihad/kentos-cad/blob/72728b04c33fc9c64962d64903ef81e6449f310e/crates/native/domain/src/followers.rs#L25
[C04]: https://github.com/mcihad/kentos-cad/blob/72728b04c33fc9c64962d64903ef81e6449f310e/crates/native/application/src/entities_select.rs#L59
[C05]: https://github.com/mcihad/kentos-cad/blob/72728b04c33fc9c64962d64903ef81e6449f310e/apps/desktop/src/properties/mod.rs#L416
[C06]: https://github.com/mcihad/kentos-cad/blob/72728b04c33fc9c64962d64903ef81e6449f310e/crates/native/application/src/trim.rs#L529
[C07]: https://github.com/mcihad/kentos-cad/blob/72728b04c33fc9c64962d64903ef81e6449f310e/crates/shared/geometry-core/src/ops/fillet.rs#L101
[C08]: https://github.com/mcihad/kentos-cad/blob/72728b04c33fc9c64962d64903ef81e6449f310e/crates/shared/geometry-core/src/ops/offset.rs#L276
[C09]: https://github.com/mcihad/kentos-cad/blob/72728b04c33fc9c64962d64903ef81e6449f310e/crates/shared/geometry-core/src/ops/constraints/model.rs#L106
[C10]: https://github.com/mcihad/kentos-cad/blob/72728b04c33fc9c64962d64903ef81e6449f310e/crates/native/application/src/constraints_query.rs#L66
[C11]: https://github.com/mcihad/kentos-cad/blob/72728b04c33fc9c64962d64903ef81e6449f310e/crates/shared/contracts/src/block_dynamic.rs#L24
[C12]: https://github.com/mcihad/kentos-cad/blob/72728b04c33fc9c64962d64903ef81e6449f310e/crates/native/application/src/blocks_sync.rs#L69
[C13]: https://github.com/mcihad/kentos-cad/blob/72728b04c33fc9c64962d64903ef81e6449f310e/crates/shared/contracts/src/assoc_array.rs#L49
[C14]: https://github.com/mcihad/kentos-cad/blob/72728b04c33fc9c64962d64903ef81e6449f310e/crates/native/domain/src/hatch_boundaries.rs#L56
[C15]: https://github.com/mcihad/kentos-cad/blob/72728b04c33fc9c64962d64903ef81e6449f310e/crates/native/domain/src/hatch_ties.rs#L61
[C16]: https://github.com/mcihad/kentos-cad/blob/72728b04c33fc9c64962d64903ef81e6449f310e/crates/native/application/src/dimensions_query.rs#L71
[C17]: https://github.com/mcihad/kentos-cad/blob/72728b04c33fc9c64962d64903ef81e6449f310e/crates/shared/geometry-core/src/text/paragraph.rs#L242
[C18]: https://github.com/mcihad/kentos-cad/blob/72728b04c33fc9c64962d64903ef81e6449f310e/crates/native/plot/src/images.rs#L197
[C19]: https://github.com/mcihad/kentos-cad/blob/72728b04c33fc9c64962d64903ef81e6449f310e/crates/native/style/src/line_types.rs#L27
[C20]: https://github.com/mcihad/kentos-cad/blob/72728b04c33fc9c64962d64903ef81e6449f310e/crates/native/application/src/standards.rs#L506
[C21]: https://github.com/mcihad/kentos-cad/blob/72728b04c33fc9c64962d64903ef81e6449f310e/crates/shared/contracts/src/table.rs#L34
[C22]: https://github.com/mcihad/kentos-cad/blob/72728b04c33fc9c64962d64903ef81e6449f310e/crates/shared/contracts/src/xref.rs#L45
[C23]: https://github.com/mcihad/kentos-cad/blob/72728b04c33fc9c64962d64903ef81e6449f310e/crates/native/application/src/assets_check.rs#L263
[C24]: https://github.com/mcihad/kentos-cad/blob/72728b04c33fc9c64962d64903ef81e6449f310e/crates/native/application/src/drawing_compare.rs#L49
[C25]: https://github.com/mcihad/kentos-cad/blob/72728b04c33fc9c64962d64903ef81e6449f310e/crates/shared/contracts/src/cad_views.rs#L39
[C26]: https://github.com/mcihad/kentos-cad/blob/72728b04c33fc9c64962d64903ef81e6449f310e/crates/shared/sheet/src/preflight.rs#L54
[C27]: https://github.com/mcihad/kentos-cad/blob/72728b04c33fc9c64962d64903ef81e6449f310e/crates/native/application/src/file_import_dwg.rs#L14
[C28]: https://github.com/mcihad/kentos-cad/blob/72728b04c33fc9c64962d64903ef81e6449f310e/crates/native/application/src/cleanup_cmd.rs#L74
[C29]: https://github.com/mcihad/kentos-cad/blob/72728b04c33fc9c64962d64903ef81e6449f310e/crates/native/application/src/audit_cmd.rs#L45
[C30]: https://github.com/mcihad/kentos-cad/blob/72728b04c33fc9c64962d64903ef81e6449f310e/apps/desktop/src/recovery.rs#L114
[C31]: https://github.com/mcihad/kentos-cad/blob/72728b04c33fc9c64962d64903ef81e6449f310e/crates/render/wgpu/src/precision.rs#L35
[C32]: https://github.com/mcihad/kentos-cad/blob/72728b04c33fc9c64962d64903ef81e6449f310e/crates/native/headless/src/dispatch.rs#L90
