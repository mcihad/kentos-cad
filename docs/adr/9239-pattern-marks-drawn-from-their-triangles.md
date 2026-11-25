# ADR 9239: Desen dolgusunun SVG işaretleri büyüyünce üçgenlerinden çizilir

- **Durum:** kabul edildi (2026-11-25). Plan: [plan/render.md](../../plan/render.md) RM-11 (desen dolguları); RM-15'teki
  aynı madde.
- **İstek:**
  - sahibin 23 Kasım sözü: “SVG çizgileri, stil çizgileri çok zoom yapınca testere gibi”;
  - planın açığı: “Desen dolguları atlastaki karo bitmap'iyle çiziliyor ve yakında büyütülür. Vektöre geçmek, alanla
    kırpma ister (maske ya da kalıp).”
- **Sözleşmeler:**
  - Paylaşılan WGSL ve bağlamaları değişmedi; web etkilenmez.
  - Stil verisi değişmedi: `StyledLayer`, toplu çizimler ve fikstürleri aynı.
  - Eklenenler yalnız masaüstüne ait: bir gölgelendirici eki (`styled/vector_tiles.wgsl`) ve bir ardışık düzen.

## §1 Bulgu

**BÖHHBÜY'ün 30 desen dolgusu:**

- 22'si şekil işaretli. Şekiller gölgelendiricide analitik çiziliyor (`patternFs`), her yakınlıkta zaten keskin.
- 8'i SVG işaretli. Örnekler: 0633 Bataklık, 1404 Çalılık, 1407 Zeytinlik, 1412 Gül bahçesi.

**Testerenin kaynağı:** SVG, yazı ve resim işaretleri tek bir atlas karosuna basılıyor. Karo en çok 512 px genişlikte
(sayfa dolarsa 256) ve en çok 1 024 px yükseklikte. Ekranda bundan büyük göründüğünde büyütülüyor.

- 1407 Zeytinlik: 10 mm'lik kaydırmalı karo, içinde 3,6 mm'lik bir dal, 1:1000'de. Karo 51 px/m'de 512 px'e varıyor.
- Ölçek çubuğu 50 cm olan görünümde dalın kenarı 8–10 px'lik bulanık bir rampa, çaprazları basamaklı (gerçek pencere).

**Kırpma şart:** sınırı kesen işaretin alanın dışına taşmaması gerekiyor.

- CPU'da kırpma denendi ve ölçüldü. Simgelerin ağı büyük: zeytin 800, çalılık 732, bataklık 520 üçgen.
- Sınırı kesen her işaret kırpılınca ağının kopyası ~38 KB tutar. Binlerce küçük zeytin parselinde işaretlerin
  çoğu sınırdadır; bu gigabaytlar eder.
- Kalıp (stencil) ise bütün ardışık düzenlerin ve geçişlerin ekini değiştirirdi.

**Ayrıca bulundu:** `svg_marks`'ın üçgenleyicisi kendini kesen halkayı yanlış dolduruyordu.

- Keskin dönen bir çizginin dış hattı kendini keser (eklemin içi katlanır).
- Kulak kırpma, kesişmenin ötesini kaybediyordu: dalın yaprak ucu ve dibi delik çıkıyordu.
- [ADR 9208](9208-svg-symbols-as-vectors.md)'in vektör işaretleri de bundan etkileniyordu.

## §2 Karar

### §2.1 Program: işaretlerin üçgenleri, kutularında bir ızgara

`kentos_native_style::vector_tiles::program` karonun SVG işaretlerini tutar:

- Her işaretin üçgenleri renk başına (`svg_marks::mesh`).
- Yeri:
  - hücredeki ortası (kaydırmalı karonun ikinci sırası yarım kaydırılmış);
  - ötelenmesi;
  - kutusuna sığdırılışı (`raster::fit`);
  - kendi dönüşü.

  `raster::tile_image` nasıl basıyorsa tam öyle.
- Her işaretin kutusunda nx × ny'lik bir ızgara. Her göz, değdiği üçgenlerin listesini tutar.

İstisnalar:

- Bir sayfaya (256 KiB) sığmayan karo bitmap'te kalır.
- Yazı ya da resim işareti olan karo da bitmap'te kalır, çünkü bütün işaretleri tek bir resimdedir.

