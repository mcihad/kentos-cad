# ADR 9001: Masaüstünden portal dosyaları

- **Durum:** kabul edildi (bulut-portal dalı)
- **Tarih:** 2026-11-17
- **Bağlam belgesi:** ADR 9000 §5–§8 (bulut portalı); ADR 0040 (masaüstü bulut istemcisi), 0041 (masaüstünün bulut
  arayüzü), 0086 (bulut projeleri penceresi), 8000 (gerçek pencerede kabul); plan/bulut.md `BUL-19`; DESIGN.md §7.9

## §1 Bağlam

Bulut portalı (ADR 9000) belediye ile meslek mensubu arasındaki işi bir **dosyada** tutuyor: başvuru meslek
mensubundan belediyeye, aktarım belediyeden meslek mensubuna gidiyor. Dosyanın paketleri, yazışması, geçmişi ve sırası
web portalındaydı. Oysa mimar, haritacı ve mühendis projeyi KentOS masaüstünde çiziyor: aktarılan projeyi açmak için
web portalında “Proje olarak aç”a basıp sonra masaüstünde Bulut projeleri'ni açıyor, teslim için masaüstünde kaydedip
web portalına dönüp projeyi pakete ekliyor ve oradan teslim ediyordu. ADR 9000 §11 bunu açık iş olarak yazdı
(“Masaüstü istemcide portal dosyaları”), plan/bulut.md `BUL-19` olarak.

Bu karar masaüstüne bir pencere ekliyor: kişi dosyalarını görür, birini açar, dosyadaki KentOS projesini çizim alanında
açar, açık bulut projesini pakete ekler ve teslim eder ya da gönderir; karşı tarafa yazar. Bunun için web portalına
gitmesi gerekmez.

## §2 Karar

- **Komut:** `cloud.portal`, “Portal dosyaları…”. Şeritte Dosya › Bulut panelinde büyük düğme (CAD ve CBS), Menülerde
  ara'da “Portal”. Oturum yoksa önce Buluta giriş açılır, girişten sonra pencere gelir (`account::Next::Portal`).
  Kaydı `catalog/v1/additions.json` (simge `portalDossiers`: portalın gelen kutusu), düzeyi `host`
  (`catalog/v1/operations.json`); çizimi kendisi değiştirmez, açılan proje var olan açma yolundan geçer.
- **Sunucu karar verir, masaüstü gösterir.** Dosyanın sayfası (`DossierDetail`) şimdi atılabilecek adımları
  (`actions`), pakete dosya eklenip eklenemeyeceğini (`canAddFiles`), yazışmanın açıklığını (`canMessage`) ve KCAD'in
  proje olarak açılıp açılamayacağını (`canImport`) söyler. Pencere yalnız bunları gösterir; hiçbir adımı eklemez,
  gizlemez. Durumların adı ve tonu, mesleklerin adı sunucunun kataloğundandır (`GET /v1/portal/catalog`).
- **Davranışın kaynağı web portalı** (`apps/portal/src/pages/dossier/`): düğme adları, her adımın sorusu, notun zorunlu
  olup olmadığı, kararın resmî yazı olmadığı uyarısı, paketlerin adları. Yerleşim masaüstünün tasarım sistemindendir.

## §3 Pencere

DESIGN.md §7.9'un liste ve ayrıntı kalıbı (`ListDetails`):

