# KentOS CAD — Geliştirme kılavuzu

Güncelleme: 28 Kasım 2026. Bu dosya her oturumda okunan çekirdektir: çalışma biçimi, değişmez kurallar ve kılavuzun
geri kalanının haritası. Ayrıntılı bölümler [docs/kilavuz/](docs/kilavuz/) altındadır. Gelecek işler ve kabul kapıları
[TODOS.md](TODOS.md), görsel kurallar [DESIGN.md](DESIGN.md), karar gerekçeleri [docs/adr/](docs/adr/) içindedir.

Mevcut özellik ile hedefi ayırın. Kodun bulunması testlerin bu oturumda çalıştırıldığı ya da üretim kabulünün
tamamlandığı anlamına gelmez. Çelişkide kodu doğrulayın; ürün yönünde son kullanıcı kararlarını ve güncel TODOS.md
sınırlarını izleyin.

## Kılavuzun haritası

Bölüm numaraları korunur: koddaki ve ADR'lerdeki “CLAUDE.md §N” atıfları aşağıdaki dosyayı gösterir. Bu dosyada kalan
bölümler (3, 5–8, 11, 13–15, 18–20, 23, 25, 26) aşağıdadır.

| § | Dosya | Ne zaman okunur |
|---|---|---|
| 0 | [Güncel karar özeti](docs/kilavuz/00-guncel-karar-ozeti.md) | Bir modüle ya da özelliğe dokunmadan önce: o alanın son kararları, ADR'leri ve açıkları (Yol, mimari, form, bulut…) |
| 1 | [Ürün ve mevcut durum](docs/kilavuz/01-urun-ve-durum.md) | Ürünün kapsamı, modüller, neyin var neyin hedef olduğu |
| 2 | [Çalıştırma](docs/kilavuz/02-calistirma.md) | Bir komutun, testin, ortak durum betiğinin, resim testinin tam satırı |
| 4 | [Web mimarisi](docs/kilavuz/04-web-mimarisi.md) | Kaldırılan web uygulamasının mimarisi (ADR 9402; tarih). Koddaki “§4.N” atıflarının gösterdiği davranışlar (durum kapsamı, komut, klavye, araç, kayıt) masaüstünde geçerlidir |
| 9 | [Yeni iş ekleme tarifleri](docs/kilavuz/09-yeni-is-tarifleri.md) | Komut, araç (§9.2: araç = komut), ayar, test, render, processing aracı, dosya biçimi, ayrı işletim sistemi penceresi (§9.8), yeni modül ve yeni istemci (§9.9) eklerken |
| 10 | [Yol haritası](docs/kilavuz/10-yol-haritasi.md) | Devir notu ve faz geçmişi |
| 12 | [Dosya haritası](docs/kilavuz/12-dosya-haritasi.md) | Hangi iş hangi dizinde |
| 16 | [Tenant, proje yetkisi ve paylaşım](docs/kilavuz/16-tenant-yetki-paylasim.md) | Yetki, rol, paylaşım |
| 17 | [Harita yayını](docs/kilavuz/17-harita-yayini.md) | Olası gelecek kapsamı |
| 21 | [Bağlantı, açılış ve autosave](docs/kilavuz/21-baglanti-acilis-autosave.md) | Sürekli bağlantı, asenkron açılış, otomatik kayıt |
| 22 | [Gelecekte 3D şehir, yol ve mimari](docs/kilavuz/22-gelecek-3d.md) | Uzun vadeli yön |
| 24 | [Ürün kapsamının izlenmesi](docs/kilavuz/24-kapsam-izleme.md) | Veri, iş akışı, paylaşım ve operasyon kapsamı |

