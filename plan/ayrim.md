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

Sıra: AY-01 → AY-02 → AY-03 → AY-04 → AY-05 ve AY-06 → AY-07. ADR'ler **9195**'ten başlar.

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
- [ ] **AY-05 — CBS verisi veritabanında.**
  - Sihirbaz iki şey sorar: verinin yeri (GeoPackage öntanımlı, PostgreSQL, KentOS Bulut) ve proje dosyasının yeri
    (`.kcad`, veritabanının içinde, bulut).
  - Amaçlar katmanlarını tablo olarak kurar; yalnız açıklama katmanı proje dosyasında kalır.
  - İçe alma tabloya yazar ve veri katmanı ekler. CBS'de bağlı katman yok. Çizim araçları veri katmanına ya da
    açıklama katmanına çizer.
  - Kayıt, açıklama dışında nesneli katmanı proje dosyasına yazmaz.
  - Bulutta CBS yalnız veritabanı biçimi. Buluta yükleme veri katmanlarının satırlarını aktarır.
  - Örnekler ve sınamalar kurala çevrilir; tablo kurma ve içe alma ortak durumları.
- [x] **AY-06 — CAD projesinin tamamı proje dosyasında.** ([ADR 9196](../docs/adr/9196-storage-by-project-kind.md) §4)
  - Sunucu saklama türünü ortak modül tanımından alır (`kentos_launch::Module::storage`). CAD türleri saklamasız
    dosya olur; veritabanı oluşturmada (`storage`) ve dönüşümde (`to`) nedenini söyleyerek reddedilir. Veritabanındaki
    projenin türü CAD türüne çevrilmez (`projectType`).
  - Masaüstü: Buluta yükle CAD türünde yalnız dosyayı sunar; “PostGIS'e aktar…” CAD'de kapalıdır ve nedenini söyler.
    Python ve MCP sunucunun kuralına uyar; SDK'nın belgesi yenilendi.
  - Harita ve Yol'da CBS tablosu salt okunur altlıktır: düzenleme oturumu menüde yalnız CBS'dedir (AY-03).
  - Sınamalar: sunucuda kural sınaması, veritabanı biçimine dayanan sunucu ve API sınamaları CBS projesiyle.
  - Kalan: bulutta CBS'nin yalnız veritabanı biçimi AY-05'te (satır aktarımıyla).
- [ ] **AY-07 — Paketlere bölme (uzun soluklu).**
  - CAD çekirdeği ayrı paket. Sonra Harita, Mimari, Yol ve CBS masaüstü paketleri, en az bağlıdan başlayarak.
  - Her adımda bağımlılık denetimi ve derleme süresi ölçülür.
  - Hepsi bir arada program bütün paketleri, modül programı yalnız kendininkini taşır.
