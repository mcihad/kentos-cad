# Devir notları

> **25 Eylül 2026 notu (TODOS.md `BASE-01`):** Bu dosya 24 Eylül'ün devir notudur. Güncel yol haritası [TODOS.md](../TODOS.md), güncel kurallar [CLAUDE.md](../CLAUDE.md)'dir. Kapsam kararları [ADR 0010](adr/0010-platform-boundaries.md) (web/masaüstü sınırı), [ADR 0011](adr/0011-kcad-binary-snapshot.md) (binary `.kcad`) ve [ADR 0012](adr/0012-server-scope-project-cloud.md)'dedir (proje bulutu, Martin kapsam dışı). Bu belgenin bölümlerinin durumu:
>
> - **Geçerli:**
>   - §4: taşıma yöntemi, fixture ve kaydedici kuralları.
>   - §5: ortam notları.
>   - §6: açık kararlar; hâlâ yanıt bekliyor.
> - **Yerini aldı:** §3 “Sıradaki işler” sırasının yerine TODOS.md §22'deki fazlar geçti (F0–F10). Kalan işler şuralarda izlenir:
>   - kabul ölçümü: `PERF-01..04`;
>   - DXF'in gerçek programlarda denenmesi: `FMT-02`;
>   - kalan sağlam karar dilimleri: `NUM-02`.
> - **Değişti:** kullanıcı söylemeden alt ajan ya da workflow çalıştırılmaz. Derleme ve testler hiçbir zaman aynı anda çalışmaz (kullanıcı, 25 Eylül). Bu, §2 “Alt ajanlar”, §5 “Ağır işler” ve “Alt ajan” maddelerinin yerine geçer.

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
4. `docs/adr/0009-*.md`: dosya biçimleri (kabul edildi, 24 Eylül).
5. `DESIGN.md`: yalnız arayüze dokunulursa.

## 2. Nerede kaldık

