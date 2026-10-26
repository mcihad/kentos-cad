# ADR 0643: Raster çözümlemesinde analiz ortamı, blok düzeninden bağımsızlık ve ağırlık duyarlılığı

- **Durum:** kabul edildi (2026-10-26). `plan/gis.md` CBS-30'un kalanı. Kapsamı ben belirledim.
- **Kalan işler:**
  - öbür raster aileleri;
  - ortak hücre boyu, hizalama ve NoData ortamı;
  - büyük harici raster için disk ara dosyası;
  - karoların halo karşılaştırması;
  - belirsizlik raporu.

  Yüzey analizinin alanı ve ızgara kaydı ADR 0535'te bitti.
- **Bağlam belgesi:** ADR 0231 (raster analizi), 0232–0237 (aileler), 0242 (uzaktan algılama), 0508 (`cad.processing.run`),
  0529 (toplu çalıştırma), 0535 (yüzey analizinin alanı), 0642 (adresteki raster).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **`.kcad`:** şema değişmez; ortam çizime yazılmaz, oturumun ve komutun girdisidir.
- **Sonraya kalan:**
  - Ortam sunucu işine gitmez. Masaüstü sunucuda çalıştırmayı ortam varken reddeder ve nedenini söyler.
  - Adresteki rasterin diske alınması pencerede ilerleme göstermez ve durdurulamaz. Başsız sunucu adresi önceki gibi parça
    parça okur.
  - Sensörün ve datumun belirsizliği raporlanmaz; yalnız ağırlık duyarlılığı raporlanır. Çalışma kaydı girdinin ve sonucun
    ızgarasını yazar; sensör bilgisi (STAC) CBS-31'dedir.
  - Ortam Kapsam'ı olan yüzey araçlarında alanın yerine geçmez; aracın kendi Kapsam'ı önce gelir.

## Bağlam

Raster araçlarının her biri kendi ızgarasında çalışıyordu:

- girdinin ızgarası;
- girdilerin ortak ya da birleşik ızgarası;
- noktalardan yapılan rasterde aracın Hücre boyu'yla kutu.

Bu yüzden aynı alanı, aynı hücre boyunu ve aynı hizalamayı bir araçtan ötekine taşımanın yolu yoktu. ArcGIS'in ortam
ayarları ve QGIS'in işlem kapsamı ayarı bunu çözer: işlem alanı, hücre boyu, hizalama rasteri.

Yalnız yüzey analizinin kendi Kapsam'ı vardı (ADR 0535). NoData değeri araçtan araca değişiyordu; kayan noktalı sonuçta
değer NaN'di.

Plan iki şey daha istiyor:

- blok boyutu ya da raster karolarının düzeni sonucu değiştirmemeli;
- uygunluk ağırlıklarının belirsizliği raporlanmalı.

## Karar

### 1. Ortam

Analiz ortamı dört isteğe bağlı parçadır (`kentos_raster::env::Environment`):

- **Alan:** en küçük doğu ve kuzey, en büyük doğu ve kuzey.
- **Hücre boyu:** kenar, çizim biriminde.
- **Hizalama rasteri:** sonuç hücrelerinin kenarları bu rasterin çizgilerine oturur.
- **NoData:** değersiz sonuç hücresinin aldığı değer. Dosyanın NoData etiketi de budur.

Araç sonucun ızgarasını her zamanki gibi bulur; buna taban denir. Ortam, işin çalıştığı ızgarayı bu tabandan kurar:

- **Yalnız alan verilirse:** tabanın alanın dokunduğu hücreleri alınır. Kafes aynıdır; köşenin hücre kenarına `1e-9` hücre
  yakınlığı kenar sayılır. Kural ADR 0535'teki pencereyle aynıdır. Alan rasterle kesişmezse reddedilir.
- **Hücre boyu ya da hizalama verilirse:** yeni bir kafes kurulur.
  - Hücreler karedir. Kenarı verilen hücre boyudur; verilmezse hizalama rasterinin hücresi, o da yoksa tabanın hücresi.
  - Kenarlar hizalama rasterinin çizgilerine oturur; hizalama yoksa tabanın başlangıcından geçen çizgilere.
  - Kafes, tabanın kapsamıyla kesilmiş alanı (alan yoksa kapsamın tamamını) bütün hücrelerle örter.
  - İki ızgaranın da kuzeyi yukarıda olmalıdır.

Komutun girdisi `cad.processing.run`'ın ve `.batch`'in `environment`'ıdır (`RasterEnvironment`):

| Alan | Anlamı |
|---|---|
| `area` | kutu |
| `areaObjects` | kutusu alınan nesneler; `area` da verildiyse ikisinin birleşik kutusu |
| `cell` | hücre boyu |
| `snap` | hizalama rasterinin kalıcı kimliği |
| `nodata` | değersiz hücrenin değeri |

Komut ortamı çizim üstünde çözer (`kentos_processing::raster_env::resolve`):

- okunamayan değeri `invalid_parameters` ile ve `environment.…` yoluyla reddeder;
- kimliği okunamayanı `invalid_uid` ile, çizimde olmayanı `entity_not_found` ile reddeder;
- hizalama rasteri bir raster değilse reddeder.