Öbür başvurular: bir maddeyi baştan sona yapma sırası [docs/MADDE-TARIFI.md](docs/MADDE-TARIFI.md); Yol modülünün planı
[plan/road.md](plan/road.md), mimarininki [plan/architecture.md](plan/architecture.md) ve
[plan/architecture-advanced.md](plan/architecture-advanced.md) (iş sırası §5, madde talimatı §6); BIM yönetmeliği ve IFC
uyumu [plan/bim-plan.md](plan/bim-plan.md) (sıra §6) ve [plan/ifc-uyum.md](plan/ifc-uyum.md), yönetmelik eklerinin metni
[mevzuat/bim/](mevzuat/bim/README.md); GUI resimlerinin kuralı [docs/baseline/README.md](docs/baseline/README.md); bulut
portalı (`apps/portal`) [ADR 9000](docs/adr/9000-cloud-portal.md) ve [plan/bulut.md](plan/bulut.md).
Masaüstündeki portal dosyaları [ADR 9001](docs/adr/9001-desktop-portal-files.md)'dedir.
E-imza (KentOS İmza: crate'ler, yerel servis, PIN ajanı, masaüstünde süreç içi imza) [ADR 9002](docs/adr/9002-e-imza.md)'dedir.
Veri sürücüleri (GeoPackage, PostGIS; ortak sözleşme `kentos-data`), canlı veri katmanı ve düzenleme oturumu [plan/veri.md](plan/veri.md) ve [ADR 9100](docs/adr/9100-data-drivers.md)'dedir.
Render motoru (arayüzden bağımsız tuval, LOD, NCZ ve DWG alımı) [plan/render.md](plan/render.md) ve [ADR 9200](docs/adr/9200-render-engine-research.md)'dedir.
Mimari'nin 3B görünümü, kesiti ve eksik araçları [plan/mimari-bim.md](plan/mimari-bim.md) ve [ADR 8171](docs/adr/8171-mimari-3d-section-quality-and-missing-tools.md)'dedir.
Bakım ve refactor planı (kod incelemesinin bulguları, biçim tabanı, `make guard`) [plan/bakim.md](plan/bakim.md) ve [ADR 9400](docs/adr/9400-maintenance-plan.md)'dedir.

**Bu dosyaya ne yazılır:** yalnız her oturumda gereken kural ve haritanın satırı. Bir özelliğin kararı, ölçümü ve
geçmişi o alanın ADR'sine, planına ve §0'ın dosyasına yazılır; buraya en çok bir satırlık bağlantı. Dosya 4432
satıra (749 KB) büyüdüğü için bölündü.

## Çalışırken

- **Masaüstü ve bulut:** web uygulaması ve WASM desteği sistemden kaldırıldı (sahibin sözü, 28 Kasım 2026;
  [ADR 9402](docs/adr/9402-web-and-wasm-removed.md)). `apps/web` ve `crates/wasm` yoktur ve geri gelemez (`make guard`);
  web'e ya da WASM'a ait adım, not ve atıf yazılmaz. Tarayıcıda çalışan tek program bulut portalıdır (`apps/portal`,
  ADR 9000); sunucunun tarayıcı istemcisi (`x-kentos-client: web`, `kentos_launch::Client::Web`) odur.
  - Komut kataloğu `catalog/v1/commands.json`'dadır; ayar şeması `catalog/v1/settings-schema.json`'da.
  - TypeScript yalnız portal için üretilir (`apps/portal/src/api/generated`, elle düzenlenmez): portalın kullandığı tür
    `derive(TS)`, adını andığı tür ayrıca `ts(export)` alır. Başka türe `ts` özniteliği yazılmaz.
- **Modül ve istemci ayrımı (kesin kural, 23 Kasım 2026):** §25'in sekiz kuralı her değişiklikte uygulanır; yeni
  panel, sekme, menü öğesi, komut, modül ya da istemci onlarla eklenir.
- **Modül ve otomasyon denetimi (kesin kural, 24 Kasım 2026):** yeni yapılan ya da güncellenen her işte üç şey açıkça
  saptanır. Eksik bulunan aynı işte tamamlanır, sonraya bırakılmaz:
  1. **Modüller:** Mimari, Harita, Yol (CAD) ve CBS projelerinin her biri bu özelliği taşıyacak mı? Her modül için var ya
     da yok, gerekçesiyle (§25 kural 2); yeni modül eklendikçe o da. Var denen her modülde çalışır ve sınanır.
  2. **Kanallar:** MCP, Python, eklenti ve yapay zekâ ajanı aynı işi yapabiliyor mu? Her biri için var ya da yok.
  3. **Tam uyum:** her kanal işi arayüzle aynı komutla yapar; doğrulama, yetki, önizleme ve sonuç aynıdır. Kanal kendi
     hesabını yazmaz.

  Karar tablosu (modül × var/yok, kanal × var/yok) işin ADR'sine yazılır. “Yok” yalnız gerekçeyle yazılır; örneğin geri
  al, yinele ve dosya işleri eklentiye kapalıdır. Var olan bir işe dokunulunca aynı denetim onun için de yapılır.
  Kanalların nerede bağlandığı ve hangi sınamanın koşacağı §9.2'dedir; biri kırmızıyken iş bitmiş sayılmaz.
- **Bekçiler ve biçim (28 Kasım 2026, ADR 9400):** kod `cargo fmt --all` ile biçimlenir; commit'ten önce `make guard`
  yeşildir (bağımlılık yönü, kaynak boyu, modül bağları, CAD manifesti, kaldırılanlar, biçim; derleme istemez). Boy sınırı
  biçimlenmiş kodu ölçer: satır sıkıştırmak ya da başka bir dolanma yok, dosya bölünür. Bir crate'in bütünleşik
  sınamaları tek ikilidir (`tests/all/main.rs`).
- **Ağır iş tek tek:** derleme, test ve ölçüm `scripts/dev/svc.sh heavy <komut>` ile ortak kilidin altında koşar
  (`scripts/dev/svc.sh heavy cargo test -p kentos-desktop --lib …`); ikincisi bekler, masaüstü aç kalmaz.
- **Masaüstü testleri:** sınama iş parçacığının 64 MB yığınını cargo verir (`.cargo/config.toml` `RUST_MIN_STACK`, ADR 9400);
  ikiliyi cargo'suz koşan `RUST_MIN_STACK=67108864` verir. Pencere resimleri `#[ignore]` `screens` testleridir:
  `cargo test -p kentos-desktop --lib <modül>_tests::screens -- --ignored --nocapture`. Kendi paketine ayrılan modülün
  testleri onun paketindedir (CBS `-p kentos-desktop-cbs`, Mimari `-p kentos-desktop-arch`, Harita `-p kentos-desktop-map`, Yol `-p kentos-desktop-road`), programlar arası olanlar (yüzey matrisi,
  katman menüsü, izler) `cargo test -p kentos-cad`'dedir; program `cargo build -p kentos-cad` (ADR 9199).
- **GUI kanıtı:** resimler `.run/gui-evidence/` altına yazılır, Git'e eklenmez; görüntü yokluğu doğrulama başarısı
  sayılmaz. Resim bakılarak gözden geçirilir, kusur düzeltilir, yeniden çekilir.
  Kullanıcıya dönük kabul gerçek pencerede, gerçek fare ve klavyeyle yapılır: `scripts/usage/real/` (ADR 8000).
- **Araştırma önce:** her yeni mimari adımda üreticilerin resmî belgeleri (Civil 3D, OpenRoads, NETPRO…) araştırılır;
  kaynak ve KentOS kararı ADR'ye ve plana yazılır. Değer uydurulmaz, kaynak beklenir.
- **Başka uygulamaya atıf yok (kesin kural, 10 Kasım 2026):** arayüzde (pencere, ipucu, durum çubuğu, ileti),
  kod yorumlarında ve kodda (ad, sabit, test adı, metin) hiçbir başka uygulama (Netcad, AutoCAD, Civil 3D, ArcGIS,
  QGIS, OpenRoads, MicroStation…) açıkça anılmaz; karşılaştırma ve kaynak yalnız ADR'de, planda ve araştırma notunda
  durur. Şeritteki (ribbon) ipuçları ve arayüz metinleri ADR'ye de atıf yapmaz (“ADR …”, “docs/adr/…” yok). Dokunulan
  her dosyada eski atıflar da aynı işte temizlenir; yeni kodda hiç yazılmaz. Dosya biçimi adları (DXF, DWG, LandXML,
  GeoPackage…) uygulama adı sayılmaz.
- **Etiket CBS'nindir (kesin kural, 26 Kasım 2026):** etiket kavramı yalnız CBS'dedir. Bu kavram katmanın
  etiketlemesini, etiket araçlarını, Nokta etiketleri ve Nokta kodları düğmelerini ve Etiketleri yazıya çevir'i kapsar.
  CAD modüllerinde (Mimari, Harita, Yol) sahip açıkça istemedikçe etiket sunulmaz: katman menüsünde Etiketler, şeritte
  etiket komutu, ayarlarda etiket satırı, işlem aracının yeni katmanında etiket stili olmaz. CAD'de nokta adı noktaya
  bağlı yazıdır (ADR 9314); etiket desteği ortak çekirdekte kalır. CAD'de stil seçilir (Katman stili). Stil içe alma ve
  başka programların stil dosyaları (Stil yükle/kaydet, SLD, QML, MapLibre) yalnız CBS'dedir
  ([ADR 9316](docs/adr/9316-labels-are-gis-only.md)).
- **Araç = komut** (§9.2): çizimi değiştiren her iş anlamsal komutla yapılır; katalog, ≥20 bağımsız başvurulu ortak durum,
  modüllerin kararı, Python, MCP, yapay zekâ ajanı, eklenti ve manifest olmadan bitmiş sayılmaz.
- **Pencere düzeni** (DESIGN.md §7.19): Yol pencereleri `road_dialog::RoadDialog` iskeletindedir; çizim alanı
  `widget::picture::Picture` görüntüleyicisidir (ADR 0937), tablolar kitin veri tablosudur (ADR 0938).
- **Kabul:** görünür ayar, anlaşılır önizleme, az işlem, tek onay/Esc/undo; planlar yalnız kanıtlanan alt maddelerle
  ilerletilir, bütün görevler bitmiş gibi sunulmaz.

## 3. Teknik kısıtlar

- TypeScript kuralları bulut portalı (`apps/portal`) içindir. Strict,
  `erasableSyntaxOnly`, `verbatimModuleSyntax`: `enum`, `namespace`, constructor parameter property
  kullanmayın; `import type`, union ve `as const` tercih edin. Portalın araç zinciri pnpm/Vite/TypeScript'tir.
- Portal UI çatısı DOM `h()` ve `Signal`'dır (`apps/portal/src/lib/dom.ts` ve `signal.ts`, portalın kendi
  dosyaları). React/Vue/Lit geçişi yapmayın.
