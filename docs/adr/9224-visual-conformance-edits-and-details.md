# ADR 9224: Görsel uygunluk: düzenleme ve duvar ayrıntısı, baştan çizilenle aynı

- **Durum:** kabul edildi (2026-11-25). Plan: [plan/render.md](../../plan/render.md) RM-16, 2. adım.
- **İstek:**
  - sahibin 24 Kasım bildirimi: pencere ve kapı ancak yakınlaştırınca görünüyordu ([ADR 9221](9221-objects-the-store-draws-anew.md));
  - “Bu yazılımlar kaliteden ödün verilmemesi gereken yazılımlar” ([ADR 9222](9222-quality-is-not-traded-for-speed.md));
  - 25 Kasım: “Devam et, asla durma, bitene kadar tüm maddeler.”
- **Sözleşmeler:** değişmedi. Geometri deposuna yalnız okuma yöntemi eklendi (`Store::following_view`).

## §1 Bulgu

### §1.1 Duvar ayrıntısı değişince kapı ve pencere eski ayrıntıda kalıyordu

Bu adımın uygunluk sınaması, yazılır yazılmaz yeni bir hata buldu.

- **Duvarın ayrıntısı yakınlığa göre değişir** ([ADR 0339](0339-walls-model-styles-and-tool.md) §4): 1:30'a kadar ayrıntılı,
  1:75'e kadar standart, ötesi şematik. ±%10'luk gecikmeyle seçilir.
- **Depo yeniden açar:** ayrıntı değişince geometri deposu görünümü izleyen her nesneyi yeniden açıyor
  (`Store::set_wall_view`, `follows_view`). Bunlar duvarlar, kapı ve pencereler, mahaller, akslar, kolonlar,
  kirişler, çatılar, korkuluklar, merdivenler, rampalar, etiketler, görünüş işaretleri ve mobilyadır.
- **Önbellek ise yalnız duvarları yeniden kuruyordu:** çizim önbelleği (`StyledCache`) yalnız duvar nesnelerinin
  parçalarını kirli sayıyordu.
- **Mimari'de sonuç:** katman anahtarı ([ADR 8008](8008-ek4-layer-key.md)) her türü kendi katmanına koyuyor
  (MM-DUVA-…, MM-KAPI-GENEL, MM-PENC-…). Kapı ve pencere, duvarı yeni ayrıntıya geçmişken eski ayrıntının sövesiyle
  çiziliyordu. Bu, başka bir düzenleme ya da yakınlaştırma o parçayı yeniden kurana dek sürüyordu.
- **ADR 9221'deki yanlış:** o ADR “Duvar görünümü önbelleğin kendi anahtarlarıyla zaten yeniden kuruluyor” diyordu. Bu
  yalnız duvarlar için doğruydu.

**Ölçüm:**

- **Başsız, ikiz uygulama** (§2.3):
  - 1:20'den 1:120'ye uzaklaşmada pencerenin sövelerinde 48 piksel eşsiz.
  - Ayrı açılan ikizde ilk görünümden ayrıntı değişince, pencerede 52, kapıda 64 piksel.
- **Gerçek pencere:** 1:50'den dört çentik dışarı (1:166) çıkılınca önce ile sonra arasında 77 piksel fark var, hepsi
  açıklıkların sövelerinde.

### §1.2 Xvfb'de tuşlar

24 Kasım'da Escape ve Enter araçlara ulaşmıyordu. Neden programda değil, senaryodaydı: senaryo `fit KentOS Mimari`
satırıyla başlamıyordu. Xvfb'de pencere yöneticisi yok, klavyeyi pencereye `fit` veriyor
([README](../../scripts/usage/real/README.md)). Bu adımın senaryosunda koordinat yazmak, Enter, Escape, Delete ve
Ctrl+Z çalışıyor.

## §2 Karar

### §2.1 Depo, ayrıntıyla yeniden açtıklarını sayar

`Store::set_wall_view`, yeniden açtığı her nesneyi yeniden çizilenler listesine yazar (`redrew`, ADR 9221). Önbellek
ADR 9221'in yoluyla bunların hepsinin parçasını yeniden kurar: duvarı, kapısını, penceresini, mahal etiketini,
kolonunu.

### §2.2 Kendi ayrıntısıyla çizen görünüm de hepsini kurar

Bölünmüş görünümün öbür yarısı duvarları kendi ayrıntısında çizer
([ADR 0696](0696-model-wall-detail-and-dense-hairlines.md), 29 Ekim eki). Bunun için depoyu
değiştirmez, çizimi depodan o görünüm için ister. Önbellek kendi `wall_view`'ı değişince artık yalnız duvarları
değil, deponun görünümü izleyen bütün nesnelerini kirli sayar (`Store::following_view`).

### §2.3 İkiz uygulama sınaması (masaüstü, GPU)

**Koşum:** `perf::frame::Harness` çekirdeğin sınama takımına taşındı (`window_testing.rs`). Pencere açmadan,
uygulamanın kendi resmini çizen başsız koşum bu. Modül paketleri de kullanabilsin diye taşındı.

**Denetim** (`render_testing.rs`, `Edit::check`):

