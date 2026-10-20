# ADR 0547: Sunucu işlerinin sınırları, ölçümleri ve masaüstü işler görünümü

- **Durum:** kabul edildi (2026-10-20). `plan/gis.md` CBS-39'un ikinci dilimi ([ADR 0542](0542-durable-server-jobs.md)'nin “Sonraya kalan”
  listesinden). Kapsamı ben belirledim.
- **Bağlam belgesi:**
  - ADR 0542 (dayanıklı sunucu işleri: kira, çit, staged sonuç);
  - 0031 (nesne deposu), 0034 (kontrol noktaları), 0044 (masaüstü olayları bekleyerek izler), 0045 (Range ile kaldığı yerden indirme);
  - 0087 (masaüstü Geçmiş sekmesi, bu görünümün örneği);
  - CLAUDE.md §18 (kotalar, iptal, kesinleşmede yetki), §21.
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - CPU ve bellek kotası (bugün süre, deneme, kurum başına iş sayısı ve dosya boyu var);
  - sunucuda İşlemler aracı, büyük içe aktarma, atlas PDF'i iş türleri;
  - işin ilerlemesini işçinin yazması (bugün `progress` kaba: iş bitince 1; `beat_job` ilerleme alır, işler henüz yazmıyor);
  - önbellek anahtarı, yedek ve geri yükleme tatbikatı (G2).

## Bağlam

ADR 0542 işi dayanıklı yaptı ama işin neye mal olacağını sınırlamadı, kullanıcıya göstermedi ve operatörün göremeyeceği bir kuyruk bıraktı:
- bir kurum sınırsız iş isteyip kuyruğu doldurabilir;
- sonuç dosyasının boyu ve diske yazılamaması tanımsızdı; biten işlerin dosyaları sonsuza dek duruyordu;
- kuyruğun uzunluğunu, düşen işi, en eski bekleyeni görmenin yolu yoktu;
- masaüstünün bulut arayüzünde işler yoktu: iş isteyen kullanıcı pencereyi kapatınca onu bulamıyordu;
- tek tür vardı.

## Karar

### 1. Kurum başına sınır

- Bir kurumun sırada ve çalışan işlerinin toplamı `KENTOS_JOB_LIMIT`'i aşamaz (1–10 000, varsayılan 50).
- Aşan istek `quota_exceeded` kodu, 429 ve Türkçe iletiyle reddedilir; ileti sınırı sayıyla söyler, `retryable` doğrudur (iş bitince geçer).
- Sayma ve ekleme aynı işlemdedir; iki istek son yeri birlikte alamaz: `kentos.job_slot(kurum, sınır)` kurum için işlem kilidi (`pg_advisory_xact_lock`)
  alır, sayar ve “yer var mı” der (security definer; satır güvenliği altında işlerin hepsi görünmez).
- Aynı kullanıcının aynı istek kimliğiyle yinelenen isteği sınıra bakmadan işini geri alır (ADR 0542 §1).
- Başka kurumun işleri sayılmaz. Çalışan iş yerini tutar; biten, düşen ve durdurulan bırakır.

### 2. Sonuç dosyası

- **Boy sınırı:** `KENTOS_JOB_ARTIFACT_MAX_MB` (1–1 048 576, varsayılan 2 048). Projenin görüntüsü bundan büyükse iş `too_large` kodu ve boyları söyleyen
  iletiyle düşer; nesne deposuna hiçbir şey yazılmaz. Sınır deneme sayısına girmez: iş yeniden denenmez.
- **Yazılamayan dosya** (yer dolu, klasör ya da yetki): iş `storage` koduyla düşer ve yazılan yarım nesne silinir. Hata kalıcı sayılır: iş ilk denemede düşer,
  yeniden denenmez (kira kaybı gibi geçici sayılmaz; yöneticinin yeri ya da yetkiyi düzeltip işi yeniden istemesi gerekir).
- **Yarım nesne:** işçi dosyayı yazarken ölürse nesne anahtarında (`jobs/{kurum}/{proje}/{iş}-{sha}`) kalır. Sonraki deneme aynı anahtarı keser ve baştan yazar
  (anahtar işin kimliği ve içeriğin özetidir). İş hiç bitmezse (iptal, düşme) nesneyi deponun temizliği siler: hiçbir işin tutmadığı `jobs/` dosyası sahipsizdir.

### 3. Saklama

- Biten (başarılı, düşen, durdurulan) işler `KENTOS_JOB_RETENTION_DAYS` gün sonra (1–3650, varsayılan 14) satırıyla ve dosyasıyla silinir.
- Silme `kentos.expire_jobs(önce, parti)` ile yapılır (security definer; silinen satırların dosya anahtarlarını döner), sunucu dosyaları siler.
  Dosyayı silemezse anahtar sahipsiz kalır ve deponun saatlik temizliği (`files::cleanup`, ADR 0542 §3) onu bulur: iki yol aynı sonuca gider.
