# ADR 1067: Kaydırırken tutulan resmi kaydır, yalnız açılan şeridi çiz

- **Durum:** kabul edildi, kısmen (6 Kasım 2026). Kapsam: plan/cad-advanced.md `CAD-ADV-19`. 10⁶ karma çizimde uzak
  kaydırma GPU p50 110,7 → 8,9 ms (16,7 ms tuttu; 10⁵ 10,8 → 2,8 ms); uzak tekerlek bu dilimin kapsamı dışında kaldı (ölçek değişir, resim taşınamaz) ve 60 Hz'de
  değil (§5).
- **Bağlam belgesi:** ADR 1062 (ölçüm düzeni, `perf::stability`), 0023 (görünümün kendi hedefleri, tutulan resim), 0597
  (görünüm hareket ederken son yerleşim), 1066 (bu ADR'nin “önce”si), CLAUDE.md §6.
- **Web:** ertelendi (ADR 0228); değişiklik yalnız masaüstü çizicisinde (`kentos-render-wgpu`), paylaşılan
  gölgelendiriciler ve ortak çekirdek değişmedi.
- **Şema:** `.kcad` değişmedi. Komut, ayar, pencere değişmedi.

## Bağlam

ADR 1066'dan sonra 10⁶ karma çizimin uzak kaydırması GPU'da p50 108 ms'ydi (çizgi katmanı tek başına 51 ms: 400 000
ayrı düz çizgi). Masaüstünün çizim alanı resmini zaten tutar (ADR 0023): görünen hiçbir şey değişmezse kare resmi yeniden
çizmez. Kaydırmada ise kameranın yeri değiştiği için her kare bütün sahneyi yeniden çiziyordu; oysa yeni resmin
kaydırma kadar ötelenmiş bölümü bir önceki resimde zaten vardır.

### Araştırma

