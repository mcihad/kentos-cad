# ADR 9208: Model ekranında SVG simgeleri vektör olarak

- **Durum:** kabul edildi (2026-11-23). Plan: [plan/render.md](../../plan/render.md) RM-11 (ilk adım).
- **İstek:** sahibin sözü (23 Kasım 2026): “Çapraz çizgileri çok zoom yaptığımda testere gibi gözüküyordu, bu neden”,
  ardından “Svg çizgileri / Stil çizgilerini”.
- **Sözleşmeler:**
  - Ortak çekirdek (`kentos-style-core`) ve web değişmedi.
  - Yerel stilin çözümü (`kentos_native_style::batches::decode`), model ekranında yere ölçülü SVG işaretlerini örnekli
    düz dolgulara çevirir. `Instances` yeni bir alan aldı: `mark`.
  - Çizici, maske kutularını bu örneklerden de üretir.
- **Bağımlılık:** ADR 1063 §2 (örnekli bloklar, `SolidInstanced`), 0226 §3 (maskeler), 0285 §2 (görünümün dönüşü),
  0793 §1 (model ekranı).

## §1 Sorun

Bir işaretin görünüşü SVG ise işaret atlastan çiziliyordu (`styled/atlas.rs`):

- Her SVG 8–512 px'lik bir bitmap'e rasterleştirilir. Bütün bitmap'ler 2048 × 2048'lik tek bir sayfadadır ve sayfa
  dolunca üst sınır yarıya iner.
