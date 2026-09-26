# ADR 0034: Proje geçmişi: kontrol noktaları ve yeni proje olarak geri yükleme

- **Durum:** kabul edildi (2026-09-26). Yön TODOS.md `SYNC-11` (adlandırılmış kontrol noktası, karşılaştırma, yeni proje olarak geri yükleme) ve `CLOUD-07`'den (sürüm geçmişi ile kontrol noktası tek modelde; aç, indir, yeni proje olarak geri yükle) gelir. Birinci adım kontrol noktası oluşturma, listeleme, indirme ve silmedir; ikinci adım yeni proje olarak geri yüklemedir (aşağıda, aynı gün).
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** CLAUDE.md §13, §16, §21; TODOS.md §10, §12; ADR 0015 (erişim), 0028 (katalog), 0031 (dosya projeleri), 0033 (tek anlık görüntü)

## Bağlam

- Veritabanı projesinin geçmişi komut günlüğü ve olaylardır. Olaylar bir süre sonra budanır; projenin eski bir hâline dönülecek bir nokta yoktu.
- Dosya projesinin her revizyonu zaten değişmez bir `.kcad`'dir (ADR 0031). Revizyonların adı yoktu; “belediyeye teslim edilen” hangisi bilinmiyordu.
- ADR 0033 veritabanı projesinin tek anlık görüntüsünü kilitsiz üretir. Kontrol noktası onu saklayabilir.

## Karar

### Model

- **Veritabanı projesinin kontrol noktası**, o anın görüntüsüdür (ADR 0033). Nesne deposunda `checkpoints/{kurum}/{proje}/{kontrol noktası}-{sha256}` olarak saklanır. Satırı ad, not, gösterdiği veri revizyonu, boyut, SHA-256 ve nesne sayısını tutar (`kind: snapshot`).
- **Dosya projesinin kontrol noktası** bir revizyona ad verir (`kind: revision`); verilmezse en yeni revizyona. Satır revizyonun numarasını, boyutunu, özetini ve nesne sayısını tekrarlar; bir şey kopyalanmaz. Revizyonlar yalnız projeyle birlikte gider.
- Kontrol noktası değişmez; yalnız silinebilir. Silinen veritabanı kontrol noktasının nesnesi de gider; dosya projesinde adlandırdığı revizyon kalır.
- Ad 1–120 karakterdir, tek satırdır. Not en çok 2000 karakterdir; satır sonu ve sekme içerebilir. İkisi de kırpılarak saklanır; aynı ad birden fazla kez kullanılabilir.
- Veritabanı: `kentos.project_checkpoint`, migration 0008. Projenin kendi satırlarıdır, satır düzeyi güvenlik nesnelerinkiyle aynıdır.

### Komutlar ve yollar

- `project.checkpoint.create` v1 `{ name, note?, fileRevision? }`. `feature.write` ister; arşivdeki ya da çöpteki projede alınmaz. `fileRevision` yalnız dosya projesinde geçerlidir.
- `project.checkpoint.delete` v1 `{ checkpointId }`. Kontrol noktasını oluşturan kişi (`feature.write` ile) ya da `project.edit` sahibi siler.
- İkisi de `CheckpointChange` döner, denetim kaydına girer ve projenin açık bağlantılarına `project.checkpoint` olayı gider. Aynı idempotency anahtarıyla gelen yeniden deneme saklı yanıtı alır.
- `GET …/checkpoints`: en yenisi önce; `project.history` ister.
- `GET …/checkpoints/{id}`: dosyası. `project.history` ve `project.download` ister; kurumun `viewer_download` politikası uygulanır. Başlıklar: `ETag` (SHA-256), `X-Kentos-Revision`, dosya adı “proje - kontrol noktası.kcad”.
- Erişilemeyen projede iki yol da var olmayan proje gibi 404'tür (ADR 0015).

### Depo ile veritabanı arasında

- Veritabanı projesinin görüntüsü proje kilidi alınmadan çekilir (ADR 0033); o sırada kaydeden beklemez.
- Nesne önce depoya yazılır. Ardından proje kilidi altında erişim yeniden sorulur (`SYNC-16`) ve satır, denetim kaydı, olay ve saklı yanıt tek işlemde yazılır.
- Ara durumlar:
  - işlemden önce hata: nesne silinir;
  - yeniden deneme saklı yanıtı aldıysa: bu denemenin nesnesi kimsenin değildir, silinir;
  - işlemin sonucu bilinmiyorsa nesne kalır. Satırı yazılmadıysa saatlik temizlik, bir saat bekledikten sonra onu siler (`kentos.existing_checkpoints`, `files::SETTLED`).
- Kalıcı silinen projenin kontrol noktası nesneleri, revizyonlarıyla birlikte temizlikte gider.

## Bu adımda olmayanlar

- ~~Yeni proje olarak geri yükleme~~: ikinci adımda geldi, aşağıya bakın.
- Revizyonlar ya da kontrol noktaları arasında karşılaştırma; dal ve senaryo.
- Kontrol noktalarının otomatik alınması (sıklık politikası) ve kota (`CLOUD-26`).
- Web ve masaüstü arayüzü: geçmiş paneli, oluşturma ve indirme. Web tarafı web ajanının dosya projesi işine eklenecek.

