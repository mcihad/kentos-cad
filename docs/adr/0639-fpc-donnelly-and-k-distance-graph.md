# ADR 0639: Kararlılığın fpc'yle aynılığı, Donnelly'nin kenar düzeltmesi ve k-uzaklık grafiği

- **Durum:** kabul edildi (2026-10-25). `plan/gis.md` CBS-25'in kalanı: R'ın fpc'siyle çapraz denetim, Donnelly'nin kenar
  düzeltmesi, k-uzaklık grafiği. Kapsamı ben belirledim; R'ı `.run/r`'ye kurmak sahibin onayıyla (25 Ekim).
- **Bağlam belgesi:** ADR 0238 (mekânsal istatistik), 0520 (permütasyon ve deney profili), 0566 (En yakın komşu'nun
  benzetimi, kümelerin kararlılığı), 0574 (çalışma alanı, DBSCAN taraması), 0227 (Grafik sekmesi, İşlemler'in tabloları).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **`.kcad`:** şema değişmez.
- **Sonraya kalan:** Donnelly'nin düzeltmesi dikdörtgen bölgeler için türetildi. Karmaşık çalışma alanında yaklaşıktır ve
  söylenir; orada Monte Carlo sınaması asıl sınamadır.

## Karar

### 1. Kararlılık fpc'nin kuralıyla

Hennig'in kararlılığı (ADR 0566 §2) iki yerde fpc'nin `clusterboot`'undan ayrılıyordu. İkisi de fpc'ye uyduruldu:

- **Bootstrap örneği her nesneyi bir kez alır.** fpc'nin varsayılanı `multipleboot = FALSE`'tur: yerine koyarak çekilen n
  sıranın yinelenenleri atılır (`unique(bsamp)`). Önceden yinelenen nesneler kümelemeye birden çok kez giriyordu, bu da
  DBSCAN'da yoğunluğu şişiriyordu. Şimdi `resample` sıralı ve teksiz kimlikler verir.
- **Örnekte hiç nesnesi olmayan küme sayılır.** fpc her tekrarın γ'sını 0 ile başlatır ve `bootmean` bütün tekrarların
  ortalamasıdır. Önceden kümesi örnekte olmayan tekrar sayılmıyordu. Şimdi o tekrar γ = 0'dır ve “Sayılan tekrar” her
  zaman tekrar sayısıdır.

Ortak durumlar yeniden yazıldı (`spatial-inference.json`'un kararlılık durumları), değerler biraz değişti (DBSCAN'da
yarıçap 12 m, en az 4 nokta: 0,88 / 0,86 / 0,91'den 0,78 / 0,73 / 0,81'e).

**Çapraz denetim (`spatial_r_cases.py`, `spatial_r.R`):** R'ın fpc 2.2.15'i aynı yeniden örneklerle çalışır. Betik
`clusterboot`'un bir kopyasının `sample`'ını KentOS'un kuralının çektiği sıralarla değiştirir. Kümelemeyi R kendisi yapar:

- DBSCAN'ı `dbscan` paketiyle (standart DBSCAN; fpc'nin kendi `dbscan`'ı bir türevdir);
- k-ortalamaları `stats::kmeans`'la (Lloyd, tek başlangıç), ADR 0238 §10'un en uzak ilk tohumlarından; tohumlar R'da
  ADR'den yeniden yazıldı.

Yirmi çalıştırmada (DBSCAN on iki, k-ortalamalar sekiz; bootstrap ve alt örnek; sınır noktaları gürültü de) kümelerin
bölüntüsü, her tekrarın her kümedeki γ'sı (8 400 değer, 10⁻¹² içinde), ortalamalar, yeniden bulunma ve çözülme sayıları
aynıdır.

fpc'de bir hata bulundu: gürültü kümesinin γ'sı, kümesi olmayan tekrarda bir önceki tekrarın `ncases`'iyle okunuyor.
Karşılaştırılan değerleri etkilemez; betik kümesiz tekrarda gürültü bildirmez.

### 2. Donnelly'nin kenar düzeltmesi

En yakın komşu'ya **Kenar düzeltmesi** seçeneği eklendi: `edge`, değerleri Yok ve Donnelly; varsayılan Yok, sonuçlar
önceki gibidir. Donnelly'de (1978) n nesne, A alan ve P bölgenin çevresiyle:

- beklenen ortalama uzaklık `E = 0.5·√(A/n) + (0.0514 + 0.041/√n)·P/n`;
- standart hata `√(0.0703·A/n² + 0.037·P·√(A/n⁵))`.

Oran, z ve p bunlarla hesaplanır. P bölgenin çevresidir:

- yerlerin kutusunun çevresi;
- Alan yazılmışsa o alana ölçeklenmiş kutunun çevresi;
- çalışma alanında birleşimin bütün halkaları, delikler de (`geom::space::perimeter`).

Tabloya “Çevre (m)” ve “Kenar düzeltmesi: Donnelly (1978)” satırları girer. Özet “Donnelly kenar düzeltmesiyle z …”
der, bir bilgi satırı formülü yazar. 7'den az nesnede (Donnelly'nin benzetimlerinin altı) uyarı verilir. Monte Carlo
sınaması değişmez; yalnız deney profiline “Kenar düzeltmesi” eklenir.

