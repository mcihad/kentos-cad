# ADR 0867 — Yaylı ve genişlikli çoklu çizgileri yapısını koruyarak tanı

Tarih: 2026-11-03. CAD-ADV-07'nin kaynak türü genişletmesidir; bloklaştırma
ve ilişkili diziye dönüştürme tamamlanmış sayılmaz. ADR 0866'nın sorgusunu
genişletir, ikinci bir arama veya geometri modeli kurmaz.

## Dayanak ve kod incelemesi

[AutoCAD 2027 Edit Polylines](https://help.autodesk.com/cloudhelp/2027/ENU/OARX-DevGuide-Managed/files/GUID-7B4A7B69-8780-4F8E-A1C6-A195CA1F122E.htm)
çoklu çizginin kapalı durumunu, bulge ve kenar genişliğini ayrı özellikler
olarak tanımlar. [Mac PEDIT 2026](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-MAC-Core/files/GUID-648BDC21-E835-478F-A318-9D201479998D.htm)
açma/kapamanın son kenarı değiştirdiğini açıklar.
[PLINEREVERSEWIDTHS](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-Core/files/GUID-9E0951E0-1917-4309-85C6-96A9ACFC04CF.htm)
yön tersine çevrilirken kenar genişliklerinin davranışının ayrıca denetlenmesi
gerektiğini gösterir. KentOS çıkarımı: aynı köşeler ve kutu, aynı geometriyi
kanıtlamaz; daralan kenarın iki uç genişliği de eşlenmelidir.

KentOS `PathEntity`, geometri dışında Z, M ve mahal ayırıcı rolü taşır.
`Shape::Polyline` bu verileri taşımaz. Native uyarlayıcı bunları sessizce
bırakıp 2B eşleşme üretemez. `transform_shape` bulge işaretini aynada
çevirir ve genişliği uzunluk olarak ölçekler; bu mevcut davranış korunur.
Poligon ayrı türdür; kapalı çizgiye dönüştürülmez.
[BricsCAD V27 BLOCKIFY](https://help.bricsys.com/en-us/document/command-reference/b/blockify-command?version=V27)
geometrik karşılaştırma ile parametrik bağları koruyarak dönüştürmeyi ayırır.
Bu teslim de yalnız aday bulur ve UUID eşlemesini döndürür.

## Kapsam ve eşitlik

- Tek parçalı, deliksiz 2B çoklu çizgi; açık veya kapalı, analitik yaylı
  kenarlar ve değişken başlangıç/bitiş genişlikleri desteklenir. Kaynakta
  desteklenmeyen tek öğe bütün kaynağı reddettirir.
- Z/M değeri, mahal ayırıcı rolü, delik ve ek parçalar desteklenmez. Bunlar
  `geometryOnly` açıkken de sıradan 2B çizgi sayılmaz. Hedefte bu tür öğe
  atlanabilir; kaynakta açık `recognition_unsupported` döner.
- Köşe sayısı ve açık/kapalı durum aynıdır. Kapalı çizgide başlangıç köşesi
  döngüsel kayabilir; açık ve kapalı çizgide yön ters olabilir. Yeniden
  parçalanmış veya fazladan köşe eklenmiş bir çizgiyi sadeleştirerek aynı
  sayma bu teslimde yoktur. Nesnelerin yapısı değiştirilmez.
- Her köşe ve karşılık gelen kenar analitik doğrulanır. Yayın merkezi,
  yarıçapı, işaretli süpürmesi ve orta noktası denetlenir. Yalnız örneklenmiş
  benzerlik yeterli değildir. Ters yönde bulge işareti ve kenarın başlangıç/
  bitiş genişlikleri beraber çevrilir. Yanlış genişlik veya farklı yay,
  aynı köşelere sahip olsa da eşleşmez.
- Kaynak çerçevesi bir gerçek kenardan kurulur. Hedefin iki yöndeki kenarları
  dönüşüm adayıdır. Köşe ortalaması uzamsal indeksi daraltır; son kararı
  vermez. Çizgi/çember/yay ile karma koleksiyon aynı bire bir eşlemeyi kullanır.
- Float64, kaynakta yerelleştirme ve mutlak metre toleransı korunur. Proje
  birimi, gösterim ölçeği veya çizim motoru değiştirilmez.

## İş bütçesi ve yüzeyler

Bir çoklu çizgi en çok 4 096, kaynak ve arama birlikte toplam en çok
200 000 çoklu çizgi köşesidir. İlk sınır desteklenmeyen kaynak, toplam sınır
kapsamı daraltma hatasıdır. Hazırlık da sınırlıdır; döngüsel/ters karşılaştırma
her köşe/kenarda iptal ve iş bütçesini kontrol eder. Dönüşüm hazırlığındaki
köşe işi de sayaca eklenir. Sınırda kısmi sonuç açıkça raporlanır.

GUI aynı görünür ayarları ve aday panelini kullanır. Komut kataloğu,
yetenek açıklaması, Python dokümanı ve MCP aynı destek sınırını anlatır.
Python doğrudan sorgu, plan ve arka plan işi aynı kimlikleri üretir.

## Kabul

Bağımsız matematikle oluşturulan 40 döndürülmüş/aynalanmış, başlangıç köşesi
ve yönü farklı, yaylı ve genişlikli çizgi aranır. İki kat ölçekli örnek
yalnız ölçek seçeneğiyle bulunur. Farklı yay, farklı daralma ve açık çizgi
birleşmez. Harita koordinatları çekirdek testindedir. Karma çizgi/çoklu
çizgi kaynağında UUID sırası; toplam köşe sınırında yazısız ret; public
Python/MCP planı ve bağlı 2 m ölçünün değişmezliği ayrıca sınanır.
Gerçek GUI kabulü ve sonuç sayıları planın ilerleme kaydına yazılır;
ham görüntüler depoya eklenmez.

Bu eşleşme, kapalı çizginin köşe sıralamasını değiştirme yetkisi değildir.
Gelecek bloklaştırma, bağlı `Vertex`/`Mid`/`Along` yerlerini aynı indeks diye
kopyalamamalıdır; döngüsel/ters karşılığını ayrıca üretip doğrulamalıdır.
