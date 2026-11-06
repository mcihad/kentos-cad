# ADR 1022: Blok içindeki çok satırlı yazı da gömülü SHX fontuyla çizilir

- **Durum:** kabul edildi (2026-11-06). Kapsam: plan/cad-advanced.md `CAD-ADV-11`, ADR 1020 §2'nin sınırı (“Çok
  satırlı ve eğri boyunca blok yazısı ... hâlâ stilin yüzüyle çizilir”).
- **Bağlam belgesi:** ADR 1020, ADR 1125 §2 (SHX yazısı ilk satırın yerinde bütün çizilir), ADR 0182 §3.

## 1. Karar

- Görünümün satır kaydı (`LabelSpot::Line`) ve paragraf maskesi (`LabelSpot::ParagraphMask`) bir bloğun parçası için
  parçanın yerleşmiş biçimini taşır (`placed`, deponun açılmış `Shape::Text`'i). Parça gömülü SHX stilindeyse ve font
  onu çizebiliyorsa, yazı nesnesinde olduğu gibi ilk satırın kaydında bütün paragraf fontun çizgileriyle çizilir (satır
  aralığı, kutuya kaydırma, hiza ADR 1125'in yerleşimi), öbür satır kayıtları ve kutu biçimli maske atlanır; maske
  yazının kendi kutusudur. Ekran, PDF ve CPU baskısı aynı yolu kullanır.
- `shx_text::placed_piece`: yerleşmiş parçadan, kendi sözcükleriyle bir yazı nesnesi.
- Denetim: blok yazısı için ayrı “SHX ile çizilmiyor” uyarısı kalktı; blok yazısı yazı nesnesinin kuralını alır (font
  çizemiyorsa `text_shx_unsupported` hatası; eğri boyunca yazı da buna girer, çünkü SHX yerleşimi onu reddeder).

## 2. Kanıt

`kentos-plot` `shx_texts::a_blocks_text_prints_in_its_embedded_shx_font_where_the_insert_puts_it` beş başvuru durumuyla:
tek satır, çarpan/eğiklik/dönüş, Big Font, **üç satır aralık 1,5 üst sağ** ve **kutuya kaydırma ortalı**; kaydırılmış ve
çeyrek dönmüş yerleştirmede PDF çizgileri bağımsız başvurununkiyle 1e-9 içinde aynı, yerine yüz yazılmaz, denetim
yerine geçme demez. Resim `shx-blok-metin`: iki yerleştirmede tek ve iki satırlı yazılar, bakıldı.

## 3. Sınırlar

Özniteliğin çok satırlı değeri bloğun varsayılan sözcükleriyle değil, parçanın kendi sözcükleriyle çizilir (çok satırlı
öznitelik değeri bu dilimde sınanmadı). Eğri boyunca SHX yazısı yine çizilmez (hata olarak söylenir).
