# ADR 9107: Proje bir veri kaynağında

- **Durum:** kabul edildi (2026-11-21); adımlar VY-09a (§2–§5) ve VY-09b (§6). Plan: [plan/veri.md](../../plan/veri.md) VY-09.
- **İstek:** sahibin sözü (21 Kasım 2026): projeler veritabanında “saklanacak değil saklanabilecek”; “veriler her
  halükarda veritabanında saklanacak CBS projelerinde”; GeoPackage, PostgreSQL ya da ileride başka bir veritabanı.
  Sürücü soyutlaması ileride başka veritabanlarını da desteklemek içindir.
- **Web:** ertelendi (ADR 0228). **`.kcad`:** biçim değişmedi. Veri katmanının `relative` alanı ADR 9101'den beri
  sözleşmedeydi, şimdi yazılıyor ve okunuyor (§3). **Komut:** yok. Kaydet ve Aç kabuk işlemleridir, katalog komutu
  değildir. Python'da `Document.save_to`, `Document.open_from`, `kentos.cad.projects`.
- **Sürücü sözleşmesi:** `kentos_data::ProjectStore` ve `Source::project_store`; hata türü `NoProject`.
- **Bağımlılık:** yeni dış kitaplık yok.

## §1 Bağlam ve araştırma

**Masaüstü CBS** (QGIS 4.2; kullanım kılavuzu “Working with project files”, 21 Kasım 2026'da okundu; kurulu
programın ekranları `.run/gui-evidence/veri/qgis/09`, `13`, `14`, `16`):

- “Projects can also be saved to and loaded from a PostgreSQL, GeoPackage, or Oracle database”: Project ► Save to ve
  Open from altında saklama yerleri listelenir. GeoPackage'ta bağlantı ve proje, PostgreSQL'de bağlantı, şema ve proje
  sorulur.
- Veritabanında saklanan projeler Tarayıcı panelinden de açılır. Tarayıcıda GeoPackage'ın altında katmanlar ve proje
  yan yana görünür.
- Kurulu kitaplıkla ölçüldü: proje GeoPackage'ta `qgis_projects(name TEXT PRIMARY KEY, metadata BLOB, content BLOB)`
  tablosunda durur. `content`, `.qgz` zip'inin onaltılık metnidir; `metadata` son yazılma anı ve yazanı taşır.
  Tablo `gpkg_contents`'e de `gpkg_extensions`'a da kaydedilmez.

**Türkiye:**

- Mekânsal Planlar Yapım Yönetmeliği (RG 14.06.2014, 29030):
  - md. 19/5: “coğrafi bilgi sistemleri ve uzaktan algılama yöntemleri kullanılarak güncellenebilir ve sorgulanabilir
    sayısal veri tabanı oluşturulur”;
  - md. 34/4–5: kesinleşen planlar “Bakanlık tarafından belirlenen sayısal veri yapısına göre coğrafi bilgi sistemi
    ortamında kaydedilerek” iletilir; coğrafi veri ve bilgilerin “coğrafi bilgi sistemi ortamında sayısal olarak
    üretilmesi … paylaşılması, arşivlenmesi ve güncellenmesi zorunludur”.

  Verinin veritabanında durması ve projenin verisinin yanında saklanabilmesi bu kayıt ve arşivleme yükümlülüğüne uyar.
- TUCBS Birlikte Çalışabilirlik Usul ve Esasları Rehberi (2020) ilke 6 (açık standartlar) ve §3.2.11 (uzun süre
  erişilebilir dosya biçimleri): proje açık bir standardın (OGC GeoPackage) içinde ya da PostgreSQL'de durur.

## §2 Karar: proje deposu sürücü sözleşmesindedir

- **`ProjectStore`:** `projects(schema)`, `read_project(schema, name)`, `write_project(schema, name, content, user)`,
  `delete_project(schema, name)`. `Source::project_store()` proje saklayabilen kaynakta onu verir, öbürlerinde
  `None`'dır. Yeni bir veritabanı sürücüsü (Oracle, MySQL, SQL Server) projeyi de saklayabilir.
  `kentos_data::conformance::projects` onun doğru sakladığını sınar (14 denetim):
  - yazma, listede adı/boyutu/yazanı/anı, bayt bayt geri okuma (0–255 bütün değerler, 70 000 bayt);
  - üstüne yazma, kırpılmış adla okuma;
  - olmayan proje `NoProject`, boş ve satır sonlu ad `Invalid`; silme.
