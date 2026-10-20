# ADR 0559: İşlemler araçları dayanıklı sunucu işi olarak

- **Durum:** kabul edildi (2026-10-20). `plan/gis.md` CBS-39'un üçüncü dilimi ([ADR 0542](0542-durable-server-jobs.md) ve
  [0547](0547-server-jobs-limits-and-desktop.md)'nin “Sonraya kalan”ından: sunucuda İşlemler aracı, CPU ve bellek kotası,
  işçinin ilerleme yazması). Kapsamı koordinatörün önerisine uyarak ben belirledim.
- **Bağlam belgesi:**
  - ADR 0542 (kira, çit, staged sonuç, yetkinin kesinleşmede yeniden sorulması), 0547 (sınırlar, masaüstü Sunucu işleri sekmesi);
  - 0508, 0529, 0551 (`cad.processing.run`, `.batch`, çalışma kaydı, sonucu GeoPackage'a yazma);
  - 0253 (komut protokolü), 0550 (erişim çözücüsü: kesilmiş kopya), 0015 (proje erişimi);
  - CLAUDE.md §9.2, §18.
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan** (MCP'de iş türleri, blok tanımı yazan sonuçlar, 5000'den çok nesne yazan sonuçlar, büyük içe aktarma ve atlas PDF'i
  [ADR 0561](0561-import-atlas-and-mcp-jobs.md) ile kapandı; blok tanımı delta'ya girdi ve işin kesinleşmesi 100 000 değişikliğe çıktı):
  - dosya alan ya da uzaktan okuyan araçlar (raster, nokta bulutu, dosya parametresi): sunucuda yerel dosya yok, o araçlar reddedilir
    ya da `tool_failed` verir; proje içinde gömülü olanlar çalışır;
  - modeller (`model:` kimlikleri) sunucuda çalışmaz;
  - Linux'ta `/proc` ölçümünün gerçek bir Linux sunucuda sınanması (bu işte yalnız ayrıştırıcıları birim testli, macOS'ta `ps` yolu
    gerçek süreçle sınandı) ve cgroup ile işletim sistemi düzeyinde bellek tavanı;
  - kullanıcı başına iş kotası (bugün kurum başına sayı ve iş başına CPU/bellek var);
  - yedek ve geri yükleme tatbikatı, önbellek anahtarı (ADR 0542'den kalan).

## Bağlam

İşlemler'in yüzlerce aracı masaüstünde kendi iş parçacığında çalışıyordu; pencere kapanınca iş de bitiyordu. ADR 0542 sunucuya kalıcı
iş getirdi ama yalnız iki türü vardı (dışa aktarma, kontrol noktası) ve işin neye mal olacağı sınırlı değildi. Büyük bir parsel
kesişimini ya da tamponu kullanıcının bilgisayarında saatlerce çalıştırmak yerine veritabanı projesinin üstünde sunucuda
çalıştırmak, sonucunun projeye kesinleşmesini işçinin ölmesine ve başkasının araya girmesine karşı güvenli yapmak ve işin ilerlemesini
kullanıcının görmesi gerekiyordu. İşçi ilerlemeyi yazamıyordu (`beat_job` alıyor, işler yazmıyordu); CPU ve bellek sınırı yoktu.

## Karar

### 1. Yeni iş türü: `project.processing`

- Girdi `ProcessingJobInput`'tur: `cad.processing.run`'ın girdisi (aracın kimliği, değerleri ada göre, seçili nesneler kalıcı
  kimlikleriyle, görünür alan kutusu) ve sonucun nereye gideceği (`mode`: `commit` ya da `sink`), isteğe bağlı `expectedRevision`, daha küçük
  `cpuSeconds` ve `memoryMb`, listeler için `label`. Aynı çalıştırıcı çalışır: işi sunucuda yapan, başsız ev sahibinin `cad.processing.run`'ıdır
  (Python ve MCP'nin yolu); hesap ikinci kez yazılmaz.
- İstek **isteme anında denetlenir** (`check_input`, bütün retler yoluyla; sözleşmenin `processing_job` modülü, çocuk süreçle ortak):
  - araç kimliği 1–200 karakter, değer sayısı ≤ 256, seçili nesne ≤ 100 000 ve küçük harfli UUID, kutu sonlu ve sıralı,
    `expectedRevision` sayı, `cpuSeconds` ≥ 1, `memoryMb` ≥ 64;
  - **yerel dosya yolu yok**: değerlerde herhangi bir derinlikte `{ "path": "…" }` biçimi reddedilir (`input.parameters.<ad>.path`
    yoluyla), çünkü iş sunucuda çalışır ve sunucunun dosyasını okumamalıdır; tablo `{ name, rows }` olarak verilir. Sonuç dosyasının yolunu istek
    vermez, sunucu verir.
- Yetki: `jobs.run` ve **commit için** `feature.write` (+ projeyi yazılabilir: arşivde değil), **sink için** `project.download` (sonuç, verinin
  bir dosyasıdır; dışa aktarma gibi). Dosya projesinde iş yoktur (nesneler tek tek yazılmaz): iletiyle reddedilir. Rol tablosunda
  `jobs.run` düzenleyici ve üstündedir ve `feature.write`'ı da getirir.
- **Sabitlenen revizyon:** aracın okuduğu proje, `expectedRevision` verildiyse onunki, verilmediyse işçinin işi başlatırken okuduğu
  (kuyrukta beklerken değişen proje bir işi boşa düşürmesin); aracın okuduğu revizyon sonuçta `baseRevision`dır. `expectedRevision` projenin
  revizyonu değilse iş araç çalışmadan `revision_conflict` ile düşer.

### 2. Sonuç: tek komutla kesinleşme ya da dosya

**`commit`:** araç, projenin o revizyondaki `.kcad` görüntüsünün (ADR 0033) bir kopyasında çalışır. Sonuç **delta**'dır
(`kentos_contracts::processing_job::delta`): çizimin açıldığındaki hâli ile araçtan sonraki hâli, ikisi de aynı belge koduyla yazılmış
iki görüntü olarak karşılaştırılır; böylece aracın dokunmadığı nesne geri gönderilmez, nesnenin yuva numarası içerik sayılmaz.
- yeni kalıcı kimlik: oluştur; kendisine göre değişen: güncelle; kalmayan: sil (bu sırayla);
- katman ağacı, ayarlar, ad, etkin katman, stiller, pafta kitabı, diziler ve kısıtlar farklıysa bütünüyle projenin yamasına girer;
- blok tanımları farklıysa iş `unsupported_change` ile düşer (bu sürümde blok yazılmaz); komut en çok 5000 nesne alır
  (`too_many_changes`).

Kesinleşme **işin bitirme işleminin içindedir**: `finish` ADR 0542'deki gibi yetkiyi yeniden sorar, sonra aynı işlemde
`changes::commit_in` (`project.changes`'in işlem dışarıdan verilen hâli) çalışır, ardından `finish_job` kiranın çitini denetler:
- proje **araç okuduğu revizyonda değilse** (kilit altında bakılır) hiçbir şey yazılmaz, iş `revision_conflict` ile düşer;
- iş sürerken kullanıcının yetkisi alındıysa ya da nesne kilitli katmandaysa `forbidden`; metadata (katman ağacı vb.) `project.edit` ister
  (editör araç sonucu nesne yazar ama katman açtıran aracın işi `forbidden` ile düşer);
- kira kaybedildiyse (`finish_job` geçmez) işlem geri alınır: **commit de gider** (bu işin sınanan en önemli kuralı: ilk uygulamada
  geçmeyen bitirme işlemi yine de yazıyordu; bir test yakaladı);
- komutun kimliği `job-<iş kimliği>`dir: aynı iş iki kez kesinleşmez;
- nesne sürümleri aynı işlemde okunur; proje revizyonu eşit olduğundan okunan sürümler aracın okuduğundur.

**Formların gizlediği alanlar:** araç, çağıranın alabileceği kopyada (ADR 0550) çalışır. Kopya `redact` alanları kesmişse sonucu projeye yazmak
o alanları bozar: commit modu `forbidden` ile reddedilir ve dosya moduna yönlendirir (dosya kesilmiş kopyanın sonucudur).

**`sink`:** araç kopyada çalışır, yaptığı nesneler GeoPackage'a yazılır (ADR 0508 §4); dosya iş artifact'ı olarak, dışa aktarmanın yoluyla
(nesne deposu, boy sınırı `too_large`, `storage`) saklanır; **proje değişmez**. Dosya adı `<proje> - <araç>.gpkg`.

Sonuç JSON'u (`ProcessingJobResult`): araç, kip, `baseRevision`, `revision` (commit), aracın özet cümlesi, eklenen/değişen/silinen
sayıları, metadata değişti mi, aracın notları, **ne harcandığı** (`usage`).

### 3. İşçi ve ayrı süreç

Sunucu crate'i araçları bilmez (mimari kural: `scripts/arch/deps.mjs`); araçları çalıştıran bir **`Processor`** arayüzü vardır
(`kentos_application::processing_job`). Üretimdeki gerçekleştirmesi `kentosd`'nin `ChildProcessor`'ıdır: aracı **ayrı bir süreçte**
(`kentos-jobrun`, `apps/jobrun`) çalıştırır. Neden ayrı süreç:
- **CPU ve bellek ölçülür ve aşılınca öldürülür** (iş parçacığı öldürülemez, süreç öldürülür); bir aracın bellek taşması sunucuyu değil
  yalnız işi götürür;
- aracın çökmesi (abort, sinyal) sunucuyu götürmez (`crashed`);
- **sunucunun sırları çocukta yoktur**: çocuk boş ortamla başlar (veritabanı adresi, JWT anahtarı çocuğa geçmez); çocuk veritabanına ve ağa
  ihtiyaç duymaz, bir klasörü okur ve yazar.

Ayrı **paket** olmasının nedeni mimaridir: başsız ev sahibi, İşlemler, GeoPackage ve çizim hattı `kentosd`'ye bağlanamaz (api grubu onlara
bağlanamaz). `kentos-jobrun` kendi grubudur (`jobrun`); `kentosd` yalnızca yanındaki ikiliyi başlatır (`KENTOS_JOBRUN` başka yeri söyler).
Sonuç: `kentosd` hata ayıklama ikilisi 264 MB'tan 97 MB'a indi.

Protokol (bir klasör):
- giriş: `request.json` (`ProcessingJobInput`), `snapshot.kcad` (işçinin okuduğu proje);
- çıkış: `answer.json` (özet, notlar, delta ya da sink satır sayısı), `sink.gpkg`; ya da `failure.json` (kod ve ileti) ve çıkış 2;
- standart çıktı: ilerleme satırları `KENTOS-PROGRESS 0.4000 sözler`;
- standart giriş: ebeveyn açık tutar; kapanınca (ebeveyn öldü) çocuk 3 ile biter: sahipsiz iş kalmaz.

Çocuk aracı başsız ev sahibinin protokol işiyle (`start`, `job`) çalıştırır (ilerleme oradan gelir) ve süreç kuralları:
`forbid_local_files` (araç hiçbir yerel dosyayı okuyamaz, yazamaz: raster ve nokta bulutu araçları dosyasız ev sahibinde `tool_failed` verir), sink yolu
çocuğun kendi klasörüdür, istek çocukta **yeniden** denetlenir. Çalışma klasörü geçici klasörde 0700, iş bitince silinir.

### 4. CPU ve bellek bütçesi

- **Sunucunun** sınırı: `KENTOS_JOB_CPU_SECONDS` (varsayılan 1800) ve `KENTOS_JOB_MEMORY_MB` (varsayılan 4096). İstek daha küçük bir
  bütçe isteyebilir (`cpuSeconds`, `memoryMb`), büyüğünü isteyemez (`budget_of`).
- **İş başlamadan red:** projenin dosyasının büyüklüğünden aracın ihtiyacı kestirilir: `192 MB + 24 × dosya boyu`
  (`processing_memory_estimate`; belge bütünüyle okunur ve bir de çalışma kopyası vardır). Kestirim bütçeyi aşıyorsa iş `memory_estimate`
  ile düşer, araç hiç çalışmaz ve ileti hem sayıları hem çareyi söyler.
- **Çalışırken durdurma:** ebeveyn çocuğun CPU süresini ve çekirdek belleğini 100–200 ms'de bir ölçer (Linux'ta `/proc/<pid>/stat` ve `status`,
  başka yerde `ps`) ve bütçeyi aşarsa çocuğu öldürür: `cpu_limit`, `memory_limit`. Sistem çocuğu belleğe yakınken 9 sinyaliyle öldürdüyse (OOM) bu da
  `memory_limit`'tir.
- Bu kodlar kalıcıdır: iş **yeniden denenmez** (deneme 1), hiçbir şey yazılmaz. `stopped` (işçi durdurdu) iptal ya da kira kaybıdır.
- Harcanan, sonuca `usage` olarak yazılır (CPU saniyesi, en çok bellek MB).

### 5. İlerleme

İşçinin vuruşu (`beat_job`) artık ilerleme ve **birkaç Türkçe sözcüğü** de yazar (`progress_text`, göç 0031): işin doğrulanan 0–1'i ve
“Proje okunuyor”, “Araç çalışıyor: Tampon”, “Değişiklikler bulunuyor”, “Sonuç projeye yazılıyor” gibi sözler. Çocuk aracın kendi ilerlemesini
(protokol işinin kesri) 0,10–0,85 aralığına eşler; aşamalar öncesi ve sonrasıdır. Vuruş `min(kira/3, 2 sn)` aralıkla gider, kullanıcı işi yoklayarak
okur (masaüstünün 2 sn'lik yoklaması). `beat_job` eski dört bağımsız değişkenli biçimini bıraktı (tek çağıranı işçi).

### 6. Masaüstü

- **Nerede çalışır** listesine **“KentOS sunucusunda”** eklenir (araç hangi yerleri söylerse söylesin): açık çizim bulut veritabanı projesiyse listelenir;
  hesap `jobs.run` yetkisine sahipse açık, değilse kapalı ve satırın notu nedeni söyler (“yetkiniz yok (project.jobs.run)”, “proje arşivde”,
  “oturum açık değil”); yerel çizimde, dosya projesinde ve modelde listelenmez. Otomatik, bu bilgisayardaki yerler varken sunucuyu seçmez.
- Seçilince **Sonuç** grubu çıkar: “Sonucu projeye yaz” (varsayılan) ya da “Sonucu dosyaya yaz (GeoPackage)”; **Çalıştır** düğmesi “Sunucuda çalıştır” der.
- Düğme: açık projenin gönderilmemiş değişiklikleri önce gider (`Settle::ProcessingJob`; iş onları görür), sonra iş istenir. İstek kimliği
  başarısızlıkta korunur (aynı kimlikle yeniden istek aynı işi verir). Pencerenin satırı “Sunucu işi istendi: İşlem: <araç>. Sunucuda çalışır;
  pencereyi kapatsanız da sürer.” der ve **Sunucu işleri** düğmesiyle katalogda projenin işler sekmesini açar. Araç burada **çalışmaz**: istek çizimi
  değiştirmez.
- **Sunucu işleri sekmesi** (ADR 0547): satırın başlığı “İşlem: <araç>” (ya da “İşlem (dosyaya): …”), çalışan işte “%62 · Araç çalışıyor: Tampon”,
  biten işte özet · sayılar · veri revizyonu · harcanan (“işlemci 12,4 sn, bellek 612 MB”), dosya modunda boy ve satır sayısı ve İndir (`.gpkg`),
  düşenlerde sunucunun iletisi (`cpu_limit`, `memory_limit`, `memory_estimate`, `revision_conflict`, `unsupported_change` … iletileri öneriyi kendisi taşır).
  Sonuç projeye yazılınca açık çizime başkasının değişikliği gibi gelir (ADR 0044: olayları bekleyerek izler).

### 7. Python

`ProjectRef.processing_job(tool, parameters=, selection=, view=, mode="commit"|"sink", expected_revision=, label=, cpu_seconds=,
memory_mb=, request_id=, timeout_seconds=)`; ötesi `wait_job`, `job`, `cancel_job`, `job_artifact` (dosya modunda GeoPackage baytları). İşin
`progress` ve `progressText`'i ve `result`'ı sözlükte gelir.

### 8. Sözleşme

`PROJECT_PROCESSING_JOB`, `ProcessingJobInput`, `ProcessingJobMode`, `ProcessingJobResult`, `ProcessingJobNote`, `ProcessingUsage`,
`processing_memory_estimate`; `ServerJob.progress_text`; `ServerJobError.code`'a `cpu_limit`, `memory_limit`, `memory_estimate`, `tool_failed`,
`tool_unavailable`, `unsupported_change`, `too_many_changes`, `crashed`; `kentos_contracts::processing_job`: `check_input`, `delta`, `Delta`, `Processed`,
`ProcessFailure`. Göç 0031: üçüncü tür ve `progress_text`. Bu bir **komut kataloğu** girdisi değildir (ADR 0542/0547'nin işleri gibi sunucu iş sözleşmesidir);
manifest ve kapsam matrisi değişmez; MCP'ye eklenmedi.

## Doğrulama

- Saf mantık, bağımsız başvuruyla: `python3 scripts/fixtures/processing_job_cases.py --check` KentOS kodu olmadan 24 girdi durumu, 10 bellek kestirimi, 9 bütçe ve
  31 değişiklik (delta) durumu yazar (`fixtures/server-jobs/v1/processing.json`); `cargo test -p kentos-application --test processing_job_cases` aynı
  durumları Rust'ın işlevleriyle karşılaştırır (belge yapımı kompakt tanımdan).
- Veritabanı, sahte işlemciyle (`KENTOS_TEST_DB=required`, geçici PostGIS): `cargo test -p kentos-application --test processing_jobs` (10 test):
  commit tek komutla ve revizyon bir artar, sonuç ve ilerleme sözcükleri araç sürerken yoklamayla okunur; araç sürerken proje değişirse hiçbir şey yazılmaz
  (`revision_conflict`); kira bitişte kaybedilirse commit de gider; yetki araç sürerken alınırsa `forbidden`; sabitlenen yanlış revizyon ve
  sığmayan bellek (`memory_estimate`) araç çalışmadan reddedilir; aracın hata kodları işin kodudur ve yeniden denenmez; durdurma aracı durdurur ve iş
  iptal olur; dosya modu artifact'tır ve proje değişmez; işlemcisiz işçi işi almaz; istek yolu işaretli reddedilir; metadata `project.edit` ister.
- Gerçek çocuk süreç, veritabanı olmadan: `cargo test -p kentos-api --test job_run` (12 test): araç çocukta çalışır ve delta döner, dosya modu SQLite
  verir, bilinmeyen araç ve yerel dosya yolu çocukta da reddedilir, bellek bütçesi çocuğu öldürür, CPU bütçesi CPU harcayan çocuğu öldürür, durdurma çocuğu
  öldürür, cevapsız çocuk `crashed`, hiç başlamayan program `tool_unavailable`; `/proc` ve `ps` ayrıştırıcıları.
- Tüm zincir gerçek çocukla ve veritabanıyla: `cargo test -p kentos-api --test job_run_db`: iki parselli proje, Öznitelik hesapla (3 nesne güncellenir, revizyon 2,
  özellik değeri `600.00`), dosya modu (GeoPackage, proje değişmez), Köşe noktalarını numarala (6 nokta ve yeni katman tek commit'te, revizyon 3) ve revizyon
  eski bir `expectedRevision`'la reddi.
- HTTP: `cargo test -p kentos-api --bin kentosd jobs_tests` (işleme türü: isteme, dosya yolu 422 ve yolu, işlemcisiz işçi, dosya indirme, başka kurumdan 404).
  Önceki testler değişmeden geçer: `server_jobs`, `changes`, `project_checkpoints`, `project_forms`, `persistent_ids`, `blocks`.
- Masaüstü: `cargo test -p kentos-desktop processing::server_tests` (yer, açık/kapalı, istek içeriği, istek kimliğinin korunması, katalogda iş sekmesi) ve
  `cloud::catalog_jobs_tests`; `cargo test -p kentos-cloud --lib server_jobs` (sözcükler, haklar, iletiler).
- **Gerçek `kentosd` ile masaüstü akışı:** `apps/desktop/scripts/jobs-live.sh` (`jobs_live` testi): pencere → sunucu → çocuk süreç → commit → olaylarla çizime dönüş.
  Sonuç: “1 nesnede ‘Hesap alanı’ yazıldı. · 1 nesne değişti · veri revizyonu 2 · işlemci 0,0 sn, bellek 38 MB”.
- Python: `python/tests/test_connection.py`.
- Resimler (koyu/açık, 1440×900 ve 1100×650): `cargo test -p kentos-desktop processing::server_tests::screens -- --ignored --nocapture` (`.run/shots/islem-sunucu-*`:
  seçildi, istendi, yetkisiz, dosya) ve `cloud::catalog_jobs_tests::screens::the_tab_with_processing_jobs` (`.run/shots/isler-islem-isleri-*`).

## Performans

- Süreç başına maliyet: iki parselli projede uçtan uca bir iş (çocuğun açılması, araç, delta, commit) hata ayıklama derlemesinde 0,3–0,7 sn; çocuğun çıkışta bellek ayak izi ≈ 10–40 MB. Ölçüm
  (CPU, bellek) 100–200 ms'de bir ve yalnız çocuk süresince; `ps` yolu macOS içindir, Linux'ta okuma `/proc`'tur.
- Delta, projenin iki görüntüsünü bir kez karşılaştırır: nesne başına bir klon ve bir eşitlik; sınır 5000 değişikliktir.
- Vuruş her ≤ 2 sn'de bir kısa güncellemedir; kiranın yenilenmesi de o vuruşta olur.
