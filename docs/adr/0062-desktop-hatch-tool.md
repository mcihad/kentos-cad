# ADR 0062: Masaüstünde Tarama aracı

- **Durum:** kabul edildi (2026-09-26).
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** CLAUDE.md §4.5, §4.7; TODOS.md `UX-01`, `CMD-04`, `UI-11`; ADR 0021 (native araç oturumu), 0029 (geometri deposu), 0057 (`cad.entities.create`)
- **Sahibin yönü (26 Eylül):** web'deki araçlar ve düzenleyiciler masaüstüne birebir taşınır.
- **Web ajanının tarifi (26 Eylül):** `tools/hatchTool.ts` ve `tools/visibleFaces.ts` koddan okundu. Web'de değişiklik gerekmedi. Web'in bilinen sınırları, hata sayılmayanlar:
  - içine tıklanacak nokta yazılamaz;
  - aynı yer iki kez taranabilir;
  - desen yalnız dört hazır desenden seçilir; açı ve aralık sonra Öznitelikler'de değişir.

## Bağlam

- Masaüstünde Tarama (`tool.hatch`) "web'de var; masaüstüne henüz taşınmadı" diyordu. Şeridin Açıklama grubunda soluktu.
- Masaüstü taramaları zaten çiziyordu (ADR 0019). Eksik olan, taramayı koyan araçtı.

## Karar

### Araç (`crates/native/interaction/src/hatch.rs`)

- **İstem** web'inkidir, kelimesi kelimesine: "Tarama: taranacak yerin içine tıklayın [Desen (D): Çizgili 45° / Sınır (B): kapalı nesne / Adalar (A): taranmaz]".
  - Sınır çizgilerken sona "Sınır katmanı (K): tümü" ya da katmanın adı eklenir.
  - Katman seçilirken: "Tarama: sınır olacak katmandan bir nesneye tıklayın [Tüm katmanlar (K)]".
