# ADR 0534: Atlas PDF'i sabitlenen sürümden; açık bağlar ve izinli öznitelikler

- **Durum:** kabul edildi (2026-10-19). `plan/gis.md` CBS-21'in ilk bütün dilimi (“Mevcut atlas/GeoPDF'yi provider revision'ı ve
  export yetkisine bağla”). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0164 (pafta düzenleri; atlas planlayıcısı `kentos-sheet::atlas`), 0252 (başsız kitap PDF'i, `kentos-plot`),
  0251 (pafta kitabı projenin dosyasında), 0253 §10 (dosya yazan komutlar), 0503 ve 0510 (bağlı katman, kaynak dosyanın SHA-256'sı ve
  katmanın özeti), 0210 (zaman), 0211 (katman süzgeci), 0314 (haritanın temaları), 0443 §2 (formun `redact`'ı), 0265 (yeni şemanın kuralı).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - kapsam katmanını sağlayıcıdan (PostGIS, milyon satırlı GeoPackage) kontrollü sayfalarla okumak: bugün atlas çizimdeki çalışma
    kopyasından yapılır; sayfa başına bütün katman kurulur, 10⁵ nesneli bir katmanda yüzlerce sayfa için sayfanın erişimindeki nesneleri
    alan sorgusuyla seçip kurulumu önbelleğe almak gerekir (CBS-02, CBS-42);
  - haritaların yazıları (etiketler, ölçü değerleri, yazı nesneleri): başsız çıktıda yok, masaüstünün sayfa PDF'indeki gibi ortak ölçümle
    yazı düzeni ayrı iş (ADR 0252 “Açık kalanlar”); atlas sayfalarında paftanın kendi yazıları (başlık, öznitelik notları, tablolar) vardır;
  - haritanın “atlas nesnesine kırp” seçeneği (`clipToAtlasFeature`): PDF'e uygulanmıyor, söylenir (`notes`);
  - başka koordinat sisteminde basma (`crs`): bugün yalnız projenin sistemi, başkası `crs_unsupported`;
  - çıktının ayrı bir kayıt dosyası (yan `.json`) ve sayfa küçük resimleri; kayıt PDF'in `Keywords`'ünde ve cevaptadır;
  - öznitelik izninin kalıcı bir alan özelliği olması (alanın `hidden` ya da `export`'u, `.kcad` şema 95 ayrılmıştı): bugün izin formun
    `redact`'ı ve komutun `fields`'ıdır; gizli sütunun kopya ve dışa aktarmada süzülmesi CBS-20'dedir;
  - atlasın kâğıt ölçüsü ve sayfa sayısı için ayrı dosya (sayfa başına PDF) seçeneği.

## Bağlam

Paftanın atlası (`Sheet.atlas`: kapsam katmanı, süzgeç, sıralama, sayfa adı, haritanın ölçek ilkesi) ve planlayıcısı
(`atlas_plan`) çekirdekte vardı; ne masaüstünde ne başsız sunucuda ona bağlı bir şey yoktu: Pafta kipinde Atlas düğmesi soluktu, kitap
PDF'i atlas haritasını “yerleştirilmemiş” diye boş basıyordu. Plan üç şey ister: çıktı hangi sürümden yapıldığını söylemeli ve kaynak
iş sırasında değişirse ya sabitlenen sürümden yapılmalı ya yenilenmesi istenmeli; tema, zaman ve koordinat sistemi açık olmalı;
çıktıda gizli öznitelik ve süzgeç dışı nesne bulunmamalı. Başlarken ikinci ve üçüncü maddelerin açıkları vardı: `kentos-plot`'un harita
içeriği katmanın bütün nesnelerini alıyordu, katman süzgecinden ve zamandan geçmeyen nesne de PDF'te çiziliyordu (masaüstünün sahnesi
süzüyor, kâğıt süzmüyordu); tablolar ve lejantlar da bütün nesnelerden ve bütün özniteliklerinden kuruluyordu.

## Karar

### 1. `file.export.atlas` v1

Atlası PDF'e yazan, `file.export.pdf` gibi `FILE_COMMANDS`'tan başsız sunucuda çalışan dosya komutu (`validate`, `plan`, `execute`;
Python `cad.file.export.atlas`, MCP aracı, masaüstünün penceresi). Girdi: `path`, `sheet`, `overwrite`, `fields`, `theme`, `time`, `crs`, `pin`.
Sayfaları paftanın atlas kuralları seçer ve adlar (süzgeç, sıralama, ad, ölçek ilkesi: `atlas_plan`, yeniden yazılmadı); komut onları
çizimin verisine bağlar. `plan` dosyayı yazmaz, PDF'i yapar (SHA-256 ve boyut cevaptadır; tekrar oynatma onu karşılaştırır), sayfaları
(`pages`: ad, nesne, ölçek, merkez, kutu), pin'i ve bağları söyler. Yazma, öteki dosya komutları gibi geçici dosyaya, geri okuyup
karşılaştırarak, tek yeniden adlandırmayla olur. Çizimi değiştirmez, geri alma adımı değildir. Reddedilenler (sırasıyla):
`invalid_file_path`, `file_exists`, `no_sheets`, `unknown_sheet`, `no_atlas`, `atlas_no_layer`, `crs_unsupported`, `unknown_theme`,
`invalid_time`, `field_not_found`, `atlas_expression`, `atlas_empty`, `pin_mismatch` (çatışma), `source_changed` (çatışma),
`pdf_unwritable`, `file_unwritable`; uyarılar `source_stale` ve `source_unsaved`.

### 2. Sabitlenen sürüm (`AtlasPin`)

Pin, sayfaların okunduğu durumdur: projenin ayarlarının, katman ağacının, stillerin ve paftaların damgaları (`Document::*_stamp`, ADR
0253), atlasın okuduğu her katmanın nesne sayısı ve özeti (`sources::layer_digest`: nesnelerin türü, geometrisi ve öznitelikleri kalıcı
kimlikle) ve bağlı katmanın kaynak dosyasının SHA-256'sı (bağlandığı andaki `revision` ve şimdiki `current`). Atlasın okuduğu katmanlar:
kapsam katmanı, haritalarının gösterdiği (temalar ve geçersiz kılmalar dahil), tablolarının ve koordinat listelerinin kaynağı; okunmayan
katmandaki değişiklik çıktıyı eskitmez. Belgenin sürüm sayacı (`revision`) söylenir ama karşılaştırılmaz: geri al ve yinele sayacı
değiştirir, içeriği değiştirmez. Pin'in `digest`'i sayaçsız kalanın SHA-256'sıdır; tek söz olarak çıktının `Keywords`'üne yazılır.

- **Kaynaktan sabitleme.** Komut çizimin okuma kopyası değil, kendi `&Document`'i üstünde çalışır ve iş boyunca belge değişemez: sayfalar
  pin'deki durumdan yapılır. Masaüstünün penceresi kopya alır ve arka plan iş parçacığında çalıştırır; pencere kapanırken çizimin pin'ini
  yeniden hesaplar ve değiştiyse söyler (“çıktı şu durumdan yapıldı, çizim sonra değişti”).
- **Yenileme istenir.** Girdideki `pin`, sayfaların okunacağı durumla karşılaştırılır; ikisi aynı içerikte değilse hiçbir şey yapılmadan
  `pin_mismatch` (hangi parçanın değiştiği iletide: “Parsel katmanının nesneleri”, “ayarlar”, “kaynak dosyası”) döner; plan yeniden alınır.
  Pencere önizlemenin pin'ini Dışa aktar'a verir: önizlemeden sonra çizim ya da bağlı dosya değiştiyse reddedilir ve önizleme yenilenir.
- **İş sırasında kaynak dosya.** Sayfalar yapıldıktan sonra, dosya yazılmadan önce bağlı kaynakların dosyaları yeniden okunur; biri pin'dekinden
  başkaysa `source_changed` ve hiçbir şey yazılmaz: iki durumun karışımı çıktı olmaz. Dosyası bağlandığındakinden başkaysa (`source_stale`) ya da
  katmanda dosyaya yazılmamış değişiklik varsa (`source_unsaved`) sayfalar katmandan, yani sabitlenen sürümden yapılır ve söylenir.
- **Çıktının kaydı.** PDF'in `Keywords`'ü: `kentos.atlas/1; paftalar=…; durum=<digest>; revizyon=…; kaynak=<id>:<sha>; tema=…; zaman=…;
  koordinat=EPSG:…`. Paftanın boş `@kaynak` değişkeni sayfada “Proje (çizim durumu 12 hane)” olur. Cevap pin'in tamamını, bağları ve
  sayfaları taşır.

### 3. Açık bağlar

- **Tema.** `theme`: projenin katman durumunun adı; atlas haritaları o durumun görünürlüğü, dondurması, yazdırması ve stilleriyle çizilir
  (`MapLayers::Theme`; ADR 0314), çizim değişmez. Olmayan ad `unknown_theme`. Verilmezse haritalar kendi katman ayarını korur ve bu böyle kayıtlıdır.
- **Zaman.** `time`: zamanlı katmanların gösterildiği an (`2024-06-30`, `2024-06-30T12:00`; çekirdeğin `time::read`'i). Aynı anda geometri
  deposunun zaman penceresi kurulur: kapsam katmanının nesneleri (sayfa olacaklar), haritaların nesneleri, tablolar ve lejantlar onunla
  kesilir. Okunamayan an `invalid_time`. Verilmezse kesilmez.
- **Koordinat sistemi.** Çıktı projenin sisteminde basılır (GeoPDF: TM sistemde her harita çerçevesi coğrafi görüntü penceresi taşır, GDAL
  koordinatı okur). `crs` verilirse projeninkine eşit olmalı, değilse `crs_unsupported`; yerel proje GeoPDF olmaz ve söylenir.

### 4. Yalnız görünen ve izinli olan çıkar

- **Görünen nesneler.** Katman süzgecinden ve zamandan geçmeyen nesne (`Spatial::view_shown`) sayfa olmaz ve haritada, tabloda, lejantta
  yoktur. Bu kural atlasa özel değildir: `vector_content` (kâğıdın harita içeriği) süzülmemiş nesne çiziyordu, düzeltildi; masaüstünün sayfa
  PDF'i ve başsız kitap PDF'i de artık süzgeçli katmanın dışarıda bıraktıklarını basmaz. Süzgeçten hiç nesne geçmeyen katman dosyada içerik
  grubu bile olmaz.
- **İzinli öznitelikler.** Sayfaların taşıdığı öznitelikler `AtlasFeature.attributes`'tür; paftanın bütün ifadeleri (süzgeç, sıralama, ad,
  yazılar, tablo hücreleri) yalnız onu okur, olmayanı `‹ad?›` işaretiyle yazar. Taşınanlar: `fields` verilmişse onlar, yoksa kapsam katmanının
  tanımlı alanları (alanı tanımlı olmayan katmanda bütün öznitelik adları); hepsinden **bir formda `redact` işaretli alan çıkarılır**
  (`python_call::redacted_names`: ADR 0443'ün kayıtçıdan gizlediği alanlar, çıktıdan da gizlenir; istense de taşınmaz). Taşınmayanlar
  cevabın `withheld`'ındadır. Aynı kural kapsam katmanının tablosuna (`Permit`) ve atlas nesnesine göre süzülen tablolara
  (`FeatureInput.in_atlas`: nesnenin kutusuna değenler) ve lejantlara uygulanır.

### 5. Çok sayfalı PDF çekirdekte

PDF yazıcısı `PdfInputs.atlas`'ı (`PdfAtlas`: pafta ve sayfalar; her sayfada atlas girdisi, haritaların içeriği, tablolar, koordinat
listeleri, lejantlar) ve `keywords`'ü alır: atlas paftası kendi sıra yerinde atlasın sayfa sayısı kadar sayfa olur, her sayfa kendi nesnesinin
girdileriyle çizilir; katman içerik grupları, yazı tipi alt kümeleri ve resimler bütün sayfalar için bir kez yazılır; GeoPDF görüntü penceresi
her sayfanın haritası için kendi merkeziyle. Aynı girdi aynı bayt (ADR 0164'ün kuralı). Atlas olmayan bütün girdiler eskisi gibidir
(`kentos-sheet` testleri değişmedi). Sayfaların haritaları başsız kitap PDF'inin çizicisiyle (`MapDrawer`, `book.rs`'ten çıkarıldı)
yazılır; atlas olmayan (sabit görünüşlü, genel bakış) haritalar bir kez.

### 6. Masaüstü

Çıktı › Pafta › Atlası PDF yap… ve Pafta kipinin Atlas düğmesi (`sheet.atlas`, ADR 0164'ün soluk bıraktığı) aynı pencereyi açar
(`exchange/atlas_export.rs`): pafta, atlas tanımı (kapsam katmanı, süzgeç, sıralama, sayfa adı; Tanımı uygula `cad.sheets.edit`'in
`setAtlas`'ıyla yazar), tema, zaman; sağda komutun planı: sayfa sayısı, süzgeçten geçmeyenler, taşınan ve taşınmayan öznitelikler, sabitlenen
durum ve kaynakları, uyarılar, sayfaların listesi (ad, ölçek, merkez). Dışa aktar… pin'i komuta verir ve PDF'i arka plan iş parçacığında
çizimin kopyasında yazar. Önizleme çizimin ana iş parçacığındadır (özetler nesne sayısıyla doğrusal); büyük kaynaklarda arka plana almak
sonraya kalandır.

### 7. Ortak durumlar

`file.export.*` komutları belge komutları değildir (`FILE_COMMANDS`); `fixtures/commands/v1`'in koşucuları (masaüstü, Python, web) o klasörü
`DESKTOP_COMMANDS` ile eşler. Bu yüzden 27 durum `fixtures/atlas/v1/cases.json`'dadır (`kentos.file-command-cases` v1) ve başsız
sunucunun koşucusu `crates/native/headless/tests/headless/atlas_cases.rs` onları protokolden geçirir. Durumları ve beklenen sayfaları
(yüz sayfa, adlar, sıra, merkezler, ölçekler) `scripts/fixtures/atlas_cases.py` KentOS kodu olmadan kurar: 105 parsel, beşi iptal,
her birinin malik alanı (`redact`), alanı tanımlı olmayan bir öznitelik, 90 × 50 m'lik büyük bir parsel (1/1000), sistemin atlas şablonu.

## Doğrulama

- Paylaşılan durumlar ve başvuru betiği:

  ```bash
  python3 scripts/fixtures/atlas_cases.py --check
  cargo test -p kentos-headless --test headless atlas
  ```

- Gerçek dosya poppler ve GDAL'la (yüz sayfa, `Keywords`, her örnek sayfanın metni, tablo, gizli öznitelik ve iptal parsel yok, GeoPDF'in
  koordinat sistemi ve her sayfanın haritasının merkezi ve genişliği):

  ```bash
  KENTOS_WRITE_ATLAS=1 cargo test -p kentos-headless --test headless atlas
  python3 scripts/fixtures/atlas_cases.py --files
  ```

- Çekirdek ve çizim: `cargo test -p kentos-plot` (yüz sayfa, süzgeç dışı nesnenin haritada olmadığı, pin'in kuralları, bağların retleri,
  redact, iş sırasında değişen bağlı dosya), `cargo test -p kentos-sheet --test all an_atlas_sheet`.
- Python: `python/tests/test_atlas.py` (plan, yazma, reddedilen pin); MCP aracı katalogdan; `pnpm arch:deps`, `python3 scripts/cad/manifest.py --check`.
- Masaüstü: `cargo test -p kentos-desktop atlas`; resimler `cargo test -p kentos-desktop exchange::atlas_tests::screens -- --ignored --nocapture`
  (`.run/shots/atlas-*`, 1440 × 900 ve 1100 × 650, koyu ve açık).
- Bulgular: kaynaktaki dosyanın SHA-256'sı atlas yapılırken (`pdf_with`'in kancasıyla) değiştirilince `source_changed`; plan'dan sonra bir nesne
  eklenince `pin_mismatch`, geri alınınca (içerik aynı) pin kabul edilir; süzgeçli çizimde komşu parselin sayfasında süzgeçsiz çizime göre daha
  az çizgi vardır (süzgeç sızıntısı testi, düzeltmeden önce düşer).
- Süre: hata ayıklama derlemesinde yüz sayfalık atlas (PDF 635 KB) 1,3 s.

## Sonuçlar

- Atlas ilk kez uçtan uca: tanım penceresi, önizleme, sabitlenen durumdan yüz sayfalık PDF/GeoPDF, Python ve MCP'den aynı komut.
- Çıktı hangi sürümden olduğunu söyler; kaynak değişirse ya sabitlenen sürümden yapıldığı ya yenilenmesi istendiği açıktır.
- Süzgeç dışı nesne ve gizli öznitelik çıktıda yoktur; bu kural kâğıt çıktısının geneline (sayfa PDF'i, kitap PDF'i) yayıldı.
- Açık kalanlar yukarıdaki “Sonraya kalan”dadır: sağlayıcıdan okuma ve büyük katman için ölçek, harita yazıları, nesneye kırpma.