Ortamı almayan araç (§2'deki ailelerin dışındakiler) ortamı yok saymaz, `environment` yoluyla reddeder. Bir model ortamı,
adımlarından biri alıyorsa alır. Boş ortam ortam yok sayılır.

### 2. Ailelerin ortama bağlanması

Hücre değerinin kaynağı ailesine göre değişir:

- Yerel araçlar hücreyi kaynağın komşularından hesaplar: komşuluk istatistiği, eğim ve öbür yüzey araçları. Alandaki hücrenin
  değeri, bütün rasterde alacağı değerin aynısıdır.
- Küresel araçlar bütün rastere bakar: hidroloji, uzaklık ve maliyet. Bu araçlar ortamın alanıyla sınırlıdır; alanın kenarı
  rasterin kenarı gibidir. ArcGIS'in işlem alanı da böyle çalışır.

| Aile | Alan | Hücre boyu, hizalama | NoData |
|---|---|---|---|
| Raster hesaplayıcı, Yeniden sınıflandır, Mozaik, Hücre istatistiği, Bölgesel istatistik, Histogram, ROC | ✓ | ✓ (en yakın hücre) | ✓ |
| Uygunluk (üyelik, bulanık çakıştırma, ağırlıklı toplam ve çakıştırma, İkili karşılaştırma) | ✓ | ✓ | ✓ |
| Uzaktan algılama: Bant birleştir, Bantlara ayır, Spektral indis, Değişim, Görüntü birleştirme | ✓ | ✓ | ✓ |
| Maskeyle kırp, COG olarak kaydet, Komşuluk istatistiği, Hidroloji, Uzaklık ve maliyet | ✓ (kendi hücreleri) | ret | ✓ |
| Yüzey analizi (eğim, bakı, kabartma, eğrilik, pürüzlülük, güneşlenme, eş yükselti) | ✓ (Kapsam önce) | ret | kayan noktada ✓, 8 bitte ret |
| İnterpolasyon, yoğunluk, Uzaklık yüzeyi (nesnelerden) | ✓ | ✓ | ✓ |
| Rasterleştir | ✓ | ✓ | ret |
| Yeniden örnekle, rasterden vektöre araçlar, Denetimli ve Denetimsiz sınıflandırma, Doğruluk | ret | ret | ret |

Retlerin sözleri:

- Kendi hücreleriyle çalışan araç: “… rasterin kendi hücreleriyle çalışır: hücre boyu ve hizalama ortamı için girdiyi önce
  Yeniden örnekle ile o ızgaraya getirin.”
- Ortamı hiç almayan araç: “… analiz ortamının alanını, hücre boyunu ve hizalamasını almaz; …”.

Kendi hücreleriyle çalışan araçta alan, kaynağın hücrelerine kayma (`offset`) olarak verilir. Okunan bölgeler ve komşuluk
payı bu kaymayla kaynağa taşınır.

Doğrulama sırasında Komşuluk istatistiği'nin okuma bölgesinin kaymayı atladığı bulundu ve düzeltildi. Pencerenin üst
kenarındaki hücrelerin komşuları yanlış satırdan geliyordu.

### 3. Adresteki raster diske alınır

Aşağıdaki araçlar rasteri bütün tutar ya da birden çok kez okur (`OpsSpec::whole`):

- rasterden vektöre araçlar ve taranmış pafta araçları;
- hidroloji;
- uzaklık ve maliyet;
- Denetimli ve Denetimsiz sınıflandırma.

Bu araçlar adresten okunan rasteri önce yerel bir kopyaya alır (`Files::stage_raster`). Masaüstünde kopya
`$XDG_CACHE_HOME/kentos-cad/raster-gecici`'dedir. ADR 0642'nin indirmesiyle iner; çalıştırma başına ayrı adla, en çok
8 GB. Çalıştırma bitince, nasıl biterse bitsin, kopya silinir.

### 4. Kayıt ve ağırlık duyarlılığı

Çalışma kaydı her raster ailesinde girdinin ve sonucun ızgarasını yazar (`RunResult.grids`):

- afin, boy, NoData, sistem;
- yüzey analizinde ayrıca bant ve pencere (ADR 0535).

Aracın kendi kaydı yoksa ortak sürücüler not eder (`grid_record`). Python'da `ran.record.grids` olarak okunur.

Ağırlıklı toplam, İkili karşılaştırma'nın ağırlıklı toplamı ve Ağırlıklı çakıştırma duyarlılığı özette söyler. Duyarlılık,
her ağırlık kendi başına %10 artıp azalınca bir hücrenin değerinin en çok ne kadar değişebileceğidir. Bu bir üst sınırdır,
örnekleme değildir:

- **Toplamda:** bir hücrenin oynaması `0,1 · Σ|wₖ·xₖ|`'dir.
- **Çakıştırmada:** etkiler tam birimlerdedir; payın onda biri yukarı yuvarlanır. Söylenen, toplamın ulaşabildiği sınıf
  farkıdır.
- **Rapor:** değeri olan hücrelerde en büyük ve ortalama oynama. Hücreler satır sırasıyla toplanır; iş parçacığı sayısı
  sonucu değiştirmez.

Örnek: “Her ağırlık ayrı ayrı %10 değişirse bir hücre en çok 2.0475, ortalama 1.5684 değişebilir.”

### 5. Masaüstü

İşlemler penceresinde ortamı alan araçlarda katlanır bir **Analiz ortamı** bölümü vardır:

- **Alan:** Bütün raster, Görünen alan ya da Seçili nesnelerin kutusu.
- **Hizalama rasteri:** çizimin rasterleri, boyları ve hücre boylarıyla.
- **Hücre boyu:** çizim biriminde.
- **NoData.**
- **Temizle.**

Başlıktaki rozet verilen parça sayısını gösterir. Okunamayan sayının alanı kırmızı çerçevelenir ve Çalıştır'ın yanında
söylenir.

Ortam oturumundur: bir raster aracında verilen ortam öbür raster araçlarının pencerelerinde de geçerlidir. Seçili nesnelerin
kutusu boşsa çalıştırma başlamaz.

Ortam yerel çalıştırmaya, arka plandaki modele ve sonucu dosyaya ya da veritabanına yazan çalıştırmaya gider. Toplu
çalıştırmada her satır ortamı alır.

### 6. Süreler

Bu makinede, release, 7 iş parçacığı, `environment_timing`. 4096² Float32 raster, bellekte karolu Deflate GeoTIFF. Ölçülen
süre bütün işin süresidir: blokların çözülmesi, hücreler, katlar ve kodlama dahil.

| İş | p50 |
|---|---|
| Hesaplayıcı `[A] * 2 + [B]`, ortamsız | 0,606 s |
| aynısı, alan (ortadaki çeyrek, 2048²) | 0,191 s |
| aynısı, hücre 4 m (hücrelerin ¼'ü) | 0,437 s |
| aynısı, alan ve hücre 1 m (4096²) | 0,464 s |
| aynısı, 1 m kayık hizalama | 0,673 s |
| aynısı, NoData −9999 | 0,604 s |
| Komşuluk 5 × 5 ortalama, ortamsız | 0,707 s |
| aynısı, alan (çeyrek) | 0,203 s |
| Ağırlıklı toplam, üç raster, duyarlılıkla | 0,839 s |

Sonuçlar:

- Alan, okunan blokları ve hesaplanan hücreleri alanınkilere indirir.
- Kayık hizalama hücreyi en yakın kaynak hücresinden okur; maliyeti %11'dir.
- NoData'nın maliyeti yoktur.

## Doğrulama

- **Ortak durumlar:** `scripts/fixtures/raster_env_cases.py` (GDAL ve numpy; KentOS kodu kullanılmaz) 31 durum yazar
  (`fixtures/raster-env/v1`).
  - Aynı yükseklik modeli dört blok düzeninde yazılır: 16'lık ve 32'lik karolar, birer ve yedişer satırlık şeritler. Bunlardan
    çalışan hesaplayıcı, komşuluk, eğim ve doldurma aynı baytları yazar (`sameAs`). Bu, blok sınırının sonucu değiştirmediğinin
    ve komşuluk payının öbür karodan geldiğinin karşılaştırmasıdır.
  - Izgaralar §1'in kuralıyla kurulur.
  - Değerlerin kaynakları:
    - hesaplayıcı ve sınıflandırma: hücrenin ortasının düştüğü kaynak hücresi;
    - komşuluk: bütün rasterin kesirlerle ortalaması (`raster_ops_cases.py`);
    - eğim: bütün rasterin Horn eğimi (`terrain_cases.py`);
    - doldurma: alanın alt modelinin doldurulması (`hydrology_cases.py`).
  - Ret durumları: hücre boyu, hizalama, ortamı almayan araç, dışarıdaki ve ters alan, sıfır hücre, raster olmayan hizalama.
  - `kentos-processing`'in `the_environment_cases_do_what_they_say` testi durumları ortamı komutun çözümüyle vererek oynatır.
    Her yazılan rasterin ızgarasını, 0. katını ve kayıttaki ızgaraları denetler.
- **Komut durumları:** 20 durum (`fixtures/commands/v1/cad.processing.run.environment.json`). Ortamın geçerli biçimleri ve
  her ret, kodu ve yoluyla denetlenir.
- **Duyarlılık:** `suitability_processing_cases.py` duyarlılığı başvurunun girdilerinden bağımsız hesaplar. Dört özet
  Rust'ınkiyle aynıdır.
- **Python:** `python/tests/test_raster_environment.py`. Alan ve kayık hizalama kayıttaki ızgarayla denetlenir; ortamı
  almayan araç reddeder.
- **Masaüstü:** `processing::environment_tests`.
  - 25 m'lik hücre ve hizalamayla yazılan rasterin afini `[499976, 25, 0, 4420324, 0, −25]` ve boyu 17 × 13'tür.
  - Ortam pencereden pencereye geçer.
  - Okunamayan sayı çalıştırmayı durdurur.
  - Ortamı almayan araç bölümü göstermez.
  - Resimler `.run/shots/islem-ortam-*`'tadır.