- Yere ölçülü (`Unit::World`) bir işaret yakınlaştıkça büyür (gölgelendiricinin `unitK`'sı). Bitmap'in boyu ise
  sınırlıdır; ötesinde büyütülür. Çapraz kenarlar önce basamaklanır, daha yakında birkaç yüz piksellik bulanık bir
  rampaya döner.

Örnek: BÖHHBÜY 0807 “Yüksek gerilim hattı”. Şerare simgesi 9 mm, 1:1000'de yerde 9 m. Görünüm 24 çentik
yakınlaştırılınca simgenin kenarı yaklaşık 300 piksel genişliğinde bir geçişe dönüşüyor
(`.run/gui-evidence/render/rm11/once/05-en-yakin.png`).

## §2 Araştırma

- **Ekran ölçülü simge** (harita motorlarının yol işaretleri, POI simgeleri): bitmap ya da SDF atlası yeterli, çünkü
  simge hiçbir zaman atlastaki boyunu aşmaz.
- **Yere ölçülü simge** CAD'deki bloğa benzer: büyüdükçe vektör kalmalıdır. CAD çizicileri bu tür simgeleri bir kez
  üçgenler ve her yerleşimde örnekleyerek çizer. KentOS'ta blok örneklemesi (ADR 1063 §2) bu yolu zaten kullanıyor.
- **Seçenekler:**
  - Yakında daha büyük bitmap: 512 px sınırı ve tek sayfa yüzünden yetmez. Simge ekrandan büyüdükçe bellek de büyür.
  - MSDF: köşeleri keskin tutar. Ama ince çizgilerde ve çok renkli simgelerde zayıftır, piksel başına doku okuması da
    sürer.
  - **Vektör örnekleme:** her yakınlıkta kesin kenar verir. Kenar yumuşatmayı çizimin MSAA'sı yapar. Seçildi.

## §3 Karar

`kentos_native_style::svg_marks`:

- **Kapsam:** model ekranında (`hold_weights`), yere ölçülü ve görünüşü SVG olan işaret toplu çizimleri. Bitmap'te
  kalanlar:
  - ekran ölçülü işaretler: bitmap'lerinin boyunu aşmazlar;
  - yazı ve raster işaretler, desen karoları;
  - ayrıştırılamayan, boş ya da yalnız gradyan veya desen boyalı SVG.
- **Simgenin ağı:**
  - SVG bir kez okunur (usvg).
  - Dolgular geometri çekirdeğinin kulak kırpmasıyla üçgenlenir. İç içe halkalar SVG'nin kendi dolgu kuralıyla
    (`nonzero` ya da `evenodd`) dolar ya da delik olur. Her alan bir kez üçgenlenir; yarı saydam bir simgede üst üste
    binen üçgen koyulaşırdı.
  - Çizgiler tiny-skia'nın kalınlaştırıcısıyla dış hatta çevrilir ve sarım kuralıyla üçgenlenir.
  - Eğriler, simgenin büyük kenarının 1/8000'i içinde düzleştirilir: 8 000 px boyundaki simgede bile sapma bir
    pikselin altında kalır.
- **Toplu çizimler:** renk başına bir tane, SVG'de ilk görüldükleri sırayla. Her işaret bir örnektir:
  - doğrusal parça: işaretin dönüşü ve boyu;
  - öteleme: yeri ve dönen ötelenmesi.
- **Yerleşim, işaret gölgelendiricisinin birebir aynısı** (`markerVertex`):
  - Kutu yerin ortasındadır; çapa ve ötelenme kutunun içinde uygulanır.
  - Dönüş saat yönünün tersine ve y yukarıdır, çünkü gölgelendirici döndürmeyi `pxToClip`'ten önce yapar ve
    `pxToClip` ekranı ters çevirmez.
  - İlk denemede y aşağı varsayıldı: simge −açıyla döndü ve çizginin üstüne yattı. Sınama artık gölgelendiricinin
    formülünü bağımsız bir ikiz olarak kullanıyor.
- **Duyarlılık:** öteleme çift duyarlıkta hesaplanır. Artığı, işaretin yerinde olduğu gibi katmanın düşük akışına
  (`low`) yazılır; örnek gölgelendiricisi onu `toPxLow` ile ekler.
- **Çizim sırası:** simgenin toplu çizimleri işaretin yerini alır. Parça parça kurulan büyük katmanda
  (`merged_order`) da işaretlerin sırasında kalırlar (`Instances.mark`, `kind_rank` 2). Bu olmasaydı dolgu sayılıp
  çizgilerin altına inerlerdi.
- **Maske** (ADR 0226 §3): simgenin ilk rengi işaretlerin kutularını verir (`svg_marks::boxes`). Çizici bu kutuları
  işaret kutularıyla aynı biçimde ekrana çevirir.
- **Kenar yumuşatma:** çizimin MSAA'sı (öntanımlı 4×).
- **Bilinen sınır:** SVG'de aynı rengin başka bir renkle bölünmüş parçaları tek toplu çizimde birleşir; çizim sırası
  renk sırasına göre olur. BÖHHBÜY simgeleri tek renklidir.
- Pafta, PDF ve CPU yolu değişmedi; onlarda `hold_weights` kapalı.

## §4 Kanıt

**Sınamalar** (`svg_marks::tests`, 6):

- bir SVG işaret örnekli üçgenlere döner; çeyrek dönüş yerde saat yönünün tersine çeyrek dönüştür;
- delikli halka ve çizgi renk renk dolgu olur; alanları doğrudur;
- aynı yönde iç içe iki kare `nonzero` ile bir kez dolar (400), `evenodd` ile delik kalır (300);
- gölgelendiricinin ikizi: 20 × 10'luk simgenin bir köşesi, çapa, ötelenme ve 30° dönüşle tam gölgelendiricinin
  koyduğu yerde. Maske kutusunun ortası, boyu ve dönüşü de aynı;
- iki renkli simge: renk başına toplu çizim, ötelemenin artığı düşük akışta, parçalı katmanda çizginin ardından;
- ekran ölçülü ve okunamayan simgeler bitmap'te kalır.

Yerel stilin (53 + 51 + 4), çizicinin (98) ve plot'un (22) bütün sınamaları geçti.

**Gerçek pencere** (Xvfb, lavapipe, `.run/gui-evidence/render/rm11/{once,sonra}`): sınama çizimi 380 m'lik bir 0807
hattı. Önce tümü gösterildi, ardından ortadaki şerareye 8, 4, 4 ve 8 çentik yakınlaşıldı.

- **Yer:** parlak piksellerin ağırlık merkezi önce ve sonra en çok 0,12 piksel ayrı (02–04).
- **Kenar:** “önce” yakınlaştıkça bulanıklaşıp basamaklanıyor. “Sonra” her yakınlıkta keskin; kenar pikselleri zemin
  ile beyazın tam %25, %50 ve %75 karışımı (82, 140, 198), yani 4× MSAA.

Karşılaştırma sayfası: `.run/gui-evidence/render/rm11/rm11-karsilastirma.png`.

**Ölçüm** (UHD 770, release, `perf::files`): büyük örnek dosyaların (sivas.kcad, NCZ, DWG, DXF) hiçbirinde SVG işaret
yok, onlarda dönüşüm hiçbir şey yapmadan döner. Maliyet yapay bir çizimde ölçüldü: 0807 stilli 20 000 hat, 380'er m.

| | Önce | Sonra |
|---|---|---|
| Tekerlek karesinde GPU (p50 / p95) | 15,8 / 24,0 ms | 16,3 / 20,2 ms |
| Kaydırma karesinde GPU (p50) | 12,7 ms | 13,0 ms |
| İmleç karesinde GPU (p50) | 12,8 ms | 13,2 ms |
| Açılışın ilk karesi, GPU / arayüz | 88 / 52 ms | 136 / 36 ms |
| GPU'da tutulan | 20,4 MB | 29,4 MB |
| En yüksek bellek | 341 MB | 363 MB |

Kare başına maliyet yarım milisaniyenin altında kaldı. İlk karedeki GPU artışı sonraki karelerde yok, yani tek
seferlik; olası nedeni örnek tamponlarının ilk yüklenmesi. Arayüzün payı azaldı; olası nedeni simgelerin artık atlasa
rasterleştirilmemesi. İki neden de ayrıca ölçülmedi. Yazı işaretleri (ENH) bitmap'te kaldı.

## Uygulama

- `crates/native/style/src/svg_marks.rs` (yeni), `batches.rs` (`Instances.mark`, `Mark`, `kind_rank`, `decode`),
  `lib.rs`, `Cargo.toml` (`resvg`).
- `crates/render/wgpu/src/styled/gpu.rs` (`marker_boxes`).
