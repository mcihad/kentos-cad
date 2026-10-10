# ADR 0213: Ek işleyiciler

- **Durum:** kabul edildi (2026-10-10). Kapsamı ben belirledim (sahibin sözü: “Benim seçmeme gerek yok sen sıradan devam et”); iki
  platform (masaüstü ve web), bulut, Python ve MCP; ikonlar sorulmadan seçilir; ilke “Performance First”. Madde tek parçada biter.
- **Bağlam belgesi:** TODOS.md `GIS-17`; docs/STYLE.md §4 (işleyiciler), §6 (çizim hattı); ADR 0090 (masaüstünün stilli çizimi), ADR 0091
  (Katman stili), ADR 0093 (Lejant), ADR 0157 (karolar), ADR 0192 (resim nesnesi: resmin kendi dokusu), ADR 0205 (açıklama boyları), ADR
  0212 (etiket motoru), ADR 0232 (Çekirdek yoğunluğu, raster).
- **Araştırma:** QGIS'in işleyicileri (kaynakları: `qgsheatmaprenderer.cpp`: noktalar görüntü hücresine kesilir, dördüncü dereceden (quartic)
  çekirdek `(1 − (d/r)²)²`, en büyük değer kendiliğinden ya da sabit, renk rampadan; `qgspointdistancerenderer.cpp`: noktalar
  sırayla, en yakın merkezli gruba ya da yeni gruba, merkez üyelerin ortalaması; `qgspointdisplacementrenderer.cpp`: halka, iç içe halkalar
  ve ızgara, sembolün köşegeniyle yarıçap; `qgsinvertedpolygonrenderer`: dışarıyı boyamak, “geometrileri önceden işle” ile birleşim;
  boyut yardımcısı: Alan, Yarıçap, Flannery üstelleri; diyagramlar: pasta, çubuk, yığılmış çubuk), ArcGIS Pro'nun semboloji türleri
  (Proportional symbols, Unclassed colors, Bivariate colors, Dot density, Charts, Heat map: Dynamic ve Constant) ve MapLibre'nin ısı
  haritası katmanı (`heatmap-radius`, `-weight`, `-intensity`, rampa). Netcad'in Tematik'i (Sayısal, Grafik Tematik) belgelerinde aynı
  türlerin bir alt kümesidir.

## Bağlam

Stil motoru (docs/STYLE.md) bugün dört işleyici bilir: tek sembol, kategorili, aralıklı, kurallar. Her biri nesneye bir **sembol takımı**
seçer; sembol derlenir, toplu çizime gider. Tematik haritanın öbür yaygın biçimleri yoktur: değerle sürekli değişen renk ya da boy, iki
değişkenin birlikte gösterimi, alanın içinde değerle orantılı noktalar, nesnenin üstünde pasta ve çubuk grafik, nokta yoğunluğunun ısı
haritası, üst üste binen noktaların kümelenmesi ya da yayılması, bir alanın dışının boyanması. Bunların bir kısmı nesne başına sembolün
değiştirilmesidir (renk, boy), bir kısmı nesneden yeni çizim üretir (noktalar, grafikler), bir kısmı da nesnelerin birlikte ele alınmasını
ister (kümeler, ısı, ters alan) ve görünümün ölçeğine ya da kutusuna bağlıdır.

## Karar

### 1. Kapsam

Katman stilinin işleyicisine (`LayerStyle.renderer`, KCAD'de opak değer: şema değişmez) dokuz tür eklenir:

| Tür (`type`) | Arayüzdeki adı | Ne yapar |
|---|---|---|
| `unclassed` | Sürekli renk | Değer, rampada sürekli bir renge (sembolün ana rengi) |
| `proportional` | Orantılı sembol | Değer, sembolün boyuna (nokta ve alanın iç noktasında işaret, çizgide kalınlık) |
| `bivariate` | İki değişkenli renk | İki değerin sınıfları, renk ızgarasında bir renge |
| `dotDensity` | Nokta yoğunluğu | Alanın içinde alan başına değer / nokta değeri kadar rastgele nokta, alan alan renkli |
| `chart` | Grafik | Nesnenin üstünde pasta, çubuk ya da yığılmış çubuk |
| `heatmap` | Isı haritası | Noktaların çekirdek yoğunluğu, rampayla renklenmiş resim |
| `cluster` | Kümeleme | Yakın noktalar tek küme sembolü ve sayısı |
| `displacement` | Yayma | Üst üste binen noktalar merkezin çevresine dağıtılır |
| `inverted` | Ters alan | Alanların dışı boyanır (alanlar boş kalır) |

- Kümeleme ve Yayma, tek noktaları ve nokta olmayan nesneleri bir **iç işleyiciyle** çizer (tek sembol, kategorili, aralıklı, kurallar,
  sürekli renk, orantılı sembol ya da iki değişkenli renk; yoksa katmanın basit görünüşü).
- Arayüz: Katman stili penceresinde işleyici listesi (iki platform), türe göre form, Lejant satırları (iki platform, pafta da), komut
  `cad.layers.renderer` (web, masaüstü, başsız sunucu, Python, MCP).
- **Kapsam dışı:** 2,5B işleyici; QGIS'in sözlük işleyicisi; yazı diyagramı; değerle normalleştirme (ArcGIS'in Normalization'ı; ifadeyle
  yazılır: `Nüfus / $alan`); iki değişkenli ızgaranın döndürülmüş lejantı; ısı haritasında çizgi ve alanların yoğunluğu (Çizgi yoğunluğu
  raster aracıdır, ADR 0232); kümenin içini açan etkileşim (tıklayınca yakınlaşma); nokta yoğunluğunda ölçekle değişen nokta değeri.

