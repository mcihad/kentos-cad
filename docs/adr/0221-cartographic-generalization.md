# ADR 0221: Kartografik genelleştirme

- **Durum:** kabul edildi (2026-10-11). Kapsamı ben belirledim (sahibin sözü: “Benim seçmeme gerek yok sen sıradan devam et”). İki platform
  (masaüstü ve web); `.kcad` şeması değişmez; ikonlar sorulmadan seçilir; ilke “Performance First”. Madde tek parçada biter.
- **Bağlam belgesi:** TODOS.md `GIS-25`; ADR 0201 (geometri işlemleri: overlay, Tampon, Sadeleştir, Çıktı katmanı, `addedShapes`),
  ADR 0215 §2.4 (komşu alanların ortak sınırı, 1 mm), ADR 0220 (Analiz › Geometri ▾, doğru boyunca kesim), ADR 0140 (Sadeleştir'in
  kuralı), ADR 0142 (kotların taşınması).
- **Araştırma:** QGIS'in Smooth'u (Chaikin'in köşe kesmesi: yineleme 1–10, kesim oranı 0–0,5, en büyük köşe açısı) ve Orthogonalize'ı
  (iD ve JOSM'un kuralı: açı eşiği 15°, en çok 1000 yineleme, 1e-8; köşe açıortayı boyunca `0,1 · d · 2 · min(|p|, |q|)` hareket, en
  kareye yakın sonuç), Simplify coverage'ı (GEOS'un CoverageSimplifier'ı: Visvalingam–Whyatt, tolerans alan eşiğinin karekökü, düğümler
  değişmez, üçgeninde başka köşe olan köşe kalır, iç kenarlar ya da hepsi), Eliminate selected polygons'u (en uzun ortak sınırlı ya da en
  büyük komşuya, katılamayanlar sonraki geçişte); ArcGIS'in Simplify Building'i, Aggregate Polygons'ı (kümeleme uzaklığı, en küçük alan,
  en küçük boşluk, yalnız köprüler), Smooth Line'ı (PAEK, Bézier), Simplify Shared Edges'i, Eliminate'i ve Eliminate Polygon Part'ı (alan
  ya da yüzde, yalnız delikler ya da her parça, hepsi küçükse en büyüğü kalır); CartAGen'in `simplify_building_ruas`'ı (Ruas 1999, AGENT:
  en kısa kenar önce, komşu kenarların doğrultularına göre dik, paralel ve ters paralel durumlar, sonuç iki kat büyükse eski bina);
  PostGIS'in `ST_CoverageSimplify`'ı; Netcad'in Çizgileri Basitleştir ve Alanları Basitleştir'i.

## Bağlam

Bir haritayı küçük ölçekte okunur kılmak için nesneler sadeleşir, yumuşar, birleşir ya da kalkar: binaların küçük çıkıntıları ve
girintileri atılır, açıları dik olur; birbirine yakın binalar yapı adası olur; dere ve yollar yumuşar; küçük alanlar komşularına katılır.
Komşu alanlar (parseller, mahalleler, il sınırları) nesne nesne sadeleşince ortak sınırlarında boşluk ve örtüşme açılır; ortak sınır
bir kez sadeleşmelidir. KentOS'ta yalnız nesne başına Sadeleştir (ADR 0201 §7) vardı.

## Karar

### 1. Kapsam

İşlemler'in yeni **Genelleştirme** kategorisinde yedi araç, iki platformda:

1. **Yumuşat** (`generalize.smooth`): Chaikin'in köşe kesmesi; Ortak kenarları koru ile komşular ortak kenarlarını birlikte yumuşatır.
2. **Dik açılandır** (`generalize.orthogonalize`): dik açıya ve düzlüğe yakın köşeler tam dik ve düz olur.
3. **Bina sadeleştir** (`generalize.building`): kısa kenarlar komşularına göre atılır, dik açılar kalır.
4. **Alanları kümele** (`generalize.aggregate`): birbirine yakın alanlar aralarına köprü kurularak birleşir.
5. **Küçük parçaları ayıkla** (`generalize.dropParts`): çok parçalı alanların küçük parçaları ve küçük delikler atılır.
6. **Küçükleri komşuya kat** (`generalize.eliminate`): küçük alanlar en uzun ortak sınırlı ya da en büyük komşusuna katılır.
7. **Ortak kenarları koruyarak sadeleştir** (`generalize.coverageSimplify`): komşu alanların ortak kenarları bir kez sadeleşir.

