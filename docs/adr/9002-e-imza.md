# ADR 9002: E-imza (KentOS İmza)

- **Durum:** kabul edildi (bulut-portal dalı)
- **Tarih:** 2026-11-18
- **Bağlam belgesi:** ADR 9000 (bulut portalı), 9001 (masaüstünde portal dosyaları), 9004 (portalın e-imza tarafı:
  sunucunun uçları ve web sayfası), 8000 (gerçek pencerede kabul); DESIGN.md §7.9; docs/deps/README.md

## §1 Bağlam

Sahibin isteği (18 Kasım 2026): *“üst dizinde kentos platform uygulamasında e imza var onu bu tarafa kopyala ve al;
mantık aynı olacak ama arayüz değişecek ve bizim sistem uygun şekilde iced veya gpui olacak, onu uyarla.”* Aynı gün:
*“Hem masaüstü hem de cloud uygulamasına entegre edeceksin.”* İstek, e-imzanın bağımlılıklarının da onayıdır.

Tarayıcı akıllı karta erişemez. Portalda imza, kullanıcının bilgisayarında yalnız geri döngüde dinleyen bir servisten
(KentOS İmza) geçer; masaüstü ise karta kendi sürecinden konuşur. İki yolda da yetkiyi veren şey sunucunun Ed25519 ile
imzaladığı istektir, imzanın hukukî dayanağı kullanıcının kartıdır ve sunucu teslim edilen imzayı kendi deposundaki
baytlara karşı yeniden doğrular.

## §2 Kaynak

KentOS platformunun `imza/` çalışma alanı, commit `32a230e2` (31 Ağustos 2026). Belgeleri: `CLAUDE.md`,
`docs/architecture.md`, `docs/environment.md`, `docs/kentos-integration.md`, `docs/signing-*.md`, `docs/adr/001–017`.
Platformun şartnamesi (`imza/sartname.md`) kopyalanmadı; kodun yorumlarındaki bölüm numaraları (§13, §46 …) bu yüzden
çıkarıldı, gerekçeler cümle olarak kaldı. Platformun belgelerindeki kişisel veriler (sertifika sahibi, kimlik numarası,
kart seri numarası) ve anahtarlar bu depoya hiçbir yerde girmedi.

| Platform | KentOS CAD | Ne |
|---|---|---|
| `crates/kentos-imza-core` | `crates/imza/core` | `KSI-*` hata taksonomisi; `retryable` koddan türer |
| `crates/kentos-imza-domain` | `crates/imza/domain` | Batch → Package → File, üç durum makinesi; **yeni:** tel, kayıt |
| `crates/kentos-imza-security` | `crates/imza/security` | Ed25519 imzalı istek, replay, özet; **yeni:** `issue` |
| `crates/kentos-imza-signature` | `crates/imza/signature` | defter, doğrulama; **yeni:** kart, motor, X.509, teslim |
| `crates/kentos-imza-audit` | `crates/imza/audit` | hash zincirli denetim kaydı |
| `crates/kentos-imza-pkcs11` | `crates/imza/pkcs11` | PKCS#11: cihaz, sertifika, tek oturumda imza |
| `crates/kentos-imza-api` | `crates/imza/api` | yerel HTTP + WebSocket, `LoopbackAddr`, `Origin`/`Host` kapıları |
| `apps/kentos-imza-service` | `apps/imza-service` | servis ikilisi (pencere açmaz) |
| `apps/kentos-imza-agent` (egui) | `apps/imza-agent` + `crates/imza/ui` (Iced) | PIN penceresi, **yeniden yazıldı** |

## §3 Aynı kalanlar

- **Hata taksonomisi:** kod numaraları sözleşmedir; `retryable` çağıran seçmez (yanlış PIN tekrar denenmez); kullanıcı
  iletisi Türkçe, teknik ileti olduğu gibi (`CKR_*`).
