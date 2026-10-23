# ADR 0597: Görünüm penceresi sınırı ve çerçeve baskısı

Tarih: 22 Ekim 2026 · CAD-SHEET-12

Kırpma, modelin geometrisini veya kameranın ölçeğini değiştirmez. `MapItem.clip` içerik çerçevesinin milyonda birlik koordinatlarında `frame`, `rectangle {bounds}` veya `polygon {points}` taşır. Yeni yazım tür alanıyla nesnedir; eski açık `"frame"` yazımı okunmaya devam eder. Varsayılan tam çerçeve eski kitapların anlamını değiştirmez.

- Dikdörtgenin alanı pozitif; çokgen 3–256 köşeli, basit ve sıfırdan farklı alanlı olmalıdır. Köşeler çerçevenin %0–100 aralığındadır; kendine kesişme/değme, ardışık tekrar ve sıfır alan reddedilir. Geometrik retler tamsayı/i128 yönelimle verilir. Çokgen içbükey olabilir; iki sarım yönü de çalışır.
- Kırpma çerçeveye göre saklandığından öğe taşınınca, döndürülünce veya yeniden boyutlandırılınca sınır onu izler; model merkezi, dönüşü ve gerçek paydası korunur. Model görünümü kilitliyken sınır/baskı özelliği düzenlenebilir; bütün öğenin kilidi bunları da korur.
- `setViewport` işlemi isteğe bağlı `clip` ve `framePlot` alanlarını alır. Ham özellik yaması da aynı kitap validasyonundan geçer. `framePlot=false` görünümün sınır çizgisini çıktıdan çıkarır, model içeriğini korur. Tasarım sırasında sınır görünür; tüm öğeyi baskıdan çıkarmak mevcut `printable` alanıdır. Haritanın kartografik ızgara/süs açıklamaları ayrı öğe özellikleridir.
- İçbükey boşluklar kâğıt seçiminde tıklamayı almaz; sınırın kendisi toleransla seçilebilir. Öğenin tutamaç kutusu düzenleme için tam dikdörtgenidir. Model kimliğine seçim/snap/measure yönlendirmesi CAD-SHEET-13/16'dır.

## Görünüş ve çıktı

`MapPrim.clipShape` kaynaktaki sınırı taşır. PDF vektörleri, yazıları ve resimleri gerçek çokgen clip path'inden geçer; SVG harita resmi ve eksik kaynak yer tutucusu da aynı sınırı kullanır. Kâğıt çerçevesinin dönüşü sınır ve içerikle birlikte uygulanır. Kâğıt koordinat ızgarasının içerik/etiket bandı mevcut çerçevesinde kalır; bu özellik model görüntüsünün sınırıdır.

Masaüstünde tam dikdörtgenin mevcut hızlı çizim yolu sürer. Özel sınırın önizlemesi, native çıktı stil motorunun ortak RGBA üreticisinden gelir; model yazıları da gömülü fontlardan aynı resme çizilir. Ortak sınır maskesi son RGBA'ya uygulanır: satır kesişimleri, yatay piksel örtüşmesi ve dört düşey örnekle kenar kapsamı. Sahne ikinci kez farklı bir iş kuralıyla çizilmez. Görünüm/ölçek/sınır/katman ezmesi değişince sahne önbelleği yenilenir; aynı durumda tekrar üretim yoktur. Bu yol bir GPU stencil maskesi değildir; önizleme CPU'da üretilen dokuyu GPU arayüzünde gösterir. Native vektör PDF özel sınır nedeniyle raster'a dönmez. Ekran resmi 4096 px; mevcut çıktı 8192 px/32 MP sınırları korunur.

Masaüstü sınır penceresi, yüzdeyle dikdörtgen veya çokgen köşelerini ve gerçek en-boy oranlı önizlemeyi gösterir. Çerçeve/dikdörtgen/çokgen arasında dolaşırken ayrı taslaklar korunur. Geçersiz sınır uygulanamaz; iptal belgeyi değiştirmez, geçerli değişiklik tek ortak işlem/undo'dur. Değişmemiş geçerli bir sınırda Uygula da pencereyi kapatır. Kırpma/baskı alanları, kamera kilidinden ayrı görünür.

## CAD alışverişi

