# ADR 0650: Sunucunun yedeği ve geri yükleme tatbikatı

- **Durum:** kabul edildi (2026-10-26). `plan/gis.md` CBS-39'un dördüncü dilimi. Kapsamı ben belirledim.
- **Kalan işler:** CBS-39'un “Backup; PostgreSQL, nesne deposu ve bağlı asset sürümlerini tutarlı restore edebilsin”i ve
  “restore tatbikatı revision/kimlik/ek/form ilişkilerini korumalı; backup alındı bildirimi restore kanıtı yerine geçmemeli”
  ölçütü; TODOS.md `OPS-07`, `OPS-08`'in ilk parçası.
- **Bağlam belgesi:**
  - ADR 0031 (dosya projesinin nesne deposu: nesneler değişmez, satır yazılmadan önce yazılır);
  - ADR 0034 (kontrol noktaları), 0542 ve 0561 (sunucu işlerinin dosyaları ve hazırlanan girdileri);
  - ADR 0004 (sahip rol ve sunucu rolü).
- **Web:** bir şey değişmez.
- **`.kcad`:** şema değişmez.
- **Bağımlılık:** yeni kitaplık yok. `pg_dump` ve `pg_restore` PostgreSQL'in kendi araçlarıdır, sunucunun makinesinde bulunur
  (`KENTOS_PG_DUMP` başka yolu söyler).
