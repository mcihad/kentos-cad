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
- E-posta ya da SMS bildirimi.
- Resmî yazı, e-imza ve e-tebligat bağlantısı.
- Harç ve ücret (sahibin mesajındaki “para yüzü” bir ödeme yüzü demekse).

**Teknik işler:**
- Kayıtta hız sınırı.
- Yüklenen dosyanın taranması (CLOUD-06).
- Büro kurumunda meslektaşların ortak dosyası.
- Canlı projeyi misafir düzenleyiciyle paylaşma seçeneği (ADR 0035), kopya yerine.
- Resmî tatilde sürenin uzaması.
- Gerçek zamanlı bildirim.
- Kişisel verinin saklama süresi.
- Masaüstü istemcide portal dosyaları.

## Kaynaklar

- 3194 sayılı İmar Kanunu, güncel metin: <https://www.mevzuat.gov.tr/mevzuatmetin/1.5.3194.pdf>. Dayanılan maddeler:
  - m.8: planların hazırlanması ve yürürlüğe konulması;
  - m.15: ifraz ve tevhid;
  - m.22: ruhsat alma şartları (belgeler, 30 ve 15 gün, eksiklerin bütün olarak yazıyla bildirimi);
  - m.30: yapı kullanma izni (30 gün).
- Aplikasyonun tanımı ve lisanslı büroların işleri: TKGM 2010/4 ve 2023/5 sayılı genelgeler
  (<https://www.tkgm.gov.tr/sites/default/files/2020-11/2010-4_genelge.pdf>,
  <https://www.tkgm.gov.tr/sites/default/files/2025-07/2023_5.pdf>). Röperli krokinin yalnız lisanslı bürolara ait
  olmadığına ilişkin yargı yorumu araştırma notunda geçti; bu nedenle portal türleri mesleğe kısıtlamaz.