- **Model ve durumlar:** evrak türü serbest metin, rol üç değer; geçiş bir fonksiyondur (`try_to`), n×n çapraz taranır;
  `Signing → Signed` yok (doğrulama atlanmaz), `COMPLETED` yalnız sunucunun teslim cevabıyla gelir.
- **İmzalı istek:** JWS-compact, `alg` başlıktan seçilmez, önce imza sonra gövde; `transactionId` ve `nonce` tek
  kullanımlık.
- **Kart:** sertifika PIN'siz okunur; bir batch tek oturum, tek `C_Login`; EC'de özet yazılımda hesaplanıp ham
  `CKM_ECDSA` ile imzalanır, RSA'da DigestInfo önekini çağıran ekler; PIN kararı `C_Login`in dönüş koduna dayanır
  (`user PIN locked` bayrağı ölçülen kartta yanıltıcıydı); NUL dolgulu alanlar kırpılır; karta eşzamanlı çağrı
  kilitlidir; PIN, kart ya da cihaz hatasında batch durur, kısmi başarı meşrudur.
- **Yerel doğrulama:** her imza açık anahtara karşı hemen doğrulanır; zincir ve iptal denetlenmediği için en iyi sonuç
  `VALID_WITH_WARNINGS`'tir ve yapılmayan kontroller yazılır.
- **Yerel servis:** adres bir tiptir (`0.0.0.0` yazılamaz); `Origin` beyaz listesi ve `Host` denetimi; `/pin` ve
  `/devices/pin-check` `Origin` taşıyan (tarayıcıdan gelen) her isteği reddeder; on bir uç ve olay akışı; yapılandırma
  ortamdan ve `ANAHTAR=değer` dosyasından, ortam kazanır, süreç ortamı değiştirilmez.
- **Ajan:** imza isteğinde öne gelir, takılı bütün kartları ve sertifikaları seçtirir (oradaki seçim bağlayıcıdır),
  PIN'i önce imzalamadan sınar (yanlışsa aynı pencerede yeniden sorar), sonra servise verir; kapatmak süreci değil
  pencereyi gizler, bekleyen bir istekte kapatmak ya da Vazgeç bir cevaptır (istek hemen iptal olur, 180 saniye
  beklenmez); başarıdan 3 saniye sonra gizlenir; tek örnektir (kilit bir port); oturum açılışında kullanıcı
  kapsamında başlar (`--autostart`).

## §4 Değişenler ve nedenleri

1. **Pencere Iced 0.14 ve KentOS UI ile.** Platformun ajanı eframe/egui idi. KentOS'un masaüstü Iced'dir; imza penceresi
   aynı bileşenlerle (`Dialog`, `Table`, `Select`, `Pairs`, `InputField::secure`, `Banner`, ilerleme çubuğu) ve aynı
   temayla çizilir, çünkü kullanıcı PIN'ini “bu KentOS mu?” diye sormadığı bir pencereye yazmalı. Pencere ayrı bir
   crate'te (`kentos-imza-ui`): ajan onu kendi işletim sistemi penceresinde (`Dialog::window`), masaüstü portal
   penceresinin üstünde (`Dialog::modal`) gösterir. Pencere kartla konuşmaz; istediği işi ev sahibine verir. Platformun
   KDE Wayland'de eframe'in kare üretmediği için X11'i seçen kuralı taşınmadı: o egui'nin ölçümüydü, Iced'in arka ucunu
   masaüstüyle aynı winit seçer.
2. **Sistem tepsisi ve masaüstü bildirimi yok.** Platformda `tray-icon` (GTK 3 bağlayıcıları bakımda değil) ve
   `notify-rust` vardı; bu bağımlılıklar alınmadı. Yerine: ajanı ikinci kez başlatmak gizli pencereyi öne getirir
   (ikinci örnek kilit portuna `goster` yazar), sonuç pencerede ve portalda görünür. §8'de açık iş.
3. **PIN varsayılan olarak ajanın penceresinde** (`native`). Platformda varsayılan tarayıcıydı (ajan kurulu olmayabilir
   diye); burada ajan servisle birlikte gelir ve PIN tarayıcıdan hiç geçmez. `KENTOS_IMZA_PIN_ENTRY=client`
   geliştirme için kalır, `/info` onu `insecurePinEntry: true` diye söyler.