- Sıradaki ve çalışan işe dokunulmaz. Süpürme deponun temizliğinin turunda (açılıştan iki dakika sonra, sonra saatte bir), 200'lük partilerle, ayrı bir iş
  parçacığı açmadan çalışır.

### 4. Ölçümler

- `kentosd jobs` yöneticinin okuyacağı satırları yazar (`ad<TAB>değer`): `sırada`, `çalışan`, son 24 saatte `düşen`, `biten`, `durdurulan`,
  `en_eski_bekleyen_saniye` ve işi olan kurumlar (`kurum<TAB>kimlik<TAB>sırada<TAB>çalışan`).
- Veri `kentos.job_metrics()` ve `kentos.job_load()` ile (security definer) okunur. Hiçbir işin girdisi, sonucu, adı ya da parola bilgisi çıkmaz; yalnız sayılar ve
  kurum kimlikleri.
- Uç noktaya çevrilmedi: kurumlar arası bir okuma, kurumdan bağımsız bir yetki sorusu getirirdi; yönetici aracı yetkiyi sahip rolüyle verir.

### 5. İkinci iş türü: `project.checkpoint`

- Girdi `CheckpointCreate`'dir (ad, not, dosya projesinde revizyon). Ad ve not iş istenirken denetlenir (`checkpoints::name_of`, `note_of`: aynı kural).
- `jobs.run` ve `feature.write` ister, arşivde istenmez. İş **isteyen kullanıcı olarak** `project.checkpoint.create` komutunu çalıştırır; komutun idempotency
  anahtarı `job-{iş kimliği}`dir: kirasını kaybeden işçinin işi ikinci işçi yeniden çalıştırsa tek kontrol noktası olur (ikinci çağrı saklı cevabı alır).
- Dosya komutun kendi nesnesidir (`checkpoints/…`), işin `artifact`'ı yoktur; sonuç `CheckpointChange`'dir. Bitirmede `jobs.run` yeniden sorulur
  (komutun kendisi yetkiyi kendi işleminde zaten yeniden sorar).
- Hata kodları: `archived` (proje arşivde), ötekiler ADR 0542'deki gibi.

### 6. Masaüstü: “Sunucu işleri” sekmesi