- **Kullanıcının hedefi “öncelikle ortak çekirdeği tamamlayalım” tamamlandı** (P0–P8, S1–S6; `main`, 24 Eylül). CLAUDE.md §14: CAD hesabı `crates/shared/geometry-core` içinde bir kez yazılır, native ve wasm32 olarak derlenir; TS algoritmaları eşdeğerlik kanıtlanınca silindi.
- **Kapsam: yalnız hesap Rust'ta, arayüz TypeScript'te kalır.** Kullanıcı bunu açıkça sordu ve doğruladı; bu ayrımı koruyun.
  - Rust'ta (`crates/shared/geometry-core`, `style-core` ve `svg-core`, tarayıcıda WASM): SVG düzenleyicisinin geometrisi (yol işlemleri, düğümler, kenet, hizalama ve diziler, bitmap izleme, SVG okuma ve yazma; `crates/shared/svg-core`, kendi paketi düzenleyiciyle yüklenir), ifade dili (derleme, toplu değerlendirme, JavaScript'in sayı ve metin anlamı, Türkçe sıralama; `crates/shared/style-core`), stil derleyicisi (sembollerin yerleşimi ve derlenmesi, işleyiciler, katmanın GPU toplulukları tek çağrıda; `style-core/src/style`), geometri işlemleri (kesişim, budama, uzatma, öteleme, köşe yuvarlama, alan cebiri ve bindirme, yay/elips/eğri, ölçü yerleşimi, tarama çizgileri, üçgenleme), nesne ölçüleri, geometri deposu (seçme, kenet, pencere seçimi, etiket ve tutamaç kararları, araç önizlemeleri, çizilecek geometri, ifadelerin geometri değerleri), işlem araçlarının geometrisi (köşe numaralama, kenar ölçüleri), araçların yapı hesapları (nokta girişi, orto/kutupsal, nesne izleme), dosya biçimleri (`crates/shared/formats`), sunucunun PostGIS geometrisi (tessellate, EWKB), §23 sayısal politika.
  - TypeScript'te: bütün arayüz (DOM, paneller, pencereler, menüler, komutlar, kısayollar), araçların akışı (tıklama, istem, önizlemenin çizimi), belge modeli ve geri alma, çizim motorları (WebGL2/WebGPU), kamera ve ekran pikseli hesapları, bulut eşitleme, stil motorunun gösterim yarısı (tema renkleri, atlas görüntüleri, GPU'ya yükleme; `render/styledBatches.ts`), stil pencereleri ve SVG düzenleyicisinin arayüzü (tuval, yakınlaştırma, araçların akışı; geometrisi `svg-core`'da).
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
  | Katmanlar paneli | `a3c7c5d`, `1fc00f4` | Düzenlemede sayılar ve katman durumu yerinde (panel p50 20,6 → 0,1 ms/düzenleme); biçim değişikliğinde görev başına tek kurulum (300 katmanlı içe aktarma 3,4 s → 28 ms) |
  | İfade dili (style-core Y1) | 24 Eylül | `crates/shared/style-core`: derleme, toplu değerlendirme (`exprEvaluate`: okunanların tablosu gider, sütun döner), JavaScript'in `String`/`toPrecision`/`toFixed`'i birebir, UTF-16 metin, `tr-TR` harf kuralları, sabit Türkçe sıralama tablosu. TS ile 20 000 rastgele kaynakta beş sonuç biçiminde aynı; TS silindi, dondurulmuş yanıtlar `fixtures/expression/v1/cases.json` (ADR 0008 “İfade dili”) |
  | Stil derleyicisi (style-core Y2 + Y3) | 24 Eylül | Yerleşim, derleme, işleyiciler ve toplama Rust'ta; katman geometri deposunun yanında tek çağrıda kurulur (`GeometryStore.buildStyled`), sayfa renkleri, atlas görüntülerini ve yüklemeyi yapar. TS ile sistem kitaplığının 695 sembolünde, 20 000 rastgele sembolde ve 20 000 rastgele katmanda (235 787 995 float32 sayı) aynı; TS silindi, dondurulmuş yanıtlar `fixtures/style/v1/cases.json`. Katman kurma 2–9 kat hızlı (ADR 0008 “Stil derleyicisi”) |
  | SVG düzenleyicisi (style-core Y4) | 24 Eylül | Yol verisi, Bézier ve uydurma, yol cebri, çizgi dış hattı, yol ve düğüm işlemleri, kenet, hizalama ve diziler, bitmap izleme, SVG okuma ve yazma `crates/shared/svg-core`'da; kendi paketi düzenleyici açılınca yüklenir. TS ile (TS libm'in sin, cos, tan, atan2 ve acos'uyla koşuldu) işlem başına 20 000 rastgele durumda bit bit ve alan sırasıyla aynı; TS silindi, dondurulmuş yanıtlar `fixtures/svg/v1/cases.json`. Taşırken bulunan: `[constructor]` seçicisi her öğeyi seçiyordu (ADR 0008 “SVG düzenleyicisi”) |
  | Taşı/kopyala/yapıştır | `e81a616` | Dönüşüm geometri deposunda (`Store::transform_packed`), sonuç paketli (`Packer` → `unpackEntities` → `transformedFrom`); JSON yok. 10 000 karışık TM nesnesinde dönüşüm p50 294 → 18 ms, komut ~310 → 35–60 ms |

- **Dosya biçimleri** (`wip/formats-dxf` dalından, `33f9981` … `96f4460`): koordinat listesi (Netcad NCN, TXT, CSV) içe/dışa aktarma ve DXF içe aktarma Rust'ta (`crates/shared/formats`, ayrı ve yalnız komutla yüklenen WASM paketi `apps/web/src/io/pkg`). ADR 0009 kabul edildi (24 Eylül). **DXF dışa aktarma** da Rust'ta (`51a13a7`, `9f1c383`, `717f2f2`; alt ajan, ADR 0009 “DXF yazma”): AutoCAD 2007 ASCII DXF (AC1021; UTF-8, metre); KentOS'a aynı nesneler olarak, koordinatlar bit bit döner. Ölçüler de (24 Eylül) gerçek DXF ölçüsüdür: DIMENSION, kendi anonim bloğu (KentOS'un çizdiği, değer MTEXT), çekirdeğin yerleşiminden tür ve tanım noktaları, ölçü başına stil boyları; KentOS'a ölçü olarak döner, başka programda değiştirilmişse bloğundan patlatılır (ADR 0009 “Ölçü”). DXF'in tutamadığı etiket, öznitelik, sembol ve tema renkleri `KENTOS` genişletilmiş verisiyle (XDATA) gider. Biçim paketi (yalnız içe/dışa aktarmada yüklenir) 493 639 → 663 791 bayt, gzip -9 187 224 → 247 338 (~3 KB'ı `ring_contains`'in artık kullandığı sağlam yüklemden); gerçek ölçüyle 664 136 → 669 595, gzip -9 247 742 → 250 028.
- **WASM paketi:** 1 128 063 bayt, gzip 394 938 bayt (stil derleyicisiyle; ifade dilinden sonra 1 008 667 / 349 550). Başlangıç sınırı kullanıcı kararlarıyla 300'den 350'ye, sonra 400 KB gzip'e yükseltildi, sonra kaldırıldı (24 Eylül: “WASM boyutu önemli değil, artabilir”; ADR 0005 “Değişiklikler”); işlev adları pakette kalır. SVG düzenleyicisinin paketi ayrıdır (861 318 bayt, gzip 315 900) ve düzenleyici açılınca yüklenir (CLAUDE.md §20: ilk yük için, boyut için değil). Her çekirdek diliminden sonra boyut yine ölçülüp ADR 0008'e yazılır; bir sınır değildir.
- **Son doğrulama** (`main`, SVG düzenleyicisinin çekirdeği, 24 Eylül, bulut konteyneri): `pnpm typecheck` temiz; `pnpm test` 723 test geçti, 13 atlandı (fixture kaydedicileri, istekle çalışan ölçümler); `pnpm build` başarılı; `pnpm rust:test` temiz (334 test, clippy `-D warnings`; veritabanı testleri sır olmadığı için atlandı); `cargo fmt --all -- --check` temiz; SVG çekirdeğinin derin koşusu (106 giriş × 20 000 durum) TS'le bit bit aynı, alan sırası 2 000 durumda JSON metniyle aynı (yol düğümleri ve alt yollar dışında, ADR 0008 “Bilinçli farklar”); `pnpm e2e` 136 denetim geçti (düzenleyicinin tıklama düzeltmesinin iki denetimi dahil), düşen üçü bu konteynerde düşen WebGPU denetimleri (§5; aynı piksel sayılarıyla). Düzenleyicinin ekran görüntüleri (yapıştırılan çizim, Yol menüsü ve birleşim, düğüm aracı, bitmap izleme, kaynak, PNG dışa aktarma, açık temada sistem piktogramı) konsol hatasız. Önceki doğrulamalar: stil derleyicisi 719 test, 324 Rust testi; ifade dili 725 test, 310 Rust testi; ikisinde de aynı e2e sonucu, stil derleyicisinde pafta, MPYY ve temel sembol katalogları ile stil yöneticisinin ekran görüntüleri eski TS ile piksel piksel aynı. DXF ölçüsündeki doğrulamada alt ajan ayrıca örnek bir dosyayı ezdxf 1.4 ile denetledi (hata yok); ölçülü dosya da hatasız ve düzeltmesiz okunur, ezdxf'in yeniden çizdiği değerler KentOS'unkiyle aynıdır. Tam takımda `ellipse.test.ts`'in yoğun örnekleme denetimi makine yüklüyken bir kez 5 sn sınırını aştı; kendi süresi 30 sn yapıldı (`67c663d`).
- **Ölçüm:**
  - S1 önce/sonra (bulut, `docs/perf/interaction-s1-*.md`): imleç başına seçme ve kenet `parsel-50k`'da ~19 ms'den 0,1–3 ms'ye, `hat-1m` budama önizlemesi 972 ms'den 18 ms'ye indi.
  - Genel görünümde kenet `602bf5c`'de ~7 kat hızlandı (`hat-1m`, WASM, Node'da mikro ölçüm: p50 11,5 → 1,6 ms, p95 16,4 → 2,4 ms). Kalan ~1,1 ms aday çizgilerin köşe ve kenar geçişidir.
  - Son bulut ölçümü (S6) makine alt ajanlarla aşırı yüklü olduğu için (4 çekirdekte yük ~8) 16 senaryonun 4'ünde durduruldu. S2–S5'in etkileşime etkisi kullanıcının makinesindeki kabul ölçümüyle görülecek (§3 madde 2).
