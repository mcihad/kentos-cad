# 1039 — Enkesit neyden hesaplandığını söyler

Tarih: 2026-11-07. Durum: kabul (Y104). Kapsam ROAD-SEC-04 (kaynak sürümleri), ADR 0903 (kesit takımı), ADR 1038
(aplikasyonun kaynakları).

## Sorun

ROAD-SEC-04, mevcut, tasarım, malzeme ve jeoloji kesitlerinin aynı km'de ayrı çizgiler ve kaynak sürümleriyle
gösterilmesini istiyor. Denetimde (2026-11-05) şu durum vardı:

- arazi, tasarım ve üstyapı çizgileri ayrı;
- jeoloji yok;
- kaynak sürümleri yok.

Kesit takımının raporu aksını, boykesitini, tip kesitini, deverini ve yüzeyini kimlikle anıyordu, sürümleriyle değil.

## Araştırma (7 Kasım 2026)

- **ADR 1038:** aynı soru aplikasyon cetveli için cevaplandı. Kaynakların sürümleri ve arazinin kabul özeti tanımda
  durur, yeniden hesap gerekmez.
- **Civil 3D'nin kesit görünüşü:** örnek çizgisi grubu hangi yüzeylerin, koridorun ve malzeme listesinin
  örneklendiğini tutar. Kaynak değişince kesitler "out of date" olur. KentOS'ta karşılığı takımın eskime uyarısıdır
  (`section_set_stale`).
- **Jeoloji:** tabaka (sondaj ve jeolojik kesit) modeli KentOS'ta yok; ayrı bir veri modeli ister.
- **Matematik:** yok.

## Karar

- **`SectionsQueried.sources`:** bir takım sorulunca, her biri bugünkü hâliyle:
  - takım ve aksı, sürümleriyle;
  - takımın andığı boykesit, tip kesit ve dever çizelgesi, sürümleriyle;
  - arazi yüzeyi ve son kabul özeti.
  Projede olmayan bir kimlik sürüm 0 okunur. Bütün takımların raporunda yazılmaz.
- **Ortak tür:** ADR 1038'in türleri iki komutta kullanılsın diye `RoadSource` ve `RoadSurfaceSource` adını aldı
  (eskiden `StakeoutSource`, `StakeoutSurface`).
- **Enkesit penceresi:** kesitin altında "Kaynaklar: takım k-2 (3), aks g-2 (3), boykesit b-2 (4), tip kesit tk-2 (2),
  arazi t-1 (kabul edilmedi)". Penceredeki taslak takım kendi tanımıyla okunur.
- **Açık:** jeoloji kesiti; ROAD-SEC-04 kısmen kalır.

## Ölçüm

- **Bağımsız başvuru** `road_section_command_cases.py` (`sources_of`): her durumun kendi kurulumundaki tanımlardan.
- **`road.sections.query`:** 37 durumun 27'si kaynakları karşılaştırıyor (takım sorulan her durum).
- Tip kesitin sürümü bir artırılınca durum düşüyor.
- Öbür kesit ve aplikasyon durumları değişmedi.
- **Masaüstü testi:** ana takımın satırı; yan takımda eskime uyarısıyla birlikte aksın 3. sürümü.
- **Resim** `.run/gui-evidence/yol-y104/`.
