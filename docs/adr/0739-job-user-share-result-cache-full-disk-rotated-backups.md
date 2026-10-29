# ADR 0739: Sunucu işlerinde kullanıcının payı, sonuç önbelleği, dolu disk ve düzenli yedek

- **Durum:** kabul edildi (2026-10-29). `plan/gis.md` CBS-39'un beşinci dilimi. Kapsamı ben belirledim.
- **Kalan işler:**
  - ADR 0547, 0559 ve 0561'in "kullanıcı başına kota"sı;
  - planın "Cache anahtarı source revision+query+CRS/style/time+yetki kapsamını içersin; cache temizleme kaynak veriyi
    silmesin"i ve ADR 0542'den beri kalan "önbellek anahtarı";
  - planın G2 ölçütü "… **disk dolması**, yarım upload … ile sınanmalı"sı;
  - [ADR 0650](0650-server-backup-and-restore-drill.md)'nin kalanı "düzenli yedek".
- **Bağlam belgesi:**
  - ADR 0542 (dayanıklı sunucu işi), 0547 (sınırlar, saklama süresi);
  - ADR 0559 ve 0561 (işçinin çalıştırdığı araç, içe aktarma ve atlas);
  - ADR 0550 (tek erişim çözücüsü ve damgası), 0253 §2 (kanonik JSON);
  - ADR 0031 (nesne deposu), 0045 (parçalı yükleme), 0650 (yedek ve geri yükleme tatbikatı).
- **Web:** web'in işler listesi yok (ADR 0228). TODOS.md §0.2'ye yazıldı: satırın “önceki işten alındı” notu ve
  `storage_full`'un sözü.
- **`.kcad`:** değişmez. **Sunucu:** göç 0036 (`server_job`'un `cache_key`, `cached_from`, `fresh` sütunları ve
  `kentos.job_room`).
- **Bağımlılık:** yeni üçüncü taraf kitaplık yok.
- **Sonraya kalan:**
  - PITR (WAL arşivi) ve RPO/RTO'nun ölçülmesi; S3 uyumlu nesne deposu;
  - işe başlamadan boş yer sorma (yer payı): `statvfs` için kitaplık ister, şimdilik yazarken anlaşılır (§3);
  - önbelleğin kazancının `kentosd jobs` sayılarında görünmesi;
  - içe aktarma ve projeye yazan araç önbelleğe girmez (§2): sonuçları projeyi değiştirir.

## Bağlam

Sunucu işleri (ADR 0542–0561) kurum başına sayılıyordu (`KENTOS_JOB_LIMIT`). Bir kullanıcı elli dışa aktarma isterse
kurumun diğer kullanıcıları sıra bekliyordu.

Aynı projenin aynı hâlinden aynı dosya her istekte yeniden yapılıyordu. Büyük bir projenin `.kcad`'i ya da yüz sayfalık
bir atlas saniyeler sürer. Çoğu zaman proje arada değişmemiştir: aynı ekip aynı dosyayı birkaç kez ister. Planın istediği
anahtar budur: kaynağın revizyonu, sorgu, sistem, stil, zaman ve yetki kapsamı.

Disk dolunca ne olduğu sınanmamıştı. ADR 0547 yazılamayan depoyu `storage` diye düşürüyordu; ama dolu disk ile yetkisiz
klasör aynı söz, aynı yeniden deneme bilgisiyle geliyordu. Parçalı yüklemenin parçası yarım yazılıp kalabiliyordu.

ADR 0650'nin yedeği tek seferlikti: her seferinde bütün nesneler kopyalanıyor, eski yedekleri kimse silmiyordu.

## Karar

### §1 Kullanıcının payı

- Kurumun sırada ve çalışan iş sınırının (`KENTOS_JOB_LIMIT`, varsayılan 50) içinde bir kullanıcının payı:
  `KENTOS_JOB_USER_LIMIT` (1–10 000, varsayılan 10; `JobLimits.user_active`).
