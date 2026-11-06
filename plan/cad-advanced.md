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

### CAD-ADV-06 — Parametrik blokları sürdürülebilir 2B bileşenlere dönüştür [x]

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

**İlerleme — 2026-11-03, gerçek testte görülen konsol kısayolu:**
Konsol odaktayken ⌘O'nun kaynak koda `o` eklemesi giderildi. Betik
kaydet/farklı kaydet ve yerel Undo/Redo Mac Command tuşunu kabul eder;
normal/Option/AltGr metni, pano ve çizim odağı ayrımı korunur. Gerçek
Aç/İptal, kod çalıştırma ve `.py` kayıtları doğrulandı; CAD dosyası
başlangıçla bayt düzeyinde aynı. 326 UI, 1614 masaüstü testi, Clippy ve
derleme geçti. [ADR 0862](../docs/adr/0862-python-editor-command-shortcuts.md).

**İlerleme — 2026-11-03, kalıcı tip–örnek kapsamı:** Ortak değer aynı
çizim tanımının doğrudan ve iç kullanımlarına uygulanır; eski/yeni yayınlar
bağımsız kalır. Tip/örnek formülleri, lookup kapsamı, kilitler, farklı
örneklerde atomik ret ve örneğe dönüşte literal değer koruması ortak
komuttadır. Tip değerleri penceresi kapsamı/kullanım sayısını, proje
biriminde min/max/adımı ve katalog satırını gösterir; ortak panel değeri
salt okunur, ortak grip yoktur. KCAD160 ve bağımsız okuyucu/yazıcı kalıcı
alanları doğrular. Gerçek GUI'de m/mm, üç kapıda ortak değişim, katalog,
örnek kapsamına dönüş, çakışma, iptal, dar pencere, kaydet–aç ve tek Undo/Redo kabul edildi.
GUI/korumalı Python kayıtları tam aynı. 1164 contracts, 316 çekirdek,
234 KCAD, 356 application, 64 MCP, 1619 masaüstü ve 472 Python testi,
Clippy ve derleme geçti. Ortak değerli kitaplık sürüm 3 olarak yazılır;
eski sürümle etiketlenmiş kaynak iki import yolunda atomik reddedilir.
Son sürüm koruması tekrarında 53 Python blok/9 katalog testi ve gerçek
menüden dışa/içe alma, ret ve minimum pencere kaynak kaydını korudu. [ADR 0863](../docs/adr/0863-block-type-and-instance-parameter-scope.md),
[kullanım/API](../docs/cad/block-type-values.md).

**İlerleme — 2026-11-03, iç etiket taşıma kapsamı:** Öznitelik penceresi
çizim kapsamını varsayılan olarak korur; kullanıcı seçerse diğer tanımların
iç kayıtlarındaki aynı etiket de değeri ve özel konumuyla taşınır. Plan
gerçekte değişen üst tanımları gösterir. Çakışma ve etkilenen kilitli
doğrudan/üst kullanım bütün işlemi yazısız reddeder. Python/MCP aynı
`renameNested` komut girdisini kullanır; kapsamı değişen istek eski planı
kullanamaz. Eski pencerenin başka oturum veya değişmiş revizyona yazması
reddedilir. Gerçek GUI'de K-1/K-2/İÇ-1 yeni etikette korundu; 1100 mm bağlı
ölçü değişmedi. Tek Undo/Redo kaynak kayıtlarla tam aynı. Aynı çizimde
kitaplıktan yalnız ilk kapı yayın 3'e yükseltildi; ölçüsü 1200 mm ve hesap
alanı 2.4 oldu; seçilmeyen eski tip, yön/görünüş ve iç kayıt korundu.
Kaydet–aç ve gerçek statik DXF yeniden açma da sınandı.
Son pencere düzeni en küçük boyutta sınandı; yazarken tablo yerinde kaldı,
boş etikette Kaydet pasifti ve klavyeyle kapsam seçildi. Vazgeç dosyayı
korudu. 364 application, 1622 masaüstü, 65 MCP ve tüm 475 Python testi,
SDK/katalog kontrolü, Clippy ve derleme geçti.
[ADR 0864](../docs/adr/0864-explicit-nested-attribute-tag-migration.md),
[kullanım/API](../docs/cad/block-attribute-scope.md).

**Kapanış — 2026-11-03:** Birleşik kabulde yukarıdaki iki görünüş,
lookup/formül, ortak tip–örnek, iç etiket taşıma, seçilmiş yayın yükseltme,
1100→1200 mm bağlı ölçü, gerçek Undo/Redo ve kaydet–aç doğrulandı.
Son açık olan yazım öncesi kayıp uyarısı da tamamlandı: DXF/DWG penceresi
dinamik davranışın statikleşmesini ve bağlı ölçünün geometriyi izleme
ilişkisinin taşınmamasını dosya yolu seçilmeden önce bildirir. Mevcut boyut
ve görünüşün yazılması, parametrik/ilişkisel düzenlenebilirlik sayılmaz.
KentOS metadata seçeneği artık bütün alanları koruduğunu söylemez.
Aynı `Ölçü bağı` uyarısı native/blok × metadata açık/kapalı ortak yazıcıda,
Python ve MCP plan/yürütme raporunda bulunur; plan dosya yaratmaz.

Gerçek fareyle büyük/en küçük pencere, gövde kaydırma, sabit eylemler,
yalnız iç içe üst blok seçimi, Tümü kapsamına geçiş, DXF/DWG uyarıları,
Vazgeç, gerçek DXF yazımı ve yeniden açma sınandı. GUI/Python DXF baytları
aynı; aktarım/kapsam/Vazgeç sonrasında kaynak KCAD başlangıçla tam aynı.
Son DXF tekrar açıldığında K-1/K-2/İÇ-1 ve 1100 mm güncel ölçü korundu;
bu statik kaydın bağsızlığı API ve bağımsız KCAD okuyucusuyla da doğrulandı.
51 DXF yazıcı, 1624 masaüstü, 66 MCP, 12 başsız DXF ve 56 Python blok testi,
Clippy ve son derleme geçti. Görüntüler repo dışındadır.
[ADR 0865](../docs/adr/0865-static-cad-export-behavior-loss-disclosure.md),
[kullanım/API](../docs/cad/block-static-dxf.md).

Bu işaret native masaüstü/Python/MCP kapsamının kabulüdür. Web öznitelik
penceresinin native eşitleme kapsamıyla aynı olmadığı kodda görüldü;
TODOS'daki web eşitliği görevi bu kapanışla tamamlanmış sayılmaz.

### CAD-ADV-07 — Tekrarları tanı, blok ve ilişkili diziye kontrollü dönüştür [x]

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

**İlerleme — 3 Kasım 2026, ilk dilim:**
[ADR 0866](../docs/adr/0866-repeated-2d-candidate-recognition.md),
[kullanım/API](../docs/cad/repeated-components.md).

- [x] `cad.blocks.find` ile analitik çizgi/çember/yay koleksiyonunu yazısız
  tanıma. Kaynakta başka tür varsa bütün kaynak açık ret alır; desteklenmeyen
  parça atılarak sahte bir tam eşleşme üretilmez. İlk dilim bütün CAD türlerini
  destekliyor diye sunulmaz.
- [x] Döndürme, ayna, tekdüze ölçek ve katman/görünüş/öznitelik politikası
  ayrı seçeneklerdir. Geometriye göre karşılaştırma ayrı uyarı verir.
  Float64 hesapta kaynak yerel çerçeveye alınır; tolerans mutlak metre,
  varsayılan 0,001 mm'dir. Proje birimi/ölçeği veya geometri değiştirilmez.
- [x] Uzamsal indeks, bütün üyelerde bire bir analitik doğrulama ve kaynak
  sırasındaki UUID eşlemesi. Çakışan adaylar görünürdür; yinelenen geometriler
  tek kimliğe çökmez. Kaynak 128, kapsam 50 000, sonuç 1 000 ile sınırlıdır;
  iş/sonuç sınırı kısmi tarama olarak açıklanır.
- [x] Arka planda arama, Durdur ve panelden çıkışta iptal; geç gelen iş ve
  değişen belgeye ait sonuçla seçim yapılmaz. Masaüstünde görünür birimli
  ayarlar, Bloklar panelinde sanallaştırılmış aday listesi, Göster ve açık
  aday seçimi; Python/MCP aynı native sorguyu kullanır. Python kodu çıktısı
  ayarı sorguyu da kapsar. Python iş API'sinin başarılı çıktıyı düşüren
  okuyucusu katalogdaki sonuç tipiyle düzeltildi.
- [x] Tek parçalı 2B çoklu çizgi: açık/kapalı durum, analitik yaylı kenar ve
  değişken uç genişlikleri; döngüsel başlangıç ve ters yön eşlemesi.
  [ADR 0867](../docs/adr/0867-analytic-polyline-repeat-recognition.md).
  Z/M verisi, mahal ayırıcı rolü, delik ve ek parçalar açık ret alır.
  Köşe yapısı sadeleştirilmez; bir çizgi 4 096, toplam kapsam 200 000 köşedir.
- [x] 2B çoklu çizgiye ek olarak tek satırlı yazı/öznitelik varyantları ve
  statik iç bloklar aynı doğruluk kapısından geçti (ADR 0881). Değer farkları
  ayrı açık seçeneklerdir; metin→öznitelik dönüşümü her değeri ve gerçek
  yerleştirilmiş biçimi doğrular. İç ölçü yolları ve aynı tanımlı değerler
  korunur. NURBS/tarama/dinamik blok gibi kapsam dışı kaynak veya taşınamayan
  veri/ilişki atlanarak sahte tam eşleşme ya da kayıplı dönüşüm yapılmaz.
- [x] İlk analitik kapsamda seçili adayları bloklaştırma; eski UUID→yeni
  yerleştirme/alt nesne eşlemesi, sıradan ölçü bağlarını koruma ve desteklenmeyen
  veri/görünüş/ilişkide gerekçeli atomik ret. Gerçek GUI/Python/MCP ve tek
  Undo/Redo kabulü. Metin/öznitelik, iç blok ve diğer geometri/ilişki türleri
  tamamlanmış sayılmaz; kaynak genişletme maddesi açık kalır (ADR 0871).
- [x] Grid/kutupsal/yol örüntüsünü mevcut `AssocArray` modeline benimsetme;
  iki eksik öğe, elle değişen öğe, koru/sil/reset ve sayı/aralık değişikliğinde
  bağların ve kimliklerin korunması. Dönük/eğik grid dahil son kabul ADR 0882 ile tamamlandı.

**İlk dilimin doğrulaması:** çekirdek 5, native uygulama 366, masaüstü 1 628,
MCP 67 ve Python 479 test geçti; native ortak komut kabulü 21 durumdur.
Son yerleşim değişikliğinden sonra masaüstünün 4 odak testi ve Clippy tekrar
geçti. Gerçek uygulamada varsayılan 20, ayna açık 40, ölçek de açık 41 aday;
40 adaydan 160 nesnenin seçimi ve Python konsolundaki public sorgu doğrulandı.
Küçük pencere/büyük yazıda kesilen aday satırı düzeltildi; açık/koyu tema,
birimli tolerans, sıfırda pasif Bul ve geometriye göre arama uyarısı denendi.
Arama/seçimden sonra gerçek arayüzden kaydedilen KCAD dosyası kaynak dosyayla
bayt bayt aynıdır; bağlı ölçü korunur. İptal/geç sonuç otomatik testle
doğrulandı; bu küçük kabul çiziminde elle Durdur testi yapılmış sayılmaz.
Ekran görüntüleri ve ayrıntılı kabul kayıtları depo dışında tutulur.

**Yoğun örtüşme kabulü:** 10 000 üst üste çember, ayrı kaynak ve farklı
yarıçaplı benzerle ankraj taraması iyileştirildi. Aynı veriyle 2 000 001
karşılaştırma/199 kısmi aday yerine 2 002 karşılaştırma/1 000 kısmi aday;
kimlikler, geometri/veri denetimi ve sonuç sınırı korunur. Çekirdeğin tüm
601 testi (6 bilinçli atlanan), 3 native sorgu testi, 21 ortak durum,
4 masaüstü testi, 59 Python blok testi ve ilgili paketlerin Clippy kontrolü
geçti. Gerçek küçük pencerede dört haneli sayaç taşması düzeltildi;
kısmi sonuç başlığı, 0/1 000 aday seçimi ve son kaydın kaynakla bayt
eşitliği doğrulandı. Kaynak türü genişletme ve dönüşüm işleri açık kalır.

**Çoklu çizgi diliminin doğrulaması:** çekirdeğin 604, native uygulamanın
369, masaüstünün 1 628, MCP'nin 68 ve Python blok yüzeyinin 61 testi geçti;
ortak komut kabulü 25 durumdur. Son arayüz değişikliğinden sonra 4 odak testi,
derleme ve Clippy yeniden geçti. Gerçek uygulamada yaylı, değişken genişlikli,
ters/döngüsel sıralı çizgilerde 20/40/41 aday; son derlemede 40 adayın seçimi,
Göster ile kaynak ve bağlı 2 000 mm ölçü doğrulandı. Z verili kaynak,
geometriye göre aramada da açık gerekçeyle reddedildi; hata metni ve düğmeler
büyük yazıda okunur. 50 000 çemberlik arama son derlemede gerçek fareyle
Durdur üzerinden iptal edildi; iki nesnelik seçim korundu. İptal çizimi ve
çoklu çizgi kabulü arayüzden kaydedilip kendi başlangıç dosyalarıyla bayt bayt
karşılaştırıldı: ikisi de aynıdır. Ham görüntüler ve kabul kayıtları depo
dışındadır. Bu kabul bloklaştırma veya ilişkili dizi dönüşümünü kapatmaz.

**Bloklaştırma önkoşulu — [ADR 0868](../docs/adr/0868-explicit-dimension-rebind-uses-current-geometry.md):**
yeni blok alt nesnesine aynı işlemde açıkça geçirilen ölçü bağı eski geometri
üzerinden izlenip kopmuyor. Aynı/yeni yerleştirme kimliği, yeni ölçü, geçersiz
yer ve tek Undo/Redo domain testlerinde; üç public komutun dış işlemde
birleşmesi uygulama testinde doğrulandı. Gerçek konsolda önce kopma tekrarlandı,
yeni sürümde iki bağ canlı kaldı; taşıma ile 2 000 mm ölçü izledi. GUI'den
kaydedilen taşıma Undo, blok+bağ Undo ve Redo dosyaları karşılaştırıldığında
beklenen dosyalarla bayt bayt aynı çıktı. Genel dönüştürme hâlâ açık;
geniş çizginin blok içi dolgu farkı ayrı ADR 0869 kapsamında giderildi.
Bu dilimde 136 domain, 370 native uygulama ve 1 628 masaüstü testi geçti;
294 isteğe bağlı test atlandı. İlgili paketlerin Clippy kontrolü ve masaüstü
derlemesi başarılıdır.

**Blokta görünüş koruma — [ADR 0869](../docs/adr/0869-wide-polyline-ink-inside-blocks.md):**
geniş çoklu çizgi blok alt nesnesi olunca katmanın alan dolgusu yerine kendi
rengindeki dolgusunu korur. İç içe/ayna/ölçek, farklı katman dolgusu,
alan dolgusu görünümü ve fiziksel çıktı hassasiyetiyle 32 karşılaştırmada
kök çizgiyle çizim verisi eşittir. Stil paketinde 77, son masaüstü tekrarında
1 628 test geçti; 259 isteğe bağlı test atlandı. Gerçek son GUI'de seçili ve
seçimsiz dolgu, zoom ve 2 000 mm ölçü doğrulandı; kaydedilen dosya önceki
veriyle bayt bayt aynı. Ölçü bağı diliminin tam Python tekrarı da 481 testle
başarılıdır. Genel aday dönüştürme ve ilişkili dizi işleri açık kalır.

**Kapalı çizginin son kenarı — [ADR 0870](../docs/adr/0870-closed-polyline-dimension-geometry.md):**
ölçüye verilen native geometri kapalı durum ve genişlikleri korur; kökte veya
iç içe blokta son yay kenarı artık açık çizgi gibi okunmaz. Eski JSON/WASM
varsayılanı ayrı kalır. 605 çekirdek birim, 316 çekirdek entegrasyon, 137 belge,
371 uygulama, 1 628 masaüstü ve 68 MCP testi; 62 Python blok ve 6 çıktı testi geçti.
Clippy ve masaüstü derlemesi başarılıdır. Gerçek konsolda önce ret tekrarlandı,
sonra kapanış yayının orta noktası (800,425) mm olarak bağlandı, canlı kaldı
ve blokla taşındı. Taşıma Undo, bağlama Undo ve Redo kayıtları ilgili başlangıç
ve son dosyalarıyla bayt bayt aynıdır. Çizgi gerçekten açılınca yalnız kapanış
kenarı bağının kopması Python testindedir. Genel dönüştürme işi açık kalır.

**Seçilmiş aday dönüşümü — [ADR 0871](../docs/adr/0871-selected-repeat-conversion-and-topological-maps.md):**
`cad.blocks.convert` ve Bloklar panelindeki **Bloğa dönüştür…**, yalnız işaretli
adayları yeniden doğrulayıp tek blok tanımına geçirir. İnceleme; ad, aday/parça
sayısı, kök→alt nesne eşlemesi, ölçü bağı ve gerçek nokta sapmasını açıklar.
Her adayın ilk UUID/slot'u yerleştirme olarak korunur; kalan kökler tanımın
parçalarına eşlenir. Döndürülmüş/aynalanmış yaylarda dünya geometrisi, ters ve
döngüsel çoklu çizgide köşe/kenar topolojisi esas alınır. Belirsiz kenar,
güncel olmayan ölçü noktası, türetilmiş ölçü kuralı, field/grup/tarama/dizi
ve diğer taşınamayan ilişkiler sessizce koparılmaz. İlk kapsam aynı veri
davranışsız CAD katmanındaki sürekli çizgili analitik nesnelerdir; öznitelik,
kural tabanlı görünüş, kesikli çizgi fazı, araya başka nesne giren parça sırası
ve tablo bulunan çizim gerekçeli ret alır. Daha geniş destek açık iştir.

İnceleme/yazma arka plandadır; iptal ve değişen belge/geç sonuç koruması
masaüstü testlerindedir. Kaynak 128, grup 1 000, ilişki denetimi yapılan çizim
50 000 nesneyle sınırlıdır; eksik tarama doğrulanmış başarı sayılmaz.
50 000 nesneli bağımsız kabul çiziminde 1 000 tek parçalı adayın dönüşümü
bu makinenin debug derlemesinde yaklaşık 0,36 saniye sürdü; tek Undo başlangıç
verisini tam geri getirdi. Bu ölçüm genel performans garantisi değildir.

Gerçek arayüzde 40 yaylı/genişlikli adayın toplu dönüşümü ve yalnız bir adayın
seçilmesi denendi. Diğer 39 aday tekli dönüşümde değişmedi; bütün kök UUID'leri
korundu. Kaynak 2 000 mm ölçü canlı kaldı, fareyle taşınan blokla birlikte
yer değiştirdi. Toplu/tekli dönüşümün Undo/Redo dosyaları ve taşıma Undo'su
ilgili başlangıç/son dosyalarına bayt bayt eşittir. Boş ad, ad değiştirince
yeniden inceleme, ayrıntı aç/kapat, Esc ile vazgeçme ve küçük pencerede büyük
yazı kontrol edildi. Geçmişte komutun Türkçe adı ve toleransın metre girdi
yerine okunur mm özeti bulunur; public Python tekrar girdisi değişmez.

