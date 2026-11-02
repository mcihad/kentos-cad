# 0835 — Elips ucu sürüklenirken yalnız hedef noktasını tutmak

2026-11-02. Durum: uygulandı; çekirdek/komut ve gerçek Mac arayüzü kabulü geçti. CAD-ADV-05 açık.

## Kodda ve testte bulunan iki sorun

Elips Start/End noktasını üretmek için merkez, eksenler, yön ve uç
parametreleri okunur. `Hold` bu bilinmeyenlerin tamamını donduruyordu.
Ayrıca `moved_parts`, yalnız t0 değişse bile ortak bilinmeyenler nedeniyle
End'i de taşınmış sayıyordu. Sabit merkez + uçlar arası mesafede, mümkün
bir uç sürüklemesi yanlış çatışma veriyordu. İlk çekirdek kabulünde bu
sorunlar ve tam elips dikişinin gereksiz eksen kilidi üç testle yakalandı.

[AutoCAD grip düzenleme](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-Core/files/GUID-01DE459C-21A7-4E92-A1D6-E2C36CD89F0C.htm)
nesne ve grip türüne göre farklı düzenlemeler tanımlar.
[SolveSpace çözücü kaynağı](https://github.com/solvespace/solvespace/blob/master/src/system.cpp)
sürüklenen parametreyi ayrı ele alır. KentOS mevcut dual/Fix denklemini
kullanır; dış çözücü veya bu yazılımlardan kod eklenmedi.

## Karar

- Elips Start/End için geçici tutuş, güncellemenin istediği **gerçek
  nokta konumuna** iki Fix denklemi ekler. Eksen ve diğer uç için gerekli
  serbestlik kalır. Kaynak `fix_from` ile tutulan kalıcı Fix ise önceki
  belgeyi kullanmaya devam eder.
- Geçici hedefler önce değerlendirilir; hedefi engelleyen kalıcı bağ
  hata teşhisinde adlandırılır. Hedefler kaydedilmez, sahte kısıt ID'si
  üretilmez. Dahili denklem kaynağı `None`, kalıcı kaynak `Some(index)`dir.
- İki aynı geçici hedef fazladan kalıcı bağmış gibi raporlanmaz; gerçek
  kalıcı mükerrer bağ denetimi korunur. Geçici hedef çelişkisi yazmaz.
- Elipste değişen parça, ortak parametrelerinin değişmesiyle değil,
  önce/sonra gerçek nokta koordinatlarıyla bulunur. Sadece t0 değişirse
  yerinde kalan End geçici tutulmaz. Diğer geometri türlerinin tutuş
  davranışı genişletilmez.
- İkinci kalıcı teşhis ADR 0834'teki gibi geçici hedefler olmadan okunur;
  sürüklemenin geçici serbestliği çizimin kalıcı serbestliği diye sunulmaz.
- Mevcut pozitif eksen/arc-span sınırları, dal denetimi, atomik ret,
  plan/execute/Undo, dosya ve ortak API yolu korunur.

## Dünya koordinatındaki sabit uç doğrulaması

Etkileşim kabulü (487000,4420000) yakınında ayrı bir sayısal sorun buldu:
kaydedilmiş elips ucuyla izleyen çizgi noktası dünya koordinatında bit
bit aynıydı. Yerel merkezde yeniden değerlendirilen analitik uç ile
saklanmış çizginin yerel sayısı, yuvarlama işlemlerinin sırası nedeniyle
farklıydı. Query bu yüzden kalıcı Çakışık bağını tutmuyor gösteriyordu.

Yalnız **bütün bilinmeyenleri tutulan** bir Point–Point Çakışık denklemi
ve elips ucu varsa, başarısız yerel denetim mevcut `ops/subref::point`
ile saklanmış dünya noktalarında da doğrulanır. Bu, yakalama ve ilişkili
ölçünün okuduğu noktayla aynıdır. Aynı mevcut tolerans kullanılır; Newton
adımı veya toleransı değiştirilmez. Serbest denklem, çizgi–eğri teğeti ya
da başka bir geometriye örtülü hassasiyet payı verilmez.

[Autodesk büyük koordinat açıklaması](https://www.autodesk.com/blogs/autocad/working-large-coordinates-in-autocad/)
sonlu sayı hassasiyetinin sonuçlarını açıklar; bu kaynak geometrik boşluğu
rastgele büyütülmüş bir toleransla kabul etmek için gerekçe değildir.
Kabulde dünya noktasının bir sonraki temsil edilebilir Y değeri ve 1 µm
boşluk reddedilir. Orijine yakın 0.5 nm gerçek boşluk da reddedilir.

### Gerçek fare kabulünde yakalanan ek yuvarlama

İlk fiziksel sürüklemede farklı bir uç parametresi yine kırmızı Çakışık
simgesi üretti. Saklanan fark 0.93 nm idi. Elips noktası hesabı dünya
merkezine iki ayrı terimi sırayla ekliyordu; bir fazla dünya-koordinatı
yuvarlaması oluşuyordu. Yerel iki terim artık önce toplanır, dünya merkezi
bir kez eklenir. Dual nokta hesabı da aynı sıralamayı kullanır.
Gerçek kaydın uç noktası mpmath/100 basamakla bağımsız hesaplandığında
(486999.61320742796, 4420002.10790213) olarak yuvarlanır; yeni hesapla
ve saklanan izleyen noktayla aynıdır.

32 farklı yerleşim kabulü, polar çözücü parametrelerinin kaynak
major/ratio alanlarına yazılması sırasında da son bitin değişebildiğini
yakaladı. Başarılı çözümden sonra serbest, doğrudan saklanan izleyen
nokta, saklanmış analitik ucun kanonik dünya noktasına yazılır. Geçici
hold, katman kilidi ve kalıcı Fix'in koruduğu koordinatlar değiştirilmez.
Son saklanabilir şekillerde **bütün kalıcı bağlar yeniden, her nesne
sabitken** denetlenir; başka ölçü bozuluyorsa işlem kabul edilmez. Yeni
bir tolerans kullanılmaz. Durdurma isteği bu son, yinelemesiz doğrulamada
gelse de sonuç Cancelled olur ve değişiklik dönmez.

## Sonuç geometrisinin önizlemede kullanılması

Daha sıkı plan kabulü başka bir eksik gösterdi: `cad.entities.edit plan`
yalnız ham istenen geometriyi dönüyordu; çalıştırmada kısıtların değiştirdiği
elips ve izleyen çizgi aynı değildi. Doğrudan düzenlenen nesne artık plan
çıktısında son çözülmüş haliyle döner. Mevcut `changed` sözleşmesi korunur:
yalnız doğrudan düzenlenenler giriş sırasındadır. İzleyenler UI'nin dahili
`DerivedEntitiesPlan` sonucuna ayrı aktarılır; genel komut protokolünün
etki özeti yine bütün işlemi kapsar. Wire türü veya şema değiştirilmedi.

- Bu yol yalnız update içeren Grip/Properties içindir. Kesme/birleştirme
  araçlarının dış komutuna ait bağ kaldırma politikası değiştirilmez.
- Aynı ürün komutu okuma kopyasında gerçek grup içinde çalışır. Yeni salt
  okunur `Document::group_report` yalnız dokunulan nesnelerin son durumunu
  verir; grup sonra iptal edilir. Kaynak belge, Undo ve journal değişmez.
- Sürükleme, kaynak nesnenin Arc kimliği ve source generation ile tutulan
  okuma kopyasını tekrar kullanır. Kare başına bütün belge kopyalanmaz;
  her kare grubu iptal eder. Kaynak değişikliği/Undo kopyayı yeniler;
  tutamacın bitmesi kopyayı bırakır. Busy kopyaya gelen istek ayrı okumada
  planlanır; açık kullanıcı grubuna yazılmaz.
- Seçili elips ve izleyen çizgi artık aynı çözümden çizilir. Türetilmiş
  nesneler/komşular ikinci kez ayrıca eklenmez; eski hayaletler birikmez.
- Geometrik bağın reddi kırmızı önizleme kartında gösterilir. Hatanın
  gerçek kısıt ID'lerinden bağın sırası ve adı (örneğin `4. Sabit`) yazılır;
  bilinmeyen bir otomatik gevşetme eylemi önerilmez.

## Sınır

Bu iş Start/End nokta hedefi ve kısmi yay grip'idir. Tam elipsin tüm eksen
grip'leri için bağımsız eksen başvuruları, dikiş açısının anlamı ve eksen
sıralaması ayrı tasarlanmalıdır. Genel elips teğeti uygulanmış sayılmaz.

## Kabul

Ortak ham örnek: `fixtures/interaction/v1/constraint-ellipse-drag.kcad`.
Merkez (0,0), yarı eksenler 4/2 m, uçlar 0/π/2; End'e bağlı çizgi.
Sabit merkez, √20 m kiriş ve Çakışık ile Start hedefi
(4 cos(0.3), 2 sin(0.3)) olmalı; End ve izleyen çizgi mesafeyi koruyarak
çözülmeli. Başvuru kimlikleri/öznitelikler ve çizginin diğer ucu kalmalı.
Plan yazmamalı, dosya okunmalı, tek Undo kaynak belgeye dönmeli. Kalıcı
Start Fix ile aynı hedef atomik reddedilmeli. UI/Python/MCP kabulü aynı
ortak komut ve geometriyi doğrular.

- Çekirdek 904, domain 132, uygulama 392, etkileşim 946 test geçti;
  MCP hedef kabulü 1, Python 4 test geçti. Masaüstü 1579 test geçti
  (256 mevcut ignored); bütün ilgili paketlerde ve masaüstünde all-targets
  clippy `-D warnings` geçti. Derleme ve 51 crate / 67 hedef bağımlılık
  denetimi geçti. Mevcut ignored/perf testler çalıştırılmış sayılmadı.
- Gerçek Mac'te (487000,4420000) yakınında fareyle bağlanan ortak çizim
  kullanıldı. Fiziksel uç sürüklemesi, ölçülü `@dX,dY` hedefi ve tek Enter,
  merkez/kiriş/izleyen çizgi, üç yeşil bağ ve 6 kalıcı serbestlik doğrulandı.
  Sayısal girişin son geometrisi Python ile 1e-8 içinde; UID/öznitelik ve
  teşhis aynı. Her başarılı düzenleme tek Undo ile kaynak kanonik belgeye
  tam döndü.
- Ayrı kalıcı Start Fix gerçek fareyle eklendi. Önizleme kırmızı kartında
  gerçek `4. Sabit` bağını adlandırdı. Sayısal ve fare hedefi atomik
  reddedildi; dosya aynı kaldı. Sonraki tek Undo Fix eklemesini geri aldı;
  başarısız denemeler geçmiş eklemedi.
- Sürükleme önizlemesinde çözülmüş yay ile izleyen çizgi birlikte görüldü;
  Escape kaynak çizimi korudu. Yakınlaştırmayı küçültüp geri getirmek
  geometriyi değiştirmedi. Hareket sırasında kare hızı ölçülmüş sayılmaz.
- Kanıtlar repo dışı `/tmp/kentos-cad-adv04-2026-11-01/` altında:
  `99-ellipse-real-drag-canonical.png`, `100-ellipse-typed-one-enter.png`,
  `101-ellipse-typed-three-bonds-hold.png`,
  `103-ellipse-conflict-names-fixed-bond.png`,
  `104-ellipse-settled-preview-with-follower.png`,
  `ellipse-target-gui-acceptance.json` ve `ellipse-roundtrip-*.log`.
  Ekran görüntüleri repoya eklenmedi.

Arayüzde ayrıca bulunan açık işler: kısmi elips için bağlamsal sekme yanlış
“Alan” diyor; sağ panel sınırındaki kısıt etiketi kırpılabiliyor. Bu kabul
bu iki sorunu çözmüş veya tam elips eksen grip'lerini tamamlamış sayılmaz.
