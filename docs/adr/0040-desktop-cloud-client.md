# ADR 0040: Masaüstünün bulut istemcisi: aynı protokol, ayrı adapter

- **Durum:** kabul edildi (2026-09-26). Yön TODOS.md `SYNC-01`'den gelir: native istemci, web'in protokolünü ayrı bir adapter ile uygular; web'in eşitleme kodu Rust'a taşınmaz. Sahibin seçimi (26 Eylül): sunucu tarafından sonraki büyük iş masaüstü bulut istemcisi.
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** CLAUDE.md §0, §13, §21; ADR 0007 (kimlik), 0015 (proje erişimi), 0026 (kalıcı kimlikle eşitleme), 0031 (dosya projeleri), 0036 (içe aktarım)

## Bağlam

- Masaüstünde bulut yoktu: giriş, katalog, açma, kaydetme hiçbiri. Web'in bulut istemcisi yaklaşık 5000 satır TypeScript'tir (`apps/web/src/app/cloud/`).
- Veri modelleri ortaktır (`kentos-contracts`): nesne, katman, ayarlar, `.kcad` görüntüsü, komut zarfı, sunucu yanıtları. Ayrı olan, onları taşıyan istemci kodudur.
- Masaüstünün Iced yürütücüsü bir iş parçacığı havuzudur (futures `thread-pool`). HTTP istemcisi (reqwest) tokio ister.
- Sunucu, çerezli oturumda değişiklik yapan istekte `x-kentos-client: web` başlığını arıyordu (siteler arası istek sahteciliği). OpenID'nin `Bearer` yolu yalnız erişim belirteci alır.
- `ProjectInfo` saklama biçimini söylemiyordu. Bir projeyi kimliğiyle açan istemci onun dosya projesi olduğunu bilemezdi.

## Karar

### Kütüphane

- `crates/native/cloud` (`kentos-cloud`), mimari denetimde `native` grubu: arayüz, GPU, veritabanı sürücüsü, sunucu çatısı ve tarayıcı yok.
- Web'in bulut kodunun bir karşılığıdır, kopyası değildir. İkisini aynı tutan, sunucu ve ortak sözleşmelerdir.
- HTTP, kütüphanenin kendi tokio çalışma zamanında yürür (iki iş parçacığı, ilk istekte başlar, program boyunca yaşar). Her çağrı herhangi bir yürütücüde beklenebilen bir future döndürür; bırakılan future isteği durdurur. Masaüstü Iced'in yürütücüsünü değiştirmez.
- Kilit dosyasına yeni dış paket girmedi. `reqwest` ve `tokio` sunucu için zaten çalışma alanındaydı; `bytes` reqwest ile her ikilide vardır (bkz. docs/deps).

### Giriş ve oturum

- Yerel hesapla giriş: `POST /v1/auth/login`. Sunucunun verdiği oturum çerezi **yalnız bellekte** tutulur. Dosyaya ve günlüğe yazılmaz, hata ayıklama çıktısında gösterilmez (`Token(gizli)`, TODOS.md `SYNC-13`).
- Oturum her istekte çerez olarak ve `x-kentos-client: desktop` başlığıyla gider. Sunucu artık `web` ile `desktop`'u kabul eder. Kural aynıdır: başka bir sitenin sayfası özel başlık ekleyemez. Ad yalnız isteği kimin yaptığını söyler.
- 401 yanıtı oturumu unutturur. Yanlış parolayla giriş denemesi açık oturumu kapatmaz.
- **Şifresiz http yalnız bu bilgisayardaki sunucuya** (`localhost`, `127.0.0.1`, `::1`) kullanılır. Başka yerde oturum ağda okunabilirdi, bu yüzden https gerekir.
- Yönlendirme izlenmez: oturum, kimsenin seçmediği bir adrese gitmesin.
- Programı kapatıp açınca yeniden giriş gerekir. Oturumu işletim sisteminin anahtarlığında saklamak yeni bağımlılıktır ve sahibin kararını bekler.
- Masaüstünde OpenID girişi (RFC 8252: sistem tarayıcısı, PKCE, geri dönüş adresi) ayrı bir dilimdir.

### Açma