- **Ad:** kırpılır, boş olamaz, en çok 200 karakter, denetim karakteri olamaz. Liste Türk alfabesiyle sıralanır.
- **GeoPackage** (`kentos-data-gpkg`): proje `kentos_projects(name TEXT PRIMARY KEY, metadata TEXT, content BLOB)`
  tablosunda durur. `metadata` bir JSON'dur: son yazılma anı (UTC, kaynağın saati), yazan, biçim (`kcad`).
  - Tablo ilk yazmada yapılır ve `gpkg_extensions`'a `kentos_projects` uzantısı olarak bir kez kaydedilir
    (`kentos_uids`, `kentos_lineage` gibi). `gpkg_contents`'e girmez, katman olarak listelenmez; dosyanın öbür
    tabloları olduğu gibi kalır.
  - Yazma kendi bağlantısında tek işlemdir (`BEGIN IMMEDIATE`).
  - Kayıtta `INSERT OR IGNORE` kullanılmaz: `column_name` NULL olduğu için UNIQUE kuralı ikinci satırı durdurmuyor,
    her yazmada yeni satır ekleniyordu. Aynı hata `kentos_uids` kaydında da vardı ve düzeltildi: her lasting-id yazımı
    bir satır daha ekliyordu.
- **PostgreSQL** (`kentos-data-postgis`): proje şemanın `kentos_projects(name text PRIMARY KEY, metadata jsonb,
  content bytea)` tablosunda durur. Tablo o şemadaki ilk yazmada yapılır (`CREATE TABLE IF NOT EXISTS`); hesap orada
  tablo yapabilmelidir, okumak için okuma hakkı yeter. Yazma tek işlemde upsert'tür, an sunucunun UTC saatidir.
  Katalog `kentos_projects`'i katman listesine almaz.

## §3 Karar: veri katmanı dosyasıyla taşınır

- **Kaydederken** her GeoPackage veri katmanının `relative` alanına dosyasının projenin klasöründen yolu yazılır.
  Başka diskteyse yazılmaz, veritabanı katmanında yoktur.
- **Proje GeoPackage'ta saklandığında** o dosya projenin yeridir. Aynı dosyadaki katmanlar onu adıyla anar
  (`binalar.gpkg`), bağlı GeoPackage'lar ve dış başvurular da yollarını oradan alır.
- **Açarken** adresinde dosya olmayan GeoPackage veri katmanı, göreli yolunda bir dosya varsa oraya taşınır. İçeriği
  karşılaştırılmaz: veri katmanı canlıdır, bağlı kaynaktaki SHA-256 denetimi ona uymaz. Bulunamayan katman söylenir.
  Taşıma bir düzenlemedir, geri alma adımı değildir (bağlı kaynak gibi).
- Böylece GeoPackage, içindeki proje ve katmanlarıyla birlikte başka klasöre ya da makineye taşınabilir. Masaüstü
  sınaması: dosya başka klasöre kopyalanıp eskisi silindi; proje yeni yerinden açıldı, katman oradan 30 satır okudu.

## §4 Karar: masaüstünde Dosya › Veri kaynağı

- **Bölme:** Dosya ekranında Farklı kaydet'in altında Veri kaynağı satırı. Bölmesinde Veri kaynağına kaydet… ve Veri
  kaynağından aç… vardır; çizim bir kaynakta saklanıyorsa “Bu proje: “Sivas altyapı”, binalar.gpkg içinde. Kaydet
  oraya yazar.” yazar. Satır veri katmanı olan programlarda görünür (CBS, Harita). Komut değil bölmedir, çünkü
  arayüz komut kataloğu webden türetilir ve web işi donduruldu (ADR 0228).
- **Pencere:**
  - Kaynak: GeoPackage dosyası (yol, Gözat…, Listele) ya da PostgreSQL veritabanı (kayıtlı bağlantı, adres, şema;
    parola adreste yazılmaz);
  - orada saklanan projeler: ad, yazan, an (cihazın saatiyle), boyut;
  - Proje adı.

  Kaydet penceresi çizimin adıyla açılır, yeri çizimin saklandığı yer ya da ilk veri katmanının GeoPackage'ıdır. Var
  olan adın üstüne yazılacağı söylenir.
