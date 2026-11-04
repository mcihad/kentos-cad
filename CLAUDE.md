# KentOS CAD — Geliştirme kılavuzu

Güncelleme: 4 Kasım 2026. Bu dosya her oturumda okunan çekirdektir: çalışma biçimi, değişmez kurallar ve kılavuzun
geri kalanının haritası. Ayrıntılı bölümler [docs/kilavuz/](docs/kilavuz/) altındadır. Gelecek işler ve kabul kapıları
[TODOS.md](TODOS.md), görsel kurallar [DESIGN.md](DESIGN.md), karar gerekçeleri [docs/adr/](docs/adr/) içindedir.

Mevcut özellik ile hedefi ayırın. Kodun bulunması testlerin bu oturumda çalıştırıldığı ya da üretim kabulünün
tamamlandığı anlamına gelmez. Çelişkide kodu doğrulayın; ürün yönünde son kullanıcı kararlarını ve güncel TODOS.md
sınırlarını izleyin.

## Kılavuzun haritası

Bölüm numaraları korunur: koddaki ve ADR'lerdeki “CLAUDE.md §N” atıfları aşağıdaki dosyayı gösterir. Bu dosyada kalan
bölümler (3, 5–8, 11, 13–15, 18–20, 23) aşağıdadır.

| § | Dosya | Ne zaman okunur |
|---|---|---|
| 0 | [Güncel karar özeti](docs/kilavuz/00-guncel-karar-ozeti.md) | Bir modüle ya da özelliğe dokunmadan önce: o alanın son kararları, ADR'leri ve açıkları (Yol, mimari, form, bulut…) |
| 1 | [Ürün ve mevcut durum](docs/kilavuz/01-urun-ve-durum.md) | Ürünün kapsamı, modüller, neyin var neyin hedef olduğu |
| 2 | [Çalıştırma](docs/kilavuz/02-calistirma.md) | Bir komutun, testin, ortak durum betiğinin, resim testinin tam satırı |
| 4 | [Web mimarisi](docs/kilavuz/04-web-mimarisi.md) | Web'e dönüşte (web duruyor, ADR 0228); katmanlar, AppContext, durum kapsamları, komut, araç, çizim hattı |
| 9 | [Yeni iş ekleme tarifleri](docs/kilavuz/09-yeni-is-tarifleri.md) | Komut, araç (§9.2: araç = komut), ayar, test, render, processing aracı, dosya biçimi eklerken |
| 10 | [Yol haritası](docs/kilavuz/10-yol-haritasi.md) | Devir notu ve faz geçmişi |
| 12 | [Dosya haritası](docs/kilavuz/12-dosya-haritasi.md) | Hangi iş hangi dizinde |
| 16 | [Tenant, proje yetkisi ve paylaşım](docs/kilavuz/16-tenant-yetki-paylasim.md) | Yetki, rol, paylaşım |
| 17 | [Harita yayını](docs/kilavuz/17-harita-yayini.md) | Olası gelecek kapsamı |
| 21 | [Bağlantı, açılış ve autosave](docs/kilavuz/21-baglanti-acilis-autosave.md) | Sürekli bağlantı, asenkron açılış, otomatik kayıt |
| 22 | [Gelecekte 3D şehir, yol ve mimari](docs/kilavuz/22-gelecek-3d.md) | Uzun vadeli yön |
| 24 | [Ürün kapsamının izlenmesi](docs/kilavuz/24-kapsam-izleme.md) | Veri, iş akışı, paylaşım ve operasyon kapsamı |

Öbür başvurular: bir maddeyi baştan sona yapma sırası [docs/MADDE-TARIFI.md](docs/MADDE-TARIFI.md); Yol modülünün planı
[plan/road.md](plan/road.md), mimarininki [plan/architecture.md](plan/architecture.md); GUI resimlerinin kuralı
[docs/baseline/README.md](docs/baseline/README.md).

**Bu dosyaya ne yazılır:** yalnız her oturumda gereken kural ve haritanın satırı. Bir özelliğin kararı, ölçümü ve
geçmişi o alanın ADR'sine, planına ve §0'ın dosyasına yazılır; buraya en çok bir satırlık bağlantı. Dosya 4432
satıra (749 KB) büyüdüğü için bölündü.

## Çalışırken

