# ADR 0862 — Python metin alanlarında komut kısayolları

Tarih: 2026-11-03. CAD-ADV-06 gerçek GUI kabulünde görülen arayüz düzeltmesi.

## Doğrulanan sorun

Gerçek masaüstü konsoluna çalıştırılmadan `unrun_marker = 123` yazılıp
⌘O basılınca dosya seçici açılmadı; kaynak `unrun_marker = 123o` oldu.
Konsolun kendi metin Undo'su ise beklendiği gibi yereldi; çizime geri alma
uygulamak için çizim odağı gerekti. Bu iki davranış aynı hata değildir.

[Apple'ın kısayol tanımı](https://support.apple.com/en-us/102650)
⌘O'yu Aç, ⌘Z/⇧⌘Z'yi geri al/yinele ve ⌘S'yi kaydet olarak tanımlar.
Kurulu Iced 0.14.2 `text_editor::Binding::from_key_press`, tanınmayan
kısayolda olayın yazdırılabilir `text` alanından `Insert` üretebiliyor.
Konsol ve betik alanının özel bağlaması bu varsayılana dönüyordu;
betik bağlaması ayrıca Mac'in Command tuşunu Ctrl eşdeğeri saymıyordu.

## Karar

- KentOS UI Python bileşeninin `default_key_binding` yardımcısı standart
  metin bağlarını korur, ancak tanınmayan Command/Super veya Ctrl
  kısayolundan `Insert` üretilirse onu uygulamaya geri bırakır. Üstteki
  mevcut kısayol yönlendiricisi Aç/Kaydet gibi kendi eylemini yürütür.
- Ctrl+Alt ile AltGr ve Mac Option metni normal yazıdır; `@`, Türkçe
  karakterler, kopyala/yapıştır, tümünü seç ve imleç hareketleri korunur.
  Her değiştiricili tuşu yutmak veya tümünü çizime göndermek yoktur.
- Konsolun Enter, geçmiş, tamamlama ve metin Undo/Redo bağları yerel kalır.
  Konsol odaktaki Undo, çizimin geometrisini değiştirmek için kullanılmaz.
- Betik alanının mevcut kaydet, farklı kaydet, geri al/yinele ve seçimi
  çalıştır bağları platform Command tuşunu da kabul eder. Ctrl davranışı
  korunur. Böylece Mac'te ⌘S betiği kaydeder, kaynak koda `s` eklemez.
- Yeni bir belge, dosya biçimi veya sahne işlemi yoktur. Mevcut Python
  komutlarını çalıştırma/planlama ve MCP sözleşmeleri değişmez. Bu,
  kaynak metni ile klavye kısayolunun ayrılmasıdır; Python/MCP'ye yeni
  bir geometri komutu eklenmiş sayılmaz.
- Kendi tuş haritasını kuran UI tüketicisi varsayılan dönüş için dışa açık
  `kentos_ui::widget::python::default_key_binding` kullanabilir. Bilinçli
  özel bağlamalara müdahale edilmez; yardımcı yalnız standart dönüş içindir.

## Kabul — tamamlandı

Komut harfinin yazılmaması, normal/Option/AltGr yazısı, pano, seçim,
imleç ve odaksız alan test edilir. Betik Command/Ctrl kaydet, farklı kaydet,
Undo/Redo ile normal yazma ayrı doğrulanır. Gerçek GUI'de dolu konsolda ⌘O
ile Aç/İptal, metnin korunması ve metin Undo'sunun çizime dokunmaması;
betikte ⌘S ile `.py` kaydı ve kodun değişmeden korunması görülmeden
bu dilim tamamlanmış sayılmaz. Ekranlar ve deneme dosyaları depo dışındadır.

2026-11-03: 326 UI (`snapshot` özelliğiyle), 1614 masaüstü testi,
her iki paketin tüm hedeflerinde Clippy ve masaüstü derlemesi geçti.
Gerçek QA penceresinde çalıştırılmamış `unrun_marker = 123` odağında
⌘O Aç iletişimini getirdi; İptal sonrası aynı kaynak çalıştırıldı ve
`assert unrun_marker == 123` geçti. Yerel konsol Undo'su çizime dokunmadı.
Mevcut isimsiz betik taslağı önce ayrı `.py` olarak korundu; Türkçe ve `@`
içeren ekleme ⌘S ile harf eklenmeden kaydedildi. ⇧⌘S, ⌘Z ve ⇧⌘Z ile
ayrı dosyalar alındı: Undo son eklenen satır sonunu kaldırdı, Redo önceki
kaynak baytlarını tam geri getirdi. Son CAD kaydı başlangıçla bayt düzeyinde
aynı. Eski hata ekranı 258; kabul ekranları 261–264 ve JSON/dosyalar
`/tmp/kentos-cad-adv04-2026-11-01/` altında, depo dışındadır.