- **Üst satır:** “Çalışma yeri” seçimi (hesap hem meslek mensubu hem bir belediyenin üyesiyse; yalnız kullanılabilir
  belediye üyelikleri), yön (`Tümü | Başvurularım | Aktarılan işler`; belediyede `Gelen başvurular | Proje
  aktarımları`) ve arama (sunucunun `q`'su, son tuştan 300 ms sonra).
- **Liste:** numara ve yön, başlık ve altında türü ile karşı taraf, durum hapı ve sıra bu taraftaysa “Sıra sizde”
  (renk tek başına bir şey söylemez). En çok 100 dosya istenir; fazlası varsa liste altı söyler.
- **Ayrıntı:** numara satırı (“Aktarım 2026/0007, Proje düzeltme”), başlık, durum ve sıranın kimde olduğu cümlesi,
  karşı taraf ve yer (“412 ada 7 parsel”), istenen teslim günü; sunucunun izin verdiği adımlar, yanlarında; önce
  bilinmesi gerekenler (taslak, istenen düzeltmenin listesi, karar), yasal süre ve dayanağı; açıklama; dört sekme:
  - **Paket:** hazırlanan paket (yalnız hazırlayan taraf görür) ve açık projenin oradaki yeri, sonra gönderilen her
    paket, en yenisi üstte. Dosyanın satırında Proje olarak aç (gönderilmiş KCAD), Paketten çıkar (gönderilmemiş),
    İndir… (SHA-256'sı denetlenerek diske).
  - **Yazışma:** iki tarafın iletileri sırayla, “Eksik ve yanlışlar” ve “Karar gerekçesi” adlarıyla; dosya açıkken
    yazma kutusu ve Gönder.
  - **Geçmiş:** her adım bir cümle, en yenisi üstte.
  - **Bilgiler:** dosyanın bilgileri, belge listesi ve dayanağı, karşı taraf (belediyenin iletişimi ya da meslek
    mensubunun odası, sicil numarası, bürosu).
- **Düğmeler:** yüzeyde tek dolu düğme vardır: sıradaki ileri adım (Gönder, İncelemeye al, İşi üstlen, Teslim et,
  Onayla, Teslimi onayla), en sağda. Geri çeviren adımlar (Reddet, Üstlenme, İptal et, Geri çek) kırmızı çizgilidir.
  Gönder (ileti) ve Açık projeyi ekle bu yüzden çizgili düğmedir. Pencerenin alt çubuğunda yalnız kendi düğmeleri
  durur: Yenile ve Kapat; solunda son yapılan işin ya da hatanın cümlesi.
- **Kapatınca yerini korur:** seçilen yer, yön ve dosya saklanır; pencere yeniden açılınca oradan başlar. Oturum
  kapanınca unutulur.

## §4 Adımlar

- Her adım önce sorar (web portalının soruları): ne olacağı, alacağı not (Üstlenme'nin nedeni, düzeltmenin eksik ve
  yanlışları, ret ve iptal gerekçesi zorunlu), kararlarda “Bu karar portalda dosyanın kaydına geçer; ıslak ya da
  elektronik imzalı resmî yazının yerine geçmez.” uyarısı. İncelemeye al sormadan gider; Sorumlu ata kişiyi sordurur.
- Zorunlu not boşsa adım gitmez, soru neyin eksik olduğunu söyler. Adım, kişinin gördüğü durumla gider
  (`expectedStatus`): arada başkası adım attıysa sunucunun çakışması (409) sorunun içinde sunucunun cümlesiyle söylenir
  ve dosya yeniden okunur; hiçbir şey üstüne yazılmaz.
- Adımdan sonra sunucunun döndürdüğü sayfa gösterilir, liste yeniden okunur (satır yeni durumu söyler).

## §5 Paket ve açık proje

- **Proje olarak aç:** gönderilmiş bir KCAD dosyası okuyanın alanında yeni dosya projesi olur (`POST …/files/{dosya}/
  import`; meslek mensubu için kişisel alanı, belediye üyesi için kurumun alanı) ve hemen çizim alanında açılır: var
  olan açma yolu (`Then::OpenCloud`, ekrandaki çizimin sorusu, ilerleme penceresi). Pencere bu sırada çekilir; ekrandaki
  çizimden vazgeçilirse geri gelir ve “Projeyi aç” ile yeniden sunar.
- **Açık projeyi ekle:** çizim alanındaki bulut projesinin şimdiki hâli hazırlanan pakete girer
  (`POST …/files/from-project`):
  - dosya projesinde buluttaki son revizyon girer; çizimde kaydedilmemiş değişiklik varsa düğme kapalıdır ve uyarının
    Kaydet'i önce revizyonu yazar;
  - veritabanı projesinde önce gönderilmemiş değişiklikler gider (`Settle::Portal`, Yeniden adlandır ve Çöpe taşı'nın
    yolu); gidemezse eklenmez ve nedeni söylenir;
  - projede indirme yetkisi (`project.download`) yoksa ya da sunucuya ulaşılamıyorsa düğme kapalıdır, nedeni altında.
- Sonra Teslim et (aktarım) ya da Başvuruyu gönder (başvuru) paketi gönderir; boş teslimi sunucu reddeder.

## §6 İstemci

`kentos-cloud`'un `api/portal.rs`'i, öbür yollar gibi: bulutun kendi çalışma zamanında future, oturum çerezi ve
`x-kentos-client: desktop` (sunucunun CSRF kuralı), hata sunucunun cümlesiyle `ApiFailure`. Yollar: `GET /v1/portal/me`,
`GET /v1/portal/catalog`, `GET /v1/me/dossiers`, `GET /v1/tenants/{kurum}/dossiers` (sorgusu `DossierQuery`),
`GET /v1/dossiers/{id}`, `POST …/actions`, `POST …/messages`, `POST …/files/from-project`, `DELETE …/files/{dosya}`,
`GET …/files/{dosya}` (baytlar ETag'deki SHA-256'yla denetlenir, yarıda kalan indirme kaldığı yerden sürer),
`POST …/files/{dosya}/import`.

Masaüstü tarafı `apps/desktop/src/shell/cloud/portal/`:

| Dosya | İçerik |
|---|---|
| `mod.rs` | pencerenin durumu ve iletileri, açılması ve kapanması |
| `reading.rs` | çalışma yerleri, süzgeçler, hesap, liste ve dosya okumaları (geç gelen cevap düşer) |
| `plan.rs`, `words.rs`, `facts.rs` | ne gösterileceği ve sözleri (Iced yok; sınamalar okur) |
| `steps.rs` | adımın sorusu ve atılması, KCAD'in proje olarak açılması |
| `package.rs` | açık proje pakete, bekleyen dosya paketten, dosya diske |
| `messages.rs` | ileti yazma ve gönderme |
| `view.rs`, `view_tabs.rs`, `view_ask.rs` | pencere, sekmeler, soru |

## §7 Bu kararın dışında kalanlar

- Yeni başvuru ya da aktarım açmak, dosyanın bilgilerini düzenlemek, taslağı silmek: web portalında kalır.
- Diskten dosya yüklemek ve iletiye ek koymak: pakete masaüstünden yalnız bulut projesi girer. PDF gibi belgeler web
  portalından yüklenir.
- Bildirimler (zil) masaüstünde yok; liste “Sıra sizde” ile ve pencere her açılışta yeniden okunarak güncel kalır.
- Dosyanın akış şeridi (Taslak → Gönderildi → …) yok; durum hapı ve sıranın kimde olduğu cümlesi yerini tutar.

## §8 Kabulde bulunan ve düzeltilenler

Gerçek pencerede (aşağıda) görülen ve aynı işte düzeltilenler:

- **Dosyadan açılan proje “Yeni proje” adıyla açılıyordu.** Dosya projesinin çizimi revizyondaki KCAD'in kendi adını
  taşıyordu; katalogdaki ad (dosyanın numarasıyla “Konut-412-7 (2026/0001)”) görünmüyordu. `kentos-cloud`'un açışı
  artık dosya projesinin çizimine kataloğun adını verir (`open.rs` `revision_drawing`); yeniden adlandırılan dosya
  projesinin çizimi de zaten böyle sessizce ad alıyordu.
- **Soru kapanınca ayrıntı başa kayıyordu.** Pencere soru açılınca ağaçta yer değiştiriyor, listenin ve ayrıntının
  kaydırma yeri sıfırlanıyordu. Pencere artık hep bir yığının ilk katmanıdır, soru üstüne eklenir.
- **Aynı revizyon pakete iki kez eklenebiliyordu.** Dosya projesinin pakette olan revizyonu yeniden eklenmez; düğme
  kapanır, altında nedeni yazar. Veritabanı projesinin hâli her an değişebildiği için o yeniden eklenebilir.
- **650 piksel yüksek pencerede alt çubuk görünmüyordu.** Pencere en çok 720 piksel (12 piksellik yazıya göre) boyda,
  pencere alçaksa onun boyundadır; liste ve ayrıntı kendi içinde kayar.
- **Satır içi “Paketten çıkar” kırmızısını kaybediyordu** (yazının kendi rengi düğmeninkini eziyordu); “220 ada —
  parsel” yalnız ada verilmişken “220 ada” yazılır; zorunlu notun eksiği alanın altında söylenir.

## §9 Doğrulama (17 Kasım 2026)

- **Sınamalar:**
  - `kentos-cloud`: sahte sunucuyla 4 sınama (`tests/portal.rs`: adresler ve sorguları, `x-kentos-client: desktop`,
    gövdeler, 204, SHA-256'sı tutmayan dosya, çakışmanın sözü), sorgu yazımı (`api::portal`) ve dosya projesinin
    adı (`open::tests`).
  - Masaüstü: `cloud::portal` 18 sınama (7'si sözler ve kararlar, 11'i ileti ileti pencere: hesap, liste, seçim,
    soru, zorunlu not, bayat adım, proje olarak açma, açık projenin pakete girmesi ve iki kez girmemesi, veritabanı
    projesinin önce gönderilmesi, ileti, kapatıp yerinden açma, geç gelen cevap). Pencere resimleri `#[ignore]`
    `cloud::portal::screens`: altı sahne, koyu ve açık, 1440×900 ve 1100×650 (24 resim).
- **Gerçek pencere** (ADR 8000; Xvfb, gerçek fare ve klavye, sahibin ayarlarının kopyası; `kentosd` ayrı veritabanında,
  örnek veriyle). Resimler `.run/gui-evidence/desktop-portal/kabul/`:
  - mimar (Ayşe Yılmaz): şeritten Portal dosyaları, giriş, beş dosya; aktarılan işi notla üstlenme; 2026/0001'in
    KCAD'ini proje olarak açma (“Konut-412-7 (2026/0001)”, 113 nesne); proje açıkken pencere, Açık projeyi ekle,
    teslim notuyla Teslim et; yazışmaya ileti; Geçmiş; aynı projeyle düzeltme istenen başvuruyu gönderme;
  - belediye proje yöneticisi (Ali Çelik): kurumun sekiz dosyası, teslim edilen işin adımları (Düzeltme iste, Sorumlu
    ata, İptal et, Teslimi onayla), kararın uyarısı, KCAD'in kurum alanında açılma sorusu, boş zorunlu notun alanda
    söylenmesi (adım atılmadı);
  - koyu tema, 16 piksel yazı, 1100×650: giriş, pencere, Bilgiler.
- **El hareketi** (`scripts/usage/real/count.py`): portal penceresi açıkken aktarılan işi notla üstlenmekten teslime
  (üstlen, KCAD'i proje olarak aç, pencereyi yeniden aç, açık projeyi ekle, Teslim et) 11 tık, 10 tekerlek adımı (eski
  turdaki KCAD'e inmek) ve iki not: KLM 57 s, çoğu notların yazımı. Düzeltilmiş başvuruyu aynı projeyle göndermek
  6 tık ve bir not: KLM 22 s.
- **Gerçek pencerede sınanmayanlar:** İndir… (sistemin kaydetme penceresi oturum veriyolu ister, kabul ortamında yok;
  baytların SHA-256 denetimi istemcinin sınamasında), kaydedilmemiş dosya projesinin Kaydet uyarısı ve veritabanı
  projesinin önce gönderilmesi (ileti ileti sınamalarda), Paketten çıkar, belediyenin adımları (yalnız soruları
  açıldı; demo verisi değişmesin diye atılmadı).
