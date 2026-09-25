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
