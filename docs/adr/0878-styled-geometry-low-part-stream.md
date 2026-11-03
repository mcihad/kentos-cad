# ADR 0878 — Stilli CAD geometrisinin düşük konum parçasını koru

Tarih: 3 Kasım 2026. Durum: native geometri dilimi doğrulandı; aşağıdaki sınırlar açıktır.

## Gerçek hata ve kapsam

CAD-ADV-07'nin gerçek masaüstü denemesinde X=4 000 000 000 m,
Y=1 000 000 m çevresindeki 0,6×0,5 mm bloklar yatay çizgiye dönüştü.
Bağımsız KCAD okuyucusu blok tanımında köşelerin ve dairenin doğru olduğunu
doğruladı. Dizi tanıma/bağlama kökleri değiştirmemişti (ADR 0877).

Stil çekirdeğinin `BatchSink::finish` adımı, karo içindeki konumu tek
FP32'ye indiriyordu. ADR 0157'nin 65 536 m karosu uzaklığa bağlı büyük
kaybı sınırlıyor, milimetre altı ayrıntıyı garanti etmiyordu. Kamera düşük
parçasının bulunması, çoktan kaybolan köşe farkını geri getiremiyordu.
Eski kodda bağımsız CPU görüntü testi beklenen 10 piksel yerine sıfır
piksel yüksekliği ölçerek başarısız oldu; başlangıçtaki yerel örnek geçti.

## Karar

- Artık akışını yalnız bunu kullanan native derleyici açıkça ister
  (`View.position_low`); eski/web bağlamında boş kalır ve fazladan paketleme
  belleği ayrılmaz. Bu bir kullanıcı ayarı veya model seçeneği değildir.
- Mevcut karo, topluluk anahtarı, çizim sırası, yüksek veri dizisi ve eski
  GPU girişleri korunur. Her paketlenmiş değere FP32 artık eşlik eder:
  `high = f32(value)`, `low = f32(value - f64(high))`. Kaynak belge, birim,
  model koordinatı, geometri ve ölçek değişmez.
- Native `StyledLayer`, paralel düşük diziyi taşır. Boş düşük dizi, eski
  elle üretilmiş katmanlarda sıfır kabul edilir. CPU resim ve PDF vektör
  yolları görüntü dönüşümünden önce FP64'te iki parçayı birleştirir. PDF
  üçgen kenarı anahtarları da FP64 bitlerini kullanır; tekrar FP32'ye dönmez.
  Son halka yönü sınıflaması ortak çekirdeğin ilk köşeye göre alan hesabını
  kullanır. Ham dünya çarpımları bu örnekte yönü sıfıra düşürüp dolguyu PDF'den
  çıkarıyordu; kaynak FP64 köşelerinin bağımsız Python `Fraction` referansı
  pozitif yönü doğrular. Tolerans/epsilon eklenmez.
- Ortak WGSL sözleşmesi sürüm 10'da **isteğe bağlı** hassas girişler ve ikinci
  konum tamponunu tanımlar. Native çizgi, alan ve işaret yolları bunları
  kullanır. Formül `((high - cameraHighRelativeToTile) +
  (low - cameraLow)) * pixelsPerMetre` olur. İki konum parçasını kamerayı
  çıkarmadan toplamak yasaktır. Görünüm dönüşü küçük piksel farkına uygulanır.
- [Cesium'un güncel kamera göreli gölgelendiricisi](https://github.com/CesiumGS/cesium/blob/main/packages/engine/Source/Shaders/Builtin/Functions/translateRelativeToEye.glsl)
  aynı yüksek/yüksek ve düşük/düşük çıkarma düzeninin birincil örneğidir.
  KentOS'un mevcut düz çizim yolu zaten benzer yaklaşımı kullanır;
  bu karar stilli yolun geometri kaybını kapatır.
- Tamponlar katman yüklenirken oluşur; her fare hareketinde yeniden
  paketleme veya GPU okuma yoktur. Konum tamponu belleği iki katına çıkar;
  GPU katmanının bellek hesabı iki tamponu da sayar. CPU gölge akışı ve
  CPU çıktı için geçici FP64 dizi maliyeti açık bir doğruluk tercihidir.
  MSAA, atlas, stil blokları, iki bind grubu ve çizim çağrısı sayısı değişmez.

## Kabul ve sınırlar

CPU regresyonu; aynı 0,5 mm yüksekliğin yerel, büyük pozitif ve büyük
negatif konumda korunmasını ölçer. Gerçek GPU testi üç konum × üç geometri
(dolgu, çizgi, işaret) × iki kaydırma × iki görünüm açısını kontrol eder.
İlgili stil fixture'ları, PDF ve ortak WGSL sözleşmesi ayrıca doğrulanır.
Gerçek KentOS penceresinde aynı KCAD dosyası son derlemeyle tekrar açılıp
sahne, yakınlaştırma ve kaydırma ekranları alınmadan GUI kabulü tamamlanmaz.
Son birleşik koşuda native stil, pafta/PDF ve render testlerinde 13 grup
üzerinden 207 test başarılı, sıfır başarısızdır. Altı elle çalıştırılan
performans testi atlandı; bunlar başarılı sayılmaz. Gerçek GPU'nun üç testi
çalıştı; yedi stilli WGSL sözleşme testi de başarılıdır. PDF halka yönü son
kez genişletildikten sonra 17 PDF birim testi ayrıca tekrar başarılıdır.
İlgili desktop/style/render/plot tüm hedeflerde son Clippy denetimi geçti.
Ek masaüstü stil/önbellek/pencere denetiminde 84 test başarılı, dokuz elle
performans testi atlandı. Son teslim derlemesi de yeniden açılıp sembolün
kenarına fareyle tıklanarak #6 seçildi, 11 öğe/3 mm arama tekrar doğrulandı;
413–414 ekranları ve `styled-precision-qa-delivery-binary.json` bu son
programı kaydeder.

Son masaüstü derlemesi gerçek pencerede açıldı: 11 sembolde dikdörtgen ve
iç daire görünür, Kaydır aracıyla 60×60 piksel hareket ve tekerlekle uzaklaşma/
yakınlaşma denendi. 409–412 ekranları repo dışındadır. GUI'den yeniden
kaydedilen dosya başlangıç KCAD'iyle byte düzeyinde aynıdır; model/birim
veya ölçeği değiştirerek görüntü düzeltme yapılmamıştır. Manifest:
`styled-precision-gui-acceptance.json`; test penceresindeki programın
SHA-256'sı `styled-precision-qa-binary.json` içindedir.

Bu dilim native stilli geometri içindir. Web'in eski veri/giriş yolları
korunur; web henüz düşük akışı bağlamaz ve aynı hassasiyet iyileştirmesi
orada teslim edilmiş sayılmaz. Dinamik raster dörtgenleri ile fragman
boya koordinatlarının (desen evresi, resim, degrade) FP32 sınırı da ayrı
kapsamdır. Bu çalışma tüm render hassasiyet sorunlarının veya CAD-ADV-07'nin
bittiği anlamına gelmez. Ham ekranlar ve test kayıtları repo dışında
`/tmp/kentos-cad-adv04-2026-11-01/styled-precision-*` altında tutulur.
