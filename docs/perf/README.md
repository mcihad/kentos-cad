# Performans ölçümleri

Ölçümler betikle alınır ve buraya yazılır; elle sayı girilmez. Hedefler [ADR 0005](../adr/0005-performance-acceptance-targets.md)'tedir. ADR **taslaktır ve kullanıcı onayı bekler**; aşağıdaki karşılaştırmalar bu yüzden yalnızca kayıttır, kabul kararı değildir.

```bash
node apps/web/scripts/perf/bundle.mjs  --label baseline   # production build + chunk envanteri
node apps/web/scripts/perf/startup.mjs --label baseline   # vite preview + başsız Chrome, soğuk/ılık × 3
pnpm perf:interaction --label baseline           # etkileşim tabanı: Vite + başsız Chrome (GPU), parsel-50k ve hat-1m × 3 koşu
pnpm perf:interaction --label s1                 # sonraki ölçüm: interaction-s1.{json,md}, tabanla karşılaştırmalı
node apps/web/scripts/perf/modules.mjs --label y4 # pnpm build'den sonra: ağır modüllerin ilk ve ikinci açılışı (boş profil × 3)
```

Ölçüm sırasında makinede başka ağır süreç (Vite, e2e, cargo) çalışmaz.

## S6 kabul ölçümü: kullanıcının makinesinde etkileşim (2026-09-25, `405c364`)

- **Kaynak:** [interaction-s6.md](interaction-s6.md) (ham veri `.json`). Komut `pnpm perf:interaction --label s6`; kod F0 belgeleri dışında `4399477` ile aynı, temiz ayrı bir worktree'de çalıştı.
- **Ortam:** i5-11300H, Iris Xe (ANGLE, OpenGL ES 3.2), başsız Chrome 154, WebGL2. 3 koşunun ortancası.
- **Karşılaştırma geçersiz.** Taban (`c110b15`) Chrome 153 ile alınmıştı, betik sürüm değişince karşılaştırmayı kendi kuralıyla geçersiz sayar. Farklar yine de sürüm gürültüsünün çok üstündedir. Aynı koşullarda yeni bir taban alınana kadar değerler yalnız kayıttır.

| Ölçüt (p95) | Taban (TS geometri, Chrome 153) | S6 (Rust deposu, Chrome 154) |
|---|---|---|
| `parsel-50k`, imleç başına seçme (yakın / genel) | 9,2 / 9,9 ms | 0,09 / 0,17 ms |
| `parsel-50k`, imleç başına kenet (yakın / genel) | 9,2 / 12,6 ms | 0,11 / 0,41 ms |
| `hat-1m`, imleç başına kenet (yakın / genel) | 5,7 / 31,0 ms | 0,93 / 2,13 ms |
| `hat-1m`, buda önizlemesi karesi | 763 ms | 17,5 ms |
| Büyük katmanı yeniden kurma (`parsel-50k` / `hat-1m`) | 112 / 131 ms | 38,6 / 52,5 ms |
| `hat-1m`, kaydırma kare aralığı, GPU dahil (yakın / genel) | 17,1 / 48,1 ms | 17,3 / 48,1 ms |

- Betik 11 gerileme işaretledi. Hepsi p95'i 0,6 ms'nin altında olan araç ve kare adımlarıdır; en büyük mutlak fark 0,22 ms'dir (`parsel-50k` genel görünümde kaydırma olayı 0,60 → 0,82 ms).
- **ADR 0005 taslağına göre açık kalanlar** (onaylanmamış hedefler, yalnız kayıt):
  - `hat-1m` genel görünümde kaydırma kare aralığı 48 ms (≈ 21 kare/sn). Ana iş parçacığı 0,57 ms, yani darboğaz GPU'dur. Hedef 16 ms; iş LOD ve kırpmadadır (TODOS.md `REN-10`).
  - `hat-1m` katman kurma 52,5 ms. Hedef 100 bin segment için 50 ms; bu veri seti 1 milyon segmenttir.

