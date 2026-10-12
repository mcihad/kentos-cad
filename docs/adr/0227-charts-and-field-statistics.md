# ADR 0227: Grafikler ve alan istatistikleri

- **Durum:** kabul edildi (2026-10-12). Sıra devir notunundur (12 Ekim: “`GIS-39` … ardından `GIS-40`”); kapsamı sahibin kurallarıyla ben
  belirledim: iki platform, simgeler sorulmadan, “Performance First”. Madde tek parçada biter.
- **Bağlam belgesi:** TODOS.md `GIS-39` (ve araştırma notu), ADR 0199 (katmanın alanları, değerlerin tek biçimi, Verilerden al'ın türü,
  öznitelik tablosunun satırları ve Göster'i), ADR 0200 (sayıların kuralı `kentos.statistics/1`, Özet istatistik'in grupları, İşlemler'in
  tablo çıktısı), ADR 0237 (ROC'un eğrisi bu maddeye bırakıldı), ADR 0233 §11 (Histogram'ın tablosu), ADR 0243 §8–§9 (Kesit ve Zaman
  serisi'nin tabloları), ADR 0063 (Öznitelikler'in seçim toplamları), ADR 0211 (katman süzgeci).

## Bağlam

Bir katmanın verisine bakmanın bugünkü yolları satır satırdır: Öznitelik tablosu (ADR 0199) nesneleri gösterir, Özet istatistik (ADR 0200)
bir alanın grup grup sayılarını bir İşlemler tablosu olarak verir, Öznitelikler çoklu seçimin uzunluk ve alan toplamını söyler. Bir alanın
dağılımını görmek (kaç farklı değer var, en sık hangisi, aykırı değer var mı), kullanımların paylarını karşılaştırmak, iki ölçü arasındaki
ilişkiye bakmak için grafik yoktur; İşlemler'in eğri veren tabloları (ROC, Kesit, Zaman serisi, Histogram) yalnız sayı olarak kalır.

- **ArcGIS Pro**'nun Charts'ı katmanın verisinden çubuk, çizgi, histogram, saçılım, kutu ve pasta grafiği çizer; grafik seçime bağlıdır:
  çubuğa tıklamak nesnelerini seçer, haritadaki seçim grafikte vurgulanır. Toplamalar Count, Sum, Mean, Median, Min, Max; histogram
  aralık sayısıyla, ortalama ve ortanca çizgileriyle; kutu grafiği çeyreklerle ve aykırı değerlerle; saçılımda eğilim çizgisi ve R².
  Data Engineering görünümü bir katmanın bütün alanlarının istatistiklerini (Count, Nulls, Min, Max, Mean, Median, Mode, Unique) bir
  tabloda gösterir.
- **QGIS**'in Statistical Summary paneli bir alanın (ya da ifadenin) istatistiklerini türüne göre verir: Count, Count (distinct), Count
  (missing), Sum, Mean, Standard deviation, Minimum, Maximum, Range, Median, Q1, Q3, IQR, Majority, Minority; metinde en kısa ve en uzun
  uzunluk. Grafikler İşlemler'dedir (Bar plot, Box plot, Vector layer histogram, Vector layer scatterplot) ve HTML dosyası yazar.
- **Netcad**'in Tematik › Grafik'i grafikleri haritaya çizer; KentOS'ta karşılığı GIS-17'nin Grafik işleyicisidir (ADR 0213).

## Karar

### 1. Kapsam

- Alt panelde iki yeni sekme:
  - **İstatistik** (`data.statistics`): bir katmanın bütün alanlarının özet tablosu ve seçilen alanın bütün istatistikleri, değerleri ve
    sayıları (§3, §8).
  - **Grafik** (`data.chart`): altı grafik türü (§4), katmandan ya da İşlemler'in sonuç tablosundan; seçime bağlı (§6); Grafik | Tablo
    görünümü, Panoya kopyala, PNG ve SVG olarak kaydet (§9).
- Altı tür: **Çubuk**, **Pasta** (halka), **Histogram**, **Kutu**, **Saçılım**, **Çizgi**. Çizgi, ROC eğrisi (ADR 0237), Kesit ve Zaman
  serisi (ADR 0243) gibi araç tablolarının ve tam sayı ya da ondalık X'li verinin grafiğidir.
- Hesap (veri, istatistikler, grafiklerin modeli, eksenler, yerleşim, tıklama, SVG) ortak çekirdektedir (`ops::charts`); iki platform aynı
  çizim listesini çizer (§5).
- **Kapsam dışı:** grafiklerin projede saklanması ve paftaya konması (§16.4), birden çok grafik ve gösterge panosu, ifadeyle alan
  (yalnız alanlar ve iki geometri ölçüsü, §2), tarih ekseni (tarih alanları kategori olur), yatay, gruplu ve yığılmış çubuk, saçılımın renge
  göre gruplanması ve saçılım matrisi, Q–Q grafiği, takvim ve veri saati grafikleri, logaritmik eksen, eksen sınırlarının elle
  yazılması, Statistical Summary'nin En seyrek'i ve uzunluk ortalaması, grafiklerin İşlemler'de araç olması (QGIS'in Plots'u).

