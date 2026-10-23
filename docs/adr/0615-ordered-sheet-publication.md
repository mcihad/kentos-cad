# 0615 — Sıralı ve atomik pafta yayını

Tarih: 23 Ekim 2026. Kapsam: CAD-SHEET-25.

## Karar

`file.publish.sheets` v1 masaüstü, Python, CLI/headless ve MCP için aynı native
üreticidir. Seçim verilmezse bütün paftalar; açık liste verilirse tam o sıra
kullanılır. Boş liste, tekrar kimlik ve bulunmayan pafta reddedilir. Tek PDF veya
her pafta için ayrı PDF seçilir. Ayrı dosyalarda `{index}`, `{sheet}`, `{id}`
şablonu vardır; adlar taşınabilir yaprak adlarına çevrilir, büyük/küçük harf
çakışmasına kararlı sonek eklenir. Mevcut dolu klasörün üzerine yazılmaz.

Plan dosya yazmaz; gerçek PDF hattını çalıştırarak adları, sayfaları, boyutları,
hash'leri ve paftaya ait bulguları verir. Arayüz bu işi de arka planda çalıştırır.
Yayın için yeni/boş bir teslim klasörü gerekir. PDF'ler ve `yayin.json` görünmeyen
kardeş klasöre hazırlanır; bütün işlem başarılıysa klasör tek yeniden adlandırma
ile teslim edilir. Hata/iptalde geçici klasör temizlenir. Son kesinleştirme
sonrasındaki iptal tamamlanmış teslimi iptal edilmiş göstermez.

İlerleme, durdurma, kapatırken iptal ve geç gelen klasör seçimini ayırt eden
pencere kimliği vardır. İş belge okuma kopyasını kullanır; açık belge değiştiyse
sonuç başka belgenin günlüğüne yazılmaz. Planın revision/hash değeri yayına
iletilir; belge değişmişse yeniden denetim gerekir.

Tam preflight bulguları pafta sahipliğiyle korunur; aynı ana sayfa öğesinin
etkilediği her sayfa raporlanır. Yerleştirilmemiş model görünümü ve hata seviyeli
bulgu yayını engeller. Birleşik PDF üretilemezse bozuk sayfalar ayrı ayrı
incelenerek eksik kaynak raporu tamamlanır. Önizlemede yerleştirilmemiş çerçeve
PDF'teki gibi görünür. Dosya komutu işi artık zarfın Plan/Execute türünü korur;
Plan işinin yanlışlıkla dosya yazması ayrıca sınanır.

## Sınırlar

Bu dilim PDF setini ve kaynak belge kimliğini teslim eder. Harici kaynakların
byte snapshot'ı/provenance'ı CAD-SHEET-29, xref/font/plot kaynaklarının taşınması
CAD-SHEET-27 kapsamındadır. Diskte dışarıdan değiştirilmiş PDF'lerin tekrar
oynatmada ayrıca doğrulanması paket bütünlüğü kapsamında tamamlanacaktır.
Web uygulaması ADR 0228 gereği ertelidir.

## Kabul

Bağımsız Python üreticisinin 28 olgusu sıra, gerçek PDF sayfa içeriği, 100 mm
fiziksel çizgi, adlandırma, ret, eksik kaynak ve yarım teslim kalmamasını sınar.
Aynı dosya native dosya host'unda ve üretilmiş tipli Python SDK'da çalıştırılır.
Masaüstü plan/yayın, MCP, iptal ve açık/koyu/büyük yazı ekranları ayrıca sınanır.
28 dosya-host olgusu, planın dosya yazmaması, önceden iptal, masaüstü seçili
sıra/gerçek yayın ve MCP plan/sabitlenmiş yayın geçti. 242 Python testi ve 869
TS türü üretim testi geçti. Native plot'un 12 birim ve 38 entegrasyon kontrolü
çalıştırıldı: tam preflight ile yeni gelen `no_kerning` bilgi bulgusunun beklentisi
açıkça güncellendi ve ilgili fiziksel ölçü testi tekrar geçti. Yayın penceresi
üç tema/yazı boyunda gerçek Denetle tıklamasıyla sınandı; görüntüler açıldı,
kaydırma çubuğunun sıra düğmelerini örtmesi düzeltilip yeniden görüntülendi.
Katalog, SDK/manifest drift, clippy, bağımlılık yönleri ve ortak WASM smoke geçti.

Mevcut baskı önizlemesi, gerçek PDF Kaydet ve Yazdır yollarının iki masaüstü
gerileme testi de geçti; güncel düğme konumlarıyla önizleme görüntüleri açıldı.
