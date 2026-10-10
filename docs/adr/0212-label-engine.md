# ADR 0212: Etiket motoru

- **Durum:** kabul edildi (2026-10-10). Kapsamı ben belirledim; sahibin sözü: “bu konu çok önemli, QGIS falan nasıl çözüyor incele ve
  ona göre yap”. İki platform (masaüstü ve web), bulut, Python ve MCP; ikonlar sorulmadan seçilir; ilke “Performance First”. Madde
  tek parçada biter.
- **Bağlam belgesi:** TODOS.md `GIS-16`; ADR 0055 (çizimin yazıları), ADR 0175 (Etiketleri yazıya çevir), ADR 0196 (eğri boyunca yazı),
  ADR 0205 (açıklamaların yükseklikleri), ADR 0100 (ifade dili), ADR 0211 (katman süzgeci), ADR 0164 (pafta).
- **Araştırma:** QGIS'in PAL'ı (`src/core/pal`: aday üretimi `feature.cpp`, maliyetler `costcalculator.cpp`, FALP başlangıcı ve zincir
  araması `problem.cpp`, öncelik ve engeller `pal.cpp`; kullanıcı belgesi “Label settings”, Etiket araç çubuğu, kurallı etiketleme),
  ArcGIS Pro'nun Maplex'i (`CIMMaplexLabelPlacementProperties`: Land parcel, Boundary, Contour (Uphill, Ladder), yığma, kısaltma
  sözlüğü, yazı küçültme, sığdırma stratejilerinin sırası, Remove duplicate labels), MapLibre GL'nin çakışma dizini (`collision_index.ts`:
  25 piksellik hücreler, görünümün 100 piksellik payı, sıralı açgözlü yerleştirme). Netcad'in etiket belgeleri açıkta yok.

## Bağlam

Bugün katmanın tek bir etiket stili var (`LabelStyle`: dört yerleşim, boy, büyüme, şablon, ölçek aralığı, en küçük nesne). Geometri
deposu her nesneye tek bir yer verir (merkez, köşe, yanı, iki köşe arası); çizimin üstündeki katman (web, masaüstü), paftanın haritası
(web, masaüstü) ve Etiketleri yazıya çevir etiketleri kendi 8 piksellik hücreleriyle seyreltir: önce gelen kalır, ona değen düşer.
Etiketin ikinci bir yeri yoktur (sağ üstü dolu nokta adı sol üstü boşken kaybolur), öncelik ve engel yoktur, çizgiyi izleyen ve parselin
içine sığdırılan yerleşim yoktur, etiket elle taşınamaz; etiket ayarlarını değiştiren bir arayüz de yoktur.

**QGIS** her nesne için maliyetli aday konumlar üretir (noktada kartografik sekiz yer ya da çevresinde ışınlar; çizgide paralel, kıvrık,
yatay; alanda içeride ızgara, eğik, çevre boyunca, dışarıda), engellerin (nokta, çizgi, alanın içi ya da sınırı; ağırlıklı) maliyetini
ekler, öncelik engeli aşamıyorsa adayı düşürür; adaylar arasındaki çakışmaları kurup FALP'la başlar, sınırlı “ejection chain”lerle
iyileştirir. Kurallı etiketleme her kurala kendi ayarlarını verir; etiket araç çubuğu etiketi taşır, döndürür, sabitler, gizler; sabit
etiketlerin yeri yardımcı depodadır. **Maplex** parsel kipinde etiketi önce yatay, sonra eğik içeri sığdırır, sonra yığar, kısaltır,
küçültür; sınır kipinde alanın kenarı boyunca içeride yazar; eş yükselti kipinde etiketin üstü yokuş yukarı bakar; yinelenenleri ayıklar.
**MapLibre** her karede sıralı açgözlü yerleştirmeyle, ızgaralı bir çakışma diziniyle binlerce etiketi milisaniyelerde yerleştirir;
görünümün dışındaki pay kenardaki değişimi ekran dışına iter.

## Karar

### 1. Kapsam

