# KentOS — Modül ve istemci ayrımı, saklama modeli planı

Başlangıç: **23 Kasım 2026**. Sahibin isteği (özetle):

- Paneller, alt paneller ve menüler her programda her şeyi gösteriyor: CBS'de CAD denetimi, katman sağ tık
  menüsünde ilgisiz öğeler, bir türde proje açıldıktan sonra Yeni proje'de bütün menüler. Net bir ayrım isteniyor.
- Ekranı açık projenin türü belirlesin. Modüller ileride ayrı yazılımlara dönüştürülebilecek kadar gevşek bağlı
  olsun.
- İleride mobil görüntüleyiciler, düzenleyiciler ve başka modüller gelecek. Ayrım netleşince sistem bir daha
  bozulmasın diye kesin kurallar CLAUDE.md'ye yazılsın (§25).
- **CBS projesi:** verisi ve öznitelikleri veritabanında. Proje dosyası yapıyı, stilleri, ayarları ve açıklamaları
  tutar; dosyaya, veritabanına ya da buluta kaydedilir.
- **CAD projesi:** tamamı proje dosyasında; dosyaya ya da buluta (yalnız dosya revizyonu) kaydedilir.
- Uygulama geliştirmede; eski projelere taşıma yolu yazılmaz.

Kararlar: [ADR 9195](../docs/adr/9195-module-separation.md) (ayrım),
[ADR 9196](../docs/adr/9196-storage-by-project-kind.md) (saklama). Kesin kurallar CLAUDE.md §25'te, yeni modül ve
istemci tarifi kılavuz §9.9'dadır.

