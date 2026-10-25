# ADR 0639 — Duvar dönüşümünde açık kat ve katlar arası birleşim ayrımı

Durum: bu uygulama dilimi kabul edildi; genel mimari yol haritası açıktır. Tarih: 25 Ekim 2026.

`ARCH-DRAW-03` için mevcut kat veri modeli kullanılır. Genel CAD alan/çizgi
nesneleri mimari kat taşımaz; adlarından, özniteliklerinden, katmanından veya
kotundan kat tahmin edilmez.

## Karar

- Mevcut `cad.walls.convert` komutunun isteğe bağlı `arch` alanı bütün çıktı
  duvarlarının mimari bilgisidir. `arch.level` mevcut proje katının kimliğidir;
  bulunmayan kat `level_not_found` ile kaynak silmeden, atomik reddedilir.
  Each, Pairs ve Contours aynı `NewWall` doğrulamasından geçer.
- Kat seçimi 2B planın XY koordinatlarını kaydırmaz; mevcut kat kimliği metaverisidir.
- Alan verilmezse eski katsız davranış sürer. Sözleşme sürümü ve KCAD şeması
  değişmez; `ArchInfo` ve katlar zaten kalıcı modeldedir.
- Çizgiden duvar aracında görünür Kat (L) seçimi ve mevcut Katları düzenle
  komutu vardır. Yeni araç oturumu Belirtilmedi ile başlar; seçilen kat yalnız
  o taslağın açık seçimidir. Kaldırılan kat sessizce katsıza çevrilmez.
  Yinelenen adlarda seçim tahmin edilmez; kimlikler büyük/küçük harfe duyarlıdır.
- Kontur tekrarı yalnız aynı hedef katman ve aynı semantik katta reddedilir.
  Ortak sınır ağı da bu grupların içinde kurulur.
- Ortak FP64 birleşim çözücüsü, iki duvarın kat kimlikleri aynıysa birleşim
  kurar; katsız duvar yalnız katsızla birleşir. Eşit kot farklı kimlikleri
  birleştirmez. Köşe/uç grupları, T ve çapraz adaylar aynı kurala tabidir.
  Komşu duvarın katı değiştiğinde mevcut depo yeniden kurma yolu kullanılır.

## Sınır

Bu dilim etkin kat/görünür kat filtresi, kat planı kopyalama veya katlara göre
mahal üretimi sağlamaz. `ARCH-DRAW-03` bütün kabul matrisi tamamlanmadan
bitmiş işaretlenmez. Mevcut birim/ölçek, kamera ve renderer değişmez.

## Doğrulama

4.093 test, optimize WASM, clippy ve 20 gerçek GUI JPEG
`docs/baseline/2026-10-25-architecture-wall-levels/` içindedir. Tam komutlar,
ham çıktılar, bağımsız KCAD doğrulaması ve açık kabul kapıları aynı kayıttadır.
