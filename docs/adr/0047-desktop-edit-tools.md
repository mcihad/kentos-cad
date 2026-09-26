# ADR 0047: Masaüstünde değiştirme araçları, 2. tur: kenar, köşe ve nesne araçları, esnetme, diziler ve hizalama; `cad.entities.edit` v1, `cad.entities.array` v1

- **Durum:** 1. kısım kabul edildi (2026-09-26): Ötele, Buda, Uzat, Köşe yuvarla, Pah, Kır, Birleştir, Patlat, Uzat-kısalt, Köşe ekle/sil. 2. kısım kabul edildi (2026-09-26): Esnet, Dizi, Kutupsal dizi, Hizala; `cad.entities.array` v1, `cad.entities.transform` v1'e hizalama, `cad.entities.edit` v1'e ölçü ve tarama. Yön ADR 0037'nin ertelenenlerinden (öteki değiştirme araçları) ve TODOS.md `UX-01`, `CMD-04`, `CMD-07`, `CAD-02`'den gelir.
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** TODOS.md `UX-01`, `UX-07`, `CMD-04`, `CMD-07`, `CAD-02`; ADR 0008 (ortak çekirdek sınırı), 0013 (ürün komutu sözleşmesi), 0014 (kalıcı kimlik), 0018 (araç oturumu ve izler), 0021 (native araç oturumu), 0022 (ilk ürün komutu), 0029 (seçim, kenet, silme), 0032 (çizim araçları), 0037 (değiştirme araçları, 1. tur)

## Bağlam

- Masaüstünde yalnız taşı, kopyala, döndür, ölçekle ve aynala vardı (ADR 0037). Web'in öbür değiştirme araçları şeritte soluk duruyordu.
- Web'in kenar, köşe ve nesne araçları belgeye kendileri yazıyordu (koddan doğrulandı, 26 Eylül):
  - `EdgePickTool.replace`: ilk parça `doc.replace` ile nesnenin yerinde ve kimliğiyle, öbür parçalar `doc.add` ile yeni nesne; tek parçada öznitelik ve etiket kalır (kapalı alanda kalmaz); hepsi bir `transact` içinde;
  - Ötele `doc.add`, Uzat `doc.update` ile işlemsiz yazıyordu: geri alma adımları “Ekle” ve “Değiştir” adını taşıyordu;
  - Köşe yuvarla, Pah, Uzat-kısalt ve aynı türde kalan Köşe ekle/sil `doc.update` ile yama birleştiriyordu; Birleştir ve Patlat `SelectionActionTool` üstünde doğrudan yazıyordu.
- Hesabın çoğu zaten çekirdekteydi (`trim_entity`, `extend_entity`, `offset_entity`, `break_entity`, `fillet_lines`, `chamfer_lines`, `corner_of_path`, `join_entities`, `explode_entity`, `lengthen_entity`, `insert_vertex`, `remove_vertex`; kırpma ve uzatmanın sınırlarını geometri deposu topluyordu). **TypeScript'te üç hesap kalmıştı:** Ötele'nin “Noktadan geç” uzaklığı (kenarlara en küçük uzaklık), köşe araçlarının imleç altındaki köşeyi araması (`cornerAt`) ve Köşe ekle/sil'in iç halka denetimi (`nearHole`).

## 1. kısım: Ötele, Buda, Uzat, Köşe yuvarla, Pah, Kır, Birleştir, Patlat, Uzat-kısalt, Köşe ekle/sil

### Kalan üç hesap ortak çekirdekte

Üçü `kentos-geometry-core`'a taşındı; web onları WASM'dan çağırır, TypeScript kopyaları silindi (CLAUDE.md §4.8.1, §14):

- `offsetThroughDistance` (`ops/offset.rs` `through_distance`): noktanın nesnenin en yakın kenarına uzaklığı; kenarsız nesnede +∞.
- `cornerNear` (`tools/editing.rs` `corner_near`): verilen adaylar arasında imlece en yakın köşe; çoklu çizginin ya da kapalı alanın köşesi, ya da uçları `same` (2 piksel) içinde buluşan iki çizgi; eşitlikte ilki; her çizgi uzak ucunun tarafını tutar. Köşenin geometrisi yine `vertex_corner`, `lines_corner_at`'tir.
- `nearHole` (`ops/vertex.rs` `near_hole`): tıklama bir deliğe dış halkadan yakın mı.
- **Silmeden önce yan yana deneme:** üç işlemde 60 000'den çok rastgele çağrı ve adlı durumlar, sıfır toleransla TypeScript aslıyla bit bit aynı (1 174 çizgi köşesi, 3 636 köşe noktası, 2 119 delik, 16 057 sonlu uzaklık dahil; `.run/side-by-side-0047.log`, tek seferlik test, sonra silindi).
- **Çağrı durumları:** `fixtures/geometry/v1/calls-s5-tools.json`'a 22 adlı, 75 rastgele durum eklendi; eski durumlar satır satır aynı kaldı (yeniden kayıt öbür dosyalarda son bit farkları yazdığı için yalnız yeni işlemlerin satırları eklendi). Native ve WASM aynı cevapları verir.
- **Bağımsız referans** (`scripts/fixtures/geometry_call_reference.py`, `reference-calls.json`) üç durum aldı: TM çizgisine noktadan geçen uzaklık (13 m), deliğe yakın tıklama, iki TM çizgisinin ortak ucundaki köşe (3-4-5 ve 7-24-25 kenarlar, açı 60 basamaklı atan).

### Katalog kaydı: `cad.entities.edit` v1

`crates/shared/contracts/src/cad_edit.rs`, `catalog.rs`; ADR 0022 ve 0037'nin kalıbıyla.

| Alan | Değer |
|---|---|
| `title` | Nesneleri düzenle |
| `effect`, `hosts`, `headless`, `requires` | `document`; `web`, `desktop`; evet; `document` |
| `permissions`, `undo`, `cost` | yok; `step` (işlemin adıyla); `instant` |
| `examples` | budama (yerinde kalan parça ve yeni parça); uzatma, planlandığı sürümde |