- Önce `GET …/projects/{proje}`. Yanıtta artık saklama biçimi de var (`ProjectInfo.storage`; alanı olmayan eski sunucu yanıtı veritabanı projesi sayılır).
- **Veritabanı projesi:** nesneler web'in sayfasıyla (2000) sayfa sayfa gelir. Her nesnenin kalıcı kimliği sunucudaki kimliğidir (ADR 0026) ve sunucudaki sürümü tutulur. Çizim sırası kimlik sırasıdır, web'de de öyle (ADR 0033).
- **Dosya projesi:** en yeni revizyon indirilir. SHA-256'sı hem `ETag` ile hem de revizyon listesiyle karşılaştırılır, ortak kodekle çözülür. Hiç kaydedilmemiş proje, üst verisiyle boş açılır.
- Çizim, teslim edilmeden önce eşzamansız iş parçacıklarının dışında kurulur ve doğrulanır (`Document::from_snapshot_v2`). Okunamayan proje açık çizimin yerine geçmez (CLAUDE.md §4.8).
- Boş sayfanın ardından yeni sayfa bildiren bir yanıt reddedilir; bitmeyen bir açılış olmaz.

### Dosya projesine kayıt

- Kayıt adımları:
  - yükleme, dosyanın boyutu ve SHA-256'sı bildirilerek açılır;
  - baytlar gönderilir;
  - `project.file.commit` yeni revizyonu, çizimin dayandığı revizyonun üstüne kaydeder (`expectedVersions["@file"]`).
- Arada başkası kaydettiyse çakışmadır. Hiçbir şey yazılmaz, iki revizyon da kalır. Sunucunun şimdiki revizyonu `conflicting_revision` ile okunur.
- Yanıtı gelmeyen adım yeniden gönderilir: aynı yükleme, aynı idempotency anahtarı. Bekleme 1 saniyeden başlayıp ikiye katlanır; en çok 5 deneme yapılır.
- **Yeni yol, `GET …/uploads/{yükleme}`:** yüklemenin durumunu (baytları geldi mi) yalnız sahibine verir. Sunucu alınmış baytları ikinci kez almaz. Bu yüzden baytlarının yanıtı kaybolan istemci, yeniden göndermeden önce durumu sorar.

### Çizimden yeni bulut projesi

- Proje, çizimin üst verisiyle açılır. Dosya ya ilk revizyonu olur (dosya projesi) ya da nesneleri olur (veritabanı projesi, `project.import`, ADR 0036).
- Oluşturma anahtarı, kullanıcının istediği her yükleme için birdir. Aynı anahtarla yeniden çağrılınca sunucu aynı projeyi verir. İçeriği zaten gelmiş proje ikinci kez doldurulmaz; içeriği olduğu gibi bildirilir (`replayed`).
- Sunucu çizimi kalıcı olarak reddederse (±10⁹ sınırı gibi, yeri `entities[i]`), boş kalan proje çöpe taşınır. Başarısız yükleme arkada proje bırakmaz. Sunucunun asla almayacağı büyüklükteki dosya hiç proje açtırmaz.

### Veritabanı projesinde değişiklikler (`ProjectSync`)

- Web'in izleyici ve otomatik kayıt kuralları (`tracker.ts`, `sync.ts`), masaüstünün kendi zamanlayıcılarıyla sürdüğü bir durum olarak yazıldı.
- Sunucunun her nesnede neye sahip olduğu tutulur: sürümü ve o sürümdeki içeriği.
  - Değişiklik düzenlemeleri yeniden oynatarak değil, karşılaştırmayla bulunur.
  - Kaydedilmiş hâle geri dönen geri alma hiçbir şey göndermez.
  - Geri alınan silme, nesneyi aynı kimlikle yeniden oluşturur.
- Neyin değişmiş olabileceği belgenin değişiklik günlüğünden (`changes_since`) okunur. Her yuvanın kimliği hatırlanır, böylece silinen nesnenin de adı bilinir.
- Aynı anda tek komut gider: `project.changes` v1, en çok 2000 nesne ve üst veri.
  - Yanıt gelmeyen komut, aynı anahtarla aynen yeniden gider; sunucu onu bir kez yazar.
  - Komut yoldayken yapılan düzenlemeler bir sonraki komutu bekler.
- Üst veri (ad, ayarlar, katman ağacı, stiller) `@project` sürümüyle gider. Bunu yalnız `project.edit` izni olan gönderir; diğerinin değişikliği bu cihazda kalır (`keeps_meta_here`).
- Retler web'deki gibidir:

  | Ret | Sonuç |
  |---|---|
  | Çakışma | Kullanıcı seçene kadar gönderim durur. |
  | Proje çöpte | Gönderim sona erer. |
  | Proje arşivlenmiş | Gönderim sona erer. |
  | Erişim kaldırılmış (404) | Gönderim sona erer. Komut anahtarını korur. |
  | Geçici hata | Artan beklemeyle aynı komut yeniden gider. |
  | Kalıcı ret (kilitli katman, eksik izin, biten oturum) | Nedeni gösterilir; değişiklik bekler. |