- Yeni runtime bağımlılığı veya sürüm değişikliği bakım, lisans, platform ve
  ölçüm gerekçesi ister; mevcut küçük/pure çekirdek sınırını koruyun.
  Yeni portal runtime kütüphanesi küçük, tree-shaking'e uygun, MIT/BSD lisanslı
  ve bakım altında olmalı; eklemeden kullanıcı onayı alın. Rust sürümleri workspace/lockfile politikasına uysun.
- Portalda tarayıcının ayırdığı `Ctrl+N/T/W`, `Ctrl+Shift+T`, `Alt+F/D/E`
  kısayollarını uygulama komutuna bağlamayın.

## 5. Koordinat, birim ve gösterim

- Kodda `x = doğu`, `y = kuzey`; Türk ölçmeciliği UI'ında sıra
  `Y (sağa), X (yukarı)`dır. Noktalı ondalık ve `Y,X` girişi korunur.
- Her proje CRS/SRID taşır; metadata kaynağı `crates/native/project/src/crs.rs`'tir (kayıt `fixtures/crs/v1/registry.json`).
  SRID atamak koordinat dönüşümü değildir. Kaynak CRS bilinmiyorsa sorun;
  sessiz reprojection/fallback veya tahmin yapmayın.
- Geometri açısı ile kuzeyden saat yönünde ölçmecilik semtini karıştırmayın.
  Birim dönüşümü açık; metre, m², dönüm (1000 m²), hektar (10000 m²).
