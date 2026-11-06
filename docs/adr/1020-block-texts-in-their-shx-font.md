# ADR 1020: Blok içindeki tek satırlık yazı gömülü SHX fontuyla çizilir

- **Durum:** kabul edildi (2026-11-06). Kapsam: plan/cad-advanced.md `CAD-ADV-11`, ADR 1125 §3'ün sınırı “Blok
  içindeki yazı (`PieceText`) bu dilimde SHX ile çizilmez”.
- **Bağlam belgesi:** ADR 1125 (yazı nesnesinde SHX), ADR 0144 (bloğun yazıları ve öznitelikleri), ADR 0182 §3.

## 1. Kaynak

AutoCAD'de blok tanımındaki TEXT/ATTDEF yazı nesnesiyle aynı stil ve fontla çizilir; yerleştirmenin noktası, dönüşü
ve ölçeği yazının noktasına, dönüşüne ve yüksekliğine geçer (DXF INSERT'in 10/41/50 grupları). SHX yazısının kendisi
ADR 1125 §1'in Autodesk kaynaklarıyladır. KentOS kararı: blok yazısı ayrı bir SHX kuralı
almaz, yazı nesnesinin kuralını yerleşmiş hâliyle kullanır.

## 2. Karar

- Görünümün etiket kaydı `LabelSpot::PieceText` artık parçanın **yerleşmiş** biçimini taşır (`placed`: deponun
  açılmış `Shape::Text`'i; bloğun kendi koordinatındaki parça değil). `shx_text::piece_text` bundan, gösterdiği
  sözcüklerle (özniteliğin değeri ya da varsayılanı) bir yazı nesnesi kurar; `place_cached` onu ADR 1125'in yerleşimiyle
  çizer.
- Ekran (`labels.rs`), PDF ve CPU baskısı (`kentos_plot::labels`, `MapLabel.shx`) aynı çizgileri çizer; kalınlık
  yerleştirmenin (yoksa katmanın) çizgi kalınlığıdır, maske yazının kendi kutusudur.
- Çok satırlı ve eğri boyunca blok yazısı satır satır (`Line` kayıtları) çizildiği için bu dilimde hâlâ stilin yüzüyle
  çizilir; denetim bunu `text_font_substituted` uyarısıyla “Blok içindeki çok satırlı ya da eğri boyunca yazı SHX
  fontuyla çizilmiyor” diye söyler. Tek satırlık blok yazısında font çizemiyorsa yazı nesnesi gibi
  `text_shx_unsupported` hatası, font gömülü değilse adlandırılmış yerine geçme uyarısı.

## 3. Kanıt

`kentos-plot` `shx_texts::a_blocks_text_prints_in_its_embedded_shx_font_where_the_insert_puts_it`: bağımsız başvurunun
(`fixtures/shx/v1/expected.json`, `shx_font_reference.py`) üç yazısı, yerleştirmesi kaydırılmış ve çeyrek dönmüş bir
bloğa, dünyada aynı noktaya ve dönüşe gelecek biçimde konur; PDF'in çizgileri başvurununkiyle 1e-9 içinde aynıdır,
yerine yüz yazılmaz, denetim yerine geçme ya da çizilemez demez. İlk koşu gerçek bir hata buldu: blok kitaplığının
parçası bloğun kendi koordinatındaydı (çizgiler yerleştirmenin noktası kadar kaymıştı); yerleşmiş parça kullanıldı.
Resim `shx-blok-metin`: düz ve 30° dönmüş, 1,5 ölçekli iki yerleştirme.

## 4. Sınırlar

Çok satırlı ve eğri boyunca blok yazısı; boyut değeri, kılavuz notu ve tablo hücresi SHX ile çizilmez. Seçim kutusu
TrueType genişliğindedir. Blok parçasının kendi çizgi kalınlığı SHX çizgisine geçmez (yerleştirmeninki kullanılır).
