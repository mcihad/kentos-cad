# ADR 0028: Proje kataloğu: tür ve bilgiler, yaşam döngüsü komutları, kişinin listeleri

- **Durum:** kabul edildi (2026-09-26). Yön TODOS.md §12.1'den (`CLOUD-02`, `CLOUD-03`, `CLOUD-04`'ün kalanı, `CLOUD-05`) ve §22.1'in 10. maddesinden gelir. Tür listesi, bilgilerin sürümlenmesi, izinlerin komutlara dağılımı, çöp kutusunun saklama kuralı, kopyanın içeriği, listelerin sayfalanması ve web ekranı bu dilimin kararıdır. Sahibe sorulanlar sonda.
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** CLAUDE.md §13, §16, §21, §4.10; TODOS.md §10.1, §12.1, `CLOUD-12`, `CLOUD-25`, `CLOUD-26`, `CLOUD-30`; ADR 0012 (proje bulutu), 0013 ve 0022 (ürün komutları), 0015 (sahiplik ve erişim), 0024 (web'de paylaşım), 0026 (kalıcı kimlik)

## Bağlam

Koddan doğrulandı (26 Eylül):

- **Oluşturma** `POST /v1/tenants/{alan}/projects` ile, idempotency başlığıyla yapılıyordu; katalogda ürün komutu değildi.
- **Yeniden adlandırma** `project.changes`'in proje bilgisi yamasıydı (`@project`); denetim kaydı yalnız “meta değişti” diyordu.
- **Silme** yumuşaktı (migration 0002, `deleted_at`, `deleted_by`): proje listelerden kalkıyor, açılış ve yazma 410 alıyordu. Geri getirmeyi yalnız işletmeci yapabiliyordu (`kentosd project restore`). Kalıcı silme ve saklama kuralı yoktu.
- **Listeler:** bir alanın projeleri (`GET /v1/tenants/{alan}/projects`) ve “Projelerim” (`GET /v1/me/projects`) sayfalamasız, aramasız, en yeni önce geliyordu. Web'de iki sekme vardı: çalışma alanı ve “Benimle paylaşılanlar” (ADR 0024).
- Projenin türü, açıklaması, etiketi, arşivi, kopyası, son kullanılanlar ve favoriler yoktu.

## Karar

### Proje türü (`CLOUD-02`)

| API adı | Arayüzde | Not |
|---|---|---|
| `cad` | Genel CAD | Varsayılan; türden önce açılmış bütün projeler |
| `gis` | CBS | |
| `landReadjustment` | 18 uygulaması | Arazi ve arsa düzenlemesi |
| `zoningPlan` | İmar planı | |
| `subdivision` | İfraz / tevhit | |
| `road` | Yol | ADR 0012'nin saydığı türlerden |
| `architecture` | Mimari | ADR 0012'nin saydığı türlerden |

- Tür yalnız **katalog etiketidir**: projeleri bulmak ve düzenlemek içindir. Bir modül açmaz, mevzuata uygunluk ya da resmî onay anlamına gelmez, saklama biçimini (`ProjectStorage`) değiştirmez. Arayüz bunu türün seçildiği her yerde söyler.
- Yol ve Mimari, ADR 0012'de proje türü olarak sayıldığı için şimdiden listededir. Modülleri yoktur; etiket olarak iş düzenlemeye yarar.
- Yeni tür yeni bir migration ister (veritabanında `check`); sözleşmede `ProjectType` büyür.

### Katalog bilgileri (`CLOUD-03`)

`ProjectSummary`, katalogun her satırı:

| Bilgi | Kaynak | Not |
|---|---|---|
| Ad | `project.name` | Değişiklik belgenin adıdır da: `meta_version` artar, açık editörler `meta` olayıyla alır |
| Açıklama | `project.description` | En çok 2000 karakter, baştaki ve sondaki boşluk atılır |
| Tür | `project.project_type` | Yukarıdaki tablo |
| Etiketler | `project.tags` | En çok 12, her biri 1–32 karakter; iç boşluklar teke iner, büyük/küçük harf ve Türkçe harf farkıyla aynı olan tekrar atılır |
| Sahip alan | `tenant` | Kurum ya da kişisel alan (`workspaceName`) |
| Sahip, oluşturan | `owner_user_id`, `created_by` | Adları satır güvenliği gösterirse |
| CRS ve birim | `srid`, ayarların `areaUnit`'i | Uzunluk ve koordinat CRS'nin metresidir |
| Kapsam | `…/details` | Aşağıda |
| Durum | `archived_at`, `deleted_at` | `active`, `archived`, `trashed` |
| Değişiklik zamanı | `updated_at` | İçerik ya da katalog bilgisi değişince; arşiv, çöp kutusu ve favori değiştirmez |
| Geçerli revizyon | `data_revision` | |
| Katalog sürümü | `catalog_version` | Ad, açıklama, tür ya da etiket her değiştiğinde 1 artar |
| Kişiye ait | `favorite`, `opened_at` | Yalnız çağıranın |

- **Sürüm iki katmanlıdır.** Biçimin sürümü komutların girdi sürümüdür (`project.metadata.update` v1); her projenin değerleri ise `catalog_version` ile sürümlenir. Bir katalog düzenlemesi hazırlandığı sürümü `expectedVersions["@catalog"]` olarak verir; proje ilerlemişse hiçbir şey yazılmaz (409, `@catalog` çakışması). `project.changes` ile gelen ad değişikliği de sürümü artırır.
- **Kapsam** bir projenin ayrıntısında hesaplanır (`GET …/projects/{proje}/details`), listelerde değil: nesnelerin saklanan CBS geometrisinin kapsamı, projenin koordinat sisteminde. Eğri CAD nesnelerinde geometri izdüşüm olduğu için milimetre içindedir (`PROJECTION_TOLERANCE`). Katalog ipucudur, ölçü değildir. Sınandı: PostGIS `st_extent` saklanan koordinatları kesin verir (float kutuya yuvarlamaz).
- **Kurumun kendi alanları (tasarım, uygulanmadı).** Kurum bir alan şeması tanımlar (`tenant_project_field`: anahtar, etiket, tür, zorunluluk, seçenekler; şemanın kimliği ve sürümü). Projenin değerleri `project.custom_fields` (şema kimliği, şema sürümü, değerler) olarak durur, `project.metadata.update` v2 ile yazılır, sunucuda şemaya göre doğrulanır. Şema değişince eski değerler eski sürümüyle okunur. Aramaya ancak açıkça işaretlenen alanlar girer.

### Yaşam döngüsü (`CLOUD-05`)

| Durum | Açılış | Yazma | Listeler |
|---|---|---|---|
| Etkin | açılır | izne göre | Projelerim, kurum, paylaşılanlar, son kullanılanlar, favoriler |
| Arşivde | salt okunur açılır | içerik ve katalog bilgisi 409 `project_archived`; paylaşım değişebilir | Arşivlenmişler, son kullanılanlar, favoriler |
| Çöp kutusunda | 410 `project_deleted` | 410 | Çöp kutusu (yalnız geri yükleyebilenlere) |
| Kalıcı silinmiş | 404, var olmayan gibi | 404 | hiçbiri |

- Arşiv ve çöp kutusu iki ayrı işarettir: arşivlenmiş proje çöpe taşınıp geri yüklenirse arşivde döner.
- Çöp kutusu migration 0002'nin yumuşak silmesidir; `DELETE …/projects/{proje}` bugün de çalışır ve `project.trash` ile aynıdır. Olay adı `project.deleted` olarak kaldı: istemciler onu tanıyor.

### Komutlar

Hepsi sunucu ürün komutu (ADR 0013), katalogda; projeye bağlı olanlar `access::project` ile başlar, kilit altında yeniden sorar (ADR 0015), idempotency anahtarıyla bir kez yazar ve denetime istek kimliğiyle yazılır (`CLOUD-25`).

| Komut | İzin | Olay | Denetim |
|---|---|---|---|
| `project.create` | alanda `project.create` | — | `project.create` (tür) |
| `project.rename` | `project.edit` | `project.metadata` (`meta`) | `project.rename` (eski, yeni) |
| `project.metadata.update` | `project.edit` | `project.metadata` | ne değişti (açıklamanın metni değil) |
| `project.duplicate` | kaynakta `project.download`, hedefte `project.create` | — | kaynakta `project.duplicate`, kopyada `project.create` (`copyOf`) |
| `project.archive`, `project.unarchive` | `project.edit` | `project.archived`, `project.unarchived` | kim |
| `project.trash` | `project.delete` | `project.deleted` | kim, `purgeAfter` |
| `project.restore` | `project.delete` | `project.restored` | kim |
| `project.purge` | `project.delete` | — | `project.purge` (kim, ad, nesne sayısı); denetim kalır |
| `project.favorite` | `project.read` | — | yok (kişinin kendi listesi) |

- **`project.create`** projesi olmayan bir komuttur: `POST /v1/tenants/{alan}/commands`, zarfın `projectId`'si boş. Eski `POST …/projects` yolu aynı işlevi çağırır; web yüklemede onu kullanmaya devam eder.
- **`project.rename`** listeden (açık olmayan projede) ve Python/AI için vardır; açık projenin adı yine otomatik kayıttan (`project.changes`) gider, kendi sürümünü bildiği için kendisiyle çakışmaz.
- **İzinler:** arşiv `project.edit`'tir (yönetici ve üstü): editörleri salt okunur yapar, bunu yönetici rol düşürerek zaten yapabilir. Çöp kutusu, geri yükleme ve kalıcı silme `project.delete`'tir: proje sahibi ve politika açıksa kurum sahibi ve yöneticisi (ADR 0015'teki sahip kararı 3). Paylaşımla gelen yönetici çöpe taşıyamaz; sahibe soruldu.
- **Kalıcı silme** yalnız çöp kutusundaki projede ve açık onayla çalışır: `confirmName` projenin şimdiki adıyla aynı olmalıdır. Arayüz bunu kullanıcı “Kalıcı olarak sil”e bastıktan sonra doldurur; Python ya da AI adı kendisi yazmak zorundadır. Silinen: nesneler, paylaşımlar, favoriler, son kullanılanlar, komut günlüğü, olaylar ve olay ufku. Kalan: denetim kaydı. Önceden indirilmiş kopyalar ve yedekler bu komutla silinmez (`CLOUD-30`). Yanıt kaybolursa yeniden deneme 404 alır: proje zaten yoktur.

### Kopya

- **Kopyalanan:** ayarlar, katman ağacı (kilitleriyle), stiller, köken, başlangıç görünümü, açıklama, tür, etiketler ve bütün nesneler. Her nesne kalıcı kimliğini korur (ADR 0014, 0026: aynı kimlik iki projede iki ayrı nesnedir); sürümü 1'dir, çünkü kopya yeni projenin ilk commit'idir (veri revizyonu 1). Geometri ve CAD tanımı bit bit aynıdır (sınandı).
- **Kopyalanmayan:** geçmiş (komut günlüğü, olaylar, denetim), paylaşımlar, favoriler ve son kullanılanlar, arşiv durumu. Kopya çağıranındır; paylaşana kadar yalnız ona ve kurum politikasıyla kurum yöneticilerine görünür.
- **Nereye:** çağıranın proje açabildiği bir alan (varsayılan kaynağın alanı). Görüntüleyici `viewer_download` kapalıyken kopyalayamaz: kopya da bir indirmedir.
- Dış kaynak bağlantısı bugün yoktur (bütün projeler yönetilen PostGIS). Geldiğinde kopyalanıp kopyalanmayacağı bu komutun açık bir seçimi olur.
- Kopyalama tek SQL deyimiyle yapılır (`kentos.duplicate_project`), kaynağın kilidi altında: aynı anahtarla gelen tekrar beklenir ve günlükten yanıtlanır. Çok büyük projelerde bu süre boyunca kaynağa commit bekler; iş kuyruğu gelince maliyeti `job` olabilir.

### Çöp kutusunun saklama kuralı

- Proje çöpe taşındığı anda `purge_after = şimdi + saklama süresi` yazılır; süre sunucu ayarıdır: `KENTOS_TRASH_RETENTION_DAYS`, 1–3650 gün, **varsayılan 30**. Sonradan süre değişirse çöpteki projelerin tarihi değişmez: kullanıcıya söylenen tarih geçerlidir.
- `kentosd serve` saatte bir, süresi geçenleri kalıcı olarak siler (`kentos.purge_trash`, parti başına 20 proje; her biri denetime “retention” olarak yazılır). Bir günden yeni taşınan proje hiçbir zaman otomatik silinmez.
- **Bu kuraldan önce silinmiş projelerin `purge_after`'ı yoktur:** o kurala (işletmeci geri getirir) göre silinmişlerdir, otomatik silinmezler. `kentosd project deleted` onları “-” ile gösterir.
- Çöp kutusu listesi her projenin kalıcı silinme gününü, uyarı pencereleri süreyi gün olarak söyler.

### Listeler (`CLOUD-04`)

`GET /v1/me/catalog?view=&tenant=&q=&type=&sort=&limit=&after=`, `ProjectPage { projects, total, next, trashRetentionDays }`:

| Görünüm | Arayüzde | İçerik |
|---|---|---|
| `recent` | Son kullanılanlar | Kişinin açtığı ve oluşturduğu projeler, çöptekiler hariç; varsayılan sıra son açılma |
| `favorites` | Favoriler | Kişinin favorileri, çöptekiler hariç |
| `mine` | Projelerim | Sahibi olduğu etkin projeler, bütün alanlarda |
| `organization` | Kurum projeleri | Seçilen kurumun (ya da kişisel alanın) görebildiği etkin projeleri |
| `shared` | Benimle paylaşılanlar | Rolü paylaşımdan gelen etkin projeler |
| `archived` | Arşivlenmişler | Rolü olduğu arşivlenmiş projeler (görüntüleyici dahil) |
| `trash` | Çöp kutusu | Geri yükleyebildiği (sahip ya da politika) çöpteki projeler; varsayılan sıra taşınma |

- **Yetki süzgeci önce gelir:** her sorgu satır güvenliğiyle o alanın kapsamında çalışır; arama, sayım ve sayfa görülebilen satırlar üstündedir. Başka kurumun projesi hiçbir sayıya girmez (`CLOUD-12`).
- **Arama:** her sözcük adda, açıklamada ya da etiketlerde geçmeli; Türkçe harfler ADR 0024'teki tabloyla katlanır, `%` ve `_` harf sayılır, en çok 100 karakter. Kişi aramasından farklı olarak tek harf yeter: liste zaten kişinin kendi projeleridir.
- **Sıra:** son değişiklik, ad, oluşturulma, son açılma (yalnız son kullanılanlar), taşınma (yalnız çöp kutusu). Eşitlikte proje kimliği. Ad, arama gibi katlanıp bayt bayt karşılaştırılır (`collate "C"`): sıralama veritabanının yerel ayarına bağlı değildir.
- **Sayfalama** anahtarla yapılır (`after` opak; `[anahtar, kimlik]`), en çok 200, varsayılan 50. Birden çok alanı kapsayan görünümde her alan kendi kapsamında sayfanın devamını verir, sunucu birleştirir; anahtar veritabanından gelir, birleştirme veritabanıyla aynı sırayı izler.
- **`.kcad`'e arama dizini konmaz;** katalog sunucudadır. Eski iki liste yolu kalır, artık arşivlenmişleri de dışarıda bırakır.

### Son kullanılanlar ve favoriler

- `project_recent` ve `project_favorite` kişiye aittir: satır güvenliği her satırı yalnız kendi kişisine gösterir; yazma yalnız projenin kapsamında ve proje görülebilirken olur. Liste proje satırlarıyla birleştiği için erişimi kalkan proje satırı dursa da listede görünmez.
- Açılış (nesnelerin ilk sayfası), oluşturma ve kopyalama son kullanılanlara yazar. Favori `project.favorite` ile değişir. İkisi de denetime ve olaylara girmez: kişinin kendi listesidir, başkası öğrenmez.
- Proje kalıcı silinince ikisi de gider.

### Web

- **“Bulut projeleri”** (`cloud.open`): solda listeler, ortada sunucuda aranan, türe göre süzülen, sıralanan ve sayfalanan liste, sağda seçili projenin bilgileri (tür, açıklama, etiketler, alan, sahip, rol, CRS, alan birimi, nesne ve katman sayısı, kapsam, oluşturan, revizyon, saklama) ve eylemleri. Eylemler hep görünür; yapılamayan devre dışıdır ve hangi izni istediğini söyler. Bilgiler kaydırılır, eylemler görünür kalır.
- Pencerenin tek amber düğmesi listenin ana eylemidir: **Aç** (arşivlenmiş proje salt okunur açılır), çöp kutusunda **Geri yükle**.
- Sorular `confirm.ts` iledir: çöpe taşıma süreyi, kalıcı silme geri alınamadığını, arşiv açık editörlere ne olacağını söyler; silen yanıt kırmızı yazılıdır, odak Vazgeç'tedir.
- Yükleme türü, açıklamayı ve etiketleri alır. “Proje bilgileri” adı, türü, açıklamayı ve etiketleri gösterildiği katalog sürümünden değiştirir. “Kopyasını oluştur” kopyanın adını ve alanını sorar; kopya Projelerim'de seçili açılır.
- Uygulama menüsünün “Son projeler”i sunucunun son kullanılanlarıdır.
- **Açık proje başkası tarafından arşivlenince** silinmedeki gibi davranılır: kayıt durur, durum hücresi “Proje arşivde” der, gönderilmemiş değişiklikler cihaz taslağında kalır, 409 `project_archived` da aynı sonucu verir. Arşivden çıkarılıp yeniden açılınca saklanan değişiklikler gönderilir. Kendisi arşivlediğinde uyarı değil bilgi verilir; arşivden çıkarınca proje yeniden açılır.
- Çöpe taşınan açık proje bugünkü gibi bırakılır; çizim ekranda kalır.

### Migration 0005

`crates/server/postgres/migrations/0005_project_catalog.sql`:

- `project`: `description`, `project_type` (varsayılan `cad`), `tags`, `catalog_version`, `archived_at`, `archived_by`, `purge_after` ve bunların denetimleri.
- `kentos.project_access` arşiv bilgisini de döndürür (işlev yeniden kuruldu).
- `project_recent`, `project_favorite` ve satır güvenliği.
- `kentos.duplicate_project`, `kentos.purge_project`, `kentos.purge_trash` (sahip olarak çalışır; her biri güncel kullanıcının rolünü kendisi denetler, sunucunun kuralıyla aynı) ve yalnız bunların çağırdığı `kentos.remove_project`.
- Var olan veri: projeler `cad` türünde, açıklamasız, etiketsiz, `catalog_version = 1`; silinmiş projeler `purge_after`'sız.
- **Geliştirme veritabanına (`kentos_cad`) uygulanmadı.** Sahip onaylar, sorumlu yedekten sonra uygular. Checksum'ı ancak kalıcı bir veritabanı uyguladıktan sonra `RELEASED`'e sabitlenir.

## Sonuçlar

- **Testler (sunucu, `KENTOS_TEST_DB=required`):**
  - `tests/project_lifecycle.rs`: arşiv (görüntüleyici ve editör yapamaz, yönetici yapar; 409; paylaşım değişir; listeler; arşivden çıkarma), çöp kutusu (paylaşımla gelen yönetici taşıyamaz; `purge_after` 30 gün; olay; yalnız geri yükleyebilenin çöp kutusu), geri yükleme, kalıcı silme (çöpte değilse, ad tutmazsa, görüntüleyici; sonra yalnız denetim kalır; herkese aynı 404), veritabanı işlevinin paylaşımla gelen yöneticiyi reddetmesi, saklama süresi (bir günlük taban, eski silinmişler), sahibin her değişiklikte korunması.
  - `tests/project_duplicate.rs`: kopyanın içeriği (kimlikler, sürüm 1, geometri ve CAD tanımı bit bit, katmanlar), kopyalanmayanlar, kimin nereye kopyalayabildiği (`viewer_download`, `project.create`, başka alan 404, çöpteki 410), veritabanı işlevinin kendi reddi, tekrarın bir kez kopyalaması.
  - `tests/project_catalog.rs`: bilgilerin denetimi, sürümü, `@catalog` çakışması, adın belge sürümüne etkisi, denetim ayrıntısı; görünümler, arama, tür süzgeci, sıralar, sayfaların birleşince bütün listeyi vermesi, başka kurumun hiçbir sayıya girmemesi; son kullanılanlar ve favorilerin kişiye özel oluşu; ayrıntı ve kesin kapsam.
  - `apps/api/src/http/catalog_tests.rs`: bütün yeni komutların ve ayrıntının, göremeyene var olmayan projeyle aynı 404'ü; görüntüleyicinin 403'leri; `project.create`'in alan yolu ve tekrarı; arşivde 409; `DELETE`'in çöp kutusu; parametre denetimleri.
- **Testler (web, Vitest):** `catalog.test.ts` (sözcükler, istek, sayfalama ve eskiyen yanıtın atılması), `lifecycle.test.ts` (arşivlenen açık projenin kaydı: olay, kendi olayı, 409, arşivli açılış; eylemlerin açık projede sırası).
- **e2e:** `pnpm e2e:cloud` yeni adımlarıyla. `KENTOS_E2E_DB=scratch` onu geçici bir veritabanında, bu yapının migration'larıyla ve geliştirme hesaplarıyla çalıştırır (`apps/api/examples/e2e_database.rs`); `kentos_cad`'e dokunmaz.
- **Kasıtlı bozma:** arşiv için `project.edit` yerine `project.read` istendi (görüntüleyici arşivleyebilir). `archiving_makes_a_project_read_only_until_it_is_unarchived` (“expected 403 (project.edit), got Ok(… role: Viewer … state: Archived)”) ve HTTP `catalog_routes_answer_404_alike_and_check_every_permission` (“project.archive: left 200, right 403”) düştü. Geri alındı.

## Ertelenenler

- Masaüstünde bulut yok; masaüstü aynı uçları kullanacak.
- Kurumun kendi alanları (tasarım yukarıda), kuruma göre saklama süresi, kota (`CLOUD-26`).
- Arşivlenen açık projede **Farklı kaydet** çizimi dosyaya yazar ama projeden ayrılmaz (Ctrl+S ayrılır): `app/fileIO.ts`'in ayrılma koşuluna `archived` eklenmeli (o dosya binary `.kcad` dilimindedir).
- Kopyalamanın iş kuyruğuna alınması (`JOB-*`), dış kaynak bağlantılarının kopyalanması.
- Çok projeli kurumlarda listelerin ölçümü (`CLOUD-28`): her satır için rol işlevi iki kez çalışır.
- Gruplar, davet ve sahiplik devri (`CLOUD-08`, `CLOUD-16`, `CLOUD-17`) bu dilimin dışındadır.

## Sahibe sorular

1. **Çöp kutusunun saklama süresi:** 30 gün (önerilen, uygulanan varsayılan) · 14 gün · 90 gün · süresiz (yalnız elle kalıcı silme).
2. **Paylaşımla gelen yönetici çöpe taşıyıp geri yükleyebilsin mi?** Hayır, sahip ve politika yöneticisi (önerilen, uygulanan; ADR 0015 kararı 3) · geri yükleyebilsin, taşıyamasın · ikisini de yapabilsin.
3. **Kalıcı silme kimde?** Çöpe taşıyabilenlerde (önerilen, uygulanan) · yalnız proje sahibinde · yalnız kurum yöneticisinde.

## Doğrulama (26 Eylül 2026, Linux; main `1f77280` üstünde)

- `cargo fmt --all --check` temiz.
- `KENTOS_TEST_DB=required pnpm rust:test`: 518 test geçti, hiçbir veritabanı testi atlanmadı; clippy `-D warnings` temiz; bağımlılık yönü temiz (18 crate, 23 crate × hedef). Migration 0005 her testin geçici veritabanına uygulandı.
- `pnpm typecheck` temiz. `pnpm test`: 1086 geçti, 13 atlandı (başlangıçtaki 13).
- `pnpm inventory:check` güncel (yeni dört pencere ve `cloud.delete`'in yeni adı işlendi).
- `pnpm e2e` (duman): 160 kontrol geçti.
- `KENTOS_E2E_DB=scratch pnpm e2e:cloud`: 58 kontrol geçti, konsolda hata yok; geçici veritabanında, `kentos_cad`'e dokunulmadan.
- **Çalıştırılmayan:** `kentos_cad` üstünde `pnpm e2e:cloud`. Migration 0005 orada uygulanmadı; `kentosd` eksik migration görünce başlamaz. Sahibin onayı ve yedekten sonra 0005 uygulanınca çalıştırılmalı.
