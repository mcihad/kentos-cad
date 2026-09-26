# ADR 0037: Masaüstünde değiştirme araçları, 1. tur: taşı, kopyala, döndür, ölçekle, aynala; `cad.entities.transform` v1

- **Durum:** kabul edildi (2026-09-26). Yön ADR 0029 ve 0032'nin ertelenenlerinden (değiştirme araçları) ve TODOS.md `UX-01`, `CMD-04`, `CMD-07`, `UI-11`'den gelir. Komutun sözleşmesi, kilitli katmandaki nesnenin kopyası, geometrinin JSON'suz yolu, seçimden önce seçen araç tabanı, izlerin yeni beklentisi bu dilimin kararıdır.
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** TODOS.md `UX-01`, `UX-07`, `UX-09`, `CMD-04`, `CMD-07`, `UI-11`; ADR 0008 (ortak çekirdek sınırı), 0013 (ürün komutu sözleşmesi), 0014 (kalıcı kimlik), 0018 (araç oturumu ve izler), 0021 (native araç oturumu), 0022 (ilk ürün komutu), 0029 (seçim, kenet, silme), 0032 (çizim araçları)

## Bağlam

- Masaüstünde değiştirme aracı yoktu. Taşı, Kopyala, Döndür, Ölçekle ve Aynala şeritte soluk duruyordu.
- Web'in bu araçları (`SelectionFirstTool` ve alt sınıfları, `tools/modifyTools.ts`) belgeye kendileri yazıyordu (koddan doğrulandı, 26 Eylül):
  - seçim yoksa önce seçtirir; sonra aracın noktaları ve seçenekleri;
  - `applyTransforms`: geometri deposu seçimin kendi kopyalarını dönüştürür, yalnız yeni geometri paketli döner (`transformPacked`); yerinde `updateMany`, kopyada `addMany`; adım aracın adıdır.
- Dönüşümün geometrisi zaten ortak çekirdekteydi (`ops/transform.rs`): her nesne türü; yay saat yönünün tersine kalır, aynalanan yazı okunur kalır, hizalı ölçünün uzaklığı işaret değiştirir. TypeScript'te hesap yoktu.
- Web'in kilit kuralı:
  - yerinde değişende kilitli katmandakiler atlanır, uyarılır (“N nesne kilitli katmanda olduğu için atlandı.”);
  - hepsi kilitliyse hiçbir şey değişmez, araç yine “0 nesne taşındı” der;
  - **kopyada kilide bakılmaz:** kilitli nesnenin kopyası kilitli katmana eklenir.

## Karar

### Katalog kaydı: `cad.entities.transform` v1

`crates/shared/contracts/src/cad_transform.rs`, `catalog.rs`; ADR 0022'nin kalıbıyla.

| Alan | Değer |
|---|---|
| `title` | Nesneleri dönüştür |
| `effect`, `hosts`, `headless`, `requires` | `document`; `web`, `desktop`; evet; `document` |
| `permissions`, `undo`, `cost` | yok; `step` (aracın adıyla); `instant` |
| `aliases` | yok: `M`, `CO`, `RO`, `SC`, `MI` arayüz komutlarınındır (`tool.*`) |
| `examples` | iki nesneyi taşıma; bir nesnenin 90° döndürülmüş kopyası, planlandığı sürümde |

- **Girdi** `EntitiesTransform { uids, transform, copy?, expectedRevision? }`. `transform`, araçların sorduğu biçimde tiplidir (`Transform`, `kind` etiketli):
  - `move { dx, dy }`: doğuya ve kuzeye kaydırma;
  - `rotate { center, angle }`: merkez etrafında, saat yönünün tersine, radyan;
  - `scale { center, factor }`: merkeze göre, sıfırdan büyük faktör;
  - `mirror { a, b }`: iki ayrı noktadan geçen eksene göre.
