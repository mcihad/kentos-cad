# ADR 0607 — Çok sayfalı 2B mimari pafta takımı

Tarih: 23 Ekim 2026. CAD-SHEET-17.

## Mevcut üretim yolu

Pafta takımı projenin tek `SheetBook`'udur. Kat planı, görünüş, kesit ve detay için farklı adlı model bölgeleri (`NamedView`) hazırlanır; her pafta `viewLink` veya sabit capture ile kendi model bölgesini gösterir. Pafta ekleme/yeniden adlandırma/sıralama ve kaynak seçimi önceki ortak işlemlerle masaüstü, Python ve AI'dan yapılır. Sayfa sırası kitapta tutulur; tek PDF bütün seçilen paftaları bu sırayla içerir. `.kcad` ve `.kpafta` kitap, ana sayfa ve kaynak bağlarını korur.

Bu madde 2B çizimlerin pafta takımı olarak hazırlanmasıdır. Kesit ve görünüşün 3B bina modelinden otomatik türetilmesi ADR 0347 §3'teki ayrı model/tasarım kapsamındadır; bu kabulde elle hazırlanmış 2B görünüş/kesit kaynakları kullanılır. Pafta numarası, revizyon, durum ve teslim paketi CAD-SHEET-26/27'de genişletilir.

## Sayfaya özel ana sayfa haritası

Ana sayfanın harita kimliği kitapta tek kalır; farklı kâğıtların kısıtlarıyla farklı çerçeve ve model kapsamı üretebilir. Önceki PDF toplama kodu içeriği yalnız harita `item` kimliğiyle tekilleştiriyordu. Küçük sayfa önce gelince büyük sayfanın modelde daha geniş alanı görünse bile ilk dar kapsamdan toplanmış vektör/yazılar kullanılıyordu. Resim haritalarının PDF kaynak anahtarı da aynı hatayı taşıyordu.

`PdfMap.sheet` isteğe bağlı çıktı bağlamıdır. Masaüstü ve native PDF her `(sheet,item)` için doğru içerik toplar. PDF yazıcısı atlas sayfasının içeriğini, sonra tam sayfaya ait içeriği, sonra eski `sheet` alanı olmayan ortak içeriği arar. Eski `PdfInputs` JSON'u aynı şekilde çalışır; atlasın mevcut ayrı sayfa içeriği önceliğini korur. Raster kaynak anahtarı `(sheet,atlasPage,item)` olur. Font alt kümesi kullanımı ve GeoPDF köşe girdisi de aynı sayfaya ait haritadan alınır. Kaynak model nesneleri veya kalıcı harita kimliği çoğaltılmaz; bu yalnız çıktı içeriğinin doğru sayfaya bağlanmasıdır.

## Kabul

KentOS kullanmayan `multipage_set_cases.py`, beş fiziksel payda × dört dönüş açısı ile 20 takım üretir. Her takım altı sayfadır: aynı ana sayfayı kullanan A4/A3, kat planı, görünüş, kesit ve detay. Dört mimari kaynak ayrı model bölgelerinde elle çizilmiş 2B duvar/oda bölmeleri, merdiven çizgileri, cephe/çatı/pencere, döşeme/kesit ve birleşim detaylarından oluşur. Canlı adlı görünüm bağları ve sayfa serileri aynı kitaptadır; bazı durumlar 500.000/4.500.000 m arazi koordinatındadır.

Bağımsız okuyucu Poppler ile küçük ana sayfa haritasında `FAR` nesnesinin bulunmadığı, büyükte bulunduğu; dört mimari sayfada kendi model yazısının geldiği ve A4/A3 fiziksel ölçüleri denetlenir. Kaynak snapshot'ı değişmez. Ayrı raster regresyonunda aynı master kimliğinin küçük sayfası kırmızı, büyük sayfası mavi resim taşır; gerçek PDF iki sayfa olarak rasterize edilip iç pikselleri bağımsız okunur. Vektör ve resim için yalnız ilk sayfanın içeriğini yeniden kullanmak bu kontrolleri geçemez.

Görsel kontrolde yalnız ana sayfadan içerik alan paftada “Bu paftada öğe yok” yazısının yanıltıcı kaldığı görüldü; yerel öğe bulunmadığı ve hangi ana sayfadan kaç öğe geldiği açıklanır. Masaüstü aynı altı sayfalı PDF'yi kendi çıktı yolundan üretir; ortak ana sayfa, sayfa sekmeleri ve plan/kesit/detay görünümleri açık/koyu ve büyük yazıyla resimlenir. Python kitabı ortak komutlarla oluşturur, `.kcad` yeniden açma ve serileri denetler; MCP aynı takımı komutlardan hazırlayıp PDF üretir. Web arayüzü ADR 0228 gereği ertelidir; dönüşte PDF toplama ve raster kaynak anahtarı aynı sayfa bağlamını taşımalıdır. Ortak TS ve WASM sürer.

Kabul kaydı: native plot 25 test, core kütüphane 380 ve entegrasyon 64 (+4 mevcut ignore), 233 Python testi; masaüstü altı sayfalı çıktı ve GPU ekran testi, MCP takım oluşturma/PDF testi geçti. Clippy (ortak/native/UI/masaüstü ve son panel değişikliği), bağımlılık kapısı, fixture/SDK kontrolü, optimize sheet WASM ve bütün mevcut smoke aileleri temiz. Üç tema/yazı-boyu varyantında küçük/büyük master, plan ve kesit ekranları `.run/shots/sheet17-*`; altı sayfalı PDF ve üçüncü sayfanın Poppler görüntüsü açılarak incelendi.
