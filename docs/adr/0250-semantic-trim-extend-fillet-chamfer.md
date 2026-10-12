# ADR 0250: Anlamsal Buda, Uzat, Köşe yuvarla ve Pah; yerleriyle ilişkili ölçü

- **Durum:** kabul edildi (2026-10-13). [plan/cad.md](../../plan/cad.md) §24.1'in beşinci işi (TODOS.md `CADP-05`; plan
  `CAD-CMD-03`, `04`, `06`). Kalıp Ötele'ninkidir ([ADR 0246](0246-semantic-offset-command.md)). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0047 (değiştirme araçları), ADR 0140 (Çit, tüm köşeler), ADR 0249 (alt nesne başvuruları ve ilişkili ölçü).
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

Buda, Uzat, Köşe yuvarla, Pah ve tüm köşeler araçları sonucu kendileri hesaplayıp `cad.entities.edit` ile hazır geometri
yazıyordu. Python ve ajan aynı işi yapmak için geometriyi kendileri bulmak zorundaydı. Buda ve Uzat ayrıca sınırlarını
örtük görünüm durumundan alıyordu: görünümdeki görünen bütün kenarlar. Bu, komutun girdisinde yoktu (TODOS.md `CMD-07`).

## Karar

### 1. Dört yeni ürün komutu

| Komut | Adım | Girdi |
|---|---|---|
| `cad.entities.trim` v1 | Buda | `picks` (nesne ve tıklanan yer), `boundaries`, `expectedRevision` |
| `cad.entities.extend` v1 | Uzat | aynı (`EntitiesTrim`) |
| `cad.entities.fillet` v1 | Köşe yuvarla | `corner`, `radius`, `trim` (Kırp, varsayılan açık), `expectedRevision` |
| `cad.entities.chamfer` v1 | Pah | `corner`, `distance`, `second` (yoksa `distance`), `trim`, `expectedRevision` |

Tel üzerinde uzunluklar ve koordinatlar metredir (ADR 0165 §2). Her komutun planı yazmadan sonucu verir; planın sürümüyle
yazma tam o planı yazar.

### 2. Buda ve Uzat

- **Sınırlar** (`boundaries`) açıktır:
  - `all` (varsayılan): görünen bütün nesneler. Görünen: katmanı açık, katman süzgecinden geçen (ADR 0211); zaman dilimi
    gösteren ev sahibinde o dilimde (ADR 0210).
  - `area {min, max}`: dikdörtgene değen görünen nesneler. Araç kendi görünümünü verir; AutoCAD'in hızlı kipi budur.
  - `objects {uids}`: verilen nesneler (Sınır seç), görünsün ya da görünmesin.
  - Nesne kendini sınırlamaz.
- **Seçimler** (`picks`):
  - Aynı nesnenin seçimleri sırayla işlenir. Buda'da ikinci ve sonraki seçim, öncekilerin bıraktığı parçalardan üstünde
    durduğunu (1 µm içinde) keser; Çit'in kuralı budur.
  - Uzat'ta her uç bir kez uzar. Hangi ucun uzayacağı, öncekilerin uzattığı geometriye göre seçilir.
  - Her nesne, komuttan önceki çizimle sınırlanır.