### 2. Veri

- **Veri kümesi:** satırlar ve sütunlar. Katmanda satırlar katmanın nesneleridir, çizim sırasıyla, katman süzgecinden geçenler (ADR 0211;
  öznitelik tablosunun satırlarıyla aynı); İşlemler tablosunda tablonun satırları, sırasıyla.
- **Sütun:** anahtar, ad, tür ve her satırda bir hücre: **boş**, **değer** (tek biçimi ve gösterilişi) ya da **uymayan** (yazıldığı gibi).
  Türler: **Sayı** (tam sayı ve ondalık), **Metin**, **Tarih**, **Evet/hayır**.
- **Katmanın sütunları**, öznitelik tablosunun sırasıyla:
  - katmanın alanları (ADR 0199 §1), sırasıyla, adları takma adlarıyla: hücre alanın kuralıyla okunur (`check_value`): boş, tek biçim
    (gösterilişi `display_value`: değer listesinde etiket, Evet/Hayır, GG.AA.YYYY) ya da kurala uymayan;
  - şemada olmayan anahtarlar doğal sırayla: türü Verilerden al'ın kuralıyla (`infer_field`, ADR 0199 §3) katmanın **bütün** satırlarının
    değerlerinden (Göster'in seçtiğinden değil: tür seçimle değişmez); hücre o türün kuralıyla okunur;
  - iki geometri ölçüsü, Sayı türünde: **Alan (m²)** (alanı olan nesneler: kapalı alan, parçaları ve delikleriyle, daire, tam elips, kapalı
    eğri) ve **Uzunluk (m)** (çizginin, yolun ve yayın uzunluğu; alanın çevresi, delikleri dahil; ADR 0063'ün toplamlarının ölçüleri).
    Değer, geometri deposunun float64 ölçüsünün 3 kesir basamaklı ondalık metnidir (ikili değerin tam açılımı, yarımlar çifte;
    `statistics::fixed`); ölçüsü olmayan nesnede boş. Hiçbir nesnesinde değeri olmayan ölçünün sütunu yoktur (çizgilerin katmanında
    Alan (m²), noktalarınkinde ikisi de). Yerel projede ikisi çizim biriminde ve karesindedir: **Alan (mm²)**, **Uzunluk (mm)**
    (12 Ekim eki; kural ADR 0165'in “ifadeler çizim biriminde” ekinde).
- **İşlemler tablosunun sütunları:** tablonun sütunları sırasıyla; türleri Verilerden al'ın kuralıyla bütün satırlardan.
- **Kimlik ve sıra** (istatistiklerde ve grafiklerde aynı): Sayı tam değeriyle (`1.50` ile `1.5` aynı değerdir; gösterilişi tek
  biçimlerden ilk görülenidir), Tarih takvimle, Evet/hayır Hayır önce, Metin başındaki ve sonundaki boşluklar atılmış yazısıyla (boşluk
  atılınca boş kalan boştur) ve doğal sırayla (ADR 0153 §6). Uymayan hücreler yazıldıkları gibi, boşlukları atılarak, doğal sırayla,
  geçerli değerlerden sonra; boş en sonda.
- **Sayılar:** tek biçimden `kentos.statistics/1`'in ondalık sayısı olarak okunur (ADR 0200 §4). Kural sürümlüdür:
  **`kentos.charts/1`** (bu ADR'nin §3–§5'i).

### 3. Alanın istatistikleri

Göster'in seçtiği satırlarda (§8), bir sütun için:

- **Nesne** (satır), **Dolu** (değerli hücre), **Boş**, **Uymayan** (yalnız şemalı alanda), **Benzersiz** (dolu değerlerin farklı
  olanları, §2'nin kimliğiyle), **En sık** (en çok görülen değer ve sayısı; eşitlerde §2'nin sırasıyla ilki; her değer bir kez görülüyorsa
  yok).
- **Sayı:** Toplam, Ortalama, Std. sapma (`kentos.statistics/1`: kesin toplam, ortalama girdilerin en çok kesir basamağı s'ye 2 eklenmiş
  basamakta yarımlar çifte, örneklem sapması float64'te ortalamanın basamağıyla; tek değerde yok), **En az**, **Birinci çeyrek**,
  **Ortanca**, **Üçüncü çeyrek**, **En çok**, **Aralık** (en çok − en az, s basamakta kesin), **Çeyrekler arası** (Q3 − Q1).
  - Çeyrekler Hyndman–Fan'ın 7. tanımıyla (NumPy'ın, R'ın ve Excel'in QUARTILE.INC'inin): sıralı x₀ ≤ … ≤ xₙ₋₁, p ∈ {¼, ½, ¾} için
    h = (n − 1)·p, i = ⌊h⌋, f = h − i; Q = xᵢ + f·(xᵢ₊₁ − xᵢ) (f = 0 ise xᵢ). f ∈ {0, ¼, ½, ¾} olduğundan Q kesindir; s + 2 basamakta
    yazılır (ortalamanın basamağı). Çeyrekler arası da s + 2 basamaktadır.
- **Tarih:** En erken, Ortanca (alt ortanca: x_⌊(n−1)/2⌋), En geç, Aralık (gün; proleptik Gregoryen takvimde gün farkı).
- **Evet/hayır:** Evet ve Hayır sayıları.
- **Metin:** En kısa ve En uzun (karakter sayısı, boşluklar atılmış yazının Unicode karakterleri).
- **Değerler listesi:** dolu değerler, sayıları çoktan aza (eşitlerde §2'nin sırasıyla); her satırda değer, sayı ve pay (sayı / Nesne
  × 100, bir kesir basamağında, yarımlar çifte; tam kesirle); en çok 500 değer, gerisi “+k değer daha” satırıyla. Ardından varsa
  **(uymayan)** ve **(boş)** satırları (sayılarıyla). Her satırın nesneleri vardır (§6).
- **Özet tablo** (İstatistik sekmesinin solu): sütunlar Alan, Tür, Dolu, Boş, Benzersiz, En az, En çok, Ortalama, Toplam; tarihte En az ve
  En çok en erken ve en geçtir; metinde ve evet/hayırda boştur.

### 4. Grafikler

Her grafiğin satırları Göster'in seçtikleridir (§8; İşlemler tablosunda bütün satırlar). Alan seçimi türün istediği sütunlarla sınırlıdır
(sayı isteyen yere yalnız Sayı türündekiler).

- **Kategori** (Çubuk, Pasta, Kutu'nun Grubu, Çizgi'nin Serisi): hücrenin gösterilişi, §2'nin kimliği ve sırasıyla; boş “(boş)”.
- **Toplamalar:** Sayı (kategorideki satırlar), Toplam, Ortalama, En az, En çok, Ortanca (kategorinin sayı olarak okunan değerlerinden;
  ortalama ve ortanca grafiğin bütün değerlerinin en çok kesir basamağına 2 eklenmiş basamakta). Sayı dışındakilerde hiç sayısı olmayan
  kategorinin değeri yoktur (çubuğu çizilmez, adı kalır).

| Tür | Girdi | Kurallar |
|---|---|---|
| **Çubuk** | Kategori, Toplama, Değer (Sayı'da yok), Sırala: Kategori, Değer azalan, Değer artan | En çok 30 çubuk: sıradaki ilk 29'u ve geri kalanların bütün satırları tek “Diğer (k)” çubuğu (toplaması o satırlarla). Değeri olmayanlar değer sıralamasında sonda, eşitler kategori sırasıyla. Değer ekseni 0'ı içerir; eksi değer aşağı doğru. |
| **Pasta** | Kategori, Toplama (Sayı ya da Toplam), Değer | Dilimler sıfırdan büyük değerlilerdir (öbürleri söylenir). Renk kategorinin **bütün** satırlardaki sırasıdır (Göster'le değişmez): bütün satırlarda değeri büyükten küçüğe (eşitler kategori sırasıyla) ilk 7 kategori 1–7. renkleri alır, gerisi tek “Diğer” dilimidir (gri); kategori 8 ya da daha azsa hepsi kendi rengiyle. Dilimler bu sırayla, saat 12'den saat yönünde, Diğer sonda. Pay = değer / toplam × 100, bir kesir basamağı, yarımlar çifte. Halkanın ortasında toplam. |
| **Histogram** | Değer, Aralık sayısı (Otomatik ya da 1–100), Ortalama ve ortanca çizgileri (açık) | Otomatik: Sturges, k = ⌈log₂ n⌉ + 1 (tam sayılarla), en çok 50. En az = en çok ise tek aralık. Değer x'in aralığı i = ⌊k·(x − a)/(b − a)⌋, [0, k − 1]'e sıkıştırılır (a en az, b en çok; s basamaktaki tam sayılarla kesin): son aralık kapalıdır. Aralık sınırları a + j·(b − a)/k, s + 2 basamakta yarımlar çifte yazılır. |
| **Kutu** | Değer, Grupla (isteğe bağlı) | Grup başına §3'ün çeyrekleri; çitler Q1 − 1,5·ÇA ve Q3 + 1,5·ÇA (s + 3 basamakta kesin); bıyıklar çitlerin içindeki en küçük ve en büyük değer; çitlerin dışındakiler aykırı (her biri bir nokta); ortalama ×. En çok 30 grup, kategori sırasıyla; gerisi gösterilmez, sayısı söylenir. |
| **Saçılım** | X, Y, Eğilim çizgisi (açık) | İki değeri de sayı olan satırlar noktadır; öbürleri söylenir. Eğilim: en küçük kareler y = a + b·x (float64; önce ortalamalar, sonra satır sırasıyla Sxx, Sxy, Syy toplamları), R² = Sxy² / (Sxx·Syy); n < 2 ya da Sxx = 0 ise çizgi yok, Syy = 0 ise R² yok. Yazısı “y = b·x + a; R² = r” (sayılar 4 anlamlı basamakta, §5; a eksiyse “y = b·x - |a|”). |
| **Çizgi** | X, Y (bir ya da birkaç sütun; her biri bir seri), Seri (tek Y'de, isteğe bağlı), Toplama: Yok, Sayı, Toplam, Ortalama, En az, En çok, Ortanca | Seri başına noktalar X'in tam değerine göre artan; Yok'ta her satır bir nokta (eşit X'ler satır sırasıyla), öbürlerinde X'in her farklı değeri bir nokta. X'i ya da Y'si sayı olmayan satır atlanır, söylenir. En çok 8 seri (sütun sırasıyla ya da kategori sırasıyla); gerisi gösterilmez, söylenir. |

- **Renkler:** tek serili grafikler (Çubuk, Histogram, Kutu, Saçılım, tek serili Çizgi) 1. renkle; Pasta ve çok serili Çizgi kategori
  renkleriyle sırayla, döngüsüz (§1. ilke: 8'den çok renk üretilmez). Renkler arayüz jetonlarıdır (§10).
- **Araç tabloları:** İşlemler tablosunun grafiği aracına göre başlar (`specs::for_table`: aracın kimliği ve tablonun sütunlarıyla;
  araçların çıktısı değişmez): ROC (`suitability.roc`) Çizgi, X Yanlış pozitif oranı (ya da Alan oranı), Y Doğru pozitif oranı, Toplama
  Yok; Kesit (`multidim.profile`) Çizgi, X Uzaklık (m), Y Değer, Seri Çizgi; Zaman serisi (`multidim.series`) Çizgi, X ilk sütun (Adım ya
  da Bant), Y öbür sayı sütunları (en çok 8); Histogram (`raster.histogram`) Çubuk, Kategori Alt sınır, Toplam Sayı. Aracın istediği
  sütunlar tabloda yoksa ya da önerisi olmayan tabloda Çubuk: Kategori ilk sütun, Toplam, Değer ilk öbür sayı sütunu (yoksa Sayı).
- **Başlangıç ve tamamlama** (`specs`, iki platform aynı kuralla): katmanın ilk grafiği Çubuk'tur. Tür değişince seçilen sütunlar kalır,
  toplama türün kendininkiyle başlar (Çizgi'de Yok, öbürlerinde Sayı). Eksik ya da artık olmayan sütun türün istediği ilk uygun sütunla
  tamamlanır (`specs::complete`): kategori ilk sayı olmayan ya da değer listeli sütun; değer, X ilk sayı sütunu; Y X'ten başka ilk sayı
  sütunu; olmayan grup ve seri kalkar, birden çok Y'de seri yoktur; türün almadığı toplama Sayı olur. İstatistik'in Grafik'i sayı
  alanında Histogram, öbürlerinde Çubuk açar (`specs::for_column`). Başka katman seçilince tür kalır, sütunlar yeniden seçilir.

### 5. Eksenler, sayılar ve yerleşim

- **Eksenin bölmeleri** (kesin ondalık): aralık [lo, hi], en çok m bölme; çubuğun ve histogramın değer ekseninde lo ≤ 0 ≤ hi yapılır.
  lo = hi ise: değer 0'sa [0, 1], değilse |v|'nin ilk anlamlı basamağı 10ᵉ ile bir basamak aşağısı: [v − 10ᵉ⁻¹, v + 10ᵉ⁻¹] (101 için
  91…111). Adım d·10ᵉ (d ∈ {1, 2, 2,5, 5}, e tam sayı) adımların küçükten büyüğe sırasında ⌈hi/adım⌉ − ⌊lo/adım⌋ + 1 ≤ m olan
  ilkidir; tam sayı eksenlerinde adım tam sayıdır (1, 2, 5, 10, 20, 25 …). Tam sayı eksenleri: Sayı'nın, histogramın sayısının; değerlerinin
  hepsi tam sayı olan (yazılışında kesir basamağı olmayan) eksenler: çubukta Toplam, En az ve En çok, histogramın, saçılımın ve çizginin
  x'i, saçılımın ve çizginin y'si. Eksen [⌊lo/adım⌋·adım, ⌈hi/adım⌉·adım], bölmeler adımın katları. Yazısı adımın basamağıyla (e < 0'da
  −e, d = 2,5'te bir fazla), ondalık noktayla, binlik ayırıcısız (DESIGN.md §10.3); x yazıları çerçevenin içinde tutulur, bir öncekine
  değecek olan yazılmaz.
- **4 anlamlı basamak** (eğilimin yazısı): değerin ikili tam değeri bilimsel gösterimde 3 kesir basamağına yarımlar çifte yuvarlanır,
  sonra üssüz yazılır (`5.234e2` → `523.4`, `1.000e-3` → `0.001000`); eksi sıfır `0`.
- **Çerçeve** (CSS pikseli): genişlik W, yükseklik H, yazı boyu f (arayüzün `--fs-xs`'i) ve arayüz yazı tipinin ailesi; u = f / 12.
  Yazının genişliği tahmindir: rakamlar, nokta, virgül, eksi ve yüzde ailenin rakam genişliğiyle, öbürleri ortalama harf genişliğiyle
  (`ops::charts::fonts`; aileler KentOS UI'ın `typography`'sindeki ölçülmüş ortalamalarla). Satır yüksekliği 1,4f.
- **Yerleşim:** 12u kenar boşluğu; üstte y ekseninin adı (sol, ikincil mürekkep), altta x ekseninin adı (sağ); y bölmelerinin yazıları
  çizim alanının solunda sağa dayalı, 6u aralıkla; x yazıları altında. m: y ekseninde ⌊yükseklik / 2,5f⌋, x ekseninde ⌊genişlik / 6,5f⌋,
  en az 2. Izgara yatay kıl çizgileridir (saçılımda dikey de), taban çizgisi 0'dadır.
  - Çubuk: bant = genişlik / çubuk sayısı; çubuk min(24u, 0,8·bant) genişliğinde, ucu 4u yuvarlak, tabanı köşeli. Kategori yazıları
    bandın genişliğine sığdırılır (“…” ile); bant 24u'dan darsa her k'ıncı yazı (k = ⌈28u / bant⌉).
  - Histogram: aralıklar bitişik, aralarında 2u zemin boşluğu; ortalama ve ortanca dikey çizgi, adları zeminin rengindeki küçük bir
    kutuda (çubukların üstünde okunur); iki ad birbirine değecekse ikincisi bir satır aşağıda.
  - Kutu: kutu min(32u, 0,6·bant); ortanca 2u çizgi; bıyık uçları kutunun yarısı; aykırılar 3u halka; ortalama ×.
  - Pasta: halka, iç yarıçap 0,58R; dilimler arasında 2u zemin boşluğu; sağda lejant (renk, ad, değer ve pay iki sütunda); lejant
    sığmazsa son satır “+k dilim”.
  - Saçılım: nokta 4u ve 2u zemin halkası (2 000'den çok noktada 2u, halkasız); eğilim çizgisi ve yazısı (sağ üst).
  - Çizgi: 2u çizgi; seride 60 ya da daha az noktada 4u işaretler; iki ve daha çok seride lejant üstte sağda (çizgi anahtarı ve ad).
- **Çizim listesi:** dikdörtgen (yuvarlak ucuyla), çizgi, kırık çizgi, daire, dilim, çarpı, yazı (hiza, ağırlık, boy) ve toplu noktalar;
  her biri rolüyle (zemin, ızgara, eksen, birinci, ikinci ve üçüncü mürekkep, 1.–8. seri, Diğer, seçim) ve öğesiyle. Platform rolü temanın
  rengine çevirir; yerleşim, kırpma ve tahminler çekirdektedir.

### 6. Seçimle bağ

- Her grafik öğesinin (çubuk, dilim, aralık, kutu, nokta; İstatistik'te değer satırı) satırları ve onların nesneleri vardır.
- **Tıklamak** öğenin nesnelerini seçer; Shift seçime ekler, Ctrl seçimden çıkarır. Saçılımda sürükleyerek çizilen kutu içindeki noktaları
  aynı kurallarla seçer. Pasta'nın lejant satırı dilimidir. İşlemler tablosunun satırları nesne değildir: tıklama seçim yapmaz.
- **Vurgu:** çizimin seçimi değişince öğelerin durumu yeniden bulunur: nesnelerinden biri seçili olan öğe seçim rengiyle (vurgu, ADR 0195'in
  `highlightColor`'u değil, arayüzün vurgusu) çizilir, böyle bir öğe varken öbürleri %35 saydamlıkta. Saçılımda seçili noktalar seçim rengiyle
  en üstte; çizgide seçili satırların işaretleri.
- **Üzerine gelme:** öğe birinci mürekkeple 1,5 px çerçevelenir (noktada halkalanır) ve ipucu çıkar: değer önce, ad sonra (kategori, toplama ve değeri, seçili nesne sayısı;
  aralıkta sınırlar; kutuda beş değer, ortalama ve aykırı sayısı; noktada X, Y ve nesnenin adı). Hedef öğeden büyüktür: çubukta bütün
  bant, noktada 12u içindeki en yakın nokta (eşitlerde satır sırasıyla ilki).

### 7. Grafik | Tablo, Panoya kopyala, kaydet

- **Tablo görünümü** grafiğin sayılarıdır (başlıklarda sütunların adları): Çubuk kategori, Nesne ve (Sayı dışında) toplamanın adı; Pasta
  kategori, Nesne ya da “Toplama · değer”, Pay (%); Histogram Alt sınır, Üst sınır, Sayı; Kutu (gruplanınca Grup,) Nesne, En az, Birinci
  çeyrek, Ortanca, Üçüncü çeyrek, En çok, Ortalama, Aykırı; Saçılım X ve Y; Çizgi Seri, X, Y (Yok'ta Değer, öbürlerinde toplamanın adı).
  Sayı sütunları sağa yaslı; sütunlar genişliği paylaşır.
- **Panoya kopyala:** bu tablo sekmeyle ayrılmış. **Kaydet ▾:** PNG (2×) ve SVG; açık temanın renkleriyle beyaz kâğıtta, arayüz yazı
  tipiyle. SVG çekirdeğin çizim listesinden yazılır; masaüstünde PNG onu resvg ile, web'de tarayıcı tuvaliyle çizer.
- İstatistik'te Panoya kopyala özet tabloyu, değer listesi açıkken seçilen alanın istatistiklerini ve değerlerini de kopyalar.

### 8. Arayüz

- **İstatistik sekmesi** (alt panel; Veri › Tablo › İstatistik, CAD'de Yönet › Tablo; Katmanlar'da katmanın sağ tıkı; `ALANISTATISTIKLERI`,
  `ISTATISTIKPANELI`, `FIELDSTATS`; `ISTATISTIK` Özet istatistik'indir):
  - üstte Katman ▾ (açılışta seçili ilk nesnenin katmanı, yoksa etkin katman; Tablo sekmesinin kuralı; liste katmanları nesne sayılarıyla,
    kutu katmanın yolunu gösterir; açılan sekme katmanı tutar, çizimde başka katmanda seçim onu değiştirmez), Göster: Tümü, Seçililer,
    Görünümdekiler (Tablo'nunki), sayaç “n nesne”, sağda Grafik (seçilen alanın: sayıda Histogram, öbürlerinde Çubuk) ve Panoya kopyala;
  - solda özet tablo (§3), satır seçilir; sağda seçilen alanın istatistikleri ve Değerler listesi; değer satırına tık seçer (§6).
- **Grafik sekmesi** (Veri › Tablo › Grafik ▾ ve altında altı tür; CAD'de Yönet › Tablo; katmanın sağ tıkı; İşlemler'in sonuç
  tablosunda Grafik; `GRAFIK`, `GRAFİK`, `CHART`):
  - üstte türün altı simgeli düğmesi (adı ve ne yaptığı ipucunda), sağda Grafik | Tablo, Panoya kopyala, Kaydet ▾;
  - solda ayarlar sütunu (260 px): önce Kaynak ▾ (katmanlar nesne sayılarıyla ve varsa İşlem tablosu) ve Göster (katmanda), sonra türün
    ayarları (seçim kutuları: alan listeleri yalnız uygun türdeki sütunlar; geometrinin sütunları “geometri” notuyla; Çizgi'nin Y'si en çok
    8 sütunun işaretlendiği, açık kalan liste; Ortalama ve ortanca ile Eğilim çizgisi anahtar), altında notlar (atlanan değerler,
    gösterilmeyen gruplar; ⚠ ile); sağda grafik. (Kaynak ve Göster üst çubuktaydı: 1100 px'te çubuk iki satıra taşıyordu.)
  - Saçılım'da sürükleyerek çizilen kutu içindeki noktaları seçer (§6); imleç öğenin üstünde el olur.
  - İşlemler penceresinin sonuç tablosunda **Grafik**: pencere kapanır, tablonun grafiği (aracın önerisiyle, §4) açılır; tablonun
    satırları nesne olmadığından tıklama seçim yapmaz.
- İki sekme alt paneli en az 300 px yükseklikte açar (grafiğe yer).
- **Katmanlar'ın sağ tıkı:** Alanlar…'ın altında **İstatistik** ve **Grafik**: o katmanla açar.
- **Ayarlar** oturumdadır; başka çizim açılınca sıfırlanır (Tablo sekmesi gibi).
- **Komutlar:** `data.statistics`, `data.chart`; türler `chart.bar`, `chart.pie`, `chart.histogram`, `chart.box`, `chart.scatter`,
  `chart.line` (Çubuk grafik, Pasta grafik, Histogram, Kutu grafiği, Saçılım grafiği, Çizgi grafik): Grafik sekmesini o türle açar.
- **Alt panelin sekme satırı** sığmayınca etkin olmayan sekmeler yalnız simgeleriyle görünür (adları ipucunda), dokun sekmeleri gibi:
  bugün 1100 px'te Uyarılar görünmüyordu (web; masaüstünde KentOS UI'ın sekmeleri zaten sıkışır ve taşanı listeye alır).
- **Simgeler** (yeni, sorulmadan): İstatistik, Grafik ve altı tür; her biri ayrı.

### 9. Komutlar ve otomasyon

Grafikler ve istatistikler belgeyi değiştirmez: ürün komutu yoktur (seçim de oturumun durumudur). Python ve MCP'ye yüzey açılmaz (kapsam
dışı; çekirdeğin hesabı Python başvurusunun karşılığıdır).

### 10. Renkler

- Grafik jetonları `tokens.css`'te (iki tema): `--chart-1` … `--chart-8` (kategori sırası) ve `--chart-other`; ızgara ve eksen arayüzün
  çizgileridir (`--c-line`, `--c-line-strong`), zemin panelinki (`--c-panel`), yazılar mürekkep jetonları, seçim vurgu rengi. Masaüstünde
  aynı değerler `charts/palette.rs`'te; bir test `tokens.css`'le karşılaştırır. Dizi görselleştirme kılavuzunun doğrulanmış sırasıdır
  (mavi, turuncu, deniz yeşili, sarı, eflatun, yeşil, mor, kırmızı); koyu temada adımları koyu zemine göre. Renk denetleyicisi KentOS'un
  panel zeminlerinde koşar (Doğrulama). Kaydedilen resim açık temanın renkleriyle beyaz kâğıttadır (ızgara `#d3d9df`, eksen `#b5bfc9`).
- Metin rengi hiçbir zaman seri rengi değildir: değerler, adlar ve lejant mürekkep jetonlarıyla, yanlarında renk örneği.

### 11. Performans

- **Nerede:** web'de çekirdek WASM'da, verisi bir kez kurulan `ChartWork` nesnesinde (sütunlar birleşik metin ve sınırlar dizisiyle,
  JSON'suz); seçim değişince yalnız vurgu yeniden hesaplanır (seçili satırlar `Uint32Array`). Saçılımın noktaları `Float32Array` olarak
  gelir. İstatistik'in özet tablosu alan alan karelere bölünür (karede en çok 8 ms, en az bir alan), web'de de masaüstünde de arayüz donmaz.
  Masaüstünde grafik bir kez çizilir ve grafik, vurgusu, alanı ya da tema değişene dek tutulur; üzerine gelme ayrı katmandadır.
- **Bütçeler** (release; masaüstü / web):

| İş | Masaüstü | Web |
|---|---|---|
| Verinin kurulması: 100 000 nesne, 2 sütun | ≤ 40 ms | ≤ 80 ms |
| Özet tablonun bir alanı: 100 000 nesne | ≤ 12 ms | ≤ 25 ms |
| Seçilen alanın istatistikleri ve değerleri: 100 000 nesne | ≤ 25 ms | ≤ 50 ms |
| Çubuk (Toplam, 30 kategori): 100 000 nesne, model ve yerleşim | ≤ 15 ms | ≤ 30 ms |
| Histogram ve Kutu (30 grup): 100 000 nesne | ≤ 25 ms | ≤ 50 ms |
| Saçılım: 100 000 nokta, model ve yerleşim | ≤ 25 ms | ≤ 50 ms |
| Seçimin vurgusu: 100 000 nesne | ≤ 4 ms | ≤ 8 ms |
| Üzerine gelme (tıklama sınaması): 100 000 nokta | ≤ 1 ms | ≤ 2 ms |

## Uygulama

- **Çekirdek** `crates/shared/geometry-core/src/ops/charts/`: `data` (sütunlar, okuma, kimlik ve sıra), `fields` (§3), `specs` (§4'ün
  başlangıcı ve tamamlaması, araç tabloları), `model` (altı tür), `ticks` ve `fonts` (§5), `layout` (çizim listesi, tıklama, öğenin yeri),
  `svg`, `work` (verinin bir kez okunduğu iş: grafik, vurgu, yerleşim, tıklama, kutu içindekiler), `json` (yalnız web'in sınırı).
  `ops::statistics`'e `figures_of`; sayıların okunması ayırmasız, `Dec::to_f64` Clinger'in hızlı yoluyla (tamsayı kısmı 2⁵³'ten küçük,
  ölçek ≤ 22: bölme doğru yuvarlar, metinden okunanla bit bit aynı; `a_decimals_double_is_its_texts`), ölçek eşitken karşılaştırma doğrudan;
  grafiğin haritaları çekirdeğin hızlı karmasıyla (`store::IdHasher`). Sonuçlar değişmez.
- **WASM** `crates/wasm/geometry-wasm/src/charts.rs`: `ChartWork` (sütunlar birleşik metin ve UTF-16 boylarıyla; özet, ayrıntı, grafik,
  `complete`, `forColumn`, `forTable`, vurgu, `marked`, yerleşim, noktalar, tıklama, `elementAt`, kutu içindekiler, SVG).
- **Web:** `model/chartData.ts` (katmanın ve tablonun sütunları), `model/ops/charts.ts` (`ChartData`), `app/charts.ts` (komutlar, oturumun
  seçimleri, `shownCharts`: sekmelerin gösterdiği, izler okur), `ui/bottom/StatisticsPanel.ts`, `ui/bottom/ChartPanel.ts`,
  `ui/charts/drawChart.ts`, `ui/charts/layerData.ts`, `styles/charts.css`, `tokens.css`'in grafik jetonları; alt panelin iki sekmesi ve
  sıkışık sekme satırı (`logPlan.ts`, `layoutPlan.ts`, `BottomPanel.ts`, `shell.css`), şerit (CBS'de Veri › Tablo, CAD'de Yönet › Tablo;
  Grafik ▾ altında altı tür), menüler, sekiz simge, Katmanlar'ın sağ tıkı, İşlemler penceresinin Grafik düğmesi (`ToolDialog.ts`).
- **Masaüstü:** `kentos_interaction::chart_data`; `apps/desktop/src/charts/` (`mod` seçimler ve olaylar, `now` verinin ve grafiğin
  önbelleği, `view`, `canvas` grafik ve üzerine gelme katmanı, `paint` şekiller tiny-skia ile ekranın pikselinde tek resim, noktalar
  doğrudan dökülür, yazılar Iced'in; `export` SVG ve resvg ile PNG, KentOS'un yazı tipleriyle; `palette`; `data`), `statistics/`
  (özetler karede 8 ms), `chart_scenes.rs`; `bottom.rs`, `layout.rs`, `layout_plan.rs`, `log_plan.rs`, `message_log.rs`, `app.rs`
  (iletiler, komutlar, özetlerin kare aboneliği, yeni çizimde sıfırlama), `catalog.rs`, `layering.rs`, İşlemler'in Grafik'i
  (`processing/`). Bağımlılık: çalışma alanının tiny-skia'sı, resvg'nin `text` özelliği ve Iced'in kodeksiz resmi; kilit dosyasında yeni
  paket yok.
- **Ortak:** `fixtures/charts/v1/cases.json` (`scripts/fixtures/chart_cases.py`), resimlerin ve izin sahnesi
  `fixtures/interaction/v1/charts.kcad` (`chart_scene.py`), iz `charts.json` (oynatıcılarda `chartPress` eylemi, `chart` ve `statistics`
  beklentileri), `fixtures/shell/v1`'in sekmeleri.

## Doğrulama

- **Bağımsız başvuru:** `python3 scripts/fixtures/chart_cases.py --check` (25 alan, 25 grafik, 12 eksen, 12 anlamlı basamak ve 20 genişlik
  durumu; Python'un kesirleriyle, KentOS kodu olmadan); çekirdekte durumlar ve yerleşimin özellikleri, `specs`'in testleri
  (`cargo test -p kentos-geometry-core --test all charts::`).
- **Ortak iz** `charts.json` iki platformda, üç varyantta geçti: İstatistik'in on özet satırı (ayrıca Python'la bağımsız hesaplandı:
  toplamlar, ortalamalar, en az ve en çok, alanlar ve çevreler), Çubuk'un ilk sütunu, çizimdeki seçimin vurgusu, çubuğa basmak, Shift ve
  Ctrl, Pasta'da aynı vurgu.
- **Masaüstü testleri** (`charts::tests`, 12): komutlar ve katman, basma ve vurgu, Seçililer, başka katman, ayarlar, araç tablosu,
  İstatistik'in sayımı ve değer seçimi, yeni çizim, PNG (resvg, 960 × 600), resmin zemini ve renkleri, jetonların `tokens.css`'le aynılığı.
- **Resimler:** masaüstü `KENTOS_SHOTS_ONLY=grafik-… cargo test -p kentos-desktop tools_screens -- --ignored`, web
  `shots.mjs charts`; on sahne, 1440 × 900 ve 1100 × 650, iki tema, yan yana karşılaştırıldı (masaüstünde Kaynak'ın yazısı, özet tablonun
  genişlikleri ve Tablo görünümü web'e göre düzeltildi).
- **Renkler:** sıra dizi görselleştirme kılavuzunun denetleyicisinden (`validate_palette.js`) iki temada, KentOS'un panel zeminlerinde
  (`#f4f6f8`, `#212932`) geçti; komşu dilimlerin renk körlüğündeki ayrımı için lejant ve Tablo görünümü vardır.
- **Süreler** (12 Ekim, Apple M5, 10 çekirdek, macOS 26.6.2, release; masaüstü
  `cargo test --release -p kentos-interaction --test perf charts_on_a_large_layer -- --ignored --nocapture`, web
  `CHARTS_BENCH=1 … scripts/perf/charts.test.ts` gönderilen WASM'la, Node 24.14; ortanca, ms):

| İş, 100 000 nesne | Masaüstü (bütçe) | Web (bütçe) |
|---|---|---|
| Verinin kurulması (2 sütun; web'de iki sütunun özeti dahil) | 32,9 (40) | 47,6 (80) |
| Özet tablonun bir alanı | 3,2 (12) | 5,3 (25) |
| Seçilen alanın istatistikleri ve değerleri | 15,0 (25) | 24,9 (50) |
| Çubuk (Toplam, 30 kategori), model ve yerleşim | 1,7 (15) | 3,2 (30) |
| Histogram, model ve yerleşim | 3,3 (25) | 5,1 (50) |
| Kutu (30 grup), model ve yerleşim | 3,1 (25) | 5,8 (50) |
| Saçılım (100 000 nokta), model ve yerleşim (web'de noktaların aktarımı dahil) | 14,2 (25) | 29,2 (50) |
| Seçimin vurgusu (50 000 seçili nesne) | 0,07 (4) | 0,13 (8) |
| Üzerine gelme, tıklama sınaması (en yavaşı) | 0,33 (1; 0,49) | 0,36 (2; 0,74) |
| Masaüstünde saçılımın resmi, 100 000 nokta, 1800 × 520 piksel | 19,7 (40) | — |

İlk ölçümde üç iş bütçeyi aşıyordu (kurulum 57 ms, alanın istatistikleri 39 ms, saçılım 28 ms): değer listesi, tarih ya da evet/hayır
olmayan alanda da gösterim metni üretiliyordu, sayılar ayırarak okunup her farklı değer için metin yazılıyordu, `Dec::to_f64` her sayıyı
metne yazıp geri okuyordu (~150 ns), haritalar SipHash'liydi ve ayrıntı rakamları iki kez sayıyordu. Düzeltmeler sonuçları değiştirmedi
(başvuru, çekirdek testleri ve iz aynı).
