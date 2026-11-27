# ADR 9345: İfadeyle seç bütün kapsamlarda veri katmanlarını okur; seçimin sonucu katman katman saklanır

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 27 Kasım 2026, ikinci tur
  (`gis-eksik-27-11-2026.md`, T3 P2 ve T4 P3). Sahibin sözü: “yeni maddeler ekledim onları düzelt komple”.
- **İlgili:**
  - [ADR 9340](9340-data-layers-in-select-by-location-and-the-inspector.md) (Konuma göre seç veri katmanlarında);
  - [ADR 9133](9133-select-by-expression-on-data-layers.md) (İfadeyle seç tek veri katmanında, koşul kaynakta);
  - [ADR 9341](9341-info-card-actions-last-expression-and-data-labels.md) (Son ifade);
  - [ADR 9101](9101-data-layers.md) (veri katmanı).

## Bağlam

- **T3:** yedi kaynak satırı olan projede Harita › İfadeyle seç › Tümü “Görünen katmanlarda uygun nesne yok” dedi;
  Çalıştır kapalıydı.
  - Aynı pencere Katman kipinde tek veri katmanının satırını okuyordu (ADR 9133).
  - Konuma göre seç Tümü'de yedi kaydı buluyordu (ADR 9340); İfadeyle seç bu yolu kullanmıyordu.
  - Katmanların alanları farklıyken eksik alanın nasıl ele alındığı ve hangi katmanın kapsama girdiği söylenmiyordu.
- **T4:** Konuma göre seç'in sonucu beş eşleşmeyi saydı, tabloda yalnız bir katmanın iki satırı seçildi.
  - Öbür katmanlardaki eşleşmeleri görmek için Seçilecek nesneler'de o katmanı seçip işlemi yeniden çalıştırmak
    gerekiyordu.
  - Aynı kısa adlı iki katman sonuçta ayırt edilmiyordu.

## Araştırma