Dış sözleşmedeki `CadViewport.paperClipMm`, sınırı sağa X/yukarı Y kâğıt mm olarak; `framePlot` baskı durumunu taşır. DXF'te aynı paper-space sahibine ait kapalı LWPOLYLINE ve VIEWPORT grup 340 bağlantısı, bayrak 65536 kullanılır. Kırpma nesnesi yeniden içe aktarımda ikinci bir kâğıt şekline çoğaltılmaz. No-plot sınırı, görünür/donmamış ve plot kapalı ayrı bir katmanda yazılır; kullanıcının aynı adlı katmanıyla çakışmaz. Katman 0 donmuş veya no-plot ise basılacak viewport sınırı da ona yanlışlıkla bağlanmaz.

Kâğıt çerçevesi 30° gibi eğikse görünüm, dönen çerçevenin eksen paralel kutusu ve gerçek kırpma çokgeniyle yazılır; fiziksel ölçek korunur. Önceki köprünün çeyrek dönüş dışındaki çerçeveyi atlama sınırı böylece kalkar. Kaynağa geri gelince yazarın özgün çerçeve dönüşü yerine eşdeğer kamera dönüşü + çokgen saklanabilir; görünüş korunur.

Okuyucu düz kenarlı kapalı LW/POLYLINE sınırlarını destekler. Eksik/eğri/uygunsuz kırpma referansında görüşü genişleterek veri göstermemek için viewport atlanır ve kayıp raporlanır. Daire/elips/spline clip ve viewport katman freeze alışverişi ayrıca kapsamdır. OpenCADCodec bağımlılığı gömülmedi veya değiştirilmedi; mevcut sabitlenmiş sürüm bu sınır bağlantılarını DWG'de taşır.

Bu anlam [Autodesk VIEWPORT DXF](https://help.autodesk.com/cloudhelp/2015/ENU/AutoCAD-DXF/files/GUID-2602B0FB-02E4-4B9A-B03C-B1D904753D34.htm), [nonrectangular viewport](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-DidYouKnow/files/GUID-E79B6007-9897-4F69-AB5F-54C2D0BAF081.htm) ve [viewport sınırının plot katmanı](https://help.autodesk.com/cloudhelp/2022/ENU/AutoCAD-DidYouKnow/files/GUID-645754CB-7CF4-418F-8FA3-43E54AACD692.htm) başvurularıyla kontrol edildi.

## Otomasyon, bağımlılık ve kabul

Python tipli sınır sınıfları ve eski `PaperViewportClip.FRAME`/`"frame"` girdisini destekler. MCP aynı `setViewport` şemasını kullanır. `ScaleRatio` veri sözleşmesi alt `contracts` katmanına taşındı ve sheet'te aynı isimle yeniden sunuldu: 11. maddede eklenen ayrıntılı hazır oran çıktısı, schema kapalı native derlemeyi isteğe bağlı sheet bağımlılığına bağlamaz. Fiziksel payda sınırı tek ortak sözleşmededir.

- KentOS çağırmayan generator: 26 tam/dikdörtgen/üçgen/içbükey/ters yön/kilit/baskı ve ret senaryosu. Analitik bölge probu, alan/piksel kapsamı, aynı kamera, ters işlem ve dosyada yeniden açma sınanır.
- Python: aynı senaryolar, grup undo/redo ve legacy/tipli girdi; gerçek PDF'nin içbükey boşluğunda kırmızı dolgu bulunmaması bağımsız Poppler pikselleriyle ölçülür.
- Bağımsız ezdxf: gerçek 340 handle'ı, kapalı sınır noktaları, 65536 bayrağı ve plot katmanı; DXF ve gerçek DWG yeniden açıldığında tek viewport öğesi.
- Mevcut WASM yüzeyinde 26 sınır, 56 kamera ve 40 fiziksel projeksiyon; yeni binding/web uygulaması eklenmez. Web dönüşünde sınır düzenleyici, maske ve seçim aynı çekirdeğe bağlanmalıdır.
- Gerçek GPU ekranları: tam/üçgen/içbükey üç görünüm aynı modelden; açık/koyu 1440×900 ve 18 px/1100×650. Kaynak dolgu ve yazının içbükey boşluğa sızmadığı piksel bölgesiyle korunur. Sınır penceresi ve gerçek fareyle Uygula kontrolü de alınır; PDF ayrıca Poppler ile açılır.

Son kabul: 226 Python testi, 13 gerçek DXF/DWG sınır çifti, 26 WASM sınır durumu; ilgili Rust takımları, schema kapalı commands testi, clippy, bağımlılık kapısı ve format WASM derlemeleri geçti. CAD standartları preflight tarafından da tanınır; 1:50 için GIS listesinden gelen yanlış özel ölçek uyarısı kaldırıldı.
