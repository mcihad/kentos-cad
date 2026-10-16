# ADR 0330: Stil pencerelerinde kayıt, bildirimler, araç şeritleri ve ifade alanları

- **Durum:** kabul edildi (2026-10-16). Sahibin isteği (16 Ekim). Altı iş:
  - Stil düzenleyicide “Geometri üretici uygulanmıyor” hatası;
  - Kaydet başarılıysa pencere kapanır;
  - bildirimler (toast);
  - SVG düzenleyicisi ve Sembol tasarımcısının üstündeki araçlar gerçek araç çubuğu gibi;
  - ifade seçilemeyen ifade girişi, ifade oluşturucu ve akışı;
  - formların tutarlılığı.

  Kapsamı ben belirledim.
- **Bağlam belgesi:**
  - ADR 0091 (Katman stili), 0094 (Sembol tasarımcısı), 0095 (SVG düzenleyicisi);
  - ADR 0100 §5 (İfade oluşturucu), 0101 (ifadenin akışı), 0226 (Geometri üretici);
  - DESIGN.md §7.16.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

- **Geometri üreticili sembol kaydedilemiyordu.** Kitaplığın denetimi (`kentos_native_style::file::validate_symbol`) katman türlerini kendi
  listesinden okuyordu. O listede `geometryGenerator` yoktu, alan sembolünde `gradientFill` da yoktu. Tasarımcı bu katmanları kurabiliyordu,
  Kaydet ise her türde “… sembolünde “geometryGenerator” katmanı olamaz” diyordu.
- **Kaydet pencereyi açık bırakıyordu.** Sembol tasarımcısında ve SVG düzenleyicisinde kayıt başarılı olsa da pencereyi Vazgeç kapatıyordu.
- **Başarı yalnız günlüğe yazılıyordu.** Alt panelin son satırında görünüp kayboluyordu.
- **İki pencerenin üst araçları ayrı çerçeveli düğmelerdi.**
  - SVG düzenleyicisinde her düğme kendi kutusunda, iki satıra taşıyordu.
  - Tasarımcının önizleme başlığında ölçek düğmeleri düz yazı gibiydi.
- **İfade alanları aynı değildi.**
  - Katman stili'nin ifadesinde Alanlar, Değişkenler ve İşlevler menüleri vardı.
  - Sembol tasarımcısının ƒ alanları ve Geometri üreticinin adım değerleri çıplak bir satırdı: alan ya da ifade seçilemiyordu.
  - İkisinden de İfade oluşturucu açılmıyordu. Yanlış ifade hiçbir şey söylemeden yedek değeri çiziyordu.
- **Akışta bir düğüm seçiliyken işleç ya da işlev eklemek onu sarmıyordu.** `$alan` seçiliyken + basılınca `+` akışın altına bağlantısız
  düşüyordu. Okunamayan metinde akış yalnız “Metin'de düzeltin” diyordu.

## Karar

### 1. Kitaplığın denetimi tasarımcının kurduğu her katmanı tanır

- `layer_types`'a üç türde `geometryGenerator`, alanda `gradientFill` eklendi.
- `walk` iki katmanın alanlarını denetler:
  - `gradientFill`: renk, ikinci renk, açı.
  - `geometryGenerator`: bilinen adım türleri, adımların sayıları (değer ya da ifade) ve çizen sembol.
- On beş adımın sayıları `STEPS` tablosundadır.
- Test: `what_the_designer_makes_the_library_saves`. Tasarımcının her türde kurabildiği her katman ve on beş adım kaydedilir.

### 2. Kaydet başarılıysa pencere kapanır, bildirim söyler

- **Sembol tasarımcısı:** Kaydet ya da Uygula başarılıysa kapanır.
  - Kitaplığa kaydedilince bildirim: ““Ad” kaydedildi” ve yeri.
  - Katman stilinin yuvasına konunca bildirim: “Sembol katman stiline uygulandı”.
- **SVG düzenleyicisi:** Kaydet başarılıysa kapanır ve bildirim gösterir. Farklı kaydet pencereyi açık bırakır, bildirim yine gösterilir.
- **Katman stili:** Tamam kapanınca bildirim gösterir.
- **Öbür pencereler:** başarılı kayıtları (`write_time`, `save_layer_fields`, `save_topology_rules`, `save_networks`, Proje ayarları …)
  `App::announce` ile hem günlüğe yazar hem bildirim gösterir.
  - İlk cümle bildirimin başlığıdır, kalanı açıklaması.
  - Uyarı ve hata düzeyleri kendi renkleriyle gösterilir.
- **Yer:** bildirimler kabuğun sağ altında, durum çubuğunun üstündedir; bir pencere açıkken onun alt düğmelerinin (Uygula, Tamam)
  üstünde durur: `toasts.rs`, KentOS UI'ın `Toasts` ve `Toaster`'ı.

### 3. Araç şeridi: `style/strip.rs`

