# ADR 0015: Proje sahipliği ve asgari erişim modeli

- **Durum:** kabul edildi (2026-09-25). Aşağıdaki dört politika sahibin yanıtıdır. Öbür ayrıntılar önerilir ve uygulama dilimlerinde (F4/F5) testle kesinleşir.
- **Tarih:** 2026-09-25
- **Bağlam belgesi:** CLAUDE.md §16, §21; TODOS.md `CLOUD-01`, `CLOUD-09..14`, §12, §22 (F1 ve F4 kapıları); ADR 0006, 0007, 0012
- **Sahibin kararları (25 Eylül):**
  1. Kişisel projeler kullanıcıya otomatik açılan bir **kişisel alanda** durur.
  2. Kurum sahibi ve yöneticisi, kendisiyle paylaşılmamış kurum projelerine **kurum politikasıyla** erişir. Politikanın varsayılanı açıktır, kurum kapatabilir.
  3. Kurum projesini **proje sahibi ve kurum yöneticisi** silebilir. Silme yumuşaktır.
  4. (Komut şemaları için `schemars`: ADR 0013.)

## Bağlam

Bugünkü model kurum düzeyindedir (koddan doğrulandı, 25 Eylül):

- `kentos.membership.role` beş kurum rolünden birini taşır: `owner`, `admin`, `project_manager`, `editor`, `viewer`.
- `tenancy::allows` bir rolden yetkileri çıkarır: `project.read`, `project.create`, `project.edit`, `feature.write`, `project.delete`, `member.manage`.
- Her HTTP ucu önce `tenancy::access` ile üyeliği, koltuğu ve kurumun etkinliğini denetler, sonra kullanım durumu `Access::require` ile yetkiyi sorar. WebSocket aboneliği her olayda kurum erişimini yeniden sorar.
- Satır güvenliği yalnız kurum kimliğine bakar (`app.tenant_id`).

Bu yüzden kurumun her üyesi kurumun bütün projelerini görür. TODOS.md §12 bunu açıkça reddeder: “Kuruma üye olmak bütün projelere erişim vermek değildir.” Kişisel projeler için de yer yoktur: hesabı olan ama kurumu olmayan kişi buluta kaydedemez.

## Karar

### Çalışma alanları

- **Kurum alanı:** bugünkü `kentos.tenant`; türü `organization`.
- **Kişisel alan:** her kullanıcıya ilk gerektiğinde (ilk bulut kaydı ya da “Projelerim”) sunucunun açtığı, türü `personal` olan bir tenant.
  - Kullanıcı kurum kurmaz, adını seçmez, koltuk ayırmaz. Arayüzde kurum değil “Kişisel” görünür.
  - Tek üyesi sahibidir. Başkası üye olarak eklenemez; birlikte çalışma proje paylaşımıyla olur.
  - Gerekçe: bugünkü yalıtım (satır güvenliği, `command_log`, `outbox`, audit ve bunların `tenant_id` anahtarları) hiç değişmeden kişisel projelere de uygulanır. Kurumsuz proje ise tablo anahtarlarını ve satır güvenliğini yeniden yazmayı gerektirirdi.
- Bir kullanıcının tek kişisel alanı vardır (`tenant.owner_user_id` benzersiz). Kurumdan ayrılmak kişisel alanı etkilemez.

### Proje sahipliği

- Her projenin bir **sahibi** vardır: bir kullanıcı; varsayılan olarak oluşturan kişi.
- Kişisel alanda sahip, alanın sahibidir.
- Kurumda proje kurumundur; sahip, projeden sorumlu kişidir. Sahiplik açık bir komutla devredilir (`CLOUD-08`). Son yetkili sahip kaldırılamaz.
- Kurumdan ayrılan kişinin kurum projeleri silinmez. Sahiplik bir kurum yöneticisine geçirilir.

### Proje rolleri ve izinler

İzin adları sabittir ve komut ile yetenek şemasına girer (`CLOUD-10`, ADR 0013). Web, masaüstü, Python ve AI aynı adları görür. Bugün var olan adlar anlamlarıyla korunur, yalnız eksikler eklenir; istemcilerin bugünkü yetenek denetimleri geçerli kalır.