- Çakışmada "benimkini koru" (`keep_mine`) var: benim kopyam sunucunun şimdiki sürümünün üstüne gider.

### Başkalarının değişiklikleri (ikinci dilim, 26 Eylül)

- **Belgede dışarıdan değişiklik** (`Document::apply_external`, `kentos-domain`), web'in `applyExternal`'ı gibi:
  - başkasının değişikliği ve çakışmada seçilen sunucu kopyası kullanıcının düzenlemesi değildir: geri al geçmişine girmez, çizim kaydedilmemiş olmaz, revizyonu değişmez;
  - dokunduğu nesnelerin geri al ve yinele adımları yuva ya da kalıcı kimlikle atılır (web: `forgetHistoryOf`); geri alma başkasının yerine koyduğu hâli geri getirmez;
  - nesneler kalıcı kimlikle gelir: çizimde olan yuvasını korur, yeni olan sıradaki yuvayı alır;
  - aynı kimliği iki kez ya da boş kimliği anan değişiklik, bir de açık düzenleme sırasında gelen her değişiklik, hiçbir şey değişmeden reddedilir;
  - yeni katman ağacında kullanıcının etkin katmanı hâlâ katmansa kalır, değilse ilk katman etkin olur.

  Ortak fixture'lar bu işlemi kapsam dışı sayar (`fixtures/document-ops/README.md`); web'in kuralları masaüstünün kendi testinde yazılıdır (`crates/native/domain/tests/external.rs`).
- **Olaylar** (`ProjectSync::incoming`, `take_remote`, web'in `syncRemote.ts`'i gibi):
  - olaylar sırayla okunur; bu eşitlemenin kendi komutlarının olayları istek kimliğiyle atlanır;
  - adı geçen her nesne en yeni sürümüyle bir kez getirilir ve dışarıdan değişiklik olarak konur;
  - burada sunucuda olmayan değişikliği olan nesne ezilmez, çakışma olur: başkası değiştirdiyse `changed`, sildiyse `deleted`;
  - yeni üst veri önce gelir, çünkü yeni nesne onun getirdiği katmanda olabilir; burada gönderilmemiş üst veri varsa `@project` çakışmasıdır;
  - çizimde olmayan katmandaki nesne atlanır ve adıyla bildirilir;
  - projenin silinmesi eşitlemeyi hemen bitirir. Arşivlenmesi, ondan önceki olaylar geldikten sonra bitirir. Yetki olayı, hesabın yetkisinin yeniden sorulmasını ister (`set_access`).
- **Çakışmada “sunucudakini al”** (`take_theirs`): sunucunun kopyası gelir, silinen gider, üst veri çakışmasında projenin şimdiki üst verisi gelir. Gönderilecek bir şey kalmaz.
- **İzleme, sorarak** (`follow`): masaüstü proje açıkken birkaç saniyede bir `GET …/events?after=` ile sorar; sayfa doluysa hemen yeniden sorar. Adı geçen nesneler 500'erli kimlik listeleriyle getirilir.
  - Web'in canlı kanalı (WebSocket) aynı olayları yalnız daha erken getirir. wss için TLS bağlayıcısını elle kurmak gerekir; bu yüzden masaüstünde sonraki adımdır.
  - Sorarak izlemede imleç sürer, kaçırılan olaylar sırayla gelir, saklanmayan imleç `resync_required` (410) ile yeniden açtırır.
- **Sunucuda bulunan hata:** `events` modülünün yorumu, en yeni olaydan ileride kalan imlecin de (geri yüklenmiş veritabanı) `ResyncRequired` aldığını söylüyordu. WS aboneliği öyle davranıyordu, HTTP yolu (`events::after`) ise boş sayfa veriyordu.
  - Sonuç: sorarak izleyen istemci, kaçırdığı olayları hiç bilmeden bekleyecekti.
  - Düzeltme: `after`, imleci aboneliğin kuralıyla (`Bounds::can_continue`) denetliyor. `retention.rs`'e gerileme denetimi eklendi. Eski denetim geri konunca test düştü.

### Cihaz taslağı (üçüncü dilim, 26 Eylül)