Çekirdeğin 610 birim ve 316 ana entegrasyon, uygulamanın 99 birim ve 378 ana
entegrasyon, MCP ana yüzeyinin 69 ve Python'un 487 testi geçti; dönüşümün
ortak komut dosyası 30 bağımsız durum içerir. Masaüstünün 1 632 testi 8 MiB
test yığını/dört paralel iş ile geçti; önceki varsayılan turdaki yığın taşması
ve GeoPackage yazma hatası gizlenmez. Son Geçmiş birim özeti değişikliğinde
11 odak testi yeniden geçti. Derleme ve Clippy sonuçları teslim kaydında
tutulur. Ekranlar, gerçek GUI dosyaları ve ayrıntılı kayıtlar depo dışındadır.
İlişkili diziye benimsetme ve kaynak/ilişki genişletme tamamlanmadan
CAD-ADV-07 bütünü kapatılmaz.

**Mevcut dizi önkoşulu — [ADR 0872](../docs/adr/0872-associative-array-closed-path-geometry.md):**
kapalı çoklu çizgiyi ilişkili dizi hesabında açık okuyan native okuyucu
düzeltildi. Önizlemede dört kenara dağılan öğelerin onaydan sonra üç kenara
kayması gerçek eski GUI'de tekrarlandı. Aynı fare/klavye adımları son
derlemede dört kenarda aynı sekiz yerleşimi verdi. **Diziyi düzenle** ile
8→12 adet değişimi dört kenarı kullanır ve ilk sekiz öğenin UUID'sini korur.
Dizi oluşturma ve adet değişiminin gerçek Undo/Redo kayıtları başlangıç,
sekizli ve on ikili dosyalara bayt bayt eşittir. Proje mm gösterimi ve
metre model koordinatları değişmedi. Üç regresyon testi önce başarısız olup
düzeltmeyle geçti; domain paketinin toplam 140 testi, native/Python'da 136
ortak dizi durumu, dört Python dizi testi ve 70 MCP testi başarılıdır.
Masaüstü derlemesi ve ilgili paketlerin Clippy kontrolü geçti. İki yönde
kapanış yayı istasyonu/teğeti, yol aç/kapat izleme, iki silinmiş öğe,
elle taşınmış istisna ve açık reset otomatik kabulde vardır; bunların
tamamı fareyle denenmiş gibi sayılmaz. Ham ekranlar ve dosyalar depo
dışındadır. Bu önkoşul, bağımsız öğelerden örüntü tanıma/benimsetme
maddesini kapatmaz.

**Mevcut yerleştirmeleri bağlama altyapısı — [ADR 0873](../docs/adr/0873-existing-insert-array-adoption.md):**
`cad.arrays.adopt` incelenmiş açık grid/doğru/kutupsal/yol yerleşimini ve
öğe eşlemesini mevcut `AssocArray` modeline bağlar. Yalnız metadata eklenir;
mevcut kök UUID/slot/çizim sırası, blok tanımı, geometri ve ölçü referansları
değişmez. İki eksik yer boş, açıkça `kept` denmiş öğe yerinde kalır.
Yanlış öğe kendiliğinden taşınmaz veya korunmuş sayılmaz. Ortak şablona
uymayan görünüş/veri, başka dizi üyeliği ve eski belge durumu atomik ret alır.
Statik, ortak görünüşlü, verisiz yerleştirme kapsamı ve ayrı açı/ölçek/konum
toleransları katalogda/Python belgesinde açıklanmıştır. Durdurulabilir işin
hesabı sürerken belge değişirse metadata-only yazma da reddedilir.

Native 99 birim + 386 entegrasyon, komut sözleşmelerinde 1069, MCP'de 71,
masaüstü komut geçmişinde 5 ve Python'un tam paketinde 495 test başarılıdır
(dizi modülünde 9 test). Bağımsız 30 ortak durum native ve Python
koşucularında geçmiştir; ilgili beş paketin
tüm hedeflerinde Clippy başarılıdır. Gerçek uygulamada konsoldan yazısız
inceleme/bağlama, ardından **Diziyi düzenle** menüsünden 8→10 sütun ve
2000→3000 mm aralık, öğe silme ve açık Sıfırla kullanılmıştır. Altı özgün
UUID, iki boş yer ve elle taşınmış öğe korunmuş; iç blok çemberlerine bağlı
ölçü 2000→3000 mm izlemiştir. Sekiz gerçek Undo/Redo kaydetme çifti birebir
dosya eşitliğiyle, yeniden açılışta dizi/bağlar da API ile doğrulanmıştır.
Son ikiliyle gerçek konsolda sayı taşması ve tüm kaynaklar birlikte
taşındıktan sonra eski planla yazma reddi de doğrulanmıştır; ret belgeye
dizi eklememiştir. Yeniden bağlama/Undo/Redo ve menüden sayı/aralık
değişikliği aynı ikiliyle tekrarlanmıştır. Görüntüler ve ham kabul kayıtları
depo dışındadır.

**Doğrusal aday bulma — [ADR 0874](../docs/adr/0874-read-only-linear-array-patterns.md):**
`cad.arrays.find` aynı statik blok tanımı, görünüş, ayna, yön ve ölçekteki
yerleştirmelerden doğrusal öneriler üretir. Kaynak sıranın ortasında olabilir;
aday ilk dolu uca eşlenir. En az üç dolu yer ve gözlenmiş komşu çift gerekir;
her ikinci yer doluysa gizli ara yer varsayılmaz. İki UUID aynı yere düşerse
rastgele biri seçilmez. Eşleşmeyen öğe kendiliğinden korunmuş sayılmaz.
Öneriler mevcut `cad.arrays.adopt` denetiminden geçer; sorgu yazmaz ve açık
uygulama için aynı tipte girdi döndürür. Kapsam 2 048 kök, geometri iş sınırı
iki milyon deneme; kısmi sonuç ve çakışan aday açıkça bildirilir. Python/MCP
ve durdurulabilir native Job aynı sorguyu kullanır.

**Diziyi düzenle** aracında doğru düzenine eksik olan görünür **Aralık (A)**
eklendi. Birimli pozitif uzunluk gerçek komşu aralığıdır; mevcut yön ve
fit anlamı korunur. Gerçek uygulamada yazısız arama, eşleşmeyen öğenin açıkça
korunması, menüden 2000→3000 mm ve 8→10 adet denenmiştir. Altı özgün UUID,
iki boş yer, korunmuş öğe ve bağlı ölçünün 3000 mm sonucu korunmuştur.
Sıfır aralık ret dosyası ile aralık/adet Undo/Redo'nun toplam beş çifti
birebir dosya eşitliği vermiştir. Grafik aday paneli henüz teslim edilmez.

Son kaynaklarla komut sözleşmelerinde 1 072, native uygulamada 99 birim +
395 entegrasyon, etkileşimde 71 birim + 882 entegrasyon ve MCP ana kabulünde
72 test başarılıdır; MCP'nin ek protokol/mimari kabul dosyaları da geçmiştir.
Python dizi modülünde 12 test ve bağımsız 27 ortak durum başarılıdır.
Komut geçmişinin beş kontrolü, SDK/manifest/bağımsız durum üretimi denetimi
ve ilgili paketlerin Clippy denetimi başarılıdır. Son derlemeyle gerçek
sorgu, bağlama/Undo/Redo ve görünür aralık değişimi tekrar doğrulanmıştır.
Ekranlar, ikili özetleri ve dosya karşılaştırmaları depo dışındadır.

**Grafik doğrusal inceleme — [ADR 0875](../docs/adr/0875-linear-array-review-panel.md):**
Bloklar panelindeki **Dizileri bul…**, şeridin mevcut Blok ek menüsü ve
`DIZIBUL` aynı akışı açar. Görünür birimli arama ayarı, sanallaştırılmış aday
kartı, dolu/boş/korunan yer şeması, Göster ve ayrı İncele vardır. Ortak native
sorgu arka planda durdurulabilir çalışır; Python çıktısı ayarına uyar.
Eşleşmeyen blok, kullanıcının seçtiği boş yere açıkça korunmuş öğe olarak
atanır; kaynak/dolu yerin üzerine yazılmaz. Atama kaldırılabilir, taslak ve
vazgeçme yazmaz. Diziye bağla aynı native Job ile metadata yazar; eski
belge/iş sonucu uygulanmaz. Tek Undo/Redo ve iptal/yeniden inceleme vardır.

Gerçek fare/klavyeyle arama, Göster, İncele, eşleşmeyen blok listesi,
Bloğu göster, atama/kaldırma/yeniden atama, vazgeçme, uygulama ve Undo/Redo
kullanılmıştır. 2200×1364 pencerede açık/koyu tema denendi; dar dock başlığının
kırpılması görülüp düzeltilmiş ve yeniden kontrol edilmiştir. DIZIBUL gerçek
komut satırından açılmıştır. Kaydet-aç dosyaları bağımsız KCAD okuyucusuyla
incelendi: bağlama bütün kökleri, altı özgün UUID'yi, tanımı, ayarları ve ölçü
referanslarını korur; iki boş yer ve tek korunan öğe vardır. Aralık menüsünden
2000→3000 mm değişiminde korunan öğe yerinde, bağlı ölçü 3000 mm'dir.
Vazgeçme/Undo başlangıç dosyasıyla, Redo bağlanmış dosyayla birebir aynıdır.

Masaüstü genel koşusunda 1640 test geçti; tek katalog envanteri uyumsuzluğu
üreticisiyle giderildi ve dokuz katalog testi geçti. Son görünüm düzeltmesinden
sonra 26 Bloklar paneli testi ve tüm hedeflerin Clippy denetimi başarılıdır.
Manifestte 946 satır tutarlıdır. Ekranlar, ikili özetleri ve kabul dosyaları
repo dışındadır. Grafik aramanın ilk kapsamı hâlâ doğrusal statik bloklardır.

**Dünya X/Y grid tanıma — [ADR 0876](../docs/adr/0876-axis-aligned-grid-pattern-review.md):**
`cad.arrays.find` opsiyonel `kind=grid` ile mevcut eksen paralel grid modeline
öneri üretir; alan verilmezse önceki doğrusal davranış aynıdır. En az iki
satır, iki sütun, dört dolu yer, iki yönde gözlenen komşu çift ve dolu bir
sınır köşesi gerekir. Boş köşe doldurulmaz; dolu köşeden işaretli aralıkla
başlanır. Gizli yarım aralık, çakışan UUID ve dönük/eğik grid varsayılmaz.
Yerel Float64 hesap ve ortak bağlama denetimi kullanılır. Ek keşif sınırları
500 000 çift / eksende 128 aralık adayı; aşımı açık kısmi sonuçtur.

Masaüstünde Düzen türü, satır/sütunlu şema ve kaymış öğenin korunacağı ayrı
Satır/Sütun alanları vardır. Gerçek fareyle iç kaynaktan 3×4 aday, yanlış
kaynak yere atama/ret/düzeltme ve bağlama denendi. Uzun formda görünmeyen
hata mesajı düğmelerin üstünde sabitlenip gerçek ekranda doğrulandı. Bağlama
on kök UUID'sini, tanımı, proje ayarlarını ve ölçü bağlarını korur; iki boş
ve tek korunan öğe kalır. Menüden sütun 2000→2500 mm, satır 3000→4000 mm
ayrı düzenlendi; korunan blok yerinde, bağlı ölçü 2500 mm'dir. Beş dosya
çifti vazgeçme/bağlama/aralık Undo/Redo'da birebir eşittir. Ham ekranlar ve
bağımsız dosya kontrolleri repo dışındadır.

Grid doğrulamasında 15 native tanıma, 99 birim + 400 entegrasyon ve ayrıca
filtresiz masaüstü fixture kontrolü başarılıdır. 31 bağımsız durum native/Python,
13 Python dizi, 128 MCP, 28 panel, 12 geçmiş ve dokuz UI katalog testi geçti.
SDK/fixture/manifest (946 satır), TS bağlamaları ve ilgili tüm hedeflerde son
Clippy denetimi başarılıdır. Geniş koşudaki filtre, masaüstüne özel fixture
klasörünü boş bıraktı; bu tek koşu ayrıca filtresiz doğrulandı.

Son teslim kaynağında dokuz dizi arayüz testi tekrar başarılıdır. Son derlemeyle
arayüzden grid arama/inceleme/açık atama/bağlama tekrarlandı; bağımsız okuyucu
özgün köklerin aynılığını ve incelenen üyeliğin korunduğunu doğruladı.

**Büyük koordinatta kısa aralık düzeltmesi — [ADR 0877](../docs/adr/0877-full-span-pattern-spacing-validation.md):**
Gözlenen komşu aralıktan kurulan üyelik, toplam açıklıkla yeniden denetlenir;
ilk tahminde dışarıda kalan uç kök yeniden taranır. Başarısız öneri sonraki
geçerli öneriyi bastırmaz. Tolerans, kaynak koordinatı ve ölçek değiştirilmez.
18 native tanıma, 33 ortak native/Python, 13 Python ve dört ilgili MCP testi
başarılıdır. Gerçek arayüzde 11 blok/3 mm bulundu ve bağlandı; bağımsız
okuyucuda yalnız dizi metaverisi değişti, Undo/Redo dosyaları birebir eşittir.
180 bilinen yerleşim taramasında toleransı koordinat ULP'sine eşit/üstünde
olanlarda tam örüntü eksilmedi; ULP altındaki 52 örnek açık sayısal sınırdır.
Aynı uç koordinat testinde küçük sembol render'ı düzleşti; ayrı hata olarak
incelenecek, bu sonuç render kabulü veya 07'nin tamamlanması sayılmaz.

**Tanıma denemesinde bulunan native çizim hassasiyeti — [ADR 0878](../docs/adr/0878-styled-geometry-low-part-stream.md):**
Büyük koordinatta 0,6×0,5 mm sembolün düzleşmesi giderildi. Native stilli
konumlar FP32 yüksek/düşük akışla, kamera parçaları çıkarıldıktan sonra
birleştirilir. Kaynak koordinatı, birimi ve proje ölçeği değişmez. CPU/PDF
aynı düşük parçayı korur; PDF halka yönü ortak yerel alan hesabındadır.
Eski/web akışı isteğe bağlı ek tamponu üretmez. 207 stil/PDF/render testi,
üç gerçek GPU testi dahil, başarılı; altı elle performans testi atlandı.
Son halka regresyonuyla 17 PDF testi ayrıca tekrar geçti; ilgili tüm
hedeflerde Clippy başarılıdır. Gerçek pencerede dikdörtgen ve daireler,
Kaydır ve tekerlek yakınlaştırması doğrulandı; kaydedilen KCAD başlangıçla
birebir aynıdır. Web düşük akış bağlantısı, dinamik raster ve boya/desen
koordinatı sınırları ADR'de açıkça ayrı tutulur; tüm render motorunun veya
CAD-ADV-07'nin tamamlandığı anlamına gelmez. Ek olarak 84 masaüstü stil/
önbellek testi geçti (dokuz elle performans testi atlandı). Son teslim
programında kenardan seçim ve 11 öğeli arama 413–414 ekranlarıyla tekrarlandı.

**Kutupsal aday bulma — [ADR 0879](../docs/adr/0879-full-circle-pattern-recognition.md):**
Aynı arama penceresinde açık **Kutupsal (tam çember)** ailesi, en az dört
mevcut öğe ve gözlenen komşu aralıkla 360° yerleşim önerir. Yönü koruma ve
merkezle dönme ayrı doğrulanır; merkez/yarıçap/açı/yer sırası görünürdür.
Kaymış kök yalnız açık boş-yer atamasıyla korunur; dolu yere atama reddedilir.
Sorgu/bağlama aynı durdurulabilir native, Python ve MCP yüzeyidir. 26 native
tanıma, 38 ortak native/Python, 14 Python dizi, beş ilgili MCP ve 11 panel
testi geçti; ilgili bütün hedeflerde Clippy başarılıdır. Gerçek iki çizimde
yön davranışları ve son derleme doğrulandı. Bağımsız okuyucuda bağlama yalnız
dizi kaydı ekledi; bütün kökler ve kimlikler aynıdır. Vazgeçme/Undo/Redo ve
menüden 8→10 adet değişiminin dosya çiftleri bayt bayt eşittir. Korunan
öğe yerinde, eksik yer boş kaldı. Ekranlar 415–424 depo dışındadır.

**Kılavuz boyunca aday bulma — [ADR 0880](../docs/adr/0880-explicit-guide-pattern-recognition.md):**
Açık seçilmiş 2B kılavuzun kayıtlı başlangıcından/yönünde sabit yay uzunluğu
aralıkları aranır. Kaynak başlangıçta olmalıdır; nesne taşınmaz, ters yön
ve ofset varsayılmaz. En az üç normal öğe ve gözlenen komşuluk gerekir.
Köşeli/kapalı yol ve analitik yayda yönü koruma/teğeti izleme aynı yerleşim
motoruyla doğrulanır. 415 native entegrasyon testi (37 elle test atlandı),
son koruyucularla 29 tanıma testi, 47 ortak native/Python durum, 15 Python,
altı ilgili MCP ve 12 panel testi geçti; ilgili tüm hedeflerde Clippy
başarılıdır. Gerçek pencerede ortak seçim, arama, açık koruma, bağlama,
2000→1500 mm aralık ve kılavuz köşesi sürükleme denendi. Bağımsız okuyucu
metadata-only bağlamayı, bütün kimlikleri ve tek Undo/Redo dosya eşitliğini
doğruladı; yeni kılavuzdaki konum/açılar bağımsız hesapla denetlendi.
Ret → yeni kaynak → başarılı inceleme teslim programında tekrarlandı.
Ekranlar 425–433 depo dışındadır. Bu dilim tüm 07'yi kapatmaz.

**Bu dilim sonundaki durum:** dönük/eğik grid açık kalmıştı. Yazı/iç blok
ADR 0881, son düzen ailesi ADR 0882 kabulüyle aşağıda tamamlandı.


**İlerleme — yazı değerleri ve statik iç blok kabulü (3 Kasım 2026):**
[ADR 0881](../docs/adr/0881-repeat-text-values-and-nested-blocks.md).
Ayrı yazı/öznitelik seçenekleri, açık metin→öznitelik eşlemesi, gerçek dünya
pozu doğrulaması, iç blok tanım/değer koruması ve [0,0]→[0,0,0] ölçü yolu
aktarımı tamamlandı. Tanımların bütünüyle açılması 200000 iş birimi ve
iptal sınırıyla denetlenir; aday kutusu içeriği kapsar. Temsil edilemeyen
font/bağ ve farklı iç değerler çizimi değiştirmeden gerekçeli ret alır.

420 native, 16 çekirdek, 76 MCP, 8 panel ve 12 doğrudan Python testi;
63 bağımsız native/Python ortak durum, katalog/TS/SDK/manifest ve tüm hedef
Clippy/derleme doğrulandı. Gerçek son programda dört farklı döndürülmüş yazı,
bağlı 2000 mm ölçü, iki iç blok ve 4000 mm ölçü, inspector'da tek değer
düzenleme, inceleme/ret ve tek Undo/Redo sınandı. Bağımsız KCAD okuyucusunda
UUID/değer/yol koruması ve dosya eşitlikleri doğrulandı. Ham ekranlar 434–453
depo dışındadır. Bu tarihli dilimden sonraki son kabul aşağıdadır.


**Tamamlandı — dönük/eğik 2B ızgara ve son kabul (3 Kasım 2026):**
[ADR 0882](../docs/adr/0882-directed-2d-grid-recognition.md),
[kullanım/API](../docs/cad/repeated-components.md).

- [x] Eski dünya-X/Y `Grid` korunarak bağımsız yönlü `Lattice` eklendi.
  Dünya doğusundan derece eksenler, metre aralıklar ve öğenin kendi yönü
  ayrıdır. Tanıma açık seçilir; tam açıklıkla FP64 uyarlaması yapılır,
  son kabul özgün mutlak toleransta mevcut benimseme motorundan geçer.
