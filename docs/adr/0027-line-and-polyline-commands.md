# ADR 0027: Çizgi ve çoklu çizgi ürün komutları, masaüstünde iki araç, komut satırını izleyen iz oynatıcısı

- **Durum:** kabul edildi (2026-09-26). Yön ADR 0022'nin “öbür araçların ürün komutları” ve ADR 0021'in ertelenenlerinden gelir: çizgi ve çoklu çizgi, komut adı yazan iz. Girdi ve çıktı tipleri, zincirin komut başına bölünmesi, yay değerlerinin sayısı, denetimlerin sırası, kodlar, oynatıcının komut satırı modeli ve Esc düzeltmesi bu dilimin kararıdır.
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** TODOS.md `CMD-04..07`, `UX-01`, `UX-03`, `UX-06`, `UX-10`, `UI-11`; ADR 0013 (ürün komutu sözleşmesi), 0018 (araç oturumu ve izler), 0020 (native belge), 0021 (native araç oturumu ve oynatıcı), 0022 (ilk ürün komutu)

## Bağlam

- ADR 0022'den sonra yalnız kapalı alan aracı ürün komutundan yazıyordu. Çizgi aracı (`LineTool`) ve açık `PathTool` (çoklu çizgi) belgeye kendileri yazıyordu: `PointInputTool.create` → `writableLayer` → `CadDocument.add` (koddan doğrulandı, 26 Eylül).
- Masaüstünde yalnız kapalı alan aracı vardı (ADR 0021). `tool.line` ve `tool.polyline` şeritte soluk duruyor, “web'de var” diyordu.
- Native iz oynatıcısı, uygulamanın görevlerindeki bir bileşen işlemini (odak) görünce izi durduruyordu. Komut satırının öneri listesini de bilmiyordu. Bu yüzden çizim alanından komut adı yazmak (ADR 0018, 6. adım) izlere girmemişti (ADR 0021, ertelenenler).
- Masaüstü, metin alanındayken geçen akorların listesini web'in `allowInInput` bağlarından elle kopyalıyordu (`keys::GLOBAL`).
- `apps/desktop/src/traces.rs` 1177 satırdı (CLAUDE.md §8).

## Karar

### Katalog kayıtları

İki komut, ADR 0022'nin kalıbıyla (`crates/shared/contracts/src/cad.rs`, `catalog.rs`):

| Alan | `cad.line.create` v1 | `cad.polyline.create` v1 |
|---|---|---|
| `title` | Çizgi oluştur | Çoklu çizgi oluştur |
| `effect`, `hosts`, `headless`, `requires` | `document`; `web`, `desktop`; evet; `document` | aynı |
| `permissions`, `undo`, `cost` | yok; `step` (“Ekle”); `instant` | aynı |
| `aliases` | yok: `L`, `LINE`, `CIZGI` arayüz komutu `tool.line`'ındır | yok: `PL`, `PLINE`, `COKLUCIZGI` `tool.polyline`'ındır |
| `examples` | iki çizgi; biri renkli, öznitelikli, beklenen sürümlü | üç noktalı; ikinci kenarı çeyrek daire yayı, renkli, beklenen sürümlü |

**Girdiler:**

| `LineCreate` | Anlamı |
|---|---|
| `layerId` | Hedef katmanın kimliği; grup değil |
| `a`, `b` | Başlangıç ve bitiş: x doğu (Y), y kuzey (X), float64. Belgedeki çizginin alanlarıyla aynı adlar |
| `color`?, `attrs`?, `expectedRevision`? | `PolygonCreate`'teki gibi |

| `PolylineCreate` | Anlamı |
|---|---|
| `layerId` | Hedef katmanın kimliği; grup değil |
| `pts` | Noktalar sırasıyla; en az 2 |
| `bulges`? | **Kenar başına** bir yay değeri, yani noktalardan bir eksik: tan(θ/4), saat yönünün tersi artı, 0 düz |
| `color`?, `attrs`?, `expectedRevision`? | `PolygonCreate`'teki gibi |

**Çıktı ve plan:** `LineCreated`, `PolylineCreated` `{ uid, id, revision }`; `LinePlan`, `PolylinePlan` `{ entity, revision }`. Biçim `PolygonCreated` ve `PolygonPlan`'ınkiyle aynıdır.