- **Masaüstü ve bulut önce:** web'e ara verildi (ADR 0228); web'e ait adımlar atlanır, gerekenleri TODOS.md §0.2'ye yazılır.
- **Ağır iş tek tek:** derleme, test ve ölçüm `scripts/dev/svc.sh heavy <komut>` ile ortak kilidin altında koşar
  (`scripts/dev/svc.sh heavy cargo test -p kentos-desktop --lib …`); ikincisi bekler, masaüstü aç kalmaz.
- **Masaüstü testleri** `RUST_MIN_STACK=67108864` ister. Pencere resimleri `#[ignore]` `screens` testleridir:
  `cargo test -p kentos-desktop --lib <modül>_tests::screens -- --ignored --nocapture`.
- **GUI kanıtı:** resimler `.run/gui-evidence/` altına yazılır, Git'e eklenmez; görüntü yokluğu doğrulama başarısı
  sayılmaz. Resim bakılarak gözden geçirilir, kusur düzeltilir, yeniden çekilir.
- **Araştırma önce:** her yeni mimari adımda üreticilerin resmî belgeleri (Civil 3D, OpenRoads, NETPRO…) araştırılır;
  kaynak ve KentOS kararı ADR'ye ve plana yazılır. Değer uydurulmaz, kaynak beklenir.
- **Araç = komut** (§9.2): çizimi değiştiren her iş anlamsal komutla yapılır; katalog, ≥20 bağımsız başvurulu ortak durum,
  Python, MCP ve manifest olmadan bitmiş sayılmaz.
- **Pencere düzeni** (DESIGN.md §7.19): Yol pencereleri `road_dialog::RoadDialog` iskeletindedir; çizim alanı
  `widget::picture::Picture` görüntüleyicisidir (ADR 0937), tablolar kitin veri tablosudur (ADR 0938).
- **Kabul:** görünür ayar, anlaşılır önizleme, az işlem, tek onay/Esc/undo; planlar yalnız kanıtlanan alt maddelerle
  ilerletilir, bütün görevler bitmiş gibi sunulmaz.

## 3. Teknik kısıtlar

- TypeScript strict, `erasableSyntaxOnly`, `verbatimModuleSyntax`: `enum`,
  `namespace`, constructor parameter property kullanmayın; `import type`,
  union ve `as const` tercih edin. Web araç zinciri pnpm/Vite/TypeScript'tir.
- Web UI çatısı DOM `h()` (`ui/dom.ts`) ve `core/signal.ts`'tir.
  React/Vue/Lit geçişi yapmayın.
- Yeni runtime bağımlılığı veya sürüm değişikliği bakım, lisans, platform ve
  ölçüm gerekçesi ister; mevcut küçük/pure çekirdek sınırını koruyun.
  Yeni web runtime kütüphanesi küçük, tree-shaking'e uygun, MIT/BSD lisanslı,
  bakım altında ve DOM gerektirmeden worker'da kullanılabilir olmalı;
  eklemeden kullanıcı onayı alın. Rust sürümleri workspace/lockfile politikasına uysun.
- Tarayıcının ayırdığı `Ctrl+N/T/W`, `Ctrl+Shift+T`, `Alt+F/D/E`
  kısayollarını uygulama komutuna bağlamayın.

## 5. Koordinat, birim ve gösterim

- Kodda `x = doğu`, `y = kuzey`; Türk ölçmeciliği UI'ında sıra
  `Y (sağa), X (yukarı)`dır. Noktalı ondalık ve `Y,X` girişi korunur.
- Her proje CRS/SRID taşır; metadata kaynağı `geo/crs.ts`'tir.
  SRID atamak koordinat dönüşümü değildir. Kaynak CRS bilinmiyorsa sorun;
  sessiz reprojection/fallback veya tahmin yapmayın.
- Geometri açısı ile kuzeyden saat yönünde ölçmecilik semtini karıştırmayın.
  Birim dönüşümü açık; metre, m², dönüm (1000 m²), hektar (10000 m²).
- UI sayıları `ctx.format` üzerinden gösterilir; `toFixed` iş kuralı değildir.
  Gösterim basamakları kaynak koordinatı değiştirmez; ayrıntılı doğruluk §23'tedir.

## 6. Performans

### 6.1 Bütçeler

Hedef/ölçüm kaynağı ADR 0005 ve `docs/perf/`'dir; ADR taslak durumunu korur.
Donanım/driver/browser/kalite/commit bilgisi olmadan karşılaştırma yapmayın.
TODOS.md §20 güncel kabul kapsamıdır; eski tile hedefleri bugünkü cloud kapısı değildir.

