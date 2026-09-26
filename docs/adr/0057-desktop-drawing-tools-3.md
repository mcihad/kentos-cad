# ADR 0057: Masaüstünde çizim araçları, 3. tur: elips, eğri, yardımcı çizgi, ışın, paralel çizgi, dikler, halka, revizyon bulutu, kot noktası, böl; `cad.entities.create`

- **Durum:** kabul edildi (2026-09-26).
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** CLAUDE.md §4.5, §4.7, §7, §18; TODOS.md `UX-01`, `UX-06`, `CMD-04..07`, `AI-01`; ADR 0013 (ürün komutu sözleşmesi), 0018 ve 0021 (araç oturumu ve izler), 0022 (ilk ürün komutu), 0027, 0032 (çizim araçları, 2. tur), 0047 (`cad.entities.edit`), 0055 (yardımcı çizgilerin çizimi), 0056 (pano ve görünüm araçları)

## Bağlam

- Masaüstü şu web araçlarını “web'de var; masaüstüne henüz taşınmadı” diye geri çeviriyordu: Elips (`tool.ellipse`), Eğri (`tool.spline`), Yardımcı çizgi (`tool.xline`), Işın (`tool.ray`), Paralel çizgi (`tool.parallel`), Dik in (`tool.perpIn`), Dik çık (`tool.perpOut`), Halka (`tool.donut`), Revizyon bulutu (`tool.revcloud`), Kot noktası (`tool.spot`), Böl (`tool.divide`).
- Web'in yazma yolu koddan okundu (26 Eylül):
  - Kot noktası `cad.point.create` ile yazıyordu (ADR 0032).
  - Elips, eğri, yardımcı çizgi, ışın, halka ve revizyon bulutu belgeye kendileri yazıyordu: `PointInputTool.create` → `writableLayer` → `doc.add`.
  - Paralel çizgi `doc.transact('Paralel çizgi', …)` içinde her yanı ve ekseni ayrı `create` ile yazıyordu. Dikler `doc.transact('Dik in' | 'Dik çık', …)` içinde `doc.add` ile. Böl `doc.transact('Böl', …)` içinde noktaları `doc.add` ile yazıyordu; kilitli katmanı kendisi denetliyor, gizli katmanı hiç denetlemiyordu.
- ADR 0022: “öbür bütün araçlar kendi ürün komutuyla taşınacak”. Masaüstünün bütün araçları ürün komutundan yazıyor.
- Hesap zaten ortak çekirdekteydi; web onu WASM'la çağırıyordu: elipsin kurulumu ve parametreleri, Catmull-Rom eğrisi, yardımcı çizginin doğrultusu, paralel yanlar ve koridor alanı, dik ayak ve dik boy, halkanın halkaları, bulutun yayları, bölme noktaları.

## Karar

### Yazma yolu: araç araç

| Araç | Ürün komutu | Geri alma adımı |
|---|---|---|
| Elips, Eğri, Yardımcı çizgi, Işın, Halka | `cad.entities.create` | “Ekle” |
| Paralel çizgi | `cad.entities.create`, `operation: parallel` | “Paralel çizgi” |
| Dik in, Dik çık | `cad.entities.create`, `perpendicularIn`, `perpendicularOut` | “Dik in”, “Dik çık” |
| Böl | `cad.entities.create`, `divide` | “Böl” |
| Revizyon bulutu | `cad.polygon.create` (yay değerli kapalı alan) | “Ekle” |
| Kot noktası | `cad.point.create` (kot katmanı, kot, yazı, öznitelikler) | “Ekle” |

**Neden tek genel komut, türe göre beş yeni komut değil:**
- Paralel çizgi ve Böl birden çok nesneyi tek adımda yazar; türe göre komutlar (`cad.line.create` …) tek nesneliktir.
- Nesnenin geometri tipi (`EntityGeometry`) ve denetimleri `cad.entities.edit`'te iki platformda zaten vardı. Aynı kurallar ikinci kez yazılmadı: iki komut aynı denetim işlevini çağırır (masaüstü `edit::check_geometry`, web `checkGeometry`). Bir komutun kabul ettiği geometriyi öbürü de kabul eder.
- Python ve AI için “bir nesne ekle” tek çağrıdır. Türe göre komutlar olduğu gibi kalır.
- Revizyon bulutuna ve kot noktasına var olan komut tam uyar; yeni komut gerekmedi.

### Katalog kaydı: `cad.entities.create` v1

`crates/shared/contracts/src/cad_create.rs`, `catalog.rs`:

| Alan | Değer |
|---|---|
| `effect`, `hosts`, `headless` | `document`; `web`, `desktop`; evet |
| `requires`, `permissions` | `document`; yok (yerel çizim; bulutta değişiklik `project.changes` ile gider) |
| `undo`, `cost` | `step`; `instant` |
| `examples` | Bir elips (çıktısıyla); beklenen sürümlü paralel çizgi |

