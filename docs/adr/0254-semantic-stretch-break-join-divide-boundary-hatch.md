# ADR 0254: Anlamsal Esnet, Kır, Birleştir, Böl, Alan oluştur ve Tarama

- **Durum:** kabul edildi (2026-10-13). [plan/cad.md](../../plan/cad.md) §5 `CAD-CMD-01`, `05`, `06`. Kalıp Buda'nınkidir
  ([ADR 0250](0250-semantic-trim-extend-fillet-chamfer.md)); çağrıların ortak protokolü [ADR 0253](0253-command-protocol.md).
  Kapsamı ben belirledim (sahibin sözü: “CAD-CMD adımlarını tamamla”, “bunu iyi yap”).
- **Bağlam belgesi:** ADR 0047 (değiştirme araçları), 0057 (Böl), 0062 ve 0186 (Tarama), 0065 ve 0069 (İçine tıklayarak
  alan), 0160 (Topolojik düzenleme), 0161 (Zincir), 0165 §2 (tel üzerinde metre).
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

Esnet, Kır, Birleştir, Böl, İçine tıklayarak alan ve Tarama sonucu kendileri hesaplayıp hazır geometri yazıyordu:
`cad.entities.edit`'in `stretch`, `break`, `join` işlemleriyle ya da `cad.entities.create`'in `divide`, `boundary`,
`hatch` işlemleriyle. Python ve ajan aynı işi yapmak için geometriyi kendileri bulmak zorundaydı (TODOS.md `CMD-01`). Esnet,
İçine tıklayarak alan, Tarama ve Zincir girdisini örtük görünüm durumundan da alıyordu: görünümdeki görünen nesneler.

## Karar

### 1. Altı yeni ürün komutu

| Komut | Adım | Girdi |
|---|---|---|
| `cad.entities.stretch` v1 | Esnet | `window {min, max}`, `dx`, `dy`, `uids?`, `topology?`, `expectedRevision?` |
| `cad.entities.break` v1 | Kır | `uid`, `first`, `second?`, `expectedRevision?` |
| `cad.entities.join` v1 | Birleştir | `uids`, `tolerance?`, `chain?`, `expectedRevision?` |
| `cad.entities.divide` v1 | Böl | `uid`, `division`, `fromEnd?`, `layerId`, `expectedRevision?` |
| `cad.entities.boundary` v1 | Alan oluştur | `at`, `layerId`, `boundaries?`, `boundaryLayer?`, `islands?`, `expectedRevision?` |
| `cad.entities.hatch` v1 | Tarama | `region`, `pattern`, `layerId`, `expectedRevision?` |

- Tel üzerinde uzunluklar ve koordinatlar metredir (ADR 0165 §2).
- Her komutun planı yazmadan sonucu verir; protokolün planıyla yazma tam o planı yazar (ADR 0253 §5).
- Yazma, eski düşük düzey işlemlerden geçer: Esnet, Kır ve Birleştir `cad.entities.edit`'in `stretch`, `break` ve `join`
  işlemleriyle; Böl, Alan oluştur ve Tarama `cad.entities.create`'in `divide`, `boundary` ve `hatch` işlemleriyle. Düşük düzey
  işlemler kalır (`CAD-CMD-01`: “mevcut düşük seviyeli geometri yaması komutlarını koru”).
- Kır ve Birleştir'in çıktısı ve planı `cad.entities.edit`'inkidir (`EntitiesEdited`, `EntitiesEditPlan`). Böl, Alan
  oluştur ve Tarama'nınki `cad.entities.create`'inkidir (`EntitiesCreated`, `EntitiesCreatePlan`). Esnet'in kendi çıktısı
  var: esneyenlerle komşular ayrı söylenir.
- **Sınırlar** (`boundaries`, Buda'nın `TrimBoundaries`'i, ADR 0250 §2):
  - `all` (varsayılan): görünen bütün nesneler (katmanı açık, katman süzgecinden geçen, zaman dilimindeki);
  - `area {min, max}`: dikdörtgene değen görünen nesneler; araç kendi görünümünü verir;
  - `objects {uids}`: verilen nesneler, görünsün ya da görünmesin.

### 2. Esnet (`cad.entities.stretch`)

