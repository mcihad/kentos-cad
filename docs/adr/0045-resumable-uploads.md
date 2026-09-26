# ADR 0045: Kaldığı yerden süren yükleme: parçalar

- **Durum:** kabul edildi (2026-09-26).
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** CLAUDE.md §21; TODOS.md `SYNC-10`; ADR 0031 (dosya projeleri), 0040 (masaüstü istemcisi), 0043 (çevrimdışı çalışma)
- **Sahibin yönü:** proje zayıf bağlantıda da kusursuz ve güçlü biçimde eşitlenecek.

## Bağlam

- Dosya projesinin revizyonu tek bir `PUT` ile gidiyordu. Bağlantı yarıda koparsa sunucu aldığını siler ve bütün dosya baştan gider.
- Sahada, zayıf ya da kesik kesik bağlantıda büyük bir dosyanın hiç bitmemesi mümkündü.

## Karar

- `PUT …/uploads/{yükleme}?offset=N` dosyanın bir parçasıdır: en çok 32 MiB, `N` o ana kadar gelen bayt sayısıdır.
  - Parça önce bütünüyle alınır, sonra bir şey yazılır; yarıda kalan parça dosyaya hiç ulaşmaz.
  - Yükleme satırı kilitlenir (`select … for update`); `N` sunucudaki boyla aynı değilse parça `offset` alanıyla reddedilir ve ileti nereden sürüleceğini söyler.
  - Parça dosyanın sonuna eklenir ve diske işlenir. Aynı yüklemenin iki parçası birbirine karışamaz.
  - Bildirilen boyu aşacak parça `size` ile reddedilir.
- **Son parça** dosyayı tamamlar. Dosyanın SHA-256'sı diskten okunarak hesaplanır ve bildirilenle karşılaştırılır; dosya KCAD v2 olarak doğrulanır (tek parça gönderimle aynı kurallar, ADR 0031). Tutmazsa hiçbir şey saklanmaz, yeni yükleme gerekir.
- **Durum:** yüklemenin yanıtı ve `GET …/uploads/{yükleme}` artık gelen bayt sayısını da verir (`FileUpload.receivedBytes`). Bu sayı nesnenin diskteki boyudur; veritabanında yeni bir şey tutulmaz, migration gerekmez. Günlük temizlik bitmemiş yüklemeyi ötekiler gibi siler.
- **İstemci (masaüstü):**
  - 8 MiB'tan büyük dosya 8 MiB'lık parçalarla gider (`saving::PART`).
  - Geçici hatada ya da sıra dışı parçada yükleme sorulur; sunucunun aldığı yerden sürer. Yanıtı kaybolan parça iki kez gönderilmez.
  - Ardışık deneme sınırı parça başınadır; ilerleme olunca sıfırlanır.
  - İlerleme sunucunun aldığı bayt olarak bildirilir (`save_revision_watched`).
- Tek parça `PUT` (web'in kullandığı) aynen kalır. En büyük dosya 256 MiB'dır; doğrulama dosyayı bütün okur (ADR 0031).

### İndirme ve açılış

- Revizyon değişmez; kopan indirme kaldığı yerden sürer.
  - `GET …/files/{n}` şu başlığı alır: `Range: bytes=N-`. Yanıt `206 Partial Content`, `Content-Range`, aynı varlık etiketi (SHA-256).
  - Dosyanın sonunu aşan başlangıç `416` alır. Başka türden aralık istenirse bütün dosya gelir. Tam yanıt `Accept-Ranges: bytes` der.
- **İstemci:**
  - Revizyonda kopan indirme `Range` ile, veritabanı projesinin görüntüsünde (her istekte yeniden üretilir) baştan yeniden denenir. İlerleme olmadan en çok 5 kez.
  - Gelen parça aynı dosyanın devamı değilse (başka etiket, başka başlangıç) indirme baştan başlar.
  - Sonuç her durumda varlık etiketiyle denetlenir.
- Açılışın sunucu adımları (proje bilgisi, nesne sayfaları, revizyon listesi) geçici hatada bekleyerek yeniden denenir. Tek bir kopuk yanıt bütün açılışı düşürmez.

## Bu dilimde olmayanlar

- **Programın kapanıp açılmasından sonra aynı yüklemeye devam etmek:** bugün yeni yükleme başlar. Bekleyen kayıt cihazda durduğu için (ADR 0043) iş kaybolmaz, yalnız yeniden gönderilir.
- **256 MiB üstü dosyalar:** akışla doğrulama gerekir.
- **Web'de parçalı yükleme:** web çevrimiçi kalır; ihtiyaç olursa aynı yol kullanılır.

## Doğrulama (26 Eylül 2026, Linux)

- Gerçek sunucu, `a_file_goes_in_parts_and_a_cut_goes_on_from_what_arrived`:
  - örnek çizim 1 KiB'lık parçalarla kaydedildi; ilerleme her parçada arttı, sonunda bütün dosyaya ulaştı; 1. revizyon, 13 nesne;
  - kesinti: ilk parça geldi, aynı sıradan ikinci gönderim `offset` ile reddedildi, durum 1024 bayt dedi, kalan parça dosyayı tamamlayıp doğruladı, 2. revizyon olarak kaydedildi;
  - bildirilen özeti tutmayan dosyanın son parçası `sha256` ile reddedildi, hiçbir şey saklanmadı (gelen bayt 0).
- Kasıtlı bozma: sunucu parça sırasını denetlemeyince aynı parça iki kez eklendi (2048 bayt) ve test düştü; geri alındı.
- `pnpm rust:test` (veritabanı zorunlu, clippy, bağımlılık yönü) ve `pnpm typecheck` geçti.
- **İndirme:**
  - `files_tests.rs`: 10. bayttan aralık 206, doğru `Content-Range` ve baytlar verir; dosya boyunda başlangıç 416, kapalı aralık bütün dosyayı getirir.
  - `crates/native/cloud/tests/download_resume.rs`: yerel bir sahte sunucu ilk yanıtı yarıda keser. İstemci ikinci istekte `Range: bytes=<yarı>-` sorar, parçaları birleştirir, SHA-256 tutar.
  - Kasıtlı bozma: revizyon sürdürülemez sayılınca istemci baştan istedi, sunucu artık yanıt vermedi ve test düştü; geri alındı.