### 6.2 Sıcak yol kuralları

Pointermove/frame içinde allocation, JSON/FFI, bütün belge gezisi ve tam GPU
upload'ı azaltın; toplu tipli veri kullanın. Uzamsal sorgu indeksten, uzun iş
worker'dan, uzun liste sanallaştırmadan geçer. DOM okuma/yazmayı ayırın.
Optimizasyonu aynı veri ve kalite düzeyinde ölçün; geometri/etiketi gizlice eksiltmeyin.

### 6.3 Durum ve darboğazlar

Rust pick/snap deposu, katman dizinleri, kirli güncelleme ve ağaç sanallaştırması
mevcuttur; bunları yeni yapılacak işler gibi yeniden kurmayın. Güncel darboğazı
profil çıkararak belirleyin; eski tek-makine ölçümlerini bugünün sonucu diye aktarmayın.

## 7. CAD/GIS veri doğruluğu

Belge değişiklikleri geri alınabilir ve kilitli katman kurallarına uygun olmalı.
Kilitli katmandaki nesnenin kopyası da yapılmaz; dizilerde de (ADR 0037, 0047). Düzenleme (`cad.entities.edit`) bütün
yazılır ya da hiç: bir nesnesi kilitliyse hepsi reddedilir (ADR 0047).
Gizli katmana çizimde uyarı, atlanan/kayıplı nesnelerde açıklanabilir rapor gerekir.
Geometrik alan, kayıtlı/hukuki alan ve gösterim değeri ayrıdır; birbirini
otomatik ezmez. CAD analitik kaynak ve GIS izdüşümü §15'e uyar.
Topoloji, parsel/hisse ve resmî çıktılar bağımsız referans/kurum kabulü olmadan
tamamlanmış veya mevzuata uygun diye sunulmaz.

## 8. Kod kuralları

Çevredeki adlandırmayı/üslubu koruyun; yorum nedenini anlatsın. Tek dosya tek
sorumluluk taşısın; başında kısa amaç yorumu olsun, 400 satır üstünde ayrıştırmayı değerlendirin.
Signal'i sahibi değiştirir; UI command/belge API'sini kullanır.
Sabit UI rengi, vurgu, font boyu veya CDN fontu eklemeyin; CSS jetonları,
`readCanvasPalette()` ve yerel fontları kullanın. Katman rengi veridir;
renderer/UI davranışını sabit katman ID'sine göre dallandırmayın.
Hata mesajı nedeni ve çözümü söylesin. Global listener/GPU/worker kaynaklarının
dispose/cancel yolu olsun. Kullanıcının mevcut değişikliklerini koruyun.

### 8.1 Tamamlanma

İlgili testler, sözleşme/undo/yetki, UI gerekiyorsa gerçek klavye/fare ve
tema/yazı ölçeği kontrolü yapılır. Çalıştırılmayan kontrol ve nedenini bildirin.
Üretim iddiası veya tamamlandı işareti için TODOS.md kabul kapılarını kullanın.
Komut, araç, ayar, pencere, depo ya da `.kcad` alanı değiştiyse `pnpm inventory`
çalıştırın; farkı okuyup değişiklikle aynı commit'e koyun (docs/inventory/README.md). Web dururken (ADR 0228) envanter donmuştur;
yeni komutların kaydı `catalog/v1/additions.json`'dadır, web'de gerekenler TODOS.md §0.2'ye yazılır.
Çizimi ya da projeyi değiştiren yeni araç ve komut, §9.2'deki komut, Python ve MCP bütünlüğü olmadan tamamlandı sayılmaz.

## 11. Teknik borç

Borçları TODOS.md'de ilgili görev/kabul koşuluyla izleyin. Mevcut kodda
olmayan kusuru eski nottan hareketle yeniden düzeltmeye çalışmayın:
transaction rollback, gerçek kaydet/aç, backend, cloud, stil/SVG Rust çekirdeği
ve sanallaştırma temelleri zaten vardır. Native desktop, geniş
paylaşım, typed attributes ve otomasyonun kalan kapsamı ayrı gelecek iştir.
Production DB TLS ve bağımsız gerçek ortam doğrulaması gibi eksikleri özellik
varlığına bakarak kapatmayın.

## 13. Rust backend ve depolama sınırı

