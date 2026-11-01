# ADR 0821 — Bağımsız dairesel eğrilerde analitik köşe yuvarlama

2026-11-01. CAD-ADV-04'ün ilk uygulama dilimi; madde bütünü açık.

## Karar

Mevcut düz kenar/yol köşesi çözücüsü korunur. En az bir bağımsız yay veya
daire içeren çift `geometry-core/ops/fillet_curves` çözümüne gider.
`cad.entities.fillet` yeni bir geometri yazma kestirmesi yerine aynı
anlamsal komut olarak kalır. Native araç, Python ve MCP aynı doğrulamayı
ve hesabı kullanır; giriş şeması değiştirilmez.

Doğrunun ±r paralelleri ile dairenin R+r ve |R−r| merkez yerleri
kesiştirilir. Her adayın iki temas noktası, sonlu yay aralığı, kalan uç,
yarıçapı ve yönlü G1 koşulu denetlenir. Seçim noktalarına toplam uzaklık
geçerli dallar arasından seçim yapar. Üçgenleştirme ve ekran pikseli
hesaba karışmaz; büyük koordinatlar yerel başlangıca göre hesaplanır.
Native/WASM ortak matematik fonksiyonları kullanılır.

Bir daire kırpılarak gizlice yaya çevrilmez. Mevcut sonlu yayın dışına
uzatma yapılmaz. Elips/NURBS ve yol içindeki yaylı kenarlar bu yeni
çözücüye verilmez. Pah mevcut düz kenar çözümündedir ve iki bağımsız
mesafe alır. Dairesel yuvarlama tek yarıçaplıdır; asimetrik geçiş
eğrisi onun ikinci bir yarıçapı olarak modellenmez.

## İlişkiler ve önizleme

İki seçili nesne aynı ilişkili taramanın sınırındaysa kırpma sonrasında
eklenen bağlayıcı o taramanın sınır UID listesine katılır. Bu güncelleme
dış anlamsal komut adımı içinde, takipçiler çalışmadan yapılır. Kaynaklar,
bağlayıcı ve tarama tek Undo'dur; kırpmasız ekleme kaynak ilişkiyi değiştirmez.

İlişkili öğe bulunan çizimde iki nesneli önizleme aynı komutu belgenin okuma kopyasında çalıştırır;
ilişkili tarama/diğer takipçi değişiklikleri de önizlemeye katılır.
Önizlemenin yeni nesneleri mevcut sözleşmeye göre geçici `id=0` taşır;
asıl belgeye nesne, UID, revizyon veya geçmiş yazılmaz. Plan kimliğinin
protokol doğrulaması mevcut taze UID normalizasyonunu kullanır.
İlişkili öğe bulunmayan çizimde mevcut doğrudan düzenleme planı kullanılır;
her fare hareketinde büyük çizimin okuma kopyası ve takipçi taraması yapılmaz.

Genişliği olan iki köşeli yolun budanan parçası polyline kalır. Uzatmada
depolanmış uç genişlikleri korunur. Delikli alan Buda/Kır retleri
korunur; kullanıcıya halkaları patlatmak yerine mevcut Alan böl önerilir.

## Kanıt ve kalan kapsam

Çekirdek testleri bağımsız daire kesişim formülleri, teğet normalleri,
iki çözüm dalı, uç dışı tıklama, sıfır yarıçap, ters seçim sırası ve
harita koordinatlarını kapsar. Native testler readonly plan, genişlik,
metadata, ilişkili tarama, save/open, tam Undo/Redo'yu kapsar. Typed Python
ve gerçek MCP Server çağrıları aynı komuttan geçirilir.

Gerçek macOS testinde çizgi–yay, yay–yay ve daire–daire kullanıldı;
100 cm yarıçap sonuçları kaydedilmiş dosyada bağımsız formülle kontrol
edildi. İlişkili tarama örneğinde klavyeden 50 cm girildi; kaydedilen
tarama yeni yay UID'sini tuttu, GUI Undo sonrası bütün kaynak nesneler
ve tarama başlangıçla eşit bulundu. Ekran görüntüleri/test dosyaları repo
dışında `/tmp/kentos-cad-adv04-2026-11-01/` altındadır.

Ortak destek matrisi ve **açık** kabul işleri
[curve-editing-support.md](../cad/curve-editing-support.md) içindedir.
Elips/NURBS offset hata garantisi, eğri sürekliliği arayüzü, bütün
alt nesne ilişkileri ve web etkileşim eşliği tamamlandı sayılmaz.
