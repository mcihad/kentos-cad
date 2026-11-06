# 1620 — Sıfır hattı: iki yüzeyin kazı-dolgu sınırı (`road.terrains.zeroline`)

Tarih: 2026-11-07. plan/map-cad.md MAP-15, üçüncü dilim. Alt madde: “Kazı/dolgu sınırı, sıfır hattı … ayrı parametre
olsun”. Durum: taslak. İlgili ADR'ler:
- [ADR 1600](1600-surface-volume-method-and-factors.md): yüzey farkının yöntemi;
- [ADR 0916](0916-road-surface-analysis-and-quality.md) §3: yüzey farkı;
- plan/road.md Y2.5: eş yükselti eğrileri, aynı yazım yolu.

## Dayanak

- **Civil 3D:** iki yüzeyin hacim yüzeyinde (TIN volume surface) kazı ile dolgunun sınırı, farkın sıfır eş yükselti
  eğrisi olarak gösterilir ve çıkarılabilir. Volumes Dashboard kazı ve dolgu alanlarını ayrı renklendirir
  ([Volumes Dashboard](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-7246FF86-2C5F-4B5A-991D-5571DAAC0CDF.htm)).
- **Bentley OpenSite:** kazı/dolgu ayrımını (“cut and fill delineation”) saha tasarımının bir çıktısı olarak sunar
  ([OpenSite Designer](https://www.bentley.com/en/products/product-line/civil-design-software/opensite-designer)).
- **Saha uygulaması:** sıfır hattı, aplikasyonda kazının bittiği ve dolgunun başladığı yerdir. Tesviye projesinin
  pafta çizimidir.
- **KentOS bugün:** yüzey farkı hacmi, alanı, uçları ve aralıkları veriyordu (ADR 0916, 1600), ama sınırın kendisini
  çizime yazmıyordu.

## Karar

1. **`road.terrains.zeroline` v1:**
   - **Girdi:** `base`, `comparison` (kimlik ya da ad), `layerId`, `expectedRevision`.
   - **Hesap:** karşılaştırmanın planında farkın (karşılaştırma − taban) sıfır olduğu çizgiler bulunur.
   - **Yazım:** çizgiler tabanın kotuyla 3B çoklu çizgi olarak, tek geri alma adımı “Sıfır hattı” ile yazılır.
   - **Kapsam:** Harita ve Yol (ADR 1520).
   - **Çıktı:** her çizginin kapalılığı, plan uzunluğu ve yerleri; toplam uzunluk; iki yüzeyin çakıştığı alan
     (`coincident`; orada çizgi yoktur, söylenir).
2. **Çekirdek** (`kentos_road::volume::zero_lines`):
   - İki yüzeyin örtüşmesinin her dışbükey parçasında fark doğrusaldır. Sıfır kümesi kestiği kenarlar arasında bir
     parçadır; sıfırdaki köşe bir kez sayılır.
   - Parçalar uçlarından birleştirilir; uçlar 10⁻⁹ m'ye eşitlenir.
   - İki yanın bulduğu aynı parça bir kez alınır.
   - Üç ya da daha çok parçanın buluştuğu köşe çizgileri ayırır.
   - Komşularıyla doğrudaş köşe atılır (10⁻¹² oranında, yalnız sayısal gürültü).
   - Her çizgi en küçük yerinden başlar (x, sonra y) ve çizgiler buna göre sıralanır.
3. **Retler:**
   - proje türü, katman, sürüm, bilinmeyen yüzey, aynı yüzey iki kez, kurulamayan yüzey;
   - `no_zero_line` (yeni kod): fark hiçbir yerde işaret değiştirmiyor ya da yüzeyler örtüşmüyor.
4. **Masaüstü:** Yüzey analizi › Fark'ta Katman seçimi ve **Sıfır hattını çiz**. Yeni şerit düğmesi ve ikon yoktur.
   Pencerenin `operations.json` kaydı `road.terrains.zeroline`'ı anar.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/road_terrain_zeroline_cases.py` 23 durum üretir.
  - Bağımsız hesap:
    - kafeslerin (z = g(x) + h(y)) birleşik ızgarasının her dikdörtgeninde fark doğrusaldır; sıfır parçası
      kenarlardan bulunur;
    - aynı birleştirme kuralı uygulanır;
    - KentOS kodu kullanılmaz.
  - Durumlar:
    - düz üstünde tümsek: kapalı eşkenar dörtgen, 4·4·√2 m;
    - teras üstünde set: kafes çizgileri üstünde iki açık çizgi; iki yanın parçası bir kez alınır;
    - tersleri, kimlikle, adla, plan, doğrulama, başka ve gizli katman, Harita projesi;
    - retler: sıfır hattı yok, örtüşmeyen yüzeyler, aynı yüzey, olmayan yüzey, kurulamayan yüzey, kilitli katman,
      grup, olmayan katman, sürümler, mimari proje.

  Rust ve Python geçer.
- **Masaüstü** (`surface_analysis_window_tests::fark_draws_the_zero_line_in_one_step`): Fark'tan çizilen kapalı çizgi
  katmanındadır; geri alma adımı “Sıfır hattı”dır. Resimler `.run/gui-evidence/sifir-hatti-1620/` altındadır.
- **Yan bulgu:** `cad.pointclouds.overlap`'in iki kodu (MAP-13) ne durumla ne gerekçeyle anılmıştı. Gerekçeleri
  yazıldı; `history::text`'in başlıkları yenilendi.

## Sonra

- Kazı ve dolgu bölgelerinin alan olarak (taralı) yazılması, malzeme bölgesi ve stok sahası.
- Sıfır hattının tasarım değişince canlı güncellenmesi (`road.project.rebuild`'in tutulan çizimleri gibi).
