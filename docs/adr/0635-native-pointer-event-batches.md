# 0635 — Native fare olay grubunda basış konumu

25 Ekim 2026 · ARCH-BASE-01 / ARCH-OPEN-01 takip · Uygulandı; bu dilimin kabulü tamamlandı.

## Kanıtlanan sorun

Gerçek Kaydır sürüklemesi aralıklı olarak kamerayı yerinde bırakıyordu.
İzleme eklenmemiş önceki derlemede iki ekran arasındaki duvarlar aynı yerdeydi.
Yalnız test derlemesine eklenen geçici olay kaydı ters sürüklemede şunu gösterdi:

- `CursorMoved.position=(740,523)`, Iced `cursor=(560,448)`.
- Sonraki `ButtonPressed` aynı son `cursor=(560,448)` ile gelir.
- Ardından hareket `(560,448)` ve bırakış `(560,448)` olur.

Eski adaptör basışı (560,448)'de başlatıyor; sonraki hareketin farkı sıfır
olduğu için kamera yerinde kalıyordu. Olay kaydı zamanlamayı etkileyebildiğinden
izleme sürümünün tek başarılı sürüklemesi çözüm kanıtı sayılmaz.

Iced 0.14.1 `iced_winit` bir pencerenin olaylarını `UserInterface::update`'e
birlikte ve pencerenin son `cursor` değeriyle verir. `iced_runtime` her olaya
aynı cursor'u iletir. `Snapshot::input` ise olayları tek tek verip uygulama
mesajlarını aralarında işler; tek başına bu hatayı sınayamaz.
Yeni test eski kodda basışı (50,20) yerine (250,120) olarak yakaladı.

## Sınırlı düzeltme

- Yalnız desktop viewport fare adaptörü, son hareket olayının mutlak konumunu
  ve o teslimin Iced cursor'unu geçici olarak tutar.
- Aynı teslim cursor'u sürdükçe basış/bırakış/teker konumu olay sırasından gelir.
  Yeni Iced cursor'u eski kaydı geçersiz kılar.
- `Levitating` ve `Unavailable` durumu korunur: menü/üst katman üzerinden yeni
  tıklama veya zoom başlamaz. Daha önce başlayan sol/orta tutmanın mevcut
  bırakma davranışı korunur.
- Yeni çizim karesi, pencereyi terk etme ve bounds değişimi kaydı temizler.
- Kamera matematiği, model FP64, render, ölçek/birim, snap toleransı, çizim
  komutları ve dosya modeli değiştirilmez. Geçici olay günlüğü ürün koduna alınmaz.

## Kabul

- Hareketten önceki doğru basış; orta/sağ/teker sırası; dışarıda biten grup;
  menü/erişilemeyen cursor; yeni cursor ve pencereyi terk etme testleri.
- GPU testi gerçek runtime gibi bütün grubu tek `update` çağrısıyla verir;
  kamera ileri/geri tam (80,40) px gider, belge değişmez.
- Tam desktop regresyonu, clippy ve izleme kodu olmayan gerçek yeni derlemede
  iki yönde tekrarlı Kaydır; mimari seçim, açıklık yerleştirme ve sağ tuş
  menüsünde tek Onayla ile bitirme; zoom, tek undo/redo, kaydet/aç kontrolü.

Kabul ve önce/sonra görüntüler:
[2026-10-25-architecture-pointer-batches](../baseline/2026-10-25-architecture-pointer-batches/README.md).
ARCH-BASE-01 ve ana mimari plan bu tek düzeltmeyle tamamlanmış sayılmaz.

Son derlemede 1279 desktop, 14 ilgili viewport ve toplu olay GPU testi geçti;
clippy temiz. Gerçek altı sürükleme, kapı yerleşimi/undo/redo ve yeniden açma
doğrulandı. Kenet aday kimliği takibi ve tam gerçek pencere matrisi açık kalır.
