# ADR 9004: Portalda e-imza: sunucunun imzalı isteği, kendi kaydına karşı doğrulama

- **Durum:** kabul edildi (bulut-portal dalı)
- **Tarih:** 2026-11-18
- **Bağlam belgesi:** ADR 9000 (bulut portalı); ADR 9002 (KentOS İmza: imza servisi, PIN penceresi, masaüstü);
  plan/bulut.md `BUL-21`; 5070 sayılı Elektronik İmza Kanunu (güvenli elektronik imza ıslak imzayla eşdeğerdir,
  m.5)

## §1 Bağlam

Sahibin isteği (18 Kasım 2026): platformdaki e-imza buraya alınsın. Mantık aynı kalsın, arayüz KentOS'un araç
takımıyla yapılsın. İmza hem masaüstüne hem buluta bağlansın.

İmza servisi, PIN penceresi ve masaüstü ADR 9002'nin konusudur. Bu karar sunucunun ve portalın tarafıdır: neyin
imzalanacağını kim söyler, sonuç nasıl doğrulanır, imza nerede durur.

Temel tehdit şudur: tarayıcı da yerel servis de sunucunun denetiminde değildir. Bir istemcinin “bu dosyayı imzaladım”
demesine, söylediği dosyaya, özete ya da imzalayana güvenilemez.

## §2 Karar

### Neyin imzalanacağını sunucu söyler

1. `POST /v1/dossiers/{id}/signing {"clientType","fileIds"}` ile dosyanın 1–50 belgesi seçilir.
2. Sunucu her belgenin **kendi deposundaki baytlarının** SHA-384 özetini okur. Özet parça parça hesaplanır; büyük
   belge belleğe alınmaz.
3. İsteği (`SignedRequestPayload`: işlem, nonce, 15 dakikalık süre, alıcı ve yayıncı adı) kendi Ed25519 anahtarıyla
   imzalar. Sonuç JWS'tir (`EdDSA`, `KSI-SR`).
4. İsteğin kendisini saklar (`kentos.signing_request`, migration 0049).

- **Anahtar:**
  - `KENTOS_IMZA_SIGNING_KEY` ya da `KENTOS_IMZA_SIGNING_KEY_FILE` (dosya, anahtarı süreç listesinin ve alt
    süreçlerin ortamının dışında tutar).
  - İsteğe bağlı `KENTOS_IMZA_ISSUER` ve `KENTOS_IMZA_AUDIENCE`.
  - Anahtar hiçbir yanıtta ve günlükte görünmez; `Debug` yalnız açık anahtarı yazar.
  - Anahtar yoksa uçlar anlaşılır bir 409 ile döner.
- **Açık anahtar** `GET /v1/signing/trust` ile okunur: `publicKey`, `issuer`, `audience`.
- **Kim imzalar:**
  - meslek mensubu tarafında belgeyi hazırlayan (büronun düzenleyicisi dahil);
  - belediyenin düzenleyicisi ve üstü.

  Görüntüleyici yalnız okur. Dosyanın sayfası bunu `canSign` ile söyler.

### Sonuç sunucunun kendi kaydına karşı sınanır

- Teslim `POST /v1/signing/{işlem}/result` ile yapılır. Gövde, imza servisinin verdiği sonucun (`BatchResult`)
  kendisidir.
- Sunucu sonucu ADR 9002'deki `verify_delivery` ile sınar. Bu, ortak ve sınanmış tek yerdir. Sınamanın girdileri:
  - sunucunun **sakladığı istek**, istemcinin gönderdiği değil;
  - belgenin **şimdiki** baytlarının özeti;
  - teslim edilen sertifikanın **kendi açık anahtarı**.

  Her imza isteğin içinden, en çok bir kez gelmeli; özet algoritması istektekiyle aynı olmalıdır.
- **İmzalayan, sertifikadan sunucunun okuduğu kişidir:** konu, yayıncı, seri numarası, SHA-256 parmak izi.
  İstemcinin yazdığı ad kullanılmaz.
- **Teslim kuralları:**
  - Yalnız isteği açan teslim eder. Bir başkasına istek “yok” görünür (RLS: `requested_by`).
  - İkinci teslim 409 döner, ilk hüküm geçerli kalır.
  - Süresi dolan ya da iptal edilen istek kabul edilmez.
  - Kısmi başarı geçerlidir (`partially`). Doğrulanmayan imza saklanmaz, yerine gerekçesi Türkçe döner.
- **Doğrulanan imza belgede saklanır** (`kentos.dossier_signature`). Saklananlar:
  - özet;
  - imza değeri;
  - sertifikanın DER'i (sonradan zincir ve iptal denetimi yapılabilsin diye);
  - belgenin o anki sha256'sı.

  `GET /v1/dossiers/{id}/signatures` iki tarafa da `stillMatches` alanını verir: belge sonradan değiştiyse imza onu
  söyler.
- **Geçmiş:** dosyanın geçmişine “… N dosyayı e-imzaladı (sertifika sahibinin adı)” yazılır.

