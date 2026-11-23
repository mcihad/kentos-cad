# ADR 9177: Yolun planlı → kontrol geçişinde sayaç, harita, kâğıt, tablo ve lejant tek kuralı izler

- **Durum:** kabul edildi (2026-11-23); adım VY-76. Plan: [plan/veri.md](../../plan/veri.md) VY-76.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”; 23 Kasım: “küçük kalanlarla”.
  Değerlendirmenin 10. maddesinin kalanı
  ([ADR 9143](9143-renderers-by-geometry-and-unclassed-values.md) §4): “Yolun planlı → kontrol geçişinde harita,
  sınıf sayacı, tablo ve çıktı aynı politikayı izlemeli.”
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam ve araştırma

- **Raporun denemesi.** Yol çizgi katmanı “durum”a göre Kategorili çizildi; sınıflandırıldığında bütün yollar
  planlıydı. Sonra GeoPackage'ta bir satır kontrol yapıldı. Yeniden açılınca lejantta yalnız planlı kaldı; haritada
  yolun tamponu görünmeye devam etti. Tamponun görünmesi altındaki çizginin görünürlüğünü kanıtlamaz.
- **KentOS'un kuralı** (ADR 9143): Diğer değerler kapalıyken hiçbir kategoriye girmeyen nesne çizilmez ve sayısıyla
  söylenir. Harita ve kâğıt aynı stil motorundan geçer (`kentos_plot::layers`). Veri katmanının satırları kaynaktan
  okunur (ADR 9101).
- **Araştırma.** QGIS 3.44 kullanıcı kılavuzunun “Categorized Renderer” bölümüne bakıldı
  ([vektör özellikleri](https://docs.qgis.org/3.44/en/docs/user_manual/working_with_vector/vector_properties.html),
  okundu 23 Kasım 2026). Masaüstü CBS'lerde Kategorili çizicinin “bütün öbür değerler” sınıfı, sınıflardan hiçbirine
  girmeyen nesneler içindir; örneğin sınıflama alanına yeni değer giren nesneler.
  - KentOS bu sınıfı Diğer değerler olarak tutar ve kapalı başlatır (ADR 9143).
  - Bu karar, kuralın her yüzde aynı uygulandığını sınar.
- **Sınarken iki kusur bulundu; aynı nedenden:**
  1. **Lejant veri katmanını hiç göstermiyordu.** `legend_of` bir katmanın geometri sınıflarını çizimdeki
     nesnelerinden çıkarıyordu. Veri katmanının satırları çizimde olmadığından katman atlanıyordu. Bu, Lejant
     penceresinde de paftanın kâğıttaki lejantında da oluyordu.
  2. **Katman stili veri katmanında alan, çizgi ve nokta sembollerinin üçünü birden sunuyordu.** Yol çizgi
     katmanının kategorisi de üç sembolle yapılıyordu.

## §2 Karar

1. **Veri katmanının geometri sınıfı tablosundan okunur**
   (`kentos_native_style::classify::data_layer_present`).
   - `POINT`/`MULTIPOINT` nokta, çizgi ve eğri türleri çizgi, alan ve yüzey türleri alan olarak okunur.
   - Karışık tür (`GEOMETRY`, koleksiyon) katmanın türüne göre okunur, o da yoksa üçü birden alınır.
   - Geometrisiz tablo hiçbir şey çizmez.
   - Lejantın kaynakları (`LegendSources::declared`) bunu kullanır: Lejant penceresi ve paftanın lejantı. Katman
     stili penceresi de kullanır.
   - Çizimdeki katmanlar eskisi gibi nesnelerine bakar.
2. **Tek kural.** Yollardan biri dosyada kontrol olunca, Diğer değerler kapalıyken her yüz aynı şeyi söyler:
   - Katman stili sayacı planlı 1 ve “1 nesne hiçbir kategoriye girmiyor ve çizilmez” der;
   - ekran da kâğıt da yalnız planlı yolu çizer;
   - tablo yolun “kontrol” değerini gösterir;
   - lejantta kontrol için satır yoktur.

   Diğer değerleri aç'tan sonra ekran ve kâğıt iki yolu kendi sembolleriyle çizer; lejant “Diğer değerler” satırını
   ekler. Sayaç aynıdır, uyarı kalkar.

## §3 Kanıt

- **Masaüstü** (`gis/style_policy_tests.rs`). Fikstürdeki GeoPackage'ın `yollar` tablosunda Y1 ve Y2 var; ikisi de
  planlı, aralarında 50 m.
  - Sınıflama yalnız planlı'yı çıkarıyor. Pencerenin sembol sınıfı yalnız çizgi; kategori yalnız çizgi sembolü
    taşıyor.
  - İki yol çiziliyor. Lejantta “planlı” var, çizim satırların hiçbirini tutmadığı hâlde.
  - Başka bir program Y2'yi kontrol yaptıktan sonra:
    - sayaç (planlı 1, kategorisiz 1) diyor;
    - ekran da kâğıt da tek yol çiziyor (çizilenin kuzey açıklığı 0, tek sembol);
    - tablo Y2'ye kontrol, Y1'e planlı diyor;
    - lejant yalnız planlı.
  - Diğer değerleri aç'tan sonra ekran ve kâğıt iki yolu iki sembolle çiziyor (açıklık 50); lejant planlı ve Diğer
    değerler.
- **Stil çekirdeği** (`kentos-native-style`, `tests/all/legend.rs`):
  - nesnesiz veri katmanının lejantı tablosunun geometrisinden geliyor;
  - tür adları doğru sınıfa düşüyor (`MULTILINESTRING`, `POINT Z`, `MULTIPOLYGON`, geometrisiz);
  - çizimin nesnesiz katmanı eskisi gibi satır vermiyor.
- Stil (resimleriyle), lejant, veri katmanı ve ortak masaüstü sınamaları geçiyor (297); paftanın lejant sınaması
  (`kentos-plot`) geçiyor.
- **Resimler** `.run/gui-evidence/kullanim-9177/` altında:
  - `01-sinif-sayaci`: Katman stili, planlı 1, Diğer değerler çizilmez 1 ve uyarı;
  - `02-lejant`: Lejant, yollar başlığında planlı satırı.

## §4 Kalan

- Gerçek pencerede deneme; PostGIS katmanında aynı deneme (Docker yanıt vermedi).
- Web (TODOS.md §0.2).
