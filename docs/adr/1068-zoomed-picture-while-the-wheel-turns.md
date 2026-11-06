# ADR 1068: Tekerlek dönerken son keskin resmi ölçekle, durunca bütün çiz

- **Durum:** kabul edildi (6 Kasım 2026). Kapsam: plan/cad-advanced.md `CAD-ADV-19`. Sahibinin kararı: “Evet, büyüt”
  (tekerlek dönerken resim ölçeklenerek yeniden kullanılır, durunca keskin çizilir). 10⁶ karma çizimde uzak tekerlek
  GPU p50 65,0 → 7,0 ms; tekerlek izinde bütçeyi aşan arayüz karesi 2 → 0 (§5).
- **Bağlam belgesi:** ADR 1062 (ölçüm düzeni, `perf::stability`), 1067 (kaydırırken taşınan resim; bu ADR'nin
  şeritleri ve ayrı dokusu ondandır), 1064 §3 (tekerlek dönerken tutulan yazılar), 0023 (görünümün kendi hedefleri),
  CLAUDE.md §6.
- **Web:** ertelendi (ADR 0228); değişiklik masaüstü çizicisinde (`kentos-render-wgpu`) ve masaüstü çizim alanındadır.
  Paylaşılan gölgelendiriciler ve ortak çekirdek değişmedi.
- **Şema:** `.kcad` değişmedi. Komut, ayar ve pencere değişmedi.

## Bağlam

ADR 1067'den sonra 10⁶ karma çizimin uzak kaydırması 60 Hz'e indi. Uzak tekerlek ise GPU p50 65 ms'de kaldı: tekerlekte
ölçek değişir, tutulan resim tam piksel taşınamaz ve her kare bütün sahneyi yeniden çizer. 10⁶'da tekerlek izinin bir
karesi de arayüz iş parçacığında 22 ms sürüyordu (§5).

### Araştırma

