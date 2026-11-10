# 4660 — Senaryo ölçülerinde eğim sınıfları ve bakı

Tarih: 2026-11-10. plan/map-cad.md MAP-10, dokuzuncu tur. Durum: taslak. ADR 4260'ın “Sonra”sındaki “Eğim sınıflarına göre
alanlar …; bakı”.

İlgili:
- [ADR 4260](4260-scenario-terrain-slope.md): arazi eğimi (`cad.scenarios.impact`'in `slope`'u);
- [ADR 2040](2040-impact-impervious-and-access.md): senaryo ölçüleri, değerler kullanıcının.

## Dayanak

- **ArcGIS Pro**, [Surface Slope (3D Analyst)](https://doc.esri.com/en/arcgis-pro/latest/tool-reference/3d-analyst/surface-slope.html)
  (10 Kasım 2026'da okundu):
  - üçgenlenmiş yüzeyin eğim aralıklarını alan olarak verir;
  - her yüzün eğimi iki kenarının vektörel çarpımı olan normalden bulunur; birim yüzde (öntanımlı) ya da derecedir;
  - öntanımlı dokuz sınıf vardır; özel sınıflar bir sınıf sınırları tablosuyla verilir (birinci sütun sınır, ikincisi
    kod).
- **ArcGIS Pro**, [Surface Aspect (3D Analyst)](https://doc.esri.com/en/arcgis-pro/latest/tool-reference/3d-analyst/surface-aspect.html)
  (10 Kasım 2026'da okundu):
  - üçgenlenmiş yüzeyin bakısını alan olarak verir; bakı yüzeyin yatay yönelimidir, derece;
  - öntanımlı sınıflar: Kuzey 0°–22,5° ve 337,5°–360°, Kuzeydoğu 22,5°–67,5°, Doğu 67,5°–112,5°, Güneydoğu
    112,5°–157,5°, Güney 157,5°–202,5°, Güneybatı 202,5°–247,5°, Batı 247,5°–292,5°, Kuzeybatı 292,5°–337,5°;
  - eğimsiz yüz −1, “No Slope”;
  - özel sınıflar sınıf sınırları tablosuyla verilir.
- **ArcGIS Pro**, [How Aspect works](https://doc.esri.com/en/arcgis-pro/latest/tool-reference/3d-analyst/how-aspect-works.html)
  (10 Kasım 2026'da okundu): bakı kuzeyden (0) saat yönünde 360'a ölçülür; eğimsiz yer −1 alır.
- **KentOS bugün** (ADR 4260): ortalama eğim, dik alan ve payı, yüzeyin dışındaki alan. Alan eğim sınıflarına göre
  dağıtılmıyor, bakı ölçülmüyordu.

## Karar

1. **`cad.scenarios.impact`'in `slope`'una** (sürüm aynı, ek alanlar):
   - `classes`: kullanıcının sınıf sınırları, yüzde, artan; en çok 20. KentOS sınır önermez (ArcGIS'in öntanımlı dokuz
     sınıfı KentOS'un değeri değildir);
   - `aspect`: bakıya göre alanlar.
2. **Sınıflar:** 0'dan ilk sınıra, iki sınır arasına ve son sınırdan yukarı. Her sınıf alt sınırını tutar: eğim = 5
   ise 5–15 sınıfındadır (dik eşiğin “en az”ı gibi). Her sınıfın alanı (m²) o sınıftaki üçgenlere düşen alandır.
   Satırlar `slopeClass1` … `slopeClassN+1`: “Eğimi 5 %'den az”, “Eğimi 5–15 %”, “Eğimi en az 30 %”.
3. **Bakı:**
   - üçgenin yukarı bakan normalinin yatay parçası eğimin indiği yöndür (z = c + a·x + b·y için (−a, −b));
   - yönü kuzeyden saat yönünde ölçülür; sınıfı 1 + ⌊(yön + 22,5°) / 45°⌋ mod 8'dir (ArcGIS'in sekiz yönü, her biri alt
     sınırını tutar);
   - normali tam düşey olan (yatay) üçgen **Düz**'dür; tolerans yoktur;
   - satırlar `aspectFlat`, `aspectN`, `aspectNE`, `aspectE`, `aspectSE`, `aspectS`, `aspectSW`, `aspectW`, `aspectNW`:
     “Bakı: Düz”, “Bakı: Kuzey” …, her birinin notunda sınırları.
4. **Ret** (`invalid_input`, `slope.classes`): sonlu ve 0'dan büyük olmayan, artmayan ya da 20'den çok sınır. Söz:
   “Eğim sınıflarının sınırları 0'dan büyük, artan yüzdeler olmalı, en çok 20 sınır (yazılan 15; 5).”
5. **Masaüstü** (Senaryo ölçüleri): **Eğim sınıfları (%) / bakı** satırı: sınırlar noktalı virgülle (“5; 15; 30”;
   virgül ondalık), yanında **Bakıya göre alanlar** anahtarı. Okunmayan sınır söylenir, tahmin edilmez.
6. **Kapsam işareti:** ADR 4260 gibi **Harita + CBS** (eğim yüzeyi olan projede).

## Sınırlar

- Sınıflar ve bakı çizimde (haritada) gösterilmez; ölçü satırlarıdır.
- Sınıflar yalnız yüzdedir; derece birimi yoktur.
- Bakının sekiz yönü sabittir; özel bakı sınıfı yoktur.
- Sınıf sınırına tam düşen yön ya da eğim alt sınırın sınıfındadır; kayan noktanın yuvarlaması kesin sınırda sınıfı
  değiştirebilir (durumların üçgenleri sınırlardan uzaktır).

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/scenarios_impact_cases.py`; KentOS kodu yok, kesin kesirli kırpma), 5 yeni
  durum (50):
  - 5; 15; 30 sınıfları ve bakı: iki üçgen ayrı sınıflarda (≈ 22,6 % ve ≈ 13,4 %), bakıları Güney (≈ 186°) ve
    Güneybatı (≈ 240°);
  - konutta tek sınır (20 %);
  - aynı köşeler 100 m'de yatay: bütün alan Düz ve ilk sınıfta;
  - iki ret.

  Eski 45 durum aynı. Rust ve Python geçer. Bozma denemesi: bakı yokuş yukarı alınınca koşucu yakalar.
- **Python** (`python/tests/test_scenario_slope_classes_aspect.py`), testin kendi hesabıyla: yüzeyi kendi kurar,
  üçgenlerin bakısı Güney ve Güneybatı; konut alanlarını 10 cm ızgarada tümler. Sınıfların ve bakıların alanları
  ızgaranın hatası içinde; sınıfların toplamı yüzeydeki alan; iki ret.
- **MCP** (`crates/native/mcp/tests/scenario_slope_classes_aspect.rs`): sınıf ve bakı satırları durumunki; artmayan
  sınırlar `slope.classes`'ta reddedilir.
- **Masaüstü** (`impact_slope_classes_tests`): sınırlar ve anahtarla pencerenin 13 satırı durumunki; okunmayan sınır
  söylenir. Senaryo ölçüleri testleri ve çapraz denetimler geçer. Resimler `.run/gui-evidence/egim-sinif-baki-4660/`.

## Sonra

- Eğim sınıflarının ve bakının haritada gösterimi; derece birimi; özel bakı sınıfları.
- Ağ üzerinden erişim; damganın çizimde saklanması (MAP-10'un kalanı).
- Web (TODOS §0.2).
