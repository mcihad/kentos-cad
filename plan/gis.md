# KentOS — Kullanılabilir GIS için öncelikli geliştirme planı

İnceleme: **17 Ekim 2026**. Yerel dal `main`; kaynak taraması `9c230413` üzerinde başlamış, son kontrolde HEAD **`507f95e0`** olmuştur. Çalışma ağacındaki devam eden değişiklikler de okunmuştur. GitHub'daki açık [PR #25 — forms-core](https://github.com/mcihad/kentos-cad/pull/25), **`5307026911844d754862b78c3106deb7296f0d4b`** anlık görüntüsündeki gerçek kaynaklardan ayrıca incelenmiş; araştırma sırasında eklenen **`3106609e12a037016353f826a2610bab0c6a01ba`** commit'inin native/sözleşme/çekirdek kod farkı ve ADR 0441'i de değerlendirilmiştir. PR devam etmektedir; bu sabit noktalardan sonraki commit'ler ve birleştirme sonucu yeniden değerlendirilmelidir.

**Hedef:** Kullanıcı KentOS'ta gerçek bir GIS projesini veri bulma/açma, doğru koordinatta birlikte gösterme, öznitelik ve geometri düzenleme, sorgulama, analiz, kalite kontrolü, kaydetme ve yeniden açma boyunca tamamlayabilsin. Dünya standartlarını aşma hedefi; araç sayısıyla değil doğruluk, veri kaybını önleme, büyük veride çalışma ve aynı işin otomasyondan güvenilir tekrarıyla ölçülsün.

**Belgenin düzeni:** Takip listesi **50 satırdır: 6 mevcut yetenek `[x]`, 9 kategoride 44 kalan iş `[ ]`** (`CBS-01…CBS-44`). Her açık kutunun başlığı yalnız kalan işi tanımlar; altındaki **Mevcut** paragrafı yeniden geliştirilecek iş değildir. CAD geliştirme işleri ve tamamlanma kayıtları ayrı [cad.md](cad.md) dosyasındadır. Bu plan, diğer planlarla ortak altyapı ve bağımlılıkları aşağıda açıkça eşler.

**Bu teslim:** Kod ve plan incelemesi, birincil kaynak araştırması ve uygulanabilir geliştirme planıdır. Uygulama kodu değiştirilmemiş; KentOS/QGIS/ArcGIS karşılaştırmalı uygulama testleri, performans ölçümleri ve PR kabul testleri bu incelemede çalıştırılmamıştır. Aşağıdaki kabul koşulları ölçülmüş sonuç değil, geliştirmede geçirilecek kapılardır.

## Kesin kapsam ve teknoloji kararları

- **QGIS karşılaştırma tabanı 4.2'dir.** Resmî 4.2 değişiklik kaydı, 4.2 API belgeleri ve `release-4_2` kaynakları esas alınır. Araştırma gününde resmî yol haritası 4.2.3'ü desteklenen sürümler arasında gösteriyor. Bazı `/4.2/` kullanıcı kılavuzu alt adresleri `testing`'e yönlendi: bu sayfalar açıklayıcı yardımcı kaynaktır; yalnız bunlarda görülen bir davranış 4.2 özelliği diye kabul edilmez. 3.44 eski referansları veya 4.3/testing yenilikleri karşılaştırmaya sessizce karıştırılmaz. [Q01], [Q02], [Q03]
- **Masaüstü önceliklidir; web tamamen durmuştur.** `apps/web` ekranı, envanterdeki `implemented` işareti veya üretilmiş TS tipi, çalışan ürün kanıtı değildir. İşler masaüstü, ortak Rust, gereken sunucu yolları, Python ve MCP üzerinde teslim edilir. Ortak sözleşme üretimi sürebilir; bu plan web geliştirmesini veya web testlerini başlatmaz. [ADR 0228](../docs/adr/0228-web-pause-desktop-and-cloud-first.md)
- **Oracle veya başka ticari veritabanı olmayacaktır.** Oracle Spatial, SQL Server, SAP HANA veya ücretli/proprietary veritabanı motoru için sağlayıcı, kurulum ya da lisans gereksinimi bu yol haritasına girmez. Çok kullanıcılı veri için **PostgreSQL/PostGIS**, yerel taşınabilir GIS verisi için **SQLite/GeoPackage** esas alınır. SpatiaLite ancak somut veri uyumluluğu ihtiyacında açık kaynak adaptör olarak değerlendirilir; ikinci zorunlu depolama sistemi yapılmaz.
- **Yerel GIS sunucu kurmadan kullanılabilmelidir.** `.kcad` proje/çizim biçimi korunur; GeoPackage onun yerine geçirilmez. Bağlı veri, gömülü veri, çevrimdışı çalışma kopyası ve hesap türevi ayrı anlam taşır. Mevcut PostgreSQL'in transaction/MVCC motoru kullanılır; özel veritabanı motoru tasarlanmaz.
- QGIS ve ArcGIS'ten iyi iş akışları alınır. ArcGIS Enterprise/Online, Esri geodatabase, SDE veya ticari servis üyeliği KentOS'un önkoşulu olmaz. Dış ArcGIS REST verisini **isteğe bağlı okuyabilmek**, böyle bir altyapıya bağımlı olmak değildir. FileGDB uyumluluğu gerekirse açık kaynak **OpenFileGDB** adaptörüyle sınırlı dosya alışverişi olarak ele alınır. [F03]
- GDAL/OGR, PROJ ve gerekirse GEOS; lisans, dağıtım, sürüm ve doğruluk kararı verilerek **native adaptör/süreç sınırında** değerlendirilebilir. Saf `crates/shared` çekirdeklerine I/O, UI veya zorunlu C/C++ bağımlılığı sokulmaz. Kütüphane seçmek doğruluk kabulünün yerine geçmez.

## Koddan çıkan durum ve araştırmanın dayanakları

**Takip kuralı:** `[x]` belirtilen dar kapsamın mevcut kodda uygulanmış olduğunu; `[ ]` başlıkta yazan ek işin kaldığını gösterir. Bir özellik zaten varsa yeni özellik diye tekrar açılmaz. Kalan iş yeni sağlayıcı entegrasyonu, somut genişleme veya adı açık bir kabul çalışması olabilir; sıradan regresyon kontrolü mevcut özelliği eksik yapmaz. Bir açık kutu, altındaki kalan iş ve tamamlanma ölçütü bitince `[x]` yapılır ve tarih/ADR/commit kanıtı eklenir. **PR'de** işareti, özelliğin belirtilen PR commit'inde bulunduğunu söyler; `main`'de birleşmiş veya bu oturumda test edilmiş olduğu anlamına gelmez.

### Zaten uygulanmış yetenekler — yeniden yapılmayacak

- [x] **MEVCUT-01 — Yerel belgede ortak köşe ve kenarları birlikte düzenleme.** Köşe taşıma/ekleme/silme, ortak yayın biçimini değiştirme ve esnetmede komşu geometriler aynı düzenleme/undo adımına katılır; delik/parça ve kot kuralları vardır. Kilitli komşuyu değiştirmeyip sayısıyla bildirmek mevcut karardır. Kanıt: [çekirdek](../crates/shared/geometry-core/src/ops/topology_edit.rs), [native uygulama yolu](../crates/native/application/src/topology_follow.rs), [araç bağlantısı](../crates/native/interaction/src/neighbours.rs), [etkileşim testleri](../crates/native/interaction/tests/all/topology_edit.rs), [ADR 0160](../docs/adr/0160-topological-editing.md). **CBS-12 bu özelliği yeniden yapmak değildir.**

- [x] **MEVCUT-02 — 13 topoloji kuralı, bulguyu haritada gösterme, düzeltme ve istisna.** Mevcut Topoloji paneli ve kural motoru kullanılacak. Kanıt: [kurallar](../crates/shared/geometry-core/src/ops/topology_rules/mod.rs), [masaüstü paneli](../apps/desktop/src/topology/mod.rs). CBS-14'ün kalan işi artımlı ve kaynak revision'ına bağlı kalite kaydıdır.

- [x] **MEVCUT-03 — Native semboloji, sınıflandırma ve kurallı etiketleme.** Sembol/stil kitaplığı, `.kstil`, kategorili/dereceli/kurallı görünüş, etiket önceliği/engeller ve sabit pin temeli mevcut. Kanıt: [native stil](../crates/native/style/src/lib.rs), [etiketleme](../apps/desktop/src/labelling/mod.rs). CBS-17/18 yalnız yeni veri sağlayıcısındaki bağlama ve hedeflenen ek kabulü izler.

- [x] **MEVCUT-04 — Parçalı GeoTIFF/BigTIFF okuma, raster piramidi ve altı georeferans yöntemi.** Raster ekleme/yerleştirme mevcut; yeniden raster veya georeferans motoru kurulmayacak. Kanıt: [raster I/O](../crates/shared/formats/src/raster/mod.rs), [piramit](../apps/desktop/src/rasters/pyramid.rs), [georeferans](../crates/shared/geometry-core/src/ops/georef.rs), [native akış testleri](../apps/desktop/src/rasters/tests.rs). CBS-28/29 yeni kaynak/mozaik/COG ve kalıcı işlem kaydı kapsamıdır.

- [x] **MEVCUT-05 — Rota, servis alanı, OD/en yakın tesis, şebeke izleme ve temel LRS.** Bunların native hesap ve araç yolları mevcut. Kanıt: [ağ çekirdeği](../crates/shared/geometry-core/src/ops/network/mod.rs), [LRS](../crates/shared/geometry-core/src/ops/lrs/mod.rs), [native işlem araçları](../crates/native/processing/src/builtin/mod.rs). CBS-26/27 dönüş kısıtı, terminal/varlık modeli ve kaynak sürümünü izleme gibi ek kapsamı taşır.

- [x] **MEVCUT-06 — Yerleşik Processing araçları, model çalıştırma, arka plan ve tek undo.** Araç/model altyapısı yeniden yazılmayacak. Kanıt: [runner](../crates/native/processing/src/runner.rs), [model runner](../crates/native/processing/src/model_runner.rs), [masaüstü arka planı](../apps/desktop/src/processing/background.rs). CBS-23/24 yeni provider girdisi, tipli çıktı/batch ve açık doğruluk sorunlarını; CBS-39 kalıcı sunucu işini izler.

Bu altı işaret bütün ürünün kabulünün tamamlandığı iddiası değildir; belirtilen mevcut yeteneklerin yeniden iş açılmasını önler. PR #25'in form/domain/ilişki işleri de aşağıdaki P25 kanıtlarıyla mevcut PR kapsamıdır; yeniden yapılacak form görevleri sayılmaz.

