# Bulut portalı planı

Kararlar ve gerekçeler [ADR 9000](../docs/adr/9000-cloud-portal.md)'dedir. Bu plan, bulut-portal dalında yapılanları ve
sıradakileri tutar. Bir madde ancak sınaması ve gerçek pencerede görüntüsüyle birlikte kapanır.

## Yapılanlar (17 Kasım 2026)

- [x] `BUL-01` **Platform yönetimi:** genel bakış, kurumlar, meslek mensupları, hesaplar, denetim kaydı ve platform
  ayarları.
  - Kurum ilk sahibiyle birlikte açılır.
  - Koltuk, kullanılan koltuk sayısının altına indirilemez.
  - Kurum askıya alınabilir.
  - Hesap kapatılınca oturumları da kapanır.
  - Yönetici yalnız `kentosd platform-admin` ile atanır.
- [x] `BUL-02` **Meslek mensubu kaydı ve doğrulama:**
  - Kayıt bir ayarla açılır; ayar varsayılan olarak kapalıdır.
  - Kaydı sistem yöneticisi oda sicil numarasına bakarak doğrular, onaylamaz ya da askıya alır; gerekçe kişiye
    bildirim olarak gider.
  - Kendi açılan hesabın e-postası doğrulanınca doğrulanmış sayılır.
- [x] `BUL-03` **Kurumun portal ayarları ve üyeleri:**
  - Başvuru alıp almadığı ve hangi türleri aldığı; iletişim bilgileri.
  - Üye ekleme; rol, koltuk ve üyelik durumu.
  - Son sahip korunur; sahip rolünü yalnız sahip verir.
- [x] `BUL-04` **Başvuru akışı:** taslak → gönderim → inceleme → düzeltme (tur tur) → onay ya da ret; geri çekme.
  - Sorumlu atanır.
  - Gerekçesiz ret ve düzeltme isteği reddedilir.
  - Bayat adım çakışmadır.
  - Her kurumun kendi yıllık dosya numarası vardır.
- [x] `BUL-05` **Proje aktarımı:** taslak → gönderim → üstlenme ya da üstlenmeme → teslim → düzeltme ya da tamamlama;
  iptal edilebilir. Aktarımı açmak ve göndermek proje yöneticisi ve üstünün işidir.
- [x] `BUL-06` **Paketler ve dosyalar:**
  - Paketler sürüm sürüm tutulur; gönderilen dosya değiştirilmez.
  - Dosyalar SHA-256'yla saklanır ve her okunuşta denetlenir.
  - Bulut projesinin şimdiki hâli pakete eklenebilir.
  - KCAD dosyası okuyanın alanında proje olarak açılabilir.
- [x] `BUL-07` **Yazışma ve bildirim:**
  - Ekli iletiler.
  - Eksik ve yanlış listeleri ile karar gerekçeleri ayrı türdedir.
  - Bildirim karşı tarafa gider; belediyede sorumluya, sorumlu yoksa dosyalara bakan üyelere.
- [x] `BUL-08` **Yasal süreler:** yapı ruhsatı için İmar Kanunu m.22 (15/30 gün, düzeltmeden sonra 15 gün), yapı
  kullanma izni için m.30 (30 gün). Belge listesi yapı ruhsatında yasadandır, öbür türlerde önerilir.
- [x] `BUL-09` **Web portalı (`apps/portal`):** üç yüz (sistem yönetimi, belediye, meslek mensubu), açık ve koyu tema,
  Türkçe tarayıcıyla alınmış 38 ekran görüntüsü, tohum betiği.

- [x] `BUL-13` **Yüklenen dosyanın taranması** (TODOS.md CLOUD-06): tür listesi ve içerik imzası, programın, betiğin,
  makronun ve arşivdeki programın reddi, EICAR, isteğe bağlı antivirüs servisi (`KENTOS_AV_ADDRESS`, INSTREAM).
  Reddedilen dosya saklanmaz, denetim kaydına düşer; saklanan dosya `Tarandı` rozetini taşır (ADR 9000 §14).
