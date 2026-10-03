# Bulut arayüzünün ortak durumları

Bulut projeleri penceresinin (katalog, [ADR 0028](../../docs/adr/0028-project-catalog.md)) ve Paylaş penceresinin ([ADR 0015](../../docs/adr/0015-project-ownership-and-access.md), [0024](../../docs/adr/0024-project-sharing-web.md), [0035](../../docs/adr/0035-project-invitations-and-guests.md), [0042](../../docs/adr/0042-web-invitations.md)) iki platformda aynı sözleri ve kuralları kullanması için. Sunucunun işi (arama, sıralama, sayfalama, erişim denetimi) burada değildir; burada pencerenin ne yazdığı ve neyi sunduğu vardır.

- **Web**: `apps/web/src/app/cloud/catalogPlan.test.ts` (Vitest), `app/cloud/catalog.ts` ve `app/cloud/catalogPlan.ts`'e karşı. Pencere (`ui/cloud/CatalogDialog.ts`, `catalogRows.ts`, `catalogDetails.ts`, `ProjectActions.ts`) bu kararları yalnız çizer.
- **Masaüstü**: katalog penceresi (`apps/desktop/src/cloud/`) aynı dosyayı okur.
- **Durum çubuğunun bulut hücreleri**: web'de `apps/web/src/ui/statusbar/cellsPlan.test.ts`, `ui/statusbar/cellsPlan.ts`'e karşı; hücreler (`cloudCells.ts`, `StatusBar.ts`) yalnız çizer. Cevaplar koddan ayrı, `scripts/fixtures/cells_cases.py` ile bulunur ve yazılır (`--check` karşılaştırır).
- **Proje formları**: web'de `apps/web/src/app/cloud/formsPlan.test.ts`, `app/cloud/formsPlan.ts` (ve etiketler için `catalog.ts`'in `parseTags`'i) karşısında; formlar (`ui/cloud/ProjectForms.ts`, `ProjectActions.ts`'in Yeniden adlandır'ı) yalnız çizer. Cevaplar koddan ayrı, `scripts/fixtures/forms_cases.py` ile bulunur ve yazılır (`--check` karşılaştırır).
- **Dosya projesinin revizyonları** ([docs/specs/file-revisions.md](../../docs/specs/file-revisions.md)): web'de `apps/web/src/app/cloud/fileRevisionsPlan.test.ts`, `app/cloud/fileRevisionsPlan.ts`'e ve kayıt hücresi için `ui/statusbar/cellsPlan.ts`'e karşı; `app/cloud/fileProject.ts` durumunu planın `step`'iyle tutar, `ui/cloud/FileConflict.ts` planın sorularını sorar. Masaüstünde `apps/desktop/src/cloud/revisions_tests.rs`, `cloud/revisions.rs`'e (ve kayıt hücresi için `cloud/cells_plan.rs`'e) karşı; `cloud/file_follow.rs` olayları izler, soruları sorar (ADR 0119). Cevaplar koddan ayrı, `scripts/fixtures/file_revisions_cases.py` ile bulunur ve yazılır (`--check` karşılaştırır).
- **Paylaş**: web'de `apps/web/src/app/cloud/sharePlan.test.ts`, `app/cloud/sharing.ts`, `invitations.ts` ve `sharePlan.ts`'e karşı; pencere (`ui/cloud/ShareDialog.ts`, `shareFind.ts`, `shareInvites.ts`) yalnız çizer. Dosyanın cevapları koddan ayrı, `scripts/fixtures/share_cases.py` ile bulunur ve yazılır (`--check` hiçbir şey yazmadan karşılaştırır); masaüstünün Paylaş penceresi aynı dosyayı okur.

| Dosya | İçerik |
|---|---|
| `v1/catalog.json` | Katalogun sözleri ve kuralları |
| `v1/share.json` | Paylaş penceresinin sözleri ve kuralları |
| `v1/cells.json` | Durum çubuğunun bulut hücreleri: kayıt hücresi, sunucu hücresi ve hesap menüsü |
| `v1/forms.json` | Proje formlarının (Proje bilgileri, Yeniden adlandır, Kopyasını oluştur, öbür saklama biçimine çevirme) sözleri ve kuralları |
| `v1/file-revisions.json` | Açık dosya projesinin revizyonları: yeni revizyonun öğrenilmesi, hücrenin durumu, Kaydet'in ilk adımı, yeniden eşitleme, sorular ve cevapları, sözler |

