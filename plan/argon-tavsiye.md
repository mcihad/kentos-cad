# KentOS CAD — Mimari Değerlendirme, Uygulama Ayrımı ve Geliştirme Tavsiyeleri

İnceleme tarihi: **12 Kasım 2026**. Kapsam: **KentOS Mimari**, **KentOS Harita**, **KentOS GIS** ve **KentOS Yol** modüllerinin ortak kod tabanı, derleme/dağıtım mimarisi ve ölçeklenebilirlik yol haritası.

Bu belge; mevcut ~2,1 milyon satırlık Rust (50+ crate) ve ~315 bin satırlık TypeScript kod tabanının yapısal değerlendirmesini, dört ana mühendislik disiplininin **tek yazılım içinde mi yoksa ayrı derlenen uygulamalar olarak mı** yürütülmesi gerektiğine dair mimari kararı ve kod tabanının sürdürülebilirliği için **madde madde (`ARG-01` … `ARG-12`) uygulama tavsiyelerini** içerir.

---

## 1. Mevcut Yapı ve Mimarinin Değerlendirmesi

### 1.1. Kod Tabanının Sayısal Görünümü (12 Kasım 2026)

| Katman / Crate | Dosya Sayısı | Satır Sayısı | Sorumluluk ve Mevcut Durum |
|---|---:|---:|---|
| `apps/desktop` | 1.159 | ~549.800 | Masaüstü kabuğu, şerit, tüm modül pencereleri ve ekran testleri (`src/` kökünde 650 `.rs` dosyası) |
| `crates/native/application` | 756 | ~301.500 | Dört disiplinin tüm anlamsal ürün komutu işleyicileri |
| `crates/shared/geometry-core` | 491 | ~200.900 | 2B/3B temel analitik geometri, Delaunay, exact predicates + alana özgü geometrik kurallar |
| `crates/native/interaction` | 363 | ~142.400 | Çizim/düzenleme araç oturumu (`Session`, `Tool`), kenetleme ve önizleme |
| `crates/shared/formats` | 221 | ~92.800 | DXF, LandXML, Shapefile, GeoJSON, CSV/XYZ, PDF/A, raster/mesh okuyucu ve yazıcıları |
| `crates/shared/commands` | 207 | ~90.300 | Ürün komutlarının şema/tip tanımları ve katalog üretimi |
| `crates/shared/contracts` | 173 | ~86.500 | Ortak veri sözleşmeleri (`Entity`, `ProjectSettings`, `DocumentSnapshotV1`, ayarlar) |
| `crates/ui` | 155 | ~85.000 | Iced 0.14 tabanlı KentOS UI bileşen kütüphanesi ve tema jetonları |
| `crates/native/processing` | 142 | ~49.000 | İşlemler (`processing.*`) araçları, model tasarımcısı ve arka plan iş koşucusu |
| `crates/shared/kcad` | 116 | ~41.100 | `.kcad` v2 CBOR kapsayıcısı, şema doğrulama ve sürüm geçişleri |
| `crates/server/application` | 77 | ~40.800 | Bulut/PostGIS proje işlemleri, revizyon, paylaşım ve yetkilendirme |
| `crates/native/domain` | 116 | ~39.800 | Masaüstü belge modeli (`Document`), işlem geçmişi (` undo/redo`), katmanlar ve takipçiler |
| `crates/shared/sheet` + `sheet-ui` | 131 | ~63.600 | Pafta düzeni çekirdeği, PDF/GeoPDF yazımı, Türk ruhsat/antet şablonları ve masaüstü arayüzü |
| `crates/shared/raster` | 86 | ~32.900 | Yüzey analizi, eşyükselti, eğim/bakı, hidroloji ve uzaktan algılama çekirdeği |
| `crates/shared/road` | 116 | ~24.500 | Yol mühendisliği çekirdeği (arazi yüzeyi, yatay/düşey güzergâh, dever, kesit, kübaj) |
| `crates/render/wgpu` | 43 | ~17.500 | Native `wgpu` çizim hattı, 65 km karo merkezli FP32/FP64 hassasiyet ve stil batch'leri |