### 2. Tanımlar

Ortak: `expr` alanları İfadeyle seç'in dilidir (ADR 0100; `$alan`, `$uzunluk`, alanlar); sayı vermeyen değer “değeri yok”tur. `symbols`
bir sembol takımıdır (alan, çizgi, işaret). `ramp` 2–16 renktir (`#RRGGBB` ya da `#RRGGBBAA`), eşit aralıklı duraklar; `t` (0–1) iki durak
arasında doğrusal karışımla renk olur (kanal kanal, tam sayıya yarım yukarı yuvarlanır; Aralıklı'nın `rampColors`'ı ile aynı kural).
`other` (isteğe bağlı sembol takımı) değeri olmayan nesneleri çizer; yoksa onlar çizilmez (Aralıklı gibi).

#### 2.1 Sürekli renk

`{ type: 'unclassed', expr, min, max, ramp, symbols, other? }`, `min ≤ max` (sonlu). `t = (v − min) / (max − min)`, `[0, 1]`'e
kırpılır; `max = min` ise `t = 0`. `t` **256 basamağa** yuvarlanır (`round(255 t) / 255`): renk 8 bitte zaten bu kadardır, toplu çizim en
çok 256 renkle sınırlı kalır. Renk sembolün **ana rengidir** (§2.10). Lejant: `t = 0, ¼, ½, ¾, 1`'in renkleri ve değerleri.

#### 2.2 Orantılı sembol

`{ type: 'proportional', expr, minValue, maxValue, minSize, maxSize, unit: 'mm'|'px', scaling: 'area'|'radius'|'flannery', symbols,
other? }`; `minValue ≤ maxValue`, `0 < minSize ≤ maxSize ≤ 200`. Boy QGIS'in boyut yardımcısının kuralıdır:
`s = minSize + (maxSize − minSize) · tᵉ`, `t` §2.1'deki gibi, `e` Alan'da 0,5, Yarıçap'ta 1, Flannery'de 0,57.