4. **Port 32147, kilit 32148, dosya `kentos-cad/imza.env`.** Geliştirme makinesinde platformun kurulumu 32145 ve
   32146'yı kullanıyor ve `kentos-imza/config.env`'de başka bir güven anahtarı taşıyor (ölçüldü: ilk denemede servis onu
   okudu). Ajanın oturum açılışı girdisi de kendi adını taşır (`kentos-cad-imza.desktop`).
5. **Tek motor, tek tel sözleşmesi.** Platformda imza döngüsü servisin durumuna bağlıydı. Döngü
   `kentos-imza-signature`'a taşındı (`sign_batch` + `RunSink`), kart bir trait oldu (`Token`, `TokenSession`; PKCS#11
   ve deneme kartı uygular), batch kaydı alan modeline geçti (`BatchRecord`). Servis, masaüstü ve sunucu aynı tiplerle
   konuşur (`kentos_imza_domain::wire`).
6. **Mekanizma karttan seçilir.** Oturum açılırken kartın mekanizma listesi okunur; anahtarın türüne uyan ham
   mekanizma yoksa imza başlamaz ve kartın sunduğu liste söylenir. İmza sırasında kart çıkarılırsa hata artık
   `KSI-1102` (cihaz) olur ve batch durur; platform her `C_Sign` hatasını `KSI-1401` saydığı için durma kuralına hiç
   ulaşmıyordu.
7. **Paket `VERIFYING` üzerinden `SIGNED` olur.** Platformda paket `SIGNING`den doğrudan `SIGNED`e çekiliyordu; kendi
   durum makinesi bunu reddediyor ve sonuç `let _ =` ile atılıyordu: paketler hiç `SIGNED` olmuyordu. Taşırken bulundu;
   testler paketi `SIGNED` görüyor.
8. **Süre ayrıştırılarak karşılaştırılır.** Platform RFC 3339 dizelerini sözlük sırasıyla karşılaştırıyordu; `+03:00`
   ofsetli ya da kesirsiz bir damga süreyi kaydırırdı.
9. **Sonuç imzalayan sertifikayı taşır** (`signer.certificateDer`). Platformda tarayıcı açık anahtarı ayrıca ekliyordu.
   Sunucu imzayı sertifikanın kendi anahtarına karşı doğrular: `kentos_imza_signature::delivery::verify_delivery`.
10. **Zarfı imzalayan kod Rust'ta, yalnız sunucuda** (`kentos-imza-security`'nin `issue` özelliği; platformda sunucu
    Node.js'ti). Ajan, servis ve masaüstü bu kodu derlemez.
