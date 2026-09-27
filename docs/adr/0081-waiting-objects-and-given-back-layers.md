# ADR 0081: Katmanını bekleyen nesneler, kullanılan katmanın geri verilmesi

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** CLAUDE.md §13.1, §21.3; ADR 0026 (bulutta nesne kimliği), 0040–0044 (masaüstünün bulutu), 0072 (katman silme ve sunucu koruması), 0079 (başkasının sildiği katman)
- **Kaynak:** web ajanının `b3c022f` ve `36d87de`'si (`app/cloud/waiting.ts`, `keptLayers.ts`, `sync.ts`, `syncCore.ts`, `syncRemote.ts`, `syncRestore.ts`). Web ajanı önerdi. Kararı sahibin genel yönüyle verdik: veri silmeden önce gelir.

## Bağlam

- Bir katman burada silinmiş, silme henüz sunucuya gitmemiş olabilir. O sırada başka bir düzenleyici o katmana nesne çizer ya da taşır.
- **Nesne katmansız kalıyordu.** Masaüstü böyle bir nesneyi "“X” katmanı çizimde yok" diye atlıyordu; web okunamadı diye reddediyordu. Katman sonradan geri gelse de (geri al, başkasının ağacı) nesne proje yeniden açılana kadar görünmüyordu.
- **Sunucu silmeyi reddediyordu.** Koruma (ADR 0072), içinde nesne kalan katmanı düşüren ağacı reddeder. "Benimkini koru" aynı ağacı yeniden gönderip yeniden reddediliyordu; o nesneler orada durdukça bu sürüyordu.

## Karar

### Nesne katmanını bekler (web `b3c022f`)

- **Kural:** çizimde olmayan bir katmandaki nesne atlanmaz ve reddedilmez; katmanını bekler. Bir şey söylenmez.
  - Buradaki kopyası varsa (başkası taşıdıysa) o arada çizimden gider: dışarıdan değişiklik olarak, geri alma adımı olmadan.
- **Gelişi:** çizim katmana yeniden kavuşunca nesne o anki sürümüyle getirilir ve alınır. Katmanı getirenler:
  - başkasının ağacı;
  - "sunucudakini al";
  - buradaki geri al;
  - geri verilen katman.
- **Bekleme ne zaman biter:**
  - Nesne alınınca, silinince ya da çakışmaya dönünce listeden çıkar.
  - Burada gönderilmemiş değişikliği olan nesne beklemez; mevcut çakışma karar verir.
  - Getirme başarısız olursa nesne listede kalır; kaybolmaz.
- **Projeden çıkarken** hâlâ bekleyen varsa katman başına bir kez söylenir. Katmanın adı sunucunun son bilinen ağacındandır: "“X” katmanı bu çizimde olmadığı için başka birinin N nesnesi burada gösterilmedi; proje yeniden açılınca görünür."
- **Saklanmaz:** yeniden açılan proje sunucudan bütün gelir.

### Kullanılan katman geri verilir (web `36d87de`)

- **Ne zaman:** sunucu metaverinin çakışmasıyla (`@project`) reddeder ve gönderdiğimiz ağaç katman düşürüyordur.
- **Sıra:**
  1. Kaçırılan olaylar hemen istenir. Katmanı olmayan nesneler beklemeye girer.
  2. Sunucunun ağacı okunur.
  3. Bizim ağacın düşürdüğü, sunucunun ağacında hâlâ olan ve üzerinde başkasının nesnesi bekleyen her katman, sunucunun ağacındaki yerine geri konur.
- **Geri kalanımız bizimdir:**
  - yeniden adlandırmalar, stiller ve öbür silmeler kalır;
  - yalnız bizim nesnelerimizi tutan katman yine gider;
  - o katmandaki nesnelerimizin silinmesi bekler ve sonraki komutla gider.
- **Katman başına bir kez söylenir:** "“X” katmanında başkasının nesnesi olduğu için katman silinmedi; sizin nesneleriniz silindi."
- **Yalnız koruma reddettiyse:** çakışma burada biter ve kalan iş hemen yeniden gider.
  - Kanıt: çakışmanın beklenen sürümü (`expected`) sunucununkine (`actual`) eşittir, yani metaveriyi arada kimse değiştirmemiştir.
  - Bunun için `Conflict` artık `expected`'ı da taşır.