Backend vardır: Axum/Tokio/SQLx ile PostGIS, kimlik/tenant, proje işlemleri ve WS.
Cloud dosya modunda binary revizyonlar nesne deposunda, katalog/izinler DB'de
olabilir; projeyi buluta koymak zorunlu CAD→GIS import'u değildir. Saklama biçimi
(`database`/`file`) proje açılırken seçilir ve değişmez (öbür biçime geçiş, kaynağı değiştirmeyen
`project.convert` ile yeni projedir, ADR 0039); dosya revizyonu yükleme →
doğrulama → `project.file.commit` ile yazılır, depo ile DB arasında ortak işlem
varsayılmaz (ADR 0031). Canlı DB
modunda PostgreSQL/PostGIS otoritedir; projenin `.kcad` görüntüsü tek bir salt okunur
`REPEATABLE READ` işlemde, kilit almadan okunur (`Db::snapshot`, ADR 0033). Özel DB/WAL/MVCC motoru tasarlamayın.
Sunucu transaction'ı istemcinin undo/transaction sorumluluğunun yerine geçmez.

### 13.1 Entegrasyon

Mevcut `project.changes` expected version, idempotency, audit/outbox temellerini
genişletin. UI command registry ile bu protokolü tek kavram sanmayın.
Yerel hızlı edit, dosya dayanıklılığı ve server commit onayı ayrı anlam taşır.
Oluşturulan nesnenin sürümü commit'in veri revizyonudur; silinen kimlik yeniden
oluşturulabilir, eski sürüme dayanan değişiklik çakışmadır (ADR 0026).
Sunucu hatası `ApiError`'dur: sabit kod (`error`), Türkçe ileti, bilinen alanın yolu, çakışmada
revizyon, yeniden deneme bilgisi (`retryable`, `retryAfter`; ARCH-07, ADR 0013). Alanı bilinen
doğrulamada `AppError::invalid_at` kullanın; istemci yeniden denemeyi bu alanlardan karar verir.

## 14. Workspace ve tek hesaplama kaynağı

Saf Rust hesap crate'leri DOM, Iced, SQLx, ağ veya host runtime bilmez.
Web bunları WASM, desktop/server native çağırır. Rust'tan üretilen veri/komut
sözleşmesi ortak olabilir; web uygulama runtime'ı ortaklaştırılmaz.
Sıcak yollarda toplu tipli buffer kullanın; nesne başına JSON/FFI üretmeyin.
Mevcut `jsmath`/`libm`, lint ve deterministic davranış kurallarını koruyun;
`unwrap/expect` ile kullanıcı girdisinde panic üretmeyin.
Yeni hesap çağrısına sınır/rastgele fixture, native/WASM ve bağımsız referans
ekleyin. Taşıma ayrıntıları [ADR 0008](docs/adr/0008-shared-core-boundary.md)'dedir.

## 15. CAD ve PostGIS kaynak otoritesi

Mevcut `crates/server/application/src/cad.rs`: basit geometride `geom`,
analitik CAD'de `cad_definition` kaynak; GIS geometri türevdir. Kaynak türü,
projection sürümü/toleransı ve revision birlikte korunur. Gelecekteki blok,
ölçü, ilişki ve proje metadata'sı da round-trip sözleşmesine dahil edilir.
Doğrudan PostGIS projesinde `.kcad` metadata/bağlantı referansı taşıyabilir;
parola taşımaz. Tam snapshot/export seçimi açık olmalı; aynı veriye iki
bağımsız yazılabilir otorite kurmayın. Ayrıntı TODOS.md §10–11, ADR 0006.

## 18. Command, Python/AI ve ağır işler

Ürün komutları tipli/sürümlü, yetkili, iptal/önizleme/sonuç sözleşmeli olmalı.
Python ve AI bunları sarar; repository/SQL/DOM'dan iş kurallarını atlamaz.
Web processing worker'ı mevcut; kalıcı server worker ayrı teslimdir.
Server job tasarımında lease/heartbeat/fencing, idempotency, staged sonuç,
revision/yetkiyi commit anında doğrulama ve kaynak kotaları gerekir.
Uzun hesap API event loop/UI thread'de çalışmaz; socket kapanması işi yok etmez.

## 19. Teslim ve kabul

Faz sırası TODOS.md §22'dedir. Kütüphane eklemek veya ekran göstermek tek başına
teslim değildir. Proje bulutu önceliği eski yayın fazlarının yerini alır.

### 19.0 Kabul kapıları

