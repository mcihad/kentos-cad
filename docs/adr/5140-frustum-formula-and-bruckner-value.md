# 5140 — Kesitler arası kesik koni formülü; kübaj cetvelinde Brükner değeri

Tarih: 2026-11-11. plan/map-cad.md MAP-15, onuncu tur. Durum: taslak. ADR 4760'ın “Açık”ındaki “kesik koni formülü ve
Brükner”.

İlgili: [ADR 4340](4340-surface-comparison-by-sections.md) (kesit yöntemi), [ADR 4760](4760-sections-along-an-axis-and-materials-drawn.md)
(aks boyunca kesit, kübaj cetveli), [ADR 1600](1600-surface-volume-method-and-factors.md) (kazı ve dolgu katsayıları).

## Dayanak

- **KGM 2019 Karayolları teknik şartnamesi** (`.run/research/map-cad/mevzuat/MAP-15.9.md`, 10 Kasım 2026): kübaj kesit
  alanları ve aralıklarıyla hesaplanır; yarma hacmi araştırma raporundaki kabarma ya da sıkışma katsayısıyla çarpılır;
  Brükner diyagramı çizilir. Hacim formülü şartnamede adıyla geçmez.
- **Netcad**, Enkesitlerden Hacim Hesabı (resim `.run/research/map-cad/uygulama/MAP-15.9-resim/netcad-enkesitlerden-hacim-cetveli.jpg`,
  11 Kasım 2026'da yeniden bakıldı; not `MAP-15.10.md`): yöntemler KGM Yöntemi, (A1+A2+Karekök(A1·A2))·L/3,
  (A1+A2)·L/2, DSİ Yöntemi; cetvelin son sütunu Brükner Değeri, resimdeki satırda birikimli yarma − birikimli dolma
  (15487,575 − 0,803 = +15486,773).
- KGM ve DSİ yöntemlerinin formülü açık kaynakta bulunamadı: yapılmadı.

## Karar

1. **Kesik koni formülü** (`formula: frustum`; `SectionFormula::Frustum`): iki kesit arasında L/3·(A₀ + A₁ + √(A₀·A₁)),
   kazı ve dolgu ayrı ayrı. `road.terrains.compare`'in kesit yönteminde (semtle ve aks boyunca) ve `road.terrains.cutfill`'in
   kesitlerinde. Çekirdekte `volume_sections::Formula` ve `frustum`.
2. **Brükner değeri:** `road.terrains.cutfill`'in her kesitinde (ilki hariç) `mass` = birikimli kazı × `cutFactor` −
   birikimli dolgu × `fillFactor` (`sections.cutFactor`, `sections.fillFactor`; yoksa 1; 0'dan büyük, en çok 10;
   aksi `invalid_analysis`). Kübaj cetveline son sütun “Brükner (m³)”. Katsayılar resmî değer değildir: projenin
   araştırma raporundan, kullanıcınındır.
3. **Masaüstü** (Yüzey analizi › Fark › Kesit): Formül seçimine **Kesik koni**; kesitlerin çizime yazılmasında Brükner
   değeri karşılaştırmanın Kazı ve Dolgu katsayılarıyla.
4. **Kapsam işareti:** **Harita + Yol**.

## Sınırlar

- Brükner diyagramı (grafik) çizilmez; değer cetvelde.
- Kesit sayfaları, UVDF §6.4 dışa aktarımı ve ek kesitler (üçgen kenarı, ekseni kesen çizgi) bu turda yapılmadı.

## Kanıt

- **Bağımsız başvuru:** `scripts/fixtures/road_terrain_compare_section_cases.py` 46 durum (3 yeni: semtle kesik koni, katsayılı
  kesik koni, yaylı eksende kesik koni); `scripts/fixtures/road_terrain_cutfill_cases.py` 62 durum (4 yeni: kesik koni ve
  Brükner sütunlu cetvel, 1,25 / 0,9 katsayılı Brükner, iki ret). Rust ve Python geçer. Bozma denemesi: bir kesik koni
  kazısı ve bir Brükner değeri değiştirilince koşucu tam o ikisini yakalar.
- **Çekirdek:** `kentos-road` `volume_sections` testleri (yeni: kesik piramidin hacmi kesin, eşit alanda ortalama alan).
- **Masaüstü:** `compare_sections_tests` (kesik koni ortak durumun hacmi); çapraz denetimler 153 test. Resim
  `.run/gui-evidence/kesik-koni-brukner-5140/`. Gerçek pencerede kabul yapılmadı.
- **Python:** iki komutun 108 ortak durumu Python'un tipli katmanından geçer. Yeni bir MCP testi yazılmadı (komutlar
  aynı; araç kataloğu yeni alanları şemadan alır).

## Sonra

- Kesit sayfaları (kesitin kendi çizimi), UVDF §6.4 dışa aktarımı, ek kesitler (üçgen kenarı, ekseni kesen çizgi),
  Brükner diyagramı.
- Web (TODOS §0.2).