| Kanıt | Okunan gerçek kaynaklar ve çıkan sonuç |
|---|---|
| K01 — Mimari ve öncelik | [Workspace](../Cargo.toml), [geliştirme kuralları](../CLAUDE.md), [ana yol haritası](../TODOS.md): ortak Rust hesap, native uygulama, ayrı masaüstü renderer, mevcut PG/PostGIS ve durdurulmuş web. Eski belgeler güncel kodun yerine kullanılmadı. |
| K02 — Veri ve şema | [EntityBase](../crates/shared/contracts/src/entity.rs), [alanlar](../crates/shared/contracts/src/fields.rs), [katman](../crates/shared/contracts/src/layer.rs), [proje](../crates/shared/contracts/src/document.rs): kalıcı kimlik altyapısı var; öznitelikler hâlâ metin haritası. Yerel `main` beş, PR yedi alan türü taşıyor. PR'de de tam `int64`/UUID/JSON/null ayrımı çözülmüş değil. |
| K03 — Kaynak açma | [Kaynaklar paneli](../apps/desktop/src/sources/mod.rs), [biçim modülleri](../crates/shared/formats/src/lib.rs): klasör ve KentOS projesinden katman alma var; KentOS projesinden alınan katman bir snapshot/kopya. Genel dış veri sağlayıcısı, GeoPackage, GeoParquet ve FlatGeobuf için native uygulama yolu bulunamadı. |
| K04 — Sunucu verisi | [Proje ve feature sayfalama](../crates/server/application/src/projects.rs), [CAD otoritesi](../crates/server/application/src/cad.rs), [değişiklik commit'i](../crates/server/application/src/changes.rs), [temel şema](../crates/server/postgres/migrations/0001_foundation.sql): yönetilen `kentos.feature`, UUID, sayfalama, sürüm/commit ve CAD kaynak–GIS türev ayrımı mevcut. `features()` kimlik üzerinden sayfalıyor; bu tek başına dış PG tablosuna bağlanan bbox/kolon/filtre sağlayıcısı değildir. |
| K05 — CRS ve ölçme | [CRS çekirdeği](../crates/shared/geometry-core/src/crs.rs), [native CRS bağlamı](../crates/native/project/src/crs.rs), [ölçme](../crates/shared/geometry-core/src/crs/measure.rs), [GNSS aktarımı](../crates/native/interaction/src/gnss.rs): TM/coğrafi/Mercator/yerel, Helmert/NTv2 ve düzlem/elipsoid/yer ölçü temelleri var. CRS çekirdeği açıkça 2D; genel dünya CRS kataloğu, düşey/epoch dönüşümü ve kaynak CRS'sini koruyan genel katman sağlayıcısı tamamlanmış sayılmaz. |
| K06 — Tablo ve sorgu | [Native öznitelik tablosu](../apps/desktop/src/features/mod.rs), [tablo sıralaması](../crates/shared/geometry-core/src/ops/feature_table.rs), [mekânsal ilişkiler](../crates/shared/geometry-core/src/ops/spatial_query.rs), [katman filtresi](../crates/shared/contracts/src/layer_filter.rs): düzenleme, seçime/görünüme göre süzme, ifade ve türüne göre sıralama var. Altı ilişki ve sabit `1e-3` tolerans, tam DE-9IM sözleşmesi değildir. |
| K07 — Düzenleme ve kalite | [Ortak sınır düzenleme](../crates/shared/geometry-core/src/ops/topology_edit.rs), [13 topoloji kuralı](../crates/shared/geometry-core/src/ops/topology_rules/mod.rs), [native hata paneli](../apps/desktop/src/topology/mod.rs): komşu köşe/kenar izleme, denetleme, düzeltme ve istisna mevcut. Bunların kaynaklar arası transaction ve artımlı kalıcı kalite yaşam döngüsü genişletilmeli. |
| K08 — Bilinen doğruluk açığı | [Ağ testindeki GEO-01](../crates/shared/geometry-core/tests/all/network.rs), `TODOS.md` GEO-01/GEO-02: zincirlenen birleşimlerde alanın iki kez sayılmasını gösteren test kaynakta hâlâ `ignore` taşıyor; değen tamponlarda performans açığı ayrıca kayıtlı. Bu incelemede test koşturulmadı; hatanın kapandığı varsayılmadı. |
| K09 — İşlemler | [97 kaynak dosyalı yerleşik araç ailesi](../crates/native/processing/src/builtin/mod.rs), [runner](../crates/native/processing/src/runner.rs), [model runner](../crates/native/processing/src/model_runner.rs), [masaüstü arka plan işi](../apps/desktop/src/processing/background.rs): buffer/overlay, raster, ağ, LRS, istatistik, modeller, iptal ve tek undo temeli güçlü. Bu sayı araç sayısı değildir. Kalıcı server job, genel sağlayıcı girdisi ve bütün Processing kataloğunun tek otomasyon yolu ayrıca kapanmalı. |
| K10 — Kartografya | [Native stil](../crates/native/style/src/lib.rs), [stil çekirdeği](../crates/shared/style-core/src/lib.rs), [etiket UI](../apps/desktop/src/labelling/mod.rs), [bölünmüş görünüm](../apps/desktop/src/split_view.rs), [pafta çekirdeği](../crates/shared/sheet/src/lib.rs): sınıflandırma, kurallı görünüş/etiket, sembol kitaplığı, iki görünüm, pafta ve PDF mevcut. Yeniden stil/etiket motoru yazmak yerine kaynak sağlayıcısıyla bütünleştirme ve çıktı kabulü gerekir. |
| K11 — Raster ve büyük veri | [Raster I/O](../crates/shared/formats/src/raster/mod.rs), [georeferans](../crates/shared/geometry-core/src/ops/georef.rs), [raster analiz](../crates/shared/raster/src/lib.rs), [nokta bulutu](../crates/shared/pointcloud/src/lib.rs), [çok boyutlu biçimler](../crates/shared/formats/src/multidim/mod.rs): parçalı TIFF/BigTIFF, piramit, altı georeferans yöntemi, çok sayıda analiz, LAS/LAZ/COPC/VPC ve klasik NetCDF/mesh temeli var. Bütün raster biçimleri, COG üretim uygunluğu veya tam 3D ürün desteği buradan çıkarılamaz. |
| K12 — Servis, zaman ve ağ | [Servis çekirdeği](../crates/shared/services/src/lib.rs), [native veri alma](../apps/desktop/src/services/feed_window.rs), [ağ çekirdeği](../crates/shared/geometry-core/src/ops/network/mod.rs), [LRS](../crates/shared/geometry-core/src/ops/lrs/mod.rs), [zaman](../apps/desktop/src/temporal/mod.rs): WMS/WMTS/XYZ/MVT, WFS/OGC Features/ArcGIS okuma, veri yenileme, rota/servis alanı/izleme ve zaman-senaryo var. Veri alma+yenileme yolu proje CRS'sine kopyalıyor; canlı harici feature düzenleme ile aynı ürün değildir. |
| K13 — Otomasyon ve yetki | [Headless dispatch](../crates/native/headless/src/dispatch.rs), [sorgu](../crates/native/headless/src/query.rs), [Python](../crates/native/python/src/lib.rs), [MCP](../crates/native/mcp/src/tools.rs), [sunucu erişimi](../crates/server/application/src/access.rs): kullanılacak mevcut omurga var. CAD komutunun yayımlanmış olması her GIS seçeneğinin/Processing aracının otomasyona açıldığını kanıtlamaz. |
| P25-A — Form ürünü | [PR form çekirdeği][P25A], [form sözleşmesi][P25B], [alanlar][P25C]: tasarımcı, yayın, kurallar, hesap, lookup, stable binding, datetime/time ve başka pek çok iş var. Bu plan bunları yeniden yapılacak form özellikleri diye yazmaz. |
| P25-B — Kayıt ve ilişkiler | [PR submission][P25D], [ilişki izleme][P25E], [ADR 0440][P25F]: geometrisiz kayıt, 1:N/N:M, silme politikası, alt kayıtlarla atomik yerel gönderim, snapshot ve undo cevabı var. N:M yerelde çok değerli başvuru alanıyla temsil ediliyor; dış SQL junction tablosu desteği kendiliğinden oluşmuyor. |
| P25-C — Merkezi form sınırı | [PR server guard][P25G], [ADR 0411][P25H], [ADR 0439][P25I]: design/publish/use izinleri, entry rolü, alan okuma izdüşümü ve sunucu yazma denetimi mevcut. ADR'ler dış kaynak lookup, görev/kişi ataması, yetkili ilişkili kayıt sayfalama ve sunucunun ilişkiler arası işlem sınırının kalan kısımlarını açıkça ayırıyor. |
| P25-D — Son PR farkı | [ADR 0441][P25J], [ortam gereksinimi hesabı][P25K], [native form girişi][P25L]: bölüm kitaplığı, taslak kurtarma, review yorumları, ilişkili kayıt grafiği, kayıtlı Processing aracını açan form eylemi ve `requires` profili eklenmiş. Eylem, aracı seçili kayıtla açıyor; otomatik ortak job/executor yüzeyinin bütünüyle tamamlandığı anlamına gelmiyor. Bunlar yeniden form işi olarak planlanmadı. |

**Ana sonuç:** KentOS'ta eksik olan yalnız yeni analiz düğmeleri değildir. Öncelikli boşluk; mevcut analiz ve CAD doğruluğunu **gerçek GIS veri sağlayıcılarına, büyük tabloya, tutarlı düzenlemeye ve taşınabilir projeye** bağlamaktır. `TODOS.md`'deki `PG-*`, `FMT-*`, `GEO-*`, `GIS-*` işleri burada birleştirilir; aynı özellik için ikinci uygulama açılmaz.

### QGIS 4.2 ve ArcGIS'ten seçilen iyi yaklaşımlar

| Kaynak yaklaşım | KentOS'ta karşılığı ve yapılacak fark |
|---|---|
| QGIS 4.2 provider/capability ve CRS işlem bağlamı [Q04], [Q05] | Kopyalama akışının yanına canlı dataset, kaynak CRS ve yetenek tablosu: CBS-01/02/04/07. |
| QGIS 4.2 FeatureServer görünüm sorgusunun verimliliği [Q02] | Servisten veri alma mevcut; bbox/kolon/filtreyle çalışan genel provider ve ölçülmüş istek bütçesi: CBS-02/42/43. |
| ArcGIS query layer [A09] | Kaynağı kopyalamadan SQL/view sonucu harita ve tablo olarak açma; kararlı kimlik ve açık salt okunurluk: CBS-04/09. Ticari DB eklenmez. |
| ArcGIS calculation/constraint/validation ayrımı [A08] | PR'deki hesap, kayıt kuralları ve audit korunur; dış sağlayıcı ve sunucu transaction'ında aynı anlamla uygulanır: CBS-14/15/38. |
| ArcGIS Error Inspector ve dirty area [A01], [A05] | Mevcut 13 kural/panel üzerine artımlı, revision'a bağlı QA ve güvenilir istisna: CBS-12/14/27. |
| QGIS 4.2 model adım sayıları ve ArcGIS ModelBuilder [Q02], [A03] | Mevcut modeller üzerine tipli kaynak/sink, batch, canlı sayılar ve tekrar üretim kaydı: CBS-24/41. |
| ArcGIS sürümlü düzenleme [A06] | Mevcut expected-version/idempotency temelinden çatışma ve kontrollü review; ayrı DB motoru kurulmaz: CBS-37. |
| QGIS 4.2 GeoPDF katman yönetimi ve ArcGIS map series [Q02], [A02] | Mevcut pafta motoru üzerinden veri revision'ına bağlı atlas ve izinli export: CBS-21. |

**Daha ileri ürün hedefi:** Bu yaklaşımları hassas analitik CAD kaynağı, form/ilişki transaction'ı, açıklanabilir veri kaybı ve eksiksiz komut/Python/MCP yüzeyiyle tek iş akışında birleştirmek. Bunun rakiplerden daha iyi olduğu ancak aynı veri ve görevin doğruluk, süre, kurtarma ve tekrar üretim kabulüyle gösterilebilir.

## Öncelik, sahiplik ve ilk kullanılabilir sürüm

**P0:** İlk yerel GIS'in veri güveni ve günlük kullanım kapısı. **P1:** Kurum verisiyle üretim, performans ve gelişmiş iş akışları. **P2:** İleri disiplin ve isteğe bağlı dağıtım. Bir madde birden fazla öncelik taşıyorsa ilk sürümde yalnız açıkça belirtilen alt dilimi gerekir; 44 kalan işin tamamı ilk sürümün önkoşulu değildir. Mevcut altı yetenek doğrudan kullanılır; yeniden uygulanmaz.

| Mevcut plan | Bu GIS planıyla iş paylaşımı |
|---|---|
| [form.md](form.md) + PR #25 | Form standardı, designer, domain/subtype/contingent, ilişkiler ve form kuralları bunların sahipliğinde kalır. Buradaki CBS-06/09/15/35/38, bunları sağlayıcıya ve çok kullanıcılı GIS'e taşır. PR'de kalan bulut ve dış kaynak işleri yeniden doğrulanır. |
| [python.md](python.md) | `.kek`, uv, isteğe bağlı eklenti ortamı, broker ve yayın kataloğu bu planın işidir. Yerleşik GIS araçları Python kurulumu veya plugin manager beklemez; CBS-41 aynı komut yüzeyini tamamlar. |
| [road.md](road.md) | Kalıcı TIN, breakline, koridor, analitik yol ve doğrulanmış kazı/dolgu oradadır. GIS DEM, ağ/LRS ve CRS kaynakları ortak kullanılır; ikinci yol mühendisliği motoru kurulmaz. |
| [3d.md](3d.md) | Gerçek 3D kamera, büyük dünya precision, 3D Tiles, Digital Twin ve fizik solver oradadır. CBS-32/33 veriyi/kimliği bağlar; kullanılabilir 2D GIS bu işlerin tamamlanmasını beklemez. |
| [Önceki CAD planı](cad.md) | Hassas çizim, analitik eğri, blok, ölçü, pafta ve komut temeli korunur. CBS-16, CAD kaynağını GIS'e kontrollü eşler; geometriyi tek tip polyline'a indirgemez. |

| Teslim dilimi | Somut kullanıcı sonucu ve gereken madde dilimleri | Çıkış kanıtı |
|---|---|---|
| G0 — Temeli eşle | PR'nin güncel commit'ini ve birleşmiş sözleşmeleri envantere işle; CBS-02/06/13/15/36/44'ün kaynak, kimlik ve doğruluk kararlarını netleştir. | Şema/fixture sürüm çakışmaları ayrılmış; GEO-01 için bağımsız doğrulama ve düzeltme yolu seçilmiş. PR başlığı veya eski tamamlandı işareti kanıt yerine geçmez. |
| G1 — **İlk kullanılabilir yerel GIS** | GeoPackage/parsel/bina aç → farklı CRS'li ortofotoyla göster → tablo/form/geometri düzenle → konuma göre seç ve temel overlay → mevcut topoloji araçlarıyla denetle → GeoPackage'a kaydet → `.kcad` projeyi kapat/aç. CBS-01…03, 05…07, 11, 13, 15…17, 20, 22…23, 28, 36, 40…42, 44'ün asgari dilimleri. Yerel topolojik düzenleme/denetim, stil, etiket ve modeller mevcut koddan kullanılır; yeni CBS-12/14 genişlemeleri G1'in önkoşulu değildir. | Kimlik, öznitelik, ilişki, geometri ve CRS korunumu; iptal/undo; kaynak kopması; QGIS 4.2 ile açık dosya alışverişi. Sunucu, mobil, tam 3D, eklenti mağazası ve yayın servisi gerekmez. |
| G2 — Kurum GIS'i | CBS-04/06/09/12/14/15/18/24/37…39/41/42: gerçek dış PostGIS tablosu, provider'da ortak sınır commit'i, artımlı QA, yetkili sorgu/yazma, çatışma, büyük veri ve dayanıklı job. | İki kullanıcılı düzenleme, yetki iptali, kesilen bağlantı ve restore senaryoları; kaynakta değişiklik yapan QGIS istemcisiyle birlikte çalışma. |
| G3 — Analiz ve kartografya üretimi | CBS-10/17…34: mevcut motorların ürün kabulü, batch/model, atlas, ağ, raster/uzaktan algılama ve zaman. Alanlara göre ayrı teslim edilir. | Her teslimde bağımsız hesap referansı, veri provenance'ı, gerçek veri performansı ve aynı otomasyon sonucu. |
| G4 — Saha ve ileri dağıtım | CBS-27/32…35/43'ün ileri dilimleri; form, 3D ve yol planlarının ilgili teslimleri. | Çevrimdışı replika ve güvenli eşitleme; seçilen açık servis profili; ileri domain doğruluğu. Harita yayınlama ayrıca kapsamlandırılır. |

## A — Veri açma ve gerçek veri kaynakları

- [ ] **CBS-01 — Kaynak gezginine canlı GeoPackage/PostGIS ve servis bağlantıları ekle — P0**

  *Kısmen (17 Ekim 2026; [ADR 0503](../docs/adr/0503-bound-layers.md)): GeoPackage tablosu bağlı katman olarak açılır (`cad.sources.edit`), bağ proje kapatılıp açılınca korunur (`.kcad` şema 74), taşınan dosya yeniden bağlanır (relink), yeri bilinmeyen kaynak nesneleriyle açılır; Katmanlar'da rozet ve Kaynak ▸. Kalan: kaynak gezgininde dosyanın katmanlarını eklemeden önizleme ve servis katalogları. PostGIS tablosu ADR 0505'le bağlanır.*

  **Mevcut:** Kaynaklar paneli klasörleri ve KentOS projelerinden katman kopyalamayı biliyor [K03]. GIS kullanıcısı için verinin nerede olduğu, canlı mı kopya mı olduğu ve neden yazılamadığı aynı yerde anlaşılmalı.

  **Yapılacak kalan iş:** Kaynak gezginine GeoPackage içindeki katman/tablo, PostGIS bağlantısı, raster klasörü ve servis kataloglarını ekle. Ad/konum/tür/CRS/şema araması, sık kullanılanlar, son kaynaklar, örnek satırlar, kapsam önizlemesi ve sürükleyerek ekleme sun. Katman kartında kaynak, yazma yeteneği, kimlik alanı, filtre, yükleme kapsamı ve güncellik görülsün. Aynı kaynağın başka stilli iki görünümü iki ayrı veri kopyası oluşturmasın; kopyalama ayrı ürün eylemi olsun. QGIS 4.2'nin kaynak ve sağlayıcı yaklaşımı kıyastır. [Q01], [Q04]

  **Bağımlılık ve tamamlanma ölçütü:** CBS-02/03/36. Bir belediye çalışanı dosyanın katmanlarını veriyi içe aktarmadan görebilsin, doğru katmanı ekleyebilsin ve kapatıp açınca bağlantıyı korusun. Taşınan kaynak için yeniden konumlandırma sunulsun; erişilemeyen kaynak boş katman diye kaydedilmesin. İlk dilimde yerel kaynaklar yeterli; STAC ve kurumsal katalog CBS-10/31'de genişler.

- [ ] **CBS-02 — Dış dataset'ler için ortak provider sözleşmesini uygula — P0**

  *Kısmen (18 Ekim 2026; [ADR 0510](../docs/adr/0510-bound-layer-of-a-part.md)): GeoPackage tablosunun bir alanı çalışma kopyası olur (`cad.sources.edit`'in `extent`'i, `.kcad`'de bağla): yalnız kutusu alana değen satırlar okunur (mekânsal dizinle; dizinsiz tabloda geometrinin kutusuyla, eskiden sessizce bütün tablo dönüyordu), eşitleme alanın dışını silmez, dışarı taşınan nesne kendi satırına yazılır. 1 milyon parselde 400 × 300 m'lik alan 372 ms ve 24 MB. Kalan: alanı değiştirmek, görünüme göre kendiliğinden okuma, PostGIS'te alan, sürüm damgasının dosyanın tamamından ucuzlaması.*

  *Kısmen (17 Ekim 2026; [ADR 0503](../docs/adr/0503-bound-layers.md)): GeoPackage sağlayıcısının ilk dilimi: tablo keşfi ve ayrı yetenekler (okuma, ekleme, güncelleme, silme, tek işlem, Z, M, eğri; `cad.sources.describe`), kalıcı kimlik, dosyanın sürümü (SHA-256) ve katmanın özeti, desteklenmeyen yazmanın gerekçeli reddi. Kalan: genel `Source/Dataset` arayüzü, kaynağa itilen bbox ve sütun sorgusu, sayfa ve akış, sorgu planının açıklaması (PostGIS ile).*

  **Mevcut:** Native `Document` ve yönetilen sunucu projesi var; harici veri için genel provider sözleşmesi yok [K02–K04]. Bu, GIS'in diğer birçok açığının ortak köküdür.

  **Yapılacak kalan iş:** `Source/Dataset/LayerView`, kararlı feature/field kimliği, source revision, kaynak CRS, schema discovery, bbox/alan/kolon sorgusu, sayfa/stream, identify, aggregate, cancellation ve hata durumları tanımla. Sağlayıcı; okuma, ekleme, silme, geometri/alan güncelleme, transaction, Z/M/eğri ve çatışma yeteneklerini ayrı bildirsin. `Document` içindeki yerel nesneler ilk adapter olsun; tüm mevcut CAD/GIS'i tek seferde yeniden yazma. Query planı hangi kısmın kaynağa itildiğini ve hangisinin yerelde hesaplandığını açıklasın. Görünür veri parçası tam dataset sayılmasın. [Q04]

  **Bağımlılık ve tamamlanma ölçütü:** `PG-01/02/05/06/12/17` ile tek tasarım. Aynı kaynak iki pencerede açıldığında aynı kayıt kimliğiyle bulunmalı; filtre/sayfa değişmesi ilişkileri bozmamalı. Kısmi yüklenmiş haritada “tüm kayıtları analiz et” işlemi doğru kapsamı sağlayıcıdan istemeli. Desteklenmeyen yazma açık gerekçeyle reddedilmeli; görünürde başarı veya gizli içe aktarma olmamalı.

- [ ] **CBS-03 — GeoPackage feature ve tablolarını doğrudan oku/yaz — P0; uzantılar P1**

  *Kısmen (17 Ekim 2026; [ADR 0502](../docs/adr/0502-geopackage.md)): GDAL'sız kendi crate'i `kentos-geopackage` (keşif, tipli sütunlar, geometri Z/M ve eğrilerle, R-tree, tek işlemde toplu yazma, `kentos_uids` ile fid ↔ UUID); `file.import.gpkg`/`file.export.gpkg` ve masaüstü penceresi; QGIS 4.2.3'ün gerçek dosyası okunur, KentOS'un yazdığı QGIS'te açılır, GDAL'ın doğrulayıcısından geçer. Dosyada yerinde düzenleme ve bilinmeyen uzantıları koruyarak geri yazma ADR 0503'le geldi. Raster karo profili (20 Ekim 2026; [ADR 0556](../docs/adr/0556-geopackage-tiles.md)): `tiles` modülü, `cad.rasters.gpkgread` (karo tablosunun bir düzeyi GeoTIFF olarak, GDAL'ın pikselleriyle) ve `cad.rasters.gpkgwrite` (rasterin PNG karo piramidi; GDAL'ın doğrulayıcısından geçer, QGIS 4.2'de açılır), masaüstünde iki pencere. Kalan: canlı karo kaynağı, JPEG ve WebP karolar, yükseklik karoları, ilişki profili.*

  **Mevcut:** GeoJSON/Shapefile temeli var; kaynakta GeoPackage provider/codec bulunamadı [K03]. Kullanılabilir bir açık GIS için en büyük günlük veri alışverişi boşluklarından biridir.

  **Yapılacak kalan iş:** `.gpkg` keşfetme, feature ve geometrisiz tablo okuma/oluşturma/düzenleme, CRS ve tipli kolonlar, spatial index, tek işlemde toplu yazma ve güvenli dosya kapatma uygula. Desteklenen geometry type, empty/null geometry, Z/M ve extension'ları açıkça bildir. Kaynak `fid` ile KentOS kalıcı UUID eşlemesi korunmalı; aç/kapa veya VACUUM sonrası rastgele yeni kimlik üretilmemeli. Bilinmeyen extension silinmesin. İlişki/stil/form paketinin GeoPackage standardı mı KentOS uzantısı mı olduğu belgeli olsun. SQLite native adapter ile GDAL/OGR adapter seçeneklerini gerçek round-trip ve paketleme denemesiyle seç; kendi SQLite motorunu yazma. [S01], [F01]

  **Bağımlılık ve tamamlanma ölçütü:** CBS-02/06/36; ilk sürümde vektör+tablo, sonra raster tile/ilişkili tablo profilleri. QGIS **4.2'de üretilen gerçek GeoPackage** açılıp düzenlenmeli; QGIS'te yeniden açıldığında kimlik, Türkçe alan/değer, null, delik, multipart ve desteklenen Z/M kaybolmamalı. Yazma ortasında iptal/çökme dosyayı yarım şemayla bırakmamalı; ağ paylaşımı çok yazarlı sunucu gibi sunulmamalı.

- [ ] **CBS-04 — Kurumun mevcut PostGIS tablo/view'larına doğrudan bağlan — P1**

  *Kısmen (17 Ekim 2026; [ADR 0505](../docs/adr/0505-postgis-bound-layers.md)): PostGIS tablosu bağlı katman olarak açılır (`cad.sources.*`'ın adresle çalışması), TLS'li sqlx sağlayıcısı `kentos-postgis` ev sahiplerince kaydedilir; sürüm satırların SHA-256'sı (`xmin` değil), yazma kilit altında sürüm denetimiyle tek işlemde, kalıcı kimlikler `kentos_uids`'te; parola projeye yazılmaz; masaüstünde PostGIS'e bağlan. Geçici PostGIS'te ogr2ogr ve psycopg2'yle denetlendi. Kalan: sorguyu kaynağa itmek ve sayfa sayfa ilk harita, view ve SQL katmanı, tetikleyici/outbox izleme, milyon satır ölçümü, QGIS'le eşzamanlı düzenleme senaryosu.*

  **Mevcut:** KentOS bulutunun kendi şeması ve sayfalaması mevcut; kurumun var olan `parsel`, `bina`, `hat` tablolarına provider olarak bağlanmak ayrı iş [K04].

  **Yapılacak kalan iş:** TLS/sertifika doğrulamalı bağlantı, şema/tablo/view keşfi, geometri kolonu/SRID/birincil anahtar seçimi, geometrisiz tablolar ve başlangıçta salt okunur query layer sun. Dış şemayı kendiliğinden `kentos.*` yapısına dönüştürme. Bbox/kolon/filtre/aggregate sorgusunu kaynağa it; uygun GiST/B-tree/KNN kullanımını sorgu planıyla ölç. Yazılabilir kaynakta PK ve revision stratejisi, transaction, rollback ve optimistic conflict zorunlu olsun. Kalıcı revision yerine tek başına `xmin` kullanılmasın; güvenilir sürümlemeye sahip olmayan kaynak sınırlı/salt okunur açılsın. İzinli trigger/outbox veya polling seçenekleri; `LISTEN/NOTIFY` dayanıklı geçmişin yerine geçmesin. [D01], [D02]

  **Bağımlılık ve tamamlanma ölçütü:** `PG-01…18` burada birleşir; CBS-02/06/37/38. Milyon kayıtlı dış tabloda ilk harita tüm satırları indirmeden gelsin. QGIS 4.2 aynı kaydı değiştirince KentOS bunu algılasın; iki editör birbirini sessizce ezmesin. DB bağlantı hatasında yerel taslak korunsun; eksik sayfa doğru toplam diye gösterilmesin.

- [ ] **CBS-05 — Eksik açık biçim aktarımlarını ve kayıp raporlu ETL'yi tamamla — P0/P1**

  *Kısmen (17 Ekim 2026; ADR 0502): GeoPackage iki yönde, kayıplar türlerine göre (`import_losses`, `export_losses`), başka sistemdeki dosya açık onayla çevrilir. 19 Ekim (ADR 0514): Shapefile dışa aktarımı kod sayfası, alan adı, tür ve boyut kayıplarıyla, ESRI `.prj`'siyle; QGIS 4.2 ve OGR'la denetlenir. 19 Ekim ([ADR 0522](../docs/adr/0522-csv-tables.md)): CSV iki yönde, GDAL'ın CSV'si (geometri ISO WKT ya da X ve Y, yaylar kesin, `.csvt` türleri, `.prj`); okuma GeoPackage içe aktarmasının yolundan, GDAL ve OGR'la denetlenir. 20 Ekim ([ADR 0544](../docs/adr/0544-flatgeobuf.md)): FlatGeobuf iki yönde, kendi kodeğimizle (paketli Hilbert R-ağacı, yaylar kesin, Z ve M, NULL ile boş metin ayrı); GDAL'ın dosyaları okunur, KentOS'unkiler OGR'da kaynağın geometrileriyle aynı, dizini OGR'ın süzgeciyle denetlenir. 20 Ekim ([ADR 0545](../docs/adr/0545-gml-and-xyz.md)): GML iki yönde (GML 3.2 yazma: her katman bir nesne türü, `.xsd` şeması, yaylar kesin, srsName ve EPSG eksen sırası; GML 2, 3.1 ve 3.2 okuma, türler şemadan, `.gfs`'den ya da değerlerden) ve XYZ yazımı (noktalar ya da bütün köşeler); GDAL'ın dosyaları okunur ve OGR'ın okuduğuyla denetlenir, KentOS'unkiler OGR'da kaynağın geometrileriyle aynı, XYZ satırları bağımsız köşe listesiyle denetlenir. Kalan: GeoParquet (kitaplığı sahibin onayını bekler), staging ve upsert.*

  *Kısmen (21 Ekim 2026; [ADR 0563](../docs/adr/0563-gis-file-import-commands.md)): Shapefile, GeoJSON ve KML/KMZ içe aktarma ürün komutu (`file.import.shp`, `.geojson`, `.kml`): dosyanın sistemi girdinin sorusu (`srid`, `reproject`) ve cevabın çevirme yolu ve doğruluğu (`moved`), katman yönlendirmesi, alınamayanlar raporda, tek geri alma adımı ve günlükte komutun adı; masaüstü pencereleri komutla yazar; sunucu işi üç biçimi alır; ortak durumlar GDAL/OGR, PROJ ve bağımsız okuyucularla.*

  **Mevcut:** Ortak okuyucular, GeoJSON, Shapefile okuma, KML/GPX, DXF ve tablo dosyaları var [K03/K11]. “Desteklenen uzantı” listesi yerine okuma/yazma ve semantik kayıp matrisi gerekir.

  **Yapılacak kalan iş:** İlk olarak GeoPackage, GeoJSON ve CSV/XYZ çift yönlü; Shapefile dışa aktarımı kodlama, alan adı/tür/boyut kayıplarıyla tamamla. Sonra GML, FlatGeobuf ve GeoParquet'i ihtiyaç sırasıyla ekle; büyük dağıtım/sorgu biçimi ile işlem yapılabilen kaynak ayrımını koru. OpenFileGDB, MapInfo gibi uyumluluklar açık kaynak adapter ve belgeli alt kapsamla genişler. İçe alma; kaynak CRS/birim, alan türü/kimlik/domain eşleme, önizleme, reddedilen satır dosyası, staging ve atomik commit içersin. Artırımlı yükleme/upsert için doğal anahtar ve yinelenen kayıt politikası açık olsun. [F03], [F04], [F05]

  **Bağımlılık ve tamamlanma ölçütü:** `FMT-03/04/06/08/09`, `PG-13/14/16`; CBS-02/06/07/15. Dönüştürücü kaç nesne okuduğunu, yazdığını, atladığını ve nedenini bildirsin; onarım/sadeleştirme ayrıca seçilsin. Büyük decimal/kimlik, tarihi, eğriyi veya Z/M'yi sessizce kaybetmesin. İlk GIS sürümünü bütün tarihsel biçimleri bitirmeye bağlama; kaynak/hedef kabiliyeti yeterli değilse açıklanabilir ret ver.

## B — Veri anlamı, koordinat doğruluğu ve sorgu

- [ ] **CBS-06 — Provider'da null/boş ayrımı, geniş tipler ve kararlı field eşlemesini tamamla — P0/P1**

  *Kısmen (17 Ekim 2026; ADR 0502): GeoPackage yolunda NULL özniteliğin yokluğu, boş metin boş metin; 2⁵³'ü aşan tam sayı her basamağıyla metin alanına, söylenerek; UTC zamanın dilimi söylenir. Kalan: int64/decimal/UUID/zaman dilimli anın kendi türleri ve dosya↔Python yolu.*

  *Kısmen (18 Ekim 2026; [ADR 0512](../docs/adr/0512-wide-values-through-a-bound-layer.md)): bağlı katmanın eşitlemesi hücreleri sütunun okuma kuralıyla karşılaştırır: 2⁵³'ü aşan tam sayının değişikliği yazılır, çizimin tutmadığı saat dilimi dokunulmamış satırı yeniden yazdırmaz. `9007199254740993`, `-9223372036854775807`, `0`, `false`, `""`, NULL ve +03:00 dilimli zaman dosya↔sağlayıcı↔tablo↔Python yolunda aynı; OGR'ın yazdığı dosya ve geri yazılan kopyası sqlite3'le hücre hücre denetlenir. Kalan: öznitelik modelinin kendi türleri, PostGIS'te aynı denetim, alan adı değişiminin stil ve birleştirmeye kadar izlenmesi.*

  **Mevcut:** `attrs: BTreeMap<String,String>` korunuyor. PR #25 stable field binding, tarih-saat/saat, domain, alt tür ve göç ekliyor; `MAX_INTEGER` hâlâ `2^53−1`, boş değer alanın yokluğu [K02/P25-A]. Bunları “tam tipli GIS veri modeli bitti” diye kabul etme.

  **Yapılacak kalan iş:** Form planının kimlik ve value adapter'ını kullanarak absent, SQL NULL, boş metin, boş liste ve sıfır ayrımını taşı. `int64`, exact decimal, UUID, timezone'lu instant, yerel datetime/date/time, JSON/array ve binary reference için kayıpsız provider codec'leri belirle. Büyük sayı metin temsilinden kayan noktaya zorlanmasın. Alan adı değişimi form, stil, ifade, join ve indeksi kararlı field ID üzerinden izlesin. Şema göçü dry-run, uyumsuz satır raporu ve geri dönüş yolu sunsun; PR'nin `cad.fields.migrate` işini yeniden yazma.

  **Bağımlılık ve tamamlanma ölçütü:** CBS-02/03/04/15; `FORM-DATA`/`PG-08`. İlk G1'de desteklenen tipler ve açık ret yeterli; bütün eski çizimlere zorunlu büyük göç yapılmaz. `9007199254740993`, uzun decimal, `NULL`, `""`, `0`, `false` ve saat dilimi geçişi dosya↔provider↔tablo↔Python yolunda aynı anlamı korumalı. Değer kaybolacaksa commit öncesi raporlanmalı.

- [ ] **CBS-07 — Katman CRS'sini koruyan genel on-the-fly provider dönüşümü ekle — P0**

  *Kısmen (17 Ekim 2026; [ADR 0504](../docs/adr/0504-source-crs-and-box-transform.md)): bağlı katmanda kaynak sistemi korunur (`reproject`: katman projenin, tablo kendi sisteminde; yazma tablonun sistemine geri çevirerek, gidip gelen koordinat aynı sayılır), sıklaştırılmış ve antimeridyeni bilen kutu dönüşümü `crs::transform_bounds` (PROJ'un `transform_bounds`'uyla). Kalan: her katmanın kendi sistemiyle çizimde anında dönüşüm, ters dönüşümlü tıklama ve kenet, WKT2 ve kullanım alanı, genel PROJ bağdaştırıcısı.*

  **Mevcut:** Türkiye odaklı dönüşüm/ikinci CRS ve servis dönüşümleri var. Genel feature içe alma/servis verisi çoğunlukla proje sistemine kopyalanıyor [K05/K12]. Bir harita CRS'si ile her veri kaynağının CRS'si ayrı olmalı.

  **Yapılacak kalan iş:** Kaynak CRS, harita gösterim CRS'si ve işlem/ölçüm CRS'sini ayır. On-the-fly çizim, ters dönüşümlü identify/snap, kaynak uzayında yazma ve dönüştürülmüş bbox sorgusu birlikte tasarlansın. WKT2/authority kimliği, axis order, birim ve area-of-use saklansın. Bbox dönüşümünde yalnız iki köşe yerine gereken sıklaştırma; antimeridyen ve kutup durumları ele alınsın. Desteklenen yerel Rust CRS kapsamı ile genel PROJ adapter kapsamı açık olsun. CRS atama, görüntüleme dönüşümü ve kalıcı reprojection ayrı komutlardır. [Q05], [S02]

  **Bağımlılık ve tamamlanma ölçütü:** CBS-02/06/08/16. TUREF/TM, WGS84 ve Web Mercator kaynakları aynı projede hizalansın; yalnız harita CRS'sini değiştirmek kaynak koordinatlarını değiştirmesin. Aynı noktayı çizimden seçme ve koordinatla sorgu aynı feature'ı bulsun. Bilinmeyen CRS tahmin edilmesin; EPSG:4326/WMS 1.3 eksen sırası ve Türkçe Y/X gösterimi fixture'larla ayrı doğrulansın.

- [ ] **CBS-08 — Düşey CRS/epoch ve işlem doğruluğu kaydını genişlet — P0 metadata; P1/P2 hesap**

  *Kısmen (19 Ekim 2026; [ADR 0519](../docs/adr/0519-vertical-crs-and-epoch.md)): projenin kotlarının ne olduğu (elipsoit/ortometrik, düşey sistem, geoit adı ve SHA-256) ve koordinat epoku `.kcad` şema 86'te; `cad.crs.transform` kotlarıyla dönüştürür ve işlemin kaydını verir (EPSG kimlikleri, adımlar, PROJ pipeline'ı, motor, ızgara SHA-256'sı, kullanım alanı, belirtilen doğruluk); rapor belirsizlikleri ayrı tutar; geoit, düşey datum ve epok dönüşümü açıkça reddedilir; PROJ'la 28 bağımsız durum. Kalan: geoit ve hız modeli motoru, dış biçimlerin düşey sistemi, sunucu–masaüstü fark sınaması.*

  **Mevcut:** Helmert/NTv2, elipsoidal ölçü ve GNSS yükseklik farkındalığı var; genel CRS dönüşümü 2D olarak belgeli [K05]. Z'nin bulunması düşey dönüşüm yapıldığı anlamına gelmez.

  **Yapılacak kalan iş:** Yatay/düşey/compound CRS, ellipsoidal–ortometrik yükseklik, geoid grid'i, koordinat epoch'u, gözlem zamanı ve birim metadata'sını taşı. Dönüşüm seçimi; operation kimliği/pipeline, engine ve grid sürümü/hash'i, kullanım alanı ve belirtilen doğrulukla kaydedilsin. Eksik grid veya kapsam dışı noktada düşük doğruluklu fallback'e sessiz geçme. Kaynak ölçme belirsizliği, dönüşüm doğruluğu, topoloji toleransı ve gösterim basamakları ayrı olsun. Geometrik/kayıtlı alan ile düzlem/elipsoid/yer/3D uzunluk seçenekleri hesap raporunda açık görünsün. [S02], [Q05]

  **Bağımlılık ve tamamlanma ölçütü:** CBS-07/16/29/34; yol ve 3D planlarının ortak koordinat politikası. G1'de desteklenmeyen düşey/epoch dönüşümüne açık ret ve korunmuş metadata yeterli; olmayan motorla “santimetre doğruluğu” sözü verilmez. Bağımsız PROJ/GeographicLib ve bilinen kontrol noktalarıyla yatay/düşey hata bütçesi doğrulansın; aynı girdinin sunucu ve native sonuç farkı belgeli toleransı aşmasın.

- [ ] **CBS-09 — Dış kaynaklar arasında canlı join ve salt okunur query layer ekle — P1**

  *Kısmen (19 Ekim 2026; [ADR 0527](../docs/adr/0527-join-views.md)): G1'in yerel tipli birleşimi: projenin birleşik görünümleri (`.kcad` şema 89), satırları her okuyuşta çizimden; çoğalan satır iki kaydın kalıcı kimliğiyle, eşleşme raporu ve kardinalite, toplamlar her kaydı bir kez sayar, hücre sahibi olan kayda yazılır, anahtar ve kilitli katman nedeniyle salt okunur, kopuk görünüm satır vermez; `cad.joins.edit` ve `.query`, Öznitelik tablosunda görünüm, Birleşik görünümler penceresi, Python ve MCP. Kalan: dış kaynaklar (bağlı GeoPackage ve PostGIS tablosu) arasında sağlayıcı sorgusunda birleşim, PG görünümü ve izinli salt okunur SQL sonucunun katman olması (G2), görünümün çizimde katman olması.*

  **Mevcut:** Anahtarla veri aktarma, ifadeler, PR'de kalıcı ilişkiler/lookup var. Bunlar bütün dış kaynaklar için canlı joined view veya SQL query layer değildir [K06/P25-B].

  **Yapılacak kalan iş:** Yerel anahtar aktarımı, PR ilişkileri ve lookup'ı yeniden yazma. Eksik olan canlı joined view/query layer için provider'lar arasında kaynak bağı ve yazma yönlendirmesi tanımla. Anahtar türü, null/yinelenen anahtar, cardinality ve çoğalan satır kimliği sağlayıcı sorgusunda korunsun; PG view/izinli salt okunur SQL sonucu yeniden açılabilen katman/tablo olsun. Geçici `row_number()` sonucu kalıcı kimlik sayılmasın. Mevcut katman/field/ilişki kimliklerini adapter ile kullan; raw SQL metni birleştirme. [Q04], [D01]

  **Bağımlılık ve tamamlanma ölçütü:** CBS-02/06/15/38; `FORM-REL/SRC`. G1'de yerel tipli join/ilişki yeterli; dış PG query layer G2'dir. Aynı parselin üç malik kaydı bulunduğunda çoğalan satırlar kayıp/yanlış toplam üretmesin; yazma doğru ana kayda gitsin veya salt okunur gerekçesi gösterilsin. Join kaynağı kopunca son kopya canlı ve güncelmiş gibi sunulmasın.

- [ ] **CBS-10 — Dataset metadata'sını ve analiz soy ağacını kalıcılaştır — P0 asgari; P1 katalog**

  *Kısmen (18 Ekim 2026; [ADR 0511](../docs/adr/0511-result-lineage.md)): analiz damgası: `cad.processing.run`'ın girdileri içeriklerinin özetiyle (`digest`) ve bağlı katmanın kaynak sürümüyle; dosyaya yazılan sonucun her tablosu soyunu GeoPackage'ın metadata uzantısında taşır (araç, istek, girdiler ve özetleri, çizimin sürümü, motor, zaman; `https://kentos.app/lineage/1`), sqlite3 ve GDAL'ın doğrulayıcısıyla denetlenir. Kalan: çizimin katmanına yazılan sonucun soyu, bağımlılık grafı ve eski türev göstergesi, veri kümesinin tanımı (başlık, sorumlu, lisans, kalite), katalog adaptörleri.*

  **Mevcut:** Proje bilgileri, layer info kartı ve işlem geçmişi var; bunlar veri üretim yöntemi/kalite/provenance standardının bütünü değil [K06/K09].

  **Yapılacak kalan iş:** Dataset başlığı, açıklama, sorumlu, lisans/kullanım kısıtı, kapsam, tarih/güncellik, CRS/datum, ölçek/çözünürlük, veri sözlüğü ve kalite özetini kaynak kimliğiyle sakla. Her türev; girdi revision/hash'leri, filtre/kapsam, algoritma/sürüm, parametre/birim, engine ve üretim zamanını taşısın. “Bu harita hangi veriden üretildi?” sorusuna bağımlılık grafıyla cevap ver. Yerel arama sonrasında kurum kataloğu, OGC API Records ve görüntü varlıklarında STAC adapter'ı eklenebilir; taslak standardı yayımlanmış sürüm diye kullanma. [S03], [F06]

  **Bağımlılık ve tamamlanma ölçütü:** CBS-02/24/31/36. İlk sürümde kaynak ve analiz damgası yeterli; tam portal zorunlu değil. Kaynak değiştiğinde türevin eski kaldığı anlaşılsın; yetkisiz kaynağın varlığı arama, otomatik tamamlama veya provenance'da sızmasın. Taşınabilir proje başka cihazda açıldığında kaynağı/üretim yöntemi anlaşılabilsin; salt dosya adı soy ağacı kabul edilmesin.

## C — Güvenilir veri üretimi, topoloji ve CAD/GIS ortaklığı

- [ ] **CBS-11 — Mevcut çizim araçlarına provider düzenleme/commit oturumu bağla — P0**

  *Kısmen (17 Ekim 2026; [ADR 0503](../docs/adr/0503-bound-layers.md)): bütün çizim araçları bağlı katmanda çalışır; yerel geri alma, kaynağa yazma (`cad.sources.sync` write, tek SQLite işlemi) ve kaynaktan yeniden okuma ayrı durumlar; başkasının değiştirdiği dosya ezilmez (`source_changed`), katmandaki yazılmamış iş sorulmadan atılmaz (`pending_changes`). Kalan: aynı kaynaktaki katmanların birlikte commit'i, nesne nesne üç yollu birleştirme (CBS-37), PostGIS işlemi.*

  **Mevcut:** CAD araçları, transaction/undo ve PR'nin yeni nesnede form açması güçlü bir temel. Harici kaynağa yazma, yerel undo ile veritabanında kalıcı commit'in farkını görünür kılmalı [K07/P25-A].

  **Yapılacak kalan iş:** Mevcut nokta/çizgi/alan, multipart/delik, reshape/split/merge, trace ve köşe düzenleme araçlarını yeni provider oturumuna bağla; aynı araçları yeniden geliştirme. Provider'ın hedef dataset, yazılabilirlik ve feature template bağlamını mevcut form/domain/alt türden kur. Taslak, yerel undo, kaynağa commit ve kaynaktan yeniden yükleme birbirinden ayrı durumlar olsun. Aynı transaction kaynağındaki katmanların değişikliklerini birlikte commit et; bağımsız veritabanları için atomiklik vaat etme. Z/M/eğri aktarımında destek dışı durumları yazmadan önce reddet.

  **Bağımlılık ve tamamlanma ölçütü:** CBS-02/03/12/15/37. Parsel bölünürken yeni kimlikler, eski–yeni bağlantısı, alanların bölme politikası, ilişkiler ve form hesabı aynı işlemde sonuçlansın. Kilitli/yetkisiz komşu veya constraint hatası yarım veri bırakmasın. Tek undo yerel adımı eksiksiz geri alsın; sunucuda onaylanmış değişikliği geri alma yeni sürüm denetimli işlem olarak ele alınsın.

- [ ] **CBS-12 — Dış sağlayıcıda görünüm dışı komşularla topolojik commit'i tamamla — P1**

  *Kısmen (19 Ekim 2026; [ADR 0531](../docs/adr/0531-neighbours-outside-the-area.md)): dosya kaynağında (GeoPackage) alan sınırlı parçanın yazması; ortak köşeleri değişen, haritaya yüklenmemiş komşular bbox sorgusuyla aynı dosya sürümünden okunur, mevcut `topology_edit` motoruyla izler, kaynağın sisteminde aynı SQLite işleminde yazılır; `strict` profilde geçersiz kalacak komşu hiçbir şeyin yazılmamasıdır, `loose`'da söylenerek bırakılır; dosya değiştiyse çakışma. Kalan: dış PostGIS kaynağı (sağlayıcının alan sınırı, CBS-02/04), yetki ve başka kullanıcının sürüm engeli, komşunun tam geçerlilik denetimi.*

  **Mevcut:** Ortak köşe/kenar düzenleme **zaten yapılmış**; MEVCUT-01 tamamlanmış kapsamdır. Native `topology_follow::around` ve `interaction::neighbours` görünen geometri deposundan komşuları bulur; köşe/kenar değişimleri aynı adımda yazılır. Kilitli komşu atlanıp bildirilir; bu mevcut davranış bir eksiklik diye yeniden açılmıyor. Açık iş, henüz olmayan dış GIS sağlayıcısında aynı işlemin kaynak sorgusu/transaction ile yürütülmesidir. [K07]

  **Yapılacak kalan iş:** CBS-02/04'ün dış sağlayıcısı için komşu adaylarını haritada yüklü veriden bağımsız bbox/kaynak indeksi sorgusuyla getir; source CRS/birim ve feature revision'larını aynı işlem bağlamında taşı. Mevcut `topology_edit` motorunun ürettiği değişiklikleri yeni geometri algoritması yazmadan provider transaction'ına uygula. Yüklenmemiş, yetkisiz veya başka kullanıcı tarafından değişmiş komşunun sonucu eksiksiz güncellemeye izin vermediğini commit öncesinde bildir. Kaynak bütünlüğü için istenen sıkı işlem profilinde tamamı ya da hiçbiri kuralı uygula; yerel çizimin mevcut kilitli-komşu davranışını sessizce değiştirme.

  **Bağımlılık ve tamamlanma ölçütü:** CBS-02/04/07/11/13/14/16. Dış PostGIS kaynağında iki komşudan yalnız biri haritaya yüklenmişken ortak sınırı düzenle; ikisi de aynı transaction'da ve beklenen revision'larla güncellenmeli. Komşuda yetki/sürüm engeli varsa sıkı profil hiçbirini yazmamalı. Yerel topolojik düzenlemenin mevcut fixture'ları değişmeden korunmalı. Dış provider olmadan bu genişleme tamamlandı sayılmaz; yerel özellik ise zaten tamamdır.

- [x] **CBS-13 — GEO-01 birleşim hatasını gider ve zincirli overlay doğruluğunu doğrula — P0**

  *Yapıldı (17 Ekim 2026; [ADR 0500](../docs/adr/0500-overlay-correctness-corpus.md)): GEO-01 testi açık ve geçiyor; `fixtures/overlay/v1` 30 durum (GEOS ve 50 basamaklı kapalı formüller), geçerlilik, idempotentlik, özdeşlik ve zincir denetimiyle; çekirdeğin 10⁻⁶ m toleransı durumda açık. GEO-02 ayrı.*

  **Mevcut:** Robust predicate'ler ve analitik CAD kenarları var. Ancak GEO-01'in yok sayılan testi kaynakta duruyor; sabit toleranslarla farklı işlemlerin aynı sınırı farklı yorumlama riski incelenmeli [K08]. Bu konu yeni analiz eklemekten önce gelir.

  **Yapılacak kalan iş:** Zincirlenen union, tamponların teğet teması, kendine değen halka, çok küçük sliver, yakın paralellik, büyük koordinat ve çok parçalı/delikli eğriler için bağımsız korpus kur. Alan korunumu, geçerlilik, idempotence ve beklenen birleşim/kesişim özdeşliklerini hata bütçeleriyle denetle. Kaynak eğri hesabı, yaklaşık linearization ve render tessellation farklı katmanlar olsun. Hata kapatılırken toleransı büyüterek veya sonucu sessiz onararak doğruluk gizlenmesin. Gerekirse native GEOS adapter'ı karşılaştırma/alternatif backend olarak prototiplensin; GEOS'un düzlemsel ve sonlu hassasiyetli olması hesaba katılsın. [S04]

  **Bağımlılık ve tamamlanma ölçütü:** `GEO-01/02`, sayısal politika ve CBS-16/22/23. GEO-01 testi etkinleşip geçmeli; bağımsız analitik/yüksek hassasiyetli ve PostGIS/GEOS karşılaştırmaları aynı açıklanmış semantiği sağlamalı. Mevcut fixture güncellemesi tek başına kanıt değildir. Hata kapanmadan ilgili analiz kapsamı üretim için doğrulandı diye işaretlenmemeli; GEO-02 ayrı performans ölçütüyle izlenmeli.

- [ ] **CBS-14 — Topoloji bulgularını artımlı ve kaynak revision'ına bağlı izle — P1**

  *Kısmen (19 Ekim 2026; [ADR 0524](../docs/adr/0524-incremental-topology-findings.md)): bulgunun kalıcı anahtarı, denetimin çizimin durumuna bağlanması, yalnız değişenin, komşularının ve boşluk kümesinin yeniden denetlenmesi (rastgele düzenlemelerde tam denetimle aynı), Değişenleri denetle. Kalan: dış sağlayıcıda revizyon ve görünüm dışı komşular, istisnanın geometri bağlamı, düzeltmenin etki önizlemesi.*

  **Mevcut:** 13 topoloji kuralı, bulgu paneli, düzeltme ve istisna var [K07]. ArcGIS'in Error Inspector yaklaşımındaki yararlı taraf, hatayı haritada bulup gerekçeli olarak çözme akışıdır. [A01]

  **Yapılacak kalan iş:** Mevcut 13 kuralı, bulgu panelini, düzeltme ve istisna araçlarını yeniden yapma. Bunların üzerine kaynak revision'ına bağlı kalıcı bulgu kimliği, değişen bölge/komşuluk takibi ve yalnız etkilenen veriyi yeniden denetleme ekle. Kural değişimi veya feature revision'ı eski sonucu geçersiz kılmalı. İstisna aynı kimlik/geometri bağlamı değişince yeniden değerlendirilmelidir; yeni hatayı gizlememeli. Dış provider'dan tam komşuluk alınmadan temiz QA sonucu verme. Yeni kalite kaydı mevcut panelde gösterilsin; düzeltmenin kaynak etkisi ve ölçü farkı önizlenebilsin.

  **Bağımlılık ve tamamlanma ölçütü:** CBS-11/12/13/15. G1'de mevcut denetle→bul→düzelt→yeniden denetle akışı bütün kaynaklarda çalışsın. G2'de QA revision'ı ile commit revision'ı uyuşmalı; eski doğrulama yeni kaydı onaylamamalı. Tüm veriyle yapılan denetim ile artımlı denetim aynı bulgu kümesini versin; kaynaklar arası kuralda eksik sayfa “hata yok” sonucu üretmesin.

- [ ] **CBS-15 — Mevcut form/ilişki standardını dış dataset transaction'ına bağla — P0/P1**

  *Kısmen (19 Ekim 2026; [ADR 0532](../docs/adr/0532-forms-and-sources-transaction.md)): bağlı GeoPackage'ta üst ve alt kayıtların tek SQLite işleminde yazılması (`cad.sources.sync`'in `with`'i, `cad.forms.submit`'in `commit`'i; bir alt kayıt reddedilirse hiçbiri yazılmaz), çoktan çoğa yerel başvurunun OGC 18-000 ilişki profiline açık eşlemesi (`cad.sources.relate`; bilinmeyen temsil metne çevrilmez), başka katmandan kayıtların ve ilişkili kayıtların sayfalı, süzgeçli toplamla ve kaynak sürümüyle okunması (`cad.records.page`); bina→parsel→malik örneği harita, tablo, form, Python ve MCP'de aynı. Kalan: PostGIS'te çok tablolu işlem ve bağlantı tablosu, formun lookup'ının `cad.records.page` üzerinden okuması, erişim politikası (CBS-38).*

  **Mevcut:** Designer, domain/subtype/contingent, lookup, kayıt kuralları, geometrisiz tablo ve ilişkiler PR'de mevcut [P25-A…C]. Yeniden form çekirdeği veya ikinci validation motoru yazılmayacak.

  **Yapılacak kalan iş:** Provider kaynaklarını formun stable field binding ve value adapter'ına bağla. N:M çok değerli yerel başvuru temsili ile PG junction tablosu/GeoPackage ilişki profili arasında açık eşleme kur; bilinmeyen temsili noktalı virgüllü metne gizlice çevirme. Geometri değişince hesaplanan alan, split/merge politikası, lookup ve ilişkili alt kayıtlar aynı iş kuralından geçsin. Yerel parent+children atomik gönderimini sunucu/provider transaction sınırına taşı. Başka dataset'ten seçenek ve ilişkili kayıtlar yetkili, sayfalı ve kaynak revision'ıyla okunmalı; toplam sayısı dahi erişim politikasından geçmeli. Formun mevcut `requires` profili, eylemleri, bölüm kitaplığı ve taslak kurtarması korunur; kaynak/host gereksinimleri aynı modele eklenir.

  **Bağımlılık ve tamamlanma ölçütü:** `FORM-DATA/REL/SRC/RULE/ACL`, CBS-02/06/09/11/38. Bina→parsel→malik örneği harita, tablo, form, Python ve MCP'de aynı sonucu vermeli. Bir alt kayıt reddedilirse üst kayıt kısmen yazılmamalı. PR'nin mevcut entry rolü/alan projection'ı korunmalı; kullanıcı/görev ataması, reviewer ve dış lookup'ın kalan kısımları bu varlığı yok saymadan tamamlanmalı. Birleştirmede şema/migration numarası güncel dallardan seçilmeli; PR açıklamasındaki eski numara kopyalanmamalı.

- [ ] **CBS-16 — CAD kaynağının dış GIS dataset'inde izlenebilir dönüşümünü tamamla — P0**

  *Kısmen (19 Ekim 2026; [ADR 0530](../docs/adr/0530-cad-to-gis-derivation.md)): CAD→GeoPackage izlenebilir türev (`file.export.gis`: kiriş algoritması ve sapma sınırı, satır başına kaynak kimliği, türü, sapması ve özeti, Z/M, ayrı yazı ve blok tabloları, kayıplar nedenleriyle, tablo başına türetim kaydı), türevin denetimi (`cad.gis.derivation`: eski, GIS'te değişmiş, kaynağı yok, türevde yok) ve türevin kaynağın yerine geçmemesi (`derived_dataset`, `derived_import`). Kalan: GIS değişikliğinin kaynağa geri alınması ya da açık ilişki koparma, PostGIS, Shapefile ve GeoJSON türevi, mimari nesne ve ölçü türevi.*

  **Mevcut:** `source_kind`, `cad_definition` ve türev `geom` ayrımı mevcut [K04]. KentOS'un rakiplerinden ayrışabileceği yer, hassas CAD üretimini GIS veri anlamıyla birlikte korumaktır.

  **Yapılacak kalan iş:** CAD katmanı→GIS dataset/feature class dönüşümünde entity/feature kimliği, katman–şema eşlemesi, blok özniteliği, kaynak ilişki ve Z/M açıkça taşınsın. Yazı, ölçü, tarama ve blok; kendiliğinden aynı anlamdaki GIS polygon/point sayılmasın. Analitik yay/NURBS kaynak olarak kalsın; gerektiğinde GIS izdüşümü algoritma/sürüm ve maksimum sapma toleransıyla üretilebilsin. GIS'te türevi düzenleme; kaynağa geçerli ters işlem, açık ilişki koparma veya ret politikası taşısın. Kaynak geometriyi değiştirip eski türevi güncel bırakma.

  **Bağımlılık ve tamamlanma ölçütü:** `PG-09/19`, önceki CAD veri/komut işleri; CBS-07/08/13/15. Yaylı parsel ve bloklu bina verisi `.kcad`↔PostGIS↔GIS dışa aktarımı sonrasında düzenlenebilir kaynağını korusun. Haritanın zoom/LOD değeri değişince nihai alan/uzunluk değişmesin. Kayıplı teslim ayrıca raporlansın; analitik kaynakla yaklaşık sınırın farkı kullanıcı ve otomasyona açık olsun.

## D — Kartografya, tablo ve çıktı üretimi

- [ ] **CBS-17 — Mevcut stilleri provider alan kimliklerine bağla ve stil alışverişini genişlet — P0/P1**

  *Kısmen (19 Ekim 2026; [ADR 0516](../docs/adr/0516-sld-style-exchange.md)): görünüş SLD 1.1 / SE 1.1 olarak iki yönde (tek simge, kategorili, aralıklı, kurallı; süzgeçler ve ölçek aralıkları); taşınamayan her parça adıyla söylenir; QGIS 4.2'nin SLD'leri alınır, KentOS'unkiler QGIS'te aynı kurallarla açılır. Kalan: alan kimliklerine bağ, MapLibre profili, etiketler, tematik işleyiciler.* *Kısmen (20 Ekim 2026; [ADR 0555](../docs/adr/0555-qml-style-exchange.md)): görünüş QGIS'in kendi stil dosyası QML olarak da iki yönde (tek simge, kategorili, aralıklı, iç içe kurallı; dolgu, çizgi, işaret ve merkez dolgusu; birimleri, saydamlığı ve kapalı katmanıyla); QGIS 4.2'nin QML'leri alınır, KentOS'unkiler QGIS'te `loadNamedStyle` ile aynı görünüşle açılır. Kalan: QGIS'in öbür simge katmanları, veriye bağlı değerler.* *Kısmen (20 Ekim 2026; [ADR 0560](../docs/adr/0560-maplibre-style-exchange.md)): görünüş MapLibre GL stili (sürüm 8) olarak da iki yönde (kategorili `match`, aralıklı `step`, süzgeçli kurallar, ölçek aralığı zoom; fill, line, circle ve etiket symbol katmanları; mm 96 dpi'de piksel, metre zoom'la büyüyen genişlik); her katmanın WGS 84 GeoJSON'u stilin yanına ya da vektör karo adresi; KentOS'un stilleri MapLibre'nin resmî doğrulayıcısından ve ifade değerlendiricisinden geçer. Kalan: sprite ve simgeler (daire dışı şekiller, desen), veriye bağlı değerler, etiketlerin içe alınması.* *Kısmen (21 Ekim 2026; [ADR 0564](../docs/adr/0564-look-follows-field-renames.md)): görünüş alanın adını izler: işleyicinin ifadeleri, kuralların süzgeçleri ve katmanda tanımlı simgelerin veriye bağlı değerleri alanın kullanıldığı yerlerdendir, alan yeniden adlandırılınca aynı adımda yeni adı alır. Kalan: bağlı kaynağın sütunu kaynakta yeniden adlandırılınca izleme, kitaplığın simgeleri.* *Kısmen (21 Ekim 2026; [ADR 0565](../docs/adr/0565-qml-hatch-marker-line-and-data-defined-values.md)): QML'de KentOS'un taraması QGIS'in çizgi deseni, işaretli çizgisi QGIS'in işaretli çizgisi; çizginin kalınlığı, işaretin boyu ve dönüşü veriye bağlı değer olarak iki yönde, çevrilemeyen ifade ve renkler adıyla söylenir. Kalan: degrade, nokta deseni, SVG ve yazı işareti, renklerin veriye bağlı değerleri.* *Kısmen (21 Ekim 2026; [ADR 0571](../docs/adr/0571-style-exchange-gradient-pattern-svg-font-and-colours.md)): degrade (QML), nokta deseni (QML, SLD), SVG işareti (QML, SLD; çizim projenin kitaplığına girer), yazı işareti (QML, SLD, MapLibre) ve renklerin veriye bağlı değerleri (QML, SLD, MapLibre) iki yönde; üç biçimin söyleyemedikleri adıyla söylenir (SLD ve MapLibre'de degrade, MapLibre'de desen ve SVG, QGIS'in SLD'de alana bağlı rengi okumaması). Kalan: işlevli ifadeler, QGIS'in öbür simge katmanları, etiketlerin içe alınması.*

  **Mevcut:** Native sınıflandırma, kurallı stiller, sembol tasarımcısı, `.kstil` ve ortak GPU batch motoru mevcut [K10]. Öncelik bunları dış GIS kaynağında aynı davranışla çalıştırmaktır.

  **Yapılacak kalan iş:** Mevcut renderer türlerini, renk rampalarını, veriyle belirlenen görünüşü, ölçek/birim ve legend hesabını yeni provider field/feature kimliklerine bağla. Alan rename ve kaynak sayfalama stil bağını bozmamalı; aynı veri için ikinci stil motoru kurulmasın. Kalan genişleme; dış stile ait desteklenen SLD/SE/MapLibre/QML alt kümesinin açık import/export profili ve çevrilemeyen kuralların raporudur. Hızlı stil yükle/kaydet erişimi mevcut komutların üzerinden sunulsun; mevcut `.kstil` yolu korunur. [Q02]

  **Bağımlılık ve tamamlanma ölçütü:** CBS-02/06/10/18/21. Aynı stil yerel ve PG katmanında aynı sınıfları/legend'ı üretmeli; alan rename olduğunda bozulmamalı. Tek kaydı güncellemek bütün katmanı yeniden inşa ettirmemeli. Harita görünüşü, PDF ve kaydedilip açılan proje aynı ölçek/birim yorumunu korumalı; desteklenmeyen dış stil sessizce varsayılana düşmemeli.

- [ ] **CBS-18 — Etiket pin'lerini dış kaynak kimliğiyle koru; sayfa sınırı yerleşimini doğrula — P1**

  *Kısmen (19 Ekim 2026; [ADR 0533](../docs/adr/0533-label-pins-bound-layers-and-stability.md)): bağlı katmanda iğneler bağın anahtarıyla (`SourceLink.key`) kararlı kayda bağlı kalır, taşınan ve kaybolan iğneler nedeniyle söylenir; motor önceki yerleşimi tercih eder (küçük kaydırmada sıçrama ölçüldü ve giderildi), önbellek kaynağın sürümünü ve çıktı ölçeğini sayar, ekran ile PDF aynı yerleşim; `cad.labels.report` çakışan, yerleşmeyen ve yinelenen etiketleri sayar; etiket ve engel sorgusunun yüklü parçayla sınırı söylenir. Kalan: sağlayıcı sayfalanırken parçanın dışındaki engeller, anahtarsız yeniden yazım, alanın dışındaki iğnenin parkı.*

  **Mevcut:** Kurallı etiket, engel, öncelik, eğri boyunca yazı, sabit pin ve yerleşmeyen etiket görünümü mevcut [K10]. PAL/Maplex benzeri bir motor daha kurmak yerine mevcut motor gerçek veriyle zorlanmalı.

  **Yapılacak kalan iş:** Mevcut etiket türü, line merge/repeat, callout, engel/öncelik ve pin araçlarını yeniden geliştirme. Provider sayfalanırken elle yerleştirme kayıtları kararlı feature ID'ye bağlı kalsın; etiket/engel sorgusu yalnız yüklü sayfayla sınırlanmasın. Sayfa/karolar arası yinelenme ve küçük kaydırmada toplu yerleşim sıçraması yeni provider senaryosunda ölçülüp giderilsin. Cache'e kaynak revision'ı ve hedef çıktı ölçeği dahil edilsin. Doğrulama, yeni provider'ın aynı etiket motoruna aynı anlamdaki girdileri vermesine odaklansın.

  **Bağımlılık ve tamamlanma ölçütü:** CBS-02/17/19/21/42. Yoğun parsel ve yol haritasında kabul örnekleri belirlenmeli: çakışan, yerleşmeyen ve tekrarlanan etiketler ölçülsün. Küçük kaydırmada gereksiz toplu sıçrama olmasın. Aynı pafta ölçeğinde ekran ile PDF'de kabul edilen yerleşim tutarlı olsun; pin'ler yeniden açmada veya kaynak sayfalanmasında başka kayda bağlanmasın.

- [ ] **CBS-19 — Bölünmüş görünümü bağlı tema ve swipe karşılaştırmasıyla genişlet — P1**

  *Kısmen (19 Ekim 2026; [ADR 0523](../docs/adr/0523-linked-views-themes-and-swipe.md)): her görünümün teması projenin bir katman durumu (görünürlük, dondurma, stiller; çizimin kopyasında, belge değişmez), etkin görünümün teması düzenleme hedefi (seçme, kenet), etiketler görünüm görünüm; Görünümleri bağla ve Kaydırmalı karşılaştır. Kalan: görünüm başına zaman ve ölçek bağı, iki revizyonun karşılaştırılması.*

  **Mevcut:** İki bağımsız kameralı bölünmüş görünüm, named view ve katman durumları var [K10]. GIS'te görünüm; yalnız kamera değil tema, zaman, ölçek ve filtre bağlamıdır.

  **Yapılacak kalan iş:** Mevcut named view, katman durumları ve iki bağımsız görünümü yeniden kurma. Bunların üstüne seçilen kamera/ölçek/CRS/zaman bağlamını birlikte takip eden bağlı görünümler ve swipe karşılaştırması ekle. Aynı veri kaynağının iki tema/revision görünümü yeni veri kopyası oluşturmamalı. Her görünümde hangi kaynağın ve revision'ın gösterildiği, hangisinin düzenleme hedefi olduğu belirli olsun. Mevcut senaryo görünürlüğü bu yeni karşılaştırmada gerçek veri sürümüyle karıştırılmasın.

  **Bağımlılık ve tamamlanma ölçütü:** CBS-02/17/18/33/36. Kullanıcı ortofoto–kadastro ve eski–yeni durumu aynı konumda karşılaştırabilmeli; bir görünümdeki seçim doğru kaydı diğerinde vurgulamalı. Tema geri çağırma feature'ları değiştirmemeli; görünüm kapatılınca GPU/işçi kaynakları serbest kalmalı. G1'de mevcut iki görünüm ve named view güvenilirliği yeterli; çoklu tema/swipe daha sonra eklenebilir.

- [ ] **CBS-20 — Öznitelik tablosunda provider paging/sıralama/toplu yazmayı tamamla — P0/P1**

  *Kısmen (18 Ekim 2026; [ADR 0507](../docs/adr/0507-attribute-table-bulk-writes.md)): toplu yazma komutu `cad.fields.write` (Alan hesapla: ifade, kapsam, boş sonuç; Yapıştır: satır başına değerler), değerler `cad.entities.set`'in kurallarıyla denetlenir (tür, kanonik metin, zorunlu, alt tür, bileşim, kayıt kuralı, tekil alan birlikte), bütün ya da hiç (`invalid_values`) ya da uyanlar (`values_skipped`), plan önizleme ve satır satır sorun raporu; masaüstünde tablonun çubuğunda Alan hesapla… (kapsam sayılarıyla) ve Yapıştır (başlık satırı, ilk seçili satırdan). Kalan: kaynağa itilen sıralama, süzgeç ve toplam, imleçli sayfalar, gizli sütunun kopya ve dışa aktarmada süzülmesi, grafiklerin aynı süzgeçle çalışması.*

  *Kısmen (20 Ekim 2026; [ADR 0546](../docs/adr/0546-attribute-table-outgoing-and-source-line.md)): Öznitelik tablosunda Sütunlar ▾ (gizlenen sütun tabloda, kopyada, dosyada ve Yapıştır'ın sütun sırasında yok), Kopyala (gösterilen satırlar, başlık alan adları, Yapıştır'la geri), Dışa aktar'ın gösterilen satır ve sütunları; formda `redact` işaretli alan kopyadan, dosyadan, İstatistik'ten ve Grafik'ten çıkar ve söylenir; İstatistik ve Grafik'in Göster ▾'inde Tablonun süzgeciyle (İncele ▾). Kalan: sıralama, süzgeç ve toplamın kaynağa itilmesi ve imleçli sayfalar (§4'te tasarlandı), sütun başlığı menüsü, alan yetkisinin ortak çözücüsü (CBS-38).*

  **Mevcut:** Arama, ifade, türüne göre sıralama, harita seçimi, hücre düzenleme ve spreadsheet export var [K06]. PR de geometrisiz kayıt, koşullu tablo görünüşü ve ilişkili tablo ekliyor [P25-B].

  **Yapılacak kalan iş:** Mevcut sanallaştırma, hücre düzenleme, filtre/arama, seçim ve tablo export'unu yeniden yazma. Sıralama/filtre/aggregate'i provider'a it; tabloyu source revision'ına bağlı kararlı cursor sayfalarıyla besle. Yüklü, seçili, filtreli ve tüm dataset kapsamlarını açık sayılarla ayır. Provider'a toplu yapıştırma/alan hesaplama için önizleme, hedef tip kontrolü ve atomik yazma adapter'ı ekle. Grafik/istatistik aynı kaynak filtresini kullansın; yetkisiz kolon daha sonra kopya/export'tan okunamasın. QGIS 4.2 kolon başlığından alan hesabı mevcut araca erişim örneğidir. [Q02]

  **Bağımlılık ve tamamlanma ölçütü:** CBS-02/06/09/15/38/42. Milyon satırlı tabloda sıralama değişince tamamı istemciye inmeyebilmeli; kararlı cursor ve snapshot sayfa tekrarı/atlamasını önlemeli. Gizlenen kolonun değerleri kopyalama/export/AI üzerinden sızmamalı. Bin hücrelik yapıştırmada tek yanlış değer belirlenen atomiklik politikasına göre tüm işlemi durdurmalı veya açık satır raporu üretmeli; sessiz kısmi başarı olmamalı.

- [ ] **CBS-21 — Mevcut atlas/GeoPDF'yi provider revision'ı ve export yetkisine bağla — P1**

  *Kısmen (19 Ekim 2026; [ADR 0534](../docs/adr/0534-atlas-pdf-pinned-revision.md)): `file.export.atlas`: sayfalar paftanın atlas kurallarıyla, her sayfa kendi nesnesinin haritası, tablosu ve lejantıyla tek PDF'te (GeoPDF); çıktı çizimin parçalarının damgalarına, katmanların özetine ve bağlı kaynağın SHA-256'sına sabitlenir (`pin_mismatch`, `source_changed`, `source_stale`); tema, zaman ve sistem açık ve `Keywords`'te kayıtlı; süzgeçten ve zamandan geçmeyen nesne ve `redact` alan çıktıda yok; yüz parsellik atlas poppler ve GDAL'la gerçek dosyada doğrulandı; masaüstünde Atlası PDF yap, Python ve MCP. Kalan: kapsam katmanını sağlayıcıdan sayfa sayfa okumak ve büyük katman ölçeği, haritaların yazıları, nesneye kırp, başka sistemde basma, kalıcı alan izni (CBS-20).*

  **Mevcut:** Ortak pafta/PDF/GeoPDF altyapısı ve `crates/shared/sheet/src/atlas.rs` içinde filtre/sıralama/adlandırma/ölçek seçen atlas planlayıcısı zaten var [K10]. Kalan kapsam yeni provider revision/tema bağları, izinli export ve açık kalan native orkestrasyondur; atlas hesapları sıfırdan yazılmayacak.

  **Yapılacak kalan iş:** Mevcut pafta nesnelerini, `kentos-sheet::atlas` planlayıcısını, legend/grid ve PDF/GeoPDF hesaplarını yeniden yazma. Bunlara provider dataset, tema/zaman/CRS ve sabit kaynak revision'ı bağla; açık kalan native atlas önizleme/toplu export orkestrasyonunu bu çekirdekten tamamla. Büyük coverage katmanı provider'dan kontrollü okunmalı; sayfa adlandırma/ölçek/sıralama mevcut atlas kurallarıyla gelmeli. GeoPDF katman ve öznitelik export'u güncel erişim projection'ından geçsin. [Q02], [A02]

  **Bağımlılık ve tamamlanma ölçütü:** CAD pafta planı ile tek çekirdek; CBS-10/17/18/36/38. Yüz parselin atlası doğru feature/kapsam/ölçekle üretilebilmeli; işlem sırasında kaynak değişirse revision sabitlenmeli veya çıktı yenilenmesi istenmeli. Font, çizgi, raster çözünürlüğü, ölçülü baskı ve GeoPDF koordinatı gerçek dosyada doğrulansın; çıktıda gizli öznitelik bulunmasın.

## E — Vektör analizleri ve tekrarlanabilir işlemler

- [x] **CBS-22 — Mevcut ilişkilerden ayrı sürümlü standart DE-9IM profili ekle — P0**

  *Yapıldı (17 Ekim 2026; [ADR 0501](../docs/adr/0501-standard-relations-de9im.md)): `ops::de9im` (kesin matris, yaylar kesin, on yüklem, desen), `cad.entities.relate` (Python, MCP), 33 durum GEOS'la (QGIS 4.2.3). Konuma göre seç'in toleranslı anlamı değişmedi. Arayüzde seçenek CBS-40'ta, PostGIS'e itilen sorguyla karşılaştırma CBS-04'te.*

  **Mevcut:** Altı ilişki ve sabit metrik toleranslı sorgu var [K06]. `contains`, sınırda içerme ve yakınlık aynı anlamda kullanılmamalı; PostGIS ile yerel sorgu farkı ürün hatasına dönüşmemeli.

  **Yapılacak kalan iş:** Standarda uygun `intersects/contains/within/touches/crosses/overlaps/equals/disjoint/covers/coveredBy` profili ve gerektiğinde DE-9IM pattern sorgusu tanımla. “Merkezi içinde” mevcut özel işlem olarak ayrıca kalsın. Eski toleranslı komutun anlamı sessizce değiştirilmesin; yeni standart profil sürümlü olsun. Kesin topolojik ilişki ile kullanıcının toleranslı yakınlık sorgusu farklı seçenek olsun. Düzlemsel/geodesic mesafe, uygun analiz CRS'si, sınır kuralı, boş/null/invalid geometri ve Z'nin hesaba katılıp katılmadığı sözleşmede yazsın. Indeks bbox'ı yalnız aday üretir; nihai geometri kararı ayrıca verilir. [S05], [S06]

  **Bağımlılık ve tamamlanma ölçütü:** CBS-02/07/08/13. Parselin sınırındaki nokta için covers ve contains beklenen farklı sonucu vermeli; açık çizginin uç noktası, multipart ve boş geometri fixture'ları olmalı. PG'ye itilen sorgu ile aynı destek profilinin yerel sonucu eşleşmeli. Bbox dönüşümü veya yüklenmemiş kayıtlar yüzünden yanlış negatif oluşmamalı; metre toleransı coğrafi derece gibi uygulanmamalı.

- [ ] **CBS-23 — Mevcut geoprocessing'e provider girdisi ve parça parça çıktı bağla — P0/P1**

  *Kısmen (18 Ekim 2026; [ADR 0508](../docs/adr/0508-processing-tools-as-commands.md) §4): `cad.processing.run`'ın `sink`'i sonucu çizim yerine yeni bir GeoPackage'a yazar (araç kopyada, yaptığı nesnelerin katmanları birer tablo, alanlar sütun, kimlikler `kentos_uids`'te, yaylar kesin, kalanlar `export_losses`; dosya bütün ya da hiç, var olan yalnız `overwrite` ile); cevabın `inputs`'u okunan katmanlar ve bağlı katmanın kaynak sürümü. G1'in kesişimi, yola tamponu ve Ada'ya göre birleştirmesi dosyaya yazılıp OGR'la bağımsız denetlenir (alanlar, öznitelik soyu, geçerlilik). Kalan: var olan GeoPackage'a tablo ekleme, kaynaktan sayfa sayfa okuma ve parça parça yazma, disk üstünde ara depo, iptalin dosyaya etkisi, PostGIS'e yazma.*

  *Kısmen (20 Ekim 2026; [ADR 0549](../docs/adr/0549-processing-into-existing-geopackage.md)): `sink` var olan GeoPackage'a (`addTables`, `replaceTables`; öbür tablolara dokunulmaz, aynı ad `table_exists`) tek SQLite işleminde ve 5000 satırlık gruplarla yazar, her soru noktasında iptal dosyayı bayt bayt olduğu gibi bırakır, yanıtın `sinkReport`'u tabloları, sütun eşlemesini, Z/M'yi ve dokunulmayanları söyler; GeoPackage 1.4 dosyasına kendi tetikleriyle. Kalan: girdiyi bağlı katmanın kaynağından sayfa sayfa okumak (araçların akışlı hesabı), masaüstü penceresinde sink seçeneği, PostGIS'e yazma.*

  **Mevcut:** Buffer, clip, dissolve, union/intersection/difference, repair, simplify, yakınlık, grid ve genelleştirme zaten var [K09]. Yapılacak iş bu algoritmaları yeniden listelemek değil, gerçek dataset ve çıktı kurallarıyla tamamlamaktır.

  **Yapılacak kalan iş:** Mevcut buffer/clip/dissolve/overlay/repair/simplify/yakınlık araçlarını yeniden geliştirme. Girdiyi tam source revision ve sorgu kapsamıyla provider'dan al; kolon/geometri akışını parça parça hedef sink'e yaz. Yeni adapter'da alan eşleme, kaynak kimliği, çoğalan satır, Z/M/eğri kaybı, boş sonuç ve iptal politikası görünür olsun. Var olan precision/ölçü ve öznitelik dağıtım kurallarını kullan. Global veri isteyen araçlar için uygun disk staging uygula; görünür alt kümede çalışıp tüm dataset işlendi deme. GEO-01 düzeltmesi CBS-13'tedir.

  **Bağımlılık ve tamamlanma ölçütü:** CBS-06/13/16/22/24. G1'de parsel+imar kesişimi, yola tampon ve sınıfa göre dissolve kayıpsız dosyaya üretilebilmeli. Alan toplamı, çıktı geçerliliği ve öznitelik provenance'ı bağımsız beklenenlerle tutmalı. İptal kaynak veriyi ve hedefin önceki sürümünü korumalı; GEO-01 kapanmadan zincirlenen birleşimin kabulü verilmemeli.

