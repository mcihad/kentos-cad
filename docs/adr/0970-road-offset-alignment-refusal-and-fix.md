# 0970 — Paralel aksın kıvrılması ve kendini kesmesi: kesin yer ve sayıyla çözüm

Tarih: 2026-11-05. Durum: kabul (Y64). Kapsam ROAD-ALIGN-09.

## Sorun

ROAD-ALIGN-09: "Offset alignment'ı kaynak aksla kalıcı ilişkilendir; keskin/ters kurpta cusp/self-intersection detect ve
ret/explicit çözüm sun."

Paralel aks kaynağına bağlıydı (ADR 0901 §8) ve sivri uç reddediliyordu. Eksikler:

- **Kendini kesme testsizdi.** Bulunan yer de örnekleme ızgarasının km'siydi, kesişmenin kendisi değil (Y61'de "km
  596,000 ile 602,996 yakınında").
- **Küçük ötelemede kaçırılabiliyordu.** Kırığın iç yanındaki kesişme, kırıktan |d|·tan(δ/2) önce ve sonradır.
  Örnekleme metrede birdir ve komşu iki parçayı karşılaştırmaz; bu uzaklık bir metrenin altındaysa (ör. 45°'de 0,3 m
  öteleme, 0,12 m) iki kesişen parça komşu düşer.
- **Çözüm genel bir cümleydi:** "Ötelemeyi küçültün ya da kurpları genişletin". Hangi kurp, ne kadar, söylenmiyordu.
- **Km yanlış yazılıyordu.** Mesajdaki km aslında iç km'ydi (aksın başından metre). Başlangıç km'si 2+000 olan aksta
  "km 597,101" yazıyordu; doğrusu 2+597,101.
- **Pencere farklı söylüyordu.** Güzergâh penceresi kendi kısa cümlesini kuruyor, kendini kesmeyi de "kıvrılıyor" diye
  yazıyordu.

## Araştırma

- **Paralel eğri (offset curve):** κ eğrilikli eğrinin d ötelemesinin eğriliği κ/(1 + κ·d)'dir; 1 + κ·d = 0'da sivri
  uç olur. Sivri uçlar yerel tekilliktir. Kendini kesme ise küresel bir özelliktir, uzak parçalar arasında olur.
  Kaynak: R. T. Farouki ve C. A. Neff, "Analytic properties of plane offset curves", *Computer Aided Geometric Design*
  7, 1990.
- **Kırık:** İki doğrunun iç yandaki ötelemeleri, kırıktan d·tan(δ/2) uzakta kesişir. Bu, yatay kurbun teğet boyu
  T = R·tan(Δ/2)'nin R = d hâlidir.
- **Çözüm sayıları:** Sivri uç yarıçap > |d| ister. Bu yüzden ya kurbun yarıçapı |d|'nin üstüne çıkarılır ya da
  öteleme en sıkı yarıçapın altına indirilir. Kırıkta iç yan her ötelemede kendini keser; çözüm yarıçapı |d|'den büyük
  bir kurptur.

## Karar

### Çekirdek (`alignment/offset.rs`)

- **`Cusp`:** km aralığına ek olarak `element` ve `radius` taşır; radius o yanda en sıkı yarıçaptır (1/|k|).
- **`Crosses { a, b, kink }`:** P(a) = P(b) olan iki km'dir ve kesindir. `kink`, aradaki kırığın PI'ıdır (varsa).
- **Bulma:**
  - Örneklenmiş ötelemenin kesişen parçaları Newton yöntemiyle iyileştirilir. Yöntem P(a) − P(b) = 0'ı çözer;
    türev (1 + κ·d)·(cos θ, sin θ)'dır ve yakınsama 10⁻¹¹ m'dir.
  - Ayrıca her kırığın iç yanı, örneklemeden bağımsız olarak S ∓ |d|·tan(|δ|/2)'den başlatılır.
  - Bulunanların en küçük (a, b)'si söylenir: aks boyunca ilk kesişme.
  - Newton aynı noktaya (a = b) inerse örneklenmiş kesişme geçerli kalır.

### Söz (`road_alignments::offset_words`)

