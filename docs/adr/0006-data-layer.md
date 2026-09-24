# ADR 0006: Veri katmanı: PostgreSQL + PostGIS, roller, satır güvenliği (Faz B)

- **Durum:** kabul edildi
- **Tarih:** 2026-09-24
- **Bağlam belgesi:** CLAUDE.md §13, §15, §16, §19 Faz B

## Bağlam

Faz B'nin ilk dikey dilimi: tek tenant'ta gerçek proje, izinli kayıt, iki editörde çakışma. Ana kalıcı depo PostgreSQL + PostGIS'tir (§13). Geliştirme sunucusu kullanıcının Docker'daki `postgis/postgis:18-3.6` kabıdır. Aynı sunucuda başka uygulamaların veritabanları da var (örneğin başka bir uygulamaya ait `kentos`); bunlara dokunulmaz.

## Karar

### Veritabanı ve roller

- **Veritabanı:** `kentos_cad`. Şema `kentos`; PostGIS ve pgcrypto `public` şemasında.
- **Roller:**
  - `kentos_cad_owner`: veritabanının ve şemanın sahibi. Migration'ları ve yönetim komutlarını (`kentosd tenant|user|member …`) çalıştırır.
  - `kentos_cad_app`: sunucunun bağlandığı rol. Sahip değildir; `NOBYPASSRLS`, `NOSUPERUSER`, `NOINHERIT`. Yalnızca ihtiyaç duyduğu tablolara ve komutlara yetkisi vardır.
- **Kurulum:** `kentosd db-setup` yönetici bağlantısıyla çalışır ve tekrar çalıştırılabilir.
  - Rolleri, veritabanını ve eklentileri (yalnızca süper kullanıcının kurabildiği postgis ile pgcrypto) açar.
  - Parolalar rastgeledir (iki v4 UUID, 244 bit) ve depoya girmeyen, yalnızca sahibinin okuyabildiği (`0600`) `.env.local` dosyasında durur.
  - Aynı adda ama başka sahipli bir veritabanı varsa durur ve başka ad ister.
- **Migration:** `kentosd migrate` (sqlx, sahip rolüyle). `kentosd serve`, şema kendi sürümünden eskiyse başlamaz.

### Satır güvenliği (RLS)

- **Kapsam:**
  - Tenant'a bağlı her tablo (`project`, `feature`, `command_log`, `audit_event`, `outbox_event`) yalnızca `tenant_id = kentos.current_tenant()` satırlarını gösterir ve yazdırır.
  - Kapsamı `Db::scoped` kurar: her işlemin başında `set_config('app.tenant_id' / 'app.user_id', …, true)`. Ayar işlem bitince kendiliğinden düşer; havuzdaki bağlantı bir isteğin kapsamını ötekine taşımaz.
  - Kapsamsız sorgu tenant verisi görmez.
- **Üyelik ve kurum satırları:** kişi kendi üyeliklerini ve üye olduğu kurumları her kapsamda görür (`/v1/me` için). Sunucu tenant kapsamını ancak `tenancy::access` üyeliği, koltuğu ve kurumun etkinliğini doğruladıktan sonra kurar.
- **Kimliksiz girişler:** parola denetimi ve OpenID hesabının bulunması kullanıcı bilinmeden yapılır. Bunlar yalnızca iki `SECURITY DEFINER` işlevle yapılır: `kentos.check_local_login`, `kentos.resolve_identity`.
  - `search_path` sabittir; işlevler başka bir şeye dokunmaz.
  - Sunucu rolü parola özetini hiç okuyamaz.
- **Sınanan davranışlar** (`crates/server/application/tests/identity.rs`):
  - başka tenant'ın projesi görünmez;
  - başka tenant adına satır yazılamaz;
  - sunucu rolü `local_credential` okuyamaz, tenant açamaz, rol değiştiremez, denetim kaydı silemez;
  - rolde `BYPASSRLS` yoktur.

### Tablolar (migration 0001)

- **Hesaplar:**
  - `app_user(issuer, subject)`: kimlik; e-posta kimlik değildir.
  - `local_credential`: pgcrypto bcrypt, ADR 0007.
  - `auth_session`: yalnızca token özeti.
  - `oidc_login`.
