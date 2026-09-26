# ADR 0038: Web'de dosya projeleri, `.kcad` indirme, tek işlemde yükleme ve proje geçmişi

- **Durum:** kabul edildi (2026-09-26). Sunucu tarafı ADR 0031 (dosya projeleri), 0033 (tek anlık görüntü), 0034 (kontrol noktaları), 0036 (içe aktarım) ve 0039'dadır (saklama biçimleri arası dönüştürme); bu karar onların web arayüzüdür. Yön TODOS.md `SYNC-04`, `SYNC-06`, `SYNC-15`, `CLOUD-07`, `PG-14` ve `PG-15`'ten gelir. Birinci adım dosya projeleri, indirme, tek işlemde yükleme ve dönüştürmedir; ikinci adım geçmiş ve kontrol noktalarıdır (aşağıda).
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** CLAUDE.md §4.8, §13, §21.2–21.3; ADR 0025, 0026, 0028, 0030, 0031, 0033, 0034, 0036, 0039

## Bağlam

- Sunucu dosya projelerini, veritabanı projesinin tek anlık `.kcad` görüntüsünü, boş projeye tek işlemde içe aktarımı, kontrol noktalarını ve biçimler arası dönüştürmeyi biliyordu; web hiçbirini kullanmıyordu.
- Web yerel çizimi buluta 2000'lik `project.changes` parçalarıyla gönderiyordu. Yükleme yarıda kalırsa bulutta yarım bir proje kalıyordu (`PG-14`).
- Web'in bulut projesi kavramı yalnız kendiliğinden kaydedilen veritabanı projesiydi.

## Karar

### Saklama biçimi

- “Buluta yükle” penceresi saklama biçimini sorar. İki seçenek `app/cloud/sharing.ts`'teki `STORAGE_TEXT` ile anlatılır: **veritabanı** (nesne nesne PostGIS) ve **dosya** (KCAD revizyonları). Biçim sonradan değişmez (ADR 0031); pencere bunu söyler. Varsayılan veritabanıdır.
- “Buluta dosya olarak kaydet” (`cloud.uploadFile`) aynı pencereyi dosya seçili açar.

### Dosya projesi olarak kaydetme

- Proje `storage: file` ile açılır (`project.create`). Çizimin KCAD v2 baytları yerel Kaydet'in yoluyla yazılır: tipli sütunlar, biçim işçisi, geri okuyarak doğrulama (ADR 0030; `app/drawingFile.ts` `encodeDrawing`).
- Baytların SHA-256'sı tarayıcıda hesaplanır (`crypto.subtle`). Sonra yükleme açılır, baytlar tek `PUT` ile gönderilir, `project.file.commit` `@file = "0"` ile 1. revizyonu yazar.
- Yükleme yarıda kalırsa proje boş kalır ve çizim yerel kalır; kullanıcıya boş projeyi silmek ya da bırakmak sorulur (aşağıdaki içe aktarım gibi).

### Dosya projesini açma