- [x] Görünür ayarlar, iki eksenli sonuç kartı, boş hücre şeması, ayrı
  İncele/Uygula, okunabilir derece menüsü ve uzunluk birimli açıya ret.
  Sonuç satırının yüksekliği iki ekseni kesmeyecek şekilde ayrıldı.
- [x] Satır/sütun adresleri, eski UUID'ler, korunan/boş hücreler, ölçü
  bağları, sil/reset ve tek Undo/Redo; Python tipli API, native Job,
  MCP kataloğu, web kopya komutu ve KCAD 161 sözleşmesi birlikte teslim.
  Eski ızgaraların dosya baytları aynı kalır.

**Doğrulama:** 424 native uygulama, 235 KCAD, 13 panel, 132 MCP,
7 ilgili çekirdek, iki ilgili etkileşim testi; 16 doğrudan Python ve
169 bağımsız dizi komut durumu içeren ortak Python kabulü; 38 web dizi
kabulü başarılıdır. Katalog 4, TS bağlamaları 1050, SDK üretim denetimi,
bağımsız dosya okuyucusu, ilgili tüm hedeflerde Clippy ve masaüstü derleme
geçti. Son gösterim düzeltmesinden sonra etkileşim testi/Clippy/derleme
tekrar geçti. Web genel tip kontrolü önceden var olan hatalar nedeniyle
yeşil değildir: sanal `HEAD` karşılaştırması 275→267 hata, yeni hata sıfır.
Bu genel kontrol tamamlanmış gösterilmez.

Gerçek son uygulamada 23°/81° iki eksen, iki boş hücre, ayrı inceleme ve
iptal, 35° yön düzenlemesi, `2500mm`→2,5 m aralık, 3→4 satır, bir öğe
silme ve üç hücreyi sıfırlama denendi. Son sürümde yeniden açılmış dizi
ve düzeltilmiş 23.0000°/81.0000° menüsü görüldü. Kaydedilen dosyalarda
bağımsız trigonometri, eski UUID'ler ve bağlı ölçü uçları denetlendi;
iptal, bütün ilgili Undo/Redo dosyaları bayt bayt eşittir.

Ayrıca ±4 milyar metre WCS çevresinde 3/5 mm aralıkla 52 öğe/iki boş hücre
sahnede açıldı, bulundu, bağlandı ve yakınlaştırıldı. Yalnız dizi metaverisi
eklenir; Undo/Redo ve yakınlaştırma sonrası dosyalar birebir aynıdır.
Bağımsız dünya hesabında en büyük sapma 0,000000476837158203125 m olup
0,000001 m kabul toleransının altındadır. 32 768 çift/96 vektör/2 milyon iş
ve sonuç sınırları arayüzde kısmi sonuç olarak açıklanır; bu stres çiziminde
kısmi uyarı görüldü ve gizlenmedi. Ham ekranlar 454–469 ve kabul dosyaları
depo dışındadır; ekran görüntüsü Git'e eklenmedi.

**Kapsam:** kontrollü ve yeniden üretilebilir 2B tanıma teslim edildi.
Dinamik/NURBS/tarama gibi ilan edilmiş destek dışı kaynaklar, taşınamayan
veri/ilişkiler veya tolerans altı sayısal ayrışma sessizce yaklaşık kabul
edilmez. AI görsel benzerliği opsiyonel gelecek çalışmadır; bu maddede
zorunlu değildir. Önceki dilimlerdeki açık durumlar yukarıdaki son kabulle
kapanmıştır; CAD-ADV-07 tamamlandı.


### CAD-ADV-08 — Sınır, tarama ve maskeyi hatasını açıklayabilen bir üretim sistemine getir [x]

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


**Uygulama durumu — 3 Kasım 2026 (kabul tamamlandı; [ADR 0912](../docs/adr/0912-cad-boundary-and-hatch-inspection.md)):**
- `cad.drawing.inspect` salt okunur/durdurulabilir sorgusu ve KentOS UI bileşenleriyle
  CAD denetimi paneli: kapsam, teşhis eşiği, bulguya gitme, sahnede işaret, tarama
  bağ durumu ve düzenleyiciye geçiş. Eski belge/sürüm sonuçları işlem yaptıramaz.
- Yeniden sınır seçmede çizgiler/kapalı nesne, adalar, ilişki, yazı boşluğu ve açık
  boşluk eşiği seçenekleri görünür. Denetimden onarım, önceki düzenleyici kipini
  kullanarak yanlışlıkla ilişki koparmaya başlamaz.
- Tarama alanındaki somut hata giderildi: dairesel yaylar görüntü çokgeninden
  ölçülmez; kayıtlı analitik sınır kullanılır. İki adalı daire bağımsız `98π`
  referansıyla, yeniden bağlama ve tek Undo ile sınandı. Tam elips alanı analitiktir;
  karışık elips/NURBS sınırındaki mevcut kord yaklaşımı korunur, kesinlik iddiası yoktur.
- 316 geometri, 433 uygulama, 15 tarama etkileşim, 2 masaüstü önizleme, 2 Python
  ve 1 gerçek MCP testi geçti. Kaynak bölmede doğru parçaya bağlanma, analitik alan,
  desen başlangıcı/açı/ölçek ve tek Undo ayrıca üç testte doğrulandı.
- Gerçek fareyle ilişki kopar/yeniden seç/tek geri alma; iki ada + yazı boşluğu +
  4 mm açıklık çizimi, yakın/uzak görünüm ve vektör PDF incelendi. Ham görüntüler
  depo dışında `/tmp/kentos-cad-adv08-09-2026-11-03/` altında tutulur.
- Kesin topoloji teşhisi doğru/dairesel yay kapsamındadır; blok/elips/NURBS/spline
  ve sınırı aşan kaynaklar açıkça kapsam dışında raporlanır. Bunlar temiz sayılmaz.
- Güncel main ile birleşim sonrası denetim/masaüstü kontrolleri geçti. Son eklemede
  10 uygulama denetim, 17 tarama etkileşim, 26 düzenleme regresyon, 3 Python ve
  2 MCP testi geçti. `cad.hatches.edit origin` yalnız desen başlangıcını değiştirir;
  karışık dolu/desenli toplu istek atomik reddedilir. Nokta/koordinat, açı ve ölçek
  gerçek arayüzde değiştirilip tek Undo ile geri alındı.
- Gerçek budamada üstteki taramanın kaynak daireyi seçtirmeme sorunu düzeltildi.
  Daire açık yaya dönüşürken tarama geometrisi/deseni korunur; panel 1 ilişkisiz
  gösterir. Tek Undo ve tekrar denetim 1 ilişkili gösterir. Son menü taşmasız
  görüntülendi; kaynak sil/düzelt/yeniden bağla, iki ada/yazı, yakın/uzak ve PDF
  kabulüyle CAD-ADV-08 tamamlandı. Yukarıdaki kesinlik sınırları geçerlidir.

### CAD-ADV-09 — Ölçü, kılavuz ve teknik anotasyonun doğruluğunu görünür kıl [x]

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

**Tamamlandı — 4 Kasım 2026, [ADR 0913](../docs/adr/0913-cad-annotation-truth-and-reference-repair.md):**

- `cad.drawing.inspect(checks=["annotations"])`: gerçek/gösterilen değer,
  override, tam/kısmi/serbest/kopuk ilişki; kılavuzun ilk ve ek kolları.
  Aynı okuyucu `cad.dimensions.query`, Python ve MCP'de. `<>` ölçülen
  değeri korur. Öznitelikler hesaplanan değeri **Gerçek ölçü** diye gösterir.
- KentOS UI denetim tablosunda seçili kapsam, sahnede sorunlu uç,
  Kaynağa git ve doğrudan yeniden bağlama. Seçili ölçü yeniden seçtirilmez.
  Kılavuzun istenen kolu gerçek alt nesneye bağlanır; boşlukta tıklama
  yazmaz. `cad.snap.find(all=true,sources=true)` çakışan kaynakları ayırır;
  normal kenet/Tab sırası korunur.
- İç içe blok yolundaki kılavuz ucu, örnek taşınmasını ve tanım
  değişmesini izler; kaynak parça kalkınca kopuk kalır. Tanım değişimi,
  taşıma ve geri alma test edildi. Kaynağı silinmiş ölçü/kılavuz ile
  override/kısmi ilişki baskı ön denetiminde görünür; tanı işareti basılmaz.
- Zincir/baz ölçüsünün gerçek konumlu yazı önizlemesi; genel ölçüde ve
  zincir/bazda yakın yazı kutularıyla olası çakışma uyarısı. Uyarı hiçbir
  geometriyi otomatik taşımaz. Mevcut ortak referans, baz aralığı, koordinat,
  birim/alternatif birim, tolerans, yuvarlama ve teknik çerçeve motoru korunur.
- Yukarıdaki ek blok kutusu açığı kapandı: API ölçümü, kutuyla sorgu ve
  GUI'nin **Kutu eni (Y)** / **Kutu boyu (X)** alanları aynı görünür blok
  parçalarını okur. İç içe dönüşüm, aynalama, farklı eksen ölçekleri,
  kırpma, etkin parametre ve gizli parça katmanları kapsanır. Yerel
  **En × boy** da gizli parçayla şişmez; seçim vurgusu gizli parçayı çizmez.
- Gerçek GUI: kısmi ölçünün serbest B ucu işaretlendi; kaynak seçildi,
  B yeniden bağlanınca bulgu kalktı ve tek Undo ile döndü. Kılavuzun
  kopuk ikinci kolu, ölçüyle çakışan çizgi ucuna bağlandı: 5 → 4 bulgu,
  tek Undo ile 5. Zincir ölçüsü önizleme/yerleştirmede aynı 857.143 mm;
  tek Undo. Çakışan 2500 mm yazısı yerleştirmeden uyarıldı; Esc yazmadı.
  İç içe aynalı blokta GUI ve API 12000 × 8000 mm gösterdi.
- Aynı kabul çizimi 1:20 ve 1:50 GUI paftasında ve iki sayfalık vektör
  PDF'de incelendi. Gerçek 2500 mm / kasıtlı 2400 yazısı korunur;
  anotatif ölçü ve kılavuz yazısının kâğıt boyu sabittir. Poppler iki
  sayfada ölçü yazılarının aynı kutu yüksekliğini doğruladı.
- Kontroller: 17 komut dosyasının 666 bağımsız durumu; 11 uygulama
  denetimi, 67 ölçü etkileşimi + 2 biçim testi, 10 domain referans testi,
  3 iç içe blok/kenet GUI mantığı, 3 headless blok ölçümü, 3 pafta testi,
  1 gizli blok vurgusu, 64 öznitelik testi (4 mevcut ignored), 4 MCP,
  5 Python testi geçti. Kenet çekirdeğinde 7 birim + 13 entegrasyon;
  katalog 4, TypeScript bağlama 1254 test; SDK/manifest tutarlı.
- Sınırlar ADR'de açıktır: yerel çakışma ipucu 128 yakın nesne ve
  muhafazakâr kutu ile sınırlıdır; tam otomatik yerleşim/ISO-ASME
  sertifikası iddiası yoktur. Tanımsız/boş blok kutusu ekleme noktasına
  düşer; alan/uzunluk uydurulmaz. Gizli katmanları kapsayan mevcut
  Tümünü göster kuralı değiştirilmedi. Ham GUI/PDF kanıtları repo dışında.

### CAD-ADV-10 — Yazı ve field üretimini ekran ile çıktıda aynı hale getir [x]

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

**Çalışma kaydı — yerel 2D CAD kabulü tamamlandı (4 Kasım 2026):**

- [ADR 0914](../docs/adr/0914-cad-text-layout-and-live-field-delivery.md)
  mevcut dört metin yolunu ve eski yerleşimi koruma koşullarını karşılaştırır.
- Paragrafta birleşik karakteri bozmayan geri/ileri silme uygulandı.
  Beş editör testi geçti; gerçek uygulamada aksanlı `g` tek geri silmeyle
  kaldırıldı, onaylandı ve tek geri almayla özgün hali döndü.
- Aile adı önekiyle yapılan yanlış font eşdeğerlikleri daraltıldı.
  Üç çekirdek font testi ve 28 bağımsız `cad.fonts.report` komut durumu geçti.
- Canlı field/önbellek denetimi ve güncel olmayan field'ın PDF teslimini
  engellemesi uygulandı. 14 uygulama denetim testi, bir kaynak döngüsü
  testi, iki masaüstü denetim testi ve beş PDF kitap testi geçti.
  Gerçek GUI'de eski değer → yenile → temiz denetim → tek geri alma →
  aynı hata dönüşü doğrulandı. Tam baskı denetimi field ve desteklenmeyen
  karakter hatalarını birlikte gösterdi; ekranlar repo dışında.
- Mevcut PDF'de aksan/teknik sembol/RTL karakterlerinin `?` olarak
  kaybolduğu bağımsız Poppler çıktısıyla kanıtlandı. Yeni koruma bu sessiz
  kaybı hata olarak reddediyor; karakterlerin doğru üretimi henüz tamamlanmadı.
- Ortak Unicode shaping çekirdeği, 29 paketli yüz için harici HarfBuzz,
  18 yön ve 12 kesilmiş satır için harici FriBidi, iki biçimli metin için
  HarfBuzz bağlam sonuçlarıyla karşılaştırıldı; yedi Rust testi geçti.
  Font dosyaları ve lisansları hash ile doğrulanıyor; yeniden üretim denetlendi.
- MTEXT sayısal paket aktarımında girinti/tab/sütun kaybı ve web kesir
  biçimi kaybı giderildi. Doğrudan dönüşümle paketli dönüşüm karşılaştırması
  ve 15 WASM paket testi geçti; geometri Float64 kalır.
- Güncel derlemede yedi PDF kitap testi ve beş editör testi yeniden geçti.
  Gerçek GUI'de tam hata açıklaması, alan yenileme ve temiz denetim tekrar
  kontrol edildi. Standart KentOS bileşenleri kullanıldı; ekranlar repo dışında.
- Aynı glyph satırlarından fiziksel girinti/tab/sütun ve sözcük bazlı sarma
  üretimi eklendi; dört harici akış örneği, 13 bağımsız FontTools konturu,
  15 satır içi nesne sırası ve 6.000 satırlık küme bütünlüğü kontrolü geçti.
  Mevcut metin regresyonlarıyla 32 test temizdi. Bu çekirdek henüz sahne/PDF
  motoru yerine etkinleştirilmedi; üst/alt simge ve kesir adaptörü sürüyor.
- Tüm 29 yüzün byte'ları ve runtime font sırası yeni profil manifestinde
  sabitlendi; UI fontu güncellemesinin eski metni sessizce yeniden
  akıtmaması için sürüm sınırı korunuyor.
- Ek incelemede blok tanımı, tablo ve kılavuz içindeki field yollarının
  ayrıca doğrulanması gerektiği belirlendi; bu kabul de tamamlanmadan madde kapanmaz.
- Üst/alt simge, beş kesir/tolerans örneği, alt çizginin gerçek kutusu ve
  dört karışık Latin/Yunanca/Kiril örneği eklendi. Güncel 20 entegrasyon
  ve iki yazı sistemi testi geçti; eski metin regresyonlarıyla 39 test temiz.
  Ligatür içi parçalı renk/kırpma henüz eksik ve açık hata olarak tutuluyor.
  Yeni ortak planın gerçek ekran/PDF hattına bağlanması devam ediyor.
- Açık `unicode-v1` profili KCAD şema 163 ile kök/blok yazılarında saklanıyor;
  alanı olmayan eski dosya legacy kalır. Ortak model konturu, gerçek ink kutusu,
  eğri üzerindeki atomik kümeler ve sürüm anahtarlı sahne önbelleği bağlandı.
  44 çekirdek ve 15 uygulama denetim testi geçti; zoom/değişiklik/Undo önbellek
  testi temiz. Gerçek GUI'de yedi yazı, düzenleme/Undo/zoom/Farklı kaydet
  denendi; bağımsız okuyucu Undo sonrası kaydın aynı byte'ları verdiğini doğruladı.
- PDF aynı GID ve dönüşüm matrisleriyle gerçek font alt kümeleri üretiyor.
  29 font/328 CID ve GUI dosyasından beş font/60 CID konturu bağımsız okuyucuyla
  eşleşti. 13 PDF testi geçti; eksik harf/profil/matris açık hata. RTL metnin
  farklı PDF okuyucularında arama/kopyalama kabulü henüz eksik; yalnız doğru
  görseli tam kabul saymıyoruz. Ekran/PDF dosyaları repo dışında.
- Güncel uygulamada iki pafta (1:20 ve 1:50) açıldı ve arayüzden birlikte PDF'ye
  kaydedildi. Beş font/64 CID konturu bağımsız okundu; yazı boyu oranı tam 2,5,
  ilk baseline kâğıt konumu bağımsız hesaba 0,00002 mm toleransla uydu.
  Tam KCAD takımı 235/235, yerel kitap PDF takımı 8/8 (bir ayrı dosya testi ignored) ve ortak PDF takımı
  13/13 geçti (ayrı dosya üretme testi ignored).
- CPU baskı önizlemesi de ortak konturlara bağlandı. İki paftada üretildi;
  bağımsız Poppler rasteriyle 1:50 sayfanın mürekkep sınırları bir piksel
  içinde eşleşti. Python bağlamasında altı ve MCP'de beş sözleşme testi geçti.
- Tablo alanlarında kayıtlı/güncel hücre ve bağlı yazı zinciri denetimi eklendi.
  Dosyaya bağlı olmayan alanlı tablolar da açık yenileme/tek Undo destekler.
  17 uygulama denetimi, mevcut tablo komut örnekleri, 10 kitap çıktı testi,
  altı MCP ve yedi Python testi geçti. Gerçek GUI'de yenileme/Undo ve
  eski değerli baskının reddi doğrulandı; hata artık baskı penceresinde
  standart uyarı şeridiyle görünür. Undo sonrası kayıt özgün byte'larla aynı.
  Blok içi alanlar ve tablo Unicode
  yerleşim geçişi bu kabulle tamamlandı sayılmaz.
- İç içe/dinamik blok yazılarında gerçek tanım/parça kökeniyle alan denetimi
  eklendi; anonim yazı için UUID uydurulmaz. Gizli katmanlar, XCLIP ve gerçek
  viewport kırpması izlenir. Sorguda öbekli çalışma ve parça içi iptal var;
  tanım hazırlığının mevcut atomik sınırı ADR'de açık. 25 uygulama, 14 çekirdek
  birim + 12 blok entegrasyon testi, dokuz kitap çıktı testi (bir ayrı dosya
  testi ignored), yedi MCP ve sekiz Python testi geçti. Gerçek GUI'de içteki
  eski alanın konumuna gidildi ve baskı önizlemesinin açık hata verdiği görüldü.
  Denetim/ret sonrası GUI kaydı özgün dosyayla byte düzeyinde aynıydı.
  Tanımın toplu yenilenmesi veya Unicode geçişi bu kayıtla tamamlandı sayılmaz.
- Tek yazının yerleşimi mevcut editörde Eski görünüm/Unicode olarak açıkça
  önizlenebilir. Öznitelik menüsü de aynı taslağı açar; onay tek Değiştir,
  Esc yazmaz. Eğri, boşluk ve güncel field korunur; eski field veya değişen
  hedef sessizce yazılmaz. 81 düzenleyici/öznitelik/inline regresyon testi
  (beş mevcut ignored), dokuz Python ve sekiz MCP testi geçti. Gerçek GUI'de
  koyu/açık tema, kısa sahnede sabit onay/iptal, property menüsü, canlı
  önizleme, eksik glyph reddi ve tek Undo denendi. Bağımsız okuyucu yalnız
  motor bilgisinin değiştiğini ve Undo sonrası özgün byte'ları doğruladı.
  Yeni yazının varsayılanı veya toplu dönüşüm bu adımla değiştirilmedi.
