# ADR 9107: Proje bir veri kaynağında

- **Durum:** kabul edildi (2026-11-21); adımlar VY-09a (§2–§5), VY-09b (§6), VY-09c (§7), VY-09d (§8). Plan: [plan/veri.md](../../plan/veri.md) VY-09.
- **İstek:** sahibin sözü (21 Kasım 2026): projeler veritabanında “saklanacak değil saklanabilecek”; “veriler her
  halükarda veritabanında saklanacak CBS projelerinde”; GeoPackage, PostgreSQL ya da ileride başka bir veritabanı.
  Sürücü soyutlaması ileride başka veritabanlarını da desteklemek içindir.
- **Web:** ertelendi (ADR 0228). **`.kcad`:** biçim değişmedi. Veri katmanının `relative` alanı ADR 9101'den beri
  sözleşmedeydi, şimdi yazılıyor ve okunuyor (§3). **Komut:** yok. Kaydet ve Aç kabuk işlemleridir, katalog komutu
  değildir. Python'da `Document.save_to`, `Document.open_from`, `kentos.cad.projects`.
- **Sürücü sözleşmesi:** `kentos_data::ProjectStore` ve `Source::project_store`; hata türü `NoProject`.
- **Komutlar:** yeni `cad.layers.create` v1 (09c, Yeni veri katmanı, 24 ortak durum) ve `cad.layers.migrate` v1 (09d, Katmanı kaynağa taşı, 21 ortak durum).
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

## §7 Karar: yeni katman bir kaynağın yeni tablosudur (VY-09c)

**Araştırma:** QGIS'te Layer ► Create Layer ► New GeoPackage Layer… penceresi dosyayı (yoksa yapılır), tablo adını,
geometri türünü (geometrisiz, nokta, çizgi, poligon, çok parçalıları, eğriler), Z/M'yi, sistemi ve alan listesini (ad,
tür, uzunluk) sorar (kurulu programın ekranı `.run/gui-evidence/veri/qgis/14`).

- **Komut `cad.layers.create`:** tabloyu bir GeoPackage dosyasında (`path`; yoksa yapılır) ya da bir PostgreSQL
  şemasında (`address`, `schema`) yapar ve veri katmanı olarak ekler.
  - Geometri: `point`, `line`, `area` ya da `none` (geometrisiz kayıt tablosu); `multi` çok parçalı, `z` Z ile.
    `srid` verilmezse projenin sistemi kullanılır. Sistem GeoPackage'a adı ve WKT'siyle yazılır (ADR 9106 §4).
  - Tablo adı verilmezse katmanın adından üretilir (dışa aktarmanın kuralı).
  - `fields` sütunlardır (ad, tür, metinde uzunluk); `key` (fid) ve `geometryColumn` (geom) sütun adlarıdır.
  - Tablo kendi işlemiyle yapılır. Katmanın eklenmesi tek geri alma adımıdır; geri alınınca katman gider, tablo
    kaynakta kalır (kaynağı okuyan her tablo gibi).
  - Retler, bu sırayla: `empty_name`, `no_target`, `invalid_name` (en çok 63 bayt; `gpkg_`, `rtree_`, `sqlite_` ile
    başlayan ve `kentos_projects` adları kaynağındır; anahtar ya da geometri sütununun adını taşıyan alan),
    `invalid_fields`, `unknown_srid`, `target_exists`, `source_unreachable`.
- **Masaüstünde:** CBS projesinde Katmanlar'ın + ▸ Nokta, Çizgi, Alan katmanı… ve şeridin aynı komutları (`layer.newPoint`,
  `layer.newLine`, `layer.newArea`) Yeni katman penceresini açar, nesneleri proje dosyasına giden bir katman yapmaz.
  - Pencere: kaynak (GeoPackage dosyası ya da PostgreSQL bağlantısı, adresi, şeması), katmanın ve tablonun adı
    (tablo adı katmanın adını izler, elle yazılınca bırakır), geometri, çok parçalı, Z, sistem, alanlar (ad, tür,
    metinde uzunluk; Alan ekle, Kaldır).
  - Önerilen yer: projenin saklandığı GeoPackage, proje dosyasının yanındaki aynı adlı `.gpkg`, ya da ilk veri
    katmanının GeoPackage'ı.
  - Oluştur'da tablo yapılır, katman etkin katmanın yanına eklenir, etkin yapılır, seçilir ve görünür kaydırılır
    (GIS-004). Ret pencerenin alt çubuğunda söylenir.
  - Açıklama katmanı (yazılar, ölçüler) çizimin katmanı olarak kalır; Karışık katman da öyle. İkisi de bir tablonun
    satırı olacak CBS nesnesi tutmaz.