### 1.2. Mimarideki Güçlü Temeller (Korunması Gereken İlkeler)

1. **Tek Yönlü Bağımlılık Denetimi (`scripts/arch/deps.mjs`):**
   - Saf hesap crate'lerinin (`crates/shared/*`) hiçbir UI (`iced`), GPU (`wgpu`), veritabanı (`sqlx`), HTTP (`axum`/`reqwest`) veya tarayıcı (`wasm-bindgen`) bağımlılığı alamaması ve bunun her test koşusunda `cargo tree` üzerinden otomatik denetlenmesi, projenin mimari çürümesini (architectural drift) önleyen en güçlü mekanizmadır.
2. **"Araç = Anlamsal Komut" ve Üçlü Yüzey Bütünlüğü (CLAUDE.md §9.2):**
   - Çizimi veya projeyi değiştiren her işlemin `kentos-commands` kataloğunda tipli bir komut olması; masaüstü UI, Python SDK (`kentos.cad`) ve AI/MCP (`kentos-mcp`) yüzeylerinin aynı doğrulama, yetki, geri alma (`undo`) ve plan/önizleme sözleşmesini kullanması endüstri standardının üstünde bir tasarımdır.
3. **Kadastral ve Mühendislik Sayısal Doğruluğu (CLAUDE.md §23):**
   - `libm` ve `serde_json/float_roundtrip` ile platformlar arası bit-bit determinizm; geometrik alan, tapu/kayıtlı alan ve gösterim değerinin ayrılması; `rust_decimal` ve pay/payda hisse koruması harita/kadastro ve yol hakediş hesapları için sağlam bir temel oluşturmaktadır.
4. **Dört Disiplinin Aynı Belge ve Veri Modeli Üzerinde Buluşması:**
   - Mimari, Harita, GIS ve Yol disiplinleri gerçek hayatta birbirinin çıktısını girdi olarak kullanır (Harita halihazır/parsel üretir → Yol ve Mimari kullanır; GIS imar/analiz üretir → Harita ve Yol kullanır). Bunların aynı `.kcad` v2 dosyası ve aynı `kentos-domain::Document` üzerinde kayıpsız çalışması, farklı yazılımlar arasında DXF/Shapefile/LandXML taşırken yaşanan veri ve semantik kaybı ortadan kaldırır.

### 1.3. Tespit Edilen Yapısal Darboğazlar

1. **Masaüstü Katmanında Monolitik Büyüme (`apps/desktop`):**
   - `apps/desktop/src/app.rs` tek başına **5.061 satır**dır. `Dialog` enum'ı **130'dan fazla pencereyi**, `Message` enum'ı **150'den fazla olay dalını**, `App` struct'ı ise dört disiplinin tüm pencere ve panel durumlarını (~200 alan) tek bir yapıda tutmaktadır.
   - `apps/desktop/src/` kök dizininde **650 adet `.rs` dosyası** alt klasörlere ayrılmadan düz (flat) biçimde durmaktadır.
2. **Komut ve Etkileşim Katmanlarında Yığılma (`crates/native/application` ve `interaction`):**
   - 301 bin satırlık `application` ve 142 bin satırlık `interaction` crate'leri dört disiplinin tamamını tek derleme biriminde (compilation unit) toplamaktadır.
3. **Derleme Süresi, Bellek Tüketimi ve Linker Yükü:**
   - Herhangi bir modül penceresindeki küçük bir arayüz değişikliği, 550 bin satırlık `kentos-desktop` crate'inin yeniden derlenip bağlanmasını gerektirmektedir.

---

## 2. Temel Mimari Karar: Ayrı Uygulama mı, Aynı Yazılım İçinde mi?

### Karar Özeti

> **Üretim ve dağıtım (Production/Release) katmanında şimdiki gibi "Tek Çekirdek Binary (`kentos-cad`) + Modül Kimlikli Bağlantılar (`kentos-mimari`, `kentos-harita`, `kentos-gis`, `kentos-yol`) + `kentos-launcher`" (Multi-Call Binary) mimarisiyle devam edilmelidir.**
>
> **Ancak kaynak kod ve derleme (Workspace/Crate) katmanında modüller Cargo `features` ve alt-crate'ler ile birbirinden ayrıştırılmalıdır.**

