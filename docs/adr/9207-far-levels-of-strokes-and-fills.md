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

**Sonraki adım (RM-10'un devamı):** hangi çizgi toplu çizimlerinin pahalı olduğu kare yakalama ya da GPU zaman
damgasıyla bulunacak. Adaylar:

- çok geniş dörtgenler (kalınlık ya da bulanıklık dünya biriminde);
- toplu çizim başına sabit bir GPU maliyeti.

Gölgelendirici bir çizgi örneğini en az 4 piksel genişliğinde bir dörtgen olarak çiziyor
(`ext = halfW + blur/2 + 1`).

## Uygulama

- `crates/native/style/src/far_levels.rs` (yeni), `batches.rs` (`decode`), `lib.rs`.
- `crates/render/wgpu/src/styled/gpu.rs`: düz dolgunun düzeyi; `items` çizilen düzeyi sayar.
- `crates/render/wgpu/src/{renderer.rs,stats.rs}`: `FrameStats::styled_items`.
- `apps/desktop/src/shell/perf/files.rs`: açılış görünümünün sayıları (`drawn`), `KENTOS_PERF_SETTINGS`.
- **Tanı:**
  - `KENTOS_DIAG_SKIP` (`renderer.rs`: `styled`, `plain`; `styled/gpu.rs`: `stroke`, `solid`, `hatch`, `pattern`,
    `marker`, `raster`; `view/area.rs`: `labels`);
  - `perf::million`'ın `wheel` kipi;
  - `perf::large`'ın tekerlek dökümünde stillenmiş öğe sayısı ve ölçek.
