# ADR 0235: Hidroloji

- **Durum:** kabul edildi (2026-10-09). Kapsam sahibin sözleridir (9 Ekim): “yeni bir branch açarak GIS-31 ve GIS-36 aralığını
  yapacağız”; “yüksek performans ilk önceliğimiz”; “Birleştirmeyi en sona bırakacağız sana söylediğim tüm maddeleri bitirelim”;
  hidroloji için “Bu konu çok önemli iyi araştır”; MADDE-TARIFI.md'nin sırası. Madde tek parçada biter; dal
  `gis-31-36-raster-analysis`. İki platformda.
- **Bağlam belgesi:** TODOS.md `GIS-35`, ADR 0231 (raster çözümleme altyapısı; Horn'un gradyanı, coğrafi rasterde satırın metresi;
  topografik nemlilik indisi buraya bırakıldı), ADR 0233 (operasyon işi, değer okuma, sonucun yeri), ADR 0234 (bölgelerin halkaları,
  Douglas–Peucker, kirişlerin geçtiği hücreler, nesne çıktıları), ADR 0188 (güzergâhta km), ADR 0204 (raster nesnesi, rampalar).

## Bağlam

DEM'den suyun yolunu çıkarmak harita mühendisliğinin sık işidir: drenaj projesinin havzası, menfez ve köprünün beslenme alanı, dere
yatağının sayısallaştırılması, ıslak alanların ön tespiti. Netcad bunları NETHYDRO'da verir: proje alanının bütün havzaları tek
düğmeyle, bir güzergâhı kesen bütün akış ağlarının havzaları, noktaya göre havza; ayrıca Analist'te özgül havza alanı. ArcGIS'in
Hydrology araçları Fill, Flow Direction (D8, MFD, DINF), Flow Accumulation, Snap Pour Point, Watershed, Basin, Stream Link, Stream
Order ve Stream to Feature'dır. QGIS bunları SAGA ve GRASS araçlarıyla verir (Fill sinks (Wang & Liu), r.watershed, r.terraflow,
r.water.outlet, Topographic Wetness Index); TauDEM, WhiteboxTools ve RichDEM aynı işlerin yaygın açık kaynaklı uygulamalarıdır.
Taşkın, yağış ve debi hesapları doğrulanmış ayrı modüldür (`CITY-15`); bu ADR yalnız arazinin geometrisinden gelenleri tanımlar.

Yöntemler yayımlanmış kaynaklarından seçildi:
- **Çukur doldurma:** öncelik kuyruklu taşma (Barnes, Lehman ve Mulla 2014, *Computers & Geosciences* 62: 117–127) ve kuyruğu
  küçülten türevi (Zhou, Sun ve Fu 2016, *Computers & Geosciences* 90: 87–96); en küçük eğimli doldurma Wang ve Liu'nun (2006)
  kuralıdır.
- **Düzlükler:** Barnes, Lehman ve Mulla 2014b (*Computers & Geosciences* 62: 128–135); yüksekten uzaklaşan ve alçağa yaklaşan iki
  gradyan, ikincisi iki kat ağırlıkla, Garbrecht ve Martz'ın (1997) yinelemesiz biçimi.
- **Akış yönleri:**
  - D8: O'Callaghan ve Mark 1984; Jenson ve Domingue 1988; ArcGIS'in kodları.
  - D∞: Tarboton 1997.
  - Çoklu yön: Quinn ve ark. 1991'in kontur uzunluklarıyla; üs sabit ya da Qin ve ark.'nın (2007, *IJGIS* 21(4): 443–458)
    uyarlananı. ArcGIS Pro'nun MFD'si budur.
- **Dere sırası:** Strahler (1957) ve Shreve (1966).
- **Nemlilik indisi:** Beven ve Kirkby 1979; düz hücrenin taban eğimi CSIRO'nun ulusal TWI ürününün (DEM-H) %0,1'i.

Bağımsız çapraz denetim için GRASS GIS 8.4 kullanılır: değersiz hücresi olmayan DEM'de r.terraflow'un doldurduğu yüzey bizimkiyle
hücre hücre aynıdır, iç hücrelerde tek en dik yönlü D8 aynıdır (§13).

## Karar

### 1. Kapsam

İşlemler'in yeni Hidroloji kategorisinde sekiz araç:

| Araç | Kimlik | Çıktı |
|---|---|---|
| Çukur doldur | `hydrology.fill` | raster: doldurulmuş yükseklik ya da dolgu derinliği |
| Akış yönü | `hydrology.flowDirection` | raster: D8 kodları (bayt) |
| Akış birikimi | `hydrology.flowAccumulation` | raster (32 bit) |
| Topografik nemlilik indisi | `hydrology.wetness` | raster (32 bit) |
| Döküm noktası | `hydrology.pourPoint` | noktalar (yaklaştırılmış döküm noktaları) |
| Noktadan havza | `hydrology.watershed` | alanlar |
| Havzalar | `hydrology.basins` | alanlar: ana havzalar, dere kollarının alt havzaları ya da güzergâhı kesen derelerin havzaları |
| Dere ağı | `hydrology.streams` | çoklu çizgiler, Strahler ve Shreve sıralarıyla |