## Biçim (`kentos.catalog`, sürüm 1)

| Alan | Anlamı |
|---|---|
| `format`, `version` | `"kentos.catalog"`, `1` |
| `timeZone` | Zamanlar cihazın yerel saatiyle yazılır; durumlar bu bölgeyle (`Europe/Istanbul`) denetlenir |
| `views` | Listeler, pencerenin sırasıyla: `id`, `label`, `sorts` (sunulan sıralamalar; ilki listenin kendi sırası), `empty` (arama yokken boş listenin sözü), `note` (listenin üstündeki not; varsa) |
| `sorts`, `states`, `types`, `typeHint` | Sıralama, durum ve iş türü adları; iş türünün ne anlama gelmediğini söyleyen not |
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

## Biçim (`kentos.cells`, sürüm 1)

| Alan | Anlamı |
|---|---|
| `format`, `version` | `"kentos.cells"`, `1` |
| `saveTexts` | Veritabanı projesinin kayıt durumu sözü (`state`; bekleyen ya da çakışan sayı `sample` ile) |
| `linkTexts` | Canlı bağlantının sözü (`none` boş) |
| `views` | Kayıt hücresi: girdi (`none`; `database`: durum, bekleyen, çakışma sayısı; `file`: durum, dayandığı revizyon `base` ('0' ilkinden önce), yükleme `progress` 0…1, çakışmadaki revizyon, sunucudaki yeni revizyon) → `view` (`hidden`, söz, lambanın durumu) ve tıklamanın komutu (`action`: çakışmada `cloud.conflicts`; dosyada yeni revizyon `cloud.openNewest`; salt okunur ya da kayıt yoldayken hiçbiri; yoksa `file.save`, silinmiş ya da erişimi kaldırılmış projede de: yerel dosyaya kaydeder). Yükleme yüzdesi JavaScript'in yuvarlamasıyladır (yarım yukarı) |
| `ago` | “ne zaman önce”: hiç yoksa “henüz yok”; 60 saniyeden az saniye, yoksa dakika (ikisi de yarım yukarı yuvarlanır); ileri bir saat eksi saniye yazar |
| `databaseTips` | Veritabanı projesinin hücre ipucu: nerede (“kurum › proje”), nasıl kaydedildiği ve son kayıt, canlı bağlantı, hata, bu tarayıcının taslak saklayıp saklayamadığı; silinmiş, erişimi kaldırılmış ve arşivlenmiş projede ayrı sözler |
| `fileTips` | Dosya projesinin hücre ipucu: dayandığı revizyon (sayı ekten ayrı yazılır), Kaydet'in ne yaptığı, pencerenin son kaydı, sunucudaki yeni revizyon (kaydedenle), hata, bağlantı, kaydedilmemiş değişiklik |
| `serverTexts`, `serverTips` | Sunucu hücresinin sözü ve ipucu. Girdi: durum, sunucunun son iyi yanıtı (sözleşmenin `Health`'i: `status` “ok”, hizmet, sürüm, bilinirse `commit`, sözleşme sürümü), neden ve geliştirme yapısı mı. Bağlıyken hizmet, sürüm, commit'in ilk 8 hanesi ve sözleşme sürümü; uyumsuzken hizmet, sürüm ve commit, ardından iki sözleşme sürümünü de söyleyen neden; sorulurken; yokken nedeni, çizimin sunucusuz çalıştığı ve (geliştirme yapısında) sunucunun nasıl başlatılacağı |
| `projectActions` | Açık projenin menüdeki işleri ve her birinin istediği yetki |
| `accountRows` | Hesap menüsü: başlık (hesabın adı · açık projenin çalışma alanı, ya da “Oturum açılmadı”), giriş/çıkış, bulut komutları, açık projenin işleri, sunucu denetimi. Açık projede kapalı olan ve hesabın yetkisi olmayan iş hangi yetkinin eksik olduğunu söyler; başka bir nedenle kapalı ya da yetkisi eksik ama açık olan söylemez |

Kayıt hücresinin iki yeni hâli bu dosyada değil, `file-revisions.json`'dadır: kaydedilmemiş iş üstünde yeni revizyonun sözü (girdide `dirty`) ve ipucunda kim ile ne zaman (`newer.at`). Buradaki durumlarda `dirty` ve `at` yoktur; onlarsız sözler değişmedi.

## Biçim (`kentos.fileRevisions`, sürüm 1)

Durumlar, bir açık dosya projesinin bildiklerinden (`RevisionState`) yola çıkar: `base` (çizimin dayandığı revizyon, yoksa `"0"`), `newer` (sunucudaki yeni revizyon: `revision`, `by` kaydeden ya da boş, `at` RFC 3339 zamanı ya da `null`), `conflict` (`expected`, `actual`), `dirty`, `stage` (`idle`, `encoding`, `uploading`, `verifying`), `failed`, `writable`, `ended` (`deleted`, `revoked`, `archived` ya da `null`). Anlamları [docs/specs/file-revisions.md](../../docs/specs/file-revisions.md) §1'dedir.

| Alan | Anlamı |
|---|---|
| `format`, `version`, `timeZone` | `"kentos.fileRevisions"`, `1`; zamanlar cihazın yerel saatiyle yazılır, durumlar bu bölgeyle (`Europe/Istanbul`) denetlenir |
| `resyncRetryMs` | Yanıtsız kalan yeniden eşitlemenin yeniden abone olmadan önce beklediği süre (30 000 ms) |
| `texts` | Tek satırlık sözler, `{ sample: [değerler], text }`: Kaydet sürerken (`busy`), projesi silinmiş ya da erişimi kalkmışken (`unreadableDeleted`, `unreadableRevoked`), yeniden eşitlemeye yanıt gelmeyince (`resyncFailed`), çizim yerel dosyaya kaydedilip projeden ayrılınca (`detached`), son revizyon açılamayınca (`openFailed`: ad ve neden) |
| `marks` | Geçmiş sekmesinin işaretleri: en yeni revizyon, açık çizimin dayandığı revizyon |
| `who` | Kim ve ne zaman: “ (Mehmet Demir, 27.09.2026 14:32)”; yalnız bilinen yazılır, ikisi de yoksa boş |
| `cellStates` | Bilinenlerden hücrenin durumu (`cellState`): bitiş, Kaydet'in aşaması, çakışma, yeni revizyon (kaydedilmemiş işte ve salt okunurda da), salt okunur, başarısızlık, kaydedilmemiş, kaydedildi |
| `steps` | Tek girdi (`from`, `input`) → sonraki bilinenler ve `say` (yeni revizyon bir kez söylenecek mi). Girdiler: `dirty`, `newest` (sunucunun en yenisi ya da `null`), `stage`, `committed` (`revision`, `dirty`), `refused` (`actual`), `failed`, `unchanged`, `access` (`writable`), `ended` (`why`) |
| `saveSteps` | Kaydet'in ilk adımı (`saveStep`): `ended`, `readonly`, `conflict` (soru yeniden), `unchanged`, `behind` (yeni revizyon biliniyor: hiçbir şey yüklenmez, durum reddedilmiş gibi olur, soru gelir), `save` |
| `events` | Olay dizisi (`seq`, `kind`, `requestId`) ve bu pencerenin kendi istek kimlikleri (`own`) → son imleç, bitiş (`deleted`; `archived` ve bu pencere yaptıysa `quiet`), erişimin yeniden sorulması, en yeni revizyonun sorulması (dizi başına bir kez) |
| `newest` | Revizyon listesi (`GET …/files`, sözleşmenin `FileRevisions`'ı) → en yeni revizyon, kim ve ne zaman; revizyon yoksa `null` |
| `resync` | Yeniden eşitlemede projenin cevabı (`project` ve durumu ile `eventCursor`, `deleted`, `notFound`, `forbidden` ve iletisi, `unreachable`, `failed`) → `follow` (yeni imleç; ardından erişim ve en yeni revizyon sorulur), `end` (neden ve sunucunun sözü), `retry` |
| `offers` | Sorulan (`name`, `state`, `busy`: Kaydet sürüyor, `via`: `newest` ya da `conflict`) → `ask` ve sorunun tamamı (kimliği, başlık, soru, maddeler, cevaplar sırasıyla: `value`, `label`, `kind`, `aside`, `does`: `copy` / `local` / `latest` / `nothing`, `work`: kaydedilmemiş işe ne olduğu `saved` / `dropped` / `kept` / `none`; `cancel`), `say` (`tone`, `line`) ya da `none` |
| `lines` | Yeni revizyonun günlük satırı: temiz çizimde, kaydedilmemiş işte, kim ya da zaman bilinmezken, çizimin revizyonu yokken |
| `tips` | Kayıt hücresinin ipucu (`ui/statusbar/cellsPlan.ts` `fileTip`), yeni revizyonun kimi ve zamanıyla, kaydedilmemiş iş varken ve yokken |
| `cells` | Hücrenin sözü, durumu ve tıklaması, bilinenlerden (yükleme ilerlemesi 0) |
| `historyMarks` | Geçmiş'te bir revizyon satırının işaretleri (`revision`, `current`, `openBase`: proje burada açık değilse `null`) |
| `traces` | Bütün sıralar: başlangıç, adımlar, son bilinenler. Bir adım üç türlüdür. Girdi adımında girdi, `say` ve hücre (`state`, `text`, `action`) vardır. Kaydet adımında (`save`) ilk adım ve hücre vardır; `behind` durumu `refused` ile, `unchanged` ise `unchanged` ile değiştirir, `save`'de aşamalar sonraki girdilerdir. Tıklama adımında (`offer`, `busy`) gelen vardır: `ask` ve sorunun kimliği ile cevapları, ya da `say` ve satırı |

## Biçim (`kentos.forms`, sürüm 1)

| Alan | Anlamı |
|---|---|
| `format`, `version` | `"kentos.forms"`, `1` |
| `texts` | Formların sözleri; değerlerden yapılan söz `{ sample: [değerler], text }`. İş türlerinin adları ve iş türünün notu `catalog.json`'dadır |
| `limits` | Adın (200) ve açıklamanın (2000) en çok uzunluğu |
| `tags` | Etiket alanının okunuşu: virgülle bölünür, her parça kırpılır ve içindeki boşluklar teke iner, boşlar düşer |
| `copyNames` | Kopyaya önerilen ad: `<ad> (kopya)` |
| `metadata` | Proje bilgileri: gösterilen katalog sürümü (`shown`) ve formun şimdiki değerleri (`now`) → gönderilen yama (yalnız değişenler: ad kırpılarak, tür, açıklama, etiketler sırasıyla) ve Kaydet açık mı (ad boş değil ve bir şey değişti) |
| `rename` | Yeniden adlandır açık mı: kırpılmış ad boş değil ve şimdiki addan başka |
| `duplicate` | Kopyasını oluştur açık mı: bir çalışma alanı var ve ad boş değil |
| `places` | Sunulan çalışma alanları: etkin, koltuklu ve `project.create` yetkili üyelikler, kaynağın alanı başta, gerisi hesabın sırasıyla; ad kurumda kurumun adı, kişisel alanda “Kişisel”. Liste ikiden azken kapalıdır. Üyelikler sözleşmenin `MembershipView`'ıdır (rol `TenantRole`); rol okunmaz, bu yüzden üyeliklerde farklıdır |
| `convert` | Öbür saklama biçimine çevirme: proje (`name`, `storage`) ve açık çizimin kaydedilmemiş değişikliği (`openDirty`: bu proje burada açık ve değişmiş) → hedef, başlık, giriş, sonuçlar (veritabanına aktarırken sunucu sınırı ve varsa kaydedilmemiş değişiklik notu), ad alanının örneği, sürerken söz |
| `counts` | Sunucunun ondalık metin olarak gönderdiği sayılar, Türkçe binlik ayırıcıyla (1.234.567; JavaScript sayısına çevrilmeden); sayı olmayan metin olduğu gibi |
| `convertedLines` | Çevirmenin günlük satırı |
| `failures` | Başarısız isteğin cümlesi: çakışma, bağlantı yok, sunucunun sözü, sayfanın hatası, hata olmayan şey |
| `convertFailures` | Çevirmenin reddi: sunucunun reddettiği nesne dosyadaki yeriyle (`entities[i]` → “(dosyanın i+1. nesnesi; hiçbir proje oluşturulmadı).”) |

## Kurallar

- Her şey tam metinle karşılaştırılır.
- Web'in bugünkü sözleri ve kuralları yazılıdır; biri değişince bu dosya, iki çalıştırıcı ve bu belge birlikte değişir.
- Beklenen değeri hataya göre yenilemek yasaktır (CLAUDE.md §9.4).
