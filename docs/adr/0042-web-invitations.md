# ADR 0042: Web'de e-postayla davet, tek gösterimlik bağlantı ve kabul sayfası

- **Durum:** kabul edildi (2026-09-26). Sunucu tarafı ADR 0035'tir (`project.invite` ve `project.invitation.revoke` v1, `GET …/invitations`, `POST /v1/invitations/accept`); bu karar onun web arayüzüdür. Yön TODOS.md `CLOUD-16` ve `CLOUD-17`'den gelir.
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** CLAUDE.md §16, §20; DESIGN.md §7.7, §7.9.1; ADR 0015, 0024, 0035, 0038

## Bağlam

- Sunucu, kurum dışından birini bağlantıyla davet etmeyi ve misafiri biliyordu; web bilmiyordu.
- Paylaşım penceresi kişiyi yalnız kurumun üyeleri arasında bulur. Kurum projesi üye olmayanla `project.share` ile paylaşılamaz; misafirin rolü de bu yüzden paylaşımla değişmez.
- Bağlantının belirteci tek kullanımlık bir sırdır. Sunucu yalnız özetini saklar ve belirteci yalnız ilk yanıtta verir. Sahibin kararı: web yalnız çevrimiçi çalışır; service worker ve çevrimdışı proje kopyası yoktur.

## Karar

### Paylaşım penceresi: iki sekme

- Pencere iki sekmedir: **Kişiler** (kurum üyesi ekleme ve erişimi olanlar) ve **Davetler**. Her sekmenin tek amber düğmesi vardır (DESIGN.md §7.9).
- “Kişi ekle”de kimse bulunmazsa liste nedenini söyler. Yazılan tam bir e-postaysa “… adresine e-postayla davet gönder…” satırı çıkar; tıklanınca Davetler sekmesi o adresle açılır.
- Kurum dışından biri erişim listesinde “Misafir (davetle)” diye görünür. Misafirin rolü seçilemez: kurum projesi yalnız üyeleriyle paylaşılır. İpucu, rolün yeniden davetle değiştiğini söyler. Erişimi “Kaldır” ile kalkar; soru, erişimin yeniden davetle geri geldiğini söyler.
- Pencerenin alt notu kurum dışına yolun davet olduğunu söyler.

### Davet

- Davetler sekmesindeki form:
  - e-posta;
  - rol: Görüntüleyici, Yorumcu ya da Düzenleyici; varsayılan **Görüntüleyici** (yöneticilik davetle verilmez);
  - geçerlilik: 1, 3, 7, **14 (varsayılan)**, 30, 60 ya da 90 gün.
- 14 gün seçiliyken `expiresAt` gönderilmez; sunucu kendi 14 gününü koyar. 90 gün, sınırın 10 dakika içinde gönderilir: sunucudan birkaç dakika ileri bir saat de reddedilmez.
- E-posta, sunucunun kuralıyla önceden denetlenir; kararı yine sunucu verir. Sunucunun reddi alan yoluyla ve kendi sözleriyle söylenir.
- Gönderilmeden önce sorulur:
  - aynı adrese bekleyen davet varsa: yenisi onu geri alır ve eski bağlantı çalışmaz;
  - adres projeye zaten erişebiliyorsa: davet ancak daha güçlü bir rol verir.

### Tek gösterimlik bağlantı

- Başarılı davetin bağlantısı (`…/?davet=<belirteç>`) Davetler sekmesinde bir kez gösterilir; “Kopyala” düğmesi ve şu uyarı vardır: sunucu saklamaz, pencere kapanınca yeniden gösterilemez, bir kez kullanılır, kaybedilirse yeniden davet edilir.
- Bağlantı yalnız o alanın içindedir. Günlüğe yalnız kimin hangi rolle davet edildiği yazılır. Tarayıcının hiçbir deposuna yazılmaz. Pencere yeniden açılınca liste davetleri gösterir, bağlantıyı göstermez.
- Yanıtta belirteç yoksa (yeniden denemenin saklı yanıtı) bu söylenir ve yeniden davet önerilir.

### Davetler listesi

- Bekleyenler önce, sonra son 30 günün sonuçlanmışları gelir; her grup en yeni önce.
- Her satırda adres, rol, davet eden ve tarihi, ayrıca bitiş (bekleyende), kabul eden (kabul edilende) ya da sonuç (geri alınan, süresi dolan) yazar. Durum bir çip olarak görünür.
- Bekleyen davet “Geri al” ile, `confirmDialog` sorusundan sonra geri alınır.
- Liste ve davet `project.share` ister. Yetkisi olmayan hesapta form kapalıdır ve nedenini söyler.

