# 0620 — Yazıcıdan bağımsız ortak pafta PDF üretimi

Tarih: 23 Ekim 2026. Kapsam: CAD-SHEET-30.

PDF üretimi native plot katmanındadır. Masaüstü kaydetme ve Yazdır, pencere
istemeyen headless oturum, Python'un `Document.export_pdf` ve tipli
`kentos.cad.file.export.pdf` yolu, MCP'nin katalog komutu aynı PDF hattını
kullanır. Toplu yayın ve atlas da aynı renderer, fiziksel dönüşüm, paketli fontlar
ve kaynak Capture'ından yararlanır. Python açıklamasındaki eski “metinler henüz
yazılmaz” bilgisi kaldırılır; aranabilir Unicode metin desteklenir, gerçek
rasterleştirme tercihleri ve sınırlar notes/bulgularda bildirilir.

Dosya komutunun planı baytları hesaplar, dosya yazmaz. Execute dosyayı ortak
atomik yazıcıyla yazar; çıktı dosyası SHA-256/uzunlukla raporlanır. Aynı requestId
ile tekrar çağırmak yeniden yazma değildir. Belge geometrisi veya pafta kitabı
bu işlemle değişmez; çizimin geri alınması dışarıdaki PDF'yi kaldırmaz.

İşletim sisteminin fiziksel yazıcı/kağıt seçimi host işlemidir. Masaüstü aynı
PDF'yi sistem görüntüleyicisinde açar; renderer bir yazıcı sürücüsüne, pencereye,
sistem fontuna veya tarayıcının print API'sine bağlı değildir. Test Yazdır yolunu
sistem görüntüleyicisini gerçekten açmadan yakalayıp Kaydet ile aynı baytları
verdiğini doğrular. Fiziksel yazıcıya otomatik iş gönderilmez.

Web UI ve tarayıcı print diyalogu ADR 0228 nedeniyle bu teslimin etkin host'u
değildir. Ortak TS sözleşmeleri ve mevcut WASM derleme/smoke hattı sürdürülür.
Windows/Linux yazıcı sürücüleri ve gerçek cihazda çıktı bu macOS oturumunda
sınanmış sayılmaz. GIS raster/nokta bulutu/servis provider sınırları ADR 0618'deki
açık ret davranışını korur; boş pafta başarılı çıktı gibi sunulmaz.

Kabul kanıtları:

- Bağımsız `calibration.json` içindeki 20 fiziksel senaryo MCP komutundan
  pencere/yazıcı olmadan çalıştırıldı. Her plan dosya yazmadı; execute SHA'sı
  planla eşitti. PDF talimatları KentOS kullanmayan okuyucuyla çözülerek hem
  modelden hem kâğıttan gelen farklı renkli 100 mm çizgiler ölçüldü. mm/cm/m
  tercihi, 1:1–1:200 ve büyük arazi koordinatlarında 90 derece kamera kapsandı.
  RequestId tekrarının dosyayı yeniden yazmadığı doğrulandı.
- Aynı 20 örnek Python'un mevcut `Document.export_pdf` ve tipli
  `file.export.pdf` yolundan aynı çizimde birebir aynı dosya baytlarını verdi.
  Plan dosya yazmadı; kaynak nesneler değişmedi; export geri alma adımı olmadı.
  Tam Python takımının 246 testi geçti.
- Masaüstünde gerçek Kaydet ve Yazdır yolları aynı PDF baytlarını verdi ve
  özgün çizim/pafta kitabı aynı kaldı. 42 native plot entegrasyonu ve kaynak
  kimliği için MCP senaryosu geçti. Yeniden açılan çizimin bütün PDF akışları
  ve gerçek önizleme pikselleri aynı; metadata gerçek oturum revizyonunu söyler.
- Üç tema/yazı boyutunda son kaynak raporu görüntüleri açılıp incelendi.
  Native clippy, katalog, 875 TS türü, SDK/manifest ve bağımlılık yönü geçti.

Optimize ortak WASM derlemesi ve mevcut smoke koşucusu da geçti. Native
otomasyon host kabulü tamamlandı; fiziksel cihaz ve ertelenen web sınırları
yukarıda açıkça ayrılmıştır.
