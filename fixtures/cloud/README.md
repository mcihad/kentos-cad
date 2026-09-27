# Bulut arayüzünün ortak durumları

Bulut projeleri penceresinin (katalog, [ADR 0028](../../docs/adr/0028-project-catalog.md)) ve Paylaş penceresinin ([ADR 0015](../../docs/adr/0015-project-ownership-and-access.md), [0024](../../docs/adr/0024-project-sharing-web.md), [0035](../../docs/adr/0035-project-invitations-and-guests.md), [0042](../../docs/adr/0042-web-invitations.md)) iki platformda aynı sözleri ve kuralları kullanması için. Sunucunun işi (arama, sıralama, sayfalama, erişim denetimi) burada değildir; burada pencerenin ne yazdığı ve neyi sunduğu vardır.

- **Web**: `apps/web/src/app/cloud/catalogPlan.test.ts` (Vitest), `app/cloud/catalog.ts` ve `app/cloud/catalogPlan.ts`'e karşı. Pencere (`ui/cloud/CatalogDialog.ts`, `catalogRows.ts`, `catalogDetails.ts`, `ProjectActions.ts`) bu kararları yalnız çizer.
- **Masaüstü**: katalog penceresi (`apps/desktop/src/cloud/`) aynı dosyayı okur.
- **Proje formları**: web'de `apps/web/src/app/cloud/formsPlan.test.ts`, `app/cloud/formsPlan.ts` (ve etiketler için `catalog.ts`'in `parseTags`'i) karşısında; formlar (`ui/cloud/ProjectForms.ts`, `ProjectActions.ts`'in Yeniden adlandır'ı) yalnız çizer. Cevaplar koddan ayrı, `scripts/fixtures/forms_cases.py` ile bulunur ve yazılır (`--check` karşılaştırır).
- **Paylaş**: web'de `apps/web/src/app/cloud/sharePlan.test.ts`, `app/cloud/sharing.ts`, `invitations.ts` ve `sharePlan.ts`'e karşı; pencere (`ui/cloud/ShareDialog.ts`, `shareFind.ts`, `shareInvites.ts`) yalnız çizer. Dosyanın cevapları koddan ayrı, `scripts/fixtures/share_cases.py` ile bulunur ve yazılır (`--check` hiçbir şey yazmadan karşılaştırır); masaüstünün Paylaş penceresi aynı dosyayı okur.

| Dosya | İçerik |
|---|---|
| `v1/catalog.json` | Katalogun sözleri ve kuralları |
| `v1/share.json` | Paylaş penceresinin sözleri ve kuralları |
| `v1/forms.json` | Proje formlarının (Proje bilgileri, Yeniden adlandır, Kopyasını oluştur, öbür saklama biçimine çevirme) sözleri ve kuralları |

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

## Biçim (`kentos.forms`, sürüm 1)

| Alan | Anlamı |
|---|---|
| `format`, `version` | `"kentos.forms"`, `1` |
| `texts` | Formların sözleri; değerlerden yapılan söz `{ sample: [değerler], text }`. Proje türlerinin adları ve türün notu `catalog.json`'dadır |
| `limits` | Adın (200) ve açıklamanın (2000) en çok uzunluğu |
| `tags` | Etiket alanının okunuşu: virgülle bölünür, her parça kırpılır ve içindeki boşluklar teke iner, boşlar düşer |
| `copyNames` | Kopyaya önerilen ad: `<ad> (kopya)` |
| `metadata` | Proje bilgileri: gösterilen katalog sürümü (`shown`) ve formun şimdiki değerleri (`now`) → gönderilen yama (yalnız değişenler: ad kırpılarak, tür, açıklama, etiketler sırasıyla) ve Kaydet açık mı (ad boş değil ve bir şey değişti) |
| `rename` | Yeniden adlandır açık mı: kırpılmış ad boş değil ve şimdiki addan başka |
| `duplicate` | Kopyasını oluştur açık mı: bir çalışma alanı var ve ad boş değil |
| `places` | Sunulan çalışma alanları: etkin, koltuklu ve `project.create` yetkili üyelikler, kaynağın alanı başta, gerisi hesabın sırasıyla; ad kurumda kurumun adı, kişisel alanda “Kişisel”. Liste ikiden azken kapalıdır |
| `convert` | Öbür saklama biçimine çevirme: proje (`name`, `storage`) ve açık çizimin kaydedilmemiş değişikliği (`openDirty`: bu proje burada açık ve değişmiş) → hedef, başlık, giriş, sonuçlar (veritabanına aktarırken sunucu sınırı ve varsa kaydedilmemiş değişiklik notu), ad alanının örneği, sürerken söz |
| `counts` | Sunucunun ondalık metin olarak gönderdiği sayılar, Türkçe binlik ayırıcıyla (1.234.567; JavaScript sayısına çevrilmeden); sayı olmayan metin olduğu gibi |
| `convertedLines` | Çevirmenin günlük satırı |
| `failures` | Başarısız isteğin cümlesi: çakışma, bağlantı yok, sunucunun sözü, sayfanın hatası, hata olmayan şey |
| `convertFailures` | Çevirmenin reddi: sunucunun reddettiği nesne dosyadaki yeriyle (`entities[i]` → “(dosyanın i+1. nesnesi; hiçbir proje oluşturulmadı).”) |

## Kurallar

- Her şey tam metinle karşılaştırılır.
- Web'in bugünkü sözleri ve kuralları yazılıdır; biri değişince bu dosya, iki çalıştırıcı ve bu belge birlikte değişir.
- Beklenen değeri hataya göre yenilemek yasaktır (CLAUDE.md §9.4).
