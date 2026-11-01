# 2B eğri düzenleme: destek ve sınırlar

2026-11-02 — CAD-ADV-04 masaüstü kabul kaydı. Desteklenen kapsam ve
açıkça desteklenmeyen tür çiftleri aşağıdadır. Masaüstünün native anlamsal komutlarını,
aynı komutları çağıran Python ve MCP'yi tarif eder. Web araçlarının
TypeScript etkileşimleri ayrıca doğrulanmadan eşdeğer sayılmaz; web işi
[ADR 0228](../adr/0228-web-pause-desktop-and-cloud-first.md) kapsamında ertelenmiştir.

## Hedef geometriler

| İşlem | Çizgi / düz yol | Yay / daire / yaylı yol | Elips | NURBS | Delik / çoklu parça |
|---|---|---|---|---|---|
| Buda | Var | Analitik yay/bulge korunur | Elips yayı olarak kalır | Knot ekleyerek NURBS parçaları | Delikli alan reddedilir; Alan böl önerilir. Multipart native katmanı seçilen parçayı işler. |
| Uzat | Açık uçlar | Açık yay ve açık yolun uç kenarı | Elips yayı aynı elips üzerinde ilk sınıra uzar; tam elips reddedilir | Genel destek yok | Kapalı hedef uzamaz; multipart için uygun açık parça gerekir. |
| Kır | Var | Yay/bulge korunur | Elips yayı olarak kalır | NURBS parçaları | Delikli alan reddedilir. Multipart seçilen parçada kırılır, kalan parçalar korunur. |
| Birleştir | Çizgi/açık çoklu çizgi | Bağımsız yay, yaylı yol | Genel destek yok | Genel komutta destek yok | Her ada otomatik tek kontur sayılmaz; bağlantı ve görünüş kuralları denetlenir. |
| Köşe yuvarla | Mevcut düz kenar/yol köşeleri | Bağımsız çizgi–yay, çizgi–daire, yay–yay, yay–daire ve daire–daire için yeni analitik çözüm | Yok | Yok | Yeni dairesel çözüm, yolun yaylı alt kenarını bağımsız yay gibi değiştirmez. |
| Pah | İki düz kenar ve düz yol köşesi | Yay/daire çiftleri reddedilir | Yok | Yok | Mevcut yol köşesi sınırları geçerlidir. |
| Ötele | Var | Daire/yay ve uygun bulge analitik | Açık onay ve kullanıcı toleransıyla rasyonel Bézier paraleli | Pozitif ağırlıklı parçalarda aynı türev sınırı, kaynak/kopya raporu | Alanın delikleri karşı yönde ötelenir; çökme reddedilir. Yaklaşımın sayısal payı ve hesap sınırları ADR 0822'dedir. |

`Spline` etiketli eski fit eğrisi ile `Nurbs` aynı tür değildir.
Eski spline'ın Buda desteği yoktur. `cad.curves.edit`, NURBS'in fit ve
kontrol verilerini ayrı işlemlerle düzenler. Bir tablo hücresinde “var”
yazması her tür çiftinin ve her topolojinin desteklendiği anlamına gelmez.

### Sınırlar, kimlikler ve görünüş

- Buda/Uzat sınırları çekirdeğin `Edge` temsiline gider. Çizgi ve daire
  yayları analitiktir; eğri sınırların aday kesişimleri örneklenebilir.
  NURBS hedefin kesişim adayları 1 mm kirişlerden başlatılıp eğri/sınır
  üzerinde iyileştirilir. Bu, tüm eğri çiftlerinde sertifikalı kök
  izolasyonu vaadi değildir.
- NURBS bölünmesinde knot ekleme kaynak eğriyi korur. Kaynak kontrol
  noktalarını körlemesine kesmek veya NURBS'i habersiz polyline yapmak yoktur.
