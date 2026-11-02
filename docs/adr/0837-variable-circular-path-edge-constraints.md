# 0837 — Çoklu çizginin dairesel kenarında değişken eğrilik

2026-11-02. Durum: uygulandı; otomatik ve gerçek Mac arayüz kabulü geçti. CAD-ADV-05 tamamlanmadı.

## Araştırma ve sınır

[AutoCAD geometrik kısıtları](https://help.autodesk.com/cloudhelp/2023/ENU/AutoCAD-Core/files/GUID-668B1B7D-9991-44CA-8607-83665A82FF7F.htm)
çoklu çizginin parçalarına ayrı nesneler gibi bağ kurulabildiğini anlatır.
[DCRADIUS](https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Core/files/GUID-C0FE6334-7C41-4382-8C28-3EB776CDCACB.htm)
daire/yay yarıçapını sürer;
[BCPARAMETER](https://help.autodesk.com/cloudhelp/2023/ENU/AutoCAD-Core/files/GUID-F22AD919-40DF-4AC3-8339-CD212D79EB94.htm)
yaylı polyline parçasının yarıçapını açıkça kapsar. KentOS'ta düz
Segment'in anlamı değiştirilmedi; yeni parça mevcut çözücüye eklenir.
Dış çözücü veya üçüncü taraf kod alınmadı.

## Geometrik sözleşme

- `CadElement::ArcEdge {part,ring,index}` / `Element::ArcEdge`, mevcut
  dairesel path kenarının alt dairesidir. Nokta–daire, teğet, eş yarıçap,
  eş merkezlilik, Fix, yarıçap/çap mevcut daire kurallarını kullanır.
  Bu, sınırlı yay üzerinde yeni bir serbest nokta veya yay süpürme açısı
  kısıtı değildir. Uçları ayrı kalıcı `point/vertex` başvurularıdır.
- `model.rs` her path kenarının eğrilik yuvasını kararlı tutar; yalnız
  açıkça ArcEdge ile anılan eğrilik serbesttir. Eski köşe/düz kenar
  bağlarında bulge ve serbestlik sayısı korunur. Noktaların, halkaların ve
  parçaların başlangıç yuvaları model tarafından izlenir. Kapanış kenarı
  gerçek sıfırıncı köşeye döner; yanlış `index+1` başvurusu üretilmez.
- Merkez/yarıçap mevcut bulge geometrisiyle analitik dual sayılardan
  kurulur: orta nokta + kirişin sol normali × `(1-b²)/(4b)`,
  `r = chord × (1+b²)/(4|b|)`. Eğrilik adımı yarım kiriş uzunluğuyla
  ağırlanır; bir açının yanlışlıkla path'in üçüncü koordinatıyla
  ağırlanması önlenir. Hareket tespiti de fiziksel sagitta farkıdır.
- Sıfır/ters işaret, çökmüş kiriş ve kaynak küçük/büyük dalını geçen
  deneme reddedilir; tolerans genişletilmez. Yarım dairede sabit uçlu
  yarıçapın türevi sıfırdır. Açık yarıçap/çap isteği mevcut dal üzerinde
  analitik başlangıç verir; sonra bütün kalıcı denklemler normal çözülür.
  Yarım daire küçük dalı seçer, çok yakın mevcut büyük dal korunur.
- Yalnız değişen bulge yazılır. Diğer eğrilikler, köşeler, genişlik,
  delikler, parçalar ve kimlik/öznitelik kaynakları korunur. Açık uç
  genişliğinin son satırı mevcut dosya kuralı gereği sıfırdır.
- Fix'in önceki şekli değerlendirilirken o şeklin kendi dalı kullanılır;
  düzenlenmiş şeklin yeni işareti eski Fix sabitini geçersizleştirmez.

## Etkileşim ve komut yolu

- `constraint_tool.rs`: gerçek işaretlenen yaylı kenar seçilir. Aynı path'in
  uzaktaki yayını/düz kenarını sırf o tür istendi diye seçmek yerine
  tıklama ile kenar arasındaki mesafe denetlenir. Uç için köşe seçimi
  korunur; radius, diameter ve daire kuralları yeni kenarı alır.
- `solve.rs::tangent_end`: aynı path'in bitişik düz ve yaylı kenarları
  ortak köşelerini zaten paylaşır; mükerrer bir Çakışık bağı gerekmez.
  Ayrı çizgi/yayda açık Çakışık bağı yine ortak ucu tanımlar. Teğet
  türevi o uçtaki yarıçap/doğru dikliğinden hesaplanır.
- `constraints_edit.rs`: daire parçasının gerçek merkeziyle yeni bağın
  çizimdeki tarafı okunur; ArcEdge merkezine sahte tek-nesne `Center`
  başvurusu yapılmaz. Kilit, plan, query, execute ve Undo ortak komutlardır.
- `constraint_marks.rs` gerçek yayın orta noktasını ve analitik yay
  vurgusunu kullanır; dal kapsamı araç isteminde görünür.
- Katalogtaki yarıçap/yay uzunluğu ölçülendirme yetenek listesi zaten
  desteklenen polyline/polygon yaylarını artık açıkça anar. Tür testi
  düz köşe gerektiren araçlara düz köşe, yay ölçen araca yaylı kenar verir;
  iki noktalı yayı yanlışlıkla her polyline işleminin örneği yapmaz.

## Kalıcılık ve AI/Python

- Şema 157 yalnız ArcEdge kullanan kısıtta yazılır. Önceki desteklenen
  şema bu etiketi reddeder. Şema 156 başka devam eden çalışma için
  ayrıldığı için yeniden kullanılmadı; şema kabul testi sayısal boşlukta
  varsayımsal bir okuyucu yerine **önceki desteklenen** şemayı denetler.
  Bilinmeyen 156 sürümü `schema_version`, onu destekleyen çalışma dalında
  ArcEdge'nin eski şemadaki varlığı `bad_value` verir.
- Native codec ve bağımsız Python okuyucusu/generatöründe açık etiket,
  indeks sınırı, fazla alan ve eski şema reddi eşleşir. Eski dosyaların
  en eski uygun şemada baytları korunur. Bağımsız yazılmış örnekler
  `fixtures/kcad/v2/arc-edge-*` ve `broken/arc-edge-*` altındadır.
- Ortak komut şemasından yalnız ilgili CadElement TypeScript tipi ve
  Python `ArcEdgeCadElement` üretildi. MCP aynı komutları ve yeni oneOf
  şemasını okur. [Python/MCP kullanımı](../cad/arc-edge-constraints-python.md).

## Kabul

Çekirdek: iki işaret, küçük/büyük dal, yarım daire, büyük koordinat,
korunan uçlar/genişlikler; imkânsız yarıçap; eski bağın serbestliği;
çok parçalı deliğin kapanış yayı; aynı path'te örtülü ortak uç teğeti.
Komut: plan yazmaz, execute son geometrisi, tam dosya geri okuma, tek
Undo, kilit ve atomik ret, ayrı çizginin ortak uçtaki teğeti.
Etkileşim: iki sabit köşe + fareyle yay ortası + tek değer onayı; düz
kenarı radius saymama; aynı path'te düz/yay seçimi ve teğet.

Büyük koordinatta yerel çözümün f64 saklama farkını sahte çelişki sayan
sorgu, [ADR 0838](0838-stored-constraint-roundtrip-certificate.md) ile
sınırlı salt okunur doğrulama alır; tolerans değiştirilmez.

- Yeni odaklı kabul: çekirdek 10, native komut 4, etkileşim 3, MCP 1
  test. Geniş çekirdek/domain/application/interaction/KCAD paketleri
  hatasız; KCAD 234 test. Masaüstü 1582 geçti, mevcut 256 test ignored.
  Python'un kısıt dosyalarında 14 test ve ilgili bütün Rust hedeflerinde
  Clippy geçti. Bağımsız üreticinin 1481 dosyası, SDK ve katalog güncelliği yayımlanan kod tek başınayken doğrulandı.
- Gerçek Mac arayüzünde 487000 / 4420000 m çizimde altı bağ fareyle
  kuruldu: iki sabit köşe, ArcEdge yarıçapı, ayrı çizgi ucunun Çakışık
  bağı, çizgi başlangıcı–sabit köşe uzaklığı ve ortak uçta Teğet.
  Yarıçap tek Enter ile kabul edildi, parametre 2.5→3 m değiştirildi.
  Altı bağ geçerli, kalıcı serbestlik sıfır; yay ve teğet çizgi birlikte
  değişti. Uçlar, genişlikler, UID ve öznitelikler aynı kaldı.
- Gerçek GUI'nin kaydettiği başlangıca aynı Python komutu uygulandı:
  geometri farkı **0**, bağlar/parametreler/teşhis aynı. Plan belgeye
  yazmadı, query salt okunur kaldı. Dosya gerçek uygulamada yeniden
  açılınca altı bağ yine yeşildi. Tek araç çubuğu Undo'su başlangıç
  dosyasının baytlarına tam döndü. 1.5 m isteği reddedildi; kaydedilmiş
  belge önceki dosyayla tam aynı kaldı.
- İkinci gerçek GUI çiziminde aynı path'in üç köşesi sabitlendi;
  düz kenar 0 ve yaylı kenar 1 fareyle seçilerek ortak uçta Teğet
  kuruldu. Köşeler tam aynı kaldı, bulge 0.35→0.5 oldu; dört yeşil
  bağ ve sıfır katılan serbestlik doğrulandı. Tek Undo, 0.35 ve üç
  Fix bağını geri getirdi; tek Redo kabul edilmiş dosyanın baytlarına
  tam döndü. Son ArcEdge başvurusu kaldırılınca eğrilik mevcut sabit
  veri olarak kalır; kapsam dışı eğrilik serbestlik iddiası yapılmaz.
  Ek kanıt `arc-edge-adjacent-gui-acceptance.json`, ekranlar
  `115-arc-edge-adjacent-tangent.png`, `116-arc-edge-adjacent-four-green.png`,
  `117-arc-edge-adjacent-undo-three-fixed.png`.
- Ekranlar repo dışında: `109-arc-edge-six-ui-bonds.png`,
  `110-arc-edge-three-m-six-green.png`, `111-arc-edge-three-m-scene.png`,
  `112-arc-edge-single-undo.png`, `113-arc-edge-impossible-radius-refused.png`,
  `114-arc-edge-reopened-six-green.png`. Sayısal kayıt
  `arc-edge-gui-acceptance.json`; çalışma dizini `/tmp/kentos-cad-adv04-2026-11-01/`.

Açık UX: imkânsız yarıçap için genel tekillik iletisi atomik reddi
bildirir; sabit kirişe göre asgari yarıçapı söyleyen daha özel açıklama
henüz yok. Bu iş boyut cebirini, tam elips eksen grip'lerini veya
CAD-ADV-05'in bütün birleşik dal kabulünü tamamlamış sayılmaz.


## Yayımlanacak kodun bağımsız doğrulaması

Diğer bekleyen çalışmalar geri yüklenebilir biçimde ayrıldı; yalnız bu
commit'in kaynaklarıyla geniş Rust paketleri, MCP kabulü, tam katalog
karşılaştırması, SDK üretim karşılaştırması ve 1481 bağımsız KCAD dosyası
geçti. All-targets Clippy, 1582 masaüstü testi ve masaüstü derlemesi
hatasız. Python uzantısı bu kaynaklardan yeniden derlendi; 14 kısıt
testi yeniden geçti. Temiz masaüstü derlemesinde gerçek GUI ile aynı
altı bağda 2.5→3 m tekrarlandı: `118-clean-main-arc-edge-six-green.png`.
Kaydettiği `arc-edge-clean-main-three.kcad`, önceki kabul dosyasıyla
**bayt bayt aynı**. Görseller ve test çıktıları repo dışında kaldı.
