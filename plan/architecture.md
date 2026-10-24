# 2B mimari üretim — araştırma, araçlar ve uygulama planı

Tarih: 24 Ekim 2026. Başlangıç: `main` / `bb1f3ffd0`.
Çalışma dalı: `codex/architecture-2d-workflows`.

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

## 2. Güncel envanter ve ilk bulgular

`catalog/v1/ribbon.json` Mimari sekmesi sekiz panel içeriyor:

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
  oluşturma ayrı bir araç olarak henüz yok. Normal alan çizmek bu ihtiyacı karşılamaz.
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

Araştırma genişletilirken donatı/mutfak, merdiven gösterimi, açıklık referansı ve
geçici ölçü için üreticinin ilgili araç sayfası maddeye eklenir. Tanıtım videosu
veya ekran görüntüsü veri modeli, undo ya da dosya desteği kanıtı sayılmaz.

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

### ARCH-UX-01 — Görünür mimari araç ayarları [ ]

- Etkin araca ait stil, kalınlık, referans yüz, açıklık tipi/en/konum, mahal
  yöntem/ad/numara alanlarını çizimden önce görülebilir yap.
- Seçili nesnenin özellikleri ile yeni nesnenin araç ayarları açıkça ayrı başlıkta
  olsun. Mevcut seçili nesne paneli ve komut satırı kaybolmasın.
- Mevcut seçim/parametre doğrulama yolu kullanılsın. Sayı yazarken her tuş belgeye
  işlenmesin; Enter/odak kaybı uygulasın, Escape geri dönsün.
- Kabul: parametre adı/harfi ezberlenmeden 250 mm duvar, 900 mm kapı ve net mahal
  seçilebilir; büyük yazıda alanlar çakışmaz; klavye akışı korunur.

### ARCH-UX-02 — Mimari şerit ve bağlam [ ]

- Çiz / Yerleştir / Düzenle / Belgele / Proje sırasını kullan; tek düğmeye yığılmış
  menüler yerine sık işlere yazılı erişim ver. Tüm komutları aynı anda sergileme.
- Seçili duvarda böl/birleştir/uzat/buda/stil; açıklıkta yön/menteşe/taşı/en;
  mahalde ad/numara/alan/etiket ve çizelge ilgili bağlamsal sekmede bulunsun.
- Taşıma, kopyalama ve genel ölçü komutlarını ikinci kez yazma. Arama, F1 ve
  menüler aynı katalog kimliğini kullansın.
- Kabul: 1440×900 ve 1100×650, 13 ve 18 px, açık/koyu; kesilen düğme, anlamsız
  ikon yığını veya erişilemez komut yok. Kullanım adımı sayısı tabanla karşılaştırılır.

### ARCH-UX-03 — Güvenli mimari başlangıç/şablon [ ]

- m/1:100 mimari başlangıcı ile cm/1:50 eski ofis şablonu farkını gideren açık
  uygulama tasarımı yap. Mevcut şablon sözleşmesi incelenmeden sessiz değişiklik yapma.
- Şablonun ekleyeceği katman/stil/sembol/pafta ve değiştireceği ayarlar önce gösterilsin.
- Var olan projede birim/ölçek korunması varsayılan olsun; alternatif ayar uygulanacaksa
  kullanıcı somut farkı görsün. Geometri yeniden ölçeklenmesin.
- Kabul: boş/dolu CAD, gizli/kilitli katman, aynı şablonu iki kez uygulama,
  mevcut kat/stil kimliği çakışması, Python/MCP ve kaydet/aç.

## 6. P1 — hızlı ve ölçülü plan çizimi

### ARCH-DRAW-01 — Dikdörtgen duvar [ ] — ilk uygulama dilimi

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

### ARCH-DRAW-02 — Ölçülü bölme duvar [ ]

- Bir mevcut duvar yüzünden net uzaklıkla paralel bölme; başlangıç/bitişi seçilen
  sınırlarla önizlemeli bulma. İç açıklık ile eksen aralığı ayrı seçenek olsun.
- Yakın duvarı otomatik benimseme kullanıcıya gösterilsin; büyük görünmez snap
  toleransı veya kaynak duvarı sessiz silme olmasın.
- Uca kadar, iki duvar arasında, tek yön ve iki yön seçenekleri; düz ilk dilim,
  yaylı kaynak ayrı kabul.