### Yasal geçerlilik bu sürümde denetlenmez, söylenir

Kanıtlanan, bu sertifikanın anahtarının bu baytları imzaladığıdır. Şu ikisi henüz yapılmaz:

- sertifikanın Kamu SM kök sertifikalarına zinciri;
- iptal durumu (OCSP / SİL).

Platform da yapmıyordu. Her yanıt bunu söyler (`legalCheck: "not_done"`), portal da imza ayrıntısında yazar. Deneme
kartının imzası da doğrulanır; nitelikli olup olmadığı bu yüzden önemlidir ve sertifikadan okunur.

### Portal

- **Dosya sayfası:** paketlerin başında “E-imzala”, belgelerin yanında “e-imzalı: …” işareti; işarete tıklayınca imza
  ayrıntısı açılır.
- **İmza penceresinin akışı:**
  1. Sayfa KentOS İmza'yı arar (127.0.0.1:32147).
  2. Sunucu isteği hazırlar, sayfa onu servise verir.
  3. Servis isteği açar ve imzayı başlatır.
  4. Kişi kartı ve PIN'i **servisin kendi penceresinde** girer (yerli PIN kipi). PIN sayfaya hiç gelmez.
  5. Sayfa sonucu sunucuya teslim eder, hükmü gösterir, servise hükmü bildirir.
  6. Vazgeçince hem servisteki iş hem sunucudaki istek iptal edilir.
- **Hesabım › E-imza:**
  - servisin durumu (sürüm, PIN penceresi, takılı kart, deneme kartı);
  - bu tarayıcının servis adresi (varsayılan `http://127.0.0.1:32147`; makinede 32145'te başka bir imza kurulumu
    çalıştığı için ayrı port, ADR 9002);
  - servisin ayar dosyasına yazılacak satırlar: sunucunun açık anahtarı, yayıncı, alıcı ve portalın adresi
    (`KENTOS_IMZA_ORIGINS`).

### Uçlar

| Uç | Ne yapar |
|---|---|
| `GET /v1/signing/trust` | Sunucunun açık anahtarı, `issuer`, `audience` |
| `POST /v1/dossiers/{id}/signing` | İmzalı istek (201) |
| `POST /v1/signing/{işlem}/result` | Teslim ve hüküm |
| `POST /v1/signing/{işlem}/cancel` | İptal (204) |
| `GET /v1/dossiers/{id}/signatures` | Saklanan imzalar |

Masaüstü aynı uçları kullanır ve kendi içinde imzalar (ADR 9002). Yanıt tipleri `kentos-contracts`'ta tektir:
`SigningTrust`, `SigningRequestCreated`, `SigningVerdict`, `DossierSignatureView`.

## §3 Ölçülenler

- **Sunucu, gerçek PostgreSQL'de ve deneme kartıyla**
  (`tests/portal.rs` `a_dossier_s_files_are_signed_on_the_card_and_checked_against_the_server_s_own_record`):
  - anahtar yokken 409;
  - görüntüleyiciye 403;
  - sunucunun özeti imza servisinin hesabıyla aynı;
  - başkası teslim edemez (404);
  - iki belge `completed` 2/2, ikinci teslim 409;
  - iki taraf da okur, geçmiş “2 dosyayı e-imzaladı” yazar;
  - imzanın bir harfi değişince `partially` 1/2 ve doğrulanmayan saklanmaz;
  - iptal edilen istek kabul edilmez;
  - başka işlemin sonucu reddedilir.
- **Gerçek yerel servisle:** örnek veri (seed) imzaları tarayıcının yolundan geçirdi: servis ve deneme kartı ile
  istek → servis → kart → sunucu. İki dosyada altı imza kabul edildi.
- **Gerçek pencereyle:** portalın akışı, Xvfb üzerindeki PIN penceresiyle “PIN'inizi girin” adımına kadar yürüdü.
  Ekran görüntüleri `.run/gui-evidence/portal/96–100`.
- **Gerçek kartla, tek deneme** (sahibin takılı kartı ve verdiği PIN, geliştirme PIN kipi, betikle):
  - kartta tek bir nitelikli imza sertifikası;
  - `ecdsa-with-SHA384`, özet SHA-384;
  - sunucu `completed 1/1` dedi, `stillMatches: true`.

  Sertifika sahibinin bilgisi kayda geçirilmedi. PIN dosyası denemeden sonra silindi.

## §4 Açık kalanlar

- **Yasal geçerlilik:** zincir (Kamu SM kökleri) ve iptal durumu (OCSP / SİL); zaman damgası.
- **Kap biçimi:** imza şimdi ham değer ve sertifika olarak saklanıyor. Dışarıda doğrulanabilir bir CAdES (`.p7s`) ya
  da PAdES çıktısı ayrı bir iştir.
- **Resmî yazı ve e-tebligat:** kararın resmî yazısı (antetli PDF) ve tebligatı kurumun KEP / e-tebligat anlaşmasına
  bağlıdır.
- **Mobil:** imza mobilde ayrı bir iştir.