- **Neden her komutun kendi tipi:** komutun sürümü kendi girdi ve çıktı şemasını kapsar (ADR 0013). Ortak bir `EntityCreated` bir gün değişirse onu kullanan her komutun sürümünü birlikte açtırırdı. Aynı biçimli iki küçük tip bu bağdan ucuzdur. `cad.polygon.create` v1'in adları değişmedi.

### Çizgi zinciri parça başına bir çağrıdır

- **Karar:** çizgi aracı zincirin her parçası için `cad.line.create`'i bir kez çalıştırır. Her parça ayrı nesne ve ayrı geri alma adımıdır.
- **Neden:** bugünkü davranış budur ve izler onu tutar. Geri (G) son parçayı geri alır. Komuttan sonra Ctrl+Z bir parçayı kaldırır. Zinciri tek komut yapmak onu tek adıma çevirir; bu, ADR 0018'in Ctrl+Z kararını değiştirirdi.
- Zincirin tamamını bir çağrıda isteyen bir otomasyon aynı komutu parça parça çağırır ya da bir işleme (`transact`) sarar. İşlemin içinde komut ona katılır (ADR 0022).

### Çoklu çizginin yay değerleri kenar başınadır

- Girdide kenar başına bir değer verilir: `n` nokta için `n − 1` değer. Kapalı alanın kuralıyla aynıdır: “her kenara bir değer”. Açık yolun kapanış kenarı yoktur.
- Belge ise yay değerini DXF LWPOLYLINE gibi **nokta başına** saklar. Komut verilen değerlerin sonuna olmayan kapanış kenarı için `0` ekler. Çoklu çizgi aracı hep böyle yazdı; kayıtlı dosya değişmez.
- **Neden nokta başına değil:** açık yolda son değerin anlamı yoktur. Python ya da AI çağıranı için doğal sayı kenar sayısıdır.
- Başka bir sadeleştirme yok: sıfır yay değerleri de yazılır (CLAUDE.md §23.3).

### Denetimler ve sıraları

İlk tutmayan denetim cevap verir. Beklenen sürüm ve katman denetimleri, ileti ve yol dahil, üç komutta aynıdır. İkisi ortak dosyalara taşındı: `crates/native/application/src/checks.rs`, `apps/web/src/product/checks.ts`. Kapalı alanın 23 durumu değişmeden geçer.

| Sıra | `cad.line.create` | `cad.polyline.create` | Durum | Yol |
|---|---|---|---|---|
| 1 | `not_finite`: önce `a`, sonra `b`; her uçta önce x | `too_few_points` | failed | `a.x` … `b.y`; `pts` |
| 1 | | `not_finite` (noktalar), `bulge_count`, `not_finite` (yay değerleri) | failed | `pts[i].x`, `bulges`, `bulges[i]` |
| 2 | `invalid_revision`, `revision_conflict` | aynı | failed, conflict | `expectedRevision` |
| 3 | `layer_not_found`, `not_a_layer`, `layer_locked` | aynı | failed | `layerId` |
| — | `layer_hidden` | aynı | uyarı | `layerId` |
| — | `slots_exhausted` (yalnız masaüstü) | aynı | failed | — |

- **Yeni iletiler** (tam metin durum dosyalarında):
  - `Başlangıç noktasının doğu (Y) değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin.`
  - `Çoklu çizginin en az 2 noktası olmalı; 1 nokta verildi. Eksik noktaları ekleyin.`
  - `Yay değerleri kenar sayısı kadar olmalı, her kenara bir değer: 3 nokta, 2 kenar, 3 yay değeri verildi. Eksik ya da fazla değerleri düzeltin.`
- Kilitli ve gizli katman iletileri araçların kendi sözleridir (`tools/targetLayer.ts`).
- **Denetlenmeyen:** uçları çakışan çizgi, kendini kesen yol, sıfır uzunlukta kenar. Geometrik geçerlilik ADR 0022'deki gibi denetlenmez. Çizgi aracı çakışan uçlu çizgi zaten yazmaz: aynı yere ikinci tıklama nokta eklemez.
- **Hesap yok:** denetimler sayım, sonlu sayı ve katman ağacıdır (CLAUDE.md §4.8.1).

### Ortak durumlar

