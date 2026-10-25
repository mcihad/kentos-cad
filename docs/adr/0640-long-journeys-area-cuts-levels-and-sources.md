# ADR 0640: Yedi günden uzun yolculuk, hizmet alanını kesenler, seviyenin kesmesi ve bağlı katmanın kaynağı

- **Durum:** kabul edildi (2026-10-25). `plan/gis.md` CBS-26'nın kalanı: yedi günden uzun yolculuk, hizmet alanında
  ulaşılamayanın nedeni, seviyenin kesmesi, dış yönlendirici, bağlı katmanın revizyonu. Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0521 (dönüşler, seviyeler, zaman profili), 0567 (araç profilleri, tarihli kapanmalar), 0568 (haftanın
  günleri ve saatleri), 0570 (ulaşılamayanın nedeni), 0575 (en az kapatan), 0577 (düğüm, dönüş, cihaz), 0503 ve 0597 (bağlı
  katman, dosyanın SHA-256'sı).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **`.kcad`:** şema değişmez.
- **Sonraya kalan:**
  - Bir uca değen seviye: başka seviyedeki bir kenarın içine değen uç (yerleri bir düğüm değil) seviyenin kesmesi olarak söylenmez;
    yalnız iki yerin de düğümü olduğu yerde söylenir.
  - Veritabanına bağlı katmanın kaynağının değişmesi: satırları hızla özetlenemez; yalnız dosya kaynakları denetlenir.
  - Yolculuk, sorunun gününden 92 günden uzun sürerse o günlerin kapanmaları kapanmanın kendi tarihlerini değil, çerçevenin
    ilk ya da son haftasının yinelenişini izler.

## Bağlam

ADR 0568'in saati haftayı yineliyordu: sorunun gününden önceki gün ve sonraki beş gün. Bir hafta süren yolculuk aynı haftayı
yeniden yaşıyordu. Bu, tarihli bir kapanmanın ilk ya da son günü yolculuğun içine düştüğünde yanlış süre veriyordu: gelecek
hafta iki gün kapanacak bir geçit, yedinci günden sonra hiç kapanmıyor sayılıyordu.

Hizmet alanı nerede bittiğini söylemiyordu. Bir kenarı neden almadığını görmek için ayrıca rota sormak gerekiyordu.

Seviyenin (köprü, tünel) yolu kesmesi, ulaşılamayanda “Başka” olarak kalıyordu: aynı yerde iki kenar vardı ama seviyeleri ayrıydı.

Bağlı katmanın dosyası çalışma kopyası okunduktan sonra değişirse ağ eski kopyadan kuruluyor ve bunu söylemiyordu.

## Karar

### 1. Çerçeveli saat

Saatli kapanması olan her kenarın saati artık kendi **çerçevesindedir**: sorunun gününün gece yarısından dakikalarla `[lead, end)`.

- Çerçeve tam günlerdir. Sorunun haftasını (`[0, 7)` gün) ve kapanmanın değiştiği günleri (ilk günü ve son gününden sonraki gün)
  her yanda birer haftayla tutar. Değişim günü sorunun gününden en çok 92 gün uzaktaysa çerçeveye girer (**ufuk**).
- Çerçevenin içinde her günün gerçek kapanması vardır. Önceki gecenin pencereleri de: çerçevenin ilk gününden önceki günün gece
  penceresi ilk güne taşar.
- Çerçeveden önce ilk haftası, sonra son haftası yinelenir. Değişim günleri çerçevenin içinde olduğundan bu iki bölge haftalıktır:
  yineleme kesindir.
- Saatli kapanması olmayan kenarlar günlerini yineler, bir şey değişmez.

`Clock::framed` her kenarın adımlarını çerçevesi boyunca yazar. `forward`, `backward`, `reach_forward` ve `reach_backward` zamanı
çerçeveye taşıyarak adım adım ilerler (`Frame::place`, `place_before`). Kapanmanın günleri ve saatleri `access.rs`'in `closing`'inde
çerçeveyle birlikte hesaplanır.

**Saatli kapalı kenar:** soruyu ilgilendiren kenar, ufkun içinde (sorunun gününden 92 gün önce ve sonra) kapalı bir günü olan
kenardır. Sayılar (`byHours`) bu kurala göredir. Ufkun dışında başlayıp bitmiş ya da hiç başlamamış kapanma kenarı kapatmaz.

### 2. Hizmet alanını kesenler

Hizmet alanının cevabına `cut` eklendi: `{blocked, apart}`.

- `blocked`, reddedilen rotanın söylediği biçimdedir (ADR 0570, 0577): `edges` nedenleriyle, `barriers`, varsa `junctions`, `turns`
  ve `levels`.
- Kural:
  - Son aralığa kadar bir arama yapılır.
  - Alanın hiçbir ucundan girmediği her kenar parçası, alanın son aralığın içinde vardığı ucundan neyin durdurduğunu söyler:
    - o yönde kapalıysa kenar ve nedeni;
    - açıksa düğümün varılan yaydan dönüşü (cihazın terminalleri).
  - Alanın son aralığın içinde vardığı, hiçbir şeyin geçmediği düğüm kendini söyler: kapalı düğüm ya da engel. Arama bu düğümlere
    hiç girmez, varış kenarın girildiği uçtan hesaplanır.
  - Seviyesi yanındaki bir yeri ayıran, alanın vardığı düğüm de söylenir (§3).
  - Sıra, alanın onlara vardığı maliyettir; eşitlikte önce kenarlar ve ağın sırası. Her biri bir kez.
- `apart`: hiçbir tesisin parçasında olmayan kenarlar. Her kenar iki yönde, seviyelerin aralıkları da geçilir. Parçalar ağ
  kurulurken bir kez numaralanır (`Graph::find_parts`); bir soru yalnız tesislerin parçalarını toplar.

Söz: “alanı son aralığın içinde kesenler: kenar 5 (araç profiline kapalı), …; 26 kenar hiçbir tesise bağlanmayan parçalarda”.

- Hizmet alanı aracı sözü bir kez söyler: yalnız değişince yeniden.
- İşlemler'in Hizmet alanları aracı da sözü yazar.
- Python'da `ServiceAreas.cut` (`AreaCut`) aynı JSON'u verir.

Büyük ağda alanın vardığı uçları olmayan parçalar iki bakışla geçilir. Seviye denetimi yalnız seviye aralığı olan ağda yapılır.

### 3. Seviyenin kesmesi

Ağ kurulurken, tolerans içinde olup seviyeleri birbirini tutmayan yerler (köprünün üstü ve altındaki yol) **seviye aralığı**
olarak kaydedilir (`Graph::level_gaps`). Bunlar iki yönlü ve tekildir.

- **Ulaşılamayan:** en az kapatanlı yolun araması (ADR 0575) bir düğümden aralığın öbür yanına geçebilir. Bu bir kapatandır ve
  dönüş ya da düğüm geçişi sayılmaz. Söz: “kenar 26 ile kenar 27 aynı yerde ama seviyeleri ayrı”. JSON'da `levels` `[{from, to}]`.
  Başka artık yalnız ağın dışında kalan bir durak içindir.
- **Hizmet alanı:** §2'deki gibi, alanın vardığı düğüm ve varmadığı aralık komşusu, iki yerin ilk kenarlarıyla.

### 4. Bağlı katmanın kaynağı

Ağın bir katmanı bir dosyaya bağlıysa ve dosya, çalışma kopyası okunduktan sonra değiştiyse, ağ çizimdeki kopyadan kurulur ve bu
söylenir:

- Denetim `sources::stale_layers`'tadır: dosyanın SHA-256'sı bağın sürümünden farklıdır ya da dosya okunamaz. SHA-256, dosyanın
  damgası aynı kaldıkça bellekten gelir (ADR 0597); tekrar sormak ucuzdur.
- Söz: “Yollar: yol katmanının kaynağı çalışma kopyası okunduktan sonra değişti; ağ çizimdeki kopyadan kuruldu. Güncel ağ için
  önce Kaynak ▸ Kaynaktan oku.”
  - Masaüstünde ağın araçları sorduğunda söylenir, yalnız değişince yeniden.
  - İşlemler'in ağ araçlarında her çalışmada söylenir.
- Bağın sürümü değişince (Kaynaktan oku) masaüstünün ağı yeniden kurulur (`Stamp.sources`).
- Veritabanı kaynakları denetlenmez (Sonraya kalan).

### 5. Dış yönlendirici: gerekmiyor

`plan/gis.md`, bölgesel ölçek gerektirirse Valhalla türü bir dış yönlendiricinin ölçülerek seçilmesini istiyor. Ölçüldü:
1 200 600 parçalık bir bölge ağında (775 × 775 kavşak, 50 m) kurma, rota ve hizmet alanı etkileşimli bütçelerin içindedir
(Doğrulama'daki tablo). Dış yönlendirici eklenmez. Yeni bir bağımlılık, ayrı bir süreç ve ayrı bir veri hazırlığı, KentOS'un kendi
verisindeki kapanma, profil ve seviye kurallarını ikinci kez yazmak demekti.

## Doğrulama

- **Ortak durumlar:**
  - `fixtures/network/v1/timed-closures.json` 22 soru. Yenileri bir haftadan uzun dört yolculuk:
    - gelecek hafta iki tam gün kapanan geçit (süreyle ve uzunlukla);
    - hafta sonları gündüz kapanan iz;
    - yirminci günden sonra her gün öğle saatinde kapanan iz.

    Üç tek kenar da ufkun dışındaki ve içindeki kapanmaları sınar. Başvuru (`network_timed_closures_cases.py`) pencereleri her
    gün için, saat o güne vardıkça yeniden hesaplar. Haftaya katlamaz, ufku yalnız “saatli kapalı” sayısında kullanır.
  - `fixtures/network/v1/blocked.json` 20 soru ve 11 hizmet alanı:
    - rampa ile altındaki yolun seviye kesmesi ve köprünün devamı;
    - hizmet alanlarında profil, kare, kapalı düğüm, yasak dönüş, seviye, mola, saat, ücret, en az kapatan, engel ve engelin
      aralığın dışında kalması.

    Başvuru (`network_blocked_cases.py`) yerleri (köşe, seviye) olarak kurar, aralık adımını bir kapatan sayar, hizmet alanını
    kendi aramasıyla keser. KentOS kodu kullanmaz.
- **Testler:**
  - `kentos-geometry-core`'un `network_access` testleri (dördü bu iş);
  - `clock.rs`'in çerçeveli saat birim testi;
  - `kentos-processing`'in ağ durumları;
  - Python `test_network_blocked.py`, `test_network_timed_closures.py`.
- **Süreler** (bu makinede, release, `network::timing`):

  | İş | Şehir (100 800 parça) | Bölge (1 199 700 parça) | Bölgenin bütçesi |
  |---|---|---|---|
  | Kurma (bir kez, ağın iş parçacığında) | 126 ms | 1 735 ms | 2 s |
  | Bir noktanın konumu | 0,001 ms | 0,002 ms | 0,05 ms |
  | Bütün ağda Dijkstra | 7,6 ms | 122 ms | 150 ms |
  | Ağaçtan imlece yol | 0,007 ms | 0,037 ms | 0,1 ms |
  | Köşeden köşeye rota | 6,4 ms | 96 ms | 150 ms |
  | 10 dakikalık hizmet alanının çizgileri (kesenlerle) | 28,0 ms | 46 ms | 100 ms |
  | Aynısının alanları, pay 50 m | 179 ms | 300 ms | 500 ms |
  | 1 dakikalık alan, pay 50 m | 9,0 ms | 18,9 ms | 300 ms |
  | 4 dakikalık alan, pay 50 m | 47 ms | 59 ms | 300 ms |
  | Denetle | 80 ms | 981 ms | 2 s |

  İlk ölçümde hizmet alanını kesenler ayrı bir aramayla bulunuyordu: şehirde çizgiler 26 ms'den 36 ms'ye çıktı. Kesenler artık
  çizgilerin aramasından okunuyor (tesisler birleşikken tek arama). Alanın uçlarına varmadığı parçalar iki bakışla geçiliyor:
  çizgiler 28 ms'ye indi. Bölgede 1 dakikalık alan 29 ms'den 19 ms'ye indi.

  İzleme (ilk 105 ms, sonraki 25 ms) bu işten önce de şehrin bütçesinin (100 ve 20 ms) biraz üstündeydi. Bu iş ona dokunmaz.