1. Bir uygulama çizimi açar ve çerçeveler. Araçlarla çizilenler, alan boyutunu aldıktan sonra çizilir.
2. Resimler kurulur; ardından kullanıcının düzenlemesi ya da kamera değişikliği yapılır.
3. Araç bırakılır (iki Escape). İmleç çizimin dışına alınır ve resim hemen alınır.
4. İkinci bir uygulama düzenlenmiş belgeyi baştan açar, aynı kamerayla çizer.
5. İki çizim alanı piksel piksel karşılaştırılır. Bir piksel, öbür resimde bir piksel yakınında, her kanalı en çok 40
   ayrı bir rengi varsa eşleşmiş sayılır.

**Kırmızıysa ek resimler** (hatanın türünü söyler):

- tekerlek gidip geldikten sonraki resim (kullanıcının görmek için yaptığı);
- aynı uygulamada stilli sahneler baştan kurulunca (`Viewport::forget_styled`) çıkan resim.

Resimler `.run/gui-evidence/render/rm16/`'ya yazılır. `KENTOS_GPU_TESTS=1` ile koşar, renderer'ın GPU sınamaları gibi.

**Durumlar:**

- **Çekirdek** (CAD araçları): çizgi, çizgi silme, taşıma, geri alma, daire, yazı (Yazı'nın alanından), derin
  görünümde (1 200 px/m) çizgi.
- **Mimari paketi**, katman anahtarı açık: duvar, pencere, kapı, kapı silme, iç duvar taşıma, kapının geri alınması,
  derin görünümde pencere, 1:120 → 1:20 yakınlaşma, 1:20 → 1:120 uzaklaşma.

### §2.4 Gerçek pencere senaryosu

`fixtures/interaction/real/duzenleme-hemen-gorunur.txt`, Yeni proje (Mimari) üzerinde şu düzenlemeleri yapar:

1. Oda (10 × 7 m, yazılan köşelerle);
2. pencere;
3. kapı;
4. kapıyı Delete ile silme;
5. Ctrl+Z;
6. uçları bağlanan bölme duvarı;
7. düz çizgi.

Her düzenlemenin resmi hemen alınır; tekerlek bir çentik içeri girip döndükten sonra bir resim daha alınır.
`scripts/usage/real/same.py` ikisini karşılaştırır. Son iki resim (b1 ve b2), duvar ayrıntısının değiştiği uzaklaşma
ve yakınlaşmadır.

## §3 Kanıt

**Sınamalar:**

- `kentos-geometry-core`
  `store::walls::another_detail_names_every_object_drawn_for_it_as_drawn_anew`:
  - ayrıntı değişince duvar ve penceresi listede, çizgi listede değil;
  - aynı görünüm yeniden verilince liste boş.

  Deponun 96 sınaması geçti.
- **Çekirdek:** `kentos-desktop` `render_conformance_tests::an_edit_with_cad_s_tools_shows_at_once_as_a_fresh_app_draws_it`.
  7 durum, hepsinde tazeden farklı piksel 0.
- **Mimari:** `kentos-desktop-arch`
  `render_conformance_tests::an_edit_of_walls_doors_and_windows_shows_at_once_as_a_fresh_app_draws_it`. 9 durumun
  hepsinde 0.
- **Düzeltmesiz çalıştırma (sınamanın gücü):**
  - ADR 9221'in günlüğü kapatılınca 7 Mimari durumundan 6'sı kırmızı: pencere 4 779, kapı 4 645, derin pencere 17 396
    piksel.
  - Bu ADR'nin düzeltmesi kapatılınca pencere, kapı ve uzaklaşma kırmızı (48–64 piksel).
- **Masaüstü:** stilli sahne, görünüm, bölünmüş görünüm, duvar ve açıklık sınamalarından 78'i geçti.

**Gerçek pencere** (`.run/gui-evidence/render/rm16/`; önce: main'in ucu d50952990, sonra: bu ADR):

- Altı düzenlemenin hepsinde, iki ikilide de hemen alınan resim ile tekerlekten sonraki resim aynı (0 piksel). ADR
  9221 main'de bunu zaten sağlıyordu.
- `once-sonra-b1-uzak.png`: 1:166'da önce kapı ve pencere standart ayrıntının çift sövesiyle, duvarları şematik.
  Sonra hepsi şematik.
- `once-sonra-b1-pencere.png` ve `once-sonra-b1-kapi.png`: 12 kat büyütülmüş hâli.

## Uygulama

- `crates/shared/geometry-core/src/store/mod.rs`: `set_wall_view` yeniden açtıklarını sayar; `following_view`.
- `crates/shared/geometry-core/src/store/walls.rs`: sınama.
- `apps/desktop/src/cad/style/scene.rs`: kendi duvar görünümü değişince görünümü izleyen bütün nesnelerin parçaları.
- `apps/desktop/src/shell/viewport/state.rs`: `forget_styled` (yalnız sınama takımında).
- `apps/desktop/src/shell/window_testing.rs`: başsız pencere koşumu (`perf/frame.rs`'ten taşındı).
- `apps/desktop/src/shell/render_testing.rs`: ikiz uygulama denetimi.
- `apps/desktop/src/tests/render_conformance_tests.rs`, `apps/desktop-arch/src/tests/render_conformance_tests.rs`:
  durumlar.
- `fixtures/interaction/real/duzenleme-hemen-gorunur.txt`, `scripts/usage/real/same.py`: gerçek pencere.