- **Desenler**, D'nin sırasıyla:
  1. Çizgili 45°, 3 mm;
  2. Çapraz 45°, 3 mm;
  3. Yatay çizgili, 2 mm;
  4. Dolu.
  - Aralık kâğıt milimetresidir; projenin pafta ölçeği onu metreye çevirir (3 mm, 1:1000'de 3 m).
- **Bölge, kapalı nesneyle** (varsayılan, Netcad'in yolu):
  - Tıklanan yeri çevreleyen en küçük görünür kapalı nesne alınır: kapalı alan, daire, tam elips ya da kapalı eğri (deponun `enclosing`'i).
  - Adalar açıkken, onunla kutusu kesişen ve alanı ondan küçük her kapalı nesne ada olur. Taramalar ve nesnenin kendisi sayılmaz.
  - Adalar bölgeden çıkarılır; tıklanan yeri içeren parça taranır.
  - Sonuç, çizim değişene dek aynı sınır nesnesi için saklanır.
- **Bölge, çizgilerle** (B, AutoCAD'in yolu): görünümdeki çizgilerin kapattığı yüz alınır. Adalar açıkken içindeki kapalı gruplar ada olur.
  - Nokta, yazı, ölçü ve tarama sınır sayılmaz.
  - Sınır katmanı seçildiyse yalnız o katmanın çizgileri sayılır.
  - Yüzler görünüm, çizim ya da sınır katmanı değişince yeniden kurulur (`FaceIndex`).
- **Sınır katmanı** (K, yalnız çizgilerle): o katmandan bir nesneye tıklanarak seçilir.
  - İmlecin altındaki nesne vurgulanır. Boş yer: "Sınır katmanını seçmek için bir nesneye tıklayın."
  - K yeniden bütün katmanlara döner. Esc önce seçmeyi bırakır.
  - Sınır katmanı çalıştırmanındır; desen, sınır yolu ve adalar uygulama açık kaldıkça hatırlanır (`Memory`).
- **Uyarılar:**
  - Kapalı nesne yoksa: "Tıklanan noktayı çevreleyen kapalı bir alan, daire ya da kapalı eğri yok. Çizgilerle çevrili yerler için “Sınır: çizgiler” seçin."
  - Yüz yoksa: "Tıklanan yer çizgilerle kapalı bir bölgenin içinde değil; görünüm dışındaki çizgiler sayılmaz."
  - Desen sıksa hiçbir şey yazılmaz (`hatch_lines` sınırı aşar): "Desen bu alan için çok sık; çizim ölçeğini büyütün ya da başka bir desen seçin."
- **Yazma:** tarama `cad.entities.create` ile etkin katmana yazılır. Kilitli katmanda komutun iletisi söylenir; gizli katmanda uyarıyla yazılır.
  - İleti: "Çizgili tarama eklendi: 272.00 m², 1 ada taranmadı". Alan projenin biriminde ve basamağındadır.
  - Araç kalır; her tık bir tarama daha ekler.
- **Tuşlar:**
  - Enter, Boşluk ya da kısa sağ tık araçtan çıkar.
  - Ctrl+Z çizimin geri almasıdır: son taramayı kaldırır ("Geri alındı: Tarama").
  - Kenet yoktur.
- **Önizleme:**
  - İmlecin altındaki bölge vurgu renginde, 1,5 px kesikli (4/3) çizilir.
  - Dolu desende %25 dolgu vardır. Çizgili desenlerde tarama çizgileri %60 soluk görünür; 3000 çizgiyi aşan önizleme çizgisiz kalır (web'deki gibi).
  - Önizlemeye kesikli alan (`Area::dash`) ve soluk çizgiler (`Preview::hatch`) eklendi.

### Sözleşme: `cad.entities.create`'in `hatch` işlemi

- Web taramayı "Tarama" adlı tek adımla yazar (`doc.transact('Tarama', …)`). Masaüstü aynı adı ürün komutuyla verir: `CreateOperation::Hatch`, adım "Tarama". Bu, Paralel çizgi ve Böl'ün yoludur.
- Değişenler:
  - sözleşme ve üretilen TypeScript (`CreateOperation.ts`, `commandCatalog.json`);
  - iki işleyicinin adı (`CREATE_LABEL`, `create::label`);
  - yeni ortak durum "Tarama: halka, adası ve deseniyle tek adım; adı “Tarama”" (`scripts/fixtures/create_command_cases.py`, 27 durum).
- Web'in aracı bugün belgeye doğrudan yazıyor; ürün komutuna taşınması web ajanının önerisindedir.

### Ortak iz

- `fixtures/interaction/v1/hatches.json`, `fixtures/interaction/v1/hatch.kcad` üstünde oynar. Çizimde 20 × 16 m parsel, içinde 8 × 6 m yapı, iki 12 × 16 m yüz kapatan çizgiler ve soldaki yüzde başka katmanda kapalı çoklu çizgi vardır.
- İz şunları iki platformda oynatır:
  - yapıyı ada bırakan tarama ve çizimin geri alması;
  - Adalar kapalıyken bütün parsel;
  - kapalı nesne olmayan yerin uyarısı;
  - çizgilerle yüz;
  - Sınır katmanını nesneyle seçme, Esc ve üzerine gelme;
  - Dolu desen ve Enter.

## Web'den ayrılanlar

- **İmleç:** web artı yerine seçme kutusu (`pick`) gösterir. Masaüstünün seçme aracı da artı gösterdiği için Tarama artıyla kalır.
- **Araç bitince vurgu:** web seçilen katmanın nesnesindeki vurguyu hemen bırakır. Masaüstünde, öbür kenar araçlarında olduğu gibi, bir sonraki fare hareketinde bırakılır.

## Doğrulama

- `crates/native/interaction/tests/hatch.rs` (9 test; alanlar elle hesaplandı):
  - parselde yapı ada: 272 m²;
  - yapının içi: 48 m²;
  - adalar kapalı: 320 m²;
  - kapalı nesne yokken uyarı;
  - çizgilerle yüz ve adası: 176 m², yüz yoksa uyarı;
  - sınır katmanı: 192 m², Esc sırası;
  - desenlerin sırası, Dolu ve hatırlananlar;
  - kilitli katman;
  - çok sık desen.
- `cargo test -p kentos-native-application` ve web Vitest: `cad.entities.create`'in 27 durumu iki işleyicide geçiyor.
- İz `hatches`: web `pnpm e2e:interaction` ve masaüstü `cargo test -p kentos-desktop traces`.
- `pnpm rust:test`, `pnpm rust:test:desktop`, `pnpm typecheck`, `pnpm test`, `pnpm build`, `pnpm e2e`, `pnpm inventory:check`.
- Görüntüler (`preview::screens`, `.run/shots/tarama-*`), koyu ve açık, 1440×900 ve 1100×650:
  - parselin önizlemesi (yapı ada);
  - çizgilerle yüz;
  - Dolu desen.