- [x] `BUL-14` **Büronun ortak dosyası:** sorumlu meslek mensubu dosyayı bürosuyla paylaşır ya da büro adına başvurur;
  büroda düzenleyici hazırlar ve yazışır, proje yöneticisi gönderir ve teslim eder; büronun kendi dosya sayfaları
  (ADR 9000 §18).
- [x] `BUL-19` **Masaüstünde portal dosyaları:** Dosya › Bulut › Portal dosyaları; aktarılan projeyi KentOS'ta açmak,
  açık projeyi pakete eklemek, adım atmak, yazışmak (ADR 9001).
- [x] `BUL-23` **Programların girişi:** `POST /v1/auth/token` taşıyıcı oturumu (masaüstü, mobil, ajan), kişisel erişim
  anahtarları (`read`/`write`, 1–365 gün), açık oturumların listesi ve kapatılması; portalda Hesabım (ADR 9000 §12).
- [x] `BUL-24` **Bulut dosyaları:** hesabın parça parça yüklemesi (`/v1/uploads`, 512 MiB, kaldığı yerden sürer),
  projenin ekleri, `ETag`/`Range`/`?inline=1` indirme; başvuru belgeleri de bu yoldan (ADR 9000 §13, §15).
- [x] `BUL-25` **Çizimin resmi:** `preview.svg` projenin ve dosyadaki KCAD belgesinin çizimini sunucuda SVG'ye çevirir;
  portalda proje sayfası çizim, ekler ve revizyonlarla (ADR 9000 §15).
- [x] `BUL-11` **E-posta ve SMS bildirimi:** sistem yöneticisi kanalı açar, kişi Hesabım'da seçer (e-posta yalnız
  doğrulanmış adrese); giden kutusu, `sendmail` / imzalı webhook / klasör, yeniden deneme. Yeni bağımlılık yok; doğrudan
  SMTP ve hazır SMS sağlayıcıları sahibin onayına kalır (ADR 9000 §17).
- [x] `BUL-26` **API'nin açıklaması:** `GET /v1/openapi.json` (OpenAPI 3.1, 120 işlem), `GET /v1/llms.txt`,
  `docs/api/README.md`; testler kataloğu yönlendiriciyle eşit tutar (ADR 9000 §16).

- [x] `BUL-22` **Harç ve ücret** (sahibin 18 Kasım isteği: “parametrik olsun, gelişime açık olsun”):
  - Tarife veridir: girdiler, dilimli birim tutar tabloları, maktu / birim başına / formül kalemleri, koşul, sınır,
    KDV. 2464'e göre başlangıç tutarsız gelir; yeni yıl katsayıyla kopyalanır.
  - Tahakkuk, ödeme ve iptal dosyaya bağlıdır; bildirim ve geçmiş kaydı tutulur.
  - Ek Madde 5 kapısı: ödenmeden onay yok.
  - Portalda kurumun tarife sayfası ve dosyanın sekmesi (ADR 9003).

