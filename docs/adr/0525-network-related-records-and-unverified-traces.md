# ADR 0525: Şebeke izlemede ilişkili kayıtlar ve doğrulanmamış bağlantılar

- **Durum:** kabul edildi (2026-10-19). `plan/gis.md` CBS-27'nin ilk dilimi (“vana kapama senaryosu etkilenen aboneleri
  ilişkiler üzerinden bulabilmeli; doğrulanmamış bağlantı sonucu güvenilir trace diye sunulmamalı”). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0209 §8 (Şebeke izleme: Bağlı, Akış aşağı, Akış yukarı, Yalıtım ve beslemesiz kalanlar), §9
  (Denetle), 0405 ve 0440 (projenin ilişkileri), 0521 (dönüşler ve seviyeler).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - terminaller (bir cihazın uçları arasındaki bağlantı kuralı: vananın girişi ve çıkışı, trafonun sargıları) ve alt ağlar
    (basınç bölgesi, besleme alanı) ağın tanımında;
  - dış sağlayıcının revizyonu değişince grafın yalnız etkilenen bölgesinin yeniden kurulması (bugün graf katmanlardan
    bütün kurulur; çizimin değişiklik damgası değişince yeniden kurulur, eski graf yeni nesnelerle karışmaz);
  - doğrusal referansta olayların rotanın kaynağına kalıcı bağı ve kaynağın M'si ya da geometrisi değişince kaymanın raporu.

## Bağlam

Şebeke izleme (ADR 0209 §8) ulaşılan boruları, kapatılacak vanaları ve beslemesiz kalanları buluyordu; ama bir arıza
senaryosunun asıl sorusu “kimler susuz kalır?”dır: aboneler borulara ya da bağlantı noktalarına bağlıdır, vanalar
haznelerin içindedir. KentOS'ta bu bağlar zaten projenin ilişkileriyle (ADR 0405: üst katman, alt katman ve alt kaydın
üstün kalıcı kimliğini tutan alanı) kurulur. Ayrıca izleme, Denetle'nin bulduğu kopuk bağlantılardan (yakın ama bağlı
olmayan uç, bağlanmayan kesişim) geçebilir; o zaman sonucu güvenilir değildir ve bu söylenmelidir.

## Karar

### 1. İlişkili kayıtlar

İzlemenin cevabı ulaşılan düğüm nesnelerini de verir (`TraceAnswer.junctions`: ulaşılan bir düğümdeki her düğüm nesnesi,
başlangıcınki ve durulan kapalı ya da engelli düğümünki dahil). Uygulama ulaşılan boruları, düğümleri ve (Yalıtım'da)
beslemesiz kalan boruları projenin ilişkilerine sorar (`kentos_domain::Document::related_of`):

- **Bağlı kayıtlar** (`attached`): üst katmanı ulaşılan bir nesnenin katmanı olan ilişkide, alanı o nesnenin kalıcı
  kimliğini tutan alt kayıtlar (bir borunun aboneleri). Çok değerli alanda kimliklerden biri olması yeter.
- **İçeren kayıtlar** (`container`): alt katmanı ulaşılan bir nesnenin katmanı olan ilişkide, nesnenin alanının andığı üst
  kayıtlar (bir vananın haznesi).

İlişkiler projenin sırasıyla, her biri adıyla söylenir; hiçbir şey bağlamayan ilişki söylenmez. Ağın tanımı değişmez: bütün
ilişkiler sorulur (ağın katmanlarına bağlı olmayanlar zaten bir şey vermez). Okumadır; ilişkilerin silme kuralları sorulmaz.

### 2. Doğrulanmamış bağlantılar

İzleme, Denetle'nin bağlantıyla ilgili bulgularından (yakın ama bağlı değil, bağlanmayan kesişim, ağın dışında düğüm,
düşen kısa parça, parçalarında olmayan dönüş, okunamayan seviye) ulaştığı bir nesneyi ananları cevabına koyar
(`TraceAnswer.unverified`); Denetle oturumda bir kez hesaplanır. Böyle bir izleme sonuç olarak verilir ama güvenilir diye
sunulmaz: araç bulguları çizimde çarpıyla gösterir, istemde “doğrulanmamış n bağlantıdan geçti (Denetle)” der ve Enter'da
sonucu uyarı düzeyinde, ağın Ağlar › Denetle ile düzeltilmesini söyleyerek bildirir.

### 3. Arayüz, Python

- **Şebeke izleme:** ilişkili kayıtlar istemde ilişkinin adıyla sayılır (“5 bağlı kayıt (Aboneler)”), nokta olanları
  çizimde halkayla gösterilir; Enter onları borularla ve vanalarla birlikte seçer.
- **Python:** `kentos.network.Network.trace` cevabında `junctions`, `related` (`Related`: ilişki, ad, `attached` ya da
  `container`, kayıtların kimlikleri) ve `unverified` (`Problem`).
- Ürün komutu yoktur: izleme çizimi değiştirmez (ADR 0209 §11'in sorgusu).

## Doğrulama

- `cargo test -p kentos-domain --test all related`: iki boru, üç abone, bir hazne ve vanası, iki boruyu çok değerli alanla
  anan malik; ulaşılan boru ve vananın aboneleri, haznesi ve maliki ilişkilerin sırasıyla; hiçbir şey bağlamayan ilişki yok.
- `cargo test -p kentos-geometry-core --lib network::trace`: ulaşılan düğümler (Yalıtım'da durulan vanalar, akış aşağıda
  arkadaki kaynak değil, bağlıda hepsi); ana borunun 0,5 m yakınında biten servis borusu: ana borudan izleme yakın ama bağlı
  olmayanı söyler, uzaktaki boru söylemez.
- `cargo test -p kentos-desktop networks::tests`: kolda Yalıtım; V-1 ve V-2 kapanır, koldaki üç abone ve V-2'nin ötesinde
  beslemesiz kalan ikisi Aboneler ilişkisiyle sayılır ve seçilir, kısa kalan servis borusu doğrulanmamış bağlantı olarak
  söylenir. Ağ izleri değişmeden geçer.
- Resim: `KENTOS_SHOTS_ONLY=iz-abone cargo test -p kentos-desktop networks::tests::screens -- --ignored --nocapture`
  (`.run/shots/aglar-iz-abone-*`).