- Unicode `Okunur yap`, gerçek glyph/fallback/eğiklik çerçevesinin ortası
  çevresinde döner; JSON'daki eski genişlik tahmini yeni profili geçersiz
  kılamaz. Eski veya doğrulanamayan alan ve eksik glyph bütün dönüşümü
  yazmadan reddeder. 47 çekirdek, 28 uygulama, dokuz MCP ve 11 Python
  testi, sıkı masaüstü Clippy ve güncel derleme geçti. Gerçek GUI'de iki
  yazı topluca çevrildi; kaydedilen konumlar bağımsız HarfBuzz/FontTools
  çerçevesiyle iki nanometre içinde eşleşti, referans çizgileri değişmedi.
  Tek GUI Undo sonrası dosya özgün byte'lara döndü. Eski alanlı seçimin
  açıklamalı reddi de GUI'de denendi; ret sonrası kayıt özgün byte'larla aynı.
  Ekranlar ve QA dosyaları repo dışında; madde genel kabulü hâlâ açık.
- Unicode aynalama gerçek ölçülen merkezle ve miras alınan proje fontuyla
  çalışır; iç içe/dinamik blok önbelleği font değişiminde kaynaklardan yenilenir.
  Gerçek GUI'de önizlemeden Esc, kopyalı onay, kayıt ve tek Undo doğrulandı.
  Bağımsız referans çerçeveleri ve yazı merkezleri aynı aynaya 4 nm içinde uydu;
  Undo dosyası özgün byte'larla aynı. Tam çekirdek 973, uygulama 459,
  interaction 72, MCP 10, Python 12 testi; sıkı Clippy ve derleme geçti.
  31 çekirdek ve 37 uygulama testi kendi ayrı/ignored koşullarında kaldı.
- Son kabulde Türkçe/aksan/teknik sembol/RTL/sütun/kesir/eğri ve field
  içeren yedi yazı gerçek GUI'de açıldı, düzenlendi, aynalandı ve PDF'ye
  basıldı. Beş font/64 CID konturu özgün fontlarla eşleşti; CPU/PDF
  mürekkep sınırı en çok 1 px farklıydı. Preview'da Arapça, İbranice ve
  Türkçe sözcük araması; PDFium'da dört sözcüğün tam geri dönüşü doğrulandı.
- Toplu değişim grapheme sınırını ve önerilen Unicode planını yazmadan
  denetler. Uyumsuz profil/biçim birleştirmesi sessiz dönüşmez. Sekmeli
  satırları ayırma gerçek baseline'ı korur. GUI'de ayır/birleştir/bul-değiştir,
  açıklamalı ret ve tek Undo denendi; bağımsız okuyucuda ret/Undo kayıtları
  özgün byte'larla aynıydı. Son başarı bildirimi gerçek ekranda tekrar denendi.
- Son dilimde 50 çekirdek metin, 468 uygulama (37 ayrı/ignored), 72 interaction,
  18 plot birim, dokuz kitap testi (bir ayrı dosya testi ignored), ayrıca
  GUI kaydından CPU/PDF üretme testi; 11 MCP ve 13 Python testi geçti.
  Sıkı Clippy ve güncel derleme temiz. Ekranlar repo dışında tutuldu.
- Güncel destek matrisi ve sınırlar ADR 0914'te açık: eski metin otomatik
  değişmez; ligatür içi parçalı boya ve kapsam dışı glyph açıklamalı
  reddedilir. Üretilen etiketler/tablo legacy yolları değerlendirilip korunur.
  Bütün okuyucularda karışık paragraf kopya sırası, PDF/UA/A ve ertelenmiş
  web kabulü iddia edilmez. **CAD-ADV-10 bu açık yerel destek sözleşmesiyle tamamlandı.**

### CAD-ADV-11 — Karmaşık çizgi tiplerini ve çizgi görünüşünü eksiksiz çiz [~]

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

**Başlangıç incelemesi (4 Kasım 2026):**

- [ADR 0930](../docs/adr/0930-complex-linetype-renderer-audit.md) AutoCAD
  kaynaklarını, mevcut sözleşmedeki 12 öğe sınırını ve rendererin dört
  ayrı sekize kesme/periyot noktasını karşılaştırır. Yazı/şekil kaybı
  ve görünür kaynaklara dayanan denetim gereği kaydedildi.
- Gerçek GUI'de 12 öğeli çizgi deseni bağımsız tam desen geometrisiyle
  yan yana açıldı; sekizde kesilme ve HW işaretlerinin yokluğu görüldü.
  Ekran ve deney dosyaları repo dışında. Bu başlangıç hatası kanıtıdır;
  düzeltme sonrası kabul veya maddenin tamamlanması değildir.

**İlk çıktı koruması (4 Kasım 2026):**

- Gerçek stil derleme sonucundaki kaynak kaybı artık `linetype_unsupported`
  bulgusuyla GUI tam ön denetimine, Python ve MCP sorgusuna taşınır.
  PDF/önizleme ve doğrudan CPU/vektör çıktısı eksik görünüşü sessizce
  teslim etmez. Kullanılmayan tanım ve desteklenen özel sembol yanlış
  engel oluşturmamalı; bunlar regresyon testleriyle kapsandı.
- Gerçek 1:50 GUI testinde LONG12 ve WATER ayrı hata verdi; eksik PDF
  oluşmadı. Kaydetme reddinde kapanıp açıklamayı kaybeden çıktı penceresi
  düzeltildi. Aynı kullanılmayan tanımları taşıyan desteklenen desen
  dosyası GUI'den PDF üretti. Kanıtlar repo dışında; test sayıları ve
  kapsam sınırları ADR 0930'da.
- Bu, uzun/yazılı/şekilli deseni çizme tamamlanması değildir. Çokgen veya
  döndürülmüş viewportun kırpma denetimi 6 Kasım'da ADR 1007 ile eklendi
  (aşağıda). Maddenin tamamlandı işareti değiştirilmedi.

**Tam çizgi/boşluk yürütmesi (4 Kasım 2026):**

- Native GPU'da uzun tarifler paylaşılan storage sayfalarından okunur;
  kısa desenlerin mevcut uniform yolu korunur. CPU, hatch vektörleştirme
  ve uzak-origin periyodu tam listeyi kullanır. Ekran/PDF sekiz öğede
  kesilmez. Gerçek GPU'da 12/16 ve normalize 18 öğeli örnekler bağımsız
  mesafe hesabıyla karşılaştırıldı; genel GPU regresyonu geçti.
- Gerçek GUI'de 12 öğeli kaynak bağımsız referansla zoom yapılarak,
  1:50 CPU önizleme ve PDF ile karşılaştırıldı. Bağımsız Poppler çıktısı
  ve PDF operatörleri doğrulandı; GUI KCAD kaydı kaynakla bayt bayt aynı.
  1:1/1:20/1:100 vektör tarifleri ayrıca otomatik test edildi.
- PDF stroke başlangıç fazı da eklendi; aynı desenin faz değişimi ve
  PDF save/restore önbelleği kapsandı. Gerçek GUI'de özel stil içindeki
  12/+1,25 mm ve 16/−1,25 mm desenlerin PDF'leri bağımsız referansla
  karşılaştırıldı; PDF operatörlerinde faz ve tam liste doğrulandı.
- Kaynak KCAD/LIN'in adlandırılmış 12 öğe sınırını sürümlü genişletme,
  yazı/şekil ve kalan geometri/viewport/performance matrisi açık.
  Detay ve kanıt kapsamı ADR 0930'da; maddenin tamamlandı işareti kapalı.

**Viewport birimi ve pafta ekranı (4 Kasım 2026):**

- Çizim birimindeki desenin proje ölçeği yerine gerçek viewport ölçeğinde
  çözülmesi sağlandı. mm/cm/m/ft, kâğıt/çizim deseni, global/nesne
  çarpanı ve 1:1/1:20/1:100 için 48 durumlu çıktı matrisi geçti.
  Asıl masaüstü stil önbelleği ve salt okunur belge davranışı ayrıca sınandı.
- Gerçek arayüzde bulunan son yarım çizgi kaybı ve pafta ekranında fazın
  sıfırlanması düzeltildi. Kırpılan başlangıç, köşe ve kapalı dikiş devamı
  Float64 hesapla korunur; yoğun/noktalı tarif mevcut CPU yolunu kullanır.
- Gerçek 1:20/1:100 paftalar, iki sayfalı GUI PDF'si ve pozitif/negatif
  fazlı 12/16 öğeli stil örnekleri bağımsız mavi referanslarla karşılaştırıldı.
  Test ve kapsam ayrıntıları ADR 0930'da. Bu ilerlemeyi göstermek için
  ana durum `[~]`; yazı/şekil ve kalan kabul tamamlanmadan `[x]` yapılmayacak.

**Uzak path fazı (4 Kasım 2026):**

- Büyük birikmiş path mesafesinin kesirli fazı GPU upload'ında Float64
  korunup tam periyoda katlanır; kaynak geometri/mesafe değişmez.
  Büyük pozitif/negatif başlangıç fazı da Float32 uniformdan önce çözülür.
- Apple M5 gerçek GPU testi hatayı önce yakaladı, düzeltme sonrası CPU ve
  bağımsız mesafe referansıyla geçti. Gerçek uygulamada birikmiş mesafesi
  1.005.000.999,625 m olan path'in yakındaki parçası mavi referansla
  zoom yapılarak karşılaştırıldı; sınır farkı en fazla 1 piksel.
  Kapsam ve kalan hassasiyet sınırları ADR 0930'da; ana madde `[~]`.

**Karmaşık dosya kaynakları (4 Kasım 2026):**

- Yalnız çizgi tipinin kullandığı yazı stilleri ve ayrı adsız SHX kaynakları
  içe almada korunur. Sabit/değişken STYLE yüksekliğinin farklı S birimleri,
  gerçek yazı stili handle'ı ve U yönü aktarımda izlenir.
- Ham derece açısı bağımsız kodekle; kaynaklar AC1027/AC1032 DWG ile
  karşılaştırıldı. Gerçek GUI'de iki kaynak yazı stili ve parametreleri
  doğrulandı. Test ayrıntıları ADR 0930'da. Yazı/SHX çizimi hâlâ açık.

**Yazılı desen çiziminin ilk dilimi (4 Kasım 2026):**

- Paketli fontların ortak glyph planı artık çizgi tipi içinde GPU/CPU/PDF
  geometri hattından çizilir. R/A/U, köşe fazı, kaynak stil ve iki ölçek
  çarpanı korunur. Eksik kaynak ve iş yükü sınırında tam çıktı engeli sürer.
- Gerçek GUI'de HW, açılı/ters yazı ve köşe örnekleri; 1:20/1:50 pafta,
  iki sayfalı PDF ve KCAD kaydet–yeniden aç denendi. PDF'deki dört kaynak
  kelimenin sınırları ayrı referansla 0 piksel fark verdi. Test sayıları,
  değişikliksiz HEAD'de de olan iki eski golden-test hatası ve açık sınırlar
  ADR 0930'da. SHX/affine/genişlik/performance kabulü bitmedi; ana durum `[~]`.

**Geniş çoklu çizgi deseni (4 Kasım 2026):**

- Geometrik genişlikli yol artık kesintisiz dolu bant yerine adlandırılmış
  desenin mürekkep parçalarıyla çizilir. Yay kesimleri analitik uzunlukla,
  uç genişlikleri özgün kenardan hesaplanır; köşe ve kapalı dikiş korunur.
  Aynı tarif atlaması kılavuz/tablo ve blok içi geniş çizgilerde giderildi.
- 1:1/1:20/1:100 alan integrali, aynalı/iç içe blok, ortak model/plot,
  dolgu görünürlüğü, PDF reddi ve Python plan/execute kontrolleri geçti.
  Gerçek GUI ve iki sayfalı PDF'de 1:20/1:50 desenler ölçüldü: 5 mm parça,
  8 mm periyot; bitmap yok. Sonuç ve kalan sınırlar ADR 0930'da.
- SHX, tam affine, noktalı bant, analitik metin-yay fazı ve performans
  kabulü açık; madde `[~]` olarak kalır. Ekranlar repo dışında.

**Yönetici önizlemesi (4 Kasım 2026):** Çizgi tipleri listesi ve seçili
büyük örnek, mevcut KentOS GPU önizleme bileşeniyle gerçek yazılı deseni
çizer. Genel yanlış desteklenmiyor uyarısı kaldırıldı; çizilemeyen kaynağın
adı seçimde açıkça gösterilir. Konturlar açılışta/düzenlemede hazırlanır.
Gerçek LIN yükleme, farklı kaydetme ve yeniden açma testiyle doğrulandı;
kanıtlar ADR 0930'da. Bu dilim SHX çizimini tamamlamaz.

**SHX şekillerinin ilk çizim dilimi (4 Kasım 2026):** Klasik SHX 1.0/1.1
kaynakları açık seçimle projeye gömülür; ortak GPU/CPU/PDF hattında gerçek
stroke şekilleri çizilir. Kaynak/özet/yürütme hataları çıktı engelidir.
Aynı ekleme Python/MCP komutuyla yapılır; kullanılan kaynak silinemez.
Gerçek GUI'de ekleme, zoom, 1:20/1:100 pafta, iki sayfalı vektör PDF,
KCAD kaydı ve harici dosya olmadan yeniden açma doğrulandı. Testler ve
ölçülmüş PDF sonuçları ADR 0930'da. Unicode/Big Font ve genel SHX metin
fontları, harici teslim paketi ve kalan geometri/performance kabulü açık;
maddenin durumu `[~]` olarak korunur.

**SHX CAD teslimi (4 Kasım 2026):** Gömülü kaynak içindeki şekil adı gerçek
numarasına çözülür; DXF LTYPE kaydı adsız, shape-flag'li STYLE kaydına bağlanır.
DXF/DWG aktarımı doğrulanmış SHX baytlarını çizimin yanına yazar; aynı dosya
tekrar kullanılır, farklı içerik/symlink üzerine yazılmaz. Python/MCP planı
kaynak yolu, özet ve tekrar kullanımını bildirir, dosya yazmaz. Gerçek GUI'den
DXF ve AC1032 DWG üretildi; bağımsız DXF denetimi 128 nesne, şekil 1/2,
45° açı ve doğru kaynak bağlantısını doğruladı. Çakışan kaynakla GUI aktarımı
reddedildi, mevcut dosya korundu. Ayrıntılar ADR 0930'da; otomatik kaynak
ithali, diğer DWG sürümleri ve ana maddenin kalan çizim kabulü açık.

**Analitik yay üzerindeki işaretler (4 Kasım 2026):** Yerleşik CAD çizgi
tiplerinde daire, yay ve tek parçalı bulge path'in yazı/SHX konumu artık
çizim için üretilen küçük kirişlerden hesaplanmaz. Gerçek yay uzunluğu ve
teğet kullanılır; geniş bant da aynı yerleştiriciyi çağırır. Kapalı dikiş,
CW/CCW, köşe ve pozitif/negatif faz testleri eklendi. 48 çekirdek, 18 plot
birim ve 70 plot entegrasyon testi geçti (3 mevcut test atlandı). Gerçek
GUI zoom/iki pafta/PDF denendi; bağımsız okuyucuyla 1:20'de 91, 1:100'de 18
şeklin konumu ve 2 mm kol boyu doğrulandı. Elips/nonuniform dönüşüm,
çok parçalı path, özel offset/effect ve genel performans kabulü bu dilimin
dışında; taşıyıcı dash fazının bütün eğrilerde analitik olduğu iddia edilmez.

**Yazı stili bağımlılığı (4 Kasım 2026):** Yalnız çizgi tiplerinin kullandığı
yazı stili artık kullanılmıyor sayılmaz ve purge/silme ile kaybolmaz. Kullanım
sayacı ve silme açıklaması gerçek GUI'de doğrulandı. Ad değiştirme iki çizgi
tipinin bağını ve görünüşünü korudu; GUI Undo/Redo ve bağımsız KCAD okuması
geometri/kimlik/pafta korunmasını doğruladı. Komut validate/plan/execute,
düşük düzey kayıt, iki adın takası ve eşzamanlı değişikliği koruyan Undo testleri
geçti. Kalan ana kabul nedeniyle 11. madde `[~]` kalır.

**Etki açıklaması ve API doğrulaması (4 Kasım 2026):** `cad.styles.impact`
notları, doğrudan yazı nesnesi sayısından ayrı olarak bağımlı çizgi tipi
adlarını açıklar; stil penceresi aynı açıklamayı gösterir. Güncel Python
SDK'sı yeniden derlendi; 3 testte korunan stil, rename/KCAD döngüsü,
salt okunur plan, DXF/DWG yan kaynakları ve önceki 6 çıktı alt durumu geçti.
Gerçek MCP server testinde de SHX ekleme, kaynak bulgusunun çözülmesi,
stil etkisi/ret/rename ve iki CAD aktarımı doğrulandı. GUI/Python/MCP
kanıtları ADR 0930'da; ana maddenin açık kabulü değişmedi.

**Elips işaret mesafesi (4 Kasım 2026):** Mevcut sayısal integral ve
braketli ters çözüm, elipste yazı/SHX konumunu gerçek yay mesafesine bağlar.
Basıklık, döndürülmüş eksen, sıfır açısını geçen yay ve büyük koordinatlar
bağımsız 50 basamaklı eliptik integral referansıyla karşılaştırıldı. On birim
testi, üç ölçekte altı yolun 18 vektör/PDF senaryosu ve strict Clippy geçti.
Gerçek GUI zoom/iki pafta ve iki sayfalı PDF kontrol edildi; 106/21 şeklin
konum hatası 0,000016 mm altında, bitmap yok. Bu elips desteği, bütün blok
dönüşümü/performance kabulünün bittiği anlamına gelmez. Ayrıntılar ADR 0930'da.

**Blok matrisi ve tekrar hazırlığı (4 Kasım 2026):** Yazı/SHX/uzun tarif
üç ölçekte, aynalı, farklı X/Y ölçekli ve iç içe bloklarda 432 root/blok
karşılaştırmasını geçti. Gerçek model/zoom/iki pafta/PDF denendi; SHX kâğıt
boyu bağımsız ölçüldü. Aynı yazının üçgenlemesi artık bir kez hazırlanır;
kontur/hassasiyet değişmez. Apple M5/Metal debug ölçümünde 800 çizginin HW
hazırlığı 1591 → 294 ms (tek örnekte yaklaşık 5,4×), gerçek çizim alanının
önce/sonra görüntüsü 0 piksel fark. 51 çekirdek, 44 native-style, 18 plot
birim ve 70 entegrasyon testi geçti; strict Clippy/build geçti. GPU belleği
azalmadı, 1000 yoğun yazılı çizginin mevcut 2 milyon nokta sınırına takıldığı
açıkça kaydedildi; sınır artırılmadı. Genel release performansı ve diğer açık
kapsam tamamlandı sayılmaz. Kanıt ve ölçüm sınırları ADR 0930 §22–23'te.

**Uzun desen ve açılan DXF'in desen birimi (5 Kasım 2026, [ADR 1001](../docs/adr/1001-long-line-type-patterns-and-opened-pattern-units.md)):**
Çizimin sakladığı desen 256 öğeye çıktı (`.kcad` şema 181; LIN'in 12 sınırı LIN'e özgü kaldı). DXF/DWG içe alma
12'yi aşan deseni artık sessizce kesmez; 256'yı aşanı keser ve raporda söyler. LIN dışa aktarımı uzun deseni
`pattern_too_long_for_lin` uyarısıyla atlar. Gerçek pencere kanıtında bulunan ayrı hata düzeltildi: metre birimli
DXF açılınca çizim birimli desenler 1000 kat uzuyor ve çizgiler sürekli çiziliyordu. 16 öğeli desenin tire dizisi
açılan çizimin ekranından piksel ölçümüyle doğrulandı. Unicode/Big Font SHX, noktalı bant, eğride analitik faz ve
2M nokta sınırının ötesi açık; madde `[~]`.

