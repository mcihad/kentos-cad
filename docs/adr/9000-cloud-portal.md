# ADR 9000: Bulut portalı — belediyeler, meslek mensupları ve platform yönetimi

- **Durum:** kabul edildi (bulut-portal dalı)
- **Tarih:** 2026-11-17
- **Bağlam belgesi:** CLAUDE.md §13, §16; ADR 0007, 0015, 0028, 0031, 0035; TODOS.md §12 (CLOUD-08, CLOUD-23, CLOUD-24)

## §1 Bağlam

Sahibin isteği (17 Kasım 2026; yazım düzeltilerek):

> “Şimdi senden yeni bir dal açmanı ve bu dal üzerine bulut sistem ile ilgili geliştirmeleri yapmayı istiyorum: bulut
> tarafının uygulamasını ve web tarafını geliştireceğiz. Şimdilik OIDC yerine normal giriş olacak. Daha önce dediğim
> gibi çok kiracılı bir sistem olacak. Buradaki yönetici arayüzünü, kiracı arayüzünü ve dışarıdan giren mimar, haritacı,
> mühendis arayüzlerini tek tek tasarlamanı istiyorum; web tabanlı. Bunların birbirleri arasında bilgi verip paylaşımı
> olacak: belediye projeyi mühendise aktarabilecek ya da mühendis proje olarak gönderebilecek.”

Sahip ayrıca uygulamanın üretimde kullanılmadığını, göç dosyalarının gerektiğinde baştan kurulabileceğini söyledi. Bu
çalışma yine de mevcut göçlere dokunmadı: yeni göç 0043 eklendi, önceki 42 göç olduğu gibi kaldı. Böylece `main` dalının
sonraki göçleriyle birleşmek kolay kalıyor.

KentOS sunucusunda kurum (tenant), üyelik, koltuk, proje yetkisi, davet ve misafir zaten vardı (ADR 0007, 0015, 0035).
Eksik olan, bunların üstünde üç tarafın birlikte çalıştığı yüzdü:
- platformu işletenlerin yönetimi;
- belediyelerin başvuru ve iş akışı;
- belediyeye üye olmayan mimar ve mühendislerin hesabı ve dosyaları.

## §2 Kişiler ve hesaplar

- **Platform yöneticisi:** hiçbir kurumda rol değildir; `kentos.platform_admin` tablosundadır. Yalnız
  `kentosd platform-admin add|remove|list` ile atanır. Web'den atanmaz: yönetici yetkisini bir web isteğinin vermesi
  saldırı yüzeyini büyütürdü.
- **Kurumun portal türü:** `tenant.category` ile belirlenir (`municipality`, `office`, `other`); il ve ilçe de kurumda
  tutulur. Başvuru alan ve iş aktaran yalnız belediyedir.
- **Meslek mensubu:** `kentos.professional` tablosunda meslek, oda sicil numarası, büro, telefon, il ve dizinde görünme
  tercihi tutulur. Oda, meslekten türetilir:
  - Mimar → Mimarlar Odası;
  - Harita mühendisi → Harita ve Kadastro Mühendisleri Odası;
  - İnşaat mühendisi → İnşaat Mühendisleri Odası;
  - Makine mühendisi → Makina Mühendisleri Odası;
  - Elektrik mühendisi → Elektrik Mühendisleri Odası;
  - Şehir plancısı → Şehir Plancıları Odası;
  - Peyzaj mimarı → Peyzaj Mimarları Odası;
  - Jeoloji mühendisi → Jeoloji Mühendisleri Odası.