- **Nokta** ve **alanın iç noktası** (sembol takımının işaret sembolü; iç nokta İç noktada işaret'inkidir, `interior_point`): sembolün
  boyu `s` olur; katmanları oranla büyür (boy, yükseklik, kayma; çizgi kalınlıkları değişmez). Sembolün boyu katmanlarının en büyük
  boyudur. Alanın kendisi takımın alan sembolüyle (varsa) zemin olarak çizilir.
- **Çizgi:** takımın çizgi sembolünün kalınlığı `s` olur; katmanları oranla.
- Boy 256 basamağa yuvarlanır (`t`'nin 255'te biri: en çok 256 farklı sembol).
- Lejant: `minValue`, ortadaki ve `maxValue` değerlerinin sembolleri.

#### 2.3 İki değişkenli renk

`{ type: 'bivariate', exprX, exprY, breaksX, breaksY, colors, symbols, other? }`. `breaksX` ve `breaksY` artan, 1–3 sınır; sınıf sayısı
`n = sınırlar + 1` iki eksende eşit (2, 3 ya da 4). Değerin sınıfı onu aşmayan sınırların sayısıdır (`v ≥ b`; sınır üst sınıfa gider,
Aralıklı'nın alt sınırı dahil kuralı). `colors` `n²` renk: `colors[j · n + i]`, `i` X'in, `j` Y'nin sınıfı (sol alt düşük-düşük). İki
değerden biri yoksa nesne `other`'dır. Sınıflama (penceredeki Sınıfla): eşit sayı (Aralıklı'nın Eşit sayı'sının dilimleri, ArcGIS'in
varsayılanı; değerler çok yinelendiğinden `n` sınıf çıkmazsa eşit aralık) ya da eşit aralık, iki eksen ayrı ayrı; sınırlar sınıfların
ikinciden sonuncuya alt sınırlarıdır. Hazır ızgaralar (Pembe – mavi, Yeşil – mor, Turuncu – mavi) köşe renklerinden çift doğrusal
karışımla, kanal kanal yarım yukarı yuvarlanarak kurulur. Lejant: `n²` satır, ikisinin sınıfıyla (“Nüfus < 100, Gelir ≥ 30”).

#### 2.4 Nokta yoğunluğu

`{ type: 'dotDensity', fields: [{ expr, label, color }], dotValue, dotSize, unit: 'mm'|'px', seed, symbols? }`; 1–12 alan,
`dotValue > 0`, `0 < dotSize ≤ 20`, `seed` 0–2³¹ − 1 (tam sayı). Yalnız alanlar nokta alır; takımın sembolleri zemin olarak çizilir (alan, çizgi,
nokta). Alan başına alanın nokta sayısı `round(v / dotValue)` (yarım yukarı; değeri yok ya da sıfırdan küçükse 0).

- **Yer:** noktalar alanın kutusunda tek biçimli rastgele, içindekiler kabul edilir (çift-tek kuralı, delikler ve çok parça dahil); en
  çok `50 · sayı + 1000` deneme. Rastgelelik SplitMix64'tür, tohumu `seed`, alanın halkalarının köşeleri (FNV-1a, sayıların 64 bitiyle) ve
  alanın sırası (0, 1, …) birlikte verir: aynı alan her kurulumda aynı noktaları alır (ArcGIS'in sabit yerleşimi), alan değişince
  değişir. Denemede `x = minX + u · genişlik`, `y = minY + w · yükseklik`; `u`, `w` 64 bitin üst 53 biti / 2⁵³.
- **Çizim:** nokta, alanın renginde, çerçevesiz daire işareti; alanlar sırayla (ilk alan altta). Bir katmanda en çok **1 000 000**
  nokta; fazlası çizilmez, katman başına bir kez uyarı olarak söylenir (iki platformda: “… noktası çizilmedi … Nokta değerini
  büyütün.”). Sınır katmanındır: masaüstünde katman parçalara bölünmeden kurulur (§3).
- Lejant: “1 nokta = `dotValue`” ve alan başına renkli nokta.

#### 2.5 Grafik

`{ type: 'chart', kind: 'pie'|'bar'|'stacked', fields: [{ expr, label, color }], size, unit: 'mm'|'px', sizeBy?, maxValue?, barWidth?,
outline?, symbols? }`; 1–12 alan, `0 < size ≤ 200`. Nesnenin **yeri**: nokta kendisi, alan iç noktası (çok parçalıda en büyük
parçanın), çizgi uzunluğunun ortası. Değeri olmayan alan 0 sayılır; pastada ve yığılmış çubukta sıfırdan küçük değer de 0'dır, çubukta
sıfırdan küçük değer aşağı iner. Hiçbir şey çizilmeyecek nesneye (toplamı 0) grafik çizilmez. Takımın sembolleri zemindir.

- **Pasta:** çap `size`; `sizeBy { minValue, maxValue, minSize, maxSize }` varsa çap toplamdan Alan kuralıyla (§2.2, `e = 0,5`). Dilimler
  alanların sırasıyla, **kuzeyden saat yönünde**; dilimin açısı `360° · v / toplam`. Yay en çok 5°'lik parçalarla çizilir (dilim başına
  `⌈açı / 5°⌉` parça); yayın noktaları ilk noktanın adım adım döndürülmesidir (dilim başına bir sinüs ve kosinüs; bağımsız başvuruyla
  1e−9'da aynı). Dilim merkezinden yıldız biçimlidir: üçgenleri merkezden yelpazedir, kulak kesmeden geçmez (`BatchSink::fan`).
- **Çubuk:** alanlar yan yana, her biri `barWidth` (yoksa `size / 4`) genişliğinde, ortaları yere göre ortalanmış; yükseklik
  `size · v / maxValue` (`maxValue > 0` zorunlu), taban yerde, yukarı.
- **Yığılmış çubuk:** tek çubuk, `barWidth` genişliğinde, alanlar alttan üste; toplam yükseklik `size · toplam / maxValue`.
- **Çerçeve:** `outline { color, width }` (kalınlık `unit`'te) dilimlerin ve çubukların kenarlarını çizer.
- Grafikler dünyada çokgendir (ölçek kâğıdın ölçeği ya da Ekranda sabit'te görünümün çeyrek oktavlık ölçeği; iki durumda da sembollerin
  boyu gibi değişir). Lejant: alan başına renk.

#### 2.6 Isı haritası

`{ type: 'heatmap', radius, unit: 'px'|'m', weight?, max?, ramp, quality, opacity? }`; `radius` px'te `0 < radius ≤ 500`, metrede `> 0`; `quality`
1–5 (hücre `quality` pikseldir); `max > 0` verilirse **sabit**, yoksa **dinamik** (hesaplanan kutunun en büyüğü; QGIS'in kendiliğinden,
ArcGIS'in Dynamic'i); `opacity` 0–1. Yalnız nokta nesneleri ve çok noktalı nesnenin noktaları sayılır; katmanın öbür nesneleri çizilmez
(QGIS gibi).

- **Izgara:** kutu (§3) `c = quality` piksellik hücrelere bölünür; satır ve sütun `⌈genişlik / c⌉`, `⌈yükseklik / c⌉` (en çok 4096).
  Noktanın hücresi `⌊(x − minX) / hc⌋`, `⌊(maxY − y) / hc⌋` (`hc` hücrenin metresi; QGIS'in tam sayıya kesmesi). Yarıçap hücrede
  `R = round(radius_px / c)` (en az 1); px'te `radius`, metrede `radius · ölçek`'ten.
- **Değer:** hücre `(i, j)`'de `D = Σ w · (1 − ((di² + dj²) / R²))²`, `di² + dj² ≤ R²` olan noktalar üstünden (`di`, `dj` hücre farkı;
  QGIS'in dördüncü dereceden (quartic) çekirdeği). `w` ağırlık ifadesi (sayı değilse 1; sıfırdan küçükse 0); kutunun `R` hücre dışındaki noktalar
  katkı vermez. Hücre sırası soldan sağa, yukarıdan aşağı; toplama noktaların sırasıyla.
- **Renk:** `t = D / max` (dinamikte kutunun en büyük `D`'si), 1'de kırpılır; `D = 0` saydamdır (rampanın ilk rengi `t = 0+`'dır);
  rampa §2'deki gibi, **1024 basamakta** (önceden hesaplanan renk tablosu, `round(1023 t)`). Resmin her pikselinin saydamlığı `opacity`
  ile çarpılır.
- **Çizim:** resim kutuya oturan tek resimdir (resim nesnesinin dokusu, ADR 0192 §3); ev sahibinin anahtarıyla gelir, kurulum
  yenilenince eskisi bırakılır.
- Lejant: rampanın `t = ¼, ½, 1`'i “Az”, “Orta”, “Çok” (`t = 0+` rampanın ilk, çoğu zaman saydam rengidir).

#### 2.7 Kümeleme

`{ type: 'cluster', distance, unit: 'px'|'m', symbol?, count?, grow?, renderer? }`; `distance` px'te `0 < distance ≤ 500`, metrede `> 0`.

- **Gruplar** (QGIS'in kuralı, dairesel uzaklıkla): noktalar belgedeki sıralarıyla (çok noktalı nesne parça parça); her nokta merkezi
  ondan `d` (dünyada) uzaklığa kadar olan grupların merkezi en yakın olanına katılır (eşitlikte önce kurulan), yoksa yeni grup kurar;
  grubun merkezi üyelerinin ortalamasıdır (katılınca yeniden hesaplanır ve saklanır). Merkezler `d` boyunda ızgarada aranır (hücreler
  sabit ve hızlı özetle: her nokta dokuz hücreye bakar, haritanın sırası okunmaz).
- **Çizim:** tek üyeli grup iç işleyiciyle kendi yerinde; çok üyeli grup merkezinde küme sembolüyle (`symbol`; yoksa katmanın renginde,
  beyaz çerçeveli 24 px daire), `count` (varsayılan açık) üyelerin sayısını beyaz kalın yazıyla yazar (sembolün boyunun 0,45'i), `grow`
  sembolü `min(2, 1 + ln n / ln 1000)` katına büyütür.
- Nokta olmayan nesneler iç işleyiciyle çizilir. Lejant: “Küme” ve iç işleyicinin satırları.

#### 2.8 Yayma

`{ type: 'displacement', tolerance, unit: 'px'|'m', placement: 'ring'|'rings'|'grid', spacing?, center?, circle?, renderer? }`;
`tolerance` px'te `0 < tolerance ≤ 100`, metrede `> 0`; `spacing` (px, 0–100) aralara eklenen pay.

- **Gruplar** Kümeleme'nin kuralıyla, `tolerance` uzaklıkla.
- **Yerleşim** (ekran pikselinde, `s` grubun üyelerinin işaret sembollerinin en büyük köşegeni `√2 · boy`, en az 4 px):
  - **Halka:** yarıçap `r = max(s/2, n · s / 2π) + spacing`; üye `k` (sırasıyla) merkezden `r` uzaklıkta, kuzeyden saat yönünde `k · 360°/n`.
  - **İç içe halkalar:** ilk halka `r₁ = c/2 + s/2 + spacing` (`c` merkez sembolünün köşegeni, yoksa 0); sonraki halka `rₖ₊₁ = rₖ + s +
    spacing`; halka `k` en çok `max(1, ⌊2π rₖ / s⌋)` üye alır; halkanın üyeleri eşit açılarla, kuzeyden saat yönünde.
  - **Izgara:** `g = ⌈√n⌉` sütun, `⌈n / g⌉` satır, aralık `a = (c/2 + s/2 + s) / 2 + spacing`; üyeler satır satır soldan sağa,
    yukarıdan aşağı; ızgaranın kutusu merkeze ortalanır.
- **Çizim:** merkezde `center` sembolü (yoksa merkezde bir şey çizilmez), `circle { color, width }` halkaların dairesini (iç içe
  halkalarda her halka) çizer; üyeler iç işleyiciyle yeni yerlerinde. Tek üyeli grup yerindedir.

#### 2.9 Ters alan

`{ type: 'inverted', symbols, merge? }`. Yalnız alanlar (alan, daire, kapalı elips ve eğri, tarama değil) sayılır; öbür nesneler çizilmez.

- **Bölge:** kutunun (§3) içinde alanların **dışı**: `merge` kapalıyken çift-tek kuralı (iki alanın örtüştüğü yer yeniden boyanır;
  QGIS'in varsayılanı), açıkken sarım sayısı sıfır olan yer (örtüşmeler boş kalır; QGIS'in “önceden işle”si, birleşimsiz). Alanın
  deliği dışarıdır.
- **Hesap:** tarama çizgisiyle yamuklara ayırma: kenarlar `y`'ye göre olaylardır, etkin kenarlar `x` sırasında; iki kenar arası boşluğun
  açık yamuğu ancak sınırlayan kenarlardan biri değişince kapanır (Seidel'in birleştirilmiş yamukları: `O(n)` yamuk, `O(n log n + n·k)`
  süre, `k` bir yatay çizgiyi kesen kenar sayısı). Kesişen kenarlar (örtüşen ya da kendini kesen alanlar) iki olay arasında komşu çiftin
  kesişme yüksekliğinde ara olaydır. Yamuk kutunun `x` aralığına kırpılır, üçgenlerle doldurulur.
- **Çizim:** alan sembolünün dolgu katmanları bölgeyi, çizgi katmanları yalnız alanların halkalarını (kutunun kenarını değil) çizer.
- Lejant: takımın alan sembolü, “Dışı”.

#### 2.10 Ana renk

Sürekli renk ve İki değişkenli renk sembolün **ana rengini** değiştirir (ArcGIS'in sembol rengi; QGIS'in kilitsiz katmanları gibi):
alan sembolünde dolgu katmanlarının rengi (düz, tarama, degradenin ilk rengi, desen ve iç noktada işaretin işaret rengi), çizgi
katmanları (çerçeve) değişmez; çizgi sembolünde çizgi katmanlarının rengi ve çizgi boyunca işaretin işaret rengi; işaret sembolünde
şeklin dolgusu (açık şekillerde çizgisi), SVG'nin dolgu rengi, yazının rengi. Ramp rengi `#RRGGBB` olur; saydamlık sembolün katmanınkidir.

### 3. Görünüme bağlı kurulum

Katman, işleyicisi görünüme bağlıysa görünüm değişince yeniden kurulur (bugünkü yardımcı çizgilerin kuralı genişler):

| İşleyici | Kurulumun girdisi | Yeniden kurulma |
|---|---|---|
| Isı haritası | kutu: görünüm ve her yanda yarısı (iki kat genişlik ve yükseklik); ölçek: görünümün piksel/metresi çeyrek oktava yuvarlanmış (`2^(round(4 · log₂ px/m) / 4)`) | görünüm kutudan çıkınca hemen, yuvarlanmış ölçek değişince (yakınlaştırma durduktan 150 ms sonra) |
| Kümeleme, Yayma (px) | yuvarlanmış ölçek | yuvarlanmış ölçek değişince (yakınlaştırma durduktan sonra) |
| Grafik (px) | yuvarlanmış ölçek | yuvarlanmış ölçek değişince (yakınlaştırma durduktan sonra) |
| Ters alan | yardımcı çizgilerin kutusu (görünüm ve her yanda üç katı) | görünüm kutudan çıkınca ya da çok yakınlaşınca (yardımcı çizgilerin kuralı) |
| Grafik (mm), Nokta yoğunluğu, Orantılı sembol | — (Ekranda sabit'te sembollerin kuralı) | — |

Isı haritası, Kümeleme, Yayma ve Ters alan katmanı **bütün olarak** kurulur (stil çekirdeğinin `whole`'u: nesneler birlikte ele alınır;
masaüstünün parçaları yok, gruplar parçaların arasından geçer); masaüstünde Nokta yoğunluğu da parçalanmaz (noktalarının sınırı
katmanındır). Isı haritasının resmi ev sahibinin anahtarıyla gelir (`heat:<katman>:<sayı>`); her kurulum yeni anahtar alır, çizilmeyen
eski resim bırakılır (masaüstünde resim kaynağının `keep_made`'i, web'de çizim hattının katman başına anahtarları). Pafta kâğıdın
ölçeğiyle kurar (96 dpi'lık piksel, `96 000 / (25,4 · ölçek)` px/m); kutusu web'de haritanınki, masaüstünde çizimin kapsamı ve her
yanda bir kenarı kadar pay; resmi olan harita resim olarak gider (rasterler gibi).

### 4. Arayüz

- **Katman stili** (iki platform): üstteki işleyici seçimi açılır listedir: on dört tür, her biri ikonu ve ne yaptığını söyleyen ikinci
  satırıyla, gruplar: (başlıksız) Basit, Tek sembol; *Değere göre* Kategorili, Aralıklı, Sürekli renk, Orantılı sembol, İki değişkenli
  renk; *Kurallar* Kurallar; *Tematik* Nokta yoğunluğu, Grafik, Isı haritası; *Noktalar* Kümeleme, Yayma; *Alanlar* Ters alan. Her türün
  formu: ifade alanları (Alanlar, Değişkenler, İşlevler), sayılar (yazılan metin tutulur, değer okunup aralıktaysa alınır), “Verilerden
  al” (en küçük ve en büyük değer; Grafik'te en büyük toplam), rampa seçimi, Ters ve rampanın şeridi, sembol yuvaları (türün geometri
  sınıflarıyla), değer listesi (renk, ifade, etiket; ekle, yukarı, aşağı, sil; en çok 12), birimler, Değeri olmayanlar. Kümeleme ve
  Yayma'nın **Tek noktalar**'ı sekiz türden biridir (Basit görünüş, Tek sembol, Kategorili, Aralıklı, Sürekli renk, Orantılı sembol, İki
  değişkenli renk, Kurallar); ayarları o türün kendi sayfasındaki taslağıdır. Değerleri olan ve olmayan nesneler sayılır. Uygula ve Tamam
  `cad.layers.renderer` ile yazar; çekirdeğin kuralları reddederse altta nedeni yazılır.
- **Lejant:** §2'deki satırlar (iki platformda `legend.json`); Orantılı sembolün üç satırı ortak ölçekte çizilir (`pxPerMm`).
- **İkonlar** (sorulmadan seçildi): `rendererSimple`, `rendererSingle`, `rendererCategorized`, `rendererGraduated`, `rendererUnclassed`,
  `rendererProportional`, `rendererBivariate`, `rendererRules`, `rendererDotDensity`, `rendererChart`, `rendererHeatmap`,
  `rendererCluster`, `rendererDisplacement`, `rendererInverted` (web'in `ui/icons.ts`'i; masaüstüne envanterle gelir).

### 5. Komut ve otomasyon

- **`cad.layers.renderer`** v1: `{ layer, renderer?, expectedRevision? }` (`renderer` nesne; yoksa ya da `null`sa işleyici kalkar, basit
  görünüş); tek adım (“Katman stili”); aynıysa bir şey yazılmaz (`changed` false). Retler sırasıyla: `invalid_renderer` (stil çekirdeğinin
  kuralları, `style::rules::renderer_problem`: bilinmeyen tür, eksik ya da aralık dışı değer, boş ya da derlenmeyen ifade, iç işleyici
  olarak Kümeleme ve Yayma; ileti hangisi olduğunu söyler), `invalid_revision`, `revision_conflict`, `layer_not_found`, `not_a_layer`,
  `service_layer` (`null`'da da). Stil katmanın görünüşüdür: kilitli katman da alır.
- Python `kentos.cad.layers.renderer`; MCP'de araç (yıkıcı: `null` kaldırır). Sunucu işleyiciyi opak taşır (KCAD gibi).

### 6. Performans

Nesne başına JSON yok; işleyiciler stil çekirdeğinin tek kurulum çağrısındadır. Sürekli renk ve boy 256 basamakla en çok 256 toplu çizim
üretir; boyu nesne başına değişen işaretler toplu çizimi bölmez. Gruplar ızgarayla doğrusal; ısı haritası hücreye kesilmiş noktaların
çekirdeğini satır satır toplar (vektörleşir); Ters alan yamukları doğrusal sayıdadır. Pastanın dilimleri merkezden yelpaze üçgenlerdir
(kulak kesmenin dışbükey halkada her kulak için bütün köşeleri tarayan `O(n²)`'si yok), yayları dönüşle. Tek nokta (ısı haritası,
kümeleme, yayma) çizim kaydı yazılıp okunmadan yerinden alınır; sayfa nesne başına katmanın rengini ve kalınlığını yeniden aramaz.
Bütçeler (release, bu makine, 1440×900 görünüm, 2 km): 10 000 alanda Sürekli renk ≤ 30 ms (kurulum), 10 000 alanda çerçeveli pasta
grafik ≤ 70 ms (çerçevesiz ≤ 60 ms; her dilimin çerçevesi kendi çizgisidir, QGIS'teki gibi), 1 000 alanda 100 000 nokta yoğunluğu
noktası ≤ 60 ms, 100 000 noktanın ısı haritası (20 px, kalite 2) ≤ 40 ms, 100 000 noktanın kümelenmesi ≤ 30 ms, 10 000 parselin ters
alanı ≤ 40 ms; web'de aynı işler ≤ 2,5 katı. Pastanın ilk bütçesi (60 ms) çerçevesizdi; çerçeve ölçülünce ayrıldı (Doğrulama).

## Uygulama

- **Stil çekirdeği** (`kentos_style_core::style`): okuyucu `model.rs` (dokuz tür, `inner`, `view_needs`, `whole`; okurken hoşgörülü),
  kurallar `rules.rs` (`renderer_problem`, `renderer_problem_text`: komutun ve pencerenin reddi), değer kuralları `thematic.rs` (rampa,
  pay, 256 basamak, boy, sınıf, ana renk ve boy yamaları; `resolve.rs`'in `Patch`'i), `dots.rs` (SplitMix64, çift-tek iç sınaması,
  halkaların FNV-1a'sı), `charts.rs` (pasta dönüşle, çubuklar), `groups.rs` (gruplar hızlı hücre özetiyle, yerleşimler), `heat.rs`
  (ızgara, damga, değerler, 1024'lük tablo), `inverted.rs` (tarama çizgisiyle yamuklar, çift-tek ve sarım); kurulum `build.rs`
  (`build_layer_in`, `ViewFrame`, nesne başına çizim ve `take` ile bütün katmanın çizimi `draw_whole`, en çok 1 000 000 nokta);
  toplu çizim `batch.rs` (`Batches`'in `pictures`'ı ve `dropped`'ı, `BatchSink::fan`). WASM `store.rs`: `viewNeeds`, `rendererProblem`,
  `buildStyled`'ın ölçeği ve resmin anahtarı.
- **Sözleşme ve komut:** `kentos_contracts::cad_layers` (`LayersRenderer`, `LayersRendered`, `LayersRendererPlan`), katalog kaydı;
  masaüstü `kentos_native_application::layers_renderer` (kod `invalid_renderer`), web `product/layersRenderer.ts`, başsız sunucu
  `dispatch.rs`; Python `kentos.cad.layers.renderer` (üretilir; `sdk.py` açıklaması olan türsüz şemayı `Any` okur); MCP'de araç (yıkıcı).
- **Masaüstü:** stilli sahne `style/scene.rs` (görünüme bağlı katmanlar `ViewBuilt` ile: çeyrek oktavlık ölçek, ısı haritasının kutusu,
  bütün katmanlar parçalanmaz; Ters alan'ın parçaları yardımcı çizgilerinki gibi yenilenir), `viewport.rs` (ölçek, yakınlaştırma
  durunca yeniden kurma, resimler `Images::put_made` ve `keep_made`, çizilmeyen noktaların uyarısı `take_dropped`), pafta `sheets.rs` ve
  `scene.rs`'in `sheet_frame`'i; yerli sayfa `kentos_native_style`: `renderer.rs` (dokuz türün tipli JSON'u, `needs_of`), `program.rs`
  (çerçeve, renk ve kalınlığın önbelleği), `thematic.rs` ve `legend.rs` (lejantın satırları, `px_per_mm`); Katman stili
  `style/layer_style/` (`mod.rs`: türler, taslaklar, komutla yazma, sayıların değerce karşılaştırılması; `thematic.rs`: dokuz form;
  `panels.rs`: gruplu, ikonlu tür listesi; `widgets.rs`'in `slots_in`'i); Lejant penceresi ve PNG'si ortak ölçekle.
- **Web:** `model/style.ts` (türler, `rendererNeeds`), `style/thematic.ts`, `style/legend.ts`, `render/styledLayer.ts`, `render/pictures.ts`,
  iki çizim hattının resimleri (`styledRenderer.ts`, arka uçların katman başına anahtarları), `viewport/ViewportController.ts` (görünüme
  bağlı yeniden kurma, 150 ms), pafta `app/sheet/mapFrames.ts` (`layerStyleAt`), Katman stili `ui/style/LayerStyleDialog.ts` ve
  `thematicPanels.ts` (açılır liste `Dropdown`), lejant ve küçük resimler, `styles/style.css`, ikonlar `ui/icons.ts`.
- **KentOS UI:** menünün iki satırlı (açıklamalı) radyo satırları ikonlarını kendi sütununda çizer; başlıklar etiketlerle hizalıdır
  (`context_menu.rs`).
- **Bağımsız başvurular:** `scripts/fixtures/renderer_cases.py` (`fixtures/renderers/v1/cases.json`: rampa 24, sürekli 21, boy 18, sınıf
  24, nokta 5, grafik 6, grup 4, yayma 6, ısı 2, ters alan 14 durum; noktalar, gruplar ve ısı değerleri bit bit, ters alanın alanları
  shapely'yle), sahne `renderer_scene.py` (`fixtures/interaction/v1/renderers.kcad`), komut durumları `renderer_command_cases.py`
  (`fixtures/commands/v1/cad.layers.renderer.json`, 45 durum).
- **Ortak dosyalar:** `fixtures/style/v1/legend.json` (dokuz türün katmanı), `fixtures/style/v1/batches.json` (dokuz tematik durum: sayfanın
  çağrısı ve toplu çizimler iki platformda aynı; durumun kutusu `clip`).
- **Resimler:** masaüstü `renderer_scenes.rs` (`arac-isleyici-*`, 21 sahne), web `shots.mjs renderers` (`isleyici-*`, 21 sahne);
  `e2e:layout`'ta Katman stili'nin listesi ve dört tematik formu.
- **Süre ölçümleri:** masaüstü `crates/native/style/tests/all/renderer_timing.rs`, web `apps/web/scripts/perf/renderers.test.ts`.

## Doğrulama

10 Ekim 2026, geliştirme makinesinde (11. nesil Intel Core i5-11300H, 8 iş parçacığı, Linux; derleme 4 işle):

- **Bağımsız başvurular** (KentOS kodu olmadan, `--check` ile hepsi geçer): `renderer_cases.py` (124 durum; nokta yoğunluğunun
  noktaları, gruplar ve ısı haritasının değerleri bit bit, grafikler 1e−9'da, ters alanın alanları shapely'yle), `renderer_scene.py`,
  `renderer_command_cases.py` (`cad.layers.renderer` 45 durum).
- **Platformlar:** komut durumları web'de, masaüstünde, başsız sunucuda ve Python'da; `legend.json` (dokuz türün katmanlarıyla) ve
  `batches.json` (dokuz tematik durum: sayfanın çağrısı ve toplu çizimler) iki platformda aynı; parçalara bölünen katmanın bütünle aynı
  çizdiği sınamada parçalanmayan dört tür atlanır. MCP'de araç, yıkıcı (`cargo test -p kentos-mcp`, 11). Resimler iki platformda
  1440×900 ve 1100×650'de, iki temada: masaüstü `arac-isleyici-*`, web `isleyici-*` (21'er sahne: dokuz türün yakın görünümü, Ters alan,
  Katman stili'nin listesi ve dokuz formu, lejant).
- **Bulunan ve düzeltilen:** Nokta yoğunluğunu çekirdeğin `whole`'una koymak çizimini kesiyordu (bütün katmanın yolu onu bilmez):
  parçalanmama kuralı yalnız masaüstünün sahnesindedir (`ViewNeeds.whole`). Masaüstünün penceresi açılır açılmaz “uygulanmadı” diyordu
  (dosyanın `4000`'ı taslağın `4000.0`'ı): renderer'lar değerce karşılaştırılır. Kuralın iletisi sınırsız üst değeri `f64::MAX` olarak
  yazıyordu: yalnız alt sınır söylenir. KentOS UI'ın açıklamalı radyo satırları ikonlarını çizmiyordu. Katalogda “çeyrek dereceden”
  çekirdek “dördüncü dereceden (quartic)” oldu. Python üreticisi açıklaması olan türsüz şemayı (`renderer`) okuyamıyordu. Duman testinin
  Katman stili adımı açılır listeye göre yenilendi.
- **Takım:** `pnpm typecheck`; `pnpm test` (350 dosya, 4836 test geçti, 25 atlandı); `pnpm rust:test` (3038 geçti, 38 yok sayıldı;
  clippy ve bağımlılık yönü temiz); `pnpm rust:test:desktop` (1277 geçti, 194 yok sayıldı; clippy temiz); `pnpm py:test` (61);
  `pnpm e2e:interaction` (142 iz × 3 varyant) ve masaüstünde bütün izler bütün varyantlarda; `pnpm e2e` (209 denetim); `pnpm build`
  (Katman stili penceresi 49,2 kB, tematik kurallar ayrı parça 7,4 kB); `pnpm inventory:check`. `e2e:layout`'ta Katman stili ve listesi,
  Grafik, Isı haritası, İki değişkenli renk ve Yayma formları (24 görünüm, sorunsuz). `pnpm e2e:cloud` çalıştırılmadı: sunucu koduna
  dokunulmadı, işleyici sunucuda opak taşınır. Gerçek GPU sınaması (`KENTOS_GPU_TESTS`) çalıştırılmadı.
- **Süreler** (release; masaüstü `renderer_timing`'in en iyi 10 koşusu, web `scripts/perf/renderers.test.ts`'in gönderilen WASM'la p50'si;
  ölçerken masaüstü uygulaması açıktı, yük 1–3,7):

  | İş | Masaüstü | Bütçe | Web (p50) | Web bütçesi |
  |---|---|---|---|---|
  | Sürekli renk, 10 000 alan | 14,8 ms | 30 | 20,0 ms | 75 |
  | Pasta grafik, 10 000 alan, çerçeveli | 67,4 ms | 70 | 75,8 ms | 175 |
  | Nokta yoğunluğu, 1 000 alanda 100 000 nokta | 5,1 ms | 60 | 7,6 ms | 150 |
  | Isı haritası, 100 000 nokta (20 px, kalite 2) | 37,9 ms | 40 | 64,4 ms | 100 |
  | Kümeleme, 100 000 nokta (40 px) | 28,4 ms | 30 | 37,6 ms | 75 |
  | Ters alan, 10 000 parsel | 7,4 ms | 40 | 10,6 ms | 100 |

  İlk ölçümde pasta 132 ms, ısı haritası 44 ms, kümeleme 49 ms'ydi. Pastanın dilimleri yelpaze üçgenleri oldu (kulak kesmenin dışbükey
  halkada her kulak için bütün köşeleri taraması kalktı: 132 → 78 ms), yaylar dönüşle (→ 66 ms; çerçevesiz 50 ms). Gruplama SipHash'li
  haritadan sabit, hızlı hücre özetine geçti ve grubun merkezi saklanır (29 → 17 ms); tek nokta çizim kaydı yazılıp okunmadan alınır;
  sayfa nesne başına renk ve kalınlık aramaz. Isı haritasının hesabı kendisi 27 ms'dir (değerler 23, renk 2,5, en büyük 1,8).