## F0 başlangıç kaydı: kullanıcının makinesinde açılış (2026-09-25, `481d7c4`)

- **Kaynak:** [bundle-f0-2026-09-25.md](bundle-f0-2026-09-25.md), [startup-f0-2026-09-25.md](startup-f0-2026-09-25.md). Ortam ve test sonuçları [docs/baseline/2026-09-25.md](../baseline/2026-09-25.md)'dedir.
- **Ortam:** i5-11300H (Iris Xe ve RTX 3050 Mobile), başsız Chrome 154, WebGL2, `vite preview`. Kod `4399477` ile aynı, temiz ayrı bir worktree'de. 3 ölçümün ortancası.

| Ölçüt | Soğuk | Ilık |
|---|---|---|
| Etkileşime hazır (aralık) | 797 ms (776–819) | 278 ms (258–282) |
| Ana iş parçacığında script süresi | 83 ms | 21 ms |
| Aktarılan toplam (WASM) | 706,4 KB (305,0 KB) | 0,8 KB |

- İlk sayfa JS'i ham / gzip / brotli 551,3 / 174,2 / 147,2 KB; CSS'i 114,4 / 19,2 / 16,8 KB.
- Bulut konteynerindeki ölçümde ılık açılış soğuktan yavaştı (2 274 ms, [startup-wasm-compressed](startup-wasm-compressed-2026-09-25.md)). Bu makinede ılık açılış beklendiği gibi hızlı. Bulut ölçümündeki fark açıklanmış değildir (TODOS.md §20.1).

## Açılış yükü: kitaplık, örnek proje ve pencereler ayrı parçada (2026-09-25)

Kaynak: önce [bundle-f6-before-2026-09-25.md](bundle-f6-before-2026-09-25.md), [startup-f6-before-2026-09-25.md](startup-f6-before-2026-09-25.md); sonra [bundle-f6-after-2026-09-25.md](bundle-f6-after-2026-09-25.md), [startup-f6-after-2026-09-25.md](startup-f6-after-2026-09-25.md). Sistem sembol kitaplığı (bütün MPYY sayfaları), örnek proje ve gösterim kataloğu tek bir ayrı parçadır ve WASM çekirdeği derlenirken paralel iner (`app/startContent.ts`); ayar, işlem aracı ve bulut pencereleri ilk açılışta, WebGPU arka ucu seçilince yüklenir. Davranış değişmedi (uygulama yine örnek projeyle açılır). Bulut konteyneri, başsız Chrome, `vite preview`, 5 ölçümün ortancası.

| Ölçüt | Önce | Sonra |
|---|---|---|
| İlk sayfa JS (giriş ve statik içe aktarmaları; ham / gzip) | 844,5 / 263,5 KB | 538,9 / 170,4 KB |
| Açılışta aktarılan JS (paralel inen parça dahil) | 269,4 KB | 243,8 KB |
| Etkileşime hazır, soğuk (aralık) | 865 ms (835–910) | 808 ms (756–819) |
| Ana iş parçacığında script süresi, soğuk | 107 ms | 96 ms |

Açılıştaki aktarımın çoğu WASM çekirdeğiydi (1 109,7 KB): `vite preview`'in kendi sıkıştırması `application/wasm`'ı dışarıda bırakıyordu. Derleme artık her `.wasm`, `.js`, `.css` dosyasının yanına Brotli (`.br`, kalite 11) ve gzip (`.gz`, düzey 9) kopyasını yazar ve `vite preview` bunları gönderir (`apps/web/vite.config.mjs` `kentosCompress`, Node zlib, bağımlılık yok): soğuk açılışın aktarımı 1 535,9 → 682,6 KB, WASM 1 109,7 → 298,3 KB ([startup-wasm-compressed-2026-09-25.md](startup-wasm-compressed-2026-09-25.md)); yerel sunucuda ağ beklemesi olmadığı için hazır olma süresi aynı kaldı (808 → 830 ms, ölçüm aralığında), kazanç gerçek ağdadır. Üretimde statik sunucu bu kopyaları olduğu gibi göndermelidir: nginx `brotli_static on; gzip_static on;` (brotli için ngx_brotli), Caddy `file_server { precompressed br gzip }`; `application/wasm` türü ve `Vary: Accept-Encoding` gerekir, `/assets/` altı adında özet taşıdığı için `immutable` önbelleklenebilir.