- UI sayıları projenin ayarlarından `kentos_domain::format::Format` üzerinden gösterilir (`crates/native/domain/src/format.rs`); yuvarlama
  ve `toFixed` iş kuralı değildir.
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
sorumluluk taşısın; başında kısa amaç yorumu olsun, 400 satır üstünde ayrıştırmayı değerlendirin. Hiçbir Rust dosyası
2 000 satırı geçmez: `scripts/dev/source_size.py` (`make guard`) denetler; bugün aşanlar tabanda, büyüyemez, eklemeden önce bölünür.
Signal'i sahibi değiştirir; UI command/belge API'sini kullanır.
Sabit UI rengi, vurgu, font boyu veya CDN fontu eklemeyin; CSS jetonları,
yerel fontları kullanın. Katman rengi veridir;
renderer/UI davranışını sabit katman ID'sine göre dallandırmayın.
Hata mesajı nedeni ve çözümü söylesin. Global listener/GPU/worker kaynaklarının
dispose/cancel yolu olsun. Kullanıcının mevcut değişikliklerini koruyun.

### 8.1 Tamamlanma

İlgili testler, sözleşme/undo/yetki, UI gerekiyorsa gerçek klavye/fare ve
tema/yazı ölçeği kontrolü yapılır. Çalıştırılmayan kontrol ve nedenini bildirin.
Üretim iddiası veya tamamlandı işareti için TODOS.md kabul kapılarını kullanın.
Masaüstü kataloğunun tabanı dondurulmuş envanterdir (`docs/inventory/web.json`; tek kataloğa geçene kadar,
plan/bakim.md BK-23); komut, araç, ayar, pencere, depo ya da `.kcad` alanı değiştiyse yeni komutların kaydı
`catalog/v1/additions.json`'dadır.
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
Masaüstü ve sunucu bunları native çağırır. Rust'tan üretilen veri/komut sözleşmesi ortaktır.
Sıcak yollarda toplu tipli buffer kullanın; nesne başına JSON/FFI üretmeyin.
Mevcut `jsmath`/`libm`, lint ve deterministic davranış kurallarını koruyun (yerlerine geçecek sayısal temel
araştırılıyor: ADR 9402 §7, plan/bakim.md BK-88; karar çıkana kadar bu kurallar sürer);
`unwrap/expect` ile kullanıcı girdisinde panic üretmeyin.
Yeni hesap çağrısına sınır/rastgele fixture ve bağımsız referans
ekleyin. Taşıma ayrıntıları [ADR 0008](docs/adr/0008-shared-core-boundary.md)'dedir.

