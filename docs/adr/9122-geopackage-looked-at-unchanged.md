# ADR 9122: GeoPackage'a bakmak dosyayı değiştirmez; proje ilk Kaydet'te, yeri gösterilerek saklanır

- **Durum:** kabul edildi (2026-11-22); adım VY-21. Plan: [plan/veri.md](../../plan/veri.md) VY-21.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Belediye kullanım değerlendirmesinin
  1. maddesi (P0): üçüncü taraf bir paket Dosya › Aç ile seçilince, Kaydet'e basılmadan dosya 151.552 bayttan 159.744
  bayta büyüdü. SHA-256'sı değişti, `kentos_projects` tablosu ile indeksi eklendi.
- **Değişen karar:** [ADR 9116](9116-geopackage-opened-as-a-project.md) §2'deki “Proje yoksa … dosyanın içine yazılır”.
  Sahibin VY-19'daki isteği (“projeyi de oluştursun içine”) durur: proje yine dosyanın içinde saklanır. Ancak bu yazım
  açılışta değil, sahibin Kaydet'iyle ve yeri gösterildikten sonra olur.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok. **Bağımlılık:** yok.

## §1 Bağlam ve araştırma

- **Değerlendirmenin ölçütü:** veri teslimi, dosya özetiyle karşılaştırma ve salt okunur arşiv işlerinde dosyayı sırf
  görüntülemenin kalıcı bir etkisi olmamalı. Kabul: paket açılır, göz atılır, kapatılır; dosyanın özeti ile tablo ve
  indeks listesi aynı kalır. Yazma izni olmayan kopya açılır. Açık onaydan sonra saklanan proje geri yüklenir.
- **Masaüstü CBS** (QGIS 4.2 kullanım kılavuzu, *Working with Project Files*, 22 Kasım 2026'da okundu):
  - proje bir GeoPackage'a Proje menüsünün *Save To* alt menüsünden yazılır;
  - seçilen saklama yeri için bir pencere açılır, hedef orada seçilir;
  - katman eklemek ya da paketi açmak dosyaya proje yazmaz.
- **KentOS'taki neden:** `gpkg_open::build` projeyi kurduktan hemen sonra `project_store::write` ile dosyaya yazıyordu.
  Sürücünün okuma bağlantıları zaten salt okunurdu (`GeoPackage::open(path, false)`). Dosyayı değiştiren tek yer bu
  yazımdı.

## §2 Karar

1. **Açılış yazmaz.** Projesi olmayan dosyanın tabloları bellekte bir proje olur (`gpkg_open::Built`; `written` alanı
   kalktı).
   - Belge temizdir: hiçbir şey değiştirilmeden kapatılırsa soru sorulmaz.
   - Dosyanın baytları, tabloları ve indeksleri aynı kalır.
   - Yazma izni olmayan dosya da aynı yoldan açılır.
2. **Önerilen yer.** Masaüstünde belge `Source::Offered` taşır: dosya ve önerilen proje adı (dosyanın adı ya da Veri
   kaynağından aç'ta yazılan ad).
   - Açılış iletisi: “… tabloları “katmanlar” adıyla bir harita olarak açıldı (5 veri katmanı). Dosyaya bir şey
     yazılmadı; Kaydet, projeyi bu dosyada saklamayı önerir.”
   - Dosya menüsünde Kaydet'in alt satırı ve Veri kaynağı bölmesi de projenin henüz saklanmadığını söyler.
   - Son açılanlarda dosyanın alt satırı “GeoPackage — tabloları harita olarak”tır.
3. **İlk Kaydet yeri gösterir.** Veri kaynağına kaydet penceresi o dosya ve o adla açılır. Üstteki satır şunu söyler:
   Kaydet projeyi dosyanın içine yazar, tablolardaki veri değişmez, ayrı bir `.kcad` dosyası için Farklı kaydet
   kullanılır.
   - Pencerenin Kaydet'i yazar ve proje `Stored` olur. Sonraki Kaydet'ler sormadan oraya yazar.
   - Vazgeç hiçbir şey yazmaz.
   - Farklı kaydet her zamanki gibi `.kcad` dosyası sorar.
4. **“Kaydet ve kapat” pencereyi bekler.** Pencerenin Vazgeç'i kapatmayı durdurur, çizim değişiklikleriyle açık kalır.
   Kaydet projeyi yazınca kapatma sürer.
   - Bu, veri kaynağında saklanan her proje için düzeldi. Önce kayıt bitince çizim açık kalıyordu: “Kaydet ve …”
     yalnız dosyaya kayıttan sonra sürüyordu.
5. **Headless (Python, MCP):** açılış yazmaz.
   - Özette `openedGeoPackage.kept = false` ve `project.kept = false` görünür.
   - `save()` ya da yolsuz `drawing.save` projeyi dosyada saklar. Betiğin kendi kayıt çağrısı onun açık onayıdır.
   - Yerel API'de `gpkg_open::keep`, belgeyi dosyaya göre göreli yollarla yazar ve geri okuyup karşılaştırır.
6. **Değişmeyenler:**
   - tek projeli dosya o projeyi açar;
   - çok projeli dosyada seçim penceresi açılır, yalnız okur;
   - döşeme tabloları ve varsayılan stiller ADR 9116 ile 9117'deki gibi alınır;
   - görünüşlerin `layer_styles`'a yazılması Kaydet'tedir (ADR 9117 §4), ilk Kaydet'te de.

## §3 Kanıt

- **Yerel çekirdek** (`crates/native/application/tests/all/gpkg_open.rs`):
  - fixtures/data/v1/gpkg-open'ın 13 durumunun her birinde dosyanın SHA-256'sı ve `sqlite_master` listesi açılıştan ve
    bütün veri katmanlarının satırları okunduktan sonra aynı;
  - `keep` ile saklanan proje yeniden açılınca o proje geliyor;
  - salt okunur dosya açılıyor, satırları okunuyor; `keep` “salt okunur” diyor, dosya aynı kalıyor.
- **Masaüstü** (`gis/gpkg_open_tests.rs`):
  - ilk Kaydet pencereyi o dosya ve adla açıyor; pencere açıkken dosya hâlâ aynı;
  - pencerenin Kaydet'i yazıyor; ikinci Kaydet penceresiz yazıyor;
  - salt okunur dosya açılıyor ve sorusuz kapanıyor;
  - Kaydet ve kapat: pencerenin Vazgeç'i kapatmayı durduruyor; Kaydet yazıp çizimi kapatıyor.
- **Python** (7 sınama) ve **MCP** (3 sınama): açılıştan sonra dosya bayt bayt aynı; yolsuz kayıt projeyi saklıyor.
- **Resimler** (`.run/gui-evidence/gpkg-oneri-9122/`): açılan harita ve ilk Kaydet'in penceresi; açık ve koyu,
  1440 × 900 ve 1100 × 650.

## §4 Kalan

- Gerçek pencerede fare ve klavyeyle deneme (`scripts/usage/real/`, ADR 8000).
- Web (TODOS.md §0.2).
