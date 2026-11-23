# ADR 9164: Etiket penceresi aynı veriden örnekler gösterir

- **Durum:** kabul edildi (2026-11-23); adım VY-63. Plan: [plan/veri.md](../../plan/veri.md) VY-63.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Değerlendirmenin 10. maddesinin
  kalanı (ADR 9143): “Etiket önizlemesinde aynı verinin boş ve uzun adları, yakın/uzak ölçek ve baskı örneği
  görülsün.”
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam

- Etiketler penceresi şablon yazıldıkça yalnız ilk nesnenin metnini söylüyordu (ADR 9145). Metni boş ilk nesneler
  varsa sayılarını da ekliyordu.
- Kişi şunları göremiyordu:
  - en uzun adın nasıl çıkacağını;
  - kaç nesnenin hiç etiket almayacağını;
  - etiketin hangi ölçeklerde göründüğünü ve şimdiki görünümün bunlardan olup olmadığını;
  - yazının kâğıtta ne kadar büyük olacağını.
- **Araştırma.** Masaüstü CBS'lerin etiket ayarında yazı biçimi (boy, renk, gölge) ve çizim sekmesinde ölçeğe ve
  piksel boyutuna bağlı görünürlük ayarları vardır; ayar penceresi bir metin örneği gösterir. Kaynak:
  [QGIS “Setting a label”, Text Format ve Rendering sekmesi](https://docs.qgis.org/3.44/en/docs/user_manual/style_library/label_settings.html).

## §2 Karar

1. **Aynı veriden** (`labelling/examples.rs`). Pencere, katmanın ilk 64 nesnesinin metinlerini motorun kendi
   kodundan alır (`texts_of`); şablon ya da ifade yazıldıkça yenilenir. Şunları söyler:
   - ilk metni: “Örnek metin (ilk nesne): “P1”.”;
   - ilkinden farklıysa en uzun metni, 40 harfte kesip bütün uzunluğuyla: “En uzun metin: “Atatürk Bulvarı Kuzey Yan
     Yolu Kavşağı D…” (48 harf).”;
   - metni boş olanları: “İlk 9 nesnenin 4 tanesinde metin boş; onlara etiket yazılmaz.”
2. **Yakın ve uzak ölçek.** İlk sınıfın ölçek aralığı ve şimdiki görünümün ölçeği söylenir. Motor aralığı nasıl
   okuyorsa öyle okunur (`scale::visible_at`):
   - “Ölçek: 1:5.000 ile 1:500 arasında görünür; şimdiki görünüm 1:1.000, görünür.”;
   - tek uçlu aralık: “1:2.000 ve daha yakın ölçeklerde görünür”; aralıksız: “Her ölçekte görünür”;
   - görünüm aralığın dışındaysa satır uyarıdır (“… görünmez.”).
3. **Baskı.** Yazının boyu CSS pikselidir. Kâğıttaki yüksekliği inçte 96 piksel üzerinden söylenir: “Baskı: yazı
   10.5 px, kâğıtta yaklaşık 2.8 mm.” Ölçekle büyüyen yazının en büyük boyu da söylenir.
4. Pencerenin eski ilk-metin satırı bu örneklerin ilkidir. Ayrı `sample` yardımcısı kaldırıldı.

## §3 Kanıt

- **Masaüstü** (`gis/labelling/examples_tests.rs`):
  - Çizim katmanında `{Ad}` şablonuyla ilk metin “P1”;
  - 48 harflik ad 40 harfte kesiliyor, uzunluğu söyleniyor;
  - 9 nesnenin 4'ünün metni boş;
  - pencerenin yeni etiketinin aralığı 1:1.890'dan yakın ve uydurulmuş görünüm içinde;
  - baskıda 10.5 px, 2.8 mm.
- Ölçek sözleri iki uçlu aralıkta (görünür ve görünmez görünümle) ve tek uçlu aralıkta kelimesi kelimesine sınanıyor;
  baskı sözü büyüyen yazıyla da sınanıyor.
- Etiket, iz ve ortak masaüstü sınamaları geçiyor. İki iz (2+1 dairenin saveAndReopen adımı) paralel koşuda düştü,
  tek başına geçti. Bu bilinen bir kararsızlıktır.
- **Resim:** `.run/gui-evidence/kullanim-9164/01-etiket-ornekleri-{light,dark}.png`. Etiketler penceresinin alt
  satırlarında ilk metin, en uzun metin, boş sayısı, ölçek ve baskı.

## §4 Kalan

- 10. maddenin öbür kalanları: stil/QML alışverişinde kayıpların tek tek bildirimi, planlı→kontrol geçişinde uçtan
  uca sınama.
- Etiket önizlemesinin kendi biçimiyle (yazı, hale, arka plan) çizimi (ADR 9145 §4).
- Kurallı etiketlemede her sınıfın aralığı; şimdilik ilk sınıfınki söylenir.
- Web (TODOS.md §0.2).
