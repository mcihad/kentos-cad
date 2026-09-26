# ADR 0032: Masaüstünde nokta, daire, yay ve dikdörtgen araçları; `cad.point.create`, `cad.circle.create`, `cad.arc.create` v1

- **Durum:** kabul edildi (2026-09-26). Yön ADR 0021, 0027 ve 0029'un ertelenenlerinden (öbür çizim araçları) ve TODOS.md `UX-01`, `CMD-04`, `CMD-07`, `UI-11`'den gelir. Üç komutun sözleşmesi, web'de TypeScript'te kalan iki hesabın ortak çekirdeğe taşınması, araçların çalışmalar arası belleği, istemin ve önizlemenin yeni alanları, şeritteki yöntem menüsü ve izlerin yeni beklentileri bu dilimin kararıdır.
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** TODOS.md `UX-01`, `UX-02`, `UX-06`, `UX-07`, `CMD-04`, `CMD-07`, `UI-11`, `REN-11`; ADR 0008 (ortak çekirdek sınırı), 0013 (ürün komutu sözleşmesi), 0014 (kalıcı kimlik), 0018 (araç oturumu ve izler), 0021 (native araç oturumu), 0022 (ilk ürün komutu), 0027 (çizgi ve çoklu çizgi), 0029 (seçim, kenet, silme)

## Bağlam

- Masaüstünde kapalı alan, çizgi, çoklu çizgi, seçim ve Sil araçları vardı. Nokta, daire, yay ve dikdörtgen şeritte soluk duruyor, “web'de var” diyordu.
- Web'in bu araçları belgeye kendileri yazıyordu: `PointInputTool.create` → `writableLayer` → `CadDocument.add` (koddan doğrulandı, 26 Eylül):
  - nokta aracı ve Kot noktası aracı `point`, daire aracı `circle`, yay aracı `arc` nesnesi yazıyordu;
  - dikdörtgen, döndürülmüş dikdörtgen ve düzgün çokgen `polygon` nesnesi yazıyordu; yuvarlatılmış köşeler yay değeri (`bulges`) olarak.
