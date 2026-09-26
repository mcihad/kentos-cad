# ADR 0041: Masaüstünün bulut arayüzü: giriş, katalog, açma, kaydetme, izleme, çakışma, yükleme ve bağlantısız çalışma

- **Durum:** kabul edildi (2026-09-26).
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** CLAUDE.md §4.4, §13, §21; TODOS.md `SYNC-01`, `SYNC-04`, `SYNC-08`, `SYNC-09`, `SYNC-12`, `SYNC-15`, `CLOUD-04`; ADR 0015 (proje erişimi), 0023 (tipli ayarlar), 0026 (kalıcı kimlikle eşitleme), 0028 (katalog), 0030 (aşamalı açma ve kayıt), 0031 (dosya projeleri), 0038 (web'in dosya projeleri), 0040 (masaüstü bulut istemcisi), 0043 (çevrimdışı yerel kopya), 0044 (olayları bekleyerek sorma), 0045 (kaldığı yerden süren yükleme)

## Bağlam

- `kentos-cloud` (ADR 0040) masaüstünün bulut istemcisidir: giriş, katalog, iki tür projeyi açma, dosya projesinin revizyonu, çizimden yeni proje, veritabanı projesinin değişiklikleri (`ProjectSync`), başkalarının değişiklikleri, cihaz taslağı. Arayüzü yoktu.
- Aynı gün kütüphaneye eklenenler de bu dilime girdi:
  - yerel kopya ve çevrimdışı çalışma (ADR 0043; sahibin kararı: masaüstü bağlantısız da çalışır, web çevrimiçi kalır);
  - olayları bekleyerek sorma (ADR 0044);
  - parçalı, kaldığı yerden süren yükleme (ADR 0045).
- Web'in bulut arayüzü (`apps/web/src/ui/cloud/`, `app/cloud/session.ts`) örnektir: sözcükleri, soruları, pencereleri. Masaüstü onu Iced ve KentOS UI ile kurar; kod paylaşılmaz, sunucu ve sözleşmeler ortaktır.

## Karar

### Yerleşim

`apps/desktop/src/cloud/` (her dosya tek iş):

| Dosya | İçerik |
|---|---|
| `mod.rs` | durum (`CloudState`), olaylar (`Event`), komutlar (`cloud.*`), saat |
| `account.rs` | giriş penceresi, çıkış |
| `catalog.rs` | “Bulut projeleri”: listeler, arama, sayfa, “Bu cihazdaki projeler”, “Bu cihazdan kaldır” |
| `opening.rs` | açma: kopyanın kilidi, çevrimiçi ya da kopyadan, ilerleme, taslağın geri konması |
| `leaving.rs` | çizimden ayrılma: önce taslak, olmazsa soru |
| `live.rs` | veritabanı projesinin kendiliğinden kaydı ve cihaz taslağı |
| `follow.rs` | başkalarının değişiklikleri, erişim, çakışmanın iki seçeneği, biten proje |
| `copy.rs` | yerel kopya ve bağlantı durumu |
| `file.rs` | dosya projesinin kaydı ve çakışması |
| `upload.rs` | “Buluta yükle” |
| `view.rs`, `words.rs` | pencereler, durum çubuğu; web'in sözcükleri |

- Her istek kendi görevidir (Iced'in `Task::perform`, iptal tutamacıyla). Yanıt, isteğin ya da çizimin kimliğiyle döner; geç yanıt yeni çizime uygulanmaz (CLAUDE.md §21.2). Tutamaç düşünce istek durur.
- Zamanlayıcılar tek bir saattir (`Tick`, 200 ms), yalnız bekleyen bir şey varken çalışır. Kararlar uygulamanın durumudur; testler kendi saatleriyle sürer.
- Mesajlar bulut olayını kutuda taşır (`Message::Cloud(Box<Event>)`): bazı olaylar projenin bütün bilgisini taşır.
- Ported komutlar: `cloud.signIn`, `cloud.signOut`, `cloud.open`, `cloud.upload`, `cloud.conflicts` (`apps/desktop/ported.json`, envanter).

### Sunucu adresi ve son hesap: tipli ayar

- Ayar şemasına **metin türü** eklendi (`SettingType::Text`): en çok `max` karakterlik metin. Rust ve TypeScript aynı sayar (bayt değil karakter). Ortak durumlar (`fixtures/settings/v1`) türü, sınırı ve katmanları sınar.
- `cloud.server` (masaüstü, kullanıcı tercihi, varsayılan `http://127.0.0.1:8787`): giriş penceresinde değişir, giriş başarılı olunca saklanır. Adres gönderimden önce `Cloud::new` ile denetlenir; şifresiz http yalnız bu bilgisayara (ADR 0040).
- `cloud.account` (masaüstü): son giriş yapan hesabın kimliği. Bağlantı ya da oturum yokken bu cihazdaki projeler onunla bulunur. Kimlik gizli değildir; oturum ve parola hiçbir yere yazılmaz.

### Giriş ve çıkış

- “Buluta giriş” penceresi: sunucu, giriş adı, parola. Sunucunun Türkçe iletisi pencereye yazılır. Parola yalnız pencere açıkken bellekte durur, pencereyle gider. Açık bir bulut projesi varken sunucu değiştirilemez (değişiklikler oraya gider).
- Hesap gerektiren komut (Bulut projesi aç, Buluta yükle) önce girişi açar, sonra kendiliğinden sürer (web gibi).
- Durum çubuğu: projenin çalışma alanı ve adı, kaydın durumu, bağlantı, hesabın adı ve **Çıkış**; giriş yokken **Buluta giriş**.
- Çıkış açık projeden ayrılır (web gibi): çizim ekranda yerel çizim olarak kalır. Gönderilmemiş iş varsa önce taslak yazılır (aşağıda “Ayrılma”).

### Katalog: “Bulut projeleri”

- Listeler, web'in sırasıyla: Son kullanılanlar, Favoriler, Projelerim, her kurum için “Kurum: …”, Benimle paylaşılanlar, Arşivlenmişler; ayrıca **Bu cihazdaki projeler** (ADR 0043). Web'in Çöp kutusu masaüstünde yok.
- Arama ve sayfa sunucudadır (`q`, `after`); yazma 250 ms durunca sorulur, eski yanıt atılır.
- Satır: ad (Açık, Arşivde, ★), çalışma alanı, sahip, son değişiklik (“3 saat önce”), saklama rozeti (Dosya / PostGIS), rol. Tıklama seçer, çift tıklama açar.
- Oturum yokken ya da sunucu yanıt vermezse pencere “Bu cihazdaki projeler”i gösterir; sunucu listeleri giriş ister.
  - Bu listenin satırı kopyanın ne zaman yazıldığını ve sunucunun projeyi bitirip bitirmediğini söyler (Çöp kutusunda, Erişiminiz kaldırıldı, Arşivlendi).
  - **Bu cihazdan kaldır** kopyayı siler. Başka pencere tutuyorsa kütüphanenin iletisi yazılır. Taslakta gönderilmemiş iş varsa önce sorulur: “Gönderilmemiş N değişiklik de silinsin mi?”; taslak yalnız evet denince gider.

### Açma

- Önce projenin kopyası kilitlenir. Başka KentOS penceresi tutuyorsa kütüphanenin iletisi yazılır, açılış olmaz.
- **Çevrimiçi:** `open` → kopya sunucunun çiziminden yeniden yazılır (`reset`, arayüz iş parçacığının dışında) → `ProjectSync::new` → cihaz taslağı geri konur.
- **Çevrimdışı:** oturum yoksa ya da sunucu yanıt vermezse (`network`, `timeout`) proje kopyadan açılır (`load`, arayüz dışında), aynı sırayla. Komut satırı “Çevrimdışı — değişiklikler bu cihazda saklanıyor” der.
- İlerleme katalogda ya da kendi penceresinde; Vazgeç ya da Esc durdurur. Açılış sürerken uygulama komut almaz (ADR 0030'un kuralı).
- Çizim tek adımda değişir: yalnız son açılış, yalnız ekrandaki çizim o arada değişmediyse.
- Arşivlenmiş proje salt okunur açılır, web'in iletisiyle.
- Taslak geri konunca: “Bu cihazda gönderilmemiş değişiklikler vardı; çizime geri kondu.”; konamayan her değişiklik ayrı uyarı; dayandığı sürümü geçilmiş değişiklik çakışmadır; yoldaki komut hemen gider.

### Çizimin kaynağı

- `Document.source`: `Local` (dosyası `path`) ya da `Cloud` (kurum, proje, proje bilgisi, çalışma alanı adı, dosya projesinde dayandığı revizyon).
- Başlık, şeridin sağı ve durum çubuğu projeyi ve çalışma alanını adlandırır. Özellikler paneli “Dosya” yerine “Bulut” satırını gösterir.
- Farklı kaydet bulut projesini yerel dosya yapar: çizim projeden ayrılır (web gibi).
- Veritabanı projesinin gönderilmemiş işi kurtarma kopyasına değil cihaz taslağına gider (iki güvenlik ağı birbirine karışmasın).

### Dosya projesinin kaydı

- Kaydet (Ctrl+S): çizim kayıt işçisinde doğrulanmış KCAD v2 baytlarına yazılır, sonra parçalı yüklenir (`save_revision_watched`, “Yükleniyor %N”). Başarıda çizim kaydedilmiş sayılır, yeni revizyon alınır, kopya o revizyonu tutar.
- Arada başkası kaydettiyse hiçbir şey yazılmaz; pencere:
  - **Ayrı proje olarak kaydet** (birincil): aynı çalışma alanında “… (kopya)” adlı yeni dosya projesi; ilerleme yükleme penceresinde; proje sunucudan açılır.
  - **Sunucudaki son revizyonu aç**: önce sorulur, buradaki değişiklikler bırakılır.
  - **Vazgeç**.
- **Bağlantısız kayıt:** kopyada bekler (`keep_save`); durum “Kaydedildi (bu cihazda) · gönderilecek”. Bağlantı dönünce ya da proje yeniden açılınca gider, sonra silinir. Çakışırsa ikisi de korunur: aynı pencere sorar; bekleyen kayıt sessizce atılmaz. “Ayrı proje olarak kaydet” başarılı olunca bekleyen kayıt silinir.
- Durum sözcükleri web'inkilerdir: “Buluta kaydedildi · r3”, “Kaydedilmedi · r3 üstüne”, “Henüz revizyon yok”, “Çakışma: r5 kaydedilmiş”.

### Veritabanı projesinin kendiliğinden kaydı

- Zaman kuralı web'inkidir: son düzenlemeden 1 s sonra, ilk bekleyenden en çok 5 s sonra. Sonraki parti, yeniden gönderilen komut ve Ctrl+S hemen gider. Tek komut yolda olur.
- Geçici hatada `After::Retry(d)` kadar beklenir, sonra aynı komut aynı anahtarla gider.
- Cihaz taslağı yazılır:
  - düzenlemeden 300 ms sonra;
  - her gönderimden hemen önce (yoldaki komut diske ulaşmadan ayrılmaz);
  - her yanıttan ve hatadan sonra.

  Tek yazım yolda olur; arada istenen, ilkinin ardından gelir. Tutulacak bir şey kalmayınca taslak silinir. Yazılamazsa söylenir, iş sürer.
- **Sıra (ADR 0043):** sunucu tarafını değiştiren her adımdan sonra (yanıt, başkasının değişikliği, benimkini koru, sunucudakini al) önce adım kopyanın günlüğüne eklenir, sonra taslak yazılır. Günlük 200 adımı geçince ve proje kapanırken kopya bütünüyle yazılır.
- Durum sözcükleri: Kaydedildi · Kaydedilmedi (n) · Kaydediliyor · Bağlantı yok — yeniden denenecek · Çakışma · Salt okunur · Arşivlendi · Çöp kutusunda · Erişim kaldırıldı; kalıcı retlerde sunucunun iletisi (“Kaydedilmedi: …”).
- Görüntüleyicinin düzenlemeleri bir kez söylenir ve gönderilmez. Yetkisi olmadığı üst veri değişiklikleri de bir kez söylenir.

### Başkalarının değişiklikleri

- Açık ve bitmemiş veritabanı projesinde olaylar sunucuda beklenir (`follow::wait`, 25 s). Yanıt gelir gelmez yeniden sorulur: dolu sayfa, boş sayfa, alınan değişiklik fark etmez.
- Yanıt yoksa bekleme 1 s'den başlayıp 30 s'ye kadar ikiye katlanır, sonra imleçten sürülür.
- Adlanan nesneler getirilir, dışarıdan değişiklik olarak çizime girer: geri alma adımı yok, kaydedilecek bir şey yok. Burada değişmiş nesne çakışmadır. Açık bir düzenleme sırasında değişiklik bekler, düzenleme bitince alınır.
- Rol değişimi sorulur ve uygulanır. Saklanmayan imleç (`resync_required`) projeyi yeniden açar: önce taslak yazılır, açılış onu üstüne koyar.
- Sunucu projeyi bitirirse (silindi, arşivlendi, erişim kalktı) bir kez söylenir. Kopya işaretlenir (`mark_ended`). Web'in bildirimi gösterilir: ne oldu, ne kaldı, **Yerel kopya kaydet…** (Farklı kaydet).

### Çakışma penceresi

- Satırlar: nesnenin türü, katmanı ve etiketi (“Nokta · Çizim · P1”) ya da “Proje bilgileri (katman ağacı, ayarlar, ad, stiller)”.
- Nedenler: Başkası değiştirdi, Başkası sildi, Kimlik başka nesnede, Proje bilgileri değişti.
- Düğmeler: **Sonra**; **Benimkini koru** (`keep_mine`, sonra hemen gönderim); **Sunucudakini al** (birincil, web'deki gibi güvenli seçenek; üst veri çakışmasında önce projenin bilgisi alınır).
- Açılış: durum çubuğundaki Çakışma, `cloud.conflicts` ya da şerit. Pencere kendiliğinden açılmaz (web gibi); komut satırı söyler.

### Buluta yükle

- Çalışma alanları: etkin, koltuklu ve `project.create` yetkili üyelikler. Açık projenin alanı önce seçilir.
- Saklama: “Dosya (her kayıt bir revizyon)” ya da “Veritabanı (PostGIS)”; varsayılan web'inki gibi Veritabanı.
- Çizim arayüz dışında doğrulanmış KCAD v2'ye yazılır. Dosya projenin adını taşır (dosya projesinin çizimi o adla açılır). `upload_new` projeyi kurar ve doldurur. Aynı yüklemenin yeniden denemesi aynı anahtarı kullanır; alanlardan biri değişince anahtar yenilenir.
- Başarıda proje sunucudan açılır: ekrandaki çizim artık bulut projesidir.
- Ret nesneyi adlandırır ve çizimde seçer: “… Reddedilen nesne: 1. nesne: Nokta · Çizim · P1; çizimde seçildi.”

### Ayrılma

- Pencereyi kapatma, başka çizim ya da proje açma, çıkış ve yükleme önce veritabanı projesinin gönderilmemiş işini taslağa yazar ve söyler: “Gönderilmemiş N değişiklik bu cihazda saklandı; proje yeniden açılınca gönderilecek.”
- Soru yalnız bir şey kaybolacaksa gelir, sayıyla:
  - taslak yazılamadıysa;
  - görüntüleyicinin düzenlemeleri (hiç tutulmaz);
  - taslak klasörü yoksa;
  - dosya projesinin ya da yerel çizimin kaydedilmemiş değişiklikleri.
- Katalogdan gelen soruda Vazgeç kataloğa, dosya çakışmasından gelende çakışma penceresine döner.

### Bağlantı

- Yanıtsız istek (ağ, zaman aşımı) bulutu çevrimdışı yapar; ilk yanıt geri getirir: bekleyen gider, izleme sürer, bekleyen dosya kaydı gönderilir.
- Oturum açıkken çevrimdışıyken hesap 15 s'de bir sorulur (`me`).
- Durum çubuğu: kaydın hücresinde renkli nokta, yanında Çevrimiçi ya da Eşitleniyor. Oturum yokken hücre “Çevrimdışı — değişiklikler bu cihazda saklanıyor” der.
- Hücreler tek satır ve sınırlıdır; 1440 px'lik pencerede çizimin hücreleriyle birlikte sığar.

## Sahibe sorular (uygulanan varsayılanlar)

1. **Dosya çakışması düğmeleri:** brifin sözcükleri kullanıldı (“Sunucudaki son revizyonu aç”, “Ayrı proje olarak kaydet”, “Vazgeç”). Web'de (ADR 0038) “Son revizyonu aç”, “Ayrı kopya olarak kaydet” ve üçüncü seçenek “Yerel dosyaya kaydet” var. Güvenli seçenek iki platformda da birincildir.
   - Önerilen: iki platformu web'in üç seçeneğinde birleştirmek.
   - Varsayılan: brifin iki seçeneği.
2. **Ayrı proje olarak kaydet:** yetki varsa hemen aynı çalışma alanına yüklenir, ilerleme yükleme penceresinde görünür. Web önce pencereyi açar.
   - Önerilen ve varsayılan: hemen.
3. **Çıkış:** açık proje bırakılır, çizim yerel kalır (web gibi).
   - Seçenek: projenin çevrimdışı sürmesi, giriş yapılınca göndermesi.
   - Varsayılan: bırakmak.
4. **Masaüstü kataloğunun kapsamı:** yalnız açma ve bu cihazın kopyaları. Web'in ayrıntı paneli, paylaşma, bilgi düzenleme, kopya, arşiv, çöp kutusu, favori işaretleme ve geçmiş masaüstünde henüz yok.
   - Önerilen: sonraki dilim.
5. **Yükleme saklama varsayılanı:** Veritabanı (web gibi).
   - Seçenek: masaüstünde Dosya.
   - Varsayılan: Veritabanı.

## Bu dilimde olmayanlar

- Proje paylaşma, yeniden adlandırma, silme, arşivleme, geçmiş ve indirmeler masaüstünde: şeritte web'de olduklarını söylerler.
- Dosya projesinde başkasının yeni revizyonunu izleme (web'in “Yeni revizyon: r4”).
- Masaüstünde OpenID girişi ve oturumun anahtarlıkta saklanması (her açılışta parola yeniden sorulur).
- Kopyanın ve taslağın şifrelenmesi (ADR 0043).

## Bilinen sınırlar

- Kopyanın bütünüyle yazılması (200 adımda bir ve kapanışta) ile dosya projesinin kaydından sonraki `reset` ve bağlantısız `keep_save` arayüz iş parçacığında çalışır. Büyük projede kısa bir duraklama olabilir. Ölçüm ADR 0043'te: 100 bin parselin açılışı 0,4 s, bir adım ~1,4 ms. Açılıştaki `reset` ve `load` arayüz dışındadır.
- Saat 200 ms'dir. Zaman kuralları ±200 ms'ye kadar gecikebilir.
- Oturum yokken yapılan veritabanı değişikliği ancak giriş yapılınca gider. Program her açılışta girişi yeniden ister (oturum yalnız bellekte).

## Doğrulama (26 Eylül 2026, Linux)

- **Birim testleri, sunucusuz, mesaj mesaj (`apps/desktop/src/cloud/tests.rs`, 41):**
  - giriş: boş alan, reddedilen adres (hiçbir şey gönderilmez), sunucunun iletisi, geç yanıt, başarı; ayar ve parola hiçbir yerde değil;
  - katalog: sayfa, geç liste yanıtı, sonraki sayfa, hata, arama gecikmesi, kurum listesi;
  - açma: veritabanı projesi, geç ve durdurulan açılış, arada değişen çizim, arşiv, açık çizim sorusu;
  - kendiliğinden kayıt: 1 s ve 5 s, tek komut, aynı anahtarla yeniden gönderim, Ctrl+S, kalıcı ret iletisi;
  - çakışma: iki seçenek; başkalarının değişikliği ve silme; bekleme ve boş yanıttan sonra hemen yeniden sorma, bağlantı kaybında geri çekilme; yeniden açma (`resync`);
  - taslak ve ayrılma: taslak önce, yazılamayınca soru, görüntüleyici;
  - dosya projesi: revizyon, çakışma penceresi, iki seçenek;
  - yükleme: çalışma alanları, ret, aynı anahtar, açılış;
  - kopya: bağlantısız açılış ve bekleyen iş, ikinci pencerenin kilidi, bekleyen dosya kaydı ve çakışması, biten projenin işareti, kopyayı kaldırma sorusu, bağlantı noktası, bağlantı dönünce sonraki düzenleme.
- **Kasıtlı bozmalar** (her biri geri alındı, her biri kendi testinde düştü):
  - kopyanın adımı atlandı;
  - gönderimden önce taslak atlandı;
  - 5 s sınırı kaldırıldı;
  - geç açılış denetimi kaldırıldı;
  - çevrimdışı açılış yedeği kaldırıldı;
  - bağlantı dönünce itme koşulsuz yapıldı.
- **Gerçek sunucu** (`apps/desktop/scripts/cloud-live.sh`, `cloud/live_run.rs`): geliştirme veritabanında `kentosd` testin denetiminde başlar ve durur. Uygulamanın görevleri Iced gibi iş parçacıklarında koşar. İkinci düzenleyici `mehmet` web'in HTTP'siyle konuşur (`web-client.mjs`, `x-kentos-client: web`). 19 s'de geçti:
  1. giriş, yükleme (veritabanı), katalog, katalogdan açılış;
  2. düzenleme Kaydedildi'ye ulaşır;
  3. web istemcisinin düzenlemesi masaüstüne **~0,3 s**'de gelir (dört koşuda 312–339 ms);
  4. ikisi aynı noktayı değiştirir, çakışma penceresi, Benimkini koru;
  5. sunucu durur (Bağlantı yok — yeniden denenecek), geri gelir, iş gider;
  6. Kaydedilmedi'yken zorla kapatma (kapanış yolu çalışmadan bırakma), o arada web düzenlemesi; yeniden açılışta taslak geri konur, dayandığı sürüm geçildiği için çakışma, Benimkini koru;
  7. sunucu yokken “Bu cihazdaki projeler”, kopyadan açılış, düzenleme; sunucu dönünce iş gider ve sunucudan yeniden açılan projede vardır;
  8. dosya projesi: revizyon 2; web istemcisi revizyon 3 kaydeder; masaüstünün kaydı çakışma penceresini açar.
- **Koşunun bulduğu hatalar** (düzeltildi):
  - bağlantı dönünce kayıt kalıcı itiliyordu, sonraki düzenleme beklemeden gidiyordu (birim testi eklendi);
  - yeni adla yüklenen dosya projesi dosyanın içinde eski adı taşıyordu;
  - durum çubuğu taşıyordu;
  - özellikler paneli bulut projesine “Dosya: kaydedilmedi” diyordu.
- **Kontroller:**
  - `pnpm rust:test:desktop`: masaüstü 115, 5 atlanan: 4 ölçüm ve canlı koşu; UI 171, vitrin 55;
  - `pnpm rust:test`: 776, veritabanı testleri dahil; clippy; bağımlılık yönü temiz;
  - `cargo fmt --all --check`, `pnpm typecheck`;
  - `pnpm test`: 1379;
  - `pnpm inventory`, `pnpm inventory:check`.
- Görüntüler: `.run/shots/bulut-01…19-*.png` (yerel; depoda değil).
