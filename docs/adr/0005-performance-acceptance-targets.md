# ADR 0005: Performans ve kabul hedefleri

- **Durum:** **taslak, kullanıcı onayı bekliyor.** Ölçümden önce yazıldı (CLAUDE.md §19.0, §20.4). Değerler ölçüm sonucuna göre değiştirilmez; değişiklik ancak gerekçeli yeni bir ADR sürümüyle yapılır.
- **Tarih:** 2026-09-23
- **Bağlam belgesi:** CLAUDE.md §6.1, §17, §18, §19.0, §20, §21
- **Sonraki kararlar (2026-09-25):** tile satırları ve Martin karşılaştırması tarihsel/gelecek yayın başvurusudur, bugünkü proje bulutunun kabul kapısı değildir ([ADR 0012](0012-server-scope-project-cloud.md), TODOS.md §20.1). Proje kataloğu, büyük binary aç/kaydet, paylaşım ve yetki iptalinin bütçeleri ölçülüp ayrıca yazılacak (TODOS.md `CLOUD-28`). ADR yine taslaktır ve onay bekler.

## Referans ortam

| Öğe | Değer |
|---|---|
| Makine | Intel Core i5-11300H (4 çekirdek / 8 iş parçacığı), 16 GB RAM |
| İşletim sistemi | Ubuntu 26.04 |
| Tarayıcı | Google Chrome 153; ölçümde başsız, WebGL2 |
| Ağ | yerel (`vite preview`); aktarım bayt olarak ölçülür, süre ağdan bağımsız raporlanır |
| Önbellek | soğuk (boş profil) ve ılık (ikinci yükleme) ayrı |

Makine masaüstü, Vite ve başsız Chrome ile paylaşılıyor. Ölçüm sırasında başka ağır süreç çalışmaz. Her ölçüm 3 kez alınır, ortanca raporlanır.

## Hedefler (öneri)

### Başlangıç (§20)

Günlük 2D çizim, demo katalogu olmadan açılır.

| Ölçüt | Hedef |
|---|---|
| İlk sayfada istenen JS (gzip) | ≤ 350 KB |
| İlk sayfada istenen CSS (gzip) | ≤ 40 KB |
| Başlangıç WASM (gzip) | sınır yok (2026-09-24, sahibinin kararı; önce 300, 350, 400 KB idi; aşağıda “Değişiklikler”). Boyut ölçülür ve raporlanır, hedef değildir |
| Etkileşime hazır (`kentos:interactive`), soğuk | ≤ 1,5 s |
| Etkileşime hazır, ılık | ≤ 0,8 s |
| Ana iş parçacığında script süresi, soğuk | ≤ 600 ms |
| Ağır modülün (stil yöneticisi, model tasarımcısı, SVG düzenleyici) ilk açılışı | ≤ 400 ms (indirme + başlatma, yerel) |
| Aynı modülün ikinci açılışı | ≤ 150 ms |

- İlk yük bugün demo projeyi ve bütün sistem sembol katalogunu içeriyor. Bu hedefler demo ayrıldıktan sonrası içindir.
- **Bugünkü başlangıç ölçümü** (`docs/perf/bundle-baseline-*` ve `docs/perf/startup-baseline-*`) yalnızca kayıttır. Hedefi geçmemesi bir hata değil, §20.4 adım 2'nin iş listesidir.

### Etkileşim (§6.1 korunur)

| Senaryo | Hedef |
|---|---|
| Kaydırma ve yakınlaştırma | 1 milyon segmentte 60 fps (kare ≤ 16 ms, p95) |
| İmleç hareketinde seçme ve kenet | 100 bin nesnede olay başına < 2 ms (p95) |
| Bir katmanı yeniden kurma | 100 bin segmentte < 50 ms |
| Açık panelde seçim değişikliği | < 8 ms |

### Sunucu (Faz B–D; ölçüm ilgili faz başında, aynı veri setiyle)

| Senaryo | p50 | p95 | p99 |
|---|---|---|---|
| Nesne commit'i (tek nesne, `expected_version`, audit, outbox) | 30 ms | 80 ms | 200 ms |
| Tile, ılık önbellek (gateway + yetki dahil) | 15 ms | 40 ms | 100 ms |
| Tile, soğuk (PostGIS `ST_AsMVT`, 50 bin nesnelik katman) | 80 ms | 250 ms | 600 ms |
| Commit sonrası değişikliğin ikinci istemcide görünmesi | 500 ms | 1,5 s | 3 s |
| Job kabul yanıtı (`202 + job_id`) | 20 ms | 50 ms | 150 ms |
| Worker'ın kuyruktan iş alması (boşken) | 250 ms | 1 s | 2 s |

- İki “Tile” satırı **2026-09-25'ten beri kabul kapısı değildir** ([ADR 0012](0012-server-scope-project-cloud.md)); yayın kapsamı ileride açılırsa yeniden değerlendirilir.
- ~~Martin karşılaştırması (§17) aynı veri, indeks ve donanımla iki rapor olarak yapılır: saf Martin ve KentOS gateway dahil.~~ (2026-09-25: Martin rolü kapsam dışı, ADR 0012.)
- "Daha hızlı" iddiası yalnızca bu koşullarda geçen ölçümle yapılır.

### Veri setleri

