# 1580 — Harita projesinde tesviye ve drenaj (`.kcad` şema 197)

Tarih: 2026-11-07. plan/map-cad.md MAP-15, ilk dilim. Alt maddeler:
- “Meydan, otopark, park, platform, şev … için hedef kot/eğim, referans yüzey ve gün ışığına çıkış”;
- “Akış yönü/çukur/su birikme denetimi”;
- “road planının yeteneklerini Harita projesinde kullanılabilir hale getir”.

Durum: taslak. İlgili ADR'ler:
- [ADR 0927](0927-road-platform-grading.md): platform ve tesviye;
- [ADR 0929](0929-road-surface-drainage.md): drenaj;
- [ADR 1520](1520-map-project-terrain-surfaces.md): Harita'da yüzey.

## Dayanak

- **Bentley OpenSite:** saha tasarımını yol tasarımından ayrı bir ürün olarak sunar; arazi modeli, kazı/dolgu
  ayrımı, hacim karşılaştırma ve yağmur suyu drenajı bir aradadır
  ([OpenSite Designer](https://www.bentley.com/en/products/product-line/civil-design-software/opensite-designer),
  [SITEOPS](https://www.bentley.com/en/products/product-line/civil-design-software/siteops)).
  Meydan, otopark ve bina platformu bir harita/saha işidir, yol projesi gerektirmez.
- **Civil 3D:** grading (feature line, grading group, gün ışığına çıkış) ve yüzey hacmi yol koridorundan bağımsızdır.
- **Türk mevzuatı:** Planlı Alanlar İmar Yönetmeliği (RG 03.07.2017/30113) taban alanını “tabii zemin veya tesviye
  edilmiş zemin” üzerinden tanımlar. Tesviye edilmiş zemin, yapı ruhsatı ve harita işinin konusudur. Kazı ve dolgu
  sınırlarına dair ayrıntılı ölçüt bu dilimde aranmadı; sonraki dilimin (kazı/dolgu raporu) konusudur.
- **KentOS bugün:**
  - platformlar (taban kotu, eğim ve yön, yarma ve dolgu şevi, şev çizgileri, yelpaze, gün ışığı, dengeleme) ve
    drenaj (gölet, su damlası, havza, Rasyonel debi) yalnız yolu bilir;
  - ikisi de yalnız bir arazi yüzeyine dayanır, hizalamaya ve koridora dayanmaz;
  - Harita projesi bunları açamıyordu.

## Karar

1. **Platformlar ve drenaj Harita ile Yol'undur:**
   - `road.platforms.edit`, `.query`, `.draw`, `road.drainage.query`, `.draw`;
   - mimari projede ret aynı koddur (`not_a_road_project`);
   - iletiler “Harita ve Yol projelerinde olur” der.

   Koridorun platform suyu (`road.corridors.water`) Yol'da kalır.
2. **`.kcad` şema 197:**
   - Harita projesinin `road`'u yüzeylerin yanında `platforms` da taşır (`RoadSettings::map_part_only`). Hizalama,
     koridor ve öbür yol parçaları yine reddedilir.
   - Yazıcı, harita projesinde platform varken 197'yi seçer; 196 ve öncesi bunu reddeder.
   - Bağımsız okuyucu ve başvuru yazıcısı da aynı kuralı uygular.
3. **Menü:** Arazi sekmesinin Tesviye (Platform) ve Drenaj panelleri Harita'da da açılır. Kesit sorgusu (Arazi paneli)
   Yol'da kalır. Yeni ikon yoktur.
4. **Bu dilimin dışında kalanlar:**
   - kazı/dolgu raporunda yöntem, çözünürlük ve hata sınırı;
   - kabarma/sıkışma, malzeme bölgesi, stok sahası;
   - geometrik hacim ile iş miktarının ayrılması.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/road_platform_cases.py` ve `road_drainage_cases.py` yeniden üretildi.
  - Retler mimari projede.
  - Yeni durumlar: “Harita projesinde de platform”, “Harita projesinde de su damlası”. Harita kurulumunun yol
    ayarında yalnız yüzeyler ve platformlar bulunur.
  - Rust ve Python (`.kcad` gidiş-dönüşüyle) geçer.
- **`.kcad`:** `map-platforms.json/.kcad` ve `broken/map-platforms-in-schema-196.kcad`; `schema-version-198.kcad`.
  Rust ve bağımsız okuyucu aynı sonucu verir.
- **Masaüstü** (`platform_window_tests::a_map_project_grades_its_pads`):
  - Harita'da platform penceresi tesviye eder;
  - kaydedilen çizim `.kcad`'e yazılıp geri okunur;
  - Drenaj penceresi açılır;
  - `terrain_window_tests` Harita'nın Tesviye ve Drenaj'ı sunduğunu, kesit sorgusunu sunmadığını sınar.

  Resimler `.run/gui-evidence/harita-tesviye-1580/` altındadır.

## Sonra

MAP-15'in kalanı:
- tasarım ile mevcut yüzey karşılaştırmasında yöntem (TIN–TIN, ızgara, kesit), çözünürlük, eksik alan ve hata sınırı;
- kazı/dolgu sınırı ve sıfır hattı;
- kabarma/sıkışma katsayısı ve malzeme ile stok parametreleri;
- canlı boy/en kesit türevi.
