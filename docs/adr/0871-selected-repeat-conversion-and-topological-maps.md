# ADR 0871 — Seçilmiş tekrarların bağlarıyla bloğa dönüştürülmesi

Tarih: 3 Kasım 2026. CAD-ADV-07'nin seçilmiş aday dönüşümü; ilişkili dizi ve
metin/iç blok/NURBS tanıması ayrı açık kapsamlardır.

## Dayanak

[BricsCAD V27 BLOCKIFY](https://help.bricsys.com/en-us/document/command-reference/b/blockify-command?version=V27)
aday seçimi, örtüşen kümelerin çözülmesi ve parametrik ilişkilerin alt nesnelere
yeniden eşlenmesini ele alır.
[AutoCAD for Mac 2027 Detect and Convert](https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-MAC-WhatsNew/files/GUID-0876F8BC-9ED5-4EBB-A19C-FF019BAA3510.htm)
tanıma, aday incelemesi ve dönüşümü ayrı kullanıcı eylemleri olarak sunar.
KentOS kararı: bir arama eşleşmesi, kök veri ve ilişkilerini kaybetme izni değildir.

## Ortak işlem

`cad.blocks.convert` kaynak, açıkça seçilmiş üye grupları, ad, dönüşüm ölçütleri,
mutlak metre toleransı ve beklenen revizyonu alır. İstemciden dönüşüm matrisi
alınmaz. Aynı analitik arama, seçilmiş kimlikleri ve katman/görünüş eşitliğini
yeniden doğrular. Her grup tam doğrulanmalıdır; kısmi taramada bulunamamış
bir grup varsayımla kabul edilmez. Nesne paylaşan gruplar atomik reddedilir.

Kaynak üyeler belge çizim sırasıyla tanıma girer. Tanımın yerel kimliği
1 tabanlı, ölçü yolunun parça sırası 0 tabanlıdır. Her adayın ilk kök nesnesi,
kendi UUID/slot ve çizim yerinde bir Insert olur; diğer köklerin karşılıkları
`mapping: from → insert + path` ile açıklanır. Kaynak seçilmiş gruplarda yoksa
olduğu yerde kalır. Tanım `units=m` kullanır; model metre koordinatları,
proje gösterim birimi veya çizim ölçeği yeniden ölçeklenmez.

Native plan tanımı, yerleştirmeleri, silinecek kökleri, kimlik eşlemesini,
ölçü noktalarının eski/yeni bağlarını ve gerçek nokta sapmalarını verir.
Her yazma aynı ön denetimi tekrarlar; kayıtlı tek işlem/Undo adımıdır.
Protokol planının özeti ve plan kimliğiyle korunan yürütme de ortak komut
yolundadır. Python SDK ve MCP katalogdan bu sözleşmeyi kullanır.

## Ölçü topolojisi

- Hesap, yeni bloğun gerçekten yerleştirilmiş **dünya geometrisinde** yapılır.
  Çemberin dünya doğusundan ölçülen oranına ikinci bir ters açı dönüşümü
  uygulanmaz; aynalanmış yayın kanonik başlangıç/sonu tekrar ters çevrilmez.
- Ters çizgide Start/End değişir; Along oranı `1−t`, orta nokta aynı kalır.
- Çoklu çizginin analitik bulge/genişlik eşlemesi bütün döngüsel ve ters
  sıralamaları sınar. Eski köşe ve kenar doğru tanım sırasına gider; ters
  kenarda oran da döner. En yakın noktaya bakarak köşe tahmin edilmez.
- Bütün tam geometrik karşılıklar aynı bağ yeri üzerinde anlaşmalıdır.
  Aynı yolu birden çok dolaşan çizgide farklı kenar kimlikleri mümkünse
  dönüşüm reddedilir. İş sınırında yalnız ilk karşılık bulunması yeterli değildir.
- Yalnız canlı, kök nesneye bağlı, bağımsız nokta yerleri taşıyan sıradan
  ölçüler ilk kapsamda taşınır. Türetilmiş Curve/Edges/Pair/Cross kuralı,
  mevcut kopuk bağ veya beklenmeyen alt yol açık ret alır.
- Kayıtlı ölçü noktası kaynak bağıyla güncel değilse veya yeni yer seçilen
  toleranstan fazla saparsa nokta sessizce taşınmaz. Plan sapması, yalnız
  eski geometriyi değil, kullanıcının kayıtlı ölçü noktasını esas alır.

## Veri ve görünüş sınırı

İlk kapsam, tek görünür ve dondurulmamış, tür/şema/veri davranışı olmayan
CAD katmanındaki desteklenen analitik 2B nesnelerdir. Kök öznitelik, etiket,
sembol, ek dosya, kural tabanlı görünüş ve kesikli/özel çizgi fazı henüz
korunamadığından açıkça reddedilir. Araya başka nesne giren veya parça çizim
sırası değişen kümeler de blokla aynı üst üste çizme sırasına güvenilerek kabul edilmez.

Bağ denetimi `copy_links` sayımıyla sınırlanmaz. Belge başlığındaki kısıt,
dizi, ölçü tarifi, modül/seçenek, grup, pafta ve ayar kayıtları; diğer köklerin
özellikleri taranır. UUID'nin ifade içinde veya nesne anahtarında bulunması
da dikkate alınır. Yalnız açıkça eşlenen ölçü noktaları bu taramadan çıkarılır;
diğer ilişkiler tutulur. Katman adı/kimliğiyle tanımlanan form, feed, şema ve
join davranışları için katman/ayar kaydı denetimi de vardır. Tarihî istek
günlüğü ve belgenin kendi kimlik listesi canlı ilişki sayılmaz.

Çizimde tablo bulunması ilk dilimde muhafazakâr ret verir: bütün çizimin
nesne türünü/sayısını sorgulayan tablo hiçbir kök UUID vermeden de dönüşümden
etkilenebilir. Bu sınır ilişki desteği tamamlanmış gibi sunulmaz; sonraki
dilimde tablonun sorgu türüne göre daha dar ve açık bir eşleme gerekir.

## Arayüz ve çalışma sınırları

Bloklar panelinde işaretlenmiş adaylar için **Bloğa dönüştür…**, ayrı inceleme
penceresini açar. Ad, kök→blok sayısı, korunacak ölçü noktaları, nokta sapması
ve okunur ilk parça eşlemeleri görünür; tam inceleme kaydı kopyalanabilir.
Ad değişince eski inceleme geçersizleşir. Kabulden önce çizim değişmez.

İnceleme ve yazma hesabı arka plandadır. Durdur/Esc pencereyi bırakırken
işi iptal eder; geç gelen sonuç kullanılmaz. Yazan iş mevcut native Job
ile hesaplanır ve yalnız aynı belge oturumu/revizyonu/nesli hâlâ geçerliyse
kısa commit aşamasına geçer. Protokolün iptal ve atomiklik davranışı korunur.
Kaynak 128, seçilmiş grup 1 000, ilişki denetimi yapılan belge 50 000 nesneyle;
eşleme işi ve ilişki taraması ayrıca sınırlandırılır. Eksik denetim başarı değildir.

## Doğrulama kaydı

Çekirdek testleri ters çizgi, açık/kapalı yaylı/genişlikli yol, döngüsel
köşe, çoklu dolaşım belirsizliği ve durdurulan eşleme davranışını sınar.
Native kabul, bağımsız çizgi/çember kümelerinde kimlik/yerel slot/parça yolu,
ters uç bağı, taşıma takibi, kapalı son yay kenarı, görünüş/veri/çakışma/ilişki
retleri ve tek Undo/Redo snapshot eşitliğidir. Ortak komut senaryoları bağımsız
betikten üretilir. Son GUI, Python/MCP ve derleme sonuçları yalnız çalıştırılıp
doğrulandıkça CAD planına eklenir; bu karar dosyası tek başına kabul kanıtı değildir.