Her araç yükseklik modelini (DEM) alır. Çukur doldur dışındakiler önce çukurları doldurur (Çukurları doldur, varsayılan açık; §3,
en küçük eğim 0) ve akış yönlerini kendisi bulur. Ara rasterler (doldurulmuş DEM, akış yönü, birikim) yazılmadan tek çalıştırmada
biter.

### 2. Ortak kurallar

- **Değer, yer, hücre:** ADR 0233 §2'deki gibi: bandın örneği; NaN, nodata ve alfası 0 olan piksel değersizdir; hücre (i, j)'nin
  merkezi (i + ½, j + ½).
- **Komşular** bu sırayla: (i + 1, j), (i + 1, j + 1), (i, j + 1), (i − 1, j + 1), (i − 1, j), (i − 1, j − 1), (i, j − 1),
  (i + 1, j − 1). Kuzeyi yukarıda rasterde bunlar doğu, güneydoğu, güney, güneybatı, batı, kuzeybatı, kuzey ve kuzeydoğudur. Dik
  komşular 1., 3., 5. ve 7.'dir. Değeri olan, rasterin içindeki komşu geçerlidir.
- **Uzunluk ve alan:** akışın çıktığı hücrenin satırının eksenleriyle (ADR 0231 §2: projeksiyonlu rasterde afin, coğrafi rasterde
  satırın GRS80 metresi; `[a, b, c, d]`). Komşuya adım (Δi, Δj) (a·Δi + b·Δj, c·Δi + d·Δj) olur, uzunluğu d = √(x² + y²), float64'te
  bu sırayla. Hücrenin alanı |a·d − b·c|, genişliği √(alan).
- **Çıkış hücresi:** rasterin kenarındaki ya da bir komşusu değersiz olan değerli hücre. Su rasterin dışına ya da değersiz hücreye
  akabilir (RichDEM ve TauDEM gibi; GRASS'ın r.terraflow'u içerideki değersiz hücreleri doldurur, ArcGIS bunu belgelemez).
- **Sınır:** bütün raster bellekte çalışılır (float64): en çok 2²⁵ hücre (33 554 432; hücre başına en çok yaklaşık 30 bayt). Aşınca
  ret, çaresiyle (Yeniden örnekle, Maskeyle kırp).
- **Sonuçlar:** raster sonuçlar ADR 0231 §2'nin dosyası ve nesnesidir. Raster, noktalar, alanlar ve çizgiler DEM'in katmanının hemen
  üstündeki yeni katmana yazılır (`above`), tek adımda.

### 3. Çukur doldur

- **Girdi:** DEM ve bant; En küçük eğim (yüzde; varsayılan 0, 0–100); Sonuç: Doldurulmuş yükseklik (varsayılan) ya da Dolgu
  derinliği.
