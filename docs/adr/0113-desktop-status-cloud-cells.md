# ADR 0113: Masaüstünde durum çubuğunun bulut hücreleri ve hesap menüsü

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** CLAUDE.md §21.1; ADR 0038 (web'in dosya projeleri), 0041 (masaüstünün bulut arayüzü), 0043 (bağlantısız çalışma), 0058 (sunucu denetimi).
- **Kaynak:** web'in `ui/statusbar/cellsPlan.ts`, `cloudCells.ts`, `StatusBar.ts`; ortak durumlar `fixtures/cloud/v1/cells.json`.

## Bağlam

Masaüstünün durum çubuğunda bulut için kendi hücreleri vardı:

- projenin yeri,
- kendi sözleriyle kayıt durumu (“Kaydedildi”, “Kaydedilmedi (3)”, “Bağlantı yok — yeniden denenecek” …),
- bağlantı sözcüğü,
- hesabın adı ve Çıkış.

Web'de ise iki hücre var:

- **Kayıt hücresi:** kendi sözleri, lambası ve tıklanınca yaptığı iş.
- **Sunucu hücresi:** “Sunucu: bağlı/yok/uyumsuz”. Arkasında hesap menüsü açılır: kim girmiş, bulut komutları, açık projenin işleri (yetkisi olmayan iş hangi yetkinin gerektiğini söyler) ve sunucu denetimi.

Masaüstünde sunucu hücresi ve hesap menüsü yoktu. Sunucuya da yalnız istenince soruluyordu.

## Karar

Hücrelerin sözleri ve kararları `cloud/cells_plan.rs`'tedir. Bu, web'in `cellsPlan.ts`'inin karşılığıdır. İki platform `fixtures/cloud/v1/cells.json`'u oynatır (`cells_plan_tests.rs`). Çizim `cloud/cells.rs`'tedir.

### Kayıt hücresi

Yalnız açık bulut projesiyle görünür. Lambası ve rengi web'in `data-state` renkleridir: yeşil dolu “kaydedildi”, amber bekleyen iş, kırmızı çakışma ya da hata, soluk salt okunur. Tıklanınca sıradaki işi yapar: çakışmada Çakışmaları çöz, aksi hâlde Kaydet. Salt okunurken ya da Kaydet sürerken bir şey yapmaz.

- **Veritabanı projesi:** web'in sözleri, bekleyen sayısıyla (“Kaydedilecek: 3”, “Çevrimdışı: 2 bekliyor”, “Çakışma: 1”). İpucu projenin yerini, son kaydın ne zaman olduğunu, canlı bağlantıyı ve hatayı söyler. Hata hücrede “Kayıt hatası”, nedeni ipucundadır.
- **Dosya projesi:** Kaydet'in aşamaları ayrı ayrı: “Dosya hazırlanıyor…”, “Yükleniyor %42”, “Sunucu doğruluyor…”. Sonra “Buluta kaydedildi · r4”. Başarısız kayıt (hata, proje silinmiş, erişim kaldırılmış) bir sonraki Kaydet'e kadar hücrede kalır. İpucu çizimin dayandığı revizyonu ve bu pencerenin son kaydını söyler.

### Masaüstüne özgü durumlar

Masaüstü bağlantısız da çalışır (ADR 0043); web bunu hiç yapmaz. Bu yüzden:

- Oturum yokken veritabanı projesinin işi cihaz taslağında bekler. Hücre bunu web'in çevrimdışı bekleyen iş sözüyle söyler, ipucu “oturum gerekli” der.
- Bu cihazda saklanıp gönderilmeyi bekleyen dosya kaydı “Kaydedildi (bu cihazda) · gönderilecek” der.

### Sunucu hücresi ve hesap menüsü

Hücre web'in sözlerini ve ipucunu kullanır. İpucu sunucunun yapımını, sözleşme sürümünü ve yoksa nedenini söyler. Geliştirme yapımında “pnpm api” ile başlatmayı da söyler.

Sunucuya program açılınca bir kez, sessizce sorulur: yanıtı hücre söyler, günlük söylemez. `server.check` eskisi gibi günlüğe yazar. Yeni soru sürerken son yanıt hücrede kalır.

Menünün satırları web'in `accountRows`'udur. Satırlar masaüstünün komut satırı olarak çizilir: başlık, ikon, kısayol; çalışamayan satır soluktur. Açık projede yetkisi olmayan işin altında hangi yetkinin gerektiği ve kime başvurulacağı yazar.

Eski hücreler kaldırıldı: projenin yeri, bağlantı sözcüğü, hesabın adı ve Çıkış. Projenin yeri ipucundadır; hesap ve Çıkış menüdedir. Uygulama menüsünün bulut kartı da web'inki gibi kayıt hücresinin ve canlı bağlantının sözlerini yazar.

## Sonuçlar

- Masaüstünün eski kayıt sözleri (`words::save_state`) kalktı. Testler web'in sözleriyle yazıldı.
- Kalan farklar:
  - masaüstü dosya projesinin olaylarını izlemediği için “Yeni revizyon” durumu yoktur;
  - web'in son iletiyi birkaç saniye gösteren durum çubuğu hücresi yoktur;
  - web'in soket durumları (bağlanıyor, yeniden bağlanıyor) masaüstünde yoktur (ADR 0044).

## Doğrulama

- **`cloud::cells_plan_tests`:** `cells.json`'un bütün bölümleri:
  - sözler, görünüşler ve tıklamalar;
  - “… önce”;
  - iki projenin ipuçları;
  - sunucunun sözleri ve ipuçları;
  - projenin işleri ve hesap menüsünün satırları.
- **`cloud::cells_tests`:**
  - dosya projesinin Kaydet aşamaları;
  - başarısız kaydın kalması;
  - veritabanı projesinin ipucu ve bekleyen işi;
  - sunucunun yanıtları ve sessiz ilk soru;
  - görüntüleyicinin menüsündeki yetki satırları.
- **`cloud::tests`:** eski kayıt sözlerini bekleyen durumlar web'in sözlerine geçti.
- **Görüntüler:** `cloud::cells_tests::screens` (`.run/shots/bulut-hucre-*`), web'in `status-*` resimleriyle karşılaştırıldı:
  - veritabanı projesi kaydederken, dosya projesi yüklerken, sunucu yokken, hesap menüsü;
  - koyu ve açık tema, 1440×900 ve 1100×650.