11. **Deneme kartı** (`kentos-imza-signature`'ın `mock-token` özelliği): iki kart, P-384 ve P-256 anahtarlı, elle
    yazılan DER ile kendinden imzalı, konuları “(gerçek değil)” diye biten sertifikalar ve süresi geçmiş bir sertifika;
    PIN `1234`, üç yanlışta kilit. Özellik varsayılan olarak kapalıdır; açıkken `/info` `mockToken: true` döner ve
    pencere “Deneme kartı” uyarısını gösterir.
12. **Sürümler kilitteki sürümler:** `p256`/`p384` 0.13 (kaynakta 0.14), `ed25519-dalek` 2.2 (3.0), `base64` 0.22
    (0.23), `tower-http` 0.7 (0.6), `reqwest` 0.13 (0.12). Kullanılan API aynıydı. Yeni olanlar: `cryptoki` 0.12.1,
    `x509-parser` 0.18.1 ve parçaları (docs/deps/README.md).

## §5 Tel sözleşmesi

Sunucunun tarafı ADR 9004'tedir; aşağıdaki biçimler `kentos_imza_domain::wire`'dadır ve testlerle sabittir.

- **Zarf:** `B64URL(başlık).B64URL(gövde).B64URL(imza)`, dolgusuz base64url. Başlık `{"alg":"EdDSA","typ":"KSI-SR"}`
  (isteğe bağlı `kid`); imza Ed25519 (RFC 8032), `başlık.gövde`nin ASCII baytları üzerine.
- **Gövde** (`SignedRequestPayload`, camelCase): `transactionId`, `batchId`, `userId`, `sessionId`,
  `signatureMethod` (`ELECTRONIC_SIGNATURE`), `clientType` (`WEB` · `DESKTOP` · `MOBILE`), `packages[]` (`id`, `title`,
  `referenceNumber?`, `documentType?`, `metadata`, `files[]`: `id`, `name`, `role`, `mimeType`, `size`,
  `digestAlgorithm` `SHA-256|384|512`, `digest` küçük harf onaltılık, `signingRequired`), `createdAt`, `expiresAt`
  (RFC 3339), `nonce` (en az 128 bit), `issuer` (varsayılan `kentos-server`), `audience` (`kentos-imza`).
- **Güven anahtarı:** sunucu Ed25519 çiftini üretir (`ServerKey::generate`, ya da `openssl genpkey -algorithm ed25519`);
  özel yarısı yalnız sunucuda (`KENTOS_IMZA_SIGNING_KEY` ya da dosyası `KENTOS_IMZA_SIGNING_KEY_FILE`: PEM, base64
  PKCS#8 DER ya da base64 ham 32 bayt); açık yarısı ham 32 baytın standart base64'ü (44 karakter), servisin
  `KENTOS_IMZA_TRUST_KEY`'i. Masaüstü anahtarı `KENTOS_IMZA_TRUST_KEY` ile sabitlenmişse onu kullanır (sunucunun
  bildirdiği farklıysa imza başlamaz), yoksa sunucunun `GET /v1/signing/trust` cevabını: aynı oturumdan geldiği için bu
  yerel kurcalamaya ve tekrara karşı korur, tamamen ele geçirilmiş bir TLS kanalına karşı korumaz.
- **Sonuç** (`BatchResult`): `batchId`, `transactionId`, `status`, `signatureMethod`, `clientType`, `total`, `signed`,
  `failed`, `signer` (`certificateDer` base64 DER, `publicKeyDer`, `subject`, `issuer`, `serialNumber`,
  `fingerprintSha256`, `keyAlgorithm`, `keyParameters`, `signatureAlgorithm`, `qualified`, `notBefore`, `notAfter`),
  `results[]` (`fileId` ile birlikte imza: `signatureValue` EC'de ham `r‖s` (P-384'te 96 bayt), RSA'da PKCS#1 v1.5;
  `digestAlgorithm`, `signatureAlgorithm`, `signedAt`, istemcinin `verification`'ı), `lastError`.
- **Sunucunun denetimi** (`verify_delivery`): işlem kimliği aynı mı; her imza istekte imzalanması gereken bir dosyaya
  mı ve bir kez mi; özet algoritması istekteki mi; dosyanın bugünkü özeti istekteki mi (değişmişse “Belge, imza isteği
  üretildikten sonra değişmiş”); imza teslim edilen sertifikanın kendi açık anahtarıyla doğrulanıyor mu. İmzalayan,
  sertifikadan sunucunun kendi okuduğudur.
- **Yerel servisin akışı (portal):** `POST /api/v1/signing/batches {signedRequest}` → `…/open` → `…/sign {}` (ajan
  penceresi öne gelir) → olay akışı ya da `GET …/{id}` ile `DELIVERING` beklenir → `GET …/result` sunucuya iletilir →
  sunucunun cevabıyla `POST …/acknowledge {accepted, reason}`. İzinli kaynak `KENTOS_IMZA_ORIGINS`
  (`http://127.0.0.1:8797`); yerel ağ izni için CORS `Access-Control-Allow-Private-Network` döner.

## §6 Ajan ve imza penceresi

Pencere yukarıdan aşağı: deneme kartı uyarısı (varsa), evrak, sayısı ve belge sayıları; belge tablosu (ad, boyut,
özet (SHA-384) kısaltılmış, hâl: İmzalanacak, İmzalanıyor, İmzalandı, İmzalanamadı, Yalnız bilgi); imzalayacak
sertifika seçimi ve ayrıntısı (kart, düzenleyen, geçerlilik, anahtar, nitelik); maskeli PIN alanı; uyarı; işlem
(ilerleme çubuğu ve canlı akış). Alt çubukta solda o anki durum, sağda Vazgeç ve İmzala. Sahibin adı sertifikanın
`CN`'idir; konunun tamamı (kimlik numarası dahil) gösterilmez. Akış satırları duvar saati yerine ilk olaydan geçen
süreyi yazar (`+1.4 sn`): damgalar sunucunun UTC'sidir ve çok iş parçacıklı bir süreçte yerel saat dilimi güvenle
okunamaz.

Kart okunamazsa ya da sertifika yoksa uyarı “Yeniden tara” eylemini taşır. PIN alanı kartlar okununca ve yanlış
PIN'den sonra odak alır: kullanıcı fareye uzanmadan yazar (gerçek pencere senaryosu PIN alanına tıklamaz).

Ajanın günlük süzgeci varsayılan olarak `warn,kentos_imza_agent=info`'dur (`RUST_LOG` değiştirir): Iced'in pencere
katmanı `info`da her pencere açılışında simgenin bütün baytlarını satır satır yazıyordu; bir kabul koşusunda günlük
126 MB'tı, süzgeçle 1,3 KB. Windows'ta ajanın sürüm derlemesi bir pencere programıdır: oturum açılışında yanında konsol
açılmaz (`--durum`un çıktısı da orada görünmez).

## §7 Masaüstü

Portal dosyaları'nda (ADR 9001) dosyanın adımları satırının solunda **E-imza** anahtarı; kapalıyken pencere kartsız
olduğu gibi çalışır (anahtar pencere kapanınca hatırlanır, oturum kapanınca unutulur). Açıkken dosya satırları kayıtlı
imzaları gösterir; sunucu dosyayı bu kullanıcının imzaladığını söylüyorsa (`DossierDetail.canSign`: paketi hazırlayan
taraf, büronun düzenleyenleri, belediyenin düzenleyenleri ve üstü; ADR 9004) ve hazırlanan pakette dosya varsa
**E-imzala**, Gönder adımı izinliyse **E-imzayla gönder** gelir. Yalnız mesaj da (`EsignStart`) satırın sunmadığını
başlatmaz:

1. `GET /v1/signing/trust`, sonra `POST /v1/dossiers/{id}/signing {clientType: DESKTOP, fileIds}`;
2. zarf güven anahtarıyla doğrulanır, süreç ömrünce tek replay kapısından geçer, batch açılır;
3. kartlar ve sertifikalar ayrı bir iş parçacığında okunur (PKCS#11 kütüphanesi süreçte bir kez yüklenir:
   `KENTOS_IMZA_PKCS11` ya da sistemdeki bilinen sürücü);
4. İmzala: motor ayrı bir iş parçacığında tek oturumda imzalar ve yerelde doğrular; olaylar pencereye akar; PIN yalnız
   o iş parçacığına geçer. Yanlış PIN imzayı başlatmaz, aynı pencerede yeniden sorulur;
5. `POST /v1/signing/{tx}/result`; sunucunun hükmüyle batch `COMPLETED` (ya da kısmen) olur;
6. Kapat: dosyanın imzaları okunur (`GET /v1/dossiers/{id}/signatures`; dosyanın satırında “E-imzalı: …, gün saat
   (sertifika zinciri ve iptal durumu denetlenmedi)”, saat cihazınki, dosya imzadan sonra değiştiyse kırmızı); E-imzayla
   gönder'de Gönder adımının sorusu açılır. Esc ve karartılmış alana basmak Kapat'tır. Vazgeç
   `POST /v1/signing/{tx}/cancel`; sunucuya hiç sonuç gitmeden kapanan istek de (zarf doğrulanamadı, hiçbir belge
   imzalanamadı, teslim koptu) aynı uçla bırakılır, süresi dolana kadar askıda kalmaz. Kart yüklenemezse açılmış batch
   korunur: kart takılıp yeniden denenir.

İstemci `kentos-cloud`'un `api/esign.rs`'idir; cevap tipleri portalın sözleşmeleridir (`kentos_contracts::signing`).
Deneme derlemesi (`--features esign-mock`) `KENTOS_IMZA_DENEME_KARTI=1` ile deneme kartını,
`KENTOS_IMZA_DENEME_SUNUCU=1` ile süreç içi deneme sunucusunu (zarfı imzalar, teslimi `verify_delivery` ile sınar)
kullanır.

## §8 Henüz olmayanlar

Platformun kendi “bugün yapılmıyor” listesi aynen geçerlidir:

- **Kalıcı depolama ve çökme kurtarma:** yerel servisin batch'leri ve replay kapısı bellektedir; servis yeniden
  başlarsa yarım oturum kaybolur (sunucudaki kayıt kalıcıdır).
- **Güvenilir teslim (outbox):** teslim tarayıcıdan ya da masaüstünden yapılır; arada kopan bir teslim `DELIVERING`de
  kalır ve bunu söyler.
- **PAdES / CAdES:** üretilen şey ham imza değeridir (`CMS`, düzey `RAW`); imzalı öznitelikler yok.
- **Sertifika zinciri, OCSP/CRL, zaman damgası:** doğrulanan şey “bu sertifikanın anahtarı bu baytları imzaladı”dır,
  hukukî geçerlilik değil; deneme kartının imzası da doğrulanır. Sunucu bunu `legalCheck: not_done` ile söyler.
- **Mobil imza:** defterde var, kapalı (`KSI-2001`).
- **Güncelleyici, kurulum aracı, servis ↔ ajan IPC'si:** yok. Platformun kurulum betikleri (systemd kullanıcı birimi,
  LaunchAgent, Zamanlanmış Görev) taşınmadı; ajan `--autostart` ile kendi girdisini kurar.