Ortak kurallar ADR 0201'inkiler: sonuç **Çıktı katmanı**na yeni nesnelerdir (yoksa adıyla açılır), girdi değişmez ve yalnız okunur
(kilitli katmanınki de); her girdi nesnesi sonuçta bir kez yazılır (değişmeyen olduğu gibi; Alanları kümele ve Küçükleri komşuya kat
nesneleri birleştirir), öznitelikleriyle. Yaylı kenarlar korunur: uçları kıpırdamaz, kenar yay kalır (Yumuşat, Dik açılandır, Bina
sadeleştir, Ortak kenarları koruyarak sadeleştir); Alanları kümele ve Küçükleri komşuya kat yaylarla kesindir (overlay). Elips ve eğri
0,1 mm'lik kirişleriyle girer (ADR 0149 §5.3) ve söylenir. Kotlar Yumuşat'ta, Dik açılandır'da, Küçük parçaları ayıkla'da ve Ortak
kenarları koruyarak sadeleştir'de köşeleriyle taşınır (§2–§8); öbürlerinde taşınmaz ve söylenir. Hesaplar çekirdekte
(`ops::generalize`); web'e bütün nesneler bir çağrıyla gider. Şeritte CBS'nin Analiz › Geometri ▾ listesinde, Geometri üretimi ve
Örnekleme'nin ardından; liste İşlemler kategorileriyle başlıklanır (§9).

### 2. Yumuşat

- **Girdi:** alanlar ve çoklu çizgiler (parçaları ve delikleriyle); **Yineleme** (1–10, varsayılan 3); **Kesim oranı** (0,01–0,5,
  varsayılan 0,25); **En büyük açı** (0–180°, varsayılan 180); **Ortak kenarları koru** (kapalı).
- **Kural** (Chaikin; QGIS'in Smooth'u gibi): bir yinelemede yolun (açık yol ya da halka) her düz kenarı `p_i → p_(i+1)` iki nokta
  verir: `q = p_i + o · (p_(i+1) − p_i)`, `p_i` sabitse `p_i`; `r = p_i + (1 − o) · (p_(i+1) − p_i)`, `p_(i+1)` sabitse `p_(i+1)`
  (koordinat koordinat, bu işlem sırasıyla). Yay kenarı kendi başlangıcını yayıyla verir. Noktalar kenar sırasıyla dizilir, art arda iki
  aynı nokta bir olur (arkadakinin kenarıyla), halkada son nokta ilkiyle aynıysa düşer. **Sabit köşeler:** açık yolun uçları, bir yay
  kenarın uçları, dönüşü (gelen ve giden doğrultu arası açı, `|atan2(u × w, u · w)|`) En büyük açı'dan büyük köşe ve Ortak kenarları
  koru'da düğümler (§8). Dönüş her yinelemede o anki yoldan okunur. Kot: `q`'nun ve `r`'nin kotu kenarın iki ucunun kotundan aynı
  oranla (ikisi de varsa), sabit köşe kendi kotuyla.
- **Ortak kenarları koru:** alanların halkaları §8'in kenarlarına ayrılır; her kenar bir kez, uçları sabit açık yol olarak (düğümsüz
  halka kapalı yol olarak) yumuşar, halkalar kenarlarından yeniden kurulur. Ortak kenarın kotları onu ilk kullanan halkanınkilerdir.

### 3. Dik açılandır

- **Girdi:** alanlar ve çoklu çizgiler; **Açı eşiği** (0–45°, varsayılan 15); **En çok yineleme** (1–10 000, varsayılan 1000).
- **Kural** (iD, JOSM ve QGIS'in Orthogonalize'ı): `alt = cos((90 − eşik)°)`, `üst = cos(eşik°)`. Halkanın her köşesinde (açık yolun
  iç köşelerinde) `p = p_(i−1) − p_i`, `q = p_(i+1) − p_i`, uzunlukları `√(x² + y²)`; biri sıfırsa hareket yok; `ölçek = 2 · min(|p|,
  |q|)`, `d = p̂ · q̂`; köşe yalnız dik açıya ya da düzlüğe eşik kadar yakınsa (`|d| < alt` ya da `|d| > üst`) hareket eder: `d < −√½`
  ise `d + 1`, `v = p̂ + q̂` (sıfırsa hareket yok), hareket `v̂ · (0,1 · d · ölçek)`. Bir yinelemede bütün hareketler o anki köşelerden
  bulunur, sonra birlikte uygulanır. **Karelik:** eşikteki köşelerin `2 · min(|d − 1|, |d|, |d + 1|)` toplamı (`d` uzunluğu sıfır
  olan yanı sıfır sayarak). Başta girdinin karelikle değeri tutulur; her yinelemeden sonra daha küçükse o an tutulur; tutulanın karelikle
  değeri 1e-8'in altına inince ya da yinelemeler bitince tutulan yazılır. **Sabit:** açık yolun uçları, yay kenarın uçları. Kotlar
  köşeleriyle gider.