| İzin | Anlamı | Bugün |
|---|---|---|
| `project.read` | Projeyi listede görmek, açmak, nesneleri ve olayları okumak | var (kurum düzeyinde) |
| `feature.write` | Nesneleri değiştirmek | var |
| `project.edit` | Ad, ayarlar, katman ağacı, stiller | var |
| `project.delete` | Projeyi (yumuşak) silmek | var |
| `project.comment` | Yorum ve inceleme notu (`CLOUD-23`) | yeni |
| `project.download` | Dosya/revizyon indirmek ve dışa aktarmak (`CLOUD-20`: görüntüleme DRM değildir) | yeni |
| `project.history` | Revizyon geçmişini görmek, eski revizyonu açmak | yeni |
| `project.share` | Proje rollerini vermek ve geri almak | yeni |
| `project.transfer` | Sahipliği devretmek | yeni |
| `project.jobs.run` | Sunucu işi başlatmak (`JOB-*`) | yeni |

Kurum düzeyindeki `project.create` ve `member.manage` projeye bağlı değildir, kurum rolünden gelir.

| Proje rolü | İzinler |
|---|---|
| `viewer` | read, download, history |
| `commenter` | viewer + comment |
| `editor` | commenter + `feature.write`, jobs.run |
| `manager` | editor + `project.edit`, share |
| `owner` (sahip) | manager + delete, transfer |

- **Etkin rol:** açık proje yetkisi (grant), kurum politikası ve sahiplikten gelen rollerin en yükseğidir.
  - İlk sürümde açık reddetme (deny) yoktur, grup yetkisi sonraki dilimdedir.
  - Kişiye verilen yetki bitiş zamanı taşıyabilir.
- **Kurum politikası (tenant ayarı):**
  - `admins_access_all_projects`: kurum sahibi ve yöneticisi paylaşılmamış kurum projelerinde `manager` + `project.delete` haklarıyla çalışır. **Varsayılan açık** (sahibin kararı). Kapatılırsa yönetici yalnız kendisine verilen projeleri görür. Üye yönetimi hakkı ayrıdır ve politikadan etkilenmez.
  - `viewer_download`: `viewer` ve `commenter` rollerinden `project.download`'u kaldırabilir. Önerilen varsayılan açık.
- **Silme** (sahibin kararı): `project.delete` proje sahibindedir. Politika açıksa kurum sahibi ve yöneticisinde de vardır. Silme yumuşaktır; geri yükleme bugünkü gibi yönetici/operatör işlemidir (`kentosd project restore`).
- **Kurum rolleri:**
  - Bugünkü beş rol kurum düzeyinde anlam taşımaya devam eder: proje oluşturma `project_manager` ve üstü, üye yönetimi `admin` ve üstü.
  - Bir kurum rolü, kendisiyle paylaşılmamış projeye erişim vermez. Tek istisna yönetici politikasıdır.
  - Dış misafir (`CLOUD-17`) kurum üyesi değildir; yalnız kendisine verilen projeleri görür.

### Denetimin yeri

- **Tek yetki işlevi:** projeye bağlı her kullanım durumu bir `ProjectAccess` alır (aktör, tenant, proje, etkin rol, izinler).
  - Bunu tek bir uygulama işlevi hesaplar.
  - Bugünkü kurum düzeyindeki `Access::require(Capability)` çağrıları projeye bağlı yollarda `ProjectAccess::require(permission)` olur.
  - HTTP, WebSocket aboneliği, nesne sayfaları, olaylar, komutlar, dışa aktarma, revizyon indirme, işler, Python ve AI (ADR 0013 komut yolu) aynı işlevden geçer (`CLOUD-11`).
- **Var olduğu bile sızmaz:** erişilemeyen proje 404 döner. Liste, sayılar, arama ve öneriler yetki süzgecinden sonra hesaplanır (`CLOUD-12`).
- **Satır güvenliği ikinci kattır:**
  - Proje listesinin politikası yetki tablosuna bakar.
  - Projeye bağlı tablolar (`feature`, `command_log`, `outbox_event`, projeye bağlı `audit_event`) işlemde ayarlanan `app.project_id`'ye kısıtlanır. Uygulama katmanındaki bir hata başka projenin satırını açamaz.
  - Bağlam `set_config(…, true)` ile yalnız işlemin ömrü kadardır, havuzdaki bağlantıya taşınmaz (`CLOUD-14`, `OPS-04`).
