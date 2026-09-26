# ADR 0031: Dosya olarak saklanan bulut projeleri: yükleme, doğrulama, revizyon

- **Durum:** kabul edildi (2026-09-26). Yön ADR 0011 (`.kcad` binary snapshot), ADR 0012 (içerik nesne deposunda, katalog ve izinler PostgreSQL'de) ve TODOS.md §10'dan (`SYNC-02..06`, `SYNC-16`) gelir. Saklama biçiminin sözleşmesi, nesne deposunun ilk arka ucu, yükleme ve kayıt akışı, sınırlar ve temizlik bu dilimin kararıdır.
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** CLAUDE.md §13, §16, §18, §21; TODOS.md §10.1, `SYNC-02..06`, `SYNC-10`, `SYNC-16`; ADR 0013 (ürün komutu), 0015 (erişim), 0025 (KCAD v2), 0026 (kalıcı kimlik), 0028 (katalog)

## Bağlam

- Bulut projelerinin hepsi nesne nesne PostGIS'te saklanıyordu (`project.changes`). TODOS.md §10.1 ikinci bir biçim ister: tamamlanmış, değişmez `.kcad` dosya revizyonları; içerik nesne deposunda, katalog ve izinler veritabanında (ADR 0012).
- KCAD v2 kodeği ortak Rust'ta var (ADR 0025); sunucu yüklenen dosyayı aynı kodekle doğrulayabilir.
- Yeni bir nesne deposu kütüphanesi (S3 istemcisi) yeni bağımlılık ister; sahibin onayı gerekir.

## Karar

### Saklama biçimi

- `ProjectStorage` iki değerlidir: `database` (bugünkü, nesne nesne PostGIS) ve `file` (değişmez KCAD v2 revizyonları).
- Biçim proje açılırken seçilir (`ProjectCreate.storage`, yoksa `database`) ve değişmez: `project.storage` sütununu değiştiren güncelleme veritabanında reddedilir (migration 0006'daki tetikleyici). Biçimler arası geçiş ileride kendi açık komutudur (“PostGIS'e aktar”, “dosyaya göm”).
- Dosya projesinde nesneler tek tek yazılmaz: `project.changes` açık bir iletiyle reddedilir. Veritabanı projesine dosya yüklenmez.

### Nesne deposu

- Arayüz `kentos_application::blobs::Blobs`: nesne oluştur (yazılırken SHA-256 hesaplanır, bildirilen boydan fazlası kesilir), son anahtarına taşı, oku, sil, kalıcı silinen projenin nesnelerini sil.
- **İlk arka uç sunucunun bir klasörüdür:** `KENTOS_BLOB_DIR`; verilmezse env dosyasının yanında `.run/blobs` (geliştirme). Üretimde klasör veritabanıyla birlikte yedeklenen bir diskte olmalıdır. S3 uyumlu arka uç aynı arayüzün arkasına sonra gelir (`SYNC-03`); yeni bağımlılık olduğu için sahibin onayıyla.
- Anahtarlar yalnız kimlik, sayı ve onaltılık özetten kurulur: `uploads/{kurum}/{proje}/{yükleme}` ve `revisions/{kurum}/{proje}/{revizyon}-{sha256}`. Başka bir şey içeren anahtar reddedilir, klasörün dışına çıkamaz.
- Yazma diske işlenmeden (fsync) bildirilmez; son anahtara taşıma aynı klasör ağacında yeniden adlandırmadır, klasör de diske işlenir.

### Bir revizyonun kaydı

1. **Yükleme aç:** `POST …/projects/{proje}/uploads` gövdesi `{ size, sha256 }`. `feature.write` ister; proje arşivde ya da çöpte değilse. Boyut 1 bayt ile 256 MiB arasında olmalı, özet 64 küçük harfli onaltılık rakam. Yanıt `FileUpload` (kimlik, bitiş zamanı).
2. **Baytları gönder:** `PUT …/uploads/{yükleme}`, `application/octet-stream`.
   - Gövde çerçeve çerçeve depoya akar; sunucu dosyanın tamamını bellekte tutmaz. Bu yolun kendi sınırları vardır: en çok 256 MiB, on dakika.
   - Bildirilenden fazla bayt gelince kesilir. Boyut ya da özet tutmazsa ya da dosya KCAD v2 olarak okunamazsa hiçbir şey saklanmaz; hata alanı söyler (`size`, `sha256`).
   - Gövde yarıda kesilirse (bağlantı koptu) de hiçbir şey saklanmaz; aynı yükleme yeniden gönderilebilir.
   - KCAD doğrulaması ortak kodekle, async iş parçacıklarının dışında yapılır. Aynı anda en çok iki dosya doğrulanır; her biri bütün olarak okunup çözüldüğü için birkaç büyük yükleme sunucunun belleğini birlikte tüketemez.
3. **Kaydet:** ürün komutu `project.file.commit` v1, projenin komut yolunda, girdisi `{ uploadId }`.
   - Proje kilitlenir; erişim kilit altında yeniden sorulur (`SYNC-16`: yüklemenin başında izinli olmak yetmez).
   - `expectedVersions["@file"]` dosyanın dayandığı revizyondur (ilk kayıtta `"0"`). En yeni revizyon başkaysa hiçbir şey yazılmaz: çakışma (`@file`, beklenen ve gerçek revizyonla; `ApiError.revision` veri revizyonu). İki cihazın dosyaları bayt düzeyinde birleştirilmez (`SYNC-06`).
   - Yükleme, depo nesnesi son anahtarına taşınarak revizyon olur. Satır, projenin `file_revision`'ı ve veri revizyonu, `project.file` olayı, denetim kaydı ve saklı yanıt aynı işlemde yazılır.
   - Aynı idempotency anahtarıyla gelen yeniden deneme saklı yanıtı alır; revizyon iki kez yazılmaz.
- Depo ile veritabanı arasında ortak işlem yoktur (`SYNC-03`). Ara durumlar şöyle kapanır:
  - Veritabanına yazılamadan hata: nesne yükleme anahtarına geri taşınır; yükleme yeniden kaydedilebilir.
  - İşlemin sonucu bilinmiyorsa (bağlantı koptu, yanıt kayboldu) ya da sunucu taşımayla kayıt arasında çöktüyse nesne son anahtarında kalır. Kayıt yazıldıysa revizyonun baytları yerindedir. Yazılmadıysa aynı yüklemenin yeniden kaydı nesneyi orada bulur; anahtar içeriğin özetini taşıdığı için içerik aynıdır.
  - Kaydedilmemiş böyle bir nesneyi projenin bir sonraki kaydı kilit altında siler: en yeni revizyonun üstündeki nesneler yalnız bitmemiş kayıtlarındır. O yükleme artık kaydedilemez (404); istemci dosyayı yeniden yükler.
- **Yalnız yüklemeyi açan kişi** baytları gönderir ve onu kaydeder. Başka herkes için yükleme yoktur (404).
- Bir revizyon kaydedildikten sonra değişmez ve silinmez; proje kalıcı silinince onunla gider.

### Okuma

- `GET …/files`: revizyonlar, en yenisi önce, kaydedenin adıyla (`project.read`).
- `GET …/files/{n}`: bir revizyonun baytları (`project.download`; kurumun `viewer_download` politikası uygulanır). Dosya parça parça okunarak gönderilir. `ETag` SHA-256'dır, `X-Kentos-Revision` revizyondur, dosya adı projenin adı ve revizyonla verilir.
- Erişilemeyen projede bütün yollar, var olmayan projedeki gibi aynı 404'ü verir (ADR 0015).

### Temizlik

- Sunucu saatte bir çalıştırır (`files::cleanup`):
  - 24 saat içinde kaydedilmeyen yükleme satırı ve baytları silinir (`kentos.expire_uploads`);
  - 48 saatten eski yükleme dosyası, veritabanı ne derse desin, silinir: yüklemeler zaten geçicidir;
  - kalıcı silinen projelerin depodaki revizyonları silinir (`kentos.existing_projects`).
- İki işlev de sahibin haklarıyla çalışır, çünkü sunucunun rolü başka projelerin satırlarını göremez. Yalnız yukarıdakileri yapar.

## Bu dilimde olmayanlar

- **Web ve masaüstü arayüzü:** dosya projesi açma, kaydetme, revizyon listesi ve durumlar (`SYNC-04`, `SYNC-15`). Web tarafını web ajanı yapar; masaüstünde bulut henüz yok.
- **Dosya projesinin kopyası:** `project.duplicate` açık bir iletiyle reddeder. Kopya, son revizyonun nesnesini yeni projeye kopyalamayı ister.
- **Revizyon geçmişi işlemleri:** karşılaştırma, adlandırılmış checkpoint, yeni proje olarak geri yükleme (`SYNC-11`, `CLOUD-07`).
- **Parçalı ve sürdürülebilir yükleme** ve 256 MiB'den büyük dosyalar (`SYNC-10`).
- **Veritabanı projesinden dosya snapshot'ı** (`SYNC-05`).
- **S3 uyumlu depo, kota, antivirüs taraması** (`SYNC-03`, `CLOUD-06`, `CLOUD-26`).

## Doğrulama (26 Eylül 2026, Linux)

- Uygulama testleri (`crates/server/application/tests/project_files.rs`, gerçek veritabanı):
  - revizyonlar sırayla kaydedilir, eski revizyona dayanan kayıt çakışmadır, yeniden deneme saklı yanıtı alır;
  - boyut, özet ve KCAD doğrulaması; reddedilen yüklemeden depoda hiçbir şey kalmaz;
  - veritabanı projesi dosya almaz, dosya projesi nesne değişikliği almaz;
  - yalnız yazar yükler, başkasının yüklemesi yoktur; indirme politikaya uyar;
  - temizlik süresi dolan yüklemeyi ve kalıcı silinen projenin nesnelerini siler;
  - yarım kalan kayıt: aynı yükleme nesneyi son anahtarında bulup kaydedilir; başka bir yükleme kaydedilince artık nesne silinir ve yarım kalan yükleme bulunmaz.
- Depo birim testleri: özet, kesme, taşıma (ikinci kez taşıma ve hiçbir yerde olmayan nesne dahil), en yeni revizyonun üstündekilerin silinmesi, anahtarın klasör dışına çıkmaması, eski yükleme dosyalarının süpürülmesi.
- HTTP testi (`apps/api/src/http/files_tests.rs`): akışla yükleme, kayıt, liste, başlıklarıyla indirme, bildirilenden uzun gövdenin alan adıyla reddi, yarıda kesilen gövdeden sonra aynı yüklemenin yeniden gönderilmesi, başka kurumdan birine bütün yollarda aynı 404.
- Kasıtlı bozma, ikisi de geri alındı:
  - beklenen revizyon denetimi kapatılınca çakışma testi düştü;
  - kayıttaki artık silme kapatılınca yarım kalan kayıt testi düştü (3 nesne, beklenen 2).
- Migration 0006 geliştirme veritabanına yedek alınarak uygulandı; sağlama toplamı `RELEASED`'de sabit.