- **Neden matris değil:** Python, AI ve araç insanın verdiğini verir. Matrisi ortak çekirdek kurar (`similarity`, aşağıda); iki platformda aynı bittir.
- **Neden tek komut:** dört dönüşüm aynı akıştır (kimlikler, kilit, yerinde ya da kopya, tek adım). Dizi (çok dönüşüm) bu sürümde yok; ertelenenlere bakın.
- **Seçim girdide açıktır (`CMD-07`):** komut seçimi okumaz. Araç seçimi kalıcı kimlik listesine çevirir.
- **Yerinde** (`copy` yok ya da `false`): nesne yuvasını, kalıcı kimliğini ve öbür alanlarını korur; yalnız geometrisi değişir. **Kopya** (`copy: true`): yeni nesne aslının bütün alanlarını (katman, renk, öznitelikler, yazı, simge) ve yeni bir kalıcı kimlik alır; asıl yerinde kalır.
- **Adım aracın adıdır:** `move` “Taşı”, `move` kopya “Kopyala”, `rotate` “Döndür”, `scale` “Ölçekle”, `mirror` “Aynala” (kopya da). Web'in araçları bu adları yazıyordu.
- **Çıktı** `EntitiesTransformed { changed, created, locked, revision }`: yerinde değişenlerin kimlikleri (girdinin sırasıyla, tekrar bir kez), kopyaların yeni kimlikleri (asıllarının sırasıyla), kilitli kalanlar, yazmadan sonraki sürüm.
- **Plan** `EntitiesTransformPlan { sources, entities, locked, revision }`: yazılacak nesnelerin kendisi. Yerinde her nesne kendi yuvasıyla; kopya `id` 0 ile (yuva yazılınca verilir). Önizleme budur.

### Denetimler ve sıraları

İlk tutmayan cevap verir.

| Sıra | Kod | Durum | Yol |
|---|---|---|---|
| 1 | `no_entities` | failed | `uids` |
| 2 | `invalid_uid` (ilk bozuk kimlik) | failed | `uids[i]` |
| 3 | `not_finite`: dönüşümün sayıları alanlarının sırasıyla | failed | `transform.dx`, `transform.center.y`, `transform.angle`, `transform.factor`, `transform.a.x` … |
| 4 | `invalid_factor` (ölçek sıfırdan büyük değil), `invalid_axis` (eksenin yönü yok) | failed | `transform.factor`, `transform.b` |
| 5 | `invalid_revision`, `revision_conflict` | failed, conflict | `expectedRevision` |
| 6 | `entity_not_found` (ilk bulunmayan) | failed | `uids[i]` |
| 7 | `layer_locked`: hepsi kilitli | failed | `uids` |
| — | `layer_locked`: bir kısmı kilitli | uyarı | `uids` |
| 8 | `not_finite`: dönüşüm sonucu sonlu değil (sayı taşması) | failed | `transform` |

- Kimlik denetimleri silme komutuyla ortaktır (`checks.rs`, `checks.ts`; `uids`, `objects`, `checkUids`, `findObjects`); yalnız “verilmedi” cümlesi komutundur.
- **Eksen:** `dx² + dy²` sıfırsa reddedilir: çekirdeğin `mirror`'ının kendi ölçüsü, epsilon yok. Araçlar 1e-9 m'den yakın iki noktayı zaten almaz (web'in kuralı, iki platformda).
- **Taşma:** kaynak sonluyken sonucun bir sayısı sonlu değilse (1e308 kat ölçek gibi) hiçbir şey yazılmaz (CLAUDE.md §23.1). Kaynağında zaten sonlu olmayan sayı olan nesneye bu denetim uygulanmaz; web de onu dönüştürüyordu.
- **Yeni iletiler** (tam metin durum dosyasında):
  - `Dönüştürülecek nesne verilmedi. En az bir nesnenin kalıcı kimliğini verin.`
  - `Doğu (Y)` / `Kuzey (X) yönündeki kaydırma sonlu bir sayı değil (NaN ya da sonsuz). Kaydırmayı sonlu bir sayıyla verin.`
  - `Dönme açısı …`, `Ölçek faktörü …` sonlu değil iletileri; eksen noktaları için `Eksenin ilk noktasının doğu (Y) değeri …`.
  - `Ölçek faktörü sıfırdan büyük olmalı. Pozitif bir faktör verin.`
  - `Simetri ekseninin iki noktası aynı; eksenin yönü yok. Birbirinden ayrı iki nokta verin.`
  - `N nesne kilitli katmanda olduğu için atlandı. Değiştirmek için katmanın kilidini Katmanlar panelinden açın.`
  - `Dönüşüm sonucunda sonlu olmayan bir değer çıktı (sayı taşması). Daha küçük bir değer verin.`
- **Gizli katmandaki nesne dönüştürülür, uyarısız,** web'deki gibi (seçim gizli nesneyi zaten seçmez).
- **Yürütme** belgenin kendi `updateMany` / `update_many`'si ya da `addMany` / `add_many`'sidir: tek adım; açık işlem ya da grup varsa ona katılır. Değişmeyen nesne (sıfır kaydırma) adım yazmaz; çıktı yine onu `changed`'de sayar, web'in aracı da sayıyordu.

### Kilitli katman: kopya da yapılmaz

- **Kural:** kilitli katmandaki nesne ne değişir ne kopyalanır. Öbürleri uyarıyla yazılır; hepsi kilitliyse `layer_locked` ile hiçbir şey yazılmaz.
- **Web'de iki davranış değişikliği:**
  - Kopya (Kopyala, Kopya (K), kaynağı koruyan Aynala) artık kilitli nesneyi atlar. Eskiden kopya kilitli katmana ekleniyordu. Nedeni: kilitli katman düzenlenmez (CLAUDE.md §7); oluşturma komutları kilitli katmana yazmaz; kopya da kilitli katmana yeni nesne yazmaktır. Sahibin sorusu aşağıda.
  - Hepsi kilitliyken araç yalnız komutun iletisini gösterir; “0 nesne taşındı” demez. Taşı, Döndür, Ölçekle, Aynala yine biter; Kopyala sürer.
- İletinin ilk cümlesi web'in araçlarınındır; ikinci cümle çözümü söyler (CLAUDE.md §8, ADR 0029'daki gibi).
- Dizi, kutupsal dizi ve hizala araçları eski yoldadır (`applyTransforms`); onlarda kopya kuralı eskisi gibidir (ertelenenler).

