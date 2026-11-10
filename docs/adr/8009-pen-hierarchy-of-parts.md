# 3009 — Kalem hiyerarşisi: açıklığın ve duvarın parçaları ISO 128-23'ün kalın/ince oranıyla

Tarih: 2026-11-07. Durum: kabul (M10, ADV-14; plan S4).

## Sorun

- **Açıklığın bütün parçaları tek kalınlıktaydı.** Kapının kasası, kanadı ve açılma yayı; pencerenin çerçevesi, camı ve
  denizliği aynı kalemle çiziliyordu.
  - Parçalar ekrana ve dosyaya kendi kalınlığı olmadan gidiyordu (`line_weight: None`), yani açıklığın katman ya da
    nesne kalınlığını alıyordu.
  - 1:50 PDF'te kapı yayı kanatla aynı 0,25 mm'ydi.
- **Duvarın malzeme sınırları dış yüzlerle aynı kalınlıktaydı.** Kesilen duvarın dış çizgisi ile içindeki tuğla/yalıtım
  sınırı aynı kalemle çiziliyordu.
- **Çizim mimari pafta gibi okunmuyordu (S4).** Neyin kesildiği, neyin görüldüğü kalınlıktan anlaşılmıyordu.

## Araştırma (7 Kasım 2026)

- **ISO 128-23:1999, *Lines on construction drawings*** (resmî örnek sayfalar, `.run/gui-evidence/mimari-m10/referans/`).
  Standart bugün TS EN ISO 128-2:2022'nin içindedir. 2026 Dijital Hazırlama Yönetmeliği EK-4 4.22 çizgi tipleri için
  TS EN ISO 128-2'yi esas alır.
  - **Tablo 2 (çizgi grupları, mm):** ince / kalın / çok kalın; 0,25 grubu 0,13 / 0,25 / 0,5; 0,35 grubu 0,18 / 0,35 /
    0,7; 0,5 grubu 0,25 / 0,5 / 1. "The proportions between the line widths are 1:2:4." Sembollerin kalemi ince ile
    kalın arasındadır.
  - **Tablo 1:**
    - sürekli ince (01.1): malzeme sınırları, tarama, ölçü, merdiven ve rampa okları, görünen dış çizgiler, kapı,
      pencere ve merdivenin basitleştirilmiş gösterimi;
    - sürekli kalın (01.2.1): taramalı kesitin dış çizgisi;
    - sürekli çok kalın (01.3.1): taramasız kesitin dış çizgisi;
    - kesikli ince (02.1.3): gizli çizgiler;
    - uzun çizgi-noktalı ince (04.1.2): eksenler;
    - uzun çizgi-noktalı kalın (04.2.2): kesme düzleminin önünde kalan görünen parçalar.
- **Revit, *About Subcategories for Object Styles*** (2024): alt kategoriler "control the visibility and graphics of
  discrete geometry within a category". Resmî görselde kapının alt kategorileri Frame/Mullion, Glass, Opening, Panel,
  Plan Swing ve Elevation Swing; her birinin kendi çizgi ayarı vardır (`referans/revit-alt-kategori-yay.png`).
- **KentOS'ta var olan:**
  - ADR 0793: kalem kâğıtta mm'dir ve çizim ölçeğinde gösterilir.
  - ADR 0783: görünüş tarifinin kalem seti nesne türünün kalınlığını topluca verir.
  - M9 (ADR 8008): EK-4 katmanlarının kalınlıkları; duvar 0,5, açıklık 0,25.

## Karar

- **Parçanın rolü** (`kentos_geometry_core::geom::opening::Part`, `symbol_parts`): her açıklık parçası ne olduğunu
  söyler.
  - Kasa, kanat, açılma yayı (dönen kapının tamburu da), cam ve kayıt, denizlik, eşik, pervaz, ray, kullanım alanı;
    boşluğun ve nişin dış çizgisi; bacanın çaprazı.
  - `symbol()` aynı kalır, yeni `symbol_parts()` parçaları rolüyle verir.
- **Kalemin payı** (`PieceStroke::weight_scale`, `Part::pen`): parça kendi kalınlığını değil, nesnesinin kaleminin payını
  taşır.
  - Kasa ve kanat bütün kalemi alır.
  - Yay, cam, kayıt, denizlik, eşik, pervaz, ray, kullanım alanı ve boşluk çizgileri yarısını alır: ISO'nun ince çizgisi,
    kalının yarısı (Tablo 2'nin 1:2'si).
  - Duvarın dış yüzleri ve uçları (kesilen dış çizgi) bütün kalemi alır. Bileşenler arasındaki çizgiler, yani malzeme
    sınırları (ISO 01.1.1), yarısını alır. Bileşenin kendi kenar kalınlığı varsa o geçerlidir.