- **Taslak** (`ProjectSync::draft`): gönderilmemiş her nesne, kalıcı kimliğiyle, dayandığı sunucu sürümüyle ve son hâliyle (silindiyse boş) yazılır; yanına gönderilmemiş üst veri yaması ve yoldaki komut (idempotency anahtarıyla) konur.
  - Biçim web'in taslağıdır (biçim 2, `drafts.ts`): aynı JSON alanları.
  - Görüntüleyicinin kendi düzenlemeleri taslağa girmez (web'de de).
- **Depo** (`DraftStore`): masaüstünün seçtiği klasörde, sunucu / hesap / `kurum_proje.json`. Masaüstünde `$XDG_DATA_HOME/kentos-cad/bulut-taslak` önerilir, kurtarma kopyalarının yanında.
  - Yazma kalıcıdır: geçici dosyaya yazılır, diske işlenir, eskisinin yerine adlandırılır, klasör işlenir. Çökme ya eskiyi ya yeniyi bırakır.
  - Okunamayan ya da başka hesabın taslağı silinmez; olduğu gibi `…#unreadable-<ms>.json` olarak kenara alınır ve kullanıcıya söylenir (web gibi).
  - Taslakta oturum, parola ya da adres yoktur; klasörü yalnız sunucunun adını taşır.
- **Geri koyma** (`ProjectSync::restore`), yeni açılışın hemen ardından:
  - yoldaki komut önce, aynı anahtarla gider; yazılmışsa sunucu kaydından yanıtlar (`replayed`) ve hiçbir şey iki kez yazılmaz;
  - kalan her değişiklik çizime geri alma adımı olmadan, gönderilmemiş iş olarak konur;
  - dayandığı sürümü sunucunun o arada geçtiği değişiklik çakışmadır; çizim kullanıcı seçene kadar bu cihazın kopyasını gösterir. Yoldaki komutun taşıdığı nesnede bu denetim yapılmaz: onun yanıtı sürümü getirir, kendi komutuyla çakışma olmaz (web'de burada gereksiz bir çakışma çıkabiliyordu);
  - iki tarafta da silinmiş nesne için yapılacak bir şey yoktur;
  - katmanı artık olmayan değişiklik gönderilmez, sonraki taslakta saklanır, kullanıcıya söylenir; aynı nesnenin burada düzenlenmesi onun yerini alır.

## Bu dilimde olmayanlar

- Masaüstü arayüzü (giriş penceresi, bulut kataloğu, açma, kaydetme, durum çubuğu, çakışma iletisi, taslağın ne zaman yazılacağı). Masaüstü ajanına gider (ADR 0041); bu kütüphaneyi kullanır.
- Masaüstünde canlı kanal (WebSocket): bugün sorarak izlenir, olaylar birkaç saniye geç gelir.
- Yetki değişikliğini izleme (web'de `accessWatch.ts`). `set_access` hazırdır, besleyen yoktur.
- Masaüstünde OpenID girişi ve oturumun anahtarlıkta saklanması.
- Parçalı ve sürdürülebilir yükleme (`SYNC-10`): 256 MiB sınırı sürer.

## Doğrulama (26 Eylül 2026, Linux)

- `crates/native/cloud`, 20 birim testi:
  - sunucu adresi kuralları; çerezden oturumun okunması ve gösterilmemesi;
  - hata kodları ve bekleme süreleri;
  - sunucu nesnelerinin numaralanması; hatalı kimliğin reddi;
  - izinler ve arşiv;
  - otomatik kayıt: oluşturma, değiştirme, silme, geri alma, yanıtı kaybolan komut, çakışma ve "benimkini koru", görüntüleyici, arşiv, üst veri, 2000'lik parçalar, retler;
  - kendi çalışma zamanında yürüme ve bırakılınca durma.
- `apps/api/src/http/native_tests.rs`: aynı istemci, bu sunucuya gerçek TCP soketi ve gerçek veritabanıyla (`KENTOS_TEST_DB=required`), 6 test:
  - giriş, yanlış parola, masaüstü başlığıyla komut, çıkış ve unutulan oturum;
  - dosya projesi:
    - çizimden 1. revizyon; aynı anahtarla tekrar aynı proje, ikinci kez doldurulmadan;
    - açılışta nesne nesne aynı çizim;
    - 2. revizyon;
    - eski revizyona dayanan kayıt çakışma (`conflicting_revision` = 2), hiçbir şey yazılmadı;
  - yükleme durumu: baytlar gelmeden `received: false`, sonra `true` ve nesne sayısı; ikinci gönderim reddedildi; başkasının yüklemesi 404;
  - veritabanı projesi:
    - çizimden 13 nesne, açılışta nesne nesne aynı, hepsi 1. sürüm;
    - değiştirme, silme, yeni nesne ve katman adı tek komutta gitti; yeniden açılınca sunucudaki çizim yereldekiyle nesne nesne aynı;
    - kilitli katmandaki nesne kalıcı ret: durum hata, değişiklik bekliyor, geri alınınca gönderilecek bir şey kalmadı;
  - iki düzenleyici aynı nesnede: ikinciye çakışma (sunucunun sürümü 2); "benimkini koru" ile gitti, sunucu onunkini tutuyor;
  - sunucunun almadığı çizim (`entities[13]`): proje "Projelerim"de yok, çöpte.
- Kasıtlı bozmalar:
  - masaüstü başlığı kabul edilmeyince 6 testin 6'sı düştü;
  - `ProjectInfo.storage` hep "database" dönünce dosya projesi testi düştü;
  - değiştirmede beklenen sürüm sabit "1" olunca iki düzenleyici testi düştü.

  Hepsi geri alındı.
- `pnpm rust:test` (veritabanı zorunlu, clippy `-D warnings`, bağımlılık yönü: 20 crate), `pnpm typecheck`, `pnpm test` (1303 test) geçti.

### İkinci dilim

- **`kentos-domain`, `tests/external.rs` (8 test):**
  - başkasının nesnesi geri al adımı ve kaydedilmemiş işareti olmadan gelir, izleyenler onu görür;
  - dokunduğu nesnelerin geri al adımları atılır, öbürleri kalır;
  - burada silinip başka yerde geri getirilen nesnenin geri alması düşer;
  - silinenler gider, bilinmeyenler atlanır;
  - bozuk değişiklik ve açık düzenleme sırasında gelen değişiklik hiçbir şeyi değiştirmez;
  - yeni üst veride etkin katman kuralı;
  - kaydedilmemiş çizim kaydedilmemiş kalır;
  - aynı kopya hiçbir şeyi değiştirmez.
- **`kentos-cloud`, 6 yeni birim testi:**
  - başkasının nesneleri gelir, kendi olaylarımız atlanır, gelen geri gönderilmez;
  - iki tarafta değişen ve burada değişip orada silinen nesneler çakışma olur, “sunucudakini al” ile çözülür;
  - üst veri önce gelir, yeni katmandaki nesne onunla gelir, katmansız nesne adıyla atlanır;
  - üst veri çakışması iki seçenekle çözülür;
  - silinen ve arşivlenen proje;
  - açık düzenleme sırasında bekleme ve yetki olayı.
- **Cihaz taslağı:** `kentos-cloud`'da 4 taslak testi ve 3 depo testi:
  - gönderilmemiş iş taslağa girer, web'in biçimindedir, yeni açılışta geri gelir ve gönderilir;
  - yoldaki komut aynı anahtarla yeniden gider, ardından gelen düzenleme kendi komutuyla çakışmaz;
  - sunucunun geçtiği değişiklik çakışma, katmansız değişiklik saklanır;
  - görüntüleyicinin düzenlemesi taslağa girmez;
  - depo sunucu, hesap ve projeye göre ayırır; okunamayan ya da başka hesabın taslağı kenara alınır; dışarıdan gelen adlar klasörün dışına çıkamaz.

  Gerçek sunucu testi `a_lost_answer_survives_the_program_ending_through_the_device_draft`:
  - yazılan ama yanıtı kaybolan komut ve sonraki düzenleme taslakla diske yazılır ve geri okunur;
  - yeniden açılışta sunucu aynı anahtara kaydından yanıt verir (`replayed`); yeni nokta bir kez vardır, sunucudaki çizim cihazdakiyle nesne nesne aynıdır.
  - Kasıtlı bozma: taslak yoldaki komutu yazmayınca test düştü; geri alındı.
- **Gerçek sunucu testi `other_editors_changes_come_in_by_following_the_events`:**
  - Ayşe'nin değiştirme, silme, ekleme ve katman adı Dilek'in çizimine sorarak gelir; iki çizim nesne nesne aynıdır, Dilek'in çizimi kaydedilmemiş olmaz, geri gönderecek bir şey yoktur;
  - Dilek'in kendi komutu olaylarda atlanır, Ayşe'ye gelir;
  - ileride kalan imleç `resync_required` alır.
