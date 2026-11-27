# ADR 9341: Bilgi kartı kaydını seçer, tabloda ve formda açar; son ifade yazılmaz, sunulur; etiket araçları veri katmanının etiketini söyler

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 27 Kasım 2026 (`gis-eksik-27-11-2026.md`, madde
  6, 7 ve 10, P2). Sahibin sözü: “Gis eksik 27 11 dosyasındaki maddeleri tamamla”.
- **İlgili:**
  - [ADR 9101](9101-data-layers.md) §6 (Bilgi veri katmanında);
  - [ADR 9145](9145-label-candidates-and-overlapping-hits.md) ve [ADR 9167](9167-objects-and-data-rows-counted-together.md) (üst üste nesneler,
    Sonraki);
  - [ADR 9133](9133-select-by-expression-on-data-layers.md) (İfadeyle seç);
  - [ADR 0212](0212-label-engine.md) §4 (etiket araçları);
  - [ADR 9186](9186-label-example-and-data-layer-labelling.md) (veri katmanının etiketleri).

## Bağlam

- **Madde 6:** yalnız kaynaklı projede Bilgi, CB yolunun kartını katmanı, tablosu ve `fid` ile gösterdi. Kartta
  Tabloda aç, Formu göster ve Seç yoktu. C noktasında birden çok satır varken kart yalnız birini gösterdi.
  - Kod boş yere tıklamada katmanlardan ilk bulunan satırı alıyordu. Bir tabloda da yalnız en yakın satırı
    alıyordu.
- **Madde 7:**
  - İfadeyle seç'in koşulu önceki çalıştırmanın ifadesiyle açıldı (`bedel ≥ 100`, veri setinde olmayan alan).
  - Denetçi'nin Katman satırı yalnız katmanın adını gösterdi (`qa_yollar`); tablo grup yolunu (`Senaryo 1/qa_yollar`)
    gösteriyordu.
  - Tümünü seç 34 nesne seçtiğini söyledi; proje özeti 37 nesne diyordu. Aradaki fark söylenmiyordu.
- **Madde 10:** kaynağa bağlı katmanın `{ad}` etiketi haritada göründü. Etiketi taşı, Etiketi sabitle ve Etiketi
  gizle yalnız “bir etikete tıklayın” dedi.
  - Veri katmanının etiketleri kendi satır belgesinde yerleşir; araçlar çizimin etiketlerine bakar.
  - Etiket iğnesi nesnenin kendisindedir. Kaynak satırının iğnesini proje dosyasında tutan bir yer yoktur.

## Araştırma

