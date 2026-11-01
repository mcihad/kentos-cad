# 0832 — Açık NURBS uçlarını mevcut kısıt sistemine bağlama

2026-11-02. Durum: uygulandı; otomatik ve gerçek GUI kabulü doğrulandı. CAD-ADV-05 açık.

## Kodda bulunan boşluk

CAD-ADV-04 gerçek NURBS ölçü başvurularını ve eğri uçlarını düzenlemeyi
sağlar. Ancak `ops/constraints/model.rs` NURBS için bilinmeyen ayırmıyordu;
Start/End başvuruları kısıt motorunda parça sayılmıyordu. Yeni çekirdek
kabulü önce bunu gösterdi: 6→8 m uç ölçüsü geometriyi değiştirmedi ve
NURBS uç serbestlikleri modellenmedi. Ortak `SubRef` uçları zaten
parametrik eğriden doğru hesaplar; yeni bir uç seçme yaklaşımı kurulmaz.

## Sınırlı ve açık adaptör

İlk kapsam, geçerli, açık, periyodik olmayan ve iki ucunda tam olarak
`degree+1` tekrar ile bağlanmış (clamped) NURBS'tür. Domain, düğüm sayısı,
ağırlıklar ve sonlu kontrol koordinatları denetlenir. Uçtaki tekrar
fazlası veya eksikliği uygun sayılmaz. Bu koşullarda gerçek Start/End,
ilk/son kontrol noktasının kendisidir; genel NURBS'te bu varsayılmaz.

Çözücü yalnız bu iki ucun dört koordinatını bilinmeyen yapar.
İç kontrol noktaları, derece, düğümler, ağırlıklar ve eğrinin açık/periyodik
bayrakları kaynak sabitleridir; ölçüler poliline veya örneklenmiş eğriye
uygulanmaz. Gösterilen serbestlik bu uç adaptörünün serbestliğidir.
Bu aşama iç kontrol noktası kısıtı, teğet veya G2 çözümü sunmaz.

Mevcut Point kuralları kullanılır: Sabit, Çakışık, iki uç Yatay/Düşey,
mesafe ve nokta simetrisi. Uç grip hedefi yalnız kendi iki koordinatını
tutar; diğer uç gerektiğinde çözülür. Uç kimliği mevcut Start/End'dir;
kamu komutu, kalıcı başvuru, paket veya dosya şeması eklenmez.

Gerçek değişimde eski fit noktaları, fit toleransı ve fit teğetleri
geçersiz hale geldiği için temizlenir (ADR 0259'un kaynak kuralı).
Değişiklik yoksa metadata korunur. İç kontrol noktaları bit düzeyinde
aynı kalır; çözücüye sokulup yerel koordinattan yeniden yazılmaz.
Undo hem geometriyi hem eski fit bilgisini geri getirir.

Araç, NURBS ucu seçildiğinde “yalnız uçlar değişir; iç noktalar korunur”
notunu gösterir; seçim geri alınınca not kalkar. Destek dışı parçaya
NURBS bağlamaya çalışılırsa araç ve ürün komutu açıklamalı ret verir.
Kilit, çelişki ve hata halinde son geçerli çizim atomik korunur.
Parametre penceresi, kısıtlı NURBS varsa serbestlik sayısının yalnız
uç modeline ait olduğunu ve iç kontrol noktalarının düzenlenebilir
kaldığını başlığın hemen altında açıklar. Salt NURBS bulunması bu notu
getirmez; NURBS gerçekten bir bağda yer almalıdır.

## Araştırma

[AutoCAD GEOMCONSTRAINT](https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-Core/files/GUID-D79452FB-DDA7-44C0-A9EA-F4EA656022D2.htm)
spline için kısıt noktalarını uçlar olarak listeler.
[AutoCAD spline düzenleme](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-MAC-Core/files/GUID-F0371668-7F1E-4A3C-80B7-781DDD67D023.htm)
fit noktası, kontrol noktası ve uç teğeti işlemlerini ayırır.
[OCCT Geom_BSplineCurve](https://occt3d.com/dev/doc/refman/html/class_geom___b_spline_curve.html)
ilk/son eğri noktasının ilk/son pole ile her zaman aynı olmadığını
özellikle belirtir; uç düğüm tekrarını ve periyodik domaini belgeler.
KentOS'un muhafazakâr clamp denetimi ve dört bilinmeyenli sınırı kendi
kod modeline ilişkin karardır; başka yazılımın iç çözücüsünü kopyalama
veya genel NURBS çözümü eşdeğerliği iddiası değildir.


## Kabul kanıtı

Çekirdek: 1/2/3/7. derece, pozitif ve eşit olmayan ağırlıklar, değişken
knot aralığı, büyük dünya koordinatı, gerçek domain uçları, tek uç
geçici tutuşu ve fit bilgisinin korunma/temizlenme kuralları doğrulandı.
Eksik/fazla uç tekrarı, kapalı/periyodik eğri ve genel Circle/Segment
başvurusu desteklenmiş gibi kabul edilmez. Bütün çekirdek hedeflerinde
894 test başarılı.

Yerel ürünün 289 testi, etkileşimin 71+869 testi, Python ve MCP kabulü
başarılı. mm projede `6m`→`8m`, kaynak sabitlerinin/kimliklerin korunması,
izleyen çizgi, planın salt okunurluğu, dosyada roundtrip, kilitli atomik
ret ve tek Undo denetlendi. Masaüstünde 1578 test başarılı; NURBS kapsam
notunun yalnız gerçekten bağlı NURBS'te görünmesi ve Undo sonrası
kalkması da testte. İlgili bütün hedefler Clippy, uygulama derlemesi ve
bağımlılık yönü denetimi temiz (51 crate/67 hedef).

Gerçek masaüstünde ham açık rasyonel NURBS üzerinde Sabit, Yatay,
Uzaklık (tek Enter ile d1=6 m), Çakışık bağları fareyle kuruldu.
Parametre 8m yapıldı: ilk/iç kontrol noktaları, derece, düğümler,
ağırlıklar, nesne ve bağ kimlikleri aynı; yalnız son kontrol noktası
(6,0)→(8,0), çizgi başlangıcı (8,0), bitişi (10,0). Dört bağ tutuyor;
kalan iki serbestlik çizginin bitişindedir. İç kontrol noktalarının bu
sayıya dahil olmadığı notu hem 1440 hem 1100 mantıksal piksel pencerede
okunuyor. Kaydedilmiş bütün nesneler/kimlikler/parametreler/bağlar/teşhis,
aynı başlangıca Python'un uyguladığı komutla birebir eşleşti.
Tek görünür Undo geometri ve fit bilgisini 6 m başlangıcına geri getirdi.

Ayrı kapalı-bayraklı dosyada gerçek uç tıklaması açıklamalı reddedildi;
komut geçmişi “bu parça destek dışında; çizim değiştirilmedi” dedi,
belge değişti işareti ve yeni bağ oluşmadı. Ekranlar 78–84 ve bağımsız
`nurbs-gui-acceptance.json` raporu repo dışında
`/tmp/kentos-cad-adv04-2026-11-01/` altında tutulur.