Program, katmanın uzun kesik desen sayfalarına konur (`styled/dashes.rs`, grup 1'in depolama tamponu). Aynı karo tek
kopya tutulur ve yeni bir bağlama grubu gerekmez: Iced'in aygıtı iki grup alıyor.

### §2.2 Gölgelendirici: `tileVectorFs`

Alanın kendi üçgenlerini `tileFs` gibi çizer. Her piksel için:

- MSAA'nın dört örnek noktasında, o noktaya uzanan işaretlerin hücreleri bulunur.
- Nokta işaretin SVG'sine çevrilir ve gözündeki üçgenler sınanır.
- Her renk, örneklerden kapladığı pay kadar üst üste konur. Sonraki işaret öncekinin, sonraki renk öncekinin üstünde.

Kırpma kendiliğinden olur: alanın üçgenleri dışında piksel yoktur. İşaret başına bellek tutulmaz.

[`covers`] gölgelendiricinin CPU'daki okumasıdır; sınamalar onu ikiz olarak kullanır.

### §2.3 Geçiş: bitmap keskinken bitmap

Eşik `vector_tiles::switch` ile hesaplanır:

`s = en büyük(en geniş işaretin kutusu, karonun eni × 128/512, karonun boyu × 128/1024)`

- `s × px/m > 128` olunca üçgenler çizilir. Bu, ADR 9208'in 128 px kuralıyla atlasın sınırından hangisi önce
  gelirse odur.
- Ardışık düzen çizim anında seçilir. Büyüteç kendi ölçeğiyle seçer.
- Üçgenler çizilirken atlas karoyu basmaz.
- Sonuç: bitmap'in keskin olduğu her ölçekte resim öncekinin aynısı.

### §2.4 Kendini kesen dış hatlar

`svg_marks::fill_rings`, nonzero dolguları ve çizgilerin dış hatlarını önce basit alanlara ayırır, sonra kulak
kırpar.

- Ayırmayı geometri çekirdeğinin kesişim motoru yapar (`overlay`, `Rule::Any`, sarım ≠ 0).
- Even-odd dolgular eski iç içe halka yolunda kalır.

## §3 Kanıt

**Sınamalar:**

- `vector_tiles` (3 sınama):
  - Bitmap'in (`raster::tile_image`) açıkça boyalı ya da açıkça boş pikselleri, programın okumasıyla karşılaştırıldı.
    Biri kaydırmalı karoda dönük ve ötelenmiş bir L, öbürü yuvarlak uçlu bir dal.
  - Dal §2.4'ten önce 2 501 pikselin 35'inde ters okunuyordu. Şimdi en çok 1/500.
- `svg_marks`: yaprağın ucu ve dibi boyalı. Düzeltme olmadan kırmızı.
- GPU sınaması (UHD 770 ve lavapipe):
  - Eşiğin %95'inde bitmap, %105'inde, 6 ve 40 katında üçgenler; 1 ve 4 örnekle.
  - Açıkça iç ve dış pikseller CPU ikizinden en çok %0,5 farklı.
- naga yerel modülü doğruluyor; paylaşılan kaynakta `dashPage` yok.
- Paketlerin sonuçları:
  - native stil 112, stil çekirdeği 98, render-wgpu'nun bütün sınamaları (GPU dahil) geçti;
  - masaüstünün altın görüntüleri, uygunluk ve stil sınamaları: 99'u geçti.

**Gerçek pencere** (1407 Zeytinlik, Xvfb ve lavapipe): açılış ve ilk yakınlaşma öncekiyle piksel piksel aynı (fark 0).
Sonraki iki yakınlaşmada 1 344 ve 16 050 piksel değişti: kenar keskin. Görüntüler:
`.run/gui-evidence/render/rm11-desen/rm11-desen-{en-yakin,kenar}.png`.

**Maliyet** (sürüm derlemesi, UHD 770, 4×, 1920 × 1080'in tamamı desenli alan, yükleme ve geri okuma dahil):

| | Kare süresi |
|---|---|
| Eşiğin altında bitmap | 9,1 ms |
| Eşiğin üstünde üçgenler | 11,7 ms |
| Dört kat daha yakın | 11,7 ms |

Bedel 2,5 ms ve yalnız büyütülmüş görünümde.

## Uygulama

**Native stil:**

- `crates/native/style/src/vector_tiles.rs` (yeni): `program`, `switch`, `covers`.
- `crates/native/style/src/svg_marks.rs`:
  - `mesh` ve `Meshes` paketin içine açıldı;
  - `nonzero` / `nested`.

**Çizici:**

- `crates/render/wgpu/src/styled/vector_tiles.wgsl` (yeni) ve `shader.rs`.
- `crates/render/wgpu/src/styled/gpu.rs`:
  - `Pipe::TileVector`;
  - `GpuBatch::vectors`;
  - programın sayfaya konması;
  - görünürlükte atlası atlama;
  - çizimde seçim.
- Sınamalar: `crates/render/wgpu/tests/gpu/vector_tiles.rs`. Sınama kaynağı (`RedTiles`) SVG'leri de okuyor.

[`covers`]: ../../crates/native/style/src/vector_tiles.rs