- **Tepsi simgesi ve masaüstü bildirimi:** §4.2.
- **Masaüstünün gerçek pencere kabulü:** imza yolu ekransız sınandı (zarf, deneme kartı, sunucunun doğrulaması,
  resimler); gerçek pencerede, gerçek fare ve klavyeyle denenmedi. Bir bulut oturumu, imza anahtarlı bir API
  (`KENTOS_IMZA_SIGNING_KEY`) ve dosyalı bir başvuru ister.
- **Windows ve macOS:** kod iki platformu da gözetir (`CREATE_NO_WINDOW`, `%APPDATA%`, sürücü yolları, konsolsuz ajan)
  ama bu ortamlarda çalıştırılmadı; Windows için yalnız derleme denetimi yapıldı (§10).

## §9 Derleme, çalıştırma, sınama

```bash
# Paylaşılan crate'ler, servis ve deneme kartıyla bütün testler
scripts/dev/svc.sh heavy cargo test -p kentos-imza-core -p kentos-imza-domain -p kentos-imza-security \
  -p kentos-imza-signature -p kentos-imza-audit -p kentos-imza-pkcs11 -p kentos-imza-api -p kentos-imza-service \
  --all-features
# Pencere ve resimleri (.run/gui-evidence/imza/, Git'e girmez)
RUST_MIN_STACK=67108864 scripts/dev/svc.sh heavy cargo test -p kentos-imza-ui -- --include-ignored
# Masaüstünün süreç içi yolu; resimleri (.run/gui-evidence/imza/masaustu/) ayrıca `-- --ignored esign_screens` ile
RUST_MIN_STACK=67108864 scripts/dev/svc.sh heavy cargo test -p kentos-desktop --lib cloud::portal::esign_tests
# Gerçek pencerede, gerçek fare ve klavyeyle (Xvfb; deneme kartı)
scripts/usage/real/imza.sh .run/gui-evidence/imza/gercek acik
```

