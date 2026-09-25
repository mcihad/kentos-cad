# ADR 0020: Masaüstünün native belgesi ve web'le ortak işlem fixture'ları

- **Durum:** kabul edildi (2026-09-25). Yön TODOS.md `DOM-02`, `TX-01` ve §2.1'dendir (`native/domain`); crate, fixture biçimi ve aşağıdaki farklar bu dilimin kararıdır.
- **Tarih:** 2026-09-25
- **Bağlam belgesi:** TODOS.md `DOM-02`, `TX-01`, `UI-11`, `ARCH-01`, `ARCH-04`; ADR 0003 (işlem), 0010 (platform sınırı), 0014 (kimlik), 0017 (masaüstü kabuğu)

## Bağlam

- Web'in belgesi TypeScript `CadDocument`'tir (`apps/web/src/model/document.ts`):
  - ekleme, değiştirme, silme ve toplu karşılıkları;
  - hep ya da hiç çalışan işlem, iç içe kayıt noktası, grup ve iptali (ADR 0003);
  - geri alma, yineleme, kirli bayrağı, sürüm ve `markSaved(revision)`.
- Masaüstü kabuğu (ADR 0017) çizimi çıplak `DocumentSnapshotV1` olarak tutuyordu. Kendi kirli bayrağı ve katman düğmeleri vardı, geri alma yoktu.
- TODOS.md `DOM-02` şunu ister: web belgesinin davranışı ortak fixture'larla belgelensin, native belge bu sözleşmeyle geliştirilsin. TS belge sahibi korunur, Rust facade'a dönüştürülmez.
- ADR 0010: uyum ortak çalışma zamanıyla değil, sürümlü sözleşme, ortak fixture ve davranış testiyle kurulur.

## Karar

### Native belge: `crates/native/domain`

- Paket `kentos-domain`, crate `kentos_domain`. Saf Rust, yalnız native. Bağımlılıkları `kentos-contracts` (varsayılan özellikler kapalı) ve UUID üretimi için `uuid`'dir.
  - `scripts/arch/deps.mjs`'te kendi grubu vardır: `domain`. Yalnız `shared`'i kullanır. Sunucu çalışma zamanı, tarayıcı bağları, Iced/wgpu, PyO3, GDAL ve PROJ altında bulunamaz.
  - `native`, `server`, `api` ve `desktop` grupları onu kullanabilir. Tarayıcı paketleri (`wasm`) kullanamaz.
  - Çalışma alanının `default-members`'ındadır: `pnpm rust:test` onu da sınar.
- `Document` bir `.kcad` dosyasının tuttuğu her şeyi tutar: ad, ayarlar, yerel orijin, başlangıç görünümü, katman ağacı, nesneler, proje stilleri.
  - `from_snapshot` / `to_snapshot`: v1 dosyası değişmeden gidip gelir. Okurken web okuyucusunun belge için gerekli denetimleri yapılır: en az bir katman, benzersiz pozitif kimlik, nesnenin katmanı dosyada.
- **Düzenlemeler web'deki gibidir:**
  - `add`, `add_many`, `update`, `update_many`, `remove`, `set_layer_style`: her çağrı bir geri alma adımıdır ya da açık işleme katılır. Aynı kimlik iki kez verilirse ikinci değişiklik birincinin sonucunun üstüne yazılır.
  - `transact`: gövde hata döndürürse yaptıkları en yeniden başlayarak geri alınır. Hiçbir şey kaydedilmez; kirli bayrağı, sürüm, geri alma ve yineleme olduğu gibi kalır. İç içe işlem bir kayıt noktasıdır.
  - `begin_group` / `end_group` / `cancel_group`: bitene dek gelen adımlar tek adım olur. İptal yaptıklarını geri alır.
  - Geri alma geçmişi 200 adımdır. Yeni adım yinelemeyi siler. Verilen kimlik geri alınan işlemden sonra da yeniden verilmez.
  - Katman görünürlüğü, kilidi, “yalnızca bunu göster”, “tümünü göster” ve yeniden adlandırma belgeyi kirletir ama geri alma adımı değildir. Grubu açıp kapamak ve etkin katmanı seçmek düzenleme değildir.
  - Model katman kilidini denetlemez; web'de de denetlemez. Kilitli katmana yazmayı araçlar ve komutlar `LayerTree::is_locked` ile sorup reddeder (CLAUDE.md §7).
- **Kimlik (ADR 0014):**
  - Her nesnenin bir **yuvası** vardır: `Slot(u32)`. Web'in `Entity.id`'sidir, v1 dosyasının yerel kimliği de odur.
  - Her nesnenin bir de **kalıcı kimliği** vardır: yeni nesnede UUIDv7. Değiştirmek korur. Geri alma ve yineleme aynı kimliği geri getirir. Belge `uid → yuva` dizinini tutar.
  - v1 dosyasından okunan nesnelerin kimliği tek bir işlevden gelir: `v1_entity_uids`. İşlev ADR 0014'ün belirlenimli UUIDv5 göçünü çağırır (`DOM-04`, `kentos_contracts::v1_uids`). Aynı dosya masaüstünde ve web'de aynı kimlikleri alır; `tests/identity.rs` bağımsız referansın fixture'ıyla sınar (`fixtures/document/v1/identity`).
    - İlk sürümde işlev geçici olarak v7 veriyordu. Göç yalnız gövdesini ve dönüş tipini değiştirdi: yerel kimlikler benzersiz pozitif sayı değilse `Result` ile reddeder. `from_snapshot` bunları önce denetlediği için bu yola varılmaz, ama hata saklanmaz.
