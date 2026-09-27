# ADR 0111: Masaüstünde Projeyi paylaş: kişiler, roller ve davetler

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** CLAUDE.md §16; ADR 0015 (proje erişimi), 0024 (paylaşım penceresi), 0035 (davet ve misafir), 0042 (web'in davetleri), 0086 (masaüstünün kataloğu).
- **Kaynak:** web'in `ui/cloud/ShareDialog.ts`, `shareFind.ts`, `shareInvites.ts`; sözleri ve kuralları `app/cloud/sharePlan.ts`, `sharing.ts`, `invitations.ts`; ortak durumlar `fixtures/cloud/v1/share.json`.

## Bağlam

Masaüstünde bulut projesi açılıyor, kaydediliyor ve katalogda yönetiliyordu, ama paylaşılamıyordu. Kataloğun “Paylaş…” düğmesi kapalıydı: “Masaüstüne henüz taşınmadı”. Açık proje için `cloud.share` komutu da yoktu. Kimin hangi rolle eriştiğini görmek, birini eklemek ya da e-postayla davet etmek için web'e gitmek gerekiyordu.

## Karar

### Pencere

Pencere web'inkidir: 680 piksel, iki sekme.

- **Baş:** projenin adı ve çalışma alanı, sonra projenin nerede saklandığı.
- **Kişiler:**
  - “Kişi ekle”: ad ya da e-postayla arama, rol (varsayılan Düzenleyici) ve isteğe bağlı bitiş günü.
  - Rolün ne yaptığını anlatan satır.
  - Erişimi olanlar ve sayıları; her satırda harfler, ad, e-posta ve erişimin kaynağı, rol ve Kaldır ya da kilit.
  - Kurumun politikası.
- **Davetler:**
  - E-posta, rol (en çok Düzenleyici, varsayılan Görüntüleyici) ve geçerlilik (1–90 gün, varsayılan 14).
  - Yeni davetin bağlantısı.
  - Davetler: bekleyenler önce, sonra son 30 günde sonuçlananlar; her satırda durumu, bekleyende Geri al.
  - Davet edilenin ne alacağını anlatan not.
- **Durum satırı:** gövde kaysa da altta görünür kalır.

Sözler, sıralar, ipuçları, sorular, komut girdileri ve hataların söylenişi `cloud/share_plan.rs`'tedir. Bu, web'in `sharePlan.ts`'inin karşılığıdır. İki platform `fixtures/cloud/v1/share.json`'u oynatır (`share_plan_tests.rs`).

İki yerden açılır:

- **Kataloğun seçili projesinde “Paylaş…”:** planın izin verdiği projede açılır; kapalıysa `project.share` gerektiğini söyler. Pencere kataloğun üstünde durur, katalog altta kalır. Kapanınca katalog listesini yeniden ister, web'in `done`'u gibi.
- **`cloud.share` (“Bulut projesini paylaş…”):** açık proje için, tek başına (`Dialog::Share`). Web'in `openMay('project.share')`'i gibi yalnız bu yetkiyle, bağlantı varken ve proje bitmemişken çalışır.

### Sunucunun kararı

Pencere açılınca sunucuya iki şey sorar: kimin eriştiği (`GET …/access`) ve web uygulamasının adresi (`GET /v1/auth/config`). Liste gelmeden formlar kapalıdır. Liste gelince formlar açılır ve davetler istenir (`GET …/invitations`).

Liste `forbidden` ile reddedilirse paylaşım yetkisi yoktur. Hata listenin yerinde sunucunun sözleriyle söylenir; davetlerin yerinde yetki gerektiği yazar. Formlar kapalı kalır, kapalı alanlar sönük seçim kutusu gibi çizilir.

Her istek ürün komutudur ve kendi tekrar anahtarını taşır:

- `project.share`: ekleme ve rol değiştirme; rol değişince eski bitiş korunur.
- `project.access.revoke`
- `project.invite`
- `project.invitation.revoke`

Her yanıt pencerenin kimliğini taşır. Pencere kapandıktan sonra gelen yanıt hiçbir şeyi değiştirmez.

Başarı günlüğe yazılır, web'in `log.success`'i gibi. Ardından liste yeniden istenir. Kaldır ve Geri al önce sorar. Aynı adreste bekleyen davet varsa ya da kişi projeye zaten erişiyorsa, yeni davet de önce sorar.

### Kişi arama: KentOS UI `Suggest`

Web'in `shareFind.ts` alanı yeni bir KentOS UI bileşenidir: `widget::suggest`, öneri alanı.

- **Öneriler:** uygulamadan gelir. Liste alanın altında, alanın genişliğinde açılır; en çok altı satır görünür, gerisi tekerlek ve oklarla.
- **Tuşlar:**
  - ↑ ↓ gezinir.
  - Enter vurgulananı seçer; liste kapalıyken Paylaş'a basar.
  - Esc açık listeyi kapatır, pencereyi kapatmaz.
- **Fare:** tıklama seçer; odak alanda kalır.

Pencere yazılanı 250 ms dinlenince sunucuya sorar (`…/access/candidates?q=`), web'deki gibi. Yazı değişince eski arama durur. Başka sözcüklerin geç gelen yanıtı yok sayılır.

Kimse bulunamazsa listenin üstünde kurumun kuralı yazar. Yazılan bütün bir e-postaysa, altında “… adresine e-postayla davet gönder…” satırı çıkar. Bu satır Davetler'e geçer ve adresi alana yazar.

### Tek gösterimlik bağlantı

Web bağlantıyı kendi adresinden kurar. Masaüstünün kendi adresi yoksa, sunucunun söylediği web adresini kullanır.

Bu yüzden `AuthConfig`'e isteğe bağlı `publicUrl` eklendi: `KENTOS_PUBLIC_URL`, sonunda `/` ile. Adresi söylemeyen eski sunucuda bağlantı sunucunun kendi adresiyle kurulur.

Bağlantının davranışı:

- Yalnız pencerede gösterilir; satırlara bölünür, bütünü görünür.
- Kopyala onu sistem panosuna yazar.
- Günlüğe yalnız kimin hangi rolle davet edildiği yazılır, bağlantı yazılmaz.
- Yeniden deneme yanıtında bağlantı yoktur. Pencere o zaman nasıl yeni bağlantı alınacağını söyler.

## Sonuçlar

- `cloud.share` masaüstüne taşındı. Kataloğun “Paylaş…” düğmesi açıldı.
- Masaüstü davet bağlantısını açıp kabul etmez; kabul web'in `?davet=` sayfasındadır. Planın `accepted_how`'u ve alanların ekran okuyucu adları yalnız fixture'la sınanır.
- Grup paylaşımı ve e-posta gönderimi, web'de olduğu gibi, yoktur.

## Doğrulama

- **`cloud::share_plan_tests`:** `fixtures/cloud/v1/share.json`'un 8 bölümü (sözler, roller, satırlar, arama, davetler, tarihler, girdiler, hatalar).
- **`cloud::share_tests`:**
  - katalogdan açılış ve kişiler;
  - dinlenen yazıyla arama ve eski yanıtın yok sayılması;
  - seçilen kişiyle paylaşım ve günlük;
  - kimse bulunamayınca davet önerisi;
  - rol değiştirme; soruyla ve Esc'le Kaldır;
  - bağlantısıyla davet ve günlükte bağlantının olmaması; Kopyala; Geri al hatası;
  - yetkisiz pencere;
  - Esc sırası: soru, pencere, sonra katalog;
  - açık proje için `cloud.share`.
- **`widget::suggest::tests`:** vurgulanan satır görünür kalır.
- **Görüntüler:** `cloud::share_tests::screens` (`.run/shots/bulut-paylas-*`), web'in `share-*` resimleriyle karşılaştırıldı:
  - kişiler, arama listesi, davet önerisi, bağlantısıyla davetler, Kaldır sorusu, yetkisiz;
  - koyu ve açık tema, 1440×900 ve 1100×650.
- **Sunucu:** `apps/api` `native_tests` `publicUrl`'ü bekler.
- **Canlı:** `apps/desktop/scripts/cloud-live.sh` (`live_run.rs`, `share_live`) gerçek `kentosd` ile:
  - rol değiştirme, soruyla kaldırma;
  - “meh” yazılınca bulunan kişiyle yeniden paylaşma;
  - bağlantılı davet (belirteç 64 hane, günlükte yok) ve geri alma;
  - görüntüler `.run/shots/bulut-30…33-*`.
