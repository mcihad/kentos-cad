# IFC görüntüleyici ve okuma motoru — araştırma notu

*KentOS CAD · `kentos-ifc` ve masaüstü için · Kaynaklara erişim: 15 Kasım 2026 · Plan dalı `0ea04d0f6`; 3B sahne,
pencere ve modül olguları `main` `8dba344ba`*

Bu not [ADR 8061](../adr/8061-ifc-viewer-and-reading-engine.md)'in ve [plan/ifc-uyum.md](../../plan/ifc-uyum.md)'deki
IFC-07, IFC-09 ve IFC-12'nin dayanağıdır. Yönetmelik tarafı [öbür notta](2026-11-15-bim-ifc-yonetmelik.md) durur.

## 0. Soru, yöntem ve işaretler

Sahibin sorusu: KentOS'ta bir IFC görüntüleyici olsun; Rust ile yazılmış IFClite'ın lisansı uygunsa doğrudan
kullanılabilir mi; görüntüleyici ayrı bir uygulama mı olmalı, uygulamanın içine mi girmeli?

- ✔ kaynağın kendisi bu oturumda okundu · ◐ arama özetinden ya da ikincil kaynaktan · ✖ denenmedi ya da doğrulanamadı.
- Depo ve paket sayıları GitHub'ın ve crates.io'nun açık arayüzünden, kimliksiz istekle okundu.
- **Hiçbir dosya indirilmedi, hiçbir paket derlenmedi, hiçbir görüntüleyici çalıştırılmadı.** Aşağıdaki hız ve kapsam
  sayıları üreticilerin beyanıdır; KentOS ölçümü değildir (kılavuz §6.1).

## 1. IFClite

