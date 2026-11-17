# KentOS Bulut API kılavuzu

KentOS'un bulut sunucusu (`kentosd serve`) masaüstüne, mobil uygulamaya, web portalına ve yapay zekâ ajanlarına aynı
JSON API'yi açar. Bu kılavuz API'yi ilk kez kullanacak geliştirici ve ajan içindir; her ucun ayrıntısı (gövde,
yanıt, hata) makinenin okuyacağı açıklamadadır:

| Belge | Ne için | Nerede |
|---|---|---|
| OpenAPI 3.1 açıklaması | Bütün uçlar, şemalar (JSON Schema 2020-12), hatalar, giriş yolları | `GET /v1/openapi.json`, `docs/api/openapi.json` |
| Ajanlar için kısa yol | Girişi, kuralları ve sık akışları anlatan kısa metin | `GET /v1/llms.txt`, `docs/api/llms.txt` |
| TypeScript sözleşmeleri | Aynı türlerin TypeScript karşılıkları | `apps/web/src/contracts/generated/` |
| Kararlar | Neden böyle olduğu | `docs/adr/9000-cloud-portal.md` (§12–§16) |

İki açıklama da sunucunun içine gömülüdür: depoda ne görünüyorsa sunucu onu verir. Uçlar değişince açıklama yeniden
üretilir (bkz. [Bu belgeyi güncellemek](#bu-belgeyi-güncellemek)).

## 1. Adres ve sürüm

- Bütün uçlar `/v1` altındadır. Sunucunun ayakta olduğunu ve sürümünü `GET /v1/health` söyler (oturum istemez).
- Geliştirmede sunucu `http://127.0.0.1:8787`'dedir; kurumun sunucusu kendi adresini verir.
- Her yanıt `x-request-id` taşır. Bir hatayı bildirirken bu kimlik yazılır; sunucu günlüğünde aynı kimlik vardır.

## 2. Giriş

Üç yol var; programlar ilk ikisini kullanır.

### 2.1 Program oturumu (masaüstü, mobil, ajan)

```bash
curl -s https://bulut.ornek-belediye.test/v1/auth/token \
  -H 'content-type: application/json' \
  -d '{"login":"ayse","password":"…","client":"mobile","label":"Ayşe'\''nin telefonu"}'
# 201 {"token":"kentos_s_3f9a…","expiresAt":"2026-12-17T09:00:00Z","me":{…}}
```

- Sonraki her istek `Authorization: Bearer kentos_s_…` taşır. Çerez ve `x-kentos-client` başlığı gerekmez.
- Oturum 7 gün kullanılmazsa, kullanılsa da en geç 30 günde biter. 401 gelince yeniden giriş yapılır.
- `client` programın türüdür (`desktop`, `mobile`, `agent`…), `label` cihazın kişinin bildiği adı. İkisi de
  hesabın oturum listesinde görünür: `GET /v1/me/sessions`; kaybolan telefonun oturumu
  `DELETE /v1/me/sessions/{id}` ile kapatılır.
- Çıkış: `POST /v1/auth/logout` (204).
- Yanlış parola 401; art arda yanlışlar 429 ve `Retry-After`.

### 2.2 Kişisel erişim anahtarı (betik, ajan)

Oturum açmış hesap web portalında **Hesabım → Erişim anahtarları**'ndan ya da API'den anahtar üretir:

```bash
curl -s https://…/v1/me/tokens -H "authorization: Bearer $OTURUM" -H 'content-type: application/json' \
  -d '{"name":"Rapor ajanı","scope":"read","expiresInDays":30}'
# 201 {"token":"kentos_pat_9c1e…","view":{"id":"…","prefix":"kentos_pat_9c1e","scope":"read",…}}
```

- Anahtar **yalnız bu yanıtta** görünür; sunucu yalnız SHA-256'sını saklar.
- `read` yalnız okur (`GET`, `HEAD`); başka her istek 403. `write` hesabın yapabildiği her şeyi yapar.
- 1–365 gün geçerlidir, hesap başına en çok 20 tanedir, `DELETE /v1/me/tokens/{id}` ile hemen geri alınır.
- Bir anahtarla yeni anahtar üretilemez ve parola değiştirilemez (403): sızan anahtar kendini çoğaltamaz.
- Ajanlara önce `read` anahtarı verin; değiştirmesi gerekiyorsa ayrı bir `write` anahtarı.

### 2.3 Tarayıcı (çerez)

Web portalı `POST /v1/auth/login` ile `kentos_session` çerezini alır. Çerezle yapılan her değiştirme isteği
`x-kentos-client: web` başlığını taşır; başlıksız istek reddedilir (siteler arası istek koruması). Kurum kimliğiyle
(OIDC) giriş `GET /v1/auth/oidc/start` ile başlar; hangi yolların açık olduğunu `GET /v1/auth/config` söyler.

## 3. İstekler ve yanıtlar

- Gövdeler JSON'dur, alan adları `camelCase`. Kimlikler UUID metnidir, tarihler RFC 3339.
- JavaScript'in 2⁵³ sınırını aşabilecek sayılar (revizyon, olay imleci, bazı bayt sayıları) ondalık **metin** olarak
  gelir; bulut dosyalarının bayt sayıları sayıdır.
- Kurumun kimliği (`{tenant}`) `GET /v1/me` yanıtındaki üyeliklerde, projelerinki `GET /v1/me/projects`'te.
- Yetki rolden gelir: projede `project.read`, `feature.write`, `project.download`, `project.history`,
  `project.share`…; portalda kurumun rolü (görüntüleyici okur, düzenleyici çalışır, proje yöneticisi karar verir).

## 4. Hatalar

Her hata aynı gövdeyi taşır (`ApiError`):

```json
{ "error": "invalid", "message": "Bir parça 1 bayt ile 32 MiB arasında olmalı (0 bayt geldi).",
  "path": "size", "retryable": false, "requestId": "6b1f…" }
```

| Durum | `error` | Anlamı | Ne yapılır |
|---|---|---|---|
| 401 | `unauthenticated` | Oturum yok ya da bitti | Yeniden giriş |
| 403 | `forbidden` | Yetki yok; ileti hangi yetkinin eksik olduğunu söyler | Kullanıcıya iletiyi gösterin |
| 404 | `not_found` | Yok ya da göremezsiniz (ayırt edilmez) | Kimliği denetleyin |
| 409 | `conflict`, `archived` | Beklenen sürüm/durum değişmiş, proje arşivde | Yeniden okuyup karar verin |
| 410 | `deleted` | Proje çöpte | — |
| 422 | `invalid` | Girdi geçersiz; `path` alanı söyler | Girdiyi düzeltin |
| 429 | `limited`, `quota` | Çok deneme ya da hesabın sınırı | `retryAfter` saniye bekleyin |
| 5xx | `unavailable`, `storage_full`… | Sunucu tarafı | `retryable: true` ise bekleyip tekrarlayın |

`message` Türkçedir ve kullanıcıya olduğu gibi gösterilebilir. `retryable: false` olan isteği değiştirmeden
tekrarlamayın.

## 5. Projeler ve komutlar

- Liste: `GET /v1/me/projects` (her kurumdan) ya da `GET /v1/tenants/{tenant}/projects`. Açma:
  `GET …/projects/{project}` (ayarlar, katmanlar, rol); künye: `…/details`; nesneler: `…/features`.
- **Projeyi değiştiren her şey bir komuttur:** `POST …/projects/{project}/commands`.

```json
{ "commandName": "project.changes", "version": 1, "tenantId": "…", "projectId": "…",
  "requestId": "istek-42", "idempotencyKey": "0193…", "expectedVersions": { "@data": "17" }, "input": { … } }
```

  Aynı `idempotencyKey` ikinci kez uygulanmaz: yanıtı kaybolan komut aynı anahtarla tekrarlanır. Beklenen sürüm
  tutmazsa 409 ve `conflicts`.
- **Canlı izleme:** `GET …/events?after=<imleç>&wait=25` yeni olay yoksa 25 saniyeye dek bekler (uzun yoklama).
  Masaüstü projeyi böyle izler; `GET /v1/ws` aynı olayları WebSocket'le iletir.
- **Dosya projeleri** (KCAD revizyonları): `GET …/files` revizyonlar, `GET …/files/{revision}` indirme
  (`Range: bytes=N-`), yeni revizyon `POST …/uploads` + `PUT …/uploads/{upload}` + `project.file.commit` komutu.

## 6. Bulut dosyaları

### 6.1 Yükleme: her boyut, kesintiye dayanıklı

```text
POST /v1/uploads            {"name":"zemin-etudu.pdf","size":7340032,"sha256":"<64 hex>"}
                            → 201 {"id":"…","partMax":33554432,"receivedBytes":0,"received":false,…}
PUT  /v1/uploads/{id}?offset=0         gövde: ilk 32 MiB'a kadar   → 200 {"receivedBytes":…}
PUT  /v1/uploads/{id}?offset=<gelen>   sonraki parça    → … son parçada {"received":true,"scan":"unscanned"}
GET  /v1/uploads/{id}                  bağlantı koparsa: nereden sürdürüleceği (receivedBytes)
DELETE /v1/uploads/{id}                vazgeçmek
```

- Dosya en çok 512 MiB, parça en çok `partMax` (32 MiB). Hesap aynı anda en çok 20 yükleme açık tutar.
- `offset` o ana dek gelen bayt sayısı olmalıdır; değilse 422 ve doğru yer iletidedir. Yarıda kesilen parça hiç
  yazılmaz.
- Son parçada boyut ve SHA-256 diskteki baytlardan denetlenir, sonra dosya **taranır** (6.2). Geçen yükleme
  `received: true` olur ve bir yere eklenene dek 24 saat bekler.
- Tamamlanan yükleme eklenir: projeye `POST …/projects/{project}/attachments {"uploadId":"…","note":"…"}`, başvuru
  dosyasına `POST /v1/dossiers/{id}/files/from-upload {"uploadId":"…","purpose":"package"}`. Yükleme bir kez alınır.

Mobil uygulama için öneri: dosyanın SHA-256'sını önceden hesaplayın, parçayı 4–8 MiB tutun (mobil ağda yeniden
gönderilecek miktar küçük kalır), `id`'yi cihazda saklayın; uygulama yeniden açılınca `GET /v1/uploads/{id}` ile
sürdürün.

### 6.2 Tarama

Karşı tarafa (belediyenin personeline, projenin üyelerine) gitmeden önce her dosya taranır:

- **Tür:** yalnız tanınan türler alınır; tür adın uzantısından okunur ve baytların o türün imzasıyla başlaması
  gerekir. Saklanan ortam türü (`mediaType`) istemcinin söylediği değil, bulunan türdür.

  | Grup | Uzantılar |
  |---|---|
  | Çizim ve harita | kcad, dxf, dwg, ncz, shp, shx, dbf, prj, cpg, gpkg, geojson, json, kml, kmz, gml, xml, las, laz, tif, tiff |
  | Belge | pdf, docx, xlsx, pptx, odt, ods, odp, doc, xls, ppt, rtf, txt, csv, tsv, md |
  | Resim ve video | png, jpg, jpeg, gif, webp, heic, heif, bmp, svg, mp4, mov |
  | Arşiv | zip |

- **Program yok:** çalıştırılabilir program ve betik her adla reddedilir; metin diye gönderilen web sayfası, betikli
  SVG, içinde program bulunan arşiv, parolalı arşiv ve makrolu ofis belgesi de.
- **Zararlı yazılım:** sektörün deneme dosyası (EICAR) her sunucuda bulunur. Sunucunun bir antivirüs servisi varsa
  (`KENTOS_AV_ADDRESS`) her dosya ondan da geçer.
- Saklanan dosya ne kadar denetlendiğini söyler: `scan: "clean"` (antivirüs taradı), `"unscanned"` (yalnız tür
  kuralları; sunucuda antivirüs yok ya da o an ulaşılamadı, nedeni `scanNote`'ta).
- Reddedilen dosya saklanmaz: yanıt 422, neden `message`'da. Sistem yöneticisi kayıt defterinde kimin neyi
  gönderdiğini görür.

### 6.3 İndirme

Ekler (`GET …/attachments/{attachment}`) ve başvuru belgeleri (`GET /v1/dossiers/{id}/files/{file}`) aynı kurallarla
iner:

- `ETag` dosyanın SHA-256'sıdır; `If-None-Match` ile değişmemişse **304** (birkaç bayt). Telefon dosyayı önbellekte
  tutup her açılışta yalnız sorar.
- `Range: bytes=a-b`, `bytes=a-`, `bytes=-n` → **206** ve `Content-Range`: yarıda kalan indirme sürer, PDF ve video
  parça parça açılır. Aralık dosyanın dışındaysa 416.
- `?inline=1`: PDF, resim, düz metin ve video tarayıcıda/telefonda yerinde gösterilir; başka türler hep indirilir.
- Her yanıt `X-Content-Type-Options: nosniff` ve hiçbir şey çalıştırmayan bir `Content-Security-Policy` taşır:
  yüklenen bir sayfa ya da resim uygulama gibi davranamaz.

### 6.4 Çizimi görmek (SVG)

`GET /v1/tenants/{tenant}/projects/{project}/preview.svg?width=800` projenin çizimini tek SVG resmi olarak verir:
CAD görünümü olmayan telefon, web sayfası ve ajan için.

- Dosya projesinde en yeni revizyon (`&revision=N` ile eskisi, `project.history` ister), veritabanı projesinde
  nesnelerin şimdiki hâli. Katman renkleri ve görünürlüğü uyulur, bloklar yerleştirilir, duvarlar kalınlığıyla,
  mekânlar adlarıyla çizilir. Ölçü, tablo, resim, nokta bulutu gibi nesneler çizilmez; sayıları `x-kentos-drawn` ve
  `x-kentos-skipped` başlıklarında.
- `ETag` revizyona (ya da veri sürümüne) ve genişliğe bağlıdır: değişmemişse 304. Dosya projesinin resmi bir kez
  çizilip saklanır.
- Başvuru dosyasındaki KCAD belgesinin resmi: `GET /v1/dossiers/{id}/files/{file}/preview.svg`.

## 7. Portal: belediye ile meslek mensubu

- **Başvuru** (meslek mensubu → belediye): `POST /v1/me/dossiers` taslak açar; belgeler eklenir
  (`/files`, `/files/from-upload`, `/files/from-project`); `send` adımıyla gönderilir. Belediye incelemeye alır, eksik
  bildirir ya da karar verir; her tur ayrı bir pakettir.
- **Aktarım** (belediye → meslek mensubu): `POST /v1/tenants/{tenant}/dossiers`; meslek mensubu kabul eder, işi
  teslim eder (`deliver`), belediye tamamlar.
- `GET /v1/dossiers/{id}` ayrıntının `actions` alanı o an atılabilecek adımları sayar; adım
  `POST /v1/dossiers/{id}/actions {"action","expectedStatus","note"}` ile atılır. `expectedStatus` tutmazsa 409:
  bu arada başkası davranmıştır, ayrıntı yeniden okunur.
- Yasal süreler (İmar Kanunu m.22 ve m.30) ayrıntıda gün olarak gelir.
- Yazışma: `POST /v1/dossiers/{id}/messages`. Bildirimler: `GET /v1/me/notifications`.
- **Büro:** sorumlu meslek mensubu dosyayı bürosuyla paylaşır (`PUT /v1/dossiers/{id}/office {"officeId": "…"}`) ya da
  başvuruyu baştan büro adına açar (`officeId`). Büroda düzenleyici belge hazırlar ve yazışır, proje yöneticisi gönderir
  ve teslim eder; büronun listesi `GET /v1/tenants/{büro}/dossiers`.
- **Harç ve ücret** (ADR 9003). Tarife bir JSON belgesidir:
  - `parameters`: girdiler;
  - `tables`: seçeneğe ve miktar dilimine göre birim tutar;
  - `items`: maktu, birim başına ya da KentOS ifade dilinde formül.

  Uçlar:
  - `GET /v1/tenants/{t}/fees/template`: 2464 sayılı Kanun'un yapısı, tutarlar 0.
  - `POST /v1/tenants/{t}/fees/try`: kaydetmeden dener.
  - `POST …/fees/schedules`, `…/{id}/activate`, `…/{id}/copy {"factor": 1.25}`: tarifeyi yazar, yürürlüğe koyar,
    yeni yıla kopyalar.

  Dosyada sıra şöyledir:
  1. `POST /v1/dossiers/{id}/fees/preview {"inputs": {}}` sorulacak girdileri verir.
  2. Aynı gövde `POST /v1/dossiers/{id}/fees` ile tahakkuk olur.
  3. Ödeme `…/fees/{fee}/paid {"receiptNo"}` ile kaydedilir.

  Tutarlar kuruştur (tam sayı). Her satırın `basis` alanı hesabı yazıyla verir. “Karardan önce ödenir” bir kalem
  ödenmeden başvuru onaylanmaz: `approve` adımı bu durumda 400 döner.

## 8. Bildirimler: zil, e-posta, SMS

- Zil: `GET /v1/me/notifications?unread=true`, okundu: `POST /v1/me/notifications/read {"all": true}`.
- E-posta ve SMS: sistem yöneticisi kanalı açar (`PUT /v1/platform/settings {"emailNotifications": true}`), kişi kendi
  seçer: `PUT /v1/me/notification-settings {"email": true, "sms": true, "phone": "0532 123 45 67"}`. E-posta yalnız
  doğrulanmış adrese gider. `GET /v1/me/notification-settings` son 20 iletiyi ve durumlarını (`sent`, `pending`,
  `failed`) da verir.
- Kurumun ağ geçidine giden istek (webhook) gövdenin HMAC-SHA256'sını taşır; alan taraf şöyle doğrular:

```python
import hmac, hashlib
expected = "sha256=" + hmac.new(SECRET.encode(), request_body, hashlib.sha256).hexdigest()
ok = hmac.compare_digest(expected, request.headers["x-kentos-signature"])
```

  Gövde: `{"id", "channel": "email"|"sms", "to", "recipient", "subject", "text", "link", "createdAt", "attempt"}`;
  2xx dışındaki her yanıt yeniden denenir (1, 5, 15, 60, 240 dakika), aynı `id` birden çok gelebilir.

## 9. İstemcilere notlar

- **Mobil uygulama:** program oturumunu (`client: "mobile"`, `label`: cihazın adı) güvenli depoda saklayın. Liste ve
  ayrıntıyı `ETag`'le önbelleğe alın; proje kartında `preview.svg?width=480` kullanın. Fotoğraf ve belgeyi 6.1'deki
  gibi parça parça gönderin. Bildirimler şimdilik `GET /v1/me/notifications?unread=true` ile sorulur.
- **Masaüstü:** bulut projelerini komutlarla ve uzun yoklamayla izler; portal dosyaları penceresi aynı uçları
  kullanır (ADR 9001).
- **Yapay zekâ ajanı:** önce `GET /v1/llms.txt`, sonra gerekirse `GET /v1/openapi.json`. `read` anahtarıyla başlayın;
  gönderme, onaylama, silme gibi geri alınmaz adımlardan önce kullanıcıya sorun.

## 10. Sunucu ayarları (dosyalar ve bildirimler)

| Ayar | Anlamı |
|---|---|
| `KENTOS_BLOB_DIR` | Dosyaların saklandığı klasör (veritabanıyla birlikte yedeklenir) |
| `KENTOS_AV_ADDRESS` | Antivirüs servisinin adresi: `tcp://ana-makine:3310` ya da `unix:///yol/servis.sock` (INSTREAM) |
| `KENTOS_AV_REQUIRED` | `1`: taranamayan dosya (servis kapalı, dosya sınırdan büyük) alınmaz, istemci 429 ile bekletilir |
| `KENTOS_AV_MAX_BYTES` | Antivirüse gönderilen en büyük dosya (varsayılan 25 MiB, servisin kendi sınırı) |
| `KENTOS_MAIL_TRANSPORT` | E-posta: `sendmail` (makinenin posta programı), `webhook` (kurumun ağ geçidi), `folder` (dosya) |
| `KENTOS_SMS_TRANSPORT` | SMS: `webhook` ya da `folder` |
| `KENTOS_MAIL_FROM` | Gönderen: `Örnek Belediyesi <bildirim@ornek.bel.tr>` |
| `KENTOS_SENDMAIL` | Posta programının yolu (varsayılan `/usr/sbin/sendmail`) |
| `KENTOS_NOTIFY_WEBHOOK`, `KENTOS_NOTIFY_WEBHOOK_SECRET` | Ağ geçidinin `https://` adresi ve imza anahtarı (en az 16 karakter) |
| `KENTOS_NOTIFY_FOLDER` | `folder` aktarımının klasörü |

## Bu belgeyi güncellemek

`docs/api/openapi.json`, uç kataloğundan (`apps/api/src/http/openapi/catalog.rs`) ve paylaşılan sözleşmelerin
şemalarından üretilir. İki test onu kilitler:

- `every_route_is_described_and_nothing_else`: yönlendiricideki her `(yöntem, yol)` katalogda, katalogdaki her uç
  yönlendiricide olmalıdır. Yeni bir uç eklenip katalog unutulursa test kırılır.
- `the_published_description_is_the_generated_one`: depodaki dosya üretilenle aynı olmalıdır.

Uç ya da sözleşme değişince:

```bash
KENTOS_WRITE_OPENAPI=1 cargo test -p kentos-api --bin kentosd openapi
```