Komutun reddi (`offset_cusp`) ve Güzergâh penceresinin durumu aynı cümleyi söyler; km'ler kaynağın km yazısıyladır:

- **Kıvrılma:** "“Sol kenar” paralel aksı km 2+597.101–2+602.598 arasında kıvrılıyor: p1 kurbunun yarıçapı orada 7.000
  m'ye iniyor, öteleme 7.500 m. p1 kurbunun yarıçapını 7.500 m'nin üstüne çıkarın ya da ötelemeyi 7.000 m'nin altına
  indirin."
- **Kırıkta kesişme:** "… km 2+596.893 ile km 2+603.107 arasında kendini kesiyor: p1 kurpsuz bir kırık ve iç yanında
  her öteleme kendini keser. p1 noktasına yarıçapı 7.500 m'nin üstünde bir kurp koyun."
- **Başka kesişme:** "… km 0+128.114 ve km 0+579.160 noktaları aynı yere düşüyor: aksın bu iki yeri 50.000 m'lik
  ötelemeye göre birbirine çok yakın. Ötelemeyi küçültün ya da aksın bu iki yerini birbirinden uzaklaştırın."

Pencere reddi kırmızıyla yazar. Kırmızı olup olmadığı artık yazının başına bakılarak değil, durumla birlikte dönen
bayraktan okunur; eski ölçüt yeni cümleyle bozulmuştu.

## Ölçüm

### Bağımsız referans

`scripts/fixtures/road_alignment_command_cases.py` KentOS kodu olmadan çalışır:

- ötelemeyi 0,25 m'de bir örnekler;
- kesişen parçaları shapely'nin STRtree'siyle bulur;
- her birini mpmath'in `findroot`'uyla 40 basamakta iyileştirir.

Yeni durumlar:

- **"Kurp kaldırılır: sol kenar kırıkta kendini keser"** (`edit`): δ = 45°, d = 7,5. Bulunan km'ler ders kitabının
  600 ∓ 7,5·tan 22,5° = 600 ∓ 3,107 değeriyle 10⁻⁹ içinde aynıdır; referans bunu kendisi de sınar.
- **"Kolları yaklaşan dönüşün iç kenarı kendini keser"** (`create`): iki 60 m'lik kurp vardır. Kurplar 50 m'lik
  ötelemeden büyük olduğu için sivri uç yoktur; kollar yaklaşır ve iç kenar km 0+128,114 ile 0+579,160'ta kendini keser.

Var olan iki kıvrılma durumu yeni cümleyi ve km yazısını aldı. Rust'ta ve Python'da geçer.

### Çekirdek testi

`alignments::a_kinks_inner_offset_crosses_at_the_textbook_stations` şunları sınar:

- 0,3 m, 7,5 m ve 40 m ötelemede kırığın iç yanı ders kitabının km'lerinde kesişir (10⁻⁹) ve PI'ı p1'dir;
- dış yanı kurulur;
- 7 m'lik kurpta 7,5 m öteleme `p1/…` elemanını ve 7 m'yi söyler.

0,3 m'de kesişme kırıktan 0,124 m uzaktadır; örnekleme tek başına bu kesişmeyi komşu iki parçaya düşürür.

### Pencere testi

`the_window_says_the_parallel_axis_fix`, Ana aks p1'de sağa döndürülünce Sağ kenar'ın (5 m) durumunu sınar:

- 3 m'lik kurpta "yarıçapını 5.000 m'nin üstüne çıkarın ya da ötelemeyi 3.000 m'nin altına indirin" yazar;
- kurpsuzda "p1 kurpsuz bir kırık" yazar;
- ikisi de kırmızıdır.

Resimler `.run/gui-evidence/yol-y64/` altındadır.

## Açık

- **Kollar yaklaşınca:** Aksın kendine yaklaştığı yerde kurulabilecek en büyük öteleme hesaplanmaz. Bu, eğrinin yerel
  özellik boyudur (medial axis); ayrı bir hesap ister. Mesaj yer ve ötelemeyi söyler.
- **Dış yan:** Kırığın dış yanında öteleme bir boşlukla kesilir. Bu eskisi gibi kabul edilir.
