# 4260 — Senaryo ölçülerinde arazi eğimi

Tarih: 2026-11-09. plan/map-cad.md MAP-10, sekizinci tur. Durum: taslak. ADR 1440'ın ve 3860'ın “Sonra”sındaki
“arazi eğimi (yüzey)”.

İlgili:
- [ADR 1440](1440-design-alternative-impact.md): Senaryo ölçüleri (`cad.scenarios.impact`);
- [ADR 2660](2660-scenario-infrastructure-proximity.md): altyapıya yakınlık, kullanıcının değeriyle istenen ölçü;
- [ADR 1520](1520-map-project-terrain-surfaces.md): Harita projesinin arazi yüzeyleri;
- [ADR 4180](4180-elevations-from-surface.md): Yüzeyden kot ver, yüzeyin üçgenleri.

## Dayanak

- **ArcGIS Pro, Surface Slope (3D Analyst)**
  ([belge](https://doc.esri.com/en/arcgis-pro/latest/tool-reference/3d-analyst/surface-slope.html), 9 Kasım 2026'da
  okundu):
  - TIN yüzeyinde eğim her üçgenin normalinden bulunur; normal iki kenarın vektörel çarpımıdır;
  - yüzde eğim yükseklik değişiminin yatay uzaklığa oranıdır;
  - çıktı eğim sınıflarına göre alanlardır, sınıf sınırları kullanıcının tablosuyla da verilir.
- **Plan maddesi** (plan/map-cad.md MAP-10): karşılaştırma arazi eğimini de kaynaklı ölçmeli. Birim ve varsayım
  sonuçla taşınır, eksik veriyle kesin uygunluk verilmez.
- **KentOS bugün:** Senaryo ölçüleri alan, inşaat alanı, geçirimsizlik, erişim ve altyapıya yakınlığı ölçüyordu,
  eğimi ölçmüyordu. Harita ve Yol projelerinin arazi yüzeyleri vardı (ADR 1520).

## Karar

1. **`cad.scenarios.impact`'e `slope`** (sürüm 1 kaldı; istenince hesaplanan ölçü, ADR 2040'taki gibi):
   - projenin arazi yüzeyi (`surface`: kimliği ya da adı);
   - dik sayılan eğim (`threshold`, %, kullanıcının; KentOS değer vermez);
   - ölçülen kullanımlar (`from`; boş: bütün alanlar).

   Yüzey iki tarafta da aynıdır.
2. **Hesap:**
   - Yüzeyin her üçgeninin eğimi düzleminden bulunur, ArcGIS'teki gibi:
     100 · √(nx² + ny²) / |nz|; n, iki kenarın vektörel çarpımı. Dikey ya da yozlaşmış üçgen atlanır.
   - Her ölçülen alan her üçgenle kesiştirilir (çekirdeğin örtüşmesi, yaylar ve delikler kesin).
   - Ortalama eğim (`slopeMean`, %): parçaların alan ağırlıklı ortalamasıdır.
   - Dik alan (`slopeSteep`, m²): eğimi eşik ya da üstü olan üçgenlere düşen alandır.
   - Dik alanın payı (`slopeSteepShare`, %): dik alanın yüzeydeki alana oranıdır.
   - Yüzeyin dışındaki alan (`slopeOff`, m²): üçgenlere düşmeyen kısımdır, eğimi bilinmez.
3. **Veri eksiği:** yüzeyin dışında 1 mm²'den çok alan kalan taraf söylenir: “Ölçülen alanların bir kısmı senaryoda
   Zemin yüzeyinin dışında; eğim yalnız yüzeyin üstündeki kısımdan.” 1 mm²'nin altı örtüşmenin yuvarlamasıdır, yüzey
   dışı arazi değildir; ölçünün kendisi değeri yine verir.
4. **Retler**, sırasıyla:
   - `invalid_input`: `slope.threshold` 0'dan büyük değil;
   - `scenario_not_found`, `nothing_to_compare`;
   - `not_a_road_project`: yüzeyi olmayan proje;
   - `unknown_terrain`: `slope.surface`;
   - `terrain_unbuildable`.
5. **Masaüstü** (Senaryo ölçüleri, “Ölçüler (değerler sizin)” bölümü):
   - yeni satırlar Eğim yüzeyi / dik eşik (%) ve Eğimde ölçülen kullanımlar;
   - yüzey listesi projenin yüzeyleridir, ilki “—” (sorulmaz);
   - dört ölçü tablonun ölçüleri arasında, veri eksiği özet satırlarında;
   - okunmayan eşik söylenir, tahmin edilmez.
6. **Kapsam işareti:** **Harita + CBS**, ADR 1440'taki gibi; eğim yalnız arazi yüzeyi olan (Harita, Yol) projede.
   Eğim bir mühendislik kararı ya da imar uygunluğu değildir.

## Sınırlar

- Eğim üçgen başına tek değerdir; eğim sınıflarına göre alan (ArcGIS'in sınıf poligonları) ve haritada gösterim yok.
- Yön (bakı), yamaç uzunluğu, kazı-dolgu ve arazi işi hacmi bu ölçüde yok.
- Senaryoya göre değişen yüzey yok: iki taraf aynı yüzeyle ölçülür.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/scenarios_impact_cases.py`; KentOS kodu yok): yeni 8 durum, toplam 45.
  - Dört noktalı yüzeyin Delaunay köşegeni boş çember sınamasıyla (kesirlerle) seçilir. Üçgenlerin eğimi düzlemden,
    alanlar üçgenlerle Sutherland–Hodgman'la kesin kesirlerle kırpılır.
  - Durumlar:
    - Zemin'de 10 % eşikle; senaryonun doğudaki konutu yüzeyden taşar;
    - yalnız konut alanları, yüzey adıyla; yüksek eşik;
    - altyapıyla birlikte;
    - dört ret.

  Eski 37 durum değişmedi. Rust ve Python geçer; Altyapı bağlarını çiz'in işlem durumları güncel.
- **Python** (`python/tests/test_scenario_slope.py`): testin kendi Delaunay seçimi ve 10 cm ızgara integrali.
  - konut alanlarının iki taraftaki ortalama eğimi;
  - senaryoda yüzey dışında 50 m² ve yalnız senaryonun veri eksiği;
  - senaryoda değişen kararın (K2 ticaret-konut → konut) konut eğimini değiştirmesi.
- **MCP** (`crates/native/mcp/tests/scenario_slope.rs`): dört ölçü durumunkiler, yüzey dışı 50 m², tek veri eksiği.
  MCP'nin ana testleri geçer.
- **Masaüstü** (`impact_slope_tests`): eşiksiz sorulmaz; eşikle dört ölçü ve veri eksiği durumunkiler; okunmayan eşik
  söylenir. Senaryo ölçüleri testleri ve çapraz denetimler geçer. Resimler `.run/gui-evidence/senaryo-egim-4260/`.

## Sonra

- Eğim sınıflarına göre alanlar ve haritada gösterim; bakı.
- Ağ üzerinden erişim; damganın çizimde saklanması (MAP-10'un kalanı).
- Web (TODOS §0.2).
