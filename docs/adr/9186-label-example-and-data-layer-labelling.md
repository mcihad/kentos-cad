# ADR 9186: Etiketler örneği haritanın biçimiyle çizer; veri katmanında örnekler ve Denetle

- **Durum:** kabul edildi (2026-11-23); adım VY-85. Plan: [plan/veri.md](../../plan/veri.md) VY-85.
- **İstek:** sahibin sözü (23 Kasım 2026): “büyük arayüz işi (3, 12, 13, 15) ile devam”. Değerlendirmenin 13. maddesinin
  kalanı (ADR 9145, 9160, 9164, 9167): “etiket önizlemesinin kendi biçimiyle çizimi, veri katmanında etiket.” Maddenin
  kabulü:
  - etiket şablonu için alan seçici, gerçek ilk satırdan örnek metin;
  - aday → çözümlenen metin → yerleşen ve gizlenen sayıları;
  - ölçek, görünürlük, boş değer, en küçük nesne ve çakışma ayrı nedenler;
  - tablo “planlı” dediğinde önizleme de “planlı” göstermeli ya da niçin görünmeyeceğini açıklamalı.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam ve araştırma

- Etiketler penceresinin örnekleri (ADR 9164) metinleri yazıyla veriyordu: “Örnek metin (ilk nesne): “planlı”.” Etiketin
  rengi, halesi, zemini, gölgesi ve yazı boyu görünmüyordu; harita açılmadan etiketin nasıl görüneceği anlaşılmıyordu.
- Veri katmanının satırları kaynağındadır, çizimde değil (ADR 9101). Pencerenin örnekleri, boş metinleri ve Denetle'nin
  adayları çizimin nesnelerini okuyordu. Veri katmanında hiçbirini bulmuyordu: özet “0 nesne” diyordu, Denetle “0 etiket
  yerleşti” diyordu. Oysa haritada etiketler görünümün okuduğu satırlara yerleşiyordu (data_labels.rs).
- **Araştırma.** QGIS 3.44 kılavuzunun etiket ayarları bölümü
  ([label_settings.html](https://docs.qgis.org/3.44/en/docs/user_manual/style_library/label_settings.html), okundu
  23 Kasım 2026): etiket ayarları penceresi bir metin örneğini ayarların biçimiyle gösterir. Maske sekmesinin resminde
  örneğin arkasında, başka bir katmanı temsil eden yeşil bir zemin vardır.

## §2 Karar

1. **Örnek haritanın biçimiyle çizilir** (`labels/example.rs`).
   - Pencerede “Haritada böyle görünür (gerçek boyunda)” şeridi örneklerin ilk metnini gösterir. Şerit, çizim alanının
     zemininde ve tek etikette o stille, Kurallı'da seçili sınıfın stiliyle çizer.
   - Görünüş haritanınkiyle aynı işlevdendir (`placed_look`, haritanın yerleşen etiketlerinden ayrıldı): renk,
     mürekkep, hale, zemin (dikdörtgen, yuvarlak, elips; dolgu ve çizgi), gölge, kalınlık, eğiklik ve projenin yazı tipi.
     Yazı boyu stilin boyudur, 40 px'te kesilir. Görünüm kipi (renkli, tek renk, gri) de uygulanır.
   - Etiketleme Yok ya da örnek metin yokken şerit görünmez.
2. **Veri katmanının örnekleri kaynağından** (`labelling/rows.rs`).
   - Pencere bir veri katmanında açılınca ya da katman değişince tablonun ilk 64 satırı arayüzü bekletmeden okunur.
     Okuma, çizimin okuduğu gibidir: projenin sistemine çevrilir, katmanın bildirdiği sütunlar alınır. Satırlar kendi
     belgelerinde, katmanın nesneleri olarak tutulur.
   - Örnekler, en uzun metin, boş metinler ve şeridin metni bu satırlardan gelir. Metinler motorun kendi metinleridir
     (`texts_of`).
   - Okunurken “Örnekler için kaynağın ilk satırları okunuyor…”, okununca “Örnekler tablonun ilk 6 satırından.” denir.
     Okunamazsa nedeni söylenir. Sonraki okuma öncekinin geç gelen cevabını düşürür.
   - Özet veri katmanını nesne saymaz: “parseller: veri katmanı, satırları kaynağında; tek etiket.”
3. **Denetle veri katmanında.**
   - Görünümün okuduğu satırlar tutuluyorsa etiketler orada, çizimin etiketleri gibi sayılır. Katman bağsız sayılır,
     dosyaya bağlı tabloların bantları karışmaz.
   - Görünüm henüz okumadıysa pencere bunu söyler: etiketler görünümün okuduğu satırlara yerleşir, etiketleme kapalıysa
     önce Uygula ile açılmalı. Adaylar, örneklerin satırlarından aynı nedenlerle verilir.

## §3 Kanıt

- **Masaüstü** (`labelling/data_tests.rs`), bir GeoPackage'ın altı parseli veri katmanı olarak:
  - `a_data_layers_examples_come_from_its_sources_first_rows`: okunurken okunduğu söyleniyor. `{kullanim}` şablonuyla
    örnek “Konut” ve “ilk 6 satır”. Katman nesne sayılmıyor; alan seçicide tablonun alanları var; tabloda olmayan alan
    söyleniyor.
  - `denetle_counts_a_data_layers_labels_on_the_rows_the_view_read`: okuma yokken açıklama ve örnek satırlardan adaylar
    geliyor. Görünümün okuduğu satırlar tutulunca “Görünümde N etiket yerleşti” diyor, N > 0.
  - `the_first_text_is_drawn_in_the_styles_look`: şerit örnek varken görünüyor, Yok'ta görünmüyor.
- Haritanın etiket sınamaları (`labels`, `labelling`, bant sınamaları) görünüşün ayrılmasından sonra da geçiyor.
- **Resimler:** `.run/gui-evidence/kullanim-9186`: veri katmanında örnek; 16 px kalın, yuvarlak zeminli ve gölgeli
  etiket (Git'e girmez).

## §4 Kalan

- Şeridin eğri ve çok satırlı etiketleri (bugün tek satır, düz).
- Gerçek pencerede deneme; web (TODOS.md §0.2).
