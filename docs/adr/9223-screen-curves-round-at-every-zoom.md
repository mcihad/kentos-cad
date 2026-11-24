# ADR 9223: Ekranın eğrileri her yakınlıkta yuvarlak

- **Durum:** kabul edildi (2026-11-24). Plan: [plan/render.md](../../plan/render.md) RM-09, 1. adım.
- **İstek:**
  - sahibin 23 Kasım isteği (“yakında daire köşeli görünmez”, plan madde 4);
  - 24 Kasım'da “kaliteden ödün verilmez” ([ADR 9222](9222-quality-is-not-traded-for-speed.md));
  - aynı gün “sen karar ver ve devam et”.
- **Sözleşmeler:** stil çekirdeğinin `View`'ına `curve_tolerance` eklendi; yoksa (web, kâğıt, PDF) çizim değişmez.
  Ortak çekirdeğin web bağının (`crates/wasm/geometry-wasm`) `View` değeri de alanları aldı. O değer RM-24'ün 1. adımından
  (`deep`) beri derlenmiyordu; web durduğu için (ADR 0228) kimse derlemiyordu.

## §1 Bulgu

- **Düzenli sahne** (500 px/m'nin altı, derin görünümün dışı) daire, yay, elips ve bombeli kenarları en çok 72 kirişle
  çiziyor (`store::draw::OUTLINE_SEGMENTS`, eğri düzeylerinin ince düzeyi `FINE_CHORDS`).
- **Sapma:** kirişin sapması yarıçapın 2⁻¹⁰'u kadar, 9,5·10⁻⁴ r. Ekranda yarıçapı 263 pikseli geçen eğri çeyrek
  pikselden çok sapıyor.
  - 22 m'lik bir daire 400 px/m'de 8,4 piksel köşe yapıyor.
  - Evin DXF'indeki terasın 5,57 m'lik yayı 495 px/m'de 2,6 piksel.
- **Derin görünüm:** 500 px/m'nin üstünü RM-24 kutusunda yuvarlıyor ([ADR 9220](9220-deep-view-scene.md)).

## §2 Karar

**Ekranın eğrileri bütün boylarınca bir dünya toleransına yuvarlanır** (`View::curve_tolerance`). Derin görünüm kendi
kutusunu ve toleransını kullanır. Derin değilken stil çekirdeği:

- daireyi, yayı ve elipsi derin görünümün yoluyla çizer (`deep::curve`), ama kutusu her yer (`deep::everywhere`);
- bombeli çoklu çizgi ve alanın yaylarını aynı biçimde noktaya çevirir (`deep::refined`).

Ayrıntılar:

- **Kiriş sayısı:** her kiriş eğriden en çok tolerans kadar sapar. Hiçbir eğri 72 kirişten azıyla çizilmez (bir kirişin
  açısı en çok 5°).
- **Kesik evresi:** CAD çizgi tipinin kesiği yine kaynak eğri üzerinden ölçülür (ADR 1122); noktalar eğrinin üstünde.
- **Eğri düzeyleri** (ADR 1063) uzak görünüm için yerinde. İnce düzey artık bu kirişlerdir.

**Tolerans** derin görünümün eşiğinde çeyrek piksel: `DEEP_TOLERANCE_PX / (DEEP_PX_PER_M × birim)`. Metre çiziminde
0,5 mm, milimetre çiziminde 0,5 birim.

- Eşiğin altındaki her yakınlıkta sapma çeyrek pikselin altında kalır; üstünü derin sahne alır.
- Tolerans görünüme bağlı değil: yakınlaştırınca hiçbir parça yeniden kurulmaz.
- Görünüme göre yeniden kurma da tartıldı:
  - eğri yoğun bir çizimde (NCZ) her iki katlık yakınlaşmada sahnenin çoğunu kurardı;
  - her bandın ince düzeyini saklamak yüzlerce MB tutardı.

**Bedeli:** büyük eğri daha çok kiriş alır. Yarıçap r metrede kiriş sayısı yaklaşık π·√(1 000·r):

| Yarıçap | Kiriş (bir tur) |
|---|---|
| 0,5 m | 72 |
| 5 m | ~222 |
| 22 m | ~466 |
| 100 m | ~993 |

**Dışında kalanlar:**

- **Harita projeksiyonlu görünüm:** eğrinin noktaları sonra projeksiyonla taşınıyor, toleransı haritanın biriminde
  olmalı.
- **Kâğıt ve PDF:** kendi kesinlikleri var (`plot_precision`).
- **Blok tanımları:** tanım kendi biriminde kaydediliyor ve örnekleri ölçekleniyor; dünya toleransı orada yanlış olur.
  Tanımın eğrileri 72 kirişte kalır. Kayıt sırasında tolerans geri alınıyor (`instances.rs`).

## §3 Kalan (RM-09)

- **Bloklardaki eğriler:** en büyük örneğin ölçeğine göre tanımın toleransı.
- **Eğri duvarlar:** depo, duvarın gövdesini sabit 5°'lik adımla parçalıyor (`geom::wall`, `DEFAULT_STEP`). Derin
  görünümde bile köşeli; 5 m yarıçapta 5 000 px/m'de ~24 piksel.
- **Eğriler** (spline, NURBS) kendi kirişleriyle.
- **GPU'da analitik eğriler:** belleği eğri sayısına bağlar, kiriş sayısına değil. İleride RM-15 ile.

## §4 Kanıt

**Sınamalar:**

- `kentos-style-core` `style::build::deep::tests::the_screen_s_curves_are_round_to_its_tolerance_everywhere`: 22 m'lik
  daire 0,5 mm toleransla 460–470 kiriş, her kirişin ortası daireden en çok tolerans kadar uzakta. Toleransla çizilen
  katmanın kenarları toleranssızınkinin 6 katından çok. Stil çekirdeğinin 85 sınaması geçti.
- `kentos-native-style` bütün sınamalar. Aynı işte `bound_legend` da onarıldı: CBS projesinde bağlı katman artık
  reddediliyor (a489301e7), sınama Harita (CAD) projesine geçti, kuralın kendi commit'inin öbür sınamaları gibi.
- `kentos-desktop`: görünüm, stilli sahne, kararlılık ve bölünmüş görünüm sınamalarından 57.
- `cargo check -p kentos-geometry-wasm` geçti.

**Gerçek pencere** (`.run/gui-evidence/render/rm09/`): Xvfb, evin DXF'i, 495 px/m (derin görünümün hemen altı).
Yay görünümün ortasında; çentik ve bağlama ADR 9220'nin yöntemiyle.

- `once-sonra-50-yay-495.png`, `bindirme-50-yay-495.png` (önce kırmızı, sonra yeşil).
- Önce yay 72 kirişle çiziliyor. Ölçüldü: önce ile sonra arasındaki yatay fark kiriş köşelerinde 0, aralarında 4
  piksele kadar; dikine 2,8 piksel, yani 5,3 mm × 495 px/m.

## Uygulama

- `crates/shared/style-core/src/style/build.rs`: `View::curve_tolerance` (eşitlik ve varsayılan dahil), `Cx`,
  `output_parts`.
- `crates/shared/style-core/src/style/build/deep.rs`: `everywhere`, sınama.
- `crates/shared/style-core/src/style/build/instances.rs`: tanım kaydında tolerans geri alınır.
- `apps/desktop/src/shell/viewport/state.rs`: `metres_a_unit` ve görünümün toleransı.
- `apps/desktop/src/cad/style/mod.rs`, `crates/native/style/tests/batches.rs`, `crates/wasm/geometry-wasm/src/store.rs`:
  `View` değerleri.