**IFC (kesin kural):** IFC ile ilgili her şey `crates/shared/ifc` (`kentos-ifc`) içindedir: yazma, okuma, sınıf,
ad, özellik seti, denetim. Başka crate IFC kodu yazmaz; IFC'ye dokunan kod değiştiğinde oraya taşınır ve diğer
crate'ler onun işlevlerini, metotlarını ve türlerini doğrudan kullanır (uygulamada yalnız komut/belge bağlayıcısı
kalır). Ayrıntı [ADR 8047](docs/adr/8047-ifc-building-export.md).

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
Kalıcı server worker ayrı teslimdir.
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

Ağır modülleri (stil/SVG düzenleyicisi, format pencereleri, Python, 3B, ileri analiz) başlangıçta yüklemeyin;
ilk kullanımda yükleyin.

### 20.2 Yükleme

Yükleme async, hatası görünür ve yeniden denenebilir olsun. İptal/geç kalan
sonuç açık belgenin veya yeni oturumun üstüne uygulanmasın.

### 20.3 Yaşam döngüsü

Aç/kapat, tema/backend değişimi ve yeniden login'de listener, worker,
GPU kaynağı ve task temizlenir; mount/unmount tekrarları sızıntı yaratmasın.

### 20.4 Ölçüm

Başlangıç ve modül açılışını üretim (release) derlemesinde ölçün.
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

Bağımsız yüksek hassasiyetli referans gerekir; sonuç her platformda (macOS, Windows, Linux; masaüstü ve sunucu) aynıdır.
Sınır, yarım değer, negatif, yakın paralel/teğet, büyük koordinat, delik/eğri,
ortak sınır ve alan/hisse korunumu fixture'larını koruyun.
Nihai decimal, yuvarlama yönü ve topolojik karar platformlar arasında aynı
olmalıdır; farkta server sonucunu sessiz seçmeyin. Algoritma/build, kaynak
revision, numeric policy ve dönüşüm provenance'ı sonucu yeniden üretilebilir kılsın.
Mevcut toleransı büyütmek veya beklenen fixture'ı hataya göre yenilemek yasaktır.
Ayrıntı: ADR 0004/0008, TODOS.md §3/§20 ve [docs/specs/numeric-policy.md](docs/specs/numeric-policy.md) (dört tür sayı, toleranslar,
büyük koordinatta küçük ayrıntının testleri).

