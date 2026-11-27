# ADR 9342: Dosya eylemleri önkoşullarını söyler; Veri ekle kaynak türlerine göre başlıklanır

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 27 Kasım 2026 (`gis-eksik-27-11-2026.md`, madde
  3 ve 9, P2). Sahibin sözü: “Gis eksik 27 11 dosyasındaki maddeleri tamamla”.
- **İlgili:**
  - [ADR 0193](0193-drawing-exchange.md) (Seçilenleri kaydet) ve
    [ADR 9339](9339-cbs-selection-copy-with-its-geopackage.md) (CBS'de kopyanın türü);
  - [ADR 0506](0506-relative-sources-and-package.md) ve
    [ADR 9338](9338-export-and-package-data-layer-rows.md) (Paketle);
  - [ADR 0622](0622-interface-arrangement.md) §3 (şerit ve menüler);
  - [ADR 0556](0556-geopackage-tiles.md) (GeoPackage karoları).

## Bağlam

- **Madde 3:** kaydedilmemiş yerel CBS projesinde Dosya şeridinin eylemleri denendi.
  - Seçilenleri kaydet seçim yokken açık göründü; basınca yalnız komut geçmişine uyarı yazdı.
  - Aç ile Yeni sekmede aç'ın sekmeye ne yaptığı basmadan önce bilinmiyordu.
  - Açılış raporu hangi dosyanın hangi açılışına ait olduğunu söylemiyordu.
  - Yerel projede Paylaş, Geçmiş ve Çakışmaları çöz neden kapalı olduklarını ayırmadan kapalıydı.
  - Paketle'nin penceresi neyin kopyalanacağını listelemiyordu.
- **Madde 9:** Veri ekle 18 satırı başlıksız gösterdi.
  - Şeridin tanımında satırlar bloklara ayrılmıştı (Vektör, Raster ve bulut …). Onay listesi olmayan menü onları tek
    liste yapıyordu.
  - GeoPackage karoları Katman ekle'nin taşmasındaydı, vektörleri Veri ekle'de.
  - Harita servisi ile Servisten veri al'ın ayrımı pencerede söylenmiyordu.

## Araştırma

- **QGIS, Layer › Add Layer:** sağlayıcı türüne göre ayrı alt menüler vardır: vektör, raster, mesh, nokta bulutu,
  ayrımlı metin, GeoPackage, veritabanları, WMS/WMTS, WFS/OGC API Features. Data Source Manager aynı gruplamayı sol
  sütununda tutar ([QGIS GUI › Menu Bar › Layer](https://docs.qgis.org/latest/en/docs/user_manual/introduction/qgis_gui.html#layer)).
- **QGIS, GeoPackage:** bir GeoPackage'ın vektör tabloları da raster karoları da aynı tarayıcı girişinde listelenir.
- **KentOS kararı:**
  - Menüler bloklarının başlıklarıyla okunur.
  - Kapalı bir eylemin nedeni ipucunda söylenir (ADR 9337'nin kuralı). Neden türleri ayrı sözlerle verilir.

## Karar

### 1. Dosya eylemleri (madde 3)

- **Seçilenleri kaydet:** seçim yokken kapalıdır. İpucu: “Önce kaydedilecek nesneleri seçin: çizimde bir ya da daha
  çok nesne. Seçilenler yeni bir projeye yazılır.”
- **Aç / Yeni sekmede aç:** açıklamaları sekmeyi söyler:
  - Aç bu sekmede açar, açık çizim kapanır (kaydedilmemiş değişiklik önce sorulur);
  - Yeni sekmede aç açık çizimi kendi sekmesinde geri alması, seçimi ve görünümüyle bırakır.
- **Açılış raporu:** raporun ilk satırı dosyayı ve açıldığı anı söyler: ““resimli.dxf” ayrı bir çizim olarak açıldı
  (27.11.2026 14:05); rapor bu açılışındır, kaynak dosya değişmedi.” Kapalıyken ipucu nedenini söyler (DWG, DXF ya da
  NCZ'den açılmadı).
- **Bulut eylemleri:** açık projenin bulut eylemleri (Paylaş, Geçmiş, Çakışmaları çöz, Yeniden adlandır, Sil, En son
  revizyon) kapalıyken nedenleri ayrıdır:
  - “Yalnız bulut projesinde: açık proje yerel bir dosya. …”
  - “Oturum gerekli: Bulut › Oturum aç ile girin.”
  - “Bağlantı yok: …”
  - “Bekleyen çakışma yok: bulutla eşitleme temiz.”
  - “Proje arşivlenmiş: …”
  - rol izni yok.
- **Paketle'nin listesi:** pencere açılınca ve bir yol ya da bağ değişince `file.package`'ın planı veritabanı
  okunmadan ve devir dosyası yapılmadan arka planda alınır. “Pakete girecekler” alanı iki şeyi listeler:
  - kopyalanacak her dosyayı paketteki yeriyle ve kaynağıyla (“veri/qa.gpkg ← kaynak · /iş/qa.gpkg”);
  - olduğu gibi kalan her bağı nedeniyle (“bağlı kalır”, “yerinde yok”).

  On satırdan fazlası sayılır.

### 2. Veri ekle ve kaynak türleri (madde 9)

- **Başlıklar:** onay listesi olmayan şerit menüsünde birden çok başlıklı blok varsa her blok başlığıyla okunur.
- **Veri ekle'nin blokları:**
  - Vektör;
  - Raster ve karo (Raster ekle, **GeoPackage karoları**);
  - Nokta bulutu ve mesh;
  - Harita ve CAD (DWG, DXF, NCZ, PDF);
  - Saha verileri (koordinat listesi, GNSS, konumlu fotoğraflar).
- **Aynı GeoPackage'da vektör ve karo:** GeoPackage penceresi dosyanın karo tablolarını da söyler. **Karoları aç…**
  aynı dosyayı GeoPackage karolarını oku penceresinde açar.
- **Servis pencereleri:**
  - Harita servisi'nin başı: “Görüntü katmanı: servis haritayı resim olarak çizer; nesneleri seçilmez, öznitelikleri
    tabloda okunmaz. …”
  - Servisten veri al'ın başı: “Özellik tablosu: servisin nesneleri öznitelikleriyle alınır; seçilir, tabloda okunur,
    işlemlere girer. …”
  - Bağlantı ve isteğin durumu pencerenin özetinde kalır (önceden de öyleydi).

### 3. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Seçilenleri kaydet'in kapanması, Aç / Yeni sekmede aç açıklamaları, Açılış raporunun anı | var | var | var | var |
| Bulut eylemlerinin nedenleri | var | var | var | var |
| Paketle'nin listesi | var | var | var | var |
| Menü blok başlıkları | var | var | var | var |
| Veri ekle'nin blokları (şerit menüsü) | yok: Mimari'nin şeridinde Veri ekle yoktur | yok: Harita'nın İçe aktar menüsü kendi bloklarıyla başlıklanır | yok: aynı gerekçe | var |
| GeoPackage penceresinde karo tabloları, servis pencerelerinin başı | var | var | var | var |

### 4. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | yok | Hepsi arayüzün sunumudur. Paketin planı `file.package` (`op: plan`) ile, GeoPackage karoları `cad.rasters.gpkgread` ile alınır; sonuç aynıdır. |
| MCP | yok | Aynı gerekçe. |
| Yapay zekâ ajanı | yok | Aynı gerekçe. |
| Eklenti | yok | Aynı gerekçe. |

## Açık

- **Klavye odağında ipucu:** kapalı düğmenin nedeni fare üstündeyken görünür; klavye odağında gösterilmez (ADR 9337'nin
  açığı).
- **Canlı servisler:** WMS, WFS ve PostGIS pencerelerinde canlı okuma bu dilimde yapılmadı.
- **Gerçek pencere:** fare ve klavyeyle kabul (`scripts/usage/real/`) bu dilimde yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Masaüstü:**
  - `file_menu_tests`:
    - seçim yokken Seçilenleri kaydet kapalı ve nedeni, seçimle açık;
    - yerel projede Paylaş, Geçmiş, Çakışmaları çöz “Yalnız bulut projesinde” diyor;
    - Paketle'nin listesi `veri/qa.gpkg`'yi kaynağıyla gösteriyor; Bağlı bırak'ta dosya listeden çıkıyor, bağ “bağlı
      kalır”;
    - blok başlıkları birden çok başlıklı blokta konuyor, tek blokta konmuyor;
    - vektör ve karo tutan GeoPackage'da pencere `yollar`'ı ve bir karo tablosunu gösteriyor, Karoları aç karo
      penceresini açıyor.
  - `-- package_tests gpkg_tests drawing_exchange selection_save source_export open_report cloud:: catalog ribbon
    services module_tests`: hepsi geçti, iki sınama dışında. Kırmızılar bu dilimden değil:
    - bilinen bağımlılık tabanı;
    - `open_report_tests::a_lossy_file_brings_its_report_and_it_comes_back`: okuyucu adı olmayan IMAGE'ı
      “değiştirilerek alındı” sayıyor, sınama “alınmadı” bekliyor. Raporun öğe kararı bu dilimde değişmedi.
  - Resim sınamalarından `ribbon_bar::screens::screens` (“Kapalı alan” yazısı bulunamadı) ve
    `templates_panel::tests::ribbon_screens` kırmızı; ikisi de Veri ekle'ye dokunmuyor, bu dilimden değil.
- **Şerit:** `python3 scripts/ui/ribbon.py --check` güncel.
- **Resim** (`.run/gui-evidence/kullanim-9342/`, açık ve koyu tema; büyütülerek bakıldı):
  - `01-veri-ekle`: Vektör, Raster ve karo (GeoPackage karoları), Nokta bulutu ve mesh, Harita ve CAD, Saha verileri;
  - `02-paketle-liste`: “Pakete girecekler”de `veri/qa.gpkg ← kaynak · …/qa.gpkg`.
