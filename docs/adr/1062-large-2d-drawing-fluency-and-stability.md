# ADR 1062: Büyük 2B çizimde akıcılığı ve görsel kararlılığı ölçerek koru

- **Durum:** kabul edildi, kısmen (6 Kasım 2026). Kapsam: plan/cad-advanced.md `CAD-ADV-19`. Ölçüm düzeni, veri
  setleri, kararlılık ölçütleri ve uzak görünümde yazıların kaydırmada sıçramasının düzeltilmesi bu dilimdedir.
  Uzak görünümün GPU bütçesi (100 000 ve 1 000 000 nesne) **tutmadı**; sınır ve nedeni §3'te ölçülerek yazıldı,
  madde tamamlandı sayılmaz.
- **Bağlam belgesi:** ADR 0005 (taslak hedefler), 0120/0121/0742 (kare, parça önbelleği, tek düzenleme), 0205 §5 ve
  0626 §6 (Kaybolmasın yazı boyu), 0930 §22–23 (karmaşık çizgi tipi, 2 000 000 kontur noktası), CLAUDE.md §6, §19.
- **Web:** ertelendi (ADR 0228). §4'teki düzeltme ortak çekirdektedir (`kentos-geometry-core`); web'in WASM'ı da aynı
  kodu kullanır, web'de ayrıca ölçülmedi.
- **Şema:** `.kcad` değişmedi (182).

## Bağlam

Mevcut temeller ölçüldü, yeniden yazılmadı: wgpu çizici, stilli katmanların 4096 yerlik parçaları (ADR 0121),
uzamsal dizin, f32 hi/lo konum ayrımı ve düz sahnenin eğri bandı (`lod::build_band`). Masaüstünde `perf::frame`
50 000–500 000 parselin tek tür (20 köşeli alan) karesini ölçüyordu; karma çizim, yazı girişi, iptal, GPU belleği ve
görsel kararlılık ölçülmüyordu. Ölçüm makinesi Linux i5'ten Apple M5'e geçti (`/proc` yok).