- Kabul: 3 m net oda, farklı duvar kalınlığı, T birleşimi, kapılı host, kilitli
  sınır, birden çok aday, bölme taşıma ve bağlı mahal/ölçü güncellemesi.

### ARCH-DRAW-03 — Oda/alan konturundan duvar [ ]

- Mevcut Çizgiden duvar komutunu kullanarak kapalı konturu duvara dönüştür;
  iç/dış/eksen referansı ve kaynak koru/sil önizlemede açık olsun.
- Seçilen birden çok oda konturunun ortak duvarını iki kez üretmeme ayrı iş;
  eşleme toleransı ve çakışma raporu açık, işlem atomik.
- Kabul: L oda, iç avlu, ortak kenar, ters halka, delik, eşleşmeyen parça,
  farklı kat ve aynı çizgiden tekrar üretim.

### ARCH-OPEN-01 — Kapı/pencereyi hassas yerleştirme [ ]

- Mevcut köşeden mesafe ve ortala seçeneklerini görünür kıl; referans başlangıç/
  bitiş/merkez/menteşe tarafı açık seçilsin.
- Önizlemede yakın köşeye mesafe, en, açılma yönü ve host vurgusu göster.
- İki noktayla açıklık eni seçimi ve bir açıklıktan tipi/eni alma yeni yöntemleri;
  eski serbest yerleştirme korunur.
- Kabul: düz/yaylı/ters duvar; aynalı kapı; uçta taşma; çakışma; host değiştirme;
  tutamaç/sayısal en değişikliği; duvar kalınlığı/bölme/undo sonrası bağ korunumu.

### ARCH-OPEN-02 — Açıklık dağıt / çoğalt [ ]

- Aynı duvarda adet + net ara mesafe veya toplam aralığa eşit dağıtma; uç payı
  ve merkez aralığı ayrı seçenek. Her açıklık ayrı host ilişkisi taşır.
- Kapı numaraları için başlangıç/adım/önek; yinelenen numara denetimi.
- Kabul: açık/yaylı duvar, köşe aşımı, mevcut açıklıkla çakışma, 1 ve çok adet,
  bir başarısızlıkta kısmi yazmama, tek undo ve çizelge güncellemesi.

### ARCH-SPACE-01 — Toplu mahal bul ve tanı [ ]

- Seçili kat/bölge veya kapalı duvar ağında mahalleri bul; mevcut mahalı yineleme.
- Kapalı olmayan sınırı sahnede vurgula; kapı geçişi, kolon/delik ve açık plan
  sanal sınırı için açık kural sun. Net/eksen/brüt sınır önizlemesi olsun.
- Kabul: iki oda, açık koridor, kolon, iç avlu, çakışan seed, farklı kat;
  kaynak duvar değişince sınır, alan, etiket ve tablo birlikte değişir.

### ARCH-SPACE-02 — Mahal kimliği, etiket ve numaralandırma [ ]

- Tıkla sırala veya sıra yönüyle numarala; kat öneki/ad/kullanım listesi;
  yinelenen ve boş numaraları göster. Kimlik numaradan ayrı kalsın.
- Etiket şablonu: ad, numara, net alan, döşeme/tavan kotu; sürüklenebilir yer,
  manuel metin kopyasına dönüşmeden kaynak güncellemesi.
- Kabul: etiket taşıma sonrası duvar değişikliği; numara değişimi; nesne silme;
  eski etiket koru/yenile; çoklu kat ve aynı planın farklı pafta oranları.

### ARCH-DIM-01 — Otomatik mimari ölçü dizileri [ ]

- Cephe/duvar grubu/mahal seç; açıklık, ara duvar, yapı ve toplam boy için
  seçilebilir sıralar. İç yüz, dış yüz, çekirdek ve açıklık merkezi/uçları açık.
- Ölçüler mevcut ilişkili ölçü modelinden üretilsin; Zincir/Baz/Hızlı ölçü
  davranışı değişmesin. Yeni araç görünüş çizgilerine ölçü bağlamasın.
- Metin sığmama ve sıra aralığı önizlensin; üst üste yazıları gizleyerek çözme.
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

### ARCH-DOC-02 — Çizelge ve metraj iş akışı [ ]

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
