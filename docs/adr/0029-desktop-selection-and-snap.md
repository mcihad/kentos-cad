# ADR 0029: Masaüstünde seçim, kenet ve silme; `cad.entities.delete` v1

- **Durum:** kabul edildi (2026-09-26). Yön ADR 0021'in ertelenenlerinden (seçim, kenet) ve TODOS.md `UX-07`, `UX-09`, `CMD-07`'den gelir. Deponun masaüstünde nasıl izleneceği, kenet ve seçim kurallarının kaynağı, vurgunun nerede çizileceği, silme komutunun sözleşmesi ve izlerin yeni alanları bu dilimin kararıdır.
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** TODOS.md `UX-07`, `UX-09`, `CMD-04`, `CMD-07`, `UI-11`, `SET-06`, `REN-09`, `REN-11`, `REN-13`; ADR 0013 (ürün komutu sözleşmesi), 0014 (kalıcı kimlik), 0018 (araç oturumu ve izler), 0020 (native belge), 0021 (native araç oturumu), 0022 (ilk ürün komutu), 0023 (tipli ayarlar), 0027 (çizgi ve çoklu çizgi)

## Bağlam

- Masaüstünde seçim, kenet ve silme yoktu. `tool.select`, `tool.erase` ve `draft.snap` şeritte soluk duruyordu. Araçlar imlecin ham noktasını alıyordu. Native iz oynatıcısı kenedi açık bir izi “desteklenmiyor” diyerek durduruyordu.
- Web'de seçme, kenet ve pencere seçimi ortak Rust deposundan gelir (`kentos_geometry_core::store`). Web onu WASM'dan besler: nesneler paketlenir (`packEntities` → `putPacked`), belgenin `touched` olayı değişenleri taşır (`viewport/picking.ts`, `PickIndex`).
- Web'in Sil aracı (`EraseTool`) belgeye doğrudan yazıyordu (`doc.remove`). Kilit kuralı aracın içindeydi (koddan doğrulandı, 26 Eylül):
  - kilitli katmandakiler silinmez, “N nesne kilitli katmanda olduğu için silinmedi.” uyarısı verilir;
  - öbürleri silinir;
  - hepsi kilitliyse hiçbir şey silinmez.
- Kenet ve seçim ayarları (`drafting.snap`, `snap.*`, `drafting.pickAperture`) şemada yalnız web'indi (ADR 0023).

## Karar

### Geometri deposu masaüstünde

- Masaüstü aynı depoyu kullanır. Depoya iki tipli giriş eklendi: `put_many` (nesneleri koyar, sonra dizini gerekirse yeniden kurar) ve `set_layers` (katman tablosu). JSON girişleri artık bunları çağırır; test `typed_puts_and_layers_are_the_json_ones`. WASM arayüzü değişmedi.
- Depoyu `kentos_interaction::Spatial` tutar. Sorguları yuvayla cevaplar: `pick`, `in_rect`, `snap`, `extent`.
- **Nesne → depo şekli tek yerde:** `spatial::record`. JSON yoktur. Ortak durumlar `fixtures/store-records/v1/cases.json` (20 kayıt, bütün nesne türleri). Web `packEntities` ile (`viewport/packedRecords.test.ts`), masaüstü `record` ve depo paketleyicisiyle (`tests/records.rs`) aynı sayıları ve metinleri vermelidir.
- **Belgenin günlüğü** (`kentos_domain::changes`) web'in `touched` olayıdır:
  - Nesne ekleyen, değiştiren ya da silen her uygulanmış işlem yuvasını yazar: düzenleme, geri alma, yineleme, geri sarılan işlem, iptal edilen grup.
  - Okuyucu bir `ChangeMark` tutar. `changes_since` o işaretten beri değişen yuvaları verir; günlük o kadar geriye uzanmıyorsa `All` der.
  - Günlük sınırlıdır: en az 65 536 kayıt tutar, çizimin iki katını aşınca boşaltılır. O noktada her şeyi yeniden okumak izlemekten pahalı değildir.
