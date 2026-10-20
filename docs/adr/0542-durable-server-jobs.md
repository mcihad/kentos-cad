# ADR 0542: Dayanıklı sunucu işleri

- **Durum:** kabul edildi (2026-10-20). `plan/gis.md` CBS-39'un ilk dilimi. Kapsamı ben belirledim.
- **Bağlam belgesi:**
  - ADR 0031 (dosya projeleri ve nesne deposu);
  - 0033 (veritabanı projesinin `.kcad` görüntüsü);
  - 0015 (proje erişimi);
  - 0253 (komut protokolü: idempotency, uzun işler);
  - CLAUDE.md §18 (lease, heartbeat, fencing, idempotency, staged sonuç, commit anında yetki).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - başka iş türleri (sunucuda İşlemler aracı, büyük içe aktarma, atlas PDF'i);
  - işin CPU, bellek ve disk kotası (bugün yalnız süre ve deneme sınırı var);
  - masaüstünün bulut arayüzünde işler listesi;
  - önbellek anahtarının kaynak sürümü, sorgu, sistem, stil, zaman ve yetki kapsamıyla kurulması;
  - PostgreSQL, nesne deposu ve bağlı dosyaların tutarlı yedeği ve geri yükleme tatbikatı (G2);
  - işlerin ölçümleri (kuyruk uzunluğu, süre, düşen iş).

## Bağlam

Masaüstünde uzun iş kendi iş parçacığında, Durdur'la çalışıyordu. Pencere kapanınca iş de bitiyordu. Sunucuda kalıcı bir iş
yoktu: kullanıcı bir işi isteyip pencereyi kapatamıyor, sonra dönüp sonucunu alamıyordu. Plan şunları istiyor:
- işin kimliği ve değişmez girdisi;
- durum, ilerleme ve sonuç dosyası;
- işçi ölünce işin kaybolmaması, iki işçinin aynı işi bitirememesi;
- yeniden denemede aynı işin dönmesi;
- süre sınırı;
- sonucun önce bir kenara yazılması (staged);
- yetkinin ve kaynak sürümünün kesinleşme anında yeniden sorulması.

## Karar

### 1. İş satırı

`kentos.server_job` tablosu (göç 0029) projenin satırıdır; satır güvenliği denetim noktalarınınki gibi projenin kapsamı ve
okunabilirliğidir. Satır şunları taşır:
- tür (bugün yalnız `project.export`);
- girdi, olduğu gibi, ve SHA-256'sı; girdi bir daha değişmez;
- istendiği andaki veri revizyonu;
- isteyen kullanıcı ve onun istek kimliği;
- durum: `queued`, `running`, `succeeded`, `failed`, `cancelled`;
- deneme sayısı ve en çok deneme (3), süre sınırı (varsayılan 600 s, en çok 86 400 s);
- kira sahibi, kiranın bitişi ve çit (fence);
- ilerleme (0–1), başlangıç ve bitiş anı;
- sonuç, sonuç dosyasının anahtarı, boyu ve SHA-256'sı ya da hatanın kodu ve iletisi.

Aynı kullanıcı aynı projede aynı istek kimliğini yeniden gönderirse aynı işi alır; ikinci isteğin girdisine bakılmaz.

### 2. Kira ve çit

İşçi işi `kentos.claim_job` ile alır (security definer: işçi başka türlü hiçbir projenin satırını görmez). Alma:
- süresini aşan çalışan işi `timeout` ile düşürür;
- kirası biten çalışan işi kuyruğa geri koyar, deneme hakkı bittiyse `lease_lost` ile düşürür;
- en eski kuyruktaki işi `for update skip locked` ile alır; çiti bir artırır, denemeyi bir artırır.

İşçi çalışırken `kentos.beat_job` ile kirasını yeniler ve ilerlemesini yazar. Cevap `run`, `cancel` ya da `lost`'tur. Yalnız
güncel çiti tutan işçi yenileyebilir ve bitirebilir (`kentos.finish_job`). Kirası biten işçinin geç gelen bitirmesi hiçbir şeyi
değiştirmez.

### 3. Staged sonuç ve kesinleşme

`project.export` projenin `.kcad` görüntüsünü (ADR 0033) yazar:
1. `expectedRevision` verildiyse proje o revizyonda değilse iş `revision_conflict` ile düşer;
2. dosya nesne deposuna `jobs/{kurum}/{proje}/{iş}-{sha}` anahtarıyla yazılır;
3. iş, isteyen kullanıcı olarak, projenin satır güvenliği altında bitirilir; erişim (`jobs.run` ve `project.download`) orada
   yeniden sorulur;
4. bitirme geçmezse (yetki alınmış, kira kaybedilmiş, iş iptal edilmiş) dosya silinir.

Silinemeyen dosyayı deponun temizliği bulur: `kentos.existing_job_artifacts` hiçbir işin tutmadığı `jobs/` dosyalarını söyler,
temizlik onları siler. Proje silinince işleri ve dosyaları da gider.

### 4. İptal

Kuyruktaki iş hemen iptal olur. Çalışan iş işaretlenir ve işçinin bir sonraki yoklamasında iptal olur. İşi isteyen kullanıcı ya da
projeyi düzenleyebilen (`project.edit`) iptal edebilir; ötekiler `forbidden` alır.

### 5. Yollar ve işçiler

- `POST /v1/tenants/{t}/projects/{p}/jobs`: işi iste (201);
- `GET …/jobs`: projenin işleri, yeniden eskiye, en çok 100;
- `GET …/jobs/{j}`, `POST …/jobs/{j}/cancel`;
- `GET …/jobs/{j}/artifact`: başarılı dışa aktarmanın dosyası, SHA-256'sı ve revizyonu başlıklarda.

Erişilemeyen proje var olmayan gibi 404'tür. `kentosd serve` API'nin yanında `KENTOS_WORKERS` kadar işçi açar (0–16,
varsayılan 1); `kentosd worker` yalnız işçidir. İşçinin kirası 60 s, boştaki bekleyişi 2 s'dir. İşçi günlüğe işin kimliğini
ve durumunu yazar; girdiyi, veriyi ya da parolayı yazmaz.

### 6. Sözleşme ve Python

Türler `kentos_contracts::server_jobs`'tadır (`ServerJob`, `ServerJobCreate`, `ProjectExportInput`, `ServerJobState`, …).
Python `ProjectRef` `export_job`, `jobs`, `job`, `wait_job`, `cancel_job` ve `job_artifact` yöntemlerini alır.

## Doğrulama

- `KENTOS_TEST_DB=required cargo test -p kentos-application --test server_jobs` (geçici PostGIS'te):
  - aynı istek aynı iş; bilinmeyen tür ve okunamayan girdi reddedilir;
  - işçinin dosyası projenin görüntüsüyle aynı SHA-256'da;
  - kirayı alıp ölen işçi: sonraki işçi yeni çitle bitirir; ölünün yoklaması `lost`, bitirmesi hiçbir şey değiştirmez;
  - deneme hakkı biter (`lease_lost`);
  - kuyruktaki ve çalışan işin iptali; düzenleyici sahibinin işini iptal edemez;
  - süre aşımı (`timeout`); eski revizyon (`revision_conflict`, dosya yok);
  - iş istendikten sonra yetkisi alınan kullanıcı: iş `forbidden` ile düşer, dosya kalmaz;
  - hiçbir işin tutmadığı dosyayı temizlik siler, tutulan kalır.
- `KENTOS_TEST_DB=required cargo test -p kentos-api jobs_tests`: yollar uçtan uca, başka kurumdan 404.
- `python/tests/test_connection.py`: Python yöntemleri bir yerine geçen sunucuyla.

Disk dolması, yarım yükleme, kaynak değişimi ve geri yükleme tatbikatı G2'ye kalır (plan/gis.md CBS-39).
