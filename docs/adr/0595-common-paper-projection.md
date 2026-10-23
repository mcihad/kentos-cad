# ADR 0595: Modelden kâğıda tek fiziksel hesap yolu

Tarih: 22 Ekim 2026 · CAD-SHEET-10

Model nesneleri gerçek boyunda, çekirdeğin kanonik metresinde kalır. Projenin mm/cm/m/in/ft tercihi giriş, gösterim ve alışveriş birimidir; pafta kamerasında ikinci kez çarpan olmaz. Dış CAD dosyasının kaynak birimi içe aktarımda bir kez normalize edilir. Kâğıt arayüzde mm, kalıcı geometride mevcut tamsayı µm ile yönetilir. Kâğıt CAD nesnelerinin fiziksel metre koordinatları ile modelin arazi koordinatları farklı sınırlardır; kâğıt nesnesine viewport paydası uygulanmaz.

## Ortak sınır

`kentos_sheet::projection`, arayüz/host bilmeyen tek fiziksel hesap modülüdür:

- `Scale`: payda `d = model/paper`, `paper_um = model_m × 1 000 000 / d`; ters yönde `model_m = paper_um × d / 1 000 000`. Örneğin 1:50'de 5 m, kâğıtta 100 mm; 2:1'de 5 cm yine 100 mm'dir.
- Fiziksel uzunluklardan payda, model–kâğıt uzunluğu, piksel çözünürlüğü ve kâğıt çizgi/yazı boyunun model karşılığı aynı sınırdan geçer. Komut girdisi sonlu ve 10⁻⁶–10⁸ aralığında olmalıdır; render'ın önceki alt sınır davranışı ayrı, açık bir yardımcıda kalır.
- `Transform`: önce merkez çıkarımı `f64`, sonra ölçek/dönüş; ekranın aşağı yönlü Y ekseni, ileri/ters dönüşüm, dört arazi köşesi ve kapsam kutusu ortak hesaplanır. Son ekran noktası gerektiğinde `f32` olur. GPU/CPU batch'lerine yerel başlangıca göre affine matris verir.
- Döndürülmüş kapsamın çerçeveye sığması aynı `fitting_denominator` hesabını kullanır. Tam oran ile standart/tamsayı ölçeğe sığdırma politikası ayrı kalır; hesabın ortasında oran kesilmez.

Ortak display/grid/kapsam, Iced pafta noktaları, vektör PDF, native CPU resim çıktısı, masaüstü resim önizlemesi, harita yazıları/çizgi boyları ve CAD viewport içe/dışa aktarımı bu modülü kullanır. Yeni ürün komutu, ikinci kamera modeli veya WASM binding'i eklenmez. Python/MCP ve masaüstü mevcut ortak komut/çıktı yolunu çağırır.

## Büyük koordinatta küçük ayrıntı

Bağımsız fiziksel PDF testi, merkezi `(487150, 4417820)` olan 5 cm çizginin 2:1 çıktısında 100 mm yerine yaklaşık 101,563 mm olduğunu gösterdi. Kamera `f64` olsa da stil motoruna sıfır belge başlangıcından çok uzakta verilen koordinatlar batch'in `f32` çözünürlüğünü kaybediyordu.

Pafta haritasının stil üretimi artık **o viewport'un merkezini yerel başlangıç** alır. Çıktının ve ekranın kaynak başlangıçları aynı kalır; modelin belge başlangıcı ve nesne koordinatları değişmez. Masaüstü stil önbelleği merkez X/Y bitlerini de anahtarına katar; aynı ölçekli iki uzak viewport birbirinin yerel batch'ini kullanamaz. Annotative kopyanın ölçeğe göre paylaşılması sürer; bu kopya yerel çizim başlangıcını sahiplenmez. Önbelleğin mevcut dört giriş sınırı korunur.

Bu değişiklik bütün renderer'ı FP64 yapmaz. Batch/tile koordinatları hâlâ FP32'dir; viewport'tan çok uzağa uzanan uzun nesneler, çok büyük görünen kapsam veya aşırı yakınlaştırma için sınırsız hassasiyet iddiası yoktur. Kaynak geometri ve nihai ölçü renderer mesh'inden üretilmez. Bu maddede ölçülen sınır, metre/altı mimari ayrıntının milyon mertebesindeki koordinatta doğru kâğıt ölçeğiyle gösterimidir.

