# Devir notları

Tarih: 24 Eylül 2026. Bu notlar işi sürdürecek yapay zekâ ajanı içindir.
Önce bu dosyayı, sonra aşağıdaki belgeleri okuyun. İş ilerledikçe bu dosyayı
güncel tutun; biten maddeyi silin, yeni kararı ekleyin. Dilimlerin ayrıntılı
geçmişi ADR 0008'de ve commit iletilerindedir; burada yalnız durum, sıradaki
işler ve kurallar durur.

## 1. Önce okunacaklar

1. `CLAUDE.md`: bağlayıcı proje kuralları. Özellikle §3 (kısıtlar), §4.1
   (katmanlar), §4.8.1 (geometri çekirdeği), §8 (kod kuralları), §9.4
   (testler), §11 (teknik borç) ve §14 (“Tek hesaplama kaynağı kapısı”).
2. `docs/adr/0008-shared-core-boundary.md`: ortak çekirdeğin bütün kararları.
   Taşıma yöntemi, çağrı tablosu, JSON sınırı, geometri deposu, doğrulama,
   taşırken bulunan hatalar ve WASM boyut tablosu buradadır.
3. `docs/perf/README.md`: ölçümlerin özeti (TS tabanı, S1 önce/sonra, S6).
4. `docs/adr/0009-*.md`: dosya biçimleri (onay bekliyor, §6).
5. `DESIGN.md`: yalnız arayüze dokunulursa.

## 2. Nerede kaldık

- **Kullanıcının hedefi “öncelikle ortak çekirdeği tamamlayalım” tamamlandı** (P0–P8, S1–S6; `main`, 24 Eylül). CLAUDE.md §14: CAD hesabı `crates/shared/geometry-core` içinde bir kez yazılır, native ve wasm32 olarak derlenir; TS algoritmaları eşdeğerlik kanıtlanınca silindi.
- **Kapsam: yalnız hesap Rust'ta, arayüz TypeScript'te kalır.** Kullanıcı bunu açıkça sordu ve doğruladı; bu ayrımı koruyun.
  - Rust'ta (`crates/shared/geometry-core`, tarayıcıda WASM): geometri işlemleri (kesişim, budama, uzatma, öteleme, köşe yuvarlama, alan cebiri ve bindirme, yay/elips/eğri, ölçü yerleşimi, tarama çizgileri, üçgenleme), nesne ölçüleri, geometri deposu (seçme, kenet, pencere seçimi, etiket ve tutamaç kararları, araç önizlemeleri, çizilecek geometri, ifadelerin geometri değerleri), işlem araçlarının geometrisi (köşe numaralama, kenar ölçüleri), araçların yapı hesapları (nokta girişi, orto/kutupsal, nesne izleme), dosya biçimleri (`crates/shared/formats`), sunucunun PostGIS geometrisi (tessellate, EWKB), §23 sayısal politika.
  - TypeScript'te: bütün arayüz (DOM, paneller, pencereler, menüler, komutlar, kısayollar), araçların akışı (tıklama, istem, önizlemenin çizimi), belge modeli ve geri alma, çizim motorları (WebGL2/WebGPU), kamera ve ekran pikseli hesapları, bulut eşitleme, stil motoru, ifade dili ve SVG düzenleyicisi. Son üçünün kendi geometrisi ileride ayrı bir `style-core` dilimidir (§3); acelesi yok.
  - Cepheler (`apps/web/src/model/geom`, `apps/web/src/model/ops`, `model/geometry.ts`, `entities.ts`, `render/triangulate.ts`, `tools/constructions.ts` …) yalnız çağırır: `op('ad')` ile çağrı tablosuna, sıcak yollarda tipli girişlere. `apps/web/src/model/singleSource.test.ts` bu dosyalarda aritmetik ya da `Math.` görürse düşer.
