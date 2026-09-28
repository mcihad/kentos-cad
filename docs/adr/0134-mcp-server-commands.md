# ADR 0134: MCP'de sunucu komutları

- **Durum:** kabul edildi (2026-09-28).
- **Tarih:** 2026-09-28
- **Bağlam belgesi:** ADR 0133 (MCP sunucusu), ADR 0131 (Python SDK'sı), ADR 0040 (masaüstünün bulut istemcisi), ADR 0015 (proje yetkisi); TODOS.md §15 (AI-03, AI-08), §22 F6.

## Bağlam

F6'nın kabul ölçütü şudur: aynı çizim, sorgu, kayıt ve paylaşım akışı arayüzde, Python'da ve yapay zekâda eşdeğer olmalıdır.

ADR 0133'ün MCP sunucusu yalnız yerel çizimle çalışıyordu. Paylaşım ve öbür proje komutları (`project.*`) sunucudadır ve bir hesap ister. MCP'nin stdio kuralı açıktır: kimlik bilgisi ortamdan gelir; HTTP yetkilendirme akışı stdio'ya uygulanmaz.

## Karar

**Hesap ortamdandır.** `KENTOS_URL`, `KENTOS_LOGIN`, `KENTOS_PASSWORD` MCP istemcisinin sunucu tanımında verilir; model bunları görmez ve vermez.

- Oturum ilk sunucu aracında açılır.
- Sunucu oturumu bitirirse (401) bir kez yeniden açılır.
- Ortam eksikse araç `not_configured` ile reddeder; ret neyin gerektiğini söyler.
- Liste bağlantıya göre değişmez: sunucu araçları her zaman listededir (MCP 2026-07-28).

**Masaüstünün bulut istemcisi.** Sunucu araçları `kentos-cloud` üzerinden gider; istemci işini kendi çalışma zamanında yapar, MCP ikilisi küçük bir bekleyiciyle sonucunu alır.

- İstemci artık kendini tanıtan programı seçebilir (`Cloud::for_program`). MCP isteklerinde `x-kentos-client: mcp` gider.
- Sunucu bu adı tanır (`CLIENTS`). Kural aynıdır: özel başlık siteler arası formu durdurur; ad yalnız kimin sorduğunu söyler.
- `kentos-cloud`'a kurum yolu eklendi (`tenant_command`, `POST /v1/tenants/{t}/commands`): `project.create` onunla gider.
- `deps.mjs`'te `mcp` grubu `native`'i kullanabilir; veritabanı ve sunucu çatısı yine yasaktır.

**Araçlar.**

- **`project.list`:** hesabın proje kataloğundan bir sayfa. `view` mine, organization, shared, recent, favorites, archived ya da trash olabilir; `q`, `tenant`, `limit` ve `after` alır.
- **`project.open`:** veritabanı projesinin bu anki `.kcad` görüntüsünü indirir ve yerel bir çizim olarak açar.
  - Tutamaç ve projenin revizyonu döner.
  - Açılan yerel kopyadır: değişiklikleri buluta gitmez, buluta `project.changes` yazar.
- **Kataloğun 21 sunucu komutu** kendi kimlikleriyle gelir (`project.share`, `project.rename`, `project.checkpoint.create` …). Girdi şemasına şunlar eklenir:
  - zorunlu `tenant`;
  - projesi olan komutlarda zorunlu `project` (`project.create`'te yoktur);
  - isteğe bağlı `expectedVersions` ve `idempotencyKey`.

  Komut sunucunun zarfıyla gider (`mcp-…` istek kimliği, verilmezse yeni bir idempotency anahtarı).
- **İpuçları:** sunucu araçları dış dünyaya dokunur (`openWorldHint`). Kalıcı silme, çöpe taşıma, erişimi ve daveti geri alma, kontrol noktası silme ve `project.changes` yıkıcıdır.

**Yetkiyi sunucu verir.** Her komut hesabın proje rolüyle denetlenir. MCP'nin ipuçları yetki değildir (AI-08).

**Sunucunun reddi araç sonucudur** (`isError`). Model reddi okuyup kendini düzeltebilir. Sonuçta şunlar vardır: HTTP durumu, sabit kod, Türkçe ileti, varsa alan yolu (`name`) ve revizyon, yeniden denenebilir olup olmadığı.

## Sonuçlar

- F6'nın kabul akışı üç yüzde de çalışır: kapalı alan çiz, ölç, katmana ata, kaydet, paylaş.
- **Güven:**
  - Parola MCP istemcisinin yapılandırmasındadır, düz metindir. Bu, yerel geliştirme ve güvenilir makine içindir.
  - Kurumsal kullanımda OpenID erişim belirteci (`Authorization: Bearer`) ve MCP'nin HTTP taşıması ayrı iştir (AI-08).
- **Açık kalanlar:**
  - dosya projesine revizyon (yükleme ister);
  - bir bulut projesine buradan açılan çizimin değişikliğini geri yazmak (`project.changes`'e nesneleri model verir);
  - denetim kaydı (AI-11).

## Doğrulama

- `cargo test -p kentos-mcp`, 6 test: sunucu araçları listede, şemaları `tenant` ve `project` ister (`project.create` projesiz), dış dünya ipucu; ortam yoksa `not_configured`.
- `scripts/python/live.py` gerçek kentosd ile geçici veritabanında çalışır; `kentos_cad`'e dokunulmaz. Önce Python akışı koşar, ardından aynı proje ayşe'nin hesabıyla MCP'den işlenir:
  - `project.list` projeyi bulur;
  - `project.open` ve `drawing.measure` aynı kimlikle 250 m² verir;
  - `project.share` mehmet'i görüntüleyici yapar, `project.access.revoke` geri alır;
  - boş adla `project.rename` sunucunun reddini `422 invalid (name)` olarak getirir.
- `cargo test -p kentos-cloud` (56), `cargo test -p kentos-api client_tests`, clippy.