**Viewport'un görünen şekline göre desen kaybı (6 Kasım 2026, [ADR 1007](../docs/adr/1007-line-type-loss-inside-the-viewport-shape.md)):**
Desteklenmeyen desen artık yalnız kırpma şekline (çokgen, dikdörtgen ya da döndürülmüş çerçeve) değiyorsa ön
denetimde hata verir ve teslimi engeller; ön denetim, vektör, raster ve PDF kitabı aynı zemini kullanır. Altı
geometrik durumla test edildi, çokgen sınaması kapatılınca test kırmızı verdi. Yığın kutusu sınaması tutucudur;
parça düzeyinde kesin kırpma açık.

**SHX unifont/Big Font, noktalı bant, eğride analitik tire (6 Kasım 2026, [ADR 1120](../docs/adr/1120-shx-unifont-and-big-font-linetype-text.md),
[ADR 1121](../docs/adr/1121-dots-on-a-wide-polyline.md), [ADR 1122](../docs/adr/1122-dash-phase-along-the-source-curve.md),
[ADR 1123](../docs/adr/1123-frozen-ellipse-outline-order.md)):**

- SHX okuyucusu AutoCAD-86 unifont 1.0 (iki baytlık alt şekil, tanım kaydı, gömme türü) ve bigfont 1.0'ı (kaçış
  aralıkları, konumlu dizin) okur; genişletilmiş Big Font'un `7,0` ilkeli açıkça reddedilir. Yazı stilinin fontu
  `.shx` olan çizgi tipi yazısı projeye gömülü kaynaktan çizilir, Big Font karakteri `\M+cXXXX` ile; eksik font,
  karakter ya da Big Font `linetype_unsupported`'tır. `TextStyleDef.bigFontFile` DXF STYLE grup 4'tür, `.kcad`
  şema 184 (geçici numara); gömülmesine izin vermeyen unifont kaynak olamaz, kullanılan font silinemez. Test fontları
  ve bağımsız okuyucu `scripts/fixtures/shx_font_reference.py`; Rust yolları bağımsız okuyucuyla bit bit aynı;
  22 yeni ortak yazı stili durumu; katalog, Python SDK'sı ve Python testi güncel.
- Geniş çoklu çizgide nokta artık bandın kesitidir (genişlik boyunda, çizgi kalınlığıyla); sıfır genişliğe düşen
  nokta açık ret. AutoCAD'in davranışı belgelenmemiş; bu KentOS kararıdır.
- CAD çizgi tipinin tiresi yay, daire, elips, yaylı yol ve NURBS'te kaynak eğrinin yay uzunluğuyla kesilir
  (`View::cad_phase`: masaüstü model/pafta ekranı ve çıktı; web ve dondurulmuş yanıtlar değişmez). Kesim noktası
  eğrinin kendi noktasıdır; 1:1/1:20/1:100 vektör çıktısında tire başları bağımsız ölçüyle 1e-6 mm içinde; GPU,
  CPU ve PDF resimlerinde tire başı fazı birikmiyor (yayılım ≈1 px). NURBS işaretleri de gerçek yay uzunluğuyla.
- `main`'de kırmızı olan iki style-core dondurulmuş testinin nedeni 31c296103'teki elips toplama sırasıydı; çizilen
  kontur dondurulmuş sırayla, çözücü kendi doğru yuvarlanmış uç noktasıyla (ADR 1123). Beklenenler yenilenmedi.
- Açık: PDF'de sıfır uzunluklu tire (nokta) `butt` uçla görünmüyor (düz çizgide de, önceden vardı); çizgi tipi
  dışındaki SHX yazıları; DXF/DWG aktarımında SHX yazı fontlarının dosya yanına yazılması; Unicode → Big Font kod
  sayfası; AutoCAD `A` hizalaması/PLINEGEN=0; 2M nokta sınırının ötesi; parça düzeyinde kesin kırpma; genel
  performans kabulü. Madde `[~]`.

### CAD-ADV-12 — Katman, stil, şablon ve araç paletini sürümlü ofis standardına bağla [~]

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

**Özel çizgi tipi denetimi (5 Kasım 2026):** Adlandırılmış katman çizgi tipi,
standart yalnız yerleşik çizgi tipini tarif ediyorsa artık sessizce uygun
sayılmaz. Denetim tanım adını ve beklenen yerleşik tipi gösterir; eşlemesi
bilinmediği için otomatik düzeltilebilir sayılmaz ve uygulama kaynak tanımı
korur. Komut testi kontrolün salt okunur olduğunu ve uygulama sonrası
bulgunun kaldığını doğrular. Gerçek çizim standardı penceresinde P_SHX bulgusu
ve açıklaması görüldü; ekran görüntüsü repo dışında. Sürümlü paket ve diğer
kabul koşulları açık olduğundan madde `[~]` kalır.

Standartta açıkça tanımlanmayan fakat adlandırma kuralına uyan ek katmanlardaki
özel çizgi tipleri ve nesnenin kendi özel çizgi tipi de artık denetlenir.
İki ayrı bulgu ve kaynak çizimin değişmemesi komut testiyle; A-DETAY ve bir
nesneye ait bulgular gerçek arayüzde doğrulandı. Bu, paket kapsamının tamamı
değildir.

Nesnenin adlandırılmış özel çizgi tipi, nesneye özel görünüşe izin veren
standartlarda da ayrı raporlanır; izin listesi boşsa bu denetim yapılmaz.
Komut sınamaları ve gerçek penceredeki P_TEXT bulgusu doğrulandı.
Blok tanımlarındaki özel ve izin dışı yerleşik çizgi tipleri de tanım başına
bir kez taranır; yerleştirme sayısıyla çarpılmaz. Kullanılmayan tanımlar da
rapora dahil edilir. Otomatik test ve gerçek penceredeki üç blok doğrulandı.
Katmana göre görünüş kuralı açıksa, blok parçalarındaki açık renk, çizgi tipi
ve kalınlık atamaları da tanım başına raporlanır; ByBlock değerleri değiştirilmez.
Parça sayısı ve beklenen görünüş komut testi ve gerçek pencerede doğrulandı.

Elle müdahale gerektiren bulgular standart penceresinde ilk sırada gösterilir;
satır seçimi rapordaki asıl bulguya bağlı kalır. Gerçek arayüzde ayrıntılar
ve otomatik düzeltmeden sonra kalan iki bulgu doğrulandı; görüntü repo dışındadır.
Rapor değişirse eski seçim temizlenir; böylece aynı sıra numarasına gelen
başka bir bulgu yanlışlıkla seçili gösterilmez. Bu durum gerçek uygulamada
eksik katman seçilip uygulandıktan sonra doğrulandı.

Standart uygulaması, ön izlemenin üretildiği belge revizyonunu doğrular.
Arada başka bir işlem çizimi değiştirirse eski ön izleme uygulanmaz; rapor
yenilenerek yeniden değerlendirilir. Çakışma otomatik testle, güncel raporun
normal uygulanması gerçek arayüzde doğrulandı.

Yazı ve ölçü stillerinde standartta belirtilen kâğıt yükseklikleri artık
denetlenir; farklar mm cinsinden ayrı bulgu olarak raporlanır ve çizimi
kendiliğinden değiştirmez. Komut sınamaları ile gerçek arayüzde 4 → 2,5 mm
yazı ve 3 → 2,5 mm ölçü yazısı farkları, uygulamadan sonra korunmalarıyla
birlikte doğrulandı. Bu çalışma sürümlü standart paketinin tamamı değildir.

Çizim standardı penceresi mevcut `cad.standards.check/apply` komutlarının
desteklediği özel JSON standart tanımını geçici olarak seçebilir. Dosya
boyutu ve tanımı denetlenir, bilinmeyen alanlar reddedilir; kaynak ve seçimin
çizime kaydedilmediği gösterilir. Gerçek arayüzde iki farklı tanımla rapor
değişimi, yerleşik standarda dönüş ve hatalı dosyanın reddi doğrulandı.
Bu seçim kalıcı veya sürümlü ofis paketi değildir; paket kapsamı açık kalır.

Kimlik ve ofis revizyonu taşıyan sürümlü standart tanımının ilk biçimi
eklendi (`docs/specs/drawing-standard-package.md`). Eski yalın standart
dosyaları okunur; bilinmeyen kurallar ve desteklenmeyen sürümler reddedilir.
Otomatik testler ve gerçek uygulamada dosya seçimiyle kimlik/revizyon gösterimi
doğrulandı. Bu ilk biçim yalnız mevcut standart kurallarını taşır; seçimi
projeye kaydetmez ve blok, tarama, şablon veya palet kaynaklarını paketlemez.
Dolayısıyla 12. maddenin kalıcı, kapsamlı ofis paketi kabulü açık kalır.
KCAD şema 165, çizimde sıralı standart belgelerini saklama altyapısını taşır;
bozuk veya yinelenen kimlikli belgeler reddedilir. Bağımsız Python okuyucusu,
eski şema sınaması ve 235 KCAD testi doğrulandı. Gerçek uygulamada örnek
çizim açılıp kaydedildi ve iki belgenin sırası/içeriği bağımsız okuyucuyla
karşılaştırıldı. `cad.settings.set` komutu sürümlü belgeler listesini
doğrulayıp revizyon kontrolüyle bağlar, sıralar veya kaldırır; Python
bağlamaları güncellendi. Python'un gerçek yerel motorunda bağlama, kaydetme,
yeniden açma ve kaldırma doğrulandı. Standart penceresi sürümlü bir dosyayı
projeye bağlayabilir ve bağlı belgelerden birini seçip denetleyebilir; ilk
belge pencere açıldığında seçilir. Gerçek arayüzde üç belgeyle bağlama,
kaydetme, yeniden açma, seçim ve küçük pencerede kaydırma doğrulandı.
`cad.standards.resolve` ile kullanıcı bağlı belgeleri birlikte denetleyebilir:
genel kurallar ilk belgeden, her adlandırılmış katman ve stil ilk görüldüğü
belgeden alınır. Birleştirme sorgusu salt okunurdur; uygulama ayrıca ve belge
revizyonu denetlenerek yapılır. Bağlı belgeler değişince eski birleşik ön izleme
uygulanamaz. İki kaynaklı gerçek arayüz testinde ikinci kaynağın A-EK katmanı
ön izlemede görüldü, uygulandı ve kayıt bağımsız okuyucuyla doğrulandı. Python
üzerinden çözümleme, denetim ve uygulama planı da sınandı. Blok ve çizgi tipi
tanımlarının paketlenmesi ve önceliği henüz desteklenmez.
Birleşik sorgu genel kural ile her katman, yazı ve ölçü stili tanımının
kaynak kimliğini ve ofis sürümünü de döndürür. Bulgu seçildiğinde ilgili
tanımın kaynağı pencerede gösterilir; bu bilgi nesne veya viewport görünüşünün
kaynağı değildir. Kaynak önceliği komut ve Python testleriyle, seçilen katmanın
kaynağı gerçek arayüzde doğrulandı.
`cad.standards.conflicts`, bağlı standartlardaki aynı adlı katman, yazı ve
ölçü stili tanımlarının farklı alanlarını ve iki kaynağı raporlar. Sıra
değişince ilk kaynak yeniden belirlenir; okunamayan belge varsa eksik rapor
üretilmez. Arayüz aynı sorguyu kullanır; genel kural farklılıkları da bildirilir.
Karşılaştırma sorgusu kendiliğinden çizimi değiştirmez. Gerçek arayüzde
genel kural ve üç adlandırılmış fark, sıra değişimi; komutta salt okunurluk ve
hatalı belge reddi sınandı. Python bağlamasıyla genel ve katman farkı sınandı.
Fark satırı seçilince iki kaynağın ilgili değerleri gösterilir; sıralama
değişirse eski seçimin ayrıntıları temizlenir. Bu davranış gerçek arayüzde,
renk ve genel kural değerleri ise komut ve Python testlerinde doğrulandı.
Çizgi kalınlığı ve yazı/ölçü yüksekliğindeki sayısal farklılıklar, standart
denetiminin kullandığı 1e-9 toleransıyla karşılaştırılır. Sınırın altı ve üstü
otomatik testte ve gerçek pencerede iki ayrı çizimle doğrulandı.
Bağlı standartlar pencereden sıralanabilir ve açık onayla projeden
çıkarılabilir; kaynak dosya ve çizim geometrisi değiştirilmez. Eski ön izleme
ile sıralama engellenir. Otomatik testler ve gerçek arayüzde sıralama,
çıkarma iptali, onay ve kaydedilen dosyanın bağımsız okunması doğrulandı.
Okunamayan bağlı standart uygulanmaz, uygunluk raporu yerine uyarı gösterilir
ve açık onayla çıkarılabilir. Bu kurtarma
akışı otomatik test ve gerçek arayüzde denendi; çizimin kaydedilen kopyasında
diğer standart, katman ve nesne bağımsız okuyucuyla doğrulandı. Birden çok
okunamayan belge de tek tek çıkarılabilir: mevcut sıralı belgelerin yalnız
değiştirilmemiş alt kümesine izin verilir. İki belgeyle gerçek arayüz ve
kaydedilen çizim, komutta ekleme ve sıralama reddi otomatik testle doğrulandı.
Standart, katman ve yazı/ölçü stili tanımlarındaki bilinmeyen alanlar doğrudan
komut girdisinde de reddedilir; arayüzde olduğu gibi Python/MCP çağrısı da
tanınmayan bir kuralı atlayıp uygunluk sonucu üretemez. Komut sözleşmesi ve
ilgili otomatik testler güncellendi.
JSON dosyasında aynı alanın birden fazla yazılması da reddedilir; aksi hâlde
son değerin sessizce seçilmesi farklı denetim sonuçlarına yol açabilirdi.
Paket, standart ve katman örnekleri otomatik testte; uyarı ve uygulamanın
devre dışı kalması gerçek dosya seçicisinde doğrulandı.
CAD özellik paneli, seçilen nesnenin temel model görünüşünde renk, çizgi tipi,
kalınlık, saydamlık ve çizgi tipi ölçeğinin etkin değerini ve nesne/katman/
varsayılan kökenini salt okunur bölümde gösterir. Adlandırılmış nesne çizgi
tipinin kökeni de nesne olarak hesaplanır. Nokta ve çizgi otomatik sınamaları,
gerçek uygulamada noktanın seçilip bölümün açılması ve ekran görüntüsüyle
doğrulandı (`/tmp/kentos-cad-adv12-model-appearance.png`; repo dışında).
Bu yalnız modelin temel görünüşünü açıklar; blok içi parça, viewport, plot
profili ve stillerin görsel etkisi için tam köken/override analizi açıktır.
Sürüm 2 standart belgesi, çizgi tipi ve tarama deseninin adlandırılmış
tanımlarını `resources` içinde taşıyabilir. Dosya, komut ve KCAD sınırında
biçim/sürüm/alan/doğruluk denetimi vardır; V1'e kaynak eklenmesi veya V2'de
kaynak bölümünün olmaması reddedilir. `cad.standards.resolve` ilk kaynak
tanımını ve kökenini verir, `cad.standards.conflicts` aynı adlı farklı
tanımların alanlarını ve iki değerini raporlar. Bağlama, kaydetme, yeniden
okuma, sıra değiştirme ve salt okunur sorgular komut sınamasında doğrulandı;
masaüstü derlemesi geçti. İki sürüm 2 paketi gerçek uygulamada bağlanıp
çizgi tipi ile tarama tanımı farkları standart penceresinde görüldü; ekran
görüntüsü repo dışında `/tmp/kentos-cad-adv12-v2-conflicts.png` konumunda.
Kaynak tanımları çizime otomatik uygulanmaz.
`cad.standards.check`, sürüm 2 paketindeki çizgi tipi ve tarama tanımlarını
çizimin kaynak tablolarıyla artık karşılaştırır. Eksik veya geometrisi farklı
tanımlar ayrı, salt okunur bulgulardır; açıklama/kaynak meta verisi görünüş
farkı sayılmaz. `plan` ve `execute` bu tanımları kendiliğinden kurmaz veya
değiştirmez. Komut ve Python sözleşmeleri güncellendi; eksik, farklı ve eşit
tanımlar ile Undo otomatik sınandı. Gerçek standart penceresinde iki eksik
kaynak bulgusu, kısa liste metni, seçilen satırın ayrıntılı tanımı ve pasif
Uygula düğmesi doğrulandı (`/tmp/kentos-cad-adv12-resource-check.png`, repo
dışında). Kaynakların güvenli eşleme/kurma akışı hâlâ açıktır.
Sürüm 2 paketi yayımlanmış kitaplık bloklarının adını, aile/tip kimliğini ve
yayın numarasını da `resources.blocks` içinde taşıyabilir; geometri pakete
kopyalanmaz. Bağlı standartlar ilk kaynak önceliğiyle bu referansları çözer,
aynı adlı fakat farklı yayınları iki kaynağıyla bildirir. Denetim eksik bloğu
veya farklı yayın kimliği/sürümünü salt okunur bulguya çevirir; blok tanımı ve
yerleştirmeler değiştirilmez. Paket doğrulaması, komut/Python/MCP sözleşmesi,
eşit/eksik/farklı ve çakışan kaynak senaryoları otomatik sınandı. Gerçek CAD
penceresinde eksik kitaplık bloğu ve pasif Uygula görüldü; uzun kimlik metninin
taşması aynı testte saptanıp okunur, satıra sığan ayrıntıyla düzeltildi
(`/tmp/kentos-cad-adv12-block-detail-after.png`, repo dışında). Yayın meta
verisinin eşitliği geometri eşitliğinin kanıtı değildir; blokların güvenli
eşlenmesi ve içerik denetimi hâlâ açıktır.
Blok yayın uyuşmazlığının açıklaması artık doğrudan yerleştirme ile blok
tanımındaki iç başvuru sayılarını ayrı verir; `objects` alanı doğrudan
yerleştirmeyi taşır. Üç yerleştirmeli komut sınaması ve tek yerleştirmeli gerçek
CAD penceresinde seçili bulgunun tam metni doğrulandı. Bu yalnız salt okunur
etki bilgisidir; blok tanımını veya yerleştirmeleri değiştirmez.
Ofis blok referansı isteğe bağlı çizim birimi de taşıyabilir. Beklenen ve yerel
birim ayrışırsa, yayın bilgisi aynı olsa dahi ayrı ve otomatik düzeltilemeyen
`blockUnit` bulgusu çıkar; birim veya ölçek sessizce değiştirilmez. Paketler
arasındaki birim farkı da kaynaklarıyla raporlanır. Eski paketlerde birim alanı
yoksa bu denetim yapılmaz. Paket, Rust komutu ve Python SDK sınamaları
geçti. Gerçek CAD penceresinde `m → mm` bulgusu, bir doğrudan yerleştirme,
tam ayrıntı ve pasif Uygula düğmesi ekran görüntüsüyle doğrulandı; görüntü
repo dışında tutuldu.
Pafta/araç paleti ve diğer stil kaynakları, kullanıcı eşlemesi ve
tam görünüş kökeni ile kabul senaryosu hâlâ açıktır.
Sürüm 2 paketine `sheetTemplates` altında onaylı pafta şablonu kimliği ve
revizyonu eklendi. Birleşik sorgu ilk kaynak önceliğini ve kökeni, çakışma
sorgusu aynı kimliğin farklı revizyonlarını bildirir. Denetim paftanın
kaydedilmiş `origin` alanını onaylı sürümle salt okunur karşılaştırır; elle
oluşturulan paftayı yeniden üretmez ve aynı içerik olduğunu iddia etmez.
Komut sınamasında farklı/eşit revizyon, kaynak önceliği, çakışma ve çizimin
değişmemesi doğrulandı. Gerçek uygulamada şablon sürümü 1 olan pafta,
ofis sürümü 2 karşısında ayrı bulgu olarak göründü; satır seçiminin açıklaması
ve pasif Uygula düğmesi ekran görüntüsüyle doğrulandı (repo dışında).
Şablon içeriğinin
paketlenmesi/eşlenmesi ve palet akışı açık kaldığı için madde `[~]` kalır.
Sürüm 2 paketine adlandırılmış kılavuz stilleri de eklendi. Çözümleme ilk
kaynağı ve ofis revizyonunu, çakışma raporu aynı adlı farklı görünüş alanlarını
gösterir; yerel stil kimliği farklı olsa da aynı ad ve görünüş uygun sayılır.
Eksik veya farklı kılavuz stili denetimi salt okunurdur; mevcut liderleri
sessizce değiştirmez. Paket/komut ve Python sınamaları geçti. Gerçek uygulamada
eksik A-NOT stili bulgusu, satır seçimi ve pasif Uygula düğmesi ekran görüntüsüyle
doğrulandı (repo dışında).
Sürüm 2 paketi yazı ve ölçü stillerinin kısa standart kurallarından ayrı,
tam çizim tanımlarını da taşıyıp ilk kaynağı, revizyonu ve aynı adlı farklı
görünüş alanlarını raporlar. Yerel kimlikler eşleşmek zorunda değildir; adı ve
görünüşü eşit olan stil uygundur. Eksik/farklı tanımlar otomatik uygulanmaz;
özellikle mevcut yazı ve ölçüler sessizce değiştirilmez. Paket, komut ve Python
sınamaları geçti. Gerçek arayüzde A-NOT ve A-DIM eksik tanım bulguları,
satırların ayrıntısı ve pasif Uygula düğmesi görüldü. Ölçü stili JSON ayrıntısı
tek satırda kesildiği için görünüm satır kaydırmalı yapıldı; gerçek uygulamada
aşağı kaydırılarak tanımın tamamının okunabildiği doğrulandı. Görüntüler repo
dışındadır. Tablo stili,
güvenli kaynak eşlemesi,
palet ve tam görünüş kökeni açık olduğundan madde
`[~]` kalır.
Doğrudan `cad.standards.check` girdisi ile komut normalizasyonu da paketle
aynı kaynak alanı denetimini çalıştırır; bilinmeyen yazı/ölçü stili alanları
Python/MCP yolunda sessizce kaybolmaz. Üç komut aşamasında çizimin ve Undo
geçmişinin değişmediği otomatik sınandı. Eski KCAD çizimlerindeki toleranslı
stil okuma korunur; bu değişiklik arayüz davranışını değiştirmez.
Çizgi tipi ve tarama kaynağı bulgularında kaynak adının katman, doğrudan nesne
ve blok tanımı içindeki kullanım sayıları ayrı gösterilir. Tarama nesnesi desen
geometrisini kendi içinde sakladığı için ad eşleşmesi yalnız inceleme ipucudur;
otomatik kaynak değiştirme yetkisi vermez. Kapsam tek çizim taramasıyla
hesaplanır, `objects` yalnız doğrudan kullanım sayısını taşır. Katman, nesne ve
blok parçası içeren komut sınamaları çizim ile Undo geçmişinin değişmediğini
doğruladı. Gerçek uygulamada GAZ çizgi tipinin farklı tanımı seçildiğinde
1 katman, 0 doğrudan nesne ve 0 blok parçası açıklaması ile pasif Uygula
düğmesi ekran görüntüsünde doğrulandı; görüntü repo dışında tutuldu.
Kayıtlı katman durumundaki adlandırılmış çizgi tipi de etki sayısına eklendi;
katman durumunun geri yüklenmesinde bu kaynak kullanılabilir. KESIK-İNCE
örneğinde bir kayıtlı durum, bir doğrudan nesne ve sıfır blok parçası otomatik
sınamada doğrulandı. Aynı sayılar gerçek uygulamanın kaynak ayrıntısında
görüldü; Uygula düğmesi pasif kaldı ve ekran görüntüsü repo dışında tutuldu.
Çizgi tipi ve tarama deseninin iç içe sayısal alanları, paketler arası çakışma
ve çizim denetiminde aynı mutlak `1e-9` toleransını kullanır. Eşiğin altında
gereksiz bulgu çıkmaması, üstünde hem çakışma hem yerel tanım farkının çıkması
komut sınamasında doğrulandı; küçük farklar çizim kaynaklarını değiştirmez.
Gerçek uygulamada `0,5` ve `0,5000000005` tanımlarıyla yalnız katman adı
bulguları göründü; yanlış çizgi tipi tanımı bulgusu çıkmadı. Ekran görüntüsü
repo dışında tutuldu.
Yazı, ölçü ve kılavuz stilinin farklı yerel tanımlarında doğrudan nesne ile
blok parçası kullanım sayısı ayrı gösterilir; yazı stili ayrıca aynı adı
kullanan çizgi tipi tanımlarını sayar. Bir çizgi tipindeki tekrar eden yazı
elemanları tek tanım olarak sayılır. Yerel stil kimliğiyle sayım ve salt okunur
denetim komut sınamasından geçti. Gerçek çizimde Yol adı için bir doğrudan
yazı ve bir blok parçası, Mimari için bir doğrudan ölçü görüldü; iki seçimin
ayrıntısı ve pasif Uygula düğmesi ekran görüntüleriyle doğrulandı (repo dışında).
Bu etki bilgisi stil eşleme veya değiştirme işlemi değildir; 12. madde `[~]`
kalır.
Ofis paketindeki çok sayıda blok için doğrudan ve iç içe yerleştirme etkisi
artık çizim başına tek taramada hesaplanır. İki tanımlı blok sınaması doğrudan
1/2 ve iç başvuru 0/1 sayılarını, çizim ile Undo geçmişinin korunmasını
doğruladı. Gerçek CAD penceresinde iki bulgu seçilip aynı sayılar, satır
taşması olmadan ve pasif Uygula düğmesiyle ekran görüntülerinde doğrulandı
(repo dışında). Bu performans/rapor işi kaynak eşleme kabulünü kapatmaz.
Mevcut Şablonlar paneli kişisel araç paleti olarak genişletildi: şablonun
sağ menüsünden favoriye ekleme/çıkarma, listenin başında Favoriler grubu ve
satır yıldızı; şerit menüsünde de aynı grup vardır. Son kullanılan beş şablon
ve favoriler kullanıcı ayar klasöründeki `sablon-paleti.json` dosyasında atomik
olarak saklanır; çizim ve ofis standart paketi değişmez. Kaybolan proje
şablonu o proje yeniden açılana kadar listede gizlenir. Arama, uygulama ve
klavye akışı korunur. Panel sınamaları ve dosyadan yeniden okuma geçti;
gerçek CAD penceresinde sağ menü, yıldız ve yeni uygulama oturumunda favorinin
geri gelmesi ekran görüntüleriyle doğrulandı (repo dışında). Bu, tüm komutları
kapsayan genel araç tarifi ve ofis paleti yayını değildir; madde `[~]` kalır.
Kişisel favori/geçmiş anahtarları kitaplık kaynağına, proje şablonunda ayrıca
kalıcı proje kimliğine bağlandı. Aynı şablon ID'si başka çizimde geçse bile
yanlış favori olarak görünmez; eski, kaynak belirtmeyen proje favorisi
güvenle gizlenir. Farklı proje kimliği sınaması eklendi. Gerçek uygulamada
favori satırı ve `project:<kimlik>:<şablon>` kaydı doğrulandı; ekran görüntüsü
repo dışındadır.