- `fixtures/commands/v1/cad.line.create.json` 20 durum, `cad.polyline.create.json` 23 durum.
- Beklenenler bu karardan elle yazıldı. Bir uygulamanın çıktısından kopyalanmadı.
- Koşucular komuta göre ayrılır: web `fixtures.test.ts`, masaüstü `tests/fixtures.rs` (girdinin NaN yerleri komut başına). Her dosyada en az 20 durum aranır. Kayıttaki her komutun bir dosyası olmalıdır.
- **Kapsam:** yazma, geri alma, aynı kimlikle yineleme; her nesneye yeni kimlik; çizgide zincirin parça parça geri alınması; çoklu çizgide yay değerlerinin saklandığı biçim; renk, öznitelik, grubun içindeki katman; gizli, kilitli katman ve grupları; bilinmeyen katman; grup; nokta sayısı; NaN ve ±∞ (uçlar, noktalar, yay değerleri) ve ilk bozuk değerin sırası; yay değeri sayısı; beklenen sürüm, çakışma, yazımı bozuk sürümler; denetim sırası; doğrulama ve plan hiçbir şey yazmaz; plan ile sürümüyle yazma.

### Araçlar komuttan yazar

- **Web:**
  - `LineTool` her parçayı `lineCreate.execute` ile yazar. Etkin katman ve güncel renk girdidedir (`CMD-07`).
  - Yazılan çizgi, Geri (G) ve Ctrl+Z için `PointInputTool.noteMade` ile kaydedilir.
  - `PathTool`'un açık biçimi `polylineCreate.execute` ile yazar ve çizilen parça başına yay değerlerini verir.
  - Katalogda `productCommand` alanı: `tools[line]` → `cad.line.create`, `tools[polyline]` → `cad.polyline.create`.
- **Masaüstü (`kentos-interaction`):**
  - Kapalı alan aracı iki biçimli **yol aracına** dönüştü (`path.rs`, `Shape::Closed`/`Open`). Kod kopyalanmadı: Kapalı alan `cad.polygon.create` ile, Çoklu çizgi `cad.polyline.create` ile yazar.
  - **Çizgi aracı** (`line.rs`) web'in `LineTool`'udur:
    - Geri (G) son çizgiyi, çizim o zamandan beri değişmediyse geri almayla kaldırır, değiştiyse siler.
    - Kapat (K) üç nokta ve üstünde ilk noktaya döner ve zinciri bitirir.
    - Ctrl+Z ADR 0018'in sırasını izler.
    - Onay “N çizgi eklendi.” der.
  - İkisinin ortak parçaları `points.rs`'tedir: etkin imleç, noktanın yankısı, komutun cevabının iletiye dönüşü.
  - İletiler kelimesi kelimesine aynıdır. Hesabın tamamı `kentos-geometry-core`'undur.
  - `tool.line` (L) ve `tool.polyline` (P) şeritten, kısayoldan ve komut satırından çalışır. Masaüstünde 24 / 164 komut.
- **Değişmeyenler:** iletiler ve sıraları, geri alma adı “Ekle”, parça başına (çizgi) ya da nesne başına (çoklu çizgi) tek adım, araç sonrası durum.
- **Doğrudan yazmaya devam edenler:**
  - Geri'nin yedek yolu: çizim değiştiyse çizgi `remove` ile silinir (“Sil”), iki platformda da.
  - `PathTool`'un parsel biçimi: numara, tapu alanı, seçim.
  - Ölçme hiçbir şey yazmaz.

### Etkileşim izleri

`fixtures/interaction/v1`'e üç iz eklendi. Web onları tarayıcıda, masaüstü native oynatıcıda, üç varyantta ve değiştirmeden geçer.

| İz | Ne tutar |
|---|---|
| `line-chain` | tıkla, `12`, Enter, `@0,8`, tıkla, Geri (yinelenebilir geri alma), Ctrl+Z, Kapat, parça parça geri alma, son komutu yinele, sağ tık, tek noktayla onay, boş taslakta sağ tık |
| `polyline-arc` | tıkla, `12`, Enter, Yay, teğet yay, Geri (yay kipi sürer), Düz, `@-6,0`, Ctrl+Z, sağ tıkla bitirme, tek adımda geri alma, Uzunluk |
| `command-name` | kısayolu olmayan `k` komut satırını açar, Esc siler, `ka` ve Enter Kapalı alan'ı başlatır, klavye çizime döner (değer alanı açılır) |

- **Biçim:** alan eklenmedi. `newest.points` bir çizgide iki uçtur. İki oynatıcı da böyle okur.

### Oynatıcı komut satırını izler