QCAD'in kaynağında blok başına uzamsal dizin ve işlem sonucuna bağlı güncelleme vardır (plan [R21]); KentOS'ta karşılığı
parça önbelleği ve değişiklik işaretidir (`changes_since`). Autodesk'in AutoCAD yardımı ekrandaki yükü görüntüleme
ayarlarıyla sınırlar: [QTEXTMODE](https://help.autodesk.com/view/ACDLT/2024/ENU/?guid=GUID-95370B7F-B389-4026-94B7-7E869BF2AAB6)
yazının yerine çerçevesini gösterir (“improving speed by displaying simplified objects”),
[HPMAXLINES](https://help.autodesk.com/view/CIV3D/2026/ENU/?guid=GUID-0DE22398-D77D-40C6-83CA-06A24581539A) bir
taramanın üretilen çizgi sayısını sınırlar (100–10 000 000; ilk değer sürüme göre 100 000 ya da 1 000 000); ikisi de
çizim verisini değiştirmez. KentOS aynı ayrımı korur: ekran sadeleştirmesi kaynak geometriyi, kenetlemeyi ve kâğıdı
etkilemez (§5, §6). Bu dilimde böyle bir ekran sadeleştirmesi eklenmedi.

## Karar

### 1. Sabit, betikle üretilen karma veri seti

`apps/desktop/src/perf/mixed.rs` çizimi tohumdan (19) üretir; repo'ya ikili konmaz, dosya `.run/perf/` altına
uygulamanın kendi kaydıyla (`saving::write_shared`) yazılır. 25 m'lik her hücrede on nesne: dört çizgi, iki yazı, bir
daire ya da yay, üç kat iç içe bir blok yerleşimi (Ada → 2 Grup → 2 Kapak; yerleşim başına 31 parça, aynalı ve döndürülmüş
olanlar dahil), yoğun tarama (0,2 m çapraz ya da kesikli iki aileli desen, delikli) ya da ölçü, kısa çoklu çizgi; her
onuncu hücrenin çoklu çizgisi 400 köşeli uzun çizgidir. Altı katman, her biri tek tür. Parmak izi (FNV-1a, üretilen her
sayı ve metin): 10 000 `7f200b185434b8be`, 100 000 `a0142ee95cc85d46`, 1 000 000 `c48958125ed2220e`. Aynı tohum aynı
çizimi verir (`perf::mixed` birim testi).

### 2. Ölçüm düzeni ve ayrı raporlanan ölçütler

`perf::large::large` her boyutu kendi sürecinde çalıştırır (tepe bellek o sürecindir) ve sabit izi sürer:

- **Açılış:** uygulamanın kendi açılışı (`opening.rs`, ayrı iş parçacığı): iş parçacığının süresi, çizimi ekrana koyan
  kare (MİB ve GPU), toplam; açılış sürerken 16 ms'de bir çizilen karelerin MİB'i.
- **İlk etkileşim:** açılıştan sonraki ilk imleç, tekerlek ve kaydırma karesi.
- **Kare:** imleç yolu (Lissajous), tekerlek (2 × 8 içeri ve dışarı, duruştan sonra 200 ms), orta tuşla 40 adımlık
  kaydırma; uzak (bütün çizim) ve yakın (1:200, orta hücre); iki kez (ilk, ısınmış). MİB p50/p95/p99 ve 16,7 ms'yi
  aşan kare sayısı; GPU ayrı (çizim + geri okuma). Boş pencerenin aynı yolla çizilen karesi geri okumanın tabanı olarak
  yanında raporlanır (M5'te p50 ≈ 2,2–2,4 ms).
- **Yazı girişi:** komut satırına 14 tuş, çizgi çizerken değer alanına 9 tuş; her tuşun karesi (yerel yankı = MİB + GPU).
- **Komut:** ajan bağlantısından (Python/MCP'nin yolu) bir yazıyı taşı, bir yazının metnini değiştir, bir blok
  yerleşimini taşı, bir çizgiyi sil, geri al, yinele, hepsini seç; her birinde yeniden kurulan stilli parça / bütün
  parçalar. Bağlantının ilk komutu (katalog kurulumu) ayrı satırdır.
- **İptal:** ikinci açılış, iş parçacığı okurken Esc: Esc'nin karesi (pencerenin o karede kapanması dahil) ve iş
  parçacığının durması; hiçbir şeyin uygulanmadığı da denetlenir.
- **Bellek:** `ps` yerleşik bellek, `footprint` fiziksel ayak izi ve sürecin tepesi, GPU belleği (IOAccelerator) ve
  görünümün kendi GPU tamponları (`FrameStats::resident_bytes + target_bytes`).

Ortam bilgisi macOS'ta `sysctl`/`sw_vers`'ten alınır (`perf::environment`); GPU adı ve sürücüsü wgpu'nun
bağdaştırıcısından (aygıt açılmadan). Rapor JSON'dur; tabloları `scripts/perf/large_report.py` üretir.

### 3. Uzak görünümün GPU bütçesi tutmadı: payı iç içe blokların eğrileri alıyor

Ölçüm (§7) yakın görünümün, yazı girişinin, iptalin ve tek düzenlemenin hedefte olduğunu; uzak görünümde (bütün çizim
ekranda) kaydırma ve tekerleğin GPU'da 100 000 nesnede ~0,2 s, 1 000 000'de ~1,8 s sürdüğünü gösterdi. Görünümün GPU
tamponları 1 000 000 nesnede ~2,07 GB'tır; her uzak kare hepsini çizer.

`perf::large::layers` (100 000 nesne, her katman tek başına, aynı uzak kaydırma, GPU p50): çizgi 23,5 ms (24 MB),
yazı 2,5 ms (0,8 MB), daire/yay 20,7 ms (26 MB), **blok 165,5 ms (152 MB)**, tarama 3,7 ms (5,9 MB), ölçü 3,3 ms
(2,0 MB); hepsi 212,8 ms (207 MB). Bloklardan parça türü çıkarılarak (`KENTOS_MIXED_BLOCK_LEAVES_OUT`, yalnız tanı):
daireler çıkınca 152 → 14 MB ve 167,8 → 10,6 ms; yazı, tarama ya da çizgi çıkınca değişim ≤ 13 ms. Neden kodda:
stilli katmanın geometrisi (`geometry-core/store/draw.rs`) daireyi, yayı ve elipsi ekran boyundan bağımsız **72
kirişle** çizer (`OUTLINE_SEGMENTS`) ve her blok yerleşimi parçalarını ayrı ayrı açar; 10 000 yerleşimdeki 40 000
küçük daire 2,9 milyon kiriştir, uzak görünümde her biri bir pikselin altındadır.

Bu dilimde değiştirilmedi, gerekçe: kiriş sayısını boya bağlamak yakın görünümde küçük daireleri köşeli gösterir (1:20'de
0,3 m'lik daire 57 px yarıçap; 8 kiriş ≈ 4 px sapma) ya da her bant değişiminde bütün parçaların yeniden kurulmasını
ister (1 000 000 nesnede ilk kurulum MİB'de ~1 s); `draw.rs` vurgu katmanlarının ve web'in ortak geometrisidir. Doğru
çözüm iki ayrı iştir: (a) blok tanımının geometrisini bir kez kurup yerleşimleri dönüşümüyle çizmek (instancing; ADR
0930 §23'teki tekrar eden yazı üçgenlemesiyle aynı ilke), (b) stilli eğrilerin düz sahnedeki gibi bantla kurulması.
İkisi de ölçülmeden hız iddiası yoktur; önerilen plan satırları raporda.

### 4. Uzak görünümde kaydırma yazıları değiştiriyordu: hücreler dünyaya bağlandı

Kararlılık ölçümü (§5) uzak görünümde 37/23 px'lik kaydırmada 22 318 pikselin görünür biçimde (bir kanal > 64)
değiştiğini buldu. Fark yalnız yazılardaydı: Kaybolmasın (ADR 0205 §5) küçük yazıyı okunur boya büyütür ve
büyütülmüş yazılar 8 px'lik hücrelerde birbirini örterse sonrakini çıkarır; hücre ızgarası görünümün köşesine bağlıydı,
8'in katı olmayan her kaydırma hücre sınırlarını dünyaya göre kaydırıp başka yazıların kalmasına yol açıyordu.

`keep_apart` (`geometry-core/store/legible.rs`) artık hücreleri dünyanın başlangıcından `8 px / ölçek` aralıkla
sayar; görünüm yalnız hangi hücrelerin okunacağını seçer. Aynı yazı her kaydırmada aynı hücrelere düşer. Kalan tek
etki: görünümün kenarının kestiği bir yazının girip çıkması kenarın yakınında kimin kalacağını değiştirebilir (ölçüldü,
§5). Büyük koordinatta `x · ölçek` f64'tür (486 000 m × 10 000 px/m ≈ 4,9·10⁹, tam). Gerçek boy ve Ekranda sabit
kurallarına, tablo hücrelerine, kâğıda ve seçime dokunmaz (büyütme yalnız ekranın gösterimidir). Katman etiketlerinin
(CBS) seyreltme ızgarası ayrı koddur, bu dilimde ölçülmedi.

Yeni çekirdek testi `a_pan_keeps_the_same_grown_texts_in_the_middle`: 1200 küçük yazı, hepsi her görünümün içinde (kenar
etkisi yok), 37/−23, 3/5, −13/1, 7,5/0,25 px kaydırma: kalan yazılar aynı olmalı. Eski kodla başarısız (35 kimlikten
farklı kümeler), yenisiyle geçer.

### 5. Görsel kararlılığın bağımsız ölçütü

`perf::stability` (10 000 nesne, gerçek GPU, uygulamanın kendi resmi `Renderer::screenshot`): çizim alanının ortası (her
kenardan %15 dışarıda; ekrana sabit katmanlar köşelerde), resim A'nın (x, y)'si resim B'nin (x + dx, y + dy)'siyle
karşılaştırılır. Ölçütler: farklı piksel, görünür fark (bir kanal > 64), en büyük kanal farkı, mürekkepli piksel
(alanın en sık renginden farklı). Kontroller: aynı görünüm yeniden (0 fark beklenir), 8 px'in katı kaydırma, eğri
bandının ve stilli çeyrek oktavın sınırında 10⁻⁷'lik yakınlaştırma (aynı bant içindeki eşit adım kontrol). Uzak dışındaki
görünümlerde ve sınır geçişlerinde görünür fark 0 olmalıdır (test bunu doğrular); uzak görünümün sayısı raporlanır.

| Ölçüt (10 000 nesne) | Önce | Sonra |
|---|---:|---:|
| Uzak, 37/23 px kaydırma: görünür / farklı piksel / en büyük fark | 22 318 / 59 468 / 199 | 0 / 1 082 / 2 |
| Uzak, 40/24 px (8'in katı) kaydırma | – | 0 / 1 026 / 3 |
| Yakın 1:200 ve milimetrik 1:20, 37/23 px kaydırma | 0 / ~1 717 / 1 | 0 / ~1 717 / 1 |
| Eğri bandı sınırı (−8 → −9): geçiş / aynı bant içi | 0 / 30 / 1 ve 0 / 77 / 1 | aynı |
| Stilli çeyrek oktav sınırı (16 → 19,03): geçiş / içi | 0 / 505 / 1 ve 0 / 4 638 / 2 | aynı |
| Aynı görünüm yeniden | 0 | 0 |

Kalan farklı pikseller 1–3/255'lik kenar yumuşatma yuvarlamasıdır (karşılaştırılan 330 128 pikselin %0,3–0,5'i);
eşik büyütülmedi. Kaynak: `docs/perf/stability-desktop-once-2026-11-05.json`, `…-sonra-2026-11-05.json`.

**Kenet ve seçim:** aynı kaynak noktada (dairenin üstü, çizginin ucu, uzun çizginin 200. köşesi) her görünümde 10 px'lik
toleransla sorulan kenet uzak, 1:200 ve 1:20'de **bite kadar aynı** noktayı verdi; dairenin en yakın noktası gerçek
yarıçapın 1,4·10⁻¹⁰ m içinde (72 kirişin sapması milimetre düzeyindedir: kenet kirişten değil kaynaktan). Ekran
sadeleştirmesi kenet/seçim dizinine girmez.

### 6. Kâğıt ve ekran ayrı yollar

`kentos-plot` (PDF/pafta) `kentos-render-wgpu`'ya bağımlı değildir (`cargo tree -p kentos-plot -e normal`: 0 eşleşme):
kameranın bandı ya da ekran kirişi kâğıda geçemez; pafta eğrileri `kentos-sheet`'in kesin yay/Bezier adımlarıyla
yazılır (ADR 0930'ın PDF kanıtları). Bu dilimde yeni PDF ölçümü yapılmadı.

### 7. Ölçüm (Apple M5, release)

Ortam: Apple M5 (10 çekirdek), 32 GB, macOS 26.6.2 (25G83), GPU Apple M5 (Metal; wgpu sürücü adı vermiyor), rustc
1.96.0, `--release` (lto thin), commit `e6145d28c` + bu dilim, 1440×900, 4× örnekleme. Bütün tablolar ve ham veri:
[docs/perf/large-desktop-2026-11-06.md](../perf/large-desktop-2026-11-06.md). Önce: düzeltmeden önceki ikili (aynı
ölçüm kodu), sonra: §4'le; aynı iz arka arkaya. Ortak kilit (`/tmp/kentos-heavy.lock`) ölçüm boyunca tutuldu, bu ajanın
başka ağır işi yoktu; makinedeki başka ajanların kilitsiz işi dışlanamaz.

| Sonra (ısınmış, ms) | 10 000 | 100 000 | 1 000 000 |
|---|---:|---:|---:|
| Açılış: iş parçacığı / ilk kare MİB / ilk kareye toplam | 23,7 / 57,4 / 98,0 | 155,2 / 240,0 / 447,9 | 1 646 / 2 546 / 4 642 |
| Açılış sürerken karelerin MİB p95 | 2,8 | 0,4 | 0,4 |
| İlk etkileşim, imleç MİB / GPU | 21,2 / 55,5 | 25,4 / 395,2 | 17,6 / 7 285 |
| Yakın 1:200 kaydırma MİB p95 / GPU p95 | 0,7 / 8,1 | 0,8 / 10,1 | 0,9 / 19,2 |
| Uzak kaydırma MİB p95 önce → sonra / bütçeyi aşan | 0,7 → 1,2 / 0 | 22,5 → 8,0 / 8 → 1 | 19,9 → 11,3 / 14 → 0 |
| Uzak kaydırma GPU p50 (sonra) | 23,7 | 218,8 | 1 765 |
| Uzak tekerlek MİB p95 önce → sonra | 1,4 → 1,4 | 54,9 → 35,0 | 55,0 → 34,5 |
| Tuş yankısı en kötü (MİB + GPU) | 11,6 | 9,9 | 16,3 |
| Tek düzenleme MİB (yazı taşı) / kurulan parça | 1,2 / 1⁄6 | 0,7 / 1⁄102 | 21,0 / 1⁄1470 |
| Hepsini seç MİB | 18,0 | 172,3 | 2 256 |
| İptal: Esc karesi MİB + GPU / iş parçacığı durması | 4,0 / 1,6 | 4,8 / 1,7 | 5,1 / 1,1 |
| Görünümün GPU tamponları (MB) / süreç tepe ayak izi (MB) | 36 / 1 227 | 222 / 3 195 | 2 081 / 16 384 |

- Hedefe göre: 60 Hz yakın görünümde ve uzak imleçte tutuyor (resim saklanıyor); uzak kaydırma ve tekerlekte 10⁵ ve
  10⁶'da GPU tutmuyor (§3); uzak tekerlekte MİB p95 ~35 ms (çizim kaydı: ekrandaki büyütülmüş yazıların Iced'e
  verilmesi, `uiP95Parts`). Tuş yankısı p95 ≤ 50 ms ve iptal ≤ 100 ms hedefleri tutuyor (üç boyutta da).
- §4'ten sonra uzak kaydırmada MİB p95 düştü ve bütçeyi aşan kare kalmadı; nedeni ayrıca profillenmedi (kenar
  hücreleri artık her karede başka yazı bırakmıyor; yerleşim belleği daha çok tutuyor olabilir), sonuç yalnız ölçüdür.
- İlk komut (ajan bağlantısının ilki) 243–291 ms'dir (katalog kurulumu), sonrakiler yukarıdaki gibi.
- 1 000 000 nesnede tek düzenleme ~21 ms'nin ~20 ms'si değişen tek parçanın kurulmasıdır (4096 yer, blok yerleşimleri
  açılmış); bütçenin üstündedir, parça boyu ayrı bir ayardır (ADR 0121), bu dilimde değiştirilmedi.

### 8. ADR 0930'daki 2 000 000 kontur noktası sınırı

Release ölçümü (M5/Metal, `complex_linetype_timing`, 4 m çizgiler, 1:50, HW yazılı desen; `KENTOS_LINETYPE_LINES`
eklendi): 800 çizgi kaynak kurulumu 54,5 ms, geometri 82 214 400 bayt, ısınmış kare medyanı 3,1 ms (1×) / 3,2 ms (4×);
1000 çizgi kurulumda reddedildi (51,7 ms, `lineTypeLoss` söylendi, eksik desen çizilmedi). ADR 0930'daki debug
ölçümünün (1591 → 294 ms ilk kurulum) release karşılığıdır; ADR 0930'un 8 000 tekrarına göre tekrar başına ~10 KB GPU
belleği sürer.

Karar: sınır açık gerekçeli olarak korunur, instancing bu dilimde yapılmadı (§3a ile aynı iş: bir tanımı bir kez
kurup tekrarları dönüşümle çizmek). Gerekçe: sınır bir yapının (katman, ya da 8192 nesneden büyük katmanda 4096 yerlik
parça) GPU tamponunu ~95 MB'ta tutar (82 MB'ı 800 çizgi doldurur; ADR 0930'a göre sınır ~917 çizgide); aşımda desen sessizce kısaltılmaz, `lineTypeLoss`
ile açıkça söylenir. Sınırı büyütmek çözüm sayılmadı.

## Sonuçlar

- Ölçüm düzeni ve veri seti her sonraki maddede yeniden kullanılır; hedefler ADR 0005 gibi taslaktır.
- Uzak görünümde yazılar artık kaydırmada değişmez; tek düzenleme yalnız kendi parçasını kurar (1 000 000 nesnede 1/1470).
- Açık: uzak görünümün GPU bütçesi (§3), açılışın ilk karesinin MİB'de bütün parçaları kurması (1 000 000'de ~2,4 s),
  hepsini seçmenin 1 000 000'de ~2,2 s'si, 2 000 000 kontur noktası sınırı (§8).
