# Bulut arayüzünün ortak durumları

Bulut projeleri penceresinin (katalog, [ADR 0028](../../docs/adr/0028-project-catalog.md)) ve Paylaş penceresinin ([ADR 0015](../../docs/adr/0015-project-ownership-and-access.md), [0024](../../docs/adr/0024-project-sharing-web.md), [0035](../../docs/adr/0035-project-invitations-and-guests.md), [0042](../../docs/adr/0042-web-invitations.md)) iki platformda aynı sözleri ve kuralları kullanması için. Sunucunun işi (arama, sıralama, sayfalama, erişim denetimi) burada değildir; burada pencerenin ne yazdığı ve neyi sunduğu vardır.

- **Web**: `apps/web/src/app/cloud/catalogPlan.test.ts` (Vitest), `app/cloud/catalog.ts` ve `app/cloud/catalogPlan.ts`'e karşı. Pencere (`ui/cloud/CatalogDialog.ts`, `catalogRows.ts`, `catalogDetails.ts`, `ProjectActions.ts`) bu kararları yalnız çizer.
- **Masaüstü**: katalog penceresi (`apps/desktop/src/cloud/`) aynı dosyayı okur.
- **Paylaş**: web'de `apps/web/src/app/cloud/sharePlan.test.ts`, `app/cloud/sharing.ts`, `invitations.ts` ve `sharePlan.ts`'e karşı; pencere (`ui/cloud/ShareDialog.ts`, `shareFind.ts`, `shareInvites.ts`) yalnız çizer. Dosyanın cevapları koddan ayrı, `scripts/fixtures/share_cases.py` ile bulunur ve yazılır (`--check` hiçbir şey yazmadan karşılaştırır); masaüstünün Paylaş penceresi aynı dosyayı okur.

| Dosya | İçerik |
|---|---|
| `v1/catalog.json` | Katalogun sözleri ve kuralları |
| `v1/share.json` | Paylaş penceresinin sözleri ve kuralları |

## Biçim (`kentos.catalog`, sürüm 1)

| Alan | Anlamı |
|---|---|
| `format`, `version` | `"kentos.catalog"`, `1` |
| `timeZone` | Zamanlar cihazın yerel saatiyle yazılır; durumlar bu bölgeyle (`Europe/Istanbul`) denetlenir |
| `views` | Listeler, pencerenin sırasıyla: `id`, `label`, `sorts` (sunulan sıralamalar; ilki listenin kendi sırası), `empty` (arama yokken boş listenin sözü), `note` (listenin üstündeki not; varsa) |
| `sorts`, `states`, `types`, `typeHint` | Sıralama, durum ve proje türü adları; türün ne anlama gelmediğini söyleyen not |
| `emptySearch`, `noOrganization` | Arama ya da tür süzgeci varken boş liste; etkin kurum üyeliği yokken “Kurum projeleri” |
| `tips` | Yetkinin olmadığı işin nedeni (`denied`: `{name}`, `{what}`, `{permission}` yerlerine konur) ve arşivlenmiş projenin nedeni (`archived`) |
| `project` | Durumların temel projesi (`ProjectSummary`); her durum üstüne değişikliklerini yazar (`access` alan alan birleşir) |
| `details` | Seçili projenin bölmesi: `{ project, open }` → `favorite` (düğmenin adı; çöpte `null`), `chips`, `tabs` (çöpte `null`), `facts` (gösterilen satırların adları, sırasıyla), `actions` (sırasıyla `id`, `label`, `icon`, `why`: kapalıysa nedeni, açıksa `null`, `danger`: silen eylem) |
| `primary` | Pencerenin tek amber düğmesi: `{ view, project }` (`project` `null`: seçim yok) → `label`, `enabled`, `why` (düğmenin ipucu) |
| `rows` | Listenin satırı: `{ view, sort, open, place, project }` (`place`: projenin nerede olduğu, hesabın adlandırdığı gibi) → `marks` (adın yanındakiler: yıldızın ipucu “Favorilerinizde”, “Açık”, “Arşivde”), `sub` (altında: tür ve yeri ya da sahibi; çöpte taşıyan), `side` (sağda: paylaşılanlarda rol, çöpte silinme günü, listenin sıraladığı zaman) |
| `questions` | Çöpe taşıma, kalıcı silme ve arşivleme soruları: `args` → `title`, `message`, `details`, `action` |
| `lines` | Eylemlerin günlüğe ve listenin altına yazdığı satırlar: `line` (adı), `args`, `text` |