- [x] `BUL-21` **E-imza** (sahibin 18 Kasım isteği: platformdaki e-imza, mantık aynı, arayüz KentOS'un):
  - İmza servisi, PIN penceresi ve masaüstü ADR 9002'de.
  - Sunucu belgelerin özetini kendi deposundan yazar, isteği Ed25519 ile imzalar, teslimi kendi kaydına ve belgenin
    bugünkü baytlarına karşı sınar.
  - Portalda e-imzala, imza işareti ve Hesabım › E-imza var (ADR 9004).
  - Gerçek kartla tek deneme: `completed 1/1`.

- [x] `BUL-16` **Son gün tatile denk gelirse süre ilk iş gününe uzar** (İYUK m.8):
  - Hafta sonu ve 2429'un sabit günleri kodda; dinî bayramlar tabloda, yönetici yıl yıl ekler (2026–2027 hazır).
  - Arife iş günüdür; ayrıntıda `extendedFrom` ve gerekçe yer alır (ADR 9000 §19).

- [x] `BUL-12` **Makinenin açtığı hesaba karşı kayıt:**
  - Adres başına deneme ve hesap sınırı (`KENTOS_SIGNUP_PER_HOUR`).
  - Görünmez bal küpü alanı.
  - Sunucunun imzaladığı, tek kullanımlık, en erken 3 saniye sonra geçen form bileti (ADR 9000 §20).

- [x] `BUL-18` **Kişisel verilerin saklanması:**
  - Kapanan dosya süresi dolunca arşive geçer, silinmez.
  - Gönderilmemiş eski taslak belgeleriyle, eski bildirimler ve teknik kayıtlar her gün imha edilir.
  - Her çalışma kayıtta; süreleri yönetici değiştirir (ADR 9000 §21).

## Sıradakiler

- [ ] `BUL-10` Portala kurum kimliğiyle giriş (OpenID Connect; sunucu tarafı ADR 0007'de hazır).
- [ ] `BUL-15` Aktarımda kopya yerine canlı projede misafir düzenleyici (ADR 0035) seçeneği.
- [ ] `BUL-17` Gerçek zamanlı bildirim (WebSocket); şimdi dakikada bir sorulur.
- [ ] `BUL-20` Belediyenin istatistikleri ve dönem raporları (PDF ve Excel).
- [ ] `BUL-27` Resmî yazı (kararın antetli PDF'i), e-tebligat / KEP bağlantısı (kurumun anlaşması gerekir) ve
  e-imzanın yasal denetimi (Kamu SM zinciri, OCSP / SİL, zaman damgası, CAdES / PAdES çıktısı).

## Çalıştırma

- **Sunucu:** Göçler 0043–0047 `kentosd migrate` ile uygulanır. Yönetici `kentosd platform-admin add --user GİRİŞ` ile
  atanır. Antivirüs servisi isteğe bağlıdır: `KENTOS_AV_ADDRESS=tcp://ana-makine:3310` (ya da `unix:///yol`),
  taranamayan dosyayı geri çevirmek için `KENTOS_AV_REQUIRED=1`. Bildirimler: `KENTOS_MAIL_TRANSPORT`
  (`sendmail`, `webhook`, `folder`), `KENTOS_SMS_TRANSPORT` (`webhook`, `folder`), `KENTOS_MAIL_FROM`,
  `KENTOS_SENDMAIL`, `KENTOS_NOTIFY_WEBHOOK` ve `KENTOS_NOTIFY_WEBHOOK_SECRET`, `KENTOS_NOTIFY_FOLDER`.
- **API'nin açıklaması:** uç ya da sözleşme değişince
  `KENTOS_WRITE_OPENAPI=1 cargo test -p kentos-api --bin kentosd openapi` (`docs/api/openapi.json`'ı yeniden yazar).
  Kılavuz: `docs/api/README.md`.
- **Portal:** `pnpm portal` portalı 5180'de açar; `/v1` istekleri `KENTOS_API_PORT`'taki sunucuya gider.
- **Örnek veri ve görüntüler:** Temiz bir veritabanında
  `pnpm -C apps/portal seed` (`PORTAL_ADMIN_PASSWORD`, `PORTAL_DEMO_PASSWORD` ile), ardından `pnpm -C apps/portal
  shots`. Görüntüler `.run/gui-evidence/portal/` altına yazılır.
- **Sınamalar:**
  - `cargo test -p kentos-application --lib portal`
  - `cargo test -p kentos-application --test portal` (veritabanı ister)
  - `pnpm portal:test`
  - `cargo test -p kentos-api --bin kentosd cloud_files access_tests openapi` (bulut dosyaları, anahtarlar, açıklama)
  - `cargo test -p kentos-application --lib scan preview` (tarama ve çizim)