### 4. Bina sadeleştir

- **Girdi:** alanlar; **En kısa kenar** (m, varsayılan 3); **Paralel sınırı** ve **Dik sınırı** (°, varsayılan 20).
- **Kural** (Ruas, AGENT; CartAGen'in `simplify_building_ruas`'ı gibi), parça parça: alanın halkalarının (dış halka, sonra delikler)
  En kısa kenar'dan kısa düz kenarları uzunluklarına göre sıralanır (eşitse halka, sonra kenar sırasıyla); sırayla denenir, biri
  başarınca kısa kenarlar yeniden aranır; hiçbiri başaramayınca biter. `a → b` kenarının öncesi `a′`, sonrası `b′` (halkada döngüsel);
  halkanın en az 4 köşesi, `a′a`, `ab` ve `bb′` kenarlarının düz, `a′a` ile `bb′`'nün sıfırdan uzun olması gerekir (paralel ve ters
  paralelde `a′`'ne gelen ve `b′`'nden çıkan kenarların da düz olması). `α = atan2(b′ − b) − atan2(a′ − a)`,
  (−π, π]'ye çekilir:
  - **Dik** (`|α|` π/2 ± dik sınırı içinde): `a′a` ve `bb′` doğrularının kesişimi `c = a + t · (a′ − a)`, `t = ((b − a) × (b′ − b)) /
    ((a′ − a) × (b′ − b))` (payda sıfırsa olmaz); halka `c`'den başlar, `b`'den sonraki köşelerle sürer (bir köşe azalır).
  - **Paralel** (`|α|` < paralel sınırı; çıkıntı ya da girinti; halkanın en az 5 köşesi): `a′`'nün `bb′` doğrusuna izdüşümü `ca = b +
    t · (b′ − b)`, `t = ((a′ − b) · (b′ − b)) / |b′ − b|²`, `0 < t < 1` ise geçerli; `b′`'nün `aa′` doğrusuna izdüşümü `cb` aynı biçimde.
    Yalnız `ca` geçerliyse yerine `ca, b′`; yalnız `cb` ise `a′, cb`; ikisi de geçerliyse `|a′ − a| ≤ |b′ − b|` iken `ca, b′`, değilse
    `a′, cb`; hiçbiri geçerli değilse olmaz. Halka bu iki noktadan başlar, `b′`'den sonraki köşelerle sürer (iki köşe azalır).
  - **Ters paralel** (`|α|` > π − paralel sınırı; basamak; en az 5 köşe): `u = (a − a′) + (b′ − b)` (sıfırsa olmaz), `m = (a′ + b′) /
    2`; `a′` ve `b′` `m`'den `û` doğrultusundaki doğruya izdüşer; halka bu iki noktadan başlar, `b′`'den sonrakilerle sürer.
  - Başka açıda olmaz. Sonucun net alanı girdininkinin iki katından büyükse ya da sonuç geçersizse (ADR 0201 §6) bina olduğu gibi
    yazılır ve söylenir.

### 5. Alanları kümele

- **Girdi:** alanlar; **Kümeleme uzaklığı** (m, varsayılan 10); **En küçük alan** ve **En küçük boşluk** (m², varsayılan 0);
  **Girintileri doldur** (kapalı); **Grupla** (alan, isteğe bağlı).
- **Kural:** gruplar ayrı ele alınır (Grupla'nın değeri; boşsa hepsi bir grup). Grubun alanları aralarındaki uzaklık (biri öbürüne
  değiyor ya da örtüşüyorsa 0) kümeleme uzaklığından büyük değilse aynı kümededir (bağlı bileşenler). Kümenin kapanışı: alanların
  birleşimi `uzaklık / 2` dışa, sonra `uzaklık / 2` içe tamponlanır (ADR 0201'in kesin tamponu, yaylarla). Kapanışın birleşimden
  fazlası parçalara ayrılır; iki ya da daha çok girdi nesnesiyle ortak sınırı (1 mm içinde, uzunluğu sıfırdan büyük) olan parça
  **köprüdür**; Girintileri doldur açıkken her parça alınır. Sonuç kümenin alanlarıyla alınan parçaların birleşimidir. Tek nesneli
  kümede köprü olmaz: kapanış Girintileri doldur açık değilse ya da nesne tek, dışbükey, deliksiz ve yaysız alansa hesaplanmaz. En
  küçük boşluktan küçük delik doldurulur; En küçük alandan küçük sonuç yazılmaz ve söylenir. Sonuçlar kümelerin ilk nesnesinin
  sırasıyla, küme içinde dış halkasının en alttaki (eşitse en batıdaki) köşesine göre yazılır.
- Sonuç **Nesne sayısı** (bir alanının iç noktası içine düşen girdi nesneleri) ve Grupla'nın değerini alır; öbür öznitelikler
  alınmaz.

### 6. Küçük parçaları ayıkla

- **Girdi:** alanlar; **En küçük alan** (m²); **Yüzde** (0–100, varsayılan 0: kullanılmaz); **Parçalar** ve **Delikler** (ikisi de
  açık).
- **Kural** (ArcGIS'in Eliminate Polygon Part'ı gibi): bir parça ya da delik, net alanı En küçük alan'dan küçükse ya da Yüzde sıfırdan
  büyükken nesnenin dış halkalarının alanları toplamının yüzdesinden küçükse atılır. Bütün parçaları küçük alanın en büyük parçası
  (eşitse ilki) kalır. Kalan halkalar köşeleri ve kotlarıyla değişmez.

### 7. Küçükleri komşuya kat

- **Girdi:** alanlar; **En küçük alan** (m²); **Kime** (En uzun ortak sınır ya da En büyük komşu).
- **Kural** (ArcGIS'in ve QGIS'in Eliminate'i gibi): net alanı En küçük alan'dan küçük alanlar küçüktür, öbürleri büyük. Küçükler
  alanlarına göre (eşitse girdi sırasıyla) geçiş geçiş ele alınır: bir küçük, o anki büyükler arasında ortak sınırı (1 mm içinde,
  ADR 0215 §2.4) sıfırdan uzun olanlara bakar; En uzun ortak sınır'da ortak sınırı en uzun olana, En büyük komşu'da o anki net alanı
  en büyük olana (eşitse girdide önce olana) katılır: büyüğün şekli ikisinin birleşimi olur (overlay). Komşusu olmayan küçük sonraki
  geçişe kalır; bir geçiş hiç katmazsa biter. Katılamayan küçükler olduğu gibi yazılır ve söylenir. Büyükler öznitelikleriyle yazılır;
  rapor tablosu her katılmayı söyler.

### 8. Ortak kenarları koruyarak sadeleştir

- **Girdi:** alanlar; **Tolerans** (m, varsayılan 1); **Sınırı koru** (kapalı).
- **Kapsama grafı** (§2 ile ortak): bütün girdi alanlarının halkaları (nesneler girdi sırasıyla, parçalar, dış halka sonra delikler)
  kenar parçalarına ayrılır; bir parça iki ucunun tam koordinatları ve yönüne göre düzenlenmiş büküm değeriyle tanınır; aynı parçayı
  kullanan halkalar onun **halka kümesidir**. Bir köşe, ona değen farklı parçaların sayısı ikiden başkaysa ya da halkada ona gelen ve
  ondan çıkan parçaların halka kümeleri farklıysa **düğümdür**. Halkalar düğümden düğüme **kenarlara** ayrılır; düğümü olmayan halka
  kendi başına kapalı kenardır. Her kenar bir kez tutulur, onu ilk kullanan halkanın yönünde (kapalı kenar o halkanın ilk köşesinden).
  Komşular ortak köşelerini tam aynı koordinatla paylaşmalıdır (geçerli kapsama); paylaşmayanların sınırı ayrı ayrı sadeleşir.
- **Kural** (GEOS'un CoverageSimplifier'ı gibi): kenarlar sırayla Visvalingam–Whyatt'la sadeleşir. Kenarın sabit köşeleri: uçları
  (açık kenar), ilk köşesi (kapalı kenar), yay parçalarının uçları. Sabit olmayan köşenin alanı `|((s − ö) × (k − ö))| / 2` (`ö` önceki,
  `s` sonraki kalan köşe); en küçük alanlı köşe (eşitse kenardaki sırası önce olan) alanı toleransın karesinden küçükse atılmaya
  bakılır: üçgeninin (`ö`, köşe, `s`; kenarları da içinde) içinde bütün kenarların kalan köşelerinden başkası (`ö`, köşe ve `s`
  dışında) varsa, ya da atılınca kenar `ö`–`s` arası iki köşeli düz kenara inip aynı iki köşe arasında böyle başka bir kenar varsa
  atılmaz ve sıradan çıkar; yoksa atılır, komşularının alanları yenilenir ve sıraya geri girer. Açık kenarın iki ucu, uçları aynı
  düğüm olan kenarın en az iki iç köşesi, kapalı kenarın en az üç köşesi kalır. **Sınırı koru:** yalnız bir halkanın kullandığı
  kenarlar değişmez. Halkalar sadeleşen kenarlarından yeniden kurulur; köşeler kotlarıyla kalır.

### 9. Şerit

Analiz sekmesi 1100 px'te dolu (ADR 0219 §7); yedi araç Geometri ▾ listesine Örnekleme'nin ardından eklenir. Bir panelin ▾ listesi
iki ya da daha çok İşlemler kategorisinin araçlarını taşıyorsa her kategorinin araçları onun adıyla başlıklanır (iki platformda;
Geometri ▾: Geometri üretimi, Örnekleme, Genelleştirme). Başka listeler değişmez.

### 10. Kesişim çekirdeğinin sağlamlığı

Alanları kümele'nin süre sahnesinde (dörtlü bloklarda dönük, köşeleri kaydırılmış basamaklı binalar) kümelerin çoğu birleşmedi:
2 500 binadan 2 144 sonuç çıktı. Bir bloğun kapanışı 545,58 m² yerine 0,63 m²'ydi. Hata yeni araçta değil, ADR 0201'in tamponunun ve
bütün alan işlemlerinin kullandığı kesişim çekirdeğindeydi (`geom::arrangement`). Dışa tamponun sınırında, teğet olması gereken
kapsül kenarları çembere 10⁻⁹ m içeriden geçer: kapsül mutlak koordinatlarda kurulur. Bu yüzden sınırda 2–4·10⁻⁵ m'lik yaylar kalır.
İçe tamponun diskleri ve kapsül uçları bu yayların uçlarında ve binanın köşesinde neredeyse tek noktada buluşur. Çekirdek üç yerde
düzeldi; düzeltmeler bütün alan işlemleri içindir:

1. **Bir çember.** Merkezi ve yarıçapı TOL (10⁻⁶ m) içinde olan yaylar aynı çembere oturur: ilk yerleşen yayın çemberine. Yayların
   uçları yerinde kalır. Bir köşenin diski ile sonraki kenarın kapsül ucu arasında 10⁻¹⁰ m fark vardı. Üçüncü bir çember bu ikisini
   7·10⁻⁶ rad'lık sığ açıyla, birbirinden 1,4·10⁻⁵ m uzak noktalarda keserdi. Böylece üst üste yatan parçaları ayrı köşelerde biter ve
   hiç birleşmezdi.
2. **Yakındaki kesişim.** Bir kesişim noktası, TOL içinde kalan üçüncü kenarı da keser. Birbirine yakın iki çembere birden teğet olan
   bir doğru, onlarla TOL'dan ince şeritler kurar. Bu şeritlerin kenarları ayrı köşelerde biterdi.
3. **Köşelerin dengesi.** Bir parça, iki yanına en yakın başka parçaya uzaklığın dörtte biri kadar adım atılarak sınıflanır. TOL'dan
   ince karmaşada adım, parçaya birleşen kenarların sapmasından kısa kalabilir. Örnek: bir köşeden geçen üç çemberin birleşmiş,
   10⁻⁶ m'den biraz uzun parçası. Böyle bir parça yanlış sınıflanınca halka kopar ve bütün bir yüz düşer. Sonucun her köşesinde giren
   ve çıkan parça sayısı eşit olmalıdır. Eşit olmayan bir köşede, adımı 10⁻⁵ m'den kısa (sarsak) parçaların kullanımı değişir (ileri,
   geri ya da hiç). Değişiklik, o köşeden ters yönde eksik olan bir köşeye giden en kısa sarsak parça yolu boyuncadır. Kesim
   çizgilerinin iki yönde kullanılan parçaları değişmez. Doğru bir sonuçta dengesiz köşe olmaz; onarım yalnız yanlış sınıflamada
   çalışır.

Birinci ve ikinci düzeltme topolojiyi kaynağında tutarlı kılar: süre sahnesinin bloğu bunlarla düzeldi. Üçüncüsü son güvenlik ağıdır:
rastgele 300 bloğun 4'ünde kalan hata, adımı kısa kalan tek bir parçaydı. Denetim için önce GEOS'un kapanışı kullanıldı. Ancak GEOS
tamponun girdisini uzaklığın yüzde biri kadar sadeleştirir ve kesin kapanıştan 0,006 m²'ye kadar sapar (4 m'de). Bu yüzden ayrı
bir başvuru yazıldı (`closing_cases.py`). Başvuru, GEOS'un dışa tamponunu (çeyrek turda 1024 kiriş) kesin olarak içe aşındırır:
taşıdığı her kenarın iç şeridi ve içe dönen köşelerindeki diskler çıkarılır. Sonuç kesin kapanışa 10⁻⁴ m² içinde yakındır.
Bir blokta dışa tamponda 1,5·10⁻⁵ m²'lik bir delik vardı: avlunun ortası binalara 2,5 m'den biraz uzaktır. Çekirdeğin kapanıştaki
deliği 19,6872311 m² çıktı; üç çemberin köşelerinden kurulan analitik değer 19,6872307 m²'dir. GEOS bu blokta 2·10⁻³ m² saptı.

### 11. Performans

- **Kapsama grafı** (§8, §2): köşeler tam koordinatlarının bitleriyle hızlı bir karmada numaralanır; parçalar uç numaraları ve büküm
  bitleriyle tanınır. Halka kümeleri ve köşelerin parçaları düz dizilerdedir; düğüm denetimi bunlardan okunur.
- **Visvalingam–Whyatt** (§8): bir köşenin üçgeninde başka köşe aranırken bütün kenarların köşeleri sayım ızgarasındadır (sıralamasız,
  hücrede ortalama dört köşe); iki köşeli düz kenarlar uçlarıyla karma kümededir; sıra yığındır, eskiyen kayıt atlanır.
- **Alanları kümele** (§5): kümeler R-ağacı (kutular uzaklıkla genişletilir) ve birleşim-bul ile bulunur; bir kümenin kapanışı başka
  kümeye erişmez, bu yüzden her küme ayrı kapanır. Masaüstü kümeleri iş parçacıklarında kapatır (çekirdeklerin biri eksik, en çok
  sekiz, sıradaki kümeyi alan iş parçacığı); sonuçlar kümelerin sırasıyla dizilir. Web işçide sırayla kapatır.
- **Küçükleri komşuya kat** (§7): komşu adayları R-ağacından, katılan küçüğün şimdiki sahibi üzerinden.
- **Web:** bütün nesneler bir çağrıyla gider; sonuçlar köşe sayıları ve alanlarıyla döner, raporlar masaüstününkiyle aynı kuraldan.
  Küçük parçaları ayıkla'da deliği ve ikinci parçası olmayan alan çekirdeğe gitmez (atacak bir şeyi yoktur), değişmeyen nesnenin
  şekli geri dönmez; nesne olduğu gibi yazılır (100 000 parselde 2,5 s'den 0,34 s'ye).
- **Kesişim çekirdeği** (§10): kuralların bedeli çekirdeğin kendi hızlandırmalarıyla karşılanır. Karma tablolar SipHash yerine
  deponun hızlı karmasıyla çalışır; bu tablolar sırayla gezilmez, sonuç bit bit aynıdır. Sarım dizininin bantları düz dizidedir.
  Yakın kesişim kuralı ikinci bir ızgara kurmaz, aday çiftlerinin ızgarasını kullanır: kutular orada TOL kadar genişletilmiştir,
  bir noktanın hücresi yeter. Bir noktayı bir kez dener; kesişimi kuran yayın çemberindeki yayları atlar, çünkü onlar o noktada
  kendi çiftlerinden kesilir. En yakın noktayı hesaplamadan önce doğruya ya da çembere uzaklığa bakar. Eski ve yeni çekirdek aynı
  makinede (release): Tampon 400 parselde tek tek 362 → 331 ms, birleştirerek 454 → 403 ms; Gruplayarak birleştir 22 → 18 ms;
  400 × 400 Kesişim 126 → 90 ms; ağ analizinin hizmet alanları 50 m payla 247 → 245 ms, 25 m payla 814 → 733 ms, 4 dakikalık alan
  83 → 82 ms. Hızlandırmalar olmadan kurallar bu işleri %10–25 yavaşlatıyordu; bunun büyük kısmı yakın kesişim kuralının ikinci
  ızgarasındandı (anahtarlarla tek tek kapatılarak ölçüldü; çember ve denge kurallarının ölçülebilir bedeli yok).

## Sonuçlar

- Yedi araç iki platformda, ortak durumlar bağımsız başvuruyla; Analiz › Geometri ▾ on yedi araç taşır, kategorileriyle başlıklı.
- Kesişim çekirdeği TOL ölçeğindeki karmaşada yüz yitirmez (§10). Tampon'un içe uzaklığı, Gruplayarak birleştir ve alan işlemleri de
  bundan yararlanır.
- Ortak kenarları koruyarak sadeleştir ve Yumuşat'ın Ortak kenarları koru'su yalnız ortak köşeleri tam paylaşan komşularda kapsamayı
  korur; Topolojik temizlik (ADR 0148) ya da topoloji kuralları (ADR 0202) önce çalıştırılır.
- Kapsam dışı: Alanları kümele'nin dik açılı köprüleri (ArcGIS'in ORTHOGONAL'ı), çizgilerin yer değiştirmesi (displacement), seçici
  atma (typification), PAEK ve Bézier yumuşatma, bina yönlendirme ve dikdörtgene indirme.

## Uygulama

- **Çekirdek** `ops::generalize` (`crates/shared/geometry-core`): `mod.rs` (halkaların izi `Track`, kotlarıyla; şekle geri yazma),
  `smooth.rs` (Chaikin), `orthogonal.rs` (iD'nin kuralı), `building.rs` (Ruas), `coverage.rs` (kapsama grafı), `simplify.rs`
  (Visvalingam–Whyatt), `parts.rs`, `aggregate.rs` (`clusters`, `close`, `aggregate`), `eliminate.rs`, `calls.rs` (araç başına
  işlevler ve web'in `generalizeSmooth`, `generalizeSquare`, `generalizeBuilding`, `generalizeParts`, `generalizeCoverage`,
  `generalizeAggregate`, `generalizeEliminate`'i). `spatial_query::geometry_of_areas`, `convert::paths::rebuild` ve deponun `IdHasher`'ı
  paylaşıldı. Kesişim çekirdeğinin sağlamlığı `geom::arrangement`'te: `Circles`, yakın kesişimlerin kesmesi (`build`, `BoxCells`'in
  `at`'i; `BoxCells` eski `candidate_pairs`'tir), `balance` (`classify_with`); hızlı karma `FastMap` (deponun `IdHasher`'ı), sarım
  dizininin bantları düz dizi.
- **Masaüstü** `kentos-processing`'in `builtin/generalize/` (yedi araç; Alanları kümele kümeleri iş parçacıklarında kapatır), kategori
  `generalization`; şeridin ▾ listesi `view.rs`'in `headed_menu_of`'u ve `overflow_heads`'iyle başlıklı; komutlar `catalog.rs` ve
  `ported.json`'da; pencere testleri ve resimler `apps/desktop/src/processing/generalize_tests.rs`.
- **Web** `model/ops/generalize.ts`, `processing/builtin/generalize/` (`shared.ts` ve yedi araç), kategori `generalization`, ikonlar
  `generalization`, `smoothLine`, `orthogonalize`, `buildingSimplify`, `aggregateAreas`, `dropParts`, `eliminateSmall`,
  `coverageSimplify`; şeritte `GENERATE_UNDER`, listenin başlıkları `ui/ribbon/controls.ts`'in `overflowRows`'u; resimler
  `shots.mjs generalize`, yerleşim `layout.mjs`'in `processing-generalize-*` görünümleri; süreler `scripts/perf/generalize.test.ts`.

## Doğrulama

- **Bağımsız başvuru** `scripts/fixtures/generalize_cases.py` (KentOS kodu olmadan; `fixtures/processing/v1/generalize.json` ve
  `generalize.kcad`, 22 durum). Chaikin, iD'nin dik açılandırması ve Ruas'ın kuralı ADR'deki işlem sırasıyla; parçalar, delikler,
  katılma ve kapsama grafı Python'un kendi kodlarıyla; Visvalingam–Whyatt GEOS'un kuralıyla hesaplanır. Kümelerin alanları elle
  çözülmüş kapalı biçimlerden gelir (çeyrek dolgu `25 − 6,25π`, kiriş parçası `12,5(t − sin t)`) ve GEOS'un kapanışıyla çapraz
  denetlenir (2·10⁻⁴ göreli). İki platform 22 durumu sayfada, işçide, masaüstünde doğrudan ve okuma kopyasında geçer.
- **Kapanışın başvurusu** `scripts/fixtures/closing_cases.py` (§10; `fixtures/geoprocess/v1/closings.json`, 45 blok): süre sahnesinin
  bloğu, rastgele 300 bloğun yitirilen dördü, 1, 1,5, 2,5 ve 4 m'de 40 blok. Çekirdeğin `tests/all/geoprocess.rs`'i hepsini 10⁻³ m²
  içinde geçer. Zorlama (`closings_of_random_blocks`, `--stress`): altı tohumda 2 700 blok, başvurudan en çok 8,9·10⁻⁵ m² farklı,
  parça sayıları aynı.
- **Her kuralın kendi testi** (`geom::arrangement::tests`): `circles_within_tol_are_one` (çember kuralı olmadan 12 parça, olması
  gereken 8) ve `a_crossing_beside_an_edge_cuts_it` (yakın kesişim kuralı olmadan teğet doğrunun tepesinde 3 parça, olması gereken 2).
  Denge onarımı olmadan kapanış fixture'ının `random-45`'i, üç kural birden olmadan süre sahnesinin bloğu yitirilir.
- **Öbür kullanıcılar:** SVG düzenleyicisinin çekirdeğinin dondurulmuş 1 341 yanıtından biri (`fixtures/svg/v1/cases.json`,
  `offsetRegion`'ın −4 birimlik sivri daralması) kayıtçısıyla yeniden yazıldı. Bölge aynıdır: simetrik fark 3·10⁻¹¹, düğüm sayıları
  aynı; bir düğüm 5·10⁻¹⁴, eğri tutamaçları 4·10⁻¹¹ kaydı. Nedeni yakın kesişim kuralının sonuç halkasına TOL içinde doğrusal bir köşe
  eklemesidir; eğri uydurma o yüzden biraz farklı tutamaç verir. Öbür bütün dondurulmuş ve ortak durumlar değişmedi.
- **Çekirdek testleri** (`ops::generalize::tests`): yumuşatma köşeleri keser, uçları ve kotları tutar; dik açılandırma dik açıya
  yakın köşeleri dik yapar; kısa basamak gider, dik açılar kalır; komşular ortak kenarlarını bir kez kullanır; dönük dörtlü blok tek
  kümeye birleşir.
- **Süreler** (release; masaüstü `cargo test --release -p kentos-processing --test cases generalize_timing -- --ignored --nocapture`,
  web `GENERALIZE_BENCH=1 … scripts/perf/generalize.test.ts` gönderilen WASM'la; p50 / p95, çalıştırıcının denetimi, araç ve tek geri
  alma adımı dahil):

  | İş | Masaüstü | Web | Bütçe |
  |---|---|---|---|
  | Yumuşat, 10 000 parselde ortak kenarlarla, üç yineleme (160 000 → 1 000 028 köşe) | 157,9 / 169,4 ms | 771,5 / 825,7 ms | 500 / 1200 ms |
  | Dik açılandır, 10 000 eğik bina | 343,6 / 350,7 ms | 539,8 / 549,4 ms | 700 / 1500 ms |
  | Bina sadeleştir, 10 000 basamaklı bina, 3 m | 60,1 / 71,0 ms | 143,6 / 151,6 ms | 300 / 800 ms |
  | Alanları kümele, dörtlü bloklarda 5 m (masaüstü 10 000 bina, web 2 500; dörtte bir sonuç) | 3052,9 / 3053,4 ms | 4101,1 / 4104,7 ms | 5000 / 8000 ms |
  | Küçük parçaları ayıkla, 100 000 parsel | 211,9 / 220,2 ms | 319,6 / 323,7 ms | 500 / 1500 ms |
  | Küçükleri komşuya kat, 100 000 parselin 5 000 şeridi | 365,7 / 371,1 ms | 582,7 / 649,2 ms | 800 / 2000 ms |
  | Ortak kenarları koruyarak sadeleştir, 100 000 parsel, 1,6 milyon köşe, 1 m | 1037,8 / 1056,5 ms | 2452,8 / 2497,6 ms | 2000 / 4000 ms |

  Alanları kümele masaüstünde kümeleri yedi iş parçacığında kapatır. Kesişim çekirdeğinin hızlandırmaları (§11) kümelemeyi
  masaüstünde 4,03 s'den 3,05 s'ye, web'de 5,61 s'den 4,10 s'ye indirdi. Web'de vitest ölçümün ardından kapanırken dakikalarca
  oyalanabiliyor; sonuçlar o sırada yazılmıştır.
- **Resimler:** masaüstü `.run/shots/islem-gen-*` (40), web `apps/web/scripts/e2e/out/shots/generalize` (40); şeridin Geometri ▾
  listesi başlıklarıyla 1440 × 900 ve 1100 × 650'de, iki temada; yerleşim denetimi `processing-generalize-*` görünümlerinde.