**Kaynakların kullanıcı eşlemesiyle kurulması (5 Kasım 2026, [ADR 1002](../docs/adr/1002-installing-office-standard-resources.md)):**
`cad.standards.install` bağlı standartların çizgi tipi, tarama, yazı/ölçü stili tanımı ve kılavuz stillerini
çizimdekilerle karşılaştırır (eksik/aynı/farklı, farklı alanlarıyla) ve seçime göre kurar: eksik eklenir, farklı olan
seçim olmadan korunur; Değiştir çizimdeki ad ve kimliği koruyarak yerinde değiştirir, Yeni adla standarttakini başka
adla ekler. Hiçbir kaynak silinmez; tek Undo “Standart kaynaklarını kur” (çizgi tipi/tarama tabloları için yeni
geri alma işlemi). Çizim standardı penceresinde **Kaynakları kur…**. Bağımsız başvuruyla 25 ortak durum ilk koşuda
geçti; gerçek pencerede önce/sonra görüntülendi. Blok/pafta şablonu kurulumu, tablo stili, ofis paleti yayını, tam
görünüş kökeni ve iki profilli kabul senaryosu açık; madde `[~]`.

**Blok kitaplığı, onaylı pafta şablonu, değerin kökeni ve iki ofis profili (6 Kasım 2026, [ADR 1100](../docs/adr/1100-office-standard-blocks-templates-and-value-origin.md)):**
`cad.standards.install` artık standardın blok başvurularını yayın kimliğiyle (aile, tip, yayın, isteğe bağlı birim)
okunabilen kitaplıklardan çözer (bugün `kentos-temel` ve yayın kimlikli yeni `kentos-ofis`; proje/bulut kitaplığı
`Libraries::with_blocks` ile önüne takılır) ve diğer kaynaklarla aynı eksik/aynı/farklı durumu ve
ekle/koru/değiştir/yeni adla seçimini verir. Değiştir çizimdeki ad ve kimliği korur, model ve tanım içindeki
yerleştirmeler kimlikle izler; hiçbir tanım silinmez. Hiçbir kitaplığın tutmadığı başvuru `unavailable` bulgusudur
(yakın girdinin farkı: `release`, `units`), sessizce atlanmaz. Onaylı pafta şablonu sürümü çizime kaydedilir
(`approvedSheetTemplates`, KCAD şema 183, dalda geçici numara); var olan paftalar değişmez, satır kaç paftanın o
şablondan yapıldığını söyler. Kurulum tek Undo adımıdır. Öznitelikler panelinde **Bu değer nereden geliyor?** renk,
çizgi tipi, kalınlık ve yazı stilinin nesne/katman/blok/görünüm penceresi kökenini, bağlı standardın aynı ya da farklı
tanımını ve sıfırlamanın kapsamını (bu nesne, katmandaki kendi değerli nesneler, bütün pencereler) yazmadan gösterir.
Bağımsız başvurudan 24 ortak durum ilk koşuda geçti; iki ofis profili kabul testi (aynı adlı farklı blok ve çizgi
tipi, önizleme = sonuç, nesne rengi ve iki pencerenin farklı geçersiz kılması korunur, tek Undo, yeni ve yeniden
açılmış projede aynı sonuç) ile MCP testi geçti; gerçek pencereler görüntülendi (repo dışında). Yayımlanan ofis araç
paleti bu dilimde yok: paket alanı, komut kaydına bağlı araç tarifi ve bağlılık kararı gerekir (ADR 1100 §4); tablo
stili, blok içi parça ile plot profilinin görünüş kökeni de açık olduğundan madde `[~]` kalır.

**Ofis araç paleti (6 Kasım 2026, [ADR 1014](../docs/adr/1014-office-tool-palettes-published-with-the-standard.md)):**
Sürüm 2 standart paketi `resources.toolPalettes` ile komut kaydına bağlı araç tarifleri yayımlar (komut, sürüm, tam
girdi); palet çizime kurulmaz, bağlı paketten okunur. `cad.standards.palettes` her aracın favori anahtarını, tarif
özetini ve bu çizimde çalışıp çalışmadığını komutun kendi denetimiyle verir; masaüstü Şablonlar'da **Ofis paletleri**
hazır aracı tek adımda çalıştırır. 23 bağımsız vaka geçti. Açık: ofis aracını kişisel favoriye alma, tablo stili
kaynağı, blok içi parçanın kökeni, masaüstünden proje/bulut kitaplığı seçimi.

### CAD-ADV-13 — Çizim verisinden güvenilir tablo ve metraj üret [x]

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

**Blok özniteliği tutarlılığı (5 Kasım 2026):** Çizime bağlı blok tablosunun
gruplama ve ek sütunları artık blok sorgusuyla aynı etkin öznitelik kuralını
kullanır: sabit tanım değeri, örneğin açık değeri ve tanımın varsayılanı.
Blokta tanımlanmayan ek veri alanı örnekteki kaydından okunmaya devam eder.
Üç yerleştirmeli sınamada iki varsayılan/standart ve bir özel değer doğru
gruplandı; toplam 3, sabit `KAPI` sütunu her iki grupta görüldü. Gerçek CAD
penceresinde tablonun görünümü, seçimi ve canlı yenileme düğmesi doğrulandı;
ekran görüntüleri repo dışında tutuldu. Genel çıkarım tarifi, iç içe blok
kapsamı, anahtarlı dış eşleme ve çok sayfalı çıktı kabulü açık kalır.

**Bağlı tablonun satır kayması koruması (5 Kasım 2026):** Dış dosyadan gelen
bir hücre elle düzeltilmişse ve eski kaynak satırının değişmeyen verileri
yeni dosyada başka bir sırada bulunuyorsa, satır indeksine göre yenileme artık
düzeltmeyi yanlış kaleme taşımaz. Uyarı verir; tablo, kaynak özeti ve Undo
geçmişi olduğu gibi kalır. Aynı satırdaki fiyatın değişmesi yanlış kayma
sayılmaz. Komut sınamasında `A/11, B/10` ile `B/10, A/10` dosya sırası;
gerçek CAD penceresinde yenileme uyarısı, çizimin değişmemesi ve komut
geçmişindeki "Çizimde değişiklik yok" kaydı doğrulandı. Bu yalnız
belirlenebilen satır kaymalarını korur; kalıcı satır anahtarıyla eşleme ve
uyuşmazlık çözümü hâlâ açık kabul işidir.

Ek sınamada elle düzeltme içeren satırın silinmesi veya tekil ilk veri alanının
değişmesi de güvenli olmayan eşleme sayıldı. Tekil `ID` aynı kalırken kaynak
açıklamasının değişmesi güncellemeyi gereksiz yere engellemez. Tekil alan
frekansları yalnız gerekirse ve tek geçişte çıkarılır; çok satırlı tabloda
her satır için tüm kaynak tekrar taranmaz.

**Aynı hücre çatışması:** Dosyaya bağlı tabloda `Keep` kuralı seçiliyken hem
kullanıcı hem kaynak dosya son okunan kopyaya göre aynı hücreyi değiştirmişse
yenileme artık kullanıcı değerini sessizce silmez. Uyarıyla tablo ve kaynak
özeti korunur; açıkça `Replace` seçimi dosya değerinin üzerine yazılmasını
sağlar. Komut sınaması değerleri, rapor durumunu ve Undo geçmişini kontrol
eder. Gerçek CAD penceresinde `A/11` tablosunun dosyadaki `A/12` değişimine
rağmen korunduğu, uyarı ve "Çizimde değişiklik yok" kaydı ekran görüntüsüyle
doğrulandı; görüntü repo dışındadır. Bu koruma satır anahtarıyla eşleme ve hücre bazında uyuşmazlık çözümü
yerine geçmez; madde `[~]` kalır.

**Dış tablo satır anahtarı (5 Kasım 2026):** Dosyaya bağlı tabloda başlıklı,
benzersiz ve boş olmayan bir sütun satır anahtarı olarak seçilebilir. Bu tercih
komut/şema/KCAD dosyasında saklanır. Yenileme, kaynak satırları yer değiştirse
de elle değiştirilen hücreyi aynı anahtarlı satıra taşır; yeni kaynak sırasını
korur. Çakışan hücre değişimi, yinelenen/boş anahtar veya elle düzeltilmiş
satırın kaynaktan silinmesi tabloyu ve Undo kaydını değiştirmeden uyarı verir.
Farklı veri satırı yükseklikleri de satır anahtarıyla birlikte taşınır; gerçek
CAD penceresinde `B/10` kısa, `A/11` uzun satırın yenileme sonrası da kendi
yüksekliğini koruduğu görüldü.
Tek veri satırına bağlı dolgu/renk/hizalama stilleri de aynı anahtarlı satıra
taşınır; başlık ve tüm tablo stili kapsamı korunur. Gerçek CAD penceresinde
`B/10` mavi, `A/11` kırmızı dolgusunu sıra değişiminden sonra korudu. Bu
ekran kontrolü koyu dolguda tablo düzenleyicisinin varsayılan siyah yazısının
okunmadığını da gösterdi; varsayılan yazı rengi arka planın karşıtlığına göre
seçilecek biçimde düzeltildi. Çok satıra yayılan birleşim veya yeniden
sıralamayla parçalanan formül aralığı varsa yenileme durdurulur.
Gerçek CAD arayüzünde KentOS seçim bileşeninden `ID` anahtarı seçildi; `A/11,
B/10` tablosu `B/10, A/11` sırasına yenilendi ve elle düzeltilen bir hücre
korundu. Ekran görüntüleri repo dışındadır. Genel çıkarım tarifi, iç içe blok
sayımı, parçalanan formül aralıkları/çok satırlı birleşimlerin taşınması ve çok paftalı çıktı açık kaldığından
madde `[~]` durumundadır.

**Çakışma kuralı arayüzü (5 Kasım 2026):** Dosyaya bağlı tablonun
düzenleyicisinde `Koru` veya `Dosyayla değiştir` seçimi artık görünürdür ve
komutla aynı kaynağa kaydedilir. Seçim değiştirildiğinde uyarı metni elle
yazılan değerlerin sonraki yenilemede silineceğini açıklar. Komut sınaması
varsayılan korumada aynı hücre çatışmasını reddetti ve açık `Replace`
seçiminden sonra güncellemeyi kabul etti. Gerçek CAD penceresinde `A/11`
değeriyle açılan tablo için `Dosyayla değiştir` seçilip kaydedildi; `Tabloyu
güncelle` sonrasında kaynak dosyadaki `A/12` görüntülendi. Ekran görüntüleri
repo dışında tutuldu.

**Blok adları tekrarlandığında sayım (5 Kasım 2026):** Aynı blok adı bir
çıkarım isteğinde birden çok kez verilirse blok artık iki kez sayılmaz;
çizelgenin ek sütun düzeni de tek kez verilen adla aynı kalır. Yerel sınama
satırları ve toplamı karşılaştırır. İç içe blok, xref ve dizi kapsamı için
açık sayım seçimi ile izlenebilir örnek kimlikleri hâlâ gereklidir.

**Çok satırlı biçimlerin taşınması (5 Kasım 2026):** Birden fazla veri
satırına uygulanmış hücre biçimi, anahtarlı satırlar araya yeni satır girerek
ayrıldığında aynı biçim önceliğini koruyan aralıklara bölünür. Bütün tablo
veya bütün veri satırları biçimi kapsamını korur; biçimli satır kaybolursa
ya da biçim sınırı aşılırsa yenileme durur. Yerel sınama `A/B` biçimli,
`C` biçimsiz kaynağı `A/C/B` sırasına taşıdı. Gerçek CAD penceresinde
yenileme sonrası `A/11` ve `B/10` dolgusu kaldı, `C/30` biçimsiz kaldı;
ekran görüntüsü repo dışında tutuldu.

**Tek satırlı birleşimlerin taşınması (5 Kasım 2026):** Başlıkta duran veya
tek anahtarlı veri satırının içinde kalan birleşim, kaynak satırları yer
değiştirince konumunu satırla birlikte günceller. Veri satırları arasında
yayılan birleşim hâlâ güvenli biçimde reddedilir. Yerel sınama `A` satırının
birleşimini 1. veri satırından 2. veri satırına taşıdı. Gerçek CAD
penceresinde `A / First 10` birleşik satırı `B / Second / 20` satırının
altına taşındı; hücre sınırı ve yazı korundu. Görüntü repo dışındadır.