Servis: `KENTOS_IMZA_TRUST_KEY=<açık anahtar> KENTOS_IMZA_ORIGINS=<portal adresi> kentos-imza-service`
(`KENTOS_IMZA_PKCS11=/usr/lib/libakisp11.so` ya da sistemdeki bilinen sürücü). Ajan: `kentos-imza-agent`
(`--autostart` oturum açılışına ekler, `--gizli` penceresiz başlar, `KENTOS_IMZA_URL` başka bir servis).

**Gerçek kart, tek imza** (PIN yalnız ortamdan okunur, tek `C_Login`, iki belge, yerel ve sunucu doğrulaması):

```bash
read -rs KENTOS_IMZA_TEST_PIN && export KENTOS_IMZA_TEST_PIN
KENTOS_IMZA_PKCS11=/usr/lib/libakisp11.so scripts/dev/svc.sh heavy cargo test -p kentos-imza-pkcs11 \
  --test real_card -- --ignored --exact gercek_kartla_uctan_uca_imza --nocapture
unset KENTOS_IMZA_TEST_PIN
```

PIN istemeyenler: aynı komutta `--exact …` yerine `okunur` süzgeci (cihaz, sertifika, mekanizmalar).

## §10 Ölçümler (18 Kasım 2026)

- **Testler:** paylaşılan crate'ler ve servis 121 (deneme kartıyla: zarf → imza → teslim doğrulaması; değişmiş belge,
  bozuk imza, yabancı dosya ve başka işlem reddedilir; yanlış PIN batch'i başlatmaz); pencere 11 ve bir resim testi;
  Portal dosyaları'nın 21 testi, e-imzanın 3'ü onların içinde (anahtar, `canSign` ve teklif; zarf → kartlar → yanlış PIN
  aynı pencerede → imza → sunucunun doğrulaması → Gönder'in sorusu; Vazgeç ve Esc pakete dokunmaz, sunucudaki isteği
  bırakır) ve bir resim testi. Gerçek kart testleri `#[ignore]` (5). Deneme sunucusu her test iş parçacığında ayrıdır,
  isteklerinin numarası süreçte tektir: süreç ömrünce tek olan replay kapısı ikinci testin ilk isteğini tekrar sayıp
  reddetmişti (ölçüldü).