- **Kanıt:** 24 ortak durum (`scripts/fixtures/layer_create_command_cases.py`; Rust ve Python). Koşucunun dosyayı
  okuduğu bir beklenti bozularak doğrulandı. Masaüstü GIS-004 sınamaları yeni akışla geçer: tablo GeoPackage'ta yapılır,
  katman seçilir ve görünür.

## §8 Karar: eski katman tek adımda kaynağa taşınır (VY-09d)

- **Komut `cad.layers.migrate`:** nesneleri proje dosyasında duran bir CBS çizim katmanını bir GeoPackage dosyasına
  (yoksa yapılır) ya da bir PostgreSQL şemasına taşır.
  - Nesneler `cad.layers.export`'un yazdığı gibi yeni bir tablonun satırları olur.
  - O tabloyu okuyan bir veri katmanı katmanın yerine geçer: aynı yerde, aynı adla, aynı stille (işleyici, etiketler,
    renk); katman etkinse o da etkin olur.
  - Çizim katmanı nesneleriyle gider. Hepsi tek geri alma adımıdır: geri alınınca çizim katmanı nesneleriyle döner,
    tablo kaynakta kalır.
- **Hiçbir şey kaybolmaz:**
  - Katman süzgecinin gizlediği nesneler de taşınır. Dışa aktarma onları atlıyordu; taşımada atlansalardı katmanla
    birlikte silinirlerdi. Bu yüzden dışa aktarma planına `all_objects` bayrağı eklendi.
  - Tabloya yazılamayan nesne (yazı, ölçü) varsa hiçbir şey yazılmaz (`unwritable_objects`). Sayıları ve ilkinin türü
    söylenir; önce bir kuru dışa aktarmayla sayılır.
  - Kilitli katman taşınmaz (`layer_locked`); veri katmanı zaten kaynağındadır (`data_layer`).
- **Retler, bu sırayla:** `no_layer`, `no_target`, `invalid_name`, `layer_not_found`, `not_a_layer`, `service_layer`,
  `data_layer`, `layer_locked`, `target_exists` (“… tablosu kaynakta var; başka bir tablo adı verin”),
  `source_unreachable`, `nothing_to_export` (boş katman ya da yalnız yazı), `unwritable_objects`.
- **Masaüstünde:** CBS projesinde nesne tutan çizim katmanının sağ tuşunda Veri kaynağına taşı… çıkar.
  - Pencere: kaynak (GeoPackage dosyası ya da PostgreSQL bağlantısı, adresi, şeması) ve tablonun adı. Önerilen yer
    09c'deki gibidir.
  - Taşı'dan sonra bildirim: “… veri kaynağına taşındı: N nesne … içinde “…” tablosunun satırları oldu … geri al
    (Ctrl+Z) çizimdeki hâline döndürür.”
- **Kanıt:**
  - 21 ortak durum (`scripts/fixtures/layer_migrate_command_cases.py`; Rust ve Python), koşucunun okuduğu bir beklenti
    bozularak doğrulandı. Durumlar: nokta, çizgi ve alan katmanı; satırlar WKT'leriyle geri okunur; katmanın gittiği
    ikinci taşımanın reddiyle görülür; süzgeçli katmanın bütün nesneleri; retler.
  - Masaüstü `layer_migrate_tests`: katman GeoPackage'a taşınır; yerinde aynı adlı, aynı renkli, etkin veri katmanı;
    proje dosyasında nesne kalmaz; tablo 3 satır; tek geri al katmanı nesneleriyle ve etkin olarak döndürür, tablo
    dosyada kalır.
  - Gerçek pencere: 60 direkli “Eski direkler” `eski.gpkg`'ye taşındı (60 satır, 3 durum); katman sınıflarıyla
    tablodan çizildi; Ctrl+Z katmanı 60 nesnesiyle ve etkin olarak döndürdü. İlk denemede geri almadan sonra etkin
    katman “0”a düşüyordu: etkinlik ayrı bir çağrıydı, artık veri katmanının eklendiği adımın içinde verilir
    (`layers_data::write_activated`).

## §9 Kalan

- Projeyi veritabanında saklamak masaüstünde PostgreSQL için kayıtlı bir bağlantı ya da adres ister; parola
  sistemin parola deposundan ya da ortamdan gelir (adreste yazılmaz).
- Yeni katman ve taşıma tabloyu kendi işleminde yapar; geri alma katmanı geri alır, tabloyu silmez (QGIS'teki gibi).
- Taşıma tabloya yazılamayan nesneleri (yazı, ölçü) ayrı bir katmana kendisi almaz; kullanıcı onları önce başka bir
  katmana taşır.
- Web (ADR 0228).
