# 0670 — Mahal sınırında açık kat kimliği

Tarih: 25 Ekim 2026. ARCH-SPACE-01'in kat ayrımı önkoşulu;
toplu bulma ve genel mimari yol haritası açık kalır.

## Bulgular ve ürün kuralı

`Document::space_found`, bir mahalın `arch.level` alanı olsa da sınır aramasına
bütün katların duvarlarını veriyordu. Duvar birleşimlerinin farklı katlarda
ayrılması bu sorunu çözmüyordu: yüz araması yine bütün katların çizgilerinden
en küçük kapalı alanı seçebiliyordu. Aynı XY konumundaki katlar birbirinin
mahal sınırı olabiliyordu. Katı değiştiren mahal komutunun planı da sınırı
yeniden hesaplamıyordu.

- Açık `arch.level` varsa hem yakın aramada hem tam aramada yalnız aynı kat
  **kimliğinin** duvarları kullanılır. Eşit kotlar kimlik eşitliği değildir.
- Katı verilmemiş eski mahaller ve çağrılar bütün duvarlardan arama davranışını
  korur. Dosya açılışında kat tahmini, sessiz göç veya geometri değişimi yoktur.
  Mahal aracında bu seçim artık açıkça **Tüm katlar** adını taşır.
- Mevcut Mahal aracının görünür **Kat (L)** seçimi önizleme ve yazmada aynı
  `NewSpace.arch` alanını verir. Seçenek listesi Çizgiden duvarın mevcut kat
  seçimiyle ortak küçük bileşendir; duvar aracının davranışı değişmez.
- Kaldırılmış seçim kaybolmaz; “Bulunamadı” görünür ve komut
  `level_not_found` ile atomik reddeder. Bilinmeyen kat, kapalı olmayan oda
  şeklinde yanlış raporlanmadan önce doğrulanır.
- Mahalın yönteminin **veya katının** değişmesi plan sırasında sınırını
  yeniden bulur. Doğrudan belge düzenlemesinde de kat değişimi follower'a
  dahildir. Önizleme yazmaz; execute ve tek undo/redo aynı sınırı taşır.
- Kat artık tohumu kapatmıyorsa mevcut modelin `lostEnclosure` durumu
  kullanılır; son sınır korunur. Başka katın duvarı kaybı örtmez. Duvarın
  kalınlaşması/silinmesi de aynı kat kuralıyla izlenir.
- Mevcut `cad.spaces.create/edit/quantities`, Python ve MCP yolları kullanılır.
  Yeni komut, sözleşme türü, TS türü veya KCAD şeması yoktur. Ortak katalog ve
  üretilmiş Python yardım metni kuralı açıklar; web UI'ı değiştirilmez.

## Kaynaklarla karşılaştırma

Autodesk'in [ilişkili mahal güncellemesi](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-Architecture/files/GUID-F51C80FE-3B37-43E8-8A20-2850D07AFDD9.htm)
sınır nesneleri değişince mahalın güncellenmesini; [mahal sınırları](https://help.autodesk.com/cloudhelp/2023/ENU/AutoCAD-Architecture/files/GUID-833A18AF-8F5C-4520-A50C-055B45C9F6D9.htm)
ise temel/net/kullanılabilir/brüt sınırların ayrı amaçlarını açıklar.
KentOS için çıkarım: sınırın hangi kaynaklardan bulunduğu ve hangi yöntemle
ölçüldüğü ayrı, görünür kurallardır. Bu dilim yalnız kat kaynağını düzeltir.
Mevcut Net/Eksen/Brüt hesabını değiştirmez; Autodesk'in standart tabanlı brüt
alan paylaşımının veya bir mevzuatın bütünü uygulanmış sayılmaz.

## Kabul ve kalanlar

[Kabul kaydı](../baseline/2026-10-25-architecture-space-levels/README.md)
bağımsız ölçülü üst üste odaları, eski referansların korunmasını, native,
Python/MCP ve gerçek pencere kontrollerini tutar.

Bu, toplu yüz üretimi değildir. Toplu bulma, mevcut mahalı yinelememe,
seçim/bölge kapsamı, kapanmayan sınırı vurgulama, kolon çıkarımı ve kalıcı
sanal sınır hâlâ ARCH-SPACE-01 işidir. Katı atanmamış eski mahalın otomatik
katlandırılması da bu düzeltmenin kapsamı değildir.