- **Girdi** `EntitiesCreate { layerId, objects, operation?, expectedRevision? }`. Her nesne `NewObject { geometry, color?, attrs?, label? }`. Nesneler girdinin sırasıyla yazılır, her biri yeni kalıcı kimlik alır.
- **Renk nesne başınadır.** Tek nesnelik komutlarda renk girdinin kendisindedir; çok nesnelikte her nesne kendi rengini taşır. Web aracı güncel rengi her nesneye koyar; masaüstünde güncel renk yok.
- **Çıktı** `EntitiesCreated { created, ids, revision }`: kalıcı kimlikler, yuvalar, sürüm. **Plan** `EntitiesCreatePlan { entities, revision }`: yuvaları 0 olan nesneler.
- **Adımın adı** `operation`'dan gelir; yoksa belgenin kendi “Ekle”sidir.

### Denetimler ve sıraları

| Sıra | Kod | Yol |
|---|---|---|
| 1 | `no_objects` | `objects` — “Eklenecek nesne verilmedi. En az bir nesne verin.” |
| 2 | `too_few_points`, `too_few_corners`, `not_finite`, `invalid_radius` | `objects[i].geometry…`, her nesne sırayla; ileti “N. nesnenin geometrisinde …” |
| 3 | `invalid_revision`, `revision_conflict` | `expectedRevision` |
| 4 | `layer_not_found`, `not_a_layer`, `layer_locked` | `layerId`; kilitli katmanın metni araçlarınkidir |
| — | `layer_hidden` (uyarı) | `layerId`; nesneler yine yazılır |
| — | `slots_exhausted` | yalnız masaüstü; hiçbir nesne yazılmaz, sığanlar da |

- **Denetlenmeyen:** geometrik geçerlilik: elipsin oranı, doğrultunun birim uzunluğu, yay değeri sayısı. `cad.entities.edit` de denetlemiyor; araçlar böyle geometri üretmez.
- Yazma bütün ya da hiçtir: tek `add_many` / `addMany`, tek adım, açık işleme ya da gruba katılır.

### Ortak durumlar

- `fixtures/commands/v1/cad.entities.create.json`: 26 durum, 64 adım. Her tür (elips, eliptik yay, açık ve kapalı eğri, yardımcı çizgi, ışın, halka, paralel yanlar ve koridor, dikler, bölme noktaları, kapalı alan, daire, yay, çoklu çizgi, yazı, ölçü), her işlemin adı, renk, öznitelik ve etiket, grubun içindeki katman, gizli katman ve gizli grup, bütün retler ve sıraları, plan ve doğrulamanın hiçbir şey yazmaması.
- Dosyayı `scripts/fixtures/create_command_cases.py` yazar; `--check` yeniden kurup karşılaştırır. Beklenen nesne sözleşmenin kuralıyla kurulur (geometrisi, katmanı, verildiyse rengi, öznitelikleri, etiketi), bir uygulamanın çıktısından alınmaz.
- Yalnız masaüstünde olabilenler `crates/native/application/tests/create.rs`'tedir: yuvası biten belgede sığan nesne de yazılmaz; işlem içinde yürütme işleme katılır; −0 olduğu gibi yazılır; adımların adları.

### Web araçları komuttan yazar

- `PointInputTool.writeObjects` (`tools/drawTools.ts`) ve `tools/createCommand.ts`: girdide etkin katman ve güncel renk (CMD-07); komutun reddi ve uyarıları aracın iletisidir. Katalog her aracın ürün komutunu söyler (`productCommand`); envanterde görünür.
- **Değişen web davranışı** (hepsi aynı iletinin yinelenmesi ya da yanlış ikinci iletiydi):
  - Paralel çizgi kilitli katmanda kilit iletisini her yan ve eksen için bir kez (üç kez) değil, bir kez söyler; gizli katman uyarısı da bir kez.
  - Dik in ve Dik çık kilitli katmanda yalnız kilit iletisini söyler. Önceden ardından “Nokta zaten hattın üzerinde.” ya da “Dik boy sıfır olamaz.” da çıkıyordu.
  - Revizyon bulutu kilitli katmanda yalnız kilit iletisini söyler. Önceden “Bulut için alanı olan bir dikdörtgen ya da en az üç köşe gerekir.” da çıkıyordu.
  - Böl kilitli etkin katmanda bütün çizim araçlarının iletisini söyler: ““Ad” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.” Önceki metin (“…; noktalar etkin katmana konur.”) çözümü söylemiyordu (CLAUDE.md §8). Gizli etkin katmanda artık öbür araçlar gibi uyarır; noktalar yine konur.
- **Değişmeyen:** öbür iletiler ve sıraları, adımların adları, tek adım, araç sonrası durum. Yeni izler önce web'de geçti.

### Masaüstünde on bir araç (`kentos-interaction`)

