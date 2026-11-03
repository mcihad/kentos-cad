# 0912 — Salt okunur sınır teşhisi ve tarama düzenleme kabulü

Tarih: 3 Kasım 2026. Kapsam: CAD-ADV-08.

## Karar

`cad.drawing.inspect` geometriyi değiştirmeyen, iptal edilebilir bir sorgudur.
Görünür çizim, seçili nesneler veya etkin katman incelenir. Sınır açıklıkları,
açık uçlar, yinelenen kenarlar, kesişmeler ve teğet temaslar kalıcı UUID ve dünya
koordinatıyla raporlanır. Kesişme tek başına hata sayılmaz; bir yüz birleşimi de
olabilir. Boşluk eşiği yalnız teşhis içindir, geometriyi kapatmaz.

CAD denetimi paneli KentOS UI'ın Select, NumberInput, Table ve onay bileşenlerini
kullanır. Bulgu seçildiğinde sahne işareti ve yakınlaştırma gösterilir; ham
geometriye denetim nesnesi eklenmez. Oturum/sürüm değişince eski sonuç onarım
başlatamaz. Çalışan sorgu durdurulabilir. Sonuç, karşılaştırma ve kaynak sınırına
ulaşınca kısmi olduğu açıkça belirtilir.

Taramanın ilişkisiz olması, kaynağının eksik/açık olması ve mevcut kaynaklarını
izlemesi ayrı raporlanır. Sonuncusu bütün bölgenin topolojik olarak doğru olduğu
iddiası değildir. Geçmiş ilişki kopmasının nedeni veri yokken uydurulmaz.
Kaynak değişikliği ilişkiyi bırakırsa ortak işlem raporu uyarı üretir; açıkça
İlişkiyi kopar isteği bu uyarıya dönüştürülmez.

Denetimden onarım her seferinde Yeniden seç kipine girer; önceki oturumdaki
İlişkiyi kopar kipi yanlışlıkla çalıştırılmaz. Sınır yöntemi, adalar, ilişki,
yazı boşluğu ve çizgiler yönteminin boşluk eşiği görünür seçeneklerdir.

`cad.hatches.edit`'in ek `origin` işlemi yalnız desen başlangıcını taşır.
Nokta/dünya/köşe kuralları mevcut tarama oluşturucusunun kurallarıdır; dünya
başlangıcının kanonik kaydı `None`'dır. Dolu/degrade tarama reddedilir, karışık
bir toplu istek bütünüyle reddedilir. Desen tanımı, açı, ölçek, analitik sınır,
kimlikler ve ilişkiler korunur. Masaüstünde Taramayı düzenle › Başlangıcı taşı
(N), nokta veya çizimin biriminde koordinat kabul eder; bir işlem bir Undo'dur.
Python'un `OriginHatchEdit` türü ve MCP aynı ürün komutunu kullanır.

Gerçek budama kabulü, taramanın üstteki kenarının alttaki kaynak daireyi
seçtirmediğini ortaya çıkardı. Buda/Uzat hedefi, üst üste kenarlar arasında
önce tarama olmayan düzenlenebilir kaynağı arar. Tek başına tarama için mevcut
ret mesajı korunur; kilit ve seçim süzgeci atlanmaz. Kaynak daire açık yaya
dönüşünce tarama donmuş geometrisiyle kalır, bağı bırakılır; Undo ikisini birlikte
geri getirir.

## Alan doğruluğu ve sınırlar

Tarama alanı artık ekranın örneklenmiş `ring`'inden değil, varsa kayıtlı analitik
sınırından ölçülür. Doğru/dairesel yay ve tam elips alanları analitiktir. Karışık
elips/NURBS sınırındaki mevcut kord yaklaşımı bu değişiklikle kesinleştirilmiş
sayılmaz. Tam topoloji teşhisi doğru/dairesel yay kapsamındadır; blok, spline,
elips ve NURBS kaynakları destek dışı listesinde açıkça görünür.

En çok 50 000 kapsam nesnesi, 20 000 kenar, 2 milyon iş adımı ve 1000 bulgu vardır.
Büyük path'ler çoğaltılmadan önce sayılır. Sınırı aşan çizim temiz kabul edilmez.

## Bu oturumdaki kanıt

- Geometri: 316 geçti, 21 bilinçli ignored. Uygulama: birleşim öncesi 433 geçti,
  37 ignored; güncel main ve yeni origin işlemiyle 10 denetim testi geçti.
- Etkileşim: 17 tarama testi; 26 düzenleme regresyon testi geçti. Yeni test,
  üstteki taramanın altındaki daireyi gerçek oturum olaylarıyla budar; analitik
  sınır/desen korunur ve tek Undo bütün çizimi geri getirir.
- Python: 3 test geçti. MCP: gerçek `Server` üzerinden 2 test geçti. Katalog 4,
  TypeScript bağlama 1254, masaüstü denetim/önizleme 2 test geçti.
- Bağımsız alan referansı: dış yarıçap 10, iki ada yarıçapı 1 için `98π`.
  Kaynak bölme testinde alanlar `3+π/2` ve `4+π/2`; başlangıç/açı/ölçek ve
  kaynak parçasının kimliği korunur.
- Gerçek uygulamada iki ada, KENTOS yazı boşluğu ve 4 mm açıklık çizimi açıldı.
  Yakın/uzak görünüm, vektör PDF, kaynak silme, ilişki kopar/yeniden bağla ve
  tek Undo karşılaştırıldı. Tarama açısı 30→45°, ölçeği 1→2 ve başlangıcı
  (123,456)→(1000,2000) mm düzenlendi; her biri tek Undo ile geri geldi.
- Son derlemede fareyle daire budandı: panel 0 ilişkili/1 ilişkisiz gösterdi.
  Tek Undo ve yeniden denetim 1 ilişkili/0 ilişkisiz gösterdi. Tek başına
  budama alanı/deseni veya yazı/ada boşluklarını silmedi.
- Ham ekran görüntüleri ve PDF depoya eklenmez; yerel kanıt dizini
  `/tmp/kentos-cad-adv08-09-2026-11-03/`. Başlıca kayıtlar:
  `inspection-origin-menu-final.png`, `inspection-origin-coordinate-gui.png`,
  `inspection-origin-undo-gui.png`, `inspection-trim-detached-final.png`,
  `inspection-trim-undo-final.png`, `inspection-acceptance-far.png`,
  `inspection-acceptance-zoom-return.png`, `hatch-acceptance-pdf.png`.

## Başvurular

- [AutoCAD 2026 HATCH](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-410ECEBF-7CC2-4000-A45E-18F1F6BEE423.htm): sınır, adalar ve desen başlangıcı.
- [AutoCAD 2026 ortak tarama seçenekleri](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-006BE8D3-0731-47BA-A7CF-1B50282CD108.htm): sınırı yeniden oluşturma ve başlangıcı koruma.
- [BricsCAD V26 -HATCH](https://helpcenter.bricsys.com/en-us/document/command-reference/h/-hatch-command?version=V26): boşluk toleransı, ada ve ilişki seçeneklerinin ayrı anlamları.
