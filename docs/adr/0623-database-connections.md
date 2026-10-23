# 0623 — Veritabanı bağlantıları: bağlantı penceresi, Kaynaklar'da bağlantı ağacı, parola

Tarih: 23 Ekim 2026. Dal: `ux/arayuz-duzeni`. Kapsam: masaüstü, başsız sunucu, Python ve MCP (bağlantılar ve parolalar
cihazdadır; web ADR 0228 gereği bekler, gerekenler TODOS.md §0.2'de).

## Sahibin isteği

Sahip (23 Ekim) PostgreSQL bağlantısını "çok kötü" buldu: "kullanacak insanlar veritabanı yöneticisi değil, bağlantı
dizesi oluşturmasını bekleyemezsiniz". QGIS'in bağlantı penceresini ve bağlantı yönetimini örnek gösterdi. Sonra şunu
ekledi: "QGIS üzerinden almayacak, onun gibi olacak". Yani QGIS'ten bağlantı alınmaz ve QGIS'e verilmez; KentOS'un
kendi bağlantı yönetimi QGIS'inki gibi çalışır.

Önceki durum (ADR 0505): PostGIS'e bağlan penceresine `postgresql://kullanıcı@sunucu:5432/veritabanı` adresi elle
yazılıyordu. Parola yalnız `PGPASSWORD` ortam değişkeninden ya da `~/.pgpass`'tan okunuyordu. Tablo listesi her
tabloyu `count(*)` ile baştan sona sayıyordu; büyük bir belediye veritabanında bu dakikalar sürer.

## Karar

### 1. Bağlantı cihazda bir kayıttır

Bağlantının alanları şunlardır (`kentos_postgis::connections::DbConnection`):

- ad;
- sunucu, bağlantı noktası (5432), veritabanı, kullanıcı adı;
- SSL kipi: libpq'nun altı kipi, varsayılanı “Varsa kullan” (`prefer`);
- yalnız kullanıcı isterse parola;
- iki liste seçeneği: “Yalnız public şemasını listele”, “Geometrisiz tabloları da listele”.

Bağlantılar `$XDG_CONFIG_HOME/kentos-cad/veritabani-baglantilari.json` dosyasındadır (yoksa `~/.config/kentos-cad/`).
Bu yer harita servislerinin gizli değerleriyle aynıdır (ADR 0208 §12). Dosyanın biçimi `kentos.db-connections`
sürüm 1'dir ve yalnız kullanıcı okuyabilir (0600). Okunamayan dosya `…-okunamadi-<zaman>.json` adıyla kenara
ayrılır, üzerine yazılmaz. Ad cihazda tekildir; karşılaştırmada Türkçe büyük-küçük harf ayrımı yapılmaz.

Bağlı katmanın tuttuğu adres alanlardan üretilir (`DbConnection::address`):

- kullanıcı ve veritabanı yüzde kodlanır;
- bağlantı noktası her zaman yazılır;
- SSL kipi varsayılandan farklıysa eklenir;
- parola hiçbir zaman adrese yazılmaz.

Adresin kuralları ADR 0505'tekilerdir (`address_problem`).

### 2. Parolanın yeri ve sırası

Komut bir veritabanına bağlanırken parola bu sırayla aranır (`connections::password_for`, sürücünün `connect`'i):

1. bu oturumda yazılmış parola (`remember`; kaydedilmeyen, program kapanınca unutulan);
2. cihaza kaydedilmiş parola: sunucu (büyük-küçük harf ayrımı yok), bağlantı noktası, veritabanı ve kullanıcı
   eşleşmeli; kullanıcısı boş bağlantı her kullanıcıya uyar ama kullanıcıyı adıyla söyleyen bağlantı önce gelir;
3. `PGPASSWORD`;
4. `~/.pgpass` (sqlx adresi okurken uygular).

Bu yüzden çizim hiçbir zaman parola taşımaz. Aynı bilgisayarda Python ve MCP sunucusu da masaüstünün kaydettiği
parolayı bulur; tıpkı `~/.pgpass` gibi kullanıcının kendi dosyasıdır. Testler bağlantıları bellekte tutar
(`keep_in_memory`; masaüstünde her test uygulamasının kendi deposu vardır).

### 3. Bağlantıyı dene ve hatanın dili

`probe::probe` bağlantıyı pencerenin alanlarıyla dener (10 saniye). Başarılıysa şunları söyler:

- PostgreSQL'in sürümü;
- PostGIS'in sürümü, kurulu değilse bunu ve yöneticinin ne yapacağını (`CREATE EXTENSION postgis;`);
- görülebilen şemaların ve tabloların sayısı.

Hata düz Türkçeyle ve ne yapılacağıyla söylenir (`probe::diagnose`). Sınıflandırma SQLSTATE'ten, G/Ç hatasının
türünden ve sözlerinden yapılır:

| Durum | Söz |
|---|---|
| sunucu bulunamadı | adı ya da IP'yi, VPN'i denetleyin |
| bağlantı reddedildi | PostgreSQL çalışmıyor ya da güvenlik duvarı; bağlantı noktasını denetleyin |
| yanıt yok | sunucu kapalı ya da ağ engelli |
| 28P01, parola verilmemişken | sunucu parola istiyor (pencere parolayı sorar) |
| 28P01 | parola ya da kullanıcı adı yanlış |
| 28000 / pg_hba | bu bilgisayardan bu kullanıcıyla izin yok; yöneticiden izin isteyin |
| 3D000 | o adda veritabanı yok |
| 42501 | CONNECT yetkisi yok |
| SSL desteklenmiyor | kipi “Varsa kullan” yapın |
| sertifika doğrulanmadı | kurum sertifikası için “Zorunlu” |

Bağlı katmanın komutları da aynı sözleri kullanır: `connect` hatayı bununla yazar ve `source_unreachable` reddinin
iletisi okunur olur.

### 4. Tablolar katalogdan okunur

`probe::browse`, şemaları, tabloları, view'ları ve somut view'ları yalnız katalogdan okur. Satır sayılmaz;
`reltuples` tahmini gösterilir, analiz edilmemiş tablonun ve view'ın tahmini yoktur. Her biri için şunlar okunur:

- ilk geometri sütununun türü ve sistemi, sütun sayısı;
- açıklaması;
- okuma ve yazma yetkisi;
- bağlı katmanın satırları izleyeceği anahtar. Kural sürücünün `shape` kuralıdır: tabloda tek sütunlu tam sayı
  birincil anahtar, view'da tam sayı `fid`, `id`, `gid` ya da `ogc_fid`.

Bağlanamayan tablo soluk görünür ve nedenini söyler. Gerçek veritabanında da sınanır: katalogun bağlanabilir dediğini
sağlayıcı bağlar, bağlanamaz dediğini reddeder.

### 5. Pencereler

- **Bağlantı penceresi** (`db_connections.rs`; QGIS'in “Create a New PostGIS Connection”ı gibi):
  - Bağlantı: Ad, Sunucu, Bağlantı noktası, Veritabanı (zorunlular yıldızlı).
  - Kimlik: Kullanıcı adı, Parola ve “Parolayı bu bilgisayarda sakla”.
  - Güvenlik: SSL ve açıklaması.
  - Seçenekler.
  - Alanın sorunu alanın altında söylenir. “Adresten doldur” zorunlu değildir: yapıştırılan adres ya da libpq
    bağlantı dizesi alanlara dağılır, içindeki parola parola alanına gider.
  - “Bağlantıyı dene” kaydetmez. Başarılı denemenin parolası oturumda kalır.
  - Düzenlemede kayıtlı parola gösterilmez; alan boş bırakılırsa değişmez.
  - Kopyasını oluştur boş bir adla açılır.
  - Sil önce sorar; bu bağlantıyla bağlanmış katmanlar çizimde kalır.
- **Parola sorusu:** Sunucu KentOS'un bilmediği bir parola isterse açılır. Yazılan parola denenir. Yanlışsa soru
  açık kalır; doğruysa liste kaldığı yerden gelir. Kapatılırsa yeniden sorulmaz; satırda “Parolayı yaz…” kalır.
- **PostGIS'e bağlan** (QGIS'in Veri kaynağı yöneticisi › PostgreSQL'i gibi):
  - Bağlantı kayıtlılar arasından seçilir; yanında Bağlan/Yenile, Yeni…, Düzenle… vardır. Son kullanılan seçili
    gelir; parolası biliniyorsa hemen listelenir.
  - Tablolar şemalara göre ve aranarak listelenir. Görünenleri seç ve Hiçbiri vardır.
  - Yalnız görünen alanın satırları; gerekirse Projenin sistemine çevir.
  - Sorgu katmanı “Gelişmiş” altındadır. Adres kutusu yoktur.
  - Hiç bağlantı yoksa pencere bunu söyler ve Yeni bağlantı… önerir.
- **Kaynaklar › Veritabanları** (QGIS'in tarayıcısı gibi):
  - Bağlantı açılınca şemaları, şemalar da tablolarıyla açılır. Tablonun satırında ikon, ad ve kısa satır sayısı
    vardır; ayrıntı ipucundadır.
  - Çift tık ya da Katman olarak ekle tabloyu bağlı katman yapar. Sağ tıkta “Yalnız görünen alanla ekle” de vardır.
  - Tablo başka sistemdeyse önce sorulur.
  - Bağlantının menüsü: Yenile, Düzenle…, Kopyasını oluştur…, Sil.
  - Bölümün menüsü: Yeni bağlantı…, Bağlantıları içe aktar…, Bağlantıları dışa aktar…, Parolalarıyla dışa aktar….
  - Başlıkta Klasör ekle'nin yanında Bağlantı ekle durur.

### 6. Ekiple paylaşım KentOS'un kendi dosyasıyla

Dışa aktar aynı biçimde bir dosya yazar; parolalar yalnız “Parolalarıyla dışa aktar”da yazılır ve kullanıcı bunun
için uyarılır. İçe aktar aynı adı taşıyan bağlantıya `(2)` ekler. Eksik bağlantıyı (sunucusuz, veritabanısız) almaz
ve söyler. Parolası olmayan bağlantının parolası ilk bağlanışta sorulur.

QGIS'ten bağlantı alınmaz ve QGIS'e verilmez (sahibin kararı).

## Sonuçlar

- Bağlantı dizesi yazmak gerekmez. Pencerede yapıştırma yalnız kolaylıktır.
- `count(*)` kalktı: binlerce tablolu, milyon satırlı veritabanı hemen listelenir.
- Parola kullanıcının isteğiyle düz metin olarak kaydedilir; bu, harita servislerinin gizli değerleriyle aynı
  kuraldır. İşletim sisteminin anahtar zinciri (Secret Service, Keychain) yeni bir bağımlılık ister ve sahibin
  onayını bekler.
- `pg_service.conf` hizmet adları desteklenmez.

## Doğrulama

- `cargo test -p kentos-postgis --lib`:
  - adres, ayrıştırma, alanların sorunları;
  - dosya (0600, kenara ayırma);
  - parola sırası, dışa ve içe aktarma;
  - SQLSTATE'lerin sözleri;
  - katalogun gösterimi.
- Gerçek veritabanında (`KENTOS_PG_URL`, `scripts/fixtures/postgis_cases.py`'nin geçici sunucusu; `--ignored`):
  - `a_connection_is_tried_and_its_tables_listed_from_the_catalogue`: deneme; katalog; katalogun bağlanabilir ve
    bağlanamaz dediklerinin sağlayıcıyla aynı olduğu; yanlış parola, olmayan veritabanı, kapalı bağlantı noktası,
    bulunamayan sunucu ve SSL.
  - `a_password_typed_for_the_session_reaches_the_bound_layer`: oturum parolası bağlı katmanın komutuna ulaşır;
    çizimde parola yoktur.
- Masaüstü (`cargo test -p kentos-desktop exchange::postgis_tests`):
  - pencerede yeni bağlantı;
  - liste ve arama;
  - parola sorusu, bir kez;
  - eksik alanlar ve Adresten doldur;
  - düzenleme, kopya ve silme;
  - bağlantı dosyası;
  - Kaynaklar'ın ağacı ve sistem sorusu.

  Gerçek veritabanıyla bağlama ve resimler `--ignored` ile koşar: `postgis-*` ve `baglanti-*`, `.run/shots`'ta.
