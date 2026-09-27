# ADR 0073: Masaüstünde açık bulut projesinin işlemleri: yeniden adlandır, çöpe taşı, son revizyon, dosya olarak yükle

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** CLAUDE.md §13.1, §16, §21.3; ADR 0028 (proje kataloğu ve yaşam döngüsü), 0038 (web'de dosya projeleri), 0040–0044 (masaüstünün bulutu)
- **Kaynak:** web'in `app/cloud/commands.ts`'i, `ui/cloud/ProjectActions.ts` (`openRenameDialog`, `trashProject`) ve `FileConflict.ts` (`offerNewest`; kaydedilmemiş iş sorusu web ajanının `4b393ae`'sinde düzeltildi).

## Bağlam

- Masaüstünde bulut komutlarının altısı "web'de var; masaüstüne henüz taşınmadı" diyordu: `cloud.uploadFile`, `cloud.openNewest`, `cloud.rename`, `cloud.delete`, `cloud.history`, `cloud.share`.
- Masaüstünün bulut istemcisi (`kentos-cloud`) genel komut çağrısını, çöp kutusu komutunu ve saklama biçimi seçen yükleme penceresini zaten taşıyordu. Dosya projesinde "son revizyonu aç" akışı da vardı.
- Bu ADR ilk dördünü taşır. Geçmiş ve paylaşım ayrı dilimlerdir.

## Karar

### Ne zaman açık (web'in `openMay`'i)

- Açık bir bulut projesi olmalı ve gönderimi bitmemiş olmalı: silinmiş, arşivlenmiş ya da erişimi kaldırılmış proje sayılmaz.
- Hesabın projede gereken izni olmalı: adlandırmada `project.edit`, çöpe taşımada `project.delete`, son revizyonda `project.read`.
- Oturum açık, sunucu erişilebilir olmalı.
- Adlandırma projeyi değiştirdiği için arşivlenmiş projede kapalıdır.
- Son revizyonu aç yalnız dosya projesinindir.

### Buluta dosya olarak kaydet (`cloud.uploadFile`)

- "Buluta yükle" penceresidir; saklama "Dosya (her kayıt bir revizyon)" seçili açılır (web'in `openUploadDialog({storage: 'file'})`'i).
- Oturum yoksa önce giriş istenir, sonra pencere aynı saklamayla açılır.

### Bulut projesini yeniden adlandır (`cloud.rename`)

- **Pencere** (460 px): "Yeni ad" alanı, projenin adı seçili. Altında "Projeye erişimi olan herkes yeni adı görür." Düğmeler: Vazgeç, "Yeniden adlandır".
- Düğme ad boşken ya da aynıyken kapalıdır; Enter de gönderir.
- İstek sürerken "Kaydediliyor…" yazar.
- **Veritabanı projesi:** ad, otomatik kaydın bekleyen öteki değişiklikleriyle birlikte gider; böylece kendi kendisiyle çakışmaz (web'in `flush`'ı).
  - Sunucu adı alınca pencere kapanır: "Proje “X” olarak yeniden adlandırıldı."
  - Gönderim durursa ya da bağlantı yoksa: "Yeni ad (“X”) bu cihazda bekliyor; sunucuya ulaşılınca kaydedilir."
- **Dosya projesi:** katalogun `project.rename` komutuyla. Çizim adı düzenleme saymadan alır (`apply_external`).
- **Ret** pencerede, web'in `reason` sözleriyle söylenir; çakışma ve bağlantı için kendi cümleleri, öbürlerinde sunucunun iletisi.

### Çöp kutusuna taşı (`cloud.delete`)

- **Soru** (web'in `askRemove`'u): "“X” projesi (çalışma alanı) erişimi olan herkes için çöp kutusuna taşınsın mı?" Altında web'in dört açıklaması, kırmızı "Çöpe taşı" ve Vazgeç.
- **Önce gönderim:** veritabanı projesinde bekleyen iş önce gönderilir (web'in `settle`'ı). Gönderilemese de çöp kutusu isteği gider.
- **Sonrası:** proje çizimden ayrılır; çizim ekranda yerel çizim olarak kalır. İleti: "“X” bulut projesi çöp kutusuna taşındı (27.10.2026 tarihine kadar geri yüklenebilir). Çizim ekranda kaldı; saklamak için Dosya → Farklı kaydet ile yerel bir dosyaya kaydedin." Tarih sunucunun `purgeAfter`'ıdır.
- **Ret:** "“X” çöp kutusuna taşınamadı: neden".

### Son revizyonu aç (`cloud.openNewest`, dosya projesi)

- **Reddedilmiş bir Kaydet varsa:** onun sorusu açılır (ayrı kopya, yerel dosya, son revizyon).
- **Kaydedilmemiş iş varsa:** web'in `4b393ae`'deki sorusu.
  - Başlık "Kaydedilmemiş değişiklikler".
  - İleti: "“X” içinde kaydedilmemiş değişiklikler var. Sunucudaki en yeni revizyon açılırsa bu değişiklikler atılır (açık çizim revizyon N). Saklamak için önce Kaydet ile kaydedin."
  - Düğmeler "Kaydetmeden aç" ve Vazgeç. "Kaydet ve aç" yoktur: önce kaydetmek yalnız kendi kaydını yeniden açardı.
  - "Kaydetmeden aç" kurtarma kopyasını da atar.
- **Temiz çizimde:** "“X” projesinin sunucudaki en yeni revizyonu açılsın mı? Açık çizim revizyon N; kaydedilmemiş değişikliği yok." Düğmeler "Son revizyonu aç" ve "Sonra".

## Web'den ayrılanlar

- **Yeni revizyon bildirimi:** masaüstü dosya projesinde başkasının yeni revizyonunu izlemiyor. "Başka bir yerde kaydedildi: revizyon N (kişi)" biçimi bu yüzden yok; soru hep genel biçimdedir.
- **Tarih:** ileti sunucunun gününü UTC'ye göre yazar; web tarayıcının yerel saatine göre yazar.
- **Ad sınırı:** alan UTF-16 birimiyle 200'ü aşan adı "Proje adı boş olamaz ve en çok 200 karakter olabilir." diye reddeder. Web'de bu sınırı alanın `maxlength`'i koyar; sunucu 200 baytı denetler.

## Doğrulama

- **`cloud::tests`** (5 yeni):
  - dosya olarak yükleme penceresi;
  - dosya projesinin adlandırılması: aynı ad gönderilmez, çakışma reddi pencerede kalır, başarı çizimi kirletmez;
  - veritabanı projesinin adlandırılması: ad otomatik kayıtla gider, yanıtla söylenir; bağlantı yokken bekleme iletisi;
  - çöpe taşıma: izin, soru, çizimin yerel kalması, geri yükleme tarihi;
  - son revizyonun iki sorusu ve kaydedilmemiş işte yeniden açılış; veritabanı projesinde komut kapalı.
- **`apps/desktop/scripts/cloud-live.sh`:** gerçek `kentosd` ve geliştirme veritabanı. Üç yeni adım (13–15):
  - çakışmanın "Son revizyonu aç"ı ve kaydedilmemiş işte komutun sorusu, iki kez revizyon 3'ün açılması;
  - `project.rename` ile adlandırma;
  - çöpe taşıma; sunucunun `purgeAfter`'ı iletide.
  - Görüntüler: `.run/shots/bulut-20-son-revizyon`, `-21-yeniden-adlandir`, `-22-cope-tasi`.
- **Görüntüler** (`cloud::tests::action_screens`, `.run/shots/bulut-{yeniden-adlandir,cope-tasi,son-revizyon,son-revizyon-kaydedilmemis}-*`): koyu ve açık, 1440×900 ve 1100×650.
- `pnpm rust:test`, `pnpm rust:test:desktop`, `pnpm typecheck`, `pnpm test`, `pnpm build`, `pnpm e2e`, `pnpm e2e:interaction`, `pnpm inventory:check`, `KENTOS_E2E_DB=scratch pnpm e2e:cloud`.
