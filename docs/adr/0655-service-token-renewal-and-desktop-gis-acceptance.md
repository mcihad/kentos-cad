# ADR 0655: Servis belirtecinin yenilenmesi ve CBS kabul projesinin masaüstünde oynatılması

- **Durum:** kabul edildi (2026-10-26). `plan/gis.md` CBS-43 ve CBS-44'ün son dilimi. Sahibin kararı (26 Ekim): “43 ve 44
  maddeyi birlikte yap, sadece en acil olanları yap, konuyu şimdilik kapatacağız.” İki madde bu dilimle kapanır; kalanlar
  aşağıda ertelenmiş olarak yazılıdır.
- **Bağlam belgesi:** ADR 0208 (harita servisleri, bağlantılar ve belirteçler), 0513 (servisten veri almanın bitişi), 0554 (CBS'nin
  ilk kabul projesi), 0654 (kayıtta sınırlı bellek).
- **Web:** web'in servis istemcisi de belirteci süresi dolmadan yenilemeli, TODOS.md §0.2.
- **`.kcad`:** değişmez.
- **Bağımlılık:** yeni kitaplık yok.
- **Ertelenenler (sahibin kararıyla, maddeler kapalı):**
  - CBS-43: servis üstünde canlı sorgu ve düzenleme (`cad.sources.query`'nin bir OGC API Features sağlayıcısı), CQL2 JSON ve
    queryables, servisten alınan verinin expires/ETag ile yenilenmesi, QGIS'in FeatureServer'ıyla performans kıyası, yayın (P2);
  - CBS-44: PostGIS'in, yetkinin ve geri yüklemenin kabul senaryosuna bağlanması, performans kabulü.

## Bağlam

CBS-43'ün kalanında “belirteç yenileme” vardı. İnceleme somut bir açık gösterdi. ArcGIS ve OAuth 2 bağlantılı karo servislerinde
belirteç servis hazırlanırken bir kez alınıyordu (`resolve::run`) ve hiç yenilenmiyordu. Bu belirteçler genellikle bir saatte
biter. Sonrasında her karo 401 ya da 403 ile reddediliyordu ve kullanıcı yalnız “erişimi reddetti” uyarısı görüyordu. Servisten
veri al'da (`net::send`) belirteç her istekte alınıyordu, orada açık yoktu. Kesilen ağ da sorun değildi: alma bir sayfada
düşünce hata döner ve çizime hiçbir şey yazılmaz (`take_all`).

CBS-44'ün kalanında “aynı senaryo MCP'de ve masaüstünde” ile “kesilen işlem ve yeniden açma” vardı. MCP'de senaryo zaten
oynatılıyordu (`crates/native/mcp/tests/acceptance.rs`); plandaki not eskimişti. Masaüstünde ise oynatılmıyordu.

## Karar

### 1. Belirteç bitmeden yenilenir

- `kentos_services::auth::renew(conn, held, now)`: bağlantı belirteç alıyorsa (ArcGIS, OAuth 2; `takes_token`) ve elde
  belirteç yoksa, bitimine bir dakikadan az kaldıysa ya da sunucu onu reddettiyse yenisi istenir.
- Masaüstünün karo isteği (`net::tile_request`) her karodan önce sorar. Gerekirse belirteci servisin iş parçacığında alır
  (`resolve::token_for`) ve hazır servise yazar.
- Sunucu bir karoyu 401 ya da 403 ile reddederse belirteç reddedilmiş sayılır (`Token::refused`). Bir sonraki karo yenisini
  alır ve uyarı verilmez. Yeni belirteç de reddedilirse uyarı verilir. Bir karo gelince sayaç sıfırlanır
  (`Ready::token_refused`), böylece sessiz yeniden deneme sonsuz sürmez.

### 2. CBS kabul projesi masaüstünde

`acceptance_gis_tests.rs`, `fixtures/acceptance/v1/parcels.json`'u uygulamada oynatır:

- komutlar MCP istemcisinin bağlandığı ajan bağlantısından;
- geri alma Ctrl+Z gibi;
- yeniden açma uygulamanın kendi kaydıyla (`saving::write_shared`, ADR 0654) ve açışıyla;
- beş biçim yazılır, GeoPackage yeni projede eksiksiz açılır.

Yeniden açma adımında kayıt bir kez de durdurulur. Dosya önceki baytlarında kalır ve yanında geçici dosya kalmaz. Her yazma
kayıtta komutun satırıdır. Böylece senaryo Python'da, MCP'de ve masaüstünde aynı dosyadan oynatılır.

## Doğrulama

- `cargo test -p kentos-services --lib auth`: belirtecin yenilenme kuralı (yok, bir saat kala, son dakika, bitmiş, reddedilmiş,
  belirteçsiz bağlantı).
- `cargo test -p kentos-desktop --bin kentos-cad acceptance`: odanın ve parsellerin kabul senaryoları masaüstünde.
- `cargo clippy -p kentos-services -p kentos-desktop --all-targets -- -D warnings`.
- Gerçek bir ArcGIS ya da OAuth 2 servisinde bir saatlik oturum denenmedi (hesap yok). Kural birim testinde, akış kodda.
