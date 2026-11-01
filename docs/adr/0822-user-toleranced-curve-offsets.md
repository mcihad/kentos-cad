# ADR 0822 — Elips ve eğri paralelinde açık yaklaşım ve hata bütçesi

Tarih: 2026-11-01. Kapsam: CAD-ADV-04, native ortak komut; bu dilim doğrulandı. CAD-ADV-04 bütünü halen açıktır.

## Sorun ve ürün kararı

Eski NURBS ötelemesi kaynak eğrinin kirişlerini öteler. Kaynak eğrinin
0,1 mm örnekleme toleransı sonuç paralelinin hata sınırı değildir. Eski
elips notundaki `a*h²/8`, öteleme normalinin değişimini içermez. Bu
sayıların kullanıcıya sonuç garantisi gibi sunulması yanlıştır.

`cad.entities.offset` elips, fit eğrisi ve NURBS için artık açık
`approximation: {tolerance: metre}` ister. Seçenek verilmezse kaynak
korunur, `invalid_tolerance` ve çözümü açıklayan mesaj döner. Düz/analitik
yaylı mevcut ötelemeler bu seçeneği istemez. Eski kayıtlı doğrudan
`cad.entities.edit/offset` geometrisini oynatma yolu değiştirilmez;
`offset_noted.approximate` eski örnekleme tahminidir, garantili sınır değildir.

Araç ayarlarında **Eğri yaklaşımı** varsayılan kapalıdır. Açılınca
**Tolerans** alanı görünür (başlangıç 0,1 mm). Proje metre veya mm
gösterse de açık birimli giriş yapılabilir. **Kaynağı sil** ayrı seçimdir;
yaklaşım onayı kaynağı silme onayı değildir. Önizleme ve yazma aynı
anlamsal komuttan geçer; başarısız önizlemede onaylanmış kopya çizilmez.
Yaklaşık sonuç ve hata sınırı imleçte belirtilir.

## Geometrik yöntem

Elips en çok dört rasyonel ikinci derece Bézier parçasına tam olarak
ayrılır. Fit eğrisi mevcut centripetal Catmull–Rom kübik parçalarıdır.
Pozitif ağırlıklı NURBS'ün düğümleri derece çokluğuna getirilir; her
sıfırdan büyük aralık kendi rasyonel Bézier parçasıdır. Kaynak belgeye
kontrol noktası veya düğüm yazılmaz. İşlem yerel koordinatlarla yürür,
ağırlıklar ortak ölçekte normalize edilir. Bu araştırmada ortaya çıkan
açık sağ uçta düğüm ekleme hatası `Nurbs::insert` içinde düzeltildi:
uç değerini okumanın sol aralığı ile eklemenin sağ düğümü farklıdır.

Her parçada `r=A/W`, `W>0`, `q=r+dN` kullanılır. Bernstein katsayılarının
konveks zarfından ağırlık, türev ve hız sınırları hesaplanır. `V=A'W−AW'`
katsayılarının uç kiriş yönündeki en küçük izdüşümü pozitifse hız alt
sınırını verir. Rasyonel türev üst sınırları bölüm türevlerinden çıkar:

```
B1 = (A1 + W1*B0) / Wmin
B2 = (A2 + 2*W1*B1 + W2*B0) / Wmin
B3 = (A3 + 3*W1*B2 + 3*W2*B1 + W3*B0) / Wmin
M  = B2 + |d|*(2*B3/vmin + 6*B2²/vmin²)
```

Parçanın normalize `[0,1]` aralığında kiriş enterpolasyon hatası `M/8`
ile sınırlanır. Sağlanmıyorsa homojen de Casteljau ile ikiye bölünür;
orta nokta örneklerinin uygun görünmesi kabul kriteri değildir.
`cross(V,V')` Bernstein katsayılarından eğrilik aralığı ve
`1−d*kappa` sınırı kontrol edilir. Sıfır hız, içe katlanma, birleşimde
ayrılan normaller ve kapalı eğride uyuşmayan uçlar hayali kirişle bağlanmaz.

Hata raporu türev sınırına koordinat büyüklüğü, derece ve ağırlık oranına
bağlı muhafazakâr f64 payını ekler. Yuvarlama denetimleri tam yönlü
aralık aritmetiğiyle yapılmış bir IEEE kanıtı değildir; mühendislik sınırı
bağımsız analitik eğriler ve yoğun denetimlerle doğrulanır. Ağırlık oranı
veya koordinat hassasiyeti güvenlik payını tüketiyorsa sonuç reddedilir;
istenen tolerans sessizce büyütülmez. Ekran LOD'u bu hesaba girmez.

## Rapor, güvenli ret ve otomasyon