### Neden Tamamen Ayrı Derlenen 4 Bağımsız Uygulama (veya 4 Ayrı Repo) Olmamalı?

1. **%65–75 Ortak Makine Kodu ve Disk/RAM Maliyeti:**
   - ADR 0802 §1'de ölçüldüğü üzere, dört modül ayrı binary olarak derlendiğinde her biri ~255 MB yer kaplamakta (toplam >1 GB), semboller temizlendiğinde ise tek `kentos-cad` binary'si **~216 MB** olmaktadır.
   - Çizim motoru (`wgpu`), arayüz çatısı (`iced` + `kentos-ui`), yazı tipleri, geometri çekirdeği, `.kcad`/DXF/DWG kodekleri, pafta/PDF motoru, stil kütüphanesi ve bulut istemcisi dört uygulamada da ortaktır. Multi-call binary (sembolik/sabit bağlantı) yaklaşımında işletim sistemi çekirdeği aynı makinede hem `kentos-harita` hem `kentos-yol` çalıştığında binary'nin kod sayfalarını (text segment / page cache) RAM'de **tek kez** tutar.
2. **Sıfır Kayıplı Dosya Devri (`hand_over`) ve Ortak `.kcad` Evrimi:**
   - Kullanıcı yanlış modülde bir proje açtığında veya bir harita projesini yol/mimari modülüne devrettiğinde (`App::opened_in_program`), iki uygulamanın aynı `.kcad` şema sürümünü ve aynı koordinat/birim kurallarını konuşması şarttır. Ayrı kod tabanları kısa sürede sürüm uyumsuzluğuna yol açar.
3. **Kullanıcı Deneyiminde Ayrım Zaten Tamamlanmış Durumda:**
   - ADR 0801 ve ADR 0802 ile her modül kendi pencere başlığına, kendi `.desktop` ve `StartupWMClass` kimliğine, kendi yerleşim dosyasına (`yerlesim-mimari.json` vb.), kendi IPC soketine (`masaustu-mimari.sock` vb.), kendi şeridine, kendi komut arama kümesine (`App::offers`) ve kendi başlangıç/yeni proje akışına sahiptir. Kullanıcı açısından bunlar zaten dört ayrı uzmanlık programıdır.

---

## 3. Madde Madde Mimari ve Mühendislik Tavsiyeleri

Aşağıdaki maddeler, mevcut mimariyi bozmadan derleme hızını, kod bakımını, çalışma zamanı belleğini ve ürün kalitesini artırmak için öncelik sırasıyla düzenlenmiştir.

---

### ARG-01 — `apps/desktop` Dizin Yapısının Disiplinlere Göre Klasörlenmesi (Öncelik: P0)

- **Sorun:** `apps/desktop/src/` kök dizininde **650 adet `.rs` dosyası** düz bir listede duruyor (`arch_audit.rs`, `asbuilt_window.rs`, `corridor_window.rs`, `zoning_window.rs`, `parcel_register_window.rs` vb. yan yana). `plan/road.md` ROAD-MOD-08'de hedeflenen `apps/desktop/src/road/` ayrımı henüz yapılmamış.
- **Tavsiye:** Kök dizindeki dosyalar mantıksal modül klasörlerine taşınmalıdır:
  - `apps/desktop/src/shell/` — Kabuk, şerit (`ribbon*`), durum çubuğu, komut satırı, kısayollar, yerleşim, görünüm (`viewport`), katman ağacı ve ortak diyalog çerçeveleri.
  - `apps/desktop/src/arch/` — Mimariye özgü tüm pencereler, denetim (`arch_audit`), duvar/açıklık/mahal/metraj/ruhsat takımı/IFC yüzeyleri.
  - `apps/desktop/src/map/` — Harita CAD'e özgü pencereler (`map_work*`, `map_survey*`, `parcels*`, `readjustment*`, `zoning*`, `baselines*`, `ortho*`, `scan_network*`, `grade*`).
  - `apps/desktop/src/gis/` — CBS'ye özgü yüzeyler (`address/`, `networks/`, `temporal/`, `rasters/stac`, `data_profile*`, `subnetworks*`, tematik haritalama ve grafikler).
  - `apps/desktop/src/road/` — Yol modülüne özgü 25+ pencere ve görünüş (`terrain_window`, `alignment_window`, `profile_window`, `section_window`, `superelevation_window`, `corridor_window`, `volume_window`, `haul_window`, `guardrail_window`, `culvert_window`, `retaining_wall_window`, `road_*`).