- **Bileşen işlemleri:** oynatıcı görevlerdeki işlemi komut satırının bir modelinde çalıştırır (`traces/command_line.rs`). Model komut satırının yazı kutusudur (`Focusable`, `TextInput`).
  - Çizimde yazılan harf onu odaklar (`focus`). Komut satırından başlayan araç odağı bırakır (`unfocus`). Model değişikliği, bileşenin yaptığı gibi uygulamaya bildirir.
  - Odağa dokunmayan bir işlem izi yine durdurur: yazı seçmek, özel işlem.
  - **Görevler eylem eylem işlenir.** Çalışma zamanı da böyle işler. Sonuç bildiren bir işlemin görevi (`widget::operate`) işlem çalışıp bırakılmadan bitmez. Bütün eylemleri önce toplamak oynatıcıyı kilitliyordu.
- **Öneri listesi:** sıra bileşenin kendisinindir. Yeni `kentos_ui::widget::command_line::suggested` onu verir; girdisi view.rs'in bileşene verdiğidir (`line_commands`, `App::line_prompt`).
  - Liste, komut satırı klavyedeyken, yazı boş değilse ve bir şey eşleşiyorsa açıktır.
  - Enter ve Boşluk ilk öneriyi çalıştırır: komutu adıyla ya da çalışan komutun seçeneğini. Tab adını yazar.
- **Model testi** (`the_command_line_model_is_the_widget`) gerçek bileşeni ve modeli yan yana sürer: rakamlar, harfler, öneriler, Tab, Enter, Boşluk, Esc ve iki odak işlemi.
  - İşlemler gerçek bileşende yeni `Snapshot::operate` ile çalışır.
  - Model ile bileşenin ayrıldığı ilk tuşta test durur ve iki cevabı söyler.
- **ADR 0018'in Esc'i masaüstünde eksikti.** Komut satırında yazı varken ilk Esc, öneri listesi açıksa yalnız listeyi kapatıyordu, yazı kalıyordu. Web yazıyı hemen siler.
  - KentOS UI komut satırına `escape_clears` seçeneği eklendi (varsayılanı kapalı, vitrin değişmedi); masaüstü onu açar.
  - Bu yeni bir karar değildir, ADR 0018'in uygulanmasıdır. `command-name` izi onu tutar.
- **Görüntü:** `kentos-cad snapshot --iz`, iz komut satırını klavyede bırakınca görüntünün bileşen ağacına da aynı odağı verir. Öneri listesi pencerede olduğu gibi görünür.

### Metin alanında geçen kısayollar

- Envanter üreticisi `allowInInput` bağlarını komut başına yazar: `shortcutsInInput`. Alan yalnız böyle bir akoru olan komutlarda bulunur.
- Masaüstü kataloğu bu alanı okur. Değer alanı ve komut satırı tam bu akorları geçirir.
- `keys::GLOBAL` kaldırıldı. Okunan küme onun 18 akorudur. Bir katalog testi okumayı tutar: Ctrl+S ve F tuşları geçer, Ctrl+Z geçmez, her biri komutunun kısayoludur.

### Dosya düzeni

- `apps/desktop/src/traces.rs` sorumluluklarına göre bir klasöre bölündü, davranış değişmeden:
  - `format.rs`: izin biçimi;
  - `keyboard.rs`: varyantlar ve klavyeler;
  - `command_line.rs`: komut satırı modeli ve testi;
  - `player.rs`: oynatıcı;
  - `compare.rs`: karşılaştırma.
- Bölme ayrı bir commit'tir. O commit'te 40 masaüstü testinin hepsi geçti.

### Ortak olan, ortak olmayan

| | Web | Masaüstü | Ortak |
|---|---|---|---|
| İşleyici | `product/lineCreate.ts`, `polylineCreate.ts`, `checks.ts` | `native/application/src/line.rs`, `polyline.rs`, `checks.rs` | — |
| Girdi, çıktı, plan | üretilen TS tipleri | `kentos-contracts` | Rust sözleşme tipleri, JSON Schema |
| Davranış, kodlar, iletiler | | | `fixtures/commands/v1` |
| Araç | `LineTool`, `PathTool` | `kentos_interaction::{line, path}` | `fixtures/interaction/v1` |
| Komut satırı | DOM `<input>` ve kendi önerileri | KentOS UI bileşeni; izlerde modeli | izler; model testi |
| Metin alanındaki akorlar | `allowInInput` | envanterin `shortcutsInInput`'u | `docs/inventory/web.json` |

## Sonuçlar

