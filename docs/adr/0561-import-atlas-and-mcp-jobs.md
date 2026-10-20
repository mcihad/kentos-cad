# ADR 0561: Dosya içe aktarma ve atlas PDF'i sunucu işi olarak, MCP'de iş araçları

- **Durum:** kabul edildi (2026-10-20). `plan/gis.md` CBS-39'un dördüncü dilimi ([ADR 0559](0559-processing-server-jobs.md)'un “Sonraya
  kalan”ından: büyük içe aktarma ve atlas PDF'i iş türleri, MCP'de iş türleri, blok yazan ve 5000'den çok nesne yazan sonuçlar). Kapsamı
  koordinatörün önerisine uyarak ben belirledim.
- **Bağlam belgeleri:**
  - ADR 0542 (kira, çit, staged sonuç, yetkinin kesinleşmede yeniden sorulması), 0547 (sınırlar, saklama, masaüstü Sunucu işleri sekmesi), 0559
    (ayrı süreç, bütçe, ilerleme, sonucun işin bitirme işleminde kesinleşmesi);
  - 0502, 0522, 0544, 0545, 0257 (`file.import.gpkg`, `.csv`, `.fgb`, `.gml`, `.dxf`), 0534 (`file.export.atlas`, sabitlenen durum), 0550 (kesilmiş
    kopya), 0133 ve 0134 (MCP sunucusu ve sunucu komutları), 0253 (komut protokolü);
  - CLAUDE.md §9.2, §18.