- Dosya projesi en yeni revizyonundan açılır. Revizyonun baytları açılışın penceresinde indirilir (ilerlemesi ve Vazgeç'iyle; `DocumentFiles.openCloud`).
- İndirilen baytlar sunucunun SHA-256'sıyla (`ETag` ve revizyon listesi) karşılaştırılır; tutmayan dosya çizim olmaz.
- Sonra yerel dosya gibi aşamalı okunup denetlenir (ADR 0030). Çizim yalnız bütün revizyon okununca tek adımda değişir. Açılan revizyon, çizimin **dayandığı revizyon** olarak saklanır.
- Projenin adı katalogdakidir; revizyonun içindeki ad, kaydedildiği andaki addır.
- Henüz revizyonu olmayan dosya projesi kendi bilgileriyle ve nesnesiz açılır; ilk Kaydet 1. revizyonu yazar.
- Katalog projenin biçimini kendi kaydından (`ProjectSummary.storage`) bilir. Kopya, geri yükleme ve dönüştürmede yeni projenin biçimi komuttan bellidir; web onu açarken bu biçimi kullanır.

### Kaydet

- Dosya projesi kendiliğinden kaydedilmez. Kaydet (Ctrl+S) dayandığı revizyonun üstüne yeni revizyon yazar.
- Aşamalar durum çubuğunda ayrı görünür (`SYNC-04`):
  - **Dosya hazırlanıyor** (kodlama);
  - **Yükleniyor %N** (`XMLHttpRequest`'in yükleme ilerlemesi; `fetch` gönderdiğini bildiremez);
  - **Sunucu doğruluyor** (boyut, özet, KCAD v2 okunabilirliği ve kayıt);
  - **Buluta kaydedildi · rN**.
- `FileCommitted` gelmeden hiçbir şey “kaydedildi” denmez.
- Yalnız yazılan revizyon kaydedilmiş sayılır (`markSaved(revizyon)`, CLAUDE.md §4.8). Kayıt sürerken yapılan değişiklik kaydedilmemiş kalır.
- Bağlantı baytlar giderken koparsa aynı yükleme yeniden gönderilir (sunucu kopan gövdeden bir şey saklamaz). Süresi dolan yükleme bir kez yeniden açılır.
- Yanıtı kaybolan gönderimde baytlar sunucuya ulaşmış olabilir; sunucu aynı baytları ikinci kez almaz. Bu yüzden yeniden göndermeden önce yükleme sorulur (`GET …/uploads/{id}`, ADR 0040). Baytlar alınmışsa yeniden gönderilmez. Bu yolu bilmeyen sunucuda baytlar yeniden gider; ret gelirse Kaydet hatayı söyler, sonraki Kaydet yeni bir yüklemeyle yazar.
- Yanıtı kaybolan kayıt aynı idempotency anahtarıyla yeniden gönderilir; sunucu saklı yanıtı verir. Bu yüzden istemci kendi kaydıyla asla çakışmaz. Yanıtı hiç gelmeyen kayıt sonraki Kaydet'te, anahtarıyla, önce gönderilir.
- Kayıt hatası durumda ve iletide görünür; çizim olduğu gibi durur, Kaydet yeniden denenir.
- “Kaydedilmemiş değişiklikler” sorusundaki **Kaydet** da açık dosya projesine yeni revizyon yazar.

### Çakışma ve başkasının revizyonu

- Başkası daha önce kaydettiyse sunucu kaydı `@file` çakışmasıyla reddeder; hiçbir şey yazılmaz. İki dosya bayt düzeyinde birleştirilmez (`SYNC-06`).
- `confirmDialog` sorar:
  - **Ayrı kopya olarak kaydet**: çizim yeni bir dosya projesi olur (yükleme penceresi, dosya seçili, “(kopya)” adıyla); açık proje o olur;
  - **Yerel dosyaya kaydet**: Farklı kaydet; çizim buluttaki projeden ayrılır;
  - **Son revizyonu aç**: en yeni revizyon açılır; çizimin kaydedilmemiş değişiklikleri bilerek atılır (düğme kırmızı, uyarı sorudadır);
  - **Vazgeç**: hiçbir şey olmaz; durum çakışma olarak kalır.
- Proje açıkken başkası revizyon kaydederse (`project.file` olayı, istek kimliği bu pencerenin değil) bu söylenir ve durum çubuğu “Yeni revizyon: rN” der. Tıklanınca en yeni revizyon önerilir; kaydedilmemiş değişiklik varsa aynı çakışma sorusu gelir. Çizim kendiliğinden yeniden yüklenmez.
- Proje silinir, arşivlenir ya da erişim kalkarsa Kaydet reddedilir; çizim ekranda kalır, Kaydet yerel dosya önerir. Rol düşerse Kaydet “salt okunur” olur, dönünce yeniden yazar.

### Kurtarma kopyası

- Dosya projesinin kaydedilmemiş işi yerel çizimin işi gibi yalnız çizimdedir; ADR 0030'un kurtarma kopyaları ona da yazılır.
- Cihaz taslağı yalnız veritabanı projesinindir (ADR 0026). Kopya yazılmayan tek durum açık bir veritabanı projesidir (`CloudSession.keepsDeviceDraft`).
- Çizim sorusuz değiştirilirken (bulut projesi açılırken) kaydedilmemiş iş önce kopyaya yazılır.

### `.kcad` olarak indirme

- Katalogda “.kcad olarak indir”: veritabanı projesinde tek anlık görüntü (ADR 0033), dosya projesinde en yeni revizyon. Geçmiş sekmesinde her revizyon ayrıca indirilir.
- Kaydedilecek yer önce sorulur (tıklama henüz geçerliyken); sonra baytlar ilerlemeyle gelir, SHA-256'yla denetlenir ve yazılır. Dosyaya yazamayan tarayıcıda indirme olarak verilir.

### Veritabanı projesine yükleme: tek içe aktarım

- “Buluta yükle” (veritabanı) artık şöyledir: proje açılır, çizimin `.kcad`'i yüklenir, `project.import` tek işlemde aktarır (ADR 0036). Her nesne kalıcı kimliğiyle ve 1. sürümüyle gelir. Otomatik kayıt bu sürümlerle başlar; olay imleci içe aktarımdan sonraki projeden okunur.
- Sunucunun almadığı ilk nesne bütün dosyayı reddeder (`entities[i]`). Proje boş kalır, çizim yerel kalır. Soru “Boş projeyi sil” ya da “Çizimi yerelde tut” der. Reddedilen nesne sırası, türü, katmanı ve etiketiyle söylenir; çizimde seçilip gösterilir.
- Eski parça parça yol yalnız içe aktarımı olmayan sunucu için kalır: yükleme yolu yok (çıplak 404/405), veritabanı projesine dosya almıyor ya da `project.import` bilinmiyor.
- Yükleme sürerken çizim değiştiyse (pencere kipli olduğu için pratikte olmaz) proje içe aktarılmış hâliyle kalır. Çizim ona bağlanmaz; değişiklikleri yerinde durur.
- Açık bir veritabanı projesine başkası içe aktarım yaparsa (`project.import` olayı) proje yeniden açılır.

### Dönüştürme (ADR 0039)

- Katalogda dosya projesine “PostGIS'e aktar…”, veritabanı projesine “Dosya projesine çevir…”. `project.convert` yeni bir proje açar; kaynak değişmez.
- Pencere isteğe bağlı adı ve çalışma alanını sorar, olacakları söyler: kaynağın hangi hâli aktarılır, kaynak olduğu gibi kalır, reddedilen nesne.
- Yeni proje kopya gibi “Projelerim”de seçilir ve açılır; biçimi komuttan bellidir.

### Kod yerleşimi

| Parça | Yer |
|---|---|
| Yükleme, indirme, özet, kayıt zarfı | `app/cloud/api.ts`, `app/cloud/transfer.ts` |
| Dosya projesinin Kaydet'i | `app/cloud/fileProject.ts` |
| Açma, dosya olarak kaydetme, Kaydet'in iletileri | `app/cloud/fileSession.ts` |
| Tek işlemde yükleme ve eski yol | `app/cloud/importing.ts`, `app/cloud/uploading.ts`, `app/cloud/upload.ts` |
| Açılış penceresinde indirme | `app/fileIO.ts` (`openCloud`) |
| Pencereler | `ui/cloud/UploadDialog.ts`, `FileConflict.ts`, `downloads.ts`, `catalogDetails.ts`, `catalogHistory.ts`, `ProjectForms.ts` (`openConvertDialog`) |
| Durum | `ui/statusbar/cloudCells.ts`, `ui/appmenu/AppMenu.ts` |

## Doğrulama (birinci adım)

- Vitest, sahte sunucuyla (`app/cloud/fakeFiles.ts`):
  - `fileProject.test.ts`: aşamaların sırası; “kaydedildi”nin kayıttan sonra gelmesi; kirli kuralı; kopan yükleme; baytları ulaşmış ama yanıtı kaybolmuş gönderim (yükleme sorulur, ikinci kez gönderilmez; yolu bilmeyen sunucuda sonraki Kaydet yazar); kaybolan kayıt yanıtı; başkasının önce kaydettiği revizyon; açıkken gelen başkasının revizyonu ve bu pencerenin kendi olayı; rol düşmesi ve silinme; sunucuya ulaşılamaması ve yeniden deneme.
  - `fileSession.test.ts`:
    - dosya olarak kaydetme ve açma, bozuk indirme, revizyonsuz proje;
    - soru üzerinden Kaydet;
    - tek içe aktarım, reddedilen nesne, eski sunucu;
    - dosya projesinin kurtarma kopyası.
  - `transfer.test.ts`, `api.test.ts`, `lifecycle.test.ts`: özet, yeniden gönderme, indirme denetimi, başlıklar, içe aktarımı olmayan sunucu, dönüştürme.
- `pnpm e2e:cloud` (gerçek sunucu, geçici veritabanı):
  - yükleme tek içe aktarımla;
  - dosya olarak kaydetme, iki Kaydet ve aşamaları;
  - indirilen revizyonun yüklenen baytlarla aynılığı;
  - başka istemcinin revizyonunun duyulması;
  - çakışmanın iki cevabı: son revizyon ve ayrı kopya.

## İkinci adım: geçmiş ve kontrol noktaları (26 Eylül)

Sunucu tarafı ADR 0034'tür (kontrol noktası oluşturma, listeleme, indirme, silme; yeni proje olarak geri yükleme). Yön `SYNC-11` ve `CLOUD-07`'den gelir.

### Geçmiş sekmesi

- Katalogda seçili projenin **Geçmiş** sekmesi iki liste gösterir, en yeni önce:
  - **Kontrol noktaları**: ad, not, tür (anlık görüntü ya da adlandırılmış revizyon), gösterdiği revizyon, kim, ne zaman, boyut ve nesne sayısı;
  - **Revizyonlar** (yalnız dosya projesinde): numara, kim, ne zaman, boyut ve nesne sayısı. Veritabanı projesinde revizyon dosyası olmadığı ve şimdiki hâlin “.kcad olarak indir”le alındığı söylenir.
- Her satır indirilir ve “Yeni proje olarak geri yükle…” ile yeni proje olur; kontrol noktası satırında ayrıca “Sil…” vardır.
- Hesabın yapamayacağı iş görünür kalır, düğmesi kapalıdır ve nedenini (eksik hakkı) söyler. Sunucu her isteği yine kendisi denetler (`app/cloud/history.ts`):
  - listeyi görmek `project.history`, indirmek ve geri yüklemek ayrıca `project.download` ister;
  - kontrol noktası oluşturmak `feature.write` ister; arşivdeki projede ve revizyonu olmayan dosya projesinde yoktur;
  - silmek yalnız oluşturanın (yazma hakkı sürdükçe) ya da `project.edit` sahibinindir; arşivde yoktur.
- Çöpteki projenin geçmişi gösterilmez (açılmaz).

### Kontrol noktası oluşturma

- “Kontrol noktası oluştur…” adı (en çok 120 karakter) ve isteğe bağlı notu (en çok 2000) sorar.
- Dosya projesinde adlandırılacak revizyon seçilir; varsayılan en yenisidir. Dosya kopyalanmaz; kontrol noktası silinse de revizyon kalır.
- Veritabanı projesinde projenin şimdiki hâli tek `.kcad` olarak saklanır. Proje bu pencerede açıksa gönderilmeyi bekleyen değişiklikler önce gönderilir; kontrol noktası onları da tutar.

### Silme

- “Sil…” `askRemove` ile sorar. Adlandırılmış revizyonda revizyonun kaldığı, anlık görüntüde dosyasının da silindiği söylenir.

### Yeni proje olarak geri yükleme

- Kontrol noktası ya da dosya projesinin herhangi bir revizyonu geri yüklenir. Pencere isteğe bağlı adı ve çalışma alanını sorar (hesabın proje açabildiği alanlar, `project.create`). Olacakları da söyler:
  - kaynak olduğu gibi kalır;
  - yeni projenin biçimi: dosya noktası dosya projesi, anlık görüntü veritabanı projesi olur;
  - yeni proje hesabın olur; geçmiş ve paylaşım gelmez.
- Yeni proje kopya gibi “Projelerim”de seçilir ve açılır; biçimi komuttan bellidir.
- Yeni proje yalnız liste onu gösterdiğinde açılır. Arama ve tür süzgeci önce temizlenir. Liste onu göstermezse durum satırı söyler; sonraki bir liste kendiliğinden hiçbir proje açmaz. Aynı kural dönüştürmede de geçerlidir.

### Güncellik

- Sekme açıkken projenin olayları dinlenir (`CloudSession.watchProject`):
  - açık proje kendi kanalından (`CloudSession.events`);
  - başka bir proje, sekmenin kendi kanalından; kanal projenin o anki olay imlecinden başlar.
- `project.checkpoint` ya da `project.file` olayı, ya da kanalın yeniden eşitlemesi, listeyi 250 ms sonra yeniden sorar. Liste yanıt gelene kadar olduğu gibi kalır.
- Sekme ya da pencere kapanınca kanal kapanır; geç gelen yanıt yeni seçimin üstüne yazılmaz.

### Saklama biçimi

- Bilgiler sekmesi dosya projesinin nesne sayısını en yeni revizyondan verir: “3 nesne (revizyon 5)”. Sunucu yalnız veritabanı projesinin satırlarını sayar; dosya projesinin kapsamı hesaplanmaz ve bu söylenir.
- Açık projenin biçimi, oturumun onu açtığı biçimdir (`HistoryPanel.storageOf`). Katalog kaydının biçimi (`ProjectSummary.storage`) bu dal açılırken sunucuda her proje için “database” diyordu; main'deki 5c3d2a6 (ADR 0039) bunu düzeltti. Açık olmayan dosya projesinin geçmişi, indirmesi ve dönüştürmesi o düzeltmeye dayanır.

### Bu adımda olmayanlar

- Bir noktayı salt okunur açmak ve iki noktayı karşılaştırmak yoktur (`SYNC-11`, `CLOUD-07`'nin kalanı).

### Kod yerleşimi (ikinci adım)

| Parça | Yer |
|---|---|
| Geçmişin okunması, hak kuralları | `app/cloud/history.ts` |
| Oluşturma, silme, geri yükleme komutları | `app/cloud/lifecycle.ts` |
| Projenin olaylarını dinleme | `app/cloud/session.ts` (`watchProject`) |
| Sekme, satırlar, pencereler | `ui/cloud/historyPanel.ts`, `ui/cloud/catalogHistory.ts`, `ui/cloud/HistoryForms.ts`, `ui/cloud/CatalogDialog.ts` |

## Doğrulama (ikinci adım)

- Vitest, sahte sunucuyla: `history.test.ts`.
  - Silme hakkı: oluşturan, başka düzenleyici, yönetici, görüntüleyici olmuş oluşturan, arşiv.
  - Oluşturma ve alma hakları.
  - Veritabanı projesinin listesi; `project.history` yoksa sorulmaması.
  - Dosya projesinde en yeni ve seçilen revizyonun adlandırılması; kontrol noktasının ve revizyonun yeni proje olarak geri yüklenmesi, kaynağın değişmemesi; silme; başkasınınkini düzenleyicinin silememesi.
  - Açık olmayan ve açık projede `project.checkpoint` olayıyla listenin yenilenmesi; sekme kapanınca kanalın kapanması.
- `pnpm e2e:cloud` (gerçek sunucu, geçici veritabanı):
  - açık dosya projesinin revizyonuna kontrol noktası; indirilen baytların özeti; yeni dosya projesi olarak geri yükleme ve açılması; kaynağın değişmemesi;
  - veritabanı projesinin kontrol noktası; “.kcad olarak indir” ile okunabilir tek dosya; aynı kimliklerle yeni veritabanı projesi olarak geri yükleme; sorulduktan sonra silme;
  - açık olmayan dosya projesinin revizyonları, bilgileri ve revizyon geri yükleme; “PostGIS'e aktar” ve “Dosya projesine çevir”. Bu son adımlar main'deki sunucuya dayanır (5c3d2a6).
- Sonuç (26 Eylül): bu dalın sunucusunda bütün geçmiş ve kontrol noktası adımları geçti; açık olmayan dosya projesinin geçmişinde durdu (katalog onu veritabanı projesi sanıyordu). main'in sunucusuyla (`893a63b`'nin yapısı, `KENTOS_E2E_SERVER`) bütün adımlar geçti.


## Bilerek bozma (26 Eylül)

Her kural kaynakta bilerek bozuldu, adı geçen Vitest testi düştü, kaynak geri alındı ve ağaç temiz kaldı (`ece5a95`'in kaynağı, bu kaydın testleriyle). “Başkasının revizyonu kendiliğinden yüklenmez” kuralını oturum düzeyinde yakalayan test yoktu; `fileSession.test.ts`'e eklendi, bozma onunla düştü. Kaynağın değişmemesi ve yeni projenin açılması sunucunun ve arayüzün işidir; web'de bozulacak bir satırı yoktur, `pnpm e2e:cloud` sınar.

| Kural | Bozma | Düşen test |
|---|---|---|
| “kaydedildi” yalnız kayıttan sonra | yükleme biter bitmez “kaydedildi” | `…calls it saved only once the server committed it` |
| yalnız yazılan revizyon kaydedilmiş sayılır (§4.8) | `markSaved` çizimin o anki sürümüyle | `an edit made while the file goes up stays unsaved…` |
| `@file` çakışmasında hiçbir şey yazılmaz | çakışmada en yeni revizyonun üstüne yeniden kaydedilir | `someone else saved first: nothing is written…` |
| yanıtı kaybolan kayıt aynı anahtarla gider | her denemede yeni anahtar | `a commit whose answer is lost goes again with its key…` |
| kopan yükleme aynı yüklemeyle yeniden gönderilir | ilk kopuşta vazgeçilir | `a connection cut while the bytes go…`, `sends a cut-off upload again…` |
| baytları ulaşmış gönderim ikinci kez gönderilmez (ADR 0040) | yükleme sorulmaz | `bytes that arrived but whose answer was lost…`, `asks an upload whose answer was lost…` |
| indirilen baytlar SHA-256'yla denetlenir | denetim yok | `never takes a download whose bytes are not the server’s`, `a download that changed on the way…` |
| reddedilen içe aktarımda proje boş, çizim yerel kalır | ret parça parça yola düşer | `an import refused by one object leaves the new project empty…` |
| eski yol yalnız içe aktarımı olmayan sunucu için | her `invalid` “içe aktarım yok” sayılır | `tells a server without the import from a refusal`, `an import refused by one object…` |
| başkasının revizyonu kendiliğinden yüklenmez | duyulunca en yeni revizyon açılır | `…never reloaded by itself` |
| dosya projesinin işine kurtarma kopyası yazılır | açık dosya projesi de cihaz taslağı sayılır | `a file project’s unsaved work gets a copy…` |
| kontrol noktasını yalnız oluşturanı ya da `project.edit` siler | yazabilen herkes | `removing a checkpoint: its maker…` |
| `project.checkpoint` olayı açık listeyi yeniler | yalnız `project.file` dinlenir | `a project.checkpoint event asks for the list again…` |
| sekme kapanınca projenin kanalı kapanır | kanal açık kalır | `a project.checkpoint event asks for the list again…` |