- **QGIS, Identify Features:** sonuç panelinde bir noktadaki bütün nesneler katman katman listelenir. Bağlam menüsünde
  nesneyi seçme, öznitelik tablosunda gösterme ve formu açma vardır. Araç seçimi kendiliğinden değiştirmez
  ([Identify features](https://docs.qgis.org/latest/en/docs/user_manual/introduction/general_tools.html#identify-features)).
- **QGIS, ifade seçimi:** ifade penceresi son ifadeleri “Recent” altında sunar; alana kendiliğinden yazmaz
  ([Expressions](https://docs.qgis.org/latest/en/docs/user_manual/expressions/expression.html#the-expression-string-builder)).
- **QGIS, etiket araçları:** etiket taşıma, sabitleme ve gizleme verinin “auxiliary storage”ında tutulur; katmanın
  verisine yazılmaz ([Label toolbar](https://docs.qgis.org/latest/en/docs/user_manual/style_library/label_settings.html#the-label-toolbar)).
- **KentOS kararı:**
  - Kartın eylemleri açık basışla çalışır. Bilgi seçimi kendiliğinden değiştirmez.
  - Son ifade alana yazılmaz, sunulur.
  - Veri satırlarının etiket iğneleri için proje dosyasında bir saklama yeri bu dilimde kurulmadı (sözleşme,
    komut ve ortak durumlar ister). Araç tıklanan veri katmanının etiketini tanır, neden alınmadığını ve ne
    kullanılacağını söyler.

## Karar

### 1. Bilgi kartı (madde 6)

- **Adaylar:** boş yere tıklamada gösterilen veri katmanlarının o noktadaki bütün satırları adaydır.
  - Sıra: katmanlar üstten, bir tabloda çizgiler tıklamaya uzaklıklarıyla (mikrometreye yuvarlanır), eşitler
    anahtarlarıyla, sonra alanlar küçükten.
  - Her tıklamada aynı sıra gelir. Kart ilk adaydır; Sonraki öbürlerine geçer, başa döner.
  - Yalnız satır varken yer sözü “Bu noktada 2 veri satırı · 1/2” olur.
- **Eylemler:** kartın altında **Seç**, **Tabloda aç** ve **Formu göster** düğmeleri vardır; nesne kartında da
  aynıdır. Bilgi kendisi hiçbir seçimi değiştirmez.
  - **Seç:** satır tablosunun seçimine alınır, tablo katmana gelir, harita satırı çizer, Öznitelikler onu gösterir.
    Nesne çizimin seçimine alınır.
  - **Tabloda aç:** Seç'e ek olarak tablo yalnız seçilenleri gösterir. Satır bu yüzden en üstte vurgulu durur.
    Bütün satırlar için Göster › Tümü seçilir; bunu ileti söyler. Nesnenin tablosu seçili nesneye kendisi kayar.
  - **Formu göster:** katmanın formu okuma için açılır; Düzenle bir basış ötededir. Formu olmayan katmanda düğme
    kapalıdır. İpucu nedeni ve yolu söyler: “Katmanın formu yok · Veri › Formlar'da bu katman için bir form yapın.”

### 2. İfadeyle seç ve seçimin sözleri (madde 7)

- **Son ifade:** pencere açılırken ifade alanlarına son çalıştırmanın ifadesi yazılmaz; aracın kendi ifadesi gelir.
  - Son ifade alanın altında “Son ifade: bedel >= 100” düğmesidir; basınca alana yazılır.
  - Yeniden aç, model adımı ve toplu çalıştırma gibi bilerek verilen değerler olduğu gibi kalır.
- **Kapsamın anlamı:** Tümü seçiliyken “Görünen katmanların bütün nesneleri; ekranın dışında kalanlar da.”, Görünen
  seçiliyken “Yalnız ekranda görünen alandaki nesneler; gizli katmanlarınki alınmaz.” yazar.
- **Katman yolu:** Denetçi'nin Katman satırı katmanı grup yoluyla söyler (“Kadastro / Parsel”); tablo ve menü de
  böyle söyler.
- **Tümünü seç:** projedeki nesneden azını seçtiyse, seçim süzgeci ya da Otomatik seçim nedenini söylemediyse, kaç
  tanesinin neden seçilmediğini söyler (ADR 9343'te düzeltildi): “34 nesne seçildi:
  görünen ve seçilebilir katmanların nesneleri. Projedeki 37 nesneden 3 tanesi seçilmedi: gizli, donmuş ya da
  seçilemez katmanda ya da seçim süzgecinin dışında.”

### 3. Etiket araçları veri katmanının etiketinde (madde 10)

- Etiketi taşı, Etiketi döndür, Etiketi sabitle ya da Etiketi gizle çizimde etiket bulamazsa tıklanan yerde etiketli
  bir veri katmanının etiketine ya da satırına bakılır. Bulunursa araçların “bir etikete tıklayın”ının ardından şu
  söylenir:
  “Tıklanan etiket “qa_secim” veri katmanının. Veri katmanının etiketleri kaynağın satırlarından yerleştirilir: tek tek
  taşınmaz, döndürülmez, sabitlenmez ve gizlenmez; yerleri kaynağa yazılmaz. Yerleri için katmanın Etiketler ›
  Yerleşim ayarlarını, bir satırın etiketini göstermemek için etiket kuralının koşulunu kullanın.”
- Kaynak tablosuna hiçbir şey yazılmaz.

### 4. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Bilgi kartının Seç, Tabloda aç, Formu göster'i | var | var | var | var |
| Bilgi'de veri satırı adaylarının sırası | yok: Mimari'de veri katmanı yoktur | var | var | var |
| Son ifade, kapsamın anlamı | var | var | var | var |
| Denetçi'de katman yolu, Tümünü seç'in sözü | var | var | var | var |
| Etiket araçlarının veri katmanı sözü | yok: etiket CBS'nindir (ADR 9316) | yok: aynı gerekçe | yok: aynı gerekçe | var |

### 5. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | yok | Hepsi arayüzün sunumudur. Satır `cad.layers.rows` (anahtarla) ve `cad.entities.select` ile seçilir; seçim komutları aynıdır. |
| MCP | yok | Aynı gerekçe. |
| Yapay zekâ ajanı | yok | Aynı gerekçe. |
| Eklenti | yok | Aynı gerekçe. |

## Açık

- **Veri satırlarının etiket iğneleri:** proje dosyasında satır anahtarıyla taşıma, sabitleme ve gizleme kaydı
  (QGIS'in “auxiliary storage”ı gibi) yoktur. Komut, sözleşme ve ortak durumlarla ayrı dilimde kurulmalıdır.
- **Veri tablosunda kaydırma:** Tabloda aç satırı “yalnız seçilenler”le öne getirir; bütün satırlar arasında o
  satırın sayfasına kaydırmak sıralı anahtar okuması ister, yapılmadı.
- **Aynı adlı iki katmanda seçim kopyası:** incelemenin söylediği doğrulama (iki Senaryo katmanında AB'nin dosyaya
  doğru grup kimliğiyle yazılması) bu dilimde ayrı sınanmadı.
- **Gerçek pencere:** fare ve klavyeyle kabul (`scripts/usage/real/`) bu dilimde yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Masaüstü:**
  - `data_info_card_tests` (`qa_yollar` AB, BC, CB; `qa_secim` AB):
    - C'de adaylar BC (2), CB (3); kart 2, Sonraki 3, sonra yeniden 2; ikinci tıklamada aynı sıra;
    - Bilgi seçim yapmıyor;
    - Seç satır 3'ü tablonun seçimine alıyor, tablo katmana geliyor, çizimin seçimi boş;
    - Tabloda aç Göster'i Seçililer yapıyor; formsuz katmanda Formu göster kapalı.
  - `recall_tests`:
    - İfadeyle seç `$alan > 500` ile açılıyor; çalıştırılan `bedel >= 100` Son ifade olarak sunuluyor, basınca
      yazılıyor;
    - Denetçi'nin Katman'ı “Kadastro / Parsel”;
    - Tümünü seç projedeki ve seçilen sayıyı ayırıyor.
  - `labelling::data_tests::a_label_tools_click_on_a_data_label_says_why_and_what_to_use`: Etiketi taşı, sabitle ve
    gizle veri katmanının parselinde katmanın adını ve yolu söylüyor.
  - `-- labelling info_card data_info processing:: properties selecting select_all data_layers features module_tests
    ribbon_tests history::text catalog settings source_status`: hepsi geçti. Kırmızı tek sınama bilinen bağımlılık
    tabanıdır (`shell/approved_layer.rs`, `shell/gpu_frame.rs`, `gis/data_selection.rs`; bu dilimden değil).
  - `source_status_tests`'in Denetçi sınaması ADR 9340'ın sözlerine çevrildi (Veri katmanları “2 katman”, Çizimdeki
    nesne, 0 olmayan Kaynaktaki satır). ADR 9340'ın dilimi bu sınamayı koşmamıştı.
- **Resim** (`.run/gui-evidence/kullanim-9341/`, açık ve koyu tema; büyütülerek bakıldı):
  - `01-bilgi-karti-eylemler`: C'de “qa_yollar · BC”, Seç, Tabloda aç, Formu göster (kapalı), “Bu noktada 2 veri
    satırı · 1/2”, Sonraki;
  - `02-ifadeyle-sec-son-ifade`: Tümü'nün anlamı, aracın kendi koşulu, Son ifade düğmesi.
