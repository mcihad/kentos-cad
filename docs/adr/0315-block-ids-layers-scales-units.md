# ADR 0315: Blokta kararlı kimlik, katman kuralı, X ve Y ölçeği, blok birimi

- **Durum:** kabul edildi (2026-10-16). `plan/cad.md` §10 `CAD-BLOCK-01`, `02`, `03`, `04`. Sahibin isteği: “CAD blocok ile devam
  et”; önceki grupların kuralıyla 3–4 madde birlikte yapılıp sınanır. Bu dört madde birinci gruptur. Kapsamı ben belirledim.
- **Bağlam belgesi:**
  - ADR 0144 (bloklar), 0263 (blok içi yol), 0274 (blokların içine kenet), 0156 (afin oturtma, `ops::warp`);
  - 0257 (DXF birimi, `BLOCK_RECORD` 70), 0260 ve 0312 (görünüşün kaynağı, `byLayer`), 0256 (birim değiştirme);
  - CLAUDE.md §9.2.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

- **Blok içi yol sıradan yapılıyordu.** Ölçünün blok içindeki parçaya bağı (ADR 0263) tanımın `entities` dizisindeki sıradır.
  Tanıma bir nesne girince ya da çıkınca sıralar kayar ve bağ yanlış parçaya geçer.
- **Parçanın katmanı çizimi etkilemiyordu.** ADR 0144'ün v1 sadeleştirmesi: içerik yerleştirmenin katmanında çizilir, parçanın kendi
  katmanının gizliliği uygulanmaz. AutoCAD'de ise:
  - 0 katmanındaki parça yerleştirmenin katmanını alır;
  - gerçek katmandaki parça o katman kapalıyken görünmez;
  - BYLAYER o katmanın, BYBLOCK yerleştirmenin değeridir.

  DXF içe aktarma BYLAYER'ı o anki katman değerine çeviriyordu; katman sonra değişince blok izlemiyordu.
- **Yerleştirmenin tek ölçeği vardı.** X ve Y ölçeği farklı INSERT patlatılarak alınıyordu. KentOS'ta bir pencereyi eninde
  germek mümkün değildi.
- **Bloğun birimi yoktu.** KentOS'ta geometri her zaman metredir, bu yüzden milimetre çizimde yapılan blok metre çizimde de
  gerçek boyundadır. Ama:
  - birimsiz semboller (kuzey oku gibi, çizim birimiyle büyüyen işaretler) yoktu;
  - yanlış birimde alınmış bir bloğu (metre sanılan milimetre) düzeltmenin yolu yoktu;
  - DXF'e bloğun birimi yazılmıyordu.

## Karar

### 1. Parçanın yerel kimliği kararlıdır (`CAD-BLOCK-01`)

- Bloğun ve yerleştirmenin kimlikleri zaten kalıcıdır (`BlockId`, `uid`). Tanımın nesnelerinin yerel `id`'si artık yolun izlenmesinde
  kullanılır.
- **Yolun izlenmesi:** tanım değişince, yoldaki her adımın nesnesi önceki tanımdaki yerel kimliğiyle yeni tanımda aranır
  (`Blocks::path_in`, `ops::subref::follow_path`):
  - kimlik iki tanımda da bir kez varsa yol yeni yerine geçer, ölçünün başvurusundaki `path` güncellenir (`dimension_ties.rs`);
  - kimlik tekil değilse adım yerinde kalır (eski davranış).
- **Kimliği koruyan yollar:** yeniden adlandırma, taban noktası, öznitelikler ve birim değişikliği kimlikleri korur. Yeniden
  tanımlama (seçili nesnelerden) yeni kimlikler verir; bu AutoCAD'deki gibidir. Blok düzenleyici (`CAD-BLOCK-05`) kimlikleri
  koruyacak.
- Kenet (`cad.snap.find`) ve seçim (`cad.entities.pick`) yolları değişmez.

### 2. Katman kuralı: 0 katmanı, kendi katmanı, BYLAYER, BYBLOCK (`CAD-BLOCK-02`)