- **Kurumlar:** `tenant` (koltuk sınırı), `membership` (rol ve durum), `seat_allocation`. Üyelik koltuk değildir; koltuklar kurumun satırı kilitlenerek sınır içinde ayrılır.
- **`project`:** ad, SRID, ayarlar, katman ağacı, stiller (sözleşme JSON'u), `meta_version` ve `data_revision`.
- **`feature`:**
  - Birincil anahtar `(tenant_id, project_id, id)`. §15'teki örnekten farklı olarak `layer_id` anahtarda değildir: nesnenin katmanını değiştirmek bir güncellemedir, anahtar değişikliği değil.
  - `source_kind = 'geom'`: nokta, çizgi, yaysız çoklu çizgi ve alan. PostGIS geometrisi kaynaktır; ikili EWKB ile yazılıp okunur, bitler korunur.
  - `source_kind = 'cad'`: diğer bütün türler. `cad_definition` (sözleşmedeki geometri alanları) kaynaktır; `geom` Rust'ta üretilen doğrusal izdüşümdür. Yay kirişi en çok 1 mm sapar, tek parça en çok 4 096 kiriş alır (`geometry-core::tessellate`, `projection_version`). Yardımcı çizgi ve ışının sonlu geometrisi yoktur (`geom` boş).
  - `check (st_srid(geom) = srid)`.
- **Diğerleri:**
  - `command_log`: idempotency; istek özeti ve saklı yanıt.
  - `audit_event`.
  - `outbox_event`: değişiklikle aynı işlemde yazılır; `seq` olay imlecidir.

### Sayılar ve kimlikler

- **Kimlikler:**
  - Sunucu kimlikleri UUIDv7'dir.
  - Nesne kimliğini (UUID) istemci seçer; aynı komutun tekrarı nesneyi iki kez yaratamaz.
  - Tarayıcının numaralı nesne kimlikleri sunucuya gitmez.
- **Sayılar:** sürüm, revizyon ve imleç API'de ondalık metindir (§24.1).

## Sonuçlar

- **Bilinen sınırlar (Faz B):**
  - Veritabanı bağlantısı TLS'siz (yerel sunucu). Üretimde sqlx'in TLS özelliği açılır.
  - Proje başına katman tablosu yok; katman ağacı projenin JSON'udur. Katman politikası ve yayın geldiğinde (Faz C) tabloya ayrılır.
  - Tipli öznitelik şeması yok (`properties jsonb`, metin değerler).
  - PostGIS eğri türleri (CircularString …) denenmedi (§15 PoC); izdüşüm doğrusal.
- **Testler:** her test kendi `kentos_cad_test_<zaman>_<rastgele>` veritabanını açar ve kapatır. Çöken bir çalıştırmanın artığı bir saat sonra silinir. Veritabanı yoksa testler görünür bir uyarıyla atlanır; `KENTOS_TEST_DB=required` bu durumda testi başarısız sayar.

## 2026-09-24: Bulut projesini silme ve yeniden adlandırma (migration 0002)

- **Silme yumuşaktır:** `project.deleted_at` ve `deleted_by` (ikisi birlikte dolu ya da boş). Silinen proje listeden çıkar; bilgisi, nesneleri ve yazma komutları 410 (`project_deleted`) ile reddedilir. Nesneler, komut günlüğü, denetim ve olaylar kalır. Silinmeden önce kaydedilmiş bir komutun tekrarı yine günlükten yanıtlanır.
- **Kim siler:** `project.delete` yetkisi; yönetici (admin) ve sahip (owner). Proje yöneticisi projeyi açar, düzenler, yeniden adlandırır ama silemez: silme kurumdaki herkesi etkiler.
- **Tek işlemde:** proje satırı kilitlenir (sürmekte olan commit önce biter, sonrakiler silindiğini görür), işaret, `project.delete` denetim kaydı ve nesnesiz `project.deleted` olayı (`outbox_event`). Projeyi açık tutan editörler bu olayla gönderimi durdurur; gönderilmemiş değişiklikleri cihaz taslağında kalır. Olay günlüğü silinmiş projede de okunur: bağlantısı kopmuş bir editör de nedenini öğrenir. Aynı projeyi yeniden silmek bir şey değiştirmez (kaybolan yanıttan sonraki tekrar).
- **Sunucu rolü proje satırı silemez** (`delete` yetkisi yok); silme bir güncellemedir. Satır güvenliği kuralları değişmedi: tenant sınırı aynıdır, silinmişliği uygulama katmanı denetler.
- **Geri getirme** işletmecinindir (sahip rolü, komut satırı): `kentosd project deleted --tenant KISA` listeler, `kentosd project restore --tenant KISA --project KİMLİK` geri getirir; denetime aktörsüz `project.restore` yazılır.
- **Yeniden adlandırma** yeni bir yazma yolu değildir: `project.changes` içindeki proje bilgisi yamasıdır (`name`, `project.edit` yetkisi, `expectedVersions["@project"]`).
- **Geliştirme:** `sqlx::migrate!` migration dosyalarını derlerken gömer ve klasöre yeni eklenen dosyayı kendiliğinden görmez; `crates/server/postgres/build.rs` klasör değişince yeniden derletir (yoksa test veritabanları yeni migration olmadan kurulur).
- **Sınırlar:** belirli bir süre sonra kalıcı silme ve arayüzden geri getirme yok. Geri getirilen projeyi silinmiş hâlde açık tutan editör onu listeden yeniden açar; cihaz taslağı o zaman geri gelir.

## 2026-09-24: Olay günlüğünün budanması (migration 0003)

- **Saklama süresi:** `outbox_event` satırları `KENTOS_EVENT_RETENTION_DAYS` gün (varsayılan 7; 1–3650) saklanır. `kentosd serve` açılıştan bir dakika sonra, sonra saatte bir eskileri 2 000'lik partilerle siler. Her parti tek ve kısa bir ifadedir, havuzdan bir bağlantı alır; istekler beklemez. Hata günlüğe yazılır, sonraki tur yeniden dener. Çok süreçte partiler `for update skip locked` ile birbirini beklemez.
- **Silen işlev:** `kentos.prune_outbox(p_keep interval, p_batch integer)`, `SECURITY DEFINER` (sahip rolüyle çalışır, `search_path` sabit): bütün tenant'ların eski olaylarını siler ve aynı ifadede her projenin **ufkunu** (`outbox_horizon.pruned_through`: silinen en yeni `seq`) yazar. Bir saatten kısa pencereyi reddeder: sunucu rolü canlı istemcilerin okuduğu günlüğü boşaltamaz. Sunucu rolü olay satırı silemez, ufku yalnızca okur (tenant kapsamlı RLS).
- **Neden ufuk:** `seq` bütün projelerde ortak bir sayaçtır; bir projenin kalan en eski olayına bakarak aradaki olayların silinip silinmediği anlaşılmaz. Ufuk sessiz bir projeyi (imleçten sonra hiç olay yok) olayları silinmiş projeden ayırır.
- **İstemciye etkisi:** imleç ufkun altındaysa (bazı olaylar gitmiş) ya da en yeni olayın ötesindeyse (geri yüklenmiş veritabanı) günlük oradan sürdürülemez: HTTP olay günlüğü 410 `resync_required`, WebSocket `resyncRequired` döner; istemci projeyi yeniden açar (cihaz taslağı geri gelir). Proje bilgisi (`event_cursor`) ve en yeni imleç ufkun altına inmez; bütün olayları silinmiş bir proje açılınca ufuktan sürer, yeniden açma döngüsü olmaz.
- **Yarış:** olay okuması önce, ufuk okuması sonra yapılır. Aradaki bir budama en çok gereksiz bir yeniden açma doğurur, eksik olayla sürdürmeyi değil.
- **Sınanan:** `crates/server/application/tests/retention.rs` (partiler, iki tenant, ufuk, 410, boş proje, RLS), `apps/api/src/http/ws_tests.rs` (gerçek soket: eski imleçte `resyncRequired`, ufuktan kalan olay, en yeni olayın ötesi), `identity.rs` (sunucu rolü olay silemez, ufku değiştiremez).
- **Sınırlar:** `command_log` (idempotency) ve `audit_event` budanmaz; idempotency penceresi ve denetim saklama süresi ayrı kararlardır.
