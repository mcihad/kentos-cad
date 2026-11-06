# 1600 — Yüzey farkında hacim yöntemi ve kazı/dolgu katsayıları (`road.terrains.compare`)

Tarih: 2026-11-07. plan/map-cad.md MAP-15, ikinci dilim. Alt maddeler:
- “TIN–TIN, raster veya kesit yöntemi seçildiyse yöntem, çözünürlük, eksik alan ve sayısal hata sınırı raporda
  bulunsun; farklı sonuçlar açıklamasız aynı ‘hacim’ başlığı altında sunulmasın”;
- “kabarma/sıkışma katsayısı … ayrı parametre; geometrik hacim ile iş/malzeme miktarı farklı hesaplar”.

Durum: taslak. İlgili ADR'ler:
- [ADR 0916](0916-road-surface-analysis-and-quality.md) §3: yüzey farkı;
- [ADR 0974](0974-road-volume-grid-method-and-step.md): ızgara hacmi, orta nokta kuralı;
- [ADR 1580](1580-map-project-grading-and-drainage.md).

## Dayanak

- **Civil 3D Volumes Dashboard:**
  - Kazı katsayısı (cut factor) genellikle 1'den büyüktür, kabarmayı anlatır. Dolgu katsayısı (fill factor) 1'den
    büyüktür, sıkışmayı anlatır. 1,00 düzeltme yapmaz.
  - Düzeltilmiş (Adjusted) ve düzeltilmemiş (Unadjusted) hacimler ayrı verilir (API'de `AdjustedCutVolume`,
    `UnadjustedCutVolume`).
  - Varsayılan katsayılar kullanıcının ayarıdır.

  Kaynaklar:
  - [Volumes Dashboard](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-7246FF86-2C5F-4B5A-991D-5571DAAC0CDF.htm);
  - [VolumeSurfaceProperties](https://help.autodesk.com/cloudhelp/2021/ENU/Civil3D-API/files/html/3ccfb05b-56ef-dcfe-d13b-1661a96ccd57.htm).

  Bir Autodesk forum başlığında Cut/Fill Summary ile Elevations Table toplamlarının ayrıştığı bildirilmiştir. Yöntem
  söylenmeden verilen iki hacim karıştırılır; plan bunu yasaklar.
- **Türk kaynağı:** Karayolları Teknik Şartnamesi (2013) sıkıştırmayı yüzde ile ister (%97–100). Hacim dönüşüm
  katsayısı (kabarma, oturma) için sayısal değer bulunamadı (arama 7 Kasım 2026). KentOS değer uydurmaz: katsayı
  kullanıcınındır, yoksa 1'dir.
- **KentOS bugün:**
  - `road.terrains.compare` TIN–TIN prizmalarını kesin hesaplıyor (iki yüzeyin üçgenleri üst üste);
  - ızgara yöntemi yol hacimlerinde var (`Ground::grid_volumes`, ADR 0974);
  - karşılaştırmada yöntem ve katsayı yoktu.

## Karar

1. **`method`:**
   - `tin` (yoksa): kesin prizmalardır.
   - `grid` ve `cell` (m): ızgaradır.
     - Hücre köşeleri `cell`'in katlarındadır; her hücre merkezi karşılaştırmanın planında bir kez ölçülür (orta nokta
       kuralı, ADR 0974'ün çekirdeği).
     - Ana hacimler (kazı, dolgu, net, alanlar, tabanın dışı) ızgaranındır. Yanında kesin TIN–TIN hacmi `exact`
       olarak verilir: yöntem farkı ikisinin farkıdır, açıkça.
     - Uçlar (en küçük ve en büyük fark) her zaman kesin TIN'indir.
     - Fark aralıkları yalnız TIN yöntemindedir; ızgarada istenirse reddedilir.
     - En çok 4 000 000 hücre ölçülür; aşan hücre reddedilir, neden söylenir.
2. **`cutFactor`, `fillFactor`:**
   - 0'dan büyük, en çok 10; yoksa 1.
   - Verilirse `adjusted` hesaplanır: kazı × kazı katsayısı, dolgu × dolgu katsayısı, net. Geometrik hacimler
     değişmez.
3. **Hata sınırı:** ızgaranın sayısal hata sınırı için bir formül uydurulmaz. Ölçülen yöntem farkı (kesin TIN'e göre)
   raporlanır. TIN–TIN kesindir; kayan nokta dışında hatası yoktur.
4. **Masaüstü** (Yüzey analizi › Fark):
   - Yöntem seçimi (TIN–TIN | Izgara);
   - ızgarada hücre alanı, aralıklar yerine;
   - Kazı katsayısı ve Dolgu katsayısı alanları;
   - sağdaki özet: Yöntem, ana hacimler, Kesin (TIN), Yöntem farkı, Katsayılı.

   Harita ve Yol'da.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/road_terrain_compare_cases.py` 15 yeni durum ekler, toplam 37.
  - Bağımsız ızgara, kafeslerin kapalı biçimli yüksekliğiyle (z = g(x) + h(y)) her hücre merkezinde hesaplanır.
  - Kaba hücrede yöntem farkı görünür: 2 m'de kazı 29,000 m³, kesinde 28,125 m³. Kafese denk gelmeyen 0,3 m ve
    0,7 m hücreler de sınanır.
  - Katsayılar tek başına ve ızgarayla birlikte sınanır.
  - Retler: hücresiz ızgara, sıfır hücre, 4 milyonu aşan hücre, ızgarada aralık, sıfır ve 10'u aşan katsayı.

  Rust ve Python geçer.
- **Masaüstü** (`surface_analysis_window_tests`): ızgara ile kesin TIN'in yan yana verildiği ve katsayıların
  uygulandığı sınanır. Resimler `.run/gui-evidence/hacim-yontemi-1600/` altındadır.

## Sonra

- Kesit (ortalama alan) yönteminin yüzey farkına bağlanması.
- Malzeme bölgeleri ve stok sahası.
- Kazı/dolgu sınırının ve sıfır hattının çizime yazılması.
- CBS'nin tematik fark haritası.
