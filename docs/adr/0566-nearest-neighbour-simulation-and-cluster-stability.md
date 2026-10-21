# ADR 0566: En yakın komşu'nun benzetimi, kümelerin kararlılığı ve esda ile çapraz denetim

- **Durum:** kabul edildi (2026-10-21). `plan/gis.md` CBS-25'in kalanı. ADR 0520'nin kapsam dışında bıraktıkları:
  - DBSCAN ve k-ortalamaların kararlılık sınaması;
  - En yakın komşu'nun permütasyonu;
  - PySAL/esda ile çapraz sınama.

  Kapsamı ben belirledim.
- **Bağlam belgesi:**
  - ADR 0238 (En yakın komşu, DBSCAN, k-ortalamalar);
  - ADR 0520 (kural `kentos.spatial-inference/1`, deney profili, alt küme);
  - ADR 0220 (tohumlu sayılar).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - çalışma alanının bir alan nesnesinden verilmesi (bugün kutu ya da alanla ölçeklenmiş kutu);
  - DBSCAN'ın parametre taraması (k-uzaklık grafiği);
  - kümelerin bootstrap'tan güven bölgeleri;
  - R'ın `fpc::clusterboot`'uyla çapraz denetim (bu makinede R yok).

## Bağlam

ADR 0238'in En yakın komşu aracı Clark ve Evans'ın (1954) z'sini verir. Bu z rastgele desenin sınırsız düzlemde olduğunu varsayar.
Gerçek çalışma alanının kenarına yakın noktaların en yakın komşusu daha uzaktadır, bu yüzden küçük ve kenarı uzun alanlarda oran
dağınıklığa doğru kayar. Çözüm Monte Carlo sınamasıdır (Diggle 2003; pointpats ve spatstat'ın benzetim zarfları): aynı alana aynı
sayıda nokta rastgele atılır, kenar etkisi gözlenene de benzetime de aynı biçimde girer.

DBSCAN ve k-ortalamalar her girdide bir bölüntü verir ama bölüntünün ne kadar güvenilir olduğunu söylemez. Hennig (2007, “Cluster-wise
assessment of cluster stability”, *Computational Statistics & Data Analysis* 52) bunu her küme için ölçer. R'ın `fpc::clusterboot`'u
bu yöntemi uygular: veri yeniden örneklenir ve yeniden kümelenir. Özgün kümenin örnekteki karşılığına en çok benzeyen yeni kümenin
Jaccard benzerliği alınır, bunların tekrarlar boyunca ortalaması kümenin kararlılığıdır. Hennig'in eşikleri şunlardır:
- 0,5 ve altı “çözülmüş” sayılır;
- 0,75'ten büyükse küme “yeniden bulunmuş” sayılır;
- 0,6'nın altı güvenilmezdir;
- 0,75 ve üstü geçerlidir;
- 0,85 ve üstü çok kararlıdır.

ADR 0520'nin bağımsız başvurusu esda'nın tanımlarıyla yazılmıştı ama esda bu makinede yoktu. esda 2.10, libpysal 4.15 ve pointpats 2.6
`uv` ile kurulabiliyor.

## Karar

### 1. En yakın komşu'nun benzetimi

- **Çıkarım:**
  - Normal dağılım (varsayılan, ADR 0238);
  - Monte Carlo (rastgele desen).

  Ayrıca Benzetim sayısı (9 … 99 999, varsayılan 999) ve Tohum (varsayılan 1).
- **Benzetim alanı:**
  - Alan boşsa yerlerin kutusu.
  - Alan yazılmışsa kutu, merkezi ve en–boy oranı korunarak alanı A olacak biçimde ölçeklenir (kenarlar √(A / (w·h)) ile çarpılır).
  - Kutunun bir kenarı sıfırsa (bütün yerler bir doğruda), merkezinde kenarı √A olan bir kare kullanılır.
- **Sayılar** (`kentos.spatial-inference/1`'in eki):
  - Tek dizi.
  - Benzetim benzetim, nokta nokta: önce x = x₀ + u·w, sonra y = y₀ + u·h.
  - u, ADR 0220'nin [0, 1)'deki sayısıdır (üst 53 bit / 2⁵³).
  - Yerler ilk yere göre farktır, ADR 0238'in çalışma çerçevesi.
- **İstatistik ve p:**
  - İstatistik her benzetimin ortalama en yakın komşu uzaklığıdır; gözlenenle aynı yoldan, k-d ağacıyla bulunur.
  - Benzetimin uzaklığı gözlenenden kutunun köşegeninin 10⁻⁹ katı kadar küçükse de “en az o kadar” sayılır (ADR 0520 §2'nin bağ kuralı).
  - p katlanmış sözde p'dir.
  - z = (gözlenen − benzetimlerin ortalaması) / standart sapma.
  - Desen benzetimin z'si ve p'sinden: eksi z kümelenmiş demektir (ADR 0238 §2).
- **Tablo:** analitik satırlar kalır. Eklenenler:
  - Benzetim sayısı, Tohum, Benzetim alanı;
  - Benzetimlerin ortalama uzaklığı (m, 3 ondalık);
  - Benzetim oranı (gözlenen / benzetimlerin ortalaması, 4 ondalık);
  - Benzetim z (4 ondalık), Benzetim p (6 ondalık);
  - p'nin standart hatası.
- **Çıktılar:** `zSimulation`, `pSimulation`, `profile`, `experiment`.
- **Özet:** “En yakın komşu oranı 0.8123 (benzetim z −2.1234, p 0.012000; 999 rastgele desen, tohum 1): kümelenmiş.”
- **Sınır:** benzetim sayısı × nesne sayısı en çok 5·10⁷.

### 2. Kümelerin kararlılığı (Hennig'in clusterboot'u)

- **DBSCAN ve k-ortalamalar'a eklenen seçenekler:**
  - Kararlılık sınaması: Yok (varsayılan), Bootstrap ya da Alt örnek;
  - Tekrar sayısı (10 … 999, varsayılan 100);
  - Tohum (varsayılan 1).
- **Yeniden örnekleme:**
  - **Bootstrap**, `fpc`'nin `boot`'u: n kez `below(n)`, yerine koyarak.
  - **Alt örnek**, `fpc`'nin `subset`'i: ⌊n/2⌋ nesne yerine koymadan, özdeşlikten kısmi Fisher–Yates ile (j = t + below(n − t)).
  - Örneğin konumları artan sıraya dizilir. Böylece yeniden kümeleme nesneleri özgün sıralarıyla gezer, aynı nesnenin kopyaları yan yana
    durur.
  - Örnek aynı parametrelerle aynı yöntemle kümelenir.
  - k-ortalamalar'da örneğin farklı yer sayısı k'den azsa o tekrar farklı yer sayısı kadar kümeyle kümelenir ve söylenir.
- **Benzerlik:**
  - Özgün küme C'nin örnekteki karşılığı C\*, örnekte en az bir kopyası olan özgün nesnelerdir.
  - Yeni küme D, ilk kopyasının etiketi D olan özgün nesnelerin kümesidir; gürültü küme sayılmaz.
  - J(C\*, D) = |C\* ∩ D| / (|C\*| + |D| − |C\* ∩ D|), float64'te bir bölmedir.
  - γ, en büyük J'dir; hiçbir D C\*'a değmiyorsa 0'dır.
  - C\* boşsa tekrar o küme için sayılmaz (`fpc`'nin `bootmean`'i).
- **Kümenin kararlılığı:**
  - Sayılan tekrarların γ'larının toplamının sayılarına bölümüdür (tekrarların sırasıyla toplanır).
  - Yeniden bulunma, γ > 0,75 olan tekrarların sayısıdır.
  - Çözülme, γ ≤ 0,5 olan tekrarların sayısıdır.
- **Sınıflar:**
  - Çok kararlı (≥ 0,85), Kararlı (≥ 0,75), Belirsiz (≥ 0,6), Güvenilmez (< 0,6);
  - hiç sayılmayan küme “Sınanamadı”.
- **Çıktılar:**
  - Kopyalara Kararlılık (3 ondalık) ve Kararlılık sınıfı eklenir; gürültüde ikisi de boştur.
  - `table` sütunları: Küme, Nesne sayısı, Kararlılık, Sınıf, Yeniden bulunma, Çözülme, Sayılan tekrar.
  - `stable`: kararlılığı ≥ 0,75 olan küme sayısı.
  - `profile` ve `experiment`.
- **Özet:** eskisinin arkasına “ 3 kümeden 2'si kararlı (≥ 0.75; 100 bootstrap tekrarı, tohum 1).” eklenir.
- **Sınır:** tekrar sayısı × nesne sayısı en çok 2·10⁷. Her tekrar ADR 0238'in komşu çifti sınırına tabidir; aşan bütün çalıştırmayı reddeder.

### 3. Deney profili

ADR 0520 §5'in profili, şu anahtarlarla:

| Anahtar | En yakın komşu | Kararlılık |
|---|---|---|
| `rule` | `kentos.spatial-inference/1` | aynı |
| `tool` | En yakın komşu | DBSCAN kümeleme, k-ortalamalar kümeleme |
| `inference` | Monte Carlo (rastgele desen) | Bootstrap (yerine koyarak), Alt örnek (yarısı, yerine koymadan) |
| `simulations` / `resamples` | Benzetim sayısı | Tekrar sayısı |
| `seed`, `generator` | Tohum, Sayı üreteci | aynı |
| `region` / `parameters` | Benzetim alanı | Yöntemin parametreleri (Yarıçap, En az nokta, Sınır noktaları; k) |
| `objects` | Nesne sayısı | aynı |
| `scope`, `inputs` | Kapsam, girdi özeti | aynı |

- Girdi özetinin satırı değersizdir: `girdideki sırası \t doğu \t kuzey\n`.
- İletinin sözü profilin sayısını adlandırır:
  - “Deney profili (…): tohum 1, 999 rastgele desen; girdi özeti …”;
  - “…: tohum 1, 100 tekrar; …”;
  - ADR 0520'ninki değişmez.

### 4. esda ile çapraz denetim

`scripts/fixtures/spatial_esda_cases.py` (`uv run --no-project --python 3.12 --with esda --with libpysal --with pointpats`) ortak çizimin
kazalarını ve parsellerini okur. ADR 0520'nin ve bu ADR'nin durumlarındaki değerleri esda'nın kendi hesabıyla karşılaştırır.

Ağırlıklar, bağımsız başvurunun komşu listelerinden libpysal'ın `W`'su olarak kurulur. Böylece ağırlıkların kuralı değil, hesap
sınanır.

| Denetim | Nasıl |
|---|---|
| Moran I | `esda.Moran`: I, beklenen I, rastgele dağıtım varyansı, z ve p aynıdır |
| Gi\* | `esda.G_Local(star=True)`: her nesnenin z'si aynıdır |
| Yerel Moran | `esda.Moran_Local`: her nesnenin Iᵢ'si ve çeyreği aynıdır |
| Permütasyon p'leri | KentOS'unkiyle esda'nın 9 999 permütasyonlu p'si binom hatasının beş katı içindedir (esda numpy'ın üretecini kullanır, sayılar aynı değildir) |
| FDR | `esda.fdr`'ın eşiğinin altında kalan nesneler, KentOS'un düzeltilmiş p'si eşiğin altında kalanlardır |
| En yakın komşu | `pointpats.PointPattern`: ortalama en yakın komşu uzaklığı aynıdır |
| DBSCAN ve k-ortalamalar | scikit-learn'ün `DBSCAN`'ı ve `KMeans`'i (aynı başlangıç merkezleriyle, `n_init=1`): bölüntü numaralar dışında aynıdır |

İki bulgu:
- esda'nın `Moran_Local`'ı Iᵢ'yi (n − 1)·zᵢ·Σwᵢⱼzⱼ / Σz² olarak yazar. Anselin'in (ve KentOS'un) tanımı n ile böler; karşılaştırma
  bu çarpanla yapılır, p'ler etkilenmez.
- Satır standartlaştırması kapalı (ikili ağırlıklı) Yerel Moran'da esda'nın koşullu permütasyon p'si tanımdan sapar. Örneğin ortak
  çizimin 14. kazasında esda 0,0003 verir; numpy'ın kendi çekilişleriyle 200 000 koşullu permütasyon 0,0274, KentOS 0,028 verir. Satır
  standartlaştırmalı durumlarda esda ile KentOS uyuşur. Bu durumda betik p'yi numpy'ın kaba kuvvetiyle (20 000 çekiliş)
  karşılaştırır. esda'nın kodundaki neden incelenmedi.

## Doğrulama

- **Bağımsız başvuru** `scripts/fixtures/spatial_inference_cases.py`'nin yeni bölümleri:
  - benzetimin noktaları kuralın tam sayılarıyla ve Python'un float64'üyle (aynı IEEE işlemleri) üretilir;
  - en yakın komşu uzaklıkları kesirlerle ve 60 basamaklı kökle bulunur;
  - kararlılığın yeniden örneklemesi, DBSCAN'ı ve k-ortalamaları kesirlerle ADR 0238'in kuralından yazılmıştır;
  - Jaccard ve kararlılık kesir olarak hesaplanır, sınıf eşiğine yakın değer reddedilir.
- **Kendi denetimleri:**
  - kare alanda, kenar etkisi yokken benzetimlerin ortalaması Clark–Evans'ın 0,5/√ρ'suna, Donnelly'nin kenar düzeltmesi içinde
    yaklaşır;
  - iyi ayrılmış iki küme 1'e yakın kararlıdır;
  - yan yana iki gauss kümesinin k = 3'teki bölüntüsünde en az bir küme kararsızdır.
- **Ortak durumlar:** `fixtures/processing/v1/spatial-inference.json`'a yeni durumlar eklenir:
  - En yakın komşu: benzetim, alanla ve alt kümeyle;
  - DBSCAN: bootstrap ve alt örnekle;
  - k-ortalamalar: bootstrap, k = 3 ve k = 5;
  - retler.

  ADR 0520'nin 13 durumu değişmeden geçer.
- **Çapraz denetim:** `spatial_esda_cases.py --check` (§4). 15 durumun hepsinde değerler paketlerinkiyle aynıdır:
  - üç Moran I'nın I, E[I], varyans, z ve p'si;
  - üç Gi\*'ın her nesnesinin z'si ve permütasyon p'si, iki FDR'ın üç düzeyde reddettikleri;
  - üç Yerel Moran'ın her Iᵢ'si, çeyreği ve p'si;
  - iki En yakın komşu'nun ortalama uzaklığı;
  - dört kümeleme.
- **Masaüstü:** pencerelerde yeni seçenekler. Resimler `tools_screens`'in `ist-komsu-benzetim`, `ist-kararlilik` ve
  `ist-kararlilik-cizim`'idir.
- **Süreler** (release, geliştirme makinesi, `spatial_stability_timing`):

  | İş | Süre | Bütçe |
  |---|---|---|
  | En yakın komşu, 10 000 nokta, 999 rastgele desen | 2,89 s | 5 s |
  | DBSCAN, 10 000 nokta, 100 bootstrap tekrarı | 0,26 s | 3 s |
  | k-ortalamalar, 10 000 nokta, k = 10, 100 bootstrap tekrarı | 0,29 s | 3 s |

  Düzgün rastgele noktalarda Monte Carlo sınaması “rastgele” der (oran 1,0029, p 0,409); bu, sınamanın kendi denetimidir.
