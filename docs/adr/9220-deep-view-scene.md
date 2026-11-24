# ADR 9220: Derin görünümün sahnesi: yollar görünüm kutusunda bölünür

- **Durum:** kabul edildi (2026-11-24). Plan: [plan/render.md](../../plan/render.md) RM-24, 1. ve 2. adım.
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

### §2.4 Kalan (RM-24'ün 3. adımı)

- **Yaylar:** stilli yol daireleri en çok 72 kirişle çiziyor. Ekranda yarıçapı 263 pikseli geçen dairede sapma çeyrek
  pikseli aşıyor. Derin sahnede yalnız kutudaki açı aralığı, piksele göre kirişlenecek (RM-09 ile).
- **Sınır:** bundan sonra 10⁴'ten 10⁷ px/m'ye çıkar. Yaylar düzelmeden yükseltilmesi köşeli daireleri büyütürdü.
- **Diğer yollar:** ham CAD yolu (yalnız stil kitaplığı yokken) ve seçim vurgusu aynı kutuya sonra geçer.

## §3 Kanıt

**Sınamalar:**

- `kentos-geometry-core`, `geom::deep` (6):
  - kenar giriş ve çıkış noktasını alıyor;
  - içeriden çıkan kenar yalnız çıkışını alıyor;
  - kutunun dışındaki ya da içindeki yol değişmiyor;
  - halkanın kapanış kenarı bir kez bölünüyor;
  - eklenen noktalar kenarın üstünde (10⁻⁹ m);
  - köşeye değen kenar tek nokta alıyor.
- `kentos-style-core`, `style::build::deep` (2): uzun bir çizgi derin görünümde kutunun içinden ayrı parça olarak
  çiziliyor; kutuyu kesmeyen çizgi aynı.
- `kentos-desktop`, `a_deep_view_builds_the_objects_its_box_meets_split_at_its_edge`. 400 parsellik çizimde derin sahne
  yalnız kutunun değdiği parseli ve yolu kuruyor: 6 kenar. Kutu kenarı kesince 10 kenar. Yakınlaştırma, görünüm ve
  bölünmüş görünüm sınamalarından 36'sı geçti.

**Gerçek pencere** (`.run/gui-evidence/render/rm24/`, Xvfb, Sivas imar planı NCZ'si, demiryolu kuşağı):

- Ölçek durum çubuğundaki koordinatlardan okundu: 500 pikselde 0,798 m, yani **627 px/m**. 500 pikselde 0,050 m,
  yani **10 000 px/m**, sınır.
- İki derinlikte de derin sahnenin resmi düzenli sahneninkiyle aynı: 1 128 × 660 pikselde **0 farklı piksel**
  (`once-sonra-02-derin.png`, `once-sonra-03-sinir.png`).
- Bu sınırda kesinlik kazancı bir pikselin altında; görünür kazanç 3. adımda sınır yükselince.

## Uygulama

- `crates/shared/geometry-core/src/geom/deep.rs` (yeni), `geom/mod.rs`.
- `crates/shared/style-core/src/style/build.rs`: `View::deep`, `Cx::deep`, `output_parts`, bloklar ve uzak düzeyler.
- `crates/shared/style-core/src/style/build/deep.rs` (yeni).
- `apps/desktop/src/cad/style/scene.rs`: `StyledCache::set_only`, `keep`'teki kutu süzgeci, `meets`.
- `apps/desktop/src/shell/viewport/mod.rs`: `deep_styled`, `deep`, `DEEP_PX_PER_M`.
- `apps/desktop/src/shell/viewport/state.rs`: `deep_box`, `styled_in_use`.
- `apps/desktop/src/cad/style/mod.rs`, `crates/native/style/tests/batches.rs`: `View`'ın yeni alanı.
- `apps/desktop/src/cad/style/scene_tests.rs`: sınama.
