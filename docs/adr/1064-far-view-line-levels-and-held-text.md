# ADR 1064: Uzun çizgileri uzakta ekran toleransında sadeleştir, tekerlekte büyütülmüş yazıyı yerleştiği boyda tut

- **Durum:** kabul edildi, kısmen (6 Kasım 2026). Kapsam: plan/cad-advanced.md `CAD-ADV-19`, TODOS.md `PERF-15`,
  `PERF-13`. Uzak görünümün GPU'su 10⁵ karma çizimde **16,7 ms'ye inmedi** (25,5 ms, §7);
  madde tamamlandı sayılmaz.
- **Bağlam belgesi:** ADR 1062 (ölçüm düzeni, `perf::stability`), 1063 (blok örnekleme, eğri düzeyleri; bu ADR'nin
  “önce”si), 0597 (görünüm hareket ederken son yerleşim), 0205 §5 (Kaybolmasın), 1122 (CAD çizgi tipi kaynak eğride),
  CLAUDE.md §6, §23.
- **Web:** ertelendi (ADR 0228). Ortak çekirdek (`kentos-style-core`) bir seçenek daha aldı (`View::line_lod`,
  varsayılan kapalı); web'in kurduğu batch'ler ve `fixtures/style/v1` değişmez. TODOS.md §0.2'ye yazıldı.
- **Şema:** `.kcad` değişmedi (182). Komut, ayar, pencere değişmedi (envanter etkilenmez).

## Bağlam