Parçanın görünüşü (ADR 0312 §4'ün sözlüğüyle):

| Parçada | Değer |
|---|---|
| kendi değeri | kendi değeri |
| `byLayer` (katmana göre) ve kendi katmanı var | kendi katmanının değeri |
| `byLayer` ve katmanı `""` (bloğun katmanı, DXF'in 0'ı) | onu çizen yerleştirmenin **katmanının** değeri |
| ikisi de yok (bloğa göre, BYBLOCK) | yerleştirmenin kendi değeri, yoksa yerleştirmenin katmanının değeri |

- **Görünürlük:** kendi katmanı olan parça o katman görünürken görünür. İç içe yerleştirmenin parçaları da onun katmanı görünürken
  görünür. Gizli ya da dondurulmuş katmandaki parça çizilmez, seçilmez, kenetlenmez (`Piece::gates`, deponun `Item::veiled`'ı,
  `set_layers` değişince yeniden kurulur). Kilit ve kenet türleri yerleştirmenin katmanınındır (ADR 0274).
- **İç içe:** içteki tanımın 0'daki parçası, içteki yerleştirmenin katmanını alır. İçteki yerleştirme de 0'daysa en dıştakinin
  katmanını alır. İçteki parçanın bloğa göre olan değeri içteki yerleştirmenin kendi değerini alır. O da katmana göreyse
  içteki yerleştirmenin katmanını alır (`flatten`).
- **Eski çizimler değişmez:** kendi değeri olmayan, `byLayer`'ı olmayan parça zaten bloğa göreydi. Değişen tek şey şu: kendi
  katmanı gizli olan parça artık görünmez. Bu AutoCAD'in davranışıdır.
- **Blok oluştur ve Seçili nesnelerle yeniden tanımla:** nesneler katmanlarıyla kopyalanır ve kendi değeri olmayan her parça
  `byLayer` alır (`as_drawn`). Böylece blok, çizildiği gibi görünür (AutoCAD'in BLOCK'u gibi). Kopyala ve içe al dokunmaz.
- **DXF okuma:** tanım içindeki BYLAYER artık değere çevrilmez, `byLayer` olarak kalır (0'da yerleştirmenin katmanı). BYBLOCK
  boş kalır. "o katmanların gizliliği ve kilidi uygulanmaz" notu kalkar, çünkü artık uygulanıyor. Yazıcı zaten bu kuralla
  yazıyordu (BYLAYER, BYBLOCK, 0).
- **Sözleşme:** `resolve_look`'u çağıran, 0'daki parça için yerleştirmenin katmanını verir.
- **Doğrulama çizimi:** `block_scenes.rs` aynı kapıyı üç kez yerleştirir: Kapılar'da, Mobilya'da ve kendi rengi mavi olarak.
  - kanat 0'da ve katmana göre: yerleştirmenin katmanının rengi;
  - yay 0'da ve bloğa göre: mavide mavi;
  - kasa Duvar'da: her yerde kırmızı;
  - gizli Ayrıntı'daki daire görünmez.

### 3. X ve Y ölçeği (`CAD-BLOCK-03`)

- **Sözleşme:**
  - `InsertEntity.yScale`: pozitif, sonlu, `scale`'den farklıysa yazılır; yoksa `scale` geçerlidir.
  - Aynalama bloğun x ekseninde kalır.
  - DXF'in eksi ölçekleri aynalama ve yarım dönüşe çevrilir: `sx < 0` iken dönüş 180° artar, işaretler farklıysa aynalanır.
- **Yerleşim:** x' = p + R(θ)·S·M·(x − taban), S = diag(sx, sy).
  - Eşit ölçek eskisi gibi benzerliktir, bit bit aynıdır.
  - Eşit olmayan ölçekte parçalar Vektör oturtma'nın afin bükmesiyle yerleşir (`block::place`, `ops::warp`):
    - düz geometri kesindir;
    - daire, yay ve elips görüntülerinin kesin elipsi olur;
    - yolun yay kenarları 0,1 mm içinde köşelere bölünür;
    - yazı noktasının türeviyle biçimini korur: yükseklik h·|det J|/|J·u|, genişlik çarpanı |J·u|²/|det J|.
  - İç içe yerleştirmeler bileşir.
- **Patlat:** iç içe yerleştirme, bileşik eksenleri dik kaldıkça (dönüş, iki ölçek, aynalama) yerleştirme olarak kalır
  (`insert_through`). Eksenler eğiliyorsa (eşit olmayan ölçekli dış bloğun içinde dönük blok) Patlat nedenini söyleyerek reddeder.
  Parçaları önce içteki bloğu patlatarak açılır.
- **Düzenlemeler:**
  - Taşı, döndür ve aynala dönüşü bileştirir.
  - Ölçekle iki ölçeği birlikte çarpar.
  - Esnet ve tutamaç yerleştirme noktasını taşır.
  - Taban noktası yerleştirmenin tersiyle bulunur (`insert_local_xy`).
- **DXF:** INSERT'in 41/42'si okunur ve yazılır. Eşit olmayan ölçekli yerleştirme artık patlatılmaz.
- **Arayüz:**
  - Blok ekle'de **Y ölçeği (Y)**; Ölçek (Ö) iki ölçeği birlikte koyar.
  - Öznitelikler'de **Ölçek** (eşitken), **X ölçeği** ve **Y ölçeği**.

### 4. Blok birimi ve `cad.blocks.insert` (`CAD-BLOCK-04`)

- **`BlockDefinition.units`:** `mm`, `cm`, `m`, `in`, `ft` ya da `unitless`.
  - **Birimli blok** gerçek boyutludur (geometri çizimin metresi). Birim, çizildiği birimi söyler ve DXF'e yazılır. Her çizimde
    aynı boyda yerleşir.
  - **Birimsiz blok:** sayıları çizim birimidir. Yerleştirmenin ölçeği çizim biriminin metresiyle çarpılır: 1 birimi milimetre
    çizimde 1 mm, metre çizimde 1 m'dir.
- **`cad.blocks.insert` v1:** Blok ekle'nin anlamsal komutu.
  - **Girdi:** blok (kimlik ya da Türkçe katlamalı ad), `at`, `scale`, `yScale`, `rotation` (radyan), `mirror`, `layerId`,
    `color`, `attributes` (etikete göre değerler).
  - **Çıktı:** `uid`, `id`, `block`, `unitFactor`, `revision`. Plan, yuvası 0 olan yerleştirmedir.
  - **Retler:** `no_block`, `not_finite` (`at`, sonra dönüş), `invalid_scale` (önce `scale`), sürüm, `unknown_block`,
    `unknown_tag`, katman denetimleri.
  - **Uyarılar:** `layer_hidden`, `insert_units_differ` (birimli blok çizimden başka birimde).
  - Adım “Ekle”. Blok ekle aracı yazmayı ve hayaletini bu komutla yapar (birimsiz bloğun çarpanı dahil). Manifestte
    `blockInsert` artık `cad.blocks.insert`'le anlamsaldır.
- **`cad.blocks.edit`'in `units` işlemi:**
  - Birimin adı değişir, boy korunur.
  - `rescale` ile sayılar korunur: geometri birimlerin oranıyla ölçeklenir, iç içe yerleştirmelerin ölçekleri de. Metre sanılan
    milimetre bloğu böyle binde birine iner.
  - Birimsiz yapmak ve birimsizlikten çıkarmak sayıları bu çizimin birimine ya da metreye çevirir. Yerleştirmeleri boy
    değiştireceği için yerleştirilmiş blokta reddedilir (`block_in_use`).
  - Birim verilmezse `no_units`. Adım “Blok değiştir”.
- **`cad.blocks.define`'ın `units`'i:** verilmezse yazılmaz. Blok oluştur çizimin birimini verir. Birimsiz bir blok
  tanımlanırken sayılar çizim birimine çevrilir; yerine konan yerleştirme çarpanı alır, görünüş değişmez.
- **DXF:**
  - `BLOCK_RECORD` 70 okunur: yoksa ve dosya bir birim bildiriyorsa dosyanın birimi kullanılır.
  - Yazılan değer dosyanın birimidir (blokların sayıları onda), birimsiz blokta 0.
- **Arayüz:** Bloklar panelinde **Birim ▸**:
  - altı birim, seçili olan işaretli;
  - “Sayıları koru: milimetre / santimetre / metre”;
  - satırın ipucunda bloğun birimi.

### 5. Dosya

- **`.kcad` şema 57** (`FORMATS_VERSION` 66): yerleştirmenin `yScale`'i (yalnız `scale`'den farklıysa) ve tanımın `units`'i.
  - Yalnız biri varken yazılır.
  - Eski okuyucu ikisini de `unknown_field` ile reddeder.
  - Yazıcı ve okuyucu `scale`'e eşit `yScale`'i reddeder: bir yerleştirmenin tek yazılışı vardır.
- **Tipli sütunlar:** yerleştirmenin seçenek biti 1 ve dönüşten sonraki sayı.
- **Deponun paket kaydı:** bayrağın 2'si y ölçeğinin geldiğini söyler. Y ölçeği olmayan kayıt eskisiyle aynı bayttır.

## Sonuçlar

- Ölçünün bağı, tanıma nesne girip çıkınca parçasını yerel kimliğiyle izler.
- Bloklar AutoCAD gibi görünür:
  - 0'daki parça yerleştirmenin katmanını alır;
  - kendi katmanındaki parça o katmanın değerlerini alır ve o katman gizliyken görünmez.

  DXF'ten gelen blok katman değişince izler.
- Kapı eninde, pencere boyunda gerilebilir; daire elips olur. DXF'in eşit olmayan ölçekli INSERT'i blok olarak kalır.
- Birimsiz semboller her çizim biriminde doğru boyda yerleşir. Yanlış birimde alınmış blok tek komutla düzelir. Python ve ajan blok
  yerleştirmeyi geometri hesaplamadan yapar (`kentos.cad.blocks.insert`).
- **Sınırlar:**
  - Yolun yay kenarları eşit olmayan ölçekte 0,1 mm'lik köşelerdir, analitik eliptik yay değildir.
  - Eğilen iç içe blok patlatılmaz.
  - Masaüstünün seçim vurgusu (`piece_entities`) gizli katmandaki parçayı da çizer.
  - Web ertelendi.

## Doğrulama

- **Bağımsız başvurular** (KentOS kodu olmadan):
  - `block_insert_cases.py`: `cad.blocks.insert`, 26 durum;
  - `blocks_command_cases.py`: Blok oluştur'un `byLayer`'ı ve `units` işleminin 7 durumu, toplam 27 + 43;
  - `block_scale_cases.py`: 13 yerleştirme, 4 Patlat; elips, yay, yazı, yay kenarının köşeleri;
  - `look_cases.py`: 0 katmanının 6 durumu, toplam 28;
  - `subref_cases.py`: yerel kimlikle yol izleme, 2 durum;
  - `snap_block_cases.py`: kendi gizli katmanındaki parça kenetlenmez;
  - `kcad_v2_reference.py`: `block-extras.kcad` ve 7 bozuk dosya, `tools/kcad/kcad.py` ile;
  - `dxf_write_reference.py`: `BLOCK_RECORD` 70.
- **Çekirdek:**
  - `tests/all/block_scales.rs`;
  - deponun `a_piece_shows_only_while_its_own_layer_does`'u;
  - `subref.rs`'in yol beklentisi;
  - DXF testleri: eşit olmayan INSERT blok kalır, BYLAYER `byLayer` olur.
- **KCAD:** `tests/all/block_extras.rs` (şema 57, sütunlar, retler).
- **Masaüstü resimleri:**
  `KENTOS_SHOTS_ONLY=blok-katman-kurali,blok-olcek,blok-olcek-oznitelik,blok-ekle-y cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`
  (`.run/shots/arac-blok-*`).
- **Süre** (release, bu makine), `cargo test --release -p kentos-geometry-core --test all block_scales::timing -- --ignored --nocapture`:
  odanın 10 000 yerleştirmesi (40 000 parça) eşit ölçekte 3,6 ms, x 1,5 y 0,5 ile 7,1 ms.