## Doğrulama (26 Eylül 2026, Linux)

- `crates/server/application/tests/project_checkpoints.rs`, gerçek veritabanı:
  - veritabanı projesi: ad ve not kırpılır, satır sonlu not korunur. Kontrol noktası alındığı anı tutar: sonra eklenen nesne dosyasında yoktur. Yeniden deneme saklı yanıtı alır ve depoda tek nesne kalır;
  - görüntüleyici listeler ve politika izin verdikçe indirir; kontrol noktası oluşturamaz; politika kapanınca indirme 403;
  - başka bir düzenleyici, kendisinin olmayanı silemez; kendi oluşturduğunu siler. Oluşturan siler ve nesne depodan gider;
  - ad ve not sınırları alan yoluyla reddedilir; veritabanı projesinde `fileRevision` reddedilir; arşivdeki projede kontrol noktası alınmaz;
  - dosya projesi: revizyon yokken reddedilir; verilmezse en yeni revizyon, istenirse eskisi adlandırılır. Depoya bir şey eklenmez; olmayan revizyon reddedilir. İndirme revizyonun baytlarını verir; silmek revizyonu bırakır;
  - temizlik: kaydı olmayan kontrol noktası nesnesi bekleme süresi içinde kalır, sonra gider; kaydı olan kalır; kalıcı silinen projenin kontrol noktaları gider.
- HTTP (`apps/api/src/http/checkpoints_tests.rs`): komut yolundan oluşturma, liste, başlıklarıyla indirme, başka kurumdan birine iki yolda da aynı 404.
- Kasıtlı bozma: silmedeki “oluşturan ya da yönetici” denetimi kaldırılınca test düştü; geri alındı.

## İkinci adım: yeni proje olarak geri yükleme (26 Eylül)

- `project.checkpoint.restore` v1 `{ checkpointId? , fileRevision?, name?, tenantId? }`: `checkpointId` ile `fileRevision`'dan tam olarak biri verilir. Kaynak proje değişmez; geri yükleme her zaman yeni bir proje açar (`SYNC-11`: başkasının güncel işi ezilmez).
- Yeni proje bir kopya gibi açılır (ADR 0028): kendi kimliği vardır, çağıranındır; geçmiş, paylaşım, sık kullanılanlar ve arşiv durumu gelmez. Kaynağın açıklaması, türü ve etiketleri gelir. Adı verilmezse kaynağın adına noktanın adı eklenir: “Ada 101 (Teslim)”, revizyonda “Ada 101 (r3)”. Yanıt kopyanınkiyle aynıdır (`ProjectDuplicated`); web onu kopya gibi açar.
- **Dosya projesinin noktası** (kontrol noktası ya da numarasıyla bir revizyon), 1. revizyonu o revizyon olan bir dosya projesi olur. Nesne depoda paylaşılır (sabit bağlantı ya da kopya, ADR 0031 eki); dosya çözülmez.
- **Veritabanı projesinin kontrol noktası**, dosyasından içe aktarılan bir veritabanı projesi olur:
  - dosyanın ayarları, katmanları, stilleri, orijini ve görünümü projenin olur;
  - her nesne kalıcı kimliğiyle ve 1. sürümüyle yazılır, veri revizyonu 1'dir (ADR 0026);
  - nesneler binerli toplu eklemeyle yazılır (`unnest`);
  - dosya önce SHA-256'sıyla karşılaştırılır. Çözme ve nesnelere çevirme kaynağın kilidi alınmadan, async iş parçacıklarının dışında yapılır; kilit yalnız satırlar yazılırken tutulur (kopyada olduğu gibi).
- **Yetki:** kaynakta `project.history` ve `project.download`, hedef çalışma alanında proje açma hakkı. Kaynak, kilit altında yeniden sorulur.
- Kaynağın denetim kaydı geri yüklemeyi, yeni projeninki kaynağını (`restoredFrom`: kurum, proje, kontrol noktası, revizyon) tutar. Aynı idempotency anahtarıyla gelen yeniden deneme aynı yeni projeyi alır; dosya noktasında bu denemenin paylaştığı nesne silinir.
- Sınandı (`project_checkpoints.rs`):
  - veritabanı kontrol noktasından geri yüklenen proje, kontrol noktasının dosyasıyla nesne nesne aynıdır (kimlikler, ayarlar, katmanlar, stiller). Kontrol noktasından sonraki değişiklik gelmez, kaynak değişmez, geçmiş gelmez, yeniden deneme aynı projeyi verir;
  - dosya projesinde numarayla revizyon ve adlandırılmış kontrol noktası geri yüklenir; baytlar aynıdır;
  - iki nokta birden ya da hiçbiri reddedilir; veritabanı projesinde `fileRevision` reddedilir; proje açamayan görüntüleyici 403 alır.
  - HTTP: komut yolundan geri yükleme.
  - Kasıtlı bozma: içe aktarmada öznitelikler boş yazılınca nesne karşılaştırması düştü; geri alındı.