## Biçim (`kentos.share`, sürüm 1)

| Alan | Anlamı |
|---|---|
| `format`, `version`, `timeZone` | `"kentos.share"`, `1`; tarihler cihazın yerel saatiyle yazılır, durumlar bu bölgeyle (`Europe/Istanbul`) denetlenir |
| `roles` | Rol adları (`labels`), paylaşımın verdiği roller zayıftan güçlüye (`grant`: sahiplik paylaşılmaz, devredilir), davetin verdikleri (`invite`: yönetici yok), her rolün bir satırlık açıklaması (`hints`) |
| `blocks`, `storage`, `invitationStates` | Erişemeyenin nedeni; saklama biçiminin adı ve açıklaması; davetin durum adları |
| `inviteDays` | Davetin varsayılan bekleyişi (14 gün), en çoğu (90) ve listedeki seçenekler adlarıyla |
| `texts` | Pencerenin sözleri (sekmeler, alanlar, düğmeler, boş listeler, süren işler, başarısızlıkların başı); bir değerden yapılan söz `{ sample, text }` |
| `access` | Durumların erişim listesi (`ProjectAccessList`): sahip, kurum politikasından gelen yönetici (bir paylaşımı da var), paylaşımla yönetici, süreli düzenleyici, süresi dolmuş paylaşım, koltuğu olmayan üye, politikadan gelen yönetici, misafir, adı görünmeyen hesap |
| `people` | Kişiler sekmesi: `{ me, mayShare, list }` (`list`: `access`'in üstüne yazılanlar) → `storage` (`lead` kalın baş, `detail`), `rows`, `count` ("n kişi erişebiliyor": erişebilenler), `policy` (listenin altındaki not) |
| `sources` | Bir kişinin erişiminin nereden geldiği ya da neden erişemediği, bir satırda |
| `shareTips`, `shared`, `revokeQuestions` | Paylaş düğmesi kapalıyken ipucu; paylaşmanın sonucu (değişmedi, rol değişti, eklendi); erişimi kaldırma sorusu (misafir davetle, üye yeniden paylaşımla geri gelir) |
| `finder` | Kişi ekle: kimse bulunmayınca söz ve bütün bir e-postaysa davet önerisi (`nobody`), bulunanın şu anki rolü (`notes`), aramanın başladığı uzunluk (boşluklar dışında 2 harf; `searches`) |
| `emails` | Davetin e-postası: sunucunun aldığı mı, değilse nedeni |
| `expiry` | Davetin `expiresAt`'i: `now`'dan `days` gün sonra; varsayılanda yok, en uzunu sınırın 10 dakika içinde |
| `endOfDay` | Paylaşımın bitişi: seçilen günün yerel 23:59:59'u; ayın olmayan günü ve bozuk yazı `null` |
| `dates` | Arayüzün tarihi (gg.aa.yyyy, yerel saatle) |
| `inviteTips` | Davet et düğmesi kapalıyken ipucu |
| `invitations` | Davetlerin sırası (bekleyenler önce, her bölüm yeniden eskiye), sayı ("n bekliyor"), satırın alt yazısı, davetten önce sorulan (aynı adrese bekleyen davet, zaten erişebilen) ve sorunun sözleri, notlar, yeni davetin bağlantısı (bir kez gösterilir; yinelenen isteğin cevabında yoktur), geri alma sorusu, simgenin harfi, bağlantının kendisi (`?davet=` belirteç) |
| `accepted` | Kabul edilen davetin projeye nasıl eriştirdiği |
| `envelopes` | Pencerenin gönderdiği ürün komutları (`project.share`, `project.access.revoke`, `project.invite`, `project.invitation.revoke`); `requestId` ve `idempotencyKey` her istekte yenidir, burada yazılmaz |
| `failures` | Başarısız isteğin sözü: bağlantı yok, oturum bitti, proje yok, sunucu geçici olarak yanıt vermiyor, öbür durumlarda sunucunun kendi iletisi |
| `lines` | Günlüğe ve durum satırına yazılanlar |

## Kurallar

- Her şey tam metinle karşılaştırılır.
- Web'in bugünkü sözleri ve kuralları yazılıdır; biri değişince bu dosya, iki çalıştırıcı ve bu belge birlikte değişir.
- Beklenen değeri hataya göre yenilemek yasaktır (CLAUDE.md §9.4).
