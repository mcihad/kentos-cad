# ADR 0079: Başkasının sildiği katman, gönderilmemiş nesneleri varken kalır

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** CLAUDE.md §13.1, §21.3; ADR 0026 (bulutta nesne kimliği), 0040–0044 (masaüstünün bulutu), 0072 (katman silme ve sunucu koruması), 0076 (katman ekleme)
- **Kaynak:** web ajanının `88ca558`'i (`app/cloud/keptLayers.ts`, `sync.ts`, `syncRemote.ts`). Web ajanı önerdi, kararı sahibin genel yönüyle verdik: kullanıcının nesnesini kaybetmek, başkasının sildiği bir katmanı geri getirmekten kötüdür.

## Bağlam

- Başka bir düzenleyici bir katmanı silerken bu cihazın o katmanda gönderilmemiş nesnesi olabilir: nesne orada çizilmiş ya da oraya taşınmış, silme bize henüz ulaşmamıştır. Sunucunun koruması (ADR 0072) yalnız kendi bildiklerini görür.
- Sunucunun ağacı olduğu gibi alınırsa o nesneler katmansız kalır:
  - gönderilemezler: sunucu olmayan katmandaki nesneyi reddeder;
  - çizim kaydedilip yeniden açılamaz: okuyucu olmayan katmandaki nesneyi reddeder.

## Karar

- **Kural:** başkalarının ağacı bu çizimin bir katmanını düşürürse ve o katmanda sunucunun henüz almadığı değişikliği olan nesne varsa, katman çizimde kalır.
  - Nesne gönderilmemiş sayılır: değişmişse ya da yoldaki komuttaysa.
  - Katman yerel düğümün kopyasıdır (bayrakları, stili). Gelen ağaçta grubu hâlâ varsa o gruptaki yerine konur; yoksa ağacın en üstüne, yerel sırasına (sığdığı kadar).
- **Tümü gönderilmişse** katman gider. Diğer düzenleyicinin istediği budur; nesneleri aynı olaylarla silinir.
- **İleti** (katman başına, sayısıyla): “‘X’ katmanını başka biri sildi; üzerinde gönderilmemiş N nesneniz olduğu için katman bu çizimde kaldı ve yeniden kaydedilecek.”
- **Sunucuya geri dönüş:**
  - Temel (sunucunun bildiğimiz hâli) sunucunun ağacıdır.
  - Çizimin ağacı ondan kalan katman kadar farklıdır. Bu yüzden sonraki komut ağacı katmanla birlikte, sunucunun meta sürümü üzerinden, nesnelerin oluşturmalarıyla gönderir.
- **Nerede uygulanır:**
  1. Başkalarının değişiklikleri alınırken, meta geldiğinde (`ProjectSync::take_remote`).
  2. Meta çakışmasında “sunucudaki” seçilince (`take_theirs`). Önce nesnelerin çakışmaları biter; artık gönderilmemiş sayılmazlar.
- **`project.edit` yoksa** katman yine kalır: nesneler katmansız kalmaz, çizim kaydedilebilir. Ağaç gönderilemez; bilinen “yalnız bu cihazda” iletisi geçerlidir. (ADR 0078'den sonra böyle bir hesap katman ekleyemez; kural yine de katmanı ve nesneleri korur.)

## Doğrulama

- `crates/native/cloud/src/sync/tests.rs`, üç yeni:
  - gönderilmemiş nesnesi olan silinmiş katman grubunda kalır, ileti sayar, sonraki komut ağacı katmanla ve nesneyle sunucunun sürümü üzerinden gönderir;
  - hepsi gönderilmiş katman silinir ve bir şey söylenmez;
  - meta çakışmasında “sunucudaki”: ad sunucununkine döner, katman kalır ve geri gönderilir.
- Masaüstü iletiyi söyler (`cloud/follow.rs`); iki yol da `live` testlerinde değişmeden geçer.
- Geçenler: `pnpm rust:test` (bulut istemcisi dahil), `pnpm rust:test:desktop`, `pnpm typecheck`, `pnpm test`, `pnpm build`, `pnpm inventory:check`, `KENTOS_E2E_DB=scratch pnpm e2e:cloud`.