## Bağımsız kabul

- `scripts/fixtures/viewport_projection_cases.py`: KentOS çağırmadan beş proje birimi × dört payda (50, 100, 37,5, 0,5) × iki dönüş (0°, 30°) için 40 durum. İleri/ters koordinat, fiziksel boy, dört köşe, piksel konumu ve 0,1 mm arazi ayrıntısı; büyük ve küçük merkezler. Native core ve mevcut WASM PDF arayüzü aynı bağımsız beklentiyi okur.
- Python: 20 **gerçek PDF** beş proje birimi/dört paydada Poppler ile bağımsız okunur. Fiziksel uzunluk ve sayfa konumu 0,005 mm içinde karşılaştırılır; model nesnesi, kayıt/yeniden açma ve pafta undo'sundan sonra aynıdır. Ölçülen çizgi yalnız görüntüde tahmin edilmez.
- Poppler'ın SVG yazdırma yolu kesirli PDF point boyutunu varsayılan olarak küçültebilir. Ölçüm `-origpagesizes -noshrink -nocenter` ile gerçek sayfa boyunda yapılır; tolerans büyütülerek baskı ölçeği hatası örtülmez.
- Bağımsız ezdxf ile hazırlanmış m/mm/cm/dm/in/ft dosyaları farklı proje gösterim birimine alınır. Modelin 5 m kalması, görünümün 1:37,5 olması ve kapsamın 7,5×3,75 m olması sınanır. `dm` dış dosya birimidir; ürünün proje birimi listesine eklenmiş değildir.
- Poppler piksel çıktısında 400 mm uzunluğa düşen çizginin 200 mm içerik penceresinde gerçekten kırpılması ölçülür.
- Eski kabul PDF okuyucusunun ikili stream sonundaki CR/LF baytını `rstrip` ile silmesi düzeltildi: stream `/Length` kadar okunur. Gerçek çıktı değişince zlib checksum'unun son baytı LF olmuştu. CR ve LF ile biten iki bağımsız sıkıştırılmış payload regresyonu bu hatayı korur; ürün PDF'si okuyucunun hatasına uydurulmaz.
- `sheet10_precision_screens`: 5×3 cm iki model detayı, biri sıfırda diğeri büyük arazi koordinatında; iki ayrı 2:1 viewport. Açık/koyu 1440×900 ve 18 px yazılı 1100×650 gerçek GPU çıktısında kâğıt bölgesindeki kırmızı/mavi ayrıntının genişlik ve yüksekliği en çok 1 piksel farkla eşittir. Aynı kitaptan PDF alınır; export'un modelin tam snapshot'ını değiştirmediği de denetlenir.

Web uygulaması ADR 0228 gereği ertelidir. Ortak çekirdek optimize WASM olarak derlenir; mevcut arayüzlerle 32 viewport ve 40 fiziksel projeksiyon kabulü çalışır. Web dönüşünde kendi render/önbellek yolu bu ortak fiziksel sınırı ve viewport yerel başlangıcını kullanmalıdır.

Kullanıcının ertelediği DWG blok yerleştirme birimi/varsayılanı bu değişiklikte değiştirilmedi. Standart ölçek seçicisinin ileri kapsamı CAD-SHEET-11, serbest klip ve frame plot/no-plot CAD-SHEET-12 olarak kalır.

Son kabul: 222 Python testi; 714 katalog/sözleşme, 131 native uygulama entegrasyonu; 379 sheet birim + 56 entegrasyon; 44 UI birim + 33 tasarımcı + 7 kitaplık; plot/headless/MCP ve masaüstü pafta takımları geçti. Değişmeyen SVG/PDF golden dosyaları, clippy, Python SDK eşleşmesi, optimize WASM ve 47 crate / 61 hedef bağımlılık denetimi temiz. Gerçek GPU kâğıt alanında iki detay 13 px yazıda **268×161 px**, 18 px yazıda **138×83 px**; açık/koyu tema boyları aynı.