Katalog penceresinde seçili projenin üçüncü sekmesi (Bilgiler, Geçmiş, **Sunucu işleri**); ortak plan (`plan.tabs`, web'le ortak fixture) değişmez, sekme
masaüstüne özgüdür.

- **İstemci:** `kentos_cloud::Cloud` `server_jobs`, `server_job`, `create_server_job`, `cancel_server_job`, `server_job_artifact`. Dosya indirmesi
  nesne deposundaki dosyalarınki gibidir: SHA-256 entity tag olarak denetlenir, kesilen indirme `Range` ile sürer. Sunucu iş dosyasında da `Range: bytes=N-`
  yanıtlar (206, 416). Ağ ve hesap `kentos_cloud`'dadır; kurallar ve sözcükler `kentos_cloud::server_jobs`'tadır (arayüzsüz, testlidir).
- **Satırlar:** işin türü, durum çipi (Sırada, Çalışıyor, Bitti, Başarısız, Durduruldu), çalışan işte ilerleme çubuğu ve yüzde, istenme ve bitme zamanı (cihazın
  saatinde), deneme sayısı, biten dışa aktarmada boy, nesne sayısı ve veri revizyonu, başarısızda sunucunun iletisi ve koda göre öneri.
- **Düğmeler** (ikonlar sahibe sorulmadan seçildi; hiçbiri tekrarlanmaz): Projeyi dışa aktar (.kcad) (`play`), Yenile (`feedRefresh`), İndir (`export`),
  Durdur (`close`). Yapılamayan düğme görünür kalır, kapalıdır ve nedenini söyler (hangi hak eksik, dosya projesi, iş bitmiş, durdurma zaten istenmiş).
- **Kimin neyi yapabildiği:** istemek `project.jobs.run` ve `project.download` ister (veritabanı projesinde; dosya projesinin `.kcad`'i zaten revizyonlarıdır).
  Durdurmayı işi isteyen ya da `project.edit` sahibi yapar. İndirmek `project.download` ister. Sunucu her isteği yine karara bağlar.
- **Pencere kapanınca iş kaybolmaz:** iş sunucudadır. Yeniden açılışta sekme listeyi sunucudan alır; çalışan iş orada çalışmaya devam eder.
  İsteğin cevabı kaybolursa (ağ koptu) aynı **istek kimliği** saklanır ve düğmeye yeniden basılınca aynı kimlikle gider: sunucu ilk işi geri verir, ikinci iş
  yapılmaz. İstek kimliği iş yapıldığında bırakılır; sonraki istek yeni kimlik alır.
- **İzleme:** masaüstünde WebSocket yok (ADR 0044) ve işler proje olaylarında görünmez. Sekme açıkken ve bir iş sırada ya da çalışıyorken liste
  `POLL_ACTIVE` (2 s) aralıkla sorulur; bütün işler bittiğinde zamanlayıcı kalkar. Yoklama başarısız olursa liste durur ve 5 s sonra yeniden denenir; ilk
  okuma başarısızsa “Yeniden dene” görünür. İzlenirken biten iş günlüğe bir kez yazılır.
- **Açık projede:** istekten önce açık veritabanı projesinin gönderilmemiş değişiklikleri gönderilir (`Settle::Job`, Geçmiş'teki kontrol noktasındaki gibi);
  dışa aktarma bu değişiklikleri taşır.
- **İndir:** kataloğun indirmesidir (`Fetch::Job`): kaydetme penceresi, ilerleme, SHA-256 denetimi, geçici dosya ve yeniden adlandırarak yazma. Dosya adı
  “Proje adı (veri revizyonu N)”dır.

### 7. Sözleşme ve Python

- `ServerJobError.code` `too_large`, `storage` ve `archived`'ı da sayar; `PROJECT_CHECKPOINT_JOB`, `SERVER_JOB_TENANT_ACTIVE_DEFAULT`,
  `SERVER_JOB_ARTIFACT_MAX_DEFAULT`, `SERVER_JOB_RETENTION_DAYS_DEFAULT`.
- Göç 0030: `kind` denetimi iki türü kabul eder; `job_slot`, `expire_jobs`, `job_metrics`, `job_load`.
- Python `ProjectRef.checkpoint_job` eklendi; kurum sınırı `ServerError` (`quota_exceeded`, `retryable`) olarak gelir.

## Performans

Sayım ve süpürme kısa tek ifadelerdir; `server_job_queue` ve `server_job_project` dizinleri bunları taşır. Ölçüm işlevleri tek geçişle sayar. Masaüstünde yoklama
yalnız sekme açık ve iş varken 2 sn'de bir küçük bir JSON'dur (en çok 100 satır); çizim hattına dokunmaz.

## Uygulama

- Sözleşme: `crates/shared/contracts/src/server_jobs.rs`. Göç: `crates/server/postgres/migrations/0030_server_jobs_limits.sql`.
- Sunucu: `crates/server/application/src/jobs.rs` (`JobLimits`, kurum sınırı, boy sınırı, `storage`, `project.checkpoint`), `job_upkeep.rs` (saklama ve ölçümler),
  `error.rs` (`AppError::Quota`); HTTP `apps/api/src/http/jobs.rs` (Range); `apps/api/src/config.rs`, `main.rs`, `cli.rs` (`kentosd jobs`).
- İstemci: `crates/native/cloud/src/api.rs`, `server_jobs.rs`. Masaüstü: `apps/desktop/src/cloud/catalog_jobs.rs`, `catalog_jobs_view.rs`.

## Doğrulama

- `KENTOS_TEST_DB=required cargo test -p kentos-application --test server_jobs` (geçici PostGIS'te, 7 test):
  - ADR 0542'nin bütün durumları değişmeden geçer;
  - kurum sınırı: iki yer, üçüncü `quota_exceeded`, yinelenen istek işini geri alır, başka kurum ayrı sayar, iptal yer açar, çalışan iş yerini tutar; sekiz istek aynı
    anda beş yer için gelince tam dört iş yapılır;
  - boy sınırı (`too_large`, depoda nesne yok) ve yazılamayan depo (`storage`, deneme 1, dosya yok);
  - işçi dosyanın yarısını yazıp ölür: sahipsiz nesne görünür, sonraki işçi baştan yazar ve dosya projenin görüntüsüyle bayt bayt aynıdır; iptal edilen işin yarım
    nesnesini temizlik siler;
  - sekiz işçi bir işe aynı anda bakar: biri alır, deneme 1, tek dosya;
  - saklama: eski üç iş (başarılı, düşen, durdurulan) iki'şerli partilerle silinir ve dosya gider; yeni ve sıradaki iş kalır; silinemeyen dosyayı temizlik bulur;
    ölçümler satırlarla uyuşur;
  - kontrol noktası işi: girdi denetimi, izleyici reddi, ölen işçinin ardından tek kontrol noktası, `artifact` yok.
- `KENTOS_TEST_DB=required cargo test -p kentos-api jobs`: Range ile kesik indirme (206, aynı entity tag, parçalar dosyayı verir, 416); `kentosd jobs` çıktısı;
  ayar sınırları (`config`).
- `cargo test -p kentos-cloud`: iş yolları ve kesik indirmenin sürmesi bir kuklayla; `server_jobs` kuralları (istek, sözcükler, kim neyi yapar).
- `cargo test -p kentos-desktop --bin kentos-cad catalog_jobs_tests`: sekmenin iletileri sunucusuz; resimler `.run/shots/isler-*.png`.
- `python/tests/test_connection.py`: `checkpoint_job` ve kurum sınırının tipli reddi.
- Gerçek `kentosd` ile masaüstü akışı: `cargo test -p kentos-desktop --bin kentos-cad jobs_live -- --ignored` (geçici veritabanında).