- **Geri alma (`CLOUD-13`):**
  - Yetki değişikliği audit ve outbox olayı yazar.
  - O projeye abone WebSocket'ler bu olayda erişimi yeniden sorar, yetkisi kalkanın aboneliği kapanır.
  - Her istek yetkiyi yeniden hesaplar; istek ömrünü aşan önbellek yoktur.
  - Uzun işler sonucu yazmadan önce yetkiyi yeniden denetler (`TX-07`, `JOB-06`).
  - Önceden indirilmiş kopyalar geri alınamaz; arayüz bunu iddia etmez.
- **Dış PostGIS (`CLOUD-14`):** etkin hak, KentOS proje izni ile kaynak bağlantısının yetkisinin kesişimidir. Kimlik bilgisi sunucuda kalır. Proje paylaşımı alıcıya veritabanı yetkisi vermez.

### Göç ve uyum

- Yeni projelerde yalnız sahip (ve politika gereği yöneticiler) erişir. Başkası paylaşımla eklenir.
- **Mevcut projelerde** göç, bugünkü erişimi açık yetkiye çevirir:
  - kurum `viewer` → `viewer`;
  - `editor` → `editor`;
  - `project_manager` → `manager`;
  - `owner` ve `admin` erişimi yönetici politikasından gelir.

  Böylece bugün çalışan akışlar (`pnpm e2e:cloud`) göçten sonra da çalışır. Göç sonrasında erişim ancak paylaşımla genişler.
- `MembershipView.capabilities` kurum düzeyinde kalır. Proje düzeyindeki izinler proje bilgisiyle (`ProjectInfo`) birlikte döner. İstemci düğmeleri buna göre açar; güvenlik sunucudadır.

## Sonuçlar

- **Uygulama dilimleri** (her biri kendi testleriyle; `KENTOS_TEST_DB=required`):
  1. Migration: `tenant.kind`, `tenant.owner_user_id`, `project.owner_user_id`, `project_grant`, kurum politikası sütunları; satır güvenliği politikaları.
  2. Uygulama katmanı: `ProjectAccess` ve izin denetimleri; projeye bağlı her HTTP ve WebSocket yolu.
  3. Kişisel alan: ilk kullanımda açılış, “Projelerim” listesi, kişisel alana yükleme.
  4. Paylaşım komutları: `project.share`, `project.access.revoke`; web paylaşım penceresi (`CLOUD-16`, `CLOUD-21`).
  5. Negatif yalıtım takımı (`CLOUD-12`, `TEST-10`):
     - kurumun paylaşılmamış üyesi;
     - başka kurumun üyesi;
     - yetkisi kaldırılmış kişi;
     - politika kapalıyken yönetici;
     - silinmiş proje;
     - tahmin edilen proje kimliği.
- Bu ADR bir izin şemasıdır, uygulanmış bir erişim modeli değildir. Kod dilimleri gelene kadar erişim bugünkü kurum düzeyindeki modeldir.
- Açık sorular (engelleyici değil):
  - grupların ilk sürüme girip girmeyeceği;
  - `viewer_download` varsayılanı;
  - misafir hesabının davetle mi, hesap açmayla mı geleceği (`CLOUD-16`, `CLOUD-17`).

## Uygulama notu (2026-09-25): dilimler 1, 2, 3, 5 ve dilim 4'ün sunucu tarafı

Yukarıdaki “Bu ADR bir izin şemasıdır” cümlesi artık geçerli değildir: erişim proje düzeyindedir.

- **Migration 0004** (`crates/server/postgres/migrations/0004_project_access.sql`):
  - `tenant.kind` (`personal` | `organization`, var olanlar `organization`), `tenant.owner_user_id` (kişisel alanın sahibi, kişi başına bir), `admins_access_all_projects` (varsayılan açık), `viewer_download` (varsayılan açık).
  - `project.owner_user_id` (zorunlu).
  - `project_grant`: tenant, proje, kişi, rol (`viewer`, `commenter`, `editor`, `manager`), isteğe bağlı bitiş zamanı, veren kişi.
  - `kentos.project_role`: rolü tek yerde hesaplar (sahiplik, kurum politikası, yetki; kurumda önce etkin üyelik ve koltuk). Satır güvenliği de uygulama da bunu kullanır; ikisi ayrışamaz.
  - `kentos.project_access`: açılışın bilgisi (rol, ad, silinmişlik, alanın adı ve türü, `viewer_download`). Proje olsa da olmasa da aynı sorguları çalıştırır; erişim yoksa boş döner.
  - `kentos.ensure_personal_tenant`: kişisel alanı yalnız bu işlev açar (sunucu rolü tenant açamaz, üye ekleyemez). Aynı anda gelen istekler tek alan açar.