| Ad | İçerik | Amaç |
|---|---|---|
| `demo` | Bugünkü örnek pafta ve MPYY katalogu (~2 000 nesne) | Başlangıç ve görsel doğruluk |
| `parsel-50k` | 50 000 delikli/yaylı parsel, TM36 koordinatında, öznitelikli | Commit, seçme (tile amacı 2026-09-25'te düştü, ADR 0012) |
| `hat-1m` | 1 milyon segment (eşyükselti benzeri) | Kaydırma, katman kurma |
| `ifraz-referans` | §23'ün bağımsız referanslı alan ve hisse örnekleri | Kadastral doğruluk |

Büyük veri setleri betikle, tekrarlanabilir bir tohumla üretilir. Hangi alanın ya da parselin gerçek veri olduğu ayrıca belirtilir.

### Renderer görsel toleransı

- WebGL2 ile WebGPU aynı sahnede, ızgara kapalıyken çizilen piksel sayısında %1'den az fark verir (bugünkü e2e ölçütü).
- MPYY görsel karşılaştırmasında sembol konumu ± 1 CSS px, çizgi kalınlığı ± 0,5 px, renk ΔE < 3.

### Eşzamanlılık

Faz B–D yük testlerinde tek tenant'ta 25 eşzamanlı editör ve 200 görüntüleyici kullanılır. Bu sayı ilk hedeftir; müşteri yükü bilinince yeni ADR ile büyütülür.

## Değişiklikler

- **2026-09-24, başlangıç WASM 300 → 350 KB gzip (sahibinin kararı).** Ortak çekirdek (ADR 0008) uygulamanın bütün CAD hesabını tek pakette topladı; paket 297,9 KB'a ulaştı ve sıradaki çekirdek işleri (§23.3 sağlam kararlar, depoda dönüşüm) kod ekleyecek. Değerlendirilen seçenekler: işlev adları bölümünü üretim paketinden atmak (gzip 297 → 276 KB; bedeli, bir tuzağın yığın izinde adlar yerine numaralar), ağır işlemleri ilk kullanımda yüklenen ayrı bir pakete bölmek (geometri deposu tek modülde kalmak zorunda) ve sınırı yükseltmek. Sahip sınırı yükseltmeyi seçti; adlar pakette kalır. Öbür hedefler değişmedi.

- **2026-09-24, ifade dili Rust'a taşındı (ADR 0008 “İfade dili”):** başlangıç WASM'ı 303,9 → 349,6 KB gzip; 350 KB sınırına 0,45 KB kaldı. Sınır değişmedi. Stil motorunun geometrisi (sıradaki style-core dilimleri) sınırı aşacak; karar sahibinindir.

- **2026-09-24, başlangıç WASM 350 → 400 KB gzip (sahibinin kararı).** Stil derleyicisi Rust'a taşınınca (ADR 0008 “Stil derleyicisi”) paket 350 KB'ı aşacaktı. Sahibe üç seçenek sunuldu: sınırı yükseltmek, işlev adları bölümünü üretim paketinden atmak (~24 KB gzip; bedeli, bir tuzağın yığın izinde adlar yerine numaralar), stil motorunu ayrı pakete koymak (çizim açılışta stil motorunu istediği için ilk yükü azaltmaz). Sahip sınırı 400 KB'a yükseltmeyi seçti; adlar pakette kalır. Stil derleyicisiyle paket 394,9 KB gzip oldu. Öbür hedefler değişmedi.

- **2026-09-24, SVG düzenleyicisinin geometrisi kendi paketinde (ADR 0008 “SVG düzenleyicisi”).** Düzenleyici başlangıçta yüklenmediği için geometrisi (yol işlemleri, düğümler, kenet, izleme, SVG okuma ve yazma) başlangıç paketine girmedi: ayrı paket 860 890 bayt, gzip -9 315 799; düzenleyici ilk açılışta indirir ve derler. Başlangıç paketi değişmedi (394,9 KB gzip). Ağır modülün ilk açılışı hedefi (≤ 400 ms) bu paketi de kapsar. Üç ağır modülün açılışı ilk kez ölçüldü (`apps/web/scripts/perf/modules.mjs`, [modules-y4-2026-09-24.md](../perf/modules-y4-2026-09-24.md); üretim derlemesi, bulut, 3 ölçümün ortancası): ilk / ikinci açılış stil yöneticisi 220 / 107 ms, model tasarımcısı 125 / 36 ms, SVG düzenleyicisi 163 / 42 ms. Üçü de hedefin içinde.

- **2026-09-24, başlangıç WASM sınırı kaldırıldı (sahibinin kararı).** Sahip: “WASM boyutu önemli değil, artabilir.” Başlangıç paketinin boyutu artık bir hedef değildir. Boyut her çekirdek diliminden sonra yine ölçülüp ADR 0008'e yazılır, değişimi görünür kalsın diye. Açılış süresi hedefleri (etkileşime hazır, script süresi) ve ağır modüllerin açılış hedefleri sürer; paketin indirilmesi ve derlenmesi onlara girer. Ağır modüllerin isteğe bağlı yüklenmesi (CLAUDE.md §20) boyut için değil ilk yük için sürer: SVG düzenleyicisinin paketi ayrı kalır.

## Onay

Kullanıcı onaylayınca **Durum** "kabul edildi" olur. Ölçüm raporları hep bu tablolara göre yazılır.
