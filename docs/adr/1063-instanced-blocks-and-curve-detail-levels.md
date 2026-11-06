# ADR 1063: Blokları tanımından bir kez çiz, eğrileri uzakta seyrek kirişle göster

- **Durum:** kabul edildi, kısmen (6 Kasım 2026). Kapsam: plan/cad-advanced.md `CAD-ADV-19`, TODOS.md `PERF-11`,
  `PERF-12`. Uzak görünümün GPU'su 10⁵ karma çizimde **16,7 ms'ye inmedi** (§5); madde tamamlandı sayılmaz.
- **Bağlam belgesi:** ADR 1062 (ölçüm düzeni, `perf::mixed`, `perf::stability`), 0121 (parçalar), 0144 (blok
  yerleşimi), 0157 (karo ve f32 hi/lo), 0793 (kâğıt kalınlığı), 0930 §23 (tekrar eden üçgenleme), CLAUDE.md §6, §23.
- **Web:** ertelendi (ADR 0228). Ortak çekirdek (`kentos-style-core`) iki yeni seçenek aldı; ikisi de varsayılan
  kapalıdır, web'in kurduğu katmanlar ve `fixtures/style/v1` değişmez. TODOS.md §0.2'ye yazıldı.
- **Şema:** `.kcad` değişmedi (182). Komut, ayar, pencere değişmedi (envanter etkilenmez).

## Bağlam

ADR 1062 §3 uzak görünümde (bütün çizim ekranda) kaydırmanın GPU'da 10⁵ nesnede ~0,22 s sürdüğünü ve payın blok
katmanında olduğunu ölçtü: her yerleşim parçalarını ayrı açıyor, stilli geometri daireyi ekran boyundan bağımsız 72
kirişle çiziyordu. Önerilen iki iş: (a) blok tanımını bir kez kurup yerleşimleri dönüşümle çizmek, (b) stilli eğrileri
ekran boyuna göre kurmak.

### Araştırma