- **Kabul Ölçütü:** `apps/desktop/src/` kökünde yalnız `lib.rs`, `main.rs`, `app.rs`, `program.rs` ve üst düzey modül bildirimleri kalır; tüm birim ve ekran testleri kırılmadan geçer.

---

### ARG-02 — `App` Struct, `Dialog` ve `Message` Enum'larının Modül Alt-Durumlarına (Sub-State) Bölünmesi (Öncelik: P0)

- **Sorun:** `apps/desktop/src/app.rs` içindeki `App` struct'ı dört disiplinin tüm pencere durumlarını aynı anda barındırıyor. Benzer şekilde `Dialog` ve `Message` enum'ları yüzlerce varyanttan oluşan dev birer merkezi dağıtıcıya (dispatcher) dönüşmüş durumda.
- **Tavsiye:**
  1. **Durum (State) Yalıtımı:** Modüle özgü pencere ve oturum durumları `App` içinde dört alt yapıda toplanmalıdır:
     ```rust
     pub struct App {
         // ... ortak kabuk, belge, viewport, katmanlar, bulut ...
         pub(crate) arch: crate::arch::ArchUiState,
         pub(crate) map: crate::map::MapUiState,
         pub(crate) gis: crate::gis::GisUiState,
         pub(crate) road: crate::road::RoadUiState,
     }
     ```
     Hatta program belirli bir modülle (`self.program == Some(Module::Architecture)`) başlatıldığında diğer modüllerin ağır UI state'leri hiç ilklendirilmeyebilir (`Default`/lazy).
  2. **Olay (Message) ve Pencere (Dialog) Yönlendirmesi:**
     - `Message::Terrains(...)`, `Message::Alignments(...)`, `Message::Profiles(...)`, `Message::Corridors(...)` gibi 30+ ayrı Yol mesajı tek bir `Message::Road(crate::road::Event)` dalında;
     - `Dialog::Terrains`, `Dialog::Alignments`, `Dialog::Corridors` gibi 25+ Yol diyaloğu tek bir `Dialog::Road(crate::road::RoadDialogKind)` dalında toplanmalıdır.
     - Aynı gruplama `Message::Arch(...)` / `Dialog::Arch(...)`, `Message::Map(...)` / `Dialog::Map(...)` ve `Message::Gis(...)` / `Dialog::Gis(...)` için yapılmalıdır.
- **Kazanç:** `app.rs` 5.000 satırdan ~1.200 satıra iner; yeni bir Yol veya Mimari penceresi eklemek `app.rs`'e dokunmayı gerektirmez, dallar (branch) arası merge çakışmalarını ortadan kaldırır.

---

### ARG-03 — "Üretimde Tek Binary, Geliştirmede Feature-Gated Modüller" Derleme Mimarisi (Öncelik: P1)

