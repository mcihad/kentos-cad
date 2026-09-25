# ADR 0012: Sunucunun görevi kişisel ve kurumsal proje bulutu; Martin rolü kapsam dışı

- **Durum:** kabul edildi (2026-09-25, sahibin kararı: CLAUDE.md §0, TODOS.md “Son kapsam kararı”)
- **Tarih:** 2026-09-25
- **Bağlam belgesi:** CLAUDE.md §0, §13, §15–17, §24.3; TODOS.md §10–12, §12.5, §20.1
- **Değiştirdiği:**
  - eski CLAUDE.md §17 “Martin sınıfı MVT, stil ve güncellik” ve §17.1 (27f771d'ye kadar);
  - eski Faz C “Yayın ve stil” ve Faz E “Yayın motoru bağımsızlığı”;
  - ADR 0001'deki planlı `crates/tiles`;
  - ADR 0002'deki “Sunucudan veri MVT/TileJSON ile gelir”;
  - ADR 0005'teki tile hedefleri ve Martin karşılaştırması;
  - ADR 0006'daki “yayın geldiğinde (Faz C)” notu.

## Bağlam

Faz B'den beri çalışan bir sunucu var (koddan doğrulandı, 25 Eylül):

- `kentosd` (Axum, Tokio, SQLx) ve PostgreSQL + PostGIS (`kentos_cad`, satır güvenliği, iki rol; ADR 0006).
- Yerel hesap ve OpenID ile giriş (ADR 0007).
- Kurum (tenant) üyeliğinde `owner`, `admin`, `project_manager`, `editor` ve `viewer` rolleri.
- Kurum altında proje listesi, bilgisi, nesneleri, komutları (`project.changes`: beklenen sürüm, idempotency, audit, outbox) ve olayları. WebSocket, yeniden adlandırma, yumuşak silme ve olay saklama süresi.
- Tarayıcıda bulut açma ve yükleme, otomatik kayıt, cihaz taslağı ve çakışma çözümü.

Eski plan sunucuya ikinci bir ana görev de veriyordu: KentOS ağ geçidi arkasında Martin ile MVT, TileJSON, stil, sprite ve glyph yayını, ileride de Martin'den bağımsız bir yayın motoru. Tarayıcının sunucudan veriyi MVT ile alacağı varsayılıyordu (ADR 0002). Sahip 25 Eylül'de sunucunun görevini daralttı ve netleştirdi.

## Karar

- **Sunucunun ana görevi kişilerin ve kurumların projelerini güvenle saklamak.** Bunun yanında sunucu projeleri cihazlar arasında eşitler, sürümler, yetkiye göre erişilebilir kılar ve paylaştırır.
  - 18 uygulaması, imar planı, ifraz/tevhit, genel CAD, CBS, yol ve mimari çalışmaları aynı proje modelinin türleridir. Tür başına ayrı depolama ya da kullanıcı sistemi kurulmaz.
  - Proje türü saklama biçiminden bağımsızdır.
- **Martin tümleşimi ve Martin eşdeğeri bir yayın motoru şimdiki kapsamda yoktur.** Proje bulutunun önkoşulu da değildir.
  - Bir projeyi saklamak, açmak ya da paylaşmak için MVT, TileJSON ya da tile sunucusu gerekmez. Paylaşım yetkili proje, nesne ve varlık servisleriyle çalışır.
  - Olası bir harita/tile yayın rolü ileride ancak somut ihtiyaç, ayrı ürün kararı ve yeni bir ADR ile açılır (TODOS.md §12.5, `FUTURE-01..04`).
- **Saklama biçimleri ve otorite** (TODOS.md §10.1):

  | Biçim | Düzenlenebilir verinin otoritesi | `.kcad` |
  |---|---|---|
  | Yerel dosya | Açık belge; son dayanıklı kayıt | Tam snapshot |
  | Bulut dosya revizyonu | Sunucudaki tamamlanmış, değişmez dosya revizyonu (içerik nesne deposunda, katalog ve izinler PostgreSQL'de) | Aynı tam binary snapshot (ADR 0011) |
  | Yönetilen PostGIS CAD/CBS projesi | PostgreSQL transaction ve revizyon akışı | Tutarlı bir revizyonun tam snapshot'ı; seçilirse açıkça işaretli, yalnız metadata taşıyan kayıt |
  | Doğrudan dış PostGIS projesi | Dış veritabanındaki yetkili kaynak | Yalnız proje metadata'sı, bağlantı referansı, şema, stil ve yerleşim. Parola taşımaz |

  - Biçimler arasında sessiz geçiş yapılmaz. Veriyi projeye gömmek, PostGIS'e aktarmak ve referanslı kaydetmek açık komutlardır.
  - Aynı geometrinin iki bağımsız, yazılabilir otoritesi olmaz.
  - Bir dosya projesini buluta yüklemek nesne tablolarına içe aktarma ya da CAD'den CBS'e dönüştürme tetiklemez.
- **CAD projeleri de PostGIS'te saklanabilir, CAD anlamı korunarak.** Mevcut ayrım sürer (`crates/server/application/src/cad.rs`, ADR 0006): basit geometride `geom`, analitik CAD'de `cad_definition` kaynaktır; CBS geometrisi türetilmiş izdüşümdür. CAD sessizce CBS çoklu çizgisine indirgenmez.
- **Mevcut kimlik, tenant, rol, `project.changes`, WebSocket ve outbox altyapısı temeldir.** Kişisel çalışma alanı, proje düzeyinde yetki, davet, paylaşım ve erişimi geri alma bunun üzerine eklenir (TODOS.md §12, `CLOUD-01..30`). Kurum üyeliği her projeyi görme hakkı değildir. Yetki her girişte sunucuda denetlenir.

## Sonuçlar

- **ADR 0001:** planlı bileşen listesindeki `crates/tiles` (Martin arkasında TileJSON/MVT, önbellek geçersizleştirme) çıkar. `apps/worker` gelecek iş olarak kalır (TODOS.md §13).
- **ADR 0002:** `SceneLayer` notundaki “sunucudan veri MVT/TileJSON ile gelir” varsayımı geçersizdir. Bulut verisi yetkili proje ve nesne servisleriyle ve binary snapshot'larla gelir. `SceneLayer`'ın sözleşme olmadığı kuralı sürer.
- **ADR 0005:** “Tile, ılık önbellek”, “Tile, soğuk” satırları ve Martin karşılaştırması tarihsel ya da gelecek yayın başvurusudur, bugünkü proje bulutunun kabul kapısı değildir (TODOS.md §20.1). Sunucunun öbür hedefleri (commit, ikinci istemcide görünme, iş kabulü) taslak olarak sürer. ADR 0005 hâlâ sahibin onayını bekler.
- **ADR 0006:** katman tablosuna ayırma yayınla değil, yönetilen CAD/CBS proje şemasıyla (`PG-19`) ve proje yetkileriyle gelir.
- **Eski “backend henüz yok” notları geçersizdir.** Sunucu vardır (CLAUDE.md §1, §13). Eksik olan kişisel alan, proje düzeyinde yetki ve paylaşım, binary revizyonlar, nesne deposu ve kalıcı worker'dır.
- Martin, PostGIS `ST_AsMVT` ve PMTiles araştırma kaynakları yalnız gelecek değerlendirmesi için TODOS.md §12.5'te tutulur.