## 25. Modül ve istemci ayrımı (kesin kurallar)

Modüller (bugün CBS, Mimari, Harita, Yol; yarın başkaları) ve istemciler (masaüstü, mobil görüntüleyici ve
düzenleyici, web) ayrı programlara bölünebilecek kadar gevşek bağlıdır. Bu kuralları çiğneyen iş bitmiş sayılmaz.
Ayrıntı [ADR 9195](docs/adr/9195-module-separation.md), [ADR 9196](docs/adr/9196-storage-by-project-kind.md), tarif
§9.9, sıra [plan/ayrim.md](plan/ayrim.md).

1. **Ekranı projenin modülü belirler.** Şerit, paneller, alt sekmeler, katman menüsü, ayarlar, biçimler ve sihirbaz
   açık projenin modülünün profilinden gelir. Bu her programda ve her istemcide böyledir; hepsi bir arada program
   (`kentos-cad`) da istisna değildir.
2. **Her yüzey açıkça yerleşir.** Yeni panel, sekme, menü öğesi, ayar ya da komut her modül için ayrı ayrı var/yok
   kararıyla eklenir; "her yerde" öntanımlısı yoktur.
   - Profil tabloları joker kolsuz eşleşmedir: yeni değer, her modüle yerleşmeden derlenmez.
   - Matris sınaması her hücreyi sınar.
   - Katman menüsünün öğesi modülle ve katmanın türüyle (çizim, açıklama, veri tablosu, bağlı, servis, raster, nokta
     bulutu, grup) yerleşir.
3. **Kabuk modülü tanımaz.** Kabuk ve istemciler yüzey için modül adına göre dallanmaz; modülün kaydettiği profili
   sorar.
   - Modül başka modüle doğrudan bağlanmaz; ortak çekirdeğe ve sözleşmeye bağlanır.
   - Bağımlılık denetimi (`scripts/dev/module_deps.py --check`; masaüstü sınamaları da koşar) yeşil kalmadan iş
     bitmez; tabanı yalnız azalır. Kabuk yüzeyi modülün profiline sorar (`apps/desktop/src/shell/module_profile.rs`).
4. **Modülün tanımı ortaktır.** Modülün kimliği, proje türleri, işleri, saklama türü, komut kümesi ve katman türleri
   paylaşılan sözleşmededir (`kentos-launch`; ortak katalog `catalog/v1/modules.json`, üretici
   `KENTOS_WRITE_MODULES=1 cargo test -p kentos-desktop --lib module_catalog`). Masaüstü, mobil ve web aynı tanımı
   okur; istemci başına modül mantığı çatallanmaz.
5. **Saklama modeli proje türündendir.**
   - **CBS:** katman verisi ve öznitelikler veritabanındadır (GeoPackage, PostgreSQL, KentOS Bulut). Proje dosyası
     yapıyı, stilleri, ayarları ve açıklamaları (yazı, ölçü, pafta) tutar; dosyaya, veritabanına ya da buluta
     kaydedilir. Bulutta yalnız veritabanı biçimi vardır.
   - **CAD (Mimari, Harita, Yol):** projenin tamamı proje dosyasındadır; dosyaya ya da buluta kaydedilir (yalnız dosya
     revizyonu).
   - CBS projesinin kaydı nesne tutan çizim ya da bağlı katmanı proje dosyasına yazmaz. Kural ve katmanın türü ortak
     çekirdektedir (`kentos_native_application::layer_kind`); her istemcinin kaydı onu sorar.
   - Yeni modül bu iki türden birini seçer; üçüncü bir biçim ADR ister.
6. **Veri yalnız komutla değişir.** Masaüstü, mobil düzenleyici, Python ya da MCP fark etmez: çizim ve veri komutla
   yazılır, CBS verisi düzenleme oturumuyla ya da sunucu komutuyla. İstemci kuralı atlayarak dosyaya ya da
   veritabanına yazmaz. Rol ve form kuralları (ortak çözümleyici `kentos_forms::reach`, sunucuda `form_guard`) her
   istemcide aynıdır.