- **Testler:**
  - `kentos-native-application`: katalog eşitliği; 66 ortak durum (23 + 20 + 23); masaüstüne özgü iki test.
  - `kentos-interaction`: kapalı alanın 13, çizginin 11, çoklu çizginin 8 kuralı, tek test tezgâhında (`tests/common`).
  - Masaüstü: 42 test. Yedi iz üç varyantta, komut satırı modeli, metin alanındaki akorlar ve iki aracın kısayolundan ve adından başlaması bunlara dahil.
  - KentOS UI: `suggested`'ın sırası.
  - Web: 66 ortak durum, kayıt eşitliği, her komutun dosyası.
- **Bağımlılıklar:** yeni paket yok. `Cargo.lock` değişmedi.
- **Ters deneme:**
  - Çizginin Geri'si geri almak yerine hep silince `line-chain` iki platformda da 9. adımda düştü (`key G`: `canRedo: false, beklenen true`).
  - Çoklu çizginin Geri'si yay kipinden de çıkınca `polyline-arc` iki platformda da 8. adımda düştü (seçenekler `["Y","U","G","Enter"]`, beklenen yay seçenekleri).
  - Masaüstünde `escape_clears` kaldırılınca model testi ilk ayrılan tuşta durdu: `Esc: widget [], model [CommandInput("")]`.
  - Komut durumlarında bir ileti değiştirilince native koşucu tam o iletiyi söyleyen 4 durumda düştü.

## Ertelenenler

- **Komut çalışırken komut satırının önerileri: karar verildi (26 Eylül, önerilen varsayılan).** Masaüstü de web gibi komut çalışırken komut önermez (`App::line_commands`); yazılan çalışan komutundur, Enter onu araca verir (ADR 0018). Komut adı yazıp Enter'a basmak artık başka komutu başlatıp taslağı bırakmaz; araç “anlaşılamadı” der, taslak kalır (`a_running_command_owns_what_is_typed`).
  - Masaüstü çalışan komutun seçeneklerini önermeye devam eder. Kalan küçük fark: seçenek adının başı (`uzu`) masaüstünde öneriden seçilir, web'de seçilmez. Seçeneğin harfi iki platformda aynıdır.
  - İzlere girmedi.
- **Bilinmeyen komut adı:** web yazıyı komut satırında seçili bırakır; masaüstü satırı boşaltır ve “Bilinmeyen komut” der. Küçük fark, izlere girmedi (`UX-10`).
- Öneri listesinde ok tuşları izlerin tuşları arasında yok.
- Öbür araçların ürün komutları: nokta, daire, yay, dikdörtgen, parsel … `CMD-08` ve `CMD-09` ADR 0022'deki gibi açık.

## Doğrulama (26 Eylül 2026, Linux; main `b7a154c` üstünde)

- `cargo fmt --all --check` temiz.
- `pnpm rust:test` (üç parçası ayrı ayrı: `cargo test`, `cargo clippy --all-targets -- -D warnings`, `node scripts/arch/deps.mjs`):
  - 508 test geçti, clippy temiz;
  - bağımlılık yönü temiz (18 crate, 23 crate × hedef);
  - veritabanı testleri yerel sunucuda çalıştı: `KENTOS_TEST_DB=required cargo test -p kentos-application -p kentos-postgres -p kentos-api` 56 test geçti.
- `pnpm rust:test:desktop`: masaüstü 42, render 28, KentOS UI 171, vitrin 55 test geçti; clippy temiz.
- `cargo test -p kentos-desktop traces`: 7 iz × 3 varyant ve komut satırı modeli geçti.
- `cargo test -p kentos-native-application`: 66 ortak durum ve masaüstüne özgü iki test geçti. `cargo test -p kentos-interaction`: 36 test geçti.
- `pnpm typecheck` temiz. `pnpm test`: 1098 geçti, 13 atlandı (başlangıçtaki 13).
- `pnpm inventory:check` güncel.
- `pnpm e2e:interaction`: 7 iz × 3 varyant geçti. `pnpm e2e`: 160 denetimin hepsi geçti.
- Görüntüler (`KENTOS_SNAPSHOT_BACKEND=wgpu kentos-cad snapshot --iz`):
  - `line-chain` 8. ve 11. adım: çizgi zinciri, değer alanında `@0,8`, ölçü etiketi, iki yerde istem;
  - `polyline-arc` 7. ve 14. adım: teğet yay, değer alanında `@-6,0`;
  - `command-name` 4. adım: komut satırında `ka`, öneri listesinde vurgulu Kapalı alan.
- Pencerede (`make desktop`) elle klavye ve fare denemesi bu çalışmada yapılmadı.
