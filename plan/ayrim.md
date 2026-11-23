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
- [ ] **AY-02 — Ekranı açık proje belirler (her programda).**
  - `App::active_module()`: projenin modülü, proje yoksa programınki. Hepsi bir arada programda proje yokken yalnız
    ortak yüzeyler.
  - Bütün `shows_*`, komut kümesi (`offers`, `not_offered`) ve sihirbaz etkin modüle bakar. ADR 9127 ve 9185'in
    yamaları genel kurala döner.
  - Yerleşim modül başına (`layout::file_name`). Hepsi bir arada programda sekme değişince kabuk o modüle geçer.
  - Yeni proje'de bütün menülerin gelmesi sınamayla yeniden üretilir ve düzeltilir.
  - Sınama: hepsi bir arada programda her türün yüzeyleri, modülün programındakiyle bire bir aynı; sekme değişimi.
  - Resim: dört türün penceresi.
- [ ] **AY-03 — Yüzey matrisi ve katman menüsü.**
  - Her panel (12), alt sekme (16), durum hücresi, ayar bölümü, Dosya menüsü satırı ve şerit artığı modüle göre
    gözden geçirilir; tablo ADR 9195 §4'e yazılır.
  - Katman menüsü modül × katman türü (çizim, açıklama, veri tablosu, bağlı, servis, raster, nokta bulutu, grup) ile
    kesilir; `LayerEntry` tabloya genişler.
  - Matrisin her hücresinin sınaması; her modülde her katman türünün menüsünün resmi.
- [ ] **AY-04 — Modül profili kaydı ve bağımlılık denetimi.**
  - Her modülün profili kendi klasöründe (`apps/desktop/src/<modül>/profile.rs`), ortak `ModuleProfile` kaydıyla.
    Kabuk yüzey için modül adına göre dallanmaz.
  - Joker kolsuz profil tabloları: yeni yüzey her modüle yerleşmeden derlenmez.
  - `scripts/dev/module_deps.py --check`: bugünkü bağlar taban, yalnız azalır; kabuktan modülün içine, modülden
    modüle yeni bağ ve kaydın dışında modül adına göre dal reddedilir.
  - Ortak modül kataloğu: `kentos-launch`'tan `catalog/v1/modules.json` (`--check`'li üretici). Kimlik, proje türleri,
    işler, saklama türü, komut kümesi, katman türleri.
  - Kural sınaması: profile eklenmeyen yeni bir sekmenin derlenmediği ve modülden modüle eklenen bir bağın denetimde
    düştüğü bilerek denenir.
- [ ] **AY-05 — CBS verisi veritabanında.**
  - Sihirbaz iki şey sorar: verinin yeri (GeoPackage öntanımlı, PostgreSQL, KentOS Bulut) ve proje dosyasının yeri
    (`.kcad`, veritabanının içinde, bulut).
  - Amaçlar katmanlarını tablo olarak kurar; yalnız açıklama katmanı proje dosyasında kalır.
  - İçe alma tabloya yazar ve veri katmanı ekler. CBS'de bağlı katman yok. Çizim araçları veri katmanına ya da
    açıklama katmanına çizer.
  - Kayıt, açıklama dışında nesneli katmanı proje dosyasına yazmaz.
  - Bulutta CBS yalnız veritabanı biçimi. Buluta yükleme veri katmanlarının satırlarını aktarır.
  - Örnekler ve sınamalar kurala çevrilir; tablo kurma ve içe alma ortak durumları.
- [ ] **AY-06 — CAD projesinin tamamı proje dosyasında.**
  - Sunucu CAD türlerinde (`cad`, `architecture`, `zoningPlan`, `landReadjustment`, `subdivision`, `road`) veritabanı
    biçimini oluşturmada ve dönüşümde nedenini söyleyerek reddeder.
  - Masaüstünde Buluta yükle ve katalogdaki “PostGIS'e aktar…” CAD'de sunulmaz. Python ve MCP aynı kurala uyar.
  - Harita ve Yol CBS tablolarını salt okunur altlık olarak gösterebilir.
- [ ] **AY-07 — Paketlere bölme (uzun soluklu).**
  - CAD çekirdeği ayrı paket. Sonra Harita, Mimari, Yol ve CBS masaüstü paketleri, en az bağlıdan başlayarak.
  - Her adımda bağımlılık denetimi ve derleme süresi ölçülür.
  - Hepsi bir arada program bütün paketleri, modül programı yalnız kendininkini taşır.
