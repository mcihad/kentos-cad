# ADR 1065: Uzakta birkaç pikseli geçmeyen bloğu tanımın çizgilerini saran kutuyla çiz

- **Durum:** kabul edildi (6 Kasım 2026). Kapsam: plan/cad-advanced.md `CAD-ADV-19`, TODOS.md `PERF-11`, `PERF-15`.
  10⁵ karma çizimin uzak kaydırması GPU'da 16,7 ms'nin altına indi (12,1 ms, §5); 10⁶'da inmedi, madde `[~]` kalır.
- **Bağlam belgesi:** ADR 1062 (ölçüm düzeni, `perf::stability`), 1063 (blok örnekleme), 1064 (çizgi düzeyleri; bu
  ADR'nin “önce”si), 0144 (blok yerleşimi ve parçaların renkleri), CLAUDE.md §6, §23.
- **Web:** ertelendi (ADR 0228). Ortak çekirdek (`kentos-style-core`) bir seçenek daha aldı (`View::block_proxy`,
  varsayılan kapalı); web örnekli batch kurmaz, batch'leri ve `fixtures/style/v1` değişmez. TODOS.md §0.2'ye yazıldı.
- **Şema:** `.kcad` değişmedi (182). Komut, ayar, pencere değişmedi (envanter etkilenmez).

## Bağlam

ADR 1064'ten sonra 10⁵ karma çizimin uzak kaydırması GPU'da p50 25,5 ms'ydi; kalan payın yarıdan çoğu blok katmanıydı
(17 ms). Ölçü: 1× örneklemede blok katmanı 7,1 ms; maliyet uzakta ~1 px'lik yerleşimlerin kırka yakın ayrı parçasının
dört örneğe karışması.

### Araştırma

- **MicroStation:** View Attributes'in [Fast Cells](https://DOCS.BENTLEY.COM/LiveContent/web/OpenPlant%20Modeler%20Help-v12/en/GUID-14DBA888-C271-55BB-D43C-3E946EEC90AA.html)
  seçeneği açıkken hücreler (MicroStation'ın blokları) “outline” olarak, yani çerçeveleriyle gösterilir ve görünümün
  güncellenmesi kısalır. Bütün görünüme uygulanan, kullanıcının açtığı bir ayardır; ölçeğe bakmaz. KentOS'ta karşılığı
  yalnız ekranda birkaç pikseli geçmeyen yerleşimlere uygulanır, açık ayar değildir (§2).
- **AutoCAD:** blok için uzakta vekil çizim belgelenmez; yazı için benzer kısayol
  [QTEXTMODE](https://help.autodesk.com/view/ACDLT/2024/ENU/?guid=GUID-95370B7F-B389-4026-94B7-7E869BF2AAB6)'dur
  (yazının yerine çerçevesi, “improving speed by displaying simplified objects”; ADR 1062). Eğriler için
  [VIEWRES](https://help.autodesk.com/cloudhelp/2021/ENU/AutoCAD-LT/files/GUID-77B1C617-E4BB-4D1E-823A-8E2B055B258E.htm)
  ekranın ayrıntısıdır (ADR 1063).
- **QGIS:** [hata 1186](https://issues.qgis.org/issues/1186): bir pikselin altına inen işaretlerin bir kısmı kayboluyor,
  bir kısmı tek piksel kalıyordu; düzeltme en küçük işaret boyuydu. Ders: uzakta küçülen sembol yok olmamalı — vekil
  her yerleşimi çizer (§2).
- **Apple GPU'da örnekleme:** [Metal 2, A11](https://developer.apple.com/videos/play/tech-talks/606/): karışım yalnız
  örnekleri farklı (kenar) piksellerde örnek başına çalışır; alt piksel parçalarda her piksel kenardır (ADR 1063 §1,
  1064 §1). Örnekleme yalnız uzak bloklarda düşürülemez: aynı geçişteki bir çizimin örnek sayısı boru hattına değil
  hedefe bağlıdır (wgpu [`MultisampleState`](https://docs.rs/wgpu/latest/wgpu/struct.MultisampleState.html): `count`
  hedefin örnek sayısıyla aynı olmalı; `mask` örnekleri seçer ama çözümlemede ortalamaya girer, mürekkep açılır).

## Karar

### 1. Önce ölçüldü: neden bloklar

ADR 1064 §1 ve §7: 10⁵ karma çizimin uzak kaydırmasında blok katmanı tek başına 16–18 ms'dir; 1× örneklemede 7,1 ms'ye
iner (aynı tamponlar). Bir Ada yerleşimi 10⁵ uzak görünümde (~0,22 px/m) ~1,3 × 0,9 px'tir ve içinde ~40 ayrı segment
(20 çizgi, çerçevenin 4 kenarı, 4 dairenin 4'er kirişi) ve 4 dolu üçgen vardır; parçalar birbirine bağlı tek yol olmadığı
için çizgi düzeyleri (ADR 1064 §2) onlara bir şey kazandırmaz. Maliyet, aynı 2 × 2 piksele düşen kırk parçanın her birinin
dört örneğe karışmasıdır.

İki yol tartıldı:

- **Uzak bloklarda örneklemeyi düşürmek** (ayrı 1× geçiş ya da örnek maskesi): bir geçişteki çizimin örnek sayısı
  hedefinkidir; maske örnekleri seçer ama çözümleme dört örneği ortalar, yalnız birine yazılan mürekkep dörtte bire
  açılır. Ayrı bir 1× geçiş ise katmanların çizim sırasını böler (bloktan sonra çizilen katmanlar bloğun üstüne
  4× örneklemeyle gelmelidir; çözülmüş resmin üstüne yeniden 4× çizim örnekleri kaybeder). Uygulanmadı.
- **Uzak blok vekili:** her tanım bir kez kurulduğu için (ADR 1063 §2) vekil de tanımla bir kez kurulur, bütün
  yerleşimler aynı vekili kullanır. Seçildi; ölçüm §5'te.

### 2. Uzak blok vekili

`View::block_proxy` (yalnız çizim alanı; kâğıt, pafta, PDF, çizim çıktısı, lejant, önizleme kapalı;
`plot_precision`'da kapalı) açıkken tanımı kaydeden kuyu (`BatchSink::set_block_proxy`) tanımın her **çizgi**
batch'ine, en kaba düzeyinde en az 12 segment çiziyorsa (`PROXY_LEAST_SEGMENTS`), bir vekil ekler: o batch'in
çizgilerini saran kutunun dört kenarı (tanımın taban noktasından; tek noktaya inen kutu tek bir nokta), kutunun büyük
kenarıyla (`extent`, tanım birimi) birlikte. Sayılar batch'in düzeylerinden sonra, yerleşimlerden önce gelir
(`"inst":{…,"proxy":[uzunluk, extent]}`); GPU vekili kendi köşe aralığı olarak tutar.

Kare, görünümün mantıksal px/m'si × en büyük yerleşimin ölçeği × `extent` en çok `BLOCK_PROXY_PX` = 3 px ise o
batch'in tanımını değil vekilini çizer (`Instances::proxied`; büyüteç kendi ölçeğiyle). Böylece:

- **Hiçbir yerleşim kaybolmaz:** her yerleşim yine çizilir, yalnız tanımının çizgileri yerine kutusu.
- **Renk:** vekil, tanımın batch'inin kendisidir: aynı stil, aynı renk (ByBlock, ByLayer ya da parçanın kendi rengi
  kuruluşta çözülmüştür, ADR 0144). Tanımın her renkli çizgi batch'i kendi kutusunu çizer.
- **Dolgular ve az çizgili batch'ler** eskisi gibi çizilir: dolgunun kutusu (ör. Kapak'ın 0,15 m'lik üçgenleri yerine
  bütün Ada'yı kaplayan bir dikdörtgen) mürekkebi büyütürdü; birkaç çizgide kazanç yoktur.
- **Eşik:** en büyük yerleşimin ölçeğine göre seçildiği için bir batch'in bütün yerleşimleri birlikte geçer; o anda en
  küçük yerleşimler 3 px'ten de küçüktür. 3 px'te bir yerleşimin bütün parçaları kenar yumuşatmasıyla aynı ~3 × 3
  piksele karışır; kutunun dört kenarı da aynı lekeyi çizer (§4'te ölçüldü).
- **Kaynak, kenet, seçim, kâğıt** değişmez: vekil yalnız ekranın örnekli batch'indedir; örnekleme (ADR 1063) zaten
  yalnız çizim alanındadır ve kâğıt `kentos-plot`'ta `kentos-render-wgpu`'ya bağımlı değildir.
- **Ayar yok:** sahibin açıp kapatması gerekecek bir durum yok (yakında hiçbir şey değişmez, uzakta 3 px'lik bir
  lekenin içi değişir); ölçüm için `Styles::block_proxy` ve `KENTOS_PERF_SHORTCUTS=adr1064` kapatır.

### 3. Bellek ve kurulum

Vekil tanım batch'i başına 24 sayıdır (10⁵ blok katmanının görünüm tamponları 3,049 → 3,051 MB). İlk kare: §5.

### 4. Kararlılık ve eşitlik

`perf::stability` (10⁴, gerçek GPU, [ham veri](../perf/stability-desktop-adr1065-2026-11-06.json)) yeni satır grubu
`blok vekili`: uygulamanın kendi çizimi vekil açık/kapalı, aynı kamerada. 1 px/m'den yakın her görünümde görünür fark
0 olmalıdır (test doğrular). Görünür / farklı / en büyük kanal farkı (364 518 piksel):

| Görünüm (px/m) | Blok vekili |
|---|---:|
| Uzak (0,70; Ada ~6 px, vekil çizilmez) | 0 / 0 / 0 |
| 1:200 (18,9) | 0 / 0 / 0 |
| 1:20 (189) | 0 / 0 / 0 |
| Birinci tavanın altı ve üstü (4) | 0 / 0 / 0 |
| En uzak düzey (0,125; Ada ~1,1 px) | 0 / 1 487 / 49 |

Yakın görünümler piksel piksel aynıdır (farklı piksel de 0). Kaydırma, örnekleme, eğri ve çizgi düzeyleri satırları ve
kenet (üç görünümde bite kadar aynı) ADR 1064'tekiyle aynı kaldı.

Uygulamanın kendi resimleri (10⁵, `large_scenes.rs`'in yeni `buyuk-100k-uzak-vekilsiz`, `buyuk-100k-yakin`,
`buyuk-100k-yakin-vekilsiz` sahneleri; karşılaştırma yalnız çizim alanında, komut geçmişinin saati dışarıda): uzak
1440×900 0 görünür / 18 685 farklı piksel (en büyük fark 58/255), açık temada 0 / 16 939; yakın 1:200 iki temada da
0 / 0. Fark yeşil blok noktalarının içindedir; okunan yazılar ve çizgiler değişmez.

### 5. Ölçüm (Apple M5, release)

Ortam: Apple M5 (10 çekirdek), 32 GB, macOS 26.6.2 (25G83), GPU Apple M5 (Metal), rustc 1.96.0, `--release` (lto
thin), 1440×900, 4× örnekleme. “Önce” bu dalın vekilsiz ikilisi (`main` `a97b7eea2` + ADR 1064'ün dalı), “sonra”
vekilli ikili; aynı sabit iz arka arkaya, ortak ağır iş kilidi tutularak. Bütün tablolar:
[docs/perf/large-desktop-adr1065-2026-11-06.md](../perf/large-desktop-adr1065-2026-11-06.md).

**Katman katman, 10⁵, uzak kaydırma** (`perf::large::layers`, aynı ikili, `KENTOS_PERF_SHORTCUTS=adr1064` → uygulamanın
kendisi):

| Katman (nesne) | GPU p50 | Görünümün tamponları | İlk kare MİB |
|---|---:|---:|---:|
| Çizgi (50 000) | 7,5 → 6,3 ms | 48,9 → 48,9 MB | 59,4 → 53,3 ms |
| Yazı (20 000) | 2,2 → 2,2 ms | 0,8 → 0,8 MB | 12,0 → 11,9 ms |
| Daire/yay (10 000) | 3,6 → 3,6 ms | 36,9 → 36,9 MB | 31,0 → 31,4 ms |
| **Blok (10 000)** | **16,4 → 3,6 ms** | 3,049 → 3,051 MB | 106,2 → 117,5 ms |
| Tarama (5 000) | 3,6 → 5,0 ms | 5,9 → 5,9 MB | 48,6 → 49,3 ms |
| Ölçü (5 000) | 4,5 → 4,6 ms | 2,0 → 2,0 MB | 8,1 → 8,0 ms |
| Hepsi | 25,4 → 11,3 ms | 95,3 → 95,3 MB | 214,4 → 217,0 ms |

(Blok dışındaki satırlarda vekil yoktur; farkları aynı ikilinin iki koşusu arasındaki oynamadır.)

**Bütün iz** (ısınmış koşu, ms; önce → sonra):

| | 10 000 | 100 000 | 1 000 000 |
|---|---:|---:|---:|
| Uzak kaydırma GPU p50 | 8,4 → 7,6 | **25,1 → 12,1** | 287,2 → 120,6 |
| Uzak kaydırma GPU p50, ilk koşu | 7,5 → 7,5 | 25,2 → 12,1 | 284,4 → 119,4 |
| Uzak tekerlek GPU p50 / p95 sonra | 7,3 → 7,5 / 12,9 | 20,6 → 16,3 / 23,9 | 166,7 → 70,8 / 120,5 |
| Uzak tekerlek MİB p95 / bütçeyi aşan kare | 0,8 → 1,4 / 0 → 0 | 1,1 → 1,2 / 0 → 0 | 28,8 → 2,0 / 11 → 1 |
| Yakın 1:200 kaydırma GPU p50 | 7,7 → 7,4 | 7,0 → 7,6 | 11,1 → 11,5 |
| Açılış: ilk kare MİB / GPU | 26,1 / 16,1 → 47,0 / 19,3 | 228,4 / 35,6 → 238,5 / 34,7 | 2 266 / 253 → 2 214 / 250 |
| Hepsini seç MİB | 14,7 → 16,9 | 164,1 → 175,3 | 2 018 → 2 094 |
| Görünümün GPU tamponları (MB) | 24 → 24 | 109 → 109 | 959 → 959 |
| Süreç tepe ayak izi (MB) | 1 013 → 1 011 | 2 174 → 2 058 | 14 336 → 14 336 |

Tuş yankısı en kötü 12,4–14,0 ms, iptal Esc karesi 3,2–5,5 ms: hedefte kaldı. 10⁴'te vekil çizilmez (Ada uzakta
~6 px); o sütunun farkları oynamadır (imleç karelerinin GPU'su için ADR 1064 §5).

**Hedefe göre:** 10⁵ karma çizimin uzak kaydırması GPU p50 **12,1 ms: 16,7 ms tuttu** (ilk ve ısınmış koşuda, p95 13,5
ms); uzak tekerlek GPU p50 16,3 ms (p95 23,9 ms). 10⁶'da uzak kaydırma 120,6 ms, tekerlek 70,8 ms: **10⁶'da 60 Hz
tutmuyor.**

### 6. Sınırlar

- 10⁶ uzak görünüm hâlâ 120 ms (GPU): kalan pay o çizimde artık bloklar değil, tek tek bütün katmanlardır; ayrıca
  ölçülmedi.
- Vekil, en büyük yerleşimin ölçeğiyle batch'in bütün yerleşimleri için birlikte seçilir; yerleşim başına seçim
  (gölgelendiricide) yapılmadı.
- 3 px'lik eşik bir karardır: 10⁵ çizimde Ada'nın en büyüğü ~2 px'tir. Daha büyük bir eşik daha çok kazandırır ama
  görünür blok çizimini kutuya çevirir; sahibinin kararıdır.
- Dolgu batch'leri ve 12'den az segmentli batch'ler vekilsizdir.
- Web çizicisi (ADR 0228) `View::block_proxy`'yi bilmez ve örnekli batch kurmaz; ortak çekirdekte kapalıdır.

## Sonuçlar

- 10⁵ karma çizimde uzak kaydırma GPU p50 25,1 → 12,1 ms (ADR 1062'nin başlangıcı 221 ms): PERF-11 ve PERF-15'in
  kabulü (≤ 16,7 ms) tuttu. Blok katmanı 16,4 → 3,6 ms.
- Yakın görünümler piksel piksel aynı (test doğrular); kenet ve seçim değişmedi; bellek ve ilk kare değişmedi.
- CAD-ADV-19 `[~]` kalır: 10⁶'da uzak görünüm 60 Hz'de değil, 10⁶'da açılışın ilk karesi ve Hepsini seç ~2 s
  (PERF-13).