- **Eşitleme:** yuvalar tekrarsız, ilk değiştikleri sırayla alınır. Gidenler silinir, kalanlar konur. Depo boşalan yeri geri gelen nesneye verir; belge sırası korunur. Katman tablosu yalnız değişince gönderilir. Sürüm ve işaret yerindeyse hiçbir şey okunmaz.
  - Eşitleme yerleri: her mesajdan sonra (oturum ya da sürüm değişince), her imleç sorgusundan ve her araç çağrısından önce.
  - Her şey yalnız iki durumda yeniden okunur: çizim açılınca ve günlük geride kalınca. Testler bunu sayar (`reloads`).
- **Katmanlar web'in `layerTable`'ıdır:** görünürlük ve kilit üstteki gruplarla çözülür, iç seçimi (`pickInterior`) ve etiket kuralı gider.
- Hesap depodadır. Masaüstünde yeni geometri kodu yoktur (CLAUDE.md §4.8.1, §14).

### Kenet

Kurallar web'in `updateSnap` ve `constrainPoint` kurallarıdır:

- **Ne zaman:** çalışan araç kenetlenir (kapalı alan, çizgi, çoklu çizgi) ve Kenetleme açıktır (`drafting.snap`, F3, oturum ayarı). Seçim ve Sil araçları kenetlenmez.
- **Türler** `snap.*` ayarlarından gelir. Uç nokta ayarı çeyrek noktaları da açar (ayarın metni öyle der). En yakın varsayılan olarak kapalıdır.
- **Açıklık:** `drafting.snapAperture` piksel, görünümün ölçeğiyle dünya uzunluğuna çevrilir. Dik ve teğet, aracın son noktasından hesaplanır (`snap_from`).
- **Basışta ve bırakışta yeniden hesaplanır.** Son hareketin sonucu kullanılmaz (CLAUDE.md §4.7).
- **Kenetlenen nokta kesindir:** orto ve kutupsal izleme onu kaydırmaz.
- **Gizli katman kenetlenmez; kilitli katman kenetlenir.** Kilit yalnız düzenlemeyi engeller.
- **İşaret** web'in biçimleridir (DESIGN.md: uç kare, orta üçgen, merkez ve nokta daire, çeyrek eşkenar dörtgen, kesişim çarpı, dik açı, teğet, en yakın kum saati).
  - Renk `--canvas-snap`: koyu temada `#6fd08c`, açıkta `#1a9a48`.
  - Türün adı işaretin sağ üstünde, alanın renginde haleyle yazar.
  - İşaret Iced'in canvas'ında çizilir (`apps/desktop/src/marks.rs`); birkaç şekildir.
- **Ayarlar:** `drafting.snap`, sekiz `snap.*` ve `drafting.pickAperture` artık `web` ve `desktop` host'larındadır; `settingsSchema.json` yeniden üretildi. Masaüstü ayar penceresine “Kenetleme” bölümü eklendi: sekiz tür, kenet ve seçim yarıçapı.
- **Masaüstünde henüz yok:** tek seferlik kenet, nesne izleme (Shift+F3), ızgaraya kenet.

### Seçim

Kurallar web'in `SelectTool`'udur. Komut çalışmazken fare seçim aracınındır (`kentos_interaction::select`):

- **Tıklama:** seçim yarıçapı (`drafting.pickAperture`, 5 px) içindeki en belirgin nesne seçilir: önce nokta ve kenarlar, sonra içinde kalınan en küçük alan. `pickInterior: false` katmanlar yalnız kenarından seçilir.
  - Tıklama seçimin yerine geçer. Shift ile tıklama nesneyi seçime ekler ya da çıkarır. Boşluğa tıklama seçimi kaldırır; Shift ile kaldırmaz.
- **Kutu:** basılı tutup 4 pikselden fazla sürüklemek kutu çizer.
  - Soldan sağa pencere: tamamı içeride kalanlar. Sağdan sola kesişim: kutuya dokunanlar.
  - Shift ile kutu seçime ekler, yoksa seçim olur. Sıra belge sırasıdır.
  - Görünüşü ve sorguyu tek kural belirler: `SelectBox::crossing`.