- Yerinde düzenlenen nesne UID'sini ve verisini korur. Ayrı parçaların yeni
  UID alması normaldir; bunlar aynı alt nesnenin değişmeyen kenar indeksi
  olarak sunulmaz. Split/merge soy kaydı ve ilgili ölçü/tarama politikası
  ayrıca denetlenir. Her eğri işlemi için kalıcı alt kenar kimliği
  korunuyor iddiası henüz doğrulanmamıştır.
- Değişken genişlikli, iki köşeli çoklu çizginin budanan parçası LINE'a
  düşürülmez. Yeni uç genişliği kaynak boyunca enterpole edilir. Uzatmada
  mevcut kenarın başlangıç/bitiş genişlikleri korunur; kaynak dışındaki
  yeni uç yanlışlıkla sıfır genişlik almaz.
- Bağımsız iki sınır nesnesi yuvarlandığında/pahlandığında ve ikisi de
  aynı ilişkili taramanın sınırıysa yeni bağlayıcı sınır listesine katılır.
  Tarama takipçisi dış komut adımı kapanırken çalışır. `trim=false`
  bu ilişkiyi değiştirmez. Tüm köşeler/toplu bağlantı değişikliklerinin
  aynı kapsamda sınandığı varsayılmaz.

## Yuvarlama ile iki mesafeli pah

Pahın `distance` değeri birinci, `second` değeri ikinci kenardır.
`second` verilmezse eşit mesafe kullanılır. Arayüzde `10cm;20cm`
girilebilir; kenar sırası sahnede **1. kenar / 2. kenar** diye gösterilir.
Tek değer eşit pah oluşturur. Noktalı ondalık ve açık birim, iki sayının
ayıracıyla ondalık virgülün karışmasını önler.

Dairesel yuvarlamada `radius` tektir. Dik köşede iki teğet uzaklığının
eşit olması bu dairenin geometrisinden gelir. Kenarlara farklı
uzaklıklarda oturan eğrisel geçiş, ayrı bir **eliptik/geçiş eğrisi**
işlemidir; iki farklı “daire yarıçapı” diye adlandırılmaz. Bu özellik
henüz yoktur. Eklenmeden önce uç mesafeleri, teğet yönleri, G1/G2
seçeneği ve birden fazla çözümün önizlemesi tanımlanmalıdır.

Yeni dairesel çözüm bütün merkez adaylarını offset doğruları/dairelerinin
kesişiminden bulur; tıklamalara en yakın geçerli teğet çiftini seçer.
Hesap yerel başlangıca taşınmış f64 koordinatlarla yapılır. Kaynak yay
sonlu aralığında kırpılır; daire bütün kalır. Uç noktanın birkaç piksel
dışına tıklama, yanlışlıkla yayın öbür ucunu seçmez. G1 kontrolü yalnız
teğet doğruların paralelliğine bakmaz: kalan parça ve yeni yayın temas
noktasından ayrılan yönleri karşıt olmalıdır; sivri dönüş reddedilir.
Sıfır yarıçap gerçek kesişimde açık hedefi kırpar, yeni yay üretmez.
Hiçbir şeyi değiştirmeyen/çöken/eş merkezli çözüm güvenle reddedilir.

```python
from kentos import cad

corner = cad.LinesCornerPick(
    first=cad.TrimPick(uid=first_uid, at=(x1, y1)),
    second=cad.TrimPick(uid=second_uid, at=(x2, y2)),
)
cad.entities.fillet.plan(doc, corner=corner, radius=.5)
cad.entities.fillet(doc, corner=corner, radius=.5, trim=True)
# Pah: birinci kenar 10 cm, ikinci kenar 20 cm; wire birimi metredir.
cad.entities.chamfer(doc, corner=straight_corner, distance=.1, second=.2)
```

## CAD-ADV-04 kabul kaydı