- [ ] **CBS-24 — Model çalıştırıcıya provider stream/sink, batch ve kalıcı run kaydı ekle — P1**

  *Kısmen (19 Ekim 2026; [ADR 0529](../docs/adr/0529-processing-batch-and-run-record.md)): `cad.processing.batch` ve modellerin komutla çalışması (`model:` kimliği), her çalışmanın kaydı (`kentos.run-record` 1: sürüm, parametre, girdi içerik özeti, çizim sürümü, srid, tohum, motor, adım başına kayıt sayısı ve süre, kesin toplam; aynı sürüm / aynı veri / değişmiş veri ayrımı) masaüstünde, Python'da ve MCP'de; üç adımlı parsel modeli komutta pencereyle aynı sonucu verir, ortadaki adım durunca yerel değişiklikler geri alınır. Kalan: provider stream girdisi, tablo ve raster sink'i, kaydın projede saklanması, `onError: continue`, satır içinde iptal, hızlı katman özeti.*

  *Kısmen (20 Ekim 2026; [ADR 0551](../docs/adr/0551-model-runner-batch-and-sink.md)): `onError: continue` (duran satır tek başına geri alınır, `failed` ve `batch_rows_failed` uyarısı; hepsi durursa hiçbir şey yazılmaz), durdurma çalışan aracın içinde de duyulur, `sink.tables` ile tablo çıktısı GeoPackage'a öznitelik tablosu, iş parçacıklı katman özeti (162 ms yerine 54 ms; özetin önbelleği değişiklik işaretiyle), `cad.processing.run` ve `.batch` protokolün kayıtlı adımı (komut geçmişi, `requestId`'li cevap istek defterinde); kayıt bilerek `.kcad`'e yazılmaz; Toplu çalıştır'da Hata olursa: Dur / Devam et. Kalan: sağlayıcıdan akan girdi, artımlı katman özeti, sink'in dosya etkisinin kayıtta yazılması, tek araç penceresinde sink seçeneği.*

  **Mevcut:** Parametreli araç, tool registry, model tasarımcısı, bağımlılık sırası, arka plan ve tek undo var [K09]. QGIS Processing/ArcGIS ModelBuilder'ın güçlü tarafı, işi tekrar çalıştırılabilir bir tanıma dönüştürmesidir. [A03]

  **Yapılacak kalan iş:** Mevcut registry, model tasarımcısı, bağımlılık sırası, arka plan ve tek undo'yu koru. Eksik provider dataset/feature stream girdisi, tipli tablo/raster/dosya sink'i ve batch parametre tablosunu aynı descriptor/executor üzerinden ekle; `python.md` ile ayrı araç standardı oluşturma. Model bağlantılarında akan kayıt sayısı/adım süresi ve çalışmanın kalıcı kaynak revision/filtre/CRS/seed/engine kaydı bulunsun. Yeniden çalıştırmayı aynı parametre-güncel veri ile aynı revision arasında ayır. [Q02]

  **Bağımlılık ve tamamlanma ölçütü:** CBS-02/10/23/39/41. Üç adımlı parsel modeli arayüz, Python ve MCP'de aynı sonuçları üretmeli. Orta adım hata/iptalde yerel değişiklikler geri alınmalı; uzun dosya çıktıları staged kalmalı. Bir veri kaynağını değiştirmek eski çalışmanın provenance'ını değiştirmemeli; ilerleme sayıları yaklaşık ise son kesin toplam ayrıca verilmelidir.

