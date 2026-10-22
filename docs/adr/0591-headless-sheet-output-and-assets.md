# ADR 0591: Başsız pafta çıktısı ve gömülü resimler

- Tarih: 22 Ekim 2026
- Kapsam: CAD-SHEET-06; ADR 0252'nin çıktı eksiklerini kapatır.

## Ortak hesap ve çıktı

1. `cad.sheets.edit` tasarımcının bütün `kentos.sheet/1` işlemlerini uygular. İşlemlerin JSON Schema'sı `Op` tipinden üretilir; AI kataloğundaki `ops` artık içeriği bilinmeyen bir nesne dizisi değildir. Sayfa, öğeler, bağlar, değişkenler ve şablon seçenekleri bu şemanın parçasıdır.
2. `pictures` gömülü PNG/JPEG/SVG baytlarını base64 ile alır. Her yükleme kitapta kayıtlı SHA-256 ve uzunlukla eşleşmelidir; tekrar eden yükleme veya bozuk bayt bütün işlemi reddeder. `applyTemplate` resimleri otomatik gömer. Metadata kaldırılırken kalan kullanılmayan baytlar da atılır.
3. Baytlar değişince cevap `inversePictures` verir. Tam geri dönüş için `inverse` işlemleriyle birlikte bu liste `pictures` olarak ve `replacePictures=true` verilmelidir. Boş liste de anlamlıdır. Metadata tek başına bayt yaratmaz; sonradan yükleme mümkündür. Plan ve doğrulama belgeyi değiştirmez.
4. Harita yazıları, ölçüler, çok satırlı metinler, etiketler, maskeler, tablolar ve yerleştirilmiş etiket bileşenleri ortak Spatial/paragraph yerleşiminden alınır. Masaüstü vektör PDF'si ile başsız PDF aynı `kentos_plot::labels` toplayıcısını kullanır. Fontlar uygulamayla gelen fontlardır; işletim sistemi fontlarına bağlı çıktı üretilmez.
5. Vektörle yazılamayan desen, resim ve benzeri stiller masaüstü GPU gölgelendiricilerinin mevcut CPU eşleniğiyle çizilir. CPU resim/stil kodu GPU paketinden native stil çekirdeğine taşınmıştır; ikinci bir stil motoru yazılmaz. Yazı konturları aynı gömülü fontlardan şekillendirilir.
6. Zengin harita resmi sayfanın seçili dpi değeriyle üretilir; en uzun kenar 8192 piksel, toplam yaklaşık 32 MP sınırında oran korunarak küçültülür. Cevabın `notes` alanı resme çevrilen stilleri ve sınırı açıkça söyler. Haritadaki okunamayan yerel resim kaynağı boş kutu olarak teslim edilmez; PDF bütünüyle reddedilir. Bu raster haritaların yazıları arama metni değildir; desteklenen vektör haritaların yazıları seçilebilir ve aranabilir kalır.
7. SVG pafta resimleri aynı native SVG rasterizer ve gömülü fontlarla PDF'e girer. `file.export.pdf.drawing` bağlı yerel resimlerin çizime göre çözülmesini sağlar; açık dosyanın yolu headless oturum tarafından otomatik sağlanır. PDF üretimi kaydetme işlemi değildir.

## Doğrulama

- `scripts/fixtures/sheets_edit_command_cases.py`: metadata, PNG/SVG baytları, geç yükleme, tam set değiştirme, ters baytlar, plan/doğrulama ve bozuk yükleme için 20 bağımsız ek durum; aynı dosya native, Python ve MCP komut yollarında kullanılır.
- `scripts/fixtures/sheet_output_cases.py`: font, fiziksel ölçek, dönüş, model birimi, maske ve katman görünürlüğü için 24 bağımsız durum. Gerçek PDF'in yazısı Poppler ile okunur.
- `crates/native/plot/tests/all/output.rs`: desenli harita PDF'e yazılır, Poppler ile PNG'ye çevrilir; desen ve yazı pikselleri ayrıca denetlenir. `.run/shots/sheet06-rich-12a20d4e79.png` açılarak incelendi.
- Taşınan CPU raster/atlas/SVG testleri korunur. Eksik kaynak ve resimli şablon kalıcılığı ayrıca sınanır.
- Web arayüzü ADR 0228 gereği ertelidir; ortak katalog/TypeScript üretimi ve WASM derlenebilirliği korunur.

- Masaüstü `sheets::tests`: 10 normal çıktı testi geçti. `sheet_comparison_screens` gerçek örnek `fixtures/document/v1/sample.json` verilerek çalıştırıldı; 1440×900 ve 1100×650 açık/koyu pafta, galeri ve logo ekranları üretildi. Açık logo ve koyu tam sayfa görüntüleri açılıp incelendi; örneğin eksik parsel veri bağları ön denetimde kalır.
- Python'un eski ham JSON işlem girdileri korunur; geçersiz girdiler Python decoder'da sessizce temizlenmeden çekirdeğe ulaşır. Şemadan üretilen tiplerde nesne/liste varsayılanları her örnek için ayrı oluşturulur.
- Raster veri katmanı, nokta bulutu ve ağ harita servisi çıktı sağlayıcıları ADR 0204/0207/0208'deki ayrı teslimlerdir; bu çalışma onların GPU geçişini CPU'da yapılmış saymaz.