- **Web:** ertelendi (ADR 0228); web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - ~~Shapefile, GeoJSON ve KML içe aktarma işleri~~: [ADR 0563](0563-gis-file-import-commands.md) ile kapandı (`file.import.shp`, `.geojson`, `.kml` komutları;
    iş Shapefile'ı `.zip` arşivi olarak alır).
  - **Devam ettirilebilir (parçalı) dosya yükleme**: hazırlanan dosya tek akışlı bir gövdedir; kesilen gönderim baştan yapılır (yarım bayt saklanmaz). Masaüstü
    istemcisi de dosyayı bütünüyle belleğe okur (`reqwest`'in akış özelliği çalışma alanında yok, yeni bağımlılık eklenmedi).
  - **Yan dosyalar** (`.csvt`, `.prj`, `.xsd`, `.gfs`) hazırlanmaz; CSV ve GML'nin türleri değerlerden, sistemi `srid` seçeneğinden belirlenir.
  - **Toplu SQL yazma**: bir iş sonucunun nesneleri tek tek `insert … on conflict` ile yazılır (nesne başına ≈ 300 µs, aşağıda); 100 000 nesnelik sınır
    bundan konuldu; çok satırlı (`unnest`) yazma ayrı iş.
  - **İkinci iş türleri**: yayın/atlas dışında `file.export.*` (GeoPackage, FlatGeobuf, Shapefile, GML, CSV, DXF, PDF) iş olarak; atlasın bağlı kaynağı
    sunucuda yok (bağlı GeoPackage katmanı yerel dosyaya bağlıdır; sayfalar katmanın nesnelerinden yapılır, `source_stale` ile söylenir).
  - Linux'ta `/proc` ölçümünün gerçek sunucuda sınanması ve cgroup, kullanıcı başına iş kotası, önbellek anahtarı, yedek ve geri yükleme tatbikatı
    (ADR 0542 ve 0559'dan kalan).

## Bağlam

ADR 0559 ile İşlemler araçları sunucuda çalışır oldu, ama kullanıcının asıl ağır işleri iki tanesi daha: yüz binlerce nesnelik bir GeoPackage'ı projeye
almak ve yüzlerce sayfalık atlası PDF'e çizmek. İkisi de masaüstünde pencereyi açık tutar, ikincisi bütün bir CPU çekirdeğini dakikalarca yer
(hata ayıklama derlemesinde 100 sayfa ≈ 60 sn). İkisinin de **aynı** komutu zaten var (`file.import.*`, `file.export.atlas`); eksik olan, onları veritabanı
projesinin üstünde, dosyası kullanıcının bilgisayarında olan bir girdiyle, bütçeli ve kalıcı bir iş olarak çalıştıracak yol, ve bir yapay zekâ ajanının bu işleri MCP'den isteyip
izleyebilmesiydi. Sunucu işlerinin MCP'de aracı yoktu; Python'da vardı.

## Karar

### 1. `project.import`: hazırlanan dosyanın sunucuda içe alınması

**Dosyanın sunucuya gelişi** (`kentos.job_input`, göç 0032): dosya iş istenmeden önce **hazırlanır**.
- `POST …/projects/{p}/job-inputs` `{ name, size, sha256 }` (hak: `project.jobs.run` ve nesne yazma `feature.write`, proje yazılabilir ve veritabanı
  projesi): kayıt açılır (dosya adı tek ad, klasör yok; boy ≤ `KENTOS_JOB_INPUT_MAX_MB`, varsayılan 2 048); cevap `JobInput` (`id`, `received: false`).
- `PUT …/job-inputs/{id}` ham baytlar, tek akışlı gövde (kendi boy sınırı ve süre aşımıyla ayrı yönlendirici): nesne deposuna kare kare yazılır, **boy ve SHA-256
  bildirilenle aynıysa** saklanır; bayt fazlaysa, kesilirse ya da özet tutmazsa hiçbir şey saklanmaz ve aynı dosya yeniden gönderilebilir.
- `GET …/job-inputs/{id}` hazırlanan dosyanın durumu (`received`): cevabı kaybolan istemci bunu sorar. Dosya yalnız hazırlayanındır ve o projenindir
  (RLS ve `created_by`); başka kurumdan 404.
- Saklama: dosyanın satırı ve nesnesi biten işlerle aynı süre kalır (`KENTOS_JOB_RETENTION_DAYS`, `job_upkeep::expire`); satırı olmayan nesneyi
  depo temizliği siler (`files::cleanup`).

**İş** `project.import`, girdisi `ProjectImportInput`: `upload` (hazırlanan dosyanın kimliği), `format` (`gpkg`, `fgb`, `gml`, `csv`, `dxf`; yoksa dosyanın adından:
`ImportFormat::of_name`), `options` (içe aktarma komutunun girdisi **yolsuz**: `tables`, `layers`, `reproject`, `srid`, `layer`, `geometry`, `fileUnit`, `explodeBlocks`,
`group` …), `label`, `expectedRevision`, daha küçük `cpuSeconds` ve `memoryMb`. İstek **istendiğinde** denetlenir (`check_import_input`; ortak sözleşme
modülü, çocukla ortak): kimlik küçük harfli UUID, seçeneklerde dosya yolu yok (`input.options.path`, iç içe `{ "path": … }` de), seçenek sayısı ≤ 256,
revizyon ve bütçe aralıkları; ardından **hazırlanan dosyanın var, çağıranın, baytlarının gelmiş ve biçiminin belli olduğu** (`input.upload`, `input.format`).
Hak: iş isteme + `feature.write`; dosya projesinde iş yoktur.

**Çalışma:** işçi, ADR 0559'daki gibi projeyi sabit bir revizyonda okur (`expectedRevision` ya da işçinin başladığında okuduğu), bütçeyi ve **bellek kestirimini**
denetler (`192 MB + 24 × proje dosyası + 32 × içe aktarılan dosya`, `import_memory_estimate`; sığmıyorsa `memory_estimate`, araç hiç çalışmaz), hazırlanan
dosyanın nesne deposunda olup boyunun doğru olduğuna bakar (yoksa `gone`) ve `Processor`'a verir. `ChildProcessor` `kentos-jobrun`'ı çalıştırır; dosya çocuğun
klasörüne **bağlanır** (sembolik bağ, bağlanamazsa kopya; bellek kullanmaz; bağ dosyanın mutlak yoluyla kurulur: depo kökü göreli olabilir,
`KENTOS_BLOB_DIR=.run/blobs`; bu hata gerçek sunucuda bulundu ve düzeltildi, testi var). Çocuk **masaüstünün aynı komutunu**
(`file.import.gpkg` …) projenin görüntüsünde çalıştırır; yolu klasördeki dosyadır, başka yol veremez.

**Sonuç:** işin bitirme işleminde, ADR 0559'daki kurallarla (yetki yeniden sorulur, proje aracın okuduğu revizyonda değilse hiçbir şey yazılmaz, geçmeyen kira
commit'i de geri alır) tek `project.changes` komutu olarak kesinleşir. İki değişiklik bunun için gerekti:
- **Delta artık blok tanımlarını taşır.** Çizimin açıldığı ve işten sonraki blok tanımları kimliğiyle karşılaştırılır; yeni kimlik `create`, kendisine göre
  değişen `update` (bütün tanım), kalmayan `delete`; sürümleri (`expectedVersions["block:<id>"]`) kesinleşmede aynı işlemde okunur. ADR 0559'un
  `unsupported_change`'i (blok yazmayan sonuçlar) bu yüzden bu türde artık çıkmaz; DXF'in blokları bu yolla projeye gelir. (İşlemler araçları da blok
  yazabilir; `unsupported_change` kodu sözleşmede kalır, bu sürümde üretilmez.)
- **Bir işin kesinleşmesi 100 000 değişikliğe kadar** (`JOB_CHANGES_MAX`; ADR 0559'da 5000'di): istek olarak gelen `project.changes` komutu 5000'de kalır
  (`MAX_CHANGES`), işin kendi kesinleşmesi sunucunun içinde yapıldığı ve gövdesi bir istek olmadığı için daha büyük olabilir (`commit_in`'in
  `max_changes` bağımsız değişkeni). Aşan sonuç `too_many_changes` ile düşer, mesajı çareyi söyler.

**Sonuç JSON'u** (`ProjectImportResult`): komut (`file.import.gpkg`), dosyanın adı, baytı ve SHA-256'sı, okunan revizyon, kesinleşen revizyon, komutun cümlesi, eklenen
(değişen, silinen) nesne sayısı, metadata değişti mi, getirilen blok sayısı, **notlar** (komutun uyarıları: `import_losses` — okunamayan satırlar, `crs_undeclared`, `unit_assumed` …
kodlarıyla; kayıplar böylece sonuçta söylenir), `details` (komutun kendi cevabı: tablolar, katmanlar, sistem), ve ne harcandığı.

**Komutun reddi işin kodudur:** `crs_mismatch`, `import_unreadable`, `nothing_to_import`, `entity_exists`, `layer_not_found` …; çocuk komutun kodunu
(`^[a-z][a-z0-9_]{1,63}$` biçimindeyse) olduğu gibi söyler, öbürleri `failed` olur (`Outcome::Refused`). Yeniden denenmez.

**Bilinen sınır:** sunucunun nesne yazma kuralı (`to_stored`) en az 3 köşeli alan ister; yaylı kenarla 2 köşeli alanı (daire gibi, ADR 0069) içeren bir dosyanın
komutu masaüstünde kabul eder ama sunucuda kesinleşirken `invalid` verir (ADR 0563 §4'te düzeldi) (“Alan en az 3 köşe ister”; gerçek GeoPackage'da görüldü). İş bu durumda hiçbir şey yazmaz ve
iletide nesnenin sırasını söyler. Sunucunun bu kuralı ayrı iştir.

### 2. `project.atlas`: atlasın sunucuda PDF'e çizilmesi

Girdisi `ProjectAtlasInput`: `file.export.atlas`'ın girdisi **yolsuz**: `sheet`, `fields`, `theme`, `time`, `crs`, **`pin`** (önizlemenin sabitlediği durum, ADR 0534 §2),
`label`, `expectedRevision`, bütçe. Hak: iş isteme + `project.download` (PDF projenin verisinin bir dosyasıdır; dışa aktarma gibi). Dosya projesinde iş yoktur; arşivdeki proje
atlas verir (okur).

Çocuk komutu projenin görüntüsünde `atlas.pdf` yoluyla çalıştırır. **Sabitlenen durum masaüstündeki gibi denetlenir**: komut, okuduğu proje ile pin'in özetini
(ayarlar, katmanlar, stiller, paftalar, her katmanın nesne özeti) karşılaştırır; farklıysa `pin_mismatch` ve iş düşer, hiçbir şey yazılmaz (ve atlas planını yenileyip
yeniden istenir). Masaüstünde planın pin'i ile sunucunun veritabanından kurduğu görüntü **aynı** durumdur: gerçek `kentosd` ve gerçek çocukla masaüstü penceresinin pin'iyle
sayfalar yapıldı (canlı test), ve ayrıca bağımsız bir planla (Python SDK) üretilen pin veritabanı projesinde tuttu (`job_run_db`).

Sonuç PDF'tir: iş artifact'ı (`<proje> - <pafta ya da etiket>.pdf`, `application/pdf`), **Range ile indirilir** (ADR 0045/0547'nin yolu); proje değişmez. Sonuç JSON'u
(`ProjectAtlasResult`): pafta, okunan revizyon, sayfa sayısı, ilk ve son sayfanın adı, pin'in özeti, bağlamalar (sistem, GeoPDF mi), sayfaların taşıdığı ve
**taşımadığı** öznitelikler (formun `redact`'ı dahil), komutun uyarıları ve notları (`source_stale`, resim kutusu notu …), harcanan.

Formların gizlediği alanlar (`redact`): atlas **kesilmiş kopyada** çalışır (ADR 0550) ve zaten bu alanları taşımaz; bu yüzden commit değil dosya olan bu iş `snap.cut` ile
reddedilmez. Kesilmiş kopyanın özeti tam kopyanınkinden farklı olabilir; pin'i tam kopyadan planlayan kullanıcıya `pin_mismatch` gelir ve iletisi bunu söyler.

### 3. Çocuk süreç: isteğin biçimi ve ölçüm

- **`RunRequest`** (sözleşmede, `request.json`): `{ kind: "processing" | "import" | "atlas", input }`. Çocuk üç işi de yapar: araç (ADR 0559), içe aktarma
  (`import.<uzantı>` dosyası), atlas (`atlas.pdf`); çıkış: `answer.json` (`Processed`: cümle, notlar, delta, komutun cevabı) ve `sink.gpkg` ya da `atlas.pdf`, ya da `failure.json`.
- `Processed.sink` → `file`, `sink_rows` → `rows`, ve `details` (komutun cevabı).
- **Kullanım ölçümü çocuğun kendi sözüyle de gelir** (§5): çocuk bitmeden önce kendi işlemci süresini ve en çok belleğini söyler (Linux'ta `/proc/self/status`'un `VmHWM`'i ve
  `/proc/self/stat`; başka yerde `ps`), ebeveyn de örneklediğiyle birleştirir (büyüğü).

### 4. MCP: `jobs.start`, `jobs.status`, `jobs.cancel`, `jobs.result`

Ortamdaki hesapla (ADR 0134: `KENTOS_URL`, `KENTOS_LOGIN`, `KENTOS_PASSWORD`; model vermez), `x-kentos-client: mcp`. Hepsinde `tenant` ve `project`.
- **`jobs.start`** `kind` (`export`, `checkpoint`, `processing`, `import`, `atlas`; `project.` öneki isteğe bağlı), `input` (türün girdisi), `requestId`, `timeoutSeconds`; **`import`'ta
  `file`**: MCP sunucusunun çalıştığı bilgisayardaki dosyanın tam yolu: araç dosyayı okur, boyunu ve SHA-256'sını bildirir, gönderir ve işin `upload`'ını kendisi koyar (`label` dosyanın adı). `file` yalnız
  içe aktarmadadır. İşaretler: yazar (kesinleşen işler projeyi değiştirir), `openWorldHint`.
- **`jobs.status`** işin durumu, ilerlemesi ve sözcükleri; `waitSeconds` (≤ 120) iş bitene kadar saniyede bir sorar. Başarısız iş `isError` ile gelir (kodu `error.code`).
- **`jobs.cancel`** sırada olanı hemen, çalışanı çalıştıranın bir sonraki yoklamasında durdurur.
- **`jobs.result`** biten işin sonucu; dosyası varsa **`savePath`** (tam yol) ile bu bilgisayara yazılır (var olan dosyanın üstüne yalnız `overwrite: true`); dosyanın baytları modele verilmez, boyu ve
  SHA-256'sı söylenir. Bitmemiş iş için `job_not_finished`.
- Kodlar: sunucunun reddi `status`, `code`, `message`, `path`, `retryable` ile; yerel hatalar `invalid_input`, `file_exists`, `file_unwritable`.

### 5. `job_run` testlerindeki bir kez görülen düşüşün kök nedeni

ADR 0559'da son biçimlendirmeden sonra `job_run`'ın iki testi bir kez düşmüş, sonra altı koşuda geçmişti. **Kök neden:** ebeveyn çocuğun işlemci süresini ve belleğini 100–200 ms'de
bir örnekler; ilk örnek çocuk daha başlamadan alınır. Küçük bir araç (iki parsel) 100 ms'den kısa sürebilir: o zaman çocuk iki örnek arasında biter, hiç örnek alınamaz,
sonuçta `usage` sıfır ve bütçe hiç denetlenmez. İki test tam buna dayanıyordu (`peak_memory_mb > 1.0` ve 1 MB bellek bütçesinin çocuğu öldürmesi); makine meşgulken ya da ikili
önbellekteyken çocuk daha hızlı biter. Aynı yarış gerçek sunucuda masaüstü testinde (`jobs_live`) de görüldü (`ProcessingUsage { 0.0, 0.0 }`). **Düzeltme:**
- ebeveyn çocuk başlarken ilk 2 saniyede dörder kat sık örnekler;
- çocuk kendi kullanımını bitmeden söyler (§3), ebeveyn büyüğünü alır: kısa işin kullanımı artık sıfır değildir;
- bütçeyi **çocuğun örneklenmesine bağlı** bırakmayı sınayan test, ömrü uzun bir çocukla (`sleep 30`) yapılır: bütçenin kuralı artık işin hızına bağlı değil; ve işin bütçeyi ancak örnek
  alındığı sürece zorladığı (iki örnek arasında biten işin zorlanmadığı) açıkça sözleşmedir: bu iş zaten bitmiştir ve kullanımı sonuçta söylenir.

### 6. Masaüstü

- **Sunucu işleri sekmesi** iki türü bilir: satır başlığı “İçe aktar: <dosya>” ve “Atlas (PDF): <pafta>”; çalışan işte ilerleme ve çalışanın sözcüğü; biten içe aktarmada
  komutun cümlesi, veri revizyonu, uyarı sayısı (“1 uyarı (ayrıntı iş sonucunda)”), harcanan; biten atlasta “100 sayfa · 3,2 MB · veri revizyonu 11 · 1 alan taşınmadı”, **İndir** (`.pdf`);
  komut kodlarının önerisi (`pin_mismatch`: atlas planını yenile; `crs_mismatch`: Projenin sistemine çevir; `gone`: dosyayı yeniden gönder).
- **İndir doğru uzantıyla kaydeder** (bir hatanın düzeltmesi): ADR 0559'da araç sonucu GeoPackage de `.kcad` filtresi ve uzantısıyla kaydediliyordu. Kayıt penceresi artık
  işin dosyasının uzantısını (`kcad`, `gpkg`, `pdf`) önerir ve ekler (`server_jobs::download_extension`).
- **“Sunucuya içe aktar…”** GeoPackage, FlatGeobuf, GML, CSV ve DXF içe aktarma pencerelerinin altında; **“Atlası sunucuda yap…”** Atlası PDF yap penceresinde. Düğme, açık çizim bir
  bulut **veritabanı** projesiyse görünür (yerel çizimde ve dosya projesinde yok); hesabın hakkı yoksa kapalı (içe aktarma için `feature.write`, atlas için `project.download`,
  ikisi için `project.jobs.run`). Düğme, pencerenin seçimini bir **iş** olarak taşır (`Work`: komutun girdisi yolsuz, dosya, seçimin sözle kaydı; atlasta önizlemenin pin'i)
  ve ortak pencereyi (`exchange/server_work.rs`) açar: hangi dosyanın hangi projeye gideceğini, sonucun tek değişiklik olarak yazılacağını (proje değişirse hiçbir şey yazılmaz), yan
  dosyaların gitmediğini söyler; **Sunucuya gönder** önce açık projenin gönderilmemiş değişikliklerini yollar (iş onları görsün), sonra dosyayı gönderir ve işi ister;
  istek kimliği cevap gelene kadar korunur (kaybolan cevaptan sonra aynı istek aynı işi verir); pencere “Sunucu işi istendi” der, **Sunucu işleri** o projenin sekmesini açar;
  iş pencere ve uygulama kapansa da sürer, sonuç projeye açık çizimin izlediği olaylarla gelir (ADR 0044).
- Hakkı olmayan hesap için **Gönder** kapalıdır ve gönderilse de istemci istemez (neden söylenir).

### 7. Python

`ProjectRef.stage_file(dosya_ya_da_bayt, name=)`, `import_job(dosya_ya_da_bayt | upload=, format=, options=, expected_revision=, label=, cpu_seconds=, memory_mb=, request_id=, …)` ve
`atlas_job(sheet, fields=, theme=, time=, crs=, pin=, …)`; ötesi `wait_job`, `job`, `cancel_job`, `job_artifact` (atlasın PDF'i, ADR 0559'daki gibi); standart kütüphane (`urllib`), dosya
tek akışlı gövdeyle gider (`Content-Length`).

### 8. Sözleşme ve veri

`PROJECT_IMPORT_JOB`, `PROJECT_ATLAS_JOB`, `ImportFormat`, `ProjectImportInput`, `ProjectImportResult`, `ProjectAtlasInput`, `ProjectAtlasResult`, `JobInputBegin`, `JobInput`,
`import_memory_estimate`, `kentos_contracts::processing_job`'da `RunRequest`, `ImportRun`, `check_import_input`, `check_atlas_input`, `JOB_CHANGES_MAX`, `Delta.blocks`. Göç 0032: iki tür, `kentos.job_input`,
`expire_job_inputs`, `existing_job_inputs`. Ayarlar: `KENTOS_JOB_INPUT_MAX_MB`. Bu bir **komut kataloğu** girdisi değildir (ADR 0542/0547/0559 gibi sunucu iş sözleşmesi);
manifest ve kapsam matrisi değişmez, çünkü çocuğun çalıştırdığı `file.import.*` ve `file.export.atlas` komutları zaten kayıtlıdır.

## Doğrulama

- Saf mantık, bağımsız başvuruyla: `python3 scripts/fixtures/server_job_kinds_cases.py --check` KentOS kodu olmadan 23 içe aktarma girdisi, 23 atlas girdisi, 18 dosya adı → biçim ve komut, 12 bellek
  kestirimi yazar (`fixtures/server-jobs/v1/kinds.json`); `cargo test -p kentos-application --test job_kinds_cases` aynı durumları Rust'ın işlevleriyle karşılaştırır.
  `processing_job_cases.py` delta durumları blok tanımlarını ve 100 000'lik sınırı kapsar (36 durum: oluşturulan, değişen, silinen blok, ad değişince değiştirilen, nesne ve blok toplamı sınırı).
- Veritabanı, sahte işlemciyle (`KENTOS_TEST_DB=required`, geçici PostGIS, 19 test): hazırlanan dosyanın boy ve özet denetimi, bayt akışı kesilince hiçbir şeyin saklanmaması, başkasının ve
  gelmemiş dosyanın reddi, biçimin adından anlaşılması; içe aktarmanın tek revizyonda kesinleşmesi, sonuçta uyarıların ve komut cevabının bulunması; **5000'den çok** (6 001) nesnenin tek commit'i
  (istekle aynı küme reddedilir); blok tanımlarının eklenmesi, değiştirilmesi ve silinmesi (sürümleriyle); depodan kaybolan dosyanın `gone`'u; komut kodlarının işin kodu olması ve bozuk
  kodun `failed`'e inmesi; proje değişince hiçbir şeyin yazılmaması; atlasın PDF artifact'ı, sonucundaki sayfa ve pin özeti ve `pin_mismatch`'in tekrar denenmemesi; saklama ve temizlik. HTTP:
  `job_inputs` rotaları (boy, özet, başka kurum 404) ve `a_file_is_staged_imported_by_a_job_and_an_atlas_downloaded_over_http` (atlasın PDF'i `application/pdf` ve Range ile).
- Gerçek çocuk süreç, veritabanı olmadan (`cargo test -p kentos-api --test job_run`, 18 test): GeoPackage içe aktarma (komutun cevabı, sayısı), başka sistemdeki dosyanın `crs_mismatch`'i
  ve `reproject` ile alınması, DXF'in blok tanımları, yolun reddi, **atlasın PDF'i** (6 sayfa, pin tutuyor), bozuk pin'in `pin_mismatch`'i, olmayan paftanın `unknown_sheet`'i, göreli yolla bağlanan dosya.
- Tüm zincir gerçek çocukla ve veritabanıyla (`job_run_db`, 8 test): GeoPackage 3 nesne (revizyon 2), başka sistem reddi, `reproject`, DXF'in blokları projede; atlas (**Python SDK'nın planladığı pin
  veritabanı projesinde tuttu**, 6 sayfa PDF, nesneler değişmedi), bozuk pin, olmayan pafta.
- **Gerçek `kentosd` ile masaüstü akışı** (`apps/desktop/scripts/jobs-live.sh`, `jobs_live` 7. ve 8. adım): atlas projesi sunucuya yüklenir, Atlası PDF yap penceresi önizlemenin pin'iyle işi ister, Sunucu işleri
  sekmesinde biter ve PDF indirilir (özeti iş sonucunun SHA-256'sı); GeoPackage penceresi (Projenin sistemine çevir) işi ister, biter, proje büyür.
- **Gerçek `kentosd` ile Python ve MCP** (`scripts/python/live.py`): `import_job` (aynı istek kimliği aynı iş), başka sistem reddi, `export_job` ile dosya; MCP `jobs.start`/`status`/`result`
  (`savePath`) ve başka sistemdeki dosyanın `isError` ile `crs_mismatch`'i.
- MCP, sahte sunucuyla gerçek süreçte (`cargo test -p kentos-mcp --test jobs`): hazırlama, bayt, istek içeriği, bekleme, sonuç dosyası, var olan dosyaya yazmama, göreli yolun reddi, başarısız işin `isError`'ı.
- Masaüstü: `cargo test -p kentos-desktop exchange::server_work_tests` (düğmenin yeri ve hakları, her pencerenin seçiminin iş olarak söylenişi, atlasın pin'i, isteğin bir kez kurulması ve kimliğin korunması, sunucu işleri
  sekmesine yol) ve `cloud::catalog_jobs_tests` (iki türün satırı, indirme uzantısı), `cargo test -p kentos-cloud --lib server_jobs`.
- Resimler (koyu/açık, 1440×900 ve 1100×650): `cargo test -p kentos-desktop exchange::server_work_tests::screens -- --ignored --nocapture` (`.run/shots/islem-sunucu-ice-aktar-*`: GeoPackage ve DXF
  penceresinde düğme, ortak pencere seçildi/istendi/yetkisiz, atlas penceresi ve sunucuya yapma) ve `cloud::catalog_jobs_tests::screens::the_tab_with_import_and_atlas_jobs`
  (`isler-ice-aktar-atlas-isleri-*`); gerçek sunucuyla `bulut-isler-08…13`.

## Performans

- Bir içe aktarmanın kesinleşmesi (hata ayıklama derlemesi, emüle amd64 PostGIS, işçi sahte): 50 000 nokta ≈ 15 sn, nesne başına ≈ 300 µs; sınır (100 000) ≈ 30 sn
  (`KENTOS_PERF_CHANGES=50000 … import_timing -- --ignored`). Bu süre işin bitirme işleminin içindedir (proje kilidi altında; başkalarının commit'i bekler): işin süresi ve kiranın yenilenmesi vuruşta sürer.
- 6 001 nesne ≈ 1,5 sn. Çocuğun belleği dosya boyunun ≈ 32 katı kestirilir (kestirim gerçek ölçümlere bağlı değil, tutucu; her iş sonuçta ölçülen belleği söyler ve kestirim buna göre ayarlanır).
- Dosya depodan çocuğa kopyalanmaz (bağ); sunucu dosyayı belleğe almaz (akış). Masaüstü dosyayı belleğe okur.