### Geometri: ortak çekirdek, JSON'suz

Dönüşümün hesabı zaten çekirdekteydi; taşınacak TypeScript yoktu. İki platformu aynı bite bağlayan yeni parçalar:

- `geom/affine.rs` `similarity(kind, params)`: komutun dönüşümünden matris (`translation`, `rotation`, `scaling`, `mirror`). Masaüstü işleyicisi onu doğrudan, web WASM'dan çağırır. Test: `a_similarity_is_its_constructor` (−0 dahil bit bit).
- `store/pack.rs` `transform_packed_objects(nums, strings, affines)`: paketli nesneleri depo kurmadan dönüştürüp paketler. WASM girişi `transformObjects(nums, strings, kind, params)`; matris WASM'da kurulur, hiçbir sayısı JSON'dan geçmez. Web cephesi `model/ops/transform.ts` `transformObjects`.
- **Neden:** eski JSON çağrısı (`transformEntities`) −0'ı 0 yazar; ürün komutu viewport'un deposunu kullanamaz (katman sırası: `product` depodan önce gelir). Depo kurmadan paketli yol ikisini de çözer.
- **Testler:** her tür × dört dönüşüm, deponun cevabıyla bit bit (`transforms_packed_objects_as_the_store_does`); web'de 200 tur rastgele nesne ve dönüşüm, deponun yoluyla bit bit; −0 web'de ve masaüstünde ayrı testte korunur (aynalanan sıfır yay değeri −0 olur, taşımada −0 kalır).
- **Masaüstünde:** `kentos-native-application` artık `kentos-geometry-core`'a bağlıdır (iç bağımlılık, yeni paket değil). Sözleşme ↔ çekirdek dönüşümü `application::geometry`'dedir (`shape`, `with_shape`): komut ve geometri deposu (`kentos_interaction::spatial`) onu paylaşır, depo kendi kopyasını bıraktı. Render crate'inin kopyası kalır: render application'a bağlanamaz (ADR 0010).

### Ortak durumlar

- `fixtures/commands/v1/cad.entities.transform.json`: 29 durum. İki koşucu 187 durumun hepsini geçer (23 + 20 + 23 + 23 + 23 + 23 + 23 + 29).
- Kapsam: yerinde ve kopya, her dönüşüm, birden çok nesne ve tekrarlanan kimlik, kilitli ve kilitli grup, gizli katman, her ret, sıralar, plan ve doğrulama, taşma, her nesne türü (yay, daire, yazı, elips, ölçü, eğri, kapalı alan).
- **Beklenen geometri** dönüşümlerin tanımından, aynı işlem sırasıyla çift duyarlıkla bağımsız hesaplandı; bir uygulamanın çıktısından alınmadı. Üretici `scripts/fixtures/transform_command_cases.py`'dir; `--check` dosyayı bellekte yeniden kurup diskteki ile karşılaştırır. Döndürmede cos/sin gürültüsü de böyle yazıldı; iki koşucu bire bir geçti.
- **Yeni yer tutucu** `$uidOf:N`: N yuvasındaki nesnenin şimdiki kalıcı kimliği (az önce yazılan kopyanınki). İki koşucuda.
- −0 dosyaya yazılmaz (JavaScript'in JSON'u 0 yazar); durumlar ondan kaçınır, −0 kendi testlerindedir (`fixtures/commands/README.md`).
- Masaüstüne özgü testler (`tests/transform.rs`): −0, yuvası tükenmiş belgede kopya, açık işleme katılma, adım adları.