- **Girdi** `EntitiesEdit { operation, changes, expectedRevision? }`:
  - `operation` (`EditOperation`): `offset`, `trim`, `extend`, `fillet`, `chamfer`, `break`, `join`, `explode`, `lengthen`, `vertexAdd`, `vertexRemove`, `stretch`. Adımın adını verir: Ötele, Buda, Uzat, Köşe yuvarla, Pah, Kır, Birleştir, Patlat, Uzat-kısalt, Köşe ekle, Köşe sil, Esnet.
  - `changes` (`EntityEdit`, `kind` etiketli), sırayla ve hepsinden önceki belgeye göre:
    - `update { uid, geometry }`: nesne yeni geometriyi alır (tür değişebilir), öbür her alanı kalır: yuva, kalıcı kimlik, katman, renk, öznitelikler, etiket, simge;
    - `replace { uid, geometry, keepData? }`: nesne yerinde ve kimliğiyle başka bir nesne olur; katmanı ve rengi kalır, öznitelikleri ve etiketi yalnız `keepData` ile, simgesi hiç (web'in `inherit`'i);
    - `add { from, geometry, keepData? }`: `from`'dan yeni nesne: katmanı ve rengi; öznitelikleri ve etiketi yalnız `keepData` ile;
    - `remove { uid }`: nesne silinir.
  - `geometry` (`EntityGeometry`): nesnenin yalnız geometrisi, `kind` etiketli: nokta, çizgi, çoklu çizgi, kapalı alan (delikleriyle), daire, yay, elips, eğri, yardımcı çizgi, ışın, yazı; 2. kısımdan beri ölçü ve tarama da (Esnet için).
- **Çıktı** `EntitiesEdited { changed, created, removed, revision }`; **plan** `EntitiesEditPlan { changed, created, removed, revision }`: yazılacak nesnelerin kendisi, yenilerin yuvası 0.
- **Neden geometri girdide, işlem değil:** araçların sonucu görünüme bağlıdır: hızlı budama görünen bütün kenarlarla keser (sınırları geometri deposu toplar), köşe imleç altında bulunur. Komut, önizlemenin gösterdiğini yazar. Ortak olan yazma kuralıdır: hangi nesne yerinde ve kimliğiyle kalır, parça neyi alır, kilitli katman, tek adım. Python ve AI için işlem düzeyinde tipli komutlar (`cad.entity.trim { uid, at, boundaries }` gibi) ertelenenlerdedir; bu komutun üstüne kurulabilir.
- **Neden tek komut:** on araç aynı dört değişikliği kullanır; web'in araçları da bunları ortak bir tabandan yazıyordu.
- **Seçim ve görünüm girdide açıktır (`CMD-07`):** komut seçim, katman ya da görünüm okumaz.

### Denetimler ve sıraları

İlk tutmayan cevap verir.

| Sıra | Kod | Durum | Yol |
|---|---|---|---|
| 1 | `no_changes` | failed | `changes` |
| 2 | `invalid_uid` (her değişikliğin kimliği, sırayla) | failed | `changes[i].uid`, `changes[i].from` |
| 3 | her geometri, sırayla: `too_few_points` (çoklu çizgi < 2), `too_few_corners` (kapalı alan ya da deliği < 3), `not_finite`, `invalid_radius` (daire, yay) | failed | `changes[i].geometry.pts`, `…holes[h].pts`, `changes[i].geometry`, `…r` |
| 4 | `invalid_revision`, `revision_conflict` | failed, conflict | `expectedRevision` |
| 5 | `entity_not_found` (ilk bulunmayan) | failed | `changes[i].uid`, `changes[i].from` |
| 6 | `repeated_entity`: bir nesne iki değişiklikte değişiyor (`add` aynı nesneden gelebilir) | failed | `changes[i].uid` |
| 7 | `layer_locked`: bir nesne kilitli katmanda (kendisi ya da üstündeki grup) | failed | `changes[i].uid`, `changes[i].from` |
| — | masaüstünde `slots_exhausted`: yeni parçalara yuva kalmadı; hiçbir şey yazılmaz | failed | — |

- **Kilitli katman: düzenleme bütün yazılır ya da hiç.** Budanan nesnenin kalanı ve yeni parçası, birleşen zincirin nesneleri birbirine aittir; bir kısmını yazmak veriyi bozar. `cad.entities.transform`'un “öbürlerini uyarıyla yaz” kuralından bu yüzden ayrılır. Araçlar kilitli nesneyi zaten seçtirmez (web'in `editable`'ı, masaüstünde `edge::unlocked`); komutun denetimi Python ve AI içindir, araçta ikinci savunmadır.
- **Gizli katmandaki nesne düzenlenir, uyarısız** (`cad.entities.transform` gibi).
- **Yeni iletiler** (tam metin durum dosyasında):
  - `Yapılacak değişiklik verilmedi. En az bir değişiklik verin.`
  - `N. değişikliğin geometrisinde sonlu olmayan bir değer var (NaN ya da sonsuz). Geometriyi sonlu sayılarla verin.`
  - `“…” kimlikli nesne birden çok değişiklikte değişiyor. Bir nesneye tek değişiklik verin.`
  - `“Kilitli katman” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın.`
  - Nokta sayısı, köşe sayısı ve yarıçap iletileri oluşturma komutlarınınkidir.
- **Yürütme:** tek işlem, adı işlemin: önce silinenler, sonra yerinde değişenler (web `replace`, masaüstü `update_many`), sonra yeniler (`addMany` / `add_many`). Açık işlem ya da grup varsa ona katılır. Geometrisi aynı kalan `update` adım yazmaz; çıktı onu yine `changed`'de sayar. Web'de `update` nesnenin kendi alan sırasını korur: aynı kalan değişiklik JSON'un yazdığı gibi aynıdır (`sameJson`).

### Ortak durumlar

- `fixtures/commands/v1/cad.entities.edit.json`: 26 durum. İki koşucu 213 durumun hepsini geçer (23 + 20 + 23 + 23 + 23 + 23 + 23 + 29 + 26).
- Kapsam: dört değişiklik, `keepData`, simge, tür değişimi, zincir, patlatma (silinenden `add`), delikli alan, adım adları (on bir işlem ve Esnet), aynı kalan geometri, gizli katman, her ret ve sıraları, plan ve doğrulama, çakışma.
- Beklenen nesneleri sözleşmenin kuralı kurar (`scripts/fixtures/edit_command_cases.py`: `updated`, `inherited`); bir uygulamanın çıktısından alınmadı. `--check` dosyayı yeniden kurup karşılaştırır.
- Masaüstüne özgü testler (`tests/edit.rs`): yuvası tükenmiş belgede hiçbir şey yazılmaz (sığan kısım da geri alınır), açık işleme katılma, −0 ve son bit (verildiği gibi yazılır), çekirdek şekli ile komutun geometrisi arasında gidip gelme, adım adları.

### Web araçları komuttan yazar