- **Yazma:**
  - Budanan nesnenin yerini kalan ilk parça alır; öbür parçalar ondan yeni nesnelerdir.
  - Tek parça kalırsa nesnenin verisi (öznitelikler, etiket) kalır; kapalı alanda kalmaz (ADR 0047'nin kuralı).
  - Uzayan nesne yeni geometrisini alır.
- **Hata davranışı:**
  - Bir şey yapamayan seçim `trim_failed` (`extend_failed`) uyarısıyla atlanır; ileti çekirdeğin sözüdür.
  - Hiçbir seçim bir şey yapamazsa ilkinin nedeniyle reddedilir. Adalı alan budanmaz (`not_trimmable`).
- **Depo:** sınırların sorgusu paylaşılan geometri deposundandır (`trim_preview`, `extend_preview`). Komut bağlamı
  (`ExecutionContext`) ev sahibinin güncel deposunu isteğe bağlı taşır (`with_store`):
  - Masaüstü kendi deposunu verir; tıklamada yeniden kurulmaz.
  - Başsız sunucu, Python ve MCP'de işleyici belgeden aynı depoyu kurar (`geometry_store::store_of`).
  - Depo yanıtı değiştirmez, yalnız maliyetini.

### 3. Köşe yuvarla ve Pah

- **Köşe** (`corner`):
  - `vertex {uid, index}`: çoklu çizginin ya da alanın düz kenarlar arasındaki köşesi; alanın köşeleri parça parça sayılır.
  - `lines {first, second}`: iki çizgi; her biri tıklandığı tarafı tutar.
  - `all {uids}`: çoklu çizgi ve alanların bütün köşeleri (Tüm köşeleri yuvarla, Tüm köşelere pah). Bu kipte Kırp hep
    açıktır.
- **Kırp:**
  - Açıkken kenarlar yeni geometrisini alır; iki çizginin yayı ya da pahı ilk çizgiden yeni nesnedir.
  - Kapalıyken kenarlar kalır, yalnız yay ya da pah eklenir.
  - Sıfır yarıçap ya da iki sıfır mesafe iki çizgiyi buluştukları noktada birleştirir.
- **`all` kipi:** sığmayan köşe atlanır ve `skipped`'te sayılır. Hiçbir köşesi sığmayan nesne `fillet_failed`
  (`chamfer_failed`) uyarısıyla atlanır.
- **Retler:**
  - `not_a_corner`: köşe olmayan yer.
  - `invalid_distance`: eksi değer; Kırpmadan ya da `all`'da sıfır.
  - `fillet_failed` / `chamfer_failed`: sığmayan değer; ileti çekirdeğin sözüdür.
- **Hesap** çekirdeğindir: `path_corner_at`, `lines_corner_at`, `corner_in_path`, `fillet_lines`, `chamfer_lines`,
  `fillet_arc`, `chamfer_line`, `all_corners`. Araçların hesabı (`corner/plan.rs`) uygulamaya taşındı.

### 4. Araçlar

- Buda ve Uzat'ın tıklaması ve Çit'i komutu çalıştırır. Önizleme komutun planıdır (silinecek kısım kırmızı kesikli, uzayan
  nesne kesikli).
- Köşe yuvarla ve Pah önizlemeyi planla çizer, uygulamayı komutla yapar. Tüm köşeler araçları `all` köşesiyle yazar.
- Görünen davranış ve iletiler değişmedi.

### 5. Yerleriyle ilişkili ölçü

- `cad.entities.create` ilişkili ölçüyü yerleriyle yazar: canlı başvurusu olan nokta, verilen koordinat ne olursa olsun
  yerinin şimdiki noktasına konur. Python ve ajan ölçüyü nesnenin yerleriyle ister, koordinat hesaplamaz.
- Kopuk başvurunun noktası verildiği gibi yazılır.
- Retler: `entity_not_found` (başvurunun nesnesi), `no_such_place` (yer).

## Uygulama

- **Sözleşme:** `crates/shared/contracts/src/cad_trim.rs` (`EntitiesTrim`, `TrimBoundaries`, `TrimPick`, `EntitiesTrimmed`,
  `EntitiesTrimPlan`), `cad_corner.rs` (`EntitiesFillet`, `EntitiesChamfer`, `CornerPick`, `EntitiesCornered`,
  `EntitiesCornerPlan`); katalog kaydı ve örnekleri; `cad_create.rs`'in açıklaması.
- **İşleyiciler:** `crates/native/application/src/trim.rs` (`Act`, `replace_changes`), `corner.rs` (`Size`, `Corner`,
  `Site`, `done`, `piece`), `edge_commands.rs` (`entities_trim`, `_extend`, `_fillet`, `_chamfer`), `geometry_store.rs`
  (`store_of`, `record`), `context.rs`'in `store`'u, `create.rs`'in `placed_points`'i. Yeni ret kodları: `not_trimmable`,
  `trim_failed`, `extend_failed`, `not_a_corner`, `fillet_failed`, `chamfer_failed`.
- **Araçlar:** `crates/native/interaction/src/trim.rs`, `corner/` (`site.rs` uygulamanın türleriyle, `plan.rs` yönlendirme),
  `reshape.rs` (tüm köşeler).
- **Başsız sunucu, MCP, Python:** `dispatch.rs`; `tools.rs`'te dördü `destructiveHint`; `python/kentos/cad/` üretildi.
- **İşlem manifesti:** `trim`, `extend`, `fillet`, `filletAll`, `chamfer`, `chamferAll` `semantic`.

## Doğrulama

- **Bağımsız başvuru** `scripts/fixtures/edge_command_cases.py` (KentOS kodu olmadan, ADR 0047 ve bu ADR'nin tanımından):
  - Eksenlere paralel çizgilerde budama ve uzatma kesirlerle hesaplanır.
  - Köşenin teğet noktaları, merkezi, açıları ve bükümü tanımın sırasıyla çift duyarlıkta hesaplanır. Aşkın değerler
    (yarım açının tan'ı ve sin'i, bükümün tan((π − φ)/4)'ü) 60 basamaklı seriyle bulunup en yakın double'a yuvarlanır;
    platformun libm'i kullanılmaz. Yolda görülen: macOS'un `tan(π/8)`'i bir ulp sapıyor, çekirdeğin fdlibm'i doğru
    yuvarlıyor.
  - 85 durum `fixtures/commands/v1/cad.entities.{trim,extend,fillet,chamfer}.json`'da (21, 20, 22, 22): sınırların üç
    biçimi, gizli katman, Çit sırası, uyarılar, plan, sürüm, retlerin sırası; yolun köşesi, iki çizgi, sıfır yarıçap, Kırp
    kapalı, tüm köşeler, sığmayan köşeler.
  - Masaüstü (`fixtures.rs`) ve Python (`test_command_cases.py`) bit bit geçer.
- **Yerleriyle ilişkili ölçü:** `create_command_cases.py`'ye iki durum (yerinden nokta, kopuk bağın noktası; boş başvuru,
  yeri olmayan, nesnesi olmayan).
- **Araç = komut:** `crates/native/interaction/tests/all/edits.rs`'in
  `the_trim_and_extend_tools_write_what_the_commands_write_without_a_store` testi. Aracın görünümle ve masaüstü
  deposuyla yazdığı, başsız sunucunun depo kurup `all` ile yazdığına eşittir.
- **İzler:** `edge-tools.json`, `corner-tools.json` (değişmedi) ve yeni `edge-commands.json` masaüstünde üç çeşitte geçer.
- **Resimler:** `kentos-cad kullan edge-commands` (Buda'nın ve Uzat'ın önizlemesi, sonuç, Köşe yuvarla'nın önizlemesi ve
  sonucu).

## Sonuçlar

- Buda, Uzat, Köşe yuvarla ve Pah'ın ne yaptığı katalogda bir sözleşmedir; Python ve ajan aracın sonucunu ister,
  hesaplamaz.
- Açık kalanlar:
  - Esnet, Birleştir, Kır, Böl, Sınır, Tarama ve ölçü oluşturmanın kalan yöntemleri (`CAD-CMD-05`).
  - Eğri ve elips sınırlarının bağımsız başvurusu.
  - İki çizginin Kırpmadan yayı ortak durumlarda yok (Kırpmadan yalnız pah sınanıyor).
