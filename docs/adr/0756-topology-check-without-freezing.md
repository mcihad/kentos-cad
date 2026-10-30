# ADR 0756: Topoloji denetimi büyük çizimde donmaz

- **Durum:** kabul edildi (2026-10-30).
- **İstek:** sahibin sözü (30 Ekim 2026): “topoloji denetiminde çok fazla sahnede obje olduğunda kilitlenip kalıyor bu ciddi bir
  problem, kilitlenme yerine yumuşak bir ilerleme çubuğu ile gösterilmesi çok daha iyi olur iptal edilebilir şekilde … qgis içinde
  bu problemlerin hiçbiri yok”.
- **Web:** bir şey değişmez (web'in çağrısı aynı `check`'tir; ilerleme çubuğu TODOS.md §0.2'ye yazıldı). **`.kcad`:** değişmez.
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

Masaüstünün Topoloji sekmesi Denetle'de bütün kuralları arayüzün iş parçacığında, çizimin kendisinde koşuyordu: denetim bitene dek
pencere boyanmıyor, tıklama ve Esc gitmiyordu. Kuralların çekirdeği (`ops::topology_rules`) de büyük çizimde karesel işler
yapıyordu:

- **Kutu çiftleri** (`pairs`: Çakışmamalı, Yinelenmemeli, Ortak sınırda köşe eksik olmamalı): x'e göre sıralı bir tarama, ama her
  kutu için taramaya baştan başlıyordu: 100 000 parselde 5 × 10⁹ karşılaştırma.
- **Sarkan uç olmamalı:** her çizgi ucu için katmanın bütün bağlayıcıları kutularının uzaklığına göre yeniden sıralanıyordu.
- **… çizgilerinin ucunda olmalı:** her nokta bütün uçlarla; **Sınırı … sınırlarında olmalı:** her kenar örtünün bütün
  kenarlarıyla; **Boşluk** ve **İnce alan**'ın komşuları her bölge için katmanın bütün alanlarıyla karşılaştırılıyordu.
- **… içinde kalmalı:** örtü katmanının bütün alanları bir kez birleştiriliyor, sonra her nesne bu birleşimin tamamından
  çıkarılıyordu; parseller birbirine değdiğinde birleşim tek ve büyük bir alandır ve her bina onunla bindiriliyordu. Noktada her
  nesne için birleşimin bütün kenarları üzerine bir sarım dizini yeniden kuruluyor, her nokta bütün kenarlarla ölçülüyordu.

Ölçüm (release, i5-11300H; `topology_rules::timing`, 10 000 parsel, her parselde bir bina, parsel kenarlarında 20 000 yol, her yol
başında bir nokta):

| Kural | Önce | Sonra |
|---|---:|---:|
| Çakışmamalı | 0,067 s | 0,019 s |
| Boşluk olmamalı | 0,108 s | 0,108 s |
| Ortak sınırda köşe eksik olmamalı | 0,100 s | 0,051 s |
| … içinde kalmalı (bina / parsel) | 4,033 s | 0,089 s |
| … ile çakışmamalı (bina / parsel) | 0,176 s | 0,106 s |
| Sınırı … sınırlarında olmalı (parsel / yol) | 0,563 s | 0,137 s |
| Yinelenmemeli (yol) | 0,273 s | 0,068 s |
| Sarkan uç olmamalı (yol) | **185,032 s** | 0,153 s |
| … çizgilerinin ucunda olmalı (nokta / yol) | 2,045 s | 0,045 s |

Bulgu sayıları iki sütunda aynıdır. 100 000 parselde (`KENTOS_PERF_GRID=316`; nesneler 400 000) eski kodla Çakışmamalı 5,671 s ve
Eksik köşe 6,042 s idi; Sarkan uç'un karesel işi orada saatler sürerdi. Yenisi:

| Kural (100 000 parsel) | Süre | Bulgu |
|---|---:|---:|
| Çakışmamalı | 0,177 s | 1 |
| Boşluk olmamalı | 1,985 s | 1 |
| Ortak sınırda köşe eksik olmamalı | 0,524 s | 8 |
| İnce alan / Kısa kenar / Küçük açı / Geçerli | 0,081 / 0,075 / 0,077 / 0,122 s | 0 |
| … içinde kalmalı | 0,895 s | 8 |
| … ile çakışmamalı | 1,039 s | 101 846 |
| Sınırı … sınırlarında olmalı | 1,407 s | 632 |
| Yinelenmemeli | 0,719 s | 0 |
| Sarkan uç olmamalı | 1,699 s | 632 |
| … çizgilerinin ucunda olmalı | 0,526 s | 14 265 |

Başvurulan yol: QGIS'in topoloji denetleyicisi (`src/plugins/topology/topolTest.cpp`, 30 Ekim 2026'da okundu) iki katmanlı ve
dizin isteyen kurallarda katmanı bir `QgsSpatialIndex`'e koyar, adayları kutu sorgusuyla (`index->intersects( bb )`) alır; her
100 nesnede `progress` sinyali verir, döngüler `testCanceled()` ile durur. Kılavuzu (<https://docs.qgis.org/latest/en/docs/user_manual/plugins/core_plugins/plugins_topology_checker.html>)
Tümünü denetle ve Görünümde denetle'yi anlatır.

## Karar

### 1. Adaylar ağaçtan, kararlar yine kesin testten

- `Boxes` (çekirdeğin paketli Hilbert R-ağacı, `store::rtree::PackedTree`): sonlu kutular ağaçta, sonlu olmayanlar ayrı ve hep
  aday, boşlar hiç. Sorgu kutusu `t` kadar ve yuvarlamanın üstünde bir pay (`(|koordinat| + |t|) · 10⁻¹²`) büyür; böylece aday
  kümesi eski testin kabul ettiği her kutuyu içerir, sonra eski test aynen uygulanır ve adaylar sıralanır. `pairs`, Boşluk ve İnce
  alan'ın komşuları, Sınırı … sınırlarında olmalı'nın kenarları bu yoldan; sonuç bit bit eskisidir.
- **En yakın arama** (`PackedTree::nearest`, en iyi önce): Sarkan uç'ta bağlayıcılar kutularıyla ağaçta; bir bağlayıcının ölçüsü
  en yakın kenarının uzaklığı (eşitte ilk kenar), eşit ölçüde nesnelerin sırası: eskinin sözlük sırası (uzaklık, sıra) aynen.
  Kutunun uzaklığı yuvarlama payı düşülerek alt sınırdır (`below`). … çizgilerinin ucunda olmalı'da uçlar ağaçta, eşitte ilk uç.
  Bir nokta ya da bağlayıcı sonlu değilse eski yürüyüş kullanılır.
- **… içinde kalmalı:** örtünün birleşimi hiç kurulmaz. Bir nesnenin birleşimin dışında kalanı, örtünün kutusu nesneninkine değen
  alanlarının dışında kalanıdır (diğerleri onu kesemez); nesne yalnız bunlardan çıkarılır (en az biri, bindirme birleşimin
  bindirmesiyle aynı kuralla olsun diye). Nokta, kutusuna değen örtü alanlarıyla sarım kuralından yer alır; dışarıdaysa uzaklığı
  örtünün bütün kenarlarının ağacında en yakın arama. Bölge aynıdır; sayılar birleşimin kenarları yerine örtünün kendi
  kenarlarından hesaplandığı için son basamakta ayrılabilir (ortak durumlar göreli 10⁻⁹ içinde, artımlı denetimin eşitliği 6
  basamakta geçer).
- **Boşluk olmamalı** katmanın tek bindirmesidir (dışarıdan çerçeve, içinde boşluklar); değişmedi: 100 000 parselde 1,9 s ve
  doğrusal-logaritmik.

### 2. İzlenen denetim

- `Watch` (çekirdekte): `progress(pay)`, `stretch(baş, son, saniye)` ve `stopped()`. `check_watched` ve
  `incremental::recheck_watched` her kurala payını verir: kuralın baktığı nesne sayısı × türünün nesne başına süresi (100 000
  parselde ölçülen mikrosaniyeler: Boşluk 20, Sarkan uç 8, Eksik köşe / içinde kalmalı / ile çakışmamalı / sınırlarında 5,
  Yinelenmemeli 4, Çakışmamalı / ucunda 2, tek nesnelikler 1). Kuralın döngüsü her 16 adımda payını söyler ve durmayı sorar
  (`Pace::go`); durdurulan denetim cevapsız döner (`None`), yarım bulgu verilmez.
- Ölçülemeyen tek adım Boşluk'un bindirmesidir: öncesinde `stretch` ile süresi köşe sayısından tahmin edilir (köşe başına 5 µs).
- `check` ve `recheck` izlenmeyen sürümdür (`Unwatched`); web ve diğer çağıranlar değişmez.
- `cad.topology.check` iş olarak koşarken (ADR 0253 §9) işin durmasını ve ilerlemesini izler (`JobWatch`); durdurulursa
  `cancelled`.

### 3. Masaüstünde kendi iş parçacığında

- Kuralların katmanlarında 2 000 ya da daha çok nesne varsa (İşlemler'in eşiği) denetim çizimin okuma kopyasında (nesneler
  paylaşılır, kopyalanmaz) `kentos-topoloji` iş parçacığında koşar: toplama (çizimin nesneleri ve bağlı parçaların bantları) ve
  kurallar. Daha azı eskisi gibi hemen yapılır.
- Sekmenin çubuğunun altında: dönen gösterge, “Topoloji denetleniyor…”, yüzde, ilerleme çubuğu ve Durdur. Çubuk her karede
  (`window::frames`) söylenene yumuşakça yaklaşır (0,12 s'lik üstel yumuşatma, hiç geri gitmez); ölçülemeyen adımda geçen
  zamanla, beklenen sürede yolun %63'ünü, en çok %95'ini gider. Denetim sürerken Denetle ve Değişenleri denetle kapalıdır.
- Durdur denetimi hemen bitirir: iş parçacığına durması söylenir, sonradan söyledikleri beklenmez; son denetimin bulguları kalır
  (“Topoloji denetimi durduruldu.”). Yeni bir denetim süreni durdurur.
- Değişenleri denetle, Düzelt'ten sonraki denetim ve İstisna yap'tan sonraki denetim de aynı yoldan; ardından olan (seçili
  satırların anahtarlarıyla izlenmesi, düzeltilen satırın yerindekinin seçilmesi, istisnanın sözü) cevap gelince yapılır
  (`Then`). Cevap yalnız başladığı çizime uygulanır; çizimin gördüğü durumu (kuşak, revizyon) başlarken alınır, denetim
  sürerken çizim değiştiyse sonuç “eski” sayılır.
- Bulgular ve toplananlar `Arc` içinde: Değişenleri denetle önceki denetimi iş parçacığına kopyalamadan verir.

## Sonuçlar

- 10 000 parselde Sarkan uç'un 185 saniyesi 0,15 saniyeye iner; 100 000 parselde bütün kurallar birkaç saniyededir ve pencere
  bu sürede boyanır, Durdur'la biter.
- Kural ekleyen, aday kümesini ağaçtan alır; karesel bir yürüyüş yazılmaz. Ölçülemeyen uzun adım `stretch` ile söylenir.
- Boşluk olmamalı hâlâ tek bindirmedir: çubuk o adımda geçen zamanla ilerler, durdurma adımın sonunda gerçekleşir (arayüz hemen
  bırakır).

## Doğrulama

- Bulguların eşitliği: `cargo test -p kentos-geometry-core --test all topology` (8 geçti; 3 süre ölçümü ayrı) — ortak durumlar,
  artımlı denetimin tam denetimle eşitliği (üç tohum, on iki kural) ve ağaçla adayların eski yürüyüşle aynı bulguları.
  Bağımsız başvurular: `python3 scripts/fixtures/topology_rules_cases.py --check` (17 durum) ve
  `python3 scripts/fixtures/topology_check_cases.py --check` (23 + 21 durum); `KENTOS_CASES_ONLY=cad.topology cargo test -p
  kentos-native-application --test all every_command_case`.
- İş olarak: `topology_check::tests::a_job_asked_to_stop_answers_cancelled` (durdurulan iş `cancelled`, durdurulmayan bütün
  bulguları ve çubuğun sonunu verir).
- Masaüstü: `cargo test -p kentos-desktop --lib topology::` (14 geçti): 2 500 parselde denetim kendi iş parçacığında ve
  çekirdeğin bulgularını verir; Düzelt'ten sonraki denetim de iş parçacığında, değişenler üstünden ve bütün denetimin
  bulgularıyla; Durdur hemen biter, sonradan gelen cevap beklenmez, son bulgular kalır; çubuk yumuşar, ölçülemeyen adımda
  zamanla ilerler ve geri gitmez. Küçük çizimin eski testleri (hemen yapılan denetim) değişmeden geçer.
- Süreler: `cargo test --release -p kentos-geometry-core --test all topology_rules::timing -- --ignored --nocapture`
  (10 000 parsel; `KENTOS_PERF_GRID=316` ile 100 000), Bağlam'daki tablolar.
- Resimler: `KENTOS_SHOTS_ONLY=suruyor cargo test -p kentos-desktop --lib topology::tests::screens -- --ignored --nocapture
  --test-threads=1` (`.run/shots/topoloji-suruyor-*`: koyu ve açık, 1440 × 900 ve 1100 × 650; çubuk, yüzde, Durdur, boş listede
  “… kural denetleniyor; bulgular denetim bitince burada.”).