- `ellipse.rs`, `spline.rs`, `construction.rs` (Yardımcı çizgi ve Işın), `parallel.rs`, `perpendicular.rs` (Dik in ve Dik çık), `donut.rs`, `revcloud.rs`, `point.rs`'te Kot noktası, `divide.rs`. Web'in adımları, istem metinleri (kelimesi kelimesine), seçenekleri, önizlemeleri, Enter, Esc ve Ctrl+Z anlamları; web'in tuhaflıkları da: eliptik yayın açıları beklenirken Enter araçtan çıkar, Ctrl+Z çizimi geri alır; yatay yardımcı çizgide Enter araçtan çıkar.
- **Oturum belleği** (`Memory`, web'in statik alanları): yardımcı çizginin açısı; paralel çizginin sol ve sağ mesafesi, ekseni ve alanı; halkanın iç ve dış çapı; bulutun biçimi ve yay boyu; Böl'ün kipi, parça sayısı ve aralığı. İzler onları başladığı gibi bırakır.
- **Önizleme** web'in çizdiğidir. Sonsuz çizgi görünümün köşegeninin 4 katı uzanır (web'in `strokeInfinite`'i), referans hattın uzantısı 2 katı. Önizlemenin yeni parçaları (`Preview`):
  - `areas`: dolgulu alan, çift-tek kuralıyla delikli (paralel çizginin koridoru, halka);
  - `labels`: nokta yanında kısa yazı (referans hattın başlangıcı “A”, kenet renginde);
  - `MarkerShape::Circle`: Böl'ün konacak noktaları, 1 px, en çok 2000 (web gibi);
  - `MarkerShape::RightAngle`: dikin ayağında 8 piksellik dik açı işareti.
- **Şerit:** yeni bir şey gerekmedi. Aileler (Yardımcı çizgi ▾ Işın, Dik in ▾ Dik çık) ve seyrek araçlar (Halka, Revizyon bulutu ▾ altında) envanterden gelir; Kot noktası Harita sekmesindedir.

### Etkileşim izleri

- Altı yeni iz, iki platformda üç varyantta geçer: `ellipse-spline`, `construction-lines`, `parallel-line`, `perpendiculars`, `donut-cloud`, `spot-divide`. Yeni çizim `tools.kcad`: bölünecek çizgi, dik referansı çizgi, daire, kilitli katmanda çoklu çizgi, `kot` katmanı.
- İz biçimi genişledi; iki oynatıcı ve README birlikte değişti: eğrinin `points`'i geçtiği noktalardır; elipsin `points`'i eksen uçları (büyük, küçük, saat yönünün tersine; eliptik yayda başlangıç ve bitiş), `center`'ı merkezidir; yardımcı çizginin ve ışının `points`'i geçtiği nokta ve doğrultusunda 1 m ötesidir.

## Sahibin kararına bırakılanlar (varsayılanla uygulandı)

1. **Genel komut mu, türe göre komutlar mı?** Varsayılan: genel `cad.entities.create` (yukarıdaki nedenler). Öbürü: `cad.ellipse.create`, `cad.spline.create`, `cad.xline.create`, `cad.ray.create`, `cad.hatch.create` ve çok nesnelikler için ayrıca bir komut.
2. **Böl'ün kilit iletisi.** Varsayılan: bütün araçların ortak metni (çözümü söyler). Öbürü: web'in eski metnini araçta koruyup komuttan önce kendi denetimini yapmak.
3. **Kot katmanı çizimde yoksa.** Varsayılan: web gibi, komutun “katman yok” iletisi. Öbürü: katmanı proje şablonundaki gibi kurup yazmak (web'le birlikte).

## Ertelenenler

- Masaüstünde güncel renk: araçlar katmanın rengiyle yazar (ADR 0032'den beri).
- Yazı, ölçü, tarama ve alan araçları; elipsin ve eğrinin tutamaçları.
- `cad.entities.create`'in sunucuda çalışması ve Python/AI sarmalayıcısı (ADR 0022'nin ertelenenleri).

## Doğrulama

- `python3 scripts/fixtures/create_command_cases.py --check` (26 durum); `cargo test -p kentos-native-application` (ortak durumlar ve `tests/create.rs`); `crates/native/interaction/tests/drawing.rs`.
- Altı iz iki platformda üç varyantta: web `pnpm e2e:interaction` (30 iz × 3 varyant), masaüstü `cargo test -p kentos-desktop traces`.
- `pnpm rust:test`, `pnpm rust:test:desktop` (clippy temiz), `pnpm typecheck`, `pnpm test` (1529), `pnpm inventory:check`.
- Görüntüler (`preview::screens`, `.run/shots/cizim-*`), koyu ve açık, 1440×900 ve 1100×650:
  - elipsin ve eğrinin önizlemesi;
  - görünümü kesen yardımcı çizgi ve açısı;
  - paralel çizginin dolgulu koridoru;
  - dik, dik açı işareti ve “A”;
  - halkalar;
  - revizyon bulutu;
  - Böl'ün konacak noktaları.
- Görüntülerde bir KentOS UI hatası bulundu ve düzeltildi. Büyük bölünmüş düğmede (Yardımcı çizgi ▾, Dik in ▾) araç çalışırken yalnız üst kısım dolgulu vurgu alıyor, ama alttaki etiket vurgu zeminin yazı rengine (beyaz) geçiyordu; açık temada kayboluyordu. Etiket artık kendi renginde kalır, web'deki gibi (`.rsplit[data-active]` yalnız `.rsplit__main`'i doldurur).