### Web araçları komuttan yazar

- `SelectionFirstTool.transformSelection(transform, copy)`: seçimin kalıcı kimlikleri komuta gider; ret ve uyarılar aracın iletisidir; başarı iletisi yalnız yazılınca söylenir. Taşı, Kopyala, Döndür, Ölçekle, Aynala bunu kullanır; iletiler ve akış değişmedi.
- Katalogda beş aracın komutu yazılıdır (`productCommand`); envanter onu okur.
- Önizleme (hayaletler) eskisi gibidir: depo çizer, matris önizleme için JSON'dan gelir (kalıcı değildir).

### Masaüstünde beş araç

`kentos-interaction`'da web'in sınıfları adım adım: ortak taban `modify.rs` (`Modify<S: Stages>`), araçlar `move_copy.rs`, `rotate.rs`, `scale.rs`, `mirror.rs`. İstemler, seçenekler, tuşlar ve iletiler aynı metindir.

| Araç | Web sınıfı | Akış |
|---|---|---|
| Taşı (`tool.move`, Shift+M) | `MoveTool` | temel nokta, hedef (tıklanan ya da `@dY,dX`); taşır ve biter |
| Kopyala (`tool.copy`, Shift+C) | `MoveTool` (kopya) | temel nokta, her tık ya da yazılan bir kopya; Bitir (Enter) |
| Döndür (`tool.rotate`, Shift+R) | `RotateTool` | merkez; yazılan derece ya da gösterilen doğrultu; Referans (R); Kopya (K) |
| Ölçekle (`tool.scale`, Shift+S) | `ScaleTool` | temel nokta; yazılan faktör ya da gösterilen referans ve yeni uzunluk; Referans (R); Kopya (K) |
| Aynala (`tool.mirror`, Shift+I) | `MirrorTool` | eksenin iki noktası; Kaynağı sil (S) |

