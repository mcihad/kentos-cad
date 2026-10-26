# 0656 — Görünür yeni etiket ayarları

26 Ekim 2026. **Dar kapsam kabul edildi.** ARCH-SPACE-02 / ARCH-UX-01 takip dilimi.

Etiket aracı Şablon/Biçim/Kol seçeneklerini komut satırında gösteriyor,
fakat etkin araç ayarları paneli boş. Etiketin yeni nesne ayarları mevcut
native ayar tanımlarıyla ayrı başlıkta sunulur: şablon, çerçeve, kol ve
kağıt üzerindeki yazı yüksekliği (mm). Seçili nesnenin özellikleri korunur;
her tuş model değişikliği değildir. Mevcut alan Enter/blur davranışı sürer.

Şablon aynı ortak `tag_template` doğrulamasını kullanır. Boş giriş türün
varsayılanına döner; `\\n` satır ayırıcıdır. Geçersiz şablon/kağıt boyu eski
ayar ve hedefi korur. Görünür şablon onayı bekleyen konsol şablon sorusunu
bitirir. Çerçeve/Kol menüleri ile komut satırı aynı araç durumunu kullanır;
eski Biçim döngüsü ve hedef→yer tıklama/Enter/Escape adımları korunur.

Kağıt yazı boyu, proje birimi veya 1:n ölçeği değildir; `NewTag.size` alanının
mevcut mm anlamıdır. Boş boy 2.5 mm varsayılanı, açık boy 0 < mm ≤ 50;
ondalık virgül ve isteğe bağlı mm eki kabul edilir. Son etiket/önizleme
mevcut `cad.tags.create` plan/execute yolundan gelir. Yeni şema, ürün komutu,
Python/MCP hesabı, TS/TSX, model birimi veya renderer değişikliği yoktur.

Gerçek GUI ilk pakette iki takip kusuru yakaladı: çok satırlı şablon alttaki
sabit yükseklikteki düğmede kesiliyordu; yeni odakta ilk yapıştırma bazen
`v` yazıyordu. Şablonun tamamı panelde kalır; komut düğmesi artık tek satır
Özel/Türün özetidir. İkinci kusur eski widget testinde de yeniden üretildi:
Iced iç metin kutusu ModifiersChanged önbelleğini kullanıyor; yeni odak bu
bildirimi kaçırmışsa KeyPressed içindeki gerçek Cmd/Ctrl bayraklarını okumuyor.
CommandLine/EditCell yalnız iç metin kutusuna her basışın kendi bayraklarını
önce eşitler; yeni genel kısayol/host onayı eklenmez. Eski A/C/X/V, AltGr,
Cmd-S tampon koruması, düz metin ve odak kaybı yolu regresyonla korunur.

Son değişmez pakette ilk hızlı yapıştırma, görünür alan/menü, yanlış değer,
hedef/yer, tek Enter/undo/redo, kaynak adının etiket/çizelgeye takibi ve yeni
süreçte açma gerçek native pencerede doğrulandı. 26 ham görüntü (tanısal
eski paket kayıtları ayrıca ayrılır), 11 bağımsız tam KCAD karşılaştırması;
697 interaction, 40 ilgili UI/host, üç mevcut host ayar ve üç mevcut Python
testi, bağımsız 165 durum, clippy/build geçti. Atlanan testler ve tekrarlar
[kabul kaydında](../baseline/2026-10-26-architecture-tag-settings/README.md) açıktır.

Dar 1100×683 pt açık18/koyu13 ve geniş 1440×863 pt görünümleri denendi.
Uzun şablon alan içinde kesilebilir; düzenleme tamponu ve değer korunur.
Önceki nesne özelliklerinde aşağı kaydırılmış panel yeni araçta aynı yerde
kalabiliyor; alanlara elle yukarı kaydırarak erişildi. Bu otomatik başlangıç
konumu takip işi açıktır. Toplu etiket Koru/Yenile, zengin kot şablonları ve
bütün mimari araçların tam matris kabulü tamamlanmış sayılmaz.