- Sayma tek SQL işlevinde, kurumun kilidi altında: `kentos.job_room(kurum, kullanıcı, kurum sınırı, kullanıcı sınırı)`
  `'user'`, `'tenant'` ya da boş verir. Kilit `kentos.job_slot`'unkiyle aynıdır, eşzamanlı istekler sınırı aşamaz.
- İkisi de doluysa kullanıcının payı söylenir: yalnız onun kendi işi bitince yer açılır.
- Aşan istek `quota_exceeded` (429) alır, iki sözle:
  - kullanıcının: “Sırada ve çalışan işleriniz sınıra (10) ulaştı; biri bitince ya da durdurulunca yeniden isteyin.
    Kurumun diğer kullanıcılarının işleri sürer.”;
  - kurumun: ADR 0547'deki söz.
- Aynı istek kimliğiyle yeniden istek, pay dolu olsa da aynı işi verir (ADR 0542'nin tek iş kuralı önce gelir).

### §2 Sonuç önbelleği

**Ne önbelleğe girer.** Sonucu yalnız okuduğuna bağlı olan ve dosya veren işler:

- `project.export`'un `.kcad`'i;
- `project.atlas`'ın PDF'i;
- `project.processing`'in `mode: sink` GeoPackage'ı.

Kontrol noktası, içe aktarma ve projeye yazan araç girmez: projeyi değiştirirler.

**Anahtar** (`kentos_application::job_cache`), şu kaydın kanonik JSON'unun (ADR 0253 §2) SHA-256'sıdır:

- `format: kentos.job-cache/1`: kural değişirse bütün anahtarlar değişir;
- işin türü;
- girdi, sonucu değiştirmeyen alanları çıkarılmış olarak: `expectedRevision` (yalnız reddeder), `cpuSeconds` ve
  `memoryMb` (bütçe), `label` (ad);
- projenin veri revizyonu ve olay imleci;
- erişim damgası (ADR 0550): formların kestiği alanlar, kullanıcının sınıfı;
- yazılım: sunucunun sürümü ve programının boyu ile değişme zamanı (`job_cache::software`). İşçinin çalıştırdığı işte
  bir de çocuk programın (`kentos-jobrun`) boyu ve değişme zamanı eklenir (`Processor::software`). Yeniden derlenen ya da
  güncellenen sunucu ya da araç programı eski yapının dosyasını almaz;
- atlas ve araçta kullanıcı ve gün (UTC). Bu ikisi `@` değerlerini okuyabilir (ADR 0214: kullanıcı, tarih): bir
  kullanıcının PDF'i başkasına gitmez, ertesi gün yeniden yapılır. Dışa aktarmanın dosyası aynı erişimdeki herkes için
  aynıdır, kullanıcı ve gün anahtara girmez.

Planın saydıkları buradadır:

- kaynak revizyonu: revizyon ve imleç. Projedeki her yazma olay günlüğüne bir olay yazar: nesneler, ayarlar, katmanlar,
  stiller, ad. İmleç `greatest(son olay, budanan sınır)`'dır, olay budansa da geri gitmez;
- sorgu, sistem, stil ve zaman: girdinin kendisi (atlasın `crs`, `theme`, `time`'ı; aracın değerleri);
- yetki kapsamı: erişim damgası.

**Alma.** İşçi işi talep edince, bir şey okumadan önce `jobs::reuse`:

- iş `fresh` istenmişse dosyayı yeniden yapar;
- girdinin `expectedRevision`'ı projenin revizyonu değilse almaz: iş kendi yolunda `revision_conflict` ile reddedilir;
- iş için gereken haklar (`project.jobs.run`, `project.download`) sorulur;
- revizyon, imleç ve damga tek bir okuma görüntüsünden alınır;
- projenin bu anahtarlı en yeni başarılı işi aranır. Nesnesi depoda satırının dediği boyda değilse (saklama süresi
  silmiş) dosya yeniden yapılır;
- nesne yeni işin kendi anahtarına bağlanır (`Blobs::share`: sabit bağ, başka dosya sisteminde kopya). Sonuç olduğu gibi
  alınır, iş her iş gibi biter: bitirmede yetki ve damga yeniden sorulur (ADR 0550 §3). İşin `cachedFrom`'u aldığı işi
  söyler.

**Saklama.** Bir işin anahtarı, okuduğu görüntünün kendi revizyonu, imleci ve damgasıyla bitirmede yazılır (`cache_key`).
Aramadan önce bakılan değerlerle yazılmaz. Arada bir yazma olursa iş yeni durumun anahtarını alır; eski anahtarla yeni
dosya yan yana gelmez.

**Silme.** Her işin dosyası kendi anahtarıdır. Saklama süresi (ADR 0547) bir işi silince yalnız onun bağı gider, aynı
dosyayı alan diğer işler okunmaya devam eder. Önbelleği temizlemek kaynak veriye dokunmaz: kaynak proje satırlarıdır,
önbellek iş dosyalarıdır.

Saatlik temizlik, bir saattir değişmemiş ve hiçbir satırın tutmadığı iş nesnesini sahipsiz sayıp siler. Sabit bağ eski
nesnenin değişme zamanını taşır: bağlanan anahtar, satırı bitirmede yazılmadan önce temizliğe yakalanabilirdi. Bu yüzden
`Blobs::share` yeni anahtarın değişme zamanını şimdiye çeker. Aynı dosyayı paylaşan anahtarların hepsi tazelenir; içerik
değişmez, yalnız sahipsiz sayılmaları bir saat gecikir.

**İstemciler:**

- sözleşmenin `ServerJobCreate.fresh`'i ve `ServerJob.cachedFrom`'u;
- Python `export_job`, `processing_job` ve `atlas_job`'un `fresh=True`'su;
- MCP `jobs.start`'ın `fresh`'i;
- masaüstü önbelleği kullanır, satır “… · önceki işten alındı” der (`server_jobs::result_text`).

### §3 Dolu disk

- `blobs::is_full`: `StorageFull`, `QuotaExceeded`, `FileTooLarge` ya da errno 28, 122, 27.
- Dolu depo `AppError::Storage` olarak gelir. Kodu `storage_full` (507 Insufficient Storage), yeniden denemesi
  `(true, 300 sn)`: yer yönetici açınca açılır, birkaç saniyede değil. Sözü: “Sunucunun dosya deposunda yer kalmadı;
  dosya saklanmadı ve yarım kalan parçası silindi. Yönetici yer açınca aynı isteği yeniden gönderin.”
- Hiçbir yazıcı yarım bir şey bırakmaz:
  - **İşin dosyası:** silinir, iş `storage_full` ile düşer, sözü neyin kaç bayt olduğunu söyler.
  - **Kontrol noktası:** reddedilir, ne nesne ne satır kalır.
  - **Parçalı yüklemenin parçası:** `Blobs::append` dosyayı eski boyuna geri keser. Yükleme kaldığı boydan sürer;
    parça ya bütün alınır ya hiç.
  - **İş girdisi:** reddedilir, hiçbir şey hazırlanmış sayılmaz.
  - **Yedek:** yarım klasörünü siler, önceki yedekler sağlam kalır (§4).
- `BlobWriter::finish` diske yazamayınca dosyayı siler.
- Tatbikatın bulduğu kusur: tokio'nun dosyası yazmayı arka plandaki bir iş parçacığında yapar ve o yazmanın hatasını
  yalnız sonraki yazmaya ya da `flush`'a söyler, `sync_data` onu bekleyip düşürür. `Blobs::append` yalnız
  `sync_data` çağırıyordu: dolu diskte parçanın sığan başı (253 342 baytın 61 440'ı) yazılıyor ve parça alınmış
  sayılıyordu. Şimdi `sync_data`'dan önce `flush` çağrılır, hata döner ve dosya eski boyuna kesilir.
  `Blobs::share`'in kopyası da (başka dosya sistemi) yarım kalırsa silinir.
- Sıradan takımda da sınanır: Linux'un `/dev/full`'u her yazmaya ENOSPC der. `blobs`'un birim testi nesnenin
  anahtarını ona bağlar; `append` ve `BlobWriter` yerin kalmadığını söylemelidir. `flush` olmadan test düşer
  (hata EINVAL olur, ENOSPC kaybolur).
- Tatbikat `scripts/ops/disk_full_drill.sh`'tır: `kentos-application`'ın `disk_full` testini, nesne deposu kendi küçük
  tmpfs'inde olacak biçimde çalıştırır; tmpfs özel bir bağlama ad alanında bağlanır (`unshare -rm`), kök yetkisi
  gerekmez. Disk gerçekten dolar (ENOSPC), sahte bir hata yoktur. Test diski doldurur, her yazıcıyı dener, sonra yer
  açar ve aynı isteklerin geçtiğini görür.

### §4 Düzenli yedek

- `kentosd backup --into KÖK [--keep 7]`: KÖK altında, başladığı anın adıyla (`2026-10-29T12-04-05Z`, aynı saniyede
  ikincisi `-2`) yeni bir yedek klasörü (`backup::take_into`). İçeriği ADR 0650'nin yedeğidir: döküm, nesneler,
  `yedek.json`.
- **Değişmeyen nesneler bağlanır:** KÖK'teki en yeni tam yedekte aynı anahtar, boy ve SHA-256'yla duran nesne oradan sabit
  bağla alınır. Depodaki nesneler hiç değişmez (ADR 0031). Bağlanan dosya da okunup SHA-256'sı denetlenir; tutmazsa ya da
  bağ kurulamazsa (başka dosya sistemi) kopyalanır. Yedi yedek, değişmeyen nesneleri bir kez tutar.
- **Döndürme** (`backup::rotation`): `keep`'ten fazla tam yedek varsa en eskileri silinir. En yeni tamdan eski yarım
  klasörler (başarısız denemeler) silinir. En yeni tamdan yeni yarım klasöre dokunulmaz: sürüyor olabilir. Yedek adı
  taşımayan klasörlere hiç dokunulmaz.
- Başarısız yedek kendi yarım klasörünü siler, başka bir şeyi silmez: diski doldurduğu yer geri gelir.
- `kentosd backup list --into KÖK` yedekleri ad sırasıyla söyler: tam ya da yarım, ne zaman, göç, kaç nesne, dökümün
  boyu.
- **Zamanlama** sunucunun içinde değildir. Yedek veritabanının sahibinin adresini ister, `kentosd serve` o adresi
  taşımamalıdır. Yedeği sistemin zamanlayıcısı alır:

  ```ini
  # /etc/systemd/system/kentos-yedek.service
  [Service]
  Type=oneshot
  User=kentos
  EnvironmentFile=/etc/kentos/yedek.env   # KENTOS_DATABASE_OWNER_URL, KENTOS_BLOB_DIR
  ExecStart=/usr/local/bin/kentosd backup --into /srv/yedek/kentos --keep 7

  # /etc/systemd/system/kentos-yedek.timer
  [Timer]
  OnCalendar=*-*-* 03:30
  Persistent=true
  [Install]
  WantedBy=timers.target
  ```

- Bağlı nesneler aynı dosyadır. Yedekler salt okunur tutulur, başka diske `rsync -aH` ile bağları korunarak taşınır.
  Geri yüklemeden önce `kentosd backup check --dir KLASÖR` yedeği denetler (ADR 0650 §2).

## Doğrulama

- **Bağımsız başvuru:** `scripts/fixtures/job_cache_cases.py`, KentOS kodu olmadan. Anahtarın kaydı ve SHA-256'sı
  `protocol_cases.py`'nin kanonik JSON'uyla yazılır. Döndürme ve klasör adları ADR'nin kuralıyla yazılır.
  `fixtures/server-jobs/v1/cache-rotation.json`'da 53 durum var:
  - 29 anahtar: aynı anahtarı verenler ve vermeyenler, önbelleğe girmeyen dört tür, yazılımın iki yeniden derlenmesi ve
    sürümü;
  - 13 döndürme;
  - 11 ad.

  `job_cache_cases.rs` sunucunun hesabını bunlarla karşılaştırır.
- **Geçici PostGIS'te:**
  - `server_job_cache.rs`:
    - kullanıcının payı: payı dolunca kendi sözü, kurum dolunca kurumunki, ikisi doluyken kullanıcınınki, yeniden
      istek, durdurma yer açar;
    - önbellek: aynı durumda dışa aktarma alınır (aynı baytlar, kendi anahtarı), aynı erişimdeki başka kullanıcı da
      alır, `fresh` yeniden yapar, ilk işin nesnesi silinince diğerleri okunur ve sonraki iş onlardan alır, yazma
      yeniden yaptırır, `expectedRevision`'ı tutmayan iş reddedilir, form tasarlayan sahibin erişimi başka olduğundan
      almaz;
  - `processing_jobs.rs`: aynı atlas ikinci kez işlemciye gitmeden alınır, `fresh` iş işlemcinin reddine ulaşır;
  - kentos-application'ın bütün takımı geçti (işler, yedek, erişim, işlem işleri);
  - `blobs`'un birim testleri: iki saat önce yazılmış nesneye verilen yeni anahtar temizliğin “duran” listesine girmez;
    `/dev/full`'a bağlanan nesne `append`'de ve `BlobWriter`'da yerin kalmadığını söyler.
- **Dolu disk tatbikatı** (`scripts/ops/disk_full_drill.sh`, 24 MB tmpfs; ilk koşu §3'teki `append` kusurunu buldu):
  - dışa aktarma `storage_full` ile düşer, `jobs/` altında yeni nesne yoktur;
  - kontrol noktası reddedilir;
  - yüklemenin parçası reddedilir ve yükleme `0` bayttan sürer;
  - iş girdisi reddedilir;
  - yedek yarım klasörünü siler, ilk yedek `check_folder` ile sağlamdır;
  - yer açılınca aynı istekler geçer, ikinci yedek nesneleri ilkinden bağlar.
- **İstemciler:** `kentos_cloud::server_jobs` (satırın notu, dolu diskin sözü), Python ve MCP'nin `fresh`'i, kentos-api'nin
  ayar testleri (`KENTOS_JOB_USER_LIMIT`) ve gerçek çocuk süreçle `job_run_db`.
- **Masaüstü:** Sunucu işleri sekmesinde önbellekten alınan dışa aktarma ve atlas, dolu diskte düşen iş; koyu ve açık
  temada, 1440×900 ve 1100×650'de (`catalog_jobs_tests::screens::the_tab_with_taken_and_full_disk_jobs`,
  `.run/shots/isler-onbellek-dolu-disk-*`).

## Ölçümler

Release, bu makinede, geçici PostGIS'te. Proje 50 000 noktalı, dört kontrol noktası var. Süre, işin istenmesinden bitip
okunmasına kadardır.

```sh
KENTOS_TEST_DB=required cargo test --release -p kentos-application --test server_job_cache timing -- --ignored --nocapture
```

| Dışa aktarma (4 211 742 bayt) | Süre |
|---|---|
| yapıldı | 280 ms |
| önceki işten alındı | 27 ms |
| `fresh`: yeniden yapıldı | 264 ms |

Alınan işin süresi büyük ölçüde iş satırının yazılması ve okunmasıdır. Projenin okunmasına ve dosyanın yazılmasına hiç
girmez; proje büyüdükçe fark büyür.

| Yedek (7 nesne, 29 482 194 bayt; döküm 4 523 390 bayt) | Süre | Nesneler |
|---|---|---|
| ilk | 511 ms | 7 kopya |
| sonraki | 503 ms | 7 bağ |

Bağın kazancı süre değil, disk yeridir. Bağlanan nesne de okunup SHA-256'sı denetlenir, süreyi `pg_dump` ve bu okuma
belirler. Yedi günlük döndürmede değişmeyen nesneler diskte bir kez yer kaplar; her yedek yalnız dökümünü ve yeni
nesnelerini ekler.