## Ağır modüllerin açılışı (2026-09-24, `e0b9168`)

Kaynak: [modules-y4-2026-09-24.md](modules-y4-2026-09-24.md) (ham veri `.json`; betik `apps/web/scripts/perf/modules.mjs`). Üretim derlemesi `vite preview` ile, her modül kendi boş profiliyle; süre komut satırında Enter'dan pencerenin boyandığı kareye kadardır. Bulut konteyneri, başsız Chrome; kabul ölçümü kullanıcının makinesinde yapılır.

| Modül | İlk açılış | İkinci açılış | İlk açılışta indirilen | ADR 0005 önerisi |
|---|---|---|---|---|
| Stil yöneticisi | 220 ms | 107 ms | 12,9 KB (gzip JS) | ≤ 400 / ≤ 150 ms |
| Model tasarımcısı | 125 ms | 36 ms | 11,1 KB (gzip JS) | ≤ 400 / ≤ 150 ms |
| SVG düzenleyicisi | 163 ms | 42 ms | 51,3 KB gzip JS + 841 KB WASM (bu ölçümde sunucu WASM'ı sıkıştırmadı; derleme artık Brotli kopyasını yazar: 242,6 KB) | ≤ 400 / ≤ 150 ms |

## Etkileşim tabanı (2026-09-24, `c110b15`)

Kaynak: [interaction-baseline.md](interaction-baseline.md) (ham veri `.json`; betik `apps/web/scripts/perf/interaction.mjs`, veri setleri `apps/web/scripts/perf/datasets.mjs`). [ADR 0008](../adr/0008-shared-core-boundary.md)'in S1 diliminden (geometri deposu) önce, kenet, seçme ve katman geometrisi henüz TypeScript'teyken alındı. S1 ve sonraki dilimler aynı betikle, aynı makinede bu tabanla karşılaştırılır; betik gerilemeleri raporun sonunda listeler.

| Ölçüt (p95, 3 koşunun ortancası) | `parsel-50k` (81 229 nesne) | `hat-1m` (1 M segment) | ADR 0005 önerisi |
|---|---|---|---|
| Seçme, imleç hareketi başına (1:1000 / genel) | 9,2 / 9,9 ms | 3,7 / 7,2 ms | < 2 ms (100 bin nesne) |
| Kenet, imleç hareketi başına (1:1000 / genel) | 9,2 / 12,6 ms | 5,7 / 31,0 ms | < 2 ms |
| Buda: kenar seçme / önizleme karesi (1:1000) | 13,3 / 8,5 ms | 2,8 / 763 ms | – |
| Kaydırma: kare, ana iş parçacığı (1:1000 / genel) | 9,0 / 11,8 ms | 1,5 / 1,6 ms | ≤ 16 ms (1 M segment) |
| Kaydırma: kare aralığı, GPU dahil (1:1000 / genel) | 17,0 / 26,4 ms | 17,1 / 48,1 ms | ≤ 16 ms |
| Büyük katmanı yeniden kurma | 112 ms (50 400 alan) | 131 ms (1 M segment) | < 50 ms (100 bin segment) |

Notlar:

- **Seçme ve kenet önerinin 5–6 katı.** `parsel-50k`'da süre yakınlıktan neredeyse bağımsız (9–13 ms): her olayın çoğu `PickIndex`'in bütün nesneleri gezmesi (CLAUDE.md §6.3). Eşyükseltilerde genel görünümde kenet 31 ms'ye çıkıyor: imleç yakınındaki ~100 çoklu çizginin her biri 500 kenarıyla taranıyor.
- **Eşyükseltide buda önizlemesi kare başına ~0,75 s:** önizleme görünen bütün kenarları (1:1000'de ~35 bin) hedefin 500 kenarıyla kesiştiriyor; araç bu veride pratikte kullanılamıyor.
- **Kaydırmada ana iş parçacığının payı küçük, kare hızını GPU belirliyor:** `hat-1m` genel görünümde kare 1,6 ms CPU ama kare aralığı 48 ms (~21 fps, Iris Xe). `parsel-50k`'da karenin çoğu etiketler: üst katman her karede 81 bin nesneyi geziyor (8,6 / 11,3 ms).
- Gürültü: p95'i 0,5 ms'yi geçen 94 ölçütte koşular arası yayılımın ortancası %6, en çok %61 (6 örnekli bir kare ölçütü). p95 aralıkları örtüşüyorsa fark gürültüdür; betik tabanın en kötü koşusunu %10 ve 0,1 ms aşan p95'i gerileme sayar.
- Ölçüm koşulları: başsız Chrome 153, WebGL2 **makinenin GPU'sunda** (Mesa Intel Iris Xe, `--use-angle=gl`), 60 Hz; Vite geliştirme sunucusu, zamanlayıcı 5 µs (cross-origin isolated). Başlangıç ölçümü SwiftShader kullanıyor; burada kullanılamadı: bu veri setlerinde tek kare SwiftShader'da saniyeler sürüyor, kareler GPU sürecinde birikip sayfayı sonradan bir dakikaya varan sürelerle durduruyordu (izleme kaydında commit sırasında `WaitForToken`). Ana iş parçacığı süreleri iki durumda aynıydı; betik SwiftShader'ı `--allow-swiftshader` verilmedikçe reddeder.
- Nesne izleme ve bilgi kartı ölçümde kapalı (ikisi de beklemeye bağlı). Chrome en çok 959 MB (PSS) kullandı; betik 3,5 GB'ı aşarsa durur. Üç koşu 5 dakika sürdü.
- Başka bir makinede ya da başka bir Chrome ya da GPU ile alınan ölçüm bu tabanla karşılaştırılmaz; önce aynı koşullarda yeni taban alınır.

## S1 önce/sonra, bulut (2026-09-24, `d8a7beb` → `21ac4c5`)

Kaynaklar: [interaction-s1-before.md](interaction-s1-before.md) ve [interaction-s1-after.md](interaction-s1-after.md) (karşılaştırması raporun sonunda; ham veri `.json`). Geometri deposundan (ADR 0008 S1) önce P8'de, sonra S1c'de; ikisi de bulut konteynerinde (Xeon, 4 iş parçacığı, Chromium 141, SwiftShader, `--allow-swiftshader`), tek koşu, makinede başka işler çalışırken alındı. Yalnız ana iş parçacığı süreleri anlamlıdır; tabanla (kullanıcının makinesi) karşılaştırılmaz.

| Ölçüt (p95, ms) | `parsel-50k` önce → sonra | `hat-1m` önce → sonra |
|---|---|---|
| Seçme, imleç hareketi başına (1:1000 / genel) | 18,4 / 20,6 → 0,11 / 0,27 | 3,40 / 8,21 → 0,66 / 1,60 |
| Kenet, imleç hareketi başına (1:1000 / genel) | 19,0 / 22,6 → 0,12 / 3,15 | 7,64 / 46,8 → 1,40 / 31,9 |
| Buda: kenar seçme (1:1000) | 19,3 → 0,18 | 4,10 → 1,07 |
| Buda önizlemesi: kare, ana iş parçacığı (1:1000) | 31,6 → 2,59 | 972 → 17,9 |
| Etiketler, kare başına (kaydırma, 1:1000 / genel) | 19,2 / 21,2 → 0,71 / 10,0 | 0,58 / 0,73 → 0,47 / 0,31 |
| Büyük katmanı yeniden kurma (stil değişikliği) | 219 → 307 | 11 289 → 13 129 |

Notlar:

- **Seçme, kenet, buda ve etiketler** 10–1000 kat hızlandı. `hat-1m` genel görünümde kenet hâlâ 19 ms (p50) ve 32 ms (p95): imlecin yakınındaki eşyükseltilerin kesişim adayları. Sıradaki iyileştirme hedefi budur.
- **Yeniden kurma** P8 ile S1c arasında değişmeyen koddur (`apps/web/src/render`, `apps/web/src/style` aynı). Fark iki koşunun makine yükünden gelir; `hat-1m`'de süreyi SwiftShader'ın yazılımla yüklemesi belirler.
- Betik 29 ölçütü gerileme sayıyor. Çoğu p50'si değişmeyen karelerin p95 sıçramaları (`hat-1m` seç karesi p50 1,09 → 1,00, p95 1,36 → 12,3), SwiftShader'ın kare aralığı ve 1 ms altındaki GPU gönderimleridir. Tek koşu ve yüklü makinede p95 güvenilir değildir.
- Kabul ölçümü kullanıcının makinesindedir: `pnpm perf:interaction --label s1`, tabanla karşılaştırmalı.

## Faz A başlangıç kaydı (2026-09-23, `85be871`)

Kaynaklar:
- [bundle-baseline-2026-09-23.md](bundle-baseline-2026-09-23.md)
- [startup-baseline-2026-09-23.md](startup-baseline-2026-09-23.md) (ham veri `.json`)

| Ölçüt | Ölçülen | ADR 0005 önerisi |
|---|---|---|
| İlk sayfa JS (gzip) | 258,4 KB | ≤ 350 KB |
| İlk sayfa CSS (gzip) | 15,8 KB | ≤ 40 KB |
| Başlangıç WASM | yok (uygulama WASM yüklemiyor) | sınır yok (2026-09-24, sahibinin kararı; önce 300, 350, 400 KB) |
| Etkileşime hazır, soğuk (ortanca) | 557 ms | ≤ 1,5 s |
| Etkileşime hazır, ılık (ortanca) | 288 ms | ≤ 0,8 s |
| Script süresi, soğuk | 177 ms | ≤ 600 ms |
| Ağır modülün ilk / ikinci açılışı | ölçülmedi | ≤ 400 ms / ≤ 150 ms |
| Etkileşim bütçeleri (§6.1) | ölçülmedi (2026-09-24'ten beri yukarıda) | ADR 0005 tablosu |

Notlar:

- **İlk yük demo projeyi ve bütün MPYY sistem kitaplığını içeriyor.** ADR'nin başlangıç hedefleri demo ayrıldıktan sonrası içindir; bugünkü değerler bu yükle birlikte ölçüldü.
- **Oturumun ilk soğuk yüklemesi ~1,8 s sürüyor** (her ölçümde tekrarlandı). Sonraki soğuk yüklemeler de boş profille açılıyor ve ~0,55 s sürüyor. Fark uygulamada değil, tarayıcının ve işletim sisteminin soğuk başlangıcında görünüyor. Ortanca bu ilk yüklemeyi içermiyor; en kötü durum aralık sütunundadır.
- Aktarım `vite preview`'un gzip boyutudur. Brotli ile ilk sayfa JS'i 210,7 KB olur (envanter).
- İstek listesinde `/v1/health` (API yokken 503) vardır. Denetim uygulama boştayken yapılır, etkileşime hazır olmayı beklemez.
- Ağır modül açılışları için henüz betik yok; bu, ADR 0005 onaylandıktan sonraki ölçüm işidir. §6.1 etkileşim bütçeleri ve `hat-1m`, `parsel-50k` veri setleri 2026-09-24'te eklendi (yukarıda).
- Referans ortam: Intel Core i5-11300H, 16 GB, Ubuntu, Chrome 153 başsız, WebGL2.
