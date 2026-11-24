# ADR 9220: Derin görünümün sahnesi: yollar görünüm kutusunda bölünür

- **Durum:** kabul edildi (2026-11-24). Plan: [plan/render.md](../../plan/render.md) RM-24, 1.–4. adım (bitti).
- **İstek:** sahibin 23 Kasım isteği (“AutoCAD kusursuz büyütebiliyor”, “çapraz çizgiler çok yakında testere gibi”).
- **Sözleşmeler:** değişmedi.

## §1 Bulgu

**Hata kaynağı:** GPU bir noktayı kameraya göre farkı olarak, iki float32 parçayla alıyor
(`crates/render/wgpu/src/precision.rs`). Fark, noktanın kameraya uzaklığının yaklaşık 2⁻²⁴'ü kadar doğru. Bir kenar
kameranın yanında ancak uçları kadar doğru: uçları uzaktaysa kenar görünümde kayıyor.

- **Ham CAD yolu:** kenarları 128 m'de bölüyor. Bugünkü 10⁴ px/m sınırı buna göre seçilmiş.
- **Stilli yol:** uygulamada bütün katmanlar buradan çiziliyor, ama kenarları hiç bölmüyor. 2 km'lik bir kenar
  10 000 px/m'de 0,6 piksel kayıyor. 10⁶ px/m'de bu onlarca piksel olurdu.
- **GPU'da çözülemiyor:** uzak uçtan hesaplanan her değer aynı hatayı taşıyor. WGSL'de iki-float hilesi sürücünün
  işlem sırasına bağlı.

## §2 Karar

### §2.1 Kenar kutunun sınırında bölünür, kesilmez

`kentos_geometry_core::geom::deep::split_at_box`, bir yolun (ya da halkanın) kenarına kutunun sınırını kestiği
noktaları ekler. Kutudaki parçanın iki ucu da kutuda kalır; parça kutu kadar doğru olur.

- Eklenen noktalar kenarın üstünde. Yolun uzunluğu, çizgi tipinin kesik evresi, işaret aralığı ve dolgu aynı kalır.
  Kesmek kesik evresini ve işaretleri kaydırırdı.
- Dolgunun iç üçgenleri ortak köşeleri paylaşıyor. Uzak köşeli bir üçgen aralık açmaz, kenarı görünümde iki komşu
  üçgende aynı hesaplanır.
- Kutuyu kesmeyen yol olduğu gibi kalır; kopya yapılmaz.

### §2.2 Stil çekirdeğinin derin görünümü

`View::deep`, görünümün kutusunu taşır; yoksa çizim eskisi gibi çizilir. Kutu verildiğinde:

- her nesnenin yolları ve halkaları bölünür;
- bloklar tanımdan yerleştirilerek değil, açılarak çizilir;
- uzak görünüm düzeyleri (`curve_lod`, `line_lod`) kapanır.

### §2.3 Masaüstünün derin sahnesi

- **Ne zaman:** görünüm 500 px/m'yi geçince (`DEEP_PX_PER_M`, çizim birimiyle çarpılır) derin sahne kurulur. 250'nin
  altına inince düzenli sahneye dönülür; aradaki pay, eşikte gidip gelmeyi önler.
- **Kutu:** görünüm ve her yana üç katı (`construction_clip`, yardımcı çizgilerinki). Görünüm kutudan çıkınca ya da
  20 kat içeri girince yenisi kurulur.
- **İkinci önbellek:** `deep_styled`, yalnız kutusu bu kutuyla kesişen nesneleri kurar. 500 px/m'de kutu birkaç düzine
  metre; az nesne tutar ve hemen kurulur. Düzenli sahne önbellekte kalır, uzaklaşınca yeniden kurulmaz.
- **Süzgeç ayrıntısı:** depoda kutusu olmayan nesne kurulur. Zaman penceresinin gizli kümesine dokunulmaz; dokunulsaydı
  kirli parça denetimi her karede yeniden kurdururdu.

### §2.4 Daireler ve yaylar kutuda yuvarlak (3. adım)

**Sorun:** stilli yol daireleri en çok 72 kirişle çiziyor (`OUTLINE_SEGMENTS`, `FINE_CHORDS`). Ekranda yarıçapı 263
pikseli geçen dairede sapma çeyrek pikseli aşıyor.

**Karar:**