- [x] Elips/fit/NURBS ötelemesinde açık yaklaşım onayı ve tolerans,
  rasyonel Bézier parçalarının türev sınırı + f64 payı, kaynak/kopya
  raporu ve tekillik/kesişim denetimi eklendi. Bu tam yönlü IEEE aralık
  ispatı değildir; sayısal ve hesap bütçesi sınırları [ADR 0822](../adr/0822-user-toleranced-curve-offsets.md)'de açıktır.
  Eski kaynak kiriş tahmini sonuç garantisi olarak kullanılmaz. Native,
  Python, MCP ve GUI plan/yazma/Undo kontrolleri geçti; gerçek GUI sonucu
  bağımsız normal uzaklığı ve yoğun paralel/kiriş denetimiyle kontrol edildi.
- [x] Fit/kontrol noktaları, uç teğetleri ve açık/kapalı geçişler için
  görünür ayarlar, doğrudan nokta seçimi, koordinat girişi ve gerçek arayüz
  kabulü. G2 eşlemesi olmadığı, Aç/Kapat'ın yeniden kurduğu ve düğümün şekli
  koruduğu açık gösterilir. Aynı mevcut komutlar, kaydedilen GUI/Python
  karşılaştırması ve fare paneldeyken Undo/Redo kontrol edildi.
  [ADR 0824](../adr/0824-visible-curve-editing-and-handles.md).
- [x] Ağırlıklı NURBS başlangıç/bitiş/normalize parametre başvuruları:
  gerçek knot domain'i, knot eklemede aynı konum, denetim düzenlemede
  takip, Kır'da yeni parçaya aktarım, Buda/dönüşümde açıklanmış kopma.
  Native/Python/MCP, gerçek GUI ve bağımsız dosya/De Boor doğrulaması
  geçti. [ADR 0826](../adr/0826-nurbs-measurement-references.md),
  [Python örneği](curve-references-python.md).
- [x] Delikli/yaylı kapalı alan, bağlı ölçü/tarama ve çoklu parçada
  birleşik alan/uzunluk/kimlik/Undo kabulü. Alan bölmede tarama tohumunu
  içeren parçaya, ölçü gerçek köşeye aktarılır. Çoklu parçanın dokunulmayan
  kısmı korunur; değişen parça indeksi başvurulara taşınır. Analitik
  kapsül alanı ve çevresi bağımsız Green integraliyle doğrulandı.
  [ADR 0827](../adr/0827-curved-area-split-reference-identity.md).
  Destek tablosundaki retler genel desteğe dönüşmüş sayılmaz; genel
  elips/NURBS Birleştir ve NURBS Uzat bu dilimin uygulanmış yetenekleri
  değildir. Delikli alanda Kır yerine alan yapısını koruyan Alan böl vardır.
- Web'in bağımsız yay/daire yuvarlaması ve toleranslı offset araç yüzeyi
  ADR 0228 uyarınca ertelendi; [TODOS §0.2](../../TODOS.md) dönüş işini tutar.
  Masaüstü kabulü için bu arayüz şimdi uygulanmaz.
- [x] Asimetrik eliptik/geçiş araştırması ayrı geometrik sözleşmeyle
  tamamlandı: [karar ve sonraki özellik sınırı](asymmetric-curve-blends.md).
  Bu, yeni Blend aracının uygulandığı anlamına gelmez; mevcut dairesel
  `radius` alanına ikinci yarıçap eklenmez.
- [x] Masaüstü imleç değer kutusu ortak KentOS metin bileşenine geçirildi.
  Açıldığı yerde sabit durur; tıklama, tümünü seçme, düzenleme ve yapıştırma
  desteklenir. Kutunun üstünde CAD artısı çizilmez. Gecikmiş girişler
  geçici kutu kimliğiyle reddedilir. Gerçek GUI'de `10cm;20cm`, tek Enter,
  kaydedilmiş uç koordinatları, tek Undo ve Esc ile iptal kontrol edildi.
