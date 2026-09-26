# ADR 0047: Masaüstünde değiştirme araçları, 2. tur: kenar, köşe ve nesne araçları; `cad.entities.edit` v1

- **Durum:** 1. kısım kabul edildi (2026-09-26): Ötele, Buda, Uzat, Köşe yuvarla, Pah, Kır, Birleştir, Patlat, Uzat-kısalt, Köşe ekle/sil. 2. kısım (Esnet, Dizi, Kutupsal dizi, Hizala) sonraki teslimdir; bölümü aşağıda. Yön ADR 0037'nin ertelenenlerinden (öteki değiştirme araçları) ve TODOS.md `UX-01`, `CMD-04`, `CMD-07`, `CAD-02`'den gelir.
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
  - `geometry` (`EntityGeometry`): nesnenin yalnız geometrisi, `kind` etiketli: nokta, çizgi, çoklu çizgi, kapalı alan (delikleriyle), daire, yay, elips, eğri, yardımcı çizgi, ışın, yazı. Ölçü ve tarama v1'de yok.
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

Sonraki teslimdir. Bu bölüm o teslimle yazılacak. Web'de dördü eski yoldadır: Esnet `updateMany`, diziler ve Hizala `applyTransforms` ile doğrudan yazar; kopyada kilit kuralı ADR 0037'den öncekidir.

## Sonuçlar

- **Testler (1. kısım):**
  - `kentos-native-application`: 213 ortak durum, katalog eşitliği, `tests/edit.rs` (5).
  - `kentos-interaction`: `tests/edits.rs` (11: on aracın akışı, istemleri, önizlemeleri, bellek, Esc, kilitli çizgi) ve istemin notları.
  - Geometri çekirdeği: yeni çağrı durumları, bağımsız referans.
  - Masaüstü ve web: 20 iz üç varyantta.
- **Bağımlılıklar:** yeni paket yok.
- **Dosya düzeni:** kenar tabanı `edge.rs`, her araç kendi dosyasında; köşe araçları bir klasörde: araç `corner/mod.rs` (560 satır: istemler, olaylar, yazma, önizleme), köşenin bulunması `corner/site.rs`, boyutun yaptığı `corner/plan.rs`. Komut `edit.rs` (421 satır; yarısı sıralı denetimler).
- **Ölçüm yapılmadı.** Önizlemeler web'deki gibi her imleç hareketinde depodan ya da çekirdekten gelir.

## Ertelenenler

- **İşlem düzeyinde tipli komutlar** (`cad.entity.trim { uid, at, boundaries }`, `cad.entities.join { uids, tolerance }` gibi): Python ve AI'nın insanın verdiğini vermesi için. Sınırları görünümden toplayan hızlı kip için `boundaries`'in açık listesi gerekir; depo onu verebilir.
- **Simge (sahibin sorusu):** `replace` ve `add` simgeyi taşımaz, web'in `inherit`'i gibi. Seçenekler: olduğu gibi (bu dilimin kuralı; önerilen, çünkü patlatılan kapalı alanın dolgu simgesi çizgiye anlamsızdır); tür aynı kalınca simge kalsın; hep kalsın.
- Nesne izleme (masaüstünde yok), Böl (`tool.divide`) ve öbür yol araçları, tutamaçla düzenleme.

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
