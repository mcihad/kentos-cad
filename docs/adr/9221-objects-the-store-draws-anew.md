# ADR 9221: Deponun kendiliğinden yeniden çizdiği nesneler: düzenleme hemen görünür

- **Durum:** kabul edildi (2026-11-24). Plan: [plan/render.md](../../plan/render.md) RM-25.
- **İstek:** sahibin 24 Kasım bildirimi: “mimari çizerken pencere koyuyoruz ama görünmüyor, zoom yapınca ancak
  gözüküyor; kapılar da aynı şekilde, duvar bir tuhaf gözüküyor; tekerleği çevirmeden veya hareket ettirmeden göremiyoruz
  değişikliği”. Ardından: “Bu yazılımlar kaliteden ödün verilmemesi gereken yazılımlar.”
- **Sözleşmeler:** değişmedi.

## §1 Bulgu

**Yeniden üretim** (gerçek pencere, Xvfb, main'in ucu e6c3b29b6, `--modul mimari`):

1. Yeni proje açılır, Oda iki köşeyle çizilir.
2. Alt duvara Pencere tıklanır. Duvar kesilmez, pencere görünmez.
3. Üst duvara Kapı tıklanır. Kapının yayı ve kanadı çizilir, ama duvar kapının içinden geçer. Pencerenin yeri ancak bu
   ikinci düzenlemeyle belli belirsiz değişir.
4. Tekerlekle bir çentik içeri ve geri dönülür. Hiçbir şey değişmez.

Görüntüler: `.run/gui-evidence/render/mimari/oda-o/`.

**Kök neden:** çizim önbelleği (`StyledCache`) bir düzenlemeden sonra yalnız belgenin günlüğünün adını verdiği nesnelerin
parçalarını yeniden kuruyor (`doc.changes_since`).

- Duvarın boşluğu ise açıklıklarından türüyor (ADR 0341 §2, “Duvarın kendisi değişmez; boşluk her çizimde türer”).
- Geometri deposu açıklık eklenince duvarı yeniden birleştirip yeni boşluğuyla açıyor (`set_hosts` → `settle_walls`).
  Ama duvarın kaynağı değişmediği için günlükte duvar yok.
- Açıklık duvardan başka bir katmanda (örnek çizimde `kapi` ve `duvar`) ya da aynı katmanın başka bir parçasında
  olunca, duvarın parçası yeniden kurulmuyordu.
- Kesik ancak duvarların ayrıntısını değiştiren bir yakınlaştırmayla (ADR 0339 §4) bütün duvarlar yeniden kurulunca
  çıkıyordu.

**Aynı yolun başka kurbanları:**

- **Bir duvara bağlanan yeni duvar:** eski duvarın köşesi değişir, kaynağı değişmez.
- **Kaldırılan açıklık:** duvar kapanmalı.
- **Kolonlar:** bir kolonun görünüşü yanındaki duvarlardan alınır (ADR 8041 §2).

**Ne zamandan beri:** iki parçası bu daldan eski: parça parça kurma (28 Eylül, 446359e6e) ve depoda duvar birleşimleri
(ADR 0340, 18 Ekim).

- Main'in ucu ile RM-24 4. adımlı program aynı davranıyor; yani 4. adımın etkisi yok.
- Daldaki önceki adımların bunu görünür kılıp kılmadığı ölçülmedi. Main'in birleşmeden önceki ucu (d2fcda15b) derlenip
  aynı senaryoyla bakılmadı.

## §2 Karar

**Depo, kendiliğinden yeniden çizdiklerini sayar.** `Store` artık yeniden çizdiği nesnelerin kimliklerini sırayla tutuyor:
kaynağı değişmeden açılımı değişenler (`redrew`).

- Kayda giren yerler:
  - birleşimi ya da boşluğu değişen duvarlar (`settle_walls`'ın iki dalı);
  - görünüşünü yeniden alan kolonlar (`settle_columns`);
  - Tadilat görünümünün işaretleri değişince yeniden açılan duvar ve açıklıklar (`set_renovation`);
  - etiketi gelen ya da giden mahaller, kendi adlarını artık çizen ya da çizmeyenler (`set_tagged_spaces`);
  - çizim yazı tipi değişince yeniden ölçülen yazılar ve bloklar (`set_font`).
- Duvar görünümü (`set_wall_view`), bloklar ve zaman penceresi önbelleğin kendi anahtarlarıyla zaten yeniden kuruluyor.
  **Düzeltme (25 Kasım):** duvar görünümünde bu yalnız duvarlar için doğruydu. Ayrıntı değişince kapı, pencere ve
  görünümü izleyen öbür nesneler eski ayrıntıda kalıyordu. Depo artık onları da bu listeye yazıyor
  ([ADR 9224](9224-visual-conformance-edits-and-details.md)).
- `redraw_mark()` listenin sonunu verir. `redrawn_since(mark)` o işaretten beri yeniden çizilenleri verir.
- Liste nesne sayısının dört katını (en az 65 536) aşınca bırakılır. Ulaşamadığı işarete `None` döner, ve önbellek her
  şeyi yeniden kurar.

**Önbellek, deponun saydıklarını değişmiş sayar.** `StyledCache` her kurulumda deponun işaretini saklıyor (`redraw`).

- Sonraki kurulumda o işaretten beri yeniden çizilen her nesnenin parçası kirli sayılır: hem şimdiki yerinin parçası,
  hem eski parçası (`slot_part`).
- İşaret, “değişen yok, eskisini ver” kısa yolunun koşullarına da girdi.
- İlke: önbellek, baştan kurulan sahnenin aynısını çizer. Bir nesnenin çizimi başka nesnelerden türüyorsa, onları
  değiştiren düzenleme o nesneyi de yeniden kurar.

## §3 Kanıt

**Sınamalar:**

- **Depo:** `store::walls::a_wall_its_openings_or_another_wall_change_is_named_as_drawn_anew`. Listede olanlar:
  - başka katmandaki pencereyle kesilen duvar;
  - pencere kalkınca kapanan duvar;
  - ucuna yeni duvar bağlanan duvar;
  - yazı tipi değişince yeniden ölçülen yazı.

  Gelecekteki bir işaret `None`. Deponun 95 sınaması geçti.
- **Masaüstü, değişmez** (RM-16'nın 1. adımı):
  `style::scene_tests::every_edit_of_walls_and_openings_draws_as_a_new_cache_draws`. Beş düzenlemenin her birinden
  sonra önbelleğin sahnesi yeni bir önbelleğinkiyle aynı: kapı kaldırma, pencere ekleme, duvar taşıma, ucuna duvar
  bağlama, geri alma. Depo günlüğünün kullanımı kapatılınca ilk düzenlemede kırmızı.
- **Masaüstü:** `style::scene_tests::a_door_moved_along_its_wall_draws_the_wall_again`. Örnek çizimde
  (`fixtures/kcad/v2/openings.json`, duvar ve kapı ayrı katmanlarda) kapı duvar boyunca 1 m kayar; önbelleğin sahnesi
  yeni bir önbelleğinkiyle aynı. Önbellek düzeltmesi geri alınınca bu sınama kırmızı (“drawn as a new cache draws it”),
  düzeltmeyle yeşil. Stilli sahne sınamalarının 15'i geçti.

**Gerçek pencere** (`.run/gui-evidence/render/mimari/`):

- `once-sonra-pencere-hemen.png`: pencere tıklandığı anda. Önce duvar kesiksiz, pencere yok. Sonra duvar kesik,
  pencerenin kasa ve cam çizgileri görünüyor.
- `once-sonra-kapi-hemen.png`: kapı tıklandığı anda. Önce duvar kapının içinden geçiyor. Sonra duvar kesik; yay ve
  kanat yerinde.
- Tekerlek gerekmiyor.

## Uygulama

- `crates/shared/geometry-core/src/store/mod.rs`: `redrawn`, `redrawn_gone`, `REDRAWN_KEPT`, `redraw_mark`,
  `redrawn_since`, `redrew`.
- `crates/shared/geometry-core/src/store/wall_joins.rs`: `settle_walls`, `settle_columns` ve `set_renovation` yeniden
  çizdiklerini sayar.
- `crates/shared/geometry-core/src/store/mod.rs`: `set_font` ve `set_tagged_spaces` de.
- `crates/shared/geometry-core/src/store/walls.rs`: sınama.
- `apps/desktop/src/cad/style/scene.rs`: `StyledCache::redraw`, kısa yolun koşulu, kirli parçalar.
- `apps/desktop/src/cad/style/scene_tests.rs`: sınama ve `drawn_of`.
