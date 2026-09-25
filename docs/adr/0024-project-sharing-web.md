# ADR 0024: Web'de proje paylaşımı, “Benimle paylaşılanlar” ve açık projede erişim değişikliği

- **Durum:** kabul edildi (2026-09-26). Yön ADR 0015'ten (dilim 4'ün web tarafı) ve TODOS.md `CLOUD-04`, `CLOUD-13`, `CLOUD-16`, `CLOUD-19`, `CLOUD-21`'den gelir. Kişi bulmanın kapsamı, erişim listesinin biçimi, açık projede rol değişince autosave'in davranışı ve erişim kaldırılınca olanlar bu dilimin kararıdır.
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** CLAUDE.md §16, §21; TODOS.md §12.1–12.3; ADR 0007 (kimlik), 0013 (ürün komutları), 0015 (proje sahipliği ve erişim)

## Bağlam

- ADR 0015'in uygulama notuna göre sunucuda paylaşım vardı: `project.share` ve `project.access.revoke` komutları, `GET …/access` (sahip ve paylaşımlar), `project.access` olayı ve erişimi kalkan WebSocket aboneliğine `not_found`. Web'in paylaşım ekranı yoktu; kişi yalnız hesap kimliğiyle (`UserView.id`) seçilebiliyordu.
- Web açık projede yalnız silinmeyi biliyordu (`project.deleted`, 410). Rolü düşürülen ya da erişimi kaldırılan kişinin editörü her kayıtta hata alıyor, ne olduğunu söylemiyordu.
- İlk paylaşım dilimi kimliği doğrulanmış alıcılarla çalışır (`CLOUD-19`). E-postayla davet ve kurum dışı misafir sonraki dilimlerdir (`CLOUD-16`, `CLOUD-17`).

## Karar

### Paylaşım penceresinin sunucu desteği

- **Erişim listesi** (`GET …/projects/{proje}/access`, `ProjectAccessList`) artık kişi kişi yazılır (`people`):
  - kimler: proje sahibi; kurumun politikası açıksa ve bugün geçerliyse kurum sahipleri ve yöneticileri; her paylaşım, süresi dolanlar da;
  - her kişi için bugünkü rolü ve kaynağı (`role`, `via`: sahip, paylaşım, politika) ya da neden erişemediği (`blocked`: süresi doldu, kurumun üyesi değil, hesabı ya da üyeliği etkin değil, koltuğu yok);
  - paylaşımı (`grant`), bitişi ve bitişin geçip geçmediği;
  - listenin kendisinde alan türü, saklama biçimi (`storage`) ve kurum politikası.

  Eski `grants` alanı kalktı; listeyi yalnız web kullanıyordu. Liste, ADR 0015'teki gibi yalnız `project.share` iznine açıktır.
- **Bugünkü rol Rust'ta hesaplanır** (`people::standing`). Sıra `kentos.project_role` ile aynıdır: kurumda önce etkin üyelik ve koltuk, sonra sahiplik, politika, süresi geçmemiş paylaşım; kişisel alanda alanın kişisi sahiptir, öbürleri paylaşımla girer.
  - Veritabanı işlevi başkasının rolünü hesaplamaz (yalnız o anki kullanıcınınkini). Bunu yapan yeni bir `security definer` işlev migration 0005 isterdi.
  - Bunun yerine `crates/server/application/tests/people.rs` listedeki her kişi türü için rolü, o kişinin projeyi açtığında `access::project`'ten aldığı sonuçla karşılaştırır. İki kopya ayrışırsa test düşer.
  - Liste yalnız gösterir; erişim kararını her istekte yine `kentos.project_role` verir.