**Alan hücrelerinin taşınması (5 Kasım 2026):** Proje veya nesne kimliğine
bağlanan field hücresi, anahtarlı dış kaynak satırı yeniden sıralandığında
aynı kaydın yeni satırına taşınır. Alanın kaydı kaynaktan silindiyse yenileme
reddedilir. Yerel sınamada `A` satırındaki field adresi 1'den 2'ye taşındı;
gerçek CAD penceresinde `B/20, A/Dinamik bloklar` sırası ve başarılı yenileme
görüldü. Ekran görüntüsü repo dışında tutuldu. Parçalanan formül aralıkları ve
birden çok satıra yayılan birleşimler için güvenli taşıma hâlâ açıktır.

**Formül başvurularının taşınması (5 Kasım 2026):** Formüllerin hem kendi
hücresi hem de başvurdukları kaynak satırı, yenilemede satır anahtarlarıyla
yeniden eşlenir. Aralık başvurusu yalnız kaynak satırların tamamı yeni sırada
kesintisiz kaldığında taşınır. Silinen satıra başvuru veya parçalanan aralık
yenilemeyi çizimi değiştirmeden durdurur. Yerel
sınamada `$B$3` başvurusu `$B$2` oldu; gerçek CAD penceresinde `A/10/20,
B/20/40` tablosu `B/20/40, A/10/20` sırasına yenilendi. Ekran görüntüsü
repo dışında tutuldu. Aralıkların bitişik ve parçalanmış durumları ayrıca
yerel testlerle denetlendi.

**Blok adetlerinin dışa aktarımı (5 Kasım 2026):** `file.export.table`,
blok çizelgesi kaynağındaki adetleri çizimden yeniden sayar ve son sütunu
çalışma kitabına sayı türünde yazar. Önbellekteki yanlış `999` hücreleri
varken yerel sınama dışa aktarılan toplamı `3` buldu. Kaynak çizim salt okunur
kalır. Birim taşıyan genel metraj alanları ve ortak çıkarım tarifi açık kalır.

**Seçili tabloyu dışa aktarma (5 Kasım 2026):** CAD tablosunun bağlamsal
sekmesine ve Açıklama sekmesindeki Tablo menüsüne `Tabloyu dışa aktar`
eklendi. Seçili tablonun kalıcı kimliği, seçilen XLSX/ODS/CSV yolu ile
`file.export.table` komutuna aktarılır; masaüstü, Python ve MCP aynı
sayısal çalışma kitabı üretimini kullanır. Kaynak tablosu seçilmediğinde
işlem kullanıcıyı uyarır. Gerçek CAD penceresinde üç bloklu çizelge
seçilip XLSX olarak kaydedildi; komut geçmişi 3 satır/4 sütun yazıldığını
gösterdi ve bağımsız XLSX okuması `Adet` sütununu sayı türünde `2, 1, 3`
olarak doğruladı. Ekran görüntüsü repo dışındadır. Genel metraj alanları ve ortak çıkarım tarifi
açık kaldığından madde `[~]` durumundadır.

**Veri çıkarımı tarifi ve parçalı tablo (5 Kasım 2026, [ADR 1030](../docs/adr/1030-data-extraction-recipes-and-table-parts.md)):**
Yeniden kullanılabilir veri çıkarımı tarifi (`cad.extract.run/table`, `.kxtarif`). Kapsam ve sayım: açık çizim ile
salt okunur başka çizimler; iç içe blok, dinamik varyant, dizi, görünmeyen parça ve dış referans için açık sayım
kapsamı; her oluşum izlenebilir yoluyla bir kez sayılır. Değerler: birimli miktarlar (adet, uzunluk, alan, gerçek
ölçü, katsayılı hesaplanan sütun); ölçü yazısı toplanmaz; toplam yuvarlamadan önce alınır. Dış tablo anahtarla eşlenir;
Tabloyu güncelle elle düzeltmeleri, kullanıcı sütunlarını ve formülleri anahtarla korur. Tablo: çok satırlı hücre,
sığmayan hücre bildirimi, başlığı yinelenen parçalara bölme (`.kcad` 182). Bağımsız başvurular
`extract_command_cases.py` (78 durum) ve `table_parts_cases.py`. Kabul: iki çizimden iç içe blok ve dizili parça
listesi (31 adet), override'lı ölçü toplanmaz, dış açıklama tablosu anahtarla yenilenir, 43/128 m'lik üç çizgi
1,01. Sınır: pafta kitabında tablonun başka paftaya devamı CAD-ADV-16'ya; masaüstü tablo düzenleyicide satır sonu
yazma, görünürlük dışı dinamik eylemle gizlenen parça ve büyük çizimde önizleme ölçümü açık. Madde `[~]`.

**Satır sonu, görünürlüğün gizlediği parça ve büyük çizimde önizleme (6 Kasım 2026, [ADR 1131](../docs/adr/1131-table-line-breaks-hidden-dynamic-pieces-and-extraction-preview-scale.md)):**
Tabloyu düzenle'nin çubuğu çok satırlı bir yazı alanıdır: Enter hücreyi yazar, **Shift+Enter** ya da AutoCAD'deki gibi
**Alt+Enter** hücrede yeni satır açar; ızgara satırları `↵` ile gösterir, Kaydet tek Undo, kısayol listesinde de. Dinamik
bloğun görünürlük durumu parçayı artık kopyalandığı parçanın kimliğiyle gizler (`Dynamic::vary_split`): dizi eyleminin
gizli parçadan yaptığı kopyalar çizicide görünmez, çıkarımda sayılmaz ve listelenmez; `notes.hiddenStates` ve uyarının
sözü kaçının görünürlük durumundan, kaçının gizli katmandan sayılmadığını söyler; açıkça seçilince eylemlerin koyduğu
yerde sayılırlar. Bağımsız başvuru dizi eylemini ve kaynağın görünürlüğünü yeniden yazdı (Kat 3: Sandalye 3 / 7, Sıra
2 / 3; `cad.extract.run` 33, `cad.extract.table` 47 durum geçti). Çıkarım `Watch` ile ilerlemesini söyler ve 256 nesnede
bir durdurmaya bakar; pencere 20 000 nesneden büyük çizimde önizlemeyi okuma kopyası üzerinde ayrı iş parçacığında yapar,
“n / N nesne” ve **Durdur** gösterir, eski cevabı numarasıyla ayıklar. Yayın derlemesinde (Apple M5) 100 000 nesnelik
önizleme ortanca **122 ms**, yerleşik bellek tepesi **+81 MB**, 20 ms'de istenen durdurma **5,5 ms**'de cevaplandı.
Kabulün satırları: iki çizimden iç içe blok ve dizili parça listesi, gerçek ölçü ve override, anahtarlı dış tablo ve
yenileme, tekil sayım, birimler, yuvarlamadan önce toplam ve kullanıcı düzeltmeleri ADR 1030'un bağımsız durumlarında;
çok sayfalı çıktı tablo parçalarıyla (ADR 1030 §7, `table_parts_cases.py`) ve parçaların ayrı paftalara detay olarak
dağılmasıyla (ADR 1090 §3, ADR 1112'nin 24 `table` durumu) kanıtlı. Sınırlar ADR 1030 §9 ve ADR 1131'de (parçaların elle
konumu, web). Madde `[x]`.

### CAD-ADV-14 — Dış referans ve altlık bağımlılıklarını kaybolmadan yönet [x]

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

**Aynı adlı yanlış kaynağa bağlanmayı önleme (5 Kasım 2026):** Dış
başvurunun saklı mutlak yolu kaybolduğunda, proje göreli yolunda veya
arama klasörlerinde bulunan adayların SHA-256 özeti son yüklenen
kaynakla karşılaştırılır. Aynı adlı başka çizim otomatik kaynak olmaz;
başvuru eksik görünür ve aday ile `Yolu değiştir…` eylemi gösterilir.
İlk yanlış adaydan sonra sonraki klasördeki doğru dosya da aranır.
Kullanıcının açıkça bağladığı mutlak dosya yerinde değişirse mevcut
`Değişti`/`Yeniden yükle` akışı korunur. `cad.xrefs.list`, yeniden
yükleme ve `cad.assets.check` aynı bulma kuralını kullanır. Kaynak
grafiği, değişim etkisi önizlemesi, paket ve çoklu pafta kabulü açık.
Gerçek CAD penceresinde ilk arama klasöründeki aynı adlı farklı dosya
`Bulunamadı` ve açık aday uyarısı olarak göründü; `Yeniden yükle`
doğru olmayan dosyayı almadı. Doğru dosya ikinci arama klasörüne
konunca `Yüklü · arama yolundan bulundu` durumu görüntülendi. İki
durumun ekran görüntüleri repo dışında tutuldu.

**Ağ, kimlik, önizleme ve paket (5 Kasım 2026, [ADR 1060](../docs/adr/1060-xref-network-review-identity-and-package.md)):**
`cad.xrefs.list` `review` ile kaynak ağını tek görünümde verir: kullanılan revizyon, derinlik (iç içe zincir artık
düzleşmez), çizimin gösterdiği (kaynak / son yüklenen kopya), getirdikleri, katmanlarını adlandıran katman durumları
ve taşınan kaynağın adayları (önce aynı içerik, sonra aynı ad; kendiliğinden bağlanmaz). Yeniden yükle ve Yolu
değiştir önce `diff` ile önizlenir: parça farkı ve bağlı ölçü/kılavuz/tarama/alan izler mi kopar mı (`ties_break`).
Kaynakta silinen parçanın bağı aynı sıradaki nesneye geçmez (`piece_gone`); kimliği bilinmeyen kaynak nesneleri
(PDF, v1, tutamaksız DXF) içerikle eşlenir. `resetLayers` çizim override'ını kaynağa döndürür, katman durumlarına
dokunmaz. Bulma kuralı liste, paket ve “Kaynağı aç”ta aynıdır, yedekler özetle doğrulanır; başka dizinde açılan
paket iç içe kaynaklarını `veri/` içinde bulur. PDF ölçü uyarısı kâğıt–çizim katsayısını söyler. Kabul
`headless/tests/headless/xref_network.rs`'te kanıtlı; masaüstü ağ ve önizleme penceresi 1100×650/1440×900'de
görüntülendi. Açık: pafta nesnesinin kendi katman override'ı (KentOS'ta katman durumu ve görünüş tarifi), paketin
stil/desen ayrıntı raporu. Madde `[~]`.

**Pafta penceresinin kendi ezmesi ve paketin kaynak raporu (6 Kasım 2026, [ADR 1110](../docs/adr/1110-viewport-own-overrides-of-xref-layers.md),
[ADR 1111](../docs/adr/1111-package-resource-report.md)):** Pafta penceresinin dış başvuru katmanı için kendi ezmesi
ADR 0314'ün harita `layerOverrides`'ıdır (dondur, yazdır, renk, çizgi tipi, desen, kalınlık; katman kimliğiyle): ADR
1060 §12'deki “veri modeli yok” sözü düzeltildi. Yeniden yükleme, Yolu değiştir, Boşalt ve Katmanları kaynağa döndür
katman kimliğini korur ve pafta kitabına yazmaz; ezme yerinde kalır, çizimin kendi ezmesinden ayrıdır. `cad.xrefs.list`
`review` her başvurunun `viewportOverrides`'ını (pafta, pencere, katmanın ağaçtaki yolu, ezmenin alanları) verir;
“Bu değer nereden geliyor?” pencerenin dondurduğu katmanı söyler ve dış başvurunun yerleştirmesinde “Dış başvuru
katmanları” parçası çizimin ve pencerelerin düzeyini, iki sıfırlamanın erimini gösterir. Pafta tasarımcısının katman
listesi katmanları yollarıyla verir (dış başvurunun Duvar'ı çizimin Duvar'ından ayrılır). Paketle cevabında ve
`paket.json`'da `resources`: her yazı stili yüzü, SHX şekil dosyası, çizgi tipi, tarama deseni ve baskı profili,
`builtIn`/`embedded`/`carried`/`substituted`/`missing` ve kullananlarıyla; dış başvurunun kaynağında tanımlı ama
çizimde olmayan çizgi tipi ve yazı stili `carried`, hiçbir yerde olmayan `missing` olarak `package_resources`
uyarısıdır. Kanıt: `xref_command_cases.py --check` (`cad.xrefs.list` 46, yeni 9: dondurma, alanlar, desen, çizimin
katmanı ve olmayan kimlik, grup ve iki paftanın sırası, `review`'siz, plan, Katmanları kaynağa döndür ve Boşalt'tan
sonra), `package_resource_cases.py --check` (`file.package.resources.json` 24 bağımsız durum, başsız koşucuda 24/24);
kabul testi `xref_network.rs`'te iki paftanın iki ezmesi (biri dondurur, biri başka renk ve kalınlık) Yolu değiştir,
Katmanları kaynağa döndür ve paketin başka klasörde açılmasından sonra yerinde ve incelemede, değerin kaynağı dondurmayı
söyler, paket `resources`'ı verir. Masaüstünde Dış başvurular ayrıntısı, Denetçi ve Paketle iletisi 1440×900 ve
1100×650'de, açık ve koyu görüntülendi. Kabulün bütün satırları (iç içe üç referans, aynı adlı iki dosya, PDF altlığı,
taşı/değiştir/yeniden bağla, paketin başka dizinde açılması, kırpma, iki paftadaki override'lar, tek Undo) kanıtlı.
Sınır: Bağla › Ekle'de pencere ezmesi kendiliğinden birleşen katmana taşınmaz (ADR 1110 §7). Madde `[x]`.

### CAD-ADV-15 — Revizyon farkını semantik karşılaştırma ve 2B inceleme notlarına bağla [x]

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

**Uygulama durumu (2026-11-05):** Blok tanımı değiştiğinde yerleştirme
UUID'si ve konumu aynı kalsa da `cad.drawing.compare` artık etkilenen bütün
yerleştirmeleri `blockDefinition` farkıyla bildiriyor. İç içe blok tanımları
da parmak izine katılıyor. Masaüstü bölünmüş revizyon görünümü, tanım
değişikliklerinde yalnız nesne slotlarını izlemek yerine tam karşılaştırma
yapıyor; küçük blok örneğindeki üç yerleştirme için hem komut hem canlı
izleyici regresyonu geçti. Güncel masaüstü derlemesinde değişmiş çizim
ile temel KCAD dosyası bölünmüş görünümde açıldı; `0 eklendi · 0 silindi ·
3 değişti` sayacı ve sahnedeki fark vurgusu gerçek ekran görüntüsüyle
doğrulandı. Yalnız katman rengi değişimi de `layerStyle` alanı ve
`attributes` durumu olarak ayrıldı; bölünmüş görünümde mavi/açık kahverengi
çizimler ve `3 değişti` sayacı gerçek uygulamada doğrulandı. Ölçü yazısı
konumu ve yuvarlama override'ı `dimensionPresentation`, ölçü referansları
`dimensionAssociation` olarak ayrıldı; yalnız sunum değişimi yanlışlıkla
geometri farkı sayılmıyor. Gerçek uygulamada ölçü yazısı kaydırılmış dosya
temel çizimle karşılaştırıldı; bölünmüş görünümde tek ölçü vurgulandı ve
`0 eklendi · 0 silindi · 1 değişti` görüldü. Tarama bağı değişimi
`hatchAssociation` alanı olarak raporlanıyor; yalnız bağı çözmek geometri
değişikliği sayılmıyor, gerçek sınır değişikliği geometri olarak kalıyor.
Altı karşılaştırma regresyonu geçti; gerçek bölünmüş görünümde yalnız
bağı çözülmüş tarama vurgulandı ve `1 değişti` görüldü. Bu test tek KentOS
süreciyle yapıldı. Pafta/görünüş semantiği `projectChanges` listesine eklendi:
viewport ölçeği/oranı, kırpma, kilit, katman kapsamı, çerçeve ve baskı ayarları;
pafta sayfası/anteti/öğe sırası; kayıtlı görünüş ve şablon; kitap ayarları
model nesnelerinden ayrı karşılaştırılıyor. 14 native regresyonu, 8 masaüstü
bölünmüş görünüm testi ve 3 gerçek Python API testi geçti. Python testinde
Undo/Redo ve salt okunurluk doğrulandı. Gerçek uygulamada yalnız viewport
ölçeği farklı iki dosyada model sayacı `0 değişti`, ayrı sayaç `Pafta /
görünüş: 1 fark` gösterdi. API ve kapsam ayrıntıları
[revizyon karşılaştırma belgesinde](../docs/cad/revision-comparison.md).
Genel `clippy -D warnings` kapısı, bu teslimin dışındaki mevcut
`standards.rs`, `table_export.rs`, `registry.rs` ve `tables_edit.rs`
uyarıları nedeniyle geçmedi; başarılı kontrol olarak sayılmadı.
Farklı UUID'ler için açık `matchSearch` ile kaynak kimliği özniteliği veya
geometri yakınlığına dayalı `matchCandidates` önerileri eklendi. Öneriler
eklenen/silinen sayısını ve kimlikleri değiştirmiyor; çıktı sınırında
gizlenen alternatifler de belirsizlik hesabına katılıyor. Aday kapsamı
her tarafta 5000 nesne, hesap iki milyon iş birimiyle sınırlı; native Job
iptali var. Masaüstünde KentOS UI penceresi, seçim bileşeni ve tabloyla
1/10/100 mm arama sunuluyor. Okuma kopyasında çalışan tek arka plan işçisi
belge değişince veya revizyon kapanınca iptal ediliyor; önceki iş gerçekten
sonlanmadan başka işçi başlayamıyor. Gerçek fare testinde 10 mm'de
`2 öneri · 2 belirsiz`, 1 mm'de `1 öneri · 0 belirsiz` görüldü. 21 native,
3 işçi yaşam döngüsü, 8 bölünmüş görünüm ve 5 Python API testi geçti;
ortak geometri çekirdeğinin bağımsız karşılaştırma örnekleri de geçti.
Python'da kaynak kimliğiyle uzakta taşınmış nesne önerisi, salt okunurluk,
liste sınırındaki belirsizlik ve Job sonucu doğrulandı. GUI testi tek
KentOS süreciyle yapıldı ve test süreci kapatıldı. Görüntüler Git'e eklenmedi.

**Kapanış — [ADR 1000](../docs/adr/1000-review-notes-and-taking-revision-differences.md) (5 Kasım 2026):**
İnceleme notları `.kcad` şema 180'de (`settings.reviewNotes`) tasarımdan ayrı saklanır; `cad.review.notes.edit`
(ekle/güncelle/yanıtla/yeniden bağla/sil, tek Undo “İnceleme notu”) ve `cad.review.notes.list` nesne damgasıyla
`current/changed/blockChanged/removed` eskimesini verir; kilitli katmandaki nesneye not bağlanabilir, AI/PDF notu
tasarımı değiştirmez. `cad.drawing.take` seçilen farkları kalıcı kimlikle alır (ekle/değiştir/sil, hepsi ya da
hiçbiri, tek Undo “Farkları al”); kilit, eksik katman/blok, ortak tabanla iki taraflı değişim (`take_conflict`,
açık `overwrite`) ve blok tanımı/katman görünüşü uyarıları vardır. Bağımsız `review_cases.py` başvurusuyla 76 ortak
durum (28 düzenleme, 20 liste, 28 alma), 4 Rust testi (blok tanımı eskimesi ve uyarısı, eksik tanım, sınırlar,
kaydet–aç/Undo), 236 KCAD testi, bağımsız `tools/kcad/kcad.py` okuyucusuyla 1581 dosya ve 7 Python yerel motor testi
geçti. Masaüstünde bölünmüş revizyonun **Farkları al…** penceresi ve Açıklama › İşaretleme › **İnceleme notları**
penceresi 1440×900 ve 1100×650'de, iki temada görüntülendi; görüntüler repo dışında. Sınırlar ADR 1000 §8'de:
alt nesne bağı, özel durum modeli, PDF yorumu içe alma, blok tanımı/pafta farkı alma ve web karşılığı bu maddenin
dışında. Sahnelerle sürülen pencere testinin dışında ayrı bir fare kaydı yapılmadı.

