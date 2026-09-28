# ADR 0133: MCP sunucusu (`kentos-mcp`)

- **Durum:** kabul edildi (2026-09-28).
- **Tarih:** 2026-09-28
- **Bağlam belgesi:** ADR 0130 (başsız komut sunucusu), ADR 0131 (Python SDK'sı), ADR 0013 (ürün komutları); TODOS.md §15 (AI-02…AI-06, AI-12, AI-13, AI-17); MCP 2026-07-28 (modelcontextprotocol.io/specification/2026-07-28).

## Bağlam

F6'nın sırası çekirdek → Python → MCP'dir. Yapay zekâ ajanları KentOS çizimleriyle, betiklerin ve arayüzün kullandığı aynı komutlarla çalışmalıdır. Model kendi yolunu uyduramaz: geometriyi hesaplayan da, yetki kararını veren de o değildir (§15).

MCP 2026-07-28 durumsuzdur:

- Tokalaşma yoktur. Her istek protokol sürümünü ve istemcinin yeteneklerini `_meta`'da taşır.
- Sunucu `server/discover`'ı uygulamak zorundadır.
- Sonuçlar `resultType` taşır.
- Birden çok çağrıya yayılan durum açık bir tutamaçla anılır.

Eski istemciler (2025-11-25 ve öncesi) `initialize` ile tokalaşır.

## Karar

**Crate: `kentos-mcp`** (`crates/native/mcp`). İkili stdio üzerinden konuşur: satır başına bir JSON-RPC mesajı girer, bir yanıt çıkar.

- Başsız komut sunucusunun üstündedir: `Session`, katalog ve sorgular.
- JSON-RPC ve MCP belirtimden elle yazıldı; MCP kitaplığı ya da async çalışma zamanı yoktur.
- Yeni bağımlılık yoktur. `deps.mjs`'te `mcp` grubudur.

**İki dönem.**

- **Yeni istek:** `_meta`'da `io.modelcontextprotocol/protocolVersion` (2026-07-28) ve `clientCapabilities` taşır.
  - Sürüm başkaysa cevap -32022'dir: `data.supported`, `data.requested`.
  - Yetenekler eksikse cevap -32602'dir.
  - Sonuç `resultType: "complete"` ve `_meta`'da sunucunun kimliğini taşır.
  - `server/discover` şunları verir: desteklenen sürümler, yetenekler (`tools`, `resources`), yönergeler.
- **Eski istemci:** `initialize` bu süreç için bir sürümde anlaşır.
  - Bilinen sürümlerden biri istenirse o, yoksa 2025-11-25 seçilir.
  - Sonraki istekler `_meta`'sız karşılanır; sonuçlar eski biçimdedir, `resultType` yoktur.
- **İkisi de değilse:** `_meta`'sız ve tokalaşmasız istek -32602 alır; ileti iki yolu da söyler.
- **Bildirimlere** yanıt verilmez. Nesne olmayan mesaj `id: null` ile -32600 alır, bozuk JSON -32700.

**Çizimler tutamaçla anılır.**

- `drawing.open` ve `drawing.new` bir tutamaç (UUIDv4) verir; sonraki her araç `drawing` bağımsız değişkeniyle onu alır. Protokolde oturum yoktur.
- Süreçte en çok 16 çizim açık olabilir. Fazlası reddedilir ve `drawing.close` önerilir; sessizce kapatılan olmaz.
- Kaydedilmemiş çizim ancak `discard: true` ile kapanır.

**Araçlar** sabit sıradadır.

- **Çizimin kendi araçları:** `drawing.open`, `drawing.new`, `drawing.save`, `drawing.close`, `drawing.list`, `drawing.summary`, `drawing.layers`, `drawing.entities`, `drawing.entity`, `drawing.measure`, `drawing.undo`, `drawing.redo`.
- **Kataloğun çizim komutları** kendi kimlikleriyle gelir (`cad.polygon.create` … 12 komut).
  - Girdi şemaları kataloğun şemalarıdır; başına zorunlu `drawing` ve `op` (execute, plan, validate) eklenir.
  - Açıklama kataloğun özetidir, `CommandResult`'ın biçimini de söyler.
- **İpuçları (`annotations`):** sorgular salt okunurdur. Silme, düzenleme, taşıma ve özellik değiştirme yıkıcıdır. Kaydetme ve kapatma da yıkıcıdır.
- **Sunucu komutları (`project.*`) yoktur.** Kimlik gerektirirler; stdio sunucusu kimlik bilgisini ortamdan almalıdır. Bu ayrı bir karardır (AI-08).
- **Sonuç:** her aracın sonucu yapılandırılmış içeriktir (`structuredContent`: `CommandResult`, sayfa, ölçü …). Aynı JSON, yalnız metin okuyan istemciler için metin olarak da verilir.

**Hatalar iki türlüdür.**

- **Araç sonucu (`isError: true`):** modelin düzeltebileceği her şey. Kod, ileti ve alan yolu yapılandırılmış içerikte gelir (AI-12, AI-13):
  - komutun reddi (`failed`, `needs_input`, `conflict`);
  - girdinin tipi tutmaması (`invalid_input`);
  - bilinmeyen tutamaç (`unknown_drawing`);
  - koordinat sistemi verilmeyen yeni çizim;
  - kaydedilmemiş çizimi kapatma (`unsaved`).
- **JSON-RPC hatası:** bilinmeyen araç ya da yöntem, bozuk istek.

**Sayfalar küçüktür** (AI-05). `drawing.entities` varsayılan olarak 100, en çok 1 000 nesne verir; katman, tür ve kutu süzgeciyle, `next` imleciyle sürer. Bütün çizim bağlama dökülmez.

**Kaynaklar** (AI-04):

- `kentos://catalog`: bütün katalog, şemaları ve örnekleriyle;
- `kentos://drawing/{tutamaç}`: açık çizimin özeti ve katman ağacı.

Olmayan kaynak -32602 alır, 2026-07-28'in kuralı budur.

## Sonuçlar

- Bir ajan, Python betiği ve masaüstünün aracı aynı komutları aynı işleyicilerle çalıştırır; sonuçları aynıdır (`fixtures/commands/v1`).
- **Komut kaydı:** yeni bir katalog komutu, `hosts`'unda `desktop` varsa kendiliğinden araç olur.
- **Güven** (AI-07, AI-09):
  - Sunucu, kullanıcının yetkisiyle çalışan yerel bir süreçtir; yolları kullanıcı gibi açar ve yazar.
  - Onay istemcinindir; MCP de insanın döngüde kalmasını ister.
  - Katman adları ve öznitelikler veridir, talimat değildir. Sunucu onları olduğu gibi döner.
- **Açık kalanlar:**
  - sunucu komutları ve kimlik (AI-08);
  - masaüstünde açık çizime bağlanmak (konsolun kanalı gibi);
  - istem şablonları (`prompts`);
  - denetim kaydı (AI-11);
  - değerlendirme senaryoları (AI-17, AI-18).

## Doğrulama

`cargo test -p kentos-mcp` (`tests/mcp.rs`), 5 test:

- **Yeni istemci:** keşif, araç listesi (aynı sırayla ikinci kez), yeni çizim, plan (hiçbir şey yazmaz), yazma (metin ve yapılandırılmış içerik aynı JSON), ölçü (250 m²), kutulu sayfa, kayıt, kapatma, yeniden açma (aynı kimlik, aynı alan).
- **Redler:**
  - araç sonucu olarak: komutun `layer_not_found`'u, tipi tutmayan girdi, bilinmeyen tutamaç, CRS'siz yeni çizim, kaydedilmemiş çizimi kapatma ve `discard`;
  - JSON-RPC hatası olarak: bilinmeyen araç ve yöntem, `_meta`'sız istek, bilinmeyen sürüm (-32022, `data`), yeteneksiz istek.
- **Eski istemci:** `initialize` (2025-06-18 ve bilinmeyen sürüm), bildirim, `_meta`'sız `tools/list`, `ping`.
- **Kaynaklar:** liste, katalog, çizim, olmayan kaynak.
- **Gerçek süreç:** stdio'da JSON satırları; bildirime yanıt verilmez; nesne olmayan mesaja -32600.

Ayrıca `cargo clippy -p kentos-mcp --all-targets -- -D warnings` ve `node scripts/arch/deps.mjs`.