- **Ctrl'ün seçimde anlamı yoktur**, web'de de yoktur.
- **Esc**, komut yokken seçimi kaldırır.
- **Üzerine gelme:** tuş basılı değilken imlecin altındaki nesne vurgulanır. Sil aracı da silinecek nesneyi vurgular.
- **Gizli katmanın nesnesi** seçilmez: tıklama da kutu da atlar. **Kilitli katmanınki seçilir**; Sil onu yerinde bırakır.
- Silinen nesne seçimden düşer: her belge değişikliğinden sonra (web: `changed` olayında `retain`).
- **Komutlar:**
  - `edit.selectAll` (Ctrl+A): görünür katmanlardaki her nesne; “N nesne seçildi.” der.
  - `edit.deselect`.
  - `edit.invertSelection`: görünür katmanlarda.
- **Kutunun görünüşü** web'in `drawSelectionBox`'ıdır, Iced canvas'ında çizilir:
  - pencere `#6DB3F2`, düz çizgi;
  - kesişim kenet renginde, 5/4 kesikli;
  - ikisinde de %10 dolgu.
- **Vurgu** web'in `uploadHighlight`'ıdır:
  - Seçim vurgu renginde çizilir, kapalı alanlarda %13 dolgu, noktalarda 15 px halka.
  - Üzerine gelinen nesne vurgu renginde %85, dolgusuz. Seçiliyse ayrıca vurgulanmaz.
  - İkisi de wgpu sahne parçasıdır, bütün katmanlardan sonra çizilir (`kentos_render_wgpu::scene::build_highlight`, `apps/desktop/src/viewport/highlight.rs`). Ölçüm ve gerekçe aşağıda.
  - Parça yalnız gösterdiği değişince kurulur: seçim, üzerine gelinen nesne, çizim, vurgu rengi ya da eğri bandı. Kaydırma ve band içinde yakınlaştırma GPU'dakini çizer. Üzerine gelme yalnız kendi parçasını kurar, seçiminkini kurmaz (`a_hover_builds_only_the_hover_highlight`).
- **Özellikler paneli** seçimi söyler:
  - Tek nesne: Nesne, Tür (etiketiyle), Katman (grup yoluyla; kilitliyse “(kilitli)”), Renk, Uzunluk ya da Alan, öznitelikler.
  - Birden çok nesne: “N nesne seçili”, türler (ilk görülme sırasıyla: “3 çizgi, 1 kapalı alan”), ortak katman ya da “Çeşitli” ya da “Kilitli katman içeriyor”, renk, Toplam uzunluk, Toplam alan.
  - Toplamları depo hesaplar (`measure`), web gibi.
  - Panelde düzenleme yoktur. Durum çubuğu “N seçili” der.
- **Web'le fark:** masaüstünde seçimin çizgisi düzdür. Web'de 6/3 kesiklidir (DESIGN.md). wgpu çizim hattında kesik çizgi yoktur (`REN-11`). Varsayılan: `REN-11`'e kadar düz; ertelenenlere bakın.

### Silme: `cad.entities.delete` v1

| Alan | Değer |
|---|---|
| `title` | Nesneleri sil |
| `effect`, `hosts`, `headless`, `requires` | `document`; `web`, `desktop`; evet; `document` |
| `permissions`, `undo`, `cost` | yok; `step` (“Sil”); `instant` |
| `aliases` | yok: `E`, `ERASE`, `SIL` arayüz komutu `tool.erase`'ındır |
| `examples` | iki nesneyi silme; bir nesneyi yalnız planlandığı sürümde silme |

- **Girdi** `EntitiesDelete { uids, expectedRevision? }`. `uids` kalıcı kimliklerdir (ADR 0014): küçük harfli, tireli UUID yazısı.
- **Çıktı** `EntitiesDeleted { removed, locked, revision }`: silinenler girdinin sırasıyla, kilitli katmanda kalanlar, silmeden sonraki sürüm. Plan `EntitiesDeletePlan` aynı biçimdedir ve planın sürümünü verir.
- **Neden yuva değil kalıcı kimlik:** yuva bir oturumun iç sayısıdır. Python, AI ve bulut nesneyi kimliğiyle bilir. Geri alma nesneyi aynı kimlikle getirir, aynı komut onu yine silebilir.
- **Seçim girdide açıkça verilir (`CMD-07`).** Komut seçimi okumaz. Sil aracı ve Delete tuşu seçimi ya da tıklanan nesneyi kimlik listesine çevirir.

