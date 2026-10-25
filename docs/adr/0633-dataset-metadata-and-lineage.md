# ADR 0633: Veri kümesi bilgisi, türetim kaydı ve bağımlılık grafı

- **Durum:** kabul edildi (2026-10-25). `plan/gis.md` CBS-10'un kalanı (P0 asgari: veri kümesinin tanımı ve analiz soy
  ağacı; P1 katalog: yerel kayıt). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0511 (işlem sonucunun soyu, dosyadaki `kentos.lineage`), 0529 (çalışma kaydı), 0503 ve 0505
  (bağlı katman), 0583 (katmanın kaynak sistemi), 0627 §6 (GeoPackage'ta KentOS belgeleri), 0265 (yeni şemanın kuralı).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:** kurum kataloğuna yayın ve OGC API Records sunucusu (bugün yalnız kaydın kendisi verilir), STAC
  kaydı (raster varlıkları için), alanın kendi açıklaması (bugün sözlük alanın adı, takma adı, türü, birimi ve değer
  listesinden kurulur), türetim kaydının çizimin dışına yazılan sonuçta da tutulması (dosyadaki soy ADR 0511'de),
  katman ağacında eski türevin işareti.

## Bağlam

ADR 0511 dosyaya yazılan analiz sonucunun neyden üretildiğini dosyanın içinde tuttu. Plan kalanı şöyle sayar: çizimin
katmanına yazılan sonucun soyu, “bu harita hangi veriden üretildi?” sorusunun bağımlılık grafıyla cevabı ve kaynak
değişince türevin eskidiğinin söylenmesi; veri kümesinin kendi tanımı (başlık, açıklama, sorumlu, lisans ve kullanım
kısıtı, kapsam, tarih ve güncellik, koordinat sistemi, ölçek ve çözünürlük, veri sözlüğü, kalite özeti) kaynak
kimliğiyle saklansın; katalog adaptörleri sonradan eklenebilir, taslak bir standart yayımlanmış sürüm diye kullanılmasın.

## Karar

### 1. Veri kümesi bilgisi katmanındır

Katman düğümünün `dataset`'i (`DatasetInfo`): başlık, açıklama, sorumlu (ad, kurum, e-posta), lisans (ad ya da adres),
kullanım kısıtı, en çok 50 anahtar sözcük, güncelleme günü (`YYYY-AA-GG`, gerçek bir gün), ölçek (paydası), çözünürlük ve
konum doğruluğu (metre, sıfırdan büyük), tamlık ve üretim açıklaması (saha ölçmesi, sayısallaştırma, aktarım). Her parça
isteğe bağlıdır; boş parça yazılmaz, hiçbir şey söylemeyen bilgi yoktur. Satırlar (başlık, lisans, ad) 200 karakteri, metinler
10 000'i geçmez. Bilgi yalnız katmanındır, grubun olmaz.

`cad.datasets.edit` bilgiyi tek geri alma adımında (“Veri kümesi bilgisi”) yazar, `dataset` verilmezse kaldırır, aynısını
yazmaz. Retler: `invalid_dataset`, `invalid_revision`, `revision_conflict`, `layer_not_found`, `not_a_layer`.

Kapsam, koordinat sistemi, nesne sayıları ve veri sözlüğü **saklanmaz**: her soruluşta çizimden hesaplanır, yazılı bir kopya
çizimle ayrışmaz.

### 2. Türetim kaydı: işlem aracının açtığı katman neyden üretildiğini taşır

İşlemler'in bir aracı yeni bir katman açtığında (çalıştırıcının uygulama adımında, aynı geri alma adımında) katmanın
`derivation`'ı yazılır (`LayerDerivation`):

- araç, adı ve sürümü (aynı değerlerden aynı sonuç: bugün her araç 1);
- aldığı değerler, varsayılanları doldurulmuş, özellik ve katman girdileri hariç (onlar `inputs`'tadır);
- okuduğu katmanlar: kimliği, o anki adı, içerik özeti (`sources::layer_digest`: nesnelerin tür, geometri ve
  özniteliklerinin kalıcı kimlikle SHA-256'sı, ADR 0511) ve bağlı katmanda kaynağı (`dosya › tablo`); özetler araç
  yazmadan **önce** alınır;
- çizimin sürümü, motor (`KentOS <sürüm>`) ve başlama zamanı (UTC).

Kayıt hem penceredeki çalıştırmada hem `cad.processing.run`'da, modelin her adımında ve toplu çalıştırmanın her satırında
aynı yoldan yazılır (`runner::apply`). Var olan katmana yazan araç kayıt bırakmaz: katman yalnız o aracın ürünü değildir.
Geri alma kaydı katmanla birlikte götürür.

### 3. `cad.datasets.describe`: tanım, sözlük, soy, graf

Salt okunur; katmanlar verilmezse nesne tutan bütün katmanlar ağacın sırasıyla. Her katman için:

- başlık (bilgiden, yoksa katmanın adı), ağaçtaki yolu, bilgisi;
- nesneleri türlerine göre, kutusu projenin sisteminde ve projenin bir sistemi varsa WGS 84'te (kutu kenar başına 16
  noktayla sıklaştırılarak, `crs::transform_bounds`; PROJ'un `transform_bounds`'uyla aynı kural), sistemin adı, nesnelerin
  geldiği sistem (ADR 0583), bağlı tablosu;
- veri sözlüğü: her alanın adı, takma adı, türü, zorunluluğu, birimi, değer listesinin (ya da listenin) uzunluğu ve dolu
  kayıt sayısı;
- türetim kaydı ve her girdinin bugünkü durumu: `current` (içerik özeti kayıttakiyle aynı), `changed` (değil), `gone`
  (katman çizimde yok). **Eski türev** (`stale`): bir girdisi `current` değil ya da geldiği bir katman eskimiş (kayıtlar
  üzerinden, döngüde kendisi üzerinden eskimez);
- bağımlılık grafı: geldiği katmanlar (`upstream`, türetim kayıtları üzerinden, yakından uzağa, her biri bir kez) ve ondan
  üretilenler (`downstream`, aynı kuralla);
- `records: true` ile katmanın **OGC API Records** kaydı: GeoJSON Feature, kayıt çekirdeğinin özellikleri (`type: dataset`,
  başlık, açıklama, anahtar sözcükler, dil, iletişim `pointOfContact` rolüyle, lisans, `rights`, güncelleme günü), kapsamın
  WGS 84 dörtgeni, `time`; KentOS'un ekleri `kentos:` önekiyle (sistem, ölçek, çözünürlük, doğruluk, nesne sayısı,
  türetim). Kayıt OGC 20-004'ün kayıt çekirdeği biçimindedir; standardın onay durumu bu ADR'de iddia edilmez, kayıt bir
  kataloğa verilecek veridir, bir katalog sunucusu değildir.

### 4. `.kcad` şema 106

Katmanın `dataset`'i ve `derivation`'ı JSON'larıyla şema 106'dır; yazıcı 106'yı yalnız bunlardan biri olan katman varken
yazar, başka her çizim eskisiyle bayt bayt aynıdır. Şema 2–105'te iki alan bilinmeyen alandır. Örnek
`fixtures/kcad/v2/datasets.kcad`, bozukları `broken/dataset-*.kcad`, `broken/derivation-*.kcad`,
`broken/datasets-in-schema-105.kcad` ve `broken/schema-version-107.kcad`; bağımsız Python okuyucusu aynı kuralı okur
(`docs/specs/kcad-v2.md`).

### 5. GeoPackage'ta kaynak kimliğiyle

`file.export.gpkg` her tablonun yanına katmanının bilgisini (`https://kentos.app/dataset/1`) ve türetim kaydını
(`https://kentos.app/layer-derivation/1`; ADR 0530'un `https://kentos.app/derivation/1`'i değil: o tabloyu CAD kaynağının GIS türevi yapar ve bağlanmasını reddettirir) GeoPackage'ın metadata uzantısıyla (`gpkg_metadata`, kapsam `dataset`) yazar.
`file.import.gpkg` bilgiyi aldığı tablonun açtığı katmana aynı adımda koyar (`datasets` sayısı); türetim kaydını almaz:
girdileri başka bir çizimin katmanlarıdır, burada eskiyip eskimediği bilinemez. Uzantı standarttır: GDAL ve QGIS dosyayı
okur, GDAL'ın doğrulayıcısı bir şey bulmaz.

### 6. Masaüstü

Katmanlar'ın sağ tıkında **Veri kümesi bilgisi…** (türetilmiş katmanda “(türetilmiş)”), komut adı `layer.dataset`
(VERIKUMESI, METAVERI, METADATA, SOYAGACI). Pencere dört sekmedir: **Tanım** (bilginin parçaları, ölçek `1:1000` de yazılır,
sayılar virgülle de; yazılamayanın nedeni altta, Kaydet `cad.datasets.edit` ile), **Sözlük**, **Soy** (eski türevin uyarısı ve
sekmenin uyarı ikonu, araç, zaman, değerler, girdilerin durumu, geldiği ve ondan üretilen katmanlar) ve **Kayıt** (OGC API
Records JSON'u, Panoya kopyala). Altta nesne sayıları, sistem ve WGS 84 kutusu.

## Doğrulama

- Ortak durumlar `fixtures/commands/v1/cad.datasets.edit.json` (30) ve `cad.datasets.describe.json` (21):
  `python3 scripts/fixtures/dataset_cases.py --check` (KentOS kodu olmadan; WGS 84 kutusu PROJ'un `transform_bounds`'uyla,
  kayıt ADR'nin kuralıyla). Bir işlem aracının açtığı katmanın kaydı, girdisi değişince eskimesi, değişiklik geri alınınca
  yeniden güncel olması ve kaydın katmanla geri alınması `cad.processing.run`'la oynatılır; içerik özetleri KentOS'undur
  (`$any`), durumlarına bakılır.
- KCAD: `python3 scripts/fixtures/kcad_v2_reference.py --check`, `cargo test -p kentos-kcad`.
- GeoPackage: `cargo test -p kentos-headless --test headless datasets` (KCAD örneği → GeoPackage → yeni çizim; bilgiler geri
  gelir, türetim kaydı gelmez); `KENTOS_WRITE_DATASETS=1` ile yazılan dosya `dataset_cases.py --written` ile sqlite3'ten
  belge belge ve GDAL'ın `validate_gpkg`'siyle denetlenir.
- Masaüstü: `cargo test -p kentos-desktop --bin kentos-cad dataset_window` (pencereden yazma, olmayan günün reddi,
  anahtar sözcüklerin yinelenmeyişi, tek geri alma adımı; türetilmiş katmanın eskiliği ve grafı); resimler
  `dataset_window::tests::screens` (`.run/shots/veri-kumesi-*`).
- Süreler (`dataset_timing`, release, bu makinede):

  | İş | Süre |
  |---|---:|
  | 100 000 parselin içerik özeti (bir işlem aracı katman açarken girdisi için bir kez) | 125 ms |
  | parsel katmanının tanımı (kapsam, WGS 84 kutusu, sözlük, kayıt) | 57 ms |
  | türetilmiş katmanın tanımı (girdisinin özetiyle) | 125 ms |
  | bütün katmanlar | 184 ms |

  Tanım her soruluşta hesaplanır ve özetler bir soruda bir kez alınır; pencere onu katman seçilince bir kez ister.