- **Dilimler** (her biri tek commit; ayrıntı ADR 0008'in aynı adlı başlığında):

  | Dilim | Commit | Ne yapıldı |
  |---|---|---|
  | 0, P1–P8 | `7ace476` … `d8a7beb` | `model/geom`, `model/ops`, `render/triangulate` işlemlerinin hepsi Rust'ta; TS ile işlem başına 20 000 rastgele durumda aynı sonuç; donmuş çağrı fixture'ları `fixtures/geometry/v1/calls-*.json` |
  | S1a–S1c | `c6e8a4c`, `1f82a59`, `d109f25` | Geometri deposu (`geometry-core::store`, `viewport/picking.ts` ince yüz): seçme, kenet, kutular, etiketler, tutamaçlar, buda/uzat önizlemesi, hayaletler, toplamlar |
  | S1d | `b6812b0` | Bulutta önce/sonra ölçümü (`docs/perf/interaction-s1-*.md`) |
  | S2 | `0d27941` | Katman kurulurken çizilen geometri ve dolgu üçgenlemesi depodan (`PickIndex.drawn`, `fillQueue.ts`) |
  | S3a, S3b | `c63da07`, `c7445e0` | Modüller cepheye döndü, TS algoritmaları silindi; sayı alan girişler ve kazıma tamponu; depoda `extent` |
  | S4 | `d8f9c45` | İşlem araçlarının geometrisi, sayfada ve worker'da aynı `runJob` ve çalıştırmanın kendi deposu |
  | S5 | `af83ccd` | Nokta girişi, orto/kutupsal, nesne izleme, araç yapıları (`geometry-core::tools`) |
  | S3c | `65ed097` | Son TS referansları silindi (son derin koşular temiz), tek kaynak bekçisi, referanssız yerine geçen testler, kaydediciler çekirdekten |
  | S6 | `dcca64c` | CLAUDE.md güncellendi (bulut ölçümü durduruldu, §2 “Ölçüm”) |
  | Performans | `24d466e` | `CadDocument` katman dizini (`byLayer`), ızgaranın yeniden kullanımı, `JSON.stringify`'sız geometri karşılaştırması (CLAUDE.md §6.3) |
  | Kenet | `602bf5c` | Genel görünümde kesişim keneti ~7 kat hızlı, yanıtlar bit bit aynı (ADR 0008, geometri deposu) |

- **Dosya biçimleri** (`wip/formats-dxf` dalından, `33f9981` … `96f4460`): koordinat listesi (Netcad NCN, TXT, CSV) içe/dışa aktarma ve DXF içe aktarma Rust'ta (`crates/shared/formats`, ayrı ve yalnız komutla yüklenen WASM paketi `apps/web/src/io/pkg`). ADR 0009 “önerildi”, onay bekliyor. DXF dışa aktarma yok (§3).
- **WASM paketi:** 883 087 bayt, gzip 297 912 bayt. ADR 0005 taslağındaki başlangıç sınırı 300 KB gzip; ~2 KB kaldı. Çekirdeğe eklenecek bir sonraki kod bu sınırı aşar: önce kullanıcının kararı gerekir (§6 madde 8).
- **Son doğrulama** (`main`, şerit işinden sonra, 24 Eylül, bulut konteyneri): `pnpm typecheck` temiz; `pnpm test` 623 test geçti, 6 atlandı (fixture kaydedicileri); `pnpm build` başarılı; `pnpm e2e` 109 denetim geçti, düşen üçü bu konteynerde hep düşen WebGPU denetimleri (§5). Rust'a dokunulmadı; son Rust doğrulaması `602bf5c`'de: `cargo fmt --all -- --check` ve `pnpm rust:test` (cargo test ve clippy `-D warnings`) temiz, veritabanı testleri sır olmadığı için atlandı.
- **Ölçüm:**
  - S1 önce/sonra (bulut, `docs/perf/interaction-s1-*.md`): imleç başına seçme ve kenet `parsel-50k`'da ~19 ms'den 0,1–3 ms'ye, `hat-1m` budama önizlemesi 972 ms'den 18 ms'ye indi.
  - Genel görünümde kenet `602bf5c`'de ~7 kat hızlandı (`hat-1m`, WASM, Node'da mikro ölçüm: p50 11,5 → 1,6 ms, p95 16,4 → 2,4 ms). Kalan ~1,1 ms aday çizgilerin köşe ve kenar geçişidir.
  - Son bulut ölçümü (S6) makine alt ajanlarla aşırı yüklü olduğu için (4 çekirdekte yük ~8) 16 senaryonun 4'ünde durduruldu. S2–S5'in etkileşime etkisi kullanıcının makinesindeki kabul ölçümüyle görülecek (§3 madde 2).
