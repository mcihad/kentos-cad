# 0686 — Yeni araçta ayarların başlangıcını göster

26 Ekim 2026. **Dar kapsam kabul edildi.** ARCH-UX-01 gerçek GUI takip kusuru.

Etiket paneli veya seçili nesne özellikleri aşağı kaydırıldıktan sonra yeni
mimari araç başladığında aynı kaydırma konumu kalır; ilk yöntem/kat alanları
görünmez. Kullanıcı yeni aracın ayarlarını görmek için elle üste dönmek zorunda.

Öznitelikler panelinin mevcut tek kaydırması masaüstündeki kendi gövdesine
alınır ve özel widget kimliğiyle adreslenir. Başarıyla başlayan, görünür
ayarları olan yeni araç paneli açıp yalnız bu kaydırmayı başlangıca getirir.
Ayar değişikliği, önizleme veya sıradan görünüm yenilenmesi kaydırmayı
sıfırlamaz. Diğer panellerin kaydırması ve genel docking bileşeni değişmez.
Şeffaf gezinme/ölçü aracı başlatma rotası eski erken dönüşünü korur.

Belge, ölçek, geometri, renderer, kayıt şeması veya ürün komutu değişikliği
yoktur. Mevcut araç/komut/Python/MCP yolları korunur. Yeni TS/TSX üretilmez.
Gerçek eski sürümde kusur, yeni sürümde seçili nesneden Etiket'e ve
Mahal→Kapı→Etiket geçişleri doğrulandı. Koyu13 geniş ve açık18 dar pencerede
ilk ayarlar üste gelir; aynı araçta kaydırılmış değer/nesne düzenlemesi ve
önizleme paneli sıfırlamaz. Kapatılan panel yeni araçla tekrar açılır.
19 ham görüntü (iki tanısal başarısız giriş ayrıca ayrılır), beş bağımsız
tam KCAD karşılaştırması; 62 ilgili test, clippy/build geçti. Altı mevcut
görüntü/performans testi atlandı; tam mimari matris veya bütün panel yaşam
döngülerinin kabulü sayılmaz. [Kayıt](../baseline/2026-10-26-architecture-settings-scroll/README.md).