7. **Görüntüleyici yazmaz.** Mobil ya da web görüntüleyici yalnız okur. Rolün okumadığı alanı göstermez, adını
   söyler. Her istemcinin neyi yazdığı ortak tanımdadır (`kentos_launch::Client::writes`, ADR 9197): mobil uygulama
   CBS'nin veri satırlarını sunucu komutuyla yazar, proje dosyasını hiç yazmaz.
8. **Yeni modül ya da istemci tarifle eklenir** (§9.9). Tarif sırasıyla: modül tanımı, profil, sihirbaz, komut
   kümesi, saklama türü, bulut türleri, matris sınaması, bağımlılık denetimi, resim.

## 26. Kurulum paketi ve dizinler (kesin kural, 26 Kasım 2026)

Programın çalışırken okuduğu her dosya aşağıdaki yerlerden birindedir ve yerini `kentos_launch::install`'dan sorar
(ADR 9312). Yeni bir dizin önce bu tabloya ve `install.rs`'e eklenir; kurulum paketi yalnız bu tablodan kurulur.

- **`<program>`:** programların klasörü (Windows ve Linux'ta kurulum klasörü, macOS'ta `KentOS.app/Contents/MacOS`).
- **`<kaynak>`:** programın yalnız okuduğu dosyalar (Windows ve Linux'ta kurulum klasörü, macOS'ta
  `KentOS.app/Contents/Resources`). Depoda `paket/`'tir; paketçi içini olduğu gibi kopyalar.
- **`<kullanıcı>`:** kullanıcının kurduğu ve yazdığı dosyalar (macOS `~/Library/Application Support/kentos-cad`,
  Windows `%APPDATA%\kentos-cad`, Linux `$XDG_DATA_HOME/kentos-cad`).

| Depoda | Kurulumda | İçerik |
|---|---|---|
| derlenen programlar | `<program>/` | `kentos-cad`, `kentos-launcher`, modül adları (`kentos-mimari` …), `kentos`, `kentos-mcp` |
| `paket/fonts/` | `<kaynak>/fonts/` | bütün yazı tipleri, lisansları yanlarında: `cizim/` çizimin, `arayuz/` arayüzün, `yedek/` eksik harflerin yüzleri. Programlar yazı tipi listesini buradan ve `<kullanıcı>/fonts/`'tan kurar (ADR 9313). Arial'ın ve Times New Roman'ın dosyası depoya konmaz (lisansı dağıtmayı yasaklar, ADR 9311) |
| `paket/eklentiler/` | `<kaynak>/eklentiler/` | KentOS ile gelen eklentiler, her biri bir klasör; örnek Python işlem aracı `ornek-isleme/` |
| `paket/betikler/` | `<kaynak>/betikler/` | örnek Python betikleri |
| `catalog/standards/` | `<kaynak>/catalog/standards/` | standart kural paketleri (Python) |
| SDK tekerleği, uv | `<program>/tools/` (macOS: tekerlek `<kaynak>/python/`, uv `<program>/`) | Python ortamı |
| — | `<kullanıcı>/eklentiler/`, `fonts/`, `betikler/`, `python/` | kullanıcının kurdukları ve yazdıkları |

- **Python işlem aracı** eklenti olarak yazılır: tanımı `tools/<id>.tool.json` (`modules`, `category`), işi
  `@ext.processing`. Örneği `paket/eklentiler/ornek-isleme/`'dir (ADR 9310).
- **Depodan çalışan program** kaynakları `paket/`'ten bulur (`target/` üstü). `KENTOS_RESOURCES_DIR` ve
  `KENTOS_USER_DIR` başka yeri gösterir.
- **Yazı tipi** yalnız `paket/fonts/` altına konur; kod yüzü oradan alır (`include_bytes!` da oradan). Yeni bir
  program açılışta `kentos_headless::load_fonts()`'u (masaüstü `drawing_fonts::load_font_folders()`'ı) çağırır.
  Yazı tipi listesi her yerde `font_folders::choices`'tan gelir; sabit liste yazılmaz (ADR 9313).