Plan/yazma `approximations` dizisinde kaynak UID, kopyanın sonuç dizinindeki
`copyIndex` değeri, istenen tolerans, `errorBound`, köşe ve kendi kesişim
sayıları döner. Bu bir kaynak izidir; kaynağın sonraki değişikliklerini
izleyen ilişkisel kısıt değildir. Plan kimliği tolerans ve sonucu bağlar.
Tek adım kaynak silme dahil Undo/Redo'yu kapsar. Yaklaşık bir kaynakta
başarısızlık, karma toplu istekte hazırlanmış diğer kopyaları da yazmaz.

En çok 2.048 kaynak kontrol noktası, derece 11, 16.384 sonuç noktası,
24 bölme derinliği ve sınırlı aday kesişim işi uygulanır. Sınırda uyarı
verip eksik sonuç yazılmaz. Bütün komşu olmayan kiriş çiftleri sıralı
kutularla denetlenir; 4.000 noktayı geçince “kesişim yok” dönülmez.
Kendi kesişimi varsa açık uyarı/rapor gelir; döngüler sessizce budanmaz.
Noktadan geç mesafesi görüntü kirişlerinden değil analitik eğriden okunur;
açık uç normalinin dışında seçilen ve paralelin ulaşamadığı nokta reddedilir.

Python tipleri ve MCP şeması aynı komut sözleşmesinden üretilir. Kullanıcı
belgesindeki birim dönüşümünü otomasyon istemcisi tahmin etmez; tel metre
kullanır. Eski komut kaydında onaysız elips/NURBS isteği varsa istemci
artık açık tolerans göndermelidir; bu davranış düzeltmesi bilinçlidir.

## Birincil kaynaklar ve doğrulama

- [QCAD Offset öğreticisi](https://www.qcad.org/en/tutorials/offsetting/):
  elips paralelinin elips olmadığı, eğri/contour ve noktadan geç kullanımının
  önizleme ve görünen seçeneklerle sunulması incelendi. KentOS'un sonuç
  biçimi ve tolerans algoritması bu koddan alınmadı.
- [OCCT Geom2d_OffsetCurve](https://www.occt3d.com/dev/doc/refman/html/class_geom2d___offset_curve.html):
  normalin tekil olmaması, temel eğri sürekliliği, değişmeyen kaynak
  kopyası ve kendi kesişimlerinin otomatik silinmemesi incelendi.

Geometri testleri rasyonel çeyrek dairenin bağımsız `R±d` paraleli,
dönük elipsin analitik normal noktaları, ağırlıklı/açık düğümlü çok
parçalı NURBS, kapalı fit eğrisi, çökme, C0 birleşim, büyük koordinat ve
5.000 kenarlı denetimi kapsar. Native/interaction/Python/MCP plan, yazma,
ret ve Undo kontrolleri geçti. Tüm geometri testleri 561 başarılı/6 atlanan;
son yedi bounded testi ayrıca tekrarlandı. Native kütüphane 93 başarılı,
entegrasyonda diğer 269 test başarılı; ortak durum okuyucusunun yeni
`approximation.tolerance` sonlu-olmayan değer yolu düzeltildikten sonra
37 offset durumu ve kod kapsamı ayrıca geçti. Interaction 71 kütüphane,
847 entegrasyon testi başarılı (2 atlanan). Masaüstünde diğer 1.567 test
başarılı; yeni Yaklaşım seçeneğine güncellenen ortak fare/klavye izleri
ayrıca bütün varyantlarda geçti (256 atlanan). Clippy lib kontrolü,
altı TS sözleşme ihracı, SDK güncelliği, beş Python testi ve gerçek MCP
Server testi geçti. Web envanteri/testleri ADR 0228 gereği çalıştırılmaz.

Gerçek macOS GUI'de 20 cm içe öteleme, 0,01 mm tolerans, 60 cm'de içe
katlanma reddi, ret sonrası yeniden seçmeden ayar düzeltme, tek Undo ve
kaydet–aç kontrol edildi. 2.520 köşeli dosya typed Python planıyla aynı;
bağımsız analitik elips normalinin uzaklığı ve yoğun paralel/kiriş
hatası rapor sınırıyla denetlendi. Kayıtlar/görseller repo dışında
`/tmp/kentos-cad-adv04-2026-11-01/` altındadır (15–21 numaralı görseller,
`offset-gui-audit.txt`, `bounded-*.log`). Sabit mesafede aynı yöndeki
fare hareketleri, belge/ayar/taraf değişene kadar planı tekrar kullanır;
noktadan geç ve iki yana isteğinin tam nokta anahtarı korunur.
Bu belge CAD-ADV-04'ün diğer açık kabul işlerini tamamlandı saymaz.