- **Masaüstü** (`apps/desktop/src/document.rs`): `Document` = model + dosya yolu + oturum numarası.
  - Açma `DocumentSnapshotV1::from_json` → `Document::from_snapshot`, kaydetme modelin anlık görüntüsüdür.
  - Katman paneli ve `layer.showAll` modelden geçer.
  - `edit.undo` ve `edit.redo` taşındı: Ctrl+Z, Ctrl+Y, Ctrl+Shift+Z. Hızlı erişim düğmeleri geri alınacak adım yokken soluktur (web'in `isEnabled`'ı).
  - Kayıt sonucu açılan çizimin oturum numarasını taşır. Başka bir çizim açıldıktan sonra biten kayıt onu “kaydedildi” saymaz, yolunu değiştirmez.

### Ortak fixture'lar: `fixtures/document-ops/v1`

- Altı dosyada 33 senaryo; ADR 0014'ün ilk dilimiyle yedinci dosya geldi (`identity.json`, 2 senaryo). Biçim `fixtures/document-ops/README.md`'dedir.
  - Her senaryo bir `.kcad` v1 çizimini uygulamanın açtığı yoldan açar, sonra adımları uygular.
  - Her adımdan sonra durum denetlenir: kimlikler (belge sırasıyla), nesnelerin tamamı, katman başına nesneler, `canUndo`/`canRedo`, kirli bayrağı, sürüm, katman bayrakları ve miras alınan yanıtlar, etkin katman.
  - Kalıcı kimlik nesne karşılaştırmasına girmez: rastgeledir (v7) ya da dosyadan türetilir (v5). Kimlikler birbirleriyle karşılaştırılır (`captureUid`, `uids`).
- Aynı dosyaları iki koşucu çalıştırır:
  - web: `apps/web/src/model/documentOps.test.ts`;
  - masaüstü: `crates/native/domain/tests/fixtures.rs`.
- **Başvuru web'dir.** Beklenen değerler web'in kodundan elle yazıldı ve iki koşucuyla doğrulandı. Bozulan her beklentiyi iki koşucu da yakaladı.
- **Sürüm sayılmaz, karşılaştırılır** (`"same"` / `"changed"`, `captureRevision` → `markSaved`).
  - Sözleşme şudur: yazılan içerik değişince sürüm değişir, ve kayıt belgeyi yalnız yazdığı sürüm hâlâ güncelse temizler.
  - Web katman stili değişikliğini iki kez sayar, masaüstü bir kez. Bu fark anlam taşımaz.
- **Yama farkı:** web alan yaması alır (`update(id, { p })`), native tam nesne alır. Çeviri koşucudadır: yamayı JSON üstünde nesneye uygular.

### Ortak olan, ortak olmayan

| | Web | Masaüstü | Ortak |
|---|---|---|---|
| Belge kodu | `CadDocument` (TS) | `kentos_domain::Document` (Rust) | — |
| Dosya ve nesne tipleri | üretilen TS tipleri | `kentos-contracts` | `DocumentSnapshotV1`, `Entity`, `LayerNode` |
| Davranış | | | `fixtures/document-ops/v1` |
| Değişiklik bildirimi | `changed`, `attrs`, `touched` olayları | henüz yok | — |
| Kalıcı kimlik | `uid` (ADR 0014 dilim 1) | `uid` | v1 dosyasının türetilen kimlikleri (`fixtures/document/v1/identity`); göreli denetim (`fixtures/document-ops/v1/identity.json`) |
| Kilit denetimi | çağıranda | çağıranda (araçlar henüz yok) | — |

### Web'de düzeltilen farklar (25 Eylül)

Bu beş durumda web yazılı karardan ayrılıyordu; masaüstü baştan karara uyuyordu. Aynı gün web de düzeltildi. Hepsi artık ortak fixture'dadır: web ve masaüstü aynı senaryoyu geçer.


1. **Başarısız işlemde ya da iptal edilen grupta katman stili.** Web'de belge kirli kalıyor, sürüm değişiyordu. Katman deposu her stil değişikliğinde belgeyi düzenlenmiş sayıyordu, geri almada da. Bu ADR 0003'e aykırıdır (“dirty değişmez”). Artık belgenin kendi uyguladığı stil, nesne değişiklikleri gibi yalnız adım kaydedilince kirletir; açık grupta da grup bitince.
   - Web'de açık grupta katman stili belgeyi hemen kirletir; nesne değişiklikleri grup bitene dek kirletmez. Masaüstünde ikisi de grup bitince kirletir.