- **QGIS, İşleme › Vektör seçimi:** “Select by expression” ve “Select by location” tek bir girdi katmanında seçer;
  birden çok katman için katman başına ayrı çalıştırılır
  ([Vector selection](https://docs.qgis.org/latest/en/docs/user_manual/processing_algs/qgis/vectorselection.html)).
- **QGIS, NULL karşılaştırması:** NULL değerle karşılaştırma doğru ya da yanlış değil NULL'dur; süzgeçte sağlanmamış
  sayılır. Yalnız `IS NULL` / `IS NOT NULL` doğru döner (geliştiricilerin kararı,
  [Bug report #5102](https://issues.qgis.org/issues/5102)).
- **ArcGIS Pro, İçindekiler › Seçime göre listele:** seçilebilir her katman yanında seçili nesne sayısıyla listelenir;
  katmanın seçimi oradan temizlenir ya da açılır
  ([Contents pane](https://pro.arcgis.com/en/pro-app/3.3/help/mapping/map-authoring/contents-pane.htm)).
- **KentOS kararı:**
  - Seçim işlemleri birden çok katmanda bir kez çalışır.
  - Sonuç katman başına anahtarlarla saklanır ve katman katman sayılarıyla sunulur.
  - Tablo bir katmanı gösterir; öbürüne geçmek işlemi yeniden çalıştırmaz.
  - Alanı olmayan katmanda alanın değeri boş sayılır; bu kural girdinin satırında söylenir.

## Karar

### 1. İfadeyle seç'in kapsamları (T3)

- **Tümü, Görünen, Seçili:** veri katmanlarının satırları, Konuma göre seç'in yolundan (ADR 9340) okunur.
  - Satırlar çizimin bir kopyasına nesne olarak konur.
  - Pencere girdiyi bu kopyada sayar; işlem `cad.processing.run` ile her kanalda olduğu gibi orada çalışır.
  - Seçilenler katman ve anahtarla geri gelir: çizimin nesneleri çizimin seçimine, satırlar tablonun seçimine.
  - Seçim biçimi (yeni, ekle, çıkar, içinde ara) çizimin seçimi ve tablonun seçili satırlarıyla birlikte uygulanır.
- **Katman** (tek veri katmanı): koşul kaynakta sorulur (ADR 9133, değişmedi).
- **Kapsamın sözü:** girdinin satırı, kapsamdaki veri katmanlarını satır sayılarıyla söyler: “Kapsamdaki veri
  katmanları: kontrol (1), koseler (3), ucgen (1).”
  - Aynı kısa adı başka bir katmanla paylaşan katman grubunun yoluyla anılır (“Senaryo 1 / koseler”).
- **Eksik alan kuralı:** koşulun okuduğu bir alan bir katmanda yoksa satır şunu da söyler: “Alanı olmayan katmanda
  değeri boş sayılır, eşitlik sağlanmaz: ucgen (durum); …”.
  - Alanlar katmanın tanımındakiler ve satırlarının taşıdıklarıdır.
- **Okunurken:** pencere Tümü'de açılınca veri katmanlarının satırları okunmaya başlar. Okunurken Çalıştır bekler;
  okunamayan kaynak adıyla söylenir.

### 2. Sonuç katman katman (T4)

- **Saklama:** Konuma göre seç ve İfadeyle seç, birden çok veri katmanında eşleşme bulunca sonucu katman başına
  anahtarlarla pencerede saklar.
- **Sonuç kartı:** formun başında “Sonuç katmanlara göre” kartı durur. Her katman tablo simgesi, adı ve eşleşme
  sayısıyla bir düğmedir; tablonun gösterdiği katman işaretlidir.
  - Çalıştır'dan sonra form başa kaydırılır, kart görünür.
  - Bir katman seçilince o katmanın eşleşmeleri tabloda seçili gösterilir; işlem yeniden çalışmaz.
- **Durum satırı:** “4 nesne ilişkiyi sağladı: “kontrol” 1, “koseler” 3. Tabloda “kontrol”: 1 satır seçili
  (anahtar: 1). Öbür katmanınkini formun başından tabloda açın; işlem yeniden çalışmaz.”
  - Katman değişince satır da değişir. Tek katmanda eski söz kalır.
- **Sıra:** katmanlar Katmanlar panelindeki sırayla gelir. İlk gösterilen, girdi bir katmansa odur; değilse en çok
  eşleşeni olan katmandır.
- **Silinme:** bir değer değişince ya da Varsayılanlar'da kart durum satırıyla birlikte kalkar.
- **Denetçi:** çok satır seçiliyken Denetçi'nin “İlki gösteriliyor” durumu değişmedi.

### 3. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| İfadeyle seç'in Tümü, Görünen, Seçili kapsamında veri katmanları | yok: Mimari'de veri katmanı yoktur | var: CBS tabloları altlık olabilir | var: aynı gerekçe | var |
| Kapsamın ve eksik alanın sözü | yok: aynı gerekçe | var | var | var |
| Sonuç katman katman (iki işlem) | yok: aynı gerekçe | var | var | var |

### 4. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | yok | Seçim arayüzün durumudur. Betik `cad.processing.run`'ı satırları nesne olan bir çizimde çağırır; seçilen kimlikler katman başına ayrılmış döner, betik kendisi ayırır. |
| MCP | yok | Aynı gerekçe. |
| Yapay zekâ ajanı | yok | Aynı gerekçe. |
| Eklenti | yok | Aynı gerekçe. |

## Açık

- **Büyük tablolar:** kopya satırları bütün olarak okur (ADR 9340'ın açığı). Milyonlarca satırda koşulu kaynakta
  sormak (ADR 9133'ün yolu) kapsamın her katmanı için ayrı ayrı yapılmalıdır.
- **Harita:** tablo tek katmanın seçimini tutar. Katman değişince öbür katmanın eşleşmeleri haritada çizilmez; kart
  onları saklar.
- **Gerçek pencere:** fare ve klavyeyle kabul (`scripts/usage/real/`) bu dilimde yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Masaüstü:**
  - `data_location_tests` (5 geçti):
    - Uzaklıkta 6 m: sonuç `kontrol` 1 ve `koseler` 3 olarak saklanıyor. `kontrol` seçilince tablo onu `{1}` ile
      gösteriyor, çizim değişmiyor (işlem yeniden çalışmıyor). `koseler` `{1, 2, 3}` ile geri geliyor. Bir değer
      değişince kart kalkıyor.
    - İfadeyle seç Tümü: girdi 5 nesne sayıyor, Çalıştır açık, satır kapsamdaki üç katmanı söylüyor.
    - `kontrol`, `koseler` adıyla Senaryo 1'e taşınınca `ad = 'A' veya ad = 'K'` iki katmanın da 1 anahtarını
      buluyor. Katman kimliğiyle ayrılıyorlar; sonuç “Senaryo 1 / koseler” diyor.
    - `durum = 'x' veya ad = 'A'`: satır üç katmanda `durum`'un olmadığını kuralıyla söylüyor; sonuç 1 satır.
  - `data_select_tests`: pencere Tümü'de açılınca satırlar okunduğu için katman seçilir seçilmez sayılıyor.
  - `-- processing:: data_select select module_tests ribbon_tests history::text catalog settings`: 401 geçti.
    Kırmızı tek sınama bilinen bağımlılık tabanıdır.
  - `cargo test -p kentos-desktop-cbs`: 63 geçti.
- **Resim** (`.run/gui-evidence/kullanim-9345/`, açık ve koyu tema; büyütülerek bakıldı):
  - `01-katmanlara-gore-sonuc`: formun başında kart, `kontrol` 1 işaretli, `koseler` 3; durum satırı;
  - `02-ifadeyle-sec-tumu`: Tümü'de 5 nesne, kapsamdaki katmanlar ve `durum` alanının kuralı.
