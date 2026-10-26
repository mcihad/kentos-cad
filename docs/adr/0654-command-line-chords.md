# 0654 — Komut satırı odaktayken kaydetme kısayolu

26 Ekim 2026. **CommandLine/EditCell dilimi kabul edildi.** ARCH-BASE-01'in
genel giriş matrisi açık kalır.

Gerçek Mac penceresinde Mahal numarala ilk numarasını düzenle → Tab → Cmd-S,
komut satırına `s` yazdı; sahne odağında aynı kısayol Kaydet'i çalıştırdı.
Bu bir numara hesabı veya dosya kodlayıcı kusuru değildir. Kusurun ilk hangi
commit'te başladığı araştırılmadan köken iddiası yapılmaz.

Yüklü `iced_widget 0.14.2/src/text_input.rs` incelendi: metin girişi A/C/V/X
kısayollarından sonra olayın yazdırılabilir `text` alanını değiştirici filtresi
olmadan ekler. Mac'in Cmd-S olayı `s` metni taşıyabildiğinden odaklı kutu
olayı yakalar; desktop'un yalnız yakalanmamış olayları alan yönlendiricisine
ulaşmaz. Test, gerçek bileşen ağacı/odak işlemi ve `text: Some("s")` olayıyla
bu davranışı eski kodda başarısız göstermelidir.

Karar: Komut satırı ve Özellik hücresi, ortak tuş yardımcısıyla komut değiştiricili karakter kısayollarını
iç metin girişine vermeden host'a bırakır. Platformun A/C/V/X düzenleme işleri
kutuda kalır. Düz/Shift/Türkçe yazı, Option karakteri ve Ctrl+Alt ile AltGr
sembolleri korunur. Alan odağı host'a bildirilir; kısayol tuşu yakalanmaz.
Katalog/izinli kısayol listesi, araç hesapları, Kaydet kodu ve belge şeması
aynıdır. Diğer giriş bileşenlerinin genel denetimi bu dar düzeltme değildir.

Kabul: eski/yeni gerçek bileşen olayı; kısayolun yakalanmaması ve harf
üretmemesi; A/C/X/V, Türkçe/Shift/AltGr korunumu; desktop'un mevcut Kaydet
rotası; son paketli Mac GUI'de Tab'dan sonra gerçek Cmd-S ile disk yazımı,
araç/önizleme korunumu, tek onay/undo. Sadece Kaydedildi iletisi yeterli değildir;
gerçek dosya bağımsız okunur. Kanıt
[kabul klasörüne](../baseline/2026-10-26-commandline-save/README.md) eklenir.

## Gerçek GUI takibinin genişlettiği kapsam

İlk koruma komut satırında çalıştı (04): bellekte U-099, diskte V-099 iken
Tab→Cmd-S U-099'u gerçekten yazdı; P-101 önizlemesi kaldı. Dar pencerede
Tab sonrası odak hâlâ Özellik hücresindeyken aynı Iced davranışı hücreye `s`
ekledi (05); Enter geçersiz P-102s'yi reddedip P-101'e döndü (06). Bu iki
ham görüntü tanısaldır; dosya adları/açıklamaları buna göre düzeltildi, baytları
korundu. İkinci düzeltme aynı kuralı EditCell'in iç kutusuna da uygular.
Bu durumda kısayol düzenleme tamponunu onaylamaz; Enter/odak kaybı kuralı
korunur. `red-cell` gerçek bileşen ağacında eski hücrenin olayı Captured
ettiğini başarısız testle gösterir. Tab odağının hangi hücreye geçeceği
bu değişiklikle yeniden tasarlanmaz.

Son kaynakta 38 ilgili test, clippy ve desktop derlemesi geçti. Son değişmez
pakette doğrudan hücre Cmd-S, Tab→Cmd-S, açık komut satırı odağı, tek araç
onayı/undo/Escape ve gerçek kaydedilmiş dosyalar doğrulandı. Kaydet,
onaylanmamış alan tamponunu onaylamaz; mevcut Enter/odak kaybı sürer.
15 ham görüntü ve dokuz disk kopyası bağımsız KCAD/UUID/geometri/etiket/
çizelge kontrolünden geçti. Gözlenen 1440×870, 1100×682, 1512×872 pt,
açık/18 ve koyu/13 px kapsamı tam boyut/font/tema matrisi değildir.
Başarısız ilk EditCell denemeleri ve test düzeneği hatası kabul kaydında
açıkça ayrıdır; ham kanıtlar korunmuştur.