2. **Taramanın adaları.** Web'in `updateOp`'u çokgen dışındaki her nesneden `holes`'u siliyordu; tarama da çokgen değildir. Taramanın öznitelik değişikliği bile, taşınması da adalarını siliyordu. Veri kaybıydı. Artık adalar yalnız çokgen çoklu çizgiye açılınca düşer (masaüstündeki gibi).
3. **Açık işlem ya da grup varken geri alma.** Web grubun öncesindeki adımı geri alıyordu (örneğin model çalışırken Ctrl+Z); grup bitince o adım bir daha yinelenemiyordu. Artık iki tarafta da geri alma ve yineleme, işlem ya da grup kapanana dek bir şey yapmaz.
4. **Aynı grubun iki kez bitirilmesi.** Web'de ikinci `end()` grubu bir kez daha kaydediyordu. Artık ikinci bitiş bir şey yapmaz. Masaüstünde grup tutamacı bitirilince tükenir; fixture'daki `endGroupAgain` adımını masaüstü koşucusu atlar.
5. **Bilinmeyen katmanı yalnız göstermek.** Web'de bütün katmanlar gizleniyor ve belge kirleniyordu. Artık iki tarafta da bir şey olmaz.

### Sahibin varsayılanlarıyla değişen davranışlar (25 Eylül)

İki taraf ve fixture birlikte değişti:
- **Geri gelen nesne eski yerine döner.** Geri alınan silme, yinelenen ekleme ve başarısız işlemin geri çevirdiği silme nesneyi belgenin sonuna değil eski yerine koyar. Çizim ve dosya sırası silmeden önceki gibidir.
  - Yuvalar hiç yeniden verilmediği için yerini tutmak yeter: web belgesi ve native `Store` silinen yuvanın yerini saklar (`places`, `vacated`).
  - Web'in geometri deposu da aynı kuralı Rust'ta uygular (geometry-core `Store`), böylece seçme, kenet ve işlem araçlarının sırası belgeninkiyle aynı kalır.
- **Hiçbir şeyi değiştirmeyen düzenleme düzenleme değildir.** Adım kaydedilmez, belge kirlenmez, yineleme geçmişi korunur. Buna, her şey görünürken “Tümünü göster” ve aynı adı yeniden vermek de dahildir.

### Web'de garip bulunan ama aynen izlenen davranışlar

Fixture'larda “(web bugün böyle)” notuyla işaretlidir. Masaüstü aynısını yapar; değişirse ikisi fixture'la birlikte değişir.

- Grubun açık/kapalı hâli ve etkin katman dosyaya yazılır ama belgeyi kirletmez; yalnız bunlar değiştiyse kapanışta sorulmaz.
- Başarısız işlemin içinde değiştirilen görünürlük, kilit ya da ad geri alınmaz; işlemin adımı değildirler.
- Geri alma geçmişi 200 adımda en eskisini sessizce bırakır (`TX-06`).
- Grup açıkken (bir model çalışırken) belge kirli görünmez.
- `updateMany` uygulanan yama sayısını döndürür (aynı nesne iki kez sayılır); belgesi “nesne sayısı” der.

## Sonuçlar

- Sınanan: web koşucusu 40 test (33 senaryo), `cargo test -p kentos-domain` 12 test (33 senaryo ve 11 kendi testi), `cargo test -p kentos-desktop` 14 test. Bağımlılık yönü denetimi yeni grubu tanır. Crate'e `tokio` eklenince denetim düşer.
- `kentos-cad snapshot` görüntüsüne `--komut <id>` eklendi (ADR 0017'deki kullanıma ek). Çizim açıldıktan sonra komut çalıştırır; örneğin kısayol penceresini açar ya da geri almayı dener.
- Masaüstünde bugün geri alınabilir bir arayüz düzenlemesi yoktur: çizim araçları ve katman stili menüsü gelmedi. Geri alma ve yineleme hazırdır; ilk araçla birlikte kullanılır.
- **Açık:**
  - Çizim alanı için değişiklik bildirimi (hangi nesneler değişti, `touched`): kirli katman güncellemesi (CLAUDE.md §4.9) buna dayanacak.
  - Masaüstü araçları kilitli katmanı web'deki gibi reddetmeli.
  - Komut satırında Ctrl+Z. Iced'in yazı kutusunun kendi geri alması yoktur, tuşu da yakalamaz; masaüstünde komut satırındayken de çizim geri alınır. Web'de yazı alanındaki Ctrl+Z alanın kendi geri almasıdır.
  - Masaüstü okuyucusu SRID'yi denetlemiyor, web denetliyor (CLAUDE.md §4.8). Nesne alanlarının ayrıntılı denetimi de web okuyucusunda.
  - Dışarıdan gelen değişiklik (`applyExternal`, bulut) ve açık belgeyi yerinde değiştirme (`replaceWith`) masaüstünde yok.