Kazaların kutusunda düzeltmesiz z −2,35, Donnelly'nin z'si −2,71, 999 rastgele desenin z'si −2,77'dir. Düzeltme,
benzetimin yakaladığı kenar etkisini formülle verir.

**Katsayı:** spatstat (`clarkevans`) 0.041 yerine 0.0412 yazar ve Donnelly'nin varyansı yerine Clark ve Evans'ınkini
kullanır. KentOS, Donnelly'nin kendi formülünü Dixon'un notlarının verdiği gibi alır (0.041, 0.0703, 0.037).

**Çapraz denetim:** spatstat 3.8-2'yle (`spatial_r_cases.py`), ortak durumların dört penceresinde:

- gözlenen ortalama uzaklık, alan ve çevre aynıdır;
- spatstat'ın beklenen değeri, aynı formül onun katsayısıyla yazılınca 10⁻⁹ içinde aynıdır;
- iki katsayının oranları en çok %0,027 ayrılır.

### 3. k-uzaklık grafiği

DBSCAN parametre taraması önerdiği en az nokta için sıralı k-uzaklıkları ikinci tablo olarak verir (`chart`, “k-uzaklık
grafiği”). Sütunlar Sıra, “(m − 1). en yakın uzaklık (m)” ve Dirsek'tir; dirsek “◀ dirsek” ile işaretlidir.

İşlemler penceresinin Grafik'i bir aracın `chart` çıktısını ilk tablosundan önce alır (`ToolDialog.chart`). Grafik
sekmesi tarama için çizgi kurar (`specs::for_table`'ın `stats.dbscanSweep`'i: x Sıra, y uzaklık).

**Çapraz denetim:** scikit-learn'ün `NearestNeighbors`'ı (`spatial_esda_cases.py`) satır satır aynı uzaklıkları ve aynı
dirseği verir.

## Doğrulama

- **Ortak durumlar:** `fixtures/processing/v1/spatial-inference.json` 44 durum; yeni olanlar:
  - Donnelly'de kutu, alanla ölçeklenmiş kutu, benzetimle, 5 nesne (uyarı), 16 nesnelik alt küme;
  - Donnelly'de çalışma alanı, tek nesneli çalışma alanı ve çalışma alanında benzetim;
  - dört taramanın k-uzaklık grafiği.

  Hepsi KentOS kodu olmadan `spatial_inference_cases.py`'den (kesirler ve 60 basamaklı kökler; çevre, birleşimin hücre
  kenarlarından kesin). `spatial_stability_cases.py` ve `spatial_study_cases.py` Donnelly'yi ve fpc'nin kuralını taşır;
  öz denetimleri Donnelly'nin beklentisini 200 benzetimle karşılaştırır.
- **R:** `python3 scripts/fixtures/spatial_r_cases.py --check`. R, fpc, dbscan, spatstat.explore ve jsonlite
  `.run/r`'de (micromamba, conda-forge); R yoksa betik söyler ve geçer.
- **PySAL ve scikit-learn:** `spatial_esda_cases.py` 23 durum; k-uzaklık grafiğinin satırları da.
- **Testler:** `kentos-processing`'in ortak durumları, `kentos-geometry-core`'un kararlılık birim testi (bootstrap teksiz)
  ve `charts.rs`'in tarama grafiği.
- **Resimler:** `arac-ist-donnelly`, `arac-ist-donnelly-alan`, `arac-ist-tarama-grafik`; 1440×900 ve 1100×650, iki tema
  (`stats_scenes.rs`).
- **Süreler** (bu makinede, release, `spatial_stability_timing` ve `spatial_study_timing`):

  | İş | Süre | Bütçe |
  |---|---|---|
  | DBSCAN, 10 000 nokta, 100 bootstrap tekrarı | 0,27 s | 3 s |
  | k-ortalamalar, 10 000 nokta, k = 10, 100 bootstrap tekrarı | 0,35 s | 3 s |
  | DBSCAN taraması, 42 hücre (k-uzaklık grafiğiyle) | 0,40 s | 1 s |
  | DBSCAN taraması, 42 hücre, 10 bootstrap | 2,47 s | 6 s |

  Donnelly'nin düzeltmesi yalnız iki formül ve bir çevredir; süreye katkısı ölçülemeyecek kadar küçüktür.