ADR 1063 §5'ten sonra 10⁵ karma çizimin uzak kaydırması GPU'da p50 44,7 ms sürüyordu (hedef 16,7 ms); payın yarısı
çizgi katmanıydı: 1 000 uzun çoklu çizginin 400'er köşesi uzak görünümde alt piksel segmentlerdir. Uzak tekerlekte
arayüz iş parçacığının (MİB) p95'i ~25,7 ms'ydi (ADR 1062'de “büyütülmüş yazıların çizim kaydı”), 10⁴'te imleç
karelerinin GPU'su eğri düzeyleriyle 2,5 → 4–7 ms çıkmıştı ve açıklanamamıştı.

### Araştırma

- **AutoCAD:** düz çoklu çizgiyi uzakta sadeleştiren belgelenmiş bir ayar yok; eğriler için
  [VIEWRES](https://help.autodesk.com/cloudhelp/2021/ENU/AutoCAD-LT/files/GUID-77B1C617-E4BB-4D1E-823A-8E2B055B258E.htm)
  vardır (ADR 1063). Donanım hızlandırmada [LINEFADING](https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-LT/files/GUID-7AA085DB-2F0C-43B6-933A-DB8170E20F58.htm)
  “çizgi yoğunluğu sınırı aşılınca” çizgileri soldurur; Autodesk bunu başarım için önerir ve çok uzakta çizgilerin
  bulanık görünebileceğini söyler: uzak görünümün mürekkebi orada da kaynakla aynı değildir.
- **Civil 3D:** [Level of Detail](https://help.autodesk.com/cloudhelp/2015/ENU/Civil3D-UserGuide/files/GUID-F715FE3C-66CB-458B-848F-8743B2760F07.htm)
  uzaklaştıkça yüzeylerin gösterilen verisini azaltır; çıktı (plot) her zaman tam ayrıntıdır, düşük ayrıntıda nokta
  seçen düzenlemeler kapalıdır. KentOS'un farkı: kenet ve seçim her ölçekte kaynaktan çalışır (ADR 1062 §5–6).
- **QGIS:** vektör katmanın [Rendering özelliklerinde](https://docs.qgis.org/3.34/en/docs/user_manual/working_with_vector/vector_properties.html)
  “Simplify geometry”: küçük ölçekte çok karmaşık nesnelerin çiziminde anında genelleştirme; belge bunun çıktıda
  bozulma (komşu alanlar arasında boşluk) getirebileceğini yazar. Yöntemleri `QgsVectorSimplifyMethod`
  ([API](https://api.qgis.org/api/classQgsVectorSimplifyMethod.html)): mesafe, ızgaraya kenetleme, Visvalingam.
- **OpenLayers:** çizici geometriyi çizmeden önce görünümün çözünürlüğüne bağlı toleransla (0,5 cihaz pikseli ×
  çözünürlük) Douglas–Peucker ile sadeleştirir (`SIMPLIFY_TOLERANCE`, `getTolerance`, `simplifyTransformed`;
  [ol 6.3 kaynağı](https://app.unpkg.com/ol@6.3.0/files/renderer/vector.js)).
- **Apple GPU'da örnekleme:** Apple'ın [A11 / Metal 2 konuşması](https://developer.apple.com/videos/play/tech-talks/606/)
  karışımın yalnız örnekleri farklı (kenar) piksellerde örnek başına çalıştığını anlatır. Alt piksel segmentlerde her
  piksel kenar pikselidir: maliyet örnek sayısıyla büyür (ADR 1063 §1'in ölçümü, §3'te yeniden).

## Karar

### 1. Önce ölçüldü: kalan pay nerede

`perf::large::layers` (10⁵, yalnız bir katmanın nesneleri, bütün çizim ekranda, kaydırma; GPU p50, geri okuma tabanı
~2,3 ms dahil), ADR 1063'ün kısayollarıyla: çizgi 23,4, blok 17,9, daire 4,1, tarama 3,9, ölçü 3,4, yazı 2,6, hepsi
45,0 ms (§7). Çizgi katmanının 50 000 nesnesinden 1 000'i 400 köşeli uzun çoklu çizgidir ama segmentlerin ~%82'si
onlardadır (399 000 / ~485 000); 10⁵ uzak görünümde (~0,22 px/m) bir segment ~0,14 px'tir.

Blok katmanının kalanı (aynı ölçü, tanı düğmesi `KENTOS_MIXED_BLOCK_LEAVES_OUT`; bu dilimin ilk ikilisiyle, §3'teki geri
alınan denemeyle birlikte):

| Blok tanımından çıkarılan | GPU p50 |
|---|---:|
| hiçbiri | 15,4 ms |
| yazılar | 16,0 ms |
| daireler | 11,2 ms |
| taramalar | 16,1 ms |
| çizgiler | 8,6 ms |
| çoklu çizgi (Ada'nın çerçevesi) | 14,7 ms |
| çizgiden başka her şey | 8,9 ms |
| hiçbiri, 1× örnekleme (`graphics.msaa=1`) | 7,1 ms |

Bir Ada yerleşimi 10⁵ uzak görünümde ~1,3 × 0,9 px'tir; içinde 20 çizgi, 4 dairenin 4'er kirişi, 4 dolu üçgen ve
çerçevenin 4 kenarı vardır. Bunlar birbirine bağlı tek bir yol değil, ayrı parçalardır: çoklu çizgi sadeleştirmesi
bloklara bir şey kazandırmaz (çerçeve 4 köşelidir); maliyet ADR 1063 §1'deki gibi alt piksel parçaların aynı pikselde
örnek başına karışmasıdır (1× örneklemede yarısından azı).

### 2. Uzun çizgilerin uzak düzeyleri (PERF-15)

`View::line_lod` (yalnız çizim alanı; kâğıt, pafta, lejant, önizleme, başka sistemde çizilen harita ve
`plot_precision` kapalı) açıkken çizim alanının batch kuyusu (`BatchSink::set_line_lod`) en az 8 köşeli her çizgiyi
(`line_lod::LEAST_POINTS`; kapalı yolun halkası, ilk noktası sonda yinelenerek) iki biçimde yazar: düzey 0 eskisinin
sayılarıyla, sayı sayı aynı; sonra ADR 1063'ün tavanları (`CURVE_LEVELS` = 4, 1, 0,25 mantıksal px/m) için o tavanda
0,25 px'ten (`CURVE_TOLERANCE_PX`) uzaklaşmayan Douglas–Peucker sadeleştirmesi (dünya toleransı 0,25 / tavan:
0,0625 / 0,25 / 1 m). Düzeyler eğrilerinkiyle aynı batch'te, aynı `"lod"` sayılarıyla durur; GPU'nun düzey seçimi
değişmedi (ADR 1063 §3): kare görünümün px/m'sine göre en çok iki çizim çağrısı yapar, kaydırma ve tekerlek hiçbir şey
kurmaz. Örnekli blok tanımları da aynı kuyudan geçer (tanımdaki 8+ köşeli çizgiye düzey, yerleşimin ölçeğiyle seçilir).

- **Tek geçişte sıralama:** her köşenin önemi bir kez hesaplanır (Douglas–Peucker'in bölme uzaklığı, üst aralığınınkini
  aşmayarak); bir düzey önemi toleransını aşan köşelerdir. Bu, o toleransla ayrı ayrı çalışan Douglas–Peucker'in
  kümesiyle aynıdır ve düzeyler iç içedir (kaba düzeyin köşeleri ince düzeyde de vardır). Uzaklık kirişin doğrusuna
  değil, parçasına ölçülür: geri dönen uç ve sivri çıkıntı kalır (birim testleri).
- **Değmiyorsa düzey yok:** en kaba düzey segmentlerin en az dörtte birini atmıyorsa yol eskisi gibi tek kez yazılır
  (`MOST_KEPT` = 0,75); bellek boşa iki kat olmaz.
- **Desen evresi:** GPU'nun tiresi segmentin başındaki yol boyu uzaklıkla (`d`) ölçülür. Düzeyin her segmenti, ilk
  köşesinin düzey 0'daki uzaklığıyla başlar (test: her düzey segmentinin başı düzey 0'ın aynı köşesinin `d`'si, bite
  kadar); faz her korunan köşede kaynaktakiyle aynıdır, birleşen segmentin içinde kiriş boyuyla ilerler (sonraki korunan
  köşede kiriş ile yay arasındaki fark kadar atlar; bu, toleransın altındaki ayrıntının boyudur). **CAD çizgi tipi**
  (ADR 1122, `View::cad_phase`, çizimin kendi görünüşleri) bundan önce kaynak eğri üzerinde kesilir; kuyuya her tire
  ayrı, kesintisiz bir yol olarak gelir ve sadeleştirme uçlarını korur: tirelerin başı ve sonu değişmez.
- **Kaynak, kenet, seçim, kâğıt:** sadeleştirme yalnız ekranın batch sayılarındadır. Kenet ve seçim uzamsal dizinden
  kaynak geometriyle çalışır (`perf::stability`'nin kenet satırları üç görünümde bite kadar aynı); `kentos-plot`
  `kentos-render-wgpu`'ya bağımlı değildir ve kâğıdın kurulumunda `View::line_lod` kapalıdır (`View::default`).
- **Yakın görünüm:** 4 px/m'nin üstünde (1:950'den yakın) düzey 0 çizilir; açık/kapalı fark 0 olmalıdır ve
  `perf::stability` bunu doğrular (§4).

### 3. Denenen ve geri alınan: çizginin dörtgen payı

Çizginin dörtgeni kaplamanın bittiği yerden (`halfW + 0,5 · blur`) bir piksel daha öteye uzanıyor (`stroke.wgsl`
`ext`); 4× örneklemede bir pikselin örnekleri merkezinden en çok 0,4 px uzakta olduğundan yarım piksel yeterdi ve
aynı pikseller çizilirdi (alt piksel segmentin dörtgeni 4 × 4 yerine 3 × 3 px). Masaüstünün kaynağında
(`shader.rs`, paylaşılan v10 kaynağına dokunmadan) denendi: 10⁵ çizgi katmanı 23,8 → 22,1 ms, blok katmanı
16,4 → 16,1 ms. Atılan parçalar zaten kaplamasızdı ve ucuzdu (Apple'da karışım yalnız örnekleri farklı piksellerde
örnek başınadır); kazanç küçük, açılıp kapanamayan ortak bir gölgelendirici değişikliği olduğu için geri alındı.

### 4. Kararlılık ve eşitlik: neyi değiştirdi, neyi değiştirmedi

`perf::stability` (10⁴, gerçek GPU, [ham veri](../perf/stability-desktop-adr1064-2026-11-06.json)) yeni bir satır grubu
aldı: çizgi düzeyleri, uygulamanın kendi çiziminin (örnekleme ve eğri düzeyleri açık) üstünde açık/kapalı, aynı
kamerada. 4 px/m'nin üstünde (yakın görünümler ve birinci tavanın üstü) görünür fark 0 olmalıdır; test bunu doğrular.
Uzak satırlar ölçülür ve raporlanır, doğrulanmaz. Görünür / farklı / en büyük kanal farkı (karşılaştırılan 364 518
piksel):

| Görünüm (px/m) | Çizgi düzeyleri |
|---|---:|
| Uzak (0,70) | 626 / 27 683 / 98 |
| 1:200 (18,9) | 0 / 0 / 0 |
| 1:20 (189) | 0 / 0 / 0 |
| Birinci tavanın altı (4 − 4·10⁻⁷) | 44 / 3 714 / 80 |
| Birinci tavanın üstü (4 + 4·10⁻⁷) | 0 / 287 / 1 |
| En uzak düzey (0,125) | 284 / 3 852 / 149 |

Uzakta fark yalnız uzun çoklu çizgilerdedir (fark resmi `.run/gui-evidence/cad-adv-19/kisayol-cizgi-düzeyleri-uzak-*`):
eski yolda 0,14 px'lik yüzlerce segmentin her biri kenar yumuşatmasıyla aynı piksellere karışır ve çizgiyi 1 px'lik
genişliğinden koyu ve kalın gösterir; sadeleşmiş düzeyde çizgi ince ve açık görünür. Geometrik sapma 0,25 px'in
altındadır; fark mürekkep yoğunluğudur, eğri düzeylerindeki (ADR 1063 §4) gibi. Gizlenmedi: kabul sahibinindir.
Birinci tavanın üstündeki 287 piksel 1/255'lik yuvarlamadır (düzey 0'ın sayıları batch'in ana parçasından sonra çizilir;
aynı renkte karışım sırası). Diğer satırlar ADR 1063 ile aynı kaldı: kaydırmalar uzak/1:200/1:20 görünür fark 0, aynı
görünüm yeniden 0; blok örnekleme her görünümde 0; kenet üç görünümde bite kadar aynı (uzun çizginin 200. köşesi
dahil), dairenin en yakın noktası yarıçapın 1,4·10⁻¹⁰ m içinde.

Uygulamanın kendi resimleri (10⁴ ve 10⁵, uzak, iki tema, 1440×900 ve 1100×650; `large_scenes.rs`'in yeni
`buyuk-uzak-duzeysiz`, `buyuk-100k-uzak`, `buyuk-100k-uzak-duzeysiz` sahneleri): 10⁵ uzakta 1 602 görünür / 46 920
farklı piksel, 10⁴ uzakta 315 / 21 438 (açık temada 0 / 18 737), yalnız uzun çizgilerde.

### 5. 10⁴'te imleç karesinin GPU'su: iş değil, GPU'nun saati

ADR 1063 §5'in açık gözlemi yeniden üretildi ve ayrıştırıldı (`perf::large` çocuğu `KENTOS_PERF_TRACE_VIEW` ile kare
kare; `perf::cursor::cursor`):

| 10⁴, imleç karesi GPU p50 (ms) | uzak ilk | yakın ilk | uzak ısınmış | yakın ısınmış |
|---|---:|---:|---:|---:|
| `perf::large`, kısayollar kapalı | 2,19 | 2,25 | 3,46 | 2,33 |
| yalnız örnekleme | 2,15 | 2,29 | 2,25 | 2,29 |
| yalnız eğri düzeyleri | 4,42 | 6,84 | 6,18 | 7,61 |
| yalnız çizgi düzeyleri (§2) | 6,17 | 7,55 | 4,90 | 3,54 |
| örnekleme + eğri düzeyleri (ADR 1063) | 5,48 | 4,55 | 6,75 | 7,37 |

Her karede resim tutuluyor (`picture_kept`), yüklenen 192 B (çerçeve tekdüzesi) ve stilli katmanlardan **hiçbir geçiş
çizilmiyor** (geçici sayaçla doğrulandı, sayaç kaldırıldı): karenin GPU işi dört satırda da aynıdır (resmin bindirilmesi,
imleç ve vurgu kaplaması, arayüz). Fark, öncesinde ne çizildiğindedir: düzeyler uzak karenin işini 23 → 7 ms'ye
indirdiği için iz boyunca GPU daha hafif yüklenir ve macOS GPU'nun başarım durumunu düşürür (Apple:
[WWDC21 10157](https://developer.apple.com/videos/play/wwdc2021/10157/), başarım durumu sıcaklığa, sistem ayarlarına
ve GPU kullanımına göre işletim sistemince değiştirilir). Kanıt, aynı ikili ve aynı kısayollarla (ADR 1063'ünkiler)
yalnız imleç karelerinden öncesi değiştirilerek (`KENTOS_PERF_BEFORE`):

| Öncesi | uzak p50 / p95 | yakın p50 / p95 |
|---|---:|---:|
| hiçbir şey | 3,43 / 4,71 | 3,62 / 4,78 |
| 40 ağır kare (kısayollar kapalı uzak kare) | **2,08** / 3,45 | **2,23** / 3,52 |
| 3 s boşta | 3,40 / 5,24 | 7,64 / 8,09 |

Sonuç: imleç karesinin işi artmadı; GPU yükü hafifledikçe saati düşüyor ve sabit iş daha uzun sürüyor. Düzeltilecek
bir şey yoktur (GPU'yu boşuna yüksek saatte tutmak güç harcar); ölçümde, bir karenin GPU süresinin o ana kadarki yüke
bağlı olduğu not edilir ve resim tutulan karelerde “iş” karşılaştırması `picture_kept` ve yüklenen bayta bakılarak
yapılır. Tüm bu süreler 16,7 ms bütçesinin içindedir.

### 6. Tekerlekte büyütülmüş yazı yerleştiği boyda (PERF-13)

**Ölçüldü** (`perf::cursor::wheel`, 10⁵, uzak, sabit iz: 8 içeri, 8 dışarı, iki tur; kare kare MİB'in parçaları ve
yapılan yazı resmi sayısı, `label_picture::counts`): içeri dönen sekiz karenin her biri MİB'de 25–44 ms sürüyordu ve
tamamı çizim kaydıydı (olay, uygulama, görünüm, düzen < 0,3 ms); her karede 399 yazı resmi baştan boyanıyordu
(tiny-skia, ADR 0055'in 17 Ekim eki). Neden: görünüm hareket ederken son yerleşim tutulur (ADR 0597), yerleşimin
büyütme çarpanı da (`Grow::k`); Kaybolmasın yazıyı 8 px'e büyütür (`k = 8 / (h · ölçek)`), ama tutulan `k` yeni ölçekle
çarpılınca yazının ekrandaki boyu her karede değişiyordu: her kare her döndürülmüş yazının resmini yeni boyda yeniden
yapıyordu ve büyüttükçe resimler büyüyordu (25 → 44 ms). Dışarı dönen kareler aynı boyları bulduğu için ucuzdu.

**Karar:** hareket eden görünümde büyütülmüş yazı (k ≠ 1) yerleştiği ölçekteki boyunda çizilir (`labels.rs`
`held_grown`): ekrandaki yüksekliği yerleşimdeki `ölçek · k`'dir ve her karede aynı sayıdır (bite kadar), çapası
yerinde kalır. Bu, Kaybolmasın'ın ve Ekranda sabit'in tanımıdır (büyütülmüş yazının ekran boyu ölçekten bağımsızdır);
yeni bir yerleşim aynı boyu verirdi. Kaybolmasın yazıyı gerçek boyunun altına küçültmez (ölçek yazıyı 8 px'in üstüne
çıkardıysa görünümün kendi ölçeği); Ekranda sabit küçültmez sınırı taşımaz. Kaydırmada (ölçek aynı) eski yol bite kadar
aynen çalışır. Gerçek boy (`Print`) ve k = 1 olan yazılar değişmedi. Böylece resimler bulunur; dörtte bir piksellik
konumu değişen yazı, hareket sürerken kendi çeyreği yoksa aynı yazının son resmini en yakın tam piksele koyar
(`label_picture.rs`, en çok yarım cihaz pikseli); görünüm durunca (150 ms) yazılar yeniden yerleşir ve tam konumda
çizilir.

**Görüneni değiştirdiği:** tekerlek dönerken büyütülmüş yazılar artık ölçekle büyüyüp küçülmez, durduklarında
gösterecekleri boyda kalır (öncesinde 150 ms boyunca yanlış boyda görünüp duruşta sıçrıyordu). Hareket sırasında en çok
yarım piksellik konum farkı olabilir; durunca yoktur. Duruştaki yeniden yerleşimin karesi değişmedi (§7: ~50 ms, ayrı
iş).

### 7. Ölçüm (Apple M5, release)

Ortam: Apple M5 (10 çekirdek), 32 GB, macOS 26.6.2 (25G83), GPU Apple M5 (Metal), rustc 1.96.0, `--release` (lto
thin), 1440×900, 4× örnekleme. “Önce” `main`'in `3e5087350` ikilisi (ADR 1063'ün çizimi), “sonra” bu dalın ikilisi;
aynı sabit iz arka arkaya, ortak ağır iş kilidi tutularak. Bütün tablolar:
[docs/perf/large-desktop-adr1064-2026-11-06.md](../perf/large-desktop-adr1064-2026-11-06.md).

**Katman katman, 10⁵, uzak kaydırma** (`perf::large::layers`, aynı ikili, çizgi düzeyleri kapalı → açık;
`KENTOS_PERF_SHORTCUTS=adr1063` → uygulamanın kendisi):

| Katman (nesne) | GPU p50 | Görünümün tamponları | İlk kare MİB |
|---|---:|---:|---:|
| Çizgi (50 000) | 23,4 → 7,2 ms | 24,0 → 48,9 MB | 47,8 → 62,1 ms |
| Yazı (20 000) | 2,6 → 2,8 ms | 0,8 → 0,8 MB | 11,6 → 13,0 ms |
| Daire/yay (10 000) | 4,1 → 4,3 ms | 36,9 → 36,9 MB | 32,9 → 33,8 ms |
| Blok (10 000) | 17,9 → 17,0 ms | 3,0 → 3,0 MB | 117,2 → 121,9 ms |
| Tarama (5 000) | 3,9 → 4,2 ms | 5,9 → 5,9 MB | 51,1 → 51,4 ms |
| Ölçü (5 000) | 3,4 → 3,8 ms | 2,0 → 2,0 MB | 8,0 → 8,6 ms |
| Hepsi | 45,0 → 25,9 ms | 70,3 → 95,3 MB | 214,4 → 253,8 ms |

**Bütün iz** (ısınmış koşu, ms; önce → sonra):

| | 10 000 | 100 000 | 1 000 000 |
|---|---:|---:|---:|
| Uzak kaydırma GPU p50 | 7,6 → 7,4 | 45,3 → 25,5 | 406,5 → 281,8 |
| Uzak tekerlek GPU p50 | 7,2 → 7,6 | 28,2 → 21,2 | 273,3 → 162,9 |
| Uzak tekerlek MİB p95 / bütçeyi aşan kare | 1,2 → 1,1 / 0 → 0 | 25,7 → 1,4 / 6 → 0 | 53,7 → 29,3 / 30 → 12 |
| Uzak tekerlek MİB p95, ilk koşu / aşan | 26,7 → 1,2 / 7 → 0 | 36,5 → 1,2 / 7 → 0 | 56,9 → 49,2 / 28 → 13 |
| Yakın 1:200 kaydırma GPU p50 | 7,5 → 7,4 | 7,1 → 6,0 | 10,0 → 7,9 |
| Uzak imleç GPU p50 (§5) | 2,3 → 4,2 | 2,5 → 2,8 | 5,9 → 5,6 |
| Açılış: ilk kare MİB / GPU | 76,0 / 24,0 → 48,3 / 16,7 | 294,2 / 33,1 → 239,6 / 39,7 | 2 248 / 240 → 2 239 / 273 |
| Hepsini seç MİB | 16,9 → 17,8 | 181,9 → 173,4 | 2 026 → 2 020 |
| Tuş yankısı en kötü (MİB + GPU) | 18,0 → 16,6 | 27,6 → 16,9 | 20,7 → 15,6 |
| Görünümün GPU tamponları (MB) | 22 → 24 | 84 → 109 | 709 → 959 |
| Süreç tepe ayak izi (MB) | 1 052 → 1 013 | 2 294 → 2 192 | 13 312 → 14 336 |

İptal Esc karesi 3,2–7,1 ms, iş parçacığının durması ≤ 1,7 ms: hedefte kaldı.

**Hedefe göre:** 10⁵ uzak kaydırma GPU p50 **25,5 ms; 16,7 ms tutmadı.** Çizgi katmanı 23,4 → 7,2 ms; kalan payın
yarıdan çoğu blok katmanıdır (17,0 ms, §1). Uzak tekerlekte MİB 10⁴ ve 10⁵'te bütçe içine girdi (bütçeyi aşan kare
kalmadı); 10⁶'da medyan 33,6 → 3,1–5,6 ms'ye indi, p95 29,3 ms (12 kare) ve duruştaki yeniden yerleşimin karesi (~50 ms,
iz dışında) kaldı.

### 8. Sınırlar

- 10⁵ uzak kaydırmada GPU 25,5 ms, 10⁶'da 282 ms: hedefte değil. Sonraki pay bloklardır: uzakta ~1 px'lik yerleşimin
  40'a yakın ayrı parçası. Tolerans içinde bir uzak blok vekili (tanımın parçalarını tavanın toleransında birleştiren
  bir düzey) bugünkü üç tavanla bu ölçekte bir şey kazandırmaz (Kapak'ın 1 m'lik karesi 1 px/m tavanında 1 px'tir); daha
  ince tavanlar ya da örnekleme sayısını uzakta düşürmek (görünenin kenar yumuşatmasını değiştirir) sahibinin kararıdır.
- Çizgi düzeylerinin belleği: uzun çizgiler düzey başına yeniden yazılır (ilk düzey 0,0625 m'de köşelerin ~%80'ini
  tutar); çizgi katmanı 24 → 49 MB, 10⁶'da görünümün tamponları 709 → 959 MB, ilk kurulum 47,8 → 62,1 ms (10⁵ çizgi
  katmanı).
- Uzakta uzun çizgilerin mürekkep yoğunluğu değişir (§4): kabul sahibinindir.
- Tekerlekte büyütülmüş yazı artık durunca alacağı boyda gösterilir; hareket sürerken en çok yarım cihaz pikseli konum
  farkı olabilir (§6). Duruştaki yeniden yerleşim (10⁵'te ~50 ms, tek kare) ayrı iştir.
- Web çizicisi (ADR 0228) `View::line_lod`'u bilmez; ortak çekirdekte kapalıdır.

## Sonuçlar

- 10⁵ uzak kaydırma GPU p50 45,3 → 25,5 ms, 10⁶ 406 → 282 ms; çizgi katmanı 3,2× hızlandı; yakın görünüm aynı kaldı
  (piksel piksel 0 fark).
- Uzak tekerlekte arayüz iş parçacığı 10⁵'te p95 25,7 → 1,4 ms; bütçeyi aşan kare 6 → 0.
- 10⁴ imleç karesinin GPU artışının nedeni bulundu: iş değil, GPU'nun başarım durumu (§5).
- CAD-ADV-19 `[~]` kalır: uzak 60 Hz bütçesi 10⁵ ve 10⁶'da tutmuyor.
