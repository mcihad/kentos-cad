# ADR 0035: Bağlantıyla davet, e-posta eşleşmesi ve misafir

- **Durum:** kabul edildi (2026-09-26, sahibin kararı: “Bağlantı + e-posta + misafir”). Yön TODOS.md `CLOUD-16` (davet belirli e-posta ve süreyle bağlı, tek kullanımlık, başka hesapla kabul yok) ve `CLOUD-17`'den (kurum dışı misafir yalnız paylaşılan projeleri görür; dış paylaşım politikayla kapatılabilir) gelir. ADR 0024'teki öneri taslağının kararıdır.
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** CLAUDE.md §16; TODOS.md §12; ADR 0007 (kimlik), 0015 (proje erişimi), 0024 (paylaşım penceresi)

## Bağlam

- Paylaşım penceresi kişiyi yalnız projenin kurumunun (kişisel projede arayanın kurumlarının) üyeleri arasında bulur (ADR 0024). Hesap sayımını önlemek için genel e-posta araması yoktur.
- Kurum projesinde üye olmayanın paylaşımı işlemez (`notMember`); kişisel alanda üyelik gerekmez.
- E-posta gönderme altyapısı yoktur. Hesabın e-postasının sahibine ait olduğu kaydedilmiyordu.

## Karar

### Davet

- `project.invite` v1 `{ email, role, expiresAt? }`. `project.share` ister; çöpteki projede yapılmaz.
  - E-posta kırpılır, küçük harfle saklanır ve biçimi denetlenir.
  - Rol en çok düzenleyicidir. Yöneticilik ve paylaşım kurum üyelerinde kalır; onlara mevcut paylaşımla verilir.
  - Bitiş verilmezse 14 gün sonradır, en çok 90 gün sonra olabilir.
- Belirteç 32 rastgele bayttır (pgcrypto), 64 onaltılık hane olarak verilir. Sunucuda yalnız SHA-256'sı saklanır.
- Belirteç yalnız ilk yanıttadır. Yeniden denemenin saklı yanıtında yoktur, çünkü komut günlüğüne belirteç yazılmaz. Bağlantıyı yitiren, yeniden davet eder.
- Bağlantıyı davet eden kişi kendisi iletir; sunucu e-posta göndermez. Web bağlantıyı belirteçten kurar.
- Bir projede aynı e-postaya tek bekleyen davet olur: yenisi eskisini geri alır, eski bağlantı çalışmaz.
- `project.invitation.revoke` v1 `{ invitationId }` bekleyen daveti geri alır (`project.share`). Kabul edilmiş davetin verdiği erişim `project.access.revoke` ile kaldırılır.
- `GET …/invitations` (`project.share`): bekleyenler ve son 30 günün kabul edilen, geri alınan, süresi dolan davetleri; en yenisi önce. Durumu `pending`, `accepted`, `revoked` ya da `expired`'dır.

### Kabul

- `POST /v1/invitations/accept { token }`: oturum açmış hesap alır. Hesabın o projede henüz rolü olmadığı için ayrıcalıklı kısmı sahibin haklarıyla çalışan `kentos.accept_invitation` yapar:
  - belirteç bekleyen, süresi dolmamış bir davet bulmalıdır; yoksa 404. Aynı yanıt kullanılmış, geri alınmış ve süresi dolmuş davete de verilir;
  - hesabın e-postası davetinkiyle aynı olmalı (büyük/küçük harf fark etmez) ve doğrulanmış olmalıdır, yoksa 403 ve nedeni:
    - OpenID hesabında sağlayıcının son girişteki `email_verified`'ı sayılır. Metin olarak gelen "true" da kabul edilir;
    - yerel hesabın e-postasını yönetici girer ve doğrulanmış sayılır;
  - silinmiş (çöpteki) projede ya da etkin olmayan kurumda 410;
  - kurum projesinde etkin, koltuklu üye düz paylaşım alır. Hiç üye olmayan kişi misafir olur, kurum misafir kabul ediyorsa; etmiyorsa 403. Üyeliği ya da koltuğu gitmiş üye misafir olamaz (403);
  - kişisel alanın projesinde herkes düz paylaşım alır (üyelik zaten gerekmez);
  - var olan paylaşımın rolü daha güçlüyse o kalır (yönetici, görüntüleyici davetiyle düşmez). Misafirin rolü en çok düzenleyicidir. Aynı kalan paylaşım bitişini korur; davetin verdiği yeni rolün bitişi yoktur;
  - projenin sahibi kendi davetini açarsa sahip kalır.
