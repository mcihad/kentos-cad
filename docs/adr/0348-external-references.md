# ADR 0348: Dış başvurular

- Durum: Kabul edildi (19 Ekim 2026)
- Kapsam: plan/cad.md `CAD-REF-05`, `06`, `07`, `08`, `10`, `11`, `13`, `14`; CAD-REF'in ilk grubu (8a). Çoklu belge (`01`–`04`),
  kırpma, yerinde düzenleme ve bildirim (`09`, `12`, `15`, `19`), altlıklar ve paket (`16`–`18`, `20`) sonraki gruplardadır.
- İlgili: ADR 0144 (blok), ADR 0193 §3 ve ADR 0316 (dosyadan blok), ADR 0315 (blok içi yol ve yerel kimlik, birimli blok),
  ADR 0506 (bağlı kaynağın göreli yolu), ADR 0274 ve 0276 (blok içine kenet ve alt nesne seçimi), ADR 0333 (DXF'te XREF'in atlanması).

## Bağlam

Mimarın planı, taşıyıcı projesi ve detayları ayrı dosyalardır; bir pafta onları birlikte gösterir. KentOS'ta başka bir çizim ya bir
kez blok olarak alınıyor (Dosyadan blok ekle) ya da nesneleriyle kopyalanıyordu: kaynak değişince çizim bunu bilmiyordu. AutoCAD'in dış
başvurusu (XREF) kaynağa bağlı, yeniden yüklenebilen, kendi katmanlarıyla gelen bir bloktur.

## Karar

### 1. Model

Dış başvuru bir blok tanımıdır, kaynağını taşır (`BlockDefinition.xref`, `.kcad` şema 83, `FORMATS_VERSION` 90):