- **Dizin düzeni değişti (24 Eylül, kullanıcı kararı; ADR 0001 “Güncelleme”):** depo monorepo oldu, çünkü ortak Rust kodu ileride wgpu masaüstü uygulamasında da kullanılacak. Tarayıcı uygulaması `apps/web/` (pnpm paketi `@kentos/web`); Rust crate'leri `crates/shared/` (geometry-core, contracts, formats: platformdan bağımsız), `crates/wasm/` (geometry-wasm, eski `kentos-wasm`; formats-wasm) ve `crates/server/` (postgres, application). `contracts`'ta ts-rs `ts` özelliğinin arkasında. Komutlar eskisi gibi kökten çalışır. Bu belgedeki ve CLAUDE.md'deki `model/…`, `tools/…` gibi adlar `apps/web/src/`'ye göredir; tip denetimi kökten `pnpm typecheck` (`apps/web`'de `tsc --noEmit`), fixture kaydedicisi `pnpm -C apps/web exec vitest run scripts/fixtures/record-calls.test.ts` ile çalışır.
- **Şerit arayüzü (24 Eylül, kullanıcının isteği):** Uygulama ayarları → Görünüm → Arayüz düzeni: Klasik (varsayılan; menüler, araç çubuğu, araç kutusu) ya da Şerit; Görünüm → Şerit arayüzü de geçer. Kullanıcının şartı: **yeni bir araç iki arayüze de kendiliğinden girer, ikisi ayrı ayrı düzenlenmez.** Bu yüzden klasik menüler de araçları artık katalogdan alır (`@tools:draw`; araçların bölümü `section`, `tools/Tool.ts` → `TOOL_SECTIONS`), şerit menü modelinden ve katalogdan türer (`app/ribbon.ts`), `app/ribbon.test.ts` her aracın ve menü komutunun iki tarafta da olduğunu denetler. Şerit uyarlamalı (paneller pencereye göre adım adım küçülür), bağlamsal Seçim sekmesi, çalışan araç noktası, daraltma (`Ctrl+F1`) ve üstte açılma, Komut ara (`Alt+Q`), hızlı erişim (sağ tıkla ekleme) taşır; ayrı parçadır (JS 27 kB, gzip 9 kB; CSS 10 kB), yalnız seçilince yüklenir. Ayrıntı CLAUDE.md §4.5, §4.7, §4.10; DESIGN.md §7.3.1.
- **Açık alt ajan yok.** Kullanıcı yenisini istemiyor (kredi); son ikisinin işi main'de (`24d466e`, `602bf5c`).

## 3. Sıradaki işler (öncelik sırasıyla)

Kullanıcı kredinin azaldığını söyledi: alt ajanı yalnız gerçekten gerekirse ve tek tek çalıştırın; işi küçük, doğrulanmış dilimlerle ilerletin.

1. **Açık kararları sorun (§6).** Özellikle madde 8 (WASM bütçesi): aşağıdaki çekirdek dilimlerinin hepsi pakete kod ekler.
2. **Kabul ölçümü kullanıcının makinesinde.** Kullanıcı `pnpm perf:interaction --label s6` çalıştırır (tabanla karşılaştırmalı, `docs/perf/interaction-baseline.json`); sonuç `docs/perf/README.md`'ye ve ADR 0008 “Uygulamada önce/sonra”ya yazılır. Bulut ölçümleri yazılım GPU'suyladır; yalnız ana iş parçacığı süreleri ve aynı makinedeki önce/sonra çifti anlamlıdır.
3. **§23.3 sağlam geometrik kararlar** (robust predicates), madde 8'den sonra. Yalnız Rust'ta; yeni bağımlılık eklemeden (Shewchuk'un uyarlamalı `orient2d` ve `incircle`'ı çekirdeğe yazılır, lisansı kamu malı). Yol:
   - `geometry-core`'a bir `predicates` modülü ve bağımsız referansa (Python kesirleri; `scripts/fixtures/geometry_call_reference.py` gibi) karşı testler;
   - kararlar tek tek değiştirilir: bindirmede yön ve sıralama (`geom/arrangement.rs`, `overlay.rs`), parça kesişimi ve çakışıklık (`geom/intersect.rs`), nokta-çokgen ve iç/dış;
   - her değişiklikte kaydediciyle yeniden kayıt (`GOLDEN_WRITE=1`, §4) ve farkın satır satır okunması; değişen her golden durum ADR 0008'e gerekçesiyle yazılır;
   - sabit toleransları (1e-9, `TOL = 1e-6`) sessizce büyütmek yasak (CLAUDE.md §23.4).
4. **Taşıma ve kopyalamada JSON maliyeti:** 10 000 nesneyi taşımak ~0,15 s (TS'te ~0,01 s). Dönüşüm depoda yapılıp sonuç paketli (`wasm/pack.ts` biçiminde) döndürülür; `modifyTools.ts`, `editTools.ts` (yapıştır) çağırır. WASM'a kod ekler (madde 8).
5. **Katmanlar paneli:** `LayersPanel` her değişiklikte bütün ağacı yeniden çiziyor (300 katmanlı bir DXF'ten sonra nesne düzenlemesi başına ~15 ms). İlk adım sayı hücrelerini yerinde yazmak (~20 satır; geri alma ve yinelemede sayıların izlendiğini ve satırların aynı kaldığını denetleyen bir e2e denetimiyle). Sanallaştırma daha büyük bir `TreeView` işidir (katmanlar, işlemler, stil yöneticisi ortak; satır yüksekliği yazı ölçeğine bağlı, klavye, odak, yeniden adlandırma, satıra kaydırma).
6. **DXF dışa aktarma** (ADR 0009 onaylanınca): uzak `wip/formats-dxf` dalındaki `0320b7f` başlangıcı (sözleşmede `DxfWriteLayer`/`DxfWriteInput`, `KENTOS` genişletilmiş verisinin okunması, `catmull_rom_beziers`) derlenmiyordu; yazıcı, `writeDxf`, pencere, komut ve testler yazılacak. Büyük dosyada içe aktarmanın ana iş parçacığındaki süresi de ölçülmedi.
7. **style-core** (uzun vade): stil motoru, ifade dili ve SVG düzenleyicisinin geometrisi aynı yöntemle (§4) taşınır. Bekçinin (`singleSource.test.ts`) listesi o zaman genişler.
8. **CLAUDE.md'nin öbür fazları** (Faz B kalanları, Faz C/D): çoğu veritabanı ister; bu bulut konteynerinde sır olmadığı için doğrulanamaz. Tipli öznitelik şeması kullanıcı kararını bekliyor (§6 madde 6).

## 4. Taşıma yöntemi ve yeni çekirdek işlevleri

- **Birebir taşıma.** JavaScript sayı anlamı `crates/shared/geometry-core/src/jsmath.rs`'tedir:
  - `js_round`, `js_sign`;
  - NaN yayan `js_min`/`js_max`;
  - V8 algoritmalı `js_hypot`;
  - `js_cmp`, `or`, `truthy`;
  - kararlı `stable_sort`.
- **Aşkın işlevler `libm`'den gelir.** `clippy.toml` std `sin/cos/tan/atan2/hypot/powi/mul_add/round/signum/min/max`'ı yasaklar.
  - V8'in `Math.sin`/`cos`'u çağrıların ~%2'sinde son bitte farklıdır.
  - Native ve WASM ise hep bit bit aynıdır.
- **Kayıt:** TS dosyası başına bir modül; `pub(crate) static OPS: &[Op]` içinde `op!("tsAdı", |a: A, b: B| gövde)`. Modül `crates/shared/geometry-core/src/api/tables.rs` içindeki `TABLES` listesine eklenir.
- **JSON:** `apps/web/src/api/json.rs`.
  - `json_struct!`, `json_tagged!`; açık `null` için `Nullable`.
  - `None` alan yazılmaz.
- **Hata ve panik:**
  - TypeScript'in istisna fırlattığı yerde `Result<_, String>` döner, JS'te istisna olur.
  - Panik yok: `unwrap`/`expect` test dışı kodda yasaktır.
- **Nesne alanları:** nesnenin kimlik, katman ve öznitelik gibi alanları `Entity.rest`'te olduğu gibi geri döner.
- **Çağrı kümesi:** `apps/web/src/wasm/calls/sets/*.ts` dosyasına adlı sınır durumları ve tohumlu rastgele çağrılar (`repeat`, `Gen`) yazılır, küme `sets.ts`'e eklenir. Yeni bir çekirdek işlevi (TS karşılığı olmayan) kümeye yazılır ve kaydediciyle dondurulur; beklenen değerler çekirdekten gelir, fark okunarak doğrulanır, bağımsız referans eklenir.
  - Üreteçler çekirdeğin hata döndürdüğü girdileri üretmemeli; test donanımı istisna yakalamaz.
- **TS'ten taşıma (style-core gibi):** S3c'de TS ↔ Rust karşılaştırması (`parity.test.ts`, kümelerin `fns` ve `ties` alanları) son referanslarla birlikte silindi; `git show c7445e0:apps/web/src/wasm/parity/parity.test.ts` ve `c7445e0:apps/web/src/wasm/parity/harness.ts` yöntemin çalışan biçimidir. Taşınacak TS'i kümenin `fns` alanına koyup testi geri getirin:
  - normal: işlem başına 200 durum; derin: `PARITY_CASES=20000`;
  - tolerans 1e-9 + 1e-14 · büyüklüktür; gerekçeli istisnalar kümede `tolerance`, eşit ölçülü sıra değişimleri `ties` ile bildirilir;
  - derin koşu temizse TS silinir, karşılaştırma yeniden kaldırılır ve tek kaynak bekçisi (`apps/web/src/model/singleSource.test.ts`) yeni cephe dosyalarını listesine alır.
- **Fark çıkarsa:** çoğu zaman TypeScript'te gizli bir kırılganlık ya da hatadır.
  - Önce hatayı yeniden üreten bir TS birim testi yazın.
  - Sonra TS ve Rust'ı aynı biçimde düzeltin ve ADR 0008'e yazın.
  - Örnekler ADR'dedir: halka izlemede ikiz parça, elipste en yakın nokta, ortak köşede en yakın kenar.
- **Fixture:**
  - `GOLDEN_WRITE=1 pnpm -C apps/web exec vitest run scripts/fixtures/record-calls.test.ts` (depo için `record-store.test.ts`, `record-store-processing.test.ts`); kaydediciler S3c'den beri yanıtı çekirdekten alır, yeniden kayıt bilinçli bir golden değişikliğidir;
  - sonra `pnpm -C apps/web exec vitest run src/wasm` ve `cargo test -p kentos-geometry-core --test calls`.
  - Kaydedici işlem başına en çok 25 rastgele durum ve 48 KB tutar.
- **Boyut:** `apps/web/src/wasm/pkg/kentos_wasm_bg.wasm` ham ve `gzip -9` boyutu ADR 0008 tablosuna yeni satır olarak yazılır.

## 5. Ortam ve çalışma kuralları

- **Kurulum:**
  - `pnpm install --frozen-lockfile`;
  - `rust-toolchain.toml`'daki Rust (wasm32 hedefiyle);
  - `cargo install wasm-bindgen-cli --version 0.2.128 --locked`.
  - `pnpm dev/test/build/e2e`, WASM paketini kaynak değiştiyse kendisi derler (`scripts/wasm/ensure.mjs`). `pnpm e2e` başsız Chrome ister.
- **Sırlar depoda yok** (`.env.local`):
  - Veritabanı testleri atlanır, `pnpm e2e:cloud` çalışmaz; veritabanı kurmaya çalışmayın.
  - `kentos` adlı veritabanına asla dokunulmaz; o başka bir uygulamanındır. KentOS CAD'in veritabanı `kentos_cad`'dir.
- **Bulut konteyneri (Claude Code on the web):** kök kullanıcıyla çalışır.
  - Chromium `/opt/pw-browsers/chromium`'dadır ve kökte `--no-sandbox` ister. `cdp.mjs` bayrak eklemez; depoya dokunmadan `exec /opt/pw-browsers/chromium --no-sandbox "$@"` diyen bir sarmalayıcıyı `CHROME_BIN` ile verin.
  - Başsız SwiftShader'da WebGPU aygıtı ilk karelerde kaybolur (“A valid external Instance reference no longer exists”). `pnpm e2e`'nin üç WebGPU denetimi bu yüzden düşer; taban commit'te de aynıdır. Öbür denetimler anlamlıdır. Bir alt ajan WebGPU'nun bu konteynerde `--use-angle=vulkan` bayrağı olmadan çalıştığını gördü; `cdp.mjs` değiştirilmedi (kullanıcının makinesindeki bayraklar bozulmasın diye denemeden değiştirmeyin).
  - `wasm-bindgen-cli` kurulu gelmez (`cargo install … --locked`, ~1,5 dk).
- **Ölçüm:** taban kullanıcının makinesinde (Intel Iris Xe GPU) alındı.
  - Karşılaştırmayı kullanıcı kendi makinesinde `pnpm perf:interaction --label s6` ile yapar.
  - Bulutta yalnız aynı makinede önce/sonra çifti anlamlıdır. SwiftShader'da `--allow-swiftshader` gerekir; yalnız ana iş parçacığı süreleri anlamlıdır, bir koşu (`--runs 1`) ~50 dk sürer.
  - Düzenek çalışma dizinindeki kaynağı sunar: ölçüm sürerken `apps/web/src/` değişirse ölçüm bozulur. Ölçülecek commit'i ayrı bir git worktree'sinde çalıştırın (`node_modules` bağı ve `apps/web/src/wasm/pkg` kopyasıyla); ana dizinde çalışmaya devam edilebilir.
- **Ağır işler:** kullanıcının makinesinde cargo, tam vitest, e2e ve ölçüm aynı anda çalışmaz (makine bir kez dondu). Bulut konteynerinde paralel çalıştırılabilir (kullanıcı izin verdi), ama ölçüm sürerken başka ağır iş çalışmaz.
- **Alt ajan:** kullanıcı kredinin azaldığını söyledi; yalnız gerçekten gerekirse ve tek tek. Alt ajan ayrı worktree'de çalışır, main'e push etmez; sonucunu siz inceleyip sınar ve alırsınız.
- **Commit ve push (her iş sonunda, kullanıcının kuralı):** dilim başına bir commit, mevcut biçimde İngilizce mesajla (ör. “Shared core, P8: …”). `pnpm typecheck`, `pnpm test`, clippy ve gerekiyorsa `pnpm e2e` geçince:
  1. çalışma dalı push edilir;
  2. kendi deponun (`ilhanalacahan/kentos-cad`, `origin`) `main`'i o dala ileri sarılır (`git push origin HEAD:main`; birleştirme commit'i yok);
  3. asıl depoya (`mcihad/kentos-cad`, uzak adı `upstream`) PR: bu oturumdan açılamıyor (aynı adlı iki depo bir oturuma bağlanmaz; Claude GitHub uygulamasının `mcihad`'e erişimi yok), kullanıcıya karşılaştırma bağlantısı verilir: https://github.com/mcihad/kentos-cad/compare/main...ilhanalacahan:kentos-cad:main?expand=1 . Açık bir PR varsa `main`'e her push onu kendiliğinden günceller.
  - Asıl depo ilerlediyse önce `git fetch upstream main` ve ileri sarma; iki taraf ayrıştıysa birleştirmeden önce kullanıcıya sorulur.
- **Test ve doğrulama:**
  - Hata düzeltmesi önce hatayı yeniden üreten testle başlar.
  - Her değişiklikte `pnpm typecheck` temiz, `pnpm test` geçer.
  - Arayüze dokunan değişiklik tarayıcıda denenir.
- **Sorulmadan yapılmayanlar:**
  - Çalışma zamanı bağımlılığı eklemek (kullanıcıya sorulur).
  - Global git ayarını değiştirmek.
  - CLAUDE.md §0 ve §13 sonrasını değiştirmek: bunlar kullanıcının metnidir; yalnız doğrulanmış durum notu eklenir. §1–12 gerçeğe uygun tutulur.

## 6. Kullanıcıya sorulacak açık kararlar

1. Bulut projesini silme yalnız yönetici ve sahipte mi kalsın, proje yöneticisi de silebilsin mi?
2. Olay günlüğünü 7 gün tutmak uygun mu? (Veritabanı bir saatten kısasını reddeder.)
3. Silinen projeler bir süre sonra kalıcı silinsin mi? Yönetici arayüzden geri alabilsin mi?
4. Komut günlüğü (idempotency) ve denetim tablosu için saklama süresi gerekiyor mu?
5. ADR 0005 (performans hedefleri) hâlâ taslak; onay bekliyor.
6. Tipli öznitelik alanlarının tasarım onayı. Önerilen: katman başına şema; türler metin, tam sayı, ondalık, mantıksal, tarih ve sabit liste.
7. Gerçek OpenID denemesi için kurumun OpenID sunucusu bilgileri (issuer, client id).
8. WASM paketi ADR 0005 taslağındaki 300 KB gzip başlangıç sınırına dayandı (S3a'da 267 KB, S5 ile 283 KB, S4 ile 297 KB, S3b'de kullanılmayan girişler silinince 298 KB; ~2 KB kaldı, bir sonraki dilim aşar). Seçenekler: işlev adları bölümünü üretim paketinden atmak (S1 sonunda −18 KB gzip; bedeli tuzakta yığın izinde ad yerine numara), sınırı değiştirmek ya da ağır işlemleri ayrı pakete bölmek.
9. Dosya biçimleri (ADR 0009) main'e alındı; ADR'nin onayı bekliyor.
10. İçe aktarmada “Bu koordinatlar hangi sistemde?” sorusu projenin sistemi seçili açılıyor; içe aktarılabilen tek seçenek o olduğu için kullanıcı hiçbir şeye dokunmadan içe aktarabiliyor. Seçim yapılmadan “İçe aktar” düğmesi kapalı mı kalsın (açık onay)?
11. DXF ACI 251–254 gri tonları AutoCAD 2000 ve sonrasının tablosuna (ezdxf ile aynı: 80, 105, 130, 190) göre düzeltildi; bir AutoCAD çizimiyle doğrulanması iyi olur.
12. Bilgi için (kullanıcı aksini isterse değişir; ayrıntı ADR 0008 S4, S5): hedef katmandaki adsız nokta artık numaralı sayılmaz (önce bir numarayı yutuyordu); köşesiz yolun `$y`/`$x`'i boştur (önce bütün ifadeyi boşaltan hata veriyordu); yazılan değerin tek IEEE işlemiyle yeniden ifadesi (derece → radyan, kâğıt mm → metre, `hedef − temel`) ve dikdörtgen dizinin ötelemeleri TS'te kaldı, çünkü iki dilde bit bit aynıdır ve önizlemede her karede JSON'a değmez.

## 7. Devralan ajan için ilk adımlar

1. Bu dosyayı ve §1'deki belgeleri okuyun. CLAUDE.md §0 ve §13 sonrası kullanıcının metnidir: yalnız doğrulanmış durum notu eklenir.
2. Ortamı kurun (§5): `pnpm install --frozen-lockfile`, `cargo install wasm-bindgen-cli --version 0.2.128 --locked`, bulutta Chromium sarmalayıcısı (`CHROME_BIN`).
3. `main`'i doğrulayın: `pnpm typecheck`, `pnpm test`, `pnpm rust:test`, `pnpm e2e`. Beklenen sonuçlar §2 “Son doğrulama”dadır; bulutta üç WebGPU denetimi bilinen biçimde düşer.
4. Kullanıcıya §6'daki açık kararları sorun; özellikle 8 (WASM bütçesi) ve 9 (ADR 0009), çünkü §3'teki işlerin çoğu bunlara bağlı.
5. §3'ten sıradaki işi alın. Dilim başına bir commit, İngilizce ileti (“Shared core, …” ya da “File formats, …”), sonunda oturumun atıf satırları; `tsc`, `pnpm test`, Rust'a dokunulduysa `pnpm rust:test` ve arayüze ya da çekirdeğe dokunulduysa `pnpm e2e` geçince `main`'e ve oturum dalına push edilir.
6. Yeni bir çekirdek işlevi: önce Rust'ta işlev ve birim testi, sonra çağrı tablosu (`op!`), çağrı kümesi ve donmuş fixture (§4), sonra TS cephesi (`op<Sig>('ad')`), en son çağıranlar. Cephede aritmetik yazmayın; bekçi test düşer. WASM boyutunu ADR 0008 tablosuna yazın.
7. İş bitince bu dosyayı güncelleyin: biten maddeyi silin, yeni kararı ekleyin.