- **Pay neden mutlak değil:** kalem katmandan, nesnenin kendi kalınlığından ya da görünüş tarifinin kalem setinden
  (ADR 0783) gelir. Pay hangisi olursa olsun hiyerarşiyi korur. Kalem seti duvara 0,7 verirse sınırları 0,35 olur.
- **Çizim** (`kentos_native_style::program`): parçanın kalınlığı kendi kalınlığı, katmanınki ya da nesnenin kalınlığı
  olarak çözülür, sonra payla çarpılır. Ekran, pafta ve PDF aynı yoldan geçer.
- **Ekran:** kalınlık gösterimi zaten açık (ADR 0793). 1:100'de 0,25 mm yaklaşık 1 piksel, yarısı da en az 1 piksel
  kalır; fark 1:50'de ve kâğıtta görünür.
- **Kapsam dışı:**
  - çizim ölçeğine göre çizgi grubu: katman kalınlıkları bugün sabit (ADV-15, M12). ISO 128-23 §5 genişliği "çizimin
    türüne, boyutuna ve ölçeğine göre" seçtirir; Tablo 2 grupları verir, ölçekle eşleştirmez. (7 Kasım düzeltmesi,
    M11: ilk yazımda "1:100'de 0,35, 1:50'de 0,5" Tablo 2'ye mal edilmişti.)
  - kesme düzleminin önündeki parçaların uzun çizgi-noktalı kalın çizgisi (M13);
  - DXF'te parça başına kalınlık: DXF parçaları katmanın kalınlığını (ByLayer) taşımayı sürdürür.

## Ölçüm

- **1:50 PDF** (`crates/native/headless/tests/headless/pens.rs`):
  - Yeni Mimari projesinde 5 × 4 m dikdörtgen duvar (0,5 mm), bir kapı ve bir pencere (0,25 mm) çizildi.
  - Projede `cad.sheets.levels` ile 1:50 pafta açıldı, `file.export.pdf` ile yazıldı ve KentOS'un PDF okuyucusuyla geri
    okundu.
  - Çizgi kalınlıkları (`pdftocairo -svg` ile sayıldı):

    | Kalınlık | Yol sayısı | Ne |
    |---|---|---|
    | 0,50 mm | 9 | duvar yüzleri ve uçları |
    | 0,25 mm | 9 | kasa, kanat, pencere çerçevesi |
    | 0,125 mm | 2 | kapının açılma yayı ve pencerenin camı (önce 0,25) |
    | 0,18 / 0,35 / 0,7 mm | — | paftanın kendi çerçeve ve antet çizgileri |

  - 600 dpi resim: `.run/gui-evidence/mimari-m10/m10-pdf-kalemler.png`.
- **Testler:**
  - `openings::an_openings_pieces_say_what_they_are_and_their_pen`: başvuru fikstürlerinin (açıklıklar, özel kapılar, çok
    kanatlı sürmeler) her kapı ve penceresinde yay bir yaydır ve yarım kalemdedir; tam kalemde yalnız kasa ve kanat
    vardır.
  - `wall::the_faces_take_the_whole_pen_and_the_boundaries_between_materials_half`: dört katmanlı duvarda 2 yüz ve 2 uç
    tam kalem, 3 sınır yarım; kendi kenar kalınlığı olan bileşen onu korur.
  - `wall_detail_sheet_tests` güncellendi:
    - 1:100'de ayrıntılı duvarın 3 cm'lik dış sıva sınırı artık ince ve yüzün yanında ayrı bir çizgi olarak basılır
      (Uygulama'nın 3 çizgisine karşı 4);
    - 1:200'de mürekkep 23'e 20 (eskiden kalın sınırlar 4 fazlasını veriyordu).
- **Test takımları:**
  - kentos-geometry-core 635 + 356 ve öbürleri;
  - kentos-native-style 43 + 44 + 2 + 2; kentos-plot 18 + 90;
  - kentos-formats bütün takımlar; kentos-headless 22 + 3 + 6 + 97;
  - kentos-interaction 77 + 896; kentos-native-application 146 + 526.
  - Masaüstünde 1981 geçti, 1 düştü. Düşen `module_surface_tests::each_program_docks_its_own_panels`, `main`'den gelen
    eski hata.

## Kalanlar

- Ölçeğe göre çizgi grubu (M12), kesme düzleminin önündeki çizgiler (M13).
- DXF'te parça başına kalınlık; kapı yayının DXF'te ayrı katmanı (EK-4 bir ad vermiyor).
- Kolon, merdiven ve döşemenin görünen ve kesilen parçalarına rol.