### Bağlantının açılışı

- Belirteç, sayfa başlar başlamaz adresten alınır (`main.ts`, `history.replaceState`); bu, başka hiçbir şey istenmeden önce olur.
- Belirteç yalnız bu sekmede tutulur: bellekte ve sessionStorage'da (`kentos.invitation`). localStorage'a hiç yazılmaz. Böylece sayfadan ayrılan bir giriş (OpenID) sırasında da kalır.
- Sayfa yalnız kendi kaynağını gönderen bir referrer ilkesiyle açılır (`<meta name="referrer" content="strict-origin">`): bir sonraki istekte de bağlantı sızmaz.
- Bağlantıyla açılan sayfada başlangıç ekranı yerine **Projeye davet** penceresi açılır. Açılışın kurtarma sorusunu kapatmaz.
- Pencerenin durumları:
  - sunucu yanıtlayana kadar bekler;
  - oturum yoksa “Giriş yap” der: davetin gönderildiği adresin hesabı gerekir;
  - oturum varsa daveti **bir kez** kabul eder;
  - kabulden sonra projeyi, çalışma alanını, rolü ve misafir ya da üye olduğunu gösterir. “Projeyi aç” projeyi ilerlemeyle açar.
- Sahibin kendi daveti “sizin projeniz” der; bir şey değişmez.
- Ret sunucunun sözleriyledir; her sonucun kendi iletisi vardır: yok, başka e-posta, doğrulanmamış e-posta, silinmiş proje ya da etkin olmayan kurum, etkin olmayan üyelik, misafir almayan kurum.
- Belirteç ancak başka bir hesabın aşabileceği retlerde (403) kalır. Pencerede giriş yapan hesap yazar ve “Başka hesapla giriş yap” vardır: oturum kapanır, giriş penceresi açılır, giriş olunca kabul yeniden denenir. Öbür yanıtlarda belirteç unutulur.
- Pencere kapanınca belirteç unutulur. Bağlantının kendisi, kullanılana, geri alınana ya da süresi dolana kadar çalışır.

### Küçük düzeltmeler (ADR 0038'in ardından)

- Açılış saklama biçimini projenin kendi bilgisinden alır (`ProjectInfo.storage`). Katalog da kendi kaydındakini kullanır. `storage` parametreleri ve `HistoryPanel.storageOf` kalktı.
- `lifecycle.convert` üretilen `ProjectConvert` tipini kullanır.
- Paylaşım penceresinin kişi bulucusu kendi dosyasına ayrıldı (`shareFind.ts`). `CatalogDialog.ts` ve `session.ts`'te doğal bir dikiş görülmedi; bölünmediler.

### Kod yerleşimi

| Parça | Yer |
|---|---|
| Bağlantı: adresten alma, sekmede tutma | `app/cloud/invitationLink.ts`, `main.ts` |
| Kurallar, komutlar, metinler | `app/cloud/invitations.ts` |
| Kabulün durumları | `app/cloud/acceptance.ts` |
| API | `app/cloud/api.ts` (`invitations`, `acceptInvitation`) |
| Pencereler | `ui/cloud/ShareDialog.ts`, `ui/cloud/shareInvites.ts`, `ui/cloud/shareFind.ts`, `ui/cloud/InvitationDialog.ts`, `styles/invite.css` |
| Sahte sunucu | `app/cloud/fakeInvites.ts` |

## Sahibe sorular ve varsayılanlar

Aşağıdakiler karar verilene kadar önerilen varsayılanla çalışır:

1. **Davetin varsayılan rolü.** Varsayılan Görüntüleyici (önerilen): kurum dışından biri en az yetkiyle başlar. Seçenek: paylaşım satırındaki gibi Düzenleyici.
2. **Kabulün zamanı.** Bağlantı açılınca, giriş varsa, kendiliğinden kabul edilir (önerilen): bağlantıyı açmak niyettir ve kabulü yalnız davetteki adresin doğrulanmış hesabı yapabilir. Seçenek: önce bir “Kabul et” düğmesi. Sunucu önizleme vermediği için o düğme projenin adını söyleyemez.
3. **Geçerlilik seçimi.** 1–90 gün arasında yedi hazır süre (önerilen): tarih alanının 90. gün sınırındaki saat sorunu yoktur. Seçenek: paylaşım satırındaki gibi tarih alanı.

