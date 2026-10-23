# 0618 — Paftanın gerçek baskı hattında ön denetimi

Tarih: 23 Ekim 2026. Kapsam: CAD-SHEET-28.

`file.preflight.sheets` v1 dosya yolu istemeyen ve dosya yazmayan bir native
sorgudur. Seçili veya bütün paftaları ortak PDF hattı üzerinden denetler; kaynak
pinini, pafta kimliklerini, kodlu bulguları, çözüm metinlerini ve `ready` sonucunu
verir. Hata bulgusu olan rapor hazır sayılmaz. Plan ve execute aynı salt okuma
raporudur. Masaüstü aynı komutu iptal edilebilir arka plan işi olarak kullanır;
Python ve MCP katalogdan aynı tipli sorguya ulaşır.

Pafta tasarımcısının Ön denetim paneli yerleşim bulgularını göstermeye devam
eder; **Baskı öncesi tam denetim** yerel kaynak ve gerçek çıktı denetimini açar.
Yerleşim denetiminin temiz olması yerel dosyaların varlığına kanıt olarak sunulmaz.
Tam denetim penceresinde yayınlama ve çıktı klasörü seçme eylemi bulunmaz.

Ortak çekirdeğin font/glyph, eksik varlık, kâğıttan taşma, kenar boşluğu, sığmayan
yazı/tablo, kopuk pafta bağlantısı ve yer tutucu denetimleri korunur. Native katman:

- Sahnenin gerçek master/pafta öğeleri ve viewport katman durumları üzerinden
  görünür/basılabilir içeriği okur. Gizli, baskı dışı ve viewport genel kapsamının tamamen dışında olan
  içerik bilerek atlandığında yanlış desteklenmeme hatası verilmez.
- Kilitsiz viewport ölçeğini uyarır; kaynak modelin veya import biriminin değeri
  değiştirilmez. Tasarımcıda kilitleme seçeneği kullanılabilir.
- Kaynak ilişkisi kopuk ölçü/yazı/kılavuz ve kullanılan özgün font yerine paketli
  yüzle çizilen metin açıkça raporlanır. Önbellekteki xref içeriği kaynağı yoksa
  kendiliğinden yenilenmez; saklanan son içeriğin çizildiği belirtilir.
- CAD resimlerinde iki eksendeki kaynak piksel boyu, model metreleri, viewport
  ölçeği ve nihai plot büyütmesi birlikte kullanılır. Kâğıttaki resimlerde
  contain/cover/original yerleşiminin gerçek display boyutu okunur. 150 PPI altı
  uyarıdır. Gerçek raster fallback PNG'sinin piksel sayısı ayrıca ölçülür;
  8192 px/32 MP sınırının ve nihai baskı büyütmesinin etkisi görünür olur.
- Native PDF/CPU hattının henüz çizmediği görünür GIS raster, nokta bulutu ve harita
  servisi içerikleri açık hata olur. Görünüşü eksik olan pafta başarılı PDF gibi
  üretilmez. Bu karar provider renderer desteğinin tamamlandığı anlamına gelmez.
- SVG dış dosya/adres/stil referansları ve bozuk gömülü resimler reddedilir;
  yerel dosyayı sessizce okuyup başka cihazda kaybetme veya boş görüntüyü başarılı
  verme yolu kapatılır. İç `#id`, gradient ve gerçek gömülü resimler korunur.
  SVG 2 `href` baskı okuyucusunun kullandığı xlink biçimine XML konumlarıyla
  dönüştürülür; rasterizer dış dosya çözücüsü kapalıdır. Sayfa SVG okuyucusundan
  daha dar olan CPU sembol okuyucusunun atlayacağı gradient/style/image gibi
  içerikler de açık ret olur; tam kaynak paketinde kullanılmayan gömülü SVG
  kaynaklarının dış bağımlılıkları dahil denetlenir.

Canlı tablo/çizelge/lejant verisi denetimden önce ortak native veri sağlayıcısından
alınır. PDF'in aynı kontrolleri kullanan hazırlığı, desteklenmeyen görünür içerikte
reddedilir. Masaüstü kaydetme/yazdırma, Python ve MCP için ikinci bir çizim motoru
yazılmaz. Web UI ADR 0228 gereği ertelidir; ortak sözleşmeler üretilir.

Çalıştırılan kontroller:

- Native komut koşucusunda 51 bağımsız olgu plan/execute eşitliği, kaynak
  değişmezliği ve çıktı yazılmamasıyla geçti; fiziksel PPI Python'da bağımsız
  hesaplanan model/kâğıt boylarıyla karşılaştırıldı.
- 39 native PDF/preview/atlas/kalibrasyon/stil regresyonu geçti. Eski iki kilitsiz
  viewportlu senaryo artık iki uyarıyı tam kod/öğe/pafta sahipliğiyle bekler;
  fiziksel PDF karşılaştırmaları aynı kaldı.
- Üç plot birimi (20 SVG girdi olgusu + gerçek kırmızı piksel, CPU sembol retleri,
  üç provider için doğrudan CPU/atlas guard) ve üç kaynak paketi birimi
  (değişen kaynak, iptal, kullanılmayan SVG'nin iki kaynak türü) geçti.
- Masaüstü tam sorgu ve yayınlamama testi, MCP sorgusu ve gerçek Denetle fare
  tıklaması üç açık/koyu/büyük yazı ekranında geçti. İlk ekran incelemesinden
  sonra hatalar öne alındı, dosya yolu eklendi ve tehlike rengiyle ayrıldı; son
  üç görüntü de açılıp tekrar incelendi.
- Katalog üç kontrolü, 875 TS tür üretimi, SDK/manifest/47 crate-61 hedef
  bağımlılık yönü, native/masaüstü/MCP clippy ve ortak WASM derleme/smoke geçti.

Python genel koşucusunun dosya komutu yönlendirmesi yeni sorguya bağlandı;
yayın koşucusuna yanlış gönderilen yeni fixture düzeltildi. Tam Python takımının
245 testi geçti; 51 yeni olgu tipli ortak komut koşucusunda bir kez çalışır.
Kaynaklarıyla teslimin 35 olgulu regresyonu da tekrar geçti. Kabul tamamlandı.