- **Satır güvenliği (ikinci kat):**
  - Proje satırı yalnız projede rolü olana görünür.
  - Nesne, komut günlüğü, denetim, olay, olay ufku ve yetki satırları yalnız işlemde `app.project_id` o projeyse ve kişi projeyi görebiliyorsa görünür ve yazılır. Bağlam `Db::scoped` ile işlem ömürlüdür.
  - Kişinin kendi yetkileri her kapsamda görünür (“benimle paylaşılanlar” bunlardan başlar).
  - Bir kişisel alanın projesi kendisiyle paylaşılan kişi, o projenin kapsamında alanın adını ve sahibinin üyeliğini de görür. Kurumlarda böyle bir yol yoktur, çünkü kurum projeleri yalnız kurum üyeleriyle paylaşılır.
- **Mevcut veri, açık varsayılan:**
  - Mevcut projelerin sahibi, onu oluşturan kişidir (`created_by`).
  - Kurumun her üyesine her mevcut projede kurum rolünün karşılığı yetki olarak verilir: `viewer` → `viewer`, `editor` → `editor`, `project_manager` → `manager`. Üyeliğin durumu fark etmez.
  - `owner` ve `admin` erişimi politikadan gelir, yetki yazılmaz.
  - Üyeliğin durumu ve koltuk her istekte eskisi gibi denetlenir. Göç anında kimse erişim kazanmaz ya da kaybetmez.
  - Her proje için bir `project.access.migrate` denetim kaydı verilen yetkileri listeler.
  - Göçten sonra açılan proje yalnız sahibinindir (ve politika gereği yöneticilerin). Başkası paylaşımla eklenir.
- **Uygulama katmanı:**
  - `crates/server/application/src/access.rs`: `ProjectAccess`, `access::project`, rol → izin tablosu.
  - Projeye bağlı her yol buradan geçer: bilgi, nesneler, olaylar, komutlar (`commands.rs`), silme, erişim listesi, WebSocket aboneliği. Her istek yeniden hesaplar; önbellek yoktur.
  - Yazan komutlar (`project.changes`, silme, paylaşım) erişimi projenin kilidi altında yeniden sorar: kilitten önce kaldırılan ya da düşürülen yetki sayılır.
  - `MembershipView.capabilities` yalnız kurum düzeyindedir (`project.create`, `member.manage`). Proje izinleri `ProjectInfo.access` ve `ProjectSummary.access` ile gelir.
- **404:**
  - Var olmayan, paylaşılmamış, başka kurumun, tahmin edilen ya da biçimi bozuk kimlik aynı gövdeyi alır: `not_found`, “Proje bulunamadı.”
  - Silinmiş proje, erişimi olana 410, olmayana 404'tür. Listeler ve sayılar erişim süzgecinden sonra hesaplanır.
  - Koltuğu olmayan ya da üyeliği kapalı kurum üyesi, istediği proje kimliği ne olursa olsun aynı 403'ü alır; bu yanıt proje hakkında bir şey söylemez.
- **Kişisel alan:**
  - İlk girişte açılır (giriş yanıtı ve `/v1/me`). `MembershipView.tenantKind` = `personal`; arayüz “Kişisel” der. Üyelik listesinde kurumlardan sonra gelir.
  - Kişisel alana proje açmak bugünkü uçtur: `POST /v1/tenants/{kişisel alan}/projects`.
  - “Projelerim”: `GET /v1/me/projects`. Kişinin sahip olduğu ve kendisiyle paylaşılan projeler, bütün alanlardan. Yalnız politika ile erişilen kurum projeleri kurumun listesindedir.
  - `kentosd member add` kişisel alana üye eklemez.