## Bu adımda olmayanlar

- E-posta gönderimi: bağlantıyı davet eden iletir (ADR 0035).
- Kabulden önce projenin adını gösteren önizleme; sunucuda yoktur.
- Masaüstünde davet ve kabul.

## Doğrulama (26 Eylül 2026, Linux)

- Vitest:
  - `invitations.test.ts`:
    - bağlantı adresten alınır; öbür parametreler ve hash kalır; belirteç sekmede tutulur, depo reddedilince bellekte kalır;
    - e-posta kuralı, roller, geçerlilik, sorular, liste sırası ve metinleri;
    - belirteç yalnız ilk yanıttadır; aynı adrese yeni davet eskisinin bağlantısını geçersiz kılar; geri alınan bağlantı çalışmaz; yetki;
    - kabul: girişten sonra bir kez; başka adres ve öbür bütün sonuçlar kendi iletileriyle; yalnız 403'te belirteç kalır; sahip ve güçlü rol; sunucuya ulaşılamaması ve yeniden deneme; biten oturum.
  - `sharing.test.ts`: misafir satırı.
- `KENTOS_E2E_DB=scratch pnpm e2e:cloud`: 98 denetimin hepsi geçti. Önceki 83 denetimin ardından davet adımları, gerçek sunucuyla (`scripts/e2e/cloud-invite.mjs`):
  - kurum dışından, e-postalı bir hesap yönetim CLI'siyle açılır;
  - üye bulunmayan adres davet olarak önerilir; form roller ve 14 günle açılır;
  - bağlantı uygulamanın adresidir ve bir kez gösterilir; belirteç günlükte, tarayıcı depolarında ve sunucunun listesinde yoktur; pencere yeniden açılınca gösterilmez;
  - oturumsuz açılış: belirteç adresten hemen kalkar, yalnız sessionStorage'da durur;
  - Mehmet'in hesabı başka adresin daveti diye reddedilir; belirteç kalır;
  - “Başka hesapla giriş yap”tan sonra misafir kabul eder; proje, alan, rol ve misafirlik yazar; projeyi açar, düzenler ve sahip düzenlemeyi görür;
  - kullanılan bağlantı 404'tür ve unutulur;
  - bekleyen davete yeniden davet sorulur; yenilenen ve geri alınan davetin bağlantıları 404'tür;
  - misafir erişim listesinde doğru okunur; erişimi kaldırılınca proje ona 404'tür.

## Bilerek bozma (26 Eylül)

Her kural kaynakta bilerek bozuldu, adı geçen denetim düştü, kaynak geri alındı ve ağaç temiz kaldı (`853d165` üzerinde).

| Kural | Bozma | Düşen denetim |
|---|---|---|
| belirteç adresten hemen alınır | `replaceState` yok | `is taken off the address at once…`, `leaves an address without one alone…` |
| belirteç sayfadan ayrılan girişte de kalır | sessionStorage'a yazılmaz | `is taken off the address at once and kept for this tab only…` |
| yalnız 403 belirteci tutar | her ret tutar | `each refusal says its own reason; only a 403 keeps the token` |
| davet bir kez kabul edilir | kabulden sonra yeni giriş yeniden gönderir | `…then accepts once…` |
| biten oturum girişi sorar, belirteç kalır | 401 ret sayılır | `…a session that ended asks to sign in` |
| yanıt gelmezse yeniden denenir | ağ hatası ret sayılır, belirteç unutulur | `no answer: said, and tried again…` |
| davet en çok düzenleyici rolü verir | Yönetici de sunulur | `takes the e-mails the server takes, and no manager` |
| en uzun bekleme 90 günün içinde gönderilir | tam 90 gün gönderilir | `…the longest inside 90` |
| bekleyen davetin yerine yenisi sorulmadan gönderilmez | bekleyen davet aranmaz | `asks before replacing a waiting invitation…` |
| misafirin rolü paylaşımla değiştirilmez | misafir satırında rol seçilir | `a guest who accepted an invitation reads as one…` |
| e-posta sunucunun kuralıyla denetlenir | alan adında nokta aranmaz | `takes the e-mails the server takes…` |
| bağlantı hiçbir yerde tutulmaz | bağlantı günlüğe yazılır | e2e: `the token is kept nowhere: not in the log…` (98 denetimden yalnız o düştü) |