İlgili correctness, platform uyumu, yetki, veri kaybı/restore ve performans
testlerinin kanıtını verin. Test atlamayı başarı, golden üretmeyi bağımsız
doğrulama, çalışan bir örneği bütün ürünün tamamlanması saymayın.

## 20. Lazy load ve modül yaşam döngüsü

### 20.1 Mevcut sınırlar

Stil/SVG düzenleyicisi, format pencereleri, ribbon ve ağır modüllerin mevcut
JS/CSS/WASM yükleme sınırlarını koruyun; başlangıca gereksiz import eklemeyin.

### 20.2 Yükleme

Yükleme async, hatası görünür ve yeniden denenebilir olsun. İptal/geç kalan
sonuç açık belgenin veya yeni oturumun üstüne uygulanmasın.

### 20.3 Yaşam döngüsü

Aç/kapat, tema/backend değişimi ve yeniden login'de listener, worker,
GPU kaynağı ve task temizlenir; mount/unmount tekrarları sızıntı yaratmasın.

### 20.4 Ölçüm

Startup ve modül açılışını üretim bundle'ında ölçün. WASM boyutu izlenir,
ancak kaldırılmış keyfi başlangıç boyut sınırını yeniden eklemeyin.
Python/3D/ileri analiz sıradan 2D açılışa zorunlu yük olmamalı.

## 23. Sayısal ve kadastral doğruluk

Kaynak sınır, koordinat, alan ve hak doğruluğu performanstan önce gelir.
f64/decimal veya ortak Rust tek başına sıfır hata/kadastral uygunluk kanıtı değildir.

### 23.1 Kayıpsız sayı yolu

Kaynağın hassasiyet, birim, CRS/datum ve gerekiyorsa özgün ondalık değerlerini
koruyun. Kesin decimal/rasyonel değerler JS Number üzerinden geçmez;
decimal string ve pay/payda sözleşmesi kullanılır. `NUMERIC` ölçeğinin örtük
yuvarlamasına güvenmeyin; taşma/NaN/Infinity kontrollü hatadır.
Geometrik alan, kayıtlı alan ve gösterim ayrı anlamlardır.

### 23.2 Yuvarlama

`numeric_policy_id/version`, birim, ölçek, eşitlik yönü, ara adımlar ve
dağıtım artığı kuralı açık/sürümlü olmalı. Gösterim metni hesap girdisi değildir.
`toFixed`, `Math.round`, SQL round veya kütüphane varsayılanını iş kuralı yapmayın.
Hisse/alan toplamı korunur; artığı rastgele son parsele vermeyin.
Onaysız veya belirsiz kuralda nihai kayıt tamamlanmaz.

### 23.3 Geometrik kararlar

Robust/adaptive gerektiğinde exact predicates kullanın; kesin predicate
üretilen koordinatın kesinliğini garanti etmez. Seçim piksel toleransı,
snap/topoloji toleransı, hesap hata sınırı ve kaynak belirsizliği ayrıdır.
Gizli snap/grid/repair/simplify veya büyük epsilon ile hata örtmeyin.
Alan/uzunluk kaynaktan ve açık düzlemsel/elipsoidal/3D yöntemle hesaplanır;
render tessellation'ı nihai ölçü değildir. CRS grid/epoch/sürümü kaydedilir;
eksik grid'de sessiz düşük doğruluk fallback'i yapılmaz.
Eşiği kesen hata aralığında hassasiyeti artırın; hâlâ belirsizse `needs_review`
veya atomik ret, dayanağı kayıtlı çözüm gerekir.

### 23.4 Bağımsız doğrulama

Native/WASM eşitliği yanında bağımsız yüksek hassasiyetli referans gerekir.
Sınır, yarım değer, negatif, yakın paralel/teğet, büyük koordinat, delik/eğri,
ortak sınır ve alan/hisse korunumu fixture'larını koruyun.
Nihai decimal, yuvarlama yönü ve topolojik karar platformlar arasında aynı
olmalıdır; farkta server sonucunu sessiz seçmeyin. Algoritma/build, kaynak
revision, numeric policy ve dönüşüm provenance'ı sonucu yeniden üretilebilir kılsın.
Mevcut toleransı büyütmek veya beklenen fixture'ı hataya göre yenilemek yasaktır.
Ayrıntı: ADR 0004/0008, TODOS.md §3/§20 ve [docs/specs/numeric-policy.md](docs/specs/numeric-policy.md) (dört tür sayı, toleranslar,
büyük koordinatta küçük ayrıntının testleri).