- **Saklama biçimi** `ProjectStorage` sözleşmesidir. Bugün tek değeri `database`'dir: nesneler sunucudaki PostGIS'te tek tek, her kayıt tek işlemde. Dosya tabanlı (binary `.kcad` revizyonları) ve dış PostGIS projeleri kendi değerleriyle gelir. Ürün komutu kataloğu değişmedi.
- **Kişi bulma** (`GET …/access/candidates?q=`, `ShareCandidates`), yalnız arayanın görebildiği ve paylaşabildiği kişiler arasında:
  - kurum projesinde kurumun etkin üyeleri (etkin hesap, etkin üyelik);
  - kişisel alanın projesinde arayanın kendi kurumlarının etkin üyeleri; yalnız arayanın etkin üyeliği ve koltuğu olan kurumlarda aranır;
  - başka bir kurumun üyesi ya da arayanla hiçbir kurumu paylaşmayan hesap bulunmaz. Her sorgu satır güvenliğiyle o kurumun kapsamında çalışır.

  Eşleşme: sorgunun her sözcüğü adda ya da e-postada geçer; en az iki harf, en çok 100 karakter; en çok 20 sonuç, ada göre. Türkçe harfler Rust'ta ve SQL'de (`translate`) aynı tabloyla katlanır (“ayse” “Ayşe”yi, “isik” “IŞIK”ı bulur). Sonuç veritabanının yerel ayarına bağlı değildir. `%` ve `_` harf sayılır. Arayan ve sahip listeden çıkar. Zaten paylaşılan kişi çıkmaz: yeniden paylaşmak rolünü değiştirir.
- **Neden genel e-posta/giriş adı araması yok:** herhangi bir e-postanın hesabı olup olmadığını ve kişinin adını, onunla hiçbir ortaklığı olmayana söylerdi (hesap sayımı). Giriş adları sunucu rolüne zaten kapalıdır (`local_credential`, ADR 0007). Kurum dışından birini eklemenin yolu e-postayla davettir (aşağıda).
- Negatif testler (`KENTOS_TEST_DB=required`):
  - yeni uçların 404 gövdesi, var olmayan projeninkiyle aynıdır;
  - başka kurumun üyesi ve tahmin edilen ya da bozuk kimlik aynı yanıtı alır;
  - paylaşamayan 403 alır;
  - arama başka kuruma ulaşmaz;
  - HTTP üzerinden kişi kendi rolünü yükseltemez, sahibin erişimine dokunamaz, sahiplik veremez, kurum dışına paylaşamaz.

### Web

- **Paylaşım penceresi** (`ui/cloud/ShareDialog.ts`), `cloud.share` komutuyla ve proje listesindeki “Paylaş…” düğmesiyle açılır.
  - Komut ve düğme `project.share` yoksa devre dışıdır ve nedenini söyler. Denetimler, liste sunucudan geldikten sonra açılır; bir ret sunucunun sözleriyle, neden ve çözümüyle yazılır.
  - Kendi erişim satırında ve sahibin satırında denetim yoktur. Politikadan gelen erişim değişmez; yanındaki paylaşım kaldırılabilir. Süresi dolan paylaşımın rolü değiştirilmez, yeniden paylaşılır; değiştirmek bitişsiz geri verirdi.
  - Kaldırmadan önce `askRemove` sorar ve indirilmiş kopyaların geri alınamayacağını söyler.
