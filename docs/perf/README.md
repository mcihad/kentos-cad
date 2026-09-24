# Performans ölçümleri

Ölçümler betikle alınır ve buraya yazılır; elle sayı girilmez. Hedefler [ADR 0005](../adr/0005-performance-acceptance-targets.md)'tedir. ADR **taslaktır ve kullanıcı onayı bekler**; aşağıdaki karşılaştırmalar bu yüzden yalnızca kayıttır, kabul kararı değildir.

```bash
node apps/web/scripts/perf/bundle.mjs  --label baseline   # production build + chunk envanteri
node apps/web/scripts/perf/startup.mjs --label baseline   # vite preview + başsız Chrome, soğuk/ılık × 3
pnpm perf:interaction --label baseline           # etkileşim tabanı: Vite + başsız Chrome (GPU), parsel-50k ve hat-1m × 3 koşu
pnpm perf:interaction --label s1                 # sonraki ölçüm: interaction-s1.{json,md}, tabanla karşılaştırmalı
```

Ölçüm sırasında makinede başka ağır süreç (Vite, e2e, cargo) çalışmaz.

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
| Başlangıç WASM | yok (uygulama WASM yüklemiyor) | ≤ 300 KB |
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
