# 0681 — Metraj CSV'si seçili tabloyu yazar

26 Ekim 2026. ARCH-DOC-02. **Bu CSV/geri bildirim dilimi kabul edildi; ana madde [~].**

ADR0680'in gerçek dışa aktarımında 400 satırlık CSV için 600 satır bildirimi
bulundu. İkinci gerçek denemede Duvarlar sekmesi açıkken yine 400 mahal
satırı yazıldı; dosyanın başlığı ve verisi bağımsız CSV okuyucuyla doğrulandı.

- CSV tek tablodur: Metraj'ın açık sekmesi ve mevcut kat süzgeci kullanılır.
  Boş tabloda başlık korunur; başka sekmenin verisiyle sessizce doldurulmaz.
- XLSX/ODS aynı kat süzgecindeki Mahaller/Duvarlar/Açıklıklar üç sayfasını,
  boş sayfa başlıklarını ve sayı hücrelerini korur. CSV/kitap farkı dışa
  aktarım düğmesinin yanında kullanıcıya açıklanır.
- Masaüstünün ortak dosya yazıcısı, CSV için yalnız gerçekten yazılan ilk
  sayfanın veri satırlarını sayar. XLSX/ODS bütün sayfaları sayar. Başlık veya
  bir hücre içindeki satır sonu kayıt sayısını artırmaz. Başarısız yazma ve
  kullanıcı iptalinde başarı bildirimi üretilmez.
- Ortak biçim yazıcı, veri/ölçü/birim/yuvarlama hesabı, Python/MCP sözleşmesi ve
  KCAD modeli değişmez. Yeni bir belge düzenleme komutu veya TS kaynağı yoktur.

Kabul: gerçek Duvar/Mahal/boş Açıklık CSV'si 200/400/0, üç sayfalı XLSX/ODS 600,
doğru bildirim/geçmiş, iptal, tekrar açma, bağımsız dosya okuma ve bütün 21.000
nesnenin model korunumu. Üç desktop testi başarılı (iki yeni, bir eski uyarı
regresyonu); son paket all-target clippy/derleme başarılı. 17 ham görüntüde açık 18,
koyu 18/13 örnekleri vardır. Mevcut uzun sayı→metin hassasiyet politikası korunur;
kitaplarda 1600 uzun değer notu da gerçek geçmişte doğrulandı. Genel çizelge/metraj ve bütün GUI matrisi bu dar dilimle kapanmaz.


[Ham gerçek dosyalar, görüntüler ve bağımsız kontrol](../baseline/2026-10-26-architecture-quantity-export/README.md).