- Davet tek kullanımlıktır: kabul edilince `accepted` olur. Projenin denetim kaydına `project.invitation.accept` yazılır; açık bağlantılara `project.access` olayı gider.
- Yanıt `InvitationAccepted`'dır: kurum, proje, rol ve misafir olup olmadığı. Web projeyi doğrudan açar.

### Misafir (`CLOUD-17`)

- `project_grant.guest`: kurum dışından birinin davetle aldığı paylaşım. `kentos.project_role` onu, kişi kurumun hiç üyesi değilse ve kurum misafir kabul ediyorsa sayar.
- Misafir yalnız kendisiyle paylaşılan projeleri görür. “Projelerim”de ve katalogda görünürler: çalışma alanları paylaşım aldığı kurumları da kapsar. Kurumun proje listesine, üyelerine ve öteki projelerine erişemez.
- `tenant.allow_guests`, varsayılan açık: `kentosd tenant policy --slug … --guests on|off`. Kapatılınca misafirlerin paylaşımları hemen işlemez; erişim listesinde `guestsOff` nedeniyle görünürler. Açılınca yeniden işler. `kentosd tenant list` politikayı gösterir.
- Erişim listesi (`ProjectAccessHolder.guest`) misafiri ayrıca işaretler. Web onu “Misafir (davetle)” diye gösterir.
- Paylaşım penceresinin kişi araması değişmedi: kurum dışındaki kişiye yol davettir.

### Kimlik

- `app_user.email_verified` (migration 0009):
  - OpenID girişi sağlayıcının `email_verified`'ını yazar; yoksa ya da e-posta yoksa false;
  - yerel hesapların yöneticinin girdiği e-postası doğrulanmış sayılır; göç var olanları da işaretler. Kuralı veritabanı tutar (migration 0010, `app_user_local_email` tetikleyicisi): yerel hesabı yazan her yol, e-posta varsa doğrulanmış yazar.
- `kentos.resolve_identity` bu bilgiyle yeniden tanımlandı (altı değişken).

## Bu dilimde olmayanlar

- E-posta gönderimi (SMTP, yeni bağımlılık): bağlantıyı davet eden iletir.
- Davet önizlemesi (kabulden önce projenin adını göstermek), grup daveti, kurum üyeliğine davet.
- Web arayüzü (paylaşım penceresinde davet, bekleyenler, kabul sayfası) ve masaüstü: web tarafı web ajanına gider.
- Misafirin rol üst sınırının kurum politikasıyla daraltılması; misafirlerin toplu listesi.

## Doğrulama (26 Eylül 2026, Linux)

- `crates/server/application/tests/project_invitations.rs`, gerçek veritabanı:
  - misafir:
    - davet büyük harfli e-postayı küçültür; belirteç 64 hanedir;
    - kurum dışı kişi kabul edince misafir düzenleyici olur, projeye yazar;
    - kurumun listesine ve öteki projesine erişemez; “Projelerim”de yalnız o projeyi görür;
    - erişim listesinde misafir işaretlidir; belirteç ikinci kez çalışmaz; davet listesi kabul edeni gösterir;
    - kurum misafiri kapatınca erişim kesilir, liste `guestsOff` der; açınca döner;
  - e-posta ve durum: başka hesap (üye bile) kabul edemez; doğrulanmamış OpenID e-postası reddedilir. Yeni davet eskisinin bağlantısını geçersiz kılar; geri alınan, süresi dolan davet açılmaz ve listede öyle görünür. Kurum misafir almıyorsa kabul reddedilir; anlamsız belirteç 404;
  - denetimler: e-posta, yönetici rolü ve 90 günü aşan bitiş alan yoluyla reddedilir. Görüntüleyici davet edemez ve listeyi göremez. Yeniden deneme belirteçsiz saklı yanıtı alır;
  - üye ve kişisel alan: üye düz paylaşım alır; yönetici yapıldıktan sonraki görüntüleyici daveti rolünü düşürmez. Kişisel alanda herkes düz paylaşım alır; sahip kendi davetinde sahip kalır.
- `crates/server/application/tests/identity.rs`: `email_verified` yazılır ve güncellenir; `people.rs` birim testi misafir kurallarını SQL ile aynı sırada sınar.
- HTTP (`apps/api/src/http/invitations_tests.rs`): davetten önce yabancı projeyi ve davet listesini 404 bulur; komut yolundan davet, kabul, misafirin projeyi açması, ikinci kabulde 404, liste.
- Web: `sharing.test.ts`: misafirin kaynak metni ve `guestsOff` metni.
- Kasıtlı bozma, ikisi de geri alındı:
  - kabulde e-posta eşleşmesi kaldırılınca başka hesabın kabulü testi düştü;
  - `project_role`'de kurumun misafir kuralı yok sayılınca “misafir kapalı” testi düştü.