- **AutoCAD / BricsCAD:** [REDRAW](https://help.autodesk.com/cloudhelp/2015/ENU/AutoCAD-LT/files/GUID-6AD77E11-D549-47E7-AE4C-EFE372A0F0C4.htm)
  ekranı tazeler, REGEN çizimi veritabanından yeniden üretir; BricsCAD'in belgesi farkı “redraw yalnız ekranı yeniden
  boyar, ekranı çizim veritabanıyla eşlemeye çalışmaz” diye anlatır
  ([Redrawing and regenerating](https://help.bricsys.com/en-us/document/bricscad/viewing-your-drawing/redrawing-and-regenerating-a-drawing)).
  KentOS'ta karşılığı iki katmandır: stilli parçalar (REGEN'in işi) kaydırmada zaten kurulmaz; bu ADR kaydırmada
  ekranın yeniden boyanmasını da açılan şeride indirir.
- **QGIS:** harita tuvali (`QgsMapCanvas`) kaydırma sürerken var olan resmi taşır ve kaydırma bitince yeniden çizer:
  `panAction` fare hareket ederken çağrılır, `panActionEnd` “kaydırmayı bitirir ve tuvali yeniden çizer”
  ([PyQGIS 3.40, QgsMapCanvas](https://qgis.org/pyqgis/3.40/gui/QgsMapCanvas.html)); katmanların resimleri isteğe bağlı
  önbelleğe alınır (`setCachingEnabled`). KentOS'un farkı: açılan şerit kaydırma sürerken de hemen çizilir, kenar
  boş kalmaz.
- **wgpu / WebGPU:** dokudan dokuya kopya (`copy_texture_to_texture`) kaynağın `COPY_SRC`, hedefin `COPY_DST` kullanımı
  ister ([WebGPU, GPUTextureUsage](https://www.w3.org/TR/webgpu/#typedefdef-gputextureusageflags)); aynı dokunun örtüşen
  bölgesine kopya olmadığından ötelenen bölüm ayrı bir dokuda (`scratch`) bekletilir. Çok örnekli geçişin çözümlemesi
  (resolve) kesme dikdörtgeninden bağımsız olarak bütün eki yazar; bu yüzden şerit çizildikten sonra ötelenen bölüm
  resmin üstüne geri kopyalanır (§2).

## Karar

### 1. Ne zaman taşınır

Görünüm kaydırılırken (`FrameInput::moving`: masaüstünde `Viewport::moving`, son kaydırma ya da tekerlekten sonraki
150 ms) bir kare, tutulan resmin yerine şunların hepsi doğruysa onu taşır:

- resim görünüm kaydırılırken tutulmuştur ve gösterdiği her şey kameranın yeri dışında aynıdır: düz parçaların
  kimlikleri, vurgu kalınlığı, zemin, çerçevenin merkezi dışındaki her sayısı; stilli katmanların kimlikleri, çizim
  sırası, resimlerin atlas yerleri (`ViewStyled::shift_key`);
- ölçek, boyut, başlangıç (origin) bite kadar aynı, görünüm döndürülmemiş, harita başka sistemde değil;
- kameranın kayması **tam resim pikselidir** (binde bir pikselden yakın) ve resmin içindedir;
- sahnede ekrana bağlı ya da kameraya göre üretilen bir şey yoktur: raster ve harita servisi karoları, nokta bulutu,
  efekt grupları (birleştirilerek çizilir) ve ekran biriminde tarama, desen ya da karo dolgu (deseni ekrana bağlıdır).

Değilse kare bugünkü gibi bütün resmi çizer. **Kesirli kayma** (ör. klavyeyle ya da komutla dünya uzunluğu kadar kaydırma,
kesirli ekran ölçeği) taşınmaz, bütün çizilir: resim hiçbir zaman yeniden örneklenmez, bulanıklık yoktur.

### 2. Nasıl taşınır

`Renderer::render_moved`: tutulan resmin görünümde kalan bölümü ayrı dokuya (`Targets::scratch`) yeni yerine kopyalanır;
çok örnekli geçiş açılan şeritleri (sol/sağ sütun, üst/alt satır) kesme dikdörtgeniyle çizer; her şerit için yalnız
şeride (çevresinde 4 piksel, kenar yumuşatması için) erişen stilli batch'ler çizilir (`StyledGpu::draw_strip`,
büyütecin kutu süzgeciyle aynı yol); geçiş resmin tamamını çözümler; sonra ayrı dokudaki bölüm resmin üstüne geri
kopyalanır. Şeritlerin pikselleri resmin bütün çizilseydi orada olacak pikselleridir; ötelenen bölüm bir önceki karenin
pikselleridir, tam piksel kaydırılmış (yeniden örnekleme yok). Görünüm durunca (`moving` yanlış) resmin anahtarı değişir
ve kare resmi bütün çizer: duran kare bugünküyle aynıdır.

Taşınan bir resim bir sonraki taşımanın da kaynağıdır; kaydırma sürdükçe ötelenen bölüm hep bir önceki karedendir.

### 3. Bellek ve ilk kare

Görünüm başına bir resim boyunda doku daha (`scratch`): ölçümün penceresinde (1440×900, çizim alanı ~1 128×660 cihaz
pikseli) ~3 MB (görünümün tamponları 10⁶'da 959 → 962 MB); tam ekran bir HiDPI çizim alanında (2 880×1 800) 20,7 MB.
Hedeflerin sayılan baytına eklendi. İlk kareye ve açılışa iş eklenmedi (doku hedeflerle birlikte kurulur).

### 4. Eşitlik ve kararlılık

`perf::stability` (10⁴, gerçek GPU, [ham veri](../perf/stability-desktop-adr1067-2026-11-06.json)) üç görünümde (uzak
0,70 px/m, 1:200, 1:20) yeni bir satır grubu aldı; görünüm 37/23 px tam piksel kaydırılır:

| Görünüm | Resim taşındı | Taşınan / bütün çizilen (kaydırırken): görünür / farklı / en büyük | Durunca / bugünkü: farklı |
|---|---|---:|---:|
| Uzak (0,70) | evet | 0 / 634 / 2 | **0** |
| 1:200 (18,9) | evet | 0 / 1 912 / 1 | **0** |
| 1:20 (189) | evet | 0 / 1 963 / 1 | **0** |

- **Duran kare bugünküdür** (test doğrular): kaydırma bitince kare resmi bütün çizer (`settledDrawnWhole`) ve aynı
  görünümden aynı kaydırmanın bugünkü yoldan (taşımadan) çizilen karesiyle piksel piksel aynıdır (farklı piksel 0).
  Karşılaştırmada iki yolun öncesi aynıdır: ızgaranın parçası (`Viewport::grid`, görünümün çevresi için kurulur) ve
  yazıların yerleşimi önceki görünüme göre oluşur; öncesi farklı iki bugünkü kare de birbirinden 1–17/255 ayrılır.
- **Kaydırırken** ötelenen bölüm bir önceki karenin pikselleridir, tam piksel taşınmış: aynı anda bütün çizilmiş
  görünümden karşılaştırılan 364 518 pikselin %0,2–0,5'inde 1–2/255 ayrılır, görünür fark yoktur (test doğrular). Bu,
  ADR 1062 §5'in kaydırma satırındaki farkın aynısıdır (f32 konumların kayma sonrası yuvarlanması): bütün çizilen bir
  resim de kendi kaydırılmış hâliyle tam aynı değildir. Açılan şeritler bütün çizimin pikselleridir.
- ADR 1066'daki 30 satır ve kenetler sayı sayı aynı; kenet ve seçim çizimden bağımsızdır (kaynak geometri).
- Uygulamanın kendi resimleri (10⁴ uzak, 1:200, 1:20; 10⁵ uzak, 1:200; iki tema; duran görünüm) iki ikilide çizim
  alanında piksel piksel aynı.
- Fark resimleri: `.run/gui-evidence/cad-adv-19/kaydirirken-*` (taşınan / bütün çizilen), `durunca-*` (duran / bugünkü).

### 5. Ölçüm (Apple M5, release)

Ortam: Apple M5 (10 çekirdek), 32 GB, macOS 26.6.2, GPU Apple M5 (Metal), rustc 1.96.0, `--release` (lto thin,
sembollü), 1440×900, 4× örnekleme. “Önce” `origin/main` `483a4cc09`, “sonra” bu dal; aynı sabit iz (orta tuşla 40 adım,
her adım 9/5 px: iki şerit), arka arkaya, ortak ağır iş kilidi tutularak. Bütün tablolar:
[docs/perf/large-desktop-adr1067-2026-11-06.md](../perf/large-desktop-adr1067-2026-11-06.md).

| (ms; ısınmış koşu, önce → sonra) | 10 000 | 100 000 | 1 000 000 |
|---|---:|---:|---:|
| Uzak kaydırma GPU p50 | 7,4 → 3,0 | 10,8 → 2,8 | **110,7 → 8,9** |
| Uzak kaydırma GPU p50, ilk koşu | 7,3 → 4,0 | 10,7 → 2,9 | 110,3 → 8,8 |
| Uzak kaydırma GPU p95 sonra | 4,6 | 4,4 | 11,5 |
| Yakın 1:200 kaydırma GPU p50 | 6,4 → 6,3 | 7,5 → 7,2 | 11,4 → 9,3 |
| Uzak tekerlek GPU p50 | 7,1 → 7,4 | 16,9 → 15,3 | 66,4 → 65,0 |
| Açılış: ilk kare MİB | 22,1 → 44,2 | 179,3 → 176,6 | 1 523 → 1 446 |
| Görünümün tamponları (MB) | 24 → 27 | 109 → 112 | 959 → 962 |

Kaydırmanın MİB'i değişmedi (p95 ≤ 1,6 ms). 10⁴'ün ilk kare farkı koşudan koşuya oynamadır (ADR 1066 §5).

### 6. Sınırlar

- Kaydırma sürerken ötelenen bölüm bir önceki karenin pikselleridir: aynı görünümün o an bütün çizilmiş hâlinden
  kenar yumuşatmasında en çok 2/255 ayrılır (§4); görünür fark yok, durunca resim bütün çizilir.
- Tekerlek (ölçek değişir), döndürülmüş görünüm, raster/servis karoları, nokta bulutu, efekt grupları ve ekrana bağlı
  desenler eskisi gibi her karede bütün çizilir. 10⁶ uzak tekerlek GPU'su 60 Hz'de değil.
- Şeritlerde düz parçalar (ızgara, vurgu) süzgeçsiz çizilir (kesme dikdörtgeni pikselleri sınırlar); büyük bir seçimin
  vurgusu kaydırmada bu yüzden pahalı kalabilir (ölçülmedi).
- Büyük seçimin eşikleri (PROPOBJLIMIT benzeri) ve vurgunun arka planda kurulması sahibinin kararı olarak açık (ADR 1066 §7).

## Sonuçlar

- Uzak kaydırmanın GPU'su 10⁶ karma çizimde 110,7 → 8,9 ms, 10⁵'te 10,8 → 2,8 ms: 60 Hz bütçesinde; duran kare bugünkü
  kareyle piksel piksel aynı.
- Kaydırma sürerken resim bir önceki karenin pikselleri tam piksel taşınmış hâlidir (bütün çizimden en çok 2/255).
- 10⁶'da uzak tekerlek (65 ms), Hepsini seç (1,15 s) ve açılışın ilk karesi (1,4 s) 60 Hz'de değil; CAD-ADV-19 `[~]`.
