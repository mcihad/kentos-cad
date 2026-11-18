KentOS'un bulut sunucusu: kurumların (belediye, büro, şirket) projeleri, bulut dosyaları ve belediye ile meslek
mensupları arasındaki başvuru/aktarım portalı. Masaüstü, mobil uygulama, web portalı ve yapay zekâ ajanları aynı
uçları kullanır. Ayrıntılı kılavuz: `docs/api/README.md`; ajanlar için kısa yol: `GET /v1/llms.txt`.

## Giriş

- **Program (masaüstü, mobil, ajan):** `POST /v1/auth/token` `{ "login", "password", "client": "mobile", "label":
  "Saha telefonu" }` → `201 { "token": "kentos_s_…", "expiresAt", "me" }`. Sonraki her istek
  `Authorization: Bearer kentos_s_…` taşır. Oturum 7 gün kullanılmayınca, en geç 30 günde biter; 401 gelince yeniden
  giriş yapılır. Çıkış: `POST /v1/auth/logout`.
- **Kişisel erişim anahtarı (betik, ajan):** oturum açmış hesap `POST /v1/me/tokens` `{ "name", "scope": "read" |
  "write", "expiresInDays" }` ile `kentos_pat_…` anahtarı üretir; anahtar yalnız bir kez gösterilir. `read` yalnız
  `GET`/`HEAD` yapar. Anahtar yeni anahtar üretmez, parola değiştirmez.
- **Tarayıcı:** `POST /v1/auth/login` çerez yazar; çerezle yapılan her değiştirme isteği `x-kentos-client: web`
  başlığını taşır (taşıyıcı anahtarla gerekmez).

## Sözleşmeler

- Gövdeler JSON, alan adları `camelCase`. Kimlikler UUID metnidir; tarihler RFC 3339.
- Büyük sayılar (revizyon, imleç, bayt) bazı yanıtlarda ondalık **metin** olarak gelir (JavaScript'in 2⁵³ sınırı).
- Hata gövdesi her yerde aynıdır (`ApiError`): `error` (kararlı kod: `invalid`, `not_found`, `conflict`, `forbidden`,
  `unauthenticated`, `limited`, …), `message` (Türkçe, kullanıcıya gösterilebilir), `path` (hangi alan), `retryable`,
  `retryAfter`, `requestId`. Her yanıt `x-request-id` taşır; hata bildirirken o yazılır.
- 404 "yok" ile "göremezsin"i ayırt etmez: başka kurumun kaydı yokmuş gibidir.
- Projeyi değiştiren her şey bir komuttur (`POST …/commands`); aynı `idempotencyKey` ile yeniden gönderilen komut
  ikinci kez uygulanmaz, yanıtı kaybolan istek güvenle tekrarlanır.

## Dosyalar

Her boyuttaki dosya `POST /v1/uploads` → `PUT /v1/uploads/{id}?offset=N` (parça en çok 32 MiB) ile gider; son parçada
SHA-256 denetlenir ve dosya taranır. Tamamlanan yükleme bir yere eklenir: projenin eki
(`POST …/projects/{project}/attachments`) ya da dosyanın belgesi (`POST /v1/dossiers/{id}/files/from-upload`).
İndirmeler `ETag` (SHA-256), `If-None-Match` (304) ve `Range` (206) destekler; `?inline=1` PDF ve resmi yerinde
gösterir. Projenin çizimi telefonda `GET …/projects/{project}/preview.svg` ile görülür.