- **“Benimle paylaşılanlar”:** “Bulut projesi aç” penceresinin ikinci sekmesi. `GET /v1/me/projects`'in paylaşımla gelen projeleridir (`via = grant`), en yeni önce, sahibinin adı (`ProjectSummary.ownerName`) ve kişinin rolüyle. Kişinin kendi projeleri ve yalnız politikayla eriştikleri kendi listelerinde kalır.
- **Açık projede erişim değişince** (`app/cloud/accessWatch.ts`):
  - Bir `project.access` olayı, bir 403 ya da WebSocket'in `not_found`'u gelince proje bilgisi yeniden okunur. Düğmeler yeni izinlere uyar.
  - **Rol düşünce** (yazma izni gitti): gönderim durur. Bekleyenler ve bundan sonraki düzenlemeler cihaz taslağında tutulur, durum “Salt okunur” olur. Yetki dönünce gönderilir.
  - **Görüntüleyiciyken yapılan düzenlemeler** hiç saklanmamıştı (söylenmişti). Yetki gelince de gönderilmez; kullanıcıya projeyi yeniden açması söylenir. Çizim kendiliğinden yeniden yüklenmez: ekrandaki çalışma habersiz değişmesin diye.
  - **Erişim kalkınca** (404, ya da kurumdaki üyelik kullanılamıyor: 403) proje silinmiş gibi durur:
    - kayıt hücresi “Erişim kaldırıldı” der, canlı bağlantı kapanır, projeye ait eylemler kapanır;
    - çizim ve gönderilmemiş değişiklikler cihazda kalır;
    - reddedilen komut anahtarıyla taslakta kalır. Sunucu erişimi idempotency kaydından önce sorduğu için önceki denemenin yazılıp yazılmadığı bilinemez; aynı anahtar, proje yeniden paylaşılıp açılınca komutun bir kez yazılmasını sağlar;
    - bir bildirim (`confirmDialog`) ne olduğunu ve neyin kaldığını söyler, yerel kopya kaydetmeyi önerir. Kaydet ya da Farklı kaydet yerel dosyaya yazar ve projeden çıkar.
- Sınandı: vitest (`sharing`, `api`, `sync`, `accessWatch`), `pnpm e2e:cloud`: pencereden paylaşım, rol değişikliğinin etkisi, alıcının açık editöründe düşürülen ve yükseltilen rol, kaldırılan erişim, konsol hatası yok.

### Bu dilimde olmayan: e-postayla davet

- Kişiyle hiçbir kurumu paylaşmayan bir hesap (ya da hesabı olmayan kişi) bugün paylaşım penceresinden eklenemez. Önerilen tasarım ayrı bir ADR ve migration ister:
  - `project_invitation` tablosu: davet eden, proje, küçük harfli e-posta, rol, bitiş, token'ın yalnız SHA-256'sı, durum (bekliyor, kabul, iptal, süresi doldu) ve kabul eden hesap;
  - token tek kullanımlık ve kısa ömürlüdür. Yalnız o e-postası doğrulanmış hesapla kabul edilir. Yerel hesapların e-postası bugün doğrulanmıyor; OpenID'de sağlayıcının `email_verified`'ı gerekir;
  - kurum projesinde kabul, kurum üyeliği mi yoksa misafirlik mi (`CLOUD-17`), kurum politikasına bağlıdır;
  - e-posta gönderme altyapısı yok; davet bağlantısı ilk sürümde paylaşan kişiye gösterilebilir.
- Hesap sayımını önlemek için davet yanıtı, e-postanın bir hesabı olsa da olmasa da aynıdır.

## Sonuçlar

- `ProjectAccessList` biçimi değişti (`grants` yerine `people`). Sözleşme sürümü aynı kaldı; bu uç yalnız web'in kullandığı yeni bir uçtu. `ProjectSummary`'ye `ownerName`, yeni tipler `ProjectAccessHolder`, `AccessBlock`, `ProjectStorage`, `ShareCandidate(s)`.
- Paylaşım bilgisi yalnız paylaşabilene açıktır; görüntüleyici ve düzenleyici kimlerin erişebildiğini göremez (ADR 0015'in kuralı; açık soru).
- Kişisel alanın sahibi, kurumlarından biriyle ortak olmadığı kişiyi bugün arayarak bulamaz. Bunun yolu davettir.
- Masaüstünde bulut yok; masaüstü aynı uçları ve kuralları kullanacak (`CLOUD-21`).
- Açık sorular:
  - ortak kurumu olmayan kişiye paylaşımın yolu (davet mi, e-postayla tam eşleşme mi);
  - erişim listesinin görüntüleyici ve düzenleyiciye açılıp açılmayacağı.
