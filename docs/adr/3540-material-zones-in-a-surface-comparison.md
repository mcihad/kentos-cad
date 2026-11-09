# 3540 — Yüzey farkında malzeme bölgeleri (`road.terrains.compare`)

Tarih: 2026-11-09. plan/map-cad.md MAP-15, altıncı tur. Durum: taslak. Planın “Kazı/dolgu sınırı, sıfır hattı, malzeme
bölgesi, kabarma/sıkışma katsayısı ve stok sahası ayrı parametre olsun” maddesinin kalanı; ADR 1600, 1840, 2340, 2740 ve
3140'ın “Sonra”larındaki “malzeme bölgesi”.

İlgili:
- [ADR 1600](1600-surface-volume-method-and-factors.md): hacim yöntemi, kazı ve dolgu katsayıları;
- [ADR 1840](1840-cut-and-fill-zones.md): kazı ve dolgu bölgeleri (farkın işaretine göre bölgeler; bu ADR'nin
  bölgeleri malzemenin yeridir, ayrı kavramdır);
- [ADR 3140](3140-stockpile-volume-over-a-boundary-base.md): stok hacmi (sınırla kırpma ve moment tümlemesi).

## Dayanak

- **Civil 3D 2024 Kullanıcı Kılavuzu** (9 Kasım 2026'da okundu):
  - [To Add a Bounded Area to the Volumes Dashboard for Analysis](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-040A33A4-BC45-460D-93B7-36413BF4AD60.htm):
    hacim yüzeyi seçilir, Add Bounded Volume ile çizimde bir sınır seçilir. Sınır çoklu çizgi, 3B ya da 2B çoklu çizgi,
    parsel çizgisi, feature line, daire, elips, survey figure ya da parsel olabilir. Sınırlı alan hacim yüzeyinin
    altında bir alt satır olarak eklenir;
  - [Volumes Dashboard](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-7246FF86-2C5F-4B5A-991D-5571DAAC0CDF.htm):
    Add Bounded Volume hesabı seçilen sınırlı alanla sınırlar. Tabloda sınırlı alanın da Boundary, Cut Factor, Fill
    Factor, 2D Area, Cut/Fill/Net (Adjusted) sütunları vardır; sınır çizimde değişince hacim güncellenir.
  - Kılavuz sınırlı alanların örtüşmesini ve katsayının üst satırdan gelip gelmediğini söylemez.
- **Türk kaynağı:** ADR 1600'ün araması: Karayolları Teknik Şartnamesi (2013) hacim dönüşüm katsayısı vermez. KentOS
  malzemenin adını ve katsayısını uydurmaz: ikisi de kullanıcınındır.
- **KentOS bugün:**
  - `road.terrains.compare` iki yüzeyin farkını kesin TIN–TIN prizmalarıyla ölçüyor; katsayılar bütün farka tek
    uygulanıyor (ADR 1600);
  - stok hacmi bir sınırın içini yüzeyin üçgenleriyle kırpıp momentlerle tümlüyor (ADR 3140);
  - zeminin bir yeri kaya, öbürü toprak olduğunda malzemeye göre miktar çıkmıyordu.

## Karar

1. **`zones` girdisi** (`road.terrains.compare`, ek alan; komut sürümü değişmez). Her bölge:
   - `name`: malzemenin adı (kullanıcının; kırpılır, boş olamaz);
   - `boundary`: sınırın kimliği. Kapsam ve stokla aynı kural: düz kenarlı, tek parçalı kapalı çoklu çizgi ya da alan,
     delikleri olabilir;
   - `cutFactor`, `fillFactor`: bölgenin katsayıları. Yoksa karşılaştırmanınki, o da yoksa 1. 0'dan büyük, en çok 10.
2. **Ölçü** (`kentos_road::volume_materials`, belirli kural):
   - iki yüzeyin bindirme parçaları (dışbükey, farkı doğrusal) ilk köşesinden üçgenlere bölünür;
   - her üçgen bölgenin halkalarını kırpar (Sutherland–Hodgman, üçgen pencere). Parça farkın sıfırında bölünür;
     doğrusal fark parçanın alanı ve birinci momentleriyle kesin tümlenir (ADR 3140'ın kuralı);
   - koordinatlar bölgenin ilk köşesinden alınır: büyük harita koordinatlarında basamak kaybolmaz;
   - bölge her zaman kesin TIN–TIN ile ölçülür; ızgara yönteminde bölge istenmez (ret).
3. **Örtüşme:** iki bölge 1 mm²'den (10⁻⁶ m²) çok ortak alan paylaşamaz: bir yer tek malzemenin olur. Ortak alan bir
   bölgenin halkalarının öbürünün halkalarının yelpaze üçgenleriyle (dönüşüne göre işaretli) kırpılmasıyla bulunur. Ortak
   kenar örtüşme değildir; 1 mm²'nin altı ortak kenarın yuvarlamasıdır. Örtüşen bölge ikincinin sınırında reddedilir,
   ortak alanı söylenir.
4. **Cevap** (ek alanlar):
   - `zones`: her bölgenin adı, sınırı, alanı, iki yüzeyin örttüğü kısmı, kazı, dolgu, net, alanları ve katsayılı
     hacimleri (`adjusted`);
   - `rest`: bölgelerin dışı. Bütün farktan bölgelerinkinin çıkarılmasıdır (bölgeler örtüşmez); karşılaştırmanın
     katsayılarıyla;
   - `zoned`: malzemeye göre katsayılı toplam (bölgelerinki kendi katsayılarıyla, dışarısı karşılaştırmanınkiyle).

   Geometrik hacimler (`cut`, `fill`, `net`) ve karşılaştırmanın tek katsayılı `adjusted`'ı değişmez.
5. **Uyarı:** iki yüzeyin ortak alanının dışına taşan bölge `compare_uncovered` (bölgenin sınırında, taşan alanla).
6. **Retler** (sırayla): ızgarada bölge, bölgenin adı boş, katsayısı aralık dışı (`invalid_analysis`); olmayan sınır
   (`entity_not_found`); sınır olamayan nesne ve örtüşme (`invalid_analysis`). Hepsinin yolu bölgeyi gösterir
   (`zones[1].boundary`).
7. **Masaüstü** (Yüzey analizi › Fark):
   - **Malzeme bölgeleri** satırı (Hacim bölümünde, yalnız TIN yönteminde): bir katman seçilir (Yok seçilebilir).
     Katmanın kapalı nesneleri çizimin sırasıyla bölge olur;
   - ad nesnenin **Malzeme** özniteliği (yoksa katmanın adı), katsayılar **Kazı katsayısı** ve **Dolgu katsayısı**
     öznitelikleri (yoksa karşılaştırmanınki). Adında “malzeme” geçen katman kendiliğinden seçilir;
   - **Grafik** sekmesi: fark aralığı yokken her malzemenin ve bölge dışının katsayılı kazı ve dolgu çubukları;
   - **İçerik** sekmesi: Kazı, Dolgu, Net ve Malzemeye göre net göstergeleri; Özet | Malzemeler seçimiyle tek tablo
     (Malzeme, Alan, Kazı, Dolgu, Katsayı, katsayılı Net; Bölge dışı son satır).
8. **Pencerenin düzeni** (sahibin sözü: “Daha derli toplu zarif hale getir formu, grafik ve içerik ayrı sekmelerde
   olsun”): Yüzey analizi Yol pencerelerinin iskeletine taşındı (DESIGN.md §7.19, `RoadDialog`):
   - solda 300 px'lik bölümlü form: Kip; Yüzey; kipin bölümü (Aralıklar, Hacim, Denetim ve Kontrol noktaları, Sınır);
     Çizime yaz;
   - sağda Grafik | İçerik sekmeleri (`RoadDialog::over`). İçerik'te en çok dört gösterge ve seçimli tek tablo;
   - sonuç form değiştikçe okunur, Önizle düğmesi yoktur. Çizen işler alt çubuktadır, kipin son yazan işi birincil
     düğmedir;
   - pencerenin bütün kipleri (Analiz, Fark, Kalite, Kapsam, Yoğunluk) aynı düzendedir.
9. **Kapsam işareti:** ADR 1600'deki gibi **Harita + Yol** (CBS aynı hesabı tüketir).

## Kanıt

- **Çekirdek** (`volume_materials::tests`):
  - harita koordinatlarında eğik farkta bölgenin kazısı ve dolgusu prizmanınki (saat yönünde verilen sınır da);
  - delik ve yüzeylerin dışındaki kısım hacme girmez;
  - örtüşme yalnız ortak alanı sayar: ortak kenar 0, iç bükey L'nin köşesi 3 m², deliğin içindeki ada 0.
- **Bağımsız başvuru** (`scripts/fixtures/road_terrain_compare_zone_cases.py`, uv ile shapely; KentOS kodu yok):
  - her bölge iki kafesin dikdörtgenleriyle GEOS'ta kesişir. Parçanın integrali alanı çarpı ağırlık merkezindeki fark;
    örtüşme GEOS'un kesişim alanı;
  - ayrı durum dosyası `road.terrains.compare.zones.json`: 24 durum. İki malzeme kendi katsayılarıyla, katsayısız bölge,
    katsayısız her şey, delikli bölge, ortak kenarlı bölgeler, üç bölge, yalnız kazı katsayısı, bütün alanı kaplayan
    bölge, kapalı çoklu çizgi, ters yön, kimlikle ve aralıklarla, kırpılan ad; yarısı tabanın dışında, içbükey (kazı
    oluğunu keser), yüzeylerin dışında. Dokuz ret: örtüşme, aynı sınır, boş ad, iki katsayı, olmayan sınır, açık ve yaylı
    çoklu çizgi, ızgara;
  - eski 43 durum değişmeden geçer. Rust ve Python geçer.
- **Python** (`python/tests/test_compare_material_zones.py`):
  - üç malzemenin cevabı durumunki; bölgeler ve dışarısı bütün farkı verir;
  - örtüşen bölgeler reddedilir, çizim değişmez.
- **MCP** (`crates/native/mcp/tests/compare_material_zones.rs`): iki malzeme ve yarısı tabanın dışındaki bölge durumunki
  (bölgeler, dışarısı, toplam ve bölge uyarısı).
- **Masaüstü** (`material_zones_tests::fark_reads_the_material_zones_from_a_layer`):
  - “Malzeme bölgeleri” katmanı adından seçilir. Kaya, Toprak ve Kil özniteliklerinden okunur, cevap durumunki;
  - Yok seçilince bölge yok; ızgarada bölge gönderilmez.

  Yüzey analizinin öbür testleri, Yol iskeletini kullanan pencerelerin testleri ve çapraz denetimler geçer. Kazı ve
  dolgu bölgelerinin eski testi `.kcad` şemasını 205 bekliyordu. ADR 2840'tan beri çizim kaydı sürüm ve revizyon
  taşıdığından şema 209'dur; test buna göre düzeltildi.
- **Resimler:**
  - `.run/gui-evidence/malzeme-bolgeleri-3540/`: Fark'ın Grafik'i, İçerik'in Özet ve Malzemeler tablosu, açık geniş ve
    koyu dar;
  - öbür kipler yeni düzende: `yol-y18-yuzey-analizi-*`, `yol-y19-kapsam-*`, `yol-y19-yogunluk-*`, `yol-y39/`
    (Kalite'nin İçerik'i: Kontrol noktaları ve Sağlam ölçüler).

## Sonra

- Bölgelerin çizime yazılması (malzemeye göre taranmış alan ve yazı; kazı ve dolgu bölgeleriyle kesişimi).
- Kesit yöntemiyle hacim karşılaştırması (MAP-15'in öbür maddesi; stokta üçgenlenmiş taban [ADR 3940](3940-stockpile-triangulated-base.md)'ta).
- Malzemenin birim fiyatı ve maliyet: kaynak ve kullanıcının değeri beklenir.
- Web (TODOS §0.2).
