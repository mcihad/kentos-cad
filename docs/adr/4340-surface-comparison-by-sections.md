# 4340 — Yüzey karşılaştırmasında kesit yöntemi

Tarih: 2026-11-10. plan/map-cad.md MAP-15, sekizinci tur. Durum: taslak. ADR 3940'ın “Sonra”sındaki “kesit yöntemiyle
hacim karşılaştırması”; planın maddesi: “TIN–TIN, raster veya kesit yöntemi seçildiyse yöntem, çözünürlük, eksik alan
ve sayısal hata sınırı raporda bulunsun; farklı sonuçlar açıklamasız aynı ‘hacim’ başlığı altında sunulmasın.”

İlgili:
- [ADR 1600](1600-surface-volume-method-and-factors.md): karşılaştırmanın yöntemi (TIN–TIN, ızgara), kesin TIN yanında;
- [ADR 0906](0906-road-earthwork-volumes.md): yolun kesitlerden hacmi (ortalama alan, prizmoidal: `kentos_road::volume`);
- [ADR 3540](3540-material-zones-in-a-surface-comparison.md): malzeme bölgeleri, Yüzey analizi'nin düzeni.

## Dayanak

- **Autodesk, Civil 3D 2024 kılavuzu, About Analyzing Sectional Volumes**
  ([belge](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-1E3A9CD9-DFD6-4455-9EB1-D8E46721A0B4.htm),
  10 Kasım 2026'da okundu): örnek çizgi takımının hacmi üç yöntemle hesaplanır:
  - **Average End Area:** bir malzemenin bir kesitteki alanı sonraki kesitteki alanıyla toplanır, ikiye bölünür,
    kesitler arası uzaklıkla (L) çarpılır;
  - **Prismoidal:** ortalama alana benzer, iki ardışık kesitin ortasında ek bir kesit kullanır;
  - **Composite:** kesitler arasında çokgenler kurar, sınırlı hacimlerini yüzeylerin verisinden hesaplar (iki
    yüzeyle sınırlı).
- **KentOS bugün:**
  - Yol modülü kesitlerden hacmi ortalama alan ve prizmoidal formülle zaten hesaplar (`kentos.end-area/1`,
    `kentos.prismoidal/1`; ADR 0906), ama bir aksın kesit takımı ve koridorla.
  - Harita projesindeki iki yüzeyin karşılaştırması (`road.terrains.compare`) yalnız kesin TIN–TIN ve ızgarayla
    ölçüyordu.
  - Bileşik yöntem KentOS'un kesin TIN–TIN'idir: iki yüzeyin üçgenleri üst üste kırpılır (ADR 0916 §3).
- **Karar için:** prizmoidal formül L/6·(A₀ + 4·Aₘ + A₁) yolun çekirdeğindeki tanımdır (Simpson); ortadaki kesitin
  alanı ortadaki gerçek kesitten okunur, iki uç kesitin ortalamasından türetilmez.

## Karar

1. **`road.terrains.compare`'e `method: sections`** ve:
   - `interval` (m): kesitlerin aralığı, zorunlu;
   - `azimuth` (derece): eksenin semti, kuzeyden saat yönünde; boş 0 (eksen kuzeye, kesitler doğu–batı);
   - `formula`: `endArea` (boş) ya da `prismoidal`.
2. **Kesitler:**
   - eksene diktir;
   - ilki karşılaştırma yüzeyinin eksen boyunca ilk yerinde, sonra her `interval`'da, sonuncusu son yerinde;
   - en çok 10 000 kesit.
3. **Kesitin alanları:**
   - kesit iki yüzeyin üçgenlerini keser, her üçgende yüzey düzlemdir;
   - iki yüzeyin parçalarının ortak yerinde fark (karşılaştırma − taban) doğrusaldır, kesin tümlenir;
   - fark sıfırında bölünür: dolgu d > 0, kazı d < 0 (m²);
   - kesit bir kenar boyunca giderse iki üçgen aynı parçayı verir, bir kez sayılır;
   - tabanın dışındaki kısım sayılmaz (`compare_uncovered` uyarısı TIN'inki gibi).
4. **Hacim:** ardışık iki kesit arasında ortalama alan L·(A₀ + A₁)/2. Prizmoidal formülde ortadaki kesit ölçülür,
   L/6·(A₀ + 4·Aₘ + A₁) (yolun `volume::prismoidal`'ı). Kazı ve dolgu ayrı toplanır.
5. **Cevap:**
   - kazı, dolgu ve net kesitlerindir;
   - kesin TIN–TIN `exact`'te yanında: yöntem farkı ikisinin farkıdır, açıkça;
   - plan alanları, uçlar ve kapsam kesin TIN'indir;
   - `interval`, `azimuth`, `formula` ve `sections` (uzaklık, kazı alanı, dolgu alanı);
   - katsayılar kesitlerin hacmine uygulanır.
6. **Retler** (`invalid_analysis`, sırayla):
   - kesitte aralık yok; aralık 0'dan büyük değil;
   - semt [0, 360) dışında;
   - kesitte fark aralığı ya da malzeme bölgesi (ikisi de kesin TIN'indir);
   - kesit yöntemi olmadan formül;
   - 10 000'den çok kesit (yüzeyler kurulunca).
7. **Hata sınırı:** kesit yönteminin hata sınırı için bir formül uydurulmaz. Kesin TIN'e göre ölçülen yöntem farkı
   raporlanır (ADR 1600 §3'ün kuralı). Kesit alanları kesindir; yöntemin farkı kesitler arasındaki varsayımdandır.
8. **Masaüstü** (Yüzey analizi › Fark):
   - Yöntem: TIN–TIN | Izgara | **Kesit**;
   - Kesit'te **Kesit aralığı (m)** (boş 10), **Eksenin semti (°)**, **Formül** (Ortalama alan | Prizmoidal);
   - Grafik'te kesit alanları eksen boyunca: dolgu yukarı, kazı aşağı;
   - İçerik'te özet (Yöntem satırı, Kesin (TIN), Yöntem farkı) ve **Kesitler** tablosu (sıra, uzaklık, kazı ve
     dolgu alanı).
9. **Kapsam işareti:** ADR 1600'deki gibi **Harita + Yol** (CBS aynı hesabı tüketir).

## Sınırlar

- Kesitler paraleldir: kıvrımlı bir aks boyunca kesit (yolun kesit takımı) burada yok; yolun hacmi onu yapar.
- Bileşik yöntem ayrı bir seçenek değildir; kesin TIN–TIN onun yerini tutar.
- Kesitlerin çizime yazılması yok.

## Kanıt

- **Çekirdek** (`kentos_road::volume_sections::tests`, analitik):
  - dikdörtgende düzlem fark: dört yönde ortalama alan kesin 2400 m³;
  - üçgen planda (alan karesel): prizmoidal kesin 1000 m³, ortalama alan 975 m³ (L³/12·A″ farkı);
  - sıfırda bölünme; retler.
- **Bağımsız başvuru** (`scripts/fixtures/road_terrain_compare_section_cases.py`; KentOS kodu yok), 27 durum
  (`fixtures/commands/v1/road.terrains.compare.sections.json`):
  - kafes yüzeyler g(x) + h(y), her kare bir düzlem: kesit kafes çizgilerinde kırılır, aralarında fark doğrusal, kesin
    integral;
  - durumlar: dört semt, kafese denk gelmeyen aralık, prizmoidal, eğik eksen (30°, 120°), kazı ve dolgu aynı kesitte,
    ters yön, katsayılar, plan, doğrulama, Harita projesi ve dokuz ret;
  - kesin TIN `road_terrain_compare_cases.py`'nin.

  Rust ve Python geçer; eski 67 karşılaştırma durumu değişmedi.
- **Python** (`python/tests/test_compare_sections.py`), testin kendi kapalı formülleriyle: üçgen planlı düzlem farkta
  kuzey–güney ortalama alan 975 (her kesit A(x) = 30(1 − x/40)(1 + 0,05x)), doğu–batı 1022,2…, prizmoidal 1000; semt
  360 reddi.
- **MCP** (`crates/native/mcp/tests/compare_sections.rs`): üç durumun hacimleri, kesin TIN'i ve kesitleri; semt reddi.
  MCP'nin ana testleri geçer.
- **Masaüstü** (`compare_sections_tests`): pencerenin cevabı durumunki (kesitler, hacimler, prizmoidal); boş aralık
  10 m; semt 400 söylenir. Yüzey analizi, malzeme bölgeleri testleri ve çapraz denetimler geçer. Resimler
  `.run/gui-evidence/kesit-yontemi-4340/` (Grafik, İçerik › Özet, İçerik › Kesitler).

## Sonra

- Kesitlerin çizime yazılması (kesit çizgileri ve alan yazıları); bir aks boyunca kesit.
- Malzeme bölgelerinin çizime yazılması (MAP-15'in öbür maddesi).
- Web (TODOS §0.2).
