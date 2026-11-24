# ADR 9207: Model ekranında düz çizgilerin ve dolguların uzak düzeyleri

- **Durum:** kabul edildi (2026-11-23). Plan: [plan/render.md](../../plan/render.md) RM-10 (ilk adım).
- **İstek:** sahibin sözü (23 Kasım 2026): “lod sistemi yok … performans iyileştirmeleri optimizasyonlar sağlamamız
  gerekiyor”. Bulgu: [ADR 9200](9200-render-engine-research.md) §1.2.
- **Sözleşmeler:**
  - Ortak çekirdek (`kentos-style-core`) ve web değişmedi.
  - Yerel stilin çözümü (`kentos_native_style::batches::decode`) model ekranında toplu çizimlere `Lod` ekler.
  - Çizici, düzeyli düz dolguyu çizer.
- **Bağımlılık:** ADR 1063 §3 (eğri düzeyleri, `Lod`), 1064 §2 (uzun çizgilerin düzeyleri), 0793 §1 (model ekranı).

## §1 Sorun

Bütün kenti gösteren görünümde bir piksel yaklaşık 30 m'dir. Parsellerin ve yapıların çoğu bir pikselden küçük, ama
her biri bütün kenarları ve dolgu üçgenleriyle çiziliyordu:

| Dosya | Nesne | Açılış görünümünde çizilen öğe | Tekerlek karesinde GPU |
|---|---|---|---|
| sivas.kcad | 999 975 | 3 188 380 | 104 ms |
| İmar planı NCZ | 453 534 | 4 173 040 (nesne başına 9) | 87 ms |

Ölçüm: UHD 770, release. Var olan düzeyler yalnız eğrileri (1063) ve en az 8 köşeli uzun çizgileri (1064) kapsıyor.
En kaba eşikleri 0,25 px/m, yani 4 m/piksel; kent görünümünden çok daha yakın.

## §2 Araştırma

- **Harita motorları** (QGIS, MapLibre) uzak görünümde geometriyi ekran toleransıyla sadeleştirir. QGIS'in öntanımlı
  eşiği 1 pikseldir.
- **Köşe kümeleme** (vertex clustering): noktaları bir ızgaraya oturtup yinelenenleri atmak. Ekrandaki sapmayı ızgara
  boyuyla sınırlar ve hiçbir pikseli boş bırakmaz. Douglas–Peucker ise küçük bir halkayı noktaya indirebilir ama
  komşu iki parselin ortak kenarını iki kez çizer.
- **KentOS'un kuralı:** CLAUDE.md §6.2, “geometriyi gizlice eksiltmeyin”. Düzey ekranda en çok yarım piksel oynatabilir
  ve hiçbir nesne pikselinden çıkamaz.

## §3 Karar

`kentos_native_style::far_levels`:

- **Kapsam:** model ekranında (`hold_weights`) düz kenar ve düz dolgu toplu çizimleri. En az 1 024 öğesi olmalı; kendi
  düzeyleri (eğri, uzun çizgi), örneklemesi, deseni ya da tarama dolgusu olanlar dışarıda.
- **Düzeyler:** üç uzak düzey. Eşikler 16, 64 ve 256 m/piksel (`FAR_CEILINGS`). Her biri o eşiğin yarım pikseli olan
  ızgarada: 8, 32 ve 128 m.
- **Kenarlar:**
  - Uçlar hücrelerin ortasına gider; aynı hücre çifti bir kez çizilir. İki parselin ortak kenarı da böylece bir kez
    çizilir.
  - İki ucu aynı hücreye düşen kenar, çizgi kalınlığında bir nokta olur: başlıkları açık, hücrenin yüzde biri
    uzunluğunda bir kenar.
- **Dolgular:**
  - Hücrelerde alan bırakan üçgen bir kez kalır.
  - Alan bırakmayan üçgenin hücreleri bir hücre karesiyle örtülür. Sık bir yapı adasının rengi kaybolmaz.
- **Düzen ve çizim:**
  - Düzeyler toplu çizimin kendi sayılarının ardına gider; `Lod { main: 0, levels: [kendisi, uzak1, uzak2, uzak3] }`.
  - GPU, eğri düzeylerinde olduğu gibi her karede tek düzey çizer. Düz dolgunun düzeyi için çiziciye bir dal eklendi.