### CAD-ADV-16 — Genel 2B pafta ve yayın işini tek güvenilir üretim akışına bağla [~]

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

**Detay seti, sayfa özeti, yayın durumu ve devamı (6 Kasım 2026, [ADR 1090](../docs/adr/1090-general-2d-sheet-set-and-publication-flow.md)):**
`cad.sheets.details` aynı modelin kesin oranlı genel 2B detaylarını kilitli görünümlerle, detay başına katman
durumu, ezme ve açıklama temsiliyle, mimari kat verisi olmadan pafta setine yerleştirir; tablo parçası da detay
kaynağıdır (ADR 1030 §9). `cad.sheets.set` `recipes` ile tarifi geri verir; etki hesabı görünümün katman durumunu ve
tabloyu okuyan nesneleri sayar. PDF sayfa denetimi çizgi, dolgu, sıfır kalınlık, aranabilir metin, yüz, kırpma,
saydamlık ve resim görünümü sayar; bağlamsız sayfa özeti yayın kaydına yazılır. `file.publish.status` güncel ve eski
paftaları ayırır; `resumable`/`resume` durdurulan yayını sürdürür. 1:1/1:5/1:20/1:100 seti, iki farklı temsilli
viewport, bağımsız PDF okuyucu ve Poppler denetimi, eski pafta ve durdurulan yayın kabul testleriyle kanıtlı; 32
bağımsız ortak vaka geçti. Açık: masaüstünde detay kurma penceresi, `table` nedeni için bağımsız vakalar, ikili
CTB/STB, AutoCAD/Acrobat denetimi. Madde `[~]`.

**Detay paftaları penceresi ve tablo nedeninin ortak durumları (6 Kasım 2026, [ADR 1112](../docs/adr/1112-detail-set-window-and-table-reason-cases.md)):**
Masaüstünde Çıktı › Pafta › **Detay paftaları…** (`sheet.details`) `RoadDialog` iskeletinde: detay listesi (ad, kaynak,
kesin oran, sil) kitin veri tablosunda; **Seçimden ekle** seçimin kutusunu uzun kenarının %5'i paylı alır, **Tablo
parçası ekle** bölünmüş tablonun her parçasını ayrı detay yapar; seçili detayda tablo parçası, katman durumu, yalnız o
pencerede dondurulan katmanlar, açıklama temsili ve kilit; set için ilk numara ve başlık. Sağda `cad.sheets.details`'in
planı yazmadan izlenir: paftaların kâğıt önizlemesi (görüntüleyicide), Pafta/Detay/Oran kartları, ret notu ve detay ·
pafta · oran · kâğıt boyu tablosu. **Paftaları kur** seti çizimin bir grubunda çalıştırır: pafta kitabının tek
değişikliği, tek Undo; Esc/Kapat yazmaz. `cad.sheets.set`'in `table` nedeni için `sheet_set_table_cases.py` ADR 1090
§6'dan KentOS kodu olmadan 24 durum yazdı (koordinat, alan, adsız/adlı blok çizelgesi, satır üyeli çizelge, hücre alanı;
pencerenin dondurduğu ve gizli katmandaki tablo; kenara değen ve 1 cm ötesindeki tablo; tablonun kendisi verilince;
silinen tablo/çizelge; sonradan verilen dondurma; retler; kitapsız proje); native koşucu ilk çalıştırmada 24/24.
Masaüstü testleri (önizleme yazmaz, 1:5 detayın kâğıt eni model uzunluğundan, dondurma ve kilit pencerede, tek Undo,
Kapat yazmaz) geçti; pencere 1440×900 ve 1100×650'de açık ve koyu görüntülendi, incelemede oranların sırası ve dar
pencerede tablonun sütunları düzeltildi. Açık: ikili CTB/STB, AutoCAD/Acrobat ile karşı denetim. Madde `[~]`.

**İkili CTB/STB baskı stili tabloları (6 Kasım 2026, [ADR 1130](../docs/adr/1130-autocad-plot-style-tables-ctb-stb.md)):**
`cad.plotstyles.import` AutoCAD'in renge bağlı `.ctb`'sini (255 kalem, ACI 1–255) ve adlı `.stb`'sini pafta kitabına baskı
profili olarak alır; dosya yalnız okunur, profil biçimini, SHA-256'sını ve adını taşır (`PlotProfile.source`). Üst bilgi
(`PIAFILEVERSION_2.0,CTBVER1|STBVER1,compress`, `pmzlibcodec`, üç küçük-sonlu u32) ve zlib metni çekirdeğin okuyucusunda
(`kentos_sheet::plot_profile::acad`); her alan açıkça eşlenir: RGB/ACI renk, gri tonlama, titreşimle tarama, kalınlık
tablosu (µm), sürekli çizgi, uç ve birleşim aynen; ByLayer/ByBlock rengi, `mode_color`, kalem numaraları, 1–30 baskı
çizgi tipleri, elmas uç/birleşim, dolgu desenleri, açıklama, değişik Normal, bilinmeyen alan ve ölçek çarpanı türü,
ağırlığı (lost/approximate/note), stilleri ve dosyadaki değerleriyle raporlanır; sessiz kırpma yok. Kalem katmanın
renginden tahmin edilmez: `assignments` katmana ya da kâğıt öğesine açıkça verir, `sheets` paftalara uygular; yayın
mevcut profil yoluyla basar. Masaüstünde Çıktı › Pafta › **Baskı stili tablosu…** (`sheet.plotStyles`) planı yazmadan
önizler (kalemler, kayıp satırları, katmana kalem, paftalar); İçe aktar tek Undo. Bağımsız `plot_style_cases.py` 17
tabloyu belgelenen biçimden kendisi yazdı ve 37 ortak durumu kendi okuyucusundan çıkardı (native 37/37); MCP sınamasında
CTB'nin 0,50 mm siyah 1. kalemi katmana verilip paftaya uygulanınca PDF'teki çizgi 0,50 mm siyah. Doğrulanamadığı için
CTB/STB **yazıcısı yok**; üst bilginin sağlaması ve tablo dışı kalınlık sırası AutoCAD'le karşılaştırılmadı (rapor
satırı olarak söylenir); UTF-8 dışı kod sayfası reddedilir. Açık: AutoCAD/Acrobat ile karşı denetim, gerçek CTB
korpusu, baskı çizgi tiplerinin eşlenmesi. Madde `[~]`.

### CAD-ADV-17 — Dosya alışverişinde görüntü, geometri ve düzenlenebilirliği ayrı güvenceye al [~]

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

**Kesin geometri, açık düzleme indirme, kaynağın korunması (5 Kasım 2026, [ADR 1061](../docs/adr/1061-exchange-geometry-elevation-and-source-guard.md)):**
Blok patlatılarak okunurken eşit olmayan ölçek altında düz genişlikli kenarlar şeridini kesin korur; açık, düz,
genişliksiz yaylı yol kesin 2. derece ağırlıklı NURBS olur. `cad.entities.flatten` (2B düzleme indir) kotları ve
isteğe bağlı ölçüleri önizlemeli, tek Undo ile kaldırır; açılışta Z/M silinmez. Dışa aktarma KentOS'un yazmadığı
DXF/DWG'nin üzerine yalnız `replaceSource` ile yazar (`source_overwrite`). ezdxf'in yazdığı 2B kümeyle
aç–düzenle–dışa aktar–yeniden oku kanıtlı; ezdxf dışa aktarmayı bağımsız okur. Destek matrisi
`docs/cad/exchange-support.md`'de görüntü ve düzenlenebilirlik ayrı. Açık: KentOS içi Patlat'ta eşit olmayan yay,
kapalı/kotlu yaylı yol, birim/OCS/blok/katman/kâğıt kararlarının tek yapılı rapor bölümü; AutoCAD denetimi yapılmadı
(CAD-IO-20). Madde `[~]`.

**Patlat'ta eşit olmayan yay (6 Kasım 2026, [ADR 1008](../docs/adr/1008-exploding-a-nonuniform-insert-keeps-arcs-exact.md)):**
ADR 1061'in kesin NURBS ve kesin genişlik kuralları ortak çekirdeğe taşındı; DXF okuyucusu ve Patlat aynı kodu
kullanır. Eşit olmayan ölçekli bloğun açık yaylı yolu Patlat'ta kesin NURBS, genişlikli düz yolu kenarına dik ölçülen
genişlikler olur. Bağımsız (merkez/açıyla hesaplanan) ortak vaka ve masaüstü resmi var. Açık: kapalı/genişlikli/
kotlu yaylı yol, AutoCAD denetimi. Patlat NURBS'e dönen yolları `semantics_lost` uyarısıyla sayar (ADR 1008 eki).

**Okuyucu kararlarının tek bölümü (6 Kasım 2026, [ADR 1010](../docs/adr/1010-one-section-for-the-readers-decisions.md)):**
DXF/DWG içe alma raporunda `decisions` bölümü birim, düzlem (OCS), blok, katman ve kâğıt kararlarını sabit kodlarla
konu konu verir; eski notlar aynı sözlerle kalır, yeni kararlar (nesnesiz/tabloda olmayan/durumu korunan katman, ters
ya da eğik düzlem, boş Layout1, inç pafta) yalnız bu bölümdedir. ezdxf'in yazdığı dosya ve kuruluşundan hesaplanan
beklentiyle sınandı; Çizim al penceresi “Okuma kararları”nı gösterir. Açık: kapalı/genişlikli/kotlu yaylı yol, AutoCAD
denetimi.

### CAD-ADV-18 — Çizim sağlığını kontrollü onarım ve kanıtlı kurtarmayla tamamla [~]

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

**Sağlık adayları ve T birleşimi onarımı (5 Kasım 2026, [ADR 1005](../docs/adr/1005-drawing-health-candidates-and-junction-repair.md)):**
`cad.drawing.inspect` yalnız adıyla istenen `health` grubunu kazandı: T birleşiminde kısa kalma/taşma, küçük açı
sapması, uzakta başıboş nesne, aynı içerikli blok tanımı; varsayılan denetimler değişmedi. `cad.drawing.repair`
çizgi ucunu eşik içinde kenara getirir, ölçülmüş/kısıtlı ya da kilitli geometriye dokunmaz, kaymayı ve boy farkını
raporlar, tek Undo. CAD denetimi penceresinde Çizim sağlığı ve Ucu kenara getir. Bağımsız başvuruyla 45 ortak durum.
Açılışta bütçeli salt okunur denetim, onarım tarifinin kaydı, kurtarma senaryoları ve kısmi okuma açık; madde `[~]`.

**Gerçek süreç ölümünde kayıt (5 Kasım 2026, [ADR 1006](../docs/adr/1006-save-survives-a-killed-process.md)):**
Test kendini alt süreç olarak başlatıp büyük, paftalı bir çizimi art arda kaydederken on farklı anda `SIGKILL` ile
öldürülür; her seferinde adın altındaki dosya özgün dosya ya da tam yazılmış bir sürümle bayt bayt aynıdır, açılır ve
paftalarını korur. Yazmayı reddeden klasörde çizim kaydedilmemiş kalır, önceki dosya ve Undo zinciri sağlamdır.
Gerçek ENOSPC, ayrı onarım kopyası ve kısmi okuma açık.

**Açılışta sağlık denetimi (6 Kasım 2026, [ADR 1009](../docs/adr/1009-health-check-on-opening.md)):** İsteğe bağlı,
varsayılanı kapalı `appearance.openHealthCheck` ayarıyla CAD çizimi açılınca sağlık adayları arka planda, salt okunur
ve en çok 2 saniye aranır; süre dolarsa sonuç kısmi olduğunu söyler. Aynı dosya aynı hâliyle yeniden açılınca önceki
sonuç kullanılır. Sonuç CAD denetimi sekmesine, tek satır Komut geçmişine gider. Ayrı bellek sınırı, oturumlar arası
önbellek, onarım tarifi kaydı, ayrı kurtarma kopyası ve kısmi okuma açık; madde `[~]`.

**Onar'da açı ve blok, tarif kaydı (6 Kasım 2026, [ADR 1012](../docs/adr/1012-repair-angles-merge-blocks-and-the-recipe.md)):**
`cad.drawing.repair` eksenden en çok `maxAngle` sapan çizgiyi pivot ucu çevresinde boyunu koruyarak eksene döndürür
ve aynı içerikli blok tanımlarını birleştirir (yerleştirmeler tutulan tanıma bağlanır, kopyalar silinir); cevap
uygulanan tarifi sırası, eşikleri ve kapsamıyla verir. CAD denetiminde **Eksene döndür** ve **Tek tanımda birleştir**.
41 bağımsız vaka ilk koşuda geçti. Açık: ayrı kurtarma kopyası, kısmi okuma, çoklu çizgi kenarı; madde `[~]`.

**Bozuk çizimin ayrı kopyaya kurtarılması (6 Kasım 2026, [ADR 1013](../docs/adr/1013-recovering-a-damaged-kcad-into-a-separate-copy.md)):**
`file.recover` ve Dosya › **Kurtar**: katı okuyucunun reddettiği `.kcad` (özeti tutmayan, kesik, bozuk nesneli) ayrı
bir okuyucuyla açılır; kurtarılan her parça katı okuyucudan geçer, çıkarılan bölüm ve nesne okuyucunun sözüyle ve
kalıcı kimliğiyle listelenir, kesikte kaybolan ayarlar boş çizimin değeriyle konur ve söylenir. Kopya
`<ad>-kurtarilan.kcad` olarak yazılır, kaynak yalnız okunur. 22 bağımsız vaka ve yapıdan beklentili testler geçti;
ilk koşu kesikten sonra sahte nesne okuyan bir hatayı gösterdi, düzeltildi. Açık: blok tanımı düzeyinde bölme, gerçek
disk dolu senaryosu; madde `[~]`.

### CAD-ADV-19 — Büyük 2B çizimde akıcılığı ve görsel kararlılığı ölçerek koru [~]

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

**Ölçüm düzeni ve kaydırmada yazı kararlılığı (6 Kasım 2026, [ADR 1062](../docs/adr/1062-large-2d-drawing-fluency-and-stability.md)):**
Sabit karma veri seti (`perf::mixed`, 10⁴/10⁵/10⁶), ölçüm düzeni (`perf::large`) ve piksel/kenet kararlılık ölçütü
(`perf::stability`) eklendi; Apple M5/Metal release ölçümleri `docs/perf/large-desktop-2026-11-06.md`'de. Uzak
görünümde kaydırmada büyütülen yazıların değişmesi düzeltildi (çakışma hücreleri dünyaya bağlı; görünür fark
22 318 → 0 piksel). Yakın görünüm, tuş yankısı, iptal ve tek düzenleme kapsamı hedefte. Tutmayan: uzak görünüm GPU'su
(10⁶'da p50 ~1,8 s; bloklar ve 72 kirişli stilli eğriler), uzak tekerlekte MİB (~35 ms), 10⁶'da ilk kare ve hepsini
seç (~2,3–2,5 s), 2M kontur noktası için instancing. Madde `[~]`.

**Blok örnekleme ve eğrilerin uzak düzeyleri (6 Kasım 2026, [ADR 1063](../docs/adr/1063-instanced-blocks-and-curve-detail-levels.md)):**
Önce ölçüldü: uzak karenin GPU'su bellek bant genişliğinde değil, alt piksel kirişlerin aynı piksellere karışmasında
(aynı 152 MB'ta 4× → 1× örnekleme 171 → 31 ms). Blok yerleşimleri benzerlikle yerleşebildiklerinde tanımdan bir kez
kurulup örnekle çizilir (düzgün olmayan, kırpılmış, kesikli/desenli, tematik olanlar eskisi gibi; açık/kapalı görünür
fark 0); daire, yay ve elips uzak görünüm için 0,25 px toleranslı seyrek kiriş düzeyleri taşır (1:950'den yakında eski
72 kiriş, fark 0). 10⁵ uzak kaydırma GPU p50 221 → 45 ms, 10⁶ 1 772 → 409 ms; blok katmanının belleği 152 → 3 MB,
10⁶'da görünümün tamponları 2,08 → 0,71 GB. **Tutmayan:** 10⁵'te 16,7 ms (kalan payın yarısı uzun çoklu çizgiler), uzakta
küçük dairelerin mürekkep yoğunluğu değişti (eski 72 kiriş onları koyulaştırıyordu; sahibinin kararı), 10⁴'te imleç
karesinin GPU'su 2,5 → 4–7 ms (açıklanamadı). Madde `[~]`.

### CAD-ADV-20 — Her 2B yeteneği Python, MCP, AI ve eklenti üzerinden aynı doğrulukla sun [~]

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
- Yalnız metadata değiştiren işler de gerçek Undo adımını ve okudukları
  nesne/tanım damgalarını raporlasın; plan kimliği yalnız kök geometri
  yazıldığı varsayımına dayanmasın (ADR0873). Komut günlüğünün yeniden
  oynatılmasında `AssocArray` kimliği → yeni dizi kimliği eşlemesi ayrıca
  tamamlanmalı: mevcut `native/application/src/replay.rs` kök nesne ve
  otomatik ölçü grubu kimliklerini eşliyor; dizi kurma/benimsetme ardından
  `cad.arrays.edit` içeren günlük kabulü bu maddenin açık işidir.
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

**Günlükte dizi kimliği (5 Kasım 2026, [ADR 1003](../docs/adr/1003-journal-replay-maps-associative-arrays.md)):**
Komut satırı ve özeti ilişkili dizi kimliklerini (`arrays`) ayrıca taşır; yeniden oynatma kurulan dizinin kimliğini
kendi yaptığına eşler ve sonraki `cad.arrays.edit` doğru diziye uygulanır (JSON'a yazılıp okunan günlükle test).
ADR 0873'ün bu maddeye bıraktığı açık kapandı. Maddenin GUI/Python/MCP eşdeğerlik kabulü, eklenti ve AI sağlayıcı
kapsamı açık; madde `[ ]` → başladı.

**Konaklar arası kabul (5 Kasım 2026, [ADR 1004](../docs/adr/1004-parametric-detail-acceptance-across-hosts.md)):**
KentOS kodu kullanmayan `acceptance_detail.py` parametreli bağlantı plakası senaryosunu yazar; Python ve MCP aynı
dosyayı oynatır ve ilk koşuda aynı sonuçları verir: kısıtlarla dikdörtgen, ilişkili ölçü, parametreyle genişlik 10 ve
`1200cm`, Undo, eski revizyon reti, kilitli adım yüzünden atomik ret, istek tekrarının bir kez uygulanması ve
kaydet–aç. Masaüstünde aynı adımlar Parametreler penceresiyle görüntülendi. Pafta/iptal bu senaryoda değil (oda
senaryosu ve protokol testlerinde); eklenti ve AI sağlayıcı kabulü açık; madde `[~]`.

**Tarama, pafta, çöken betik ve iptal (6 Kasım 2026, [ADR 1011](../docs/adr/1011-host-acceptance-hatch-sheet-crash-and-cancel.md)):**
Aynı senaryo Python ve MCP'de ilişkili taramayı (alanı her adımda w × h), yarıda çöken betiği (Python'da grup
istisnayla geri alınır; MCP'de aynı adımlar tek değişiklik kümesi olarak reddedilir), bakılmadan durdurulan işi ve 1:50
detay paftasını da oynatır; ilk koşuda iki konak aynı sonucu verdi. Açık: eklenti ve AI sağlayıcı kabulü.
Bağımlılık (6 Kasım 2026, kodla doğrulandı): `.kek` paket biçimi, eklenti katkı kaydı ve uv ortamı henüz yok;
[plan/python.md](python.md) PY-KEK-01–17'nin hepsi açık. Kullanıcının seçtiği AI sağlayıcısı da TODOS.md §15'in işi.
Bu iki madde orada teslim edilmeden CAD-ADV-20'nin eklenti/AI satırı kanıtlanamaz; madde `[~]` kalır.

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
