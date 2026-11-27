# ADR 9357: Etiketler ve Katman stili pencereleri: kurallı sınıflar, değer listesi ve Değer yok, etiket araçlarının ilk sözü, nokta etiketinin yeri, katmanın yolu, tek bildirim

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 27 Kasım 2026, kullanım turu 6
  (`gis-eksik-27-11-2026.md`: T22 P2, T23 P2, T24 P2, T25 P3, T26 P2, T27 P3). Sahibin sözü: “Gis eksik 27 11
  dosyasındaki yeni maddeleri tamamla”.
- **İlgili:**
  - [ADR 0212](0212-label-engine.md) (etiket motoru ve araçları), [ADR 9186](9186-label-example-and-data-layer-labelling.md)
    (etiket örneği, veri katmanının etiketleri), [ADR 9341](9341-info-card-actions-last-expression-and-data-labels.md)
    §3 (etiket araçları veri katmanının etiketinde);
  - [ADR 9110](9110-classes-from-the-source.md) (sınıflar kaynaktan);
  - [ADR 9316](9316-labels-are-gis-only.md) (etiket CBS'nindir).

## Bağlam

- **T22:** Kurallı Etiketler'de varsayılan boyutta sınıf listesi 200 px sabitti. Altındaki ekleme, çoğaltma, taşıma,
  silme, Ad ve Koşul görünmüyordu; kullanıcı pencereyi büyütmek zorundaydı.
- **T23:**
  - Değer listeli alanla Değerlerden sınıfla, sınıfın etiketini ham kod yaptı (“Y”).
  - İki boş kayıt kapalı Diğer değerler'e düştü, başta çizilmedi.
- **T24:** yalnız veri katmanı etiketleri varken Etiketi taşı, sabitle ve gizle tıklama istedi; nedeni ancak tıklayınca
  geldi, durum çubuğunda yarım okundu.
- **T25:**
  - Veri katmanındaki nokta etiketi ilk ayarda sembolün ortasına geldi.
  - Önizleme yalnız metni gösterdi, yerleşim farkını göstermedi.
  - Nokta katmanında çizgi ve alan seçenekleri aynı ağırlıktaydı.
- **T26:** iki aynı adlı katman (biri grupta) Katman stili başlığında ayırt edilemiyordu.
- **T27:** Uygula ardından Kaydet iki bildirim verdi (“yazıldı”, “zaten böyle”). “Kaydet” sözü proje dosyasına yazım
  sanıldı.

## Araştırma

- **QGIS, Rule-based labeling:** kurallar ağacı pencerenin kalanını kaplar. Ekle, düzenle, sil düğmeleri ağacın
  altındadır; kuralın süzgeci düzenleme penceresinin başındadır
  ([Rule-based labeling](https://docs.qgis.org/latest/en/docs/user_manual/working_with_vector/vector_properties.html#rule-based-labeling)).
- **QGIS, Categorized renderer:** Classify değer listesi (Value Map) olan alanda kategorilerin adını listenin
  etiketlerinden alır. Boş değerler için ayrı bir kategori oluşur (“all other values” dışında)
  ([Categorized Renderer](https://docs.qgis.org/latest/en/docs/user_manual/working_with_vector/vector_properties.html#categorized-renderer)).
- **QGIS, etiket yerleşimi:** noktada varsayılan “Around point”tır; uzaklık sembolün kenarından ölçülür. Önizleme
  örnek metni gösterir
  ([Placement](https://docs.qgis.org/latest/en/docs/user_manual/style_library/label_settings.html#placement-tab)).
- **QGIS, etiket araçları:** haritada etiket taşıma ve sabitleme yardımcı saklama (auxiliary storage) ister. Araç
  kullanılamıyorsa nedenini araç seçilince söyler
  ([Customize the labels from the map canvas](https://docs.qgis.org/latest/en/docs/user_manual/working_with_vector/vector_properties.html#customize-the-labels-from-the-map-canvas)).
- **KentOS kararı:**
  - Sınıf listesi esner; araçları, adı ve koşulu altta sabittir.
  - Değer listeli kod adıyla, boş kayıt kendi sınıfıyla çizilir.
  - Etiket araçları veri katmanı etiketinde baştan söyler.
  - Nokta etiketi çevresindedir; önizleme sembolle çizer.
  - Pencere katmanı yoluyla adlandırır; Uygula tek kez söyler, Tamam kapatır.

## Karar

### 1. Kurallı sınıflar (T22)

- Sınıf listesi kalan yüksekliği alır.
- Altında sabit:
  - sınıf ekle, kopyası, yukarı, aşağı, sil;
  - Ad;
  - Koşul ve İfade oluşturucu.
- Varsayılan pencere boyutunda (860 × 640) önizleme ve tanı doluyken de sınıf eklenir, adlandırılır, koşul yazılır.

### 2. Değer listesi ve Değer yok (T23)

- **Ad:** Değerlerden sınıfla, değer listesi olan alanda sınıfı listenin etiketiyle adlandırır: `M1 → Çınar`.
  - Sınıfın değeri koddur; eşleşme kodla olur.
  - Listede olmayan kod kendisiyle kalır (`M9`).
  - Kullanıcının değiştirdiği ad yeniden sınıflandırmada korunur (eski sınıf değeriyle eşleşir).
- **Değer yok:** alanın boş kayıtları varsa (veri katmanında kaynağın gruplarından, çizimde nesnelerden) değeri boş,
  adı “Değer yok” olan kendi sınıfı eklenir ve çizilir.
  - Motor boş ve olmayan değeri boş değere eşler (`resolve.rs`); Nesne sütunu boş kayıtları o sınıfta sayar.
  - İfadeyle sınıflandırmada (`$alan` gibi) eklenmez: ifadenin boş sonucu alanın kaydı değildir.
- **Diğer değerler:** kapalı Diğer değerler uyarısı ve açma düğmesi önceki gibidir; sayısı görünür kalır.

### 3. Etiket araçlarının ilk sözü (T24)

- **Araç açılınca:** Etiketi taşı, döndür, sabitle ya da gizle, haritada yalnız veri katmanlarının etiketleri
  görünürken açılırsa (görünen çizim katmanında etiket yok) hemen söyler.
  - Söz: “Haritadaki etiketler veri katmanının (“parseller”): kaynağın satırlarından yerleşir, tek tek taşınmaz,
    döndürülmez, sabitlenmez ve gizlenmez …”.
  - Bildirim bütün okunur; “Yerleşim ayarlarını aç” eylemi Etiketler'i o katmanda Yerleşim sekmesinde açar. Kurallı
    ve koşulu aynı penceredir.
- **Karma sahne:** çizim etiketleri de varken araç onlarda çalışır. Tıklanan veri katmanı etiketi için ADR 9341'in
  sözü aynı eylemli bildirimle söylenir; araç sürer.
- **Tekil kaynak etiketi:** iğnenin saklanması bu dilimde yoktur (ADR 9341 §3, Açık).

### 4. Nokta etiketinin yeri ve önizleme (T25)

- **Yeni nokta etiketi:** Tek etiket'in başlangıç stili nokta katmanında Nokta › Çevresinde'dir (sembol kenarını
  hesaba katar). Katmanın türü veri katmanında tablonun geometrisinden, çizimde nesnelerden okunur. Kayıtlı yerleşim
  olduğu gibi kalır.
- **Önce kendi türü:** Yerleşim sekmesinde katmanın türünün satırı önce gelir (nokta katmanında Nokta); öbürleri
  ardından.
- **Önizleme:** nokta katmanında “Haritada böyle görünür” şeridi katmanın renginde sembolü ve metni gerçek yerinde
  çizer. Çevresinde'de metin sağ üstte, sembolün kenarından açıkta; Üstünde'de sembolün üstündedir.

### 5. Katman stili: katmanın yolu (T26)

- **Yol satırı:** başlığın altında “Katman: Senaryo 1 / qa_yollar · Kaynak: Mahalle-Verileri.gpkg › qa_yollar”.
  - Veri katmanında dosya ya da sunucu ve tablo.
  - Kopyala simgesi yolu panoya kopyalar.
- **Bildirim:** Tamam'ın bildirimi katmanı yoluyla adlandırır (“… katmanına; Ctrl+Z geri alır.”). Geri alma adımı
  “Katman stili”dir; o katmana uygulanır.

### 6. Uygula ve Tamam (T27)

- **Tamam:** Etiketler penceresinin son düğmesi “Tamam”dır (önce “Kaydet”). Uygular ve kapatır; proje dosyasına
  Projeyi kaydet yazar.
- **Uygula:** tek bildirim verir: ““X” katmanının etiketlemesi uygulandı.”; değişiklik yoksa “zaten böyle”.
- **Uygula'dan sonra Tamam:** değişiklik yoksa sessiz kapanır; ikinci bildirim yoktur.

### 7. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Etiketler penceresi (kurallı sınıflar, nokta yeri, önizleme, Tamam), etiket araçlarının sözü | yok: etiket CBS'nindir (ADR 9316) | yok: aynı gerekçe | yok: aynı gerekçe | var |
| Değer listesiyle sınıf adı, Değer yok sınıfı | yok: Mimari'de kategorili katman stili ve değer listeli alan yoktur | var: Katman stili CAD'de de vardır | var: aynı | var |
| Katman stilinde katmanın yolu ve kaynağı | var: Katman stili ortaktır | var | var | var |

### 8. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | var | Etiketleme `cad.layers.labels`, stil `cad.layers.edit` (Stil) ile yazılır; kanal kategorileri kendisi verir. Değer listesinden ad ve Değer yok, pencerenin sınıf önerisidir; kanal aynı sınıfı aynı komutla yazar. |
| MCP | var | Aynı gerekçe. |
| Yapay zekâ ajanı | var | Aynı gerekçe. |
| Eklenti | var | Eklenti aynı komutlarla yazar. Pencereler arayüzün işidir. |

## Açık

- **İki eylemli bildirim:** kitin bildirimi tek eylem taşır; “Etiket kuralını aç” ayrı düğme değildir, kural aynı
  penceredir.
- **Önizlemenin katlanması:** önizleme ve tanı kartı küçük pencerede katlanmıyor; sınıf listesi esneyerek yer açıyor.
- **Gerçek pencere:** bu dilimin akışları fare ve klavyeyle denendi (Doğrulama); `scripts/usage/real/`'ın Linux koşusu yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Masaüstü:**
  - `style::layer_style::source_values_tests::listed_codes_are_named_by_their_labels_and_rows_without_a_value_have_a_category`:
    - `bakim`'in `mahalle`si (M1 Çınar, M2 İstasyon listeli) ile sınıflar (M1 Çınar), (M2 İstasyon), (M9 M9),
      (“” Değer yok);
    - Nesne sütunu 2, 2, 1, 1; Diğer 0;
    - yol kopyalanıyor.
  - `labelling::data_tests::a_point_layers_new_label_goes_around_and_apply_then_tamam_says_it_once`: nokta katmanının
    yeni etiketi Çevresinde; Uygula bir bildirim; Tamam ikinci bildirim vermeden kapatıyor.
  - `labelling::data_tests::a_label_tool_says_at_once_when_only_data_labels_are_on_the_map`:
    - Etiketi sabitle açılınca söz ve “Veri katmanının etiketi” bildirimi geliyor;
    - “Yerleşim ayarlarını aç” Etiketler'i katmanda Yerleşim'de açıyor.
  - `unit_tests::the_window_classes_and_the_drawing_read_areas_in_the_project_unit`: ifadeyle (`$alan`) sınıflandırmada
    Değer yok eklenmiyor.
  - `-- style:: labelling labels data_labels module_tests ribbon_tests history::text catalog settings input`: 322
    geçti. Kırmızı tek sınama bilinen bağımlılık tabanıdır (render birleşmesinin bağları).
- **Programlar:** `cargo test -p kentos-cad` 37 geçti. `label-tools` izi Tamam'a basıyor ve “etiketlemesi uygulandı”
  bekliyor.
- **Resim** (`.run/gui-evidence/kullanim-9357/`, açık ve koyu tema; büyütülerek bakıldı):
  - `01-kurali-siniflar`: varsayılan boyutta liste, araçlar, Ad ve Koşul;
  - `02-nokta-onizleme`: Yerleşim'de Nokta önde, Çevresinde; önizlemede sembol ve sağ üstte metin;
  - `03-stil-deger-yok`: Çınar, İstasyon, M9, Değer yok; yol satırı;
  - `04-etiket-araci`: “Veri katmanının etiketi” bildirimi ve Yerleşim ayarlarını aç.
- **Gerçek pencere** (27 Kasım 2026, macOS; sistem imleci ve klavye olaylarıyla, yalıtılmış kullanıcı dizininde, sınama
  GeoPackage'ı üzerinde; resimler `.run/real-9353/shots/`, Git'e eklenmez):
  - Kurallı: varsayılan boyutta araçlar, Ad ve Koşul görünür.
  - Tek etiket `bakim`'de: Yerleşim'de Nokta önde, Çevresinde; önizlemede sembol ve sağ üstte metin; Üstünde'de metin
    sembolün üstünde; Uygula ardından Tamam tek “uygulandı” sözü.
  - Etiketi sabitle açılınca söz ve “Veri katmanının etiketi” bildirimi; Yerleşim ayarlarını aç Etiketler'i Yerleşim'de
    açtı.
  - Kategorili `mahalle`: önce M1, M2, M9, Değer yok; Alanlar'da değer listesi (M1 Çınar, M2 İstasyon) tanımlanınca
    Çınar, İstasyon, M9, Değer yok; altı nokta çizildi; Katman stili başlığında yol ve kaynak; bildirim katmanı
    adlandırdı.
  - Kusur bulundu ve düzeltildi: bildirimin 8 saniyesi uzun sözü okuyup eyleme basmaya yetmedi (ilk denemede eylem süre
    dolunca gitti). Bu bildirim artık 20 saniye durur (üzerine gelince zaten bekler).
