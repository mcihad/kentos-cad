# 1520 — Harita projesinde arazi yüzeyi (`.kcad` şema 195)

Tarih: 2026-11-06. plan/map-cad.md MAP-14 (ilk dilim: mevcut yüzey tarifi Harita'ya açılır). Durum: taslak. İlgili:
[ADR 0802](0802-strict-module-separation.md) (modül ayrımı), plan/road.md Y2 (yüzey tarifi: nokta ve kırık hat katmanları, sınır,
boşluklar, en uzun kenar, kot toleransı, nokta bulutu; üçgenler kaydedilmez, kaynaklardan kurulur),
[ADR 1500](1500-scan-overlap-check.md).

## Dayanak

- **Türk mevzuatı:** BÖHHBÜY (RG 26.06.2018/30460) şunları söyler (metin harita.gov.tr'den okundu):
  - **m. 6(1)(l)** (2026-11-10 düzeltmesi: önce “m. 4(1)(l)” yazılmıştı): sayısal arazi modeli, çıplak arazinin “karesel düzenli bir grid yapıda veya düzensiz üçgensel
    formdaki” yükseklik verisidir. TIN biçimi haritacılığın kendi tanımıdır, yalnız yol projesinin değil.
  - **m. 45(1)(c):** eş yükseklik eğrisi için ölçülen detay noktası 25 nokta/ha'dan az olamaz; desen ve karakteristik
    noktalar da ölçülür. Kırık hat bunların taşıyıcısıdır.
  - **m. 63(1):** yerleşim dışındaki alanlarda topoğrafya eş yükseklik eğrileriyle gösterilir; otomatik çizimde
    arazinin morfolojik yapısı dikkate alınır.
  - **m. 78(1)(c–g):**
    - (c) aralık 1/5000'de 5 m, 1/2000'de 2 m, 1/1000 ve 1/500'de 1 m;
    - (d) her beşinci eğri kalın (0,25 mm, ötekiler 0,13 mm);
    - (e) eğri arası 20 mm'yi aşınca kesik yardımcı eğri;
    - (g) eğri yol, nehir ve bina gibi detayları kesmez.
- **Netcad NetSURF:** sayısal arazi modeli, topoğrafik harita ve hacim hesabının ortak kaynağıdır; Netcad'de harita
  üretiminin parçasıdır, yol modülüne bağlı değildir ([Netcad ürünleri](https://en.netcad.com/en/products)). Ayrıntılı
  algoritma belgesi herkese açık bulunamadı (arama 6 Kasım 2026).
- **Civil 3D:** yüzey bir tanım listesidir. Noktalar, kırık hatlar, konturlar ve sınırlar sırayla eklenir; kırık hat
  üçgenlerin kesemeyeceği çizgidir; sınırın gizlediği alan alan ve hacim hesabına girmez. Kaynaklar:
  - [About the Surface Definition](https://help.autodesk.com/cloudhelp/2021/ENG/Civil3D-UserGuide/files/GUID-EB41E9FE-56DC-4469-8280-2C2C8B7F1B1B.htm);
  - [About Breaklines](https://help.autodesk.com/cloudhelp/2015/ENU/Civil3D-UserGuide/files/GUID-FCCAF28A-506B-4773-9706-DD1F1F8CC933.htm);
  - [Creating and Adding Data to a Surface](https://help.autodesk.com/cloudhelp/2025/ENU/Civil3D-Tutorials/files/GUID-899731B5-0B6A-451E-9CF2-0DCF00FA9B64.htm).

  BricsCAD'in TIN yüzeyi de aynı tanım düzenini izler
  ([TIN Definitions](https://help.bricsys.com/en-us/document/bricscad/civil-tools/tin-surfaces/tin-definitions/tin-definitions-for-editing)).
- **KentOS bugün:** yüzey tarifi, Delaunay ve kısıtlı kırık hat, eş yükselti, sorgu, analiz, kalite, kapsam, yoğunluk,
  onarım, karşılaştırma, LandXML ve nokta bulutundan yüzey Yol'dadır (plan/road.md Y2). Harita projesi bunları açamıyordu:
  komutlar `not_a_road_project` ile reddediyordu. Arazi sekmesi de yalnız Yol'undu.

## Karar

1. **Yüzey Harita ve Yol'undur.**
   - `road_terrains::holds_surfaces` iki işi de kabul eder.
   - Yüzey komutları `terrain_project` ile denetlenir: `road.terrains.*` (yaratma, düzenleme, sorgu, eş yükselti,
     analiz, karşılaştırma, kalite, kapsam ve çizimi, yoğunluk, onarım, bulut), `file.import.landxml`.
   - Mimari projede ret aynı koddur (`not_a_road_project`), ileti “Arazi yüzeyi Harita ve Yol projelerinde olur”.
   - Tesviye, drenaj ve platform Yol'un kalır (`road_project`).
2. **`.kcad` şema 195.**
   - Harita projesinin yüzeyleri ayarların `road`'unda yazılır, ama yalnız `surfaces`. Hizalama, profil, kesit,
     koridor ya da yol'un başka bir parçası harita projesinde `bad_value`'dur (`RoadSettings::surfaces_only`).
   - Yazıcı 195'i harita projesinde `road` varken seçer. 194 ve öncesi bunu reddeder; eski program yeni dosyayı
     “daha yeni sürümle yazılmış” diye açmaz.
   - Bağımsız okuyucu `tools/kcad/kcad.py` aynı kuralı uygular.
   - Bu dilimin kendi bulgusudur: şema yükseltmesi olmadan yüzeyli Harita dosyası kaydedilemez ya da açılamazdı. Python
     ortak durumları `.kcad`'den geçerken yakaladı.
3. **Menü.**
   - Yol'un Arazi sekmesi Harita'da da açılır.
   - Harita'da görünen paneller: Yüzey (Yüzeyler, Yüzey oluştur, Buluttan), Analiz (Yüzey analizi), İşlemler.
   - Tesviye, Drenaj ve Arazi (kesit sorgusu) panelleri `works: ["road"]` ile Yol'da kalır.
   - Yeni ikon yoktur; Yol'un ikonları kullanılır.
4. **Eş yükselti aralığı ölçekten.** Yüzey penceresi aralığın ilk değerini m. 78(1)(c)'ye göre çizim ölçeğinden alır:
   - 1/5000 → 5 m, 1/2000 → 2 m, 1/1000 ve 1/500 → 1 m;
   - yönetmeliğin anmadığı ölçekte 1 m, kullanıcı değiştirir.

   “Ana eğri her 5” m. 78(1)(d)'dir.

## Kanıt

- **Ortak durumlar:** 13 `road.terrains.*` dosyası ve `file.import.landxml` yeniden üretildi.
  - Yeni durum “Harita projesinde de yüzey kurulur”.
  - Mimari projede ret durumları korunur.
  - 408 Rust durumu geçer; Python durumları `.kcad`'den gidip gelerek geçer.
- **.kcad:**
  - Örnek `map-surfaces.json/.kcad`.
  - Bozuk dosyalar: `broken/map-surfaces-in-schema-194.kcad`, `broken/map-surfaces-alignment.kcad`.
  - `broken/road-settings-of-a-map-project.kcad` (şema 162) reddedilmeye devam eder.
  - `schema-version-196.kcad`; Rust codec ve bağımsız Python okuyucu aynı sonucu verir.
- **Masaüstü:**
  - `terrain_window_tests`: Harita'da yüzey ve eğri kurulur; Tesviye ve Drenaj sunulmaz; mimari projede yüzey yoktur;
    aralık ölçeğe göre başlar.
  - `module_tests` Harita ve CBS menülerini sınar.
  - Resimler `.run/gui-evidence/harita-yuzey-1520/`: Arazi sekmesi ve Yüzeyler penceresi, açık/koyu, geniş/dar.

## Sonra (MAP-14'ün kalanı)

- Yüzey tarifinin düzenleme adımları: nokta ekle/sil, kot değiştir, kenar çevir, üçgen dışla; geçmişten yeniden üretim.
- Aynı XY'de farklı Z ve çok yakın nokta politikasının raporu.
- Yalnız etkilenen bölgenin yeniden kurulması; bağlı eğri, profil ve hacmin yüzey revizyonuna bağlanması.
- Eğrinin detay kesmemesi (m. 78(1)(g)), yardımcı eğri (e), kalın eğride yükseklik yazısı (f).
- Gerçek büyük veriyle bellek ve süre.