- [ ] **CBS-25 — Mevcut istatistiklere permutation/FDR ve tekrarlanabilir deney profili ekle — P1**

  *Kısmen (19 Ekim 2026; [ADR 0520](../docs/adr/0520-permutation-and-fdr.md)): Moran I, Gi* ve yeni Yerel Moran'da tohumlu permütasyon, FDR ve Bonferroni, deney profili ve alt küme etiketi masaüstünde. Kalan: DBSCAN ve k-ortalamaların kararlılık sınaması, En yakın komşu'nun permütasyonu, PySAL/esda ile çapraz sınama.* *Kısmen (21 Ekim 2026; [ADR 0566](../docs/adr/0566-nearest-neighbour-simulation-and-cluster-stability.md)): En yakın komşu'da Monte Carlo sınaması (aynı alanda tohumlu rastgele desenler, kenar etkisinden arınmış p), DBSCAN ve k-ortalamalarda Hennig'in bootstrap ve alt örnek kararlılığı (kümenin Jaccard ortalaması, yeniden bulunma, çözülme), deney profiliyle; esda, pointpats ve scikit-learn ile çapraz denetim. Kalan: çalışma alanının bir alan nesnesinden verilmesi, DBSCAN'ın parametre taraması, R'ın fpc'siyle çapraz denetim.*

  **Mevcut:** Moran's I, Getis-Ord Gi*, en yakın komşu, merkez/dağılım, DBSCAN/k-means ve k-d tree kaynakta var [K09]. Bu çekirdeklerin bulunması bütün istatistiksel geçerlilik seçeneklerinin tamamlandığını göstermez.

  **Yapılacak kalan iş:** Mevcut Moran I, Gi*, DBSCAN/k-means ve ağırlık matrisi araçlarını yeniden açma. Mevcut analitik p-değerine ek olarak permutation testi, seed kaydı ve çoklu test/FDR düzeltmesi ekle; gerekirse yerel Moran ayrı destek profili olsun. Yeni seçeneklerin sparse komşuluk/bellek sınırı ve sıfır komşu davranışı tanımlansın. Kaynak adapter'ı centroid/mesafe/CRS ve tam veri kapsamını korusun. Heatmap ile istatistiksel hot spot farkı, yeni test profili ve belirsizlik çıktısında açıklansın. [A04]

  **Bağımlılık ve tamamlanma ölçütü:** CBS-10/22/24/42. Bilinen küçük örnekler elle/bağımsız bilimsel referansla doğrulansın; sabit seed tekrarlanabilir sonuç versin. Sayısal z/p ile haritanın sınıf etiketi tutmalı. Tüm komşular yüklenmemişken harita alt kümesi üzerinden tam dataset istatistiği üretilmemeli; örnek veya AOI analizi açık etiketlenmeli.