- [x] Girilen sayının yazarken canlı geometri önizlemesine bağlanması.
  Pahın iki mesafesi ve yuvarlama yarıçapı salt okunur aynı komut planıyla
  gösterilir; fare hareketi yazılan değerin yerini almaz. Geçersiz metin veya
  geometri, son hatırlanan değere düşmeden kutuyu düzeltmeye açık bırakır.
  Tek Enter, sahne tıklaması veya menü onayı aynı yazılı değeri uygular.
  Menüde değer görünür; ilk Esc yalnız menüyü, sonraki Esc değer taslağını
  kapatır. Gerçek GUI'de kaydedilen 10/20 cm pah uçları, 25 cm yay merkezi/
  yarıçapı, kaynak kimlikleri ve tek Undo bağımsız dosya okuyucusuyla doğrulandı.
  [ADR 0823](../adr/0823-live-corner-value-previews.md).

## Kod dayanakları

- `crates/shared/geometry-core/src/ops/fillet_curves.rs`: analitik merkezler,
  seçim, sonlu yaylar, yönlü G1 ve bağımsız matematik testleri.
- `crates/native/application/src/corner.rs`: doğrulama, salt okunur tam
  takipçi önizlemesi, tarama sınırı, tek Undo ve metadata testleri.
- `crates/native/interaction/src/corner/`: sahne seçimleri ve aynı anlamsal
  komut. `cad_corner.rs` ve `capabilities.rs`: AI/Python/MCP açıklaması.
- `ops/path.rs`, native `edit.rs`, `widths.rs`: genişlik taşıma.
- `application/src/trim.rs::extend_ends`: elips yayı tıklama/çit/pencere
  uçları ve çekirdekle aynı uç seçimi; [ADR 0825](../adr/0825-native-elliptical-arc-extension.md).
- `ops/trim.rs`, `ops/nurbs_cuts.rs`, native `trim.rs`, `break_join.rs`:
  parçalar, sınır hesabı ve delik retleri. `interaction/src/area.rs`:
  mevcut Alan böl alternatifi.
- `ops/offset_bounded/`: rasyonel parçalar, paralel türev sınırı,
  katlanma/birleşim, kiriş bütçesi ve büyük sonuçların kesişim denetimi.
- `application/src/offset.rs`, `interaction/src/offset.rs`: açık onay,
  aynı plan/yazma, kaynak/kopya raporu, sabit mesafede plan önbelleği.
- `ops/offset.rs`, `ops/curve_cuts.rs`: eski geometri oynatma uyumluluğu;
  eski örnekleme sayıları sonuç paralelinin hata garantisi değildir.
- `application/src/curves_edit_cmd.rs`, `interaction/src/curve_edit.rs`:
  mevcut kontrol/fit/knot/ağırlık/teğet/aç/kapat araçları.

## İncelenen birincil kaynaklar

- [AutoCAD FILLET](https://help.autodesk.com/cloudhelp/ENU/AutoCAD-Web-Help/files/List-Commands/AutoCAD_Web_Help_List_Commands_FILLET_html.html):
  yarıçap, kenar seçimi ve kırpma davranışı.
- [AutoCAD CHAMFER](https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-Core/files/GUID-B1DCF991-90A7-4DB0-96FC-BDA3FB76337C.htm):
  birinci ve ikinci nesnenin ayrı mesafeleri ile mesafe/açı yönteminin ayrımı.
- [QCAD Offset](https://www.qcad.org/en/tutorials/offsetting/):
  paralel oluşturma ve eğrisel sonuçların uygulama davranışı.
- [QCAD RSpline arayüzü](https://qcad.org/doc/qcad/latest/developer/_r_spline_8h_source.html):
  toleranslı `approximateWithArcs`, kesişim ve spline düzenleme API ayrımı.
- [Open CASCADE Geom2d_OffsetCurve](https://www.occt3d.com/dev/doc/refman/html/class_geom2d___offset_curve.html):
  offsette kaynak sürekliliği, teğet ve tanımlılık sınırları.

Bu kaynaklar KentOS'un hata garantisi için kanıt değildir. Sayısal
garanti kendi algoritması ve bağımsız kabul verisiyle doğrulanmalıdır.