- **AutoCAD:** [VIEWRES](https://help.autodesk.com/cloudhelp/2020/ENU/AutoCAD-Core/files/GUID-77B1C617-E4BB-4D1E-823A-8E2B055B258E.htm)
  daire, yay ve eğrili çoklu çizgiyi kısa vektörlerle gösterir; vektör sayısı “yakınlaştırma yüzdesi” ile büyür ve
  ayar değişince çizim yeniden üretilir (REGEN). Ekrandaki kiriş sayısı görünüme bağlıdır, çizim verisi değişmez.
  Donanım hızlandırmalı sürümlerde aynı işi
  [WHIPARC](https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-LT/files/GUID-DFB5E247-3ADF-4F73-9DE3-E6EA4D2F4AAA.htm)
  ve grafik sürücüsü üstlenir. KentOS'un farkı: yeniden üretim (REGEN) beklenmez; düzeyler önceden kurulur (§3).
- **ODA (Teigha/Visualize, Graebert CFx SDK belgesi):**
  [`OdGsBlockReferenceNode`](https://docs.dev.graebert.com/html/2026.0.1/cfx/d9/d4c/class_od_gs_block_reference_node.html)
  bir blok başvurusunun ortak tanımını (`sharedDefinition`) ve ortak başvurunun dönüşümünü (`getSharedTransform`)
  tutar; kesilmiş (clip) başvurular ve kendi geometrisini taşıyanlar (`kBrOwnGeom`) ayrı yoldan çizilir. KentOS'ta
  karşılığı §2'deki “tanım bir kez, yerleşim dönüşümle; benzeri olmayan parça parça” ayrımıdır.
- **wgpu / WebGPU örnekleme (instancing):** [Learn Wgpu, Instancing](https://sotrh.github.io/learn-wgpu/beginner/tutorial7-instancing/)
  aynı ağı örnek başına bir dönüşümle (`VertexStepMode::Instance` tamponu) çizer. WebGPU'nun
  [adım kipi](https://www.w3.org/TR/webgpu/#enumdef-gpuvertexstepmode) yalnız `vertex` ve `instance`'tır (bölen yok);
  stilli çizgi zaten parça başına örneklenmiş olduğundan (6 köşe) iki düzeyli örnekleme için tanımın parçaları köşe
  başına yinelenip köşe tamponuna, yerleşim örnek tamponuna konur (§2).
- **Döşemeli (TBDR) GPU:** [PowerVR TBDR](https://docs.imgtec.com/starter-guides/powervr-architecture/html/topics/tile-based-deferred-rendering-index.html)
  saydam (karışımlı) çizimde gizli yüzey elemenin işe yaramadığını anlatır: üst üste binen her parça işlenir. Apple
  GPU'ları da döşemelidir. §1'deki ölçüm bunu bu çizimde doğrular.

## Karar

### 1. Önce ölçüldü: GPU süresi kirişlerin örtüştüğü piksellerde, bellek bant genişliğinde değil

10⁵ karma çizimin yalnız blok katmanı, uzak kaydırma, GPU p50 (`perf::large::layers`, aynı ikili, tanı düğmeleriyle):

| Deneme | GPU p50 | Görünümün tamponları |
|---|---:|---:|
| Olduğu gibi (4× örnekleme, 72 kiriş) | 170,8 ms | 152 MB |
| Aynı tamponlar, 1× örnekleme (`graphics.msaa=1`) | 31,2 ms | 152 MB |
| Dairenin kirişleri 72 → 8 (tanı) | 26,8 ms | 30 MB |
| Bloklardan daireler çıkarılınca | 10,1 ms | 14 MB |

Aynı 152 MB okunurken örnekleme 4×'ten 1×'e inince süre 5,5 kat düşüyor: kare, köşe tamponunun okunmasıyla değil,
alt piksel kirişlerin aynı piksellere yüzlerce kez karışmasıyla doluyor (uzak görünümde bir Ada yerleşimi ~3 × 2 px'tir,
içinde ~310 kiriş). Sonuç: örnekleme (instancing) belleği ve kurulumu küçültür, uzak karenin GPU süresini düşürmez;
GPU süresini kirişlerin sayısı belirler (§3). İki iş ayrı ayrı yapıldı ve ayrı ölçüldü (§5).

### 2. Blokların yerleşimleri tanımdan çizilir (PERF-11)

`kentos_style_core::style::build` (yeni `build/instances.rs`) ekranın isteğiyle (`View::instancing`, yalnız çizim
alanı; kâğıt, pafta, lejant, önizleme kapalı) katmandaki yerleşimleri görünümlerine göre sayar (tanım, parçaların
kümeleri, renk). Aynı görünümden en az iki yerleşim varsa tanımın düz parçaları (`Flat::pieces`, taban noktasından)
bir kez, yerleşimlerin çizildiği yoldan (`draw_object`) ayrı bir kayıt havuzuna (`BatchSink::recorder`) çizilir;
her yerleşim o tanımın her batch'ine bir örnek ekler: benzerliğin doğrusal parçası (sütunlarıyla), karonun
başlangıcından ötelemesi ve düzgün ölçeği (`INSTANCE_FLOATS` = 8). Örnekli batch, aynı stildeki “kardeş” batch'in
sırasını alır ve hemen ardından çizilir; parçalara ayrılmış katmanda (`merged_order`) anahtarı kardeşininkidir.

**Örneklenmeyen (eskisi gibi parça parça çizilen) durumlar,** her biri çizileni değiştirebileceği için:

- kendi y çarpanı olan (düzgün olmayan) yerleşim: eğriler elips olur, desen uzunluğu ölçekle değişir;
- kırpılmış (clip) yerleşim, gizli parçası olan yerleşim (`piece_shown`), kendi çizim sırası olan nesne (bant ≠ 0);
- yalın görünüm (`MODE_SET`) dışındaki çizim (kendi sembolü, kural/tematik çizici, ölçü, resim);
- tanımda kesikli çizgi ya da çizgi tipi (desen yolun başından ölçülür; dönen yerleşimde başlangıç döner), tarama ya
  da desenli dolgu (desen dünyaya bağlıdır), işaret (marker), efekt grubu, ölçek aralığı, sonsuz çizgi/ışın;
- haritanın başka sistemde çizildiği görünüm, duvarları kendi ayrıntısında çizen bölünmüş görünüm.

GPU'da (`styled/gpu.rs`, `styled/instances.wgsl`, yalnız masaüstü uzantısı; paylaşılan v10 kaynağı değişmez)
iki yeni boru hattı vardır: çizgi segmenti tanımda altı köşesi için yinelenir (köşe tamponu), yerleşim örnek
tamponudur; konum `toPxLow`'a yüksek parça öteleme, alçak parça öteleme alçağı + yerleşmiş küçük uzaklık olarak girer
(far bir yerleşimin basamağı kaybolmaz). Düz dolgu üçgenleri aynı biçimde. Ölçü boyu (`mt.x`) örneğin ölçeğiyle
çarpılır. Örnekli batch'in kendi tamponu kurulamazsa (beklenmeyen tür) hiçbir şey çizilmez, ham sayıları asla.

**Tam aynı olmayan tek nokta:** dairenin kirişleri tanımda daire başlangıcından (açı 0), parça parça yolda ise
yerleşmiş dairenin başlangıcından sayılır; dönen yerleşimde 72-gen dönmüş olur. İki çokgen de dairenin 0,25 px
toleransı içindedir (72 kirişte sapma R·0,00095; R ≤ 262 px). `perf::stability` iki yolu aynı görünümlerde karşılaştırır
(§4).

### 3. Stilli eğrilerin uzak düzeyleri (PERF-12)

`View::curve_lod` (yalnız çizim alanı) dairenin, yayın ve elipsin çizgisini iki biçimde kurar: düzey 0 eskisi
gibidir (72 kiriş; yakın görünüm değişmez), sonra her tavan için (`CURVE_LEVELS` = 4, 1, 0,25 mantıksal px/m) o tavanda
0,25 px'ten (`CURVE_TOLERANCE_PX`, çizim alanının kendi toleransı) uzaklaşmayan kiriş sayısı:
`n = ⌈π / acos(1 − 0,25 / (r·tavan))⌉`, en az 4, en çok 72 (yay açısıyla orantılı, elips büyük yarı ekseniyle).
Batch'in sayıları `[her düzeyde çizilen | düzey 0 | düzey 1 | …]` sırasıyla yazılır (`"lod"`); kare, görünümün
mantıksal px/m'si (örnekli batch'te en büyük ölçekle çarpılarak) hangi tavanın altındaysa o düzeyi çizer: en çok iki
çizim çağrısı. Düzey yeniden kurulmaz (AutoCAD'in REGEN'i yok); kaydırma ve tekerlek hiçbir şeyi yeniden kurmaz.
Büyüteç kendi ölçeğiyle düzey seçer.

Düzeyler yalnız çizgidir: dolgu, işaret ve karmaşık çizgi tipi (`cad_band`, `cad_marks`) düzey 0'da bir kez çizilir;
karmaşık tip eğriyi kendisi işliyorsa düzey yoktur. Kesikli eğride desen kiriş boyuyla ilerler; seyrek kirişte çevre
farkı en çok ~2,1 × tolerans (≤ 0,5 px) birikir. Kenet, seçim, ölçü ve kâğıt kaynak geometriyi kullanır (ADR 1062
§5–6); `kentos-plot` `kentos-render-wgpu`'ya bağımlı değildir ve plotun kurulumunda iki seçenek de kapalıdır.

**Bellek:** örneklenmeyen eğrilerin çizgisi düzeylerle büyür (10⁵ karma çizimin daire katmanında §5); bloklarda
düzeyler yalnız tanımdadır.

### 4. Kararlılık ve eşitlik ölçütü; düzeylerin uzakta değiştirdiği

`perf::stability` iki satır grubu aldı; her kısayol tek başına açılıp kapatılır (`Styles::instancing`, `curve_lod`),
aynı kamerada: uzak, 1:200, 1:20, birinci tavanın 10⁻⁷ altı ve üstü, en uzak düzey. (a) **Blok örnekleme:** görünür fark
0 olmalıdır, test bunu doğrular. (b) **Eğri düzeyleri:** ölçülür ve raporlanır; doğrulanmaz, çünkü uzakta çizileni
gerçekten değiştirir:

Eski yolda bir ekranda 1–5 px yarıçaplı daire 72 kısa kirişle çizilir; her kirişin yuvarlak uçlu kenar yumuşatması
komşu kirişlerinkiyle aynı piksele onlarca kez karışır ve halkayı gerçek 1 px'lik çizgisinden koyu, küçük dairede dolu
bir nokta gibi gösterir. Seyrek kirişte üst üste binme azdır: halka ince ve ortası boş görünür (resimler §5). Kirişlerin
geometrik sapması 0,25 px'in altındadır; fark mürekkep yoğunluğudur. Bu, eski çizimin bir yanlışını düzeltse de
çizileni değiştirdiği için gizlenmedi: rapor ve resimler sahibine sunulur, kabul onun kararıdır. Yakın görünümde
(4 px/m'nin üstü, 1:950'den yakın) düzey 0 çizilir ve eskisiyle aynıdır (§5'te 1:200 ve 1:20 satırları).

`eğri düzeyi` geçiş satırı (tavanın iki yanı) da raporlanır, doğrulanmaz: bu ölçekte kontrolü (düzey 0'ın içinde aynı
adım, iki resim de eski kirişlerle) de durgun değildir; fark düzeylerden bağımsızdır (§5).

### 5. Ölçüm (Apple M5, release)

Ortam: Apple M5 (10 çekirdek), 32 GB, macOS 26.6.2 (25G83), GPU Apple M5 (Metal), rustc 1.96.0, `--release` (lto
thin, codegen-units 4), commit `ba9a78280` + bu dilim, 1440×900, 4× örnekleme. “Önce” aynı ikilinin kısayollar kapalı
koşusudur (`KENTOS_PERF_SHORTCUTS=off`, eski çizimle aynı yol), “sonra” açık; aynı sabit iz arka arkaya, ortak ağır iş
kilidi tutularak. Bütün tablolar: [docs/perf/large-desktop-adr1063-2026-11-06.md](../perf/large-desktop-adr1063-2026-11-06.md).

**Katman katman, 10⁵ karma çizim, uzak kaydırma** (`perf::large::layers`):

| Katman (nesne) | GPU p50 önce → sonra | Görünümün tamponları önce → sonra | İlk kare MİB önce → sonra |
|---|---:|---:|---:|
| Çizgi (50 000) | 23,6 → 24,0 ms | 24,0 → 24,0 MB | 52,6 → 48,6 ms |
| Yazı (20 000) | 2,5 → 2,5 ms | 0,8 → 0,8 MB | 19,4 → 11,8 ms |
| Daire/yay (10 000) | 25,2 → 4,1 ms | 26,2 → 36,9 MB | 21,4 → 36,0 ms |
| Blok (10 000 yerleşim, 31 parça) | 166,2 → 17,0 ms | 152,4 → 3,0 MB | 210,7 → 121,8 ms |
| Tarama (5 000) | 3,9 → 3,8 ms | 5,9 → 5,9 MB | 50,7 → 49,2 ms |
| Ölçü (5 000) | 3,5 → 3,4 ms | 2,0 → 2,0 MB | 8,6 → 8,7 ms |
| Hepsi | 218,1 → 45,0 ms | 207,4 → 70,3 MB | 233,8 → 212,5 ms |

Blok katmanında kısayolları ayrı ayrı (aynı iz): yalnız örnekleme 170,5 → 168,3 ms ve 152 → 3 MB (GPU'yu değiştirmedi,
§1); yalnız eğri düzeyleri 170,5 → 25,5 ms (183 MB: her yerleşimin düzeyleri ayrı); ikisi 17,0 ms, 3 MB.

**Bütün iz** (ısınmış koşu, ms; önce → sonra):

| | 10 000 | 100 000 | 1 000 000 |
|---|---:|---:|---:|
| Uzak kaydırma GPU p50 | 23,2 → 7,6 | 221,2 → 44,7 | 1 772 → 409 |
| Uzak tekerlek GPU p50 | 17,1 → 11,1 | 134,8 → 29,1 | 1 075 → 273 |
| Uzak tekerlek MİB p95 / bütçeyi aşan kare | 1,3 → 1,1 / 0 → 0 | 51,4 → 25,7 / 29 → 7 | 35,0 → 53,1 / 28 → 30 |
| Yakın 1:200 kaydırma GPU p50 | 7,5 → 7,5 | 6,9 → 5,8 | 11,7 → 11,8 |
| Açılış: ilk kare MİB / GPU | 64,8 / 18,6 → 23,4 / 15,9 | 259,7 / 49,1 → 222,7 / 36,6 | 2 682 / 392 → 2 227 / 216 |
| İlk etkileşim, imleç GPU | – → 29,4 | – → 77,4 | – → 871 |
| Bir blok yerleşimini taşı MİB | 22,2 → 8,7 | 9,4 → 3,9 | 31,2 → 25,3 |
| Hepsini seç MİB | 17,0 → 15,2 | 171,7 → 172,2 | 2 292 → 2 059 |
| Görünümün GPU tamponları (MB) | 36 → 22 | 222 → 84 | 2 081 → 709 |
| Süreç tepe ayak izi (MB) | 1 226 → 1 037 | 3 185 → 2 306 | 16 384 → 12 288 |

(İlk etkileşimin “önce” sütunu ADR 1062'de: 55,5 / 395,2 / 7 285 ms.) Tuş yankısı en kötü 14,7 / 16,0 / 14,8 ms, iptal
Esc karesi 3,3–5,6 ms: hedefte kaldı.

**Hedefe göre:** 10⁵ uzak kaydırma GPU p50 44,7 ms; **16,7 ms tutmadı.** Kalan payın yarısı çizgi katmanıdır
(24 ms, değişmedi): 1 000 uzun çoklu çizginin 400'er köşesi uzak görünümde alt piksel segmentlerdir (aynı örtüşme,
§1); bu, eğri değil düz çizgidir, PERF-12'nin kapsamı dışında kaldı. 10⁴ uzak kaydırma 7,6 ms ile bütçe içine girdi.

**Kararlılık** (`perf::stability`, 10⁴, [ham veri](../perf/stability-desktop-adr1063-2026-11-06.json)): kaydırma
satırları ADR 1062 ile aynı (uzak/1:200/1:20 görünür fark 0, aynı görünüm yeniden 0); kenet üç görünümde bite kadar
aynı, dairenin en yakın noktası yarıçapın 1,4·10⁻¹⁰ m içinde. Kısayollar tek tek açık/kapalı, görünür / farklı / en
büyük kanal farkı:

| Görünüm (px/m) | Blok örnekleme | Eğri düzeyleri |
|---|---:|---:|
| Uzak (0,70) | 0 / 376 / 4 | 2 483 / 21 620 / 193 |
| 1:200 (18,9) | 0 / 136 / 5 | 0 / 0 / 0 |
| 1:20 (189) | 0 / 0 / 0 | 0 / 0 / 0 |
| Birinci tavanın altı (4 − 4·10⁻⁷) | 0 / 425 / 3 | 663 / 6 848 / 133 |
| Birinci tavanın üstü (4 + 4·10⁻⁷) | 0 / 424 / 4 | 0 / 49 / 2 |
| En uzak düzey (0,125) | 0 / 35 / 5 | 73 / 2 214 / 156 |

Örneklemenin farkı 1–5/255'lik yuvarlamadır (karşılaştırılan 364 518 pikselin ≤ %0,12'si). Eğri düzeylerinin uzak
farkı §4'te anlatılan mürekkep yoğunluğudur (resimler `.run/gui-evidence/cad-adv-19/kisayol-düzeyleri-*`); yakında
sıfırdır. `eğri düzeyi` geçiş satırı: tavanın iki yanı 1 377 görünür piksel, kontrolü (iki resim de düzey 0) 901: geçişin
payı kontrolün üstündeki kısımdır.

**Açık gözlem:** 10⁴ nesnede imleç karelerinin GPU'su 2,5 → 4–7 ms çıktı. Resim saklanıyor (tanı izi: resmin anahtarı
sabit, hedef onu tutuyor); fark yalnız eğri düzeyleri açıkken var, 10⁵ ve 10⁶'da yok; nedeni bulunamadı, bütçenin
içinde. Ayrı iş olarak yazıldı.

### 6. Sınırlar

- 10⁵ uzak kaydırmada GPU 44,7 ms, 10⁶'da 409 ms: hedefte değil. Sonraki pay: uzun çoklu çizgilerin uzak görünüm
  sadeleştirmesi (ekran toleransında, kaynak ve kenetten bağımsız; desen evresi korunarak) ve tekerlekte MİB (PERF-13).
- Örneklenmeyen eğrilerin çizgi belleği düzeylerle büyür (daire katmanı 26 → 37 MB); ilk kurulumu da (21 → 36 ms).
- Eğri düzeyleri uzakta küçük dairelerin mürekkep yoğunluğunu değiştirir (§4); kabul sahibinindir.
- Düzgün olmayan, kırpılmış, desenli, kesikli, tematik çizilen yerleşimler eskisi gibi parça parça çizilir.
- Web çizicisi (ADR 0228) iki seçeneği bilmez; ortak çekirdekte kapalıdır.

## Sonuçlar

- Blok katmanının görünüm belleği 10⁵'te 152 → 3 MB, 10⁶'da görünümün tamponları 2,08 → 0,71 GB; yerleşim taşıma ve
  ilk kare MİB'de kısaldı.
- Uzak görünümün GPU'su 10⁵'te ~5×, 10⁶'da ~4× hızlandı; yakın görünüm aynı kaldı (piksel piksel 0 fark).
- CAD-ADV-19 `[~]` kalır: uzak 60 Hz bütçesi 10⁵ ve 10⁶'da tutmuyor.