- [ ] **CBS-26 — Mevcut rota analizine dönüş kısıtı, seviye ve zaman profili ekle — P1/P2**

  *Kısmen (19 Ekim 2026; [ADR 0521](../docs/adr/0521-turns-levels-and-time-profiles.md)): dönüş yasağı ve gecikmesi (geri dönüş kuralı, kavşakta açıya göre gecikme, dönüş katmanları), köprü ve tünelin seviyeleri, hızla maliyetin gün içi zaman profili (kapanmada beklenir, ilk giren ilk çıkar); `.kcad` şema 87, Ağlar penceresi, araçlarda Saat, İşlemler'de Kalkış saati, Python; bağımsız başvuru 39 soru. [ADR 0567](../docs/adr/0567-vehicle-profiles-and-dated-closures.md) (21 Ekim): araç profilleri (kenar alanlarına göre sınırlar, yasaklanan değerler, en yüksek hız, iki yön) ve tarihli kapanmalar (ilk ve son gün alanı); graf bir kez kurulur, profil ve gün maliyetleri değiştirir; `.kcad` şema 95, Ağlar penceresi, araçlarda Profil ve Tarih, İşlemler, Python; bağımsız başvuru 26 soru. [ADR 0568](../docs/adr/0568-weekday-and-hour-closures.md) (21 Ekim): kapanmanın haftanın günleri ve saatleri (`Pzt-Cum`, `22:00-06:00`); saatli kapanma kalkış saatiyle hız maliyetinin haftalık saatinde sıfır hız (bekler ya da dolaşır), öbür maliyetlerde sorunun anında kapalı; `.kcad` şema 96, Ağlar'da Gün ve saat, Python; bağımsız başvuru 18 soru, en erken varış Dijkstra'sı. [ADR 0570](../docs/adr/0570-why-a-stop-is-unreachable.md) (21 Ekim): ulaşılamayan durak ve çiftin nedeni kaynak nesnelerin kimlikleriyle (her kenarın açık olduğu en kısa yolu kapatan kenarlar ve nedenleri, engeller; kopuk ağ; dönüş, düğüm ya da cihaz); Maliyet matrisi satırı olmayan çiftleri sayar ve söyler (sıfır sayılmaz), En yakın tesis ulaşamayan olayın nedenini söyler; bağımsız başvuru 14 soru. Kalan: yedi günden uzun yolculuk, en küçük kesim, dış yönlendirici, bağlı katmanın revizyonu.*

  **Mevcut:** En kısa rota, duraklar, OD matrisi, en yakın tesis, servis alanı, yön/maliyet ve kapalı hat temeli var [K12]. İlk hedef bunların gerçek yol verisinde güvenilir kullanılabilmesidir.

  **Yapılacak kalan iş:** Mevcut rota/OD/tesis/servis alanı, tek yön, maliyet ve kapalı hat araçlarını yeniden geliştirme. Yeni kapsam; dönüş yasağı/cezası, köprü-tünel seviye ayrımı, profil ve zamana bağlı kapanma/maliyettir. Harici dataset revision'ı değişince graph/maliyet cache'i yenilensin; erişilemeyen durak/çiftin nedeni source ID ile verilsin. Kaynak maliyet birimini mevcut ağ sözleşmesinden taşı; yol parçaları ile tampon servis polygon'unu aynı çıktı sayma. Bölgesel ölçek gerektirirse Valhalla türü açık adapter ölçülerek seçilir. [D04]

  **Bağımlılık ve tamamlanma ölçütü:** CBS-07/13/22/24/33/42. Köprü altından geçen yol yanlış düğümle bağlanmamalı; yasak dönüş rota sonucunu değiştirmeli. Kapalı yol veya ağ güncellemesi eski graph üzerinden sessiz rota üretmemeli. OD matrisinde erişilemeyen çiftler sıfır maliyet sayılmamalı; servis alanı süre dilimleri ve alan hesabı bağımsız karşılaştırmadan geçmeli.