- Derin sahnede daire ve yay, `geom::deep::arc_in_box` ile çizilir. Kutunun içinde kirişler çeyrek pikselden fazla
  sapmaz (`View::deep`'in toleransı, `DEEP_TOLERANCE_PX`). Dışında eskisi gibi 72 kiriş bir tur. Kutunun sınırını
  kestiği yerde bir nokta vardır; her nokta yayın üstündedir.
- Kutuyu kesen bölüm kısa olduğundan nokta sayısı az. Yine de en çok 65 536; daha çoğunu isteyen kutuda adımlar büyür.
- Tolerans o anki ölçekten. Ölçek kutunun kurulduğu ölçeğin iki katını aşınca ya da yarısına inince derin sahne
  yeniden kurulur.

**Sınır:** `MAX_SCALE` 10⁴ → **10⁵ px/m** (10 µm/piksel, 2 mm 200 piksel).

- Seçim vurgusu ve ham CAD yolu (yalnız stil kitaplığı yokken) hâlâ 128 m'de kesiliyor. 10⁵'te hataları 0,4 pikselin
  altında (64 m × 2⁻²⁴ × 10⁵).
- Kesinlik sınamaları bu yolların kendi sınırına (10⁴) bağlandı. 10⁷ onlar da kutuya geçince gelir.

### §2.5 Seçim vurgusu, bombeli kenarlar, elipsler; sınır 10⁷ (4. adım)

**Vurgu.** Seçim ve üzerinde durulan nesnenin vurgusu (`scene::build_highlight`) derin görünümde aynı kutuyu alır:

- çizicinin `Builder`'ı her kenarı ve dolgu halkasını kutunun sınırında böler (`split_at_box`), sonra 128 m'lik
  parçalara ayırır;
- daire, yay, bombeli kenar ve elips kutuda çeyrek piksellik kirişle çizilir;
- vurgunun anahtarı kutuyu ve toleransı tutar: kutu değişince vurgu yeniden kurulur, değişmeyince önbellekte kalır.

Önce vurgu, kirişlenmiş çizimden ayrı yerde duruyordu: ~1 640 px/m'de 12 piksel, 10⁵'te yüzlerce piksel.

**Bombeli kenarlar ve elipsler.**

- Stil çekirdeği bombeli kenarlı çoklu çizgi ve alanı (`deep::refined`) yaylarını noktaya çevirerek çizer, sonra keser.
  Noktalar `geom::deep::bulge_path_in_box`'tan gelir; köşeler aynı kalır.
- CAD çizgi tipinin kesik evresi kaynak eğri üzerinden ölçüldüğü için (ADR 1122) kesikler kaymaz.
- Geniş çoklu çizgi (bant) köşelerine bağlı olduğundan olduğu gibi kalır.
- Elips `geom::deep::ellipse_in_box` ile çizilir: daireninkiyle aynı yol, yarı büyük eksenin kirişiyle.

**Kaba kirişin kutuyu kesmesi düzeltildi.** 3. adımın `arc_in_box`'ı kutunun dışında kalan yay parçasını kaba çiziyordu.

- Sorun: kutuya iç yüzünden yakın geçen bir yayın 5°'lik kirişi kutunun içinden geçebiliyordu. Bu, görünümde olmayan
  yayın yerine görünümde düz bir çizgi demek.
- Çözüm: yay artık kutunun bir kaba kirişin sapmasının 1,25 katı kadar genişletilmiş hâlinde de ince çizilir. Dışında
  kalan kaba kirişler kutuya giremez (sınama `a_curve_passing_beside_the_box_never_has_a_chord_across_it`).
- Seyreltme yalnız ince parçalara uygulanır.

**Sınır:** `MAX_SCALE` 10⁵ → **10⁷ px/m** (0,1 µm/piksel; 2 mm 20 000 piksel).

- Kalan hata iki float32 parçanın kendi tabanı: noktanın sahne kökenine uzaklığının 2⁻⁴⁸'i.
- Kökenin verinin yanında (belgenin çapası) olduğu olağan durumda 10⁷'de 10⁻⁴ pikselin altında.
- Köken 4 400 km uzakta, örneğin (0, 0) ise 0,3 piksele kadar.
- Kesinlik sınamaları bu tabanı ve ekran pikselinin kendi float32'sini (2⁻²⁴) sınır olarak taşıyor (`bound`). Yakın
  kökenle sınır binde bir piksel.
- 128 m parçalarının sınamaları kendi sınırlarında (10⁴) kaldı. Ham CAD yolu uygulamada çizim için kullanılmıyor; her
  alan stil kitaplığıyla çiziyor (`view/area.rs`, `split_view.rs`).

**Kalan:**

- Olağan yakınlıkta (500 px/m'nin altında) büyük daireler hâlâ 72 kiriş: RM-09.
- Eğriler (spline, NURBS) derin sahnede kendi kirişleriyle çizilip kutuda bölünüyor, kutuda yuvarlanmıyor.

## §3 Kanıt

**Sınamalar:**

- `kentos-geometry-core`, `geom::deep` (10):
  - kenar giriş ve çıkış noktasını alıyor;
  - içeriden çıkan kenar yalnız çıkışını alıyor;
  - kutunun dışındaki ya da içindeki yol değişmiyor;
  - halkanın kapanış kenarı bir kez bölünüyor;
  - eklenen noktalar kenarın üstünde (10⁻⁹ m);
  - köşeye değen kenar tek nokta alıyor;
  - kutudan geçen 50 m'lik daire kutuda 0,1 mm içinde, dışında 72 kirişlik kabalıkta; kutu sınırında iki noktası var;
  - yay uçlarını ve yönünü koruyor;
  - kutunun uzağındaki daire eskisi gibi 72 kiriş;
  - çok küçük toleransta nokta sayısı sınırda kalıyor.
- `kentos-style-core`, `style::build::deep` (3): uzun bir çizgi derin görünümde kutunun içinden ayrı parça olarak
  çiziliyor; kutuyu kesmeyen çizgi aynı. Derin sahnenin dairesi kutuda 72'den çok noktalı; kutunun uzağındaki yay
  eski adımında.
- `kentos-render-wgpu`: kesinlik sınamaları (8) ve gerçek aygıtta GPU sınamaları (8) geçti; ham yolunkiler 10⁴'e
  bağlı.
- `kentos-desktop`, `a_deep_view_builds_the_objects_its_box_meets_split_at_its_edge`. 400 parsellik çizimde derin sahne
  yalnız kutunun değdiği parseli ve yolu kuruyor: 6 kenar. Kutu kenarı kesince 10 kenar. Yakınlaştırma, görünüm ve
  bölünmüş görünüm sınamalarından 36'sı geçti.

**Gerçek pencere** (`.run/gui-evidence/render/rm24/`, Xvfb, Sivas imar planı NCZ'si, demiryolu kuşağı):

- Ölçek durum çubuğundaki koordinatlardan okundu: 500 pikselde 0,798 m, yani **627 px/m**. 500 pikselde 0,050 m,
  yani **10 000 px/m**, sınır.
- İki derinlikte de derin sahnenin resmi düzenli sahneninkiyle aynı: 1 128 × 660 pikselde **0 farklı piksel**
  (`once-sonra-02-derin.png`, `once-sonra-03-sinir.png`).
- Bu sınırda kesinlik kazancı bir pikselin altında; görünür kazanç 3. adımda sınır yükselince.
- **Daire** (`.run/gui-evidence/render/rm24-daire/`): yapılaşma koşulu dairelerinden biri (yarıçap ~22 m), tepesine
  iki aşamada yakınlaşıldı.
  - ~650 px/m'de önce iki kirişin köşede kırılması, sonra pürüzsüz yay (`orta/once-sonra-05-daire-orta.png`).
  - 10⁴ px/m'de önce kiriş, sonra gerçek daire. Aralarındaki 1,6 cm, kirişin bu noktadaki sapması (en çok 2,1 cm).

**4. adım** (`.run/gui-evidence/render/rm24b/`): Xvfb, iki katlı evin DXF'i (metre). Terasın 5,57 m'lik bombeli yayı
(PROIEZIONI, ortası 2 474,819 / 1 401,217) üç aşamada yakınlaştırıldı. Önce: main'in ucu (3. adım). Sonra: 4. adım.

- **1 644 px/m** (ölçek durum çubuğundan): `once-sonra-20-yay.png`, `bindirme-20-yay.png`. Önce yay 72 kirişle çiziliyor:
  iki köşe görünür ve ortada kiriş yaydan 11–12 piksel içeride. Ölçüldü: önce ile sonra arasındaki yatay fark uçlarda 1,
  ortada 12 piksel; 5,3 mm × 1 644 px/m / sin 46°.
- **Aynı ölçek, yay seçili** (`once-sonra-21-secim.png`): önce vurgu çizimden ayrı, sonra tam üstünde.
- **~1,1·10⁵ px/m** (`once-sonra-22-derin.png`): önce vurgu ile çizim yüzlerce piksel ayrı; eski program 10⁵'te
  durmuş. Sonra tek çizgi.
- **~9,9·10⁶ px/m** (`once-sonra-23-en-derin.png`): eski program yerinde kaldı (çizgi iki aşamada da x = 595,5'te).
  Yeni program bir kez daha ~90 kat derine indi; çizgi bağlantı noktasından 148 piksel uzaklaştı. Vurgu yine çizginin
  üstünde.

**4. adımın sınamaları:**

- `kentos-geometry-core` `geom::deep` (13): yeni üçü yayın yanından geçince kirişin kutuyu kesmemesi (daire ve elips),
  elipsin kutuda yuvarlak ve eğrisinin üstünde olması, bombeli yolun köşelerini koruması.
- `kentos-style-core` `style::build::deep` (5): elips ve bombeli alan kutuda yuvarlak; geniş çoklu çizgi olduğu gibi;
  bombeli çoklu çizgi derin görünümde ince çiziliyor.
- `kentos-render-wgpu`:
  - `tests/scene.rs` `a_deep_view_s_highlight_is_split_at_its_box_and_round_in_it`; derin vurgunun çok çekirdekte kurulanı
    tek çekirdektekiyle aynı.
  - `tests/precision.rs` `a_deep_view_s_highlight_stays_on_its_exact_place_at_the_deepest_zoom`: 20 km'lik çizgiler
    10⁷'de 0,05 pikselin içinde. 128 m parçalarıyla 1 pikselden çok sapıyor.
  - Bütün hedefler ve GPU sınamaları geçti.
- `kentos-desktop`: `viewport::highlight::tests::a_deep_view_s_highlight_is_split_at_its_box`.

## Uygulama

- `crates/shared/geometry-core/src/geom/deep.rs` (yeni), `geom/mod.rs`.
- `crates/shared/style-core/src/style/build.rs`: `View::deep`, `Cx::deep`, `output_parts`, bloklar ve uzak düzeyler.
- `crates/shared/style-core/src/style/build/deep.rs` (yeni).
- `apps/desktop/src/cad/style/scene.rs`: `StyledCache::set_only`, `keep`'teki kutu süzgeci, `meets`.
- `apps/desktop/src/shell/viewport/mod.rs`: `deep_styled`, `deep`, `DEEP_PX_PER_M`.
- `apps/desktop/src/shell/viewport/state.rs`: `deep_box`, `styled_in_use`.
- `apps/desktop/src/cad/style/mod.rs`, `crates/native/style/tests/batches.rs`: `View`'ın yeni alanı.
- `apps/desktop/src/cad/style/scene_tests.rs`: sınama.
- 3. adım: `geom::deep::{arc_in_box, step_for, COARSE_STEP, MOST_POINTS}`; stil çekirdeğinin `Deep` (kutu ve
  tolerans) ve `build/deep.rs`'in `curve`'ü; masaüstünde `DEEP_TOLERANCE_PX` ve ölçekle bayatlama;
  `crates/render/wgpu/src/camera.rs` (`MAX_SCALE`); `tests/precision.rs` ve `tests/gpu.rs` (ham yolun sınırı).
- 4. adım:
  - `geom::deep::{Deep, ellipse_in_box, bulge_path_in_box}`, iç `Conic` ve `in_box`. `Deep` buraya taşındı; stil
    çekirdeği onu yeniden veriyor.
  - Stil çekirdeği `build/deep.rs`: `curve`'ün elipsi, `refined`; `output_parts`'ın inceltilmiş şekli.
  - `crates/render/wgpu/src/scene.rs`: `Builder::deep`, `segment` ve `cut`, `fill`'in halkaları;
    `build_highlight(_parallel)`'ın `deep`'i, `highlight_one`, `deep_ellipse`.
  - `camera.rs` (`MAX_SCALE` 10⁷); `tests/{precision,scene}.rs`.
  - Masaüstü: `viewport/highlight.rs` (anahtarın `deep`'i, `Viewport::deep`) ve `cad/sheet_model.rs`.