- İki hesap web'de TypeScript'teydi (CLAUDE.md §4.8.1'e aykırı):
  - dikdörtgenin köşe biçimi: `RectangleTool.styled`, her köşeye sondan başa `cornerOfPath` uygulayan döngü;
  - teğet dairede tıklanan nesnenin en yakın kenarı: `curveTools.ts`'teki `nearestEdge`.
- Web araçlarının oturumlar arası hatırladıkları statik alanlardadır: son daire yarıçapı (`CircleTool.lastRadius`), dikdörtgenin dönmesi ve köşeleri, düzgün çokgenin kenar sayısı ve çemberi. Sayfa açık kaldıkça durur, dosyaya yazılmaz.
- Web'in istem metni seçeneğin değerini de yazar: `Dikdörtgen: karşı köşeyi belirtin [Döndür (D): 0° / Boyutlar (B)]`. Masaüstünün istem modelinde değer yoktu.

## Karar

### Katalog kayıtları

Üç yeni komut, ADR 0022'nin kalıbıyla (`crates/shared/contracts/src/cad_primitives.rs`, `catalog.rs`):

| Alan | `cad.point.create` | `cad.circle.create` | `cad.arc.create` |
|---|---|---|---|
| `title` | Nokta oluştur | Daire oluştur | Yay oluştur |
| girdi | `PointCreate { layerId, p, z?, label?, color?, attrs?, expectedRevision? }` | `CircleCreate { layerId, c, r, color?, attrs?, expectedRevision? }` | `ArcCreate { layerId, c, r, a0, a1, color?, attrs?, expectedRevision? }` |
| çıktı | `PointCreated { uid, id, revision }` | `CircleCreated` (aynı) | `ArcCreated` (aynı) |
| plan | `PointPlan { entity, revision }` | `CirclePlan` | `ArcPlan` |

- Üçünde de: `effect: document`, `hosts: web, desktop`, `headless`, `requires: document`, izin yok, `undo: step`, `cost: instant`, `aliases` yok (`PO`, `C`, `A` arayüz komutlarının, `tool.*`'ındır). Her biri iki örnekle.
- **Komut nesneyi belgenin sakladığı biçimde alır.** Yöntemler (Çap, 2N, 3N, TTY, TTT; üç nokta, başlangıç–merkez, Devam…) komut değildir, aracın akışıdır. Araç yöntemin dairesini ya da yayını ortak çekirdekle bulur, komut yalnız yazar. Böylece Python ve AI aynı nesneyi tek bir sözleşmeyle yazar; yöntem başına komut gerekirse ayrı karardır.
- **Katman ve renk girdide açıktır (`CMD-07`).** Araç onları etkin katmandan ve güncel renkten doldurur. Kot noktası aracı kendi katmanını (`kot`) verir.
- **Nokta:** kot (`z`) ve yanında görünen yazı (`label`) isteğe bağlıdır; Kot noktası aracı ikisini de yazar. Sıfır kot ve boş yazı da yazılır.
- **Yay:** saat yönünün tersine, `a0`'dan `a1`'e, radyan. **Açılar verildiği gibi saklanır**: sıfırı geçen, eksi ve tam turdan büyük açılar değişmez; eşit açılar belgenin okuduğu gibi tam turdur. Bugün araçların yazdığı da buydu; komut açıyı düzeltmez.
- **Dikdörtgen, döndürülmüş dikdörtgen ve düzgün çokgen için yeni komut yok.** `cad.polygon.create` (ADR 0022) yazar; yuvarlatılmış köşeler yay değeri, pah kırılmış köşeler ek köşedir. Nesne türü öncekiyle aynıdır (`polygon`).

### Denetimler ve sıraları

İlk tutmayan cevap verir. Sürüm ve katman denetimleri bütün oluşturma komutlarınındır (`checks.rs`, `checks.ts`; ADR 0022, 0027).

| Sıra | Nokta | Daire | Yay | Kod | Yol |
|---|---|---|---|---|---|
| 1 | `p` sonlu (önce x, sonra y) | `c` sonlu | `c` sonlu | `not_finite` | `p.x`, `c.y`… |
| 2 | `z` verildiyse sonlu | `r` sonlu | `r`, `a0`, `a1` sonlu | `not_finite` | `z`, `r`, `a0`, `a1` |
| 3 | — | `r > 0` | `r > 0` | `invalid_radius` (yeni) | `r` |
| 4 | beklenen sürüm | aynı | aynı | `invalid_revision`, `revision_conflict` | `expectedRevision` |
| 5 | katman | aynı | aynı | `layer_not_found`, `not_a_layer`, `layer_locked`; gizli katmanda uyarı `layer_hidden` | `layerId` |

- Sonlu olmayan açı, sıfır yarıçaptan önce söylenir: önce değerler okunabilir olmalı, sonra anlamları denetlenir.
- **Yarıçap yalnız sıfırdan büyük olmalıdır.** Çok küçük pozitif yarıçap yazılır; geometrik küçüklüğü komut denetlemez. Daire aracı 1e-9 m'nin altını zaten yazmaz (web'in `r > 1e-9`'u, iki platformda aynı); yay aracının yapıları yay çıkmayınca uyarır.
- Aynı yere ikinci nokta da yazılır; geometrik geçerlilik denetlenmez (kapalı alanda olduğu gibi).
- **Yeni iletiler** (tam metin durum dosyalarında):
  - `Kot sonlu bir sayı değil (NaN ya da sonsuz). Kotu sonlu bir sayıyla verin.`
  - `Yarıçap sonlu bir sayı değil (NaN ya da sonsuz). Yarıçapı sonlu bir sayıyla verin.`
  - `Başlangıç açısı` / `Bitiş açısı sonlu bir sayı değil (NaN ya da sonsuz). Açıyı sonlu bir sayıyla verin.`
  - `Yarıçap sıfırdan büyük olmalı. Pozitif bir yarıçap verin.`
  - Koordinat iletileri ortaktır: `Merkezin doğu (Y) değeri sonlu bir sayı değil …`, `Noktanın kuzey (X) değeri …`.
- **Yürütme** belgenin kendi `add`'idir: tek geri alma adımı, adı “Ekle”; açık işlem ya da grup varsa ona katılır. Yinelemede nesne aynı kalıcı kimlikle döner (ADR 0014).

### Ortak durumlar

`fixtures/commands/v1/cad.point.create.json`, `cad.circle.create.json`, `cad.arc.create.json`: her biri 23 durum, elle yazıldı. İki koşucu (web `product/fixtures.test.ts`, masaüstü `tests/fixtures.rs`) 158 durumun hepsini geçer (23 + 20 + 23 + 23 + 23 + 23 + 23).

- Sürüm, katman, plan ve doğrulamanın ortak durumları; ayrıca NaN ve sonsuz her değer, ilk bozuk değerin sırası, sıfır ve eksi yarıçap, çok küçük yarıçap, büyük koordinat, açıların olduğu gibi saklanması, eşit açılar, kot ve yazı.
- `nonFinite` tablosu isteğe bağlı sayıyı da bozabilir: girdi `z`'yi bir sayıyla verir, tablo onu değiştirir (`fixtures/commands/README.md`).
- `DESKTOP_COMMANDS` ve `WEB_COMMANDS` katalogla eşittir (test). Katalog ve üretilen tipler `KENTOS_WRITE_CATALOG=1 cargo test -p kentos-contracts catalog` ile yazıldı.

### Web araçları komuttan yazar

ADR 0027'nin çizgi aracına yaptığı gibi. Her nesne yine tek geri alma adımıdır, adı aynıdır:

- Nokta ve Kot noktası: `cad.point.create` (`PointTool.writePoint`).
- Daire: `cad.circle.create`. Yay: `cad.arc.create`.
- Dikdörtgen, döndürülmüş dikdörtgen, düzgün çokgen: `cad.polygon.create` (`writeRing`).
- `PointInputTool`'a üç yardımcı eklendi: `written` (komutun cevabını çizgi aracının yazdığı gibi yazar: ret iletisi uyarıdır, yazmanın uyarıları uyarıdır; nesne Ctrl+Z için not edilir), `writeRing`, `colour` (güncel renk, yoksa katmanınki). Çizgi aracı da `written`'ı kullanır.
- Katalogda her aracın komutu yazılıdır (`tools[…].productCommand`); envanter onu okur.
- **Bir küçük fark:** Kot noktası aracının katmanı (`kot`) çizimde yoksa eski araç sessizce hiçbir şey yazmıyordu. Şimdi komutun iletisi uyarı olarak görünür: `“kot” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.` Yeni projelerde `kot` katmanı vardır (`standardLayers.ts`).

### Geometri ortak çekirdekte

İki hesap `kentos-geometry-core`'a taşındı; web onları WASM'dan çağırır, TypeScript kopyası silindi (CLAUDE.md §4.8.1, §14):

- `cornersOfRing` (`ops/fillet.rs`): kapalı halkanın her köşesine, sondan başa, `cornerOfPath`'i uygular (yuvarlama yarıçapı ya da pah uzaklıkları); ilk hata döner, araç düz halkayı yazar ve “Köşeler işlenmedi: …” der.
- `nearestEdge` (`ops/edges.rs`): nesnenin noktaya en yakın kenarı (eşitlikte ilki); çoklu çizgide tıklanan parça, dairede dairenin kendisi.
- **Silmeden önce yan yana deneme:** işlem başına 20 000 rastgele çağrı, sıfır toleransla TypeScript aslıyla aynı cevabı verdi (tek seferlik test, sonra silindi).
- **Çağrı durumları:** `fixtures/geometry/v1/calls-s5-tools.json`'a 65 durum eklendi (`cornersOfRing` 34, `nearestEdge` 31; adlı ve rastgele). Rastgele üreticiler kümenin sonuna eklendi, önceki çekilişler ve cevaplar değişmedi. Native ve WASM aynı cevapları verir.
- **Bağımsız referans** (`scripts/fixtures/geometry_call_reference.py`, `reference-calls.json`) üç durum aldı: TM koordinatında 2 m yuvarlatılmış dikdörtgen (yay değeri √2−1), döndürülmüş karede 1,5 m pah, TM çoklu çizgisinde tıklanan kenar.
- **Masaüstünde yeni geometri kodu yoktur.** Araçlar çekirdeğin işlevlerini çağırır: `rect_from_corners`, `rect_from_size`, `rect_from_edge`, `side_distance`, `regular_polygon*`, `circle_through`, `circle_on_diameter`, `tangent_tangent_radius`, `tangent_tangent_tangent`, `arc_through`, `arc_start_*`, `end_tangent`, `corners_of_ring`, `nearest_edge`, `tessellate_*`.
- **Devam'ın kaynağı akıştır, hesap değil:** belge sırasında geriye doğru ilk çizgi, yay ya da çoklu çizgi; ucu ve oradaki doğrultu çekirdeğin `endTangent`'ıdır. İki platform bu seçimi kendi dilinde yazar (web `ArcTool.lastEnd`, masaüstü `arc::last_end`); `arc-variants` izi ikisini bağlar.

### Masaüstünde altı araç

`kentos-interaction`'da web'in sınıfları adım adım (`point.rs`, `circle.rs`, `arc.rs`, `rectangle.rs`, `rotated.rs`, `regular.rs`): istemler, seçenekler, tuşlar ve iletiler aynı metindir.

| Araç | Web sınıfı | Yöntemler ve seçenekler |
|---|---|---|
| Nokta (`tool.point`, N) | `PointTool` (`askZ: false`) | her tıklama ya da yazılan nokta bir nesne ve bir adım; Enter çıkar |
| Daire (`tool.circle`, C) | `CircleTool` | merkez ve yarıçap, Çap (Ç), 2N, 3N, TTY (yazılan yarıçap ya da Enter ile son yarıçap), TTT |
| Yay (`tool.arc`, A) | `ArcTool` | üç nokta; başlangıç–merkez (M) ve bitiş, Açı (A), Kiriş (U); başlangıç–bitiş (B) ve Merkez (M), Açı (A), Yön (Y), Yarıçap (R); önce merkez (M); Devam (D) |
| Dikdörtgen (`tool.rectangle`, R) | `RectangleTool` | iki köşe; Köşe yuvarla (Y), Pah (P), Döndür (D), Boyutlar (B) |
| Döndürülmüş dikdörtgen (`tool.rectangle3`, Alt+R) | `RotatedRectangleTool` | kenar, sonra genişlik (fareyle ya da yazılan) |
| Düzgün çokgen (`tool.regularPolygon`, Shift+G) | `RegularPolygonTool` | Kenar sayısı (S, 3–1024), Çember (Ç: köşelerden ya da kenarlara teğet), Kenardan (K), Merkezden (M) |

- **Kenet** bütün bu araçlarda çalışır. İki istisna web'inkidir: teğet yöntemlerinde nesne seçilirken kenet kapalıdır (`snaps`); dikdörtgenin köşelerine orto ve kutupsal izleme uygulanmaz (kutuyu yassılatırdı), kenet uygulanır.
- **Teğet seçimi** ortak depodan, web'in `pickEdge`'i gibi: `hit_edge` seçim yarıçapı (`drafting.pickAperture`) içindeki nesneleri yakından uzağa verir (eşit uzaklıkta belge sırası); uygun türden en yakını alınır (çizgi, çoklu çizgi, kapalı alan, yay, daire, yardımcı çizgi, ışın), kenarı `nearest_edge`'dir. Gizli katman seçilmez. Tıklanan yerler 9 px kareyle işaretlenir.
- **Araçların belleği (`Memory`)** web'in statik alanlarıdır: son daire yarıçapı, dikdörtgenin dönmesi ve köşeleri, düzgün çokgenin kenar sayısı ve çemberi. Uygulama tutar, açık kaldıkça durur, dosyaya yazılmaz; yeni uygulama web'in varsayılanlarıyla başlar (0, 0°, keskin köşe, 6 kenar, köşelerden geçen çember). Araç `Context.memory` ile okur ve yazar.
- **İstemde değer:** `Prompt.step` değer taşıyabilir (`yarıçapı yazın (Enter: 2.000 m)`); `PromptOption.value` seçeneğin şimdiki değeridir. `Prompt::text` web'in metnini yazar: `Köşe yuvarla (Y): 2.000 m`, `Pah (P): kapalı`, `Döndür (D): 30°`, `Kenar sayısı (S): 6`. Durum çubuğunun seçenek düğmeleri ve komut satırının seçenekleri değeri adın yanında gösterir. Tipli `PromptSpec` yine `UX-02`'dir.
- **Önizleme:** `Preview.strokes` web'in `strokePath`'idir (düz ya da kesikli, 1–2 px): yazılacak daire, yay ya da dikdörtgen, kesikli kılavuz çember. `Preview.squares` teğet seçimlerinin kareleridir. İkisi de Iced canvas'ında çizilir (birkaç şekil, her harekette değişir; ADR 0029'un gerekçesi). Etiketler web'inkidir (`r 5.000 m`, `Açı 180.00°`, `10.000 × 3.000 m`, `Alan 30.00 m²`).
- Her araç kendi komutuyla yazar (`cad.point.create`, `cad.circle.create`, `cad.arc.create`, `cad.polygon.create`); bir nesne, bir geri alma adımı. Çalışırken Ctrl+Z en yenisini geri alır (ADR 0018).

### Şeritte yöntem menüsü

- Web'de Daire ▾ ve Yay ▾ düğmelerinin menüsü yöntemleri listeler (`splitControl`): başlıkta aracın adı, her satırda yöntem (`Merkez, yarıçap`, `2 nokta`, … ; `3 nokta`, `Merkez, başlangıç, bitiş`, `Devam`). Seçilen yöntem aracı başlatır, seçeneğini yazılmış gibi verir (`runEntry`).
- Masaüstü menüsü aynı listeyi envanterden okur (`catalog::Entry`: kimlik, ad, seçenek). Yöntem `Message::RunMethod` ile aracı başlatır ve seçeneği verir. Araç seçeneği reddederse ““Daire: 2 nokta” şu an başlatılamadı.” der; açık çizim yoksa aracın kendi iletisi yeter.
- Araçlar taşınmadan önce bu menü yoktu: düğme soluktu. Menü envanterdeki `option`'ı okumasaydı “Daire” beş kez görünür ve hepsi varsayılan yöntemi başlatırdı.
- **Web'le iki fark:** web son seçilen yöntemi düğmenin üstünde tutar ve oturumlar arasında hatırlar (`ctx.ui.ribbonSplits`); masaüstü düğmesi hep ilk yöntemi başlatır. Web menüsü yöntemin açıklamasını ipucu olarak gösterir; KentOS UI menüsünde ipucu alanı yoktur. İkisi de ertelenenlerdedir.
- `kentos-cad snapshot`'a iki seçenek eklendi: `--tikla x,y` (pencereye tıklar, ör. menüyü açar) ve `--imlec x,y` (imleci taşır).

### Etkileşim izleri

`fixtures/interaction/v1`'e dört iz eklendi. Web ve masaüstü onları değiştirmeden, üç varyantta geçer.

| İz | Çizim | Ne tutar |
|---|---|---|
| `point-series` | `objects.kcad` | uca kenetlenen, boşluğa tıklanan ve yazılan noktalar; her nokta ayrı nesne ve adım; Ctrl+Z en yenisini geri alır; Enter araçtan çıkar |
| `circle-methods` | `objects.kcad` | merkez ve yarıçap (tıklanan, yazılan, Çap ile yazılan çap); 2N (yazılan `2n`); 3N; TTY (komut satırından, yazılan yarıçap, sonra Enter ile son yarıçap); TTT; Ctrl+Z |
| `arc-variants` | `empty.kcad` | nesne yokken Devam uyarır; üç nokta; başlangıç–merkez ve yazılan açı; başlangıç–bitiş ve yazılan yarıçap; önce merkez; çizgiye teğet Devam; Ctrl+Z |
| `rect-options` | `empty.kcad` | iki köşe; köşe yuvarlama (2 m, dört yay); döndürme (30°) ve boyutlar (10,5); pah (1 m); döndürülmüş dikdörtgen (yazılan genişlik); düzgün çokgen (yazılan kenar sayısı 4); Ctrl+Z |

- **Biçim eklemeleri:** `newest.points` noktanın yerini ve yayın saat yönünün tersine başlangıcını ve bitişini de verir; yeni `newest.center` ve `newest.radius` (dairenin ya da yayın; `clickTolerance` içinde). İki oynatıcı okur (`interaction.mjs`; masaüstü `traces/player.rs`, `compare.rs`, `format.rs`).
- **Kural:** iz araçların belleğini başladığı gibi bırakır. Web oynatıcısı sayfayı izler ve varyantlar arasında yeniden açmaz; masaüstü her izi yeni bir uygulamada oynatır. `rect-options` bu yüzden köşeleri ve kenar sayısını geri kurar (`fixtures/interaction/README.md`).
- Dairenin, yayın ve dikdörtgenin son tıklamasından önce imleç o noktaya gider: görüntü önizlemeyi gösterir, başka bir şey değişmez.
- `select-delete`'in kaynağı artık `tool.select` ve `tool.erase`'ı adlarıyla verir; envanter izi bu araçlara bağlar (ADR 0029'dan kalan eksik).

### Taşınan komutlar

`tool.point`, `tool.circle`, `tool.arc`, `tool.rectangle`, `tool.rectangle3`, `tool.regularPolygon` (`apps/desktop/ported.json`). Masaüstünde 36 / 164 komut çalışıyor. Envanter yeniden üretildi: her araç komutunu adlandırır, yeni izler testleridir.

### Ortak olan, ortak olmayan

| | Web | Masaüstü | Ortak |
|---|---|---|---|
| Komutlar | `product/pointCreate.ts`, `circleCreate.ts`, `arcCreate.ts` | `native/application/src/point.rs`, `circle.rs`, `arc.rs` | sözleşme tipleri, katalog; `fixtures/commands/v1` |
| Yapılar (köşe, teğet, yay, çokgen) | WASM çağrıları | native çağrılar | `kentos-geometry-core`; `fixtures/geometry/v1` |
| Araç akışı, istem, bellek | `drawTools.ts`, `curveTools.ts`, `shapeTools.ts` (statik alanlar) | `kentos-interaction` (`Memory`) | `fixtures/interaction/v1` |
| Önizleme | 2D canvas (`strokePath`) | Iced canvas (`Stroke`) | çizgi biçimleri, etiketler |
| Yöntem menüsü | `splitControl`, `runEntry` | `split_menu`, `RunMethod` | envanterin şerit düzeni |

## Ters deneme

Her iz ailesinde bir kural bilerek bozuldu; iki platformda da aynı adım düştü, üç varyantta; sonra geri alındı (kayıtlar `.run/break-web-0032.log`, `.run/break-desk-0032.log`).

- **Nokta, kenet:** nokta aracı kenetlenmez yapıldı. `point-series` 2. adımda (`snap: null`, beklenen `endpoint`) ve 3. adımda düştü: nokta imlecin ham yerinde kaldı (web `[-23.5, -11.69]`, masaüstü `[-23.44, -11.63]`), beklenen uç `[-24, -12]`.
- **Daire, Çap:** yazılan çap yarıya bölünmedi. `circle-methods` 8. adımda düştü (`radius: 6`, beklenen 3).
- **Yay, Devam:** devam yönü ters çevrildi. `arc-variants` 30. adımda düştü: yay çizginin ucundan ters yöne gitti, uçları ters sırada (`[[-15, -5], [-15, -15]]` yakınında), beklenen `[[-15, -15], [-15, -5]]`.
- **Dikdörtgen, köşe biçimi:** köşeler işlenmedi. `rect-options` 8. adımda (4 köşe, 0 yay; beklenen 8 köşe, 4 yay) ve 28. adımda (pahsız 4 köşe, beklenen 8) düştü.
- Masaüstünde öbür dokuz iz bu bozmalarda da üç varyantta geçti. Web'de bozmalar dört yeni izle oynatıldı.

## Sonuçlar

- **Testler:**
  - `kentos-native-application`: 158 ortak durum, katalog eşitliği.
  - `kentos-interaction`: 69 test; şekil araçlarının 16'sı (`tests/shapes.rs`: yöntemler, bellek, istemdeki değerler, teğet seçimi, Devam) dahil. Ölçüm testi elle koşulur.
  - Geometri çekirdeği: `corners_of_ring` ve `nearest_edge` birim testleri, 65 yeni çağrı durumu (native ve WASM), 3 bağımsız referans.
  - Masaüstü: 13 iz üç varyantta; şerit yöntem menüsünün envanterden okunması; yöntemin aracı seçeneğiyle başlatması.
  - Web: 158 ortak durum, çağrı durumları; `pnpm e2e`'nin döndürülmüş dikdörtgen, düzgün çokgen ve teğet devam eden yay denetimleri artık komuttan yazan araçlarla geçer.
- **Bir düzeltme:** masaüstünün “taşınmamış komut” birim testi `tool.circle`'ı kullanıyordu; `8682517`'den beri düşüyordu, dilimin sonundaki tam koşuda görüldü. Artık `tool.ellipse`'i kullanır.
- **Bağımlılıklar:** yeni paket yok. `Cargo.lock` ve `pnpm-lock.yaml` değişmedi.
- **Dosya düzeni:** her araç kendi dosyasında, web'in bir sınıfının karşılığıdır. `arc.rs` (482), `circle.rs` (471) ve `rectangle.rs` (422) 400 satırı aşar (CLAUDE.md §8): her biri tek bir durum makinesidir, bölmek onu dağıtırdı. Ortak parçalar `points.rs`'tedir (`Taken`: alınan noktalar, üzerine gelme, izleme, Ctrl+Z; `chain_preview`, `write_ring`).
- **Ölçüm yapılmadı.** Önizleme hareket başına bir yapı kurar, eğriyi çekirdek parçalar (daire 96 parça). Depo sorgusu kenetten başka yalnız teğet seçiminde, tıklamada yapılır.

## Ertelenenler

- **Şeritte son seçilen yöntemin hatırlanması (sahibin sorusu).** Web, Daire ▾ ve Yay ▾ düğmesinin üstünde son seçilen yöntemi tutar ve oturumlar arasında hatırlar. Masaüstünde yerleşim kalıcılığı henüz yoktur. Seçenekler:
  - kalıcı yerleşim deposu gelene kadar düğme ilk yöntemi başlatsın, menü hepsini versin (önerilen; bugünkü durum);
  - seçim uygulama açık kaldıkça tutulsun, dosyaya yazılmasın;
  - `ayarlar.json`'a bir yerleşim alanı eklensin (ayar değil, yerleşimdir; CLAUDE.md §4.4).
- Menüde yöntemin açıklaması (ipucu): KentOS UI menüsüne ipucu alanı gerekir.
- **Kot noktası aracı (`tool.spot`) masaüstünde yok.** Komutu hazırdır (`z`, `label`, `attrs`); aracın kot sorusu ve `kot` katmanı yoksa ne yapılacağı ayrı iştir.
- Öbür çizim araçları: elips, eğri, halka, yardımcı çizgi ve ışın, paralel çizgi, dik in/çık, bölme, yazı, ölçü, bulut, tarama.
- Yay düğmesinin simgesi masaüstünde dairenin simgesidir: KentOS UI'da yay simgesi yok (`icons.rs`).
- Yöntem başına ürün komutu (ör. üç noktadan daire) Python ya da AI isterse ayrı karardır; bugün araç yapıyı çekirdekten bulur, komut yazar.

## Doğrulama (26 Eylül 2026, Linux; main `2232267` üstünde, dal `worktree-agent-afc824c9621543135`)

- `cargo fmt --all --check` temiz.
- `pnpm rust:test`: 632 test geçti, 2 ölçüm testi atlandı (elle koşulur); clippy temiz; bağımlılık yönü temiz (19 crate, 25 crate × hedef).
  - Veritabanı testleri yerel sunucuda, geçici veritabanlarında çalıştı: `KENTOS_TEST_DB=required cargo test -p kentos-application -p kentos-postgres -p kentos-api` 86 test geçti.
- `pnpm rust:test:desktop`: masaüstü 54 (1 ölçüm testi atlandı), render 29, KentOS UI 171, vitrin 55 test geçti; clippy temiz.
- `cargo test -p kentos-desktop traces`: 13 iz × 3 varyant ve komut satırı modeli geçti.
- `cargo test -p kentos-native-application`: 158 ortak durum ve masaüstüne özgü testler geçti. `cargo test -p kentos-interaction`: 69 test geçti.
- `pnpm typecheck` temiz. `pnpm test`: 1272 geçti, 13 atlandı (başlangıçtaki 13).
- `pnpm inventory:check` güncel.
- `pnpm e2e:interaction`: 13 iz × 3 varyant geçti. `pnpm e2e`: 161 denetimin hepsi geçti.
- Görüntüler (`KENTOS_SNAPSHOT_BACKEND=wgpu kentos-cad snapshot --iz … --adim …`): uca kenetlenen nokta; yarıçapı gösterilen daire; teğet dairenin seçilen kareleri ve yarıçap istemi; üç noktalı yay; çizgiye teğet Devam; köşe yuvarlama değeri istemde; 30° döndürülmüş 10 × 5 dikdörtgen; döndürülmüş dikdörtgen; düzgün çokgen; Daire ▾ ve Yay ▾ yöntem menüleri (`--tikla`, `--imlec`).
- Pencerede (`make desktop`) elle klavye ve fare denemesi bu çalışmada yapılmadı.