Her madde tek dilimde biter: araştırma, ADR, kod, sınama, resim (`.run/gui-evidence/kullanim-NNNN`, Git'e girmez),
plan ve TODOS işareti, commit ve push.

Sıra: AY-01 → AY-02 → AY-03 → AY-04 → AY-05 ve AY-06 → AY-07; AY-08 (mobil istemciler) ortak tanımla başladı. ADR'ler
**9195**'ten başlar.

## Maddeler

- [x] **AY-01 — Kesin kurallar ve kararlar.** ([ADR 9195](../docs/adr/9195-module-separation.md),
  [ADR 9196](../docs/adr/9196-storage-by-project-kind.md))
  - CLAUDE.md §25 (sekiz kesin kural), “Çalışırken”de göstergesi, haritada §9.9.
  - Kılavuz §9.9: yeni modül ve yeni istemci tarifi (mobil görüntüleyici ve düzenleyici dahil).
  - Araştırma:
    - programlar ve profiller: ArcGIS Pro yapılandırmaları, AutoCAD araç takımları, QGIS profilleri, BricsCAD ve
      Vectorworks lisans düzeyleri, ortak platformda ayrı ürünler;
    - saklama: ArcGIS Pro projesi veriye başvurur, QGIS projesi veritabanında saklanır ve açıklama katmanı projede
      durur.
- [x] **AY-02 — Ekranı açık proje belirler (her programda).** ([ADR 9195](../docs/adr/9195-module-separation.md) §4)
  - `App::active_module()`: projenin modülü, proje yoksa programınki. Hepsi bir arada programda proje yokken yalnız
    ortak yüzeyler.
  - Bütün `shows_*`, komut kümesi (`offers`, `not_offered`) ve sihirbaz etkin modüle bakar. ADR 9127 ve 9185'in
    yamaları genel kurala döner.
  - Yerleşim modül başına (`layout::file_name`). Hepsi bir arada programda sekme değişince kabuk o modüle geçer.
  - Yeni proje'de bütün menülerin gelmesi sınamayla yeniden üretilir ve düzeltilir.
  - Sınama: hepsi bir arada programda her türün yüzeyleri, modülün programındakiyle bire bir aynı; sekme değişimi.
  - Resim: dört türün penceresi.
- [x] **AY-03 — Yüzey matrisi ve katman menüsü.** ([ADR 9195](../docs/adr/9195-module-separation.md) §5)
  - Panel (12), alt sekme (16), durum hücresi, ayar bölümü, Kaynaklar ve Dosya menüsünün biçimleri modüle göre
    tabloda. Şerit AY-02'den beri komut kümesinden gelir.
  - Değişen yüzeyler:
    - Yol'a CAD denetimi, Eklenti denetimi ve Harita sistemi hücresi eklendi.
    - Eklenti denetimi yalnız modüle kural sunan eklenti ya da koşu varken görünür.
    - Şablonlar CBS'de kalır: nesne şablonu CBS'nin öğe şablonudur; planın örneği düzeltildi.
  - Katman menüsü modül × katman türü (`apps/desktop/src/shell/layer_entries.rs`):
    - 42 öğe; türler joker kolsuz eşleşme, modüller profilin listesi;
    - şeritte karşılığı olan 18 öğe modülün komut kümesini izler.
  - Sınama: `layer_menu_tests` (9 × 8 × 42 hücre, komut kümesi). Masaüstü: 3053 sınama geçti.
  - Resim: `.run/gui-evidence/kullanim-9195/menu-*` (4 modül × 8 tür, modül başına sayfa, alt menüler).
- [x] **AY-04 — Modül profili kaydı ve bağımlılık denetimi.** ([ADR 9195](../docs/adr/9195-module-separation.md) §6)
  - Her modülün profili kendi klasöründe (`arch/`, `map/`, `road/`, `gis/` altında `profile.rs`), ortak
    `ModuleProfile` kaydıyla (`shell/module_profile.rs`). `module_surfaces.rs` hiçbir modülü adıyla anmaz.
  - Joker kolsuz profil tabloları: alt sekme, panel, durum hücresi, proje ve uygulama ayarı, katman menüsü,
    Kaynaklar, Öznitelikler.
  - `scripts/dev/module_deps.py --check` ve tabanı: kabuk → cad 77, gis 44, arch 28, map 13, road 12 dosya;
    gis → cad 54, cad → gis 30; kabukta 36 dal. Masaüstü sınaması koşar.
  - Ortak katalog `catalog/v1/modules.json`: `kentos-launch`'ın `Storage`'ı ve `LayerKind`'ı, şeridin komut kümesi;
    üretici `KENTOS_WRITE_MODULES=1` ile sınama.
  - Kural sınaması yapıldı: yerleşmeyen `Deneme` sekmesi beş profilde derlenmedi; `arch → map` bağı ve kabukta
    `Module::Gis` denetimde düştü.
  - Kalan: taban AY-07'de azalır; `arch/program.rs` `kentos-launch`'a doğru taşınır.
- [~] **AY-05 — CBS verisi veritabanında.** ([ADR 9196](../docs/adr/9196-storage-by-project-kind.md) §5)
  - Sihirbaz iki şey sorar: verinin yeri (GeoPackage öntanımlı, PostgreSQL, KentOS Bulut) ve proje dosyasının yeri
    (`.kcad`, veritabanının içinde, bulut). Oluştur tabloları kurar ve projeyi hemen yazar.
  - Amaçların katmanları tablodur (`kentos_project::cbs_tables`); taslak, pafta, karelaj ve yazı açıklamadır.
  - Katmanın türü ortak tek yerdedir (`kentos_native_application::layer_kind`); kayıt, nesne tutan çizim ya da bağlı
    katmanı proje dosyasına yazmaz: masaüstü Veri kaynağına taşı'yı açar, Python ve MCP `data_held_in_file` der.
  - İçe alınan yeni katman projenin veri yerine tablo olur; GeoPackage penceresinde “Bağ” yerine “Veri katmanı”.
  - Bağlama komutta reddedilir (`bound_in_gis`); `source.link` Harita ve Yol'un şeridinde.
  - Bulut: sunucu CBS'yi yalnız veritabanında tutar; Buluta yükle CBS'de yalnız veritabanını sunar ve veri
    katmanlarının satırlarını aktarır (`data_take`); CBS veritabanı projesi dosyaya çevrilmez.
  - Bağlı katman sınamaları ve ortak durumları Harita (CAD) belgesine çevrildi.
  - Kalan: Python ve MCP'de yeni CBS projesinin veri yeri (`NewDrawing`); bağlı katmanın taşınmasında bağın
    düşmesi (eski projeler).
- [x] **AY-06 — CAD projesinin tamamı proje dosyasında.** ([ADR 9196](../docs/adr/9196-storage-by-project-kind.md) §4)
  - Sunucu saklama türünü ortak modül tanımından alır (`kentos_launch::Module::storage`). CAD türleri saklamasız
    dosya olur; veritabanı oluşturmada (`storage`) ve dönüşümde (`to`) nedenini söyleyerek reddedilir. Veritabanındaki
    projenin türü CAD türüne çevrilmez (`projectType`).
  - Masaüstü: Buluta yükle CAD türünde yalnız dosyayı sunar; “PostGIS'e aktar…” CAD'de kapalıdır ve nedenini söyler.
    Python ve MCP sunucunun kuralına uyar; SDK'nın belgesi yenilendi.
  - Harita ve Yol'da CBS tablosu salt okunur altlıktır: düzenleme oturumu menüde yalnız CBS'dedir (AY-03).
  - Sınamalar: sunucuda kural sınaması, veritabanı biçimine dayanan sunucu ve API sınamaları CBS projesiyle.
  - Bulutta CBS'nin yalnız veritabanı biçimi AY-05'te yapıldı (satır aktarımıyla).
- [~] **AY-08 — Mobil görüntüleyici ve mobil uygulama için ayrım.** ([ADR 9197](../docs/adr/9197-mobile-clients.md))
  - Sahibin isteği (23 Kasım 2026): “Mobil görüntüleyici ve uygulama da geliştireceğiz ileride ona göre de ayrım yap”.
  - Yapıldı:
    - istemciler ortak tanımda (`kentos_launch::Client`): masaüstü, web, mobil görüntüleyici, mobil uygulama;
      saklama türüne göre ne yazdıkları;
    - katman türünün tutulduğu yer ve yazılışı (`LayerKind::kept`, `written_as`, `written_by`);
    - `catalog/v1/modules.json`'da istemciler ve katman türlerinin yetenekleri;
    - CLAUDE.md §25 kural 7, kılavuz §9.9;
    - katmanın mobil davranışı ([ADR 9198](../docs/adr/9198-layer-on-mobile.md)): göster, düzenle, gösterme;
      `LayerNode.mobile`, `.kcad` şema 256, `cad.layers.set`'in `mobile` işlemi (`mobile_not_offered`), seçenekler
      ortak tanımda (`LayerKind::mobile_choices`: düzenle yalnız CBS'nin veri tablosunda), mobil istemcinin okuması
      `layer_on_mobile`; masaüstünde Görünürlük ▸ Mobilde ▸. Resim `.run/gui-evidence/kullanim-9198/`.
  - Kalan:
    - mobil istemcinin modül profilleri;
    - CBS'de çevrimdışı düzenleme (değişiklik günlüğü, eşitlemede sunucu komutları);
    - mobilde CAD düzenleme kararı;
    - mobil uygulamanın teknolojisi.
  - AY-07'nin paketleri mobilin kullanacağı ortak crate'leri arayüzsüz tutar.
- [~] **AY-07 — Paketlere bölme (uzun soluklu).** ([ADR 9199](../docs/adr/9199-desktop-packages.md))
  - CAD çekirdeği ayrı paket. Sonra Harita, Mimari, Yol ve CBS masaüstü paketleri, en az bağlıdan başlayarak.
  - Her adımda bağımlılık denetimi ve derleme süresi ölçülür.
  - Hepsi bir arada program bütün paketleri, modül programı yalnız kendininkini taşır.
  - Ölçüm (ADR 9199 §1): 379 üretim modülünden 15'i `App`'e ulaşmaz; sıfırdan 72 s, tek satır 17,4 s, `App`'e yöntem
    24,5 s. Düzenek önce Harita'da kanıtlanır; CAD çekirdeği aynı düzenekle sonra ayrılır (ADR 9199 §3.1).
  - Adım 1 yapıldı (ADR 9199 §4.1): Harita modül arayüzünün (`ModuleParts`) arkasında; iletisi, penceresi ve durumu
    silinen türlerle; ortak parçalar (`map_marks`, `map_crs`, `point_calc`, `zoning_badge`, `road_dialog`, `picture`)
    çekirdekte. Taban: kabuk → Harita 13 → 2, Harita → Yol 22 → 0.
  - Adım 2 yapıldı (ADR 9199 §4.2): Harita kendi paketinde (`apps/desktop-map`), program `apps/kentos-cad`'de;
    kayıt dinamik, `impl App`'ler uzantı trait'i, sınamalar üç yerde (çekirdek 2837, Harita 191, programlar 34). Harita'da
    tek satır 17,4 s → 3,7 s. Kabuk → Harita 0.
  - Adım 3 yapıldı (ADR 9199 §4.3): Harita → CBS 3 → 0 (sayı ve adım yardımcıları çekirdekte); Harita yalnız
    çekirdeğe bağlı.
  - Adım 4 yapıldı (ADR 9199 §4.4): Harita ile Yol'un ortak arazi işi (15 modül) çekirdekte, kendi merkeziyle
    (`terrain_work`); proje ağacının sözcükleri profilden.
  - Adım 5–6 yapıldı (ADR 9199 §4.5–4.6): Yol modül arayüzünün arkasında, sonra kendi paketinde (`apps/desktop-road`);
    Yol'da tek satır 4,0 s. Sınamalar: çekirdek 2710, Harita 191, Yol 127, programlar 34.
  - Adım 7–8 yapıldı (ADR 9199 §4.7–4.8): Mimari klasöründe olup çekirdeğin kullandığı parçalar çekirdekte; Yol'un
    tasarım denetimi Yol'da; şablonlar ve etkin kat çekirdekte.
  - Adım 9 yapıldı (ADR 9199 §4.9): Mimari modül arayüzünün arkasında; ızgaralar, araç ayarları, altlık okunuşu
    çekirdekte; on bir yeni kanca. Taban: kabuk → Mimari 14 → 1, CAD → Mimari 2 → 0.
  - Adım 10 yapıldı (ADR 9199 §4.10): Mimari'nin `impl App`'leri uzantı trait'i (32 trait, `arch/ext.rs`).
  - Adım 11 yapıldı (ADR 9199 §4.11): Mimari kendi paketinde (`apps/desktop-arch`); çekirdeğin sınama kiti `testing`
    özelliğiyle açıldı; Mimari'de tek satır 3,7 s. Sınamalar: çekirdek 2503, Mimari 204, Harita 191, Yol 129, programlar
    36. Kabuk → Mimari 0.
  - Adım 12 yapıldı (ADR 9199 §4.12): `gis/` çekirdeğin ortak CBS altyapısı (QGIS'in `core`'u gibi; komutlarının çoğunu
    Harita, Yol ve Mimari de sunar); CBS modülünün kendisi `cbs/` (profili, Ağlar, Alt ağlar) ve kendi merkezi.
  - Sıradaki: `cbs/`'nin uzantı trait'leri ve paketi; CAD çekirdeğinin kabuktan ayrılması.
