# ADR 0574: Çalışma alanının bir alan nesnesinden verilmesi ve DBSCAN'ın parametre taraması

- **Durum:** kabul edildi (2026-10-21). `plan/gis.md` CBS-25'in kalanı. ADR 0566'nın “sonraya kalan” listesinden iki iş:
  - çalışma alanının bir alan nesnesinden verilmesi;
  - DBSCAN'ın parametre taraması (k-uzaklık grafiği).

  Üçüncü iş, R'ın `fpc` paketiyle çapraz denetim, bu makinede R olmadığı için yapılmadı (§6). Kapsamı sahibin sözleriyle ben belirledim.
- **Bağlam belgesi:**
  - ADR 0238 §6 (En yakın komşu), §10 (DBSCAN);
  - ADR 0566 (benzetim, kararlılık);
  - ADR 0520 (kural `kentos.spatial-inference/1`, deney profili);
  - ADR 0220 (tohumlu sayılar, alanda rastgele nokta);
  - ADR 0201 (alanların birleşimi ve ölçüsü).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - Donnelly'nin kenar düzeltmesi (alanın çevresi artık bilinir ama düzeltmenin bağımsız başvurusu `spatstat`'ınki olurdu, R yok);
  - k-uzaklık eğrisinin grafiği (bugün yalnız dirseğin yarıçapı yazılır);
  - R'ın `fpc::dbscan` ve `fpc::clusterboot`'uyla çapraz denetim.

## Bağlam

En yakın komşu (ADR 0238 §6) beklenen uzaklığı 0,5 / √(n / A) ile bulur. A bugün yerlerin kutusunun alanıdır ya da yazılan Alan. İkisi de
gerçek çalışma alanını bilmez: ortası boş bir parkın, nehrin ya da denizin kıyısı olan bir mahallenin alanı kutudan çok küçüktür, oran
dağınıklığa kayar. ADR 0566'nın Monte Carlo sınaması aynı kutuya nokta atar; kutuda parkın ya da denizin içine de nokta düşer.

Mekânsal istatistiğin on aracından (ADR 0238) bir bölgeyi kullanan yalnız En yakın komşu'dur: Moran I, Gi\* ve Yerel Moran komşuluktan,
merkezler ve elips yerlerin kendisinden, DBSCAN yarıçaptan, k-ortalamalar k'dan hesaplanır; hiçbiri kutuya bakmaz (`ops::spatial_stats`'te
kutu yalnız `nearest.rs`'tedir). Bu ADR'nin birinci bölümü bu yüzden yalnız En yakın komşu'yu değiştirir.

DBSCAN'ın iki parametresi, yarıçap (ε) ve en az nokta, sonucu belirler ve doğrusu veriden bilinmez. Ester ve arkadaşları (1996) ε'u
sıralı k-uzaklık grafiğinin dirseğinden seçmeyi önerir; Schubert ve arkadaşları (2017, “DBSCAN Revisited, Revisited”) en az nokta için
boyutun iki katını (iki boyutta 4), gürültülü veride daha büyüğünü önerir; dirsek için Satopää ve arkadaşlarının (2011) Kneedle'ı
“köşegenden en uzak nokta” fikrini kullanır. Bugün kullanıcı iki sayıyı elle dener.

## Karar

### 1. Çalışma alanı (En yakın komşu)