- **Başkası metaveriyi de değiştirdiyse** çakışma eskisi gibi sorulur; katman ağacımıza zaten geri konmuştur.
- **Cihaz taslağı geri konurken:** taslağın ağacı, sunucunun nesnelerini hâlâ tutan bir katmanı düşürüyorsa o katman kalır. Nesne taslakta yoksa ya da değişikliği onu aynı katmanda bırakıyorsa sunucunundur. Aynı ileti söylenir. Böylece yeniden açılış o nesneleri katmansız bırakamaz.

### Masaüstünde nasıl

- **Çekirdek:** `kentos-cloud`'un `ProjectSync`'inde:
  - `waiting`: bekleyenler, katmana göre;
  - `arrived`: katmanı gelenler; durumu değiştirmez;
  - `is_waiting`, `waiting_texts`;
  - `may_give_back`, `give_back`: sonucu `GivenBack` (adlar ve yalnız korumanın reddettiği);
  - `Restored.given_back`, `given_back_text`.
- **Artık yok:** `Taken.skipped` ve "Başkasının bir değişikliği çizime alınamadı" iletisi.
- **Arayüz** (`cloud/follow.rs`, `live.rs`):
  - `fetch_arrived`: bekleyenleri getirir. Başkalarının değişiklikleri alındıktan, "sunucudakini al"dan ve katman geri verildikten sonra çağrılır. Çizim değişince (geri al) saatin her tıkında da bakılır.
  - `catch_up`: reddedilen ağaçtan sonra olayları beklemeden ister.
  - `server_tree` ve `give_back_to`: sunucunun ağacını okuyup katmanları geri verir.
- **Uzun sorgu yol verir.** Getirme ve olay isteği uzun sorgunun yerini alır; uzun sorgu onlardan sonra yeniden başlar. Yolda bir getirme varsa, o alınınca yeniden bakılır. Böylece aynı anda tek istek olur ve sürümler sırayla gelir.
- **Açık düzenleme varken** geri verme düzenleme bitince yapılır (`giving`).
- **Kullanıcı seçerse** ("Benimkini koru", "Sunucudakini al"), yolda kalan geri verme bir şey yapmaz.
- **Nesne katmanını her yerde bekler.** "Sunucudakini al"da da, çizimin olmayacağı bir katmandaki sunucu kopyası bekler.
  - Web bu yolda nesneyi okunamadı diye reddediyor. Web ajanına bildirildi; web ajanı `setAside`'ı burada da değerlendirecek.
  - Web, bekleyen bir nesne başkası tarafından çizimde olan bir katmana taşınınca onu listede bırakıyor; masaüstü alınca listeden çıkarır. Bu da web ajanına bildirildi.

## Doğrulama

- **`crates/native/cloud/src/sync/tests.rs`**, web'in dokuz durumu:
  1. geri al katmanı ve nesneyi getirir;
  2. sonra gelen ağaç bekleyeni getirir (eski "atlandı" testi artık bunu sınar);
  3. taşınan kopya katmanı gelene dek gider, bekleyen silinirse bekleme biter;
  4. çıkarken katman başına ileti;
  5. yalnız korumanın reddi: katman geri verilir, bir kez söylenir, silmelerimiz ağaçsız gider, onların nesnesi gelir;
  6. geri verilince ağacımızın geri kalanı bizimdir (yeniden adlandırma gider);
  7. metaveri de değiştiyse çakışma katman geri verilmiş olarak kalır ve "Benimkini koru" gider;
  8. reddedilmeyen ağaçta bir şey geri verilmez;
  9. taslak geri konurken kullanılan katman kalır.
- **`apps/desktop/src/cloud/tests.rs`**, iki akış:
  - Bekleyen nesne katmansız kalır, bir şey söylenmez. Geri al katmanı getirince saatin tıkı onu uzun sorgunun yerine getirir. Çıkarken kalan söylenir.
  - Koruma reddedince olaylar hemen istenir, sonra sunucunun ağacı. Katman geri verilir, bir kez söylenir. Çakışma söylenmez ve silmeler hemen gider. Nesne getirilir.