- `tools/editCommand.ts` (`writeEdit`, `uidOf`, `editGeometry`, `createdIds`): araç, seçtiği nesneleri kalıcı kimlikle ve çekirdeğin hesapladığı geometriyi komuta verir; ret ve uyarılar aracın iletisidir; başarı iletisi yalnız yazılınca söylenir.
- `EdgePickTool.replace(operation, e, pieces)`; Ötele `add`; Uzat, Uzat-kısalt, köşe araçlarının kenarları ve aynı türde kalan Köşe ekle/sil `update`; Birleştir `replace` (keepData) ve `remove`; Patlat `remove` ve `add`.
- Katalog on aracın komutunu yazar (`productCommand`); envanter onu okur.
- **Web'de davranış değişiklikleri:**
  - Ötele ve Uzat'ın geri alma adımı artık aracın adıdır (“Ekle” ve “Değiştir” idi).
  - Çoklu çizgiyi çizgiye çeviren köşe silme “Köşe sil” adımıdır (“Köşe ekle” diyordu).
  - **Hata düzeltmesi:** delikli kapalı alanın dış halkasına köşe eklemek ya da oradan köşe silmek delikleri siliyordu (`entityOp` eksik `holes`'u tanımsız yazar, `update` onu siler; geçici bir testle doğrulandı). Artık bütün geometri, delikleriyle yazılır. Köşe yuvarla ve Pah zaten delikleri koruyordu; şimdi açıkça verir.

### Masaüstünde on araç

`kentos-interaction`'da web'in sınıfları adım adım. İstemler, seçenekler, tuşlar, iletiler ve bellek aynıdır.

| Araç | Web sınıfı | Masaüstü | Akış |
|---|---|---|---|
| Ötele (`tool.offset`, Shift+O) | `OffsetTool` | `offset.rs` | nesne, sonra taraf; yazılan mesafe; Noktadan geç (N) ile geçilecek nokta (kenetlenir) |
| Buda (`tool.trim`, Shift+T), Uzat (`tool.extend`, Shift+E) | `BoundaryEdgeTool` | `trim.rs` | görünen bütün kenarlar ya da Sınır seç (S); Tüm kenarlar (T); Shift+tık öbür işlem |
| Köşe yuvarla (`tool.fillet`, Shift+F), Pah (`tool.chamfer`, Shift+P) | `CornerTool` | `corner/` | imleç altındaki köşe ya da sırayla iki çizgi; fareyle çekilen, yazılan ya da son boyut; Kırp (K) |
| Kır (`tool.break`, B) | `BreakTool` | `breaking.rs` | kenetlenen iki nokta; Aynı noktadan böl (Enter) |
| Birleştir (`tool.join`, J), Patlat (`tool.explode`, X) | `JoinTool`, `ExplodeTool` | `object.rs` | seçimle hemen; yoksa önce seçtirir; kilitli nesne uyarıyla atlanır; seçerken yazılan sayı Birleştir'in toleransıdır |
| Uzat-kısalt (`tool.lengthen`, Shift+U) | `LengthenTool` | `lengthen.rs` | Dinamik (D), Fark (F), Yüzde (Y), Toplam (T) |
| Köşe ekle/sil (`tool.vertex`, V) | `VertexTool` | `vertex.rs` | kenara tık ekler, köşeye tık siler |

- **Kenar seçen taban** (`edge.rs`): hedef, düzenlenebilir nesneler arasında kenarıyla seçilir (`Spatial::pick_edge`, deponun `hit_edge`'i, web'in `pickEdge`'i), üzerine gelinen vurgulanır; yazma `cad.entities.edit` ile; web'in `replace`, `refuseHoled`'i; önizleme çizgileri çekirdeğin `outline_paths`'ından (`strokeGeometry`'nin karşılığı).
- **Seçimden önce seçen taban** genişledi: aşama bir seçimle hemen iş görüp bitebilir (`Stages::begin` artık `Flow` döner), seçme istemine not ekleyebilir (`picking_hint`) ve seçerken yazılanı alabilir (`picking_input`). Taşı, Döndür, Ölçekle, Aynala değişmedi.
- **Esc bir adım geri** (`Tool::cancel`, web'in `cancel`'ı): seçilen nesneyi, sınır seçmeyi ya da istenen değeri bırakır ve araç kalır; bırakacak bir şey yoksa araç biter (`Session::cancel`, `App::cancel`).
- **İstemin notları** (`Prompt::note`, `Prompt::then`): köşeli ayraçta seçenek olmayan metin (`mesafe 1.000 m`, `Shift+tık: uzat`, `kip: dinamik`), web'in ayraç düzeniyle (grup arası “; ”, grup içi “ / ”). `Prompt::text()` web'in metnini birebir yazar; komut şeridi notları adımın yanında soluk, komut satırı adımın ardında ayraç içinde gösterir (web'in `cmdbar__note`'u ve `CommandLine.setPrompt`'u).
- **Önizleme:** çizginin tonu (`Tone`: vurgu, tehlike, kenet rengi; budamada gidecek kısım kırmızı kesikli, köşenin payı yeşil), işaretler (`Marker`: köşenin 7 piksellik halkası, silinecek köşenin ×'i, eklenecek köşenin +'sı, 2 piksel) ve etiketin tonu.
- **Bellek** (`Memory`, web'in statik alanları): öteleme mesafesi (1 m) ve Noktadan geç, Kırp (açık), son yarıçap ve son pah mesafeleri (yok), Birleştir toleransı (0,001 m), Uzat-kısalt kipi ve değerleri (dinamik; 1 m, %100, 10 m).
- **Görünümün kutusu** (`View::visible`): hızlı budama ve uzatma görünen kenarları kullanır.
- **Kenet ve orto:** noktası olan adımlar web'deki gibi kenetlenir (Ötele'nin geçilecek noktası, Kır'ın iki noktası, Uzat-kısalt'ın dinamik ucu; Kır'ın dik ve teğet keneti ilk noktadan). Web bu araçlarda orto ve kutupsal izleme uygulamaz (noktalar nesnenin üstündedir, bağlama noktası yoktur); masaüstü de uygulamaz.
- **Şerit:** araçlar web'deki sekme ve panellerinde (Giriş ve Değiştir) etkinleşir; `ported.json`'a on komut eklendi. Masaüstünde 56 / 167 komut çalışıyor.

### Etkileşim izleri

`fixtures/interaction/v1`'e dört iz ve bir çizim (`edits.kcad`) eklendi. Web ve masaüstü onları değiştirmeden, üç varyantta geçer.

| İz | Ne tutar |
|---|---|
| `edge-tools` | görünen kenarlarla budama, Shift+tık ile uzatma, tek adımda geri alma; kilitli çizgi vurgulanmaz ve düzenlenmez; Sınır seç (S), sağ tıkla onay, seçilen sınıra göre uzatma, Tüm kenarlar (T); Uzat ve Shift+tıkla budama; yazılan mesafeyle ve Noktadan geç (N) ile öteleme |
| `corner-tools` | imleç altındaki çoklu çizgi köşesi, yazılan yarıçap; iki çizginin ortak ucu, Enter ile son yarıçap; sırayla seçilen iki çizgi ve iki mesafeli (`1,2`) pah; Kırp kapalıyken yalnız pah çizgisi |
| `path-edit-tools` | kenetlenen kırılma noktası, aradaki kısmın silinmesi, Enter ile bölme; köşe ekleme, silme, kapalı alana köşe; dinamik uç ve yazılan toplam uzunluk, Toplam (T) kipinde tıklanan uç |
| `object-tools` | seçmeden başlama, kesişim penceresi, kilitli çizginin atlanması, sağ tıkla birleştirme; seçimle patlatma, parçaların seçilmesi; seçip Enter ile patlatma; temel nesnenin reddi |

- İzler araçların belleğini başladığı gibi bırakır (mesafe, Noktadan geç, Kırp, uzat-kısalt kipi ve toplamı yazılarak geri kurulur). Geri kurulamayanlar (son yarıçap, son pah mesafeleri) izin kendi yazdığı değerle kurulur; ondan önceki beklentiler onlara bağlı değildir. Web oynatıcısı sayfayı varyantlar arasında yeniden açmadığı için bu kural sınandı: üç varyant geçer.
- İz biçimi değişmedi.

### Ters deneme

Her kural bilerek bozuldu; iki platformda aynı adım aynı değerle düştü, sonra geri alındı (kayıtlar `.run/break-*-0047.log`).

- **Kilitli katman, komutta:** iki işleyicide kilit denetimi kaldırıldı. 26 durumdan aynı 2'si iki koşucuda düştü (kilitli katman, kilitli grubun katmanı).
- **Kilitli katman, araçta:** kenar araçlarının ve Birleştir/Patlat'ın kilit süzgeci kaldırıldı. `edge-tools` 8. adımda (kilitli çizgi vurgulandı: `hover` 13, beklenen yok), `object-tools` 5. adımda (kilitli çizgi zincire girdi, komut hepsini reddetti: `entities` 13, beklenen 12) iki platformda, üç varyantta düştü. İlk denemede izler bunu yakalamıyordu (komutun denetimi örtüyordu); iz ve çizim bu yüzden güçlendirildi (`525b6c5`).
- **Tek adım:** iki işleyicide işlem (transaction) kaldırıldı. Ortak durumlardan web'de 6, masaüstünde 5'i düştü (masaüstünün `update_many`'si iki güncellemeyi yine tek adımda yazıyor); `corner-tools` 28., `object-tools` 6., `path-edit-tools` 12. adımda iki platformda aynı değerlerle düştü.
- **Ortak hesap:** çekirdeğin `through_distance`'ına 0,5 m eklendi (WASM yeniden derlendi). `edge-tools` 37. adımda iki platformda düştü (kopya x = −16,5, beklenen −17); çağrı durumlarından 22'si ve bağımsız referans düştü.

## 2. kısım: Esnet, Dizi, Kutupsal dizi, Hizala

Web'de dördü eski yoldaydı (koddan doğrulandı, 26 Eylül): Esnet `doc.updateMany` ile, diziler ve Hizala `SelectionFirstTool.applyTransforms` ile (görünümün deposundan JSON'suz `transformEntities`, sonra `addMany` ya da `updateMany`) doğrudan yazıyordu; dizilerde kilitli katmandaki nesne de kilitli katmanına kopyalanıyordu (ADR 0037'den önceki kural).

### Karar: dizilere kendi komutu, Hizala dönüşüm komutunda, Esnet düzenleme komutunda

- **Dizi ve Kutupsal dizi yeni bir komutla yazar: `cad.entities.array` v1.** Seçenek, `cad.entities.transform`'a bir dönüşüm listesi (“çoklu dönüşüm”) vermekti. Seçilmedi:
  - Kullanıcının, Python'un ve AI'nın verdiği satır × sütun ve aralık ya da merkez, adet, açı ve dönmedir; 9 999 matrislik bir liste değil.
  - Sayıların sınırları (2 ile 10 000 yer, 2 ile 1000 öğe) ve aralığın, açının anlamı komutta denetlenir; bir matris listesinde denetlenemez.
  - Dönmeyen kutupsal kopyaların yeri kopyalanan nesnelerin kutusunun ortasına bağlıdır. Komut onu kendi ölçer; çağıranın hesapladığı matrise güvenmez.
  - Bütün kopyalar tek adımdır (`add_many` / `addMany`), adım aracın adıdır: “Dizi”, “Kutupsal dizi”.
- **Hizala `cad.entities.transform` v1'in yeni `align` dönüşümüdür.** Hizalama bir benzerlik dönüşümüdür (öteleme, dönme, eşit ölçek). Taşı, döndür ve ölçekle ile aynı kurallara uyar: yerinde yazılır, kilitli nesne uyarıyla kalır, kopya seçeneği vardır. Ekleme eski girdilerin hiçbirini bozmaz (ADR 0013: uyumlu değişiklik); yeni sürüm açılmadı.
- **Esnet `cad.entities.edit` v1 ile yazar** (`stretch` işlemi, `update` değişiklikleri). Esnetme bir benzerlik değildir: pencerede kalan köşeler kayar, öbürleri kalır. Çekirdeğin `stretch_entity`'si her nesnenin yeni geometrisini verir, komut önizlemenin gösterdiğini yazar. Web'in Esnet'i ölçü ve taramayı da esnetiyordu; bu yüzden `EntityGeometry`'ye ölçü (`dimension`) ve tarama (`hatch`) eklendi (uyumlu ekleme). İşlem düzeyinde tipli bir esnetme komutu ertelenenlerdedir.

### Ortak çekirdek

- `geom/affine.rs` `align(pts, scale)`: birinci kaynak birinci hedefe; ikinci çiftle kaynak doğrultusu hedef doğrultusuna döner, `scale` ile boy eşitlenir. İkinci çiftin noktası birincinin bir nanometre yakınındaysa yok. Önceden `tools/editing.rs` `align_transform`'daydı; o artık bunu çağırır. `similarity` iki tür daha kurar: `align` (4 ya da 8 sayı) ve `alignScale` (8 sayı).
- `tools/editing.rs`:
  - `grid_array_transforms(rows, cols, dx, dy)`: satır satır, her satırda sütun sütun; asılların yeri dışarıda; 1'den az satır ya da sütunda, 10 000'den çok yerde boş.
  - `shapes_middle(shapes, font)`: şekillerin kutularının birleşiminin ortası, deponun `extent`'i ve web'in `centreOf`'u gibi (`js_min`/`js_max`, orta nokta); yazı çizimin yazı tipiyle ölçülür.
  - `array_transforms(kind, p, shapes, font)`: iki komutun iki platformda çağırdığı: `grid` (rows, cols, dx, dy) ya da `polar` (cx, cy, count, fill, rotate 1/0); sayılar tam değilse ya da aralıklarının dışındaysa yok.
- `store/pack.rs` `array_packed_objects` ve WASM `arrayObjects`: web'in işleyicisi nesneleri paketler, çekirdek kopyaları yerleştirip taşır, yalnız yeni geometri sayı olarak döner (JSON yok, −0 korunur); `transformObjects`'in karşılığı.
- **Çağrı durumları:** `calls-s5-tools.json`'a `gridArrayTransforms`, `shapesMiddle`, `arrayTransforms` için 18 adlı, 75 rastgele durum; eski durumlar satır satır aynı kaldı. Native ve WASM aynı cevapları verir.
- **Bağımsız referans** (`reference-calls.json`) dört durum aldı: ızgaranın çarpımları (tam), TM çizgi ve dairenin kutusunun ortası, TM merkez çevresinde 180° içinde 5 öğe dönerek ve dönmeden (60 basamaklı sin ve cos).
- Web'in dizi önizlemesi (`ArrayTool.offsets`: her yer tek çarpım) TypeScript'te kaldı: binlerce öteleme her karede JSON'la geçmesin diye; bitleri çekirdeğinkiyle aynıdır. Yazılan kopyalar komuttan, yani çekirdekten gelir. Masaüstünün önizlemesi çekirdeğin `grid_array_transforms`'unu çağırır.

### Katalog kaydı: `cad.entities.array` v1

`crates/shared/contracts/src/cad_array.rs`, `catalog.rs`.

| Alan | Değer |
|---|---|
| `title` | Nesneleri diziye kopyala |
| `effect`, `hosts`, `headless`, `requires` | `document`; `web`, `desktop`; evet; `document` |
| `permissions`, `undo`, `cost` | yok; `step` (aracın adıyla); `instant` |
| `examples` | bir parselin 2 × 3 dizisi; bir direğin meydan çevresinde 8 kez, planlandığı sürümde |

- **Girdi** `EntitiesArray { uids, layout, expectedRevision? }`; `layout` (`ArrayLayout`, `kind` etiketli):
  - `grid { rows, cols, dx, dy }`: `rows` × `cols` yer (2 ile 10 000 arası), `j` sütun ve `i` satır ötedeki kopya `j·dx` doğuya, `i·dy` kuzeye;
  - `polar { center, count, fill, rotate }`: merkez çevresinde `count` öğe (2 ile 1000 arası, asıllar dahil), `fill` derecelik açıya (saat yönünün tersine; eksi saat yönünde; 360 tam tur). Tam tur eşit bölünür, kısmi dolguda son kopya bitiş açısındadır. `rotate` ile kopya merkez etrafında döner; yoksa yönünü korur, kopyalanan nesnelerin kutusunun ortası döner.
- **Çıktı** `EntitiesArrayed { created, locked, revision }`; **plan** `EntitiesArrayPlan { sources, entities, locked, revision }`: kopyaların kendisi, yuvaları 0.
- Kopyalar yer yer, her yerde nesneler girdinin sırasıyla yazılır. Kopya aslının bütün alanlarını (katman, renk, öznitelikler, etiket, simge) ve yeni bir kalıcı kimlik alır.
- **Kilitli katmandaki nesnenin kopyası yapılmaz** (ADR 0037): öbürleri uyarıyla kopyalanır, `locked`'da adlanır; hepsi kilitliyse hiçbir şey yazılmaz. Dönmeyen kutupsal kopyaların ortası yalnız kopyalanan nesnelerden ölçülür. Gizli katmandaki nesne uyarısız kopyalanır. Tekrarlanan kimlik bir kez sayılır.

| Sıra | Kod | Yol |
|---|---|---|
| 1 | `no_entities` | `uids` |
| 2 | `invalid_uid` (her kimlik, sırayla) | `uids[i]` |
| 3 | `not_finite`: ızgarada `dx`, `dy`; kutupsalda merkez, sonra `fill` | `layout.dx`, `layout.dy`, `layout.center.x` / `.y`, `layout.fill` |
| 4 | `invalid_count`: satır ya da sütun 0; yer 2 ile 10 000 dışında; adet 2 ile 1000 dışında | `layout.rows`, `layout.cols`, `layout`, `layout.count` |
| 5 | `invalid_spacing`: birden çok sütunda `dx`, birden çok satırda `dy` bir nanometreden küçük | `layout.dx`, `layout.dy` |
| 6 | `invalid_fill`: `fill` bir nano dereceden küçük ya da 360'tan büyük (mutlak) | `layout.fill` |
| 7 | `invalid_revision`, `revision_conflict` (conflict) | `expectedRevision` |
| 8 | `entity_not_found` (ilk bulunmayan) | `uids[i]` |
| 9 | `layer_locked`: hepsi kilitli | `uids` |
| 10 | `not_finite`: bir kopya en büyük float64'ü aşıyor | `layout` |
| — | masaüstünde `slots_exhausted` | — |

- **Yeni iletiler** (tam metin durum dosyasında):
  - `Diziye alınacak nesne verilmedi. En az bir nesnenin kalıcı kimliğini verin.`
  - `Doğu (Y) yönündeki aralık sonlu bir sayı değil (NaN ya da sonsuz). Aralığı sonlu bir sayıyla verin.` (kuzey için de aynısı)
  - `Satır × sütun 2 ile 10 000 arasında olmalı; satır ve sütun en az 1'dir. Başka bir satır ve sütun sayısı verin.`
  - `Adet 2 ile 1000 arasında bir tam sayı olmalı. Başka bir adet verin.`
  - `Sütunlar arasındaki aralık (dY) sıfır; kopyalar üst üste düşer. Sıfırdan farklı bir aralık verin.` (satırlar için dX)
  - `Doldurma açısı 0 ile ±360 derece arasında olmalı; 0 olamaz. Başka bir açı verin.`
  - `N nesne kilitli katmanda olduğu için atlandı. Kopyalamak için katmanın kilidini Katmanlar panelinden açın.`
- **Sayılar:** satır, sütun ve adet sözleşmede tam sayıdır (JSON şemasında aralıklarıyla). Web işleyicisi TypeScript'ten gelen kesirli ya da NaN sayıyı da `invalid_count` ile reddeder; masaüstünde tip onları taşıyamaz.

### `cad.entities.transform` v1: `align`

- `Transform::Align { source, target, source2?, target2?, scale? }`: tek çift yalnız taşır; iki çiftle nesneler `source` etrafında döner (ve `scale` ile ölçeklenir), sonra `source` `target`'a gider. İkinci çift olmadan `scale` bir şey değiştirmez. Adım “Hizala”.
- Denetimler, dönüşümün sırasında: noktalar sonlu (`transform.source`, `target`, `source2`, `target2`, x sonra y), sonra `invalid_align`: ikinci çift yarımsa eksik nokta (`transform.target2` ya da `transform.source2`), ikinci çiftin noktası birincinin bir nanometre yakınındaysa o nokta. Ölçü çekirdeğinkidir (`js_hypot`, web'de aynı bitlerle `Math.hypot`).
- **Yeni iletiler:** `Hizalamanın ikinci hedef noktası verilmedi; ikinci çift iki noktayla verilir. İkinci hedef noktasını verin ya da ikinci kaynak noktasını çıkarın.` (kaynak için de); `Kaynak noktaları çakışıyor; kaynak doğrultusunun yönü yok. Birbirinden ayrı iki kaynak noktası verin.` (hedef için de).

### `cad.entities.edit` v1: ölçü ve tarama

- `EntityGeometry::Dimension { a, b, offset, height, text?, style?, angle?, c? }`, `EntityGeometry::Hatch { ring, holes?, pattern }`. `update` onları öbür geometriler gibi yazar.
- Denetim: taramanın halkası ve her deliği en az 3 köşe (`too_few_corners`: `Taramanın en az 3 köşesi olmalı; N köşe verildi. Eksik köşeleri ekleyin.`, delik için kapalı alanınki), her sayı sonlu (desenin açısı ve aralığı dahil).

### Ortak durumlar

- `fixtures/commands/v1/cad.entities.array.json`: 25 durum (yeni üretici `scripts/fixtures/array_command_cases.py`, `--check`); `cad.entities.transform.json`'a 9 hizalama durumu (38); `cad.entities.edit.json`'a ölçü ve tarama için 2 durum (28). İki koşucu 249 durumun hepsini geçer.
- Beklenen kopyalar ve hizalamalar tanımdan, aynı işlem sırasıyla hesaplanır: dönüşüm ve dizi üreticilerinin ortak dosyası `scripts/fixtures/affine_reference.py` (dönüşüm üreticisi ona taşındı, çıktısı bit bit aynı kaldı). Bir uygulamanın çıktısından alınmadı.
- **Açılar:** kutupsal durumlar çeyrek ve sekizde bir turlarla kuruldu. -240°'de çekirdeğin sinüsü (fdlibm'in, V8'in) doğru yuvarlanmış değerden bir birim yukarıdadır; Python'un libm'i doğru yuvarlar. Bu, TM koordinatını yaklaşık 1e-10 m kaydırır; referans o zaman kütüphaneleri sınardı, diziyi değil. Kusur değil: çekirdek V8 ile bit bit aynı kalmak için fdlibm'i izler.
- Masaüstüne özgü: çekirdek şekli ile komutun geometrisi arasında gidip gelme ölçü ve tarama için de (`tests/edit.rs`, `geometry.rs`).

### Web araçları komuttan yazar

- Esnet: her nesnenin esnemiş geometrisi `writeEdit('stretch', …)` ile `update` olarak. Yazılacak değişiklik kalmazsa (esnerken bozulan yaylar) komut çağrılmaz, “0 nesne esnetildi” denir (web'in eski davranışı).
- Dizi, Kutupsal dizi: `SelectionFirstTool.arraySelection(layout)`; Hizala: `transformSelection({ kind: 'align', … })`. Hizala kendi ön denetimini (çekirdeğin `alignTransform`'u) ve iletisini (`Kaynak ya da hedef noktaları çakışıyor; hizalama yapılamaz.`) korur: çakışan ikinci çift yeniden istenir.
- Kullanan kalmadığı için `applyTransforms` ve görünümün `transformEntities`'i kaldırıldı. `transformedFrom` kaç tur okunacağını cevaptan da bulabilir (dizi için).
- Katalog dört aracın komutunu yazar (`productCommand`); envanter onu okur.
- **Web'de davranış değişiklikleri:**
  - Dizilerde kilitli katmandaki nesnenin kopyası yapılmaz; uyarı komutun iletisidir. Hizala'da hepsi kilitliyse komutun reddi söylenir (“0 nesne hizalandı.” diyordu).
  - Birden çok yeri olan bir yönün aralığı sıfırsa dizi reddedilir (3 × 1 dizide satır aralığı 0 gibi); eskiden kopyalar asılların üstüne yazılıyordu. Araç yeni aralık bekler; son değerler yalnız yazılınca hatırlanır. İki noktanın çakışması (iki aralık da sıfır) eskisi gibi sessizce yok sayılır.
  - Dönmeyen kutupsal kopyaların ortası kopyalanan (kilitli olmayan) nesnelerden ölçülür; önizleme de öyle.

### Masaüstünde dört araç

| Araç | Web sınıfı | Masaüstü | Akış |
|---|---|---|---|
| Esnet (`tool.stretch`, E) | `StretchTool` | `stretch.rs` | pencere (iki tık ya da 4 pikselden uzun sürükleme; köşeleri kenetlenmez), temel nokta, hedef (kenetlenir; orto ve kutupsal izleme temelden; `@dY,dX`) |
| Dizi (`tool.array`, Shift+A) | `ArrayTool` | `array.rs` | nesneler; `2,3` gibi satır ve sütun (virgül, noktalı virgül ya da boşluk; Enter son değer); `dY,dX` ya da iki nokta; yazılan nokta okunmaz |
| Kutupsal dizi (`tool.arrayPolar`) | `PolarArrayTool` | `polar.rs` | nesneler, merkez; Adet (N), Açı (A), Nesneleri döndür (D); tek başına sayı adettir; sağ tık uygular |
| Hizala (`tool.align`) | `AlignTool` | `align.rs` | nesneler; birinci kaynak ve hedef (sağ tık yalnız taşır); ikinci kaynak ve hedef; Ölçekle (Ö, O da olur) |

- **Seçimden önce seçen taban** yine genişledi: bir önizlemede çok dönüşüm (`Stages::previews`: dizinin bütün kopyaları, imleç olmadan da), aşamanın onayı (`Stages::confirm`: diziler bir sonraki aşamaya geçer ya da yazar) ve yazılan noktanın okunmaması (`Stages::typed_points`: Dizi ve merkezden sonra Kutupsal dizi). Taşı, Döndür, Ölçekle, Aynala, Birleştir, Patlat değişmedi.
- Esnet kendi aracıdır (web'de de `SelectionFirstTool` değil). Çizilen pencere, hangi yöne çekilirse çekilsin kesişim kutusunun görünümündedir (`SelectBox::touching`: kenet rengi, kesikli, %10 dolgu, web'in penceresi gibi); çizildikten sonra dünyada kenet renginde kesikli dikdörtgen, nesneler imlecin farkıyla esnemiş hâlleriyle (deponun `stretch_outlines`'ı, en çok 400), temelden imlece çizgi ve uzunluk.
- **Bellek** (`Memory`): dizinin son satır, sütun ve aralığı (2, 3, 10, 10), kutupsal dizinin adedi, açısı ve dönmesi (6, 360°, evet), Hizala'nın Ölçekle'si (hayır).
- Çizimin yazı tipi (`drawing_font`) `kentos-native-application`'a taşındı: geometri deposu, Patlat ve dizi komutu aynı eşlemeyi kullanır.
- **Şerit:** Dizi ▾ ailesi zaten katalogdan kuruluyor; iki aracı taşınınca açılır ve menüsü iki aracı gösterir (`.run/shots/0047b-ribbon-dizi-menu.png`); şeritte değişiklik gerekmedi. Masaüstünde 70 / 167 komut çalışıyor.

### Komut şeridi: uzun adım ipuçlarını itmez

Şeritte adım, seçenekler ve fare ipuçları (“Sağ tık onayla / Esc çık”) tek sıradaydı; uzun bir adım (Dizi'nin aralık adımı gibi) ipuçlarını 0 piksele sıkıştırıp şeridin dışına itiyordu. Artık adım ve seçenekler, ipuçlarının, ayracın ve boşluğun bıraktığı genişliği alır (tipografinin genişlik tahminleri metinden hiç kısa kalmaz): uzun adım satır atlar, seçenekler alt satıra geçer, ipuçları şeridin sonunda bütün kalır. Test şeridi 1100 piksellik pencerede Dizi'nin aralık adımıyla yerleştirir (`command_bar.rs`); sınır kaldırılınca ipuçlarının 0 piksele sıkıştığını bulur. Görüntü: `.run/shots/0047b-strip-1100*.png`.

### Etkileşim izleri

| İz | Ne tutar |
|---|---|
| `stretch-align` | tıklanan pencere, yazılan fark, tek adımda geri alma; kilitli çizgiye değen sürüklenen pencerenin yeniden istenmesi; kapalı alanın bir kenarı; seçim varken yalnız seçili nesne; iki çiftle dönen hizalama, Ölçekle (O ile), sağ tıkla yalnız taşıma, çakışan ikinci hedef |
| `arrays` | Enter ile son sayılar, iki noktayla aralık, aynı noktanın yok sayılması; yazılan `3,1` ve reddedilen sıfır satır aralığı, sonra yazılan aralık; kilitli çizginin kopyalanmaması; yazılan merkez, Adet (N) ile reddedilen 1 ve kabul edilen 4, tam tur; Açı (A) 180, Nesneleri döndür (D) kapalı yarım tur; her dizi tek adımda geri alınır |

- İki iz de araçların belleğini başladığı gibi bırakır: son dizi `2,3` ve `10,10` ile yazılır, kutupsal dizinin adedi, açısı ve dönmesi, Hizala'nın Ölçekle'si geri kurulur. Web ve masaüstü 22 izi üç varyantta geçer. İz biçimi değişmedi.

### Ters deneme (2. kısım)

Her kural bilerek bozuldu, düştüğü görüldü, geri alındı (`.run/breaks-0047b.log`).

- **Komut şeridi:** adımın genişlik sınırı kaldırıldı: test “Sağ tık 0 piksele sıkıştı” diye düştü.
- **Kilit, araçta:** masaüstü ve web Esnet'inde kilit süzgeci kaldırıldı: `stretch-align` iki platformda aynı adımlarda düştü (kilitli çizgiye değen sürüklenen pencere yeniden istenmedi; kapalı alanı esnetecek adımda komut kilitli çizgiyi de gördüğü için hepsini reddetti). Adım numaraları önizleme hareketleri eklenmeden önceki izindir.
- **Kilit, komutta:** dizi komutunda (masaüstü ve web) kilit denetimi kaldırıldı: iki koşucuda aynı 4 durum düştü.
- **Tek adım:** masaüstünde her kopya kendi adımıyla yazıldı: 4 durum geri almada (“Ekle”, beklenen “Dizi”/“Kutupsal dizi”) düştü.
- **Ortak hesap:** `grid_array_transforms`'ta satır ve sütun yer değiştirdi: çağrı durumları, bağımsız referans ve 9 ortak durum düştü. `align`'da dönüşün işareti çevrildi: 4 hizalama durumu düştü.

## Sonuçlar

- **Testler (1. kısım):**
  - `kentos-native-application`: 213 ortak durum, katalog eşitliği, `tests/edit.rs` (5).
  - `kentos-interaction`: `tests/edits.rs` (11: on aracın akışı, istemleri, önizlemeleri, bellek, Esc, kilitli çizgi) ve istemin notları.
  - Geometri çekirdeği: yeni çağrı durumları, bağımsız referans.
  - Masaüstü ve web: 20 iz üç varyantta.
- **Testler (2. kısım):**
  - `kentos-native-application`: 249 ortak durum (25 dizi, 38 dönüşüm, 28 düzenleme dahil), katalog eşitliği; ölçü ve taramanın çekirdek şekli ile gidip gelmesi (`tests/edit.rs`, `geometry.rs`).
  - `kentos-interaction`: `tests/arrange.rs` (10: dört aracın istemleri, önizlemeleri, iletileri, bellek, kilitli nesne, ölçü, tarama ve yayın esnemesi) ve Dizi'nin sayı okuyucusu.
  - Geometri çekirdeği: ızgara, orta, dizi ve hizalama birim testleri; yeni çağrı durumları, bağımsız referans.
  - Masaüstü: komut şeridinin genişlik testi. Masaüstü ve web: 22 iz üç varyantta.
- **Bağımlılıklar:** yeni paket yok.
- **Dosya düzeni:** kenar tabanı `edge.rs`, her araç kendi dosyasında; köşe araçları bir klasörde: araç `corner/mod.rs` (560 satır: istemler, olaylar, yazma, önizleme), köşenin bulunması `corner/site.rs`, boyutun yaptığı `corner/plan.rs`. Komut `edit.rs` (421 satır; yarısı sıralı denetimler). 2. kısımda Esnet `stretch.rs` (349), diziler `array.rs` ve `polar.rs`, Hizala `align.rs`; dizi komutu `array.rs` (296). Seçimden önce seçen taban `modify.rs` 429 satırdır (taban ve iki yazma yardımcısı); deponun düz çizgilerini okuyan kod iki tabandan `outlines.rs`'e alındı.
- **Ölçüm yapılmadı.** Önizlemeler web'deki gibi her imleç hareketinde depodan ya da çekirdekten gelir. Masaüstü kutupsal önizlemesi ortayı her olayda deponun `extent`'inden alır (web'in `centreOf`'u gibi); 10 000 yerlik ızgarada bile en çok 401 nesnenin hayaleti çizilir.

## Ertelenenler

- **İşlem düzeyinde tipli komutlar** (`cad.entity.trim { uid, at, boundaries }`, `cad.entities.join { uids, tolerance }` gibi): Python ve AI'nın insanın verdiğini vermesi için. Sınırları görünümden toplayan hızlı kip için `boundaries`'in açık listesi gerekir; depo onu verebilir.
- **Simge (sahibin sorusu):** `replace` ve `add` simgeyi taşımaz, web'in `inherit`'i gibi. Seçenekler: olduğu gibi (bu dilimin kuralı; önerilen, çünkü patlatılan kapalı alanın dolgu simgesi çizgiye anlamsızdır); tür aynı kalınca simge kalsın; hep kalsın.
- Nesne izleme (masaüstünde yok), Böl (`tool.divide`) ve öbür yol araçları, tutamaçla düzenleme.
- **İşlem düzeyinde tipli esnetme** (`cad.entities.stretch { uids, window, dx, dy }`): Python ve AI pencere ve farkla çağırabilsin diye; bugün Esnet geometriyi `cad.entities.edit` ile yazar.
- **İlişkili dizi** (AutoCAD'in düzenlenebilir dizisi): diziler bugün bağımsız kopyalardır, web'de de öyle.
- **Kopya sayısının üst sınırı:** komut yerleri (10 000) ve adedi (1000) sınırlar, yer × nesne sayısını değil (web'in araçları gibi). Büyük bir seçimi büyük bir diziye kopyalamak milyonlarca nesne yazabilir; masaüstünde yuva sınırı (`slots_exhausted`) vardır.

## Doğrulama (26 Eylül 2026, Linux; main `199d3b6` üstünde, dal `worktree-agent-afc824c9621543135`)

- `cargo fmt --all --check` temiz.
- `pnpm rust:test`: 800 test geçti, 5 ölçüm testi atlandı (elle koşulur); clippy temiz; bağımlılık yönü temiz (20 crate, 26 crate × hedef).
- `pnpm rust:test:desktop`: 373 test geçti, 53 atlandı; clippy temiz. Masaüstü izleri: 20 iz × 3 varyant.
- `pnpm typecheck` temiz. `pnpm test`: 1426 geçti, 13 atlandı (başlangıçtaki 13).
- `pnpm e2e`: 162 denetimin hepsi geçti (budama, patlatma ve birleştirme, pah, köşe yuvarlama artık komuttan yazan araçlarla).
- `pnpm e2e:interaction`: 20 iz × 3 varyant geçti.
- `pnpm inventory:check` güncel. `python3 scripts/fixtures/edit_command_cases.py --check` eşit.
- Görüntüler (`KENTOS_SNAPSHOT_BACKEND=wgpu kentos-cad snapshot … --iz … --adim …`, koyu ve `--tema acik`): `.run/shots/0047-*-preview*.png` (önizleme) ve `0047-*-after*.png` (sonra), on araç, 40 görüntü.
- Pencerede (`make desktop`) elle klavye ve fare denemesi bu çalışmada yapılmadı.

## Doğrulama, 2. kısım (26 Eylül 2026, Linux; main `6a969a7` üstünde, dal `worktree-agent-afc824c9621543135`)

- `cargo fmt --all --check` temiz.
- `pnpm rust:test`: 854 test geçti, 5 ölçüm testi atlandı (elle koşulur); clippy temiz; bağımlılık yönü temiz (20 crate, 26 crate × hedef).
- `pnpm rust:test:desktop`: 417 test geçti, 56 atlandı; clippy temiz. Masaüstü izleri: 22 iz × 3 varyant.
- `pnpm typecheck` temiz. `pnpm test`: 1500 geçti, 14 atlandı. `pnpm build` temiz.
- `pnpm e2e`: 177 denetimin hepsi geçti (kutupsal dizi artık komuttan yazıyor).
- `pnpm e2e:interaction`: 22 iz × 3 varyant geçti.
- `pnpm inventory:check` güncel. Durum üreticileri `--check` ile eşit: dönüşüm 38, düzenleme 28, dizi 25; `geometry_call_reference.py` dosyayı değiştirmeden yazar.
- Görüntüler (koyu ve `--tema acik`): `.run/shots/0047b-*.png`: Esnet'in çizilen penceresi, önizlemesi ve sonucu; Hizala'nın önizlemesi ve sonucu; Dizi'nin, Kutupsal dizi'nin (dönerek ve dönmeden) önizlemeleri ve sonuçları; komut şeridi uzun adımla ve seçenek düğmeleriyle; 1100 piksellik pencerede şerit (`0047b-strip-1100*.png`); Değiştir sekmesinde Dizi ▾ menüsü.
- `pnpm e2e:cloud` koşulmadı: bulut koduna dokunulmadı. Pencerede (`make desktop`) elle klavye ve fare denemesi bu çalışmada yapılmadı.