**Denetimler ve sıraları** (ilk tutmayan cevap verir):

| Sıra | Kod | Durum | Yol |
|---|---|---|---|
| 1 | `no_entities` | failed | `uids` |
| 2 | `invalid_uid` (ilk bozuk kimlik) | failed | `uids[i]` |
| 3 | `invalid_revision`, `revision_conflict` | failed, conflict | `expectedRevision` |
| 4 | `entity_not_found` (ilk bulunmayan) | failed | `uids[i]` |
| 5 | `layer_locked`: hepsi kilitli | failed | `uids` |
| — | `layer_locked`: bir kısmı kilitli | uyarı | `uids` |

- Aynı kimlik iki kez verilirse bir kez sayılır. Sürüm denetimi ortak dosyalardandır (`checks.rs`, `checks.ts`).
- **Yeni iletiler** (tam metin durum dosyasında):
  - `Silinecek nesne verilmedi. En az bir nesnenin kalıcı kimliğini verin.`
  - `“…” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (…). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın.`
  - `“…” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.`
  - `N nesne kilitli katmanda olduğu için silinmedi. Silmek için katmanın kilidini Katmanlar panelinden açın.`
- **Kilitli katman web'in bugünkü kuralıdır:** kilitliler kalır, öbürleri uyarıyla silinir; hepsi kilitliyse hiçbir şey silinmez. Üç küçük fark kayıtlıdır:
  - İleti çözümü de söyler (“Silmek için … açın.”, CLAUDE.md §8).
  - Hepsi kilitliyken sonuç `failed`'dır; araç yine yalnız uyarı gösterir.
  - Çizimde olmayan kimlik artık reddedilir. Eski araç onu atlıyor ve kilitli sayıyordu. Arayüzde görünmezdi: seçim silinen nesneyi hep bırakır.
