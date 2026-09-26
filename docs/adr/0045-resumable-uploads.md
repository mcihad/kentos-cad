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
- ~~**Web'de parçalı yükleme**~~: aynı gün geldi, aşağıdaki “Web” bölümüne bakın.

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

## Web (26 Eylül)

- **Yükleme** (`app/cloud/transfer.ts`, `uploadInParts`): 8 MiB'tan büyük dosya 8 MiB'lık parçalarla gider (`UPLOAD_PART`, masaüstünün parçası). Daha küçük dosya eskisi gibi tek `PUT` ile gider.
  - Her parça sunucudaki bayt sayısından sürer (`?offset=N`).
  - Bağlantı koparsa ya da parça sıra dışı diye reddedilirse yükleme sorulur (`receivedBytes`). Sonraki parça sunucunun aldığı yerden gider. Yanıtı kaybolan parça böylece iki kez gönderilmez. Son parçanın yanıtı kaybolduysa yükleme doğrulanmış bulunur.
  - Ardışık deneme sınırı parça başınadır; ilerleme olunca sıfırlanır. Süresi dolan yükleme bir kez baştan açılır.
  - İlerleme bütün dosyanın gönderilen baytıdır.
  - Parça boyu için bir test kancası vardır: `CloudSession.uploadPart`. Uçtan uca test onu 128 bayta indirir.
- **İndirme** (`app/cloud/download.ts`):
  - Revizyon ve kontrol noktası dosyası yarıda koparsa `Range: bytes=N-` ile sürer. Veritabanı projesinin görüntüsü her istekte yeniden üretildiği için baştan istenir.
  - 206 aynı varlık etiketini ve istenen başlangıcı taşımalıdır. Başka etiket ya da başlangıç, 416 ya da bütün dosya (200) gelirse indirme baştan okunur; parçalar karışmaz.
  - İlerleme olmayan en çok 5 deneme yapılır. Ret (403, 404, 410) yeniden denenmez.
  - Sonuç, eskisi gibi, varlık etiketiyle (SHA-256) denetlenir.
- **Doğrulama:**
  - Vitest:
    - `api.test.ts`: `Range` ile süren revizyon; başka etiketle gelen devam ve bütün dosya baştan okunur; görüntü baştan istenir; beş boş denemede vazgeçilir; ret bir kez sorulur.
    - `transfer.test.ts`: parçalar sunucunun aldığı yerden gider, küçük dosya tek parçadır; kesilen parça yeniden gider; yanıtı kaybolan parça iki kez gitmez; son parçanın yanıtı kaybolunca yükleme doğrulanmış bulunur; sıra dışı parça sunucunun sayısından sürer.
    - `fileProject.test.ts`: büyük çizimin Kaydet'i parçalarla, kesilen parçayla bir revizyon yazar.
  - `KENTOS_E2E_DB=scratch pnpm e2e:cloud`, gerçek sunucuyla (100 denetim geçti):
    - ikinci Kaydet 128 baytlık parçalarla gider (yaklaşık 700 baytlık çizim, altı parça); üçüncü parçanın yanıtı tarayıcıda düşürülür; parçalar 0, 128, 256, 384, 512, 640 sırasıyla ve tekrarsız gider; revizyonun baytları yüklenenlerle aynıdır;
    - “Son revizyonu aç”ın indirmesi yarıda kesilir ve `Range: bytes=N-` ile sürer (`bytes=353-`).
    - İlk koşuda parça 256 bayttı ve yanıtı düşürülen üçüncü parça son parçaydı: istemci yüklemeyi sorup doğrulanmış buldu, dördüncü parça gitmedi. Davranış doğruydu, denetim yanlış kurulmuştu; parça 128 bayta indirildi ki yanıtı düşen parça ortada kalsın.
  - Kasıtlı bozmalar (her biri tek başına, sonra geri alındı):
    - A: başarısız parçadan sonra yükleme sorulmadı: `transfer.test.ts`'in iki testi düştü (kesilen parça, yanıtı kaybolan parça).
    - B: kesilen indirme `Range` yerine baştan istendi: `api.test.ts`'in iki testi düştü.
    - C: başka etiketli 206 devam sayıldı: “başka dosyanın devamı baştan okunur” testi düştü.
    - D: parça `offset`'siz gönderildi: `transfer.test.ts`'in üç, `fileProject.test.ts`'in bir testi düştü.