- **Dizin düzeni değişti (24 Eylül, kullanıcı kararı; ADR 0001 “Güncelleme”):** depo monorepo oldu, çünkü ortak Rust kodu ileride wgpu masaüstü uygulamasında da kullanılacak. Tarayıcı uygulaması `apps/web/` (pnpm paketi `@kentos/web`); Rust crate'leri `crates/shared/` (geometry-core, contracts, formats: platformdan bağımsız), `crates/wasm/` (geometry-wasm, eski `kentos-wasm`; formats-wasm) ve `crates/server/` (postgres, application). `contracts`'ta ts-rs `ts` özelliğinin arkasında. Komutlar eskisi gibi kökten çalışır. Bu belgedeki ve CLAUDE.md'deki `model/…`, `tools/…` gibi adlar `apps/web/src/`'ye göredir; tip denetimi kökten `pnpm typecheck` (`apps/web`'de `tsc --noEmit`), fixture kaydedicisi `pnpm -C apps/web exec vitest run scripts/fixtures/record-calls.test.ts` ile çalışır.
- **Şerit arayüzü (24 Eylül, kullanıcının isteği):** Uygulama ayarları → Görünüm → Arayüz düzeni: Klasik (varsayılan; menüler, araç çubuğu, araç kutusu) ya da Şerit; Görünüm → Şerit arayüzü de geçer. Kullanıcının şartı: **yeni bir araç iki arayüze de kendiliğinden girer, ikisi ayrı ayrı düzenlenmez.** Bu yüzden klasik menüler de araçları artık katalogdan alır (`@tools:draw`; araçların bölümü `section`, `tools/Tool.ts` → `TOOL_SECTIONS`), şerit menü modelinden ve katalogdan türer (`app/ribbon.ts`), `app/ribbon.test.ts` her aracın ve menü komutunun iki tarafta da olduğunu denetler. Şerit uyarlamalı (paneller pencereye göre adım adım küçülür), bağlamsal Seçim sekmesi, çalışan araç noktası, daraltma (`Ctrl+F1`) ve üstte açılma, Komut ara (`Alt+Q`), hızlı erişim (sağ tıkla ekleme) taşır; çizim alanına hafif bir gölge düşürür (`--shadow-bar`, kullanıcının isteği; yan paneller gölgesiz; DESIGN.md §5.4'ün tek istisnası); ayrı parçadır (JS 27 kB, gzip 9 kB; CSS 10 kB), yalnız seçilince yüklenir. Ayrıntı CLAUDE.md §4.5, §4.7, §4.10; DESIGN.md §7.3.1.
- **Onay penceresi (24 Eylül, kullanıcının isteği; `455df4e`):** düzenleyiciler kapanırken soruyu durum satırına yazıyordu, hiç değiştirilmemiş yeni çizim/sembol/model de “kaydedilmemiş” sayılıyordu, model tasarımcısında ikinci × kaydetmeden kapatıyordu. Artık uygulamada soru sormanın tek yolu `ui/widgets/confirm.ts`'tir (`confirmDialog`, `askUnsaved`, `askRemove`; soranın üstünde açılır, Esc/×/arka plan Vazgeç'tir); “değişti mi” kaydedilen ya da açılan hâle göre bakılır. Kurallar DESIGN.md §7.9.1 ve CLAUDE.md §4.10'da; yeni bir soru bunlarla yazılır.
- **Alt ajanlar (24 Eylül):** kullanıcı en çok üç alt ajanla iş dağıtımına izin verdi ve bunlar bitince bir daha alt ajan çalıştırılmamasını istedi. Üçü ayrı worktree'lerde çalıştı; işleri incelendi, sınandı ve alındı: taşıma/kopyalama/yapıştırmada JSON'suz dönüşüm (`e81a616`, §3 madde 4), Katmanlar panelinde yerinde güncelleme (`a3c7c5d`, `1fc00f4`, madde 5), DXF dışa aktarma (`51a13a7`, `9f1c383`, `717f2f2`, madde 6). Bu oturumda yeni alt ajan çalıştırılmaz.

## 3. Sıradaki işler (öncelik sırasıyla)

Kullanıcı kredinin azaldığını söyledi: alt ajanı yalnız gerçekten gerekirse ve tek tek çalıştırın; işi küçük, doğrulanmış dilimlerle ilerletin.

1. **Açık kararlar (§6)** çekirdek işlerini artık engellemiyor: WASM bütçesi ve ADR 0009 kararlaştırıldı (24 Eylül). Kalanlar bulut, veri modeli ve içe aktarma kurallarıdır; o işe gelince sorun.
2. **Kabul ölçümü kullanıcının makinesinde** (25 Eylül'de alındı: `docs/perf/interaction-s6.md`; taban Chrome 153 ile alındığından karşılaştırma resmen geçersiz, bkz. `docs/perf/README.md`). Kullanıcı `pnpm perf:interaction --label s6` çalıştırır (tabanla karşılaştırmalı, `docs/perf/interaction-baseline.json`); sonuç `docs/perf/README.md`'ye ve ADR 0008 “Uygulamada önce/sonra”ya yazılır. Bulut ölçümleri yazılım GPU'suyladır; yalnız ana iş parçacığı süreleri ve aynı makinedeki önce/sonra çifti anlamlıdır.
3. **§23.3 sağlam geometrik kararlar** (robust predicates; kullanıcı 24 Eylül'de başlattı). Yalnız Rust'ta, yeni bağımlılık eklemeden. Yapılanlar (ADR 0008 “Sağlam kararlar”): R1 `predicates::orient2d` (Shewchuk'un uyarlamalı yüklemi) ve bağımsız referans (`geometry_call_reference.py`, 85 kesin işaret), donmuş çağrılar `calls-r1-predicates.json` (`6ef7e39`); R2a nokta-halka (`95ec850`); R2b sarım sayıları, açı toplamı ve bindirmenin ışın dizini (`83b11a4`); R3 halka izlemede köşe çevresindeki sıra (düz parçalar kesin, yaylarda teğet ve eğrilik; `641393e`); R4 kesişim parametreleri (`line_line`, `seg_seg`: üç çapraz çarpım `cross_accurate` ile 2⁻⁴¹ içinde; 1e-9 bandı ve paralellik sınırı korundu; 24 Eylül). R1–R3'te hiçbir donmuş durum değişmedi; R4'te yardımcı çizgi kenarlı 1 çağrı ve 45 depo durumu (96 sayı, en çok 1,95e-10 m) tam kesre yaklaştı ve yalnız onlar yeniden yazıldı. Her dilimin testi eski kuralın yanıldığı noktaları içerir. Kalanlar, her biri ayrı dilim:
   - **Yay kesişimleri** (`seg_arc`, `circle_circle`, `line_circle_params`): karekök ve toleranslı karar (diskriminant `1e-12·r²`, `r1 + r2 ± 1e-9`); kesin karar cebirsel sayı karşılaştırması ister (kareyi alıp kesin açılımla işaret), önce hangi kararın gerçekten yanıldığı bir testle gösterilmeli.
   - **Ortak sınır kararları** (bindirmede `TOL = 1e-6` ile köşe birleştirme, örtüşen parçaların birleşmesi): tolerans CAD anlamıdır, kesin yüklem yalnız bandın dışındaki kararı verebilir.
   - **incircle** (Shewchuk): TIN/eşyükselti gelince (Delaunay); şimdi kullanan yok, eklenmedi.
   - Kurallar: her değişiklikte kaydediciyle yeniden kayıt ve farkın satır satır okunması (§4); sabit toleransları (1e-9, `TOL = 1e-6`) büyütmek yasak (CLAUDE.md §23.4). Kaydedici bütün durumları yeniden yazar ve donmuş dosyalardaki TypeScript'ten kalan son bitleri (V8'in `Math.sin` gibi işlevleri) libm'inkiyle değiştirir: kaydediciyi değişiklikten önce ve sonra çalıştırıp iki çıktıyı karşılaştırın, yalnız değişen durumları donmuş dosyaya işleyin (R4 böyle yapıldı).
4. **Taşıma ve kopyalamada geri alma adımı yapıldı:** dönüşüm depoda yapılır, sonuç paketli döner (`e81a616`, alt ajan; ADR 0008 “Taşı, kopyala ve yapıştır depodan”): 10 000 karışık TM nesnesinde dönüşüm p50 294 → 18 ms. Belgenin adımı artık tek değişiklik (`updateMany`, `addMany`, çok kimlikli `remove`; 24 Eylül): nesne başına işlem ve geri alma aynı, olaylar bir kez. Tarayıcıda bütün dinleyicilerle 10 000 nesneyi taşımanın belge adımı 145 → 14 ms; Node'da komut p50 taşı 35, kopya 28, yapıştır 26 ms. Kullananlar: taşı, kopyala, döndür, ölçekle, aynala, diziler, hizala, yapıştır, esnet, birleştir, patlat, işlem araçları, seçime sembol/katman/renk. Kalan: 10 000 nesnenin sonraki karesi (katmanın yeniden kurulması) bu konteynerde yazılım GPU'suyla ~800 ms; kullanıcının makinesinde ölçülmeli.
5. **Katmanlar paneli, sanallaştırma yapıldı:** sayıların yerinde yazılması (`a3c7c5d`, `1fc00f4`, alt ajan) ve `TreeView` sanallaştırması (24 Eylül; katmanlar, işlemler, stil yöneticisi ortak; CLAUDE.md §4.10, §6.3): yalnız kaydırma penceresindeki satırlar ve iki yanında 40'ar satır kurulur, görünen satırlar veri, iki boşluk öğesi gerisi; satır boyu gizli örnek satırdan (`--row-h`, yazı ölçeğiyle); `focus`/`rowOf` önce görünüme getirir; `aria-setsize`/`aria-posinset`; `releaseRow` Katmanlar panelinin satır kaydını kurulu satırlarla sınırlar; satırın ipucu `renderRow`'un dönüşüyle satır gidince bırakılır. 300 katmanlı grupta 317 yerine 52 satır; yeniden adlandırma 23 + 41 ms (kurulum + stil/yerleşim) → 7 + 8 ms, grubu açıp kapama 16 + 34 → 5 + 8 ms (bulut). Kalan: biçim değişikliğinde satırları kimliğe göre yeniden kullanmak (pencere ~50 satır olduğu için küçük kazanç); öznitelik tablosu ve koordinat listesi geldiğinde aynı düzenek.
6. **DXF dışa aktarma yapıldı** (ADR 0009 “DXF yazma”). Ölçüler gerçek DXF ölçüsü (24 Eylül, ADR 0009 “Ölçü”): kendi anonim bloğu KentOS'un çizimini taşır (değer MTEXT; KentOS'ta yazı tek satırlı olduğu için öbür yazılar TEXT kalır), tür ve tanım noktaları çekirdeğin yerleşiminden, ölçü başına stil boyları (DSTYLE); KentOS'a ölçü olarak döner, başka programda değiştirilmişse bloğundan patlatılır. Büyük çizim ölçüldü (100 000 nesne, 34,5 MB; bulut): dışa aktarmada ana iş parçacığı kopya için ~0,3 s bekler (toplam ~2,2 s); içe aktarmada worker ~0,8 s okur, sayfa JSON'u ~0,87 s'de ayrıştırır, ~0,19 s'de denetleyip ekler, ilk kare ~1,1 s. Kalanlar:
   - dosyanın gerçek AutoCAD, Netcad, BricsCAD ve QGIS'te açılması: yalnız ezdxf ile denetlendi, asıl risk bu (ölçülerde özellikle: türler, bloklar ve DSTYLE boyları);
   - içe aktarmada sayfanın JSON ayrıştırması büyükse (kullanıcının makinesinde ölçülmeli) ayrıştırma worker'a alınıp nesneler yapılandırılmış kopyayla ya da paketli sayılarla geçirilebilir;
   - duman testi boş bir “DXF deneme” katmanı bırakır, çünkü katman silme yok (sonraki denetimleri etkilemez).
7. **style-core:** kullanıcı 24 Eylül'de başlattı (“5 maddeyi de yap”), dilim dilim aynı yöntemle (§4).
   - **Y1 ifade dili yapıldı** (ADR 0008 “İfade dili”): `crates/shared/style-core`; bekçi `model/expression`'ı da denetler.
   - **Y2 + Y3 stil derleyicisi yapıldı** (ADR 0008 “Stil derleyicisi”): yerleştirme geometrisi (işaret yerleri, gruplar, dalgalar, iç nokta), sembol × geometri → çizim ilkelleri, işleyicilerin çözümü ve GPU toplulukları `style-core/src/style`'da; katman tek çağrıda kurulur. Bekçi `style/compile.ts`, `style/geometry.ts`, `style/primitives.ts` ve `render/styledLayer.ts`'i de denetler. Başlangıç sınırı için karar alındı (önce 400 KB, sonra sınır kaldırıldı; §6 madde 14).
   - **Y4 SVG düzenleyicisi yapıldı** (ADR 0008 “SVG düzenleyicisi”): `crates/shared/svg-core`, kendi WASM paketi (`crates/wasm/svg-wasm` → `apps/web/src/style/svg/pkg`, düzenleyici açılınca `initSvgCore`); bekçi `style/svg`'yi de denetler. Kalan: düzenleyicide şekiller JSON'la geçer; 300 ayrıntılı şekillik bir çizimde taşıma 10 ms, öğelerin kurulması 13 ms sürüyor (eski TS 1 ve 3 ms). Sistem kitaplığının çizimleri küçük olduğu için şimdilik yeterli; büyük çizimler sıklaşırsa şekiller paketli sayılarla geçebilir ya da değişmeyen şekillerin öğeleri saklanabilir (önce düğüm aracının `subs`'u yerinde değiştirmesi kalkmalı).
   - **Düzenleyicinin tıklamaları düzeltildi** (Y4 sınanırken bulundu, portla ilgisiz; STYLE.md “Araçlar”): tuvalde çift tık hiç çalışmıyordu (tuval imleci yakalıyor ve basışta yeniden çiziyor, basılan öğe bırakırken gitmiş olduğu için tarayıcı click/dblclick göndermiyor): kırık çizgi çift tıkla bitmiyor, yola çift tık düğüm düzenlemeyi açmıyor, düğüme çift tık köşe/yumuşak yapmıyor, parçaya çift tık düğüm eklemiyordu. Çift tığı artık tuval kendisi sayar (iki basış 450 ms ve 4 piksel içinde; ana çizimin seçim aracı gibi). Hareketsiz tık da seçimi ızgaraya kenetleyip taşıyabiliyordu (1600×900 pencerede 1,6 birim): taşıma, boyutlandırma, döndürme ve kol sürükleme artık imleç 3 piksel yürüyünce başlar (düğüm sürükleme zaten böyleydi). Duman testi ikisini de denetler.
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
- **JSON:** `crates/shared/geometry-core/src/api/json.rs`.
  - `json_struct!`, `json_tagged!`; açık `null` için `Nullable`.
  - `None` alan yazılmaz.
- **Hata ve panik:**
  - TypeScript'in istisna fırlattığı yerde `Result<_, String>` döner, JS'te istisna olur.
  - Panik yok: `unwrap`/`expect` test dışı kodda yasaktır.
- **Nesne alanları:** nesnenin kimlik, katman ve öznitelik gibi alanları `Entity.rest`'te olduğu gibi geri döner.
- **Çağrı kümesi:** `apps/web/src/wasm/calls/sets/*.ts` dosyasına adlı sınır durumları ve tohumlu rastgele çağrılar (`repeat`, `Gen`) yazılır, küme `sets.ts`'e eklenir. Yeni bir çekirdek işlevi (TS karşılığı olmayan) kümeye yazılır ve kaydediciyle dondurulur; beklenen değerler çekirdekten gelir, fark okunarak doğrulanır, bağımsız referans eklenir.
  - Üreteçler çekirdeğin hata döndürdüğü girdileri üretmemeli; test donanımı istisna yakalamaz.
- **TS'ten taşıma (style-core gibi):** Y4'te (`svg-core`) karşılaştırma TS'i çekirdeğin libm işlevleriyle koştu: paket geçici olarak `libmSin`, `libmCos`, `libmTan`, `libmAtan2`, `libmAcos` dışa açtı, test `Math`'in işlevlerini onlarla değiştirdi; böylece tolerans sıfır (20 000 durum). Alan sırası da bütün girişlerde JSON metniyle karşılaştırıldı (2 000 durum): yalnız yol düğümleri ve alt yollar ayrıldı, çekirdek onları tek sırayla yazar (ADR 0008 “Bilinçli farklar”). `sameResult` nesnelerin anahtarlarını küme olarak karşılaştırır: sıra önemliyse metni de karşılaştırın. V8'in atan2'si de seyrek ayrılır (`atan2(1, −1,5e-19)`). Kimlik üreten işlemlerde yeni kimlikler görünüş sırasıyla adlandırılıp karşılaştırıldı. Karşılaştırma testi (Y1–Y4'te olduğu gibi) commit'lenmedi; üreteçler kalır (`style/svg/cases/`, `calls.ts` `CALLS`), yöntem ADR 0008 “SVG düzenleyicisi”ndedir. S3c'ye kadarki dilimler için: S3c'de TS ↔ Rust karşılaştırması (`parity.test.ts`, kümelerin `fns` ve `ties` alanları) son referanslarla birlikte silindi; `git show c7445e0:apps/web/src/wasm/parity/parity.test.ts` ve `c7445e0:apps/web/src/wasm/parity/harness.ts` yöntemin çalışan biçimidir. Taşınacak TS'i kümenin `fns` alanına koyup testi geri getirin:
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
  - Kaydedici bütün kümeleri yeniden yazar. TS'ten kaydedilmiş eski dosyalar sin/cos'a duyarlı işlemlerde çekirdekten son bitte ayrılır (tolerans içinde, ADR 0008); yeni bir küme eklerken yalnız onun dosyasını alın, ötekileri `git checkout -- fixtures/geometry/v1/calls-*.json` ile geri koyun.
- **Boyut:** `apps/web/src/wasm/pkg/kentos_geometry_wasm_bg.wasm` ham ve `gzip -9` boyutu ADR 0008 tablosuna yeni satır olarak yazılır.

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
  - Başsız SwiftShader'da WebGPU aygıtı ilk karelerde kaybolur (“A valid external Instance reference no longer exists”). `pnpm e2e`'nin WebGPU denetimleri bu yüzden düşer (iki ya da üç: “WebGL2'ye dönüşte tek tuval” ara sıra geçer); taban commit'te de aynıdır. Öbür denetimler anlamlıdır. Bir alt ajan WebGPU'nun bu konteynerde `--use-angle=vulkan` bayrağı olmadan çalıştığını gördü; `cdp.mjs` değiştirilmedi (kullanıcının makinesindeki bayraklar bozulmasın diye denemeden değiştirmeyin).
  - `wasm-bindgen-cli` kurulu gelmez (`cargo install … --locked`, ~1,5 dk).
- **Ölçüm:** taban kullanıcının makinesinde (Intel Iris Xe GPU) alındı.
  - Karşılaştırmayı kullanıcı kendi makinesinde `pnpm perf:interaction --label s6` ile yapar.
  - Bulutta yalnız aynı makinede önce/sonra çifti anlamlıdır. SwiftShader'da `--allow-swiftshader` gerekir; yalnız ana iş parçacığı süreleri anlamlıdır, bir koşu (`--runs 1`) ~50 dk sürer.
  - Düzenek çalışma dizinindeki kaynağı sunar: ölçüm sürerken `apps/web/src/` değişirse ölçüm bozulur. Ölçülecek commit'i ayrı bir git worktree'sinde çalıştırın (`node_modules` bağı ve `apps/web/src/wasm/pkg` kopyasıyla); ana dizinde çalışmaya devam edilebilir.
- **Ağır işler:** kullanıcının makinesinde cargo, tam vitest, e2e ve ölçüm aynı anda çalışmaz (makine bir kez dondu). Bulut konteynerinde paralel çalıştırılabilir (kullanıcı izin verdi), ama ölçüm sürerken başka ağır iş çalışmaz.
- **Alt ajan:** çalıştırmayın. Kullanıcı 24 Eylül'deki üç alt ajan bittikten sonra yenisini istemedi (önceden de kredi yüzünden yalnız gerçekten gerekirse ve tek tek). Alt ajan ayrı worktree'de çalışır, main'e push etmez; sonucunu siz inceleyip sınar ve alırsınız.
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
8. İçe aktarmada “Bu koordinatlar hangi sistemde?” sorusu projenin sistemi seçili açılıyor; içe aktarılabilen tek seçenek o olduğu için kullanıcı hiçbir şeye dokunmadan içe aktarabiliyor. Seçim yapılmadan “İçe aktar” düğmesi kapalı mı kalsın (açık onay)?
9. DXF ACI 251–254 gri tonları AutoCAD 2000 ve sonrasının tablosuna (ezdxf ile aynı: 80, 105, 130, 190) göre düzeltildi; bir AutoCAD çizimiyle doğrulanması iyi olur.
10. Bilgi için (kullanıcı aksini isterse değişir; ayrıntı ADR 0008 S4, S5): hedef katmandaki adsız nokta artık numaralı sayılmaz (önce bir numarayı yutuyordu); köşesiz yolun `$y`/`$x`'i boştur (önce bütün ifadeyi boşaltan hata veriyordu); yazılan değerin tek IEEE işlemiyle yeniden ifadesi (derece → radyan, kâğıt mm → metre, `hedef − temel`) ve dikdörtgen dizinin ötelemeleri TS'te kaldı, çünkü iki dilde bit bit aynıdır ve önizlemede her karede JSON'a değmez.
11. DXF dışa aktarmada tema renkleri: `fg` (ana mürekkep) 7, `fg-dim` (ikincil) 8, `paper` 255 yazılır; KentOS'a geri okununca KentOS verisinden yine tema rengi olur, başka programlarda bu sabit renkler görünür. Başka bir eşleme istenir mi?
12. DXF yazı stili Standard / `arial.ttf` (TrueType; Türkçe harflerin hepsi var). Kurumun kullandığı bir yazı tipi var mı?
13. DXF'te noktalar `$PDMODE` 2 (artı işareti) ve çizim ölçeğinde kâğıtta 1,5 mm boyla gösterilir (tek piksellik nokta haritada görünmez). Uygun mu?
14. ~~Başlangıç WASM sınırı (350 KB gzip) stil motorunun taşınmasıyla aşılacak.~~ **Kararlaştırıldı (24 Eylül):** kullanıcı sınırı 400 KB gzip'e yükseltmeyi seçti; işlev adları pakette kalır (ADR 0005 “Değişiklikler”). Stil derleyicisiyle paket 394,9 KB. Aynı gün sınırı kaldırdı: “WASM boyutu önemli değil, artabilir.” Boyut yine her dilimde ölçülüp yazılır, ama sınır değildir.

## 7. Devralan ajan için ilk adımlar

1. Bu dosyayı ve §1'deki belgeleri okuyun. CLAUDE.md §0 ve §13 sonrası kullanıcının metnidir: yalnız doğrulanmış durum notu eklenir.
2. Ortamı kurun (§5): `pnpm install --frozen-lockfile`, `cargo install wasm-bindgen-cli --version 0.2.128 --locked`, bulutta Chromium sarmalayıcısı (`CHROME_BIN`).
3. `main`'i doğrulayın: `pnpm typecheck`, `pnpm test`, `pnpm rust:test`, `pnpm e2e`. Beklenen sonuçlar §2 “Son doğrulama”dadır; bulutta iki ya da üç WebGPU denetimi bilinen biçimde düşer.
4. §6'daki açık kararları ilgili işe gelince kullanıcıya sorun (WASM bütçesi ve ADR 0009 24 Eylül'de kararlaştırıldı).
5. §3'ten sıradaki işi alın. Dilim başına bir commit, İngilizce ileti (“Shared core, …” ya da “File formats, …”), sonunda oturumun atıf satırları; `tsc`, `pnpm test`, Rust'a dokunulduysa `pnpm rust:test` ve arayüze ya da çekirdeğe dokunulduysa `pnpm e2e` geçince `main`'e ve oturum dalına push edilir.
6. Yeni bir çekirdek işlevi: önce Rust'ta işlev ve birim testi, sonra çağrı tablosu (`op!`), çağrı kümesi ve donmuş fixture (§4), sonra TS cephesi (`op<Sig>('ad')`), en son çağıranlar. Cephede aritmetik yazmayın; bekçi test düşer. WASM boyutunu ADR 0008 tablosuna yazın.
7. İş bitince bu dosyayı güncelleyin: biten maddeyi silin, yeni kararı ekleyin.