- **Paylaşım (dilim 4, sunucu tarafı):**
  - Ürün komutları `project.share` v1 ve `project.access.revoke` v1 (katalogda, `project.share` ister). Erişim listesi: `GET …/projects/{proje}/access`; 26 Eylül'den beri kişi kişi bugünkü rol, kaynağı ya da erişememe nedeniyle (ADR 0024). Paylaşılacak kişiyi bulma: `GET …/access/candidates?q=` (ADR 0024).
  - Rol verilirken sahiplik verilmez (girdi tipi `GrantRole`). Kişi kendi erişimini değiştiremez, sahibin erişimi paylaşımla değişmez. Kurum projesi yalnız kurum üyesiyle paylaşılır.
  - Her değişiklik denetim kaydı ve nesnesiz bir `project.access` olayı yazar. Olay kim olduğunu taşımaz.
  - Açık WebSocket'ler her teslimden önce erişimi yeniden sorar: bu olayda, her commit'te ve 5 sn'lik yoklamada. Erişimi kalkana `not_found` gider ve aboneliği kapanır. Kurumdaki üyeliğin komut satırından kapatılması da en geç yoklamada yakalanır.
- **Kurum politikası:** işletmeci `kentosd tenant policy --slug KISA --admins-access-all on|off --viewer-download on|off` ile değiştirir; değişiklik denetime yazılır.
- **Web:** düğmeler projenin kendi izinlerine göre açılır. Kişisel alan kurum seçicisinde “Kişisel” adıyla durur; yükleme ve açma onu da kullanır.
- **Sınanan** (`KENTOS_TEST_DB=required`):
  - `crates/server/application/tests/access.rs`: rol kaynakları, ADR'nin negatif yalıtım listesi, satır güvenliği, kişisel alan (eşzamanlı açılış), “Projelerim”, paylaşım kuralları ve tekrarı, 0004'ün veri göçü.
  - `apps/api/src/http/tests.rs`: 404 gövdelerinin eşitliği, paylaşım ve “Projelerim” HTTP üzerinden.
  - `apps/api/src/http/ws_tests.rs`: paylaşımı kaldırılan kişinin açık aboneliği kesilir, ardından hiçbir olay gelmez.
- **Kalanlar:**
  - ~~Web'de paylaşım penceresi, “Benimle paylaşılanlar”, açık projede erişim değişince uyarı ve salt okunura geçiş~~: 26 Eylül'de yapıldı, [ADR 0024](0024-project-sharing-web.md). ~~Web'de “Projelerim” ayrı bir liste değil~~: 26 Eylül'den beri katalogun kendi listesidir ([ADR 0028](0028-project-catalog.md)).
  - ~~Geri yükleme yalnız işletmecinin~~: 26 Eylül, [ADR 0028](0028-project-catalog.md). Silme çöp kutusuna taşımadır (`project.trash`; `DELETE` yolu aynıdır). Geri yükleme (`project.restore`) ve çöp kutusundan kalıcı silme (`project.purge`, projenin adıyla onaylı) uygulamada da yapılır; ikisi de silmeyle aynı izni ister, `project.delete` (proje sahibi; politika açıksa kurum sahibi ve yöneticisi). Çöpe taşınan proje saklama süresi dolunca kalıcı olarak silinir (`KENTOS_TRASH_RETENTION_DAYS`, varsayılan 30 gün); bu kuraldan önce silinenler silinmez. İşletmecinin `kentosd project restore`'u durur. Arşiv (`project.archive`, `project.unarchive`) `project.edit` ister; arşivlenmiş proje salt okunurdur ve `kentos.project_access` bunu da döndürür (migration 0005).
  - Gruplar, kurum dışı misafir, e-postayla davet ve ortak kurumu olmayan kişiyi bulma (`CLOUD-16`, `CLOUD-17`; tasarım önerisi ADR 0024'te). Bugün paylaşım penceresi kişiyi projenin kurumunun (kişisel alanda arayanın kurumlarının) etkin üyeleri arasında adıyla ya da e-postasıyla bulur.
  - Sahiplik devri (`project.transfer`, `CLOUD-08`) ve kurumdan ayrılanın projelerini yöneticiye geçirmek.
  - `project.comment`, `project.download`, `project.history` ve `project.jobs.run` adlandırıldı ve rollere dağıtıldı; bunları isteyen uç henüz yok.
  - Kurum politikasını arayüzden değiştirmek; “bu kişi neden erişebiliyor?” açıklaması (`CLOUD-15`).
  - Dış PostGIS kesişimi (`CLOUD-14`): henüz kaynak bağlantısı yok.
  - Proje listesi her satır için rol işlevini çağırır; çok projeli kurumlarda ölçülmeli (`CLOUD-28`).
