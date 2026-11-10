# 4560 — GNSS ağının seyrek çözümü ve büyük ağ ölçümü

Tarih: 2026-11-10. plan/map-cad.md MAP-05, dokuzuncu tur. Durum: taslak. ADR 1380'in “Bütçe”sindeki “Seyrek çözüm,
bölümlü büyük ağ ve seçili kovaryans MAP-05'in kalanıdır; ölçüm raporu olmadan ölçeklenebilirlik iddia edilmez” ve ADR
4160'ın “Sonra”sındaki “seyrek çözüm”.

İlgili:
- [ADR 0203](0203-network-adjustment.md) §4: ortak istatistik çekirdeği ve pivot kuralı (köşegenin 10⁻¹⁰'undan küçük
  pivot datum eksikliğidir, bilinmeyeniyle adlandırılır);
- [ADR 1380](1380-gnss-baseline-network-adjustment.md): GNSS baz ağı dengelemesi; yoğun normal denklem, en çok 1500
  bilinmeyen (500 istasyon);
- [ADR 1740](1740-gnss-network-from-archive.md): arşivden ağ (`map.survey.network`), aynı çekirdek.

## Dayanak

- **Hodson ve Leslie, “Integrating Least Squares Analysis with GIS for Cadastral Data Quality Enhancements”**, FIG e-Working
  Week 2021, TS07.3 ([fig.net](https://www.fig.net/resources/proceedings/fig_proceedings/fig2021/papers/ts07.3/TS07.3_hodson_leslie_10902.pdf);
  PDF'in metni çıkarılarak 10 Kasım 2026'da okundu), §2.1 “Network Segmentation”:
  - en küçük karelerin güçlüğü çok büyük ağlardır; on binlerce ölçülü ağ saatler, günler sürebilir;
  - nedeni normal denklem matrisinin tersini hesaplamanın yüküdür;
  - DynAdjust ağı otomatik olarak küçük bloklara bölen aşamalı (phased) dengelemeyle çalışır; bölme işlemciye ve
    belleğe göre ayarlanır.
- **ArcGIS Pro**, [The DynAdjust least-squares adjustment engine](https://doc.esri.com/en/arcgis-pro/latest/help/data/parcel-editing/thedynadjustengine.html)
  (10 Kasım 2026'da okundu): büyük ağlar ardışık bloklarda dengelenir; motor küçük mühendislik ölçmelerinden ulusal
  jeodezik ağlara ölçeklenir. Sayfa yöntemin ayrıntısını vermez.
- **SciPy**, [`scipy.sparse.csgraph.reverse_cuthill_mckee`](https://docs.scipy.org/doc/scipy/reference/generated/scipy.sparse.csgraph.reverse_cuthill_mckee.html)
  (10 Kasım 2026'da okundu): seyrek bir matrisin satır ve sütunlarını ters Cuthill–McKee sırasına koyan permütasyon;
  kaynak Cuthill ve McKee, “Reducing the Bandwidth of Sparse Symmetric Matrices”, ACM '69.
- **R sparseinv**, [`Takahashi_Davis`](https://search.r-project.org/CRAN/refmans/sparseinv/html/Takahashi_Davis.html)
  (10 Kasım 2026'da okundu): seyrek matrisin tersinin alt kümesi Takahashi denklemleriyle hesaplanır; dönen girdiler
  permütasyonlu matrisin Cholesky çarpanının sıfır olmayan girdileridir; kaynak Takahashi, Fagan ve Chin (1973),
  “Formation of a sparse bus impedance matrix and its application to short circuit study”, 8th PICA, ve Davis (2006).
- **KentOS bugün** (ADR 1380): normal denklem yoğundu; tam ters O(u³) hesaplanıyordu ve 1500 bilinmeyenin üstü
  `network_too_large` ile reddediliyordu. Ölçüm yoktu.

## Karar

1. **Seyrek çözüm** (`survey::adjust::sparse`, `gnss::adjust`):
   - **Sıra:** sabit olmayan istasyonlar, bazların kurduğu çizgede ters Cuthill–McKee sırasına konur: her parça kendi
     en küçük numaralı düğümünden; George ve Liu'nun aramasıyla sözde çevresel bir kökten; komşular artan dereceyle
     (eşitlikte numarayla); bütün sıra ters çevrilir. Her istasyonun üç bilinmeyeni yan yanadır.
   - **Zarf:** normal matris satır satır, ilk sıfır olmayan sütunundan köşegene kadar tutulur (istasyonun en erken
     komşusunun ilk bilinmeyeni). Cholesky çarpanı zarfın dışına dolmaz; yerinde hesaplanır; ADR 0203'ün pivot kuralı
     aynıdır.
   - **Seçili ters:** tam ters kurulmaz. Zarfın içindeki girdileri Takahashi denklemleriyle son sütundan geriye
     hesaplanır. İstatistiğin okuduğu her girdi (bir gözlem iki istasyonu bağlar), istasyonun 3×3 kovaryansı ve
     dengelenmiş bazın kovaryansı zarfın içindedir.
   - Sonuç yoğun çözümle aynıdır: bütün ortak durumlar ve bağımsız başvurular değişmeden geçer.
2. **“Bağlı değil” adı korunur:** ağ datuma ulaşmazsa, yoğun çarpanın girdi sırasında adlandıracağı istasyon
   adlandırılır. Bazlarla birleşen parçalardan sabit ya da ağırlıklı istasyonu olmayanın son istasyonudur; birden çok
   parça yüzerse son istasyonu en önce geleninki. Böyle bir parça yoksa (sayısal tekillik) pivotun düştüğü istasyon
   adlandırılır.
3. **Bütçe:** zarf en çok **30 milyon** girdi tutar (`MAX_ENVELOPE`). Çarpan ve ters 8'er bayt, sütunların satır
   numaraları 4 bayttır; girdi başına 20 bayt, sınırda yaklaşık 600 MB eder. Aşan ağ `network_too_large` ile reddedilir:
   “Ağın normal denklemi seyrek çözümde N girdi tutuyor (M istasyon); en çok 30000000 girdi alınır. Ağı bölgelere ayırıp
   ayrı dengeleyin.” 1500 bilinmeyen sınırı GNSS ağında kalkar; düzlem ve nivelman ağları yoğun kalır.
4. **`map.survey.network`:** işaretlerin, bilinen noktaların ve noktaların adla aranması eşlem tablosuyla yapılır;
   büyük ağda doğrusal kalır (sonuç aynı).
5. **Masaüstü** (GNSS baz ağı): özette ağın büyüklüğü: “N istasyon, B baz: u bilinmeyen, n gözlem denklemi.”
6. **Kapsam işareti:** ADR 1380 gibi **Harita + CBS**.

## Ölçüm

Makine: Apple M5 (Mac17,2, 32 GB; macOS 26.6.2), commit e473ea5be üstündeki değişiklik, `--release`; ızgara ağ (istasyonlar 1 km
arayla, doğu, kuzey ve kuzeydoğu komşularına bazla bağlı; ilk istasyon sabit, her onuncusu 2 cm ağırlıklı;
`gnss::tests::large_grids`). Süre `gnss::adjust`'ın bütünüdür (sıra, kurma, çarpan, çözüm, seçili ters, istatistik).

| Izgara | İstasyon | Bilinmeyen | Zarf girdisi | Süre |
|---|---:|---:|---:|---:|
| 50 × 50 | 2 500 | 7 497 | 774 828 | 0,13 s |
| 100 × 100 | 10 000 | 29 997 | 6 102 153 | 1,51 s |
| 140 × 140 | 19 600 | 58 797 | 16 665 813 | 6,0 s |
| 180 × 180 | 32 400 | 97 197 | 35 327 073 | reddedilir (0,01 s) |

- Bütçe öncesi aynı koşuda 180 × 180 ızgara 16,4 s'de çözülmüştü; sınır bu ölçümle süreyi birkaç saniyeye ve belleği
  1 GB'ın altına tutacak biçimde seçildi.
- 140 × 140'ı da içeren koşunun en yüksek bellek kullanımı (`/usr/bin/time -l`) 735 MB'dı. Çarpan o sırada kopyayla
  hesaplanıyordu; sonra yerinde hesaplanır.
- Gerçek ağların zarfı ızgarayınkinden farklıdır; ölçüm bir örnektir, her ağın süresi değildir.

## Sınırlar

- Bölümlü (aşamalı) dengeleme yoktur: sınırı aşan ağ tek seferde çözülmez.
- Düzlem (`horizontal`) ve nivelman ağları yoğun ve 1500 bilinmeyenle sınırlı kalır. Pivotun düştüğü bilinmeyenin adı
  orada geometriye de bağlıdır; ayrı iş.
- Sıra ters Cuthill–McKee'dir; en küçük derece gibi dolguyu daha çok azaltan sıralar denenmedi.
- Web'de `cad.survey.baselines` işleyicisi yoktur (TODOS §0.2).

## Kanıt

- **Çekirdek** (`survey::adjust::sparse::tests`, `gnss::tests`):
  - sıra bir permütasyondur; halkanın bant genişliği 39'dan 7'nin altına iner; parçalı ve yalnız düğümlü çizge
    sıralanır;
  - zarfın çözümü ve seçili tersi yoğunla 10⁻¹² içinde aynıdır; yüzen çift pivotta düşer;
  - 14 × 14 ızgara ağ (585 bilinmeyen) yoğun normal denklemle aynı: koordinatlar 10⁻⁹ m, sapmalar 10⁻¹² m içinde;
  - iki yüzen parçada yoğun çarpanın adlandıracağı istasyon adlandırılır.
- **Ortak durumlar:** `cad.survey.baselines` ve `map.survey.*` durumlarının hepsi (numpy'nin yoğun en küçük kareleriyle
  bağımsız; “bağlı değil” adları dahil) değişmeden geçer, Rust ve Python. Bozma denemesi: Takahashi köşegeninde işaret
  değişince koşucu yakalar.
- **Python** (`python/tests/test_large_gnss_network.py`), testin kendi çözümüyle: 26 × 26 ızgara (676 istasyon, 2 025
  bilinmeyen; eski sınırın üstü), numpy'nin bütün ağı kapsayan ağırlıklı en küçük kareleri: koordinatlar 1 µm,
  sapmalar 10⁻⁹ m içinde; n, u, f, vᵀPv aynı.
- **MCP** (`crates/native/mcp/tests/large_gnss_network.rs`): aynı ağ `cad.survey.baselines`'la dengelenir (u 2 025,
  χ² geçer); iki kez adlandırılan istasyon yine reddedilir.
- **Masaüstü** (`baselines_large_tests`): ağ dosyası GNSS baz ağı penceresinde okunur; 676 istasyon, 2 025 bilinmeyen,
  χ² geçer; özet ağın büyüklüğünü söyler. GNSS testleri ve çapraz denetimler geçer. Resimler
  `.run/gui-evidence/seyrek-cozum-4560/`.

## Sonra

- Bölümlü (aşamalı) dengeleme; düzlem ve nivelman ağlarının seyrek çözümü.
- Robust kestirim; ağırlıklı benzerlik (serbest aşamanın kovaryansı tutulan noktaya bağlıdır: S-dönüşümü ister).
- Web (TODOS §0.2).