- **QGIS:** tuval, tekerlekte son çizilen haritayı ölçekleyerek gösterir; gerçek çizim sonra başlar. Martin Dobias'ın
  yanıtı: “it zooms the most recently rendered map in/out”
  ([QGIS özellik isteği #20102](https://trac.qgis.org/issues/20102), 17 Ekim 2018). İsteği açan, ölçeklenen resmin
  kenarında tampon olmadığını not eder: uzaklaşırken açılan kenar boş kalır.
- **OpenLayers:** karo katmanı, yenileri gelene kadar önceden yüklenmiş düşük çözünürlüklü karoları gösterebilir.
  `preload` seçeneğinin tanımı: “Load low-resolution tiles up to `preload` levels”
  ([ol/layer/Tile](https://openlayers.org/en/latest/apidoc/module-ol_layer_Tile-TileLayer.html)). KentOS'un geniş resmi
  (§2) bunun tek düzeyli karşılığıdır: uzaklaşırken açılan halka, daha önce daha uzaktan çizilmiş resimden gelir.
- **AutoCAD / BricsCAD:** REDRAW ekranı tazeler, REGEN çizimi veritabanından yeniden üretir (ADR 1067'deki kaynaklar).
  Ölçeklenen resim REDRAW bile değildir, ekrandaki son resmin yeniden örneklenmesidir. Durunca KentOS bütün çizer
  (REDRAW'ın karşılığı); stilli parçalar ölçeğe göre zaten yeniden kurulur (REGEN'in karşılığı).
- **WebGPU:** bir dokunun örneklenmesi `TEXTURE_BINDING` kullanımı ve süzgeçli (`filterable`) örnek türü ister. Doğrusal
  süzgeç (`FilterMode::Linear`) komşu pikselleri karıştırır
  ([WebGPU, GPUSamplerDescriptor](https://www.w3.org/TR/webgpu/#dictdef-gpusamplerdescriptor)). Çok örnekli bir ekin
  bütün örnekleri parçanın rengini alır; çözümleme (resolve) aynı rengi geri verir.

## Karar

### 1. Ne zaman ölçeklenir

Görünüm hareket ederken (`FrameInput::moving`: son tekerlek ya da kaydırmadan sonraki 150 ms) bir kare resmi bütün
çizmez, son keskin resmi ölçekler. Bunun için şunların hepsi doğru olmalıdır:

- Resim ne tutulmuş ne taşınmıştır (önce ADR 0023 ve 1067 denenir).
- Son keskin resim aynı çizimi gösterir. Bunu parçalar söyleyemez, çünkü yeni ölçek için yeniden kurulurlar. Bu yüzden
  ev sahibi söyler: `FrameInput::drawing`. Masaüstünde bu, belgenin oturumu ve nesli (`generation`: düzenleme, geri
  alma, katman değişikliği, dışarıdan gelen değişiklik), seçimin sürümü, zemin, vurgu rengi ve kalınlığı, örnekleme,
  ızgara ve stilin ekranda sabit/kalınlık seçenekleridir. İmlecin altındaki nesne resmin üstüne ayrıca çizildiğinden
  anahtara girmez. Görünüm kipleri ve stil kitaplığı tekerlek dönerken değişmez, pencerelerde değişir.
- Ekrana bağlı ya da birleştirilerek çizilen bir şey yoktur (raster ve servis karoları, nokta bulutu, efekt grupları,
  ekran biriminde desen; ADR 1067 §1'in koşulları, `ViewStyled::zoom_key`).
- Başlangıç, boyut ve cihaz piksel oranı aynıdır, görünüm döndürülmemiştir (`Place::same_frame`).
- Yeni ölçek kaynağın ölçeğinin **dörtte biri ile dört katı** arasındadır (iki oktav; `zoomed::MIN_RATIO`,
  `MAX_RATIO`). Tekerleğin her çentiği e^0,15 ≈ 1,16 kattır, sınır 9 çentikte bir aşılır.

Kaynak hep **son keskin resimdir**, ölçeklenmiş bir resim değildir: bir hareketin bulanıklığı kareden kareye birikmez.
Önce en yeni keskin resim denenir (bütün çizilen ya da taşınan son resim), yoksa hareketin ayırdığı kaynak. Sınırı aşan
kare bütün çizilir ve yeni kaynak olur.

### 2. Nasıl ölçeklenir

`Renderer::render_zoomed` yeni bir pikselin merkezi `f`'yi kaynakta `f·a + b`'den örnekler: `a` kaynak pikseli / yeni
piksel, `b` kameraların merkezleri arasındaki farktan gelir. `zoomed::plan` bunu ve kaynağın kapladığı pikselleri
(`inner`) hesaplar. Tek bir tam resim üçgeni, kesme dikdörtgeni `inner` olarak, ayrı dokudaki (`Targets::scratch`)
kaynağı doğrusal süzgeçle örnekler. Kaynak, bütün çizilen ya da taşınan resmin ilk ölçeklenmesinde ayrı dokuya
kopyalanır; sonraki çentikler aynı kaynağı yeniden örnekler.

Uzaklaşırken kaynağın kaplamadığı halka şöyle dolar:

- **Geniş resimden** (`Targets::wide`): bir keskin resim bütün çizildiğinde, kendisinden daha uzaktan çizilmiş ve onu
  bütünüyle kaplayan geniş resim kalır (`zoomed::keeps_wide`). Kalmıyorsa, hareketin ayırdığı kaynak onu kaplıyorsa geniş
  resim o olur (yakınlaşmanın geride bıraktığı, bir adım uzaktaki resim); o da kaplamıyorsa bu resmin kendisi. Geniş
  resim yeni resimden en çok sekiz kat uzaktan çizilmiş olabilir (`zoomed::WIDE_RATIO`: bir kaynağın dört katlık aralığı
  ve bir adım daha, böylece dört katı az geçerek geride kalan kaynak da sayılır). Zoom karesinde geniş resim yeni resmi
  bütünüyle kaplıyorsa (`zoomed::cover`) önce o ölçeklenip bütün resme çizilir, kaynak üstüne gelir. Yakınlaşıp durup
  sonra uzaklaşan kullanıcı, halkayı bir adım önce gördüğü resimden görür.
- **Yoksa şeritlerle:** ADR 1067 §2'nin şeritleri, yalnız şeride erişen batch'lerle bütün çizilir. 10⁶'da yoğun bir
  bölgede bu pahalıdır: ölçüldüğünde 40–95 ms tutar, çünkü halkadaki içerik gerçekten çizilir.

Görünüm durunca (`moving` yanlış) resmin anahtarı değişir ve kare resmi bütün çizer. Duran kare ölçeklemeden bağımsızdır
(§4).

### 3. Uzak yakınlaşmada stilli sahnenin boşuna kurulması

Ölçüm sırasında bulundu (§5): stilli sahne, yapı çizgilerinin (sonsuz doğru, ışın) kırpma kutusu görünümün 20 katından
geniş kaldığında (2,86 kat yakınlaşmada) bütün yolundan geçiyordu. Çizimde hiç yapı çizgisi olmasa da 10⁶'da bu 20 ms
sürüyordu. Artık yapı çizgisi parçası yoksa yalnız kutu görünümü izler ve sahne olduğu gibi döner
(`StyledCache::scene_themed`); varsa eskisi gibi o parçalar yeniden kırpılır. Testi:
`style::scene_tests::a_far_zoom_builds_again_only_for_construction_lines`.

### 4. Eşitlik ve kararlılık

`perf::stability` (10⁴, gerçek GPU, [ham veri](../perf/stability-desktop-adr1068-2026-11-06.json)) üç görünümde (uzak
0,70 px/m, 1:200, 1:20) yeni bir satır grubu aldı. Görünüm hareket ederken iki tekerlek çentiği atılır (içeri 1,25 × 1,25,
dışarı 0,8 × 0,8), sonra görünüm durur. “Geniş” satırında daha önce iki kat uzaktan çizilmiş bir resim vardır; “dışarı”
satırında geniş resim bilerek başka bir çizimin resmiyle değiştirilir, halka şeritlerle çizilir. “Uzun” satırında görünüm
10,5 kat yakınlaşır (4,1 katta bütün çizilir), durur, sonra 6,7 kata uzaklaşır: halka yakınlaşmanın geride bıraktığı
4,1 katlık kaynaktan gelmelidir. Karşılaştırma için aynı kareler ölçekleme kapalıyken de çizilir
(`Viewport::set_zoom_scaled(false)`: her zoom karesi bütün çizilir).

| Görünüm | Yön | İki çentik ölçeklendi | Halka geniş resimden | Duran / kontrol: farklı | Duran / hareketsiz bugünkü: farklı (görünür) | Ölçeklenen / bugünkü: görünür |
|---|---|---|---|---:|---:|---:|
| Uzak (0,70) | içeri | evet, evet | – | **0** | 57 683 (22 299) | 55 514 |
| Uzak (0,70) | dışarı | evet, evet | hayır | **0** | 71 778 (27 158) | 35 018 |
| Uzak (0,70) | geniş | evet, evet | evet | **0** | 71 768 (27 168) | 35 029 |
| 1:200 (18,9) | içeri | evet, evet | – | **0** | 0 | 4 581 |
| 1:200 (18,9) | dışarı | evet, evet | hayır | **0** | 1 177 (499) | 5 451 |
| 1:200 (18,9) | geniş | evet, evet | evet | **0** | 1 177 (499) | 5 777 |
| 1:20 (189) | içeri | evet, evet | – | **0** | 0 | 276 |
| 1:20 (189) | dışarı | evet, evet | hayır | **0** | 0 | 53 |
| 1:20 (189) | geniş | evet, evet | evet | **0** | 0 | 61 |
| Uzak, 1:200, 1:20 | uzun | evet | evet | **0** | – | – |

(Her satırda 364 518 piksel karşılaştırıldı.)

- **Duran kare ölçeklemeden bağımsızdır** (test doğrular): kare durunca resmi bütün çizer ve ölçekleme kapalıyken aynı
  hareketin duran karesiyle piksel piksel aynıdır.
- Duran karenin hareketsiz yoldan çizilen kareden farkı (aynı ölçeklerden hiç hareket etmeden geçilen yol) bu ADR'den
  değil, hareketin kendisinden gelir: kontrol de aynı farkı taşır. Uzakta tekerlek dönerken yazılar yerleştikleri boyda
  tutulur (ADR 1064 §3). Test kamerayı tekerlek olayı olmadan değiştirdiğinden yazıların duruştaki yeniden yerleşimi
  (`Event::Settled`) bu karede gelmez.
- **Tekerlek dönerken** resim son keskin resmin ölçeklenmiş hâlidir. Bütün çizimden görünür fark yakında 53–5 777,
  uzakta 35 018–55 514 pikseldir: ekran boyutlu yazılar ve çizgi kalınlıkları ölçekle büyür ya da küçülür. Geometri
  yerinde kalır. Resimler `.run/gui-evidence/cad-adv-19/yakinlastirirken-*` altındadır (`-a` ölçeklenen, `-b` bütün
  çizilen, `-durunca-*` duran ve bugünkü).
- ADR 1067'nin 33 satırı (kaydırma, kaydırırken, ayrıntı geçişleri, kısayollar) ve kenetler sayı sayı aynıdır.

### 5. Ölçüm (Apple M5, release)

Ortam: Apple M5 (10 çekirdek), 32 GB, macOS 26.6.2, GPU Apple M5 (Metal), rustc 1.96.0, `--release` (lto thin),
1440×900, 4× örnekleme. “Önce” ADR 1067'nin aynı gün aynı makinede alınmış “sonra”sıdır (`483a4cc09`); “sonra” bu dalın
ikilisidir (`a2f6c8646` üstünde) ve aynı ikiliyle iki kez tekrarlandı. Bütün tablolar:
[docs/perf/large-desktop-adr1068-2026-11-06.md](../perf/large-desktop-adr1068-2026-11-06.md).

Teşhis (`KENTOS_PERF_TRACE_VIEW`, kare kare): ilk biçimde (oran iki kat, geniş resim yok) yakınlaşan kareler 6–10 ms'ydi.
Uzaklaşan kareler ise halkanın şeritleri yüzünden 40–95 ms tutuyordu; 2 kat sınırını aşan kareler de 58–117 ms'de bütün
çiziliyordu. 10⁶ uzak tekerlek GPU p50'si 65 → 37 ms'de kaldı. Oranın iki oktava çıkması ve geniş resim bunları
kaldırdı. Kalan 22 ms'lik arayüz karesi §3'tü.

| (ms; ısınmış koşu, önce → sonra) | 10 000 | 100 000 | 1 000 000 |
|---|---:|---:|---:|
| Uzak tekerlek GPU p50 | 7,4 → 3,5 | 15,3 → 2,8 | **65,0 → 7,0** |
| Uzak tekerlek GPU p50, ilk koşu | 7,5 → 4,4 | 16,8 → 4,4 | 67,4 → 8,1 |
| Uzak tekerlek GPU p95 sonra (üç koşu) | 5,6 | 4,5 | 12,8 / 8,9 / 13,0 |
| Yakın 1:200 tekerlek GPU p50 | 7,1 → 5,3 | 7,6 → 5,2 | 9,2 → 8,4 |
| Tekerlek izinde bütçeyi aşan arayüz karesi (uzak + yakın) | 0 → 0 | 0 → 0 | **2 → 0** |
| Tekerlek MİB p99, uzak | 1,6 → 1,9 | 1,3 → 1,1 | **22,9 → 1,8** |
| Görünümün tamponları (MB) | 27 → 30 | 112 → 115 | 962 → 965 |

Üç koşunun hepsinde, 10⁴–10⁶ ve uzak/yakın, imleç, tekerlek ve kaydırma izlerinin GPU p50'si en çok 13,7 ms'dir;
bütçeyi aşan arayüz karesi hiçbir izde yoktur. p95 tekerlekte en çok 15,5 ms'dir. 10⁶ yakın kaydırmada p95 15–17 ms,
bütçenin sınırındadır. Bu bu ADR'nin değil ADR 1067'nin yoludur: onun kendi ölçümü 12,7–17,2 ms'ydi, p50'si koşudan
koşuya 9,3–13,7 ms arasında oynar. Kaydırmanın 10⁴/10⁵ ısınmış p50'si de 2,9–7,1 ms arasında oynar (GPU'nun başarım
durumu, ADR 1064 §5). Hepsini seç 10⁶'da 1 131 → 271–274 ms'dir (ADR 1027'nin sınırları; bu ADR'nin değil).

### 6. Bellek

Görünüm başına bir resim boyunda doku daha (`wide`) eklendi. Ölçümün penceresinde ~3 MB tutar (görünümün tamponları
10⁶'da 962 → 965 MB); tam ekran bir HiDPI çizim alanında (2 880×1 800) 20,7 MB. Hedeflerin sayılan baytına eklendi.
İki tekdüze tampon (32 B) ve iki bağlama grubu da eklendi. Geniş resmin kopyası yalnız geniş resim değiştiğinde bütün
çizilen karede yapılır.

### 7. Sınırlar

- Tekerlek dönerken resim son keskin resmin yeniden örneklenmesidir. Yaklaşırken çizgiler ve yazılar kalınlaşır ve
  bulanıklaşır (en çok 4 kat), uzaklaşırken incelir; ekran boyutlu simgeler, yazılar ve tarama desenleri de ölçeklenir.
  150 ms sonra resim bütün çizilir. Kenet, seçim ve ölçü resimden bağımsızdır (kaynak geometri); hareket sırasında da
  etkilenmez.
- Uzaklaşırken halkayı kaplayan bir geniş resim yoksa (ilk kez görülen bir uzaklık ya da uzağa kaydırılmış görünüm),
  halka şeritlerle bütün çizilir. 10⁶'nın yoğun bir bölgesinde bu kareler 60 Hz'de değildir (40–95 ms). Ölçüm izinin
  tekerleği bu durumu içermez.
- Halka geniş resimden geldiğinde daha bulanıktır (en çok 4 kat ölçeklenir); kaynağın kenarında kesilen bir yazı orada
  geniş resmin yazısıyla birleşmeyebilir (geçici).
- Çizim anahtarı, ev sahibinin bildiklerini kapsar (§1). Tekerlek dönerken başka bir yoldan gelen bir değişiklik, en çok
  150 ms ölçeklenen resimde görünmeyebilir; şeritlerde ve duran karede görünür.
- Döndürülmüş görünüm, raster ve servis karoları, nokta bulutu, efekt grupları ve ekrana bağlı desenler eskisi gibi her
  karede bütün çizilir.

## Sonuçlar

- Tekerlek dönerken 10⁶ uzak görünümün GPU'su 65 → 7,0 ms (p50), 10⁵'te 15,3 → 2,8 ms; tekerlek izinde bütçeyi aşan
  arayüz karesi kalmadı. Hafif etkileşimin (imleç, tekerlek, kaydırma) üç izi 10⁴–10⁶'da 60 Hz bütçesinde
  (p50 ≤ 13,7 ms); 10⁶ yakın kaydırmanın p95'i bütçenin sınırında (15–17 ms, ADR 1067'nin yolu).
- Duran kare, ölçekleme olmadan çizilecek kareyle piksel piksel aynıdır. Tekerlek dönerken gösterilen ölçeklenmiş
  resimdir, bulanıklığı duruşta biter.
- Görünüm başına ~3 MB (HiDPI tam ekranda 20,7 MB) bellek.