- **Bellek kuralı:** ilk uzak düzey öğelerin en az dörtte birini atmıyorsa toplu çizim düzey almaz; bellek boşa
  gitmez (1064 §2'nin kuralı).
- **Pafta ve PDF** her kenarı çizer.

## §4 Kanıt

**Sınamalar** (`far_levels::tests`):

- komşu iki karenin kenarları ve küçük bir halka tek noktaya iner;
- uzun bir çizgi hücreden hücreye çizilir;
- küçük bir üçgenin hücresi kareyle örtülür;
- rastgele 5 000 kenarın uçlarının düştüğü her hücreye düzeyde yine dokunulur.

Yerel stilin (48 + 51), çizicinin ve plot'un (98) bütün sınamaları geçti.

**Ölçüm** (UHD 770, release, `perf::files`, açılış görünümü):

| Dosya | Çizilen öğe | Tekerlek karesinde GPU (p50) | GPU belleği |
|---|---|---|---|
| sivas.kcad | 3 188 380 → **1 557 539** (−%51) | 104,3 → **84,8 ms** | 138 → 153 MB |
| İmar planı NCZ | 4 173 040 → **903 516** (−%78) | 86,6 → 86,7 ms | 208 → 208 MB |

Açılış ve en yüksek bellek değişmedi: sivas.kcad 4,7 s / 2,75 GB, NCZ 3,7 s / 2,47 GB.

**Gerçek pencere** (Xvfb, `.run/gui-evidence/render/rm10/{once,sonra}-{ncz,sivas}`): kent görünümleri önce ve sonra
aynı görünüyor. Yarım piksellik kaymalar dışında fark yok:

| Dosya | 20'den büyük farkla değişen piksel |
|---|---|
| NCZ | 4 281 / 738 840 (%0,6) |
| sivas.kcad | 664 |

Çekirdeğin yakın görüntüsü `ncz-yakin-karsilastirma.png`.

## §5 NCZ'de tekerlek karesinin GPU süresi nereye gidiyor

NCZ'de öğe sayısı beşte bire indi ama kare süresi değişmedi. Neyin pahalı olduğu çizilmeden bırakılarak ölçüldü
(`KENTOS_DIAG_SKIP`, yalnız ölçümde okunur), tekerlek kareleri 76–93 ms:

| Atlanan | Kare | Sonuç |
|---|---|---|
| Stillenmiş katmanlar | 20–31 ms | süre burada |
| Yalnız çizgiler | 21–32 ms | stillenmiş payın tamamı çizgilerde |
| Dolgu, tarama, desen, işaret, raster | değişmiyor | |
| Düz parçalar | değişmiyor | |
| Çizim yazıları | 54–90 ms | yakında 20 ms'ye kadar |
| Stillenmiş, düz ve yazılar birlikte | 11–13 ms | arayüz ve geri okuma tabanı |

**Çizgilerin süresi öğe sayısıyla değişmiyor:**

- Bütün ölçeklerde ~0,9 M çizgi örneği çiziliyor.
- Süre çizim çağrısıyla birlikte değişiyor: 685 çağrıda 93 ms, 627 çağrıda 77 ms.
- `graphics.fills=false` ve `graphics.lineWeights=false` da süreyi değiştirmiyor.
- CPU'nun çoğu GPU'yu bekliyor. Görünen pay `memmove`/`realloc` ve iced'in üçgen katmanının her karede yeniden
  hazırlanması (%3,7).

**Düzey zorlama ölçümü** (`KENTOS_DIAG_LEVEL`): süre örnek sayısıyla doğru orantılı, örnek başına ~90 ns (UHD 770).

| Bütün düzeyli toplu çizimler bu düzeyde | Çizilen örnek | Kare |
|---|---|---|
| 0 | 2 500 145 | 237 ms |
| 1 | 1 428 486 | 136 ms |
| 2 | 1 096 518 | 106 ms |
| 3 | 901 651 | 89 ms |

**Kendi seçiminde** uzak düzeyli toplu çizimler doğru düzeydedir: 0,042 px/m'de düzey 1, yani 8 m
(`KENTOS_DIAG_LEVEL=print`). Kalan 0,9 M örneğin çoğu uzak düzey almayan toplu çizimlerde: eğrilerin (1063) ve uzun
çizgilerin (1064) düzeyleri. NCZ'de on binlerce küçük daire ve yay var; piksel altında kalanlar bile en kaba eğri
düzeyinde tur başına birkaç kirişle çiziliyor.

**Sonraki adım (RM-10'un devamı):**

- Uzak düzeyler eğri ve uzun çizgi düzeylerinin ardına da eklenecek. Düzeyin en kaba hâli ızgarada kümelenecek.
- `Lod`'un sabit üç tavanı (`[f64; 3]`) bunun için genişleyecek.

Ayrıca hangi çizgi toplu çizimlerinin pahalı olduğu kare yakalama ya da GPU zaman
damgasıyla bulunacak. Adaylar:

- çok geniş dörtgenler (kalınlık ya da bulanıklık dünya biriminde);
- toplu çizim başına sabit bir GPU maliyeti.

Gölgelendirici bir çizgi örneğini en az 4 piksel genişliğinde bir dörtgen olarak çiziyor
(`ext = halfW + blur/2 + 1`).

## §6 Eğri ve uzun çizgi düzeylerinin ardına uzak düzeyler (24 Kasım 2026)

§5'in sonraki adımı.

- **Kapsam:** kendi düzeyleri olan çizgi toplu çizimleri de uzak düzey alır. Bunlar eğrilerin kirişleri (1063 §3) ve
  uzun çizgiler (1064 §2).
- **Uzak düzeyin kaynağı:** en uzakta çizilen iki parça, yani ana parça ve en kaba düzey. Aynı ızgarada (8, 32 ve
  128 m) kurulur ve kendi düzeylerinin ardına eklenir.
- **Tavanlar:** kendi tavanlarının ardına eklenir: 4, 1, 0,25, 1/16, 1/64 ve 1/256 px/m.
  `Lod.ceilings` sabit üç sayılık bir diziden vektöre geçti.
- **Ana parça:** uzak düzey ana parçanın çizgilerini de içerir. Bu yüzden o düzeylerde ana parça ayrıca çizilmez
  (`Lod.alone`, `Lod::ranges`). GPU boş aralığı zaten atlıyordu; çizicide değişiklik gerekmedi.
- **Örnekli toplu çizimler** (bloklar) uzak düzey almaz: tanımları kendi birimindedir. Uzak bloklar 1065'in vekiliyle
  çizilir.
- **Bellek kuralı aynı:** ilk uzak düzey, ana parça ile en kaba düzeyin çizgilerinin en çok dörtte üçünü tutabilir.

**Sınama:** `a_curve_batch_takes_far_levels_after_its_own_and_drops_its_main_part_there`.

- Dört düzeyli bir eğri toplu çizimi yedi düzey alır.
- Uzak düzeyde ana parça çizilmez.
- En kaba düzeyin ve ana parçanın dokunduğu her hücreye uzak düzeyde yine dokunulur.

Yerel stilin (56 + 51 + 4), çizicinin (98) ve plot'un (22) sınamaları geçti.

**Ölçüm** (UHD 770, release, `perf::files`, açılış görünümü; ham sonuçlar `docs/perf/files-desktop-rm10b-*.json`):

| Dosya | Çizilen öğe | Tekerlek karesinde GPU (p50 / p95) |
|---|---|---|
| sivas.kcad | 1 557 539 → **1 382 830** (−%11) | 84,9 / 92,2 → **69,9 / 90,0 ms** |
| İmar planı NCZ | 903 516 → **831 268** (−%8) | 87,5 / 93,8 → **81,5 / 86,9 ms** |
| SİVAS AVM DWG | 259 012, aynı | 21,7 → 21,6 ms |
| Suşehri DXF | 493 447, aynı | 50,6 → 49,9 ms |

- DWG'de ve DXF'te çizilen öğe değişmedi; nedenine ayrıca bakılmadı.
- GPU belleği NCZ'de 208 → 222 MB, sivas.kcad'de 153 → 154 MB.
- Açılış süresi ve en yüksek bellek değişmedi.

**Gerçek pencere** (Xvfb, `.run/gui-evidence/render/rm10b/{once,sonra}-{ncz,sivas}`):

- NCZ'nin kent görünümünde 738 840 pikselin 642'si (%0,09) 20'den büyük farkla değişti; daha uzak görünümde 57.
- sivas.kcad'de fark yok.
- Farklar kırmızı eğrilerin yarım piksellik kaymaları, dağınık tek pikseller.

**NCZ'de kalan:** öğe sayısı yalnız %8 azaldı. Kalan 831 bin öğenin hangi toplu çizimlerde olduğu ölçülmedi. Adaylar:

- 1 024 öğeden küçük toplu çizimler: 685 çizim çağrısında ortalama 1 300 öğe düşüyor;
- örnekli bloklar;
- uzak tavanın üstündeki tekerlek kareleri: ölçülen kareler 0,037–0,077 px/m'de, 1/16 = 0,0625'in üstündeki ikisinde
  uzak düzey seçilmiyor.

Sonraki adım: öğeleri toplu çizim türüne, düzeyine ve boyuna göre sayan bir tanı.

## Uygulama

- `crates/native/style/src/far_levels.rs` (yeni), `batches.rs` (`decode`), `lib.rs`.
- §6: `far_levels.rs` (`source`, `per`), `batches.rs` (`Lod.ceilings` vektör, `Lod.alone`, `Lod::ranges`), `gpu.rs`
  (tanının tavan karşılaştırması `ends_with`).
- `crates/render/wgpu/src/styled/gpu.rs`: düz dolgunun düzeyi; `items` çizilen düzeyi sayar.
- `crates/render/wgpu/src/{renderer.rs,stats.rs}`: `FrameStats::styled_items`.
- `apps/desktop/src/shell/perf/files.rs`: açılış görünümünün sayıları (`drawn`), `KENTOS_PERF_SETTINGS`.
- **Tanı:**
  - `KENTOS_DIAG_LEVEL` (`styled/gpu.rs`: bir sayı bütün düzeyli toplu çizimleri o düzeye zorlar, `print` seçileni
    yazar);
  - `DRAWN_INSTANCES` (karede GPU'ya verilen örnekler);
  - `KENTOS_DIAG_SKIP` (`renderer.rs`: `styled`, `plain`; `styled/gpu.rs`: `stroke`, `solid`, `hatch`, `pattern`,
    `marker`, `raster`; çizgilerin yarıları `stroke-dash`, `stroke-plain`, `stroke-small`, `stroke-big`, `stroke-wide`,
    `stroke-blur`; `view/area.rs`: `labels`);
  - `perf::million`'ın `wheel` kipi;
  - `perf::large`'ın tekerlek dökümünde stillenmiş öğe sayısı ve ölçek.
