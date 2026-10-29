# ADR 0744: Önbelleğin kazancı ölçümlerde ve zamanda geri dönüş tatbikatı

- **Durum:** kabul edildi (2026-10-29). `plan/gis.md` CBS-39'un altıncı dilimi. Kapsamı ben belirledim.
- **Kalan işler:**
  - [ADR 0739](0739-job-user-share-result-cache-full-disk-rotated-backups.md)'un kalanı “önbelleğin kazancının
    ölçümlerde görünmesi”;
  - [ADR 0650](0650-server-backup-and-restore-drill.md)'nin kalanı “zamanda geri dönüş (PITR, WAL arşivi) ve RPO/RTO'nun
    ölçülmesi” (`OPS-08`).
- **Web:** bir şey değişmez. **`.kcad`:** değişmez. **Sunucu:** göç 0037 (`kentos.job_cache_metrics`).
- **Bağımlılık:** yeni kitaplık yok. Tatbikat PostgreSQL'in kendi araçlarını (`pg_basebackup`, `restore_command`) ve
  docker'ı kullanır.

## Bağlam

ADR 0739 sunucu işlerine sonuç önbelleği getirdi: aynı durumdan istenen dışa aktarma, atlas ya da dosya yazan araç önceki
işin dosyasını alır. Operatör bunun ne kazandırdığını göremiyordu: `kentosd jobs` yalnız kuyruğun sayılarını yazıyordu.

ADR 0650 yedeği ve geri yüklemeyi tatbik etti, ama yedek bir anın kopyasıdır: iki yedek arasındaki iş kaybolur. Planın
`OPS-08`'i ve CBS-39 kaybın (RPO) ve geri dönüşün süresinin (RTO) ölçülmesini ister.

Üreticilerin yolu (29 Ekim 2026'da okundu):

- PostgreSQL 18 belgeleri, “Continuous Archiving and Point-in-Time Recovery”: `archive_mode` ve `archive_command` ile
  dolan her WAL parçası arşive kopyalanır; `pg_basebackup` ile alınan temel yedek ve arşiv, `restore_command` ve
  `recovery_target_time` ile istenen ana kadar oynatılır; `recovery.signal` kurtarmayı başlatır. `archive_timeout`, az
  yazan sunucuda bir parçanın en geç ne kadar sonra arşive gideceğini sınırlar.
  <https://www.postgresql.org/docs/18/continuous-archiving.html>

## Karar

### 1. Önbelleğin kazancı `kentosd jobs`'ta

Göç 0037'nin `kentos.job_cache_metrics()`'i son 24 saatte önbellek anahtarıyla biten işleri tür başına sayar: kurulan,
önceki işten alınan, ikisinin başlangıçtan bitişe ortalama süresi ve alınan baytlar. İşin içeriği ve kurum yazılmaz.
`kentosd jobs` her tür için bir satır yazar:

```text
önbellek	project.export	kurulan	alınan	kurulanın_ort_s	alınanın_ort_s	alınan_bayt	kazanılan_s
```

Kazanılan süre, alınan her işin kurulanların ortalamasıyla kendi süresi arasındaki farktır
(`CacheGain::saved_seconds`).

### 2. Zamanda geri dönüş tatbikatı

`scripts/ops/pitr_drill.sh [SATIR]` geçici bir docker PostgreSQL/PostGIS'inde (geliştiricinin veritabanına ve 5432'ye
dokunmaz):

1. WAL'ı bir birime arşivleyen sunucu açar (`archive_mode=on`, `archive_timeout=1`);
2. temel yedeği alır (`pg_basebackup -X none`: WAL arşivden gelir);
3. hedef andan (T) önce ve sonra ayrı satırlar yazar, son parçanın arşive düştüğünü bekler;
4. ikinci bir sunucuyu temel yedekten, arşivden ve `recovery_target_time = T` ile açar, kurtarma bitince satırları sayar.

T'den önceki bütün satırlar dönmeli, sonrakilerin hiçbiri dönmemeli; yoksa tatbikat başarısızdır. Kurtarmanın süresi
(RTO) ikinci sunucunun açılmasından yazılabilir olmasına kadardır; kaybın sınırı (RPO) `archive_timeout`'tur.

### 3. Nesne deposu ile birlikte

Veritabanı T'ye döner; nesne deposu dönmez. T'den sonra yazılan nesneler adsız kalır (ADR 0739'un saatlik temizliği
onları siler). T'den sonra silinen bir nesne (saklama süresi, kalıcı silme) T'deki bir satırın adladığı nesnedir ve eksiktir:
ADR 0739'un döndürülen yedeklerinden (`kentosd backup --into KÖK --keep 7`, değişmeyen nesneler bağlantıyla) T'den
önceki son yedeğin `nesneler/`'i depoya geri kopyalanır, ardından `kentosd backup check` çalışan sunucunun bütün
nesnelerini SHA-256'yla denetler. Denenmemiş geri dönüş geri dönüş değildir.

## Doğrulama

- `cargo test -p kentos-api the_jobs_report`: raporun önbellek satırı ve kazanılan süre.
- `KENTOS_TEST_DB=required cargo test -p kentos-application --test server_job_cache`: dört kurulan, üç alınan dışa
  aktarma ve alınanların baytları ölçümlerde.
- `scripts/ops/pitr_drill.sh`: aşağıdaki ölçümler.

## Ölçümler

Aynı makine (Intel i5-11300H, 15 GB, Ubuntu 26.04), `postgis/postgis:18-3.6`, docker birimi yerel diskte. Kurtarma süresinin
çoğu arşivdeki WAL'ın yeniden oynatılmasıdır: temel yedekten bu yana yazılanla büyür, temel yedeğin sıklığı RTO'yu sınırlar.

| Satır (T'nin her yanında) | Dönen önce | Dönen sonra | Kurtarma (RTO) | Kaybın sınırı (RPO) |
|---:|---:|---:|---:|---:|
| 2 000 | 2 000 | 0 | 2,50 sn | 1 sn |
| 20 000 | 20 000 | 0 | 2,36 sn | 1 sn |
| 1 000 000 | 1 000 000 | 0 | 14,10 sn | 1 sn |

## Sonuçlar

- Operatör önbelleğin işe yarayıp yaramadığını `kentosd jobs`'tan okur.
- WAL arşivi kurulmuş bir sunucu, son temel yedekten bu yana herhangi bir ana `archive_timeout` kadar kayıpla döner.

## Kalan

- Gerçek bir KentOS veritabanıyla (göçler, projeler, nesne deposu) uçtan uca PITR ve `backup check`'in tatbikatı;
  arşivin S3 uyumlu depoya gitmesi; temel yedeğin düzenli alınması.
- S3 uyumlu nesne deposu, işten önce boş yer payı (dosya sisteminin boş yerini sormak yeni bir bağımlılık ister).