| Konu | Bulgu | Kaynak | |
|---|---|---|---|
| Lisans | MPL-2.0; çalışma alanındaki bütün Rust paketleri aynı lisansı bildirir | [depo](https://github.com/LTplus-AG/ifc-lite) (lisans alanı, kök `Cargo.toml`, `README.md`) | ✔ |
| Depo | `LTplus-AG/ifc-lite` (eski adres `louistrue/ifc-lite` buraya yönleniyor). 10 Ocak 2026'da açılmış; 473 yıldız, 128 çatal; son gönderim 15 Kasım 2026 | GitHub arayüzü | ✔ |
| Ne sunuyor | Tarayıcıda WebGPU görüntüleyici, TypeScript paketleri, komut satırı aracı, MCP sunucusu ve sunucu. Çekirdek Rust'tır ve tarayıcı için WASM'a derlenir | depo açıklaması; [ifclite.dev](https://ifclite.dev/) | ✔ |
| Görüntüleyicinin kendisi | Arayüz ve çizici TypeScript'tir (tarayıcı uygulaması). Dosya tarayıcı sekmesinde, yerelde işlenir | [ifclite.com](https://www.ifclite.com/); `apps/viewer` | ✔ |
| Masaüstü | **Hazır bir masaüstü uygulaması yoktur.** Belge, paketlerin bir Tauri kabuğuna nasıl sarılacağını ve geometrinin WASM yerine yerel Rust'ta nasıl çalıştırılacağını anlatır; yerel yol aynı `ifc-lite-core` ve `ifc-lite-geometry` paketlerini kullanır | [Building for Desktop](https://ifclite.dev/docs/guide/desktop/) | ✔ |
| Rust paketleri | crates.io'da 28.2.0 (2 Kasım 2026): `ifc-lite-core` (STEP ayrıştırıcı, şema, tembel çözücü), `ifc-lite-geometry` (üçgenleme, boole), `ifc-lite-processing` (sunucunun ve yerel kabuğun ortak üretim hattı), ayrıca `clash`, `export`, `ffi`, `landxml`, `wasm` | [Rust API](https://ifclite.dev/docs/api/rust/); crates.io | ✔ |
| Sürüm hızı | `ifc-lite-core`: 12 Ocak – 2 Kasım 2026 arasında 103 sürüm ve 24 ana sürüm; 19 ana sürüm 21 Ağustos'tan sonra. Belgeye göre kıran değişiklik ana sürümü artırır; ağ ve dışa aktarım biçimleri küçük sürümler arasında bayt bayt kararlı sayılmaz | crates.io sürüm listesi; [Error Handling & Integration](https://ifclite.dev/docs/guide/error-handling/) | ✔ |
| Derleyici | Depo `nightly-2025-11-15`'i sabitler; README bunu WASM paketinin derlenmesine bağlar. `core` ve `geometry`'nin kökünde `#![feature]` yok; crates.io kaydında en düşük Rust sürümü bildirilmemiş | `rust-toolchain.toml`, `README.md`, iki `lib.rs`, crates.io | ✔ (kararlı 1.100 ile derlenmedi ✖) |
| Bağımlılıklar | `core`: nom, fast-float2, memchr, rustc-hash, twox-hash, bytemuck, thiserror, futures-core, futures-util. `geometry`: earcutr, i_overlay, nalgebra, rayon, serde, png, jpeg-decoder, smallvec, geometry-predicates, num-rational, num-traits, bnum. `processing`: bunlara ek serde_json, tracing, sha2 ve Unix'te libc. C ya da C++ kodu yok | üç `Cargo.toml` | ✔ |
| KentOS'un kilit dosyasında bulunmayanlar | nalgebra, rayon, earcutr, i_overlay, num-rational, bnum, jpeg-decoder, fast-float2, twox-hash, geometry-predicates: 10 doğrudan paket (geçişlileri sayılmadı) | `main`'in `Cargo.lock`'u | ✔ |
| Geometri hattı | Tek Rust hattı (`produce_element_meshes`, `processing` paketi) tarayıcıda da yerelde de aynı ağı üretir. İşleyiciler: öteleme, yüzey gövdesi, boole işlemi, eşlenmiş gövde. Boole işlemleri paketin içindeki saf Rust, kesin yüklemli çekirdekte yapılır; eski C++ çekirdeği kaldırılmış | [Geometry Processing](https://ifclite.dev/docs/guide/geometry/); `geometry/Cargo.toml` | ✔ |
| Ağın biçimi | Nesne başına STEP kimliği, sınıf, float32 konum ve float64 yerel orijin, normal, indis ve renk; model için orijin kaydırma ve büyük koordinat bilgisi | Geometry Processing; Rust API (`Mesh`) | ✔ |
| Üçgen yönü | Üçgenlerin dışa bakması garanti edilmez; belge çizicinin iki yüzü de çizmesini ister | Geometry Processing | ✔ |
| Üçgenleme sıklığı | Ayar yalnız WASM yollarında çalışır; yerel hat düzeyi henüz kullanmaz | Building for Desktop; Geometry Processing | ✔ |
| Hata ve sınırlar | Rust paketleri tipli `Result` döner. Belge, STEP ayrıştırıcısının bozuk girdide panik yapmadığını, asılmadığını ve yığını taşırmadığını; bunun bozuk girdi testleri ve bir fuzz hedefiyle sınandığını söyler. Gösterim zincirinde iş bütçesi (65 536 düzey) ve ret bildirimi; eleman başına bütçe ve bekçi vardır | Error Handling & Integration; Rust API; [Performance](https://ifclite.dev/docs/guide/performance/) | ✔ (kendi sınamamız yok ✖) |
| Belirlenimcilik | Kesin yüklem ve boole çekirdeği x86_64, aarch64 ve wasm32'de bayt bayt aynıdır. Gerisi (üçgenleme sıklığı, renk sırası, kesin olmayan yollardaki yuvarlama) yalnız aynı mimaride yinelenir | Error Handling & Integration | ✔ |
| Şema kapsamı | IFC2X3 653, IFC4 776, IFC4X3 876 sınıf; IFC5 (IFCX) önizleme | ifclite.dev | ✔ (üreticinin beyanı) |
| Hız | Tarayıcı görüntüleyicisinde toplam yükleme: 2,4 MB ve 317 ağ 3,3 s; 8,3 MB ve 17 380 ağ 3,7 s (CI makinesi, yazılım çizici). 169 MB 11,0 s; 327 MB 11,9 s (daha hızlı yerel donanım). Üretici iki grubun karşılaştırılamayacağını yazar | Performance | ✔ (üreticinin ölçümü) |
| İkinci çekirdek | İsteğe bağlı "retained" yol NVIDIA'nın Apache-2.0 lisanslı SMLib'ini kullanır; varsayılan derlemede yoktur | `licenses/retained/README.md`; `geometry/Cargo.toml` özellikleri | ✔ |
| WASM | Çekirdeğin asıl hedefi tarayıcıdır: ayrıştırıcı, geometri ve dışa aktarıcılar tek WASM modülü olarak dağıtılır (yaklaşık 7,9 MB, ağda yaklaşık 2,6 MB). Tarayıcıda tek iş parçacıklı WASM işçileri havuzu, yerelde süreç içi Rayon havuzu çalışır; wasm32'de örnek başına 4 GB adres alanı sınırı vardır | Performance; Building for Desktop | ✔ |

## 2. MPL-2.0 ne ister

Kaynak: [Mozilla, MPL 2.0 FAQ](https://www.mozilla.org/en-US/MPL/2.0/FAQ/) (✔, S8–S12). Hukuki görüş değildir; lisans
metninin üreticisinin kendi açıklamasıdır.

- **Değiştirilmemiş MPL kodundan derlenen programı dağıtmak (S8):** alıcıya MPL'li kodun kaynağını nereden alacağı
  söylenir (lisansın 3.2 maddesi). Programın kendisi, alıcının o kaynağa ilişkin haklarını kısıtlamayan başka bir
  lisansla dağıtılabilir.
- **Değiştirilmiş MPL kodunu dağıtmak (S9, S10):** değişiklikler MPL ile sunulur; alıcıya kaynağın nasıl alınacağı
  söylenir; kaynak dosyalardaki bildirimler kaldırılmaz.
- **Kapalı kaynak ürünün içinde kullanmak (S11):** MPL'li kod içermeyen yeni dosyalar "değişiklik" sayılmaz ve MPL ile
  dağıtılmak zorunda değildir; MPL'li kod kapalı bir ürüne statik bağlanabilir.
- **Kapsam (S12):** MPL'in paylaşma yükümlülüğü dosya düzeyindedir; LGPL'de kitaplık, GPL'de bütün yazılım düzeyindedir.

KentOS için sonucu: IFClite'ın paketleri değiştirilmeden bağlanırsa iki yükümlülük doğar (lisans bildirimi ve kaynağın
adresi). Paketlerin dosyaları değiştirilirse o dosyalar MPL ile yayımlanır.

## 3. Öbür motorlar

| Motor | Lisans | Dil | KentOS için anlamı | |
|---|---|---|---|---|
| IfcOpenShell | LGPL-3.0 | C++ (OpenCASCADE ile) | Bugünkü bağımsız başvurumuz (`scripts/fixtures/arch_ifc_check.py`). Ürüne bağlamak C++ ve OpenCASCADE zinciri getirir; başvuru olarak kalır | ✔ lisans |
| web-ifc (That Open) | MPL-2.0 | C++, WASM'a derlenir | Tarayıcı içindir; masaüstünde C++ köprüsü ister | ✔ lisans |
| xeokit | AGPL-3.0 (ticari lisans ayrıca) | JavaScript | Lisansı kapalı kaynak ürüne uymaz | ✔ lisans |
| OpenCASCADE | LGPL-2.1 | C++ | Genel katı modelleme çekirdeği; plan K7 gereği şimdi alınmıyor | ✔ lisans |
| `ifc_rs` | okunmadı | Rust | Son sürümü 0.1.0-alpha.9, 4 Aralık 2024 | ✔ kayıt |
| `openbim-ifc` ailesi (`ifc-model`, `ifc-schema`, `ifc-step`, `ifc-geometry`, `ifc-properties`, `ifc-alignment`) | okunmadı | Rust | Kasım 2026'da güncellenmiş, paket başına 2–4 bin indirme; kapsamı ve lisansı bu turda okunmadı | ✔ kayıt, ✖ içerik |
| `oxideav-ifc` | okunmadı | Rust | 0.0.4, 7 Kasım 2026; yalnız STEP okuyucu | ✔ kayıt |

Kaynak: GitHub'ın lisans alanı ve crates.io araması (15 Kasım 2026).

## 4. Benzer görüntüleyicilerde iş akışı

| Ürün | Belgesinden okunan | Kaynak | |
|---|---|---|---|
| Open IFC Viewer (ODA) | Ücretsiz ve tümüyle yerel; Windows, macOS, Linux. IFC 2x3, 4, 4x2, 4x3. 3B gezinme ve görsel stiller; seçim ve özellik inceleme, gizleme ve yalnız bırakma; kesit düzlemleri; noktaya yakalayan uzaklık ölçümü; doğrulama (IDS dahil); çakışma ve BCF; 3B PDF; açık modele bağlanan MCP sunucusu. Yol haritasında birleşik modeller, katman yöneticisi, ağaçta öznitelikle arama, mahal ve açıklığın isteğe bağlı gösterimi | [openifcviewer.com](https://openifcviewer.com/) | ✔ |
| BIMvision | Ücretsiz. Model yapısı "IFC Structure" penceresinde tablo olarak; kat seçilince modelde vurgulanır, işareti kaldırılınca gösterimden çıkar. Katları ayıran kaydırma, elemanların doğru kata bağlandığını denetlemek için kullanılır. 2B izdüşüm. Eğrilerin yaklaşım sıklığı ayarlanır; üretici bunun ölçüm değerlerini yüzdenin kesirleri kadar değiştirebildiğini yazar | [bimvision.eu](https://bimvision.eu/products-and-functionalities/visualization-of-ifc-models-in-the-ifc-file-browser-bimvision/) | ✔ |
| BIMcollab Zoom | Kesit düzlemi yüzeye tıklanarak konur, ekseni boyunca sürüklenir, ok tuşlarıyla kaydırılır; kesit kutusu seçimin çevresine altı düzlem kurar; düzlemleri gösterme, askıya alma ve silme kısayolları vardır | [yardım merkezi](https://helpcenter.bimcollab.com/en/articles/334321-creating-and-managing-section-planes-and-boxes-in-bimcollab-zoom) | ✔ |
| BIMcollab Zoom | Model ağacı ve tür ağacı; ağaçta seçilen modelde seçilir; özellik paneli (özet, konum, özellik setleri); özelliğe göre süzen ve boyayan kayıtlı görünümler | yardım merkezi (arama özeti) | ◐ |
| Solibri | Eski ücretsiz görüntüleyici (Anywhere) "eski ürünler" sayfasındadır. Güncel kademeler görsel incelemeden kural tabanlı denetime doğru sıralanır | [solibri.com](https://www.solibri.com/products/legacy-products) | ✔ |
| IFClite görüntüleyicisi | Dosya sürükle bırak ile açılır; hiyerarşi, özellikler, 2B çizim, kesit düzlemleri; IFC yanında glTF, nokta bulutu, LandXML ve DXF altlık | [ifclite.com](https://www.ifclite.com/); ifclite.dev | ✔ |

Ürünlerin ortak akışı (KentOS görüntüleyicisinin ilk kapsamı buradan çıkar):

1. Dosyayı aç: seç ya da pencereye bırak.
2. Yapı ağacı (saha, bina, kat, nesne) ve tür ağacı; ağaçta seçilen modelde seçilir.
3. Modelde tıkla: nesnenin özeti, konumu, özellik ve miktar setleri.
4. Gizle, yalnız bırak, saydam yap, hepsini göster.
5. Kesit düzlemi ve seçimin çevresinde kesit kutusu.
6. Katları ayır ya da tek katı göster.
7. Uzaklık ölçümü (üçgen ağ üstünde, yaklaşık).
8. İleri adımlar: özelliğe göre boyama, birden çok modeli birlikte açma, doğrulama, çakışma ve sorun değişimi.

## 5. KentOS'ta hazır parçalar

| Parça | Yer | Görüntüleyici için anlamı |
|---|---|---|
| 3B sahne çekirdeği | [`crates/shared/scene3d`](../../crates/shared/scene3d/src/lib.rs) | float64 kamera, karo hiyerarşisi, ekran hatasına göre seçim, büyük ağın karolara bölünmesi (`SourceMesh`: parça, float64 köşeler, üçgenler), kaynak üçgene inen seçme |
| 3B çizici | [`crates/render/wgpu/src/scene3d`](../../crates/render/wgpu/src/scene3d/mod.rs) | Derinlik tamponu, saydam geçiş, bellek bütçesi; yüz ayıklaması kapalı, yani iki yüz de çizilir (`main`'de `cull_mode: None`) |
| 3B görünüm penceresi | [`scene3d_window.rs`](../../apps/desktop/src/shell/scene3d_window.rs) | Bugün Yol'un arazisini ve koridorunu gösterir: yörünge, kaydırma, yaklaşma, hazır bakışlar |
| Ayrı işletim sistemi penceresi | `apps/desktop/src/shell/windows/` (`main`'de, ADR 8060) | Bir yüzeyin ikinci ekrana alınması; tek uygulama durumu |
| Modüller ve Launcher | [`crates/native/launch`](../../crates/native/launch/src/module.rs), ADR [0800](../adr/0800-kentos-launcher.md), [0801](../adr/0801-kentos-modules-and-their-programs.md), [0802](../adr/0802-strict-module-separation.md) | Tek program dosyası adına göre Mimari, Harita, GIS ya da Yol olur; her biri ayrı uygulama |
| IFC çekirdeği | [`crates/shared/ifc`](../../crates/shared/ifc/src/lib.rs) | Yazıcı, adlar, koordinat sistemi, kontrol formu; okuyucu yok |
| Bağımsız başvuru | [`arch_ifc_check.py`](../../scripts/fixtures/arch_ifc_check.py) | IfcOpenShell ile şema denetimi; ürünle dağıtılmaz |
| Bağımlılık yönü denetimi | [`scripts/arch/deps.mjs`](../../scripts/arch/deps.mjs) (ADR 0010) | `crates/shared/` altındaki her paket yerel hedef ve wasm32 için denetlenir; Iced, wgpu, tarayıcı bağlamaları ve sunucu çalışma zamanları yasaktır. Yeni bir paket bir gruba bilerek yerleştirilmeden denetim geçmez |
| Üç paketli örnek | `crates/shared/sheet`, `crates/sheet-ui`, `crates/wasm/sheet-wasm` | Pafta: saf çekirdek, masaüstü arayüzü ve web bağlaması ayrı paketlerde. Görüntüleyici aynı düzeni izleyebilir |
| 3B çizicinin bağımsızlığı | `crates/render/wgpu/src/scene3d/` | İki dosyasında 2B çizim hattına başvuru yok (`crate::` sayısı 0); gerekirse kendi paketine taşınabilir |
| Ortak 3B gölgelendirici | [`shaders/wgsl/scene3d/mesh.wgsl`](../../shaders/wgsl/scene3d/mesh.wgsl), `shaders/wgsl/scene3d.layout.json` | Masaüstünün wgpu çizicisi ile web'in WebGPU arka ucunun (`apps/web/src/render/webgpu`) ortak sözleşmesi |
| WASM paketleri | `crates/wasm/` (dokuz paket) | IFC çekirdeği ve 3B sahne için henüz web bağlaması yok; ikisi de bugün yalnız yerelde kullanılıyor |
| Disiplin bayrakları | `crates/native/application/Cargo.toml` (ADR 8054 §11) | `mod-arch` ve `mod-road` `kentos-ifc`'i bağlar; masaüstü bayraklanmaz (sahibin 13 Kasım kararı) |

## 6. Çıkarımlar

- IFClite'ın **uygulaması** KentOS'a uymaz: hazır masaüstü uygulaması yok, arayüzü web teknolojisi, KentOS web'e ara
  verdi (ADR 0228). Türkçe arayüz ve başka uygulamanın anılmaması için bütün görüntüleyiciyi çatallamak gerekirdi.
- IFClite'ın **Rust çekirdeği** KentOS'un eksiğini karşılar: dosyayı okumak ve gövdeleri üçgene çevirmek. Çizim, pencere
  ve modül düzeni KentOS'ta hazırdır.
- Lisans kapalı kaynak ürüne engel değildir; yükümlülük bildirim ve, dosya değiştirilirse, o dosyaların yayımıdır.
- En büyük bakım riski sürüm hızıdır; motorun türleri tek pakette (`kentos-ifc`) sarılmalı ve sürüm sabitlenmelidir.
- Ücretsiz görüntüleyicilerin hepsi ayrı programdır; KentOS'un modül düzeni aynı koddan hem ayrı programı hem Mimari'nin
  penceresini verebilir.
- **Sahibin kararı (15 Kasım 2026):** görüntüleyici uygulamanın içinde gelir ama gevşek bağlıdır; gerektiğinde hızlıca
  ayrı bir uygulama olabilmeli ya da WASM ile web'de çalışabilmelidir. Ayrı program ve web şimdi yapılmaz; açık tutulur.
- Bu kararı ucuz kılan üç olgu: motorun asıl hedefi zaten WASM'dır (§1); `crates/shared/` altına konan paket yerel ve
  wasm32 için kendiliğinden denetlenir (§5); 3B çizicinin 2B hatta bağı yoktur ve gölgelendiricisi web ile ortaktır
  (§5).
- Web'de sınır bellektir: wasm32'nin 4 GB adres alanı büyük modelleri masaüstüne ya da sunucuya bırakır (§1).

## 7. Okunmayanlar ve açık noktalar

- IFClite paketleri KentOS'un derleyicisiyle derlenmedi; süre, bellek, ikili boyutu ve doğruluk ölçülmedi (ADR 8061 §6).
- `ifc-lite-processing`'in yerel çağrı yüzü docs.rs'ten okunmadı; denemede okunur.
- Rust kayıt defterindeki öbür IFC paketlerinin kapsamı ve lisansı okunmadı.
- Görüntüleyiciler çalıştırılıp resimleri alınmadı; akış üreticilerin belgelerinden yazıldı. Uygulama diliminde gerçek
  ekranlarla yeniden bakılır.
- buildingSMART'ın belgeleme modelleri deposunda (`IFC4.x-specification-models`) lisans bildirimi görünmüyor; örnekler
  depoya kopyalanmadan önce kullanım koşulu okunur.