- **Seçmeden başlama:** seçim yoksa araç önce seçtirir (istem “nesnelere tıklayın ya da pencereyle seçin, bitince sağ tıklayın (N seçili)”). Tıklama nesneyi seçime ekler ya da çıkarır (Shift gerekmez); 4 pikselden uzun sürükleme kutu çizer, kutununkiler eklenir (soldan sağa pencere, sağdan sola kesişim). Enter, Boşluk ya da kısa sağ tık seçim varken aşamalara geçer, yoksa araç biter. Seçim sırasında imleç altındaki nesne vurgulanır; kutu seçim aracınınki gibi çizilir (`Tool::select_box`).
- **Aşamalar:** noktalar kenetlenir (temel nokta, merkez, eksen, referans uçları); orto ve kutupsal izleme aracın bağlama noktasından; yazılan `@` noktaları ondan. Seçim aşamasında seçim ham imleçten yapılır, kenet seçimi değiştirmez.
- **Biten araç:** yazdıktan sonra biten araç (Taşı, Döndür, Ölçekle, Aynala) çağrının sonunda çıkar: `Tool::finished` (web'in `ctx.tools.exit()`'i). Seçim kalır.
- **Önizleme** web'inkidir: seçimin dönüşüm sonrası dış çizgileri kesikli (4/3 piksel), en çok 401 nesne; noktalar ve yazılar 7 piksellik kare (`Preview.marks`); bağlama noktasından imlece düz çizgi; imlecin yanında uzunluk, açı ya da faktör; kutupsal izleme ışını. Dış çizgileri geometri deposu verir (`transform_outlines`). Araç onları her olayda depoyla hesaplayıp saklar: `preview(&self)` depoyu görmez.
- Hayaletler seçimin tamamıdır, kilitli nesne dahil: web de öyle çizer; kilitli nesne yazılmaz.
- Ctrl+Z araç çalışırken çizimi geri alır (web'de de aracın kendi adımı yok): Kopyala'da en son kopya gider, araç sürer.

### Etkileşim izleri

`fixtures/interaction/v1`'e üç iz eklendi. Web ve masaüstü onları değiştirmeden, üç varyantta geçer.

| İz | Ne tutar |
|---|---|
| `move-copy` | seçim yokken araç önce seçer (tıklama, pencere, sağ tıkla devam); temel nokta uca kenetlenir; yazılan `@dY,dX` ile taşıma, tek adımda geri alma; her tık bir kopya, kilitli katmandaki nesne kopyalanmaz; Bitir (Enter) |
| `rotate-scale` | kenetlenen merkez, Kopya (K) ile yazılan 90°; referans uzunluğu (R, kenetlenen iki uç) ve yazılan yeni uzunluk; gösterilen referans uzunluktan sonra yazılan faktör; tek adımda geri alma |
| `mirror` | kenetlenen eksenle simetrik kopya; Kaynağı sil (S) ve yazılan eksen noktalarıyla yerinde çevirme; tek adımda geri alma |

- **Biçim eklemesi:** `objects`: kimliğiyle verilen nesneler, her biri `newest` gibi okunur (`id`, `kind`, `points`, …). Yerinde taşınan, döndürülen ya da aynalanan nesne böyle denetlenir. İki oynatıcı okur (`interaction.mjs`; masaüstü `traces/format.rs`, `player.rs`, `compare.rs`).
- Önizlemelerin görüntüye girmesi için tıklamalardan önce imleç hareketleri var; başka bir şey değiştirmezler.

### Taşınan komutlar

`tool.move`, `tool.copy`, `tool.rotate`, `tool.scale`, `tool.mirror` (`apps/desktop/ported.json`). Masaüstünde 41 / 164 komut çalışıyor. Envanter yeniden üretildi: her araç komutunu adlandırır, yeni izler testleridir.

### Ortak olan, ortak olmayan

| | Web | Masaüstü | Ortak |
|---|---|---|---|
| Komut | `product/entitiesTransform.ts` | `native/application/src/transform.rs` | sözleşme tipleri, katalog; `fixtures/commands/v1` |
| Dönüşüm | WASM `transformObjects` (paketli) | native `transform_shape` | `similarity`, `ops/transform.rs`, `store/pack.rs` |
| Araç akışı, istem | `modifyTools.ts` (`SelectionFirstTool`) | `kentos-interaction` (`modify.rs` ve dört araç) | `fixtures/interaction/v1` |
| Hayaletler | depo (`ghosts`), 2D canvas | depo (`transform_outlines`), Iced canvas | biçim (kesikli 4/3, 7 px kare) |

## Ters deneme

Her araç ailesinde bir kural bilerek bozuldu; iki platformda da aynı adım aynı değerle düştü, üç varyantta; sonra geri alındı (kayıtlar `.run/break-web-0037.log`, `.run/break-desk-0037.log`, `.run/break-native-lock.log`, `.run/break-web-lock.log`, `.run/break-desk-lock.log`).

- **Taşı / Kopyala:** kopya bayrağı düşürüldü (Kopyala yerinde taşıyor). `move-copy` 15. adımda düştü (`entities: 7`, beklenen 9; en yeni nesne çizgi, beklenen kapalı alan); 17–19. adımlar da.
- **Döndür / Ölçekle:** referans uzunluğu yok sayıldı (yazılan uzunluk faktör sanıldı). `rotate-scale` yeni uzunluğu yazan Enter adımında (o sırada 17. adım) düştü: çizgi `[-24, -12]`–`[104, -12]`, beklenen `[-16, -12]`'ye.
- **Aynala:** Kaynağı sil yok sayıldı (hep kopya). `mirror` 13. adımda düştü (`entities: 9`, beklenen 8; 2 numaralı çizgi yerinde kalmış).
- **Komutun kilit kuralı:** iki işleyicide kilit denetimi kaldırıldı. 29 durumdan aynı 4'ü iki koşucuda düştü (kilitli nesne kalır, kilitli grubun nesnesinin kopyası yapılmaz, hepsi kilitliyse ret, doğrulamanın uyarısı). `move-copy` masaüstünde 15. adımda düştü (`entities: 10`, beklenen 9).

## Sonuçlar

- **Testler:**
  - `kentos-native-application`: 187 ortak durum, katalog eşitliği, `tests/transform.rs` (4).
  - `kentos-interaction`: 79 test; değiştirme araçlarının 10'u (`tests/modify.rs`: seçmeden başlama, kutu, taşıma ve hayaletler, kopyalar, yazılan açı ve Kopya, referans doğrultusu, faktör ve referans uzunluğu, iki aynalama, kilitli katman) dahil.
  - Geometri çekirdeği: paketli dönüşümün depoyla eşitliği, `similarity`.
  - Masaüstü: 16 iz üç varyantta.
  - Web: 187 ortak durum; paketli dönüşümün depoyla bit bit eşitliği ve −0; `pnpm e2e`'nin taşıma, kopya, aynalama ve referanslı döndürme denetimleri artık komuttan yazan araçlarla geçer.
- **Bağımlılıklar:** yeni paket yok. `Cargo.lock` yalnız iç bir bağımlılık kazandı (`kentos-native-application` → `kentos-geometry-core`).
- **Dosya düzeni:** ortak taban `modify.rs` (372 satır), her araç kendi dosyasında. `transform.rs` (388) tek komuttur; uzunluğunun yarısı `finite_shape` ve sıralı denetimlerdir.
- **Ölçüm yapılmadı.** Hayaletler web'deki gibi her olayda depodan gelir, en çok 401 nesne.

## Ertelenenler

- **Kilitli nesnenin kopyası (sahibin sorusu).** Seçenekler:
  - kilitli nesne kopyalanmaz, uyarı verilir; hepsi kilitliyse ret (önerilen; bu dilimin kuralı);
  - web'in eski kuralı: kopya kilitli katmana eklenir;
  - kopya etkin katmana eklenir (kilitli değilse).
- **Ötele (`tool.offset`) taşınmadı.** Araç başka bir ailedendir (kenar seçen `EdgePickTool`: üzerine gelinen kenar, taraf ya da “Noktadan geç”, mesafe belleği). Sonucu nesnenin türüne göre değişir (elips ve yardımcı çizgi için oluşturma komutu yok); kendi komutu (`cad.entity.offset` gibi), durum dosyası ve izi gerekir. Ucuz değildi; ayrı dilim.
- Dizi, kutupsal dizi, hizala: web'de eski yoldadır (`applyTransforms`), masaüstünde yoktur. Çok dönüşümlü bir sürüm (`cad.entities.transform` v2 ya da `cad.entities.array`) gerekir.
- Araçların nokta hesaplayıcısı (`acceptPoint`, iç içe araç) masaüstünde yok (`UX-07`).
- Tutamaçla taşıma ve germe (`stretch`), kopyala-yapıştır (pano).
- Render crate'inin sözleşme → çekirdek dönüşüm kopyası: ortak bir yere taşınması için render'ın bağımlılık kuralı (ADR 0010) değişmeli.

## Doğrulama (26 Eylül 2026, Linux; main `fcb51b1` üstünde, dal `worktree-agent-afc824c9621543135`)

- `cargo fmt --all --check` temiz.
- `pnpm rust:test`: 705 test geçti, 3 ölçüm testi atlandı (elle koşulur); clippy temiz; bağımlılık yönü temiz (19 crate, 25 crate × hedef).
  - Veritabanı testleri yerel sunucuda, geçici veritabanlarında çalıştı: `KENTOS_TEST_DB=required cargo test -p kentos-application -p kentos-postgres -p kentos-api` 111 test geçti.
- `pnpm rust:test:desktop`: masaüstü 73 (4 ölçüm testi atlandı), render 29, KentOS UI 171, vitrin 55 test geçti; clippy temiz.
- `cargo test -p kentos-desktop traces`: 16 iz × 3 varyant ve komut satırı modeli geçti.
- `cargo test -p kentos-native-application`: 187 ortak durum ve masaüstüne özgü testler geçti. `cargo test -p kentos-interaction`: 79 test geçti.
- `pnpm typecheck` temiz. `pnpm test`: 1335 geçti, 13 atlandı (başlangıçtaki 13).
- `pnpm inventory:check` güncel.
- `pnpm e2e:interaction`: 16 iz × 3 varyant geçti. `pnpm e2e`: 161 denetimin hepsi geçti.
- Görüntüler (`KENTOS_SNAPSHOT_BACKEND=wgpu kentos-cad snapshot --iz … --adim …`): seçim kutusu (`--yarida`); taşımanın hayaletleri; öncesi ve sonrası; kopyaların hayaletleri (kilitli çizgi dahil); döndürülmüş kopyanın hayaleti ve açı; referanslı ölçeğin hayaleti ve faktör; aynalamanın ekseni ve hayaleti.
- Pencerede (`make desktop`) elle klavye ve fare denemesi bu çalışmada yapılmadı.