- **Kaydet** kayıt hattından geçer (`saving::Target::Source`): çizim baytlara yazılır, kaynağa yazılır, geri okunup
  karşılaştırılır; ancak ondan sonra kaydedilmiş sayılır. Çizim o proje olur, dosyası yoktur. Kaydet (Ctrl+S) yine
  oraya yazar, Farklı kaydet dosya sorar. Dosya menüsündeki Kaydet satırı “"Sivas altyapı" · binalar.gpkg” der.
- **Aç** projeyi arka planda okur ve açar, katmanlar §3'teki gibi bulunur: “"Sivas altyapı" projesi açıldı
  (binalar.gpkg)”.
- Dosyaya kaydetmek olduğu gibi kalır; kaynakta saklamak bir seçimdir.

## §5 Kanıt

- `kentos-data-gpkg` 5 sınama (proje deposu uygunluğu; uzantı satırı bir kez; proje katman değil). `kentos-geopackage`:
  lasting-id uzantısı bir kez kaydedilir.
- `kentos-data-postgis`: proje deposu uygunluğu yalıtılmış veritabanında (`KENTOS_DATA_PG`); `kentos_projects`
  katman listesinde yok.
- Masaüstü `project_store_tests`: GeoPackage'a kaydet, listele, dosya taşındıktan sonra yeni yerinden aç, katman
  oradan okunur.
- Python `test_project_store`: kaydet, listele, aç, üstüne yaz, Türkçe sıra, retler; PostgreSQL şeması.
- Gerçek pencere: Dosya › Veri kaynağı bölmesi; çizim Binalar katmanının GeoPackage'ına “Sivas altyapı” adıyla
  kaydedildi (5141 bayt, yazan ve an `metadata`'da); Kaydet satırı yeni yeri söyledi; Aç listesi projeyi yerel saatle
  gösterdi; proje açıldı.

## §6 Karar: Kaynaklar'da projeler (VY-09b)

- **Nerede görünür:** GeoPackage açılınca tablolarıyla birlikte saklı projeleri de okunur ve tabloların altında
  listelenir (QGIS tarayıcısında olduğu gibi). Veritabanı bağlantısı açılınca `kentos_projects` tablosu olan
  şemaların projeleri okunur ve şemanın tablolarının altına gelir. Satırda ad ve “proje” yazar; kim ve ne zaman
  yazdığı ipucundadır, çünkü satır dardır (ilk denemede ad ile ayrıntı üst üste bindi).
- **Tıklayınca** proje Veri kaynağından aç'taki yoldan açılır: arka planda okunur, kendi sekmesinde açılır,
  katmanları bulunur. Okunamazsa söylenir.
- **Silme** Veri kaynağı penceresindedir: listeden seçilen projede Projeyi sil… çıkar. Önce “… silinecek; geri
  alınamaz. Verisi yerinde kalır.” diye sorar, Sil'den sonra siler ve listeyi yeniden okur.
- **Katalog:** veritabanı kataloğu (Kaynaklar'ın bağlantıları) `kentos_projects`'i tablo olarak göstermez ve gizli
  geometrisiz tablolar arasında da saymaz.
- **Kanıt:** masaüstü `sources` sınaması (GeoPackage'taki proje tablonun yanında listelenir, `kentos_projects`
  tablo olarak görünmez, tıklayınca açılır); `project_store_tests` silme (Vazgeç tutar, Sil siler, liste yenilenir).
  Gerçek pencere: Kaynaklar › demo › binalar.gpkg altında binalar tablosu ve “Sivas altyapı” projesi; tıklayınca
  açıldı. Gerçek pencere betiğine `KENTOS_REAL_STATE` eklendi: Kaynaklar'ın klasörleri, sistemin klasör penceresi
  açılmadan oturuma verilir.

## §7 Sıradaki adımlar (VY-09)

- **09c** CBS projesinde yeni katman bir kaynağın yeni tablosudur (projenin GeoPackage'ı ya da seçilen GeoPackage
  veya şema).
- **09d** Nesne tutan eski CBS katmanlarını tek adımda bir kaynağa taşıma.
- Web (ADR 0228).