- **Sorun:** Geliştirme sırasında yalnız tek bir modül üzerinde çalışırken bile tüm modüllerin UI ve komut kodları derlenip tek test ikilisine bağlanıyor. Ayrıca ADR 0800 §7 ve ADR 0802 §1'de açık bırakılan *"her modülü yalnız kendi koduyla derleme"* hedefi henüz tamamlanmamış.
- **Tavsiye:**
  - `apps/desktop` (veya alt UI crate'leri) içinde `mod-arch`, `mod-map`, `mod-gis`, `mod-road` Cargo feature'ları tanımlanmalıdır (`default = ["mod-arch", "mod-map", "mod-gis", "mod-road"]`).
  - **Normal dağıtım (`cargo build --release`):** Tüm feature'lar açıktır; bugünkü gibi tek `kentos-cad` binary'si ve `module_links.py` ile 4 modül bağlantısı üretilir.
  - **Geliştirme koşusu:** Örneğin sadece Yol modülünü geliştirirken `cargo test -p kentos-desktop --no-default-features --features mod-road` ile derleme ve link süresi büyük oranda kısaltılır.
  - **Kurumsal özel paketleme:** Yalnızca belirli bir modülü talep eden kurumlara tek komutla kırpılmış ikili üretilebilir.

---

### ARG-04 — `crates/native/application` ve `crates/native/interaction` Katmanlarının Ayrıştırılması (Öncelik: P1)

- **Sorun:** `crates/native/application` **756 dosya / 301 bin satır**, `crates/native/interaction` ise **363 dosya / 142 bin satır** boyutuna ulaşmış durumda. Her iki crate de temel CAD çizim işlemleriyle Mimari, Harita, GIS ve Yol uzmanlık komutlarını aynı pakette derliyor.
- **Tavsiye:**
  - Bu iki katman ya kendi içlerinde disiplin alt-modüllerine (`src/core/`, `src/arch/`, `src/map/`, `src/gis/`, `src/road/`) ayrılmalı ya da doğrudan disiplin crate'lerine bölünmelidir:
    - `crates/native/application-core` (ortak CAD, katman, blok, stil, pafta, ayar komutları)
    - `crates/native/application-arch` (duvar, açıklık, mahal, mevzuat/yangın/otopark denetimi, ruhsat takımı)
    - `crates/native/application-map` (parsel, imar, 18. madde, GNSS/poligon/ağ, ortofoto/LiDAR)
    - `crates/native/application-gis` (kaynak bağlama, GeoPackage/PostGIS, topoloji, ağ, zaman/sensör)
    - `crates/native/application-road` (`road.*` ailesindeki ~100 komut)
  - `scripts/arch/deps.mjs` kuralları bu alt crate'lerin birbirine yatay bağımlılık kurmasını yasaklamalı, yalnız `application-core` ve `crates/shared/*` üzerine oturmalarını zorunlu kılmalıdır.

---

### ARG-05 — `kentos-geometry-core` İçinden Alana Özgü Geometrinin Ayrılması (Öncelik: P1)

- **Sorun:** `crates/shared/geometry-core` **491 dosya ve ~201 bin satır**a büyümüş durumda. İçinde genel amaçlı 2B/3B analitik geometri, Delaunay üçgenleme, boolean/clipping ve exact predicates yanında; mimari duvar/kolon birleşimleri (`column_join`), doğrama/merdiven/rampa parçaları, imar planı yol kavşakları gibi alana özgü kurallar da bulunuyor.
- **Tavsiye:**
  - Yol mühendisliği için `crates/shared/road` ve IFC için `crates/shared/ifc` nasıl ayrı ve temiz birer `shared` crate olarak konumlandırıldıysa, mimariye özgü saf geometri ve yapı fiziği hesapları da **`crates/shared/arch`** (`kentos-arch`) adlı ayrı bir platform-bağımsız crate'e taşınmalıdır.
  - Böylece `kentos-geometry-core` yalnızca temel matematik, 2B/3B eğri/yüzey primitifleri, uzamsal indeks (R-tree/BVH), Delaunay ve exact predicates içeren, çok hızlı derlenen çekirdek bir kütüphane olarak kalır.

---

### ARG-06 — `.kcad` v2 Şema Sürümlemesinde Modüler Uzantı (Domain Extension) Yapısına Geçiş (Öncelik: P1)

- **Sorun:** `.kcad` v2 dosya biçiminde her yeni özellik eklendiğinde tek bir doğrusal şema numarası (`SCHEMA_VERSION`, şu an **236+**) artırılıyor. Dört farklı disiplin paralel geliştikçe:
  1. Farklı geliştirme dallarında (branch) aynı şema numarasının çakışması,
  2. Sadece Mimari çizimi içeren bir dosyanın, Yol veya GIS şemasındaki değişikliklerle sürüm numarasının jumplaması,
  3. Her şema artışında `fixtures/kcad/v2/` ve bağımsız Python doğrulayıcısında (`tools/kcad/kcad.py`) yüzlerce durumun güncellenmesi maliyeti doğuyor.
- **Tavsiye:**
  - `.kcad` v2 başlığında **Çekirdek Şema (`core_schema`)** ile **Modül Uzantı Sürümleri (`extensions: { arch?: u16, map?: u16, gis?: u16, road?: u16 }`)** ayrıştırılmalıdır.
  - Bir modül programı (ör. `kentos-mimari`), başka bir modüle ait bilinmeyen ama imzası/bölümü korunan uzantı bloklarını (preserved CBOR chunks) düşürmeden koruyabilmeli; yalnız kendi alanındaki şema sürümü desteklenmiyorsa reddetmelidir.

---

### ARG-07 — Yol ve Harita İçin Çoklu Görünüm (Multi-Viewport) ve 3B Sahne Entegrasyonu (Öncelik: P1)

- **Sorun:** `plan/road.md` §2a (`ROAD-MOD-05`) ve `[3d.md]` kapsamında; Yol modülünde şu an boykesit, enkesit, dever ve kübaj pencereleri `RoadDialog` içinde `widget::picture::Picture` ile çizilen ayrı diyaloglar olarak çalışıyor. OpenRoads Designer ve Netpro iş akışlarında ise mühendis **Plan + Boykesit + Enkesit + 3B Perspektif** görünüşlerini aynı anda ekranda açık tutup birinde yaptığı değişikliği (ör. düşey somenin sürüklenmesi veya ripaj) diğer üç görünüşte anlık izlemek ister.
- **Tavsiye:**
  - `apps/desktop/src/split_view.rs` (ADR 0523) ile başlayan bölünmüş görünüm altyapısı, yalnız iki harita görünümü karşılaştırmakla kalmayıp **farklı sahne türlerini (Plan Tuvali + Boykesit Tuvali + Enkesit Tuvali + `kentos-scene3d` 3B Tuvali)** aynı pencerede 2×2 veya 1+2 düzende gösterecek şekilde genişletilmelidir.
  - Ortak bir **İstasyon/Km İmleci (`StationCursor`)**, fare plan üzerinde güzergâhta gezerken boykesitteki düşey çizgiyi ve enkesit penceresindeki aktif kesiti GPU üzerinde sıfır gecikmeyle eşzamanlı hareket ettirmelidir.

---

### ARG-08 — Büyük Veri (Yol TIN / Nokta Bulutu / CBS Katmanları) ile `Document` Bellek Sınırının Korunması (Öncelik: P1)

- **Sorun:** Mimari CAD çizimleri birkaç bin nesne içerirken; Yol ve Harita projelerindeki LiDAR/COPC nokta bulutları (on milyonlarca nokta), geniş alan TIN yüzeyleri (milyonlarca üçgen) ve CBS'deki il/ülke ölçekli GeoPackage/PostGIS katmanları doğrudan `kentos-domain::Document` içindeki standart `Entity` yuvalarına (`SlotMap`) kopyalanırsa RAM ve `undo/redo` snapshot maliyeti patlar.
- **Tavsiye:**
  - `Document` içinde yalnızca mühendislik kaynağının **kimliği, üstverisi (CRS, düşey datum, özet/digest, sınır zarfı) ve parametrik tanımı** tutulmalı; ağır nokta/üçgen/raster yığınları belge dışı akışlı (streaming / memory-mapped / chunked) veri depolarında (`kentos-pointcloud`, `kentos-geopackage`, `kentos-road` yüzey önbelleği) tutulmaya devam edilmelidir.
  - Geri alma (`undo/redo`) adımları milyonlarca üçgeni kopyalamak yerine, kaynağın değişmez revizyon kimliğine (immutable revision/snapshot ID) işaret etmelidir.

---

### ARG-09 — Türkiye Mevzuat ve Standart Kurallarının Veri Tabanlı Profillere (Rule Packs) Dönüştürülmesi (Öncelik: P2)

- **Sorun:** KentOS'un en güçlü rekabet avantajlarından biri olan Türkiye mevzuat denetimleri (PAİY, BYKHY Yangın, TBDY 2018, Otopark/Sığınak Yönetmeliği, 2026 Dijital Bina Yönetmeliği EK-3/EK-4, Sivas/Yerel İmar Yönetmelikleri, MPYY, KGM/KTEK 2022) şu an büyük ölçüde Rust kodu içinde (`regulation_audit.rs`, `fire_audit.rs`, `provisions_audit.rs` vb.) sabit eşiklerle tanımlanıyor.
- **Tavsiye:**
  - Denetim motorunun **geometrik ölçüm ve graf yürüme algoritmaları** (ör. `kentos_domain::escape` kaçış grafiği, `paiy_areas` net/brüt alan hesabı, görüş mesafesi analizi) saf Rust kodunda kalmalı; ancak **eşik değerleri, madde numaraları, yürürlük tarihleri ve yerel belediye farkları** sürümlü, imzalı ve şeması doğrulanmış (`schemars`) JSON/TOML **Mevzuat Profilleri (`catalog/standards/*.json`)** olarak dışsallaştırılmalıdır.
  - Böylece Resmî Gazete'de bir yönetmelik değişikliği olduğunda veya yeni bir büyükşehir belediyesinin imar yönetmeliği eklendiğinde, hesap motorunu yeniden yazmadan yeni standart profili eklenebilir ve eski projeler kendi ruhsat tarihindeki yönetmelik sürümüyle yeniden denetlenebilir.

---

### ARG-10 — Modüller Arası Canlı Altlık ve Referans (Cross-Module Xref & Source Link) Akışının Standartlaştırılması (Öncelik: P2)

- **Sorun:** Dört uygulama (`kentos-mimari`, `kentos-harita`, `kentos-gis`, `kentos-yol`) sıkı biçimde ayrıldığında (ADR 0802), disiplinler arası işbirliği genellikle bir modülde kaydedilen dosyanın diğerinde Dış Başvuru (`xref`) veya Kaynak (`sources`) olarak bağlanmasıyla yürür.
- **Tavsiye:**
  - **Disiplinlerarası Sözleşme:**
    1. *Harita → Mimari:* `kentos-harita`'da onaylanan aplikasyon krokisi, parsel sınırı ve ±0,00 kot noktası, `kentos-mimari`'de "Harita Altlığı Bağla" ile salt-okunur ve koordinat/datum kilidi korunarak vaziyet planına oturmalıdır.
    2. *Harita → Yol:* `kentos-harita`'da üretilen onaylı halihazır TIN yüzeyi ve imar planı adaları, `kentos-yol`'a doğrudan onaylı arazi (`Approved Terrain`) ve kamulaştırma/yasak alan sınırı olarak bağlanmalıdır.
    3. *Mimari + Yol + Harita → GIS / 3B:* Üç CAD modülünün ürettiği yapı ayak izleri, IFC 4.3 modelleri ve yol koridorları, `kentos-gis` (ve gelecekteki `kentos-3d`) içinde doğrudan katman/kaynak olarak sorgulanabilmelidir.
  - Kaynak dosyada yeni bir revizyon oluştuğunda, bağlı modüldeki `xref_watch` / `RoadProject` / `MapWork` ağacı hangi türetilmiş hesapların (ör. kazı/dolgu kübajı veya vaziyet yaklaşma mesafesi) eskidiğini (`stale`) açıkça göstermelidir.

---

### ARG-11 — Test ve Doğrulama Süreçlerinin Hızlandırılması (`nextest`, Ortak Durum Şardlama) (Öncelik: P2)

- **Sorun:** Kod tabanında binlerce birim testi, yüzlerce ortak durum (`fixtures/*/v1`) ve ağır masaüstü ekran testleri (`#[ignore]` `screens`) bulunuyor. `scripts/dev/svc.sh heavy` kilidi masaüstünün donmasını engellese de tam test koşuları ve dev test ikililerinin bağlanması (linking) uzun sürüyor.
- **Tavsiye:**
  1. `apps/desktop` içindeki yüzlerce `*_scenes.rs` ve `*_tests.rs` dosyası, `ARG-01` ve `ARG-03` ile modül feature'larının arkasına alınmalıdır; böylece Mimari testi koşarken Yol sahneleri derlenmez.
  2. Linux geliştirme ortamında `lld` veya `mold` linker kullanımının `.cargo/config.toml` içinde standartlaştırılması, ~550 bin satırlık masaüstü test ikilisinin link süresini saniyelere indirir.
  3. Bağımsız Python referans betikleri (`scripts/fixtures/*_cases.py --check`) için değişen dosyaya göre seçici koşu (incremental check) hedefleri `Makefile`'a eklenmelidir.

---

### ARG-12 — Eklenti (`.kek`), Python SDK ve MCP Yüzeyinin Modül Sınırlarıyla Uyumlu Tutulması (Öncelik: P2)

- **Sorun:** Masaüstü arayüzünde modüller sıkı biçimde ayrılmışken (`App::offers`), başsız sunucu (`kentos-headless`), Python SDK (`kentos.cad`) ve MCP (`kentos-mcp`) tüm kataloğu sunmaya devam ediyor (ADR 0802 §4). Ancak masaüstündeki açık çizime bağlanan yerel Python konsolu ve MCP `desktop.attach` (`masaustu-<modül>.sock`) oturumlarında modül sınırı önem taşıyor.
- **Tavsiye:**
  - Başsız (`headless`) modda bir `.kcad` dosyası üzerinde otomasyon çalışırken tüm komutların açık olması doğrudur (bir betik aynı anda hem harita hem yol hem mimari raporu üretebilir).
  - Ancak `masaustu-mimari.sock` üzerinden canlı bir `kentos-mimari` penceresine bağlanan ajan veya Python konsolu, o modülün sunmadığı bir komutu çalıştırmaya kalktığında masaüstüyle aynı `not_offered` kuralına tabi olmalı; Python SDK içinde de komutlar `kentos.cad`, `kentos.arch`, `kentos.map`, `kentos.gis`, `kentos.road` alt ad alanlarıyla (mevcut `kentos.cad` geriye dönük uyumluluğu korunarak) belgelenmelidir.

---

## 4. Önerilen Uygulama Sırası (Özet Yol Haritası)

| Aşama | Maddeler | Hedeflenen Çıktı | Risk |
|---|---|---|---|
| **1. Aşama (İç Temizlik)** | `ARG-01`, `ARG-02` | `apps/desktop/src/` altının `shell/`, `arch/`, `map/`, `gis/`, `road/` klasörlerine ayrılması; `App`, `Dialog` ve `Message` yapılarının modül alt-durumlarına bölünmesi | Düşük (davranış değişmez, yalnız iç organizasyon) |
| **2. Aşama (Derleme Hızı)** | `ARG-03`, `ARG-04`, `ARG-05`, `ARG-11` | Modüllerin Cargo feature'ları ile izole derlenebilmesi, `kentos-arch` çekirdeğinin ayrılması, geliştirme derleme/test süresinin 5–10 kat hızlanması | Orta (`scripts/arch/deps.mjs` güncellenir) |
| **3. Aşama (Veri & Sahne)** | `ARG-06`, `ARG-07`, `ARG-08` | `.kcad` modüler uzantı şeması, Yol/Harita için 4'lü bölünmüş canlı sahne görünümü ve büyük veri bellek sınırının tahkimi | Orta–Yüksek (kapsamlı fixture doğrulaması ister) |
| **4. Aşama (Ekosistem)** | `ARG-09`, `ARG-10`, `ARG-12` | Veri tabanlı mevzuat profilleri, disiplinlerarası canlı altlık sözleşmesi ve Python/MCP ad alanı düzeni | Düşük–Orta |