- **Parametreler:**
  - “Çalışma alanı” (`region`): Kutu (varsayılan; yerlerin kutusu ya da yazılan Alan) ya da Alan nesneleri.
  - “Çalışma alanı nesneleri” (`studyArea`), nesne girdisi, yalnız Alan nesneleri seçilince görünür. Türleri alan nesneleridir:
    delikli ve çok parçalı alan (`parts`); daire, tam elips, kapalı eğri ve NURBS (`class_of`'un alanları). Seçili, görünen,
    katman ya da kimliklerle verilebilir. Alan seçilmemişse (ya da seçilen nesnelerde alan yoksa) bugünkü kural gibi, uygun nesne yok
    iletisiyle reddedilir.
- **Bölge:** verilen bütün nesnelerin bütün alanlarının birleşimidir (ADR 0201'in `union_all`'ı, sıfırdan farklı sarım). Parçalar ayrı
  kalır, delikler bölgeden çıkar, üst üste binen alanlar bir kez sayılır. Alan olmayan nesneler alınmaz ve söylenir. Hiç alan yoksa ret.
- **A:** bölgenin net alanı (ADR 0201'in `areas_measure`'ı): dış halkaların alanı eksi deliklerinin. Beklenen uzaklık, standart hata ve z
  bu A ile bugünkü formüllerle bulunur.
- **Gözlenen yerler:** yerleri bölgenin içinde ya da sınırında (1 µm, ADR 0220 §3'ün kuralı) olan nesneler alınır. Dışında kalanlar
  alınmaz ve sayılarıyla söylenir: n / A, yalnız bölgenin içindekilerin yoğunluğudur. Delikte kalan nesne dışarıdadır.
- **Alan (m²) ile:** Alan nesneleri seçiliyken yazılan Alan yok sayılır (pencerede gizlenir; komutla verilirse de etkisizdir).
- **Monte Carlo (ADR 0566 §1) bölgede:**
  - Noktalar bölgenin kutusunda (halkaların sıkı kutusu, mutlak koordinatlarda) atılır: önce x = x₀ + u·w, sonra y = y₀ + u·h.
  - Bölgede ya da sınırında olmayan aday atılır ve noktanın yerine yenisi çekilir. Adaylar tek dizidir (u'lar arka arkaya, x sonra y);
    bir nokta için en çok 10 000 aday (ADR 0220 §4'ün sınırı), aşılırsa ret.
  - Yerler ilk alınan nesnenin yerine göre farktır (ADR 0238 §1); atılan nokta mutlak koordinattan o farka çevrilir.
  - Kenar kuralı kutudakiyle aynıdır, yalnız kenar bölgenin kendi sınırıdır: en yakın komşu yalnız atılan noktalar arasında aranır,
    sınırın ötesindeki komşu yoktur; kenar etkisi gözlenene de benzetime de aynı biçimde girer. Delikler de kenardır.
  - Tolerans bağı (ADR 0566 §1) bölgenin kutusunun köşegeniyle.
  - Sınır: kutuya düşme olasılığı A / (w·h); benzetim × nesne / olasılık en çok 5·10⁸ aday. Aşılırsa ret.
- **Tablo:** analitik satırlar kalır (Alan (m²) bölgenin A'sıdır). “Alan (m²)”'nin ardından eklenenler: “Çalışma alanı” (“3 alan, 1 delik”),
  “Alanın dışında kalan nesne” (sayı). Benzetim alanı satırı “Çalışma alanı nesnesi”dir.
- **Profil:** `region` = “Çalışma alanı nesnesi, A m²”. Girdi özeti yerlerden sonra bölgenin köşelerini de katar: halka başına köşe satırı
  `a<alan sırası>.<halka sırası>\tdoğu\tkuzey` (alan sırası bütün nesnelerin alanlarının sırasıyla 0'dan; halka 0 dış halka, 1 … delikler;
  köşeler nesnede yazıldığı gibi). Böylece aynı yerlerle başka bölge başka özet verir.
- **Uyarılar:** dışarıda kalan nesneler; alan olmayan çalışma alanı nesneleri.
- **Kenar düzeltmesi yok:** analitik z kenar etkisini düzeltmez (ADR 0566 bağlamı); düzeltme Monte Carlo'dadır. Donnelly'nin düzeltmesi
  sonraya kaldı; bağımsız başvurunun kuram denetiminde (§4) yalnız beklentisi kullanılır.

### 2. DBSCAN parametre taraması

- **Araç:** İşlemler'in “DBSCAN parametre taraması” (`stats.dbscanSweep`), CBS › Analiz'de Yerel Moran'ın yanında. Nesne girdisi
  DBSCAN'ınkidir; yeni katman yazmaz, tablo ve sayılar verir.
- **Izgara:**
  - **En az nokta:** “İlk”, “Son”, “Adım” (tam sayı, 1 … 1000; varsayılan 3, 8, 1), en çok 20 değer. Değerler `İlk, İlk + Adım, …` ve
    Son'u aşmayanlardır. En az nokta nesne sayısından büyük olamaz.
  - **Yarıçap:** “İlk”, “Son” (m) ve “Adım sayısı” (2 … 50, varsayılan 7). İlk ve Son boş bırakılırsa **otomatik aralık**: önerilen
    yarıçapın (aşağıdaki kural) yarısından iki katına. j'inci değer, j = 0 … adım sayısı − 1, `ilk + (son − ilk) · j / (adım sayısı − 1)`
    (float64'te çarpma, bölme, toplama). Yalnız biri yazılırsa ret; Son > İlk > 0 olmalıdır. Varsayılan 7 adımda üçüncü değer
    (j = 2: 0,5 ε\* + 1,5 ε\* · 2 / 6) ε\*'dır.
  - Hücre sayısı (yarıçap × en az nokta) en çok 400.
  - “Sınır noktaları gürültü” DBSCAN'ınki gibi (DBSCAN\*), bütün hücreler için.
- **Her hücre:** ADR 0238 §10'un DBSCAN'ı (komşuluk kuralı modülün kuralıdır: d = √(dx² + dy²) ≤ ε, float64'te; otomatik aralığın
  üçüncü yarıçapı bir veri uzaklığına düştüğü için bu hücrelerde kesin aritmetik değil bu kural karar verir). Tabloda satır satır, en az
  nokta sonra yarıçapla sıralı:
  - Küme sayısı; Gürültü (nesne sayısı); Gürültü payı (%, `100 · gürültü / n`, 1 ondalık);
  - Kararlılık sınaması seçiliyse (ADR 0566 §2, Bootstrap ya da Alt örnek, tekrar sayısı, tohum; aynı tohum her hücrede): Ortalama
    kararlılık (kümelerin kararlılıklarının ortalaması, 3 ondalık; küme yoksa “—”) ve Kararlı küme (kararlılığı ≥ 0,75 olanların sayısı).
    Sınır: hücre × tekrar × nesne en çok 2·10⁷.
- **Önerilen çift (kural):**
  1. **En az nokta:** ızgaradaki değerlerden 4'e en yakın olan; eşitlikte büyük olan (Schubert ve arkadaşları: iki boyutta 2·boyut = 4,
     emin olunamazsa büyük).
  2. **k-uzaklık eğrisi:** bu en az nokta m için her yerin, kendisi hariç (m − 1)'inci en yakın yere uzaklığı (m = 1'de 0): çünkü yarıçap
     içinde kendisi dahil m yer olan nokta çekirdektir. Uzaklıklar artan sırada d₀ ≤ … ≤ d₍n−1₎.
  3. **Dirsek (Kneedle, düzgünleştirmesiz, dışbükey artan eğri):** xᵢ = i / (n − 1), yᵢ = (dᵢ − d₀) / (d₍n−1₎ − d₀),
     gᵢ = xᵢ − yᵢ. Dirsek gᵢ'nin en büyük olduğu ilk i'dir (kesin büyük); ε\* = dᵢ. Tüm uzaklıklar eşitse ya da n < 3 ise ε\* = d₀.
  4. **Yarıçap:** ızgaradaki yarıçaplardan ε\*'a en yakın olan (eşitlikte küçük).
  5. Öneri ızgaradaki bir hücredir; tabloda “Önerilen” sütununda “◀ önerilen” ile işaretlenir. ε\* sıfırsa (en az nokta − 1 yer çakışık)
     otomatik aralık yazılamaz, ret; İlk ve Son yazılmışsa öneri yine bulunur.

  Kural bir sezgidir, kanıt değil: dirsek veriden okunan bir ipucudur; pencerenin açıklaması bunu söyler. Dirsek en büyük g'nin ilk yeridir:
  iki kıvrımı olan bir grafikte (kümeler ve ardından aykırılar) en belirgin olanı bulur; `kneed`'in varsayılan `online=False`'u ilk
  bulduğunu, kuyruktaki aykırıların kıvrımını bildirir (§4).
- **Çıktılar:**
  - `table`: sütunlar “En az nokta”, “Yarıçap (m)”, “Küme sayısı”, “Gürültü”, “Gürültü payı (%)”, [“Ortalama kararlılık”, “Kararlı
    küme”,] “Önerilen”;
  - sayılar: `cells`, `suggestedRadius`, `suggestedMinPoints`, `kneeRadius`, `clusters` (önerilen hücrenin), `noiseShare` (önerilenin, %);
  - kararlılık seçiliyse `profile` ve `experiment` (ADR 0520 §5; `parameters` ızgarayı yazar).
- **Özet:** “42 çift denendi; önerilen: yarıçap 13.841 m, en az 4 nokta (3 küme, %11.1 gürültü); k-uzaklık dirseği 13.841 m.”
- **Bilgi notu:** kuralın bir cümlesi (Ester 1996; Schubert 2017; Satopää 2011).
- **Aynı sayılar her platformda:** yalnız float64'ün dört işlemi, kareköke dayanan uzaklıklar (ADR 0238'in `dist2` ve √'ı) ve sıralama;
  rastgelelik yalnız kararlılıktadır ve ADR 0566'nın tohumlu dizisindedir.

### 3. Python, MCP ve arayüz

İşlemler'in araçları `cad.processing.run` ile çalıştığından yeni parametreler ve yeni araç Python ve MCP'ye kendiliğinden gelir
(Python'da nesneler kalıcı kimlikleriyle ya da katman kapsamıyla verilir). Arayüz kaydı `catalog/v1/additions.json`'dadır (komut
`processing.run.stats.dbscanSweep`, ikon `statsDbscanSweep`, CBS › Analiz'de Yerel Moran'ın yanında).

### 4. Bağımsız başvuru

`scripts/fixtures/spatial_study_cases.py` (`spatial_inference_cases.py` içinden; KentOS kodu olmadan, kesirler ve ADR'nin float64 işlemleri):
- **Bölge:** köşeleri doğrudan nesnelerden okunan çokgenlerde (delikli, iki parçalı, üst üste binen dikdörtgenler) kesin alan (koordinat
  sıkıştırmasıyla kesirlerle), kesin içeride–dışarıda (geçiş sayısı kesirlerle; bir kenara 10⁻⁵ m'den yakın yer ya da aday reddedilir:
  çekirdeğin sınırı 1 µm), benzetimdeki adaylar ve noktalar ADR'nin sırasıyla; çıkan tablo ve profil sözcüğü sözcüğüne.
- **Tarama:** k-uzaklık eğrisi, dirsek, yarıçaplar ve öneri float64'te; DBSCAN modülün komşuluk kuralıyla; kararlılıkta ADR 0566'nın
  başvurusu.
- **Kuramla denetim:** delikli karede (alan 100² − 40²) benzetimin ortalama uzaklığı, Donnelly'nin kenar düzeltmeli beklentisine
  (0,5 √(A/n) + (0,0514 + 0,041/√n) · P/n; P dış halka ile deliğin çevresi) %3 içinde; dirsek düz-sonra-dik grafikte son düz uzaklık.
- **Dışarıdan çapraz denetim** (`spatial_esda_cases.py`, `uv run … --with shapely --with kneed`; 23 durum):
  - çalışma alanı: shapely'nin birleşiminin alanı KentOS'unki, shapely'nin kapsadığı yerler KentOS'un aldıkları; numpy'ın 2 000 desenli
    Monte Carlo'sunun ortalaması KentOS'un benzetimlerinin ortalamasıyla üç standart hata içinde;
  - tarama: her hücrenin küme ve gürültü sayısı scikit-learn `DBSCAN`'ınki (DBSCAN\*: çekirdek noktalarının kümeleri; bir uzaklığı yarıçapın
    10⁻⁹'u içinde olan hücre, float64 kuralı karar verdiği için atlanır: 72 hücrenin 6'sı, hepsi `sweep-auto`'da); k-uzaklık grafiği
    `NearestNeighbors`'ın, dirsek `kneed.KneeLocator`'ınki (dışbükey, artan, S = 0, `online=True`).

  Bulgu: `kneed`'in varsayılan `online=False`'u, kümelerin ardından aykırıların ikinci bir kıvrımı olan grafikte (en az nokta 5 ve 6) ilk
  bulduğu kıvrımı, kuyruktakini (66 m) bildirir; en belirgin kıvrım (14,7 ve 15,2 m) `online=True`'nundur. Kural ikincisidir: kümeleri
  ayıran ölçek, aykırıları ayıran değil.

### 5. Sınırlar

- Bölge halkalarının köşe sayısı toplamı en çok 1 000 000.
- Tarama: hücre ≤ 400, en az nokta değeri ≤ 20, kararlılıkla hücre × tekrar × nesne ≤ 2·10⁷. Her hücre ADR 0238'in komşu çifti sınırına
  tabidir; aşan bütün çalıştırmayı reddeder ve hangi yarıçapta aştığını söyler.

### 6. R'ın fpc'siyle çapraz denetim

`which Rscript R` bu makinede bir şey bulmadı; R.framework ve Homebrew'da R yok. Sahibin izni olmadan kurulmadı. Bu yüzden `fpc::dbscan`
ve `fpc::clusterboot` ile çapraz denetim yapılmadı; kararlılığın başvurusu ADR 0566'da olduğu gibi Hennig'in (2007) tanımından yazılmış
Python'dur, DBSCAN'ın dışarıdan başvurusu scikit-learn'dür. R kurulursa `fpc::dbscan(x, eps, MinPts)`'in bölüntüsü taramanın her
hücresiyle, `clusterboot(…, bootmethod = "boot")` bu ADR'nin ve ADR 0566'nın kararlılığıyla (sayılar farklı üreteçle üretileceği için binom
hatası içinde) karşılaştırılır.

## Sonuç

- En yakın komşu, çalışma alanı bilinen bir bölgede kenar etkisi ve boş yerlerden arınmış sonuç verir; kutu kuralı değişmez. Düzgün
  dağılmış noktalar delikli bir halka bölgede “rastgele” çıkar (oran 1,0029); aynı noktalar kutuyla ölçülseydi dağınık görünürdü.
- DBSCAN'ın iki parametresi, ızgarada küme sayısı, gürültü ve kararlılığıyla tek tabloda görülür ve kuralı yazılı bir öneri verir.
- Yeni `.kcad` alanı yoktur; şema değişmez.

## Doğrulama

| Denetim | Nasıl | Sonuç |
|---|---|---|
| Ortak durumlar | `python3 scripts/fixtures/spatial_inference_cases.py` (başvuru ve kuram denetimleri kendi içinde) yazar; `cargo test -p kentos-processing --test cases` (pencere ve `cad.processing.run` eşdeğerliği, varsayılanlar dahil) | 14 yeni durum: `nn-study`, `nn-study-sim`, `nn-study-area-ignored`, `nn-study-selection`, `nn-study-none`, `nn-study-points`, `sweep-auto`, `sweep-range`, `sweep-stability`, `sweep-subset`, `sweep-many`, `sweep-too-big`, `sweep-half-range`, `sweep-steps`; 32 test geçti |
| Çekirdek | `cargo test -p kentos-geometry-core --lib spatial_stats` | delikli alanın net alanı ve dışarıda kalanlar, alan olmayan çalışma alanı, dirsek, 4'e en yakın, değer listesi; 12 test geçti |
| Dışarıdan | `uv run --no-project --python 3.12 --with esda --with libpysal --with pointpats --with scikit-learn --with shapely --with kneed python3 scripts/fixtures/spatial_esda_cases.py --check` | 23 durum, paketlerle aynı (yukarıdaki 6 belirsiz hücre atlandı) |
| Python | `.run/py/bin/python -m unittest python.tests.test_spatial_study` | ortak durumlar başsız çalıştırıcıda aynı özet, tablo ve sayılarla; 3 test geçti |
| Masaüstü | `cargo test -p kentos-desktop stats_tests` | çalışma alanı penceresi (alan 1 031 700 m², delikli, 155 = alınan + dışarıda) ve tarama (42 hücre, tek işaret, kararlılıkla 8 sütun); 7 test geçti |
| Resimler | `KENTOS_SHOTS_ONLY=ist-alan-pencere,ist-alan-sonuc,ist-alan-cizim,ist-tarama-pencere,ist-tarama,ist-tarama-oneri,ist-tarama-kararlilik cargo test -p kentos-desktop tools_screens -- --ignored --nocapture` | `.run/shots/arac-ist-alan-*`, `arac-ist-tarama-*` (iki tema, iki boyut) |
| Süre | `cargo test --release -p kentos-geometry-core --test all spatial_study_timing -- --ignored --nocapture --test-threads=1` | 10 000 nokta (6 800'ü 360 kenarlı halkada, 120 kenarlı deliğin dışında): 999 desenle En yakın komşu 4,0 s (bütçe 8 s); 42 hücreli tarama 0,23 s (bütçe 1 s), 10 bootstrap tekrarıyla 2,3 s (bütçe 6 s) |