- `path`: kaynak `.kcad` dosyasının mutlak yolu (son bağlanan ya da bulunan); `relative`: kayıtta çizimin klasöründen yazılan göreli
  yol (ADR 0506'nın kuralı: açılışta yol yoksa göreli yoldaki dosya, içeriği özetle aynıysa alınır);
- `kind`: `attach` (Ekle: bu çizimi dış başvuru olarak alan bir çizimde de görünür) ya da `overlay` (Bindir: yalnız bu çizimde);
- `digest`: yüklenen içeriğin SHA-256'sı; `unloaded`: boşaltılmış (içerik yok, yerleştirmeler kalır, çizilmez);
- `group`: kaynağın katmanlarını tutan katman grubunun kimliği; `parent`: iç içe gelen dış başvurunun onu getiren dış başvurusu.

İçerik çizimde kalır (son yüklenen hâli): çizim kaynak olmadan da açılır ve basılır, kaynağın yokluğu ya da değişmesi söylenir; çizim
kendiliğinden yeniden yüklenmez (`CAD-REF-09`'un “sessizce değiştirmez” kuralı). Kaynak çizimin nesneleri blok tanımının
koordinatlarında (taban 0, 0), kaynaktaki sırayla ve dosyanın numaraladığı gibi 1'den numaralı gelir; her birinin kaynaktaki kalıcı
kimliği yanında yazılır (`objects`, tanımın nesneleri sırasıyla; okuyucu sayılarının eşitliğini denetler). Yeniden yüklemede tanımda
olan nesne yerel kimliğini korur, yeni nesne en büyüğün ardından numara alır: blok içi yol yerel kimlikle izlendiği için (ADR 0315 §1)
ölçü ve kılavuz bağları nesneleriyle gider, noktaları yeniden hesaplanır; kaynakta artık olmayan nesnenin bağı nedeniyle kopar, başka
nesneye sessizce geçmez. Resim, raster ve
tablo bloğa konamaz, söylenir (ADR 0193 §3). Birim: kaynak çizimin birimi tanımın `units`'idir (ADR 0315 §4); geometri iki tarafta da metre.

### 2. Katmanlar (`CAD-REF-11`)

Kaynağın katman ağacı, bu çizimde “Dış başvuru: ad” grubunun (açık) altına yolu ve görünüşüyle gelir; parçalar o katmanlarda. Gizlemek,
dondurmak, rengi ve kilidi bu çizimin katmanlarıdır, yeniden yüklemede korunur (AutoCAD'in VISRETAIN'i açık): yeniden yükleme yalnız
eksik katmanları ekler. Bu katmanlara nesne çizilemez değildir; kural sonraki grupta (`CAD-REF-15`).

### 3. Komutlar (`CAD-REF-06`, `07`, `08`, `10`, `14`)

`cad.xrefs.edit` (bir geri alma adımı her biri):

- `attach`: `path` (mutlak), `mode` (`attach` ya da `overlay`), `name` (yoksa dosyanın adı, benzersiz), `at`, `scale`, `rotation`, `layerId`: tanım, katmanları
  ve bir yerleştirme. `drawing` (açık çizimin dosya yolu, varsa) göreli yolu ve döngüyü denetlemek için verilir.
- `reload`: kaynağı yeniden okur; içerik, özet ve eksik katmanlar yazılır, yerleştirmeler aynı kalır; dosya değişmediyse hiçbir şey
  yazılmaz. `unload`: içeriği ve getirdiklerini boşaltır, yerleştirmeler kalır.
- `repath`: yeni yoldaki dosyayı bağlar ve yükler.
- `detach`: tanımı, iç içe getirdiklerini ve bütün yerleştirmelerini siler; katman grubu, katmanlarında bu çizimin nesnesi ya da başka
  bir tanımın parçası yoksa gider (katman silmek nesnelerini de siler; kaldırmak bu çizimin nesnelerini silmez).
- `bind`: dış başvuru sıradan bloğa döner (`mode: bind` katmanlar grubunda kalır; `insert`: katmanlar grubun dışında aynı yoldaki
  katmanlarla birleşir, olmayanlar açılır, grup boş kalırsa gider); iç içe gelenler de bağlanır; cevap birleşen katmanların adlarını verir.

İç içe: kaynağın kendi `attach` dış başvuruları onun içeriğiyle (kaydedilmiş son hâlleriyle) iç içe tanım olarak gelir, yolları kaynağın
klasöründen çözülür; `overlay` olanlar ve yerleştirmeleri gelmez (`CAD-REF-07`). Döngü reddedilir (`xref_cycle`): kaynak açık çizimin
kendisiyse ya da iç içe zincirde açık çizim ya da kaynak yeniden görünüyorsa.

Salt okunur `cad.xrefs.list`: her dış başvurunun adı, türü, yolu, göreli yolu, durumu (`loaded`, `unloaded`, `missing`: dosya yok,
`changed`: dosyanın özeti yüklenenden başka), iç içe olanı, yerleştirme sayısı ve katman grubu.

Retler: `invalid_file_path`, `file_not_found`, `import_unreadable` (ADR 0316'nınkiler), `xref_cycle`, `xref_not_found` (adı verilen dış
başvuru yok), `nested_xref` (iç içe gelen dış başvuru kendi başına yeniden yüklenemez, boşaltılamaz, kaldırılamaz: onu getireni),
`xref_empty` (kaynakta bloğa konacak nesne yok), `invalid_revision`, `revision_conflict`; uyarılar `objects_left_out`, `layer_hidden`.

### 4. Seçim ve kenet (`CAD-REF-13`)

Dış başvurunun parçaları blok parçaları gibi seçilir ve kenetlenir (ADR 0274, 0276): `cad.entities.pick` ve `cad.snap.find` yerleştirmenin
kimliğini ve blok içi yolu verir; yoldaki tanım bir dış başvuruysa `cad.xrefs.list` onun kaynağını söyler.

### 5. Arayüz

Masaüstünde CAD › Ekle › Dış başvuru ▾: Dış başvuru ekle… (dosya, tür, nokta) ve Dış başvurular penceresi (liste, durum, Yeniden yükle,
Boşalt, Yolu değiştir…, Kaldır, Bağla ▾). Açılışta değişmiş ya da bulunamayan dış başvuru uyarı satırıyla söylenir.

### 6. DXF

DXF'e dış başvuru, bloğun içeriğiyle sıradan blok olarak gider (`CAD-IO-13` kendi kaynağını yazacak); DXF'in XREF bloğu okumada eskisi
gibi söylenerek atlanır.

## Sonuçlar

- Çizim kaynak dosyalar olmadan da açılır ve basılır; içeriği son yüklenen hâlidir.
- Bir dış başvurunun her yeniden yüklenmesi geri alınabilir bir adımdır.

## Doğrulama

- Ortak durumlar: `python3 scripts/fixtures/xref_command_cases.py --check` (`cad.xrefs.edit` 41, `cad.xrefs.list` 21 durum;
  kaynak çizimler `fixtures/xref/v1/*.kcad` bağımsız KCAD yazıcısıyla, katman ve nesne kuralları Çizimler arası alışverişin bağımsız
  başvurusundan).
- Durumların söyleyemedikleri `kentos-native-application`'ın `tests/all/xrefs.rs`'inde: yeniden yüklemede ölçünün bağı nesnesiyle
  gider ve noktası yeniden hesaplanır, kaynakta olmayan nesnenin bağı kopar, geri alma eskisini döndürür; kaynağın kendi dış başvurusu
  kendine ya da açık çizime dönerse döngü; kayıtta göreli yol ve taşınan klasörde göreli yolla bulunma.
- `.kcad` şema 83: kodek, bağımsız okuyucu ve yazıcı, örnek ve bozuk dosyalar.
- Resimler: `KENTOS_SHOTS_ONLY=dis-basvuru,dis-basvuru-pencere,dis-basvuru-degisti cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`.
