# ADR 0033: Veritabanı projesinin tek anlık KCAD v2 görüntüsü

- **Durum:** kabul edildi (2026-09-26). Yön TODOS.md `SYNC-05` (revizyonu tutarlı görüntü, kilit tutmadan), `PG-15` (PostGIS'ten gömülü `.kcad`) ve `PG-22`'den (`.kcad` → PostGIS → `.kcad` gidiş-dönüşü) gelir.
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** CLAUDE.md §13, §15, §21; TODOS.md §10, §11; ADR 0006 (CAD kaynağı ve GIS türevi), 0025 (KCAD v2), 0026 (kalıcı kimlikle eşitleme), 0031 (dosya projeleri)

## Bağlam

- Veritabanı projesi nesne nesne PostGIS'tedir. İstemci onu sayfa sayfa açar (`GET …/features`); bir bütün olarak `.kcad` dosyası sunucudan alınamıyordu.
- Sayfalar ayrı işlemlerde okunur. Arada yazılan bir değişiklik, projenin bir kısmını eski, bir kısmını yeni gösterebilir; açılış bunu olay akışıyla sonradan düzeltir. Dosya ise tek bir revizyonu göstermelidir (`SYNC-05`: metadata ve geometri farklı revizyonlardan karışmaz).
- Görüntüyü almak için projeyi kilitlemek, o sırada kaydedenleri bekletir (`SYNC-05`: serileştirme boyunca yazma kilidi tutulmaz).

## Karar

### Tek an, kilitsiz

- Görüntü tek bir `REPEATABLE READ, READ ONLY` işlemde okunur (`kentos_postgres::Db::snapshot`): proje satırı (ad, ayarlar, katmanlar, etkin katman, orijin, açılış görünümü, stiller, veri revizyonu) ve bütün nesneler aynı anı görür. Arada kaydedilen değişiklik görüntüye girmez.
- Kilit alınmaz; o sırada kaydeden kimse beklemez.
- Satır düzeyi güvenlik aynıdır: erişimi o arada kaldırılan kişi projeyi göremez (404).
- Aynı anın olay imleci görüntüyle birlikte verilir. İstemci dosyayı açıp o imleçten sonraki olayları izleyerek güncel kalabilir. Bir projenin olayları proje kilidi altında sırayla yazıldığı için, o anın en yeni olayı arada eksik olay bırakmaz.

### Dosya

- Ortak kodek yazar ve dışarı vermeden önce geri okur (`kentos_kcad::encode_verified`; kaydetme ile aynı kural).
- Nesneler kalıcı kimlikleriyle, kimlik sırasında gelir: projeyi açan istemcinin aldığı sıra. Dosya projenin kimliğini taşır; göç kaynağı (`migratedFrom`) yazılmaz, çünkü nesnelerin kimlikleri zaten kalıcıdır.
- **Korunmayan tek şey eksi sıfırdır:** PostgreSQL sayılarında −0 yoktur, sıfır olur (`cad.rs`). Başka her değer bit bit geri gelir. KCAD v2 −0'ı korur (ADR 0025); fark yalnız veritabanı yolundadır.
- Veritabanına hiç girmeyen değerler görüntüde de yoktur: sunucu ±10⁹'u aşan koordinat ve boyutları nesne yazılırken reddeder (`cad.rs`).
- Kodlama async iş parçacıklarının dışında yapılır. Dosya yüklemelerinin doğrulamasıyla aynı sınır uygulanır: aynı anda en çok iki çizim ve baytları bellekte.

### Yol ve yetki

- `GET /v1/tenants/{kurum}/projects/{proje}/snapshot`: dosyanın baytları. `project.download` ister; kurumun `viewer_download` politikası uygulanır.
- Başlıklar:
  - `ETag`: SHA-256;
  - `X-Kentos-Revision`: veri revizyonu;
  - `X-Kentos-Event-Cursor`: olay imleci;
  - dosya adı: projenin adı ve revizyon.
- Dosya projesi reddedilir (422): o zaten `.kcad` revizyonlarıdır (`GET …/files/{n}`, ADR 0031).
- Çöpteki proje 410, erişilemeyen proje 404'tür (ADR 0015).

## Bu dilimde olmayanlar

- Görüntünün nesne deposuna kontrol noktası olarak kaydedilmesi ve sıklık politikası (`SYNC-11`, `CLOUD-07`). Bugün görüntü istenince üretilir, saklanmaz.
- Veritabanı projesini dosya projesine ya da tersine çevirme komutları (“dosyaya göm”, “PostGIS'e aktar”, §10.1).
- Web ve masaüstünde “sunucudan .kcad indir” ve görüntüden açılış. Web tarafı dosya projesi arayüzüyle birlikte web ajanına gider.
- Çok büyük projede akışla kodlama (`SYNC-10`). Bugün çizim ve baytları bellekte bir kez tutulur.
- Bloklar, tipli öznitelikler ve ilişkiler modelde yok; `PG-22`'nin onları sınayan kısmı açık.

## Doğrulama (26 Eylül 2026, Linux)

- `crates/server/application/tests/project_snapshot.rs`, gerçek veritabanı:
  - gidiş-dönüş: `fixtures/kcad/v2/drawing.kcad` projeye çevrilir. Proje kilitsiz başlar, nesneler bir kayıtta yazılır, sonra kilit konur. Alınan görüntü aynı çizimi verir: 13 türün hepsi (14 nesne), kalıcı kimlikler, ayarlar, katman ağacı (kilitli ve gizli katman), stiller, orijin ve görünüm. Nesneler kimlik sırasındadır. Revizyon ve olay imleci projenin bilgisiyle aynıdır;
  - dosyanın uç değerli nesnesi (f64'ün en büyüğü, en küçük alt normal sayı) sunucunun ±10⁹ sınırı yüzünden buluta girmez (mevcut kural, `cad.rs`). Reddi alanını söyler (`features[13].entity`) ve hiçbir nesne yazılmaz. Böyle bir çizim buluta ancak o nesne olmadan yüklenir;
  - tek an: görüntü işlemi açıkken yapılan kayıt beklemez (10 saniye sınırı) ve görüntüye girmez; yeni görüntü onu gösterir, revizyon bir artar;
  - yetki: görüntüleyici politika izin verdikçe alır, politika kapanınca 403; dosya projesi 422.
- HTTP (`apps/api/src/http/files_tests.rs`): baytlar geri okunur, başlıklar, başka kurumdan birine 404.
- Kasıtlı bozma: `Db::snapshot`'ta yalıtım düzeyi kaldırılınca “tek an” testi düştü; geri alındı.