- **Kayıt (ADR 0007'de değişiklik):** ADR 0007 “açık kayıt yok” diyordu. Sahibin “dışarıdan giren mimar, haritacı,
  mühendis” isteği üzerine kayıt yalnız meslek mensupları için ve bir ayarla açılır (`platform_setting.signup_open`).
  - Ayar varsayılan olarak kapalıdır; sistem yöneticisi açar ve her değişiklik denetim kaydına geçer.
  - Kayıt olan kişi oturum açar ve portalı görür. Doğrulanana kadar başvuru gönderemez, ona iş aktarılamaz.
- **E-posta:** Yöneticinin açtığı yerel hesabın e-postası doğrulanmış sayılır (göç 0010). Kendi açtığı hesabınki
  sayılmaz (`app_user.signup`). Sayılsaydı, başkasının adresini yazan kişi o adrese gelen proje davetini kabul
  edebilirdi (ADR 0035). Sistem yöneticisi kaydı doğruladığında hesap onaylanmış olur ve e-posta da doğrulanmış sayılır.
- **Giriş:** Şimdilik yerel hesapla yapılır (giriş adı ve parola; ADR 0007'nin bcrypt, oturum çerezi ve CSRF başlığı).
  Kayıttan sonra oturum hemen açılır. Kişi kendi parolasını değiştirirse bütün oturumları kapanır ve o istek için yeni
  bir oturum verilir. OpenID Connect sunucuda var, portala sonra eklenecek (§11).

## §3 Yönetim

- **Değişiklik yolu:** Hesaplara, üyeliklere ve koltuklara sunucunun rolü yazamaz (göç 0001). Portalın her yönetim
  işlemi bir `security definer` işlevinden geçer. İşlev çağıranı kendisi denetler (platform yöneticisi mi, kurumun
  sahibi ya da yöneticisi mi), yaptığını kaydeder ve bir kodla yanıt verir (`login_taken`, `no_seat`, `last_owner`…).
  Kodu kişinin okuyacağı Türkçe cümleye sunucu çevirir (`portal::refused`).
- **Platform kaydı:** `kentos.platform_event` tablosu şunları tutar:
  - kurum açma ve değiştirme (koltuk, askı);
  - hesap açma, kapatma, parola;
  - meslek mensubu kaydı ve kararı;
  - kayıt ayarı;
  - yönetici atama.

  Kayıt silinmez. Ekranda her satır bir cümle olarak okunur: “Sistem Yöneticisi, “Örnek Belediyesi” kurumunu açtı.”
- **Kurumun üyeleri:** Kurumun sahibi ve yöneticisi üye ekler. Her üye için yeni bir hesap ve bir koltuk açılır;
  koltuklar doluysa reddedilir.
  - Yönetici, yönetici rolüne kadar rol verir. Sahip rolünü yalnız sahip (ya da platform yöneticisi) verir.
  - Son etkin sahibin rolü düşürülemez, üyeliği kapatılamaz.
  - Koltuk sayısını platform yöneticisi değiştirir ve kullanılan koltuk sayısının altına indiremez.

## §4 Katalog: türler, belgeler, yasal süreler

Türlerin adları, belgeleri ve süreleri sunucuda tek yerdedir (`portal::catalog`, `GET /v1/portal/catalog`). İstemci
yalnız gösterir.

- **Başvuru türleri:** yapı ruhsatı, mimari proje, statik proje, mekanik tesisat projesi, elektrik projesi, yapı
  kullanma izni, aplikasyon krokisi, röperli kroki, ifraz/tevhit, plan değişikliği teklifi, diğer.
- **Aktarım türleri:** ölçü ve aplikasyon işi, proje hazırlama, proje düzeltme, teknik görüş, diğer.
- **Yasa adı geçen yerler.** Yalnız ikisi yasadandır; geri kalan belge listeleri “önerilen belgeler” olarak yazılır.
  Eksik belge gösterilir ama gönderimi engellemez; neyin gerektiğine belediye karar verir.
  - **Yapı ruhsatı:** belgeleri ve süreleri İmar Kanunu (3194) m.22'dendir. Belgeler: tapu (ya da yerine geçen belge),
    mimari proje, statik proje, elektrik ve tesisat projeleri, resim ve hesaplar, röperli kroki (yoksa ebatlı kroki).
    Süreler:
    - eksiksiz başvuru başvurudan itibaren en geç 30 günde sonuçlanır;
    - eksik ve yanlışlar 15 gün içinde bütün olarak yazıyla bildirilir;
    - giderildikten sonraki başvurudan itibaren karar en geç 15 gündedir.
  - **Yapı kullanma izni:** süresi m.30'dandır: başvuru en geç 30 günde sonuçlandırılır, aksi hâlde izin verilmiş
    sayılır.
- **Açıklamalarda anılan maddeler:** ifraz/tevhit için m.15 (“İfraz ve tevhid”), plan değişikliği için m.8 (“Planların
  hazırlanması ve yürürlüğe konulması”).
- **Süre hesabı (`portal::clock`):** Süre yalnız sıra belediyedeyken (gönderildi, incelemede) işler; Türkiye takviminde
  takvim günüyle sayılır.
  - İlk gönderimde ilk 15 gün “Eksikliklerin bildirimi”, sonrası “Karar” (30. gün) gösterilir.
  - Düzeltilmiş gönderimde 15 günlük “Karar” süresi gösterilir.
  - Son günün tatile denk gelmesi gibi genel süre kuralları uygulanmaz; bu açıkça yazılıdır (§11).
- **Meslek kısıtı yok:** Aplikasyon ve röperli krokiyi kimin yapabileceği konusunda kaynaklar çelişiyor (5368 sayılı
  Kanun ile lisanslı büroların yetkisi, yargı kararı ve TKGM yazıları). Portal hiçbir türü bir mesleğe kısıtlamaz;
  başvuranın mesleği görünür, karar belediyenindir.

## §5 Dosya: başvuru ve aktarım

Bir belediye ile bir meslek mensubu arasındaki her iş bir **dosyadır** (`kentos.dossier`). Yönü iki türlüdür:
- **başvuru:** meslek mensubundan belediyeye;
- **aktarım:** belediyeden meslek mensubuna.

| Yön | Akış |
|---|---|
| Başvuru | taslak → gönderildi → incelemede → (düzeltme istendi → gönderildi …) → onaylandı / reddedildi; karar verilene kadar geri çekilebilir |
| Aktarım | taslak → yanıt bekleniyor → üstlenildi (ya da üstlenilmedi) → teslim edildi → (düzeltme istendi → teslim edildi …) → tamamlandı; tamamlanana kadar iptal edilebilir |

- **Kim ne yapar (`portal::flow`, saf ve sınanmış):**
  - Kurumda koltuğu olan herkes dosyaları görür.
  - Düzenleyici ve üstü inceler, sorumlu atar, düzeltme ister, yazar.
  - Proje yöneticisi ve üstü karar verir (onay, ret), aktarım açar, gönderir, tamamlar, iptal eder.
  - Meslek mensubu kendi adımlarını yalnız kaydı doğrulanmışken atar; doğrulanmamışken okur.
- **Turlar:** `round`, gönderilen paket sayısıdır; hazırlanan paket `round + 1` olur. Gönderme adımı bekleyen dosyaları
  kilitler ve sonraki turu yapar. Başvuru ve teslim boş gönderilmez. Aktarım dosyasız gidebilir (yalnız iş tarifi).
- **Numara:** Dosya ilk gönderildiğinde kurumun o yılki sıradaki numarasını alır (`2026/0007`); bir danışma kilidi
  altında, aynı anda iki gönderimde de tek numara verilir.
- **Bayat adım:** Her adım, istemcinin gördüğü durumla (`expectedStatus`) gelir. Arada başkası adım attıysa çakışmadır
  (409), üzerine yazılmaz.
- **Gerekçe zorunluluğu:** Düzeltme isteme, ret, üstlenmeme ve iptal gerekçesiz atılmaz. Gerekçe yazışmaya özel türle
  geçer: “Eksik ve yanlışlar”, “Karar gerekçesi”. Düzeltme isteme penceresi m.22'nin “bir defada” kuralını hatırlatır.
- **Resmî karar değildir:** Onay ve tamamlama pencereleri şunu söyler: “Bu karar portalda dosyanın kaydına geçer; ıslak
  ya da elektronik imzalı resmî yazının yerine geçmez.” (CLAUDE.md §7: resmî çıktı ve kurum kabulü ayrı kapsamdır.)
- **Kişisel veri (KVKK):** Yapı sahibinin yalnız adı tutulur; kimlik numarası istenmez ve istemci bunu söyler.

## §6 Yazışma, geçmiş ve bildirim

- **Yazışma:** Gönderilmiş ve kapanmamış dosyada iki taraf yazışır; ileti ekleri de olur. Her iletinin yazarı ve tarafı
  yazılıdır. Dosya kapanınca yazışma da kapanır.
- **Geçmiş:** Dosyanın geçmişi (`dossier_event`) her adımı yapanın adıyla tutar ve cümle olarak okunur.
- **Bildirim (`kentos.notify_dossier`):** Kime gideceğini veritabanı işlevi seçer:
  - meslek mensubu tarafında, kendisi;
  - belediye tarafında, sorumlu atanmışsa yalnız o; atanmamışsa koltuğu olan düzenleyici ve üstü bütün üyeler;
  - adımı atana hiçbir zaman gitmez.
- **Okuma yeri:** Bildirimler şimdilik portalda okunur (zil ve sayfa); e-posta ve SMS gönderilmez (§11).

## §7 Dosyalar ve projeler

- **Saklama:** Baytlar proje dosyalarının nesne deposunda, `dossiers/{dosya}/{dosya eki}` anahtarıyla durur (ADR 0031).
  Her biri en çok 200 MB'tır ve yüklenirken SHA-256'sı alınır.
- **Okuma:** Okunan her dosyanın baytları kayıttaki özetle karşılaştırılır; tutmazsa dosya bozuk sayılıp verilmez.
- **Görünürlük:** Gönderilmemiş dosyayı yalnız ekleyen taraf görür; gönderilen dosya değiştirilmez, kaldırılamaz.
- **Projeden ekleme:** Bulut projesinin şimdiki hâli pakete şöyle girer (`project.download` ister):
  - dosya projesinin en yeni revizyonu nesne deposunda ortak edilir, kopyalanmaz;
  - veritabanı projesinin KCAD görüntüsü o an yazılır.
- **Proje olarak açma:** Dosyadaki bir KCAD, okuyanın çalışma alanında revizyonu 1 olan yeni bir dosya projesi olur
  (`project.create` ister). Projenin adına dosyanın numarası eklenir, bir kez.
  - Meslek mensubu için kişisel alanı;
  - belediyede proje yöneticisi ve üstü için kurumun alanı.

  Aktarılan proje mühendisin alanına böyle gelir, teslim edilen proje belediyeninkine böyle döner.

## §8 Satır güvenliği: taraflar birbirinin neyini görür

- **Kapsamsız işlem:** Meslek mensubunun işlemleri hiçbir zaman kurum kapsamı (`app.tenant_id`) taşımaz. Taşısaydı
  göç 0001'in kurum politikaları, ona o kurumun bütün projelerini, üyelerini ve hesaplarını açardı. Belediye üyesinin
  işlemi, dosya ona aitse kendi kurumunun kapsamına geçer.
- **Dosya:** Dosyayı her taraf kendi yanından görür (`kentos.dossier_side`). Taslağı yalnız kendi tarafı görür;
  başka kurum ve başka meslek mensubu için dosya yokmuş gibidir (404).
- **Meslek mensubu:** Belediye üyesi şunları görür (`professional_visible`):
  - doğrulanmış ve dizinde görünmeyi seçmiş meslek mensuplarını;
  - dosyası olduğu meslek mensubunu, dizinde olmasa da.
- **Belediye:** Belediyenin adı ve ili herkese açıktır; meslek mensubu başvuracağı belediyeyi seçer.
- **Sınama:** Satır güvenliği bilerek bozulduğunda (belediyenin taslağı görmesi) sınama kırmızıya döndü.

## §9 Web portalı

- **Ayrı paket:** Portal `apps/portal`'dır. CAD web uygulamasının yükünü, yani WASM ve çizim hattını almaz.
- **Ortak çatı:** Arayüz çatısı web'inkidir (`h()`, `Signal`); renk jetonları, vurgu, yazı tipleri ve marka işareti de
  web'den gelir. Portalın varsayılanı açık temadır (“Pafta”); koyu tema (“Grafit”) seçilebilir.
- **Simgeler:** Kendi küçük simge kümesi aynı elle çizilmiştir (20×20, 1,4 px). Web'in CAD simgeleri pakete girmez.
- **ADR 0228'in istisnası:** Web'e ara verilmişti; portal, sahibin bu isteğiyle bu dalda açıldı. CAD web uygulaması
  durmaya devam ediyor.
- **Çalışma yerleri:** Hesabın sahip olabileceği yerler “Çalışma yeri” seçicisindedir:
  - sistem yönetimi;
  - her kurumu;
  - meslek mensubu olarak kendi işi.
- **Sayfalar:**
  - **Sistem yönetimi:** genel bakış, kurumlar (yeni kurum, ayrıntı), meslek mensupları (doğrulama), hesaplar, denetim
    kaydı, platform ayarları.
  - **Belediye:** genel bakış, gelen başvurular, proje aktarımları (yeni aktarım), meslek mensupları dizini, üyeler,
    kurum ayarları.
  - **Meslek mensubu:** genel bakış, başvurularım (yeni başvuru), aktarılan işler, projelerim, profil.
  - **Ortak:** dosya sayfası (paketler, yazışma, geçmiş, durum, yasal süre, belge listesi, karşı taraf) ve bildirimler.
- **Dil ve görünüm (DESIGN.md):**
  - Türkçe cümle düzeni; meta satırı yerine cümle kurulur.
  - Durum hiçbir zaman yalnız renkle anlatılmaz: rozetin işareti vardır, uyarıda ⚠ bulunur.
  - Bir görünümde tek dolu vurgulu düğme olur; paneller gölgesizdir.

## §10 Ölçümler (17 Kasım 2026)

- **Paket boyutu:** Üretim paketinde portalın JavaScript'i 108 KB (gzip 32 KB), CSS'i 45 KB'tır. Yazı tipleri
  gerektiğinde iner.
- **Sınamalar:**
  - Sunucu: 9 birim sınaması (katalog, süre, akış, adlar), 6 veritabanı sınaması (kayıt ve doğrulama; tur tur başvuru;
    tarafların görünürlüğü; aktarım ve içe aktarma; bayat adım; yönetimin sınırları).
  - Portal: 5 birim sınaması (biçimlendirme ve adresler).
  - Etkilenebilecek mevcut sınamalar da geçti: kişi, erişim, kimlik, davet, yaşam döngüsü (24 sınama) ve API'nin
    oturum sınamaları (8 sınama).
- **Örnek veri:** Tohum betiği (`apps/portal/scripts/seed.mjs`) her şeyi API üzerinden, bir insanın yapacağı sırayla
  kurar:
  - 4 kurum, 14 hesap, 5 meslek mensubu kaydı;
  - 5 başvuru ve 1 taslak: düzeltmeli, onaylı, reddedilmiş, belge bekleyen;
  - 3 aktarım: teslim bekleyen, yanıt bekleyen, tamamlanmış;
  - KCAD dosyasının belediyede proje olarak açılması;
  - mühendisin aynı projeyi kendi alanına alıp teslim etmesi.
- **Ekran görüntüleri:** `apps/portal/scripts/shots.mjs` 38 görüntü alır. Her kişi giriş sayfasından klavyeyle oturum
  açar; tarayıcı dili Türkçedir.

## §11 Açık işler

Sıra ve ayrıntı [plan/bulut.md](../../plan/bulut.md)'dedir.

**Sahibin kararını bekleyenler:**
- Portala kurum kimliğiyle giriş (OpenID Connect).
- SMTP'ye doğrudan bağlantı ve hazır SMS sağlayıcıları (bugün `sendmail` ve imzalı webhook ile gider, §17).
- Resmî yazı, e-imza ve e-tebligat bağlantısı.
- Harç ve ücret (sahibin mesajındaki “para yüzü” bir ödeme yüzü demekse).

**Teknik işler:**
- Kayıtta hız sınırı.
- Canlı projeyi misafir düzenleyiciyle paylaşma seçeneği (ADR 0035), kopya yerine.
- Resmî tatilde sürenin uzaması.
- Gerçek zamanlı bildirim.
- Kişisel verinin saklama süresi.

Masaüstündeki portal dosyaları penceresi ADR 9001'de; yüklenen dosyanın taranması §14'te.

## §12 Programların girişi: taşıyıcı oturum ve kişisel erişim anahtarı

Sahibin isteği (17 Kasım 2026): “API sistemi düzgün olsun; hem masaüstünden hem de ileride yapacağımız mobil
uygulamadan bağlanacağız … yapay zekâ ajanları için çok iyi belgelenmiş olmalı.” OIDC şimdilik bekler.

- **Program oturumu:** `POST /v1/auth/token` yerel hesabın kullanıcı adı ve parolasıyla `kentos_s_` + 64 onaltılık
  rakam verir. Tarayıcının oturumuyla aynı tablodadır (`auth_session`), yalnız türü (`bearer`), programın adı
  (`client`) ve cihazın etiketi (`label`) eklenir: hesap oturumlarını tek tek görür ve kapatır (`/v1/me/sessions`).
  7 gün kullanılmayınca, en geç 30 günde biter; tarayıcınınki gibi her kullanımda kayar.
- **Kişisel erişim anahtarı:** `kentos_pat_` + 64 onaltılık rakam; adlı, `read` ya da `write`, 1–365 gün, hesap başına
  en çok 20. `read` yalnız `GET`/`HEAD` yapar (sunucu yöntemle denetler, uç uç değil: yeni bir uç kendiliğinden
  korunur). Bir anahtarla anahtar üretilmez ve parola değiştirilmez: sızan anahtar kendini çoğaltamaz, hesabı ele
  geçiremez.
- İkisinde de yalnız SHA-256 saklanır; anahtar bir kez gösterilir. Taşıyıcı istek `x-kentos-client` başlığı
  istemez: o başlık çerezi siteler arası isteğe karşı korur, taşıyıcı anahtarı tarayıcı kendiliğinden göndermez.
- `CLIENTS` listesine `mobile` eklendi. Göç 0044.

## §13 Bulut dosyaları: hesabın yüklemesi, indirmesi

Dosya projesinin revizyon yüklemesi (ADR 0031, 0045) projeye bağlıdır ve KCAD ister. Telefondaki fotoğraf, belediyeye
gidecek PDF, ajanın raporu için tek bir yol gerekiyordu:

- **Yükleme hesabındır** (`kentos.upload`, satır güvenliği: yalnız sahibi). `POST /v1/uploads` adı, boyutu ve
  SHA-256'yı bildirir; `PUT /v1/uploads/{id}?offset=N` en çok 32 MiB'lık parçaları taşır. Satır kilidi altında
  `offset` gelen baytlara eşit olmalıdır: iki parça karışmaz, yarıda kesilen parça yazılmaz, `GET` nereden
  sürdürüleceğini söyler. Dosya en çok 512 MiB, hesap başına aynı anda 20 yükleme, alınmayan yükleme 24 saatte
  silinir (saatlik temizlikte).
- **Bir hedef alır:** projenin eki ya da başvuru dosyasının belgesi, yüklemenin kimliğiyle. Ek, yüklemeyi projenin
  işlemi içinde alır (projenin kapsamı kişiyi de taşır): satır ve nesne birlikte taşınır ya da hiçbiri. Başvuru
  belgesi kaydedilemezse nesne geri taşınır, yükleme yeniden denemek için yerinde kalır.
- **İndirme bir yerde** (`apps/api/src/http/download.rs`): `ETag` = SHA-256, `If-None-Match` → 304, `Range`
  (`a-b`, `a-`, `-n`) → 206, `If-Range`, 416. `?inline=1` yalnız hiçbir şey çalıştırmayan türleri (PDF, resim,
  metin, video) yerinde gösterir; her yanıt `nosniff` ve `sandbox`'lı bir CSP taşır. Seçim: tus gibi ayrı bir yükleme
  protokolü yerine projede zaten çalışan `offset` yolu genişletildi; istemcinin öğreneceği tek bir kural kalır.

## §14 Tarama

TODOS.md CLOUD-06 ve plan BUL-13: dışarıdan gelen dosya belediyenin personeline ulaşmadan bakılmalı.

- **Tür listesi, kara liste değil:** uzantıdan tür okunur ve baytlar o türün imzasıyla başlamalıdır (`%PDF-`, PNG,
  JPEG, KCAD, DWG, SHP, SQLite/GeoPackage, LAS…). Saklanan ortam türü bulunanınkidir, istemcinin söylediği değil.
  `rapor.pdf.exe` bir `exe`'dir. Uzantısız dosya alınmaz.
- **Program hiçbir adla girmez:** `MZ`, ELF, Mach-O, `dex`, `#!`. Metin diye gelen HTML, betikli SVG (`<script`,
  `on…=`, `javascript:`), içinde program/betik adı bulunan ya da parolalı zip, makrolu ofis belgesi
  (`vbaProject.bin`) reddedilir. Arşivin adları paylaşılan zip okuyucusuyla (bomba sınırlarıyla) okunur; 256 MiB'tan
  büyük arşiv içi denetlenemediği için alınmaz.
- **Zararlı yazılım:** EICAR deneme dosyası her sunucuda bulunur (kaynakta iki parça hâlinde durur, depo kendisi
  virüs sanılmasın). `KENTOS_AV_ADDRESS` verilmişse her dosya clamd'nin INSTREAM protokolüyle (TCP ya da Unix
  soketi) taranır. `KENTOS_AV_REQUIRED=1` ile kapalı başarısızlık: servis yoksa ya da dosya sınırından
  (`KENTOS_AV_MAX_BYTES`, varsayılan clamd'nin 25 MiB'ı) büyükse dosya alınmaz, istemci 429 ile bekletilir.
- **Sonuç dürüst yazılır:** `clean` yalnız bir antivirüs taradığında; yalnız tür kuralları geçtiyse `unscanned` ve
  nedeni. Reddedilen dosya saklanmaz; gönderen 422 ve nedenini alır, sistem yöneticisinin kayıt defterine
  “dosya alınmadı” düşer (`kentos.log_refused_file`).
- Doğrudan gövdeyle gönderilen başvuru belgesi de aynı taramadan geçer. Göç 0045.

## §15 Projenin ekleri ve çiziminin resmi

- **Ekler** (`kentos.project_attachment`, CLOUD-06): rapor, pafta, çizelge, fotoğraf, kaynak dosya. Okuyan listeler
  (`project.read`), indiren indirir (`project.download`), nesneleri değiştiren ekler ve kaldırır (`feature.write`;
  arşivde 409, çöpte 410). İstenirse bir revizyona bağlanır. Proje sonsuza dek silinince nesneleri de gider.
- **Çizimin resmi:** `GET …/preview.svg` çizimi sunucuda tek SVG'ye çevirir (`kentos_application::preview`):
  telefonun, web sayfasının ve ajanın CAD görünümü yok. Çizgi, yol (yaylar örneklenir), çember, yay, elips, eğri,
  yazı, blok (yerleştirilerek, en çok 8 kat), tarama, duvar (gövdesinin ortasından, kalınlığıyla; hizası ve kayması
  uyulur), mekân (adıyla), kolon çizilir; gizli katman atlanır, tema renkleri kâğıdın mürekkebine çevrilir. Ölçü,
  tablo, resim, nokta bulutu, açıklık çizilmez ve sayılır (`x-kentos-skipped`). Aynı belge aynı baytları verir.
  Dosya projesinin resmi revizyonu ve genişliğiyle bir kez çizilip saklanır; veritabanı projesininki her istekte,
  `ETag` veri sürümüyle (değişmemişse 304). Form ile veri giren (Veri girişi) projenin bütün çizimini görmez.

## §16 API'nin açıklaması

- `GET /v1/openapi.json`: OpenAPI 3.1, şemalar paylaşılan sözleşmelerden JSON Schema 2020-12 olarak (schemars).
  Kaynağı uç kataloğu (`apps/api/src/http/openapi/catalog.rs`); her ucun Türkçe özeti, açıklaması, kimin
  çağırabileceği, sorgu parametreleri, gövdesi ve yanıtı orada.
- `GET /v1/llms.txt`: ajanın ilk okuyacağı kısa metin (llmstxt.org biçimi): giriş, kurallar, sık akışlar.
- `docs/api/README.md`: insan için kılavuz (curl örnekleri, mobil ve masaüstü notları).
- İki dosya da sunucuya gömülüdür. Testler yönlendiricinin kaynağındaki her `.route(` ile katalogu karşılaştırır
  (eksik ya da fazla uç kırmızı) ve depodaki açıklamanın üretilenle aynı olduğunu denetler.

## §17 E-posta ve SMS bildirimi

Plan BUL-11; sahibin isteği: “diğerlerini yapalım” (e-posta/SMS bildirimi bu listedeydi). Kanalın kendisi (kurumun posta
sunucusu, SMS sağlayıcısı) kurumun ayarıdır; kod hiçbirine bağlanmaz ve yeni bağımlılık eklemez.

- **Tek giriş noktası:** her bildirim `kentos.notification`'a yazılır; oradaki tetikleyici (`kentos.enqueue_delivery`)
  teslimleri giden kutusuna (`kentos.notification_delivery`) koyar. Yeni bir bildirim türü kendiliğinden e-posta/SMS
  olur.
- **Kim alır:** sistem yöneticisi kanalı açmadıkça hiçbir şey çıkmaz (Platform ayarları → Bildirim kanalları, denetim
  kaydına geçer). Kişi kendi seçer (Hesabım → Bildirimler): e-posta varsayılan olarak açık ama **yalnız doğrulanmış
  adrese** (doğrulanmamış adres, onu kim yazdıysa odur); SMS kapalı, numara verip açana. Kapalı ya da askıdaki hesaba
  gitmez. Erişim anahtarıyla tercih değiştirilmez.
- **Nasıl gider** (`apps/api/src/delivery.rs`, sunucu 15 saniyede bir kutuyu okur, 50'şer, 5 dakikalık kirayla):
  - `sendmail`: makinenin kendi posta programına (`sendmail -t -i`); kurum onu kendi posta sunucusuna yönlendirir.
  - `webhook`: kurumun ağ geçidine JSON `POST`, gövdenin HMAC-SHA256 imzasıyla (`x-kentos-signature: sha256=…`); SMS
    sağlayıcısı (her birinin kendi API'si var) kurumun küçük bir uyarlayıcısıyla bağlanır. Adres `https://` olmalı.
  - `folder`: her ileti bir dosya (`.eml`, `.sms.txt`): başka bir sistemin alması ve sınamak için.
- **İletinin kendisi:** RFC 5322; konu ve Türkçe görünen adlar RFC 2047 ile kodlanır, gövde UTF-8 base64,
  `Auto-Submitted: auto-generated`. Gövde kişiye adıyla seslenir, bildirimin metnini ve dosyanın portal bağlantısını
  verir, tercihlerin değiştirileceği yeri söyler. SMS kısa: başlık ve bağlantı.
- **Olmayınca:** gitmeyen ileti 1, 5, 15, 60 ve 240 dakika arayla yeniden denenir; altıncı denemede bırakılır ve nedeni
  saklanır. Kişi son 20 iletisini ve durumunu Hesabım'da görür.
- SMTP'ye doğrudan bağlanmak (bir kütüphaneyle) ve hazır SMS sağlayıcıları sahibin onayına kalır; `sendmail` ve
  `webhook` ikisini de bugün karşılar.

## §18 Büronun ortak dosyası

Plan BUL-14: başvuruyu yalnız bir kişi değil, büro yürütür. Kararlar:

- **Sorumlu meslek mensubu kalır.** Dosyanın `professional_id`'si imzalayan, odası denetlenen kişidir; `office_id`
  dosyayı bir büroyla (kategori `office` olan kurum) paylaşır. Belediye hâlâ o meslek mensubuyla muhatap; dosyada
  büronun adını görür (`kentos.dossier_office_name`, yalnız dosyayı görene).
- **Büroda rol yetkiyi söyler** (`Party::Colleague`): görüntüleyici okur; düzenleyici belge hazırlar, taslağı düzenler,
  yazışır; proje yöneticisi ve üstü meslek mensubunun adımlarını atar (gönderir, geri çeker, üstlenir, teslim eder) ve
  taslağı silebilir. Bu, kurumların "çalışmak düzenleyicinin, karar yöneticinin" kuralıyla aynıdır. Sorumlu meslek
  mensubunun doğrulanmış olması büronun adımları için de şarttır.
- **Paylaşımı yalnız sorumlu meslek mensubu yapar ya da kaldırır** (`PUT /v1/dossiers/{id}/office`), yalnız düzenleyici
  ya da üstü olduğu bir büroyla; başvuru baştan büro adına da açılabilir. Paylaşım ve kaldırılışı zaman çizelgesine
  geçer. Kaldırılınca büro dosyayı hemen göremez.
- **Satır güvenliği tek yerden:** `kentos.dossier_side` büro üyesini de meslek mensubu tarafı sayar; bütün dosya, belge,
  yazışma ve olay politikaları onu kullandığı için ayrı politika gerekmedi. Değiştirme ve taslak silme politikaları,
  dosya numarası veren işlev ve meslek mensubunun kaydını görme kuralı büroyu tanıyacak biçimde yenilendi (göç 0047).
- **Bildirim** sorumlu meslek mensubuna kendi alanında, büronun düzenleyici ve üstü üyelerine büronun alanında gider
  (görüntüleyiciye gitmez); e-posta/SMS de buradan türer (§17).
- **Büronun portal sayfaları:** genel bakış (sırası büroda olanlar), “Başvurular” ve “Aktarılan işler”; dosya sayfası
  büro içinden açılır. `GET /v1/tenants/{büro}/dossiers` büronun paylaşılan dosyalarını verir.

**Ölçümler (17 Kasım 2026):**

| Ölçü | Değer |
|---|---|
| Açıklanan uç (bildirimler ve büro ile) | 123 işlem, 96 yol |
| Bildirim (örnek veri, klasör aktarımıyla canlı sunucuda) | ilk tur 45 e-posta + 5 SMS dosyaya yazıldı, kalanlar sonraki turda |
| Büro testi | düzenleyici hazırlar ama gönderemez, büronun yöneticisi gönderir; bildirim meslek mensubu + düzenleyici + yöneticiye, görüntüleyiciye değil; paylaşım kalkınca büro görmez |
| Bildirim testleri | HMAC (RFC 4231 örneği), base64 ve RFC 2047; kanal kapalıyken 0 teslim; imzalı webhook, yeniden deneme ve bırakma; sendmail'e giden ileti |
| Örnek çizimlerin resmi | Konut100m2: 110 nesne çizildi, 3 ölçü atlandı; konut-3-1: 320/14; yol-ornegi: 727/0 |
| Yeni testler | erişim 2, bulut dosyaları (HTTP) 3, tarama 5, resim 3, portal +1, açıklama 3 |
| Tüm sunucu testleri | API 135 geçti (1 bellek sınırı testi yük altında zamanlamadan kırmızı, tek başına geçiyor); uygulama 239 |

## §19 Sürenin son günü tatile denk gelirse

Yasal süre (İmar Kanunu m.22 ve m.30) takvim günüyle sayılır. Son gün dairelerin çalışmadığı bir güne denk gelirse
süre izleyen ilk iş gününe uzar. Bu, sürelerin hesabındaki genel kuraldır; 2577 sayılı İYUK m.8 son günün resmî
tatile rastlamasını böyle yazar. Uzayan sürede ayrıntıdaki `clock` iki şey taşır:

- `extendedFrom`: kanunun saydığı gün;
- `basis`: neden uzadığını söyleyen cümle (“Son gün 19 Mayıs 2026 Salı (Atatürk'ü Anma, Gençlik ve Spor Bayramı) iş
  günü olmadığından süre izleyen ilk iş gününe uzar: 20 Mayıs 2026 Çarşamba …”).

Tatil günleri şöyle belirlenir:

- **Hafta sonu ve sabit tatiller koddadır.** Sabit tatiller 2429 sayılı Kanun'un günleridir: 1 Ocak, 23 Nisan,
  1 Mayıs, 19 Mayıs, 15 Temmuz, 30 Ağustos, 29 Ekim. Yıldan yıla değişmezler.
- **Dinî bayramlar tablodadır** (`kentos.holiday`, migration 0050):
  - Ay takvimiyle kaydıkları için her yıl Diyanet'in açıkladığı günler yazılır. 2026 ve 2027 göçle gelir.
  - Sistem yöneticisi Platform ayarları › Tatil takvimi'nden ekler ve siler. Yazma, yöneticiyi denetleyen ve
    platformun kaydına geçen iki işlevden geçer (`holiday_set`, `holiday_remove`).
  - Sunucu tabloyu açılışta ve her değişiklikte belleğe okur.
- **Arife yarım gündür ve süre için iş günüdür:** sabahı çalışılır. İhtiyatlı okuma budur: Kanun'un uzatmadığı bir
  süreyi belediyeye kaçırtmaz.
- **İdari izin tatil değildir:** süreyi uzatmaz, tabloya yazılmaz.

Ölçülenler:

- `holidays` sınamaları: hafta sonu, 29 Ekim ve bayram uzatır; arife uzatmaz. Kurban Bayramı'nın ilk günü 27 Mayıs
  2026'dan 1 Haziran Pazartesi'ye; Ramazan Bayramı'nın ikinci günü 21 Mart'tan 23 Mart'a.
- `clock` sınaması: düzeltilmiş başvurunun on beşinci günü 19 Mayıs 2026'dan 20 Mayıs'a.
- Gerçek PostgreSQL'de: yalnız yönetici yazar (403); eklenen gün takvime hemen girer; sabit gün eklenmez; silinen gün
  çıkar; iki değişiklik de platformun kaydında.
- Portal sınamalarının “15 gün kaldı” beklentisi aynı takvimle hesaplanır. Sabit 15 beklemek, on beşinci günü hafta
  sonuna denk gelen bir günde sınamayı kırardı.

## §20 Makinenin açtığı hesaba karşı kayıt

Kayıt açıkken giriş sayfası herkese açıktır. Bir betik saniyede onlarca hesap açabilir, bu da doğrulama kuyruğunu
doldurur ve sahte meslek mensubu üretir. `POST /v1/portal/signup` dört kapıdan geçer (`http/signup_guard.rs`):

- **Deneme sınırı:** bir adresten 10 dakikada en çok 20 kayıt denemesi; aşılırsa 429 ve `Retry-After`.
- **Bal küpü:** formda insanın görmediği bir `website` alanı vardır. Ekranın dışındadır ve sekmeyle gelinmez.
  `display: none` kullanılmaz, çünkü bazı betikler gizli alanı tam da bundan tanır. Doluysa istek makinenindir ve
  ayrıntı verilmeden reddedilir.
- **Formun bileti:** `GET /v1/portal/signup` açıkken bir bilet verir: zaman, tek kullanımlık nonce ve sunucunun ikisi
  üzerine HMAC-SHA256'sı. Kayıt bileti şu koşullarla geri getirir:
  - en erken 3 saniye sonra (insan okur ve yazar, betik hemen gönderir);
  - en geç 2 saat sonra;
  - bir kez.

  Gizli anahtar süreç açılırken üretilir. Sunucu yeniden başlarsa açık form yeni bilet ister; portal bunu kendiliğinden
  alır.
- **Hesap sınırı:** bir adresten saatte en çok `KENTOS_SIGNUP_PER_HOUR` (varsayılan 5) hesap.

Sınırlar adrese göre sayılır. Adres bağlantının kendisidir. `X-Forwarded-For` yalnız bağlantı aynı makinedeki bir ters
vekilden (loopback) geliyorsa okunur, başkasının başlığına inanılmaz. Sayaçlar giriş sınırı gibi sürecin belleğindedir
(ADR 0007): yeniden başlatma onları unutur, birden çok süreç ayrı sayar.

Ölçülenler:

- **Bilet** (`signup_guard` sınaması): 1 saniyede “çok çabuk”, 3 saniyede geçer, ikinci kez geçmez, 2 saati aşınca
  “süresi doldu”. Başka sunucunun bileti ve değiştirilmiş bilet geçmez.
- **Sınır** (`limit` sınaması): adres başına ayrı sayılır, pencere bitince yeniden başlar.
- **Uç düzeyinde, veritabanına gitmeden** (`http::tests`):
  - biletsiz kayıt 422 döner ve hata `ticket` alanına bağlanır;
  - dolu bal küpü 422 döner;
  - 20 denemeden sonra 429 ve `Retry-After`.
- **Demo:** örnek veri (seed) biletleri önce alıp 3 saniye bekler, sonra kaydeder. Demo ortamında saatlik hesap
  sınırı 50'dir.

## §21 Kişisel verilerin saklanması ve periyodik imha

Portal kişisel veri tutar: meslek mensubunun kaydı, yapı sahibinin adı, yazışmalar, belgeler, bildirimler, e-posta ve
SMS kayıtları, imza sertifikaları. KVKK (6698 sayılı Kanun m.7) amacı biten veriyi siler. Kişisel Verilerin
Silinmesi, Yok Edilmesi veya Anonim Hâle Getirilmesi Hakkında Yönetmelik iki şey ister:

- periyodik imha, iki imha arası en çok 6 ay (m.11);
- imha işlemlerinin kayıtları en az 3 yıl saklanır (m.7).

Bu yüzden ayrım şöyledir:

- **Dosya bir idari kayıttır, silinmez.** Kapanmış dosya (onay, ret, geri çekme, tamamlanma, iptal) süresi dolunca
  (varsayılan 365 gün) **arşive** geçer (`archived_at`). Çalışma listelerinden çıkar, “Arşiv” sekmesinde durur, her
  zaman açılır. Saklama süresi kurumun arşiv mevzuatının işidir.
- **Amacı biten imha edilir:**
  - hiç gönderilmemiş ve dokunulmamış taslak (180 gün), belgeleriyle; belgelerin baytları da depodan silinir;
  - eski bildirim (180 gün);
  - teknik kayıtlar (90 gün): tamamlanmış e-posta / SMS gönderimleri ve imza bırakmamış imza istekleri. İmzası olan
    istek imzayla birlikte durur.
- **Süreleri sistem yöneticisi değiştirir** (Platform ayarları › Kişisel verilerin saklanması). Değişiklik platformun
  kaydına geçer.
- **Sunucu imhayı her gün çalıştırır.** İlk çalışma açılıştan bir saat sonradır. Yönetici elle de çalıştırabilir.
- **Her çalışma `kentos.retention_run`'a yazılır:** zaman, günlük mü elle mi (ve kimin elinden), her türden kaç kayıt
  ve hangi sürelerle. Kayıt silinmez.
- **İş tek bir yetkili işlevdedir** (`kentos.retention_sweep`). Kurumlar arası çalışır; silinen belgelerin depo
  anahtarlarını sunucuya döndürür.

Ölçülenler (`tests/portal.rs` `personal_data_past_its_period_is_archived_or_destroyed_and_each_run_is_recorded`):

- 400 günlük kapanmış dosya arşive geçti; çalışma listesinden çıktı, `archived=true` listesinde göründü.
- 200 günlük taslak belgesiyle imha edildi, baytı depodan silindi. Bu haftanın taslağı kaldı.
- 200 günlük bildirimler silindi.
- Elle çalıştırmak yalnız yöneticinin işi (403), 5 günlük süre reddedildi.
- Günlük çalışma ardından bir şey bulmadı. İki çalışma da kayıtta, yenisi önce.

## §22 Belediyenin istatistikleri ve dönem raporu

Belediye, gelen ve karara bağlanan başvuruları, sürelerini ve açık işleri bir dönem için görmek ve dönem raporu
almak istiyor (“bu yıl kaç ruhsat, ortalama kaç günde?”). `GET /v1/tenants/{t}/portal/statistics?from&to` kurumun
üyelerine (görüntüleyici dahil) şunları verir:

- dönemde ilk kez gönderilen başvurular;
- karara bağlananlar (onay, ret), geri çekilenler;
- karar süresi: ilk gönderimden karara kadar gün olarak, ortalama ve ortanca;
- ortalama tur sayısı;
- şimdi açık olanlar, ve bunlardan yasal süresi geçenler: dosyanın sayfasındaki saatle aynı hesap, tatil takvimi dahil;
- gönderilen ve tamamlanan aktarımlar;
- dönemin harç tahakkuku ve tahsili;
- türlere, aylara ve sorumlulara göre dağılım.

Dönem Türkiye takvimiyle iki ucu dahil günlerdir. Varsayılan bu yılın başından bugünedir, en çok üç yıldır.

- **Çalışma kitabı sunucudadır:** `…/statistics/export?format=xlsx|ods|csv`.
  - XLSX ve ODS beş sayfadır: Özet, Türlere göre, Aylara göre, Sorumlulara göre, Dosyalar. Dosyalar sayfası dönemin
    dosyalarını sürelerle birlikte verir.
  - CSV dönemin dosyalarıdır (UTF-8 BOM, noktalı virgül).
  - Yazıcı ortaktır: masaüstünün tablo dışa aktarımıyla aynı `kentos_formats::workbook::write`.
- **PDF tarayıcının yazdırmasıdır.** Sayfanın yazdırma görünümü portalın çerçevesini ve düğmeleri gizler, başa
  kurum ve dönem satırını koyar. “Yazdır / PDF” tarayıcının “PDF olarak kaydet”ine gider. Sunucuda PDF üretmek
  Türkçe harfler için yazı tipi gömmeyi gerektirirdi. Paftanın PDF yazıcısı yazı tipini ana programdan alıyor, sunucuda
  böyle bir kaynak yok. Rapor insanın okuyacağı bir kâğıttır; makinenin okuyacağı biçim zaten XLSX ve CSV'dir.
- **Grafik veri görselleştirme kurallarıyla çizilir:**
  - aylara göre gelen ve karara bağlanan başvurular, tek eksen, 24 px'i geçmeyen sütunlar;
  - doğrulanmış kategorik renkler (`--viz-1`, `--viz-2`): açık ve koyu yüzeyde parlaklık, doygunluk, renk körlüğü
    ayrımı ve karşıtlık sınamaları geçti;
  - göstergede seriler, her serinin en büyük değeri sütunun üstünde yazılı;
  - her ay için fareyle ve klavyeyle açılan ipucu;
  - aynı sayılar “Tablo olarak” bölümünde de var.
  - Grafik kutusunun kendi genişliğinde çizilir: geniş ekranda yazı büyümez, telefonda küçülmez.

Ölçülenler:

- Gerçek PostgreSQL'de (`tests/portal.rs` `a_municipality_s_figures_for_a_period_and_its_workbook`):
  - görüntüleyici okur;
  - 3 gelen, 2 karar (1 onay, 1 ret), 1 açık;
  - mimari proje satırı 2/1/1;
  - bu ayın sütunu 3/2, ilk ay Ocak;
  - XLSX `PK` ile başlar;
  - CSV bir başlık ve üç dosya satırıdır.
- Ekran görüntüleri (`.run/gui-evidence/portal/101–105`, demo verisi):
  - istatistik sayfası ve ipucu;
  - tatil takvimi ve saklama ayarları;
  - elle çalıştırılan imha ve kaydı;
  - arşiv sekmesi.

  Arşiv görüntüsü için demo veritabanında bir onaylı başvurunun tarihi 400 gün geriye alındı.

## §23 Bildirimler canlı

Zil, sayfa açıkken dakikada bir sorarak güncelleniyordu. Belediye eksik bildirdiğinde mimar bunu bir dakikaya kadar
geç görüyordu, açık her sekme de sunucuyu dakikada bir yokluyordu. Artık bildirim yazıldığı an gider:

- **Kaynak veritabanıdır.** `kentos.notification`'a bir satır yazılınca ya da okununca bir tetikleyici
  `pg_notify('kentos_notification', kişi)` gönderir (migration 0052). Böylece bildirim hangi yoldan yazılırsa yazılsın
  haber gider: uygulamanın kendisi, `notify_dossier` gibi yetkili bir işlev ya da başka bir sunucu süreci.
- **Sunucu tek bağlantıyla dinler** (`hub::listen`, `PgListener`). Sinyali süreç içindeki kanala (`Notices`) aktarır;
  bağlantı koparsa 5 saniye sonra yeniden kurar.
- **Kişinin açık sayfası bir SSE akışıdır** (`GET /v1/me/notifications/stream`):
  - ilk olay şimdiki okunmamış sayısıdır;
  - sonra yalnız o kişinin sinyalinde, sayı kişinin kendi yetkisiyle yeniden okunur; sinyal yalnızca kimlik taşır;
  - 25 saniyede bir canlı tutma yorumu gider;
  - geride kalan akışa sayı yeniden gönderilir.
- **Portal `EventSource` ile dinler.** Kopan akışı tarayıcı kendisi yeniden açar. Sayı artınca “Yeni bildiriminiz
  var.” der. Beş dakikalık sorma yedek olarak kalır. Masaüstü ve mobil aynı akışı kullanabilir.

Ölçülenler:

- **Gerçek PostgreSQL'de** (`http::tests::a_notification_reaches_the_person_s_open_stream_as_it_is_written`):
  akış önce `unread: 0` verir; veritabanına doğrudan yazılan bildirim tetikleyici, dinleyici ve akış üzerinden
  `unread: 1` olarak gelir.
- **Demo, portalın geliştirme sunucusunun arkasından:** ilk olay 49 ms'de geldi. Belediyenin iletisinden sonra yeni
  sayı 18 ms'de ulaştı. Ara katman akışı tamponlamıyor.

## §24 Aktarımda kopya yerine canlı proje

Belediye bir işi (ölçüm, proje düzeltmesi) aktarınca projesinin bugünkü hâli KCAD kopyası olarak gidiyor. Meslek
mensubu kopyada çalışıp yeni bir kopya teslim ediyordu. Aynı proje iki yerde ilerliyor, belediye teslimi kendi
projesine elle taşıyordu. Artık belediye işi kendi canlı bulut projesine bağlayabilir:

- **Kim bağlar:** belediyenin proje yöneticisi ve üstü, dosyanın sayfasındaki “Canlı proje” panelinden, aktarım
  kapanmadan (`PUT /v1/dossiers/{id}/live-project`). Kopya göndermek gerekmez; aktarım dosyasız gönderilebilir.
- **Erişimin yaşamı:**
  - Meslek mensubu işi **üstlenince** projede düzenleyici olur. Kurumun üyesi değilse **misafir**dir (ADR 0035), kurum
    misafir kabul etmiyorsa üstlenme anlaşılır bir hatayla durur.
  - İş tamamlanınca, iptal edilince ya da üstlenilmeyince bu paylaşım **geri alınır**.
  - Bağlantı iş üstlenildikten sonra kurulursa paylaşım hemen verilir.
- **Paylaşım dosyanınsa geri alınır** (`live_granted`). Kişinin projede zaten bir paylaşımı varsa o kendisinindir:
  dokunulmaz, iş kapanınca da kalır.
- **Kayıt ve canlı haber:** her verme ve geri alma projenin denetim kaydına geçer (`dossier.live_project.grant` /
  `.revoke`) ve projenin açık bağlantılarına `project.access` olayı gider, paylaşma penceresinin yaptığı gibi.
- **Teslim projenin kendisidir:** canlı projeye bağlı aktarımda teslim adımı dosya istemez.
- **Projenin adı iki tarafa görünür.** Meslek mensubu belediyenin projelerini okuyamaz; ad yetkili bir işlevden gelir.
  “Projeyi aç” paylaşım verilince görünür.

Ölçülenler (`tests/portal.rs` `a_transfer_in_the_live_project_makes_its_professional_a_guest_editor_until_it_closes`):

- Düzenleyici bağlayamaz (403), başka kurumun projesi bulunamaz (404).
- Gönderilince meslek mensubunun projede rolü yoktur; üstlenince `editor` olur (`guest`, denetim kaydında bir satır).
- Teslim dosyasız geçer; tamamlanınca rol kalkar.
- Kapanmış aktarımın bağlantısı değişmez.
- Ekran görüntüleri (`.run/gui-evidence/portal/106–108`): belediyenin paneli; haritacının “Misafir
  düzenleyicisiniz”i; haritacının açtığı belediye projesi.

## Kaynaklar

- OpenAPI 3.1.0 (<https://spec.openapis.org/oas/v3.1.0>), JSON Schema 2020-12, llms.txt önerisi
  (<https://llmstxt.org>).
- HTTP koşullu istekler ve aralıklar: RFC 9110 §8.8.3 (ETag), §13 (If-None-Match, If-Range), §14 (Range, 206, 416);
  taşıyıcı anahtar: RFC 6750.
- Antivirüs servisinin akış protokolü (INSTREAM) ve 25 MiB'lık varsayılan sınırı: clamd belgesi
  (<https://docs.clamav.net/manual/Usage/Scanning.html#clamd>); deneme dosyası: EICAR
  (<https://www.eicar.org/download-anti-malware-testfile/>).

- 3194 sayılı İmar Kanunu, güncel metin: <https://www.mevzuat.gov.tr/mevzuatmetin/1.5.3194.pdf>. Dayanılan maddeler:
  - m.8: planların hazırlanması ve yürürlüğe konulması;
  - m.15: ifraz ve tevhid;
  - m.22: ruhsat alma şartları (belgeler, 30 ve 15 gün, eksiklerin bütün olarak yazıyla bildirimi);
  - m.30: yapı kullanma izni (30 gün).
- Sürenin son günü: 2577 sayılı İdari Yargılama Usulü Kanunu m.8; tatiller: 2429 sayılı Ulusal Bayram ve Genel
  Tatiller Hakkında Kanun; dinî bayramların günleri: Diyanet İşleri Başkanlığı'nın yıllık takvimi.
- Kişisel veriler: 6698 sayılı Kişisel Verilerin Korunması Kanunu m.7; Kişisel Verilerin Silinmesi, Yok Edilmesi
  veya Anonim Hâle Getirilmesi Hakkında Yönetmelik m.7 ve m.11.
- Aplikasyonun tanımı ve lisanslı büroların işleri: TKGM 2010/4 ve 2023/5 sayılı genelgeler
  (<https://www.tkgm.gov.tr/sites/default/files/2020-11/2010-4_genelge.pdf>,
  <https://www.tkgm.gov.tr/sites/default/files/2025-07/2023_5.pdf>). Röperli krokinin yalnız lisanslı bürolara ait
  olmadığına ilişkin yargı yorumu araştırma notunda geçti; bu nedenle portal türleri mesleğe kısıtlamaz.