- **Tek yüzey:** panel başlıklarının zemini, altında bir çizgi.
- **Düğmeler:** düz; kenar yalnız imleç üstündeyken görünür. Açık anahtarlar vurgu zeminli ve vurgu kenarlı.
- **Menüler:** yanındaki düğmeler gibi görünür, ▾ ile.
- **Gruplar:** arada kısa bir dikey çizgi.
- **Boyut:** her denetim satır içi yükseklikte. Dar pencerede grup grup sarar.
- **İpucu:** “Ad: ne yaptığı” biçimindeki metin, ipucunun başlığı ve gövdesidir.
- **SVG düzenleyicisi:**
  - Dosya ▾, Altlık, Bitmap izle, Dışa aktar, Kaynak;
  - Yol ▾, Nesne ▾, Seç ▾;
  - geri al ve yinele;
  - Kenet ▾ ve Cetvel;
  - yakınlaştırma.
  - Seyrek kullanılanlar yalnız ikonla; 1440 px'te tek satırdır.
- **Sembol tasarımcısı:**
  - Önizlemenin başlığı aynı şerittir: örnekler, −, ölçeğin kutusu, +, 1:1.
  - Katmanlar listesinin Katman ekle'si ve alt araçları aynı düğmelerdir.

### 4. Bir ifade alanı, her yerde aynı

- **`fields::expression` ve `expression_aids`:**
  - Satır monospace yazıyla.
  - Altında ya da yanında **Ekle ▾**: katmanın alanları ve kaç nesnede oldukları; Değişkenler ve İşlevler. Seçilen, yazının sonuna eklenir.
  - **Oluşturucu…:** İfade oluşturucu.
- **Tasarımcının ƒ alanları:** üst katman değerleri ve Geometri üreticinin `step.i.field` değerleri bu alanı kullanır.
  - **Katman:** Katman stili'nden açılan sembolde onun katmanı, öbürlerinde çizimin etkin katmanı (`App::designer_layer`).
  - **Hata:** okunmayan ifade kırmızı kenarlıdır; hatası altında yazar (`expression_problem`, oluşturucunun denetimi). Boş ifade hata
    değildir; yedek değer çizilir.
- **Katman stili:**
  - Değer, sınıf ve tematik ifadeler aynı yardımları alır.
  - Kural koşulunun ƒ menüsüne “İfade oluşturucu…” eklendi.
- **İfade oluşturucunun yeni hedefleri:** `Target::Symbol { at, key }` ve `Target::LayerStyle(field)`.
  - Tamam metni alanına yazar, yedeği korur.
  - Tasarımcıda oluşturucunun yazdığı kendi geri alma adımıdır.

### 5. Akış ve tamamlama

- **Sar** (`flow::Edit::Wrap { node, key }`, JSON'da `wrap`):
  - Paletten bir öğe düğümün yerine geçer; düğüm yeni öğenin ilk girişine girer.
  - Değer almayan öğe ve Sonuç sarılamaz.
  - Masaüstü kullanımı: seçili düğümün boş girişi yoksa, işleç düğmesi ya da ağaçta çift tık onu sarar. Sonuç seçiliyse bütün ifade
    sarılır. Yeni düğüm seçili kalır; sıradaki değer onun boş girişine gider.
- **Okunamayan metin:**
  - Akışın notunda “Metin'de düzelt” düğmesi vardır.
  - Akıştan Metin'e dönülünce hatanın aralığı seçilir.
- **Harf sırasıyla eşleşme:**
  - Tamamlama, üç ya da daha çok harfte, harfleri adda sırayla geçen kendi adları da bulur: `yvrl` yuvarla'yı, `tapalan` “Tapu
    alanı”nı.
  - Yalnız daha yakın bir eşleşme yoksa sunulur. Böylece sabitlenmiş yanıtlar (`fixtures/expression/v2/builder.json`) değişmez.

## Denetimler

- `cargo test -p kentos-style-library`
- `cargo test -p kentos-native-style --test all designer`:
  - `what_the_designer_makes_the_library_saves`.
- `cargo test -p kentos-expression`:
  - akışın `wrapping_puts_the_node_into_the_new_one_s_first_input`'u;
  - tamamlamanın harf sırası durumları;
  - `tests/builder.rs` ve `tests/flow.rs` değişmeden geçer.
- `cargo test -p kentos-desktop style:: expression::`:
  - tasarımcının kapanması ve bildirimi;
  - SVG düzenleyicisinin Kaydet'i;
  - `an_expression_takes_fields_and_the_builder_s_text`;
  - `a_step_s_expression_is_written_in_its_step`;
  - `an_expression_s_problem_is_said_where_it_is`;
  - Katman stili'nin `an_expression_is_written_with_the_builder`'ı;
  - akışta sarma;
  - `an_unreadable_text_goes_back_to_metin_at_its_error`.
- Resimler:
  - `style::svgedit::screens` (`svge-*`);
  - `style::designer::screens` (`sdes-*`);
  - `style::screens::layer_style_screens` (`lstil-*`);
  - `expression::tests::flow_screens` (`ifade-akisi-*`).

## Sonuçlar

- Tasarımcının kurduğu her sembol kaydedilir. Kitaplığın denetimi yeni bir katman türüyle birlikte değişmelidir; test bunu ister.
- Kayıt pencereyi kapatır; başarıyı bildirim söyler. Kayıt başarısızsa pencere açık kalır ve nedeni penceredeki alanların yanında yazar.
- Stil pencerelerinin üst araçları tek bir şerit bileşenindedir. Yeni bir araç oraya `strip::…` ile gelir.
- İfade yazılan her stil alanı aynı yardımları ve İfade oluşturucuyu sunar.