- **Gizli katmandaki nesne silinir, uyarısız,** web'in bugünkü davranışı gibi. Durum dosyası bunu tutar.
- **Yürütme** belgenin kendi `remove`'udur: tek geri alma adımı, adı “Sil”. Açık işlem ya da grup varsa ona katılır (`tests/delete.rs`). Geri alma nesneleri aynı kimlik ve aynı yerle getirir. İz bunu belge sırasıyla denetler (`ids`).
- **Ortak durumlar:** `fixtures/commands/v1/cad.entities.delete.json` (23 durum), iki koşucu. Var olan nesnenin kimliği için yeni yer tutucu: `captureUid` ile alınır, `$uid:ad` ile yazılır (bir iletinin içinde de).
- **Araçlar komuttan siler:**
  - Web: `EraseTool.eraseIds` → `entitiesDelete.execute`. Uyarıları ve reddi uyarı olarak yazar. Başarıda “N nesne silindi.” der. Katalogda `tools[erase].productCommand`.
  - Masaüstü: Sil aracı (`kentos_interaction::erase`). Seçim varken başlayınca siler ve çıkar. Seçim yokken tıklananı siler, üzerine geleni vurgular. Delete tuşu (`tool.erase`'ın kısayolu) iki platformda da onu başlatır.

### Web'de bir değişiklik: komutun adı önce yazılır

- `ToolManager` komutun adını artık aracı başlatmadan **önce** geçmişe yazar (`activate`, `run`, `nest`). Sil, seçimi başlarken siler; “Sil” satırı “N nesne silindi.” satırından önce gelmeliydi. Masaüstü de bu sırayla yazar.
- `select-delete` izi en yeni iletinin düzeyini okur (`log: success`). Başka davranış değişmedi; bütün izler ve `pnpm e2e` geçti.

### Etkileşim izleri

`fixtures/interaction/v1`'e iki iz ve bir çizim eklendi. Web ve masaüstü onları değiştirmeden, üç varyantta geçer.

| İz | Ne tutar |
|---|---|
| `snap-polygon` | kapalı alanın köşeleri uç, orta ve kesişim noktalarına tam oturur; orto (F8) kenetlenen noktayı kaydırmaz; F3 keneti kapatır; gizli katman kenetlenmez, kilitli katman kenetlenir; nokta nesnesi; seçim aracı kenetlenmez |
| `select-delete` | üzerine gelme; tıklama; Shift ile ekleme ve çıkarma; pencere ve kesişim; Esc; gizli ve kilitli katman; Delete, tek adım, Ctrl+Z ile aynı nesneler aynı yerde; kilitli nesne kalır; seçimsiz Delete tıklananı siler |

- `objects.kcad`: üç çizgi (2 ile 3 (9,6; 8,8)'de kesişir), kapalı alan, nokta, kilitli ve gizli katmanda birer çizgi.
- **Biçim eklemeleri:**
  - eylem `drag` ve `click`/`drag` için `shift`;
  - tuşlar `Delete`, `F3`, `F8`;
  - beklentiler `selected`, `hover`, `snap`, `ids`.
- **Taslak ayarları:** kenet türleri, yarıçaplar ve kutupsal adım ayarların varsayılanlarıdır. Masaüstü oynatıcısı `draft`'ı tipli ayarlardan uygular (`drafting.ortho`, `drafting.polar`, `drafting.snap`, `drafting.cursorInput`). Izgara ya da nesne izleme açık bir izde açık bir iletiyle durur.
- **Oynatıcılar:** web `interaction.mjs` (sürükleme, Shift, yeni tuşlar; kenedi `window.kentos.view.currentSnap`'ten okur). Masaüstü `traces/` (aynısı).
- **Görüntü:** `kentos-cad snapshot --iz <iz> --adim <n> --yarida` son adımı yarıda bırakır: sürükleme tuş basılıyken durur, seçim kutusu görünür.

### Taşınan komutlar

`tool.select`, `tool.erase`, `draft.snap`, `edit.deselect`, `edit.selectAll`, `edit.invertSelection` (`apps/desktop/ported.json`). Masaüstünde 30 / 164 komut çalışıyor.

### Ortak olan, ortak olmayan

| | Web | Masaüstü | Ortak |
|---|---|---|---|
| Depo | WASM, paketlenmiş kayıtlar | native, `spatial::record` | `kentos_geometry_core::store`; `fixtures/store-records/v1` |
| Değişenler | `touched` olayı | belgenin günlüğü (`changes_since`) | — |
| Kenet, seçim kuralları | `ViewportController`, `SelectTool`, `tracking.ts` | `kentos-interaction` (`session.rs`, `select.rs`, `points.rs`) | `fixtures/interaction/v1` |
| Silme | `product/entitiesDelete.ts` | `native/application/src/delete.rs` | sözleşme tipleri; `fixtures/commands/v1` |
| Vurgu | `__sel`, `__hover` katmanları (WebGL2/WebGPU) | wgpu sahne parçaları | renkler, alfa, halka boyu |
| Kutu ve kenet işareti | 2D canvas | Iced canvas | biçimler, renkler |

## Ölçümler

Makine: Intel Core i5-11300H (4 çekirdek, 8 iş parçacığı), 15 GB, Linux 7.0, rustc 1.96.0, `--release`, `4afe640`. Çizim `side × side` parsel (beş köşeli, her biri biraz farklı) ve `side` yol çizgisidir. Tek koşudur; ortalamalar 200 (düzenleme) ve 2000 (sorgu) tekrarındır, öbür satırlar tek ölçümdür.

**Depo** (`cargo test --release -p kentos-interaction --test perf -- --ignored --nocapture`):

| | 10 100 nesne | 100 172 nesne |
|---|---|---|
| Belgeyi açma (karşılaştırma için) | 18,9 ms | 173,4 ms |
| Depoya ilk okuma | 5,9 ms | 47,5 ms |
| Bir nesne ekle + eşitle / geri al + eşitle | 1,8 / 1,3 µs | 1,4 / 1,0 µs |
| 1000 nesne sil + eşitle / geri al + eşitle | 0,72 / 1,96 ms | 0,74 / 0,49 ms |
| Değişiklik yokken eşitleme | 0,1 µs | 0,05 µs |
| Katman görünürlüğü + eşitleme | 0,003 ms | 0,001 ms |
| Kenet, yakın (11 px = 1,375 m): ortalama / en kötü | 1,9 / 53 µs | 6,0 / 27 µs |
| Kenet, son noktadan dik ve teğetle | 1,9 µs | 6,1 µs |
| Seçme (5 px), yakın | 1,7 µs | 5,6 µs |
| Kenet, bütün çizim ekranda: ortalama / en kötü | 6 / 12 µs | 23 / 32 µs |
| Seçme, bütün çizim ekranda | 3 µs | 11 µs |
| Pencere (alanın %9'u) / kesişim | 900 nesne 0,04 ms / 991 nesne 0,04 ms | 8 836 nesne 0,37 ms / 9 119 nesne 0,31 ms |

- Her şey yalnız çizim açılınca bir kez okundu (`reloads == 1`). Bir kenet sorgusu ortalama 2–23 µs, en kötü 53 µs sürdü: 16 ms'lik karenin binde üçü.

**Vurgu: wgpu parçası mı, Iced canvas'ı mı** (`cargo test --release -p kentos-desktop highlight_costs -- --ignored --nocapture`). Bütün nesneler seçili, çizimin tamamı 1400 × 800 alanda. Canvas satırı, web'in kesikli seçim çizgisini (6/3) ve %13 dolguyu Iced'in canvas'ında bir karede kurmanın CPU süresidir (`Frame` → geometri). GPU'nun çizim süresi iki yolda da ölçülmedi.

| Seçili | Sahne | wgpu parçası (bir kez) | Parça boyu | Kare başına CPU, wgpu | Kare başına CPU, canvas |
|---|---|---|---|---|---|
| 1 056 | 0,2 ms | 0,4 ms | 365 KB | 0 (önbellekten) | 2,5 ms |
| 10 100 | 2,3 ms | 3,7 ms | 3,6 MB | 0 | 14,8 ms |
| 100 172 | 39,4 ms | 41,7 ms | 35,7 MB | 0 | 109,2 ms |

- **Karar:** seçim ve üzerine gelme wgpu parçasıdır. Canvas'ın yolu ekran koordinatındadır; her kaydırma ve yakınlaştırma karesinde yeniden kurulur. 10 bin seçili nesnede kare başına 15 ms eder ve 16 ms'lik kareyi yer. Parça seçim değişince bir kez kurulur, sonra CPU'ya karede bir şey ödetmez.
- **Bedeli:** parça GPU'da yer tutar (100 bin seçili nesnede 36 MB) ve seçim değişince yeniden kurulur (100 binde 42 ms, sahne kurmayla aynı düzey).
- **Canvas'ta kalanlar:** kenet işareti ve seçim kutusu. Birkaç şekildir, her hareketle zaten değişirler.

## Ters deneme

Her kural bir kez bilerek bozuldu; iki platformda da aynı iz adımı ya da aynı durumlar düştü; sonra geri alındı.

- **Kenet:** kenetlenen noktaya orto uygulandı (web `constrainPoint`, masaüstü `points::constrain`). `snap-polygon` iki platformda, üç varyantta 14. adımda düştü: dördüncü köşe `[-12, 8.8]`, beklenen `[-12, 4]`. 21. adımda da düştü (`hover: 8`, beklenen 1).
- **Seçim:** kesişim yönü ters çevrildi. Web `select-delete`'te 9–12. adımlarda düştü (9. adım: `selected: []`, beklenen `[2, 3]`).
  - Masaüstünde ilk denemede iz geçti. Sorgu yönü kendi karşılaştırmasından alıyordu; bozulan yalnız kutunun görünüşüydü.
  - Düzeltme (`b1b8585`): görünüşü ve sorguyu tek kural belirler (`SelectBox::crossing`). Yeniden bozulunca masaüstü de aynı adımlarda, aynı değerlerle düştü.
- **Silme:** iki işleyicide kilit denetimi kaldırıldı. Ortak durumlardan aynı 7'si iki koşucuda düştü (89 durumdan); ör. “kilitli katmandaki nesne tek başına silinmez”. `select-delete` iki platformda 21. adımda düştü (`ids: [1,2,4,5,7]`, beklenen `[1,2,4,5,6,7]`).

## Sonuçlar

- **Testler:**
  - `kentos-native-application`: 89 ortak durum (23 + 20 + 23 + 23), katalog eşitliği, silmenin işleme katılması.
  - `kentos-interaction`: 51 test; seçimin 13 kuralı, depo kayıtları dahil. Ölçüm testi elle koşulur.
  - `kentos-domain`: günlük testi.
  - Geometri deposu: tipli ve JSON girişlerin eşitliği.
  - Masaüstü: 49 test (ölçüm hariç). Dokuz iz üç varyantta, seçim komutları, özellikler paneli, F3 ve ayarlar, deponun uygulamayı izlemesi, vurgunun önbelleği bunlara dahil.
  - Render: vurgu parçası (`tests/scene.rs`).
  - Web: 89 ortak durum, 20 depo kaydı, kayıt eşitliği.
- **Bağımlılıklar:** yeni paket yok. `Cargo.lock` değişmedi.
- **Dosya düzeni:** vurgu `apps/desktop/src/viewport/highlight.rs`'tedir (`viewport.rs` 1312 satıra çıkmıştı). Seçim komutları ve özellikler paneli `selecting.rs`, çizim alanındaki işaretler `marks.rs`'tedir.

## Ertelenenler

- **Seçim çizgisinin kesikli olması (sahibin sorusu).** DESIGN.md seçimi 6/3 kesikli ister; masaüstü düz çizer. Seçenekler:
  - `REN-11` wgpu hattına kesik çizgiyi getirene kadar düz kalsın (önerilen; karede maliyet yok, tek görsel fark);
  - kesikli çizgi şimdi Iced canvas'ında çizilsin (10 bin seçili nesnede kare başına 15 ms);
  - DESIGN.md seçimi iki platformda da düz yapsın.
  - **Varsayılan (26 Eylül, birleştirmede):** önerilen seçenek uygulanır; `REN-11`'e kadar düz. Sahip başka bir seçenek isterse değişir.
- Tutamaçlar ve tutamaçla düzenleme; yazı ve ölçünün çift tıkla düzenlenmesi; seçim aracında basılı sağ tık menüsü; nesne bilgi kartı (`drafting.hoverInfo`); bağlamsal Seçim sekmesi (DESIGN.md); özelliklerin panelde düzenlenmesi.
- Tek seferlik kenet, nesne izleme, ızgaraya kenet.
- Gizli katmandaki seçili nesnenin uyarısız silinmesi web'in bugünkü davranışıdır. Değişecekse önce karar, sonra iki uygulama ve durum dosyası.
- Bulut projesinde silme web'de yerel belgeden `project.changes`'e gider (ADR 0026). Masaüstünde bulut yoktur.

## Doğrulama (26 Eylül 2026, Linux; main `d26b5eb` üstünde, dal `worktree-agent-afc824c9621543135`)

- `cargo fmt --all --check` temiz.
- `pnpm rust:test`: 534 test geçti, 1 ölçüm testi atlandı (elle koşulur); clippy temiz; bağımlılık yönü temiz (18 crate, 23 crate × hedef).
  - Veritabanı testleri yerel sunucuda çalıştı: `KENTOS_TEST_DB=required cargo test -p kentos-application -p kentos-postgres -p kentos-api` 58 test geçti.
- `pnpm rust:test:desktop`: masaüstü 49, render 29, KentOS UI 171, vitrin 55 test geçti; clippy temiz.
- `cargo test -p kentos-desktop traces`: 9 iz × 3 varyant ve komut satırı modeli geçti.
- `cargo test -p kentos-native-application`: 89 ortak durum ve masaüstüne özgü testler geçti. `cargo test -p kentos-interaction`: 51 test geçti.
- `pnpm typecheck` temiz. `pnpm test`: 1164 geçti, 13 atlandı (başlangıçtaki 13).
- `pnpm inventory:check` güncel.
- `pnpm e2e:interaction`: 9 iz × 3 varyant geçti. `pnpm e2e`: 160 denetimin hepsi geçti.
- Görüntüler (`KENTOS_SNAPSHOT_BACKEND=wgpu kentos-cad snapshot --iz`): kapalı alan çizilirken uç ve kesişim kenedi; pencere ve kesişim kutusu (`--yarida`); vurgulu seçim ve özellikler paneli; üzerine gelme.
- Pencerede (`make desktop`) elle klavye ve fare denemesi bu çalışmada yapılmadı.
