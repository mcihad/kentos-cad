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

## Sıradakiler

- [ ] `BUL-10` Portala kurum kimliğiyle giriş (OpenID Connect; sunucu tarafı ADR 0007'de hazır).
- [ ] `BUL-11` E-posta ya da SMS ile bildirim. Gönderim kanalı ve kurumun ayarı sahibin kararı.
- [ ] `BUL-12` Kayıtta hız sınırı ve otomatik kayıt koruması (girişin sınırıyla birlikte, ADR 0007).
- [ ] `BUL-13` Yüklenen dosyanın taranması ve karantinası (TODOS.md CLOUD-06).
- [ ] `BUL-14` Büro kurumunda (category `office`) meslektaşların ortak dosyası: başvuruyu bir kişi değil, büro yapar.
- [ ] `BUL-15` Aktarımda kopya yerine canlı projede misafir düzenleyici (ADR 0035) seçeneği.
- [ ] `BUL-16` Son günün resmî tatile denk gelmesinde sürenin uzaması (kaynakla birlikte).
- [ ] `BUL-17` Gerçek zamanlı bildirim (WebSocket); şimdi dakikada bir sorulur.
- [ ] `BUL-18` Kişisel verinin saklama süresi ve kapanan dosyaların arşivi (KVKK).
- [ ] `BUL-19` Masaüstü istemcide portal dosyaları: aktarılan projeyi KentOS'tan açmak, teslimi KentOS'tan göndermek.
- [ ] `BUL-20` Belediyenin istatistikleri ve dönem raporları (PDF ve Excel).
- [ ] `BUL-21` Resmî yazı, e-imza ve e-tebligat bağlantısı. Kapsamı ve kurumun kabulü sahibin kararı.
- [ ] `BUL-22` Harç ve ücret (sahibin mesajındaki “para yüzü” bir ödeme yüzü demekse). Kapsamı sahibin kararı.

## Çalıştırma

- **Sunucu:** Göç 0043 `kentosd migrate` ile uygulanır. Yönetici `kentosd platform-admin add --user GİRİŞ` ile atanır.
- **Portal:** `pnpm portal` portalı 5180'de açar; `/v1` istekleri `KENTOS_API_PORT`'taki sunucuya gider.
- **Örnek veri ve görüntüler:** Temiz bir veritabanında
  `pnpm -C apps/portal seed` (`PORTAL_ADMIN_PASSWORD`, `PORTAL_DEMO_PASSWORD` ile), ardından `pnpm -C apps/portal
  shots`. Görüntüler `.run/gui-evidence/portal/` altına yazılır.
- **Sınamalar:**
  - `cargo test -p kentos-application --lib portal`
  - `cargo test -p kentos-application --test portal` (veritabanı ister)
  - `pnpm portal:test`