- **Tanım:** ε = eğim / 100. Doldurulmuş yüzey f, f ≥ z olan ve çıkış hücresi olmayan her değerli hücresinin
  f(n) + ε·d(c, n) ≤ f(c) olan geçerli bir komşusu bulunan en küçük yüzeydir:

  f(c) = max(z(c), minₙ (f(n) + ε·d(c, n)))   (çıkış hücresi olmayan c; çıkış hücresinde f = z)

  çarpma ve toplama float64'te bu sırayla. En küçük çözüm tektir; hesabın sırası sonucu değiştirmez.
  - ε = 0, Wang ve Liu'nun (2006) ve Planchon ile Darboux'nun (2002) çukursuz yüzeyidir: her hücre taşma yüksekliğine (çıkışa giden
    yolların en yüksek noktalarının en küçüğüne) yükselir; değerler girdinin değerlerindendir.
  - ε > 0'da çukurlar ve düzlükler en az ε eğimle çıkışa doğru alçalan yüzey olur; ε'dan yatık doğal yamaçlar da yükselir (SAGA'nın
    Fill Sinks (Wang & Liu)'su gibi).
- **Hesap:** öncelik kuyruklu taşma. Çıkış hücreleri kuyruğa girer, en alçağı çıkarılır.
  - ε = 0'da bir hücre ilk ulaşıldığında belirlenir: f = max(z, düzey). Düzeyin altındaki çukur hücreleri FIFO kuyrukla taşırılır
    (Barnes 2014). Düzeyin üstündeki yamaç hücresinin ziyaret edilmemiş komşuları hep kendisinden yüksekse yamaç hemen izlenir;
    yoksa kendi yüksekliğiyle kuyruğa girer (Zhou ve ark. 2016'nın fikri). Yamaç hücresinin f'si z'sidir; izlemek yalnız daha yüksek
    komşuları kapatır, sonucu değiştirmez.
  - ε > 0'da hücre daha alçak bir adayla yeniden belirlenir (Knuth'un genelleştirdiği Dijkstra; aday hiçbir zaman çıkarılandan alçak
    değildir).
  - Kuyruk taban-2 öbeğidir (radix heap): float64 yükseklikler sırası korunan 64 bitlik anahtarlarla tutulur, çıkarılanlar tekdüze
    artar.
- **Sonuç:** Doldurulmuş yükseklik: girdi 64 bit ondalıksa 64 bit, değilse 32 bit ondalık. Dolgu derinliği f − z'dir, 32 bit.
  Değersiz hücre değersiz kalır. Görünüş: yükseklikte girdininki, derinlikte Viridis (en küçükten en büyüğe).

### 4. Akış yönü

- **Girdi:** DEM ve bant; Çukurları doldur (varsayılan açık); Kodlama: ESRI (varsayılan; komşuların sırasıyla 1, 2, 4, 8, 16, 32, 64,
  128) ya da 1–8 (TauDEM'inki: 1 doğu, 2 kuzeydoğu, 3 kuzey, 4 kuzeybatı, 5 batı, 6 güneybatı, 7 güney, 8 güneydoğu).
- **D8:** yüzeyde (doldurulmuş ya da girdinin kendisi) hücre, geçerli komşular arasında s = (f(c) − f(n)) / d(c, n) > 0'ı en büyük
  olana akar. Eşitlerde komşuların sırasıyla ilki seçilir (ArcGIS eşitlikte komşuluğu genişletir; belgelenmiş bir kural olmadığı
  için burada sıra kesindir).
- **Çıkış hücresi**nin alçak komşusu yoksa dışarı akar: komşuların sırasıyla rasterin dışına ya da değersiz hücreye düşen ilk yön.
- **Düzlükler** (Barnes, Lehman ve Mulla 2014b): alçak komşusu olmayan, çıkış olmayan değerli hücre düzlük hücresidir.
  - *Alçak kenar:* yönü olan ve aynı yükseklikte bir düzlük hücresine komşu olan hücre. *Yüksek kenar:* daha yüksek geçerli komşusu
    olan düzlük hücresi.
  - *Düzlük:* alçak kenarlardan aynı yükseklikteki hücrelerin 8 bağlı kümesi. Alçak kenarı olmayan düzlük akaksızdır, yönsüz kalır.
  - *Yüksekten uzaklık* u(c): düzlüğün yüksek kenarlarından (u = 1) düzlük hücreleri üstünden adım sayısı; U düzlükteki en büyük u.
    Yüksek kenardan ulaşılamayan hücrede yükseklik payı 0, ulaşılanda U − u(c).
  - *Alçağa uzaklık* l(c): alçak kenarlardan (l = 1) düzlük hücreleri üstünden adım sayısı.
  - *Maske* m(c) = 2·l(c) + yükseklik payı. Düzlük hücresi, aynı düzlükteki komşularından maskesi kendisininkinden küçük olanlardan
    maskesi en küçüğüne akar; eşitlerde komşuların sırasıyla ilki (makalenin Algoritma 7'si). Böyle bir komşu her zaman vardır:
    komşuların l'leri birden, payları birden çok farklı olmaz.
- **Tek hücrelik çukur:** ArcGIS'in Flow Direction'ı kendisi doldurur. Burada Çukurları doldur kapalıysa her çukur yönsüz kalır.
- **Kod 0:** yönü olmayan hücre (dolgu kapalıyken çukur ya da akaksız düzlük). Değersiz hücre 255.
- **Görünüş:** Spektral rampa, el ile 1–128 (1–8 kodlamasında 1–8), en yakın örnekleme.

### 5. Akış birikimi

- **Girdi:** DEM ve bant; Çukurları doldur; Yöntem: D8 (varsayılan), D∞ (Tarboton 1997) ya da Çoklu yön; Çoklu yönün üssü (0:
  uyarlanan, varsayılan; 0,1–100: sabit); Birim: Hücre sayısı (varsayılan), Alan (m²) ya da Özgül havza alanı (m).
- **Birikim:** A(c) = w(c) + Σₙ pay(n → c)·A(n): hücrenin kendi payı w (Hücre sayısında 1, öbürlerinde hücrenin alanı) ve
  yukarısındaki hücrelerin payları, komşuların sırasıyla toplanarak (float64).
  - Hücre kendini sayar (TauDEM ve GRASS gibi). ArcGIS'in Flow Accumulation'ı saymaz: onunki bir eksiktir.
  - Özgül havza alanı A / genişlik (hücrenin genişliği, §2).
- **D8:** pay(n → c) = 1, n'nin yönü c ise (§4).
- **Çoklu yön:** alçak komşuları olan hücrenin payı pay(c → n) = Lₙ·tₙᵖ / Σₘ Lₘ·tₘᵖ.
  - tₙ = §4'ün eğimi; toplam alçak komşular üstünden, komşuların sırasıyla.
  - Lₙ Quinn ve ark.'nın (1991) kontur uzunluğudur: dik komşuda 0,5, çaprazda 0,354.
  - Üs p ya sabittir (p = 1 Quinn'inki) ya da uyarlanır: p = 8,9·min(e, 1) + 1,1, e hücrenin en büyük tₙ'si (Qin ve ark. 2007; ArcGIS
    Pro'nun MFD'si).
  - Alçak komşusu olmayanın (düzlük, çıkış) bütün payı §4'ün yönüne gider (dışarı akanınki dışarı).
- **D∞:** hücrenin sekiz üçgen yüzü sırasıyla doğu–kuzeydoğu, kuzey–kuzeydoğu, kuzey–kuzeybatı, batı–kuzeybatı, batı–güneybatı,
  güney–güneybatı, güney–güneydoğu ve doğu–güneydoğudur. Her yüzde birinci komşu dik (e₁), ikincisi çaprazdır (e₂); ikisi de geçerli
  olmalı.
  - d₁ hücreden e₁'e, d₂ e₁'den e₂'ye uzunluktur (ikisi de hücrenin satırının eksenleriyle).
  - s₁ = (f₀ − f₁)/d₁, s₂ = (f₁ − f₂)/d₂, r = atan2(s₂, s₁), s = √(s₁² + s₂²).
  - r < 0 ise r = 0, s = s₁; r > α = atan2(d₂, d₁) ise r = α, s = (f₀ − f₂)/√(d₁² + d₂²).
  - En büyük s > 0'lı yüz (eşitlerde ilki) akışın yüzüdür: payın r/α'sı e₂'ye, 1 − r/α'sı e₁'e gider.
  - Böyle yüz yoksa bütün pay §4'ün yönüne gider (TauDEM düzlüğü yükseklikleri artırarak çözer; burada D8'in düzlük kuralıyla).
- **Sonuç:** 32 bit (2²⁴'ten büyük hücre sayıları en yakın float32'ye yuvarlanır). Görünüş Viridis, Yüzde gerdirme (dere ağı
  belirgin).

### 6. Topografik nemlilik indisi

- **Girdi:** DEM ve bant; Çukurları doldur; Yöntem (Çoklu yön varsayılan, D∞, D8) ve Çoklu yönün üssü; En küçük eğim (yüzde;
  varsayılan 0,1).
- **Tanım:** TWI = ln(a / tan β), Beven ve Kirkby'nin (1979) indisi.
  - a §5'in özgül havza alanıdır (m).
  - tan β yüzeyde Horn'un gradyanının büyüklüğüdür, √(gx² + gy²) (ADR 0231 §3: kenarda ve değersiz komşuda merkezin değeri).
  - tan β en küçük eğimin (yüzde / 100) altındaysa en küçük eğim kullanılır. TauDEM düz hücreyi değersiz bırakır; varsayılan %0,1
    CSIRO'nun DEM-H TWI ürünündeki tabandır.
- **Sonuç:** 32 bit. Görünüş Mavi-kırmızı ters (kuru kırmızı, ıslak mavi), Yüzde gerdirme.

### 7. Döküm noktası

- **Girdi:** DEM ve bant; Noktalar (nokta nesneleri, çok noktalının her noktası; Katman, Seçili, Görünen, Tümü); Çukurları doldur;
  Yaklaştırma uzaklığı (m; varsayılan 0).
- **Döküm hücresi:** noktanın hücresi. Yaklaştırma r > 0 ise merkezinin noktaya uzaklığı r'yi aşmayan değerli hücrelerden D8 birikimi
  (hücre sayısı) en büyük olan seçilir; eşitse noktaya en yakın, o da eşitse satır satır ilki (ArcGIS'in Snap Pour Point'i). Uzaklık
  metredir: noktadan hücrenin merkezine hücre uzayındaki fark noktanın satırının eksenleriyle (§2) çevrilir. Aday hücresi olmayan nokta
  atlanır, söylenir.
- **Sonuç:** her nokta için döküm hücresinin merkezinde bir nokta. Öznitelikler:
  - Nokta: girdideki sırası, 1'den.
  - Birikim: D8 hücre sayısı.
  - Alan: m², D8 birikiminin alanı.
  - Uzaklık: noktanın taşındığı uzaklık, m.

### 8. Noktadan havza

- **Girdi:** §7'ninkiler.
- **Havza:** D8 akış yolu döküm hücresinden geçen hücreler. Hücre, yolundaki ilk döküm hücresinin havzasındadır: yukarıdaki nokta
  aşağıdakinin havzasından kendi havzasını ayırır (ArcGIS'in Watershed'i gibi). İki nokta aynı hücreye düşerse hücre girdide sonra
  gelenindir (ArcGIS gibi); öbürünün havzası boştur, söylenir.
- **Sonuç:** her havza bir alandır, ADR 0234 §4'ün halkalarıyla (8 komşu: bir D8 havzası her zaman 8 bağlıdır). Öznitelikler: Havza
  (noktanın girdideki sırası, 1'den) ve Alan (m², hücre alanlarının satır satır toplamı).

### 9. Havzalar

- **Girdi:** DEM ve bant; Çukurları doldur; Biçim; Eşik alanı (Alt havzalarda ve güzergâhta, §10); Güzergâh (güzergâhta: çizgi, çoklu
  çizgi, yay; Katman, Seçili, …); En küçük alan (m²; varsayılan 0: hepsi).
- **Ana havzalar** (varsayılan; Netcad'in “bütün havzalar”ı, ArcGIS'in Basin'i):
  - Her hücre D8 akış yolunun bittiği hücreye göre ayrılır: dışarı akan çıkış hücresi ya da yönü olmayan hücre.
  - Havzalar bitiş hücrelerinin satır satır sırasıyladır; tek hücrelik havza da havzadır.
  - Öznitelikler: Havza, Alan.
- **Alt havzalar** (ArcGIS'te Watershed'in dere kollarıyla):
  - §10'un her dere kolunun hücreleri ve D8 yolu dereye o koldan giren hücreler; dereye ulaşmayan hücre alt havzasızdır.
  - Öznitelikler: Bağ (kolun numarası), Sıra (Strahler), Alan.
- **Güzergâhı kesen dereler** (Netcad'in “güzergâhı kesen akış ağlarının havzaları”; menfez ve köprü):
  - *Geçen hücreler:* güzergâhın 0,1 mm'lik kirişlerinin geçtiği hücreler (ADR 0234 §3'ün açık şekil kuralı).
  - *Geçiş:* geçilen bir dere hücresidir (§10), ama D8 yönünün gösterdiği hücre geçilen bir dere hücresi değilse. Böylece dere
    boyunca giden yol tek geçiş sayılır, en aşağıdaki hücresiyle.
  - *Havza:* geçişin bütün havzasıdır, D8 yolu geçiş hücresinden geçen bütün hücreler. Aşağıdaki geçişin havzası yukarıdakini içerir:
    alanlar örtüşebilir; menfezin beslenme alanı budur.
  - Geçişler güzergâhın girdideki sırasıyla, sonra güzergâh boyunca km'leriyle (eşitse satır satır) numaralanır.
  - Öznitelikler: Havza, Km (geçiş hücresinin merkezinin güzergâhtaki yeri, ADR 0188'in okuması, m), Sıra (geçilen kolun Strahler'i),
    Alan.
- En küçük alandan küçük havza yazılmaz, sayısı söylenir. Kalanlar sırasıyla 1'den numaralanır (alt havzalarda Bağ kolun
  numarasıdır).
- **Sonuç:** alanlar (halkalar §8'deki gibi).

### 10. Dere ağı

- **Girdi:** DEM ve bant; Çukurları doldur; Eşik alanı (m²; 0: en büyük D8 birikiminin (alan) yüzde biri, en büyük / 100);
  Sadeleştirme (hücre; varsayılan 0: yalnız doğrusal köşeler atılır).
- **Dere hücresi:** D8 birikimi (alan, m²) eşikten küçük olmayan hücre. Dere hücresinin aşağısı da deredir (birikim aşağı doğru
  artar).
- **Kollar (bağlar):** kaynakta (yukarısında dere hücresi olmayan) ya da kavşakta (iki ya da daha çok dere hücresi ona akan) başlar.
  D8 yönünce sürer; bir sonraki kavşaktan önce ya da yolun sonunda (dışarı akış, yönü olmayan hücre) biter. Kollar başlangıç
  hücrelerinin satır satır sırasıyla 1'den numaralanır.
- **Çizgi:** kolun hücrelerinin merkezleri, akış yönünde. Kavşakta biten kol, aktığı kolun ilk hücresinin merkezine uzanır (ağ bağlı
  kalır). Köşeler ADR 0234 §5'in Douglas–Peucker'ıyla sadeleşir (açık yol, hücre uzayında; uçlar kalır; 0 yalnız doğrusal köşeleri
  atar).
- **Sıralar:**
  - Strahler: kaynak 1; kavşakta gelenlerin en büyüğü k ve k'li iki ya da daha çok kol geliyorsa k + 1, yoksa k.
  - Shreve: kaynak 1, kavşakta gelenlerin toplamı.
- **Öznitelikler:**
  - Bağ, Sıra (Strahler), Shreve.
  - Uzunluk: m, adımların uzunlukları, kavşağa uzanan dahil, sırayla toplanır.
  - Düşü: m, çizginin ilk hücresinin yüzey yüksekliği eksi son hücresininki.
  - Eğim: Düşü / Uzunluk.
  - Alan: m², kolun son hücresinin birikimi.
  - Aşağı: aktığı kolun numarası; yoksa 0.
- **Sonuç:** çoklu çizgiler, yeni katmanda.

### 11. Arayüz

- **Araçlar** iki platformda aynı adlar, parametreler ve varsayılanlarla; **komutlar** `processing.run.<kimlik>`. **Şerit:** CBS'nin
  Raster sekmesinde Hidroloji paneli.
- **Takma adlar:** CUKURDOLDUR, FILLSINKS; AKISYONU, FLOWDIR; AKISBIRIKIMI, FLOWACC; NEMLILIK, TWI; DOKUMNOKTASI, SNAPPOUR; HAVZABUL,
  NOKTAHAVZA, WATERSHED; HAVZALAR, ALTHAVZA, BASINS; DEREAGI, STREAMORDER.

### 12. Performans

- DEM tek okumada belleğe alınır; blokları iş parçacıklarında çözülür.
- ε = 0'da taşma şeritlerde paraleldir (Barnes 2016, *Environmental Modelling & Software* 81: 191–201): raster satır şeritlerine
  bölünür; her şerit kendi kenar satırlarının hücrelerini kendi çıkışları sayarak ayrı taşırılır, her hücre ulaşıldığı çıkışın
  etiketini alır. Etiketlerin buluşma yükseklikleri (şeridin içinde ve şeritlerin dikişlerinde) küçük bir çizge kurar; gerçek
  çıkışlardan en küçük taşma yükseklikleri (yolun en yüksek buluşmasının en küçüğü) o çizgede bir öncelik kuyruğuyla bulunur; hücre
  etiketinin taşma yüksekliğine yükselir. En küçük çözüm tek olduğundan sonuç tek taşmanınkiyle hücre hücre aynıdır. Tek iş parçacığında
  ve ε > 0'da taşma adım adımdır: adımda en çok 2²¹ hücre, ilerleme ve Durdur çalışır. Kuyruğa yalnız yamaç sınırları girer.
- D8 ve paylar satırlarda paralel hesaplanır; düzlükler düzlük başına iş parçacıklarında, iki BFS ve maske atomik dizilerle.
- Birikim seviyeli Kahn'la hesaplanır: yukarısı bitmiş hücreler aynı seviyede, 4096 ve daha çok hücreli seviyeler iş parçacıklarında
  (giriş dereceleri atomik azalır). Her hücre toplamını komşularından kendi çeker, komşuların sırasıyla: sonuç iş parçacığı sayısına
  bağlı değildir. Çoklu yönün payları hücre başına bir kez hesaplanır (tᵖ = exp(p·ln t), p = 1'de t), en dik komşu bayt olarak tutulur.
- Sonuçların yazılması paraleldir.
- **Bütçeler** (release, geliştirme makinesi, 4096² DEM):

| İş | Masaüstü (8) | Web (işçi) |
|---|---|---|
| Çukur doldur | ≤ 1,5 s | ≤ 6 s |
| Akış yönü | ≤ 2 s | ≤ 8 s |
| Akış birikimi, D8 | ≤ 2,5 s | ≤ 10 s |
| Akış birikimi, Çoklu yön | ≤ 3,5 s | ≤ 16 s |
| Akış birikimi, D∞ | ≤ 3,5 s | ≤ 16 s |
| Topografik nemlilik indisi | ≤ 4 s | ≤ 18 s |
| Noktadan havza, 10 nokta | ≤ 2,5 s | ≤ 10 s |
| Havzalar, ana havzalar | ≤ 3 s | ≤ 12 s |
| Dere ağı | ≤ 3 s | ≤ 12 s |

### 13. Doğrulamanın ilkesi

Bağımsız başvuru (KentOS kodu olmadan, ADR'den Python'la) her aracın sonucunu hücre hücre verir. Yalnız libm'e bağlı değerler (Çoklu
yönün üssü, D∞'nin açısı, TWI'nın logaritması) float32'de en çok bir son basamak birimi farkla karşılaştırılır. GRASS GIS 8.4 çapraz
denetler:
- değersiz hücresi olmayan DEM'lerde r.terraflow'un doldurduğu yüzey;
- iç hücrelerde tek en dik yönlü D8;
- en küçük eğimle doldurulup düzlüksüz ve çukursuz yapılmış bir yüzeyde r.watershed -s'nin D8 birikimi, kenara bağlı olmayan iç
  hücrelerde (r.terraflow böyle yüzeyde çöker);
- yönlerimiz r.watershed'in kodlamasına çevrilince r.water.outlet'in havzaları.

## Uygulama

- **Raster çekirdeği** `kentos-raster`'ın `hydro`'su:
  - `surface` (`Surface`: değerler float64, satırların eksenleri, adımları, alanı ve genişliği (§2), komşular, çıkışlar; `N`,
    `NOFLOW`, `NONE`), `heap` (taban-2 öbeği, sırası korunan anahtarlar), `fill` (`Filling`: ε = 0'da taşma, çukur FIFO'su ve yamacın
    izlenmesi; ε > 0'da en küçük sabit noktalı Dijkstra; adım adım), `tiled` (şeritlerde paralel doldurma, etiketlerin çizgesi,
    taşma yükseklikleri), `flow` (`d8`, `receiver`, `resolve_flats`: düzlükler düzlük başına paralel), `accum` (`Routing`: D8, Çoklu
    yön ve D∞'nin payları; `Accumulating`: seviyeli Kahn, adım adım), `basins` (yaklaştırma, yukarıya etiketleme, alanlar, halkalar,
    geçen hücreler, bitiş hücreleri), `streams` (kollar, Strahler ve Shreve, çizgiler ve uzunlukları), `mod` (`HydroTool`, `HydroNotes`,
    `HydroWork`: okuma, doldurma, yönler, birikim, sonuç ve şeritlerin yazılması aşamalarıyla).
  - `ops`'un sekiz türü (`fill`, `flowDirection`, `flowAccumulation`, `wetness`, `pourPoint`, `watershed`, `basins`, `streams`),
    `Work::Hydro`, görünüşler ve `Notes.hydro`; `vector`'ün `Features.fields` ve `numbers`'ı (nesnenin adlı sayıları); `par::each_mut`.
- **WASM** `raster-wasm`: `featureFields`, `featureNumbers`, notlarda `hydro`.
- **İşlemler (Rust)** `builtin::hydrology` (`mod.rs`: raster ve nesne sonuçlarının koşucuları, özniteliklerin metni; `tools.rs`: sekiz
  araç), `hydrology` kategorisi. Ortak durumlar `fixtures/processing/v1/hydrology.json` ve `.kcad`, rasterleri `hydrology/*.tif`
  (`hydrology_processing_cases.py`; anahtar `hydrologyOf`), oynatıcı `tests/cases/surface.rs`'in `the_hydrology_cases_do_what_they_say`'i.
- **Masaüstü:** `catalog.rs`'in `PORTED`'ı, `ported.json`; testler `processing/hydrology_tests.rs`, resimler `hydrology_scenes.rs`
  (`tools_screens`'in `hid-*`); sahne `fixtures/interaction/v1/hydrology.kcad` (`hydrology_scene.py`: vadinin yükseklik modeli, doğu
  derelerini kesen yol ekseni, iki çıkış noktası).
- **Web:** `io/rasterAnalysisProtocol.ts`'in `AnalysisFeatures.fields` ve `numbers`'ı, işçi ve `processing/surfaceTesting.ts` aynı
  biçimde; araçlar `processing/builtin/hydrology/shared.ts` ve `tools.ts`; kategori; şeridin `RASTER_ANALYSIS`'i Hidroloji'yle; ikonlar.
  Testler `processing/cases.test.ts`'in hidroloji bloğu, `io/raster.wasm.test.ts`'in hidroloji bloğu (test içindeki TIFF yazıcısı
  64 bitlik ondalık bandı da yazar); ölçüm `scripts/perf/raster.mjs --only hydro`; resimler `shots.mjs`'in `hydrology` grubu.
- **İkonlar** (sorulmadan seçildi): `fillSinks` (çanak, içinde su ve düzeyi, üstünde inen ok), `flowDirection` (soluk ızgarada
  güneydoğuya tek ok), `flowAccumulation` (ızgarada akış aşağı koyulaşan hücreler: iki kaynaktan çıkışa), `wetness` (zeminde damla),
  `pourPoint` (derenin ucunda hedef), `watershed` (havzanın sınırı, içinde bir noktada birleşen kollar), `basins` (bölünmüş üç
  havza), `streams` (dallanan dere ağı; Hidroloji kategorisinin de).

## Doğrulama

- **Bağımsız başvurular** (KentOS kodu yok):
  - `scripts/fixtures/hydrology_cases.py --check`: sekiz aracın kuralları ADR'den Python'la, 141 durum. On DEM: vadi (çukur ve teras),
    teraslar (düzlükler), değersiz hücreli, dikdörtgen hücreli, dönük, coğrafi, 16 bitlik tam sayı (nodata), 64 bitlik, koni (eşit
    düşüler, çukursuz) ve dendritik drenaj ağı. Her DEM'de doldurma (yükseklik, derinlik, en küçük eğim), yön (dolgulu, dolgusuz,
    TauDEM), sekiz birikim (D8, Çoklu yön sabit ve uyarlanan üsle, D∞; üç birim), üç nemlilik indisi; ağda döküm noktaları ve havzalar
    (0 ve 25 m), ana ve alt havzalar, en küçük alanın düşürdükleri, dere ağı üç eşikle ve iki sadeleştirmeyle, güzergâhı kesen dereler,
    dolgusuz dere ağı; dört ret.
  - GRASS GIS 8.4 çapraz denetimi (aynı betik, §13): 8100 doldurulmuş hücre, 5859 D8 yönü, 2494 birikim ve 2518 havza hücresi aynı.
  - `scripts/fixtures/hydrology_processing_cases.py --check`: İşlemler'in 20 ortak durumu; rasterleri GDAL yazar, okunup başvurunun
    girdisiyle karşılaştırılır; yazılan dosyalar başvurunun örnekleriyle durumun kuralıyla (`hydrologyOf`); özetlerin en büyük
    değerleri başvurunun 64 bitlik hesabından.
  - `scripts/fixtures/hydrology_scene.py --check`: resimlerin çizimi.
- **Kurallar:** D8'e dayananlar bit bit; libm'e bağlı değerler float32'de en çok bir son basamak birimi; güzergâhın km'si 10⁻⁹ m içinde.
  Çekirdek her durumu bir ve üç iş parçacığında oynatır: şeritli ve tek taşma, paralel ve tek birikim aynı sonucu verir.
- **Testler:** `kentos-raster` hidroloji testi (141 durum, iki iş parçacığı sayısıyla) ve 4 birim testi (öbeğin sırası, şeritli
  doldurmanın tek taşmayla bit bit aynılığı dört DEM'de 2, 3 ve 8 iş parçacığıyla, D∞ yüzlerinde e₁'den e₂'ye adımın tablosu);
  `kentos-processing` ortak durumların 20'si ve varsayılanlar; masaüstü `hydrology_tests` 3 (dere ağı modelin katmanının hemen
  üstünde, tek adımda geri alınır; birikimin dosyası ve rasteri; iki çıkışın havzaları ve yolun geçişleri km sırasıyla), araç sayısı
  68; web `cases.test.ts`'in 22'si, `raster.wasm.test.ts`'in 142'si (141 durum WASM modülünde), `processing.test.ts` (kategori
  ağacında Hidroloji), şeridin panelleri, pencere formları (`dialog.json`'a sekiz form eklendi, başka satır değişmedi).
- **Tarayıcıda uçtan uca:** `shots.mjs hydrology` her sahnede aracı İşlemler penceresinden çalıştırır (10 sahne, iki tema, iki boy);
  masaüstü aynı yerlerde aynı değerlerle (`hid-*`); iki platform aynı sonucu verir (dere ağı 115 kol, yolun geçişlerinin havzaları 3).
- **Araştırmada ve denetimde öğrenilenler:** ArcGIS'in Flow Accumulation'ı hücrenin kendisini saymaz, Flow Direction'ı tek hücrelik
  çukuru kendisi doldurur (§4, §5); GRASS'ın r.terraflow'u içeride float32 çalışır (DEM'ler float32'de tam değerlerle yazıldı), içerideki
  değersiz hücreleri doldurur ve çukursuz yüzeyde çöker (8.4.2); r.watershed az saydığından emin olmadığı birikimi eksi işaretler;
  TauDEM düz hücrenin TWI'sını değersiz bırakır, CSIRO'nun DEM-H'si %0,1 taban kullanır.
- **Görsel ve kapsam incelemesinde düzeltilenler:** başvurunun hiçbir durumu En küçük alan'la havza düşürmüyordu (600 m²'lik havza
  sınırda kalıyordu); düşüren üç durum (601 m², alt havzalarda 500 m², güzergâhta 20 000 m²) ve çukursuz koni eklendi. Özetlerde “0
  hücre dolduruldu” “Doldurulacak çukur yok”, yönsüz hücreler nedenleriyle yazıldı. Akış yönü ikonu 16 px'te dört okla karışıyordu
  (tek ok), Havzalar ikonu topa benziyordu (üç bölge), Akış birikimi'ninki Dere ağı'nınkine (ızgaranın hücreleri). Güzergâh
  sahnesinin görünümü DEM'in doğu kenarının dışına taşıyordu.
- **Süreler** (9 Ekim 2026; Intel Core i5-13500, 20 iş parçacığı, Linux; 4096² 32 bitlik, karolu ve Deflate'li DEM, 5 m, tepeler,
  eğilim ve gürültü: çukurlar ve düzlükler; dosyanın blokları, iş ve sonucun ya da nesnelerin yazılması dahil; masaüstünde 8 iş
  parçacığı, web'de işçi, release WASM, tek iş parçacığı; üç koşunun ortancası):

  | İş | Masaüstü (8) | Bütçe | Web (işçi, 1) | Bütçe |
  |---|---|---|---|---|
  | Çukur doldur | 0,714 s | 1,5 s | 2,279 s | 6 s |
  | Akış yönü | 0,986 s | 2 s | 3,219 s | 8 s |
  | Akış birikimi, D8 | 1,734 s | 2,5 s | 6,053 s | 10 s |
  | Akış birikimi, Çoklu yön | 3,265 s | 3,5 s | 10,895 s | 16 s |
  | Akış birikimi, D∞ | 2,816 s | 3,5 s | 13,268 s | 16 s |
  | Topografik nemlilik indisi | 3,398 s | 4 s | 11,662 s | 18 s |
  | Havzalar, ana havzalar | 1,400 s | 3 s | 4,030 s | 12 s |
  | Dere ağı | 1,643 s | 3 s | 5,471 s | 12 s |
  | Noktadan havza, 10 nokta | 1,601 s | 2,5 s | 5,431 s | 10 s |

  Ölçümle yapılan iyileştirmeler: doldurmanın taşması tek iş parçacığında 0,72 s sürüyordu, şeritli paralel doldurmayla yaklaşık
  0,25 s; düzlüklerin çözümü 0,61 s'den 0,28 s'ye (düzlük başına paralel, atomik uzaklıklar); Çoklu yön birikimi 7,36 s'den 3,24 s'ye
  (paylar hücre başına bir kez, tᵖ'nin üstel ve logaritmayla, büyük seviyeler iş parçacıklarında). Seviyelerin hücrelerini sıralamak
  denendi, yavaşlattığı için bırakıldı. Masaüstü:
  `cargo test --release -p kentos-raster --test all hydro_timing -- --ignored --nocapture --test-threads=1` (`KENTOS_PHASES=1` birikimin
  aşamalarını da yazar); web: `node scripts/perf/raster.mjs --only hydro`.
- **Ölçülmeyenler:** p99 (az koşu); 2²⁵ hücreye varan DEM; web'de bellek tepesi; coğrafi ve dönük rasterlerde süreler.

## Kapsam dışı

- Taşkın, yağış-akış, debi ve zaman (`CITY-15`); akış uzunluğu; ağırlıklı birikim.
- Dereyi DEM'e yakma ve çukuru kırarak açma (breaching); doldurma sınırı (ArcGIS'in z limit'i); kenar hücrelerini zorla dışarı akıtma.
- D∞ açısının raster çıktısı; havzaların ve derelerin raster çıktısı (Rasterleştir ile, ADR 0234); dere ağının yazılarından ad ve kot.