- **Sonraya kalan:**
  - zamanda geri dönüş (PITR, WAL arşivi) ve RPO/RTO'nun ölçülmesi (`OPS-08`);
  - S3 uyumlu nesne deposu (bugün depo bir klasördür);
  - yedeğin kendiliğinden ve düzenli alınması, saklama süresi, şifreleme;
  - önbellek anahtarı (CBS-39'un öbür kalanı), kullanıcı başına kota, disk dolması senaryosu.

## Bağlam

Sunucunun verisi iki yerdedir: PostgreSQL (projeler, nesneler, izinler, revizyonların ve kontrol noktalarının satırları) ve nesne
deposu (dosya projelerinin revizyonları, veritabanı projelerinin kontrol noktası görüntüleri, işlerin dosyaları ve hazırlanan
girdileri). Bir satır depodaki bir nesneyi anahtarı, boyu ve SHA-256'sıyla adlar. İkisini ayrı ayrı yedeklemek tutarsız bir çift
verebilir: veritabanı yedeğinden sonra silinen bir projenin nesnesi depo yedeğinde olmaz, yedekteki satır olmayan nesneyi adlar.

Bugüne dek yedeğin nasıl alınacağı ve geri yüklenmiş sunucunun sağlam olduğunun nasıl anlaşılacağı yazılı değildi; geri yükleme hiç
denenmemişti.

## Karar

### §1 Yedek: `kentosd backup --dir D`

Sahip rolüyle, boş ya da olmayan bir klasöre:

1. Salt okunur, `repeatable read` bir işlem açılır; `pg_export_snapshot()` anı sabitler.
2. O anda satırların adladığı nesneler okunur: dosya revizyonları, kontrol noktası görüntüleri, başarıyla biten işlerin dosyaları,
   alınmış iş girdileri. Henüz kesinleşmemiş yüklemeler geçicidir ve yedeğe girmez.
3. `pg_dump --format=custom --snapshot=…` aynı anı `veritabani.dump`'a yazar. Parola komut satırına değil, ortama verilir.
4. Her nesne `nesneler/<anahtar>`'a kopyalanırken SHA-256'sı ve boyu hesaplanır, satırınkiyle karşılaştırılır. Tutmazsa ya da
   nesne yoksa yedek başarısızdır ve neden söylenir (anda silinmiş olabilir: yeniden alınır).
5. İşlem kapanır; `yedek.json` (`kentos.backup/1`) yazılır: an, veritabanı, son göç, dökümün boyu ve SHA-256'sı, nesneler.

Nesneler bir kez yazılıp değişmediği için, anın satırlarının adladığı her nesne o an depodadır; yedek veritabanıyla tutarlıdır.

### §2 Denetim: `kentosd backup check [--dir D]`

- `--dir` ile: yedeğin klasörü: döküm kaydındaki mi, her nesne kopyalandığı gibi mi.
- `--dir`'siz: çalışan (ya da geri yüklenmiş) sunucu: her satırın adladığı nesne depoda, boyu ve SHA-256'sıyla mı.

Cevap: sağlam nesne sayısı, eksikler, farklı olanlar, depoda olup hiçbir satırın adlamadıkları (silinmez, yalnız söylenir). Eksik ya
da farklı nesne ya da farklı döküm varsa komut sıfırdan farklı kodla biter.

### §3 Geri yükleme

1. `kentosd db-setup --database yeni` (roller, veritabanı, eklentiler);
2. yönetici olarak `pg_restore --exit-on-error --dbname …/yeni D/veritabani.dump`;
3. `D/nesneler`'in içeriği yeni sunucunun `KENTOS_BLOB_DIR`'ine;
4. yeni yapı daha yeni göç taşıyorsa `kentosd migrate`;
5. `kentosd backup check` sağlam demeden sunucu açılmaz.

## Doğrulama

Geçici PostGIS'te tatbikat (`KENTOS_TEST_DB=required cargo test -p kentos-application --test backup`):

- veritabanı projesi (nesneleri ve kontrol noktası), iki revizyonlu ve kontrol noktalı dosya projesi, hazırlanmış iş girdisi;
- yedek alınır, klasörü sağlamdır, dolu klasöre ikinci yedek reddedilir;
- döküm yeni bir veritabanına `pg_restore` ile, nesneler yeni depoya; geri yüklenmiş sunucunun denetimi sağlamdır;
- sekiz tablonun satır sayıları aynı; veritabanı projesinin nesneleri sunucu rolüyle aynı okunur (kimlikler ve sürümler dahil);
  dosya projesinin iki revizyonu aynı baytlarla iner;
- bozulan bir nesne “farklı”, silinen “eksik”, satırsız bir dosya “adlanmamış” olarak söylenir; değişen döküm yedeğin denetiminde
  bulunur.

Yedeğin klasörü KentOS kodu olmadan da denetlenir (`scripts/ops/backup_check.py`): `yedek.json`'un biçimi, dökümün boyu ve
SHA-256'sı `hashlib`'le, dökümün içindekiler PostgreSQL'in kendi `pg_restore --list`'iyle (dört tablonun yapısı ve verisi), her
nesne anahtarının kuralıyla ve SHA-256'sıyla, adlanmamış dosya. Tatbikatın yedeği `KENTOS_BACKUP_OUT` ile bırakılır ve betikten
geçer.

Adresin parolası komut satırına gitmez: `kentos_postgres::env::split_password` (yüzde kodlu parola dahil) birim testinde.

## Ölçümler

Release, bu makinede, geçici PostGIS'te (`KENTOS_TEST_DB=required cargo test --release -p kentos-application --test backup
backup_timing -- --ignored --nocapture`): veritabanı projesinde 100 000 nesne, depoda 8 MiB'lık 20 iş girdisi (160 MiB).

| İş | Süre |
|---|---|
| `kentosd backup`'ın işi (`take`: döküm 3,5 MB ve 20 nesnenin SHA-256'lı kopyası) | 0,58 s |
| yedeğin klasörünün denetimi | 0,11 s |
| `pg_restore` ile yeni veritabanına | 2,23 s |
| geri yüklenen sunucunun denetimi | 0,12 s |

Nesneler bir kez okunur: kopyalanırken SHA-256'ları hesaplanır. Denetim her nesneyi bütünüyle okur; depo büyüdükçe süresi
nesnelerin toplam boyuyla doğru orantılıdır.