- **Tek motor** geometri çekirdeğinde (`kentos_geometry_core::labels`): aday üretimi, maliyet, engeller, yerleştirme. Çizimin üstü (iki
  platform, Büyüteç dahil), paftanın haritası (ekranda ve PDF'te) ve Etiketleri yazıya çevir aynı motordan geçer; seyrelten dört ayrı
  hücre kodu kalkar.
- **Kurallı etiket sınıfları:** katmanın etiketlemesi Tek etiket (bugünkü stil), Kurallı (sıralı sınıflar: ad, koşul, kendi stili),
  Yalnız engel (etiketsiz, nesneleri engel) ya da Yok. Bir nesneyi koşulu tutan her sınıf etiketler.
- **Metin:** şablon (`{label}`) ya da ifade (İfadeyle seç'in dili; `$etiket` nesnenin etiketi).
- **Yerleşim** nesnenin türüne göre: nokta (çevresinde, üstünde), çizgi (paralel, kıvrık, yatay, eş yükselti), alan (yatay, eğik, çevre
  boyunca, sınır, parsel, köşe); konum, uzaklık, yineleme, harfler arası en büyük açı, bağlı çizgileri birleştirme, yalnız içine sığarsa,
  sığmazsa dışarıda.
- **Çakışma:** adaylardan çakışmayan; öncelik (0–10); çakışma kuralı (çakışmasın, gerekirse, her zaman); engel katmanları (ağırlık ve
  tür); çizimin yazıları ve nokta sembolleri de engeldir; yinelenen metinler ayıklanır.
- **Sığdırma:** yığma (satırlara bölme), kısaltma sözlüğü, küçültme, dışarıda çağrı çizgisiyle.
- **Biçim:** renk, kalınlık, eğiklik, hale, zemin (dikdörtgen, yuvarlak köşeli, elips), gölge, çağrı çizgisi (düz, dik açılı).
- **Elle:** Etiketi taşı, Etiketi döndür, Etiketi sabitle (ve çöz), Etiketi gizle (ve göster); Sabit etiketleri vurgula, Yerleşmeyen
  etiketleri göster.
- **Otomasyon:** `cad.layers.labels` ve `cad.labels.pin`; Python ve MCP; sunucu yeni alanları saklar ve denetler.
- **Kapsam dışı:** nesne başına veriyle tanımlı stil (QGIS'in data-defined özellikleri; sabitleme dışında), Maplex'in anahtar numaralama
  (key numbering), eş yükselti merdiveni (ladder), etiketin başka katmanları maskelemesi (`GIS-30`), alanın içinde kıvrık yerleşim,
  çok parçalı nesnenin her parçasına etiket, cadde adresi ve dere kipleri, harf aralığını yayma, etiketin çizim sırası (z), diyagramlar.
  Çizimin yazı nesneleri, ölçüleri ve kılavuzları etiket değildir; yerleri değişmez (engeldirler).

### 2. Veri modeli

`kentos_contracts` (`layer.rs`, yeni `labels.rs`):

- **`LabelStyle`'ın yeni alanları** (hepsi isteğe bağlı; yokluğu bugünkü davranış):
  - metin: `text` (ifade; varsa şablonun yerine), `color` (yazının rengi: onaltılık ya da tema adı; varsa `ink`'in yerine), `italic`,
    `align` (çok satırda `left`, `center`, `right`);
  - yerleşim: `point` (`around`, `center`), `line` (`parallel`, `curved`, `horizontal`, `contour`), `area` (`horizontal`, `free`,
    `perimeter`, `boundary`, `parcel`, `corner`), `position` (`on`, `above`, `below`, `sides`; alanın çevresinde `above` içeri,
    `below` dışarı demektir), `distance` (px), `repeat` (px), `maxAngle` (derece), `curved` (çevre ve sınır kıvrık), `mergeLines`,
    `inside`, `outside`;
  - biçim: `halo` (`width` px, `color`), `background` (`shape`: `rect`, `round`, `ellipse`; `fill`, `stroke`, `padding` px), `shadow`
    (`dx`, `dy` px, `color`, `opacity`), `callout` (`kind`: `straight`, `manhattan`; `color`, `width` px, `minLength` px);
  - sığdırma: `stack` (`mode`: `ifNeeded`, `always`; `chars` satırın en çok harfi; `at` bölme karakterleri, yoksa boşluk), `abbreviate`
    (`always`; `words`: `{ word, short }` dizisi), `shrink` (en küçük boy çarpanı, 0,5–1);
  - öncelik: `priority` (0–10, yoksa 5), `overlap` (`never`, `ifNeeded`, `always`), `duplicates` (px).
- **`LayerStyle.labels`** (`LayerLabels`): `mode` (`single`: stilin `label`'ı ya da türün varsayılanı; `rules`; `off`), `classes`
  (`LabelClass`: `name`, isteğe bağlı `when` ifadesi, `style`; adlar katmanda tek; yalnız `rules`'ta), `obstacle` (`LabelObstacle`:
  `weight` 1–10, `kind` `interior` ya da `boundary`). `labels` yoksa bugünkü gibidir (tek etiket). `off` katmanda engel kalabilir (QGIS'in Blocking'i).
- **`EntityBase.labelPins`** (`LabelPin` dizisi): `class` (kuralın adı; yoksa nesnenin ilk etiketi), `at` (etiketin ortasının nesnenin
  çapasından uzaklığı, metre; nesne taşınınca, kopyalanınca etiket onunla gider), `rotation` (derece, saat yönünün tersine), `hidden`.
  Çapa bugünkü etiket noktasıdır (`entity_anchor`: nokta, alanın ağırlık merkezi, çoklu çizginin orta köşesi …).
- **Kurallar** (`labels::style_problem`, `layer_labels_problem`, `pins_problem`): boy 1–200 px, kalınlık 100–900, uzaklık 0–500, yineleme
  20–100 000, açı 5–90, küçültme 0,5–1, öncelik 0–10, yinelenen uzaklığı 1–10 000, hale 0–10, pay 0–50, gölge kayması ±50, opaklık 0–1,
  yığma 2–500 harf, kısaltma 1–500 sözcük (sözcük 1–100 harf), kurallar 1–64 (adı 1–100 harf, tek), ifadeler 1–10 000 harf, engel
  ağırlığı 1–10; iğnede sonlu sayılar, sınıfın adı 1–100 harf. İfadelerin derlenmesi dosyanın kuralı değildir: komutlar derler.
- **`.kcad` şema 36** (`FORMATS_VERSION` 46): etiket stilinin yeni anahtarları, katman stilinin `labels`'ı, nesnenin `labelPins`'i;
  yalnız biri varken şema 36 yazılır. Bağımsız Python okuyucusu ve yazıcısı, örnek `labels.kcad`, bozuk dosyalar.
- **Sunucu:** `kentos.feature`'a `label_pins` (jsonb) sütunu (göç 0015); nesnenin alanı gibi gider ve gelir. Katman ağacının kuralları
  `project.changes`'te denetlenir.

### 3. Motor

Bir çağrı bir **pencereyi** (çizimin kutusu, metre) bir **ölçekte** (piksel/metre) yerleştirir. Ev sahipleri pencereyi görünümden
geniş verir (web'in etiket resminin payı, masaüstünün aynısı): pencerenin içinde yerleşim kararlıdır, kenardaki değişim ekran dışında
kalır (MapLibre'nin payı). Hesap sayfa pikselindedir: pencerenin sol alt köşesi başlangıç, y yukarı; açılar doğudan saat yönünün
tersine. Sonuçlar dünya koordinatlarıyla döner.

#### 3.1 Girdi

- **Katmanların etiketlemesi** (ev sahibi `labels`'ın JSON'unu verir; masaüstü de aynı JSON'la, ayar seyrek değişir): sınıflar, engel,
  katmanın çizim sırası (üstteki önce) ve nokta sembolünün boyu.
- **Metinler** (ev sahibi hesaplar, depo tutar): nesne başına, onu etiketleyen her sınıf için metin ve eş yükselti kipinde kot (çoklu
  çizginin ilk kotu). Şablon `fill_template` ile, ifade ve koşul ifade motoruyla; `$sıra` ve `$ölçek` reddedilir. Katmanın etiketlemesi
  değişince bütün katman, nesneler değişince değişenler (katman süzgecinin yolu, ADR 0211 §3). Yeni nesnenin metni yoksa etiketi yoktur.
- **İğneler** nesneyle gelir (depo nesnenin her konuşunda yeniler).

#### 3.2 Yazının kutusu

- Boy `f = min(maxSize ?? size, size + grow · ölçek)` (bugünkü kural), küçültmede `f · k`.
- Harfin genişliği çizimin yazı tipinin ölçülmüş ilerlemesi (`text::metrics`; kalınlık 600 ve üstü kalın tablo) çarpı `f / 1000`;
  satırın genişliği toplamı. Satırın kutusu `f` yüksekliğindedir, satır aralığı `1,2 f`; blok `W` (en geniş satır) × `H = f + (n − 1) · 1,2 f`.
  Ev sahibi her satırı ortasından çizer (sola ya da sağa hizada `±(W − w)/2` kayar).
- **Çakışma kutusu:** blok her yanda `pad = max(hale, zeminin payı) + 1` piksel büyür; hale yoksa 1,5 piksellik varsayılan haledir.
  Kutu yönlü dikdörtgendir (merkez, eksen, iki yarı uzunluk); kıvrık etiket harf başına bir kutudur.
- İki kutu ayırma ekseni denemesiyle (dört eksen) çakışır; değen kutular çakışmaz.

#### 3.3 Adaylar

Her etiket **birimi** (nesnenin bir sınıfının etiketi; yinelenen çizgide parça başına bir birim) sıralı adaylar üretir; maliyet küçük olan
öncedir. `P` nesnenin noktası, `D = r + d` (`r` nokta sembolünün yarıçapı, `d` uzaklık, yoksa 2 piksel), `k = D / √2`.

- **Nokta, çevresinde** (QGIS'in kartografik sırası): bloğun merkezi sağ üst `P + (k + W/2, k + H/2)`, sol üst `(−k − W/2, k + H/2)`,
  sağ alt `(k + W/2, −k − H/2)`, sol alt `(−k − W/2, −k − H/2)`, sağ `(D + W/2, 0)`, sol `(−D − W/2, 0)`, üst `(W/4, D + H/2)`, alt
  `(−W/4, −D − H/2)`; maliyet `0,0001 + 0,001 · sıra`. **Üstünde:** merkez `P`. Eski `beside` çevresinde, `center` üstündedir.
- **Köşe** (eski `corner`, paftanın etiketi): satırın sol ortası kutunun sol üst köşesinden 8 piksel sağda, 14 piksel aşağıda; tek aday.
- **Çizgi:** yolun sayfadaki uzunluğu `L`. Yineleme `R` varsa yol `[iR, (i + 1)R]` parçalarına bölünür, her parça bir birimdir (eş
  yükselti kipinde `R` yoksa 400 piksel); yoksa yol pencereye kırpılır, en uzun parçası tek birimdir. Birimin aralığında adayların
  ortası ortadan başlayıp iki yana `adım = max(W/4, (Lᵤ − W)/20, 2)` ile, en çok 21 tane.
  - **Paralel:** ortanın iki yanında `W/2` uzaklıktaki yol noktalarının kirişi; düzlük `q = kiriş / W ≥ 0,8` ve aradaki köşelerin kirişe
    uzaklığı `e ≤ f/2` olmalı. Açı okunur yapılır (`(−90°, 90°]`). Konum: üstünde (kirişin ortası), üstte ve altta (`± (d + H/2 + e)`
    yazının yukarısına). Maliyet `0,0001 + 0,001 · |t − Lᵤ/2| / Lᵤ + 0,01 · (1 − q) + 0,001 · e / f` (+ altta 0,0005).
  - **Kıvrık:** harfler yol boyunca (harfin ortası yolda, açısı yolun oradaki doğrultusu); harflerin ortalama doğrultusu sola bakıyorsa
    yol ters yürünür. Komşu iki harfin açı farkı `maxAngle`'ı (yoksa 25°) aşarsa aday yoktur. Maliyete açı farklarının ortalaması
    eklenir (`0,01 · ortalama / π`).
  - **Yatay:** blok yol noktasında yatay.
  - **Eş yükselti:** kıvrık, yolun üstünde; yazının yukarısı **yokuş yukarı** bakar: adayın ortasından yolun iki yanına `6 f` piksellik
    yoklama, aynı katmanın kotu olan öbür eğrilerini keser; daha yüksek kotlu yan yukarıdır (bir yanda eğri bulunursa kendi kotuyla
    karşılaştırılır; bulunmazsa ya da eşitse okunur kural). Zemin yoksa eğrinin üstünde çizgiyi kesen zemin rengi bir maske (pay 1).
- **Alan:** ölçüsü en büyük parça; halkalar pencereye kırpılır; erişilmezlik kutbu `p*` (Mapbox'ın polylabel'ı, 1 piksel duyarlıkla).
  - **Yatay:** merkezler `p* + (i · max(W/4, 2), j · max(H/2, 2))`; `i` ve `j` her yönde alanın kutusunun kenarına varacak kadar,
    en az 3, en çok 8 adım (etiket kutuya hiç sığmıyorsa, eğik ve parsel kipi dışında yalnız kutup); sıra kutba yakından uzağa, eşit
    uzaklıkta halka, sütun, satır; maliyet `0,0001 + 0,0001 · (i² + j²)`. İçeride
    sayılmak için bloğun dört köşesi alanın içinde olmalı ve hiçbir kenar bloğu kesmemeli; `inside` ise içeride olmayan aday yoktur,
    değilse maliyetine 0,5 eklenir (eski `center` budur: ağırlık merkezi yerine kutup).
  - **Eğik:** yatay adaylar ve dışbükey zarfın en küçük alanlı dikdörtgeninin uzun kenarı doğrultusunda döndürülmüş aynı ızgara
    (maliyetine 0,0005 eklenir): sığarsa yatay, sığmazsa eğik.
  - **Çevre boyunca:** dış halka saat yönünün tersine yürünen kapalı yol; çizginin paralel (ya da `curved`) adayları, konum üstünde
    (varsayılan), içeride (`above`) ya da dışarıda (`below`).
  - **Sınır** (Maplex Boundary): çevre boyunca, içeride, `R` yoksa 300 piksellik yinelemeyle.
  - **Parsel** (Maplex Land parcel): yalnız içeride (`inside` her zaman); yatay, sonra eğik; sığmazsa sığdırma sırası (§3.4).
  - **Dışarıda** (`outside`; her alan kipinde son çare): kutusunun çevresinde sağ, sol, üst, alt, sağ üst, sol üst, sağ alt, sol alt
    (`D = d`), çağrı çizgisi `p*`'ye; maliyet `1 + 0,001 · sıra`.
- **Tür:** nokta ve blok noktadır; çizgi, çoklu çizgi, yay, eğri ve açık elips çizgidir; alan, daire, kapalı elips, tarama, resim, raster
  ve nokta bulutu alandır. Yazı, ölçü, kılavuz ve tablo etiket almaz (bugünkü gibi).

#### 3.4 Sığdırma sırası

Birimin **biçimleri** sırayla denenir; birinin bir adayı yerleşince sonrakilere bakılmaz (Maplex'in strateji sırası):

1. metin olduğu gibi (`\n` satır böler; `stack.mode = always` ise yığılmış);
2. `stack.mode = ifNeeded`: yığılmış (bölme karakterlerinden açgözlü, satır en çok `chars` harf; daha uzun tek sözcük bölünmez; boşluk
   satır sonunda düşer, öbür bölme karakterleri satırda kalır);
3. kısaltma sözlüğü varsa (`always` değilse): kısaltılmış (sözcük sözcük, tam eşleşme), yığma da varsa kısaltılmış ve yığılmış;
4. `shrink < 1`: en derli toplu metin `0,9 f`, `0,8 f` … `shrink · f`'ye kadar;
5. alanda `outside`: ilk biçim dışarıda.

Değişmeyen biçim (yığmanın tek satırı, sözlükte olmayan metin) atlanır.

#### 3.5 Engeller

- **Kesin** (aday onu kesemez): yerleşmiş ve sabit etiketler; pencerenin yazı nesneleri, kılavuz notları ve ölçü değerleri.
- **Nokta sembolleri:** görünen her nokta nesnesi (`r` en az 2 piksel); kesen adayın maliyetine sembol başına 0,05 eklenir (kendi
  noktası hariç).
- **Engel katmanları** (`obstacle`): görünen nesneleri (etiketin kendi nesnesi hariç); nokta sembolüyle, çizgi parçalarıyla, alan
  içiyle (`interior`: blok alanla örtüşür) ya da sınırıyla (`boundary`: blok bir kenarı keser). Etiketin önceliği ağırlıktan küçükse
  kesen aday yoktur; değilse nesne başına `0,05 · ağırlık` maliyet (QGIS'in sürüm 2 kuralı).
- **Yinelenenler** (`duplicates`): aynı metinli yerleşmiş bir etiketin ortası `duplicates` pikselden yakın olan aday yoktur (katmanlar
  arası, büyük küçük harf duyarlı).

#### 3.6 Sıra ve çözüm

1. **Sabitler** (§3.7) önce yerleşir.
2. Birimler sıralanır: öncelik (büyük önce), katmanın çizim sırası (üstte çizilen önce), sınıfın sırası, nesnenin belgedeki sırası,
   parçanın sırası. Sıra görünüme değil nesnelere bağlıdır: kaydırınca etiketler yerinde kalır.
3. **Açgözlü:** her birim biçimlerini sırayla, her biçimin geçerli adaylarını maliyet sırasıyla dener; çakışmayan ilk aday yerleşir.
   `overlap = always` birim en ucuz adayına yerleşir.
4. **İyileştirme** (QGIS'in zincirinin bir adımlığı): yerleşemeyen her birim (sırasıyla) ilk biçiminin adaylarını dener; adayı tek bir
   yerleşmiş etiketle çakışıyorsa (sabit değil, önceliği küçük ya da eşit) o etiket kendi biçiminin başka bir adayına taşınabiliyorsa
   ikisi de yerleşir. Birim başına en çok 32 aday denenir.
5. `overlap = ifNeeded` olup yerleşemeyen birim ilk biçiminin en ucuz adayına yerleşir (çakışarak).
6. Kalanlar **yerleşmeyen**dir; Yerleşmeyen etiketleri göster açıkken en ucuz adaylarında kırmızı çizilir.

#### 3.7 Sabit etiketler

İğnesi olan birim tek adaydır: çapa + `at`, `rotation` (yoksa 0), ilk biçim; her zaman çizilir, başka bir şeyle çakışsa da; sonrakiler
ondan kaçar. Sabitlenen nesnenin o sınıfta yinelenmiş öbür parçaları çizilmez. `hidden` olan birim çizilmez (Etiketi gizle aracı açıkken
soluk çizilir ki gösterilebilsin).

#### 3.8 Çıktı

Kayıtlar (`LABEL_STRIDE` sayı, ADR 0055'in kayıtları gibi; nesne, tür, yer …): etiketin kutusu (orta, açı, `W`, `H`, sınıf, durum:
sabit, yerleşmeyen, gizli, çakışarak, kıvrık, dışarıda), satırları (orta, açı, boy, metnin sırası, sınıf), kıvrık harfleri (orta, açı,
boy, metin, harfin sırası) ve çağrı çizgileri. Satırların ve kıvrık etiketlerin metinleri ayrı bir listededir (web'e tek bir metin
olarak). Ev sahibi biçimi (renk, kalınlık, hale, zemin, gölge, çağrı çizgisinin rengi) sınıftan okur. Ana görünümün son yerleşimi
depoda kalır: araçlar tıklanan etiketi ondan bulur (`label_at`).

### 4. Arayüz

- **Etiketler penceresi** (CBS'de Harita › Etiket, iki türde Katmanlar'ın sağ tıkı, `layer.labels`): katman, Etiketleme (Yok, Tek
  etiket, Kurallı, Yalnız engel); Kurallı'da sınıf listesi (ekle, sil, yukarı, aşağı, kopyala; ad ve koşul); sekmeler Metin, Yerleşim,
  Biçim, Sığdırma, Öncelik; altta Engel. İfadelerin yanında İfade oluşturucu (ε). Uygula `cad.layers.labels` ile tek adımda yazar,
  pencere açık kalır (çizim hemen yeni etiketlerle görünür); Kaydet yazar ve kapatır; Vazgeç kapatır, yazılan adım Ctrl+Z ile
  geri alınır.
- **Araçlar** (CBS'de Harita › Etiket, komut aramada her iki türde): Etiketi taşı (etikete tıkla, yerine tıkla), Etiketi döndür (etikete
  tıkla, açıyı imleçle ya da yazarak ver; Shift 15°'lik adımlar), Etiketi sabitle (tıklanan etiket olduğu yerde sabitlenir; Çöz (Ç)
  seçeneği; pencereyle çok etiket), Etiketi gizle (tıklanan gizlenir; Göster (G) seçeneğiyle soluk çizilen gizliler gösterilir). Hepsi
  `cad.labels.pin` ile tek adımda yazar.
- **Görünüş:** Sabit etiketleri vurgula ve Yerleşmeyen etiketleri göster kullanıcının tercihleridir (`graphics.pinnedLabels`,
  `graphics.unplacedLabels`); çizim değişmez.
- **Pafta:** haritanın etiketleri motorun kâğıttaki ölçekteki yerleşimidir; PDF'e satırlar, harfler, zeminler ve çağrı çizgileri vektör
  olarak gider.
- **Etiketleri yazıya çevir:** motorun seçilen ölçekteki yerleşimi yazı olur: tek satır yazı, çok satır çok satırlı yazı, kıvrık eğri
  boyunca yazı; çağrı çizgileri çizgi; sabitler yerlerinde. Bağlı yazının yeri motorun o nesne için tek başına seçtiği adaydır.

### 5. Komutlar ve otomasyon

- **`cad.layers.labels`** v1: `{ layer, label?, labels? }` (yoksa değişmez, `null` kaldırır); katmanın etiketlemesi tek adımda
  (“Etiketler”). Ret: bilinmeyen katman, grup, servis katmanı, kural ihlali, derlenmeyen ifade, `$sıra`, `$ölçek`. Etiketleme
  katmanın görünüşüdür: kilitli katman da alır (katman süzgeci gibi, ADR 0211).
- **`cad.labels.pin`** v1: `{ pins: [{ uid, class?, pin }] }` (`pin` `null`: iğne kalkar); tek adım (“Etiket”). Ret: bilinmeyen nesne,
  kilitli katman, katmanda olmayan sınıf adı, kural ihlali.
- Python `kentos.cad.layers.labels`, `kentos.cad.labels.pin`; MCP'de araçlar. Sunucu ağacın ve nesnelerin kuralını denetler.

### 6. Performans

Yerleşim pencere başına, ağır işler sayfa pikselinde ve ızgarada; nesne başına JSON yok; metinler ayar ya da nesne değişince bir kez
hesaplanır; görünüm kayarken resim yeniden yerleştirilmez (iki platform da etiket resmini payıyla tutar). Bir birimin adayları maliyet
sırasıyla ve ancak gerektiğinde üretilir (çoğu etiket ilk adayına yerleşir; Uygulama). Bütçeler (release, bu makine): 2 000 nokta adı
≤ 5 ms (40 px arayla), 10 000 parsel ≤ 30 ms, 500 eş yükselti eğrisi (25 tepe, 400 px'te bir) ≤ 15 ms, 100 000 parsellik genel bakış
≤ 10 ms; web'de aynı işler ≤ 2,5 katı. 100 000 nesnenin ifadeli metinleri ≤ 80 ms. Zorlama sahnesi olarak 2 000 nokta 20 px arayla
(adların yarısı sığmaz, iyileştirme her birini dener) ≤ 8 ms.

## Uygulama

- **Sözleşme:** `kentos_contracts::labels` (etiket stilinin motor alanları `layer.rs`'te; `LabelClass`, `LayerLabels`, `LabelObstacle`,
  `LabelPin` ve kipler; `style_problem`, `layer_labels_problem`, `pins_problem`, `labels_problem`, `has_engine_fields`), `EntityBase`'in
  `labelPins`'i; `cad_labels` (`LayersLabels`, `LayersLabelled`, `LayersLabelsPlan`, `LabelsPin`, `LabelPinChange`, `LabelsPinned`,
  `LabelsPinPlan`; yok, `null` ve değer `nullable` ile ayrı) ve katalog kayıtları; `FORMATS_VERSION` 46. Web'in kuralları
  `model/labelRules.ts`.
- **KCAD:** şema 36 (`encode/labels.rs`, `decode/labels.rs`); bağımsız Python okuyucusu ve yazıcısı, örnek `labels.kcad` ve
  `labels.json`, 31 bozuk dosya (`broken/label-*.kcad`, `broken/labels-*.kcad`); `docs/specs/kcad-v2.md` §6.1, §6.5, §6.6.
- **Motor:** `kentos_geometry_core::labels` (`engine`, `geom`, `style`, `text`): biçimler (§3.4), türe göre adaylar, engeller, iğneler,
  açgözlü yerleşim, bir adımlık iyileştirme, çakışmaya izin; 32 piksellik ızgarada yönlü kutular, kutuların sınırlarıyla ön eleme.
  Bir birimin adayları **gruplar** hâlinde planlanır (alt sınır ve üretim sırası), gerektikçe üretilir ve yığında `(maliyet, kendi
  maliyeti, sıra)` ile tutulur; hesaplanmamış hiçbir aday önüne geçemeyecekse çıkar: sıra bütün listeyi sıralamakla bit bit aynıdır.
  Alanın ızgarası bir kez sıralanmış tablodan okunur, döndürülmüş hücreler ve uzun eksen sıraları gelince hesaplanır. Erişilmezlik
  kutbu polylabel'dır; en iyiyi geçemeyecek hücre yığına girmez. Depo `store/placing.rs` (`place_labels`, `labels_in`, `label_at`,
  `label_anchor`, `set_label_layers_json`, `set_object_labels`, `set_label_pins_json`), katman başına en küçük nesne eşiğiyle (genel
  bakış) ve boş süzgeç ve zaman denetimlerinden erken dönüşle; ana görünümün son yerleşimi araçlar için saklanır. Kayıtlar
  `LABEL_PLACED` … `LABEL_PLACED_CALLOUT`, durumlar `PINNED`, `UNPLACED`, `HIDDEN`, `OVERLAPPING`, `CURVED`, `OUTSIDE_AREA`, `MASKED`.
  WASM'da `placeLabels`, `labelsIn`, `labelAt`, `labelAnchor`, `labelTexts`.
- **Metinler:** `kentos_native_application::label_texts` (`compile_label_expression`, `layer_texts`, `object_texts`) ve web
  `model/labelTexts.ts`; depoyu masaüstünde `Spatial`, web'de `PickIndex` besler (katmanın etiketlemesi değişince bütün katman).
- **Bağımsız başvuru:** `scripts/fixtures/label_engine_cases.py` (ADR'den, KentOS kodu olmadan; `fixtures/labels/v1/cases.json`, 43
  durum: kartografik sıra, yinelenen çizgi, kıvrık ve eş yükselti, parsel ve sığdırma, engeller, öncelik, yinelenenler, iğneler,
  çağrı çizgisi, tıklama), sahne `label_engine_scene.py` (`fixtures/interaction/v1/label-engine.kcad`), komut durumları
  `label_command_cases.py` (`cad.layers.labels.json`, `cad.labels.pin.json`).
- **Komutlar:** `cad.layers.labels` ve `cad.labels.pin` masaüstünde `layers_labels.rs`, `labels_pin.rs`, web'de
  `product/layersLabels.ts`, `product/labelsPin.ts`, başsız sunucuda `dispatch.rs`; Python `kentos.cad.layers.labels`,
  `kentos.cad.labels.pin` (üretilen); MCP'de araç (ikisi de yıkıcı: `null` kaldırır).
- **Sunucu:** `check_tree` `labels_problem`'ı denetler; nesnenin iğneleri `kentos.feature.label_pins` sütunundadır (göç 0015; `cad.rs`'in
  taban anahtarı, geometrisi kaynak olan basit türlerde de korunur; kurallar ve blok nesnesinin iğnesizliği `validate`'te).
  Göç 0015 tür denetimini sözleşmenin bütün türlerine genişletir: kılavuz, tablo, resim, raster ve nokta bulutu veritabanı projesine
  yazılamıyordu. `duplicate_project` iğneleri de kopyalar.
- **Arayüz:** web `app/labelCommands.ts` (`layer.labels`, `view.pinnedLabels`, `view.unplacedLabels`), `ui/labels/LabelsDialog.ts`
  ve `labelForm.ts` (ilk açılışta yüklenir), `styles/labels.css`, `tools/labelTools.ts` (dört araç), Katmanlar'ın sağ tıkında
  “Etiketler…”, CBS şeridinde Harita › Etiket; masaüstünde `labelling/` (`mod.rs`, `form.rs`), `kentos_interaction::label_tools`,
  `layering.rs`'in menüsü, İfade oluşturucunun `Target::Labels`'ı. Ayarlar `graphics.pinnedLabels`, `graphics.unplacedLabels`.
  İkonlar (sorulmadan seçildi): Etiketler `layerLabels`, Etiketi taşı `labelMove`, Etiketi döndür `labelRotate`, Etiketi sabitle
  `labelPin`, Etiketi gizle `labelHide`, Sabit etiketleri vurgula `labelsPinned`, Yerleşmeyen etiketleri göster `labelsUnplaced`.
- **Çizim ve pafta:** yerleşen etiketler satır ya da harf olarak, hale, zemin (dikdörtgen, yuvarlak, elips), gölge ve çağrı
  çizgisiyle; sabitlerin çerçevesi, yerleşmeyenler kırmızı, gizliler Göster'de soluk (web `viewport/placedLabels.ts`, masaüstü
  `labels.rs`). Paftanın haritası motorun kâğıttaki yerleşimidir; PDF'e satırlar, harfler, zeminler, gölgeler ve çağrı çizgileri
  vektör olarak gider (web `app/sheet/mapLabels.ts`, masaüstü `labels.rs`'in `texts_in_map`'i ve `sheet_pdf.rs`).
- **Etiketleri yazıya çevir:** `ops::label_text` motorun yerleşimini yazıya çevirir (`Store::label_texts`; tek satır yazı, çok satır
  çok satırlı yazı, kıvrık eğri boyunca yazı, çağrı çizgisi çizgi); bağlı yazının yeri motorun nesne için tek başına seçtiğidir
  (`label_text_of`). ADR 0175'in hücreli kuralı ve başvurusu (`label_text_cases.py`, `fixtures/label-text/v1`) kalktı; izleri
  `labels-to-text.json` ve `labels-linked.json` motorun yerleşimiyle yenilendi, `linked-texts.json` Python başvurusuyla yeniden hesaplandı.
- **İzler:** `fixtures/interaction/v1/label-tools.json` (taşı, döndür, sabitle ve çöz, gizle ve göster, Etiketler penceresi); yeni
  beklenti `labelPins` iki oynatıcıda (`tracePlayer.mjs`, `traces/compare.rs`); masaüstü oynatıcısı etiketleri her adımdan önce
  çizim alanının yaptığı gibi yerleştirir (resim çizmez).
- **Süre ölçümleri:** `crates/shared/geometry-core/tests/all/label_engine.rs`'in `timing`'i, `crates/native/interaction/tests/perf.rs`'in
  `label_texts_on_a_large_layer`'ı, web `apps/web/scripts/perf/labels.test.ts`.

## Doğrulama

10 Ekim 2026, geliştirme makinesinde (11. nesil Intel Core i5-11300H, 8 iş parçacığı, Linux; derleme 4 işle):

- **Bağımsız başvurular** (KentOS kodu olmadan, `--check` ile hepsi geçer): `kcad_v2_reference.py` (516 dosya; örnek `labels.kcad`
  ve 31 bozuk dosya `tools/kcad/kcad.py` ile), `label_engine_cases.py` (43 durum; motorun iki platformdaki çıktısı bit bit aynı),
  `label_engine_scene.py`, `label_command_cases.py` (`cad.layers.labels` 31, `cad.labels.pin` 26 durum). `linked-texts.json`'un yerleri
  `label_engine_cases.py`'nin kuralıyla yeniden hesaplandı; ADR 0175'in `label_text_cases.py`'si motorun başvurusuna yer bıraktı.
- **Platformlar:** komut durumları web'de, masaüstünde, başsız sunucuda ve Python'da; ortak izler `label-tools.json` (yeni),
  `labels-to-text.json` ve `labels-linked.json` iki platformda bütün varyantlarda; resimler iki platformda (`tools_screens`'in
  `etiket-*`'ı, `shots.mjs labels`) 1440×900 ve 1100×650'de, iki temada: Etiketler penceresinin beş sekmesi, engel katmanı, dört araç,
  sabit ve yerleşmeyen etiketler, sahnenin çizimi.
- **Sunucu:** göç 0015 geçici veritabanlarında (`KENTOS_TEST_DB=required`): iğneler sütunda gider ve gelir, blok nesnesinde reddedilir,
  `duplicate_project` onları da kopyalar (`label_pins_come_back_and_travel_with_a_copy`). Geliştirme veritabanı `kentos_cad` yedeklenip
  (`pg_dump`, `pg_restore --list` ile denetlendi) 0013–0015'e yükseltildi; üç göçün SHA-384'ü `RELEASED`'da. `pnpm e2e:cloud` 108 denetim;
  betiğin iki eski kırığı düzeldi (seçicinin adı artık “İş türü”, dosya projesinin yüklemesinden önce proje türü verilir; yoksa soru
  penceresi Ctrl+S'yi tutar). MCP'de iki araç, ikisi de yıkıcı (`cargo test -p kentos-mcp`, 10).
- **Takım:** `pnpm typecheck`; `pnpm test` (350 dosya, 4781 test geçti, 24 atlandı); `pnpm rust:test` (3018 geçti, 37 yok sayıldı;
  API'nin veritabanı testleri geçici veritabanlarında, atlanan yok; clippy ve bağımlılık yönü temiz); `pnpm rust:test:desktop` (1277
  geçti, 194 yok sayıldı; clippy temiz); `pnpm py:test` (61); `pnpm e2e:interaction` (142 iz × 3 varyant) ve masaüstünde bütün izler
  bütün varyantlarda; `pnpm e2e` (209 denetim); `pnpm build` (Etiketler penceresi ayrı parça: 18,7 kB, CSS'i 3,6 kB);
  `pnpm inventory:check`.
- **Görünüş:** `e2e:layout`'ta Etiketler penceresi (Tek etiket, Kurallı'nın beş sekmesi uzun koşullu sınıfla, yarım kalmış ifadenin
  iletisi) 1100×650 ve 1440×900'de, iki temada (28 görünüm, sorunsuz). İlk koşuda bütün görünümlerde pencerenin gövdesi 30 piksel
  taşıyordu: Etiketleme'nin alanı daralabiliyordu ve seçicisi sağdan kesiliyordu (artık kendi boyunda kalır, Katman kalanı alır); sınıf
  listesinde kesilen koşul artık ipucunda tam okunur. Sayı alanlarının örnekleri noktalı ondalıkla (CLAUDE.md §5'in girişi). İkonlar 16, 20 ve 28
  pikselde iki temada denetlendi.
- **Süreler** (release; motor `label_engine::timing`, metinler `label_texts_on_a_large_layer`, web `scripts/perf/labels.test.ts`
  gönderilen WASM'la; yedinin ortancası):

  | İş | Masaüstü | Web | Bütçe (web) |
  |---|---|---|---|
  | 2 000 nokta adı, 40 px arayla (2 000 etiket) | 2,0 ms | 2,2 ms | 5 ms (12,5) |
  | 2 000 nokta adı, 20 px arayla; zorlama sahnesi (1 057 etiket) | 7,7 ms | 8,8 ms | 8 ms (20) |
  | 10 000 parsel, parsel kipi (9 999 etiket) | 21,3 ms | 24,8 ms | 30 ms (75) |
  | 500 eş yükselti, 25 tepe, 400 px'te bir (950 etiket) | 11,3 ms | 14,4 ms | 15 ms (37,5) |
  | 100 000 parsellik genel bakış (0 etiket: parseller 2,7 px, en küçük nesne 26 px) | 4,8 ms | 5,3 ms | 10 ms (25) |
  | 99 856 parselin iki ifadeli sınıflı metinleri | 67,1 ms | 125,5 ms | 80 ms (200) |

  İlk ölçümde parseller 68,3 ms, genel bakış 11,1 ms, yoğun noktalar 10,3 ms'ydi. Bütün sonuçlar aynı kalarak: bir birimin adayları
  artık bütün liste kurulup sıralanmadan, alt sınırlarıyla gruplar hâlinde ve gerektikçe üretilir (Uygulama; yığının sırası tam
  sıralamayla bit bit aynı, 43 durum ve izler değişmedi); alanın ızgarası bir kez sıralanmış tablodan okunur; erişilmezlik kutbunun
  uzaklık ve içerde mi soruları tek geçiştedir ve en iyiyi geçemeyecek hücre yığına girmez; çakışma soruları kutuların sınırlarıyla ön
  elenir; depo en küçük nesne eşiğini ve nesnenin türünü süzgeç, zaman ve metin aramalarından önce sorar, halkası pencerenin içindeki
  alanı kırpmaz. Eğrinin bütün yolunun açılarını önceden hesaplamak eş yükseltileri yavaşlattı (23 ms) ve geri alındı. İlk bütçe
  2 000 nokta adı için 5 ms'ydi ve 20 px aralıklı sahnede ölçülüyordu; o sahnede adların yarısı sığmaz ve iyileştirme her birini
  dener: 5 ms 40 px'lik olağan sahneye bağlandı, 20 px'lik sahne 8 ms'lik zorlama sahnesi oldu (§6). Eş yükselti sahnesi ölçümler
  sırasında gerçekçi bir paftaya (25 tepe, her biri 8 piksel aralıklı 20 halka; yine 500 eğri) çevrildi; ilk sahnenin 32 ms'si
  karşılaştırılamaz. Web'in metinleri bütçesinin içinde, masaüstününkinin iki katıdır (ifadelerin sütun motoru WASM'da).
- **Bilinen sınırlar:** görünüm payının dışına kayınca ya da yakınlaşınca yerleşim baştan yapılır; önceki karenin yerleşimi ipucu
  olmaz, etiket kareler arasında yer değiştirebilir. Alanın içinde kıvrık yerleşim yok (çevre ve sınırda var). Öbürleri §1'in kapsam
  dışısıdır.