- **Gerçek pencere** (`scripts/usage/real/imza.sh`, Xvfb 1440×900, ayrı ev dizini, ayrı port): istek gelince pencere öne
  geldi; PIN alanına tıklanmadı, odak kartlar okununca ve yanlış PIN'den sonra kendiliğinden oradaydı; yanlış PIN aynı
  pencerede uyarıyla yeniden soruldu; doğru PIN'le üç belge imzalandı, batch `DELIVERING`, teslim cevabından sonra
  `COMPLETED`, pencere 3 saniye sonra gizlendi; ikinci istekte Vazgeç batch'i hemen `CANCELLED` yaptı. Açık ve koyu
  temada aynı. Resimler `.run/gui-evidence/imza/gercek/`.
- **Masaüstü** (ekransız çizici, 1440×900, `.run/gui-evidence/imza/masaustu/`): anahtar ve E-imzayla gönder, Portal
  dosyaları'nın üstünde imza penceresi ve sonucu, dosya satırlarında cihazın saatiyle “E-imzalı”; açık ve koyu. Gerçek
  pencerede denenmedi (§8).
- **Windows hedefi** (`cargo check --target x86_64-pc-windows-msvc`): pkcs11, api, servis ve pencere derleniyor; ilk
  denemede `CK_ULONG`'un Windows'ta 32 bit olması pkcs11'de iki satırı kırdı, düzeltildi. Ajan ve masaüstü bu
  makinede o hedefe denetlenemedi: `reqwest`'in TLS'i `aws-lc-sys`'i MSVC C derleyicisiyle ister (ADR 8060 §7'deki
  durumun aynısı).