- [ ] **CBS-27 — Şebekeyi terminal/varlık ilişkisi ve artımlı graph sürümüyle genişlet — P1/P2**

  *Kısmen (19 Ekim 2026; [ADR 0525](../docs/adr/0525-network-related-records-and-unverified-traces.md)): Şebeke izleme projenin ilişkileriyle etkilenen kayıtları bulur (borunun aboneleri, vananın haznesi; Yalıtım'da beslemesiz kalanlarınki de), istemde adlarıyla sayar ve seçer; Denetle'nin bağlantı sorunlarından geçen izleme doğrulanmamış diye işaretlenir ve söylenir; Python'da `junctions`, `related`, `unverified`. [ADR 0526](../docs/adr/0526-event-drift.md): Olay kaymasını denetle, olay nesneleri rota kimliğiyle rotalarına bağlı; rotanın geometrisi ya da M'si değişince kayan ve gösterilemeyen olaylar raporlanır, istenirse yerlerine tek adımda taşınır. [ADR 0557](../docs/adr/0557-network-terminals-and-subnetworks.md): terminaller (cihazın terminalleri ve yolları, kenar uçlarının bağlı olduğu terminal; izleme ve Denetle yollardan geçer, terminalsiz uç doğrulanmamış) ve alt ağlar (Kaynak düğümleri denetleyici, yetim, paylaşılan, eski; `cad.subnetworks.find` ve `.update`, `.kcad` şema 93). [ADR 0562](../docs/adr/0562-network-terminals-everywhere-and-subnetworks-kept-current.md): en kısa yol, hizmet alanı, en yakın tesis ve maliyet matrisi de cihazdan yalnız terminal yollarıyla geçer; cihazın terminalleri noktasının özniteliğinden gelebilir (`by`, `configs`); `keepCurrent` alt ağ adlarını her yazmada aynı adımda yeniden yazar (`subnetwork_dirty`); `.kcad` şema 94. [ADR 0569](../docs/adr/0569-events-follow-their-routes.md): projenin olay bağları (`linearEvents`, `.kcad` şema 97, `cad.linearref.define`, Olay bağları penceresi): bağın olay katmanının olayları, rotası ya da kendisi değişen her yazmada aynı adımda rotalarının verdiği yere konur (Olay kaymasını denetle'nin kuralıyla; geri alma ikisini birlikte geri alır); yerleştirilemeyen olay olduğu yerde kalır ve yazma `events_dirty` uyarısıyla söyler. Kalan: etkilenen bölgenin artımlı kurulması, yalnız dokunulan rotaların olaylarının yerleştirilmesi.*

  **Mevcut:** Utility trace, kaynak/vana, ağ denetimi ve LRS route/calibration/event işlemleri mevcut [K12]. ArcGIS Utility Network'ten alınacak fikir, ağın bağlantı kuralları ve denetlenmiş türev graph yaşam döngüsüdür; Esri veri şeması zorunlu değildir. [A05]

  **Yapılacak kalan iş:** Mevcut vana, yön/bağlantı, utility trace ve LRS route/calibration/event araçlarını yeniden geliştirme. Kalan şebeke modeli; terminal, containment/attachment ve alt ağ varlık ilişkileridir. Dış dataset güncellenince graph'ın etkilenen bölgesini ve doğrulanmış source revision'ını izle; eski graph ile yeni feature'ları karıştırma. LRS'yi dış sağlayıcıya bağlarken mevcut M/kalibrasyon kuralını kullan ve kaynak değişiminde hangi event'lerin yeniden yerleştirileceğini kalıcı bağımlılıkla takip et. Yol chainage/station equation modeli `road.md`'de kalır.

  **Bağımlılık ve tamamlanma ölçütü:** CBS-09/14/15/24/33. Vana kapama senaryosu etkilenen aboneleri ilişkiler üzerinden bulabilmeli; doğrulanmamış bağlantı sonucu güvenilir trace diye sunulmamalı. Kaynak M ve geometri değişince event kayması raporlansın. İlk teslim basit su/hat şebekesi ve mevcut LRS kabulü olabilir; tüm elektrik/telekom mühendisliği veya fiziksel hidrolik solver önkoşul yapılmaz.

## F — Raster, uzaktan algılama ve büyük coğrafi veri

- [ ] **CBS-28 — Raster kaynağına sanal mozaik ve doğrulanan COG export ekle — P0 provider; P1 genişleme**

  *Kısmen (19 Ekim 2026; [ADR 0517](../docs/adr/0517-virtual-raster-mosaic.md)): çok dosyalı sanal mozaik GDAL'ın VRT'siyle, sıra, çözünürlük ve tarih önceliği, kopyasız; okuyucu ve komutun yazdığı GDAL'la piksel piksel aynı.*

  *Kısmen (19 Ekim 2026; [ADR 0515](../docs/adr/0515-cloud-optimized-geotiff.md)): KentOS'un yazdığı her raster COG'dur (dizinler başta, küçük katlar önce; GDAL'ın COG denetleyicisi hatasız ve uyarısız geçirir), Raster işlemleri'nde COG olarak kaydet; başsız sunucu bağlı ve gömülü GeoTIFF ve PNG'yi okur, raster araçları Python ve MCP'den çalışır. Kalan: sanal mozaik, ortak raster sağlayıcısı (Range, ETag, disk ve RAM bütçesi), codec profili.*

  **Mevcut:** GeoTIFF/BigTIFF, parçalı okuma, LZW/Deflate/PackBits, piramit/cache ve native raster katmanı var [K11]. COG yazma uygunluğu, mozaik kaynak yönetimi ve geniş codec desteği ayrıca kabul edilmelidir.

  **Yapılacak kalan iş:** Mevcut parçalı TIFF okuyucu, piramit, Range/cache ve band/NoData metadata'sını yeniden yazma; bunları ortak raster provider'a adapter olarak bağla. Yeni kapsam; çok dosyalı sanal mozaik, overlap/çözünürlük/tarih önceliği ve doğrulanmış COG dışa aktarımıdır. Tiled TIFF'in COG uygunluğunu bağımsız validator ile denetle; codec desteğini profilde bildir. Yeni kaynak adapter'ında Range desteği yokluğu, ETag değişimi ve disk/RAM bütçesi aynı kaynak yaşam döngüsüne bağlansın. [F02]

  **Bağımlılık ve tamamlanma ölçütü:** CBS-02/07/10/36/42. Çok gigabaytlı ortofoto ilk görünüm için bütünüyle indirilmemeli; veri transferi ve ilk yararlı görüntü süresi ölçülmeli. Mozaik birleşiminde seam/NoData delikleri veya yanlış band ölçeği oluşmamalı. Kaynak versiyonu değişince eski tile karışmamalı; desteklenmeyen sıkıştırmada açık native conversion yolu sunulmalı.

- [ ] **CBS-29 — Georeferans GCP/check point ve warp işlem kaydını kalıcılaştır — P1**

  *Kısmen (19 Ekim 2026; [ADR 0518](../docs/adr/0518-georeference-record.md)): rasterin oturtma kaydı (yöntem, kontrol ve denetim noktaları piksel köşeleriyle, hedeflerin sistemi, kaynak ve SHA-256, m0, denetimin karesel ortalama ve en büyük hatası, çıktı ızgarası ve örnekleme; `.kcad` şema 88), anlamsal `cad.rasters.georef` (yeniden örnekleme kaynağı değiştirmez, kayıtla yeniden çalıştırma aynı ızgarayı verir), Raster oturt'ta Denetim ve kayıttan açılış, `.points` dosyası, Python ve MCP. Kalan: dış sağlayıcının sistemine adaptör, projenin özel tanımında hedefler.*

  **Mevcut:** Helmert, affine, projective, polynomial 2/3 ve thin plate yöntemleri mevcut; inverse ve residual hesabı da var [K11]. Yeni bir georeferans motoru başlatmak gerekmiyor.

  **Yapılacak kalan iş:** Mevcut altı fit yöntemi, residual/RMS, nokta seçimi ve warp motorunu yeniden geliştirme. Kalan iş; GCP/check point seti, kaynak CRS, seçilen yöntem, bağımsız kontrol hatası ve çıktı grid/resampling ayarlarının kaynak revision'ıyla kalıcı, yeniden üretilebilir işlem kaydı olmasıdır. Mevcut raster dönüşüm arayüzü bu kaydı oluşturup geri açabilsin; dış provider CRS'sine dönüşüm yeni adapter'dan geçsin. Önceden bulunan GCP içe/dışa alma ve önizleme yolları tekrar yazılmadan kullanılsın. [F07]

  **Bağımlılık ve tamamlanma ölçütü:** CBS-07/08/10/28. Kontrol noktası sayısından fazla dağıtılmış check point ile doğruluk ölçülsün; RMS tek başına bütün görüntüde doğruluk garantisi olmasın. Döndürülmüş raster, piksel merkezi/köşesi ve NoData sınırında yarım piksel kayma olmamalı. Çıktı grid'i ve georeferans tekrar açmada aynı olmalı; yeniden örnekleme kaynak dosyayı sessizce değiştirmemeli.

- [ ] **CBS-30 — Mevcut raster analizlerini ortak provider/grid ortamına bağla — P1**

  *Kısmen (20 Ekim 2026; [ADR 0535](../docs/adr/0535-raster-analysis-area-and-grids.md)): yüzey analizinin sekiz aracında ortak AOI: Kapsam nesnelerin kutusu kaynağın ızgarasına oturur (aynı başlangıç, hücre boyu ve yön), kenar hücrelerinin komşuları kaynaktan okunur ve pencerenin değerleri bütün rasterinkiyle bit bit aynıdır (39 yüzey durumunda iki alanla, bağımsız başvurunun hücreleriyle); çalışma kaydı girdinin ve sonucun ızgarasını (afin, boy, bant, NoData, sistem, pencere) yazar. Kalan: öbür raster aileleri, ortak hücre boyu, hizalama ve NoData ortamı, büyük harici raster için disk ara dosyası, sağlayıcı karolarının halo karşılaştırması, belirsizlik raporu.*

  **Mevcut:** Eğim/aspect/hillshade, kontur, enterpolasyon, raster hesap, focal/reclass, uzaklık, uygunluk ve hidroloji aileleri mevcut [K09/K11]. İş paketinin değeri bunları aynı grid/NoData/birim sözleşmesinde güvenilir hale getirmektir.

  **Yapılacak kalan iş:** Mevcut DEM/raster cebiri, terrain, kontur, enterpolasyon, uygunluk ve hidroloji çekirdeklerini yeniden geliştirme. Yeni provider çalışmasının ortak AOI/grid origin/extent/hücre boyutu/band-time/NoData/birim bağlamını bu araçların mevcut parametrelerine eşle; ortak işlem kaydına yaz. Büyük harici rasterı mevcut blok/satır motoruna kontrollü ver; global geçiş isteyen araçlar için disk staging ekle. Yeni adapter'ın tile halo/grid birimini bozmadığı bağımsız karşılaştırmayla doğrulansın. Mühendislik TIN ve kazı/dolgu `road.md`'de kalır.

  **Bağımlılık ve tamamlanma ölçütü:** CBS-08/24/28/29/39. Düz yüzey, bilinen eğim, çanak ve drenaj örnekleri bağımsız referansla tutmalı; blok boyutunu değiştirmek sonucu sınırların ötesinde değiştirmemeli. Rasterın gösterim renkleri analizin kaynak değerlerini değiştirmesin. Uygunluk ağırlığı ve sensör/datum belirsizliği raporlansın; yağış-akış fiziği doğrulanmadan DEM drenaj analizi afet simülasyonu diye sunulmasın.

- [ ] **CBS-31 — Mevcut görüntü analizine STAC keşfi ve veri hazırlama kaydı ekle — P1**

  *Kısmen (19 Ekim 2026; [ADR 0528](../docs/adr/0528-stac-discovery.md)): STAC API araması (`cad.stac.search`: bbox, tarih, koleksiyon, bulut oranı, sayfalama), seçilen öğenin varlıklarının adresli raster olarak hazırlanması (`cad.stac.prepare`: ızgara, veri türü, nodata, ölçek ve öteleme, QA bandı ve maske kuralı, hazırlama kaydı), kalıcı varlık kimliği ile geçici imzalı adresin ayrımı (imza yazılmaz), masaüstü penceresi, MCP ve Python. Kalan: imzalı adres akışı, s3 adresleri, yeniden izdüşüm, maskenin hesaplara uygulanması, eğitim ve test örneği ile model kaydı.*

  **Mevcut:** Bant işlemleri, indis, sınıflandırma, doğruluk, değişim ve fusion araçları mevcut [K09]. Eksik günlük iş; görüntüyü bulma, doğru band/kaliteyle hazırlama ve güvenilir deney kaydını birleştirmektir.

  **Yapılacak kalan iş:** Mevcut band/indis/sınıflandırma/değişim/fusion hesaplarını koru. Yeni STAC adapter'ı bbox/tarih/cloud cover/sensör/band/lisansla collection/item/asset bulsun; kalıcı asset ID ile geçici signed URL'yi ayırsın. Seçilen ürünün band grid/scale-offset/QA mask ve acquisition time bilgisi mevcut hesap parametrelerine açık eşlensin. Eğitim/test örneği, model/sınıf/seed ve mekânsal doğrulama kaydı dış kaynak revision'ıyla saklansın; yalnız yeni eksik hazırlama adımları eklenir. [F06], [Q02]

  **Bağımlılık ve tamamlanma ölçütü:** CBS-10/24/28…30/33. Sentinel/Landsat benzeri açık ürünün desteklenen örneği baştan sona işlenebilmeli; bulutlu NoData pikselleri bitki kaybı diye sınıflanmamalı. Eğitim ve test aynı örneklerden gizlice oluşmamalı. Tarihler arası değişimde CRS/grid ve mevsim/ürün farkları raporlansın; hazır AI segmentasyon modeli yerleşik GIS'in önkoşulu olmasın.

- [ ] **CBS-32 — Mevcut bulut/mesh metadata'sını provider'a bağla ve eksik biçimleri genişlet — P1/P2**

  *Kısmen (20 Ekim 2026; [ADR 0536](../docs/adr/0536-point-cloud-metadata-and-lasting-ids.md)): nokta bulutunun kaynak bilgisi komutla (`cad.pointclouds.describe`: yatay ve düşey sistem, çekim günü, GPS zamanının türü, boyutlar ve ek baytlar, dönüş sayıları, COPC düğümleri; sınıf sayıları bütün dosyadan, örneklenmeden) ve nokta seçiminin kararlı kaynak kimliğine dönmesi (`cad.pointclouds.query`: bulut, kaynak ve kaynağın sırasındaki yer, kaynağın tam çözünürlüğünde, COPC'de yalnız değen düğümler okunarak); başsız sunucu, Python ve MCP bulutu okur. Kalan: mesh node-face ve zaman bilgisinin aynı sözleşmeye bağlanması, sağlayıcıda AOI, profil ve istatistik sorgusu, çoklu VPC özeti, NetCDF-4, HDF5, GRIB ve Zarr bağdaştırıcıları.*

  *Kısmen (20 Ekim 2026; [ADR 0552](../docs/adr/0552-point-cloud-areas-meshes-and-virtual-clouds.md)): `cad.pointclouds.region` (alanın içindeki noktaların sayısı, kotu, yoğunluğu ve sınıfları, kaynağın çözünürlüğünde, okunan nokta sayısıyla) ve `.profile` (hat boyunca koridor, istasyonlara göre; noktalar kalıcı kimlikleriyle), `cad.meshes.describe` ve `.query` (yüzün kalıcı kimliği, değerler, an ve seri), çok dosyalı sanal bulutun özeti ve uyuşmazlıkları; masaüstünde Nokta bulutu alan istatistiği, Nokta bulutu profili ve Mesh sor pencereleri, Öznitelikler'de satırlar. Kalan: NetCDF-4, HDF5, GRIB, Zarr (kitaplıkları sahibin onayını bekler), adresli bulutun ve NetCDF'in başsız okunması, metin bulutunda alan ve profil, mesh'te alan ve hat sorgusu.*

  **Mevcut:** LAS/LAZ/COPC/VPC, sınıflama/rasterleştirme, klasik NetCDF CDF-1/2/5, UGRID/2DM ve zaman serisi mevcut [K11]. Mesh'in raster olarak çizilmesi tam 3D mesh düzenleyicisi veya solver değildir.

  **Yapılacak kalan iş:** Mevcut LAS/LAZ/COPC/VPC ve klasik NetCDF/UGRID/2DM okuyucularını yeniden kurma. CRS/düşey datum, acquisition time, point dimension/classification ve mesh node-face/time bilgilerini genel provider/metadata sözleşmesine bağla. Yeni provider query'si AOI/profile/statistics için kaynak çözünürlüğünü istesin; render LOD'unu hesap verisi yerine kullanmasın. Çoklu VPC overview ve NetCDF-4/HDF5/GRIB/Zarr genişlemeleri mevcut capability matrisiyle ayrı doğrulanıp gereken adapter olarak eklenir. [F08], [Q02]

  **Bağımlılık ve tamamlanma ölçütü:** CBS-08/24/28/33/42 ve `road.md`/`3d.md`. Nokta/mesh seçimi kararlı kaynak kimliğine dönmeli; örneklenmiş görüntü tam bulut istatistiği sayılmamalı. Dosyanın küçük bölümü okunarak görünüm ve AOI sorgusu yapılabilmeli. Gerçek 3D kamera/sahne, engineering TIN ve fizik hesabı ilgili planlardan gelir; ilk 2D GIS sürümünü bunlara bağlama.

## G — Zaman, Türkiye iş akışları ve saha

- [ ] **CBS-33 — Zaman filtresini provider'a taşı; bitemporal/sensör kaynağı ekle — P1/P2**

  *Kısmen (20 Ekim 2026; [ADR 0537](../docs/adr/0537-bitemporal-layers.md)): geçerlilik zamanı ile kayıt zamanı ayrı modellenir (katmanın zaman ayarında sürümlerin `recordedStart` ve `recordedEnd`'i, `.kcad` şema 90); “şu tarihte geçerli” ile “o gün bilinen” `cad.time.query` ile ayrı sorulur (an ya da aralık, açık uç, eşit başlangıç ve bitiş, bölgeli anlar ve UTC, geç gelen kayıt); Zaman sürgüsünde Bilinen gün harita, seçim ve kenette aynı kuralla. Kalan: süzgecin sağlayıcıya itilmesi, değişmez revizyona bağlı tarihsel sorgu, sensör ve gözlem akışı (varlık kimliği, alım zamanı, kalite, geç ve yinelenen kayıt politikası), tablo, analiz ve dışa aktarmada aynı süzgeç.*

  **Mevcut:** Zaman sürgüsü, temporal filtre, raster time slice ve senaryo katmanları var [K12]. Görünürlük senaryosu, kayıt geçmişi ve fiziksel gözlem zamanı farklı kavramlardır.

  **Yapılacak kalan iş:** Instant/interval, açık uç, timezone/UTC ve takvim sözleşmesiyle zaman filtresi provider'a itilebilsin. Verinin gerçek dünyada geçerli olduğu zaman ile sisteme kayıt/işlem zamanı ayrı modellensin; tarihsel sorgu immutable revision'a bağlansın. “Şu tarihte geçerli imar durumu” ile “o gün sistemde bildiğimiz durum” gerektiğinde farklı cevap verebilsin. Sensör/gözlem akışı; asset ID, event/ingest time, quality ve geç gelen/tekrarlanan kayıt politikasını taşısın. OGC SensorThings ve QGIS 4.2'nin sürüm/extension discovery yaklaşımı ileri entegrasyon referansıdır. [S07], [Q02]

  **Bağımlılık ve tamamlanma ölçütü:** CBS-02/06/10/24/37 ve `3d.md`. Aynı tarih filtresi harita, tablo, analiz ve export'ta tutarlı olmalı. Zaman dilimi değişimi, eşit başlangıç-bitiş, açık aralık ve geç gelen gözlem fixture'ları geçmeli. Senaryo görünürlüğü değişince geçmiş veri değiştirilmemeli; sensör akışı kopunca son değer güncelmiş gibi gösterilmemeli.

- [ ] **CBS-34 — Mevcut parsel/ölçme araçlarını TUCBS profil ve referans GIS projesine bağla — P0/P1**

  *Kısmen (20 Ekim 2026; [ADR 0541](../docs/adr/0541-data-profiles.md)): veri profili veri olarak (`catalog/v1/profiles/tucbs-temel.json`, sürüm 1: mahalle, ada, parsel, bina, yol, adres ve planlı alan kullanımı sınıfları katman yolları, alanları, kimlikleri ve parselin kayıtlı alanı ve toleransıyla; dört değer listesi; parsel–bina ve bina–adres ilişkileri; adres araması); kolon adı kodda yok. `cad.profiles.apply` eksik grup, katman, alan, değer listesi, ilişki ve adres verisini ekler, var olanı korur; `cad.profiles.check` eksikleri, alan türünü, geometriyi, boş zorunlu alanı, kurala uymayan değeri, yinelenen kimliği, kayıtlı alanla geometrik alanın farkını (hiçbiri öbürünü ezmez) ve kopuk bağı söyler. Profilin temeli resmî uygulama şeması olmadığını söyler. Kişisel verisiz referans belediye projesi `fixtures/profiles/v1`; masaüstünde Veri profili penceresi; Python ve MCP. Kalan: kurumun profilinin içe alınması ve sürüm göçü, kurum eşlemeleri, adres normalizasyonu ve yinelenen adres kodunun gösterilmesi, ifraz, tevhit, hisse ve PlanGML, gerçek kullanıcılarla günlük kullanım kabulü.*

  *İkinci dilim (20 Ekim 2026; [ADR 0558](../docs/adr/0558-institution-profile-import-migration-mapping.md)): kurumun kendi profili. Profil ve eşleme çizimin yanındaki `profiller` klasöründe dosya; `cad.profiles.import` getirilen JSON'u (ya da profilin ve eşlemenin kendisini) biçim ve sürüm denetimiyle alır (`unknown_profile_format`, `unsupported_profile_version`, `invalid_profile`, `invalid_profile_mapping`), eski ve yeni sürümü ayırır (`profile_downgrade`, `profile_replaced`), geçici adla yazıp geri okuyarak yerine koyar; `cad.profiles.list` KentOS'un ve projenin profillerini ve eşlemelerini sürüm, sınıf ve göç sayılarıyla söyler; `cad.profiles.migrate` profilin sürümleri arasında veriyi kendi adımlarıyla (katman ve alan adı, alan türü kuraldan geçirilerek, değer yeniden kodlama ve bölme, kodu kullanımdan kaldırma) taşır: planı, tek geri alma adımı (katman adları hariç), uymayan değerler değer değer ve nesne kimlikleriyle; `cad.profiles.apply` ve `.check` eşlemeyle (`mapping`, `mappingName`) kurumun katman, alan ve kod adlarıyla çalışır, hiçbir ad kendiliğinden yeniden adlandırılmaz. Ortak durumlar KentOS kodu olmadan yazılmış başvurudan (import 40, list 26, migrate 40; apply 53, check 39); masaüstünde Veri profili penceresinde profil ve eşleme seçici, İçe aktar… ve göç; Python ve MCP. Paketle projenin `profiller/` dosyalarını bayt bayt pakete taşır (profil ve eşleme olarak manifestte; okunamayan söylenir). Kalan: profil dosyalarının `.kcad`'de taşınması, göçün eşlemeyle çalışması, değer listesi kimliği değiştirme.*

  **Mevcut:** Ölçme, parsel üretimi, geocoding, kayıtlı alan ayrımı ve CAD hassasiyeti var. Bunlar kendiliğinden onaylanmış kadastro/imar ürünü anlamına gelmez [K04/K05/K12].

  **Yapılacak kalan iş:** İlk referans projesi; idari sınır, ada/parsel, bina, yol, adres ve imar kullanımını ilişkili veri olarak içersin. TUCBS veri sözlüğü/modeli üzerinden kimlik, domain, metadata ve kurum eşlemeleri sürümlü profil olarak kurulmalı; uygulama koduna sabit kolon adı gömülmemeli. Geometrik alan, tapu/kayıt alanı ve hukuki hak/hisse ayrı tutulmalı. Adres normalizasyonu, Türkçe arama, yinelenen kod ve geocoding confidence/aday seçimi açıklanmalı. İfraz/tevhit, hisse dağıtımı ve PlanGML gibi uzman teslimler mevcut plan/TODOS işleriyle ortak, bağımsız kurum kabulüne tabi olmalı. [T01], [T02]

  **Bağımlılık ve tamamlanma ölçütü:** CBS-06…16/26 ve form/yol planları. G1'de kişisel veri içermeyen gerçekçi belediye örnek projesiyle günlük kullanım kabulü yapılmalı. Hesap alanı kayıtlı alanı otomatik ezmemeli; kimlikler join/aktarımı korumalı. Resmî çıktı/kural sürümü ve yetkili onay olmadan mevzuata tam uygunluk iddiası yapılmamalı. Adres araması ticari servis gerektirmeden yerel/kurum verisiyle çalışabilmeli.

- [ ] **CBS-35 — Dosyadan GNSS alımını canlı cihaz ve AOI offline replica ile genişlet — P1/P2**

  **Mevcut:** GPX/NMEA dosya alımı, GNSS yükseklik farkındalığı, cloud yerel kopya ve PR form ürünü mevcut [K05/P25]. Canlı cihaz toplama ve AOI bazlı feature replika bunlardan ayrı kabul ister.

  **Yapılacak kalan iş:** Canlı GNSS adapter'ında fix/RTK durumu, yatay/düşey doğruluk, timestamp, anten yüksekliği ve koordinat dönüşümü kayıtla ilişkilensin. Manuel/GNSS konumu ile son bilinen konum ayrılmalı; doğruluk eşiği, ortalama alma ve geometriyi güncelleme açık seçenek olsun. Çevrimdışı paket; AOI, kaynak revision, form release, domain/ilişki, medya ve izin kapsamını taşısın. Outbox kimlikleri ve idempotent gönderim, eklerin staged upload'u, silme tombstone'u, çatışma ve schema migration tasarlansın. Field Maps'in alan paketi/sync akışı karşılaştırma örneğidir; ürün bağımlılığı değildir. [A07]

  **Bağımlılık ve tamamlanma ölçütü:** `FORM-MOBILE/WORK`, CBS-02/08/15/36…39. İnternet yokken oluşturulan kayıt+alt kayıt+fotoğraf tekrar açmada kalmalı; iki kez sync yinelenen kayıt yaratmamalı. İzin veya form sürümü değişince güncel sunucu politikası uygulanmalı; yerel emek kurtarılabilmeli. İlk yerel GIS gerçek mobil uygulama beklemez; ortak standart hazırlanır, mobil renderer kendi tesliminde doğrulanır.

## H — Kalıcılık, birlikte çalışma ve işletim

- [ ] **CBS-36 — Mevcut .kcad projesine provider kaynak manifesti ve taşınabilir paket ekle — P0**

  *Kısmen (17 Ekim 2026; [ADR 0503](../docs/adr/0503-bound-layers.md), [ADR 0506](../docs/adr/0506-relative-sources-and-package.md)): `.kcad` şema 74'ün `sources`'ı: kaynak kimliği, dosya ve tablo, katman eşlemesi, içerik özeti (SHA-256), EPSG, göreli yol (kaydetmede yazılır, açılışta aynı içerikteki dosya bulunur); Kaynak ▸ Yeniden bağla; Paketle (`file.package`): proje ve veri/ altında bağlı GeoPackage, resim, raster (dünya dosyalarıyla), bulut, tablo ve ek kopyaları, bütün ya da hiç, SHA-256 denetimiyle; veritabanı, adres ve eksik dosya söylenir. Kalan: PostGIS tablosunun çevrimdışı kopyası, lisans ve erişim süzgeci, bağı gömülü kopyaya çevirme.*

  **Mevcut:** `.kcad`, bağlı/gömülü raster ve kitaplık, sunucu snapshot/revision ve kurtarma temeli mevcut [K01/K04/K11]. Genel sağlayıcı eklenince proje açmanın veri kopyalamaktan ayrılması gerekir.

  **Yapılacak kalan iş:** Proje manifesti; kaynak kimliği/bağlantı referansı, dataset/layer mapping, CRS dönüşüm bağlamı, şema/alan ID, stil, form/ilişki, named view, model ve pafta bağımlılıklarını sürümlü saklasın. Relative path, yeniden konumlandırma ve içerik hash'i desteklensin. Bağlı kaynak, gömülü snapshot ve offline çalışma kopyası kullanıcı seçimi olsun; parola/refresh token/signed URL proje içine yazılmasın. Paketle komutu lisans/erişim sınırına göre asset'leri ve kaynakları toplasın; bütünlüğü doğrulayıp atomik dosya değişimiyle bitirsin. `.kcad` hızlı sorgu/replica veritabanı yapılmasın.

  **Bağımlılık ve tamamlanma ölçütü:** CBS-02/03/07/10/15/17/37. Başka bilgisayara taşınan proje katman, kimlik, form, ilişki, CRS ve stillerle açılmalı veya eksik bağımlılıkları tek tek bildirmeli. Eksik raster/DB kaynağı önceki içeriğin silinmesine yol açmamalı. PR ile `main`'in schema/format/migration çakışmaları her birleştirmede yeniden çözülmeli; iki codec ve bağımsız okuyucu aynı sürümü anlamalı.

- [ ] **CBS-37 — Mevcut sürüm denetimini dış kaynak/ilişki çatışması ve review ile genişlet — P1**

  *Kısmen (20 Ekim 2026; [ADR 0538](../docs/adr/0538-source-reconcile.md)): tek revizyon üzerinde iyimser düzenleme ve açık uzlaştırma. Bağın tabanı (`.kcad` şema 91: son eşleşmede her nesnenin katmandaki ve tablodaki özeti) ile `cad.sources.reconcile` dış QGIS düzenlemesini ve katmanın işini nesne nesne ayırır: tek yanın değişikliği o yandan, aynı değişiklik uyuşur, öbürleri çatışma (farklı öznitelikler iki yanın değerleriyle, geometri, silme ile değiştirme); son yazan kazanmaz, yan `take`/`rest` ile seçilir, çözülmeden yürümez; yürütme tek geri alma adımı ve tek işlem. Masaüstünde Kaynakla uzlaştır penceresi, reddedilen yazma onu açar. 31 ortak durum GDAL'ın yazdığı iki dosyayla, yazılan dosya OGR'la denetlenir. Kalan: alan alan birleştirme, ilişkili değişikliklerin birlikte denetimi, çok katmanlı uzlaştırma, bulut ve sunucu işinin aynı sözleşmesi, adlı sürüm ve review.*

  *Kısmen (20 Ekim 2026; [ADR 0553](../docs/adr/0553-field-merge-groups-and-layers.md)): tabanın alan kaydı (`.kcad` şema 92: tür ve geometri, öznitelik ve köşe özetleri) ile iki yanın farklı alanlarını değiştirdiği nesne tabana karşı alan alan birleşir (yalnız aynı alanı farklı değiştirmek çatışır, iki değerle; çözüm çatışan yeri seçer); ortak köşeyi iki yanın da taşıdığı komşular ve silinen üst kayıtla eklenen alt kayıt bir grup olur ve birlikte, tek yanla çözülür; `with` ile aynı GeoPackage dosyasının katmanları tek çağrıda ve tek SQLite işleminde uzlaştırılır; bulut istemcisi dışarıdan gelen değişiklikte ve sunucunun 409'unda farklı alanları birleştirir. Kalan: sunucuda `project.changes`'in istemcinin tabanıyla alan alan birleştirmesi (ADR'de tasarlandı), çok parçalı nesnenin köşe bağı, alan sınırlı kopyanın dışındaki komşular, PostGIS'te alan birleşimi, adlı çalışma sürümü ve review → publish.*

  **Mevcut:** `project.changes`, expected version, idempotency, audit/outbox ve cloud senkronizasyonu var [K04/K13]. Bu temel sıfırdan kurulmaz. ArcGIS versioning'deki yararlı fikir, uzun düzenleme işini üretimden izole edip denetimli birleştirmektir. [A06]

  **Yapılacak kalan iş:** Feature/alan/geometri ve ilişki düzeyindeki çatışmalar anlamlı farkla gösterilsin; son yazan kazanır varsayımı kullanılmasın. Provider revision sözleşmesi harita, form taslağı, source cache ve job commit'inde aynı olsun. Önce tek revision üzerinde iyimser düzenleme ve açık reconcile çöz; ihtiyaç kanıtlanırsa adlı çalışma sürümü/change set ve review→publish akışı ekle. Yeni MVCC/WAL motoru yazma; PostgreSQL transaction'ları ve sürümlü uygulama kayıtları kullanılsın. Ortak kenar veya N:M bağı gibi ilişkili değişiklikler birlikte çatışma kontrolünden geçsin.

  **Bağımlılık ve tamamlanma ölçütü:** CBS-04/11/12/15/35/38. İki editörün aynı kaydı, komşu parselleri veya üst/alt kaydı değiştirdiği senaryolar veri ezmeden çözülebilmeli. Dış QGIS edit'i, bağlantı kopması, yinelenen istek ve yetki iptali sınansın. Kullanıcı kendi taslağını dışa alabilsin; eski sürüme dayanan job sonucu yeni kaydın üzerine otomatik yazılmasın.

- [ ] **CBS-38 — Mevcut yetki modelini yeni dataset/satır/alan/job kapsamına uygula — P0/P1**

  **Mevcut:** Proje/tenant/RLS ve PR'de form design/publish/use, entry rolü ve alan projection'ı var [K13/P25-C]. Bunları yok sayan ikinci izin sistemi kurulmaz; eksik görev/satır/alan ve dış kaynak kapsamı tamamlanır.

  **Yapılacak kalan iş:** Kaynak, dataset, katman, satır, alan, geometri, export, medya, işlem ve onay yetkileri ortak resolver'da değerlendirilir. Kaynak DB rolü ile KentOS rolünün otoritesi açık olsun; doğrudan PG erişimi kurum DB yetkisini aşamaz. Count/aggregate, lookup, ilişki grafı, bbox/extent, hata raporu, tile/preview/cache ve AI bağlamı da aynı projection'a tabi olsun. Parametreli sorgu/identifier doğrulama, secret store ve credential redaction uygulanır. RLS owner/BYPASSRLS ve pool scope davranışı test edilir. [D03]

  **Bağımlılık ve tamamlanma ölçütü:** CBS-02/04/09/15/35/37/41/43. Formda gizli alan tablo export'u veya MCP sorgusundan okunamamalı; erişilemeyen kaydın varlığı autocomplete/sayımda açığa çıkmamalı. Yetki işlem çalışırken kaldırılırsa commit reddedilmeli. İndirilmiş bir offline kopyanın uzaktan geri alınabileceği iddia edilmemeli; geri alınabilen erişim ile önceden indirilmiş veri ayrılmalı.

  *Kısmen (20 Ekim 2026; [ADR 0539](../docs/adr/0539-form-scope.md)): Veri girişinin satır kapsamı. Formun gövdesinde `scope` ifadesi (katmanın alanları ve `@` değerleri; yeni `@kullanici_eposta` oturumdaki kişinin e-postası), çekirdek `kentos_forms::scope` (canlı formlardan biri ulaşırsa kayıt erişilebilir; boş, yanlış ya da derlenmeyen kapsam hiçbir şey vermez), ön denetimin `form_scope_invalid`'i ve sürüm farkı. Sunucuda Veri girişinin nesne sayfaları, kimlikle okuması ve projenin nesne sayısı kapsamın dışındakileri görmez; kapsam dışındaki kayda yazma 404, kaydı kapsamın dışına çıkaran yazma 403, denetim proje kilidi altındaki commit'te. Masaüstünde Formlar penceresinde Kapsam. Geçici PostGIS'te iki Veri girişi ve düzenleyiciyle uçtan uca test. Kalan: olayların kimlik listesi, sağlayıcıya ve DB rolüne taşınma, dataset/katman/alan/dışa aktarma/medya/işlem/onay yetkilerinin ortak çözücüsü, yapay zekâ bağlamı ve önbellekler.*

  *Kısmen (20 Ekim 2026; [ADR 0550](../docs/adr/0550-one-reach-resolver.md)): tek çözücü `kentos_forms::reach` (çağıran sınıfı, alan düzeyi gizli/işaretli/açık, kayıt sorgusu, kopya kuralı, damga; 16 ortak durum bağımsız Python başvurusundan). Sunucuda Veri girişinin sayısı ve sınırı, olayları, çakışması, formun liste değerleri (var olmayan değerle aynı ret) ve geçmiş sayıları aynı çözücüden; bütün formları kaldırılmış katman hiçbir şey vermez. `redact` alan proje dosyasından ve dışa aktarma işinden kesilir (form tasarlayan hariç); saklı kopya yalnız kesilmeden alabilene verilir; işin kesinleşmesi damgayı yeniden sorar (`reach_changed`); yerel çizim ve başsız sunucu tüm hak; indirilmiş çevrimdışı kopyanın geri alınamayacağı ADR'de ve paylaşım penceresinde söylenir. Kalan: kapsamın SQL'e itilmesi, ayrı olay günlüğü, saklı kopyaların kesilebilmesi, kaynak, dataset, medya ve onay yetkileri, AI bağlamı ve önbellekler.*

- [ ] **CBS-39 — Yerel arka plan işleminin yanına dayanıklı server job ve restore kabulü ekle — P1**

  *Kısmen (20 Ekim 2026; [ADR 0542](../docs/adr/0542-durable-server-jobs.md), [ADR 0547](../docs/adr/0547-server-jobs-limits-and-desktop.md)): kalıcı sunucu işi (kimlik, değişmez girdi, kira ve çit, deneme ve süre sınırı, staged sonuç, iptal) ve sınırları: kurum başına sırada ve çalışan iş sınırı (`quota_exceeded`, eşzamanlı isteklerde de), sonuç boy sınırı (`too_large`), yazılamayan depo (`storage`), biten işin saklama süpürmesi, `kentosd jobs` ölçümleri; iki iş türü (`project.export`, `project.checkpoint`); masaüstünde Sunucu işleri sekmesi (iste, yokla, Durdur, İndir, pencere kapanınca iş kalır). Geçici PostGIS'te ve gerçek kentosd ile sınandı. Kalan: başka iş türleri, CPU/bellek kotası, işçinin ilerleme yazması, önbellek anahtarı, yedek ve geri yükleme tatbikatı.*

  *Kısmen (20 Ekim 2026; [ADR 0559](../docs/adr/0559-processing-server-jobs.md)): İşlemler araçları sunucu işi (`project.processing`): `cad.processing.run`'ın girdisi, sonuç tek komutla projeye kesinleşir (proje aracın okuduğu revizyondaysa, yetki ve kira işin bitirme işleminde, geçmeyen bitirme commit'i de geri alır) ya da GeoPackage dosyası olur; araç ayrı süreçte (`kentos-jobrun`) çalışır, işçi CPU ve belleği ölçer ve bütçeyi (`cpu_limit`, `memory_limit`, `memory_estimate`) uygular; işçi ilerleme ve sözcüklerini yazar; masaüstünde Nerede çalışır › KentOS sunucusunda ve Sunucu işleri'nde işlem satırları; Python `processing_job`. Gerçek kentosd ve çocuk süreçle sınandı. Kalan: MCP, blok yazan ve 5000'den çok nesne yazan araçlar, dosya alan araçlar, modeller, büyük içe aktarma ve atlas PDF'i, kullanıcı başına kota, Linux'ta ölçümün sınanması, önbellek anahtarı, yedek ve geri yükleme tatbikatı.*

  *Kısmen (20 Ekim 2026; [ADR 0561](../docs/adr/0561-import-atlas-and-mcp-jobs.md)): dosya içe aktarma ve atlas PDF'i sunucu işi (`project.import`, `project.atlas`): hazırlanan dosya `kentos.job_input` (tek akışlı yükleme, boy ve SHA-256 denetimi), çocuk süreç masaüstünün `file.import.gpkg`, `.fgb`, `.gml`, `.csv`, `.dxf` ve `file.export.atlas` komutlarını çalıştırır; sonuç tek commit (blok tanımları delta'da, işin kesinleşmesi 100 000 değişikliğe kadar) ya da pin'i denetlenen PDF artifact'ı (Range); komut kodu işin kodu, kayıplar sonuçta; MCP `jobs.start`, `jobs.status`, `jobs.cancel`, `jobs.result`; Python `import_job`, `atlas_job`; masaüstünde “Sunucuya gönder” penceresi ve Sunucu işleri'nde iki tür; `job_run` testlerindeki düşüşün kök nedeni (kısa çocuk için kullanım örneğinin alınamaması) giderildi. Geçici PostGIS'te, gerçek `kentosd`, çocuk süreç, Python ve MCP ile sınandı. Kalan: Shapefile, GeoJSON ve KML içe aktarma işleri, parçalı yükleme, gerçek Linux'ta `/proc` ölçümü ve cgroup, kullanıcı başına kota, önbellek anahtarı, yedek ve geri yükleme tatbikatı.*

  **Mevcut:** Native background thread, iptal, raster/service cache ve sunucu outbox var; kalıcı server worker ayrı teslim [K09/K11/K12].

  **Yapılacak kalan iş:** Job kimliği, immutable girdi/parametre kaydı, durum/ilerleme, artifact ve yeniden bağlanma sun. Sunucu işinde lease/heartbeat/fencing, retry/idempotency, CPU/RAM/disk kota, timeout ve staged sonuç uygula; pencere/socket kapanması işi belirsiz bırakmasın. Kaynak revision ve izin commit anında tekrar doğrulansın. Cache anahtarı source revision+query+CRS/style/time+yetki kapsamını içersin; cache temizleme kaynak veriyi silmesin. Backup; PostgreSQL, nesne deposu ve bağlı asset sürümlerini tutarlı restore edebilsin. Sağlayıcı/iş/indeks durumları ölçülebilir olsun; log'a veri/parola dökülmesin.

  **Bağımlılık ve tamamlanma ölçütü:** CBS-10/24/28/35…38/42. G1'de iptal, kapanma ve yerel recovery geçmeli; sunucu scheduler gerekmiyor. G2'de worker ölmesi, disk dolması, yarım upload, duplicate dispatch ve kaynak değişimiyle sınanmalı. Restore tatbikatı revision/kimlik/ek/form ilişkilerini korumalı; backup alındı bildirimi restore kanıtı yerine geçmemeli.

  *Kısmen (20 Ekim 2026; [ADR 0542](../docs/adr/0542-durable-server-jobs.md)): kalıcı sunucu işi: kimlik, değişmez girdi ve SHA-256'sı, istendiği revizyon, durum ve ilerleme, istek kimliğiyle tek iş (duplicate dispatch), kira, yoklama ve çit (ölen işçinin işi sonrakine geçer, geç bitirme geçmez), deneme ve süre sınırı, iptal; `project.export`'un sonucu staged, bitirmede yetki ve revizyon yeniden sorulur, sahipsiz dosyayı temizlik siler; işçi `kentosd serve`'ün yanında ya da `kentosd worker`. Geçici PostGIS'te sınandı. Kalan: başka iş türleri, CPU/bellek/disk kotası, masaüstünde işler listesi, önbellek anahtarı, disk dolması ve yarım yükleme senaryoları, yedek ve geri yükleme tatbikatı, ölçümler.*

## I — Kullanım, otomasyon, performans ve teslim kabulü

- [ ] **CBS-40 — Yeni provider akışını mevcut GIS arayüzüne bağla ve kullanıcı kabulünü yap — P0**

  **Mevcut:** Şerit, arama, tool dialog, bilgi kartı ve klavye odak altyapısı var; PR formları geliştiriyor [K09/K10/P25]. Çok araç bulunması kullanıcının doğru aracı ve hedefi bulabildiğini kanıtlamaz.

  **Yapılacak kalan iş:** Mevcut şerit, arama, identify kartı, işlem penceresi ve klavye/tema altyapısını yeniden geliştirme. Yeni source/dataset durumu, yazılabilirlik, filtreli-yüklü-toplam kapsam, CRS ve pending commit/taslak bilgisi bu yüzeylerde görünsün. Provider hatası, eksik sayfa, bağlantı kopması ve yeniden konumlandırma aynı akıştan çözülebilsin. Kullanıcı kabulünü yeni GeoPackage/PostGIS işinin baştan sona tamamlanmasına odakla; mevcut araçları yeni oldukları varsayımıyla ayrı backlog'a taşıma.

  **Bağımlılık ve tamamlanma ölçütü:** CBS-01/11/14/20/24. İki referans kullanıcı grubu—GIS uzmanı ve form kullanan belediye personeli—G1 akışını geliştiriciden komut adı öğrenmeden tamamlasın. Tamamlama oranı, süre, yanlış hedef/kapsam ve kurtarma ihtiyacı kaydedilsin; tek tasarımcı ekranı veya screenshot kullanılabilirlik kabulü sayılmasın. Kısayol ve menüler `catalog/v1/additions.json` üzerinden kaydedilsin; web envanteri açılmasın.

  *Kısmen (20 Ekim 2026; [ADR 0543](../docs/adr/0543-source-status-and-way-out.md)): bağlı katmanın durumunda yüklü ve toplam satır, nesne sayısı ve durumun istediği iş (`next`: yaz, oku, uzlaştır, yeniden bağla; işi götüren öneri yok), Python ve MCP'de de; masaüstünde Kaynak durumu penceresi kaynak, salt okunurluk, durum, kapsam ve sistemle, satırın düğmesi o işi Kaynak ▸'nın komutlarıyla yapar. Kalan: Öznitelik tablosu ve bilgi kartı, süzgeç ve bulut taslağı, sağlayıcı hataları, iki referans kullanıcı grubuyla kabul.*

  *Kısmen (20 Ekim 2026; [ADR 0546](../docs/adr/0546-attribute-table-outgoing-and-source-line.md)): Öznitelik tablosunda bağlı katmanın Kaynak satırı (Kaynak durumu'nun durumu, yüklü, gösterilen, seçili ve toplam satır, durumun istediği tek iş). Kalan: bilgi kartında kaynak, süzgeç ve bulut taslağı, sağlayıcı hataları, iki referans kullanıcı grubuyla kabul.*

- [ ] **CBS-41 — Yeni GIS provider ve Processing seçeneklerini komut/Python/MCP'ye aç — P0/P1**

  *Kısmen (18 Ekim 2026; [ADR 0508](../docs/adr/0508-processing-tools-as-commands.md)): İşlemler'in yerleşik araçları `cad.processing.tools` (araçlar, parametreleri, çizimin varsayılanları) ve `cad.processing.run` (pencerenin çalıştırıcısı, açık değerler, nesneler kalıcı kimliklerle ya da girdinin açık `selection` ve `view`'u; retler sırayla, eklenen, değişen, seçilen, katmanlar, çıktılar, notlar; plan kopyada) ile Python'a ve MCP'ye açık; ev sahipleri araçları kaydeder (`processing_host`). Kalan: başsız dosya erişimi (raster, bulut, NetCDF araçları), modeller, pencerenin yazmasının komut kaydına girmesi, uzun iş yolu, işlem + seçenek + veri türü + ev sahibi kapsam matrisi, doğal dil başarısının ayrı ölçümü.*

  *Kısmen (20 Ekim 2026; [ADR 0548](../docs/adr/0548-coverage-matrix-and-run-equivalence.md)): işlem + seçenek + veri türü + ev sahibi kapsam matrisi `scripts/gis/coverage.py` → `docs/gis/coverage.{json,md}` (213 komutun 3534 seçenek hücresinin 1180'i, 151 aracın 769 hücresinin 436'sı bir durumda deneniyor); eşdeğerlik testi vektör ve raster araçlarının ve modellerin 420 durumunu İşlemler penceresinden ve `cad.processing.run`'dan oynatır, çizim, katmanlar, özet, notlar, retler ve yazılan dosyalar (bayt bayt) aynı; ev sahibi `drawing` alanlı komutlara açık çizimin dosyasını verir (Python ve MCP'de raster göreli yolları çizimin klasöründen ve arama yollarından bulunur). Her yazan komut kayıtta kendi adı ve girdisiyle (`call::recorded`; ortak durum koşucusu denetler; on bir komut düzeldi, planın kuruluğu adımın içinde korunur). Kalan: nokta bulutu ve çok boyutlu araçların başsız yolu, açık seçenek hücreleri.*

  **Mevcut:** Headless/native registry, Python ve MCP mevcut; ağ/servis/zaman için de Python yolları bulunuyor [K13]. Bütün GIS veri kaynaklarının ve Processing seçeneklerinin ortak yüzeyi ayrıca kapatılmalı.

  **Yapılacak kalan iş:** Source discovery/query, schema/domain, feature edit, form/ilişki, CRS, analiz/model, job ve export sürümlü tipli command/query sözleşmesiyle erişilsin. `validate/plan/execute`, etki sayıları, scope, unit, expected revision, uyarı, cancellation ve artifact cevabı ortak olsun. UI hesaplarını Python/AI kullanıcısına yeniden yazdırma; doğrudan SQL/repository ile iş kuralı atlatma. Ekrana ait locator/selection etkileşimleri session ID ile yönetilsin; headless yolu fare istemesin. `.kek` ve üçüncü taraf Processing/widget lifecycle'i `python.md`'yi kullansın; yerleşik GIS araçları Python kurulmadan çalışsın.

  **Bağımlılık ve tamamlanma ölçütü:** Bütün maddelerin kendi çıkış kapısı; CBS-02/15/24/38. Aynı proje ve açık parametrelerle UI, Python ve MCP aynı semantic changeset/uyarıyı vermeli. Kapsam matrisi yalnız komut adı değil **işlem+seçenek+veri türü+host** düzeyinde tutulmalı. AI planı görünmeyen kayıt veya yetki uyduramasın; salt okunur plan belge revision/dirty/undo durumunu değiştirmesin. Doğal dil başarısı API kapsamından ayrı ölçülsün.

- [ ] **CBS-42 — Yeni provider akışında sınırlı bellek ve performans bütçelerini doğrula — P0/P1**

  *Kısmen (18 Ekim 2026; [ADR 0509](../docs/adr/0509-provider-baseline-and-link-memory.md), [ölçüm](../docs/perf/provider-2026-10-18.md)): 1 milyon parsel ve 10 milyon köşelik GeoPackage'da ölçüm tabanı (makine, release derleme, verinin SHA-256'sı, sıcak önbellek). Sağlayıcının ilk sayfası 0,1 ms, anahtarla sonraki sayfa 0,1 ms, mekânsal dizinle görünen kutu 0,6 ms; bellek veriyle büyümüyor. Bağlamanın belleği düzeltildi: katmanın özeti ve dosyanın SHA-256'sı akışla, yeni katmana bağlamada satırlar bir kez okunuyor; 100 000 parselde 1,3 GB'tan 323 MB'a. Kalan: bağlı katman bugün çalışma kopyası (1 milyonda 13,7 s, 3 GB): görünür küme ve sayfa sayfa okuma; çizim alanında kenet, seçim ve kare süreleri; soğuk önbellek, yavaş ağ; 10 milyon satır ve 20 GB stres.*

  **Mevcut:** Spatial index, kirli güncelleme, GPU batching, tablo sanallaştırma, raster/cloud range ve arka plan işi temelleri var [K01/K09…K12]. Bunlar korunur; tüm belgeyi dolaşma/indirme varsayımı provider katmanında kaldırılır.

  **Yapılacak kalan iş:** Visible set, bbox/kolon/filtre pushdown, kararlı paging, index ve bounded CPU/RAM/VRAM/disk cache ortak planlansın. Render LOD yalnız görüntü maliyetini düşürür; source query, snap, ölçüm ve final analiz hassasiyeti ayrı kalsın. UI thread'de I/O/decode/analiz olmasın; kaynak sayfası ve GPU upload artımlı gelsin. Baseline; cihaz/driver, release build, veri hash'i, cold/warm cache, ağ ve kalite ayarıyla kaydedilsin. Sentetik veri yanında yoğun parsel, karmaşık geometri ve yavaş ağ verisi kullanılsın.

  **Bağımlılık ve tamamlanma ölçütü:** CBS-02/04/13/20/24/28/32. Başlangıç hedefleri: 1 milyon feature/10 milyon köşe içeren testte bütün veri indirilmeden ilk yararlı görünüm; yerel ilk tablo sayfası ≤1 s, sıcak görünümde pick/snap p95 ≤30 ms, sürekli gezinmede p95 frame ≤33 ms. Bunlar **ölçülmemiş aday bütçelerdir**; G0'da donanım/örnek/ağ tanımıyla kesinleştirilir. 10 milyon satırlı provider ve 20 GB raster/bulut stresinde bellek veri büyüklüğüyle sınırsız artmamalı. Hedefe ulaşmak için etiket/geometri gizlice eksiltilmemeli.

- [ ] **CBS-43 — Mevcut servis istemcisini canlı feature provider ve açık filtre profiliyle genişlet — P1; yayın P2**

  *Kısmen (18 Ekim 2026; [ADR 0513](../docs/adr/0513-feed-completeness-and-filter-profile.md)): Servisten veri al'ın bitişi `complete`, `capped`, `short`, `pages` (sınırlı ya da sayfalı servisin eksik sonucu tam sayılmaz; WFS 2.0 sunucunun kestiği sayfada sürer); OGC API'nin uygunluk sayfası profil olarak okunur (Bölüm 1–4, CQL2; Bölüm 4 taslak, kullanılmaz), CQL2 metin süzgeci yalnız bildiren servise gider. Bağımsız başvuru `feed_paging_cases.py` sahte sunucuyla; Python yerel sunucuyla uçtan uca. Kalan: canlı sağlayıcı (servis üstünde sorgu ve düzenleme), expires/ETag, belirteç yenileme, kesilen ağ, CQL2 JSON ve queryables, QGIS'in FeatureServer kıyası, yayın (P2).*

  **Mevcut:** WMS/WMTS/XYZ/MVT, WFS/OGC API Features/ArcGIS REST okuma, auth/cache/atıf ve feed yenileme mevcut [K12]. İlk GIS için harita sunucusu/Martin eşdeğeri kurmak zorunlu değildir; mevcut yayın kapsamı sınırı korunur.

  **Yapılacak kalan iş:** Mevcut WMS/WMTS/XYZ/MVT ve feature feed/auth/cache istemcilerini yeniden geliştirme. Ayrı kalan canlı feature provider'ını mevcut bağlantı/capability okumaya bağla; kopya veri alma ile source üzerinde canlı sorgu/düzenlemeyi ayrı göster. OGC API Features Part 1/2/3 ve CQL2 için desteklenen filtre/CRS profilini açık capability olarak bildir; yeni query adapter'ının paging/filtre sonucunu tam veriyle karşılaştır. QGIS 4.2 FeatureServer bbox veri çekme yaklaşımı yeni provider'ın performans kıyasıdır. [S08], [S09], [Q02]

  **Bağımlılık ve tamamlanma ölçütü:** CBS-02/07/38/39/42. Limitli/sayfalı servis eksik sonucu “tam veri” saymamalı; expires/ETag, token yenileme, kesilen ağ ve attribution çalışmalı. **P2'de ayrıca seçilirse** açık OGC API/tiles yayın servisi veya açık kaynak server adapter'ı; immutable yayın revision'ı, stil/metadata, erişim ve cache izolasyonuyla eklenir. Ticari servis/DB bağımlılığı oluşmaz; kaynak CAD/GIS ile yayın tile'ı ayrı kalır. İlgili OGC standardının yazma kısmı taslaksa bu durum sürüm profilinde açıkça yazılır.

- [ ] **CBS-44 — Yeni provider'lar için QGIS 4.2 round-trip ve uçtan uca kabul korpusu ekle — P0**

  *Kısmen (17 Ekim 2026; ADR 0502): `fixtures/geopackage/v1` — QGIS 4.2.3'ün yazdığı GeoPackage → KentOS → GeoPackage → GDAL/OGR ve QGIS 4.2; kimlik, Türkçe metin, NULL/boş, delik, çok parça, Z/M, eğri korunumu; GDAL doğrulayıcısı. Kalan: PostGIS, yetki, restore ve performans kabulü.*

  *Kısmen (20 Ekim 2026; [ADR 0554](../docs/adr/0554-gis-acceptance-project.md)): CBS'nin ilk uçtan uca kabul projesi (`fixtures/acceptance/v1/parcels.json`, beklentiler `acceptance_parcels.py`'den, KentOS kodu olmadan): TUREF/TM36 projede parsel ve bina katmanları tipli alanlarıyla, NULL ve boş metin ayrı, yanlış değerin geri alınması, Alan hesapla, Konuma göre seç, kaydet ve aç, GeoPackage, FlatGeobuf, GML, Shapefile ve CSV'ye dışa aktarma ve GeoPackage'ın yeni projede eksiksiz açılması Python'da; beş dosya OGR'la (QGIS 4.2'nin GDAL'ı) katmanlar, sistem, her alanın değeri, halkalar ve alanlarla denetlenir. Kalan: aynı senaryo MCP'de ve masaüstünde, PostGIS, yetki, restore, kesilen işlem ve yeniden açma, performans kabulü.*

  **Mevcut:** Bağımsız Python referansları, ortak fixture'lar, interaction izleri, hata testleri ve performans kayıtları mevcut [K01/K08/K09]. Yeni GIS kabiliyeti bu altyapıya bağlanır; ekran veya komut ismi tamamlandı sayılmaz.

  **Yapılacak kalan iş:** Mevcut bağımsız Python referansları, fixture/interaction izleri, native test ve performans çatısını koru. Yeni GeoPackage/PostGIS/provider akışı için QGIS **4.2** karşılıklı dosya testi, kimlik/şema/null/CRS/ilişki korunumu ve kesilen işlem/yeniden açma senaryolarını ekle. Aynı yeni komutun UI/Python/MCP sonuçları ile yetki/restore/performance kabulü uygun mevcut koşucuya bağlansın. Eski özelliğin rutin regresyonu yeni eksik özellik sayılmaz; CBS-13'teki bilinen yanlış sonuç kapatılmadan ilgili kabul verilmez.

  **Bağımlılık ve tamamlanma ölçütü:** G1 çıkışı aşağıdaki korpusun yerel kapsamının geçmesidir; G2/G3'te aynı korpus dış kaynak/yetki/ölçekle genişler. Kanıt dosyası fixture, gerçek veri lisansı/hash'i, sürüm, komut, sonuç ve ölçümü taşır. PR'de test geçtiği beyanı, bu incelemede çalıştırılmış test diye sunulmaz. Son kabul; bir parsel/bina GIS projesinin KentOS içinde oluşturulup düzenlenmesi, sorgulanıp doğrulanması ve başka cihaz/uygulamada veri kaybı olmadan açılmasıdır.

## Ortak kabul korpusu ve aşama kapıları

Bu tablo yeni takip kutusu eklemez; yukarıdaki **44 kalan işin test kapsamıdır**. Mevcut altı yetenek yeni geliştirme diye sayılmaz. G1'de yerel, G2'de PG/çok kullanıcı, sonraki dilimlerde ilgili ileri veri profilleri çalıştırılır.

| Durum | Beklenen kanıt |
|---|---|
| QGIS 4.2 GeoPackage → KentOS → QGIS 4.2 | Aynı kayıt kimliği, Türkçe alan/metin, null/boş ayrımı, domain eşlemesi, multipart/delik ve desteklenen Z/M; raporlanan her kayıp. |
| Farklı CRS'de parsel+bina+ortofoto | Doğru hizalama ve ters identify/snap; harita CRS'sini değiştirmek kaynak veriyi değiştirmez; datum/grid seçimi kayıtlı. |
| Parsel split/merge + form + malik ilişkisi | Yeni/korunan kimlikler, doğru alan dağıtımı, korunmuş ilişki ve tek atomik undo; kilit/validation hatasında yarım kayıt yok. |
| Sınırdaki nokta, boş/invalid geometri | Contains/covers/touches ve toleranslı yakınlık farkları açık; yerel ve PG aynı destek profilinde aynı sonucu verir. |
| GEO-01 ve teğet tampon zinciri | Etkin test ve bağımsız alan doğrulaması; GEO-02'nin ayrıca ölçülen süre/bellek sonucu. |
| Ortak sınır ve görünüm dışındaki komşu | Topoloji güncellemesi bütün gerekli kayıtlara ulaşır; artımlı QA ile tam QA aynı; eski istisna yeni hatayı saklamaz. |
| Büyük tablo ve filtreli harita | Bbox/kolon/filtre pushdown, kararlı paging, doğru tam/filtreli toplam; yüklenen alt küme tam veri sayılmaz. |
| İki kullanıcı + dış QGIS edit'i | Revision çatışması ve anlaşılır fark; izin/form sürümü değişiminde yeniden doğrulama; sessiz overwrite yok. |
| Entry/alan/satır yetkisi | Form, tablo, lookup, count, ilişki, export, cache ve AI'da aynı erişim; yetkisiz değer/sayı sızıntısı yok. |
| Uzun model, iptal, worker ölümü | Tek mantıksal işlem, staged artifact ve yeniden bağlanma; eski kaynak revision'ına yazma yok. |
| Büyük raster/bulut ve farklı tile boyutu | Sınırlı I/O/bellek, doğru NoData/halo; final hesap render LOD'undan etkilenmez. |
| Zamanlı/sınıflandırılmış/istatistiksel veri | Timezone, grid, seed, ağırlık ve test örneği provenance'ı; sayısal sonuç–harita sınıfı eşleşmesi. |
| Paketle, taşı, yeniden aç, restore | Kimlik, şema/form/ilişki, stil, kaynak ve asset'ler tutarlı; eksik bağımlılık açık raporlu; sırlar pakette yok. |
| Aynı iş UI/Python/MCP | Aynı kapsam, parametre, uyarı, semantic result ve undo; plan/önizleme salt okunur. |
| Basit çıktı ve sonraki atlas | Gerçek ölçek, font, legend, CRS/GeoPDF ve izinli alanlar; atlas sayfaları aynı sabit veri revision'ından. |

## Araştırma kaynakları ve sürüm disiplini

Araştırma tarihi **17 Ekim 2026**. Kaynaklar üreticinin/projenin kendi belgeleridir. Kıyaslanan özellikler ile KentOS için önerilen uygulama tasarımı ayrıdır; buradaki mimari, öncelik ve kabul eşikleri KentOS önerileridir. `latest/stable/current` sayfalarının gerçek sürümü uygulama ADR'sine sabitlenmeli; bugün görülen sayfa gelecekte değişebilir. QGIS'in testing'e yönlenen kılavuzları sürüm kanıtı olarak kullanılmamıştır.

| Kod | Birincil kaynak ve bu plandaki kullanımı |
|---|---|
| Q01 | [QGIS 4.2 kullanıcı kılavuzu](https://docs.qgis.org/4.2/en/docs/user_manual/), [resmî yol haritası](https://qgis.org/resources/roadmap/): karşılaştırma tabanı ve desteklenen sürüm; alt sayfa yönlendirmesi yukarıda açıklanmıştır. |
| Q02 | [QGIS 4.2 resmî değişiklik kaydı](https://qgis.org/project/visual-changelogs/visualchangelog42/): hızlı stil erişimi, kolon hesabı, model adım sayıları, FeatureServer fetch, STAC, VPC, SensorThings ve GeoPDF katman yönetimi. Bunların KentOS uyarlamaları ayrı geliştirme hedefleridir. |
| Q03 | [QGIS release-4_2 kaynak dalı](https://github.com/qgis/QGIS/tree/release-4_2): kullanıcı kılavuzu yönlendirmesinde sürüm sınırını korumak için birincil kaynak. |
| Q04 | [QGIS 4.2 vector provider sözleşmesi](https://raw.githubusercontent.com/qgis/QGIS/release-4_2/src/core/vector/qgsvectordataprovider.h): capability, veri erişimi ve provider sınırı. |
| Q05 | [QGIS 4.2 CoordinateTransformContext API](https://api.qgis.org/api/4.2/classQgsCoordinateTransformContext.html): kaynak–hedef işlem bağlamı, axis order ve fallback politikasını ayrı yönetme. |
| A01 | [ArcGIS Pro Error Inspector](https://doc.esri.com/en/arcgis-pro/latest/help/editing/validate-and-fix-geodatabase-topology.html): bulguya gitme, inceleme, istisna, düzeltme ve yeniden denetleme. |
| A02 | [ArcGIS Pro map series](https://doc.esri.com/en/arcgis-pro/latest/help/layouts/map-series.html): katmana bağlı çok sayfalı çıktı iş akışı. |
| A03 | [ArcGIS Pro ModelBuilder](https://doc.esri.com/en/arcgis-pro/latest/help/analysis/geoprocessing/modelbuilder/what-is-modelbuilder-.html): tekrar çalıştırılabilir analiz zinciri yaklaşımı. |
| A04 | [ArcGIS Pro Getis-Ord Gi*](https://doc.esri.com/en/arcgis-pro/latest/tool-reference/spatial-statistics/h-how-hot-spot-analysis-getis-ord-gi-spatial-stati.html): hot spot yorumlama, p/z ve çoklu test düzeltmesi. |
| A05 | [ArcGIS Pro network topology](https://doc.esri.com/en/arcgis-pro/latest/help/data/utility-network/about-network-topology.html): bağlantı kuralları, graph ve doğrulanmamış bölgelerin yaşam döngüsü. |
| A06 | [ArcGIS Pro versioning](https://doc.esri.com/en/arcgis-pro/latest/help/data/geodatabases/overview/overview-of-versioning-in-arcgis-pro.html): izole düzenleme ve denetimli birleştirme kavramı; Esri uygulama/DB gereksinimi alınmamıştır. |
| A07 | [Field Maps offline harita hazırlama](https://doc.arcgis.com/en/field-maps/latest/prepare-maps/configure-the-map.htm), [sync akışı](https://doc.arcgis.com/en/field-maps/android/use-maps/sync.htm): alan paketi, ilişkili veri ve eşitleme kullanıcı akışı. |
| A08 | [ArcGIS attribute rule fundamentals](https://doc.esri.com/en/arcgis-pro/latest/help/data/geodatabases/overview/an-overview-of-attribute-rules.html): calculation, constraint ve validation'ın farklı değerlendirme zamanları. |
| A09 | [ArcGIS query layer](https://doc.esri.com/en/arcgis-pro/latest/help/mapping/layer-properties/what-is-a-query-layer-.html): SQL sonucu katman/tablo, benzersiz ID ve salt okunur sınır. |
| S01 | [OGC GeoPackage](https://www.ogc.org/standards/geopackage/), [GeoPackage 1.4 tanımı](https://www.geopackage.org/spec140/index.html): açık SQLite tabanlı paket, feature/tablo/tile ve extension sınırları. |
| S02 | [PROJ koordinat işlemi seçimi](https://proj.org/en/stable/operations/operations_computation.html): işlem adayları, kullanım alanı, doğruluk, grid ve ballpark seçimi. |
| S03 | [OGC API Records](https://ogcapi.ogc.org/records/): metadata keşfi; Core ile taslak ek parçaların ayrımı. |
| S04 | [GEOS doğruluk/model FAQ](https://libgeos.org/usage/faq/): düzlemsel hesap ve sonlu hassasiyet; kütüphanenin mutlak doğruluk garantisi sayılmaması. |
| S05 | [PostGIS ST_Relate](https://postgis.net/docs/ST_Relate.html): DE-9IM ve standart ilişkilerin sınır anlamları. |
| S06 | [PostGIS ST_IsValid](https://postgis.net/docs/ST_IsValid.html): 2D validity kapsamı; Z/M bulunmasının 3D geometri doğrulaması olmaması. |
| S07 | [OGC SensorThings](https://www.ogc.org/standards/sensorthings/): varlık/gözlem entegrasyonunun açık standart ailesi; uygulamada seçilen sürüm/extensions sabitlenir. |
| S08 | [OGC API Features](https://ogcapi.ogc.org/features/), [Part 3 Filtering](https://docs.ogc.org/is/19-079r2/19-079r2.html): okuma, CRS, filtre ve ayrıca ilan edilecek yazma kapsamı. |
| S09 | [OGC CQL2](https://docs.ogc.org/is/21-065r2/21-065r2.html): servis filtresinin açık profil ve conformance sınırı. |
| D01 | [PostGIS spatial indexing](https://postgis.net/workshops/postgis-intro/indexing.html), [ST_Transform](https://postgis.net/docs/ST_Transform.html): aday arama/indeks ve CRS etiketi–gerçek dönüşüm ayrımı. |
| D02 | [PostgreSQL transaction isolation](https://www.postgresql.org/docs/18/transaction-iso.html): transaction ve snapshot sınırları; uygulama undo'suyla karıştırılmaması. |
| D03 | [PostgreSQL row security](https://www.postgresql.org/docs/18/ddl-rowsecurity.html): RLS, owner ve BYPASSRLS sınırları. |
| D04 | [Valhalla resmî belgeleri](https://valhalla.github.io/valhalla/): ileri ölçek ihtiyacı için araştırılabilecek açık kaynak yönlendirme adapter'ı. |
| F01 | [GDAL GeoPackage driver](https://gdal.org/en/stable/drivers/vector/gpkg.html): native adapter için veri türü, yazma ve capability araştırması. |
| F02 | [GDAL COG driver](https://gdal.org/en/stable/drivers/raster/cog.html): oluşturma, overview/codec ve bağımsız uygunluk denetimi. |
| F03 | [GDAL OpenFileGDB](https://gdal.org/en/stable/drivers/vector/openfilegdb.html): Esri SDK/veritabanı bağımlılığı gerektirmeyen dosya uyumluluğu seçeneği. |
| F04 | [GeoParquet 1.1](https://geoparquet.org/releases/v1.1.0/): açık analitik dosya biçiminde geometri/CRS metadata'sı. |
| F05 | [FlatGeobuf](https://flatgeobuf.org/): büyük vektör veri dağıtımı ve spatial index yaklaşımı. |
| F06 | [STAC specification](https://stacspec.org/en/about/stac-spec/): collection/item/asset ve uzay-zamanlı veri keşfi. |
| F07 | [GDAL warp](https://gdal.org/en/stable/programs/gdalwarp.html): hedef grid, reprojection, resampling ve NoData seçenekleri. |
| F08 | [COPC 1.0](https://copc.io/): range-readable nokta bulutu ve kaynak çözünürlükte veri erişimi. |
| T01 | [TUCBS temel dokümanlar](https://cbs.csb.gov.tr/tucbs-temel-dokumanlar-86100), [genel kavramsal model](https://cbs.csb.gov.tr/genel-kavramsal-model-86101): yerel domain profili için resmî çerçeve; belirli kurum uygunluk kabulünün yerine geçmez. |
| T02 | [TUCBS coğrafi veri sözlüğü](https://verisozlugu.tucbs.gov.tr/): tema, kod ve alan eşleme araştırmasının kaynağı. |

[Q01]: https://docs.qgis.org/4.2/en/docs/user_manual/
[Q02]: https://qgis.org/project/visual-changelogs/visualchangelog42/
[Q03]: https://github.com/qgis/QGIS/tree/release-4_2
[Q04]: https://raw.githubusercontent.com/qgis/QGIS/release-4_2/src/core/vector/qgsvectordataprovider.h
[Q05]: https://api.qgis.org/api/4.2/classQgsCoordinateTransformContext.html
[A01]: https://doc.esri.com/en/arcgis-pro/latest/help/editing/validate-and-fix-geodatabase-topology.html
[A02]: https://doc.esri.com/en/arcgis-pro/latest/help/layouts/map-series.html
[A03]: https://doc.esri.com/en/arcgis-pro/latest/help/analysis/geoprocessing/modelbuilder/what-is-modelbuilder-.html
[A04]: https://doc.esri.com/en/arcgis-pro/latest/tool-reference/spatial-statistics/h-how-hot-spot-analysis-getis-ord-gi-spatial-stati.html
[A05]: https://doc.esri.com/en/arcgis-pro/latest/help/data/utility-network/about-network-topology.html
[A06]: https://doc.esri.com/en/arcgis-pro/latest/help/data/geodatabases/overview/overview-of-versioning-in-arcgis-pro.html
[A07]: https://doc.arcgis.com/en/field-maps/latest/prepare-maps/configure-the-map.htm
[A08]: https://doc.esri.com/en/arcgis-pro/latest/help/data/geodatabases/overview/an-overview-of-attribute-rules.html
[A09]: https://doc.esri.com/en/arcgis-pro/latest/help/mapping/layer-properties/what-is-a-query-layer-.html
[S01]: https://www.ogc.org/standards/geopackage/
[S02]: https://proj.org/en/stable/operations/operations_computation.html
[S03]: https://ogcapi.ogc.org/records/
[S04]: https://libgeos.org/usage/faq/
[S05]: https://postgis.net/docs/ST_Relate.html
[S06]: https://postgis.net/docs/ST_IsValid.html
[S07]: https://www.ogc.org/standards/sensorthings/
[S08]: https://ogcapi.ogc.org/features/
[S09]: https://docs.ogc.org/is/21-065r2/21-065r2.html
[D01]: https://postgis.net/workshops/postgis-intro/indexing.html
[D02]: https://www.postgresql.org/docs/18/transaction-iso.html
[D03]: https://www.postgresql.org/docs/18/ddl-rowsecurity.html
[D04]: https://valhalla.github.io/valhalla/
[F01]: https://gdal.org/en/stable/drivers/vector/gpkg.html
[F02]: https://gdal.org/en/stable/drivers/raster/cog.html
[F03]: https://gdal.org/en/stable/drivers/vector/openfilegdb.html
[F04]: https://geoparquet.org/releases/v1.1.0/
[F05]: https://flatgeobuf.org/
[F06]: https://stacspec.org/en/about/stac-spec/
[F07]: https://gdal.org/en/stable/programs/gdalwarp.html
[F08]: https://copc.io/
[T01]: https://cbs.csb.gov.tr/tucbs-temel-dokumanlar-86100
[T02]: https://verisozlugu.tucbs.gov.tr/
[P25A]: https://github.com/mcihad/kentos-cad/tree/5307026911844d754862b78c3106deb7296f0d4b/crates/shared/forms
[P25B]: https://github.com/mcihad/kentos-cad/blob/5307026911844d754862b78c3106deb7296f0d4b/crates/shared/contracts/src/forms.rs
[P25C]: https://github.com/mcihad/kentos-cad/blob/5307026911844d754862b78c3106deb7296f0d4b/crates/shared/contracts/src/fields.rs
[P25D]: https://github.com/mcihad/kentos-cad/blob/5307026911844d754862b78c3106deb7296f0d4b/crates/native/application/src/forms_submit.rs
[P25E]: https://github.com/mcihad/kentos-cad/blob/5307026911844d754862b78c3106deb7296f0d4b/crates/native/domain/src/relation_follow.rs
[P25F]: https://github.com/mcihad/kentos-cad/blob/5307026911844d754862b78c3106deb7296f0d4b/docs/adr/0440-relation-graph-and-submission.md
[P25G]: https://github.com/mcihad/kentos-cad/blob/5307026911844d754862b78c3106deb7296f0d4b/crates/server/application/src/form_guard.rs
[P25H]: https://github.com/mcihad/kentos-cad/blob/5307026911844d754862b78c3106deb7296f0d4b/docs/adr/0411-form-permissions-and-server-guard.md
[P25I]: https://github.com/mcihad/kentos-cad/blob/5307026911844d754862b78c3106deb7296f0d4b/docs/adr/0439-sources-references-and-migration.md
[P25J]: https://github.com/mcihad/kentos-cad/blob/3106609e12a037016353f826a2610bab0c6a01ba/docs/adr/0441-designer-extras.md
[P25K]: https://github.com/mcihad/kentos-cad/blob/3106609e12a037016353f826a2610bab0c6a01ba/crates/shared/forms/src/requirements.rs
[P25L]: https://github.com/mcihad/kentos-cad/blob/3106609e12a037016353f826a2610bab0c6a01ba/apps/desktop/src/forms/entry.rs