- **Pencere** (`window`): içindeki köşeler `dx`, `dy` kadar taşınır, öbürleri kalır. Yay bükümünü korur (çekirdeğin
  `stretch_entity`'si).
- **Nesneler:**
  - `uids` verilirse yalnız onlar. Penceresinde köşesi olmayan `outside_window` uyarısıyla atlanır.
  - Verilmezse pencereye değen görünen nesneler; köşesi pencerede olmayanlar sessizce dışarıda kalır.
  - Kilitli katmandakiler `locked_skipped` uyarısıyla atlanır. Hiçbiri kalmazsa ret: `nothing_to_stretch`; ret, kilitli
    olduğu için dışarıda kalan nesnelerin kimliklerini taşır (`uids`), araç sayılarını söyler (“3 nesne kilitli katmanda olduğu
    için atlandı.”).
- **Topoloji** (`topology {points}`, ADR 0160 §3): verilirse taşınan köşeleri paylaşan görünen komşular aynı adımda izler.
  - `points`: noktalar da köşedir.
  - Kilitli komşu `neighbour_locked`, geçersiz kalacak komşu `neighbour_invalid` uyarısıyla değişmez.
  - Esnet köşeleri yalnız taşır: komşunun köşe sayısı değişmez, geçersizlik bu yüzden beklenmez (çekirdeğin kuralı köşe
    sayısına bakar). Uyarı, kural değişirse diye yazılır; yetenek testi bunu gerekçesiyle söyler.
  - Komşular yuvalarının sırasıyla bulunur ve yazılır (ev sahibinin deposu ile başsız sunucunun deposu aynı cevabı verir);
    büküm almayan yolun sıfır bükümleri yazılmaz.
- **Yer değiştirme sıfırsa** hiçbir şey yazılmaz; sonuç tamamlandı, listeler boş.
- **Çıktı** `EntitiesStretched {stretched, followed, revision}`; plan `EntitiesStretchPlan`: nesnelerin olacakları hâlleri.

### 3. Kır (`cad.entities.break`)

- `first` ile `second` arasındaki kısım kalkar. `second` yoksa ya da `first`e eşitse nesne `first`te ikiye bölünür.
- **Türler:** çizgi, çoklu çizgi, alan, yay, daire, elips, yardımcı çizgi, ışın. Öbürleri `not_breakable`; delikli alan da
  (parçalanırken deliklerini yitirirdi).
- **Parçalar** çekirdeğin `break_entity`'sindendir. Nesnenin yerini ilk parça alır, öbürleri ondan yeni nesnelerdir
  (Buda'nın kuralı, ADR 0250 §2). Çekirdeğin söylediği sorun `break_failed`'dir; ileti çekirdeğin sözüdür.

### 4. Birleştir (`cad.entities.join`)

- **Nesneler** (`uids`): uç uca buluşan çizgi, yay ve açık çoklu çizgiler her zincir için tek nesne olur.
  - Kapanan zincir kapalı alandır.
  - Zincir, ilk nesnesinin yerini, kalıcı kimliğini ve verisini alır; öbürleri silinir.
- **Tolerans** (`tolerance`): uçların aralığı, metre; varsayılan 0,001. Sıfırdan büyük sonlu bir sayı olmalı
  (`invalid_distance`).
- **Kilitli** katmandakiler `locked_skipped` uyarısıyla dışarıda kalır.
- **Hiç zincir kurulamazsa** ret: `nothing_to_join`. Çekirdeğin reddi `join_failed`'dir.
- **Zincir** (`chain`, ADR 0161 §2): verilirse `uids` tek nesnedir, tohum. Tohum uç uca bağlı nesnelerle tek nesne olur.
  - Zincir şu nesneler arasında yürür: sınırlar (`all`, `area`, `objects`).
  - Zincir şu yerlerde durur: serbest uç, kavşak, kilitli nesne ya da başa dönüş.
  - Tohum çizgi, yay ya da açık çoklu çizgi değilse ret: `not_joinable`.
  - Zincir iki nesneden kısaysa ret: `nothing_to_join`.
  - Kilitli nesnede durursa uyarı: `chain_stopped`.
  - Tohum zincirin nesnesi olur: yerini, kimliğini ve verisini o verir.

### 5. Böl (`cad.entities.divide`)

- **Bölme** (`division`):
  - `{kind: "parts", count}`: eşit parçalar (2–10 000, `invalid_count`);
  - `{kind: "step", length}`: aralık; sıfırdan büyük, sonlu (`invalid_distance`).
- **Başlangıç:** noktalar nesnenin başından ölçülür; `fromEnd: true` ise sonundan. Araç tıklamaya yakın ucu verir.
- **Türler:** çizgi, çoklu çizgi, alan, yay, daire, elips, eğri. Öbürleri `not_divisible`.
- **Noktalar** çekirdeğin `division_points`'idir; elipsinkiler eğrinin üstündedir.
  - Aralık nesneden uzunsa ret: `no_points`.
  - 10 000'den çok nokta ret: `too_many_points`.
- **Katman:** noktalar `layerId`'ye `cad.entities.create`'in kurallarıyla yazılır (yok, grup, kilitli; gizliye uyarı).

### 6. Alan oluştur (`cad.entities.boundary`)

- `at`'yi çevreleyen ve sınırların çizgilerinin kapattığı bölge alan olur (çekirdeğin `FaceIndex`'i).
- **Adalar** (`islands`, varsayılan açık): içindeki kapalı gruplar delik olur.
- **Sınır katmanı** (`boundaryLayer`): verilirse yalnız o katmanın çizgileri sayılır.
- **Ret:** bölge yoksa `not_enclosed`.
- **Yazma:** alan `layerId`'ye `cad.entities.create`'in kurallarıyla yazılır.
- **Halkaların yazılışı** (Tarama'da da): dış halka saat yönünün tersine, delikler saat yönünde, her biri en alçak (eşitse en
  batıdaki) köşesinden başlar; delikler bu köşelerine göre sıralıdır. Yüzün yürüyüşü, kesim ve ev sahibi halkada iz bırakmaz:
  araç, Python ve ajan aynı halkaları alır.

### 7. Tarama (`cad.entities.hatch`)

**Bölge** (`region`):

- `{kind: "point", at, way, boundaries?, boundaryLayer?, islands?, texts?, associative?}`: tıklanan yer.
  - `way: "object"` (kapalı nesne, varsayılan): `at`'yi çevreleyen en küçük kapalı nesne.
    - İçindeki ya da kenarını aşan küçük kapalı nesneler adadır (`islands`, varsayılan açık).
    - `associative` (varsayılan açık): tarama nesnelerini izler (`assoc`, ADR 0186 §6).
  - `way: "lines"` (çizgiler): sınırların çizgilerinin kapattığı yüz.
    - Sınır katmanı Alan oluştur'daki gibidir.
    - Hiçbir şeyi izlemez.
  - `texts` (varsayılan kapalı): bölgedeki yazılar ve blok yerleştirmeleri kutularıyla boş kalır.
- `{kind: "objects", uids, islands?, texts?, associative?}` (Çoklu tara): her kapalı nesne (çok parçalı alanın her
  parçası) kendi içinden taranır.
  - Kapalı olmayan nesne `not_closed` uyarısıyla atlanır.
  - Hiçbiri kapalı değilse ret: `not_closed`.

**Desen** (`pattern`): `{name, scale?, angle?, secondColor?, inverted?}`.

- Adı desen kitaplığındaki gibidir (Dolu, ANSI31, ISO02W100, degradeler …). Bilinmeyen ad ret: `unknown_pattern`.
- `scale` kâğıttaki ölçektir: projenin çizim ölçeğiyle çarpılır (aracın Ölçek'i).
- `angle` derecedir. `secondColor` ve `inverted` yalnız degradede anlamlıdır.
- Desen bölge için çok sıksa ret: `pattern_too_dense`.

**Retler ve yazma:**

- Bölge yoksa ret: `not_enclosed`.
- Taramalar `layerId`'ye `cad.entities.create`'in kurallarıyla tek adımda yazılır.
- **Hesap uygulamaya taşındı.** Bölgelerin hesabı (kapalı nesne, adalar, yazı boşlukları, yüzler) araçtan uygulama katmanına
  geçti: `kentos_native_application::region`. Araçlar onu çağırır.

### 8. Araçlar

- Her araç önizlemeyi komutun planıyla çizer, yazmayı komutla yapar.
- Görünen davranış ve iletiler değişmedi. Aracın girdisi komutun girdisidir:
  - aracın görünümü `area`;
  - seçim `uids`;
  - Topoloji'nin durumu `topology`;
  - Sınır katmanı `boundaryLayer`.
- Topoloji komşularının bulunuşu (`neighbours`) uygulama katmanına taşındı (`kentos_native_application::topology_follow`).
  Aracın komşuları komutunkiyle aynıdır.
- Bölgelerin yüzleri de: aracın `faces.rs`'i `region::faces`'i çağırır.
- **Önizlemenin hızı:** `RegionCache` bir sınır kümesinin yüzlerini ve kapalı nesnenin adalarla kesimini çizimin sürümüyle
  anahtarlayıp tutar. Araç işaretçi gezdikçe aynı önbelleği kullanır, çizim değişmedikçe yeniden kurmaz. Komut her çağrıda
  kendi önbelleğini kurar; hesap aynı koddur.

## Uygulama

- **Sözleşme:**
  - `cad_stretch.rs` (`EntitiesStretch`, `StretchWindow`, `StretchTopology`, `EntitiesStretched`, `EntitiesStretchPlan`);
  - `cad_break.rs` (`EntitiesBreak`, `EntitiesJoin`);
  - `cad_divide.rs` (`EntitiesDivide`, `Division`);
  - `cad_region.rs` (`EntitiesBoundary`, `EntitiesHatch`, `HatchRegion`, `HatchWay`, `HatchPatternSpec`);
  - katalog kayıtları, örnekleri ve yetenekleri; yeni kodlar `codes.rs`'te.
- **İşleyiciler:** `crates/native/application/src/stretch.rs`, `break_join.rs`, `divide.rs`, `region.rs`,
  `topology_follow.rs`; `registry.rs`'te altısı.
- **Araçlar:** `stretch.rs`, `breaking.rs`, `object.rs` (Birleştir), `divide.rs`, `boundary.rs`, `hatch.rs`,
  `hatch_selected.rs`.
- **Başsız sunucu, MCP, Python:** kayıt yoluyla; `python/kentos/cad/` üretildi.
- **İşlem manifesti:** `stretch`, `break`, `join`, `divide`, `boundary`, `hatch`, `hatchSelected` `semantic`.

## Doğrulama

- **Bağımsız başvurular** `scripts/fixtures/reshape_command_cases.py` ve `region_command_cases.py` (KentOS kodu olmadan, ADR
  0047, 0057, 0062, 0065, 0186 ve bu ADR'nin tanımından; düz geometri kesirlerle): durumlar `fixtures/commands/v1/cad.entities.
  {stretch,break,join,divide,boundary,hatch}.json`'da (20, 20, 23, 22, 25 ve 34 durum); masaüstü (`fixtures.rs`) ve Python
  (`test_command_cases.py`, tipli katmandan da) bit bit geçer.
- **Yetenekler:** her ret ve uyarı kodu bir durumda görülür ya da gerekçesiyle söylenir (`neighbour_invalid`, `join_failed`);
  Esnet, Kır, Birleştir (Zincir), Böl ve Çoklu tara her nesne türüyle denenir (`capabilities.rs`; Çoklu tara kapalı eğriyle).
- **Araç = komut:** araçların yazdığı, başsız sunucunun aynı girdiyle yazdığına eşittir.
- **İzler:** var olan izler (`stretch`, `break`, `join-chain`, `divide`, `boundary`, `hatches`, `hatch-patterns`) değişmeden
  geçer; kabul izi komutsuz düzenleme bırakmaz (ADR 0253 Doğrulama).
- **Resimler:** `usage-semantic-tools` (`hatch.kcad` üzerinde Tarama, Alan oluştur, Böl, Kır, Esnet ve Ölçülendirme; önizlemeler
  komutların planından): `./target/debug/kentos-cad kullan usage-semantic-tools`, `.run/shots/kullanim/masaustu-usage-semantic-tools-*`.

## Sonuçlar

- Esnet, Kır, Birleştir, Böl, Alan oluştur ve Tarama'nın ne yaptığı katalogda bir sözleşmedir; Python ve ajan sonucu ister,
  hesaplamaz.
- Ölçülendirme'nin on türü de anlamsal komutla kurulur: [ADR 0255](0255-semantic-dimension-create.md).
- Açık kalanlar:
  - Zincir ölçü, Baz ölçü ve Hızlı ölçü'nün kendi komutları (bugün `cad.entities.create`'in işlemleriyle yazarlar);
  - içe aktarmanın ve İşlemler araçlarının katalog komutu olması (ADR 0253 §11).
