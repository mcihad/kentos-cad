# ADR 9212: Çizim yazıları GPU'da (tasarım)

- **Durum:** önerildi, aşama aşama uygulanıyor (2026-11-24). Plan: [plan/render.md](../../plan/render.md) RM-13.
- **İstek:** sahibin seçimi (24 Kasım 2026): “Çizim yazıları GPU'da”. İlk istekteki hedef: “çizim alanı saf wgpu
  olmalı”.
- **Bağımlılık:** ADR 0055 (masaüstünde çizim yazısı), 0182 (paragraf), 0196 (eğri boyunca yazı), 0212 (etiket
  motoru), 0292 (ayna kuralı), 0597 ve 0626 (hareket boyunca yerleşim), 0786 (küçük yazı bandı), 9207 §5.

## §1 Bugün

- **Kim çiziyor:** çizim yazıları (nesne yazıları, ölçü değerleri, etiketler, eğri boyunca harfler, çağrı kutuları)
  `apps/desktop/src/cad/labels` içinde iced'in `Canvas`'ına çiziliyor (5 252 satır).
  - Dik yazı iced'in glif önbelleğinden (cosmic-text) gelir.
  - Döndürülmüş yazı, glif dış hatları olarak `Path` ile doldurulur.
  - Hepsinin çevresinde zemin renginde bir hale var.
- **Yerleşim:** paylaşılan geometri deposundan gelir (`Spatial::labels`): görünür katman, görünümdeki kutu,
  ekrandaki boy, ölçek aralığı ve etiketlerin seyreltilmesi. Nesne yazısı ve ölçü değeri hiç düşmez.
- **Maliyet** (ADR 9207 §5, NCZ, UHD 770):
  - yazılar çizilmeyince uzak görünümün tekerlek karesi 76–93 ms'den 54–90 ms'ye iniyor;
  - yakın karelerde 20 ms'ye kadar iniyor, yani yakında yazılar karenin çoğu.
  - Kanvas her değişen karede yeniden üçgenleniyor; tuvali arayüz kütüphanesine bağlayan son büyük parça bu.
- **Yazı tipleri:** çizicinin gömülü TrueType yüzleri (`kentos_plot::fonts::FACES`). Ekran ve PDF aynı baytları
  kullanır.

## §2 Seçenekler

| Seçenek | Yakında | Hale | Bağımlılık | Not |
|---|---|---|---|---|
| Bugünkü kanvas | keskin (vektör) | yol olarak ikinci dolgu | iced | her karede üçgenleme, arayüz katmanında |
| Boyuna göre bitmap atlası (glif önbelleği gibi) | büyütülünce bulanık | ayrı bitmap | yok | SVG simgesindeki sorunun aynısı (ADR 9208) |
| Tek kanallı SDF atlası | ~8× büyütmeye dek keskin, sonra köşeler yuvarlanır | eşiğin kaydırılmasıyla bedava | yok (ttf-parser zaten ağaçta) | harita motorlarının yolu |
| MSDF atlası | çok büyük büyütmede de köşe keskin | aynı | dış kütüphanenin (fdsm) lisansı belirsiz; kendi üreteç ~600 satır | kenar renklendirme ve hata düzeltme ister |
| Vektör glif ağı (ADR 9208'in yolu) | her boyda keskin | dış hattın kalınlaştırılması | yok | küçük boyda 4× MSAA ince çizgiyi kırar (9208'in bulgusu) |

## §3 Karar

**Tek kanallı SDF glif atlası; çok büyük yazı için vektör glif.** Yerleşim ve düzen CPU'da bugünkü gibi kalır;
yalnız **çizim** GPU'ya geçer.

- **Atlas** (`kentos-render-wgpu`):
  - Her (yüz, glif) bir kez SDF'e çevrilir: 64 px'lik em, 8 px yayılma, R8 doku.
  - Kaynak, çizicinin yüzlerinin dış hatlarıdır (ttf-parser).
  - Sayfa dolarsa yeni sayfa açılır; bitmap atlasındaki gibi boy küçültülmez.
- **Çizim:**
  - Glif başına örneklenen bir dörtgen: yer (yüksek/alçak kesin), boy, döndürme ve eğim (ayna ve italik dahil),
    renk, hale rengi ve kalınlığı.
  - Kenar yumuşatma, ekran türeviyle (`fwidth`) yapılır; MSAA'ya gerek kalmaz.
  - Hale, aynı örnekte eşik kaydırılarak çizilir.
- **Çok büyük yazı:** glif ekranda ~512 px'i geçince, ADR 9208'in yoluyla vektör olarak (üçgenlenmiş dış hat)
  çizilir. Köşeler her boyda keskin kalır.
- **MSDF** ileride, köşe yuvarlanması ölçüyle görülürse SDF'in yerine geçebilir; arabirim aynı kalır.
- **Kural korunur:** nesne yazısı ve ölçü değeri hiç düşmez. Okunamayacak kadar küçük yazı bugünkü gibi banttır
  (ADR 0786).

## §4 Aşamalar

1. **Atlas ve gölgelendirici** (`kentos-render-wgpu`): SDF üretimi, glif örneği, hale, GPU sınaması (bir yazı
   döndürülmüş ve haleli, bütünüyle çizilmiş referansla karşılaştırılır).
2. **Nesne yazıları ve parçaları** (`LabelSpot::Text`, `PieceText`): masaüstü, glif örneklerini bugünkü paragraf
   düzeninden kurar ve yazının kanvas yolu kalkar. Görüntü önce ve sonra karşılaştırılır.
3. **Ölçü değerleri, etiketler, eğri boyunca harfler, hücreler, çağrı kutuları:** her biri ayrı adım.
4. **Kanvas katmanı kalkar.** Tekerlek ve kaydırma karelerinde yazılar da canlı kamerayla GPU'da çizilir.

Her aşama kendi ölçümüyle (yakın ve uzak tekerlek karesi, yazı sayısı) ve gerçek pencere görüntüleriyle kapanır.
